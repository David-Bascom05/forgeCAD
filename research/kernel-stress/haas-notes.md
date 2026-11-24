## How to read this

- **Rerun**: `tools/kernel-stress/run.sh` (all zips, about 2 h on 4 cores at `-j 3`; `-z DIR` for
  zips on disk, zip names to run some). It fetches `haas-3d-models` and reads each zip with
  `git show`, never checking the branch out; each zip is unpacked to a scratch dir and deleted.
- **Corpus**: `research/kernel-stress/corpus.tsv` holds the smallest cases of each failure class
  (zip, file, solid entity, op, what it gave). `python3 -I tools/kernel-stress/corpus.py run`
  replays them from the branch and says pass or fail now. No Haas geometry is in the repository:
  the README beside the zips says not to redistribute them, and this repository is public. The
  reproducers committed as tests are made from the kernel's own parts or written by hand.
- **Shrink one**: `python3 -I tools/kernel-stress/extract.py FILE.step ENT[,ENT] out.step` writes
  one solid (or several) as a STEP of its own; `target/release/forge-stress out.step --only OP`
  runs one op on it.
- Assemblies: the reader does not apply placements (`asm` in the per-machine table), so each part
  stands where it was modelled. Unions of a file's own solids are therefore of parts that happen
  to overlap in their own frames, which is still a fair test of the boolean.

## Fixed in this pass (forge-step, with reproducers in `kernel/forge-step/tests/stress_haas.rs`)

1. **Tolerances read with no room.** An edge (and a healed vertex) a file leaves off its faces got
   a tolerance of exactly the measured gap, so the same body moved anywhere, and any boolean of
   it, failed `check` (EdgeOffFace / VertexOffEdge at exactly that gap). Now 1e-6 relative + 1 nm
   for vertices; for edges 1e-3 relative + 1 nm, as `check` re-measures a trimmed edge at other
   points and a gap peaking between the reader's 256 samples came out 1.7e-5 larger (GM2-5AX
   Edge-Flange3, union: a wrong Ok). Tests: `a_tolerant_edge_still_checks_once_moved`,
   `a_tolerance_covers_the_gap_between_the_samples`.
2. **A rim between two seams at other angles.** A whole circle between two faces of revolution
   (SolidWorks writes turned parts with no seam edges) had its one vertex moved to each face's
   seam in turn; the second move broke the first face's loop. 24 solids failed `check`
   (LoopBroken + Euler), and 46 more were refused as "too long to measure" because their mass
   and mesh ran over the broken loops (08-1949's Sketch-Pattern parts). The circle is now split
   when its vertex is in use. Test: `a_rim_between_two_seams_at_other_angles_is_split_not_moved`.

The "before" figures for these are in the first sweep (16 zips): 24 LoopBroken solids, 47
refusals for time, 2 EdgeOffFace wrong Oks; after: 0, 1 and 0.

## Who owns what is left

| class | owner | why |
|---|---|---|
| boolean wrong Ok: FaceNoLoops, mass fails on the answer, mesh open on the answer | FLAT-SITES (planar/drift) | Oks whose bodies are invalid; the cases are in the corpus |
| boolean refused: NoClosedForm, SplitCrossed, InconsistentClassification, SplitThroughSingular, ZeroThickness, UntouchedOnOther | FALLBACK | refusals: the fallback path's work |
| section wrong (open loops; outline and holes swapped) | forge-agent `features::section` (mesh-based) | the mesh is watertight and consistently oriented, the loops come out signed wrong |
| mesh not watertight on a valid solid | forge-mesh | a crack at one chord (0.01) and not at 0.05 or 0.002, near a NURBS face's near-singular point |
| import: FaceWrongSide, LoopsCross, "loops draw no region", whole closed spline with no edges | forge-step (KERNEL-STRESS, next) | zero-area sliver faces (a file defect to heal), a torus face whose loop wraps nearly a turn, an O-ring as one closed NURBS face |
| import: entity missing or of another kind (Cobot) | none: a file defect | an EDGE_LOOP lists an EDGE_CURVE, an EDGE_CURVE's vertex is an ORIENTED_EDGE; refusing is right |

## Parasolid (.x_t)

Every zip holds the same machine as Parasolid text (`.x_t`, schema SCH_3000298 and later,
SolidWorks 2018 to 2026); there is no reader for it, so none of it is read here. What one would
take, from the files:

- **Format**: a text transmit file: a header, then a flat stream of nodes (BODY, SHELL, FACE,
  LOOP, FIN, EDGE, VERTEX, POINT, the geometry), each a type number and its fields by the
  schema. Newer files embed only the differences from a base schema, so the reader needs the base
  field lists (Siemens publishes them in the "Parasolid XT Format Reference"). Units are metres;
  every edge and vertex may carry a tolerance (Parasolid's tolerant modelling).
- **Geometry**: planes, cylinders, cones, spheres, tori map to ours exactly; B_SURFACE and
  B_CURVE to our splines; SP_CURVE (a curve in a surface's parameters) and TRIMMED_CURVE are
  simple; INTERSECTION curves (with their chart of points) map to `Curve::Intersection`;
  BLENDED_EDGE (an exact rolling-ball blend) and OFFSET_SURF, SWEPT_SURF, SPUN_SURF need our
  procedural surfaces. This is the gain: STEP gives the blends as fitted B-splines, x_t gives them
  exact.
- **Topology**: FIN is our coedge; faces carry their own sense. Parasolid, like these STEP files,
  puts no seam edges on closed faces, so the reader would go through the same `Proto` and
  `heal.rs` (seams, tolerances, vertex settling) as STEP does, and gain its fixes.
- **Estimate**: a parser and node table (about 1.5k lines), schema handling for the embedded
  deltas (0.5k), geometry mapping (1.5k, most of it the procedural surfaces), topology into
  `Proto` and the existing `finish` (0.5k): about 4k lines, two to three weeks, about the size of
  forge-step's STEP reader. Binary `.x_b` is the same with another tokenizer. Worth doing after the
  rewrite: STEP carries the same shapes and is read now; x_t would add exact blends and be
  smaller and faster to read (the HA5C2 x_t is 0.9 MB to the STEP's 2.5 MB).
