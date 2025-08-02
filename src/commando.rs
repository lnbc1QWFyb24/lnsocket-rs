use crate::Error;
use crate::LNSocket;
use crate::commando;
use crate::ln::msgs::{self, DecodeError};
use crate::ln::wire::Message;
use crate::ln::wire::Type;
use crate::util::ser::{LengthLimitedRead, Readable, Writeable, Writer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io;

impl CommandoCommand {
    pub fn new(id: u64, method: String, rune: String, params: Vec<serde_json::Value>) -> Self {
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

    pub fn params(&self) -> &[serde_json::Value] {
        &self.params
    }
}

/// The json data in a commando command packet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandoCommand {
    id: u64,
    method: String,
    params: Vec<serde_json::Value>,
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
    if typ == commando::COMMANDO_REPLY_CONT {
        let req_id: u64 = Readable::read(buf)?;
        let mut chunk = Vec::with_capacity(buf.remaining_bytes() as usize);
        buf.read_to_end(&mut chunk)?;
        Ok(Some(IncomingCommandoMessage::Chunk(CommandoReplyChunk {
            req_id,
            chunk,
        })))
    } else if typ == commando::COMMANDO_REPLY_TERM {
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

/*
impl Readable for CommandoReplyChunk {
    fn read<R: Read>(reader: &mut R) -> Result<Self, DecodeError> {
        let req_id: u64 = Readable::read(reader)?;
        let mut chunk = Vec::with_capacity(buf.remaining_bytes() as usize);
        buf.read_to_end(&mut chunk)?;
        Ok(Self { req_id, chunk })
    }
}
*/

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

pub const COMMANDO_COMMAND: u16 = 0x4c4f;
pub const COMMANDO_REPLY_CONT: u16 = 0x594b;
pub const COMMANDO_REPLY_TERM: u16 = 0x594d;

/// A lightweight client for Core Lightning’s Commando RPC protocol.
///
/// This client:
/// - Wraps an [`LNSocket`],
/// - Sends JSON-RPC requests (`method` + `params`),
/// - Streams partial reply chunks until completion.
///
/// ### Example
/// ```no_run
/// # use lnsocket::{LNSocket, CommandoClient};
/// # use bitcoin::secp256k1::{SecretKey, PublicKey, rand};
/// # async fn example(peer: PublicKey) -> Result<(), lnsocket::Error> {
/// let sk = SecretKey::new(&mut rand::thread_rng());
/// let mut sock = LNSocket::connect_and_init(sk, peer, "ln.damus.io:9735").await?;
///
/// let mut commando = CommandoClient::new("your-rune-token");
/// let resp = commando.call(&mut sock, "getinfo", vec![]).await?;
/// println!("node info: {resp}");
/// # Ok(()) }
/// ```
pub struct CommandoClient {
    req_ids: u64,
    rune: String,
    chunks: HashMap<u64, Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct CompleteCommandoResponse {
    req_id: u64,
    json: serde_json::Value,
}

#[derive(Clone, Debug)]
pub enum CommandoResponse {
    Partial(u64),
    Complete(CompleteCommandoResponse),
}

impl CommandoClient {
    pub fn new(rune: impl Into<String>) -> Self {
        let req_ids: u64 = 1;
        Self {
            req_ids,
            rune: rune.into(),
            chunks: Default::default(),
        }
    }

    async fn send(
        &mut self,
        socket: &mut LNSocket,
        method: impl Into<String>,
        params: Vec<Value>,
    ) -> Result<u64, io::Error> {
        self.req_ids += 1;
        let req_id = self.req_ids;
        let command = CommandoCommand::new(req_id, method.into(), self.rune.clone(), params);

        socket.write(&command).await?;

        Ok(req_id)
    }

    fn update_chunks(&mut self, mut cont: CommandoReplyChunk) -> &[u8] {
        self.chunks
            .entry(cont.req_id)
            .and_modify(|chunks| chunks.append(&mut cont.chunk))
            .or_insert(cont.chunk)
    }

    fn finalize_chunks(
        &mut self,
        cont: CommandoReplyChunk,
    ) -> Result<CompleteCommandoResponse, Error> {
        let req_id = cont.req_id;
        let json = {
            let r = self.update_chunks(cont);
            serde_json::from_slice(r)?
        };
        self.chunks.remove(&req_id);
        Ok(CompleteCommandoResponse { req_id, json })
    }

    pub async fn call(
        &mut self,
        socket: &mut LNSocket,
        method: impl Into<String>,
        params: Vec<Value>,
    ) -> Result<serde_json::Value, Error> {
        let req_id = self.send(socket, method, params).await?;

        loop {
            match self.read(socket).await? {
                Message::Custom(CommandoResponse::Complete(msg)) if msg.req_id == req_id => {
                    return Ok(msg.json);
                }

                // rusty told me once that we will get disconnected if we don't reply to these
                Message::Ping(ping) => {
                    socket
                        .write(&msgs::Pong {
                            byteslen: ping.ponglen,
                        })
                        .await?;
                }

                _ => {}
            }

            // TODO: timeout?
            //println!("skipping {result:?}");
        }
    }

    async fn read(&mut self, socket: &mut LNSocket) -> Result<Message<CommandoResponse>, Error> {
        let commando_msg: Message<IncomingCommandoMessage> = socket
            .read_custom(|typ, buf| commando::read_incoming_commando_message(typ, buf))
            .await?;

        Ok(match commando_msg {
            Message::Custom(incoming) => match incoming {
                IncomingCommandoMessage::Chunk(chunk) => {
                    let req_id = chunk.req_id;
                    self.update_chunks(chunk);
                    Message::Custom(CommandoResponse::Partial(req_id))
                }
                IncomingCommandoMessage::Done(chunk) => {
                    Message::Custom(CommandoResponse::Complete(self.finalize_chunks(chunk)?))
                }
            },

            Message::Init(a) => Message::Init(a),
            Message::Error(a) => Message::Error(a),
            Message::Warning(a) => Message::Warning(a),
            Message::Ping(a) => Message::Ping(a),
            Message::Pong(a) => Message::Pong(a),
            Message::Unknown(unk) => Message::Unknown(unk),
        })
    }
}
