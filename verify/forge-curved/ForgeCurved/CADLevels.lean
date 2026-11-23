import ForgeCurved.CAD
import ForgeCurved.Bridge

/-!
# Tier 1's R1: the three levels assembled

Design §2.3 (Tier 1, a CAD with Thom closure) uses coordinates `((t, r), s)`: `t = d·x` (level 1),
`r = w·x` (level 2), and `s = u·x` (the fibres). Every carrier has a level:
* level 1, the **walls** `t = e_k`: functions of `t` alone, monotone;
* level 2, the **projection family** `B` (Disc, Res, psc₁ and their `∂_r`-closure, cylinders
  along `u`): functions of `(t, r)`;
* level 3, the **fibre family** `F` (input carriers, seams, polar planes `∂_u q`).

**`cad3_R1`**: every sign class of the whole arrangement is preconnected, given
* both levels' Thom closure (`ThomFamily` in each `r`-fibre and each `s`-fibre);
* both levels' order-invariant delineability (`FibreOI`) over each cell below.

`cyl_R1` is applied twice: the walls' cells on the `t`-line are intervals or points
(`walls_cell`), and the level-≤2 cells are the base cells of level 3.

**`cad_R1`** moves this to `ℝ³` along any continuous surjection `Φ : (ℝ × ℝ) × ℝ → ℝ³` (the
linear change of coordinates from `(d, w, u)`): every realised cell is `Connected`, which is R1.

The `FibreOI` hypotheses are what the projection guarantees. `fibreOI_of_roots` reduces them to
root functions that are pairwise equal-or-disjoint on the cell (`Thom.merge`). For level 2 the
root functions come from `Delineable.delineable`. For level 3 they are the quadratic formula,
with the coincidences decided by Res and psc₁.
-/

namespace ForgeCurved.CAD

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Set ForgeCurved.Thom

variable {T : Type}

/-- Thom fibres from a Thom family in every fibre. -/
theorem thom_fibres (cs : List (T × ℝ → ℝ)) (J isBase : ℕ → Prop)
    (hbase : ∀ i (hi : i < cs.length), J i → isBase i → ∀ t s, cs[i] (t, s) = cs[i] (t, 0))
    (hthom : ∀ t, ThomFamily (cs.map fun c s => c (t, s)) J) :
    ∀ X, ∀ t ∈ BaseCell cs J isBase X, (FibreSet cs J isBase X t).OrdConnected := by
  intro X t ht
  have : FibreSet cs J isBase X t = signSet (cs.map fun c s => c (t, s)) J X cs.length := by
    ext s
    simp only [FibreSet, signSet, mem_ofPred_eq, List.length_map, List.getElem_map]
    constructor
    · intro h i hi _ hJ
      by_cases hb : isBase i
      · rw [ht i hi hJ hb, hbase i hi hJ hb t s]
      · exact h i hi hJ hb
    · intro h i hi hJ _
      exact h i hi hi hJ
  rw [this]
  exact thom (hthom t) X _

/-- On the `t`-line, the cells of monotone base carriers (the walls) are intervals. -/
theorem walls_cell (cs : List (ℝ × ℝ → ℝ)) (J isBase : ℕ → Prop)
    (hmono : ∀ i (hi : i < cs.length), J i → isBase i → Monotone fun t => cs[i] (t, 0)) (X : Cov) :
    (BaseCell cs J isBase X).OrdConnected := by
  constructor
  intro x hx y hy z hz i hi hJ hb
  have hx' := hx i hi hJ hb
  have hy' := hy i hi hJ hb
  rcases hXi : X[i]? with _ | σ
  · rw [hXi] at hx'; cases hx'
  rw [hXi] at hx' hy'
  have h1 := hmono i hi hJ hb hz.1
  have h2 := hmono i hi hJ hb hz.2
  exact congrArg some (sgnK_between (Option.some.inj hx').symm (Option.some.inj hy').symm h1 h2).symm

variable [TopologicalSpace T]

/-- **`FibreOI` from root functions.** If every fibre carrier's zeros over `C` are a fixed set of
continuous root functions, and any two root functions are equal everywhere on `C` or nowhere, the
fibres are delineable and order invariant. -/
theorem fibreOI_of_roots (cs : List (T × ℝ → ℝ)) (J isBase : ℕ → Prop) {C : Set T}
    (hC : IsPreconnected C) {ι : Type} [Fintype ι] (g : ι → T → ℝ)
    (hg : ∀ a, ContinuousOn (g a) C)
    (hed : ∀ a b, (∀ t ∈ C, g a t = g b t) ∨ ∀ t ∈ C, g a t ≠ g b t)
    (hroots : ∀ i (hi : i < cs.length), J i → ¬isBase i →
      (∀ t ∈ C, ∀ s, cs[i] (t, s) = 0) ∨
        ∃ I : Finset ι, ∀ t ∈ C, ∀ s, cs[i] (t, s) = 0 ↔ ∃ a ∈ I, g a t = s) :
    FibreOI cs J isBase C := by
  classical
  obtain ⟨m, ρ, idx, hρc, hρm, hidx, -⟩ := merge hC g hg hed
  refine ⟨m, ρ, hρc, hρm, fun i hi hJ hb => ?_⟩
  rcases hroots i hi hJ hb with h | ⟨I, hI⟩
  · exact Or.inl h
  · refine Or.inr ⟨I.image idx, fun t ht s => ?_⟩
    rw [hI t ht s]
    constructor
    · rintro ⟨a, ha, rfl⟩
      exact ⟨idx a, Finset.mem_image_of_mem _ ha, (hidx a t ht).symm⟩
    · rintro ⟨k, hk, hks⟩
      obtain ⟨a, ha, rfl⟩ := Finset.mem_image.mp hk
      exact ⟨a, ha, by rw [hidx a t ht]; exact hks⟩

/-- The level-≤2 arrangement on the `(t, r)`-plane. -/
def lower (cs : List ((ℝ × ℝ) × ℝ → ℝ)) : List (ℝ × ℝ → ℝ) := cs.map fun c p => c (p, 0)

/-- **R1 for the three-level CAD**, on `((t, r), s)`. -/
theorem cad3_R1 (cs : List ((ℝ × ℝ) × ℝ → ℝ)) (lvl : ℕ → ℕ) (hcont : ∀ c ∈ cs, Continuous c)
    (h12 : ∀ i (hi : i < cs.length), lvl i ≤ 2 → ∀ p s, cs[i] (p, s) = cs[i] (p, 0))
    (h1 : ∀ i (hi : i < cs.length), lvl i = 1 → ∀ t r, cs[i] ((t, r), 0) = cs[i] ((t, 0), 0))
    (hmono : ∀ i (hi : i < cs.length), lvl i = 1 → Monotone fun t => cs[i] ((t, 0), 0))
    (hthom2 : ∀ t, ThomFamily ((lower cs).map fun c r => c (t, r)) fun i => lvl i ≤ 2)
    (hoi2 : ∀ X, FibreOI (lower cs) (fun i => lvl i ≤ 2) (fun i => lvl i = 1)
      (BaseCell (lower cs) (fun i => lvl i ≤ 2) (fun i => lvl i = 1) X))
    (hthom3 : ∀ p, ThomFamily (cs.map fun c s => c (p, s)) fun _ => True)
    (hoi3 : ∀ X, FibreOI cs (fun _ => True) (fun i => lvl i ≤ 2)
      (BaseCell cs (fun _ => True) (fun i => lvl i ≤ 2) X))
    (X : Cov) : IsPreconnected (SignOn cs (fun _ => True) X) := by
  have hlen : (lower cs).length = cs.length := by simp [lower]
  have hget : ∀ i (hi : i < (lower cs).length) p, (lower cs)[i] p = (cs[i]'(hlen ▸ hi)) (p, 0) := by
    intro i hi p
    exact congrFun (List.getElem_map (fun c (p : ℝ × ℝ) => c (p, 0)) (l := cs) (h := hi)) p
  have hcont2 : ∀ c ∈ lower cs, Continuous c := by
    intro c hc
    obtain ⟨c', hc', rfl⟩ := List.mem_map.mp hc
    exact (hcont c' hc').comp (continuous_id.prodMk continuous_const)
  have hbase2 : ∀ i (hi : i < (lower cs).length), lvl i ≤ 2 → lvl i = 1 →
      ∀ t s, (lower cs)[i] (t, s) = (lower cs)[i] (t, 0) := by
    intro i hi _ h t s
    rw [hget, hget]; exact h1 i _ h t s
  -- level 2: the cells of the walls and `B` on the `(t, r)`-plane are preconnected
  have hlow : ∀ X, IsPreconnected (SignOn (lower cs) (fun i => lvl i ≤ 2) X) := by
    intro X
    refine cyl_R1 (lower cs) _ _ hcont2 hbase2 (fun X => ?_) hoi2
      (thom_fibres _ _ _ hbase2 hthom2) X
    refine (walls_cell _ _ _ (fun i hi _ h => ?_) X).isPreconnected
    simp only [hget]; exact hmono i _ h
  -- level 3
  refine cyl_R1 cs _ _ hcont (fun i hi _ h => h12 i hi h) (fun X => ?_) hoi3
    (thom_fibres _ _ _ (fun i hi _ h => h12 i hi h) hthom3) X
  have : BaseCell cs (fun _ => True) (fun i => lvl i ≤ 2) X =
      SignOn (lower cs) (fun i => lvl i ≤ 2) X := by
    ext p
    simp only [BaseCell, SignOn, mem_ofPred_eq, forall_const, hget]
    constructor
    · intro h i hi hl; exact h i (hlen ▸ hi) hl
    · intro h i hi hl; exact h i (hlen ▸ hi) hl
  rw [this]; exact hlow X

/-- **R1 on ℝ³, from the CAD in coordinates.** If `Φ` is a continuous surjection and the pulled-back
carriers form a three-level CAD (the hypotheses of `cad3_R1`), every cell is connected. -/
theorem cad_R1 {Y : Type} [TopologicalSpace Y] (Φ : (ℝ × ℝ) × ℝ → Y) (hΦ : Continuous Φ)
    (hsurj : Function.Surjective Φ) (csY : List (Carrier Y ℝ))
    (hR1 : ∀ X, IsPreconnected (SignOn (csY.map fun c => c ∘ Φ) (fun _ => True) X)) (X : Cov) :
    Connected (covCellK csY X) := by
  rw [ForgeCurved.connected_iff]
  have : {x | covCellK csY X x} = Φ '' {y | covOfK (csY.map fun c => c ∘ Φ) y = X} := by
    ext x
    constructor
    · intro hx
      obtain ⟨y, rfl⟩ := hsurj x
      refine ⟨y, ?_, rfl⟩
      show covOfK _ y = X
      have : covOfK (csY.map fun c => c ∘ Φ) y = covOfK csY (Φ y) := by
        simp [covOfK, Function.comp_def]
      rw [this]; exact hx
    · rintro ⟨y, hy, rfl⟩
      show covOfK csY (Φ y) = X
      have : covOfK (csY.map fun c => c ∘ Φ) y = covOfK csY (Φ y) := by
        simp [covOfK, Function.comp_def]
      rw [← this]; exact hy
  rw [this, signOn_univ]
  rcases Classical.em (X.length = (csY.map fun c => c ∘ Φ).length) with hl | hl
  · have : {_y : (ℝ × ℝ) × ℝ | X.length = (csY.map fun c => c ∘ Φ).length} = univ := by
      ext; simp [hl]
    rw [this, univ_inter]
    exact (hR1 X).image Φ hΦ.continuousOn
  · have : {_y : (ℝ × ℝ) × ℝ | X.length = (csY.map fun c => c ∘ Φ).length} = ∅ := by
      ext; simp only [mem_ofPred_eq, mem_empty_iff_false, iff_false]; exact hl
    rw [this, empty_inter, image_empty]; exact isPreconnected_empty

end ForgeCurved.CAD
