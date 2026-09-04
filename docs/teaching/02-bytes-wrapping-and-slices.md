# Lesson 2: bytes, wrapping arithmetic, and slices

The codec-level Rust in `prism-core/src/raster.rs` and `crc.rs`.

New to images or compression? Start with [the plain-English intro](00-eli5.md); terms like predictor, residual, and checksum are defined in [the glossary](glossary.md).

## Wrapping arithmetic is opt-in, and the codec depends on it

Rust panics on integer overflow in debug builds; that catches bugs, but PRISM's residuals (each pixel's difference from what the predictor guessed) are defined as mod-256 differences (docs/raster-payload.md), where wrapping IS the semantics. So the code says it explicitly: `cur.r.wrapping_sub(p.r)` in `encode_tile`, `p.r.wrapping_add(dr)` in `decode_tile`. If prediction says 254 and the pixel is 1, the residual is +3, wrapping through 255, and both sides agree because both used the same modular arithmetic on purpose. C would have done this silently and also silently done it where you did not want it; Rust makes each wrap a visible decision.

The companion trick is the `as i8` cast in the encoder: a residual byte like 0xFD reinterpreted as i8 becomes -3, and range checks like `(-2..=1).contains(&dr)` read like the spec text. The cast costs nothing at runtime; it only changes which arithmetic rules the type system applies.

## Slices are bounds-checked windows, and `get` makes failure a value

The decoder walks a `&[u8]` with an index. Two access styles appear, chosen deliberately:

- `s.get(i)` returns `Option<&u8>`: out of bounds is a value you must handle, and the decoder maps it to `Error::Truncated` with `.ok_or(...)?`. Every read that depends on untrusted file data uses this style; a malicious file cannot make the decoder read out of bounds or panic.
- `tile[y * tw + x]` indexes directly and panics on error: used only where the math guarantees validity (tile-local coordinates the loop itself produced). A panic there would mean a bug in PRISM, never in the input file.

That split IS the memory-safety story: the language forces every buffer access to be one of "checked and handled" or "provably fine, panic if I am wrong."

## Integers to bytes and back, endianness explicit

`(len as u32).to_le_bytes()` produces a `[u8; 4]`; `u32::from_le_bytes(bytes.try_into().unwrap())` reverses it. Endianness is in the method name, so the little-endian decision from docs/container.md appears verbatim in code, and there is no way to read a multi-byte integer without saying which order you mean.

## `const fn`: computation at compile time

`crc.rs` builds its 256-entry table with a `const fn` evaluated during compilation: `const TABLE: [u32; 256] = build_table();`. The table is baked into the binary, costs nothing at startup, and the function is ordinary Rust (loops and all) rather than a macro or a generated file checked into the repo.
