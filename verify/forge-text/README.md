# Sketch text's claims (kernel/forge-font, kernel/forge-sketch: winding.rs, profile.rs; interface/forge-lang: sketch_text.rs)

`text(..)` in a sketch draws each glyph as the font holds it, placed by the layout; its regions
are each part's contours nested by the winding rule; a glyph's region is the union of its parts'
(a composite's components), the text's the union of its glyphs', refused where two glyphs meet.

`lean/` (core Lean only; `lake build`, then `lake env lean Axioms.lean`: standard axioms, no
`sorry`). **Not built in the session that wrote it** (no toolchain there); the IEEE session builds it.

| Name | Kind | What it says |
|---|---|---|
| `Seg.place_kind`, `start_place`, `stop_place` | theorem | the layout moves a glyph's pieces by one offset, kind and control points kept: the outline is the font's |
| `Seg.reverse_reverse`, `start_reverse`, `stop_reverse` | theorem | reversing a piece swaps its ends and twice is itself |
| `step_out_prefix` | theorem | the TrueType walk only appends pieces |
| `DecodeCloses` | claim | the TrueType walk's pieces chain from its start and close |
| `polyWinding` examples | `decide` | a square's count inside 1, outside 0, on a side undecided |
| `WindingFromAnyStart`, `WindingIsTopological` | claims | the count is the polygon's, and is the topological winding number for a simple polygon |
| `mem_textRegion` | theorem | the text's region is the union of every glyph's parts, in any order |
| `RefusesMeeting` | claim | a text is set only if no two glyphs' boundaries meet |

## Levels (research/PROOF-STANDARD.md)

| Level | Sketch text has |
|---|---|
| L1 | the specs above: placed outline = the font's curves moved; nesting = the winding rule; a glyph = the union of its parts; refusal iff two glyphs meet. Curves are the spline-spec project's Béziers (control points only here) |
| L2 | the exact references: the font's own numbers on an integer grid (TrueType integers, halves; CFF 2⁻¹⁶), `orient2d` exact, OpenCascade rebuilding every text part (tools/occt, 11 parts agree) |
| L3 | stated, not yet proven: `DecodeCloses`, `WindingFromAnyStart`, `WindingIsTopological`, `RefusesMeeting` |
| L4 | not yet: the Lean models are written from the Rust. Targets for Aeneas: forge-font's glyf and CFF readers (integer-only), the TrueType `contour` walk, the line crossing (`winding::line` on integers) |
| L5 | the curve pieces' counts rest on rounding bounds (winding.rs: `m` per split); the line test on forge_math's proven orient2d |
| L6 | differential in CI: OCCT oracle on text parts, text parts' exact bits pinned native = wasm32, font corruption fuzzing (forge-font, forge-lang fonts.rs) |
