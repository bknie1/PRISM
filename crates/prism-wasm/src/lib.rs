//! WebAssembly boundary for prism-core.
//!
//! Deliberately dependency-free: no wasm-bindgen, no generated glue. Every
//! export takes and returns plain integers, so JavaScript calls these
//! functions directly off `instance.exports` and reads results straight out
//! of `instance.exports.memory` (see docs/index.html for the ~40 lines of
//! JS this buys). That mirrors prism-core's own "no dependency we don't
//! need" discipline.

use std::cell::RefCell;

use prism_core::container::{decode_payload, FilePayload};
use prism_core::vector::{self, VectorImage};

enum Loaded {
    Raster { pixels: Vec<u8>, width: u32, height: u32 },
    Vector { art: VectorImage, design_w: u32, design_h: u32 },
}

struct Output {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

thread_local! {
    static LOADED: RefCell<Option<Loaded>> = const { RefCell::new(None) };
    static OUTPUT: RefCell<Option<Output>> = const { RefCell::new(None) };
}

fn to_rgba_bytes(pixels: &[prism_core::Rgba]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() * 4);
    for p in pixels {
        out.extend_from_slice(&[p.r, p.g, p.b, p.a]);
    }
    out
}

/// Allocate `len` bytes in linear memory for the caller to write a .prism
/// file into, then pass to `prism_load`.
#[no_mangle]
pub extern "C" fn prism_alloc(len: usize) -> *mut u8 {
    let mut buf = vec![0u8; len].into_boxed_slice();
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Free a buffer previously returned by `prism_alloc`.
#[no_mangle]
pub extern "C" fn prism_free(ptr: *mut u8, len: usize) {
    unsafe {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

/// Parse the .prism file at (ptr, len). Returns 1 if it is a raster
/// payload, 2 if vector, 0 on error. Caller owns and must free the input
/// buffer; this function copies what it needs out of it before returning.
#[no_mangle]
pub extern "C" fn prism_load(ptr: *const u8, len: usize) -> i32 {
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let result = match decode_payload(bytes) {
        Ok(FilePayload::Raster(img)) => {
            let pixels = to_rgba_bytes(&img.pixels);
            (Loaded::Raster { pixels, width: img.width, height: img.height }, 1)
        }
        Ok(FilePayload::Vector { art, width, height }) => {
            (Loaded::Vector { art, design_w: width, design_h: height }, 2)
        }
        Err(_) => return 0,
    };
    LOADED.with(|cell| *cell.borrow_mut() = Some(result.0));
    result.1
}

/// Native size for a loaded raster file, or design size for vector.
#[no_mangle]
pub extern "C" fn prism_width() -> u32 {
    LOADED.with(|cell| match cell.borrow().as_ref() {
        Some(Loaded::Raster { width, .. }) => *width,
        Some(Loaded::Vector { design_w, .. }) => *design_w,
        None => 0,
    })
}

#[no_mangle]
pub extern "C" fn prism_height() -> u32 {
    LOADED.with(|cell| match cell.borrow().as_ref() {
        Some(Loaded::Raster { height, .. }) => *height,
        Some(Loaded::Vector { design_h, .. }) => *design_h,
        None => 0,
    })
}

/// Render the loaded file at (out_w, out_h) into the output buffer read by
/// prism_out_ptr/prism_out_len. Raster files use the mandated Catmull-Rom
/// reconstruction (version 1 is upscale-only, matching docs/reconstruction-spec.md,
/// so out_w/out_h below native size fail); vector files rasterize freely at
/// any size. Returns 0 on success, nonzero on error.
#[no_mangle]
pub extern "C" fn prism_render(out_w: u32, out_h: u32) -> i32 {
    let rendered = LOADED.with(|cell| match cell.borrow().as_ref() {
        Some(Loaded::Raster { pixels, width, height }) => {
            if out_w == *width && out_h == *height {
                return Some(Ok(pixels.clone()));
            }
            let img = prism_core::Image {
                width: *width,
                height: *height,
                pixels: pixels
                    .chunks_exact(4)
                    .map(|c| prism_core::Rgba::new(c[0], c[1], c[2], c[3]))
                    .collect(),
            };
            Some(
                prism_core::reconstruct::scale(&img, out_w, out_h)
                    .map(|scaled| to_rgba_bytes(&scaled.pixels)),
            )
        }
        Some(Loaded::Vector { art, design_w, design_h }) => Some(
            vector::rasterize(art, *design_w, *design_h, out_w, out_h)
                .map(|pixels| to_rgba_bytes(&pixels)),
        ),
        None => None,
    });

    match rendered {
        Some(Ok(pixels)) => {
            OUTPUT.with(|cell| {
                *cell.borrow_mut() = Some(Output { pixels, width: out_w, height: out_h })
            });
            0
        }
        _ => -1,
    }
}

#[no_mangle]
pub extern "C" fn prism_out_ptr() -> *const u8 {
    OUTPUT.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|o| o.pixels.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

#[no_mangle]
pub extern "C" fn prism_out_len() -> usize {
    OUTPUT.with(|cell| cell.borrow().as_ref().map(|o| o.pixels.len()).unwrap_or(0))
}

#[no_mangle]
pub extern "C" fn prism_out_width() -> u32 {
    OUTPUT.with(|cell| cell.borrow().as_ref().map(|o| o.width).unwrap_or(0))
}

#[no_mangle]
pub extern "C" fn prism_out_height() -> u32 {
    OUTPUT.with(|cell| cell.borrow().as_ref().map(|o| o.height).unwrap_or(0))
}
