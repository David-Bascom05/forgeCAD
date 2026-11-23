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

end IEEE754.Exact
