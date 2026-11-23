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
4b. **Collapse the sub-LINEAR features** (§6.1 step 3; `DecideOnce/Metric.lean`, `coreM`). The
   metric ledger holds two more tables, decided once per side cell by the bridge: `thin` (the cell
   is thinner than LINEAR) and `enclosed` (every facet of the cell is a piece). The thin cells fall
   into features: classes under "adjacent across a facet" (`comp`). A feature's rim is the
   non-thin cells adjacent to it (`rim`). If every feature's rim is kept alike and every cell of it
   is enclosed, each thin cell takes its rim's keep (a feature with no rim keeps its own); the
   keep table is replaced (`collapse`). Otherwise the first thin cell (in `topes` order) whose
   feature is not so is refused `ZeroThickness` at that cell. With empty `thin` the stage is the
   identity.
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

## 3a. The local ledger: the region and its frontier (2026-11-23)

**Why.** §3 splits each face at every carrier crossing it. In a sequential history the target
grows, and its big faces carry every earlier tool's walls through them. On `coplanar` (a plate
gaining ~100 coplanar bosses, one boolean each) the ledger reaches 7 360 pieces and 40 000 cells
by step 30, every boolean rebuilds all of it, and the regen takes 230 s against the old path's
0.1 s. The op changes nothing far from its tools, so the ledger is built over a region round them.
Everything outside passes through. The core is unchanged: the region is a decision of the bridge,
made once and stated here so that (b) and (c) carry over.

**The region** (decided once per boolean, from the inputs alone):
- Body 0 is the target; bodies `1 .. nb` are the tools. `B` = the hull of the tools' boxes (their
  kernel vertices), and `R` = `B` grown by `G` on every side. `G` is the least power of two
  ≥ 8·LINEAR such that no wall of `R` (the six axis planes `x = R.lo.x`, …) is, as a point set, a
  carrier of any face of any input. If one is, `G` doubles, deterministically. So the walls are
  dyadic and are never carriers.
- **Applicability.** The rule must give `ρ(∅) = false`: outside every tool, the result is body 0
  if `ρ(e₀)` (union, difference, a history of adds and cuts from body 0), and empty if not
  (intersection). Otherwise, and also if the snap (§2.1, decided over **all** faces of all
  inputs) moves any face whose box does not meet `R`, the region is everything (`R = ⊤`). That is
  §3 as before, not a fallback to another path.
- **Near and far.** A face is *near* if its box meets the closed `R`, else *far*. Every tool face
  is near.

**The local arrangement.** The classes are:
- the snapped carriers of the near faces (§2.1, distinct point sets);
- the six walls, as six classes of their own (`walls`), carrying no face.

Cells, covectors and every table of §2 are over these classes, and the bridge lists only cells in
the closed region: every wall sign `inside` or `zero`. Then:
- `pieces`: the facets of the near faces that lie in the **open** region (every wall sign strict).
- `edges`, `verts`: the cells of dimension 1 and 0 round the pieces that lie in the **open**
  region. The *frontier* is the cells of the closed region with a zero on a wall. Frontier cells
  are not listed, so the touch rule and unify never look at them.
- `inside`: as in §3 stage 3. The ray (or the oriented-face decider of §6) runs against the
  whole input bodies, far faces included.
- `thin`, `enclosed` (stage 4b): as before, for side cells in the open region.

The core (§3, stages 1–8) runs unchanged on this ledger. Its faces are the result's facets in the
open region, joined by `joinRel`. A face of the core whose facets reach a wall is *open* there:
its boundary along the wall is frontier edge cells, which are not in `edges`.

**Why the frontier needs no decision.** Every tool lies in the open box `B`, and `B` is at least
`G` ≥ 8·LINEAR inside every wall. So within `G` of `∂R` the pointwise result equals body 0
(or is empty). With §6.1's snap this holds to within `K·LINEAR`, and `G > K·LINEAR` with room to
spare. Hence:
- (F1) **Far unchanged.** Outside the open region the result is body 0's set (or empty), so every
  far face, and every near face's part outside `R`, is a face piece of the result as it was:
  `result \ R° = body₀ \ R°` (or `∅`). This holds pointwise for any `ρ` with `ρ(∅) = false`, by
  the tools ⊂ `B`.
- (F2) **The frontier is manifold.** A frontier cell's neighbourhood is body 0's, and body 0 is a
  valid solid. So no edge or vertex on `∂R` is non-manifold in the result. `ZeroThickness` can
  only be at a non-frontier cell of the open region, which the core checks.
- (F3) **Stitching is local and exact.** A near face's edges that cross a wall cross it where
  their own two faces' planes meet the wall. Both faces are near, so both planes are classes, and
  the crossing is a vertex cell of the local arrangement: the same exact point from both sides.

**The write** (the bridge, after the core): the result body is written fresh from three sources:
- far faces, copied with their surface, sense, name and the kernel's own vertex points (unchanged
  geometry, so their bits are kept);
- each near face's part outside `R`: its loop edges clipped to the outside of the open box at
  their exact wall crossings (F3);
- the core's faces.

The final faces are the connected regions of their union. On each class plane and orientation,
the boundary segments are pooled, and every segment lying on a wall appears twice, once from each
side (F1), so it cancels. The remaining segments are chained into loops. Then:
- **Faces from loops.** Outer loops (positive area about the outward normal) take the holes they
  contain.
- **Names.** A final face is named by the least-named input face among its contributing far
  faces, near-face remainders and the core faces' `origin`, with §3's piece rule when several
  final faces come from one input face.
- **Edges and vertices.** An edge is a maximal straight run between one pair of final faces; a
  vertex is where three or more final faces meet. Points where a run crosses a wall are not
  vertices.

**What must be proven (extending (b) and (c)).**
- (b′) For a consistent local ledger, the core ends in `empty`, `ok b` with `Valid b` **away from
  the walls**, or a stated `ZeroThickness` at a non-frontier cell. This is (b) as it stands: the
  frontier cells are not in `edges`/`verts`, and `Valid` is over the listed cells.
- (c′) `Exact` over the local arrangement, for points of the open region: a facet in `R°` is in the
  result iff it is an exact boundary facet of the regularised boolean, and `ZeroThickness` at a
  cell of `R°` iff the exact result is non-manifold there. This is (c), with the arrangement of
  the near carriers and the walls, restricted to `R°`. `compat` holds per cell of that
  arrangement, because the far carriers do not meet `R°`: a far face's box is disjoint from `R`.
- (F1) as a lemma: for `ρ(∅) = false` and the tools' solids inside `B ⊂ R°`,
  `reg(ρ(A₀, …)) \ R° = A₀ \ R°` if `ρ(e₀)`, else `∅`.
- Global: the regularised boolean is the union of (c′)'s part in `R°` and (F1)'s part outside, and
  the result is manifold iff it is so in `R°` (F2). The write's stitching is the bridge's
  obligation (F3). It is checked by `forge_topo::check` and by the exact comparison, not proven.

**Cost.** Pieces come only from near faces, split only by near carriers and the walls. On
`coplanar` that is the tool, the plate top, and the few earlier bosses whose boxes meet the
region. Far faces cost one copy each.

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
pub struct MetricLedger {          // §6.1 step 3, built once, read-only
    pub thin: Vec<CellId>,         // side cells thinner than LINEAR
    pub enclosed: Vec<CellId>,     // side cells every facet of which is a piece
}
pub enum Rule { Union, Inter, Diff, Table(Vec<bool>) /* 2^nb rows for a history */ }

pub fn core(l: &Ledger, rule: &Rule, m: &MetricLedger) -> Outcome;   // the whole of §3 (coreM)
fn collapse(l: &Ledger, kt: &KeepTab, m: &MetricLedger) -> Result<KeepTab, CellId>; // stage 4b
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
- `brep_compat` (`DecideOnce/BRep.lean`): `compat` holds for solids given as the reference's
  B-reps (faces with loops, membership by ray parity, `Ref.Solid.contains`) that are closed and
  flat-faced, with every face plane a carrier up to a nonzero scale. It rests on
  `Jordan.contains_const` (§6.1 below).
- **(c) for every box family** (`DecideOnce/BoxesExact.lean`): `box_exact` proves every field of
  `Exact` for the box generator, `Exact (boxLedger K boxes) (gridPs K) (boxA boxes)`, for every
  finite list of boxes on the grid `0 .. K` (each of positive extent on every axis), with no
  other hypothesis. So `box_correct` (each `ok` body is the exact regularised boolean of the open
  boxes, its faces exactly the boundary facets) and `box_refused_iff` (the zero-thickness refusal
  is exactly `XNonManifold`) are theorems for all box families, under every rule, not only the
  instances of (d). The proof works in doubled coordinates (`DCell`: even = on a grid plane, odd =
  between): a cell's covector is `cov K c` (`covCell_eq_covOf`, `cov_inj`), its dimension is its
  number of odd coordinates (`isEdge_iff`, `isVert_iff`, from the rank of the zero normals), and
  the generator's facets, edges and vertices are the box faces' doubled cells and their bumps
  (`box_boundary`, `box_complete`, `box_edges`, `box_verts`). All of §6 2–5 is discharged for this
  generator.
- **(c) for every flat input** (`DecideOnce/Sample.lean`): `sample_exact` proves every field of
  `Exact` for `sampleLedger ps ss X`, the ledger built from the reference's flat B-reps `ss`
  (`Ref.Solid`, ray parity), the carriers `ps` and sample points `X`. The hypotheses are only:
  - nonzero carrier normals;
  - `FlatInput`: each solid is `FacesFlat` and `Closed`, with face planes carriers up to scale;
  - `Samples`: the samples hit every cell the boundary needs. This is implied by `Covers`, which
    asks for a point in every cell lying in a carrier, a statement about the arrangement alone.

  The decisions, each made once by evaluating the reference:
  - *piece of body `i`* (`flipB`): `i`'s ray parity differs at the two points stepped off the
    facet's sample across its carrier (`sideAt`, `side_cells`). No point-in-face test is needed.
    `flip_iff` proves this is exactly `Flip`, by `brep_compat`;
  - *side cell inside body `i`*: the ray at its side point;
  - *edge / vertex*: the samples' cells of rank 2 / 3 (`IsEdge`, `IsVert`, decided exactly) below a
    facet.

  `sample_correct` is then (c) for every flat input. With the complete sampler (`Sampler.lean`,
  `sampler_covers`), `flat_correct` states it with no hypothesis on the samples. Each `ok` body is
  the exact regularised boolean of the reference's solids, with its faces exactly the boundary
  facets. The zero-thickness refusal
  is exactly `XNonManifold`. Checked by `#guard` on boxes given as B-reps: a cube is 6 / 12 / 8,
  two cubes face to face give one box, and two cubes along an edge are refused.

**The metric ledger** (`Metric.lean`), for every ledger, keep table and metric tables:
`collapse_local` (no cell that is not thin changes), `collapse_comp` (one feature, one keep),
`collapse_uniform` (a thin cell takes its rim's keep), `collapse_idem` (collapsing again changes
nothing), `collapse_subset` (the collapse only deletes result facets, never adds one),
`collapse_refuse` (a refusal names a thin cell whose feature's rim is kept both ways or that is not
enclosed), `coverageM` ((b) with the stage: empty, a valid body, a stated ZeroThickness at a
non-manifold cell, or at a thin feature), `correct_off_thin` (for an exact ledger, a facet neither
of whose sides is thin is in the collapsed result iff it is an exact boundary facet). Checked by
`#guard`: a sub-LINEAR enclosed void in a block is filled (one box: 6 / 12 / 8, against 12 / 24 / 16
without the stage); a sub-LINEAR fin on a plate is refused at the fin's cell.

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

1. ~~**`compat` for B-reps**~~ **proven** (`ForgeExact/Jordan.lean`, `contains_const`; used by
   `brep_compat`). The reference's ray-parity membership of a B-rep whose loops lie in their face
   planes (`FacesFlat`) and whose edges are each used an even number of times, either way round
   (`Closed`), is the same at any two points strictly on one side of every face plane. This was
   RefFacts' one assumption. The proof, with no topology and no listing of configurations:
   - *perturbation* (`crosses_eq`, `pierced_eq`, `contains_eq`): the reference's simulation of
     simplicity (the ray's origin moved by `(0, ε, ε²)`) is the exact count at `q + (0, δ, δ²)` for
     every rational `δ` below an explicit bound (`bnd`: a quadratic's sign near 0 is its lowest
     nonzero coefficient's, `small_pos`), and there every test is in general position;
   - *one move* (`move_rise`, `move_level`, `move_x`): from `p + (0, δ, δ²)` to `q + (0, δ, δ²)`,
     a single straight move, with no staircase. Per face (`face_step`), its pierce status changes
     by the parity of its edges crossing the half-strip the ray sweeps along the move, and only
     faces ahead of the move count (`ahead_of_over`). Per edge (`seg_identity`), the edge's
     crossings of the two rays and of the move have the parity of its ends lying in the half-strip,
     which cancels round each closed loop (`telescope`). Summed over the faces, each edge is counted
     an even number of times (`xor_even`), so the parity does not change;
   - *the sweep* (`Aff.sweep3`), the one local fact, along one edge: crossing the boundary of the
     region where three affine functions are positive, at points where the other two are, has
     the parity of the region holding at the two ends. Proven by cases on the three functions'
     kinds (each positive, negative, rising or falling across [0, 1]), which hold for every
     geometry.

   `FacesFlat` and `Closed` are what the kernel's `check` certifies of the tools it hands over.
2. **`boundary`, `complete`** (a facet is a piece of a face of body `i` iff body `i`'s membership
   flips across it). Proven for the box generator (`box_boundary`, `box_complete`). **Proven for every flat input** (`Sample.lean`): with pieces decided by the
   ray on both sides (`flipB`), `boundary` is `s_boundary` (by `flip_iff`) and `complete` is
   `s_complete`, given `Samples`. Jordan–Brouwer again, plus the point-in-face decisions being right. This is
   the bridge's obligation (README §8.2): the predicate that fills `pieces` must be the exact one
   in the representative plane.
3. ~~**`witness`**~~ **proven** for B-reps (`DecideOnce/Witness.lean`, `witness_of_samples`): if each
   witness answer is the reference's ray parity (`Ref.Solid.contains`) at one point of its side
   cell, every answer is the solid's membership in the cell (by `brep_compat`). The point is given by
   `side_point`: from any point `m` of a piece (which `pieceReal` already asks the bridge for),
   the step `m ± δ n_P` with `δ` under `stepBound` (every other carrier's slack at `m`) lies in the
   piece's plus (minus) side cell. The bridge's remaining obligation here is only to evaluate the
   reference's ray exactly at that point.
4. **`edges`, `verts`** (the listed cells of dimension 1 and 0 round the pieces are exactly the
   arrangement's). Proven for the box generator (`box_edges`, `box_verts`). **Proven for every flat input** given `Samples` (`s_low`). The bridge enumerates them from the pieces; dimension is the exact rank of the
   zero classes' normals (`IsEdge`, `IsVert`).
5. **`pieceReal`** (each piece is a cell with a point). Proven for the box generator (`box_pieceReal`). **Proven for every flat input** (`s_pieceReal`: each piece is a sample's cell). The bridge's: a cell with no point is
   never listed.

6. ~~**`Samples`**~~ **proven** (`DecideOnce/Sampler.lean`, `sampler_covers`). The executable
   sampler `sampler ps` hits every cell lying in a carrier (`Covers`), so `flat_exact` and
   `flat_correct` make `Exact` and (c) theorems for every flat input. The only hypotheses are
   nonzero carrier normals and `FlatInput`. The route is a slab sweep, one carrier at a time:
   - *1D* (`cand1_ok`): the candidates are the roots, each root ± 1, each root's midpoint with the
     next root above (`minAbove`), and 0. For every `t`, one candidate is on the same side of every
     root;
   - *2D* (`cover2`): on a carrier, every other carrier is a line `α + β u + γ v` (`coef`, `coef_ev`).
     Take the 1D candidates of the critical `u` (`crit`: vertical lines' roots, and crossings of
     non-vertical lines). Between two critical values the lines keep their order (`aff_same`), so
     a point's place among the lines carries to the sampled vertical line (`emb`). There, the 1D
     candidates of the lines' heights (`hts`) give a sample with every sign;
   - *3D*: each carrier is a graph over two coordinates, solved for the first of z, y, x that its
     normal has (`lift`, `proj`, `lift_proj`).

   Any superset of the sampler's points also works, since `Covers` is monotone in the samples. A
   bridge may sort or deduplicate.

   For flat inputs, nothing in §6 remains a hypothesis: `flat_correct` holds for every closed
   flat-faced B-rep on carriers with nonzero normals.

What the theorems do not claim:
- That each output face is a disc, or the body's Euler characteristic: `Valid` is local
  (manifold at every edge and vertex, faces flat and partitioning the boundary). The global
  topology is the exact set's, by (c), but is not restated combinatorially.
- §6.1 step 3 is specified and its rule proven (stage 4b), but its two metric decisions are the
  bridge's: `thin` must mean the cell lies within LINEAR of its boundary everywhere (its inradius
  under LINEAR/2), and `enclosed` that every facet of the cell is a piece. Under those meanings a
  collapsed feature moves the result by under LINEAR (each filled or removed point is within
  LINEAR of the rim, which keeps its keep), which is the c·LINEAR slack of README §6; that metric
  bound is stated, not proven in the Lean, which is combinatorial. Short *edges* (under LINEAR
  between two vertex cells) are not collapsed: §6.1 step 2 collapses them before the boolean, and
  after it they bound a thin cell, which the stage handles.
- The kernel's split today is by the *other* body's carriers only; this spec splits each face at
  *every* carrier crossing it. Unify re-merges, so the faces are the same; the Rust must follow
  the spec (all carriers) for Aeneas equality.
- Curved surfaces (stage 2), and the bridge's float predicates (README §8.2).

- **The local ledger (§3a).** The witness answers are decided against the whole bodies (rays
  past far faces too), and a side cell beside one of a body's own faces takes that face's
  orientation: inside iff on its inner side. For a closed, valid body this is what ray parity
  says (Jordan.lean's per-cell constancy plus the face's orientation). The write's stitching
  (F3) is not proven: `forge_topo::check` and the exact comparison check it.

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
| `DecideOnce/BRep.lean` | `brep_compat`: `compat` for closed flat-faced B-reps |
| `Jordan.lean` | Jordan–Brouwer for the reference's ray parity: `contains_const` |
| `DecideOnce/Metric.lean` | §6.1 step 3: the metric ledger, `collapse`, `coreM`, `coverageM` |
| `DecideOnce/Witness.lean` | the witness ray: `side_point`, `witness_of_samples` |
| `DecideOnce/Sample.lean` | the ledger from samples for flat B-reps: `flip_iff`, `sample_exact`, `sample_correct` |
| `DecideOnce/Sampler.lean` | the complete sampler: `cand1_ok`, `cover2`, `sampler_covers`, `flat_exact`, `flat_correct` |
| `DecideOnce/BoxesExact.lean` | `Exact` for every box family: `box_exact`, `box_correct`, `box_refused_iff` |
