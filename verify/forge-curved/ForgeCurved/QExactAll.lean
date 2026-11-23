import ForgeCurved.QExact
import ForgeCurved.QEdgesVerts

/-!
# `curved_exact`: the curved ledger is exact on certified inputs

Every `ExactK` field, `edges` and `verts` included, over ℝ, for every **certified** input.

* `facets_negB`: every listed facet is a facet on `B`'s negative side;
* `qledger_frame`: the ledger is the frame's, with (U) on every listed vertex;
* `vert_frame_iff`, `edge_frame_iff`: vertex and edge cells are the same in the frame;
* `curved_edges_verts`: `edges` and `verts` on the input carriers (`edges_frame`, `verts_frame`);
* **`curved_exact`**: all the fields, and the bodies are the input's.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

/-- **Every listed facet lies on `B`'s negative side.** -/
theorem facets_negB {qs : List Quad.Quadric} {forms : List SForm} {R2 : ℚ} {L : Ledger}
    (h : qledgerB qs forms R2 = some L) (hb : ∀ F ∈ forms, F.Below qs.length) :
    ∀ φ ∈ L.facets, φ[qs.length]? = some Sgn.neg ∧ φ.isFacet = true := by
  intro φ0 hφ0
  set Q := qs ++ [sphereR R2]
  set n := qs.length
  have hcs : carriersOf realModel Q = Q.map qR := carriersOf_real Q
  obtain ⟨ss, -, hL⟩ := qledger_samples (qledgerB_parts h)
  unfold Ledger.facets at hφ0
  obtain ⟨⟨φ, k⟩, hpL, rfl⟩ := List.mem_map.mp ((mem_dedup _ _).mp hφ0)
  have hp := hpL
  rw [hL] at hp
  obtain ⟨i, hi, -, -, hfac, hne⟩ := curvedLedger_pieces hp
  have hi' : i < forms.length := by simpa [boundForms] using hi
  refine ⟨?_, hfac⟩
  have hF : formOf (boundForms n forms) i = .and (formOf forms i) (.lit n .neg) := by
    simp [formOf, boundForms, List.getD_eq_getElem?_getD, hi']
  rw [hF] at hne
  simp only [SForm.onCov] at hne
  have hreal : RealisedK (Q.map qR) φ := by
    have := qledger_pieceReal realModel (qledgerB_parts h) _ hpL
    rw [hcs] at this; exact this
  have hlenQ : Q.length = n + 1 := by simp [Q, n]
  have hφlen : φ.length = n + 1 := by obtain ⟨x, hx⟩ := hreal; rw [← hx]; simp [covOfK, hlenQ]
  obtain ⟨σ, hσ⟩ : ∃ σ, φ[n]? = some σ := ⟨φ[n], List.getElem?_eq_getElem (by omega)⟩
  have hbn : n < (Q.map qR).length := by simp [hlenQ]
  by_cases hjn : φ.zeroIdx = n
  · exfalso
    have e1 : φ.plus[n]? = some Sgn.pos := by
      show (φ.set φ.zeroIdx Sgn.pos)[n]? = _
      rw [hjn, List.getElem?_set_self (by omega)]
    have e : φ.minus = φ.set n Sgn.neg := by
      show φ.set φ.zeroIdx Sgn.neg = _; rw [hjn]
    have e2 : φ.minus[n]? = some Sgn.neg := by rw [e, List.getElem?_set_self (by omega)]
    rw [e1, e2] at hne
    have hmin : (formOf forms i).onCov φ.minus = true := by
      cases hm : (formOf forms i).onCov φ.minus <;> simp_all
    have hb' : (formOf forms i).Below n := by
      simp only [formOf, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi',
        Option.getD_some]
      exact hb _ (List.getElem_mem hi')
    have hon : (formOf forms i).onCov φ = true := by
      rw [e, SForm.onCov_set_ge _ hb' φ n Sgn.neg (Nat.le_refl _)] at hmin
      exact hmin
    obtain ⟨y, hy⟩ := hreal
    have hy' : (formOf forms i).den (Q.map qR) y := ((formOf forms i).den_cell hy).mpr hon
    have hyq : (formOf forms i).den (carriersOf realModel qs) y := by
      have hb'' : (formOf forms i).Below (carriersOf realModel qs).length := by
        simpa [carriersOf] using hb'
      rw [← SForm.den_append (carriersOf realModel qs) [Quadric.evK realModel (sphereR R2)] y _ hb'']
      have : carriersOf realModel qs ++ [Quadric.evK realModel (sphereR R2)] = Q.map qR := by
        rw [← hcs]; simp [carriersOf, Q]
      rw [this]; exact hy'
    have hin := qledgerB_inside realModel h hi' hyq
    rw [evK_real] at hin
    have hzn : φ[n]? = some Sgn.zero := by rw [← hjn]; exact facet_zero hfac
    have := sgn_at hy hbn hzn
    have hQn : (Q.map qR)[n] = qR (sphereR R2) := by simp [Q, n]
    rw [hQn, sgnK_zero_iff] at this
    linarith
  · rcases σ with _ | _ | _
    · exact hσ
    · exact absurd hσ (facet_unique hfac n (Ne.symm hjn))
    · exfalso
      have e1 : φ.plus[n]? = some Sgn.pos := by
        show (φ.set φ.zeroIdx Sgn.pos)[n]? = _
        rw [List.getElem?_set_ne hjn]; exact hσ
      have e2 : φ.minus[n]? = some Sgn.pos := by
        show (φ.set φ.zeroIdx Sgn.neg)[n]? = _
        rw [List.getElem?_set_ne hjn]; exact hσ
      rw [e1, e2] at hne
      rw [show (some Sgn.pos == some Sgn.neg) = false from rfl] at hne
      simp at hne

/-- **The ledger is its frame's**, with (U) on every listed vertex. -/
theorem qledger_frame {qs : List Quad.Quadric} {forms : List SForm} {L : Ledger}
    (h : qledger qs forms = some L) :
    ∃ f ss, samplesOfF qs = some (f, ss) ∧ L = curvedLedger qs.length forms ss ∧
      ∀ v ∈ L.verts, vertOneAt (qs.map (inFrame f)) v = true := by
  unfold qledger at h
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at h
  obtain ⟨⟨f, ss⟩, hss, h⟩ := h
  dsimp only at h
  split at h
  · rename_i hc
    cases h
    simp only [Bool.and_eq_true, List.all_eq_true] at hc
    exact ⟨f, ss, hss, rfl, hc.2⟩
  · cases h

theorem frameInv_T (f : Frame) (p : R3) : frameInv f (frameT f p) = p := by
  obtain ⟨x, y, z⟩ := p
  simp only [frameT, frameInv, Prod.mk.injEq]; refine ⟨by ring, by ring, trivial⟩

theorem cell_frame (f : Frame) (qs : List Quad.Quadric) (X : Cov) (p : R3) :
    covCellK ((qs.map (inFrame f)).map qR) X p ↔ covCellK (qs.map qR) X (frameT f p) := by
  show covOfK _ _ = X ↔ covOfK _ _ = X
  rw [cov_frame]

theorem realised_frame_iff (f : Frame) (qs : List Quad.Quadric) (X : Cov) :
    RealisedK ((qs.map (inFrame f)).map qR) X ↔ RealisedK (qs.map qR) X :=
  ⟨realised_unframe f, realised_frame f⟩

theorem vert_frame_iff (f : Frame) (qs : List Quad.Quadric) (X : Cov) :
    IsVertK ((qs.map (inFrame f)).map qR) X ↔ IsVertK (qs.map qR) X := by
  constructor
  · rintro ⟨hr, hu⟩
    refine ⟨realised_unframe f hr, fun x y hx hy => ?_⟩
    have hx' := (cell_frame f qs X (frameInv f x)).mpr (by rw [frameT_inv]; exact hx)
    have hy' := (cell_frame f qs X (frameInv f y)).mpr (by rw [frameT_inv]; exact hy)
    rw [← frameT_inv f x, ← frameT_inv f y, hu _ _ hx' hy']
  · rintro ⟨hr, hu⟩
    refine ⟨realised_frame f hr, fun x y hx hy => ?_⟩
    have := hu _ _ ((cell_frame f qs X x).mp hx) ((cell_frame f qs X y).mp hy)
    rw [← frameInv_T f x, ← frameInv_T f y, this]

theorem edge_frame_iff (f : Frame) (qs : List Quad.Quadric) (X : Cov) :
    IsEdgeK ((qs.map (inFrame f)).map qR) X ↔ IsEdgeK (qs.map qR) X := by
  unfold IsEdgeK
  rw [realised_frame_iff, vert_frame_iff]

/-- **`ExactK.edges` and `ExactK.verts` on certified inputs.** -/
theorem curved_edges_verts {qs : List Quad.Quadric} {forms : List SForm} {R2 : ℚ} {L : Ledger}
    (hL : qledgerB qs forms R2 = some L) (hb : ∀ F ∈ forms, F.Below qs.length) :
    let cs := carriersOf realModel (qs ++ [sphereR R2])
    (∀ e, e ∈ L.edges ↔ RealisedK cs e ∧ IsEdgeK cs e ∧ ∃ φ ∈ L.facets, Cov.le e φ = true) ∧
    (∀ v, v ∈ L.verts ↔ RealisedK cs v ∧ IsVertK cs v ∧ ∃ φ ∈ L.facets, Cov.le v φ = true) := by
  intro cs
  set Q := qs ++ [sphereR R2]
  set n := qs.length
  have hcs : cs = Q.map qR := carriersOf_real Q
  rw [hcs]
  have hφB := facets_negB hL hb
  obtain ⟨f, ss, hss, hLeq, hU⟩ := qledger_frame (qledgerB_parts hL)
  obtain ⟨-, hf, hsing, hat⟩ := samplesOfF_spec hss
  set Qf := Q.map (inFrame f)
  have hlen : Qf.length = Q.length := List.length_map _
  have hLf : L = curvedLedger Qf.length (boundForms n forms) ss := by rw [hlen]; exact hLeq
  have hbi : n < (Qf.map qR).length := by simp [Qf, Q, n]
  have hB : Bornology.IsBounded {z | (Qf.map qR)[n] z ≤ 0} := by
    have e : (Qf.map qR)[n] = fun z => qR (sphereR R2) (frameT f z) := by
      funext z; simp [Qf, Q, n, qR_frame]
    rw [e]
    exact bounded_frame f (bounded_sphereR R2)
  rw [hLf] at hφB hU
  have hE := edges_frame hf hsing hat hbi hB hφB hU
  have hV := verts_frame hf hsing hat hbi hB hφB hU
  rw [hLf]
  refine ⟨fun e => ?_, fun v => ?_⟩
  · rw [hE e, realised_frame_iff, edge_frame_iff]
  · rw [hV v, realised_frame_iff, vert_frame_iff]

/-- **The curved ledger is exact on certified inputs**: every `ExactK` field, and the bodies are
the input's. -/
theorem curved_exact {qs : List Quad.Quadric} {forms : List SForm} {R2 : ℚ}
    (hc : certified qs forms R2 = true) :
    ∃ L, qledgerB qs forms R2 = some L ∧
      ExactK L (carriersOf realModel (qs ++ [sphereR R2]))
        (bodiesOf realModel (qs ++ [sphereR R2]) (boundForms qs.length forms)) ∧
      ∀ i, i < forms.length → ∀ p,
        bodiesOf realModel (qs ++ [sphereR R2]) (boundForms qs.length forms) i p ↔
          (formOf forms i).den (carriersOf realModel qs) p := by
  obtain ⟨L, hL, hw, hcomp, hpf, hof, hpr, hwit, hbd, hcpl, hbod⟩ := curved_exact_facets hc
  have hb : ∀ F ∈ forms, F.Below qs.length := by
    simp only [certified, Bool.and_eq_true, List.all_eq_true] at hc
    exact fun F hF => belowB_spec F (hc.2 F hF)
  obtain ⟨hE, hV⟩ := curved_edges_verts hL hb
  exact ⟨L, hL, ⟨hw, hcomp, hpf, hof, hpr, hwit, hbd, hcpl, hE, hV⟩, hbod⟩

end ForgeCurved
