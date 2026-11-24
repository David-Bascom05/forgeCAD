import TopoCheck.Proof
import TopoCheck.Flood
import TopoCheck.Known

/-! The theorems the README cites use only the standard axioms: the build fails otherwise. -/

/-- info: 'TopoCheck.Body.check_sound' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms TopoCheck.Body.check_sound

/-- info: 'TopoCheck.Body.one_cycle' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms TopoCheck.Body.one_cycle

/-- info: 'TopoCheck.Body.reached_iff' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms TopoCheck.Body.reached_iff

/-- info: 'TopoCheck.known_topo' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms TopoCheck.known_topo
