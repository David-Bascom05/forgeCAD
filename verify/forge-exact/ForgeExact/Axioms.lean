import Lean
import ForgeExact.Regular

/-!
# Only the standard axioms

`#standard_axioms T₁ T₂ …` fails the build if any named theorem rests on an axiom other than
`propext`, `Classical.choice` and `Quot.sound` (`sorry` is the axiom `sorryAx`, so it fails too).
Every main theorem of the package is listed here, and this module is part of the default build.
-/

open Lean Elab Command

elab "#standard_axioms " ids:ident* : command => do
  for id in ids do
    let n ← liftCoreM (realizeGlobalConstNoOverloadWithInfo id)
    let axs ← liftCoreM (collectAxioms n)
    for a in axs do
      unless a == ``propext || a == ``Classical.choice || a == ``Quot.sound do
        throwError "{n} rests on the axiom {a}"
    logInfo m!"{n}: {axs.toList}"

#standard_axioms
  ForgeExact.reg_sub_of_diff_sub
  ForgeExact.reg_eq_of_sdiff
  ForgeExact.reg_kept
  ForgeExact.reg_kept_sampled
  ForgeExact.reg_kept_bool
  ForgeExact.reg_reg
