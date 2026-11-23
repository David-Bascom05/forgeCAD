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

**OCCT** (`tools/occt/certify_check.py` on 119 certified pairs, `certify_sweep 120 7` with
`DUMP=`). It samples `GeomAPI_IntSS`'s lines, keeping only the kernel's cone nappe, since
OpenCascade's cone runs on through the apex.
- Every OCCT point inside the region lies within 5.8e-2 of our chains' chords. That is chord sag at
  steps of about 0.2; the evaluated curve is exact by its Newton step.
- For 116 pairs, every point of ours lies within a step of OCCT's samples.
- The other 3 are cone–cone pairs, with our points up to 1.3 from any OCCT sample. Those points lie
  on both kernel cones to within 1e-8, and the certificate proves them a branch. So OCCT's walking
  lines leave those stretches out (or sample them too sparsely). Our side is proven.

**Feature-on test differences** (all refusals at singular contacts, none wrong):
- `forge-bool/tests/booleans.rs` `a_hole_on_a_rounds_centre_circle_at_any_angle`: the hole is
  tangent to the round;
- `forge-model/tests/no_feature_dropped.rs` `a_result_that_is_not_closed_is_refused`: a refusal in
  place of NotClosed;
- `forge-lang/tests/drawings.rs`: rack18u `fans/guard_120.fg`'s swept wire (a cylinder) meets its
  revolved loop (a torus of the same tube radius) tangentially along the joint circle. This is the
  pipe–torus crease case of research/fillets/general-method.md, and it needs phase B.

## 7. Phase B, refined by the failing cases

The cases that feature-on refuses fall into three kinds. §3.6 covered only the first.

| kind | cases | what the certificate needs |
|---|---|---|
| **isolated singular point** (the surfaces touch at a point, and the branches cross or touch there) | `cylinder_cylinder/perpendicular_tangent_inside`, the hole tangent to a round, the horn torus's centre | the point decided exactly once, then the branches leaving it counted on the faces of a small box Q about it |
| **surface singular point** (the curve runs through a cone's apex) | `cylinder_cone/apex_on_top_rim`, `cone_cone/perpendicular_apex_to_apex` | the same, with Q about the apex (the apex is exact data, so there is nothing to decide) |
| **tangential contact curve** (the surfaces share a curve along which they are tangent) | `plane_torus/standing_tangent_top_inside` (the top circle), rack18u guard_120 (the wire's cylinder against the loop's torus, along the joint circle) | the shared curve decided exactly once as a closed-form circle on both surfaces, a tube about it claimed by that curve, and the rest of the pair certified outside the tube |

**Isolated points.** Krawczyk on the 4 × 4 Lagrange system (∇f_a + μ∇f_b = 0, f_b = 0) isolates a
candidate x* in a box Q. Whether f_a(x*) = 0 exactly (a touch, not a near miss) is a sign that
intervals can't settle. The exact reference decides it: for quadric and torus pairs, x* is
algebraic. Given a touch, the branches through x* are the zeros of the pair on ∂Q. Each face is a
2D problem: Krawczyk for each crossing point, exclusion for the rest of the face. The face
crossings become ends of the chains outside Q, and Q's interior is claimed by the singular point.
That gives a certified local picture: k arcs meeting at x*.

**Tangential curves.** For quadric and torus pairs, the shared tangent circles come from
symmetric positions, so they can be enumerated in closed form (meet.rs's coaxial profiles already
find some). Inside a tube T about the circle, the claim is that every zero of the pair in T lies on
the circle. That needs a second-order argument: f_b restricted to the normal disc of f_a vanishes
to second order on the circle, with a definite Hessian sign (an isolated touch), certified by
intervals over T. Outside T, the existing certificate runs. Where the Hessian is indefinite, the
surfaces cross as well as touch, and further branches leave the circle; those are found on ∂T like
the face crossings above.

**Order of work.** First isolated points (they unblock 5 of the 9 scoreboard parts and the
round-hole test). Then tangential circles (guard_120, the tangent plane–torus). Then cost.

## 8. Phase B, step 1: isolated singular sites (built)

`certify` first finds the sites the chains pass:
- each surface's own singular point (a cone's apex, a horn torus's centre) wherever the other
  surface touches it;
- each touch point: a critical point of f_a on f_b = 0, found by Newton on the Lagrange system
  ∇f_a + μ∇f_b = 0, f_b = 0 from chain points where the normals are nearly parallel, then certified
  unique in a box by 4-dimensional Krawczyk (exact interval Hessians for every form).

Whether a site is a touch is **decided once by the kernel's contact rule**: the other surface's
distance there, as an interval, is within LINEAR. When the enclosure straddles the rule's edge, the
certificate refuses.

The site claims a cube Q about its point. A step with both ends in Q is not proven. A failed step
whose box holds the site is cut there, and its parts are refined towards the site, so proven steps
run up to Q's face. The coverage and no-twice walks count Q as claimed. Open chains that stop short
of a site are run on into it. Q's half side is tried at 3.2e-4, 2e-3, 2e-2 and 0.1, least first.
Near a touch the surfaces stay within κd² of each other, so the exclusion walk can only afford Q
from about 3e-3/Δκ.

Steps wholly outside the region (grown by LINEAR) are not proven either. They claim nothing the
region needs, and coverage catches any part of their arc that dips in.

Result, feature on:
- scoreboard: **0 worse** (978/985, the same as off); the 9 parts that were worse now certify;
- the three tests that failed now pass: the hole tangent to the round, no_feature_dropped, and
  rack18u guard_120 (same volume as off, but 5.6 s against 0.16 s);
- one new feature-on failure: forge-lang `kernel_history_repros::slow_histories_finish`. A swept
  pipe (a cylinder) meets a bend (a torus) of the same 0.76 radius, tangent along the joint circle.
  That is a tangential contact curve, not an isolated point, and it ran the walk out of budget.

**Next: walk in a surface's own domain.** Near a tangential contact the gap grows like t² (t the
offset from the shared curve). 3D box exclusion then needs boxes of size κt², about
32πr/(3κ²w³) of them outside a tube of radius w (6e5 for the pipe joint at w = 0.1). In the (u, v)
domain of one surface, the gap function f_b∘A has a mean-value enclosure of width about 2ct·δ
against its value ct², so a box of side t/2 is excluded: about 8πr/w boxes. That is 10³ instead of
10⁶, and it is also the main cost the certificate has today. Then the shared circles of pipe joints
(cylinder–torus, sphere–torus, torus–torus with equal tube radii), each decided once, claim a thin
tube.

## 9. The completeness walk in a surface's own domain (built)

Coverage now walks (u, v) boxes of one of the two surfaces (the host: plane, cylinder, cone,
sphere or torus, cheapest first) instead of 3D boxes. P(u, v) = o + B·l(u, v), with l the local
coordinates each form is written in, and B an interval enclosure of the exact inverse of the
frame's rows: its transpose, widened by ‖Xᵀ‖ρ/(1 − ρ), where ρ bounds ‖I − M Xᵀ‖ (the Neumann
series). So the image of the domain covers the form's zero set exactly, not just the frame's
rounding of it. A (u, v) box is done when:
- its image misses the region;
- the other surface's Lipschitz or nappe test clears the image box;
- g = f_b ∘ P excludes 0, by g(c) + g_u(D)(u − u_c) + g_v(D)(v − v_c) with g_u = ∇f_b(P(D))·P_u(D),
  which is tight near a tangency;
- or its image lies in a step's claim or a site.

The twice walk stays 3D: it runs over the small meets of two step boxes.

Feature on, after this:
- **every test passes**: forge-geom 138, forge-bool 42 (`ssi::a_stall_counts_only_on_the_faces`
  now expects the certified trace through the touch), forge-model 204, forge-lang 814, including
  the pipe-bend joint of `slow_histories_finish`;
- scoreboard: **0 worse** (978/985);
- rack18u guard_120: 674 ms (5566 before, 155 off). Its volume differs in the 12th digit,
  because the first march was refused and the quarter-step march certified;
- the 300-pair sweep: the certificate takes 2.2 s (4.2 before).

## 10. Cost, measured with the exact gate

The callgrind gate ran on the same source with the feature off and on (on: forge-geom's default
set locally, not committed). The parts' bits are **identical** on and off: every gate part's
traced pairs certify, and none needs the quarter-step march.

The cost on is concentrated in the curved bench parts. The fair lines are the cold builds (`app`,
`edit`, `cli`). The `seq`/`nary1` regen lines profit from the memo, since the build before them on
the same thread already certified the pairs. guard's regen is even 73% cheaper than off, because
the off build marches every repeat.

| part | app off | app on | × |
|---|---|---|---|
| guard | 56.9M | 387.9M | 6.8 |
| quadarm | 50.3M | 189.1M | 3.8 |
| frame | 440.1M | 1196.5M | 2.7 |
| manifold | 249.0M | 569.0M | 2.3 |

Parts with no traced analytic pair are unchanged.

Where the cost goes: about 5M instructions per certified pair, against about 0.6M for its march.
Nearly all of it is the coverage walk's boxes near the curve (1.5k–4k per pair). Each costs a
patch of trig and an interval matrix, two evaluations of the other polynomial, and the claim
tests. The fixes made so far are each box's patch once, a two-product float multiply, B as floats
plus one error, the cheap Lipschitz test first, the per-pair memo, and Interval::sin_cos. They took
guard from 610M to 388M. Inlining forge-math's interval operations helped as well, but it moved
feature-off counts by ±1%, so it was reverted.

**What would make it default-viable: certify in the host's domain.** For an analytic pair, the
curve is the zero set of g(u, v) = f_b(P(u, v)), a curve in the plane. The march's points map to
(u, v). The steps become 2 × 2 Krawczyk on pencils of lines, and the claims become thin 2D
parallelograms that domain boxes fit without the 3D detour. The chain's 3D points are then shown
to lie in P of the 2D tube. That should cut the per-pair cost several-fold, toward the march's own.

### 10.1 Long steps

A proven step may now span up to 16 of the march's nodes: the longest that Krawczyk proves at once,
with each node it passes lying in its claim. In a claim every zero is the step's arc, so the chain
still provably runs on one branch. The claims are fewer and wider, so the coverage walk fits them
with fewer boxes. Cold-build cost, feature on (off in brackets):
- guard: 354M (57M), was 388M;
- quadarm: 117M (50M), was 189M;
- manifold: 356M (249M), was 569M;
- frame: 1176M (440M), was 1196M.

The 300-pair sweep's certificate time is 1.4 s (4.2 s at first), and the last budget refusal is
gone: all 300 certify. The scoreboard is 0 worse on, and every test passes on.

**Assessment.** What is left is the completeness walk itself: about 16k instructions per box,
15k boxes for guard's 32 pairs, so ~240M against the march's 39M. Box walks can't be tuned down by
the 6–10× still needed. Default-viable needs a cheaper completeness argument. For quadric and torus
pairs, meet.rs's host form already gives one: F(u, s) is a polynomial (or trigonometric
polynomial) of degree ≤ 4 in s, so a certified count of its real roots in s over each u-interval
counts the branches. That count can come from Sturm or Descartes sign variations with interval
coefficients, refined only where the count is unsure. It is a 1D walk in u with a few hundred
intervals, not a 2D or 3D box walk. Compared with the march's chains, it proves completeness, and
the Krawczyk steps stay as they are for the branch-following part.

### 10.2 Tried and reverted: the polynomial in v per u-interval

For the line hosts (plane, cylinder, cone), P(u, v) = a(u) + v·d(u). So the other form on the host
is a polynomial in v of degree ≤ 4, whose coefficients can be enclosed once per u-interval U. The
walk then tested each V by the centred form of that polynomial, with no trig and no 3D evaluation
per v-split. It was correct (the polynomials match the forms to 1e-9 at sampled points), but it was
far slower: guard took 505M instructions, against 354M with the 2D mean-value walk, and frame took
72G against 1.2G.

The coefficients' enclosure over U carries U's whole width, with no mean-value term in u. Near the
curve it excludes almost nothing until U is tiny, and every u-split pays the trig again. A 1D root
count needs u-derivatives of the coefficients (a mean-value form in u for each c_k) or exact
coefficient polynomials in (cos u, sin u) with a proper range bound, e.g. Bernstein on the
half-angle tangent. Interval coefficients evaluated at a whole u-interval are not enough.

### 10.3 The completeness walk in Bernstein form (built)

Each host surface becomes one or more half-angle charts: t = tan(u/2), and for the sphere and torus
also s = tan(v/2), each over [-1, 1] and its half turn. The plane is one chart in (u, v). On a chart,
the host's points are o + B·l(t, s)/w with l and w polynomials. The other form times w^deg is then a
polynomial F(t, s), bidegree up to (8, 8) for torus on torus, with the same zeros. Its coefficients
are interval enclosures built from the exact stored data, and B is the interval inverse of §9.

The walk converts F to Bernstein form on the chart once, then halves boxes by de Casteljau (exact
halving of outward-rounded sums). A box is gone when:
- all its Bernstein coefficients are of one strict sign (the convex hull property: no zero);
- its rational image misses the region;
- or the other surface's Lipschitz or nappe test clears it.
It is done when its image is in a step's claim or a site.

No trig and no 3D evaluation of the forms happens per box, and coefficients live in fixed arrays.
The chart polynomials are tested against the forms at sample points, and the Bernstein hull against
sampled values (`bernstein_charts_match_their_forms`).

Cold build, feature on (off in brackets), the 2D mean-value walk → Bernstein:
- guard: 354M → 200M (57M)
- quadarm: 117M → 92M (50M)
- manifold: 356M → 316M (249M)
- frame: 1176M → 796M (440M)

The 300-pair sweep certifies all 300 (certificate 1.17 s), and native = wasm on 150 pairs. Feature
on: the scoreboard is 0 worse, and every forge-geom, forge-bool, forge-model and forge-lang test
passes. guard's remaining cost is now about equal parts walk, Krawczyk steps (43M) and the march
itself (39M).
