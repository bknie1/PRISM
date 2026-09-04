//! Reference implementation of the PRISM image format.
//! Normative documents: docs/container.md and docs/raster-payload.md.

pub mod container;
pub mod crc;
pub mod error;
pub mod pixel;
pub mod raster;
pub mod reconstruct;
pub mod vector;
#[cfg(feature = "encryption")]
pub mod crypto;

pub use container::{
    decode_file, decode_payload, encode_file, encode_vector_file, FilePayload, Header, Image,
    PayloadKind,
};
pub use error::Error;
pub use pixel::Rgba;
