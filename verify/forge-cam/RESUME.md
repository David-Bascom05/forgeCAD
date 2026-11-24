# verify/forge-cam: paused (2026-11-23)

Paused by the master. The owner wants proofs only where features are under active work, so this
waits for CAM's next code: the G0-plunge fix (b6ec16c), vises and soft jaws. When this was
written the branch was claude/repo-review-mbdrlj-cam-lean, and the last proof commit was
758f0d3. Lean 4.36.0 and Mathlib v4.36.0, standard axioms only, no `sorry`.

To build:

```
lake exe cache get && lake build && lake env lean Axioms.lean
```

Expect about 2 min with the Mathlib cache. Post.lean's `step` takes about 45 s.

## The 34 theorems (Axioms.lean), by goal

| goal | theorems | file |
|---|---|---|
| the lemma the 2.5D checker rests on | `sd_hyp_mono`, `sd_hyp_eq_sdRZ` | Hypograph |
| clearance from faces | `clear_of_frontier`, `sd_le_sd_add_dist` (sd is 1-Lipschitz, for μ), `hyp_preconnected`, `clearFace_sound`, `certified_clear` | SignedDist, Prism |
| G1, no gouge | `G1_of_closedForm`; declared removals `G1_target_iff` (P' = P ∖ Δ) | Prism, Spec |
| G2, corrected (§7b, open form): T ∪ N out of the open c-neighbourhood of F, N misses int S | `G2open_of_closedForm` (Vise); `G2closed_of_closedForm` with a margin (Prism) | Vise, Prism |
| G3, rapids | `G3_of_closedForm` | Prism |
| G4, the result | `G4_lower`, `G4_part`, `G4_upper` (the leftover is exact), `leftover_slice` (per z band), `G4_held` (held by fixture, or uncut) | Spec |
| G5, machine limits (3-axis) | `G5_line_iff`, `G5_arc`, `lineOkQ_iff` (the ℚ check equals the spec) | Machine |
| G6, the post | `round_trip`, `G6`, `roundEven_isQuant`; `faulty_counterexample` (the printer at 155e2f2), `fixed_on_counterexample` | Post |
| soft jaws | `jaw_seat`, `jaw_seat_interior`, both needing H ∩ blank ⊆ grip slab (`jaw_seat_needs_slab` shows it is needed), any k | Fixture |
| pocket coverage | `passes_cover`: passes at r + k·s with 0 < s ≤ r cover the opening R ∘ B_r | Pocket |
| slab::dist, segment–segment | `ptSegSq_le`, `ptSegSq_attained`, `min4_le_of_disjoint`, `segSegLo_le`, `segSegLo_attained`, `segSeg_D`, `clearFace_segSeg` | Dist2 |

## Remaining hypotheses

- **`ClearFace` (i): D is a lower bound on the 2D distance.** For segment against segment this is
  proven (`clearFace_segSeg`). It is still a hypothesis for:
  - **segment–arc** (slab.rs: the common normal through the centre, the crossings);
  - **arc–arc** (the line of centres);
  - **`dist_region`**, the path against a flat face's region. This is the box cutoff plus the
    inside test (`Trim::classify`), so it needs a model of the trim's point-in-region test.
- **`TopAbove`** (containment): ops::certify checks it on every move, since 01f0a30. It stays a
  hypothesis, discharged at run time.
- **Each true face lies within μ of a lowered face.** This belongs to the lowering, and is not
  proven here.
- **The tool bodies are ρ-hypographs** with Z an interval and w ≥ 0 on Z. tool.rs's chain
  validation gives this.
- **The Δ search's bound.** lower ≤ sdRZ(D, h − z) over the cells is the interval extension's
  claim, which is L5 (ival.rs against verify/ieee754).

- **H.grid (G4's tool-radius reason).** An offset's loops lie within ~4·2⁻³² mm of the exact
  offset's boundary (each vertex and arc centre rounded to offset::GRID), so (F ⊖ B_{r−m}) ⊕
  B_{r+m} as computed, m = 1e-6, contains the opening (F ⊖ B_r) ⊕ B_r. Not in Lean (hyp). It
  bears only on the label "tool radius", never on G1–G6.

## The vise code (3e8a91d, 0a4c189): done

ForgeCam/Vise.lean adds 7 theorems (41 in all).
- The fixture model, a union of boxes, meets `Certified`'s face hypothesis with μ = 0.
- forge-cam's `fixture_collision` (every body, the cutter included, at limit = fclear,
  Clear iff lower ≥ fclear + μ) gives G2 for the open c-neighbourhood (`G2open_of_closedForm`).
  This open form is G2 (the master's decision); the closed form needs c' > c.
- `held_band`: a band with z1 ≤ jaw_top is held. forge-cam labels a straddling band "uncut" whole,
  and allows a 1e-9 slack; `band_split` gives the exact split.
- The G6 counterexamples round-trip on all four dialects at 0a4c189 (checked in Rust).
  Vanishing arcs (start = end after quantisation, which a control reads as a full circle) are
  outside the G6 model. The model treats arcs as records, so 0a4c189's refusal of them is
  untested by it.

## Soft jaws (5b32f0a) and arc semantics: done

- `softJaw_seat` covers forge-cam's model, the pocket M × [seat, top] with exact seating.
  - Its grip proviso (held material inside the blank only within [seat, top]) is not checked by
    `soft_jaw`. Stock the earlier setup left above the part's top would, turned over, sit below
    the seat.
  - `softJaw_needs_grip` shows that proviso is needed.
- The three leftover mismatches, at 99f39bf:
  - the straddling band is fixed (jaw_top is a breakpoint);
  - the 1e-9 slack remains (it matches the breakpoints' dedup);
  - G4 still grows by tol only, not δ + tol (phase1.md gap 6).
  - New: "held" is now decided per loop by box overlap with a grown fixture piece, so a held
    loop may hold points away from the fixture.
- Arcs:
  - `arc3_unsafe`: 0a4c189's vanishing-arc rule compares all three coordinates. A short
    helical arc whose X, Y round onto the start is read as a full helical turn. Confirmed in
    Rust at 99f39bf: `G3 X5. Y0. Z-0.5 I-5. J0.` from X5 Y0.
  - `arc2_safe`: comparing X and Y is safe.

## 4bfe2a2 checked (go from the master, 2026-11-23)

- forge-post's arc rule is now exactly `quantizeArc2`. `arcFull_agrees`: its meant `full`
  equals the control's reading on every posted arc.
- soft_jaw refuses held material above the part's top inside the blank's footprint. That is
  `softJaw_seat`'s grip hypothesis.
- G4 grows the part by max(leave) + tol (δ + ε). Its near-face rule is
  `near_section_sqrt` (it also shrinks the radius by 1e-12 relative, which is conservative).
- The ℚ oracle: Oracle.lean (segseg, ptseg, apart), in CI.

## Segment–arc and arc–arc (DistArc.lean): done

`segArcLo_le` and `arcArcLo_le` prove slab::dist's low end for pairs with arcs is a lower bound
for every pair of points: exact signs, arcs counter-clockwise and less than a half turn (a
clockwise arc is the counter-clockwise arc with its ends swapped). `clearFace_segArc` and
`arcArc_D` discharge ClearFace (i). The soft jaws' pocket walls over arcs (quarter turns at most)
are covered.

## L6 and degenerate segments

- The CAM session's L6 harness (2ec3c15, examples/oracle_segseg.rs) against Oracle.lean: 2000
  cases, 0 outside (run here).
- `segSegLo_le_all` covers points too (a plunge or drill move projects to one).

## DistArc's precondition in the code (c316607)

- Walls of revolution are lowered to quarter turns.
- `Prism::of` refuses walls not strictly under a half turn (they go to the B&B).
- `slab::dist` debug-asserts every arc is strictly under a half turn, with the sign set by
  `ccw`.
- Its sign is exact since 815d10e: `Prism::of`'s guard and the assertion take cross(a − c, b − c)
  as an outward-rounded interval. A wall gets the closed form only if lo > 0 (ccw) or hi < 0
  (cw); otherwise it goes to the B&B. The guard can err only toward the general path. Done.
  (CAM's message named it 6e3fa7c; the branch has it as 815d10e.) The interval guard suffices: no
  exact orient2d is needed (the master, 2026-11-23).

## Helix moves and the trim fix (CAM 787239e)

- `Move::Helix` (ramps) is already covered. `certified_clear` takes any xy curve γ and tip heights
  ζ(s) in a set Zt. A helix is γ an arc and ζ linear in [z0, z1], so `HeightsIn ζ (Icc z0 z1)`.
  The checker's Δ search over H − [z0, z1] is ClearFace (ii) with that Zt.
- The trim fix bears on `dist_region` (item 1 below). Trims are now closed exactly: a trim arc is
  its circle's arc of radius r plus radial pieces to its stored ends. The model of `dist_region`
  should take that definition of the region.

## The shadow closed forms (CAM shadow.rs at 007ce37): done

Shadow.lean proves S1–S4, with `shadow_face` and `certified_clear_gen` assembling them.
- The frame (n, ê) and the dot products are exact over ℝ. shadow.rs encloses them in intervals
  (checked: n from an interval |e|, ê = (n.y, −n.x)). That is L5.
- Mismatch, sent to CAM: the cone's whole-move `dmin = pt_sec(axis, Sec::Arc)` uses the arc's
  stored f64 ends with no gap lowering (since 3a388e5, `dist` and `dist_region` lower).
  `arc_near` proves dmin − gap sound; or use ||c − o| − r|.
  - Closed in cam/path-arc-ends, wider than reported: the PATH arc's stored ends (Move::planar's
    f64 mids of the ends' enclosures) were taken as exact in slab (dist, dist_region: only the
    face's gaps were lowered), in the shadow cone's dmin, and in the B&B's planar closed form
    (cell_eval_planar's sector). Each now widens the path's distances by Move::path_gap =
    2·g + 2ε (g = arc_gap of the stored arc, ε = the ends' enclosure half-widths): the true arc
    and the model arc M (Trim.lean's: circle arc between the stored ends' rays plus radial
    pieces) are within 2ε of each other (argument in path_gap's doc), and the candidates bound
    distances to M once lowered by 2·g (segArc_trim, arcArc_trim, arc_near). The 2ε step is
    argued in the code; its metric steps are proven (PathEnds.lean: `proj_near`, `chord_mono`), the cone decomposition of the whole arc not yet (hyp H.pathends, that step only). Seen in the bits test: a bull at
    radius 11.29 about a 7.3 cylinder was a Hit at d = −0.0100000000000002 (past −0.01 by
    2e-16, on the unaccounted ends), now Undecided; refused either way. Digest re-pinned
    (native = wasm32-wasip1: 0x18f7a4235d47008e).

## Still to do when asked

1. Done: `dist_region` and the point-in-trim test (Trim.lean).
   - The stored-ends mismatch is fixed by CAM in 3a388e5: lo − 2·arc_gap per arc piece,
     outward-rounded and clamped at 0, for trims and for cylinder walls.
   - The perturbation lemma is proven. `segArcLo_shift` and `arcArcLo_shift` show that ends moved
     along their rays by ≤ g change √lo by ≤ g. `segArc_trim` and `arcArc_trim` give slab's
     bound against the full trim piece (circle arc plus radial pieces).
   - CAM's regression test is 5b97845 (slab::gap_tests): ends 1e-3 off the circle, 2000 points; it
     fails without the 2·gap and passes with it. Commit b2195f5's message names the fix 0b56c1e;
     it is 3a388e5.
   - `Piece.ok` is enforced in release since CAM bfa4a72 (`arc_ok`: r > 0, ends off the centre,
     interval-proven cross > 0 in the arc's sense; the gap bound is `arc_gap`'s, outward-rounded).
     A failing piece gives lo = 0. Checked against Trim.lean.
2. An executable ℚ(√d) form of the arc candidates, if the L6 tests want one.
3. Not yet asked for: the character level of G-code (digits and templates), canned-cycle
   expansion in the G6 model, L5 for the checker's intervals, and L4 (Aeneas).

## 3-axis surfacing (Drop.lean, CAM-3AXIS, 2026-11-24): done

The checker of drop.rs (the tool's column, research/cam/phase3.md). 10 theorems, standard axioms.
- `sd_antitone`, `column_compl_nonempty`, `sd_body_ge_of_column`: a nonempty body inside the
  column is at least as far from every point as the column, so the column's proven gap holds for
  the cutter and a shank no wider than it (ops::certify_with).
- `sd_column_ge_vertical`, `sd_column_ge_radial`: the depth caps gap_move takes.
- `drop_clear`: a tip above the drop's upper bound keeps the column off the face.
- `ring_rho_ge`, `ring_bound`, `line_rho_ge`, `line_bound`: spheres, horizontal cylinders and tori
  about a vertical axis reduce to the vertical plane through the tool's axis (b monotone).

Open: the 1D closed forms' candidates (`disk_drop`, `bowl_drop`, `plane_support`, and `conj`'s
maximiser ρ* and bound B(λ) for Concave.lean, all L6-tested) contain the
maximum; the sphere bound S − sd(centre) for a convex column (supporting half-space); each
cutter lies in its column (`Column::of`, L6-tested); the z-map's soundness; L5.

## A dome cylinder's exact move bound (Concave.lean, CAM-3AXIS, 2026-11-24): done

drop.rs `cyl_chord`, `section`, `Column::sq_convex` (research/cam/phase3.md). 12 theorems, standard
axioms.
- `disk_conj`, `disk_conj_at`: √(r² − e²) − λe ≤ r√(1 + λ²), attained at e* = −λr/√(1 + λ²).
- `top_concave`, `neg_profile_concave`: the disk's top and −b(|u|) (b convex, non-decreasing) are
  concave, so the cylinder's CL across its axis (their sup-convolution) is.
- `end_of_outside`, `move_clear_of_outside`, `move_le_unconstrained`: along a line move, a
  maximiser beyond an end makes that end decide; else the unconstrained maximum bounds it.
- `end_rho_ge`, `section_mono`, `ball_section`: past the face's end by a, the column's section a
  off its axis (a ball's is a ball).
- `superadd`, `profile_superadd`: h = b ∘ √ convex gives b(√(u² + a²)) ≥ b(|u|) + b(a) − b(0).

Open: that `conj` finds the profile's maximiser ρ* and B(λ) (the enumeration over pieces; L6-tested
in tests/drop_vs_sampling.rs), and L5.
