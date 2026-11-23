# The splines of .fg: one specification, in Lean, that the kernel matches

Design note, before building. Branch `claude/repo-review-mbdrlj-spline-spec`, from `claude/repo-review-mbdrlj` (736149e).

## What the Rust does today (read 2026-11-23)

The language (`interface/forge-lang/src/builtins.rs`, `spline(..)`), its sketch (`forge-sketch/src/sketch.rs`) and the
curve (`forge-geom/src/spline.rs`) give three forms. All end up as `Spline`: a chain of Bezier pieces of one degree in
homogeneous coordinates (x w, y w, z w, w), in f64.

| form | how it is written | what the kernel makes |
|---|---|---|
| through | `spline([..])`, `closed:`, `start_slope:`/`end_slope:`, a `tangent` tie at an end | `Spline::through`: a C2 cubic in Hermite form |
| control, open | `spline([..], cv: true)` or any of `degree:/knots:/weights:` | `Spline::from_nurbs`: Boehm insertion to Bezier pieces |
| control, closed | `spline([..], cv: true)` with no `closed: false` | wrapped poles `P[i mod n]`, n + p of them, knots 0..n+2p |
| (path segment) | `spline(through:, to:)` in a path | through, open, from the path's point |
| (STEP) | `B_SPLINE_CURVE_WITH_KNOTS`, rational or not | `from_nurbs` on the file's degree, knots, poles, weights |

What `Spline::through` does:
- parameters: centripetal. `t_i - t_(i-1) = |q_i - q_(i-1)|^(1/2)`, then all scaled by `L / sum |dq|^(1/2)`, L the
  polygon's length, so the parameter runs as long as the polygon. An end slope is per this parameter. A `tangent` tie is
  the unit tangent of the line or arc (magnitude 1 per parameter), turned to the way the chord goes.
- unknowns: the slope m_i at each point; pieces are Hermite cubics, control points `q_i, q_i + m_i h_i/3, q_(i+1) - m_(i+1) h_i/3, q_(i+1)`.
  Rows: `h_i m_(i-1) + 2(h_(i-1)+h_i) m_i + h_(i-1) m_(i+1) = 3(h_i dq_(i-1) + h_(i-1) dq_i)` (C2 at the middle points);
  natural end `2 m_0 + m_1 = 3 dq_0` (second derivative 0); slope end `m_0 = d`. Solved by Thomas; closed by
  Sherman-Morrison on the cyclic system.
- closed: when `n > 2` and the first and last points are within `tol::LINEAR` (1e-5). The sketch makes `closed: true` by
  appending the first point. **So closedness is a distance test, not the flag**: an open spline (`closed: false`, or with
  a slope) whose ends come within 1e-5 is made periodic and its slopes are dropped without a word.
- refuses (None, then a generic `BadValue`) when two consecutive points are within 1e-5.

`from_nurbs`: checks length `n + p + 1`, non-decreasing, `u_p < u_n`; unclamped ends are clamped by insertion (the curve
is over [u_p, u_n]); an inner knot of multiplicity p + 1 or more leaves a pole count that does not fit and is refused
(None). Weights must be positive. Evaluation is de Casteljau per piece in homogeneous form.

Defaults (the language, unchanged here): through-spline closed unless `closed: false` or an end slope is given; control
splines open when `degree/knots/weights` are given, else closed unless `closed: false`; degree 3, or n - 1 (n - 2 closed)
with fewer points; knots uniform clamped; weights 1.

## The spec (Lean: `research/spline-loops/lean/SplineSpec`, on that project's Mathlib pin v4.36.0-rc2)

Everything is generic over a linear ordered field K: ℝ for the theorems, ℚ for the executable model (`#eval`, the oracle).

Definitions
1. `KnotVec p n`: u_0..u_(n+p), non-decreasing, u_p < u_n, every knot strictly inside (u_p, u_n) of multiplicity ≤ p,
   the end knots ≤ p + 1. (Inner multiplicity p + 1 would make a discontinuous curve: refused, as now.)
2. `basis u i k t`: Cox–de Boor, `N_(i,0) = [u_i ≤ t < u_(i+1)]` with the last nonempty span closed at u_n, a 0/0 term 0.
3. `Nurbs p n d`: knots, poles in K^d, weights all > 0. `eval C t = (Σ N_(i,p)(t) w_i P_i) / (Σ N_(i,p)(t) w_i)` on [u_p, u_n].
4. `closedCV p Q`: the cv closed form, as the kernel builds it: poles Q_(i mod m), i < m + p, knots 0..m+2p (uniform,
   unclamped), curve on [p, m+p]. Defined as a `Nurbs`, so it has no meaning of its own.
5. Interpolation, `interp τ q e`, for strictly increasing τ_0 < .. < τ_m and points q_0..q_m:
   the cubic spline with knots (τ_0 ×4, τ_1, .., τ_(m-1), τ_m ×4) that passes q_i at τ_i, with end conditions
   `natural` (C'' = 0), `slope v` (C' = v), each end chosen separately, or `periodic` (q_m = q_0; C, C', C'' agree at
   τ_0 and τ_m, the knots repeating with period τ_m - τ_0). Stated as a `Nurbs` (one meaning for every form); the Hermite
   form the kernel builds is a theorem, not the definition.
6. `centripetal q`: τ_0 = 0, τ_i = τ_(i-1) + s |q_i - q_(i-1)|^(1/2), s = L / Σ |Δq|^(1/2). Real-valued (fourth roots),
   so the executable model takes τ as input and the oracle checks the kernel's τ against this definition (below).
7. `fgSpline`: the language's arguments to one of the above, with the defaults listed, the tie slope and the
   end-slope arguments. This is the one meaning of `spline(..)`.

Why centripetal (from the literature, not another kernel): Lee, "Choosing nodes in parametric curve interpolation",
CAD 21(6) 1989, introduced it and showed it follows sharp turns where uniform and chord-length overshoot; Floater,
"On the deviation of a parametric cubic spline interpolant from its data polygon", CAGD 25 (2008), bounds the
interpolant's distance from the polygon; Yuksel, Schaefer, Keyser, CAD 43(7) 2011, prove the centripetal Catmull–Rom has
no cusp or self-intersection within a segment, which uniform and chord-length do not have. It is invariant under
rigid motion and uniform scale (the scale factor s keeps parameter speed ~ length, so a slope is "per unit along it").
It is what the kernel already does, so it changes no answer.

Theorems (all over ℝ, no sorry, standard axioms only, `Axioms.lean` prints them)
- T1 `basis_nonneg`, `basis_sum_one` on [u_p, u_n] (partition of unity); `basis_support` (N_(i,p) = 0 off [u_i, u_(i+p+1))).
- T2 convex hull: with positive weights each point of the curve over a span is a convex combination of that span's p + 1
  poles; a box test on poles bounds the curve (what `CurveHull`, `bbox`, `derivative_bounds` use).
- T3 derivative: for a non-rational spline, `C' = Σ p (P_(i+1) - P_i)/(u_(i+p+1) - u_(i+1)) N_(i+1,p-1)`, a B-spline of
  degree p - 1 on the same knots trimmed; rational: the quotient rule on the homogeneous spline, so derivative bounds
  come from T2 applied to the homogeneous derivative and the least weight.
- T4 continuity: on each span a polynomial; at a knot of multiplicity k ≤ p, C^(p-k) (all derivatives up to p - k agree).
- T5 clamped ends: end multiplicity p + 1 gives C(u_p) = P_0 and C(u_n) = P_(n-1) exactly, and `closedCV` and
  `periodic` close: C(start) = C(end) with C^(p-1) (C^2 for the cubic) across the seam.
- T6 knot insertion (Boehm) leaves the curve unchanged; inserting to multiplicity p gives the Bezier pieces, so the
  kernel's piece form *is* the spec's curve (justifies `from_nurbs` and `insert_knot`).
- T7 existence and uniqueness of `interp` for every strictly increasing τ (so for consecutive points distinct): the
  Hermite slope system is strictly diagonally dominant (natural row 2 > 1, middle 2(h_l + h_r) > h_l + h_r, slope row 1 > 0,
  periodic the same with the wrap) hence non-singular (Levy–Desplanques, Mathlib's `det_ne_zero_of_sum_row_lt_diag`);
  the Hermite cubic built from its solution is a C2 cubic spline with those knots meeting the conditions, and any such
  spline's slopes solve the system: unique. Interpolation at the data points follows.
- T8 error bounds the differential test uses:
  - Varah: for a strictly diagonally dominant A with margin δ = min_i (|a_ii| - Σ_(j≠i) |a_ij|), ‖A⁻¹‖∞ ≤ 1/δ, so the
    kernel's slopes m̂ are within ‖A m̂ - b‖∞ / δ of the exact ones; the oracle computes the residual exactly in ℚ.
  - de Casteljau in a rounding model (each op |fl(x) - x| ≤ u|x|): |Ĉ(t) - C(t)| ≤ γ_(2p+1) max |P_i| per coordinate
    of the homogeneous point (each level is a convex combination), and a stated bound for the division by w.
  - centripetal parameters: each f64 τ̂_i lies within a stated ulp count of τ_i; checked exactly by comparing fourth
    powers of rationals.

## Matching the kernel

- Executable model (ℚ): Cox–de Boor evaluation, Boehm insertion, the Hermite system solved exactly (Gaussian elimination
  over ℚ), the periodic system.
- Oracle `lake exe splineoracle`, read by `kernel/forge-geom/tests/spline_diff.rs` under `FG_LEAN=1` (as parse_diff
  does), with a committed digest so CI checks it without Lean: random splines (degrees 1..5, random knots with
  multiplicities, weights, all forms, both end conditions, periodic) and real ones (the spline parts in examples/ and
  the test corpus, restated in my own numbers). Rust's f64 values are exact rationals; Lean decides `|rust - exact| ≤
  bound` exactly, for points, first derivatives, through-spline slopes and parameters.
- Rust changes, each reported: expected,
  - R1: a through-spline is periodic because the flag says so, not because its ends are within 1e-5. `closed: false`
    or a slope then always means open ends (a loop with a corner where the ends meet). **Changes the meaning of
    `spline(..)` for an open spline whose ends coincide**; I will scan the corpus for any, and ask before landing it.
  - R2: the refusal for consecutive points within LINEAR becomes a typed error at the `points` span
    (`SplinePointsRepeat { index }`), not a generic BadValue; the spec's precondition is distinctness, and LINEAR is the
    kernel's one tolerance for "same point".
  - R3: whatever the oracle finds past the bound.
  No change to the parametrisation, end conditions or defaults is expected.

## Importers (to agree with the import session through you)

- Source with control data (STEP, FreeCAD B-splines, Onshape BSpline, CATIA's exported B-splines): written as
  `spline(poles, degree:, knots:, weights:)` exactly. A periodic source needs no new form: its wrapped poles and full
  unclamped knot vector are an open control spline over [u_p, u_n], the same curve (T6), so no language change.
- Fit points only (CADFS skFitSpline, WHUCAD/CATIA SCP): we can only write our interpolant through those points. The
  importer emits a typed note `FitSplineReinterpolated { entity, source }` ("the source interpolates by its own rule;
  this is .fg's centripetal C2 cubic through the same points"), and an end derivative from the source is written as a
  slope in our parameter: its direction is the source's exactly, its magnitude means our per-parameter rate, stated in
  the same note. No tuning to the source's shape.
- The CADFS regression (1 match, 2 mesh fails, 1 refusal when written exactly) is the kernel's, not the spec's: under
  "nothing that works now may fail after", the per-entity opt-in stays until those faults are fixed, and I can take the
  repro of each if you want.

## A default I would question (not changing it)
Through-splines closed by default: every fit-spline source (CADFS, WHUCAD, Fusion, FreeCAD) is open by default, and the
import session's "open chain" bug came from it. Open by default, closed by `closed: true`, would be the usual rule.

## Status (built)

Lean, `research/spline-loops/lean` (`lake build`; `lake env lean Axioms.lean`, `CheckAxioms.lean`: 369 theorems,
standard axioms only, no sorry):

| file | what |
|---|---|
| `SplineSpec/Defs.lean` | the definitions, core Lean, generic over the number type (the spec over ℝ, the model over `Rat`) |
| `Slopes.lean` | the slope system: row dominance, `slopes_exist`, `slopes_unique`, `slopes_bound` (Varah, row-scaled) |
| `Interp.lean` | T7: `interp_exists_unique` (every C2 cubic interpolant is the Hermite form of a solution), `interp_through` |
| `Centripetal.lean` | the parameters over ℝ, `centripetal_strictMono`, `spline_exists_unique` (the language's through-spline) |
| `Basis.lean` | T1, T2, T5: `nb_sum_one`, `nb_nonneg`, `eval_between` (convex hull), `clamped_start`, `clamped_end` |
| `Deriv.lean` | T3: `hasDerivAt_nb`, `hasDerivAt_curve` (the derivative is the B-spline of degree p - 1 on differenced poles) |
| `Continuity.lean` | T4: `nb_continuous`, `iter_curveOn`, `curve_smooth` (C^(p-k) at a knot of multiplicity k) |
| `Cert.lean` | the control-spline certificate: `span_bernstein`, `bern_close` |
| `Interval.lean` | the oracle's intervals enclose; `rows_in`, `residual_in`, `through_cert` |
| `Oracle.lean` | `lake exe splineoracle` |

Not yet: T6 (Boehm insertion leaves the curve unchanged) as a theorem. The kernel's insertion is checked
case by case instead: the oracle compares its Bezier pieces with the spec's curve computed by Cox-de Boor,
not with an insertion of its own. The rational derivative is the quotient rule on `hasDerivAt_curve`
applied to the homogeneous coordinates and the weight.

Differential test, `kernel/forge-geom/tests/spline_diff.rs`: 698 curves, all within the curve's scale
over 2^30 of the spec. The largest bound Lean proves is 6.2e-12. A planted 1e-6 error in the natural end
row is caught.

Rust changes so far: `Spline::homogeneous` (an accessor for the test). No disagreement past the bound,
so no numeric change. R1 and R2 wait on the master's answer.
