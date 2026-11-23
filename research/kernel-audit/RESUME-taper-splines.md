# RESUME: tapered extrudes of spline outlines

## Built: the smallest exact step

A tapered extrude of an outline with a spline in it is drafted when a finite proof shows no event
can come within the depth. Otherwise it is refused with `TaperError::SplineEvent` (in
forge-model, `DraftError::SplineEvent`). Nothing is searched for. The proof is
`forge_ops::taper_spline::clear`:

- **Smooth joints.** Every joint of a loop with a spline is tangent (G1), so each point moves
  exactly δ along its normal and each section is the outline's parallel curve.
- **No curling.** δκ < 1 on the side the outline moves to, over the whole reach. For each
  non-rational Bezier span, κ's largest value is at the span's ends or at the roots of
  `2 num' den − 3 num den'` (num = x'y'' − y'x'', den = x'² + y'²), from
  `forge_math::poly::real_roots`. For an arc: its radius beyond the reach.
- **No two parts meeting.** Parts are Bezier spans, lines, and arcs cut under a quarter turn.
  - Two parts joined by a run of the loop whose tangents all lie in one open half-plane
    cannot meet. The tangents are the hodograph's control vectors, a line's direction, or a
    short arc's end tangents. Such a run is a graph over that half-plane's direction, and so
    is its parallel curve.
  - Any other two parts must sweep disjoint boxes: their control hulls grown by the reach (by
    the reach times the mitre's speed in a loop of lines).
- **Loops of lines and arcs alone.** `first_event` exactly, with no event within the reach.

The wall is `forge_geom::spline::Drafted`: S(u, v) = F(C(u) + (v − v0)·k·N(u) + v·z), exact.
- **Its value and foot.** Each section is a parallel curve of C, so a point's foot on C is its
  foot on the section. The implicit value is the distance to C less the section's offset, scaled
  to the normal.
- **Its derivatives** through the second, from C''' by way of N'' (checked against
  differences).
- **The cap edges.** These are `Section`: the parallel curve at that height, an exact procedural
  curve, drawn straight in the wall's parameters (`curve_uv`). `rebuild::edge_curve` takes it
  wherever a plane across the sweep cuts such a wall. The traced intersection it replaces left
  `mass` 1e-8 off.
- **Vertices.** A vertex of such a wall starts where its ruling now is, at its height, and a seam
  vertex is held to the plane through the ruling (`rebuild::ruled_start`, `ruling`).
- **No carry past the ends.** Unlike an extruded wall, the curve is not carried past its ends: a
  point moved in along the end ruling has its foot at the end, while a carried tail curls away
  from the neighbouring face the section goes on to.

`draft()` takes an extruded spline wall too (`draft::drafted`, `Surface::Procedural` of an
`Extruded` along the pull). That draft refused it before.

**Measured.** 800 random smooth closed splines (5–10 points, 40% with a round hole), tapered
1–40° in and 1–30° out, 1–16 deep: 668 built and every one exact against its sections
(Steiner: the parallel curve's area by Gauss–Legendre, ∫ over height by Simpson, exact in a
quadratic), wall areas to 3e-15; 132 refused (SplineEvent), mostly holes whose box comes
within the reach of the outline. Nothing that built before changed (repo parts, bench bits,
the line/arc fuzzes, identical).

## What is left

- **Splines with corners (non-tangent joints).** The joint is where C(t) + δN(t) meets the moved
  neighbour (a line or circle). That is one equation in (t, δ), polynomial once |C'| is squared
  out. Following it, and its E event (a neighbour's side running out), needs a **resultant**:
  a Sylvester or Bezout determinant with polynomial entries, eliminating t to leave one
  polynomial in δ. The kernel has none. Until then: refused.
- **Contacts the box proof cannot rule out:** a corner, or a side, reaching an offset spline (2
  unknowns t, δ); two offset splines touching, or one touching itself away from its cusps (3
  unknowns s, t, δ: the bisector, the medial axis's nearest point). Exactly, these need a
  **2–3-variable polynomial system solver** (resultants over two variables, or a certified
  subdivision with a uniqueness test). Without one, the first δ comes only from searching, which
  isn't built. Until then such outlines are refused whenever their boxes come within the reach,
  even where no contact happens.
- **Rational splines** (ellipses among them): the same curvature bound with the weights cleared,
  a higher-degree polynomial. Refused for now.
- **Outlines with events past the proof:** where the outline does curl, close, or split within
  the depth, the slabs of the line/arc taper would need the events above, the curl as a ridge
  of the parallel curve, and the slab walls as `Drafted` between event heights.
- **The oracle:** sweep-ref/sweep-diff cover lines and arcs. A spline's sections have no closed
  form (the fuzz uses one-dimensional quadrature, each value exact to rounding).
