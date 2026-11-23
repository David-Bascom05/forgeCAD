# verify/aeneas/forge-weld: a section's measures, from the shipped Rust (L4)

`tools/aeneas/translate.sh forge-weld` translates the real `kernel/forge-weld` (release build), as
`translate.conf` selects, into `lean/ForgeWeldRs/{Types,Funs}.lean`. The proofs in `lean/` are
about that translation. Each rests on `propext`, `Classical.choice` and `Quot.sound` only
(`lean/Axioms.lean`), plus the trusted base of verify/aeneas/README.md (Charon and Aeneas
translate faithfully; T, binary64 arithmetic).

| Rust (kernel/forge-weld/src/section.rs) | Lean | what is proven |
|---|---|---|
| `top` | `top_eq` | `v` when it is greater, else `best` (by comparison, not `f64::max`, whose `±0` is the platform's) |
| `support_loop` on a loop of lines | `loop_lines`, `support_loop_lines` | the loop is the fold of `top` over the float dot products of each line's start and end, from `-∞`, with nothing opaque reached |
| | `support_loop_max` | its result is one of those dot products, finite, and no less than any (where each is finite) |
| `Vec2::dot` (forge-math) | `dot_err` | its error against the exact `p · w`: `2⁻⁵³ (|s| + |m₁| + |m₂|) + 2 · 2⁻¹⁰⁷⁵`, from the computed sum and products |
| | `support_loop_near` | **every corner has `p · w ≤ r + dotErr p`, and some corner `q` has `r - dotErr q ≤ q · w`**: with research/weld-lean's `support_of_corners` (a polygonal section's support is the largest `p · w` over its corners), the Rust's support is the exact support to within the largest `dotErr` |
| `line_moments` | `line_moments_eq`, `two_val`, `six_val` | operation for operation the binary64 evaluation of weld-lean's Green's-form terms (`cross / 2`, `lineMx`, `lineMy`; Section.lean); the literals are exactly 2 and 6 |

## What is opaque, and why

`lean/ForgeWeldRs/FunsExternal.lean` and `TypesExternal.lean` define them as Lean `opaque`
constants (no axiom). No theorem reaches one: they are about loops of lines.
- an arc's angle test and points: `on_arc` (`rem_euclid`), forge-math's `dmath::{atan2, TAU}`
  and forge-ops's `arc_point`. `support_loop` takes `w`'s angle only at the first arc, so a loop
  of lines never needs it;
- a spline piece (`Segment::Spline`): its `Arc` and the spline's ends.

The next steps: the arc branch (its reach `c · w + r |w|`, research/weld-lean ArcSupport.lean, in
ℚ(√·)), which needs `dmath::atan2` and `rem_euclid` translated; and frame.rs's crossings and
`reach`.

## Changes to the Rust for the translation

Bit for bit the same results (the cut lists and builds of the weld examples and tests compared
with the commit before):
- `Section::support` is `support_loop` over the outer loop: an index loop, `top` for `f64::max`,
  and the angle taken at the first arc, not up front;
- `moments` takes a straight piece's terms from `line_moments`.
