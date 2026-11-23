/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Lean
import ForgeMath.Refine
import ForgeMath.Predicates

/-!
# Only the standard axioms

`#standard_axioms T₁ T₂ …` fails the build if a named theorem rests on any axiom other than
`propext`, `Classical.choice` and `Quot.sound` (a `sorry` is `sorryAx` and fails too). The
translation's opaque functions are Lean `opaque` constants, not axioms, and the float bridge's
operations are definitions, so neither shows up here: the check covers the translated code, the
bridge and the proofs together. (`decide +kernel` adds no axiom; `native_decide` would add
`Lean.ofReduceBool` and fail here.)
-/

open Lean Elab Command

elab "#standard_axioms " ids:ident* : command => do
  for id in ids do
    let n ← liftCoreM (realizeGlobalConstNoOverloadWithInfo id)
    let axs ← liftCoreM (collectAxioms n)
    for a in axs do
      unless a == ``propext || a == ``Classical.choice || a == ``Quot.sound do
        throwError "{n} rests on the axiom {a}"

#standard_axioms
  ForgeMath.two_sum_eq ForgeMath.split_eq ForgeMath.two_product_eq
  ForgeMath.two_sum_exact ForgeMath.split_veltkamp ForgeMath.two_product_exact
  ForgeMath.CROSS_BOUND_eq ForgeMath.CCW_BOUND_eq
  ForgeMath.cross_sign_of_fast ForgeMath.cross_sign_fast
  ForgeMath.orient2d_of_fast ForgeMath.orient2d_fast
