import ForgeCurved.Manifold
import Mathlib.Analysis.SpecialFunctions.Pow.Real
import Mathlib.Analysis.SpecialFunctions.Sqrt

/-!
# The test of the statements: two cylinders tangent along a line (design §1.3 (iii))

`c₁ = (x-1)² + y² - 1` and `c₂ = (x+1)² + y² - 1`: two unit cylinders along `z`, tangent along
the `z` axis. Their union is non-manifold along the axis: four sheets meet there. The bodies are
sign formulas (`c₁ < 0`, `c₂ < 0`), as in the Q1 domain.

**Without a seam** (`cs₀ = [c₁, c₂]`):
* `unseamed_facets`: the realised facets above the contact edge `[0, 0]` are two;
* `unseamed_no_refusal`: so **no exact ledger refuses** (`XNonManifoldK` is false), and the core
  would answer the union with no refusal: the wrong answer of §1.3 (iii);
* `unseamed_not_faithful`: the reason: R4 fails. The facet "on `c₁`, outside `c₂`" reaches the
  axis from both sides (`y > 0` and `y < 0`) in every small neighbourhood;
* `union_three_sheets`: and the geometry has the sheets: `ThreeSheets` holds at the origin for
  the regularised union.

**With the contact seam** `y = 0`, the plane holding both axes (`cs₁ = [c₁, c₂, y]`):
* `seamed_refuses`: **every exact ledger of the union refuses `ZeroThickness`**: three of the
  four sheets are now facets above the contact edge `[0, 0, 0]`.

So the statements discriminate: `xNonManifold_iff_top` cannot hold unseamed (its hypotheses fail:
`unseamed_not_faithful`), and with the seam the covector count sees the sheets.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.Tangent

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Ledger Filter Topology

/-- The points: `(x, y, z)`. -/
abbrev P3 := ℝ × ℝ × ℝ

def c₁ : Carrier P3 ℝ := fun p => (p.1 - 1) ^ 2 + p.2.1 ^ 2 - 1
def c₂ : Carrier P3 ℝ := fun p => (p.1 + 1) ^ 2 + p.2.1 ^ 2 - 1
/-- The contact seam: the plane through both axes. -/
def sy : Carrier P3 ℝ := fun p => p.2.1

def cs₀ : List (Carrier P3 ℝ) := [c₁, c₂]
def cs₁ : List (Carrier P3 ℝ) := [c₁, c₂, sy]

/-- The two solid cylinders, as sign formulas. -/
def Acyl : Nat → Pred P3
  | 0 => fun p => c₁ p < 0
  | 1 => fun p => c₂ p < 0
  | _ => fun _ => False

/-- The union of bodies 0 and 1. -/
def unionRule : Rule := fun m => m 0 || m 1

theorem c₁_cont : Continuous c₁ := by unfold c₁; fun_prop
theorem c₂_cont : Continuous c₂ := by unfold c₂; fun_prop
theorem sy_cont : Continuous sy := by unfold sy; fun_prop

theorem cont₀ : ∀ c ∈ cs₀, Continuous c := by
  simp only [cs₀, List.mem_cons, List.mem_nil_iff, or_false]
  rintro c (rfl | rfl)
  exacts [c₁_cont, c₂_cont]

theorem cont₁ : ∀ c ∈ cs₁, Continuous c := by
  simp only [cs₁, List.mem_cons, List.mem_nil_iff, or_false]
  rintro c (rfl | rfl | rfl)
  exacts [c₁_cont, c₂_cont, sy_cont]

/-- Off the axis, the two cylinders do not meet: on one, the other is positive or zero. -/
theorem c₂_of_c₁ (p : P3) (h : c₁ p = 0) : 0 ≤ c₂ p ∧ (c₂ p = 0 → p.1 = 0 ∧ p.2.1 = 0) := by
  unfold c₁ c₂ at *
  have hx : 0 ≤ p.1 := by nlinarith [sq_nonneg p.2.1, sq_nonneg (p.1 - 1)]
  refine ⟨by nlinarith, fun h2 => ?_⟩
  have : p.1 = 0 := by nlinarith
  refine ⟨this, ?_⟩
  rw [this] at h
  nlinarith [sq_nonneg p.2.1]

theorem c₁_of_c₂ (p : P3) (h : c₂ p = 0) : 0 ≤ c₁ p ∧ (c₁ p = 0 → p.1 = 0 ∧ p.2.1 = 0) := by
  unfold c₁ c₂ at *
  have hx : p.1 ≤ 0 := by nlinarith [sq_nonneg p.2.1, sq_nonneg (p.1 + 1)]
  refine ⟨by nlinarith, fun h1 => ?_⟩
  have : p.1 = 0 := by nlinarith
  refine ⟨this, ?_⟩
  rw [this] at h
  nlinarith [sq_nonneg p.2.1]

theorem cov₀ (p : P3) : covOfK cs₀ p = [sgnK (c₁ p), sgnK (c₂ p)] := rfl
theorem cov₁ (p : P3) : covOfK cs₁ p = [sgnK (c₁ p), sgnK (c₂ p), sgnK (sy p)] := rfl

/-! ## Without the seam -/

/-- The contact edge. -/
def E₀ : Cov := [.zero, .zero]
/-- On the first cylinder, outside the second. -/
def φ₀ : Cov := [.zero, .pos]

theorem shift_cell₀ (p : P3) : covOfK cs₀ (p.1, p.2.1, p.2.2 + 1) = covOfK cs₀ p := rfl

/-- Every cell of `cs₀` has two points (the carriers ignore `z`): no vertex. -/
theorem no_vert₀ (V : Cov) : ¬IsVertK cs₀ V := by
  rintro ⟨⟨p, hp⟩, h⟩
  have := h p (p.1, p.2.1, p.2.2 + 1) hp (by
    show covOfK cs₀ _ = V; rw [shift_cell₀]; exact hp)
  have := congrArg (fun q : P3 => q.2.2) this
  simp at this

theorem isEdge_E₀ : IsEdgeK cs₀ E₀ := by
  refine ⟨⟨(0, 0, 0), ?_⟩, ⟨0, 1, by decide, rfl, rfl⟩, no_vert₀ _⟩
  show covOfK cs₀ _ = _
  simp [cov₀, c₁, c₂, sgnK, E₀]

/-- The only edge cell of `cs₀` is the contact edge. -/
theorem edge_eq₀ {E : Cov} (he : IsEdgeK cs₀ E) : E = E₀ := by
  obtain ⟨⟨p, hp⟩, ⟨a, b, hab, ha, hb⟩, -⟩ := he
  have hp' : covOfK cs₀ p = E := hp
  rw [cov₀] at hp'
  subst hp'
  match a, b, ha, hb with
  | 0, 1, ha, hb => simp at ha hb; simp [E₀, ha, hb]
  | 1, 0, ha, hb => simp at ha hb; simp [E₀, ha, hb]
  | 0, 0, _, _ => exact absurd rfl hab
  | 1, 1, _, _ => exact absurd rfl hab
  | n + 2, _, ha, _ => simp at ha
  | _, n + 2, _, hb => simp at hb

theorem sgnK_c₂_pos {p : P3} (h1 : c₁ p = 0) (h2 : sgnK (c₂ p) ≠ .zero) : sgnK (c₂ p) = .pos := by
  rcases (c₂_of_c₁ p h1).1.lt_or_eq with h | h
  · exact sgnK_pos h
  · exact absurd (sgnK_zero_iff.mpr h.symm) h2

theorem sgnK_c₁_pos {p : P3} (h2 : c₂ p = 0) (h1 : sgnK (c₁ p) ≠ .zero) : sgnK (c₁ p) = .pos := by
  rcases (c₁_of_c₂ p h2).1.lt_or_eq with h | h
  · exact sgnK_pos h
  · exact absurd (sgnK_zero_iff.mpr h.symm) h1

/-- **Two facets above the contact edge**, without the seam. -/
theorem unseamed_facets {φ : Cov} (hr : RealisedK cs₀ φ) (hφ : φ.isFacet = true) :
    φ = [.zero, .pos] ∨ φ = [.pos, .zero] := by
  obtain ⟨p, hp⟩ := hr
  have hp' : covOfK cs₀ p = φ := hp
  rw [cov₀] at hp'
  subst hp'
  rw [isFacet_iff] at hφ
  by_cases h1 : sgnK (c₁ p) = .zero
  · have h2 : sgnK (c₂ p) ≠ .zero := fun h2 => by rw [h1, h2] at hφ; simp at hφ
    left; rw [h1, sgnK_c₂_pos (sgnK_zero_iff.mp h1) h2]
  · have h2 : sgnK (c₂ p) = .zero := by
      by_contra h2
      rw [List.count_cons_of_ne h1, List.count_cons_of_ne h2] at hφ; simp at hφ
    right; rw [h2, sgnK_c₁_pos (sgnK_zero_iff.mp h2) h1]

/-- **Without the seam, no exact ledger refuses the union**: the covectors see no
non-manifold cell, though the union has one (`union_three_sheets`). -/
theorem unseamed_no_refusal (L : Ledger) (A : Nat → Pred P3) (rule : Rule) :
    ¬XNonManifoldK L cs₀ A rule := by
  rintro (⟨E, -, hed, a, b, c, hab, hac, hbc, ⟨ha, -⟩, ⟨hb, -⟩, ⟨hc, -⟩⟩ | ⟨V, -, hv, -⟩)
  · rcases unseamed_facets ha.1 ha.2.1 with rfl | rfl <;>
    rcases unseamed_facets hb.1 hb.2.1 with rfl | rfl <;>
    rcases unseamed_facets hc.1 hc.2.1 with rfl | rfl <;> simp_all
  · exact no_vert₀ V hv

/-! ### R4 fails without the seam -/

/-- A point of the first cylinder near the axis, at height `y = t`. -/
noncomputable def qp (t : ℝ) : P3 := (1 - Real.sqrt (1 - t ^ 2), t, 0)

theorem c₁_qp {t : ℝ} (ht : t ^ 2 ≤ 1) : c₁ (qp t) = 0 := by
  unfold c₁ qp
  simp only
  have := Real.sq_sqrt (show 0 ≤ 1 - t ^ 2 by linarith)
  nlinarith

theorem c₂_qp {t : ℝ} (ht : t ^ 2 ≤ 1) (h0 : t ≠ 0) : 0 < c₂ (qp t) := by
  unfold c₂ qp
  simp only
  have hs := Real.sq_sqrt (show 0 ≤ 1 - t ^ 2 by linarith)
  have hlt : Real.sqrt (1 - t ^ 2) < 1 := by
    rw [Real.sqrt_lt' one_pos]; have := pow_pos (abs_pos.mpr h0) 2; rw [sq_abs] at this; linarith
  nlinarith [Real.sqrt_nonneg (1 - t ^ 2)]

theorem qp_cont : Continuous qp := by unfold qp; fun_prop

theorem qp_zero : qp 0 = (0, 0, 0) := by simp [qp]

theorem cell_qp {t : ℝ} (ht : t ^ 2 ≤ 1) (h0 : t ≠ 0) : covOfK cs₀ (qp t) = φ₀ := by
  rw [cov₀, sgnK_zero_iff.mpr (c₁_qp ht), sgnK_pos (c₂_qp ht h0)]; rfl

/-- Near the axis, the facet `φ₀` is off the plane `y = 0`. -/
theorem facet_off_seam {p : P3} (hp : covOfK cs₀ p = φ₀) (hx : |p.1| < 1) : p.2.1 ≠ 0 := by
  rw [cov₀] at hp
  simp only [φ₀, List.cons.injEq, and_true] at hp
  have h1 := sgnK_zero_iff.mp hp.1
  intro hy
  have : p.1 = 0 := by
    unfold c₁ at h1; rw [hy] at h1
    have := abs_lt.mp hx
    nlinarith
  have h2 : c₂ p = 0 := by unfold c₂; rw [this, hy]; norm_num
  rw [sgnK_zero_iff.mpr h2] at hp; exact absurd hp.2 (by decide)

/-- **R4 fails without the seam**: no small neighbourhood of the origin meets `φ₀` connectedly. -/
theorem wing_fails (U : Set P3) (hU : U ∈ 𝓝 (0, 0, 0)) (hUW : U ⊆ {p | |p.1| < 1}) :
    ¬IsPreconnected (cell cs₀ φ₀ ∩ U) := by
  intro hc
  have hq : ∀ᶠ t in 𝓝 (0 : ℝ), qp t ∈ U ∧ qp (-t) ∈ U := by
    have h1 : Tendsto qp (𝓝 0) (𝓝 (0, 0, 0)) := qp_zero ▸ qp_cont.tendsto 0
    have h2 : Tendsto (fun t => qp (-t)) (𝓝 0) (𝓝 (0, 0, 0)) := by
      have := (qp_cont.comp continuous_neg).tendsto 0
      simpa [Function.comp_def, qp_zero] using this
    exact (h1.eventually hU).and (h2.eventually hU)
  obtain ⟨ε, hε, hεq⟩ := Metric.eventually_nhds_iff.mp hq
  set t := min (ε / 2) (1 / 2)
  have ht0 : 0 < t := lt_min (by linarith) (by norm_num)
  have htε : dist t 0 < ε := by
    rw [Real.dist_eq, sub_zero, abs_of_pos ht0]
    exact lt_of_le_of_lt (min_le_left _ _) (by linarith)
  have ht1 : t ^ 2 ≤ 1 := by
    have : t ≤ 1 / 2 := min_le_right _ _
    nlinarith
  obtain ⟨hp, hm⟩ := hεq htε
  have hm1 : (-t) ^ 2 ≤ 1 := by rw [neg_sq]; exact ht1
  obtain ⟨w, hw, hwp, hwm⟩ := hc {p | 0 < p.2.1} {p | p.2.1 < 0}
    (isOpen_lt continuous_const (by fun_prop)) (isOpen_lt (by fun_prop) continuous_const)
    (fun p ⟨hpc, hpU⟩ => by
      rcases lt_trichotomy p.2.1 0 with h | h | h
      · exact Or.inr h
      · exact absurd h (facet_off_seam hpc (hUW hpU))
      · exact Or.inl h)
    ⟨qp t, ⟨cell_qp ht1 ht0.ne', hp⟩, by simp [qp, ht0]⟩
    ⟨qp (-t), ⟨cell_qp hm1 (neg_ne_zero.mpr ht0.ne'), hm⟩, by simp [qp, ht0]⟩
  exact absurd (lt_trans (show (0 : ℝ) < w.2.1 from hwp) (show w.2.1 < 0 from hwm)) (lt_irrefl _)

/-- **No faithful ledger without the seam**: a ledger listing the contact edge and the facet
`φ₀` fails R4 there. -/
theorem unseamed_not_faithful (L : Ledger) (hE : Listed L E₀) (hφ : Listed L φ₀) :
    ¬Faithful cs₀ (Listed L) := by
  intro hf
  obtain ⟨U, hUo, hUW, hxU, hUc⟩ := hf.wing E₀ φ₀ hE hφ isEdge_E₀ rfl rfl (0, 0, 0)
    (by show covOfK cs₀ _ = _; simp [cov₀, c₁, c₂, sgnK, E₀]) (fun p => |p.1| < 1)
    (show IsOpen {p : P3 | |p.1| < 1} from isOpen_lt (by fun_prop) continuous_const)
    (by simp)
  rw [connected_iff] at hUc
  exact wing_fails {p | U p} (hUo.mem_nhds hxU) (fun p hp => hUW p hp) hUc

/-! ### The geometry has three sheets -/

/-- The regularised union of the two solid cylinders. -/
def Ru : Set P3 := closure ({p | c₁ p < 0} ∪ {p | c₂ p < 0})

/-- The mirror point, on the second cylinder. -/
noncomputable def qm (t : ℝ) : P3 := (Real.sqrt (1 - t ^ 2) - 1, t, 0)

theorem c₂_qm {t : ℝ} (ht : t ^ 2 ≤ 1) : c₂ (qm t) = 0 := by
  unfold c₂ qm; simp only
  have := Real.sq_sqrt (show 0 ≤ 1 - t ^ 2 by linarith); nlinarith

theorem c₁_qm {t : ℝ} (ht : t ^ 2 ≤ 1) (h0 : t ≠ 0) : 0 < c₁ (qm t) := by
  have e : c₁ (qm t) = c₂ (qp t) := by unfold c₁ c₂ qm qp; simp only; ring
  rw [e]; exact c₂_qp ht h0

theorem qm_cont : Continuous qm := by unfold qm; fun_prop
theorem qm_zero : qm 0 = (0, 0, 0) := by simp [qm]

/-- A point of one cylinder, outside the other, is on the union's frontier. -/
theorem frontier_of_on (p : P3) (hon : c₁ p = 0 ∧ 0 < c₂ p ∨ c₂ p = 0 ∧ 0 < c₁ p) :
    p ∈ frontier Ru := by
  rw [(show IsClosed Ru from isClosed_closure).frontier_eq]
  -- the scaled points `o + s (p - o)` about the axis `o` of the cylinder `p` is on
  have scale : ∀ (o : ℝ) (c : Carrier P3 ℝ), (∀ q : P3, c q = (q.1 - o) ^ 2 + q.2.1 ^ 2 - 1) →
      c p = 0 → Tendsto (fun s : ℝ => ((o + s * (p.1 - o), s * p.2.1, p.2.2) : P3)) (𝓝 1) (𝓝 p) ∧
        ∀ s, c (o + s * (p.1 - o), s * p.2.1, p.2.2) = s ^ 2 - 1 := by
    intro o c hc h0
    refine ⟨?_, fun s => ?_⟩
    · have : Continuous (fun s : ℝ => ((o + s * (p.1 - o), s * p.2.1, p.2.2) : P3)) := by fun_prop
      have h := this.tendsto 1
      simpa using h
    · rw [hc]; rw [hc] at h0; simp only
      have : (p.1 - o) ^ 2 + p.2.1 ^ 2 = 1 := by linarith
      calc (o + s * (p.1 - o) - o) ^ 2 + (s * p.2.1) ^ 2 - 1 = s ^ 2 * ((p.1 - o) ^ 2 + p.2.1 ^ 2) - 1 := by ring
        _ = s ^ 2 - 1 := by rw [this]; ring
  have side : ∀ (o : ℝ) (c c' : Carrier P3 ℝ), (∀ q : P3, c q = (q.1 - o) ^ 2 + q.2.1 ^ 2 - 1) →
      Continuous c' → c p = 0 → 0 < c' p → (c = c₁ ∧ c' = c₂ ∨ c = c₂ ∧ c' = c₁) →
      p ∈ Ru ∧ p ∉ interior Ru := by
    intro o c c' hc hc' h0 hpos hcc
    obtain ⟨ht, hv⟩ := scale o c hc h0
    have hin : ∀ s, 0 < s → s < 1 → c (o + s * (p.1 - o), s * p.2.1, p.2.2) < 0 := fun s h1 h2 => by
      rw [hv]; nlinarith
    have hout : ∀ s, 1 < s → 0 < c (o + s * (p.1 - o), s * p.2.1, p.2.2) := fun s h1 => by
      rw [hv]; nlinarith
    have hU : ∀ q : P3, c q < 0 → q ∈ {p | c₁ p < 0} ∪ {p | c₂ p < 0} := fun q hq => by
      rcases hcc with ⟨rfl, -⟩ | ⟨rfl, -⟩
      · exact Or.inl hq
      · exact Or.inr hq
    refine ⟨?_, fun hI => ?_⟩
    · apply mem_closure_of_tendsto (ht.mono_left (nhdsWithin_le_nhds (s := Set.Iio (1 : ℝ))))
      filter_upwards [Ioo_mem_nhdsLT (show (0 : ℝ) < 1 by norm_num)] with s hs
      exact hU _ (hin s hs.1 hs.2)
    · -- just outside: off both cylinders, so off the closure
      have hev : ∀ᶠ s in 𝓝 (1 : ℝ), ((o + s * (p.1 - o), s * p.2.1, p.2.2) : P3) ∈ interior Ru ∧
          0 < c' (o + s * (p.1 - o), s * p.2.1, p.2.2) :=
        (ht.eventually (isOpen_interior.mem_nhds hI)).and
          (ht.eventually ((isOpen_lt continuous_const hc').mem_nhds hpos))
      obtain ⟨ε, hε, hεs⟩ := Metric.eventually_nhds_iff.mp hev
      set s := 1 + ε / 2
      have hs : dist s 1 < ε := by
        rw [Real.dist_eq]; simp only [s, add_sub_cancel_left]; rw [abs_of_pos (by linarith)]; linarith
      obtain ⟨hsI, hsc'⟩ := hεs hs
      have hsc := hout s (by simp only [s]; linarith)
      set q : P3 := (o + s * (p.1 - o), s * p.2.1, p.2.2)
      -- a neighbourhood of `q` where both carriers are positive misses the union
      have hN : {r : P3 | 0 < c r ∧ 0 < c' r} ∈ 𝓝 q := by
        have hcc : Continuous c := by
          rcases hcc with ⟨rfl, -⟩ | ⟨rfl, -⟩
          exacts [c₁_cont, c₂_cont]
        have ho : IsOpen {r : P3 | 0 < c r ∧ 0 < c' r} :=
          (isOpen_lt (continuous_const (y := (0 : ℝ))) hcc).inter
            (isOpen_lt (continuous_const (y := (0 : ℝ))) hc')
        exact ho.mem_nhds ⟨hsc, hsc'⟩
      obtain ⟨r, hrN, hrU⟩ := mem_closure_iff_nhds.mp (interior_subset hsI) _ hN
      rcases hcc with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
      · rcases hrU with h | h
        · exact absurd h (not_lt.mpr hrN.1.le)
        · exact absurd h (not_lt.mpr hrN.2.le)
      · rcases hrU with h | h
        · exact absurd h (not_lt.mpr hrN.2.le)
        · exact absurd h (not_lt.mpr hrN.1.le)
  rcases hon with ⟨h0, hpos⟩ | ⟨h0, hpos⟩
  · exact side 1 c₁ c₂ (fun q => rfl) c₂_cont h0 hpos (Or.inl ⟨rfl, rfl⟩)
  · exact side (-1) c₂ c₁ (fun q => by unfold c₂; ring_nf) c₁_cont h0 hpos
      (Or.inr ⟨rfl, rfl⟩)

/-- **The union has (at least) three sheets at the origin**, as sets: the facet points near the
axis sit in four open quadrants of the plane `z = 0`'s projection, none joined to another. -/
theorem union_three_sheets : ThreeSheets cs₀ Ru (0, 0, 0) := by
  refine ⟨{p | |p.1| < 1}, (isOpen_lt (by fun_prop) continuous_const).mem_nhds (by simp),
    fun U hU => ?_⟩
  have hq : ∀ᶠ t in 𝓝 (0 : ℝ), qp t ∈ U ∧ qp (-t) ∈ U ∧ qm t ∈ U := by
    have h1 : Tendsto qp (𝓝 0) (𝓝 (0, 0, 0)) := qp_zero ▸ qp_cont.tendsto 0
    have h2 : Tendsto (fun t => qp (-t)) (𝓝 0) (𝓝 (0, 0, 0)) := by
      have := (qp_cont.comp continuous_neg).tendsto 0
      simpa [Function.comp_def, qp_zero] using this
    have h3 : Tendsto qm (𝓝 0) (𝓝 (0, 0, 0)) := qm_zero ▸ qm_cont.tendsto 0
    exact (h1.eventually hU).and ((h2.eventually hU).and (h3.eventually hU))
  obtain ⟨ε, hε, hεq⟩ := Metric.eventually_nhds_iff.mp hq
  set t := min (ε / 2) (1 / 2)
  have ht0 : 0 < t := lt_min (by linarith) (by norm_num)
  have htε : dist t 0 < ε := by
    rw [Real.dist_eq, sub_zero, abs_of_pos ht0]
    exact lt_of_le_of_lt (min_le_left _ _) (by linarith)
  have ht1 : t ^ 2 ≤ 1 := by have : t ≤ 1 / 2 := min_le_right _ _; nlinarith
  have hm1 : (-t) ^ 2 ≤ 1 := by rw [neg_sq]; exact ht1
  obtain ⟨hp, hm, hq'⟩ := hεq htε
  have tn : t ≠ 0 := ht0.ne'
  have tn' : -t ≠ 0 := neg_ne_zero.mpr tn
  have sheet : ∀ p, (c₁ p = 0 ∧ 0 < c₂ p ∨ c₂ p = 0 ∧ 0 < c₁ p) → p ∈ sheetSet cs₀ Ru := by
    intro p hon
    refine ⟨frontier_of_on p hon, ?_⟩
    show (covOfK cs₀ p).isFacet = true
    rw [isFacet_iff, cov₀]
    rcases hon with ⟨h0, hpos⟩ | ⟨h0, hpos⟩
    · rw [sgnK_zero_iff.mpr h0, sgnK_pos hpos]; rfl
    · rw [sgnK_zero_iff.mpr h0, sgnK_pos hpos]; rfl
  -- the quadrant of a sheet point near the axis is constant on joined sets
  let quad : P3 → Bool × Bool := fun p => (decide (0 < p.1), decide (0 < p.2.1))
  have hloc : ∀ y ∈ sheetSet cs₀ Ru ∩ {p | |p.1| < 1}, ∃ N ∈ 𝓝 y,
      ∀ z ∈ N ∩ (sheetSet cs₀ Ru ∩ {p | |p.1| < 1}), quad z = quad y := by
    intro y ⟨⟨_, hyf⟩, hy1⟩
    -- a sheet point near the axis is off both coordinate planes
    have off : ∀ p : P3, (covOfK cs₀ p).isFacet = true → |p.1| < 1 → p.1 ≠ 0 ∧ p.2.1 ≠ 0 := by
      intro p hf hx
      rcases unseamed_facets ⟨p, rfl⟩ hf with e | e
      · have hy := facet_off_seam e hx
        rw [cov₀] at e
        simp only [List.cons.injEq] at e
        have h1 := sgnK_zero_iff.mp e.1
        refine ⟨fun h0 => ?_, hy⟩
        have : c₂ p = 0 := by unfold c₁ at h1; unfold c₂; rw [h0] at h1 ⊢; linarith
        rw [sgnK_zero_iff.mpr this] at e; exact absurd e.2.1 (by decide)
      · rw [cov₀] at e
        simp only [List.cons.injEq] at e
        have h2 := sgnK_zero_iff.mp e.2.1
        have hpos : 0 < c₁ p := by
          rcases lt_trichotomy (c₁ p) 0 with h | h | h
          · rw [sgnK_neg h] at e; exact absurd e.1 (by decide)
          · rw [sgnK_zero_iff.mpr h] at e; exact absurd e.1 (by decide)
          · exact h
        refine ⟨fun h0 => ?_, fun h0 => ?_⟩
        · unfold c₂ at h2; unfold c₁ at hpos; rw [h0] at h2 hpos; nlinarith
        · have : p.1 = 0 := by
            unfold c₂ at h2; rw [h0] at h2
            have := abs_lt.mp hx; nlinarith
          unfold c₁ at hpos; rw [this, h0] at hpos; norm_num at hpos
    obtain ⟨hx0, hy0⟩ := off y hyf hy1
    have hN : {p : P3 | (0 < p.1 ↔ 0 < y.1) ∧ (0 < p.2.1 ↔ 0 < y.2.1)} ∈ 𝓝 y := by
      have h1 : {p : P3 | 0 < p.1 * y.1} ∈ 𝓝 y :=
        (isOpen_lt continuous_const (by fun_prop)).mem_nhds (by simp; positivity)
      have h2 : {p : P3 | 0 < p.2.1 * y.2.1} ∈ 𝓝 y :=
        (isOpen_lt continuous_const (by fun_prop)).mem_nhds (by simp; positivity)
      filter_upwards [h1, h2] with p hp1 hp2
      exact ⟨⟨fun h => by nlinarith [mul_pos_iff.mp hp1], fun h => by nlinarith [mul_pos_iff.mp hp1]⟩,
        ⟨fun h => by nlinarith [mul_pos_iff.mp hp2], fun h => by nlinarith [mul_pos_iff.mp hp2]⟩⟩
    refine ⟨_, hN, fun z ⟨hzN, _⟩ => ?_⟩
    simp only [quad, Prod.mk.injEq, decide_eq_decide]
    exact hzN
  have nj : ∀ a b, a ∈ sheetSet cs₀ Ru → b ∈ sheetSet cs₀ Ru → |a.1| < 1 → |b.1| < 1 →
      quad a ≠ quad b → ¬Joined (sheetSet cs₀ Ru ∩ {p | |p.1| < 1}) a b := by
    rintro a b - - - - hne (e | ⟨C, hCS, hC, ha, hb⟩)
    · exact hne (e ▸ rfl)
    · exact hne (const_of_loc hloc hC hCS ha hb)
  have sp := sheet (qp t) (Or.inl ⟨c₁_qp ht1, c₂_qp ht1 tn⟩)
  have sm := sheet (qp (-t)) (Or.inl ⟨c₁_qp hm1, c₂_qp hm1 tn'⟩)
  have sq := sheet (qm t) (Or.inr ⟨c₂_qm ht1, c₁_qm ht1 tn⟩)
  have hs : Real.sqrt (1 - t ^ 2) < 1 := by
    rw [Real.sqrt_lt' one_pos]; nlinarith
  have hs0 : 1 / 2 < Real.sqrt (1 - t ^ 2) := by
    rw [Real.lt_sqrt (by norm_num)]; have : t ≤ 1 / 2 := min_le_right _ _; nlinarith
  have x1 : |(qp t).1| < 1 := by simp only [qp]; rw [abs_lt]; constructor <;> linarith
  have x2 : |(qp (-t)).1| < 1 := by simp only [qp, neg_sq]; rw [abs_lt]; constructor <;> linarith
  have x3 : |(qm t).1| < 1 := by simp only [qm]; rw [abs_lt]; constructor <;> linarith
  have a1 : 0 < (qp t).1 := by simp only [qp]; linarith
  have a3 : ¬0 < (qm t).1 := by simp only [qm]; linarith
  have a2 : 0 < (qp (-t)).1 := by simp only [qp, neg_sq]; linarith
  have b1 : 0 < (qp t).2.1 := by simp [qp, ht0]
  have b2 : ¬0 < (qp (-t)).2.1 := by simp [qp, ht0.le]
  refine ⟨qp t, ⟨sp, hp⟩, qp (-t), ⟨sm, hm⟩, qm t, ⟨sq, hq'⟩,
    nj _ _ sp sm x1 x2 ?_, nj _ _ sp sq x1 x3 ?_, nj _ _ sm sq x2 x3 ?_⟩
  · intro e; have := congrArg Prod.snd e; simp only [quad, b1, b2, decide_true, decide_false] at this
    exact absurd this (by decide)
  · intro e; have := congrArg Prod.fst e; simp only [quad, a1, a3, decide_true, decide_false] at this
    exact absurd this (by decide)
  · intro e; have := congrArg Prod.fst e; simp only [quad, a2, a3, decide_true, decide_false] at this
    exact absurd this (by decide)

/-! ## With the contact seam -/

def E₁ : Cov := [.zero, .zero, .zero]

theorem shift_cell₁ (p : P3) : covOfK cs₁ (p.1, p.2.1, p.2.2 + 1) = covOfK cs₁ p := rfl

theorem no_vert₁ (V : Cov) : ¬IsVertK cs₁ V := by
  rintro ⟨⟨p, hp⟩, h⟩
  have := h p (p.1, p.2.1, p.2.2 + 1) hp (by
    show covOfK cs₁ _ = V; rw [shift_cell₁]; exact hp)
  have := congrArg (fun q : P3 => q.2.2) this
  simp at this

theorem isEdge_E₁ : IsEdgeK cs₁ E₁ := by
  refine ⟨⟨(0, 0, 0), ?_⟩, ⟨0, 1, by decide, rfl, rfl⟩, no_vert₁ _⟩
  show covOfK cs₁ _ = _
  simp [cov₁, c₁, c₂, sy, sgnK, E₁]

/-- A side cell with a point inside a cylinder, or outside both. -/
theorem memTK_of {T : Cov} {i : Nat} (p : P3) (hp : covOfK cs₁ p = T) (hA : Acyl i p) :
    memTK cs₁ Acyl T i := ⟨p, hp, hA⟩

/-- The cell outside both cylinders holds no point of either. -/
theorem not_memTK_out {s : Sgn} (i : Nat) : ¬memTK cs₁ Acyl [.pos, .pos, s] i := by
  rintro ⟨p, hp, hA⟩
  have hp' : covOfK cs₁ p = _ := hp
  rw [cov₁] at hp'
  simp only [List.cons.injEq] at hp'
  have h1 : ¬c₁ p < 0 := fun h => by rw [sgnK_neg h] at hp'; exact absurd hp'.1 (by decide)
  have h2 : ¬c₂ p < 0 := fun h => by rw [sgnK_neg h] at hp'; exact absurd hp'.2.1 (by decide)
  match i, hA with
  | 0, hA => exact h1 hA
  | 1, hA => exact h2 hA

theorem xkeep_out (L : Ledger) (s : Sgn) :
    xkeepK L cs₁ Acyl unionRule [.pos, .pos, s] = false := by
  unfold xkeepK unionRule
  simp only [Bool.or_eq_false_iff, decide_eq_false_iff_not, not_and]
  exact ⟨fun _ => not_memTK_out 0, fun _ => not_memTK_out 1⟩

theorem xkeep_in (L : Ledger) (hnb : L.nb = 2) {T : Cov} {i : Nat} (hi : i < 2) (p : P3)
    (hp : covOfK cs₁ p = T) (hA : Acyl i p) : xkeepK L cs₁ Acyl unionRule T = true := by
  unfold xkeepK unionRule
  have hm := memTK_of p hp hA
  match i, hi with
  | 0, _ => simp [hnb, hm]
  | 1, _ => simp [hnb, hm]

/-- A seamed facet whose `plus` side is outside both cylinders and whose `minus` side holds a
point of one is an exact boundary facet of the union. -/
theorem bdry_facet (L : Ledger) (hnb : L.nb = 2) {φ : Cov} (hf : φ.isFacet = true)
    (pt : P3) (hpt : covOfK cs₁ pt = φ) (s : Sgn) (hout : φ.plus = [.pos, .pos, s])
    (q : P3) (i : Nat) (hi : i < 2) (hq : covOfK cs₁ q = φ.minus) (hA : Acyl i q) :
    XBoundaryK L cs₁ Acyl unionRule φ := by
  refine ⟨⟨pt, hpt⟩, hf, ?_⟩
  rw [hout, xkeep_out, xkeep_in L hnb hi q hq hA]
  decide

/-- **With the seam, every exact ledger of the union refuses `ZeroThickness`**: three facets
(`c₁` above and below the seam, `c₂` above it) are exact boundary facets above the contact edge. -/
theorem seamed_refuses (L : Ledger) (hnb : L.nb = 2) (h : ExactK L cs₁ Acyl) :
    ∃ x, core L unionRule = .refused (.zeroThickness x) := by
  rw [zeroThickness_iff_k h]
  have tq : (1 / 2 : ℝ) ^ 2 ≤ 1 := by norm_num
  have tq' : (-(1 / 2) : ℝ) ^ 2 ≤ 1 := by norm_num
  have s₀ : ∀ t : ℝ, 0 < t → sgnK (sy (qp t)) = .pos := fun t ht => sgnK_pos (by simp [sy, qp, ht])
  have s₀' : ∀ t : ℝ, 0 < t → sgnK (sy (qp (-t))) = .neg := fun t ht => sgnK_neg (by simp [sy, qp, ht])
  have s₁ : ∀ t : ℝ, 0 < t → sgnK (sy (qm t)) = .pos := fun t ht => sgnK_pos (by simp [sy, qm, ht])
  have pa : covOfK cs₁ (qp (1 / 2)) = [.zero, .pos, .pos] := by
    rw [cov₁, sgnK_zero_iff.mpr (c₁_qp tq), sgnK_pos (c₂_qp tq (by norm_num)), s₀ _ (by norm_num)]
  have pb : covOfK cs₁ (qp (-(1 / 2))) = [.zero, .pos, .neg] := by
    rw [cov₁, sgnK_zero_iff.mpr (c₁_qp tq'), sgnK_pos (c₂_qp tq' (by norm_num)), s₀' _ (by norm_num)]
  have pc : covOfK cs₁ (qm (1 / 2)) = [.pos, .zero, .pos] := by
    rw [cov₁, sgnK_zero_iff.mpr (c₂_qm tq), sgnK_pos (c₁_qm tq (by norm_num)), s₁ _ (by norm_num)]
  -- inner side points: inside a cylinder, on the given side of the seam, outside the other
  have ia : covOfK cs₁ ((1, 1 / 2, 0) : P3) = [.neg, .pos, .pos] := by
    simp only [cov₁, c₁, c₂, sy]; norm_num [sgnK]
  have ib : covOfK cs₁ ((1, -(1 / 2), 0) : P3) = [.neg, .pos, .neg] := by
    simp only [cov₁, c₁, c₂, sy]; norm_num [sgnK]
  have ic : covOfK cs₁ ((-1, 1 / 2, 0) : P3) = [.pos, .neg, .pos] := by
    simp only [cov₁, c₁, c₂, sy]; norm_num [sgnK]
  refine Or.inl ⟨E₁, isEdge_E₁.1, isEdge_E₁, [.zero, .pos, .pos], [.zero, .pos, .neg],
    [.pos, .zero, .pos], by decide, by decide, by decide, ⟨?_, rfl⟩, ⟨?_, rfl⟩, ⟨?_, rfl⟩⟩
  · exact bdry_facet L hnb rfl _ pa .pos rfl _ 0 (by norm_num) (by rw [ia]; rfl)
      (by show c₁ _ < 0; simp [c₁]; norm_num)
  · exact bdry_facet L hnb rfl _ pb .neg rfl _ 0 (by norm_num) (by rw [ib]; rfl)
      (by show c₁ _ < 0; simp [c₁]; norm_num)
  · exact bdry_facet L hnb rfl _ pc .pos rfl _ 1 (by norm_num) (by rw [ic]; rfl)
      (by show c₂ _ < 0; simp [c₂]; norm_num)

end ForgeCurved.Tangent
