/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Identities
import IEEE754.Sqrt
import IEEE754.BitsThm

/-!
# binary64 (`f64`)

The general theorems with binary64's constants: precision 53, `emin = -1022`, least subnormal
`2^-1074`, overflow threshold `2^1024`.
-/

namespace IEEE754

/-- A binary64 datum: Rust's `f64`. -/
abbrev F64 := FP binary64

@[simp] lemma binary64_prec : binary64.prec = 53 := rfl
@[simp] lemma binary64_emax : binary64.emax = 1023 := by decide
@[simp] lemma binary64_emin : binary64.emin = -1022 := by decide
@[simp] lemma binary64_qmin : binary64.qmin = -1074 := by decide
@[simp] lemma binary64_qmax : binary64.qmax = 971 := by decide
@[simp] lemma binary64_width : binary64.width = 64 := rfl

namespace F64

open FP

theorem add_rel_err {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hfin : (add x y).isFinite)
    (hn : 2 ^ (-1022 : ℤ) ≤ |x.val + y.val|) :
    |(add x y).val - (x.val + y.val)| ≤ 2 ^ (-53 : ℤ) * |x.val + y.val| := by
  simpa using IEEE754.add_rel_err hx hy hfin (by simpa using hn)

theorem sub_rel_err {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hfin : (sub x y).isFinite)
    (hn : 2 ^ (-1022 : ℤ) ≤ |x.val - y.val|) :
    |(sub x y).val - (x.val - y.val)| ≤ 2 ^ (-53 : ℤ) * |x.val - y.val| := by
  simpa using IEEE754.sub_rel_err hx hy hfin (by simpa using hn)

theorem mul_rel_err {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hfin : (mul x y).isFinite)
    (hn : 2 ^ (-1022 : ℤ) ≤ |x.val * y.val|) :
    |(mul x y).val - x.val * y.val| ≤ 2 ^ (-53 : ℤ) * |x.val * y.val| := by
  simpa using IEEE754.mul_rel_err hx hy hfin (by simpa using hn)

theorem div_rel_err {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hy0 : y.val ≠ 0)
    (hfin : (div x y).isFinite) (hn : 2 ^ (-1022 : ℤ) ≤ |x.val / y.val|) :
    |(div x y).val - x.val / y.val| ≤ 2 ^ (-53 : ℤ) * |x.val / y.val| := by
  simpa using IEEE754.div_rel_err hx hy hy0 hfin (by simpa using hn)

theorem fma_rel_err {x y z : F64} (hx : x.isFinite) (hy : y.isFinite) (hz : z.isFinite)
    (hfin : (fma x y z).isFinite) (hn : 2 ^ (-1022 : ℤ) ≤ |x.val * y.val + z.val|) :
    |(fma x y z).val - (x.val * y.val + z.val)| ≤ 2 ^ (-53 : ℤ) * |x.val * y.val + z.val| := by
  simpa using IEEE754.fma_rel_err hx hy hz hfin (by simpa using hn)

theorem mul_abs_err {x y : F64} (hx : x.isFinite) (hy : y.isFinite)
    (hn : |x.val * y.val| < 2 ^ (-1022 : ℤ)) :
    (mul x y).isFinite ∧ |(mul x y).val - x.val * y.val| ≤ 2 ^ (-1075 : ℤ) := by
  simpa using IEEE754.mul_abs_err hx hy (by simpa using hn)

theorem ofInt_exact {n : ℤ} (hn : |n| ≤ 2 ^ 53) :
    (ofInt (f := binary64) n).isFinite ∧ (ofInt (f := binary64) n).val = n :=
  FP.ofInt_exact (by simpa using hn) (by simp)

theorem toBits_lt (x : F64) : x.toBits < 2 ^ 64 := by simpa using FP.toBits_lt x

end F64

end IEEE754
