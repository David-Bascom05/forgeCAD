import ForgeCurved.QComplete

/-!
# `curved_exact`: the curved ledger is exact on certified inputs (facet fields)

For every **certified** input (`certified`, a checked Bool: the bounded ledger builds, every carrier
is a nonzero quadric, every body reads only the input carriers), over ℝ, these hold:

* `wallND` (`wallND_q`);
* `compat` (`qform_compat`);
* `pieceFacet`, `oneFace`, `pieceReal`, `witness`, `boundary` (`qledger_*`);
* `complete` (`qledgerB_complete`).

These are `ExactK`'s fields except `edges` and `verts`. Those two need R1 on the listed cells,
through route B's local models (the master's decision of 2026-11-23). The bodies are the input's
(`qledgerB_bodies`).
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Set

theorem nzB_eq (q : Quad.Quadric) : nzB q = nzQ q := rfl

/-- **The curved ledger is exact on certified inputs**: every `ExactK` field but `edges` and
`verts`. -/
theorem curved_exact_facets {qs : List Quad.Quadric} {forms : List SForm} {R2 : ℚ}
    (hc : certified qs forms R2 = true) :
    ∃ L, qledgerB qs forms R2 = some L ∧
      let Q := qs ++ [sphereR R2]
      let cs := carriersOf realModel Q
      let A := bodiesOf realModel Q (boundForms qs.length forms)
      ForgeExact.Topology.NowhereDense (wallK cs) ∧
      (∀ T, FullK cs T → ∀ i, (covCellK cs T).Sub (A i) ∨ ∀ x, covCellK cs T x → ¬A i x) ∧
      (∀ p ∈ L.pieces, p.1.isFacet = true ∧ p.1.zeroIdx = L.clsOf p.2 ∧ L.bodyOf p.2 < L.nb) ∧
      (∀ p ∈ L.pieces, ∀ q ∈ L.pieces, q.1 = p.1 → L.bodyOf q.2 = L.bodyOf p.2 → q.2 = p.2) ∧
      (∀ p ∈ L.pieces, RealisedK cs p.1) ∧
      (∀ T ∈ L.topes, ∀ i, i < L.nb → (L.inside.contains (T, i) = true ↔ memTK cs A T i)) ∧
      (∀ φ ∈ L.facets, ∀ i, i < L.nb → (FlipK cs A φ i ↔ L.isPieceOf i φ = true)) ∧
      (∀ φ, RealisedK cs φ → φ.isFacet = true → ∀ i, i < L.nb → FlipK cs A φ i → φ ∈ L.facets) ∧
      (∀ i, i < forms.length → ∀ p, A i p ↔ (formOf forms i).den (carriersOf realModel qs) p) := by
  simp only [certified, Bool.and_eq_true, Option.isSome_iff_exists, List.all_eq_true] at hc
  obtain ⟨⟨⟨L, hL⟩, hnz⟩, hbel⟩ := hc
  have hb : ∀ F ∈ forms, F.Below qs.length := fun F hF => belowB_spec F (hbel F hF)
  have hq := qledgerB_parts hL
  refine ⟨L, hL, ?_, fun T _ i => qledger_compat realModel T i, qledger_pieceFacet hq,
    qledger_oneFace hq, qledger_pieceReal realModel hq, qledger_witness realModel hq,
    qledger_boundary realModel hq, qledgerB_complete hL hb,
    fun i hi p => qledgerB_bodies realModel hL hb i hi p⟩
  rw [carriersOf_real]
  exact wallND_q (List.all_eq_true.mpr fun q hq => by rw [nzB_eq]; exact hnz q hq)

end ForgeCurved
