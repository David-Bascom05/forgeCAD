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
| G2, corrected (§7b): T ∪ N misses F ⊕ B_c, N misses int S | `G2_of_closedForm` | Prism |
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

## When the vise code lands

1. Check that `fixture_collision` certifies **both** T and N against the vise or jaws at a c'
   strictly greater than c. `G2_of_closedForm` needs c < c', because F ⊕ B_c is closed.
2. Check that the vise's jaws lower to prism faces that are within μ of the true faces and
   cover their frontier. That is `Certified`'s hypothesis.
3. Check that the soft-jaw seating check implements H ∩ blank ⊆ grip slab ("pocket no deeper
   than the part").
4. Check that the "held by fixture" leftover is computed as `leftoverHeld` (S_end ∩ K less the
   grown part).
5. If 2.5D contours with arcs are in active use: segment–arc in Dist2 (over ℝ, or ℚ(√d) for an
   executable form), then arc–arc.
6. Not yet asked for: the character level of G-code (digits and templates), canned-cycle
   expansion in the G6 model, L5 for the checker's intervals, and L4 (Aeneas).
