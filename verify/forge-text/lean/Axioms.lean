import ForgeText

/-! The claims rest on Lean's standard axioms only: `lake env lean Axioms.lean` prints each
theorem's axioms; none may be `sorryAx` or `Lean.ofReduceBool`. -/

#print axioms ForgeText.Seg.reverse_reverse
#print axioms ForgeText.Seg.start_reverse
#print axioms ForgeText.Seg.stop_reverse
#print axioms ForgeText.Seg.start_place
#print axioms ForgeText.Seg.stop_place
#print axioms ForgeText.Seg.place_kind
#print axioms ForgeText.step_out_prefix
#print axioms ForgeText.mem_textRegion
