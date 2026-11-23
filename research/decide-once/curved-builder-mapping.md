# The curved builder on today's curved boolean: the mapping (one page)

This follows the owner's ruling of 2026-11-23: the common path runs in f64, proven through the
IEEE 754 Lean model, and exact algebra is only a fallback for what the proven screens cannot
decide. The intended shape is today's fast float path doing the work, with its tolerance decisions
replaced by screens that have proven bounds. Costs are in curved-cost-estimate.md: today's curved
boolean costs about 0.15 ms, and an exact common path would cost about 0.5–2 s, so it is withdrawn.

`B` is kernel/forge-bool/src/boolean.rs and line numbers are at 816353d. Every exact step is marked
**→ float** (it is on the common path, so it moves to a proven float with a screen) or
**fallback** (degenerate only, so it stays exact).

## Reused as they are

| today | where | in the curved builder |
|---|---|---|
| pair loop: `near_b` sorted by name, the nested loop over overlapping `face_box`es | B:1097–1186; `classify::Index`, classify.rs:391/451 | candidate pairs and their fixed order, as A8 uses them (replaces my stage (a) `near_pairs`) |
| n-ary pairs: `Grid` and the face loop | nary.rs:192, 925–960 | the same, for history booleans |
| `Scratch` (cuts, segs, pieces, specs), held thread-local | B:799, 819 | the arena, reset per boolean |
| closed-form plane–quadric curves: plane–cylinder lines, circle or ellipse; plane–sphere; plane–cone conics | forge-geom intersect.rs:942, 1011, 1077 | the crossing curve of a plane–quadric pair. A circle or ellipse is exact data (centre, axes, radii as f64), so no trace is needed |
| closed-form quadric pairs: parallel cylinders, cylinders with equal radii and crossing axes, sphere–sphere, coaxial pairs | ssi.rs:262, 337, 366; meet.rs:1650 | the same |
| `meet::intersect` (discriminant events, then a float chain via `intcurve::finish`) and `intcurve::trace` | meet.rs:1113; intcurve.rs:533 | the crossing curve of a general quadric pair. Its events become the builder's events; the bound is below |
| `ssi` `MET` memo | ssi.rs:161 | kept |
| naming (`piece_name`, `vertex_names`, `region_name`, `rank_pairs`) and `apply` | B:3449–4032, 1942 | the write, unchanged, as in A8 |

## Wrapped: the same structure, with screened decisions instead of LINEAR

| today | its tolerance decision | the screened decision | exact |
|---|---|---|---|
| `ssi::closed_or_flush`: which closed form applies | ANGULAR/LINEAR, `nearly_flush` NEAR=1e-4 | exact carrier classes (stage (a): parallel = cross product zero, coincident = same class); in f64 from the stored frames with a proven bound, and exact only when the screen straddles | **→ float** (the class key); **fallback**: coincidence/parallel within the bound |
| `trim`: edge crossings (`plane_meet`, `curve_surface_in` closed form or subdivision) and `on_edge` (RIM=10·LINEAR) | LINEAR, merging cuts within `max(tol)` | a crossing is a sign change of the other carrier's value along the trim (or of the trim's along the curve), each sign a proven-bound f64 evaluation of the quadric (Horner, the `sign_filter_iv` contract) | **→ float**: my stage (c) `cert_vertices` (exact `propose_triple`) is replaced by an f64 vertex point with a proven enclosure; **fallback**: an enclosure that holds a second root (a tangency) |
| `trim`'s `inside` (samples at 0.5/0.382/0.618, `point_in_face_past`) | LINEAR, sag bound | facets come from the arrangement of trims and crossing arcs per face (route (iii)); inside a face is the cyclic order at vertices, screened by angle | **→ float** |
| `split_face`'s walk (`uv_dir`, `clockwise`, `tied`/`by_chords`), `plane_split`, `cylinder_outline`, `uv::draw_loop` | 1e-12, LINEAR, sampled polygons | the per-face graph: sub-edges, the angle sort at each vertex with an angle screen (tangent directions in f64, a proven bound on the cross product), cycles, holes by a cycle-area screen; flat decide-once's face_fast screens, carried to curves | **→ float**; **fallback**: tangent arcs at a vertex (the cross product within its bound), decided by the exact curvature order |
| `classify_specs` (lean at 1e-9, flood, ray casting with `RAYS`) | 1e-9, LINEAR | inside from face sense (same_sense) and the core's flood; a ray only for a NoSeed class, its hits screened | **→ float**; **fallback**: a ray hit at a vertex or tangent (re-cast, as today) |
| `pinched_*`, `undecided_touch`, `refuse_strays` | LINEAR reach | the ledger's single decisions; a touch is a core refusal (ZeroThickness), as in A8 | — |

## Replaced

- The retry ladder (CAREFUL, ABSORB_HAIR_ROOTS, CHORDS, NO_RUN_ON, FENCES; B:951–1052). Each rung
  exists because a tolerance decision can disagree with another. With screens there is one
  decision per incidence, so there is nothing to retry.
- LINEAR snapping, `settle`, the ±1e-3·width re-snap, and `grown_tols` (B:4860–4946, 1324–1357,
  2120). A vertex is the screened meeting of its carriers; its tolerance is the proven enclosure.
- The imprint column (imprint.rs) stays as it is: it is off by default and the builder doesn't
  touch it.

## Exact work on the common path, and why none is left

My stage (b) and (c) code does exact rational work on plane–cylinder and cylinder–cylinder pairs:
resultant events, edge proposals per slab, branch nodes, and `propose_triple` at vertices. It was
written for certification, not speed. It costs 3.8 ms per plane–cylinder pair, 0.55 s or more per
cylinder–cylinder pair, and about 1 ms per vertex, against 0.15 ms for today's whole boolean.

Under this mapping:
- Plane–quadric curves are today's closed forms.
- Quadric–quadric curves are today's `meet::intersect` chains with a proven bound.
- Vertices and crossings are f64 roots with proven enclosures.

The exact code (forge-alg, `decide_once_curved`'s proposers, the reference) moves behind the
screens as the fallback: true tangencies, coincident or parallel carriers, three carriers meeting
within the bound. It is unchanged and still certified. The fallback rate will be measured on the
gate lines (FORGE_CURVED_STATS) before any speed claim.

## Float contracts to send to the IEEE session

1. **Quadric sign at a point** (Horner in `Interval`): the enclosure holds `q(p)`, given exact
   coefficients enclosed by `lo_f64`/`hi_f64`. This is `sign_filter_iv` as it stands.
2. **Closed-form conic of a plane and a cylinder or sphere** (centre, axes, radii): the error of
   each f64 field, from the intersect.rs formulas as written.
3. **Root of a quadric along a line or conic** (the crossing parameter): the enclosure from one
   float solve plus an interval Newton/Krawczyk check, which proves exactly one root.
4. **Tangent cross product at a vertex** (the angle screen), and **a cycle's signed area** (the
   area screen): bounds as flat decide-once's touches/angle/area contracts, extended to conic arcs.
5. **`meet::intersect` chain points**: the distance of each chain point from both carriers, from
   contract 1 at the point, with interval Newton for the event points.
