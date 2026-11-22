import Placecuts.DecideOnce

/-!
# A non-transitive `near` splits a chain in two

Five cuts on a line at 0, 2.4, 1.2, 0.6, 1.8 (in that order), every tolerance 1; scaled by 10:
0, 24, 12, 6, 18 with reach 10 (`le1`) and twice the reach 20 (`le2`). Every neighbour in the
sorted order 0, 6, 12, 18, 24 is within reach, so the transitive closure of `near` is one class;
`place_cuts` makes two clusters, {0, 0.6, 1.2} and {2.4, 1.8}, whose points 1.2 and 1.8 lie 0.6
apart, within one tolerance of each other.
-/

namespace placecuts.chain
open Aeneas Aeneas.Std

def pos : List Nat := [0, 24, 12, 6, 18]
def dist (a b : Nat) : Nat := if a ≤ b then b - a else a - b

/-- The distance test of reach `c` on the five cuts (false off the table). -/
def Tc (c : Nat) (i j a : Nat) : Bool := decide (i < 5 ∧ j < 5 ∧ a < 5 ∧ dist pos[i]! pos[j]! ≤ c)

/-- The reference clustering of the chain: two clusters. -/
theorem placeRef_chain : placeRef 5 (Tc 10) (Tc 20) = [0, 1, 0, 0, 1] := by decide +kernel

/-- `near` links the five into one class (0 – 0.6 – 1.2 – 1.8 – 2.4), and is not transitive. -/
theorem near_chain :
    Tc 10 0 3 0 = true ∧ Tc 10 3 2 0 = true ∧ Tc 10 2 4 0 = true ∧ Tc 10 4 1 0 = true ∧
      Tc 10 0 2 0 = false := by decide

def vec {α} (l : List α) (h : l.length ≤ 5) : alloc.vec.Vec α :=
  alloc.vec.Vec.from l (by have : (5 : Nat) ≤ Usize.max := by scalar_tac
                           omega)

/-- The table of `Tc c`, as the extracted code takes it. -/
def mk (c : Nat) : Tbl :=
  vec ((List.range 5).map fun i => vec ((List.range 5).map fun j =>
    vec ((List.range 5).map fun a => Tc c i j a) (by simp)) (by simp)) (by simp)

theorem dflt {α} : ((default : alloc.vec.Vec α) : List α) = [] := rfl

theorem tb_mk (c : Nat) : tb (mk c) = Tc c := by
  funext i j a
  simp only [tb, mk, vec, alloc.vec.Vec.from_val]
  by_cases hi : i < 5
  · by_cases hj : j < 5
    · by_cases ha : a < 5
      · simp [getElem!_pos, hi, hj, ha, Tc]
      · simp [hi, hj, ha, Tc, dflt]
    · simp [hi, hj, Tc, dflt]
  · simp [hi, Tc, dflt]

theorem sized_mk (c : Nat) : Sized 5 (mk c) := by
  refine ⟨by simp [mk, vec], fun i hi => ⟨by simp [mk, vec, hi], fun j hj => by simp [mk, vec, hi, hj]⟩⟩

/-- **The chain on the extracted code**: `place` splits it in two clusters. -/
theorem place_chain :
    place 5#usize (mk 10) (mk 20) ⦃ r => nv r = [0, 1, 0, 0, 1] ⦄ := by
  apply WP.spec_mono (place_spec 5#usize (mk 10) (mk 20) (by simpa using sized_mk 10) (by simpa using sized_mk 20))
  rintro r ⟨hr, -⟩
  rw [hr, tb_mk, tb_mk]
  exact placeRef_chain

end placecuts.chain
