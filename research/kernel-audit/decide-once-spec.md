# The decide-once flat-face boolean: specification (2026-11-22)

The algorithm the Rust rewrite of the flat-face boolean is built to match, its interface, and what
is proven about it. The Lean is `verify/forge-exact/ForgeExact/DecideOnce/` (core Lean, no Mathlib,
no `sorry`, standard axioms only, checked by `ForgeExact/Axioms.lean`). Build: `lake build` in
`verify/forge-exact` (about 25 s on top of the existing package).

Read first: README.md §2.1 and §6.1 (the tolerance rule), decisions.md (the fact kinds and the six
retry rescues), near-coincident.md (decide in the representative), coverage.md (the box family),
exact-comparison.md (the open bug classes).

## 1. The idea in one paragraph

After §6.1's snap, the carriers are a finite set of classes, one plane each (the representative's).
A **cell** of their arrangement is named by its **covector**: its side of every class, `neg`,
`zero` or `pos`. Two cells are one cell iff their covectors are equal. So every incidence the old
kernel measured at several sites ("is this point on that edge", "are these two cuts one vertex",
"does this curve run along that edge") is a comparison of two covectors, and a covector is a row
of one table of side decisions, each made once. The combinatorial core takes that table and the
inputs' names and nothing else: no point, no plane, no number. Every grouping it makes (flood
classes, fans, faces, edges) is the transitive closure of a decided relation, never a clustering
by distance. It has no retry: the two refusals that started the ladder are unreachable for every
consistent ledger.

## 1a. The fact kinds, from the algorithm's structure, and their completeness

Any B-rep boolean of flat-faced solids must answer, for its inputs and nothing else:

1. **To split:** where each face is cut. A face is cut by every carrier crossing it, so the
   question is *which side of carrier class `j` a cell is on* (K2, `side`), and before that *which
   carriers are one surface* (K1, `surface`). The pieces of a face are then the cells of its
   carrier inside it: *is this facet inside this face* (K3, `pieces`).
2. **To classify:** *is this cell inside this body*. Off the boundary it cannot change, so it is
   asked once per flood class (K4, `inside`); everything else is propagated by K3 (being in body
   `i` changes only across a piece of `i`).
3. **To assemble and check the result:** *which cells are edges and vertices* round the pieces
   (K5, `edges`/`verts`), so that the facets round each can be counted and their fans formed.

These five are the core's whole input: `core : Ledger → Rule → Outcome` and `Ledger` holds K1–K5
and the inputs' names, nothing else, so no other fact can be asked. **They are complete**: theorem
(c) holds for *every* ledger exact for *any* snapped flat geometry (any number of carriers, faces
and bodies, any configuration), so no configuration needs a further kind.

Against decisions.md's list (a cross-check of what today's kernel asks, not the source):
- *two faces one surface* is K1; *vertex on surface* is K2;
- *point on edge*, *point at vertex*, *two cuts one vertex* and *curve along edge* are not
  primitive: each is a comparison of K2 rows (`Decisions.ask`), so they cannot disagree with each
  other or with K2 (the closure laws);
- *touch or crossing* is dropped as a kind: the core never asks it. For planes a line meets a
  carrier transversally or lies in it (a K2 answer), and a *body* touching another is decided by
  the touch rule on K3–K5, not by a contact predicate. It stays in `Fact` only as a derived lookup
  for the bridge's own use;
- *point in face* (K3), *inside the body* (K4) and *cell dimension* (K5) are added: decisions.md
  logs them as sites, not kinds, but the algorithm cannot run without them.

## 1b. Generality of the theorems

- No logged input, fuzz seed or test part appears in any definition or in the proof of (a)–(c).
  They quantify over all ledgers (b), and over all carriers `ps : List Plane`, all solids
  `A : Nat → Pred V3` and all exact ledgers for them (c).
- Global correctness is never by listing configurations: (c) is `reg_kept` over the decomposition
  of ℚ³ into the arrangement's full cells (`topeDecomp`, for any list of planes, built by induction
  on the list as in `Flat`), and the flood is proven right by induction along the chain of
  crossings joining a cell to its witness (`memb_eq`, `Conn.invariant`). Classes, fans and faces
  are transitive closures, characterised by `cls_spec` for every list.
- Local finite checks appear only where a step is about one cell: the face order `Cov.le`, a
  facet's two sides, counting the result facets round one edge (`three_iff`), the fans round one
  vertex. Each holds for every geometry giving those answers.
- The six acceptance tests (§5 (d)) are instances, checked after the general theorems and only as
  corollaries of them (`rung_covered`, `rung_exact`); the box ledgers' `decide` checks prove that
  those instances are consistent, nothing more.

## 2. The ledger

### 2.1 The two primitive tables

Built once per boolean, before the core runs (`Ledger.lean`, `Decisions.build`):

| table | key | value | decided by |
|---|---|---|---|
| `surface` | input face (arena index) | carrier class | `Snap.rep`: the transitive closure of `tolEq` (planes within LINEAR over both faces' extent, the region not empty), the least-named carrier of the class; `Ambiguous` past the spread cap |
| `side` | (cell, class) | `neg` / `zero` / `pos` | `sideIn`: the sign of the class representative's plane, in the asking face's own sense (`Snap.snapped`), at the cell's exact point (its vertices recomputed exactly from the snapped carriers, `Snap.vertexAt`) |

Each key is decided exactly once (`Table.decideAll_once`) and every lookup returns that decision
(`Table.get?_decideAll`). A cell's covector is its row of the side table (`Decisions.row`).

Three more tables the core reads, each a table of decisions made once:

| table | key | value | the numeric predicate behind it |
|---|---|---|---|
| `pieces` | (facet, input face) | the facet lies inside the face | point in face, in the representative's plane |
| `inside` | (side cell, body) | the cell is inside the body | one ray per flood class (§3, stage 3) |
| `edges`, `verts` | cell | it is a cell of dimension 1 / 0 round the pieces | rank of the zero classes' normals, exact |

### 2.2 The fact kinds are lookups

decisions.md's kinds, as `Fact` questions to the ledger (`Decisions.ask`):

| kind | question | answer | old sites it replaces (decisions.md) |
|---|---|---|---|
| point on edge | cut point `p` on edge cell `e` | `row p ≤ row e` (face order) | `on_edge` ← boolean.rs:2441, 2751, 3244, 4380; `point_in_face` nearest edge |
| point at vertex | cut point `p` is vertex `v` | `row p = row v` | `place_cuts` end vertex |
| vertex on surface | side of `p` of face `f`'s surface | `side (p, surface f)` | the coincident-pair implication, trim's edge-box and run-on end tests |
| curve along edge | the curve of faces `f`, `g` along edge cell `e` | `e` on both classes | trim's run-on (`cross().length() ≤ 1e-6`) |
| two cuts one vertex | cut points `p`, `q` one vertex | `row p = row q` | trim's dedup, `place_cuts` clusters, the n-ary clusters, the 1.2× shallow pass |
| two faces one surface | faces `f`, `g` one carrier | `surface f = surface g` | `planes_coincident`, `ssi::nearly_flush`, unify's `SAME` (0.5×) |
| touch or crossing | against `f`'s surface, the cells either side of a point | `contactOf` of their two sides | `crossings_near`, `curve_surface` hair roots |

`ask_build`: a question to a built ledger is answered from the deciders' one answer per key, by
its kind's formula, and nothing else. The closure laws, each of which an old site pair broke:
`oneVertex_symm`/`_trans`, `oneSurface_symm`/`_trans`, `side_of_oneSurface` (faces of one surface
give every point one side), `onEdge_of_atVertex`, `on_of_onEdge` (README §2.1's
`coinc(F, G) ∧ e ⊂ F ⇒ side(ends(e), G) = 0`), `along_iff`. And `sideIn_class`: decided in the
representative, the on/off answer is one across a class of tolerance-equal carriers, whichever face
of the class asks (from `Snap.onIn_class`). That is near-coincident.md's rule as a theorem: once two
surfaces are merged, no decision is made across the gap.

## 3. The combinatorial core

`core : Ledger → Rule → Outcome` (`Core.lean`). `Rule` is the op as a pointwise rule on the
membership vector: union, intersection, difference, or a whole history over any number of bodies
(so n-ary is one call, not a second evaluator). Definitions, in the order the Rust runs them:

**Covectors.** `Cov` is a list of `Sgn`, one per class. `zeroIdx X` is its first zero. A *facet*
has exactly one zero. Its sides: `plus X` and `minus X` set that zero to `pos` / `neg`.
`le X Y` (X is a face of Y): equal length, and `X[j] = zero ∨ X[j] = Y[j]` for every `j`.

1. **Split.** `facets` = the facets of `pieces`, first occurrences kept in order (`dedup`). The
   pieces are listed in input-face arena order (body 0's faces first), each face's facets in a
   fixed order, so `facets` is in arena order. `splitOk`: every piece is a facet whose zero is its
   face's class, its face's body is `< nb`, and no facet is inside two faces of one body. Else
   `SplitCrossed`.
2. **Imprint.** Nothing to compute: a facet inside faces of two bodies (a coplanar overlap) is one
   facet, by its key. `isPieceOf i φ`: some piece `(φ, f)` with `f` in body `i`.
3. **Classify: flood plus witness.** `topes` = the sides of the facets (dedup, in order). For body
   `i`, `crossings i` = the (plus, minus) pairs of the facets that are *not* pieces of body `i`;
   being in `i` cannot change across them. `memb i T`: the first tope (in `topes` order) of `T`'s
   class under the closure of `crossings i` is the class's witness; `memb i T` is the `inside`
   answer for (witness, `i`). One ray per flood class. `classifyOk`: every piece of body `i` has
   one side in `i` and one out. Else `InconsistentClassification`.
4. **Keep.** `keep T = rule (i ↦ i < nb ∧ memb i T)`, tabled once per tope (`keepTab`); every later
   stage looks it up (`keepIn`).
5. **Assemble.** `result` = the facets whose two sides' keeps differ, in `facets` order.
   `orient φ` = `pos` if the minus side is kept (outward points to the dropped side), else `neg`.
6. **Touch rule.** `around x` = the result facets `φ` with `le x φ`.
   - For each edge cell `e` in `edges` order: more than two result facets round it is
     `ZeroThickness e`.
   - Then for each vertex cell `v` in `verts` order: the fans round `v` are the classes of
     `around v` under "share an edge cell `e` with `le v e`". More than one fan is
     `ZeroThickness v`.

   Non-manifold results stay refused. There is no length or angle threshold here: a feature 1.5
   LINEAR wide is built.
7. **Empty.** No result facet: `Empty`.
8. **Unify: read by reference, write fresh.** Takes the assembled facets, writes a new `Body`:
   - faces: the classes of `result` under `joinRel` (same zero class, same orientation, sharing
     an edge cell; past the touch rule an edge cell has at most two result facets). Listed in
     the order of their first facet, which is input arena order. Each face's `origin` is the
     arena index of the first input face its first facet lies in: its name.
   - vertices: the vertex cells where three or more faces meet.
   - edges: the edge cells round which two result facets of *different* faces lie, joined into
     classes when they have the same face pair and share a vertex cell that is not a vertex.
     A straight run between two faces is one edge, with no redundant vertex (the M1 class).
     Listed by first cell (arena order of `edges`).

`Outcome` = `ok Body | empty | refused Refusal`, `Refusal` = `splitCrossed |
inconsistentClassification | zeroThickness (cell)`. There is no retry constructor and no
alternative reading anywhere.

**Complexity.** With `F` pieces, `T` topes, `n` classes, `E` edge cells: splitting is the face-pair
work the kernel does today; the flood is a union–find per body over `T`; the touch rule and unify
are local to each edge and vertex. The Lean's lists stand for arena vectors and its `cls` (a closure
by rounds) for a union–find: the Rust should use a union–find and index maps, and is proven equal
by the classes being the same sets (`cls_spec`), not by the same steps.

## 4. The interface (for the Rust and for Aeneas)

A crate of its own (say `forge-decide`), depending on nothing numeric:

```rust
pub enum Sgn { Neg, Zero, Pos }
pub struct CellId(u32);            // index into Ledger::cells
pub struct FaceId(u32);            // input face, arena order
pub struct BodyId(u32);

pub struct Ledger {                // built once, read-only to the core
    pub nb: u32,
    pub face_body: Vec<BodyId>,    // per input face
    pub face_class: Vec<u32>,      // two faces one surface
    pub cells: Vec<Vec<Sgn>>,      // a cell's covector: its row of the side table
    pub pieces: Vec<(CellId, FaceId)>,   // in face arena order
    pub inside: Vec<(CellId, BodyId)>,   // the witness answers that are "inside"
    pub edges: Vec<CellId>,
    pub verts: Vec<CellId>,
}
pub enum Rule { Union, Inter, Diff, Table(Vec<bool>) /* 2^nb rows for a history */ }

pub fn core(l: &Ledger, rule: &Rule) -> Outcome;      // the whole of §3
fn unify(l: &Ledger, asm: &Assembled) -> Body;        // reads by reference, returns a fresh Body
```

Constraints, so Aeneas can translate it:
- **No floats** in any signature or field of the crate. Every geometric fact arrives as a `Sgn`,
  a `bool` or an id. The numeric predicates (snap, side, point in face, ray, dimension) live in
  the bridge that builds the `Ledger`, and are proven against §6.1 separately (README §8.2).
- **No thread-locals, no `Cell`/`RefCell`, no global state.** The retry flags (boolean.rs:853–870,
  intersect.rs:580) have no counterpart: there is nothing to switch.
- **No closures that capture `&mut`.** Loops are `for` over index ranges or `while` with an
  explicit decreasing measure; closures, where used, capture shared references only. No
  `Arc<dyn …>`, no trait objects, no `unsafe`, no parallelism inside the core (an n-ary
  `par_map` stays outside it).
- **Fresh body, arena order kept.** `unify` allocates a new `Body`; it never edits an input body.
  Faces are pushed in the order of their first facet, edges in the order of their first cell,
  vertices in `verts` order: so a face's or edge's index is a function of the input arenas' order
  alone, and names stay stable from run to run and between native and wasm.
- **Cells by key.** A cell id is the index of its covector in `cells`; two ids are one cell iff
  their covectors are equal. The bridge must not create two ids for one covector (the Lean takes
  the covector as the key, so the refinement map is `id ↦ cells[id]`, injective).

Aeneas obligation: the translated `core` equals `ForgeExact.DecideOnce.core` on every ledger,
under that map. Where the Rust uses a union–find instead of `cls`, prove the classes equal (the
union–find's find is the class's least member in list order, which is what `head` returns), then
`head_eq` carries the names.

## 5. What is proven

All in `ForgeExact.DecideOnce`, all listed in `Axioms.lean`'s standard-axioms check.

**(a) Consistency by construction** (`Ledger.lean`): `Table.decideAll_once`,
`Table.get?_decideAll`, `Decisions.ask_build`, the closure laws of §2.2, `sideIn_class`.

**(b) Coverage** (`Coverage.lean`). `coverage`: for every `Consistent` ledger and every rule,
`core` returns `empty` (with no result facet), `ok b` with `Valid b`, or
`refused (zeroThickness x)` with `ZeroThicknessAt x` (an edge cell with more than two result
facets round it, or a vertex cell with two result facets in different fans). `Valid`: faces
partition the result facets; each face is flat (one class, one orientation); every result facet
has exactly one side kept; every edge cell has at most two result facets round it; every vertex
cell has one fan; faces are listed by first facet (names). `no_retry`: never `SplitCrossed`,
never `InconsistentClassification`. `core` is a total function, so it terminates. The heart is
`memb_eq`, the flood is right: for any membership `m` that gives the witness answers and flips
exactly across each body's pieces, the flood's membership of every side cell is `m`'s, by
induction along the chain joining the cell to its class's witness.

`Consistent`: each piece is a facet of its face's class, in a body that exists; no facet is
inside two faces of one body; and *some* membership realises the witness answers and the pieces.
This is any sign assignment some solids could give, not only the geometric one: a ulp nudge that
moves a decision inside the band gives another consistent ledger, and (b) holds for it too.

**(c) Correctness** (`Correct.lean`, `Nef.lean`). `Exact L ps A`: every decision in `L` is the
snapped geometry's (carriers `ps`, solids `A`); hypotheses listed in §6. Then:
- `Exact.consistent`: an exact ledger is consistent (so (b) applies).
- `result_iff`: a facet is in the result iff it is an exact boundary facet (`XBoundary`: a cell of
  the arrangement in one carrier, one side kept by the exact boolean).
- `correct`: and the regularised union of the full cells the exact boolean keeps is the
  regularised boolean of the solids: `reg_kept` over `topeDecomp`, the arrangement's full cells
  (any number of planes, the box family's grid generalised), compatible by `compat`.
- `correct_ok`: an `ok` body is valid and its faces partition exactly those facets.
- `correct_bool`: for two bodies and union, intersection, difference, the right side is
  `regBool o (A 0) (A 1)`.
- `zeroThickness_iff`: `ZeroThickness` iff `XNonManifold`: an edge cell of the arrangement with
  three exact boundary facets round it, or a vertex cell whose exact boundary facets form more
  than one fan, defined from the geometry alone.
- `nef_compat`: `compat` holds for solids given as half-space formulas (`Flat.HForm`, Nef
  polyhedra) whose planes are carriers in either sense, on every cell.

**(d) The six rescues** (`Rungs.lean`). `rung_covered`, `rung_exact`: any instance, under every
consistent reading of the fact its rung flipped, falls under (b) with no retry, and the exact
reading under (c). Flat instances, each a box ledger whose consistency is a kernel-checked
`decide`, so (b) applies as a theorem (`instances_covered`); their answers are checked at build
time by `#guard` (compiled evaluation: a test, not a theorem):

| rung | flat instance (after the snap) | core's answer |
|---|---|---|
| NO_RUN_ON (near_tolerance/20) | block on a plate, end walls flush; and the other reading, a gap kept | L-prism 8 faces, 18 edges, 12 vertices; 10 / 24 / 16 |
| CHORDS | cutter flush with the part's bottom, subtract | notch, 8 / 18 / 12 |
| ABSORB_HAIR_ROOTS | body on a flat, touching along its underside | 8 / 18 / 12, contact dissolved |
| FENCES | blade standing mid-face on a hub: a closed contact loop | 11 / 24 / 16 (the top keeps an inner loop) |
| CAREFUL | through-hole flush with both faces | ring, 10 / 24 / 16 |
| n-ary → sequential | three bodies in one call; and an edge touch among three | 8 / 18 / 12; `ZeroThickness` at the edge |

Five of the six inputs are curved. The flat proof covers the fact each rung flipped (one ledger
entry, decided once) but not their curved geometry, which waits for stage 2 (quadrics).

Also checked by `#guard` in Rungs.lean: two cubes at a corner, union →
`ZeroThickness` at the shared vertex (coverage.md's G1); at an edge → `ZeroThickness` at the edge
cell; face to face → one box, 6 / 12 / 8 (exact-comparison.md's "contact left imprinted" gone).

## 6. Hypotheses: what is not proven, and why

1. **`compat` for B-reps** (membership constant on each cell of the arrangement). Proven for
   half-space formulas (`nef_compat`). For a B-rep given by faces and loops, classified by ray
   parity, it is the Jordan–Brouwer property of a closed polyhedral surface: RefFacts' one
   assumption. Not proven: it needs a proof that ray parity is invariant under moving the ray's
   origin within a cell, i.e. the parity changes only when the origin crosses a face, which in turn
   needs the closedness of the surface (each edge used by an even number of faces, consistently)
   as a hypothesis and a general-position argument over the perturbed ray. Research-hard in core
   Lean without a topology library; stated, not used anywhere else.
2. **`boundary`, `complete`** (a facet is a piece of a face of body `i` iff body `i`'s membership
   flips across it). Jordan–Brouwer again, plus the point-in-face decisions being right. This is
   the bridge's obligation (README §8.2): the predicate that fills `pieces` must be the exact one
   in the representative plane.
3. **`witness`** (each witness answer is the solid's membership). The ray's obligation: exact
   rational ray parity with the reference's perturbation (`Ref.crossesYZ`).
4. **`edges`, `verts`** (the listed cells of dimension 1 and 0 round the pieces are exactly the
   arrangement's). The bridge enumerates them from the pieces; dimension is the exact rank of the
   zero classes' normals (`IsEdge`, `IsVert`).
5. **`pieceReal`** (each piece is a cell with a point). The bridge's: a cell with no point is
   never listed.

What the theorems do not claim:
- That each output face is a disc, or the body's Euler characteristic: `Valid` is local
  (manifold at every edge and vertex, faces flat and partitioning the boundary). The global
  topology is the exact set's, by (c), but is not restated combinatorially.
- §6.1 step 3, the output's sub-LINEAR features (edges shorter than LINEAR, slabs thinner than
  LINEAR, reported with their locus): not in this core. It is a pass on the output that needs
  metric facts; it should be a ledger of its own (each output edge's length against LINEAR,
  decided once) and is the next piece to specify.
- The kernel's split today is by the *other* body's carriers only; this spec splits each face at
  *every* carrier crossing it. Unify re-merges, so the faces are the same; the Rust must follow
  the spec (all carriers) for Aeneas equality.
- Curved surfaces (stage 2), and the bridge's float predicates (README §8.2).

## 7. The open bug classes of exact-comparison.md

| # | bug | how the spec removes it |
|---|---|---|
| 1 | notched part's ulp variants: SplitCrossed, EdgeDegenerate | point on edge is one covector comparison; `no_retry`: SplitCrossed unreachable; a crossing within LINEAR of a corner is the corner's cell if the snap merged them, else an exact cell of its own and a sub-LINEAR output feature (§6, not yet specified) |
| 2 | InconsistentClassification with no move | unreachable (`classifyOk_of`) |
| 3 | SplitCrossed with no move | unreachable (`splitOk_of`) |
| 4 | walls chained within LINEAR stay apart | the surface table is `Snap.rep`, a transitive class: one class, one carrier, unify merges |
| 5 | contact left imprinted | unify joins coplanar facets of one sense across an edge; the face-touch union is one box |
| 6 | band, two and three axes: SplitCrossed | as 1 and 3 |
| 7 | band builds where the snap makes a touch | the touch rule reads the exact cells: `zeroThickness_iff` |
| 8 | one-axis band: features over LINEAR read as zero thickness | no threshold in the core: ZeroThickness only when the exact result is non-manifold (`zeroThickness_iff`) |

## 8. Files

| file | content |
|---|---|
| `DecideOnce/Closure.lean` | `Conn`, `cls`, `cls_spec`, `cls_eq`, `head`, `head_eq`, `dedup` |
| `DecideOnce/Core.lean` | the ledger the core reads, the stages, `core` |
| `DecideOnce/Coverage.lean` | `Consistent`, `Valid`, `memb_eq`, `coverage`, `no_retry` |
| `DecideOnce/Correct.lean` | cells over ℚ³, `topeDecomp`, `Exact`, `correct`, `correct_ok`, `zeroThickness_iff` |
| `DecideOnce/Nef.lean` | `nef_compat`, `correct_bool` |
| `DecideOnce/Ledger.lean` | tables, `Fact`, `ask`, `ask_build`, closure laws, `sideIn_class`, `toCore` |
| `DecideOnce/Boxes.lean` | box ledgers, `checkConsistent`, `consistent_of_check` |
| `DecideOnce/Rungs.lean` | the six rungs, `rung_covered`, `rung_exact`, `instances_covered`, the `#guard` checks |
