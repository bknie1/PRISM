# PRISM container, normative spec (version 1)

Byte-level file layout. All integers little-endian.

## File layout

    magic    4 bytes   0x50 0x52 0x53 0x4D ("PRSM")
    version  u8        1
    chunks   ...       chunk sequence, END last

## Chunk layout

    type     4 bytes   ASCII
    length   u32       payload byte count
    payload  length bytes
    crc      u32       CRC32 (IEEE/PNG polynomial 0xEDB88320) over type + payload

A decoder must verify the CRC of every chunk it reads and must skip unknown chunk types (seek by length), which is the format's forward-compatibility mechanism.

## Chunk types, version 1

HEAD, required, first:

    width          u32     pixels, nonzero
    height         u32     pixels, nonzero
    payload_kind   u8      0 = raster, 1 = vector
    bit_depth      u8      8 (field reserved for future depths)
    colorspace     u8      0 = sRGB (reserved for future spaces)
    alpha_mode     u8      0 = straight (reserved)
    tile_size_log2 u8      8 in version 1; meaningful for raster only
    kernel         u8      0 = Catmull-Rom; the mandated reconstruction kernel

RAST or VECT, required, exactly one, matching payload_kind: the payload as defined in raster-payload.md (and the Phase 3 vector spec).

META, optional: reserved; contents defined later.

ENCR, optional (Phase 4): wraps the payload chunk in AEAD; when present, RAST/VECT is inside it rather than at top level. Details specced in Phase 4.

END, required, last: length 0, no payload.

## For vector payloads

width and height give the design-space size in units; the vector payload is resolution-independent and these define its aspect and default scale.
