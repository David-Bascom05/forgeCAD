/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Lean
import ForgeNum.Funs
import ForgeNum.Refine

/-!
# No axiom in forge-num's translation

`#standard_axioms D₁ D₂ …` fails the build if a named declaration rests on any axiom other than
`propext`, `Classical.choice` and `Quot.sound`. The opaque items (FunsExternal.lean,
TypesExternal.lean) are definitions or Lean `opaque` constants. List the translated functions and
every theorem about them here as they are proved.
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
  forge_num.shewchuk.two_sum forge_num.shewchuk.two_diff forge_num.shewchuk.split
  forge_num.shewchuk.two_product
  forge_num.int.Big.add forge_num.int.Big.sub forge_num.int.Big.mul forge_num.dy.Dy.add
  ForgeNum.two_sum_eq ForgeNum.split_eq ForgeNum.two_product_eq
  ForgeNum.two_sum_exact ForgeNum.split_veltkamp ForgeNum.two_product_exact
