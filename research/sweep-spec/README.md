# sweep-spec: the profile-swept solids to the proof standard

Covers extrude (every extent), taper, revolve, sweep, loft, coil and thread.

- [DESIGN.md](DESIGN.md) is the design: one shape for all of them, the domains, the refusal rules
  and the levels.
- [TAPER.md](TAPER.md) is what the taper fix is due to give.

The Lean is in verify/forge-exact/ForgeExact/Swept. It is core Lean only, and every theorem is
listed in Swept/Axioms.lean, which allows only the standard axioms. The exact reference is the
`sweep-ref` executable (Swept/Ref, TaperRef and LoftRef). The differential is `sweep-diff`, in
verify/forge-exact/fuzz; its digest is fuzz/tests/sweep-diff.digest, checked in the forge-exact
workflow.

## What is proven

| file | theorem | says |
|---|---|---|
| Prism | `coords_at`, `at_coords` | a sketch frame with det ≠ 0 is a coordinate system (Cramer's rule) |
| Prism | `prism_iff` | the reference's membership test is the extrude's set |
| Prism | `through_finite` | a range that holds the part's extent cuts or keeps exactly like the infinite prism, so the "through" margin is exact |
| Prism | `range_twoSided_union` | a two-sided extrude is one prism |
| Taper | `corner_on_both`, `corner_unique` | the mitred corner `q + δw` is the meet of the two offset carriers |
| Taper | `edge_length`, `contact_distance` | edge lengths and corner–carrier distances are linear in δ, so every event is the root of a linear function |
| Taper | `hexagon_side` | taper_collapses.fg's sides are 2 − 2δ/√3 long, so the hexagon collapses at h = √3/tan 80° |
| Rev | `lt_mul_sqrt_iff` | the sign of A − √s·B without a square root, so a revolve of a polygon is decided exactly over ℚ |
| Rev | `reach_arc` | an arc's least reach along a direction is at an end or at c·u − r, which is the kernel's exact `Segment::reach` |
| Screw | `same_point_iff`, `coil_injective` | a coil meets itself exactly when the section meets its translate by a whole number of turns that fits |
| Loft | `ruled_translate` | a ruled loft between translates is the prism, which is how the kernel builds an extrude along a direction |

## Levels

L1 to L6 are the levels of research/PROOF-STANDARD.md.

| feature | L1 | L2 | L3 | L4 | L5 | L6 |
|---|---|---|---|---|---|---|
| extrude, polygon (distance, symmetric, two-sided, along, through) | yes | ℚ | refusals decided exactly (domain) | interim (differential) | exact comparisons | repo and random |
| taper, lines | the algebra, the rule | ℚ (rational edge lengths) | the rule, still due in the kernel | interim | — | random skylines |
| revolve | sign lemma, arc reach | — (QForm or the sign lemma, next) | `CrossesAxis` exact in the kernel | — | — | — |
| coil, thread | `coil_injective` | — | `TurnsTouch` is sufficient, not exact (reported) | — | — | — |
| loft, ruled | `Ruled`, `ruled_translate` | ℚ (homothetic sections) | kept as is | interim | — | random |

## Kernel changes

**Exact axis reach.** `Segment::reach` (forge-ops profile.rs) replaces the 32 samples per arc and
circle in revolve's side choice and `CrossesAxis`. An arc that dipped 3e-5 across the axis between
two samples used to build a self-overlapping revolve; it is now refused. The test is
`an_arc_dipping_across_the_axis_between_samples_is_refused`, which fails on the old code.
