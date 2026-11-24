# The shipped boolean's proof: the ledger

The single source of truth for what remains unproven in `boolean_correct`
(research/decide-once/old-path-statements.md §E), today's boolean as a whole: kernel/forge-bool's
boolean.rs `general`, classify, imprint, unify, ssi and nary.

**How to use it.** Each agent edits only its own rows. Change a row's status in the same PR as the
Lean that changes it, and name the theorem and its file. A row is **proven** only when its theorem
builds with the standard axioms (`propext`, `Classical.choice`, `Quot.sound`) and no `sorry`. An
unproven contract is a `def … : Prop` in Lean, never a `sorry`.

**Status:**
- **open**: not yet stated in Lean;
- **stated**: a `def … : Prop` that builds;
- **proven**: a theorem, standard axioms only;
- **hyp**: a named hypothesis of the end-to-end theorem, discharged outside forge-bool (by the
  feature that makes the body, or by an instance of a trait).

**Owners:** SPINE (§E, the composition, the body layer, forge-topo validity); TRANSLATE (the
Aeneas translation of forge-bool and its types); FLAT-SITES (classify.rs, point_in_face, G1/§1a,
G11 flat, G13–G15); FALLBACK (G2–G10, G12, A8 as the fallback, and the fallback being unneeded); CURVED-SITES
(the curved guess sites: G11 on curved faces, coincidence, unify's curved decisions, poles and
seams, and the curved screens S3/S4).

**Where the gap is.** `boolean_correct_of_steps` proves §E from the contracts D1–D9 alone (the
boundary lemma `KeepRuleBd` is proven). So the end-to-end theorem is down to the rows below that are still open or stated.
Each one is a per-function contract, the shape lemma, or a named hypothesis. A bare file name means
verify/forge-bool-proof/ForgeBoolProof/<file>.

## E. The end-to-end theorem and its hypotheses

| id | contract | owner | status | file / theorem |
|---|---|---|---|---|
| E | `boolean_correct`: Ok ⇒ ValidSolid ∧ Faithful (F1) ∧ NamesOK; Err ⇒ RefusalTrue | SPINE | stated | Statement.lean `BooleanCorrect` |
| E.comp | the composition: D1–D9 ⇒ E, for `Pipeline.run` | SPINE | proven | KeepRule.lean `boolean_correct_of_steps`; Compose.lean `boolean_correct_of_contracts` (with `res_keepRoom`, `apply_correct`) |
| E.shape | `ShapeR general read P.run`: the translated (in-place, Aeneas `Result`) `general` returns `ok`, and its output read by `readInPlace` is `Pipeline.run` of its translated steps, with a `View` per stage; the retry ladder's rungs are `general` with the flags as an argument (`boolean`'s clone/`into_compact` wrapper is D0) | TRANSLATE | stated | Bridge.lean `ShapeR`, `readInPlace` |
| E.R | §E for the translation itself (`BooleanCorrectR`: returns `ok`, no panic or divergence, and `Correct`) from E.shape and D1–D9 | SPINE | proven | Bridge.lean `booleanCorrectR_of_steps` |
| E.seq | sequential regen as a fold of booleans | SPINE | proven (given E and `boolean_preserves_InputSize`) | Compose.lean `seq_correct` |
| E.fb | the flat fallback branch: today's path refuses or is undecided ⇒ A8, then a typed refusal; A8 starts from the original inputs | FALLBACK | composition proven (`TodayOK` + A8's `BooleanCorrect` ⇒ E); A8's `BooleanCorrect` open (rows A8.*) | Compose.lean `withFallback`, `fallback_correct` |
| E.ref | `RefusalTrue A B op e` per refusal kind (`ZeroThickness`, `Unhandled k`, `Undecided q`); `Internal` mapped or unreachable | SPINE (stmt), FALLBACK (`Internal`) | stated (`internal` is never true; `unhandled`/`undecided` read through the declared `Scope`) | Statement.lean `RefusalTrue`, `ManifoldPt` |
| E.names | `NamesOK`, per entity (D7) | SPINE (stmt), FALLBACK (naming fns) | open (a parameter `Names` of E; the per-entity statement is open) | — |
| E.size | `InputSize A B`, and `boolean_preserves_InputSize` | SPINE | open (a parameter `Size` of E until the translated surface data exists; preservation is `seq_correct`'s `hsize`) | — |
| E.size.rs | each operand within `InputSize` at the boolean's entries: checked in debug builds only (size.rs `debug_check`, `boolean`/`boolean_into`/`boolean_unlinked`/`nary_with`); release is a GAP: creation-path checks or a release refusal, the owner to choose | FLAT-SITES | open | kernel/forge-bool size.rs |
| H.fin | `FaithfulInput δ A`: each stored region and curve within δ of its exact meet | hyp (producing feature) | hyp | Statement.lean `Faithful`, in `Input` |
| H.abs | `FormulaOfTopology abs`: the Φ_u that `abs` reads equals the stored local cone | TRANSLATE | open | — |
| H.batch | `Batch` sound when decided: a code other than 3 is the exact op-fold verdict | FALLBACK | proven for CpuBatch's combinatorics (fold, own tool forced, early 3, grid skipping without an intersect; Csg's three-valued combination); hyp: `MemberSound` at the leaves (Prism/Revolved float screens: IEEE, open), `BoxSound`, the GPU impl | verify/forge-bool-fallback `Batch.cpuCode_sound_all`, `Batch.cpuCode_sound_grid`, `Member.csg_sound` |

## 0. The body layer

| id | contract | owner | status | file / theorem |
|---|---|---|---|---|
| B.model | `BodyModel`, `Bounds`, `bounds_unique`, `ValidExact`, `denoteExact` | SPINE | proven | verify/forge-curved/ForgeCurved/BodyModel.lean |
| B.F1 | G1, ruled F1: `Refines δ`, `exactOf`, `exactOf_refines`, `ValidSolid δ`, `denote_spec`, `denote_refines` | SPINE | proven | BodyModel.lean |
| B.rooms | the bridge: `room_in_or_out`, `denote_eq_closure_rooms`, `f6_hyps`, `oriented_denote`; F5 `res_eq_reg` | SPINE | proven | BodyModel.lean, Rooms.lean |
| B.clusters | vertex clusters (option iv): `vertCore`, `ClusterOK`, `EdgeIsolated` | SPINE | proven on dol-curved (bc5f25d3) | landing: spine PR 2 |
| B.phi | Φ_u: `formulas`, `Bounds.local_formula`, `mem_denote_zone` | SPINE | proven on dol-curved (53957ebb) | landing: spine PR 2 |
| B.types | the translated `Body`, `Face`, `Loop`, `Coedge`, `Edge`, `Vertex`, `Surface`, `Curve` | TRANSLATE | partial: `Surface`, `Curve`, `SurfaceSurface`, `Contact` and their analytic structs translated (forge-geom), with `Proc`/`ProcCurve`/`IntCurve` opaque (L4.proc, L4.intcurve); `Body`…`Vertex` next (forge-topo) | verify/aeneas/forge-geom `ForgeGeom/Types.lean` |
| B.abs | `abs : Body → BodyModel`, the abstraction function | TRANSLATE (def), SPINE (use) | open | — |
| T.model | forge-topo `check` returning no problem ⇒ `Valid` (L1/L3) | SPINE | proven | verify/forge-topo-check/lean `check_sound` |
| T.strays | `topo::strays` refines `checkStrays` (L4) | SPINE | proven (overflow-checked translation) | `forge_topo.strays_refines` |
| T.links | `topo::links` refines `checkLinks` (L4) | SPINE | proven (overflow-checked translation) | `forge_topo.links_refines` |
| T.shells | `topo_shell::shell_faces` refines `checkShellFaces` (L4) | SPINE | proven (overflow-checked translation) | `forge_topo.shell_faces_refines` |
| T.edges | `topo_edges::edges`, `loops` refine `checkEdges`, `checkLoops` (L4) | SPINE | proven (overflow-checked translation) | `forge_topo.edges_refines`, `loops_refines` |
| T.ovf | the five forge-topo functions above, translated overflow-checked: no `usize`/`u32` overflow (Checked.lean) | SPINE | proven | verify/aeneas/forge-topo/lean (`usize_add_wspec`, `usize_sub_wspec`, the `_refines` theorems) |
| T.euler | `topo_euler::shell_euler` refines `checkShellEuler` (L4; stamped distinct count = eraseDups) | SPINE | proven (overflow-checked; hyp `EulerSmall`: a shell's listed faces, loops, coedges fit i64) | `forge_topo.shell_euler_refines` |
| T.fans | `topo_fans::fans` refines `checkFans` (L4; corner table as an opaque view method) | SPINE | proven (overflow-checked; hyps `CornersOK` (the opaque corner table's contract), `nf_lt`, `FansSmall`) | `forge_topo.fans_refines` |
| T.all | `check_known`'s combinatorial part composed (`Model.check`) | SPINE | open | — |
| T.valid | `Valid` (topology) ∧ geometry ⇒ `WellFormed` of the exact counterpart | SPINE | open | — |

## D. The chain of contracts (boolean.rs `general`)

| id | function(s) | contract | owner | status | file / theorem |
|---|---|---|---|---|---|
| D0 | `boolean` → clone, `boolean_into`, `into_compact` | compaction keeps a valid faithful result valid and faithful, and keeps its solid, names and size (`D0`); the clone is gone in the functional reading (`readInPlace`) | SPINE (stmt, composition), TRANSLATE (`into_compact` against `D0`) | stated; composition proven | Compose.lean `D0`, `compact_correct` |
| D1 | `Index::new`, `face_box` | ⟦f⟧ ⊆ box f; boxes that miss ⇒ faces miss | FLAT-SITES | stated | Contracts.lean `D1`, `PairsOK` |
| D2 | `meet_region`, `ssi::surfaces_in`, `plane_meet`, `crossings_near` | every point of ⟦fa⟧ ∩ ⟦fb⟧ is on a returned curve (within δ, F1); closed-form miss = `Unhandled::NoClosedForm` | TRANSLATE | stated; open; translated: `ssi::surfaces_in`, `closed_or_flush`, `closed_form`, `nearly_flush`, `turned_flush`, `spread_apart` with forge-geom's `plane_surface`/`plane_plane` (`meet_region`, `crossings_near` wait for FALLBACK's ladder argument; `plane_meet` is curved). Proven (L4, on the translation): two planes never reach the curved search: `surfaces_in` = `closed_or_flush`, whose value is never `NotAnalytic` (from `plane_plane` or the flush rule's `Coincident`). F1 for `plane_plane` is FALSE inside InputSize: its parallel branch (|n1×n2| ≤ ANGULAR) decides Empty/Coincident at `q.frame.origin`, sound only where the crossing is within LINEAR/ANGULAR = 1e6 of it; the exact reference refutes it at L = 1.6e7 (a box and a prism whose walls cross at x = 4e6 at 5e-12 rad: `plane_plane` = Empty both ways; reference: solid for all three ops; kernel: false ZeroThickness refusals, and intersect Ok with an invalid body, EdgeUses). These were B1, a confirmed wrong Ok (exact reference), and B2, a separate confirmed wrong Ok (exact reference): a thin wedge (walls 1.2·LINEAR apart at the ends, angle > ANGULAR, `plane_plane` right) gave intersect Ok with an invalid body at L = 1e5…2e6. Fixed, by the exact reference on the tilt family (all seven tests of verify/forge-exact/fuzz/tests/tilt_repros.rs agree; exact-fuzz unchanged): B1 by ssi.rs `parallel_in` (two planes within ANGULAR of parallel, not to the bit, decided over the region: Coincident / Empty / the crossing line, symmetric) with forge-geom `near_parallel_in` (a line within ANGULAR of parallel to a plane decided over its range), B2 by boolean.rs `off_near_ends` (imprint). Proven on the translation: `parallel_in` never returns NotAnalytic (`parallel_in_only`), so two planes still never reach the curved search. F1 of the new rules (the IEEE error of the corner gaps and of the line's foot) is OPEN. research/kernel-audit/tilt-wrong-ok.md | Contracts.lean `D2`, `CurvesOK`; verify/aeneas/forge-bool `ForgeBool/Funs.lean`, `ForgeBool/D2Flat.lean` (`surfaces_in_planes`, `closed_or_flush_planes_only`, `nearly_flush_planes_only`, `parallel_in_only`, `plane_plane_only`, `spindle_in_planes`) |
| D3 | `touches`, `flows_on`, `crack_tip`, `local_class` | every contact listed with exact local classes; the non-manifold refusal is `RefusalTrue` | FALLBACK (with G4) | stated | Contracts.lean `D3`, `TouchesOK` |
| D4 | `trim`, `rank_pairs` | segments = the D2 curves' pieces inside both faces, ends at exact meets; rank by an exact key | FLAT-SITES (with G14, G15) | stated | Contracts.lean `D4`, `SegsOK` |
| D5 | `imprint`, `share_cuts`, `place_cuts`, `edge_split`, `split_face`, `plane_split` | the regions are the arrangement's facets on f (`FacetData`) | SPINE (statement), FLAT-SITES (proof, after §1a) | stated | Contracts.lean `D5`, `ArrOK`; `place_cuts`: research/kernel-audit/coverage/aeneas (proven, L4) |
| D6 | `classify_specs`, `interior_point`, `probe`, `Index::classify` (§1a), coplanar sense | each region's class is `whereR` of the other solid at the region | FLAT-SITES | stated | Contracts.lean `D6`, `ClassesOK` |
| D7 | `keep_of`, `lean`, the naming functions | kept = facets separating a kept room from an unkept one; names by the rule | SPINE (keep), FALLBACK (names, with A8's edge-name parity) | stated | Contracts.lean `D7`, `KeepOK` |
| D8 | `apply`, `edge_split`, joins/splits, `sole_survivors` | R's exact counterpart is well formed with isolated edges, R is faithful (F1) and named by the rule, and its faces meet `BoundsFaces` against the closure of the kept rooms (frontier on the faces where the arrangement's are, `faces_sub`, Φ_u, orientation, seeded) | SPINE | stated | Contracts.lean `D8`, `ApplyPost` |
| D9 | `pinched_new`, `pinched_old`, `touched_old`, `undecided_touch`, `refuse_strays` | refuses iff the exact result is non-manifold there | FALLBACK (G6, G7) | stated | Contracts.lean `D9` |
| D5.zone | `ArrOK.on_frontier` inside vertex-cluster zones: a facet on a body's faces in a zone (micro-faces) is on that solid's frontier, or the contract changes there | SPINE | open (with the cluster ruling) | Contracts.lean `ArrOK.on_frontier` |
| D7.math | `KeepRuleBd`: on exact classes, `keepRule` picks each boundary facet of the kept rooms once (a coplanar piece once, as A's) | SPINE | proven | KeepRule.lean `keepRuleBd_proof` (`facet_rooms`, `table` by `decide`) |
| D7.table | `keep_of` computes `keepRule` (the table in Contracts.lean is the spec; if boolean.rs's table differs, it changes here with a reason) | SPINE | open | — |
| D8.derived | the rest of `Bounds` for the result: the closure of the kept rooms is closed, regular, bounded, with its frontier on the arrangement's faces | SPINE | proven | Compose.lean `bounds_of_faces`, `frontier_res_sub` |
| D8.math | the assembly boundary lemma: from `KeptIsBd` and the faces `apply` writes (`WritesKept`: the result's face points are the closure of the kept facets), `BoundsFaces` | SPINE | proven for `frontier_on` and `faces_sub`, given D5.thin; the other three fields are D8.formula, D8.orient, D8.seeded | Assembly.lean `boundsFaces_boundary`, `boundsFaces_of_writes` |
| D8.writes | `apply`'s output read as faces is `WritesKept` (its face points = closure of the kept regions' facets) | SPINE | open | Assembly.lean `WritesKept` |
| D8.formula | `BoundsFaces.local_formula` for the result's zones (Φ_u) | SPINE | open | — |
| D8.orient | `BoundsFaces.oriented`: each lone point of a result face has the kept rooms on its inside side | SPINE | open | — |
| D8.seeded | `Seeded` of the result's faces | SPINE | open | — |
| D5.thin | the arrangement's face points off the facets (edges, vertices) have empty interior and do not cut small neighbourhoods (`Thin`) | SPINE (stmt), FLAT-SITES/CURVED-SITES (proof with D5) | open | Assembly.lean `Thin` |
| Dn | `nary_with` | D1–D9 over n tools, the op fold as keep; `Batch` sound when decided | FALLBACK (G10) | open | — |

## §1a. The exact `On` (classify.rs)

| id | contract | owner | status | file / theorem |
|---|---|---|---|---|
| 1a.where | `whereR`, `whereF` | FLAT-SITES | proven | FlatSites/Where.lean `whereR`, `whereF`, `whereR_on_iff`, `whereR_outside_iff` |
| 1a.classify | `classify_spec`: Ok w ⇒ w = whereR ⟦b⟧ p | FLAT-SITES | open: the `On` branch proven from two facts (`classify_on_sound`, `exists_face_of_on`): p exactly on the face's carrier (GAP: not checked by the code; held at 3,632 of 3,632 planar `On`s on the gate parts and fuzz 1–40, fails on the bench parts frame and volute, points 2.6e-16 and 8.6e-16 off the plane: an exact check there refuses ZeroThickness, so the exact answer must come from the exact key) and `point_in_face ≠ Outside` meaning p ∈ closure ⟦f⟧ (1a.pif), off the zones (D5.zone). The ray branch is open (GAP: rays rejected 0, all-degenerate 0 in the corpus) Fact 2 at point_in_face's nearest-edge `On`, trim_in's stretch sampler: tolerance-On, correct under δ-Faithful (FlatSites/TolOn.lean `edge_within_of_ends`, `near_segment`; measured on the 36 patterns-seq sites: results valid, edges within 2.7e-13 of their exact meet lines, vertices within 2.6e-13), the exact reading differs by ≤ 1e-13 (the 36 samples are exactly off the edge: the three planes share no line). | FlatSites/Where.lean |
| 1a.rays | classify's and point_in_face's degenerate rays: `plane_ray_crossings`' zero denominator (a ray parallel to a line edge), non-finite hit, hit within the margin of an edge's end, `conic_ray`'s tangency, `classify`'s Touching hit, and the uv-polygon fallback after all three planar rays are undecided | FLAT-SITES | open: **no corpus evidence** (coverage-2026-11-23 group 4: none of these branches ran in the gate parts, bench parts or fuzz): the proof must cover them. Unit tests now drive them (classify.rs `degenerate_rays`: each curve case of `plane_ray_crossings`, and 60 star faces with a vertex on every planar ray, checked against the exact even-odd answer); the non-finite hit is still unreached | classify.rs tests |
| 1a.total | `classify_total_flat`: never refuses on flat bodies | FLAT-SITES | open | — |
| 1a.pif | `point_in_face_spec`: Ok w ⇒ w = whereF ⟦f⟧ p | FLAT-SITES | open | — |
| 1a.foot | `foot`, `foot_on`; per probe site, foot ∈ relInterior region | FLAT-SITES | open: `foot_on`, `foot_self` proven (where the f64 point is on the carrier, as at the 3,632 classify `On`s of the gate parts and fuzz 1–40, the foot is the point; not at frame's and volute's); per-site membership open (G13) | FlatSites/Where.lean |
| 1a.B3387 | trim_in's stretch midpoint M: screen sound bound, cold exact helper | FLAT-SITES | open | — |

## G. Guess sites (each becomes a proven screen plus an exact test, or goes)

| id | site | owner | status | file / theorem |
|---|---|---|---|---|
| G1 | the output geometry is f64 | SPINE | stated (ruled F1: the reading `exactOf`/`Refines`/`denote` is proven in BodyModel; that the output is within δ, `Faithful δ (abs R)`, is D8's obligation, open) | BodyModel.lean; Contracts.lean `ApplyPost` |
| G2 | the retry ladder (`plan_retried`, `plan_fenced`): flags now one explicit `Rung` argument (PR proof/fallback) | FALLBACK | composition proven: an `Ok` is one try's `Ok`, a non-internal refusal is one try's refusal, the ladder is correct where every rung is; the per-rung contract (each rung's near-tangent reading as a screen + exact test) open; fired 0× on suite + fuzz 1–300: UNKNOWN, not safe | verify/forge-bool-fallback `Ladder.plan_ok`, `Ladder.plan_err`, `Ladder.plan_correct` |
| G3 | `RETRIED` + post-hoc `forge_topo::check` → `Lost::RetryInvalid` (`Tried.retried`, explicit since G2) | FALLBACK | open; it only turns an `Ok` into an `Internal` refusal (never makes an `Ok`); 0× on the corpus: UNKNOWN | — |
| G4 | `UNDECIDED` touch + post-hoc `FaceWrongSide` check (`Tried.undecided`, explicit since G2) | FALLBACK | open; **reachable**: fuzz55 f21 (a subtracted ball whose pole touches the top face from inside, seq and n-ary), check passed → `Ok`, exact verdict open | — |
| G5 | `ODD` between runs | FALLBACK | closed: an output of each try (`Tried.odd`, cleared per fenced try, PR proof/fallback); modelled in `Ladder.fenced` | verify/forge-bool-fallback `Ladder` |
| G6 | `pinched_new` / `pinched_old` (and n-ary's pinch) | FALLBACK | open; 0× on the corpus: UNKNOWN | — |
| G7 | `refuse_strays` | FALLBACK | open; 0× on the corpus: UNKNOWN | — |
| G8 | `FORGE_IMPRINT` env | FALLBACK | closed: compile-time feature `imprint_column`, off in every shipped build; release reads no env (PR proof/fallback) | kernel/forge-bool/src/imprint.rs `mode` |
| G9 | `SCRATCH`, cached `FaceInfo` (`CacheOK b`) | FALLBACK | open | — |
| G10 | `nary`'s `Batch` and threads; verdict 3 (2,447× on suite + fuzz 1–300 n-ary) goes to the full way (D6, §1a) | FALLBACK | Batch: see H.batch; the full way is D6's. Threads: the answers of `par::map_adaptive` come in item order whatever the threads (proven, `par_map_eq`, given each item pure); each item pure: hyp, by review (tools/proof/hidden-inputs.txt, every static, thread-local and env read classed; `hidden_inputs` fails on a new one; ssi's per-thread `MET` holds no `ABSORB_HAIR_ROOTS` answer: nothing on `search`'s path reaches `conic_quadric`). Measured with helpers forced on everywhere (map_adaptive's thresholds and `THREADED_AFTER_PAIRS` zeroed): n-ary bits at 1 vs 2/4/4 threads, suite + fuzz 1–300 (316 parts), 0 differ; the guard `nary_threads` forces helpers (`par::force_helpers`, a test-only feature) and checks they started | research/booleans/lean/Speedups.lean `par_map_eq`; kernel/forge-model/tests/nary_threads.rs |
| G11 | flat `LINEAR`/`RIM` tolerance tests | FLAT-SITES | open | — |
| G11c | curved `LINEAR`/`RIM` tolerance tests: near-misses (1e-9 < \|d\| ≤ limit) over 22 gate runs + fuzz 1–40 seq: 104 on main cb8edf27 (gate 46, fuzz 58), 99 with fix-curved absorbed (the per-site counts below) (curved-fallback-rates.md, "Re-measured"); the 4 not listed below are fuzz27's one 1.9e-7 fact at trim shallow ×2, dedupe, B:2798 | CURVED-SITES | open; per site below; none judged by the exact reference (every stage out of its scope): UNKNOWN | — |
| C.2830 | `on_edge` at trim's curve ∩ across hit (B:2830): the hit's exact meet is on the edge's branch | CURVED-SITES | branch proven given a separation and η (`branch_of_sep_delta`); 53 near-misses, all "on", nearest other branch ≥ 1.76 mm or none in 10 mm (measured): UNKNOWN | verify/forge-curved OnEdge.lean `branch_of_sep`, `branch_of_sep_delta`, `branch_of_empty` |
| C.sep | `SepTheorem` per pair kind; 37 of C.2830's 53 are torus–plane square to / through the axis | CURVED-SITES | stated; open pair kinds (16 near-misses): torus–torus 6, plane–torus oblique/spiric 6, torus–sphere 2, torus–cylinder 2 (hard: quartic separation) | OnEdge.lean `SepTheorem` |
| C.eta | the IEEE bound η of a torus hit (`curve_surface` on torus curves): measured 1e-9 to 2e-7 | CURVED-SITES | open | — |
| C.range | `ClosestGlobal`, `OnEdgeRange`: on_edge's off answers and its range test | CURVED-SITES | stated | OnEdge.lean |
| C.2910 | trim's traced-branch end on a rim (B:2910): 14 near-misses (fuzz 3, 9, 11, 15; 7 distinct ends, each met twice), 4e-8 to 9e-7, circle edges against a plane (10) or a cylinder (4) | CURVED-SITES | rim point: oracle-checked 14/14 (forge-bool rim_cert.rs: the other surface's implicit along the circle, a degree-2 trigonometric polynomial with enclosed coefficients, has exactly one root within the edge's reach of each end by `krawczyk1`; test `corpus_rim_points_are_unique_within_reach`); uniqueness: proven from the certificate (g' ≠ 0 on the interval: `rootUnique_of_param`, PlaceMerge.lean), the certificate oracle-checked 14/14; η_trace open (hyp: the tracer step lemma, phase B) | kernel/forge-bool/src/rim_cert.rs |
| C.slid | trim's `slid`, a grazing root a hair from an edge (B:2819): 7 near-misses (fuzz 3, 9, 11, 27), 1.3e-7 to 8.4e-7, tangent roots | CURVED-SITES | open, hard (exact tangency): UNKNOWN | — |
| C.place | `place_cuts` "two cuts one vertex": 13 merges with 1e-9 < d ≤ reach (mixed 1, fuzz2 5, fuzz11 5, fuzz25 1, fuzz27 1), all two cuts on one edge, mid-edge | CURVED-SITES | K1 (8 merges, 5 distinct: one root of one surface along the edge, computed from the edge's two faces): one exact point by `RootUnique`, proven from g' ≠ 0 on the reach's parameter interval (`zero_unique_of_deriv_ne`, Rolle; `rootUnique_of_param`); the certificate is oracle-checked for all 5 (rim_cert.rs): the 2 on analytic edges (fuzz11 circle × torus, fuzz25 circle × cylinder) by 1D Krawczyk along the edge (`k1_roots_are_unique_within_reach`), the 3 on traced Intersection edges (fuzz2 × sphere, fuzz2 × torus, mixed × torus) as the one point of A ∩ B ∩ S (the edge's two surfaces and S) in a box about the two cuts' middle by 3D Krawczyk (`point_unique`: K ⊂ int X and ‖I − YJ‖∞ < 1, the hypotheses of Krawczyk.unique_zero, verify/krawczyk Step.lean on branch claude/repo-review-mbdrlj-trace, not yet on main): no chain enclosure (η_trace) needed. K2 (4, fuzz11: two surfaces' exact points 1.57e-7 apart) and K3 (1, fuzz27: a touch and a crossing 1.92e-7 apart): the merged vertex is within ρ + reach + η of both exact points (`merge_within`), so §E's F1 holds when that is ≤ δ; the results check clean (forge_topo::check): not a bug | verify/forge-curved PlaceMerge.lean |
| C.pif | `point_in_face` nearest-edge `On` at curved callers (touch_inside's samples, the closed-curve midpoint): 9 near-misses (shaft 1, fuzz20 3, fuzz30 1, fuzz40 4), 0.2–0.9 LINEAR, every sample on the face's outside by its signed f64 distance | CURVED-SITES | proven decision-irrelevant for 7: fuzz40 ×4 (`whole_of_outside`: the other face `Outside` decides) and fuzz20 ×3 (`touch_inside_on_eq_outside`: `On` and `Outside` give `some false`), given the logged neighbouring samples and hyp "the exact side is not Inside" (signed −1.9e-6 … −9.0e-6; its rounding not bounded). shaft ×1, fuzz30 ×1: the decision differs (`touch_inside_on_ne_outside`) but the body does not: today's rule and the exact-side rule give equal V/E/F, validity and volume (bitwise), one vertex within 1 ulp and one closed one-vertex edge's range shifted by its period (measured on merge-train/1; the exact reference is out of scope for both parts): UNKNOWN as a theorem | verify/forge-curved PifNear.lean |
| C.node | a tangent node (forge-geom node.rs `mend`, from forge-bool `ssi::search`): two rounds, one per operand, tubes rolling off one common plane, whose springs cross there; the trace ended a hair (√(2 r e)) off it or ran one branch twice, so fg_mcp's plate round was refused at ~5% of radii in 10.9–24.3 (SplitCrossed, InconsistentClassification, ZeroThickness), now 0 of 1341 (0.01 steps) and acceptance monotone (Mitre from 24.4) | CURVED-SITES | proven: one branch out of the node into where both faces lie, increasing, meeting either spring only at the node (`one_branch`, `branch_increasing`, `branch_meets_springs_only_at_node`, `tubes_one_branch`), each cap the graph `h r ρ = r − √(r² − ρ²)` (`cap_section`), strictly increasing on [0, r] (`h_strictMonoOn`), the node on both (`h_zero`); the code's side checked exactly (tubes recognised, one plane S a radius over both spines, parallel, on one side, the node the springs' closed-form crossing at sine ≥ 0.02, the mend within a quarter of min(r, spine radius) and clear of other crossings by four reaches). hyp: the face is its tube's whole cap within that reach; `towards_touch`'s points are on both surfaces to its tolerance only. The exact curved reference cannot judge the part (a pipe face): volume checked smooth and strictly falling in r instead (interface/forge-lang/tests/round_node.rs) | verify/forge-curved BlendNode.lean |
| C.coinc | `ssi::coincident` / `unify::same_surface` `true`: one point set | CURVED-SITES | proven where the exact key holds: 6,864 of 6,900 trues (plane 6,115, cylinder 725, sphere 24); the 36 unkeyed are patterns seq's one-ulp pattern copies (exactly distinct planes): UNKNOWN | verify/forge-curved CarrierKey.lean `plane_key`, `plane_key_axis`, `cyl_key`, `sphere_key`; kernel/forge-bool/src/carrier_key.rs |
| C.unify | unify's curved decisions | CURVED-SITES | open | — |
| C.poles | poles and seams (`sphere_nearest`'s pole rule, periodic seams) | CURVED-SITES | open | — |
| G.param | an edge's parameter range bounded by the model (abs(t) ≤ a bound from InputSize): would let forge-geom's `line_in_range` skip `near_parallel_in` by a constant test (its range check costs patterns seq ~+0.03%); tol::MODEL_HALF_SIZE (5e5) does not match InputSize (1.6e7) today | FLAT-SITES | open | — |
| G12 | the `decide_once` feature becomes the fallback branch | FALLBACK | open | — |
| G13 | `face_point`'s unchecked inward step | FLAT-SITES | open (GAP, not shipped by decision): the probe is not inside its face by construction (a face narrower than the step puts it off; 1 planar and 6 curved of 6,675 probes on the gate parts and fuzz 1–40). A runtime check sound by argument (first edge a line or a circle wider than the step, every other boundary use's box apart from the step or its carrier provably missed; else `point_in_face`) cost +0.035% on plate seq, over the band; curved faces' `point_in_face` check +1.7% total. Kept for reference on proof/flat-sites-g13 (#43, closed). The class half is proven (`where_const`, FlatSites/G13.lean) | — |
| G14 | trim_in's box-only accept | FLAT-SITES | proven (hyp: D4's cut completeness, `Sorted`) | verify/forge-curved/ForgeCurved/G14.lean `kept_iff`, `extra_not_end`, `edge_cut_not_kept` |
| G15 | the stretch whose midpoint is exactly on ∂f_k | FLAT-SITES | proven for flat (straight stretch, edge line in the plane; hyp: D4 cut completeness): the stretch lies on the edge's carrier, today's along-the-edge reading; curved (a touch, not a crossing) open | FlatSites/G15.lean `along_or_crossing`, `on_middle_along` |
| M.ct | forge-geom `meet::intersect`, cylinder × torus (`Form::Poly4`): its events from a sampled discriminant plus #33's root-count scan (thin_window) | FALLBACK (fix), CURVED-SITES | UNKNOWN (David, 2026-11-24: keep today's search; option 1 of research/decide-once/mct-options.md): no theorem that every event is found; oracle in CI (poly4-oracle.yml: every search on the gate parts, mixed and fuzz 1–300 certified by meet_cert, 2,491 of 2,491 today; one not certified fails CI) | kernel/forge-model/tests/poly4_oracle.rs |
| M.cert | M.ct's events certified: interval cover of (u, s) (natural and mean-value enclosures of F and ∂F/∂s), Krawczyk boxes of one event each (kernel/forge-geom/src/meet_cert.rs, tests and the decision log only: a test oracle, never run by the shipped build) | CURVED-SITES | soundness of the cover proven (`no_event_of_cover`; a mean-value exclusion is a kind 0/1 box like the natural one); `IntervalSound` (now also the mean-value form), `CountConstant` stated; mixed seq + fuzz 1–40: 386 of 386 Poly4 calls Certified (median 0.70 ms); the bare 64-sample discriminant on seed 295: Missed (host u 1.918, 2.097, 2.149); the shipped search with thin_window on fuzz295: 7 of 7 Certified. Runtime certification ruled out (David: prove the code, no proofs at runtime); M.ct's proof: static sampling rule, open | verify/forge-curved EventCover.lean |
| C.trace | `certified_trace` (forge-model feature, off by default; #21) traces a root of no closed form as accurately as the default tracer: on forge-model tests/fillet_conic.rs `a_root_of_no_closed_form_carries_the_section_and_a_varying_radius_is_refused` the fill's added volume is 22.932846907963 with the feature on vs 22.932846909302 off, against the quadrature reference 22.9328469093 (OCCT agrees to 5e-11): off by 1.34e-9 on (over the test's 1e-9), 2e-12 off. Measured on #21 merged with main f04c533e | CURVED-SITES | open: tracer accuracy or the volume's own error not judged: UNKNOWN | — |
| M.cone | `Form::Poly4` with a cone host (90 calls on fuzz 1–40): not covered by the certifier | CURVED-SITES | open | — |
| C.g5.apexplane | forge-geom `intersect::plane_cone` through or near the apex (point, one line along a generator, two lines, parabola; 17 regions unrun) | CURVED-SITES | open: no corpus evidence (coverage 2026-11-23 group 5): proof must cover it, or add a fuzz family; family `apex_planes` added: 60 seeds, 0 wrong; apex_planes/3 (a plane 3 LINEAR off the apex, parallel to the axis) did not mesh, a near-degenerate hyperbola sampled in equal steps of t: fixed (forge-mesh samples hyperbolas in equal steps of asinh t, `conic_chords_hold_the_tolerance` on those samples); 3 kernel `Internal(InconsistentClassification)` refusals on the sequential path (apex_planes 8, 14, 46: a plane 0.5 LINEAR off the apex, parallel to a generator), with FALLBACK (the seq Internal class): UNKNOWN | kernel/forge-model adversarial.rs `apex_planes` |
| C.g5.apexcones | cone–cone with a shared apex (forge-geom meet) | CURVED-SITES | open: no corpus evidence (coverage 2026-11-23 group 5): proof must cover it, or add a fuzz family; family `apex_cones` added: 60 seeds, 0 wrong, 8 `Unsupported(NoClosedForm)` refusals each path: a typed refusal where the reference gives a solid (a coverage gap, not a wrong answer) | adversarial.rs `apex_cones` |
| C.g5.parallel | `intersect::plane_cylinder` a hair off parallel (tilt 1e-12 to 1e-3 rad; the d < 0.999 r branch unrun) | CURVED-SITES | open: no corpus evidence (coverage 2026-11-23 group 5): proof must cover it, or add a fuzz family; family `near_parallel` added: 60 seeds, 0 wrong, 0 refused | adversarial.rs `near_parallel` |
| C.g5.spindle | spindle and horn tori (tube ≥ centre radius) | CURVED-SITES | open: no corpus evidence (coverage 2026-11-23 group 5): proof must cover it, or add a fuzz family; family `spindle_tori` added: all 60 refused at creation (CrossesAxis, "a whole circle touching the axis pinches the solid"), so not reachable by revolve; reachable only by import (UNKNOWN) | adversarial.rs `spindle_tori` |
| C.g5.spun | spun-spline meets (forge-geom spline/meet.rs: meet_spline, roots, spline_surface, through_axis) | CURVED-SITES | open: no corpus evidence (coverage 2026-11-23 group 5): proof must cover it, or add a fuzz family (existing family `spline_revolve` not in the coverage corpus; outside the exact reference's scope) | — |
| C.g5.ruled | ruled–ruled walls and `parabola_wall` (forge-geom ruled/meet.rs) | CURVED-SITES | open: no corpus evidence (coverage 2026-11-23 group 5): proof must cover it, or add a fuzz family | — |

## NC. No corpus evidence (FALLBACK's groups of research/coverage-2026-11-23, on main 103dfb8b)

Code the corpus (gate parts, bench parts, suite fuzz 1–300 seq and nary1, the exact-reference repros)
never runs. A count of 0 is no evidence either way: each row is UNKNOWN until a proof covers it or a
fuzz family reaches it.

| id | site | owner | status | file / theorem |
|---|---|---|---|---|
| NC.ref | the 17 refusal sites that run under nothing, unit tests included: boolean.rs `RetryInvalid`, the `undecided_touch` refusal, `OpenCurve`, `Uncut`, `NoNormal`, `DegenerateEdge`, `SplitOpen`, `EmptyCycle`, `HoleOutside`, `NoInteriorPoint`, `UntouchedOnOther`; nary.rs `NoClosedForm`, `ToolVerticesMeet`, `NoNormal`, `NoInteriorPoint`, `EmptyCycle`, `MergedRegion` (refusal-sites.tsv) | FALLBACK | open; no corpus evidence: proof must cover it, or add a fuzz family | — |
| NC.fences | the FENCES rung (G2): `ODD` is set 21× (bench frame, sweep-diff), never with an error, so the fenced retry never runs | FALLBACK | open; no corpus evidence: proof must cover it, or add a fuzz family | — |
| NC.careful | the CAREFUL rung (G2): never runs in the corpus | FALLBACK | open; no corpus evidence: proof must cover it, or add a fuzz family | — |
| NC.retried | a retried boolean returning `Ok` (G3's after-retry check): none in the corpus; only 6 unit-test cases reach the check | FALLBACK | open; no corpus evidence: proof must cover it, or add a fuzz family | — |
| NC.pinch | `pinched_new` finding a pinch (G6), `refuse_strays` refusing (G7): never in the corpus | FALLBACK | open; no corpus evidence: proof must cover it, or add a fuzz family | — |
| NC.nary.isect | n-ary with an `Intersect` tool: no corpus n-ary run has one (every `any_intersect` path skipped) | FALLBACK | open; no corpus evidence: proof must cover it, or add a fuzz family | — |
| NC.nary.par | n-ary's threaded `par_map` arms: only nary1 (one thread) runs | FALLBACK | covered by tests: `nary_threads` runs them with helpers forced on (774 helper starts over 5 parts at 2/4 threads, bits equal to 1 thread); measured, helpers forced on everywhere: suite + fuzz 1–300 (316 parts) at 2/4/4 threads, 0 differ (see G10). The corpus gate still runs nary1 only | kernel/forge-model/tests/nary_threads.rs |
| NC.nary.touch | touches inside an n-ary boolean: never reached | FALLBACK | open; no corpus evidence: proof must cover it, or add a fuzz family | — |

## A8, the flat fallback

| id | contract | owner | status | file / theorem |
|---|---|---|---|---|
| A8.core | `IncCoreSpec` | FALLBACK | proven | verify/aeneas/forge-bool-core `incCoreSpec_proof` |
| A8.e2e | `EndToEndF` (thin = []) | FALLBACK | proven | `endToEndF_proof` |
| A8.thin | the thin ≠ [] theorem | FALLBACK | open | — |
| A8.R3 | face_fast: `Refines` from the input bodies to the face ledger | FALLBACK | open | — |
| A8.out | the write-out of A8's result to a `Body` (D8 for A8) | FALLBACK | open | — |
| A8.ref | A8's refusals are `RefusalTrue` (`coreI_zeroThickness_iff`, the collapse) | FALLBACK | proven (core) | verify/forge-exact `coreI_zeroThickness_iff` |
| A8.unneeded | today's path never refuses or is undecided where A8 would answer | FALLBACK | stated (`A8Unneeded`), and its use proven: under it the composition `withFallback` answers exactly where today's path does (`withFallback_ok_iff`), so A8 stays unwired. The premise itself: open (UNKNOWN beyond the corpus); measured on main 4846170a, suite + fuzz 1–300 (316 parts), sequential and n-ary: one refusal, fuzz55 `ZeroThickness` (a subtracted ball, curved), where A8 (flat only) skips; A8 run from the original operands at every sequential refusal answered none | verify/forge-bool-fallback `Unneeded.A8Unneeded`, `Unneeded.withFallback_ok_iff` (a restatement of Compose.lean's `withFallback` over any types) |

## Curved screens (route B), for the curved D-steps

| id | contract | owner | status | file / theorem |
|---|---|---|---|---|
| S3 | a screened vertex is transversal | CURVED-SITES (landing: SPINE) | proven on dol-curved (8dbdc11d) | landing: spine PR 2, VertexScreen.lean |
| S4 | the angle comparator is the angular order | CURVED-SITES (landing: SPINE) | proven on dol-curved (83b5f267) | landing: spine PR 2, AngleOrder.lean |

## L4 gaps: what the translation keeps opaque

Each row is a Lean `opaque` in a translation (a type or a function with no definition a proof can
see): an assumption, not a proof. A theorem whose computation reaches one says nothing there.
TRANSLATE closes the easy ones as it goes. research/aeneas/forge-bool.md has the survey.

| id | opaque item(s) | why | owner | status | how it closes |
|---|---|---|---|---|---|
| L4.proc | `forge_geom::curve::ProcCurve`, `forge_geom::surface::Proc` (`Arc<dyn …>`) | Aeneas takes no method call through a trait object; ~32 implementations across crates | TRANSLATE | open | a closed enum of forge-geom's kinds plus one `External` arm, or out of the theorem's scope (procedural faces refused) |
| L4.intcurve | `forge_geom::intcurve::IntCurve` (traced intersection curves) | the tracer (forge-geom intcurve) is not translated | TRANSLATE (with CURVED-SITES) | open | translate the tracer's evaluation, or an exact curve per pair kind |
| L4.cached | `forge_topo::body::Cached` (`Face::query: Arc<dyn Any>`, cached boxes) | interior mutability and `dyn Any` downcast | TRANSLATE (with FALLBACK, G9) | open (with forge-topo) | `CacheOK b` as a hypothesis of each step, the cache as a typed table |
| L4.searched | `forge_bool::ssi::searched` (and its thread-local `MET`) | G9's cache in front of the curved search | FALLBACK (G9), then TRANSLATE | open | goes with G9; not reached by an analytic pair |
| L4.geomcurved | forge-geom's `screw::coaxial::meet_coaxial`, `ruled::meet::meet_ruled`, `spline::meet::meet_spline`, `sheet::used_by`, `intersect::plane_cylinder` | curved; `plane_cylinder` stops Aeneas ("Unreachable" at a join) | TRANSLATE (with CURVED-SITES) | open | translate in forge-geom under `DEPS`; not reached by a plane–plane pair |
| L4.big | `forge_bool::boolean::plan_once_in`, `split_face`, `nary::nary_body` | survey only: too large for Charon/Aeneas whole (12 GB cap) | TRANSLATE | open | split into named steps (E.shape's `Pipeline`), after FALLBACK's ladder argument |
| L4.zero | `F64.zeroTie` (the bridge): the zero `f64::min`/`max` return for `+0`/`-0` | Rust and LLVM leave it open (outside the float trusted base T) | TRANSLATE | hyp (by design) | none needed: both zeros are equal in value; a theorem must not depend on the sign |

