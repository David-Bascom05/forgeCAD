/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Mathlib

/-!
# dol-curved-rust `div.rs`'s gcd (5aca447 onward): binary on `u128`, Lehmer on `Big`

## `gcd_u128` (binary, Stein)

`trailing_zeros` of a nonzero `n` is its 2-adic valuation (`tz`); `>> k` is `/ 2^k`. The loop's
`a` is odd throughout; each pass halves `b` to odd, orders the pair and subtracts. `stein_spec`:
the loop returns `gcd a b` for odd `a`. `gcdU128_spec`: `gcd_u128 a b = gcd a b`, with
`shift = tz(a | b) = min(tz a, tz b)` (`tz_or`). `gcdU128_le`: the result is at most `max(a, b)`,
so `<< shift` drops no bit.

## `gcd` (Lehmer, TAOCP 4.5.2 L)

The inner loop runs on the leading 64 bits `xh = ⌊x/2^sh⌋`, `yh = ⌊y/2^sh⌋` with a 2×2 matrix
`M = (a b; c d)`, `u = a·xh + b·yh`, `v = c·xh + d·yh`. Its invariant (`Inv`): the two corner
images `M·(xh+1, yh) = (u+a, v+c)` and `M·(xh, yh+1) = (u+b, v+d)` lie in `[0, 2^64]²`, and the
signs of `M` alternate. Then:

- `cone`: with `D = xh + yh + 1`, `D·(x, y) = α·(xh+1, yh) + β·(xh, yh+1)` for integers
  `α, β ≥ 0`, `α + β = x + y`: the true point is between the corners.
- `mediant`: when the two corner quotients agree (`q = q2`), `q` is the Euclid quotient of the
  true pair `(X, Y) = M·(x, y)`. So every step of the inner loop is an exact Euclid step on `(x, y)`.
- `step_inv`: the step keeps `Inv`; `step_bounds`: every i128 intermediate is at most `2^66` in
  magnitude, so none overflows.
- `run_brk`: the loop stops within `2^65` passes (`v + c` falls each pass).
- `lehmerStep_gcd`: one outer pass keeps the gcd; `lehmerStep_lt`: it makes the smaller of the
  pair strictly smaller (a full division when the run is empty, `b = 0`; otherwise at least one
  exact Euclid step was taken). `gcdL_spec`: the whole loop returns `gcd x y`; `gcdBig_spec`:
  so does `gcd(a, o)`.

The model is at the level of values: `bits()` is the bit length (`Nat.size`), `top_at(x, sh)` is
`⌊|x|/2^sh⌋` (below `2^64` here, so the `as u64` drops nothing), the `S` fast path is taken when
both magnitudes are below `2^126` (`canon`'s `S`), and `divrem`, `mul`, `add`, `abs`, `bcmp_abs` are
their exact values (`Exact/Big.lean`, `Exact/DivRem2.lean`). `gcd_u128`'s `trailing_zeros` is the
2-adic valuation (`multiplicity 2`) and its shifts are `/ 2^k`, `· 2^k`.
-/

namespace IEEE754.Exact.Gcd

noncomputable section

/-! ## Binary gcd on `u128` -/

/-- `trailing_zeros` of a nonzero number: its 2-adic valuation. -/
abbrev tz (n : ℕ) : ℕ := multiplicity 2 n

lemma pow_tz_dvd_iff {n k : ℕ} (hn : n ≠ 0) : 2 ^ k ∣ n ↔ k ≤ tz n :=
  Nat.pow_dvd_iff_le_multiplicity (by norm_num) hn

lemma tz_or {a b : ℕ} (ha : a ≠ 0) (hb : b ≠ 0) : tz (a ||| b) = min (tz a) (tz b) := by
  have hab : a ||| b ≠ 0 := by rw [Ne, Nat.or_eq_zero_iff]; tauto
  refine eq_of_forall_le_iff fun k => ?_
  rw [le_min_iff, ← pow_tz_dvd_iff hab, ← pow_tz_dvd_iff ha, ← pow_tz_dvd_iff hb, Nat.dvd_iff_mod_eq_zero,
    Nat.dvd_iff_mod_eq_zero, Nat.dvd_iff_mod_eq_zero, Nat.or_mod_two_pow, Nat.or_eq_zero_iff]

/-- `n >> tz n` is odd, and `n` again once shifted back. -/
lemma odd_part {n : ℕ} (hn : n ≠ 0) : n / 2 ^ tz n * 2 ^ tz n = n ∧ (n / 2 ^ tz n) % 2 = 1 := by
  have d := (pow_tz_dvd_iff hn).2 le_rfl
  refine ⟨Nat.div_mul_cancel d, ?_⟩
  by_contra h
  have h0 : (n / 2 ^ tz n) % 2 = 0 := by omega
  obtain ⟨m, hm⟩ : 2 ∣ n / 2 ^ tz n := Nat.dvd_of_mod_eq_zero h0
  have : 2 ^ (tz n + 1) ∣ n := ⟨m, by
    calc n = n / 2 ^ tz n * 2 ^ tz n := (Nat.div_mul_cancel d).symm
      _ = 2 ^ (tz n + 1) * m := by rw [hm, pow_succ]; ring⟩
  have := (pow_tz_dvd_iff hn).1 this
  omega

lemma coprime_pow_odd {o : ℕ} (h : o % 2 = 1) (k : ℕ) : Nat.Coprime (2 ^ k) o :=
  Nat.Coprime.pow_left _ ((Nat.coprime_two_left).2 (Nat.odd_iff.2 h))

/-- `gcd_u128`'s loop: `while b != 0 { b >>= tz b; if a > b { swap }; b -= a }`, `a` odd. (The
guard `a = 0` keeps the model total; with `a` odd it is never taken.) -/
def stein (a b : ℕ) : ℕ :=
  if h : a = 0 ∨ b = 0 then a else
    if a > b / 2 ^ tz b then stein (b / 2 ^ tz b) (a - b / 2 ^ tz b) else stein a (b / 2 ^ tz b - a)
termination_by a + b
decreasing_by
  all_goals have := Nat.div_le_self b (2 ^ tz b)
  all_goals omega

theorem stein_spec : ∀ a b : ℕ, a % 2 = 1 → stein a b = Nat.gcd a b := by
  intro a b
  induction h : a + b using Nat.strong_induction_on generalizing a b with
  | _ n ih =>
    intro ha
    rw [stein]
    split_ifs with h0 h1
    · have : b = 0 := by omega
      subst this; simp
    · have hb : b ≠ 0 := by omega
      obtain ⟨hm, ho⟩ := odd_part hb
      have hle := Nat.div_le_self b (2 ^ tz b)
      have key : Nat.gcd a (b / 2 ^ tz b) = Nat.gcd a b :=
        (Nat.Coprime.gcd_mul_right_cancel_right _ (coprime_pow_odd ha _)).symm.trans (by rw [hm])
      rw [ih _ (by omega) _ _ rfl ho, Nat.gcd_sub_self_right (by omega), Nat.gcd_comm, key]
    · have hb : b ≠ 0 := by omega
      obtain ⟨hm, -⟩ := odd_part hb
      have hle := Nat.div_le_self b (2 ^ tz b)
      have key : Nat.gcd a (b / 2 ^ tz b) = Nat.gcd a b :=
        (Nat.Coprime.gcd_mul_right_cancel_right _ (coprime_pow_odd ha _)).symm.trans (by rw [hm])
      rw [ih _ (by omega) _ _ rfl ha, Nat.gcd_sub_self_right (by omega), key]

/-- `gcd_u128`. -/
def gcdU128 (a b : ℕ) : ℕ :=
  if a = 0 then b else if b = 0 then a else stein (a / 2 ^ tz a) b * 2 ^ tz (a ||| b)

theorem gcdU128_spec (a b : ℕ) : gcdU128 a b = Nat.gcd a b := by
  unfold gcdU128
  split_ifs with ha hb
  · subst ha; simp
  · subst hb; simp
  rw [tz_or ha hb]
  obtain ⟨ma, oa⟩ := odd_part ha
  obtain ⟨mb, ob⟩ := odd_part hb
  rw [stein_spec _ _ oa]
  set A := a / 2 ^ tz a
  set B := b / 2 ^ tz b
  have e1 : Nat.gcd A b = Nat.gcd A B := by
    rw [← mb, Nat.Coprime.gcd_mul_right_cancel_right _ (coprime_pow_odd oa _)]
  rw [e1]
  rcases le_total (tz a) (tz b) with hl | hl
  · rw [min_eq_left hl]
    have : b = B * 2 ^ (tz b - tz a) * 2 ^ tz a := by
      rw [mul_assoc, ← pow_add, Nat.sub_add_cancel hl, mb]
    conv_rhs => rw [← ma, this]
    rw [Nat.gcd_mul_right, Nat.Coprime.gcd_mul_right_cancel_right _ (coprime_pow_odd oa _)]
  · rw [min_eq_right hl]
    have : a = A * 2 ^ (tz a - tz b) * 2 ^ tz b := by
      rw [mul_assoc, ← pow_add, Nat.sub_add_cancel hl, ma]
    conv_rhs => rw [← mb, this]
    rw [Nat.gcd_mul_right, Nat.Coprime.gcd_mul_right_cancel _ (coprime_pow_odd ob _)]

/-- The result is at most `max(a, b)`: below `2^128` for `u128` inputs, so `<< shift` loses no bit. -/
theorem gcdU128_le (a b : ℕ) : gcdU128 a b ≤ max a b := by
  rw [gcdU128_spec]
  rcases Nat.eq_zero_or_pos a with ha | ha
  · subst ha; simp
  · exact le_trans (Nat.gcd_le_left _ ha) (le_max_left _ _)

/-! ## Lehmer's inner loop, on `i128` -/

/-- The inner loop's state: the leading digits `(u, v)` and the matrix `(a b; c d)`. -/
structure St where
  u : ℤ
  v : ℤ
  a : ℤ
  b : ℤ
  c : ℤ
  d : ℤ

/-- The pass's quotient `q = (u + a) / (v + c)` (Rust's `/` on `i128`: truncated). -/
def St.q (s : St) : ℤ := (s.u + s.a).tdiv (s.v + s.c)

/-- The loop breaks: `v + c == 0 || v + d == 0`, or the two quotients differ. -/
def St.brk (s : St) : Prop := s.v + s.c = 0 ∨ s.v + s.d = 0 ∨ s.q ≠ (s.u + s.b).tdiv (s.v + s.d)

instance (s : St) : Decidable s.brk := by unfold St.brk; infer_instance

/-- One pass: `(a, c) ← (c, a − q c)`, `(b, d) ← (d, b − q d)`, `(u, v) ← (v, u − q v)`. -/
def St.step (s : St) : St :=
  ⟨s.v, s.u - s.q * s.v, s.c, s.d, s.a - s.q * s.c, s.b - s.q * s.d⟩

/-- The loop, `n` passes at most. -/
def run : ℕ → St → St
  | 0, s => s
  | n + 1, s => if s.brk then s else run n s.step

/-- The start: `(u, v) = (xh, yh)`, `M = 1`. -/
def St.init (xh yh : ℤ) : St := ⟨xh, yh, 1, 0, 0, 1⟩

/-- The loop invariant, with `B = 2^64`. -/
def Inv (B xh yh : ℤ) (s : St) : Prop :=
  s.u = s.a * xh + s.b * yh ∧ s.v = s.c * xh + s.d * yh ∧
    0 ≤ s.u + s.a ∧ s.u + s.a ≤ B ∧ 0 ≤ s.v + s.c ∧ s.v + s.c ≤ B ∧
    0 ≤ s.u + s.b ∧ s.u + s.b ≤ B ∧ 0 ≤ s.v + s.d ∧ s.v + s.d ≤ B ∧
    ((0 ≤ s.a ∧ s.b ≤ 0 ∧ s.c ≤ 0 ∧ 0 ≤ s.d) ∨ (s.a ≤ 0 ∧ 0 ≤ s.b ∧ 0 ≤ s.c ∧ s.d ≤ 0))

lemma init_inv {B xh yh : ℤ} (h0 : 0 ≤ yh) (h1 : yh ≤ xh) (h2 : xh < B) : Inv B xh yh (St.init xh yh) := by
  unfold Inv St.init; dsimp only; refine ⟨by ring, by ring, ?_⟩; omega

/-- At a pass that does not break, both corner quotients are the floor quotient `q ≥ 0`. -/
lemma pass_facts {B xh yh : ℤ} {s : St} (hI : Inv B xh yh s) (hb : ¬ s.brk) :
    0 < s.v + s.c ∧ 0 < s.v + s.d ∧ 0 ≤ s.q ∧
      s.q * (s.v + s.c) ≤ s.u + s.a ∧ s.u + s.a < (s.q + 1) * (s.v + s.c) ∧
      s.q * (s.v + s.d) ≤ s.u + s.b ∧ s.u + s.b < (s.q + 1) * (s.v + s.d) := by
  obtain ⟨-, -, i1, -, i3, -, i5, -, i7, -, -⟩ := hI
  unfold St.brk at hb
  push Not at hb
  obtain ⟨n1, n2, hq⟩ := hb
  have p1 : 0 < s.v + s.c := lt_of_le_of_ne i3 (Ne.symm n1)
  have p2 : 0 < s.v + s.d := lt_of_le_of_ne i7 (Ne.symm n2)
  have e1 : s.q = (s.u + s.a) / (s.v + s.c) := Int.tdiv_eq_ediv_of_nonneg i1
  have e2 : s.q = (s.u + s.b) / (s.v + s.d) := by rw [hq]; exact Int.tdiv_eq_ediv_of_nonneg i5
  have d1 := Int.mul_ediv_add_emod (s.u + s.a) (s.v + s.c)
  have r1 := Int.emod_nonneg (s.u + s.a) (ne_of_gt p1)
  have r1' := Int.emod_lt_of_pos (s.u + s.a) p1
  have d2 := Int.mul_ediv_add_emod (s.u + s.b) (s.v + s.d)
  have r2 := Int.emod_nonneg (s.u + s.b) (ne_of_gt p2)
  have r2' := Int.emod_lt_of_pos (s.u + s.b) p2
  rw [← e1] at d1
  rw [← e2] at d2
  have hq0 : 0 ≤ s.q := by rw [e1]; exact Int.ediv_nonneg i1 (le_of_lt p1)
  refine ⟨p1, p2, hq0, ?_, ?_, ?_, ?_⟩ <;> nlinarith

theorem step_inv {B xh yh : ℤ} {s : St} (hI : Inv B xh yh s) (hb : ¬ s.brk) : Inv B xh yh s.step := by
  obtain ⟨p1, p2, hq0, l1, u1, l2, u2⟩ := pass_facts hI hb
  obtain ⟨e1, e2, i1, i2, i3, i4, i5, i6, i7, i8, sg⟩ := hI
  unfold Inv St.step; dsimp only
  refine ⟨e2, by rw [e1, e2]; ring, by linarith, by linarith, by nlinarith, by nlinarith, by linarith,
    by linarith, by nlinarith, by nlinarith, ?_⟩
  rcases sg with ⟨a0, b0, c0, d0⟩ | ⟨a0, b0, c0, d0⟩
  · right; refine ⟨c0, d0, ?_, ?_⟩ <;> nlinarith
  · left; refine ⟨c0, d0, ?_, ?_⟩ <;> nlinarith

/-- The matrix entries are at most `B`. -/
lemma entries_le {B xh yh : ℤ} {s : St} (hI : Inv B xh yh s) :
    |s.a| ≤ B ∧ |s.b| ≤ B ∧ |s.c| ≤ B ∧ |s.d| ≤ B := by
  obtain ⟨-, -, i1, i2, i3, i4, i5, i6, i7, i8, sg⟩ := hI
  rcases sg with ⟨a0, b0, c0, d0⟩ | ⟨a0, b0, c0, d0⟩
  · rw [abs_of_nonneg a0, abs_of_nonpos b0, abs_of_nonpos c0, abs_of_nonneg d0]
    refine ⟨?_, ?_, ?_, ?_⟩ <;> linarith
  · rw [abs_of_nonpos a0, abs_of_nonneg b0, abs_of_nonneg c0, abs_of_nonpos d0]
    refine ⟨?_, ?_, ?_, ?_⟩ <;> linarith

/-- **No `i128` overflow.** Every value a pass computes is at most `4B = 2^66` in magnitude. -/
theorem step_bounds {B xh yh : ℤ} {s : St} (hI : Inv B xh yh s) (hb : ¬ s.brk) :
    let q := s.q
    ∀ r ∈ [s.u, s.v, s.a, s.b, s.c, s.d, s.u + s.a, s.v + s.c, s.u + s.b, s.v + s.d, q,
      q * s.c, s.a - q * s.c, q * s.d, s.b - q * s.d, q * s.v, s.u - q * s.v], |r| ≤ 4 * B := by
  intro q r hr
  have hI' := step_inv hI hb
  obtain ⟨p1, p2, hq0, l1, u1, l2, u2⟩ := pass_facts hI hb
  obtain ⟨ea, eb, ec, ed⟩ := entries_le hI
  obtain ⟨ea', eb', ec', ed'⟩ := entries_le hI'
  have hI2 := hI
  obtain ⟨-, -, i1, i2, i3, i4, i5, i6, i7, i8, sg⟩ := hI2
  obtain ⟨-, -, j1, j2, j3, j4, -, -, -, -, -⟩ := hI'
  unfold St.step at ec' ed' j1 j2 j3 j4
  dsimp only at ec' ed' j1 j2 j3 j4
  rw [abs_le] at ea eb ec ed ec' ed'
  have hB : 0 ≤ B := by linarith
  have hq : q ≤ B := by
    have : q * 1 ≤ q * (s.v + s.c) := mul_le_mul_of_nonneg_left (by linarith) hq0
    show s.q ≤ B; linarith
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hr
  rcases hr with h | h | h | h | h | h | h | h | h | h | h | h | h | h | h | h | h <;> subst h <;>
    rw [abs_le] <;> constructor <;> linarith

/-- The loop stops: after `n > v + c` passes it has broken. -/
theorem run_brk {B xh yh : ℤ} : ∀ (n : ℕ) (s : St), Inv B xh yh s → (s.v + s.c).toNat < n → (run n s).brk ∧
    Inv B xh yh (run n s)
  | 0, s, _, h => absurd h (Nat.not_lt_zero _)
  | n + 1, s, hI, h => by
    unfold run
    split_ifs with hb
    · exact ⟨hb, hI⟩
    · obtain ⟨p1, -, -, l1, u1, -, -⟩ := pass_facts hI hb
      apply run_brk n s.step (step_inv hI hb)
      unfold St.step; dsimp only
      have : s.u - s.q * s.v + (s.a - s.q * s.c) < s.v + s.c := by nlinarith
      have : 0 ≤ s.u - s.q * s.v + (s.a - s.q * s.c) := by nlinarith
      omega

/-- `run` keeps `Inv` whatever its fuel. -/
theorem run_inv {B xh yh : ℤ} : ∀ (n : ℕ) (s : St), Inv B xh yh s → Inv B xh yh (run n s)
  | 0, _, hI => hI
  | n + 1, s, hI => by
    unfold run; split_ifs with hb
    · exact hI
    · exact run_inv n _ (step_inv hI hb)

/-! ## The true pair follows Euclid -/

/-- `(X, Y) = M·(x, y)`. -/
def X (x y : ℤ) (s : St) : ℤ := s.a * x + s.b * y
def Y (x y : ℤ) (s : St) : ℤ := s.c * x + s.d * y

/-- **Mediant.** If `D·(x, y) = α·(xh+1, yh) + β·(xh, yh+1)` with `α, β ≥ 0`, `α + β > 0`, `D > 0`,
then a pass that does not break takes `q = ⌊X/Y⌋`: `0 ≤ X − qY < Y`. -/
theorem mediant {B xh yh x y α β D : ℤ} {s : St} (hI : Inv B xh yh s) (hb : ¬ s.brk)
    (hα : 0 ≤ α) (hβ : 0 ≤ β) (hαβ : 0 < α + β) (hD : 0 < D)
    (ex : D * x = α * (xh + 1) + β * xh) (ey : D * y = α * yh + β * (yh + 1)) :
    0 ≤ X x y s - s.q * Y x y s ∧ X x y s - s.q * Y x y s < Y x y s := by
  obtain ⟨p1, p2, hq0, l1, u1, l2, u2⟩ := pass_facts hI hb
  obtain ⟨e1, e2, -⟩ := hI
  have dX : D * X x y s = α * (s.u + s.a) + β * (s.u + s.b) := by
    unfold X; rw [e1]; linear_combination s.a * ex + s.b * ey
  have dY : D * Y x y s = α * (s.v + s.c) + β * (s.v + s.d) := by
    unfold Y; rw [e2]; linear_combination s.c * ex + s.d * ey
  -- integers: `u + a ≤ (q + 1)(v + c) − 1`
  have u1' : s.u + s.a + 1 ≤ (s.q + 1) * (s.v + s.c) := u1
  have u2' : s.u + s.b + 1 ≤ (s.q + 1) * (s.v + s.d) := u2
  have k1 : D * (s.q * Y x y s) ≤ D * X x y s := by
    rw [dX, show D * (s.q * Y x y s) = s.q * (D * Y x y s) by ring, dY]; nlinarith
  have k2 : D * X x y s < D * ((s.q + 1) * Y x y s) := by
    rw [dX, show D * ((s.q + 1) * Y x y s) = (s.q + 1) * (D * Y x y s) by ring, dY]; nlinarith
  have k1' := le_of_mul_le_mul_left k1 hD
  have k2' := lt_of_mul_lt_mul_left k2 (le_of_lt hD)
  constructor <;> linarith

lemma step_XY (x y : ℤ) (s : St) :
    X x y s.step = Y x y s ∧ Y x y s.step = X x y s - s.q * Y x y s := by
  unfold X Y St.step; dsimp only; constructor <;> ring

/-- Along the run: the gcd of the true pair is kept, and once a pass is taken the second of the
pair is in `[0, y)`. -/
theorem run_euclid {B xh yh x y α β D : ℤ}
    (hα : 0 ≤ α) (hβ : 0 ≤ β) (hαβ : 0 < α + β) (hD : 0 < D)
    (ex : D * x = α * (xh + 1) + β * xh) (ey : D * y = α * yh + β * (yh + 1)) :
    ∀ (n : ℕ) (s : St), Inv B xh yh s → (s.b = 0 ∧ Y x y s = y ∨ 0 ≤ Y x y s ∧ Y x y s < y) →
      Int.gcd (X x y (run n s)) (Y x y (run n s)) = Int.gcd (X x y s) (Y x y s) ∧
      ((run n s).b = 0 ∧ run n s = s ∨ 0 ≤ Y x y (run n s) ∧ Y x y (run n s) < y)
  | 0, s, _, h => ⟨rfl, by
      rcases h with h | h
      · exact Or.inl ⟨h.1, rfl⟩
      · exact Or.inr h⟩
  | n + 1, s, hI, h => by
    unfold run
    split_ifs with hb
    · refine ⟨rfl, ?_⟩
      rcases h with h | h
      · exact Or.inl ⟨h.1, rfl⟩
      · exact Or.inr h
    · obtain ⟨m1, m2⟩ := mediant hI hb hα hβ hαβ hD ex ey
      obtain ⟨sx, sy⟩ := step_XY x y s
      have hY : 0 ≤ Y x y s.step ∧ Y x y s.step < y := by
        rw [sy]; refine ⟨m1, ?_⟩
        rcases h with h | h
        · exact lt_of_lt_of_eq m2 h.2
        · linarith [h.2]
      obtain ⟨g, r⟩ := run_euclid hα hβ hαβ hD ex ey n s.step (step_inv hI hb) (Or.inr hY)
      refine ⟨?_, ?_⟩
      · rw [g, sx, sy, show X x y s - s.q * Y x y s = X x y s + (-s.q) * Y x y s by ring,
          Int.gcd_add_mul_right_right, Int.gcd_comm]
      · rcases r with r | r
        · rw [r.2]; exact Or.inr hY
        · exact Or.inr r

/-- **The cone.** With `xh = ⌊x/2^sh⌋`, `yh = ⌊y/2^sh⌋`, `D = xh + yh + 1`, the integers
`α = x(yh+1) − xh·y`, `β = (xh+1)y − yh·x` are `≥ 0`, sum to `x + y`, and
`D·(x, y) = α·(xh+1, yh) + β·(xh, yh+1)`. -/
theorem cone (x y sh : ℕ) :
    let xh : ℤ := (x / 2 ^ sh : ℕ)
    let yh : ℤ := (y / 2 ^ sh : ℕ)
    let α : ℤ := x * (yh + 1) - xh * y
    let β : ℤ := (xh + 1) * y - yh * x
    0 ≤ α ∧ 0 ≤ β ∧ α + β = x + y ∧
      (xh + yh + 1) * x = α * (xh + 1) + β * xh ∧ (xh + yh + 1) * y = α * yh + β * (yh + 1) := by
  intro xh yh α β
  have hp : (0 : ℤ) < 2 ^ sh := by positivity
  have x1 : (2 : ℤ) ^ sh * xh ≤ x := by
    have := Nat.div_mul_le_self x (2 ^ sh); simp only [xh]; push_cast; zify at this; linarith
  have x2 : (x : ℤ) < 2 ^ sh * (xh + 1) := by
    have := Nat.lt_succ_iff.mpr (le_refl (x / 2 ^ sh))
    have h := (Nat.div_lt_iff_lt_mul (by positivity)).1 this
    simp only [xh]; zify at h; push_cast at h ⊢; linarith
  have y1 : (2 : ℤ) ^ sh * yh ≤ y := by
    have := Nat.div_mul_le_self y (2 ^ sh); simp only [yh]; push_cast; zify at this; linarith
  have y2 : (y : ℤ) < 2 ^ sh * (yh + 1) := by
    have := Nat.lt_succ_iff.mpr (le_refl (y / 2 ^ sh))
    have h := (Nat.div_lt_iff_lt_mul (by positivity)).1 this
    simp only [yh]; zify at h; push_cast at h ⊢; linarith
  have hx0 : (0 : ℤ) ≤ xh := by positivity
  have hy0 : (0 : ℤ) ≤ yh := by positivity
  refine ⟨?_, ?_, by ring, by ring, by ring⟩
  · -- `x(yh+1) ≥ 2^sh·xh·(yh+1) ≥ xh·y`
    have a1 : (2 : ℤ) ^ sh * xh * (yh + 1) ≤ x * (yh + 1) := mul_le_mul_of_nonneg_right x1 (by linarith)
    have a2 : xh * y ≤ xh * (2 ^ sh * (yh + 1)) := mul_le_mul_of_nonneg_left (le_of_lt y2) hx0
    have : (2 : ℤ) ^ sh ≥ 1 := by exact_mod_cast Nat.one_le_two_pow
    show 0 ≤ x * (yh + 1) - xh * y
    nlinarith
  · have a1 : (2 : ℤ) ^ sh * yh * (xh + 1) ≤ y * (xh + 1) := mul_le_mul_of_nonneg_right y1 (by linarith)
    have a2 : yh * x ≤ yh * (2 ^ sh * (xh + 1)) := mul_le_mul_of_nonneg_left (le_of_lt x2) hy0
    show 0 ≤ (xh + 1) * y - yh * x
    nlinarith

/-! ## The outer loop -/

/-- One pass of `gcd`'s outer loop on `x ≥ y`, `x` past `S` (`2^126`): `sh = bits(x) − 64`, the inner run
on `(⌊x/2^sh⌋, ⌊y/2^sh⌋)` (its fuel `2^65` is never reached: `lehmerStep_fuel`), then a full
division if the run is empty (`b = 0`), else `(|ax + by|, |cx + dy|)`, larger first. -/
def lehmerStep (x y : ℕ) : ℕ × ℕ :=
  let sh := Nat.size x - 64
  let s := run (2 ^ 65) (St.init (x / 2 ^ sh : ℕ) (y / 2 ^ sh : ℕ))
  if s.b = 0 then (y, x % y) else
    if (X x y s).natAbs < (Y x y s).natAbs then ((Y x y s).natAbs, (X x y s).natAbs)
    else ((X x y s).natAbs, (Y x y s).natAbs)

lemma lehmerStep_le {x y : ℕ} (hy : y ≠ 0) : (lehmerStep x y).2 ≤ (lehmerStep x y).1 := by
  unfold lehmerStep; dsimp only
  split_ifs with h1 h2
  · exact le_of_lt (Nat.mod_lt _ (Nat.pos_of_ne_zero hy))
  · exact le_of_lt h2
  · exact not_lt.1 h2

/-- The setup of a pass: the leading digits are below `2^64`, in order, and the cone holds. -/
lemma pass_setup {x y : ℕ} (hyx : y ≤ x) (hx : 2 ^ 126 ≤ x) :
    let sh := Nat.size x - 64
    Inv (2 ^ 64) (x / 2 ^ sh : ℕ) (y / 2 ^ sh : ℕ) (St.init (x / 2 ^ sh : ℕ) (y / 2 ^ sh : ℕ)) := by
  intro sh
  have hs : 126 < Nat.size x := Nat.lt_size.2 hx
  have hlt : x < 2 ^ 64 * 2 ^ sh := by
    rw [← pow_add, show 64 + sh = Nat.size x by omega]; exact Nat.lt_size_self x
  have h1 : x / 2 ^ sh < 2 ^ 64 := (Nat.div_lt_iff_lt_mul (by positivity)).2 hlt
  have h2 : y / 2 ^ sh ≤ x / 2 ^ sh := Nat.div_le_div_right hyx
  apply init_inv
  · positivity
  · exact_mod_cast h2
  · exact_mod_cast h1

/-- The inner run has stopped well inside its fuel. -/
theorem lehmerStep_fuel {x y : ℕ} (hyx : y ≤ x) (hx : 2 ^ 126 ≤ x) :
    let sh := Nat.size x - 64
    (run (2 ^ 65) (St.init (x / 2 ^ sh : ℕ) (y / 2 ^ sh : ℕ))).brk := by
  intro sh
  have hI := pass_setup hyx hx
  refine (run_brk (2 ^ 65) _ hI ?_).1
  obtain ⟨-, -, -, -, -, h, -⟩ := hI
  unfold St.init at h ⊢; dsimp only at h ⊢
  have : (y / 2 ^ sh : ℕ) + (0 : ℤ) ≤ 2 ^ 64 := h
  omega

lemma natAbs_lt_of {z : ℤ} {n : ℕ} (h0 : 0 ≤ z) (h : z < n) : z.natAbs < n := by
  exact_mod_cast (show ((z.natAbs : ℕ) : ℤ) < n by rw [Int.natAbs_of_nonneg h0]; exact h)

/-- What a pass does to the pair: the gcd is kept, and the smaller strictly falls. -/
theorem lehmerStep_spec {x y : ℕ} (hyx : y ≤ x) (hx : 2 ^ 126 ≤ x) (hy : y ≠ 0) :
    Nat.gcd (lehmerStep x y).1 (lehmerStep x y).2 = Nat.gcd x y ∧ (lehmerStep x y).2 < y := by
  have hI := pass_setup hyx hx
  set sh := Nat.size x - 64
  obtain ⟨ha, hb, hab, ex, ey⟩ := cone x y sh
  set xh : ℤ := ((x / 2 ^ sh : ℕ) : ℤ)
  set yh : ℤ := ((y / 2 ^ sh : ℕ) : ℤ)
  have hx0 : (0 : ℤ) ≤ xh := by positivity
  have hy0 : (0 : ℤ) ≤ yh := by positivity
  have hD : (0 : ℤ) < xh + yh + 1 := by linarith
  have hαβ : (0 : ℤ) < (x : ℤ) * (yh + 1) - xh * y + ((xh + 1) * y - yh * x) := by
    rw [hab]; have : (0 : ℤ) < y := by exact_mod_cast Nat.pos_of_ne_zero hy
    positivity
  have hY0 : (St.init xh yh).b = 0 ∧ Y x y (St.init xh yh) = y := by
    unfold St.init Y; dsimp only; constructor <;> ring
  obtain ⟨g, r⟩ := run_euclid ha hb hαβ hD ex ey (2 ^ 65) _ hI (Or.inl hY0)
  have g0 : Int.gcd (X x y (St.init xh yh)) (Y x y (St.init xh yh)) = Nat.gcd x y := by
    unfold St.init X Y; dsimp only; simp
  rw [g0] at g
  unfold lehmerStep; dsimp only
  split_ifs with h1 h2
  · refine ⟨?_, Nat.mod_lt _ (Nat.pos_of_ne_zero hy)⟩
    show Nat.gcd y (x % y) = Nat.gcd x y
    rw [Nat.gcd_comm, ← Nat.gcd_rec, Nat.gcd_comm]
  all_goals
    rcases r with r | r
    · exact absurd r.1 h1
    have hYn := natAbs_lt_of r.1 r.2
    rw [Int.gcd_eq_natAbs] at g
  · exact ⟨by rw [Nat.gcd_comm, g], lt_trans h2 hYn⟩
  · exact ⟨g, hYn⟩

/-- `gcd`'s loop on `x ≥ y`: binary once both are `S` (below `2^126`), `x` once `y = 0`, else a Lehmer pass. -/
def gcdL (x y : ℕ) (h : y ≤ x) : ℕ :=
  if x < 2 ^ 126 ∧ y < 2 ^ 126 then gcdU128 x y
  else if hy : y = 0 then x
  else gcdL (lehmerStep x y).1 (lehmerStep x y).2 (lehmerStep_le hy)
termination_by y
decreasing_by
  exact (lehmerStep_spec h (by omega) hy).2

theorem gcdL_spec : ∀ (y x : ℕ) (h : y ≤ x), gcdL x y h = Nat.gcd x y := by
  intro y
  induction y using Nat.strong_induction_on with
  | _ y ih =>
    intro x h
    rw [gcdL]
    split_ifs with h1 hy
    · exact gcdU128_spec x y
    · subst hy; simp
    · have hp := lehmerStep_spec h (by omega) hy
      rw [ih _ hp.2, hp.1]

/-- **`gcd`.** `x = |a|`, `y = |o|`, ordered larger first; the result is `gcd(|a|, |o|)`. -/
def gcdBig (a o : ℤ) : ℕ :=
  if h : a.natAbs < o.natAbs then gcdL o.natAbs a.natAbs (le_of_lt h)
  else gcdL a.natAbs o.natAbs (not_lt.1 h)

theorem gcdBig_spec (a o : ℤ) : gcdBig a o = Int.gcd a o := by
  unfold gcdBig
  split_ifs
  · rw [gcdL_spec, Int.gcd_eq_natAbs, Nat.gcd_comm]
  · rw [gcdL_spec, Int.gcd_eq_natAbs]

end

end IEEE754.Exact.Gcd
