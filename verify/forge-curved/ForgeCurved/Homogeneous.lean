import ForgeCurved.Transversal

/-!
# Conical points: the local picture from the tangent cone

At a cone's apex the carriers through it are not transversal: the cone's gradient is zero there.
They are exactly **sign-homogeneous** about the apex, though: the cone, its axial seams and its
nappe plane keep their sign along every open ray from the apex. Then the cells near the apex are
cones, and a cone meets a small ball in a homeomorphic copy of itself (`y ↦ x + r/(1+‖y-x‖)·(y-x)`
maps the whole space onto the ball, ray by ray).

**`localGood_of_homogeneous`**: if the carriers through `x` are sign-homogeneous about `x` and
every cell of their own arrangement (`coneCell`, cut by them alone) above `x`'s is preconnected,
the local picture of Local.lean holds at `x`. The cone cells are R1 of a fixed small model (a
quadric cone with its seams), proven once.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology Set

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- The carriers through `x` keep their sign along open rays from `x`. -/
def HomogeneousAt (cs : List (Carrier E ℝ)) (x : E) : Prop :=
  ∀ i : Fin cs.length, cs[i] x = 0 → ∀ y : E, ∀ t : ℝ, 0 < t →
    sgnK (cs[i] (x + t • (y - x))) = sgnK (cs[i] y)

/-- The cell of `Y` in the arrangement of the carriers through `x` alone. -/
def coneCell (cs : List (Carrier E ℝ)) (x : E) (Y : Cov) : Set E :=
  {y | ∀ i (h : i < cs.length) (h' : i < Y.length), cs[i] x = 0 → sgnK (cs[i] y) = Y[i]}

theorem cell_sub_coneCell {cs : List (Carrier E ℝ)} {x : E} {Y : Cov} :
    cell cs Y ⊆ coneCell cs x Y := by
  intro y hy i h h' _
  have hy' : covOfK cs y = Y := hy
  have : (covOfK cs y)[i]'(by rw [length_covOfK]; exact h) = Y[i] := by simp only [hy']
  rwa [getElem_covOfK] at this

theorem coneCell_ray {cs : List (Carrier E ℝ)} {x : E} (hh : HomogeneousAt cs x) {Y : Cov}
    {y : E} (hy : y ∈ coneCell cs x Y) {t : ℝ} (ht : 0 < t) : x + t • (y - x) ∈ coneCell cs x Y := by
  intro i h h' hx
  exact (hh ⟨i, h⟩ hx y t ht).trans (hy i h h' hx)

/-- Near `x`, a cell above `x`'s is its cone cell. -/
theorem cell_iff_coneCell {cs : List (Carrier E ℝ)} {x : E} {N : Set E}
    (hN : ∀ y ∈ N, Cov.le (covOfK cs x) (covOfK cs y) = true) {Y : Cov}
    (hY : Cov.le (covOfK cs x) Y = true) {y : E} (hyN : y ∈ N) :
    y ∈ cell cs Y ↔ y ∈ coneCell cs x Y := by
  refine ⟨fun h => cell_sub_coneCell h, fun hq => ?_⟩
  obtain ⟨hlen, hent⟩ := le_iff_getElem.mp hY
  rw [length_covOfK] at hlen
  obtain ⟨-, hxy⟩ := le_iff_getElem.mp (hN y hyN)
  rw [mem_cell]
  apply List.ext_getElem (by rw [length_covOfK, hlen])
  intro j hj hjY
  have hjn : j < cs.length := by rwa [length_covOfK] at hj
  by_cases hz : cs[j] x = 0
  · rw [getElem_covOfK]; exact hq j hjn hjY hz
  · have hxj : (covOfK cs x)[j]'(by rw [length_covOfK]; exact hjn) ≠ .zero := by
      rw [getElem_covOfK]; exact fun h => hz (sgnK_zero_iff.mp h)
    have e1 := (hxy j (by rw [length_covOfK]; exact hjn) hj).resolve_left hxj
    have e2 := (hent j (by rw [length_covOfK]; exact hjn) hjY).resolve_left hxj
    rw [← e1, e2]

/-- The radial map of the whole space onto the ball of radius `r` about `x`. -/
noncomputable def toBall (x : E) (r : ℝ) (y : E) : E := x + (r / (1 + ‖y - x‖)) • (y - x)

theorem continuous_toBall (x : E) (r : ℝ) : Continuous (toBall x r) := by
  have h : ∀ y : E, 1 + ‖y - x‖ ≠ 0 := fun y => by positivity
  unfold toBall
  fun_prop (disch := exact h _)

/-- **A cone cell meets a ball about its apex in an image of itself.** -/
theorem coneCell_inter_ball {cs : List (Carrier E ℝ)} {x : E} (hh : HomogeneousAt cs x) (Y : Cov)
    {r : ℝ} (hr : 0 < r) :
    coneCell cs x Y ∩ Metric.ball x r = toBall x r '' coneCell cs x Y := by
  ext z
  constructor
  · rintro ⟨hzK, hzB⟩
    by_cases hzx : z = x
    · subst hzx; exact ⟨z, hzK, by simp [toBall]⟩
    · have hs : ‖z - x‖ < r := by rwa [Metric.mem_ball, dist_eq_norm] at hzB
      have hs0 : 0 < ‖z - x‖ := norm_pos_iff.mpr (sub_ne_zero.mpr hzx)
      set l := 1 / (r - ‖z - x‖)
      have hl : 0 < l := by unfold l; apply div_pos one_pos; linarith
      refine ⟨x + l • (z - x), coneCell_ray hh hzK hl, ?_⟩
      unfold toBall
      have hn : ‖x + l • (z - x) - x‖ = l * ‖z - x‖ := by
        rw [add_sub_cancel_left, norm_smul, Real.norm_eq_abs, abs_of_pos hl]
      rw [hn, add_sub_cancel_left, smul_smul]
      have : r / (1 + l * ‖z - x‖) * l = 1 := by
        unfold l
        have hne : r - ‖z - x‖ ≠ 0 := by linarith
        field_simp
        ring
      rw [this, one_smul, add_sub_cancel]
  · rintro ⟨y, hyK, rfl⟩
    by_cases hyx : y = x
    · subst hyx
      refine ⟨by simpa [toBall] using hyK, by simp [toBall, hr]⟩
    · have hy0 : 0 < ‖y - x‖ := norm_pos_iff.mpr (sub_ne_zero.mpr hyx)
      have ht : 0 < r / (1 + ‖y - x‖) := by positivity
      refine ⟨coneCell_ray hh hyK ht, ?_⟩
      rw [Metric.mem_ball, dist_eq_norm]
      unfold toBall
      rw [add_sub_cancel_left, norm_smul, Real.norm_eq_abs, abs_of_pos ht]
      rw [div_mul_eq_mul_div, div_lt_iff₀ (by positivity)]
      nlinarith

/-- **The local picture at a conical point.** -/
theorem localGood_of_homogeneous {cs : List (Carrier E ℝ)} (hc : ∀ c ∈ cs, Continuous c) {x : E}
    (hh : HomogeneousAt cs x)
    (hK : ∀ Y, Cov.le (covOfK cs x) Y = true → IsPreconnected (coneCell cs x Y)) :
    LocalGood cs x := by
  obtain ⟨N, hNo, hxN, hN⟩ := exists_open_le hc x
  obtain ⟨r0, hr0, hr0N⟩ := Metric.isOpen_iff.mp hNo x hxN
  constructor
  · intro W hW hxW
    obtain ⟨rW, hrW, hrWW⟩ := Metric.isOpen_iff.mp hW x hxW
    set r := min r0 rW
    have hr : 0 < r := lt_min hr0 hrW
    refine ⟨Metric.ball x r, Metric.isOpen_ball, fun y hy => hrWW (Metric.ball_subset_ball
      (min_le_right _ _) hy), Metric.mem_ball_self hr, fun Y hY => ?_⟩
    have : cell cs Y ∩ Metric.ball x r = coneCell cs x Y ∩ Metric.ball x r := by
      ext y
      constructor
      · rintro ⟨h1, h2⟩; exact ⟨cell_sub_coneCell h1, h2⟩
      · rintro ⟨h1, h2⟩
        exact ⟨(cell_iff_coneCell hN hY (hr0N (Metric.ball_subset_ball (min_le_left _ _) h2))).mpr
          h1, h2⟩
    rw [this, coneCell_inter_ball hh Y hr]
    exact (hK Y hY).image _ (continuous_toBall x r).continuousOn
  · rintro Y hY ⟨w, hw⟩
    rw [Metric.mem_closure_iff]
    intro ε hε
    set m := min ε r0 / 2
    have hm : 0 < m := by unfold m; have := lt_min hε hr0; linarith
    set s := m / (‖w - x‖ + 1)
    have hs : 0 < s := by unfold s; positivity
    have hd : dist x (x + s • (w - x)) < min ε r0 := by
      rw [dist_eq_norm, sub_add_cancel_left, norm_neg, norm_smul, Real.norm_eq_abs, abs_of_pos hs]
      unfold s
      rw [div_mul_eq_mul_div, div_lt_iff₀ (by positivity)]
      have := norm_nonneg (w - x)
      unfold m
      nlinarith [lt_min hε hr0]
    refine ⟨x + s • (w - x), ?_, lt_of_lt_of_le hd (min_le_left _ _)⟩
    have hb : x + s • (w - x) ∈ N := hr0N (by
      rw [Metric.mem_ball, dist_comm]; exact lt_of_lt_of_le hd (min_le_right _ _))
    exact (cell_iff_coneCell hN hY hb).mpr (coneCell_ray hh (cell_sub_coneCell hw) hs)

/-- **A conical point**: off the facets, the carriers through it sign-homogeneous about it, and
their own cells above its covector preconnected. -/
structure ConicalAt (cs : List (Carrier E ℝ)) (x : E) : Prop where
  notFacet : (covOfK cs x).isFacet = false
  hom : HomogeneousAt cs x
  cone : ∀ Y, Cov.le (covOfK cs x) Y = true → IsPreconnected (coneCell cs x Y)

/-- **Faithful, with transversal and conical points**: when every point is transversal or
conical, `FaithfulAll` is R1 alone. -/
theorem faithfulAll_of_mixed [FiniteDimensional ℝ E] {cs : List (Carrier E ℝ)}
    (hd : ∀ c ∈ cs, ContDiff ℝ 1 c) (hp : ∀ x, TransversalAt cs x ∨ ConicalAt cs x)
    (h1 : CellsConnected cs) : FaithfulAll cs := by
  have hc : ∀ c ∈ cs, Continuous c := fun c hc => (hd c hc).continuous
  have hl : ∀ x, LocalGood cs x := fun x => by
    rcases hp x with h | h
    · exact (localGood'_of_transversal hd h).toLocalGood
    · exact localGood_of_homogeneous hc h.hom h.cone
  refine faithfulAll_of_local hc h1 ?_ hl
  intro φ hφ x hx
  have hx' : covOfK cs x = φ := hx
  rcases hp x with h | h
  · have hg := localGood'_of_transversal hd h
    obtain ⟨hpl, hmi⟩ := le_plus_minus hφ
    rw [closure_iff, closure_iff]
    exact ⟨hg.near _ (by rw [hx']; exact hpl), hg.near _ (by rw [hx']; exact hmi)⟩
  · have := h.notFacet; rw [hx', hφ] at this; cases this

end ForgeCurved
