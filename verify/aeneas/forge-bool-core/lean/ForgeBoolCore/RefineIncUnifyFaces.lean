import ForgeBoolCore.RefineIncUnifyJoin

/-!
# Group U (inc), part 2: the faces loops

With the join's roots `Rt` (least positions), `inc::unify`'s second loop numbers the heads
(`Rt p = p`) in result order, writes per position its face number (`face_of`) and its head's facet
(`head_of`), and opens one face per head (`faces1_spec`); the third fills each face with its
positions' facets, in order (`faces2_spec`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-- The facet at result position `i`. -/
def rAt (res : List U32) (i : Nat) : Nat := (res.getD i 0#u32).val

theorem rAt_eq {res : List U32} {i : Nat} (h : i < res.length) : rAt res i = (res[i]).val := by
  unfold rAt; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]; rfl

/-- The roots of a least-root naming. -/
def RtOK (Rt : Nat → Nat) (n : Nat) : Prop := ∀ i, i < n → Rt i < n ∧ Rt i ≤ i ∧ Rt (Rt i) = Rt i

theorem hdL_idxOf {Rt : Nat → Nat} {h : Nat} (hh : Rt h = h) :
    ∀ m, (hdL Rt (h + 1 + m)).idxOf h = (hdL Rt h).length ∧ (hdL Rt h).length < (hdL Rt (h + 1 + m)).length := by
  intro m
  induction m with
  | zero =>
    rw [Nat.add_zero, u_hdL_succ, if_pos hh,
      List.idxOf_append_of_notMem (fun hm => by have := (u_mem_hdL.mp hm).1; omega)]
    simp
  | succ m ih =>
    rw [show h + 1 + (m + 1) = (h + 1 + m) + 1 by omega, u_hdL_succ]
    have hm : h ∈ hdL Rt (h + 1 + m) := u_mem_hdL.mpr ⟨by omega, hh⟩
    rw [List.idxOf_append_of_mem hm, ih.1]
    refine ⟨rfl, ?_⟩
    simp only [List.length_append]; have := ih.2; omega

/-- A head's face number: the heads before it. -/
theorem hdL_get {Rt : Nat → Nat} {n h : Nat} (hh : Rt h = h) (hn : h < n) :
    ∃ hl : (hdL Rt h).length < (hdL Rt n).length, (hdL Rt n)[(hdL Rt h).length] = h := by
  obtain ⟨e1, e2⟩ := hdL_idxOf hh (n - h - 1)
  rw [show h + 1 + (n - h - 1) = n by omega] at e1 e2
  refine ⟨e2, ?_⟩
  have hm : h ∈ hdL Rt n := u_mem_hdL.mpr ⟨hn, hh⟩
  have := List.getElem_idxOf (List.idxOf_lt_length_iff.mpr hm)
  simp only [e1] at this; exact this

/-- A face as the second loop opens it: no facets yet, its head's carrier and sense. -/
structure Face0 (fc res : List U32) (ori : List Sgn) (h : Nat) (F : OutFace) : Prop where
  facets : F.facets.val = []
  plane : F.plane.val = cAt fc res h
  orient : F.orient = oAt ori h

/-- What the second loop keeps, after positions `0 .. p-1`. -/
structure F1Inv (fc res : List U32) (ori : List Sgn) (Rt : Nat → Nat) (n p : Nat)
    (face_of : List U32) (faces : List OutFace) (head_of : List U32) : Prop where
  fol : face_of.length = n
  fo : ∀ i, i < p → (face_of.getD i NONE).val = (hdL Rt (Rt i)).length
  hol : head_of.length = n
  ho : ∀ i, i < p → (head_of.getD i NONE).val = rAt res (Rt i)
  fl : faces.length = (hdL Rt p).length
  fc : ∀ k (hk : k < faces.length), Face0 fc res ori ((hdL Rt p).getD k 0) faces[k]

theorem getD_set_ne' {L : List U32} {p i : Nat} {x : U32} (h : i ≠ p) :
    (L.set p x).getD i NONE = L.getD i NONE := by
  simp [List.getD_eq_getElem?_getD, List.getElem?_set, Ne.symm h]

theorem getD_set_eq' {L : List U32} {p : Nat} {x : U32} (h : p < L.length) : (L.set p x).getD p NONE = x := by
  simp [List.getD_eq_getElem?_getD, List.getElem?_set, h]

theorem faces1_spec (fc res : alloc.vec.Vec U32) (ori : alloc.vec.Vec Sgn)
    (horl : ori.val.length = res.val.length) (hres : ∀ x ∈ res.val, x.val < fc.val.length)
    (hn : res.val.length < U32.max) (nr : Usize) (hnr : nr.val = res.val.length)
    (Rt : Nat → Nat) (hRt : RtOK Rt res.val.length)
    (join : union_find.Uf) (face_of : alloc.vec.Vec U32) (faces : alloc.vec.Vec OutFace)
    (head_of : alloc.vec.Vec U32) (p : Usize) (hp : p.val ≤ res.val.length)
    (hu : UfR Rt res.val.length join)
    (hI : F1Inv fc.val res.val ori.val Rt res.val.length p.val face_of.val faces.val head_of.val) :
    inc.unify_loop1 fc res ori nr join face_of faces head_of p ⦃ r =>
      F1Inv fc.val res.val ori.val Rt res.val.length res.val.length r.1.val r.2.1.val r.2.2.val ⦄ := by
  unfold inc.unify_loop1
  apply loop.spec_decr_nat
    (fun (st : union_find.Uf × alloc.vec.Vec U32 × alloc.vec.Vec OutFace × alloc.vec.Vec U32 × Usize) =>
      res.val.length - st.2.2.2.2.val)
    (fun st => st.2.2.2.2.val ≤ res.val.length ∧ UfR Rt res.val.length st.1 ∧
      F1Inv fc.val res.val ori.val Rt res.val.length st.2.2.2.2.val st.2.1.val st.2.2.1.val st.2.2.2.1.val)
  · rintro ⟨u0, fo, fs, ho, j⟩ ⟨hj, hu0, hI0⟩
    unfold inc.unify_loop1.body
    simp only at hj hu0 hI0 ⊢
    split
    · rename_i hlt
      have hjn : j.val < res.val.length := by scalar_tac
      obtain ⟨hg0, hl0, hr0⟩ := hu0
      obtain ⟨hR, hRle, hRR⟩ := hRt j.val hjn
      step with find_spec u0 j (by omega) hg0 as ⟨r, u1, hr, hl1, hg1, hr1⟩
      have hrv : r.val = Rt j.val := by rw [hr, hr0]
      have hu1 : UfR Rt res.val.length u1 := ⟨hg1, by omega, fun y => by rw [hr1, hr0]⟩
      step as ⟨i, hi⟩
      have hiv : i.val = rAt res.val (Rt j.val) := by rw [rAt_eq (by omega), hi]; simp only [hrv]
      have hfol := hI0.fol
      have hhol := hI0.hol
      step as ⟨x0, back, hx0, hback⟩
      split
      · rename_i hrj
        have hRj : Rt j.val = j.val := by rw [← hrv, hrj]
        step as ⟨y0, _, hback1⟩
        obtain ⟨_, back1⟩ := y0
        try simp only at hback1 ⊢
        have hfsl : fs.val.length ≤ j.val := by
          rw [hI0.fl]; exact u_filter_range_len _ _
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = fs.val.length := by
          have := u_cast_u32 (x := alloc.vec.Vec.len fs) (by simp; omega)
          rw [hi2]; simpa using this
        have hfcj := hres _ (List.getElem_mem hjn)
        step as ⟨i3, hi3⟩
        step as ⟨i4, hi4⟩
        have hi4v : i4.val = (res.val[j.val]).val := by rw [hi4, hi3]; scalar_tac
        step as ⟨i5, hi5⟩
        step as ⟨s, hs⟩
        have hfsl2 : fs.val.length < Usize.max := by have := u32_le_usize; omega
        step as ⟨fs2, hfs2⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by omega
        refine ⟨by omega, hu1, ?_, by omega⟩
        have hsucc : hdL Rt (j.val + 1) = hdL Rt j.val ++ [j.val] := by rw [u_hdL_succ, if_pos hRj]
        rw [hj1v]
        refine ⟨by rw [hback1]; simp [hfol], ?_, by rw [hback]; simp [hhol], ?_, ?_, ?_⟩
        · intro k hk
          rw [hback1, alloc.vec.Vec.set_val_eq]
          by_cases hkj : k = j.val
          · subst hkj; rw [getD_set_eq' (by omega), hi2v, hI0.fl, hRj]
          · rw [getD_set_ne' hkj, hI0.fo k (by omega)]
        · intro k hk
          rw [hback, alloc.vec.Vec.set_val_eq]
          by_cases hkj : k = j.val
          · subst hkj; rw [getD_set_eq' (by omega), hiv]
          · rw [getD_set_ne' hkj, hI0.ho k (by omega)]
        · rw [hfs2, hsucc]; simp [hI0.fl]
        · intro k hk
          have hk' : k < fs.val.length + 1 := by rw [hfs2] at hk; simpa using hk
          rw [hsucc]
          simp only [hfs2]
          by_cases hkl : k < fs.val.length
          · rw [List.getElem_append_left hkl, u_getD_append_left (by rw [← hI0.fl]; exact hkl)]
            exact hI0.fc k hkl
          · have : k = fs.val.length := by omega
            subst this
            rw [List.getElem_append_right (le_refl _)]
            simp only [Nat.sub_self, List.getElem_cons_zero]
            have hg : (hdL Rt j.val ++ [j.val]).getD fs.val.length 0 = j.val := by
              rw [hI0.fl]; simp [List.getD_eq_getElem?_getD]
            rw [hg]
            refine ⟨rfl, ?_, ?_⟩
            · simp only; rw [hi5, cAt_eq hjn hfcj]; simp only [← hi4v]
            · simp only; rw [hs, oAt_eq (by omega)]
      · rename_i hrj
        have hRj : Rt j.val ≠ j.val := by rw [← hrv]; intro h; exact hrj (UScalar.eq_of_val_eq h)
        have hrlt : r.val < j.val := by rw [hrv]; omega
        step as ⟨i1, hi1⟩
        step as ⟨y0, _, hback1⟩
        obtain ⟨_, back1⟩ := y0
        try simp only at hback1 ⊢
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        refine ⟨by omega, hu1, ?_, by omega⟩
        have hsucc : hdL Rt (j.val + 1) = hdL Rt j.val := by rw [u_hdL_succ, if_neg hRj, List.append_nil]
        rw [hj1v]
        refine ⟨by rw [hback1]; simp [hfol], ?_, by rw [hback]; simp [hhol], ?_, by rw [hsucc]; exact hI0.fl,
          by rw [hsucc]; exact hI0.fc⟩
        · intro k hk
          rw [hback1, alloc.vec.Vec.set_val_eq]
          by_cases hkj : k = j.val
          · subst hkj
            rw [getD_set_eq' (by omega)]
            have := hI0.fo r.val hrlt
            rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some] at this
            rw [hi1, this, hrv, hRR]
          · rw [getD_set_ne' hkj, hI0.fo k (by omega)]
        · intro k hk
          rw [hback, alloc.vec.Vec.set_val_eq]
          by_cases hkj : k = j.val
          · subst hkj; rw [getD_set_eq' (by omega), hiv]
          · rw [getD_set_ne' hkj, hI0.ho k (by omega)]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = res.val.length := by scalar_tac
      rw [this] at hI0; exact hI0
  · exact ⟨hp, hu, hI⟩

/-- A face as the third loop fills it, after positions `0 .. p-1`. -/
structure Face1 (fc res : List U32) (ori : List Sgn) (Rt : Nat → Nat) (p h : Nat) (F : OutFace) : Prop where
  facets : F.facets.val.map (fun (c : FacetId) => (c : U32).val) =
    ((List.range p).filter fun d => Rt d = h).map (rAt res)
  plane : F.plane.val = cAt fc res h
  orient : F.orient = oAt ori h

theorem faces2_spec (fc res : alloc.vec.Vec U32) (ori : alloc.vec.Vec Sgn)
    (nr : Usize) (hnr : nr.val = res.val.length)
    (Rt : Nat → Nat) (hRt : RtOK Rt res.val.length)
    (face_of : alloc.vec.Vec U32) (hfol : face_of.val.length = res.val.length)
    (hfo : ∀ i, i < res.val.length → (face_of.val.getD i NONE).val = (hdL Rt (Rt i)).length)
    (faces : alloc.vec.Vec OutFace) (p : Usize) (hp : p.val ≤ res.val.length)
    (hfl : faces.val.length = (hdL Rt res.val.length).length)
    (hI : ∀ k (hk : k < faces.val.length), Face1 fc.val res.val ori.val Rt p.val ((hdL Rt res.val.length).getD k 0)
      faces.val[k]) :
    inc.unify_loop2 res nr face_of faces p ⦃ r => r.val.length = (hdL Rt res.val.length).length ∧
      ∀ k (hk : k < r.val.length), Face1 fc.val res.val ori.val Rt res.val.length
        ((hdL Rt res.val.length).getD k 0) r.val[k] ⦄ := by
  unfold inc.unify_loop2
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec OutFace × Usize) => res.val.length - st.2.val)
    (fun st => st.2.val ≤ res.val.length ∧ st.1.val.length = (hdL Rt res.val.length).length ∧
      ∀ k (hk : k < st.1.val.length), Face1 fc.val res.val ori.val Rt st.2.val
        ((hdL Rt res.val.length).getD k 0) st.1.val[k])
  · rintro ⟨fs, j⟩ ⟨hj, hl0, hI0⟩
    unfold inc.unify_loop2.body
    simp only at hj hl0 hI0 ⊢
    split
    · rename_i hlt
      have hjn : j.val < res.val.length := by scalar_tac
      obtain ⟨hR, hRle, hRR⟩ := hRt j.val hjn
      obtain ⟨hkl, hkg⟩ := hdL_get (n := res.val.length) hRR hR
      step as ⟨i, hi⟩
      have hiv : i.val = (hdL Rt (Rt j.val)).length := by
        have := hfo j.val hjn
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some] at this
        rw [hi, this]
      step as ⟨i1, hi1⟩
      have hi1v : i1.val = (hdL Rt (Rt j.val)).length := by rw [hi1, ← hiv]; scalar_tac
      have hi1l : i1.val < fs.val.length := by rw [hi1v, hl0]; exact hkl
      step as ⟨F, back, hF, hback⟩
      step as ⟨i2, hi2⟩
      have hFl : F.facets.val.length ≤ j.val := by
        have := (hI0 i1.val hi1l).facets
        have h2 := congrArg List.length this
        simp only [List.length_map] at h2
        rw [hF, h2]; exact u_filter_range_len _ _
      have hpush : F.facets.val.length < Usize.max := by scalar_tac
      step as ⟨v1, hv1⟩
      step as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by scalar_tac
      refine ⟨by omega, ?_, ?_, by omega⟩
      · rw [hback]; simp [hl0]
      · intro k hk
        have hk' : k < fs.val.length := by rw [hback] at hk; simpa using hk
        have hkI := hI0 k hk'
        rw [hj1v]
        simp only [hback, alloc.vec.Vec.set_val_eq]
        by_cases hki : k = i1.val
        · subst hki
          simp only [List.getElem_set_self]
          have hgk : (hdL Rt res.val.length).getD i1.val 0 = Rt j.val := by
            rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some]
            simp only [hi1v]; exact hkg
          rw [hgk] at hkI ⊢
          rw [← hF] at hkI
          refine ⟨?_, hkI.plane, hkI.orient⟩
          rw [hv1, List.map_append, hkI.facets, u_filter_range_succ]
          simp [rAt_eq hjn, hi2]
        · rw [List.getElem_set_ne (Ne.symm hki)]
          refine ⟨?_, hkI.plane, hkI.orient⟩
          rw [hkI.facets, u_filter_range_succ]
          have : ¬ Rt j.val = (hdL Rt res.val.length).getD k 0 := by
            intro he; apply hki
            have hnd := u_hdL_nodup Rt res.val.length
            have h1 : (hdL Rt res.val.length).getD k 0 = (hdL Rt res.val.length)[k]'(by omega) := by
              rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
            rw [h1] at he
            rw [hi1v]
            have := (List.Nodup.getElem_inj_iff hnd (i := k) (j := (hdL Rt (Rt j.val)).length)
              (hi := by omega) (hj := hkl)).mp (by rw [← he, hkg])
            exact this
          have h2 : decide (Rt j.val = (hdL Rt res.val.length).getD k 0) = false := by simpa using this
          rw [h2]; simp only [Bool.false_eq_true, if_false, List.append_nil]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = res.val.length := by scalar_tac
      rw [this] at hI0; exact ⟨hl0, hI0⟩
  · exact ⟨hp, hfl, hI⟩

end inc

end forge_bool_core
