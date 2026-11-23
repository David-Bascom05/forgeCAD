# verify/forge-cam: CAM's L1

This folder proves the obligations G1–G6 of research/cam/design.md §5.2 in Lean 4.36.0 with Mathlib
v4.36.0, over ℝ. The executable parts run over ℚ or ℤ. There is no `sorry`, no `native_decide`,
and only the standard axioms (`lake env lean Axioms.lean`). CI: .github/workflows/forge-cam.yml.

```
lake exe cache get && lake build && lake env lean Axioms.lean
```

| file | what it proves |
|---|---|
| ForgeCam/SignedDist.lean | `sd`, the signed distance; `sd_translate`; `sd_le_sd_add_dist` (sd is 1-Lipschitz, for the lowering's μ). **`clear_of_frontier`**: take a connected body X and a closed set A. If X is at signed distance ≥ c > 0 from every frontier point of A, and some point of X is outside A, then every point of X is ≥ c from A. |
| ForgeCam/Hypograph.lean | **`sd_hyp_mono`, the lemma the 2.5D checker rests on**: for a body {z ∈ Z, ρ ≤ w(z)}, the signed distance is non-decreasing in ρ at fixed z. `sd_hyp_eq_sdRZ`: it depends only on (ρ, z). `sdRZ_le_sd`: a lower bound D on ρ gives sd ≥ sdRZ(D, z). Z and w are arbitrary. |
| ForgeCam/Spec.lean | The definitions of G1 (with ε_in), G2 (corrected, §7b: T ∪ N misses F ⊕ B_c on every piece; N misses int S) and G3. Declared removals: `target P Δ = P ∖ Δ`, `G1_target_iff`. `G4_held`: the leftover split exactly into "held by fixture" (inside the clamped region K) and "uncut". G4: `G4_lower` (P's (δ − ε)-thickening survives, from G1), `G4_part`, `G4_upper` (the leftover L = S_end ∖ cthickening covers the excess, is disjoint from the grown part, and is the least such set) and `leftover_slice` (per z band). |
| ForgeCam/Prism.lean | **The closed-form checker's soundness**: prism faces {σ ∈ S, h ∈ H}; planar or vertical moves (tip at (γ(s), ζ(s)), ζ(s) ∈ Zt; γ any curve). `clearFace_sound` covers one face. `Certified` packages what the checker establishes for one body against one closed set: its top is above the set, and each face lies within μ of a lowered face answered with lower ≥ c + μ. `certified_clear` turns that into clearance, and `hyp_preconnected` shows the tool bodies are connected. `G1_of_closedForm`, `G2_of_closedForm` (the corrected G2, T and N against the fixture at c' > c) and `G3_of_closedForm` give the obligations. |
| ForgeCam/Vise.lean | The vise as fixture.rs builds it (3e8a91d), a union of boxes. `frontier_box_subset` and `frontier_fixture_faces`: its frontier lies in its boxes' six prism faces each. `certified_vise`: the per-face verdicts make it `Certified`. `G2open_of_closedForm`: certified at exactly the clearance c, as forge-cam does it, the whole tool misses the open c-neighbourhood of the fixture; the closed F ⊕ B_c needs c' > c (`G2_of_closedForm`). `held_band`, `uncut_band`, `band_split`: G4's bands against the jaws' top. |
| ForgeCam/Fixture.lean | Soft jaws: `jaw = blank ∖ ((H ⊕ B_k) ∩ slab)`. `jaw_seat` proves the held body misses the jaw, for any k, provided H meets the blank only inside the grip slab. `jaw_seat_needs_slab` shows that proviso is needed. |
| ForgeCam/Pocket.lean | **Pocket coverage** (`passes_cover`): offset passes at depths r + k·s from the original boundary, with 0 < s ≤ r, put every point of the opening R ∘ B_r within r of a pass. This is the void stock.rs assumes. |
| ForgeCam/Dist2.lean | **slab::dist's segment–segment candidates, exact** (over any ordered field, so they run over ℚ). `ptSegSq_le` / `ptSegSq_attained`: the end-or-perpendicular candidate is the least squared distance from a point to a segment. `segSegLo_le`: the low end (the four end-to-segment candidates if apart, else 0) is a lower bound for every pair of points. `segSegLo_attained`: when the segments are apart, it is attained. `clearFace_segSeg`: for a segment move against a wall over a segment, hypothesis (i) of `ClearFace` is proven, not assumed. |
| ForgeCam/Machine.lean | G5 for 3-axis. `G5_line_iff`: a line move is within travel iff its ends are. `G5_arc`: an arc or helix is within travel if its circle's box is. `lineOkQ_iff`: the ℚ Bool check equals the ℝ spec. |
| ForgeCam/Post.lean | G6 for forge-post's motion subset. The printer is modal: G only on a mode change, an axis only when it changes, F only when the feed changes, and a null move is dropped. **`round_trip`**: `read (print p) = normalize (motions p)`. `G6` adds quantisation, with error ≤ ½·10^−d (`roundEven_isQuant`, ties to even). **`faulty_counterexample`**: the printer as of 155e2f2 reads a plunge back as a rapid. |

## What the checker's verdict means here

For one face, slab.rs answers `Clear { lower }` when two things hold:
- 0 ≤ D ≤ |σ − γ(s)| for every instant and section point, where D is the low end of
  `slab::dist`'s interval;
- lower ≤ sdRZ(D, h − z_t) for every height h of the face, which the Δ branch and bound gives.

`ClearFace` is exactly these two facts. Only D's low end matters, because of the monotonicity.

## Hypotheses, stated

- **The tool's body** is `hyp Z w` with Z an interval and w ≥ 0 on Z. tool.rs's rising side gives
  this.
- **The frontier** of the part, stock or fixture lies within μ of the checked (lowered) faces,
  and each verdict holds with lower ≥ c + μ (design.md §5.2's mu).
- **`TopAbove`**: the tool has an axis point whose height lies above the set, at every instant.
  Without it, a body wholly inside the set touches no face, and the face checks see nothing.
  ops::certify checks it on every move (`containment`, 01f0a30).
- **G2 and G3** are checked against the stock S₀ as it stood before the piece (S(s) ⊆ S₀),
  which is sound.

## Not covered yet

- `slab::dist` for pairs with an arc (segment–arc, arc–arc), and for flat regions
  (`dist_region`): these are still hypothesis (i) above. Segment–segment is proven (Dist2.lean).
- The interval arithmetic (L5) and the B&B checker for non-prism faces.
- The character level of G-code text: words to digits, and the dialect's templates.
- The Aeneas translation of forge-cam-geom and forge-post (L4).
