/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Exact.DivRem

/-!
# Knuth D's outer loop, the normalisation, and `divrem_mag`
-/

namespace IEEE754.Exact

lemma kstep_fst (v : ℕ → ℕ) (n j : ℕ) (p : (ℕ → ℕ) × (ℕ → ℕ)) : (kstep v n j p).1 = (dStep p.1 v n j).1 := rfl
lemma kstep_snd (v : ℕ → ℕ) (n j : ℕ) (p : (ℕ → ℕ) × (ℕ → ℕ)) :
    (kstep v n j p).2 = Function.update p.2 j (dStep p.1 v n j).2 := rfl
lemma knuthRun_succ (u v : ℕ → ℕ) (n m k : ℕ) :
    knuthRun u v n m (k + 1) = kstep v n (m - (k + 1)) (knuthRun u v n m k) := rfl

/-- **The outer loop.** After `k` passes (`j = m − k`): `A = rem + V·q` is kept, the remainder
array is zero from `j + n` and below `V` read from `j`, every limb is under `2^64`, and the
digits sit at `[j, m)`. -/
theorem knuthRun_spec {u v : ℕ → ℕ} {n m : ℕ} (hn : 2 ≤ n) (hv : Limbs v n) (hv1 : B ≤ 2 * v (n - 1))
    (hu : Limbs u (m + n)) (hz : ∀ i ≥ m + n, u i = 0) (htop : win u m n < valF v n) :
    ∀ k ≤ m, valF (knuthRun u v n m k).1 (m + n) + valF v n * valF (knuthRun u v n m k).2 m = valF u (m + n) ∧
      (∀ i ≥ m - k + n, (knuthRun u v n m k).1 i = 0) ∧ win (knuthRun u v n m k).1 (m - k) n < valF v n ∧
      Limbs (knuthRun u v n m k).1 (m + n) ∧ Limbs (knuthRun u v n m k).2 m ∧
      (∀ i, i < m - k ∨ m ≤ i → (knuthRun u v n m k).2 i = 0) := by
  intro k hk
  induction k with
  | zero =>
    refine ⟨by simp [knuthRun, valF], fun i hi => hz i (by omega), by simpa [knuthRun] using htop, hu,
      fun i _ => by simp [knuthRun], fun i _ => rfl⟩
  | succ k ih =>
    obtain ⟨e, z, top, lim, qlim, qz⟩ := ih (by omega)
    have hj : m - k = (m - (k + 1)) + 1 := by omega
    rw [hj] at z top qz
    rw [knuthRun_succ, kstep_fst, kstep_snd]
    exact knuth_step (A := u) hn hv hv1 (show m - (k + 1) < m by omega) _ e z top lim qlim qz

end IEEE754.Exact
