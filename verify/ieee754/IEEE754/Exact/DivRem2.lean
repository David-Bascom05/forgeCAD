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

/-! ## The normalisation: `shl_small`, its top limb, `v[n−1] ≥ 2^63` -/

/-- `shl_small(a, s)` is `shl_into` with `bits = s < 64` (no whole limbs): `a·2^s`, length `la + 1`. -/
lemma shl_small_spec {a : ℕ → ℕ} {la s : ℕ} (ha : Limbs a la) (hs : s < 64) :
    valF (shlInto a la s) (la + 1) = valF a la * 2 ^ s ∧ Limbs (shlInto a la s) (la + 1) ∧
      ∀ i ≥ la + 1, shlInto a la s i = 0 := by
  obtain ⟨e, lim⟩ := shl_into_spec (bits := s) ha
  rw [Nat.div_eq_of_lt hs, zero_add] at e lim
  refine ⟨e, lim, fun i hi => ?_⟩
  unfold shlInto
  rw [Nat.div_eq_of_lt hs, Nat.mod_eq_of_lt hs]
  split_ifs with h0
  · simp only [zero_add]; split_ifs with h <;> [omega; rfl]
  · rw [Function.update_of_ne (by omega)]
    exact (shlRun_spec (w := 0) (r := s) ha (Nat.pos_of_ne_zero h0) hs la le_rfl).2.2.1 i (Or.inr (by omega))

/-- A limb's shift: `s = 64 − bitlen(top)` (`leading_zeros`) puts it in `[2^63, 2^64)`. -/
lemma norm_top {t : ℕ} (ht0 : 0 < t) (htB : t < B) :
    B ≤ 2 * (t * 2 ^ (64 - Nat.size t)) ∧ t * 2 ^ (64 - Nat.size t) < B ∧ 64 - Nat.size t < 64 := by
  have hs1 : t < 2 ^ Nat.size t := Nat.lt_size_self t
  have hpos0 : 1 ≤ Nat.size t := Nat.size_pos.mpr ht0
  have hs2 : 2 ^ (Nat.size t - 1) ≤ t := Nat.lt_size.mp (by omega)
  have hsz : Nat.size t ≤ 64 := by
    rw [Nat.size_le]; exact htB
  have hpos : 1 ≤ Nat.size t := Nat.size_pos.mpr ht0
  refine ⟨?_, ?_, by omega⟩
  · calc B = 2 ^ (Nat.size t - 1) * 2 ^ (64 - Nat.size t) * 2 := by
          rw [← pow_add, ← pow_succ]; show (2 : ℕ) ^ 64 = _; congr 1; omega
      _ ≤ t * 2 ^ (64 - Nat.size t) * 2 := by gcongr
      _ = 2 * (t * 2 ^ (64 - Nat.size t)) := by ring
  · calc t * 2 ^ (64 - Nat.size t) < 2 ^ Nat.size t * 2 ^ (64 - Nat.size t) :=
          Nat.mul_lt_mul_of_pos_right hs1 (by positivity)
      _ = B := by rw [← pow_add]; show _ = (2 : ℕ) ^ 64; congr 1; omega

/-- **The normalised divisor.** For trimmed `b` (`lb ≥ 1`) shifted by `s = leading_zeros(b_top)`:
the shifted top limb is 0 (so `v.pop()` drops nothing), `v = b·2^s` on `lb` limbs, and
`v[lb−1] ≥ 2^63`. -/
lemma norm_v {b : ℕ → ℕ} {lb : ℕ} (hb : Limbs b lb) (hl : 1 ≤ lb) (ht : b (lb - 1) ≠ 0) :
    let s := 64 - Nat.size (b (lb - 1))
    shlInto b lb s lb = 0 ∧ valF (shlInto b lb s) lb = valF b lb * 2 ^ s ∧
      B ≤ 2 * shlInto b lb s (lb - 1) ∧ Limbs (shlInto b lb s) lb := by
  intro s
  obtain ⟨n1', n2', n3'⟩ := norm_top (Nat.pos_of_ne_zero ht) (hb _ (by omega))
  have n1 : B ≤ 2 * (b (lb - 1) * 2 ^ s) := n1'
  have n2 : b (lb - 1) * 2 ^ s < B := n2'
  have n3 : s < 64 := n3'
  clear n1' n2' n3'
  obtain ⟨e, lim, -⟩ := shl_small_spec (s := s) hb n3
  have hP : 0 < B ^ (lb - 1) := by positivity
  have hsP : 0 < 2 ^ s := by positivity
  -- `b·2^s < B^lb`
  have hb1 : valF b lb = valF b (lb - 1) + b (lb - 1) * B ^ (lb - 1) := by
    rw [show lb = (lb - 1) + 1 by omega, valF_succ]; simp
  have hlow := valF_lt (fun i hi => hb i (by omega) : Limbs b (lb - 1))
  have hbs : valF b lb * 2 ^ s < B ^ lb := by
    have h1 : (b (lb - 1) + 1) * 2 ^ s ≤ B := by
      -- `b_top·2^s` is a multiple of `2^s` under `2^64`
      have hdvd : 2 ^ s ∣ B := by rw [B]; exact pow_dvd_pow 2 (by omega)
      obtain ⟨c, hc⟩ := hdvd
      have : b (lb - 1) < c := by
        by_contra hh; push Not at hh
        have : c * 2 ^ s ≤ b (lb - 1) * 2 ^ s := Nat.mul_le_mul_right _ hh
        rw [mul_comm c, ← hc] at this; omega
      calc (b (lb - 1) + 1) * 2 ^ s ≤ c * 2 ^ s := Nat.mul_le_mul_right _ this
        _ = B := by rw [hc, mul_comm]
    calc valF b lb * 2 ^ s < (b (lb - 1) + 1) * B ^ (lb - 1) * 2 ^ s := by
          apply Nat.mul_lt_mul_of_pos_right _ hsP; rw [hb1]; nlinarith
      _ = (b (lb - 1) + 1) * 2 ^ s * B ^ (lb - 1) := by ring
      _ ≤ B * B ^ (lb - 1) := Nat.mul_le_mul_right _ h1
      _ = B ^ lb := by rw [← pow_succ']; congr 1; omega
  -- the top limb is 0
  have htop : shlInto b lb s lb = 0 := by
    by_contra hc
    have h1 : valF (shlInto b lb s) (lb + 1) = valF (shlInto b lb s) lb + shlInto b lb s lb * B ^ lb := valF_succ _ _
    have : B ^ lb ≤ shlInto b lb s lb * B ^ lb := Nat.le_mul_of_pos_left _ (Nat.pos_of_ne_zero hc)
    omega
  have hv : valF (shlInto b lb s) lb = valF b lb * 2 ^ s := by
    have h1 : valF (shlInto b lb s) (lb + 1) = valF (shlInto b lb s) lb + shlInto b lb s lb * B ^ lb := valF_succ _ _
    rw [htop, zero_mul, add_zero] at h1; rw [← h1, e]
  refine ⟨htop, hv, ?_, fun i hi => lim i (by omega)⟩
  -- `v[lb−1] ≥ 2^63`
  have hv1 : valF (shlInto b lb s) lb = valF (shlInto b lb s) (lb - 1) + shlInto b lb s (lb - 1) * B ^ (lb - 1) := by
    rw [show lb = (lb - 1) + 1 by omega, valF_succ]; simp
  have hvlow := valF_lt (fun i hi => lim i (by omega) : Limbs (shlInto b lb s) (lb - 1))
  have hge : b (lb - 1) * 2 ^ s * B ^ (lb - 1) ≤ valF (shlInto b lb s) lb := by
    rw [hv, hb1]; nlinarith
  have : B * B ^ (lb - 1) < 2 * (shlInto b lb s (lb - 1) + 1) * B ^ (lb - 1) := by
    calc B * B ^ (lb - 1) ≤ 2 * (b (lb - 1) * 2 ^ s) * B ^ (lb - 1) := Nat.mul_le_mul_right _ n1
      _ ≤ 2 * valF (shlInto b lb s) lb := by nlinarith
      _ < 2 * (shlInto b lb s (lb - 1) + 1) * B ^ (lb - 1) := by nlinarith
  have := Nat.lt_of_mul_lt_mul_right this
  rw [B_val] at this ⊢; omega

/-! ## The remainder, shifted back -/

/-- `r[i] = (u[i] >> s) | (u[i+1] << (64 − s))` for `i < n`, the high part 0 at `i = n − 1`;
a copy when `s = 0`. -/
def rshift (U : ℕ → ℕ) (n s : ℕ) : ℕ → ℕ := fun i =>
  if i < n then
    (if s = 0 then U i else (U i / 2 ^ s) ||| (if i + 1 < n then (U (i + 1) * 2 ^ (64 - s)) % B else 0))
  else 0

lemma rshift_limb {x y s : ℕ} (hx : x < B) (hs0 : 0 < s) (hs : s < 64) :
    (x / 2 ^ s) ||| ((y * 2 ^ (64 - s)) % B) = x / 2 ^ s + 2 ^ (64 - s) * (y % 2 ^ s) ∧
      x / 2 ^ s < 2 ^ (64 - s) := by
  have hB : B = 2 ^ (64 - s) * 2 ^ s := by rw [← pow_add]; show (2 : ℕ) ^ 64 = _; congr 1; omega
  have hlt : x / 2 ^ s < 2 ^ (64 - s) := by
    rw [Nat.div_lt_iff_lt_mul (by positivity), ← hB]; exact hx
  have hm : (y * 2 ^ (64 - s)) % B = 2 ^ (64 - s) * (y % 2 ^ s) := by
    rw [hB]
    conv_lhs => rw [← Nat.div_add_mod y (2 ^ s)]
    rw [show (2 ^ s * (y / 2 ^ s) + y % 2 ^ s) * 2 ^ (64 - s) =
      2 ^ (64 - s) * (y % 2 ^ s) + 2 ^ (64 - s) * 2 ^ s * (y / 2 ^ s) by ring, Nat.add_mul_mod_self_left]
    exact Nat.mod_eq_of_lt (Nat.mul_lt_mul_of_pos_left (Nat.mod_lt _ (by positivity)) (by positivity))
  refine ⟨?_, hlt⟩
  rw [hm, Nat.lor_comm, ← Nat.two_pow_add_eq_or_of_lt hlt, add_comm]

/-- **The shift back.** `r·2^s + (u[0] mod 2^s) = u` on `n` limbs. -/
theorem rshift_spec {U : ℕ → ℕ} {n s : ℕ} (hU : Limbs U n) (hs : s < 64) :
    valF (rshift U n s) n * 2 ^ s + U 0 % 2 ^ s * (if n = 0 then 0 else 1) = valF U n ∧ Limbs (rshift U n s) n := by
  by_cases hs0 : s = 0
  · subst hs0
    refine ⟨?_, fun i hi => by simp only [rshift, hi, if_true]; exact hU i hi⟩
    simp only [pow_zero, mul_one, Nat.mod_one, zero_mul, add_zero]
    exact valF_congr (fun i hi => by simp [rshift, hi])
  -- the prefix sums: `Σ_{i<k} r_i B^i 2^s + u_0 mod 2^s = valF u k + (u'_k mod 2^s)·B^k`, `u'` zero past `n`
  have key : ∀ k ≤ n, 0 < n → valF (rshift U n s) k * 2 ^ s + U 0 % 2 ^ s =
      valF U k + ((if k < n then U k else 0) % 2 ^ s) * B ^ k := by
    intro k hk hn
    induction k with
    | zero => simp [valF, hn]
    | succ k ih =>
      have e := ih (by omega)
      have hUk : U k < B := hU k (by omega)
      obtain ⟨l1, -⟩ := rshift_limb (y := if k + 1 < n then U (k + 1) else 0) hUk (Nat.pos_of_ne_zero hs0) hs
      have hr : rshift U n s k = U k / 2 ^ s + 2 ^ (64 - s) * ((if k + 1 < n then U (k + 1) else 0) % 2 ^ s) := by
        simp only [rshift, show k < n by omega, if_true, hs0, if_false]
        rw [← l1]; congr 1
        split_ifs <;> simp
      rw [valF_succ, valF_succ, Nat.add_mul, hr]
      simp only [show k < n by omega, if_true] at e
      have hB : B = 2 ^ (64 - s) * 2 ^ s := by rw [← pow_add]; show (2 : ℕ) ^ 64 = _; congr 1; omega
      have hdm := Nat.div_add_mod (U k) (2 ^ s)
      rw [pow_succ, hB]
      rw [hB] at e
      zify at e hdm ⊢
      linear_combination e + ((2 : ℤ) ^ (64 - s) * 2 ^ s) ^ k * hdm
  refine ⟨?_, ?_⟩
  · by_cases hn : n = 0
    · subst hn; simp [valF]
    · have := key n le_rfl (Nat.pos_of_ne_zero hn)
      simp only [lt_irrefl, if_false, Nat.zero_mod, zero_mul, add_zero] at this
      rw [if_neg hn, mul_one]; exact this
  · intro i hi
    simp only [rshift, hi, if_true, hs0, if_false]
    have hUi := hU i hi
    obtain ⟨l1, l2⟩ := rshift_limb (y := if i + 1 < n then U (i + 1) else 0) hUi (Nat.pos_of_ne_zero hs0) hs
    have : (if i + 1 < n then (U (i + 1) * 2 ^ (64 - s)) % B else 0) =
        ((if i + 1 < n then U (i + 1) else 0) * 2 ^ (64 - s)) % B := by split_ifs <;> simp
    rw [this, l1]
    have hB : B = 2 ^ (64 - s) * 2 ^ s := by rw [← pow_add]; show (2 : ℕ) ^ 64 = _; congr 1; omega
    have : (if i + 1 < n then U (i + 1) else 0) % 2 ^ s < 2 ^ s := Nat.mod_lt _ (by positivity)
    have : 2 ^ (64 - s) * ((if i + 1 < n then U (i + 1) else 0) % 2 ^ s + 1) ≤ 2 ^ (64 - s) * 2 ^ s :=
      Nat.mul_le_mul_left _ this
    rw [hB]; nlinarith

end IEEE754.Exact
