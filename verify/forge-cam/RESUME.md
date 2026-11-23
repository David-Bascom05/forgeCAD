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

## Still to do when asked

1. `dist_region` (flat faces): the box cutoff and the trim's inside test (`Trim::classify`).
   This is the last part of hypothesis (i).
2. An executable ℚ(√d) form of the arc candidates, if the L6 tests want one.
3. Not yet asked for: the character level of G-code (digits and templates), canned-cycle
   expansion in the G6 model, L5 for the checker's intervals, and L4 (Aeneas).
