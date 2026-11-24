/-
**Knot insertion leaves the curve unchanged** (Boehm), T6: inserting x into `[u s, u (s+1))`, with
the new poles `Q_i = α_i P_i + (1 - α_i) P_(i-1)`, `α_i = (x - u_i) / (u_(i+p) - u_i)` for
`s - p < i ≤ s` (1 below, 0 above), gives the same curve on every span (`insert_curve`). This is what
`Spline::from_nurbs` does (`insert_knot`) to reach its Bezier pieces, so they are the spec's curve.

The proof is the basis identity `N_(i,k) = α^k_i N̄_(i,k) + (1 - α^k_(i+1)) N̄_(i+1,k)` by induction
on k (`insert_basis`). Its step comes down to two coefficient identities, `coefA` and `coefB`,
case by case on where i lies against s.
-/
import SplineSpec.Deriv

open Finset

namespace SplineSpec

/-- The knots with x inserted after `u s`. -/
noncomputable def ins (u : ℕ → ℝ) (s : ℕ) (x : ℝ) (i : ℕ) : ℝ :=
  if i ≤ s then u i else if i = s + 1 then x else u (i - 1)

/-- The insertion ratio at degree k. -/
noncomputable def alpha (u : ℕ → ℝ) (s : ℕ) (x : ℝ) (k i : ℕ) : ℝ :=
  if i + k ≤ s then 1 else if s + 1 ≤ i then 0 else (x - u i) / (u (i + k) - u i)

section
variable (u : ℕ → ℝ) (s : ℕ) (x : ℝ)

theorem ins_le {i : ℕ} (h : i ≤ s) : ins u s x i = u i := by simp [ins, h]
theorem ins_mid : ins u s x (s + 1) = x := by simp [ins]
theorem ins_gt {i : ℕ} (h : s + 2 ≤ i) : ins u s x i = u (i - 1) := by
  have h1 : ¬ i ≤ s := by omega
  have h2 : i ≠ s + 1 := by omega
  simp [ins, h1, h2]

theorem ins_mono (hu : Monotone u) (h0 : u s ≤ x) (h1 : x ≤ u (s + 1)) : Monotone (ins u s x) := by
  apply monotone_nat_of_le_succ
  intro i
  rcases lt_trichotomy i s with h | h | h
  · rw [ins_le u s x (by omega), ins_le u s x (by omega)]; exact hu (by omega)
  · subst h; rw [ins_le u i x le_rfl, ins_mid]; exact h0
  · rcases Nat.lt_or_ge i (s + 1) with h' | h'
    · omega
    · by_cases he : i = s + 1
      · subst he; rw [ins_mid, ins_gt u s x (by omega)]; simpa using h1
      · rw [ins_gt u s x (by omega), ins_gt u s x (by omega)]; exact hu (by omega)
end

/-- The coefficient of `N̄_(i,k)`: `ω_i α^k_i = α^(k+1)_i ω̄_i`, unless `N̄_(i,k)`'s support has
no length (`ins u s x i = ins u s x (i+k+1)`). -/
theorem coefA (u : ℕ → ℝ) (s : ℕ) (x : ℝ) (k i : ℕ) (t : ℝ)
    (hd : ins u s x i ≠ ins u s x (i + k + 1)) :
    omega u i (k + 1) t * alpha u s x k i = alpha u s x (k + 1) i * omega (ins u s x) i (k + 1) t := by
  simp only [omega_eq, alpha]
  by_cases c1 : i + k + 1 ≤ s
  · have a : i + k ≤ s := by omega
    have a2 : i + (k + 1) ≤ s := by omega
    rw [if_pos a, if_pos a2, ins_le u s x (by omega), ins_le u s x a2]
    ring
  by_cases c2 : i + k = s
  · have a2 : ¬ i + (k + 1) ≤ s := by omega
    have a3 : ¬ s + 1 ≤ i := by omega
    rw [if_pos (le_of_eq c2), if_neg a2, if_neg a3, ins_le u s x (by omega)]
    have e : i + (k + 1) = s + 1 := by omega
    rw [e, ins_mid]
    have hx : x - u i ≠ 0 := by
      intro h; apply hd
      rw [ins_le u s x (by omega), show i + k + 1 = s + 1 by omega, ins_mid]; linarith
    field_simp
  by_cases c4 : s + 1 ≤ i
  · have a : ¬ i + k ≤ s := by omega
    have a2 : ¬ i + (k + 1) ≤ s := by omega
    rw [if_neg a, if_pos c4, if_neg a2, if_pos c4]; ring
  -- i ≤ s < i + k: both middle; ins (i + k + 1) = u (i + k)
  have a : ¬ i + k ≤ s := by omega
  have a2 : ¬ i + (k + 1) ≤ s := by omega
  rw [if_neg a, if_neg c4, if_neg a2, if_neg c4, ins_le u s x (by omega), ins_gt u s x (by omega)]
  have e : i + (k + 1) - 1 = i + k := by omega
  rw [e]
  ring

/-- The coefficient across: `(1 - ω_i)(1 - α^k_(i+1)) = (1 - α^(k+1)_i)(1 - ω̄_(i+1))`, for x in
`[u s, u (s+1))` of nondecreasing knots. -/
theorem coefB (u : ℕ → ℝ) (hu : Monotone u) (s : ℕ) (x : ℝ) (h0 : u s ≤ x) (h1 : x < u (s + 1))
    (k i : ℕ) (t : ℝ) :
    (1 - omega u i (k + 1) t) * (1 - alpha u s x k (i + 1))
      = (1 - alpha u s x (k + 1) i) * (1 - omega (ins u s x) (i + 1) (k + 1) t) := by
  simp only [omega_eq, alpha]
  by_cases b1 : i + k + 1 ≤ s
  · rw [if_pos (by omega : i + 1 + k ≤ s), if_pos (by omega : i + (k + 1) ≤ s)]; ring
  by_cases b5 : s + 1 ≤ i
  · rw [if_neg (by omega : ¬ i + 1 + k ≤ s), if_pos (by omega : s + 1 ≤ i + 1),
      if_neg (by omega : ¬ i + (k + 1) ≤ s), if_pos b5,
      ins_gt u s x (by omega : s + 2 ≤ i + 1), ins_gt u s x (by omega : s + 2 ≤ i + 1 + (k + 1))]
    have e1 : i + 1 - 1 = i := by omega
    have e2 : i + 1 + (k + 1) - 1 = i + (k + 1) := by omega
    rw [e1, e2]; ring
  -- now i ≤ s < i + k + 1: α^(k+1)_i is the middle ratio over [u i, u (i+k+1)]
  have hm : ¬ i + (k + 1) ≤ s := by omega
  have hi : ¬ s + 1 ≤ i := by omega
  rw [if_neg hm, if_neg hi]
  have hD1 : 0 < u (i + (k + 1)) - u i := by
    have := hu (show i ≤ s by omega); have := hu (show s + 1 ≤ i + (k + 1) by omega); linarith
  by_cases b4 : i = s
  · subst b4
    rw [if_neg (by omega : ¬ i + 1 + k ≤ i), if_pos (le_refl (i + 1)), ins_mid,
      ins_gt u i x (by omega : i + 2 ≤ i + 1 + (k + 1))]
    have e2 : i + 1 + (k + 1) - 1 = i + (k + 1) := by omega
    rw [e2]
    have hD : 0 < u (i + (k + 1)) - x := by have := hu (show i + 1 ≤ i + (k + 1) by omega); linarith
    field_simp
    ring
  -- i < s: α^k_(i+1) is the middle ratio over [u (i+1), u (i+1+k)]; ins (i+1) = u (i+1)
  have hn : ¬ s + 1 ≤ i + 1 := by omega
  rw [if_neg (by omega : ¬ i + 1 + k ≤ s), if_neg hn, ins_le u s x (by omega : i + 1 ≤ s)]
  have hfar : ins u s x (i + 1 + (k + 1)) = u (i + (k + 1)) := by
    rw [ins_gt u s x (by omega)]; congr 1; omega
  rw [hfar]
  have e3 : i + 1 + k = i + (k + 1) := by omega
  rw [e3]
  have hD2 : 0 < u (i + (k + 1)) - u (i + 1) := by
    have := hu (show i + 1 ≤ s by omega); have := hu (show s + 1 ≤ i + (k + 1) by omega); linarith
  field_simp
  ring

/-- The old span j that new span J lies in. -/
def oldSpan (s J : ℕ) : ℕ := if J ≤ s then J else J - 1

/-- **The basis identity of knot insertion**, on any nonempty new span. -/
theorem insert_basis (u : ℕ → ℝ) (hu : Monotone u) (s : ℕ) (x : ℝ) (h0 : u s ≤ x) (h1 : x < u (s + 1))
    (J : ℕ) (hJ : ins u s x J < ins u s x (J + 1)) :
    ∀ k i t, nb u (oldSpan s J) i k t = alpha u s x k i * nb (ins u s x) J i k t
      + (1 - alpha u s x k (i + 1)) * nb (ins u s x) J (i + 1) k t := by
  have hmono := ins_mono u s x hu h0 h1.le
  intro k
  induction k with
  | zero =>
    intro i t
    by_cases hJs : J ≤ s
    · have ho : oldSpan s J = J := by simp [oldSpan, hJs]
      rw [ho]
      simp only [nb, alpha, add_zero]
      by_cases hi : i = J
      · subst hi; simp [hJs]
      · by_cases hi2 : i + 1 = J
        · subst hi2; simp [hJs]
        · simp [hi, hi2]
    · have ho : oldSpan s J = J - 1 := by simp [oldSpan, hJs]
      rw [ho]
      simp only [nb, alpha, add_zero]
      have hJ1 : J - 1 ≠ J := by omega
      have hJ1' : J ≠ J - 1 := by omega
      have hJ2 : J - 1 + 1 = J := by omega
      have hJ5 : ¬ (J - 1 < s) := by omega
      have hJ6 : ¬ (J ≤ s + 1 - 1) := by omega
      by_cases hi : i = J - 1
      · subst hi; simp [hJ1, hJ2, hJ5, hJs]
      · by_cases hi2 : i = J
        · subst hi2; simp [hJ1, hJ1', hJs]
        · have : i + 1 ≠ J := by omega
          simp [hi, hi2, this]
  | succ k ih =>
    intro i t
    -- both sides through the recurrence, then the degree-k identity
    simp only [nb]
    rw [ih i t, ih (i + 1) t]
    set N0 := nb (ins u s x) J i k t
    set N1 := nb (ins u s x) J (i + 1) k t
    set N2 := nb (ins u s x) J (i + 1 + 1) k t
    -- the new functions vanish where their support has no length
    have z0 : ins u s x i = ins u s x (i + k + 1) → N0 = 0 := fun h =>
      nb_zero_degenerate _ hmono J hJ i k h t
    have z1 : ins u s x (i + 1) = ins u s x (i + 1 + k + 1) → N1 = 0 := fun h =>
      nb_zero_degenerate _ hmono J hJ (i + 1) k h t
    have hA0 : N0 = 0 ∨ omega u i (k + 1) t * alpha u s x k i
        = alpha u s x (k + 1) i * omega (ins u s x) i (k + 1) t := by
      by_cases h : ins u s x i = ins u s x (i + k + 1)
      · exact Or.inl (z0 h)
      · exact Or.inr (coefA u s x k i t h)
    have hA1 : N1 = 0 ∨ omega u (i + 1) (k + 1) t * alpha u s x k (i + 1)
        = alpha u s x (k + 1) (i + 1) * omega (ins u s x) (i + 1) (k + 1) t := by
      by_cases h : ins u s x (i + 1) = ins u s x (i + 1 + k + 1)
      · exact Or.inl (z1 h)
      · exact Or.inr (coefA u s x k (i + 1) t h)
    have hB0 := coefB u hu s x h0 h1 k i t
    have hB1 := coefB u hu s x h0 h1 k (i + 1) t
    set A0 := omega u i (k + 1) t * alpha u s x k i - alpha u s x (k + 1) i * omega (ins u s x) i (k + 1) t
    set A1 := omega u (i + 1) (k + 1) t * alpha u s x k (i + 1)
      - alpha u s x (k + 1) (i + 1) * omega (ins u s x) (i + 1) (k + 1) t
    rcases hA0 with h | h <;> rcases hA1 with h' | h'
    · linear_combination A0 * h + (-A1) * h' + N1 * (-hB0) + N2 * hB1
    · linear_combination A0 * h + N1 * (-hB0 - h') + N2 * hB1
    · linear_combination N0 * h + (-A1) * h' + N1 * (-hB0) + N2 * hB1
    · linear_combination N0 * h + N1 * (-hB0 - h') + N2 * hB1

/-- The poles after inserting x: `Q_i = α_i P_i + (1 - α_i) P_(i-1)` (`insert_knot`'s lerp). -/
noncomputable def insPoles (u : ℕ → ℝ) (s : ℕ) (x : ℝ) (p : ℕ) (P : ℕ → ℝ) (i : ℕ) : ℝ :=
  alpha u s x p i * P i + (1 - alpha u s x p i) * P (i - 1)

/-- **Knot insertion leaves the curve unchanged**: for x in a span `[u s, u (s+1))` of the curve
(p ≤ s < n), the n + 1 inserted poles on the inserted knots make, on every nonempty new span, the
polynomial of the old span it lies in. Each coordinate, and the homogeneous ones with the weight,
the same. -/
theorem insert_curve (u : ℕ → ℝ) (hu : Monotone u) (s : ℕ) (x : ℝ) (h0 : u s ≤ x) (h1 : x < u (s + 1))
    (p n : ℕ) (hps : p ≤ s) (hsn : s < n) (P : ℕ → ℝ) (J : ℕ) (hJ : ins u s x J < ins u s x (J + 1))
    (t : ℝ) :
    ∑ i ∈ range n, nb u (oldSpan s J) i p t * P i
      = ∑ i ∈ range (n + 1), nb (ins u s x) J i p t * insPoles u s x p P i := by
  simp only [insert_basis u hu s x h0 h1 J hJ p _ t, add_mul, sum_add_distrib, insPoles, mul_add]
  have a0 : alpha u s x p 0 = 1 := by simp [alpha, hps]
  have an : alpha u s x p n = 0 := by
    simp [alpha, show ¬ n + p ≤ s by omega, show s + 1 ≤ n by omega]
  -- the second old sum is the new sum's second part shifted by one
  have shift : ∑ i ∈ range n, (1 - alpha u s x p (i + 1)) * nb (ins u s x) J (i + 1) p t * P i
      = ∑ i ∈ range (n + 1), nb (ins u s x) J i p t * ((1 - alpha u s x p i) * P (i - 1)) := by
    rw [sum_range_succ' (fun i => nb (ins u s x) J i p t * ((1 - alpha u s x p i) * P (i - 1))), a0]
    simp only [sub_self, zero_mul, mul_zero, add_zero]
    refine sum_congr rfl fun i _ => ?_
    simp only [Nat.add_sub_cancel]; ring
  rw [shift, sum_range_succ (fun i => nb (ins u s x) J i p t * (alpha u s x p i * P i)), an]
  simp only [zero_mul, mul_zero, add_zero]
  congr 1
  refine sum_congr rfl fun i _ => by ring

/-! ## `insert_knot`'s poles are Boehm's

The kernel's `insert_knot` (kernel/forge-geom/src/spline.rs, proven of the translation in
verify/aeneas/forge-geom `insert_knot_spec`) keeps poles up to `s - p`, then lerps
`P_(i-1) + (P_i - P_(i-1)) a_i` for `i` up to `s - m`, with `a_i = (x - u_i) / (u_(i+p) - u_i)`
(0 if that denominator is not positive), and then shifts the rest by one. Here `m` is x's
multiplicity, so `u_i = x` for `s - m < i ≤ s`. In exact arithmetic that is `insPoles` at every
position (`kernelPoles_eq`), so `insert_curve` holds of it. In `from_nurbs`, `m ≤ p`: a knot is
inserted only while its multiplicity is below `p + 1`.
-/

/-- `insert_knot`'s poles, as a function of the position. -/
noncomputable def kernelPoles (u : ℕ → ℝ) (s : ℕ) (x : ℝ) (p m : ℕ) (P : ℕ → ℝ) (i : ℕ) : ℝ :=
  if i ≤ s - p then P i
  else if i ≤ s - m then
    P (i - 1) + (P i - P (i - 1)) *
      (if 0 < u (i + p) - u i then (x - u i) / (u (i + p) - u i) else 0)
  else P (i - 1)

theorem kernelPoles_eq (u : ℕ → ℝ) (hu : Monotone u) (s : ℕ) (x : ℝ) (h0 : u s ≤ x)
    (h1 : x < u (s + 1)) (p m : ℕ) (hps : p ≤ s) (hmp : m ≤ p)
    (hmult : ∀ i, s - m < i → i ≤ s → u i = x) (P : ℕ → ℝ) (i : ℕ) :
    kernelPoles u s x p m P i = insPoles u s x p P i := by
  unfold kernelPoles insPoles alpha
  by_cases ha : i ≤ s - p
  · simp only [ha, if_true, show i + p ≤ s by omega]; ring
  · simp only [ha, if_false, show ¬ i + p ≤ s by omega]
    by_cases hb : i ≤ s - m
    · simp only [hb, if_true, show ¬ s + 1 ≤ i by omega]
      -- the span [u_i, u_(i+p)] holds [u_s, u_(s+1)), so it is not empty
      have hlt : u i < u (i + p) := by
        have := hu (show i ≤ s by omega)
        have := hu (show s + 1 ≤ i + p by omega)
        linarith
      simp only [sub_pos.2 hlt, if_true, show ¬ (s + 1 ≤ i) by omega, ite_false]; ring
    · simp only [hb, if_false]
      by_cases hc : s + 1 ≤ i
      · simp only [hc, if_true]; ring
      · simp only [hc, if_false]
        rw [hmult i (by omega) (by omega)]
        simp

end SplineSpec
