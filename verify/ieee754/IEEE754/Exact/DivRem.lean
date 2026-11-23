/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Exact.Big

/-!
# dol-curved-rust's `divrem_mag` (exact.rs, 5aca447)

`divrem_mag(a, b)` on trimmed magnitudes, `b ≠ 0`, gives `a = q·b + r` with `r < b`:
- the short division for a one-limb `b` (`shortDiv_spec`);
- Knuth's algorithm D otherwise (`knuth_spec`, below).
Each loop is modelled index by index, like the limb loops in `Limbs.lean`.
-/

namespace IEEE754.Exact

open Finset

/-! ## Short division -/

/-- The short division's loop from the top: after `k` steps, limbs `la − k ..` of the quotient
and the remainder. -/
def shortRun (a : ℕ → ℕ) (la d : ℕ) : ℕ → (ℕ → ℕ) × ℕ
  | 0 => (fun _ => 0, 0)
  | k + 1 =>
    let r := shortRun a la d k
    let i := la - (k + 1)
    let cur := r.2 * B + a i
    (Function.update r.1 i (cur / d), cur % d)

lemma shortRun_spec {a : ℕ → ℕ} {la d : ℕ} (ha : Limbs a la) (hd : 0 < d) (hdB : d < B) :
    ∀ k ≤ la, valF (shortRun a la d k).1 la * d + (shortRun a la d k).2 * B ^ (la - k) + valF a (la - k) =
        valF a la ∧ (shortRun a la d k).2 < d ∧ Limbs (shortRun a la d k).1 la ∧
      (∀ i, i < la - k ∨ la ≤ i → (shortRun a la d k).1 i = 0) := by
  intro k hk
  induction k with
  | zero =>
    refine ⟨by simp [shortRun, valF], hd, fun i _ => by simp [shortRun], fun i _ => rfl⟩
  | succ k ih =>
    obtain ⟨e, rd, lim, z⟩ := ih (by omega)
    simp only [shortRun]
    generalize hr : shortRun a la d k = r at e rd lim z
    set i := la - (k + 1) with hi
    have hai : a i < B := ha i (by omega)
    have hcur : r.2 * B + a i < d * B := by
      have : r.2 * B ≤ (d - 1) * B := Nat.mul_le_mul_right _ (by omega)
      have : (d - 1) * B + B = d * B := by
        rw [Nat.sub_mul, one_mul]; have : B ≤ d * B := Nat.le_mul_of_pos_left _ hd; omega
      omega
    have hq : (r.2 * B + a i) / d < B := Nat.div_lt_of_lt_mul (by linarith [hcur])
    have hdm := Nat.div_add_mod (r.2 * B + a i) d
    have hz : r.1 i = 0 := z i (Or.inl (by omega))
    refine ⟨?_, Nat.mod_lt _ hd, ?_, ?_⟩
    · have u := valF_update_lt (w := r.1) (v := (r.2 * B + a i) / d) (show i < la by omega)
      rw [hz, zero_mul, add_zero] at u
      have hla : la - k = i + 1 := by omega
      rw [hla, valF_succ, pow_succ] at e
      zify at u e hdm ⊢
      linear_combination u * (d : ℤ) + e + (B : ℤ) ^ i * hdm
    · intro m hm
      by_cases h : m = i
      · subst h; rw [Function.update_self]; exact hq
      · rw [Function.update_of_ne h]; exact lim m hm
    · intro m hm
      rw [Function.update_of_ne (by omega)]; exact z m (by omega)

/-- **Short division** (`b.len() == 1`, `d = b[0] > 0`): `q·d + r = a` with `r < d`, every
quotient limb under `2^64`. Since `cur < d·2^64`, the u128 `cur / d` fits a u64, and
`(r << 64) | a[i]` is `r·2^64 + a[i]` (`r < 2^64`). -/
theorem shortDiv_spec {a : ℕ → ℕ} {la d : ℕ} (ha : Limbs a la) (hd : 0 < d) (hdB : d < B) :
    valF (shortRun a la d la).1 la * d + (shortRun a la d la).2 = valF a la ∧
      (shortRun a la d la).2 < d ∧ Limbs (shortRun a la d la).1 la := by
  obtain ⟨e, rd, lim, -⟩ := shortRun_spec ha hd hdB la le_rfl
  simp only [Nat.sub_self, pow_zero, mul_one] at e
  have : valF a 0 = 0 := by simp [valF]
  rw [this, add_zero] at e
  exact ⟨e, rd, lim⟩

/-! ## Knuth D: the quotient digit estimate (TAOCP 4.3.1, Theorem B and the refined test) -/

section Estimate

variable {P V U v1 v2 Vl u0 u1 u2 Ul : ℕ}

/-- The setting of one step: `V = v1·B·P + v2·P + Vl` normalised (`2·v1 ≥ B`), `U = u0·B²·P + u1·B·P +
u2·P + Ul` its window, `U < B·V`; `P = B^(n−2)`. -/
structure Step (P V U v1 v2 Vl u0 u1 u2 Ul : ℕ) : Prop where
  hP : 0 < P
  hV : V = v1 * B * P + v2 * P + Vl
  hVl : Vl < P
  hv1 : v1 < B
  hv1n : B ≤ 2 * v1
  hv2 : v2 < B
  hU : U = u0 * B * B * P + u1 * B * P + u2 * P + Ul
  hUl : Ul < P
  hu1 : u1 < B
  hu2 : u2 < B
  hUV : U < B * V

lemma Step.Vpos (h : Step P V U v1 v2 Vl u0 u1 u2 Ul) : 0 < V := by
  have := h.hV; have := h.hP; have : 0 < v1 := by have := h.hv1n; rw [B_val] at this; omega
  have : 0 < v1 * B * P := by positivity
  omega

/-- `q < B`. -/
lemma Step.q_lt (h : Step P V U v1 v2 Vl u0 u1 u2 Ul) : U / V < B :=
  (Nat.div_lt_iff_lt_mul h.Vpos).mpr (by linarith [h.hUV])

/-- Theorem B, lower half: `q ≤ ⌊(u0 B + u1)/v1⌋`. -/
lemma Step.q_le_qhat (h : Step P V U v1 v2 Vl u0 u1 u2 Ul) : U / V ≤ (u0 * B + u1) / v1 := by
  have hv1p : 0 < v1 := by have := h.hv1n; rw [B_val] at this; omega
  rw [Nat.le_div_iff_mul_le hv1p]
  have h1 : U / V * V ≤ U := Nat.div_mul_le_self U V
  have h2 : U < ((u0 * B + u1) + 1) * B * P := by
    rw [h.hU]; have := h.hu2; have := h.hUl
    have : u2 * P + Ul < B * P := by
      have : u2 * P ≤ (B - 1) * P := Nat.mul_le_mul_right _ (by omega)
      have : (B - 1) * P + P = B * P := by rw [Nat.sub_mul, one_mul]; have : P ≤ B * P := Nat.le_mul_of_pos_left _ (by norm_num); omega
      omega
    nlinarith
  have h3 : v1 * B * P ≤ V := by rw [h.hV]; omega
  have h4 : U / V * (v1 * B * P) < ((u0 * B + u1) + 1) * B * P :=
    lt_of_le_of_lt (le_trans (Nat.mul_le_mul_left _ h3) h1) h2
  have hBP : 0 < B * P := by have := h.hP; positivity
  have : U / V * v1 * (B * P) < ((u0 * B + u1) + 1) * (B * P) := by
    calc U / V * v1 * (B * P) = U / V * (v1 * B * P) := by ring
      _ < _ := h4
      _ = _ := by ring
  have := Nat.lt_of_mul_lt_mul_right this
  omega

/-- `u0 ≤ v1`, so the first estimate is at most `B + 1`. -/
lemma Step.qhat_le (h : Step P V U v1 v2 Vl u0 u1 u2 Ul) : (u0 * B + u1) / v1 ≤ B + 1 := by
  have hv1p : 0 < v1 := by have := h.hv1n; rw [B_val] at this; omega
  have hu0 : u0 ≤ v1 := by
    by_contra hc; push Not at hc
    have h1 : (v1 + 1) * B * B * P ≤ U := by
      rw [h.hU]; have : (v1 + 1) * B * B * P ≤ u0 * B * B * P := by
        apply Nat.mul_le_mul_right; apply Nat.mul_le_mul_right; apply Nat.mul_le_mul_right; omega
      omega
    have h2 : V < (v1 + 1) * B * P := by
      rw [h.hV]
      have : v2 * P + Vl < B * P := by
        have : v2 * P ≤ (B - 1) * P := Nat.mul_le_mul_right _ (by have := h.hv2; omega)
        have : (B - 1) * P + P = B * P := by rw [Nat.sub_mul, one_mul]; have : P ≤ B * P := Nat.le_mul_of_pos_left _ (by norm_num); omega
        have := h.hVl; omega
      nlinarith
    have := h.hUV
    have : B * V < B * ((v1 + 1) * B * P) := Nat.mul_lt_mul_of_pos_left h2 (by norm_num)
    nlinarith
  have : (u0 * B + u1) / v1 < B + 2 := by
    apply Nat.div_lt_of_lt_mul
    have := h.hu1; have := h.hv1n
    have : u0 * B ≤ v1 * B := Nat.mul_le_mul_right _ hu0
    nlinarith
  omega

/-- The refined test, when it holds, shows `qhat > q`. -/
lemma Step.test_gt (h : Step P V U v1 v2 Vl u0 u1 u2 Ul) {qhat rhat : ℕ}
    (hr : rhat + qhat * v1 = u0 * B + u1)
    (ht : B ≤ qhat ∨ rhat * B + u2 < qhat * v2) : U / V < qhat := by
  rcases ht with ht | ht
  · exact lt_of_lt_of_le h.q_lt ht
  · rw [Nat.div_lt_iff_lt_mul h.Vpos]
    have e1 : (u0 * B + u1) * B + u2 + 1 ≤ qhat * (v1 * B + v2) := by nlinarith
    have e2 : U < ((u0 * B + u1) * B + u2 + 1) * P := by
      rw [h.hU]; have := h.hUl; nlinarith
    have e3 : qhat * (v1 * B + v2) * P ≤ qhat * V := by
      rw [h.hV]; nlinarith
    have : ((u0 * B + u1) * B + u2 + 1) * P ≤ qhat * (v1 * B + v2) * P := Nat.mul_le_mul_right _ e1
    nlinarith

/-- At the loop's exit, `qhat ≤ q + 1`. -/
lemma Step.exit_le (h : Step P V U v1 v2 Vl u0 u1 u2 Ul) {qhat : ℕ}
    (hE : qhat * (v1 * B + v2) ≤ (u0 * B + u1) * B + u2) (hq : qhat ≤ B + 1) : qhat ≤ U / V + 1 := by
  have hVp := h.Vpos
  have e1 : qhat * V ≤ ((u0 * B + u1) * B + u2) * P + qhat * Vl := by
    rw [h.hV]
    have : qhat * (v1 * B + v2) * P ≤ ((u0 * B + u1) * B + u2) * P := Nat.mul_le_mul_right _ hE
    nlinarith
  have e2 : ((u0 * B + u1) * B + u2) * P ≤ U := by rw [h.hU]; nlinarith
  have e3 : qhat * Vl ≤ (B + 1) * Vl := Nat.mul_le_mul_right _ hq
  have e4 : (B + 1) * Vl < V := by
    have hVl := h.hVl
    have : (B + 1) * Vl < (B + 1) * P := Nat.mul_lt_mul_of_pos_left hVl (by norm_num)
    have : (B + 1) * P ≤ v1 * B * P := by
      apply Nat.mul_le_mul_right
      have := h.hv1n; rw [B_val] at *; nlinarith
    rw [h.hV]; omega
  have e5 : U < (U / V + 1) * V := Nat.lt_mul_div_succ U hVp |> fun h' => by rw [mul_comm]; exact h'
  have : qhat * V < (U / V + 2) * V := by nlinarith
  have := Nat.lt_of_mul_lt_mul_right this
  omega

end Estimate

/-- The correction loop: `while qhat ≥ B || qhat·v2 > rhat·B + u2 { qhat −= 1; rhat += v1;
if rhat ≥ B { break } }`. The guard `0 < qhat` is implied by the test (`qhat > q ≥ 0`,
`Step.test_gt`); it only makes the recursion visibly terminate. -/
def corr (v1 v2 u2 : ℕ) (qhat rhat : ℕ) : ℕ × ℕ :=
  if 0 < qhat ∧ (B ≤ qhat ∨ rhat * B + u2 < qhat * v2) then
    if B ≤ rhat + v1 then (qhat - 1, rhat + v1) else corr v1 v2 u2 (qhat - 1) (rhat + v1)
  else (qhat, rhat)
termination_by qhat
decreasing_by omega

/-- **The corrected estimate.** From Theorem B's start (`qhat = ⌊num/v1⌋`, `rhat = num mod v1`),
the loop ends with `qhat ∈ {q, q + 1}`, `qhat ≤ B`, and `rhat = num − qhat·v1`. -/
theorem Step.corr_spec {P V U v1 v2 Vl u0 u1 u2 Ul : ℕ} (h : Step P V U v1 v2 Vl u0 u1 u2 Ul) :
    ∀ qhat rhat, rhat + qhat * v1 = u0 * B + u1 → U / V ≤ qhat → qhat ≤ B + 1 →
      (corr v1 v2 u2 qhat rhat).2 + (corr v1 v2 u2 qhat rhat).1 * v1 = u0 * B + u1 ∧
      U / V ≤ (corr v1 v2 u2 qhat rhat).1 ∧ (corr v1 v2 u2 qhat rhat).1 ≤ U / V + 1 ∧
      (corr v1 v2 u2 qhat rhat).1 ≤ B := by
  intro qhat
  induction qhat using Nat.strong_induction_on with
  | _ qhat ih =>
  intro rhat hr hq hB
  rw [corr]
  split_ifs with ht hbr
  · -- the test holds and the loop breaks
    have hgt := h.test_gt hr ht.2
    refine ⟨by rw [Nat.sub_mul, one_mul]; have : v1 ≤ qhat * v1 := Nat.le_mul_of_pos_left _ ht.1; omega,
      by omega, ?_, by omega⟩
    apply h.exit_le _ (by omega)
    have e1 : (qhat - 1) * v1 * B + B * B ≤ (u0 * B + u1) * B := by
      have : (qhat - 1) * v1 + B ≤ u0 * B + u1 := by
        rw [Nat.sub_mul, one_mul]; have : v1 ≤ qhat * v1 := Nat.le_mul_of_pos_left _ ht.1; omega
      nlinarith
    have e2 : (qhat - 1) * v2 < B * B := by
      have := h.hv2
      calc (qhat - 1) * v2 ≤ B * v2 := Nat.mul_le_mul_right _ (by omega)
        _ < B * B := Nat.mul_lt_mul_of_pos_left this (by norm_num)
    nlinarith
  · -- the test holds and the loop goes on
    have hgt := h.test_gt hr ht.2
    exact ih (qhat - 1) (by omega) (rhat + v1)
      (by rw [Nat.sub_mul, one_mul]; have : v1 ≤ qhat * v1 := Nat.le_mul_of_pos_left _ ht.1; omega)
      (by omega) (by omega)
  · -- the loop stops
    refine ⟨hr, hq, ?_, ?_⟩
    · by_cases h0 : qhat = 0
      · rw [h0]; exact Nat.zero_le _
      · push Not at ht
        obtain ⟨t1, t2⟩ := ht (by omega)
        apply h.exit_le _ hB
        nlinarith
    · by_cases h0 : qhat = 0
      · rw [h0]; exact Nat.zero_le _
      · push Not at ht
        exact (ht (by omega)).1.le

/-! ## Knuth D: multiply-subtract and add back, on the window `u[j..=j+n]` -/

/-- The window's value. -/
def win (u : ℕ → ℕ) (j k : ℕ) : ℕ := valF (fun i => u (j + i)) k

/-- `u[i+j] −= qhat·v[i]` with carry and borrow, its first `i` steps: `(u, carry, borrow)`. -/
def msRun (u v : ℕ → ℕ) (j qhat : ℕ) : ℕ → (ℕ → ℕ) × ℕ × ℕ
  | 0 => (u, 0, 0)
  | i + 1 =>
    let r := msRun u v j qhat i
    let p := qhat * v i + r.2.1
    let sb := ovs (r.1 (i + j)) (p % B)
    let tb := ovs sb.1 r.2.2
    (Function.update r.1 (i + j) tb.1, p / B, sb.2 + tb.2)

lemma msRun_spec {u v : ℕ → ℕ} {j n qhat : ℕ} (hu : ∀ i ≤ n, u (j + i) < B) (hv : Limbs v n)
    (hq : qhat ≤ B) :
    ∀ i ≤ n, win u j i + B ^ i * ((msRun u v j qhat i).2.1 + (msRun u v j qhat i).2.2) =
        win (msRun u v j qhat i).1 j i + qhat * valF v i ∧
      (msRun u v j qhat i).2.1 < B ∧ (msRun u v j qhat i).2.2 ≤ 1 ∧
      (∀ m, m < j ∨ j + i ≤ m → (msRun u v j qhat i).1 m = u m) ∧
      (∀ k < i, (msRun u v j qhat i).1 (j + k) < B) ∧
      (∀ k < i, qhat * v k + (msRun u v j qhat k).2.1 < B * B) := by
  intro i hi
  induction i with
  | zero => simp [msRun, win, valF]
  | succ i ih =>
    obtain ⟨e, cB, b1, keep, lim, pB⟩ := ih (by omega)
    simp only [msRun]
    generalize hr : msRun u v j qhat i = r at e cB b1 keep lim pB
    have hvi : v i < B := hv i (by omega)
    have hp : qhat * v i + r.2.1 < B * B := by
      have : qhat * v i ≤ B * (B - 1) := Nat.mul_le_mul hq (by omega)
      have : B * (B - 1) + B = B * B := by rw [Nat.mul_sub, mul_one]; have : B ≤ B * B := Nat.le_mul_of_pos_left _ (by norm_num); omega
      omega
    have hui : r.1 (i + j) < B := by rw [keep (i + j) (Or.inr (by omega))]; exact hu i (by omega) |> fun h => by rwa [add_comm] at h
    obtain ⟨t2, tc, teq⟩ := ovs2 hui (Nat.mod_lt _ (by norm_num) : (qhat * v i + r.2.1) % B < B) b1
    have hdm := Nat.div_add_mod (qhat * v i + r.2.1) B
    refine ⟨?_, Nat.div_lt_of_lt_mul hp, tc, ?_, ?_, ?_⟩
    · have hw1 : win u j (i + 1) = win u j i + u (j + i) * B ^ i := valF_succ _ _
      have hw2 : win (Function.update r.1 (i + j) (ovs (ovs (r.1 (i + j)) ((qhat * v i + r.2.1) % B)).1 r.2.2).1) j (i + 1) =
          win r.1 j i + (ovs (ovs (r.1 (i + j)) ((qhat * v i + r.2.1) % B)).1 r.2.2).1 * B ^ i := by
        unfold win; rw [valF_succ]; simp only [show j + i = i + j by ring, Function.update_self]
        congr 1
        exact valF_congr (fun k hk => by rw [Function.update_of_ne (by omega)])
      have hu' : u (j + i) = r.1 (i + j) := by rw [keep (i + j) (Or.inr (by omega)), add_comm]
      rw [hw1, hw2, valF_succ, hu', pow_succ]
      zify at e teq hdm ⊢
      linear_combination e + (B : ℤ) ^ i * teq + (B : ℤ) ^ i * hdm
    · intro m hm
      rw [Function.update_of_ne (by omega)]; exact keep m (by omega)
    · intro k hk
      rcases Nat.lt_succ_iff_lt_or_eq.mp hk with h | h
      · rw [Function.update_of_ne (by omega)]; exact lim k h
      · subst h; rw [show j + k = k + j by ring, Function.update_self]; exact t2
    · intro k hk
      rcases Nat.lt_succ_iff_lt_or_eq.mp hk with h | h
      · exact pB k h
      · subst h; rw [hr]; exact hp

/-- Multiply-subtract with its top limb: `(u, borrowed)`. -/
def msStep (u v : ℕ → ℕ) (j n qhat : ℕ) : (ℕ → ℕ) × ℕ :=
  let r := msRun u v j qhat n
  let sb := ovs (r.1 (j + n)) r.2.1
  let tb := ovs sb.1 r.2.2
  (Function.update r.1 (j + n) tb.1, sb.2 + tb.2)

lemma win_update_top {w : ℕ → ℕ} {j n x : ℕ} :
    win (Function.update w (j + n) x) j (n + 1) = win w j n + x * B ^ n := by
  unfold win; rw [valF_succ]; simp only [Function.update_self]
  congr 1; exact valF_congr (fun k hk => by rw [Function.update_of_ne (by omega)])

lemma msStep_spec {u v : ℕ → ℕ} {j n qhat : ℕ} (hu : ∀ i ≤ n, u (j + i) < B) (hv : Limbs v n)
    (hq : qhat ≤ B) :
    win u j (n + 1) + B ^ (n + 1) * (msStep u v j n qhat).2 = win (msStep u v j n qhat).1 j (n + 1) + qhat * valF v n ∧
      (msStep u v j n qhat).2 ≤ 1 ∧ (∀ i ≤ n, (msStep u v j n qhat).1 (j + i) < B) ∧
      (∀ m, m < j ∨ j + n < m → (msStep u v j n qhat).1 m = u m) := by
  obtain ⟨e, cB, b1, keep, lim, -⟩ := msRun_spec hu hv hq n le_rfl
  unfold msStep; simp only
  generalize hr : msRun u v j qhat n = r at e cB b1 keep lim
  have htop : r.1 (j + n) = u (j + n) := keep _ (Or.inr le_rfl)
  obtain ⟨t2, tc, teq⟩ := ovs2 (show r.1 (j + n) < B by rw [htop]; exact hu n le_rfl) cB b1
  refine ⟨?_, tc, ?_, ?_⟩
  · rw [win_update_top]
    have hw : win u j (n + 1) = win u j n + u (j + n) * B ^ n := valF_succ _ _
    rw [hw, ← htop, pow_succ]
    zify at e teq ⊢
    linear_combination e + (B : ℤ) ^ n * teq
  · intro i hi
    rcases Nat.lt_succ_iff_lt_or_eq.mp (Nat.lt_succ_of_le hi) with h | h
    · rw [Function.update_of_ne (by omega)]; exact lim i h
    · subst h; rw [Function.update_self]; exact t2
  · intro m hm
    rw [Function.update_of_ne (by omega)]; exact keep m (by omega)

/-- Add `v` back to the window, its first `i` steps: `(u, carry)`. -/
def abRun (u v : ℕ → ℕ) (j : ℕ) : ℕ → (ℕ → ℕ) × ℕ
  | 0 => (u, 0)
  | i + 1 =>
    let r := abRun u v j i
    let sc := ov (r.1 (i + j)) (v i)
    let tc := ov sc.1 r.2
    (Function.update r.1 (i + j) tc.1, sc.2 + tc.2)

/-- Add back with the top limb's `wrapping_add`. -/
def abStep (u v : ℕ → ℕ) (j n : ℕ) : ℕ → ℕ :=
  let r := abRun u v j n
  Function.update r.1 (j + n) ((r.1 (j + n) + r.2) % B)

lemma abRun_spec {u v : ℕ → ℕ} {j n : ℕ} (hu : ∀ i ≤ n, u (j + i) < B) (hv : Limbs v n) :
    ∀ i ≤ n, win u j i + valF v i = win (abRun u v j i).1 j i + B ^ i * (abRun u v j i).2 ∧
      (abRun u v j i).2 ≤ 1 ∧ (∀ m, m < j ∨ j + i ≤ m → (abRun u v j i).1 m = u m) ∧
      (∀ k < i, (abRun u v j i).1 (j + k) < B) := by
  intro i hi
  induction i with
  | zero => simp [abRun, win, valF]
  | succ i ih =>
    obtain ⟨e, c1, keep, lim⟩ := ih (by omega)
    simp only [abRun]
    generalize hr : abRun u v j i = r at e c1 keep lim
    have hui : r.1 (i + j) < B := by rw [keep (i + j) (Or.inr (by omega)), add_comm]; exact hu i (by omega)
    obtain ⟨t2, tc, teq⟩ := ov2 hui (hv i (by omega)) c1
    refine ⟨?_, tc, ?_, ?_⟩
    · have hw1 : win u j (i + 1) = win u j i + u (j + i) * B ^ i := valF_succ _ _
      have hw2 : win (Function.update r.1 (i + j) (ov (ov (r.1 (i + j)) (v i)).1 r.2).1) j (i + 1) =
          win r.1 j i + (ov (ov (r.1 (i + j)) (v i)).1 r.2).1 * B ^ i := by
        unfold win; rw [valF_succ]; simp only [show j + i = i + j by ring, Function.update_self]
        congr 1
        exact valF_congr (fun k hk => by rw [Function.update_of_ne (by omega)])
      have hu' : u (j + i) = r.1 (i + j) := by rw [keep (i + j) (Or.inr (by omega)), add_comm]
      rw [hw1, hw2, valF_succ, hu', pow_succ]
      zify at e teq ⊢
      linear_combination e - (B : ℤ) ^ i * teq
    · intro m hm
      rw [Function.update_of_ne (by omega)]; exact keep m (by omega)
    · intro k hk
      rcases Nat.lt_succ_iff_lt_or_eq.mp hk with h | h
      · rw [Function.update_of_ne (by omega)]; exact lim k h
      · subst h; rw [show j + k = k + j by ring, Function.update_self]; exact t2

lemma abStep_spec {u v : ℕ → ℕ} {j n : ℕ} (hu : ∀ i ≤ n, u (j + i) < B) (hv : Limbs v n) :
    ∃ o ≤ 1, win u j (n + 1) + valF v n = win (abStep u v j n) j (n + 1) + B ^ (n + 1) * o ∧
      (∀ i ≤ n, abStep u v j n (j + i) < B) ∧ (∀ m, m < j ∨ j + n < m → abStep u v j n m = u m) := by
  obtain ⟨e, c1, keep, lim⟩ := abRun_spec hu hv n le_rfl
  unfold abStep; simp only
  generalize hr : abRun u v j n = r at e c1 keep lim
  have htop : r.1 (j + n) = u (j + n) := keep _ (Or.inr le_rfl)
  have hdm := Nat.div_add_mod (r.1 (j + n) + r.2) B
  have hlt : r.1 (j + n) + r.2 < 2 * B := by rw [htop]; have := hu n le_rfl; omega
  refine ⟨(r.1 (j + n) + r.2) / B, by
    have : (r.1 (j + n) + r.2) / B < 2 := Nat.div_lt_of_lt_mul (by linarith); omega, ?_, ?_, ?_⟩
  · rw [win_update_top]
    have hw : win u j (n + 1) = win u j n + u (j + n) * B ^ n := valF_succ _ _
    rw [hw, ← htop, pow_succ]
    zify at e hdm ⊢
    linear_combination e - (B : ℤ) ^ n * hdm
  · intro i hi
    rcases Nat.lt_succ_iff_lt_or_eq.mp (Nat.lt_succ_of_le hi) with h | h
    · rw [Function.update_of_ne (by omega)]; exact lim i h
    · subst h; rw [Function.update_self]; exact Nat.mod_lt _ (by norm_num)
  · intro m hm
    rw [Function.update_of_ne (by omega)]; exact keep m (by omega)

/-! ## Knuth D: one digit -/

/-- Step `j` of algorithm D: the estimate, its correction, multiply-subtract, and the add-back
if that borrowed. Returns the window and the digit `q[j]`. -/
def dStep (u v : ℕ → ℕ) (n j : ℕ) : (ℕ → ℕ) × ℕ :=
  let num := u (j + n) * B + u (j + n - 1)
  let qr := corr (v (n - 1)) (v (n - 2)) (u (j + n - 2)) (num / v (n - 1)) (num % v (n - 1))
  let ms := msStep u v j n qr.1
  if ms.2 ≠ 0 then (abStep ms.1 v j n, qr.1 - 1) else (ms.1, qr.1)

lemma dStep_eq (u v : ℕ → ℕ) (n j : ℕ) : dStep u v n j =
    if (msStep u v j n (corr (v (n - 1)) (v (n - 2)) (u (j + n - 2)) ((u (j + n) * B + u (j + n - 1)) / v (n - 1)) ((u (j + n) * B + u (j + n - 1)) % v (n - 1))).1).2 ≠ 0 then (abStep (msStep u v j n (corr (v (n - 1)) (v (n - 2)) (u (j + n - 2)) ((u (j + n) * B + u (j + n - 1)) / v (n - 1)) ((u (j + n) * B + u (j + n - 1)) % v (n - 1))).1).1 v j n, (corr (v (n - 1)) (v (n - 2)) (u (j + n - 2)) ((u (j + n) * B + u (j + n - 1)) / v (n - 1)) ((u (j + n) * B + u (j + n - 1)) % v (n - 1))).1 - 1)
    else ((msStep u v j n (corr (v (n - 1)) (v (n - 2)) (u (j + n - 2)) ((u (j + n) * B + u (j + n - 1)) / v (n - 1)) ((u (j + n) * B + u (j + n - 1)) % v (n - 1))).1).1, (corr (v (n - 1)) (v (n - 2)) (u (j + n - 2)) ((u (j + n) * B + u (j + n - 1)) / v (n - 1)) ((u (j + n) * B + u (j + n - 1)) % v (n - 1))).1) := rfl

/-- The two top limbs and the rest. -/
lemma valF_top2 (w : ℕ → ℕ) (n : ℕ) (hn : 2 ≤ n) :
    valF w n = w (n - 1) * B * B ^ (n - 2) + w (n - 2) * B ^ (n - 2) + valF w (n - 2) := by
  obtain ⟨k, rfl⟩ : ∃ k, n = k + 2 := ⟨n - 2, by omega⟩
  simp only [show k + 2 - 1 = k + 1 by omega, show k + 2 - 2 = k by omega]
  rw [show k + 2 = (k + 1) + 1 by ring, valF_succ, valF_succ, pow_succ]; ring

set_option maxHeartbeats 1000000 in
/-- **One digit of algorithm D.** With `v` normalised (`2·v[n−1] ≥ B`, `n ≥ 2`) and the window
`U = u[j..=j+n]` under `B·V`, the step gives `U = q[j]·V + U'` with `U' < V` and `q[j] < B`.
Only the window changes. -/
theorem dStep_spec {u v : ℕ → ℕ} {n j : ℕ} (hn : 2 ≤ n) (hv : Limbs v n) (hv1 : B ≤ 2 * v (n - 1))
    (hu : ∀ i ≤ n, u (j + i) < B) (hUV : win u j (n + 1) < B * valF v n) :
    win u j (n + 1) = (dStep u v n j).2 * valF v n + win (dStep u v n j).1 j (n + 1) ∧
      win (dStep u v n j).1 j (n + 1) < valF v n ∧ (dStep u v n j).2 < B ∧
      (∀ i ≤ n, (dStep u v n j).1 (j + i) < B) ∧ (∀ m, m < j ∨ j + n < m → (dStep u v n j).1 m = u m) := by
  -- the step's setting
  have hU : win u j (n + 1) = u (j + n) * B * B * B ^ (n - 2) + u (j + n - 1) * B * B ^ (n - 2) +
      u (j + n - 2) * B ^ (n - 2) + win u j (n - 2) := by
    unfold win
    have h1 := valF_top2 (fun i => u (j + i)) (n + 1) (by omega)
    have h2 : valF (fun i => u (j + i)) (n - 1) = valF (fun i => u (j + i)) (n - 2) + u (j + (n - 2)) * B ^ (n - 2) := by
      rw [show n - 1 = (n - 2) + 1 by omega, valF_succ]
    have h3 : B ^ (n - 1) = B ^ (n - 2) * B := by rw [show n - 1 = (n - 2) + 1 by omega, pow_succ]
    simp only [show n + 1 - 1 = n by omega, show n + 1 - 2 = n - 1 by omega] at h1
    rw [h1, h2, h3]
    simp only [show j + (n - 1) = j + n - 1 by omega, show j + (n - 2) = j + n - 2 by omega]
    ring
  have st : Step (B ^ (n - 2)) (valF v n) (win u j (n + 1)) (v (n - 1)) (v (n - 2)) (valF v (n - 2))
      (u (j + n)) (u (j + n - 1)) (u (j + n - 2)) (win u j (n - 2)) :=
    { hP := by positivity
      hV := valF_top2 v n hn
      hVl := valF_lt (fun i hi => hv i (by omega))
      hv1 := hv _ (by omega)
      hv1n := hv1
      hv2 := hv _ (by omega)
      hU := hU
      hUl := valF_lt (fun i hi => by have := hu i (by omega); exact this)
      hu1 := by have := hu (n - 1) (by omega); rwa [show j + (n - 1) = j + n - 1 by omega] at this
      hu2 := by have := hu (n - 2) (by omega); rwa [show j + (n - 2) = j + n - 2 by omega] at this
      hUV := hUV }
  have hVp := st.Vpos
  have hqB := st.q_lt
  obtain ⟨cr, cq, cq1, cB⟩ := st.corr_spec _ _ (Nat.mod_add_div' _ _) st.q_le_qhat st.qhat_le
  rw [dStep_eq]
  generalize hqr : corr (v (n - 1)) (v (n - 2)) (u (j + n - 2)) ((u (j + n) * B + u (j + n - 1)) / v (n - 1))
    ((u (j + n) * B + u (j + n - 1)) % v (n - 1)) = qr at cr cq cq1 cB
  obtain ⟨me, mf, mlim, mkeep⟩ := msStep_spec hu hv cB
  generalize hms : msStep u v j n qr.1 = ms at me mf mlim mkeep
  have hW1 : win ms.1 j (n + 1) < B ^ (n + 1) := valF_lt (fun i hi => mlim i (by omega))
  have hdm := Nat.div_add_mod' (win u j (n + 1)) (valF v n)
  have hmod := Nat.mod_lt (win u j (n + 1)) hVp
  have hqV : win u j (n + 1) / valF v n * valF v n ≤ win u j (n + 1) := Nat.div_mul_le_self (win u j (n + 1)) (valF v n)
  have hsucc : win u j (n + 1) < win u j (n + 1) / valF v n * valF v n + valF v n := by
    have := Nat.lt_mul_div_succ (win u j (n + 1)) hVp
    rw [Nat.mul_succ, mul_comm] at this; exact this
  generalize win u j (n + 1) / valF v n = q at hqB cq cq1 hdm hqV hsucc ⊢
  generalize win u j (n + 1) % valF v n = rm at hdm hmod
  split_ifs with hf
  · -- it borrowed: `qhat = q + 1`, and the add-back restores `U − qV`
    have hf1 : ms.2 = 1 := by omega
    rw [hf1, mul_one] at me
    obtain ⟨o, ho, ae, alim, akeep⟩ := abStep_spec mlim hv
    have hW2 : win (abStep ms.1 v j n) j (n + 1) < B ^ (n + 1) := valF_lt (fun i hi => alim i (by omega))
    have hgt : q < qr.1 := by
      by_contra hc; push Not at hc
      have : qr.1 * (valF v n) ≤ q * (valF v n) := Nat.mul_le_mul_right _ hc
      omega
    have hq1 : qr.1 = q + 1 := by omega
    rw [hq1, Nat.add_mul, one_mul] at me
    rw [show qr.1 - 1 = q by omega]
    generalize B ^ (n + 1) = Bn at me hW1 hW2 ae
    have ho1 : o = 1 := by
      rcases Nat.le_one_iff_eq_zero_or_eq_one.mp ho with h | h
      · subst h; omega
      · exact h
    subst ho1
    refine ⟨by dsimp only; omega, by dsimp only; omega, by dsimp only; omega, alim, fun m hm => by dsimp only; rw [akeep m hm, mkeep m hm]⟩
  · -- no borrow: `qhat = q`
    have hf0 : ms.2 = 0 := by push Not at hf; exact hf
    rw [hf0, mul_zero, add_zero] at me
    have hle : qr.1 ≤ q := by
      by_contra hc; push Not at hc
      have : (q + 1) * (valF v n) ≤ qr.1 * (valF v n) := Nat.mul_le_mul_right _ hc
      rw [Nat.add_mul, one_mul] at this
      omega
    have hq : qr.1 = q := le_antisymm hle cq
    rw [hq] at me ⊢
    refine ⟨by dsimp only; omega, by dsimp only; omega, by dsimp only; omega, mlim, mkeep⟩

end IEEE754.Exact
