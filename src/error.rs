use crate::ln::msgs::{DecodeError, LightningError};
use std::fmt;
use std::io;
use std::net::AddrParseError;

#[derive(Debug, Clone)]
pub enum Error {
    NotConnected,
    FirstMessageNotInit,
    DnsError,
    Io(io::ErrorKind),
    Json,
    Lightning(LightningError),
    Decode(DecodeError),
    AddrParse(std::net::AddrParseError),
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
            Error::AddrParse(err) => write!(f, "Address parse error: {}", err),
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

impl From<AddrParseError> for Error {
    fn from(err: AddrParseError) -> Self {
        Self::AddrParse(err)
    }
}
