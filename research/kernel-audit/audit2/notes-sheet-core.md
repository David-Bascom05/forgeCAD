# forge-sheet core audit (bends, flanges, joints, reliefs, corners, hems, contour, forms, checks, region)

Read-only. Repo /home/user/forgeCAD, kernel/forge-sheet. Line numbers are for the working tree as found.
Nothing under kernel/ or interface/ was modified. A scratch probe crate (outside the repo) was built
against the crate: scratchpad/sheetprobe/src/main.rs.

## 0. Short answer

The core is mostly one definition, not case patches. There is one topology per sheet: a 2D region per strip
(walls and bend zones), with shared vertex slots on tangent lines. It is laid out by two maps,
`Geo::Plane` for flat and `Geo::Round` for folded (build.rs:137-174, 1508-1557, 3130-3136). Flat and
folded therefore cannot disagree about topology or names. The unfold is an exact isometry at the
neutral radius rn = BA/θ (build.rs:1543, 163-173).

The patching that exists is concentrated in four places:
1. **Bend zones are always full rectangles, BA × joint length** (build.rs:1527). Any wall whose
   edges meet the mold line at a slant is refused as `TangentMismatch` (build.rs:1843-1858;
   error.rs:233-235 documents it). It is not trimmed. This is the biggest functional hole, and
   the probe hits it on ordinary parts (section 7).
2. **Corner-relief constructions assume the two edge normals are orthonormal**
   (`at = v + n1 x + n2 y`, build.rs:484). They also assume up flanges for tears. Acute corners and
   tears on down flanges fail with `TangentMismatch`.
3. **region.rs, the 2D boolean**, has a tolerance zoo: TOL, 2·TOL, 4·TOL, 0.9·TOL, ALONG = 1e-9,
   NEAR = 2·TOL, an absolute 0.1 mm, cos > 0.99, and a ray turned 0.0137 rad. These are the same
   incidence fact asked at different thresholds. It is the 2D twin of the boolean audit's rank 1.
4. **Clash and touch checks**: limit = 10·TOL, an inset of 10·TOL, a hem margin of 100·TOL,
   touch tested only plane against plane, depth capped at level 60, and a `phi > π/2 + 1` cut-off.
   The test that holds these accepts any answer inside a 2× band.

## 1. Flat pattern derivation (task 5)

- **BA**: `BendRule::K` gives `BA = angle·(R + K t)` (shop.rs:50). KTable interpolates K in R/t
  (shop.rs:51-61). `Allowance` interpolates BA in angle for a single radius (shop.rs:62-82). Hems
  use a table row if one exists, else the bend rule (build.rs:1091-1102).
- **Setback**: on the *reference skin*, `s = ρ·sin θ/(1+cos θ) = ρ tan(θ/2)` (shop.rs:244-247).
  ρ = R when the bend turns toward the reference side, and R + t otherwise (build.rs:44-50). Hems
  use s = R + t (hem.rs `HemJoint::setback`).
- **Developed placement**: wall b's 2D frame is `dev[a]·ja·shift(BA − 2s)·jb⁻¹` (build.rs:1139).
  The zone strip starts at a's tangent line: `dev[a]·ja·shift(−s)` (build.rs:1169), and it is the
  rectangle [0, BA] × [0, len] (build.rs:1527). The developed length is therefore
  Σ(mold-line lengths) − Σ2s + ΣBA, i.e. flat = Σ flange lengths − Σ BD with BD = 2s − BA. Exact.
- **Folded zone**: the point (u, y, depth w) maps to angle `u/rn`, radius `ρ0 + σw`, height `−σy`.
  rn = BA/θ (build.rs:152-168, 1539-1546), and `phi(u == BA) = θ` exactly (build.rs:172-174). Arc
  length at radius rn equals u, so the strip-to-cylinder map is an exact isometry on the neutral
  layer. With the K rule, rn = R + K t lies inside the material.
  - For `Allowance` tables nothing checks that rn lies in [R, R + t]. The map stays a bijection,
    but "neutral layer" loses its meaning.
- **Cuts across a bend** are wrapped with the same unroll rn at every depth (`Wrap { unroll: rn }`,
  build.rs:3585, 3597, 3658-3667). Their side walls are ruled between the two wrapped rails (right
  conoids), consistent with research/sheet-metal/arc/README.md.
- **Two evaluators of one fact**:
  - (a) **The tangent-line crossings.** Each strip runs its own `subtract`: wall at build.rs:1466,
    bend at build.rs:1529, with the cut carried into each frame by a different rigid motion
    (build.rs:1285, 1302). The crossings are then *matched by y within TOL*
    (build.rs:1843-1858). The geometry is the wall's (one slot), but whether they match is a
    tolerance decision.
  - (b) **The flat placement** exists twice: as a 3D `Frame` composition (`frames[..][1]`,
    build.rs:1136-1138, used for the flat body) and as a 2D `Rigid` (`dev`, build.rs:1139, used
    for the pattern, the overlap check and checks.rs). They differ only by rounding.
  - (c) checks.rs:692-716 `Place` is "a copy of build::Place2's motion".
- **Refold check**: none at run time. None is needed structurally (same slots, same names).
  - Tests: `fold_unfold_round_trip` (tests/build.rs:191, one 90° L, worst < 1e-13).
  - Tests: `k_moves_nothing_folded_to_the_bit` (tests/build.rs:267).
  - Tests: closed-form volume identity `V_fold − V_flat = θ w t²(½ − K)` (tests/build.rs:104-155).

## 2. Ranked patched areas

| # | area | where | class | what it can explain |
|---|---|---|---|---|
| 1 | Bend zone always a full-length rectangle; no trim of the zone where a wall edge meets the mold line at a slant | build.rs:1527; matching 1843-1880; doc error.rs:233 | construction for one configuration (square ends) → refusal | Probe: one flange on the edge of an 85° (or 75°) corner fails `TangentMismatch` gap 0.17 (0.53). Trapezoid and chamfered blanks, flanges off non-square edges. |
| 2 | Corner reliefs in an oblique basis treated as orthonormal: `at(x,y)=v+n1x+n2y`, `square_at` (n1±n2)h, tear along (n1+n2), `inner = at(−s1,−s2)` | build.rs:478-603; ear relief 731-771 (`l.y > 0.5·len` picks the end) | constructions written for 90° corners, padded by ±t | Probe: tear and trim-to-bend at a 75° corner fail `TangentMismatch`. A tear with two down flanges on a plain rectangle fails (gap 56.1). Every corner test uses a rectangle (tests/corner.rs:12-27); `CornerKind::Tear` is never tested there. |
| 3 | region.rs incidence thresholds (2D boolean) | see section 3 | several tolerances for one fact, order-dependent clustering | `NearCoincident`, `Pinched`, `Unchained`, `FlatOverlapUndecided` (build.rs:2843-2864). The README says ALONG/NEAR were "settled" against the trim runs (research/sheet-metal/README.md:63-67, 84-86). |
| 4 | Clash and touch checks | build.rs:1955 (`limit = 10·TOL`), 2005/2388 (hem margin `100·TOL`), 2018/2422 (inset `10·TOL`), 2026 (sag `+10·TOL`), 2106-2113 (touch: planes only), 2159/2410 (`level >= 60`), 2338 (`phi.abs() > π/2 + 1` → bound 0), 2386 (`eps = 1e-3·t`), 2475 | tolerance multiples, special-case exemption for hems > π (`loose`, 1924-1929) | Penetration ≤ 10·TOL between a *bend* zone and a wall is neither clash nor touch (touch is plane–plane only), so an invalid body could pass. The test `fuzz_curved_edges_skimming_a_wall` accepts any outcome for depth in (0, 2·limit] (tests/build.rs:930-961). |
| 5 | Relief retry loop | build.rs:990-1002 (`build_with` rebuilds after each `NeedsRelief`, adding one auto relief) | retry, bounded and deterministic | Cost (a full rebuild per relief), not wrongness. `hem_at` probes by building a clone (hem.rs `hem_at`). |
| 6 | Exact-float dispatch and equality on computed values | build.rs:3562, 3574, 3644, 3657 (`a.x == b.x`, `a.y == b.y` choose line/circle/plane vs wrap/ruled); corner.rs `mold_ok` `pts[m].0 == Point2::new(0.0, len)` and `pts[..].x != h`; bendline.rs:177 `cross <= 1e-12`; graph.rs:245 convex if `c > 1e-9`; hem.rs `e.cross(d).abs() <= 1e-9`; contour.rs:108 TANGENT 1e-9 | magic epsilons unrelated to ANGULAR = 1e-11 | A 1-ulp-off axis-parallel cut edge takes the Ruled path and may be refused (`WallFolds`). After `move_edge` lengthens a flange, the seam's exact-equality test can say `NotAFlange`. |
| 7 | Measurement by sampling where a closed form exists | remnant 5 rays + 64 samples (build.rs:841-877); hem `reach` 16 samples; mount point-in-wall by 32-sample polygon (graph.rs:753-765); bendline curve crossings 64 samples (bendline.rs:163-166, side_of 56-64); checks `gap` 17 starts of alternating projection (checks.rs:912-939); `inscribed` grid 40² + pattern search (checks.rs:941-1011); `Zone::to_loop` 9 samples (checks.rs:755-763); build `contact` 9 starts (build.rs:3708) | approximation | DFM violations and FlatTouch can miss or misplace. |
| 8 | Nudges and overcuts to dodge coincidence | band rect past the mold line by `max(t, s)` (build.rs:1366-1370); relief from `−(t + width + s)` (895); corner square ±t (486-487); seam/trim rects ±t (697-711, 752); TrimToBend kerf past the tangent lines "so the two zones would not meet at a point" (577-583); turned ray RAY = 0.0137 (region.rs:592-595) | deliberate overcut | Benign while t is large against the other features; they hide coincidences rather than decide them. |
| 9 | Names: tie-break by `(p·1e6).round()` | build.rs:3104 | coordinate-dependent | Name changes under motion only on ties. |

## 3. region.rs threshold inventory (one fact: "is this point/piece on that piece")

- `TOL = LINEAR = 1e-5` (37); `ALONG = 1e-9` on-boundary (40); `NEAR = 2·TOL` near-coincident
  refusal band (42).
- Line–line uses signed distances from exact `orient2d` divided by the float length
  (region.rs:271-278). These are compared to ±TOL (279-288), with a parameter slop of `TOL/len` and
  a clamp (295-297).
- Circle–line and circle–circle use `+TOL` slack (309, 323, 326). Their points are kept only within
  **2·TOL** of both pieces (412, 426). `ends_on` uses **TOL** (436-440).
- Broad phase and grid pad by 2·TOL (753, 756, 783-784, 808, 812).
- Vertex merging is first-come within **0.9·TOL** on a **4·TOL** grid (690-696). It is
  order-dependent and not transitive.
- A sub-piece is dropped when its ends merge and its midpoint is within **2·TOL** (869). Duplicate
  coincident pieces are dropped when midpoints fall within **4·TOL** (929).
- `side_of`:
  - "On boundary" needs gap ≤ ALONG at the midpoint and at the ¼ and ¾ points, plus |cos| > 0.99
    (829-838).
  - A near miss (ALONG < gap ≤ NEAR, |cos| > 0.99) is refused, but only when the ¼–¾ span is
    **> 0.1 mm** (absolute, 821). A shorter near-coincident piece falls through to the ray test.
- Spline end classification uses `1e-9·width` (654-656), `plane_roots` tol 1e-12 (647), and
  `to_segment` snaps to breaks within 1e-9 (198).
- `holds_hole` takes a vote at u = 0.3 and 0.7 of each hole piece (1043-1056).
- Line splitting (TOL), on-boundary (1e-9) and near miss (2e-5) are three readings of one incidence.
  A piece 1e-7 from a boundary is split as "along" by line_line, is not "on" by side_of, and is
  refused only when longer than 0.1 mm.
- This is the "2× and 4× TOL tests" the earlier audit flagged (research/kernel-audit/README.md:460).
  It is the same fix: decide incidence once with exact orient2d and one rule.

## 4. The "2× tolerance tests"

- In the code: region.rs 2·TOL / 4·TOL comparisons (above). This is what kernel-audit README:460
  means.
- In the tests: `fuzz_curved_edges_skimming_a_wall` (tests/build.rs:839-961). For depth > 0 it
  accepts `FoldedClash`; for |depth| ≤ 2·limit it accepts `FoldedTouch`; it accepts a build if
  depth ≤ 2·limit. So in (0, 2·limit], with limit = 10·TOL = 1e-4 mm, any outcome passes; it only
  counts `between`. The clash doc admits "no point deeper than 1.5 limits is missed"
  (build.rs:1945-1952): the check is rigorous only outside that band.
- Other loose tolerances in tests:
  - region tests accept area within `4·err + 1e-3` of the scanline and within 1e-3 of forge-bool
    (tests/region.rs:86-103).
  - overlap.rs:256 sets the cost budget at "about twice what they ask now".

## 5. Legitimate exact special cases

- `phi(u == BA) = θ` exactly (build.rs:172-174).
- A line across a zone parallel to the axis is a line, and perpendicular to it a circle
  (build.rs:3562-3583). The matching side surfaces are planes (3644-3646, 3669-3674).
- Tangent-line edges are straight lines through the shared vertices (build.rs:3171-3181).
- `K` moves nothing folded: walls are placed from mold lines and angles only (build.rs:1119-1139),
  checked bit for bit by the test above. Corner and seam reach uses the K = 1 bound
  `θ(R + t) + t` (build.rs:710, 751; corner.rs `reach`), so folded corners do not depend on K.
- Splines are split at knots in a bend, and arcs are cut into pieces under 0.99π
  (build.rs:1722-1737).
- Free (rolled) hem ends are side edges (build.rs:1677-1680).
- A closed hem's gap is a labelled convention, `GAP_FLOOR` 0.02 mm (hem.rs). So is the seam rule
  "the skin that keeps less" (corner.rs header).
- Table interpolation is exact in range, and a single row answers only its own x (shop.rs:119-136).

## 6. Honest refusals (missing general method)

- **Joint kinds**: `JointKindUnbuilt` for Rip and Tangent (build.rs:1037-1050).
- **Rules and corners**: `ReliefExtendUnbuilt` (1383-1389). `CornerUnbuilt` for Closed/Overlap
  without a seam (606-610).
- **Trim curves**: `TrimCurveUnsupported` for splines over cubic in a bend (1707-1718).
- **Forms**: `FormError::Unbuilt` for louvre and lance (form.rs:297-299).
- **Contour flanges** (contour.rs:22-88): Spline, Closed, ArcAtEnd, ArcsMeet (no tangent joint
  between bends), NotTangent, Turn ≥ π.
- **Sketched bends**: `BendLineError::NotSquare` (crossed edges must be lines exactly square to the
  line), Crossings ≠ 2, HoleAcross (bendline.rs:28-44, 160-185).
- **Seams**: NotAFlange (a plain rectangular flange only), TipTaken, Parallel (corner.rs:26-39).
- **Edges**: `NotStraight` (flanges and hems only on lines; graph.rs:345-352). `EdgeMoveCurve`,
  `EdgeMoveBlocked`.
- **Slanted wall ends**: `TangentMismatch` (build.rs:1693-1701, 1848-1878). This is the
  slanted-wall-end gap from section 2, rank 1.
- **Tolerance limits**: Pinched, NearCoincident, Unchained, HoleOutside (error.rs:790-861);
  `FlatOverlapUndecided`; `TrimAlongBendLine` (build.rs:1764-1801); `TrimTouchesBendLine`.
- **Angles and tables**: `BendAngle` outside (ANGULAR, π − ANGULAR), up to 2π for hems
  (build.rs:1080-1090). `Hem NotTabled` (a closed hem has no formula). `RuleError::OffTable`,
  `NoRow` (an `Allowance` table serves one radius only).
- **Clashes**: FoldedClash, FoldedTouch, FlatOverlap, FlatTouch, SheetSplit. These are real
  geometric refusals.

## 7. Probe results (scratchpad/sheetprobe)

**Run 1**: quad base, corner of interior angle A at (60, 0), flanges on edges 0 and 1. The far ends
are slanted.
- Every corner kind fails `TangentMismatch` at the *far* end of edge 1 for A = 100°, 120° and 135°.
- With down (−90°) flanges at A = 90°:
  - Tear fails `TangentMismatch` gap 56.14.
  - Circular, square and trim build.
  - Up flanges at 90° build for all kinds.

**Run 2**: pentagon, both far ends square to their edges.
- **One flange alone** on an edge whose start corner is **85°** (or 75°) fails `TangentMismatch`
  (gap 0.17 / 0.53, joint 0, side A).
  - The same corner with both flanges builds with auto, circular, square or open: the corner relief
    clears the slanted stretch.
- At 75°, tear and trim fail `TangentMismatch`.
- 90° to 120°: everything builds, with valid bodies.

So the bend zone has no general end treatment where the wall's adjacent edge converges toward the
bend, and the corner constructions are 90°-shaped.

## 8. Tests (task 6)

`cargo test -p forge-sheet` exits 0. There are 107 tests across 16 binaries and none fail:

| binary | tests |
|---|---|
| lib | 5 |
| bendline | 6 |
| build | 15 |
| contour | 5 |
| corner | 7 |
| form | 8 |
| hem | 7 |
| holes | 6 |
| overlap | 5 |
| pattern | 7 |
| process | 4 |
| punch | 7 |
| region | 7 |
| relief | 10 |
| seam | 6 |
| sides | 2 |

Doc-tests: 0. No `#[ignore]` anywhere in the crate.
Log: scratchpad/sheetcore-test.log. Note that scratchpad/test.log was overwritten by another agent's
run and is not this crate's.

## 9. Git history (task 4)

kernel/forge-sheet has exactly one commit: `979190b` "Install ForgeCAD launcher and update from
latest main". It is a squashed import, and the clone is shallow (`git rev-parse
--is-shallow-repository` = true). `git log -p` on any line returns only that commit. No per-fix
history exists in this clone. The test doc comments do not name failing parts or seeds as fix
origins.

The only provenance is in research/sheet-metal/README.md:
- "Tried and dropped": the 2e-5 band, the level ray and the level scanline (63-72).
- The kernel bug `cross_sections` (54-61).

So the constants were tuned against the prototype's trim runs, results/trim-*.txt and summary.md.
