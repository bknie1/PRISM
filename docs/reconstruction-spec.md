# PRISM reconstruction, normative spec (version 1)

How a conforming viewer scales a raster payload. This resolves the five open problems from docs/research/reconstruction.md and is what prism-core implements. Kernel 0 (the only one in version 1) is Catmull-Rom evaluated in fixed-point integer arithmetic, so conforming implementations produce bit-identical output on every platform; no floating point appears anywhere in this spec.

## Decisions

1. Edges: sample coordinates outside the image clamp to the nearest edge pixel.
2. Alpha: all four channels are filtered independently as stored (straight alpha). The fringe caveat near hard alpha edges is documented and accepted for version 1.
3. Color space: filtering operates on the stored sRGB-encoded values directly (gamma-space filtering).
4. Minification is not defined in version 1; conforming scalers accept target sizes at or above the source size. (The stretched-kernel definition is reserved for a later version.)
5. Determinism: all arithmetic below is exact integer math; any implementation that follows it is bit-exact by construction.

## Coordinate mapping

For output size (W_out, H_out) from source size (W_in, H_in), the source x position of output column x is defined in Q16 (16 fractional bits, signed 64-bit intermediates):

    sx_q16 = ((2*x + 1) * W_in * 65536) / (2 * W_out) - 32768

with `/` truncating integer division. Then:

    base = sx_q16 >> 16          (arithmetic shift; floor)
    t    = sx_q16 & 0xFFFF       (Q16 fraction in [0, 1))

The four horizontal taps are columns base-1, base, base+1, base+2, clamped to [0, W_in - 1]. Rows are mapped identically with H_in and H_out.

## Weights

With t in Q16, compute t2 = (t*t) >> 16 and t3 = (t2*t) >> 16 (Q16, truncating shifts), then the Catmull-Rom weights in signed Q16:

    w0 = (-t3 + 2*t2 - t) / 2
    w1 = (3*t3 - 5*t2 + 65536*2) / 2
    w2 = (-3*t3 + 4*t2 + t) / 2
    w3 = (t3 - t2) / 2

where every division by 2 is an arithmetic shift right by 1 (floor). Then renormalize so the weights sum to exactly 65536:

    w1 += 65536 - (w0 + w1 + w2 + w3)

Renormalization makes flat regions reproduce exactly and removes the last truncation drift.

## Sampling

For each output pixel and each channel independently: with horizontal weights wx[i] and vertical weights wy[j] (both Q16) and the sixteen clamped source samples p[j][i]:

    acc = sum over i,j of p[j][i] * wx[i] * wy[j]        (i64; each term is Q32)
    value = clamp((acc + 2147483648) >> 32, 0, 255)      (round half up, then clamp)

At integer scale positions (t = 0) the weights are exactly (0, 65536, 0, 0), so a scale factor of 1 returns the stored pixels bit-exactly; the lossless guarantee and the reconstruction guarantee are the same math.

## Conformance

A scaler is conforming when it reproduces this arithmetic exactly. The reference implementation carries test vectors; any deviation in a single channel value on the test corpus is a conformance failure.
