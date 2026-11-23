import ForgeBoolCore.RefineCore
import ForgeBoolCore.Encode

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

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Sample ForgeExact.DecideOnce.Sampler ForgeExact.Ref in
/-- **The sampler's ledger, encoded, through the Rust core.** For every flat input whose sampler
ledger fits `u32` ids (`LFits`), the Rust ledger `encode` builds from it reads back as it, and
`forge_bool_core::core` on it answers the exact regularised boolean. -/
theorem end_to_end_encode (hS : SplitOkSpec) (hK : CollapseSpec) (hU : UnifySpec)
    (hR : ResultBridge) (hE : BadEdgeBridge) (hV : BadVertBridge)
    (ps : List Plane) (ss : List Solid) (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hi : FlatInput ps ss)
    (hfit : LFits (sampleLedger ps ss (sampler ps))) (r : forge_bool_core.Rule) :
    core (encode (sampleLedger ps ss (sampler ps))) r ⟨alloc.vec.Vec.new CellId, alloc.vec.Vec.new CellId⟩ ⦃ o =>
      ∃ ext,
      (∀ b, outcomeL (encode (sampleLedger ps ss (sampler ps))) ext o = .ok b →
        ForgeExact.DecideOnce.Valid (sampleLedger ps ss (sampler ps))
          ((sampleLedger ps ss (sampler ps)).keepTab (ruleL (sampleLedger ps ss (sampler ps)).nb r)) b ∧
        (∀ φ, (∃ F ∈ b.faces, φ ∈ F.facets) ↔
          XBoundary (sampleLedger ps ss (sampler ps)) ps (brepA ss) (ruleL (sampleLedger ps ss (sampler ps)).nb r) φ) ∧
        reg ((decomp (flat_exact hnz hi)).kept fun T =>
            xkeep (sampleLedger ps ss (sampler ps)) ps (brepA ss) (ruleL (sampleLedger ps ss (sampler ps)).nb r) T = true) =
          reg (combine (ruleOp (ruleL (sampleLedger ps ss (sampler ps)).nb r))
            (opnds (sampleLedger ps ss (sampler ps)) (brepA ss)))) ∧
      ((∃ x, outcomeL (encode (sampleLedger ps ss (sampler ps))) ext o = .refused (.zeroThickness x)) ↔
        XNonManifold (sampleLedger ps ss (sampler ps)) ps (brepA ss) (ruleL (sampleLedger ps ss (sampler ps)).nb r)) ⦄ := by
  have hL := toL_encode hfit
  have hnb : (encode (sampleLedger ps ss (sampler ps))).nb.val = (sampleLedger ps ss (sampler ps)).nb := by
    have := congrArg ForgeExact.DecideOnce.Ledger.nb hL; simpa [toL] using this
  -- the sampler's edges are a `dedup`: distinct by construction
  have hnd : (sampleLedger ps ss (sampler ps)).edges.Nodup := ForgeExact.DecideOnce.nodup_dedup _
  have := end_to_end hS hK hU hR hE hV ps ss hnz hi _ r hL (encode_wf hfit hnd)
  rw [hnb] at this
  exact this

end forge_bool_core
