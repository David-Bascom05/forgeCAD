import ForgeCurved.Nesting
import Mathlib.Analysis.Calculus.Deriv.MeanValue

/-!
# (S2) Crossings from a certified cover of the parameter range

curved-builder-lean.md S2: along a trim or a sample conic `γ`, the builder partitions the parameter
range `[p 0, p n]` into pieces. Each piece is certified, by a float screen with a proven
enclosure, as one of:
* **constant** (`ConstPiece`): `f = c ∘ γ` has no zero on it;
* **one crossing** (`CrossPiece`): `f` has opposite signs at its ends, and its derivative is
  nonzero throughout.

**`crossings_of_cover`**: the zeros of `f` on `[p 0, p n]` are then exactly one per crossing piece.
They are sorted, interior, and transversal (`f' ≠ 0`). These are `Nesting.parity_path`'s crossing
list `ts` and its "no other frontier point" hypothesis, and the transversality that
`flip_of_transversal` needs. A piece certifying neither is a typed refusal.
-/

namespace ForgeCurved.Crossings

open Set Filter Topology

/-- No zero on `[u, v]`. -/
def ConstPiece (f : ℝ → ℝ) (u v : ℝ) : Prop := ∀ t ∈ Icc u v, f t ≠ 0

/-- Opposite end signs and a nonzero derivative throughout `[u, v]`. -/
def CrossPiece (f f' : ℝ → ℝ) (u v : ℝ) : Prop :=
  f u * f v < 0 ∧ ∀ t ∈ Icc u v, HasDerivAt f (f' t) t ∧ f' t ≠ 0

/-- **One crossing piece, one transversal zero.** -/
theorem cross_unique {f f' : ℝ → ℝ} {u v : ℝ} (huv : u < v) (h : CrossPiece f f' u v) :
    ∃ r ∈ Ioo u v, f r = 0 ∧ HasDerivAt f (f' r) r ∧ f' r ≠ 0 ∧ ∀ t ∈ Icc u v, f t = 0 → t = r := by
  obtain ⟨hs, hd⟩ := h
  have hc : ContinuousOn f (Icc u v) := fun t ht => (hd t ht).1.continuousAt.continuousWithinAt
  have hu0 : f u ≠ 0 := fun h0 => by rw [h0, zero_mul] at hs; exact lt_irrefl _ hs
  have hv0 : f v ≠ 0 := fun h0 => by rw [h0, mul_zero] at hs; exact lt_irrefl _ hs
  obtain ⟨r, hr, hr0⟩ : ∃ r ∈ Icc u v, f r = 0 := by
    rcases lt_or_gt_of_ne hu0 with hn | hp
    · have : 0 < f v := by
        by_contra hc'; push Not at hc'; nlinarith [mul_nonneg_of_nonpos_of_nonpos hn.le hc']
      exact intermediate_value_Icc huv.le hc ⟨hn.le, this.le⟩
    · have : f v < 0 := by
        by_contra hc'; push Not at hc'; nlinarith [mul_nonneg hp.le hc']
      exact intermediate_value_Icc' huv.le hc ⟨this.le, hp.le⟩
  have hrI : r ∈ Ioo u v := ⟨lt_of_le_of_ne hr.1 fun e => hu0 (e ▸ hr0),
    lt_of_le_of_ne hr.2 fun e => hv0 (e ▸ hr0)⟩
  refine ⟨r, hrI, hr0, (hd r hr).1, (hd r hr).2, fun t ht ht0 => ?_⟩
  by_contra hne
  -- Rolle between `t` and `r`
  have rolle : ∀ a b, a < b → a ∈ Icc u v → b ∈ Icc u v → f a = 0 → f b = 0 → False := by
    intro a b hab ha hb ha0 hb0
    obtain ⟨ξ, hξ, hξe⟩ := exists_hasDerivAt_eq_slope f f' hab
      (hc.mono (Icc_subset_Icc ha.1 hb.2))
      (fun z hz => (hd z ⟨by linarith [ha.1, hz.1], by linarith [hb.2, hz.2]⟩).1)
    rw [ha0, hb0, sub_zero, zero_div] at hξe
    exact (hd ξ ⟨by linarith [ha.1, hξ.1], by linarith [hb.2, hξ.2]⟩).2 hξe
  rcases lt_or_gt_of_ne hne with h1 | h1
  · exact rolle t r h1 ht hr ht0 hr0
  · exact rolle r t h1 hr ht hr0 ht0

/-- **The zeros along a certified cover.** -/
theorem crossings_of_cover {f f' : ℝ → ℝ} (p : ℕ → ℝ) (k : ℕ → Bool) (hf0 : f (p 0) ≠ 0) :
    ∀ n : ℕ, (∀ i < n, p i < p (i + 1)) →
      (∀ i < n, if k i then CrossPiece f f' (p i) (p (i + 1)) else ConstPiece f (p i) (p (i + 1))) →
      ∃ ts : List ℝ, ts.Pairwise (· < ·) ∧ ts.length = ((List.range n).filter k).length ∧
        (∀ t ∈ ts, p 0 < t ∧ t < p n ∧ f t = 0 ∧ HasDerivAt f (f' t) t ∧ f' t ≠ 0) ∧
        ∀ t ∈ Icc (p 0) (p n), f t = 0 → t ∈ ts
  | 0, _, _ => by
    refine ⟨[], by simp, by simp, by simp, fun t ht h0 => ?_⟩
    have : t = p 0 := le_antisymm ht.2 ht.1
    exact absurd (this ▸ h0) hf0
  | n + 1, hp, hk => by
    obtain ⟨ts, hs, hl, hin, hall⟩ := crossings_of_cover p k hf0 n (fun i hi => hp i (by omega))
      (fun i hi => hk i (by omega))
    have hpn : p n < p (n + 1) := hp n (by omega)
    have hpk := hk n (by omega)
    have hp0n : p 0 ≤ p n := by
      clear hall hin hl hs ts hpk hk
      induction n with
      | zero => exact le_rfl
      | succ m ih => exact (ih (fun i hi => hp i (by omega)) (hp m (by omega))).trans (hp m (by omega)).le
    have hrange : (List.range (n + 1)).filter k = (List.range n).filter k ++ (if k n then [n] else []) := by
      rw [List.range_succ, List.filter_append]; congr 1; simp only [List.filter]; cases k n <;> rfl
    cases hkn : k n
    · -- a constant piece: no new zero
      simp only [hkn, Bool.false_eq_true, ↓reduceIte] at hpk
      refine ⟨ts, hs, by rw [hrange, List.length_append, ← hl, hkn]; simp, fun t ht => ?_, fun t ht h0 => ?_⟩
      · obtain ⟨a, b, c, d, e⟩ := hin t ht; exact ⟨a, by linarith, c, d, e⟩
      · by_cases htn : t ≤ p n
        · exact hall t ⟨ht.1, htn⟩ h0
        · exact absurd h0 (hpk t ⟨(not_le.mp htn).le, ht.2⟩)
    · -- a crossing piece: one new zero, after all the others
      simp only [hkn, ↓reduceIte] at hpk
      obtain ⟨r, hr, hr0, hrd, hrne, huniq⟩ := cross_unique hpn hpk
      refine ⟨ts ++ [r], ?_, by rw [hrange, List.length_append, List.length_append, ← hl, hkn]; simp, fun t ht => ?_, fun t ht h0 => ?_⟩
      · rw [List.pairwise_append]
        refine ⟨hs, by simp, fun a ha b hb => ?_⟩
        rw [List.mem_singleton] at hb; subst hb
        linarith [(hin a ha).2.1, hr.1]
      · rcases List.mem_append.mp ht with h | h
        · obtain ⟨a, b, c, d, e⟩ := hin t h; exact ⟨a, by linarith, c, d, e⟩
        · rw [List.mem_singleton] at h; subst h
          exact ⟨by linarith [hr.1], hr.2, hr0, hrd, hrne⟩
      · by_cases htn : t ≤ p n
        · exact List.mem_append_left _ (hall t ⟨ht.1, htn⟩ h0)
        · exact List.mem_append_right _ (by
            rw [List.mem_singleton]; exact huniq t ⟨(not_le.mp htn).le, ht.2⟩ h0)

/-- **Nesting from a certified cover.** Along `γ` from an anchor in the region `R`, if `R`'s
frontier meets the path only on the trim carrier `c`, and near each such point `R` is one side of
`c` (the trim's orientation), then the end is in `R` iff the number of crossing pieces is even. -/
theorem nesting_of_cover {X : Type} [TopologicalSpace X] {R : Set X} {γ : ℝ → X} {c : X → ℝ}
    {f' : ℝ → ℝ} (p : ℕ → ℝ) (k : ℕ → Bool) (n : ℕ) (hp : ∀ i < n, p i < p (i + 1))
    (hab : p 0 ≤ p n) (hγ : ContinuousOn γ (Icc (p 0) (p n)))
    (hf0 : c (γ (p 0)) ≠ 0)
    (hk : ∀ i < n, if k i then CrossPiece (fun s => c (γ s)) f' (p i) (p (i + 1))
      else ConstPiece (fun s => c (γ s)) (p i) (p (i + 1)))
    (hfr : ∀ t ∈ Icc (p 0) (p n), γ t ∈ frontier R → c (γ t) = 0)
    (hside : ∀ t ∈ Icc (p 0) (p n), c (γ t) = 0 → ∃ pos : Bool, ∃ W ∈ 𝓝 (γ t),
      ∀ z ∈ W, (z ∈ R ↔ if pos then 0 < c z else c z < 0))
    (ha : γ (p 0) ∈ R) :
    (γ (p n) ∈ R ↔ Even ((List.range n).filter k).length) := by
  obtain ⟨ts, hs, hl, hin, hall⟩ := crossings_of_cover (f := fun s => c (γ s)) (f' := f') p k hf0 n hp hk
  rw [← hl]
  refine Nesting.anchored ts hab hγ hs (fun t ht => ⟨(hin t ht).1, (hin t ht).2.1⟩)
    (fun t ht hnt hfrt => hnt (hall t ht (hfr t ht hfrt))) (fun t ht => ?_) ha
  obtain ⟨h1, h2, h0, hd, hne⟩ := hin t ht
  have htI : t ∈ Icc (p 0) (p n) := ⟨h1.le, h2.le⟩
  obtain ⟨pos, W, hW, hR⟩ := hside t htI h0
  exact Nesting.flip_of_transversal pos (hγ.continuousAt (Icc_mem_nhds h1 h2)) hW hR hd hne h0

end ForgeCurved.Crossings
