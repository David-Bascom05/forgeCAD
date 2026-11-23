/-
Turning (kernel/forge-turn): why a clearance proven in the (ρ, z) half-plane bounds the 3D one.

A part of revolution about z is P = {(x, y, z) : (ρ(x, y), z) ∈ H}, ρ(x, y) = √(x² + y²), H its
half-profile. forge-turn's checker works in the plane:

* `pi_lipschitz`: π(x, y, z) = (ρ(x, y), z) is 1-Lipschitz, so a body whose π-image keeps `d`
  from H keeps `d` from P (`turn_clear3`).
* `smear_le`: a body point at depth |y| ≤ t below the centre height, at x ≥ xm ≥ 0 in its
  section, has ρ − x ≤ √(xm² + t²) − xm: the checker's smear e (check.rs, insert.rs).
* `fold_clear`: the fold s ↦ (|s.1|, s.2) (π's image of a section point on either side of the
  axis): a point clear of H and of H's mirror is clear of H after folding — the checks are made
  against Hf = H ∪ mirror(H) (check.rs `Obstacle::of`).
* `sigma_mem`: σ(x, y) = sign(x)·ρ(x, y) moves a section point outward from the axis by at most
  the smear, on its own side: σ(B) ⊆ (Z₊ ∪ Z₋) ⊕ B_r.
* `turn_clear`: the assembly, by `clear_of_frontier`: a connected swept set Z (convex) whose
  frontier pieces of Hf all keep `c > 0` from Z, with one point of Z outside Hf, keeps `c` from
  all of Hf; its r-neighbourhood keeps `c − r` (`turn_clear_ball`).

Hypotheses this file does not discharge (research/cam/turning.md lists them):
* `ZonoBoundary`: the boundary of K ⊕ [0, g₁] ⊕ [0, g₂] (K a convex polygon) lies in the
  segments check.rs enumerates (edges of K at the four corners of the parallelogram, vertices
  swept along each generator from the other's ends). Stated, not proven here.
* The lower bounds of slab::dist on those segments: proven (Dist2.lean, DistArc.lean, Trim.lean);
  the separating-axis bound forge-turn adds is `sep_axis` below.
* H.arc: a control's arc between ends at unequal radii stays between the radii.
-/
import ForgeCam.SignedDist

open Metric Set Real

set_option linter.unusedSectionVars false

namespace ForgeCam.Turn

/-- The radius of a point about the z axis. -/
noncomputable def rho (x y : ℝ) : ℝ := √(x ^ 2 + y ^ 2)

/-- The squared distance between two points of the half-plane picture, and of space. -/
def d2 (a b : ℝ × ℝ) : ℝ := (a.1 - b.1) ^ 2 + (a.2 - b.2) ^ 2

def d3 (a b : ℝ × ℝ × ℝ) : ℝ := (a.1 - b.1) ^ 2 + (a.2.1 - b.2.1) ^ 2 + (a.2.2 - b.2.2) ^ 2

/-- π, the map to the half-profile plane. -/
noncomputable def pi3 (p : ℝ × ℝ × ℝ) : ℝ × ℝ := (rho p.1 p.2.1, p.2.2)

/-- |ρ(u) − ρ(v)| ≤ |u − v|, squared. -/
theorem rho_sub_sq_le (x1 y1 x2 y2 : ℝ) : (rho x1 y1 - rho x2 y2) ^ 2 ≤ (x1 - x2) ^ 2 + (y1 - y2) ^ 2 := by
  unfold rho
  have hA : 0 ≤ x1 ^ 2 + y1 ^ 2 := by positivity
  have hB : 0 ≤ x2 ^ 2 + y2 ^ 2 := by positivity
  have sA := sq_sqrt hA
  have sB := sq_sqrt hB
  have hAB : √(x1 ^ 2 + y1 ^ 2) * √(x2 ^ 2 + y2 ^ 2) = √((x1 ^ 2 + y1 ^ 2) * (x2 ^ 2 + y2 ^ 2)) :=
    (sqrt_mul hA _).symm
  -- Cauchy–Schwarz: x1 x2 + y1 y2 ≤ √A √B
  have cs : x1 * x2 + y1 * y2 ≤ √(x1 ^ 2 + y1 ^ 2) * √(x2 ^ 2 + y2 ^ 2) := by
    rw [hAB]
    apply le_sqrt_of_sq_le
    nlinarith [sq_nonneg (x1 * y2 - x2 * y1)]
  nlinarith [sA, sB, cs]

/-- π is 1-Lipschitz (squared distances). -/
theorem pi_lipschitz (p q : ℝ × ℝ × ℝ) : d2 (pi3 p) (pi3 q) ≤ d3 p q := by
  unfold d2 d3 pi3
  simp only
  have := rho_sub_sq_le p.1 p.2.1 q.1 q.2.1
  linarith

/-- G1 for turning: a body whose π-image keeps (squared) distance `d` from every point of H keeps
it from every point of the part of revolution π⁻¹(H). -/
theorem turn_clear3 {B : Set (ℝ × ℝ × ℝ)} {H : Set (ℝ × ℝ)} {d : ℝ}
    (h : ∀ b ∈ B, ∀ q ∈ H, d ≤ d2 (pi3 b) q) :
    ∀ b ∈ B, ∀ p : ℝ × ℝ × ℝ, pi3 p ∈ H → d ≤ d3 b p := fun b hb p hp =>
  le_trans (h b hb _ hp) (pi_lipschitz b p)

/-- √(x² + t²) − x falls as x ≥ 0 grows. -/
theorem smear_anti {t x x' : ℝ} (hx : 0 ≤ x) (hxx : x ≤ x') :
    √(x' ^ 2 + t ^ 2) - x' ≤ √(x ^ 2 + t ^ 2) - x := by
  -- √(x² + t²) − x = t² / (√(x² + t²) + x) when the denominator is positive; argue directly
  have h1 : 0 ≤ x ^ 2 + t ^ 2 := by positivity
  have h2 : 0 ≤ x' ^ 2 + t ^ 2 := by positivity
  set a := √(x ^ 2 + t ^ 2) with ha
  set b := √(x' ^ 2 + t ^ 2) with hb
  have sa : a ^ 2 = x ^ 2 + t ^ 2 := sq_sqrt h1
  have sb : b ^ 2 = x' ^ 2 + t ^ 2 := sq_sqrt h2
  have a0 : 0 ≤ a := sqrt_nonneg _
  have b0 : 0 ≤ b := sqrt_nonneg _
  have ax : x ≤ a := by rw [ha]; exact le_sqrt_of_sq_le (by nlinarith [sq_nonneg t])
  have bx : x' ≤ b := by rw [hb]; exact le_sqrt_of_sq_le (by nlinarith [sq_nonneg t])
  have ab : a ≤ b := by rw [ha, hb]; exact sqrt_le_sqrt (by nlinarith)
  -- (b − x')(b + x') = t² = (a − x)(a + x), and b + x' ≥ a + x ≥ 0
  nlinarith [mul_nonneg (sub_nonneg.mpr ab) (sub_nonneg.mpr hxx), sq_nonneg (b - x' - (a - x)),
    mul_nonneg (sub_nonneg.mpr bx) (sub_nonneg.mpr ax)]

/-- The smear bound: at x ≥ xm ≥ 0 and depth |y| ≤ t, ρ(x, y) − x ≤ √(xm² + t²) − xm. -/
theorem smear_le {x y xm t : ℝ} (hxm : 0 ≤ xm) (hx : xm ≤ x) (hy : y ^ 2 ≤ t ^ 2) :
    rho x y - x ≤ √(xm ^ 2 + t ^ 2) - xm := by
  unfold rho
  have h1 : √(x ^ 2 + y ^ 2) ≤ √(x ^ 2 + t ^ 2) := sqrt_le_sqrt (by linarith)
  have h2 := smear_anti (t := t) hxm hx
  linarith

/-- The smear is never negative: ρ ≥ |x|. -/
theorem abs_le_rho (x y : ℝ) : |x| ≤ rho x y := by
  unfold rho
  rw [← sqrt_sq_eq_abs]
  exact sqrt_le_sqrt (by nlinarith [sq_nonneg y])

/-- The mirror in the axis, and the fold. -/
def mirror (s : ℝ × ℝ) : ℝ × ℝ := (-s.1, s.2)

def fold (s : ℝ × ℝ) : ℝ × ℝ := (|s.1|, s.2)

theorem d2_mirror (s q : ℝ × ℝ) : d2 (mirror s) q = d2 s (mirror q) := by
  unfold d2 mirror; simp only; ring

/-- A point clear (by `c`, squared) of H and of mirror(H) is clear of H after folding. -/
theorem fold_clear {H : Set (ℝ × ℝ)} {c : ℝ} {s : ℝ × ℝ}
    (h : ∀ q, q ∈ H ∨ mirror q ∈ H → c ≤ d2 s q) : ∀ q ∈ H, c ≤ d2 (fold s) q := by
  intro q hq
  rcases le_or_gt 0 s.1 with hs | hs
  · have : fold s = s := by unfold fold; rw [abs_of_nonneg hs]
    rw [this]; exact h q (Or.inl hq)
  · have : fold s = mirror s := by unfold fold mirror; rw [abs_of_neg hs]
    rw [this, d2_mirror]
    apply h
    right
    unfold mirror; simpa using hq

/-- σ, the signed radius: a section point (x, z) at depth y goes to (sign(x)·ρ, z); π = fold ∘ σ.
It moves the point outward from the axis, on its own side, by ρ − |x| ∈ [0, ρ − |x|]. -/
noncomputable def sigma (x y : ℝ) : ℝ := if 0 ≤ x then rho x y else -rho x y

theorem fold_sigma (x y z : ℝ) : fold (sigma x y, z) = (rho x y, z) := by
  unfold fold sigma
  split_ifs with h
  · simp [abs_of_nonneg (sqrt_nonneg _), rho]
  · simp [abs_neg, abs_of_nonneg (sqrt_nonneg _), rho]

/-- σ(x, y) = x + δ (x ≥ 0) or x − δ (x < 0), with 0 ≤ δ = ρ − |x|: the outward smear. -/
theorem sigma_mem (x y : ℝ) :
    ∃ δ, 0 ≤ δ ∧ δ = rho x y - |x| ∧ sigma x y = if 0 ≤ x then x + δ else x - δ := by
  refine ⟨rho x y - |x|, by linarith [abs_le_rho x y], rfl, ?_⟩
  unfold sigma
  split_ifs with h
  · rw [abs_of_nonneg h]; ring
  · rw [abs_of_neg (lt_of_not_ge h)]; ring

/-- The separating-axis bound forge-turn adds to slab::dist (check.rs `seg_dist`): for a unit
vector w, the gap between two sets' projections onto w bounds their distance. -/
theorem sep_axis (w : ℝ × ℝ) (hw : w.1 ^ 2 + w.2 ^ 2 = 1) (p q : ℝ × ℝ) :
    ((p.1 - q.1) * w.1 + (p.2 - q.2) * w.2) ^ 2 ≤ d2 p q := by
  unfold d2
  nlinarith [sq_nonneg ((p.1 - q.1) * w.2 - (p.2 - q.2) * w.1)]

section assembly

variable {V : Type*} [NormedAddCommGroup V] [NormedSpace ℝ V]

/-- The assembly (check.rs `body_line`): Z convex (the swept core), F ⊇ the frontier of the
closed obstacle Hf (its pieces), every piece at least c > 0 from Z, one point of Z outside Hf:
then all of Z is at least c from all of Hf. -/
theorem turn_clear {Z Hf F : Set V} {c : ℝ} (hZ : Convex ℝ Z) (hH : IsClosed Hf)
    (hF : frontier Hf ⊆ F) (hc : 0 < c) (hcl : ∀ f ∈ F, c ≤ sd Z f) (hout : ∃ z ∈ Z, z ∉ Hf) :
    ∀ z ∈ Z, ∀ h ∈ Hf, c ≤ dist z h :=
  clear_of_frontier hZ.isPreconnected hH hF hc hcl hout

/-- With the body's radius: the r-neighbourhood of such a Z keeps c − r. -/
theorem turn_clear_ball {Z Hf F : Set V} {c r : ℝ} (hZ : Convex ℝ Z) (hH : IsClosed Hf)
    (hF : frontier Hf ⊆ F) (hc : 0 < c) (hcl : ∀ f ∈ F, c ≤ sd Z f) (hout : ∃ z ∈ Z, z ∉ Hf) :
    ∀ z ∈ Z, ∀ u, dist u z ≤ r → ∀ h ∈ Hf, c - r ≤ dist u h := by
  intro z hz u hu h hh
  have := turn_clear hZ hH hF hc hcl hout z hz h hh
  have := dist_triangle z u h
  rw [dist_comm] at hu
  linarith

end assembly

end ForgeCam.Turn
