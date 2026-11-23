# RESUME: tapered extrudes of spline outlines

Status: not built. Taper refuses splines today (`skin::curved`), and nothing here changes that. The
master's rule was to build this only if it could be done with exact geometry and no search. It
can't, so this note records what would be needed. Head when written: 701b4f6
(claude/repo-review-mbdrlj-fix-draft).

## What is exact already, and would carry over

- **The wall.** A drafted spline wall is a ruled surface, `S(t, z) = C(t) + z (k + tan a N(t))`,
  where C is the outline's `PlaneSpline` (polynomial or rational pieces, `forge_geom::spline`),
  N its unit normal in the sketch plane and k the extrude axis. It is not a NURBS, because N has
  `|C'(t)|` in its denominator. It is exact as a `Surface::Procedural` (`forge_geom::surface::Proc`):
  point, first and second derivatives in closed form from C's own. `draft::drafted` would need
  this case; today a spline wall answers `NoExactSurface`.
- **Edges against other walls.** Where the spline wall meets a plane wall (a line side) or a cone
  wall (an arc side), the edge is a traced `Curve::Intersection`, as for any procedural pair.
  `rebuild::edge_curve` already does this. At the bottom and top caps the edge is the
  spline itself and its offset: the offset is `Curve::Procedural`, C(t) + δ N(t), and exact.
- **The cusp event (an E for a spline).** The offset C + δN folds (a swallowtail) first where
  `δ κ(t) = 1` on the side it moves to, with κ = (x'y'' − y'x'') / |C'|³. Per polynomial piece,
  the extreme of κ comes from the roots of a polynomial: d/dt of κ² with |C'|⁶ cleared (for
  rational pieces, after clearing the weights too). So the first cusp δ* = 1 / max κ comes out
  exactly from `forge_math::poly::real_roots` on each span. A spline whose offset folds would be
  refused at δ*, as WallsCross (a fold is the offset crossing itself), unless the arc length up to
  the fold is gone, which is the same chain test as `gone_at`.
- **A spline's ends against a line or an arc neighbour.** The joint is where C(t) + δ N(t) meets
  the moved line or circle. That is one equation in (t, δ): polynomial in t and δ after squaring
  out |C'|. Following the joint as δ grows means following one branch of a plane algebraic curve,
  from the known point at δ = 0. Its E event, where the neighbour's side length runs out, is that
  curve meeting one more polynomial equation: a resultant in t gives a one-variable polynomial in
  δ. That's exact, but it needs a resultant (Sylvester or Bezout matrix determinant with
  polynomial entries) that the kernel does not have yet.

## What would need a search (why this stops)

- **S and C contacts involving a spline.** A corner, moving on a known path, touching the offset
  spline is the system
  `P(δ) = C(t) + δ N(t)` (2 equations, 2 unknowns t, δ). A line or arc side reaching the offset
  spline is a tangency: the same plus `N(t) ∥ n_side`. Both eliminate to one-variable polynomials
  by resultants. That's exact in principle, but the degrees grow fast (a cubic piece gives
  degree ≈ 2·3·(2·3−1) before the radical is squared away), and the kernel has neither a
  resultant nor an exact bivariate solver.
- **Two offset splines touching (C), or one offset spline touching itself away from a cusp (S,
  the global self-intersection).** Points C1(s) + δN1(s) = C2(t) + δN2(t) with the two normals
  opposite: three equations in s, t, δ. The first δ is the medial axis's nearest point, the
  bisector problem. Without a resultant over two variables or a certified 2-D solver, the only
  way to the first δ is subdividing (s, t) boxes and bounding: a search. Per the rule, that
  isn't built.
- **The oracle.** sweep-ref (Lean) and sweep-diff cover lines and arcs. A spline outline would
  need its exact sections too: the offset curve's area by Green's theorem is
  ∫(x dy − y dx)/2 of C + δN, which is not closed-form for a general spline. So it would be a
  1-D integral, each value of which is exact to rounding, not a closed form.

## The smallest exact step, if wanted

Build these:
- splines that are refused unless no event of any kind can come within the depth;
- the procedural drafted wall;
- the per-span cusp bound δ* (exact, above);
- a conservative bound proving no contact. Contacts are excluded when every pair of features
  (spline span vs other side, span vs span) is farther apart than 2·reach plus the spans'
  control-hull extents. That uses the hulls' exact distance, so it's a proof, not a search.

Inside that, a taper is the plain draft, exact. Outside it, refuse with a new
`TaperError::SplineEvent` (not WallsCross, which claims a crossing at a known height). That
covers the common case, a gently curved outline tapered a little, without any search. Gates as
usual: bits identical (nothing built changes, since splines are refused today), scoreboard 0
worse, fuzz 150/150, native = wasm, and a spline fuzz against a numeric section integral.
