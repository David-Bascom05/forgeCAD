# Proving a corner patch never folds

Status: research notes (fold-research branch). In order: the prior art (stage 0), the analysis
and oracle measurements (stage 1), and the Taylor-model prototype with its measurements
(stage 2). The prototype lives on `refs/experiments/corner-fold-taylor-models`. It is not wired
into builds: it misses the budget by two to three orders of magnitude.

## Prior art (stage 0)

What each line of work guarantees, what it costs, and whether it carries over to a rational,
transfinite blend of exact (transcendental) ribbons.

- **Johnen, Remacle, Geuzaine, "Geometrical validity of curvilinear finite elements" (JCP 2013);
  Johnen, Weill, Remacle, "Robust and efficient validation of the linear hexahedral element"
  (2017, arXiv:1706.01613).** For a polynomial element map, det J is itself a polynomial of
  known degree. Its Bezier coefficients are computed exactly from a few sampled determinants,
  bound it from below, and converge under adaptive subdivision. The check is sound, needs no
  remainder, and is extremely fast (about 6 million linear hexes per second on one core).
  *Applies only to polynomial or rational maps.* Our blend is rational of degree ~2(n - 1) in
  the domain and transcendental through the rails (sin/cos, normalisation), so there are no
  finite Bezier coefficients. This is the method to aim for if the patch itself becomes
  rational (below).
- **Garloff, Hamadneh and others: Bernstein expansion of rational functions** (range bounds
  over boxes and simplices, positivity certificates, convergence under degree elevation and
  subdivision). The rational version of the above: sound and cheap *for rational functions of
  modest degree*. Same limitation.
- **Makino and Berz: Taylor models** (polynomial plus rigorous remainder, sharpness of order
  n + 1 in the box width; COSY INFINITY). This is what applies to arbitrary code lists like
  ours, and what stage 2 built. They publish no cost figures. Measured here: one model of our
  patch with first derivatives costs 0.2-0.8 ms, and the remainder decides the box size.
- **Merlet (and Merlet and Donelan): interval analysis for singularity-free workspaces of
  parallel robots.** Symbolic det J, then interval bounds with bisection. Sound, but
  overestimation from repeated variables is the known weakness (and regularity of an
  interval matrix is NP-hard). This is our first experiment (a063386), which hit the same wall.
- **Poranne and Lipman, "Provably good planar mappings" (SIGGRAPH 2014).** Injectivity
  everywhere from constraints at collocation points, plus a modulus of continuity of the
  basis: a fill-distance rule. This is the sampled-plus-Lipschitz scheme of stage 1. Its cost
  is the oracle cell counts measured there.
- **Sederberg and Meyers (CAGD 1988); Christiansen, Katz, Sederberg: normal and tangent
  cones.** Cone bounds of Bezier patches' tangents and normals (from control points). They
  need a hull property, so the same limitation as Bezier methods.
- **Floater and Kosinka (2010); Floater and Muntingh (2024): injectivity of barycentric maps.**
  Wachspress maps between convex polygons are always injective. Mean-value maps can fold
  from five vertices on. Planar maps from vertex data only, so not usable for a G1 surface
  with prescribed cross-boundary normals. They do show that fold-freeness by construction
  exists only in narrow, structured cases.
- **Transfinite and Coons literature** (Gordon and Thiel 1982; TFI grid generation): validity
  is checked by det J > 0, with no general sufficient condition. Coons volumes in Bezier form
  (arXiv 2025) again reduce it to Bezier coefficients of det J.
- **Multi-sided and vertex-blend patches: Varady and Rockwood (setback vertex blending, CAD
  1997); Varady, Salvi, Rockwood (curved-ribbon transfinite patches, EG 2013); Varady, Salvi,
  Kariko (multi-sided Bezier, CGF 2016); Vaitkus et al. (GBS patches, CAGD 2021); Sipos et al.
  (I-patches); Zhou and Qian, "Polyhedral vertex blending with setbacks using rational
  S-patches" (CAGD 2010).** None of the abstracts claims a fold-free guarantee. The rational
  S-patch work is notable: an *exact* setback vertex blend as a single rational patch, for
  conic boundaries.

**What this changes.** Every fast and sound validity method in the literature works on
polynomial or rational maps, where det J has finite Bernstein coefficients and needs no
remainder. Every method for general code (intervals, Taylor models, Lipschitz sampling) pays for
overestimation, and stage 1 and stage 2 measured that cost here. So the approach that beats the
plan is not a better enclosure of today's patch. It is **a rational patch** for the input class
where the rails are rational: lines and conics, on planes, spheres and cylinders, which is every
corner of a planar-face model with constant fillets. For that patch,
n . (P_u x P_v) times a positive denominator is a polynomial, and the Johnen / Garloff check
proves it in microseconds. Rails that are not rational (Intersection, Procedural) keep today's
patch and sampled check. That is a change to the construction, so it is David's call (see the
end).

## The patch and the claim

`HolePatch` (crates/forge-geom/src/corner.rs) maps a convex cyclic n-gon D (n = 6, 8 or 10 for the
fillet corners of the survey, up to 12) into space:

```
P(x)  = sum_i B_i(x) R_i(s_i(x), h_i(x))
d_i   = affine distance to domain side i            (zero on side i)
s_i   = d_(i-1) / (d_(i-1) + d_(i+1))               (position along side i)
h_i   = d_i / (1 + d_i / H_i)                       (damped distance)
R_i   = C_i(s) + h T_i(s) + h^2/2 K_i(s)            (ribbon: exact rail, cross tangent, bend)
B_i   = d_i^-2 / sum_j d_j^-2
```

The claim: J(x) = P_u x P_v is never zero on the closed domain. J is continuous (P is C1, see
below) and D is connected, so then J keeps one orientation, the one G1 gives it along every
side (`turns_in`): the patch cannot fold or flip. Rounding is a separate, small step (a margin
on the computed values) and is left out below.

## Facts about the map that any proof must use

1. Off the corners P is real-analytic on the closed domain. The only places where its formula
   is singular are the zeros of d_(i-1) + d_(i+1) (outside D for n >= 4; at the far corner for
   n = 3, where B_i vanishes to second order), d_i = -H_i (outside D), and the corners, where
   two d's vanish together. The weights are smooth up to the sides: B_j = O(d_i^2) next to
   side i.
2. At a corner, in polar coordinates (rho, theta) about it, d_a = rho a(theta) and
   d_b = rho b(theta), and every weight is a smooth function of (rho, theta):
   `B_a = b^2 / (a^2 + b^2 + a^2 b^2 rho^2 sum_far d_j^-2)` and so on. The ribbons are smooth in x
   there. So P(rho, theta) is smooth on the closed sector rectangle [0, rho0] x [theta0, theta1],
   P(0, theta) is the corner point, and P_theta = rho G(rho, theta) with G smooth. The
   Cartesian Jacobian is `J = P_rho x G`. It is smooth in (rho, theta), and constant at
   rho = 0 (the ribbons agree to first order there, `ends`), so J is continuous at the corner.
   In Cartesian coordinates, grad J has a jump at the corner and the Hessian of J grows like
   1 / rho. **Every bound that tiles the domain in x and Taylor-expands in x pays for this
   1 / rho.** The mean-value experiment ran into exactly this.
3. Write P = R_k + sum_(i != k) B_i D_i with D_i = R_i - R_k. Then
   `P' = R_k' + sum B_i' D_i + sum B_i D_i'`. Next to side k, B_i' = O(d_k) and D_i is bounded,
   so P' is within O(d_k) of ribbon k's Jacobian. Ribbon k's own Jacobian is, at fixed s, a
   polynomial in h (degree 3 for R_s x R_h). The only non-polynomial dependence is through the
   rail jet at s: **a 1-D problem**.

## Measurements (oracle cell counts)

`crates/forge-ops/examples/fold_measure.rs` (on the experiment ref) takes the survey's
generator (the same cone / flipped corners as the 1200-corner survey: n = 3..5 edges, 6..10
hole sides, saddles and nearly flat edges) and counts the cells a tiling proof of
g = n_c . J > 0 needs (n_c is the unit normal at the cell centre). Cells split in four until
they pass. The derivative bounds are **sampled oracles**: the largest value over a 5 x 5
sample of the cell, by exact jets and central differences. A sound enclosure is at least that
large. Real interval or closed-form bounds come out 3-10x larger (5-10x in the mean-value
experiment), so k below is that over-estimate.

- order 1: pass when g(c) > k sup|grad g| r
- order 2: pass when g(c) + min over the cell of grad g(c).(y - c) > k/2 sup|Hess g| r^2
- polar: a sector of radius half the distance to the far sides at each corner, done in
  (rho, theta). The rest is done in Cartesian cells.

60 patches (30 cases x 2 orientations), budget 4000 cells per patch:

```
                      k = 1                 k = 3                 k = 10
                  median  p90  fail    median  p90  fail    median  p90  fail
order 1, cart        86   291    0       424  1296    0      1284  2696   24
order 1, polar      131   335    2       550  1466    2      1388  2858   30
order 2, cart*      169   451    2       371   935    2       926  2095    2
order 2, polar      160   427    2       339   936    2       852  2207    2
```
(*) Cartesian order 2 only passes near the corners because the samples never come close
enough to see the 1 / rho Hessian. It is not a sound design.

The same with the scale-free g = n_c . J / (|P_u| |P_v|) (the sine of the parameter angle
times the cosine of the normal's turn; its gradient is exact from the second-order jet), which
removes the parametrisation squeeze next to short sides:

```
                      k = 1                 k = 3                 k = 10
                  median  p90  fail    median  p90  fail    median  p90  fail
order 1, cart         5    79    0        60   318    0       294  1487    2
order 1, polar       15   112    2       105   398    2       602  1919    2
order 2, polar       56   172    2       133   372    2       300  1020    2
```

Normalising is worth 2-7x at the median and much less at the p90. The 2 polar failures are
case 2 (10 sides, both orientations; |J| min / max = 0.0014). They run out of the
4000-cell budget. They are not folds: Cartesian order 1 proves the patch in 517 cells.

Where the passing cells lie, by min_i d_i / H (k = 3, order 2, polar):

```
< 0.02   < 0.05   < 0.1   < 0.2   rest
  47%      24%     15%      9%     5%     (of 23 300 cells over 60 patches, corner-sector cells included)
```

Costs on this machine, release: one `deriv` (value + first + second derivatives) takes
0.8 / 1.0 / 1.25 us at 6 / 8 / 10 sides. `HolePatch::new` takes 0.28 / 0.32 / 0.54 ms (the 16x16
scan is most of it). One sound cell enclosure (third-order interval jet of P with interval rail
bounds) costs, from the two experiments, 15-40 deriv-equivalents.

Per-patch smallest |J| over a 64 x 64 grid, relative to the largest: 0.2-0.7 at 6 sides,
0.05-0.17 at 8 sides, 0.001-0.05 at 10 sides. The small values sit next to the short sides,
the sections of nearly flat edges.

## What this says

- **Even with perfect derivative bounds**, a tiling proof needs a median of 90-160 cells and a
  p90 of 300-450. At realistic k = 3-10, that is 350-1300 cells. At 15-40 us a cell, that comes
  to 5-50 ms per patch: **10-100x over budget**. This matches both experiments, and it is not
  an artefact of loose bounds. The patches are genuinely thin (small J) and fast-varying next
  to short sides.
- 71% of the cells are within 5% of the height of a side, or in a corner sector. A boundary
  strip proof from the ribbon (fact 3) looked like the way to remove them, but it is too
  weak: see the strip measurement below. Even if the strips and corners were free, the
  interior (d / H > 0.05) still needs 85 / 112 / 244 cells at k = 1 / 3 / 10, which is
  1.5-10 ms.
- The map is analytic off the corners (fact 1). A Lipschitz or
  second-order test there spends its cells on the ratio sup|J''| / min J, not on any
  singularity. **A high-order polynomial enclosure (Taylor model or Bernstein form) of J over
  a few big boxes, then Bernstein subdivision of the polynomial alone (flops only, no patch
  evaluations), removes that ratio from the cost.** This is the standard trick that makes
  curved-element validity checks fast (see the survey). The cost moves to building the
  polynomial enclosure once per box.

How far the strip lemma below reaches, measured exactly (no bound slack: the true
|P' - R_i'| against the ribbon's margin, 120 patches, every side, away from the corners).
A ratio under 1 proves the strip:

```
depth d/H                 0.01   0.02   0.05   0.1    0.2
ratio median              0.11   0.25   0.55   0.88   1.35
ratio p90                 0.32   0.71   1.38   2.09   3.47
sides with ratio < 1       98%    95%    84%    58%    35%
```

The ratio grows linearly in d (E = O(d), as it must). The ribbons disagree by about the
patch's own size in the middle (|D_j| ~ |P'| H), so with a realistic 3x bound slack a
first-order strip reaches only ~1-2% of H. **The perturbation-from-the-ribbon idea is too
weak to carry the strips.** A second-order boundary expansion
J = J0(s) + d J1(s) + d^2 Rem, with J1 exact from 1-D data, would reach further, but its
remainder needs the same third-order bounds as the interior.

Polar corner expansion, measured with oracle bounds. Over each corner sector (radius half the
distance to the far sides),
`n0 . J(rho, theta) >= J0 + rho min_theta J1(theta) - rho^2/2 sup |d^2/drho^2 (n0 . J)|`.
J0 and J1 are from the corner (a one-sided difference here, the exact corner jets in a real
proof). The second rho-derivative is sampled. This reaches 0.50 / 0.95 / 1.00 of the sector
radius at p10 / median / p90, and the whole sector at 47% of corners (60 patches, 480
corners). **Corners are the tractable part.** The remainder only needs a bound on one second
derivative along rays, but it is still a bound over a (rho, theta) box.

## Design candidates, ranked

1. **Taylor models, the whole domain.** P is analytic off the corners (fact 1). Its nearest
   complex singularities, from the zeros of d_a^2 + d_b^2, sit at a distance proportional to
   the distance to the nearest corner. So the domain is covered by a few Cartesian boxes
   away from the corners, plus one or two polar boxes per corner sector (analytic in
   (rho, theta), fact 2). On each box: a Taylor model of degree 3-5 of (P_u, P_v), i.e. a
   polynomial plus a rigorous remainder interval, built by Taylor-model arithmetic over the
   patch formula with Taylor-model rail jets. Then n_box . (P_u x P_v) is a polynomial plus
   remainder, and its positivity is decided by Bernstein coefficients with subdivision of
   the polynomial alone (flops only). Cost: a Taylor-model product on this machine (naive,
   bivariate on [-1, 1]^2, no outward rounding yet) takes 157 / 337 / 970 ns at degree
   3 / 4 / 6. Optimised index tables would make it maybe 2-3x faster, and outward rounding
   would add back about as much. Carrying (P, P_u, P_v) means about 3 Taylor-model products
   per product of a plain point evaluation, which is on the order of 400 operations for 10
   sides. That gives **~0.1-0.4 ms per box**, and (n + 8) to (2n + 16) boxes, so
   **2-14 ms per patch, 4-25x the build.** It needs Taylor-model arithmetic in forge-math (no
   crates): a few hundred lines (mul, truncation, remainder, inverse, sqrt, sin/cos with
   Lagrange remainders).
2. **Hybrid.** Corner sectors by the polar expansion
   J(rho, theta) = J0 + rho J1(theta) + rho^2 Rem, with J0 and J1 exact from corner jets;
   Taylor models only for the rest. This saves the polar Taylor-model boxes if the expansion
   reaches far enough.
3. **Change the construction so the strips are exactly ribbons.** Weights with B_i = 1 for
   d_i < delta (a smooth step instead of d^-2). A strip is then ribbon i and nothing else, and
   its regularity is the exact 1-D polynomial check. This changes the surface and only
   removes the strips, so it is not an answer by itself.
4. **Cache the certificate**, keyed by a hash of the sides. Unchanged corners cost nothing on
   rebuild. It does not change the cold cost.

Ruled out: global sufficient conditions in O(sides) inequalities. The blend term
sum B_i' D_i is not small in the interior (the ribbons disagree there by 10-50% of the patch
size), so no condition on the ribbons alone implies regularity there without looking at the
interior. Convex combinations of regular maps are not regular in general (determinants are
not concave). The positive-definite-symmetric-part trick makes the convex part sum B_i R_i'
safe, but only if all ribbon Jacobians lie in one cone `{M : sym(W M) > 0}`. Even then it
leaves the D term. Fold-free by construction would need a planar map with prescribed boundary
curve and prescribed first-order cross data that is injective by construction. I know of no
such scheme with exact data (harmonic maps take no derivative data).

## The strip lemma (for stage 2a)

On S_i = {x in D : d_i <= delta, d_j >= eta_j for j != i}, write P' = A + E with A = R_i' and
E = sum_(j != i) (grad B_j (x) D_j + B_j D_j'), D_j = R_j - R_i.

- Weights: w_j = (d_i / d_j)^2, B_j = w_j / (1 + sum_k w_k) <= w_j, and
  `|grad B_j| <= |grad w_j| + w_j sum_k |grad w_k|` with `|grad w_j| <= 2 d_i / d_j^2 + 2 d_i^2 / d_j^3`.
  So E = O(d_i) with constants from eta_j and from interval bounds of R_j, R_j' (zeroth and
  first order only, which are tight).
- Ribbon: `A_u x A_v = (R_s x R_h) det[grad s_i, grad h_i]`. The determinant is explicit and
  rational in x, and
  `R_s x R_h = c0 + h c1 + h^2 c2 + h^3 c3` with c0 = C' x T, c1 = C' x K + T' x T,
  c2 = T' x K + K' x T / 2, c3 = K' x K / 2 (' = d/ds). These are 1-D functions of s, bounded
  on s-intervals by rail enclosures.
- Regular: with e >= |E| on the cell,
  `|P_u x P_v - A_u x A_v| <= e (|A_u| + |A_v|) + e^2`, so n.(P_u x P_v) > 0 follows from a
  1-D inequality in s for each delta.

## Rail bounds that are rigorous

The hybrid needs, over parameter intervals, enclosures of C, C', C'', C''' (and C'''' for T'''
in the interior remainder) and of n, n', n''.

- Line, circle, ellipse, parabola, hyperbola: closed forms (as in `derivative_box`), with
  interval sin / cos already in forge-math.
- Normals on plane, sphere, cylinder: linear in the point. Cone and torus: w / |w| of a
  linear w, enclosed by interval arithmetic on the closed form.
- **Intersection curves** (IntCurve: two surfaces, a Hermite guide chain and a Newton step).
  The curve is the solution set of F(p) = (f_a(p), f_b(p)) = 0. Over a parameter interval, take
  the guide segment's box widened by the stored `sag`, plus a margin. A Krawczyk or interval
  Newton test on F, restricted to the normal plane of the guide tangent, proves that box holds
  exactly one curve point per plane. Derivatives come from the implicit function theorem,
  `C' ~ grad f_a x grad f_b`, and its derivatives by differentiating that identity, all
  evaluated in interval arithmetic over the proven box. The curve's parameter is the
  chain-guided one, not arc length, so the parametrisation must be fixed exactly as `deriv`
  evaluates it (Newton foot from the Hermite point). The cleanest way is to define the bounds
  for that map: Hermite point (polynomial) composed with the Newton projection, whose
  derivatives follow from the same IFT.
- **Procedural curves.** `SurfaceBezier` (the trim on a non-planar face): S(uv(t)) with uv
  quadratic. The bounds come from surface derivative enclosures over the uv box (closed form
  for the analytic surfaces) by the chain rule. Other procedural curves (Helix, Track, Cut,
  SplineRail) each need their own enclosure method. Until one exists, a rail without
  rigorous bounds makes the certificate refuse, and the build falls back to today's sampled
  check (no weaker than now).
- **Surface normals of procedural neighbours** (n'' today is a finite difference): needs
  procedural surfaces to provide jet enclosures. The same rule applies: no enclosure means
  no proof, and the fallback is today's sampled check.

## Recommendation for stage 2

Prototype the hybrid in this order, measuring after each step:
(a) the strip proof (ribbon polynomial in h, 1-D rail bounds in s, the O(d) perturbation);
(b) the polar corner expansion;
(c) the interior by Taylor models with Bernstein subdivision.
Steps (a) and (b) alone show how much is left for (c). If (c) cannot reach about 0.5 ms, the
certificate does not ship on every build. The fallback options are the cache, or running the
proof only where the sampled scan's margin is small.

## Stage 2: the Taylor-model prototype

Code: `crates/forge-geom/src/corner/proof.rs` on `refs/experiments/corner-fold-taylor-models` (7143102)
(not on the branch). What it does:

- **Taylor models.** A polynomial of total degree K = 3 in two local variables (a, b), plus a
  remainder graded by powers of |a|: `|e| <= sum_k r_k |a|^k`. The grading lets a model of
  something that vanishes at a = 0 be divided by a exactly, which the corners need.
  Operations: add, multiply (products of degree > K fold into the graded remainder),
  1/x, x^(-1/2), cos and sin (by series about the constant term with a Lagrange remainder),
  each with its rounding error folded into the remainder.
  Forward-mode jets (a model of the value and of its two derivatives) give P_a and P_b.
- **Rails as models.** Lines, circles, ellipses and parabolas on planes, spheres and cylinders
  (`Rail::model`), and the split's spokes (normal w / |w|). Anything else returns None and
  the proof reports *unsupported*, which falls back to today's sampled check.
- **Corners in (sigma, w), not polar.** sigma = d_a + d_b and w = d_a / sigma, so
  x = c + sigma (e_b + w (e_a - e_b)) is bilinear. The weights divided by sigma^2 are
  rational in (sigma, w) and smooth up to the corner:
  B_a = beta^2 F / (alpha^2 F + beta^2 F + sigma^2 alpha^2 beta^2 sum F'), with
  alpha = d_a / sigma affine in w. **No trigonometry.** Polar coordinates (tried first) put
  sin/cos of an angle range of up to 2 rad into every model, and their remainders swamped
  everything. J = (P_sigma x P_w / sigma) / det[q, e_a - e_b], where the determinant is the
  constant -det[e_a, e_b]. Near the corner, P_w / sigma is the exact graded division. The
  corner's rounding (d_a(c) = eps != 0) enters as eps / sigma, kept small by shells
  sigma in [cut, 1e-3 reach] and [1e-3 reach, reach]. Inside sigma < cut = 2e-8 side, the
  evaluation returns the nearest ribbon, and those two ribbons are proven there on their own.
- **The rest by boxes,** each shrunk to the bounding box of its part in the domain
  (clipped, with a hair of slack so rounding only keeps too much). Outside the domain the
  formula has poles near the sides. A box is skipped when its part lies in a corner's
  triangle (d_a + d_b <= reach).
- **Positivity.** n0 . (P_a x P_b) as a model. Its polynomial is bounded on up to 64 sub-boxes
  (re-bounding only the polynomial), less the graded remainder at each sub-box's largest |a|.
  On failure the region is split and modelled afresh: corners to depth 6, boxes to depth 8.
- **Cache** (`proven_cached`): keyed by the sides' full Debug text (every f64 by its
  round-trip text, so bit-exact). Values are kept, never evicted.

Tests: models enclose the functions and the patch (value and both derivatives, at boxes and
corner regions, against `deriv`), and a spherical quadrilateral proves.

### Results

12 survey patches (6, 8 and 10 sides, both orientations; the machine was shared, so times
are +-50%):

```
case  sides  proved  models (boxes)   time
0     6      yes      762 ( 382)      0.25-0.58 s
1     8      yes     2361 (1805)      1.3-1.6 s
2     10     yes     7933 (6285)      5.6-5.9 s
3     6      yes      764 ( 400)      0.25-0.34 s
4     8      yes     2055 (1511)      1.2-1.5 s
5     10     yes     6956 (6000)      4.3-5.6 s
```

The full sample, 60 single patches (30 cases x 2 orientations): **60 proved, 0 refused**;
median 1.1 s, p90 4.8 s, max 6.6 s per patch. The pieces of the forced split of 15 cases
(120 pieces of 4 sides): **118 proved, 2 refused**; median 111 ms, p90 185 ms, max 475 ms.

- **Sound proofs exist for every patch tried**, with no refusals, once the boxes may go 8
  levels deep. At depth 5 two thirds were refused.
- **Cost: 0.25-6 s per patch, 300-7000x the budget.** One model (the whole patch with first
  derivatives, n ribbons) costs 0.3-0.8 ms. Models per patch run 750-7900. Boxes dominate on
  8 and 10 sides: they pile up along the short sides, where both corners' triangles are
  small, and where a Cartesian model's radius of convergence (the distance to the nearest
  corner) is small. Corners take ~60 models each.
- **Where a model fails, it is the remainder, never the polynomial.** On failing regions the
  polynomial's lower bound sits at 80-99% of the true value, while the remainder runs 1-10^4
  times larger. Raising K to 4 or 5 did not help (refusals at the same places): the remainders
  come from compositions (1/x, x^(-1/2) of models whose range is a large fraction of their
  value), not from truncation.
- **Cache:** first build of 6 patches 18.7 s; a regeneration (same inputs, patches rebuilt)
  had 6 of 6 hits in 0.2 ms total (formatting the key is most of it); after an edit that
  moved one side's parameter range by one ulp, 5 hits and 1 miss (that patch re-proven).

### What it would take, and why it stops here

- **Per model**, a 3-5x speed-up is realistic: a fixed-index product with no branches, models
  that know they are constant, and fusing the jets' five products.
- **Models per patch**: the corner shells and the boxes along short sides would need a
  coordinate system per side strip (s_i, d_i), like (sigma, w) at the corners, so the ribbon
  structure is polynomial there. The weights' compositions would need a better remainder
  (centred forms or a mean-value remainder for 1/x).
- Even 10x on both leaves 2.5-60 ms per patch, and the floor is (n + few) models at
  ~0.1 ms, i.e. 1.5-2 ms. **The Taylor-model route cannot reach ~1 ms.** With the cache,
  its cost falls only on new or edited corners, but a cold build of a part with 50 corners
  would still take tens of seconds today and a second or more at best.
- **Intersection rails (Krawczyk):** not built. The design is in "Rail bounds" above. It
  only matters once the core is affordable, and today such rails return *unsupported* and
  keep the sampled check (no weaker than now).

### Recommendation

1. Keep today's sampled check as the production guard. It is unchanged; nothing is merged.
2. If a proof on every build matters, the prior art says how: **make the patch rational for
   the rational input class** (lines and conics on planes, cylinders, spheres: every
   planar-face fillet corner). Then det J's numerator is a polynomial, and a
   Johnen / Garloff Bernstein check proves it soundly in microseconds to a millisecond, with
   subdivision of the polynomial only. Non-rational rails keep the current patch and check.
   Open questions there: exact G1 to a cylinder along a circular section with a rational
   n-sided patch (Gregory-type rational twists, or a rational S-patch as in Zhou and Qian
   2010), the degree (it grows with n), and whether the shape stays as good as today's.
3. If that is not wanted, the Taylor-model prover with the cache could run off the build
   path (background, once per new corner) to flag patches, at 0.25-6 s per corner as it
   stands.
