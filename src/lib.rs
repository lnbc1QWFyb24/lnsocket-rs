//! # LNSocket
//!
//! `lnsocket` is an async Lightning Network socket library implementing the BOLT 8 Noise handshake
//! and typed Lightning wire message framing over TCP, using `tokio`.
//!
//! This crate is a **minimal, opinionated** wrapper around [`PeerChannelEncryptor`] that:
//! - Resolves a `host:port` string into a socket address,
//! - Opens a TCP connection (no retries or built-in timeouts),
//! - Completes the three-act Noise handshake (act1, act2, act3),
//! - Optionally exchanges `init` messages ([`LNSocket::perform_init`]),
//! - Provides typed `read`/`write` helpers for Lightning wire messages.
//!
//! ## ⚠️ Notes
//! - Key management is the caller’s responsibility.
//! - This crate does **not** handle reconnect logic, backpressure, or keepalives.
//! - [`LNSocket::perform_init`] uses minimal feature negotiation by design.
//!
//! ## Related modules
//! - [`LNSocket`] — Low-level Lightning Network TCP + Noise socket
//! - [`CommandoClient`] — Simple client for [Core Lightning Commando RPC](https://docs.corelightning.org/reference/commando)
//!
//! ## Example
//! ```no_run
//! use bitcoin::secp256k1::{SecretKey, PublicKey, rand};
//! use lnsocket::LNSocket;
//!
//! # async fn demo(their_pubkey: PublicKey) -> Result<(), lnsocket::Error> {
//! let our_key = SecretKey::new(&mut rand::thread_rng());
//! let mut sock = LNSocket::connect_and_init(our_key, their_pubkey, "ln.example.com:9735").await?;
//! // write/read Lightning wire messages
//! # Ok(()) }
//! ```
//!
//! See [`CommandoClient`] for sending RPC calls over the socket.

pub mod commando;
mod crypto;
pub mod error;
pub mod ln;
pub mod lnsocket;
mod sign;
mod socket_addr;
mod util;

pub use bitcoin;
pub use commando::CommandoClient;
pub use error::Error;
pub use lnsocket::LNSocket;

mod prelude {
    #![allow(unused_imports)]

    pub use std::{boxed::Box, collections::VecDeque, string::String, vec, vec::Vec};

    pub use std::borrow::ToOwned;
    pub use std::string::ToString;

    pub use core::convert::{AsMut, AsRef, TryFrom, TryInto};
    pub use core::default::Default;
    pub use core::marker::Sized;

    pub(crate) use crate::util::hash_tables::*;
}

#[doc(hidden)]
/// IO utilities public only for use by in-crate macros. These should not be used externally
///
/// This is not exported to bindings users as it is not intended for public consumption.
pub mod io_extras {
    use std::io::{self, Read, Write};

    /// Creates an instance of a writer which will successfully consume all data.
    pub use std::io::sink;

    pub fn copy<R: Read + ?Sized, W: Write + ?Sized>(
        reader: &mut R,
        writer: &mut W,
    ) -> Result<u64, io::Error> {
        let mut count = 0;
        let mut buf = [0u8; 64];

        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    writer.write_all(&buf[0..n])?;
                    count += n as u64;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            };
        }
        Ok(count)
    }

    pub fn read_to_end<D: Read>(d: &mut D) -> Result<std::vec::Vec<u8>, io::Error> {
        let mut result = vec![];
        let mut buf = [0u8; 64];
        loop {
            match d.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => result.extend_from_slice(&buf[0..n]),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            };
        }
        Ok(result)
    }
}
