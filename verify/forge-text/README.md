# Sketch text's claims (kernel/forge-font, kernel/forge-sketch: winding.rs, profile.rs; interface/forge-lang: sketch_text.rs)

`text(..)` in a sketch draws each glyph as the font holds it, placed by the layout; its regions
are each part's contours nested by the winding rule; a glyph's region is the union of its parts'
(a composite's components), the text's the union of its glyphs', refused where two glyphs meet.

`lean/` (core Lean only, requiring verify/winding2d by path; `lake build`, then `lake env lean Axioms.lean`: standard axioms only, no
`sorry`; CI: .github/workflows/lean-forge-text.yml).

| Name | Status | What it says |
|---|---|---|
| `Seg.place_kind`, `start_place`, `stop_place` | proven | the layout moves a glyph's pieces by one offset, kind and control points kept: the outline is the font's |
| `Seg.reverse_reverse`, `start_reverse`, `stop_reverse` | proven | reversing a piece swaps its ends, and twice is itself |
| `decode_closes` | proven | the TrueType walk (`contour`): its pieces chain from its start and end back there, a closed contour |
| `contourOf`, `contourOf_closes` | proven | a contour from its points as `contour` takes them (from the first on point, or midway between the first two; the rest in order; back to the start), always closed. The Aeneas translation of `glyf_core::contour` is proven equal to it (verify/aeneas/forge-font) |
| `polyWinding` examples | proven (`decide`) | a square's count inside 1, outside 0, on a side undecided |
| `winding_from_any_start` | proven | the crossing count is the polygon's: rotating its vertex list changes nothing |
| `mem_textRegion` | proven | the text's region is the union of every glyph's parts, in any order |
| `refuses_meeting` | proven | the check before a text is set (every pair of loops of different glyphs) passing means no two glyphs meet |
| `winding_is_topological` | proven | for a polygon `Simple` in winding2d's sense, the count off it is 0, 1 or -1, and two points joined by a segment meeting no side count alike: winding2d's `wind_simple_mem` and `wind_eq_of_seg`, through `polyWinding_eq` |
| `lineCrossQ`, `lineCross_toQ` | proven | the line test on rationals (the kernel's doubles' values); the integer one is it on the points cast |
| `lineCross_eq`, `onSeg_of_within`, `sides_toQ`, `polyWinding_eq` | proven | the bridge: off the polygon, `lineCross` is winding2d's half-open `cross` on the points cast to rationals (its refusal happens only on the side), and the sides are the same, so `polyWinding` is `wind` |

## Levels (research/PROOF-STANDARD.md)

| Level | Sketch text has |
|---|---|
| L1 | the specs above: placed outline = the font's curves moved; nesting = the winding rule; a glyph = the union of its parts; refusal iff two glyphs meet. Curves are the spline-spec project's Béziers (control points only here) |
| L2 | the exact references: the font's own numbers on an integer grid (TrueType integers, halves; CFF 2⁻¹⁶), `orient2d` exact, OpenCascade rebuilding every text part (tools/occt, 11 parts agree) |
| L3 | proven for every input of the models: `decode_closes`, `winding_from_any_start`, `refuses_meeting`, `winding_is_topological` |
| L4 | the TrueType `contour` walk: kernel/forge-font/src/glyf_core.rs, translated by Aeneas, is proven to be `contourOf` (so closed) for every well-formed glyph (verify/aeneas/forge-font: `contour_spec`, `contour_closes`). The simple-glyph record reader (`read_simple`) is proven never to fail on any bytes, and its output well-formed (`read_simple_spec`, `contour_of_read`). The line crossing: the translated `winding::line` is `lineCrossQ` on the doubles' values, and `lineCross` on grid points, given orient2d's sign (verify/aeneas/forge-sketch: `line_spec`, `line_int`; `lineCross_toQ` here). The CFF charstring machine (cff_core.rs, integers) is proven never to fail on any bytes and to return only closed contours (`run_spec`) |
| L5 | the line test on forge_math's proven orient2d. **Open, and the code's margin is not sound as it stands**: the curve pieces' counts by hull subdivision (winding.rs `bezier`) widen the box by a margin sized by the current piece, while the rounding a small piece carries is sized by its ancestors. research/sketch-text/hull-margin.md shows a case 22× over the margin and the fix (size it by the root piece), after which the bound is provable |
| L6 | differential in CI: OCCT oracle on text parts, text parts' exact bits pinned native = wasm32, font corruption fuzzing (forge-font, forge-lang fonts.rs) |

## What `winding_is_topological` covers, and what it does not

- verify/winding2d is copied unchanged from claude/repo-review-mbdrlj-winding2d at ae22b628, so the two
  branches merge it as one file set; forge-text requires it by path (lakefile.toml).
- It is about polygons: sides that are lines, the line test of forge-sketch's `winding::line`. A
  glyph's curve pieces are counted by splitting them until their control hulls decide (L5,
  rounding bounds), which the Lean model does not cover.
- winding2d's `Simple` asks more than forge-sketch's check: forge-sketch refuses a loop that crosses
  itself or overlaps itself, but lets one touch itself at a point without crossing, which `Simple`
  forbids. The shipped faces break no part of `Simple`: interface/forge-lang/src/fonts.rs
  `shipped_contours_are_simple` checks every contour of Open Sans Regular and Bold and Source Sans
  3 over Latin-1 (at least three pieces, none a point; pieces that are not neighbours apart;
  neighbours meeting only at their shared end, straight continuations included), splitting pieces
  until their control boxes are apart by more than the splits' rounding and deciding the cones at
  a shared end by exact orient2d (`the_check_finds_touches`: a pinch, a side touching another's
  interior, a doubling back and a curve crossing its neighbour are all found).
