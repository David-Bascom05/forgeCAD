/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMesh.Funs

/-!
# The binary searches return

`one` (a tangled boundary's stand-in for a point) and `find_use` (a chord's place among the
counted chords) return `ok`, each loop's window `[l, r)` shrinking inside the slice.
-/

open Aeneas Aeneas.Std Result
namespace forge_mesh

@[step] theorem one_loop_spec (same_as : Slice Std.U64) (v : Std.U32) (l r : Std.Usize)
    (hlr : l.val ≤ r.val) (hr : r.val ≤ same_as.length) :
    cdt.one_loop same_as v l r ⦃ k => k.val ≤ same_as.length ⦄ := by
  unfold cdt.one_loop
  apply loop.spec_decr_nat
    (fun (a : Std.Usize × Std.Usize) => a.2.val - a.1.val)
    (fun a => a.1.val ≤ a.2.val ∧ a.2.val ≤ same_as.length)
  · rintro ⟨a, b⟩ ⟨g1, g2⟩
    simp only at g1 g2 ⊢
    unfold cdt.one_loop.body
    simp only [lift, bind_ok]
    split <;> step* <;> (try split) <;> step* <;> (try simp_all) <;> scalar_tac
  · simp_all

theorem one_spec (same_as : Slice Std.U64) (v : Std.U32) : cdt.one same_as v ⦃ _ => True ⦄ := by
  unfold cdt.one
  simp only [lift, bind_ok]
  step*
  all_goals (try split) <;> step* <;> simp_all

@[step] theorem find_use_loop_spec (uses : Slice (Std.U32 × Std.U32 × Std.U32)) (lo hi : Std.U32)
    (l r : Std.Usize) (hlr : l.val ≤ r.val) (hr : r.val ≤ uses.length) :
    cdt.find_use_loop uses lo hi l r ⦃ k => k.val ≤ uses.length ⦄ := by
  unfold cdt.find_use_loop
  apply loop.spec_decr_nat
    (fun (a : Std.Usize × Std.Usize) => a.2.val - a.1.val)
    (fun a => a.1.val ≤ a.2.val ∧ a.2.val ≤ uses.length)
  · rintro ⟨a, b⟩ ⟨g1, g2⟩
    simp only at g1 g2 ⊢
    unfold cdt.find_use_loop.body
    split
    · step*
      rename_i t _ _
      obtain ⟨x, y, z⟩ := t
      simp only [uncurry_apply_pair, WP.uncurry'_pair]
      split <;> (try split) <;> (try split) <;> step* <;> scalar_tac
    · simp_all; omega
  · simp_all

/-- `find_use` returns a place at most the length, and whether the chord is there. -/
@[step] theorem find_use_spec (uses : Slice (Std.U32 × Std.U32 × Std.U32)) (a b : Std.U32) :
    cdt.find_use uses a b ⦃ r => r.1.val ≤ uses.length ⦄ := by
  unfold cdt.find_use
  split <;> step* <;> (try split) <;> step* <;> (try split) <;> simp_all

end forge_mesh
