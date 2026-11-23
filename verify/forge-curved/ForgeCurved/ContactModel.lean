import ForgeCurved.FaceModel
import ForgeCurved.Homogeneous
import ForgeCurved.ConeModel
import ForgeCurved.EndToEnd

/-!
# Tangent contacts: ordered graphs

Two surfaces tangent along a curve, with the contact seam through it (design §2.3, "contact
curve"), are not transversal there. They are graphs over their common tangent plane, though: in
coordinates `(u, w, z)` (`u` normal to the tangent plane, `w` across the seam, `z` along the
contact), each is `u = aₖ(w, z)` and the seam is `w = 0`. Off the seam the graphs do not meet, so
on each side of it (and on it) their order is fixed.

**`graphCell_preconnected`**: over a connected base `D` on which the graphs keep their order and
stay within `|u| < δ`, every sign class `{(u, d) : d ∈ D, |u| < δ, sgn (u - aₖ d) = σₖ}` is
connected. It is a band between two graphs (or one graph), each fibre an interval, joined
through a continuous section.

**`localConn_of_graphs`**: a point modelled on ordered graphs and a seam, through a homeomorphism,
has local connectivity (`LocalConn`), which is what the face models need (FaceModel.lean). The
closures the models ask for (`EdgeModel.near` and the like) are the builder's, cell by cell: at a
contact, a cell realised only far away does not come near.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology Set

/-! ## Max and min of finitely many continuous functions -/

/-- The largest of `b` and the values `f k d`, `k ∈ l`. -/
def lmax {α : Type} (f : α → ℝ) (b : ℝ) (l : List α) : ℝ := (l.map f).foldr max b
def lmin {α : Type} (f : α → ℝ) (b : ℝ) (l : List α) : ℝ := (l.map f).foldr min b

theorem lmax_lt {α : Type} {f : α → ℝ} {b c : ℝ} :
    ∀ {l : List α}, b < c → (∀ k ∈ l, f k < c) → lmax f b l < c
  | [], hb, _ => hb
  | k :: l, hb, h => by
    unfold lmax at *
    simp only [List.map_cons, List.foldr_cons]
    exact max_lt (h k (List.mem_cons_self ..)) (lmax_lt (l := l) hb fun j hj =>
      h j (List.mem_cons_of_mem _ hj))

theorem lt_lmin {α : Type} {f : α → ℝ} {b c : ℝ} :
    ∀ {l : List α}, c < b → (∀ k ∈ l, c < f k) → c < lmin f b l
  | [], hb, _ => hb
  | k :: l, hb, h => by
    unfold lmin at *
    simp only [List.map_cons, List.foldr_cons]
    exact lt_min (h k (List.mem_cons_self ..)) (lt_lmin (l := l) hb fun j hj =>
      h j (List.mem_cons_of_mem _ hj))

theorem le_lmax {α : Type} {f : α → ℝ} {b : ℝ} :
    ∀ {l : List α}, b ≤ lmax f b l ∧ ∀ k ∈ l, f k ≤ lmax f b l
  | [] => ⟨le_rfl, fun _ h => absurd h List.not_mem_nil⟩
  | k :: l => by
    have ih := le_lmax (f := f) (b := b) (l := l)
    unfold lmax at *
    simp only [List.map_cons, List.foldr_cons]
    refine ⟨ih.1.trans (le_max_right _ _), fun j hj => ?_⟩
    rcases List.mem_cons.mp hj with rfl | hj
    · exact le_max_left _ _
    · exact (ih.2 j hj).trans (le_max_right _ _)

theorem lmin_le {α : Type} {f : α → ℝ} {b : ℝ} :
    ∀ {l : List α}, lmin f b l ≤ b ∧ ∀ k ∈ l, lmin f b l ≤ f k
  | [] => ⟨le_rfl, fun _ h => absurd h List.not_mem_nil⟩
  | k :: l => by
    have ih := lmin_le (f := f) (b := b) (l := l)
    unfold lmin at *
    simp only [List.map_cons, List.foldr_cons]
    refine ⟨(min_le_right _ _).trans ih.1, fun j hj => ?_⟩
    rcases List.mem_cons.mp hj with rfl | hj
    · exact min_le_left _ _
    · exact (min_le_right _ _).trans (ih.2 j hj)

theorem continuous_lmax {α X : Type} [TopologicalSpace X] {a : α → X → ℝ}
    (ha : ∀ k, Continuous (a k)) (b : ℝ) :
    ∀ l : List α, Continuous fun d => lmax (fun k => a k d) b l
  | [] => continuous_const
  | k :: l => by
    unfold lmax
    simp only [List.map_cons, List.foldr_cons]
    exact (ha k).max (continuous_lmax ha b l)

theorem continuous_lmin {α X : Type} [TopologicalSpace X] {a : α → X → ℝ}
    (ha : ∀ k, Continuous (a k)) (b : ℝ) :
    ∀ l : List α, Continuous fun d => lmin (fun k => a k d) b l
  | [] => continuous_const
  | k :: l => by
    unfold lmin
    simp only [List.map_cons, List.foldr_cons]
    exact (ha k).min (continuous_lmin ha b l)

/-! ## Bands between graphs -/

theorem convex_sgnK_sub (c : ℝ) (s : Sgn) : Convex ℝ {u : ℝ | sgnK (u - c) = s} := by
  intro p hp q hq a b ha hb hab
  show sgnK (a • p + b • q - c) = s
  have : a • p + b • q - c = a • (p - c) + b • (q - c) := by
    simp only [smul_eq_mul]; linear_combination c * hab
  rw [this]
  exact convex_sgnK s hp hq ha hb hab

/-- A sign class over the base `D`. -/
def graphCell {m : Nat} (a : Fin m → ℝ × ℝ → ℝ) (D : Set (ℝ × ℝ)) (δ : ℝ) (σ : Fin m → Sgn) :
    Set R3 := {p | p.2 ∈ D ∧ |p.1| < δ ∧ ∀ k, sgnK (p.1 - a k p.2) = σ k}

/-- **Every sign class of ordered graphs is connected.** -/
theorem graphCell_preconnected {m : Nat} {a : Fin m → ℝ × ℝ → ℝ} (ha : ∀ k, Continuous (a k))
    {D : Set (ℝ × ℝ)} (hD : IsPreconnected D) {δ : ℝ} (hδ : ∀ k, ∀ d ∈ D, |a k d| < δ)
    (hord : ∀ k l, ∀ d ∈ D, ∀ d' ∈ D, sgnK (a k d - a l d) = sgnK (a k d' - a l d'))
    (σ : Fin m → Sgn) : IsPreconnected (graphCell a D δ σ) := by
  rcases (graphCell a D δ σ).eq_empty_or_nonempty with he | ⟨p0, hp0D, hp0δ, hp0σ⟩
  · rw [he]; exact isPreconnected_empty
  set d0 := p0.2
  -- a continuous section through the class
  obtain ⟨s, hs, hsD⟩ : ∃ s : ℝ × ℝ → ℝ, Continuous s ∧ ∀ d ∈ D,
      |s d| < δ ∧ ∀ k, sgnK (s d - a k d) = σ k := by
    by_cases hc : ∃ c, σ c = .zero
    · obtain ⟨c, hc⟩ := hc
      have hp0c : p0.1 = a c d0 := by
        have := hp0σ c; rw [hc] at this; linarith [sgnK_zero_iff.mp this]
      refine ⟨a c, ha c, fun d hd => ⟨hδ c d hd, fun k => ?_⟩⟩
      rw [hord c k d hd d0 hp0D, ← hp0c]
      exact hp0σ k
    · push Not at hc
      set lpos := (List.finRange m).filter fun k => σ k = .pos
      set lneg := (List.finRange m).filter fun k => σ k = .neg
      set lo := fun d => lmax (fun k => a k d) (-δ) lpos
      set hi := fun d => lmin (fun k => a k d) δ lneg
      have hpos : ∀ k, σ k = .pos → ∀ d ∈ D, ∀ l, σ l = .neg → a k d < a l d := by
        intro k hk d hd l hl
        have h1 := hp0σ k; have h2 := hp0σ l
        rw [hk] at h1; rw [hl] at h2
        have e1 : a k d0 < p0.1 := by
          rcases lt_trichotomy (p0.1 - a k d0) 0 with h | h | h
          · rw [sgnK_neg h] at h1; cases h1
          · rw [h] at h1; simp [sgnK] at h1
          · linarith
        have e2 : p0.1 < a l d0 := by
          rcases lt_trichotomy (p0.1 - a l d0) 0 with h | h | h
          · linarith
          · rw [h] at h2; simp [sgnK] at h2
          · rw [sgnK_pos h] at h2; cases h2
        have : sgnK (a k d0 - a l d0) = .neg := sgnK_neg (by linarith)
        rw [← hord k l d hd d0 hp0D] at this
        rcases lt_trichotomy (a k d - a l d) 0 with h | h | h
        · linarith
        · rw [h] at this; simp [sgnK] at this
        · rw [sgnK_pos h] at this; cases this
      have hδ0 : 0 < δ := lt_of_le_of_lt (abs_nonneg _) hp0δ
      have mpos : ∀ k, k ∈ lpos ↔ σ k = .pos := fun k => by simp [lpos]
      have mneg : ∀ k, k ∈ lneg ↔ σ k = .neg := fun k => by simp [lneg]
      have hlohi : ∀ d ∈ D, lo d < hi d := by
        intro d hd
        refine lmax_lt (lt_lmin (by linarith) fun l _ => ?_) fun k hk => ?_
        · have := hδ l d hd; rw [abs_lt] at this; exact this.1
        · refine lt_lmin ((abs_lt.mp (hδ k d hd)).2) fun l hl => ?_
          exact hpos k ((mpos k).mp hk) d hd l ((mneg l).mp hl)
      refine ⟨fun d => (lo d + hi d) / 2,
        ((continuous_lmax ha (-δ) lpos).add (continuous_lmin ha δ lneg)).div_const 2,
        fun d hd => ⟨?_, fun k => ?_⟩⟩
      · have h1 := (le_lmax (f := fun k => a k d) (b := -δ) (l := lpos)).1
        have h2 := (lmin_le (f := fun k => a k d) (b := δ) (l := lneg)).1
        have h3 := hlohi d hd
        rw [abs_lt]; constructor <;> simp only [lo, hi] at h3 ⊢ <;> linarith
      · have h3 := hlohi d hd
        rcases hk : σ k with _ | _ | _
        · have h2 := (lmin_le (f := fun k => a k d) (b := δ) (l := lneg)).2 k ((mneg k).mpr hk)
          exact sgnK_neg (by simp only [lo, hi] at h3 h2 ⊢; linarith)
        · exact absurd hk (hc k)
        · have h1 := (le_lmax (f := fun k => a k d) (b := -δ) (l := lpos)).2 k ((mpos k).mpr hk)
          exact sgnK_pos (by simp only [lo, hi] at h3 h1 ⊢; linarith)
  -- fibres are intervals, joined through the section
  have hF : ∀ d, Convex ℝ {u : ℝ | |u| < δ ∧ ∀ k, sgnK (u - a k d) = σ k} := by
    intro d
    have : {u : ℝ | |u| < δ ∧ ∀ k, sgnK (u - a k d) = σ k} =
        Ioo (-δ) δ ∩ ⋂ k, {u : ℝ | sgnK (u - a k d) = σ k} := by
      ext u; simp [abs_lt]
    rw [this]
    exact (convex_Ioo _ _).inter (convex_iInter fun k => convex_sgnK_sub _ _)
  have hsec : IsPreconnected ((fun d => (s d, d)) '' D) :=
    hD.image _ ((hs.prodMk continuous_id).continuousOn)
  refine isPreconnected_of_forall (s d0, d0) fun y hy => ?_
  obtain ⟨hyD, hyδ, hyσ⟩ := hy
  set fib := (fun u : ℝ => ((u, y.2) : R3)) '' {u : ℝ | |u| < δ ∧ ∀ k, sgnK (u - a k y.2) = σ k}
  refine ⟨fib ∪ (fun d => (s d, d)) '' D, ?_, Or.inr ⟨d0, hp0D, rfl⟩,
    Or.inl ⟨y.1, ⟨hyδ, hyσ⟩, rfl⟩, ?_⟩
  · rintro q (⟨u, ⟨hu1, hu2⟩, rfl⟩ | ⟨d, hd, rfl⟩)
    · exact ⟨hyD, hu1, hu2⟩
    · exact ⟨hd, (hsD d hd).1, (hsD d hd).2⟩
  · refine IsPreconnected.union (s y.2, y.2) ⟨s y.2, (hsD y.2 hyD), rfl⟩ ⟨y.2, hyD, rfl⟩
      ((hF y.2).isPreconnected.image _ (by fun_prop : Continuous fun u : ℝ => ((u, y.2) : R3)).continuousOn)
      hsec

/-! ## A point modelled on ordered graphs -/

/-- The model's functions: the seam (`none`) and the graphs `u - aₖ(w, z)`. -/
def mval {m : Nat} (a : Fin m → ℝ × ℝ → ℝ) : Option (Fin m) → R3 → ℝ
  | none => fun p => p.2.1
  | some k => fun p => p.1 - a k p.2

/-- A sign, flipped or not. -/
def fS : Bool → Sgn → Sgn
  | false, s => s
  | true, .pos => .neg
  | true, .neg => .pos
  | true, .zero => .zero

theorem fS_fS (b : Bool) (s : Sgn) : fS b (fS b s) = s := by
  cases b <;> cases s <;> rfl

/-- **A contact point**: through a homeomorphism `T` sending `x` to the origin, near `x`, each
carrier through `x` has the sign (flipped or not) of the seam or of one of the graphs; every one of them is
used; the graphs are continuous and pass through the origin; and on each side of the seam, and on
it, near the origin, they keep their order. -/
structure ContactAt (cs : List (Carrier R3 ℝ)) (x : R3) where
  T : R3 ≃ₜ R3
  hT : T x = 0
  m : Nat
  a : Fin m → ℝ × ℝ → ℝ
  ha : ∀ k, Continuous (a k)
  ha0 : ∀ k, a k 0 = 0
  kind : Fin cs.length → Option (Fin m)
  fl : Fin cs.length → Bool
  /-- the neighbourhood of `x` where the model holds -/
  V : Set R3
  hV : IsOpen V
  hxV : x ∈ V
  hm : ∀ i : Fin cs.length, cs[i] x = 0 → ∀ y ∈ V,
    sgnK (cs[i] y) = fS (fl i) (sgnK (mval a (kind i) (T y)))
  used : ∀ o, ∃ i : Fin cs.length, cs[i] x = 0 ∧ kind i = o
  δ₀ : ℝ
  hδ₀ : 0 < δ₀
  ord : ∀ k l, ∀ d d' : ℝ × ℝ, ‖d‖ < δ₀ → ‖d'‖ < δ₀ → sgnK d.1 = sgnK d'.1 →
    sgnK (a k d - a l d) = sgnK (a k d' - a l d')

/-- **Local connectivity at a contact point.** -/
theorem localConn_of_contact {cs : List (Carrier R3 ℝ)} (hc : ∀ c ∈ cs, Continuous c) {x : R3}
    (h : ContactAt cs x) : FaceLocal.LocalConn cs x := by
  classical
  intro W hW hxW
  obtain ⟨N, hNo, hxN, hN⟩ := exists_open_le hc x
  -- a box in model coordinates inside `T (W ∩ N)`
  have hO : IsOpen (h.T.symm ⁻¹' (W ∩ N ∩ h.V)) :=
    ((hW.inter hNo).inter h.hV).preimage h.T.symm.continuous
  have h0O : (0 : R3) ∈ h.T.symm ⁻¹' (W ∩ N ∩ h.V) := by
    rw [mem_preimage, ← h.hT, h.T.symm_apply_apply]; exact ⟨⟨hxW, hxN⟩, h.hxV⟩
  obtain ⟨δ1, hδ1, hball⟩ := Metric.isOpen_iff.mp hO 0 h0O
  set δ := min δ1 h.δ₀
  have hδ : 0 < δ := lt_min hδ1 h.hδ₀
  have hev : ∀ᶠ d in 𝓝 (0 : ℝ × ℝ), ∀ k, |h.a k d| < δ := by
    refine Filter.eventually_all.mpr fun k => ?_
    exact (continuous_abs.comp (h.ha k)).continuousAt.eventually_lt continuousAt_const
      (by simp [h.ha0 k, hδ])
  obtain ⟨η1, hη1, hη⟩ := Metric.eventually_nhds_iff.mp hev
  set η := min η1 δ
  have hηpos : 0 < η := lt_min hη1 hδ
  set Um : Set R3 := {p | |p.1| < δ ∧ ‖p.2‖ < η}
  have hUm : IsOpen Um :=
    (isOpen_lt (continuous_abs.comp continuous_fst) continuous_const).inter
      (isOpen_lt (continuous_norm.comp continuous_snd) continuous_const)
  have hUmW : ∀ p ∈ Um, h.T.symm p ∈ W ∩ N ∩ h.V := by
    intro p ⟨h1, h2⟩
    refine hball ?_
    rw [Metric.mem_ball, dist_zero_right, Prod.norm_def]
    refine max_lt (lt_of_lt_of_le (show ‖p.1‖ < δ by rw [Real.norm_eq_abs]; exact h1) (min_le_left _ _)) ?_
    exact lt_of_lt_of_le h2 ((min_le_right _ _).trans (min_le_left _ _))
  refine ⟨h.T ⁻¹' Um, hUm.preimage h.T.continuous, fun y hy => ?_, ?_, fun Y hY => ?_⟩
  · have := hUmW _ hy; rw [h.T.symm_apply_apply] at this; exact this.1.1
  · show h.T x ∈ Um
    rw [h.hT]; exact ⟨by simp [hδ], by simp [hηpos]⟩
  -- the cell near `x` is the model's
  obtain ⟨hlen, -⟩ := le_iff_getElem.mp hY
  rw [length_covOfK] at hlen
  choose rep hrep using h.used
  set σ' : Option (Fin h.m) → Sgn := fun o => fS (h.fl (rep o)) (Y[(rep o).1]'(by rw [← hlen]; exact (rep o).2))
  set D : Set (ℝ × ℝ) := {d | ‖d‖ < η ∧ sgnK d.1 = σ' none}
  have hD : IsPreconnected D := by
    refine Convex.isPreconnected ?_
    have : D = Metric.ball 0 η ∩ (LinearMap.fst ℝ ℝ ℝ) ⁻¹' {r | sgnK r = σ' none} := by
      ext d; simp [D]
    rw [this]
    exact (convex_ball _ _).inter ((convex_sgnK _).linear_preimage _)
  have hcell : IsPreconnected (graphCell h.a D δ fun k => σ' (some k)) := by
    refine graphCell_preconnected h.ha hD (fun k d hd => hη (by
      rw [dist_zero_right]; exact lt_of_lt_of_le hd.1 (min_le_left _ _)) k) ?_ _
    intro k l d hd d' hd'
    refine h.ord k l d d' (lt_of_lt_of_le hd.1 ((min_le_right _ _).trans (min_le_right _ _)))
      (lt_of_lt_of_le hd'.1 ((min_le_right _ _).trans (min_le_right _ _))) ?_
    rw [hd.2, hd'.2]
  by_cases hcons : ∀ i : Fin cs.length, cs[i] x = 0 →
      fS (h.fl i) (Y[i.1]'(by rw [← hlen]; exact i.2)) = σ' (h.kind i)
  · have he : cell cs Y ∩ h.T ⁻¹' Um = h.T ⁻¹' graphCell h.a D δ (fun k => σ' (some k)) := by
      ext y
      have hyN : y ∈ h.T ⁻¹' Um → y ∈ N := fun hy => by
        have := (hUmW _ hy).1.2; rwa [h.T.symm_apply_apply] at this
      have hyV : y ∈ h.T ⁻¹' Um → y ∈ h.V := fun hy => by
        have := (hUmW _ hy).2; rwa [h.T.symm_apply_apply] at this
      constructor
      · rintro ⟨hyY, hyU⟩
        have hK := (cell_iff_coneCell hN hY (hyN hyU)).mp hyY
        -- the model signs at `T y`
        have hsg : ∀ o, sgnK (mval h.a o (h.T y)) = σ' o := by
          intro o
          have e := h.hm (rep o) (hrep o).1 y (hyV hyU)
          have hk := hK (rep o).1 (rep o).2 (by rw [← hlen]; exact (rep o).2) (hrep o).1
          simp only [Fin.getElem_fin] at e
          rw [hk, (hrep o).2] at e
          simp only [σ']
          rw [e, fS_fS]
        refine ⟨⟨hyU.2, ?_⟩, hyU.1, fun k => hsg (some k)⟩
        exact hsg none
      · rintro ⟨⟨hd1, hd2⟩, hu, hk⟩
        have hyU : y ∈ h.T ⁻¹' Um := ⟨hu, hd1⟩
        refine ⟨(cell_iff_coneCell hN hY (hyN hyU)).mpr ?_, hyU⟩
        intro i hi hi' hx
        have e := h.hm ⟨i, hi⟩ hx y (hyV hyU)
        simp only [Fin.getElem_fin] at e
        rw [e]
        have hmv : sgnK (mval h.a (h.kind ⟨i, hi⟩) (h.T y)) = σ' (h.kind ⟨i, hi⟩) := by
          rcases hmk : h.kind ⟨i, hi⟩ with _ | k
          · exact hd2
          · exact hk k
        rw [hmv, ← hcons ⟨i, hi⟩ hx, fS_fS]
    rw [he, ← h.T.image_symm]
    exact hcell.image _ h.T.symm.continuous.continuousOn
  · push Not at hcons
    obtain ⟨i, hx, hne⟩ := hcons
    have : cell cs Y ∩ h.T ⁻¹' Um = ∅ := by
      ext y
      simp only [mem_inter_iff, mem_empty_iff_false, iff_false, not_and]
      intro hyY hyU
      have hyV : y ∈ h.V := by
        have := (hUmW _ hyU).2; rwa [h.T.symm_apply_apply] at this
      apply hne
      have hy' : covOfK cs y = Y := hyY
      have hYi : Y[i.1]'(by rw [← hlen]; exact i.2) = sgnK (cs[i] y) := by
        have : (covOfK cs y)[i.1]'(by rw [length_covOfK]; exact i.2) = Y[i.1]'(by rw [← hlen]; exact i.2) := by
          simp only [hy']
        rw [getElem_covOfK] at this; exact this.symm
      set o := h.kind i
      have hr : Y[(rep o).1]'(by rw [← hlen]; exact (rep o).2) = sgnK (cs[rep o] y) := by
        have : (covOfK cs y)[(rep o).1]'(by rw [length_covOfK]; exact (rep o).2) =
            Y[(rep o).1]'(by rw [← hlen]; exact (rep o).2) := by simp only [hy']
        rw [getElem_covOfK] at this; exact this.symm
      simp only [σ']
      have e1 := h.hm i hx y hyV
      have e2 := h.hm (rep o) (hrep o).1 y hyV
      simp only [Fin.getElem_fin] at e1 e2 hYi hr
      rw [hYi, hr, e1, e2, (hrep o).2, fS_fS, fS_fS]
    rw [this]; exact isPreconnected_empty

/-! ## The tangent cylinders are a contact point -/

/-- Two cylinders tangent along the `z` axis, and their contact seam `y = 0`. -/
def tangentPair : List (Carrier R3 ℝ) :=
  [fun p => (p.1 - 1) ^ 2 + p.2.1 ^ 2 - 1, fun p => (p.1 + 1) ^ 2 + p.2.1 ^ 2 - 1, fun p => p.2.1]

theorem sgnK_mul_neg {a b : ℝ} (hb : b < 0) : sgnK (a * b) = fS true (sgnK a) := by
  rcases lt_trichotomy a 0 with h | h | h
  · rw [sgnK_neg h, sgnK_pos (mul_pos_of_neg_of_neg h hb)]; rfl
  · rw [h, zero_mul]; simp [sgnK, fS]
  · rw [sgnK_pos h, sgnK_neg (mul_neg_of_pos_of_neg h hb)]; rfl

theorem sgnK_mul_pos' {a b : ℝ} (hb : 0 < b) : sgnK (a * b) = sgnK a := by
  rw [mul_comm]; exact sgnK_mul_pos hb a

/-- The two graphs: `u = ±(1 - √(1 - w²))`. -/
noncomputable def tgA : Fin 2 → ℝ × ℝ → ℝ
  | 0 => fun d => 1 - Real.sqrt (1 - d.1 ^ 2)
  | 1 => fun d => Real.sqrt (1 - d.1 ^ 2) - 1

/-- **Every point of the contact line is a contact point** (non-vacuity of `ContactAt`). -/
noncomputable def tangentContact (z₀ : ℝ) : ContactAt tangentPair (0, 0, z₀) where
  T := Homeomorph.addRight (0, 0, -z₀)
  hT := by simp
  m := 2
  a := tgA
  ha k := by fin_cases k <;> (simp only [tgA]; fun_prop)
  ha0 k := by fin_cases k <;> simp [tgA]
  kind i := if i.1 = 0 then some 0 else if i.1 = 1 then some 1 else none
  fl i := i.1 = 0
  V := {p | |p.1| < 1 / 2 ∧ |p.2.1| < 1 / 2}
  hV := (isOpen_lt (continuous_abs.comp continuous_fst) continuous_const).inter
    (isOpen_lt (continuous_abs.comp (continuous_fst.comp continuous_snd)) continuous_const)
  hxV := by norm_num
  hm i _ y hy := by
    obtain ⟨h1, h2⟩ := hy
    have hy2 : y.2.1 ^ 2 ≤ 1 := by nlinarith [abs_lt.mp h2, sq_abs y.2.1]
    set s := Real.sqrt (1 - y.2.1 ^ 2)
    have hs : s ^ 2 = 1 - y.2.1 ^ 2 := Real.sq_sqrt (by linarith)
    have hs0 : 0 ≤ s := Real.sqrt_nonneg _
    have hx := abs_lt.mp h1
    fin_cases i
    · have : (y.1 - 1) ^ 2 + y.2.1 ^ 2 - 1 = (y.1 - (1 - s)) * (y.1 - (1 + s)) := by
        nlinarith [hs]
      show sgnK ((y.1 - 1) ^ 2 + y.2.1 ^ 2 - 1) =
        fS true (sgnK ((y + (0, 0, -z₀)).1 - (1 - √(1 - (y + (0, 0, -z₀)).2.1 ^ 2))))
      simp only [Prod.fst_add, Prod.snd_add, add_zero]
      rw [this, sgnK_mul_neg (by linarith)]
    · have : (y.1 + 1) ^ 2 + y.2.1 ^ 2 - 1 = (y.1 - (s - 1)) * (y.1 + 1 + s) := by
        nlinarith [hs]
      show sgnK ((y.1 + 1) ^ 2 + y.2.1 ^ 2 - 1) =
        fS false (sgnK ((y + (0, 0, -z₀)).1 - (√(1 - (y + (0, 0, -z₀)).2.1 ^ 2) - 1)))
      simp only [Prod.fst_add, Prod.snd_add, add_zero]
      rw [this, sgnK_mul_pos' (by linarith)]
      rfl
    · show sgnK y.2.1 = fS false (sgnK (y.2.1 + 0))
      rw [add_zero]; rfl
  used o := by
    rcases o with _ | k
    · exact ⟨⟨2, by simp [tangentPair]⟩, show (0 : ℝ) = 0 from rfl, by simp⟩
    · fin_cases k
      · exact ⟨⟨0, by simp [tangentPair]⟩,
          show ((0 : ℝ) - 1) ^ 2 + (0 : ℝ) ^ 2 - 1 = 0 by norm_num, by simp⟩
      · exact ⟨⟨1, by simp [tangentPair]⟩,
          show ((0 : ℝ) + 1) ^ 2 + (0 : ℝ) ^ 2 - 1 = 0 by norm_num, by simp⟩
  δ₀ := 1 / 2
  hδ₀ := by norm_num
  ord k l d d' hd hd' hs := by
    have hg : ∀ e : ℝ × ℝ, ‖e‖ < 1 / 2 → sgnK (1 - Real.sqrt (1 - e.1 ^ 2)) = sq (sgnK e.1) := by
      intro e he
      have h1 : |e.1| < 1 / 2 := by
        have := (norm_fst_le e).trans_lt he; rwa [Real.norm_eq_abs] at this
      rcases eq_or_ne e.1 0 with h0 | h0
      · simp [h0, sgnK, sq]
      · have hp : 0 < e.1 ^ 2 := by positivity
        have hlt : Real.sqrt (1 - e.1 ^ 2) < 1 := by
          rw [Real.sqrt_lt' one_pos]; linarith
        rw [sgnK_pos (by linarith)]
        rcases lt_or_gt_of_ne h0 with h | h
        · rw [sgnK_neg h]; rfl
        · rw [sgnK_pos h]; rfl
    have e01 : ∀ e : ℝ × ℝ, tgA 0 e - tgA 1 e = (1 - Real.sqrt (1 - e.1 ^ 2)) * 2 := by
      intro e; simp only [tgA]; ring
    have e10 : ∀ e : ℝ × ℝ, tgA 1 e - tgA 0 e = (1 - Real.sqrt (1 - e.1 ^ 2)) * (-2) := by
      intro e; simp only [tgA]; ring
    fin_cases k <;> fin_cases l
    · simp
    · simp only [Fin.zero_eta, Fin.isValue, Fin.mk_one]
      rw [e01, e01, sgnK_mul_pos' two_pos, sgnK_mul_pos' two_pos, hg d hd, hg d' hd', hs]
    · simp only [Fin.zero_eta, Fin.isValue, Fin.mk_one]
      rw [e10, e10, sgnK_mul_neg (by norm_num), sgnK_mul_neg (by norm_num), hg d hd, hg d' hd', hs]
    · simp

/-- **Local connectivity on the contact line of two tangent cylinders**, with the contact seam. -/
theorem tangentPair_localConn (z₀ : ℝ) : FaceLocal.LocalConn tangentPair (0, 0, z₀) :=
  localConn_of_contact (fun c hc => by
    simp only [tangentPair, List.mem_cons, List.not_mem_nil, or_false] at hc
    rcases hc with rfl | rfl | rfl <;> fun_prop) (tangentContact z₀)

/-! ## Cylinder–plane line contact -/

/-- The cylinder `x² + (y - 1)² = 1`, tangent to the plane `y = 0` along the `z` axis, with the
contact seam `x = 0`. -/
def cylPlane : List (Carrier R3 ℝ) :=
  [fun p => p.1 ^ 2 + (p.2.1 - 1) ^ 2 - 1, fun p => p.2.1, fun p => p.1]

/-- Swap `x` and `y`, and move `z₀` to `0`. -/
def swapXY (z₀ : ℝ) : R3 ≃ₜ R3 where
  toFun p := (p.2.1, p.1, p.2.2 + -z₀)
  invFun q := (q.2.1, q.1, q.2.2 + z₀)
  left_inv p := by simp
  right_inv q := by simp
  continuous_toFun := by fun_prop
  continuous_invFun := by fun_prop

/-- The graphs: the cylinder `u = 1 - √(1 - w²)` and the plane `u = 0`. -/
noncomputable def cpA : Fin 2 → ℝ × ℝ → ℝ
  | 0 => fun d => 1 - Real.sqrt (1 - d.1 ^ 2)
  | 1 => fun _ => 0

theorem sgnK_one_sub_sqrt (e : ℝ × ℝ) (he : ‖e‖ < 1 / 2) :
    sgnK (1 - Real.sqrt (1 - e.1 ^ 2)) = sq (sgnK e.1) := by
  have h1 : |e.1| < 1 / 2 := by
    have := (norm_fst_le e).trans_lt he; rwa [Real.norm_eq_abs] at this
  rcases eq_or_ne e.1 0 with h0 | h0
  · simp [h0, sgnK, sq]
  · have hp : 0 < e.1 ^ 2 := by positivity
    have hlt : Real.sqrt (1 - e.1 ^ 2) < 1 := by
      rw [Real.sqrt_lt' one_pos]; linarith
    rw [sgnK_pos (by linarith)]
    rcases lt_or_gt_of_ne h0 with h | h
    · rw [sgnK_neg h]; rfl
    · rw [sgnK_pos h]; rfl

/-- **Every point of the contact line of a cylinder and its tangent plane is a contact point.** -/
noncomputable def cylPlaneContact (z₀ : ℝ) : ContactAt cylPlane (0, 0, z₀) where
  T := swapXY z₀
  hT := by simp [swapXY]
  m := 2
  a := cpA
  ha k := by fin_cases k <;> (simp only [cpA]; fun_prop)
  ha0 k := by fin_cases k <;> simp [cpA]
  kind i := if i.1 = 0 then some 0 else if i.1 = 1 then some 1 else none
  fl i := i.1 = 0
  V := {p | |p.1| < 1 / 2 ∧ |p.2.1| < 1 / 2}
  hV := (isOpen_lt (continuous_abs.comp continuous_fst) continuous_const).inter
    (isOpen_lt (continuous_abs.comp (continuous_fst.comp continuous_snd)) continuous_const)
  hxV := by norm_num
  hm i _ y hy := by
    obtain ⟨h1, h2⟩ := hy
    have hx2 : y.1 ^ 2 ≤ 1 := by nlinarith [abs_lt.mp h1, sq_abs y.1]
    set s := Real.sqrt (1 - y.1 ^ 2)
    have hs : s ^ 2 = 1 - y.1 ^ 2 := Real.sq_sqrt (by linarith)
    have hs0 : 0 ≤ s := Real.sqrt_nonneg _
    have hy' := abs_lt.mp h2
    fin_cases i
    · have : y.1 ^ 2 + (y.2.1 - 1) ^ 2 - 1 = (y.2.1 - (1 - s)) * (y.2.1 - (1 + s)) := by
        nlinarith [hs]
      show sgnK (y.1 ^ 2 + (y.2.1 - 1) ^ 2 - 1) = fS true (sgnK (y.2.1 - (1 - √(1 - y.1 ^ 2))))
      rw [this, sgnK_mul_neg (by linarith)]
    · show sgnK y.2.1 = fS false (sgnK (y.2.1 - 0))
      rw [sub_zero]; rfl
    · show sgnK y.1 = fS false (sgnK y.1)
      rfl
  used o := by
    rcases o with _ | k
    · exact ⟨⟨2, by simp [cylPlane]⟩, show (0 : ℝ) = 0 from rfl, by simp⟩
    · fin_cases k
      · exact ⟨⟨0, by simp [cylPlane]⟩,
          show (0 : ℝ) ^ 2 + ((0 : ℝ) - 1) ^ 2 - 1 = 0 by norm_num, by simp⟩
      · exact ⟨⟨1, by simp [cylPlane]⟩, show (0 : ℝ) = 0 from rfl, by simp⟩
  δ₀ := 1 / 2
  hδ₀ := by norm_num
  ord k l d d' hd hd' hs := by
    have e01 : ∀ e : ℝ × ℝ, cpA 0 e - cpA 1 e = (1 - Real.sqrt (1 - e.1 ^ 2)) * 1 := by
      intro e; simp only [cpA]; ring
    have e10 : ∀ e : ℝ × ℝ, cpA 1 e - cpA 0 e = (1 - Real.sqrt (1 - e.1 ^ 2)) * (-1) := by
      intro e; simp only [cpA]; ring
    fin_cases k <;> fin_cases l
    · simp
    · simp only [Fin.zero_eta, Fin.isValue, Fin.mk_one]
      rw [e01, e01, sgnK_mul_pos' one_pos, sgnK_mul_pos' one_pos, sgnK_one_sub_sqrt d hd,
        sgnK_one_sub_sqrt d' hd', hs]
    · simp only [Fin.zero_eta, Fin.isValue, Fin.mk_one]
      rw [e10, e10, sgnK_mul_neg (by norm_num), sgnK_mul_neg (by norm_num), sgnK_one_sub_sqrt d hd,
        sgnK_one_sub_sqrt d' hd', hs]
    · simp

/-- **Local connectivity on the contact line of a cylinder and a tangent plane.** -/
theorem cylPlane_localConn (z₀ : ℝ) : FaceLocal.LocalConn cylPlane (0, 0, z₀) :=
  localConn_of_contact (fun c hc => by
    simp only [cylPlane, List.mem_cons, List.not_mem_nil, or_false] at hc
    rcases hc with rfl | rfl | rfl <;> fun_prop) (cylPlaneContact z₀)

/-! ## Which tangencies decide-once takes (the master's ruling, 2026-11-23) -/

/-- The kinds of tangential contact between two near faces. -/
inductive TangentKind
  /-- a sphere touching a plane at one point -/
  | spherePlanePoint
  /-- a cylinder touching a plane along a line -/
  | cylPlaneLine
  /-- coaxial equal radii: surfaces of revolution about one axis touching along a circle -/
  | coaxialEqualRadii
  /-- two cylinders with parallel axes touching along a line: the union is non-manifold there,
  and the deliverable is the refusal (four wings at the contact edge) -/
  | cylCylLine
  /-- anything else (sphere–sphere, sphere–cylinder off axis, crossed cylinders touching at a
  point, cone tangencies, internal tangencies of unequal surfaces, higher contact) -/
  | other
  deriving DecidableEq, Repr

/-- **Taken by decide-once**: the three manifold kinds with local models, and the refused
tangent-cylinder line. Everything else is a counted `Skip` (the old path handles it, as with
`Skip::Wide`). -/
def TangentKind.taken : TangentKind → Bool
  | .other => false
  | _ => true

/-- Which of the taken kinds have a local model on this branch: `cylPlaneLine`
(`cylPlane_localConn`) and, as the seamed reference picture, `cylCylLine`
(`tangentPair_localConn`; refusal in TangentRefusal.lean), and `coaxialEqualRadii`
(`coaxial_localConn`, CoaxialContact.lean). `spherePlanePoint` waits on FaceLocal sectors without
bounding edges. -/
def TangentKind.modelled : TangentKind → Bool
  | .cylPlaneLine => true
  | .cylCylLine => true
  | .coaxialEqualRadii => true
  | _ => false

end ForgeCurved
