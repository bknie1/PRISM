# Lesson 4: fixed-point math and determinism

What `prism-core/src/reconstruct.rs` teaches.

New to images or compression? Start with [the plain-English intro](00-eli5.md); terms like interpolation, kernel, and fixed point are defined in [the glossary](glossary.md).

## Why no floats

The reconstruction spec promises bit-identical output on every platform. Floating point cannot promise that across implementations: results vary with evaluation order, FMA contraction, and compiler flags. So the scaler uses Q16 fixed point: an i64 where the low 16 bits are the fraction and 65536 means 1.0. Multiplying two Q16 values gives Q32, which is why the final accumulator shifts right by 32. Determinism stops being a testing problem and becomes arithmetic: integers either match or the code differs.

## Headroom analysis, the fixed-point discipline

Before trusting `acc`, bound it: 16 taps, each at most 255 * 65536 * 65536, is under 2^52, comfortably inside i64. This bound-the-worst-case habit is the entire craft of fixed point; Rust helps by making the types explicit (`i64` appears at every multiplication site, so a future 16-bit-depth change will not silently overflow).

## Shifts are floors, division truncates

For negative numbers `-5 >> 1` is -3 (floor) while `-5 / 2` is -2 (toward zero). The weight formulas produce negative intermediates, so the spec says arithmetic shift and the code uses `>>`. A one-character choice that changes output bytes is exactly the kind of decision a format spec exists to pin down.

## Renormalization buys exactness

Truncation makes the four weights sum to slightly less than 1.0; `w1 += ONE - (w0 + w1 + w2 + w3)` forces the sum to exactly 65536. Consequences visible in the tests: a flat image scales to a perfectly flat image, and integer positions produce weights (0, 65536, 0, 0), which is why `identity_scale_is_bit_exact` passes and why scaling and losslessness are the same guarantee.

## The Rust in it

`axis_steps` is the iterator style: `(0..n_out).map(|x| ...).collect()`, building a `Vec<AxisStep>` where each element owns its taps and weights; precomputing per-axis turns an O(width * height) inner cost into a table lookup. The closure `let clamp = |v: i64| ...` captures `n_in` from the enclosing scope; small named closures like this replace the private helper functions C would need. And the hot loop indexes `image.pixels[ty * src_w + tx]` through a shared borrow: the scaler reads the image, returns a new one, and mutates nothing, which the signature `(&Image) -> Result<Image, Error>` states machine-checkably.
