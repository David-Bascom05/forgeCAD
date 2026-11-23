import ForgeBoolCore.EndToEnd
import ForgeBoolCore.BridgeTouch

/-!
# Integration: the groups' proofs plugged into the end-to-end theorem

Each group's `…_proof` replaces a hypothesis of `end_to_end` / `end_to_end_encode` as it lands.
Landed: R (`result_bridge_proof`, `bad_edge_bridge_proof`, `bad_vert_bridge_proof`).
Open: S (`SplitOkSpec`), K (`CollapseSpec`), U (`UnifySpec`).
-/

namespace forge_bool_core

/-- **End to end, with the bridges proven**: what remains is `split_ok`, `collapse` and `unify`. -/
theorem end_to_end_SKU (hS : SplitOkSpec) (hK : CollapseSpec) (hU : UnifySpec) : EndToEnd :=
  end_to_end hS hK hU result_bridge_proof bad_edge_bridge_proof bad_vert_bridge_proof

end forge_bool_core
