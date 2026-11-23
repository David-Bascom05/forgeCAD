import ForgeBoolCore.RefineSplit
open Aeneas Aeneas.Std Result

/-!
# The Rust ledger as the Lean core's

`toL l` is the ledger `ForgeExact.DecideOnce.core` reads, ids replaced by their covectors. Over it,
`split`'s tables are the Lean's: facets (`facets_toL`), sides (`side_toL`), topes, and the piece
table (`piece_toL`).
-/

namespace forge_bool_core

open ForgeExact.DecideOnce ForgeBoolCore.Dedup

/-- A ledger cell's covector, as the Lean's. -/
def cellL (l : Ledger) (id : Nat) : Cov := ((l.cells.val.getD id default).val).map toS

/-- **The ledger the Lean core reads.** -/
def toL (l : Ledger) : ForgeExact.DecideOnce.Ledger where
  nb := l.nb.val
  faceBody := l.face_body.val.map fun b => (b : U32).val
  faceCls := l.face_class.val.map (·.val)
  pieces := (pcs l).map fun q => (cellL l q.1, q.2)
  inside := l.inside.val.map fun q => (cellL l (q.1 : U32).val, (q.2 : U32).val)
  edges := l.edges.val.map fun c => cellL l (c : U32).val
  verts := l.verts.val.map fun c => cellL l (c : U32).val

theorem map_toS_inj {a b : List Sgn} (h : a.map toS = b.map toS) : a = b :=
  List.map_injective_iff.mpr (fun _ _ e => toS_inj e) h

theorem cellL_covAt (l : Ledger) (e : Ext) {id : Nat} (h : id < l.cells.val.length) :
    cellL l id = (covAt l e id).map toS := by
  rw [covAt_left h]; simp [cellL, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]

/-- No two ledger cells share a covector, so `cellL` is injective on ids. -/
theorem cellL_inj (l : Ledger) (e : Ext) (hok : ExtOK l e) {a b : Nat} (ha : a < l.cells.val.length)
    (hb : b < l.cells.val.length) (h : cellL l a = cellL l b) : a = b := by
  rw [cellL_covAt l e ha, cellL_covAt l e hb] at h
  have h' := map_toS_inj h
  have hl := allCells_length l e
  have hnd := hok.nodup
  rw [covAt_left ha, covAt_left hb] at h'
  have e1 : (allCells l e)[a]'(by omega) = (allCells l e)[b]'(by omega) := by
    simp [allCells, List.getElem_append_left (by simpa using ha : a < (l.cells.val.map (·.val)).length),
      List.getElem_append_left (by simpa using hb : b < (l.cells.val.map (·.val)).length), h']
  exact (List.Nodup.getElem_inj_iff hnd).mp e1

/-- **The facets are the Lean's.** -/
theorem facets_toL (l : Ledger) (e : Ext) (hok : ExtOK l e)
    (hpc : ∀ q ∈ pcs l, q.1 < l.cells.val.length) (F : List U32)
    (hF : F.map (·.val) = dedup ((pcs l).map Prod.fst)) :
    F.map (fun x => cellL l x.val) = (toL l).facets := by
  unfold ForgeExact.DecideOnce.Ledger.facets toL
  simp only [List.map_map]
  have : ((pcs l).map (Prod.fst ∘ fun q => (cellL l q.1, q.2))) = ((pcs l).map Prod.fst).map (cellL l) := by
    simp [List.map_map]
  rw [this, dedup_map (cellL l) _ (fun a ha b hb h => by
    obtain ⟨qa, hqa, rfl⟩ := List.mem_map.mp ha
    obtain ⟨qb, hqb, rfl⟩ := List.mem_map.mp hb
    exact cellL_inj l e hok (hpc qa hqa) (hpc qb hqb) h), ← hF]
  simp [List.map_map]

/-- A side as the Lean's `plus` / `minus`. -/
theorem sideL_map (x : List Sgn) (s : Sgn) :
    (sideL x s).map toS = (x.map toS).set ((x.map toS).idxOf .zero) (toS s) := by
  rw [sideL, List.map_set, idxOf_map_toS]

theorem side_plus (x : List Sgn) : (sideL x .Pos).map toS = Cov.plus (x.map toS) := by
  rw [sideL_map]; rfl

theorem side_minus (x : List Sgn) : (sideL x .Neg).map toS = Cov.minus (x.map toS) := by
  rw [sideL_map]; rfl

/-- Any cell's covector, as the Lean's. -/
def covE (l : Ledger) (e : Ext) (id : Nat) : Cov := (covAt l e id).map toS

theorem covAt_inj (l : Ledger) (e : Ext) (hok : ExtOK l e) {a b : Nat} (ha : a < (allCells l e).length)
    (hb : b < (allCells l e).length) (h : covAt l e a = covAt l e b) : a = b := by
  simp only [covAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem ha, List.getElem?_eq_getElem hb,
    Option.getD_some] at h
  exact (List.Nodup.getElem_inj_iff hok.nodup).mp h

theorem covE_inj (l : Ledger) (e : Ext) (hok : ExtOK l e) {a b : Nat} (ha : a < (allCells l e).length)
    (hb : b < (allCells l e).length) (h : covE l e a = covE l e b) : a = b :=
  covAt_inj l e hok ha hb (map_toS_inj h)

theorem stream_map (l : Ledger) (e0 e : Ext) (F P M : List U32) (hF : ∀ x ∈ F, x.val < l.cells.val.length)
    (hL : L1Inv l e0 F e P M F.length) :
    ∀ k, k ≤ F.length → (stream P M k).map (covE l e) =
      (F.take k).flatMap (fun x => [Cov.plus (cellL l x.val), Cov.minus (cellL l x.val)])
  | 0, _ => by simp [stream]
  | k + 1, hk => by
    have hL' := hL
    obtain ⟨_, _, _, _, _, _, hPM⟩ := hL'
    obtain ⟨_, h2, _, h4⟩ := hPM k (by omega)
    rw [stream, List.map_append, List.map_append, stream_map l e0 e F P M hF hL k (by omega),
      List.take_succ, List.getElem?_eq_getElem (by omega)]
    simp only [Option.toList_some, List.flatMap_append, List.flatMap_singleton, List.map_singleton]
    have hc : (F[k]'(by omega)).val < l.cells.val.length := hF _ (List.getElem_mem _)
    have e1 : F[k]! = F[k]'(by omega) := getElem!_pos F k (by omega)
    rw [e1] at h2 h4
    rw [covE, h2, side_plus, covE, h4, side_minus, cellL_covAt l e0 hc]
    simp

/-- **The topes are the Lean's.** -/
theorem topes_toL (l : Ledger) (e0 : Ext) (s : Split) (e : Ext) (hok0 : ExtOK l e0) (hs : SplitOK l e0 s e)
    (hpc : ∀ q ∈ pcs l, q.1 < l.cells.val.length) :
    s.topes.val.map (fun x => covE l e x.val) = (toL l).topes := by
  have hF : ∀ x ∈ s.facets.val, x.val < l.cells.val.length := by
    intro x hx
    have : x.val ∈ s.facets.val.map (·.val) := List.mem_map.mpr ⟨x, hx, rfl⟩
    rw [hs.facets] at this
    obtain ⟨q, hq, e⟩ := List.mem_map.mp ((mem_dedup _ _).mp this)
    rw [← e]; exact hpc q hq
  unfold ForgeExact.DecideOnce.Ledger.topes
  rw [← facets_toL l e0 hok0 hpc s.facets.val hs.facets, List.flatMap_map]
  have hst := stream_map l e0 e s.facets.val s.plus.val s.minus.val hF hs.sides s.facets.val.length (Nat.le_refl _)
  rw [List.take_length] at hst
  have hT := hs.topes.2.1
  rw [← hst, dedup_map (covE l e) _ (fun a ha b hb h => covE_inj l e hs.ext
    (hs.topes.2.2.1 a ha) (hs.topes.2.2.1 b hb) h), ← hT]
  simp [List.map_map]

theorem bodyOf_toL (l : Ledger) (f : Nat) : (toL l).bodyOf f = bodyF l f := by
  unfold ForgeExact.DecideOnce.Ledger.bodyOf toL bodyF
  simp only [List.getD_eq_getElem?_getD, List.getElem?_map]
  cases l.face_body.val[f]? <;> rfl

/-- **The piece table is the Lean's `isPieceOf`.** -/
theorem piece_toL (l : Ledger) (e0 : Ext) (s : Split) (e : Ext) (hok0 : ExtOK l e0) (hs : SplitOK l e0 s e)
    (hpc : ∀ q ∈ pcs l, q.1 < l.cells.val.length) {p : Nat} (hp : p < s.facets.val.length)
    {b : Nat} (hb : b < l.nb.val) :
    PAt s.piece_of.val p b = (toL l).isPieceOf b (cellL l (s.facets.val[p]'hp).val) := by
  rw [hs.piece p hp b hb]
  unfold ForgeExact.DecideOnce.Ledger.isPieceOf
  simp only [toL, List.any_map]
  have hnd : (s.facets.val.map (·.val)).Nodup := by rw [hs.facets]; exact nodup_dedup _
  have hFp : (s.facets.val[p]'hp).val < l.cells.val.length := by
    have : (s.facets.val[p]'hp).val ∈ s.facets.val.map (·.val) := List.mem_map.mpr ⟨_, List.getElem_mem _, rfl⟩
    rw [hs.facets] at this
    obtain ⟨q, hq, e⟩ := List.mem_map.mp ((mem_dedup _ _).mp this)
    rw [← e]; exact hpc q hq
  apply Bool.eq_iff_iff.mpr
  simp only [List.any_eq_true, decide_eq_true_eq, Function.comp, Bool.and_eq_true, beq_iff_eq]
  apply exists_congr; intro q; apply and_congr_right; intro hq
  rw [← bodyOf_toL]
  apply and_congr_left; intro _
  have hqF : q.1 ∈ s.facets.val.map (·.val) := by
    rw [hs.facets]; exact (mem_dedup _ _).mpr (List.mem_map.mpr ⟨q, hq, rfl⟩)
  constructor
  · intro h
    have : (s.facets.val.map (·.val))[p]'(by simpa using hp) = q.1 := by
      subst h; exact List.getElem_idxOf _
    simp at this; rw [this]
  · intro h
    have e := cellL_inj l e0 hok0 (hpc q hq) hFp h
    rw [e]
    have := List.Nodup.idxOf_getElem hnd p (by simpa using hp)
    simpa using this

end forge_bool_core
