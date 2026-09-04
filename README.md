# PRISM

A universal image format: one container, a lossless raster payload with a format-defined reconstruction function for scaling, and a compact binary vector payload for geometric content. A low-level Rust project, spec-driven; the spec is written and agreed before any encoder code exists.

Current state: Phase 1 complete. The container and raster codec are implemented and round-trip tested; on the synthetic corpus PRISM compresses to 34.0% of raw against QOI's 38.2% and PNG's 35.9% ([docs/benchmarks.md](docs/benchmarks.md)).

Layout: [docs/SPEC.md](docs/SPEC.md) is the top-level spec; [docs/container.md](docs/container.md) and [docs/raster-payload.md](docs/raster-payload.md) are the normative byte-level definitions; [docs/research/](docs/research) holds the Phase 0 study notes; [docs/teaching/](docs/teaching) holds the Rust lessons the implementation generates; `crates/prism-core` is the dependency-free format library and `crates/prism-cli` the `prism` tool (encode, decode, info, bench, gen-corpus).
