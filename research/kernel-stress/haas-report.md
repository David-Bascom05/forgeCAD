# The kernel under the Haas machine models

Every STEP in the Haas solid-model zips (branch `haas-3d-models`, `research/haas-models/`)
through the kernel: import, `check`, mass properties, a mesh at the default tolerance, four
sections, and booleans, each op watched for hangs (60 s) and panics. Made by
`tools/kernel-stress/run.sh` (rerun it on the zips to regenerate this page); the harness is
`verify/forge-stress`.

A boolean's answer is judged three ways: `check` on the result, its mesh watertight, and its
volume against an oracle. Subtract and intersect with the same tool must add back to the
solid; union and intersect of two solids must add to their sum. An `Ok` that fails any of
these is **wrong**: worse than a refusal, as nothing downstream can tell.

## Totals

- zips: 3; STEP files: 3 (0.14 GB)
- imports: 3 read, 
- solids read: 204 (40719 faces); failing `check`: 3; mesh not watertight: 6; mass fails: 6; entities refused: 1
- ops on the solids that passed: 1438; wrong: 10; refused: 102; panics: 0; hangs: 0; crashes: 0
- Parasolid files beside them (not read: no x_t reader): 3 (0.08 GB)

## How to read this

- **Rerun**: `tools/kernel-stress/run.sh` (all zips, about 2 h on 4 cores at `-j 3`; `-z DIR` for
  zips on disk, zip names to run some). It fetches `haas-3d-models` and reads each zip with
  `git show`, never checking the branch out; each zip is unpacked to a scratch dir and deleted.
- **Corpus**: `research/kernel-stress/corpus.tsv` holds the smallest cases of each failure class
  (zip, file, solid entity, op, what it gave). `python3 -I tools/kernel-stress/corpus.py run`
  replays them from the branch and says pass or fail now. Small extracted solids (each under
  1 MB) are committed as fixtures in `kernel/forge-step/tests/data/haas/`, with
  `tests/haas_fixtures.rs`: fixed cases are live tests, open ones are `#[ignore]` tests that say
  what they should do (`cargo test -p forge-step --test haas_fixtures -- --ignored`). Larger
  cases stay in the manifest.
- **Shrink one**: `python3 -I tools/kernel-stress/extract.py FILE.step ENT[,ENT] out.step` writes
  one solid (or several) as a STEP of its own; `target/release/forge-stress out.step --only OP`
  runs one op on it.
- Assemblies: the reader does not apply placements (`asm` in the per-machine table), so each part
  stands where it was modelled. Unions of a file's own solids are therefore of parts that happen
  to overlap in their own frames, which is still a fair test of the boolean.

## Fixed in this pass (forge-step, with reproducers in `kernel/forge-step/tests/stress_haas.rs`)

1. **Tolerances read with no room.** An edge (and a healed vertex) a file leaves off its faces got
   a tolerance of exactly the measured gap, so the same body moved anywhere, and any boolean of
   it, failed `check` (EdgeOffFace / VertexOffEdge at exactly that gap). Now 1e-6 relative + 1 nm
   for vertices; for edges 1e-3 relative + 1 nm, as `check` re-measures a trimmed edge at other
   points and a gap peaking between the reader's 256 samples came out 1.7e-5 larger (GM2-5AX
   Edge-Flange3, union: a wrong Ok). Tests: `a_tolerant_edge_still_checks_once_moved`,
   `a_tolerance_covers_the_gap_between_the_samples`.
2. **A rim between two seams at other angles.** A whole circle between two faces of revolution
   (SolidWorks writes turned parts with no seam edges) had its one vertex moved to each face's
   seam in turn; the second move broke the first face's loop. 24 solids failed `check`
   (LoopBroken + Euler), and 46 more were refused as "too long to measure" because their mass
   and mesh ran over the broken loops (08-1949's Sketch-Pattern parts). The circle is now split
   when its vertex is in use. Test: `a_rim_between_two_seams_at_other_angles_is_split_not_moved`.

3. **An anchor far out on an axis or a plane.** A cylinder's or line's placement may stand
   anywhere on its axis and a plane's anywhere on it; SolidWorks writes some 500 m out, and 18
   solids were refused as outside the model box (TR210, VF-3YT50, UMC-1250, ST-20..). Such an
   origin is moved to the place nearest the world origin. Test:
   `a_far_anchor_on_an_axis_or_a_plane_is_read`.
4. **A surface model's closed shells.** TRT160's STEP holds no solid, only 182 OPEN_SHELLs; an
   open shell whose every edge lies between two faces is now read as a solid (2 of them close;
   the rest are refused as sheets, which is the 630 OPEN_SHELL refusals below). Test:
   `a_surface_model_shell_that_closes_is_a_solid`.
5. **A lost reference refused the whole file.** SolidWorks writes a lost reference as
   `#18446744073709551615` (-1); the parser refused the file: VF-2TR, VF-10/40, VF-6/50 TR and
   ST-30/35 read nothing. Now it is null and only its entity is refused (VF-10/40: 56 solids).
   Test: `a_lost_reference_refuses_its_solid_not_the_file`.

Fixes 3 to 5 came after the full sweep below was run, so its figures still count those cases.
The "before" figures for 1 and 2 are in the first sweep (16 zips): 24 LoopBroken solids, 47
refusals for time, 2 EdgeOffFace wrong Oks; after: 0, 1 and 0.

## Who owns what is left

| class | owner | why |
|---|---|---|
| boolean wrong Ok: FaceNoLoops, mass fails on the answer, mesh open on the answer | FLAT-SITES (planar/drift) | Oks whose bodies are invalid; the cases are in the corpus |
| boolean refused: NoClosedForm, SplitCrossed, InconsistentClassification, SplitThroughSingular, ZeroThickness, UntouchedOnOther | FALLBACK | refusals: the fallback path's work |
| section wrong (open loops; outline and holes swapped; 65) | forge-agent `features::section` (mesh-based) | Outline and hole swapped: FIXED (baa56d12). A crossing on a seam went onto the first edge the two faces share; a cylinder's two halves share both seams, so half the crossings jumped to the far one and the arc swept a whole turn wrong (HRT160SP #71564 sec_x -2301 mm2, now 5668.92 against a slab's 5669.97; HRTA6 #87677 sec_z37). Open loops (K111-K113) are another cause, still open |
| mesh open on a valid boolean answer at a micro-edge (corpus K050: HFL-3015D-6 #456315 union with a copy at an ordinary offset; the answer checks and weighs exactly, its mesh has one crack on plane face 32 between vertices 1.65e-5 mm apart, at every chord) | forge-mesh | an edge shorter than the tolerance meshed differently on its two sides |
| mesh not watertight on a valid solid | forge-mesh | a crack at one chord (0.01) and not at 0.05 or 0.002, near a NURBS face's near-singular point |
| import: FaceWrongSide, LoopsCross, "loops draw no region", whole closed spline with no edges | forge-step (KERNEL-STRESS, next) | a zero-area sliver sphere face (a file defect to heal); a torus face whose loop wraps nearly a turn; a three-sided NURBS corner patch whose fourth side is collapsed to a pole, its loop crossing itself in parameters; a sphere face of four arcs that draws no region in parameters; an O-ring as one closed NURBS face with no edges (only spheres are read whole) |
| boolean wrong Ok: VertexOffEdge after a bore (cylinder-cylinder intersection edge, vertex 2.1e-5 off at tol 1e-5) | FLAT-SITES (drift) | fixture `bore_vertex_drift_40faces.step` |
| boolean wrong Ok at the tolerance boundary (EdgeOffFace 1e-5 + 8.8e-14 on union/int with a copy) | FLAT-SITES | the copies stand 1e-5 (the box's padding) off their twins: adversarial, not ordinary use |
| memory: booleans with a moved copy past 4 GB, or killed by the kernel (rc 137) | forge-bool / forge-topo `check` | PARASOLID sees `check` alone past 8 GB in `subdivide_pieces` / `Nurbs::implicit`, unbounded by the evaluation budget |
| refused "too long to measure" (124) | forge-mesh / speed | two causes: the CDT spends the whole budget recovering the boundary of a hairline face (a cylinder r 2.5 um, 437 mm long, HRT160-2 #13299: refused in 0.3 s); and real spline cost (desktop lathe #106846: healing 24 M of 33 M evaluations, mass the rest, 8 s) |
| import: ball-end sphere patch, "no place on the rim for a second meridian" (9) | forge-step | the loop runs through the pole along two meridians; choosing meridians the rim meets makes a degenerate edge instead, so left as a refusal |
| mass silently wrong (no error): a closed cubic x linear B-surface face with 4 loops integrates to area 6e-9 against a mesh's 2732, the body's volume -216 against 15956 (from PARASOLID, an Onshape x_t with no licence: not committed) | forge-topo `mass` | the worst kind: a wrong answer that says nothing |
| a periodic procedural face's box is its whole domain (+-1e6 mm along a closed extrusion's v) | forge-topo `procedural_box` | from PARASOLID (fixture kernel/forge-step/tests/data/xt/onshape/extrude.x_t on parasolid/xt-reader): bound v by the edges where v is not periodic; loose boxes also cost culling |
| import: entity missing or of another kind (Cobot) | none: a file defect | an EDGE_LOOP lists an EDGE_CURVE, an EDGE_CURVE's vertex is an ORIENTED_EDGE; refusing is right |

## Parasolid (.x_t)

PARASOLID (session_01WJtawyMeaFg2YQpEPzN4eu) now owns a native reader; what follows is this
sweep's first look at the files, kept for them.

Every zip holds the same machine as Parasolid text (`.x_t`, schema SCH_3000298 and later,
SolidWorks 2018 to 2026); there is no reader for it, so none of it is read here. What one would
take, from the files:

- **Format**: a text transmit file: a header, then a flat stream of nodes (BODY, SHELL, FACE,
  LOOP, FIN, EDGE, VERTEX, POINT, the geometry), each a type number and its fields by the
  schema. Newer files embed only the differences from a base schema, so the reader needs the base
  field lists (Siemens publishes them in the "Parasolid XT Format Reference"). Units are metres;
  every edge and vertex may carry a tolerance (Parasolid's tolerant modelling).
- **Geometry**: planes, cylinders, cones, spheres, tori map to ours exactly; B_SURFACE and
  B_CURVE to our splines; SP_CURVE (a curve in a surface's parameters) and TRIMMED_CURVE are
  simple; INTERSECTION curves (with their chart of points) map to `Curve::Intersection`;
  BLENDED_EDGE (an exact rolling-ball blend) and OFFSET_SURF, SWEPT_SURF, SPUN_SURF need our
  procedural surfaces. This is the gain: STEP gives the blends as fitted B-splines, x_t gives them
  exact.
- **Topology**: FIN is our coedge; faces carry their own sense. Parasolid, like these STEP files,
  puts no seam edges on closed faces, so the reader would go through the same `Proto` and
  `heal.rs` (seams, tolerances, vertex settling) as STEP does, and gain its fixes.
- **Estimate**: a parser and node table (about 1.5k lines), schema handling for the embedded
  deltas (0.5k), geometry mapping (1.5k, most of it the procedural surfaces), topology into
  `Proto` and the existing `finish` (0.5k): about 4k lines, two to three weeks, about the size of
  forge-step's STEP reader. Binary `.x_b` is the same with another tokenizer. Worth doing after the
  rewrite: STEP carries the same shapes and is read now; x_t would add exact blends and be
  smaller and faster to read (the HA5C2 x_t is 0.9 MB to the STEP's 2.5 MB).

## Failure classes

| class | detail | count | ops | first seen |
|---|---|---:|---|---|
| boolean refused | Internal(SplitCrossed) | 40 | union_over 11, int_over 11, union_over2 7, int_over2 7, box_sub 2, box_int 2 | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 1 (#57887, 3902 faces) union_over |
| boolean refused | Internal(InconsistentClassification) | 32 | union_over2 6, int_over2 6, union_over 4, int_over 4, box_sub 3, box_int 3, bore_sub 3, bore_int 3 | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 1 (#57887, 3902 faces) box_sub |
| boolean refused | Unsupported(NoClosedForm) | 26 | union_over2 7, int_over2 7, union_over 4, int_over 4, union_pair 2, int_pair 2 | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 1 (#57887, 3902 faces) union_over2 |
| mass fails | face N: the face's loops draw no region | 6 |  | st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP solid 26 (#207817 NONE) |
| mesh not watertight | open edges | 6 |  | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 32 (#281128 Defeatured1) |
| unverified on import | its volume cannot be worked out (the face's loops draw no region), so nothing it measures  | 6 |  | st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP solid 26 (#207817 NONE) |
| boolean refused | Internal(HoleOutside) | 4 | union_over 1, int_over 1, union_over2 1, int_over2 1 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 27 (#259140, 675 faces) union_over |
| boolean wrong | mesh not watertight: open N vol N | 4 | box_sub 1, union_beside 1, union_over 1, union_over2 1 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 15 (#87260, 610 faces) box_sub |
| solid fails check | FaceWrongSide | 3 |  | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 32 (#281128 Defeatured1) |
| boolean wrong | check: FaceWrongSide | 2 | int_over 1, int_pair 1 | st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP solid 92 (#673016, 2558 faces) int_over |
| boolean wrong | check: VertexOffEdge | 2 | union_over2 1, int_over2 1 | st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP solid 92 (#673016, 2558 faces) union_over2 |
| boolean wrong | mass: face | 2 | union_pair 2 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 45 (#275604, 2613 faces) union_pair27 |
| refusal: MANIFOLD_SOLID_BREP | its surfaces would take too long to measure | 1 |  | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #449308 |
| solid fails check | LoopsCross | 1 |  | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 32 (#281128 Defeatured1) |

## Worst 10

1. st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP solid 92 (#673016, 2558 faces) int_over: check: FaceWrongSide:4 | FaceWrongSide { face: FaceId(472), coedge: CoedgeId(2918) } [face cone]
2. st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP solid 92 (#673016, 2558 faces) union_over2: check: VertexOffEdge:1 | VertexOffEdge { edge: EdgeId(5000), vertex: VertexId(3281), distance: 6.222464467167736e-5 } [edge Line tol 1e-5 on cylinder plane]
3. umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 15 (#87260, 610 faces) box_sub: mesh not watertight: open 3 vol 161076389.82488403
4. umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 45 (#275604, 2613 faces) union_pair27: mass: face 153: too many runs between singular points

## Slowest 20 ops

| ms | op |
|---:|---|
| 33633 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 15 (#87260, 610 faces) union_over2 |
| 29194 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 15 (#87260, 610 faces) union_over |
| 28671 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 1 (#74491, 921 faces) union_over2 |
| 25567 | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 1 (#57887, 3902 faces) union_over |
| 23139 | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 1 (#57887, 3902 faces) int_over |
| 22491 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 15 (#87260, 610 faces) int_over |
| 21634 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 15 (#87260, 610 faces) int_over2 |
| 16966 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 1 (#74491, 921 faces) int_over2 |
| 15659 | st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP solid 19 (#198489, 2336 faces) int_over2 |
| 15204 | st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP solid 19 (#198489, 2336 faces) union_over2 |
| 13353 | st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP solid 19 (#198489, 2336 faces) int_over |
| 13225 | import st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP |
| 12991 | st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP solid 19 (#198489, 2336 faces) union_over |
| 12120 | import umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP |
| 10363 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 27 (#259140, 675 faces) int_over2 |
| 9932 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 27 (#259140, 675 faces) union_over2 |
| 8369 | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 15 (#167334, 16 faces) union_over2 |
| 7583 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP solid 13 (#85431, 513 faces) union_over |
| 6849 | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 15 (#167334, 16 faces) union_over |
| 6118 | st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP solid 74 (#647540, 921 faces) union_over2 |

## Memory: peak resident, largest 10 files

| peak MB | STEP MB | file |
|---:|---:|---|
| 237 | 64.0 | st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP |
| 205 | 27.9 | TR160_10-2019 / TRT160_STEP_10-2019.STEP |
| 200 | 49.3 | umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP |

## What the files hold

Faces read, by surface kind: plane 19067, cylinder 11799, cone 4715, nurbs 2827, sphere 1341, torus 970.

Edges read, by curve kind: Line 55726, Circle 33087, Procedural 12869, Intersection 2451, Hyperbola 294, Parabola 40, Ellipse 26.

Geometric entities in the files' text: `EDGE_CURVE` 108950, `LINE` 59003, `CIRCLE` 33697, `B_SPLINE_CURVE_WITH_KNOTS` 12658, `CYLINDRICAL_SURFACE` 12111, `CONICAL_SURFACE` 4878, `SURFACE_SIDE_STYLE` 3525, `SURFACE_STYLE_FILL_AREA` 3525, `SURFACE_STYLE_USAGE` 3525, `B_SPLINE_SURFACE_WITH_KNOTS` 1834, `SPHERICAL_SURFACE` 1340, `TOROIDAL_SURFACE` 993, `COORDINATED_UNIVERSAL_TIME_OFFSET` 346, `CLOSED_SHELL` 253, `MANIFOLD_SOLID_BREP` 179, `ADVANCED_BREP_SHAPE_REPRESENTATION` 52, `ORIENTED_CLOSED_SHELL` 48, `BREP_WITH_VOIDS` 26.

## Per machine

st: import status. asm: placements not applied (assembly parts each at home). Columns after
`edges` count solids (invalid, leaky mesh, mass fails), refused entities, then ops (wrong,
refused, panic, hang, crash) over all ops run.

| zip | STEP | MB | st | read s | peak MB | solids | faces | edges | invalid | leaky | mass | refused ents | wrong | refused | panic | hang | crash | ops | wall s | asm |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| TR160_10-2019 | TRT160_STEP_10-2019.STEP | 27.9 | ok | 3.2 | 205 | 34 | 7743 | 19130 | 1 | 1 | 0 | 0 | 0 | 24 | 0 | 0 | 0 | 486 | 112 | 1 |
| st-28l_solid_models_02_2025 | ST-28L- 2_2025.STEP | 64.0 | ok | 13.2 | 237 | 98 | 19144 | 49087 | 1 | 2 | 4 | 0 | 4 | 44 | 0 | 0 | 0 | 473 | 171 | 0 |
| umc-500_ss_solid_models_04_2026 | UMC-500_4_2026.STEP | 49.3 | ok | 12.1 | 200 | 72 | 13832 | 36276 | 1 | 3 | 2 | 1 | 6 | 34 | 0 | 0 | 0 | 479 | 272 | 1 |

