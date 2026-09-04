# Lesson 6: cargo features and the crypto boundary

What the ENCR wrapper (`prism-core/src/crypto.rs`) teaches.

## Optional dependencies via features

prism-core's headline property is the empty dependency list, and encryption would have broken it. Cargo features square the circle: `chacha20poly1305 = { version = "0.10", optional = true }` plus `encryption = ["dep:chacha20poly1305"]` in `[features]`, and `#[cfg(feature = "encryption")] pub mod crypto;` in lib.rs. Consumers who never asked for crypto compile a crate with zero dependencies; the CLI opts in with one line in its own manifest. Features are additive compile-time doors, and "pay only for what you use" is the same promise the language makes at runtime.

## Use the ecosystem's crypto, audit the boundary

The module is deliberately thin: it serializes the payload chunk, hands it to a RustCrypto AEAD, and writes the result into a chunk. Everything cryptographic (the cipher, the Poly1305 tag, nonce generation from the OS RNG) comes from the audited crate; PRISM's only decisions are what is encrypted (the whole serialized payload chunk), what is authenticated but plaintext (the HEAD payload, as associated data), and how the bytes lay out. Those are format decisions, and they are the only ones a format should make. The spec's "no homebrew ciphers" rule shows up in code as an import line.

## AEAD in one paragraph

Authenticated Encryption with Associated Data gives confidentiality and integrity in one primitive: decrypt fails outright, cleanly, on a wrong key or any modified bit of ciphertext or associated data. The `tampered_header_fails_authentication` test flips one bit in the plaintext HEAD, re-signs the CRC so the container layer is satisfied, and shows AEAD still refusing: CRC catches accidents, AEAD catches adversaries, and the test demonstrates the difference between the two.

## API shape: transform files, not secrets

`encrypt_file` and `decrypt_file` map whole-file bytes to whole-file bytes; keys are `&[u8; 32]` the caller produced, and prism-core never reads key files, prompts, or derives keys from passwords (a real deployment would add a KDF at the application layer). Keeping key management out of the library keeps the security-relevant surface small enough to actually review, which is most of what "secure design" means in practice.
