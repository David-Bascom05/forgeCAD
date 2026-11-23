/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Exact.Limbs

/-!
# forge-math `exact.rs`: `Big` and its exact sign (item 3)

decide-once's `Big` (b6a9dd4 onward) has three representations:
- `S(i128)`, below `2^126` in magnitude;
- `M { neg, n, w: [u64; 8] }`, `n` trimmed limbs inline, the unused ones zero;
- `L { neg, mag: Vec<u64> }`, more than 8 limbs, the top one nonzero.

`Canon` is that invariant. Under it:
- `from_mag` builds a canonical value of the given sign and magnitude (`fromMag_spec`);
- `add`, `neg`, `sub`, `mul` and `shl` are the integer operations and stay canonical;
- `signum` is `Int.sign`, `is_zero` is `= 0`, and `cmp_big` is `compare`.

So every sign the ledger takes from a `Big` expression is exact.

The model: M's inline array is a function zero past `n`, and L's vector is a function with a
length. The i128 fast paths are modelled on `ℤ`, each with its guard (`fits`, `checked_mul`, and
`used + bits < 126` for `<<`) shown to keep i128 exact. `shl`'s `used + bits` is a u32 sum, so
`bits + 126 < 2^32` is a hypothesis.
-/

namespace IEEE754.Exact

open Finset

/-- `Big`. -/
inductive Big
  | S (v : ℤ)
  | M (neg : Bool) (n : ℕ) (w : ℕ → ℕ)
  | L (neg : Bool) (len : ℕ) (mag : ℕ → ℕ)

/-- A sign and a magnitude. -/
def sgnz (neg : Bool) (x : ℕ) : ℤ := if neg then -(x : ℤ) else x

namespace Big

/-- The value. -/
def val : Big → ℤ
  | S v => v
  | M neg n w => sgnz neg (valF w n)
  | L neg len mag => sgnz neg (valF mag len)

/-- One representation per value. -/
def Canon : Big → Prop
  | S v => |v| < 2 ^ 126
  | M _ n w => 1 ≤ n ∧ n ≤ 8 ∧ Limbs w n ∧ w (n - 1) ≠ 0 ∧ (∀ i ≥ n, w i = 0) ∧
      ¬ (n ≤ 2 ∧ valF w n < 2 ^ 126)
  | L _ len mag => 8 < len ∧ Limbs mag len ∧ mag (len - 1) ≠ 0 ∧ (∀ i ≥ len, mag i = 0)

lemma _root_.IEEE754.Exact.Limbs.mono_len {w : ℕ → ℕ} {n m : ℕ} (h : Limbs w n) (hm : m ≤ n) : Limbs w m :=
  fun i hi => h i (by omega)

/-- `fits`: inline while under `2^126`. -/
def fits (v : ℤ) : Prop := |v| < 2 ^ 126

instance (v : ℤ) : Decidable (fits v) := by unfold fits; infer_instance

/-- `from_mag(neg, m)`: trim, then the one representation. -/
def fromMag (neg : Bool) (f : ℕ → ℕ) (n : ℕ) : Big :=
  let t := trimLen f n
  let v : ℕ := (if 0 < t then f 0 else 0) + (if 1 < t then f 1 else 0) * B
  if t ≤ 2 ∧ v < 2 ^ 126 then S (sgnz neg v)
  else if t ≤ 8 then M neg t (fun i => if i < t then f i else 0)
  else L neg t (fun i => if i < t then f i else 0)

/-- `view`: sign, limbs and length (an inline value's two limbs, trimmed). -/
def view : Big → Bool × (ℕ → ℕ) × ℕ
  | S v =>
    let a := v.natAbs
    let tmp : ℕ → ℕ := fun i => if i = 0 then a % B else if i = 1 then a / B else 0
    (decide (v < 0), tmp, trimLen tmp 2)
  | M neg n w => (neg, w, n)
  | L neg len mag => (neg, mag, len)

lemma valF_restrict {f : ℕ → ℕ} {t : ℕ} : valF (fun i => if i < t then f i else 0) t = valF f t :=
  valF_congr (fun i hi => by simp [hi])

lemma valF_two (f : ℕ → ℕ) (t : ℕ) (ht : t ≤ 2) :
    valF f t = (if 0 < t then f 0 else 0) + (if 1 < t then f 1 else 0) * B := by
  interval_cases t <;> simp [valF, sum_range_succ]

/-- **`from_mag`.** -/
theorem fromMag_spec {neg : Bool} {f : ℕ → ℕ} {n : ℕ} (hf : Limbs f n) :
    Canon (fromMag neg f n) ∧ val (fromMag neg f n) = sgnz neg (valF f n) := by
  obtain ⟨tle, tval, ttop⟩ := trimLen_spec f n
  have hft : Limbs f (trimLen f n) := fun i hi => hf i (by omega)
  unfold fromMag
  simp only
  generalize hv : (if 0 < trimLen f n then f 0 else 0) + (if 1 < trimLen f n then f 1 else 0) * B = v
  by_cases h1 : trimLen f n ≤ 2 ∧ v < 2 ^ 126
  · rw [if_pos h1]
    have hvv : v = valF f n := by rw [← hv, ← tval, valF_two f _ h1.1]
    refine ⟨?_, ?_⟩
    · simp only [Canon, sgnz]
      have : ((v : ℕ) : ℤ) < 2 ^ 126 := by exact_mod_cast h1.2
      split_ifs
      · rw [abs_neg, abs_of_nonneg (by positivity)]; exact this
      · rw [abs_of_nonneg (by positivity)]; exact this
    · simp only [val]; rw [hvv]
  · rw [if_neg h1]
    have tpos : 0 < trimLen f n := by
      by_contra hc; push Not at hc
      apply h1; refine ⟨by omega, ?_⟩
      rw [← hv, show trimLen f n = 0 by omega]; simp
    have top : f (trimLen f n - 1) ≠ 0 := by rcases ttop with h | h <;> [omega; exact h]
    by_cases h2 : trimLen f n ≤ 8
    · rw [if_pos h2]
      refine ⟨⟨tpos, h2, ?_, ?_, ?_, ?_⟩, ?_⟩
      · intro i hi; simp only [hi, ite_true]; exact hft i hi
      · simp only [show trimLen f n - 1 < trimLen f n by omega, ite_true]; exact top
      · intro i hi; simp [show ¬ i < trimLen f n by omega]
      · rw [valF_restrict]
        intro hc; apply h1; refine ⟨hc.1, ?_⟩
        rw [← hv, ← valF_two f _ hc.1]; exact hc.2
      · simp only [val]; rw [valF_restrict, tval]
    · rw [if_neg h2]
      refine ⟨⟨by omega, ?_, ?_, ?_⟩, ?_⟩
      · intro i hi; simp only [hi, ite_true]; exact hft i hi
      · simp only [show trimLen f n - 1 < trimLen f n by omega, ite_true]; exact top
      · intro i hi; simp [show ¬ i < trimLen f n by omega]
      · simp only [val]; rw [valF_restrict, tval]

/-- **`view`.** Limbs under `2^64`, trimmed, and the value is the sign on the magnitude; a negative
sign means a nonzero magnitude. -/
theorem view_spec {x : Big} (hx : Canon x) :
    Limbs (view x).2.1 (view x).2.2 ∧ ((view x).2.2 = 0 ∨ (view x).2.1 ((view x).2.2 - 1) ≠ 0) ∧
      val x = sgnz (view x).1 (valF (view x).2.1 (view x).2.2) ∧ ((view x).1 = true → 0 < valF (view x).2.1 (view x).2.2) := by
  cases x with
  | S v =>
    simp only [view]
    set tmp : ℕ → ℕ := fun i => if i = 0 then v.natAbs % B else if i = 1 then v.natAbs / B else 0
    obtain ⟨tle, tval, ttop⟩ := trimLen_spec tmp 2
    have hv : v.natAbs < B * B := by
      have : |v| < 2 ^ 126 := hx
      have : (v.natAbs : ℤ) < 2 ^ 126 := by rw [Int.natCast_natAbs]; exact this
      have : v.natAbs < 2 ^ 126 := by exact_mod_cast this
      calc v.natAbs < 2 ^ 126 := this
        _ ≤ B * B := by rw [B_val]; norm_num
    have lim : Limbs tmp 2 := by
      intro i hi
      interval_cases i
      · simp [tmp]; exact Nat.mod_lt _ (by norm_num)
      · simp [tmp]; exact Nat.div_lt_of_lt_mul hv
    have v2 : valF tmp 2 = v.natAbs := by
      simp only [valF, sum_range_succ, sum_range_zero, tmp]; simp
      have := Nat.mod_add_div v.natAbs B; linarith
    refine ⟨fun i hi => lim i (by omega), ttop, ?_, ?_⟩
    · simp only [val, sgnz]; rw [tval, v2]
      split_ifs with h
      · simp at h; rw [Int.ofNat_natAbs_of_nonpos h.le]; ring
      · simp at h; exact (Int.natAbs_of_nonneg h).symm
    · intro h; simp at h; rw [tval, v2]; omega
  | M neg n w =>
    obtain ⟨h1, -, lim, top, -, -⟩ := hx
    refine ⟨lim, Or.inr top, rfl, fun _ => ?_⟩
    have := valF_ge_top (f := w) (show 0 < n by omega)
    have : 1 ≤ w (n - 1) := Nat.one_le_iff_ne_zero.mpr top
    have : 0 < B ^ (n - 1) := by positivity
    simp only [view]; nlinarith
  | L neg len mag =>
    obtain ⟨h1, lim, top, -⟩ := hx
    refine ⟨lim, Or.inr top, rfl, fun _ => ?_⟩
    have := valF_ge_top (f := mag) (show 0 < len by omega)
    have : 1 ≤ mag (len - 1) := Nat.one_le_iff_ne_zero.mpr top
    have : 0 < B ^ (len - 1) := by positivity
    simp only [view]; nlinarith

/-! ## The operations -/

/-- `neg`. -/
def neg : Big → Big
  | S v => S (-v)
  | M neg n w => M (!neg) n w
  | L neg len mag => L (!neg) len mag

/-- `add`'s general path: sign and magnitude. -/
def addSlow (x y : Big) : Big :=
  let vx := view x
  let vy := view y
  if vx.1 = vy.1 then
    fromMag vx.1 (if vy.2.2 ≤ vx.2.2 then addInto vx.2.1 vx.2.2 vy.2.1 vy.2.2 else addInto vy.2.1 vy.2.2 vx.2.1 vx.2.2)
      (max vx.2.2 vy.2.2 + 1)
  else
    match cmpMag vx.2.1 vx.2.2 vy.2.1 vy.2.2 with
    | .eq => S 0
    | .gt => fromMag vx.1 (subInto vx.2.1 vx.2.2 vy.2.1 vy.2.2) vx.2.2
    | .lt => fromMag vy.1 (subInto vy.2.1 vy.2.2 vx.2.1 vx.2.2) vy.2.2

/-- `add`: two inline values add in i128 (both under `2^126`, no overflow) and stay inline if the
sum fits. -/
def add (x y : Big) : Big :=
  match x, y with
  | S a, S b => if fits (a + b) then S (a + b) else addSlow x y
  | _, _ => addSlow x y

/-- `sub`. -/
def sub (x y : Big) : Big := add x (neg y)

def isZero : Big → Bool
  | S v => v == 0
  | _ => false

/-- `mul`. The fast path `checked_mul(a, b) = Some(p) ∧ fits(p)` is `fits(a·b)`: a product that
fits is within i128. -/
def mulSlow (x y : Big) : Big :=
  if isZero x || isZero y then S 0 else
    fromMag ((view x).1 != (view y).1) (mulInto (view x).2.1 (view x).2.2 (view y).2.1 (view y).2.2)
      ((view x).2.2 + (view y).2.2)

def mul (x y : Big) : Big :=
  match x, y with
  | S a, S b => if fits (a * b) then S (a * b) else mulSlow (S a) (S b)
  | _, _ => mulSlow x y

/-- `shl(bits)`; `used = 128 − leading_zeros(|v|)` is `Nat.size |v|`. -/
def shlSlow (x : Big) (bits : ℕ) : Big :=
  fromMag (view x).1 (shlInto (view x).2.1 (view x).2.2 bits) ((view x).2.2 + bits / 64 + 1)

def shl (x : Big) (bits : ℕ) : Big :=
  match x with
  | S v => if v = 0 then S 0 else if Nat.size v.natAbs + bits < 126 then S (v * 2 ^ bits) else shlSlow (S v) bits
  | _ => shlSlow x bits

/-- `signum`. -/
def signum : Big → ℤ
  | S v => Int.sign v
  | M neg _ _ => if neg then -1 else 1
  | L neg _ _ => if neg then -1 else 1

/-- `cmp_big`. -/
def cmpBig (x y : Big) : Ordering :=
  match x, y with
  | S a, S b => compare a b
  | _, _ => if signum (sub x y) = 1 then .gt else if signum (sub x y) = -1 then .lt else .eq

/-! ## Their specs -/

lemma sgnz_add_same (n : Bool) (a b : ℕ) : sgnz n (a + b) = sgnz n a + sgnz n b := by
  cases n <;> simp [sgnz] <;> ring

lemma sgnz_sub {an bn : Bool} (h : an ≠ bn) {a b : ℕ} (hle : b ≤ a) :
    sgnz an (a - b) = sgnz an a + sgnz bn b := by
  cases an <;> cases bn <;> simp_all [sgnz, Nat.cast_sub hle] <;> ring

lemma sgnz_mul (an bn : Bool) (a b : ℕ) : sgnz (an != bn) (a * b) = sgnz an a * sgnz bn b := by
  cases an <;> cases bn <;> simp [sgnz]

lemma canon_zero : Canon (S 0) := by simp [Canon]

theorem neg_spec {x : Big} (hx : Canon x) : Canon (neg x) ∧ val (neg x) = -val x := by
  cases x with
  | S v => exact ⟨by simpa [Canon, neg] using hx, rfl⟩
  | M n k w => exact ⟨hx, by cases n <;> simp [neg, val, sgnz]⟩
  | L n k w => exact ⟨hx, by cases n <;> simp [neg, val, sgnz]⟩

lemma len_le_of_lt {a b : ℕ → ℕ} {la lb : ℕ} (ha : Limbs a la) (tb : lb = 0 ∨ b (lb - 1) ≠ 0)
    (h : valF b lb < valF a la) : lb ≤ la := by
  by_contra hl; push Not at hl
  rcases tb with h0 | h0
  · omega
  · have h1 := valF_lt ha
    have h2 := valF_ge_top (f := b) (show 0 < lb by omega)
    have h3 : B ^ la ≤ B ^ (lb - 1) := Nat.pow_le_pow_right (by norm_num) (by omega)
    have h4 : 1 ≤ b (lb - 1) := Nat.one_le_iff_ne_zero.mpr h0
    have h5 : B ^ (lb - 1) ≤ b (lb - 1) * B ^ (lb - 1) := Nat.le_mul_of_pos_left _ h4
    omega

theorem addSlow_spec {x y : Big} (hx : Canon x) (hy : Canon y) :
    Canon (addSlow x y) ∧ val (addSlow x y) = val x + val y := by
  obtain ⟨la, ta, ex, -⟩ := view_spec hx
  obtain ⟨lb, tb, ey, -⟩ := view_spec hy
  unfold addSlow
  simp only
  generalize view x = vx at la ta ex
  generalize view y = vy at lb tb ey
  obtain ⟨an, af, al⟩ := vx
  obtain ⟨bn, bf, bl⟩ := vy
  simp only at *
  rw [ex, ey]
  by_cases hs : an = bn
  · rw [if_pos hs]
    subst hs
    split_ifs with hl
    · obtain ⟨v, lim, -⟩ := add_into_spec la lb hl
      obtain ⟨c, e⟩ := fromMag_spec (neg := an) (lim.mono_len (by omega) : Limbs _ (max al bl + 1))
      refine ⟨c, ?_⟩
      rw [e, show max al bl = al from max_eq_left hl, v, sgnz_add_same]
    · obtain ⟨v, lim, -⟩ := add_into_spec lb la (by omega)
      obtain ⟨c, e⟩ := fromMag_spec (neg := an) (lim.mono_len (by omega) : Limbs _ (max al bl + 1))
      refine ⟨c, ?_⟩
      rw [e, show max al bl = bl from max_eq_right (by omega), v, sgnz_add_same, add_comm]
  · rw [if_neg hs]
    have cm := cmp_mag_spec la lb ta tb
    generalize hc : cmpMag af al bf bl = c at cm
    cases c with
    | eq =>
      have := compare_eq_iff_eq.mp cm.symm
      refine ⟨canon_zero, ?_⟩
      simp only [val]; rw [this]
      cases an <;> cases bn <;> simp only [sgnz, Bool.false_eq_true, ite_false, ite_true] <;>
        first | exact absurd rfl hs | ring
    | gt =>
      have hgt : valF bf bl < valF af al := compare_gt_iff_gt.mp cm.symm
      have hl := len_le_of_lt la tb hgt
      obtain ⟨v, lim⟩ := sub_into_spec la lb hl hgt.le
      obtain ⟨c, e⟩ := fromMag_spec (neg := an) lim
      exact ⟨c, by rw [e, v, sgnz_sub hs hgt.le]⟩
    | lt =>
      have hlt : valF af al < valF bf bl := compare_lt_iff_lt.mp cm.symm
      have hl := len_le_of_lt lb ta hlt
      obtain ⟨v, lim⟩ := sub_into_spec lb la hl hlt.le
      obtain ⟨c, e⟩ := fromMag_spec (neg := bn) lim
      exact ⟨c, by rw [e, v, sgnz_sub (Ne.symm hs) hlt.le, add_comm]⟩

/-- **`add`.** -/
theorem add_spec {x y : Big} (hx : Canon x) (hy : Canon y) : Canon (add x y) ∧ val (add x y) = val x + val y := by
  cases x <;> cases y <;> simp only [add] <;> try exact addSlow_spec hx hy
  split_ifs with h
  · exact ⟨h, rfl⟩
  · exact addSlow_spec hx hy

/-- **`sub`.** -/
theorem sub_spec {x y : Big} (hx : Canon x) (hy : Canon y) : Canon (sub x y) ∧ val (sub x y) = val x - val y := by
  obtain ⟨c, e⟩ := neg_spec hy
  obtain ⟨c2, e2⟩ := add_spec hx c
  exact ⟨c2, by unfold sub; rw [e2, e]; ring⟩

lemma val_of_isZero {x : Big} (h : isZero x = true) : val x = 0 := by
  cases x with
  | S v => simp only [isZero, beq_iff_eq] at h; simp only [val, h]
  | M n k w => simp [isZero] at h
  | L n k w => simp [isZero] at h

theorem mulSlow_spec {x y : Big} (hx : Canon x) (hy : Canon y) :
    Canon (mulSlow x y) ∧ val (mulSlow x y) = val x * val y := by
  obtain ⟨la, -, ex, -⟩ := view_spec hx
  obtain ⟨lb, -, ey, -⟩ := view_spec hy
  unfold mulSlow
  split_ifs with hz
  · refine ⟨canon_zero, ?_⟩
    rcases Bool.or_eq_true _ _ ▸ hz with h | h
    · rw [val_of_isZero h]; simp [val]
    · rw [val_of_isZero h]; simp [val]
  · obtain ⟨v, lim, -⟩ := mul_into_spec la lb
    obtain ⟨c, e⟩ := fromMag_spec (neg := ((view x).1 != (view y).1)) lim
    exact ⟨c, by rw [e, v, sgnz_mul, ← ex, ← ey]⟩

/-- **`mul`.** -/
theorem mul_spec {x y : Big} (hx : Canon x) (hy : Canon y) : Canon (mul x y) ∧ val (mul x y) = val x * val y := by
  cases x <;> cases y <;> simp only [mul] <;> try exact mulSlow_spec hx hy
  split_ifs with h
  · exact ⟨h, rfl⟩
  · exact mulSlow_spec hx hy

theorem shlSlow_spec {x : Big} (hx : Canon x) (bits : ℕ) :
    Canon (shlSlow x bits) ∧ val (shlSlow x bits) = val x * 2 ^ bits := by
  obtain ⟨la, -, ex, -⟩ := view_spec hx
  obtain ⟨v, lim⟩ := shl_into_spec (bits := bits) la
  unfold shlSlow
  rw [show (view x).2.2 + bits / 64 + 1 = bits / 64 + (view x).2.2 + 1 by ring]
  obtain ⟨c, e⟩ := fromMag_spec (neg := (view x).1) lim
  refine ⟨c, ?_⟩
  rw [e, v, ex]
  cases (view x).1 <;> simp [sgnz]

/-- **`shl`.** (The Rust's `used + bits` is a u32 sum: `bits + 126 < 2^32` keeps it exact.) -/
theorem shl_spec {x : Big} (hx : Canon x) (bits : ℕ) :
    Canon (shl x bits) ∧ val (shl x bits) = val x * 2 ^ bits := by
  cases x with
  | S v =>
    simp only [shl]
    split_ifs with h0 h1
    · subst h0; exact ⟨canon_zero, by simp [val]⟩
    · refine ⟨?_, rfl⟩
      simp only [Canon]
      have hs : v.natAbs < 2 ^ Nat.size v.natAbs := Nat.lt_size_self _
      have : v.natAbs * 2 ^ bits < 2 ^ 126 := by
        calc v.natAbs * 2 ^ bits < 2 ^ Nat.size v.natAbs * 2 ^ bits :=
              Nat.mul_lt_mul_of_pos_right hs (by positivity)
          _ = 2 ^ (Nat.size v.natAbs + bits) := (pow_add _ _ _).symm
          _ ≤ 2 ^ 126 := Nat.pow_le_pow_right (by norm_num) h1.le
      rw [abs_mul, abs_pow, abs_two, ← Int.natCast_natAbs]
      exact_mod_cast this
    · exact shlSlow_spec hx bits
  | M n k w => exact shlSlow_spec hx bits
  | L n k w => exact shlSlow_spec hx bits

/-- **`signum` is the exact sign.** -/
theorem signum_spec {x : Big} (hx : Canon x) : signum x = Int.sign (val x) := by
  obtain ⟨-, -, ex, hpos⟩ := view_spec hx
  cases x with
  | S v => rfl
  | M n k w =>
    obtain ⟨h1, -, -, top, -, -⟩ := hx
    have : 0 < valF w k := by
      have := valF_ge_top (f := w) (show 0 < k by omega)
      have : 1 ≤ w (k - 1) := Nat.one_le_iff_ne_zero.mpr top
      have : 0 < B ^ (k - 1) := by positivity
      nlinarith
    cases n <;> simp [signum, val, sgnz, Int.sign_eq_one_of_pos (by exact_mod_cast this : (0 : ℤ) < valF w k)]
  | L n k w =>
    obtain ⟨h1, -, top, -⟩ := hx
    have : 0 < valF w k := by
      have := valF_ge_top (f := w) (show 0 < k by omega)
      have : 1 ≤ w (k - 1) := Nat.one_le_iff_ne_zero.mpr top
      have : 0 < B ^ (k - 1) := by positivity
      nlinarith
    cases n <;> simp [signum, val, sgnz, Int.sign_eq_one_of_pos (by exact_mod_cast this : (0 : ℤ) < valF w k)]

/-- **`is_zero`.** -/
theorem isZero_spec {x : Big} (hx : Canon x) : isZero x = true ↔ val x = 0 := by
  have := signum_spec hx
  cases x with
  | S v => simp [isZero, val]
  | M n k w => simp only [isZero, Bool.false_eq_true, false_iff]; intro h; rw [h] at this; cases n <;> simp [signum] at this
  | L n k w => simp only [isZero, Bool.false_eq_true, false_iff]; intro h; rw [h] at this; cases n <;> simp [signum] at this

/-- **`cmp_big`.** -/
theorem cmpBig_spec {x y : Big} (hx : Canon x) (hy : Canon y) : cmpBig x y = compare (val x) (val y) := by
  obtain ⟨c, e⟩ := sub_spec hx hy
  have sg := signum_spec c
  rw [e] at sg
  have gen : (if signum (sub x y) = 1 then Ordering.gt else if signum (sub x y) = -1 then Ordering.lt else Ordering.eq) =
      compare (val x) (val y) := by
    rw [sg]
    rcases lt_trichotomy (val x) (val y) with h | h | h
    · rw [Int.sign_eq_neg_one_of_neg (by omega), compare_lt_iff_lt.mpr h]; simp
    · rw [h, sub_self, Int.sign_zero, compare_eq_iff_eq.mpr rfl]; simp
    · rw [Int.sign_eq_one_of_pos (by omega), compare_gt_iff_gt.mpr h]; simp
  cases x <;> cases y <;> first | rfl | exact gen

end Big

end IEEE754.Exact
