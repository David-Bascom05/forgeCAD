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

/-! ## `shl_into` -/

/-- `shl_into`'s carry loop (`bits % 64 = r > 0`), its first `k` steps, writing from limb `w`. -/
def shlRun (a : ℕ → ℕ) (w r : ℕ) : ℕ → (ℕ → ℕ) × ℕ
  | 0 => (fun _ => 0, 0)
  | k + 1 =>
    let o := shlRun a w r k
    (Function.update o.1 (w + k) (((a k * 2 ^ r) % B) ||| o.2), a k / 2 ^ (64 - r))

/-- `shl_into(a, bits, out)`: `words = bits / 64` zero limbs, then `a` shifted by `bits % 64`. -/
def shlInto (a : ℕ → ℕ) (la bits : ℕ) : ℕ → ℕ :=
  if bits % 64 = 0 then fun m => if bits / 64 ≤ m ∧ m < bits / 64 + la then a (m - bits / 64) else 0
  else Function.update (shlRun a (bits / 64) (bits % 64) la).1 (bits / 64 + la) (shlRun a (bits / 64) (bits % 64) la).2

/-- One limb shifted by `0 < r < 64`: the low part, or-ed with a carry under `2^r`, is a sum. -/
lemma shl_limb {x c r : ℕ} (hx : x < B) (hr0 : 0 < r) (hr : r < 64) (hc : c < 2 ^ r) :
    ((x * 2 ^ r) % B ||| c) = 2 ^ r * (x % 2 ^ (64 - r)) + c ∧
      ((x * 2 ^ r) % B ||| c) + B * (x / 2 ^ (64 - r)) = x * 2 ^ r + c ∧
      ((x * 2 ^ r) % B ||| c) < B ∧ x / 2 ^ (64 - r) < 2 ^ r := by
  have hB : B = 2 ^ r * 2 ^ (64 - r) := by rw [← pow_add, Nat.add_sub_cancel' (by omega)]
  have hq : 0 < 2 ^ (64 - r) := by positivity
  have hdm := Nat.div_add_mod x (2 ^ (64 - r))
  have hm : (x * 2 ^ r) % B = 2 ^ r * (x % 2 ^ (64 - r)) := by
    rw [hB]
    conv_lhs => rw [← hdm]
    rw [show (2 ^ (64 - r) * (x / 2 ^ (64 - r)) + x % 2 ^ (64 - r)) * 2 ^ r =
      2 ^ r * (x % 2 ^ (64 - r)) + 2 ^ r * 2 ^ (64 - r) * (x / 2 ^ (64 - r)) by ring, Nat.add_mul_mod_self_left]
    exact Nat.mod_eq_of_lt (by
      have := Nat.mod_lt x hq
      calc 2 ^ r * (x % 2 ^ (64 - r)) < 2 ^ r * 2 ^ (64 - r) := Nat.mul_lt_mul_of_pos_left this (by positivity)
        _ = _ := rfl)
  have hor : ((x * 2 ^ r) % B ||| c) = 2 ^ r * (x % 2 ^ (64 - r)) + c := by
    rw [hm, Nat.two_pow_add_eq_or_of_lt hc]
  refine ⟨hor, ?_, ?_, ?_⟩
  · rw [hor, hB]
    conv_rhs => rw [← hdm]
    ring
  · rw [hor, hB]
    have h1 : x % 2 ^ (64 - r) + 1 ≤ 2 ^ (64 - r) := Nat.mod_lt x hq
    have h2 : 2 ^ r * (x % 2 ^ (64 - r) + 1) ≤ 2 ^ r * 2 ^ (64 - r) := Nat.mul_le_mul_left _ h1
    rw [Nat.mul_add, Nat.mul_one] at h2
    omega
  · rw [Nat.div_lt_iff_lt_mul hq, ← hB]; exact hx

lemma shlRun_spec {a : ℕ → ℕ} {la w r : ℕ} (ha : Limbs a la) (hr0 : 0 < r) (hr : r < 64) :
    ∀ k ≤ la, valF (shlRun a w r k).1 (w + k) + B ^ (w + k) * (shlRun a w r k).2 = valF a k * 2 ^ r * B ^ w ∧
      Limbs (shlRun a w r k).1 (w + k) ∧ (∀ m, (m < w ∨ w + k ≤ m) → (shlRun a w r k).1 m = 0) ∧
      (shlRun a w r k).2 < 2 ^ r := by
  intro k hk
  induction k with
  | zero =>
    refine ⟨?_, fun i _ => by simp [shlRun], fun m _ => rfl, by simp [shlRun]⟩
    simp only [shlRun, add_zero, mul_zero, valF]
    simp
  | succ k ih =>
    obtain ⟨e, lim, z, cB⟩ := ih (by omega)
    obtain ⟨l1, l2, l3, l4⟩ := shl_limb (ha k (by omega)) hr0 hr cB
    simp only [shlRun]
    generalize hrr : shlRun a w r k = o at e lim z cB l1 l2 l3 l4
    refine ⟨?_, ?_, ?_, l4⟩
    · rw [show w + (k + 1) = (w + k) + 1 by ring, valF_succ, Function.update_self, valF_update_ge le_rfl,
        valF_succ, pow_succ]
      zify at e l2 ⊢
      linear_combination e + (B : ℤ) ^ (w + k) * l2
    · intro i hi
      by_cases h : i = w + k
      · subst h; rw [Function.update_self]; exact l3
      · rw [Function.update_of_ne h]; exact lim i (by omega)
    · intro m hm
      rw [Function.update_of_ne (by omega)]; exact z m (by omega)

lemma valF_shift {a : ℕ → ℕ} {w la : ℕ} :
    valF (fun m => if w ≤ m ∧ m < w + la then a (m - w) else 0) (w + la) = B ^ w * valF a la := by
  unfold valF
  rw [sum_range_add]
  have h1 : ∑ x ∈ range w, (if w ≤ x ∧ x < w + la then a (x - w) else 0) * B ^ x = 0 :=
    sum_eq_zero (fun x hx => by simp only [mem_range] at hx; simp [show ¬ w ≤ x by omega])
  rw [h1, zero_add, mul_sum]
  exact sum_congr rfl (fun x hx => by
    simp only [mem_range] at hx
    simp only [show w ≤ w + x ∧ w + x < w + la from ⟨by omega, by omega⟩, and_self, if_true,
      show w + x - w = x by omega, pow_add]; ring)

/-- **`shl_into`.** `out` (length `la + bits/64 + 1`) is `a · 2^bits`, every limb below `2^64`. -/
theorem shl_into_spec {a : ℕ → ℕ} {la bits : ℕ} (ha : Limbs a la) :
    valF (shlInto a la bits) (bits / 64 + la + 1) = valF a la * 2 ^ bits ∧
      Limbs (shlInto a la bits) (bits / 64 + la + 1) := by
  have hbits : 2 ^ bits = B ^ (bits / 64) * 2 ^ (bits % 64) := by
    rw [B, ← pow_mul, ← pow_add]; congr 1; omega
  unfold shlInto
  split_ifs with h0
  · refine ⟨?_, ?_⟩
    · rw [valF_succ, valF_shift]
      simp only [show ¬ (bits / 64 + la < bits / 64 + la) from lt_irrefl _, and_false, if_false, zero_mul, add_zero]
      rw [hbits, h0, pow_zero, mul_one, mul_comm]
    · intro i hi
      dsimp only
      split_ifs with h
      · exact ha _ (by omega)
      · norm_num
  · obtain ⟨e, lim, z, cB⟩ := shlRun_spec (w := bits / 64) (r := bits % 64) ha (Nat.pos_of_ne_zero h0) (Nat.mod_lt _ (by norm_num)) la le_rfl
    refine ⟨?_, ?_⟩
    · rw [valF_succ, Function.update_self, valF_update_ge le_rfl, hbits]
      zify at e ⊢
      linear_combination e
    · intro i hi
      by_cases h : i = bits / 64 + la
      · subst h; rw [Function.update_self]
        exact lt_of_lt_of_le cB (Nat.pow_le_pow_right (by norm_num) (by omega))
      · rw [Function.update_of_ne h]; exact lim i (by omega)

/-! ## `trimmed` and `cmp_mag` -/

/-- `trimmed`: the length with the zero top limbs dropped. -/
def trimLen (f : ℕ → ℕ) : ℕ → ℕ
  | 0 => 0
  | n + 1 => if f n = 0 then trimLen f n else n + 1

theorem trimLen_spec (f : ℕ → ℕ) (n : ℕ) :
    trimLen f n ≤ n ∧ valF f (trimLen f n) = valF f n ∧ (trimLen f n = 0 ∨ f (trimLen f n - 1) ≠ 0) := by
  induction n with
  | zero => simp [trimLen]
  | succ n ih =>
    simp only [trimLen]
    split_ifs with h
    · obtain ⟨h1, h2, h3⟩ := ih
      exact ⟨by omega, by rw [h2, valF_succ, h, zero_mul, add_zero], h3⟩
    · exact ⟨le_rfl, rfl, Or.inr (by simpa using h)⟩

lemma valF_ge_top {f : ℕ → ℕ} {n : ℕ} (hn : 0 < n) : f (n - 1) * B ^ (n - 1) ≤ valF f n := by
  obtain ⟨m, rfl⟩ : ∃ m, n = m + 1 := ⟨n - 1, by omega⟩
  rw [valF_succ]; simp

/-- The top-down scan of `cmp_mag` over `i` limbs. -/
def cmpTop (a b : ℕ → ℕ) : ℕ → Ordering
  | 0 => .eq
  | i + 1 => if a i ≠ b i then compare (a i) (b i) else cmpTop a b i

/-- `cmp_mag(a, b)`: lengths first, then the limbs from the top. -/
def cmpMag (a : ℕ → ℕ) (la : ℕ) (b : ℕ → ℕ) (lb : ℕ) : Ordering :=
  if la ≠ lb then compare la lb else cmpTop a b la

lemma compare_add_same (x y c : ℕ) : compare (x + c) (y + c) = compare x y := by
  rcases lt_trichotomy x y with h | h | h
  · rw [compare_lt_iff_lt.mpr h, compare_lt_iff_lt.mpr (by omega)]
  · subst h; rw [compare_eq_iff_eq.mpr rfl, compare_eq_iff_eq.mpr rfl]
  · rw [compare_gt_iff_gt.mpr h, compare_gt_iff_gt.mpr (by omega)]

lemma cmpTop_spec {a b : ℕ → ℕ} {n : ℕ} (ha : Limbs a n) (hb : Limbs b n) :
    ∀ i ≤ n, cmpTop a b i = compare (valF a i) (valF b i) := by
  intro i hi
  induction i with
  | zero => simp [cmpTop, valF]
  | succ i ih =>
    simp only [cmpTop]
    have la := valF_lt (fun j hj => ha j (by omega) : Limbs a i)
    have lb := valF_lt (fun j hj => hb j (by omega) : Limbs b i)
    rw [valF_succ, valF_succ]
    split_ifs with h
    · rcases lt_or_gt_of_ne h with h' | h'
      · rw [compare_lt_iff_lt.mpr h', compare_lt_iff_lt.mpr]
        have : (a i + 1) * B ^ i ≤ b i * B ^ i := Nat.mul_le_mul_right _ h'
        nlinarith
      · rw [compare_gt_iff_gt.mpr h', compare_gt_iff_gt.mpr]
        have : (b i + 1) * B ^ i ≤ a i * B ^ i := Nat.mul_le_mul_right _ h'
        nlinarith
    · push Not at h
      rw [ih (by omega), h, compare_add_same]

/-- **`cmp_mag`.** On trimmed limb slices, `cmp_mag` is the order of the magnitudes. -/
theorem cmp_mag_spec {a b : ℕ → ℕ} {la lb : ℕ} (ha : Limbs a la) (hb : Limbs b lb)
    (ta : la = 0 ∨ a (la - 1) ≠ 0) (tb : lb = 0 ∨ b (lb - 1) ≠ 0) :
    cmpMag a la b lb = compare (valF a la) (valF b lb) := by
  unfold cmpMag
  split_ifs with h
  · have key : ∀ {x y : ℕ → ℕ} {lx ly : ℕ}, Limbs x lx → (ly = 0 ∨ y (ly - 1) ≠ 0) → lx < ly →
        valF x lx < valF y ly := by
      intro x y lx ly hx ty hl
      rcases ty with h0 | h0
      · omega
      · have h1 := valF_lt hx
        have h2 := valF_ge_top (f := y) (show 0 < ly by omega)
        have h3 : B ^ lx ≤ B ^ (ly - 1) := Nat.pow_le_pow_right (by norm_num) (by omega)
        have h4 : 1 ≤ y (ly - 1) := Nat.one_le_iff_ne_zero.mpr h0
        have h5 : B ^ (ly - 1) ≤ y (ly - 1) * B ^ (ly - 1) := Nat.le_mul_of_pos_left _ h4
        omega
    rcases lt_or_gt_of_ne h with h' | h'
    · rw [compare_lt_iff_lt.mpr h', compare_lt_iff_lt.mpr (key ha tb h')]
    · rw [compare_gt_iff_gt.mpr h', compare_gt_iff_gt.mpr (key hb ta h')]
  · push Not at h
    subst h
    exact cmpTop_spec ha hb la le_rfl

end IEEE754.Exact
