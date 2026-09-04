# Lesson 3: enums, errors, and match

How PRISM's failure handling works, from `prism-core/src/error.rs` outward.

New to images or compression? Start with [the plain-English intro](00-eli5.md); terms like chunk, CRC, and op are defined in [the glossary](glossary.md).

## Errors are values in a plain enum

`Error` in error.rs is an enum: `BadMagic`, `Truncated`, `CrcMismatch { chunk }`, and so on. Variants can carry data (the CRC failure names which chunk). There are no exceptions in Rust; a function that can fail returns `Result<T, Error>`, and the caller cannot touch the `T` without deciding what happens on the error case. Forgetting to handle a failure is a compile error, which for a file-format parser fed untrusted bytes is exactly the guarantee you want.

## `?` is the propagation operator

`decode_tile(stream, tw, th)?` means: on `Ok(v)`, unwrap to `v`; on `Err(e)`, return it from this function right now. Error handling reads like straight-line code while every failure path stays explicit in the signature. In the CLI, `run()` returns `Result<(), Box<dyn std::error::Error>>`: a trait object that can hold any error type (ours, `std::io::Error`, the PNG library's), which is the pragmatic choice at the application boundary where all you do is print the message. The library keeps the precise enum; the binary flattens it. That split is idiomatic.

## match must be exhaustive

The op dispatch in `decode_tile` matches on `b >> 6`: four possible 2-bit values, and the compiler refuses the code unless all four are covered. Add a fifth op to the format and every match over ops stops compiling until each site handles it; the compiler hands you the checklist. `PayloadKind` in container.rs works the same way: when Phase 3 makes vector payloads real, the `match header.payload_kind` in `decode_file` is where the compiler will point.

## Traits make the enum a citizen

`impl fmt::Display for Error` gives human messages; `impl std::error::Error for Error {}` (an empty impl; the defaults suffice) lets our enum ride in `Box<dyn Error>` and interoperate with the whole ecosystem's error tooling. Traits are interfaces the type opts into, and these two are the only ceremony a custom error type needs.
