import ForgeSketch

/-! The claims rest on Lean's standard axioms only: `lake env lean Axioms.lean` prints each
theorem's axioms; none may be `sorryAx` or `Lean.ofReduceBool`. (`orient2d` is an `opaque`
constant, not an axiom: what the theorems need of it is their hypothesis `OrientOk`.) -/

#print axioms ForgeSketch.le_min
#print axioms ForgeSketch.le_max
#print axioms ForgeSketch.line_spec
#print axioms ForgeSketch.line_int
#print axioms ForgeSketch.mid_err
#print axioms ForgeSketch.proj_err
