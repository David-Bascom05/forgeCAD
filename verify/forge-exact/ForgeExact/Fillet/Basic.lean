import ForgeExact.Topology

/-!
# Fillets, the spec: points and the ordered-field facts it rests on

The fillet spec (research/fillets/lean-spec-design.md) is stated over any ordered field `K`, with
the same class context as `ForgeExact.Alg.Model` (a `Lean.Grind.Field` with a linear order and
`OrderedRing`). ℚ, the real algebraic numbers and ℝ are all such fields, so every theorem holds in
each. Distances are written squared, so no square root appears in any statement: a ball is
`d2 x c < r * r`.

This file holds the vectors of `K³` and the few nonlinear sign facts `grind` (linear arithmetic over
ordered fields, ring normalisation) does not find by itself.
-/

namespace ForgeExact.Fillet

open Lean.Grind

set_option linter.unusedSectionVars false

/-- A point, or a vector, of `K³`. -/
structure V (K : Type) where
  x : K
  y : K
  z : K

section
variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

namespace V

def add (p q : V K) : V K := ⟨p.x + q.x, p.y + q.y, p.z + q.z⟩
def sub (p q : V K) : V K := ⟨p.x - q.x, p.y - q.y, p.z - q.z⟩
def smul (a : K) (p : V K) : V K := ⟨a * p.x, a * p.y, a * p.z⟩
def dot (p q : V K) : K := p.x * q.x + p.y * q.y + p.z * q.z
def cross (p q : V K) : V K := ⟨p.y * q.z - p.z * q.y, p.z * q.x - p.x * q.z, p.x * q.y - p.y * q.x⟩
/-- The squared distance. -/
def d2 (p q : V K) : K := dot (sub p q) (sub p q)

@[simp] theorem add_x (p q : V K) : (add p q).x = p.x + q.x := rfl
@[simp] theorem add_y (p q : V K) : (add p q).y = p.y + q.y := rfl
@[simp] theorem add_z (p q : V K) : (add p q).z = p.z + q.z := rfl
@[simp] theorem sub_x (p q : V K) : (sub p q).x = p.x - q.x := rfl
@[simp] theorem sub_y (p q : V K) : (sub p q).y = p.y - q.y := rfl
@[simp] theorem sub_z (p q : V K) : (sub p q).z = p.z - q.z := rfl
@[simp] theorem smul_x (a : K) (p : V K) : (smul a p).x = a * p.x := rfl
@[simp] theorem smul_y (a : K) (p : V K) : (smul a p).y = a * p.y := rfl
@[simp] theorem smul_z (a : K) (p : V K) : (smul a p).z = a * p.z := rfl
@[simp] theorem cross_x (p q : V K) : (cross p q).x = p.y * q.z - p.z * q.y := rfl
@[simp] theorem cross_y (p q : V K) : (cross p q).y = p.z * q.x - p.x * q.z := rfl
@[simp] theorem cross_z (p q : V K) : (cross p q).z = p.x * q.y - p.y * q.x := rfl

theorem dot_def (p q : V K) : dot p q = p.x * q.x + p.y * q.y + p.z * q.z := rfl
theorem d2_def (p q : V K) :
    d2 p q = (p.x - q.x) * (p.x - q.x) + (p.y - q.y) * (p.y - q.y) + (p.z - q.z) * (p.z - q.z) := rfl

theorem ext' {p q : V K} (hx : p.x = q.x) (hy : p.y = q.y) (hz : p.z = q.z) : p = q := by
  cases p; cases q; simp_all

end V

/-! ## Signs of products -/

theorem le_or_lt' (a b : K) : a ≤ b ∨ b < a := by grind

theorem sq_nonneg' (a : K) : 0 ≤ a * a := by
  rcases le_or_lt' 0 a with h | h
  · exact OrderedRing.mul_nonneg h h
  · have := OrderedRing.mul_nonneg (a := -a) (b := -a) (by grind) (by grind)
    grind

theorem mul_pos' {a b : K} (ha : 0 < a) (hb : 0 < b) : 0 < a * b := OrderedRing.mul_pos ha hb

theorem mul_nonneg' {a b : K} (ha : 0 ≤ a) (hb : 0 ≤ b) : 0 ≤ a * b := OrderedRing.mul_nonneg ha hb

theorem mul_le_mul_of_nonneg_left' {a b c : K} (h : a ≤ b) (hc : 0 ≤ c) : c * a ≤ c * b := by
  have := mul_nonneg' (a := c) (b := b - a) hc (by grind)
  grind

theorem mul_lt_mul_of_pos_left' {a b c : K} (h : a < b) (hc : 0 < c) : c * a < c * b :=
  OrderedRing.mul_lt_mul_of_pos_left h hc

/-- For `0 ≤ a`, `a < b` gives `a * a < b * b`. -/
theorem mul_self_lt_mul_self' {a b : K} (ha : 0 ≤ a) (h : a < b) : a * a < b * b := by
  have h1 := mul_le_mul_of_nonneg_left' (a := a) (b := b) (c := a) (by grind) ha
  have h2 := mul_lt_mul_of_pos_left' (a := a) (b := b) (c := b) h (show 0 < b by grind)
  grind

theorem mul_self_le_mul_self' {a b : K} (ha : 0 ≤ a) (h : a ≤ b) : a * a ≤ b * b := by
  have h1 := mul_le_mul_of_nonneg_left' (a := a) (b := b) (c := a) h ha
  have h2 := mul_le_mul_of_nonneg_left' (a := a) (b := b) (c := b) h (show 0 ≤ b by grind)
  grind

/-- Squares: `-r < a < r` with `0 < r` gives `a * a < r * r`. -/
theorem sq_lt_of_abs_lt {a r : K} (h1 : -r < a) (h2 : a < r) : a * a < r * r := by
  rcases le_or_lt' 0 a with h | h
  · exact mul_self_lt_mul_self' h h2
  · have := mul_self_lt_mul_self' (a := -a) (b := r) (by grind) (by grind)
    grind

/-- `a * a < r * r` with `0 < r` gives `a < r`. -/
theorem lt_of_sq_lt {a r : K} (hr : 0 < r) (h : a * a < r * r) : a < r := by
  rcases le_or_lt' r a with h' | h'
  · have := mul_self_le_mul_self' (by grind) h'
    grind
  · exact h'

/-- `c * c < 1` puts `c` strictly between -1 and 1. -/
theorem abs_lt_one_of_sq {c : K} (h : c * c < 1) : -1 < c ∧ c < 1 := by
  have one : (0 : K) < 1 := OrderedRing.zero_lt_one
  refine ⟨?_, lt_of_sq_lt one (by grind)⟩
  have := lt_of_sq_lt (a := -c) one (by grind)
  grind

/-- `a * b < 0` with `0 < b` gives `a < 0`. -/
theorem neg_of_mul_neg_right {a b : K} (hb : 0 < b) (h : a * b < 0) : a < 0 := by
  rcases le_or_lt' 0 a with h' | h'
  · have := mul_nonneg' h' (show 0 ≤ b by grind)
    grind
  · exact h'

/-- `c * a ≤ c * b` with `0 < c` gives `a ≤ b`. -/
theorem le_of_mul_le_mul_left' {a b c : K} (h : c * a ≤ c * b) (hc : 0 < c) : a ≤ b := by
  rcases le_or_lt' a b with h' | h'
  · exact h'
  · have := mul_lt_mul_of_pos_left' h' hc
    grind

end

end ForgeExact.Fillet
