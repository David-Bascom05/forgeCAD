/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMesh.Funs

/-!
# `bad_input` returns

The input check returns `ok` for any input: each of its loops ends, the first fault found
stopping it (the measure drops past the end when the error is set).
-/

open Aeneas Aeneas.Std Result
namespace forge_mesh

@[step] theorem bad_points_spec (points : Slice forge_math.vec.Point2) (err : Option cdt.CdtError)
    (i : Std.Usize) : cdt.bad_input_loop0 points err i ⦃ _ => True ⦄ := by
  unfold cdt.bad_input_loop0
  apply loop.spec_decr_nat
    (fun (a : Option cdt.CdtError × Std.Usize) => points.length + 1 - a.2.val - (if a.1.isNone then 0 else points.length + 1))
    (fun _ => True)
  · rintro ⟨e, k⟩ _
    unfold cdt.bad_input_loop0.body
    simp only [lift, bind_ok, core.f64.F64.is_finite]
    split <;> (try split) <;> step* <;> (try split) <;> (try split) <;> step* <;> (try split) <;> simp_all <;> scalar_tac
  · simp_all

@[step] theorem bad_loop_spec (tangled : Bool) (n : Std.U32) (err : Option cdt.CdtError)
    (li : Std.Usize) (lp : alloc.vec.Vec Std.U32) (k : Std.Usize) :
    cdt.bad_input_loop1_loop0 tangled n err li lp k ⦃ _ => True ⦄ := by
  unfold cdt.bad_input_loop1_loop0
  apply loop.spec_decr_nat
    (fun (a : Option cdt.CdtError × Std.Usize) => lp.length + 1 - a.2.val - (if a.1.isNone then 0 else lp.length + 1))
    (fun _ => True)
  · rintro ⟨e, j⟩ _
    unfold cdt.bad_input_loop1_loop0.body
    split <;> (try split) <;> step* <;> (try split) <;> step* <;> (try split) <;> (try split) <;> (try split) <;>
      simp_all <;> scalar_tac
  · simp_all

@[step] theorem bad_loops_spec (loops : Slice (alloc.vec.Vec Std.U32)) (tangled : Bool) (n : Std.U32)
    (err : Option cdt.CdtError) (li : Std.Usize) :
    cdt.bad_input_loop1 loops tangled n err li ⦃ _ => True ⦄ := by
  unfold cdt.bad_input_loop1
  apply loop.spec_decr_nat
    (fun (a : Option cdt.CdtError × Std.Usize) => loops.length + 1 - a.2.val - (if a.1.isNone then 0 else loops.length + 1))
    (fun _ => True)
  · rintro ⟨e, j⟩ _
    unfold cdt.bad_input_loop1.body
    split <;> (try split) <;> step* <;> (try split) <;> (try split) <;> step* <;> simp_all <;> scalar_tac
  · simp_all

/-- `bad_input` returns. -/
@[step] theorem bad_input_spec (points : Slice forge_math.vec.Point2)
    (loops : Slice (alloc.vec.Vec Std.U32)) (tangled : Bool) :
    cdt.bad_input points loops tangled ⦃ _ => True ⦄ := by
  unfold cdt.bad_input
  simp only [lift, bind_ok]
  step*

end forge_mesh
