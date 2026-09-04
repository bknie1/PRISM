//! Vector payload: encode, decode, and the reference rasterizer
//! (docs/vector-payload.md).

use crate::error::Error;
use crate::pixel::Rgba;

const ONE: i64 = 65536;
const SS: i64 = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Flat { color: u32 },
    Linear { x0: i64, y0: i64, x1: i64, y1: i64, c0: u32, c1: u32 },
    Radial { cx: i64, cy: i64, ex: i64, ey: i64, c0: u32, c1: u32 },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PathCmd {
    MoveTo(i64, i64),
    LineTo(i64, i64),
    QuadTo(i64, i64, i64, i64),
    CubicTo(i64, i64, i64, i64, i64, i64),
    Close,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Shape {
    pub style: u32,
    pub path: u32,
    pub fill_rule: FillRule,
}

/// In-memory vector art. Path coordinates are absolute, in fixed-point
/// units with `scale` fractional bits; the wire format stores deltas.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VectorImage {
    pub scale: u8,
    pub colors: Vec<Rgba>,
    pub styles: Vec<Style>,
    pub paths: Vec<Vec<PathCmd>>,
    pub shapes: Vec<Shape>,
}

// ---- wire helpers ----

fn put_varuint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn put_varint(out: &mut Vec<u8>, v: i64) {
    put_varuint(out, ((v << 1) ^ (v >> 63)) as u64);
}

fn get_varuint(data: &[u8], cursor: &mut usize) -> Result<u64, Error> {
    let mut v = 0u64;
    let mut shift = 0u32;
    loop {
        let byte = *data.get(*cursor).ok_or(Error::Truncated)?;
        *cursor += 1;
        if shift >= 63 && byte > 1 {
            return Err(Error::BadVector("varuint too large"));
        }
        v |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(v);
        }
        shift += 7;
    }
}

fn get_varint(data: &[u8], cursor: &mut usize) -> Result<i64, Error> {
    let z = get_varuint(data, cursor)?;
    Ok(((z >> 1) as i64) ^ -((z & 1) as i64))
}

// ---- encode ----

pub fn encode(art: &VectorImage) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(art.scale);

    put_varuint(&mut out, art.colors.len() as u64);
    for c in &art.colors {
        out.extend_from_slice(&[c.r, c.g, c.b, c.a]);
    }

    put_varuint(&mut out, art.styles.len() as u64);
    for s in &art.styles {
        match *s {
            Style::Flat { color } => {
                out.push(0);
                put_varuint(&mut out, color as u64);
            }
            Style::Linear { x0, y0, x1, y1, c0, c1 } => {
                out.push(1);
                for v in [x0, y0, x1, y1] {
                    put_varint(&mut out, v);
                }
                put_varuint(&mut out, c0 as u64);
                put_varuint(&mut out, c1 as u64);
            }
            Style::Radial { cx, cy, ex, ey, c0, c1 } => {
                out.push(2);
                for v in [cx, cy, ex, ey] {
                    put_varint(&mut out, v);
                }
                put_varuint(&mut out, c0 as u64);
                put_varuint(&mut out, c1 as u64);
            }
        }
    }

    put_varuint(&mut out, art.paths.len() as u64);
    for path in &art.paths {
        put_varuint(&mut out, path.len() as u64);
        let (mut cx, mut cy) = (0i64, 0i64);
        for cmd in path {
            match *cmd {
                PathCmd::MoveTo(x, y) => {
                    out.push(0);
                    put_varint(&mut out, x - cx);
                    put_varint(&mut out, y - cy);
                    (cx, cy) = (x, y);
                }
                PathCmd::LineTo(x, y) => {
                    out.push(1);
                    put_varint(&mut out, x - cx);
                    put_varint(&mut out, y - cy);
                    (cx, cy) = (x, y);
                }
                PathCmd::QuadTo(qx, qy, x, y) => {
                    out.push(2);
                    put_varint(&mut out, qx - cx);
                    put_varint(&mut out, qy - cy);
                    put_varint(&mut out, x - qx);
                    put_varint(&mut out, y - qy);
                    (cx, cy) = (x, y);
                }
                PathCmd::CubicTo(ax, ay, bx, by, x, y) => {
                    out.push(3);
                    put_varint(&mut out, ax - cx);
                    put_varint(&mut out, ay - cy);
                    put_varint(&mut out, bx - ax);
                    put_varint(&mut out, by - ay);
                    put_varint(&mut out, x - bx);
                    put_varint(&mut out, y - by);
                    (cx, cy) = (x, y);
                }
                PathCmd::Close => out.push(4),
            }
        }
    }

    put_varuint(&mut out, art.shapes.len() as u64);
    for sh in &art.shapes {
        put_varuint(&mut out, sh.style as u64);
        put_varuint(&mut out, sh.path as u64);
        out.push(match sh.fill_rule {
            FillRule::NonZero => 0,
            FillRule::EvenOdd => 1,
        });
    }
    out
}

// ---- decode ----

pub fn decode(data: &[u8]) -> Result<VectorImage, Error> {
    let mut cur = 0usize;
    let scale = *data.first().ok_or(Error::Truncated)?;
    cur += 1;
    if scale > 15 {
        return Err(Error::BadVector("scale above 15"));
    }

    let color_count = get_varuint(data, &mut cur)? as usize;
    if color_count.checked_mul(4).is_none_or(|n| cur + n > data.len()) {
        return Err(Error::BadVector("color pool exceeds payload"));
    }
    let mut colors = Vec::with_capacity(color_count);
    for _ in 0..color_count {
        colors.push(Rgba::new(data[cur], data[cur + 1], data[cur + 2], data[cur + 3]));
        cur += 4;
    }

    let style_count = get_varuint(data, &mut cur)? as usize;
    let color_idx = |v: u64| -> Result<u32, Error> {
        if (v as usize) < color_count {
            Ok(v as u32)
        } else {
            Err(Error::BadVector("color index out of range"))
        }
    };
    let mut styles = Vec::with_capacity(style_count.min(1024));
    for _ in 0..style_count {
        let kind = *data.get(cur).ok_or(Error::Truncated)?;
        cur += 1;
        styles.push(match kind {
            0 => Style::Flat { color: color_idx(get_varuint(data, &mut cur)?)? },
            1 | 2 => {
                let mut g = [0i64; 4];
                for v in &mut g {
                    *v = get_varint(data, &mut cur)?;
                }
                let c0 = color_idx(get_varuint(data, &mut cur)?)?;
                let c1 = color_idx(get_varuint(data, &mut cur)?)?;
                if kind == 1 {
                    Style::Linear { x0: g[0], y0: g[1], x1: g[2], y1: g[3], c0, c1 }
                } else {
                    Style::Radial { cx: g[0], cy: g[1], ex: g[2], ey: g[3], c0, c1 }
                }
            }
            _ => return Err(Error::BadVector("unknown style kind")),
        });
    }

    let path_count = get_varuint(data, &mut cur)? as usize;
    let mut paths = Vec::with_capacity(path_count.min(1024));
    for _ in 0..path_count {
        let instr_count = get_varuint(data, &mut cur)? as usize;
        let mut cmds = Vec::with_capacity(instr_count.min(4096));
        let (mut cx, mut cy) = (0i64, 0i64);
        let mut has_current = false;
        for _ in 0..instr_count {
            let op = *data.get(cur).ok_or(Error::Truncated)?;
            cur += 1;
            if op != 0 && op != 4 && !has_current {
                return Err(Error::BadVector("draw before move_to"));
            }
            match op {
                0 => {
                    cx += get_varint(data, &mut cur)?;
                    cy += get_varint(data, &mut cur)?;
                    cmds.push(PathCmd::MoveTo(cx, cy));
                    has_current = true;
                }
                1 => {
                    cx += get_varint(data, &mut cur)?;
                    cy += get_varint(data, &mut cur)?;
                    cmds.push(PathCmd::LineTo(cx, cy));
                }
                2 => {
                    let qx = cx + get_varint(data, &mut cur)?;
                    let qy = cy + get_varint(data, &mut cur)?;
                    cx = qx + get_varint(data, &mut cur)?;
                    cy = qy + get_varint(data, &mut cur)?;
                    cmds.push(PathCmd::QuadTo(qx, qy, cx, cy));
                }
                3 => {
                    let ax = cx + get_varint(data, &mut cur)?;
                    let ay = cy + get_varint(data, &mut cur)?;
                    let bx = ax + get_varint(data, &mut cur)?;
                    let by = ay + get_varint(data, &mut cur)?;
                    cx = bx + get_varint(data, &mut cur)?;
                    cy = by + get_varint(data, &mut cur)?;
                    cmds.push(PathCmd::CubicTo(ax, ay, bx, by, cx, cy));
                }
                4 => cmds.push(PathCmd::Close),
                _ => return Err(Error::BadVector("unknown path opcode")),
            }
        }
        paths.push(cmds);
    }

    let shape_count = get_varuint(data, &mut cur)? as usize;
    let mut shapes = Vec::with_capacity(shape_count.min(4096));
    for _ in 0..shape_count {
        let style = get_varuint(data, &mut cur)? as usize;
        let path = get_varuint(data, &mut cur)? as usize;
        if style >= styles.len() || path >= paths.len() {
            return Err(Error::BadVector("shape index out of range"));
        }
        let rule = match *data.get(cur).ok_or(Error::Truncated)? {
            0 => FillRule::NonZero,
            1 => FillRule::EvenOdd,
            _ => return Err(Error::BadVector("unknown fill rule")),
        };
        cur += 1;
        shapes.push(Shape { style: style as u32, path: path as u32, fill_rule: rule });
    }

    if cur != data.len() {
        return Err(Error::BadVector("trailing bytes in vector payload"));
    }
    Ok(VectorImage { scale, colors, styles, paths, shapes })
}

// ---- rasterizer (informative reference; docs/vector-payload.md) ----

fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut x = v;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + v / x) / 2;
    }
    x
}

/// Rasterize at (out_w, out_h). Design space is (design_w, design_h) units.
pub fn rasterize(
    art: &VectorImage,
    design_w: u32,
    design_h: u32,
    out_w: u32,
    out_h: u32,
) -> Result<Vec<Rgba>, Error> {
    if design_w == 0 || design_h == 0 || out_w == 0 || out_h == 0 {
        return Err(Error::BadVector("zero dimension"));
    }
    let (w, h) = (out_w as usize, out_h as usize);
    let num_x = (out_w as i64 * SS) << 16;
    let num_y = (out_h as i64 * SS) << 16;
    let den_x = (design_w as i64) << art.scale;
    let den_y = (design_h as i64) << art.scale;
    let map = move |x: i64, y: i64| -> (i64, i64) { (x * num_x / den_x, y * num_y / den_y) };

    // Premultiplied output accumulator, channel range 0..65025.
    let mut acc = vec![[0u32; 4]; w * h];
    let mut coverage = vec![0u8; w * h];

    for shape in &art.shapes {
        let edges = build_edges(&art.paths[shape.path as usize], &map);
        coverage.iter_mut().for_each(|c| *c = 0);
        fill_coverage(&edges, out_w, out_h, shape.fill_rule, &mut coverage);
        composite(art, shape, &coverage, w, h, &map, &mut acc);
    }

    Ok(acc
        .iter()
        .map(|p| {
            let a = ((p[3] + 127) / 255).min(255);
            if a == 0 {
                Rgba::default()
            } else {
                let un = |c: u32| ((c * 255 + p[3] / 2) / p[3]).min(255) as u8;
                Rgba::new(un(p[0]), un(p[1]), un(p[2]), a as u8)
            }
        })
        .collect())
}

fn lerp_pt(a: (i64, i64), b: (i64, i64), t: i64) -> (i64, i64) {
    (a.0 + (b.0 - a.0) * t / ONE, a.1 + (b.1 - a.1) * t / ONE)
}

/// Flatten one path into edges in Q16 device subpixel space.
fn build_edges(cmds: &[PathCmd], map: &dyn Fn(i64, i64) -> (i64, i64)) -> Vec<(i64, i64, i64, i64)> {
    let mut edges = Vec::new();
    let mut start = (0i64, 0i64);
    let mut cur = (0i64, 0i64);
    let push = |a: (i64, i64), b: (i64, i64), edges: &mut Vec<(i64, i64, i64, i64)>| {
        if a.1 != b.1 {
            edges.push((a.0, a.1, b.0, b.1));
        }
    };
    for cmd in cmds {
        match *cmd {
            PathCmd::MoveTo(x, y) => {
                if cur != start {
                    push(cur, start, &mut edges);
                }
                cur = map(x, y);
                start = cur;
            }
            PathCmd::LineTo(x, y) => {
                let p = map(x, y);
                push(cur, p, &mut edges);
                cur = p;
            }
            PathCmd::QuadTo(qx, qy, x, y) => {
                let (q, p) = (map(qx, qy), map(x, y));
                let mut prev = cur;
                for i in 1..=8i64 {
                    let t = i * ONE / 8;
                    let m = lerp_pt(lerp_pt(cur, q, t), lerp_pt(q, p, t), t);
                    push(prev, m, &mut edges);
                    prev = m;
                }
                cur = p;
            }
            PathCmd::CubicTo(ax, ay, bx, by, x, y) => {
                let (a, b, p) = (map(ax, ay), map(bx, by), map(x, y));
                let mut prev = cur;
                for i in 1..=16i64 {
                    let t = i * ONE / 16;
                    let ab = lerp_pt(cur, a, t);
                    let bc = lerp_pt(a, b, t);
                    let cd = lerp_pt(b, p, t);
                    let m = lerp_pt(lerp_pt(ab, bc, t), lerp_pt(bc, cd, t), t);
                    push(prev, m, &mut edges);
                    prev = m;
                }
                cur = p;
            }
            PathCmd::Close => {
                push(cur, start, &mut edges);
                cur = start;
            }
        }
    }
    if cur != start {
        push(cur, start, &mut edges);
    }
    edges
}

/// Scanline fill at 4x4 supersampling; coverage counts 0..16 per pixel.
fn fill_coverage(
    edges: &[(i64, i64, i64, i64)],
    out_w: u32,
    out_h: u32,
    rule: FillRule,
    coverage: &mut [u8],
) {
    let sub_w = out_w as i64 * SS;
    let mut crossings: Vec<(i64, i32)> = Vec::new();
    for ys in 0..out_h as i64 * SS {
        let sy = (ys << 16) + ONE / 2;
        crossings.clear();
        for &(x0, y0, x1, y1) in edges {
            let (dir, ya, xa, yb, xb) = if y0 < y1 {
                (1, y0, x0, y1, x1)
            } else {
                (-1, y1, x1, y0, x0)
            };
            if sy >= ya && sy < yb {
                let x = xa + (xb - xa) * (sy - ya) / (yb - ya);
                crossings.push((x, dir));
            }
        }
        crossings.sort_unstable();

        let mut winding = 0i32;
        let mut span_start = 0i64;
        for &(x, dir) in crossings.iter() {
            let was_inside = match rule {
                FillRule::NonZero => winding != 0,
                FillRule::EvenOdd => winding % 2 != 0,
            };
            winding += dir;
            let now_inside = match rule {
                FillRule::NonZero => winding != 0,
                FillRule::EvenOdd => winding % 2 != 0,
            };
            if !was_inside && now_inside {
                span_start = x;
            } else if was_inside && !now_inside {
                mark_span(span_start, x, ys, sub_w, out_w, coverage);
            }
        }
    }
}

fn mark_span(xa: i64, xb: i64, ys: i64, sub_w: i64, out_w: u32, coverage: &mut [u8]) {
    let c0 = ((xa - ONE / 2 + ONE - 1) >> 16).max(0);
    let c1 = ((xb - ONE / 2 - 1) >> 16).min(sub_w - 1);
    let row = (ys / SS) as usize * out_w as usize;
    for c in c0..=c1 {
        coverage[row + (c / SS) as usize] += 1;
    }
}

fn composite(
    art: &VectorImage,
    shape: &Shape,
    coverage: &[u8],
    w: usize,
    h: usize,
    map: &dyn Fn(i64, i64) -> (i64, i64),
    acc: &mut [[u32; 4]],
) {
    let style = &art.styles[shape.style as usize];
    for y in 0..h {
        for x in 0..w {
            let cov = coverage[y * w + x] as u32;
            if cov == 0 {
                continue;
            }
            // Pixel center in Q16 device subpixel space.
            let px = (x as i64 * SS + SS / 2) << 16;
            let py = (y as i64 * SS + SS / 2) << 16;
            let color = style_color(art, style, map, px, py);
            let a_eff = (color.a as u32 * cov + 8) / 16;
            if a_eff == 0 {
                continue;
            }
            let dst = &mut acc[y * w + x];
            let inv = 255 - a_eff;
            dst[0] = color.r as u32 * a_eff + (dst[0] * inv + 127) / 255;
            dst[1] = color.g as u32 * a_eff + (dst[1] * inv + 127) / 255;
            dst[2] = color.b as u32 * a_eff + (dst[2] * inv + 127) / 255;
            dst[3] = 255 * a_eff + (dst[3] * inv + 127) / 255;
        }
    }
}

fn style_color(
    art: &VectorImage,
    style: &Style,
    map: &dyn Fn(i64, i64) -> (i64, i64),
    px: i64,
    py: i64,
) -> Rgba {
    match *style {
        Style::Flat { color } => art.colors[color as usize],
        Style::Linear { x0, y0, x1, y1, c0, c1 } => {
            let (ax, ay) = map(x0, y0);
            let (bx, by) = map(x1, y1);
            let (ex, ey) = (bx - ax, by - ay);
            let den16 = (ex * ex + ey * ey) >> 16;
            let t = if den16 == 0 {
                0
            } else {
                (((px - ax) * ex + (py - ay) * ey) / den16).clamp(0, ONE)
            };
            lerp_color(art.colors[c0 as usize], art.colors[c1 as usize], t)
        }
        Style::Radial { cx, cy, ex, ey, c0, c1 } => {
            let (ax, ay) = map(cx, cy);
            let (bx, by) = map(ex, ey);
            let r = isqrt((bx - ax) * (bx - ax) + (by - ay) * (by - ay));
            let d = isqrt((px - ax) * (px - ax) + (py - ay) * (py - ay));
            let t = if r == 0 { 0 } else { (d * ONE / r).clamp(0, ONE) };
            lerp_color(art.colors[c0 as usize], art.colors[c1 as usize], t)
        }
    }
}

fn lerp_color(a: Rgba, b: Rgba, t: i64) -> Rgba {
    let ch = |x: u8, y: u8| (x as i64 + (y as i64 - x as i64) * t / ONE) as u8;
    Rgba::new(ch(a.r, b.r), ch(a.g, b.g), ch(a.b, b.b), ch(a.a, b.a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_art() -> VectorImage {
        VectorImage {
            scale: 0,
            colors: vec![
                Rgba::new(200, 40, 40, 255),
                Rgba::new(40, 40, 200, 255),
                Rgba::new(30, 200, 30, 128),
            ],
            styles: vec![
                Style::Flat { color: 0 },
                Style::Linear { x0: 0, y0: 0, x1: 96, y1: 0, c0: 0, c1: 1 },
                Style::Radial { cx: 48, cy: 48, ex: 88, ey: 48, c0: 2, c1: 1 },
            ],
            paths: vec![
                vec![
                    PathCmd::MoveTo(8, 8),
                    PathCmd::LineTo(88, 8),
                    PathCmd::LineTo(88, 88),
                    PathCmd::LineTo(8, 88),
                    PathCmd::Close,
                ],
                vec![
                    PathCmd::MoveTo(48, 12),
                    PathCmd::CubicTo(80, 12, 80, 84, 48, 84),
                    PathCmd::QuadTo(20, 48, 48, 12),
                    PathCmd::Close,
                ],
            ],
            shapes: vec![
                Shape { style: 1, path: 0, fill_rule: FillRule::NonZero },
                Shape { style: 0, path: 1, fill_rule: FillRule::EvenOdd },
            ],
        }
    }

    #[test]
    fn wire_roundtrip() {
        let art = sample_art();
        let encoded = encode(&art);
        let decoded = decode(&encoded).expect("decode failed");
        assert_eq!(decoded, art);
    }

    #[test]
    fn rejects_draw_before_move() {
        let mut data = vec![0u8];
        data.push(0);
        data.push(0);
        data.push(1);
        data.extend_from_slice(&[1, 1, 2, 2]);
        data.push(0);
        assert_eq!(decode(&data), Err(Error::BadVector("draw before move_to")));
    }

    #[test]
    fn rejects_bad_indices() {
        let art = sample_art();
        let mut encoded = encode(&art);
        let n = encoded.len();
        encoded[n - 3] = 9;
        assert!(matches!(decode(&encoded), Err(Error::BadVector(_))));
    }

    #[test]
    fn rasterizes_inside_and_outside() {
        let art = sample_art();
        let px = rasterize(&art, 96, 96, 96, 96).unwrap();
        assert_eq!(px[0].a, 0);
        let center = px[48 * 96 + 48];
        assert!(center.a == 255, "center should be opaque, got {center:?}");
        assert!(center.r > 150, "center should be red-ish, got {center:?}");
    }

    #[test]
    fn scales_without_pixelation_artifacts() {
        let art = sample_art();
        let big = rasterize(&art, 96, 96, 384, 384).unwrap();
        assert_eq!(big.len(), 384 * 384);
        assert_eq!(big[0].a, 0);
        assert_eq!(big[192 * 384 + 192].a, 255);
    }
}
