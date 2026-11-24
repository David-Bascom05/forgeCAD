import ForgeCurved.EndToEnd
import Mathlib.Analysis.Calculus.Deriv.Slope
import Mathlib.Analysis.Calculus.Deriv.Comp
import Mathlib.Analysis.Calculus.Deriv.Mul
import Mathlib.Analysis.Calculus.Deriv.Add
import Mathlib.Analysis.Calculus.FDeriv.Prod
import Mathlib.Analysis.Calculus.FDeriv.Mul

/-!
# R3 for every facet of a regular arrangement (design §1.3: "R3 holds wherever the facet's
surface is smooth, by the implicit function theorem")

`Faithful.sides` (R3) asks both sides of a listed facet to be beside it at each of its points.
Over a normed space, if the facet's carrier has a nonzero derivative `D` at a point `x` of the
facet, the line `x + t v` with `D v > 0` leaves the carrier to its positive side for small `t > 0`
and to its negative side for small `t < 0`, while every other carrier keeps its sign: so `x` is in
the closure of both sides (`sides_of_fderiv`).

The singular points of a carrier (the cone's apex; a degenerate quadric's singular line) are
where this fails, and the design makes them vertices by its seams (§2.3: the apex lies on the
nappe plane and the axial seam). `SingularSeamed`: **every singular zero of a carrier lies on
another carrier**. Under it, R3 holds for every facet, listed or not (`sides_of_seamed`), and for
quadric carriers in particular (`quadric_sides`). The planes (seams, walls) have constant nonzero
derivative, so they never add a singular point.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Filter Topology

/-- Two distinct zero positions make two zeros. -/
theorem two_le_count : ∀ (X : Cov) (j k : Nat), j ≠ k → X[j]? = some Sgn.zero →
    X[k]? = some Sgn.zero → 2 ≤ X.count .zero
  | [], _, _, _, h, _ => by simp at h
  | a :: X, 0, 0, hjk, _, _ => absurd rfl hjk
  | a :: X, 0, k + 1, _, hj, hk => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hj
    subst hj
    simp only [List.getElem?_cons_succ] at hk
    have := count_pos_of_mem (List.mem_of_getElem? hk)
    simp only [List.count_cons_self]; omega
  | a :: X, j + 1, 0, _, hj, hk => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hk
    subst hk
    simp only [List.getElem?_cons_succ] at hj
    have := count_pos_of_mem (List.mem_of_getElem? hj)
    simp only [List.count_cons_self]; omega
  | a :: X, j + 1, k + 1, hjk, hj, hk => by
    simp only [List.getElem?_cons_succ] at hj hk
    have := two_le_count X j k (fun e => hjk (by rw [e])) hj hk
    by_cases ha : a = .zero
    · subst ha; simp only [List.count_cons_self]; omega
    · rw [List.count_cons_of_ne ha]; exact this

theorem getElem?_covOfK {P : Type} (cs : List (Carrier P ℝ)) (x : P) (j : Nat) :
    (covOfK cs x)[j]? = cs[j]?.map fun c => sgnK (c x) := by
  simp [covOfK]

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- The zero position of a facet holds a zero. -/
theorem facet_zero {φ : Cov} (hφ : φ.isFacet = true) : φ[φ.zeroIdx]? = some Sgn.zero := by
  have hm : Sgn.zero ∈ φ := List.count_pos_iff.mp (by rw [(isFacet_iff φ).mp hφ]; omega)
  have hl := List.idxOf_lt_length_iff.mpr hm
  show φ[φ.idxOf Sgn.zero]? = _
  rw [List.getElem?_eq_getElem hl, List.getElem_idxOf hl]

/-- **R3 at a regular point**: a nonzero derivative of the facet's carrier puts both sides
beside the facet. -/
theorem sides_of_fderiv {cs : List (Carrier E ℝ)} (hc : ∀ c ∈ cs, Continuous c) {x : E}
    (hφ : (covOfK cs x).isFacet = true) {c : Carrier E ℝ}
    (hcj : cs[(covOfK cs x).zeroIdx]? = some c) {D : E →L[ℝ] ℝ} (hD : HasFDerivAt c D x)
    (hD0 : D ≠ 0) :
    x ∈ closure (cell cs (covOfK cs x).plus) ∧ x ∈ closure (cell cs (covOfK cs x).minus) := by
  set φ := covOfK cs x
  set j := φ.zeroIdx
  have hzero : φ[j]? = some Sgn.zero := facet_zero hφ
  have hcx : c x = 0 := by
    have := getElem?_covOfK cs x j
    rw [hzero, hcj] at this
    simp only [Option.map_some, Option.some.injEq] at this
    exact sgnK_zero_iff.mp this.symm
  have hjl : j < φ.length := by
    by_contra h; rw [List.getElem?_eq_none (by omega)] at hzero; cases hzero
  -- a direction of increase
  obtain ⟨v, hv⟩ : ∃ v, 0 < D v := by
    by_contra hno
    push Not at hno
    apply hD0
    ext w
    have h1 := hno w
    have h2 := hno (-w)
    rw [map_neg] at h2
    show D w = 0
    linarith
  -- along the line, the carrier's slope at 0 is `D v > 0`
  have hline : HasDerivAt (fun t : ℝ => c (x + t • v)) (D v) 0 := by
    have hl : HasDerivAt (fun t : ℝ => x + t • v) v 0 := by
      simpa using ((hasDerivAt_id (0 : ℝ)).smul_const v).const_add x
    exact hD.comp_hasDerivAt_of_eq 0 hl (by simp)
  have hslope := hline.tendsto_slope_zero
  have hev : ∀ᶠ t in 𝓝[≠] (0 : ℝ), 0 < t⁻¹ • (c (x + (0 + t) • v) - c (x + (0 : ℝ) • v)) :=
    hslope.eventually (lt_mem_nhds hv)
  simp only [zero_add, zero_smul, add_zero, hcx, sub_zero, smul_eq_mul] at hev
  have hcont : Tendsto (fun t : ℝ => x + t • v) (𝓝 0) (𝓝 x) := by
    have : Continuous fun t : ℝ => x + t • v := by fun_prop
    simpa using this.tendsto 0
  have hle := hcont.eventually (eventually_le hc x)
  -- a point on the line, above `φ`, with a sign at `j`, is the side of that sign
  have side : ∀ t : ℝ, Cov.le φ (covOfK cs (x + t • v)) = true → ∀ s, s ≠ Sgn.zero →
      sgnK (c (x + t • v)) = s → covOfK cs (x + t • v) = φ.set j s := by
    intro t hlt s hs hsg
    set Y := covOfK cs (x + t • v)
    have hYj : Y[j]? = some s := by rw [getElem?_covOfK, hcj]; simp [hsg]
    have hfull : Sgn.zero ∉ Y := by
      intro hz
      have e := eq_of_le_of_count hlt (by
        rw [(isFacet_iff φ).mp hφ]; exact count_pos_of_mem hz)
      rw [← e, hzero] at hYj
      exact hs (Option.some.inj hYj).symm
    rcases full_above_facet hlt hφ hfull with e | e
    · rw [e] at hYj ⊢
      have : (φ.set j Sgn.pos)[j]? = some Sgn.pos := List.getElem?_set_self hjl
      unfold Cov.plus at hYj
      rw [this] at hYj
      rw [← Option.some.inj hYj]; rfl
    · rw [e] at hYj ⊢
      have : (φ.set j Sgn.neg)[j]? = some Sgn.neg := List.getElem?_set_self hjl
      unfold Cov.minus at hYj
      rw [this] at hYj
      rw [← Option.some.inj hYj]; rfl
  constructor
  · apply mem_closure_of_tendsto (hcont.mono_left (nhdsWithin_le_nhds (s := Set.Ioi 0)))
    have h1 : ∀ᶠ t in 𝓝[>] (0 : ℝ), 0 < t⁻¹ * c (x + t • v) :=
      hev.filter_mono (nhdsWithin_mono _ fun t ht => ne_of_gt ht)
    filter_upwards [h1, nhdsWithin_le_nhds hle, self_mem_nhdsWithin] with t ht hlt ht0
    have hpos : 0 < c (x + t • v) := by
      have := (inv_pos.mpr (show (0 : ℝ) < t from ht0)); nlinarith [mul_pos_iff.mp ht]
    exact side t hlt .pos (by decide) (sgnK_pos hpos)
  · apply mem_closure_of_tendsto (hcont.mono_left (nhdsWithin_le_nhds (s := Set.Iio 0)))
    have h1 : ∀ᶠ t in 𝓝[<] (0 : ℝ), 0 < t⁻¹ * c (x + t • v) :=
      hev.filter_mono (nhdsWithin_mono _ fun t ht => ne_of_lt ht)
    filter_upwards [h1, nhdsWithin_le_nhds hle, self_mem_nhdsWithin] with t ht hlt ht0
    have hneg : c (x + t • v) < 0 := by
      have ht0' : t < 0 := ht0
      rcases mul_pos_iff.mp ht with ⟨h1, _⟩ | ⟨_, h2⟩
      · exact absurd (inv_pos.mp h1) (not_lt.mpr ht0'.le)
      · exact h2
    exact side t hlt .neg (by decide) (sgnK_neg hneg)

/-- **The seams make every singular point a vertex**: a zero of a carrier at which its
derivative vanishes lies on another carrier too. -/
def SingularSeamed (cs : List (Carrier E ℝ)) : Prop :=
  ∀ (x : E) (j : Nat) (c : Carrier E ℝ), cs[j]? = some c → c x = 0 → fderiv ℝ c x = 0 →
    ∃ (k : Nat) (c' : Carrier E ℝ), k ≠ j ∧ cs[k]? = some c' ∧ c' x = 0

/-- **R3 for every facet**, of differentiable carriers whose singular points are seamed. -/
theorem sides_of_seamed {cs : List (Carrier E ℝ)} (hc : ∀ c ∈ cs, Continuous c)
    (hd : ∀ c ∈ cs, Differentiable ℝ c) (hs : SingularSeamed cs) (Listed : Cov → Prop) :
    ∀ φ, Listed φ → φ.isFacet = true → ∀ x, covCellK cs φ x →
      ForgeExact.Topology.closure (covCellK cs φ.plus) x ∧
        ForgeExact.Topology.closure (covCellK cs φ.minus) x := by
  intro φ _ hφ x hx
  have hx' : covOfK cs x = φ := hx
  subst hx'
  set j := (covOfK cs x).zeroIdx
  have hzero := facet_zero hφ
  obtain ⟨c, hcj⟩ : ∃ c, cs[j]? = some c := by
    rw [getElem?_covOfK] at hzero
    cases h : cs[j]? with
    | none => rw [h] at hzero; cases hzero
    | some c => exact ⟨c, rfl⟩
  have hcx : c x = 0 := by
    have := getElem?_covOfK cs x j
    rw [hzero, hcj] at this
    simp only [Option.map_some, Option.some.injEq] at this
    exact sgnK_zero_iff.mp this.symm
  have hcd : Differentiable ℝ c := hd c (List.mem_of_getElem? hcj)
  have hD0 : fderiv ℝ c x ≠ 0 := by
    intro h0
    obtain ⟨k, c', hkj, hk, hc'⟩ := hs x j c hcj hcx h0
    have hz : (covOfK cs x)[k]? = some Sgn.zero := by
      rw [getElem?_covOfK, hk]; simp [sgnK_zero_iff.mpr hc']
    have := two_le_count _ j k (Ne.symm hkj) hzero hz
    rw [(isFacet_iff _).mp hφ] at this; omega
  obtain ⟨hp, hm⟩ := sides_of_fderiv hc hφ hcj (hcd x).hasFDerivAt hD0
  rw [closure_iff, closure_iff, cell_eq, cell_eq]
  exact ⟨hp, hm⟩

/-- A quadric is differentiable. -/
theorem coord_diff (i : Fin 3) : Differentiable ℝ fun p : R3 => coord p i := by
  fin_cases i <;> simp [coord]

theorem Quadric.differentiable (q : Quadric) : Differentiable ℝ q.eval := by
  unfold Quadric.eval
  have := coord_diff
  fun_prop

/-- **R3 on the Q1 domain**: quadric carriers whose singular points are seamed have both sides
beside every facet. -/
theorem quadric_sides (qs : List Quadric) (hs : SingularSeamed (qs.map Quadric.eval))
    (Listed : Cov → Prop) :
    ∀ φ, Listed φ → φ.isFacet = true → ∀ x, covCellK (qs.map Quadric.eval) φ x →
      ForgeExact.Topology.closure (covCellK (qs.map Quadric.eval) φ.plus) x ∧
        ForgeExact.Topology.closure (covCellK (qs.map Quadric.eval) φ.minus) x :=
  sides_of_seamed (quadrics_cont qs) (fun c hc => by
    obtain ⟨q, _, rfl⟩ := List.mem_map.mp hc; exact q.differentiable) hs Listed

end ForgeCurved
