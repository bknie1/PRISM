//! prism: encode, decode, inspect, and benchmark PRISM files.

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use prism_core::{container, decode_file, encode_file, Image, Rgba};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let strs: Vec<&str> = args.iter().map(String::as_str).collect();
    match strs.as_slice() {
        ["encode", input, output] => encode(input, output),
        ["decode", input, output] => decode(input, output),
        ["info", input] => info(input),
        ["bench", dir] => bench(dir),
        ["gen-corpus", dir] => gen_corpus(dir),
        ["scale", input, output, factor] => scale_cmd(input, output, factor.parse()?),
        ["compare", input, output, factor] => compare_cmd(input, output, factor.parse()?),
        _ => {
            eprintln!(
                "usage:\n  prism encode <in.png> <out.prism>\n  prism decode <in.prism> <out.png>\n  prism info <file.prism>\n  prism bench <png-dir>\n  prism gen-corpus <dir>\n  prism scale <in.prism|in.png> <out.png> <factor>\n  prism compare <in.png> <out.png> <factor>"
            );
            Err("invalid arguments".into())
        }
    }
}

fn load_png(path: &str) -> Result<Image, Box<dyn std::error::Error>> {
    let rgba = image::open(path)?.to_rgba8();
    let (width, height) = rgba.dimensions();
    let pixels = rgba
        .as_raw()
        .chunks_exact(4)
        .map(|c| Rgba::new(c[0], c[1], c[2], c[3]))
        .collect();
    Ok(Image { width, height, pixels })
}

fn save_png(image: &Image, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let raw: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|p| [p.r, p.g, p.b, p.a])
        .collect();
    let buf = image::RgbaImage::from_raw(image.width, image.height, raw)
        .ok_or("pixel buffer does not match dimensions")?;
    buf.save(path)?;
    Ok(())
}

fn encode(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let img = load_png(input)?;
    let encoded = encode_file(&img);
    fs::write(output, &encoded)?;
    let raw = img.pixels.len() * 4;
    println!(
        "{input} -> {output}: {} -> {} bytes ({:.1}% of raw RGBA)",
        raw,
        encoded.len(),
        100.0 * encoded.len() as f64 / raw as f64
    );
    Ok(())
}

fn decode(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let data = fs::read(input)?;
    let img = decode_file(&data)?;
    save_png(&img, output)?;
    println!("{input} -> {output}: {}x{}", img.width, img.height);
    Ok(())
}

fn info(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let data = fs::read(input)?;
    let (header, chunks) = container::read_chunks(&data)?;
    println!(
        "{}x{} {:?}, {}-bit, colorspace {}, alpha {}, tiles 2^{}, kernel {}",
        header.width,
        header.height,
        header.payload_kind,
        header.bit_depth,
        header.colorspace,
        header.alpha_mode,
        header.tile_size_log2,
        header.kernel
    );
    for c in &chunks {
        println!(
            "  {} {:>10} bytes",
            String::from_utf8_lossy(&c.ty),
            c.payload.len()
        );
    }
    Ok(())
}

fn load_any(input: &str) -> Result<Image, Box<dyn std::error::Error>> {
    if input.ends_with(".prism") {
        Ok(decode_file(&fs::read(input)?)?)
    } else {
        load_png(input)
    }
}

fn scale_cmd(input: &str, output: &str, factor: u32) -> Result<(), Box<dyn std::error::Error>> {
    let img = load_any(input)?;
    let scaled = prism_core::reconstruct::scale(&img, img.width * factor, img.height * factor)?;
    save_png(&scaled, output)?;
    println!(
        "{input} -> {output}: {}x{} -> {}x{}",
        img.width, img.height, scaled.width, scaled.height
    );
    Ok(())
}

fn nearest(img: &Image, w_out: u32, h_out: u32) -> Image {
    let mut pixels = Vec::with_capacity((w_out * h_out) as usize);
    for y in 0..h_out {
        for x in 0..w_out {
            let sx = (x * img.width / w_out).min(img.width - 1) as usize;
            let sy = (y * img.height / h_out).min(img.height - 1) as usize;
            pixels.push(img.pixels[sy * img.width as usize + sx]);
        }
    }
    Image { width: w_out, height: h_out, pixels }
}

/// Side-by-side: nearest-neighbor left, mandated reconstruction right.
fn compare_cmd(input: &str, output: &str, factor: u32) -> Result<(), Box<dyn std::error::Error>> {
    let img = load_any(input)?;
    let (w, h) = (img.width * factor, img.height * factor);
    let left = nearest(&img, w, h);
    let right = prism_core::reconstruct::scale(&img, w, h)?;

    let gap = 4u32;
    let total_w = w * 2 + gap;
    let mut pixels = vec![Rgba::new(255, 255, 255, 255); (total_w * h) as usize];
    for y in 0..h as usize {
        let row = y * total_w as usize;
        pixels[row..row + w as usize]
            .copy_from_slice(&left.pixels[y * w as usize..(y + 1) * w as usize]);
        pixels[row + (w + gap) as usize..row + total_w as usize]
            .copy_from_slice(&right.pixels[y * w as usize..(y + 1) * w as usize]);
    }
    save_png(&Image { width: total_w, height: h, pixels }, output)?;
    println!("{output}: nearest (left) vs Catmull-Rom reconstruction (right) at {factor}x");
    Ok(())
}

struct BenchRow {
    name: String,
    raw: usize,
    prism: usize,
    qoi: usize,
    png: usize,
    prism_enc_ms: f64,
    prism_dec_ms: f64,
    qoi_enc_ms: f64,
    png_enc_ms: f64,
}

fn bench(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut rows = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    entries.sort();
    if entries.is_empty() {
        return Err(format!("no .png files in {dir}").into());
    }

    for path in &entries {
        rows.push(bench_one(path)?);
    }

    println!(
        "{:<22} {:>10} {:>10} {:>10} {:>10} | {:>9} {:>9} {:>9} {:>9}",
        "image", "raw", "prism", "qoi", "png", "pr-enc", "pr-dec", "qoi-enc", "png-enc"
    );
    for r in &rows {
        println!(
            "{:<22} {:>10} {:>10} {:>10} {:>10} | {:>7.1}ms {:>7.1}ms {:>7.1}ms {:>7.1}ms",
            r.name, r.raw, r.prism, r.qoi, r.png, r.prism_enc_ms, r.prism_dec_ms, r.qoi_enc_ms, r.png_enc_ms
        );
    }
    let (raw, prism, qoi, png): (usize, usize, usize, usize) = rows.iter().fold(
        (0, 0, 0, 0),
        |acc, r| (acc.0 + r.raw, acc.1 + r.prism, acc.2 + r.qoi, acc.3 + r.png),
    );
    println!(
        "totals: raw {raw}, prism {prism} ({:.1}%), qoi {qoi} ({:.1}%), png {png} ({:.1}%)",
        100.0 * prism as f64 / raw as f64,
        100.0 * qoi as f64 / raw as f64,
        100.0 * png as f64 / raw as f64
    );
    Ok(())
}

fn bench_one(path: &Path) -> Result<BenchRow, Box<dyn std::error::Error>> {
    let img = load_png(path.to_str().ok_or("bad path")?)?;
    let raw_bytes: Vec<u8> = img
        .pixels
        .iter()
        .flat_map(|p| [p.r, p.g, p.b, p.a])
        .collect();

    let t = Instant::now();
    let prism_data = encode_file(&img);
    let prism_enc_ms = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    let back = decode_file(&prism_data)?;
    let prism_dec_ms = t.elapsed().as_secs_f64() * 1000.0;
    if back != img {
        return Err(format!("round-trip mismatch on {}", path.display()).into());
    }

    let t = Instant::now();
    let qoi_data = qoi::encode_to_vec(&raw_bytes, img.width, img.height)?;
    let qoi_enc_ms = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    let mut png_data = Vec::new();
    let enc = image::codecs::png::PngEncoder::new(&mut png_data);
    image::ImageEncoder::write_image(
        enc,
        &raw_bytes,
        img.width,
        img.height,
        image::ExtendedColorType::Rgba8,
    )?;
    let png_enc_ms = t.elapsed().as_secs_f64() * 1000.0;

    Ok(BenchRow {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        raw: raw_bytes.len(),
        prism: prism_data.len(),
        qoi: qoi_data.len(),
        png: png_data.len(),
        prism_enc_ms,
        prism_dec_ms,
        qoi_enc_ms,
        png_enc_ms,
    })
}

fn gen_corpus(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dir)?;
    let size = 512u32;

    let save = |name: &str, size: u32, f: &dyn Fn(u32, u32) -> Rgba| -> Result<(), Box<dyn std::error::Error>> {
        let mut pixels = Vec::with_capacity((size * size) as usize);
        for y in 0..size {
            for x in 0..size {
                pixels.push(f(x, y));
            }
        }
        let img = Image { width: size, height: size, pixels };
        save_png(&img, &format!("{dir}/{name}.png"))
    };

    save("gradient", size, &|x, y| {
        Rgba::new((x / 2) as u8, (y / 2) as u8, ((x + y) / 4) as u8, 255)
    })?;

    save("plasma", size, &|x, y| {
        let (fx, fy) = (x as f32 / 64.0, y as f32 / 64.0);
        let v = (fx.sin() + fy.cos() + (fx + fy).sin() + (fx * fy).cos()) / 4.0;
        let c = ((v * 0.5 + 0.5) * 255.0) as u8;
        Rgba::new(c, 255 - c, c / 2 + 60, 255)
    })?;

    save("shapes", size, &|x, y| {
        let (cx, cy) = (x as i32 - 256, y as i32 - 256);
        let d = cx * cx + cy * cy;
        if d < 100 * 100 {
            Rgba::new(220, 60, 60, 255)
        } else if d < 200 * 200 {
            Rgba::new(60, 60, 220, 255)
        } else {
            Rgba::new(240, 240, 235, 255)
        }
    })?;

    save("demo-small", 48, &|x, y| {
        let (cx, cy) = (x as i32 - 24, y as i32 - 24);
        let d = cx * cx + cy * cy;
        if d < 12 * 12 {
            Rgba::new(210, 70, 50, 255)
        } else if (x as i32 - y as i32).abs() < 2 {
            Rgba::new(30, 30, 30, 255)
        } else {
            Rgba::new((80 + x * 3) as u8, (90 + y * 3) as u8, 170, 255)
        }
    })?;

    let mut state = 0xDEADBEEFu64;
    let mut next = move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 56) as u8
    };
    let mut pixels = Vec::with_capacity((size * size) as usize);
    for _ in 0..size * size {
        pixels.push(Rgba::new(next(), next(), next(), 255));
    }
    save_png(
        &Image { width: size, height: size, pixels },
        &format!("{dir}/noise.png"),
    )?;

    println!("wrote gradient, plasma, shapes, noise to {dir}");
    Ok(())
}
