use crate::{
    Error,
    commando::{self, CommandoCommand, CommandoReplyChunk, IncomingCommandoMessage},
    ln::{
        msgs::DecodeError,
        peer_channel_encryptor::PeerChannelEncryptor,
        wire::{self, Message},
    },
    util::ser::Writeable,
};
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey, SignOnly, rand};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, Cursor};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream, lookup_host};

const ACT_TWO_SIZE: usize = 50;

pub struct LNSocket {
    secp_ctx: Secp256k1<SignOnly>,
    our_key: SecretKey,
    their_pubkey: PublicKey,
    channel: PeerChannelEncryptor,
    stream: TcpStream,
}

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

    pub async fn send(
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

    fn update_chunks<'a>(&'a mut self, mut cont: CommandoReplyChunk) -> &'a [u8] {
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

    pub async fn read(
        &mut self,
        socket: &mut LNSocket,
    ) -> Result<Message<CommandoResponse>, Error> {
        let commando_msg: Message<IncomingCommandoMessage> = socket
            .read_custom(|typ, buf| Ok(commando::read_incoming_commando_message(typ, buf)?))
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

impl LNSocket {
    pub async fn connect(
        our_key: SecretKey,
        their_pubkey: PublicKey,
        addr: &str,
    ) -> Result<LNSocket, Error> {
        let secp_ctx = Secp256k1::signing_only();

        // Look up host to resolve domain name to IP address
        let addr = lookup_host(addr)
            .await?
            .next()
            .ok_or_else(|| Error::DnsError)?;

        let socket = if addr.is_ipv4() {
            TcpSocket::new_v4()?
        } else {
            TcpSocket::new_v6()?
        };

        let mut stream = socket.connect(addr).await?;
        let ephemeral = SecretKey::new(&mut rand::thread_rng());

        let mut channel = PeerChannelEncryptor::new_outbound(their_pubkey, ephemeral);
        let act_one = channel.get_act_one(&secp_ctx);
        stream.write_all(&act_one).await?;

        let mut act_two = [0u8; ACT_TWO_SIZE];
        stream.read_exact(&mut act_two).await?;
        let act_three = channel.process_act_two(&secp_ctx, &act_two, &our_key)?;

        // Finalize the handshake by sending act3
        stream.write_all(&act_three).await?;

        Ok(Self {
            secp_ctx,
            our_key,
            their_pubkey,
            channel,
            stream,
        })
    }

    pub async fn write<M: wire::Type + Writeable>(&mut self, m: &M) -> Result<(), io::Error> {
        let msg = self.channel.encrypt_message(m);
        self.stream.write_all(&msg).await?;
        Ok(())
    }

    pub async fn read(&mut self) -> Result<Message<()>, Error> {
        self.read_custom(|_type, _buf| Ok(None)).await
    }

    async fn read_custom<T>(
        &mut self,
        handler: impl FnOnce(u16, &mut Cursor<&[u8]>) -> Result<Option<T>, DecodeError>,
    ) -> Result<Message<T>, Error>
    where
        T: core::fmt::Debug,
    {
        let mut hdr = [0u8; 18];

        self.stream.read_exact(&mut hdr).await?;
        let size = self.channel.decrypt_length_header(&mut hdr)? as usize;
        //println!("len header {size}");
        let mut buf = vec![0; size + 16];
        self.stream.read_exact(&mut buf).await?;
        //println!("got cipher bytes {}", hex::encode(&buf));
        self.channel.decrypt_message(&mut buf)?;
        let u8_buf: &[u8] = &buf[..buf.len() - 16];
        let mut cursor = io::Cursor::new(u8_buf);

        Ok(wire::read(&mut cursor, handler).map_err(|(de, _)| de)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ln::msgs;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_ping_pong() -> Result<(), Error> {
        let key = SecretKey::new(&mut rand::thread_rng());
        let their_key = PublicKey::from_str(
            "03f3c108ccd536b8526841f0a5c58212bb9e6584a1eb493080e7c1cc34f82dad71",
        )
        .unwrap();

        let mut lnsocket = LNSocket::connect(key, their_key, "ln.damus.io:9735").await?;

        if let Message::Init(_) = lnsocket.read().await? {
            // ok
        } else {
            assert!(false);
        }

        let req_id = lnsocket
            .write(&msgs::Init {
                features: vec![0; 5],
                global_features: vec![0; 2],
                remote_network_address: None,
                networks: Some(vec![bitcoin::constants::ChainHash::BITCOIN]),
            })
            .await?;

        //println!("got here");
        let req_id = lnsocket
            .write(&msgs::Ping {
                ponglen: 4,
                byteslen: 8,
            })
            .await?;

        loop {
            if let Message::Pong(_) = lnsocket.read().await? {
                break;
            } else {
                // didn't get pong?
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_commando() -> Result<(), Error> {
        let key = SecretKey::new(&mut rand::thread_rng());
        let their_key = PublicKey::from_str(
            "03f3c108ccd536b8526841f0a5c58212bb9e6584a1eb493080e7c1cc34f82dad71",
        )
        .unwrap();

        let mut lnsocket = LNSocket::connect(key, their_key, "ln.damus.io:9735").await?;
        let rune = "hfYByx-RDwdBfAK-vOWeOCDJVYlvKSioVKU_y7jccZU9MjkmbWV0aG9kPWdldGluZm8=";
        let mut commando = CommandoClient::new(rune);
        let req_id = lnsocket
            .write(&msgs::Init {
                features: vec![0; 5],
                global_features: vec![0; 2],
                remote_network_address: None,
                networks: Some(vec![bitcoin::constants::ChainHash::BITCOIN]),
            })
            .await?;

        let req_id = commando
            .send(&mut lnsocket, "getinfo", Vec::with_capacity(0))
            .await?;

        loop {
            let result = commando.read(&mut lnsocket).await?;
            if let Message::Custom(CommandoResponse::Complete(msg)) = &result {
                println!("{}", serde_json::to_string(&msg.json).unwrap());
                break;
            }
            //println!("skipping {result:?}");
        }

        Ok(())
    }
}
