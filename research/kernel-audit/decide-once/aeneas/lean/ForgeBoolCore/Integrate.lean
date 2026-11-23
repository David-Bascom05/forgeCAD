import ForgeBoolCore.EndToEnd
import ForgeBoolCore.BridgeTouch
import ForgeBoolCore.RefineSplitOk
import ForgeBoolCore.RefineCollapse

/-!
# Integration: the groups' proofs plugged into the end-to-end theorem

Each group's `…_proof` replaces a hypothesis of `end_to_end` / `end_to_end_encode` as it lands.
Landed: R (`result_bridge_proof`, `bad_edge_bridge_proof`, `bad_vert_bridge_proof`), S (`split_ok_spec_proof`), K (`collapse_spec_proof`).
Open: U (`UnifySpec`).
-/

namespace forge_bool_core

/-- **End to end, with the bridges proven**: what remains is `split_ok`, `collapse` and `unify`. -/
theorem end_to_end_SKU (hS : SplitOkSpec) (hK : CollapseSpec) (hU : UnifySpec) : EndToEnd :=
  end_to_end hS hK hU result_bridge_proof bad_edge_bridge_proof bad_vert_bridge_proof

/-- **End to end, with S and R proven**: what remains is `collapse` and `unify`. -/
theorem end_to_end_KU (hK : CollapseSpec) (hU : UnifySpec) : EndToEnd :=
  end_to_end_SKU split_ok_spec_proof hK hU

/-- **End to end, with S, K and R proven**: what remains is `unify`. -/
theorem end_to_end_U (hU : UnifySpec) : EndToEnd :=
  end_to_end_KU collapse_spec_proof hU

end forge_bool_core
