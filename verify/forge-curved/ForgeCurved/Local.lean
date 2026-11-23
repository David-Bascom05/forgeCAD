import ForgeCurved.Separators

/-!
# Faithful from local models: R2–R5 are local, R1 is global

`Faithful`'s five fields split by where they are decided:

* **R1** (each cell connected) is global: it is the Jordan-type part, the one the Tier-0 graph check
  certifies.
* **R2** (`Cov.le` is closure incidence), **R4** and **R5** (the wing conditions) only ask about
  small neighbourhoods of a point of a cell. **R3** is the special case of R2 for a facet's sides
  (and is proven already for seamed quadrics, `quadric_sides`).

`LocalGood cs x` is the local picture at `x` that gives them: arbitrarily small neighbourhoods
`U` of `x` meet every cell above `x`'s in a connected set, and every realised cell above `x`'s has
`x` in its closure. **`faithfulAll_of_local`**: continuous carriers, R1, R3 and `LocalGood`
everywhere give `FaithfulAll`. With `LocalGood'` (closure for every cell above, realised or not)
R3 follows too (`faithfulAll_of_local'`).

Transversal points have a `LocalGood'` picture by the inverse function theorem
(Transversal.lean). Non-transversal points (a cone apex, a tangent contact line) need their own
local models; the statement here does not care how the local picture is obtained.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology

variable {P : Type} [TopologicalSpace P]

/-- **The local picture at `x`**: small neighbourhoods meet each cell above `x`'s in a connected
set, and each realised cell above `x`'s comes arbitrarily close to `x`. -/
structure LocalGood (cs : List (Carrier P ℝ)) (x : P) : Prop where
  conn : ∀ W : Set P, IsOpen W → x ∈ W → ∃ U : Set P, IsOpen U ∧ U ⊆ W ∧ x ∈ U ∧
    ∀ Y, Cov.le (covOfK cs x) Y = true → IsPreconnected (cell cs Y ∩ U)
  near : ∀ Y, Cov.le (covOfK cs x) Y = true → (∃ y, y ∈ cell cs Y) → x ∈ closure (cell cs Y)

/-- The local picture with every cell above `x`'s near `x`, realised or not (so all of them are
realised: the transversal case). -/
structure LocalGood' (cs : List (Carrier P ℝ)) (x : P) : Prop where
  conn : ∀ W : Set P, IsOpen W → x ∈ W → ∃ U : Set P, IsOpen U ∧ U ⊆ W ∧ x ∈ U ∧
    ∀ Y, Cov.le (covOfK cs x) Y = true → IsPreconnected (cell cs Y ∩ U)
  near : ∀ Y, Cov.le (covOfK cs x) Y = true → x ∈ closure (cell cs Y)

theorem LocalGood'.toLocalGood {cs : List (Carrier P ℝ)} {x : P} (h : LocalGood' cs x) :
    LocalGood cs x :=
  ⟨h.conn, fun Y hY _ => h.near Y hY⟩

/-- R1, as a Mathlib statement: every realised cell is preconnected. -/
def CellsConnected (cs : List (Carrier P ℝ)) : Prop :=
  ∀ X, RealisedK cs X → IsPreconnected (cell cs X)

/-- R3, as `Faithful` states it, for every facet. -/
def SidesAll (cs : List (Carrier P ℝ)) : Prop :=
  ∀ φ : Cov, φ.isFacet = true → ∀ x, covCellK cs φ x →
    ForgeExact.Topology.closure (covCellK cs φ.plus) x ∧
      ForgeExact.Topology.closure (covCellK cs φ.minus) x

theorem connected_cell_iff (cs : List (Carrier P ℝ)) (X : Cov) :
    Connected (covCellK cs X) ↔ IsPreconnected (cell cs X) := connected_iff _

/-- The wing conditions (R4 and R5) at a point, from the local picture. -/
theorem wing_of_local {cs : List (Carrier P ℝ)} {x : P} (hl : LocalGood cs x) {φ : Cov}
    (hle : Cov.le (covOfK cs x) φ = true) (W : Pred P) (hW : ForgeExact.Topology.IsOpen W)
    (hx : W x) : ∃ U : Pred P, ForgeExact.Topology.IsOpen U ∧ U.Sub W ∧ U x ∧
      Connected ((covCellK cs φ).inter U) := by
  obtain ⟨U, hUo, hUW, hxU, hc⟩ := hl.conn {y | W y} hW hx
  refine ⟨fun y => y ∈ U, hUo, fun y hy => hUW hy, hxU, ?_⟩
  rw [connected_iff]
  exact hc φ hle

/-- **Faithful from local pictures.** Continuous carriers, every realised cell connected (R1),
both sides beside every facet (R3), and the local picture at every point give R1–R5 for every
cell. -/
theorem faithfulAll_of_local {cs : List (Carrier P ℝ)} (hc : ∀ c ∈ cs, Continuous c)
    (h1 : CellsConnected cs) (h3 : SidesAll cs) (hl : ∀ x, LocalGood cs x) : FaithfulAll cs where
  connected X _ hX := (connected_cell_iff cs X).mpr (h1 X hX)
  incidence X Y _ _ hX hY := by
    obtain ⟨x0, hx0⟩ := hX
    constructor
    · intro hle x hx
      have hx' : covOfK cs x = X := hx
      rw [closure_iff]
      obtain ⟨y, hy⟩ := hY
      exact (hl x).near Y (by rw [hx']; exact hle) ⟨y, hy⟩
    · intro hsub
      have hx0' : covOfK cs x0 = X := hx0
      have := (closure_iff _ _).mp (hsub x0 hx0)
      rw [← hx0']
      exact le_of_mem_closure hc this
  sides φ _ hφ x hx := h3 φ hφ x hx
  wing e φ _ _ _ _ hle x hx W hW hxW := by
    have hx' : covOfK cs x = e := hx
    exact wing_of_local (hl x) (by rw [hx']; exact hle) W hW hxW
  vwing v φ _ _ _ _ hle x hx W hW hxW := by
    have hx' : covOfK cs x = v := hx
    exact wing_of_local (hl x) (by rw [hx']; exact hle) W hW hxW

/-- R3 from the stronger local picture. -/
theorem sidesAll_of_local' {cs : List (Carrier P ℝ)} (hl : ∀ x, LocalGood' cs x) :
    SidesAll cs := by
  intro φ hφ x hx
  have hx' : covOfK cs x = φ := hx
  obtain ⟨hp, hm⟩ := le_plus_minus hφ
  rw [closure_iff, closure_iff]
  exact ⟨(hl x).near _ (by rw [hx']; exact hp), (hl x).near _ (by rw [hx']; exact hm)⟩

/-- **Faithful from the stronger local pictures**: R1 is all that is left. -/
theorem faithfulAll_of_local' {cs : List (Carrier P ℝ)} (hc : ∀ c ∈ cs, Continuous c)
    (h1 : CellsConnected cs) (hl : ∀ x, LocalGood' cs x) : FaithfulAll cs :=
  faithfulAll_of_local hc h1 (sidesAll_of_local' hl) fun x => (hl x).toLocalGood

end ForgeCurved
