pub mod device;
pub mod flash;
pub mod hex;
pub mod hid;
pub mod protocol;
pub mod serial;

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Hex(String),
    Protocol(String),
    Device(String),
    Serial(String),
    Io(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Hex(message) => write!(f, "hex error: {message}"),
            Error::Protocol(message) => write!(f, "protocol error: {message}"),
            Error::Device(message) => write!(f, "device error: {message}"),
            Error::Serial(message) => write!(f, "serial error: {message}"),
            Error::Io(message) => write!(f, "io error: {message}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
