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

## E. The end-to-end theorem and its hypotheses

| id | contract | owner | status | file / theorem |
|---|---|---|---|---|
| E | `boolean_correct`: Ok ⇒ ValidSolid ∧ Faithful (F1) ∧ NamesOK; Err ⇒ RefusalTrue | SPINE | open | verify/forge-bool-proof |
| E.comp | the composition: D0–D9 (and Dn) ⇒ E | SPINE | open | verify/forge-bool-proof |
| E.fb | the flat fallback branch: today's path refuses or is undecided ⇒ A8, then a typed refusal; A8 starts from the original inputs | FALLBACK | open | — |
| E.ref | `RefusalTrue A B op e` per refusal kind (`ZeroThickness`, `Unhandled k`, `Undecided q`); `Internal` mapped or unreachable | SPINE (stmt), FALLBACK (`Internal`) | open | — |
| E.names | `NamesOK`, per entity (D7) | SPINE (stmt) | open | — |
| E.size | `InputSize A B`, and `boolean_preserves_InputSize` | SPINE | open | — |
| E.size.rs | the `#[cold]` entry check refusing `Unhandled::InputTooLarge` | FLAT-SITES | open | kernel/forge-bool |
| H.fin | `FaithfulInput δ A`: each stored region and curve within δ of its exact meet | hyp (producing feature) | hyp | BodyModel `exactOf_refines`'s `hface`, `hedge` |
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
| T.strays | `topo::strays` refines `checkStrays` (L4) | SPINE | proven on rebase-fundamentals | `forge_topo.strays_refines` |
| T.links | `topo::links` refines `checkLinks` (L4) | SPINE | proven on rebase-fundamentals | `forge_topo.links_refines` |
| T.shells | `topo_shell::shell_faces` refines `checkShellFaces` (L4) | SPINE | proven on rebase-fundamentals | `forge_topo.shell_faces_refines` |
| T.edges | `topo_edges::edges`, `loops` refine `checkEdges`, `checkLoops` (L4) | SPINE | proven on rebase-fundamentals | `forge_topo.edges_refines`, `loops_refines` |
| T.euler | `check_shell_euler` (stamped distinct count = eraseDups) | SPINE | open | — |
| T.fans | `check_fans` (corner table as an opaque view method) | SPINE | open | — |
| T.all | `check_known`'s combinatorial part composed (`Model.check`) | SPINE | open | — |
| T.valid | `Valid` (topology) ∧ geometry ⇒ `WellFormed` of the exact counterpart | SPINE | open | — |

## D. The chain of contracts (boolean.rs `general`)

| id | function(s) | contract | owner | status | file / theorem |
|---|---|---|---|---|---|
| D0 | `boolean` → clone, `boolean_into`, `into_compact` | compact keeps ⟦·⟧ and names; clone is the identity | SPINE | open | — |
| D1 | `Index::new`, `face_box` | ⟦f⟧ ⊆ box f; boxes that miss ⇒ faces miss | FLAT-SITES | open | — |
| D2 | `meet_region`, `ssi::surfaces_in`, `plane_meet`, `crossings_near` | every point of ⟦fa⟧ ∩ ⟦fb⟧ is on a returned curve (within δ, F1); closed-form miss = `Unhandled::NoClosedForm` | TRANSLATE | open; translated: `ssi::surfaces_in`, `closed_or_flush`, `closed_form`, `nearly_flush`, `turned_flush`, `spread_apart` with forge-geom's `plane_surface`/`plane_plane` (`meet_region`, `crossings_near` wait for FALLBACK's ladder argument; `plane_meet` is curved) | verify/aeneas/forge-bool `ForgeBool/Funs.lean` |
| D3 | `touches`, `flows_on`, `crack_tip`, `local_class` | every contact listed with exact local classes; the non-manifold refusal is `RefusalTrue` | FALLBACK (with G4) | open | — |
| D4 | `trim`, `rank_pairs` | segments = the D2 curves' pieces inside both faces, ends at exact meets; rank by an exact key | FLAT-SITES (with G14, G15) | open | — |
| D5 | `imprint`, `share_cuts`, `place_cuts`, `edge_split`, `split_face`, `plane_split` | the regions are the arrangement's facets on f (`FacetData`) | SPINE (statement), FLAT-SITES (proof, after §1a) | open | `place_cuts`: research/kernel-audit/coverage/aeneas (proven, L4) |
| D6 | `classify_specs`, `interior_point`, `probe`, `Index::classify` (§1a), coplanar sense | each region's class is `whereR` of the other solid at the region | FLAT-SITES | open | — |
| D7 | `keep_of`, `lean`, the naming functions | kept = facets separating a kept room from an unkept one; names by the rule | SPINE (keep), FALLBACK (names, with A8's edge-name parity) | open | — |
| D8 | `apply`, `edge_split`, joins/splits, `sole_survivors` | R's exact counterpart: faces = the kept facets, `Bounds Rs (reg (opSet ⟦A⟧ ⟦B⟧))`, `WellFormed` | SPINE | open | — |
| D9 | `pinched_new`, `pinched_old`, `touched_old`, `undecided_touch`, `refuse_strays` | refuses iff the exact result is non-manifold there | FALLBACK (G6, G7) | open | — |
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
| G1 | the output geometry is f64 | SPINE | proven (ruled F1; B.F1) | BodyModel.lean |
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
| G11c | curved `LINEAR`/`RIM` tolerance tests (curved-fallback-rates.md) | CURVED-SITES | open | — |
| C.coinc | `ssi::coincident`, `same_carrier`: coincident carriers decided exactly | CURVED-SITES | open | — |
| C.unify | unify's curved decisions | CURVED-SITES | open | — |
| C.cuts | the `on_edge` / `place_cuts` coincidence sites on curved faces | CURVED-SITES | open | — |
| C.poles | poles and seams (`sphere_nearest`'s pole rule, periodic seams) | CURVED-SITES | open | — |
| G12 | the `decide_once` feature becomes the fallback branch | FALLBACK | open | — |
| G13 | `face_point`'s unchecked inward step | FLAT-SITES | open | — |
| G14 | trim_in's box-only accept | FLAT-SITES | open | — |
| G15 | the stretch whose midpoint is exactly on ∂f_k | FLAT-SITES | open | — |
| M.ct | forge-geom `meet::intersect`, cylinder × torus (`Form::Poly4`): its events from a sampled discriminant, which lost one of two loops on fuzz295 f69 (a wrong `Ok`) | FALLBACK (fix), CURVED-SITES (#7, certified isolation) | interim fix (PR proof/fix-torus-loops): a root-count scan where the discriminant flags double roots; NOT provably complete (a loop narrower than one sample can be lost): UNKNOWN | kernel/forge-geom/src/meet_tests.rs `thin_torus_on_a_wide_cylinder_both_loops` |

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

