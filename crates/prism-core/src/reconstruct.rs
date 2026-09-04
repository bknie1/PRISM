//! Reconstruction scaler (docs/reconstruction-spec.md). Integer-only Catmull-Rom.

use crate::container::Image;
use crate::error::Error;
use crate::pixel::Rgba;

const ONE: i64 = 65536;

/// Per-output-position taps and Q16 weights along one axis.
struct AxisStep {
    taps: [usize; 4],
    weights: [i64; 4],
}

fn axis_steps(len_in: u32, len_out: u32) -> Vec<AxisStep> {
    let (n_in, n_out) = (len_in as i64, len_out as i64);
    (0..n_out)
        .map(|x| {
            let sx = ((2 * x + 1) * n_in * ONE) / (2 * n_out) - ONE / 2;
            let base = sx >> 16;
            let t = sx & 0xFFFF;
            let t2 = (t * t) >> 16;
            let t3 = (t2 * t) >> 16;
            let w0 = (-t3 + 2 * t2 - t) >> 1;
            let mut w1 = (3 * t3 - 5 * t2 + 2 * ONE) >> 1;
            let w2 = (-3 * t3 + 4 * t2 + t) >> 1;
            let w3 = (t3 - t2) >> 1;
            w1 += ONE - (w0 + w1 + w2 + w3);

            let clamp = |v: i64| v.clamp(0, n_in - 1) as usize;
            AxisStep {
                taps: [clamp(base - 1), clamp(base), clamp(base + 1), clamp(base + 2)],
                weights: [w0, w1, w2, w3],
            }
        })
        .collect()
}

/// Scale an image to (w_out, h_out) with the mandated kernel.
/// Version 1 accepts only target sizes at or above the source size.
pub fn scale(image: &Image, w_out: u32, h_out: u32) -> Result<Image, Error> {
    if w_out < image.width || h_out < image.height {
        return Err(Error::BadHeader("version 1 reconstruction does not minify"));
    }
    if w_out == 0 || h_out == 0 {
        return Err(Error::BadHeader("zero output dimension"));
    }

    let cols = axis_steps(image.width, w_out);
    let rows = axis_steps(image.height, h_out);
    let src_w = image.width as usize;

    let mut out = Vec::with_capacity(w_out as usize * h_out as usize);
    for row in &rows {
        for col in &cols {
            let mut acc = [0i64; 4];
            for (j, &ty) in row.taps.iter().enumerate() {
                let wy = row.weights[j];
                for (i, &tx) in col.taps.iter().enumerate() {
                    let w = col.weights[i] * wy;
                    let p = image.pixels[ty * src_w + tx];
                    acc[0] += p.r as i64 * w;
                    acc[1] += p.g as i64 * w;
                    acc[2] += p.b as i64 * w;
                    acc[3] += p.a as i64 * w;
                }
            }
            let ch = |v: i64| ((v + (1 << 31)) >> 32).clamp(0, 255) as u8;
            out.push(Rgba::new(ch(acc[0]), ch(acc[1]), ch(acc[2]), ch(acc[3])));
        }
    }

    Ok(Image { width: w_out, height: h_out, pixels: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32, f: impl Fn(u32, u32) -> Rgba) -> Image {
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                pixels.push(f(x, y));
            }
        }
        Image { width, height, pixels }
    }

    #[test]
    fn identity_scale_is_bit_exact() {
        let img = image(37, 23, |x, y| Rgba::new(x as u8, y as u8, (x * y) as u8, 255));
        let scaled = scale(&img, 37, 23).unwrap();
        assert_eq!(scaled, img);
    }

    #[test]
    fn flat_image_stays_flat() {
        let img = image(16, 16, |_, _| Rgba::new(120, 7, 200, 33));
        let scaled = scale(&img, 100, 57).unwrap();
        assert!(scaled.pixels.iter().all(|&p| p == Rgba::new(120, 7, 200, 33)));
    }

    #[test]
    fn symmetric_input_gives_symmetric_output() {
        let img = image(16, 16, |x, _| {
            let d = if x < 8 { x } else { 15 - x };
            Rgba::new((d * 30) as u8, 0, 0, 255)
        });
        let scaled = scale(&img, 64, 16).unwrap();
        for y in 0..16usize {
            for x in 0..64usize {
                let a = scaled.pixels[y * 64 + x];
                let b = scaled.pixels[y * 64 + (63 - x)];
                assert_eq!(a, b, "asymmetry at ({x}, {y})");
            }
        }
    }

    #[test]
    fn minification_is_rejected() {
        let img = image(16, 16, |_, _| Rgba::OPAQUE_BLACK);
        assert!(scale(&img, 8, 16).is_err());
    }
}
