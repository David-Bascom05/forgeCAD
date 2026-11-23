import ForgeTopo.Funs
open Aeneas Aeneas.Std Result

/-!
# Overflow-checked arithmetic, in range, is the wrapping arithmetic

translate.sh builds with overflow checks, so `a + b` and `a - b` translate to the checked
`a + b`, `a - b`, where an overflow is a `fail` (a panic). Below the type's maximum (for `+`),
and with `b ≤ a` (for `-`), the checked operation returns `ok` of the value the wrapping one
gives. The proofs (Strays, Links, ShellFaces, Edges) were first written against the wrapping
translation. Here the checked operations are rewritten to `lift (wrapping_…)` (`usize_add_eq`,
`usize_sub_eq`, `u32_add_eq`), and `step` meets them through specs (`*_wspec`) that state the
wrapping value. So the same proofs go through, and their `ok` results now also prove that no
operation overflows. The range conditions are the same ones `Usize.add_spec`/`sub_spec` ask
for, and `step` discharges them from the loop guards. The generic specs are switched off in this
package, so these are the ones used.
-/

namespace forge_topo

theorem usize_add_eq (x y : Usize) (h : x.val + y.val ≤ Usize.max) :
    x + y = lift (Std.Usize.wrapping_add x y) := by
  have hq := UScalar.add_equiv x y
  split at hq
  · rename_i z hz
    rw [Result.match.isOk.mp hz, lift]
    congr 1
    apply (UScalar.eq_equiv_bv_eq _ _).mpr
    rw [hq.2.2]; first | rw [UScalar.wrapping_add_bv_eq] | simp [UScalar.wrapping_add_bv_eq]
  · exfalso
    apply hq
    simp only [UScalar.inBounds]
    scalar_tac
  · exact absurd hq (by simp)

theorem u32_add_eq (x y : U32) (h : x.val + y.val ≤ U32.max) :
    x + y = lift (Std.U32.wrapping_add x y) := by
  have hq := UScalar.add_equiv x y
  split at hq
  · rename_i z hz
    rw [Result.match.isOk.mp hz, lift]
    congr 1
    apply (UScalar.eq_equiv_bv_eq _ _).mpr
    rw [hq.2.2]; first | rw [UScalar.wrapping_add_bv_eq] | simp [UScalar.wrapping_add_bv_eq]
  · exfalso
    apply hq
    simp only [UScalar.inBounds]
    scalar_tac
  · exact absurd hq (by simp)

theorem usize_sub_eq (x y : Usize) (h : y.val ≤ x.val) :
    x - y = lift (Std.Usize.wrapping_sub x y) := by
  have hq := UScalar.sub_equiv x y
  split at hq
  · rename_i z hz
    rw [Result.match.isOk.mp hz, lift]
    congr 1
    apply (UScalar.eq_equiv_bv_eq _ _).mpr
    rw [hq.2.2]; first | rw [UScalar.wrapping_sub_bv_eq] | simp
  · exfalso
    omega
  · exact absurd hq (by simp)

theorem usize_add_one (x : Usize) (h : x.val < Usize.max) :
    x + 1#usize = lift (Std.Usize.wrapping_add x 1#usize) :=
  usize_add_eq x 1#usize (by have : (1#usize).val = 1 := rfl; omega)

/-! `step` meets checked arithmetic as the wrapping one, so the proofs read it as before. -/

@[step] theorem usize_add_wspec (x y : Usize) (h : x.val + y.val ≤ Usize.max) :
    x + y ⦃ z => z = Std.Usize.wrapping_add x y ⦄ := by
  rw [usize_add_eq x y h]; simp [lift]

@[step] theorem u32_add_wspec (x y : U32) (h : x.val + y.val ≤ U32.max) :
    x + y ⦃ z => z = Std.U32.wrapping_add x y ⦄ := by
  rw [u32_add_eq x y h]; simp [lift]

@[step] theorem usize_sub_wspec (x y : Usize) (h : y.val ≤ x.val) :
    x - y ⦃ z => z = Std.Usize.wrapping_sub x y ⦄ := by
  rw [usize_sub_eq x y h]; simp [lift]

attribute [-step] Usize.add_spec U32.add_spec Usize.sub_spec

end forge_topo
