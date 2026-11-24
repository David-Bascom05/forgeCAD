/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMath.Funs
import ForgeNum.Refine
import AeneasF64.Lemmas

/-!
# The real `forge_math::predicates`, against verify/ieee754

These lemmas are about the Aeneas translation of `kernel/forge-math` itself (`ForgeMath/Funs.lean`,
regenerated and checked by `tools/aeneas/translate.sh forge-math --check`). Each one says that
the Rust function returns `ok`, so it does not panic, overflow or diverge, and that its value is
the IEEE 754 model's function of the same name. The ieee session proved those model functions
error-free (verify/ieee754): `two_sum_exact`, `veltkamp`, `two_product_exact`. The corollaries
below carry their theorems over to the shipped code.

`two_sum`, `two_diff`, `split` and `two_product` now live in kernel/forge-num (`forge_num::shewchuk`);
forge-math calls `two_sum`/`two_diff` there and keeps `two_product` as a wrapper (in release, just
the call). The proofs live in ForgeNum/Refine.lean; these restate them, same statements, through the
re-export. (`SPLITTER_eq` went with `SPLITTER`: `ForgeNum.SPLITTER_eq`.)
-/

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open forge_math

namespace ForgeMath

/-- The literal `SPLITTER = 134_217_729.0`, as rustc encoded it. -/
abbrev splitter : F64 := ForgeNum.splitter

lemma splitter_decode : splitter = .fin false (134217729 * 2 ^ 25) (-25) (by decide) :=
  ForgeNum.splitter_decode

lemma splitter_spec : splitter.isFinite ∧ splitter.val = 2 ^ 27 + 1 := ForgeNum.splitter_spec

/-- The Rust `two_sum` is TwoSum. -/
theorem two_sum_eq (a b : F64) : forge_num.shewchuk.two_sum a b = ok (twoSum a b) :=
  ForgeNum.two_sum_eq a b

/-- The Rust `split` is Veltkamp's split with `SPLITTER`. -/
theorem split_eq (a : F64) : forge_num.shewchuk.split a = ok (IEEE754.split splitter a) :=
  ForgeNum.split_eq a

/-- The Rust `two_product` (forge-math's wrapper of forge-num's) is Dekker's product with
`SPLITTER`. -/
theorem two_product_eq (a b : F64) :
    predicates.two_product a b = ok (twoProduct splitter a b) := by
  simp only [predicates.two_product]; exact ForgeNum.two_product_eq a b

/-! ## The model's theorems, for the shipped code -/

/-- **`two_sum` is error-free** (Knuth): `ForgeNum.two_sum_exact`. -/
theorem two_sum_exact (a b : F64) (ha : a.isFinite) (hb : b.isFinite)
    (hs : (FP.add a b).isFinite) (hbv : (FP.sub (FP.add a b) a).isFinite) :
    ∃ s e, forge_num.shewchuk.two_sum a b = ok (s, e) ∧ s = FP.add a b ∧ e.isFinite ∧
      s.val + e.val = a.val + b.val :=
  ForgeNum.two_sum_exact a b ha hb hs hbv

/-- **`split` is Veltkamp's split** (σ = 27): `ForgeNum.split_veltkamp`. -/
theorem split_veltkamp (a : F64) {lam : ℤ} (ha : a.isFinite) (hca : (FP.mul splitter a).isFinite)
    (ha0 : a.val ≠ 0) (hlam : Mult a.val lam) :
    ∃ hi lo, forge_num.shewchuk.split a = ok (hi, lo) ∧ hi.isFinite ∧ lo.isFinite ∧
      ∃ eps, Split binary64 27 a.val hi.val lo.val eps lam :=
  ForgeNum.split_veltkamp a ha hca ha0 hlam

/-- **`two_product` is error-free** (Dekker), under the kernel's precondition `product_exact`
(`a`, `b` on grids `2^la`, `2^lb` with `la + lb ≥ −1074`; `check_two_product` asserts it in
debug builds), with `SPLITTER ⊗ a`, `SPLITTER ⊗ b` finite and `|a · b| < 2^1022`: the shipped
`two_product` returns `(a ⊗ b, y)` with `y` finite and `x + y = a · b` exactly. -/
theorem two_product_exact (a b : F64) (ha : a.isFinite) (hb : b.isFinite)
    (hca : (FP.mul splitter a).isFinite) (hcb : (FP.mul splitter b).isFinite)
    {la lb : ℤ} (hla : Mult a.val la) (hlb : Mult b.val lb) (hΛ : -1074 ≤ la + lb)
    (hP : |a.val * b.val| < 2 ^ 1022) :
    ∃ x y, predicates.two_product a b = ok (x, y) ∧ x = FP.mul a b ∧ y.isFinite ∧
      x.val + y.val = a.val * b.val := by
  simp only [predicates.two_product]
  exact ForgeNum.two_product_exact a b ha hb hca hcb hla hlb hΛ hP

end ForgeMath
