// This file is Copyright its original authors, visible in version control
// history.
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.

pub mod commando;
mod crypto;
pub mod error;
mod ln;
pub mod lnsocket;
mod sign;
mod socket_addr;
mod util;

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
                Err(e) => return Err(e.into()),
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
                Err(e) => return Err(e.into()),
            };
        }
        Ok(result)
    }
}
