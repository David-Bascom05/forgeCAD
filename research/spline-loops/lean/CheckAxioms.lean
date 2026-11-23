import Lean
import SplineSpec
import SplineLoops
open Lean Elab Command

/-! `lake env lean CheckAxioms.lean`: every theorem in a `SplineSpec` or `SplineLoops` module, not only the ones
`Axioms.lean` names, may use only `propext`, `Classical.choice` and `Quot.sound` (so no `sorryAx`,
no `Lean.ofReduceBool` from `native_decide`, no added axiom). Fails, with the offenders, otherwise.
CI runs it after `lake build`. -/

elab "#check_all_axioms" : command => do
  let env ← getEnv
  let allowed : List Name := [``propext, ``Classical.choice, ``Quot.sound]
  let mut n := 0
  let mut bad : Array String := #[]
  for (nm, ci) in env.constants.toList do
    let some idx := env.getModuleIdxFor? nm | continue
    let mod := env.header.moduleNames[idx.toNat]!
    unless (`SplineSpec).isPrefixOf mod || (`SplineLoops).isPrefixOf mod do continue
    unless ci matches .thmInfo _ do continue
    n := n + 1
    let axs ← liftCoreM <| Lean.collectAxioms nm
    for a in axs do
      unless allowed.contains a do bad := bad.push s!"{nm}: {a}"
  if bad.isEmpty then
    logInfo m!"{n} theorems, axioms only propext, Classical.choice, Quot.sound"
  else
    logError m!"{bad.size} uses of other axioms:\n{String.intercalate "\n" bad.toList}"

#check_all_axioms
