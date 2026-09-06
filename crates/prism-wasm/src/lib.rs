//! WebAssembly boundary for prism-core.
//!
//! Deliberately dependency-free: no wasm-bindgen, no generated glue. Every
//! export takes and returns plain integers, so JavaScript calls these
//! functions directly off `instance.exports` and reads results straight out
//! of `instance.exports.memory` (see docs/index.html for the ~40 lines of
//! JS this buys). That mirrors prism-core's own "no dependency we don't
//! need" discipline.
//!
//! Loaded files are kept in a slab and referenced by handle, not by a
//! single shared slot: a page decoding more than one .prism file (as the
//! demo page does) must be able to hold both open at once without one
//! `prism_load` call clobbering the other's state.

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
    static SLAB: RefCell<Vec<Option<Loaded>>> = const { RefCell::new(Vec::new()) };
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

/// Parse the .prism file at (ptr, len) and keep it in the slab. Returns a
/// handle (1-based) to pass to every other function below, or 0 on error.
/// Caller owns and must free the input buffer; this function copies what
/// it needs out of it before returning.
#[no_mangle]
pub extern "C" fn prism_load(ptr: *const u8, len: usize) -> u32 {
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let loaded = match decode_payload(bytes) {
        Ok(FilePayload::Raster(img)) => Loaded::Raster {
            pixels: to_rgba_bytes(&img.pixels),
            width: img.width,
            height: img.height,
        },
        Ok(FilePayload::Vector { art, width, height }) => {
            Loaded::Vector { art, design_w: width, design_h: height }
        }
        Err(_) => return 0,
    };
    SLAB.with(|slab| {
        let mut slab = slab.borrow_mut();
        slab.push(Some(loaded));
        slab.len() as u32
    })
}

/// Release a handle's memory. Safe to call more than once on the same handle.
#[no_mangle]
pub extern "C" fn prism_unload(handle: u32) {
    SLAB.with(|slab| {
        if let Some(slot) = slab.borrow_mut().get_mut(handle.wrapping_sub(1) as usize) {
            *slot = None;
        }
    });
}

/// 1 if `handle` holds a raster payload, 2 if vector, 0 if the handle is invalid.
#[no_mangle]
pub extern "C" fn prism_kind(handle: u32) -> i32 {
    with_loaded(handle, |l| match l {
        Loaded::Raster { .. } => 1,
        Loaded::Vector { .. } => 2,
    })
    .unwrap_or(0)
}

/// Native size for a raster handle, or design size for vector.
#[no_mangle]
pub extern "C" fn prism_width(handle: u32) -> u32 {
    with_loaded(handle, |l| match l {
        Loaded::Raster { width, .. } => *width,
        Loaded::Vector { design_w, .. } => *design_w,
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn prism_height(handle: u32) -> u32 {
    with_loaded(handle, |l| match l {
        Loaded::Raster { height, .. } => *height,
        Loaded::Vector { design_h, .. } => *design_h,
    })
    .unwrap_or(0)
}

fn with_loaded<T>(handle: u32, f: impl FnOnce(&Loaded) -> T) -> Option<T> {
    SLAB.with(|slab| slab.borrow().get(handle.wrapping_sub(1) as usize)?.as_ref().map(f))
}

/// Render `handle` at (out_w, out_h) into the output buffer read by
/// prism_out_ptr/prism_out_len. Raster files use the mandated Catmull-Rom
/// reconstruction (version 1 is upscale-only, matching
/// docs/reconstruction-spec.md, so out_w/out_h below native size fail);
/// vector files rasterize freely at any size. Returns 0 on success,
/// nonzero on error. The output buffer is a single shared slot: read it
/// immediately after this call, before rendering any other handle.
#[no_mangle]
pub extern "C" fn prism_render(handle: u32, out_w: u32, out_h: u32) -> i32 {
    let rendered = with_loaded(handle, |loaded| match loaded {
        Loaded::Raster { pixels, width, height } => {
            if out_w == *width && out_h == *height {
                return Ok(pixels.clone());
            }
            let img = prism_core::Image {
                width: *width,
                height: *height,
                pixels: pixels
                    .chunks_exact(4)
                    .map(|c| prism_core::Rgba::new(c[0], c[1], c[2], c[3]))
                    .collect(),
            };
            prism_core::reconstruct::scale(&img, out_w, out_h).map(|scaled| to_rgba_bytes(&scaled.pixels))
        }
        Loaded::Vector { art, design_w, design_h } => {
            vector::rasterize(art, *design_w, *design_h, out_w, out_h).map(|pixels| to_rgba_bytes(&pixels))
        }
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
