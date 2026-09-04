# Glossary

Every term of art used in the lessons and specs, in plain language. Terms assume the [plain-English intro](00-eli5.md) and nothing else.

**AEAD (authenticated encryption with associated data).** Encryption that also proves nothing was tampered with. Decrypting with the wrong key, or after any byte was changed, fails cleanly instead of producing garbage. The "associated data" is extra information that is not encrypted but is still protected against tampering; PRISM uses the file header for this.

**Alpha.** The fourth number in a pixel: how opaque it is, 0 (invisible) to 255 (solid). "Straight" alpha stores color and opacity separately; "premultiplied" alpha stores the color already scaled down by its opacity, which behaves better when blending layers.

**Anti-aliasing.** Softening the stair-step look of diagonal edges by partially coloring the pixels an edge passes through, in proportion to how much of each pixel it covers.

**Base64.** A way of writing binary data using only ordinary text characters, so it can live inside a text file. It inflates the data by about a third, which is the penalty SVG pays to carry a photograph.

**Bit and byte.** A bit is a single 0 or 1; a byte is eight of them, able to hold a number from 0 to 255. Pixel channels fit in exactly one byte each, which is not a coincidence.

**Bit depth.** How many bits each channel gets. 8-bit means 0 to 255 per channel; higher depths (10, 12, 16) allow finer color steps and brighter-than-white ranges (HDR).

**Byte-aligned.** A design where every meaningful unit starts at a byte boundary, so the decoder reads whole bytes instead of picking individual bits out of them. Simpler and faster, at some cost in compression. QOI and PRISM are byte-aligned; PNG's compressed data is not.

**Channel.** One of the four number-planes of an image: all the red values, all the green, all the blue, all the alpha.

**Checksum.** A small number computed from a block of data, stored alongside it. Recompute it on read: a mismatch means the data changed. Catches corruption, not tampering (see AEAD for that).

**Chunk.** A labeled, length-prefixed box of bytes inside a file. Container formats are sequences of chunks, and a reader can skip chunks whose labels it does not recognize, which is how old readers survive new format features.

**Codec.** Coder plus decoder: the matched pair of algorithms that turn pixels into compressed bytes and back.

**Colorspace and sRGB.** The agreement about what the numbers mean as actual colors. sRGB is the standard for screens, and its values are gamma-encoded: the numbers are spaced to match human vision rather than physical light intensity.

**CRC32.** The particular checksum PRISM and PNG use, 32 bits long, cheap to compute, and very good at noticing accidental corruption.

**Catmull-Rom.** The particular interpolation kernel PRISM mandates for scaling. Its useful property: at the original dot positions it returns the original values exactly, so scaling by 1x changes nothing.

**Delta encoding.** Storing differences instead of values. A line's points might be "start at 100, then +2, +2, +3" instead of "100, 102, 104, 107". Differences are usually small numbers, and small numbers compress well.

**Determinism.** Same input, same output, every time, on every machine. Sounds automatic but is not: ordinary decimal math on computers (floating point) can differ across machines and compilers, which is why PRISM's scaling math uses only whole-number arithmetic.

**Endianness.** The order a multi-byte number's bytes are written in. Little-endian puts the smallest part first and is what virtually all modern processors use natively; big-endian is the reverse. It only matters that everyone agrees, so the spec picks one (little).

**Entropy.** A measure of how unpredictable data is, which sets a hard floor on how small it can losslessly get. Entropy coding (Huffman, ANS, arithmetic coding) squeezes data close to that floor by giving common things short codes, at real complexity cost.

**Fill rule (nonzero and even-odd).** The rule deciding which regions count as "inside" a shape whose outline crosses itself. Even-odd toggles inside/outside at every crossing; nonzero tracks the direction of crossings. Both appear in SVG and PRISM.

**Fixed point (Q16).** Doing fractional math with whole numbers by agreeing that, say, 65536 means 1.0 (so 32768 means one half). All the precision decisions become explicit, and results are identical everywhere, which is what the reconstruction spec needs.

**Gamma.** The deliberate non-linearity in sRGB values: 128 is not half the physical light of 255, it is what looks about half as bright to a human. Math done directly on gamma values (as most software does, PRISM included, by declared choice) is technically working in "perceptual" rather than physical units.

**Gradient.** Two meanings here. In a raster image: a region where color changes smoothly. In vector styles: a paint that blends between two colors along a line (linear) or outward from a center (radial).

**Hash.** A function that mixes a value into a small number, used in PRISM to pick which of 64 memory slots a color goes into so it can be referred back to cheaply.

**Interpolation.** Estimating values between known ones. Enlarging an image is interpolation: the new pixels sit between original dots, and some rule must decide their colors from the neighbors.

**Kernel.** The specific weighting rule an interpolation uses: how much each nearby original dot influences the estimated value, as a function of distance. Nearest-neighbor, bilinear, Catmull-Rom, and Lanczos are kernels of increasing sophistication.

**Lossless and lossy.** Lossless: decoding returns every original byte exactly (PNG, QOI, PRISM). Lossy: decoding returns an approximation, with the difference discarded to save space (JPEG). Neither is "better"; they are different promises.

**Magic bytes.** A fixed signature at the start of a file identifying its format ("PRSM" here, "qoif" for QOI). The first thing a reader checks.

**MED predictor (median edge detector).** The specific guessing rule PRISM's raster codec uses: look at the pixel to the left, the pixel above, and the one diagonally up-left, and pick an estimate that follows edges instead of blurring across them. Borrowed from JPEG-LS.

**Nonce.** A number used once: a random value generated fresh for each encryption so that encrypting the same data twice produces different ciphertext. Never reused with the same key.

**Opcode, op.** One instruction in a byte stream: a tag saying what kind of operation follows and how to read its operands. PRISM's raster data and vector paths are both streams of ops.

**Pigeonhole principle.** If you have more pigeons than holes, some hole gets two pigeons. Applied to files: there are fewer short files than long ones, so no method can shorten everything. The reason "compresses everything" is always a false claim.

**Pixel.** One dot of the grid: four channel values (red, green, blue, alpha).

**Predictor and residual.** A predictor guesses each pixel from already-seen neighbors; the residual is how wrong the guess was. Store only residuals and the decoder (making the same guesses) can rebuild everything. Good prediction makes residuals small, and small numbers compress well. This is the heart of most lossless image compression.

**Raster.** An image stored as a grid of measured pixels. Photographs, screenshots, scans.

**Rasterizer.** The program that turns a vector recipe into actual pixels at a chosen size.

**Run-length encoding.** "The same thing, N times in a row" stored as one instruction. In PRISM the repeated thing is "the predictor was exactly right," which covers flat areas and smooth ramps alike.

**Supersampling.** Rasterizing at a finer grid than needed (PRISM uses 4x4 sub-dots per pixel), then averaging down, as a straightforward way to get anti-aliasing.

**Tile.** An independent square region of the image (256 by 256 in PRISM), compressed on its own, so a reader can decode just the part it needs and encoders can work in parallel.

**Varint and zigzag.** A varint stores small numbers in fewer bytes (values up to 127 in one byte). Zigzag folds negative numbers into positive ones (-1 becomes 1, 1 becomes 2, -2 becomes 3) so small negatives stay small for the varint. Together they make delta coordinates cheap.

**Vector graphics.** An image stored as drawing instructions (shapes, curves, fills) instead of pixels. Resolution-independent by nature. SVG is the ubiquitous text-based version; PRISM's vector payload is a binary one.

**Winding.** The direction (clockwise or not) an outline is drawn, which the nonzero fill rule counts to decide what is inside a self-crossing shape.
