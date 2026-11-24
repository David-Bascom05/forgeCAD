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
* `smear_hull`, `eroded_gap`: the smear vertex by vertex (it is convex in x, so by Jensen the
  smeared swept core lies in the hull of the swept vertices and their own smears), and a slab
  shrunk past the nose radius (the core eroded by e: every axis gap gains e, the smear taken e
  nearer the axis).
* `rect_in_convex`: the in-process stock as columns (stair.rs) loses only rectangles that lie in
  a cut's swept outline, so it stays a superset of the stock (G3 against it is sound).
* `turn_clear`: the assembly, by `clear_of_frontier`: a connected swept set Z (convex) whose
  frontier pieces of Hf all keep `c > 0` from Z, with one point of Z outside Hf, keeps `c` from
  all of Hf; its r-neighbourhood keeps `c − r` (`turn_clear_ball`).

* `sep_axis_hull`: the swept core Z is the convex hull of finitely many points P; for each
  boundary piece f of the obstacle and any axis w, the gap between max over P and min over f of
  w·_ bounds dist(Z, f) — check.rs's only distance bound, so no enumeration of Z's boundary.

Hypotheses this file does not discharge (research/cam/turning.md lists them):
* H.arc: a control's arc between ends at unequal radii stays between the radii.
* The obstacle's pieces cover its frontier (the half-profile's construction), and Z = conv P
  (K = conv of its vertices; then `zono_eq_hull`).
-/
import ForgeCam.SignedDist

open Metric Set Real
open scoped Pointwise

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

/-- The dot product with an axis w. -/
def wdot (w p : ℝ × ℝ) : ℝ := w.1 * p.1 + w.2 * p.2

/-- The separating axis between hulls (check.rs `sat_lower`): if every point of P has w·p ≤ M
and every point of F has m ≤ w·y, with M ≤ m, then every point of conv P and every point of
conv F are at least (m − M)/|w| apart: (m − M)² ≤ |w|² · d2. The max of a linear map over a hull
is its max over the points (a half-plane is convex). -/
theorem sep_axis_hull {P F : Set (ℝ × ℝ)} {w : ℝ × ℝ} {M m : ℝ} (hMm : M ≤ m)
    (hP : ∀ p ∈ P, wdot w p ≤ M) (hF : ∀ y ∈ F, m ≤ wdot w y) :
    ∀ z ∈ convexHull ℝ P, ∀ y ∈ convexHull ℝ F, (m - M) ^ 2 ≤ (w.1 ^ 2 + w.2 ^ 2) * d2 z y := by
  have lin : IsLinearMap ℝ (fun z : ℝ × ℝ => wdot w z) := by
    constructor
    · intro a b; unfold wdot; simp only [Prod.fst_add, Prod.snd_add]; ring
    · intro c a; unfold wdot; simp only [Prod.smul_fst, Prod.smul_snd, smul_eq_mul]; ring
  have hullP : convexHull ℝ P ⊆ {z | wdot w z ≤ M} :=
    convexHull_min (fun p hp => hP p hp) (convex_halfSpace_le lin M)
  have hullF : convexHull ℝ F ⊆ {z | m ≤ wdot w z} :=
    convexHull_min (fun y hy => hF y hy) (convex_halfSpace_ge lin m)
  intro z hz y hy
  have h1 : wdot w z ≤ M := hullP hz
  have h2 : m ≤ wdot w y := hullF hy
  have h3 : 0 ≤ m - M := by linarith
  have h4 : m - M ≤ wdot w y - wdot w z := by linarith
  have cs : (wdot w y - wdot w z) ^ 2 ≤ (w.1 ^ 2 + w.2 ^ 2) * d2 z y := by
    unfold wdot d2
    nlinarith [sq_nonneg (w.1 * (y.2 - z.2) - w.2 * (y.1 - z.1))]
  nlinarith [h3, h4, cs, mul_self_le_mul_self h3 h4]

/-- The swept core is the hull of finitely many points (check.rs builds P so): conv V ⊕ [0, g₁] ⊕
[0, g₂] = conv (V + {0, g₁} + {0, g₂}). -/
theorem zono_eq_hull (V : Set (ℝ × ℝ)) (g1 g2 : ℝ × ℝ) :
    convexHull ℝ V + segment ℝ 0 g1 + segment ℝ 0 g2 = convexHull ℝ (V + {0, g1} + {0, g2}) := by
  rw [convexHull_add, convexHull_add, convexHull_pair, convexHull_pair]

/-- The smear, vertex by vertex (check.rs `body_line`): g convex and nonnegative (the smear
√(x² + b²) − x of depth b, convex in x), every point z of conv V moved out along x by any
δ ∈ [0, g(z.1)] stays in the hull of the vertices and the vertices moved out by their own g. -/
theorem smear_hull {V : Set (ℝ × ℝ)} {g : ℝ → ℝ} (hg : ConvexOn ℝ univ g) (hg0 : ∀ x, 0 ≤ g x) :
    ∀ z ∈ convexHull ℝ V, ∀ δ, 0 ≤ δ → δ ≤ g z.1 →
      z + (δ, 0) ∈ convexHull ℝ (V ∪ (fun v => v + (g v.1, (0 : ℝ))) '' V) := by
  set H := convexHull ℝ (V ∪ (fun v => v + (g v.1, (0 : ℝ))) '' V)
  -- the points every smear of which stays in H: a convex set holding V
  let S : Set (ℝ × ℝ) := {z | ∀ δ, 0 ≤ δ → δ ≤ g z.1 → z + (δ, 0) ∈ H}
  have hV : V ⊆ S := by
    intro v hv δ h0 h1
    have hA : v ∈ H := subset_convexHull ℝ _ (Or.inl hv)
    have hB : v + (g v.1, (0 : ℝ)) ∈ H := subset_convexHull ℝ _ (Or.inr ⟨v, hv, rfl⟩)
    rcases eq_or_lt_of_le (hg0 v.1) with hz | hpos
    · have : δ = 0 := by linarith
      subst this
      simpa only [Prod.mk_zero_zero, add_zero] using hA
    · -- v + δ e₁ = (1 − t)·v + t·(v + g e₁), t = δ / g
      have ht0 : 0 ≤ δ / g v.1 := div_nonneg h0 hpos.le
      have ht1 : δ / g v.1 ≤ 1 := (div_le_one hpos).mpr h1
      have := (convex_convexHull ℝ _) hA hB (by linarith : 0 ≤ 1 - δ / g v.1) ht0 (by ring)
      convert this using 1
      ext <;> simp [Prod.smul_mk, smul_eq_mul] <;> field_simp <;> ring
  have hS : Convex ℝ S := by
    intro x hx y hy a b ha hb hab δ h0 h1
    -- g at the combination is at most the combination of g's
    have hgc : g (a • x + b • y).1 ≤ a * g x.1 + b * g y.1 := by
      have := hg.2 (mem_univ x.1) (mem_univ y.1) ha hb hab
      simpa [smul_eq_mul] using this
    set D := a * g x.1 + b * g y.1 with hD
    rcases eq_or_lt_of_le (add_nonneg (mul_nonneg ha (hg0 x.1)) (mul_nonneg hb (hg0 y.1))) with hz | hpos
    · have : δ = 0 := by
        have : δ ≤ 0 := by
          calc δ ≤ g (a • x + b • y).1 := h1
            _ ≤ D := hgc
            _ = 0 := hz.symm
        linarith
      subst this
      have hx0 := hx 0 le_rfl (hg0 _)
      have hy0 := hy 0 le_rfl (hg0 _)
      have := (convex_convexHull ℝ _) hx0 hy0 ha hb hab
      simpa only [Prod.mk_zero_zero, add_zero] using this
    · -- δ = a·(g x · δ/D) + b·(g y · δ/D), each share within its own g
      set k := δ / D
      have hk0 : 0 ≤ k := div_nonneg h0 hpos.le
      have hk1 : k ≤ 1 := (div_le_one hpos).mpr (le_trans h1 hgc)
      have hxs := hx (g x.1 * k) (mul_nonneg (hg0 _) hk0) (by nlinarith [hg0 x.1])
      have hys := hy (g y.1 * k) (mul_nonneg (hg0 _) hk0) (by nlinarith [hg0 y.1])
      have := (convex_convexHull ℝ _) hxs hys ha hb hab
      convert this using 1
      have hδ : δ = D * k := by rw [mul_div_cancel₀ _ hpos.ne']
      ext <;> simp [Prod.smul_mk, smul_eq_mul] <;> rw [hδ, hD] <;> ring
  exact convexHull_min hV hS

/-- The eroded section's axis bound (check.rs `axis_gap`'s `ero`): p has the disc of radius e
about it in K; every point q of K has w·q ≤ M and, with the smear taken e nearer the axis,
w·q + w.1·g(q.1 − e) ≤ M (g antitone: the smear falls as x grows). Then p moved
out by any δ ≤ g(p.1) has w·_ ≤ M − e·|w|: every axis gap gains e. -/
theorem eroded_gap {K : Set (ℝ × ℝ)} {g : ℝ → ℝ} {w p : ℝ × ℝ} {M e δ : ℝ} (he : 0 ≤ e)
    (hg : Antitone g) (hw : w ≠ 0)
    (hball : ∀ u, d2 u p ≤ e ^ 2 → u ∈ K)
    (hK : ∀ q ∈ K, wdot w q ≤ M) (hKs : ∀ q ∈ K, wdot w q + w.1 * g (q.1 - e) ≤ M)
    (hδ0 : 0 ≤ δ) (hδ : δ ≤ g p.1) :
    wdot w (p + (δ, 0)) + e * √(w.1 ^ 2 + w.2 ^ 2) ≤ M := by
  set L := √(w.1 ^ 2 + w.2 ^ 2) with hL
  have hpos : 0 < w.1 ^ 2 + w.2 ^ 2 := by
    rcases w with ⟨w1, w2⟩
    by_contra h
    have h1 : w1 = 0 := by nlinarith [sq_nonneg w1, sq_nonneg w2]
    have h2 : w2 = 0 := by nlinarith [sq_nonneg w1, sq_nonneg w2]
    exact hw (by simp [h1, h2])
  have hLpos : 0 < L := sqrt_pos.mpr hpos
  have hLsq : L ^ 2 = w.1 ^ 2 + w.2 ^ 2 := sq_sqrt hpos.le
  -- q = p + e·w/|w| is in K
  set q : ℝ × ℝ := (p.1 + e * w.1 / L, p.2 + e * w.2 / L)
  have hq : q ∈ K := by
    apply hball
    unfold d2; simp only [q]
    have : (e * w.1 / L) ^ 2 + (e * w.2 / L) ^ 2 = e ^ 2 := by
      rw [div_pow, div_pow, ← add_div, mul_pow, mul_pow, ← mul_add, ← hLsq]
      field_simp
    nlinarith [this]
  have hwq : wdot w q = wdot w p + e * L := by
    unfold wdot; simp only [q]; field_simp
    linear_combination (-e) * hLsq
  have hq1 : q.1 - e ≤ p.1 := by
    simp only [q]
    have : w.1 ≤ L := by
      rw [hL]; exact le_sqrt_of_sq_le (by nlinarith [sq_nonneg w.2])
    have : e * w.1 / L ≤ e := by
      rw [div_le_iff₀ hLpos]; nlinarith
    linarith
  have hgg : δ ≤ g (q.1 - e) := le_trans hδ (hg hq1)
  unfold wdot at *
  simp only [Prod.fst_add, Prod.snd_add, add_zero] at *
  rcases le_or_gt 0 w.1 with h1 | h1
  · have := hKs q hq
    nlinarith [mul_le_mul_of_nonneg_left hgg h1]
  · have := hK q hq
    nlinarith [mul_nonneg_of_nonpos_of_nonpos h1.le (by linarith : -δ ≤ 0)]

/-- The column update (stair.rs `clear_poly`): Q convex; at x0 and x1 its slices reach down to
l0, l1 and up to h0, h1. Then the rectangle [x0, x1] × [max l, min h] lies in Q: at x between,
the two points (x, (1−λ) l0 + λ l1) and (x, (1−λ) h0 + λ h1) are in Q, and z lies between them.
So a cut's swept outline holds the rectangle, and where the rectangle reaches a column's top the
column's material above its bottom is gone: the columns stay a superset of the stock. -/
theorem rect_in_convex {Q : Set (ℝ × ℝ)} (hQ : Convex ℝ Q) {x0 x1 l0 l1 h0 h1 : ℝ}
    (hx : x0 ≤ x1) (a0 : (x0, l0) ∈ Q) (b0 : (x0, h0) ∈ Q) (a1 : (x1, l1) ∈ Q) (b1 : (x1, h1) ∈ Q) :
    ∀ x z, x0 ≤ x → x ≤ x1 → max l0 l1 ≤ z → z ≤ min h0 h1 → (x, z) ∈ Q := by
  intro x z hx0 hx1 hzl hzh
  -- λ along x
  rcases eq_or_lt_of_le hx with he | hlt
  · -- a single column: x = x0 = x1, z between l0 and h0
    have hxx : x = x0 := by linarith
    subst hxx
    have hl : l0 ≤ z := le_trans (le_max_left _ _) hzl
    have hh : z ≤ h0 := le_trans hzh (min_le_left _ _)
    rcases eq_or_lt_of_le (le_trans hl hh) with heq | hlt2
    · have : z = l0 := by linarith
      subst this; exact a0
    · set t := (z - l0) / (h0 - l0)
      have ht0 : 0 ≤ t := div_nonneg (by linarith) (by linarith)
      have ht1 : t ≤ 1 := (div_le_one (by linarith)).mpr (by linarith)
      have := hQ a0 b0 (by linarith : 0 ≤ 1 - t) ht0 (by ring)
      convert this using 1
      ext <;> simp [smul_eq_mul, t] <;> field_simp <;> ring
  · set t := (x - x0) / (x1 - x0)
    have ht0 : 0 ≤ t := div_nonneg (by linarith) (by linarith)
    have ht1 : t ≤ 1 := (div_le_one (by linarith)).mpr (by linarith)
    have hA := hQ a0 a1 (by linarith : 0 ≤ 1 - t) ht0 (by ring)
    have hB := hQ b0 b1 (by linarith : 0 ≤ 1 - t) ht0 (by ring)
    have hxt : (1 - t) * x0 + t * x1 = x := by simp only [t]; field_simp; ring
    set L := (1 - t) * l0 + t * l1
    set H := (1 - t) * h0 + t * h1
    have hL : L ≤ z := by
      have : L ≤ max l0 l1 := by
        simp only [L]; nlinarith [le_max_left l0 l1, le_max_right l0 l1]
      linarith
    have hH : z ≤ H := by
      have : min h0 h1 ≤ H := by
        simp only [H]; nlinarith [min_le_left h0 h1, min_le_right h0 h1]
      linarith
    have hA' : (x, L) ∈ Q := by
      convert hA using 1; ext <;> simp [smul_eq_mul, L] <;> linarith [hxt]
    have hB' : (x, H) ∈ Q := by
      convert hB using 1; ext <;> simp [smul_eq_mul, H] <;> linarith [hxt]
    rcases eq_or_lt_of_le (le_trans hL hH) with heq | hlt2
    · have : z = L := by linarith
      rw [this]; exact hA'
    · set u := (z - L) / (H - L)
      have hu0 : 0 ≤ u := div_nonneg (by linarith) (by linarith)
      have hu1 : u ≤ 1 := (div_le_one (by linarith)).mpr (by linarith)
      have := hQ hA' hB' (by linarith : 0 ≤ 1 - u) hu0 (by ring)
      convert this using 1
      ext <;> simp [smul_eq_mul, u] <;> field_simp <;> ring

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
