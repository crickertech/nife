# 165. The rasteriser dependency, and whether the glyph atlas ships one face or four

**Status: PROPOSED.** Raised 2026-09-19 by milestone 435's lane, which found milestone 142 gated on
`DECISION` naming no decision, when half of what its gate describes was decided a month ago.
[§104](104-the-font-and-the-palette.md) (the rich-text font is DejaVu Sans Mono, and the palette is
Solarized) took the font and the palette on 2026-08-20; the gate never said so. This is the half
§104 did not take. *(Section number provisional until the merge queue lands it.)*

## What is being decided

1. **The dependency that renders the font**, which [§46](46-dependency-rule.md) (thin primitives or
   whole subsystems) makes a decision rather than a convenience.
2. **Whether the atlas ships one face or four**, which is a licence question four times over and a
   table four times larger, and which §104 is silent on.

## Question 1: the rasteriser

**The answer is already measured, twice, by two investigations that did not see each other.** What
is owed is the ruling, not the research.

A peer session measured cross-architecture output stability on 2026-08-20 at raw `f32` coverage,
over 95 ASCII characters at nine sizes across four fonts, on x86_64 and aarch64 with both `std`
floats and `libm`:

- **`ab_glyph_rasterizer` is byte-identical** across every combination.
- **`fontdue` 0.9.4 is not**, at default features. Its `simd` feature is on by default, compiles an
  SSE path on x86 only, and the four-wide prefix sum reorders float additions against the scalar
  accumulation. One pixel in 151,414 differed on JetBrains Mono.

A second investigation reproduced it at larger scale by a different route: 58,708 renderings across
fifteen fonts, identical between aarch64 and x86_64 for `ab_glyph`; 136 of 47,166 differing for
`fontdue`, **every failure exactly one pixel by exactly one 255th**. It pinned the mechanism: lane 3
of the prefix sum computes `(a3+a2) + (a1+a0)` where the scalar path computes `((a0+a1)+a2)+a3`, and
float addition is commutative but not associative.

**Why one pixel in 150,000 is a catastrophe here specifically**, rather than a curiosity: three
parties compute the picture without talking to each other (the terminal draws it, the kernel
predicts it through the direct map, the host grades QEMU's `screendump`), they compare **every**
pixel, and the negative control is a single letter changed. A rasteriser that disagrees between two
architectures breaks that agreement outright.

| | crate | cost |
|---|---|---|
| **A** | **`ttf-parser` plus `ab_glyph_rasterizer`**, pinned at `ab_glyph_rasterizer` 0.1.10. | The smallest graph on offer and the only one measured byte-identical across architectures. 0.1.4 through 0.1.8 panic with an index out of bounds on some in-bounds-adjacent geometry, fixed in 0.1.9, which is why the pin is specific. |
| **B** | **`fontdue`.** | Fewer crates. Needs `default-features = false` forced at **every** call site, which is a note somebody has to remember guarding the property the whole three-party check rests on. Its `FontSettings::scale` also feeds curve linearisation at load time, so the same glyph at the same pixel size renders differently depending on a number set elsewhere. |
| **C** | **Write the rasteriser.** | §46's first rule says write it if it is on the verification path, and this is. It is also a font rasteriser, which is a subsystem rather than a primitive, and nobody has priced one. |

**Recommendation: A, with one instruction attached.** Determinism decides it and dependency count
agrees, which is the comfortable case. The instruction is the one the cross-version measurement
produced: **quantise to `u8` at the boundary and never persist raw `f32`.** `ab_glyph_rasterizer`
0.1.5 removed the `1.0` cap on coverage and the raw hash changed; the 8-bit hash did not, because
Rust's float-to-integer cast saturates. That is the difference between a table that survived a patch
bump by luck and one that survives it by construction, and it costs nothing because 8-bit coverage is
what the table holds anyway.

**Two honest caveats, because §46 makes this a decision.** Neither project promises bit-stability
across versions; the measurements are observations. And **no rasteriser has been run on either
target**: every number above is a host measurement, because there is no rasteriser crate in this
tree, which is what §46's first rule would want checked before the dependency is taken.

## Question 2: one face or four

`crates/video_terminal` says **"bold is bright"**, and the reason is recorded: a bold weight needs a
second font, and at 8x8 a bold face is a smudge. **At an anti-aliased cell both halves of that
reason expire.** A bold face is legible and the atlas has room. So "rich" means shipping four faces
rather than one, which is four licences and a table four times larger.

| | | cost |
|---|---|---|
| **A** | **One face.** "Bold is bright" stands, as a recorded limitation rather than a default. | Smallest table, no new licence, and a terminal that cannot render italic at all. |
| **B** | **Four faces** (regular, bold, italic, bold italic). | What calef's word *rich* asks for. Four times the table, four licence checks, and it **disqualifies any family with no italic**, which is a constraint on §104's choice rather than a consequence of it. |

**Recommendation: B, and it should be ruled before the atlas is generated rather than after**,
because the generator, the table format and the attribute byte all differ between one face and four,
and retrofitting is regenerating everything. DejaVu Sans Mono has the faces, so §104's choice does
not constrain this.

**Would we still choose B if both cost the same?** Yes, and the honest form of the opposite is
available: if A is chosen, it is chosen on table size, which is an effort argument and should be
written as one.

## How reversible it is

**The dependency is the expensive half** (§46: adding one is a morning, removing one after a
subsystem is built on it is a project). The face count is expensive for a different reason: it is a
checked-in generated artefact and a cell-attribute layout, so changing it later regenerates the
table and touches the terminal's own storage.

## What is blocked until this is answered

**Milestone 142's increment three** (the glyph atlas) and everything downstream of it, which is
increments three to six. Increments one and two are built and needed neither.

**Not blocked:** increment six's palette, which waits on milestone 141's property check rather than
on this, and which §104 already chose.
