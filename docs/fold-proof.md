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

## Stage 3: a rational patch with a Bernstein proof

`crates/forge-geom/src/corner/rational.rs`, built by `HolePatch::new_rational` and proven by
`HolePatch::prove_rational`. Today's patch stays the default; nothing calls the rational form
in builds yet (superseded by stage 4's quads). The measurement example is
`research/corner-patch/rational_compare.rs`; it and a render switch ran on
`refs/experiments/corner-rational-stage1`.

### Construction

The same side-based transfinite scheme as today, with every piece made polynomial:

- **Sides are their exact rational curves.** A line, a parabola (an exact quadratic Bezier),
  or a circular arc (an exact rational quadratic, weights 1, cos(sweep/2), 1). It is the same
  point set as the rail. Only the parametrisation along the side changes, and
  `side_param_uv` maps rail parameters through it. Neighbouring sides share their end point
  bit for bit.
- **The cross tangent is T = a(s) C' + b(s) D^k (n x C')**, with n the neighbour's normal
  field: constant on a plane (k = 0); (C - o) / r, flattened for a cylinder, on a sphere or
  cylinder (n D is polynomial, k = 1). So T lies exactly in the neighbour's tangent plane and
  needs no normalisation. a and b are quadratics. Their ends make neighbouring ribbons agree
  to first order at each corner, and their middles make the frame components of T
  interpolate linearly, as today's do.
- **The side parameter is affine** (the projection onto the domain side). The ratio
  d_(i-1) / (d_(i-1) + d_(i+1)) put (a + b)^q into every ribbon's denominator and distorted
  the blend. The bend K matches the neighbour's curve to second order along it.
- **The blend is homogeneous:** P = sum B_i Num_i / sum B_i Den_i with B_i = prod_(j != i) d_j^2
  and R_i = Num_i / Den_i. Den_i = E(s) (H + d)^2 (2H - d)^2. The last factor cancels most of
  the damping denominator's growth across the domain. Without it the weights favoured far
  ribbons and the patch's curvature was 2.5-5x today's. Damping H is 0.7 of the side's
  height.
- **The domain distances** are d_i = lambda_i cross(c_(i+1) - c_i, x - c_i). That is exactly zero
  at both corners of the side (in exact arithmetic, with the stored corners), so in corner
  coordinates d_a = sigma w kappa_a and d_b = sigma (1 - w) kappa_b exactly.
- **Near a corner** (both distances under 1e-6 side) the nearest ribbon is the patch, as
  today (there with 1e-8).

### The proof

For P = N / W over a region with coordinates (u, v),
`n0 . (P_u x P_v) W^3 = W det(n0, N_u, N_v) - W_u det(n0, N, N_v) - W_v det(n0, N_u, N)`,
which is a polynomial. With W > 0 (all its Bernstein coefficients positive), its Bernstein
coefficients prove n0 . J > 0. Regions:

- **Each corner**, in (sigma, w), in two shells: sigma from cut / kappa up to 1e-3 of reach,
  then up to reach = 0.4 of each side. The weights are divided by sigma^2 symbolically:
  B_a / sigma^2 = beta^2 prod_far, and so on. The map's Jacobian is -(...) sigma det[e_a, e_b],
  of one sign.
- **Each corner's cut-off zone** (both distances under the cut), where the evaluation returns
  the nearest ribbon: both ribbons, alone.
- **The middle:** the 2n-gon left after cutting the corners, as n bilinear quads round its
  centre.
- Any region that does not prove is split in four and measured against its own centre's
  normal (n0 . J < 0 is no fold when the normal turns past 90 degrees), to depth 3. The
  polynomial is subdivided (de Casteljau) to depth 8.

**Soundness.** All polynomials are tensor Bernstein with one coefficient error bound each,
carried through every product, sum, derivative and subdivision (relative to the operands'
largest coefficient, with the number of terms). Region vertices are exact reals enclosed in
intervals. The same exact vertices are shared between neighbouring regions, so the regions
cover the domain exactly. The numerator is taken about a point of the region
(N - p0 W), which leaves J unchanged and keeps the bounds relative to the patch's size, not its
distance from the origin. The proof is of the patch defined by its stored f64 constants in
exact arithmetic. The f64 evaluation differs from it by rounding, the usual separate step.

### Stage 1 comparison: three-edge corners (six-sided holes)

Interpretation: the "3-sided corners" are vertices where three blends meet (box corners),
whose holes have six sides (three circular sections on the cylinders and three trims on the
planes). 60 survey cases, 120 holes with both orientations, all on planes and cylinders:

```
                                today              rational
fold proofs                     none (sampled)     120 of 120 proven, 0 refused
boundary distance (worst)       3.0e-15            8.2e-15
G1 normal error (worst, rad)    1.8e-15            6.5e-15
derivative check (worst)        3.2e-6             3.7e-6
projection error (worst)        8.3e-13            7.2e-13
bbox misses                     0                  0
min sampled fold cosine         0.991              0.974
max curvature, rational/today   -                  median 1.04, range 0.60-3.95
bulge off the boundary's plane  -                  median 1.11x, range 1.05-1.24x
largest gap between the two     -                  median 4.2%, max 9.7% of the hole's size
build (without proof)           0.34-0.56 ms       1.7-2.1 ms
eval point + 2nd derivatives    ~2 us              ~4-6 us
proof                           -                  median 33-43 ms, p90 66-78 ms, max ~110 ms
```

The machine's load was 35 during these runs, so times are +-50%. Ratios between the two
patches from the same run are steadier.

Renders (`forgecad render`, iso): `.scratch/{box,box2,skew}_side_by_side.png` in the worktree:
a box corner with fillets 4/7/10 and 2/5/9, and a skewed (non-right-angle) corner with 3/6/9.
They are visually the same; the rational one shades slightly darker where the corner patch
meets the largest fillet.

**What this shows.** Exact G1 and a sound fold proof are possible for the whole rational
class, with zero refusals. The shape is within a few per cent of today's at the median, with
a 4x curvature outlier still to look at. But **the proof costs ~40 ms, not < 1 ms.** The
Jacobian numerator is degree 59 in each variable (B_i alone is degree 10 for six sides), and
products of polynomials that size dominate: about two thirds of the time is the Jacobian,
one third building N and W. Ways down, none tried yet:

1. Fewer regions: 30 today (6 corners x 2 shells x 1-2 pieces, 12 zones, 6 quads). Larger
   corner regions meeting at the side midpoints would leave 3 middle quads. The zones are
   cheap.
2. Lower degree: no bend term (-1 per ribbon), and a cheaper damping or weight compensation
   (each factor of (H + d)(2H - d) costs +4 in N and W, +12 in the Jacobian).
3. Exact-arithmetic-free shortcuts that remain sound: a first test on a degree-reduced
   numerator with a bounded reduction error, before the full product.
4. With the certificate cache (keyed by the exact sides), only new or edited corners pay.

Four- to six-edge corners (8-12-sided holes) would raise the degree further (B_i is degree
2(n - 1)) and the cost with it.

## Stage 4: rational quads for 4 to 20 sides, and a faster proof

Measurement code and how to rerun it: `research/corner-patch/`.

`crates/forge-geom/src/corner/rational.rs` (`Quad`) and `close_hole_rational`, `plans` and
`Cells` in `corner.rs`. Builds use it only where today's patch has none ("Switching builds").

### Construction

- **Quads round a centre.** A hole of four sides is one quad; any other hole of 3 to 20 sides
  is split by spokes to a centre, two hole sides (or halves) per quad. The degree no longer
  grows with the sides: every quad's blend is a tensor polynomial of degree (7, 7).
- **The domain is the unit square**, with s and d per side (side 0: s = u, d = v). Ribbons are
  (A + d B + d^2 C) / E of degree 3 in s and 2 in d, undamped (a Coons-like quad needs no
  damping: linear ribbons of a bilinear surface reproduce it). Weights B_i = prod_(j != i) d_j^2
  are of degree (2, 4).
- **The cross tangent** is T = (a Q + b F) / D with a, b linear, Q = D^2 C', and F a field
  across the side in the neighbour's tangent plane of degree at most 2: n x Q for a plane, the
  quadratic (n D x Q) / D for a circle on a cylinder or sphere (|Q| is proportional to D for a
  circle), and for a spoke a linear F of its own.
- **Spokes are cubic Beziers** leaving their vertex along the hole's halving direction and the
  centre straight toward the vertex, control lengths a third of the way. Parabolic spokes
  (today's split) cannot leave in a chosen direction: their control point is pinned to the line
  where the vertex's and the centre's tangent planes meet, and for 8 or more sides it often
  left nearly along a side. The spoke's normal is w / |w|, w = p' x F, F linear from
  n_v x p'(0) to n_c x p'(1): both quads along a spoke use the same plane span(p', F), so they
  meet with G1 by construction (measured 1e-14 rad). Fitting the control lengths to today's
  patch made speeds uneven and folded a third of the 8-sided holes; equal lengths fold none.
- **The centre** is today's ribbon patch at its domain's middle (one evaluation), or the old
  heuristic beyond 12 sides, where the ribbon patch has no form.

### One face per corner

The quads are one `HolePatch` (one B-rep face, `Closing::One`), joined along the spokes by
internal G1 seams; the spokes are not B-rep edges.

- **Where spokes leave** (`plans` in `corner.rs`): every other vertex from some start, never a
  kinked vertex (a sharp edge's end, where the sides' normals differ); a side between two
  vertices without spokes is halved and a spoke leaves its middle, splitting that side's
  domain edge but not its B-rep edge. That one rule covers even holes (no halving), odd ones (one
  side halved, the longest first) and kinked vertices (both sides round one halved). Every side
  halved is the last way; at least three spokes. A build tries the first two ways (a proof
  failing on the first is retried on the second): research/corner-patch/sharp.rs.
- **The domain**: one quad is a square; m >= 3 quads are parallelograms round the origin, cell
  i with corners A_i, A_i + A_(i+1), A_(i+1) and the origin (A_i of length k at the angle
  2 pi i / m), quad sides 0 and 1 on the outline, the spokes on the cell edges from the origin.
  Each cell's map is affine, and both cells along a spoke map it alike (the spoke's own
  parameter), so the parametrisation is C0 across a seam and the surface G1. Hexagon (m = 3)
  and square (m = 4) outlines are convex; from m = 5 the outline is a star.
- **Evaluation** finds the cell holding the point (deepest in cell coordinates) and evaluates
  its quad with the affine map's gradients; derivatives jump across a seam, the surface's
  tangent plane does not.
- **Projection** is Newton in a cell's unit square, stepping into the neighbouring cell while
  the nearest point is on their seam and the distance falls across it. Each cell has its own
  13 x 13 seeds; Newton starts from the nearest seed of every cell whose nearest seed is within
  twice the nearest seed's distance, as a tight quad can hold a false minimum next to a seam
  (one survey hole projected 7e-2 off from the single nearest seed; the old per-quad patches
  failed there too).
- **Boxes**: the union of each cell's quad box over the part of the box in the cell. Points
  outside the domain evaluate at their nearest domain point: within the box's half diagonal of
  the centre's nearest point for a convex outline, else within the centre's distance from the
  domain plus the whole diagonal.
- The fold proof stays per quad (each cell's affine map has a positive Jacobian).

One face on the surveys: 6, 8 and 10 sides proven 120, 120 and 120 of 120 (10 sides was 117
with one way to place spokes; the second way proves the rest). With sharp edges (`sharp.rs`,
60 cases, holes proven of 40): one sharp edge, 5 / 7 / 9 sides 40 / 40 / 28; two, 4 / 6 / 8
sides 40 / 40 / 32; three, 5 / 7 sides 40 / 38. Worst boundary 4.8e-15, G1 normal 2.2e-13 rad
(seams included), projection 4.5e-13, no box misses. Meshes pass the same checks as today's
patch (`rational_corner_patches_mesh`).

### The proof: a cone test

Instead of the Jacobian's Bernstein coefficients (degree ~3x, products dominate), the proof
bounds the directions of W P_u and W P_v. With Y = N - q W for a constant q near a box's
middle, W P_u = Y_u - (P - q) W_u = sum_ij b_ij (Y_u,ij - (P - q) W_u,ij) for the common basis
values b_ij, and P - q lies in the hull of the control points Y_kl / W_kl (all W_kl > 0). So
W P_u lies in the union of the coefficients shifted by W_u,ij times that hull. If both sets lie
in angular sectors (of the plane across some n0) that are ordered and less than a half turn
apart, n0 . (P_u x P_v) > 0 over the box: the patch is regular there. Everything is in
tangents of angles, with every coefficient's rounding carried.

- **Normalised weights.** P = (N phi) / (W phi) for any positive phi; each region root is
  multiplied by phi fitted to 1 / W (biquadratic through 9 values, bilinear in Duffy
  triangles), so W phi is nearly constant and the shift term small. Children inherit it.
  This halved the tests.
- **Regions**: the square cut at 1/4 and 3/4 both ways. The middle box is one region (it
  nearly always passes whole); each strip between two corners' boxes is two (whole, a strip
  took ~4 tests); each corner's box is its eighth at the corner in Duffy triangles and the three
  other eighths plain. Per quad that is 8 zones, 9 plain boxes, 12 plain eighths and 8 Duffy
  triangles, each nearly always one test.
- **Corners** (W vanishes there to second order): the eighth of the square round each corner
  in two Duffy triangles x = sigma, y = sigma t. Every weight has u^2 or v^2 and
  the ribbons meet at the corner point p_c, so N - p_c W = sigma^3 G and W = sigma^2 W~ with the
  vanishing rows known exactly; the cones are of W~ P_sigma (from sigma G) and W~ P_t / sigma
  (from G), both finite at the corner.
- **Corner zones**, where evaluation takes the nearest ribbon (both distances under 1e-6): each
  ribbon alone over a power-of-two box holding the zone.
- Regions that fail are split in halves along u and v in turn. Taking
  P_x = P_sigma - t P_t / sigma instead cancels badly coefficient-wise (wider cones); boxes
  round the corners larger than a quarter need more tests, not fewer. A whole triangle of a
  quarter box rarely passes (2.4 tests each from its halves), one of an eighth nearly always
  (1.05 tests). Tried and dropped: bilinear phi for plain boxes (twice the tests), the strips
  whole (+13 tests per hole), the eighths beside a corner as two boxes (+14), corner boxes of
  an eighth with the 3 x 3 cut (strips and middle fail: 10x the tests).
- **Cache**: results keyed on the bits of every ribbon coefficient (what the proof reads), kept
  up to 20000 entries.

A test builds a quad whose top side swings across its bottom: the proof refuses it, and a
sampled Jacobian changes sign. On the survey every refusal checked was a real fold (dense
sampling: normals of neighbouring samples reversed); every proven quad sampled densely had
neighbouring-normal cosines of 0.98 or more.

### Results (survey: the stage 1 generator's 3-, 4- and 5-edge corners, both orientations)

```
sides                         6            8            10
holes proven                  120 / 120    120 / 120    117 / 120
cone tests per hole           116 (154)    168 (238)    252 (1060)
instructions per hole:
  today's patch (build)       2.8 M        4.2 M        5.4 M
  rational build              1.9 M        2.5 M        3.1 M
  rational proof              13.6 M       18.5 M       27.1 M
build ms, median (load ~20):  today 0.31   0.50         0.42-0.6
  rational, proof cached      0.22         0.29         0.24
  rational, cold              1.4          2.0          2.0
boundary (worst)              3.7e-15      2.9e-15      3.0e-15
G1 normal (worst, rad)        3.7e-15      1.6e-14      9.4e-14
projection (worst)            4.0e-13      3.9e-13      5.9e-13
bbox misses                   0            0            0
gap to today's / hole size    2.1% (7.2%)  3.2% (20%)   8.0% (68%)
bulge rational / today        1.00         1.04         1.18
max curvature rational/today  1.67 (5.7)   1.64 (4.4)   2.6 (147)
```

(median, max in brackets.) Beyond the survey, corners of 6 to 10 edges (12 to 20 sides, where
today's patch has no form at all): 12 sides 38 / 40, 16 sides 30 / 40, 20 sides 22 / 40 proven;
the rest really fold (long thin quads round one centre) and would need a second ring of quads.

- **Speed.** Cold, build plus proof was 5.5x today's instructions; with the proof cached it is
  0.65x. The proof went from ~40 ms to ~1.3 ms (6 sides): degree 59 to 7, products to cone
  tests, allocation-free arrays, normalised weights. Then (one face, `ic.sh`, instructions per
  hole; close is today's `close_hole` with its fold check, cold is `close_hole_rational`):

  ```
  sides                     6        8        10
  today (close)             2.82 M   4.19 M   5.43 M
  proof, stage 4            13.5 M   18.5 M   27.1 M
  proof                     7.47 M   10.2 M   15.0 M
  cold                      8.89 M   12.2 M   17.8 M
  cold / today              3.15     2.90     3.28
  proof, lanes              4.18 M   5.72 M   8.23 M
  cold, lanes               5.57 M   7.61 M   10.9 M
  cold / today              1.98     1.82     2.01
  ```

  Lanes: N's x, y, z and W are one polynomial whose coefficients are [f64; 4] (`Bp<4>`; a
  Duffy triangle carries sigma G, G raised a degree and W~ as `Bp<7>`, so a test there no
  longer multiplies and raises), so every split, half, product and Duffy map is one loop over
  four lanes at once; the cone test reads Y and the control points as rows of x, y, z apart,
  so its loops over coefficients run four at a time; and each polynomial carries a bound on its
  coefficients (a convex combination's is its parent's) instead of measuring it in every
  operation (error bounds a little wider). The same proofs: cone tests 14346 / 20364 / 34073 (5 rounds
  of the survey) and the sharp-edge counts unchanged.

  From: both halves of a split in one pass and the phi product in contiguous rows (identical
  arithmetic, the same proofs; 13.5 M to 9.2 M), the regions above (to 8.96 M, then 7.47 M),
  the Duffy map as dense rows of a precomputed table (identical sums; -0.9 M), Y = X - q W
  kept as one array of points instead of three polynomials (-0.3 M). Builds stay on today's
  patch: the rule is within 2x. What is left: about one test per region, so the cost is the
  regions (8 zones, 21 plain, 8 Duffy per quad) times ~70 k instructions of setup and test,
  a fifth of it copying fixed-size coefficient arrays, and the build's seeds (13 x 13 per cell;
  9 x 9 brings back a projection failure).
- **Shape.** Within a few per cent of today's patch on 6 and 8 sides, but the maximum curvature
  is ~1.65x today's (peaks near the vertices the spokes leave and along the spokes). For 10
  sides today's own patch is poor (fold cosine 0.79), so the comparison is loose.

### Curvature (explored; nothing changed)

`rq.rs` max curvature over the one face against today's patch (median, 60 cases): 1.49 for 6
sides, 1.50 for 8, 1.96 for 10. Where the peaks are (each quad's peak, a probe not kept): at the quad
corners, 284 of the 6-sided survey's quads at a corner a spoke leaves, 53 at a hole vertex.
Along rays into a corner the curvature does not settle to one value: it depends on the
direction even at r = 0.003 (0.67 to 1.17 of today's maximum round one corner). The two
ribbons meeting there agree in position, tangent plane and the second derivatives along each
side (the bend term), but not in the mixed term n . P_sd, which each takes from its own
neighbour's second fundamental form, II(side, cross). At a hole vertex those are two real
surfaces (a plane trim and a cylinder section): no G1 patch matches both, and today's patch
has the same corners. At a spoke vertex one neighbour is the spoke's own normal field.

Tried, each on 6 / 8 / 10 sides (median ratio; baseline 1.49 / 1.50 / 1.96):
- spoke leaving along today's patch's direction instead of halving the angle: 1.63 / 1.71,
  a 25x outlier;
- spoke arriving at the centre along today's patch's direction: 1.51 / 1.45;
- control length at the vertex 1/4 or 0.4 of the way (equal is 1/3): 1.61 / 1.61, 1.60 / 1.62;
  at the centre 1/4: 1.64 / 1.67; 0.4: 1.44 / 1.54 (a 10x outlier); both with the centre's
  direction, 0.37: 1.45 / 1.48 / 1.97, two 10-sided holes no longer proven;
- bend K a quarter or none of the neighbours' second derivatives (exact is a half): 2.31 /
  1.96, 3.39 / 2.80; 0.6 and 0.75: 1.50 / 1.76, 2.10 / 2.47;
- the spoke's f quadratic, its middle chosen so the spoke's II(p', X) at the vertex matches the
  two sides' neighbours' in least squares: 1.44 / 1.45 / 2.11.
None is worth its cost. What would move it: corners at hole vertices are intrinsic; the
spoke vertices are where the gain is, and a fit of the spoke's turning to both sides at once
needs a cubic f (degree 4 ribbons, a dearer proof).

### Switching builds: what whole parts would pay

`research/corner-patch/switch.sh`: every fillet-corpus case (cases, variant and variable) with
a corner hole, regenerated with builds on today's patch and on `close_hole_rational` (falling
back to today's on refusal), instructions per regen (millions, the runner's start-up taken
off; cold, nothing cached):

```
case                     today  switched        holes
box-corner-unequal        3.41    9.66  2.84x   rational 6 sides
two-blends-sharp-edge     2.70    9.38  3.48x   rational 5
step-mixed-corner         4.53   11.28  2.49x   rational 6
pocket-unequal-corners   13.00   38.24  2.94x   rational 6 x4
box-corner-variable       9.04   13.61  1.50x   rational 6 x2
apex-four-unequal         7.41   14.94  2.02x   rational 8
apex-variable            16.79   22.58  1.34x   rational 8 x2
pit-four                 13.93   21.50  1.54x   rational 8
roof-boss-on-ridge        6.59   16.04  2.43x   rational 8
roof-slot-on-ridge        6.55   14.61  2.23x   rational 8
apex-five-unequal         9.43   18.85  2.00x   rational 10
pit-five                 19.89   29.00  1.46x   rational 10
roof-diamond-on-ridge     8.17   35.00  4.29x   rational 10
box-ball-corner, box-all-edges, pocket-ball-corners, apex-four-ball: spheres, 1.00x
```

Every hole of the corpus proves (no fallback). The gate's parts (torture, plate, shaft,
housing, coplanar, patterns, mixed, hole-grid, tour, upto, upto-flat) close no corner hole,
so they would not move; but where a part has corners, the corner is most of its regen and the
switch costs 1.3x to 4.3x the whole part, far past the gate's 0.5%. Builds stay on today's
patch until the proof is cheaper.

**Switched, then held back** (after the proof in lanes, cold 1.8x-2.0x today's per hole):
`forge_ops::corner::patch` closing a hole with `close_hole_rational`, falling back to
`close_hole` when no plan proves or builds. Regen instructions (millions; today, switched):

```
box-corner-unequal 3.41 6.28   two-blends-sharp-edge 2.70 6.05   step-mixed-corner 4.53 7.61
pocket-unequal-corners 13.00 24.58   box-corner-variable 9.04 10.47   apex-four-unequal 7.41 10.62
apex-variable 16.79 18.25   pit-four 13.93 17.17   roof-boss-on-ridge 6.59 10.67
roof-slot-on-ridge 6.55 10.06   apex-five-unequal 9.43 13.34   pit-five 19.89 23.68
roof-diamond-on-ridge 8.17 8.59 (falls back)
```

Two things the switch turned up on the fillet corpus:

- **Spokes pinched at the centre.** The spokes leave the centre toward their vertices projected
  into the centre's tangent plane; on a hole far from flat (roof-diamond-on-ridge, a diamond
  boss astride a ridge) two of those projections nearly coincide (0.5 degrees apart), the
  quad between them is pinched there, and the patch, though proven regular, has a normal
  curvature of 78 against today's 4.9 (radius 0.013 on a fillet of ~1): the mesher's chord
  check failed (25 chords off at 0.002) and projection found false minima. The smallest angle
  between consecutive spokes at the centre, over the corpus: 0.5 (roof-diamond), 12.7
  (roof-boss, curvature 14 against 2.3), then 51 and up; over the survey's 10-sided holes 16.8
  (curvature 296 against 18), then 26 and up. `spokes` now refuses a plan whose spokes leave
  the centre within a quarter of an even share of the turn of each other (18 degrees for five
  spokes): roof-diamond falls back to today's patch, roof-boss takes its second plan (curvature
  10), the survey proves 120 / 120 / 118 (the two refused are the 296x outliers) and the
  sharp-edge counts are unchanged.
- **Projection started in the wrong cell.** Newton from each cell's nearest seed started in
  the cell the seed is deepest in; a seed on a seam belongs to both, so both runs started on
  one side. It now starts in the seed's own cell.

Fillet corpus meshes, reliability (0 worse of 534) and forge-proof's metamorphic checks (no
new failure) all passed switched, but forge-agent's `drafted_rib_into_blend` (R3) did not:
the drafted rib's plane meets the rational patch of the feet's fillet corner. With the
carried-on implicit (main today) the octree seeds a second branch past the patch's rim (zero
there), traced 1e-4 beside the first, and the two pieces broke the boolean's classification;
with the signed distance back (loft-speed's cc3135e) the single branch ends exactly on the
patch's rim edge at z = 16 and gets no cut there, and classification fails elsewhere. Today's
patch passes both ways. So builds stay on today's patch where it closes the hole, and take the
rational quads only where it has none (more than twelve sides, every way folding): the switch is the one-line change in
`patch` once the tracer cuts a branch ending on a face's rim (or carries it across through the
face domain), after loft-speed lands.

### Also done here

- **Boxes**: the quad's box over a parameter box is the hull of the control points there (all
  weights non-negative), grown for rounding; no misses.
- **min_radius of today's patch**: dense sampling (300 x 300) of two-blends-sharp-edge finds a
  radius of 0.028 near the corner where the sharp edge ends, against 0.53 reported; at such a
  corner the sides' normals differ and the patch's normal turns in no distance, so there is no
  bound. `min_radius` now returns 0 there (callers read that as unbounded curvature); the
  sampled value still sizes the domain. Smooth patches still report a sample, 25-40% over the
  dense one (box-corner-unequal 0.585 against 0.446, peaks on the sides).
