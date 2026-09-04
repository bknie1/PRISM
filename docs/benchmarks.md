# Benchmarks

Reproduce with `prism gen-corpus corpus` then `prism bench corpus`. Release build, Windows 11, single-threaded. Sizes in bytes; the corpus is four synthetic 512x512 RGBA images.

## 2026-09-04, Phase 1 initial

    image          raw        prism      qoi        png     | pr-enc  pr-dec  qoi-enc png-enc
    gradient       1048576    131923     263698     148692  | 4.3ms   4.2ms   1.1ms   1.9ms
    noise          1048576    1048148    1048081    1049236 | 21.8ms  17.7ms  2.4ms   6.5ms
    plasma         1048576    241085     281766     298299  | 8.6ms   6.4ms   2.3ms   4.3ms
    shapes         1048576    6840       7161       10584   | 3.5ms   3.5ms   0.4ms   0.8ms
    totals         4194304    1427996 (34.0%)  1600706 (38.2%)  1506811 (35.9%)

## Reading

The MED-prediction bet pays off exactly where docs/research/qoi.md predicted: the gradient lands at half QOI's size because a run in PRISM means "the predictor kept being right," which follows smooth ramps QOI has to spell out delta by delta. Plasma (the photo-like case) and shapes also beat both QOI and PNG. Noise is incompressible for every format, per the pigeonhole constraint in SPEC.md; the honest behavior is staying at ~100% of raw rather than growing, and all four formats manage it.

Speed: PRISM encodes and decodes slower than the heavily optimized qoi crate (MED costs branches per pixel, and the reference implementation favors clarity). Real photographs and screenshots should join the corpus before drawing ratio conclusions; synthetic images flatter predictors.
