# Parasolid transmit files

The reader is `kernel/forge-step/src/xt`; `forge_step::import` takes a transmit file as it takes a
STEP, IGES or `.brep` file, and so does every place that imports one (`import("part.x_t")` in a
part file, the Import tool). This directory holds what it was built and checked with.

## The format

Siemens publishes the *Parasolid XT Format Reference* openly (no sign-up): the V35 edition (2022)
at `q-solid.com/Parasolid_Docs_V35/pdf/xt.pdf`, older ones (2006, 2008) mirrored widely. It gives
the text and binary encodings (ch. 3), the embedded schema (2.1.2), every node's fields as of V35
(ch. 5), node types and classes (ch. 6) and the system attributes (appendix A).

What it does not give, and what was worked out here:

- **The base schema SCH_13006.** A file written since V14 carries its layout as edits (copy,
  delete, insert, append) of the V13 base; the reference prints only the current structures.
  `kernel/forge-step/src/xt/schema.rs` has the base rebuilt from the 2008 and 2022 structures and
  checked against every file of the corpus: each type's edits copy or delete exactly the base's
  fields, and every file reads node by node to its terminator. Differences from the printed
  structures: an ELLIPSE's sense comes before its centre, as on every other curve; LIST in the
  base is `node_id owner next previous list_type list_length block_length size_of_entry
  list_block`; HELIX_CU_FORM (184) and HELIX_SU_FORM (163) are in the base.
- **A cone's axis.** The reference has the cone widen against its axis; the files' cones widen
  along it (their intersection charts lie on that cone and not the other).
- **A transform's matrix** is applied by rows, `x' = (R x + t) s` with `R[i][j] = m[3i + j]`: the
  other way round stands the VF-3 3380 mm tall against its real 2717 mm (about 107 in), and takes
  bolts out of their holes (fewer coaxial cylinders between parts).
- **Senses.** A trimmed curve runs as its basis does, so an edge runs against its curve when the
  senses along the trimmed-to-basis chain multiply to minus. Swept and spun surfaces' normals,
  and blends', are found numerically (the file's definition against the kernel surface at a
  point; a blend face against the faces it is tangent to, by du x dv) rather than from rules.
- **Charts** of intersection curves are written to about a millimetre (`chordal_error` up to
  2.4 mm in the corpus); the reader puts points on both surfaces between them until the chain is
  within 0.01 mm, else a vertex cannot tell which part of a tight curve it is on.
- Knot and multiplicity arrays can be longer than `n_knots`, the rest zero or null.
- A ring edge (a closed curve with no vertex) is read as two halves, and a whole torus face (no
  edges at all) as two bands round its axis: healing and the mesher need vertices to work from.

Text files from SolidWorks wrap their records at 80 characters and never end one on a space.

## Which applications write it

| Application | Kernel | Exports .x_t / .x_b | Best route |
|---|---|---|---|
| SolidWorks | Parasolid | yes (Save As) — the whole corpus | .x_t |
| Onshape | Parasolid | yes (Export) | .x_t |
| Siemens NX | Parasolid | yes | .x_t |
| Solid Edge | Parasolid | yes (Save As) | .x_t |
| Autodesk Inventor | ShapeManager | reported (Save Copy As), not verified here | STEP |
| Autodesk Fusion | ShapeManager | not in Autodesk's own list; one third-party page says yes | STEP |

The native files (`.sldprt`, `.ipt`, `.f3d`, Onshape documents) are each application's own
container, not transmit files. A `.sldprt` embeds a Parasolid stream; reading it is a separate
piece of work, not started.

## The corpus check

`corpus.sh CMD` runs `CMD XT STEP` over every archive of the Haas corpus (branch
`haas-3d-models`, never checked out: each zip is shown out of git, extracted to scratch and deleted
after). `xtcompare` (`cargo build --release -p forge-step --example xtcompare`) reads both and
prints one JSON line: solids, refusals, and for solids paired by box and volume the volume, area,
area per surface kind and box against the STEP file's.

    XT_UNPLACED=1 XT_NOCHECK=1 research/parasolid/corpus.sh target/release/examples/xtcompare

`XT_UNPLACED` leaves each part where it stands in its own frame, as the STEP reader does (it does
not apply an assembly's placements); `XT_NOCHECK` skips the full topology check, which runs away
on one body (reported to KERNEL-STRESS), and counts a body valid when its integrals and mesh agree.
Face counts are not compared one to one: SolidWorks' STEP splits every closed face in two, its
transmit file does not; the area per surface kind is compared instead.

Other tools here:

- `xtproto.py`: a small Python reader of the text form (census of node types; the schema words
  each file wrote, raw).
- `extract.py IN.x_t BODY OUT.x_t [--bare]`: one body cut out with everything it reaches, the file's
  own schema words kept: the corpus fixtures in `kernel/forge-step/tests/data/xt/haas_*.x_t` and
  reproducers for the kernel. `extract.py IN.x_t list` lists the bodies.
- `fixtures.py OUT_DIR`: the hand-written fixtures (a block, a cylinder with rims of no vertex, a
  two-instance assembly) in text, embedded-schema text, neutral and little-endian binary.

## Results

See `results.md` (written from the last full run).
