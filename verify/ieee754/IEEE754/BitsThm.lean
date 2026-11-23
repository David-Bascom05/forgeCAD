/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Bits
import IEEE754.Round

/-!
# The encoding round-trips

* `ofBits_toBits`: decoding an encoding gives the datum back (NaN included);
* `toBits_ofBits`: encoding a decoded non-NaN pattern below `2^width` gives the pattern back;
* `toBits_lt`: encodings fit in the width.
-/

set_option linter.unusedSectionVars false

namespace IEEE754

namespace FP

variable {f : Format}

/-- The fields of an encoding: sign `sb`, biased exponent `c`, fraction `a`. -/
def enc (f : Format) (sb c a : ℕ) : ℕ := sb * (2 ^ f.ebits * 2 ^ (f.prec - 1)) + c * 2 ^ (f.prec - 1) + a

lemma signBit_eq : signBit f = 2 ^ f.ebits * 2 ^ (f.prec - 1) := by
  unfold signBit; rw [pow_add]

lemma enc_fields {sb c a : ℕ} (ha : a < 2 ^ (f.prec - 1)) (hc : c < 2 ^ f.ebits) :
    enc f sb c a % 2 ^ (f.prec - 1) = a ∧ enc f sb c a / 2 ^ (f.prec - 1) % 2 ^ f.ebits = c ∧
      enc f sb c a / signBit f = sb := by
  set F := 2 ^ (f.prec - 1)
  set W := 2 ^ f.ebits
  have hF : 0 < F := by positivity
  have hW : 0 < W := by positivity
  have h1 : enc f sb c a = a + F * (c + W * sb) := by unfold enc; ring
  have h2 : enc f sb c a / F = c + W * sb := by
    rw [h1, Nat.add_mul_div_left _ _ hF, Nat.div_eq_of_lt ha, zero_add]
  refine ⟨?_, ?_, ?_⟩
  · rw [h1, Nat.add_mul_mod_self_left, Nat.mod_eq_of_lt ha]
  · rw [h2, Nat.add_mul_mod_self_left, Nat.mod_eq_of_lt hc]
  · rw [signBit_eq, show W * F = F * W by ring, ← Nat.div_div_eq_div_mul, h2,
      Nat.add_mul_div_left _ _ hW, Nat.div_eq_of_lt hc, zero_add]

lemma two_emax : 2 * f.emax = 2 ^ f.ebits - 2 := by
  unfold Format.emax
  have h := f.two_le_ebits
  have : (2 : ℤ) ^ f.ebits = 2 * 2 ^ (f.ebits - 1) := by
    rw [← pow_succ']; congr 1; omega
  rw [this]; ring

lemma frac_lt_pow {m : ℕ} (hm : m < 2 ^ f.prec) : m - 2 ^ (f.prec - 1) < 2 ^ (f.prec - 1) := by
  have : 2 ^ f.prec = 2 * 2 ^ (f.prec - 1) := by
    rw [← pow_succ']; congr 1; have := f.two_le_prec; omega
  omega

lemma ebits_four : 4 ≤ 2 ^ f.ebits := by
  calc 4 = 2 ^ 2 := by norm_num
    _ ≤ 2 ^ f.ebits := Nat.pow_le_pow_right (by norm_num) f.two_le_ebits

lemma nan_frac_lt : 2 ^ (f.prec - 2) < 2 ^ (f.prec - 1) :=
  Nat.pow_lt_pow_right (by norm_num) (by have := f.two_le_prec; omega)

/-- The biased exponent of a normal datum. -/
lemma bexp_normal {m : ℕ} {e : ℤ} (h : Canon f m e) :
    1 ≤ (e - f.qmin + 1).toNat ∧ (e - f.qmin + 1).toNat < 2 ^ f.ebits - 1 := by
  obtain ⟨_, _, h1, h2, _⟩ := h
  have := two_emax (f := f)
  have h4 := ebits_four (f := f)
  have : f.qmax - f.qmin + 1 = 2 * f.emax := by unfold Format.qmax Format.qmin Format.emin; ring
  have : ((2 ^ f.ebits : ℕ) : ℤ) = 2 ^ f.ebits := by push_cast; rfl
  omega

lemma toBits_enc (x : FP f) : ∃ sb c a, sb < 2 ∧ c < 2 ^ f.ebits ∧ a < 2 ^ (f.prec - 1) ∧
    x.toBits = enc f sb c a := by
  have h4 := ebits_four (f := f)
  have hn := nan_frac_lt (f := f)
  cases x with
  | zero s => exact ⟨if s then 1 else 0, 0, 0, by split <;> omega, by omega, by positivity,
      by cases s <;> simp [toBits, enc, signBit_eq]⟩
  | inf s => exact ⟨if s then 1 else 0, 2 ^ f.ebits - 1, 0, by split <;> omega, by omega,
      by positivity, by cases s <;> simp [toBits, enc, signBit_eq, expOnes]⟩
  | nan => exact ⟨0, 2 ^ f.ebits - 1, 2 ^ (f.prec - 2), by omega, by omega, hn,
      by simp [toBits, enc, expOnes]⟩
  | fin s m e h =>
    by_cases hm : m < 2 ^ (f.prec - 1)
    · exact ⟨if s then 1 else 0, 0, m, by split <;> omega, by omega, hm,
        by cases s <;> simp [toBits, enc, signBit_eq, hm]⟩
    · have hb := bexp_normal h
      exact ⟨if s then 1 else 0, (e - f.qmin + 1).toNat, m - 2 ^ (f.prec - 1),
        by split <;> omega, by omega, frac_lt_pow h.2.1,
        by cases s <;> simp [toBits, enc, signBit_eq, hm] <;> ring⟩

theorem toBits_lt (x : FP f) : x.toBits < 2 ^ f.width := by
  obtain ⟨sb, c, a, hsb, hc, ha, hx⟩ := toBits_enc x
  rw [hx]
  unfold enc Format.width
  rw [pow_add, pow_add, pow_one]
  set F := 2 ^ (f.prec - 1); set W := 2 ^ f.ebits
  have h1 : c * F + a < W * F := by nlinarith
  have h2 : sb * (W * F) ≤ 1 * (W * F) := Nat.mul_le_mul_right _ (by omega)
  have h3 : 2 * W * F = W * F + W * F := by ring
  linarith

theorem ofBits_toBits (x : FP f) : ofBits (toBits x) = x := by
  have h4 := ebits_four (f := f)
  have hn := nan_frac_lt (f := f)
  have hF : 0 < 2 ^ (f.prec - 1) := by positivity
  cases x with
  | zero s =>
    have e := enc_fields (f := f) (sb := if s then 1 else 0) (c := 0) (a := 0) hF (by omega)
    have : (zero s : FP f).toBits = enc f (if s then 1 else 0) 0 0 := by
      cases s <;> simp [toBits, enc, signBit_eq]
    rw [this]; unfold ofBits; rw [e.1, e.2.1, e.2.2]; cases s <;> simp
  | inf s =>
    have e := enc_fields (f := f) (sb := if s then 1 else 0) (c := 2 ^ f.ebits - 1) (a := 0) hF (by omega)
    have : (inf s : FP f).toBits = enc f (if s then 1 else 0) (2 ^ f.ebits - 1) 0 := by
      cases s <;> simp [toBits, enc, signBit_eq, expOnes]
    rw [this]; unfold ofBits; rw [e.1, e.2.1, e.2.2]
    cases s <;> simp [show 2 ^ f.ebits - 1 ≠ 0 by omega]
  | nan =>
    have e := enc_fields (f := f) (sb := 0) (c := 2 ^ f.ebits - 1) (a := 2 ^ (f.prec - 2)) hn (by omega)
    have : (nan : FP f).toBits = enc f 0 (2 ^ f.ebits - 1) (2 ^ (f.prec - 2)) := by
      simp [toBits, enc, expOnes]
    rw [this]; unfold ofBits; rw [e.1, e.2.1, e.2.2]
    simp [show 2 ^ f.ebits - 1 ≠ 0 by omega]
  | fin s m e h =>
    by_cases hm : m < 2 ^ (f.prec - 1)
    · have he : e = f.qmin := by
        rcases h.2.2.2.2 with h' | h'
        · omega
        · exact h'
      subst he
      have ef := enc_fields (f := f) (sb := if s then 1 else 0) (c := 0) (a := m) hm (by omega)
      have : (fin s m f.qmin h : FP f).toBits = enc f (if s then 1 else 0) 0 m := by
        cases s <;> simp [toBits, enc, signBit_eq, hm]
      rw [this]; unfold ofBits; rw [ef.1, ef.2.1, ef.2.2]
      cases s <;> simp [show m ≠ 0 by have := h.1; omega, h]
    · have hb := bexp_normal h
      have hfr := frac_lt_pow h.2.1
      have ef := enc_fields (f := f) (sb := if s then 1 else 0) (c := (e - f.qmin + 1).toNat)
        (a := m - 2 ^ (f.prec - 1)) hfr (by omega)
      have : (fin s m e h : FP f).toBits =
          enc f (if s then 1 else 0) (e - f.qmin + 1).toNat (m - 2 ^ (f.prec - 1)) := by
        cases s <;> simp [toBits, enc, signBit_eq, hm] <;> ring
      rw [this]; unfold ofBits; rw [ef.1, ef.2.1, ef.2.2]
      have hm' : 2 ^ (f.prec - 1) + (m - 2 ^ (f.prec - 1)) = m := by omega
      have he' : ((e - f.qmin + 1).toNat : ℤ) + f.qmin - 1 = e := by
        rw [Int.toNat_of_nonneg (by have := h.2.2.1; omega)]; ring
      simp only [show (e - f.qmin + 1).toNat ≠ 0 by omega,
        show (e - f.qmin + 1).toNat ≠ 2 ^ f.ebits - 1 by omega, ite_false, hm', he']
      rw [dif_pos h]; cases s <;> rfl

theorem toBits_ofBits {b : ℕ} (hb : b < 2 ^ f.width) (hnan : (ofBits (f := f) b).isNaN = false) :
    toBits (ofBits (f := f) b) = b := by
  have hF : 0 < 2 ^ (f.prec - 1) := by positivity
  have h4 := ebits_four (f := f)
  have hpF : 2 ^ f.prec = 2 * 2 ^ (f.prec - 1) := by
    rw [← pow_succ']; congr 1; have := f.two_le_prec; omega
  have hsb : b / signBit f < 2 := by
    rw [Nat.div_lt_iff_lt_mul (by rw [signBit_eq]; positivity), signBit_eq]
    unfold Format.width at hb; rw [pow_add, pow_add, pow_one] at hb; linarith
  have hc : b / 2 ^ (f.prec - 1) % 2 ^ f.ebits < 2 ^ f.ebits := Nat.mod_lt _ (by positivity)
  have ha : b % 2 ^ (f.prec - 1) < 2 ^ (f.prec - 1) := Nat.mod_lt _ hF
  have h1 := Nat.div_add_mod b (2 ^ (f.prec - 1))
  have h2 := Nat.div_add_mod (b / 2 ^ (f.prec - 1)) (2 ^ f.ebits)
  have h3 : b / 2 ^ (f.prec - 1) / 2 ^ f.ebits = b / signBit f := by
    rw [signBit_eq, Nat.div_div_eq_div_mul, mul_comm]
  rw [h3] at h2
  generalize b / signBit f = sb at hsb h2
  generalize b / 2 ^ (f.prec - 1) % 2 ^ f.ebits = c at hc h2
  generalize b % 2 ^ (f.prec - 1) = a at ha h1
  rw [← h2] at h1
  have hbe : b = enc f sb c a := by unfold enc; linarith
  have ef := enc_fields (f := f) (sb := sb) ha hc
  have hsign : (if (sb % 2 == 1) = true then signBit f else 0) =
      sb * (2 ^ f.ebits * 2 ^ (f.prec - 1)) := by
    rcases (by omega : sb = 0 ∨ sb = 1) with h | h <;> subst h <;> simp [signBit_eq]
  rw [hbe] at hnan ⊢
  revert hnan
  unfold ofBits
  rw [ef.1, ef.2.1, ef.2.2]
  intro hnan
  dsimp only at hnan ⊢
  split_ifs at hnan ⊢ with hc0 ha0 hcan hcw ha0' hcan'
  · simp only [toBits]; rw [hsign, hc0, ha0]; unfold enc; ring
  · simp only [toBits, ha, ite_true]; rw [hsign, hc0]; unfold enc; ring
  · exfalso; apply hcan
    exact ⟨by omega, by omega, le_rfl, f.qmin_le_qmax, Or.inr rfl⟩
  · simp only [toBits, expOnes]; rw [hsign, hcw, ha0']; unfold enc; ring
  · simp [isNaN] at hnan
  · have hnot : ¬ (2 ^ (f.prec - 1) + a < 2 ^ (f.prec - 1)) := by omega
    simp only [toBits, hnot, ite_false]
    have : ((c : ℤ) + f.qmin - 1 - f.qmin + 1).toNat = c := by
      rw [show (c : ℤ) + f.qmin - 1 - f.qmin + 1 = c by ring]; simp
    rw [this, hsign, show 2 ^ (f.prec - 1) + a - 2 ^ (f.prec - 1) = a by omega]; unfold enc; ring
  · exfalso; apply hcan'
    have := two_emax (f := f)
    have : f.qmax - f.qmin + 1 = 2 * f.emax := by
      unfold Format.qmax Format.qmin Format.emin; ring
    have : ((2 ^ f.ebits : ℕ) : ℤ) = 2 ^ f.ebits := by push_cast; rfl
    exact ⟨by omega, by omega, by omega, by omega, Or.inl (by omega)⟩

end FP

end IEEE754
