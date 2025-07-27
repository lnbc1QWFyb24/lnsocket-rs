use crate::{
    Error,
    ln::{
        msgs::{self, DecodeError},
        peer_channel_encryptor::PeerChannelEncryptor,
        wire::{self, Message},
    },
    util::ser::Writeable,
};
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey, rand};
use std::io::{self, Cursor};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream, lookup_host};

const ACT_TWO_SIZE: usize = 50;

pub struct LNSocket {
    channel: PeerChannelEncryptor,
    stream: TcpStream,
}

impl LNSocket {
    pub async fn connect(
        our_key: SecretKey,
        their_pubkey: PublicKey,
        addr: &str,
    ) -> Result<LNSocket, Error> {
        let secp_ctx = Secp256k1::signing_only();

        // Look up host to resolve domain name to IP address
        let addr = lookup_host(addr).await?.next().ok_or(Error::DnsError)?;

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

        Ok(Self { channel, stream })
    }

    pub async fn connect_and_init(
        our_key: SecretKey,
        their_pubkey: PublicKey,
        addr: &str,
    ) -> Result<LNSocket, Error> {
        let mut lnsocket = LNSocket::connect(our_key, their_pubkey, addr).await?;
        lnsocket.perform_init().await?;
        Ok(lnsocket)
    }

    /// No commands will work until you exchange init messages with your peer
    ///
    /// See [`connect_and_init`]
    pub async fn perform_init(&mut self) -> Result<(), Error> {
        // first message should be init, if not, we fail
        if let Message::Init(_) = self.read().await? {
            // ok
        } else {
            return Err(Error::FirstMessageNotInit);
        }

        // send some bs
        Ok(self
            .write(&msgs::Init {
                features: vec![0; 5],
                global_features: vec![0; 2],
                remote_network_address: None,
                networks: Some(vec![bitcoin::constants::ChainHash::BITCOIN]),
            })
            .await?)
    }

    pub async fn write<M: wire::Type + Writeable>(&mut self, m: &M) -> Result<(), io::Error> {
        let msg = self.channel.encrypt_message(m);
        self.stream.write_all(&msg).await?;
        Ok(())
    }

    pub async fn read(&mut self) -> Result<Message<()>, Error> {
        self.read_custom(|_type, _buf| Ok(None)).await
    }

    pub async fn read_custom<T>(
        &mut self,
        handler: impl FnOnce(u16, &mut Cursor<&[u8]>) -> Result<Option<T>, DecodeError>,
    ) -> Result<Message<T>, Error>
    where
        T: core::fmt::Debug,
    {
        let mut hdr = [0u8; 18];

        self.stream.read_exact(&mut hdr).await?;
        let size = self.channel.decrypt_length_header(&hdr)? as usize;
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

        let mut lnsocket = LNSocket::connect_and_init(key, their_key, "ln.damus.io:9735").await?;

        //println!("got here");
        lnsocket
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
        use crate::commando::CommandoClient;

        let key = SecretKey::new(&mut rand::thread_rng());
        let their_key = PublicKey::from_str(
            "03f3c108ccd536b8526841f0a5c58212bb9e6584a1eb493080e7c1cc34f82dad71",
        )
        .unwrap();

        let mut lnsocket = LNSocket::connect_and_init(key, their_key, "ln.damus.io:9735").await?;
        let mut commando = CommandoClient::new(
            "hfYByx-RDwdBfAK-vOWeOCDJVYlvKSioVKU_y7jccZU9MjkmbWV0aG9kPWdldGluZm8=",
        );
        let resp = commando.call(&mut lnsocket, "getinfo", vec![]).await?;

        println!("{}", serde_json::to_string(&resp).unwrap());

        Ok(())
    }
}
