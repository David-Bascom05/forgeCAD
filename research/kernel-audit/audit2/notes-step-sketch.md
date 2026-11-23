# Audit: kernel/forge-step (in depth) and kernel/forge-sketch (brief)

Read-only, /home/user/forgeCAD, branch claude/repo-review-mbdrlj-audit2, 2026-11-22. No files changed.
LINEAR = 1e-5 mm, ANGULAR = 1e-11, MIN_LENGTH = LINEAR (forge-math/src/tol.rs:9-20).

## 0. Short answer

forge-step is mostly honest about what it reads: unknown entities are refused by name, damaged files end in
an error, bodies it cannot verify carry a warning. Its case-patching is in **healing**. One fact, "does this edge
lie on its faces, and how far off", is measured five times. Each measurement uses its own sampler and its own
threshold, and none of them is the file's stated uncertainty. Around that sit order-dependent vertex and sliver
merging, a void-orientation retry, and several thresholds for "is this a pole or periodic". The research notes
(research/step/README.md) show that some of these constants were set against particular files (crank, jack, a
normalised part). Two premises in the brief are wrong:
- The kernel *does* hold NURBS. B-spline surfaces are read as a private `Procedural` NURBS (nurbs.rs:1-2,
  read.rs:752), and B-spline curves are read through `Spline::from_nurbs` (read.rs:488). B-spline surfaces are
  not refused.
- No NURBS-to-analytic recognition exists anywhere. fit.rs is the *export* direction: it fits procedural and
  intersection geometry as cubic B-splines (fit.rs:1-2). So the "is this B-spline a cylinder" tolerance does not
  exist.

forge-sketch: the earlier "fine" verdict is confirmed. The solver has no restarts. There are small threshold
inconsistencies, and two nudges in project.rs that make up for boolean output being slightly off.

## 1. Case-patching in forge-step, ranked

### 1.1 "Is this edge on its faces?" decided five ways (highest)
| site | sampler | threshold |
|---|---|---|
| measure_gaps read.rs:1362-1382 (stats) | 9 samples, surface.distance | LINEAR |
| Proto::off + tighten heal.rs:684-690, 709 | 9 samples | skip if <= 0.5*LINEAR |
| tighten spline skip heal.rs:717-720 | same 9 | skip if <= **20*LINEAR** on a spline face |
| heal limit read.rs:1210-1212 | - | `(unc.max(1e-3)*20).min(2e-3*size)` |
| record_deviations read.rs:1162-1205 | 16, then 256 samples, project_near walk | LINEAR, else edge.tol = dev |
| export uncertainty write.rs:815-834 | re-read with curve_off_surface | 1.01*gap, only if any fit exists |

- The file's uncertainty only enters through heal_limit, and heal_limit floors it at 1e-3. A file that states
  1e-6 is still healed by up to 0.02 mm. The `.brep` reader records uncertainty 1e-7 (brep.rs:1206) but passes
  1e-3 to finish (brep.rs:1237).
- research/step/README.md, "Import: refused for the mesher's sake": the 2e-3*size cap was "tried once and not
  tuned" and came from one normalised part.
- The 20*LINEAR skip was added for one bench part, crank: "took over 80 s to read back ... now 0.9 s" (same
  README, Round trip paragraph). This trade of speed for accuracy is visible in the door below: 10 tolerant
  edges at up to 6.5e-5 mm.
- An edge between the 0.5x and 20x thresholds that is skipped then gets `tol = dev` from a different sampler
  (record_deviations). Its tolerance therefore depends on which of two samplers saw the larger gap.

### 1.2 Healing that changes topology by distance
- collapse_slivers heal.rs:733-785 merges vertices with union-find in edge order. Edge length is a 4-chord
  polyline (heal.rs:752-760), and an edge merges when `len <= max(LINEAR, min(2*vtol, limit))` (heal.rs:763-764).
  The result depends on order. Merged vertices are not moved until settle_vertices. Faces whose loops empty are
  deleted (heal.rs:777-778).
- settle_vertices heal.rs:919-947 puts each vertex at the box middle of its edges' ends and gives it a ball
  radius. That is vertex merging with tolerance.
- chain_edge heal.rs:859-916 has a near-tangency cutoff of raw sine 0.02 (about 1.15 deg; heal.rs:886) and a
  "middle lands further than 0.25 of a link" reject (heal.rs:903-906). The reject was added for one WHU failure
  (README "Two findings from the files left"). Every chain failure, whatever its cause (refine failed, limit
  exceeded, backwards step), is counted as `tangent` (heal.rs:727, read.rs:1407).
- closed_form_edge heal.rs:787-843 is exact plane∩surface: a legitimate case. It accepts a curve when 9 samples
  lie within `limit`.

### 1.3 Retries and fallbacks
- BREP_WITH_VOIDS void flag: build with `void_flip = true`. If the voids are not inward, rebuild with false
  (read.rs:1510-1522, "files differ on what a void's .F. turns"). The second try is accepted unchecked.
- mesh_agrees read.rs:1240-1247 meshes at 5e-4*diag, else 5e-3*diag. The agreement test is 1% plus chord*curved
  area (read.rs:1278-1281).
- Nurbs projection, nurbs.rs:542-566:
  - project_near falls back to the global search when d^2 > 1e-4 (0.01 mm). value_near falls back when
    d^2 > 1e-8 (1e-4 mm). Those are two thresholds for one question.
  - nearest() stops after 64 Newton starts (nurbs.rs:475) or when d^2 <= 1e-20, so it does not guarantee the
    nearest point.
  - newton: 12 iterations, steps clamped to a third of the domain (nurbs.rs:354-391).
- seam_start heal.rs:401-498 tries three things in turn: a clear vertex, then the middle of the widest gap,
  then the middle of the longest edge. Hole extents come from 9 samples per edge (heal.rs:428-440), so a hole
  can be missed. two_lunes tries 32 meridian candidates, then `unwrap_or(u_first)` even if that one is not clear
  (heal.rs:537-543).
- The file's `same_sense` is overridden by vertex order (read.rs:996-1001). edge_range snaps to curve ends
  within 1e-9 (read.rs:1134).

### 1.4 "Pole, periodic, same parameter" with uncorrelated thresholds
- Pole:
  - nurbs on_pole: `max(1e-5*size, LINEAR)` (nurbs.rs:201-222).
  - pole_of on a Procedural: 8 samples within LINEAR (heal.rs:1160-1166).
  - winding: `2*max(LINEAR, 1e-4)` (heal.rs:151).
  - revolution on_axis: the file uncertainty in STEP (read.rs:793 and 829, default 1e-3), but 1e-6 in brep
    (brep.rs:780).
- Periodic: find_closed takes 9 samples within LINEAR and ignores the file's flags (nurbs.rs:228-242).
- Same parameter (mod period): 1e-9 in pick_vertex_on (heal.rs:226), 1e-7 in seam_start (heal.rs:412), and a
  margin of 1e-6*period (heal.rs:444).
- Iso::speed_bound is "1.5 * max of 17 samples" (heal.rs:1385-1390). It is used as a bound but is not one.

### 1.5 Vendor or OCC-specific
- fit::SHEET_TOL = 1e-7 is tuned to OCCT's checker behaviour (fit.rs:13-18, "the rack's door").
- Onshape's self-placement in assembly_asks (read.rs:1300) is fine.
- The 1e-3 default uncertainty when a file has none (read.rs:254) is plausible.
- The placement hint 0.99 (read.rs:344-350) is the standard's default: fine.

### 1.6 Parse leniency
- Duplicate entity ids keep the first (part21.rs:64, 144-149). The duplicates are recorded in File but not
  surfaced in Stats or Imported (grep finds no other use).
- Binary strings are skipped (part21.rs:477).

## 2. Honest refusals
- Unknown curves or surfaces are refused by name (read.rs:567, 688). This covers OFFSET_SURFACE, OFFSET_CURVE,
  PCURVE-only edges and the like.
- Other refusals:
  - an extrusion or revolution whose generator "cannot be moved exactly" (read.rs:762);
  - a non-EDGE_LOOP bound (read.rs:1044);
  - missing same_sense or orientation;
  - weights <= 0 (read.rs:482);
  - degree out of range (read.rs:430-436);
  - too many control points or knots;
  - multiplicity outside 1..64;
  - a point outside the 1 km box (read.rs:303);
  - a ring torus with too thin a tube (read.rs:807);
  - units that cannot be read (read.rs:1460);
  - a reference cycle (MAX_WRAP);
  - a scaled or mirrored .brep location (brep.rs:1222);
  - IGES 144/143 trimmed surfaces, mirror and scale matrices (iges.rs:221, 389).
- VERTEX_LOOP is skipped silently; healing makes the apex or pole (read.rs:1041-1043).
- Assemblies are not applied but are flagged with `unplaced` (read.rs:1302-1338).
- Bodies whose integrals and mesh disagree are imported with `unverified` (read.rs:1215-1290). Only a budget run
  out refuses one.
- B-spline surfaces are NOT refused (nurbs.rs).

## 3. Legitimate exact special cases
- Conics are made exactly as rational splines through Spline::ellipse (read.rs:845-866).
- The extrusion is an exact degree-1 net (read.rs:869-888), and the revolution an exact 9-point rational net
  (read.rs:890-934).
- A spindle torus becomes Torus::spindle, else an exact turned arc (read.rs:668-686, 803-835).
- plane∩surface closed form (heal.rs:787).
- These rebuild topology without moving geometry:
  - split_closed_splines;
  - add_seams with exact iso curves (heal.rs:1173-1264; the Iso curve at heal.rs:1310-1412);
  - whole_face sphere meridians;
  - split_fans ("nothing moves", heal.rs:950);
  - outer_first.

## 4. Export
- Analytic entities are written with the kernel's own numbers: LINE, CIRCLE, ELLIPSE, PARABOLA, HYPERBOLA, and
  PLANE, CYLINDRICAL, CONICAL, SPHERICAL, TOROIDAL (write.rs:175-221, 241-290).
- Reals use Rust's shortest-round-trip `{}` and `{:E}` with a point added (write.rs:69-87). This is tested
  bit-exact (write.rs:962-972).
- Not exact:
  - The cone is written as `atan2(sin_a, cos_a)` (write.rs:258) and re-read through sin_cos. It is not
    guaranteed to round-trip bit for bit, and nothing tests it.
  - All Procedural and Intersection curves and surfaces are re-fitted as C1 cubic Hermite B-splines within
    tol 1e-4 (write.rs:223-239, 291-321). That includes imported NURBS: an imported B-spline is never written
    back verbatim. Fit errors are sampled, not bounded.
  - The stated uncertainty only accounts for fits (write.rs:820, gated on fit count > 0). An imported body with
    tolerant edges or vertices (up to heal_limit, 0.02 mm), all analytic, is re-exported claiming
    max(LINEAR, 1e-4).
- Round-trip test (tests/roundtrip.rs:71-112):
  - It covers 6 analytic parts and checks entity counts, vertex coordinate bits, volume and area to 1e-9, and
    that no tolerant edges appear.
  - It does not check surface or curve parameters bit for bit. It does not check export∘import∘export text
    identity. It does not cover spline round trips.
  - stepread on RACK-301_door.step (ForgeCAD's own export, with fits) reads back with 10 tolerant edges, the
    largest 6.5e-5.

## 5. History, docs, datasets
- `git log -- kernel/forge-step` is one squashed commit, 979190b "Install ForgeCAD launcher and update from latest
  main". The --all history (219 commits) has no forge-step fix commits, so no per-file fix hashes can be quoted.
- The substitute is research/step/README.md, which records constants and branches added per failing file or set:
  - net_of weights divided twice (fixed);
  - heal limit 0.2% "tried once and not tuned";
  - line_curvatures 1e-8;
  - chain_edge 0.25 sag reject (WHU);
  - 20*LINEAR spline skip (crank, 80 s to 0.9 s);
  - fit_surface rim measured by parameters (jack, 0.0103 mm);
  - `foot` clamped at the domain end (revolved spline, 2-6% mass);
  - LoopsCross band;
  - the tessellate_tolerant split.
- Test doc comments are descriptive, not tied to files: "A block with a cavity: the void's shell reads as a void
  whichever way a file's flag turns it" (roundtrip.rs:290), "A shallow dome turned from an arc whose circle
  crosses the axis" (roundtrip.rs:242), "The bottom's plane 0.3 above its edges" (unverified.rs:38).
- Datasets in the repo: 14 .step files, all written by ForgeCAD (FILE_NAME 'ForgeCAD'):
  examples/rack18u/asm/order/*.step (13) and examples/sheetmetal/vendor/bracket.step (despite the "vendor" path).
  There is also tests/data/block_with_hole.igs.
- No foreign-vendor STEP file is in the repo or exercised by tests. The corpora (CADFS/Onshape, WHUCAD/CATIA,
  Fusion: 6514 files; 3206 FreeCAD .brep) are outside it (research/step/README.md:7, 49, 119).

## 6. Runs
- `cargo test -p forge-step`: 42 passed, 0 failed, 0 ignored. By binary: unit 19, brep 5, iges 4, mesh_bound 3,
  roundtrip 7, seams 1, unverified 3, doc 0.
- stepread (debug build):
  - bracket.step: 13 faces, vol 2503.14, no problems;
  - RACK-313 cam: 9 faces;
  - RACK-311 hinge_bracket: 15 faces;
  - RACK-201 rail: 230 faces, 52 ms;
  - RACK-301 door: 694 faces, read 1.37 s, all 2.2 s, 10 tolerant edges with maxtol 6.5e-5, no refusals or
    problems.
  - All the analytic files have deviation around 1e-14.

## 7. forge-sketch (brief): verdict confirmed, with minor points
- **No solver restarts.** There is one Levenberg-Marquardt run per linked group (solve.rs:536-617). The lambda
  ladder 1e-6 → 1e12 is standard LM, not a retry. Solved groups are reused when their rows and parameters are
  bit-identical to the previous solve (solve.rs:451-509).
- drag (solve.rs:257-281) tries "point held on target", else a weighted solve from the target. That is a UI
  fallback and acceptable.
- SOLVED = 1e-10 and CONFLICT = 1e-7 (solve.rs:5-7). A residual in (1e-10, 1e-7] is NotConverged but no
  constraint is named as conflicting. That is a band with no verdict.
- The rank tolerance is 1e-9 times a scale, and the scale differs per question:
  - implied() takes each row's own max (solve.rs:382);
  - analyze base takes the global max row (solve.rs:633);
  - fully_constrained takes `scale.max(1.0)` (solve.rs:645);
  - redundant takes the largest remaining row (solve.rs:670-675).

  Four scalings of one rank test. The dense reference analyze_dense uses yet another (the max entry).
  Ties use exact SOLVED equality (solve.rs:87).
- profile.rs:
  - chain_loops clusters nodes greedily at LINEAR (profile.rs:132-141), which is order dependent. It then
    strictly refuses OpenChain and Branch (profile.rs:156-162): honest.
  - The self-intersection check uses `near = 1e-6 + sqrt(2 r 1e-8)` (profile.rs:454). The comment says a solve
    to "~1e-9 mm", but SOLVED is 1e-10.
  - A crossing needs a raw sine > 1e-3 (profile.rs:496).
  - Arc angle EPS is 1e-9 (profile.rs:244), and the collinear test 1e-12*scale (profile.rs:270).
- project.rs makes two nudges for boolean output ("ends a boolean settled off the curve"):
  - ellipse axes are re-solved so both ends lie on the curve, allowed to move up to 10*LINEAR, else refused
    (project.rs:117-139);
  - a circle's centre is moved onto the bisector when the ends are not equidistant within 1e-11
    (project.rs:262-271).

  These are downstream patches of the boolean near-tolerance issue (kernel-audit rank 1).
- sketch.rs:528: the spline end-tie gap is 1e-6 where everything else uses LINEAR.
- `cargo test -p forge-sketch`: 23 passed, 0 failed, 0 ignored.
