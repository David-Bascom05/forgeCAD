/-
H.pathends, the metric core (kernel/forge-cam-geom/src/check.rs, `Move::path_gap`).

A planar move's arc is carried with its ends as f64 points a, b near the true ends A₀, A₁ (each
within ε: the enclosures' half-widths). The candidates bound distances to the model arc M of
Trim.lean: the circle's arc between the rays through a and b, plus the radial pieces from the
circle to a and b. `path_gap` lowers by 2g + 2ε. Its argument has two metric steps, proven here:

- `proj_near`: the point a′ where a's ray meets the circle (a point of M) is within 2ε of A₀;
- `chord_mono`: a circle point x in the cone spanned by A₀ − c and a′ − c (the true arc's
  sliver beyond a's ray) is no farther from a′ than A₀ is (with ⟨A₀ − c, a′ − c⟩ ≥ 0: they are
  within a quarter turn, as 2ε ≪ r gives).

So every point of the true arc beyond a's ray is within 2ε of a′ ∈ M. The step from these to the
whole arc (a point of the true arc is in M's sector, or in one of the two slivers) is the cone
decomposition of the plane, argued in the code and not yet here: H.pathends stays a hypothesis
for that step only.
-/
import ForgeCam.Trim

namespace ForgeCam.PathEnds

open ForgeCam.Dist2 ForgeCam.DistArc ForgeCam.Trim

/-- **The chord to a′ shrinks inside the cone.** u = A₀ − c, v = a′ − c on the circle |·|² = R,
⟨u, v⟩ ≥ 0, and x = s·(λu + μv) (s > 0, λ, μ ≥ 0) on the circle too: then x is no farther from v
than u is. -/
theorem chord_mono {u v x : P} {R s l m : ℝ} (hu : nsq u = R) (hv : nsq v = R) (hk : 0 ≤ dot u v)
    (hs : 0 < s) (hl : 0 ≤ l) (hm : 0 ≤ m) (hx : x = s • (l • u + m • v)) (hxR : nsq x = R) :
    nsq (x - v) ≤ nsq (u - v) := by
  -- |x − v|² = 2R − 2⟨x, v⟩ and |u − v|² = 2R − 2⟨u, v⟩: it is ⟨x, v⟩ ≥ ⟨u, v⟩
  have hxv0 : dot x v = s * (l * dot u v + m * nsq v) := by
    subst hx; simp only [dot, nsq, Prod.smul_fst, Prod.smul_snd, Prod.fst_add, Prod.snd_add,
      smul_eq_mul]; ring
  have hN0 : nsq x = s ^ 2 * (l ^ 2 * nsq u + m ^ 2 * nsq v + 2 * l * m * dot u v) := by
    subst hx; simp only [dot, nsq, Prod.smul_fst, Prod.smul_snd, Prod.fst_add, Prod.snd_add,
      smul_eq_mul]; ring
  have hcs0 := lagrange u v
  set k := dot u v with hkdef
  rw [hv] at hxv0
  rw [hu, hv] at hN0 hcs0
  have hcs : k ^ 2 ≤ R * R := by nlinarith [sq_nonneg (cross v u)]
  have hR : 0 ≤ R := hu ▸ nsq_nonneg u
  have hxv : dot x v = s * (l * k + m * R) := hxv0
  have hN : nsq x = s ^ 2 * (l ^ 2 * R + m ^ 2 * R + 2 * l * m * k) := hN0
  have key : k ≤ s * (l * k + m * R) := by
    have hpos : 0 ≤ s * (l * k + m * R) := by positivity
    -- (s(lk + mR))² ≥ k²: R(lk + mR)² − k²(l²R + m²R + 2lmk) = (2lmk + Rm²)(R² − k²) ≥ 0
    have hid : R * (l * k + m * R) ^ 2 - k ^ 2 * (l ^ 2 * R + m ^ 2 * R + 2 * l * m * k)
        = (2 * l * m * k + R * m ^ 2) * (R * R - k ^ 2) := by ring
    have hge : k ^ 2 * (l ^ 2 * R + m ^ 2 * R + 2 * l * m * k) ≤ R * (l * k + m * R) ^ 2 := by
      have : 0 ≤ (2 * l * m * k + R * m ^ 2) * (R * R - k ^ 2) :=
        mul_nonneg (by positivity) (by linarith)
      linarith
    -- multiply through by s² and use s²N = R
    have hsq : k ^ 2 * R ≤ (s * (l * k + m * R)) ^ 2 * R := by
      rw [hN] at hxR
      have h2 : k ^ 2 * R = k ^ 2 * (s ^ 2 * (l ^ 2 * R + m ^ 2 * R + 2 * l * m * k)) := by
        rw [hxR]
      rw [h2]
      have := mul_le_mul_of_nonneg_left hge (sq_nonneg s)
      nlinarith [this]
    rcases eq_or_lt_of_le hR with h0 | hRpos
    · -- R = 0: u = 0, so k = 0
      have : k ^ 2 ≤ 0 := by rw [← h0] at hcs; simpa using hcs
      have hk0 : k = 0 := by nlinarith [sq_nonneg k]
      rw [hk0]; positivity
    · have : k ^ 2 ≤ (s * (l * k + m * R)) ^ 2 := le_of_mul_le_mul_right hsq hRpos
      exact (abs_le_of_sq_le_sq' this hpos).2
  have e1 : nsq (x - v) = nsq x + nsq v - 2 * dot x v := by
    simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring
  have e2 : nsq (u - v) = nsq u + nsq v - 2 * dot u v := by
    simp only [nsq, dot, Prod.fst_sub, Prod.snd_sub]; ring
  rw [e1, e2, hxR, hu, hxv]
  linarith

/-- **The ray's circle point is near the true end.** A₀ on the circle of radius r about c, a
within ε of A₀ and off the centre, a′ = c + (r/|a − c|)(a − c): then |A₀ − a′| ≤ 2ε. -/
theorem proj_near {c A a : P} {r ε : ℝ} (hA : d2 A c = r) (ha : 0 < d2 a c)
    (hε : d2 A a ≤ ε) :
    d2 A (c + (r / d2 a c) • (a - c)) ≤ 2 * ε := by
  set ρ := d2 a c with hρ
  set a' := c + (r / ρ) • (a - c) with ha'
  -- |a − a′| = |ρ − r|
  have haa' : d2 a a' = |ρ - r| := by
    have : a - a' = (1 - r / ρ) • (a - c) := by
      rw [ha']; ext <;> simp <;> ring
    have hd : d2 a a' = |1 - r / ρ| * ρ := by
      rw [d2_eq_sqrt, ← nsq_comm, this]
      simp only [nsq, dot, Prod.smul_fst, Prod.smul_snd, smul_eq_mul]
      have : (1 - r / ρ) * (a - c).1 * ((1 - r / ρ) * (a - c).1)
          + (1 - r / ρ) * (a - c).2 * ((1 - r / ρ) * (a - c).2)
          = (1 - r / ρ) ^ 2 * ((a - c).1 * (a - c).1 + (a - c).2 * (a - c).2) := by ring
      rw [this, Real.sqrt_mul (sq_nonneg _), Real.sqrt_sq_eq_abs]
      congr 1
      rw [hρ, d2_eq_sqrt, ← nsq_comm]
      simp [nsq, dot]
    rw [hd, ← abs_of_pos ha, ← abs_mul, abs_of_pos ha]
    congr 1
    field_simp
  -- reverse triangle: |ρ − r| ≤ |a − A₀|
  have h1 : ρ ≤ d2 a A + r := by
    have := d2_triangle a A c; rw [← hA]; linarith [this]
  have h2 : r ≤ d2 A a + ρ := by
    have := d2_triangle A a c; rw [hA] at this; linarith [this]
  have hrev : |ρ - r| ≤ ε := by
    rw [d2_comm a A] at h1
    exact abs_le.mpr ⟨by linarith, by linarith⟩
  have := d2_triangle A a a'
  rw [haa'] at this
  linarith

end ForgeCam.PathEnds
