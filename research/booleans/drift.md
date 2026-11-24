# Drift: how far vertices wander from their faces over a chain of booleans (2026-11-24)

FLAT-SITES, at the coordinator's request (δ-accumulation, after gap #2's ruling: tolerance-On is
judged against δ-Faithful). The questions:
- (a) whether the tolerances on record bound how far the model really is from its faces;
- (b) whether the adversarial wrong Ok becomes a refusal or a valid output;
- (c) a check, debug and fuzz only;
- (d) whether re-snapping at merge stops the ratchet, and at what cost.

Main is dca68ca3 unless said otherwise (the first runs were on 4846170a; nothing between them
touches this). The tool is `verify/forge-exact/fuzz/examples/drift_adv.rs`.

## 0. The answers

- **Ordinary modelling does not drift.** Every vertex of every gate part, and of 100-op chains of
  tilted cuts on one plate, is within 1.3e-12 of the exact meet of its planes (§2).
- **Adversarial chains drift to 1.5·LINEAR in a few ops and stay there.** It takes
  near-coplanar faces 1e-7 apart in angle, placed within two tolerances of one another. 50 ops,
  hill-climbed (§3).
- **The record broke in one place, two ways (fixed, §4).** Faces on one surface imprint each
  other's edges, but only the faces were found on the surface, not the edges.
  - An imprinted edge strayed past its own tolerance from the face it now bounded: an Ok with
    EdgeOffFace, a **wrong Ok** (the edge family's op 43 is one of these).
  - Its end vertex lay past its tolerance from that face's plane: under-recording against
    `Vertex::tol`'s own definition, "the ball every curve and surface through the vertex meets".
  - `imprint_tols` grows both. Every wrong Ok of this kind in the searches is now a valid output.
- **One under-record remains, by design of `settle`.** A vertex settled within its tolerance of
  an edge's curve can sit up to its tolerance **plus the edge's** off that edge's face (§5).
  - The composed bound always holds: `check::vertices_off_planes` asserts it after every boolean
    in debug builds.
  - The strict bound (the vertex's own tolerance alone) does not.
  - Making it hold costs a distance per settled vertex. That is a decision for David (§5).
- **A different wrong Ok turned up: an open shell after two ops** (§6). It is topology, not
  tolerance, and is still open.
- **Re-snapping does not stop the ratchet (§7).**
  - At merge, it never meets the drifting vertices: they were never merged, and their tolerance
    never grew.
  - Snapping every drifted vertex onto the exact meet of its planes makes things far worse:
    drift of millimetres, and up to 23 invalid ops in 50.
  - The ratchet lives where the meet is ill-conditioned (planes 1e-7 apart). There, recording is
    the only honest answer, not snapping.

## 1. Theory: why a ratchet is possible

A merge accepts two points within the larger tolerance of the two. The survivor's tolerance is
then the distance plus the merged one's (`grown_tols`, the alias arm: `distance + x.tol`). So
one merge can double a tolerance, and the worst case after N merges at one vertex is
2^N·LINEAR. One op can take a vertex to LINEAR off a plane without any merge: a tool face tilted
so it crosses the plane within LINEAR.

What a tolerance must bound, as the kernel defines it:
- `Vertex::tol`: the ball every curve and surface through the vertex meets.
- `Edge::tol`: how far the edge may lie from its faces.

`check` asks the two links: a vertex within its tolerance of each edge's curve end, and an edge
within its tolerance of each face at 16 points. Composed, they give a vertex within its tolerance
plus its edge's of each face. The strict reading, its own tolerance alone, is stronger.

## 2. Ordinary chains

`FORGE_DRIFT` measured every vertex after every boolean, against the exact planes of its faces
(dyadic) and the exact meet of its best-conditioned triple (`meet3`, |det| > 1e-3).

| run | plane, worst | meet, worst |
|---|---|---|
| every gate part (regen1, seq and nary1) | 6.7e-14 | 2.6e-13 |
| `a_hundred_tilted_booleans_on_one_plate` (100 ops, tilts to 0.05) | 7.5e-14 | 1.3e-12 |

Nothing grows with the length of the chain: it is rounding, and it stays rounding.

## 3. Adversarial chains

`drift_adv FAMILY 50 300`: 50 booleans on a 100×100×10 plate. Each op's six parameters are
hill-climbed for 300 rounds (seed 11) to maximise the worst vertex distance from its planes, or,
with `OBJ=under`, that distance less the vertex's tolerance.

The three families:
- **edge:** notches and walls whose faces end within ±2·LINEAR of the plate's y = 0 wall.
- **top:** 3 mm slabs whose bottom (a union) or top (a cut) is within ±2·LINEAR of the plate's
  top, tilted by up to 1e-7.
- **angle:** walls through the plate tilted 1e-9 to 1e-6 rad.

On main:

| family | worst drift | where | tol there | refused | wrong Ok |
|---|---|---|---|---|---|
| edge | 1.57e-5 | op 41 on | 1e-5 | 34/50 | op 43: EdgeOffFace 1.57e-5 |
| top | 1.51e-5 | op 34, held to op 49 | 1e-5 (under) | 12/50 | none |
| angle | 1.14e-5 | | 1.14e-5 (recorded) | 6/50 | none |

The refusals are honest: InconsistentClassification, SplitCrossed, ZeroThickness.

The exact reference (`forge-exact`) on the edge family's ops agreed on 2 and was ambiguous on 14;
on top and angle it timed out (> 120 s an op). Searching instead for under-recording
(`OBJ=under`) found 5.1e-6 (top) and 5.7e-6 (edge) past the vertex's tolerance on main. The same
top sequence held 11 wrong Oks (EdgeOffFace up to 1.29e-5).

## 4. The bookkeeping slip, and its fix

Top family, op 34 (`DETAIL=1`):
1. Face 79 of A is found on the tool's top's surface (the tilts differ by 1.7e-7 rad).
2. A's edges of face 79 are imprinted on that face.
3. The imprinted edges now bound the tool's top face, and their end vertex is 1.51e-5 from its
   plane with tolerance 1e-5.

`grown_tols` grows a vertex for the curves now ending on it (cuts, new edges, merged vertices),
never for a face of the other body that an imprinted edge now bounds. The edges had the same gap:
`imprint` checks the faces, not each edge, against the surface. On the `OBJ=under` top sequence
the imprinted edges strayed 1.29e-5 from the face with tolerance 1e-5, and `check` answered
EdgeOffFace on an Ok.

The fix, on branch proof/drift-tols (cff21c1e):
- **`imprint_tols`**, cold, runs only in booleans that imprint. Its work is skipped where the
  faces are on one surface: a plane but for rounding inside the margin below, or a cylinder bit
  for bit (`Imprint::exact`, decided once per pair of faces in the coplanar loop). It grows:
  - the pieces over each imprinted stretch, and an edge of A kept whole, to the curve's distance
    from the surface (a line from a plane exactly; a circle in closed form; else at 17 points, as
    `check` samples);
  - the end vertices the imprint reaches, to their distance from it.
  - Both get a rounding margin of 1e-11, so `check`'s own measure, rounded otherwise, agrees.
- **Tests:**
  - `an_imprinted_edge_off_its_new_face_grows_its_tolerance` (4 ops; EdgeOffFace on main);
  - `an_imprinted_edges_end_vertex_holds_its_new_face` (3 ops; vertex 1.51e-5 off, tol 1e-5 on
    main).
  - Both were shrunk from the searches by dropping ops while main still failed.

After the fix, on the saved sequences:

| sequence | main: wrong Ok | fixed: wrong Ok |
|---|---|---|
| top, plane objective | 0 | 0 |
| top, `OBJ=under` | 11 (EdgeOffFace) | 0 |
| edge, plane objective (op 43) | 1 (EdgeOffFace) | 0 |
| edge, `OBJ=under` | 3 (open shell, §6) | 3 (§6) |

The searches were then run afresh with the fixed kernel (all three families, both objectives, 300
rounds each):
- no EdgeOffFace anywhere;
- the open shell of §6 (edge family);
- the strict under-record of §5 (top 5.0e-6, edge 2.5e-6);
- the composed bound held in every op.

**Speed.** The exact callgrind gate, against main dca68ca3 on the same machine (a1328ce1, which
is cff21c1e but for formatting):
- Bits are identical; total +0.0009%. Every strict line is within the +0.03% band, but 49 of 70
  are up, so it is not zero.
- The worst line is noop regen1 at +0.0261%: +121 Ir of the startup layout artifact.
- After that: coplanar seq +0.022%, patterns seq +0.019%, upto-flat seq +0.019%.

Per function, what is left:
- `imprint_tols` itself, on the few imprints that need it. In gearbox: 24 seam lines of a
  cylinder lying on a plane, 6.6k Ir. In quadarm: 1.6k Ir.
- Codegen shuffles around it: `plan_once_in` +5k and `chords::walk` +3.7k/−2.5k in gearbox.

The first version cost up to 0.5% on bench parts. Each saving came from finding what dominated:
- Projections on curved surfaces: replaced with closed forms for planes and cylinders.
- Per-edge face lookups: replaced with one decision per face pair. A bool on `Imprint`
  re-shuffled the heap; `into_side: u32` keeps the record at 40 bytes.
- The decision in every `imprint` call: hole-grid's calls are the n-ary boolean's, which now
  passes false.
- Walking every piece: now done only when the whole stretch is past the edge's tolerance.

It is a wrong-Ok fix; the remainder is for the coordinator to judge.

## 5. What `settle` leaves: the strict bound

Top family, op 9 of the `OBJ=under` sequence: a cut of edge 102 (between a wall and the plate's
top) is settled between that edge's curve and a tool edge's.
- Its tolerance is LINEAR: the farthest curve misses it by 5.3e-6.
- Edge 102's curve itself lies 9.8e-6 off the plate's top. That is within the edge's tolerance:
  it was one of a slab's edges once.
- So the vertex is 1.5e-5 off the plate's top. That is inside its tolerance plus the edge's
  (2e-5), outside its own.

`check` passes, and the composed bound holds. `check::vertices_off_planes` asks that bound after
every boolean in debug builds. It runs against all of forge-bool's tests, and on every search
here it found 0. Release builds don't have it, so it costs nothing there.

For the strict bound, `settle` would measure the settled point against each face's surface as
well as each curve, wherever a curve misses it by more than rounding. That is a compare per new
vertex on the hot path, and a few plane distances on the rare ones. Or `Vertex::tol`'s doc could
say the composed bound.

**Ruling (coordinator, provisional; David may reopen it):** the composed bound. A hot-path check
in `settle` breaks the no-speed-loss rule. `Vertex::tol`'s doc now says so, and
`check::vertices_off_planes` asserts it in debug builds.

## 6. A different wrong Ok: an open shell

Edge family, `OBJ=under`, shrunk to two ops (by original index):
- op 2: subtract a notch across the y = 0 wall,
  q = [0.8234008506443795, 0.943091713370833, 0.9816978534888866, -0.9265032466161587,
  -0.3739713735018809, 0.8838505963032673];
- op 41: union a wall against it,
  q = [0.6199122167988423, 0.8434506663474743, -0.6076188493258957, -0.9263658549015378,
  -0.323243568577226, -0.2806420579990162].

The union returns Ok with four edges used once and Euler 3: an open shell. This is the same on
main and with the fix. It is topology, not tolerance bookkeeping. Open; not traced here.

## 7. Re-snap at merge

The probe (`RESNAP=` in `drift_adv`) runs after each boolean, outside the kernel. Each chosen
vertex:
- is moved to the exact meet (`meet3`, dyadic) of its best-conditioned triple of planes, when
  all its faces are planar and |det| > 1e-3;
- has each line edge at it, between two planes, rebuilt as their meet through it;
- has its tolerance reset to what is left, with the far ends grown to their miss of the rebuilt
  lines.

The modes:
- **merge:** vertices whose tolerance grew past LINEAR.
- **all:** every vertex more than 1e-12 off a plane it bounds.
- **cond:** as all, but not where two of its planes are within 1e-3 rad of one another.

Each family was hill-climbed afresh with re-snap in the loop (50 ops, 300 rounds, with the fix of
§4). The re-snap cost is the probe's: it scans every vertex after each boolean (O(V), outside the
kernel). A kernel version at merge sites would touch only grown vertices.

| family | mode | worst drift | wrong Ok | re-snapped / booleans | re-snap s / boolean s |
|---|---|---|---|---|---|
| top | none | 1.51e-5 | 0 | – | – |
| top | merge | 1.51e-5 | 0 | 41 / 10623 | 1.04 / 21.8 (4.8%) |
| top | cond | 1.46e-5 | 0 | 14772 / 10857 | 1.67 / 23.6 (7.1%) |
| top | all | **7.5 mm** | **23** | 85398 / 7442 | 2.95 / 29.2 (10%) |
| edge | none | 1.57e-5 | 0 | – | – |
| edge | merge | 1.57e-5 | 0 | 10 / 5027 | 0.37 / 37.1 (1.0%) |
| edge | cond | 1.43e-5 | **7** | 6769 / 4633 | 0.38 / 35.7 (1.1%) |
| edge | all | **2.9 mm** | **10** | 27013 / 4771 | 0.44 / 39.2 (1.1%) |
| angle | none | 1.14e-5 | 0 | – | – |
| angle | merge | 1.14e-5 | 0 | 0 / 13030 | 1.52 / 22.1 (6.9%) |
| angle | cond | 1.14e-5 | 0 | 23927 / 13026 | 2.35 / 22.8 (10%) |
| angle | all | **7.4 mm** | **15** | 77968 / 6844 | 2.86 / 38.2 (7.5%) |

- **merge** moves almost nothing (0 to 41 vertices in thousands of booleans) and changes no
  drift.
- **cond** takes 3 to 9% off the worst drift, but adds 7 wrong Oks (invalid results) in the edge
  family. Moving a well-conditioned vertex still moves the edges it shares with ill-conditioned
  ones.
- **all** destroys the model.

Merge sites are not where the drift is: the ratcheting vertices sit within LINEAR of their
curves, so their tolerance never grows and re-snap at merge never sees them. Snapping every
drifted vertex rebuilds edges as meets of planes 1e-7 apart. The meet of two such planes moves
by about 1/sin θ ≈ 1e7 times their error, and the model comes apart. Snapping only where the
planes are well apart (cond) leaves the worst vertices alone, since they are the near-coplanar
ones, and gains little.

So re-snapping cannot help where drift happens. It is near-coplanar faces, where the honest
answer is to record the distance (§4), or to refuse.

## 8. Open

- The open shell of §6.
- The strict vertex bound (§5): ruled composed for now; David may reopen it.
- The n-ary boolean (`nary.rs`) imprints by the same `imprint` but builds its own result. Its
  pieces don't yet take `imprint_tols`' growth: the gate's nary1 lines are diagnostic, and no
  search here ran it.
- A Lean statement of the composed bound per merge (δ_out = δ_in + edge tol, per entity), from
  gap #2's δ-Faithful.
