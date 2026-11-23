/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Mathlib

/-!
# forge-math `exact.rs`: the limb loops

A limb slice is a function `ℕ → ℕ` with a length; its value is `Σ_{i<n} w i · 2^(64 i)`.
Each Rust loop is modelled index by index, with u64 overflow made explicit (`ov`, `ovs`) and
u128 arithmetic checked to stay below `2^128`:

- `add_into`: `out = a + b`, length `max + 1`.
- `sub_into`: `out = a − b` when `a ≥ b`.
- `mul_into`: `out = a · b`, length `la + lb`.
- `shl_into`: `out = a · 2^bits`.
- `trimmed`, `cmp_mag`: drop zero top limbs; compare trimmed magnitudes.

Every output limb is below `2^64`.
-/

namespace IEEE754.Exact

open Finset

/-- The limb base `2^64`. -/
abbrev B : ℕ := 2 ^ 64

/-- The value of a limb slice. -/
def valF (w : ℕ → ℕ) (n : ℕ) : ℕ := ∑ i ∈ range n, w i * B ^ i

/-- Every limb below `2^64`. -/
def Limbs (w : ℕ → ℕ) (n : ℕ) : Prop := ∀ i < n, w i < B

lemma valF_succ (w : ℕ → ℕ) (n : ℕ) : valF w (n + 1) = valF w n + w n * B ^ n := by
  simp [valF, sum_range_succ]

lemma valF_lt {w : ℕ → ℕ} {n : ℕ} (h : Limbs w n) : valF w n < B ^ n := by
  induction n with
  | zero => simp [valF]
  | succ n ih =>
    rw [valF_succ, pow_succ]
    have h1 := ih (fun i hi => h i (by omega))
    have h2 : w n ≤ B - 1 := by have := h n (by omega); omega
    have hB : 0 < B ^ n := by positivity
    calc valF w n + w n * B ^ n < B ^ n + (B - 1) * B ^ n := by
          have := Nat.mul_le_mul_right (B ^ n) h2; omega
      _ = B ^ n * B := by
          have : (B - 1) * B ^ n + B ^ n = B * B ^ n := by
            rw [Nat.sub_mul, one_mul]; have : B ^ n ≤ B * B ^ n := Nat.le_mul_of_pos_left _ (by norm_num); omega
          rw [Nat.mul_comm (B ^ n) B]; omega

lemma valF_congr {w w' : ℕ → ℕ} {n : ℕ} (h : ∀ i < n, w i = w' i) : valF w n = valF w' n :=
  sum_congr rfl (fun i hi => by rw [h i (mem_range.mp hi)])

lemma valF_update_ge {w : ℕ → ℕ} {n k v : ℕ} (hk : n ≤ k) : valF (Function.update w k v) n = valF w n :=
  valF_congr (fun i hi => by rw [Function.update_of_ne (by omega)])

/-- u64 `overflowing_add` on limbs. -/
def ov (x y : ℕ) : ℕ × ℕ := ((x + y) % B, if B ≤ x + y then 1 else 0)
/-- u64 `overflowing_sub` on limbs. -/
def ovs (x y : ℕ) : ℕ × ℕ := ((x + B - y) % B, if x < y then 1 else 0)

lemma ov_spec {x y : ℕ} (hx : x < B) (hy : y < B) : (ov x y).1 + B * (ov x y).2 = x + y ∧ (ov x y).1 < B ∧ (ov x y).2 ≤ 1 := by
  unfold ov; split_ifs with h
  · refine ⟨?_, Nat.mod_lt _ (by norm_num), le_rfl⟩
    have : (x + y) % B = x + y - B := by
      rw [Nat.mod_eq_sub_mod h, Nat.mod_eq_of_lt (by omega)]
    omega
  · push Not at h; refine ⟨?_, Nat.mod_lt _ (by norm_num), by norm_num⟩
    rw [Nat.mod_eq_of_lt h]; ring

lemma ovs_spec {x y : ℕ} (hx : x < B) (hy : y < B) : x + B * (ovs x y).2 = (ovs x y).1 + y ∧ (ovs x y).1 < B ∧ (ovs x y).2 ≤ 1 := by
  unfold ovs; split_ifs with h
  · refine ⟨?_, Nat.mod_lt _ (by norm_num), le_rfl⟩
    rw [Nat.mod_eq_of_lt (by omega)]; omega
  · push Not at h; refine ⟨?_, Nat.mod_lt _ (by norm_num), by norm_num⟩
    have : (x + B - y) % B = x - y := by
      rw [show x + B - y = (x - y) + B by omega, Nat.add_mod_right, Nat.mod_eq_of_lt (by omega)]
    rw [this]; omega

/-! ## `add_into` -/

/-- The first `k` steps of `add_into`'s loop (`a` the longer, `b` read as 0 past `lb`): the output so
far and the carry. -/
def addRun (a b : ℕ → ℕ) (lb : ℕ) : ℕ → (ℕ → ℕ) × ℕ
  | 0 => (fun _ => 0, 0)
  | k + 1 =>
    let r := addRun a b lb k
    let y := if k < lb then b k else 0
    let s := ov (a k) y
    let t := ov s.1 r.2
    (Function.update r.1 k t.1, s.2 + t.2)

/-- `add_into(a, b, out)` with `a` the longer (the Rust swaps first): `out[la] = carry`. -/
def addInto (a : ℕ → ℕ) (la : ℕ) (b : ℕ → ℕ) (lb : ℕ) : ℕ → ℕ :=
  Function.update (addRun a b lb la).1 la (addRun a b lb la).2

lemma B_val : B = 18446744073709551616 := by norm_num

/-- Two overflowing adds in a row carry at most one. -/
lemma ov2 {x y c : ℕ} (hx : x < B) (hy : y < B) (hc : c ≤ 1) :
    (ov (ov x y).1 c).1 < B ∧ (ov x y).2 + (ov (ov x y).1 c).2 ≤ 1 ∧
      (ov (ov x y).1 c).1 + B * ((ov x y).2 + (ov (ov x y).1 c).2) = x + y + c := by
  obtain ⟨s1, s2, s3⟩ := ov_spec hx hy
  obtain ⟨t1, t2, t3⟩ := ov_spec s2 (show c < B by rw [B_val]; omega)
  refine ⟨t2, ?_, by rw [Nat.mul_add]; omega⟩
  rw [B_val] at *
  omega

lemma addRun_spec {a b : ℕ → ℕ} {la lb : ℕ} (ha : Limbs a la) (hb : Limbs b lb) :
    ∀ k ≤ la, valF (addRun a b lb k).1 k + B ^ k * (addRun a b lb k).2 =
        valF a k + valF (fun i => if i < lb then b i else 0) k ∧
      Limbs (addRun a b lb k).1 k ∧ (∀ i ≥ k, (addRun a b lb k).1 i = 0) ∧ (addRun a b lb k).2 ≤ 1 := by
  intro k hk
  induction k with
  | zero => simp [addRun, valF, Limbs]
  | succ k ih =>
    obtain ⟨e, lim, z, c1⟩ := ih (by omega)
    have hy : (if k < lb then b k else 0) < B := by
      split_ifs with h
      · exact hb k h
      · norm_num
    obtain ⟨t2, tc, teq⟩ := ov2 (ha k (by omega)) hy c1
    simp only [addRun]
    generalize hr : addRun a b lb k = r at e lim z c1 t2 tc teq
    generalize hy' : (if k < lb then b k else 0) = y at hy t2 tc teq
    generalize hs : ov (a k) y = s at t2 tc teq
    generalize ht : ov s.1 r.2 = t at t2 tc teq
    refine ⟨?_, ?_, ?_, tc⟩
    · rw [valF_succ, valF_succ, valF_succ, Function.update_self, valF_update_ge le_rfl, pow_succ]
      simp only [hy']
      have key : t.1 * B ^ k + B ^ k * B * (s.2 + t.2) = (a k + y + r.2) * B ^ k := by
        calc t.1 * B ^ k + B ^ k * B * (s.2 + t.2) = B ^ k * (t.1 + B * (s.2 + t.2)) := by ring
          _ = (a k + y + r.2) * B ^ k := by rw [teq]; ring
      nlinarith [e, key]
    · intro i hi
      rcases Nat.lt_succ_iff_lt_or_eq.mp hi with h | h
      · rw [Function.update_of_ne (by omega)]; exact lim i h
      · subst h; rw [Function.update_self]; exact t2
    · intro i hi; rw [Function.update_of_ne (by omega)]; exact z i (by omega)

/-- **`add_into`.** With `lb ≤ la`: `out` (length `la + 1`) is `a + b`, every limb below `2^64`. -/
theorem add_into_spec {a b : ℕ → ℕ} {la lb : ℕ} (ha : Limbs a la) (hb : Limbs b lb) (hab : lb ≤ la) :
    valF (addInto a la b lb) (la + 1) = valF a la + valF b lb ∧ Limbs (addInto a la b lb) (la + 1) ∧
      ∀ i > la, addInto a la b lb i = 0 := by
  obtain ⟨e, lim, z, c1⟩ := addRun_spec ha hb la le_rfl
  have hb' : valF (fun i => if i < lb then b i else 0) la = valF b lb := by
    unfold valF
    rw [← sum_range_add_sum_Ico _ hab]
    have : ∑ i ∈ Ico lb la, (if i < lb then b i else 0) * B ^ i = 0 :=
      sum_eq_zero (fun i hi => by simp only [if_neg (show ¬ i < lb by simp at hi; omega)]; simp)
    rw [this, add_zero]
    exact sum_congr rfl (fun i hi => by simp only [if_pos (mem_range.mp hi)])
  unfold addInto
  refine ⟨?_, ?_, ?_⟩
  · rw [valF_succ, Function.update_self, valF_update_ge le_rfl, ← hb', ← e]; ring
  · intro i hi
    rcases Nat.lt_succ_iff_lt_or_eq.mp hi with h | h
    · rw [Function.update_of_ne (by omega)]; exact lim i h
    · subst h; rw [Function.update_self, B_val]; omega
  · intro i hi; rw [Function.update_of_ne (by omega)]; exact z i (by omega)

/-! ## `sub_into` -/

def subRun (a b : ℕ → ℕ) (lb : ℕ) : ℕ → (ℕ → ℕ) × ℕ
  | 0 => (fun _ => 0, 0)
  | k + 1 =>
    let r := subRun a b lb k
    let y := if k < lb then b k else 0
    let s := ovs (a k) y
    let t := ovs s.1 r.2
    (Function.update r.1 k t.1, s.2 + t.2)

/-- `sub_into(a, b, out)`: `out`, length `la`. -/
def subInto (a : ℕ → ℕ) (la : ℕ) (b : ℕ → ℕ) (lb : ℕ) : ℕ → ℕ := (subRun a b lb la).1

lemma ovs2 {x y c : ℕ} (hx : x < B) (hy : y < B) (hc : c ≤ 1) :
    (ovs (ovs x y).1 c).1 < B ∧ (ovs x y).2 + (ovs (ovs x y).1 c).2 ≤ 1 ∧
      x + B * ((ovs x y).2 + (ovs (ovs x y).1 c).2) = (ovs (ovs x y).1 c).1 + y + c := by
  obtain ⟨s1, s2, s3⟩ := ovs_spec hx hy
  obtain ⟨t1, t2, t3⟩ := ovs_spec s2 (show c < B by rw [B_val]; omega)
  refine ⟨t2, ?_, by rw [Nat.mul_add]; omega⟩
  rw [B_val] at *
  omega

lemma subRun_spec {a b : ℕ → ℕ} {la lb : ℕ} (ha : Limbs a la) (hb : Limbs b lb) :
    ∀ k ≤ la, valF a k + B ^ k * (subRun a b lb k).2 =
        valF (subRun a b lb k).1 k + valF (fun i => if i < lb then b i else 0) k ∧
      Limbs (subRun a b lb k).1 k ∧ (subRun a b lb k).2 ≤ 1 := by
  intro k hk
  induction k with
  | zero => simp [subRun, valF, Limbs]
  | succ k ih =>
    obtain ⟨e, lim, c1⟩ := ih (by omega)
    have hy : (if k < lb then b k else 0) < B := by
      split_ifs with h
      · exact hb k h
      · norm_num
    obtain ⟨t2, tc, teq⟩ := ovs2 (ha k (by omega)) hy c1
    simp only [subRun]
    generalize hr : subRun a b lb k = r at e lim c1 t2 tc teq
    generalize hy' : (if k < lb then b k else 0) = y at hy t2 tc teq
    generalize hs : ovs (a k) y = s at t2 tc teq
    generalize ht : ovs s.1 r.2 = t at t2 tc teq
    refine ⟨?_, ?_, tc⟩
    · rw [valF_succ, valF_succ, valF_succ, Function.update_self, valF_update_ge le_rfl, pow_succ]
      simp only [hy']
      have key : a k * B ^ k + B ^ k * B * (s.2 + t.2) = (t.1 + y + r.2) * B ^ k := by
        calc a k * B ^ k + B ^ k * B * (s.2 + t.2) = B ^ k * (a k + B * (s.2 + t.2)) := by ring
          _ = (t.1 + y + r.2) * B ^ k := by rw [teq]; ring
      nlinarith [e, key]
    · intro i hi
      rcases Nat.lt_succ_iff_lt_or_eq.mp hi with h | h
      · rw [Function.update_of_ne (by omega)]; exact lim i h
      · subst h; rw [Function.update_self]; exact t2

lemma valF_pad {b : ℕ → ℕ} {la lb : ℕ} (hab : lb ≤ la) :
    valF (fun i => if i < lb then b i else 0) la = valF b lb := by
  unfold valF
  rw [← sum_range_add_sum_Ico _ hab]
  have : ∑ i ∈ Ico lb la, (if i < lb then b i else 0) * B ^ i = 0 :=
    sum_eq_zero (fun i hi => by simp only [if_neg (show ¬ i < lb by simp at hi; omega)]; simp)
  rw [this, add_zero]
  exact sum_congr rfl (fun i hi => by simp only [if_pos (mem_range.mp hi)])

/-- **`sub_into`.** With `lb ≤ la` and `b ≤ a`: `out` (length `la`) is `a − b`. -/
theorem sub_into_spec {a b : ℕ → ℕ} {la lb : ℕ} (ha : Limbs a la) (hb : Limbs b lb) (hab : lb ≤ la)
    (hle : valF b lb ≤ valF a la) :
    valF (subInto a la b lb) la = valF a la - valF b lb ∧ Limbs (subInto a la b lb) la := by
  obtain ⟨e, lim, c1⟩ := subRun_spec ha hb la le_rfl
  rw [valF_pad hab] at e
  have hlt := valF_lt lim
  unfold subInto
  refine ⟨?_, lim⟩
  rcases Nat.le_one_iff_eq_zero_or_eq_one.mp c1 with h | h
  · rw [h] at e; omega
  · rw [h] at e; omega

/-! ## `mul_into` -/

/-- Row `i` of `mul_into`, its first `j` inner steps: the output and the carry. -/
def mulRow (a b : ℕ → ℕ) (i : ℕ) (o : ℕ → ℕ) : ℕ → (ℕ → ℕ) × ℕ
  | 0 => (o, 0)
  | j + 1 =>
    let r := mulRow a b i o j
    let t := r.1 (i + j) + a i * b j + r.2
    (Function.update r.1 (i + j) (t % B), t / B)

/-- The first `i` rows of `mul_into` (each row ends `out[i + lb] = carry`). -/
def mulRun (a b : ℕ → ℕ) (lb : ℕ) : ℕ → ℕ → ℕ
  | 0 => fun _ => 0
  | i + 1 =>
    let r := mulRow a b i (mulRun a b lb i) lb
    Function.update r.1 (i + lb) r.2

/-- `mul_into(a, b, out)`, length `la + lb`. -/
def mulInto (a : ℕ → ℕ) (la : ℕ) (b : ℕ → ℕ) (lb : ℕ) : ℕ → ℕ := mulRun a b lb la

lemma valF_update_lt {w : ℕ → ℕ} {N m v : ℕ} (hm : m < N) :
    valF (Function.update w m v) N + w m * B ^ m = valF w N + v * B ^ m := by
  unfold valF
  rw [← sum_erase_add _ _ (mem_range.mpr hm), ← sum_erase_add _ _ (mem_range.mpr hm), Function.update_self]
  have : ∑ x ∈ (range N).erase m, Function.update w m v x * B ^ x = ∑ x ∈ (range N).erase m, w x * B ^ x :=
    sum_congr rfl (fun x hx => by rw [Function.update_of_ne (ne_of_mem_erase hx)])
  rw [this]; ring

lemma mulRow_spec {a b o : ℕ → ℕ} {i lb N : ℕ} (hai : a i < B) (hb : Limbs b lb) (ho : Limbs o N)
    (hN : i + lb ≤ N) :
    ∀ j ≤ lb, (∀ m, (m < i ∨ i + j ≤ m) → (mulRow a b i o j).1 m = o m) ∧
      valF (mulRow a b i o j).1 N + B ^ (i + j) * (mulRow a b i o j).2 = valF o N + a i * B ^ i * valF b j ∧
      Limbs (mulRow a b i o j).1 N ∧ (mulRow a b i o j).2 < B ∧
      (∀ j' < j, (mulRow a b i o j').1 (i + j') + a i * b j' + (mulRow a b i o j').2 < B * B) := by
  intro j hj
  induction j with
  | zero => simp [mulRow, valF]; exact ho
  | succ j ih =>
    obtain ⟨keep, e, lim, cB, tB⟩ := ih (by omega)
    simp only [mulRow]
    generalize hr : mulRow a b i o j = r at keep e lim cB tB
    have hoij : r.1 (i + j) < B := lim (i + j) (by omega)
    have hbj : b j < B := hb j (by omega)
    have ht : r.1 (i + j) + a i * b j + r.2 < B * B := by
      have h1 : a i * b j ≤ (B - 1) * (B - 1) :=
        Nat.mul_le_mul (by omega) (by omega)
      have : (B - 1) * (B - 1) + (B - 1) + (B - 1) < B * B := by rw [B_val]; norm_num
      omega
    have hdm := Nat.div_add_mod (r.1 (i + j) + a i * b j + r.2) B
    refine ⟨?_, ?_, ?_, ?_, ?_⟩
    · intro m hm
      rw [Function.update_of_ne (by omega)]; exact keep m (by omega)
    · have u := valF_update_lt (w := r.1) (v := (r.1 (i + j) + a i * b j + r.2) % B) (show i + j < N by omega)
      rw [valF_succ, show i + (j + 1) = (i + j) + 1 by ring, pow_succ]
      zify at u e hdm ⊢
      linear_combination u + e + (B : ℤ) ^ (i + j) * hdm
    · intro m hm
      by_cases h : m = i + j
      · subst h; rw [Function.update_self]; exact Nat.mod_lt _ (by norm_num)
      · rw [Function.update_of_ne h]; exact lim m hm
    · exact Nat.div_lt_of_lt_mul ht
    · intro j' hj'
      rcases Nat.lt_succ_iff_lt_or_eq.mp hj' with h | h
      · exact tB j' h
      · subst h; rw [hr]; exact ht

lemma mulRun_spec {a b : ℕ → ℕ} {la lb : ℕ} (ha : Limbs a la) (hb : Limbs b lb) :
    ∀ i ≤ la, valF (mulRun a b lb i) (la + lb) = valF a i * valF b lb ∧
      Limbs (mulRun a b lb i) (la + lb) ∧ (∀ m ≥ i + lb, mulRun a b lb i m = 0) ∧
      (∀ i' < i, ∀ j' < lb, (mulRow a b i' (mulRun a b lb i') j').1 (i' + j') + a i' * b j' +
        (mulRow a b i' (mulRun a b lb i') j').2 < B * B) := by
  intro i hi
  induction i with
  | zero => simp [mulRun, valF, Limbs]
  | succ i ih =>
    obtain ⟨e, lim, z, tB⟩ := ih (by omega)
    obtain ⟨keep, e2, lim2, cB, tB2⟩ := mulRow_spec (o := mulRun a b lb i) (ha i (by omega)) hb lim
      (show i + lb ≤ la + lb by omega) lb le_rfl
    have z0 : (mulRow a b i (mulRun a b lb i) lb).1 (i + lb) = 0 := by
      rw [keep (i + lb) (Or.inr le_rfl)]; exact z _ le_rfl
    simp only [mulRun]
    refine ⟨?_, ?_, ?_, ?_⟩
    · have u := valF_update_lt (w := (mulRow a b i (mulRun a b lb i) lb).1)
        (v := (mulRow a b i (mulRun a b lb i) lb).2) (show i + lb < la + lb by omega)
      rw [z0, zero_mul, add_zero] at u
      rw [u, valF_succ]
      rw [e] at e2
      zify at e2 ⊢
      linear_combination e2
    · intro m hm
      by_cases h : m = i + lb
      · subst h; rw [Function.update_self]; exact cB
      · rw [Function.update_of_ne h]; exact lim2 m hm
    · intro m hm
      rw [Function.update_of_ne (by omega), keep m (Or.inr (by omega))]; exact z m (by omega)
    · intro i' hi' j' hj'
      rcases Nat.lt_succ_iff_lt_or_eq.mp hi' with h | h
      · exact tB i' h j' hj'
      · subst h; exact tB2 j' hj'

/-- **`mul_into`.** `out` (length `la + lb`) is `a · b`, every limb below `2^64`, and every u128
intermediate `out[i+j] + a[i]·b[j] + carry` is below `2^128` (so the Rust's u128 is exact). -/
theorem mul_into_spec {a b : ℕ → ℕ} {la lb : ℕ} (ha : Limbs a la) (hb : Limbs b lb) :
    valF (mulInto a la b lb) (la + lb) = valF a la * valF b lb ∧ Limbs (mulInto a la b lb) (la + lb) ∧
      (∀ i < la, ∀ j < lb, (mulRow a b i (mulRun a b lb i) j).1 (i + j) + a i * b j +
        (mulRow a b i (mulRun a b lb i) j).2 < B * B) := by
  obtain ⟨e, lim, -, tB⟩ := mulRun_spec ha hb la le_rfl
  exact ⟨e, lim, tB⟩

end IEEE754.Exact
