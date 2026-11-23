import ForgeBoolCore.RefineSplit
import ForgeBoolCore.HeadBridge
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

/-! ## The flood's tables fit -/

theorem at32_getD {v : alloc.vec.Vec U32} {k : Nat} (h : k < v.val.length) : at32 v k = (v.val[k]).val :=
  at32_eq h

theorem getElem!_u32 {v : List U32} {k : Nat} (h : k < v.length) : v[k]! = v[k] := getElem!_pos v k h

theorem stream_mem (P M : alloc.vec.Vec U32) (n : Nat) (hP : n ≤ P.val.length) (hM : n ≤ M.val.length) :
    ∀ k, k < n → at32 P k ∈ stream P.val M.val n ∧ at32 M k ∈ stream P.val M.val n := by
  intro k hk
  have key : ∀ m, k < m → m ≤ n →
      at32 P k ∈ stream P.val M.val m ∧ at32 M k ∈ stream P.val M.val m := by
    intro m hkm hm
    induction m with
    | zero => omega
    | succ m ih =>
      by_cases e1 : k = m
      · subst e1
        rw [at32_getD (by omega), at32_getD (by omega)]
        simp [stream, getElem!_u32 (show k < P.val.length by omega), getElem!_u32 (show k < M.val.length by omega)]
      · have := ih (by omega) (by omega)
        simp only [stream, List.mem_append]
        exact ⟨Or.inl (Or.inl this.1), Or.inl (Or.inl this.2)⟩
  exact key n hk (Nat.le_refl _)

/-- **`split`'s tables fit `flood`**, for every body. -/
theorem splitWF_of (l : Ledger) (e0 : Ext) (s : Split) (e : Ext) (hs : SplitOK l e0 s e)
    (hroom : 2 * s.facets.val.length < U32.max) {i : Nat} (hi : i < l.nb.val) : SplitWF s i := by
  obtain ⟨_, hPl, hMl, hPM⟩ := hs.sides.2.2.2
  have hT := hs.topes
  have hTl : s.tope_pos.val.length = (allCells l e).length := hT.1
  have hstream : ∀ k, k < s.facets.val.length →
      at32 s.plus k ∈ stream s.plus.val s.minus.val s.facets.val.length ∧
      at32 s.minus k ∈ stream s.plus.val s.minus.val s.facets.val.length := by
    intro k hk
    have key : ∀ m, k < m → m ≤ s.facets.val.length →
        at32 s.plus k ∈ stream s.plus.val s.minus.val m ∧ at32 s.minus k ∈ stream s.plus.val s.minus.val m := by
      intro m hkm hm
      induction m with
      | zero => omega
      | succ m ih =>
        by_cases e1 : k = m
        · subst e1
          rw [at32_getD (by omega), at32_getD (by omega)]
          simp [stream, getElem!_u32 (show k < s.plus.val.length by omega),
            getElem!_u32 (show k < s.minus.val.length by omega)]
        · have := ih (by omega) (by omega)
          simp only [stream, List.mem_append]
          exact ⟨Or.inl (Or.inl this.1), Or.inl (Or.inl this.2)⟩
    exact key s.facets.val.length hk (Nat.le_refl _)
  have hpos : ∀ c ∈ stream s.plus.val s.minus.val s.facets.val.length,
      c < s.tope_pos.val.length ∧ at32 s.tope_pos c < s.topes.val.length := by
    intro c hc
    have hcn := hT.2.2.1 c hc
    refine ⟨by omega, ?_⟩
    have := hT.2.2.2 c hcn
    rw [if_pos hc] at this
    have hl := tinv_small hT
    have hlt := List.idxOf_lt_length_iff.mpr ((mem_dedup _ _).mpr hc)
    have e2 : at32 s.tope_pos c = (s.tope_pos.val.getD c NONE).val := by
      rw [at32_getD (by omega), List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
    rw [e2, this]; omega
  refine ⟨by omega, by omega, hs.piece_len, fun k hk => by rw [hs.piece_row k hk]; exact hi,
    fun k hk => (hpos _ (hstream k hk).1).1, fun k hk => (hpos _ (hstream k hk).2).1,
    fun k hk => (hpos _ (hstream k hk).1).2, fun k hk => (hpos _ (hstream k hk).2).2, ?_⟩
  have := tinv_small hT
  have := dedup_length_le (stream s.plus.val s.minus.val s.facets.val.length)
  rw [stream_length] at this; omega

/-! ## The flood's pairs are the Lean's crossings -/

theorem idxOf_map_inj {α β : Type} [BEq α] [LawfulBEq α] [BEq β] [LawfulBEq β] (f : α → β) :
    ∀ (xs : List α) (a : α), (∀ x ∈ xs, f x = f a → x = a) → (xs.map f).idxOf (f a) = xs.idxOf a
  | [], _, _ => rfl
  | x :: xs, a, h => by
    simp only [List.map_cons, List.idxOf_cons]
    by_cases e : x = a
    · subst e; simp
    · have : ¬f x = f a := fun h' => e (h x (List.mem_cons_self ..) h')
      have h1 : (f x == f a) = false := by simpa using this
      have h2 : (x == a) = false := by simpa using e
      simp only [h1, h2, cond_false, Bool.false_eq_true, if_false]
      rw [idxOf_map_inj f xs a (fun y hy => h y (List.mem_cons_of_mem _ hy))]

theorem floodEdges_eq (l : Ledger) (e0 : Ext) (s : Split) (e : Ext) (hok0 : ExtOK l e0) (hs : SplitOK l e0 s e)
    (hpc : ∀ q ∈ pcs l, q.1 < l.cells.val.length) {i : Nat} (hi : i < l.nb.val) :
    floodEdges s i s.facets.val.length =
      ForgeBoolCore.UF.posPairs (toL l).topes ((toL l).crossings i) := by
  have hFL := facets_toL l e0 hok0 hpc s.facets.val hs.facets
  have hTL := topes_toL l e0 s e hok0 hs hpc
  have hF : ∀ x ∈ s.facets.val, x.val < l.cells.val.length := by
    intro x hx
    have : x.val ∈ s.facets.val.map (·.val) := List.mem_map.mpr ⟨x, hx, rfl⟩
    rw [hs.facets] at this
    obtain ⟨q, hq, e⟩ := List.mem_map.mp ((mem_dedup _ _).mp this)
    rw [← e]; exact hpc q hq
  obtain ⟨_, _, _, _, hPl, hMl, hPM⟩ := hs.sides
  have hT := hs.topes
  -- a side's position, in the Lean's topes
  have hpos : ∀ c ∈ stream s.plus.val s.minus.val s.facets.val.length,
      ((toL l).topes).idxOf (covE l e c) = at32 s.tope_pos c := by
    intro c hc
    have hcn := hT.2.2.1 c hc
    rw [← hTL]
    have : s.topes.val.map (fun x => covE l e x.val) = (s.topes.val.map (·.val)).map (covE l e) := by
      simp [List.map_map]
    rw [this, idxOf_map_inj (covE l e) _ c (fun x hx h => covE_inj l e hs.ext
      (by rw [hT.2.1] at hx; exact hT.2.2.1 x ((mem_dedup _ _).mp hx)) hcn h), hT.2.1]
    have := hT.2.2.2 c hcn
    rw [if_pos hc] at this
    rw [at32_getD (by rw [hT.1]; exact hcn), ← this, List.getD_eq_getElem?_getD,
      List.getElem?_eq_getElem (by rw [hT.1]; exact hcn)]; rfl
  have hsm := stream_mem s.plus s.minus s.facets.val.length (by omega) (by omega)
  have hlenL : (toL l).facets.length = s.facets.val.length := by rw [← hFL]; simp
  -- by prefix
  have key : ∀ k, k ≤ s.facets.val.length → floodEdges s i k =
      ForgeBoolCore.UF.posPairs (toL l).topes
        ((((toL l).facets.take k).filter (fun φ => !(toL l).isPieceOf i φ)).map (fun φ => (φ.plus, φ.minus))) := by
    intro k hk
    induction k with
    | zero => simp [floodEdges, ForgeBoolCore.UF.posPairs]
    | succ k ih =>
      rw [floodEdges, ih (by omega), List.take_succ, List.getElem?_eq_getElem (by omega)]
      have hfk : (toL l).facets[k]'(by omega) = cellL l (s.facets.val[k]'(by omega)).val := by
        simp only [← hFL, List.getElem_map]
      have hpk : pieceAt s k i = (toL l).isPieceOf i (cellL l (s.facets.val[k]'(by omega)).val) :=
        piece_toL l e0 s e hok0 hs hpc (by omega) hi
      simp only [Option.toList_some, List.filter_append, List.map_append, ForgeBoolCore.UF.posPairs] at *
      rw [hfk]
      congr 1
      simp only [List.filter_cons, List.filter_nil, ← hpk]
      cases hpc' : pieceAt s k i
      · simp only [Bool.not_false, List.map_cons, List.map_nil, ite_true, ite_false, Bool.false_eq_true]
        obtain ⟨_, h2, _, h4⟩ := hPM k (by omega)
        have hc : (s.facets.val[k]'(by omega)).val < l.cells.val.length := hF _ (List.getElem_mem _)
        rw [getElem!_u32 (show k < s.facets.val.length by omega)] at h2 h4
        have ep : Cov.plus (cellL l (s.facets.val[k]'(by omega)).val) = covE l e (at32 s.plus k) := by
          rw [at32_getD (by omega), covE, ← getElem!_u32 (show k < s.plus.val.length by omega), h2, side_plus,
            cellL_covAt l e0 hc]
        have em : Cov.minus (cellL l (s.facets.val[k]'(by omega)).val) = covE l e (at32 s.minus k) := by
          rw [at32_getD (by omega), covE, ← getElem!_u32 (show k < s.minus.val.length by omega), h4, side_minus,
            cellL_covAt l e0 hc]
        rw [ep, em, hpos _ (hsm k (by omega)).1, hpos _ (hsm k (by omega)).2]
      · simp
  rw [key _ (Nat.le_refl _), ← hlenL, List.take_length]
  rfl

end forge_bool_core
