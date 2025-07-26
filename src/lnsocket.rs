use crate::{Error, ln::peer_channel_encryptor::PeerChannelEncryptor};
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey, SignOnly, rand};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, lookup_host};

const ACT_TWO_SIZE: usize = 50;

pub struct LNSocket {
    secp_ctx: Secp256k1<SignOnly>,
    our_key: SecretKey,
    their_pubkey: PublicKey,
    channel: PeerChannelEncryptor,
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_connection() {
        let key = SecretKey::new(&mut rand::thread_rng());
        let their_key = PublicKey::from_str(
            "03f3c108ccd536b8526841f0a5c58212bb9e6584a1eb493080e7c1cc34f82dad71",
        )
        .unwrap();

        let lnsocket = LNSocket::connect(key, their_key, "ln.damus.io:9735").await;
        if let Err(err) = lnsocket {
            eprintln!("connection failed: {err}");
            assert!(false);
        }

        //lnsocket.commando("getinvoice", /*...*/).await
    }
}
