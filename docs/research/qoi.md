# QOI study notes (Phase 0)

Source: qoi.h reference implementation (phoboslab/qoi), which contains the full spec in comments. Verified 2026-09-04.

## The whole codec

14-byte header: magic "qoif", width u32, height u32 (both big-endian), channels byte (3 or 4), colorspace byte (0 sRGB, 1 linear). Then a stream of ops, then an end marker of seven 0x00 bytes and one 0x01.

Decoder state is two things: the previous pixel, starting at (0, 0, 0, 255), and a 64-slot index array of previously seen pixels, zero-initialized. Every decoded pixel is written into the index at slot `(r*3 + g*5 + b*7 + a*11) % 64`.

Six ops, all starting on a byte boundary. Two-bit tags first:

- INDEX `00iiiiii`: emit the pixel at index slot i. One byte reproduces any recently seen color.
- DIFF `01rrggbb`: each channel delta is 2 bits, range -2..1 (stored with bias 2), alpha unchanged. Deltas wrap mod 256.
- LUMA `10gggggg` + `rrrrbbbb`: green delta 6 bits (-32..31, bias 32); red and blue deltas are stored relative to the green delta, 4 bits each (-8..7, bias 8). Encodes the fact that channels usually move together and green carries most luminance.
- RUN `11rrrrrr`: repeat the previous pixel, run length 1..62 (stored with bias -1). Lengths 63 and 64 are unusable because those bit patterns are the two 8-bit tags below.
- RGB `11111110` + 3 bytes: literal RGB, alpha unchanged.
- RGBA `11111111` + 4 bytes: literal RGBA.

That is the entire format. The encoder is a single pass with no lookahead: try RUN, try INDEX, try DIFF, try LUMA, fall back to a literal.

## Why it works

Images are locally coherent. Runs catch flat regions, the index catches palettes and repeated colors, DIFF/LUMA catch gradients, and literals catch everything else at a cost of one tag byte over raw. Byte alignment costs compression but buys radical simplicity and speed; there is no entropy coder, no bit reader, no tables.

## Weaknesses (our twist lives here)

1. No 2D context. The predictor is only the previous pixel in scan order; the pixel directly above is ignored. PNG's filters (Sub, Up, Average, Paeth) use the previous row and win on photographs because of it. A QOI-class op stream with a previous-row predictor is the most promising PRISM twist.
2. The index is tiny and collision-prone; 64 slots with a fixed multiplicative hash, and a collision silently evicts. Larger index or better hash is a tunable.
3. No partial decode. One sequential stream; you cannot decode a crop without decoding everything before it. Tiling solves this and PRISM already wants it for the reconstruction renderer.
4. Big-endian header on exclusively little-endian consumer hardware; a swap on every read for network-order aesthetics.
5. No bit depths beyond 8, no HDR, single colorspace byte with no way to extend.

## What PRISM takes from it

The op-stream shape, byte alignment as a v1 principle, the single-pass encoder discipline, and the one-page-spec bar. The raster payload starts as "QOI plus a previous-row predictor plus tiles" and earns any further complexity through benchmarks.
