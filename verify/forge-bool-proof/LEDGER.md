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
| B.types | the translated `Body`, `Face`, `Loop`, `Coedge`, `Edge`, `Vertex`, `Surface`, `Curve` | TRANSLATE | open | verify/aeneas/forge-bool |
| B.abs | `abs : Body → BodyModel`, the abstraction function | TRANSLATE (def), SPINE (use) | open | — |
| T.model | forge-topo `check` returning no problem ⇒ `Valid` (L1/L3) | SPINE | proven | verify/forge-topo-check/lean `check_sound` |
| T.strays | `topo::strays` refines `checkStrays` (L4) | SPINE | proven (overflow-checked translation) | `forge_topo.strays_refines` |
| T.links | `topo::links` refines `checkLinks` (L4) | SPINE | proven (overflow-checked translation) | `forge_topo.links_refines` |
| T.shells | `topo_shell::shell_faces` refines `checkShellFaces` (L4) | SPINE | proven (overflow-checked translation) | `forge_topo.shell_faces_refines` |
| T.edges | `topo_edges::edges`, `loops` refine `checkEdges`, `checkLoops` (L4) | SPINE | proven (overflow-checked translation) | `forge_topo.edges_refines`, `loops_refines` |
| T.ovf | the five forge-topo functions above, translated overflow-checked: no `usize`/`u32` overflow (Checked.lean) | SPINE | proven | verify/aeneas/forge-topo/lean (`usize_add_wspec`, `usize_sub_wspec`, the `_refines` theorems) |
| T.euler | `check_shell_euler` (stamped distinct count = eraseDups) | SPINE | open | — |
| T.fans | `check_fans` (corner table as an opaque view method) | SPINE | open | — |
| T.all | `check_known`'s combinatorial part composed (`Model.check`) | SPINE | open | — |
| T.valid | `Valid` (topology) ∧ geometry ⇒ `WellFormed` of the exact counterpart | SPINE | open | — |

## D. The chain of contracts (boolean.rs `general`)

| id | function(s) | contract | owner | status | file / theorem |
|---|---|---|---|---|---|
| D0 | `boolean` → clone, `boolean_into`, `into_compact` | compaction keeps a valid faithful result valid and faithful, and keeps its solid, names and size (`D0`); the clone is gone in the functional reading (`readInPlace`) | SPINE (stmt, composition), TRANSLATE (`into_compact` against `D0`) | stated; composition proven | Compose.lean `D0`, `compact_correct` |
| D1 | `Index::new`, `face_box` | ⟦f⟧ ⊆ box f; boxes that miss ⇒ faces miss | FLAT-SITES | stated | Contracts.lean `D1`, `PairsOK` |
| D2 | `meet_region`, `ssi::surfaces_in`, `plane_meet`, `crossings_near` | every point of ⟦fa⟧ ∩ ⟦fb⟧ is on a returned curve (within δ, F1); closed-form miss = `Unhandled::NoClosedForm` | TRANSLATE | stated | Contracts.lean `D2`, `CurvesOK` |
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
| D8.math | the assembly boundary lemma: from `KeptIsBd` and the faces `apply` writes (the kept facets, oriented to the kept rooms), `BoundsFaces` holds; it is what D8's proof reduces to once `apply`'s output is read as faces | SPINE | open | — |
| Dn | `nary_with` | D1–D9 over n tools, the op fold as keep; `Batch` sound when decided | FALLBACK (G10) | open | — |

## §1a. The exact `On` (classify.rs)

| id | contract | owner | status | file / theorem |
|---|---|---|---|---|
| 1a.where | `whereR`, `whereF` | FLAT-SITES | open | — |
| 1a.classify | `classify_spec`: Ok w ⇒ w = whereR ⟦b⟧ p | FLAT-SITES | open | — |
| 1a.total | `classify_total_flat`: never refuses on flat bodies | FLAT-SITES | open | — |
| 1a.pif | `point_in_face_spec`: Ok w ⇒ w = whereF ⟦f⟧ p | FLAT-SITES | open | — |
| 1a.foot | `foot`, `foot_on`; per probe site, foot ∈ relInterior region | FLAT-SITES | open | — |
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
| G10 | `nary`'s `Batch` and threads; verdict 3 (2,447× on suite + fuzz 1–300 n-ary) goes to the full way (D6, §1a) | FALLBACK | Batch: see H.batch; the full way is D6's | — |
| G11 | flat `LINEAR`/`RIM` tolerance tests | FLAT-SITES | open | — |
| G11c | curved `LINEAR`/`RIM` tolerance tests: near-misses (1e-9 < \|d\| ≤ limit) over 22 gate runs + fuzz 1–40 seq: 104 on main cb8edf27 (gate 46, fuzz 58), 99 with fix-curved absorbed (the per-site counts below) (curved-fallback-rates.md, "Re-measured"); the 4 not listed below are fuzz27's one 1.9e-7 fact at trim shallow ×2, dedupe, B:2798 | CURVED-SITES | open; per site below; none judged by the exact reference (every stage out of its scope): UNKNOWN | — |
| C.2830 | `on_edge` at trim's curve ∩ across hit (B:2830): the hit's exact meet is on the edge's branch | CURVED-SITES | branch proven given a separation and η (`branch_of_sep_delta`); 53 near-misses, all "on", nearest other branch ≥ 1.76 mm or none in 10 mm (measured): UNKNOWN | verify/forge-curved OnEdge.lean `branch_of_sep`, `branch_of_sep_delta`, `branch_of_empty` |
| C.sep | `SepTheorem` per pair kind; 37 of C.2830's 53 are torus–plane square to / through the axis | CURVED-SITES | stated; open pair kinds (16 near-misses): torus–torus 6, plane–torus oblique/spiric 6, torus–sphere 2, torus–cylinder 2 (hard: quartic separation) | OnEdge.lean `SepTheorem` |
| C.eta | the IEEE bound η of a torus hit (`curve_surface` on torus curves): measured 1e-9 to 2e-7 | CURVED-SITES | open | — |
| C.range | `ClosestGlobal`, `OnEdgeRange`: on_edge's off answers and its range test | CURVED-SITES | stated | OnEdge.lean |
| C.2910 | trim's traced-branch end on a rim (B:2910): 14 near-misses (fuzz 3, 9, 11, 15), 4e-8 to 9e-7 | CURVED-SITES | open (C.2830's lemma once the tracer's end error is bounded): UNKNOWN | — |
| C.slid | trim's `slid`, a grazing root a hair from an edge (B:2819): 7 near-misses (fuzz 3, 9, 11, 27), 1.3e-7 to 8.4e-7, tangent roots | CURVED-SITES | open, hard (exact tangency): UNKNOWN | — |
| C.place | `place_cuts` "two cuts one vertex": 12 near-misses (mixed 1, fuzz 2, 11, 25, 27), 2.3e-9 to 1.9e-7 | CURVED-SITES | open (a provenance key): UNKNOWN | — |
| C.pif | `point_in_face` nearest-edge `On` at curved callers (touch_inside's samples B:2845, the closed-curve midpoint B:3315): 9 near-misses (shaft 1, fuzz 20, 30, 40), 0.2–0.9 LINEAR | CURVED-SITES (with FLAT-SITES' `point_in_face_at`) | open: UNKNOWN | — |
| C.coinc | `ssi::coincident` / `unify::same_surface` `true`: one point set | CURVED-SITES | proven where the exact key holds: 6,864 of 6,900 trues (plane 6,115, cylinder 725, sphere 24); the 36 unkeyed are patterns seq's one-ulp pattern copies (exactly distinct planes): UNKNOWN | verify/forge-curved CarrierKey.lean `plane_key`, `plane_key_axis`, `cyl_key`, `sphere_key`; kernel/forge-bool/src/carrier_key.rs |
| C.unify | unify's curved decisions | CURVED-SITES | open | — |
| C.poles | poles and seams (`sphere_nearest`'s pole rule, periodic seams) | CURVED-SITES | open | — |
| G12 | the `decide_once` feature becomes the fallback branch | FALLBACK | open | — |
| G13 | `face_point`'s unchecked inward step | FLAT-SITES | open | — |
| G14 | trim_in's box-only accept | FLAT-SITES | proven (hyp: D4's cut completeness, `Sorted`) | verify/forge-curved/ForgeCurved/G14.lean `kept_iff`, `extra_not_end`, `edge_cut_not_kept` |
| G15 | the stretch whose midpoint is exactly on ∂f_k | FLAT-SITES | open | — |
| M.ct | forge-geom `meet::intersect`, cylinder × torus (`Form::Poly4`): its events from a sampled discriminant, which lost one of two loops on fuzz295 f69 (a wrong `Ok`) | FALLBACK (fix), CURVED-SITES (#7, certified isolation) | interim fix (PR proof/fix-torus-loops): a root-count scan where the discriminant flags double roots; NOT provably complete (a loop narrower than one sample can be lost): UNKNOWN | kernel/forge-geom/src/meet_tests.rs `thin_torus_on_a_wide_cylinder_both_loops` |
| M.cert | M.ct's events certified: interval cover of (u, s), Krawczyk boxes of one event each (kernel/forge-geom/src/meet_cert.rs, tests and the decision log only) | CURVED-SITES | soundness of the cover proven (`no_event_of_cover`); `IntervalSound`, `CountConstant` stated; on seed 295 it proves the missed events (Missed at host u 1.918, 2.097, 2.149); runtime design open (cost) | verify/forge-curved EventCover.lean |
| C.trace | `certified_trace` (forge-model feature, off by default; #21) traces a root of no closed form as accurately as the default tracer: on forge-model tests/fillet_conic.rs `a_root_of_no_closed_form_carries_the_section_and_a_varying_radius_is_refused` the fill's added volume is 22.932846907963 with the feature on vs 22.932846909302 off, against the quadrature reference 22.9328469093 (OCCT agrees to 5e-11): off by 1.34e-9 on (over the test's 1e-9), 2e-12 off. Measured on #21 merged with main f04c533e | CURVED-SITES | open: tracer accuracy or the volume's own error not judged: UNKNOWN | — |
| M.cone | `Form::Poly4` with a cone host (90 calls on fuzz 1–40): not covered by the certifier | CURVED-SITES | open | — |

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
| NC.nary.par | n-ary's threaded `par_map` arms: only nary1 (one thread) runs | FALLBACK | open; no corpus evidence: proof must cover it, or add a fuzz family | — |
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
| A8.unneeded | today's path never refuses or is undecided where A8 would answer | FALLBACK | open | — |

## Curved screens (route B), for the curved D-steps

| id | contract | owner | status | file / theorem |
|---|---|---|---|---|
| S3 | a screened vertex is transversal | CURVED-SITES (landing: SPINE) | proven on dol-curved (8dbdc11d) | landing: spine PR 2, VertexScreen.lean |
| S4 | the angle comparator is the angular order | CURVED-SITES (landing: SPINE) | proven on dol-curved (83b5f267) | landing: spine PR 2, AngleOrder.lean |
