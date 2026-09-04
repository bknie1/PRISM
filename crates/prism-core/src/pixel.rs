//! Pixel type and the MED predictor (docs/raster-payload.md, "Prediction").

/// One 8-bit RGBA pixel, straight alpha.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const OPAQUE_BLACK: Rgba = Rgba { r: 0, g: 0, b: 0, a: 255 };

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Rgba { r, g, b, a }
    }

    /// Index-table slot for this pixel: (r*3 + g*5 + b*7 + a*11) % 64.
    pub fn hash(self) -> usize {
        (self.r as usize * 3 + self.g as usize * 5 + self.b as usize * 7 + self.a as usize * 11) % 64
    }
}

/// Median edge detector for one channel.
fn med(l: u8, u: u8, ul: u8) -> u8 {
    let (lo, hi) = if l < u { (l, u) } else { (u, l) };
    if ul >= hi {
        lo
    } else if ul <= lo {
        hi
    } else {
        (l as i16 + u as i16 - ul as i16) as u8
    }
}

/// Prediction for the pixel at tile-local (x, y). `pixels` is the tile buffer,
/// row-major with row stride `width`; only positions before (x, y) are read.
pub fn predict(pixels: &[Rgba], width: usize, x: usize, y: usize) -> Rgba {
    let idx = y * width + x;
    if x == 0 && y == 0 {
        return Rgba::OPAQUE_BLACK;
    }
    if y == 0 {
        return pixels[idx - 1];
    }
    if x == 0 {
        return pixels[idx - width];
    }
    let l = pixels[idx - 1];
    let u = pixels[idx - width];
    let ul = pixels[idx - width - 1];
    Rgba {
        r: med(l.r, u.r, ul.r),
        g: med(l.g, u.g, ul.g),
        b: med(l.b, u.b, ul.b),
        a: med(l.a, u.a, ul.a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn med_selects_bound_on_edges() {
        assert_eq!(med(10, 50, 60), 10);
        assert_eq!(med(10, 50, 5), 50);
        assert_eq!(med(10, 50, 30), 30);
    }

    #[test]
    fn med_gradient_stays_in_channel_range() {
        assert_eq!(med(200, 250, 210), 240);
        assert_eq!(med(0, 255, 100), 155);
    }
}
