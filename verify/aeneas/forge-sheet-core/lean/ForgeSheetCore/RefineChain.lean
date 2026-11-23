import ForgeSheetCore.RefineUF
open Aeneas Aeneas.Std Result

/-!
# `chain`, first pass: the degrees, exactly

`chain`'s first loop reads each kept piece `e` (from vertex `F[e]` to `T[e]`) once and keeps, per
vertex `v`: how many pieces leave and arrive (`n_out`, `n_in`), the first leaving piece
(`first_out`) and the first two arriving ones (`first_in`, `second_in`), `u32::MAX` where there
is none. `loop0_chain_spec`: after it, each vector is that function of the pieces, as `occ`
(the pieces at a vertex, in order) says. No panic or overflow when every vertex is below `nv`,
`to` is at least as long as `from`, and there are at most `u32::MAX` pieces.
-/

namespace forge_sheet_core

/-- The pieces among the first of `xs` that are at vertex `v`, in order. -/
def occ (xs : List Nat) (v : Nat) : List Nat :=
  (List.range xs.length).filter fun i => xs[i]? == some v

theorem occ_take_succ (xs : List Nat) (e : Nat) (he : e < xs.length) (v : Nat) :
    occ (xs.take (e + 1)) v = occ (xs.take e) v ++ (if xs[e] = v then [e] else []) := by
  have l1 : (xs.take (e + 1)).length = e + 1 := by simp; omega
  have l0 : (xs.take e).length = e := by simp; omega
  simp only [occ, l1, l0, List.range_succ, List.filter_append]
  congr 1
  · apply List.filter_congr
    intro i hi
    simp at hi
    simp [List.getElem?_take, hi, show i < e + 1 by omega]
  · simp [List.getElem?_take, he]
    split <;> simp_all

theorem occ_lt (xs : List Nat) (v : Nat) : ∀ i ∈ occ xs v, i < xs.length := by
  intro i hi; simp [occ] at hi; exact hi.1

theorem occ_length_le (xs : List Nat) (v : Nat) : (occ xs v).length ≤ xs.length := by
  have := List.length_filter_le (fun i => xs[i]? == some v) (List.range xs.length)
  simpa [occ] using this

/-- `u32::MAX`, "none". -/
def NONE : Nat := U32.max

/-- The five functions of the first `e` pieces. -/
def nOut (F : List Nat) (e v : Nat) : Nat := (occ (F.take e) v).length
def nIn (T : List Nat) (e v : Nat) : Nat := (occ (T.take e) v).length
def fOut (F : List Nat) (e v : Nat) : Nat := (occ (F.take e) v).getD 0 NONE
def fIn (T : List Nat) (e v : Nat) : Nat := (occ (T.take e) v).getD 0 NONE
def sIn (T : List Nat) (e v : Nat) : Nat := (occ (T.take e) v).getD 1 NONE

/-- A vector holding `g` at every vertex. -/
def Holds (v : alloc.vec.Vec U32) (nv : Nat) (g : Nat → Nat) : Prop :=
  vals v = (List.range nv).map g

theorem holds_get {v : alloc.vec.Vec U32} {nv : Nat} {g : Nat → Nat} (h : Holds v nv g) {i : Nat}
    (hi : i < nv) (hl : i < v.val.length) : v.val[i].val = g i := by
  have := congrArg (fun l => l[i]?) h
  simp only [vals, List.getElem?_map, List.getElem?_range hi, Option.map_some] at this
  rw [List.getElem?_eq_getElem hl] at this
  simpa using this

theorem holds_len {v : alloc.vec.Vec U32} {nv : Nat} {g : Nat → Nat} (h : Holds v nv g) :
    v.val.length = nv := by
  have := congrArg List.length h; simpa [vals] using this

theorem holds_set {v w : alloc.vec.Vec U32} {nv : Nat} {g : Nat → Nat} (h : Holds v nv g) (i : Nat)
    (x : U32) (hw : w.val = v.val.set i x) (g' : Nat → Nat)
    (hg : ∀ j, j < nv → g' j = if j = i then x.val else g j) : Holds w nv g' := by
  have hl := holds_len h
  unfold Holds
  apply List.ext_getElem
  · simp [vals, hw, hl]
  · intro j h1 h2
    have hj : j < nv := by simpa using h2
    have hjl : j < v.val.length := by omega
    simp only [vals, List.getElem_map, List.getElem_range, hw, List.getElem_set]
    rw [hg j hj]
    by_cases e : i = j
    · subst e; simp
    · rw [if_neg e, if_neg (Ne.symm e)]; exact holds_get h hj hjl

/-! ## One piece more -/

theorem getD0_snoc (l : List Nat) (e d : Nat) :
    (l ++ [e]).getD 0 d = if l.length = 0 then e else l.getD 0 d := by
  cases l <;> simp

theorem getD1_snoc (l : List Nat) (e d : Nat) :
    (l ++ [e]).getD 1 d = if l.length = 1 then e else l.getD 1 d := by
  rcases l with _ | ⟨a, _ | ⟨b, l⟩⟩ <;> simp

section step
variable (xs : List Nat) (e : Nat) (he : e < xs.length) (v : Nat)
include he

theorem cnt_step : (occ (xs.take (e + 1)) v).length =
    (occ (xs.take e) v).length + if xs[e] = v then 1 else 0 := by
  rw [occ_take_succ xs e he]; split <;> simp

theorem first_step : (occ (xs.take (e + 1)) v).getD 0 NONE =
    if xs[e] = v ∧ (occ (xs.take e) v).length = 0 then e else (occ (xs.take e) v).getD 0 NONE := by
  rw [occ_take_succ xs e he]
  by_cases h : xs[e] = v
  · simp only [h, if_true, true_and]; exact getD0_snoc _ _ _
  · simp [h]

theorem second_step : (occ (xs.take (e + 1)) v).getD 1 NONE =
    if xs[e] = v ∧ (occ (xs.take e) v).length = 1 then e else (occ (xs.take e) v).getD 1 NONE := by
  rw [occ_take_succ xs e he]
  by_cases h : xs[e] = v
  · simp only [h, if_true, true_and]; exact getD1_snoc _ _ _
  · simp [h]

end step

/-! ## The first loop -/

/-- A slice of `u32` as the model's list. -/
def sv (x : Slice U32) : List Nat := x.val.map (·.val)

theorem sv_get (x : Slice U32) (i : Nat) (h : i < x.val.length) : (sv x)[i]'(by simp [sv, h]) = x.val[i].val := by
  simp [sv]

theorem u32_of_lt (e : Usize) (b : Nat) (h : e.val < b) (hb : b ≤ U32.max) (r : U32)
    (hr : UScalar.cast .U32 e = r) : r.val = e.val := by
  rw [← hr]; simp only [UScalar.cast_val_eq]; apply Nat.mod_eq_of_lt
  have : U32.max + 1 = 2 ^ UScalarTy.U32.numBits := by simp [U32.max_eq, UScalarTy.U32_numBits_eq]
  omega

theorem u32_inc (a : U32) (b : Nat) (h : a.val < b) (hb : b ≤ U32.max) :
    (a.wrapping_add 1#u32).val = a.val + 1 := by
  rw [U32.wrapping_add_val_eq]; apply Nat.mod_eq_of_lt
  have h1 : (1#u32 : U32).val = 1 := by simp
  have hs : UScalar.size .U32 = 2 ^ 32 := by simp [U32.size, U32.numBits]
  have hm : U32.max = 2 ^ 32 - 1 := by simp [U32.max_eq]
  rw [h1, hs]; omega

/-- The first block: the first leaving piece. -/
theorem fo_block (F : List Nat) (nv : Nat) (fo : alloc.vec.Vec U32) (f j : Usize) (c2 : U32)
    (hfo : Holds fo nv (fOut F j.val)) (hc2 : c2.val = nOut F j.val f.val)
    (hjF : j.val < F.length) (hfv : F[j.val] = f.val) (hfn : f.val < nv) (hjm : j.val < U32.max) :
    (if c2 = 0#u32 then do
        let (_, index_mut_back) ← fo.index_mut_usize f
        let i3 ← lift (UScalar.cast UScalarTy.U32 j)
        ok (index_mut_back i3)
      else ok fo) ⦃ fo1 => Holds fo1 nv (fOut F (j.val + 1)) ⦄ := by
  have hfl : f.val < fo.val.length := by rw [holds_len hfo]; exact hfn
  split
  · rename_i h0
    have h0' : nOut F j.val f.val = 0 := by rw [← hc2, h0]; rfl
    step with alloc.vec.Vec.index_mut_usize_spec fo f (by simpa using hfl) as ⟨w, back, hw, hback⟩
    step as ⟨i3, hi3⟩
    have hi3v : i3.val = j.val := u32_of_lt j U32.max hjm le_rfl i3 hi3.symm
    apply holds_set hfo f.val i3 (by rw [hback]; simp) _
    intro v hv
    simp only [fOut]
    rw [first_step F j.val hjF v, hi3v]
    by_cases e : v = f.val
    · subst e
      have hnil : occ (F.take j.val) f.val = [] := List.eq_nil_of_length_eq_zero h0'
      simp [hfv, hnil, nOut]
    · simp [e, hfv, Ne.symm e]
  · rename_i h0
    have h0' : nOut F j.val f.val ≠ 0 := by
      intro e; apply h0; apply UScalar.eq_of_val_eq; rw [hc2, e]; rfl
    simp only [WP.spec_ok]
    unfold Holds at hfo ⊢
    rw [hfo]
    apply List.map_congr_left
    intro v _
    simp only [fOut]
    rw [first_step F j.val hjF v, if_neg]
    rintro ⟨h1, h2⟩
    rw [hfv] at h1
    subst h1
    exact h0' h2

/-- The second block: the first two arriving pieces (and the leaving count's write-back). -/
theorem in_block (T : List Nat) (nv : Nat) (fi si : alloc.vec.Vec U32) (t j : Usize) (i5 : U32)
    (hfi : Holds fi nv (fIn T j.val)) (hsi : Holds si nv (sIn T j.val))
    (hi5 : i5.val = nIn T j.val t.val)
    (hjT : j.val < T.length) (htv : T[j.val] = t.val) (htn : t.val < nv) (hjm : j.val < U32.max)
    (back : U32 → alloc.vec.Vec U32) (i4 : U32) :
    (if i5 = 0#u32 then do
        let (_, index_mut_back1) ← fi.index_mut_usize t
        let i6 ← lift (UScalar.cast UScalarTy.U32 j)
        ok (back i4, index_mut_back1 i6, si)
      else do
        let (v, v1) ←
          if i5 = 1#u32 then do
              let (_, index_mut_back1) ← si.index_mut_usize t
              let i6 ← lift (UScalar.cast UScalarTy.U32 j)
              ok (back i4, index_mut_back1 i6)
            else ok (back i4, si)
        ok (v, fi, v1)) ⦃ r => r.1 = back i4 ∧ Holds r.2.1 nv (fIn T (j.val + 1)) ∧
          Holds r.2.2 nv (sIn T (j.val + 1)) ⦄ := by
  have hfl : t.val < fi.val.length := by rw [holds_len hfi]; exact htn
  have hsl : t.val < si.val.length := by rw [holds_len hsi]; exact htn
  -- what is unchanged where `T[j]` is not `v`, or the count is not the one written at
  have keep : ∀ (G : List Nat → Nat → Nat) (k : Nat) (vec : alloc.vec.Vec U32)
      (hG : ∀ v, G (T.take (j.val + 1)) v = if T[j.val] = v ∧ nIn T j.val v = k then j.val
        else G (T.take j.val) v),
      Holds vec nv (fun v => G (T.take j.val) v) → nIn T j.val t.val ≠ k →
      Holds vec nv (fun v => G (T.take (j.val + 1)) v) := by
    intro G k vec hG hv hne
    unfold Holds at hv ⊢
    rw [hv]
    apply List.map_congr_left
    intro v _
    rw [hG v, if_neg]
    rintro ⟨h1, h2⟩
    rw [htv] at h1; subst h1; exact hne h2
  have setAt : ∀ (G : List Nat → Nat → Nat) (k : Nat) (vec w : alloc.vec.Vec U32) (x : U32)
      (hG : ∀ v, G (T.take (j.val + 1)) v = if T[j.val] = v ∧ nIn T j.val v = k then j.val
        else G (T.take j.val) v),
      Holds vec nv (fun v => G (T.take j.val) v) → nIn T j.val t.val = k → x.val = j.val →
      w.val = vec.val.set t.val x → Holds w nv (fun v => G (T.take (j.val + 1)) v) := by
    intro G k vec w x hG hv hk hx hw
    apply holds_set hv t.val x hw
    intro v _
    rw [hG v]
    by_cases e : v = t.val
    · subst e; simp [htv, hk, hx]
    · simp [e]; intro h; exact absurd h.symm (by rw [htv]; exact e)
  have Gf : ∀ v, (fun xs v => (occ xs v).getD 0 NONE) (T.take (j.val + 1)) v =
      if T[j.val] = v ∧ nIn T j.val v = 0 then j.val
      else (fun xs v => (occ xs v).getD 0 NONE) (T.take j.val) v := fun v => first_step T j.val hjT v
  have Gs : ∀ v, (fun xs v => (occ xs v).getD 1 NONE) (T.take (j.val + 1)) v =
      if T[j.val] = v ∧ nIn T j.val v = 1 then j.val
      else (fun xs v => (occ xs v).getD 1 NONE) (T.take j.val) v := fun v => second_step T j.val hjT v
  split
  · rename_i h0
    have h0' : nIn T j.val t.val = 0 := by rw [← hi5, h0]; rfl
    step with alloc.vec.Vec.index_mut_usize_spec fi t (by simpa using hfl) as ⟨w, b1, hw, hb1⟩
    step as ⟨i6, hi6⟩
    have hi6v : i6.val = j.val := u32_of_lt j U32.max hjm le_rfl i6 hi6.symm
    exact ⟨setAt (fun xs v => (occ xs v).getD 0 NONE) 0 fi (b1 i6) i6 Gf hfi h0' hi6v
        (by rw [hb1]; simp),
      keep (fun xs v => (occ xs v).getD 1 NONE) 1 si Gs hsi (by omega)⟩
  · rename_i h0
    have h0' : nIn T j.val t.val ≠ 0 := by
      intro e; apply h0; apply UScalar.eq_of_val_eq; rw [hi5, e]; rfl
    split
    · rename_i h1
      have h1' : nIn T j.val t.val = 1 := by rw [← hi5, h1]; rfl
      have inner : (do
          let (_, index_mut_back1) ← si.index_mut_usize t
          let i6 ← lift (UScalar.cast UScalarTy.U32 j)
          ok (back i4, index_mut_back1 i6)) ⦃ r => r.1 = back i4 ∧
            Holds r.2 nv (sIn T (j.val + 1)) ⦄ := by
        step with alloc.vec.Vec.index_mut_usize_spec si t (by simpa using hsl) as ⟨w, b1, hw, hb1⟩
        step as ⟨i6, hi6⟩
        have hi6v : i6.val = j.val := u32_of_lt j U32.max hjm le_rfl i6 hi6.symm
        exact setAt (fun xs v => (occ xs v).getD 1 NONE) 1 si (b1 i6) i6 Gs hsi h1' hi6v
          (by rw [hb1]; simp)
      apply WP.spec_bind inner
      rintro ⟨v, v1⟩ ⟨hv1, hv2⟩
      show (ok (v, fi, v1)) ⦃ r => r.1 = back i4 ∧ Holds r.2.1 nv (fIn T (j.val + 1)) ∧
        Holds r.2.2 nv (sIn T (j.val + 1)) ⦄
      simp only [WP.spec_ok]
      exact ⟨hv1, keep (fun xs v => (occ xs v).getD 0 NONE) 0 fi Gf hfi h0', hv2⟩
    · rename_i h1
      have h1' : nIn T j.val t.val ≠ 1 := by
        intro e; apply h1; apply UScalar.eq_of_val_eq; rw [hi5, e]; rfl
      apply WP.spec_bind (Pₘ := fun r => r = (back i4, si)) (by simp)
      rintro ⟨v, v1⟩ hv
      simp only [Prod.mk.injEq] at hv
      obtain ⟨e1, e2⟩ := hv
      rw [e1, e2]
      show (ok (back i4, fi, si)) ⦃ r => r.1 = back i4 ∧ Holds r.2.1 nv (fIn T (j.val + 1)) ∧
        Holds r.2.2 nv (sIn T (j.val + 1)) ⦄
      simp only [WP.spec_ok]
      exact ⟨trivial, keep (fun xs v => (occ xs v).getD 0 NONE) 0 fi Gf hfi h0',
        keep (fun xs v => (occ xs v).getD 1 NONE) 1 si Gs hsi h1'⟩

/-- The loop's state at piece `e`. -/
structure St0 (F T : List Nat) (nv e : Nat) (no ni fo fi si : alloc.vec.Vec U32) : Prop where
  no : Holds no nv (nOut F e)
  ni : Holds ni nv (nIn T e)
  fo : Holds fo nv (fOut F e)
  fi : Holds fi nv (fIn T e)
  si : Holds si nv (sIn T e)

theorem loop0_chain_spec (fr dst : Slice U32) (m : Usize) (hm : m.val = fr.val.length)
    (ht : m.val ≤ dst.val.length) (hmax : m.val ≤ U32.max) (nv : Nat)
    (hF : ∀ x ∈ sv fr, x < nv) (hT : ∀ x ∈ sv dst, x < nv)
    (no ni fo fi si : alloc.vec.Vec U32) (e : Usize) (he : e.val ≤ m.val)
    (hs : St0 (sv fr) (sv dst) nv e.val no ni fo fi si) :
    chain_loop0 fr dst m no ni fo fi si e ⦃ r =>
      St0 (sv fr) (sv dst) nv m.val r.1 r.2.1 r.2.2.1 r.2.2.2.1 r.2.2.2.2 ⦄ := by
  unfold chain_loop0
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec U32 × alloc.vec.Vec U32 × alloc.vec.Vec U32 × alloc.vec.Vec U32 ×
      alloc.vec.Vec U32 × Usize) => m.val - q.2.2.2.2.2.val)
    (fun q => q.2.2.2.2.2.val ≤ m.val ∧
      St0 (sv fr) (sv dst) nv q.2.2.2.2.2.val q.1 q.2.1 q.2.2.1 q.2.2.2.1 q.2.2.2.2.1)
  · rintro ⟨no, ni, fo, fi, si, j⟩ ⟨hj, hst⟩
    unfold chain_loop0.body
    simp only at hj hst ⊢
    split
    · rename_i h
      have hjl : j.val < m.val := by scalar_tac
      have hjF : j.val < (sv fr).length := by simp [sv]; omega
      have hjT : j.val < (sv dst).length := by simp [sv]; omega
      have hfv : (sv fr)[j.val] < nv := hF _ (List.getElem_mem hjF)
      have htv : (sv dst)[j.val] < nv := hT _ (List.getElem_mem hjT)
      obtain ⟨hno, hni, hfo, hfi, hsi⟩ := hst
      step with Slice.index_usize_spec fr j (by simp; omega) as ⟨a, ha⟩
      step as ⟨f, hf⟩
      have hfval : f.val = (sv fr)[j.val] := by
        rw [hf, U32.cast_Usize_val_eq, ha]; simp [sv]
      step with Slice.index_usize_spec dst j (by simp; omega) as ⟨b, hb⟩
      step as ⟨t, htt⟩
      have htval : t.val = (sv dst)[j.val] := by
        rw [htt, U32.cast_Usize_val_eq, hb]; simp [sv]
      have hfl : f.val < no.val.length := by rw [holds_len hno]; omega
      step as ⟨c2, hc2⟩
      have hc2v : c2.val = nOut (sv fr) j.val f.val := by
        rw [hc2]; exact holds_get hno (by omega) hfl
      have hjm : j.val < U32.max := by omega
      step with fo_block (sv fr) nv fo f j c2 hfo hc2v hjF hfval.symm (by omega) hjm as ⟨fo1, hfo1⟩
      step with alloc.vec.Vec.index_mut_usize_spec no f (by simpa using hfl) as ⟨i3, back, hi3, hback⟩
      have hi3v : i3.val = nOut (sv fr) j.val f.val := by
        rw [hi3]; exact holds_get hno (by omega) hfl
      have hnoj : nOut (sv fr) j.val f.val ≤ j.val := by
        have := occ_length_le ((sv fr).take j.val) f.val; simp [nOut] at this ⊢; omega
      step as ⟨i4, hi4⟩
      have hi4v : i4.val = nOut (sv fr) j.val f.val + 1 := by
        rw [hi4, u32_inc i3 (j.val + 1) (by omega) (by omega), hi3v]
      have htl : t.val < ni.val.length := by rw [holds_len hni]; omega
      step as ⟨i5, hi5⟩
      have hi5v : i5.val = nIn (sv dst) j.val t.val := by
        rw [hi5]; exact holds_get hni (by omega) htl
      step with in_block (sv dst) nv fi si t j i5 hfi hsi hi5v hjT htval.symm (by omega) hjm back i4
        as ⟨no1, fi1, si1, hno1, hfi1, hsi1⟩
      step with alloc.vec.Vec.index_mut_usize_spec ni t (by simpa using htl) as ⟨i6, back1, hi6, hback1⟩
      have hi6v : i6.val = nIn (sv dst) j.val t.val := by
        rw [hi6]; exact holds_get hni (by omega) htl
      have hnij : nIn (sv dst) j.val t.val ≤ j.val := by
        have := occ_length_le ((sv dst).take j.val) t.val; simp [nIn] at this ⊢; omega
      step as ⟨i7, hi7⟩
      have hi7v : i7.val = nIn (sv dst) j.val t.val + 1 := by
        rw [hi7, u32_inc i6 (j.val + 1) (by omega) (by omega), hi6v]
      step as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by rw [hj1]; exact usize_inc j m hjl
      refine ⟨by omega, ⟨?_, ?_, ?_, by rw [hj1v]; exact hfi1, by rw [hj1v]; exact hsi1⟩, by omega⟩
      · rw [hj1v]
        apply holds_set hno f.val i4 (by rw [hno1, hback]; simp)
        intro v hv
        simp only [nOut]
        rw [cnt_step (sv fr) j.val hjF v]
        by_cases e : v = f.val
        · subst e; simp [← hfval, hi4v, nOut]
        · simp [e, ← hfval, Ne.symm e, nOut]
      · rw [hj1v]
        apply holds_set hni t.val i7 (by rw [hback1]; simp)
        intro v hv
        simp only [nIn]
        rw [cnt_step (sv dst) j.val hjT v]
        by_cases e : v = t.val
        · subst e; simp [← htval, hi7v, nIn]
        · simp [e, ← htval, Ne.symm e, nIn]
      · rw [hj1v]; exact hfo1
    · rename_i h
      have : j.val = m.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [← this]; exact hst
  · exact ⟨he, hs⟩

end forge_sheet_core
