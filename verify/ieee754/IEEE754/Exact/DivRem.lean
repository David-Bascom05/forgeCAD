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

end IEEE754.Exact
