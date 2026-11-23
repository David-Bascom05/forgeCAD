import ForgeSheetCore.RefineClasses
import ForgeSheetCore.WalkModel
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

/-! ## The second loop: the verdict on degrees -/

/-- An `Unchained` verdict at vertex `u`, as the model has it. -/
def unchainedAt (NO NI FO FI : Nat → Nat) (u : Nat) (b : ChainError) : Prop :=
  ∃ a pc l r, b = .Unchained a pc l r ∧ a.val = u ∧ l.val = NO u ∧ r.val = NI u ∧
    pc.val = (if NO u > 0 then FO u else FI u)

/-- The pinches below `v`: every vertex left (and so reached) more than once. -/
def pinchList (NO FI SI : Nat → Nat) (v : Nat) : List (Nat × Nat × Nat) :=
  ((List.range v).filter fun u => 1 < NO u).map fun u => (u, FI u, SI u)

def tvals (p : alloc.vec.Vec (U32 × U32 × U32)) : List (Nat × Nat × Nat) :=
  p.val.map fun x => (x.1.val, x.2.1.val, x.2.2.val)

/-- What the second loop decides: the first vertex with unequal degrees is `Unchained`;
with none, the pinches are every vertex of degree past one, in order. -/
def Verdict1 (nv : Nat) (NO NI FO FI SI : Nat → Nat) (bad : Option ChainError)
    (pinches : alloc.vec.Vec (U32 × U32 × U32)) : Prop :=
  (bad = none ∧ (∀ u < nv, NO u = NI u) ∧ tvals pinches = pinchList NO FI SI nv) ∨
  (∃ u < nv, (∀ u' < u, NO u' = NI u') ∧ NO u ≠ NI u ∧ ∃ b, bad = some b ∧ unchainedAt NO NI FO FI u b)

theorem pinch_succ (NO FI SI : Nat → Nat) (v : Nat) :
    pinchList NO FI SI (v + 1) = pinchList NO FI SI v ++ (if 1 < NO v then [(v, FI v, SI v)] else []) := by
  simp only [pinchList, List.range_succ, List.filter_append, List.map_append]
  split <;> simp_all

theorem u32_of_lt' (v : Usize) (b : Nat) (h : v.val < b) (hb : b ≤ U32.max + 1) (r : U32)
    (hr : UScalar.cast .U32 v = r) : r.val = v.val := by
  rw [← hr]; simp only [UScalar.cast_val_eq]; apply Nat.mod_eq_of_lt
  have : U32.max + 1 = 2 ^ UScalarTy.U32.numBits := by simp [U32.max_eq, UScalarTy.U32_numBits_eq]
  omega

theorem unchained_block (nv : Nat) (NO NI FO FI : Nat → Nat) (fo fi : alloc.vec.Vec U32)
    (hfo : Holds fo nv FO) (hfi : Holds fi nv FI) (v : Usize) (hv : v.val < nv)
    (hnv : nv ≤ U32.max) (a c : U32) (ha : a.val = NO v.val) (hc : c.val = NI v.val)
    (pin : alloc.vec.Vec (U32 × U32 × U32)) :
    (do
      let piece ← if a > 0#u32 then fo.index_usize v else fi.index_usize v
      let i2 ← lift (UScalar.cast UScalarTy.U32 v)
      ok (some (ChainError.Unchained i2 piece a c), pin)) ⦃ r => r.2 = pin ∧
        ∃ b, r.1 = some b ∧ unchainedAt NO NI FO FI v.val b ⦄ := by
  have hfl : v.val < fo.val.length := by rw [holds_len hfo]; exact hv
  have hil : v.val < fi.val.length := by rw [holds_len hfi]; exact hv
  split
  · rename_i hpos
    step with alloc.vec.Vec.index_usize_spec fo v (by simpa using hfl) as ⟨pc, hpc⟩
    step as ⟨i2, hi2⟩
    refine ⟨_, rfl, i2, pc, a, c, rfl, u32_of_lt' v nv hv (by omega) i2 hi2.symm, ha, hc, ?_⟩
    have : NO v.val > 0 := by rw [← ha]; scalar_tac
    rw [if_pos this, hpc]; exact holds_get hfo hv hfl
  · rename_i hpos
    step with alloc.vec.Vec.index_usize_spec fi v (by simpa using hil) as ⟨pc, hpc⟩
    step as ⟨i2, hi2⟩
    refine ⟨_, rfl, i2, pc, a, c, rfl, u32_of_lt' v nv hv (by omega) i2 hi2.symm, ha, hc, ?_⟩
    have : ¬ NO v.val > 0 := by rw [← ha]; scalar_tac
    rw [if_neg this, hpc]; exact holds_get hfi hv hil

theorem pinch_block (nv : Nat) (NO FI SI : Nat → Nat) (fi si : alloc.vec.Vec U32)
    (hfi : Holds fi nv FI) (hsi : Holds si nv SI) (v : Usize) (hv : v.val < nv)
    (hnv : nv ≤ U32.max) (a : U32) (ha : a.val = NO v.val) (bad : Option ChainError)
    (pin : alloc.vec.Vec (U32 × U32 × U32)) (hpl : pin.val.length < nv) :
    (do
      let v1 ←
        if a > 1#u32 then do
            let i2 ← lift (UScalar.cast UScalarTy.U32 v)
            let i3 ← fi.index_usize v
            let i4 ← si.index_usize v
            pin.push (i2, i3, i4)
          else ok pin
      ok (bad, v1)) ⦃ r => r.1 = bad ∧
        tvals r.2 = tvals pin ++ (if 1 < NO v.val then [(v.val, FI v.val, SI v.val)] else []) ⦄ := by
  have hil : v.val < fi.val.length := by rw [holds_len hfi]; exact hv
  have hsl : v.val < si.val.length := by rw [holds_len hsi]; exact hv
  split
  · rename_i hgt
    step as ⟨i2, hi2⟩
    step with alloc.vec.Vec.index_usize_spec fi v (by simpa using hil) as ⟨i3, hi3⟩
    step with alloc.vec.Vec.index_usize_spec si v (by simpa using hsl) as ⟨i4, hi4⟩
    step with alloc.vec.Vec.push_spec pin (i2, i3, i4) (by
      have : nv ≤ Usize.max := by scalar_tac
      omega) as ⟨p2, hp2⟩
    have : 1 < NO v.val := by rw [← ha]; scalar_tac
    rw [if_pos this]
    simp only [tvals, hp2, List.map_append, List.map_cons, List.map_nil]
    rw [u32_of_lt' v nv hv (by omega) i2 hi2.symm, hi3, hi4, holds_get hfi hv hil,
      holds_get hsi hv hsl]
  · rename_i hgt
    simp only [Std.bind_ok, WP.spec_ok]
    refine ⟨trivial, ?_⟩
    have : ¬ 1 < NO v.val := by rw [← ha]; scalar_tac
    rw [if_neg this]; simp

theorem loop1_chain_spec (nv : Usize) (hnv : nv.val ≤ U32.max) (no ni fo fi si : alloc.vec.Vec U32)
    (NO NI FO FI SI : Nat → Nat) (hno : Holds no nv.val NO) (hni : Holds ni nv.val NI)
    (hfo : Holds fo nv.val FO) (hfi : Holds fi nv.val FI) (hsi : Holds si nv.val SI)
    (pinches : alloc.vec.Vec (U32 × U32 × U32)) (hp0 : pinches.val = []) :
    chain_loop1 nv no ni fo fi si none pinches 0#usize ⦃ r => Verdict1 nv.val NO NI FO FI SI r.1 r.2 ⦄ := by
  unfold chain_loop1
  apply loop.spec_decr_nat
    (fun (q : Option ChainError × alloc.vec.Vec (U32 × U32 × U32) × Usize) =>
      nv.val + 1 - q.2.2.val - (if q.1.isSome then 1 else 0))
    (fun q => q.2.2.val ≤ nv.val ∧
      ((q.1 = none ∧ (∀ u < q.2.2.val, NO u = NI u) ∧ tvals q.2.1 = pinchList NO FI SI q.2.2.val) ∨
       (∃ u < nv.val, (∀ u' < u, NO u' = NI u') ∧ NO u ≠ NI u ∧ ∃ b, q.1 = some b ∧
          unchainedAt NO NI FO FI u b)))
  · rintro ⟨bad, pin, v⟩ ⟨hv, hst⟩
    unfold chain_loop1.body
    simp only at hv hst ⊢
    split
    · rename_i h
      have hvl : v.val < nv.val := by scalar_tac
      split
      · rename_i hnone
        have hb : bad = none := by cases bad <;> simp_all [core.option.Option.is_none]
        rcases hst with ⟨_, heq, hpin⟩ | ⟨u, _, _, _, b, hbs, _⟩
        swap
        · rw [hb] at hbs; cases hbs
        have hl := holds_len hno
        have hl2 := holds_len hni
        step as ⟨a, ha⟩
        step as ⟨c, hc⟩
        have hav : a.val = NO v.val := by rw [ha]; exact holds_get hno hvl (by omega)
        have hcv : c.val = NI v.val := by rw [hc]; exact holds_get hni hvl (by omega)
        have hj1 : ∀ v1 : Usize, v1 = v.wrapping_add 1#usize → v1.val = v.val + 1 :=
          fun v1 e => by rw [e]; exact usize_inc v nv hvl
        split
        · rename_i hne
          have hne' : NO v.val ≠ NI v.val := by
            rw [← hav, ← hcv]; intro e; apply (bne_iff_ne.mp hne); exact UScalar.eq_of_val_eq e
          apply WP.spec_bind (unchained_block nv.val NO NI FO FI fo fi hfo hfi v hvl hnv a c hav hcv pin)
          rintro ⟨bad1, pin1⟩ ⟨hp, b, hb, hu⟩
          show WP.spec (do
            let v1 ← lift (Std.Usize.wrapping_add v 1#usize)
            ok (ControlFlow.cont (bad1, pin1, v1))) _
          step as ⟨v1, hv1⟩
          have hv1v := hj1 v1 hv1
          refine ⟨by omega, Or.inr ⟨v.val, hvl, heq, hne', b, hb, hu⟩, ?_⟩
          simp only at hb ⊢; rw [hb, hv1v]; simp [‹bad = none›]; omega
        · rename_i hne
          have heqv : NO v.val = NI v.val := by
            rw [← hav, ← hcv]; simp at hne; rw [hne]
          have hpl : pin.val.length < nv.val := by
            have : (tvals pin).length = (pinchList NO FI SI v.val).length := by rw [hpin]
            have h2 : (pinchList NO FI SI v.val).length ≤ v.val := by
              simp only [pinchList, List.length_map]
              have := List.length_filter_le (fun u => 1 < NO u) (List.range v.val)
              simpa using this
            simp [tvals] at this; omega
          apply WP.spec_bind (pinch_block nv.val NO FI SI fi si hfi hsi v hvl hnv a hav bad pin hpl)
          rintro ⟨bad1, pin1⟩ ⟨hb, hp⟩
          show WP.spec (do
            let v1 ← lift (Std.Usize.wrapping_add v 1#usize)
            ok (ControlFlow.cont (bad1, pin1, v1))) _
          step as ⟨v1, hv1⟩
          have hv1v := hj1 v1 hv1
          simp only at hb hp
          refine ⟨by omega, Or.inl ⟨by rw [hb, ‹bad = none›], ?_, ?_⟩, ?_⟩
          · intro u hu
            rw [hv1v] at hu
            rcases Nat.lt_succ_iff_lt_or_eq.mp hu with h | h
            · exact heq u h
            · rw [h]; exact heqv
          · rw [hp, hv1v, pinch_succ, hpin]
          · rw [hb, ‹bad = none›, hv1v]; simp; omega
      · rename_i hsome
        simp only [WP.spec_ok]
        rcases hst with ⟨hb, _, _⟩ | h2
        · rw [hb] at hsome; simp [core.option.Option.is_none] at hsome
        · exact Or.inr h2
    · rename_i h
      have : v.val = nv.val := by scalar_tac
      simp only [WP.spec_ok]
      rcases hst with ⟨hb, heq, hpin⟩ | h2
      · exact Or.inl ⟨hb, by rw [← this]; exact heq, by rw [← this]; exact hpin⟩
      · exact Or.inr h2
  · exact ⟨by simp, Or.inl ⟨rfl, fun u hu => by simp at hu, by simp [tvals, hp0, pinchList]⟩⟩

/-! ## The successor is a permutation -/

theorem mem_occ (xs : List Nat) (v i : Nat) : i ∈ occ xs v ↔ ∃ h : i < xs.length, xs[i] = v := by
  simp only [occ, List.mem_filter, List.mem_range, beq_iff_eq]
  constructor
  · rintro ⟨h, h2⟩; exact ⟨h, by rw [List.getElem?_eq_getElem h] at h2; simpa using h2⟩
  · rintro ⟨h, h2⟩; exact ⟨h, by rw [List.getElem?_eq_getElem h, h2]⟩

/-- The successor of a piece: the first piece leaving the vertex it arrives at. -/
def succ (F T : List Nat) (e : Nat) : Nat := fOut F F.length (T.getD e 0)

theorem take_len (xs : List Nat) : xs.take xs.length = xs := List.take_length

/-- **With the degrees passed, the successor is a permutation** of the pieces, and each piece
leaves where its predecessor arrives. -/
theorem succ_perm (F T : List Nat) (nv : Nat) (hlen : F.length ≤ T.length)
    (hT : ∀ x ∈ T, x < nv)
    (hdeg : ∀ v < nv, nOut F F.length v = nIn T F.length v ∧ nOut F F.length v ≤ 1) :
    ForgeSheetCore.Walk.Perm (succ F T) F.length ∧
      ∀ e < F.length, ∃ h : succ F T e < F.length, F[succ F T e] = T.getD e 0 := by
  -- the piece leaving `T[e]`
  have key : ∀ e < F.length, succ F T e ∈ occ F (T.getD e 0) := by
    intro e he
    have hel : e < T.length := by omega
    have hw : T.getD e 0 < nv := by
      rw [List.getD_eq_getElem _ _ hel]; exact hT _ (List.getElem_mem hel)
    have hin : e ∈ occ (T.take F.length) (T.getD e 0) := by
      rw [mem_occ]
      refine ⟨by simp; omega, ?_⟩
      rw [List.getElem_take, List.getD_eq_getElem _ _ hel]
    have h1 : 0 < nOut F F.length (T.getD e 0) := by
      rw [(hdeg _ hw).1]; simp only [nIn]
      exact List.length_pos_of_mem hin
    simp only [nOut, take_len] at h1
    simp only [succ, fOut, take_len]
    rw [List.getD_eq_getElem _ _ h1]
    exact List.getElem_mem h1
  have meets : ∀ e < F.length, ∃ h : succ F T e < F.length, F[succ F T e] = T.getD e 0 :=
    fun e he => (mem_occ _ _ _).mp (key e he)
  refine ⟨⟨fun e he => (meets e he).1, fun e e' he he' hs => ?_⟩, meets⟩
  -- one piece arrives at each vertex
  obtain ⟨h1, h2⟩ := meets e he
  obtain ⟨h1', h2'⟩ := meets e' he'
  have hw : T.getD e 0 = T.getD e' 0 := by rw [← h2, ← h2']; simp [hs]
  have hel : e < T.length := by omega
  have hel' : e' < T.length := by omega
  have hv : T.getD e 0 < nv := by
    rw [List.getD_eq_getElem _ _ hel]; exact hT _ (List.getElem_mem hel)
  have hle : (occ (T.take F.length) (T.getD e 0)).length ≤ 1 := by
    have := (hdeg _ hv); simp only [nIn] at this; omega
  have m1 : e ∈ occ (T.take F.length) (T.getD e 0) := by
    rw [mem_occ]; exact ⟨by simp; omega, by rw [List.getElem_take, List.getD_eq_getElem _ _ hel]⟩
  have m2 : e' ∈ occ (T.take F.length) (T.getD e 0) := by
    rw [mem_occ, hw]; exact ⟨by simp; omega, by rw [List.getElem_take, List.getD_eq_getElem _ _ hel']⟩
  match h : occ (T.take F.length) (T.getD e 0), hle with
  | [], _ => rw [h] at m1; cases m1
  | [x], _ => rw [h] at m1 m2; simp at m1 m2; omega
  | _ :: _ :: _, hl => simp at hl

/-! ## The third loop: walks -/

theorem u32max_le_usize : U32.max ≤ Usize.max := by scalar_tac

open ForgeSheetCore.Walk in
/-- The marks agree with a list of pieces. -/
def Marks (used : alloc.vec.Vec Bool) (m : Nat) (P : List Nat) : Prop :=
  used.val.length = m ∧ ∀ e < m, used.val[e]? = some (decide (e ∈ P))

open ForgeSheetCore.Walk in
theorem walk_spec (fr dst : Slice U32) (m : Nat) (hm : m = fr.val.length) (hdl : m ≤ dst.val.length)
    (hmax : m ≤ U32.max) (nv : Nat) (hT : ∀ x ∈ sv dst, x < nv)
    (fo : alloc.vec.Vec U32) (hfo : Holds fo nv (fOut (sv fr) m))
    (hp : Perm (succ (sv fr) (sv dst)) m)
    (P0 : List Nat) (hnd0 : P0.Nodup) (hlt0 : ∀ e ∈ P0, e < m)
    (hpre : ∀ e < m, succ (sv fr) (sv dst) e ∈ P0 → e ∈ P0)
    (s : Usize) (hs : s.val < m) (hs0 : s.val ∉ P0)
    (used : alloc.vec.Vec Bool) (hu : Marks used m P0)
    (pieces : alloc.vec.Vec U32) (hP : vals pieces = P0) :
    chain_loop2_loop0 dst fo used pieces s ⦃ r =>
      ∃ k, 0 < k ∧ vals r.2.1 = P0 ++ path (succ (sv fr) (sv dst)) s.val k ∧
        (succ (sv fr) (sv dst))^[k] s.val = s.val ∧ r.2.2.val = s.val ∧
        Marks r.1 m (P0 ++ path (succ (sv fr) (sv dst)) s.val k) ∧
        (∀ e ∈ path (succ (sv fr) (sv dst)) s.val k, e ∉ P0) ∧
        (path (succ (sv fr) (sv dst)) s.val k).Nodup ⦄ := by
  set σ := succ (sv fr) (sv dst) with hσ
  have hFl : (sv fr).length = m := by simp [sv, hm]
  unfold chain_loop2_loop0
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec Bool × alloc.vec.Vec U32 × Usize) => m + 1 - (vals q.2.1).length)
    (fun q => ∃ k, vals q.2.1 = P0 ++ path σ s.val k ∧ q.2.2.val = σ^[k] s.val ∧
      Marks q.1 m (P0 ++ path σ s.val k) ∧ (∀ e ∈ path σ s.val k, e ∉ P0) ∧
      (path σ s.val k).Nodup)
  · rintro ⟨u, pc, a⟩ ⟨k, hpc, ha, hmk, hout, hnd⟩
    unfold chain_loop2_loop0.body
    simp only at hpc ha hmk hout hnd ⊢
    have hal : a.val < m := by rw [ha]; exact iter_lt hp hs k
    have hul : a.val < u.val.length := by rw [hmk.1]; exact hal
    step with alloc.vec.Vec.index_usize_spec u a (by simpa using hul) as ⟨b, hb⟩
    have hbv : b = decide (a.val ∈ P0 ++ path σ s.val k) := by
      have := hmk.2 a.val hal
      rw [List.getElem?_eq_getElem hul] at this
      rw [hb]; simpa using this
    split
    · rename_i hb1
      -- met a marked piece: it is the start
      have hmet : σ^[k] s.val ∈ P0 ∨ σ^[k] s.val ∈ path σ s.val k := by
        rw [← ha]; rw [hb1] at hbv; simpa using hbv.symm
      have hk : 0 < k := by
        rcases Nat.eq_zero_or_pos k with rfl | h
        · exfalso; simp [path] at hmet; exact hs0 hmet
        · exact h
      have hcl := walk_closes hp hpre hs hk hout hnd hmet
      simp only [WP.spec_ok]
      exact ⟨k, hk, hpc, hcl, by rw [ha, hcl], hmk, hout, hnd⟩
    · rename_i hb1
      have hb0 : b = false := by simpa using hb1
      have hnot : a.val ∉ P0 ++ path σ s.val k := by rw [hb0] at hbv; simpa using hbv.symm
      have hnd1 : (P0 ++ path σ s.val (k + 1)).Nodup := by
        rw [path_succ, ← List.append_assoc, ← ha]
        apply List.Nodup.append
        · exact List.Nodup.append hnd0 hnd (fun x h1 h2 => hout x h2 h1)
        · simp
        · intro x h1 h2; simp at h2; subst h2; exact hnot h1
      have hlen1 : (P0 ++ path σ s.val (k + 1)).length ≤ m := by
        have := List.Nodup.length_le_of_subset (l₂ := List.range m) hnd1 (by
          intro x hx
          simp only [List.mem_append] at hx
          rcases hx with hx | hx
          · simpa using hlt0 x hx
          · obtain ⟨i, _, rfl⟩ := mem_path.mp hx; simpa using iter_lt hp hs i)
        simpa using this
      have hpcl : (vals pc).length = (P0 ++ path σ s.val k).length := by rw [hpc]
      step with alloc.vec.Vec.index_mut_usize_spec u a (by simpa using hul) as ⟨w, back, hw, hback⟩
      step as ⟨i, hi⟩
      have hiv : i.val = a.val := u32_of_lt' a m hal (by omega) i hi.symm
      step with alloc.vec.Vec.push_spec pc i (by
        have : m ≤ Usize.max := le_trans hmax u32max_le_usize
        rw [← vals_length, hpcl, List.length_append, path_length]
        rw [List.length_append, path_length] at hlen1; omega) as ⟨pc1, hpc1⟩
      have hadl : a.val < dst.val.length := by omega
      step with Slice.index_usize_spec dst a (by simpa using hadl) as ⟨i1, hi1⟩
      step as ⟨i2, hi2⟩
      have hTa : (sv dst)[a.val]'(by simp [sv]; omega) < nv := hT _ (List.getElem_mem _)
      have hi2v : i2.val = (sv dst)[a.val]'(by simp [sv]; omega) := by
        rw [hi2, U32.cast_Usize_val_eq, hi1]; simp [sv]
      have hfl : i2.val < fo.val.length := by rw [holds_len hfo, hi2v]; exact hTa
      step as ⟨i3, hi3⟩
      have hi3v : i3.val = σ a.val := by
        rw [hi3, holds_get hfo (by rw [hi2v]; exact hTa) hfl, hi2v]
        simp only [hσ, succ, hFl]
        rw [List.getD_eq_getElem _ _ (by simp [sv]; omega)]
      step as ⟨a1, ha1⟩
      have ha1v : a1.val = σ^[k + 1] s.val := by
        rw [ha1, U32.cast_Usize_val_eq, hi3v, Function.iterate_succ_apply', ← ha]
      refine ⟨⟨k + 1, ?_, ha1v, ?_, ?_, ?_⟩, ?_⟩
      · rw [vals_push _ _ _ hpc1, hpc, hiv, path_succ, ← ha, List.append_assoc]
      · refine ⟨by rw [hback]; simp [hmk.1], fun e he => ?_⟩
        rw [hback]
        simp only [alloc.vec.Vec.set_val_eq, List.getElem?_set]
        rw [path_succ, ← ha]
        by_cases hea : a.val = e
        · subst hea; simp [hmk.1, hal]
        · rw [if_neg hea, hmk.2 e he]
          simp only [List.mem_append, List.mem_singleton]
          rw [Option.some.injEq, decide_eq_decide]; constructor
          · rintro (h | h)
            · exact Or.inl h
            · exact Or.inr (Or.inl h)
          · rintro (h | h | h)
            · exact Or.inl h
            · exact Or.inr h
            · exact absurd h.symm hea
      · intro e he
        rw [path_succ, ← ha] at he
        simp only [List.mem_append, List.mem_singleton] at he
        rcases he with he | rfl
        · exact hout e he
        · intro h; exact hnot (List.mem_append_left _ h)
      · rw [path_succ, ← ha]
        apply List.Nodup.append hnd (by simp)
        intro x h1 h2; simp at h2; subst h2; exact hnot (List.mem_append_right _ h1)
      · rw [vals_push _ _ _ hpc1, List.length_append, hpcl, List.length_append, path_length]
        rw [List.length_append, path_length] at hlen1
        simp; omega
  · exact ⟨0, by simp [hP, path], by simp, by simpa [path] using hu, by simp [path], by simp [path]⟩

/-- The offsets at which consecutive lists start, from `a`. -/
def offsets : Nat → List (List Nat) → List Nat
  | _, [] => []
  | a, c :: cs => a :: offsets (a + c.length) cs

theorem offsets_snoc (a : Nat) (cs : List (List Nat)) (c : List Nat) :
    offsets a (cs ++ [c]) = offsets a cs ++ [a + cs.flatten.length] := by
  induction cs generalizing a with
  | nil => simp [offsets]
  | cons d cs ih =>
    simp only [List.cons_append, offsets, ih, List.flatten_cons, List.length_append, List.cons.injEq,
      List.append_cancel_left_eq, true_and, List.cons_append]
    simp [Nat.add_assoc]

theorem offsets_length (a : Nat) (cs : List (List Nat)) : (offsets a cs).length = cs.length := by
  induction cs generalizing a with
  | nil => rfl
  | cons c cs ih => simp [offsets, ih]

theorem length_le_flatten (cs : List (List Nat)) (h : ∀ c ∈ cs, 0 < c.length) :
    cs.length ≤ cs.flatten.length := by
  induction cs with
  | nil => simp
  | cons c cs ih =>
    simp only [List.length_cons, List.flatten_cons, List.length_append]
    have := h c (by simp)
    have := ih (fun d hd => h d (List.mem_cons_of_mem _ hd)); omega

theorem nodup_missing_length (l : List Nat) (m j : Nat) (hnd : l.Nodup) (hlt : ∀ e ∈ l, e < m)
    (hj : j < m) (hout : j ∉ l) : l.length < m := by
  have := List.Nodup.length_le_of_subset (l₂ := (List.range m).erase j) hnd (by
    intro x hx
    rw [List.mem_erase_of_ne (by intro e; subst e; exact hout hx)]
    simpa using hlt x hx)
  rw [List.length_erase_of_mem (by simpa using hj)] at this
  simp at this; omega

open ForgeSheetCore.Walk in
/-- The outer loop's state: the cycles found so far. -/
def WalkInv (σ : Nat → Nat) (m s : Nat) (bad : Option ChainError) (used : alloc.vec.Vec Bool)
    (pieces starts : alloc.vec.Vec U32) : Prop :=
  bad = none ∧ ∃ cs : List (List Nat), vals pieces = cs.flatten ∧ vals starts = offsets 0 cs ∧
    Marks used m cs.flatten ∧ cs.flatten.Nodup ∧ (∀ e ∈ cs.flatten, e < m) ∧
    (∀ e < m, (e ∈ cs.flatten ↔ σ e ∈ cs.flatten)) ∧ (∀ e < s, e ∈ cs.flatten) ∧
    (∀ c ∈ cs, IsCycle σ c)

open ForgeSheetCore.Walk in
theorem walks_spec (fr dst : Slice U32) (m : Usize) (hm : m.val = fr.val.length)
    (hdl : m.val ≤ dst.val.length) (hmax : m.val ≤ U32.max) (nv : Nat) (hT : ∀ x ∈ sv dst, x < nv)
    (fo : alloc.vec.Vec U32) (hfo : Holds fo nv (fOut (sv fr) m.val))
    (hp : Perm (succ (sv fr) (sv dst)) m.val)
    (bad : Option ChainError) (used : alloc.vec.Vec Bool) (pieces starts : alloc.vec.Vec U32)
    (start : Usize) (hs : start.val ≤ m.val)
    (hinv : WalkInv (succ (sv fr) (sv dst)) m.val start.val bad used pieces starts) :
    chain_loop2 fr dst m fo bad used pieces starts start ⦃ r =>
      ∃ u, WalkInv (succ (sv fr) (sv dst)) m.val m.val r.1 u r.2.1 r.2.2 ⦄ := by
  set σ := succ (sv fr) (sv dst) with hσ
  unfold chain_loop2
  apply loop.spec_decr_nat
    (fun (q : Option ChainError × alloc.vec.Vec Bool × alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize) =>
      m.val - q.2.2.2.2.val)
    (fun q => q.2.2.2.2.val ≤ m.val ∧ WalkInv σ m.val q.2.2.2.2.val q.1 q.2.1 q.2.2.1 q.2.2.2.1)
  · rintro ⟨bd, u, pc, st, j⟩ ⟨hj, hbd, cs, hpc, hst, hmk, hnd, hlt, hcl, hall, hcyc⟩
    unfold chain_loop2.body
    simp only at hj hbd hpc hst hmk hnd hlt hcl hall hcyc ⊢
    split
    · rename_i h
      have hjl : j.val < m.val := by scalar_tac
      split
      · rename_i hnone
        have hul : j.val < u.val.length := by rw [hmk.1]; exact hjl
        step with alloc.vec.Vec.index_usize_spec u j (by simpa using hul) as ⟨b, hb⟩
        have hbv : b = decide (j.val ∈ cs.flatten) := by
          have := hmk.2 j.val hjl
          rw [List.getElem?_eq_getElem hul] at this
          rw [hb]; simpa using this
        split
        · rename_i hb1
          step as ⟨j1, hj1⟩
          have hj1v : j1.val = j.val + 1 := by rw [hj1]; exact usize_inc j m hjl
          have hin : j.val ∈ cs.flatten := by rw [hb1] at hbv; simpa using hbv.symm
          refine ⟨by omega, ⟨hbd, cs, hpc, hst, hmk, hnd, hlt, hcl, ?_, hcyc⟩, by omega⟩
          intro e he; rw [hj1v] at he
          rcases Nat.lt_succ_iff_lt_or_eq.mp he with h | h
          · exact hall e h
          · rw [h]; exact hin
        · rename_i hb1
          have hb0 : b = false := by simpa using hb1
          have hout0 : j.val ∉ cs.flatten := by rw [hb0] at hbv; simpa using hbv.symm
          have hpl : (vals pc).length ≤ m.val := by
            rw [hpc]
            have := List.Nodup.length_le_of_subset (l₂ := List.range m.val) hnd (by
              intro x hx; simpa using hlt x hx)
            simpa using this
          have hfl := nodup_missing_length cs.flatten m.val j.val hnd hlt hjl hout0
          step as ⟨i1, hi1⟩
          have hi1v : i1.val = pc.val.length := by
            have := u32_of_lt' (alloc.vec.Vec.len pc) (m.val + 1)
              (by
                show pc.val.length < m.val + 1
                rw [← vals_length, hpc]; omega) (by omega) i1 hi1.symm
            simpa using this
          step with alloc.vec.Vec.push_spec st i1 (by
            have : m.val ≤ Usize.max := le_trans hmax u32max_le_usize
            have hsl : st.val.length = cs.length := by rw [← vals_length, hst, offsets_length]
            have hcsl := length_le_flatten cs (fun c hc => by
              obtain ⟨s0, k, hk, rfl, _⟩ := hcyc c hc; simpa [path_length] using hk)
            rw [hsl]; omega) as ⟨st1, hst1⟩
          step with walk_spec fr dst m.val hm hdl hmax nv hT fo hfo hp cs.flatten hnd hlt
            (fun e he h => (hcl e he).mpr h) j hjl hout0 u hmk pc hpc as ⟨u1, pc1, a, k, hk, hpc1, hcyck, ha, hmk1, hout1, hnd1⟩
          -- the walk closed at its start: no refusal
          have hane : (a != j) = false := by
            simp only [bne_eq_false_iff_eq]; exact UScalar.eq_of_val_eq ha
          simp only [hane, Bool.false_eq_true, ite_false]
          step as ⟨j1, hj1⟩
          have hj1v : j1.val = j.val + 1 := by rw [hj1]; exact usize_inc j m hjl
          have hflat : (cs ++ [path σ j.val k]).flatten = cs.flatten ++ path σ j.val k := by simp
          refine ⟨by omega, ⟨hbd, cs ++ [path σ j.val k], by rw [hflat]; exact hpc1, ?_, by rw [hflat]; exact hmk1,
            ?_, ?_, ?_, ?_, ?_⟩, by omega⟩
          · rw [vals_push _ _ _ hst1, hst, offsets_snoc, hi1v, ← vals_length, hpc]; simp
          · rw [hflat]
            exact List.Nodup.append hnd hnd1 (fun x h1 h2 => hout1 x h2 h1)
          · rw [hflat]; intro e he
            simp only [List.mem_append] at he
            rcases he with he | he
            · exact hlt e he
            · obtain ⟨i, _, rfl⟩ := mem_path.mp he; exact iter_lt hp hjl i
          · rw [hflat]; exact closed_add hp hcl hjl hk hcyck
          · rw [hflat, hj1v]; intro e he
            rcases Nat.lt_succ_iff_lt_or_eq.mp he with h | h
            · exact List.mem_append_left _ (hall e h)
            · rw [h]; exact List.mem_append_right _ (mem_path.mpr ⟨0, hk, rfl⟩)
          · intro c hc
            simp only [List.mem_append, List.mem_singleton] at hc
            rcases hc with hc | rfl
            · exact hcyc c hc
            · exact ⟨j.val, k, hk, rfl, hcyck⟩
      · rename_i hsome
        rw [hbd] at hsome; simp [core.option.Option.is_none] at hsome
    · rename_i h
      have : j.val = m.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [this] at hall
      exact ⟨u, hbd, cs, hpc, hst, hmk, hnd, hlt, hcl, hall, hcyc⟩
  · exact ⟨hs, hinv⟩

/-! ## `chain`, whole -/

theorem holds_replicate (nv : Usize) (x : U32) (v : alloc.vec.Vec U32)
    (hv : v.val = List.replicate nv.val x) (g : Nat → Nat) (hg : ∀ j, g j = x.val) :
    Holds v nv.val g := by
  unfold Holds vals; rw [hv]
  apply List.ext_getElem <;> simp [hg]

open ForgeSheetCore.Walk in
/-- **`chain` refines the chaining rule.** For kept pieces `e` running from `F[e]` to `T[e]`
(vertices below `nv`, at most `u32::MAX` of each, `to` at least as long as `from`), the shipped
`chain` returns without panic or overflow, and:
* `Ok` exactly when every vertex is left as often as reached, at most once; then the loops are
  the cycles of the successor permutation (`succ_perm`: each piece leaves where the one before
  it arrives), each started at its least piece, together every piece once, with `starts` their
  offsets and the total;
* `Err Unchained` at the first vertex whose degrees differ, naming its first leaving (else
  arriving) piece and both degrees;
* `Err Pinched` when the degrees agree everywhere: every vertex of degree past one, in order.
The walk's own refusal (a walk ending elsewhere than its start) never happens. -/
theorem chain_spec (nv : Usize) (hnv : nv.val ≤ U32.max) (fr dst : Slice U32)
    (hdl : fr.val.length ≤ dst.val.length) (hmax' : fr.val.length < U32.max)
    (hF : ∀ x ∈ sv fr, x < nv.val) (hT : ∀ x ∈ sv dst, x < nv.val) :
    chain nv fr dst ⦃ r =>
      let F := sv fr
      let m := F.length
      let NO := nOut F m
      let NI := nIn (sv dst) m
      match r with
      | .Ok l => (∀ v < nv.val, NO v = NI v ∧ NO v ≤ 1) ∧
          ∃ cs : List (List Nat), vals l.pieces = cs.flatten ∧
            vals l.starts = offsets 0 cs ++ [cs.flatten.length] ∧
            cs.flatten.Perm (List.range m) ∧ ∀ c ∈ cs, IsCycle (succ F (sv dst)) c
      | .Err b =>
          (∃ u < nv.val, (∀ u' < u, NO u' = NI u') ∧ NO u ≠ NI u ∧
            unchainedAt NO NI (fOut F m) (fIn (sv dst) m) u b) ∨
          ((∀ u < nv.val, NO u = NI u) ∧ ∃ ps, b = .Pinched ps ∧
            tvals ps = pinchList NO (fIn (sv dst) m) (sIn (sv dst) m) nv.val ∧ tvals ps ≠ []) ⦄ := by
  have hmax : fr.val.length ≤ U32.max := hmax'.le
  have hFl : (sv fr).length = fr.val.length := by simp [sv]
  unfold chain
  step with alloc.vec.from_elem_spec core.clone.CloneU32 0#u32 nv (by rfl) as ⟨z, hz, _⟩
  step with alloc.vec.from_elem_spec core.clone.CloneU32 core.num.U32.MAX nv (by rfl) as ⟨mx, hmx, _⟩
  have hmxv : (core.num.U32.MAX : U32).val = NONE := by
    simp [NONE, core.num.U32.MAX, U32.max_eq, U32.rMax, U32.numBits]
  have st0 : St0 (sv fr) (sv dst) nv.val 0 z z mx mx mx :=
    ⟨holds_replicate nv _ z hz _ (fun j => by simp [nOut, occ]),
     holds_replicate nv _ z hz _ (fun j => by simp [nIn, occ]),
     holds_replicate nv _ mx hmx _ (fun j => by simp [fOut, occ, hmxv]),
     holds_replicate nv _ mx hmx _ (fun j => by simp [fIn, occ, hmxv]),
     holds_replicate nv _ mx hmx _ (fun j => by simp [sIn, occ, hmxv])⟩
  step with loop0_chain_spec fr dst (Slice.len fr) (by simp) (by simpa using hdl) (by simpa using hmax)
    nv.val hF hT z z mx mx mx 0#usize (by simp) st0 as ⟨no, ni, fo, fi, si, hst0⟩
  obtain ⟨hno, hni, hfo, hfi, hsi⟩ := hst0
  have hFl2 : (Slice.len fr).val = (sv fr).length := by simp [sv]
  rw [hFl2] at hno hni hfo hfi hsi
  step with loop1_chain_spec nv hnv no ni fo fi si _ _ _ _ _ hno hni hfo hfi hsi
    (alloc.vec.Vec.new (U32 × U32 × U32)) (by simp [alloc.vec.Vec.new]) as ⟨bad, pin, hv⟩
  rcases hv with ⟨hb, heq, hpin⟩ | ⟨u, hu, hpre, hne, b, hb, hub⟩
  · subst hb
    simp only
    step as ⟨emp, hemp⟩
    split
    · rename_i hemp1
      have hpin0 : tvals pin = [] := by
        have : pin.val = [] := by
          have := hemp; rw [hemp1] at this; simpa [alloc.vec.Vec.length] using this.symm
        simp [tvals, this]
      have hdeg : ∀ v < nv.val, nOut (sv fr) (sv fr).length v = nIn (sv dst) (sv fr).length v ∧
          nOut (sv fr) (sv fr).length v ≤ 1 := by
        intro v hv
        refine ⟨heq v hv, ?_⟩
        by_contra hgt
        have : (v, fIn (sv dst) (sv fr).length v, sIn (sv dst) (sv fr).length v) ∈
            pinchList (nOut (sv fr) (sv fr).length) (fIn (sv dst) (sv fr).length)
              (sIn (sv dst) (sv fr).length) nv.val := by
          simp only [pinchList, List.mem_map, List.mem_filter, List.mem_range]
          exact ⟨v, ⟨hv, by simpa using (by omega : 1 < nOut (sv fr) (sv fr).length v)⟩, rfl⟩
        rw [← hpin, hpin0] at this; cases this
      have hTlen : (sv fr).length ≤ (sv dst).length := by simp [sv]; omega
      obtain ⟨hperm, hmeet⟩ := succ_perm (sv fr) (sv dst) nv.val hTlen hT hdeg
      step with alloc.vec.from_elem_spec core.clone.CloneBool false (Slice.len fr) (by rfl)
        as ⟨used, hused, _⟩
      have hinv : WalkInv (succ (sv fr) (sv dst)) (Slice.len fr).val 0 none used
          (alloc.vec.Vec.with_capacity U32 (Slice.len fr)) (alloc.vec.Vec.new U32) := by
        refine ⟨rfl, [], by simp [vals, alloc.vec.Vec.with_capacity], by simp [vals, alloc.vec.Vec.new, offsets],
          ⟨by rw [hused]; simp, fun e he => by rw [hused]; simp at he ⊢; simp [he]⟩, by simp,
          by simp, by simp, by simp, by simp⟩
      step with walks_spec fr dst (Slice.len fr) (by simp) (by simpa using hdl) (by simpa using hmax)
        nv.val hT fo (by simpa [← hFl] using hfo) (by simpa [hFl] using hperm) none used _ _ 0#usize
        (by simp) hinv as ⟨bad1, pc, st, u1, hw⟩
      obtain ⟨hbad1, cs, hpc, hst, hmk, hnd, hlt, hcl, hall, hcyc⟩ := hw
      subst hbad1
      simp only
      step as ⟨i1, hi1⟩
      have hfl : cs.flatten.length = (sv fr).length := by
        have hp : cs.flatten.Perm (List.range fr.val.length) := by
          apply (List.perm_ext_iff_of_nodup hnd List.nodup_range).mpr
          intro x; simp only [List.mem_range]
          exact ⟨fun h => by simpa using hlt x h, fun h => hall x (by simpa using h)⟩
        rw [hp.length_eq]; simp [sv]
      have hi1v : i1.val = cs.flatten.length := by
        have := u32_of_lt' (alloc.vec.Vec.len pc) (U32.max + 1)
          (by show pc.val.length < U32.max + 1
              rw [← vals_length, hpc, hfl, hFl]; omega) le_rfl i1 hi1.symm
        rw [this]; show pc.val.length = _; rw [← vals_length, hpc]
      have hcsl := length_le_flatten cs (fun c hc => by
        obtain ⟨s0, k, hk, rfl, _⟩ := hcyc c hc; simpa [path_length] using hk)
      step with alloc.vec.Vec.push_spec st i1 (by
        have : U32.max ≤ Usize.max := u32max_le_usize
        have hsl : st.val.length = cs.length := by rw [← vals_length, hst, offsets_length]
        rw [hsl]; rw [hfl, hFl] at hcsl; omega) as ⟨st1, hst1⟩
      refine ⟨hdeg, cs, hpc, ?_, ?_, ?_⟩
      · rw [vals_push _ _ _ hst1, hst, hi1v]
      · apply (List.perm_ext_iff_of_nodup hnd List.nodup_range).mpr
        intro x; simp only [List.mem_range]
        exact ⟨fun h => by have := hlt x h; simpa [hFl] using this,
          fun h => hall x (by simpa [hFl] using h)⟩
      · simpa [hFl] using hcyc
    · rename_i hemp1
      simp only [WP.spec_ok]
      have hne0 : tvals pin ≠ [] := by
        intro h0; apply hemp1
        have : pin.val = [] := by simpa [tvals] using h0
        rw [hemp]; simp [alloc.vec.Vec.length, this]
      exact Or.inr ⟨heq, pin, rfl, hpin, hne0⟩
  · subst hb
    simp only [WP.spec_ok]
    exact Or.inl ⟨u, hu, hpre, hne, hub⟩

end forge_sheet_core
