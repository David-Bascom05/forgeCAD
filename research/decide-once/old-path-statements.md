# The old path, proven as written: statements, per function

**Owner ruling (2026-11-23):** "We need to prove also the old bool code as a whole, not just
individual functions." So this page works top-down:
- §E states one end-to-end theorem for today's boolean entry points;
- §D writes the chain of per-function contracts whose composition implies it;
- §G lists the places where today's code has no contract that composes;
- §0–§2 hold the per-function statements, each in its slot of §D.

## E. The end-to-end theorem

Over the body layer of §0: `⟦b⟧ : Set ℝ³` and `ValidSolid b` (dol-curved), on the translated types
(aeneas). `reg X = closure (interior X)`. `op` is one of ∪, ∩, \.

```lean
/-- Today's boolean, as translated: never a wrong `Ok`. -/
theorem boolean_correct (A B : Body) (op : Op) (feature : U64)
    (hA : ValidSolid A) (hB : ValidSolid B) :
    boolean A B op feature ⦃ r =>
      match r with
      | .ok R   => ValidSolid R ∧ Faithful R (reg (opSet op ⟦A⟧ ⟦B⟧)) ∧ NamesOK A B op R
      | .err e  => RefusalTrue A B op e ⦄
```

**Flat fallback (owner clarification, 2026-11-23).** For flat inputs, `boolean` runs today's
path first. If that path ends `Undecided`, or in a refusal, A8 runs (face_fast plus the inc core). If
A8 can't decide either, the result is a typed refusal. The theorem is unchanged in shape, and the
proof splits on the branch:
- **Today's path returns Ok:** the §D chain proves it. A wrong Ok here is not rescued by the
  fallback, so the §D chain must hold on its own.
- **Today's path refuses or is undecided, and A8 runs:**
  - A8's Ok is proven by `IncCoreSpec`/`EndToEndF` (done, for thin = []) plus R3 for face_fast
    (Refines from the input bodies to the face ledger), plus the write-out to a Body.
  - A8 needs the thin ≠ [] theorem wherever thin rooms occur.
  - A8's refusals are already `RefusalTrue`, via `coreI_zeroThickness_iff` and the collapse.
- **`Undecided`** means both paths were undecided. A refusal with a true reason (e.g.
  `ZeroThickness`) from today's path may be returned directly, or after A8 agrees: either way it is
  `RefusalTrue`.
- One check for decide-once Rust: the fallback must start from the **original inputs**, not from a
  part-built body. `boolean_into` edits `target` in place, so the in-place entry needs the target
  restored, or the path must only commit on success.

**Size hypothesis `InputSize A B`.** It collects every magnitude bound the screens' IEEE proofs
assume, as `LedgerWF2.small` did for the core:
- coordinates and plane distances ≤ about 9e9 at LINEAR (Screen/Coincide `plane_dist_sound`);
- cylinder measures ≤ 1e8 (`offp_margin`);
- all values finite (< about 1e150).

Binding today: offp_margin's S ≤ 1e8, which gives max |coord| ≤ about 1.6e7 (16 km in mm). Today's
code has no size check, so one `#[cold]` check at the four boolean entries (vertex max |coord|
against one `INPUT_SIZE` const) refuses with `Unhandled::InputTooLarge`. Each new screen adds its bound here. The list is reviewed for a single combined bound before the
theorem is closed. Outside these bounds the boolean refuses (an input out of scope); it never
answers without a proof.

The same statement covers:
- `boolean_into` and `boolean_unlinked`, for the target in place;
- `nary_with`, over n tools with the ops folded left;
- sequential regen, as the fold of `boolean_into`, by induction (each step keeps `ValidSolid`).

The parts:
- **`RefusalTrue A B op e`**: the refusal's reason holds of the exact result. `ZeroThickness at` means
  `reg (opSet ..)` is not a manifold solid at or near `at` (a pinch, or a zero-thickness wall);
  `Unhandled k` means the input is outside the declared scope `k`; `Undecided q` means question `q`
  is undecided after its exact test. `Internal` must become one of these, or be proven unreachable.
- **`NamesOK`**: each face, edge and vertex of R is named by the rule of boolean.rs's naming. This is
  stated per entity in §D7, and it is a separate obligation from geometry.
- **`Faithful R S`: the one point that needs an owner decision (G1).** The master's wording is
  `⟦R⟧ = reg(⟦A⟧ op ⟦B⟧)`. Today's result is written in f64: new vertices are rounded meets, and new
  edges are f64 intersection curves (ssi). So `⟦R⟧` is in general **not equal** to the exact set, only
  close to it. Two honest choices:
  - **(F1) faithful within δ:** R has the exact result's cell structure (the same faces, edges and
    vertices, matched one-to-one with incidences), and each entity lies within δ of its exact
    counterpart, with δ a proven bound per entity, as decide-once's `Refines` does for ledgers.
    `⟦R⟧ = reg(..)` then holds for the exact counterpart, `Rexact`, which R represents.
  - **(F2) exact output:** R carries exact geometry (exact meets, exact curves). That is a change to
    the data model, so by the ruling it is not today's code.

  I recommend F1. It is what "today's code, proven" can mean. Then ValidSolid R holds for `Rexact`,
  and for R itself when δ is below the body's own feature size, which is checked or refused.

  **Ruled (the master, 2026-11-23): F1**, refined to vertex clusters (see §0); faithful within a proven δ ("we can use floating point").
  Inputs are read the same way: `ValidSolid b := ∃ b*, Refines_δ b b* ∧ ∃ S, Bounds b* S`, and
  `⟦b⟧ := ⟦b*⟧`. `Bounds` is dol-curved's (ForgeCurved/BodyModel.lean, 30c0577b): exact, on the exact
  counterpart.

  **The counterpart is canonical (review, b0ed0be4).** dol-curved proved `bounds_unique` (under
  `Seeded`), `Refines δ`, edges and vertices, and `meet_on_edges`. ⟦b⟧ is unique per counterpart.
  The code decides on one specific counterpart: its own exact data, with each vertex the exact meet
  of its faces' carriers. So the theorem uses that one, `exactOf b`, and not an arbitrary b* within
  δ:
  - `ValidSolid δ b := ∃ bs, exactOf b = some bs ∧ ValidExact bs`;
  - `⟦b⟧ := denoteExact (exactOf b)`;
  - `exactOf = none` (a vertex whose meet isn't unique in its δ-ball) is a typed refusal.
  **Ruled (the master, 2026-11-23): option (iv), the refined F1.** (Recorded first as an open
  question; the record follows.) On real kernel bodies the f64 planes round a 4+ face
  vertex are only approximately concurrent, so exactOf's unique-meet vertex is `none` on normal
  parts. B:3387's trial refused bench part insert (VertexNotUnique at (40.3, −17, 36)).
  - Approved, option (iv), vertex clusters, for inputs and outputs alike:
    - the exact body is its exact carriers plus its topology;
    - each f64 vertex corresponds to a connected exact cluster of diameter ≤ δ;
    - `Refines_δ` is one-to-one up to vertex clusters;
    - `exactOf = none` only for invalid input;
    - ⟦b⟧ is the solid the exact planes bound under that topology;
    - a refusal happens only per decision, when a decision needs a cluster point that neither the
      screen nor exact can separate.
  - This replaces the snapping steer: decide-once's snap merges coincident planes, not concurrent
    ones.
  - Rejected:
    - (i) per-face canonical triples: the faces don't close;
    - (ii) decide-once's snap: it merges coincident planes, not concurrent ones;
    - (iii) f64 vertex points read exactly: the faces aren't planar.
  Done in BodyModel.lean, b66293ec (`exactOf`, `exactOf_refines`, `denote_spec`, `denote_refines`).
  Also asked: `exactOf = none` when a second branch of the meet enters an edge's δ-tube.
  `exactOf_refines` needs `hface` and `hedge` (each stored region and curve within δ of its exact
  meet). For inputs, this is a named hypothesis of `boolean_correct`, `FaithfulInput δ A`, supplied
  by the producing feature. For outputs, D8 proves it.

## D. The chain of contracts (boolean.rs `general`, as written)

Each step names its function, its contract, and what it hands the next. Composition: D1–D5 build
the facets of the arrangement of ∂A ∪ ∂B, restricted to the faces it touches, which is dol-curved's
`G`. D6 is the membership `m r i` (F6). D7 is the keep rule. D8 is assembly. The step from the keep
rule to `reg(opSet ..)` is the boundary lemma: ∂ reg(A op B) is the union of the facets separating a
kept room from an unkept one. That is FloodCorrect's statement, which dol-curved's F5/F6 give over
ℝ. This is the same shape as the proven decide-once core (classify → keep → unify), so its Lean
definitions (`keep`, `coreI`) are reused as the spec of D6–D8.

| # | function(s) | contract (over ⟦·⟧, exact) | hands on |
|---|---|---|---|
| D0 | `boolean` → clone, `boolean_into`, `into_compact` | compact keeps `⟦·⟧` and names; clone is the identity | — |
| D1 | `Index::new`, `face_box` (§1) | `⟦f⟧ ⊆ box f`; a face pair whose boxes miss has `⟦fa⟧ ∩ ⟦fb⟧ = ∅` | the candidate pairs |
| D2 | `meet_region`, `ssi::surfaces_in`, `plane_meet`, `crossings_near` | every point of `⟦fa⟧ ∩ ⟦fb⟧` lies on a returned curve inside the region; each curve lies on both surfaces (within δ under F1); a closed-form miss is `Unhandled::NoClosedForm`, which is scope | curves per pair |
| D3 | `touches`, `flows_on`, `crack_tip`, `local_class` | each isolated or tangential contact is listed with exact local classes on both sides; the `kept_a ∧ kept_b ∧ ¬flows ∧ ¬crack` refusal is `RefusalTrue` (a non-manifold result there) | touches, refusals |
| D4 | `trim`, `rank_pairs` | the segments are exactly the pieces of the D2 curves inside both `⟦fa⟧` and `⟦fb⟧`, split at boundary crossings, with ends at exact meets; rank orders them by an exact key | segments |
| D5 | `imprint`, `share_cuts`, `place_cuts`, `edge_split`, `split_face`, `plane_split` | the cut vertices are the exact points where segments meet edges; each face is split into regions whose union is `⟦f⟧` and whose interiors avoid ∂ of the other body. These regions are G's facets on f | facets (regions), the `Spec`s |
| D6 | `classify_specs`, `interior_point`, `probe`, `Index::classify` (§1a), coplanar sense | each region's class (In/Out/OnSame/OnOpposite) is `whereR` of the other solid at the region's interior, which is constant on the region by D5. Coplanar sense comes from the exact normals | classes |
| D7 | `keep_of`, `lean`, the naming fns (`region_name`, `piece_name`, `vertex_names`, `flush_name`, `sole_pieces_keep_names`) | the kept regions are exactly the facets that separate a kept room from an unkept one (the boundary lemma); the names are by the rule | kept set, names |
| D8 | `apply`, `edge_split`, joins/split notes, `sole_survivors` | R's faces are the kept regions glued along the cut edges; `⟦R⟧ = reg(opSet ..)` (under F1, for `Rexact`); ValidSolid R unless a pinch exists, which is detected exactly (D9) | R |
| D9 | `pinched_new`, `pinched_old`, `touched_old`, `undecided_touch`, `refuse_strays` | a refusal only when the exact result is non-manifold (`RefusalTrue`), and always then | Ok R or a refusal |
| Dn | `nary_with` | the same D1–D9 over n tools, with the rooms of the n-tool arrangement and the op fold as the keep rule; the `Batch` verdicts are sound when decided (3 = undecided goes to D6's exact test) | R |

## G. Where today's code has no contract that composes (guess sites, for the swap)

| # | site | why it doesn't compose | what the swap needs |
|---|---|---|---|
| G1 | the output geometry is f64 | `⟦R⟧ = reg(..)` is false as equality | **ruled F1**: `Refines_δ R R*` and `⟦R*⟧ = reg(..)` |
| G2 | **the retry ladder**: `plan` → `plan_retried` → `plan_fenced`, rerunning `plan_once` under the thread-local flags `CAREFUL`, `ABSORB_HAIR_ROOTS` (a forge-geom global), `CHORDS`, `NO_RUN_ON`, `FENCES` after an `Internal` error | a function's meaning depends on global flags set by a failed earlier run, so no one contract holds; the rungs pick between readings of a near-tangent case | each rung's question (hair root: two crossings or one touch; chord or curve; run-on; fence) becomes a screen plus an exact test at that site, in one pass; the ladder goes |
| G3 | `RETRIED` plus `forge_topo::check` on the result → `Lost::RetryInvalid` | a post-hoc partial check (forge_topo::check is not full ValidSolid, see decide-once's note) decides Ok or refuse, so an Ok after a retry is unproven | goes with G2 |
| G4 | **`UNDECIDED`** (thread-local): a touch with no local class continues, then `undecided_touch` refuses only if `forge_topo::check` finds `FaceWrongSide` | a guess plus a post-hoc partial check; a wrong Ok passes if the check misses it | D3: an exact local class at the touch, then `Undecided` refusal |
| G5 | `ODD` (thread-local), set inside `plan_once` and read by `plan_fenced` | hidden state between runs | goes with G2 |
| G6 | `pinched_new` / `pinched_old` after `apply` | a post-hoc detector on the built body, not a contract on the exact result | D9's statement: a pinch in R iff the exact result is non-manifold there. Provable if it reads only exact combinatorics; check whether it uses tolerances |
| G7 | `refuse_strays` (`stray_edges_from`) | a post-hoc check on f64 curves straying from their chains; under F1 it is the δ check | becomes F1's proven δ bound per edge, or a refusal with a true reason |
| G8 | **`imprint::mode()`** from env `FORGE_IMPRINT` (a static): `on` runs `imprint::column`, a second algorithm; `check` runs both and panics on a difference | the boolean's algorithm depends on an environment variable | **ruled: a hidden input.** Release runs the default path only; `column` is out of the theorem's scope. Fix: a compile-time feature or removal, with nothing read from env in release |
| G9 | `SCRATCH` (thread-local) reused across booleans; `Face::query` (`FaceInfo`) kept across booleans and patched by `extend_query`/`drop_query_loops` | caches that outlive a call: a stale cache is a wrong answer | an invariant `CacheOK b` (the cached FaceInfo equals the one built fresh), preserved by every edit; a hypothesis of each D-step, proven for apply |
| G10 | `nary`'s `Batch` (a trait object: GPU or CPU) and its threads | an external verdict source; parallel order | `Batch` sound when decided (a hypothesis on the impl; CpuBatch proven); results merged in a fixed order |
| G11 | the many `LINEAR` / `RIM` tolerance tests (`on_edge`, `settle`, `tolerant_cuts`, `grown_tols`, `holds_edge`, `rolls_off`, `through_smooth_edge`, …) | each is a guess unless it is a proven screen | each becomes a screen plus exact; listed per function as the aeneas table arrives |
| G13 | `face_point`'s inward step (min(0.01·len, 0.05)) is never checked against cuts or other loops | a probe may land in the wrong region: a wrong class, not an undecided one | a screened distance check with an exact fallback, then another probe or a typed refusal |
| G14 | trim_in B:2614/2938: `(box contains && !CAREFUL) \|\| point_in_face(..)` accepts a hit on the face's box alone | a box contains far more than the face: a guess that the hit is in the face | point_in_face_at on the exact point, behind a screen (the box test may stay as a fast *reject* only) |
| G15 | after an exact `On` at a trim_in stretch midpoint (M exactly on an edge of f_k), today's `past`/hair handling decides the stretch | not yet stated | review separately: state what the stretch's class is when its midpoint is on ∂f_k exactly |
| G12 | the `decide_once` cargo feature (off by default) | ships as the flat fallback (owner clarification) | it becomes the fallback branch: on in release, entered only after today's path refuses or is undecided, with no env switch; proven by IncCoreSpec/EndToEndF plus R3 |


Owner decisions (face-arrangement.md §3b′, c5ecf9be):
- prove today's flat boolean (boolean.rs, classify.rs, imprint.rs, nary.rs) as written, via Aeneas;
- each guess step becomes a proven per-question screen, answering *decided* or *undecided*;
- an undecided question goes to the exact code for that question only, and a question exact can't
  decide either is a typed refusal.

This page states what each function must be proven to do, in the order the aeneas session
translates them. Each statement is a refinement: the translated function, read through an
abstraction of its inputs, equals or soundly approximates a Lean definition over ℝ. Statements are
written before the translation lands, so the proofs can start as soon as it elaborates.

## 0. The missing layer: a Lean model of a `forge_topo::Body`

Every statement below is about a `Body`. The repository has **no Lean model of one**: the decide-once
models (`ForgeExact.DecideOnce`) are over ledgers, and forge-topo is not translated.

Needed:
1. **The translated types.** Aeneas's `Body`, `Face`, `Loop`, `Coedge`, `Edge` and `Vertex`, with
   `Surface` and `Curve` as translated enums, and the `Proc`/`ProcCurve` arms opaque (the aeneas
   session's markers).
2. **An abstraction `⟦b⟧ : Set ℝ³`, the solid a body bounds,** plus `⟦f⟧ : Set ℝ³` for each face: its
   surface's points inside its trimming loops. These are defined only under a validity predicate,
   `ValidSolid b`: faces meet only along common edges, the loops are closed and properly nested, and
   the shell is closed and oriented.
3. **The bridge to dol-curved's F-series.** F1–F6 work over an abstract arrangement `G` with rooms and
   facets. `⟦b⟧` should be a union of rooms of the arrangement of the body's own faces, so F5/F6 apply
   to it unchanged.

This layer is the first piece of Lean, before any refinement proof. It is shared with the curved
plan. **Owners (the master, 2026-11-23):** aeneas does (1), as part of the Charon fix. dol-curved
does (2) and (3): ForgeCurved/BodyModel.lean (30c0577b) has `FaceModel`, `BodyModel`, `Bounds`,
`ValidSolid`, `denote` and the bridge. Asked of it (review, 2026-11-23): uniqueness of `S`,
`Refines_δ` with b*, and edges and vertices with their incidences (for F1 and the names).

## 1. classify.rs

| function | statement |
|---|---|
| `face_box(b, f)` | For a live face, `⟦f⟧ ⊆ face_box b f` (with the box closed). For a removed face, the box is empty. |
| `Index::new(b)` | `ix.bbox ⊇ ⋃ live f, face_box b f`. Both branches (faces_box when there are more than 64 faces, else the fold of hulls) give such a box. |
| `Index::face_box`, `boxes` | Equal to `face_box b f` (live) or `EMPTY` (removed), whether cached or not. The `OnceLock` changes cost only. |
| `coedges_meeting(f, bx)` | Every coedge of f whose edge's box meets bx is in the result, and only those (the bands select a superset, then filter). |
| `point_in_face(f, p)`, for p within LINEAR of f's surface | `Inside ⇒` p's foot on the surface is in `⟦f⟧` and farther than the tolerance from its boundary. `Outside ⇒` the foot is not in `⟦f⟧`. `On ⇒` the foot is within the tolerance of ∂f. **Today the result is a guess wherever an f64 parity or a `sag_bound` decides it** (see flags). |
| `classify(p)` | `Inside ⇒ p ∈ interior ⟦b⟧`; `Outside ⇒ p ∉ ⟦b⟧`; `On ⇒ dist(p, ∂⟦b⟧) ≤ LINEAR`. |

### 1a. The exact `On` (ruling, 2026-11-23): the Lean statements

The master ruled: no guesses, so `On` becomes exact. A proven screen decides In/Out/On where it can.
Otherwise an exact test runs for that point. The all-rays-degenerate case goes to an exact ray test
for that point, and if that can't decide, a typed refusal. `On` never means "undecided". trim,
split_face and classify_specs are updated to consume the exact `On`. decide-once Rust owns the
Rust; these are the statements it is proven against.

```lean
/-- The exact answer for a point and a solid. -/
def whereR (S : Set ℝ³) (p : ℝ³) : Where :=
  if p ∈ interior S then .inside else if p ∈ frontier S then .on else .outside

/-- The exact answer for a point of a face's surface and the face. -/
def whereF (F : Set ℝ³) (p : ℝ³) : Where :=      -- F = ⟦f⟧, relative to its surface
  if p ∈ relInterior F then .inside else if p ∈ relFrontier F then .on else .outside

-- `classify` now returns `Result Where Refusal`; read the input point as the exact dyadic it is.
theorem classify_spec (b : Body) (hv : ValidSolid b) (p : F64Point3) :
    classify (Index.new b) p ⦃ r => ∀ w, r = .ok w → w = whereR ⟦b⟧ p.toReal ⦄

-- Never refuses on flat bodies: planes and dyadic points always decide exactly.
theorem classify_total_flat (b : Body) (hv : ValidSolid b) (hf : Flat b) (p : F64Point3) :
    classify (Index.new b) p ⦃ r => ∃ w, r = .ok w ⦄

theorem point_in_face_spec (b : Body) (hv : ValidSolid b) (f : FaceId) (p : ExactPt)
    (hp : p.toReal ∈ surf ⟦b⟧ f) :
    point_in_face (Index.new b) f p ⦃ r => ∀ w, r = .ok w → w = whereF ⟦f⟧ p.toReal ⦄
```

Proof shape:
- each screen is a lemma: when it decides, its answer equals the exact one (IEEE model);
- each exact fallback refines `whereR`/`whereF` directly;
- parity uses dol-curved's F5/F6 (a ray from p crosses `∂⟦b⟧` an odd number of times iff p is inside,
  for a ray in general position);
- the exact ray test supplies the general position, or refuses.

**The callers' points (settled with decide-once Rust, 2026-11-23).** About 40 call sites of
`point_in_face` and `classify` in boolean.rs and nary.rs fall into two kinds:

- **(A) Constructed points**, meant to be on a surface: a curve ∩ surface hit, a cut, or an edge's
  `curve.eval(t)`. These are in trim_in, find, local_split, tolerant_cuts, through_smooth_edge,
  nary_body's hit tests, and plan_once_in's NotAnalytic `off`.
  - **The caller passes the exact point**: for flat faces, the meet of three carriers (the two faces'
    planes and the edge's across plane), as its key. That point is on f's plane by construction.
  - Obligations: `pt.toReal ∈ carrier f` (so `point_in_face_spec` applies), and the f64 point the
    code also carries is within δ of it (F1).
  - Curved hits are curved Rust's: an exact curve parameter or an exact evaluation, with the same
    spec.
- **(B) Probe points**, chosen to stand for a region: face_point and spare_probes in classify_specs,
  survivors, crack_tip, local_class and its midpoints, touches, imprint, plan_once_in's probes, and
  nary_body's.
  - **Both `point_in_face` and `classify` take the foot**, the exact orthogonal projection of the
    dyadic probe onto f's exact carrier:
    ```lean
    /-- The foot of `p` on the plane `n·x = d` (n, d exact rationals): a rational point. -/
    def foot (n : ℚ³) (d : ℚ) (p : ℚ³) : ℚ³ := p - ((n ⬝ p - d) / (n ⬝ n)) • n
    theorem foot_on : n ≠ 0 → n ⬝ foot n d p = d
    ```
  - Why `classify` takes the foot as well, and not the raw dyadic probe: the region's class is
    constant on the region (D5/D6), but a raw probe sits off the face by rounding. Where the region
    lies closer than that rounding to the other body's boundary, the raw probe can classify
    differently from the region. The foot lies on the face exactly.
  - The obligation, per probe site: **`foot p ∈ relInterior region`**, the region the probe stands
    for. Then `whereR ⟦B⟧ (foot p)` and `whereF ⟦f⟧ (foot p)` are the region's class (D6).
  - Curved faces: curved Rust defines the foot (or an exact on-surface point near p) per kind.
- **Probe sites and their regions** (decide-once Rust, 2026-11-23):

  | site | region the foot must be in | the step's status |
  |---|---|---|
  | `face_point(f)` (classify_specs, survivors, plan_once_in) | the region of f, as cut by this boolean, next to f's first edge-middle | **unchecked guess** (G13): nothing checks that no cut or other loop comes within the step. It needs a screened distance with an exact fallback, then another probe or a typed refusal |
  | `spare_probes(s)` | Spec s's region | checked by an f64 distance filter, which becomes a screened distance with an exact fallback |
  | `local_class(f, p, ..)` (touches, crack_tip) | the sector of f at p between the other body's traces through p, per direction | **a guess** (step growth): replaced by D3's exact sector class (the carriers through p ordered by exact angle, face_fast's machinery), as its own step after (A) and the feet. The foot is used meanwhile |
  | midpoints in local_split, imprint, nary_body | the open arc between the two cuts | kind (A): the exact midpoint of the two exact meets, by construction |

- **The rule for both kinds:**
  1. a screen decides In/Out/On-surface when it can;
  2. otherwise the exact test at the exact point or the foot;
  3. if every ray is degenerate, the exact ray test at that point;
  4. if that can't decide either, a typed refusal.

  Measured (flat-guess-site-rates.md, 4d260609): On ≤ 1e-12 and almost always exactly 0, and the
  all-rays branch fired 0 times, so the exact path is rare.
- **Measured (decide-once Rust, 2026-11-23):**
  - the first (A) attempt, at trim_in's planar hit sites, ran 58 times on the gate and fuzz 1–40, never with all three faces planar; it cost +0.53% on coplanar by un-inlining on_edge, so it was reverted;
  - flat `On` comes mostly from B:3387, trim_in's `at(k, p)` midpoint between two cuts (3,011 of 5,602), and that is the next flat step;
  - the nine point_in_face near-misses are all curved (B:2845, B:3315);
  - rule for every exact fallback: one `#[cold] #[inline(never)]` helper with a single call site, measured against the inlining effect.
- **B:3387, approved (2026-11-23):** a stretch of a flat pair line between consecutive cuts X_i, X_j
  (exact meets). Its class is `whereF ⟦f_k⟧ M` with `M = (X_i + X_j)/2`.
  - Screen: today's f64 point_in_face at q = eval(mid), sound when
    |q − M| + δ_input + parity rounding < reach.
  - Cold helper, run only on today's On:
    - X_i = X_j tested exactly (the stretch is empty);
    - M on an edge exactly gives `On` (then G15);
    - otherwise the exact parity, with a ray that avoids every exact vertex, so it is never undecided;
    - the only refusal is VertexNotUnique.
- **Rust plan:**
  - exact companions: `classify_exact(ExactPt) → Result<Where, Refusal>` and
    `point_in_face_at(f, ExactPt)`, where `ExactPt` is a key or a foot;
  - callers move kind by kind, (A) first, in trim_in;
  - the f64 signatures stay until every caller has moved.

**Flags (guess steps in classify.rs, for the IEEE screens):**
1. **`classify` returns `On` when every ray in `RAYS` is degenerate** (`OnSurface`, `Touching`, or a
   hit with `point_in_face = On`). So `On` means both "near the boundary" and "undecided", and the
   `On ⇒ near ∂` statement is false there. **Ruled** (§1a): an exact ray test for this point, then a
   typed refusal; never `On`.
2. **`On` by `surface.distance(p) ≤ LINEAR`, then `point_in_face ≠ Outside`.** This is a tolerance
   band, not a fact. **Ruled: exact** (§1a): a screen, then an exact on-face test.
3. **The `t > LINEAR` filter on ray hits** drops a crossing within LINEAR of p. That is sound only
   when p is not On, which step 1 established within tolerance only.
4. **`line_surface` hits and `Contact::Touching`** are f64 decisions; each needs a screen with an exact
   fallback for that hit.
5. **`point_in_face`'s `Simple` closed forms, `sag_bound`, the `sphere_nearest` pole rule, and
   `other_turn_inside`** are all f64 decisions. Each needs its own screen statement.
6. **Translation constructs (not a refactor request, input for the aeneas session):**
   - `Face::query: Cached<Arc<dyn Any>>` read through `downcast_ref::<FaceInfo>()` in `info`;
   - `OnceLock` in `Index::boxes`, `FaceInfo::drawn` and `simple`;
   - the `Box<dyn Iterator>` in `edges_near`.
   If Aeneas can't take one of these once the tooling is fixed, it goes to the master, named exactly.

## 2. Next, as the translations land

These follow the aeneas session's order: ssi (`surfaces_in`, `plane_meet`, `crossings_near`,
`meet_region`) and unify; imprint (`meet`); boolean (the pair loop `plan_once_in`, `trim`,
`split_face`, `classify_specs`, `touches`, `touched`/`pinched`); then nary (`meet3`). Their
statements are written here when the aeneas table for each arrives.
