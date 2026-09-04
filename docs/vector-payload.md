# PRISM vector payload, normative spec (version 1)

Byte-level definition of the VECT chunk. Everything byte-aligned; multi-byte fixed-width integers little-endian. The architecture is the pooled one HVIF proved out: colors, styles, and paths are pools; shapes are cheap references into them, painted in order.

## Variable-length integers

VarUInt: 7 data bits per byte, low bits first; the high bit set means another byte follows. VarInt (signed) is a VarUInt of the zigzag mapping (0, -1, 1, -2, 2, ... encode as 0, 1, 2, 3, 4, ...): `zigzag(v) = (v << 1) ^ (v >> 63)`.

## Coordinates

All coordinates are VarInts in fixed-point units with `scale` fractional bits (from the payload header below). Path coordinates are deltas from the current point; style geometry (gradient endpoints) is absolute. The design-space size in units is the container HEAD's width and height; rasterizing at any target size maps design space onto the target uniformly.

## Payload layout

    scale        u8         fractional bits, 0..15
    color_count  VarUInt
    colors       4 bytes each, RGBA straight alpha
    style_count  VarUInt
    styles       see below
    path_count   VarUInt
    paths        see below
    shape_count  VarUInt
    shapes       see below

Style:

    kind u8: 0 = flat, 1 = linear gradient, 2 = radial gradient
    flat:   color_index VarUInt
    linear: x0 y0 x1 y1 (VarInt, absolute units), color_index_0, color_index_1
    radial: cx cy ex ey (VarInt; e is a point on the rim), color_index_0, color_index_1

Path: `instruction_count` VarUInt, then instructions, each an opcode byte plus delta coordinates:

    0 move_to    dx dy
    1 line_to    dx dy
    2 quad_to    dcx dcy dx dy
    3 cubic_to   dc1x dc1y dc2x dc2y dx dy
    4 close      (no operands; returns to the subpath start)

Shape: `style_index` VarUInt, `path_index` VarUInt, `fill_rule` u8 (0 = nonzero, 1 = even-odd).

## Rendering (informative in version 1)

The reference rasterizer flattens curves in Q16 fixed point (16 segments per cubic, 8 per quadratic), scan-fills at 4x4 supersampling into 17-level coverage, evaluates gradients in integer arithmetic (Newton integer square root for radial), and composites shapes in order with straight-alpha over. It is deterministic, but version 1 declares vector rasterization informative rather than conformance-normative; pinning anti-aliased rasterization down to the bit is deferred until the raster side's conformance corpus exists to host it.

## Limits

A decoder must reject: unknown style kinds or opcodes, indices out of pool range, instruction streams that draw before any move_to, and pools whose declared counts exceed the remaining payload.
