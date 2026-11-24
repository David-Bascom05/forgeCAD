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

- zips: 146; STEP files: 196 (7.08 GB)
- imports: 191 read, 5 error
- solids read: 11558 (1571990 faces); failing `check`: 168; mesh not watertight: 316; mass fails: 299; entities refused: 849
- ops on the solids that passed: 52317; wrong: 187; refused: 3439; panics: 0; hangs: 39; crashes: 4
- Parasolid files beside them (not read: no x_t reader): 196 (3.40 GB)

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
| section wrong (open loops; outline and holes swapped) | forge-agent `features::section` (mesh-based) | the mesh is watertight and consistently oriented, the loops come out signed wrong |
| mesh not watertight on a valid solid | forge-mesh | a crack at one chord (0.01) and not at 0.05 or 0.002, near a NURBS face's near-singular point |
| import: FaceWrongSide, LoopsCross, "loops draw no region", whole closed spline with no edges | forge-step (KERNEL-STRESS, next) | a zero-area sliver sphere face (a file defect to heal); a torus face whose loop wraps nearly a turn; a three-sided NURBS corner patch whose fourth side is collapsed to a pole, its loop crossing itself in parameters; a sphere face of four arcs that draws no region in parameters; an O-ring as one closed NURBS face with no edges (only spheres are read whole) |
| boolean wrong Ok: VertexOffEdge after a bore (cylinder-cylinder intersection edge, vertex 2.1e-5 off at tol 1e-5) | FLAT-SITES (drift) | fixture `bore_vertex_drift_40faces.step` |
| boolean wrong Ok at the tolerance boundary (EdgeOffFace 1e-5 + 8.8e-14 on union/int with a copy) | FLAT-SITES | the copies stand 1e-5 (the box's padding) off their twins: adversarial, not ordinary use |
| memory: booleans with a moved copy past 4 GB, or killed by the kernel (rc 137) | forge-bool / forge-topo `check` | PARASOLID sees `check` alone past 8 GB in `subdivide_pieces` / `Nurbs::implicit`, unbounded by the evaluation budget |
| refused "too long to measure" (124) | forge-mesh / speed | two causes: the CDT spends the whole budget recovering the boundary of a hairline face (a cylinder r 2.5 um, 437 mm long, HRT160-2 #13299: refused in 0.3 s); and real spline cost (desktop lathe #106846: healing 24 M of 33 M evaluations, mass the rest, 8 s) |
| import: ball-end sphere patch, "no place on the rim for a second meridian" (9) | forge-step | the loop runs through the pole along two meridians; choosing meridians the rim meets makes a degenerate edge instead, so left as a refusal |
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
| boolean refused | Unsupported(NoClosedForm) | 2358 | union_over 674, int_over 674, box_sub 267, box_int 267, bore_sub 164, bore_int 164, union_pair 73, int_pair 73, union_beside 1, int_beside 1 | 08-1949 / 08-1949_STEP_04_02_2024.STEP solid 5 (#22973, 502 faces) box_sub |
| refusal: OPEN_SHELL | a sheet, not a solid: N edges not between two faces | 630 |  | hfl-3015_solid_models_02_2026 / HFL-3015_02_2026.STEP #639632 |
| boolean refused | Internal(InconsistentClassification) | 523 | union_over 137, int_over 137, box_sub 60, box_int 60, bore_sub 42, bore_int 42, int_pair 23, union_pair 22 | 08-1949 / 08-1949_STEP_04_02_2024.STEP solid 5 (#22973, 502 faces) union_pair73 |
| boolean refused | Internal(SplitCrossed) | 358 | int_over 150, union_over 149, box_sub 16, box_int 16, int_pair 6, union_pair 5, union_beside 5, int_beside 5, bore_sub 3, bore_int 3 | APL_ST-10-25_2019-05_r1 / APL_ST-10-25_STEP_2019-05_r1.STEP solid 63 (#181991, 338 faces) union_over |
| mesh not watertight | open edges | 315 |  | 08-1949 / 08-1949_STEP_04_02_2024.STEP solid 89 (#521387 Imported1) |
| mass fails | face N: the face's loops draw no region | 295 |  | HA2TS_2019-08 / HA2TS_STEP_11-2019.STEP solid 1 (#26474 NONE) |
| unverified on import | its volume cannot be worked out (the face's loops draw no region), so nothing it measures  | 295 |  | HA2TS_2019-08 / HA2TS_STEP_11-2019.STEP solid 1 (#26474 NONE) |
| boolean refused | ZeroThickness | 140 | union_over 87, int_over 30, union_pair 13, int_pair 5, box_sub 3, box_int 2 | APL_ST-10-25_2019-05_r1 / APL_ST-10-25_STEP_2019-05_r1.STEP solid 220 (#683013, 101 faces) union_over |
| solid fails check | FaceWrongSide | 126 |  | 08-1949 / 08-1949_STEP_04_02_2024.STEP solid 160 (#983428 Imported1) |
| refusal: MANIFOLD_SOLID_BREP | its surfaces would take too long to measure | 114 |  | HRT160-2-XT_2019-08 / HRT160-2-STEP_2019-08.STEP #13299 |
| boolean wrong | check: FaceWrongSide | 36 | union_over 12, union_pair 9, int_over 9, int_pair 5, union_beside 1 | HRP3_10-2020 / HRP3_STEP_10-2020.STEP solid 39 (#424881, 15 faces) union_over |
| section wrong | N open loops | 33 | section 33 | 08-1949 / 08-1949_STEP_04_02_2024.STEP solid 9 (#49364, 86 faces) sec_x |
| section wrong | negative area | 32 | section 32 | HRT160-2-XT_2019-08 / HRT160-2-STEP_2019-08.STEP solid 0 (#643, 274 faces) sec_x |
| refusal: ORIENTED_EDGE | the entity is missing or of another kind | 29 |  | Haas_5kg_Cobot_Package_Model / Haas 5kg Cobot Package.STEP #1359504 |
| solid fails check | LoopsCross | 28 |  | 08-1949 / 08-1949_STEP_04_02_2024.STEP solid 22 (#96088 Imported1) |
| boolean wrong | mesh not watertight: open N vol N | 23 | union_beside 19, union_over 2, int_over 2 | HRP3_10-2020 / HRP3_STEP_10-2020.STEP solid 37 (#420351, 204 faces) union_beside |
| boolean refused | Internal(HoleOutside) | 22 | union_over 11, int_over 11 | hkm-1_solid_models_02_2026 / HKM-1_02_2026.STEP solid 48 (#48625, 63 faces) union_over |
| boolean wrong | check: VertexOffEdge | 20 | union_over 5, int_over 5, bore_sub 4, bore_int 4, box_sub 1, box_int 1 | ST-20-25_Reboot_Solid_Model_2019-05r2 / ST20-25_Reboot_2-axis_STEP_2019-05.STEP solid 58 (#320952, 1332 faces) union_over |
| boolean refused | Unsupported(SplitThroughSingular) | 18 | box_sub 8, box_int 8, union_over 1, int_over 1 | HA5C2-T_11-2019 / HA5C2-T_STEP_11-2019.STEP solid 19 (#38155, 4 faces) box_sub |
| refusal: CARTESIAN_POINT | outside the N km model box | 18 |  | ST-20-25_Reboot_Solid_Model_2019-05r2 / ST20-25_Reboot_2-axis_STEP_2019-05.STEP #300337 |
| solid fails check | VertexTolerance | 15 |  | VM-6_SOLID_MODELS_1_2022 / VM-6_STEP__1_2022.step solid 10 (#502701 Defeatured50) |
| refusal: VERTEX_POINT | the entity is missing or of another kind | 14 |  | Haas_5kg_Cobot_Package_Model / Haas 5kg Cobot Package.STEP #681642 |
| union_over: hang | over the op timeout | 14 |  | HRP3_10-2020 / HRP3_STEP_10-2020.STEP s16:union_over |
| unverified on import | its volume and area work out to N and N, but drawn they come to N and N: one of the two is | 14 |  | 08-1949 / 08-1949_STEP_04_02_2024.STEP solid 160 (#983428 Imported1) |
| refusal: EDGE_CURVE | the entity is missing or of another kind | 13 |  | ST20LY-25LY_3-2021 / ST20LY-25LY_STEP_3-2021.STEP #859867 |
| boolean wrong | mass: face | 11 | union_pair 8, union_over 2, int_over 1 | HA2TS_2019-08 / HA2TS_STEP_11-2019.STEP solid 0 (#26141, 296 faces) int_over |
| int_over: hang | over the op timeout | 11 |  | HRP3_10-2020 / HRP3_STEP_10-2020.STEP s16:int_over |
| refusal: BREP_WITH_VOIDS | its surfaces would take too long to measure | 11 |  | ds-30y_solid_models_12_2022 / DS-30Y_STEP__12_2022.STEP #396013 |
| solid fails check | EdgeDegenerate | 11 |  | VM-6_SOLID_MODELS_1_2022 / VM-6_STEP__1_2022.step solid 10 (#502701 Defeatured50) |
| boolean refused | Internal(Uncut) | 10 | box_sub 3, box_int 3, union_over 2, int_over 2 | TR310_10-2019 / TR310_STEP_10-2019.STEP solid 6 (#215375, 673 faces) union_over |
| boolean wrong | check: FaceNoLoops | 10 | int_over 10 | GM2-5AX_Solid_Model / 10000100371 lightweight v1.STEP solid 240 (#650885, 1520 faces) int_over |
| boolean wrong | check: EdgeOffFace | 9 | int_over 3, union_pair 2, union_over 2, bore_sub 1, union_beside 1 | HRTA5_2019-08 / HRTA5_Solid_Model_2019-08.STEP solid 32 (#79185, 1292 faces) union_pair27 |
| oracle wrong | volumes do not add up | 9 | beside_sum 4, box_sum 2, pair_sum 1, over_sum 1, bore_sum 1 | hc10_solid_models_05_2025 / HC10_5_2025.STEP solid 15 (#64039, 176 faces) beside_sum |
| refusal: MANIFOLD_SOLID_BREP | no place on the rim for a second meridian | 8 |  | TR200Y_10-2019 / TR200Y_STEP_10-2019.STEP #71139 |
| boolean refused | Internal(RetryInvalid) | 6 | int_over 3, union_pair 1, int_pair 1, box_sub 1 | TH240_1-2021 / TH240_STEP_1-2021.STEP solid 0 (#27696, 1819 faces) union_pair4 |
| refusal: MANIFOLD_SOLID_BREP | a whole spline surface with no edges | 5 |  | 08-1949 / 08-1949_STEP_04_02_2024.STEP #525424 |
| box_int: hang | over the op timeout | 4 |  | HRTA5_2019-08 / HRTA5_Solid_Model_2019-08.STEP s2:box_int |
| box_sub: hang | over the op timeout | 4 |  | HRTA5_2019-08 / HRTA5_Solid_Model_2019-08.STEP s2:box_sub |
| import error | line N (byte N): an instance number that is empty or over N bits | 4 |  | st-30-35_(2-axis)_2009-2020+ / ST-30-35_STEP_2020-03.STEP |
| boolean wrong | check: EdgeUses | 3 | int_pair 1, union_over 1, int_over 1 | HRTA5_2019-08 / HRTA5_Solid_Model_2019-08.STEP solid 32 (#79185, 1292 faces) int_pair15 |
| refusal: ADVANCED_FACE | the entity is missing or of another kind | 3 |  | VM-6_SOLID_MODELS_1_2022 / VM-6_STEP__1_2022.step #527815 |
| refusal: EDGE_CURVE | a reference is missing | 3 |  | robot_pkg-1_7_front_solid_models_06_2025 / ROBOT PKG-1_7KG_FRONT_06_2025.STEP #34102 |
| union_over: memory | over N MB resident | 3 |  | gm-1-5ax_solid_models_07_2026 / GM-1-5AX 07-01-2026.STEP s72:union_over |
| boolean refused | Internal(NoNormal) | 2 | union_over 1, int_over 1 | umc-1500-duo_2020-07 / UMC-1500-DUO_STEP_2020-07.STEP solid 78 (#215476, 285 faces) union_over |
| boolean refused | Unsupported(UntouchedOnOther) | 2 | union_over 1, int_over 1 | GM2-5AX_Solid_Model / 10000100371 lightweight v1.STEP solid 181 (#499513, 1138 faces) union_over |
| int_over: crash rc 137 |  | 2 |  | gm-1-5ax_solid_models_07_2026 / GM-1-5AX 07-01-2026.STEP s2:int_over |
| mass fails | face N: singular points jump both ways | 2 |  | tr160-2_2020-03 / TR160-2_STEP_2020-03.STEP solid 24 (#165965 10000117196-1-solid1) |
| mass fails | face N: too many runs between singular points | 2 |  | tl-3b_solid_models_07_2026 / tl-3b_mld_07-2026.STEP solid 64 (#530793 Imported4) |
| solid fails check | EdgeUses | 2 |  | vs-3_solid_models_06_2024 / VS-3_STEP__6_2024.STEP solid 15 (#478011 NONE) |
| solid fails check | ShellDisconnected | 2 |  | vs-3_solid_models_06_2024 / VS-3_STEP__6_2024.STEP solid 15 (#478011 NONE) |
| union_over: crash rc 137 |  | 2 |  | gm-1-5ax_solid_models_07_2026 / GM-1-5AX 07-01-2026.STEP s2:union_over |
| unverified on import | its volume cannot be worked out (singular points jump both ways), so nothing it measures i | 2 |  | tr160-2_2020-03 / TR160-2_STEP_2020-03.STEP solid 24 (#165965 10000117196-1-solid1) |
| unverified on import | its volume cannot be worked out (too many runs between singular points), so nothing it mea | 2 |  | tl-3b_solid_models_07_2026 / tl-3b_mld_07-2026.STEP solid 64 (#530793 Imported4) |
| boolean wrong | check: LoopsCross | 1 | int_over 1 | hbf-12_solid_models_07_2025 / Haas Bar Feeder 12_7_2025.STEP solid 7 (#24138, 180 faces) int_over |
| bore_int: hang | over the op timeout | 1 |  | umc-400_ss_solid_models_02_2025 / UMC 400_2_2025.STEP s12:bore_int |
| bore_sub: hang | over the op timeout | 1 |  | umc-400_ss_solid_models_02_2025 / UMC 400_2_2025.STEP s12:bore_sub |
| import error | the file holds no solid | 1 |  | TRT160_2019-09 / TRT160_2019-09.STEP |
| int_over: memory | over N MB resident | 1 |  | gm-1-5ax_solid_models_07_2026 / GM-1-5AX 07-01-2026.STEP s72:int_over |
| mesh fails | panic: index out of bounds: the len is N but the index is N at kernel/forge-mesh/src/cdt.r | 1 |  | vs-3_solid_models_06_2024 / VS-3_STEP__6_2024.STEP solid 15 (#478011 NONE) |
| refusal: BREP_WITH_VOIDS | no place on the rim for a second meridian | 1 |  | hcs-80_solid_models_04_2026 / HCS-80_04_2026.STEP #198078 |
| solid fails check | Euler | 1 |  | vs-3_solid_models_06_2024 / VS-3_STEP__6_2024.STEP solid 15 (#478011 NONE) |

## Worst 10

1. 08-1949 / 08-1949_STEP_04_02_2024.STEP solid 9 (#49364, 86 faces) sec_x: 41 open loops
2. GM2-5AX_Solid_Model / 10000100371 lightweight v1.STEP solid 240 (#650885, 1520 faces) int_over: check: FaceNoLoops:1 | FaceNoLoops { face: FaceId(126) } [face plane]
3. GM2-5AX_Solid_Model / 10000100371 lightweight v1.STEP solid 486 (#1273295, 656 faces) sec_z: 23 open loops
4. HA2TS_2019-08 / HA2TS_STEP_11-2019.STEP solid 0 (#26141, 296 faces) int_over: mass: face 75: the face's loops draw no region
5. HRP3_10-2020 / HRP3_STEP_10-2020.STEP solid 37 (#420351, 204 faces) union_beside: mesh not watertight: open 3 vol 111415092.45647408
6. HRP3_10-2020 / HRP3_STEP_10-2020.STEP solid 39 (#424881, 15 faces) sec_x: 2 open loops
7. HRP3_10-2020 / HRP3_STEP_10-2020.STEP solid 39 (#424881, 15 faces) union_over: check: FaceWrongSide:2 | FaceWrongSide { face: FaceId(14), coedge: CoedgeId(64) } [face cone]
8. HRT160-2-XT_2019-08 / HRT160-2-STEP_2019-08.STEP solid 0 (#643, 274 faces) sec_x: negative area
9. HRT160SP_10-2019 / HRT160SP_STEP_10-2019.STEP solid 2 (#47023, 1050 faces) union_beside: mesh not watertight: open 3 vol 17040944.819945004
10. HRT160SP_10-2019 / HRT160SP_STEP_10-2019.STEP solid 3 (#71564, 265 faces) sec_x: negative area

## Slowest 20 ops

| ms | op |
|---:|---|
| 53358 | ST30L-LY-35L-LY__2020-06 / ST30L-35L-LY_STEP_2020-06.x_t.STEP solid 100 (#1012290, 1345 faces) union_beside |
| 51489 | ST-20-25_Reboot_Solid_Model_2019-05r2 / ST20-25_Reboot_2-axis_STEP_2019-05.STEP solid 130 (#685618, 988 faces) union_over |
| 51015 | ST20LY-25LY_3-2021 / ST20LY-25LY_STEP_3-2021.STEP solid 229 (#1015581, 1353 faces) union_beside |
| 49107 | st-30(y)-35(y)_2009-2020+ / ST-30Y-35Y_STEP_2020-03.STEP solid 36 (#165334, 1345 faces) union_beside |
| 47787 | APL_ST-10-25_2019-05_r1 / APL_ST-10-25_STEP_2019-05_r1.STEP solid 118 (#330785, 2708 faces) int_over |
| 47292 | ST-20Y-25Y_Reboot_2019-03 / ST-20Y-25Y_STEP_2019-03.STEP solid 9 (#42957, 988 faces) union_over |
| 44059 | ST-20-25_Reboot_Solid_Model_2019-05r2 / ST20-25_Reboot_2-axis_STEP_2019-05.STEP solid 130 (#685618, 988 faces) int_over |
| 43012 | import Haas_5kg_Cobot_Package_Model / Haas 5kg Cobot Package.STEP |
| 39810 | ST-20Y-25Y_Reboot_2019-03 / ST-20Y-25Y_STEP_2019-03.STEP solid 9 (#42957, 988 faces) int_over |
| 35450 | desktop_mill_solid_models_old_04_2024 / desktop_mill_solid_models_04_2024.STEP solid 14 (#85799, 416 faces) union_over |
| 35368 | APL_ST-10-25_2019-05_r1 / APL_ST-10-25_STEP_2019-05_r1.STEP solid 118 (#330785, 2708 faces) union_over |
| 34643 | desktop_mill_solid_models_old_04_2024 / desktop_mill_solid_models_04_2024.STEP solid 14 (#85799, 416 faces) int_over |
| 32162 | import hab3232_solid_models_06_2026 / HAB3232_6_2026.STEP |
| 31065 | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 14 (#167334, 16 faces) union_over |
| 30475 | import 08-1949 / 08-1949_STEP_04_02_2024.STEP |
| 27829 | TR160_10-2019 / TRT160_STEP_10-2019.STEP solid 14 (#167334, 16 faces) int_over |
| 27223 | mass hab3232_solid_models_06_2026 / HAB3232_6_2026.STEP solid 13 (#137081 NONE) (1410 faces) |
| 26592 | import rpl_modular_umc_750_solid_models_09_2026 / rpl_modular_umc-750_09-2026 1.STEP |
| 26440 | import st-10_st-15_solid_models_10_2023 / st-10_st-15_solid_models_10_2023.STEP |
| 26006 | hfl-3015_solid_models_02_2026 / HFL-3015_02_2026.STEP solid 16 (#242230, 12478 faces) union_over |

## Memory: peak resident, largest 10 files

| peak MB | STEP MB | file |
|---:|---:|---|
| 1535 | 37.9 | cl-1_solid_models_11_2022 / CL-1_STEP__11_2022.step |
| 947 | 118.0 | GM2-5AX_Solid_Model / 10000100371 lightweight v1.STEP |
| 935 | 65.3 | HRP3_10-2020 / HRP3_STEP_10-2020.STEP |
| 699 | 134.3 | VM-6_SOLID_MODELS_1_2022 / VM-6_STEP__1_2022.step |
| 695 | 107.5 | vf-6_50_tr_solid_models_2_2022 / VF-6_50_TR_EXTENDED Z-AXIS CLEARANCE_STEP__2_2022.step |
| 691 | 56.2 | hfl-6512t_solid_models_08_2026 / HFL-6512T Tube Laser 08-21-2026.STEP |
| 545 | 201.1 | st-10_st-15_solid_models_10_2023 / st-10_st-15_solid_models_10_2023.STEP |
| 496 | 116.8 | mm_solid_models_01_2024 / mm_solid_models_01_2024.STEP |
| 476 | 59.4 | vf-2_solid_models_1_2023 / VF-2_STEP__1_2023.step |
| 409 | 29.3 | hfl-1313-3_solid_models_07_2026 / HFL-1313-3 07-22-2026.STEP |

## What the files hold

Faces read, by surface kind: plane 834640, cylinder 441691, cone 124466, nurbs 119610, torus 33885, sphere 17698.

Edges read, by curve kind: Line 2487161, Circle 1071492, Procedural 414475, Intersection 62355, Hyperbola 15956, Ellipse 2118, Parabola 1032.

Geometric entities in the files' text: `EDGE_CURVE` 4975372, `LINE` 3064570, `CIRCLE` 1281182, `SURFACE_SIDE_STYLE` 595943, `SURFACE_STYLE_FILL_AREA` 595943, `SURFACE_STYLE_USAGE` 595943, `CYLINDRICAL_SURFACE` 539194, `B_SPLINE_CURVE_WITH_KNOTS` 484444, `CONICAL_SURFACE` 143286, `B_SPLINE_SURFACE_WITH_KNOTS` 84438, `TOROIDAL_SURFACE` 39130, `SPHERICAL_SURFACE` 21369, `CLOSED_SHELL` 18232, `MANIFOLD_SOLID_BREP` 11404, `COORDINATED_UNIVERSAL_TIME_OFFSET` 10963, `ORIENTED_CLOSED_SHELL` 4680, `ADVANCED_BREP_SHAPE_REPRESENTATION` 3733, `OPEN_SHELL` 1353, `BREP_WITH_VOIDS` 678, `MANIFOLD_SURFACE_SHAPE_REPRESENTATION` 75, `SHELL_BASED_SURFACE_MODEL` 75.

## Per machine

st: import status. asm: placements not applied (assembly parts each at home). Columns after
`edges` count solids (invalid, leaky mesh, mass fails), refused entities, then ops (wrong,
refused, panic, hang, crash) over all ops run.

| zip | STEP | MB | st | read s | peak MB | solids | faces | edges | invalid | leaky | mass | refused ents | wrong | refused | panic | hang | crash | ops | wall s | asm |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 08-1949 | 08-1949_STEP_04_02_2024.STEP | 128.2 | ok | 30.5 | 326 | 164 | 22405 | 54685 | 2 | 2 | 0 | 2 | 1 | 102 | 0 | 0 | 0 | 375 | 123 | 492 |
| APL_ST-10-25_2019-05_r1 | APL_ST-10-25_STEP_2019-05_r1.STEP | 78.6 | ok | 4.8 | 297 | 307 | 20870 | 56494 | 0 | 0 | 0 | 0 | 0 | 10 | 0 | 0 | 0 | 421 | 121 | 296 |
| GM2-5AX_Solid_Model | 10000100371 lightweight v1.STEP | 118.0 | ok | 10.5 | 947 | 535 | 36580 | 91147 | 1 | 1 | 0 | 0 | 4 | 26 | 0 | 0 | 0 | 412 | 70 | 528 |
| HA2TS_2019-08 | HA2TS_STEP_11-2019.STEP | 8.5 | ok | 1.4 | 38 | 12 | 2975 | 7401 | 0 | 0 | 3 | 0 | 1 | 16 | 0 | 0 | 0 | 162 | 21 | 3 |
| HA5C-T_11-2019 | HA5C-T_STEP_11-2019.STEP | 7.6 | ok | 1.0 | 33 | 11 | 2113 | 5225 | 0 | 2 | 0 | 0 | 0 | 12 | 0 | 0 | 0 | 165 | 8 | 0 |
| HA5C2-T_11-2019 | HA5C2-T_STEP_11-2019.STEP | 18.9 | ok | 2.4 | 85 | 154 | 5015 | 12497 | 1 | 1 | 0 | 0 | 0 | 24 | 0 | 0 | 0 | 414 | 32 | 0 |
| HA5C2_2019-08 | HA5C2_STEP_2019-08.STEP | 2.5 | ok | 0.1 | 37 | 4 | 688 | 1713 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 77 | 3 | 0 |
| HA5C3_2019-08 | HA5C3_STEP_2019-08.STEP | 4.2 | ok | 0.2 | 58 | 1 | 1143 | 2848 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 16 | 2 | 0 |
| HA5C4_2019-08 | HA5C4_STEP_2019-08.STEP | 4.2 | ok | 0.2 | 64 | 1 | 1175 | 2972 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 4 | 0 |
| HA5C_11-2019 | HA5C_STEP_11-2019.STEP | 7.6 | ok | 1.1 | 34 | 11 | 2113 | 5225 | 0 | 2 | 0 | 0 | 0 | 12 | 0 | 0 | 0 | 165 | 8 | 0 |
| HRP3_10-2020 | HRP3_STEP_10-2020.STEP | 65.3 | ok | 1.7 | 935 | 60 | 3199 | 7932 | 0 | 0 | 0 | 0 | 4 | 21 | 0 | 4 | 0 | 408 | 1064 | 13 |
| HRT100_11-2019 | HRT100_STEP_11-2019.STEP | 7.4 | ok | 0.7 | 39 | 2 | 2133 | 5164 | 1 | 1 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 16 | 2 | 0 |
| HRT160-2-XT_2019-08 | HRT160-2-STEP_2019-08.STEP | 5.5 | ok | 0.4 | 72 | 4 | 658 | 1642 | 0 | 0 | 0 | 1 | 2 | 0 | 0 | 0 | 0 | 68 | 1 | 4 |
| HRT160SP_10-2019 | HRT160SP_STEP_10-2019.STEP | 6.7 | ok | 0.5 | 64 | 4 | 1688 | 4561 | 0 | 0 | 0 | 0 | 3 | 5 | 0 | 0 | 0 | 80 | 15 | 0 |
| HRT210SP_10-2019 | HRT210SP_STEP_10-2019.STEP | 6.6 | ok | 0.6 | 40 | 9 | 1964 | 4948 | 0 | 1 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 150 | 2 | 0 |
| HRTA5_2019-08 | HRTA5_Solid_Model_2019-08.STEP | 9.4 | ok | 0.5 | 74 | 42 | 2800 | 6853 | 0 | 0 | 0 | 0 | 3 | 19 | 0 | 2 | 0 | 410 | 140 | 162 |
| HRTA6_2019-08 | HRTA6_STEP_2019-08.STEP | 9.8 | ok | 1.0 | 43 | 38 | 2866 | 7100 | 0 | 1 | 0 | 0 | 1 | 11 | 0 | 2 | 0 | 416 | 141 | 130 |
| Haas_5kg_Cobot_Package_Model | Haas 5kg Cobot Package.STEP | 144.4 | ok | 43.0 | 375 | 173 | 25338 | 61606 | 4 | 7 | 0 | 11 | 0 | 92 | 0 | 0 | 0 | 380 | 127 | 375 |
| ST-20-25_Reboot_Solid_Model_2019-05r2 | ST20-25_Reboot_2-axis_STEP_2019-05.STEP | 63.8 | ok | 5.3 | 191 | 136 | 15803 | 38341 | 2 | 2 | 2 | 2 | 3 | 32 | 0 | 0 | 0 | 408 | 159 | 135 |
| ST-20Y-25Y_Reboot_2019-03 | ST-20Y-25Y_STEP_2019-03.STEP | 63.3 | ok | 5.6 | 183 | 152 | 13109 | 32146 | 2 | 2 | 2 | 1 | 0 | 26 | 0 | 0 | 0 | 413 | 141 | 174 |
| ST20LY-25LY_3-2021 | ST20LY-25LY_STEP_3-2021.STEP | 113.6 | ok | 9.9 | 284 | 274 | 17177 | 41237 | 3 | 3 | 3 | 19 | 2 | 30 | 0 | 0 | 0 | 410 | 98 | 38 |
| ST30L-LY-35L-LY__2020-06 | ST30L-35L-LY_STEP_2020-06.x_t.STEP | 144.2 | ok | 7.2 | 345 | 161 | 14926 | 37045 | 0 | 0 | 0 | 2 | 3 | 32 | 0 | 0 | 0 | 408 | 176 | 16 |
| Small_VF_APL_2019-10-r1 | Small_VF_APL_STEP_2019-10-r1.STEP | 24.1 | ok | 1.3 | 71 | 135 | 6374 | 16623 | 0 | 0 | 0 | 5 | 2 | 18 | 0 | 0 | 0 | 417 | 11 | 239 |
| TH240_1-2021 | TH240_STEP_1-2021.STEP | 13.2 | ok | 1.1 | 98 | 5 | 3106 | 7426 | 0 | 0 | 0 | 0 | 1 | 13 | 0 | 0 | 0 | 92 | 20 | 0 |
| TR160_10-2019 | TRT160_STEP_10-2019.STEP | 27.9 | ok | 1.3 | 88 | 33 | 3841 | 9313 | 1 | 1 | 0 | 1 | 6 | 8 | 0 | 0 | 0 | 422 | 96 | 1 |
| TR200Y_10-2019 | TR200Y_STEP_10-2019.STEP | 17.9 | ok | 1.3 | 90 | 10 | 4763 | 12296 | 0 | 1 | 1 | 1 | 0 | 10 | 0 | 0 | 0 | 166 | 12 | 0 |
| TR210_10-2019 | TR210_STEP_10-2019.STEP | 23.8 | ok | 0.7 | 83 | 5 | 2812 | 7295 | 0 | 1 | 0 | 1 | 0 | 4 | 0 | 0 | 0 | 66 | 4 | 1 |
| TR310_10-2019 | TR310_STEP_10-2019.STEP | 24.8 | ok | 1.4 | 105 | 9 | 4071 | 10225 | 0 | 0 | 0 | 1 | 1 | 14 | 0 | 0 | 0 | 161 | 15 | 1 |
| TRT100_10-2019 | TRT_STEP_10-2019.STEP | 22.5 | ok | 2.6 | 62 | 5 | 1710 | 4667 | 1 | 1 | 0 | 0 | 0 | 14 | 0 | 0 | 0 | 70 | 7 | 0 |
| TRT160_2019-09 | TRT160_2019-09.STEP | 14.6 | error | 0.1 | 35 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| TRT310_SOLID_MODELS_05_2023 | TRT310_STEP__5_2023.step | 20.3 | ok | 8.0 | 88 | 4 | 4262 | 10678 | 1 | 2 | 1 | 0 | 0 | 2 | 0 | 0 | 0 | 36 | 14 | 0 |
| UMC-1250_2021-03 | UMC-1250_2021-03 updated.STEP | 76.8 | ok | 12.0 | 214 | 277 | 18365 | 46243 | 1 | 3 | 1 | 3 | 5 | 24 | 0 | 0 | 0 | 413 | 51 | 17 |
| VF-3YT50_2019-11 | VF-3YT50_STEP_2019-11.x_t.STEP | 68.4 | ok | 7.8 | 192 | 302 | 20207 | 49695 | 1 | 2 | 1 | 7 | 0 | 22 | 0 | 0 | 0 | 415 | 29 | 572 |
| VM-6_SOLID_MODELS_1_2022 | VM-6_STEP__1_2022.step | 134.3 | ok | 22.0 | 699 | 44 | 24106 | 62696 | 1 | 4 | 5 | 5 | 0 | 38 | 0 | 2 | 0 | 404 | 455 | 33 |
| cl-1_solid_models_11_2022 | CL-1_STEP__11_2022.step | 37.9 | ok | 6.3 | 1535 | 36 | 12159 | 30400 | 1 | 6 | 0 | 1 | 3 | 20 | 0 | 0 | 0 | 415 | 70 | 26 |
| cm-1_solid_models_10_2022 | CM-1_STEP__10_2022.step | 20.7 | ok | 1.3 | 72 | 24 | 6122 | 15183 | 1 | 1 | 0 | 1 | 2 | 18 | 0 | 0 | 0 | 383 | 11 | 16 |
| cobot-30_solid_models_03_2025 | COBOT 30-3_2025.STEP | 16.3 | ok | 12.0 | 65 | 28 | 2202 | 5772 | 0 | 0 | 0 | 0 | 0 | 56 | 0 | 0 | 0 | 398 | 86 | 0 |
| cobot_20kg-f_solid_models_09_2026 | cobot 20kg-F_09-26.STEP | 12.1 | ok | 3.8 | 85 | 20 | 2628 | 7290 | 2 | 4 | 2 | 1 | 0 | 10 | 0 | 0 | 0 | 285 | 35 | 18 |
| cobot_20kg-f_st-20_solid_models_09_2026 | cobot_pkg-7_st-20_09-2026.STEP | 39.6 | ok | 6.0 | 194 | 71 | 9713 | 24964 | 2 | 4 | 2 | 4 | 1 | 34 | 0 | 0 | 0 | 408 | 62 | 19 |
| cobot_20kg-f_umc-500_solid_models_09_2026 | cobot_pkg20-f_umc-500_09-2026.STEP | 32.5 | ok | 4.6 | 158 | 36 | 7509 | 20139 | 2 | 5 | 2 | 2 | 0 | 13 | 0 | 0 | 0 | 419 | 45 | 19 |
| compact-apl_solid_models_2_2023 | COMPACT-APL_STEP__2_2023.step | 8.3 | ok | 1.3 | 61 | 22 | 3419 | 8650 | 0 | 0 | 0 | 0 | 0 | 6 | 0 | 0 | 0 | 389 | 22 | 1 |
| dc-2-40_solid_models_12_2024 | DC-2-40T_12_2024.STEP | 49.6 | ok | 3.9 | 202 | 41 | 18848 | 50727 | 2 | 1 | 1 | 0 | 0 | 12 | 0 | 0 | 0 | 420 | 16 | 1 |
| desktop_lathe_solid_models_new_08_2024 | desktop_lathe_solid_models_08_2024.STEP | 12.3 | ok | 9.6 | 60 | 20 | 3242 | 8297 | 0 | 1 | 0 | 1 | 2 | 28 | 0 | 0 | 0 | 327 | 23 | 8 |
| desktop_mill_solid_models_new_08_2024 | desktop_mill_solid_models_08_2024.STEP | 5.9 | ok | 4.2 | 118 | 15 | 1972 | 5278 | 1 | 1 | 1 | 0 | 0 | 6 | 0 | 0 | 0 | 253 | 34 | 8 |
| desktop_mill_solid_models_old_04_2024 | desktop_mill_solid_models_04_2024.STEP | 7.3 | ok | 4.5 | 122 | 15 | 2449 | 6548 | 1 | 1 | 1 | 0 | 0 | 8 | 0 | 0 | 0 | 252 | 114 | 8 |
| dm-1_solid_models_01_2026 | DM-1_1_2026.STEP | 42.4 | ok | 5.3 | 132 | 46 | 13760 | 36412 | 2 | 2 | 2 | 1 | 0 | 6 | 0 | 0 | 0 | 423 | 33 | 0 |
| dm-2_solid_models_04_2025 | DM-2_2_2025.STEP | 39.9 | ok | 6.0 | 125 | 47 | 13678 | 36248 | 2 | 2 | 2 | 0 | 0 | 8 | 0 | 0 | 0 | 422 | 31 | 0 |
| ds-28y_solid_models_10_2026 | ds-28y_solid_models_10-2026.STEP | 43.3 | ok | 15.5 | 183 | 60 | 14782 | 34805 | 0 | 0 | 14 | 1 | 2 | 30 | 0 | 0 | 0 | 410 | 107 | 2 |
| ds-30y_solid_models_12_2022 | DS-30Y_STEP__12_2022.STEP | 91.1 | ok | 11.8 | 279 | 103 | 15667 | 39509 | 1 | 1 | 0 | 3 | 3 | 36 | 0 | 0 | 0 | 406 | 67 | 35 |
| ds-35y_solid_models_10_2026 | ds-35y_solid_models_10-2026.STEP | 51.0 | ok | 12.5 | 197 | 68 | 15300 | 36369 | 3 | 14 | 15 | 0 | 1 | 20 | 0 | 0 | 0 | 415 | 68 | 0 |
| ds-40y_solid_models_05_2026 | ds-40y_solid_model_5_2026.STEP | 26.5 | ok | 13.7 | 374 | 67 | 7590 | 18987 | 2 | 3 | 1 | 0 | 0 | 26 | 0 | 0 | 0 | 413 | 102 | 0 |
| dt-2_solid_models_04_2025 | DT-2_4_2025.STEP | 43.8 | ok | 7.1 | 407 | 48 | 14587 | 38929 | 1 | 1 | 1 | 0 | 1 | 18 | 0 | 0 | 0 | 416 | 83 | 0 |
| ec-1600zt_pallet_pool_solid_models_10_2025 | EC-1600ZT- Pallet Pool -10_2025.STEP | 37.5 | ok | 5.4 | 363 | 140 | 12332 | 31796 | 0 | 0 | 5 | 1 | 5 | 30 | 0 | 0 | 0 | 409 | 60 | 0 |
| ec-1600zt_solid_models_10_2025 | EC-1600ZT-10_2025.STEP | 27.5 | ok | 4.3 | 312 | 95 | 9322 | 24122 | 0 | 1 | 0 | 0 | 2 | 26 | 0 | 0 | 0 | 413 | 52 | 0 |
| gm-1-5ax_solid_models_07_2026 | GM-1-5AX 07-01-2026.STEP | 58.0 | ok | 11.2 | 190 | 152 | 21326 | 48860 | 1 | 2 | 2 | 1 | 12 | 29 | 0 | 3 | 2 | 400 | 283 | 2 |
| gm-1xt-5ax_solid_models_07_2026 | GM-1XT-5AX-07-2026.STEP | 60.9 | ok | 11.2 | 183 | 152 | 22710 | 51688 | 1 | 2 | 2 | 1 | 8 | 29 | 0 | 3 | 2 | 401 | 405 | 2 |
| gm-1xt_solid_models_04_2025 | GM-1XT_4_2025.STEP | 41.5 | ok | 8.4 | 203 | 24 | 11952 | 30956 | 1 | 3 | 1 | 2 | 2 | 44 | 0 | 0 | 0 | 352 | 82 | 0 |
| hab3232_solid_models_06_2026 | HAB3232_6_2026.STEP | 31.0 | ok | 32.2 | 117 | 19 | 4680 | 12203 | 2 | 4 | 1 | 0 | 0 | 24 | 0 | 0 | 0 | 261 | 71 | 0 |
| hbf-12_solid_models_07_2025 | Haas Bar Feeder 12_7_2025.STEP | 2.0 | ok | 0.1 | 57 | 13 | 810 | 2079 | 0 | 0 | 0 | 0 | 9 | 8 | 0 | 0 | 0 | 230 | 5 | 0 |
| hc10_solid_models_05_2025 | HC10_5_2025.STEP | 26.4 | ok | 12.8 | 76 | 58 | 4028 | 10506 | 0 | 0 | 1 | 0 | 2 | 43 | 0 | 0 | 0 | 404 | 164 | 0 |
| hcs-80_solid_models_04_2026 | HCS-80_04_2026.STEP | 35.6 | ok | 10.9 | 375 | 121 | 10100 | 25251 | 2 | 5 | 3 | 3 | 1 | 28 | 0 | 0 | 0 | 412 | 287 | 1 |
| hdc-3-5ax_solid_models_05_2024 | HDC-3-5AX-MLM_STEP__5_8_2024.step | 68.6 | ok | 16.4 | 219 | 38 | 15420 | 38226 | 6 | 8 | 4 | 7 | 0 | 6 | 0 | 0 | 0 | 423 | 80 | 11 |
| hfl-1313-3_solid_models_07_2026 | HFL-1313-3 07-22-2026.STEP | 29.3 | ok | 4.9 | 409 | 43 | 9633 | 27416 | 0 | 2 | 0 | 0 | 0 | 16 | 0 | 0 | 0 | 418 | 83 | 0 |
| hfl-3015_solid_models_02_2026 | HFL-3015_02_2026.STEP | 80.2 | ok | 4.3 | 367 | 96 | 27926 | 83313 | 0 | 0 | 0 | 17 | 1 | 8 | 0 | 0 | 0 | 422 | 118 | 6 |
| hfl-3015d-6_solid_models_06_2026 | HFL-3015D-6 06-30-2026.STEP | 50.2 | ok | 6.0 | 380 | 60 | 16868 | 49104 | 2 | 5 | 2 | 8 | 1 | 29 | 0 | 0 | 0 | 411 | 123 | 0 |
| hfl-6512t_solid_models_08_2026 | HFL-6512T Tube Laser 08-21-2026.STEP | 56.2 | ok | 12.3 | 691 | 84 | 17899 | 47692 | 0 | 2 | 1 | 4 | 2 | 11 | 0 | 0 | 0 | 419 | 211 | 1 |
| hkm-1_solid_models_02_2026 | HKM-1_02_2026.STEP | 6.0 | ok | 1.1 | 40 | 55 | 1916 | 4970 | 1 | 2 | 1 | 0 | 0 | 17 | 0 | 0 | 0 | 417 | 11 | 0 |
| hml-1v_1v-ce_solid-Models_07_2026 | hml-1v_ce_07-2026.STEP | 32.7 | ok | 7.7 | 96 | 33 | 3285 | 8719 | 1 | 1 | 0 | 1 | 0 | 16 | 0 | 0 | 0 | 418 | 23 | 1 |
| hml-2v_2v-ce_solid_models_07_2026 | hml-2v_ce_ 07-2026.STEP | 41.8 | ok | 10.0 | 119 | 32 | 3737 | 10049 | 1 | 2 | 0 | 1 | 0 | 16 | 0 | 0 | 0 | 418 | 31 | 0 |
| hml-3v_3v-ce_solid_models_07_2026 | hml-3v_ce_ 07-2026.STEP | 39.8 | ok | 10.0 | 115 | 32 | 3734 | 10061 | 1 | 2 | 0 | 1 | 0 | 14 | 0 | 0 | 0 | 419 | 34 | 0 |
| hpb-170-10_solid_models_04_2026 | HPB-170-10_04_2026.STEP | 63.2 | ok | 10.3 | 218 | 129 | 21432 | 60689 | 1 | 0 | 1 | 0 | 0 | 22 | 0 | 0 | 0 | 415 | 93 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Die_Kit-11-2026.STEP | 0.3 | ok | 0.0 | 9 | 3 | 93 | 261 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 60 | 0 | 7 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Punch_Kit.STEP | 0.5 | ok | 0.0 | 11 | 6 | 172 | 480 | 0 | 0 | 0 | 0 | 0 | 6 | 0 | 0 | 0 | 114 | 0 | 10 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Tooling_Die-11-2026.STEP | 0.1 | ok | 0.0 | 6 | 1 | 31 | 87 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Tooling_Punch_11-2026.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Die_100mm.STEP | 0.1 | ok | 0.0 | 6 | 1 | 31 | 87 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Die_10mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 31 | 87 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Die_15mm.STEP | 0.1 | ok | 0.0 | 6 | 1 | 31 | 87 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Die_200mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 31 | 87 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Die_20mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 31 | 87 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Die_300mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 31 | 87 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Die_40mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 31 | 87 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Die_50mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 31 | 87 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB170_Die_835mm.STEP | 0.1 | ok | 0.0 | 9 | 1 | 31 | 87 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB_Punch_100mm_Left.STEP | 0.1 | ok | 0.0 | 7 | 1 | 30 | 84 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB_Punch_100mm_Right.STEP | 0.1 | ok | 0.0 | 7 | 1 | 30 | 84 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB_Punch_10mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB_Punch_15mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB_Punch_200mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB_Punch_20mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB_Punch_300mm.STEP | 0.1 | ok | 0.0 | 8 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB_Punch_40mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB_Punch_50mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-170-10_tooling_models_11_2026 | HPB_Punch_835mm.STEP | 0.1 | ok | 0.0 | 10 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-50-5_solid_models_04_2026 | HPB-50-5 04-09-2026.STEP | 61.5 | ok | 9.9 | 224 | 102 | 20521 | 58642 | 1 | 0 | 2 | 0 | 0 | 18 | 0 | 0 | 0 | 417 | 66 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Die_Kit.STEP | 0.4 | ok | 0.0 | 6 | 9 | 162 | 432 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 171 | 0 | 9 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Punch_Kit.STEP | 0.7 | ok | 0.0 | 11 | 9 | 256 | 714 | 0 | 0 | 0 | 0 | 0 | 9 | 0 | 0 | 0 | 162 | 0 | 9 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Tooling_Die.STEP | 0.1 | ok | 0.0 | 6 | 1 | 26 | 72 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Tooling_Punch.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Die_100mm.STEP | 0.1 | ok | 0.0 | 6 | 1 | 26 | 72 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Die_10mm.STEP | 0.1 | ok | 0.0 | 6 | 1 | 26 | 72 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Die_15mm.STEP | 0.1 | ok | 0.0 | 6 | 1 | 26 | 72 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Die_200mm.STEP | 0.1 | ok | 0.0 | 6 | 1 | 26 | 72 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Die_20mm.STEP | 0.1 | ok | 0.0 | 6 | 1 | 26 | 72 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Die_300mm.STEP | 0.1 | ok | 0.0 | 6 | 1 | 26 | 72 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Die_40mm.STEP | 0.1 | ok | 0.0 | 6 | 1 | 26 | 72 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Die_50mm.STEP | 0.1 | ok | 0.0 | 6 | 1 | 26 | 72 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB50_Die_835mm.STEP | 0.1 | ok | 0.0 | 6 | 1 | 26 | 72 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB_Punch_100mm_Left.STEP | 0.1 | ok | 0.0 | 7 | 1 | 30 | 84 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB_Punch_100mm_Right.STEP | 0.1 | ok | 0.0 | 7 | 1 | 30 | 84 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB_Punch_10mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB_Punch_15mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB_Punch_200mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB_Punch_20mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB_Punch_300mm.STEP | 0.1 | ok | 0.0 | 8 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB_Punch_40mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB_Punch_50mm.STEP | 0.1 | ok | 0.0 | 7 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hpb-50-5_tooling_models_11_2026 | HPB_Punch_835mm.STEP | 0.1 | ok | 0.0 | 10 | 1 | 28 | 78 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 16 | 0 | 0 |
| hqda5_solid_models_07_2025 | HQDA5_7_2025.STEP | 5.5 | ok | 0.5 | 56 | 10 | 1859 | 4402 | 0 | 0 | 0 | 0 | 1 | 18 | 0 | 0 | 0 | 178 | 19 | 0 |
| hrp-2_solid_models_1_2022 | HRP-2_STEP__1_2022.step | 73.7 | ok | 17.2 | 247 | 216 | 13519 | 35115 | 2 | 4 | 7 | 8 | 0 | 24 | 0 | 0 | 0 | 414 | 100 | 360 |
| hrp1_10-2020 | HRP1_STEP_10-2020.STEP | 111.2 | ok | 11.1 | 255 | 166 | 9263 | 22769 | 3 | 1 | 4 | 4 | 1 | 52 | 0 | 0 | 0 | 400 | 57 | 13 |
| hrt160ss_step_10-2021 | HRT160SS_STEP_10-2021.STEP | 22.6 | ok | 2.1 | 70 | 183 | 7278 | 17035 | 0 | 0 | 0 | 0 | 3 | 26 | 0 | 0 | 0 | 412 | 22 | 0 |
| hrt200hs_solid_models_08_2025 | HRT200HS_8_2025.STEP | 2.4 | ok | 0.4 | 22 | 2 | 879 | 2074 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 1 | 0 |
| hrt210_10-2020 | HRT210_STEP_10-29-2020.STEP | 9.9 | ok | 0.9 | 63 | 31 | 2895 | 7213 | 0 | 0 | 0 | 0 | 4 | 26 | 0 | 2 | 0 | 409 | 157 | 127 |
| hrt210s_solid_models_05_2026 | HRT210S 05-11-2026.STEP | 2.2 | ok | 0.1 | 24 | 13 | 903 | 2258 | 0 | 0 | 0 | 0 | 0 | 9 | 0 | 0 | 0 | 234 | 2 | 2 |
| hvs-1_solid_models_07_2025 | HVS-1.STEP | 1.9 | ok | 0.4 | 15 | 31 | 744 | 1805 | 0 | 0 | 0 | 0 | 0 | 8 | 0 | 0 | 0 | 422 | 6 | 0 |
| mini_mill_solid_models_2_2022 | MINI MILL_STEP__2_2022.step | 62.0 | ok | 8.3 | 200 | 45 | 11189 | 29018 | 1 | 2 | 1 | 2 | 1 | 28 | 0 | 0 | 0 | 412 | 104 | 33 |
| mini_mill_solid_models_2_2022 | MINI MILL_Z-AXIS CLEARANCE_STEP__2_2022.step | 62.9 | ok | 7.0 | 212 | 45 | 12161 | 31706 | 1 | 2 | 2 | 5 | 1 | 18 | 0 | 0 | 0 | 417 | 91 | 35 |
| mm_solid_models_01_2024 | mm_solid_models_01_2024.STEP | 116.8 | ok | 14.5 | 496 | 159 | 16338 | 43163 | 2 | 10 | 0 | 4 | 5 | 42 | 0 | 0 | 0 | 402 | 105 | 71 |
| pocket_lathe_solid_models_05_2026 | Pocket Lathe 05-27-2026.STEP | 36.3 | ok | 7.1 | 117 | 170 | 9973 | 24568 | 0 | 2 | 1 | 6 | 0 | 42 | 0 | 0 | 0 | 405 | 41 | 4 |
| pocket_mill_solid_models_06_2025 | Pocket Mill_6_2025.STEP | 21.5 | ok | 3.0 | 146 | 41 | 7464 | 19649 | 1 | 0 | 2 | 0 | 0 | 26 | 0 | 0 | 0 | 413 | 41 | 0 |
| robot_pkg-1_7_front_solid_models_06_2025 | ROBOT PKG-1_7KG_FRONT_06_2025.STEP | 28.0 | ok | 6.2 | 91 | 58 | 6613 | 16582 | 1 | 4 | 2 | 1 | 0 | 8 | 0 | 0 | 0 | 422 | 53 | 21 |
| robot_pkg-1_7_side_solid_models_06_2025 | ROBOT_PKG-1_7KG_SIDE_06_2025.STEP | 28.3 | ok | 5.6 | 90 | 58 | 6697 | 16833 | 1 | 4 | 2 | 1 | 0 | 10 | 0 | 0 | 0 | 421 | 57 | 0 |
| rpl-25kg_solid_models_12_2024 | RPL-25KG_12_2024.STEP | 26.6 | ok | 8.6 | 216 | 31 | 6429 | 16918 | 0 | 2 | 1 | 1 | 0 | 36 | 0 | 0 | 0 | 408 | 64 | 0 |
| rpl-30kg_solid_models_10_2025 | Fanuc 30kg w RPL_10_2025.STEP | 23.7 | ok | 20.4 | 131 | 27 | 5932 | 15291 | 1 | 1 | 2 | 1 | 0 | 22 | 0 | 0 | 0 | 415 | 49 | 0 |
| rpl_modular_solid_models_09_2026 | rpl_modular_stand-alone_09-2026.STEP | 22.9 | ok | 4.5 | 94 | 144 | 7272 | 17270 | 1 | 1 | 0 | 1 | 0 | 13 | 0 | 3 | 0 | 415 | 264 | 0 |
| rpl_modular_umc_500_solid_models_09_2026 | rpl_modular_umc-500_09-2026.STEP | 66.5 | ok | 13.4 | 197 | 208 | 18824 | 47318 | 2 | 4 | 3 | 2 | 0 | 29 | 0 | 0 | 0 | 411 | 78 | 0 |
| rpl_modular_umc_750_solid_models_09_2026 | rpl_modular_umc-750_09-2026 1.STEP | 90.9 | ok | 26.6 | 264 | 211 | 23496 | 57851 | 6 | 10 | 7 | 2 | 1 | 30 | 0 | 0 | 0 | 411 | 95 | 0 |
| sb-35l_solid_models_09_2025 | SB-35L_9_2025.STEP | 22.6 | ok | 3.8 | 72 | 39 | 6260 | 16313 | 2 | 3 | 4 | 0 | 0 | 32 | 0 | 0 | 0 | 410 | 33 | 0 |
| sb-35y_solid_models_03_2026 | SB-35Y.STEP | 17.2 | ok | 1.1 | 73 | 45 | 6590 | 16775 | 0 | 0 | 2 | 0 | 3 | 16 | 0 | 0 | 0 | 416 | 19 | 7 |
| sr-3015_solid_models_11_2024 | SR 3015_11_2024.STEP | 4.6 | ok | 0.5 | 102 | 12 | 1853 | 4988 | 0 | 0 | 0 | 0 | 0 | 26 | 0 | 0 | 0 | 209 | 25 | 1 |
| sr-4020_solid_models_04_2025 | SR-4020_4_2025.STEP | 4.9 | ok | 0.4 | 206 | 13 | 1999 | 5404 | 0 | 0 | 0 | 0 | 0 | 8 | 0 | 0 | 0 | 235 | 26 | 0 |
| st-10_st-15_solid_models_10_2023 | st-10_st-15_solid_models_10_2023.STEP | 201.1 | ok | 26.4 | 545 | 186 | 48211 | 126721 | 0 | 2 | 1 | 53 | 0 | 48 | 0 | 2 | 0 | 399 | 359 | 195 |
| st-10y_st-15y_solid_models_10_2023 | st-10y_st-15y_solid_models_10_2023.STEP | 123.7 | ok | 17.9 | 322 | 50 | 16206 | 42699 | 3 | 6 | 2 | 1 | 0 | 29 | 0 | 0 | 0 | 411 | 110 | 21 |
| st-28_st-28y_solid_models_07_2023 | ST-28:28Y_10000224585.STEP | 123.7 | ok | 12.0 | 320 | 91 | 25072 | 66318 | 2 | 4 | 4 | 6 | 4 | 30 | 0 | 0 | 0 | 409 | 72 | 44 |
| st-28l_solid_models_02_2025 | ST-28L- 2_2025.STEP | 64.0 | ok | 10.0 | 220 | 98 | 19144 | 49087 | 1 | 2 | 4 | 0 | 2 | 40 | 0 | 0 | 0 | 404 | 106 | 0 |
| st-28ly_solid_models_02_2025 | ST-28LY- 2_2025.STEP | 39.7 | ok | 6.8 | 178 | 70 | 11190 | 29326 | 1 | 2 | 2 | 1 | 0 | 38 | 0 | 0 | 0 | 407 | 73 | 0 |
| st-30(y)-35(y)_2009-2020+ | ST-30-35Y_STEP_2009-2019.STEP | 65.0 | ok | 3.1 | 241 | 43 | 7719 | 19307 | 0 | 0 | 0 | 239 | 1 | 14 | 0 | 0 | 0 | 418 | 28 | 21 |
| st-30(y)-35(y)_2009-2020+ | ST-30Y-35Y_STEP_2020-03.STEP | 104.5 | ok | 7.1 | 274 | 174 | 16169 | 39969 | 0 | 1 | 0 | 38 | 1 | 24 | 0 | 0 | 0 | 413 | 152 | 13 |
| st-30-35_(2-axis)_2009-2020+ | ST-30-35Y_STEP_2009-2019.STEP | 65.0 | ok | 3.2 | 241 | 43 | 7719 | 19307 | 0 | 0 | 0 | 239 | 1 | 14 | 0 | 0 | 0 | 418 | 28 | 21 |
| st-30-35_(2-axis)_2009-2020+ | ST-30-35_STEP_2020-03.STEP | 101.9 | error | 0.1 | 119 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| st-40_solid_models_05_2024 | st-40_solid_models_05_2024.STEP | 48.8 | ok | 7.2 | 166 | 84 | 14561 | 36424 | 1 | 1 | 4 | 1 | 0 | 28 | 0 | 0 | 0 | 412 | 49 | 3 |
| st-45_solid_models_05_2024 | st-45_solid_models_05_2024.STEP | 48.8 | ok | 7.5 | 162 | 83 | 15132 | 37981 | 1 | 1 | 5 | 2 | 0 | 32 | 0 | 0 | 0 | 410 | 49 | 3 |
| t5c2_solid_models_12_2021 | T5C2_STEP_12_2021.STEP | 10.6 | ok | 1.0 | 69 | 5 | 2553 | 6132 | 0 | 0 | 0 | 0 | 0 | 6 | 0 | 0 | 0 | 97 | 17 | 3 |
| t5c3_solid_models_12_2021 | T5C3_STEP_12_2021.STEP | 12.5 | ok | 1.3 | 86 | 7 | 2998 | 7159 | 0 | 0 | 0 | 0 | 1 | 6 | 0 | 0 | 0 | 134 | 18 | 4 |
| t5c4_solid_models_12_2021 | T5C4_STEP_12_2021.STEP | 11.4 | ok | 0.9 | 73 | 11 | 2773 | 6565 | 0 | 0 | 0 | 0 | 0 | 6 | 0 | 0 | 0 | 202 | 17 | 5 |
| t5c_solid_models_11_2021 | T5C_STEP_11_2021.STEP | 10.5 | ok | 0.7 | 74 | 3 | 2624 | 6472 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 56 | 8 | 0 |
| tl-1_solid_models_05_2023 | TL-1_STEP__5_2023.step | 27.8 | ok | 3.9 | 104 | 22 | 9060 | 22915 | 1 | 1 | 0 | 2 | 1 | 35 | 0 | 0 | 0 | 357 | 43 | 10 |
| tl-2_solid_models_11_2021 | TL-2_STEP_11_2021.STEP | 44.4 | ok | 5.6 | 157 | 55 | 10395 | 27692 | 0 | 1 | 0 | 1 | 1 | 38 | 0 | 0 | 0 | 407 | 91 | 15 |
| tl-2p_solid_models_12_2024 | TL-2P_12_2024.STEP | 34.6 | ok | 6.7 | 132 | 42 | 10118 | 26975 | 0 | 1 | 0 | 1 | 2 | 28 | 0 | 2 | 0 | 409 | 205 | 0 |
| tl-3b_solid_models_07_2026 | tl-3b_mld_07-2026.STEP | 54.6 | ok | 9.6 | 239 | 66 | 14286 | 37476 | 2 | 3 | 1 | 1 | 2 | 40 | 0 | 0 | 0 | 405 | 52 | 1 |
| tm_1_1p_10_2021 | TM-1_1P_STEP_10_2021.STEP | 49.3 | ok | 4.3 | 230 | 38 | 10832 | 29167 | 1 | 1 | 1 | 2 | 1 | 37 | 0 | 0 | 0 | 407 | 54 | 14 |
| tm_2_2p_10_2021- | TM-2_2P-STEP_2021_10.STEP | 57.1 | ok | 5.6 | 215 | 541 | 19384 | 51275 | 1 | 1 | 18 | 1 | 1 | 45 | 0 | 0 | 0 | 403 | 59 | 15 |
| tr160-2_2020-03 | TR160-2_STEP_2020-03.STEP | 21.6 | ok | 1.7 | 140 | 27 | 5384 | 13782 | 1 | 1 | 1 | 0 | 2 | 20 | 0 | 0 | 0 | 416 | 18 | 7 |
| trt210_4-2021 | TRT210_STEP_4-2021.STEP | 18.2 | ok | 2.4 | 62 | 48 | 4968 | 12009 | 1 | 2 | 0 | 0 | 3 | 22 | 0 | 0 | 0 | 413 | 14 | 0 |
| umc-1000_ss_solid_models_04_2025 | UMC-1000_4_2025.STEP | 52.5 | ok | 13.1 | 190 | 62 | 13183 | 34382 | 2 | 6 | 6 | 2 | 2 | 18 | 0 | 0 | 0 | 417 | 53 | 2 |
| umc-1500-duo_2020-07 | UMC-1500-DUO_STEP_2020-07.STEP | 70.6 | ok | 5.4 | 181 | 238 | 10646 | 26658 | 1 | 2 | 5 | 2 | 1 | 29 | 0 | 0 | 0 | 410 | 25 | 13 |
| umc-400_ss_solid_models_02_2025 | UMC 400_2_2025.STEP | 60.3 | ok | 10.9 | 183 | 49 | 15766 | 41265 | 3 | 3 | 2 | 0 | 3 | 44 | 0 | 6 | 0 | 393 | 802 | 0 |
| umc-500_ss_10_station_pp_solid_models_03_2026 | UMC-500 w 10 Station PP_3_2026.STEP | 51.2 | ok | 8.1 | 352 | 96 | 13602 | 35297 | 1 | 2 | 2 | 2 | 3 | 27 | 0 | 2 | 0 | 407 | 481 | 0 |
| umc-500_ss_20_station_pp_solid_models_03_2026 | UMC-500 w 20 Station PP_3_2026.STEP | 55.7 | ok | 9.6 | 180 | 105 | 16471 | 42641 | 0 | 2 | 2 | 1 | 1 | 19 | 0 | 0 | 0 | 415 | 47 | 0 |
| umc-500_ss_smtc120_solid_models_04_2026 | UMC-500 + 120 PKT TC_4_2026.STEP | 51.8 | ok | 13.8 | 159 | 65 | 12922 | 34021 | 2 | 4 | 2 | 1 | 7 | 35 | 0 | 0 | 0 | 405 | 116 | 1 |
| umc-500_ss_solid_models_04_2026 | UMC-500_4_2026.STEP | 49.3 | ok | 8.3 | 175 | 72 | 13832 | 36276 | 1 | 3 | 1 | 1 | 3 | 32 | 0 | 0 | 0 | 408 | 50 | 1 |
| umc-750_ss_smtc120_solid_models_04_2026 | UMC-750_120 PKT_4_2026.STEP | 49.6 | ok | 14.8 | 150 | 68 | 12650 | 32195 | 2 | 5 | 4 | 1 | 0 | 26 | 0 | 0 | 0 | 413 | 59 | 0 |
| umc-750_ss_solid_models_11_2025 | UMC-750_30_50_70PKT_11_2025.STEP | 51.9 | ok | 16.4 | 158 | 69 | 12898 | 32656 | 2 | 6 | 6 | 1 | 0 | 28 | 0 | 0 | 0 | 412 | 55 | 0 |
| vc-400-pallet-pool-option | VC-400 with Pallet Pool_STEP.STEP | 120.2 | ok | 7.9 | 305 | 646 | 25918 | 63288 | 2 | 2 | 3 | 42 | 3 | 36 | 0 | 0 | 0 | 407 | 82 | 248 |
| vf-10_40_solid_models_7_2022 | VF-10_40_STEP_8in-EXT_7_2022.step | 83.1 | ok | 13.2 | 248 | 62 | 19725 | 52574 | 2 | 3 | 6 | 4 | 2 | 32 | 0 | 0 | 0 | 410 | 100 | 64 |
| vf-10_40_solid_models_7_2022 | VF-10_40_STEP__7_2022.step | 80.7 | error | 0.0 | 88 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| vf-11_40_solid_models_7_2022 | VF-11_40_STEP__7_2022.step | 66.4 | ok | 11.1 | 211 | 43 | 14197 | 38398 | 5 | 4 | 7 | 2 | 0 | 32 | 0 | 0 | 0 | 410 | 67 | 41 |
| vf-11_40_solid_models_7_2022 | VF-11_40_Z-EXT_STEP__7_2022.step | 63.0 | ok | 10.5 | 265 | 37 | 13114 | 35097 | 5 | 4 | 6 | 6 | 0 | 32 | 0 | 0 | 0 | 410 | 75 | 59 |
| vf-11_50_solid_models_7_2022 | VF-11_50_STEP__7_2022.step | 57.8 | ok | 4.6 | 177 | 33 | 13554 | 37211 | 2 | 3 | 6 | 2 | 0 | 13 | 0 | 0 | 0 | 419 | 21 | 36 |
| vf-11_50_solid_models_7_2022 | VF-11_50_Z-EXT_STEP__7_2022.step | 59.3 | ok | 4.8 | 188 | 33 | 14218 | 38721 | 2 | 3 | 6 | 2 | 0 | 19 | 0 | 0 | 0 | 416 | 30 | 48 |
| vf-1_11_2021 | VF-1_STEP_11_2021.STEP | 52.3 | ok | 6.1 | 182 | 128 | 17456 | 45103 | 5 | 9 | 6 | 0 | 1 | 32 | 0 | 0 | 0 | 410 | 45 | 50 |
| vf-2_solid_models_1_2023 | VF-2TR_STEP__1_2023.STEP | 91.6 | error | 0.0 | 102 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| vf-2_solid_models_1_2023 | VF-2_STEP__1_2023.step | 59.4 | ok | 6.5 | 476 | 29 | 11553 | 29874 | 0 | 2 | 2 | 8 | 3 | 37 | 0 | 0 | 0 | 405 | 87 | 26 |
| vf-2yt_2ssyt_solid_models_11_2021 | VF-2YT_2SSYT_STEP__11_2021.step | 43.9 | ok | 6.3 | 158 | 60 | 11737 | 31570 | 1 | 1 | 4 | 1 | 3 | 52 | 0 | 0 | 0 | 400 | 52 | 16 |
| vf-3_ss_solid_models_2_2023 | VF-3_SS_STEP__2_2023.STEP | 100.5 | ok | 12.1 | 285 | 63 | 20441 | 54455 | 4 | 6 | 8 | 6 | 0 | 42 | 0 | 0 | 0 | 405 | 87 | 39 |
| vf-4_4ss_solid_models_1_2022 | VF-4_4SS_STEP__1_2022.step | 75.3 | ok | 12.7 | 222 | 101 | 20526 | 53301 | 1 | 5 | 11 | 3 | 0 | 30 | 0 | 0 | 0 | 411 | 57 | 20 |
| vf-5_40_ss_tr_solid_models_3_2022 | VF-5_40_SS_TR_STEP__3_2022.step | 71.1 | ok | 13.4 | 207 | 101 | 22397 | 58597 | 3 | 6 | 9 | 2 | 5 | 36 | 0 | 0 | 0 | 408 | 86 | 36 |
| vf-6_50_tr_solid_models_2_2022 | VF-6_50_TR_EXTENDED Z-AXIS CLEARANCE_STEP__2_2022.step | 107.5 | ok | 16.0 | 695 | 48 | 23622 | 62756 | 1 | 4 | 6 | 2 | 3 | 63 | 0 | 0 | 0 | 393 | 188 | 51 |
| vf-6_50_tr_solid_models_2_2022 | VF-6_50_TR_STEP__2_2022.step | 104.0 | error | 0.0 | 112 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| vp-5_solid_models_06_2024 | VP-5 _6_2024.STEP | 33.3 | ok | 6.9 | 122 | 35 | 9497 | 25712 | 2 | 2 | 2 | 2 | 1 | 22 | 0 | 0 | 0 | 415 | 58 | 0 |
| vr-11_solid_models_07_2024 | VR-11 _7_2024.STEP | 29.8 | ok | 6.8 | 114 | 38 | 8815 | 22860 | 3 | 5 | 4 | 1 | 0 | 18 | 0 | 2 | 0 | 414 | 182 | 0 |
| vr-14_solid_models_07_2024 | VR-14_7_2024.STEP | 29.9 | ok | 6.7 | 116 | 36 | 8808 | 22833 | 3 | 4 | 4 | 1 | 1 | 20 | 0 | 2 | 0 | 413 | 187 | 1 |
| vr-5_solid_models_08_2024 | VR-5_solid_model__08_2024.STEP | 28.7 | ok | 5.0 | 127 | 24 | 6286 | 16966 | 1 | 1 | 2 | 2 | 0 | 12 | 0 | 0 | 0 | 369 | 53 | 0 |
| vr-9_solid_models_07_2024 | VR-9_7_2024.STEP | 30.7 | ok | 10.9 | 115 | 44 | 7152 | 17983 | 2 | 6 | 2 | 1 | 0 | 28 | 0 | 0 | 0 | 412 | 62 | 0 |
| vs-3_solid_models_06_2024 | VS-3_STEP__6_2024.STEP | 74.4 | ok | 12.4 | 227 | 30 | 20586 | 52818 | 3 | 4 | 3 | 1 | 0 | 25 | 0 | 2 | 0 | 410 | 243 | 78 |

