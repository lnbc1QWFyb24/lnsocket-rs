use crate::ln::msgs::LightningError;
use std::io;
use std::net::AddrParseError;

#[derive(Debug, Clone)]
pub enum Error {
    NotConnected,
    Io(io::ErrorKind),
    Lightning(LightningError),
    AddrParse(std::net::AddrParseError),
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
