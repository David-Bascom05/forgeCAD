import ForgeTopo.Strays
import ForgeTopo.Links

/-! The refinement theorems rest on the standard axioms only: the build fails otherwise. -/

/-- info: 'forge_topo.strays_refines' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms forge_topo.strays_refines

/-- info: 'forge_topo.links_refines' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms forge_topo.links_refines
