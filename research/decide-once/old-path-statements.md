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
| G1 | the output geometry is f64 | `⟦R⟧ = reg(..)` is false as equality | an owner decision: F1 (recommended) or F2 |
| G2 | **the retry ladder**: `plan` → `plan_retried` → `plan_fenced`, rerunning `plan_once` under the thread-local flags `CAREFUL`, `ABSORB_HAIR_ROOTS` (a forge-geom global), `CHORDS`, `NO_RUN_ON`, `FENCES` after an `Internal` error | a function's meaning depends on global flags set by a failed earlier run, so no one contract holds; the rungs pick between readings of a near-tangent case | each rung's question (hair root: two crossings or one touch; chord or curve; run-on; fence) becomes a screen plus an exact test at that site, in one pass; the ladder goes |
| G3 | `RETRIED` plus `forge_topo::check` on the result → `Lost::RetryInvalid` | a post-hoc partial check (forge_topo::check is not full ValidSolid, see decide-once's note) decides Ok or refuse, so an Ok after a retry is unproven | goes with G2 |
| G4 | **`UNDECIDED`** (thread-local): a touch with no local class continues, then `undecided_touch` refuses only if `forge_topo::check` finds `FaceWrongSide` | a guess plus a post-hoc partial check; a wrong Ok passes if the check misses it | D3: an exact local class at the touch, then `Undecided` refusal |
| G5 | `ODD` (thread-local), set inside `plan_once` and read by `plan_fenced` | hidden state between runs | goes with G2 |
| G6 | `pinched_new` / `pinched_old` after `apply` | a post-hoc detector on the built body, not a contract on the exact result | D9's statement: a pinch in R iff the exact result is non-manifold there. Provable if it reads only exact combinatorics; check whether it uses tolerances |
| G7 | `refuse_strays` (`stray_edges_from`) | a post-hoc check on f64 curves straying from their chains; under F1 it is the δ check | becomes F1's proven δ bound per edge, or a refusal with a true reason |
| G8 | **`imprint::mode()`** from env `FORGE_IMPRINT` (a static): `on` runs `imprint::column`, a second algorithm; `check` runs both and panics on a difference | the boolean's algorithm depends on an environment variable | the theorem covers the default (`off`); `column` either gets its own D-chain or is fixed off in release. A master decision |
| G9 | `SCRATCH` (thread-local) reused across booleans; `Face::query` (`FaceInfo`) kept across booleans and patched by `extend_query`/`drop_query_loops` | caches that outlive a call: a stale cache is a wrong answer | an invariant `CacheOK b` (the cached FaceInfo equals the one built fresh), preserved by every edit; a hypothesis of each D-step, proven for apply |
| G10 | `nary`'s `Batch` (a trait object: GPU or CPU) and its threads | an external verdict source; parallel order | `Batch` sound when decided (a hypothesis on the impl; CpuBatch proven); results merged in a fixed order |
| G11 | the many `LINEAR` / `RIM` tolerance tests (`on_edge`, `settle`, `tolerant_cuts`, `grown_tols`, `holds_edge`, `rolls_off`, `through_smooth_edge`, …) | each is a guess unless it is a proven screen | each becomes a screen plus exact; listed per function as the aeneas table arrives |
| G12 | the `decide_once` cargo feature (off by default) | a second boolean, which is not shipped | out of scope (§3b′); the theorem is for the default build |


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
does (2) and (3).

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

**One open point for decide-once Rust.** `point_in_face` is called today on f64 points that are only
within LINEAR of f's surface (trim's curve samples, split_face's meets). `whereF` needs a point *on*
the surface. The exact version needs those callers to pass the exact point (an exact meet, or a curve
parameter with an exact evaluation). Otherwise the question must be restated, e.g. "the foot of p on
f's surface", which itself needs an exact definition. This is where the callers' update gets its
statement; it should be settled before the Rust changes.

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
