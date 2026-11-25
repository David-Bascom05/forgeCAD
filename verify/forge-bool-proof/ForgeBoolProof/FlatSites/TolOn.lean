import ForgeCurved.BodyModel

/-!
# A tolerance `On` at trim_in's stretch sampler, correct under δ-Faithful

old-path-statements.md §1a, ledger 1a.classify (fact 2). `point_in_face`'s nearest-edge rule
answers `On` for a point within LINEAR of an edge. At trim_in's stretch sampler (boolean.rs
`inside`'s `at`) it does so on patterns seq for 36 samples that are exactly off the edge: the
stretch runs the whole length of a line edge of face k, on the meet line of face k's plane and the
other face's, and the three planes (face k, the face across the edge, the other face) share no line
(31: the meet line crosses the plane across 1.4 to 650 mm away; 5: parallel, offset). Read exactly,
the answer there is `Inside`/`Outside`, and the exact result would keep a sliver ~1e-14 wide that
today's merges away.

The statement (§E `Correct`) does not ask for the exact reading. It asks that the result be a
`ValidSolid` and `Faithful δ`: each stored edge within `δ` of its exact meet (`WithinH δ e.curve
(edgeMeet δ b i e)`). This file proves the edge half for a straight stored edge:
* `near_segment`: a segment whose two ends are within `η` of an affine line (or any affine subspace)
  lies wholly within `η` of it;
* `edge_within_of_ends`: if every face bounding edge `i` has its carrier zero on an affine subspace
  `L` (for two planes: their meet line), the stored edge is the segment `a₀a₁`, and both ends are
  within `η ≤ δ` of `L`, then the edge is within `δ` of its exact meet.

**Measured** (FLAT-SITES probe, proof/flat-sites-probes probe36.patch, after each of the 18
booleans of patterns seq where the 36 samples fire): every result passes `forge_topo::check`; every
edge between two planar faces lies within 2.7e-13 of the exact meet line of their planes (`meet3`
in dyadics), every vertex within 2.6e-13 of the exact meet of three of its planes; δ = LINEAR =
1e-5. So at these 36 sites `edge_within_of_ends` applies with η ≤ 2.7e-13: the tolerance `On` is
correct under δ-Faithful there, and the exact reading differs by ≤ 1e-13. The rest of `Correct`
for these results (the region half of Faithful, `denote` equality) rests on the general proof.
-/

namespace ForgeBoolProof.FlatSites.TolOn

open Set ForgeCurved.BodyModel

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- A segment whose ends are within `η` of an affine subspace lies within `η` of it. -/
theorem near_segment {L : AffineSubspace ℝ E} {a b a' b' : E} {η : ℝ}
    (ha' : a' ∈ L) (hb' : b' ∈ L) (hda : dist a a' ≤ η) (hdb : dist b b' ≤ η)
    {p : E} (hp : p ∈ segment ℝ a b) : ∃ c ∈ L, dist p c ≤ η := by
  rw [segment_eq_image_lineMap] at hp
  obtain ⟨t, ⟨ht0, ht1⟩, rfl⟩ := hp
  refine ⟨AffineMap.lineMap a' b' t, AffineMap.lineMap_mem t ha' hb', ?_⟩
  rw [AffineMap.lineMap_apply_module, AffineMap.lineMap_apply_module, dist_eq_norm]
  have h : ((1 - t) • a + t • b) - ((1 - t) • a' + t • b') = (1 - t) • (a - a') + t • (b - b') := by
    rw [smul_sub, smul_sub]; abel
  rw [h]
  calc ‖(1 - t) • (a - a') + t • (b - b')‖
      ≤ ‖(1 - t) • (a - a')‖ + ‖t • (b - b')‖ := norm_add_le _ _
    _ = (1 - t) * ‖a - a'‖ + t * ‖b - b'‖ := by
        rw [norm_smul, norm_smul, Real.norm_of_nonneg (by linarith), Real.norm_of_nonneg ht0]
    _ ≤ (1 - t) * η + t * η := by
        rw [← dist_eq_norm, ← dist_eq_norm]
        gcongr
    _ = η := by ring

/-- **A straight stored edge near its faces' common meet is within `δ` of its exact meet.** -/
theorem edge_within_of_ends {δ η : ℝ} (hη : η ≤ δ) (b : BodyModel E) (i : Nat) (e : EdgeModel E)
    (L : AffineSubspace ℝ E) (hL : ∀ f ∈ b.faces, IncE f i → ∀ x ∈ L, f.carrier x = 0)
    {a₀ a₁ a₀' a₁' : E} (hseg : e.curve = segment ℝ a₀ a₁) (h₀ : a₀' ∈ L) (h₁ : a₁' ∈ L)
    (hd₀ : dist a₀ a₀' ≤ η) (hd₁ : dist a₁ a₁' ≤ η) :
    WithinH δ e.curve (edgeMeet δ b i e) := by
  constructor
  · intro a ha
    have ha' := ha
    rw [hseg] at ha'
    obtain ⟨c, hcL, hc⟩ := near_segment h₀ h₁ hd₀ hd₁ ha'
    refine ⟨c, ⟨fun f hf hi => hL f hf hi c hcL, a, ha, ?_⟩, hc.trans hη⟩
    rw [dist_comm]; exact hc.trans hη
  · intro c hc
    obtain ⟨_, q, hq, hd⟩ := hc
    exact ⟨q, hq, by rw [dist_comm]; exact hd⟩

end ForgeBoolProof.FlatSites.TolOn
