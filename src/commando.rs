use crate::commando;
use crate::ln::msgs::DecodeError;
use crate::ln::wire::Type;
use crate::util::ser::{LengthLimitedRead, Readable, Writeable, Writer};
use serde::{Deserialize, Serialize};

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
