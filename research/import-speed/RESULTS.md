# Opening a big imported model: results

Goal: the Haas VF-2 STEP (59 MB, 37 solid roots: 27 read, 10 refused, about 15k faces), file on
disk to model shown, in at most 1 s cold, every result bit-identical to main's importer.
Where the time went before: PROFILE.md.

Machine: the cloud VM, 4 cores (x86-64-v3 build), 15 GB. Native numbers are `forge-step`'s
`examples/stepopen` (the app's open: `import_drawn`, then each solid's display mesh), median of 3;
`forge-render`'s `examples/model_open` is the app function itself (`machine_model::read`).

## VF-2, file to display triangles

| | main 50af9c42 | import/speed |
|---|---:|---:|
| native, 4 cores, cold | 11.2 s | **1.95 s** |
| native, 1 thread (`FORGECAD_THREADS=1`) | 11.2 s | 7.0 s |
| native, re-open (content-hash cache, `machine_model::read`) | 11.2 s | **0.20 s** |
| wasm32-wasip1 under Node (1 thread, as the single-threaded browser build) | see below | 8.5 s |
| instructions, 1 thread (callgrind) | 149 G | 74 G |

Per phase, wall, 4 cores (main is single-threaded):

| phase | main | now | how |
|---|---:|---:|---|
| part 21 parse | 0.25 | 0.11 | pieces on all cores (taken only where each piece ends at the next's start) |
| topology (`Build`) | 0.49 | 0.10 | solids in parallel |
| heal, speck check, edge deviations, mass, check mesh | 9.5 | 1.75 | solids in parallel, largest first; faces/edges in parallel under the eval budget (`budget::map_until`); B-spline evaluation 2x cheaper |
| display mesh | 0.86 | 0 (made inside the read) | the check's mesh where provably the same computation; the rest drawn beside the read (`import_drawn`) |

## What changed (each a commit on import/speed)

1. `forge_step::nurbs`: the basis's work arrays sized to the degree (they were zeroed at degree 15,
   2 KB per evaluation, 7% of all instructions); evaluation specialised by degree (to 3 each way) and
   by the derivatives wanted; a Newton step's last point, `Procedural::point` and `deriv1` without
   derivatives nobody reads; each basis quotient worked out once (the second of function j is the
   first of j + 1). The same floating-point operations in the same order for every number made.
2. Solids read in parallel: every root's topology first (`build_root`), then the rest largest
   first (`finish_root`, `forge_math::par::map_eager`: every thread from the start), answers and
   stats put together in file order.
3. `forge_geom::budget::map` / `map_until`: a phase's items on all cores under a read's eval budget,
   with the answers one by one would give. Each item runs under what was left less what finished
   items spent; answers are taken in order while the spends summed in order stay below what was
   left (then, one by one, that item started with more than it spent: no budget test was true in
   either run), and the rest run one by one. `map_until` stops where one by one stops (a speck
   found; a face's integrals failing) and charges only what one by one spends. Used by the speck
   check, the edge deviations and `mass_rel` under a budget.
4. The check's mesh kept as the display mesh (`Solid::mesh`, `Solid::display_mesh`) where it is the
   same computation: made with no budget test true (some left after it) and on less than
   `budget::FILE` (what a display mesh outside an import gets), so a display mesh made later spends
   the same and meets no test true either. `import_drawn` draws the solids the check did not, each
   on its solid's thread after its read (no budget lives there: the drawing takes its own, as
   `display_mesh` does anywhere).
5. Part 21 in pieces (files over 4 MB): one pass to the DATA section's first instance; the rest cut at
   guessed instance starts; each piece read from its start with its own keyword and value numbering;
   taken only when every piece ends exactly where the next starts (a piece read from a place one pass
   reaches, ending between instances, is one pass's reading there), renumbered as one pass numbers
   (keywords in the order first met, values after those before); else one pass, which also gives
   the exact error. Limits are held on the totals, which only grow.
6. `forge_render::machine_model`: a STEP/IGES file's display meshes cached on disk by the hash of
   its bytes and `FORGE_KERNEL_HASH` (the sources that read and mesh it).

## Identical results: how checked

`forge-step/examples/stepdigest` hashes, per file and per solid: the body (`Debug`, with lazily
filled `OnceLock` caches taken out: whether a cache is filled yet is not what was read), the mass
properties, the display mesh, every refusal (entity, kind, words) and the read's `Stats`. Run over
the whole Haas corpus (`haas-3d-models`: 146 archives, 317 STEP/IGES files, 6.6 GB) and the 33 STEP,
IGES and BREP files in the repository: 350 files, 11,870 lines. Main's digest against each step's:
0 lines differ (steps 1-6 and `import_drawn`'s kept drawings). `forge-step`'s tests pass, plus new
ones: `budget::map_answers_as_one_by_one` (including runs out mid-way, and `map_until`),
`pieces_read_as_one_pass` (and a file whose guessed starts fall in strings), the cache's
whole-or-nothing round trip.

## Under 1 s

On this 4-core VM the read is now CPU-bound (about 90% of 4 cores): 7.0 s of work with every check
run before display. The checks are most of it: the speck refusal check 32%, `mass_rel` 20%, edge
deviations 15% (needed before the mesh: the mesher reads edge tolerances). Deferring the speck check,
the mass and the volume check until after display (the final `Imported` still identical, computed
after) measured 1.3 s, and shows 4 VF-2 solids those checks then refuse. That decision is David's.
