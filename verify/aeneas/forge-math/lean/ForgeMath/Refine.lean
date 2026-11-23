/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMath.Funs
import AeneasF64.Lemmas

/-!
# The real `forge_math::predicates`, against verify/ieee754

These lemmas are about the Aeneas translation of `kernel/forge-math` itself (`ForgeMath/Funs.lean`,
regenerated and checked by `tools/aeneas/translate.sh forge-math --check`). Each one says that
the Rust function returns `ok`, so it does not panic, overflow or diverge, and that its value is
the IEEE 754 model's function of the same name. The ieee session proved those model functions
error-free (verify/ieee754): `two_sum_exact`, `veltkamp`, `two_product_exact`. The corollaries
below carry their theorems over to the shipped code.
-/

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open forge_math

namespace ForgeMath

/-- The literal `SPLITTER = 134_217_729.0`, as rustc encoded it. -/
abbrev splitter : F64 := F64.Ops.ofBits 4728779608772575232

lemma splitter_decode : splitter = .fin false (134217729 * 2 ^ 25) (-25) (by decide) := by decide

lemma splitter_spec : splitter.isFinite ∧ splitter.val = 2 ^ 27 + 1 := by
  rw [splitter_decode]; refine ⟨rfl, ?_⟩; simp [FP.val, FP.sgn]; norm_num

lemma SPLITTER_eq : predicates.SPLITTER = splitter := by unfold predicates.SPLITTER; rfl

/-- The Rust `two_sum` is TwoSum. -/
theorem two_sum_eq (a b : F64) : predicates.two_sum a b = ok (twoSum a b) := by
  simp [predicates.two_sum, twoSum, lift]

/-- The Rust `split` is Veltkamp's split with `SPLITTER`. -/
theorem split_eq (a : F64) : predicates.split a = ok (IEEE754.split splitter a) := by
  simp [predicates.split, IEEE754.split, lift, SPLITTER_eq]

/-- The Rust `two_product` is Dekker's product with `SPLITTER`. -/
theorem two_product_eq (a b : F64) :
    predicates.two_product a b = ok (twoProduct splitter a b) := by
  simp [predicates.two_product, twoProduct, split_eq, IEEE754.split, lift]

/-! ## The model's theorems, for the shipped code -/

/-- **`two_sum` is error-free** (Knuth): for finite `a`, `b` whose sum and `s ⊖ a` are finite,
the shipped `two_sum` returns `(a ⊕ b, e)` with `e` finite and `s + e = a + b` exactly. -/
theorem two_sum_exact (a b : F64) (ha : a.isFinite) (hb : b.isFinite)
    (hs : (FP.add a b).isFinite) (hbv : (FP.sub (FP.add a b) a).isFinite) :
    ∃ s e, predicates.two_sum a b = ok (s, e) ∧ s = FP.add a b ∧ e.isFinite ∧
      s.val + e.val = a.val + b.val := by
  obtain ⟨h1, h2, h3⟩ := IEEE754.two_sum_exact ha hb hs hbv (by decide)
  exact ⟨_, _, two_sum_eq a b, h1, h2, h3⟩

/-- **`split` is Veltkamp's split** (σ = 27): for finite nonzero `a` with `SPLITTER ⊗ a` finite
and `a` a multiple of `2^lam`, the shipped `split` returns finite `(hi, lo)` with the properties
`IEEE754.Split` states: `hi + lo = a`, `hi` on the grid `2^(ε+27)`, `lo` on `2^ε`, their sizes,
and both on every grid of `a`. -/
theorem split_veltkamp (a : F64) {lam : ℤ} (ha : a.isFinite) (hca : (FP.mul splitter a).isFinite)
    (ha0 : a.val ≠ 0) (hlam : Mult a.val lam) :
    ∃ hi lo, predicates.split a = ok (hi, lo) ∧ hi.isFinite ∧ lo.isFinite ∧
      ∃ ε, Split binary64 27 a.val hi.val lo.val ε lam := by
  obtain ⟨h1, h2, h3⟩ := IEEE754.veltkamp (f := binary64) (σ := 27) (by norm_num) (by decide)
    splitter_spec.1 (by rw [splitter_spec.2]) ha hca ha0 hlam
  exact ⟨_, _, split_eq a, h1, h2, h3⟩

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
  obtain ⟨h1, h2, h3⟩ := IEEE754.two_product_exact (f := binary64) (σ := 27) (by decide)
    (by norm_num) splitter_spec.1 (by rw [splitter_spec.2]) ha hb hca hcb hla hlb
    (by simpa using hΛ) (by simpa using hP)
  exact ⟨_, _, two_product_eq a b, h1, h2, h3⟩

end ForgeMath
