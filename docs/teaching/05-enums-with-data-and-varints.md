# Lesson 5: enums with data, closures, and varints

What the vector payload (`prism-core/src/vector.rs`) teaches.

## Enums are tagged unions done right

`PathCmd` is the heart of it: `MoveTo(i64, i64)`, `QuadTo(i64, i64, i64, i64)`, `Close`. Each variant carries exactly its own operands, the size of the whole is the size of the largest variant plus a tag, and there is no way to read QuadTo fields out of a MoveTo; C's union-plus-enum-tag idiom with the safety made mandatory. `Style` works the same way, and `match *cmd { PathCmd::QuadTo(qx, qy, x, y) => ... }` destructures tag and payload in one pattern. The wire format's opcode byte and the in-memory enum tag are the same idea at two layers, and encode/decode are just the two directions of that correspondence.

## Zigzag varints

Path deltas are small signed numbers; VarUInt only shines for small unsigned ones. Zigzag interleaves the signed range (0, -1, 1, -2, ... to 0, 1, 2, 3, ...) with two bit tricks: encode `(v << 1) ^ (v >> 63)` (the arithmetic shift smears the sign bit into an all-ones or all-zeros mask), decode `((z >> 1) as i64) ^ -((z & 1) as i64)`. Small magnitudes of either sign become one wire byte. This is the same encoding protobuf uses, and writing it once beats trusting it forever.

## Closures as parameters

The rasterizer passes its coordinate transform around as `map: &dyn Fn(i64, i64) -> (i64, i64)`; `build_edges` and `style_color` do not know or care that the transform is two multiplies and a divide capturing the scale factors. `&dyn Fn` is dynamic dispatch (a fat pointer to closure state plus code), the flexible-but-indirect cousin of the generic `impl Fn` the axis code in reconstruct.rs uses implicitly. Note also `let map = move |...|`: `move` makes the closure own copies of the captured integers instead of borrowing them.

## Validation is the decoder's first job

`decode` rejects out-of-range indices, unknown opcodes, draw-before-move, pools longer than the payload, and trailing bytes, each with a named error. The type system helps (`get` plus `?` for every untrusted read, as in lesson 2), but the *policy* comes from docs/vector-payload.md's Limits section: the spec says what a hostile file may not do, and the decoder is the spec's enforcement arm. Fuzzing finds what this discipline misses; that is Phase 5 material.
