//! Raster payload codec (docs/raster-payload.md).

use crate::error::Error;
use crate::pixel::{predict, Rgba};

pub const TILE_SIZE: usize = 256;

const OP_INDEX: u8 = 0b0000_0000;
const OP_DIFF: u8 = 0b0100_0000;
const OP_LUMA: u8 = 0b1000_0000;
const OP_RUN: u8 = 0b1100_0000;
const OP_RGB: u8 = 0xFE;
const OP_RGBA: u8 = 0xFF;
const MAX_RUN: u32 = 62;

/// Tile rectangles in row-major order: (x, y, width, height) in pixels.
fn tile_rects(width: u32, height: u32) -> Vec<(usize, usize, usize, usize)> {
    let (w, h) = (width as usize, height as usize);
    let mut rects = Vec::new();
    let mut y = 0;
    while y < h {
        let th = TILE_SIZE.min(h - y);
        let mut x = 0;
        while x < w {
            let tw = TILE_SIZE.min(w - x);
            rects.push((x, y, tw, th));
            x += TILE_SIZE;
        }
        y += TILE_SIZE;
    }
    rects
}

/// Encode a full image into a RAST chunk payload.
pub fn encode(pixels: &[Rgba], width: u32, height: u32) -> Vec<u8> {
    let rects = tile_rects(width, height);
    let streams: Vec<Vec<u8>> = rects
        .iter()
        .map(|&(x, y, tw, th)| encode_tile(pixels, width as usize, x, y, tw, th))
        .collect();

    let mut out = Vec::new();
    out.extend_from_slice(&(streams.len() as u32).to_le_bytes());
    for s in &streams {
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    }
    for s in &streams {
        out.extend_from_slice(s);
    }
    out
}

fn encode_tile(image: &[Rgba], img_w: usize, tx: usize, ty: usize, tw: usize, th: usize) -> Vec<u8> {
    let mut tile = Vec::with_capacity(tw * th);
    for y in 0..th {
        let row = (ty + y) * img_w + tx;
        tile.extend_from_slice(&image[row..row + tw]);
    }

    let mut out = Vec::new();
    let mut index = [Rgba::default(); 64];
    let mut run: u32 = 0;

    for y in 0..th {
        for x in 0..tw {
            let cur = tile[y * tw + x];
            let p = predict(&tile, tw, x, y);

            if cur == p {
                run += 1;
                if run == MAX_RUN {
                    out.push(OP_RUN | (run - 1) as u8);
                    run = 0;
                }
            } else {
                if run > 0 {
                    out.push(OP_RUN | (run - 1) as u8);
                    run = 0;
                }
                let slot = cur.hash();
                if index[slot] == cur {
                    out.push(OP_INDEX | slot as u8);
                } else if cur.a == p.a {
                    let dr = cur.r.wrapping_sub(p.r) as i8;
                    let dg = cur.g.wrapping_sub(p.g) as i8;
                    let db = cur.b.wrapping_sub(p.b) as i8;
                    let dr_dg = dr.wrapping_sub(dg);
                    let db_dg = db.wrapping_sub(dg);
                    if (-2..=1).contains(&dr) && (-2..=1).contains(&dg) && (-2..=1).contains(&db) {
                        out.push(
                            OP_DIFF
                                | (((dr + 2) as u8) << 4)
                                | (((dg + 2) as u8) << 2)
                                | (db + 2) as u8,
                        );
                    } else if (-32..=31).contains(&dg)
                        && (-8..=7).contains(&dr_dg)
                        && (-8..=7).contains(&db_dg)
                    {
                        out.push(OP_LUMA | (dg + 32) as u8);
                        out.push((((dr_dg + 8) as u8) << 4) | (db_dg + 8) as u8);
                    } else {
                        out.extend_from_slice(&[OP_RGB, cur.r, cur.g, cur.b]);
                    }
                } else {
                    out.extend_from_slice(&[OP_RGBA, cur.r, cur.g, cur.b, cur.a]);
                }
            }

            index[cur.hash()] = cur;
        }
    }
    if run > 0 {
        out.push(OP_RUN | (run - 1) as u8);
    }
    out
}

/// Decode a RAST chunk payload into pixels for an image of the given size.
pub fn decode(data: &[u8], width: u32, height: u32) -> Result<Vec<Rgba>, Error> {
    let rects = tile_rects(width, height);

    let mut cursor = 0usize;
    let tile_count = read_u32(data, &mut cursor)? as usize;
    if tile_count != rects.len() {
        return Err(Error::BadOpStream("tile count does not match dimensions"));
    }
    let mut lengths = Vec::with_capacity(tile_count);
    for _ in 0..tile_count {
        lengths.push(read_u32(data, &mut cursor)? as usize);
    }

    let (w, h) = (width as usize, height as usize);
    let mut out = vec![Rgba::default(); w * h];
    for (&(tx, ty, tw, th), &len) in rects.iter().zip(&lengths) {
        let end = cursor.checked_add(len).ok_or(Error::Truncated)?;
        let stream = data.get(cursor..end).ok_or(Error::Truncated)?;
        cursor = end;
        let tile = decode_tile(stream, tw, th)?;
        for y in 0..th {
            let row = (ty + y) * w + tx;
            out[row..row + tw].copy_from_slice(&tile[y * tw..(y + 1) * tw]);
        }
    }
    if cursor != data.len() {
        return Err(Error::BadOpStream("trailing bytes after last tile"));
    }
    Ok(out)
}

fn decode_tile(s: &[u8], tw: usize, th: usize) -> Result<Vec<Rgba>, Error> {
    let total = tw * th;
    let mut tile = vec![Rgba::default(); total];
    let mut index = [Rgba::default(); 64];
    let mut pos = 0usize;
    let mut i = 0usize;

    while pos < total {
        let b = *s.get(i).ok_or(Error::Truncated)?;
        i += 1;

        if b == OP_RGB || b == OP_RGBA {
            let (x, y) = (pos % tw, pos / tw);
            let p = predict(&tile, tw, x, y);
            let cur = if b == OP_RGB {
                let lit = s.get(i..i + 3).ok_or(Error::Truncated)?;
                i += 3;
                Rgba::new(lit[0], lit[1], lit[2], p.a)
            } else {
                let lit = s.get(i..i + 4).ok_or(Error::Truncated)?;
                i += 4;
                Rgba::new(lit[0], lit[1], lit[2], lit[3])
            };
            tile[pos] = cur;
            index[cur.hash()] = cur;
            pos += 1;
            continue;
        }

        match b >> 6 {
            0b00 => {
                let cur = index[(b & 0x3F) as usize];
                tile[pos] = cur;
                index[cur.hash()] = cur;
                pos += 1;
            }
            0b01 => {
                let (x, y) = (pos % tw, pos / tw);
                let p = predict(&tile, tw, x, y);
                let dr = ((b >> 4) & 0x03).wrapping_sub(2);
                let dg = ((b >> 2) & 0x03).wrapping_sub(2);
                let db = (b & 0x03).wrapping_sub(2);
                let cur = Rgba::new(
                    p.r.wrapping_add(dr),
                    p.g.wrapping_add(dg),
                    p.b.wrapping_add(db),
                    p.a,
                );
                tile[pos] = cur;
                index[cur.hash()] = cur;
                pos += 1;
            }
            0b10 => {
                let b2 = *s.get(i).ok_or(Error::Truncated)?;
                i += 1;
                let (x, y) = (pos % tw, pos / tw);
                let p = predict(&tile, tw, x, y);
                let dg = (b & 0x3F).wrapping_sub(32);
                let dr = dg.wrapping_add((b2 >> 4).wrapping_sub(8));
                let db = dg.wrapping_add((b2 & 0x0F).wrapping_sub(8));
                let cur = Rgba::new(
                    p.r.wrapping_add(dr),
                    p.g.wrapping_add(dg),
                    p.b.wrapping_add(db),
                    p.a,
                );
                tile[pos] = cur;
                index[cur.hash()] = cur;
                pos += 1;
            }
            _ => {
                let n = (b & 0x3F) as usize + 1;
                if pos + n > total {
                    return Err(Error::BadOpStream("run past end of tile"));
                }
                for _ in 0..n {
                    let (x, y) = (pos % tw, pos / tw);
                    let p = predict(&tile, tw, x, y);
                    tile[pos] = p;
                    index[p.hash()] = p;
                    pos += 1;
                }
            }
        }
    }

    if i != s.len() {
        return Err(Error::BadOpStream("trailing bytes in tile stream"));
    }
    Ok(tile)
}

fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    let bytes = data.get(*cursor..*cursor + 4).ok_or(Error::Truncated)?;
    *cursor += 4;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}
