import Placecuts.Funs
open Aeneas Aeneas.Std Result

namespace placecuts

abbrev Tbl := alloc.vec.Vec (alloc.vec.Vec (alloc.vec.Vec Bool))

theorem idxu {α} [Inhabited α] (v : alloc.vec.Vec α) (i : Usize) (h : i.val < v.val.length) :
    alloc.vec.Vec.index_usize v i = ok v.val[i.val]! := by
  simp only [alloc.vec.Vec.index_usize]
  have : v[i.val]? = some v.val[i.val]! := by
    simp [getElem!_pos, h]
  rw [this]

theorem idx {α} [Inhabited α] (v : alloc.vec.Vec α) (i : Usize) (h : i.val < v.val.length) :
    alloc.vec.Vec.index (core.slice.index.SliceIndexUsizeSlice α) v i = ok v.val[i.val]! := by
  rw [alloc.vec.Vec.index_slice_index, idxu v i h]

/-- The table entry, as a function on naturals. -/
def tb (le : Tbl) (i j a : Nat) : Bool := le.val[i]!.val[j]!.val[a]!

def Sized (n : Nat) (le : Tbl) : Prop :=
  n ≤ le.val.length ∧ ∀ i < n, n ≤ (le.val[i]!).val.length ∧ ∀ j < n, n ≤ (le.val[i]!.val[j]!).val.length

theorem near_eq (n : Nat) (le : Tbl) (hs : Sized n le) (i j a b : Usize)
    (hi : i.val < n) (hj : j.val < n) (ha : a.val < n) (hb : b.val < n) :
    near le i j a b = ok (tb le i j a || tb le i j b) := by
  obtain ⟨h0, h1⟩ := hs
  obtain ⟨h2, h3⟩ := h1 i hi
  have h4 := h3 j hj
  unfold near
  rw [idx _ _ (by omega)]; simp only [Std.bind_ok]
  rw [idx _ _ (by omega)]; simp only [Std.bind_ok]
  rw [idx _ _ (by omega)]; simp only [Std.bind_ok]
  simp only [tb]
  split
  · rename_i h; rw [h]; rfl
  · rename_i h
    rw [idx _ _ (by omega)]
    trace_state
    sorry

end placecuts
