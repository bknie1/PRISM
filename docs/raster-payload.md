# PRISM raster payload, normative spec (version 1)

This is the byte-level definition the encoder and decoder are implemented against. It refines SPEC.md's raster section. All integers little-endian, everything byte-aligned.

## Design summary

QOI's op vocabulary, re-based on a 2D predictor. Where QOI encodes differences against the previous pixel in scan order, PRISM encodes residuals against a median edge detector (MED) prediction from the left and above neighbors, the predictor used by JPEG-LS. Runs encode "prediction exactly right," so a run covers flat regions AND smooth gradients. Tiles make decode partial and parallel.

## Container context

The raster payload lives in a RAST chunk. The HEAD chunk supplies width, height, bit depth (8), colorspace (sRGB), alpha mode (straight), and tile_size_log2. Version 1 fixes tile_size_log2 = 8 (256 x 256 tiles).

## Tiling

The image is divided into ceil(w/256) x ceil(h/256) tiles, row-major. Edge tiles are smaller. The RAST chunk is: a u32 tile count, then that many u32 byte lengths (the tile directory), then each tile's op stream concatenated. A decoder can seek to any tile by summing lengths. Each tile is coded independently with fully reset state.

## Prediction

For each pixel, per channel (r, g, b, a independently), with L = left neighbor, U = up neighbor, UL = up-left, all within the tile:

    if UL >= max(L, U): P = min(L, U)
    else if UL <= min(L, U): P = max(L, U)
    else: P = L + U - UL

Boundary rules inside a tile: first pixel P = (0, 0, 0, 255); rest of first row P = L; first column P = U. The gradient case L + U - UL is computed in signed 16-bit arithmetic; the branch conditions guarantee the result lies between min(L, U) and max(L, U), so it is always a valid 0..255 value and no clamp is needed.

## Decoder state per tile

prev: the previous decoded pixel in scan order, initialized (0, 0, 0, 255). index: 64 pixels, zero-initialized, hash (r*3 + g*5 + b*7 + a*11) % 64; every decoded pixel is inserted. Residuals are wrapping (mod 256) against P.

## Ops

Tag bits first, six ops, one byte minimum each:

    PRISM_OP_INDEX  00iiiiii             emit index[i]
    PRISM_OP_DIFF   01rrggbb             r,g,b residuals vs P, 2 bits each, -2..1 (bias 2); a = P.a
    PRISM_OP_LUMA   10gggggg rrrrbbbb    g residual -32..31 (bias 32); (r res - g res) and (b res - g res) 4 bits, -8..7 (bias 8); a = P.a
    PRISM_OP_RUN    11nnnnnn             n+1 pixels (1..62) that each equal their own prediction P exactly, all four channels
    PRISM_OP_RGB    11111110 rr gg bb    literal r, g, b; a = P.a
    PRISM_OP_RGBA   11111111 rr gg bb aa literal all four channels

Run lengths 63 and 64 are unavailable (tags 0xFE, 0xFF), as in QOI. Note RUN re-evaluates P for every pixel in the run; it is "the predictor was right n+1 times," which follows gradients, and degenerates to QOI-style flat runs when L = U.

## Encoder discipline (informative)

Single pass, no lookahead, first match wins: RUN (extend if cur == P), INDEX (if index[hash(cur)] == cur and that costs less than a smaller op does not apply; INDEX is 1 byte, tied with DIFF; prefer RUN > INDEX > DIFF > LUMA > RGB > RGBA). Encoders may do better; any op sequence that decodes to the correct pixels is conformant.

## End of tile

A tile's op stream ends exactly when width * height pixels of that tile have been emitted. No end marker; the tile directory carries the byte length.
