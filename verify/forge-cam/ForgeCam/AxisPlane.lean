/-
axisplane.rs's closed form (kernel/forge-cam-geom/src/axisplane.rs): a separation along one
coordinate axis is a lower bound on the distance, and on a line move it is least at an end.

The body at the tip t lies in t + [lo, hi] along axis k (lo, hi: z0, z1 along z; −rmax, rmax
along x or y). The face lies in a box whose extent along k is [flo, fhi]; the true object is
within mu of the face, so along k within [flo − mu, fhi + mu]. With the tip t(s) = a + s·(b − a)
for s in [0, 1]:

- `sep_le_dist`: a separation g ≥ 0 along one coordinate bounds the squared distance below.
- `lin_ge_min`: an affine function of s on [0, 1] is at least its least end.
- `axisplane_clear`: if the least end gap (a_k + lo − fhi or b_k + lo − fhi) is at least
  limit + mu (limit ≥ 0), every body point at every instant is at least limit from every point
  of the true object (in squares). The other side (flo − (t_k + hi)) is the same with the axis
  negated (`axisplane_clear_below`).

Over any ordered field, so the same statements hold over ℚ, where axisplane.rs decides the
comparison exactly (real.rs); the code takes the best over axes and sides, each a bound alone.
-/
import Mathlib

set_option linter.unusedSectionVars false

namespace ForgeCam.AxisPlane

variable {K : Type*} [Field K] [LinearOrder K] [IsStrictOrderedRing K]

/-- The squared distance of two points in K³. -/
def nsq3 (p q : Fin 3 → K) : K := ∑ i, (p i - q i) ^ 2

/-- **A separation along one coordinate bounds the distance**: if p_k − q_k ≥ g ≥ 0, then
g² ≤ |p − q|². -/
theorem sep_le_dist (p q : Fin 3 → K) (k : Fin 3) {g : K} (hg : 0 ≤ g)
    (h : g ≤ p k - q k) : g ^ 2 ≤ nsq3 p q := by
  unfold nsq3
  have h1 : g ^ 2 ≤ (p k - q k) ^ 2 := by nlinarith
  have h2 : (p k - q k) ^ 2 ≤ ∑ i, (p i - q i) ^ 2 :=
    Finset.single_le_sum (f := fun i => (p i - q i) ^ 2) (fun i _ => sq_nonneg _)
      (Finset.mem_univ k)
  linarith

/-- **An affine function on [0, 1] is at least its least end.** -/
theorem lin_ge_min (x y s : K) (h0 : 0 ≤ s) (h1 : s ≤ 1) :
    min x y ≤ x + s * (y - x) := by
  rcases le_total x y with hxy | hxy
  · rw [min_eq_left hxy]; nlinarith
  · rw [min_eq_right hxy]; nlinarith

/-- **The closed form's Clear, the body above (or right of) the face along k.** The tip at
a + s·(b − a); a body point p with p_k ≥ tip_k + lo; a point q of the true object with
q_k ≤ fhi + mu. If both ends' gaps (end_k + lo − fhi) are at least limit + mu, with
limit ≥ 0, then limit² ≤ |p − q|². -/
theorem axisplane_clear (a b p q : Fin 3 → K) (k : Fin 3) (lo fhi mu limit s : K)
    (h0 : 0 ≤ s) (h1 : s ≤ 1) (hl : 0 ≤ limit)
    (hp : a k + s * (b k - a k) + lo ≤ p k) (hq : q k ≤ fhi + mu)
    (hend : limit + mu ≤ min (a k + lo - fhi) (b k + lo - fhi)) :
    limit ^ 2 ≤ nsq3 p q := by
  have hm := lin_ge_min (a k + lo - fhi) (b k + lo - fhi) s h0 h1
  apply sep_le_dist p q k hl
  nlinarith

/-- The other side: the body below (or left of) the face along k. A body point p with
p_k ≤ tip_k + hi, a point q of the true object with q_k ≥ flo − mu; the ends' gaps are
flo − (end_k + hi). -/
theorem axisplane_clear_below (a b p q : Fin 3 → K) (k : Fin 3) (hi flo mu limit s : K)
    (h0 : 0 ≤ s) (h1 : s ≤ 1) (hl : 0 ≤ limit)
    (hp : p k ≤ a k + s * (b k - a k) + hi) (hq : flo - mu ≤ q k)
    (hend : limit + mu ≤ min (flo - (a k + hi)) (flo - (b k + hi))) :
    limit ^ 2 ≤ nsq3 p q := by
  have hm := lin_ge_min (flo - (a k + hi)) (flo - (b k + hi)) s h0 h1
  have hsym : nsq3 p q = nsq3 q p := by
    unfold nsq3; congr 1; funext i; ring
  rw [hsym]
  apply sep_le_dist q p k hl
  nlinarith

end ForgeCam.AxisPlane
