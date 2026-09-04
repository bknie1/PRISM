# Teaching materials

Reading order. Nothing here assumes prior image processing knowledge; the intro and glossary carry all of it.

0. [The whole project, in plain English](00-eli5.md) - start here, no background assumed.
- [Glossary](glossary.md) - every term of art in the lessons and specs, defined plainly. Keep it open alongside everything else.
1. [Workspaces, crates, and modules](01-workspace-and-crates.md) - how the Rust project is shaped and why.
2. [Bytes, wrapping arithmetic, and slices](02-bytes-wrapping-and-slices.md) - the codec-level Rust in the raster coder.
3. [Enums, errors, and match](03-enums-errors-and-match.md) - how failure handling works, from the error type outward.
4. [Fixed point and determinism](04-fixed-point-and-determinism.md) - the scaling math, and why it uses no floating point.
5. [Enums with data, closures, and varints](05-enums-with-data-and-varints.md) - the vector payload's encoding and rasterizer.
6. [Cargo features and the crypto boundary](06-features-and-the-crypto-boundary.md) - optional dependencies and the encryption wrapper.

Each lesson points at real files in `crates/`; the normative specs live one directory up in `docs/`.
