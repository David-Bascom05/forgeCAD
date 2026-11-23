# Curved decide-once on the B-rep route (route iii): design draft (2026-11-23)

Status: reviewed. dol-curved (session_01Q19LzxCYQCLaMBUHLhN7Zr) accepted the design on 2026-11-23
and takes F6, the 2D nesting lemma and the builder's Lean L2. Their review and dol-curved-rust's arc
model are folded in below (§1 step 1, the build order, nesting, §2, §5, §6). It goes to the master
for sign-off.

Context: research/decide-once/curved-design.md §2.5(b) and §7, and face-arrangement.md (A8).
curved Rust measured that the formula route (§2.5a, design D2) covers 6.8% of the bench parts'
curved booleans, and 0 of the 15 parts: every real part has a fillet, chamfer, draft, edit or
thread mid-history. So the master chose **route (iii)**, the B-rep route. Bodies carry no sign
formula. Facets come from the trimmed faces, and each room's membership is decided per room.

The aim is the flat A8 one: a face ledger (the v2 `inc` ledger, shared with flat) that the shared
`inc::core` runs on, with an end-to-end theorem. The difference: for curved, the theorem goes
**over the face ledger directly** (route B, already ruled), not through a carrier reference.

## 0. The design in six lines

1. **One ledger, one core.** Curved builds the same `inc::Ledger` as flat. `inc::core` and its
   refinement to `coreI` are shared, so curved needs no core proof of its own.
2. **Facets from trims.** Each near face is a trimmed patch on its carrier. It is cut by its trim
   loops and by the crossing arcs of the other bodies' near faces, and nothing else. Seams appear
   only where a facet must be cut (W2: cutting is allowed).
3. **Membership without rays.** Each room's membership comes from **orientation plus flood**.
   - A room next to a piece of body `i`'s face lies on that face's outward or inward side, and the
     face's orientation decides it exactly, as one sign.
   - Across facets that are not pieces of `i`, membership in `i` cannot change. The core's
     existing flood (`memb`: flood class, then the witness of its head) propagates it.
   - The builder fills `inside` for each flood head from an orientation seed in its class.

   Ray casting, point-in-face on curved patches, is needed only for a flood class with no piece of
   `i` at all. That is rare: a body with no face in the region. It is one certified ray or a
   counted Skip.
4. **Exact predicates.** These are all constructions plus signs, by "zeros are constructed, signs
   are evaluated":
   - crossing arcs (QSIC, exact for quadric pairs);
   - arc–trim vertices (QVert);
   - cyclic and radial orders from tangents at certified vertices and edge samples;
   - loop nesting on a face, by a certified crossing count along a constructed surface curve.
5. **The reference changes role.** The carrier ledger with formulas (`ExactK`) is no longer on the
   theorem's path. The L1/L3 statement is over the face ledger, from route B's F1–F5 plus one new
   obligation, **F6 (orientation)**. curved_exact's L2 becomes the exact face builder over B-reps.
   The formula evaluator stays as an L6 oracle for the formula subset.
6. **Skips** (§4) are counted, and the old path runs. There are no silent wrong answers.

## 1. Building the curved face ledger from B-rep trims

The inputs are the near faces of all bodies, as the region gives them (decide-once-spec §3a;
`far_unchanged` handles the rest). Each face `f` is a patch of its carrier `q_f` (a quadric, in Q1)
with:
- its trim loops, which are B-rep edges with exact end vertices. Each edge is an arc of
  `q_f ∩ q_g` for the adjacent face `g`, and each vertex is a certified point of three carriers;
- its orientation `o_f ∈ {±1}`: the body lies on the side `o_f · q_f < 0` locally. This is the
  B-rep's face sense relative to the carrier's gradient, and it is exact.

Steps, all exact, with each step's Skip named:

1. **Crossing arcs, by the sweep** (dol-curved-rust's model, adopted 2026-11-23). For each pair of
   near faces `f`, `g` from different bodies on different carriers, the curve `q_f ∩ q_g` is built
   from proposers that are already certified, with no parametrisation:
   - **Events.** These are the x's of the discriminant of the curve's y-projection: `R(x, y) =
     Res_z(q_f, q_g)` (the chain machinery computes and checks it), then the PRS of `(R, ∂_y R)`,
     Bezout-checked, with x-only RootCover. They cover turning points and apparent crossings, so
     between events the branches keep their y-order.
   - **Slabs.** On a rational plane between events, each crossing is a certified edge sample
     (QEdge), sorted by y.
   - **Matching at an event.** Two rational planes `x_e ± δ`, and QArc boxes round the fiber's
     multiple points, match the branches on each side.
   - **Arcs** are chains of slab pieces. A trim vertex (QVert) is placed on its arc by its x and
     its rank in y within its slab. The arc's "parameter" is the sweep order. A clip or a midpoint
     is a slab sample between two vertices.
   - **The frame must be generic**: no vertical component of the curve, and a nonzero `R`. The
     sweep runs in a tilted rational frame. If a frame's check fails, the builder retilts once;
     if it fails again, `Uncertified`.

   What the proofs need from step 1 is completeness (every arc of `q_f ∩ q_g` in the trims is
   found, F2) and exact adjacency (which branch continues which at an event). The sweep gives
   both from QSample's completeness and the event certificates. No obligation (F1–F6, the core,
   `flood_correct`) needs a parametrisation. The write needs each edge's carrier pair, its end
   vertices and one interior sample to fix the branch; the sweep gives all three. If the kernel's
   curved edge type wants a parametric curve, that is a conversion in the write, not in the
   ledger.

   **Build order, per face** (dol-curved: the clip in step 1 depends on step 3's nesting): the
   trims first; then all crossing arcs on the face, unclipped; then trace the face graph (step 3);
   then keep the arc pieces that lie on the face, and clip.

   For two faces of different bodies on one carrier (coplanar or co-quadric overlap), the imprint
   is `f`'s trim edges that lie inside `g`, as in flat's cut set. That needs the same nesting test
   as step 3.

   Skips: `Uncertified` (a vertex, an event certificate or a sample fails), `OutOfScope` (a
   surface kind beyond the phase).
2. **Vertices.** These are the trim vertices plus the crossing-arc ends, deduplicated exactly: one
   certified point is one vertex (QVert's isolating box, then an exact equality test as a fallback).
3. **Facets, per face.** On the face's carrier, the trim loops and the crossing arcs form a graph,
   embedded on a surface that is a quadric patch.
   - **Faces of the graph** are traced by the cyclic order of arc ends at each vertex: the
     tangents `±(∇q_f × ∇q_g)(v)` projected to `q_f`'s tangent plane, compared exactly as
     curved-design §7 describes. At a tangential vertex the order comes from the link (an isolating
     box's boundary crossings, which are edge samples).
   - **Nesting.** A traced loop's region may hold others: annuli, holes. Whether a loop `L` lies
     inside loop `M` is decided by a crossing count along one of the sweep's own sample planes
     `x = c` in the tilted frame (QSample), as dol-curved advised. `q_f ∩ {x = c}` is a conic, its
     crossings with `M`'s arcs are certified edge samples (`proposeEdgeAny_complete`), and the
     y-order along the conic is exact, so the count is the one QSample already proves complete.
     On a closed carrier (a sphere, or a cylinder band between caps) "`M`'s outside" has no
     meaning. So the count is anchored at a point known to be in the face: a trim-edge sample
     pushed to the face side by `o_f` and the trim's orientation. This is a 2D Jordan on a quadric
     patch, a much smaller thing than 3D ray parity.
   - **Simple connectivity** isn't required. A facet with holes is one facet (W2; the write handles
     holes). The builder cuts only where a facet would otherwise be a wing at an edge or a pinched
     sector at a vertex. The cut is a seam piece in the face: `q_f ∩` a rational plane through
     the edge or vertex (curved-design §7's Tier-0 seams).

   Skips: `Budget` (no isolating box for a tangential order), `Uncertified`.
4. **Edges and wings.** Each kept arc between consecutive vertices is an edge. `edge_facets[e]`
   lists the facets on both sides of `e` in each incident face: one wing per side reached.
   - A crossing arc inside face `f` contributes two wings of `f`'s facets, if the arc splits `f`,
     or one facet twice, if it is a slit.
   - A trim edge contributes one wing per adjacent face.
5. **Sectors.** `vert_sectors[v]` lists, per incident face, the consecutive arc pairs round `v` in
   that face with the facet between them. That facet is a lookup once step 3 has traced the faces.
6. **Rooms: the radial order.** Round each edge `e`, the facets of all faces through `e` are
   ordered radially.
   - Transversal case: by the angle of their inward tangent directions in the normal plane of `e`'s
     tangent at an edge sample. That is an exact comparison of rational-function values at an
     algebraic parameter (`Num.sign`).
   - Tangent faces along `e`: by second-order terms. If that is still undecided, `Skip::Tangent`.

   Consecutive facet-sides face one wedge and are joined by union-find. The classes are the rooms
   (as flat's §1a).
7. **inside, by orientation and flood.** See §2.

## 2. Membership per room, exactly, without rays

A facet `φ` that is a piece of face `f` of body `i` has its two rooms on `f`'s two sides.
`o_f` says which side is body `i`'s inside, so the room on the inside is in `i` and the other is
not. These are **seeds**, one sign each.

The core already computes membership as follows:
- `memb i R` is the witness of the head of `R`'s flood class;
- the flood joins rooms across facets that are **not** pieces of body `i`.

So the builder fills `inside` like this. For each body `i` and each flood class of rooms, find a
room of the class next to a piece of `i`, read its membership from the orientation, and set the
witness for the class's head. This is O(rooms + facets) per body, with no geometry beyond `o_f`.

If the orientations disagree within a class, the B-rep is invalid (or a decision upstream was
wrong). The core's `classifyOk` refuses `InconsistentClassification` exactly then (a piece of `i`
with both sides alike). The builder may also detect it first and Skip.

**A flood class with no piece of `i`** means body `i` has no face anywhere in that part of the
region. Then its membership is constant there. It is decided by one certified ray from a room
sample to infinity, counting crossings with `i`'s faces. That needs a 3D point-in-face on curved
trims, done with the §1 step-3 nesting test on each hit face. If it isn't certified,
`Skip::NoSeed`. This also covers a class left seedless by the region itself, where all of body
`i`'s faces are far (dol-curved). `far_unchanged` covers what they bound, but the builder must
know about such a class in order to seed it, so it counts the class. On the gate, near bodies always have faces in the region, and a seedless class
arises only for a body entirely containing, or missing, a pocket of the region. Expect it to be
rare. The builder counts it.

### Why it is exact: the obligation F6, and a combinatorial lemma

- **F6, orientation (new, geometric; LemmaD session or tier0).** For every face `f` of body `i`
  and every point `x` of `relint f`, small neighbourhoods of `x` meet `A i` exactly on `f`'s
  inside (`o_f · q_f < 0`). This is the B-rep validity of the producing feature's output, stated
  locally. It is true of every valid solid with outward-oriented faces, and decide-once's own
  outputs have it by construction (the write orients by `orient`). dol-curved states F6 with
  a hypothesis per body, "`A i` is the solid bounded by its faces", rather than proving it from
  nothing. `forge_topo::check` plus the producing feature's own validity supply that hypothesis.
  `o_f` itself is exact: the kernel's `Face.same_sense` says whether the outward normal is the
  surface's natural normal. For a plane, the natural normal is the oriented normal. For a
  quadric, the natural normal's sign against `∇q_f` (for the canonical `QFace.toQuadric`) is a
  fixed per-kind rule, which dol-curved states as a lemma per kind beside F6.
- **F5 (already route B's):** rooms are open and connected, and disjoint from all faces. With F6 and
  `∂(A i) ⊆ ⋃ faces of i`, membership in `i` is constant on each room, and across a facet it flips
  iff the facet is a piece of `i`.
- **`flood_correct` (combinatorial, mine, over IncLedger).** Suppose:
  - memberships `m : room → body → Bool` flip across exactly the pieces of each body, and are
    constant across the other facets;
  - `inside` agrees with `m` at each flood head.

  Then `memb i R = m R i` for every room. The flood joins only rooms with equal membership, and
  the head's witness is right. This is Coverage.lean's `classify` argument restated over rooms,
  about 150 lines. With it, `coreI`'s keep is the true keep of each room, which is what tier0's
  bridge (2) (`Res = reg (combine …)`) consumes.

So curved's (c) is: F1–F6 (geometric), then `flood_correct`, then bridge (2), then bridge (1) (done),
then the shared `inc` refinement. There is no carrier reference and no Jordan in 3D. The one
"Jordan" left is the 2D nesting test on a quadric patch, inside the builder's exactness (step 3).

## 3. What changes for curved_exact's L2 and for route B's obligations

**The L2 reference.** Today curved_exact's L2 decides memberships by evaluating sign formulas on
carrier cells (`qform_compat`, `historyBF`). Under route (iii), bodies have no formulas. So:
- **The carrier ledger leaves the theorem's path.** Route B already removed `Faithful`. Now
  `ExactK`'s witness clause has no formula to come from, and nothing on the path needs it, since
  bridge (2) works on the face ledger. CorrectK.lean stays proven, as the formula subset's
  theorem.
- **L2 becomes the exact face builder over B-reps:** §1's steps, executable in Lean over QLine,
  QEdge, QVert and RootCover (the dol-curved branch's L2 components). Proven against L1 means:
  its outputs satisfy F1–F6 (route B's obligations), plus `flood_correct`'s hypotheses. That is
  "the builder's exactness", the curved twin of flat R3's L3.
- **The formula evaluator stays as an L6 differential oracle**, on the 6.8% of booleans the formula
  route covers, and on synthetic formula bodies.

**Route B's obligations.**
- F1, F2, F4 (tier0) and F5 (LemmaD) stand as stated: they are about the face ledger.
- **F6 (orientation) is new.** It is the only place the B-rep's validity enters. It replaces
  `qform_compat`'s role.
- The delineability lemmas (Sweep.lean, `qsampler_covers`) were for the reference's sampler. They
  are no longer on the path. Their face-ledger analogue is completeness of step 1's crossing arcs
  and step 3's faces (F2: row completeness), which certified tracing's (ii) supplies.
- **New builder lemma:** the 2D nesting test on a quadric patch (a crossing count along `q_f ∩
  plane`), with F3 (separation) for loops.

## 4. Skips that remain (all counted; the old path runs)

| Skip | When | Expected on the gate |
|---|---|---|
| `Uncertified` | a crossing vertex, a trace chain or an edge sample fails certification | rare (filters plus exact fallback) |
| `Budget` | a tangential vertex's isolating box isn't found within budget | rare |
| `Tangent` | the radial order of faces tangent along an edge is undecided at second order | tangent fillets on tangent faces: measure first |
| `NoSeed` | a flood class with no piece of some body, and the certifying ray fails | rare |
| `OutOfScope` | a surface kind beyond the current phase (tori before Q2, splines before Q4) | the bench's tori and splines until those phases |
| `Inconsistent` | orientations disagree within a flood class (invalid input B-rep) | 0 on valid inputs; the core also refuses it |

There is no `Wide` (v2 has no covectors) and no formula Skip.

## 5. Work split (proposal)

| item | owner | needs |
|---|---|---|
| §1 builder, Rust (stages b–f) | curved Rust (dol-curved-rust) | this design |
| `flood_correct` over IncLedger | the Lean coordinator | — |
| F6 statement and proof, and the per-kind sense lemma | dol-curved (assigned by the master) | F5's room facts |
| 2D nesting lemma on a quadric patch, and the builder's L2 in Lean | dol-curved | QEdge/QVert |
| bridge (2) | tier0 | F1–F6, `flood_correct` |

## 6. Questions settled in review (2026-11-23)

1. **`o_f` is stored exactly**, as `Face.same_sense`, which stage (a) reads into `CIn.outward`.
   There is no sample. See F6 above.
2. **The tangent radial order.** Fillets are tangent to their neighbours along their own trims.
   Those are B-rep edges, so the tangency is in the input topology and `Skip::Tangent` does not
   fire for them. A tool face tangent to a fillet along a crossing is rare. It is measured before
   anything is designed for it.
3. **The nesting plane** is the sweep's own sample plane `x = c` in the tilted frame, anchored in
   the face on closed carriers (§1 step 3).
4. **The arc model** is the sweep (§1 step 1), not the pencil parametrisation. No obligation needs
   a parametrisation.
