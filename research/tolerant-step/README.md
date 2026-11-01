# Tolerant gaps in imported STEP: prior art

Research only. Nothing in the kernel uses this yet. It is for the held commit 38fc6653 (branch
rel-1-held), which closes a loop's gap with a straight piece up to 1% of the body's diagonal.

## What the gaps are, in the files that refuse

- `loops.py` (here) checks every EDGE_LOOP junction in the source STEP. On 100003655, 100000463,
  100002613, 100002032, 100001789 and 100003098, **no junction lacks a shared VERTEX_POINT**.
- So no edge is missing from the topology. Every gap sits between a curve's end and the vertex the
  two edges share.
- ForgeCAD keeps that topology: `edge_range` projects each vertex onto its curve, and
  `check::LoopBroken` compares vertex ids. The gap the mass meets is therefore a curve end lying off
  its vertex. Parasolid and ACIS call that a tolerant vertex.

The 14 WHUCAD files refused with "its volume cannot be worked out" (step-rates w0/whu_man), as the
largest vertex-to-curve distance `voff`. The stated uncertainty is 0.005 in every one:

```
voff <= 1e-3        100000124 253 465 487 489 2613 2673 2674   (8)
1e-3 .. 5e-3        100000463 (4.0e-3), 100001734 (3.6e-3)      (2)
over the stated     100001789 (1.3e-2), 100002032, 100002066 (1.0e-2), 100003098 (8.1e-3)   (4)
```

The other refusals are in automate, freecad and nist. Most of them have the curve far off its face
(`coff` up to 27 on uncertainty 0.01 to 49). That is a different failure from a vertex gap.

WHUCAD parts are normalised to about size 1 but keep CATIA's 0.005. So 0.005 is 0.5% of a part,
and the file's own figure is no guard here. `heal_limit` already caps healing at 0.2% of the part
for the same reason.

## Prior art

Every kernel decides adjacency by topology. A tolerance only decides whether an adjacency the
topology already states counts as closed. None of them bridges two coedges that do not share a
vertex.

- **Parasolid** (Functional Description V35, ch. 16 "Session and local precision"):
  - "Think of edges as tubes, and vertices as spheres."
  - A vertex's precision must be at least that of every edge at it.
  - `PK_VERTEX_set_precision` refuses a sphere that would swallow an edge.
  - `PK_LOOP_close_gaps` closes only at tolerant vertices.
  - Mass properties (`PK_TOPOL_eval_mass_props`, ch. 27) return error bounds when asked. They read
    as quadrature bounds; nothing says they include tolerance.
- **ACIS** (R10, TEDGE/TCOEDGE/TVERTEX, `api_check_entity`):
  - "Loops with tolerant edges and vertices are not required to close to within SPAresabs".
  - The checker reports "coedge start/end point outside tolerant vertex".
  - `api_body_mass_pr` returns `est_rel_accy_achieved`, a quadrature figure only. The docs warn the
    result depends on the projection plane.
- **OCCT** (`ShapeAnalysis_Wire::CheckLacking`, `ShapeFix_Wire::FixLacking`/`FixConnected`):
  - An edge is "lacking" only when the two edges share the same vertex and the 2D gap exceeds
    2·max(UResolution, VResolution)(vertex tol).
  - The fix enlarges the vertex tolerance, or adds a closing edge, within `MaxTolerance`.
  - `FixConnected` merges distinct vertices only within a given precision.
  - BRepGProp never closes loops. See research/gprop-truth: it integrates from the surface's u1
    about the vertex centroid, so an open loop gives an answer that depends on where the origin is.
  - The STEP reader takes the file's uncertainty (`read.precision.mode` 0) and lets ShapeFix grow
    tolerances past it (`read.maxprecision.val`). On 100003655 OCCT's vertex tolerance is 6.8e-3.
- **Jackson, "Boundary representation modelling with local tolerances"**, Solid Modeling 1995,
  doi:10.1145/218013.218067. This is the Parasolid design paper. "the ends of the parameter-space
  curves on an edge must lie inside the appropriate vertex sphere … the curves bounding a face need
  not meet exactly end-to-end, as long as they are within vertex tolerance."
- **STEP**: only one `global_uncertainty_assigned_context` value per representation context. There
  is no tolerance per vertex or edge, so a reader has to measure them.
- No source gives a certified volume error bar for a tolerant B-rep. The bound below is derived
  here.

## The principled method

1. **Missing edge versus gap: by topology, exactly.**
   - A loop whose consecutive coedges do not share a vertex is a missing edge, and it is refused.
     That is `LoopBroken`, already in place.
   - A gap between curve ends at a shared vertex V is a tolerance question.
2. **The vertex ball, measured and capped.**
   - At import, r_V = max |V − curve end| over the edges at V.
   - Refuse if r_V > cap. The cap is min(stated uncertainty, a fixed fraction of the part), as
     `heal_limit` does.
   - Then a gap is legal exactly when both curve ends lie in V's ball. That is the condition
     `check::VertexOffEdge` already uses (max of edge tol and vertex tol), so the mass and the
     checker agree on what is closed.
3. **Close through the vertex, not straight across.**
   - Each face goes curve end a → V.point → next curve start b, both pieces inside the ball.
   - Every face at V then closes through the same point, so the boundary the integrals see is
     closed. The result no longer depends on the origin (there is no first-order term).
   - A straight a→b chord is a different chord for each face at V. Those chords leave a small hole,
     so the result keeps a first-order dependence on the reference point.
4. **An error bar for what the tolerance leaves undecided.**
   - Any two closures that stay inside the balls and tubes differ in volume by at most
     Σ_v (4/3)π r_v³ + Σ_e π t_e² L_e.
   - Area is undecided to Σ_e 2 t_e L_e. That bound is loose: it assumes each face's edge band moves
     the whole width.
   - Report these next to the quadrature error and refuse when they pass the accuracy asked for.
     Never fold them in silently.

**Measured: which closure the intended shape agrees with.**

- WHUCAD gives the feature list, so OCCT can build the intended part. These are the volumes from
  occt-rates/whucad/raw.jsonl. The mesh closes through the shared vertices (BRepMesh shares each
  edge's discretisation between its faces).
- On 100000253, 100003098 and 100002613 the mesh does not match the build either: these may be
  translation differences, not closure.

```
part       file gprop (open)  file mesh (through V)  OCCT-built from features
100003655  0.0071141          0.0076574              0.0076574   gap 6.8e-3: open loop 7.1% low
100002032  0.0148574          0.0147626              0.0147639   gap 1.0e-2: open loop 0.64% high
100000463  0.0704249          0.0710324              0.0712923
100002613  0.00014774         0.00015097             0.00014483
```

**Cost**

- Import: r_V per vertex and the cap, about 30 lines in `read.rs` next to `record_deviations`.
- Mass: `Way::Gap` from 38fc6653 split into two pieces through `vertex.point`, with its reach
  replaced by the vertex's ball. That is roughly the same size as the held commit.
- The error-bar sum: about 20 lines in `mass.rs`, carried on the result.

**Proof**

- Unit tests:
  - A block whose curve ends are moved off its vertices by δ ≤ r_V gives the block's volume
    within the bar.
  - The same block with δ > r_V is refused.
  - Moving the reference point changes nothing. Assert this; it is the test that a straight a→b
    chord fails.
- Corpus: the 14 WHUCAD files above against the OCCT build from features. Expect the 4 over the
  stated uncertainty (and 100000463 and 100001734 if the cap is 0.2% of the part) to refuse, and
  the others to match the build or the mesh within their bars.

**Decision for David:** the cap.

- The stated 0.005 lets 10 of the 14 through.
- 0.2% of the part (about 1.5e-3 to 3e-3 here) lets 8 through.
- 100003655 (gap 6.8e-3) would refuse under either cap, although closing it through its vertices
  gives the intended part exactly.
