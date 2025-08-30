use crate::ln::msgs::{DecodeError, LightningError};
use crate::socket_addr::SocketAddressParseError;
use serde::Deserialize;
use std::fmt;
use std::io;

/// Errors surfaced by this crate.
///
/// Notably, I/O failures are reduced to an `io::ErrorKind` so the type stays small and Send/Sync.
/// For example, DNS resolution failure maps to `Error::DnsError`, other I/O paths map to
/// `Error::Io(kind)`.
///
/// Display is human-readable; use pattern matching if you need to branch on kinds.
#[derive(Debug, Clone)]
pub enum Error {
    NotConnected,
    FirstMessageNotInit,
    DnsError,
    Io(io::ErrorKind),
    Json,
    Lightning(LightningError),
    Decode(DecodeError),
    AddrParse(SocketAddressParseError),
    Rpc(RpcError),
    ProxyConnection(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotConnected => write!(f, "Not connected to server"),
            Error::FirstMessageNotInit => write!(f, "First message was not init"),
            Error::DnsError => write!(f, "Failed to resolve hostname"),
            Error::Io(kind) => write!(f, "I/O error: {}", kind),
            Error::Lightning(err) => write!(f, "Lightning error: {:?}", err),
            Error::Decode(err) => write!(f, "decoding error: {:?}", err),
            Error::Json => write!(f, "json error"),
            Error::AddrParse(err) => write!(f, "Address parse error: {err}"),
            Error::Rpc(err) => write!(f, "commando rpc error: {err:?}"),
            Error::ProxyConnection(msg) => write!(f, "TOR connection error: {msg}"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err.kind())
    }
}

impl From<serde_json::Error> for Error {
    fn from(_err: serde_json::Error) -> Self {
        Self::Json
    }
}

impl From<DecodeError> for Error {
    fn from(decode: DecodeError) -> Self {
        Self::Decode(decode)
    }
}

impl From<LightningError> for Error {
    fn from(lnerr: LightningError) -> Self {
        Self::Lightning(lnerr)
    }
}

impl From<SocketAddressParseError> for Error {
    fn from(err: SocketAddressParseError) -> Self {
        Self::AddrParse(err)
    }
}
