# Post-QOI landscape and the heavyweight codecs (Phase 0)

What happened after QOI, and what the big formats do, so PRISM knows where it sits. Sources: Nigel Tao's QOIR writeup (nigeltao.github.io/blog/2022/qoir.html), format documentation. 2026-09-04.

## QOIR: the cautionary tale

Tao (author of Wuffs and IconVG) built QOIR as a serious QOI successor: it compresses smaller and runs faster than QOI, using tiling and an LZ-style backend. The costs and conclusions matter more than the design:

- QOIR grew to roughly 3000 lines of C plus 4000 lines of data tables. The one-page-spec property died in the pursuit of ratio.
- His verdict: for lossless, WebP already sits at a strong point on the decode-speed versus ratio Pareto frontier, with near-universal browser support. Chasing that frontier head-on is a losing game for a small format.
- His encouraging meta-lesson: rolling your own image format is genuinely tractable when you prioritize speed and a specific niche over ratio supremacy.

PRISM's read: do not chase QOIR. The differentiator is the mandated reconstruction spec plus tiling for partial decode, with compression that is merely good. This is exactly the "most use cases, not edge cases" call in SPEC.md.

## JPEG XL, one level down

Two coding modes. VarDCT is the lossy photographic mode (DCT-family transforms, adaptive quantization). Modular mode is the lossless one and the interesting reference for us: reversible color transforms, per-channel predictors chosen from a fixed set (including self-correcting weighted predictors), meta-adaptive context trees, and ANS entropy coding. It is the state of the art in lossless ratio and the complexity budget shows it; the spec runs to hundreds of pages. We borrow one idea at most: prediction quality is where lossless ratio comes from, which supports our previous-row predictor bet.

## AVIF and WebP in one sentence each

AVIF is an AV1 video keyframe in a HEIF box; WebP is a VP8 keyframe (lossy) or a dedicated LZ77+Huffman coder (lossless) in a RIFF box. Both inherit enormous complexity from video codecs and both confirm the pattern: modern raster formats are containers around a prediction engine plus an entropy coder.

## Where PRISM sits

Speed and simplicity near QOI, ratio better than QOI via 2D prediction, plus two things none of them have: format-defined scaling and a vector payload in the same container. That is a defensible niche precisely because it is not on the ratio frontier.
