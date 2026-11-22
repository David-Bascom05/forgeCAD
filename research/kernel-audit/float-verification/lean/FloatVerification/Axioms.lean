/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Lean
import FloatVerification.CrossSign
import FloatVerification.Orient2d

/-!
# Only the standard axioms

The theorems about the translated kernel code rest only on `propext`, `Classical.choice` and
`Quot.sound`. The build fails otherwise, and in particular on a `sorry`. The f64 operations are
definitions (FvPredicates/FunsExternal.lean), not axioms. The counterexample
(`Counterexample.lean`) is checked by evaluation and is deliberately not listed.
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
  FloatVerification.cross_sign_of_fast FloatVerification.cross_sign_fast
  FloatVerification.shewchuk FloatVerification.orient2d_of_fast FloatVerification.orient2d_fast
  FloatVerification.finite_of_bound_slack
