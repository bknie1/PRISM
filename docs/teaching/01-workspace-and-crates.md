# Lesson 1: workspaces, crates, and modules

What the repository layout is teaching, using PRISM's actual files.

New to images or compression? Start with [the plain-English intro](00-eli5.md); every term of art is defined in [the glossary](glossary.md).

## Crates are the unit of compilation and publishing

A crate is one library or one binary. PRISM has two: `prism-core` (the format: pure logic, zero dependencies) and `prism-cli` (the `prism` binary: file IO, PNG conversion, benchmarks). The split is deliberate and it is the same discipline as keeping a C library free of `main()`: anything that might one day be a wasm decoder or an editor plugin lives in core, and core is not allowed to know that files or terminals exist. Core's `Cargo.toml` has an empty `[dependencies]` section, which is the strongest simplicity claim a Rust project can make.

## The workspace ties them together

The root `Cargo.toml` declares `[workspace]` with both crates as members. One `target/` directory, one lockfile, one `cargo test --workspace` that runs everything. `version.workspace = true` in each crate inherits shared fields from the root, so version bumps happen in one place.

## Modules are namespaces inside a crate

`prism-core/src/lib.rs` is the crate root; each `pub mod x;` line pulls in `src/x.rs`. Items are private to their module unless marked `pub`, and lib.rs re-exports the public API (`pub use container::{decode_file, ...}`) so users write `prism_core::decode_file` without knowing the internal module layout. Compare `pixel::med`, which is private: the MED predictor is an implementation detail of prediction, and the compiler enforces that nobody outside the module can depend on it.

## Dependencies are declared, versioned, and scoped

`prism-cli/Cargo.toml` declares `image` with `default-features = false, features = ["png"]`: features are compile-time switches, and turning off defaults keeps the dozens of other image codecs out of the build. `prism-core = { path = "../prism-core" }` is a path dependency: the CLI always builds against the sibling checkout, no registry involved.
