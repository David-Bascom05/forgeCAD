# Profile-swept solids to the proof standard: design note

Owner of the row "Extrude / revolve / sweep / loft / coil / thread" in research/PROOF-STANDARD.md.
Branch claude/repo-review-mbdrlj-sweep-spec, from claude/repo-review-mbdrlj at 5de0a5f. This note
comes before any build. It says what each feature's result is, exactly, when each must refuse,
and how each of the six levels will be reached. It also lists what the code does today that the
rules forbid.

## 0. One shape for all six features

Every feature here is a **motion of a planar region**. Write it as

    Swept(R, M, I) = { M(s)(q) : q ∈ R, s ∈ I }

- R ⊂ ℝ² is the profile's region, placed in its sketch frame.
- I ⊂ ℝ is a closed interval.
- s ↦ M(s) is a continuous family of maps ℝ³ → ℝ³, rigid for every feature except taper, where
  each section is an offset of R rather than a moved copy.

| feature | M(s) | I |
|---|---|---|
| extrude, distance / symmetric / two-sided | translation s·v (v the unit normal) | [lo, hi] |
| extrude along dir (oblique) | translation s·u, u·n ≠ 0 | [lo, hi] / (u·n) |
| revolve | rotation by s about the axis | [from, from + sweep] |
| sweep, line / arc / planar spline | the frame rule of §3.3 | [0, L] |
| coil | screw: Rot_z(s) ∘ Trans(a·s·x̂ + b·s·ẑ) | [0, 2π·turns] |
| taper | not rigid: the section at height h is Off(R, h·t) | [0, d] |

Each result is regularised the boolean's way: `reg (Swept …)`, using `ForgeExact.reg`, `Pred`
and `Topology` in verify/forge-exact.

The **domain** of every feature is that the motion is *injective on the interior*:

    (s, q) ↦ M(s)(q) is injective on int(I) × int(R)

Together with the frame being defined, this makes the result a solid whose boundary is

    M(I)(∂R) ∪ M(lo)(R) ∪ M(hi)(R)

That is the walls plus the two caps, one wall face per segment. It is exactly the face list the
kernel builds, so the B-rep theorem for each feature has one form:

> **brep_eq**: on the domain, the set the kernel's B-rep denotes equals `reg (Swept R M I)`.
> Its faces are the segment walls (surface `M(I)(seg)`) and the caps `M(lo)(R)`, `M(hi)(R)`.
> Its edges are `M(lo)(seg)`, `M(hi)(seg)` and `M(I)(joint)`.

The **refusal rule** is the same for every feature: refuse exactly when the input is outside the
domain. Each feature below turns injectivity into a decision on the profile and the motion that
is exact and decided once.

### 0.1 Regions

A profile is a list of loops of lines, arcs, circles and splines. Loop 0 is the outer loop, CCW
after normalisation; the rest are holes, CW. Material is on the left of every loop. The region R
is the closure of the points with winding number 1.

For straight-sided profiles, R is a boolean formula over half-planes: a 2D `HForm`, Nef style.
Its prism is the 3D `HForm` `lift R ∧ (z > lo) ∧ (z < hi)`. **So an extrude of a polygon is a
flat solid of the boolean, and `flat_boolean` applies to it unchanged.** That is the reuse the
brief asks for: no new notion of solid. Arcs, circles and revolves land in the quadric forms
(`QForm`, on the exact-curved branch) as set out in §2.3. Splines use the spline spec (§3.4).

The profile domain is checked by `Profile::normalized_touching`, and two of its checks are
sampled:

- `crosses_itself` samples curves at 16 points and refuses only when both the raw outline and the
  cusp-blunted one cross;
- `HoleOutside` samples the hole's points.

For lines and arcs both can be decided exactly: segment–segment intersection, and arc–line and
arc–arc intersection via a quadratic sign. Splines use the spline spec's Bézier hull certificate.
This is shared with every sketch consumer, so I will spec it (`Region.simple`) and **report**
the exact check to the fundamentals / sketch owners rather than change profile.rs alone.

## 1. Extrude

### 1.1 L1

`Prism R v lo hi := { q + s·v : q ∈ R, lo ≤ s ≤ hi }`, with v the unit normal of the sketch plane.

Every extent maps to one prism:

- `Distance d`: [0, d], or [d, 0] when d < 0.
- `Symmetric d`: [−d/2, d/2].
- `TwoSided f b`: [−b, f]. The kernel builds **one** prism, not a union, which matches.
- `Along f b dir`: the prism along u = dir/|dir| over [−b, f] / (u·n).
- `Through τ`: the half-infinite or infinite prism.
- `To plane ⊥`: the prism up to the plane.
- `To surface`: the ray-bounded set of §1.3.
- `ToNext`: the same as `To surface`, with the next face.

Theorems:

- **prism_hform**: for a polygonal region, `Prism` equals the den of the lifted `HForm`.
- **prism_brep**: for an extrude, `brep_eq`, with V = 2n, E = 3n, F = n + 2 per loop as the
  Builder makes them: caps "start" and "end", walls "side"+id. A circle gives one seam.
- **through_finite**: if part ⊆ slab(a, b) and [lo, hi] ⊇ [a, b], then
  `part \ Prism(lo, hi) = part \ Prism(−∞, ∞)`, and the same for ∩. This is what makes the
  kernel's finite "through" range (the reach box ± 1) exact rather than a guess. The theorem
  needs only containment, so the +1 margin is any positive number: the rule is decided once.
- **along_prism**: the ruled loft that `Along` builds between the two translated profiles is
  `Prism R u`. Its walls are planes and cylinders because the two profiles are translates.

Domain: R simple (§0.1); hi − lo > MIN_LENGTH; for `Along`, |u·n| ≥ sin 1°.

### 1.2 Refusals

`TooShort` and `BadSize` are exact comparisons. `ThroughJoins` is structural. `AlongSketch` is an
exact comparison, but the 1° threshold is a modelling choice rather than a validity condition:
any u·n ≠ 0 gives a valid prism. I will state it as a stated precondition, not a theorem.

### 1.3 Up to a surface, up to next

L1:

    UpTo(R, v, S, off) = { q + s·v : q ∈ R, 0 ≤ s ≤ τ(q) + off }

Here τ(q) is the first s > 0 with q + s·v ∈ S.

The domain is that every ray from int R meets S once, transversally, ahead of the sketch. That
is the `NotAhead` / `NeverHits` / `NotCovered` family. The kernel builds `Prism ∩ K` (or `\ K`),
where K is a solid bounded by S: a box, ball, drum or spun cone (upto.rs).

- **upto_eq**: on the domain, `Prism(0, max τ + 1) ∩ K = UpTo`, given that K is the side of S
  that the rays enter.

**ToNext is sampled today.** The rays start at one point 1e-3 inside the middle of each outer
segment, and `Ambiguous` is raised on the first ray that disagrees. The L1 pick is

> S = the surface carrying the first face of the part met by **every** ray from int R.

It is decided exactly from the earlier tools' analytic shapes as follows. Take the earliest
crossing over the swept prism, and check that the prism's intersection with the part below S is
empty. That is one boolean, which `verify_next` already almost does. Making that check decide
the pick, instead of only checking a sampled one, is a fix for L3. I will spec it here and make
the change only after the master agrees, because it touches upto.rs, which other sessions edit.

## 2. Taper

This section is the spec for the known bug.

### 2.1 L1

The profile has lines and arcs only (splines are refused with `Curve`, which stays). With slope
t = tan|angle| and σ = ±1 for in or out, the section at height h ∈ [0, d] is the **mitred offset**
P(h) of R by σ·h·t:

- every line carrier moves σ·h·t to the material side;
- every arc's radius changes by ∓σ·h·t;
- each joint is the meet of its neighbouring carriers.

This is the offset that taper.rs's `offset()` computes. It is the same family as the kernel's
drafted planes and cones, and it is **not** the erosion by a disc, which rounds reflex corners.

    Taper(R, σt, d) = ⋃_{h ∈ [0, d]} P(h) × {h}

P(h) is defined by the carriers alone only while the combinatorics hold. An **event** is the
first h > 0 at which one of these happens:

- **(E) edge event**: a segment's length or an arc's radius reaches 0;
- **(S) split event**: a vertex of P(h) reaches a non-adjacent segment of the same loop;
- **(C) collision**: two loops touch: a hole and the outer loop, or two holes;
- **(J) joint lost**: two neighbouring carriers stop meeting (arc–line, disc < 0).

Each is an exact algebraic condition in h. For a polygon every vertex moves linearly in h and
every carrier is translated linearly in h, so E is linear and S and C are linear sign changes of
orient2d with coefficients that are linear in h. Arcs make them quadratic.

### 2.2 The decided rule

This is the answer the taper fix is due to give.

1. **No event in (0, d]**: the result is the single drafted solid. `brep_eq` holds with the
   kernel's faces: planes turned about their base line, and cones from cylinders. This is the
   draft path.
2. **First event at h₁ ≤ d, of type E only, and the region keeps positive area after it**: the
   result is the stack of slabs [0, h₁], [h₁, h₂], …, each the event-free taper of the
   reduced profile, where a segment that has gone is dropped. This is today's slab path. It
   builds, and the L1 above covers it piecewise. It stays.
3. **The outer region collapses at h* ≤ d**: its area reaches 0, as with the 80° hexagon pit
   at h* = √3 / tan 80° ≈ 0.305. The result is **refused** with a new typed error
   `TaperError::Collapses { at: h* }`, reported at the feature. Section 3 of the brief asks for
   this ("a taper that collapses" must refuse), and refusing is the only answer that is not a
   different solid from the one asked for: a pit 0.5 deep cannot be built 0.305 deep.
   *Decision for the master*: the slab path today returns a roof-topped solid when the outer
   loop closes. That is a valid set, just shallower. Refusing it is the choice I propose; the
   alternative is to keep the roof. The scoreboard says which repo parts this changes (§7).
4. **A hole loop closes before d**: this is an E event inside the material, and the region
   keeps positive area. The loop is dropped, the hole ends, and the result builds as in case 2.
5. **Event S, C or J at h ≤ d**: **refused** with `TaperError::WallsCross { at: h }`.
   Nothing detects these today (no global check of the offset loops). After such an event, the
   carriers' polygon is no longer the boundary of any offset region, so every answer the kernel
   could give there is wrong.

For the hexagon pit in taper_collapses.fg, rule 3 gives `Collapses { at: 0.3054… }`. The fault
is at taper.rs:416–417: a successful `draft()` is returned without asking whether an event comes
before d. The fix computes the first event exactly. It replaces `next_event`, which takes 256
uniform samples and 60 bisections, assumes wipe-out is monotone (so a transient J can be missed)
and is tolerance-based. Then it takes branch 1, 2, 3, 4 or 5 above. **This spec goes to the
session that owns the fix (session_01Q19Lzxa8uxgdmHGsahv1hg), through the master.** I will not
edit taper.rs unless the master moves it to me.

The exact first event (L2) is the least positive root, over all pairs, of the conditions above.
Each root is rational in t and the carriers' coefficients, with one √ per arc–line joint, so the
reference computes it in `Alg.Num`. The kernel computes it in f64 with the L5 predicates
(orient2d at moving points, of degree 1 in h).

### 2.3 Slope

The user writes the angle and the kernel rounds it once to t = tan(angle) by `dmath`. The spec
is parametrised by t. The reference takes the kernel's t, an exact dyadic. That one rounding is
the stated boundary, the same convention as every f64 input.

## 3. Revolve, sweep, coil, thread

### 3.1 Revolve

L1:

    Rev(R, φ₀, φ) = { Rot_axis(θ)(q) : q ∈ R, θ ∈ [φ₀, φ₀ + φ] }

A full turn (φ = 2π) is the fillet spec's `Fillet.Rev T` with T the meridian set R in (ρ, z).
I will reuse it, not redefine it.

**Domain**: R lies in the closed half-plane ρ ≥ 0, with 0 < φ ≤ 2π. On a full turn, R may touch
the axis only along whole segments (they make no face) or at isolated points (axis vertices).

**Refusal** (`CrossesAxis`): min over R of ρ < 0. Today this is decided on 32 samples per arc or
circle, with a catch-all in the surface constructors. The exact rule is:

- a line: its two ends;
- an arc: its ends, plus the angle where the arc's radius points along −perp, if that angle lies
  in the arc's sweep;
- a spline: the polynomial sign certificate that `Spun::new` already uses.

**This is a kernel change I own**: a small, exact `min_r` in revolve.rs and the coil's `wind`
replaces the samples for arcs and circles. It can only change parts where an arc dips across the
axis between samples, and those build wrong or refuse with `Unsupported` today.

B-rep (`rev_brep`), one theorem per segment kind:

- a line off the axis sweeps a plane annulus (Δh = 0), a cylinder (Δr = 0) or a cone frustum;
- an arc sweeps a torus piece, or a sphere piece when its centre is on the axis;
- each is stated as an implicit equation over the ordered field with `Sqrt`.

The kernel's choice among plane, cylinder and cone uses |Δ| ≤ LINEAR. That is a snap, not exact
geometry. The spec states it as the boolean's snap does: an input within LINEAR is first moved
onto the exact surface, so the result is exact for the snapped profile, and Snap.lean's
`tolEq` framework is reused.

The torus → spindle → refuse chain is not a retry. The three constructors cover disjoint cases
of the sign of major − minor, so it is one decided rule written as a cascade, and I will state
and prove it as a case split.

The post-hoc pinch refusal ("refused only if `forge_topo::check` reports problems") is a
fallback. Its exact replacement is to refuse a full turn when two face-making segments share an
axis vertex and the call is not a cutter. Both of those can be tested before building.

L2: the revolve of a polygon is a **QForm**. To see why, take ρ = √(x² + y²). Then a·ρ > c − b·z
is equivalent to a boolean of `a > 0`, `c − b·z < 0` and `a²(x² + y²) ≷ (c − b·z)²`: planes and
quadrics. A partial turn adds the two half-spaces through the axis (∧ when φ ≤ π, ∨ when
φ > π). So the curved reference's `quad_boolean` decides revolves of polygons exactly, and the
differential compares the kernel's B-rep to it with one sdiff that must come out empty. An arc
off the axis gives a quartic (a torus), so arcs are outside Quad. Those are judged by face and
edge combinatorics plus pointwise membership with `Alg.Num`.

### 3.2 Coil and thread

L1 is the helical sweep `Swept(R, Screw(a, b), [0, 2π·turns])` in the frame `rf`. A section
point (r, h) at u goes to ((r + a·u) cos u, (r + a·u) sin u, h + b·u). Here b = ±pitch/2π and
a = (pitch/2π)·tan(taper).

**Exact injectivity theorem (coil_injective)**. Two section points give the same point of space
only when θ' = θ + 2πk, r − r' = 2πka and h − h' = 2πkb. So, for R in ρ > 0 along the whole
motion (r + a·u > 0):

    the coil is injective  ⇔  ∀ k ∈ {1, …, ⌊turns⌋}:  int R ∩ int (R + k·(2πa, 2πb)) = ∅

That is an exact test of the profile against its translates. The translates are polygons and
arcs, so the test uses orient2d plus arc signs. It is proved for the motion over the reals, with
`Fillet.V` and `Sqrt`.

The kernel's `TurnsTouch` refuses when pitch ≤ h_extent + LINEAR. That condition is
**sufficient but not necessary**: for a convex profile with a = 0 it equals the exact test (up
to LINEAR), but it refuses valid slanted or non-convex profiles. Swapping in the exact test can
only make parts build that refuse today, which counts as 0 worse on the scoreboard. I will
report it as an option rather than ship it in the first pass. Swapping it also removes the need
for a sampled self-intersection check on coils, which the kernel does not run anyway.

The thread is the coil Cut of the 60° groove (ISO form, depth 5√3/16·P), plus the run-out coil
and the chamfer revolve. The groove's height along the axis is below P by construction, and the
groove is convex, so `coil_injective` holds for every valid thread size (major > pitch > 0).
That is a lemma, not a check. The run-out coil's taper keeps r > 0 when drift < rj, which needs
a short proof of its own.

L2: in the screw chart (r, h, u) the coil *is* a prism, R × [0, 2π·turns], and the kernel's
Screw faces are defined in that same chart. So the reference checks the B-rep combinatorially in
the chart (§1.1's prism_brep, re-used) and the injectivity rule over ℚ or `Alg.Num`. The
helicoid is transcendental in xyz, so there is no xyz-space QForm for it; that is stated as the
reason the chart is the exact frame.

### 3.3 Sweep

Path: G1 lines, arcs and splines. The **frame rule**, stated once:

- along a line, M is a translation;
- along an arc, a rotation about the arc's axis;
- along a planar spline, the rotation about the plane's normal that takes T₀ to T(s), so no
  twist (swept.rs:236–258);
- along a spatial spline, the rotation-minimising frame, the solution of R′ = [T × T′ / |T|²] R
  with R(0) = I.

Two silent fallbacks break the rule:

- `carry` falls back to plain translation when `spine_of` fails (sweep.rs:130–139). The
  rule-conforming fix is a typed refusal, `SweepError::Frame { i }`.
- the "straight but not planar" spline gets planar treatment with an arbitrary normal. For a
  straight piece every normal gives the same rotation (T is constant), so this one is
  harmless. I will state it as a lemma.

**Domain**: injectivity of (s, q) ↦ M(s)(q). For a line/arc path it splits into two parts:

- **(local)** every bend's profile stays inside its radius. `TooTight` does this, exactly for
  lines and arcs (`rh_section(..).extent()`).
- **(global)** the pieces, prisms and revolve sectors, pairwise have regularised intersections
  that are empty, except adjacent pieces, which share only their common section.

The global part is a boolean question about flat and quadric pieces, so it is **decidable
exactly by the boolean itself**. Today `selfx` instead:

- samples the intersection curves;
- trusts the marcher;
- skips pairs that share a vertex;
- returns the first pair it finds, so the result depends on order.

The L3 replacement for line/arc paths is: refuse iff some non-adjacent pair of pieces'
analytic `Shape`s (swept.rs already builds them) intersects with positive volume, or an
adjacent pair overlaps beyond its common section.

Spatial-spline sweeps have no closed form (the RMF is an ODE; the kernel integrates it with
Magnus steps to 1e-13). They are **outside the exact domain**, and are stated as such: L1 holds
for the ODE solution, and the result is numerically defined, as fillets are. Their level-3 claim
is a certified-interval one, with the trace session's Krawczyk step as the tool. Planar-spline
sweeps have an exact frame (cos = T₀·T, sin = (T₀ × T)·b), so their L1 and L2 are exact, using
the spline spec's `Nurbs.eval`.

`PathEnds::To` is a boolean with a half-space, so `upto_eq`'s pattern applies.

### 3.4 Loft (kept as is: L1 and the differential only)

L1. The walls are:

- ruled: S(u, s) = (1 − s)·C₀(u) + s·C₁(u), segments matched by fraction;
- smooth: research/loft-ssi's Skinned surface, `Σ w_k(s) C_k(u) + …`, with LoftMeet's `splineW`
  / `polyW_eq` as the definition.

The intermediate sections are not planar, so a loft is **not** a Swept of one region. Its L1 is
the bounded component of the complement of the closed surface W = walls ∪ caps, on the domain
"W is embedded": no wall self-crossing, no two walls meeting off their shared rails, and a
normal that does not vanish (`Twisted`).

Loft L1 goes in a Mathlib project next to LoftMeet: research/loft-ssi/spec, which imports
LoftMeet's Hull. It defines the surface and the solid and states the domain. It also proves the
ruled case: when every wall is a plane or a cone between parallel or coaxial sections, the
solid equals the convex-combination set ⋃ₛ ((1 − s)R₀ + sR₁) shifted. That covers every
`Along` extrude (§1.1).

The differential test compares the kernel's loft walls with the L1 surface at exact rational
(u, s), and its refusals with an exact embeddedness verdict:

- for arc/line walls, the loft-ssi F = E1² − 4E0E2 certificate (derive/proofs) decides whether
  two sections meet;
- for the rest, a subdivision certificate.

**What a fix would need** goes in the report, without changing geometry:

- (a) `selfx` is sampled and order-dependent (as in §3.3). The exact test would be pairwise
  embeddedness by LoftMeet's subdivision plus its `apart_sound` and `misses_sound`, which is
  already proven.
- (b) `Twisted` is decided by `Ruled::new` on a sampled normal check.
- (c) The `align` tie-break is a decided rule; I will state it as one.

## 4. Levels, per feature, and what I will claim

| | L1 | L2 | L3 | L4 | L5 | L6 |
|---|---|---|---|---|---|---|
| extrude (polygon) | Prism, prism_hform, through_finite | the HForm, through forge-exact | prism_brep for every simple polygon | Aeneas on an extracted `prism_incidence` core if Charon is available; otherwise interim differential | none new: exact comparisons and orient2d | `sweep-diff` on the corpus and random polygons |
| extrude (arcs), revolve | Prism / Rev | QForm (exact-curved) | rev_brep, CrossesAxis exact | interim | `min_r` (new, small) | same |
| taper | Taper, events | first event in Alg.Num | the decided rule (§2.2) | interim | moving orient2d (degree 1 in h) | same |
| coil, thread | helical Swept, coil_injective | chart prism + ℚ test | thread lemma | interim | orient2d | same |
| sweep line/arc | Swept, frame rule | Quad pieces | the global test (§3.3) | interim | — | same |
| sweep spline | ODE frame | planar: exact; spatial: outside | certified (later) | — | — | sampled |
| loft | spec + domain (Mathlib) | — (kept) | — (kept) | — | — | differential, report |

### 4.1 The differential (L6)

A new verify/forge-exact/sweep harness: the `sweep-diff` binary plus `ForgeExact/Swept/Ref.lean`.

- Input: the feature's *definition*: profile loops with exact dyadics, the motion, the interval.
- The reference builds the L1 solid itself: an HForm for a polygonal extrude or taper, a QForm
  for a revolve. It then runs **its own boolean** to compute `reg(kernel tool △ definition)`.
  The verdict must be "empty", and the volumes and the counts (F/E/V) are compared too.
- Corpus:
  - every `.fg` in the repo (304, with 27 tapers and 95 that use revolve, sweep, loft, coil or
    thread), every tool of every feature;
  - random profiles: polygons with holes, arcs, near-axis revolves, tapers near their event
    heights, coil pitches near the injectivity bound.
- Refusals are judged too: the kernel refuses iff the reference says the input is outside the
  domain.
- The digest is committed (sweep-diff.digest) and checked with `git diff --exit-code` in the
  forge-exact workflow, the way parse_diff.digest is.

### 4.2 Lean layout

`verify/forge-exact/ForgeExact/Swept/{Region, Prism, Taper, Rev, Screw, Brep, Ref, Axioms}.lean`

- core Lean only, like the rest of the package;
- generic over the same ordered-field context as `Alg.Model` / `Fillet.V`, with the ℚ³
  instances for the reference;
- its own Swept/Axioms.lean with `#standard_axioms`, so no file is shared with the other branches;
- the existing forge-exact CI job builds it.

Loft L1 is Mathlib, in research/loft-ssi/spec, built by lean.yml's loft-ssi job.

Branches I depend on:

- exact-curved: `Alg`, `Quad`;
- fillet-spec: `Fillet.V`, `Rev`, `Sqrt`;
- spline-spec: `Nurbs`, `Defs` (core-only).

I will merge them into my branch only for what I import, and say so, so that the master's merge
order sees it. If the master prefers, I can vendor nothing and wait on their merge instead.

## 5. What the code does today against the rules (found while reading)

| where | what | rule | proposal |
|---|---|---|---|
| taper.rs:416 | draft accepted with no event check → invalid body | wrong result | §2.2 (to the taper session) |
| taper.rs `next_event` | 256 samples + 60 bisections, assumes monotone | sampled | exact first event |
| taper.rs | no S/C/J check | wrong result | `WallsCross` |
| taper.rs `tops.find` | first-match merge at 2·GONE | order-dependent | follows from exact events |
| taper.rs | lo = 0 ⇒ h = 0 ⇒ `expect("the walls rise")` | panic | an event at 0 is an input error (refuse) |
| revolve.rs, sweep `wind` | axis side from 32 samples on arcs | sampled | exact `min_r` (mine) |
| revolve.rs:545 | pinch refused only if `check` fails afterwards | fallback | decide before building |
| revolve.rs:34 vs 122 | comment says cones get two halves, code gives one | doc | fix comment |
| sweep.rs `carry` | silent fallback to translation | fallback | `SweepError::Frame` |
| sweep.rs `turned` | Cone/Torus failure → Screw a=b=0 surface | fallback | state as a decided case split (the circle reaches the axis), or refuse |
| selfx.rs | sampled, trusts the marcher, first pair | sampled, order | exact piecewise test (line/arc), certified (spline) |
| coil `TurnsTouch` | sufficient, not necessary | over-refusal | coil_injective (optional) |
| upto ToNext | rays sampled at segment mid-points | sampled | exact pick through the prism boolean |
| extent Through | +1.0 margin | arbitrary | through_finite makes any margin exact |
| profile.rs | crosses_itself, HoleOutside sampled | sampled | report to the sketch owner |

## 6. Order of work, with a milestone report after each

1. **M1, extrude L1 + L6 (polygon)**: Region, Prism, prism_hform, through_finite, Swept/Axioms; the
   sweep-diff harness on polygonal extrudes; corpus and random digest.
2. **M2, taper spec**: Taper.lean (events, the decided rule, the hexagon example proved), the
   exact first-event reference, and the differential including refusals. The spec text goes to
   the master for the taper session at M2's start, before the Lean is done, because it is
   blocking them.
3. **M3, revolve**: Rev reuse, CrossesAxis exact (kernel), QForm reference, differential.
4. **M4, coil/thread**: coil_injective, the thread lemma, the chart reference.
5. **M5, sweep line/arc**: frame rule, the exact global test spec; the kernel change only after the
   master agrees.
6. **M6, loft L1 + differential + fix report.**

## 7. Gates, at every milestone

- reliability scoreboard 0 worse;
- FUZZ_SEEDS=150 150/150;
- the exact callgrind gate (GATE_COUNTER=callgrind tools/speed/gate.sh) against a baseline built
  at 5de0a5f, with user-facing and synthetic lines ≤ base;
- native = wasm bit-identical (tools/test-wasm.sh);
- no crates in kernel/;
- Lean: `#standard_axioms`, no sorry, built in CI.

M1 to M4 add no kernel code except `min_r`, which runs only on arcs, does fewer operations than
32 samples, and so should be ≤ base. The callgrind lines are reported each time.

The changed answers I expect:

- with the §2.2 rule, any repo part whose tapered outer loop closes before its depth changes from
  "roof" to `Collapses`. I will list every such part with the scoreboard before anything ships;
- exact `min_r` changes only parts where an arc dips across the axis between samples.

No .fg language change is planned. `Collapses` and `WallsCross` are new error variants, not
syntax.
