# The old path, proven as written: statements, per function

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
