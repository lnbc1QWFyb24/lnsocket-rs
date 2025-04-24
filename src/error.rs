use crate::ln::msgs::LightningError;
use std::fmt;
use std::io;
use std::net::AddrParseError;

#[derive(Debug, Clone)]
pub enum Error {
    NotConnected,
    DnsError,
    Io(io::ErrorKind),
    Lightning(LightningError),
    AddrParse(std::net::AddrParseError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotConnected => write!(f, "Not connected to server"),
            Error::DnsError => write!(f, "Failed to resolve hostname"),
            Error::Io(kind) => write!(f, "I/O error: {}", kind),
            Error::Lightning(err) => write!(f, "Lightning error: {:?}", err),
            Error::AddrParse(err) => write!(f, "Address parse error: {}", err),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err.kind())
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
