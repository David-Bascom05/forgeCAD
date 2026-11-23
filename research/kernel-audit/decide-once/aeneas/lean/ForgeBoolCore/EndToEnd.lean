import ForgeBoolCore.RefineCore

/-!
# End to end

`coreM` with no thin cells is `core` (`coreM_nil`), so `CoreSpec` and `flat_correct` give
`EndToEnd`: `forge_bool_core::core`'s answer, on a Rust ledger that reads as the sampler's ledger
of a flat input, is the exact regularised boolean.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

open ForgeExact.DecideOnce in
/-- With no thin cells the collapse changes nothing: `coreM` is `core`. -/
theorem coreM_nil (L : ForgeExact.DecideOnce.Ledger) (rule : ForgeExact.DecideOnce.Rule) :
    L.coreM rule [] [] = ForgeExact.DecideOnce.core L rule := by
  have hc : L.collapse (L.keepTab rule) [] [] = .ok (L.keepTab rule) := by
    unfold Ledger.collapse Ledger.thinTopes
    simp only [List.contains_nil, Bool.false_eq_true, ite_false, List.filter_false, List.find?_nil]
    congr 1
    unfold Ledger.keepTab
    apply List.map_congr_left
    intro T hT
    rw [ForgeExact.DecideOnce.Ledger.keepIn_map _ hT]
  unfold Ledger.coreM ForgeExact.DecideOnce.core
  rw [hc]
  rfl

theorem end_to_end_of (hCore : CoreSpec) : EndToEnd := by
  intro ps ss hnz hi l r hL hw
  apply WP.spec_mono (hCore l r ⟨alloc.vec.Vec.new CellId, alloc.vec.Vec.new CellId⟩ hw
    ⟨fun c hc => by simp at hc, fun c hc => by simp at hc⟩)
  rintro o ⟨ext, _, ho⟩
  have hthin : thinL l ⟨alloc.vec.Vec.new CellId, alloc.vec.Vec.new CellId⟩ = [] := by simp [thinL]
  have henc : enclosedL l ⟨alloc.vec.Vec.new CellId, alloc.vec.Vec.new CellId⟩ = [] := by simp [enclosedL]
  rw [hthin, henc, coreM_nil, hL] at ho
  obtain ⟨hok, hzt⟩ := ForgeExact.DecideOnce.Sampler.flat_correct hnz hi (ruleL l.nb.val r)
  refine ⟨ext, fun b hb => hok b (ho ▸ hb), ?_⟩
  rw [ho]; exact hzt

/-- **The end-to-end theorem, from the groups' Props.** Once `split_ok` (S), `collapse` (K),
`unify` (U) and the bridges (R) are proven, `forge_bool_core::core` computes the exact regularised
boolean on every flat input the sampler describes. -/
theorem end_to_end (hS : SplitOkSpec) (hK : CollapseSpec) (hU : UnifySpec)
    (hR : ResultBridge) (hE : BadEdgeBridge) (hV : BadVertBridge) : EndToEnd :=
  end_to_end_of (core_spec_of hS (classify_on_spec_of hK (core_k_spec_of hR hE hV hU)))

end forge_bool_core
