import Placecuts.Funs
open Aeneas Aeneas.Std Result

/-!
# The extracted `place_cuts` clustering refines a pure reference, and never fails

`Placecuts/Funs.lean` is Aeneas's translation of `aeneas/placecuts/src/lib.rs` (charon 9e193f0,
aeneas 2e3d4488). Each of its loops is proven, with `loop.spec_decr_nat`, to return what a pure
recursive reference returns: `gIn`/`gOut` (the greedy pass), `shRef`, `foldRef`, `rmRef`,
`mIn`/`mOut` (the chain merge), `placeRef` (the whole). `place_spec`: on tables with every entry
below `n` present (`Sized`), `place` returns ok (no index out of bounds, no overflow, no
divergence) a vector of length `n` equal to `placeRef`. No `sorry`, no `native_decide`.
-/

namespace placecuts

abbrev Tbl := alloc.vec.Vec (alloc.vec.Vec (alloc.vec.Vec Bool))

theorem idxu {α} [Inhabited α] (v : alloc.vec.Vec α) (i : Usize) (h : i.val < v.val.length) :
    alloc.vec.Vec.index_usize v i = ok v.val[i.val]! := by
  simp only [alloc.vec.Vec.index_usize]
  have : v[i.val]? = some v.val[i.val]! := by
    simp [getElem!_pos, h]
  rw [this]

theorem idx {α} [Inhabited α] (v : alloc.vec.Vec α) (i : Usize) (h : i.val < v.val.length) :
    alloc.vec.Vec.index (core.slice.index.SliceIndexUsizeSlice α) v i = ok v.val[i.val]! := by
  rw [alloc.vec.Vec.index_slice_index, idxu v i h]

theorem idxm {α} [Inhabited α] (v : alloc.vec.Vec α) (i : Usize) (h : i.val < v.val.length) :
    alloc.vec.Vec.index_mut (core.slice.index.SliceIndexUsizeSlice α) v i = ok (v.val[i.val]!, v.set i) := by
  rw [alloc.vec.Vec.index_mut_slice_index, alloc.vec.Vec.index_mut_usize, idxu v i h, bind_tc_ok]

/-- The table entry, as a function on naturals. -/
def tb (le : Tbl) (i j a : Nat) : Bool := le.val[i]!.val[j]!.val[a]!

def Sized (n : Nat) (le : Tbl) : Prop :=
  n ≤ le.val.length ∧ ∀ i < n, n ≤ (le.val[i]!).val.length ∧ ∀ j < n, n ≤ (le.val[i]!.val[j]!).val.length

theorem near_eq (n : Nat) (le : Tbl) (hs : Sized n le) (i j a b : Usize)
    (hi : i.val < n) (hj : j.val < n) (ha : a.val < n) (hb : b.val < n) :
    near le i j a b = ok (tb le i j a || tb le i j b) := by
  obtain ⟨h0, h1⟩ := hs
  obtain ⟨h2, h3⟩ := h1 i hi
  have h4 := h3 j hj
  unfold near
  rw [idx _ _ (by omega)]; simp only [Std.bind_ok]
  rw [idx _ _ (by omega)]; simp only [Std.bind_ok]
  rw [idx _ _ (by omega)]; simp only [Std.bind_ok]
  simp only [tb]
  split
  · rename_i h; rw [h]; rfl
  · rename_i h
    rw [idx _ _ (by omega)]
    simp only [Bool.not_eq_true] at h; rw [h]; rfl

theorem near2_eq (n : Nat) (le : Tbl) (hs : Sized n le) (i j a b : Usize)
    (hi : i.val < n) (hj : j.val < n) (ha : a.val < n) (hb : b.val < n) :
    near2 le i j a b = ok (tb le i j a || tb le i j b) :=
  near_eq n le hs i j a b hi hj ha hb

@[step]
theorem near_spec (n : Nat) (le : Tbl) (hs : Sized n le) (i j a b : Usize)
    (hi : i.val < n) (hj : j.val < n) (ha : a.val < n) (hb : b.val < n) :
    near le i j a b ⦃ r => r = (tb le i j a || tb le i j b) ⦄ := by
  rw [near_eq n le hs i j a b hi hj ha hb]; simp

@[step]
theorem near2_spec (n : Nat) (le : Tbl) (hs : Sized n le) (i j a b : Usize)
    (hi : i.val < n) (hj : j.val < n) (ha : a.val < n) (hb : b.val < n) :
    near2 le i j a b ⦃ r => r = (tb le i j a || tb le i j b) ⦄ := by
  rw [near2_eq n le hs i j a b hi hj ha hb]; simp

/-- A vector of indices as naturals. -/
def nv (v : alloc.vec.Vec Usize) : List Nat := v.val.map (·.val)

theorem nv_len (v : alloc.vec.Vec Usize) : (nv v).length = v.val.length := by simp [nv]

theorem mem_nv (v : alloc.vec.Vec Usize) (k : Nat) (h : k < v.val.length) :
    (v.val[k]).val ∈ nv v := by
  simp only [nv, List.mem_map]; exact ⟨_, List.getElem_mem h, rfl⟩

theorem nv_get' (v : alloc.vec.Vec Usize) (k : Nat) (h : k < (nv v).length) :
    (nv v)[k]! = (v.val[k]'(by simpa [nv] using h)).val := by
  rw [getElem!_pos (nv v) k h]; simp [nv]

theorem nv_get (v : alloc.vec.Vec Usize) (k : Nat) (h : k < v.val.length) :
    (nv v)[k]! = (v.val[k]!).val := by
  simp [nv, getElem!_pos, h]

/-! ## The greedy pass's inner loop -/

def gIn (T T2 : Nat → Nat → Nat → Bool) (heads : List Nat) (i : Nat) (ch : Bool) (k : Nat) :
    Bool × Bool × Nat :=
  if k < heads.length then
    if T heads[k]! i heads[k]! || T heads[k]! i i then (ch, true, k)
    else gIn T T2 heads i (ch || (T2 heads[k]! i heads[k]! || T2 heads[k]! i i)) (k + 1)
  else (ch, false, 0)
termination_by heads.length - k

theorem gIn_spec (n : Nat) (le1 le2 : Tbl) (s1 : Sized n le1) (s2 : Sized n le2)
    (heads : alloc.vec.Vec Usize) (hh : ∀ x ∈ nv heads, x < n) (i : Usize) (hi : i.val < n)
    (ch : Bool) (k : Usize) (hk : k.val ≤ heads.val.length) :
    greedy_loop0_loop0 le1 le2 heads ch i k ⦃ r =>
      let g := gIn (tb le1) (tb le2) (nv heads) i ch k
      r.1 = g.1 ∧ r.2.1 = g.2.1 ∧ r.2.2.val = g.2.2 ⦄ := by
  unfold greedy_loop0_loop0
  apply loop.spec_decr_nat (fun (x : Bool × Usize) => heads.val.length - x.2.val)
    (fun x => x.2.val ≤ heads.val.length ∧
      gIn (tb le1) (tb le2) (nv heads) i x.1 x.2 = gIn (tb le1) (tb le2) (nv heads) i ch k)
  · rintro ⟨c, k1⟩ ⟨hk1, heq⟩
    unfold greedy_loop0_loop0.body
    simp only at hk1 heq ⊢
    rw [← heq]
    by_cases hlt : k1.val < heads.val.length
    · have hlt' : k1 < heads.len := by scalar_tac
      have hl : k1.val < (nv heads).length := by simpa [nv] using hlt
      have hhn : (heads.val[k1.val]!).val < n := by
        rw [← nv_get _ _ hlt]; apply hh; rw [getElem!_pos (nv heads) k1.val hl]; apply List.getElem_mem
      simp only [hlt', ite_true]
      rw [idx heads k1 hlt]; simp only [Std.bind_ok]
      rw [near_eq n le1 s1 _ _ _ _ hhn hi hhn hi]; simp only [Std.bind_ok]
      split
      · rename_i hb
        simp only [WP.spec_ok]
        rw [gIn, if_pos hl, nv_get _ _ hlt, if_pos hb]; simp
      · rename_i hb
        rw [near2_eq n le2 s2 _ _ _ _ hhn hi hhn hi]; simp only [Std.bind_ok]
        generalize hc : (tb le2 (heads.val[k1.val]!).val i (heads.val[k1.val]!).val ||
          tb le2 (heads.val[k1.val]!).val i i) = c2
        cases c2
        all_goals
          simp only [Bool.false_eq_true, ite_false, ite_true, Std.bind_ok]
          step as ⟨k2, hk2⟩
          refine ⟨by omega, ?_, by omega⟩
          conv => rhs; rw [gIn]
          rw [if_pos hl, nv_get _ _ hlt, if_neg hb, hc, hk2]
          simp
    · have hlt' : ¬ (k1 < heads.len) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      have hl : ¬ k1.val < (nv heads).length := by simpa [nv] using hlt
      rw [gIn, if_neg hl]; simp
  · exact ⟨hk, rfl⟩

/-! ## `fold` and `remove_at` -/

def foldRef (n k o : Nat) (cl : List Nat) (x : Nat) : List Nat :=
  if x < n then
    foldRef n k o (if cl[x]! = o then cl.set x k else if cl[x]! > o then cl.set x (cl[x]! - 1) else cl) (x + 1)
  else cl
termination_by n - x

theorem nv_set (v : alloc.vec.Vec Usize) (x : Usize) (y : Usize) :
    nv (v.set x y) = (nv v).set x.val y.val := by
  simp [nv, alloc.vec.Vec.set_val_eq, List.map_set]

theorem fold_spec (n : Usize) (cl : alloc.vec.Vec Usize) (hl : n.val ≤ cl.val.length) (k o x : Usize)
    (hx : x.val ≤ n.val) :
    fold_loop n cl k o x ⦃ r => nv r = foldRef n k o (nv cl) x ∧ r.val.length = cl.val.length ⦄ := by
  unfold fold_loop
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Usize × Usize) => n.val - y.2.val)
    (fun y => y.2.val ≤ n.val ∧ y.1.val.length = cl.val.length ∧
      foldRef n k o (nv y.1) y.2 = foldRef n k o (nv cl) x)
  · rintro ⟨c, x1⟩ ⟨hx1, hc, heq⟩
    simp only at hx1 hc heq ⊢
    unfold fold_loop.body
    rw [← heq]
    by_cases hlt : x1.val < n.val
    · have hlt' : x1 < n := by scalar_tac
      have hcl : x1.val < c.val.length := by omega
      have hnl : x1.val < (nv c).length := by simpa [nv] using hcl
      simp only [hlt', ite_true]
      rw [idx c x1 hcl]; simp only [Std.bind_ok]
      conv => enter [2, r]; rw [foldRef.eq_1 (x := x1.val), if_pos hlt]
      rw [nv_get c x1.val hcl]
      generalize hv : (c.val[x1.val]!) = v
      by_cases hvo : v = o
      · have hvo' : v.val = o.val := by rw [hvo]
        simp only [hvo, ite_true]
        step*
        obtain ⟨xa, xb⟩ := x
        subst x_post1
        simp only [Std.uncurry, Std.bind_ok]
        step*
        simp only [nv_set]
        refine ⟨by omega, by simp [hc, alloc.vec.Vec.set_val_eq], ?_, by omega⟩
        rw [x1_post]
      · have hvo' : v.val ≠ o.val := fun h => hvo (by scalar_tac)
        simp only [hvo, ite_false, hvo']
        by_cases hgt : v > o
        · have hgt' : v.val > o.val := by scalar_tac
          simp only [hgt, ite_true, hgt']
          rw [idxm c x1 hcl, hv]
          simp only [Std.bind_ok, Std.uncurry]
          step as ⟨i2, hi2⟩
          step as ⟨x2, hx2⟩
          refine ⟨by omega, by simp [alloc.vec.Vec.set_val_eq, hc], ?_, by omega⟩
          rw [nv_set, hx2, hi2]
        · have hgt' : ¬ v.val > o.val := by scalar_tac
          simp only [hgt, ite_false, hgt']
          step*
    · have hlt' : ¬ (x1 < n) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      rw [foldRef, if_neg hlt]; exact ⟨rfl, hc⟩
  · exact ⟨hx, rfl, rfl⟩

def rmRef (hd : List Nat) (o : Nat) (out : List Nat) (j : Nat) : List Nat :=
  if j < hd.length then rmRef hd o (if j ≠ o then out ++ [hd[j]!] else out) (j + 1) else out
termination_by hd.length - j

theorem nv_push (v : alloc.vec.Vec Usize) (x : Usize) (w : alloc.vec.Vec Usize)
    (h : w.val = v.val ++ [x]) : nv w = nv v ++ [x.val] := by
  simp [nv, h]

theorem remove_at_spec (hd : alloc.vec.Vec Usize) (o : Usize) (out : alloc.vec.Vec Usize) (j : Usize)
    (hj : j.val ≤ hd.val.length) (ho : out.val.length ≤ j.val) :
    remove_at_loop hd o out j ⦃ r => nv r = rmRef (nv hd) o (nv out) j ∧ r.val.length ≤ hd.val.length ⦄ := by
  unfold remove_at_loop
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Usize × Usize) => hd.val.length - y.2.val)
    (fun y => y.2.val ≤ hd.val.length ∧ y.1.val.length ≤ y.2.val ∧
      rmRef (nv hd) o (nv y.1) y.2 = rmRef (nv hd) o (nv out) j)
  · rintro ⟨c, j1⟩ ⟨hj1, hc, heq⟩
    simp only at hj1 hc heq ⊢
    unfold remove_at_loop.body
    rw [← heq]
    have hl : (nv hd).length = hd.val.length := nv_len hd
    by_cases hlt : j1.val < hd.val.length
    · have hlt' : j1 < hd.len := by scalar_tac
      simp only [hlt', ite_true]
      conv => enter [2, r]; rw [rmRef.eq_1 (j := j1.val), if_pos (by omega)]
      by_cases hne : j1 = o
      · have hne' : ¬ (j1.val ≠ o.val) := by simp [hne]
        simp only [hne, bne_self_eq_false, Bool.false_eq_true, ite_false, Std.bind_ok]
        step as ⟨j2, hj2⟩
        refine ⟨by omega, by omega, ?_, by omega⟩
        rw [hj2]; simp [hne]
      · have hne' : j1.val ≠ o.val := fun h => hne (by scalar_tac)
        simp only [bne_iff_ne, ne_eq, hne, not_false_eq_true, ite_true]
        rw [idx hd j1 hlt]; simp only [Std.bind_ok]
        step as ⟨c2, hc2⟩
        step as ⟨j2, hj2⟩
        refine ⟨by omega, by simp [hc2]; omega, ?_, by omega⟩
        rw [hj2, nv_push _ _ _ hc2, nv_get hd j1.val hlt]; simp [hne']
    · have hlt' : ¬ (j1 < hd.len) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      rw [rmRef, if_neg (by omega)]; exact ⟨rfl, by omega⟩
  · exact ⟨hj, ho, rfl⟩

/-! ## `shared` -/

/-- `near` on a table function. -/
def nr (T : Nat → Nat → Nat → Bool) (i j a b : Nat) : Bool := T i j a || T i j b

def shRef (T : Nat → Nat → Nat → Bool) (n : Nat) (cl : List Nat) (h hk k o : Nat) (any : Bool) (i : Nat) : Bool :=
  if i < n then
    shRef T n cl h hk k o
      (if (cl[i]! = k ∨ cl[i]! = o) ∧ nr T i h h hk = true ∧ nr T i hk h hk = true then true else any) (i + 1)
  else any
termination_by n - i

theorem shared_spec (n : Usize) (le1 : Tbl) (s1 : Sized n le1) (cl : alloc.vec.Vec Usize)
    (hcl : n.val ≤ cl.val.length) (h hk k o : Usize) (hh : h.val < n.val) (hhk : hk.val < n.val)
    (any : Bool) (i : Usize) (hi : i.val ≤ n.val) :
    shared_loop n le1 cl h hk k o any i ⦃ r => r = shRef (tb le1) n (nv cl) h hk k o any i ⦄ := by
  unfold shared_loop
  apply loop.spec_decr_nat (fun (y : Bool × Usize) => n.val - y.2.val)
    (fun y => y.2.val ≤ n.val ∧ shRef (tb le1) n (nv cl) h hk k o y.1 y.2 = shRef (tb le1) n (nv cl) h hk k o any i)
  · rintro ⟨a, i1⟩ ⟨hi1, heq⟩
    simp only at hi1 heq ⊢
    unfold shared_loop.body
    rw [← heq]
    by_cases hlt : i1.val < n.val
    · have hlt' : i1 < n := by scalar_tac
      have hil : i1.val < cl.val.length := by omega
      simp only [hlt', ite_true]
      rw [idx cl i1 hil]; simp only [Std.bind_ok]
      conv => enter [2, r]; rw [shRef.eq_1 (i := i1.val), if_pos hlt, nv_get cl i1.val hil]
      have e1 := near_eq n le1 s1 i1 h h hk hlt hh hh hhk
      have e2 := near_eq n le1 s1 i1 hk h hk hlt hhk hh hhk
      generalize hv : cl.val[i1.val]! = v
      simp only [e1, e2, Std.bind_ok]
      split_ifs
      all_goals
        simp only [Std.bind_ok]
        step as ⟨i2, hi2⟩
        all_goals first
          | scalar_tac
          | (refine ⟨by omega, ?_, by omega⟩; rw [hi2]
             first | rfl | (exfalso; simp_all [nr, UScalar.eq_equiv]))
    · have hlt' : ¬ (i1 < n) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      rw [shRef, if_neg hlt]
  · exact ⟨hi, rfl⟩

/-! ## Facts about the references used by the merge -/

theorem foldRef_len (n k o : Nat) (cl : List Nat) (x : Nat) : (foldRef n k o cl x).length = cl.length := by
  fun_induction foldRef n k o cl x with
  | case1 cl x h ih => simp only [dite_eq_ite] at ih; rw [ih]; split_ifs <;> simp
  | case2 => rfl

theorem rmRef_mem (hd : List Nat) (o : Nat) (out : List Nat) (j : Nat) (x : Nat) :
    x ∈ rmRef hd o out j → x ∈ out ∨ x ∈ hd := by
  fun_induction rmRef hd o out j with
  | case1 out j h ih =>
    simp only [dite_eq_ite] at ih
    intro hx
    rcases ih hx with h1 | h1
    · split_ifs at h1
      · simp only [List.mem_append, List.mem_singleton] at h1
        rcases h1 with h2 | h2
        · exact Or.inl h2
        · right; rw [h2, getElem!_pos hd j h]; apply List.getElem_mem
      · exact Or.inl h1
    · exact Or.inr h1
  | case2 => intro hx; exact Or.inl hx

theorem rmRef_len (hd : List Nat) (o : Nat) (out : List Nat) (j : Nat) (ho : o < hd.length) (hj : j ≤ o) :
    (rmRef hd o out j).length = out.length + (hd.length - j) - 1 := by
  fun_induction rmRef hd o out j with
  | case1 out j h ih =>
    simp only [dite_eq_ite] at ih
    by_cases hjo : j = o
    · subst hjo
      simp only [ne_eq, not_true_eq_false, ite_false]
      rw [rmRef_len_after hd j out (j + 1) (by omega)]
      omega
    · rw [ih (by omega)]; simp [hjo]; omega
  | case2 out j h => omega
where
  rmRef_len_after (hd : List Nat) (o : Nat) (out : List Nat) (j : Nat) (hj : o < j) :
      (rmRef hd o out j).length = out.length + (hd.length - j) := by
    fun_induction rmRef hd o out j with
    | case1 out j h ih =>
      simp only [dite_eq_ite] at ih
      rw [ih (by omega)]; simp [show j ≠ o by omega]; omega
    | case2 out j h => omega

/-! ## The merge -/

def mIn (n : Nat) (T T2 : Nat → Nat → Nat → Bool) (cl hd : List Nat) (o h k : Nat) : List Nat × List Nat :=
  if k < o then
    if nr T2 h hd[k]! h hd[k]! then
      if shRef T n cl h hd[k]! k o false 0 then (foldRef n k o cl 0, rmRef hd o [] 0)
      else mIn n T T2 cl hd o h (k + 1)
    else mIn n T T2 cl hd o h (k + 1)
  else (cl, hd)
termination_by o - k

theorem mIn_spec (n : Usize) (le1 le2 : Tbl) (s1 : Sized n le1) (s2 : Sized n le2)
    (cl hd : alloc.vec.Vec Usize) (hcl : n.val ≤ cl.val.length) (hh : ∀ x ∈ nv hd, x < n.val)
    (o h k : Usize) (ho : o.val < hd.val.length) (hhn : h.val < n.val) (hk : k.val ≤ o.val) :
    merge_loop0_loop0 n le1 le2 cl hd o h k ⦃ r =>
      nv r.1 = (mIn n (tb le1) (tb le2) (nv cl) (nv hd) o h k).1 ∧
      nv r.2 = (mIn n (tb le1) (tb le2) (nv cl) (nv hd) o h k).2 ∧
      r.1.val.length = cl.val.length ⦄ := by
  unfold merge_loop0_loop0
  apply loop.spec_decr_nat (fun (k1 : Usize) => o.val - k1.val)
    (fun k1 => k1.val ≤ o.val ∧
      mIn n (tb le1) (tb le2) (nv cl) (nv hd) o h k1 = mIn n (tb le1) (tb le2) (nv cl) (nv hd) o h k)
  · rintro k1 ⟨hk1, heq⟩
    unfold merge_loop0_loop0.body
    rw [← heq]
    have hl : (nv hd).length = hd.val.length := nv_len hd
    by_cases hlt : k1.val < o.val
    · have hlt' : k1 < o := by scalar_tac
      have hkl : k1.val < hd.val.length := by omega
      have hkn : (hd.val[k1.val]!).val < n.val := by
        rw [← nv_get hd k1.val hkl]; apply hh; rw [getElem!_pos (nv hd) k1.val (by omega)]
        apply List.getElem_mem
      simp only [hlt', ite_true]
      rw [idx hd k1 hkl]; simp only [Std.bind_ok]
      rw [near2_eq n le2 s2 _ _ _ _ hhn hkn hhn hkn]; simp only [Std.bind_ok]
      conv => enter [2, r]; rw [mIn.eq_1 (k := k1.val), if_pos hlt, nv_get hd k1.val hkl]
      split
      · rename_i hb
        simp only [hb, ite_true, shared]
        apply WP.spec_bind (shared_spec n le1 s1 cl hcl h _ k1 o hhn hkn false 0#usize (by simp))
        intro b1 hb1
        have hz : (0#usize : Usize).val = 0 := by simp
        rw [hz] at hb1
        rw [hb1]
        split
        · rename_i hb2
          simp only [hb2, ite_true, fold]
          apply WP.spec_bind (fold_spec n cl hcl k1 o 0#usize (by simp))
          rintro c1 ⟨hc1, hc1l⟩
          simp only [remove_at]
          apply WP.spec_bind (remove_at_spec hd o (alloc.vec.Vec.new Usize) 0#usize (by simp) (by simp))
          rintro h1 ⟨hh1, -⟩
          have hnew : nv (alloc.vec.Vec.new Usize) = [] := by simp [nv]
          rw [hz] at hc1 hh1; rw [hnew] at hh1
          simp only [WP.spec_ok]
          refine ⟨?_, ?_, hc1l⟩
          · rw [hc1, if_pos (show nr _ _ _ _ _ = true from hb)]; try rw [if_pos hb2]
          · rw [hh1, if_pos (show nr _ _ _ _ _ = true from hb)]; try rw [if_pos hb2]
        · rename_i hb2
          try simp only [Bool.false_eq_true, ite_false]
          step as ⟨k2, hk2⟩
          refine ⟨by omega, ?_, by omega⟩
          rw [hk2]; try rw [if_pos (show nr _ _ _ _ _ = true from hb), if_neg hb2]
      · rename_i hb
        try simp only [Bool.false_eq_true, ite_false]
        step as ⟨k2, hk2⟩
        refine ⟨by omega, ?_, by omega⟩
        rw [hk2]; try rw [if_neg (show ¬ nr _ _ _ _ _ = true from hb)]
    · have hlt' : ¬ (k1 < o) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      rw [mIn, if_neg hlt]; simp
  · exact ⟨hk, rfl⟩

theorem mIn_cases (n : Nat) (T T2 : Nat → Nat → Nat → Bool) (cl hd : List Nat) (o h k : Nat) :
    (mIn n T T2 cl hd o h k).2 = hd ∨ (mIn n T T2 cl hd o h k).2 = rmRef hd o [] 0 := by
  fun_induction mIn n T T2 cl hd o h k <;> simp_all

def mOut (n : Nat) (T T2 : Nat → Nat → Nat → Bool) (cl hd : List Nat) (o : Nat) : List Nat × List Nat :=
  if 1 < o then
    mOut n T T2 (mIn n T T2 cl hd (o - 1) hd[o - 1]! 0).1 (mIn n T T2 cl hd (o - 1) hd[o - 1]! 0).2 (o - 1)
  else (cl, hd)
termination_by o

theorem mOut_spec (n : Usize) (le1 le2 : Tbl) (s1 : Sized n le1) (s2 : Sized n le2)
    (cl hd : alloc.vec.Vec Usize) (hcl : n.val ≤ cl.val.length) (hh : ∀ x ∈ nv hd, x < n.val)
    (o : Usize) (ho : 1 < o.val → o.val ≤ hd.val.length) :
    merge_loop0 n le1 le2 cl hd o ⦃ r =>
      nv r.1 = (mOut n (tb le1) (tb le2) (nv cl) (nv hd) o).1 ∧
      nv r.2 = (mOut n (tb le1) (tb le2) (nv cl) (nv hd) o).2 ⦄ := by
  unfold merge_loop0
  apply loop.spec_decr_nat (fun (y : alloc.vec.Vec Usize × alloc.vec.Vec Usize × Usize) => y.2.2.val)
    (fun y => n.val ≤ y.1.val.length ∧ (∀ x ∈ nv y.2.1, x < n.val) ∧
      (1 < y.2.2.val → y.2.2.val ≤ y.2.1.val.length) ∧
      mOut n (tb le1) (tb le2) (nv y.1) (nv y.2.1) y.2.2 = mOut n (tb le1) (tb le2) (nv cl) (nv hd) o)
  · rintro ⟨c, h1, o1⟩ ⟨hc, hh1, ho1, heq⟩
    simp only at hc hh1 ho1 heq ⊢
    unfold merge_loop0.body
    rw [← heq]
    by_cases hlt : 1 < o1.val
    · have hlt' : o1 > 1#usize := by scalar_tac
      simp only [hlt', ite_true]
      step as ⟨o2, ho2⟩
      have ho2l : o2.val < h1.val.length := by have := ho1 hlt; omega
      rw [idxu h1 o2 ho2l]; simp only [Std.bind_ok]
      have hhn : (h1.val[o2.val]!).val < n.val := by
        rw [← nv_get h1 o2.val ho2l]; apply hh1; rw [getElem!_pos (nv h1) o2.val (by simp [nv_len]; omega)]
        apply List.getElem_mem
      apply WP.spec_bind (mIn_spec n le1 le2 s1 s2 c h1 hc hh1 o2 _ 0#usize ho2l hhn (by simp))
      rintro ⟨c2, h2⟩ ⟨hc2, hh2, hl2⟩
      simp only [Std.uncurry, WP.spec_ok]
      have hz : (0#usize : Usize).val = 0 := by simp
      rw [hz] at hc2 hh2
      have hlen1 : (nv h1).length = h1.val.length := nv_len h1
      have hlen2 : (nv h2).length = h2.val.length := nv_len h2
      simp only at hl2
      refine ⟨by omega, ?_, ?_, ?_, by omega⟩
      · intro x hx
        rw [hh2] at hx
        rcases mIn_cases n.val (tb le1) (tb le2) (nv c) (nv h1) o2.val (h1.val[o2.val]!).val 0 with e | e
        · rw [e] at hx; exact hh1 x hx
        · rw [e] at hx
          rcases rmRef_mem _ _ _ _ x hx with h3 | h3
          · simp at h3
          · exact hh1 x h3
      · intro hlt2
        rw [← hlen2, hh2]
        rcases mIn_cases n.val (tb le1) (tb le2) (nv c) (nv h1) o2.val (h1.val[o2.val]!).val 0 with e | e
        · rw [e]; omega
        · rw [e, rmRef_len _ _ _ _ (by omega) (by omega)]; simp; omega
      · conv => rhs; rw [mOut, if_pos hlt]
        rw [hc2, hh2, ← ho2, nv_get h1 o2.val ho2l]
    · have hlt' : ¬ (o1 > 1#usize) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      rw [mOut, if_neg hlt]; simp
  · exact ⟨hcl, hh, ho, rfl⟩

theorem mIn_len (n : Nat) (T T2 : Nat → Nat → Nat → Bool) (cl hd : List Nat) (o h k : Nat) :
    (mIn n T T2 cl hd o h k).1.length = cl.length := by
  fun_induction mIn n T T2 cl hd o h k <;> simp_all [foldRef_len]

theorem mOut_len (n : Nat) (T T2 : Nat → Nat → Nat → Bool) (cl hd : List Nat) (o : Nat) :
    (mOut n T T2 cl hd o).1.length = cl.length := by
  fun_induction mOut n T T2 cl hd o <;> simp_all [mIn_len]

/-! ## The greedy pass and `place` -/

def gOut (T T2 : Nat → Nat → Nat → Bool) (n : Nat) (cl hd : List Nat) (ch : Bool) (i : Nat) :
    List Nat × List Nat × Bool :=
  if i < n then
    if (gIn T T2 hd i ch 0).2.1 then
      gOut T T2 n (cl ++ [(gIn T T2 hd i ch 0).2.2]) hd (gIn T T2 hd i ch 0).1 (i + 1)
    else gOut T T2 n (cl ++ [hd.length]) (hd ++ [i]) (gIn T T2 hd i ch 0).1 (i + 1)
  else (cl, hd, ch)
termination_by n - i

theorem gOut_spec (n : Usize) (le1 le2 : Tbl) (s1 : Sized n le1) (s2 : Sized n le2)
    (cl hd : alloc.vec.Vec Usize) (ch : Bool) (i : Usize) (hi : i.val ≤ n.val)
    (hcl : cl.val.length = i.val) (hhd : ∀ x ∈ nv hd, x < i.val) (hhl : hd.val.length ≤ i.val) :
    greedy_loop0 n le1 le2 cl hd ch i ⦃ r =>
      nv r.1 = (gOut (tb le1) (tb le2) n (nv cl) (nv hd) ch i).1 ∧
      nv r.2.1 = (gOut (tb le1) (tb le2) n (nv cl) (nv hd) ch i).2.1 ∧
      r.2.2 = (gOut (tb le1) (tb le2) n (nv cl) (nv hd) ch i).2.2 ∧
      r.1.val.length = n.val ∧ (∀ x ∈ nv r.2.1, x < n.val) ⦄ := by
  unfold greedy_loop0
  apply loop.spec_decr_nat
    (fun (y : alloc.vec.Vec Usize × alloc.vec.Vec Usize × Bool × Usize) => n.val - y.2.2.2.val)
    (fun y => y.2.2.2.val ≤ n.val ∧ y.1.val.length = y.2.2.2.val ∧ (∀ x ∈ nv y.2.1, x < y.2.2.2.val) ∧
      y.2.1.val.length ≤ y.2.2.2.val ∧ gOut (tb le1) (tb le2) n (nv y.1) (nv y.2.1) y.2.2.1 y.2.2.2 =
        gOut (tb le1) (tb le2) n (nv cl) (nv hd) ch i)
  · rintro ⟨c, h1, ch1, i1⟩ ⟨hi1, hc, hh1, hhl1, heq⟩
    simp only at hi1 hc hh1 hhl1 heq ⊢
    unfold greedy_loop0.body
    rw [← heq]
    by_cases hlt : i1.val < n.val
    · have hlt' : i1 < n := by scalar_tac
      simp only [hlt', ite_true]
      have hhn : ∀ x ∈ nv h1, x < n.val := fun x hx => by have := hh1 x hx; omega
      apply WP.spec_bind (gIn_spec n le1 le2 s1 s2 h1 hhn i1 hlt ch1 0#usize (by simp))
      rintro ⟨c1, f, a⟩ ⟨hc1, hf, ha⟩
      have hz : (0#usize : Usize).val = 0 := by simp
      simp only [hz] at hc1 hf ha
      simp only [Std.uncurry]
      have hmax : n.val ≤ Usize.max := by scalar_tac
      have hlen : (nv h1).length = h1.val.length := nv_len h1
      cases f
      · simp only [Bool.false_eq_true, ite_false]
        step as ⟨c2, hc2⟩
        step as ⟨h2, hh2⟩
        all_goals try (have := hhl1; scalar_tac)
        try simp only [Std.bind_ok, Std.uncurry_apply_pair]
        step as ⟨i2, hi2⟩
        refine ⟨by omega, by simp [hc2]; omega, ?_, by simp [hh2]; omega, ?_, by omega⟩
        · intro x hx
          rw [nv_push _ _ _ hh2] at hx
          simp only [List.mem_append, List.mem_singleton] at hx
          rcases hx with hx | hx
          · have := hh1 x hx; omega
          · omega
        · conv => rhs; rw [gOut, if_pos hlt, if_neg (by rw [← hf]; simp)]
          rw [nv_push _ _ _ hc2, nv_push _ _ _ hh2, hi2, ← hc1]
          simp [hlen]
      · simp only [ite_true]
        step as ⟨c2, hc2⟩
        try simp only [Std.bind_ok, Std.uncurry_apply_pair]
        step as ⟨i2, hi2⟩
        refine ⟨by omega, by simp [hc2]; omega, fun x hx => by have := hh1 x hx; omega, by omega, ?_, by omega⟩
        conv => rhs; rw [gOut, if_pos hlt, if_pos (by rw [← hf])]
        rw [nv_push _ _ _ hc2, hi2, ← hc1, ← ha]
    · have hlt' : ¬ (i1 < n) := by scalar_tac
      simp only [hlt', ite_false, WP.spec_ok]
      rw [gOut, if_neg hlt]
      refine ⟨rfl, rfl, rfl, by omega, fun x hx => by have := hh1 x hx; omega⟩
  · exact ⟨hi, hcl, hhd, hhl, rfl⟩

def placeRef (n : Nat) (T T2 : Nat → Nat → Nat → Bool) : List Nat :=
  (mOut n T T2 (gOut T T2 n [] [] false 0).1 (gOut T T2 n [] [] false 0).2.1
    (if (gOut T T2 n [] [] false 0).2.2 then (gOut T T2 n [] [] false 0).2.1.length else 1)).1

/-- **The extracted `place` refines `placeRef`**, and never fails: on tables with every entry
below `n` present, `place` returns (no out-of-bounds index, no overflow, no divergence) the
clustering `placeRef` computes from the table functions. -/
theorem place_spec (n : Usize) (le1 le2 : Tbl) (s1 : Sized n le1) (s2 : Sized n le2) :
    place n le1 le2 ⦃ r => nv r = placeRef n (tb le1) (tb le2) ∧ r.val.length = n.val ⦄ := by
  unfold place greedy
  have hz : (0#usize : Usize).val = 0 := by simp
  have hnew : nv (alloc.vec.Vec.new Usize) = [] := by simp [nv]
  apply WP.spec_bind (gOut_spec n le1 le2 s1 s2 (alloc.vec.Vec.new Usize) (alloc.vec.Vec.new Usize) false 0#usize
    (by simp) (by simp) (by simp [hnew]) (by simp))
  rintro ⟨c, h, ch⟩ ⟨hc, hh, hch, hcl, hhn⟩
  simp only at hc hh hch hcl hhn
  simp only [hz, hnew] at hc hh hch
  simp only [Std.uncurry, merge]
  have hlen : (nv h).length = h.val.length := nv_len h
  cases ch
  · simp only [Bool.false_eq_true, ite_false, Std.bind_ok]
    apply WP.spec_bind (mOut_spec n le1 le2 s1 s2 c h (by omega) hhn 1#usize (by simp))
    rintro ⟨c2, h2⟩ ⟨hc2, -⟩
    simp only [Std.uncurry_apply_pair, WP.spec_ok]
    have e : nv c2 = placeRef n (tb le1) (tb le2) := by
      rw [hc2, placeRef, ← hc, ← hh, ← hch]; simp
    refine ⟨e, ?_⟩
    rw [← nv_len, e, placeRef, mOut_len, ← hc, nv_len, hcl]
  · try simp only [ite_true, Std.bind_ok, bind_tc_ok, Std.bind_assoc]
    apply WP.spec_bind (mOut_spec n le1 le2 s1 s2 c h (by omega) hhn h.len (by simp))
    rintro ⟨c2, h2⟩ ⟨hc2, -⟩
    simp only [Std.uncurry_apply_pair, WP.spec_ok]
    have e : nv c2 = placeRef n (tb le1) (tb le2) := by
      rw [hc2, placeRef, ← hc, ← hh, ← hch]; simp [hlen]
    refine ⟨e, ?_⟩
    rw [← nv_len, e, placeRef, mOut_len, ← hc, nv_len, hcl]

end placecuts
