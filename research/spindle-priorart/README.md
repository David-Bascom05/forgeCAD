# Spindle and horn tori in booleans: prior art

Research only. Nothing in the kernel uses this yet. It answers "how do the other kernels do it?"
for the boolean guard in `forge_bool::ssi::searched` and the held branch reliab-14 (a sampled net,
research/spindle-booleans on that branch).

## How the other kernels represent the surface

They all pick one sheet by limiting the tube angle. None of them intersects with the whole quartic
and then guesses which sheet a point is on.

- **ACIS**: `torus` (tordef.hxx). The major radius is signed, and negative means the lemon. The
  predicates are `apple()`, `lemon()` and `vortex()` (horn, |major - |minor|| < resabs). For a
  degenerate torus the latitude runs over [-U, U] with U = acos(-major/|minor|). It is singular at
  both ends and not periodic.
  Source: ACIS R10 doc, KERN/38CLT/0009.HTM
  (http://www-isl.ece.arizona.edu/ACIS-docs/HTM/DATA/KERN/KERN/38CLT/0009.HTM).
- **Parasolid**: XT format reference, TORUS (p.58). Uses (a + b cos v). Doughnut is a > b, apple is
  0 < a <= b, lemon is a < 0 with |a| < b. a = b is the "osculating apple", and it has no lemon. A
  loop may be a single vertex, a 0-dimensional hole in a face (LOOP, p.91), which is how an apex is
  held. SPUN_SURF records the degeneracies where the profile crosses the axis (p.72).
- **STEP Part 42**: `degenerate_toroidal_surface (select_outer : BOOLEAN)`, with
  major_radius < minor_radius.
- **OCCT** (Open-Cascade-SAS/OCCT master):
  - `Geom_ToroidalSurface` accepts r > R in its constructor. Its setters refuse it.
  - `GeomAdaptor_SurfaceOfRevolution::GetType` reports a torus only when major > minor, so a
    revolve that touches the axis stays a surface of revolution.
  - `IntAna_QuadQuadGeo` gives up (`NoGeometricSolution`) whenever aRMin >= aRMaj.
  - `IntPatch_Intersection` sends every non-ring torus to `IntPatch_ImpPrmIntersection`. That
    routine marches the other quadric's implicit function over the torus's P(u, v), inside the
    face's uv domain. So the face's v range picks the sheet. This is heuristic: it is marching.
  - `StepToGeom::MakeToroidalSurface` drops `select_outer`.
- **ForgeCAD already uses the ACIS convention.** `Torus::spindle` has a signed major. The used part
  is |v| < `pole_v` = acos(-R/r).

## Literature

- Berberich & Kerber, "Exact arrangements on tori and Dupin cyclides", SPM 2008,
  doi:10.1145/1364901.1364912. They pull the other surface back into (u, v) via tan-half-angles,
  with exact root isolation. It is certified, but for ring cyclides only.
- Reithmann, "Topologically correct intersection curves of tori and natural quadrics", PhD, Mainz
  2009. Its §4.4 gives the self-intersection part of a spindle by a sign test against the apex
  sphere. That is the same test as (1) below.
- Kim, Kim & Oh, "Torus/sphere intersection based on a configuration space approach", GMIP 60(1),
  1998. Exact case analysis, approximate curves.
- Gonzalez-Vega et al., arXiv 2503.08924 (2025): torus against quadric by projection and lifting,
  ring tori only.

No source has a certified boolean on spindle faces. The pieces it needs are below. Each is exact.

## The exact sheet test (`sheet.py`, SymPy)

With S = |q|^2 + R^2 - r^2 (q in the torus frame) and s = R + r cos v (signed distance from the
axis):

1. S = 2 R s on the torus. So a point of the quartic is on the used part (s > 0) exactly when
   **R·S > 0**. That is a quadric sign test, with no projection and no distance to the surface.
2. The quartic factors as (S - 2Rρ)(S + 2Rρ), one factor per sheet.
3. The sheets meet only where s = 0, which is the two apices ρ = 0, z = ±sqrt(r² - R²). So a
   connected intersection branch that misses both apices lies wholly on one sheet, and one
   interior point decides it. Take the point farthest from the axis: there |S| = 2|R|ρ, so the
   sign has margin.
4. In tan(v/2) = t the used part is (R + r) + (R - r) t² > 0, the same formula for apple and lemon.
   This is the interval to keep in a closed form whose host is the torus.
5. Horn (R = r): s = r(1 + cos v) ≥ 0. The other sheet is only the apex.

## What main does today (measured, judge b7d41b0f, lemon and apple of spindle-cuts.py: arc radius 13, centre ∓5)

```
plain lemon / apple                       ok, volumes = the integral of pi rho^2 to 1e-13
apple, half-space above z=3 (perp. plane) ok 12793.9214 (exact 12793.9214)
lemon, half-space above z=3               fail Internal("hole outside every region")
apple, half-space above z=-6              refused ZeroThickness (at a point off the part)
lemon, half-space above z=-6              fail Internal("hole outside every region")
either, half-space x>0 (plane on axis)    fail Internal("face split crossed itself")
either, half-space x>2, any cylinder      refused "surface pair without a closed form"
```

The cause is that `plane_torus` runs before the guard and does not know about the sheets:

- A plane perpendicular to the axis returns radius R - w even when it is negative. That is a
  circle on the other sheet.
- A plane through the axis returns both whole tube circles. Each one runs through both apices, so
  half of each is on the other sheet.

Nothing silent turned up in these cases. The failures are false refusals.

## Recommended method

1. Apply test (1) to every curve and point that a closed form returns for a spindle:
   - `plane_torus`: drop circles of negative radius (perpendicular planes).
   - `meet.rs` Trig1 and Trig2 (the torus is the host): keep only roots with t inside (4).
   - `meet.rs` Poly4 (cylinder or cone host, torus as the other surface): classify each branch by
     R·S at its point farthest from the axis.
2. Make the apices events. If the other surface passes through an apex (its implicit there is
   within tolerance), a branch can change sheet there. That case stays refused for now.
   - It covers the meridian plane: its tube circles run through both apices.
   - Later it needs open curves that end at the apex vertex, the way a curve ends at a cone apex.
3. Pairs that reach the tracer (no closed form) stay refused until the tracer marches in the
   torus's own (u, v) with v clamped to (-pole_v, pole_v). That is OCCT's ImpPrm choice: the
   unused sheet then never appears.
4. With 1 to 3 the sampled net in reliab-14 is not needed as the guard. It can stay as a test
   oracle.

**Cost.** About 100 to 200 lines in forge-geom (`plane_torus`, a sheet filter after
`meet::intersect`, an apex test) and the guard in `ssi::searched` narrowed to "tracer pair, or a
curve through an apex". The tracer in (u, v) is a separate and larger job.

**Proof.**

- (1) to (3) are identities, checked in `sheet.py`.
- Root completeness is the ring-torus argument already in meet.rs. The discriminant events do not
  depend on the sheet.
- Tests:
  - Point membership against the analytic spindle (the `Index::classify` harness that reliab-14
    used, 200000 points per box) for each cut above.
  - Pappus or slice integrals for the coaxial and perpendicular cases. Exact references are in this
    README.
  - The OCCT oracle on the 18 parts reliab-14 let through. OCCT marches, so it is an oracle and not
    a proof.
- A unit test that every curve point emitted for a spindle satisfies R·S > 0 or lies at an apex.
- Open: how many of the 98 + 6 spindle stops in research/build-failures have a curve through an
  apex. Count them with the guard narrowed.

## Horn torus (a whole circle tangent to the axis)

- `revolve.rs` refuses any whole `Circle` that touches the axis ("pinches the solid there").
  Revolved through 360°, the interior near the centre is a ball minus two cusps along the axis.
  The link of the centre is an annulus, so it is a true point pinch (non-manifold). OCCT builds it
  as a surface of revolution and BRepCheck passes it.
- Revolved through less than 360°, the link of the centre is a disk and the solid is manifold (an
  apex, like a cone's). Two of the 20 WHUCAD parts refused this way are partial revolves:
  100002728 (90.35°) and 100003722 (299.29°). These are **false refusals**. The surface is already
  `Torus::spindle` with major = minor (pole_v = π), and only the `Circle` check stops them.
- Of the 18 full revolves:
  - 14 are the first line, so the body starts pinched. 11 of those parts have one line only.
    100002839, 100002909 and 100002232 go on, and a later line may remove the centre point.
  - 4 are tools: 100002564 (add), and 100002569, 100002570 and 100003280 (cut).
  - A full tool can be applied as two 180° half revolves in sequence, each manifold. Whether the
    result pinches depends on where the centre point ends up, which the final validity check
    decides.
- **Policy question for David:** standalone full horn tori are true pinches. Refusing them is
  consistent with "true pinches stay refused". If so, score them on the progress page as policy
  refusals, not as failures.

## Later: recommendation 3 is superseded

Tracer pairs need no march in the torus's (u, v). `Surface::implicit` for a torus is sqrt((rho - R)^2 + z^2) - r with
rho = sqrt(x^2 + y^2) >= 0: in the half plane rho >= 0 its zero set is one arc of one circle, the used sheet, for either
sign of R. The other sheet appears only when that is squared into the quartic. rho is 1-Lipschitz, so the octree's reject
test stands; the gradient is discontinuous only on the axis. So `ssi::searched` refuses only an apex in the region
(`sheet::apex_met`), where a branch may change sheets, and traces the rest. Barrels cut along the axis, across it and by
slots are in interface/forge-agent/tests/spindle_cuts.rs, judged by the definition at 20000 points and a slice integral.
