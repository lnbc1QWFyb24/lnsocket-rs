use crate::Error;
use crate::LNSocket;
use crate::ln::msgs;
use crate::ln::msgs::DecodeError;
use crate::ln::wire::Message;
use crate::ln::wire::Type;
use crate::util::ser::{LengthLimitedRead, Readable, Writeable, Writer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::timeout;

pub const COMMANDO_COMMAND: u16 = 0x4c4f;
pub const COMMANDO_REPLY_CONT: u16 = 0x594b;
pub const COMMANDO_REPLY_TERM: u16 = 0x594d;

impl CommandoCommand {
    pub fn new(id: u64, method: String, rune: String, params: Value) -> Self {
        Self {
            id,
            method,
            rune,
            params,
        }
    }

    pub fn req_id(&self) -> u64 {
        self.id
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn rune(&self) -> &str {
        &self.rune
    }

    pub fn params(&self) -> &Value {
        &self.params
    }
}

/// The json data in a commando command packet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandoCommand {
    id: u64,
    method: String,
    params: Value,
    rune: String,
}

#[derive(Debug, Clone)]
pub struct CommandoReplyChunk {
    pub req_id: u64,
    pub chunk: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum IncomingCommandoMessage {
    Chunk(CommandoReplyChunk),
    Done(CommandoReplyChunk),
}

pub fn read_incoming_commando_message<R: LengthLimitedRead>(
    typ: u16,
    buf: &mut R,
) -> Result<Option<IncomingCommandoMessage>, DecodeError> {
    if typ == COMMANDO_REPLY_CONT {
        let req_id: u64 = Readable::read(buf)?;
        let mut chunk = Vec::with_capacity(buf.remaining_bytes() as usize);
        buf.read_to_end(&mut chunk)?;
        Ok(Some(IncomingCommandoMessage::Chunk(CommandoReplyChunk {
            req_id,
            chunk,
        })))
    } else if typ == COMMANDO_REPLY_TERM {
        let req_id: u64 = Readable::read(buf)?;
        let mut chunk = Vec::with_capacity(buf.remaining_bytes() as usize);
        buf.read_to_end(&mut chunk)?;
        Ok(Some(IncomingCommandoMessage::Done(CommandoReplyChunk {
            req_id,
            chunk,
        })))
    } else {
        Ok(None)
    }
}

impl Writeable for CommandoCommand {
    fn write<W: Writer>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        self.id.write(writer)?;
        writer.write_all(
            &serde_json::to_string(self)
                .expect("commando command json")
                .into_bytes(),
        )?;
        Ok(())
    }
}

impl Type for CommandoCommand {
    fn type_id(&self) -> u16 {
        COMMANDO_COMMAND
    }
}

impl Type for IncomingCommandoMessage {
    fn type_id(&self) -> u16 {
        match self {
            IncomingCommandoMessage::Chunk(_) => COMMANDO_REPLY_CONT,
            IncomingCommandoMessage::Done(_) => COMMANDO_REPLY_TERM,
        }
    }
}

// Control messages to the pump task
enum Ctrl {
    Start {
        cmd: CommandoCommand,
        done_tx: oneshot::Sender<Result<Value, Error>>,
    },
}

/// Public client: generate IDs internally; expose only `call_json`.
pub struct CommandoClient {
    tx: mpsc::Sender<Ctrl>,
    rune: String,
    next_id: AtomicU64,
}

impl CommandoClient {
    /// Spawn the background pump that owns the LNSocket.
    pub fn spawn(sock: LNSocket, rune: impl Into<String>) -> Self {
        let (tx, rx) = mpsc::channel::<Ctrl>(128);
        tokio::spawn(pump(sock, rx));

        Self {
            tx,
            rune: rune.into(),
            next_id: AtomicU64::new(1),
        }
    }

    #[inline]
    fn alloc_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub async fn call(
        &self,
        method: impl Into<String>,
        params: Value,
        wait: Option<Duration>,
    ) -> Result<Value, Error> {
        self.call_with_rune(self.rune.clone(), method, params, wait)
            .await
    }

    pub async fn call_with_rune(
        &self,
        rune: String,
        method: impl Into<String>,
        params: Value,
        wait: Option<Duration>,
    ) -> Result<Value, Error> {
        let (done_tx, done_rx) = oneshot::channel();
        let cmd = CommandoCommand::new(self.alloc_id(), method.into(), rune, params);

        self.tx
            .send(Ctrl::Start { cmd, done_tx })
            .await
            .map_err(|_| Error::Io(std::io::ErrorKind::BrokenPipe))?;

        match wait {
            Some(d) => timeout(d, async { done_rx.await })
                .await
                .map_err(|_| Error::Io(std::io::ErrorKind::TimedOut))?
                .map_err(|_| Error::Io(std::io::ErrorKind::BrokenPipe))?,
            None => done_rx
                .await
                .map_err(|_| Error::Io(std::io::ErrorKind::BrokenPipe))?,
        }
    }
}

// Background task: single reader + demux per internal req_id.
async fn pump(mut sock: LNSocket, mut rx: mpsc::Receiver<Ctrl>) {
    struct InProgress {
        done_tx: oneshot::Sender<Result<Value, Error>>,
        buf: Vec<u8>,
    }
    let mut pending: HashMap<u64, InProgress> = HashMap::new();

    loop {
        tokio::select! {
            Some(ctrl) = rx.recv() => match ctrl {
                Ctrl::Start { cmd, done_tx } => {
                    let req_id = cmd.req_id();
                    // register before write to avoid race with fast replies
                    pending.insert(req_id, InProgress { done_tx, buf: Vec::new() });
                    if let Err(e) = sock.write(&cmd).await {
                        if let Some(p) = pending.remove(&req_id) {
                            let _ = p.done_tx.send(Err(e.into()));
                        }
                    }
                }
                //Ctrl::Pong(pong) => { let _ = sock.write(&pong).await; }
            },

            res = sock.read_custom(|typ, buf| read_incoming_commando_message(typ, buf)) => {
                match res {
                    Err(e) => {
                        for (_, p) in pending.drain() {
                            let _ = p.done_tx.send(Err(e.clone()));
                        }
                        break; // drop on fatal read error
                    }
                    Ok(Message::Ping(ping)) => {
                        tracing::trace!("pump: pingpong {}", ping.ponglen);
                        let _ = sock.write(&msgs::Pong { byteslen: ping.ponglen }).await;
                    }
                    Ok(Message::Custom(IncomingCommandoMessage::Chunk(chunk))) => {
                        tracing::trace!("pump: [{}] chunk_partial {}", chunk.req_id, chunk.chunk.len());
                        if let Some(p) = pending.get_mut(&chunk.req_id) {
                            p.buf.extend_from_slice(&chunk.chunk);
                        }
                    }
                    Ok(Message::Custom(IncomingCommandoMessage::Done(chunk))) => {
                        tracing::trace!("pump: [{}] chunk_done {}", chunk.req_id, chunk.chunk.len());
                        if let Some(mut p) = pending.remove(&chunk.req_id) {
                            p.buf.extend_from_slice(&chunk.chunk);
                            let parsed = serde_json::from_slice::<Value>(&p.buf).map_err(Error::from);
                            let _ = p.done_tx.send(parsed);
                        }
                    }
                    Ok(other) => {
                        tracing::trace!("pump: other_msg {}", other.type_id());
                        //tracing::trace!()
                    }
                }
            }
        }
    }
}
