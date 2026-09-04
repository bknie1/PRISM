//! ENCR wrapper: ChaCha20-Poly1305 over the serialized payload chunk
//! (docs/container.md, "ENCR"). Compiled only with the `encryption` feature.

use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

use crate::container::{read_chunks, write_chunk, MAGIC, VERSION};
use crate::error::Error;

const ALG_CHACHA20_POLY1305: u8 = 0;

/// Rewrite a plain PRISM file as an encrypted one: the payload chunk moves
/// inside an ENCR chunk, authenticated against the HEAD payload.
pub fn encrypt_file(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, Error> {
    let (_, chunks) = read_chunks(data)?;
    let head = &chunks[0];
    let payload_chunk = chunks
        .iter()
        .find(|c| c.ty == *b"RAST" || c.ty == *b"VECT")
        .ok_or(Error::MissingChunk("RAST or VECT"))?;

    let mut plain = Vec::new();
    write_chunk(&mut plain, &payload_chunk.ty, payload_chunk.payload);

    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, Payload { msg: &plain, aad: head.payload })
        .map_err(|_| Error::CryptoFailed)?;

    let mut encr = Vec::with_capacity(13 + ciphertext.len());
    encr.push(ALG_CHACHA20_POLY1305);
    encr.extend_from_slice(&nonce);
    encr.extend_from_slice(&ciphertext);

    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.push(VERSION);
    write_chunk(&mut out, b"HEAD", head.payload);
    write_chunk(&mut out, b"ENCR", &encr);
    write_chunk(&mut out, b"END ", &[]);
    Ok(out)
}

/// Reverse of `encrypt_file`: returns the plain PRISM file bytes.
pub fn decrypt_file(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, Error> {
    let (_, chunks) = read_chunks(data)?;
    let head = &chunks[0];
    let encr = chunks
        .iter()
        .find(|c| c.ty == *b"ENCR")
        .ok_or(Error::MissingChunk("ENCR"))?;

    if encr.payload.len() < 13 {
        return Err(Error::Truncated);
    }
    if encr.payload[0] != ALG_CHACHA20_POLY1305 {
        return Err(Error::BadHeader("unknown encryption algorithm"));
    }
    let nonce = Nonce::from_slice(&encr.payload[1..13]);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let plain = cipher
        .decrypt(nonce, Payload { msg: &encr.payload[13..], aad: head.payload })
        .map_err(|_| Error::CryptoFailed)?;

    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.push(VERSION);
    write_chunk(&mut out, b"HEAD", head.payload);
    out.extend_from_slice(&plain);
    write_chunk(&mut out, b"END ", &[]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::{decode_file, encode_file, Image};
    use crate::pixel::Rgba;

    fn sample() -> Image {
        let mut pixels = Vec::new();
        for y in 0..40u32 {
            for x in 0..40u32 {
                pixels.push(Rgba::new(x as u8 * 6, y as u8 * 6, 99, 255));
            }
        }
        Image { width: 40, height: 40, pixels }
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let img = sample();
        let plain = encode_file(&img);
        let key = [7u8; 32];
        let enc = encrypt_file(&plain, &key).unwrap();
        assert!(decode_file(&enc).is_err(), "encrypted file must not decode");
        let dec = decrypt_file(&enc, &key).unwrap();
        assert_eq!(decode_file(&dec).unwrap(), img);
    }

    #[test]
    fn wrong_key_fails() {
        let plain = encode_file(&sample());
        let enc = encrypt_file(&plain, &[7u8; 32]).unwrap();
        assert_eq!(decrypt_file(&enc, &[8u8; 32]), Err(Error::CryptoFailed));
    }

    #[test]
    fn tampered_header_fails_authentication() {
        let plain = encode_file(&sample());
        let key = [7u8; 32];
        let enc = encrypt_file(&plain, &key).unwrap();
        // Re-sign a modified HEAD so the CRC passes and only AEAD can object.
        let (_, chunks) = read_chunks(&enc).unwrap();
        let mut head = chunks[0].payload.to_vec();
        head[0] ^= 1;
        let encr = chunks.iter().find(|c| c.ty == *b"ENCR").unwrap();
        let mut forged = Vec::new();
        forged.extend_from_slice(&MAGIC);
        forged.push(VERSION);
        write_chunk(&mut forged, b"HEAD", &head);
        write_chunk(&mut forged, b"ENCR", encr.payload);
        write_chunk(&mut forged, b"END ", &[]);
        assert_eq!(decrypt_file(&forged, &key), Err(Error::CryptoFailed));
    }
}
