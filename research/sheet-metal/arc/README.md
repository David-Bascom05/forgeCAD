# Circles wrapped round a bend: `wrap::Flat::Arc`

A hole or rounded cut that crosses a sheet metal bend is a circle drawn in the flat (s, h) and wrapped
round the bend's cylinder. `forge_geom::wrap::Flat::Arc` carries it exactly, as lines and parabolas are
carried for wrapped text.

## What was derived (`check_arc.py`, output `check_arc.txt`)

1. In (angle, height) a wrapped circle of radius a is an ellipse with semi-axes a / unroll and a.
2. Its wall between two radii has radial rulings square to the axis: a right conoid. Area element
   |S_u x S_rho| = sqrt(rho^2 phi'^2 + h'^2).
3. `Flat::meet_poly` for an arc, `(X - cx)^2 + (Y - cy)^2 - r^2` along quadratic X(t), Y(t), summed as
   the code sums it: a quartic at most, a quadratic along a line. An arc against a line or parabola puts
   the polynomial curve into the circle's implicit and reads the arc's fraction off the point (atan2),
   so no trigonometric equation is solved.
4. Two circles meet at x = (d^2 + ra^2 - rb^2) / 2d along the centres, y = +-sqrt(ra^2 - x^2) across.
5. Reference crossings (arc with a line, a parabola, an arc; levels in s and h) to 20 digits, solved
   exactly, and a disc wrapped between two radii: its volume in closed form and its area by 40-digit
   quadrature of 2.

## Where the code uses it

- `kernel/forge-geom/src/wrap.rs`: `Flat::Arc` (`at`, `bbox`, `meet_poly`, `param`, `level`, `reach`).
- `kernel/forge-geom/src/ruled/meet.rs`: `crossings` (wrap_wall, meet_wall), `line_wall`.
- Tests: `kernel/forge-geom/tests/wrap_arc.rs` (items 1, 2, 5 crossings), `kernel/forge-mesh/tests/wrap_arc_solid.rs`
  (item 5 volume and area).

## Rerun

```
fbuild <python with sympy and mpmath> research/sheet-metal/arc/check_arc.py > research/sheet-metal/arc/check_arc.txt
```

It asserts items 1 to 4 and prints the values the tests hold. The prototype's broader study (branch
sheet-proto, f56a93c4) is summarised in ~/projects/forgecad-gate/sheet-metal/prototype.md.
