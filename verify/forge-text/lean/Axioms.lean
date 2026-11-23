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
#print axioms ForgeText.chain_append
#print axioms ForgeText.step_inv
#print axioms ForgeText.decode_closes
#print axioms ForgeText.contourOf_closes
#print axioms ForgeText.total_append
#print axioms ForgeText.sides_rot1
#print axioms ForgeText.polyWinding_rot1
#print axioms ForgeText.winding_from_any_start
#print axioms ForgeText.refuses_meeting
#print axioms ForgeText.orient_toQ
#print axioms ForgeText.onSeg_of_within
#print axioms ForgeText.lineCross_eq
#print axioms ForgeText.sides_toQ
#print axioms ForgeText.polyWinding_eq
#print axioms ForgeText.winding_is_topological
#print axioms ForgeText.lineCross_toQ
