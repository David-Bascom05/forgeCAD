# Certified surface–surface intersection tracing

Item 6 (§2.6) of [README.md](README.md). The tracer in `kernel/forge-geom/src/intcurve.rs` marches
and hopes. This note lists every heuristic it uses, then designs a certificate that runs after the
march. The certificate either proves the march's result or refuses and names the region where it
failed. It never changes a result it has proven, and it never lets an unproven curve through.

## 1. The tracer today: every retry and heuristic

`trace(a, b, region)` (intcurve.rs:533) and `walk` (intcurve.rs:780):

| # | Where | What it does | What can go wrong |
|---|---|---|---|
| H1 | `trace`, `leaf = size/64 ∧ feature ∧ ≥ 10·LINEAR`, `step = leaf/2` | fixes one step for the whole region | too coarse at a tight bend: the step jumps to another arm (the census wrong-mass bug) |
| H2 | octree cull `|f(c)| > r` | Lipschitz exclusion, in floats | sound up to rounding; not outward-rounded |
| H3 | `covered` cells, `near_chain(…, 1.5·step)` | skips seeds near traced chains | a second branch within 1.5 steps of the first is never seeded |
| H4 | `refine`: 40 Newton steps, `det ≤ 1e-12·aa·bb` ⇒ None, accept at `1e-3·LINEAR` | the corrector | converges to whichever branch is nearest the guess |
| H5 | `ahead`: lands `> 0.5·h` ahead and within `h` of the guess | rejects a jump or a stall | a jump to a nearby arm that lands ahead passes |
| H6 | `h *= 0.5` down to `step/64` | shortened-step retries | none of them proves anything |
| H7 | `TURN_COS = 0.866`, `TURN_LEAST = 1/1024`, `all_but_touch` (`0.05`, `2e-4`) | retries steps that turn too far near a touch | thresholds tuned to fixtures |
| H8 | "turned" tie-breaks: a second probe at `2h`, "a turn a shorter step does not lessen" | decides which step stands | heuristic choice of branch at a near-crossing |
| H9 | `carried`: steps of 1, 2, 4 × step along the chord, `|t·chord| ≥ 0.866` | steps over a touch | may step onto another arm |
| H10 | `ends`: a procedural face stops where `|proj(x) − x| > h/2` | ends the branch at a rim | |
| H11 | last-ditch forward steps of 1, ¼, 1/16, 1/64 × step with `least = 0` | crawls into an apex | any headway counts |
| H12 | `STALLS` thread-local, `trace_past_stalls` | turns a stall into a branch end the caller judges | |
| H13 | closing: `q.distance(p0) ≤ 0.75·step`, `come > 2·step`, `d·first > 0`; `back` fallback | detects a closed loop | closes on a nearby arm, or runs a loop twice |
| H14 | forward/back join: `4·over ≥ 3·len` (`1.5·step`) | decides the way back already ran the whole loop | |
| H15 | `finish`: `sag = 1.5·max chord-midpoint distance + LINEAR` | the chain's error bound | measured at midpoints only |
| H16 | `refined(r)`: halves steps that turn by > ~10° (6 rounds) | densifies tight bends for balls | |
| H17 | `max = 64·diag/step + 4096` steps | stops a run-on | |

Around it: `sheet::apex_met` refuses any apex in the region (ssi.rs:173), and forge-bool's
`CAREFUL` rung (boolean.rs:937–972) retries a failed boolean with finer torus boxes.

meet.rs is a different thing: closed forms on a host surface for every quadric and torus pair,
with the curve split at the discriminant's roots ("events"). It gives up (returns `None`) where its
event search does not settle (e.g. `None if … step(j) != 2`, meet.rs:1189), and those pairs fall
to `trace`. So the analytic pairs `trace` sees are exactly the hard ones: tangencies, horn tori,
near-singular configurations.

## 2. Sources

- Grandine & Klein, "A new approach to the surface intersection problem", CAGD 14(2), 1997,
  111–134. Topology first: find the turning points (where the curve's tangent is normal to a sweep
  direction) and the singular points (where the gradients are parallel), then trace between them.
- Sederberg & Meyers, "Loop detection in surface patch intersections", CAGD 5(2), 1988, 161–171.
  A loop can hide inside a cell no seed reaches. Here, exclusion of every cell not covered by a
  certified tube (§3.3) finds every loop, or refuses.
- Krawczyk, "Newton-Algorithmen zur Bestimmung von Nullstellen mit Fehlerschranken", Computing 4,
  1969. The operator below.
- Neumaier, *Interval Methods for Systems of Equations*, CUP 1990, §5.1 (Krawczyk operator,
  existence and uniqueness), §2.1 (mean-value forms), §5.4 (parametric systems).
- Rump, "Verification methods: rigorous results using floating-point arithmetic", Acta Numerica
  19, 2010, §13. `K(X) ⊂ int X` implies uniqueness, with the Jacobian enclosure taken over X.
- Kearfott & Xing, "An interval step control for continuation methods", SIAM J. Numer. Anal. 31,
  1994; Martin, Goldsztejn, Granvilliers & Jermann, "Certified parallelotope continuation for
  one-manifolds", SIAM J. Numer. Anal. 51(6), 2013. Certified continuation by chained parametric
  Krawczyk boxes, as here.
- Beltrán & Leykin, "Certified numerical homotopy tracking", Exp. Math. 21(1), 2012 (the α-theory
  alternative; it needs analytic bounds the procedural surfaces don't give).

I checked these against the claims above as far as I could from memory. Neumaier's book is the
reference for the operator's statement (Theorem 5.1.7 and the parametric form). The exact theorem
used here is proven independently in §5 (Lean), so the design does not rest on a citation.

## 3. The certificate

### 3.1 The objects

- The two surfaces, each as a **polynomial in the stored parameters, evaluated in intervals**. The
  local coordinates are `l = (x·q, y·q, z·q)` with `q = p − origin` and x, y, z the frame's stored
  vectors. This is the kernel's own formula (`Surface::implicit`), evaluated exactly. Each form
  vanishes exactly where the kernel's implicit does:
  - plane: `l_z`;
  - sphere: `|q|² − r²`;
  - cylinder: `l_x² + l_y² − r²`;
  - cone: `cos²·(l_x² + l_y²) − (r·cos + sin·l_z)²`, and only where `r·cos + sin·l_z ≥ 0`. The
    other nappe is ruled out on every certified box by an interval sign check;
  - ring torus: `(ρ² + l_z² + R² − r²)² − 4R²ρ²` with `ρ² = l_x² + l_y²`. Both roots ρ = R ± √(r² −
    l_z²) are ≥ 0 when R ≥ r. Spindle tori (R < r) and procedural surfaces are out of scope for
    now (§3.6).

  The gradients are the same polynomials differentiated. Intervals are `forge_math::Interval`:
  IEEE round to nearest, each bound pushed out one ulp (`next_down`/`next_up`). That is sound,
  because a correctly rounded result is within half an ulp. Only `+ − ×` and division by an
  interval clear of 0 are used. No transcendental functions, no fused multiply-add. So native and
  wasm are bit-identical.

- **Enclosures.** For a box B with centre c: the natural extension in local coordinates,
  intersected with the mean-value form `f(c) + ∇f(B)·(B − c)` (Neumaier §2.1).

### 3.2 The step certificate (parametric Krawczyk)

The march's chain is p₀ … p_{n−1}, with unit directions t_i. Segment i runs from p_i to p_{i+1}.
Its **pencil of planes** is

  g_i(x, s) = (1 − s)·t_i·(x − p_i) + s·t_{i+1}·(x − p_{i+1}),   s ∈ S = [−σ, 1 + σ], σ = 1/4,

which is the plane across the curve at p_i when s = 0 and at p_{i+1} when s = 1. Let
F(x, s) = (f_a(x), f_b(x), g_i(x, s)) and take X_i to be the hull of p_i, p_{i+1}, grown by ρ_i.
The step is **certified** when

  K(X_i, S) = y − C·F(y, S) + (I − C·J(X_i, S))·(X_i − y)  ⊂  int X_i,

with y = mid X_i, C a float inverse of J(y, mid S), and J(X, S) = [∇f_a(X); ∇f_b(X);
(1 − S)t_i + S t_{i+1}], all of it in intervals.

**Lemma (step).** If K(X, S) ⊂ int X then, for every s ∈ S, F(·, s) has exactly one zero x(s) in
X, and x(·) is continuous. So the zeros of f_a = f_b = 0 in X that lie on a plane of the pencil
with s ∈ S form one arc, A_i = x(S), which is a single piece of a single branch.

The **claim** of segment i is W_i = {x ∈ X_i : g_i(x, s) = 0 for some s ∈ S}. Every zero of the pair
in W_i lies on A_i. Because g_i is linear in s, x ∈ W_i iff g_i(x, −σ) and g_i(x, 1 + σ) differ in
sign (or one is 0). So "box B ⊂ W_i" is two interval evaluations.

**Junctions.** Segment i at s = 1 and segment i + 1 at s = 0 are the same plane Π_{i+1}. Krawczyk on
the fixed-plane system (f_a, f_b, t_{i+1}·(x − p_{i+1})) over Y = X_i ∩ X_{i+1} proves one zero y
there. Since Y ⊂ X_i and Y ⊂ X_{i+1}, uniqueness in each gives x_i(1) = y = x_{i+1}(0). So
A_i ∪ A_{i+1} is connected, and the chain's arcs form **one connected piece of one branch**. A
closed chain also joins n − 1 to 0, so it is one closed loop. And p_i ∈ X_i, so each chain point
is within diam X_i of the branch.

**Refinement where it fails.** If segment i fails, its box is shrunk (ρ_i/2, a few times), then
the segment is split at q = refine(midpoint), with t_q = na × nb at q. The two halves are
certified separately. Only the certificate's nodes change: the chain the caller gets is untouched.
Below a least size (LINEAR), it **refuses** and names X_i.

### 3.3 Completeness (Lipschitz/interval exclusion)

Every zero of the pair in the region must lie in some claim W_i. A worklist of boxes starts from
the region. For each box B:
1. if 0 ∉ f_a(B) or 0 ∉ f_b(B), B is **excluded** (holds no zero);
2. else if B ⊂ W_i for some segment i of some chain, B is **covered**;
3. else if B is smaller than the least size, the certificate **refuses** and names B;
4. else B is split in two across its longest side.

Only the claims whose boxes meet B are carried down to its children, so each box checks a few. This
finds every component, including small loops (Sederberg–Meyers): a loop is a set of zeros, and its
boxes are neither excluded nor covered, so the walk subdivides down to it and refuses. "Every
branch is found" is then a theorem about the walk's output, not a hope about seeds.

### 3.4 No branch twice

Two chains that claim the same zero, or one chain that claims it from two non-adjacent segments,
are a branch traced twice. A loop run round twice, or a forward/back join gone wrong (H13, H14),
looks like this. For every pair of overlapping boxes X_i, X_j that are not adjacent in one chain,
the walk of §3.3 proves W_i ∩ W_j zero-free. It excludes by f_a, f_b, g_i's sign or g_j's sign. If
it can't, it refuses. A sort-and-sweep on x finds the overlapping pairs.

### 3.5 What is certified, and the refusal

**Theorem (trace).** If `certify(a, b, region, chains)` returns Ok, then:
- (i) each chain's points lie, in order, within the tube ∪X_i of one connected arc of
  {f_a = 0} ∩ {f_b = 0}, with no jump between branches;
- (ii) every zero in the region lies on one of those arcs;
- (iii) no zero is claimed by two chains, or twice by one.

Otherwise it returns `Refusal { region: Box3, why }`. `why` is one of `Step` (Krawczyk failed at
the least size: a tangency, a singular point, or a chain off the curve), `Junction`, `Uncovered`
(a zero not on any chain: a missed branch or loop, or a stall), `Twice`, or `OutOfScope`.

Singular points (tangencies, the horn torus's centre, crossing points) make J singular. There
Krawczyk can't succeed, so the certificate refuses at them by construction. That is right for now:
deciding that the gradients are *exactly* parallel is an exact zero test that intervals can't make.
It is the decide-once/exact-reference stage-2 problem (§6.2 of README).

### 3.6 Phase B (designed, not built): singular points first, then pipes

- **Singular and turning points (Grandine–Klein).** Critical points of f_a on {f_b = 0}:
  ∇f_a + μ∇f_b = 0, f_b = 0 (4 × 4, regular at a nondegenerate tangency). Krawczyk isolates each
  in a box Q. Whether f_a(x*) = 0 exactly (a real touch, not a near miss) is decided once, by the
  exact reference (quadric and torus pairs have algebraic coordinates). Given an exact singular
  point in Q, the branches leaving it are counted by certifying zeros of the pair on the six faces
  of Q (2D Krawczyk per face, exclusion elsewhere). Those face points become chain ends. Q's
  interior is claimed by the singular point. Turning points with respect to a sweep direction
  aren't needed, because the pencil follows the tangent.
- **Pipes, offsets, blends.** For f = dist(x, spine) − r, the value is 1-Lipschitz and the gradient
  is (1/(d − κ⁻¹))-Lipschitz within the spine's reach. Enclosures over B come from the centre's
  value, with a certified foot-point error bound, plus those constants. This needs the procedural
  surfaces to return a foot point with an error bound. That is the "FP bridge" the README rates
  high.

### 3.7 Cost, and where it runs

The feature is `certified_trace` (forge-geom; forge-bool and forge-model forward it), off by
default. Off, no code is compiled in, so the speed gate and the bits can't move. On, `trace`
certifies each result for in-scope pairs (both analytic, no spindle torus) after the march. On a
refusal it re-traces once with a quarter step (H1 is the usual culprit), certifies again, and
refuses (`None`) if that fails too. Pairs out of scope pass through unchanged and are counted, so
the report says how much of the scoreboard is certified. The cost is one Krawczyk 3×3 per segment
(~300 flops) plus the exclusion walk, which visits about as many boxes as the octree did.

## 4. Validation

- meet.rs's closed forms: certify their `IntCurve` output (same chain type). They must certify, and
  the march's and the closed form's tubes must claim each other's chains.
- OCCT: the same pairs through `GeomAPI_IntSS`; every OCCT curve point within LINEAR of a
  certified tube.
- Scoreboard curved families, and horn torus and sphere-on-rim: expect named refusals at the
  singular points, never a wrong curve.

## 5. Lean

The step lemma, over ℝⁿ with the sup norm. Let X be a closed box, F : ℝⁿ → ℝⁿ, C a matrix, and
y ∈ X. Suppose that for all u, v ∈ X there is a matrix A with F(u) − F(v) = A(u − v) and A in the
interval matrix 𝐉 (the mean-value hypothesis). Suppose also that ‖I − C·A‖ ≤ q < 1 for all A ∈ 𝐉,
and that the Krawczyk image lies in X. Then F has exactly one zero in X. The proof: the map
x ↦ x − C F(x) sends X into X, and it is a q-contraction, so it has exactly one fixed point (Banach).
C is invertible, because ‖I − CA‖ < 1 makes CA invertible. So fixed points are zeros. The interval
side is that each IEEE op widened by one ulp encloses the real result. That follows from the
IEEE754 model's `round` being within half an ulp. See `verify/krawczyk/`.

## 6. Status and results (first milestone)

Built: §3.1–3.5 for plane, sphere, cylinder, cone (one nappe) and ring/horn torus pairs, in
`kernel/forge-geom/src/certify.rs` behind `certified_trace`. The Lean step lemma is proven
(`verify/krawczyk`). Not built: §3.6 phase B (singular points, procedural surfaces).

Two refinements came up while building it.
- **Interval dependency in the pencil.** `(1 − S)t₀ + S t₁` is ~3 wide and made every Krawczyk test
  fail. Writing it as `t₀ + S(t₁ − t₀)`, and the claim test as `n(s)·x − k(s)` with the box once,
  fixed it.
- **Claims trimmed to their neighbours.** A step's claim may not reach past half of the next step.
  A Krawczyk proof over S holds for every sub-range of S, so the trimmed claims are still proven,
  and only neighbouring claims meet. Without the trim, a short closing segment let claims two apart
  overlap, which falsely flagged Twice.
- **Lipschitz exclusion.** The kernel's own implicit (a 1-Lipschitz distance for every kind in
  scope), evaluated in intervals at the box centre, clears boxes before the polynomial enclosure is
  tried.

**Random pairs** (`examples/certify_sweep`, 300 pairs, seed 1, region ±8). The march certified in
every case but one, a cone–torus pair that ran past the box budget and was refused. meet.rs's
closed forms certified in every case but one, the same cone–torus pair. So on every certified pair,
the march and the closed form are both proven to be the whole intersection, each branch once.

**Native = wasm.** On 150 pairs (seed 3), the chains and every certificate outcome, refusal boxes
included, are bit-identical (`BITS=1`, wasm32-wasip1 under Node).

**Scoreboard** (`reliability`, 985 parts):
- off: 0 worse (978/985 good);
- on: 967/985 good. The 9 worse are all singular configurations, which the certificate refuses by
  design until phase B: `cylinder_cylinder/perpendicular_tangent_inside` ×3,
  `plane_torus/standing_tangent_top_inside` Add/Cut, `cylinder_cone/apex_on_top_rim` Cut/Keep and
  `cone_cone/perpendicular_apex_to_apex` Cut/Keep. The march's answer there may be right, but
  nothing proves it.
- `torus_torus/horn` and `cylinder_sphere/centre_on_rim` stay errors. The horn torus's centre is a
  singular point. The plane–horn-torus pair through the axis refuses there (test
  `a_horn_torus_is_refused_at_its_centre_not_traced_wrong`).

**Cost.** Off: nothing compiled. The callgrind gate shows no REGRESSED line and the bits are
identical. On: the scoreboard takes 5.7 s against 5.2 s. That is little, because analytic pairs
reach `trace` only where meet.rs gives up. Per certified pair, the certificate costs about 40× the
march, almost all of it in the exclusion walk's boxes near the curve (10⁵ boxes for 700 steps).
The next thing to make faster: boxes aligned with each step's chord, instead of axis-aligned hulls.
