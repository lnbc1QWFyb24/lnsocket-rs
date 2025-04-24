use crate::{Error, ln::peer_channel_encryptor::PeerChannelEncryptor};
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey, SignOnly, rand};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpSocket;

const ACT_TWO_SIZE: usize = 50;

pub struct LNSocket {
    secp_ctx: Secp256k1<SignOnly>,
    our_key: SecretKey,
    their_pubkey: PublicKey,
    channel: PeerChannelEncryptor,
}

impl LNSocket {
    pub async fn connect_and_init(
        our_key: SecretKey,
        their_pubkey: PublicKey,
        addr: &str,
    ) -> Result<LNSocket, Error> {
        let secp_ctx = Secp256k1::signing_only();

        let addr: SocketAddr = addr.parse()?;
        let socket = TcpSocket::new_v4()?;
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
