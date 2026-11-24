import Mathlib.Analysis.SpecialFunctions.Trigonometric.Bounds
import Mathlib.Tactic

/-!
# Composing forward errors (the model of the zone fold)

Each float operation's result `r'` is within `u |r| + η` of its exact result `r` (the bridge's
`F64.Ops.*_err`). When its operands are themselves approximations `x ≈ X` (within `ex`) and
`y ≈ Y` (within `ey`), the result is within

* `mulE X ex Y ey = u (|X| + ex)(|Y| + ey) + η + ex |Y| + |X| ey + ex ey` of `X Y` (`mul_approx`),
* `addE X ex Y ey = u (|X| + |Y| + ex + ey) + η + ex + ey` of `X + Y` (`add_approx`).

A computation's forward error is then the composition of these along its operations, which is
how RefineZone.lean states the zone fold's (build.rs `Geo::Round`): `phi`, dmath's sine and
cosine, `round_local`, and the axis frame's `point_to_world`, each operand's error carried into
the next.
-/

namespace ForgeSheetCore.Zone

variable (u η : ℝ)

def mulE (X ex Y ey : ℝ) : ℝ := u * (|X| + ex) * (|Y| + ey) + η + ex * |Y| + |X| * ey + ex * ey

def addE (X ex Y ey : ℝ) : ℝ := u * (|X| + |Y| + ex + ey) + η + ex + ey

variable {u η}

theorem mul_approx (hu : 0 ≤ u) {X Y x y p ex ey : ℝ} (hx : |x - X| ≤ ex) (hy : |y - Y| ≤ ey)
    (hp : |p - x * y| ≤ u * |x * y| + η) : |p - X * Y| ≤ mulE u η X ex Y ey := by
  have hex : 0 ≤ ex := le_trans (abs_nonneg _) hx
  have hey : 0 ≤ ey := le_trans (abs_nonneg _) hy
  have ax : |x| ≤ |X| + ex := by have := abs_sub_abs_le_abs_sub x X; linarith
  have ay : |y| ≤ |Y| + ey := by have := abs_sub_abs_le_abs_sub y Y; linarith
  have hxy : |x * y - X * Y| ≤ ex * |Y| + |X| * ey + ex * ey := by
    rw [show x * y - X * Y = (x - X) * Y + X * (y - Y) + (x - X) * (y - Y) by ring]
    calc _ ≤ |(x - X) * Y + X * (y - Y)| + |(x - X) * (y - Y)| := abs_add_le _ _
      _ ≤ |(x - X) * Y| + |X * (y - Y)| + |(x - X) * (y - Y)| := by gcongr; exact abs_add_le _ _
      _ = |x - X| * |Y| + |X| * |y - Y| + |x - X| * |y - Y| := by rw [abs_mul, abs_mul, abs_mul]
      _ ≤ _ := by gcongr
  have hxy' : u * |x * y| ≤ u * ((|X| + ex) * (|Y| + ey)) := by
    rw [abs_mul]; exact mul_le_mul_of_nonneg_left (mul_le_mul ax ay (abs_nonneg _) (by positivity)) hu
  unfold mulE
  calc |p - X * Y| ≤ |p - x * y| + |x * y - X * Y| := abs_sub_le _ _ _
    _ ≤ _ := by nlinarith

theorem add_approx (hu : 0 ≤ u) {X Y x y s ex ey : ℝ} (hx : |x - X| ≤ ex) (hy : |y - Y| ≤ ey)
    (hs : |s - (x + y)| ≤ u * |x + y| + η) : |s - (X + Y)| ≤ addE u η X ex Y ey := by
  have hxy : |(x + y) - (X + Y)| ≤ ex + ey := by
    rw [show (x + y) - (X + Y) = (x - X) + (y - Y) by ring]
    exact le_trans (abs_add_le _ _) (add_le_add hx hy)
  have : |x + y| ≤ |X| + |Y| + ex + ey := by
    have := abs_sub_abs_le_abs_sub (x + y) (X + Y)
    have := abs_add_le X Y
    linarith
  have := mul_le_mul_of_nonneg_left this hu
  unfold addE
  calc |s - (X + Y)| ≤ |s - (x + y)| + |(x + y) - (X + Y)| := abs_sub_le _ _ _
    _ ≤ _ := by linarith

theorem mulE_nonneg (hu : 0 ≤ u) (hη : 0 ≤ η) {X ex Y ey : ℝ} (hex : 0 ≤ ex) (hey : 0 ≤ ey) :
    0 ≤ mulE u η X ex Y ey := by unfold mulE; positivity

theorem addE_nonneg (hu : 0 ≤ u) (hη : 0 ≤ η) {X ex Y ey : ℝ} (hex : 0 ≤ ex) (hey : 0 ≤ ey) :
    0 ≤ addE u η X ex Y ey := by unfold addE; positivity

/-- The sine and cosine at a computed angle `a'` within `ea` of `a`, each within `ε` of the true
ones at `a'`: within `ε + ea` of those at `a` (both are 1-Lipschitz). -/
theorem trig_approx {a a' ea ε s c : ℝ} (ha : |a' - a| ≤ ea) (hs : |s - Real.sin a'| ≤ ε)
    (hc : |c - Real.cos a'| ≤ ε) :
    |s - Real.sin a| ≤ ε + ea ∧ |c - Real.cos a| ≤ ε + ea := by
  have h1 := Real.abs_sin_sub_sin_le a' a
  have h2 := Real.abs_cos_sub_cos_le a' a
  constructor
  · calc _ ≤ |s - Real.sin a'| + |Real.sin a' - Real.sin a| := abs_sub_le _ _ _
      _ ≤ _ := by linarith
  · calc _ ≤ |c - Real.cos a'| + |Real.cos a' - Real.cos a| := abs_sub_le _ _ _
      _ ≤ _ := by linarith

end ForgeSheetCore.Zone
