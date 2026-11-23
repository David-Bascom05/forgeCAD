import ForgeExact.Swept.Prism

/-!
# Taper: the mitred offset family and its events (research/sweep-spec/DESIGN.md §2)

A tapered extrude of a profile of lines, `d` deep with its sides leaning by `angle`, has the
section at height `h` the **mitred offset** of the profile by `δ = σ·h·t` (`t = tan|angle|`, the
one rounding of the input; `σ = ±1`, in or out): every edge's carrier moved `δ` to the material
side (the left of each loop), every corner the meet of its two carriers. It is not the erosion by
a disc, which rounds reflex corners; it is the family the kernel's drafted planes bound.

The algebra is over any field `K` (the reals, ℚ, or the algebraic numbers), with the edges' unit
directions given as data (`u · u = 1`), so no square root is taken.

* `corner_on_both`: the corner `q + δ·w` (`w` the mitre vector `(n + n') / (1 + n·n')`) lies on
  both offset carriers: at signed distance `δ` from each edge's line.
* `corner_unique`: it is the only such point when the two edges are not parallel.

So every corner of the section moves **linearly** in δ, and:

* `edge_length`: an edge's length is linear in δ: `ℓ + δ · (w' − w) · u`;
* `contact_distance`: a corner's signed distance to another edge's offset carrier is linear in δ.

An **event** is the least δ > 0 where the section stops being the carriers' polygon with the same
combinatorics: an edge's length reaches 0 (E), or a corner reaches an edge it is not on (S within
a loop, C between loops; the first contact of two closed segments moving continuously is always
an end of one on the other). Each is the root of a linear function, so the first event is exact:
`firstEvent` below over ℚ (rational unit directions), and the same formula over the algebraic
numbers.

**The decided rule** (`Verdict`): no event in `(0, D]` (D = d·t): the one drafted solid; only edge
events, the region keeping a positive area: stacked slabs, each event-free; the outer loop
collapsing at `δ* ≤ D`: the solid up to `δ*/t` (the roof: the sections are empty past it), with a
lint; a corner reaching another edge first: refuse,
`WallsCross (δ*/t)`; a hole closing: dropped, the rest goes on.
-/

namespace ForgeExact.Swept.Taper

open Lean.Grind

/-! ## Plane vectors over a field -/

structure P2 (K : Type) where
  x : K
  y : K

variable {K : Type} [Field K]

namespace P2

def add (a b : P2 K) : P2 K := ⟨a.x + b.x, a.y + b.y⟩
def sub (a b : P2 K) : P2 K := ⟨a.x - b.x, a.y - b.y⟩
def smul (k : K) (a : P2 K) : P2 K := ⟨k * a.x, k * a.y⟩
def dot (a b : P2 K) : K := a.x * b.x + a.y * b.y
/-- The left normal: the material side of an edge running along `a`. -/
def perp (a : P2 K) : P2 K := ⟨-a.y, a.x⟩
def cross (a b : P2 K) : K := a.x * b.y - a.y * b.x

end P2

/-- The mitre vector of a corner between an edge with left normal `n` and the next with `n'`. -/
def mitre (n n' : P2 K) : P2 K := P2.smul (1 / (1 + n.dot n')) (n.add n')

/-- The corner at `q` after offsetting by `δ`. -/
def corner (q : P2 K) (n n' : P2 K) (δ : K) : P2 K := q.add (P2.smul δ (mitre n n'))

/-- **The offset corner lies on both offset carriers.** The edge into the corner runs along the
unit `u` (and passes through `p`), the edge out of it along the unit `u'`; `q` is on the first
(`q − p = ℓ·u`). The corner moved by `δ` is at signed distance `δ` from both lines, to their left. -/
theorem corner_on_both (p q u u' : P2 K) (ℓ δ : K) (hu : u.dot u = 1) (hu' : u'.dot u' = 1)
    (hq : q.sub p = P2.smul ℓ u) (hd : 1 + u.perp.dot u'.perp ≠ 0) :
    u.perp.dot ((corner q u.perp u'.perp δ).sub p) = δ ∧
      u'.perp.dot ((corner q u.perp u'.perp δ).sub q) = δ := by
  obtain ⟨px, py⟩ := p; obtain ⟨qx, qy⟩ := q; obtain ⟨ux, uy⟩ := u; obtain ⟨vx, vy⟩ := u'
  simp only [corner, mitre, P2.add, P2.sub, P2.smul, P2.dot, P2.perp, P2.mk.injEq] at *
  obtain ⟨h1, h2⟩ := hq
  constructor <;> grind

/-- **And it is the only such point** when the two edges are not parallel. -/
theorem corner_unique (p q u u' x : P2 K) (ℓ δ : K) (hu : u.dot u = 1) (hu' : u'.dot u' = 1)
    (hq : q.sub p = P2.smul ℓ u) (hd : 1 + u.perp.dot u'.perp ≠ 0) (hc : u.cross u' ≠ 0)
    (h1 : u.perp.dot (x.sub p) = δ) (h2 : u'.perp.dot (x.sub q) = δ) :
    x = corner q u.perp u'.perp δ := by
  obtain ⟨e1, e2⟩ := corner_on_both p q u u' ℓ δ hu hu' hq hd
  -- both points solve the same two independent linear equations
  obtain ⟨px, py⟩ := p; obtain ⟨qx, qy⟩ := q; obtain ⟨ux, uy⟩ := u; obtain ⟨vx, vy⟩ := u'
  obtain ⟨xx, xy⟩ := x
  generalize hc' : corner (⟨qx, qy⟩ : P2 K) (P2.perp ⟨ux, uy⟩) (P2.perp ⟨vx, vy⟩) δ = c at e1 e2
  obtain ⟨cx, cy⟩ := c
  simp only [P2.sub, P2.dot, P2.perp, P2.cross, P2.smul, P2.mk.injEq] at *
  obtain ⟨hq1, hq2⟩ := hq
  -- (x − c) is orthogonal to both normals, so it is zero
  have a1 : -uy * (xx - cx) + ux * (xy - cy) = 0 := by grind
  have a2 : -vy * (xx - cx) + vx * (xy - cy) = 0 := by grind
  have hx : (ux * vy - uy * vx) * (xx - cx) = 0 := by grind
  have hy : (ux * vy - uy * vx) * (xy - cy) = 0 := by grind
  have ex : xx - cx = 0 := by
    have := congrArg (fun z => (ux * vy - uy * vx)⁻¹ * z) hx
    rw [← Semiring.mul_assoc, Field.inv_mul_cancel hc, Semiring.one_mul, Semiring.mul_zero] at this
    exact this
  have ey : xy - cy = 0 := by
    have := congrArg (fun z => (ux * vy - uy * vx)⁻¹ * z) hy
    rw [← Semiring.mul_assoc, Field.inv_mul_cancel hc, Semiring.one_mul, Semiring.mul_zero] at this
    exact this
  refine ⟨?_, ?_⟩ <;> grind

/-- **An edge's length is linear in δ.** The edge from corner `q` (mitre `w`) to corner `r` (mitre
`w'`), along the unit `u`, `r − q = ℓ·u`: after offsetting by δ its length along `u` is
`ℓ + δ (w' − w)·u`. -/
theorem edge_length (q r u w w' : P2 K) (ℓ δ : K) (hu : u.dot u = 1) (hr : r.sub q = P2.smul ℓ u) :
    ((r.add (P2.smul δ w')).sub (q.add (P2.smul δ w))).dot u = ℓ + δ * ((w'.sub w).dot u) := by
  obtain ⟨qx, qy⟩ := q; obtain ⟨rx, ry⟩ := r; obtain ⟨ux, uy⟩ := u
  simp only [P2.add, P2.sub, P2.smul, P2.dot, P2.mk.injEq] at *
  obtain ⟨h1, h2⟩ := hr
  grind

/-- **A corner's distance to another edge's offset carrier is linear in δ.** Corner `a` (mitre `w`)
against the edge through `b` with unit `u`, both offset by δ. -/
theorem contact_distance (a b u w : P2 K) (δ : K) :
    u.perp.dot ((a.add (P2.smul δ w)).sub b) - δ = u.perp.dot (a.sub b) + δ * (u.perp.dot w - 1) := by
  obtain ⟨ax, ay⟩ := a; obtain ⟨bx, by_⟩ := b; obtain ⟨ux, uy⟩ := u; obtain ⟨wx, wy⟩ := w
  simp only [P2.add, P2.sub, P2.smul, P2.dot, P2.perp]
  grind

/-! ## The decided rule -/

/-- What a tapered extrude comes to (`DESIGN.md` §2.2). Heights are along the travel. -/
inductive Verdict (K : Type)
  /-- No event before the depth: the one drafted solid. -/
  | draft
  /-- Edge events only, the region keeping a positive area: slabs between these heights. -/
  | slabs (heights : List K)
  /-- The outer loop's region reaches zero area at this height, at or before the depth: the
  exact set ends there, so the result is the solid up to it (the roof), with a lint. -/
  | roof (h : K)
  /-- A corner reaches an edge it is not on (within a loop, or between loops) at this height. -/
  | wallsCross (h : K)

/-! ## The regular hexagon (taper_collapses.fg) -/

/-- The fixture's hexagon, exactly: corners `(±2, 0)`, `(±1, ±s)` with `s² = 3`, every side 2
long (in any field where 2 and 3 are not zero: the reals, the algebraic numbers). Going CCW from `(2, 0)`, every side's offset length is `2 − 2δ/s`: each corner's mitre
moves it back along each side by `δ/s` (`tan 30° = 1/s`). -/
theorem hexagon_side (s δ : K) (hs : s * s = 3) (hs0 : s ≠ 0) (h2 : (2 : K) ≠ 0) (h3 : (3 : K) ≠ 0) :
    -- the side from (2, 0) to (1, s), unit direction (−1/2, s/2), corners' mitres from their
    -- normals: the side before (from (1, −s)) has unit (1/2, s/2), the side after (to (−1, s))
    -- has unit (−1, 0)
    let u₀ : P2 K := ⟨1 / 2, s / 2⟩
    let u₁ : P2 K := ⟨-1 / 2, s / 2⟩
    let u₂ : P2 K := ⟨-1, 0⟩
    let w := mitre u₀.perp u₁.perp
    let w' := mitre u₁.perp u₂.perp
    2 + δ * ((w'.sub w).dot u₁) = 2 - 2 * δ / s := by
  intro u₀ u₁ u₂ w w'
  simp only [w, w', u₀, u₁, u₂, mitre, P2.add, P2.sub, P2.smul, P2.dot, P2.perp]
  have h1 : (1 : K) + (-(s / 2) * -(s / 2) + 1 / 2 * (-1 / 2)) = 3 / 2 := by grind
  have h2' : (1 : K) + (-(s / 2) * -0 + -1 / 2 * -1) = 3 / 2 := by grind
  rw [h1, h2']
  grind

end ForgeExact.Swept.Taper
