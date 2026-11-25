# PARASOLID handoff

Worker PARASOLID. The brief was a native Parasolid transmit-file (.x_t/.x_b) importer, with no
external dependencies, wired in everywhere STEP is offered. Branch `parasolid/xt-reader`. Results
are in `research/parasolid/results.md`, and the format notes, tools and app table in
`research/parasolid/README.md`.

## State

Merge-ready pending train 8. UX-LEAD OK'd it at 5b7c742f / 37712623 (the UI hasn't changed since).
The final PRE-VERIFY runs after TRAIN-CONDUCTOR's re-baseline merge, with /tmp/pv at 13d78f06 or
later.

What's on the branch:
- **Reader:** `kernel/forge-step/src/xt/`.
  - lex, schema (base SCH_13006 rebuilt), nodes, known.rs (11 schemas carried for files that
    name a schema without embedding it, as Onshape's translation service writes them).
  - geom: analytic, B-curves/B-surfaces, offset, intersection, swept, spun, trimmed,
    SP-curves, blends.
  - build: topology, assemblies and instances, names; heal and finish are shared with STEP.
- **Wiring:**
  - `forge_step::import` detects XT by content.
  - File > Import and `import()` in part files take .x_t/.x_b.
  - Machine models attach Parasolid (the "save as STEP" refusal is gone).
  - `forgecad import MODEL`, `forgecad convert IN -o OUT.step`, and the MCP `import` tool,
    which refuses .git destinations and links.
- **Kernel fix:** `Body::append` keeps `vertex_tol` (it panicked in debug).
- **Tests:** `kernel/forge-step/tests/xt.rs` with fixtures in tests/data/xt. These are hand-built
  (block, cylinder, assembly, multi-part, nested assembly, text and binary), Haas cuts, and six
  MIT-licensed Onshape exports, attributed in tests/data/xt/onshape/README.md. Plus forge-cli
  `model_cli.rs` and fg_mcp.

## Correctness work at the end (the wrong-Ok hunt)

- Pairing in `xtcompare` is global best-first now; `XT_BAD=1` prints each pair whose volumes
  disagree, with the XT solid's own verdict.
- **Found and fixed, 89cf7e2f:** SP-curves on a cone had NaN boxes (the cone's `bbox` over their
  uv span). One face integrated 421 mm² short, and the body passed `finish`'s 1% mass/mesh check
  at 1.45% under its STEP. `OnSurface::bbox` now falls back to its samples. The volume now matches
  the STEP to 2.5e-7. Fixture: haas_cone_sp.x_t.
- **Added, XT only (`cross_check` in build.rs):**
  - A volume of zero or less is refused as inside out (GM2-5AX solid 70 was −18.9e6 mm³).
  - Each face's integrated area is checked against the drawing (1%, plus twice the chord
    along its rim). One face off marks the body unverified.
  - The face check can flag a correct body where the **mesher** draws a face wrong. On the
    haas_cone_sp body, a face is drawn 72.9 against a true 9.6. That is safe: it never
    accepts a wrong body. It costs about 5% of read time. A cleaner second opinion is
    `mass::face_area` per face, but that doubled CM-1's read time.
- The 1% tolerance in shared `finish` (read.rs `mesh_agrees`) is what let a 1.45% error through.
  Tightening it for STEP too is for whoever owns `finish` (IMPORT-SPEED).

## Speed (not finished: the coordinator stopped speed work at wind-down)

XT reads ran 3.1× slower than STEP over the corpus. Callgrind on CM-1 (`forge_step::import`,
release):
- **Done (62ac5a04 + e004f79a):** 45% went to evaluating exact intersection curves, each
  point a Newton search on both surfaces. An intersection edge is now a C1 cubic through its
  densified chain with exact tangents, as STEP gives it. Each piece is checked at ¼, ½ and ¾
  against both surfaces to tol::LINEAR (1e-5 mm), refined ten times finer once, else the exact
  curve stays. CM-1 went from 4.23 s to 3.52 s, volumes the same to 7e-9. STEP takes 1.77 s.
  This is a sampled check, not a proven bound.
- **Next (not started):**
  - `forge_geom::blend::Pipe::project_near` → `rail_polish` (an 80-step golden-section
    search) → `Spline::closest` is 42% of what's left. Its callers are `finish`'s uv layout
    (`laid_out_in_a_speck`) and `record_deviations`. That is shared kernel code.
  - The XT-side alternative is fitting blend faces to NURBS (forge-step fit) before `finish`,
    as STEP files store them.
  - NURBS evaluation (`Nurbs::newton`/`basis`) is 17%.

## Open correctness classes (research/parasolid/results.md has counts)

- **"One rim and no pole"** on closed spline, cone and cylinder faces (67 in the corpus). These
  are refusals, never wrong bodies. A loop's winding is miscounted when edges meet at a closed
  curve's closing point. One cause is fixed (11abf0c4: an edge past a periodic B-curve's closing
  point is split there).
  - HA5C traced: two edges on a 308-segment closed curve share its closing point, and the loop's
    winding comes out 0.
  - The winding count in heal.rs `winding()` (16 samples per edge, `unwrap` near the last) is
    where to look.
- **232 measure-budget refusals:** waiting on KERNEL-STRESS's budget semantics.
- **Smaller classes:** a cliff-edge blend with no cliff (27), an SP-curve outside its surface's
  domain (20), no place for a second meridian (45).
- **Kernel cases with KERNEL-STRESS:**
  - forge-topo `procedural_box` takes the whole domain for a periodic-in-u surface (a closed
    extrusion's box is ±1e6 mm).
  - Mass is silently wrong on a closed cubic×linear B-surface band (an Onshape file, not
    committed: unlicensed).
  - "Loops draw no region".
- **Colours:** read, but neither `Solid` nor `Body` has a place for them.
- **Placed comparison:** against CAM-HAAS's STEP placement (machines/haas-model-import), not done.

## Risks

- `known.rs` layouts for schema numbers no file showed fall back to the nearest schema.
  A Parasolid release that changes a node type's fields would read wrongly before it refused. The
  embedded-schema path, which SolidWorks and Onshape exports mostly use, has no such risk.
- Binary .x_b is covered only by hand-built fixtures; no real binary export has been read.
- IMPORT-SPEED's branch changes `finish` to return a mesh too. On merge, xt/build.rs's
  `Solid { .. }` takes `Ok((body, mass, mesh))`. machine_model.rs should use
  `Solid::display_mesh()`, with the chord taken from the edges' box, not `faces_box`.
