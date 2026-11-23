# verify/forge-cam: CAM's L1

This folder proves the obligations G1–G6 of research/cam/design.md §5.2 in Lean 4.36.0 with Mathlib
v4.36.0, over ℝ. The executable parts run over ℚ or ℤ. There is no `sorry`, no `native_decide`,
and only the standard axioms (`lake env lean Axioms.lean`). CI: .github/workflows/forge-cam.yml.

```
lake exe cache get && lake build && lake env lean Axioms.lean
```

| file | what it proves |
|---|---|
| ForgeCam/SignedDist.lean | `sd`, the signed distance; `sd_translate`. **`clear_of_frontier`**: take a connected body X and a closed set A. If X is at signed distance ≥ c > 0 from every frontier point of A, and some point of X is outside A, then every point of X is ≥ c from A. |
| ForgeCam/Hypograph.lean | **`sd_hyp_mono`, the lemma the 2.5D checker rests on**: for a body {z ∈ Z, ρ ≤ w(z)}, the signed distance is non-decreasing in ρ at fixed z. `sd_hyp_eq_sdRZ`: it depends only on (ρ, z). `sdRZ_le_sd`: a lower bound D on ρ gives sd ≥ sdRZ(D, z). Z and w are arbitrary. |
| ForgeCam/Spec.lean | The definitions of G1 (with ε_in), G2 and G3. G4: `G4_lower` (P's (δ − ε)-thickening survives, from G1), `G4_part`, `G4_upper` (the leftover L = S_end ∖ cthickening covers the excess, is disjoint from the grown part, and is the least such set) and `leftover_slice` (per z band). |
| ForgeCam/Prism.lean | **The closed-form checker's soundness**: prism faces {σ ∈ S, h ∈ H} and planar moves (tip at (γ(s), z_t), with γ any curve). `clearFace_sound` covers one face. `hyp_preconnected` shows the tool bodies are connected. `G1_of_closedForm`, `G2_of_closedForm` and `G3_of_closedForm` give the obligations. |
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
- **The frontier** of the part, stock or fixture lies in the union of the checked faces.
  Lowering does this: every face is checked.
- **`TopAbove`**: the tool has an axis point whose height lies above the set, at every instant.
  Without it, a body wholly inside the set touches no face, and the face checks see nothing.
  check.rs does not state this; the CAM session is asked to confirm where it holds.
- **G2 and G3** are checked against the stock S₀ as it stood before the piece (S(s) ⊆ S₀),
  which is sound.

## Not covered yet

- The 2D candidate analysis of `slab::dist` (that D's interval holds the true least distance)
  is L2/L3 work. Here it is the hypothesis (i) above.
- The interval arithmetic (L5) and the B&B checker for non-prism faces.
- The character level of G-code text: words to digits, and the dialect's templates.
- The Aeneas translation of forge-cam-geom and forge-post (L4).
