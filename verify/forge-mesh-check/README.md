# forge-mesh-check: what a tessellation of a valid body is

`forge_mesh::tessellate` (kernel/forge-mesh, at 8250ebb) cuts each body edge once. Both faces use
those cut points, so the faces' triangulations meet exactly. This directory proves:
* what the kernel's watertightness test (`TriMesh::open_edges`) does and does not establish;
* that the mesh volume is then well defined;
* that the chord steps along circular edges meet the chord tolerance.

The tests check the kernel's meshes against all of it, and the mesh volume against the exact
volume.

## The proofs (`lean/`, Lean 4.36 + Mathlib, sharing verify/forge-mass's checkout; no `sorry`; `Axioms.lean` fails the build unless all 11 cited theorems use only the standard axioms)

| file | proven |
|---|---|
| `Mesh.lean` | `closed_of_open_edges`: `open_edges` empty (mesh.rs:57–76) means every directed edge occurs once and its reverse occurs. So each edge has exactly two triangles, walking it opposite ways: closed and consistently oriented. `area_vectors_cancel`: the triangles' area vectors then sum to zero. `volume_translate`: so `volume()` (mesh.rs:24–33) does not depend on the origin. `pinch_passes_open_edges`: two tetrahedra sharing one vertex pass `open_edges`, with two fans at that vertex. `open_edges` alone does not make a mesh manifold, as its doc says it does. |
| `Chord.lean` | `by_chord_sag`: `arc_step`'s step 2 acos(1 − chord/r) sags exactly `chord`. `segments_sag`: each of `edge_segments`' ⌈w/step⌉ chords sags at most `chord`, unless the 1e-4 floor on the step binds. `floor_overshoots`: it can bind, where chord/r is below about 1.25·10⁻⁹. `sqrt2_over_half`: on surfaces curved both ways the √2-shortened step sags more than half the tolerance, so two such sags exceed it. |

Assumed, not proven: that a closed, oriented mesh's `volume()` is the volume it encloses (the
divergence theorem for polyhedra), and the sag bounds for triangles inside faces
(research/mesh-bounds: derived in part, measured in part).

## The tests (`cargo test --release -p forge-mesh-check`)

- `meshes_are_closed_oriented_manifolds`: 24 benchmark and fuzz parts at chords 0.1 and 0.01.
  `open_edges` is empty, and every vertex has one fan (`vertex_fans`, the check `open_edges`
  lacks).
- `volume_within_chord_bound`: the same parts at chords 0.5, 0.1 and 0.02.
  |V_mesh − V_exact| ≤ chord·area throughout; the worst is 0.198 (hole-grid at 0.02).
- `volume_is_translation_invariant`: a mesh moved 10³ away keeps its volume to rounding.
- `circle_chords_within_chord`: 250 420 chords along circular edges each sag at most `chord`.
- `sqrt2_step_excess`: up to `max_angle`, two √2-steps sag at most 1.000714 of one full step.

## Found

1. **`open_edges` does not establish "manifold"** (mesh.rs:55–56). Its doc says empty means
   "closed, manifold and consistently oriented", but `pinch_passes_open_edges` passes it with a
   pinched vertex. On the kernel's meshes every vertex has one fan, as the B-rep's do (`check`'s
   fans), so nothing is wrong in the meshes. But a consumer reading `open_edges` as a manifold
   certificate, such as mesh export, gets less than the doc says. The fix is to the doc, or to add
   the fan test.
2. **The √2 rule on doubly curved faces is not exactly "half each way".** `edge_segments` with
   `fine` (tessellate.rs:42–64) gives each direction up to 1.0007× half the tolerance.
   `interior_steps` has an extra 0.9 that covers this inside faces, but edges get no such
   margin. The measured mesh bounds still hold (the refinement checks triangle middles). This is
   a gap in the derivation, not a failing mesh.
3. **The 1e-4 floor on `arc_step`** lets chords sag past `chord` where chord/r < ~1.25·10⁻⁹ (a
   radius of 8 km at a chord of 0.01). It is not reachable in practice; it is a precondition of
   the bound.
