/-
The oracle's interval arithmetic encloses (SplineSpec/Oracle.lean, `RI`): a real in each operand gives
a real in the result, for +, -, ×, ÷ and √, with the outward rounding. And since the slope system's
rows, residuals and Hermite pieces are the same definitions at any number type, the intervals the
oracle computes for them at the parameter boxes enclose their values at the true (irrational)
centripetal parameters (`rows_in`, `residual_in`, `piecesOf_in`). With `slopes_bound` this is the
through-spline's bound: the oracle's `bm` bounds the slopes' distance from the spec's, and its `eb`
the Bezier points' (then `bern_close`, SplineSpec/Cert.lean, bounds the curves').
-/
import SplineSpec.Oracle
import SplineSpec.Interp
import Mathlib.Data.Rat.Floor
import Mathlib.Data.Real.Sqrt
import Mathlib.Tactic

namespace SplineSpec.Oracle

open SplineSpec

/-- `x` lies in `I`, once `I` is good. -/
def In (x : ℝ) (I : RI) : Prop := I.ok = true → (I.lo : ℝ) ≤ x ∧ x ≤ (I.hi : ℝ)

theorem ratMin_cast (a b : ℚ) : ((ratMin a b : ℚ) : ℝ) = min (a : ℝ) (b : ℝ) := by
  unfold ratMin; split_ifs with h
  · rw [min_eq_left (by exact_mod_cast h.le)]
  · rw [min_eq_right (by exact_mod_cast (not_lt.1 h))]

theorem ratMax_cast (a b : ℚ) : ((ratMax a b : ℚ) : ℝ) = max (a : ℝ) (b : ℝ) := by
  unfold ratMax; split_ifs with h
  · rw [max_eq_right (by exact_mod_cast h.le)]
  · rw [max_eq_left (by exact_mod_cast (not_lt.1 h))]

theorem ratAbs_cast (a : ℚ) : ((ratAbs a : ℚ) : ℝ) = |(a : ℝ)| := by
  unfold ratAbs; split_ifs with h
  · rw [abs_of_neg (by exact_mod_cast h)]; push_cast; ring
  · rw [abs_of_nonneg (by exact_mod_cast (not_lt.1 h))]

theorem down_le (x : ℚ) : ((down x : ℚ) : ℝ) ≤ x := by
  unfold down
  have hp : (0 : ℚ) < ((2 ^ PREC : ℕ) : ℚ) := by positivity
  have : ((x * ((2 ^ PREC : ℕ) : ℚ)).floor : ℚ) / ((2 ^ PREC : ℕ) : ℚ) ≤ x := by
    rw [div_le_iff₀ hp]; exact Int.floor_le (x * ((2 ^ PREC : ℕ) : ℚ))
  exact_mod_cast this

theorem le_up (x : ℚ) : (x : ℝ) ≤ ((up x : ℚ) : ℝ) := by
  unfold up
  have := down_le (-x)
  push_cast at this ⊢
  linarith

theorem in_pt (x : ℚ) : In (x : ℝ) (RI.pt x) := fun _ => ⟨le_rfl, le_rfl⟩

theorem in_of {x : ℝ} {lo hi : ℚ} (h1 : (lo : ℝ) ≤ x) (h2 : x ≤ hi) : In x (RI.of lo hi) :=
  fun _ => ⟨(down_le lo).trans h1, h2.trans (le_up hi)⟩

theorem in_zero : In 0 (0 : RI) := by
  show In 0 (RI.pt 0); have h := in_pt 0; rwa [Rat.cast_zero] at h

theorem in_one : In 1 (1 : RI) := by
  show In 1 (RI.pt 1); have h := in_pt 1; rwa [Rat.cast_one] at h

theorem add_def (I J : RI) : I + J = ⟨down (I.lo + J.lo), up (I.hi + J.hi), I.ok && J.ok⟩ := rfl
theorem sub_def (I J : RI) : I - J = ⟨down (I.lo - J.hi), up (I.hi - J.lo), I.ok && J.ok⟩ := rfl
theorem mul_def (I J : RI) : I * J =
    let c := [I.lo * J.lo, I.lo * J.hi, I.hi * J.lo, I.hi * J.hi]
    ⟨down (c.foldl ratMin (I.lo * J.lo)), up (c.foldl ratMax (I.lo * J.lo)), I.ok && J.ok⟩ := rfl
theorem div_def (I J : RI) : I / J =
    if 0 < J.lo ∨ J.hi < 0 then
      let c := [I.lo / J.lo, I.lo / J.hi, I.hi / J.lo, I.hi / J.hi]
      ⟨down (c.foldl ratMin (I.lo / J.lo)), up (c.foldl ratMax (I.lo / J.lo)), I.ok && J.ok⟩
    else ⟨0, 0, false⟩ := rfl

theorem in_add {x y : ℝ} {I J : RI} (hx : In x I) (hy : In y J) : In (x + y) (I + J) := by
  intro hok
  rw [add_def] at hok ⊢
  have h := Bool.and_eq_true_iff.1 hok
  obtain ⟨a1, a2⟩ := hx h.1
  obtain ⟨b1, b2⟩ := hy h.2
  constructor
  · refine (down_le _).trans ?_; push_cast; linarith
  · refine le_trans ?_ (le_up _); push_cast; linarith

theorem in_sub {x y : ℝ} {I J : RI} (hx : In x I) (hy : In y J) : In (x - y) (I - J) := by
  intro hok
  rw [sub_def] at hok ⊢
  have h := Bool.and_eq_true_iff.1 hok
  obtain ⟨a1, a2⟩ := hx h.1
  obtain ⟨b1, b2⟩ := hy h.2
  constructor
  · refine (down_le _).trans ?_; push_cast; linarith
  · refine le_trans ?_ (le_up _); push_cast; linarith

/-- A product of reals in two intervals is at least one corner product and at most one. -/
theorem mul_corners {a b c d x y : ℝ} (hx1 : a ≤ x) (hx2 : x ≤ b) (hy1 : c ≤ y) (hy2 : y ≤ d) :
    (a * c ≤ x * y ∨ a * d ≤ x * y ∨ b * c ≤ x * y ∨ b * d ≤ x * y) ∧
    (x * y ≤ a * c ∨ x * y ≤ a * d ∨ x * y ≤ b * c ∨ x * y ≤ b * d) := by
  constructor
  · rcases le_total 0 y with hy | hy
    · rcases le_total 0 a with ha | ha
      · left; nlinarith
      · right; left; nlinarith
    · rcases le_total 0 b with hb | hb
      · right; right; left; nlinarith
      · right; right; right; nlinarith
  · rcases le_total 0 y with hy | hy
    · rcases le_total 0 b with hb | hb
      · right; right; right; nlinarith
      · right; right; left; nlinarith
    · rcases le_total 0 a with ha | ha
      · right; left; nlinarith
      · left; nlinarith

/-- The oracle's four-corner minimum (a fold from its first corner) is at most each corner, and
the maximum at least each. -/
theorem fold4_min (p q r s : ℚ) :
    let m : ℝ := (([p, q, r, s].foldl ratMin p : ℚ) : ℝ)
    m ≤ p ∧ m ≤ q ∧ m ≤ r ∧ m ≤ s := by
  simp only [List.foldl_cons, List.foldl_nil, ratMin_cast]
  refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [min_le_iff]

theorem fold4_max (p q r s : ℚ) :
    let m : ℝ := (([p, q, r, s].foldl ratMax p : ℚ) : ℝ)
    (p : ℝ) ≤ m ∧ (q : ℝ) ≤ m ∧ (r : ℝ) ≤ m ∧ (s : ℝ) ≤ m := by
  simp only [List.foldl_cons, List.foldl_nil, ratMax_cast]
  refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [le_max_iff]

theorem in_mul {x y : ℝ} {I J : RI} (hx : In x I) (hy : In y J) : In (x * y) (I * J) := by
  intro hok
  rw [mul_def] at hok ⊢
  have h := Bool.and_eq_true_iff.1 hok
  obtain ⟨a1, a2⟩ := hx h.1
  obtain ⟨b1, b2⟩ := hy h.2
  obtain ⟨lo, hi⟩ := mul_corners a1 a2 b1 b2
  obtain ⟨m1, m2, m3, m4⟩ := fold4_min (I.lo * J.lo) (I.lo * J.hi) (I.hi * J.lo) (I.hi * J.hi)
  obtain ⟨M1, M2, M3, M4⟩ := fold4_max (I.lo * J.lo) (I.lo * J.hi) (I.hi * J.lo) (I.hi * J.hi)
  push_cast at m1 m2 m3 m4 M1 M2 M3 M4
  constructor
  · refine (down_le _).trans ?_
    rcases lo with l | l | l | l <;> linarith
  · refine le_trans ?_ (le_up _)
    rcases hi with l | l | l | l <;> linarith

theorem in_div {x y : ℝ} {I J : RI} (hx : In x I) (hy : In y J) : In (x / y) (I / J) := by
  intro hok
  rw [div_def] at hok ⊢
  split_ifs at hok ⊢ with hs
  · have h := Bool.and_eq_true_iff.1 hok
    obtain ⟨a1, a2⟩ := hx h.1
    obtain ⟨b1, b2⟩ := hy h.2
    -- 1/y lies in [1/hi, 1/lo]: J has one sign
    have hinv : (1 / (J.hi : ℝ)) ≤ 1 / y ∧ 1 / y ≤ 1 / (J.lo : ℝ) := by
      rcases hs with hp | hn
      · have hp' : (0 : ℝ) < J.lo := by exact_mod_cast hp
        exact ⟨one_div_le_one_div_of_le (by linarith) b2, one_div_le_one_div_of_le hp' b1⟩
      · have hn' : (J.hi : ℝ) < 0 := by exact_mod_cast hn
        exact ⟨one_div_le_one_div_of_neg_of_le hn' b2, one_div_le_one_div_of_neg_of_le (by linarith) b1⟩
    obtain ⟨lo, hi⟩ := mul_corners a1 a2 hinv.1 hinv.2
    obtain ⟨m1, m2, m3, m4⟩ := fold4_min (I.lo / J.lo) (I.lo / J.hi) (I.hi / J.lo) (I.hi / J.hi)
    obtain ⟨M1, M2, M3, M4⟩ := fold4_max (I.lo / J.lo) (I.lo / J.hi) (I.hi / J.lo) (I.hi / J.hi)
    have e : ∀ p q : ℚ, ((p / q : ℚ) : ℝ) = (p : ℝ) * (1 / (q : ℝ)) := fun p q => by push_cast; ring
    simp only [e] at m1 m2 m3 m4 M1 M2 M3 M4
    rw [div_eq_mul_one_div x y]
    constructor
    · refine (down_le _).trans ?_
      rcases lo with l | l | l | l <;> linarith
    · refine le_trans ?_ (le_up _)
      rcases hi with l | l | l | l <;> linarith

/-! ### Square roots -/

theorem sqrt_scaled (q : ℚ) (hq : 0 ≤ q) (k : ℕ) (hk : 0 < k) :
    Real.sqrt (q : ℝ) = Real.sqrt ((q.num.toNat * q.den * k * k : ℕ) : ℝ) / ((q.den * k : ℕ) : ℝ) := by
  have hnum : 0 ≤ q.num := Rat.num_nonneg.2 hq
  have hden : (0 : ℝ) < q.den := by exact_mod_cast q.den_pos
  have hk' : (0 : ℝ) < k := by exact_mod_cast hk
  have hcast : ((q.num.toNat : ℕ) : ℝ) = (q.num : ℝ) := by
    have := Int.toNat_of_nonneg hnum
    exact_mod_cast this
  have hq' : (q : ℝ) = (q.num : ℝ) / q.den := by
    rw [← Rat.cast_intCast, ← Rat.cast_natCast, ← Rat.cast_div, Rat.num_div_den]
  push_cast
  rw [hcast, hq']
  have hn : (0 : ℝ) ≤ q.num := by exact_mod_cast hnum
  rw [show (q.num : ℝ) * q.den * k * k = ((q.num : ℝ) * q.den) * (k * k) by ring,
    Real.sqrt_mul (by positivity), Real.sqrt_mul_self hk'.le]
  rw [show (q.num : ℝ) / q.den = ((q.num : ℝ) * q.den) / (q.den * q.den) by field_simp,
    Real.sqrt_div' _ (by positivity), Real.sqrt_mul_self hden.le]
  field_simp

theorem in_sqrt {x : ℝ} {I : RI} (hx : In x I) (h0 : 0 ≤ x) : In (Real.sqrt x) (sqrtI I) := by
  intro hok
  simp only [sqrtI, Bool.and_eq_true, decide_eq_true_eq] at hok ⊢
  obtain ⟨a1, a2⟩ := hx hok.1
  have hk : 0 < (2 ^ 80 : ℕ) := by positivity
  constructor
  · split_ifs with hl
    · simp only [Rat.cast_zero]; exact Real.sqrt_nonneg _
    · have hpos : 0 ≤ I.lo := le_of_lt (not_le.1 hl)
      have e := sqrt_scaled I.lo hpos (2 ^ 80) hk
      have : ((((Nat.sqrt (I.lo.num.toNat * I.lo.den * 2 ^ 80 * 2 ^ 80) : ℕ) : ℚ) /
          ((I.lo.den * 2 ^ 80 : ℕ) : ℚ) : ℚ) : ℝ) ≤ Real.sqrt (I.lo : ℝ) := by
        rw [e]; push_cast
        apply div_le_div_of_nonneg_right _ (by positivity)
        have := Real.nat_sqrt_le_real_sqrt (a := I.lo.num.toNat * I.lo.den * 2 ^ 80 * 2 ^ 80)
        push_cast at this; exact this
      exact this.trans (Real.sqrt_le_sqrt a1)
  · set h := ratMax I.hi 0
    have hh0 : (0 : ℚ) ≤ h := by
      have := ratMax_cast I.hi 0; have : (0 : ℝ) ≤ (h : ℝ) := by rw [this, Rat.cast_zero]; exact le_max_right (I.hi : ℝ) 0
      exact_mod_cast this
    have hxh : x ≤ (h : ℝ) := by rw [ratMax_cast]; exact a2.trans (le_max_left (I.hi : ℝ) ((0 : ℚ) : ℝ))
    have e := sqrt_scaled h hh0 (2 ^ 80) hk
    refine (Real.sqrt_le_sqrt hxh).trans ?_
    rw [e]; push_cast
    apply div_le_div_of_nonneg_right _ (by positivity)
    have := Real.real_sqrt_le_nat_sqrt_succ (a := h.num.toNat * h.den * 2 ^ 80 * 2 ^ 80)
    push_cast at this; exact this

/-! ### Magnitudes and distances the oracle reads off an interval -/

theorem in_abs_le {x : ℝ} {I : RI} (hx : In x I) (hok : I.ok = true) : |x| ≤ (I.absHi : ℝ) := by
  obtain ⟨a1, a2⟩ := hx hok
  simp only [RI.absHi, ratMax_cast, ratAbs_cast]
  rw [abs_le]; constructor
  · have := neg_abs_le (I.lo : ℝ); have := le_max_left |(I.lo : ℝ)| |(I.hi : ℝ)|; linarith
  · have := le_abs_self (I.hi : ℝ); have := le_max_right |(I.lo : ℝ)| |(I.hi : ℝ)|; linarith

theorem in_abs_ge {x : ℝ} {I : RI} (hx : In x I) (hok : I.ok = true) : (I.absLo : ℝ) ≤ |x| := by
  obtain ⟨a1, a2⟩ := hx hok
  unfold RI.absLo
  split_ifs with h1 h2
  · have : (0 : ℝ) < I.lo := by exact_mod_cast h1
    rw [abs_of_pos (by linarith)]; exact a1
  · have : (I.hi : ℝ) < 0 := by exact_mod_cast h2
    rw [abs_of_neg (by linarith)]; push_cast; linarith
  · simp

theorem in_far {x : ℝ} {I : RI} (hx : In x I) (hok : I.ok = true) (r : ℚ) :
    |(r : ℝ) - x| ≤ (I.far r : ℝ) := by
  obtain ⟨a1, a2⟩ := hx hok
  simp only [RI.far, ratMax_cast, ratAbs_cast]
  push_cast
  rw [abs_le]; constructor
  · have := le_abs_self ((r : ℝ) - I.lo); have := le_max_left |(r : ℝ) - I.lo| |(r : ℝ) - I.hi|
    have := neg_abs_le ((r : ℝ) - I.lo); linarith [neg_abs_le ((r : ℝ) - I.hi), le_max_right |(r : ℝ) - I.lo| |(r : ℝ) - I.hi|]
  · have := neg_abs_le ((r : ℝ) - I.hi); have := le_max_right |(r : ℝ) - I.lo| |(r : ℝ) - I.hi|
    linarith [le_abs_self ((r : ℝ) - I.lo), le_max_left |(r : ℝ) - I.lo| |(r : ℝ) - I.hi|]

/-! ### The slope system in intervals -/

/-- An end condition in reals and its interval counterpart. -/
inductive EndRel : EndCond ℝ → EndCond RI → Prop
  | natural : EndRel .natural .natural
  | slope {v : ℝ} {V : RI} : In v V → EndRel (.slope v) (.slope V)

inductive EndsRel : Ends ℝ → Ends RI → Prop
  | periodic : EndsRel .periodic .periodic
  | open_ {s f S F} : EndRel s S → EndRel f F → EndsRel (.open_ s f) (.open_ S F)

/-- Closes a goal `In e E` where `E` is `e` built from intervals by the same operations. -/
macro "interval_in" : tactic => `(tactic| repeat (first
  | assumption | apply in_add | apply in_sub | apply in_mul | apply in_div
  | exact in_one | exact in_zero | apply_assumption))

theorem rows_in {τ q : ℕ → ℝ} {τI qI : ℕ → RI} (hτ : ∀ i, In (τ i) (τI i)) (hq : ∀ i, In (q i) (qI i))
    (m : ℕ) {e : Ends ℝ} {eI : Ends RI} (he : EndsRel e eI) (i : ℕ) :
    In (rows τ q m e i).lo (rows τI qI m eI i).lo ∧ In (rows τ q m e i).di (rows τI qI m eI i).di ∧
    In (rows τ q m e i).up (rows τI qI m eI i).up ∧ In (rows τ q m e i).rhs (rows τI qI m eI i).rhs := by
  have hh : ∀ j, In (τ (j + 1) - τ j) (τI (j + 1) - τI j) := fun j => in_sub (hτ _) (hτ _)
  have hd : ∀ j, In ((q (j + 1) - q j) / (τ (j + 1) - τ j)) ((qI (j + 1) - qI j) / (τI (j + 1) - τI j)) :=
    fun j => in_div (in_sub (hq _) (hq _)) (hh j)
  have h2 : In (1 + 1 : ℝ) (1 + 1 : RI) := in_add in_one in_one
  have h3 : In (1 + 1 + 1 : ℝ) (1 + 1 + 1 : RI) := in_add h2 in_one
  have mid : ∀ a b, In (τ (a + 1) - τ a) (τI (a + 1) - τI a) → In (τ (b + 1) - τ b) (τI (b + 1) - τI b) →
      In ((1 + 1) * ((τ (b + 1) - τ b) + (τ (a + 1) - τ a))) ((1 + 1) * ((τI (b + 1) - τI b) + (τI (a + 1) - τI a))) ∧
      In ((1 + 1 + 1) * ((τ (a + 1) - τ a) * ((q (b + 1) - q b) / (τ (b + 1) - τ b))
          + (τ (b + 1) - τ b) * ((q (a + 1) - q a) / (τ (a + 1) - τ a))))
        ((1 + 1 + 1) * ((τI (a + 1) - τI a) * ((qI (b + 1) - qI b) / (τI (b + 1) - τI b))
          + (τI (b + 1) - τI b) * ((qI (a + 1) - qI a) / (τI (a + 1) - τI a)))) :=
    fun a b ha hb => ⟨in_mul h2 (in_add hb ha),
      in_mul h3 (in_add (in_mul ha (hd b)) (in_mul hb (hd a)))⟩
  cases he with
  | periodic =>
    simp only [rows]
    split_ifs
    · exact ⟨hh 0, (mid 0 (m - 1) (hh _) (hh _)).1, hh _, (mid 0 (m - 1) (hh _) (hh _)).2⟩
    · exact ⟨hh i, (mid i (i - 1) (hh _) (hh _)).1, hh _, (mid i (i - 1) (hh _) (hh _)).2⟩
  | open_ hs hf =>
    simp only [rows]
    split_ifs
    · cases hs with
      | natural => exact ⟨in_zero, h2, in_one, in_mul h3 (hd 0)⟩
      | slope hv => exact ⟨in_zero, in_one, in_zero, hv⟩
    · cases hf with
      | natural => exact ⟨in_one, h2, in_zero, in_mul h3 (hd _)⟩
      | slope hv => exact ⟨in_zero, in_one, in_zero, hv⟩
    · exact ⟨hh i, (mid i (i - 1) (hh _) (hh _)).1, hh _, (mid i (i - 1) (hh _) (hh _)).2⟩

theorem unknowns_rel {m : ℕ} {e : Ends ℝ} {eI : Ends RI} (he : EndsRel e eI) :
    unknowns m e = unknowns m eI := by cases he <;> rfl

theorem residual_in {τ q x : ℕ → ℝ} {τI qI xI : ℕ → RI} (hτ : ∀ i, In (τ i) (τI i))
    (hq : ∀ i, In (q i) (qI i)) (hx : ∀ i, In (x i) (xI i)) (m : ℕ) {e : Ends ℝ} {eI : Ends RI}
    (he : EndsRel e eI) (i : ℕ) : In (residual τ q m e x i) (residual τI qI m eI xI i) := by
  obtain ⟨h1, h2, h3, h4⟩ := rows_in hτ hq m he i
  simp only [residual, ← unknowns_rel (m := m) he]
  exact in_sub (in_add (in_add (in_mul h1 (hx _)) (in_mul h2 (hx _))) (in_mul h3 (hx _))) h4

theorem slopeAt_in {x : ℕ → ℝ} {xI : ℕ → RI} (hx : ∀ i, In (x i) (xI i)) (m : ℕ) {e : Ends ℝ}
    {eI : Ends RI} (he : EndsRel e eI) (i : ℕ) : In (slopeAt m e x i) (slopeAt m eI xI i) := by
  cases he <;> simp only [slopeAt] <;> first | (split_ifs <;> exact hx _) | exact hx _

theorem piecesOf_in {τ q x : ℕ → ℝ} {τI qI xI : ℕ → RI} (hτ : ∀ i, In (τ i) (τI i))
    (hq : ∀ i, In (q i) (qI i)) (hx : ∀ i, In (x i) (xI i)) (m : ℕ) {e : Ends ℝ} {eI : Ends RI}
    (he : EndsRel e eI) (i l : ℕ) : In (piecesOf τ q m e x i l) (piecesOf τI qI m eI xI i l) := by
  have s0 := slopeAt_in hx m he i
  have s1 := slopeAt_in hx m he (i + 1)
  have hh := in_sub (hτ (i + 1)) (hτ i)
  have h3 : In (1 + 1 + 1 : ℝ) (1 + 1 + 1 : RI) := in_add (in_add in_one in_one) in_one
  simp only [piecesOf, hermite]
  rcases l with _ | _ | _ | l
  · exact hq i
  · exact in_add (hq i) (in_div (in_mul s0 hh) h3)
  · exact in_sub (hq (i + 1)) (in_div (in_mul s1 hh) h3)
  · exact hq (i + 1)

/-- `slopeAt` at `i ≤ m` reads only unknowns. -/
theorem slopeAt_in' {x : ℕ → ℝ} {xI : ℕ → RI} {m : ℕ} {e : Ends ℝ} {eI : Ends RI} (he : EndsRel e eI)
    (hm : 1 ≤ m) (hx : ∀ j < unknowns m e, In (x j) (xI j)) {i : ℕ} (hi : i ≤ m) :
    In (slopeAt m e x i) (slopeAt m eI xI i) := by
  cases he with
  | periodic =>
    simp only [slopeAt]
    split_ifs with h
    · exact hx 0 (by simp [unknowns]; omega)
    · exact hx i (by simp [unknowns]; omega)
  | open_ _ _ => exact hx i (by simp [unknowns]; omega)

/-- **The through-spline certificate.** Say the oracle's interval residuals at the Rust's slopes X
enclose the true residuals, its margins are below the true ones, and `bm` bounds each residual
over its margin, as `checkThrough` checks. Then the spec's slopes are within `bm` of X, so the
spec's pieces lie in the intervals made from `[X - bm, X + bm]`, and each of the Rust's Bezier points
is within that interval's far end of the spec's. -/
theorem through_cert {τ q : ℕ → ℝ} {m : ℕ} {e : Ends ℝ} (hd : DataOk τ q m e) (X : ℕ → ℚ) (bm : ℚ)
    (margin : ℕ → ℚ)
    (hmargin : ∀ i < unknowns m e, (margin i : ℝ) ≤ |(rows τ q m e i).di| - |(rows τ q m e i).lo|
      - |(rows τ q m e i).up|)
    (hmpos : ∀ i < unknowns m e, 0 < margin i)
    (hres : ∀ i < unknowns m e, |residual τ q m e (fun j => (X j : ℝ)) i| ≤ (bm : ℝ) * margin i)
    {τI qI : ℕ → RI} {eI : Ends RI} (hτ : ∀ i, In (τ i) (τI i)) (hq : ∀ i, In (q i) (qI i))
    (he : EndsRel e eI) :
    ∃ y : ℕ → ℝ, (∀ i < unknowns m e, residual τ q m e y i = 0) ∧
      IsInterp τ q m e (piecesOf τ q m e y) ∧
      ∀ i < m, ∀ l, In (piecesOf τ q m e y i l)
        (piecesOf τI qI m eI (fun j => RI.of (X j - bm) (X j + bm)) i l) := by
  obtain ⟨y, hy⟩ := slopes_exist (q := q) hd.toSysOk
  have hb := slopes_bound hd.toSysOk (fun j => (X j : ℝ)) y bm hy (fun i hi => by
    refine (hres i hi).trans ?_
    have hbm : (0 : ℝ) ≤ bm := by
      have := (abs_nonneg _).trans (hres i hi)
      have hm : (0 : ℝ) < margin i := by exact_mod_cast hmpos i hi
      nlinarith
    exact mul_le_mul_of_nonneg_left (hmargin i hi) hbm)
  refine ⟨y, hy, exists_of_slopes hd y hy, fun i hi l => ?_⟩
  -- every slope the pieces read is an unknown's (or, periodic, the first's)
  have hyin : ∀ j, j < unknowns m e → In (y j) (RI.of (X j - bm) (X j + bm)) := by
    intro j hj
    have := hb j hj
    rw [abs_le] at this
    exact in_of (by push_cast; linarith) (by push_cast; linarith)
  have hm : 1 ≤ m := by have := hd.size; cases e <;> simp at this <;> omega
  have s0 := slopeAt_in' he hm hyin (i := i) (by omega)
  have s1 := slopeAt_in' he hm hyin (i := i + 1) (by omega)
  have hh := in_sub (hτ (i + 1)) (hτ i)
  have h3 : In (1 + 1 + 1 : ℝ) (1 + 1 + 1 : RI) := in_add (in_add in_one in_one) in_one
  simp only [piecesOf, hermite]
  rcases l with _ | _ | _ | l
  · exact hq i
  · exact in_add (hq i) (in_div (in_mul s0 hh) h3)
  · exact in_sub (hq (i + 1)) (in_div (in_mul s1 hh) h3)
  · exact hq (i + 1)

end SplineSpec.Oracle
