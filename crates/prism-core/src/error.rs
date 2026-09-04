//! Error type for decoding and validation.

use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    BadMagic,
    UnsupportedVersion(u8),
    Truncated,
    CrcMismatch { chunk: [u8; 4] },
    MissingChunk(&'static str),
    BadHeader(&'static str),
    BadOpStream(&'static str),
    BadVector(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadMagic => write!(f, "not a PRISM file (bad magic bytes)"),
            Error::UnsupportedVersion(v) => write!(f, "unsupported format version {v}"),
            Error::Truncated => write!(f, "file ends before the data it declares"),
            Error::CrcMismatch { chunk } => {
                write!(f, "CRC mismatch in chunk {}", String::from_utf8_lossy(chunk))
            }
            Error::MissingChunk(t) => write!(f, "required chunk {t} missing"),
            Error::BadHeader(msg) => write!(f, "invalid header: {msg}"),
            Error::BadOpStream(msg) => write!(f, "invalid raster op stream: {msg}"),
            Error::BadVector(msg) => write!(f, "invalid vector payload: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
