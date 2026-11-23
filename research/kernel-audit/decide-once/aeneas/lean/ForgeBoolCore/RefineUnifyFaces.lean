import ForgeBoolCore.RefineUnifyJoin

/-!
# Group U, part 4: the faces loop

With the join's roots `Rt` (least positions), `unify`'s second loop numbers the heads (`Rt p = p`)
in result order, writes per position its face number (`face_of`), and fills each face with its
positions in order (`u_faces_spec`).
-/

open Aeneas Aeneas.Std Result
open ForgeExact.DecideOnce ForgeBoolCore.UF

namespace forge_bool_core

/-- The values of a list of cell ids. -/
def nvC (xs : List CellId) : List Nat := xs.map fun c => (c : U32).val

/-- The heads below `p`: the positions that are their own root. -/
def hdL (Rt : Nat → Nat) (p : Nat) : List Nat := (List.range p).filter fun i => Rt i = i

/-- A face's name: the first input face its head came from, `0` if none. -/
def origOf (s : Split) (a : Assembled) (h : Nat) : Nat :=
  if (s.first_face.val.getD (resAt a h) NONE).val = U32.max then 0 else (s.first_face.val.getD (resAt a h) NONE).val

/-- A face as the loop writes it at step `p`, headed by `h`. -/
structure FaceOK (l : Ledger) (ext : Ext) (a : Assembled) (s : Split) (Rt : Nat → Nat) (p h : Nat)
    (F : OutFace) : Prop where
  facets : nvC F.facets.val = ((List.range p).filter fun i => Rt i = h).map (resAt a)
  plane : F.plane.val = zR l ext a h
  orient : F.orient = oR a h
  origin : (F.origin : U32).val = origOf s a h

/-- What the faces loop keeps: facets of positions `0 .. pf-1` placed, heads and face numbers of
positions `0 .. pq-1` written (`pq = pf` between steps, `pf + 1` mid-step). -/
structure FInv (l : Ledger) (ext : Ext) (a : Assembled) (s : Split) (Rt : Nat → Nat) (n pf pq : Nat)
    (face_of : List U32) (faces : List OutFace) (heads : List U32) : Prop where
  fol : face_of.length = n
  fo : ∀ i, i < pq → (face_of.getD i NONE).val = (hdL Rt pq).idxOf (Rt i)
  hds : nv heads = (hdL Rt pq).map (resAt a)
  fl : faces.length = (hdL Rt pq).length
  fc : ∀ f (hf : f < faces.length), FaceOK l ext a s Rt pf ((hdL Rt pq).getD f 0) faces[f]

/-- The union–find through the faces loop: its roots unchanged. -/
def UfR (Rt : Nat → Nat) (n : Nat) (uf : union_find.Uf) : Prop :=
  Good (vals uf.parent) ∧ uf.parent.val.length = n ∧ ∀ y, ufRoot (vals uf.parent) y = Rt y

theorem u_hdL_succ (Rt : Nat → Nat) (p : Nat) :
    hdL Rt (p + 1) = hdL Rt p ++ (if Rt p = p then [p] else []) := by
  unfold hdL; rw [List.range_succ, List.filter_append]; by_cases h : Rt p = p <;> simp [h]

theorem u_mem_hdL {Rt : Nat → Nat} {p i : Nat} : i ∈ hdL Rt p ↔ i < p ∧ Rt i = i := by
  simp [hdL]

theorem u_hdL_nodup (Rt : Nat → Nat) (p : Nat) : (hdL Rt p).Nodup := List.nodup_range.filter _

theorem u_getD_append_left {L M : List Nat} {f : Nat} (hf : f < L.length) : (L ++ M).getD f 0 = L.getD f 0 := by
  simp [List.getD_eq_getElem?_getD, List.getElem?_append_left hf]

theorem u_filter_range_succ (P : Nat → Bool) (p : Nat) :
    (List.range (p + 1)).filter P = (List.range p).filter P ++ (if P p then [p] else []) := by
  rw [List.range_succ, List.filter_append]; by_cases h : P p <;> simp [h]

theorem u_idxOf_lt {L : List Nat} {x : Nat} (h : x ∈ L) : L.idxOf x < L.length := List.idxOf_lt_length_iff.mpr h

theorem u_getD_idxOf {L : List Nat} {x : Nat} (h : x ∈ L) : L.getD (L.idxOf x) 0 = x := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (u_idxOf_lt h)]; simp

theorem u_idxOf_getD {L : List Nat} (hnd : L.Nodup) {f : Nat} (hf : f < L.length) : L.idxOf (L.getD f 0) = f := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hf]; simp [List.Nodup.idxOf_getElem hnd]

theorem u_filter_range_len (P : Nat → Bool) (p : Nat) : ((List.range p).filter P).length ≤ p :=
  (List.length_filter_le _ _).trans (by simp)

theorem u_cast_u32 {x : Usize} (h : x.val ≤ U32.max) : (UScalar.cast .U32 x).val = x.val := by
  rw [UScalar.cast_val_eq]; apply Nat.mod_eq_of_lt; have := U32.max_eq; simp at h ⊢; omega

theorem u_root_mem_hdL {Rt : Nat → Nat} {n : Nat}
    (hRt : ∀ i, i < n → Rt i ≤ i ∧ Rt (Rt i) = Rt i) {i q : Nat} (hi : i < n) (hiq : i < q) :
    Rt i ∈ hdL Rt q := u_mem_hdL.mpr ⟨by have := (hRt i hi).1; omega, (hRt i hi).2⟩

theorem u_getD_set_ne {L : List U32} {p i : Nat} {x : U32} (h : i ≠ p) : (L.set p x).getD i NONE = L.getD i NONE := by
  simp [List.getD_eq_getElem?_getD, List.getElem?_set, Ne.symm h]

theorem u_getD_set_eq {L : List U32} {p : Nat} {x : U32} (h : p < L.length) : (L.set p x).getD p NONE = x := by
  simp [List.getD_eq_getElem?_getD, List.getElem?_set, h]

/-- A new head at `p`: a new face, numbered after the others. -/
theorem u_finv_head {l : Ledger} {ext : Ext} {a : Assembled} {s : Split} {Rt : Nat → Nat} {n p : Nat}
    (hRt : ∀ i, i < n → Rt i ≤ i ∧ Rt (Rt i) = Rt i) (hpn : p < n)
    {fo : List U32} {fs : List OutFace} {hd : List U32} (hI : FInv l ext a s Rt n p p fo fs hd) (hRp : Rt p = p)
    (F0 : OutFace) (hF0f : nvC F0.facets.val = []) (hF0p : F0.plane.val = zR l ext a p)
    (hF0o : F0.orient = oR a p) (hF0g : (F0.origin : U32).val = origOf s a p)
    (i1 : U32) (hi1 : i1.val = fs.length) (x : U32) (hx : x.val = resAt a p) :
    FInv l ext a s Rt n p (p + 1) (fo.set p i1) (fs ++ [F0]) (hd ++ [x]) := by
  have hsucc : hdL Rt (p + 1) = hdL Rt p ++ [p] := by rw [u_hdL_succ, if_pos hRp]
  have hpn' : p ∉ hdL Rt p := fun h => by have := (u_mem_hdL.mp h).1; omega
  have hfo := hI.fol
  refine ⟨by simp [hI.fol], ?_, ?_, ?_, ?_⟩
  · intro i hi
    rw [hsucc]
    by_cases hip : i = p
    · subst hip
      rw [u_getD_set_eq (by omega), hi1, hI.fl, hRp, List.idxOf_append_of_notMem hpn']; simp
    · have hil : i < p := by omega
      rw [u_getD_set_ne hip, hI.fo i hil, List.idxOf_append_of_mem (u_root_mem_hdL hRt (by omega) hil)]
  · rw [hsucc, List.map_append, ← hI.hds]; simp [nv, hx]
  · rw [hsucc]; simp [hI.fl]
  · intro f hf
    rw [hsucc]
    by_cases hfl : f < fs.length
    · rw [List.getElem_append_left hfl, u_getD_append_left (by rw [← hI.fl]; exact hfl)]
      exact hI.fc f hfl
    · have : f = fs.length := by simp at hf; omega
      subst this
      rw [List.getElem_append_right (le_refl _)]
      simp only [Nat.sub_self, List.getElem_cons_zero]
      have hg : (hdL Rt p ++ [p]).getD fs.length 0 = p := by
        rw [hI.fl]; simp [List.getD_eq_getElem?_getD]
      rw [hg]
      refine ⟨?_, hF0p, hF0o, hF0g⟩
      rw [hF0f]
      symm; rw [List.map_eq_nil_iff, List.filter_eq_nil_iff]
      intro i hi; simp at hi ⊢
      have := (hRt i (by omega)).1; omega

/-- A position that is not a head: the face number of its root. -/
theorem u_finv_non {l : Ledger} {ext : Ext} {a : Assembled} {s : Split} {Rt : Nat → Nat} {n p : Nat}
    (hRt : ∀ i, i < n → Rt i ≤ i ∧ Rt (Rt i) = Rt i) (hpn : p < n)
    {fo : List U32} {fs : List OutFace} {hd : List U32} (hI : FInv l ext a s Rt n p p fo fs hd) (hRp : Rt p ≠ p)
    (i1 : U32) (hi1 : i1.val = (fo.getD (Rt p) NONE).val) :
    FInv l ext a s Rt n p (p + 1) (fo.set p i1) fs hd := by
  have hsucc : hdL Rt (p + 1) = hdL Rt p := by rw [u_hdL_succ, if_neg hRp]; simp
  have hlt : Rt p < p := by have := (hRt p hpn).1; omega
  refine ⟨by simp [hI.fol], ?_, by rw [hsucc]; exact hI.hds, by rw [hsucc]; exact hI.fl, by rw [hsucc]; exact hI.fc⟩
  intro i hi
  rw [hsucc]
  by_cases hip : i = p
  · subst hip
    rw [u_getD_set_eq (by rw [hI.fol]; exact hpn), hi1, hI.fo _ hlt, (hRt i hpn).2]
  · rw [u_getD_set_ne hip, hI.fo i (by omega)]

/-- The step's tail: the position's cell joins its face. -/
theorem u_finv_tail {l : Ledger} {ext : Ext} {a : Assembled} {s : Split} {Rt : Nat → Nat} {n p : Nat}
    (hRt : ∀ i, i < n → Rt i ≤ i ∧ Rt (Rt i) = Rt i) (hpn : p < n)
    {fo : List U32} {fs : List OutFace} {hd : List U32} (hI : FInv l ext a s Rt n p (p + 1) fo fs hd) :
    (fo.getD p NONE).val < fs.length ∧
    ∀ (F' : OutFace) (hf : (fo.getD p NONE).val < fs.length),
      nvC F'.facets.val = nvC (fs[(fo.getD p NONE).val]'hf).facets.val ++ [resAt a p] →
      F'.plane = (fs[(fo.getD p NONE).val]'hf).plane → F'.orient = (fs[(fo.getD p NONE).val]'hf).orient →
      F'.origin = (fs[(fo.getD p NONE).val]'hf).origin →
      FInv l ext a s Rt n (p + 1) (p + 1) fo (fs.set (fo.getD p NONE).val F') hd := by
  have hmem := u_root_mem_hdL hRt hpn (Nat.lt_succ_self p)
  have hf0 : (fo.getD p NONE).val = (hdL Rt (p + 1)).idxOf (Rt p) := hI.fo p (by omega)
  have hflt : (fo.getD p NONE).val < fs.length := by rw [hf0, hI.fl]; exact u_idxOf_lt hmem
  refine ⟨hflt, fun F' hf hF1 hF2 hF3 hF4 => ?_⟩
  refine ⟨hI.fol, hI.fo, hI.hds, by simp [hI.fl], ?_⟩
  intro f hfl
  have hfl' : f < fs.length := by simpa using hfl
  have hfac := hI.fc f hfl'
  rw [List.getElem_set]
  have hupd : ∀ hh, ((List.range (p + 1)).filter fun i => Rt i = hh) =
      ((List.range p).filter fun i => Rt i = hh) ++ (if Rt p = hh then [p] else []) := by
    intro hh; rw [u_filter_range_succ]; by_cases e : Rt p = hh <;> simp [e]
  split
  · rename_i e
    subst e
    have hg : (hdL Rt (p + 1)).getD (fo.getD p NONE).val 0 = Rt p := by rw [hf0]; exact u_getD_idxOf hmem
    have hfac' := hI.fc _ hf
    rw [hg] at hfac' ⊢
    refine ⟨?_, by rw [hF2]; exact hfac'.plane, by rw [hF3]; exact hfac'.orient, by rw [hF4]; exact hfac'.origin⟩
    rw [hF1, hfac'.facets, hupd, if_pos rfl]; simp
  · rename_i e
    have hne : Rt p ≠ (hdL Rt (p + 1)).getD f 0 := by
      intro h'; apply e
      rw [hf0, h', u_idxOf_getD (u_hdL_nodup _ _) (by rw [← hI.fl]; exact hfl')]
    refine ⟨?_, hfac.plane, hfac.orient, hfac.origin⟩
    rw [hfac.facets, hupd, if_neg hne]; simp

/-- **The faces loop.** -/
theorem u_faces_loop_spec (l : Ledger) (ext : Ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (horl : a.orient.val.length = a.result.val.length) (s : Split) (Rt : Nat → Nat)
    (hRt : ∀ i, i < a.result.val.length → Rt i < a.result.val.length ∧ Rt i ≤ i ∧ Rt (Rt i) = Rt i)
    (hz : ∀ i, i < a.result.val.length → zR l ext a i ≤ U32.max)
    (hff : ∀ i, i < a.result.val.length → resAt a i < s.first_face.val.length)
    (nr : Usize) (hnr : nr.val = a.result.val.length)
    (uf : union_find.Uf) (face_of : alloc.vec.Vec U32) (faces : alloc.vec.Vec OutFace)
    (heads : alloc.vec.Vec U32) (p : Usize) (hp : p.val ≤ a.result.val.length)
    (hu : UfR Rt a.result.val.length uf)
    (hI : FInv l ext a s Rt a.result.val.length p.val p.val face_of.val faces.val heads.val) :
    unify_loop1 l ext s a.result a.orient nr uf face_of faces heads p ⦃ r =>
      FInv l ext a s Rt a.result.val.length a.result.val.length a.result.val.length r.1.val r.2.1.val r.2.2.val ⦄ := by
  have hsmall := h.small
  have hRt' : ∀ i, i < a.result.val.length → Rt i ≤ i ∧ Rt (Rt i) = Rt i := fun i hi => (hRt i hi).2
  unfold unify_loop1
  apply loop.spec_decr_nat
    (fun (st : Split × union_find.Uf × alloc.vec.Vec U32 × alloc.vec.Vec OutFace × alloc.vec.Vec U32 × Usize) =>
      a.result.val.length - st.2.2.2.2.2.val)
    (fun st => st.1 = s ∧ st.2.2.2.2.2.val ≤ a.result.val.length ∧ UfR Rt a.result.val.length st.2.1 ∧
      FInv l ext a s Rt a.result.val.length st.2.2.2.2.2.val st.2.2.2.2.2.val st.2.2.1.val st.2.2.2.1.val
        st.2.2.2.2.1.val)
  · rintro ⟨s0, u0, fo0, fs0, hd0, j⟩ ⟨hs0, hj, ⟨hg0, hl0, hr0⟩, hI0⟩
    subst hs0
    unfold unify_loop1.body
    simp only at hj hI0 hg0 hl0 hr0 ⊢
    split
    · rename_i hlt
      have hjn : j.val < a.result.val.length := by scalar_tac
      obtain ⟨hRn, hRle, hRR⟩ := hRt j.val hjn
      step with find_spec u0 j (by omega) hg0 as ⟨r, u1, hr, hl1, hg1, hr1⟩
      have hrv : r.val = Rt j.val := by rw [hr, hr0]
      have hu1 : UfR Rt a.result.val.length u1 := ⟨hg1, by omega, fun y => by rw [hr1, hr0]⟩
      have hfol := hI0.fol
      have hfsl : fs0.val.length ≤ j.val := by
        rw [hI0.fl]; exact (List.length_filter_le _ _).trans (by simp)
      have hjfo : j.val < fo0.val.length := by omega
      have hhdl : hd0.val.length < Usize.max := by
        have := congrArg List.length hI0.hds; simp only [nv, List.length_map] at this
        have h2 : (hdL Rt j.val).length ≤ j.val := (List.length_filter_le _ _).trans (by simp)
        have := u32_le_usize; omega
      split
      · rename_i hrj
        have hRj : Rt j.val = j.val := by rw [← hrv, hrj]
        step as ⟨w0, hw0, hback0⟩
        obtain ⟨_, back0⟩ := w0
        simp only at hback0 ⊢
        subst hback0
        step as ⟨i1, hi1⟩
        have hfsl' : (alloc.vec.Vec.len fs0).val = fs0.val.length := by simp
        have hi1v : i1.val = fs0.val.length := by rw [hi1, u_cast_u32 (by omega), hfsl']
        step as ⟨hh, hhh⟩
        have hhv : hh.val = resAt a j.val := by rw [hhh, u_resAt_eq a hjn]
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = hh.val := by rw [hi2]; scalar_tac
        have hffl : i2.val < s0.first_face.val.length := by rw [hi2v, hhv]; exact hff _ hjn
        step as ⟨ff, hffv⟩
        have hffg : ff = s0.first_face.val.getD (resAt a j.val) NONE := by
          rw [hffv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by rw [← hhv, ← hi2v]; exact hffl)]
          simp [hi2v, hhv]
        have hhl : hh.val < (allCells l ext).length := by rw [hhv]; exact resAt_lt h hjn
        step with cov_spec l ext hh hhl as ⟨s2, hs2⟩
        step with zero_idx_spec s2 as ⟨i3, hi3⟩
        have hz3 : i3.val = zR l ext a j.val := by
          rw [hi3, zR, ← hhv, ← hs2, covOf, ForgeExact.DecideOnce.Cov.zeroIdx, idxOf_map_toS]
        step as ⟨i4, hi4⟩
        have hi4v : i4.val = zR l ext a j.val := by rw [hi4, u_cast_u32 (by rw [hz3]; exact hz _ hjn), hz3]
        step as ⟨s3, hs3⟩
        have hs3v : s3 = oR a j.val := by rw [u_oR_eq a (by omega), hs3]
        split
        · rename_i hffn
          step as ⟨FS, hFS⟩
          step as ⟨HD, hHD⟩
          have hM : FInv l ext a s0 Rt a.result.val.length j.val (j.val + 1) (fo0.val.set j.val i1) FS.val HD.val := by
            rw [hFS, hHD]
            refine u_finv_head hRt' hjn hI0 hRj _ (by simp [nvC]) hi4v hs3v ?_ i1 hi1v hh hhv
            show (0#u32 : U32).val = origOf s0 a j.val
            unfold origOf; rw [← hffg, hffn, none_val]; simp
          rw [← alloc.vec.Vec.set_val_eq] at hM
          obtain ⟨hflt, hupd⟩ := u_finv_tail hRt' hjn hM
          have hfo1 := hM.fol
          step as ⟨fi, hfi⟩
          have hfiv : fi.val = ((fo0.set j i1).val.getD j.val NONE).val := by
            rw [hfi, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
          step as ⟨fi1, hfi1⟩
          have hfi1v : fi1.val = fi.val := by rw [hfi1]; scalar_tac
          have hfl : fi1.val < FS.val.length := by rw [hfi1v, hfiv]; exact hflt
          step as ⟨of, back, hof, hback⟩
          step as ⟨x, hx⟩
          have hxv : x.val = resAt a j.val := by rw [hx, u_resAt_eq a hjn]
          have hofe : of = FS.val[((fo0.set j i1).val.getD j.val NONE).val]'hflt := by
            rw [hof]; simp [hfi1v, hfiv]
          have hofl : of.facets.val.length < Usize.max := by
            have hfc := congrArg List.length (hM.fc _ hflt).facets
            simp only [nvC, List.length_map] at hfc
            rw [hofe, hfc]
            exact lt_of_le_of_lt (u_filter_range_len _ _) (by have := u32_le_usize; omega)
          step as ⟨v2, hv2⟩
          step as ⟨j1, hj1⟩
          refine ⟨by rw [hj1]; omega, hu1, ?_, by rw [hj1]; omega⟩
          rw [hj1, hback, alloc.vec.Vec.set_val_eq _ fi1, hfi1v, hfiv]
          apply hupd _ hflt
          · rw [← hofe]; simp [nvC, hv2, hxv]
          · rw [← hofe]
          · rw [← hofe]
          · rw [← hofe]
        · rename_i hffn
          step as ⟨FS, hFS⟩
          step as ⟨HD, hHD⟩
          have hM : FInv l ext a s0 Rt a.result.val.length j.val (j.val + 1) (fo0.val.set j.val i1) FS.val HD.val := by
            rw [hFS, hHD]
            refine u_finv_head hRt' hjn hI0 hRj _ (by simp [nvC]) hi4v hs3v ?_ i1 hi1v hh hhv
            show (ff : U32).val = origOf s0 a j.val
            unfold origOf; rw [← hffg]
            have : ff.val ≠ U32.max := fun e => hffn (UScalar.eq_of_val_eq (by rw [e, none_val]))
            simp [this]
          rw [← alloc.vec.Vec.set_val_eq] at hM
          obtain ⟨hflt, hupd⟩ := u_finv_tail hRt' hjn hM
          have hfo1 := hM.fol
          step as ⟨fi, hfi⟩
          have hfiv : fi.val = ((fo0.set j i1).val.getD j.val NONE).val := by
            rw [hfi, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
          step as ⟨fi1, hfi1⟩
          have hfi1v : fi1.val = fi.val := by rw [hfi1]; scalar_tac
          have hfl : fi1.val < FS.val.length := by rw [hfi1v, hfiv]; exact hflt
          step as ⟨of, back, hof, hback⟩
          step as ⟨x, hx⟩
          have hxv : x.val = resAt a j.val := by rw [hx, u_resAt_eq a hjn]
          have hofe : of = FS.val[((fo0.set j i1).val.getD j.val NONE).val]'hflt := by
            rw [hof]; simp [hfi1v, hfiv]
          have hofl : of.facets.val.length < Usize.max := by
            have hfc := congrArg List.length (hM.fc _ hflt).facets
            simp only [nvC, List.length_map] at hfc
            rw [hofe, hfc]
            exact lt_of_le_of_lt (u_filter_range_len _ _) (by have := u32_le_usize; omega)
          step as ⟨v2, hv2⟩
          step as ⟨j1, hj1⟩
          refine ⟨by rw [hj1]; omega, hu1, ?_, by rw [hj1]; omega⟩
          rw [hj1, hback, alloc.vec.Vec.set_val_eq _ fi1, hfi1v, hfiv]
          apply hupd _ hflt
          · rw [← hofe]; simp [nvC, hv2, hxv]
          · rw [← hofe]
          · rw [← hofe]
          · rw [← hofe]
      · rename_i hrj
        have hRj : Rt j.val ≠ j.val := by intro e; apply hrj; apply UScalar.eq_of_val_eq; rw [hrv, e]
        have hrl : r.val < fo0.val.length := by omega
        step as ⟨i0, hi0⟩
        step as ⟨w0, hw0, hback0⟩
        obtain ⟨_, back0⟩ := w0
        simp only at hback0 ⊢
        subst hback0
        have hM : FInv l ext a s0 Rt a.result.val.length j.val (j.val + 1) (fo0.val.set j.val i0) fs0.val hd0.val := by
          refine u_finv_non hRt' hjn hI0 hRj i0 ?_
          rw [hi0, ← hrv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hrl]; rfl
        rw [← alloc.vec.Vec.set_val_eq] at hM
        obtain ⟨hflt, hupd⟩ := u_finv_tail hRt' hjn hM
        have hfo1 := hM.fol
        step as ⟨fi, hfi⟩
        have hfiv : fi.val = ((fo0.set j i0).val.getD j.val NONE).val := by
          rw [hfi, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
        step as ⟨fi1, hfi1⟩
        have hfi1v : fi1.val = fi.val := by rw [hfi1]; scalar_tac
        have hfl : fi1.val < fs0.val.length := by rw [hfi1v, hfiv]; exact hflt
        step as ⟨of, back, hof, hback⟩
        step as ⟨x, hx⟩
        have hxv : x.val = resAt a j.val := by rw [hx, u_resAt_eq a hjn]
        have hofe : of = fs0.val[((fo0.set j i0).val.getD j.val NONE).val]'hflt := by
          rw [hof]; simp [hfi1v, hfiv]
        have hofl : of.facets.val.length < Usize.max := by
          have hfc := congrArg List.length (hM.fc _ hflt).facets
          simp only [nvC, List.length_map] at hfc
          rw [hofe, hfc]
          exact lt_of_le_of_lt (u_filter_range_len _ _) (by have := u32_le_usize; omega)
        step as ⟨v2, hv2⟩
        step as ⟨j1, hj1⟩
        refine ⟨by rw [hj1]; omega, hu1, ?_, by rw [hj1]; omega⟩
        rw [hj1, hback, alloc.vec.Vec.set_val_eq _ fi1, hfi1v, hfiv]
        apply hupd _ hflt
        · rw [← hofe]; simp [nvC, hv2, hxv]
        · rw [← hofe]
        · rw [← hofe]
        · rw [← hofe]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = a.result.val.length := by scalar_tac
      rw [this] at hI0; exact hI0
  · exact ⟨rfl, hp, hu, hI⟩

end forge_bool_core
