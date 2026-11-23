import ForgeExact.Fillet.PlanePlane

/-!
# Solids of revolution: the closing is the meridian's

A solid of revolution about the z axis is `Rev T = {p : T (ρ p) p.z}`, with `ρ p = √(x² + y²)` and
`T` a set of the half-plane `ρ ≥ 0`. The ball closing commutes with the rotations, and here that is
made exact without them:

* `free_rev`: a ball misses `Rev T` iff its meridian disc misses `T` in the half-plane
  (`Free2`: every point of `T` with `u ≥ 0` is at least `r` from `(ρ c, c.z)`);
* `closing_rev`: `x` is in the closing of `Rev T` iff `(ρ x, x.z)` is in the half-plane closing of
  `T` (`closing2`: centres with `ρ ≥ 0` only).

Square roots are needed to name `ρ`, so these theorems hold in any ordered field with square roots
of its non-negative elements (`Sqrt`: ℝ, the real algebraic numbers, any real closed field).
-/

namespace ForgeExact.Fillet

open Lean.Grind

set_option linter.unusedSectionVars false

/-- Square roots of the non-negative elements. -/
class Sqrt (K : Type) [Field K] [LE K] [LT K] where
  sqrt : K → K
  nonneg : ∀ a, 0 ≤ sqrt a
  sq : ∀ a, 0 ≤ a → sqrt a * sqrt a = a

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] [Sqrt K]

open Sqrt

/-- A non-negative root is the root. -/
theorem sqrt_eq {a s : K} (hs : 0 ≤ s) (h : s * s = a) : sqrt a = s := by
  have ha : 0 ≤ a := by have := sq_nonneg' s; grind
  have e := Sqrt.sq a ha
  have n := Sqrt.nonneg (K := K) a
  rcases le_or_lt' (sqrt a) s with h1 | h1
  · rcases le_or_lt' s (sqrt a) with h2 | h2
    · grind
    · have := mul_self_lt_mul_self' n h2; grind
  · have := mul_self_lt_mul_self' hs h1; grind

/-- The distance from the axis. -/
def rho (p : V K) : K := sqrt (p.x * p.x + p.y * p.y)

theorem rho_nonneg (p : V K) : 0 ≤ rho p := Sqrt.nonneg _

theorem rho_sq (p : V K) : rho p * rho p = p.x * p.x + p.y * p.y := by
  have := sq_nonneg' p.x; have := sq_nonneg' p.y
  exact Sqrt.sq _ (by grind)

/-- The solid of revolution of a meridian set. -/
def Rev (T : K → K → Prop) : Pred (V K) := fun p => T (rho p) p.z

/-- The meridian disc misses `T`. -/
def Free2 (T : K → K → Prop) (r a b : K) : Prop :=
  ∀ u z, 0 ≤ u → T u z → r * r ≤ (u - a) * (u - a) + (z - b) * (z - b)

/-- The half-plane closing. -/
def closing2 (T : K → K → Prop) (r a b : K) : Prop :=
  ∀ a' b', 0 ≤ a' → Free2 T r a' b' → r * r ≤ (a - a') * (a - a') + (b - b') * (b - b')

/-- The plane distance from `p` to `c` is at least the difference of their distances from the
axis (Cauchy–Schwarz, squared: `p.x c.x + p.y c.y ≤ ρ p ρ c`). -/
theorem plane_d2_ge (p c : V K) :
    (rho p - rho c) * (rho p - rho c)
      ≤ (p.x - c.x) * (p.x - c.x) + (p.y - c.y) * (p.y - c.y) := by
  have hp := rho_sq p; have hc := rho_sq c
  have np := rho_nonneg p; have nc := rho_nonneg c
  -- (p·c)² ≤ ρp² ρc², and ρp ρc ≥ 0, so p·c ≤ ρp ρc
  have lag : (p.x * c.x + p.y * c.y) * (p.x * c.x + p.y * c.y) + (p.x * c.y - p.y * c.x) *
      (p.x * c.y - p.y * c.x) = (rho p * rho p) * (rho c * rho c) := by rw [hp, hc]; grind
  have := sq_nonneg' (p.x * c.y - p.y * c.x)
  have m := mul_nonneg' np nc
  have dot : p.x * c.x + p.y * c.y ≤ rho p * rho c := by
    rcases le_or_lt' (p.x * c.x + p.y * c.y) (rho p * rho c) with h | h
    · exact h
    · have := mul_self_lt_mul_self' m h; grind
  grind

/-- The point at distance `u` from the axis, height `z`, on the half-plane through `c`'s azimuth
(the x axis's when `c` is on the axis). -/
noncomputable def toward (c : V K) (u z : K) : V K :=
  open Classical in if rho c = 0 then ⟨u, 0, z⟩ else ⟨u * c.x / rho c, u * c.y / rho c, z⟩

theorem rho_toward (c : V K) {u : K} (hu : 0 ≤ u) (z : K) : rho (toward c u z) = u := by
  unfold toward
  split
  · exact sqrt_eq hu (by simp only []; grind)
  · rename_i h
    apply sqrt_eq hu
    have := rho_sq c
    simp only []
    have e : u * c.x / rho c * (u * c.x / rho c) + u * c.y / rho c * (u * c.y / rho c)
        = u * u * ((c.x * c.x + c.y * c.y) / (rho c * rho c)) := by grind
    rw [e, ← this]; grind

theorem toward_z (c : V K) (u z : K) : (toward c u z).z = z := by
  unfold toward; split <;> rfl

/-- On the half-plane through `c`, plane distance to `c` is the difference of axis distances. -/
theorem d2_toward (c : V K) {u : K} (_hu : 0 ≤ u) (z : K) :
    V.d2 (toward c u z) c = (u - rho c) * (u - rho c) + (z - c.z) * (z - c.z) := by
  have hc := rho_sq c
  have nc := rho_nonneg c
  unfold toward
  split
  · rename_i h
    have : c.x * c.x + c.y * c.y = 0 := by rw [← hc, h]; grind
    have := sq_nonneg' c.x; have := sq_nonneg' c.y
    have hx : c.x * c.x = 0 := by grind
    have hy : c.y * c.y = 0 := by grind
    simp only [V.d2_def]; rw [h]; grind
  · rename_i h
    simp only [V.d2_def]
    have e : (u * c.x / rho c - c.x) * (u * c.x / rho c - c.x)
        + (u * c.y / rho c - c.y) * (u * c.y / rho c - c.y)
        = (u - rho c) * (u - rho c) * ((c.x * c.x + c.y * c.y) / (rho c * rho c)) := by grind
    rw [← hc] at e
    have : rho c * rho c / (rho c * rho c) = 1 := by
      have : rho c * rho c ≠ 0 := by
        intro h0; have := mul_self_lt_mul_self' (le_refl' 0) (show 0 < rho c by grind); grind
      grind
    grind
where le_refl' (a : K) : a ≤ a := by grind

theorem d2_split (p c : V K) :
    V.d2 p c = ((p.x - c.x) * (p.x - c.x) + (p.y - c.y) * (p.y - c.y)) + (p.z - c.z) * (p.z - c.z) := by
  simp only [V.d2_def]

/-- **A ball misses a solid of revolution iff its meridian disc misses the meridian set.** -/
theorem free_rev (T : K → K → Prop) (r : K) (c : V K) :
    Free (Rev T) r c ↔ Free2 T r (rho c) c.z := by
  constructor
  · intro h u z hu hT
    have := h (toward c u z) (by simp only [Rev]; rw [rho_toward c hu, toward_z]; exact hT)
    rwa [d2_toward c hu] at this
  · intro h y hy
    have := h (rho y) y.z (rho_nonneg y) hy
    have := plane_d2_ge y c
    rw [d2_split]; grind

/-- **The closing of a solid of revolution is the revolved half-plane closing of its meridian.** -/
theorem closing_rev (T : K → K → Prop) (r : K) (x : V K) :
    closing (Rev T) r x ↔ closing2 T r (rho x) x.z := by
  constructor
  · intro h a' b' ha hf
    let c := toward x a' b'
    have hc : Free (Rev T) r c := by
      rw [free_rev]; rw [rho_toward x ha, toward_z]; exact hf
    have := h c hc
    rw [PlanePair.d2_comm, d2_toward x ha] at this
    grind
  · intro h c hc
    rw [free_rev] at hc
    have := h (rho c) c.z (rho_nonneg c) hc
    have := plane_d2_ge x c
    rw [d2_split]; grind

end ForgeExact.Fillet
