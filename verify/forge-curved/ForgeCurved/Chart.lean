import Mathlib.Analysis.Calculus.FDeriv.Basic
import Mathlib.Analysis.Calculus.FDeriv.Linear
import Mathlib.Analysis.Convex.Basic
import Mathlib.Topology.Connected.Basic
import Mathlib.Topology.Order.IntermediateValue
import Mathlib.Topology.Algebra.Order.Field

/-!
# The local chart at a regular point: two connected sides in arbitrarily small boxes

At a zero `x` of `c` where `c` has a strict derivative `L ≠ 0` (a facet point on a regular
quadric), pick `v` with `L v = 1`. The **box** of half-height `δ` and radius `ε` round `x` is
`{y : |L (y - x)| < δ, ‖(y - x) - L (y - x) • v‖ < ε}`, a product of an interval along `v` and a
disc in `ker L`. For `ε ≪ δ` and the box inside the ball where `c` is nearly linear:
* `c` increases along `v` inside the box, at slope at least `1/2`;
* the top of the box (`L (y - x) > δ/2`) lies in `{c > 0}`.

So every point with `c > 0` rises along `v` to the top inside `{c > 0}`. **`box_pos`**: the
positive half is preconnected and has `x` in its closure. **`box_halves`**: in every neighbourhood
of `x` there is an open box whose two halves are both preconnected with `x` in their closures.

This is the geometry under F5's `sides` and `local_` (FaceLocal.lean `SideLocal`), discharged
in FacetSides.lean.
-/

namespace ForgeCurved.Chart

open Set Filter Topology

variable {E : Type*} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- The near-linearity of a strictly differentiable function, as a two-point bound on a ball. -/
theorem strict_bound {c : E → ℝ} {L : E →L[ℝ] ℝ} {x : E} (h : HasStrictFDerivAt c L x) {η : ℝ}
    (hη : 0 < η) :
    ∃ ρ > 0, ∀ y y', dist y x < ρ → dist y' x < ρ → |c y - c y' - L (y - y')| ≤ η * ‖y - y'‖ := by
  obtain ⟨ρ, hρ, hball⟩ := Metric.eventually_nhds_iff.mp (h.isLittleO.def hη)
  refine ⟨ρ, hρ, fun y y' hy hy' => ?_⟩
  have := @hball (y, y') (by rw [Prod.dist_eq]; exact max_lt hy hy')
  simpa [Real.norm_eq_abs] using this

/-- The box round `x`: an interval along `v` times a disc transverse to it. -/
def box (L : E →L[ℝ] ℝ) (v x : E) (δ ε : ℝ) : Set E :=
  {y | |L (y - x)| < δ ∧ ‖(y - x) - L (y - x) • v‖ < ε}

variable {L : E →L[ℝ] ℝ} {v x : E} {δ ε : ℝ}

theorem box_open : IsOpen (box L v x δ ε) := by
  show IsOpen ({y : E | |L (y - x)| < δ} ∩ {y | ‖(y - x) - L (y - x) • v‖ < ε})
  exact (isOpen_lt (by fun_prop) continuous_const).inter (isOpen_lt (by fun_prop) continuous_const)

theorem norm_lt_of_box {y : E} (hy : y ∈ box L v x δ ε) : ‖y - x‖ < δ * ‖v‖ + ε := by
  have hsplit : y - x = L (y - x) • v + ((y - x) - L (y - x) • v) := by abel
  calc ‖y - x‖ ≤ ‖L (y - x) • v‖ + ‖(y - x) - L (y - x) • v‖ := by
        conv_lhs => rw [hsplit]
        exact norm_add_le _ _
    _ = |L (y - x)| * ‖v‖ + ‖(y - x) - L (y - x) • v‖ := by
        rw [norm_smul, Real.norm_eq_abs]
    _ < δ * ‖v‖ + ε := by
        have := mul_le_mul_of_nonneg_right hy.1.le (norm_nonneg v)
        linarith [hy.2]

/-- Moving along `v` changes only the `v`-coordinate. -/
theorem box_shift (hLv : L v = 1) {y : E} (t : ℝ) :
    L (y + t • v - x) = L (y - x) + t ∧
      (y + t • v - x) - L (y + t • v - x) • v = (y - x) - L (y - x) • v := by
  have h1 : L (y + t • v - x) = L (y - x) + t := by
    rw [show y + t • v - x = (y - x) + t • v by abel, map_add, map_smul, hLv, smul_eq_mul, mul_one]
  refine ⟨h1, ?_⟩
  rw [h1, add_smul]
  abel

theorem box_convex : Convex ℝ (box L v x δ ε) := by
  set Pm : E →L[ℝ] E := ContinuousLinearMap.id ℝ E - L.smulRight v
  have hP : ∀ y, Pm y = y - L y • v := fun y => by simp [Pm]
  have h : box L v x δ ε =
      (L.toLinearMap ⁻¹' Ioo (L x - δ) (L x + δ)) ∩ (Pm.toLinearMap ⁻¹' Metric.ball (Pm x) ε) := by
    ext y
    simp only [box, mem_ofPred_eq, mem_inter_iff, mem_preimage, ContinuousLinearMap.coe_coe,
      mem_Ioo, Metric.mem_ball, dist_eq_norm, hP, map_sub, abs_lt, sub_smul]
    constructor
    · rintro ⟨⟨h1, h2⟩, h3⟩
      refine ⟨⟨by linarith, by linarith⟩, ?_⟩
      have : y - L y • v - (x - L x • v) = y - x - (L y • v - L x • v) := by abel
      rwa [this]
    · rintro ⟨⟨h1, h2⟩, h3⟩
      refine ⟨⟨by linarith, by linarith⟩, ?_⟩
      have : y - L y • v - (x - L x • v) = y - x - (L y • v - L x • v) := by abel
      rwa [← this]
  rw [h]
  exact ((convex_Ioo _ _).linear_preimage L.toLinearMap).inter
    ((convex_ball _ _).linear_preimage Pm.toLinearMap)

/-- **The positive half of a small box is connected and reaches `x`.** -/
theorem box_pos {c : E → ℝ} {ρ η : ℝ} (hc0 : c x = 0) (hLv : L v = 1) (hδ : 0 < δ) (hε : 0 < ε)
    (hη : 0 ≤ η) (hηv : η * ‖v‖ ≤ 1 / 2) (hηε : η * ε < δ / 4) (hsize : δ * ‖v‖ + ε ≤ ρ)
    (hb : ∀ y y', dist y x < ρ → dist y' x < ρ → |c y - c y' - L (y - y')| ≤ η * ‖y - y'‖) :
    IsPreconnected (box L v x δ ε ∩ {y | 0 < c y}) ∧ x ∈ closure (box L v x δ ε ∩ {y | 0 < c y}) := by
  set N := box L v x δ ε
  have hin : ∀ y ∈ N, dist y x < ρ := fun y hy => by
    rw [dist_eq_norm]; exact lt_of_lt_of_le (norm_lt_of_box hy) hsize
  have hxN : x ∈ N := ⟨by simpa using hδ, by simpa using hε⟩
  have hρ : dist x x < ρ := hin x hxN
  -- `c` rises along `v` in the box
  have hrise : ∀ y, ∀ t t' : ℝ, t' ≤ t → y + t • v ∈ N → y + t' • v ∈ N →
      c (y + t' • v) ≤ c (y + t • v) := by
    intro y t t' htt h1 h2
    have := hb _ _ (hin _ h1) (hin _ h2)
    have hd : y + t • v - (y + t' • v) = (t - t') • v := by rw [sub_smul]; abel
    rw [hd, map_smul, hLv, smul_eq_mul, mul_one, norm_smul, Real.norm_eq_abs,
      abs_of_nonneg (sub_nonneg.mpr htt)] at this
    have h3 := (abs_le.mp this).1
    nlinarith
  -- the top of the box is positive
  have htop : ∀ y ∈ N, δ / 2 < L (y - x) → 0 < c y := by
    intro y hy hs
    have := hb y x (hin y hy) hρ
    rw [hc0, sub_zero] at this
    have hn : ‖y - x‖ ≤ |L (y - x)| * ‖v‖ + ‖(y - x) - L (y - x) • v‖ := by
      have hsplit : y - x = L (y - x) • v + ((y - x) - L (y - x) • v) := by abel
      calc ‖y - x‖ ≤ ‖L (y - x) • v‖ + ‖(y - x) - L (y - x) • v‖ := by
            conv_lhs => rw [hsplit]
            exact norm_add_le _ _
        _ = _ := by rw [norm_smul, Real.norm_eq_abs]
    rw [abs_of_pos (by linarith : 0 < L (y - x))] at hn
    have h1 := (abs_le.mp this).1
    have h2 : η * ‖y - x‖ ≤ η * (L (y - x) * ‖v‖ + ε) :=
      mul_le_mul_of_nonneg_left (hn.trans (by linarith [hy.2])) hη
    nlinarith [hy.2]
  set T := {y ∈ N | δ / 2 < L (y - x)}
  have hTpos : T ⊆ N ∩ {y | 0 < c y} := fun y hy => ⟨hy.1, htop y hy.1 hy.2⟩
  have hTconv : Convex ℝ T := by
    show Convex ℝ (N ∩ {y | δ / 2 < L (y - x)})
    refine box_convex.inter ?_
    have : {y : E | δ / 2 < L (y - x)} = {y | δ / 2 + L x < L y} := by
      ext y; simp only [mem_ofPred_eq, map_sub]; constructor <;> intro h <;> linarith
    rw [this]; exact convex_halfSpace_gt L.isLinear _
  set p := x + (3 * δ / 4) • v
  have hp : p ∈ T := by
    have h := box_shift (x := x) (y := x) hLv (3 * δ / 4)
    rw [sub_self, map_zero, zero_add] at h
    refine ⟨⟨?_, ?_⟩, ?_⟩
    · show |L (p - x)| < δ
      rw [h.1, abs_of_pos (by positivity)]; linarith
    · show ‖p - x - L (p - x) • v‖ < ε
      rw [h.2]; simpa using hε
    · show δ / 2 < L (p - x)
      rw [h.1]; linarith
  -- each positive point rises to the top inside the positive half
  have hrise' : ∀ y ∈ N ∩ {y | 0 < c y}, ∃ S : Set E, IsPreconnected S ∧ y ∈ S ∧
      S ⊆ N ∩ {y | 0 < c y} ∧ (S ∩ T).Nonempty := by
    rintro y ⟨hyN, hyc⟩
    set s := L (y - x)
    set a := max 0 (3 * δ / 4 - s)
    have hseg : ∀ t ∈ Icc 0 a, y + t • v ∈ N := by
      intro t ht
      have h := box_shift (x := x) (y := y) hLv t
      refine ⟨?_, ?_⟩
      · rw [h.1, abs_lt]
        have := abs_lt.mp hyN.1
        constructor
        · linarith [ht.1]
        · have : t ≤ 3 * δ / 4 - s ∨ t ≤ 0 := by
            rcases le_total 0 (3 * δ / 4 - s) with h' | h'
            · left; have := ht.2; simpa only [a, max_eq_right h'] using this
            · right; have := ht.2; simpa only [a, max_eq_left h'] using this
          rcases this with h' | h' <;> linarith
      · rw [h.2]; exact hyN.2
    refine ⟨(fun t => y + t • v) '' Icc 0 a, isPreconnected_Icc.image _ (by fun_prop),
      ⟨0, ⟨le_rfl, le_max_left _ _⟩, by simp⟩, ?_, ⟨y + a • v, ⟨a, ⟨le_max_left _ _, le_rfl⟩, rfl⟩,
        hseg a ⟨le_max_left _ _, le_rfl⟩, ?_⟩⟩
    · rintro _ ⟨t, ht, rfl⟩
      refine ⟨hseg t ht, ?_⟩
      have := hrise y t 0 ht.1 (hseg t ht) (by simpa using hyN)
      simp only [zero_smul, add_zero] at this
      exact lt_of_lt_of_le (show 0 < c y from hyc) this
    · show δ / 2 < L (y + a • v - x)
      rw [(box_shift (x := x) (y := y) hLv a).1]
      have := le_max_right 0 (3 * δ / 4 - s)
      linarith
  refine ⟨?_, ?_⟩
  · choose S hS hyS hSsub hST using hrise'
    have heq : N ∩ {y | 0 < c y} = ⋃₀ ((fun y : ↥(N ∩ {y | 0 < c y}) => S y y.2 ∪ T) '' univ) := by
      ext y
      simp only [mem_sUnion, mem_image, mem_univ, true_and]
      constructor
      · intro hy; exact ⟨_, ⟨⟨y, hy⟩, rfl⟩, Or.inl (hyS y hy)⟩
      · rintro ⟨_, ⟨⟨z, hz⟩, rfl⟩, h | h⟩
        · exact hSsub z hz h
        · exact hTpos h
    rw [heq]
    refine isPreconnected_sUnion p _ ?_ ?_
    · rintro _ ⟨⟨z, hz⟩, -, rfl⟩; exact Or.inr hp
    · rintro _ ⟨⟨z, hz⟩, -, rfl⟩
      obtain ⟨w, hwS, hwT⟩ := hST z hz
      exact IsPreconnected.union w hwS hwT (hS z hz) hTconv.isPreconnected
  · -- `x + t v` for small `t > 0`
    have hev : ∀ᶠ t in 𝓝[>] (0 : ℝ), x + t • v ∈ N ∩ {y | 0 < c y} := by
      have hlt : ∀ᶠ t in 𝓝[>] (0 : ℝ), t < δ / 2 := nhdsWithin_le_nhds (gt_mem_nhds (by linarith))
      filter_upwards [hlt, self_mem_nhdsWithin] with t ht ht0
      have h := box_shift (x := x) (y := x) hLv t
      rw [sub_self, map_zero, zero_add] at h
      have hN : x + t • v ∈ N := ⟨by rw [h.1, abs_of_pos (show (0 : ℝ) < t from ht0)]; linarith, by rw [h.2]; simpa using hε⟩
      refine ⟨hN, ?_⟩
      -- the slope is at least 1/2
      have hb' := hb _ _ (hin _ hN) hρ
      rw [hc0, sub_zero, add_sub_cancel_left, map_smul, hLv, smul_eq_mul, mul_one, norm_smul,
        Real.norm_eq_abs, abs_of_pos (show (0 : ℝ) < t from ht0)] at hb'
      have h1 := (abs_le.mp hb').1
      have ht0' : (0 : ℝ) < t := ht0
      have h2 : η * (t * ‖v‖) ≤ t / 2 := by
        have := mul_le_mul_of_nonneg_left hηv ht0'.le
        nlinarith
      show 0 < c (x + t • v)
      linarith
    have ht : Tendsto (fun t : ℝ => x + t • v) (𝓝[>] 0) (𝓝 x) := by
      have : Tendsto (fun t : ℝ => x + t • v) (𝓝 0) (𝓝 (x + (0 : ℝ) • v)) :=
        ((continuous_const.add (continuous_id.smul continuous_const)).tendsto 0)
      simpa using this.mono_left nhdsWithin_le_nhds
    exact mem_closure_of_tendsto ht hev

/-- **Two connected sides in arbitrarily small boxes.** At a zero of `c` with strict derivative
`L ≠ 0`, every neighbourhood holds an open box round `x` whose positive and negative halves are
preconnected and both have `x` in their closure. -/
theorem box_halves {c : E → ℝ} {L : E →L[ℝ] ℝ} {x : E} (hc : HasStrictFDerivAt c L x) (hL : L ≠ 0)
    (hc0 : c x = 0) {W : Set E} (hW : W ∈ 𝓝 x) :
    ∃ N : Set E, IsOpen N ∧ x ∈ N ∧ N ⊆ W ∧
      IsPreconnected (N ∩ {y | 0 < c y}) ∧ x ∈ closure (N ∩ {y | 0 < c y}) ∧
      IsPreconnected (N ∩ {y | c y < 0}) ∧ x ∈ closure (N ∩ {y | c y < 0}) := by
  obtain ⟨u, hu⟩ : ∃ u, L u ≠ 0 := by
    by_contra h; push Not at h; exact hL (ContinuousLinearMap.ext h)
  set v := (L u)⁻¹ • u
  have hLv : L v = 1 := by simp [v, hu]
  set η := 1 / (2 * (‖v‖ + 1))
  have hη : 0 < η := by positivity
  obtain ⟨ρ₁, hρ₁, hb⟩ := strict_bound hc hη
  obtain ⟨ρW, hρW, hball⟩ := Metric.mem_nhds_iff.mp hW
  set ρ := min ρ₁ ρW
  have hρ : 0 < ρ := lt_min hρ₁ hρW
  set δ := ρ / (2 * (‖v‖ + 1))
  have hδ : 0 < δ := by positivity
  set ε := δ / 4
  have hε : 0 < ε := by positivity
  have hv1 : 0 < ‖v‖ + 1 := by positivity
  have hsize : δ * ‖v‖ + ε ≤ ρ / 2 := by
    have : δ * (‖v‖ + 1) = ρ / 2 := by simp only [δ]; field_simp
    have hεδ : ε ≤ δ := by simp only [ε]; linarith
    nlinarith [norm_nonneg v]
  have hηv : η * ‖v‖ ≤ 1 / 2 := by
    simp only [η]; rw [div_mul_eq_mul_div, one_mul, div_le_div_iff₀ (by positivity) (by norm_num)]
    nlinarith [norm_nonneg v]
  have hη2 : η ≤ 1 / 2 := by
    simp only [η]; rw [div_le_div_iff₀ (by positivity) (by norm_num)]; nlinarith [norm_nonneg v]
  have hηε : η * ε < δ / 4 := by
    have : η * ε ≤ ε / 2 := by nlinarith
    simp only [ε] at this ⊢; linarith
  have hsize₁ : δ * ‖v‖ + ε ≤ ρ₁ := by linarith [min_le_left ρ₁ ρW]
  -- the negative half is the positive half of `-c`, in the same box
  have hboxeq : box (-L) (-v) x δ ε = box L v x δ ε := by
    ext y
    simp only [box, mem_ofPred_eq, ContinuousLinearMap.neg_apply, abs_neg, neg_smul_neg]
  have hbneg : ∀ y y', dist y x < ρ₁ → dist y' x < ρ₁ →
      |(-c) y - (-c) y' - (-L) (y - y')| ≤ η * ‖y - y'‖ := by
    intro y y' hy hy'
    have := hb y y' hy hy'
    rw [show (-c) y - (-c) y' - (-L) (y - y') = -(c y - c y' - L (y - y')) by simp; ring, abs_neg]
    exact this
  obtain ⟨hp1, hp2⟩ := box_pos (L := L) (v := v) hc0 hLv hδ hε hη.le hηv hηε hsize₁ hb
  obtain ⟨hn1, hn2⟩ := box_pos (L := -L) (v := -v) (c := -c) (by simp [hc0]) (by simp [hLv]) hδ hε
    hη.le (by rwa [norm_neg]) hηε (by rwa [norm_neg]) hbneg
  rw [hboxeq] at hn1 hn2
  have hneg : {y | 0 < (-c) y} = {y | c y < 0} := by ext y; simp
  rw [hneg] at hn1 hn2
  refine ⟨box L v x δ ε, box_open, ⟨by simpa using hδ, by simpa using hε⟩, fun y hy => hball ?_,
    hp1, hp2, hn1, hn2⟩
  rw [Metric.mem_ball, dist_eq_norm]
  have := norm_lt_of_box hy
  linarith [min_le_right ρ₁ ρW]

end ForgeCurved.Chart
