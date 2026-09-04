# PRISM image format, spec draft 1

Status: Phase 0, 2026-09-04. This document is the scope fence; the encoder is not started until this is agreed.

Extension: `.prism`. The name is the design statement: one image goes in, multiple representations come out.

The flagship feature is the reconstruction spec: PRISM is the raster format that defines its own scaling math, so a file renders identically at any size in every conforming viewer. Compression targets most use cases, never edge-case supremacy.

## Niche

One container, two payload types. Stores any image losslessly and renders it at any scale with identical results in every conforming viewer. Raster content is bit-exact at native resolution with format-defined continuous reconstruction beyond it; geometric content is stored as true vectors and is resolution-independent by construction.

## Design constraints

These are physics, and the spec never promises around them.

1. No universal compressor exists (pigeonhole principle). Compression wins come from modeling images well, and the ratio target is per-domain, never "beats everything."
2. Detail the sensor never captured cannot be stored or recovered. Scaling raster content means evaluating a defined reconstruction surface through the true samples; it is honest interpolation, never invented detail.
3. Confidentiality is an encryption layer with standard cryptography. The encoding itself is never treated as a security mechanism.

## Container

Magic bytes, format version, then a chunk table. Chunks: header (dimensions, color space, bit depth, payload type), one payload chunk (raster or vector; mixed payloads are reserved for a later version), optional metadata, optional encryption envelope wrapping the payload. Byte-aligned throughout, all integers little-endian, CRC32 per chunk.

## Raster payload

Bit-exact lossless, 8-bit RGBA in sRGB with straight (unpremultiplied) alpha in version 1; the header reserves depth and colorspace fields so 16-bit and HDR can arrive later without breaking files. The payload is a QOI-class byte-aligned op stream extended with a previous-row predictor (QOI's known weakness against PNG's filters) and a tiled layout for partial decode. Bit-level entropy coding is deferred to a later phase and only if the ratio chase justifies the complexity. See docs/research/qoi.md.

## Reconstruction spec

The format mandates the upscaling function so every viewer renders identical output at any scale. Version 1 mandates Catmull-Rom: an interpolating spline that passes exactly through the stored samples, deterministic, and cheap enough to evaluate live. Alternative kernels (Lanczos and friends) may be added later as declared header options, never as viewer discretion.

## Vector payload

Compact binary curve and fill records with quantized coordinates, in the TinyVG and HVIF tradition. Phase 2; study those specs before designing.

## Encryption wrapper

Optional AEAD (ChaCha20-Poly1305 or equivalent) over the payload chunks. No homebrew ciphers.

## Phasing

Phase 0 (done): research (docs/research/) and this spec.
Phase 1 (done): container + raster encoder/decoder (docs/container.md, docs/raster-payload.md), round-trip tests, benchmarks against PNG and QOI (docs/benchmarks.md).
Phase 2 (done): reconstruction renderer (docs/reconstruction-spec.md).
Phase 3 (done): vector payload (docs/vector-payload.md).
Phase 4 (done): encryption wrapper (ENCR in docs/container.md), feature-gated so prism-core stays dependency-free by default.
Next candidates: real-photo corpus for honest benchmarks, encoder/decoder speed work, minification spec, fuzzing the decoders, a wasm viewer.

## Non-goals

Animation, thumbnail/metadata ecosystems, beating JPEG XL on ratio in version 1, browser adoption.

## Resolved decisions

Little-endian throughout. Version 1 pixels are 8-bit RGBA, sRGB, straight alpha; header reserves room for more depths and colorspaces. Catmull-Rom is the mandated v1 reconstruction kernel. CRC32 per chunk. One payload type per file in v1. Encryption is standard AEAD only.

## Open questions

Exact op set for the previous-row predictor (design against the QOI ops during Phase 1). Tile size. Metadata chunk contents.
