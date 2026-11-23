# A8: decide-once over the face arrangement (design, for sign-off)

Status: draft. It was written by the Lean side for review by the research session (bool-research),
then sign-off by the master. No proof work starts until it is signed off. The current proven core
(`forge_bool_core`, `end_to_end_proof`) stays shipped-ready behind the switch, and nothing is
deleted.

Why: research/booleans/small-booleans.md §5. The spec's cells are the sign classes of the whole
*plane* arrangement, about 300 per boolean on coplanar seq. Arrangement, ledger and core scale with
that count (≈4.6M of ≈10M per boolean). A *face* arrangement splits a face only where another face
actually crosses it, about 40 cells. Only that change gets under the old path's 3.0M.

## 0. The proposal in one paragraph

The cells become **rooms** (components of the region's interior minus the near faces and walls),
**facets** (pieces of faces, split only along crossings), **edges** and **verts**. The ledger
gives their incidence explicitly as id tables, not covectors.

The proof does **not** start over. The plane-arrangement ledger that the sampler builds stays the
proof's reference. It is a Lean definition that is never run. The face-arrangement ledger is proven
to be a **coarsening** of it: every plane cell maps to the face-arrangement cell it lies in. A
**simulation theorem** then says the new core's outcome is the old core's outcome, coarsened.
So `end_to_end_proof`'s conclusions (valid body, exact boundary facets, `reg` of the boolean,
ZeroThickness ↔ non-manifold) carry over unchanged to the new core through one map.

The new Rust core has no covectors at all. It drops cov, find, intern, index, sort, hash, split and
between, and its Aeneas proof is about half the size of today's.

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
    pub vert_facets: Csr,                 // per vert: the facets whose closure holds it
}
pub struct MetricLedger { pub thin: Vec<RoomId>, pub enclosed: Vec<RoomId> }   // per room (as now, per side cell)
// Body: faces list FacetIds, edges list EdgeIds, verts list VertIds; Outcome, Refusal and Rule are unchanged.
```

What each table means:

- **Rooms** lie inside the region `R°`. The walls bound them but are not facets: no body owns a
  wall. A room touching a wall is not `enclosed`.
- **Facets** are only pieces of faces. A plane piece that lies in no face does not exist here,
  because it lies inside a room.
- **The two sides of a facet are different rooms.** This always holds, since crossing a piece of
  body `i`'s face changes membership in `i`. `LedgerWF` checks it.
- **Facets need not be convex or simply connected.** An annular piece round a boss is fine. The
  core is combinatorial. The write gets each facet's boundary loops from the builder, outside the
  core.
- **Edges and verts** are the 1- and 0-cells round the facets, as now.

The core reads the tables as follows. This is today's algorithm with lookups in place of
covector arithmetic.

| stage | today (covectors) | A8 (ids) |
|---|---|---|
| split check | `isFacet`, `zeroIdx = cls`, one face per body | `facet_class[φ] = cls f`, one face per body per facet, sides distinct |
| sides | `φ.plus` / `φ.minus`, interned (`split`) | `facet_rooms[φ]` |
| classify | flood over topes across non-pieces of `i`, then a witness | the same over rooms |
| keep, collapse | per tope | per room (metric tables per room) |
| result | facets with one side kept | the same |
| around(x) | result facets with `x ≤ φ` (enumerated by covector) | `edge_facets[x]` / `vert_facets[x]` filtered by result |
| bad edge / bad vert / fan | `Cov.le` between cells | `vert_edges`, `edge_facets` |
| unify | `zeroIdx`, `orient`, a shared edge | `facet_class`, `orient` (by room keep), a shared edge from `edge_facets` |

`LedgerWF` v2 has these parts:
- Each Csr is well formed: `start` is monotone, starts at 0 and ends at `ids.len()`; ids are in
  range; rows have no repeats.
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
- ρ : `L.topes → rooms F`,
- σ : `L.facets → facets F`,
- τ : `L.edges → Option (edges F)`,
- ν : `L.verts → Option (verts F)`,

with these properties:

- **(r) rooms.** ρ is onto. Topes with one room have one membership in every body
  (`memT ps A T i`). `F.inside` is a correct witness table for that membership, through the flood
  (the same `witness` field as `Exact`, per room).
- **(s) facets.**
  - σ is onto.
  - It keeps the class and the sides: `F.facet_rooms (σ φ) = (ρ φ.plus, ρ φ.minus)`.
  - It keeps the pieces: `(φ, f) ∈ L.pieces ↔ (σ φ, f) ∈ F.pieces`.
  - Each fibre σ⁻¹(F-facet) is connected through fine edges with τ = none. A face-arrangement
    facet is one connected piece.
- **(e) edges.** If `τ e = some E`, σ is a bijection from the fine facets round `e` onto
  `edge_facets E`. If `τ e = none`, σ is constant on the fine facets round `e` (`e` lies inside a
  facet or a room). τ is onto.
- **(v) verts.** The same at vertex stars:
  - if `ν v = some V`, σ is a bijection onto `vert_facets V`, and τ is a bijection from the fine
    edges round `v` that have τ ≠ none onto `vert_edges V`;
  - if `ν v = none`, the star at `v` maps into a single facet, or a single edge's facets (`v` lies
    inside a facet, an edge or a room).

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

### 2.3 L1/L3: what the builder must establish, and the one new geometric lemma

`Refines` is the builder's obligation. It replaces today's open hypothesis
`toL l = sampleLedger …` (build_ledger, not yet proven for v1 either). One part of it is new
geometry:

- **`room_const`.** Two topes joined by a polyline whose every segment is `Clear` of every face
  (`JordanLocal.Clear`: it meets each face's plane, if at all, outside the face) have the same
  membership in every body. This is `contains_const_local`, applied segment by segment, at about
  150 lines. It is what lets the builder decide a room's membership with one ray, and it gives (r).
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

The Rust core v2 should shrink from about 1.1k lines to about 0.7k. Per boolean, at ~40 cells, the
core should cost well under the 0.6M budget. Every lookup is O(1) or a Csr row; nothing hashes or
sorts covectors.

## 4. Order of work and gates

1. **Sign-off of this design.** That covers the interface in §1, `Refines` in §2.2, and the
   statement of `EndToEndF`.
2. **In parallel:**
   - Lean: IncCore + `core_inc`, then `Refines` + the simulation theorem;
   - Rust: core v2 in Aeneas style, behind the switch as `forge_bool_core::v2` (v1 untouched),
     with the same rules as before: while loops, no iterators or closures, no count_ones;
   - Rust: the face-arrangement builder plus the debug "coarsens" check against v1's ledger on
     the gate corpus.
3. **Aeneas proof of v2.** It splits into groups as before: tables/split, classify/keep,
   around/fan, unify, collapse. Then `end_to_end_f_proof`.
4. **CI.** forge-bool-core-proof.yml gains the v2 translation and proof. Both `end_to_end_proof`
   and `end_to_end_f_proof` must stay green. The switch flips to v2 only when v2 is proven, is
   faster on the callgrind gate, and the coarsens check passes on the corpus.

## 5. Open questions for review

1. **Walls.** I treat region walls as room boundaries that are not facets. The alternative is
   facets of a pseudo-body, which would keep `enclosed` a table lookup. Which does the builder
   prefer?
2. **Witnesses.** Should witnesses be one per room per body, which needs no flood, or flood plus
   witness as now? I propose keeping the flood (fewer rays, and the proofs port). At ~40 rooms,
   per-room witnesses may be cheaper than the flood's union-find. Ask bool-speed.
3. **Facets with holes.** These are fine for the core. Does the write (F3's stitch) accept loops
   with holes, or should the builder cut such facets to be simply connected? Cutting is allowed
   by (s): fibres stay connected.
4. **Curved (dol-curved).** The same floor applies to the carrier arrangement. The refinement
   scheme of §2.2 is geometry-free over the cells, so it should transfer: the curved spec's
   carrier cells are the reference, and its face arrangement a coarsening.
