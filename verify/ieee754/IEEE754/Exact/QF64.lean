/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Binary64
import IEEE754.Exact.QBig

/-!
# dol-curved-rust `Q::cmp_f64`, `Q::lo_f64`, `Q::hi_f64` (forge-alg lib.rs, 1afff77)

The curved builder's float filters stand on these bounds.

- `cmpF_spec`: `cmp_f64(f)` is the sign of `x − f`, exactly. Here `f` is a finite float, taken as
  `Dy::from_f64(f) = m·2^e` (any decode with that value), and `x = n/d` has `d > 0`. No gcd is
  involved.
- `lo_sound`, `hi_sound`: `lo_f64` returns `−∞` or a finite float `≤ x`; `hi_f64` returns `+∞` or
  a finite float `≥ x`. This holds for any `approx` start and any `next_down`/`next_up`: soundness
  rests on the exact check alone. (Tightness, within a few ulps, is not claimed.)
-/

namespace IEEE754.Exact.QB

open Big

/-! ## `cmp_f64` -/

/-- `cmp_f64` against a float decoded as `m·2^e`: `bcmp(n, (m·d) << e)` for `e ≥ 0`, else
`bcmp(n << −e, m·d)`. -/
def qcmpF (x : QB) (m : Big) (e : ℤ) : ℤ :=
  if e ≥ 0 then bcmp x.n (shl (Big.mul m x.d) e.toNat) else bcmp (shl x.n (-e).toNat) (Big.mul m x.d)

/-- `cmp_f64` is the sign of `n·2^(−e)⁺ − m·d·2^(e⁺)`. -/
lemma qcmpF_int {x : QB} (hx : Can x) {m : Big} (hm : Big.Canon m) (e : ℤ) :
    qcmpF x m e = Int.sign (val x.n * 2 ^ (-e).toNat - val m * val x.d * 2 ^ e.toNat) := by
  obtain ⟨xn, xd⟩ := hx
  obtain ⟨-, ⟨c1, v1⟩⟩ := am hm xd
  unfold qcmpF
  split_ifs with he
  · obtain ⟨c2, v2⟩ := shl_spec c1 e.toNat
    rw [bcmp_spec xn c2, v2, v1, show (-e).toNat = 0 by omega, pow_zero, mul_one]
  · obtain ⟨c2, v2⟩ := shl_spec xn (-e).toNat
    rw [bcmp_spec c2 c1, v2, v1, show e.toNat = 0 by omega, pow_zero, mul_one]

/-- **`cmp_f64`.** For `d > 0` and a finite float of value `m·2^e`, `cmp_f64` is the sign of
`n/d − f`. -/
theorem cmpF_spec {x : QB} (hx : Can x) (hd : 0 < val x.d) {m : Big} (hm : Big.Canon m) (e : ℤ) {f : ℚ}
    (hf : (val m : ℚ) * 2 ^ e = f) :
    (0 ≤ qcmpF x m e ↔ f ≤ QZ.val (toZ x)) ∧ (qcmpF x m e ≤ 0 ↔ QZ.val (toZ x) ≤ f) := by
  rw [qcmpF_int hx hm e]
  set A : ℤ := val x.n * 2 ^ (-e).toNat - val m * val x.d * 2 ^ e.toNat
  have hdq : (0 : ℚ) < val x.d := by exact_mod_cast hd
  -- `A / (d · 2^(−e)⁺) = x − f`
  have hp : (0 : ℚ) < (2 : ℚ) ^ (-e).toNat := by positivity
  have he : (2 : ℚ) ^ e = (2 : ℚ) ^ e.toNat / 2 ^ (-e).toNat := by
    rw [← zpow_natCast, ← zpow_natCast, ← zpow_sub₀ (by norm_num)]
    congr 1; omega
  have key : (A : ℚ) = (QZ.val (toZ x) - f) * (val x.d * 2 ^ (-e).toNat) := by
    simp only [A, QZ.val, toZ, ← hf, he]; push_cast
    field_simp
  have hpos : (0 : ℚ) < val x.d * 2 ^ (-e).toNat := mul_pos hdq hp
  constructor
  · rw [Int.sign_nonneg_iff]
    constructor
    · intro h
      have : (0 : ℚ) ≤ A := by exact_mod_cast h
      rw [key] at this
      have := (mul_nonneg_iff_of_pos_right hpos).1 this
      linarith
    · intro h
      have : (0 : ℚ) ≤ A := by rw [key]; exact mul_nonneg (by linarith) (le_of_lt hpos)
      exact_mod_cast this
  · rw [Int.sign_nonpos_iff]
    constructor
    · intro h
      have : (A : ℚ) ≤ 0 := by exact_mod_cast h
      rw [key] at this
      have := nonpos_of_mul_nonpos_left this hpos
      linarith
    · intro h
      have : (A : ℚ) ≤ 0 := by rw [key]; exact mul_nonpos_of_nonpos_of_nonneg (by linarith) (le_of_lt hpos)
      exact_mod_cast this

/-! ## `lo_f64`, `hi_f64` -/

/-- `lo_f64`'s loop, `k` passes left: `(f, ok)`. -/
def loRun (cmp : F64 → ℤ) (nd : F64 → F64) : ℕ → F64 → F64 × Bool
  | 0, f => (f, false)
  | k + 1, f => if ¬ f.isFinite then (.inf true, true) else if cmp f ≥ 0 then (f, true) else loRun cmp nd k (nd f)

/-- `lo_f64`: from `approx`, four passes, `−∞` if none checked. -/
def loF (cmp : F64 → ℤ) (nd : F64 → F64) (f0 : F64) : F64 :=
  if (loRun cmp nd 4 f0).2 then (loRun cmp nd 4 f0).1 else .inf true

/-- `hi_f64`'s loop. -/
def hiRun (cmp : F64 → ℤ) (nu : F64 → F64) : ℕ → F64 → F64 × Bool
  | 0, f => (f, false)
  | k + 1, f => if ¬ f.isFinite then (.inf false, true) else if cmp f ≤ 0 then (f, true) else hiRun cmp nu k (nu f)

def hiF (cmp : F64 → ℤ) (nu : F64 → F64) (f0 : F64) : F64 :=
  if (hiRun cmp nu 4 f0).2 then (hiRun cmp nu 4 f0).1 else .inf false

lemma loRun_sound {cmp : F64 → ℤ} {nd : F64 → F64} {v : ℚ} (hc : ∀ f : F64, f.isFinite → 0 ≤ cmp f → f.val ≤ v) :
    ∀ (k : ℕ) (f : F64), (loRun cmp nd k f).2 = true →
      (loRun cmp nd k f).1 = .inf true ∨ ((loRun cmp nd k f).1.isFinite ∧ (loRun cmp nd k f).1.val ≤ v)
  | 0, f, h => by simp [loRun] at h
  | k + 1, f, h => by
    by_cases h1 : f.isFinite
    · by_cases h2 : 0 ≤ cmp f
      · have e : loRun cmp nd (k + 1) f = (f, true) := by
          simp only [loRun, h1, not_true_eq_false, if_false, ge_iff_le, h2, if_true]
        rw [e]; exact Or.inr ⟨h1, hc f h1 h2⟩
      · have e : loRun cmp nd (k + 1) f = loRun cmp nd k (nd f) := by
          simp only [loRun, h1, not_true_eq_false, if_false, ge_iff_le, h2]
        rw [e] at h ⊢; exact loRun_sound hc k (nd f) h
    · have e : loRun cmp nd (k + 1) f = (.inf true, true) := by
        simp only [loRun, h1, Bool.false_eq_true, not_false_eq_true, if_true]
      rw [e]; exact Or.inl rfl

lemma hiRun_sound {cmp : F64 → ℤ} {nu : F64 → F64} {v : ℚ} (hc : ∀ f : F64, f.isFinite → cmp f ≤ 0 → v ≤ f.val) :
    ∀ (k : ℕ) (f : F64), (hiRun cmp nu k f).2 = true →
      (hiRun cmp nu k f).1 = .inf false ∨ ((hiRun cmp nu k f).1.isFinite ∧ v ≤ (hiRun cmp nu k f).1.val)
  | 0, f, h => by simp [hiRun] at h
  | k + 1, f, h => by
    by_cases h1 : f.isFinite
    · by_cases h2 : cmp f ≤ 0
      · have e : hiRun cmp nu (k + 1) f = (f, true) := by
          simp only [hiRun, h1, not_true_eq_false, if_false, ge_iff_le, h2, if_true]
        rw [e]; exact Or.inr ⟨h1, hc f h1 h2⟩
      · have e : hiRun cmp nu (k + 1) f = hiRun cmp nu k (nu f) := by
          simp only [hiRun, h1, not_true_eq_false, if_false, ge_iff_le, h2]
        rw [e] at h ⊢; exact hiRun_sound hc k (nu f) h
    · have e : hiRun cmp nu (k + 1) f = (.inf false, true) := by
        simp only [hiRun, h1, Bool.false_eq_true, not_false_eq_true, if_true]
      rw [e]; exact Or.inl rfl

/-- The `cmp_f64` the loops call, through a decode `dec f = (m, e)` with `m·2^e = f`. -/
def cmpOf (x : QB) (dec : F64 → Big × ℤ) (f : F64) : ℤ := qcmpF x (dec f).1 (dec f).2

/-- A decode of finite floats: canonical `m`, `m·2^e` the float's value (`Dy::from_f64`). -/
def DecOk (dec : F64 → Big × ℤ) : Prop :=
  ∀ f : F64, f.isFinite → Big.Canon (dec f).1 ∧ (val (dec f).1 : ℚ) * 2 ^ (dec f).2 = f.val

/-- **`lo_f64` is a lower bound.** Whatever `approx` and `next_down` give, the result is `−∞` or
a finite float `≤ n/d`. -/
theorem lo_sound {x : QB} (hx : Can x) (hd : 0 < val x.d) {dec : F64 → Big × ℤ} (hdec : DecOk dec)
    (nd : F64 → F64) (f0 : F64) :
    loF (cmpOf x dec) nd f0 = .inf true ∨
      ((loF (cmpOf x dec) nd f0).isFinite ∧ (loF (cmpOf x dec) nd f0).val ≤ QZ.val (toZ x)) := by
  have hc : ∀ f : F64, f.isFinite → 0 ≤ cmpOf x dec f → f.val ≤ QZ.val (toZ x) := fun f hf h =>
    ((cmpF_spec hx hd (hdec f hf).1 _ (hdec f hf).2).1).1 h
  unfold loF
  split_ifs with h
  · exact loRun_sound hc 4 f0 h
  · exact Or.inl rfl

/-- **`hi_f64` is an upper bound.** The result is `+∞` or a finite float `≥ n/d`. -/
theorem hi_sound {x : QB} (hx : Can x) (hd : 0 < val x.d) {dec : F64 → Big × ℤ} (hdec : DecOk dec)
    (nu : F64 → F64) (f0 : F64) :
    hiF (cmpOf x dec) nu f0 = .inf false ∨
      ((hiF (cmpOf x dec) nu f0).isFinite ∧ QZ.val (toZ x) ≤ (hiF (cmpOf x dec) nu f0).val) := by
  have hc : ∀ f : F64, f.isFinite → cmpOf x dec f ≤ 0 → QZ.val (toZ x) ≤ f.val := fun f hf h =>
    ((cmpF_spec hx hd (hdec f hf).1 _ (hdec f hf).2).2).1 h
  unfold hiF
  split_ifs with h
  · exact hiRun_sound hc 4 f0 h
  · exact Or.inl rfl

end IEEE754.Exact.QB
