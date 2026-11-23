import ForgeTopo.Strays
import ForgeTopo.Links
import ForgeTopo.ShellFaces
import ForgeTopo.Edges

/-! The refinement theorems rest on the standard axioms only: the build fails otherwise. -/

/-- info: 'forge_topo.strays_refines' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms forge_topo.strays_refines

/-- info: 'forge_topo.links_refines' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms forge_topo.links_refines

/-- info: 'forge_topo.shell_faces_refines' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms forge_topo.shell_faces_refines

/-- info: 'forge_topo.edges_refines' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms forge_topo.edges_refines

/-- info: 'forge_topo.loops_refines' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms forge_topo.loops_refines
