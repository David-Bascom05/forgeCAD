import ForgeBoolApply.Keep

/-! The theorems the ledger cites use only the standard axioms: the build fails otherwise. -/

/-- info: 'forge_bool.keep_of_refines' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms forge_bool.keep_of_refines

/-- info: 'forge_bool.keep_of_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms forge_bool.keep_of_spec
