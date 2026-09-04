# Binary vector formats (Phase 0, feeds Phase 3)

Study of the compact binary vector tradition PRISM's vector payload joins. TinyVG details verified against its specification.tex 2026-09-04; HVIF, SWF, and IconVG sections are from general knowledge and get re-verified before Phase 3 design work.

## TinyVG (the closest prior art)

Magic {0x72, 0x56}, version byte, then a bit-packed header: scale (u4, fractional bits for fixed-point units), color encoding (u2: RGBA8888, RGB565, RGBAf32, custom), coordinate range (u2: 8/16/32-bit units), width/height, color count. Everything numeric uses VarUInt (7 data bits per byte, high bit continues, little-endian; 0..127 in one byte).

A color table up front, then commands referencing it by index. Twelve commands (end, fill polygon, fill rectangles, fill path, draw lines/loop/strip/path, outlined variants, text hint), each carrying a style: flat color index, linear gradient (two points, two color indices), or radial gradient. Gradient interpolation is defined in linear color space.

Paths are segment lists; each instruction is a tag byte (u3 type + line-width flag) then coordinates. Instruction set: line, horizontal line, vertical line, cubic bezier, arc circle, arc ellipse, close, quadratic bezier.

Lessons for PRISM: fixed-point units with a per-file scale is the right coordinate answer (no floats in the file, precision tunable per image); a color table plus VarUInt indices is the right color answer; the horizontal/vertical line specializations are cheap wins straight from SVG's path mini-language. TinyVG's weakness for us: no stroke joins/caps sophistication and no per-shape transforms, which keeps it simple but limits conversion fidelity from real SVGs.

## HVIF (the existence proof for tiny)

Haiku's icon format renders full desktop icons from roughly 500-700 bytes: separate style, path, and shape sections; coordinates in a compact fixed-point encoding with a flag scheme where common cases cost fewer bytes; gradients as first-class styles; shapes reference paths and styles by index so geometry and paint are deduplicated. The section/index architecture (styles and paths as shared pools, shapes as cheap references) is the single best idea to steal.

## SWF shape records

Flash encoded vector shapes as bit-packed edge records with delta coordinates and per-record bit widths declared inline; fills referenced by index with left/right fill semantics for shared edges. Proof that aggressive bit packing of deltas works, and a warning: SWF's bit-level reader complexity is exactly what PRISM's byte-aligned principle exists to avoid. We take delta coordinates, skip the bit packing in v1.

## IconVG

Tao again: byte-aligned-ish opcode stream for icons, registers for styles, aggressive magic-number opcodes for common values. Reinforces the same trio every format above landed on: indexed styles, delta/fixed-point coordinates, small opcode set.

## Direction for PRISM's vector payload (to be specced in Phase 3)

Byte-aligned opcode stream, VarUInt everywhere, fixed-point units with per-file scale, pooled styles and paths referenced by index (HVIF's architecture), TinyVG's instruction set as the starting command vocabulary, flat color and two gradient types in v1. Rendering target for the reference implementation: flatten curves, scanline fill with even-odd and nonzero winding.
