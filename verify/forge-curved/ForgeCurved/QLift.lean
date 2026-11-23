import ForgeCurved.QFacetSampled

/-!
# From the frame back to the input carriers

`samplesOf` works in a frame `T : (x, y, z) ↦ (x + μ(y + λz) + νz, y + λz, z)`. Its carriers are
the input's composed with `T` (`inFrame_ev`). `T` is a linear bijection, so cells, realisation and
bounded sublevel sets carry over.

* `frameT`, `frameInv`, `frameT_inv`: the map and its inverse;
* `cov_frame`: a frame point's covector over the frame carriers is its image's over the inputs;
* `bounded_frame`: a bounded sublevel set stays bounded in the frame;
* **`facet_sampled_input`**: every realised facet of the input carriers on the negative side of a
  bounding carrier is a sample's covector.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

/-- The frame map. -/
def frameT (f : Frame) (p : R3) : R3 :=
  (p.1 + (f.1 : ℝ) * (p.2.1 + (f.2.2 : ℝ) * p.2.2) + (f.2.1 : ℝ) * p.2.2,
   p.2.1 + (f.2.2 : ℝ) * p.2.2, p.2.2)

/-- Its inverse. -/
def frameInv (f : Frame) (P : R3) : R3 :=
  (P.1 - (f.1 : ℝ) * P.2.1 - (f.2.1 : ℝ) * P.2.2, P.2.1 - (f.2.2 : ℝ) * P.2.2, P.2.2)

theorem frameT_inv (f : Frame) (P : R3) : frameT f (frameInv f P) = P := by
  obtain ⟨X, Y, Z⟩ := P
  simp only [frameT, frameInv, Prod.mk.injEq]; refine ⟨by ring, by ring, trivial⟩

theorem qR_frame (f : Frame) (q : Quad.Quadric) (p : R3) : qR (inFrame f q) p = qR q (frameT f p) := by
  rw [← evK_real, ← evK_real]
  have := inFrame_ev realModel f q p.1 p.2.1 p.2.2
  simp only [realModel_q] at this
  exact this

theorem cov_frame (f : Frame) (qs : List Quad.Quadric) (p : R3) :
    covOfK ((qs.map (inFrame f)).map qR) p = covOfK (qs.map qR) (frameT f p) := by
  simp only [covOfK, List.map_map]
  apply List.map_congr_left
  intro q _
  simp only [Function.comp, qR_frame]

theorem realised_frame (f : Frame) {qs : List Quad.Quadric} {X : Cov}
    (h : RealisedK (qs.map qR) X) : RealisedK ((qs.map (inFrame f)).map qR) X := by
  obtain ⟨p, hp⟩ := h
  refine ⟨frameInv f p, ?_⟩
  show covOfK _ _ = X
  rw [cov_frame, frameT_inv]; exact hp

theorem bounded_frame (f : Frame) {g : R3 → ℝ} (hB : Bornology.IsBounded {z | g z ≤ 0}) :
    Bornology.IsBounded {p | g (frameT f p) ≤ 0} := by
  obtain ⟨C, hC⟩ := (Metric.isBounded_iff_subset_closedBall 0).mp hB
  set a := |(f.1 : ℝ)|
  set b := |(f.2.1 : ℝ)|
  set l := |(f.2.2 : ℝ)|
  set M := max C 0
  set K := 1 + a + b + l
  have ha : 0 ≤ a := abs_nonneg _
  have hb : 0 ≤ b := abs_nonneg _
  have hl : 0 ≤ l := abs_nonneg _
  have hM : 0 ≤ M := le_max_right _ _
  refine (Metric.isBounded_iff_subset_closedBall 0).mpr ⟨K * M, fun p hp => ?_⟩
  have h := hC hp
  simp only [Metric.mem_closedBall, dist_zero_right] at h ⊢
  set P := frameT f p
  have hpP : p = frameInv f P := by
    obtain ⟨x, y, z⟩ := p
    simp only [P, frameT, frameInv, Prod.mk.injEq]; refine ⟨by ring, by ring, trivial⟩
  have hPM : ‖P‖ ≤ M := _root_.le_trans h (le_max_left _ _)
  have hX : |P.1| ≤ M := _root_.le_trans (norm_fst_le P) hPM
  have hY : |P.2.1| ≤ M := _root_.le_trans (_root_.le_trans (norm_fst_le P.2) (norm_snd_le P)) hPM
  have hZ : |P.2.2| ≤ M := _root_.le_trans (_root_.le_trans (norm_snd_le P.2) (norm_snd_le P)) hPM
  have haY : a * |P.2.1| ≤ a * M := mul_le_mul_of_nonneg_left hY ha
  have hbZ : b * |P.2.2| ≤ b * M := mul_le_mul_of_nonneg_left hZ hb
  have hlZ : l * |P.2.2| ≤ l * M := mul_le_mul_of_nonneg_left hZ hl
  have hKM : K * M = M + a * M + b * M + l * M := by simp only [K]; ring
  have hlM : 0 ≤ l * M := mul_nonneg hl hM
  have haM : 0 ≤ a * M := mul_nonneg ha hM
  have hbM : 0 ≤ b * M := mul_nonneg hb hM
  rw [hpP]
  simp only [frameInv, Prod.norm_def, Real.norm_eq_abs, max_le_iff]
  refine ⟨?_, ?_, ?_⟩
  · have e : |P.1 - (f.1 : ℝ) * P.2.1 - (f.2.1 : ℝ) * P.2.2| ≤ |P.1| + a * |P.2.1| + b * |P.2.2| := by
      have := abs_sub (P.1 - (f.1 : ℝ) * P.2.1) ((f.2.1 : ℝ) * P.2.2)
      have h2 := abs_sub P.1 ((f.1 : ℝ) * P.2.1)
      rw [abs_mul] at this h2
      linarith
    linarith
  · have e : |P.2.1 - (f.2.2 : ℝ) * P.2.2| ≤ |P.2.1| + l * |P.2.2| := by
      have := abs_sub P.2.1 ((f.2.2 : ℝ) * P.2.2)
      rw [abs_mul] at this; exact this
    linarith
  · linarith

/-- **Every realised facet of the input carriers is sampled.** -/
theorem facet_sampled_input {qs : List Quad.Quadric} {ss : List QS} (h : samplesOf qs = some ss)
    {bi : ℕ} (hbi : bi < (qs.map qR).length) (hB : Bornology.IsBounded {z | (qs.map qR)[bi] z ≤ 0})
    (X : Cov) (hXb : X[bi]? = some Sgn.neg) {j : ℕ} (hj : j < (qs.map qR).length)
    (hXj : X[j]? = some Sgn.zero) (hXo : ∀ m, m ≠ j → X[m]? ≠ some Sgn.zero)
    (hX : RealisedK (qs.map qR) X) : ∃ s ∈ ss, s.cov = X := by
  obtain ⟨f, -, hf, hsing, hat⟩ := samplesOf_frame h
  have hbq : bi < qs.length := by simpa using hbi
  have hjq : j < qs.length := by simpa using hj
  refine facet_sampled hf hsing hat (bi := bi) (by simpa using hbq) ?_ X hXb (j := j)
    (by simpa using hjq) hXj hXo (realised_frame f hX)
  have := bounded_frame f (g := qR qs[bi]) (by simpa using hB)
  simpa [qR_frame] using this

end ForgeCurved
