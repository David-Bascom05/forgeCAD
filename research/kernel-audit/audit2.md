# ForgeCAD kernel audit, part 2: sheet metal, assemblies, mesh, STEP, sketch

Read-only audit. Repo `/home/user/forgeCAD`, branch `claude/repo-review-mbdrlj-audit2`, based on
`claude/repo-review-mbdrlj` at 8250ebb, 2026-11-22. No kernel or interface code was changed.
Line numbers are for that commit.

This continues [README.md](README.md), which audited the boolean, the intersections and the
predicates. It skipped sheet metal ("its own 2× tolerance tests; I did not audit it in depth") and
the other crates. The method is the same:
- find where one geometric fact is decided more than once, or patched by a retry, a nudge or a
  tolerance multiple;
- separate those from honest refusals and from legitimate exact special cases;
- for each patched area give the principled method, a Lean-provable specification, an exact
  reference, and evidence.

Working notes, with more line-level detail than this file, are in [audit2/](audit2/). There is
one per crate (`notes-*.md`), plus the repro probes:
- `sheetprobe.rs` with its `Cargo.toml`: slanted and acute wall corners;
- `shallow.fg` and `ell.fg`: flat-pattern probes;
- `dxfchk.py`: a DXF closure and number checker;
- `qr_noise_rank.rs`: the assembly rank defect.

**Git history.** Every audited crate arrived in one squashed import, `979190b` ("Install ForgeCAD
launcher and update from latest main", 2026-11-07). The clone holds no earlier history. No branch
can be tied to a fixing commit. Provenance comes instead from comments, test doc comments and
the research notes:
- research/sheet-metal/README.md:63-67, 84-86;
- research/step/README.md, which names crank, jack, WHU and the normalised part;
- research/mesh-bounds/README.md. The commit ids it cites (387a882, 2120083, a5c87f0) do not
  resolve here.

**Constraint check.** Two premises of the brief needed correcting:
- **"no NURBS" holds for the modeller, not for import.** STEP B-spline surfaces become a private
  `Procedural` NURBS surface (forge-step/src/nurbs.rs:1-2, read.rs:752). B-spline curves become
  `Spline::from_nurbs` (read.rs:488).
- **No NURBS-to-analytic recognition exists.** fit.rs runs the other way: on export it fits
  procedural and intersection geometry as cubic B-splines (fit.rs:1-2).

So "exact procedural geometry with no NURBS" is a constraint on what the modeller makes. It is not
a constraint on what STEP brings in.

---

## 0. Short answer

**Sheet metal is mostly one definition, not case patches.**
- Each strip (a wall or a bend zone) has one 2D region in developed coordinates.
- One topology, with shared vertex slots on the tangent lines, is laid out twice: `Geo::Plane` for
  the flat solid and `Geo::Round` for the folded one (forge-sheet/src/build.rs:137-174, 1508-1557,
  3130-3136). The flat and folded solids therefore cannot disagree about topology or names.
- The unfold is an exact isometry of the neutral layer, by construction (§2.0).
- The flat-pattern export is exact: no chords anywhere, shortest round-trip floats, and DWG
  rewritten byte-identical.

**Where sheet metal does patch, the pattern is "built for one configuration".**
- The bend zone is always a full-length rectangle (build.rs:1527).
- The corner reliefs are built in an oblique basis as if it were orthonormal (build.rs:478-603).
- Off the square case, both are refused, as `TangentMismatch` or a bad corner. The result is never
  wrong. These refusals are the largest source of failure found: reproduced on plain
  trapezoid-like blanks (§2.1, §2.2).

**One wrong-result bug was found and reproduced.** The flat pattern's outer contour is chosen by
the largest bounding box, and that box counts every arc as its full circle (pattern.rs:133-143,
643-646). With a shallow arc on a cutout this mislabels the outer contour and moves the part note
about 1 m away.

**In the 2D region code the boolean's rank-1 pattern recurs.** One incidence is read as `ALONG`
(1e-9), as `TOL` and as `NEAR = 2·TOL`, with further 2·TOL and 4·TOL tests (region.rs:40-42 and
following).

**The other crates:**
- **forge-mesh:** the CDT core is clean (exact orient2d, typed refusals). The tessellator around it
  carries a retry ladder:
  - cut edges finer, up to 9 rounds;
  - pin touches;
  - re-mesh a cone without unrolling;
  - nudge duplicates in uv;
  - "mend" file bodies.

  Its refinement cap silently returns `Ok` when refinement has not converged (cdt.rs:1007).
- **forge-asm:** the solve is principled: closed form on the tree, Newton only on loop blocks, no
  restarts. "Met or conflict" and "rank" are each decided at several sites with different
  epsilons, none of them LINEAR. One real defect was reproduced: a purely relative QR threshold
  gives rounding noise rank 1. Interference uses a mesh depth gate, then an unrelated absolute
  volume.
- **forge-step:** import healing decides "is this edge on its faces" five ways, and none of them
  uses the file's stated uncertainty. The healing limit can reach 0.02 mm on a file that states
  1e-6. Sliver collapse depends on order. Voids are retried with the flag flipped. Export is
  exact for analytic entities.
- **forge-sketch:** the earlier "fine" is confirmed. It has no restarts and a standard LM. Its
  minor issues are rank scales that differ per question, and two projection nudges that make up
  for the boolean's rank-1 issue downstream.

**Tests today.** Everything passes:

| crate | passed | failed | ignored | binaries |
|---|---|---|---|---|
| forge-sheet | 107 | 0 | 0 | 16 |
| forge-asm | 42 | 0 | 0 | — |
| forge-mesh | 28 | 0 | 0 | — |
| forge-step | 42 | 0 | 0 | — |
| forge-sketch | 23 | 0 | 0 | — |
| forge-dwg | 152 | 0 | 0 | lib 145, corpus 4, scene 3 |

- None of these crates has an entry in `tools/proof/known.txt` or `fuzz_known.txt`, except mesh M8
  (`certify: Mesh`, known.txt:14).
- None of these crates has a fuzzer of its own.

The failures below were found by probes written for this audit, not by the suites. That is itself
a finding: the suites test the configurations the code was built for.

---

## 1. Ranked list

The rank is by how much failure each item explains: reproduced refusals or wrong results first,
then latent wrong answers, then cost.

| # | crate | area | class | evidence | principled fix | fits constraints |
|---|---|---|---|---|---|---|
| 1 | sheet | Bend zone is always a full-length rectangle `[0,BA]×[0,len]`; the tangent-line crossing is computed twice and matched by y within TOL | one configuration, then refusal; two evaluators | probe: a flange on the edge of an 85° / 75° corner → `TangentMismatch` gap 0.17 / 0.53; 100–135° quads with slanted far ends fail for every corner kind | zone end = the developed image of the neighbouring walls' boundary, taken from the shared vertex slots (decided once, in the wall); ruled or wrapped end faces | yes |
| 2 | sheet | Corner reliefs and seams built in the oblique basis (n1, n2) as if orthonormal, padded by ±t | one configuration (90°) | probe: tear and trim-to-bend at 75° fail; tear with two down flanges on a plain rectangle fails (gap 56.1); `CornerKind::Tear` untested in tests/corner.rs | solve the corner in the true frame: intersections of offset lines by 2×2 solves; relief as an offset of the zones' flat footprints | yes |
| 3 | sheet/pattern | `outer_contour` = largest box; `bounds` boxes each arc as its full circle | **wrong result** | probe shallow.fg: part note at (−480, −971); blank goes on the interior layer with split layers | outer = the unique loop with positive signed area (orientation is already exact); box arcs by their extremes | yes |
| 4 | mesh | Tessellation retry ladder (cut_finer ×9, pins, cone re-try, uv nudge, mend) and a silent refinement cap | retries, nudges, silent fallback | cap at cdt.rs:1007 returns Ok unsettled; M8 open; parabola and hyperbola edges get 64 segments whatever the chord tolerance | split crossing chords once per face before the CDT, by exact segment tests; refinement that stops with an error or a certified bound | yes |
| 5 | sheet | region.rs: one incidence read at ALONG / TOL / NEAR = 2·TOL / 4·TOL; vertex clustering that depends on order | several tolerances for one fact (the 2D form of README rank 1) | refusals `NearCoincident`, `Pinched`, `Unchained`, `FlatOverlapUndecided` | decide each (piece, piece) incidence once by exact orient2d plus one tolerant rule; union–find vertex classes with a spread cap (as in ForgeExact/Snap.lean) | yes |
| 6 | step | Healing: "edge on face" decided 5 ways, none with the file's uncertainty; heal limit `(unc.max(1e-3)·20).min(2e-3·size)`; sliver collapse in edge order; void flag flipped and re-tried | retries, tolerance multiples, order dependence | research/step/README: the cap was "tried once and not tuned"; 20×LINEAR added for one part (crank, 80 s → 0.9 s) | tolerant B-rep (Jackson 1995): each entity carries one tolerance derived from the file; one incidence decision per (edge, face); union–find in name order | yes |
| 7 | sheet | Folded clash and touch: `limit = 10·TOL`, margins of 100·TOL, a level cap, touch tested plane-to-plane only | tolerance multiples; undecided band | a zone that penetrates a wall by ≤ 10·TOL is neither clash nor touch, so an invalid body can build; the test accepts anything in (0, 2·limit] | certified signed distance between the zone's cylinder band and the wall slab (closed form for plane/cylinder) with one tolerance | yes |
| 8 | asm | "Met / conflict" at 6 sites and 5 thresholds; rank decided 4 times; relative-only QR threshold; not invariant under rigid motion | several tolerances for one fact | qr_noise_rank.rs: `rank([1e-17, −3e-17]) = 1`; a 2e-7 mm misfit is a Conflict on a fastened cycle but Solved in a loop block | generic rank from the constraint graph (exact), plus one numeric SVD with a gap test, in the block's own frame | yes |
| 9 | asm | Interference: mesh depth gate (tolA + tolB + LINEAR), then an exact Intersect, then touch iff vol ≤ 1e-6 mm³ | two unrelated tests for one fact | face-to-face contact is reported Clear; fit-covered pairs are never checked exactly | one classification from the boolean: empty / touch (positive area, zero volume) / overlap | yes |
| 10 | sheet/pattern | `joined` merges pieces by mixed epsilons (1e-12, TOL); "whole circle" tested at TAU·(1 ± 1e-12) in 3 places; twin matching not one-to-one | magic epsilons; two evaluators | closed spline cutouts export as 2 open C0 SPLINEs | merge by identity (`Bound.id`), not by geometry | yes |
| 11 | sheet | Exact `==` on computed coordinates picks the construction (line, circle, wrap or ruled); stray 1e-9 and 1e-12 epsilons | one-ulp fragility | a cut one ulp off axis-parallel takes the ruled path, possibly `WallFolds` | classify by the cut's source (its definition), not its rounded coordinates | yes |
| 12 | sheet | Sampling where a closed form exists: remnant, hem reach, mount point-in-wall, bendline crossings, checks `gap`, `inscribed` | approximation | shop-rule violations and FlatTouch can be missed or misplaced | closed-form line/arc distances; exact point-in-region | yes |
| 13 | step | Several thresholds for pole, periodic and same-parameter; `speed_bound` 1.5 × a sampled max used as a bound; void flip | magic numbers | no failing file in the repo; the corpora are kept outside | interval enclosures for bounds; one tolerance per entity | yes |
| 14 | sketch | Rank scales per question; `project.rs` nudges ellipse axes (≤ 10·LINEAR) and circle centres | downstream of README rank 1 | none in tests | removed by the decide-once boolean; one rank scale | yes |
| 15 | sheet | Relief retry: the whole part is rebuilt once per `NeedsRelief` | bounded retry | cost only | compute the needed reliefs before building, from the joint graph | yes, faster |
| 16 | dwg (editor only) | draft/intersect.rs tangency decided 3 ways; `scan_roots` by sampling | sampling | not on the export path | certified root isolation (README §2.4) | yes |

What "fits constraints" means here:
- IEEE arithmetic plus sqrt plus `forge_math::dmath` only;
- native and wasm bit-identical;
- no third-party crates in kernel/;
- no slower;
- never a wrong result.

Every fix proposed here uses only exact orient2d, linear and quadratic solves, and the existing
dmath functions.

---

## 2. Sheet metal (forge-sheet), in depth

### 2.0 What "the flat pattern is correct" means

**How it is built today:**
- **Bend allowance:** BA = θ(R + K·t) under a K rule. K is interpolated against R/t, or BA is
  interpolated against θ for one radius from an Allowance table (shop.rs:50-82).
- **Setback** is measured on the reference skin: s = ρ·sin θ / (1 + cos θ), that is ρ·tan(θ/2)
  (shop.rs:244-247). ρ = R if the bend turns toward the reference side, else R + t (build.rs:44-50).
  Hems use s = R + t.
- **Developed placement:** the developed placement of wall b is `dev[a] · ja · shift(BA − 2s) ·
  jb⁻¹` (build.rs:1139). The zone is the rectangle [0, BA] × [0, len] (build.rs:1527).
- **Folding a zone point:** (u, y, w) goes to angle u/rn at radius ρ0 + σw, with rn = BA/θ
  (build.rs:152-168, 1543). The far tangent line snaps exactly: φ(BA) = θ (build.rs:172).
- **Cuts across a bend** are wrapped with the same rn (build.rs:3585, 3658-3667).

**Definition.** Write the part as a tree (or a graph with seams) of walls i and bends j. The
unknowns and data are:
- A wall i is a planar region Ω_i ⊂ ℝ² in its own developed frame.
- A bend j joins walls a and b along mold lines ℓ_a and ℓ_b. Its data are R_j, θ_j ∈ (0, π), t
  and BA_j.
- Let rn_j := BA_j/θ_j, the neutral radius, and κ_j := (rn_j − R_j)/t, the effective K.
- The flat domain is D = ⋃ P_i(Ω_i) ∪ ⋃ Q_j(Z_j) ⊂ ℝ², where P_i and Q_j are rigid motions of ℝ²
  and Z_j is the zone region.
- The fold is F : D × [0, t] → ℝ³:
  - on a wall, F = M_i ∘ (P_i⁻¹, w), with M_i a rigid motion of ℝ³;
  - on a zone, F(u, y, w) = C_j + (ρ0_j + σ_j·w)(cos(u/rn_j)·e1 + sin(u/rn_j)·e2) + y·e3, with
    (e1, e2, e3) orthonormal.

The flat pattern is correct iff the following hold.

1. **Refold.** The image F(D × [0, t]) equals the folded solid, as a point set. The bijection is
   also face for face, and the names agree.
2. **Isometry of the neutral layer.** At w = κ_j·t, the first fundamental form of F restricted to
   each zone is the identity: |∂F/∂u| = 1, |∂F/∂y| = 1, ∂F/∂u · ∂F/∂y = 0. On a wall this holds
   trivially, since F is rigid. The condition is equivalent to (ρ0_j + σ_j κ_j t)/rn_j = 1.
3. **Bend allowance.** The zone's width in u is exactly BA_j, and its far tangent line maps to
   angle exactly θ_j.
4. **Length law (bend deduction).** For walls a and b joined by bend j, the developed distance
   between their mold lines is L_a + L_b − (2s_j − BA_j). Here L is measured to the mold line
   and s_j is the stated setback. Along any wall-to-wall path the lengths add up.
5. **Continuity.** F is C¹ across every tangent line. The tangent of the zone's ends at u = 0
   and u = BA equals the wall's direction.
6. **Injectivity.** F is injective on D × [0, t], away from intended seams. That is no folded clash.
   The plane map D → ℝ² is injective too, the overlap rule: no flat overlap.
7. **Physical K.** κ_j ∈ [0, 1], so the neutral layer lies inside the material.
   - **Nothing checks this today.** When an Allowance table supplies BA, rn can leave [R, R + t]
     with no error.
   - A table row that implies κ ∉ [0, 1] does not describe sheet metal.
8. **Export.** The exported loops are exactly ∂D, the same curves to the bit after rigid motions.
   The outer loop is the unique loop with positive signed area. Bend and tangent lines lie at
   u = BA_j/2 and u ∈ {0, BA_j}.

**Status.** By construction, items 2, 3, 4 and 8 already hold, except for the outer-loop choice
(§2.3).
- Tests that cover them:
  - `fold_unfold_round_trip`: an L at 90°, worst error under 1e-13 (tests/build.rs:191);
  - `k_moves_nothing_folded_to_the_bit` (tests/build.rs:267);
  - the volume identity V_fold − V_flat = θ·w·t²·(½ − K) (tests/build.rs:104-155);
  - bend lines checked with `assert_eq`.
- Item 1 is structural (one topology, two layouts). Only one 90° L tests it.
- Item 6 is the clash, touch and overlap checks (§2.5).
- Item 7 is missing.

**Lean specification of the fold.** Over ℝ:
- **Data.** `Zone := (R θ t BA : ℝ) (ρ0 σ)`, with the hypotheses 0 < θ < π, t > 0, BA > 0.
- **Definitions.** `fold z (u,y,w)` as above, and `rn z := BA/θ`.
- **Theorem `iso`.** If ρ0 + σκt = rn, then at w = κt the metric of `fold` is the identity. This
  is pure calculus over ℝ, using Mathlib's `Real.cos` and `Real.sin`.
- **Theorem `deduction`.** The composition `shift(BA − 2s)` equals the developed distance of
  item 4.
- **Theorem `refold`.** For a tree of strips with walls and zones glued along shared slots,
  `fold ∘ develop = folded`, by induction on the tree.

The predicates the kernel branches on are finite and named:
- `towardRef(j)`, which chooses ρ;
- `BA_from_rule` against `BA_from_table`;
- `κ ∈ [0, 1]`, which is new;
- `θ < π`.

The proof obligation for each branch is to show that s, ρ and rn are the ones items 2–4 require.
These obligations are small and mostly algebra. This is a good second Lean pilot after the decide-
once boolean.

**Exact reference: a refold checker.**
- **Input:** the exported pattern, as exact doubles (DXF `{:?}` values or DWG raw doubles), the
  bend table, and the folded B-rep.
- **Method:**
  - recompute F with interval arithmetic: outward-rounded IEEE intervals, sin and cos enclosed by
    the dmath error bounds widened by 1 ulp;
  - map each vertex and each curve's defining points;
  - check that every folded vertex lies in its interval image and that the topology is identical.
- **Arcs on walls** map rigidly, so they are checked exactly.
- **Wraps across bends** are checked at their Bernstein control data.
- **Effort:** about 600 lines in verify/, using the developed loops `Strips::pattern` already
  produces.
- **What it judges:** items 1–5 and 8, on every sheet part in examples/ and parts/sheet.

### 2.1 Rank 1: the bend zone is always a full-length rectangle

**What happens.**
- Each bend zone is `rect(0, 0, ba, len)` (build.rs:1527).
- The wall and the bend each run an independent `subtract` of the cuts:
  - in the wall, build.rs:1466;
  - in the bend, build.rs:1529.
- Where each run crosses the tangent line is then matched by y within TOL (build.rs:1838-1863).
  An unmatched stretch is refused as `TangentMismatch` (1867-1880, documented at error.rs:233).
- The match is not one-to-one: a double match silently overwrites.
- When the wall's neighbouring edge meets the mold line at a slant, the wall's boundary leaves the
  tangent line at a point the rectangle does not have. The two runs then disagree, and the part
  is refused.

**Evidence.** sheetprobe.rs, re-run for this write-up. The blank is a pentagon with an interior
angle A at (60, 0), t = R = 2, K = 0.4.

| A | flange on one edge only | both flanges, auto / circular / square / open corner |
|---|---|---|
| 90° | builds | builds |
| 85° | `TangentMismatch` gap 0.17 | builds (the relief clears the slant) |
| 75° | `TangentMismatch` gap 0.53 | builds; tear and trim fail |

The run-1 quad, with slanted far ends at 100°, 120° and 135°, fails for every corner kind.

Every corner and flange test in the suite uses rectangles (tests/corner.rs:12-27). That is why
the suite is green.

**Classification.** This is an honest refusal, never a wrong body, but on a very common input:
any flange on an edge next to a non-square corner. The real defect is two evaluators of one
fact, the crossing on the tangent line computed in the wall and again in the bend.

**Principled method.** Decide the zone's end once, from the wall side:
1. The tangent line is shared, and its vertex slots already exist (build.rs:137-174).
2. Compute the walls' regions first.
3. Read off where each wall's boundary meets its tangent line: the points p_a(0) and p_b(BA).
4. Build the zone's end edges from those points.

There are two standard end conditions:
- **"Normal".** The zone end runs square to the bend line from the nearer point. The wall then
  needs a relief or a mitre on the slanted side: an automatic rectangular relief, or a
  `NeedsRelief`.
- **"Aligned" or "edge continued".** The zone end is the straight segment in developed
  coordinates from p_a to p_b.
  - On the folded part this is a wrapped line on the cylinder, a helix arc.
  - The kernel already has that curve: Wrap curves, as used for cuts across bends,
    build.rs:3585, 3658-3667.
  - Its side face is the ruled surface the kernel already builds for slanted cuts in a zone.

Commercial modellers offer both: flange "edge offset" and end-condition options, with bend relief
for the normal form. Either way, the crossing is taken from the wall and never recomputed. The
match at 1838-1863 disappears, and with it `TangentMismatch` as a symptom.

**Lean spec.** Over ℝ²:
- the zone region is Z = {(u, y) : 0 ≤ u ≤ BA, e0(u) ≤ y ≤ e1(u)}, with e0 and e1 affine in u;
- e0(0) and e1(0) are the wall a's boundary points on its tangent line, and likewise at BA for
  wall b;
- the predicate the code branches on is `endCondition ∈ {normal, aligned}`;
- obligations:
  - for "aligned": the wall's and the zone's boundaries share endpoints, so the developed
    boundary is closed;
  - for "normal": the relief region contains the triangle between the slanted edge and the
    square end;
  - in both: F is continuous on the closure.

**Exact reference.** The refold checker of §2.0 together with an exact 2D arrangement (§2.4)
over the dyadic inputs.

**Tests.** None today. Add the probe's pentagon and quad families at A ∈ {60, 75, 85, 90, 95,
100, 120, 135}° to tests/corner.rs, for every corner kind and with up and down flanges.

### 2.2 Rank 2: corners built in an oblique basis as if orthonormal

**What happens.**
- The basis is the edges' outward normals n1 and n2, with `at(x, y) = v + n1·x + n2·y`
  (build.rs:478-490).
- Everything below treats (n1, n2) as if it were an orthonormal frame:
  - the square relief `(n1 ± n2)·h`;
  - the tear along n1 + n2;
  - `inner = at(−s1, −s2)`;
  - the trim-to-bend kerf (577-583).
- Each construction is padded by ±t (486).
- The ear relief picks its end with `l.y > 0.5·len` (744).
- At 90°, n1 ⊥ n2 and all of this is exact. At other angles, `at(−s1, −s2)` is not the point at
  distance s1 from one mold line and s2 from the other. That distance is s/sin A.

**Evidence.**
- With a 75° corner, tear and trim-to-bend fail.
- A tear with two *down* flanges on a plain rectangle fails, with a gap of 56.1 (notes-sheet-core
  §7). This one also fails at 90°, so it is not only the angle. The tear direction does not
  account for the flange sense.
- `CornerKind::Tear` is never tested in tests/corner.rs.

**Principled method.** Construct in the true frame:
- the point at offsets (d1, d2) from two lines is the solution of a 2×2 linear system n_k·x =
  c_k + d_k, by Cramer with exact orient2d signs;
- reliefs are defined as regions, not as recipes:
  - "circular": the disc of radius r centred where the zones' inner offset lines meet;
  - "square": the parallelogram bounded by the zones' outer offset lines;
  - "tear": the minimal cut that separates the two zones' footprints. That is a segment along the
    bisector of the two inner lines, oriented by the flange senses.
- Then subtract them in the 2D region code with the decide-once rule.

The ±t padding exists to dodge exact coincidence with the zone ends. With incidence decided once
it is not needed, and it should go: it hides coincidences instead of deciding them.

**Lean spec.**
- Over ℝ², for lines ℓ1 and ℓ2 with |n1 × n2| > 0, `corner(d1, d2)` is the unique point with
  signed distances (d1, d2).
- Each relief R_kind satisfies: R ⊇ (Z_1 ∩ Z_2 footprint overlap) and R ∩ (both walls) is what the
  rule says. For example, a circular relief is tangent to both inner lines.
- Predicates:
  - `n1 × n2 > 0` (convex), `= 0` (refuse) or `< 0` (reflex);
  - the flange senses σ1 and σ2;
  - the kind.
- The obligation for each kind and sense: the relief region contains the overlap and does not cut
  past the stated depth.

**Exact reference.** Same as §2.1. A corner is right iff the refold has no clash and the flat has
no overlap, checked exactly.

### 2.3 Rank 3: outer contour by bounding box (wrong result)

**What happens.**
- `outer_contour` returns the contour with the largest axis box (pattern.rs:133-143).
- `bounds` adds an arc's whole circle box, c ± r, whatever its sweep (pattern.rs:643-646).
- Three things use this:
  - the outer/inner layer split;
  - the part note's position (pattern.rs:423-430);
  - the DWG limits (pattern.rs:807-812).

**Evidence.** The probe is shallow.fg: a 10×5 window whose bottom is an R495 arc, in a 68.4×60
blank.
- The window's "box" is 990×990, so it is taken as the outer contour.
- The default DXF puts the part note at (−480, −971).
- With split layers, the blank outline goes on the interior layer. The GUI offers split layers
  (interface/forge-render/src/gui_sheet.rs:231-245).

**Principled method.** The exact fact is already known:
- loops are oriented, outer counter-clockwise and holes clockwise (tests/pattern.rs:296);
- the sheet is one piece (build.rs:1904-1928).

So the outer loop is the unique loop whose signed area is positive. Signed area is exact for lines
(a shoelace sum of orient2d terms), and for arcs and splines it has a closed form: the segment
term ½r²(Δφ − sin Δφ) plus the chord. Use the sign only, since the magnitude is not needed.
- Box an arc by its endpoints plus the axis extremes inside its sweep.
- Better still, box it from the region code, which already knows the extremes.

**Lean spec.**
- For a set of disjoint simple closed curves with consistent orientation, the outer curve is the
  unique one with winding +1 around a point of the region.
- The predicate is `sign(signedArea(loop))`.
- The obligation is that exactly one loop is positive. That follows from the one-piece rule and
  the orientation convention.

**Tests.** tests/pattern.rs has no curved cutouts. Add shallow.fg. Also fix the stale comment
"outer contour and five holes" next to `assert_eq!(.., 4)` (tests/pattern.rs:291-292).

### 2.4 Rank 5: region.rs, one incidence read several times

The 2D boolean that cuts every wall and zone (region.rs) reads one incidence at several thresholds:
- `ALONG = 1e-9` decides "on the boundary". `NEAR = 2·TOL` is the band in which it refuses
  (region.rs:38-42).
- Line–line divides an exact `orient2d` by a float length and compares the result to ±TOL. It
  allows a TOL/len slop with a clamp (271-297).
- Arc and spline crossings are kept within 2·TOL (412, 426). `ends_on` uses TOL (436-440).
- Vertices are merged first-come within 0.9·TOL on a 4·TOL grid (690-696). This depends on order.
- A sub-piece is dropped as "too short to see" at 2·TOL (869).
- Duplicate coincident pieces are dropped at 4·TOL (929).
- The near-miss refusal applies only to pieces longer than an absolute 0.1 mm, and uses a 0.99
  cosine (821-832). A near-coincident piece shorter than 0.1 mm falls through to the ray test.
- The ray test's ray is turned by 0.0137 rad, to dodge vertices (595).
- Holes are assigned by a vote at u = 0.3 and u = 0.7 (1043-1056).

These are the "2× tolerance tests" the earlier audit flagged (README.md:460). In the tests, the
looser bounds are:
- region area within 4·err + 1e-3 of a scanline estimate, and within 1e-3 of forge-bool
  (tests/region.rs:86-103);
- `fuzz_curved_edges_skimming_a_wall` (tests/build.rs:839-961), which accepts any outcome for a
  penetration depth in (0, 2·limit] (§2.5).

**Principled method.** Use the README §6.1 rule in 2D. This is the same rule
verify/forge-exact/ForgeExact/Snap.lean proves for planes.
1. Every (piece, piece) incidence is decided once, keyed by the piece ids. For lines, use the exact
   orient2d of the endpoints against the other line, then one tolerant rule: "within LINEAR along
   the whole overlap" means coincident.
2. Vertex classes are formed by union–find over the decided pairs, with a spread cap. The class
   representative is the least-named point, so the result does not depend on order.
3. Crossings are recomputed in the representative curve.
4. Point-in-region uses winding numbers with exact orient2d against lines and an exact
   point-against-circle sign: |p − c|² − r² by two-product and two-sum, exact in doubles. No
   turned ray is needed.
5. For arcs, the line–circle and circle–circle intersection coordinates lie in ℚ(√d). Comparisons
   in that field are exact by the sign-of-a + b√d rule. That is IEEE plus sqrt, with an exact
   fallback on expansions.

**Lean spec.**
- Inputs are dyadic lines and circles.
- `tolEq(p, q)` holds iff the overlap region of the two pieces is not empty and the Hausdorff
  distance over it is at most LINEAR.
- `cls` is the transitive closure, and `rep` is the least name.
- Theorems, exactly as in Snap.lean but over ℚ² and ℚ(√d)²:
  - `rep_order`: the result is independent of order;
  - `rep_iso`: the result is invariant under the exact symmetries;
  - `onIn_class`: decisions agree across a class.
- Branch predicates: `orient2d`, `circleSign` and `spreadOk`.
- Per-branch obligation: each output loop is simple and closed, and its region equals the exact
  regularised boolean of the snapped inputs.

**Exact reference.** Flat.lean's sign-pattern arrangement, in 2D: half-planes and discs.
- The algebraic numbers are at most quadratic, so an executable over ℚ(√d) is about 400 lines of
  Lean.
- forge-exact's `Ref/Arrange.lean` is the 3D planar version to adapt.

**Tests.**
- The region fuzz is in tests/region.rs.
- Nothing fails today.
- The refusals (`NearCoincident`, `Pinched`, `Unchained`, `FlatOverlapUndecided`) are the
  observable cost.

### 2.5 Rank 7: folded clash and touch

**What happens.**
- The clash depth is `limit = 10·TOL` (build.rs:1955, 2475).
- There are further margins:
  - the hem margin, 100·TOL (2005, 2388);
  - the inset, 10·TOL (2018, 2422);
  - the sag allowance, +10·TOL (2026).
- Touch is tested only plane against plane (2106-2113).
- Refinement is capped at level 60 (2159, 2410).
- `phi.abs() > π/2 + 1` sets the bound to 0 (2338).
- `eps = 1e-3·t` (2386).
- Hems past π are exempted as `loose` (1924).
- The doc comment itself says the check misses nothing deeper than 1.5 limits (1945-1952). The
  band (0, 1.5·limit] is therefore undecided: a zone that penetrates a wall that far builds, and
  is neither a clash nor a touch. That is a possibly invalid body.

**Principled method.** For a zone against a wall, the zone is an annular sector of a cylinder
times an interval, and the wall is a slab with a polygonal or arc boundary.
- The signed distance has a closed form: plane against cylinder is the distance to the axis minus
  ρ, clipped to the sector.
- So decide "clash" (penetration > LINEAR), "touch" (|d| ≤ LINEAR on a set of positive measure)
  and "clear" with one tolerance and certified bounds: dmath error plus the interval width.
- Zone against zone is cylinder against cylinder: the coaxial or parallel cases are closed form,
  and the general case uses the README §2.4 certified root isolation.

**Lean spec.**
- `clash(Z, W) ↔ ∃ p, depth(p) > LINEAR`
- `touch ↔ ¬clash ∧ measure{p : |d(p)| ≤ LINEAR} > 0`
- The obligation is that the three verdicts are exhaustive and exclusive, with no band.

### 2.6 The remaining sheet items (ranks 10, 11, 12, 15)

**Rank 10: merging pieces in the flat.** `joined` merges pieces using several unrelated
tolerances (build.rs:3916-3943):
- cross ≤ 1e-12, relative (not ANGULAR = 1e-11);
- points and radii within TOL;
- sin of the angle gap ≤ 1e-12;
- sweep ≤ TAU·(1 + 1e-12).

"Whole circle" is tested at TAU·(1 − 1e-12) in pattern.rs:574 and 700, and again in a test.

The fix is to merge by identity. Pieces come from one source (`Bound.id`). Two consecutive pieces
of one source curve, split only by a tangent line, are one curve: no geometric test is needed.
Closed spline cutouts then export as one SPLINE.

**Rank 11: exact `==` on computed coordinates.** `a.x == b.x` or `a.y == b.y` chooses line, circle
or plane against wrap or ruled (build.rs:3562, 3574, 3644, 3657). There are also stray epsilons:
- bendline `cross ≤ 1e-12` (bendline.rs:177);
- convex `c > 1e-9` (graph.rs:245);
- hem parallel ≤ 1e-9;
- contour `TANGENT = 1e-9` (contour.rs:108).

The fix is to classify a cut by its definition: "the source edge is parallel to the bend axis".
That is a fact about the sketch, decided once. Its rounded developed coordinates should not decide
it.

**Rank 12: sampling where a closed form exists.**
- remnant: 5 rays and 64 samples (build.rs:841-877);
- hem reach: 16 samples;
- mount point-in-wall: a 32-sample polygon (graph.rs:753);
- bendline crossings: 64 samples (bendline.rs:163-166);
- checks `gap`: 17 starts (checks.rs:912);
- `inscribed`: a 40² grid plus pattern search (checks.rs:941-1011);
- `Zone::to_loop`: 9 samples (755-763).

The fix is closed-form line and arc distance, and exact point-in-region (§2.4). `inscribed`, the
largest inscribed circle, is the medial-axis problem. For line and arc boundaries it is exact by a
Voronoi of segments (Held, "VRONI", CGTA 2001), or certified by branch-and-bound with interval
distance.

**Rank 15: the relief retry.**
- build.rs:990-1002 rebuilds the whole part once per `NeedsRelief`.
- `hem_at` builds a probe clone.
- It is deterministic and bounded, so it costs time but gives no wrong answers.
- The fix is to compute from the joint graph which corners need relief, before building.

### 2.7 Sheet: honest refusals (keep, as gaps)

**Joints and features that are not built:**
- `JointKindUnbuilt`: Rip and Tangent joints (build.rs:1037-1050);
- `ReliefExtendUnbuilt`;
- `CornerUnbuilt`: a Closed or Overlap corner without a seam;
- `TrimCurveUnsupported`: a spline above cubic in a bend (1704);
- forms `Unbuilt`: louvre and lance.

**Contour flanges** are refused for:
- splines;
- closed profiles;
- an arc at an end;
- two arcs in a row;
- non-tangent arcs;
- turns of π or more.

**Sketched bends:**
- `NotSquare`;
- a number of crossings other than 2;
- a hole across the band.

**Seams:**
- `NotAFlange`: plain rectangular flanges only;
- `TipTaken`;
- `Parallel`.

**Edges:**
- `NotStraight`;
- `EdgeMoveCurve`;
- `EdgeMoveBlocked`.

**Tables and angles:**
- `BendAngle`: θ ≥ π;
- hem `NotTabled`: a closed hem has no formula;
- `OffTable` and `NoRow`.

**Overlaps:**
- `FlatOverlap`, `FlatTouch` (with a suggested seam) and `FlatOverlapUndecided`. These are decided
  by exact area booleans (build.rs:2712-2930).

The flat-overlap `LIMIT = 1e-6 mm²` (2713) is an absolute area. It should be LINEAR × the overlap
length.

### 2.8 Sheet: legitimate exact special cases (keep)

- One topology with two layouts, and the φ = θ snap at the far tangent line (build.rs:172).
- Lines in a zone parallel to the axis are built as lines, and lines perpendicular to it as circles,
  with plane side faces. This is correct once "parallel" is decided by definition (rank 11).
- K moves nothing in the folded part, held to the bit by a test. Corner and seam reach use the
  K = 1 bound θ(R + t) + t.
- Arcs in a bend are split under 0.99π and splines at their knots (1722-1737). The 0.99π split
  is a representation choice, not a decision.
- `GAP_FLOOR` = 0.02 mm for a closed hem, a labelled shop convention.
- Table interpolation is exact within range. `RADIUS_MATCH = 1e-9` (shop.rs:44) should be stated
  as "the same radius as written in the table".

### 2.9 Flat-pattern export, DXF and DWG

**The path.**
1. `Strips::pattern` walks the exact developed loops (build.rs:2982-3028).
2. `developed` moves each piece by the strip's rigid motion (3881-3913). Lines, arcs and NURBS
   pieces are kept as they are (`Spline::stretch`, forge-geom/src/spline.rs:947). Ellipse
   cutouts come out as exact rational quadratic SPLINEs.
3. Contours are chained by topology: loop order plus the `twin` map. Endpoint distance is not used.

**The writers.**
- The DXF writer is hand-written R2000, in pattern.rs:178-626. There is no forge-dxf crate.
- The DWG writer is `dwg_layered` → forge-dwg `write.rs`/`bits.rs`. forge-dwg's draft/ and scene/
  serve only the GUI editor and viewer.

**Precision.**
- Nothing is chorded.
- DXF uses `{:?}` (shortest round-trip, pattern.rs:516-519). Arc angles are in degrees, so they
  are exact to one rounding.
- DWG stores raw doubles, so it is bit-exact. `dwgroundtrip` rewrites all 10 exported flats
  byte-identical.
- The worst contour closure gap is 3.6e-15 mm on the shipped parts.
- Bend lines are exact by construction: u = BA/2 in the zone frame (build.rs:3016).
- Tangent lines are rebuilt as centre ± n·BA/2 with a normalised n (pattern.rs:146-162), so they
  are exact only to rounding. They should instead be read from the zone's own boundary, which
  already exists.

**Smaller items:**
- A spline piece whose `stretch` returns None is dropped silently, leaving the contour open
  (build.rs:2996). This should be an error.
- Missing DWG spline weights become 1.0 (forge-dwg/src/write.rs:704).
- A ±1e300 box is used when `flat_in_bend` fails (build.rs:4025).
- The DXF and DWG writers duplicate the layer, note and mark logic (pattern.rs:178-495 against
  663-815).
- The DXF prints exact rounding noise, for example `5.179731399943796e-16` where 0 is meant.
  Strict shop readers may need checking.
- `self.twin[..]` at build.rs:3036 panics if a key is missing.

**Editor only (rank 16).**
- draft/intersect.rs decides tangency at cross < 1e-7, at |dg/ds| < 1e-6 and at cross < 1e-6
  (409, 450, 554), with a `tol·10` dedup.
- `scan_roots` (draft/curve.rs:1450-1484) samples, so two roots between samples are missed.
- The fix is README §2.4's certified isolation. It is low priority, because the export path does
  not use it.

**Tests:**
- forge-dwg: 152 passed;
- `--test pattern`: 7 passed;
- `dwgcheck` and `dwgroundtrip` on the corpus: all the same;
- `fg run --dxf --dwg` on the 10 parts in parts/sheet: all exported.

### 2.10 Sheet: phased plan

**S0. Measure (days):**
- add the probe families (§2.1, §2.2) and shallow.fg as tests that expect today's refusals;
- add `κ ∈ [0, 1]` as a lint;
- write the refold checker (§2.0) in verify/ and run it over every sheet example.

**S1. Exact wins (days):**
- outer contour by sign;
- boxes of arcs by their extremes;
- merge by identity;
- classify cuts by definition;
- tangent lines read from the zone;
- the silent spline drop becomes an error.

**S2. Zone ends from the walls (rank 1).** Compute wall regions first, then build the zone from
the shared slots, with both end conditions. Remove the tangent-line match (1838-1880). The gates:
- the probe families build;
- the refold checker passes;
- bits are identical on rectangles.

**S3. Corners in the true frame (rank 2).** Define reliefs as regions and remove the ±t pads. The
gate: every kind × sense × A ∈ [60°, 135°].

**S4. region.rs decide-once (rank 5).** Port Snap.lean's rule to 2D, with ℚ(√d) comparisons for
arcs, and the Lean reference in verify/forge-exact.

**S5. Certified clash and touch (rank 7), and closed-form checks (rank 12).**

**S6. Lean.** Prove `iso`, `deduction` and `refold` (§2.0), then the region rule.

---

## 3. Assemblies (forge-asm)

**What is principled.** The solve is:
1. names first;
2. union–find into rigid clusters;
3. partitions;
4. Tarjan biconnected blocks;
5. tree edges in closed form, `World(A) = World(B)∘M(values)` (solve.rs:724-828);
6. Newton (minimum-norm Gauss–Newton, column-pivoted QR) only on loop blocks.

There are no restarts, no damping ladders and no "one solver, then another". This is the right
architecture (Kramer, *Solving Geometric Constraint Systems*, MIT Press 1992; Hoffmann, Lomonosov &
Sitharam, decomposition plans, 2001).

### 3.1 Rank 8: "met", "conflict" and rank, each decided several times

**"Met" or "conflict"** is decided at six sites with five thresholds:
- `SAME = 1e-7`, 100× tighter than LINEAR (plan.rs:14, 549-564). It is applied to:
  - fastened cycles and extra grounds, via `frame_gap` (plan.rs:374, 385; joint.rs:248);
  - joints inside a cluster, via `Ends::gap` (solve.rs:1636-1682; joint.rs:188), which is a
    different metric;
  - relations (solve.rs:528-529).
- Newton accepts `worst ≤ 1e-13·size`, or `≤ 1e-9·size` when stuck or capped (solve.rs:14-17,
  1042-1091).

**Rank** is decided at several more sites:
- the Newton step: QR at 1e-12 (1058);
- the reported rank and mobility: QR at 1e-9, relative to the largest initial row norm, with no
  absolute floor (1348; qr.rs:213-231);
- the Redundant rows: a separate modified Gram–Schmidt at 1e-8, in blame order (1462-1486);
- `implied_by`: least squares at 1e-9 (1503);
- the per-joint `free`: `projected_rank` at 1e-9 (1443-1455).

Nothing checks that the number of Redundant rows equals m − rank.

**Reproduced defect.** `qr_noise_rank.rs` is a standalone copy of qr.rs. `rank([1e-17, −3e-17])
= 1`: pure rounding noise is given full rank, because the threshold is only relative. A joint the
mechanism cannot move can report `free = 1`.

**Not invariant under rigid motion.** `size` comes from world-coordinate magnitudes (plan.rs:259-
267; solve.rs:1122-1154). It scales:
- the angular rows and the Jacobian columns;
- the TIGHT and SOLVED limits;
- the ramp unit;
- SAME's angular term.

Moving an assembly 1 m changes these decisions. This is the analogue of the boolean's mirror
finding.

**The diagnosis re-solves differently.** The main solve follows a ramp of up to 256 steps
(1172-1205). The deletion-filter diagnosis and the final re-solve start without the ramp
(1215-1219, 1262), so they can name a false Conflict.

**Singular rows.**
- `Angle` is za·zb − cos θ, whose gradient vanishes at 0 and π (joint.rs:169).
- `Distance{0}` falls back to `Vec3::X` (joint.rs:173).
- These are documented (model.rs:457-462) but not refused.

**Principled method.**
1. **Generic rank, exactly.** Compute it from the constraint graph by bipartite matching on the
   structural Jacobian (Dulmage–Mendelsohn), or by a pebble game where it applies (Jacobs &
   Hendrickson, J. Comput. Phys. 137, 1997). This gives design redundancy with no epsilon.
2. **One numeric decision.** Take one SVD of the block Jacobian in the block's own frame: the
   centroid is the origin and lengths are scaled by the block's extent, so the result is invariant
   under rigid motion. Then:
   - find the numerical rank by a gap test on σ_r/σ_{r+1} with an absolute floor of ε·‖J‖·n;
   - derive redundancy (the left null space), `implied_by`, mobility and per-joint freedom (the
     right null space) from that one factorisation (Golub & Van Loan, *Matrix Computations*,
     4th ed., §5.4);
   - if there is no clear gap, report "near-singular configuration" instead of a rank.
3. **Singular position or real redundancy.** Where the generic rank and the numeric rank differ,
   evaluate J at a seeded random witness configuration to tell which it is.
4. **One closure tolerance,** LINEAR, with angles scaled by the block's extent.
5. **The diagnosis runs through the same ramp as the solve.**
6. **Use singularity-free rows:** for `Angle`, the pair (cross, dot) against (sin θ, cos θ).
   `Distance{0}` becomes a coincidence row.

Golub–Kahan SVD is IEEE plus sqrt and deterministic with a fixed sweep order, so native and wasm
stay identical.

**Lean spec.** Over ℝ:
- `mobility = n − rank J(x*)`. Redundant rows are a basis of the left null space, chosen
  deterministically (least blame name first).
- Predicates:
  - `gap(σ_r, σ_{r+1}) > γ`;
  - `σ_{r+1} ≤ ε‖J‖n`;
  - `genericRank = numericRank`.
- Obligations:
  - in the clear-gap branch, the reported rank equals the rank of every matrix within the error
    ball (Weyl's inequality);
  - otherwise the verdict is "near-singular", never a rank.
- Invariance theorem: every verdict is the same for J and for J under any rigid motion of the
  inputs.

**Exact reference.** Rational Gaussian elimination on J evaluated at the solved pose, with the
pose rounded to dyadics. Use exact sin and cos through rational rotation parameters: Cayley or
quaternions with rational entries. Generate test assemblies with rational quaternion poses, so
the Jacobian is exactly rational. This is about 300 lines in verify/. It judges rank, redundancy
and mobility on the test mechanisms and on random linkages.

### 3.2 Rank 9: interference

**What happens.** Interference is decided by a chain of unrelated tests:
1. A broad phase over occurrence boxes.
2. A mesh narrow phase at depth tolA + tolB + LINEAR (check.rs:1389). That is about 0.02 mm at the
   interface's 0.01 chord.
   - "Sharp" edges are those turning more than 20° (check.rs:78).
   - The inside test is ray parity on a fixed ray (525). An undecided sample moves on to the next.
3. Only for mesh hits that no fit covers: a forge-bool Intersect. The result is "touch" iff vol ≤
   `TOUCH_VOLUME = 1e-6 mm³` (1256, 1486), an absolute volume.

**Consequences:**
- Face-to-face contact is reported Clear.
- `touching` only ever holds mesh false positives.
- Fit-covered pairs are never checked exactly.
- Clearance reports its tolerance without LINEAR (1563), while interference includes it (1492).
- The oracle test's doc comment says it never misses an overlap deeper than the tolerance, but the
  test asserts vol > 0.01 mm³ (tests/checks.rs:2-3 against 388).

**Principled method.** Let the boolean classify the pair once:
- **disjoint:** the exact distance exceeds LINEAR;
- **touch:** the regularised intersection is empty, but the tolerant contact set has positive
  area or length;
- **overlap:** the regularised intersection is non-empty.

The mesh stays as a *conservative* filter only: a pair whose meshes are further apart than the
chord bound plus LINEAR is certainly disjoint. Everything else goes to the boolean.

This depends on the decide-once boolean, and its contact set comes free from the incidence
ledger.

**Lean spec.**
- `overlap ↔ vol(A ∩ B) > 0`
- `touch ↔ ¬overlap ∧ dist(A, B) ≤ LINEAR`
- `clear ↔ dist > LINEAR`
- The filter's obligation: `meshDist > chord_A + chord_B + LINEAR → dist > LINEAR`. This needs a
  certified chord bound, which ties it to §4.

**Exact reference.** forge-exact's polyhedral boolean (Flat.lean) for planar parts, and the staged
reference for the rest (README §6.2).

### 3.3 Assemblies: refusals, exact cases and tests

**Refusals:** BrokenAnchor, WrongReference, SelfJoint, Duplicate*, BadFreedom, BadRelation,
CyclicAssembly, NoGround, PartChanged, Conflict (a minimal set), NotConverged (a 60-iteration
cap), LimitViolated (diag.rs:77-190), `ExactFail` (check.rs:902-919).

**Exact cases:**
- tree placement;
- fastened cycles in closed form (plan.rs:347-390);
- bundle recognition, by bitwise frame equality, checked by property.rs:276;
- relations in closed form (solve.rs:507-539);
- the branch-sign guard (976-1010).

**Minor items:**
- A tree freedom with no value becomes 0.0 (746-748).
- `rows: 0` is mapped to usize::MAX/2 (1691).
- Limits allow 1e-9 rad *or* LINEAR mm on the tree (751-761), but 1e-9 for both in loops (1295).
- The hold-at-limit loop runs at most 4 rounds and reports a non-minimal set (1276-1335).

**Tests:** 42 passed, 0 failed, 0 ignored.

### 3.4 Assemblies: phased plan

- **A0:** add the noise-rank test, a rigid-motion invariance property, and the face-contact
  interference case. All three fail today.
- **A1:** an absolute floor on QR rank; one tolerance (LINEAR) for every "met" test; block-frame
  scaling.
- **A2:** one SVD per block, with every rank question derived from it; diagnosis through the ramp.
- **A3:** generic rank from the graph and a witness check.
- **A4:** interference through the boolean's classification, once the decide-once boolean lands.
- **A5:** Lean, for the rank verdict and invariance.

---

## 4. Mesh (forge-mesh)

### 4.1 Rank 4: the tessellation retry ladder and the silent cap

**What is right.**
- The CDT core uses:
  - exact orient2d;
  - exact `==` duplicate detection (cdt.rs:155-169);
  - typed errors: DuplicatePoint, ConstraintsIntersect, ConstraintThroughPoint, BadLoop
    (cdt.rs:21-50).
- Delaunay flips use the filter-only `certainly_in_circle`. That decides shape, never validity.
- Watertightness is decided once per edge:
  - each edge is sampled once (tessellate.rs:806-834) and shared by both faces (967-977);
  - boundary chords are constraints, and refinement never splits them (cdt.rs:1031, 621-662).

**The ladder (tessellate.rs):**
1. **`cut_finer`** (700-741). On ConstraintsIntersect it re-meshes the *whole body* with that face's
   edges cut 2× finer.
   - It runs up to 9 rounds, or 3 for file bodies (709).
   - From round 5 the tightest circle is no longer doubled (726), for circles tangent at a vertex.
   - At round 4 `pin_touches` runs (744-792): 256² brute-force samples, 8 closest-point
     iterations, 2·LINEAR, a 1e-6 range margin, and a cap of 64 edges.
2. **Cone re-try.** It unrolls when the face reaches the apex (989), and on ConstraintsIntersect
   retries with `unroll = false` (883-895).
3. **The DuplicatePoint nudge.** The uv copy moves 1e-7 toward its loop neighbours, up to 16 tries
   (1330-1350, 2094-2105).
4. **Mend mode,** for file bodies only (667-680; cdt.rs:538-617). It re-triangulates with each
   crossing cut at a float lerp point that is new on *one* face. It is documented as not
   watertight.
5. **The fast-path ladder:** strips → proc_strips → zip → CDT (2002-2091, 1802-1991; cdt.rs:772-841).
   zip declines a triangle when orient2d ≤ 1e-5·side² (cdt.rs:794).
6. **Per-surface constants:**
   - the cylinder and cone rows `2r·step` were measured, not derived (85-111);
   - the sphere and torus factor 0.9/√2 was "chosen by eye" (research/mesh-bounds/README);
   - the cone has five special paths (909-914, 989, 1117-1156, 2075-2087).
7. **Seams and poles.** forge-topo/src/uv.rs:116-200 tries all 3^(runs−1) period shifts. That is
   exponential in the number of pole visits.
8. **The four-triangles patch.** Chords that would duplicate another edge's segment are
   force-split (1232-1260).
9. **The silent cap.** `for _ in 0..200` (cdt.rs:1007) falls through to `Ok` unsettled, so a mesh
   can break its chord bound with no error.
   - Delaunay restore is capped at 64 rounds (501), harmless.
   - Other constants: 1e-12·size (891), a super-triangle at 100·size (893), the arc-step floor
     1e-4 (39), pole noise 1e-8 (125), slack 1 + 1e-9 (508, 1272), and the factors 0.98, 0.9 and
     7.2 (496-613).
10. **Parabola and hyperbola** get 64 segments whatever the chord tolerance (tessellate.rs:51).
    Boundary chords are never refined, so there is no bound on them. Intersection edges use the
    chain's stored sag.

**What "correct" means, and today's status.**
- **(a) Closed, manifold and oriented.** `open_edges()` is empty and there are no degenerate
  triangles (mesh.rs:58-88). Today: checked.
- **(b) Chord bound.** Every point of every triangle is within δ of the surface, and every
  boundary chord is within δ of its edge. Today:
  - checked at centroids and edge midpoints only (research/mesh-bounds);
  - bounded a priori only for grid cells and procedural Cells;
  - not bounded for parabola and hyperbola edges, or after the cap;
  - corner_patch.rs allows 2·chord.
- **(c) No self-intersection.** Today: not checked in forge-mesh. Only verify/forge-proof cert.rs
  checks it, and `certify: Mesh` (M8) is open.
- **(d) Same edge discretisation on both faces.** Today: holds, except in mend mode.

**Principled method.**
1. **One pre-pass per face, replacing cut_finer, the pins and the cone retry.** Map the shared
   edge samples to the face's uv. Then decide every chord–chord crossing exactly:
   - use orient2d on the uv images;
   - the uv images are doubles, so the predicate is exact on the numbers the CDT will actually
     see.
   - split only the crossing chords, at the parameter of the *3D curve* where the uv images
     cross. Find it by certified root isolation on the curve's uv image; no lerp.
   - push the new sample back to the shared edge, so both faces get it: watertight by
     construction.
   This terminates, because each split strictly reduces the number of crossing pairs on a
   compact edge set once the chords are shorter than the edge's separation. With exact
   predicates the separation is positive (Shewchuk, "Delaunay refinement algorithms for
   triangular mesh generation", CGTA 22, 2002, §on segment recovery).
2. **Seams and poles:** pick the period shift per run by the winding of the loop (exact orient2d
   sums) instead of trying all of them.
3. **Chord bound:** use the a-priori bounds already derived for grid and Cells, extended to every
   curve kind: the sagitta for a conic is closed form from curvature, and for parabola and
   hyperbola κ is closed form. Inside triangles, use interval evaluation of the implicit over the
   triangle as a certificate (Boissonnat & Oudot, "Provably good sampling and meshing of
   surfaces", Graph. Models 67, 2005, for the sampling theory).
4. **Never return `Ok` unsettled.** Hitting the cap is an error, or a flag that downstream treats
   as unverified.
5. **Mend mode** stays as an import-only, labelled fallback: it is honest, since it is documented
   as non-watertight. Better still, cut the crossing on both faces through the shared edge, as in
   item 1.

**Lean spec.**
- **Topology.** A mesh is valid iff every directed edge occurs once and has its reverse. That is
  decidable, and it has a trivial proof on the CDT's output if constraints are respected.
- **Crossing pre-pass.** Predicates `orient2d(a, b, c)` and `orient2d(a, b, d)`.
  - The obligation is the loop invariant: after the pass, no two constraint segments cross
    properly.
  - Termination is by a decreasing measure: the number of crossing pairs, with chord length
    bounded below by a positive separation.
- **Chord bound.** For each triangle T, sup over T of `|implicit(p)|/|∇implicit|` ≤ δ, by an
  interval enclosure. The obligation is the enclosure's soundness, which follows from the IEEE
  model already used for the predicates.

**Exact reference.** An exact CDT over the dyadic uv inputs:
- for example, in Lean or as an independent Rust program in verify/ with expansion arithmetic;
- compare the constraint set and topology;
- certify the chord bound with interval arithmetic over each triangle (forge-proof cert.rs already
  does the self-intersection part).
- Effort: about 800 lines, most of it the CDT.

**Tests:**
- 28 passed, 0 failed, 0 ignored.
- Two tests cover the error paths: `a_boundary_past_its_budget_fails_cleanly` and
  `refinement_that_never_settles_fails`.
- No mesh or CDT fuzz target exists. It is fuzzed only indirectly, through forge-lang fuzz,
  forge-proof cert and forge-model mesh_quality.
- Add a CDT fuzz target that checks results against the `check()` in cdt.rs's tests, and a test
  for the cap.

### 4.2 Mesh: legitimate (keep)

- Filter-only incircle flips.
- Fixed-seed BRIO/Morton order (683-713).
- Exact-bit merging of seam copies (1036-1063).
- Exact cone unroll at the apex (1407-1450).
- Derived Taylor bounds for Cells (525-583), checked by `shape_bounds_hold`.
- The analytic grid-cell skip (1262-1273), backed by research/mesh-bounds/check.py.
- The walk-then-scan fallback (cdt.rs:121, 138).

### 4.3 Mesh: phased plan

- **M0:** the cap returns an error; a CDT fuzz target; a 1× chord in corner_patch; curvature
  sampling for parabola and hyperbola.
- **M1:** the per-face crossing pre-pass; delete cut_finer, pin_touches, the cone retry and the
  four-triangles patch as the counts fall to zero (measure as in README Phase 0).
- **M2:** an interval chord certificate; per-run seam shifts.
- **M3:** the exact CDT reference and Lean termination of the pre-pass.

---

## 5. STEP (forge-step)

### 5.1 Rank 6: healing decides "edge on face" five ways

**What happens.** One fact, "this edge lies on its faces", is decided five ways:
1. `measure_gaps`: 9 samples against LINEAR (read.rs:1362-1382).
2. `tighten`: 9 samples, skipping edges at or under 0.5·LINEAR (heal.rs:709). On spline faces it
   skips edges at or under 20·LINEAR (717-720). The 20·LINEAR rule was added for one bench part:
   crank, "80 s … now 0.9 s".
3. The heal limit, `(unc.max(1e-3)·20).min(2e-3·size)` (read.rs:1205-1212). A file that states
   1e-6 can be healed by up to 0.02 mm. research/step/README calls the size cap "tried once and
   not tuned", from one normalised part.
4. `record_deviations`: 16 samples, then 256 with a different projection walk. It sets `edge.tol`
   (read.rs:1162-1205).
5. Export: the stated uncertainty is 1.01·gap, measured only when the file contains fits
   (write.rs:815-834). A healed all-analytic import is re-exported claiming 1e-4.

`.brep` records an uncertainty of 1e-7 (brep.rs:1206) but passes 1e-3 to `finish` (1237).

**Healing that changes topology by distance:**
- `collapse_slivers` (heal.rs:733-785) uses union–find *in edge order*. Edge length is a 4-chord
  estimate, and the threshold is `max(LINEAR, min(2·vtol, limit))`. Faces left with no loops are
  deleted.
- `settle_vertices` (919-947) moves each vertex.
- `chain_edge` rejects at a raw sine of 0.02 (886). It also has the "middle lands further than
  0.25 of a link" reject (903), added for a WHU file.
- Every chain failure is counted as `tangent` (727).

**Retries and fallbacks:**
- BREP_WITH_VOIDS: if the voids are not inward, the solid is rebuilt with the flag flipped, and
  the second try is accepted unchecked (read.rs:1510-1522).
- The mesh check retries at two chord sizes (1240-1247).
- `project_near` falls back past 0.01 mm, but `value_near` past 1e-4 (nurbs.rs:549, 560).
- `nearest()` stops after 64 Newton starts (475).
- Seams try three methods in turn (heal.rs:401-498).
- Hole extents come from 9 samples (428-440).
- The sphere meridian uses `unwrap_or` (537-543).
- `same_sense` is overridden by vertex order (read.rs:996-1001).

**Thresholds for one question each:**
- Pole: `max(1e-5·size, LINEAR)` (nurbs.rs:201-222), 8 samples (heal.rs:1160), and
  `2·max(LINEAR, 1e-4)` (heal.rs:151).
- Distance from the revolution axis: the file's uncertainty in STEP (read.rs:793), but 1e-6 in
  `.brep` (brep.rs:780).
- Periodic: 9 samples, ignoring the file's flags (nurbs.rs:228-242).
- Same parameter modulo period: 1e-9, 1e-7 and 1e-6·period (heal.rs:226, 412, 444).
- `Iso::speed_bound` is 1.5 × the largest of 17 samples, and it is used as a bound (1385).

**Principled method: a tolerant B-rep** (Jackson, "Boundary representation modelling with local
tolerances", ACM Solid Modeling 1995; the Parasolid and OCCT model).
1. **One tolerance source.** ε_file = the stated UNCERTAINTY_MEASURE. It is clamped *below* by
   LINEAR, never multiplied, and refused or warned if it exceeds a stated fraction of the part.
2. **One incidence decision per (edge, face)** and per (vertex, edge): "the curve lies within
   ε_e of the surface". ε_e is the smallest certified enclosure of the distance, found by
   interval evaluation of the distance over the edge's parameter range on the NURBS or analytic
   surface. No 9 or 16 or 256 samples.
3. Every downstream decision uses that ε_e: tighten, collapse, chain and export.
4. **Healing is deterministic and invariant to order:**
   - run union–find over vertex pairs whose tolerance balls intersect, in name order (as
     Snap.lean's `rep`);
   - apply a spread cap, which refuses when a class grows past k·ε_file;
   - represent each class by the least-named member, never by a moved midpoint;
   - recompute vertices from their carriers where possible, as in Snap.lean's `vertexAt`.
5. **Voids.** Orient by the computed signed volume of each shell. That is exact in sign from the
   divergence theorem with certified bounds. The file's flag is then a hint, not a retry.
6. **Export** states the largest carried ε_e, not 1e-4.

**Lean spec.**
- An imported body is valid with tolerances ε iff every edge curve lies in the ε_e-neighbourhood
  of both its faces, every vertex lies within ε_v of its edges, and the tolerant faces bound a
  regular set.
- Predicates:
  - `dist_enclosure(e, f) ≤ ε_e`;
  - `ballsMeet(v, w)`;
  - `spreadOk`.
- Obligations:
  - the healed result is invariant to entity order (the analogue of `rep_order`);
  - every merged vertex lies within the cap of every member;
  - no face is deleted unless its loops all collapsed below ε.

**Exact reference.** For analytic-only files (planes, cylinders, cones, spheres, tori), the
README §6.2 staged reference applies directly once the tolerant entities are rounded to dyadics.
For NURBS, use interval enclosures of the edge–surface distance as the judge. That is about 400
lines, reusing nurbs.rs's evaluation with outward rounding.

**Tests and data.**
- 42 passed, 0 failed, 0 ignored.
- The repo's 14 `.step` files are all written by ForgeCAD, including `examples/sheetmetal/vendor/
  bracket.step` despite the path. One IGES test file exists.
- The 6514-file and 3206-file corpora are kept outside the repo, so the foreign-file failures the
  research notes describe cannot be reproduced here.
- `stepread` reads bracket, cam, hinge_bracket, rail (230 faces, 52 ms) and door (694 faces,
  1.37 s) clean.
- ForgeCAD's own door export reads back with 10 tolerant edges, the largest 6.5e-5 mm. That is a
  self-round-trip that does not close exactly.
- The first step is a pinned subset of the external corpus (licence permitting), checked in or
  fetched by a script.

### 5.2 STEP export (mostly exact)

**What is exact:**
- Analytic entities carry the kernel's numbers (write.rs:175-290).
- Floats are written shortest round-trip, and a test checks they read back bit-exact (69-87,
  962-972).

**What is not:**
- **Cones** are written via `atan2(sin_a, cos_a)` (258), with no bit-exact test.
- **Procedural, intersection and imported spline geometry** is re-fitted as a cubic B-spline
  within 1e-4 (223-321). An imported NURBS is never written back as it was. It should be written
  verbatim.
- **Fit tolerance:** `SHEET_TOL = 1e-7` is tuned to OCCT's checker (fit.rs:13-18).
- **The round-trip test** (tests/roundtrip.rs:71-112) covers 6 analytic parts. It does not cover
  surface parameters, text identity or splines.

The Lean spec for export is "read ∘ write = id" on analytic entities, bit for bit. The
predicates: the kind of entity, and `is_imported_nurbs`.

### 5.3 STEP: refusals and exact cases

**Refusals:** unknown curves and surfaces by name, including offsets (read.rs:567, 688); generators
that cannot be moved exactly; non-EDGE_LOOP bounds; bad weights, knots or degrees; the 1 km box;
thin-tube tori; units; cycles; scaled `.brep` locations; IGES 144/143. Assemblies are flagged
`unplaced`. Integral and mesh disagreement gives an `unverified` warning.

Duplicate entity ids silently keep the first (part21.rs:144-149). That should be reported.

**Exact cases:** conics as exact rational splines (845-866); exact extrusion and revolution nets
(869-934); spindle tori (668-686, 803-835); closed-form plane intersections for edges
(heal.rs:787).

### 5.4 STEP: phased plan

- **T0:** check in a pinned foreign corpus subset; make the export→import→export test cover text
  identity and splines; write imported NURBS verbatim.
- **T1:** one tolerance source and per-entity certified ε; export states it.
- **T2:** deterministic union–find healing in name order, with a spread cap; voids by signed
  volume.
- **T3:** the reference judge (interval distance) and Lean invariance.

---

## 6. Sketch (forge-sketch): confirmed fine, with minor notes

**The solver.**
- One Levenberg–Marquardt run per linked group (solve.rs:536-617). The damping ladder is standard
  LM, not a retry.
- `drag` tries a hard hold, then a weighted solve (257-281). That is a UI choice.

**Minor issues:**
- `SOLVED = 1e-10` against `CONFLICT = 1e-7` (5-7): a residual between them is "not converged"
  with no conflict named.
- The rank tolerance is 1e-9 × a scale that differs per question (382, 633, 645, 670-675). Use
  one scale, the sketch's extent, as for the assemblies.
- profile.rs:
  - Endpoints are clustered greedily at LINEAR, so it depends on order (132-141). It does strictly
    refuse open chains and branches.
  - The crossing test's `near = 1e-6 + sqrt(2r·1e-8)` (454) does not match SOLVED.
  - A raw sine threshold of 1e-3 (496).
  - Fix: union–find in name order, as §2.4.
- project.rs nudges:
  - ellipse axes re-solved to put both ends on the curve, up to 10·LINEAR (117-139);
  - a circle centre moved onto the bisector (262-271).
  These make up downstream for the boolean's rank-1 issue. They go when the decide-once boolean
  lands.
- sketch.rs:528: the spline end-tie at 1e-6, where everything else uses LINEAR.

**Tests:** 23 passed, 0 failed, 0 ignored.

---

## 7. Cross-crate summary

**The patterns found:**
- **"Built for one configuration".** Sheet ranks 1 and 2. These are found only by probes, because
  every test uses rectangles.
- **"One fact, many thresholds".** Region, asm, STEP, sketch and mesh. This is the 2D, numeric and
  import form of the boolean's rank-1 issue. Each is fixed by the same §6.1 rule (decide once,
  union–find in name order, spread cap), and Snap.lean already proves the 3D planar case.
- **"Retry with another reading".** Mesh cut_finer and the cone retry, STEP voids, the sheet
  relief rebuild. Each goes once its predicate is decided exactly.
- **"Approximate where exact is cheap".** The outer contour by box, interference by volume, many
  sampled distances. These are fixes of days each.

**Recommended order across crates:**
1. **S1 (sheet exact wins) and the mesh cap error.** Days. The outer-contour fix removes the only
   wrong result found.
2. **S2 and S3 (sheet zone ends and corners).** These remove the largest reproduced refusal class.
3. **The refold checker and A0, M0, T0 tests.** These measure.
4. **S4, and the region rule in Lean alongside the boolean's decide-once spec.** One proof
   pattern, three users: boolean, region and healing.
5. **A2, M1, T2.**
6. **Certified clash, chord and distance enclosures,** which share one interval library in
   verify/ and one in forge-math.
