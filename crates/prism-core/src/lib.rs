//! Reference implementation of the PRISM image format.
//! Normative documents: docs/container.md and docs/raster-payload.md.

pub mod container;
pub mod crc;
pub mod error;
pub mod pixel;
pub mod raster;
pub mod reconstruct;

pub use container::{decode_file, encode_file, Header, Image, PayloadKind};
pub use error::Error;
pub use pixel::Rgba;
