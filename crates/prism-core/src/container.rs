//! Container reader and writer (docs/container.md).

use crate::crc::crc32_parts;
use crate::error::Error;
use crate::pixel::Rgba;
use crate::raster;

const MAGIC: [u8; 4] = *b"PRSM";
const VERSION: u8 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PayloadKind {
    Raster,
    Vector,
}

#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub width: u32,
    pub height: u32,
    pub payload_kind: PayloadKind,
    pub bit_depth: u8,
    pub colorspace: u8,
    pub alpha_mode: u8,
    pub tile_size_log2: u8,
    pub kernel: u8,
}

/// A decoded image: 8-bit RGBA, row-major.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Rgba>,
}

/// Encode an image as a raster-payload PRISM file.
pub fn encode_file(image: &Image) -> Vec<u8> {
    assert_eq!(
        image.pixels.len(),
        image.width as usize * image.height as usize,
        "pixel buffer does not match dimensions"
    );

    let mut head = Vec::with_capacity(14);
    head.extend_from_slice(&image.width.to_le_bytes());
    head.extend_from_slice(&image.height.to_le_bytes());
    head.extend_from_slice(&[0, 8, 0, 0, 8, 0]);

    let payload = raster::encode(&image.pixels, image.width, image.height);

    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.push(VERSION);
    write_chunk(&mut out, b"HEAD", &head);
    write_chunk(&mut out, b"RAST", &payload);
    write_chunk(&mut out, b"END ", &[]);
    out
}

fn write_chunk(out: &mut Vec<u8>, ty: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(ty);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    out.extend_from_slice(&crc32_parts(&[ty, payload]).to_le_bytes());
}

/// Decode a PRISM file. Verifies every chunk CRC; skips unknown chunk types.
pub fn decode_file(data: &[u8]) -> Result<Image, Error> {
    let (header, chunks) = read_chunks(data)?;

    let payload = match header.payload_kind {
        PayloadKind::Raster => chunks
            .iter()
            .find(|c| c.ty == *b"RAST")
            .ok_or(Error::MissingChunk("RAST"))?,
        PayloadKind::Vector => return Err(Error::BadHeader("vector payloads not implemented yet")),
    };

    let pixels = raster::decode(payload.payload, header.width, header.height)?;
    Ok(Image {
        width: header.width,
        height: header.height,
        pixels,
    })
}

/// Parse and validate the file, returning the header and all chunks after HEAD.
pub fn read_chunks(data: &[u8]) -> Result<(Header, Vec<Chunk<'_>>), Error> {
    if data.len() < 5 {
        return Err(Error::Truncated);
    }
    if data[0..4] != MAGIC {
        return Err(Error::BadMagic);
    }
    if data[4] != VERSION {
        return Err(Error::UnsupportedVersion(data[4]));
    }

    let mut cursor = 5usize;
    let mut chunks = Vec::new();
    let mut saw_end = false;
    while cursor < data.len() {
        let chunk = read_chunk(data, &mut cursor)?;
        if chunk.ty == *b"END " {
            saw_end = true;
            if cursor != data.len() {
                return Err(Error::BadHeader("data after END chunk"));
            }
            break;
        }
        chunks.push(chunk);
    }
    if !saw_end {
        return Err(Error::MissingChunk("END "));
    }

    let head = match chunks.first() {
        Some(c) if c.ty == *b"HEAD" => c,
        _ => return Err(Error::MissingChunk("HEAD")),
    };
    let header = parse_header(head.payload)?;
    Ok((header, chunks))
}

pub struct Chunk<'a> {
    pub ty: [u8; 4],
    pub payload: &'a [u8],
}

fn read_chunk<'a>(data: &'a [u8], cursor: &mut usize) -> Result<Chunk<'a>, Error> {
    let ty: [u8; 4] = data
        .get(*cursor..*cursor + 4)
        .ok_or(Error::Truncated)?
        .try_into()
        .unwrap();
    let len_bytes = data.get(*cursor + 4..*cursor + 8).ok_or(Error::Truncated)?;
    let len = u32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
    let body_start = *cursor + 8;
    let body_end = body_start.checked_add(len).ok_or(Error::Truncated)?;
    let payload = data.get(body_start..body_end).ok_or(Error::Truncated)?;
    let crc_bytes = data.get(body_end..body_end + 4).ok_or(Error::Truncated)?;
    let stored = u32::from_le_bytes(crc_bytes.try_into().unwrap());
    if crc32_parts(&[&ty, payload]) != stored {
        return Err(Error::CrcMismatch { chunk: ty });
    }
    *cursor = body_end + 4;
    Ok(Chunk { ty, payload })
}

fn parse_header(payload: &[u8]) -> Result<Header, Error> {
    if payload.len() != 14 {
        return Err(Error::BadHeader("HEAD must be 14 bytes in version 1"));
    }
    let width = u32::from_le_bytes(payload[0..4].try_into().unwrap());
    let height = u32::from_le_bytes(payload[4..8].try_into().unwrap());
    if width == 0 || height == 0 {
        return Err(Error::BadHeader("zero dimension"));
    }
    (width as u64)
        .checked_mul(height as u64)
        .filter(|&n| n <= 1 << 32)
        .ok_or(Error::BadHeader("image too large"))?;

    let payload_kind = match payload[8] {
        0 => PayloadKind::Raster,
        1 => PayloadKind::Vector,
        _ => return Err(Error::BadHeader("unknown payload kind")),
    };
    if payload[9] != 8 {
        return Err(Error::BadHeader("only 8-bit depth in version 1"));
    }
    if payload[10] != 0 {
        return Err(Error::BadHeader("only sRGB in version 1"));
    }
    if payload[11] != 0 {
        return Err(Error::BadHeader("only straight alpha in version 1"));
    }
    if payload[12] != 8 {
        return Err(Error::BadHeader("tile_size_log2 must be 8 in version 1"));
    }
    if payload[13] != 0 {
        return Err(Error::BadHeader("unknown reconstruction kernel"));
    }

    Ok(Header {
        width,
        height,
        payload_kind,
        bit_depth: 8,
        colorspace: 0,
        alpha_mode: 0,
        tile_size_log2: 8,
        kernel: 0,
    })
}
