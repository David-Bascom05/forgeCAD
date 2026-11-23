# A8: decide-once over the face arrangement (design, for sign-off)

Status: **signed off by the master** (ab628a33, conditions folded in). Draft history: It was written by the Lean side and merged with bool-research's draft (6b776b5 on
-bool-research: the cell definition, the radial sides and the measurements are theirs). It goes
to the master for sign-off. No proof work starts until it is signed off. The current proven core
(`forge_bool_core`, `end_to_end_proof`) stays shipped-ready behind the switch, and nothing is
deleted.

Why: research/booleans/small-booleans.md §5, corrected in §3a below. The spec's cells are the
sign classes of the whole *plane* arrangement. `arrange` builds about 300 section cells per
boolean on coplanar seq, most of which never become pieces, and the ledger holds about 231 cells.
A *face* arrangement cuts a face only where another face crosses it. That is about 10× less
arrangement work and about 1.5× fewer ledger cells (§1a), and it removes covector scans from the
core.

## 0. The proposal in one paragraph

The cells become **facets** (pieces of faces, cut only by their cut sets, §1a), **edges**,
**verts**, and **rooms**. A room is a class of facet-sides joined radially round edges, and it
replaces a tope. The ledger
gives their incidence explicitly as id tables, not covectors.

The proof does **not** start over. The plane-arrangement ledger that the sampler builds stays the
proof's reference. It is a Lean definition that is never run. The face-arrangement ledger is proven
to be a **coarsening** of it: every plane cell maps to the face-arrangement cell it lies in. A
**simulation theorem** then says the new core's outcome is the old core's outcome, coarsened.
So `end_to_end_proof`'s conclusions (valid body, exact boundary facets, `reg` of the boolean,
ZeroThickness ↔ non-manifold) carry over unchanged to the new core through one map.

The new Rust core has no covectors at all. It drops cov, find, intern, index, sort, hash, split and
between, and its Aeneas proof is about half the size of today's.

## 1a. Cells (bool-research's definition)

The region `R°` holds the near faces `f`. Each has a closed polygon `P_f` on its carrier class,
taken after the snap and clipped by the region's walls. The walls clip faces; they are not
carriers.

A face's **cut set** is
`X_f = ∂P_f ∪ ⋃ {P_f ∩ P_g : g near, cls g ≠ cls f} ∪ ⋃ {P_f ∩ ∂P_g : cls g = cls f, body g ≠ body f}`.
The last term is the imprint of a coplanar overlap.

- **Verts** are where `X_f`'s segments end or meet: a `meet3`, or a polygon vertex.
- **Edges** are the components of `⋃ X_f` minus the verts.
- **Facets** are the components of `relint P_f ∖ X_f`. A coplanar overlap gives one facet that is
  a piece of both faces, as now.
- **Rooms.** Round each edge, the facets have an exact radial order: a degree-2 sign in the plane
  coefficients, the predicate trueform and Lévy use. Consecutive facet-sides face one wedge, so
  union-find joins them, and the classes are the rooms.

  One geometric component may hold several rooms. The outside of an inner shell and the inside of
  an outer one share no edge, for example. That is harmless: each room has its own exact witness.

Measured per boolean (bool-research, callgrind), plane-ledger pieces against the estimated
face-arrangement facets:
- coplanar: 47 → 30 (231 ledger cells in all);
- housing: 19 → 15;
- upto: 54 → 20.

The arrangement *work* falls about 10×: 9 crossing segments, against 9.5 crossing classes over
whole sections. The ledger cells fall about 1.5×. With explicit incidence, each core step is
O(degree), not an O(classes) covector scan.

## 1. The ledger interface (Rust, `forge_bool_core` v2)

Every id is a `u32` index. Nothing is a covector.

```rust
pub struct Csr { pub start: Vec<u32>, pub ids: Vec<u32> }  // row k = ids[start[k] .. start[k+1]]

pub struct Ledger {
    pub nb: u32,                          // bodies 0..nb
    pub face_body: Vec<BodyId>,           // per input face (as now)
    pub face_class: Vec<u32>,             // per input face, its carrier class (as now)
    pub n_rooms: u32,                     // rooms 0..n_rooms
    pub facet_class: Vec<u32>,            // per facet, its carrier class
    pub facet_rooms: Vec<(u32, u32)>,     // per facet: (room on the class normal's + side, room on its − side)
    pub pieces: Vec<(FacetId, FaceId)>,   // decided: the facet lies in the face (every facet is in one or more)
    pub inside: Vec<(RoomId, BodyId)>,    // decided: witnesses, one ray per flood class (as now, per room)
    pub edge_facets: Csr,                 // per edge: the facets whose closure holds it
    pub vert_edges: Csr,                  // per vert: the edges whose closure holds it
    pub vert_sectors: SecCsr,             // per vert: its sectors, (facet, bounding edge a, bounding edge b)
}
pub struct SecCsr { pub start: Vec<u32>, pub sec: Vec<(u32, u32, u32)> }
// (edge_facets lists a facet once per wing at that edge: at most twice)
pub struct MetricLedger { pub thin: Vec<RoomId>, pub enclosed: Vec<RoomId> }   // per room (as now, per side cell)
// Body: faces list FacetIds, edges list EdgeIds, verts list VertIds; Outcome, Refusal and Rule are unchanged.
```

What each table means:

- **Rooms** are the radial classes of facet-sides (§1a), inside the region `R°`. The walls clip
  faces but are not facets: no body owns a wall. A room reaching a wall is not `enclosed`.
- **Facets** are only pieces of faces. A plane piece that lies in no face does not exist here,
  because it lies inside a room.
- **The two sides of a facet are different rooms.** This always holds, since crossing a piece of
  body `i`'s face changes membership in `i`. `LedgerWF` checks it.
- **Facets need not be convex or simply connected.** An annular piece round a boss is fine. The
  core is combinatorial. The write gets each facet's boundary loops from the builder, outside the
  core.
- **Edges and verts** are the 1- and 0-cells round the facets, as now. They are keyed by their
  carrier pairs and triples, which makes τ and ν direct to define.
- **Wings and sectors (rule W2, uniform for flat and curved).** A facet can meet a cell in its
  closure more than once. At an edge it can arrive from both sides (two *wings*). At a vert it can
  meet in several angular *sectors* (a cone apex on a plate, a pinched piece). This happens wherever
  a solid only touches a face, in flat inputs as well as curved.

  The incidence rows therefore count wings and sectors, not facets:
  - `edge_facets[e]` lists a facet once per wing, so a facet appears at most twice;
  - `vert_sectors[v]` lists `(facet, edge a, edge b)` per sector, the two edges bounding it at `v`.
    A facet may appear in several sectors. This replaces `vert_facets`. `vert_edges` stays.

  For the core:
  - `around` and bad edge count wings, so a touching union (four result wings) is refused, as it
    must be;
  - `fanRel` joins two sectors at `v` that share a bounding edge;
  - `facesAt` dedups to faces as now;
  - `joinRel` may relate a facet to itself, which is harmless: classes are reflexive anyway.

  Facet ids alone at verts would not be enough. A pinched facet listed once can merge fan classes
  that the reference keeps apart, and so hide a non-manifold vert. That is why verts list sectors,
  not facets.

  For convex flat facets every multiplicity is 1 and every vert has one sector per facet.

  The common flat wing is a **slit**: a crossing segment that ends inside a face, wherever a tool
  edge pierces a face's interior. The face's facet then reaches the slit's edges from both sides,
  so it is listed twice. A builder that prefers to cut instead extends the slit along its carrier
  line to the facet boundary or to the first cut.

  `Refines`' (e) becomes a bijection from the reference's fine facets round an edge onto its wings:
  `List.Perm` of the mapped list. Each plane through an edge holds at most two fine facets round
  it, one per side, so the map is one to one. (v) is a surjection onto the sectors, with connected
  fibres.

  A builder may still cut a wing apart along a seam that lies in the reference arrangement (W1).
  The cut pieces are just more facets, with multiplicity 1, so the result is a valid W2 ledger, and
  the core and proofs do not care which a builder chose. Curved cuts along its Tier-0 seam pieces
  where that is natural (curved-design.md §7). It emits wings or sectors where cutting is not
  natural, for example tangent cylinders.

The core reads the tables as follows. This is today's algorithm with lookups in place of
covector arithmetic.

| stage | today (covectors) | A8 (ids) |
|---|---|---|
| split check | `isFacet`, `zeroIdx = cls`, one face per body | `facet_class[φ] = cls f`, one face per body per facet, sides distinct |
| sides | `φ.plus` / `φ.minus`, interned (`split`) | `facet_rooms[φ]` |
| classify | flood over topes across non-pieces of `i`, then a witness | the same over rooms |
| keep, collapse | per tope | per room (metric tables per room) |
| result | facets with one side kept | the same |
| around(x) | result facets with `x ≤ φ` (enumerated by covector) | `edge_facets[x]` (wings) / `vert_sectors[x]` filtered by result |
| bad edge / bad vert / fan | `Cov.le` between cells | `vert_edges`, `edge_facets` |
| unify | `zeroIdx`, `orient`, a shared edge | `facet_class`, `orient` (by room keep), a shared edge from `edge_facets` |

`LedgerWF` v2 has these parts:
- Each Csr is well formed: `start` is monotone, starts at 0 and ends at `ids.len()`; ids are in
  range; an `edge_facets` row holds a facet at most twice; `vert_edges` rows have no repeats.
- Each sector's two edges are in its vert's `vert_edges` row.
- Every id is in range, and every count is below `u32::MAX`.
- The two sides of each facet are distinct rooms.
- Every facet is a piece of some face.

The core packing that was planned for v1 (packed `Row`s) is superseded: v2 has no rows. If the
builder still uses covectors locally (per-face 2D signs), packing them is the builder's business.

## 2. The Lean core over incidence, and why the proofs carry over

### 2.1 One core, two ledgers

`IncLedger` is the table shape above, in Lean, and `coreI : IncLedger → Rule → OutcomeI` is
Core.lean's algorithm restated over it, at about the same length (~300 lines). There are two
instances:

- **`inc L`**, built from today's plane ledger `L`:
  - rooms are `L.topes`;
  - facets are `L.facets`, with sides `φ.plus` and `φ.minus`;
  - incidence is `Cov.le`.

  Lemma `core_inc : core L = coreI (inc L)`, read through the inclusion of ids. It is a
  definitional unfolding, about 400 lines. The point is that every theorem of Correct.lean and
  `end_to_end_proof` is then a theorem about `coreI` on `inc (sampleLedger …)`.
- **`toLF l`**, built from the Rust v2 ledger: a direct read, as `toL` is today.

### 2.2 Coarsening

`Refines (F : IncLedger) (L : Ledger) (ps A)`: there are maps
- σ : `L.facets → facets F`,
- τ : `L.edges → Option (edges F)`,
- ν : `L.verts → Option (verts F)`,

with these properties:

- **(r) rooms, as a relation.** In effect ρ is a relation, total and onto, between the fine
  facet-sides and F's rooms: `(φ, ±) ρ R` when `R` is the room on that side of `σ φ`. It is not a
  function on topes. (bool-research's review: rooms are radial side classes, not components, so a
  map from topes to rooms would be ill-defined when a tope touches two shells. Hence the condition
  is stated per facet-side.) For every fine facet `φ` and body `i`, F's membership of the room on each side
  of `σ φ`, through its flood and `inside`, equals the reference ledger's `memb i φ.plus` (resp.
  `φ.minus`). Rooms are reached through facet-sides, so no map from topes to rooms is needed, and
  several rooms in one geometric component are allowed. The condition is combinatorial, so the simulation
  theorem has no geometry in it. Geometry enters only in proving `Refines` for a builder, where
  `room_const` gives it. The theorem is therefore stated over any reference ledger: the flat
  `sampleLedger`, or dol-curved's carrier ledger, which is `ExactK` (CorrectK.lean). Curved then
  instantiates `EndToEndF` with no second simulation proof.
- **(s) facets.**
  - σ is onto.
  - It keeps the class, and the sense of the sides: `+` of `σ φ` is on `φ.plus`'s side.
  - It keeps the pieces: `(φ, f) ∈ L.pieces ↔ (σ φ, f) ∈ F.pieces`.
  - Each fibre σ⁻¹(F-facet) is connected through fine edges with τ = none. A face-arrangement
    facet is one connected piece.
- **(e) edges.** If `τ e = some E`, σ is a bijection from the fine facets round `e` onto
  the wings of `edge_facets E`. If `τ e = none`, σ is constant on the fine facets round `e` (`e` lies inside a
  facet or a room). τ is onto.
- **(v) verts.** The same at vertex stars:
  - if `ν v = some V`, each fine facet round `v` goes to the sector of `V` it lies in. This is
    onto, but not one to one, since the lines of other planes through `v` cut a sector into
    several fine facets. Each fibre is connected through fine edges at `v` with τ = none, so fan
    classes map onto fan classes by `cls_map`. Also, τ is a bijection from the fine
    edges round `v` that have τ ≠ none onto `vert_edges V`;
  - if `ν v = none`, the star at `v` maps into a single facet, or a single edge's facets (`v` lies
    inside a facet, an edge or a room).

**Fixed in Lean (L2, f9778eab).** The authoritative list of conditions is the structure `Refines`
in verify/forge-exact/ForgeExact/DecideOnce/Refines.lean. Beyond the items above, the proof needed
these:
- nodup facets, edges, sectors and coarse vert-edges;
- every coarse piece comes from a fine one, and a coarse facet's two rooms differ;
- sector facets are listed, and sector bounds are among the vert's edges;
- fine sectors across an edge inside a coarse facet lie in one coarse sector;
- at a fine vert inside a coarse cell, one coarse facet's sectors are joined, any two coarse facets'
  sectors meet at an edge, and the fine edges along a coarse edge all map to it;
- a coarse vert has a single fine vert.

The builder's coarsens check (R3) checks these. `core_refines` and `flat_refines` are proven,
with standard axioms only.

**Simulation theorem** (`core_refines`): if `Refines F L ps A`, `Exact L ps A` and
`LedgerWF F`, then `coreI F rule` corresponds to `coreI (inc L) rule` under (ρ, σ, τ, ν):
- the same refusal kind, with a ZeroThickness cell mapping to the cell named;
- `empty` ↔ `empty`;
- `ok b` ↔ `ok b₀`, where `b`'s faces are the σ-images of `b₀`'s faces as sets of facets, its
  edges are the τ-images, and its verts are the ν-images.

The argument, stage by stage:
- the split check corresponds by (s);
- memberships correspond by (r) plus `Exact.boundary`: a fine flood class lies within one room
  class;
- keep and result correspond facet by facet;
- `around` corresponds by the bijections in (e) and (v), so bad edges and verts do too. A fine
  edge with τ = none has at most two result facets round it, both of one F-facet, so it is never
  bad and never an output edge;
- unify's `joinRel` classes correspond by (s)'s connected fibres plus (e).

**What the end-to-end theorem becomes** (`EndToEndF`). For a Rust v2 ledger `l`, assume
`Refines (toLF l) (sampleLedger ps ss (sampler ps)) ps (brepA ss)` and `LedgerWF l`. Then
`core_f l r` returns `o` with `∃ o₀`, the old core's outcome on `sampleLedger …`, such that `o`
corresponds to `o₀`. Every conclusion of today's `EndToEnd` holds of `o₀`: `Valid`, faces =
`XBoundary`, `reg` of the boolean, ZeroThickness ↔ `XNonManifold`. The realised facets of `o` are
the closures of the unions of the σ-fibres. The new theorem is the old one, composed with
`core_refines`. Its only new hypothesis is `Refines`.

**Collapse** (the metric stage) is not simulated through the fine core. Thinness belongs to rooms:
a fine sliver tope inside a fat room is not thin. Metric.lean's theorems are combinatorial over a
consistent ledger: decided once, local, one feature one answer, idempotent, only deletes, valid,
exact away from thin cells. They are restated over `IncLedger` and proven there, ported line for
line. `core_refines` is stated for the keep stage and for `core_k` with any room keep table, so it
composes with collapse. "Exact away from thin" holds for facets neither of whose rooms is thin.

### 2.2a What the unify simulation needs (the risky part)

The fine core's unify does three things:
- **faces** are the classes of `joinRel` over the result facets: same plane, same orient, sharing
  an edge;
- **verts** are the vertex cells with three or more faces round them;
- **edges** are the edge cells with exactly two faces, grouped by `erel`: the same pair of faces,
  joined through vertex cells that are not verts.

The simulation needs seven things.

1. **A generic classes-under-a-map lemma**, `cls_map`, in Closure.lean. Let `f : xs → ys` be
   onto, with relations `r` on `xs` and `r'` on `ys`. Suppose:
   - (i) `r a b → f a = f b ∨ r' (f a) (f b)`;
   - (ii) `r' c d → ∃ a ∈ f⁻¹c, b ∈ f⁻¹d, r a b`;
   - (iii) each fibre is `r`-connected.

   Then `f` maps `cls r xs a` onto `cls r' ys (f a)`, and two elements are in one class iff
   their images are. This is the heart, about 300 lines. It is used twice: for faces (σ, joinRel)
   and for edges (τ, erel).
2. **Fibre connectivity, now explicit in `Refines`.**
   - (s) The fine facets of one F-facet are `joinRel`-connected through fine edges with τ = none.
     They share the plane, and the orient too, since they have the same rooms and so the same
     keep.
   - (e′), new. The fine edges of one F-edge are connected through fine verts with ν = none. Such
     a vert has the two faces of that edge round it, so it is never an output vert and `erel`
     passes through it.
3. **Star bijections** ((e), (v)). These give (i) and (ii) of `cls_map` for `joinRel` and `erel`,
   and they give `facesAt x ↔ facesAt (τ x / ν x)`.
4. **No phantom edges or verts.**
   - A fine edge with τ = none has at most one face round it, so it is not in `ecs`.
   - A fine vert with ν = none has at most two faces round it, so it is not in `outV`.

   Both follow from (e) and (v)'s "constant" clauses.
5. **Order.** The output lists correspond as sets (up to permutation), not as lists. Today's
   conclusions about `b₀` (Valid, faces = XBoundary, reg) are all set-level, so nothing is lost.
6. **Naming (`origin`): a rule change.** Today a face is named by the first piece of its *head*
   facet, and the head depends on the facet order. That order is not preserved by coarsening. So
   v2 names a face by an order-free rule: the old path's own, unify's **least of its pieces'
   wholes**. That is the least `whole` (`joins.pieces`), else the least own name, over the input
   faces of all pieces of all its facets. It is equal under σ, and v2 states it directly as a
   naming theorem, not through `b₀`. It also makes the write's naming of merged faces a function of
   the face itself, which is the fuzz seed 29 issue. Bool-speed and the write must use the same
   rule.

   **Condition of sign-off:** names identical to today's on every existing part. A test over the
   suite and the corpus compares names with the switch on and off. If "least input-face id" and
   "least whole" ever differ on a part, the whole-based rule is the one used.
7. **Collapse** composes as in §2.2, with the room keep table lifted to topes.

Estimate: `cls_map` 300 lines, faces 250, edges and erel 350, verts and facesAt 150, naming 100.
That is about 1.1k lines, the largest single piece of the Lean side. Its one real risk is a gap in
`Refines`. If a case appears that the conditions do not cover (a fine edge whose star crosses two
F-facets of one class, say), `Refines` gains a condition and the builder has to meet it. So
`Refines` is fixed before the Rust builder is written: step L2 below goes first.

### 2.3 L1/L3: what the builder must establish, and the one new geometric lemma

`Refines` is the builder's obligation. It replaces today's open hypothesis
`toL l = sampleLedger …` (build_ledger, not yet proven for v1 either). One part of it is new
geometry:

- **`room_const`, from bool-research's wedge lemma.** For consecutive facets `φ`, `ψ` round an
  edge, points just off `φ` and just off `ψ`, near a point of the edge, are joined by a polyline
  whose every segment is `Clear` of every face. (`JordanLocal.Clear`: the segment meets each
  face's plane, if at all, outside the face.) So, by `contains_const_local` applied segment by
  segment, every facet-side of one room has one membership in every body. One witness per room
  then gives (r). The wedge lemma is the one hard new geometric proof, estimated at 300–500 lines.
- (s), (e) and (v) are facts about the builder's own segment-within-polygon arrangement, set
  against the sampler's plane cells. They belong to the builder's L3 (its Aeneas proof), in the same
  place today's `sampleLedger` equation would be proven.

Until the builder is proven, `Refines` is a stated hypothesis, as `toL l = sampleLedger …` is now.
It is also a runtime check: a debug build can check (e)/(v)'s counts and (s)'s sides against a
plane-arrangement build, on the gate's corpus. The bit-identical ledger check becomes a
"coarsens" check.

## 3. What carries over

Lean spec side (verify/forge-exact):

| file | fate |
|---|---|
| Correct.lean, Coverage, Closure, Sample, Sampler, Witness, Boxes*, Local*, Frontier, JordanLocal | **kept unchanged**: the reference chain |
| Core.lean | kept. `coreI` is new beside it, and `core_inc` ties them |
| Metric.lean | ported to `IncLedger` (~430 → ~450) |
| new: IncCore, CoreInc, Refines, Simulation, RoomConst, EndToEndF | ~3–3.5k lines |

Aeneas side (research/kernel-audit/decide-once/aeneas). Today it has 15k proof lines and 6.5k of
translation.

| today | v2 |
|---|---|
| RefineIndex, Sort, SortU32, Hash, HashSort, Cells, Ext, Split, Between, Touch, BridgeTouch, Zeros, EnumModel, SortModel (~4.9k) | **gone**: no covectors, no interning, no enumeration |
| RefineSplitOk (728) | ~250: a table check |
| RefineAround (670), RefineFan (557) | ~250 + ~350: Csr reads plus a filter |
| RefineMemb, Classify, Keep, Rule, UF, UFModel (~1.6k) | port (cells become rooms): ~1.4k |
| RefineUnify* (~2.4k) | port: ~1.9k |
| RefineCollapse (1.9k) | port per room: ~1.5k |
| Refine, RefineLedger, RefineCore, Encode, Specs, EndToEnd, Integrate (~1.4k) | rewrite against `toLF`: ~1.0k |

So the v2 Aeneas proof is about **6.5–7k lines**, against 15k today. The Lean spec side adds about
3–3.5k. Both sizes are the Lean side's estimate. The single risky piece is the simulation
theorem's unify part (classes through fibres), about 800–1000 of the 3.5k. Everything else is a
port or a table read.

The Rust core v2 should shrink from about 1.1k lines to about 0.7k. Per boolean, at about 150 cells, the
core should cost well under the 0.6M budget. Every lookup is O(1) or a Csr row; nothing hashes or
sorts covectors.

## 3a. Speed, against the old path (per flat boolean, estimates)

Two corrections from bool-research's measurement (callgrind, switch off):
- **The old path costs about 1.28M per flat boolean, not 3.0M.** Its write is in place, about
  0.06M.
- **On the gate, only coplanar (70 booleans), housing (1) and upto (4) reach the flat path.** The
  15 bench parts' booleans are all NotFlat.

| phase | decide-once now (coplanar) | A8 (coplanar) | how |
|---|---|---|---|
| arrangement | 3.46M | 0.4M | about 9 exact segments, per-face insertion, `meet3` behind a filter |
| sample (witnesses) | 0.41M | 0.15M | one parity per room |
| ledger | 1.60M | 0.35M | about 150 cells; id tables; no `Vec<Sgn>` interning |
| core | 1.63M | 0.35M | Csr reads: O(degree), not 16-wide scans |
| write | 0.68M | 0.15M | **in place**, as the old path's apply: edit only the near faces |
| rest | 0.42M | 0.25M | exact planes for near faces only |
| **total** | **8.2M** | **≈1.65M** | old path **1.28M** |

So A8 alone, at today's constant factors, leaves decide-once at **about 1.3× the old path** on
coplanar.

**Budget (signed off): ≤ 1.16M per flat boolean, on every gate line.** That is the old path's real
flat cost (a8-review.md §6.5, baa5b47: of the 200.6M, the 80 rods take 119.7M, leaving 80.9M for
the 70 flat booleans). It replaces the 1.28M above. bool-research's bottom-up
estimate, from measured per-op costs and the coplanar counts, is about 0.50M. Their budget per
boolean (a8-review.md §6, deb531d):

| phase | budget |
|---|---|
| inputs, region, snap, near-only | 0.12M |
| segments | 0.06M |
| per-face arrangement and seams | 0.10M |
| radial order and rooms | 0.05M |
| witnesses, fused into the builder | 0.06M |
| Csr ledger | 0.04M |
| core v2 | 0.25M |
| collapse | 0.04M |
| write in place | 0.20M |
| **total** | **0.92M** (21% under 1.16M) |

The gap between 1.65M and 0.92M is constant factors. So these six are **binding build rules** for
the v2 bridge and builder (the master, 2026-11-23):
1. R6: no exact plane or point for a far face.
2. No HashMap in the bridge. Keys are tuples of near-class ids, so use dense tables or sorted
   arrays.
3. One reused arena per boolean, cleared rather than freed.
4. Covector-free vertices: a carrier-triple key plus a filtered f64 point, exact only on filter
   failure.
5. Witnesses fused into the builder, with the orientation shortcut first.
6. R5: the in-place write.

**Fast form from the start** (the owner's rule, PROOF-STANDARD.md): v2's core is designed in its
fast form, with dense tables, ids, `row_eq`/`row_hash`-style flat keys where the builder needs
them, and no `Vec` per cell. So the fast form is the proven one.

**Allocation pass line (R4):** ≤ 1,000 allocated blocks per flat boolean (dhat). Decide-once is at
about 4,990 today, the old path at about 1,150. This is how rule 3 is checked.

**R4-early (checkpoint, binding).** Callgrind R3's builder phases on the three benches. Any phase more than 1.5× over its
budget is fixed before the Aeneas groups continue. R5 may land first, on today's v1 path, gated by
the bits check. "Nothing slower"
needs three more things, beside A8:
- the in-place write;
- near-only `inputs`;
- about 0.4M from constants, for example witnesses fused into the builder's clipping and no
  HashMaps in the bridge.

Measured today (decide-once Rust, plane-ledger-cells.md, e529a4f), per boolean on the proven v1
core:

| | coplanar | upto | housing |
|---|---|---|---|
| total | 6.0M | 5.5M | 2.2M |
| fill | 4.0M | 4.15M | 1.57M |
| core | 1.64M | 1.03M | |
| write | 1.0M | 0.14M | |

Of the plane cells `arrange` makes, only 10–15% become facets.

**Measured face-arrangement sizes** (decide-once Rust, FORGE_A8_STATS: each real v1 ledger
coarsened, joining fine facets across a fine edge that has exactly two of them round it, with one
class and one holder set). Per boolean, fine → coarse:

| | pieces → facets | edges | verts |
|---|---|---|---|
| coplanar | 46.7 → 17.1 | 72.4 → 40.8 | 30.1 → 26.9 |
| upto | 54.0 → 16.8 | 103.5 → 53.2 | 53.2 → 41.2 |
| housing | 19 → 12 | 28 → 20 | 12 → 12 |

The §1a estimates (30/20/15 facets) were high for coplanar. R3's builder must reproduce these counts,
up to seams: a cut along a carrier line adds one facet and the seam's edges.

Housing and upto have the same phases, with fewer facets (15 and 20). They are 1 and 4 booleans, so
they move their gate lines only slightly. Coplanar is the line that decides it.

The per-phase split for upto and housing is asked of decide-once (Rust). **Risk:** if the last
0.4M does not close, the switch stays off, and v1 stays proven and shipped-ready. A8 is still
worth doing for curved: the same cells, and the Tier-1 separators move into the reference.

## 3b. Provability of the speed steps (owner's rules, 2026-11-23)

- **The shipped code is proven, not checked.** Proven code carries no runtime proof. Checks run in
  debug builds, tests and CI only. Speed is measured in wall-clock time.
  - So the builder (R3) is proven to produce a well-formed ledger (`LedgerWF2`) that refines the
    plane ledger (`Refines`), and `inc::core` is proven against `coreMI` (`IncCoreSpec`).
  - Release builds run neither `ledger_wf` nor a Refines check.
- **The checker is an oracle, not a guard.** A Refines checker, plus `ledger_wf`, may live in
  forge-bool-core, translated and proven (`ledger_wf ↔ LedgerWF2`, `refines_check ↔ Refines`).
  - It is the debug assert's oracle, and a development aid for the builder.
  - Its cost is measured separately and never counted as part of the boolean.
  - It is never a permanent fallback path.
- **Any optimisation that is provable is allowed, and the proof grows to cover it.** That covers
  trim/split reuse, the whole-plane shortcut, the D7 memo, parallel planes, and anything faster.
- **For each step, the Lean coordinator:**
  - checks that it is provable;
  - writes its statement here;
  - plans the extra Lean, asking the master for more proof sessions when the volume grows.
- **For each step, decide-once and bool-speed:**
  1. keep its semantics a function of exact data (memo keys exact; parallel results merged in a
     fixed order, so the ledger is a function of the input);
  2. keep code inside forge-bool-core in Aeneas style (plain loops, no closures, iterators, maps,
     floats or threads). Code outside it is proven through what it hands the core;
  3. send the step's statement and commit for review before it is relied on.

## 3b′. Owner rulings for both booleans (2026-11-23, later): prove the original

"Prove the original bool, fix its bugs, and fall back to the proven one." "Prove the original
implementation first." "Even the slow fallback should still be as fast as we can get it."

1. **Today's code is the base.** It stays as it is wherever it is provable. Proofs target the
   original functions (via Aeneas on the real code), not a parallel rewrite. Where A8 or curved wrap
   a function, the proof is of the original plus its screen.
2. **Guess steps become proven screens.** LINEAR snapping, the touch, the strays and pinched
   heuristics, and the retry and re-snap ladders are each replaced by an f64 screen, proven via the
   IEEE model, that answers *decided* or *undecided*.
3. **Undecided goes exact for that question only.** Never to the whole exact reference, and never
   to a whole-boolean switch of path.
4. **If exact can't decide either, the boolean refuses** with a typed error. A wrong answer is never
   allowed.
5. **The exact references are fuzz oracles only.**
6. **The exact per-question fallback is held to the speed rule too.** Provable speed steps on it are
   welcome, with the proof growing alongside.
7. **Order:** first prove today's code as it stands (pair loop, `Index`/`face_box`, `trim`,
   `split_face`, `classify`, the meet chains), with only the guess steps swapped for screens. Only
   then the exact fallback's speed work and further optimisations.

**Owner decisions (2026-11-23, via the master).**
- **Target: today's flat boolean, the old path** (boolean.rs, classify.rs, imprint.rs, nary.rs),
  with its guess steps swapped for proven per-question screens. This is the same plan as curved.
- **A8 is not shipped as a second boolean.** Its proven core (`IncCoreSpec`, `EndToEndF`) stays as a
  spec and as a debug and fuzz oracle. The R3 builder proof for face_fast is dropped.
- **No refactoring for the prover.** The code is proven as written. Charon running out of memory is
  a tooling problem to fix, not a reason to restructure. A construct that truly can't translate once
  the tooling is fixed goes to the master, named exactly.

**Owner clarification (2026-11-23, later): A8 ships as the flat fallback.** "New perfect bool code
can ship, but only as a fallback to faster code."
- Flat booleans run today's path first. If it is undecided or would refuse, A8 (face_fast plus the
  inc core) runs. If A8 can't decide either, the result is a typed refusal.
- So the earlier line "A8 is not shipped as a second boolean" is superseded: A8 ships, as the
  fallback only.
- `boolean_correct`'s Ok case covers both branches. The A8 branch is proven by `IncCoreSpec` /
  `EndToEndF` plus the R3 builder proof, which is back in scope after today's path's proofs.
- An `Undecided` refusal now means both paths were undecided.

**The refined F1 (the master, 2026-11-23).** The result, and each input, is faithful within a
proven δ to an exact counterpart:
- the exact body is its exact carriers plus its topology;
- each f64 vertex maps to a connected exact cluster of diameter ≤ δ, since 4+ planes that are only
  nearly concurrent really bound a micro-edge or micro-face there;
- edges and faces are one-to-one outside the clusters;
- ⟦b⟧ is the solid the exact planes bound under that topology;
- a refusal happens only per decision, when a decision needs a cluster point that neither the screen
  nor exact can separate. Nothing that builds today may start refusing.

**Order of Lean work.**
1. IEEE: proven screens for today's guess steps (LINEAR snapping, the touch, strays, pinched, and
   the retry and re-snap ladders as per-question screens).
2. aeneas: fix Charon on boolean.rs and nary.rs as they stand. Then translate, in order: classify
   (`Index`, `face_box`, `classify`), ssi and unify, imprint, boolean (pair loop `plan_once_in`,
   `trim`, `split_face`, `classify_specs`, `touches`, the meet chain), and nary.
3. Lean coordinator: refinement proofs of each translated function as it lands, against the Lean
   models.
4. dol-curved: the F-series, as the spec for today's curved code.
5. Then R3 for face_fast (A8's branch), and the thin ≠ [] end-to-end theorem, which A8's branch needs
   wherever thin rooms occur.
6. After all of that: speed work on the exact per-question fallback.

**Review of the plans against this ruling (Lean coordinator).** These assume a whole-boolean
fallback, and must change to per-question exact plus a typed refusal:

| where | what it assumes now | under the ruling |
|---|---|---|
| decide_once.rs `Skip` (`NotFlat`, `Ambiguous`, `Wide`) | the whole boolean leaves decide-once for the old path | each undecided question goes to its exact test; refuse if exact can't decide. `NotFlat` is scope, not a question: it routes by input kind, before any decision |
| §3c keys/held (decide-once, 41afccb) | `vertex_at` failing near the region skips the whole boolean (`Skip::Ambiguous`) | the exact vertex for that meet only |
| curved-brep-route.md §0.6 and §4 (`Uncertified`, `Budget`, `Tangent`, `NoSeed`) | counted skips, then the old path runs | per-question exact (exact vertex, exact second-order order, exact ray), then a typed refusal. `OutOfScope` routes by kind; `Inconsistent` is already a refusal (invalid input) |
| today's boolean.rs retry and re-snap ladders | rerun the boolean with other tolerances | removed: a screen per question, exact for that question |

Not affected: the A8 core proofs (`IncCoreSpec`, `EndToEndF`). The core decides nothing
numerically. The checker/oracle rule of §3b stands.

## 3c. The builder's speed steps as built (face_fast.rs), each with its correctness statement

The builder is `kernel/forge-bool/src/decide_once/face_fast.rs`, outside forge-bool-core. It is
proven through what it hands the core. In debug builds, its facets are asserted equal to
face_builder's and to the plane ledger's coarsening on all 75 flat booleans (coplanar, upto,
housing). Each step below is a statement for R3 to cover. f64 screens are proven in
`verify/ieee754/IEEE754/Screen/Face.lean` (62120f5) unless marked "contract pending".

| step | statement |
|---|---|
| keys | A vertex is `In(body, v)` or `M(a < b < c)`, the meet of three class planes. Equal keys are one point. A point is on every class in its `on` mask: an `M` point on its three classes; an input vertex on its round faces' classes when `Pts.held` (checked by `vertex_at` at input, so every plane round it holds its exact point). |
| merge | Vertex = exact point. Points no axis keeps apart (`axis_sound`) are compared exactly. Two points of the current plane C that both lie on classes a and b, with C, a, b independent (the meets table holds their meet), are one point. |
| line identity | Two points of line L = C∩K, both on a class h that crosses L (`crosses_line_sound`, else the exact d·n_h ≠ 0), are one point, so their order along L is 0. |
| crossing | Edge (a, b) of a face on plane P meets plane K strictly between its ends iff side_K(a)·side_K(b) < 0, and the point is meet(C, K, across). An edge with an end on K adds that end. No other edge meets K. |
| dependence | Three normals, each along one axis (one nonzero f64 component, so the exact Dy has one nonzero), are dependent iff two share an axis. Otherwise the exact det n_a·(n_b×n_c) decides. |
| touch | A face's box is the hull of its vertices' shadows, each widened by its proven bound, so it holds the exact polygon; a class box is the hull of its faces'. A face whose box misses plane K has no vertex on K and no edge across K, so it adds no point to line C∩K and contains no point of it. No kernel-box premise (41afccb). |
| bodies | Hypothesis `ValidSolid` (stated in §3c on decide-once's branch, coplanar adjacent faces included), for the snapped exact polygons: faces of one solid meet only along their common edges. So a stretch of C∩K inside a C face and a K face of the same body lies on that C face's boundary. Only different-body pairs need cutting. Source: the producing features' postcondition. forge_topo::check covers part of it in debug. |
| whole plane | A plane whose faces are all of one body, each strictly inside the box (boxes grown by 4·LINEAR plus 1e-9 relative), with no near face of another body whose box meets both the plane and its faces' box, has its faces as its facets: one facet per face, holders {that face}. |
| covered | A window of C∩K lying on one of C's boundary segments on the same line repeats that segment, so it is not added. |
| inside cache | Face F's inside answer is constant on a run of windows that touch none of F's points (endpoints, crossings, ends on the line), because F's boundary meets the line only there. A face with an unlisted crossing (`always`: across far, across K, or a dependent meet) is retested every window. |
| parity ray | For a point not on F's boundary, parity along any axis ray decides inside. The ray runs across the window's line. Results on F's boundary may differ from face_builder's, which changes segments only, never facets (asserted). |
| edge index | An edge whose span on axis a (its points' shadows ± their bounds) misses [x − tol, x + tol] has both ends strictly on one side of the plane x_a = x (plane exact within tol), so it is skipped by `crossing`. In mid_in it is crosses_f's `false`. |
| area | A face cycle of the planar graph is a bounded face's outer walk iff its signed area is > 0. Screened (contract pending), with the exact least-vertex turn as fallback. |
| angle | Half-edges sorted by angle, on to_f64 of the exact line direction with an 8ε bound (fix (a), efd1d6f). The half-plane comes from the exact signs. |

**D7, the ledger-outcome memo: tried and rejected (2026-11-23).**
- What was tried: `core::core` memoized on the canonical ledger (only the faces the pieces name, renumbered monotonically, origin mapped back), with a full-equality check.
- Coplanar seq: 70 booleans give 52 distinct ledgers, so at most 18 hits, and those are small ones.
- Result: the core goes from 79.0M to 72.6M, saving 6.4M, while the memo costs 9.7M. Net: 3.3M worse. Not kept.
- Many same-size ledgers differ, so "repeats" holds only up to relabelling, which needs a canonical form. Not pursued.

## 4. Order of work and gates

Step 0 is the sign-off of this design: the §1 interface, `Refines` (§2.2 and §2.2a), the
statement of `EndToEndF`, and the naming rule.

Then the Lean and Rust tracks run in parallel. Each arrow is a dependency.

| track | step | needs | size |
|---|---|---|---|
| Lean | **L1** `IncLedger`, `coreI`, and the `EndToEndF` statement in Specs.lean (so the Aeneas groups have a target) | 0 | S |
| Lean | **L2** `Refines` fixed, with `cls_map` and the unify simulation proven first (the risky part) | L1 | L |
| Lean | **L3** `core_inc` (old core = `coreI ∘ inc`) and the rest of `core_refines` | L1 | M |
| Lean | **L4** `room_const` from `contains_const_local` | — | S |
| Lean | **L5** the Metric.lean port to rooms | L1 | M |
| Rust | **R1** v2 types plus a `LedgerWF` v2 debug check, as `forge_bool_core::v2`, v1 untouched | 0 | S |
| Rust | **R2** the v2 core in Aeneas style, then translate.sh: `Funs` v2 | R1 | M |
| Rust | **R3** the face-arrangement builder plus a debug "coarsens" check against v1's ledger (`Refines`' counts and sides) on the gate corpus | R1, L2 (`Refines` fixed) | L |
| Lean | **R3-L3** the builder's proof: the wedge lemma (consecutive facets round an edge joined by a `ClearPath`, so `room_const` gives `Refines`' rooms condition), then `Refines` for the builder. **Owner: the decide-once Lean coordinator** (this session), or a helper it spins up. The Rust session builds the radial order but does not own its proof. | R3's radial order, L4 | M–L |
| Rust | **R4** callgrind on coplanar, upto and housing against the old path | R2, R3, R5, R6 | S |
| Rust | **R5** an in-place write: edit body 0's near faces only, as the old path's apply does (0.06M against 0.68M) | — | M |
| Rust | **R6** exact planes and points for near faces only (`inputs` converts all ~183 faces today) | — | S |
| Aeneas | **G1–G5** groups as before: tables/split_ok, classify/keep/rule/UF, around/fan, unify, collapse | R2, L1 | M each |
| Aeneas | **E** `end_to_end_f_proof` = the groups + `core_refines` + `EndToEnd` | G*, L2, L3 | S |
| CI | forge-bool-core-proof.yml gains the v2 translation and proof. Both end-to-end theorems must stay green. | R2 | S |

Critical path: L2 → R3 → R4 for speed, and R2 → G* → E for the proof.

The switch flips to v2 only when all of the following hold:
- E is green;
- v2 ≤ the old path on **every** gate line (target ≤ 1.16M per flat boolean), and ≤ 1,000 allocated
  blocks per flat boolean;
- fuzz seed 29 passes;
- the naming rule agrees with today's names on the gate, or the owner approves the change of bits;
- R4 beats the old path on all three benches;
- the coarsens check passes on the corpus. It costs about a full plane fill (~8M per boolean), so it
  runs in CI on the gate corpus, not in every debug test.

## 5. Open questions for review

**The two drafts, reconciled.** bool-research's draft proved the face ledger exact directly
(`ExactF`), replacing Sample, Sampler, Witness and Boxes. This design instead keeps the plane
ledger as the never-run reference and proves `Refines`. The reason: today's end-to-end conclusion
includes `reg (decomp …).kept = reg (combine …)`, a statement about point sets, proven over the
plane arrangement's tope decomposition. `ExactF` would have to re-prove a decomposition of space
into rooms, whereas `Refines` inherits it.

Both routes need the wedge lemma. `Refines` adds the simulation theorem (§2.2a, about 1.1k lines
for unify). `ExactF` would add generalising Correct and Coverage over face geometry plus `cover`
and `complete`, which I estimate as larger. bool-research's one-core-over-an-interface
recommendation is §2.1's `coreI`.

1. **Walls: settled.** Walls are clippers, not carriers or facets (bool-research agrees). The
   statements `wall_region` and `samplesIn_of` stay on the reference. The builder's clipped
   polygons enter only through `Refines`. `enclosed` comes from wall-edge flags the builder
   fills, not from geometry in the core.
2. **Witnesses.** bool-research proposes one per room. The core keeps today's flood plus
   witness, which is correct for either: one witness per room is just a finer table. The builder
   chooses.
3. **Facets with holes.** These are fine for the core. Does the write (F3's stitch) accept loops
   with holes, or should the builder cut such facets to be simply connected? Cutting is allowed
   by (s): fibres stay connected.
4. **A topological reading over the face ledger itself** (dol-curved (b), for q1). Its rooms are
   connected by construction and its incidence is explicit. `xNonManifold_iff_top` could be
   proven over it directly, rather than through `Faithful` on the reference. `Refines` does not
   give that for free, because it transfers facts from the reference, not to it. It would be a
   direct L1 for the face ledger. For curved that may be cheaper than Lemma D on the CAD.
   Flat does not need it: the flat reference is already proven. **Ruled (the master): route B for
   curved.** The topological theorem goes over the face ledger, with F1–F5 as builder
   obligations, so the curved reference needs only `ExactK` (curved-design.md §7, c4dbd57). Flat
   keeps `Refines`.
5. **Curved (dol-curved).** The same floor applies to the carrier arrangement. The refinement
   scheme of §2.2 is geometry-free over the cells, so it should transfer: the curved spec's
   carrier cells are the reference, and its face arrangement a coarsening.
