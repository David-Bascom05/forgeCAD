import ForgeBoolCore.Refine
open Aeneas Aeneas.Std Result

namespace forge_bool_core

/-- The zeros of a covector, in order. -/
def zerosL (x : List Sgn) : List Nat := (List.range x.length).filter fun i => x.getD i .Pos == .Zero

theorem zerosL_succ (x : List Sgn) (i : Nat) (h : i < x.length) :
    (List.range (i + 1)).filter (fun j => x.getD j .Pos == .Zero) =
      (List.range i).filter (fun j => x.getD j .Pos == .Zero) ++ (if x[i] = .Zero then [i] else []) := by
  rw [List.range_succ, List.filter_append]
  simp [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]
  split <;> simp_all

theorem zeros_loop_spec (x : Slice Sgn) (z : alloc.vec.Vec Usize) (i : Usize) (hi : i.val ≤ x.val.length)
    (hz : z.val.map (·.val) = (List.range i.val).filter fun j => x.val.getD j .Pos == .Zero) :
    zeros_loop x z i ⦃ r => r.val.map (·.val) = zerosL x.val ⦄ := by
  unfold zeros_loop
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec Usize × Usize) => x.val.length - q.2.val)
    (fun q => q.2.val ≤ x.val.length ∧
      q.1.val.map (·.val) = (List.range q.2.val).filter fun j => x.val.getD j .Pos == .Zero)
  · rintro ⟨w, j⟩ ⟨hj, hw⟩
    unfold zeros_loop.body
    simp only at hj hw ⊢
    split
    · rename_i hlt
      have hjl : j.val < x.val.length := by scalar_tac
      step as ⟨sg, hsg⟩
      rw [sgn_eq]; simp only [Std.bind_ok]
      have hsg' : sg = x.val[j.val] := by rw [hsg]
      have hwl : w.val.length < Usize.max := by
        have := congrArg List.length hw
        simp only [List.length_map] at this
        have h2 := List.length_filter_le (fun j => x.val.getD j .Pos == .Zero) (List.range j.val)
        rw [List.length_range] at h2
        have h3 : x.val.length ≤ Usize.max := by scalar_tac
        omega
      split
      · rename_i hz0
        step as ⟨w1, hw1⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hj1, hw1, zerosL_succ _ _ hjl, List.map_append, hw, ← hsg']
        simp at hz0; simp [hz0]
      · rename_i hz0
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hj1, zerosL_succ _ _ hjl, hw, ← hsg']
        simp at hz0; simp [hz0]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = x.val.length := by scalar_tac
      rw [hw, this]; rfl
  · exact ⟨hi, hz⟩

/-- **`zeros` lists a covector's zeros in order.** -/
theorem zeros_spec (x : Slice Sgn) : zeros x ⦃ r => r.val.map (·.val) = zerosL x.val ⦄ := by
  unfold zeros
  exact zeros_loop_spec x _ 0#usize (by simp) (by simp)

end forge_bool_core
