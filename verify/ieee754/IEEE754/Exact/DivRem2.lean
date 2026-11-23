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

/-! ## The Knuth path of `divrem_mag` -/

/-- Knuth's path: shift both, run the loop, shift the remainder back. -/
def knuthDiv (a : ℕ → ℕ) (la : ℕ) (b : ℕ → ℕ) (lb : ℕ) : (ℕ → ℕ) × (ℕ → ℕ) :=
  let s := 64 - Nat.size (b (lb - 1))
  let res := knuthRun (shlInto a la s) (shlInto b lb s) lb (la + 1 - lb) (la + 1 - lb)
  (res.2, rshift res.1 lb s)

lemma valF_ge_pow {w : ℕ → ℕ} {n : ℕ} (hn : 1 ≤ n) (ht : w (n - 1) ≠ 0) : B ^ (n - 1) ≤ valF w n := by
  have := valF_ge_top (f := w) (show 0 < n by omega)
  have : B ^ (n - 1) ≤ w (n - 1) * B ^ (n - 1) := Nat.le_mul_of_pos_left _ (Nat.pos_of_ne_zero ht)
  omega

set_option maxHeartbeats 1000000 in
/-- **Knuth's path.** For trimmed `a ≥ b` with `b` of at least two limbs: `a = q·b + r`, `r < b`,
`q` on `la + 1 − lb` limbs and `r` on `lb`, every limb under `2^64`. -/
theorem knuthDiv_spec {a b : ℕ → ℕ} {la lb : ℕ} (ha : Limbs a la) (hb : Limbs b lb) (hl : 2 ≤ lb)
    (ht : b (lb - 1) ≠ 0) (hle : lb ≤ la) :
    valF a la = valF (knuthDiv a la b lb).1 (la + 1 - lb) * valF b lb + valF (knuthDiv a la b lb).2 lb ∧
      valF (knuthDiv a la b lb).2 lb < valF b lb ∧
      Limbs (knuthDiv a la b lb).1 (la + 1 - lb) ∧ Limbs (knuthDiv a la b lb).2 lb := by
  unfold knuthDiv
  simp only
  set s := 64 - Nat.size (b (lb - 1)) with hsd
  set m := la + 1 - lb with hm
  have hmn : m + lb = la + 1 := by omega
  obtain ⟨-, n2', n3'⟩ := norm_top (Nat.pos_of_ne_zero ht) (hb _ (by omega))
  have n3 : s < 64 := n3'
  obtain ⟨vtop, vval, vnorm, vlim⟩ := norm_v hb (by omega) ht
  obtain ⟨ue, ulim, uz⟩ := shl_small_spec (s := s) ha n3
  -- the first window is under `V`
  have hV : valF (shlInto b lb s) lb = valF b lb * 2 ^ s := vval
  have hbpow := valF_ge_pow (w := b) (n := lb) (by omega) ht
  have halt := valF_lt ha
  have htop : win (shlInto a la s) m lb < valF (shlInto b lb s) lb := by
    have hsp := valF_split (shlInto a la s) m lb
    rw [hmn, ue] at hsp
    have h1 : B ^ m * win (shlInto a la s) m lb ≤ valF a la * 2 ^ s := by omega
    have h2 : valF a la * 2 ^ s < B ^ m * valF (shlInto b lb s) lb := by
      rw [hV]
      have : B ^ la ≤ B ^ m * valF b lb := by
        calc B ^ la = B ^ m * B ^ (lb - 1) := by rw [← pow_add]; congr 1; omega
          _ ≤ B ^ m * valF b lb := Nat.mul_le_mul_left _ hbpow
      calc valF a la * 2 ^ s < B ^ la * 2 ^ s := Nat.mul_lt_mul_of_pos_right halt (by positivity)
        _ ≤ B ^ m * valF b lb * 2 ^ s := Nat.mul_le_mul_right _ this
        _ = B ^ m * (valF b lb * 2 ^ s) := by ring
    have h3 : B ^ m * win (shlInto a la s) m lb < B ^ m * valF (shlInto b lb s) lb := lt_of_le_of_lt h1 h2
    exact Nat.lt_of_mul_lt_mul_left h3
  have hz : ∀ i ≥ m + lb, shlInto a la s i = 0 := fun i hi => uz i (by omega)
  have hu : Limbs (shlInto a la s) (m + lb) := by rw [hmn]; exact ulim
  obtain ⟨ke, kz, ktop, klim, kqlim, -⟩ := knuthRun_spec hl vlim vnorm hu hz htop m le_rfl
  generalize hres : knuthRun (shlInto a la s) (shlInto b lb s) lb m m = res at ke kz ktop klim kqlim
  simp only [Nat.sub_self, zero_add] at kz ktop
  -- the remainder array is `R` on `lb` limbs
  have hR : valF res.1 (m + lb) = valF res.1 lb := by
    have hsp := valF_split res.1 lb m
    have : win res.1 lb m = 0 := by
      unfold win valF; exact Finset.sum_eq_zero (fun i _ => by simp only [kz (lb + i) (by omega), zero_mul])
    rw [show m + lb = lb + m by ring, hsp, this, mul_zero, add_zero]
  have hwin0 : win res.1 0 lb = valF res.1 lb := valF_congr (fun i _ => by simp)
  rw [hwin0] at ktop
  rw [hR, hmn, ue, hV] at ke
  -- `R = 2^s (a − q b)`, so `u[0] mod 2^s = 0`
  obtain ⟨re, rlim⟩ := rshift_spec (U := res.1) (n := lb) (s := s) (fun i hi => klim i (by omega)) n3
  rw [if_neg (by omega), mul_one] at re
  have hdiv : 2 ^ s ∣ valF res.1 lb := by
    have : valF res.1 lb + valF b lb * 2 ^ s * valF res.2 m = valF a la * 2 ^ s := ke
    have h1 : 2 ^ s ∣ valF a la * 2 ^ s := dvd_mul_left _ _
    have h2 : 2 ^ s ∣ valF b lb * 2 ^ s * valF res.2 m := ⟨valF b lb * valF res.2 m, by ring⟩
    have := (Nat.dvd_add_right h2).mp (by rw [add_comm, this]; exact h1)
    exact this
  have hmod0 : res.1 0 % 2 ^ s = 0 := by
    have hsp := valF_split res.1 1 (lb - 1)
    rw [show 1 + (lb - 1) = lb by omega] at hsp
    have h1 : valF res.1 1 = res.1 0 := by simp [valF]
    rw [h1] at hsp
    have h2 : 2 ^ s ∣ B ^ 1 := by rw [pow_one, B]; exact pow_dvd_pow 2 (by omega)
    have h3 : 2 ^ s ∣ B ^ 1 * win res.1 1 (lb - 1) := Dvd.dvd.mul_right h2 _
    rw [hsp] at hdiv
    exact Nat.mod_eq_zero_of_dvd ((Nat.dvd_add_left h3).mp hdiv)
  rw [hmod0, add_zero] at re
  have hsP : 0 < 2 ^ s := by positivity
  refine ⟨?_, ?_, kqlim, rlim⟩
  · -- `(q b + r)·2^s = a·2^s`
    have : (valF res.2 m * valF b lb + valF (rshift res.1 lb s) lb) * 2 ^ s = valF a la * 2 ^ s := by
      rw [← ke, ← re]; ring
    exact (Nat.eq_of_mul_eq_mul_right hsP this).symm
  · have : valF (rshift res.1 lb s) lb * 2 ^ s < valF b lb * 2 ^ s := by rw [re, ← hV]; exact ktop
    exact Nat.lt_of_mul_lt_mul_right this

/-! ## `divrem_mag` and `Big::divrem` -/

/-- `divrem_mag(a, b)`: `(q, |q|, r, |r|)`. -/
def divremMag (a : ℕ → ℕ) (la : ℕ) (b : ℕ → ℕ) (lb : ℕ) : (ℕ → ℕ) × ℕ × (ℕ → ℕ) × ℕ :=
  if cmpMag a la b lb = .lt then (fun _ => 0, 0, a, la)
  else if lb = 1 then
    ((shortRun a la (b 0) la).1, la, fun i => if i = 0 then (shortRun a la (b 0) la).2 else 0, 1)
  else ((knuthDiv a la b lb).1, la + 1 - lb, (knuthDiv a la b lb).2, lb)

lemma len_le_of_le {a b : ℕ → ℕ} {la lb : ℕ} (ha : Limbs a la) (hl : 1 ≤ lb) (ht : b (lb - 1) ≠ 0)
    (h : valF b lb ≤ valF a la) : lb ≤ la := by
  by_contra hc; push Not at hc
  have h1 := valF_lt ha
  have h2 := valF_ge_pow (w := b) hl ht
  have h3 : B ^ la ≤ B ^ (lb - 1) := Nat.pow_le_pow_right (by norm_num) (by omega)
  omega

/-- **`divrem_mag`.** For trimmed magnitudes with `b ≠ 0`: `a = q·b + r`, `r < b`. -/
theorem divremMag_spec {a b : ℕ → ℕ} {la lb : ℕ} (ha : Limbs a la) (hb : Limbs b lb)
    (ta : la = 0 ∨ a (la - 1) ≠ 0) (hl : 1 ≤ lb) (ht : b (lb - 1) ≠ 0) :
    valF a la = valF (divremMag a la b lb).1 (divremMag a la b lb).2.1 * valF b lb +
        valF (divremMag a la b lb).2.2.1 (divremMag a la b lb).2.2.2 ∧
      valF (divremMag a la b lb).2.2.1 (divremMag a la b lb).2.2.2 < valF b lb ∧
      Limbs (divremMag a la b lb).1 (divremMag a la b lb).2.1 ∧
      Limbs (divremMag a la b lb).2.2.1 (divremMag a la b lb).2.2.2 := by
  have cm := cmp_mag_spec ha hb ta (Or.inr ht)
  unfold divremMag
  split_ifs with h1 h2
  · rw [h1] at cm
    have := compare_lt_iff_lt.mp cm.symm
    exact ⟨by simp [valF], this, fun i hi => absurd hi (Nat.not_lt_zero _), ha⟩
  · have hle : valF b lb ≤ valF a la := by
      by_contra hc; push Not at hc; exact h1 (by rw [cm]; exact compare_lt_iff_lt.mpr hc)
    subst h2
    have hd0 : 0 < b 0 := Nat.pos_of_ne_zero ht
    obtain ⟨e, rd, lim⟩ := shortDiv_spec ha hd0 (hb 0 (by omega))
    have hb1 : valF b 1 = b 0 := by simp [valF]
    have hr1 : valF (fun i => if i = 0 then (shortRun a la (b 0) la).2 else 0) 1 = (shortRun a la (b 0) la).2 := by
      simp [valF]
    simp only
    rw [hb1, hr1]
    refine ⟨e.symm, rd, lim, fun i hi => ?_⟩
    have : i = 0 := by omega
    subst this; simp only [if_true]; exact lt_trans rd (hb 0 (by omega))
  · have hle : valF b lb ≤ valF a la := by
      by_contra hc; push Not at hc; exact h1 (by rw [cm]; exact compare_lt_iff_lt.mpr hc)
    have hl2 : 2 ≤ lb := by omega
    exact knuthDiv_spec ha hb hl2 ht (len_le_of_le ha hl ht hle)

namespace Big

/-- `Big::divrem`: `o = 0` gives `(0, self)`; two inline values divide in i128 (`/`, `%` truncate);
otherwise `divrem_mag` on the magnitudes, `q` with the product sign and `r` with `self`'s. -/
def divrem (x y : Big) : Big × Big :=
  if isZero y then (S 0, x) else
    match x, y with
    | S a, S b => (S (Int.tdiv a b), S (Int.tmod a b))
    | _, _ =>
      let d := divremMag (view x).2.1 (view x).2.2 (view y).2.1 (view y).2.2
      (fromMag ((view x).1 != (view y).1) d.1 d.2.1, fromMag (view x).1 d.2.2.1 d.2.2.2)

/-- The truncated division's laws: `x = q·y + r`, `|r| < |y|`, and `r` takes `x`'s sign (or is 0). -/
def TruncDiv (x y q r : ℤ) : Prop :=
  x = q * y + r ∧ |r| < |y| ∧ (0 ≤ x → 0 ≤ r) ∧ (x ≤ 0 → r ≤ 0)

lemma truncDiv_int {a b : ℤ} (hb : b ≠ 0) : TruncDiv a b (Int.tdiv a b) (Int.tmod a b) := by
  refine ⟨by have := Int.tmod_add_tdiv_mul a b; linarith, ?_, fun h => Int.tmod_nonneg b h, fun h => ?_⟩
  · rw [Int.abs_eq_natAbs, Int.abs_eq_natAbs, Int.natAbs_tmod]
    exact_mod_cast Nat.mod_lt _ (Int.natAbs_pos.mpr hb)
  · have := Int.tmod_nonneg b (show 0 ≤ -a by omega)
    rw [Int.neg_tmod] at this; omega

lemma abs_sgnz (n : Bool) (a : ℕ) : |sgnz n a| = (a : ℤ) := by
  cases n <;> simp [sgnz]

lemma view_pos {x : Big} (hx : Canon x) (h0 : val x ≠ 0) : 0 < valF (view x).2.1 (view x).2.2 := by
  obtain ⟨-, -, ex, -⟩ := view_spec hx
  by_contra hc; push Not at hc
  have : valF (view x).2.1 (view x).2.2 = 0 := by omega
  rw [ex, this] at h0; cases (view x).1 <;> simp [sgnz] at h0

set_option maxHeartbeats 1000000 in
/-- **`Big::divrem`.** Both results are canonical. For `y ≠ 0` they are the truncated quotient
and remainder; for `y = 0`, `(0, x)`. -/
theorem divrem_spec {x y : Big} (hx : Canon x) (hy : Canon y) :
    Canon (divrem x y).1 ∧ Canon (divrem x y).2 ∧
      (val y ≠ 0 → TruncDiv (val x) (val y) (val (divrem x y).1) (val (divrem x y).2)) ∧
      (val y = 0 → val (divrem x y).1 = 0 ∧ val (divrem x y).2 = val x) := by
  by_cases hz : isZero y = true
  · have hy0 := (isZero_spec hy).mp hz
    unfold divrem; rw [if_pos hz]
    exact ⟨canon_zero, hx, fun h => absurd hy0 h, fun _ => ⟨rfl, rfl⟩⟩
  have hy0 : val y ≠ 0 := fun h => hz ((isZero_spec hy).mpr h)
  -- the general path
  have slow : Canon (fromMag ((view x).1 != (view y).1) (divremMag (view x).2.1 (view x).2.2 (view y).2.1 (view y).2.2).1
        (divremMag (view x).2.1 (view x).2.2 (view y).2.1 (view y).2.2).2.1) ∧
      Canon (fromMag (view x).1 (divremMag (view x).2.1 (view x).2.2 (view y).2.1 (view y).2.2).2.2.1
        (divremMag (view x).2.1 (view x).2.2 (view y).2.1 (view y).2.2).2.2.2) ∧
      TruncDiv (val x) (val y)
        (val (fromMag ((view x).1 != (view y).1) (divremMag (view x).2.1 (view x).2.2 (view y).2.1 (view y).2.2).1
          (divremMag (view x).2.1 (view x).2.2 (view y).2.1 (view y).2.2).2.1))
        (val (fromMag (view x).1 (divremMag (view x).2.1 (view x).2.2 (view y).2.1 (view y).2.2).2.2.1
          (divremMag (view x).2.1 (view x).2.2 (view y).2.1 (view y).2.2).2.2.2)) := by
    obtain ⟨lx, tx, ex, -⟩ := view_spec hx
    obtain ⟨ly, ty, ey, -⟩ := view_spec hy
    have ypos := view_pos hy hy0
    have hly : 1 ≤ (view y).2.2 := by
      rcases ty with h | h
      · rw [h] at ypos; simp [valF] at ypos
      · by_contra hc; push Not at hc
        have : (view y).2.2 = 0 := by omega
        rw [this] at ypos; simp [valF] at ypos
    have hty : (view y).2.1 ((view y).2.2 - 1) ≠ 0 := by
      rcases ty with h | h
      · omega
      · exact h
    obtain ⟨de, dlt, dql, drl⟩ := divremMag_spec lx ly tx hly hty
    generalize divremMag (view x).2.1 (view x).2.2 (view y).2.1 (view y).2.2 = d at de dlt dql drl ⊢
    obtain ⟨cq, vq⟩ := fromMag_spec (neg := ((view x).1 != (view y).1)) dql
    obtain ⟨cr, vr⟩ := fromMag_spec (neg := (view x).1) drl
    refine ⟨cq, cr, ?_⟩
    rw [vq, vr, ex, ey]
    generalize (view x).1 = an at *
    generalize (view y).1 = bn at *
    generalize valF (view x).2.1 (view x).2.2 = A at de ypos ⊢
    generalize valF (view y).2.1 (view y).2.2 = Bv at de dlt ypos ⊢
    generalize valF d.1 d.2.1 = Q at de ⊢
    generalize valF d.2.2.1 d.2.2.2 = R at de dlt ⊢
    subst de
    have hQB : (0 : ℤ) ≤ (Q : ℤ) * (Bv : ℤ) := by positivity
    refine ⟨?_, ?_, ?_, ?_⟩
    · cases an <;> cases bn <;> simp [sgnz] <;> push_cast <;> ring
    · rw [abs_sgnz, abs_sgnz]; exact_mod_cast dlt
    · intro h; cases an <;> simp only [sgnz, Bool.false_eq_true, ite_false, ite_true] at h ⊢ <;> push_cast at h ⊢ <;> omega
    · intro h; cases an <;> simp only [sgnz, Bool.false_eq_true, ite_false, ite_true] at h ⊢ <;> push_cast at h ⊢ <;> omega
  unfold divrem; rw [if_neg hz]
  cases x with
  | S a =>
    cases y with
    | S b =>
      simp only
      have hb0 : b ≠ 0 := by simpa [val] using hy0
      refine ⟨?_, ?_, fun _ => truncDiv_int hb0, fun h => absurd h hy0⟩
      · simp only [Canon] at hx ⊢
        calc |Int.tdiv a b| = (Int.natAbs (Int.tdiv a b) : ℤ) := Int.abs_eq_natAbs _
          _ ≤ (Int.natAbs a : ℤ) := by exact_mod_cast Int.natAbs_tdiv_le_natAbs a b
          _ = |a| := (Int.abs_eq_natAbs _).symm
          _ < 2 ^ 126 := hx
      · simp only [Canon] at hx ⊢
        calc |Int.tmod a b| = (Int.natAbs (Int.tmod a b) : ℤ) := Int.abs_eq_natAbs _
          _ ≤ (Int.natAbs a : ℤ) := by rw [Int.natAbs_tmod]; exact_mod_cast Nat.mod_le _ _
          _ = |a| := (Int.abs_eq_natAbs _).symm
          _ < 2 ^ 126 := hx
    | M n k w => exact ⟨slow.1, slow.2.1, fun _ => slow.2.2, fun h => absurd h hy0⟩
    | L n k w => exact ⟨slow.1, slow.2.1, fun _ => slow.2.2, fun h => absurd h hy0⟩
  | M n k w => exact ⟨slow.1, slow.2.1, fun _ => slow.2.2, fun h => absurd h hy0⟩
  | L n k w => exact ⟨slow.1, slow.2.1, fun _ => slow.2.2, fun h => absurd h hy0⟩

end Big

end IEEE754.Exact
