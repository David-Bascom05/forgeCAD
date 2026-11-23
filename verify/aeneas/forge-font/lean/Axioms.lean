import ForgeFont

/-! The claims rest on Lean's standard axioms only: `lake env lean Axioms.lean` prints each
theorem's axioms; none may be `sorryAx` or `Lean.ofReduceBool`. -/

#print axioms ForgeFont.mid_spec
#print axioms ForgeFont.step_spec
#print axioms ForgeFont.at_spec
#print axioms ForgeFont.loop0_spec
#print axioms ForgeFont.loop1_spec
#print axioms ForgeFont.contour_spec
#print axioms ForgeFont.contour_closes
#print axioms ForgeFont.coords_spec
#print axioms ForgeFont.read_simple_spec
#print axioms ForgeFont.contour_of_read
