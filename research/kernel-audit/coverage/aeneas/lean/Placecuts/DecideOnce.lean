import Placecuts.Proofs
open Aeneas Aeneas.Std Result

/-!
# Decide once: with an exact `near`, `place_cuts` is the model's clustering

When every distance test reads one equivalence `r` (the points either coincide or lie far apart,
whatever the tolerances: the exact identification `Coverage/Model.lean` makes), the clustering
`placeRef` puts two cuts in one cluster exactly when `r` relates them, and the chain merge changes
nothing.
-/

namespace placecuts

section
variable (n : Nat) (r : Nat → Nat → Bool)

/-- `r` is an equivalence on the cuts `0 .. n-1`. -/
structure Equiv : Prop where
  refl : ∀ i < n, r i i = true
  symm : ∀ i < n, ∀ j < n, r i j = true → r j i = true
  trans : ∀ i < n, ∀ j < n, ∀ k < n, r i j = true → r j k = true → r i k = true

theorem get!_append_left (l m : List Nat) (t : Nat) (h : t < l.length) : (l ++ m)[t]! = l[t]! := by
  rw [getElem!_pos (l ++ m) t (by simp; omega), getElem!_pos l t h, List.getElem_append_left]

theorem get!_append_last (l : List Nat) (x : Nat) : (l ++ [x])[l.length]! = x := by
  rw [getElem!_pos (l ++ [x]) l.length (by simp)]; simp

/-- The greedy pass's state after cut `i`: every cut joins a head it is `r`-related to, the heads
are pairwise unrelated. -/
structure Good (cl hd : List Nat) (i : Nat) : Prop where
  len : cl.length = i
  heads : ∀ x ∈ hd, x < i
  apart : ∀ a < hd.length, ∀ b < hd.length, a ≠ b → r hd[a]! hd[b]! = false
  joins : ∀ t < i, cl[t]! < hd.length ∧ r t hd[cl[t]!]! = true

end

variable {n : Nat} {r : Nat → Nat → Bool} {T T2 : Nat → Nat → Nat → Bool}

/-- What the inner greedy loop finds: the first head from `k` on related to `i`, or none. -/
theorem gIn_found (hT : ∀ i j a, i < n → j < n → a < n → T i j a = r i j)
    (hd : List Nat) (hh : ∀ x ∈ hd, x < n) (i : Nat) (hi : i < n) (ch : Bool) (k : Nat) :
    ((gIn T T2 hd i ch k).2.1 = true →
        (gIn T T2 hd i ch k).2.2 < hd.length ∧ r hd[(gIn T T2 hd i ch k).2.2]! i = true) ∧
      ((gIn T T2 hd i ch k).2.1 = false → ∀ t, k ≤ t → t < hd.length → r hd[t]! i = false) := by
  fun_induction gIn T T2 hd i ch k with
  | case1 ch k hk hb =>
    have hm : hd[k]! < n := hh _ (by rw [getElem!_pos hd k hk]; apply List.getElem_mem)
    simp only [hT _ _ _ hm hi hm, hT _ _ _ hm hi hi, Bool.or_self] at hb
    exact ⟨fun _ => ⟨hk, hb⟩, fun h => by simp at h⟩
  | case2 ch k hk hb ih =>
    have hm : hd[k]! < n := hh _ (by rw [getElem!_pos hd k hk]; apply List.getElem_mem)
    simp only [hT _ _ _ hm hi hm, hT _ _ _ hm hi hi, Bool.or_self, Bool.not_eq_true] at hb
    refine ⟨ih.1, fun h t ht ht' => ?_⟩
    rcases Nat.eq_or_lt_of_le ht with e | e
    · subst e; exact hb
    · exact ih.2 h t e ht'
  | case3 ch k hk =>
    exact ⟨fun h => by simp at h, fun _ t ht ht' => by omega⟩

theorem gOut_good (hT : ∀ i j a, i < n → j < n → a < n → T i j a = r i j) (hr : Equiv n r)
    (cl hd : List Nat) (ch : Bool) (i : Nat) (hi : i ≤ n) (g : Good r cl hd i) :
    Good r (gOut T T2 n cl hd ch i).1 (gOut T T2 n cl hd ch i).2.1 n := by
  fun_induction gOut T T2 n cl hd ch i with
  | case1 cl hd ch i hlt hf ih =>
    have hh : ∀ x ∈ hd, x < n := fun x hx => by have := g.heads x hx; omega
    obtain ⟨hk, hrk⟩ := (gIn_found hT hd hh i hlt ch 0).1 hf
    apply ih (by omega)
    refine ⟨by simp [g.len], fun x hx => by have := g.heads x hx; omega, g.apart, ?_⟩
    intro t ht
    rcases Nat.lt_or_ge t i with h1 | h1
    · rw [get!_append_left _ _ t (by rw [g.len]; exact h1)]; exact g.joins t h1
    · obtain rfl : t = i := by omega
      have la := get!_append_last cl (gIn T T2 hd t ch 0).2.2
      rw [g.len] at la
      rw [la]
      have hm : hd[(gIn T T2 hd t ch 0).2.2]! < n :=
        hh _ (by rw [getElem!_pos hd _ hk]; apply List.getElem_mem)
      exact ⟨hk, hr.symm _ hm _ hlt hrk⟩
  | case2 cl hd ch i hlt hf ih =>
    have hh : ∀ x ∈ hd, x < n := fun x hx => by have := g.heads x hx; omega
    have none := (gIn_found hT hd hh i hlt ch 0).2 (by simpa using hf)
    apply ih (by omega)
    refine ⟨by simp [g.len], ?_, ?_, ?_⟩
    · intro x hx
      simp only [List.mem_append, List.mem_singleton] at hx
      rcases hx with hx | hx
      · have := g.heads x hx; omega
      · omega
    · intro a ha b hb hab
      simp only [List.length_append, List.length_singleton] at ha hb
      rcases Nat.lt_or_ge a hd.length with h1 | h1 <;> rcases Nat.lt_or_ge b hd.length with h2 | h2
      · rw [get!_append_left _ _ a h1, get!_append_left _ _ b h2]; exact g.apart a h1 b h2 hab
      · have e : b = hd.length := by omega
        subst e; rw [get!_append_left _ _ a h1, get!_append_last]; exact none a (by omega) h1
      · have e : a = hd.length := by omega
        subst e; rw [get!_append_left _ _ b h2, get!_append_last]
        have hm : hd[b]! < n := hh _ (by rw [getElem!_pos hd b h2]; apply List.getElem_mem)
        cases hx : r i hd[b]!
        · rfl
        · have := hr.symm _ hlt _ hm hx; rw [none b (by omega) h2] at this; exact absurd this (by simp)
      · omega
    · intro t ht
      rcases Nat.lt_or_ge t i with h1 | h1
      · rw [get!_append_left _ _ t (by rw [g.len]; exact h1)]
        obtain ⟨j1, j2⟩ := g.joins t h1
        refine ⟨by rw [List.length_append, List.length_singleton]; omega, ?_⟩
        rw [get!_append_left _ _ _ j1]; exact j2
      · obtain rfl : t = i := by omega
        have la := get!_append_last cl hd.length
        rw [g.len] at la
        rw [la, get!_append_last]
        exact ⟨by simp, hr.refl _ hlt⟩
  | case3 cl hd ch i hlt =>
    have : i = n := by omega
    subst this; exact g

/-- With heads pairwise unrelated, no cut is related to two heads: `shared` finds none. -/
theorem shRef_none (hT : ∀ i j a, i < n → j < n → a < n → T i j a = r i j) (hr : Equiv n r)
    (cl : List Nat) (h hk k o : Nat) (hh : h < n) (hhk : hk < n) (hap : r h hk = false)
    (any : Bool) (i : Nat) : shRef T n cl h hk k o any i = any := by
  fun_induction shRef T n cl h hk k o any i with
  | case1 any i hlt ih =>
    simp only [dite_eq_ite] at ih
    rw [ih, if_neg]
    rintro ⟨-, h1, h2⟩
    simp only [nr, hT _ _ _ hlt hh hh, hT _ _ _ hlt hh hhk, Bool.or_self,
      hT _ _ _ hlt hhk hh, hT _ _ _ hlt hhk hhk] at h1 h2
    have := hr.trans _ hh _ hlt _ hhk (hr.symm _ hlt _ hh h1) h2
    rw [hap] at this; exact absurd this (by simp)
  | case2 => rfl

theorem mIn_id (hT : ∀ i j a, i < n → j < n → a < n → T i j a = r i j) (hr : Equiv n r)
    (cl hd : List Nat) (hh : ∀ x ∈ hd, x < n)
    (hap : ∀ a < hd.length, ∀ b < hd.length, a ≠ b → r hd[a]! hd[b]! = false)
    (o : Nat) (ho : o < hd.length) (k : Nat) :
    mIn n T T2 cl hd o hd[o]! k = (cl, hd) := by
  have mem : ∀ a < hd.length, hd[a]! < n := fun a ha =>
    hh _ (by rw [getElem!_pos hd a ha]; apply List.getElem_mem)
  fun_induction mIn n T T2 cl hd o hd[o]! k with
  | case1 k hk _ hs =>
    rw [shRef_none hT hr cl _ _ k o (mem o ho) (mem k (by omega)) (hap o ho k (by omega) (by omega))] at hs
    simp at hs
  | case2 k hk _ hs ih => exact ih
  | case3 k hk _ ih => exact ih
  | case4 k hk => rfl

theorem mOut_id (hT : ∀ i j a, i < n → j < n → a < n → T i j a = r i j) (hr : Equiv n r)
    (cl hd : List Nat) (hh : ∀ x ∈ hd, x < n)
    (hap : ∀ a < hd.length, ∀ b < hd.length, a ≠ b → r hd[a]! hd[b]! = false)
    (o : Nat) (ho : 1 < o → o ≤ hd.length) :
    mOut n T T2 cl hd o = (cl, hd) := by
  fun_induction mOut n T T2 cl hd o with
  | case1 cl hd o hlt ih =>
    rw [mIn_id hT hr cl hd hh hap (o - 1) (by have := ho hlt; omega) 0] at ih ⊢
    simp only at ih ⊢
    exact ih hh hap (fun h => by have := ho hlt; omega)
  | case2 => rfl

/-- **Decide once.** If every distance test `place_cuts` makes reads one equivalence `r` on the
cuts, two cuts share a cluster exactly when `r` relates them. -/
theorem placeRef_exact (hT : ∀ i j a, i < n → j < n → a < n → T i j a = r i j) (hr : Equiv n r) :
    ∀ i < n, ∀ j < n, ((placeRef n T T2)[i]! = (placeRef n T T2)[j]! ↔ r i j = true) := by
  have g := gOut_good (T2 := T2) hT hr [] [] false 0 (by omega)
    ⟨rfl, by simp, by simp, by simp⟩
  have e : placeRef n T T2 = (gOut T T2 n [] [] false 0).1 := by
    unfold placeRef
    rw [mOut_id hT hr _ _ g.heads g.apart _ (by split_ifs <;> omega)]
  rw [e]
  generalize gOut T T2 n [] [] false 0 = G at g
  have hh : ∀ x ∈ G.2.1, x < n := g.heads
  have mem : ∀ a < G.2.1.length, G.2.1[a]! < n := fun a ha =>
    hh _ (by rw [getElem!_pos G.2.1 a ha]; apply List.getElem_mem)
  intro i hi j hj
  obtain ⟨ai, ri⟩ := g.joins i hi
  obtain ⟨aj, rj⟩ := g.joins j hj
  constructor
  · intro hc
    rw [hc] at ri
    exact hr.trans _ hi _ (mem _ aj) _ hj ri (hr.symm _ hj _ (mem _ aj) rj)
  · intro hij
    by_contra hne
    have h1 := hr.trans _ (mem _ ai) _ hi _ hj (hr.symm _ hi _ (mem _ ai) ri) hij
    have h2 := hr.trans _ (mem _ ai) _ hj _ (mem _ aj) h1 rj
    rw [g.apart _ ai _ aj hne] at h2; exact absurd h2 (by simp)

end placecuts

namespace placecuts

/-- **Decide once, on the extracted code.** `place` returns, and if every distance test reads one
equivalence `r` on the cuts, two cuts share a cluster exactly when `r` relates them. -/
theorem place_exact (n : Usize) (le1 le2 : Tbl) (s1 : Sized n le1) (s2 : Sized n le2)
    (r : Nat → Nat → Bool) (hr : Equiv n r)
    (hT : ∀ i j a, i < n.val → j < n.val → a < n.val → tb le1 i j a = r i j) :
    place n le1 le2 ⦃ c => c.val.length = n.val ∧
      ∀ i < n.val, ∀ j < n.val, (c.val[i]! = c.val[j]! ↔ r i j = true) ⦄ := by
  apply WP.spec_mono (place_spec n le1 le2 s1 s2)
  rintro c ⟨hc, hl⟩
  refine ⟨hl, fun i hi j hj => ?_⟩
  have := placeRef_exact (T2 := tb le2) hT hr i hi j hj
  rw [← hc, nv_get c i (by omega), nv_get c j (by omega)] at this
  rw [← this]
  constructor
  · intro h; rw [h]
  · intro h; exact UScalar.val_eq_imp _ _ h

end placecuts
