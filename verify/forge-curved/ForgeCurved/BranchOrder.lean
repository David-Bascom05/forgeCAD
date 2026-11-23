import Mathlib.Analysis.Calculus.Deriv.MeanValue
import Mathlib.Topology.Order.IntermediateValue
import Mathlib.Topology.Algebra.Order.Field

/-!
# (L1) Between events, the branches of a plane curve keep their order

Curved Rust's stage (b) matches the branches of `q_j ∩ q_k` across a slab by their y-order on one
sample plane. The curve's y-projection is `{R = 0}` with `R(x, y) = Res_z(q_j, q_k)`. Its events
are the roots of `D(x)`, the end of the PRS of `(R, ∂_y R)`, checked by the identity
`s · R + t · R_y = D`.

* **`nonsing_of_identity`**: off the roots of `D`, every zero of `R` has `R_y ≠ 0`;
* **`local_unique`**: near a zero with `R_y ≠ 0`, each vertical line meets `{R = 0}` at most
  once (mean value theorem);
* **`branch_order`**: two continuous branches over an open interval free of such singular zeros,
  ordered at one `x`, are ordered at every `x`. So a sample plane's sorted order holds across
  the whole slab.

`R` is any function with a continuous partial derivative in `y`: no polynomial structure is used.
-/

namespace ForgeCurved.BranchOrder

open Set Filter Topology

variable {R Ry : ℝ → ℝ → ℝ}

/-- **The PRS identity makes the slab's zeros nonsingular.** -/
theorem nonsing_of_identity {s t : ℝ → ℝ → ℝ} {D : ℝ → ℝ}
    (hid : ∀ x y, s x y * R x y + t x y * Ry x y = D x) {x y : ℝ} (hD : D x ≠ 0)
    (h0 : R x y = 0) : Ry x y ≠ 0 := by
  intro h
  apply hD
  rw [← hid x y, h0, h]; ring

/-- **Local uniqueness at a nonsingular zero.** -/
theorem local_unique (hd : ∀ x y, HasDerivAt (R x) (Ry x y) y)
    (hc : Continuous fun p : ℝ × ℝ => Ry p.1 p.2) {x₁ y₁ : ℝ} (hne : Ry x₁ y₁ ≠ 0) :
    ∃ ρ > 0, ∀ x y y', |x - x₁| < ρ → |y - y₁| < ρ → |y' - y₁| < ρ → R x y = 0 → R x y' = 0 →
      y = y' := by
  have hev : ∀ᶠ p : ℝ × ℝ in 𝓝 (x₁, y₁), Ry p.1 p.2 ≠ 0 :=
    (hc.continuousAt (x := (x₁, y₁))).eventually_ne hne
  obtain ⟨ρ, hρ, hball⟩ := Metric.eventually_nhds_iff.mp hev
  refine ⟨ρ, hρ, fun x y y' hx hy hy' h0 h0' => ?_⟩
  by_contra hyy
  -- the mean value theorem between `y` and `y'`
  have key : ∀ u v : ℝ, u < v → |u - y₁| < ρ → |v - y₁| < ρ → R x u = 0 → R x v = 0 → False := by
    intro u v huv hu hv hu0 hv0
    obtain ⟨ξ, hξ, hξe⟩ := exists_hasDerivAt_eq_slope (R x) (Ry x) huv
      (fun z _ => (hd x z).continuousAt.continuousWithinAt) (fun z _ => hd x z)
    rw [hu0, hv0, sub_zero, zero_div] at hξe
    apply hball (y := (x, ξ)) _ hξe
    rw [Prod.dist_eq, Real.dist_eq, Real.dist_eq]
    refine max_lt hx ?_
    rw [abs_lt] at hu hv ⊢
    constructor <;> linarith [hξ.1, hξ.2]
  rcases lt_or_gt_of_ne hyy with h | h
  · exact key y y' h hy hy' h0 h0'
  · exact key y' y h hy' hy h0' h0

/-- **(L1) The branches keep their order.** Over an open interval where every zero of `R` is
nonsingular, two continuous branches of `{R = 0}` with `f x₀ < g x₀` have `f x < g x`
throughout. -/
theorem branch_order (hd : ∀ x y, HasDerivAt (R x) (Ry x y) y)
    (hc : Continuous fun p : ℝ × ℝ => Ry p.1 p.2) {a b : ℝ}
    (hns : ∀ x ∈ Ioo a b, ∀ y, R x y = 0 → Ry x y ≠ 0) {f g : ℝ → ℝ}
    (hf : ContinuousOn f (Ioo a b)) (hg : ContinuousOn g (Ioo a b))
    (hf0 : ∀ x ∈ Ioo a b, R x (f x) = 0) (hg0 : ∀ x ∈ Ioo a b, R x (g x) = 0)
    {x₀ : ℝ} (hx₀ : x₀ ∈ Ioo a b) (hlt : f x₀ < g x₀) : ∀ x ∈ Ioo a b, f x < g x := by
  set U := Ioo a b ∩ {x | f x < g x}
  set V := Ioo a b ∩ {x | ¬ f x < g x}
  have hfx : ∀ x ∈ Ioo a b, ContinuousAt f x := fun x hx => hf.continuousAt (isOpen_Ioo.mem_nhds hx)
  have hgx : ∀ x ∈ Ioo a b, ContinuousAt g x := fun x hx => hg.continuousAt (isOpen_Ioo.mem_nhds hx)
  have hU : IsOpen U := by
    rw [isOpen_iff_mem_nhds]
    rintro x ⟨hx, hxl⟩
    exact inter_mem (isOpen_Ioo.mem_nhds hx)
      (((hfx x hx).sub (hgx x hx)).eventually (gt_mem_nhds (by simpa using sub_neg.mpr hxl)) |>.mono
        fun z hz => by simpa using sub_neg.mp hz)
  have hV : IsOpen V := by
    rw [isOpen_iff_mem_nhds]
    rintro x ⟨hx, hxl⟩
    rcases lt_or_eq_of_le (not_lt.mp hxl) with h | h
    · exact inter_mem (isOpen_Ioo.mem_nhds hx)
        (((hfx x hx).sub (hgx x hx)).eventually (lt_mem_nhds (sub_pos.mpr h)) |>.mono
          fun z hz => by simp only [not_lt]; exact (sub_pos.mp hz).le)
    · -- equal at `x`: equal nearby, by local uniqueness
      obtain ⟨ρ, hρ, huniq⟩ := local_unique hd hc (hns x hx (g x) (hg0 x hx))
      have e1 : ∀ᶠ z in 𝓝 x, |f z - g x| < ρ := by
        have := (hfx x hx).eventually (Metric.ball_mem_nhds (f x) hρ)
        exact this.mono fun z hz => by rw [h]; simpa [Real.dist_eq] using hz
      have e2 : ∀ᶠ z in 𝓝 x, |g z - g x| < ρ :=
        ((hgx x hx).eventually (Metric.ball_mem_nhds (g x) hρ)).mono fun z hz => by
          simpa [Real.dist_eq] using hz
      have e3 : ∀ᶠ z in 𝓝 x, |z - x| < ρ :=
        Filter.mem_of_superset (Metric.ball_mem_nhds x hρ) fun z hz => by
          simpa [Real.dist_eq] using hz
      have e4 : ∀ᶠ z in 𝓝 x, z ∈ Ioo a b := isOpen_Ioo.mem_nhds hx
      filter_upwards [e1, e2, e3, e4] with z h1 h2 h3 h4
      refine ⟨h4, ?_⟩
      have := huniq z (f z) (g z) h3 h1 h2 (hf0 z h4) (hg0 z h4)
      simp [this]
  have hcov : Ioo a b ⊆ U ∪ V := fun x hx => by
    by_cases h : f x < g x
    · exact Or.inl ⟨hx, h⟩
    · exact Or.inr ⟨hx, h⟩
  have hdisj : Ioo a b ∩ (U ∩ V) = ∅ := by
    ext x; simp only [mem_inter_iff, mem_empty_iff_false, iff_false]
    rintro ⟨-, ⟨-, h1⟩, ⟨-, h2⟩⟩; exact h2 h1
  rcases isPreconnected_iff_subset_of_disjoint.mp isPreconnected_Ioo U V hU hV hcov hdisj with h | h
  · exact fun x hx => (h hx).2
  · exact absurd hlt (h hx₀).2

end ForgeCurved.BranchOrder
