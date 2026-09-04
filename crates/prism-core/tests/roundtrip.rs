//! Round-trip tests over synthetic images (SPEC.md, Phase 1).

use prism_core::{decode_file, encode_file, Error, Image, Rgba};

fn image(width: u32, height: u32, f: impl Fn(u32, u32) -> Rgba) -> Image {
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.push(f(x, y));
        }
    }
    Image { width, height, pixels }
}

fn assert_roundtrip(img: &Image) {
    let encoded = encode_file(img);
    let decoded = decode_file(&encoded).expect("decode failed");
    assert_eq!(&decoded, img);
}

/// Deterministic pseudo-random byte stream.
struct Lcg(u64);
impl Lcg {
    fn next_u8(&mut self) -> u8 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 56) as u8
    }
}

#[test]
fn solid_color() {
    assert_roundtrip(&image(64, 64, |_, _| Rgba::new(120, 40, 200, 255)));
}

#[test]
fn single_pixel() {
    assert_roundtrip(&image(1, 1, |_, _| Rgba::new(1, 2, 3, 4)));
}

#[test]
fn horizontal_gradient() {
    assert_roundtrip(&image(300, 40, |x, _| Rgba::new(x as u8, (x / 2) as u8, 255 - x as u8, 255)));
}

#[test]
fn diagonal_gradient_crossing_tiles() {
    assert_roundtrip(&image(300, 300, |x, y| {
        Rgba::new((x + y) as u8, (x * 2) as u8, (y * 3) as u8, 255)
    }));
}

#[test]
fn exact_tile_boundary() {
    assert_roundtrip(&image(256, 256, |x, y| Rgba::new(x as u8, y as u8, 7, 255)));
}

#[test]
fn one_past_tile_boundary() {
    assert_roundtrip(&image(257, 257, |x, y| Rgba::new(x as u8, y as u8, 7, 255)));
}

#[test]
fn tall_thin() {
    assert_roundtrip(&image(3, 600, |x, y| Rgba::new((x * 80) as u8, y as u8, 0, 255)));
}

#[test]
fn random_noise() {
    let mut rng = Lcg(42);
    let mut pixels = Vec::new();
    for _ in 0..(200 * 100) {
        pixels.push(Rgba::new(rng.next_u8(), rng.next_u8(), rng.next_u8(), rng.next_u8()));
    }
    assert_roundtrip(&Image { width: 200, height: 100, pixels });
}

#[test]
fn alpha_gradient() {
    assert_roundtrip(&image(100, 100, |x, y| Rgba::new(10, 20, 30, (x + y) as u8)));
}

#[test]
fn checkerboard() {
    assert_roundtrip(&image(130, 130, |x, y| {
        if (x + y) % 2 == 0 {
            Rgba::new(255, 255, 255, 255)
        } else {
            Rgba::new(0, 0, 0, 255)
        }
    }));
}

#[test]
fn corrupt_byte_is_caught_by_crc() {
    let img = image(64, 64, |x, y| Rgba::new(x as u8, y as u8, 0, 255));
    let mut encoded = encode_file(&img);
    let mid = encoded.len() / 2;
    encoded[mid] ^= 0x01;
    match decode_file(&encoded) {
        Err(Error::CrcMismatch { .. }) | Err(Error::Truncated) | Err(Error::BadOpStream(_)) => {}
        other => panic!("corruption not detected: {other:?}"),
    }
}

#[test]
fn truncated_file_is_an_error() {
    let img = image(64, 64, |x, y| Rgba::new(x as u8, y as u8, 0, 255));
    let encoded = encode_file(&img);
    let truncated = &encoded[..encoded.len() - 10];
    assert!(decode_file(truncated).is_err());
}

#[test]
fn bad_magic_is_an_error() {
    assert_eq!(decode_file(b"NOPE\x01rest"), Err(Error::BadMagic));
}
