import ForgeCurved.Thom
import ForgeCurved.Sides

/-!
# R1 for a cylindrical arrangement, one fibre direction at a time

The space is `T × ℝ`: a base `T` and a fibre coordinate `s`. Each carrier is either a **base**
carrier (constant along the fibres: a level-2 cylinder or a level-1 wall) or a **fibre** carrier.
`cyl_R1` proves that every sign class of the whole list is preconnected, from:
* (H1) every sign class of the base carriers, `BaseCell`, is preconnected (the level below);
* (H2) `FibreOI` over each base cell: the fibre carriers' zeros over the cell are fixed sets of
  continuous, strictly ordered sections `ρ 0 < … < ρ (m-1)`, or a carrier vanishes on the
  whole cylinder over the cell. This is delineability with order invariance. It follows from the
  projection polynomials' sign invariance on the cell; see `Thom.merge` for the assembly;
* (H3) Thom: in every fibre, each sign condition on the fibre carriers is an interval
  (`Thom.thom` for a family closed under `∂_s`).

The proof picks a continuous representative `σ` over the base cell, a section or the midpoint of a
sector. Every fibre carrier keeps its sign along `σ` (`Thom.sgnK_const`), so the graph of `σ`
lies in the sign class. Each fibre of the class is an interval that meets the graph, and the class
is the union of these, so it is preconnected.

Applied twice: once over the `t`-line with the walls (base) and the level-2 family `B` (fibre
`r`), then over the `(t, r)`-plane with walls and `B` (base) and the carriers, seams and polar
planes (fibre `s`, along `u`). This gives design §2.3's R1 for Tier 1.
-/

namespace ForgeCurved.CAD

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Set ForgeCurved.Thom

theorem covOfK_eq_iff {Y : Type} (cs : List (Y → ℝ)) (y : Y) (X : Cov) :
    covOfK cs y = X ↔ X.length = cs.length ∧
      ∀ i (hi : i < cs.length), X[i]? = some (sgnK (cs[i] y)) := by
  constructor
  · rintro rfl
    refine ⟨by simp [covOfK], fun i hi => ?_⟩
    rw [ForgeCurved.getElem?_covOfK]; simp [hi]
  · rintro ⟨hlen, h⟩
    apply List.ext_getElem?
    intro i
    rw [ForgeCurved.getElem?_covOfK]
    by_cases hi : i < cs.length
    · rw [h i hi]; simp [hi]
    · have : X.length ≤ i := by omega
      rw [List.getElem?_eq_none this]; simp [List.getElem?_eq_none (not_lt.mp hi)]

variable {T : Type} [TopologicalSpace T]

/-- The sign condition of `X` on the carriers with index in `J`. -/
def SignOn {Y : Type} (cs : List (Y → ℝ)) (J : ℕ → Prop) (X : Cov) : Set Y :=
  {y | ∀ i (hi : i < cs.length), J i → X[i]? = some (sgnK (cs[i] y))}

theorem signOn_univ {Y : Type} (cs : List (Y → ℝ)) (X : Cov) :
    {y | covOfK cs y = X} = {_y | X.length = cs.length} ∩ SignOn cs (fun _ => True) X := by
  ext y
  simp only [mem_inter_iff, mem_ofPred_eq, SignOn, covOfK_eq_iff, forall_const]

/-- The base cell of `X`: the signs of the base carriers in `J`. -/
def BaseCell (cs : List (T × ℝ → ℝ)) (J isBase : ℕ → Prop) (X : Cov) : Set T :=
  {t | ∀ i (hi : i < cs.length), J i → isBase i → X[i]? = some (sgnK (cs[i] (t, 0)))}

/-- The fibre of `X` over `t`: the signs of the fibre carriers in `J`. -/
def FibreSet (cs : List (T × ℝ → ℝ)) (J isBase : ℕ → Prop) (X : Cov) (t : T) : Set ℝ :=
  {s | ∀ i (hi : i < cs.length), J i → ¬isBase i → X[i]? = some (sgnK (cs[i] (t, s)))}

/-- Delineability with order invariance over `C`: the zeros of the fibre carriers in `J` are fixed
sets of continuous, strictly ordered sections, or the carrier vanishes over all of `C`. -/
def FibreOI (cs : List (T × ℝ → ℝ)) (J isBase : ℕ → Prop) (C : Set T) : Prop :=
  ∃ m, ∃ ρ : Fin m → T → ℝ, (∀ k, ContinuousOn (ρ k) C) ∧ (∀ t ∈ C, StrictMono fun k => ρ k t) ∧
    ∀ i (hi : i < cs.length), J i → ¬isBase i →
      (∀ t ∈ C, ∀ s, cs[i] (t, s) = 0) ∨
        ∃ S : Finset (Fin m), ∀ t ∈ C, ∀ s, cs[i] (t, s) = 0 ↔ ∃ k ∈ S, ρ k t = s

/-- A continuous point of every sector: below the sections in a down-closed `P`, above the rest. -/
theorem sector_rep {C : Set T} {m : ℕ} (ρ : Fin m → T → ℝ) (hρc : ∀ k, ContinuousOn (ρ k) C)
    (hρm : ∀ t ∈ C, StrictMono fun k => ρ k t) (P : Fin m → Prop) [DecidablePred P]
    (hP : ∀ a b, a ≤ b → P b → P a) :
    ∃ σ : T → ℝ, ContinuousOn σ C ∧ ∀ t ∈ C, ∀ k, (P k → ρ k t < σ t) ∧ (¬P k → σ t < ρ k t) := by
  classical
  set A := Finset.univ.filter P
  set B := Finset.univ.filter fun k => ¬P k
  by_cases hA : A.Nonempty <;> by_cases hB : B.Nonempty
  · set a := A.max' hA
    set b := B.min' hB
    have haP : P a := (Finset.mem_filter.mp (A.max'_mem hA)).2
    have hbP : ¬P b := (Finset.mem_filter.mp (B.min'_mem hB)).2
    have hab : a < b := by
      by_contra h
      exact hbP (hP b a (not_lt.mp h) haP)
    refine ⟨fun t => (ρ a t + ρ b t) / 2, ((hρc a).add (hρc b)).div_const 2, fun t ht k => ⟨?_, ?_⟩⟩
    · intro hk
      have hka : k ≤ a := A.le_max' k (Finset.mem_filter.mpr ⟨Finset.mem_univ _, hk⟩)
      have h1 := (hρm t ht).monotone hka
      have h2 := hρm t ht hab
      simp only; linarith
    · intro hk
      have hbk : b ≤ k := B.min'_le k (Finset.mem_filter.mpr ⟨Finset.mem_univ _, hk⟩)
      have h1 := (hρm t ht).monotone hbk
      have h2 := hρm t ht hab
      simp only; linarith
  · set a := A.max' hA
    refine ⟨fun t => ρ a t + 1, (hρc a).add continuousOn_const, fun t ht k => ⟨?_, ?_⟩⟩
    · intro hk
      have hka : k ≤ a := A.le_max' k (Finset.mem_filter.mpr ⟨Finset.mem_univ _, hk⟩)
      have h1 := (hρm t ht).monotone hka
      simp only; linarith
    · intro hk
      exact absurd ⟨k, Finset.mem_filter.mpr ⟨Finset.mem_univ _, hk⟩⟩ hB
  · set b := B.min' hB
    refine ⟨fun t => ρ b t - 1, (hρc b).sub continuousOn_const, fun t ht k => ⟨?_, ?_⟩⟩
    · intro hk
      exact absurd ⟨k, Finset.mem_filter.mpr ⟨Finset.mem_univ _, hk⟩⟩ hA
    · intro hk
      have hbk : b ≤ k := B.min'_le k (Finset.mem_filter.mpr ⟨Finset.mem_univ _, hk⟩)
      have h1 := (hρm t ht).monotone hbk
      simp only; linarith
  · refine ⟨fun _ => 0, continuousOn_const, fun t _ k => ?_⟩
    by_cases hk : P k
    · exact absurd ⟨k, Finset.mem_filter.mpr ⟨Finset.mem_univ _, hk⟩⟩ hA
    · exact absurd ⟨k, Finset.mem_filter.mpr ⟨Finset.mem_univ _, hk⟩⟩ hB

/-- **R1 for a cylindrical arrangement.** Every sign condition on the carriers in `J` is
preconnected, given preconnected base cells, order-invariant delineable fibres over each, and Thom
fibres. -/
theorem cyl_R1 (cs : List (T × ℝ → ℝ)) (J isBase : ℕ → Prop) (hcont : ∀ c ∈ cs, Continuous c)
    (hbase : ∀ i (hi : i < cs.length), J i → isBase i → ∀ t s, cs[i] (t, s) = cs[i] (t, 0))
    (h1 : ∀ X, IsPreconnected (BaseCell cs J isBase X))
    (h2 : ∀ X, FibreOI cs J isBase (BaseCell cs J isBase X))
    (h3 : ∀ X, ∀ t ∈ BaseCell cs J isBase X, (FibreSet cs J isBase X t).OrdConnected) (X : Cov) :
    IsPreconnected (SignOn cs J X) := by
  classical
  rcases (SignOn cs J X).eq_empty_or_nonempty with hE | ⟨⟨t₀, s₀⟩, h₀⟩
  · rw [hE]; exact isPreconnected_empty
  set C := BaseCell cs J isBase X
  have hmemE : ∀ t s, (t, s) ∈ SignOn cs J X ↔ t ∈ C ∧ s ∈ FibreSet cs J isBase X t := by
    intro t s
    constructor
    · intro h
      refine ⟨fun i hi hJ hb => ?_, fun i hi hJ _ => h i hi hJ⟩
      rw [h i hi hJ, hbase i hi hJ hb t s]
    · rintro ⟨hb, hf⟩ i hi hJ
      by_cases hib : isBase i
      · rw [hb i hi hJ hib, hbase i hi hJ hib t s]
      · exact hf i hi hJ hib
  obtain ⟨ht₀, hs₀⟩ := (hmemE t₀ s₀).mp h₀
  obtain ⟨m, ρ, hρc, hρm, hOI⟩ := h2 X
  have hC := h1 X
  have hcomp : ∀ i (hi : i < cs.length) {σ : T → ℝ}, ContinuousOn σ C →
      ContinuousOn (fun t => cs[i] (t, σ t)) C := fun i hi σ hσ =>
    (hcont _ (List.getElem_mem hi)).comp_continuousOn (continuousOn_id.prodMk hσ)
  -- a continuous representative of the class over the base cell
  have key : ∃ σ : T → ℝ, ContinuousOn σ C ∧ ∀ t ∈ C, σ t ∈ FibreSet cs J isBase X t := by
    by_cases hsec : ∃ k₀, ρ k₀ t₀ = s₀
    · obtain ⟨k₀, hk₀⟩ := hsec
      refine ⟨ρ k₀, hρc k₀, fun t ht i hi hJ hib => ?_⟩
      rw [hs₀ i hi hJ hib]
      congr 1
      rcases hOI i hi hJ hib with hz | ⟨S, hS⟩
      · rw [hz t ht, hz t₀ ht₀]
      · by_cases hkS : k₀ ∈ S
        · rw [(hS t ht _).mpr ⟨k₀, hkS, rfl⟩, (hS t₀ ht₀ _).mpr ⟨k₀, hkS, hk₀⟩]
        · rw [← hk₀]
          refine sgnK_const hC (hcomp i hi (hρc k₀)) (fun t' ht' h0 => ?_) ht₀ ht
          obtain ⟨k, hkS', hk⟩ := (hS t' ht' _).mp h0
          have : k = k₀ := (hρm t' ht').injective hk
          exact hkS (this ▸ hkS')
    · push Not at hsec
      obtain ⟨σ, hσc, hσ⟩ := sector_rep ρ hρc hρm (fun k => ρ k t₀ < s₀)
        (fun a b hab hb => lt_of_le_of_lt ((hρm t₀ ht₀).monotone hab) hb)
      have hgt : ∀ k, ¬ρ k t₀ < s₀ → s₀ < ρ k t₀ := fun k hk =>
        lt_of_le_of_ne (not_lt.mp hk) (hsec k).symm
      refine ⟨σ, hσc, fun t ht i hi hJ hib => ?_⟩
      rw [hs₀ i hi hJ hib]
      congr 1
      rcases hOI i hi hJ hib with hz | ⟨S, hS⟩
      · rw [hz t ht, hz t₀ ht₀]
      · have hne : ∀ t' ∈ C, cs[i] (t', σ t') ≠ 0 := fun t' ht' h0 => by
          obtain ⟨k, -, hk⟩ := (hS t' ht' _).mp h0
          by_cases hP : ρ k t₀ < s₀
          · exact (hσ t' ht' k).1 hP |>.ne hk
          · exact (hσ t' ht' k).2 hP |>.ne' hk
        have hstep := sgnK_const hC (hcomp i hi hσc) hne ht₀ ht
        rw [← hstep]
        -- along the fibre over `t₀`, from `s₀` to `σ t₀`, no zero
        have hfc : ContinuousOn (fun s => cs[i] (t₀, s)) (uIcc s₀ (σ t₀)) :=
          ((hcont _ (List.getElem_mem hi)).comp (continuous_const.prodMk continuous_id)).continuousOn
        refine sgnK_const isPreconnected_uIcc hfc (fun z hz h0 => ?_) left_mem_uIcc right_mem_uIcc
        obtain ⟨k, -, hk⟩ := (hS t₀ ht₀ _).mp h0
        have hz' := mem_uIcc.mp hz
        by_cases hP : ρ k t₀ < s₀
        · have := (hσ t₀ ht₀ k).1 hP
          rcases hz' with ⟨h1, -⟩ | ⟨h1, -⟩ <;> linarith
        · have := (hσ t₀ ht₀ k).2 hP
          have := hgt k hP
          rcases hz' with ⟨-, h1⟩ | ⟨-, h1⟩ <;> linarith
  obtain ⟨σ, hσc, hσ⟩ := key
  -- the class is the union of its fibres, each joined to the graph of `σ`
  set Γ := (fun t => (t, σ t)) '' C
  have hΓ : IsPreconnected Γ := hC.image _ (continuousOn_id.prodMk hσc)
  set 𝒮 := (fun t => ({t} ×ˢ FibreSet cs J isBase X t) ∪ Γ) '' C
  have hE : SignOn cs J X = ⋃₀ 𝒮 := by
    ext ⟨t, s⟩
    simp only [mem_sUnion, 𝒮, mem_image, exists_exists_and_eq_and]
    rw [hmemE]
    constructor
    · rintro ⟨ht, hs⟩
      exact ⟨t, ht, Or.inl ⟨rfl, hs⟩⟩
    · rintro ⟨t', ht', h | ⟨t'', ht'', he⟩⟩
      · obtain ⟨h1, h2⟩ := h
        simp only [mem_singleton_iff] at h1
        subst h1; exact ⟨ht', h2⟩
      · simp only [Prod.mk.injEq] at he
        obtain ⟨rfl, rfl⟩ := he
        exact ⟨ht'', hσ t'' ht''⟩
  rw [hE]
  refine isPreconnected_sUnion (t₀, σ t₀) 𝒮 ?_ ?_
  · rintro _ ⟨t, -, rfl⟩
    exact Or.inr ⟨t₀, ht₀, rfl⟩
  · rintro _ ⟨t, ht, rfl⟩
    refine IsPreconnected.union (t, σ t) ⟨rfl, hσ t ht⟩ ⟨t, ht, rfl⟩ ?_ hΓ
    exact isPreconnected_singleton.prod (h3 X t ht).isPreconnected

end ForgeCurved.CAD
