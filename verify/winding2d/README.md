# winding2d: the signed winding number of a rational polygon

One Lean project (core Lean only, no Mathlib, toolchain `leanprover/lean4:v4.36.0`) that sketch
text (`WindingIsTopological`), sheet metal (`inLoop`) and the boolean's region rules import
instead of proving the theorem three times. `lake build`, then `lake env lean Axioms.lean`: every
theorem rests on `propext`, `Classical.choice` and `Quot.sound` only. No general-position
hypothesis anywhere.

Definitions (`Winding2d.Basic`): `Pt` (rational `x y`), `orient a b c`, `cross q a b` (the signed
half-open crossing of the ray from `q` along `+x`: a vertex at `q`'s height counts as below),
`sides vs` (closing side last), `wind vs q`, `OnSeg`, `Meets` (closed segments meet), `OffPoly`.

| Theorem | File | Statement |
|---|---|---|
| `wind_rot`, `wind_rot1` | Symmetry | rotating the vertex list changes nothing |
| `wind_reverse` | Symmetry | reversing it negates the count, at every point |
| `step` | Segment | per side, a move `p → p'` missing it changes its crossing by `phi p p' b - phi p p' a` |
| `sum_sides_diff` | Segment | the signed telescope round a closed polygon |
| `wind_eq_of_seg`, `wind_eq_of_path` | Segment | constant along any segment / polygonal path missing every side |
| `crossSoS_eq`, `windSoS_eq` | SoS | the SoS `side` rule (orient, else `sgn (b.x-a.x)`, else `-sgn (b.y-a.y)`) agrees with the half-open rule off the polygon |
| `crossSoS_ne_zero_iff` | SoS | the SoS crossing is nonzero iff sheet metal's rule counts the side |
| `side_shift`, `windSoS_shift` | SoS | `side` is the sign at `q + (ε², ε)`; the SoS count at any point (boundary included) is the half-open winding there, a point off the polygon |
| `ev_not_meets` | Witness | disjoint closed segments stay disjoint under small moves |
| `Simple` | Polygon | ≥ 3 vertices, no point side, consecutive sides meet only at their vertex, others not at all |
| `jump` | Jump | left of a side the count is one more than right of it |
| `vertex`, `corner` | Vertex | a side's left (right) value carries round every vertex |
| `wind_simple` | Jordan | **for a simple polygon there is `σ = ±1` with `wind ∈ {0, σ}` off it** |
| `wind_simple_mem` | Jordan | the same as sketch text states it: `0`, `1` or `-1` |
| `wind_simple_orient`, `exists_lowest` | Jordan | **by orientation**: `σ` is the sign of the turn at the lowest (then leftmost) vertex, never zero |

The argument for `wind_simple` (Jordan for polygons, no topology): every side has a left and a
right value one apart (`jump`); they carry round each vertex (`vertex`), so there are only two; a
point off the polygon, nudged to a height with no vertex (`generic`), takes the value of the first
side its ray meets, or `0` (`reach_some`, `reach_none`); and a point left of everything has `0` and
meets a side (`zero_value`).

Ported in part from verify/forge-exact/ForgeExact/Jordan.lean (branch
claude/repo-review-mbdrlj-dol-jordan, 8a2734c8): the sign lemmas, and the shape of
`seg_identity`/`telescope` (here signed, in the plane). No dependency on forge-exact.
