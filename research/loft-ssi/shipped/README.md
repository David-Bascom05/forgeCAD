# What loftmeet.rs ships, proved

Lean 4 + Mathlib proofs of the certified steps in `crates/forge-geom/src/loftmeet.rs` and the parts
of `skinned.rs` it calls (`Poly::new`, `Poly::hull`, `Hull::extent`, `Hull::misses`), with the
assumptions each one needs, which ones it doesn't, and what the f64 code relies on. No `sorry`;
every theorem below depends only on `propext`, `Classical.choice`, `Quot.sound` (`Axioms.lean`).

## Rerun

    # toolchain: elan with lean-toolchain (v4.36.0-rc2); Mathlib v4.36.0-rc2
    lake exe cache get        # once, downloads Mathlib's build
    lake build                # ~1 min after the cache; clean, no sorry
    lake env lean Axioms.lean # axioms of the main theorems
    python3 fp/fp_check.py    # f64 rounding of hull points, extents and quadric coefficients (stdlib)
    python3 sympy/sweeps.py   # unequal sweeps: the section is not a conic (needs sympy)

`LoftMeet/Bernstein.lean` is shared, byte-identical, with lean-derive's
`research/loft-ssi/derive/proofs/Proofs/Bernstein.lean` (owned here; its de Casteljau section is
lean-derive's). lean-derive's `Rounding.lean` proves the γ_k error bounds used in the f64 section.

## The model

Points are `Fin 3 → ℝ`, a separating axis is any vector u (`ell u p = Σ u_c p_c`). The surface is
the code's own: `Poly::point` with the f64 coefficients it stores, evaluated in exact reals. One
span, b = (s - k0)/h in [0, 1], all arcs turning through the same (start, sweep), no guides:

    S(t, b) = A(b) + B(b) cos(start + sweep t) + D(b) sin(start + sweep t)      (arcs)
    S(t, b) = A(b) + E(b) t                                                     (lines)

## What is proved (file: theorem → code)

| Step | Theorem | Code |
|---|---|---|
| (a) power form = spline weight | `Hull.polyW_eq` | `Poly::new` `weight(d)` vs `Spline::weights` |
| (a) rails sum to A + B cos + D sin | `Hull.sum_arcs` | `Poly::new` parts 0, 1, 2 (end slopes add to A) |
| (a) Bernstein points over any [b0, b1] | `Hull.bpts_exact` | `Poly::hull` `bern` closure (val, val ± der·third) |
| (a) rational quadratic arc, weights 1, cos h, 1 | `Hull.arc_bernstein`, `Hull.arc_hits` | `Poly::hull` arc rows |
| (a) piece = rational (3,2) patch, W > 0 | `Patch.patch_exact` | `Poly::hull` 12 points |
| (a) lines piece = (3,1) patch | `Subdiv.lines_exact` | `Poly::hull` `!self.arcs` |
| (b) convex hull, any axis | `Patch.patch_hull`, `patch_hull_ge` | `Hull::extent` |
| (b) the code's θ range is the piece's arc | `Patch.piece_angle` | x0, x1, half in `Poly::hull` |
| (b) separating axis ⇒ disjoint, by > gap | `Patch.apart_sound` | `apart` |
| (b) halves cover; recursion | `Subdiv.halves_cover`, `clear_of_halves` | `halves`, `pieces`, `seeds` |
| (b) recomputed points = de Casteljau's | `Subdiv.recompute_eq_casteljau(_right)` | `Skinned::hull` per sub-box |
| (c) Bernstein product, degree bookkeeping | `Bernstein.bb_mul` | `C3 C3 cn cn / (C6 c2n)` |
| (c) Q(S) W² = Σ B⁶_p B^(2m)_q coef[p][q] | `Quadric.quad_subst` | `Hull::misses` loops, exactly |
| (c) one strict sign ⇒ piece misses Q | `Quadric.misses_sound`, `Subdiv.arc_misses` | `Hull::misses` |
| f64 | `Float.apart_rounded`, `misses_rounded(_sound)` | tolerances |

`quad_subst` models the four nested loops as they are: `v = h_kl.(A h_ij) + 2 b.h_ij w_l + c w_j w_l`
with `h_ij = (P_ij - o) w_j`, accumulated into `coef[i+k][j+l]` (no symmetry of A assumed). The
recursion in `seeds` does not run de Casteljau: each sub-box's points are recomputed from the
power form, which `bpts_exact` covers for any interval; `recompute_eq_casteljau` shows it is the
same polynomial as de Casteljau's split of the parent.

## Assumptions: needed, not needed, weakened

    assumption (where the code checks it)              status        evidence
    natural / clamped spline (m solves the system)     NOT NEEDED    polyW_eq, patch_exact hold for any m
    weights sum to one                                 NOT NEEDED    nothing uses it
    all arcs share START                               NOT NEEDED    arc_any_start: rotate X, Y per section
    all arcs share sweep SIGN                          NOT NEEDED    arc_any_start (σ = ±1)
    all arcs share |sweep|  (Poly::new)                NEEDED        sweeps.py: pi and pi/2 give a section
                                                                     on no conic, so no degree-2 angle form
    no line mixed with arcs (sp[3] == 0 check)         NEEDED        t vs cos t: not a (3,2) rational patch
    positive weights, for the HULL (|h| < π/2)          NEEDED        neg_weight_counterexample: π/2 < h < π,
                                                                     all control x < 0, arc reaches x = 1
    zero weight (h = π/2)                              NEEDED > 0    zero_weight_counterexample: the middle
                                                                     point is at infinity (cos h = 0)
    nonneg weights for finite-point rational hulls     WEAKENED      rational_hull: w >= 0, Σ > 0 suffice
    piece under half a turn for the EXACT FORM         WEAKENED      patch_exact needs only |h| < π (W > 0)
    piece under half a turn for the SIGN TEST          WEAKENED      arc_misses: |h| < π and cos h != 0;
                                                                     weight sign irrelevant (W² > 0)
    whole face under half a turn                       NOT NEEDED    pieces() splits into quarter turns
    convexity of anything but the hull                 NOT NEEDED    patch_hull/apart_sound: any axis u, the
                                                                     surface need not be convex; u need not
                                                                     be unit or a normal
    eps > 0 in misses / gap > 0 in apart (exact math)  NOT NEEDED    misses_sound with eps = 0, apart_sound
                                                                     with gap = 0; they are rounding margins

So the code's guard `half.abs() >= 0.49π → None` is the hull's real requirement (|h| < π/2) with
margin; `pieces()` keeps |h| ≤ π/4 anyway. `Poly::new`'s requirement of an identical (start, sweep)
could be relaxed to an identical |sweep| (fold each start and sign into its X, Y); not done, it is
the code's call.

## Floating point

What the f64 code assumes, and whether its tolerances cover it:

- Reference surface: the f64 coefficients `sp`, `start`, `sweep` evaluated exactly. The kernel's own
  f64 evaluation and the hull points differ from it by rounding only. Sub-boxes tile exactly in
  f64: both halves use the same midpoint value, `pieces` the same `k as f64 / n`.
- `apart`: the hull points and extents carry at most ~12.5 u L of error (measured, fp_check.py, L =
  max |cubic coefficient|, u = 2^-53; a γ_k bound gives the same order) plus ~1-2 ulp of dmath
  sin_cos times |B| + |D| over cos h ≥ 0.707. `apart_rounded`: the absolute gap tol::LINEAR = 1e-5
  covers that while 2 × 12.5 u L ≤ 1e-5, i.e. coordinates below ~1e9 mm. COVERED.
- `misses`: eps = 1e-12 × max|ĉ| is relative to the largest coefficient, not to the size of the
  terms, so it is NOT a rounding bound. Measured: error up to 2.3 u T (T = Σ|terms|), which is up
  to 4.7e-5 × max|ĉ| when the piece is far from the quadric's frame origin or small against its
  radius (2,783 of 10,500 coefficients exceeded the eps). What the code does get (`misses_rounded`):
  a cleared piece has Q(S) W² > -E, i.e. it can dip into the quadric by at most about
  E / (|∇Q| W²) ≈ u (|P - o| for a plane, R + |P - o|²/R for cylinder/sphere/cone), about
  1e-10 mm at 1e6 mm. Below tol::LINEAR for any model under ~1e9 mm: NOT a soundness gap at the
  kernel's tolerance, but the verdict at an exact tangency depends on rounding. Seen: the tube top
  touched by a tilted plane along a line returns Empty with the plane's origin 1e5 away and one
  curve with it at 0 (scratch experiment, not committed: the contact is within 1e-11 of either).

## Soundness gap found (seeding/marching, step (d), not the certificates)

`Pair::follow` skips a seed whose Newton point lies within 1.5 steps of any point of a curve
already traced. A second branch running closer than 1.5 steps (0.75 leaf = region diagonal / 85)
to the first is dropped, and `meet` returns one curve where there are two, with no tracer
fallback. Failing test (ignored so CI stays green): `loftmeet::tests::close_parallel_branches_are_both_found`
(tube of radius 8, plane z = 7.98, branches 1.13 apart, 1.5 steps = 1.72: one curve; at z = 7.9,
2.5 apart, it finds both). Reported, not fixed.

Also unproved by design: `meet` returns `Empty` when `follow` traces nothing, not only when every
piece is certified. A leaf whose Newton point lands outside the grown region or the carried-on
domain is dropped silently. A scan of 3,600 plane heights over three bulging tubes found no wrong
Empty; this is the place to look if one appears.

## Open

- (d) Newton/marching is numerical; no proof attempted.
- The γ_k bounds are lean-derive's `Rounding.lean`; here they enter as the hypothesis E, and the
  measured constants come from fp_check.py rather than a Lean instantiation of the exact loop.
- `lines` sign test: `quad_subst` covers n = 2 (weights 1) but there is no lines corollary like
  `arc_misses`; it is the same proof with `lines_exact`.
