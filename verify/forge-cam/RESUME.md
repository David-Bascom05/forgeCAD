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

## Still to do when asked

1. Soft jaws' `Certified`: the pockets' walls are arcs and segments, so this needs segment–arc
   (below) for walls over arcs.
2. If 2.5D contours with arcs are in active use: segment–arc in Dist2 (over ℝ, or ℚ(√d) for an
   executable form), then arc–arc.
3. Not yet asked for: the character level of G-code (digits and templates), canned-cycle
   expansion in the G6 model, L5 for the checker's intervals, and L4 (Aeneas).
