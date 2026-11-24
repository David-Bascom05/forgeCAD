import ForgeBoolApply.Keep
import ForgeBoolApply.ApplySense

/-! The theorems the ledger cites use only the standard axioms: the build fails otherwise. -/

/-- info: 'forge_bool.keep_of_refines' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms forge_bool.keep_of_refines

/-- info: 'forge_bool.keep_of_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms forge_bool.keep_of_spec

/-- info: 'ForgeBoolApply.apply_sense' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeBoolApply.apply_sense

/-- info: 'ForgeBoolApply.apply_writes' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms ForgeBoolApply.apply_writes
