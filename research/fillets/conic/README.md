# Conic fillets

`fillet(EDGES, r, conic: ratio(RHO))`, 0 < RHO < 1. Started 2026-11-23.

## The definition

The conic fillet uses the rolling ball's construction. Its springs, spine, trims, run-outs, chain joints and mitres
are the circular fillet's own. Only the blend face's section differs:

- At each point of the spine (ball centre c, radius r, feet `fa` and `fb`), the section is the rational quadratic
  Bezier (`fa`, A, `fb`) with middle weight w = RHO / (1 - RHO).
- A is the apex, where the faces' tangent planes at the feet meet in the ball's section plane:
  A = c + (ua + ub) / (1 + cos phi), with ua and ub the vectors from c to the feet and phi the angle between them.
- RHO is the shoulder ratio. M is the chord's middle and S the section's point on the line from M to A; then
  |M S| = RHO |M A|.
- At w = cos(phi / 2) the section is the ball's arc. So a constant RHO is a circle only at one turn: at a right
  angle, RHO = 0.4142.
- RHO 0.5 is a parabola, below it an ellipse, above it a hyperbola.
- This is the fillet-spec session's statement (`Section.conic w`, `conic_circle`): feet = springs, A = the
  three-plane point, no conic sphere corner.

## Onshape's numbers, measured

`onshape_sections.py` reads the source STEP of the CADFS conic parts. It measures the setbacks and RHO of each
blend face's middle section, with exact end tangents.

| part | radius | rho | turn | setbacks | r tan(turn/2) | rho measured |
|---|---|---|---|---|---|---|
| 00680840 | 38.1 | 0.5 | 90 | 38.10000 | 38.10000 | 0.5000 |
| 00838464 | 5 | 0.9 | 36.870 | 1.66667 | 1.66667 | 0.9000 |
| 00838464 | 5 | 0.9 | 66.422 | 3.27327 | 3.27327 | 0.9000 |
| 00961869 | 10 | 0.8 | 90 | 10.00000 | 10 | 0.8000 |

So Onshape's `radius` is the setback's measure: the circular fillet's feet for that radius. Its `rho` is the
shoulder ratio.

Onshape writes these blends as non-rational cubic B-splines, which is a fit. Only RHO 0.5, a parabola, is exact
in its file. Our definition is our own and is not tuned to that fit.

Measured against the STEP: 00680840 (RHO 0.5) matches to 4.1e-8 in volume and 6.5e-8 in area. That deviation is
the size of the file's other fits, not the section's.

## How it is checked

- **Closed forms.** `kernel/forge-model/tests/fillet_conic.rs`.
  - Fill per length of a straight edge: T (1 - g(w)), with T the triangle (foot, apex, foot).
  - g(w) = w (acos w - w s) / s^3 with s = sqrt(1 - w^2), below 1.
  - g(w) = 2/3 at 1.
  - g(w) = w (w s - acosh w) / s^3 with s = sqrt(w^2 - 1), above 1.
  - g is checked against the segment's area integrated off the curve, and for continuity through w = 1 from both
    sides: g = 2/3 + 4/15 (w - 1) - 6/35 (w - 1)^2 + ...
  - Convex, concave and 60-degree edges agree to about 1e-11.
  - Rings by Pappus (Green's theorem over the exact section) agree to about 1e-11.
  - A mitred rim agrees to every digit (23649.498758537102): the four prisms, less what two take at each corner,
    which is the integral of a(z)^2.
- **OpenCascade.** OCCT's fillet makes circular sections only. `occt_conic.py` instead builds each
  `examples/conic` part from exact pieces: the section as a rational Bezier, swept or turned, then a boolean.
  - Volume and area agree to 3e-9 or better on boss, edge, slot, step and wedge.
  - On the mitred rim OCCT's own integration is the outlier. Its volume moves by 1e-5 between its default and
    adaptive rules, while ours equals the closed form.
  - Our STEP export reads valid in OCCT, with volume within its cubic fit (about 5e-7).
- **Native and wasm** give the same bits on the mitred rims at RHO 0.3, 0.5, 0.7 and 0.9.

## Exact where

| case | how |
|---|---|
| straight edge (plane-plane, or plane-cylinder parallel) | `Extruded` over the rational quadratic |
| circular edge between coaxial surfaces | `Spun` of the section |
| tangent chain joint | a shared section edge (`SplineCurve`) |
| mitre beside a sharp edge | the plane between two mirror-image sweeps (`spline_cut`) |
| run-out into an end face | the circular fillet's own |

## Refused, typed, at the place

- `ConicVertex`: a vertex that only a ball closes (three blends at a corner, a pivot, a ball rolled on over the
  next face), or a setback.
- `ConicEdge`, for any of:
  - an edge that is neither a line nor a circle (phase 2: `ConicPipe`, built and held back until this merges);
  - a varying radius;
  - a ball resting on a rim, beyond a smooth edge, or across a face.
- `BadConic`: RHO outside (0, 1). The language reports it as `BadValue`.

## The importer

`tools/find-parts/recreate/cadfs.py` reads `crossSection: CONIC` with `rho`. The census sample is
`sorted(unique_models.json)[::200]`, 1914 parts, at 1e-4 (`research/import/tally.py`).

```
           before  after
match        1106   1107     00680840: untranslated -> match (vol 4.1e-8, area 6.5e-8)
refused        88     89     00092585: untranslated -> refused (ConicEdge; its circular form refuses too)
untranslated  627    625
fillet parts: match 164 -> 165
```

No other part changes class or volume. The coverage census's 9 "variable or conic" parts are 6 conic and 3
variable-radius (`isVariable`). Of the 6 conic, 4 stop on other gaps first: sketch text, an unequal chamfer,
a split piece, `BLEND_EDGE`.

`repro/` holds the two circular refusals behind 00092585 and 00961869, in our own numbers:

- `spring_touches_hole.fg`: a spring tangent to a hole's rim. It builds INVALID, or is refused when the plate is
  as thick as the radius.
- `counterbore_wall.fg`: a 10 round whose spring a counterbore's head reaches past.
