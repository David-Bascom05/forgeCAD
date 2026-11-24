import Lean
import ForgeCertify.Frames

/-!
# Only the standard axioms

The build fails if the main theorems rest on an axiom other than `propext`, `Classical.choice`
and `Quot.sound` (`sorry` is the axiom `sorryAx`, so it fails too). Part of the default build.
-/

open Lean Elab Command

elab "#certify_axioms " ids:ident* : command => do
  for id in ids do
    let n ← liftCoreM (realizeGlobalConstNoOverloadWithInfo id)
    let axs ← liftCoreM (collectAxioms n)
    for a in axs do
      unless a == ``propext || a == ``Classical.choice || a == ``Quot.sound do
        throwError "{n} rests on the axiom {a}"
    logInfo m!"{n}: {axs.toList}"

#certify_axioms
  ForgeCertify.check_sound
  ForgeCertify.check_sound_bool
  ForgeCertify.check_sound_xyz
  ForgeCertify.check_sound_axis
  ForgeCertify.at_generic
  ForgeCertify.gen_of_walls
  ForgeCertify.constancy
  ForgeCertify.tele
