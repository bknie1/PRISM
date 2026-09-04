# PRISM image format, spec draft 0

Status: Phase 0 skeleton, 2026-09-04. This document is the scope fence; the encoder is not started until this is agreed.

Extension: `.prism`. The name is the design statement: one image goes in, multiple representations come out.

## Niche

One container, two payload types. Stores any image losslessly and renders it at any scale with identical results in every conforming viewer. Raster content is bit-exact at native resolution with format-defined continuous reconstruction beyond it; geometric content is stored as true vectors and is resolution-independent by construction.

## Design constraints

These are physics, and the spec never promises around them.

1. No universal compressor exists (pigeonhole principle). Compression wins come from modeling images well, and the ratio target is per-domain, never "beats everything."
2. Detail the sensor never captured cannot be stored or recovered. Scaling raster content means evaluating a defined reconstruction surface through the true samples; it is honest interpolation, never invented detail.
3. Confidentiality is an encryption layer with standard cryptography. The encoding itself is never treated as a security mechanism.

## Container

Magic bytes, format version, then a chunk table. Chunks: header (dimensions, color space, bit depth, payload type), one or more payload chunks (raster or vector), optional metadata, optional encryption envelope wrapping the payloads. Byte-aligned throughout. Endianness, alignment, and chunk checksums are open research items.

## Raster payload

Bit-exact lossless. First implementation is a QOI-class byte-aligned op stream; candidate twists to evaluate during research: 2D prediction using the previous row (QOI's known weakness), a smarter pixel index, and tiled layout for partial decode. Bit-level entropy coding is deferred to a later phase and only if the ratio chase justifies the complexity.

## Reconstruction spec

The format mandates the upscaling function so every viewer renders identical output at any scale. Candidates to research: Catmull-Rom, exact-interpolating B-splines, Lanczos. Requirements: deterministic, passes exactly through the stored samples, cheap enough to evaluate live.

## Vector payload

Compact binary curve and fill records with quantized coordinates, in the TinyVG and HVIF tradition. Phase 2; study those specs before designing.

## Encryption wrapper

Optional AEAD (ChaCha20-Poly1305 or equivalent) over the payload chunks. No homebrew ciphers.

## Phasing

Phase 0: research (QOI, post-QOI critiques, JPEG XL modular mode overview, TinyVG, HVIF, interpolation theory) and this spec.
Phase 1: container + raster encoder/decoder in Rust, round-trip tests over a corpus, benchmarks against PNG and QOI.
Phase 2: reconstruction renderer (scale-anywhere viewer).
Phase 3: vector payload.
Phase 4: encryption wrapper.

## Non-goals

Animation, thumbnail/metadata ecosystems, beating JPEG XL on ratio in version 1, browser adoption.

## Open questions

Bit depths beyond 8 (16-bit, HDR). Color space handling (sRGB first?). Alpha semantics. Endianness. Whether mixed raster+vector in one file is allowed in version 1 or one payload type per file.
