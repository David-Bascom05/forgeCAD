import ForgeBoolCore.EndToEnd
import ForgeBoolCore.BridgeTouch
import ForgeBoolCore.RefineSplitOk
import ForgeBoolCore.RefineCollapse
import ForgeBoolCore.RefineUnify

/-!
# Integration: the groups' proofs plugged into the end-to-end theorem

Each group's `…_proof` replaces a hypothesis of `end_to_end` / `end_to_end_encode` as it lands.
Landed: R (`result_bridge_proof`, `bad_edge_bridge_proof`, `bad_vert_bridge_proof`), S (`split_ok_spec_proof`), K (`collapse_spec_proof`), U (`unify_spec'`).
Nothing is open: `end_to_end_proof` and `end_to_end_encode_proof` carry no hypothesis.
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

/-- `unify` refines `Ledger.unify` (group U's `unify_spec'`, its two extra premises supplied by
`AfterSplit`: distinct edge cells and `splitOk`). -/
theorem unify_spec_proof : UnifySpec := by
  intro l e0 s ext keep a isE ae av hA hk hasm hne hmk hae hav hE hV
  exact unify_spec' result_bridge_proof around_bridge_proof l e0 s ext keep a isE ae av hA hk hasm hne hmk
    hae hav hE hV hA.wf.edges_nodup hA.ok

/-- **End to end, proven.** On every Rust ledger that reads as the sampler's ledger of a flat
input, `forge_bool_core::core` answers the exact regularised boolean. -/
theorem end_to_end_proof : EndToEnd := end_to_end_U unify_spec_proof

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Sample ForgeExact.DecideOnce.Sampler ForgeExact.Ref in
/-- **End to end on the encoded sampler ledger, proven**: for every flat input whose sampler ledger
fits `u32` ids, the Rust core on its encoding answers the exact regularised boolean. -/
def end_to_end_encode_proof (ps : List Plane) (ss : List Solid) (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩)
    (hi : FlatInput ps ss) (hfit : LFits (sampleLedger ps ss (sampler ps))) (r : forge_bool_core.Rule) :=
  end_to_end_encode split_ok_spec_proof collapse_spec_proof unify_spec_proof result_bridge_proof
    bad_edge_bridge_proof bad_vert_bridge_proof ps ss hnz hi hfit r

end forge_bool_core
