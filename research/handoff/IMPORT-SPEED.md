# Handoff: opening a big imported model fast (IMPORT-SPEED)

Branch `import/speed-ks` (queued for train 8, right after kernel-stress). What it does, what is
proven, what is left, and where I would go next.

## What was asked, and where it stands

Open a big imported model in at most 1 second — the Haas VF-2 STEP (59 MB, about 15k faces), which
main read in 11.2 s — then hold that across the other Haas models, with every result bit-identical.

| | main | import/speed-ks |
|---|---:|---:|
| VF-2, 4 cores, every check done, every solid drawn | 11.2 s | **2.28 s** |
| VF-2, the app's own read (`model_files::read`) | 11.2 s | **2.19 s** |
| VF-2, re-open (the app's display-mesh cache) | 11.2 s | **0.18 s** |
| VF-2, one thread | 11.2 s | 7.6 s |
| VF-2, wasm32 under Node, one thread | 13.9 s | 9.2 s |
| the frozen corpus (20 files), median / worst | — | 4.5 s / 13.2 s |

So: 3-6x faster everywhere, 1 s met only on the small rotaries (2 of the 20 corpus files), and the
fully checked VF-2 is at 2.3 s, not under 1 s. David ruled that a model is shown only when its
checks are done (no provisional view), so that number is the one that counts. RESULTS.md has the
per-file table, CHECKS.md the per-check breakdown.

Bits: the whole Haas corpus and the repository's fixtures — 350 files, 12,241 digest lines (body,
mass properties, display mesh, every refusal and its words, the read's stats, the assembly's
occurrences) — identical to main's read through `import` and through `import_drawn`, and the app's
path identical piece for piece over all 348 STEP/IGES files. `forge-step/examples/stepdigest` and
`forge-machine-builder/examples/model_read` are the tools; `/tmp/claude-0/bin/digest-corpus.sh` in
my session was the driver (one line per file, 3 files at a time, diffed against a baseline build).

## The work, commit by commit

Each of these is bit-identical by construction, and was diffed over the whole corpus:

1. `forge_step::nurbs`: the B-spline basis's work arrays sized to the degree (they were zeroed at
   MAX_DEGREE 15 on every evaluation: 7% of all instructions in `memset`), evaluation specialised
   by degree and by which derivatives the caller wants, each basis quotient worked out once. The
   same floating-point operations in the same order for every number made.
2. Solids read on all cores (`forge_math::par::map_eager`), topology first, then largest first;
   answers and stats in file order.
3. `forge_geom::budget::map` / `map_until` / `map_prefix` / `map_counted` / `map_all`: a phase's
   items on all cores under a read's budget, giving what one by one gives. Used by the speck check,
   the edge tolerances, `mass_rel`, `heal::tighten` and the mesher.
4. The check's mesh kept as the display mesh where it is the same computation.
5. Part 21 parsed in pieces (files over 4 MB), taken only when every piece ends where the next
   starts, renumbered as one pass numbers.
6. `heal::tighten` planned per edge on all cores, applied in edge order.
7. A file body's faces (16 or more) meshed on all cores, merged in face order.
8. `forge_topo::mass`: a face's integral split over its loop pieces on all cores (the same shares in
   the same order), Chebyshev tile sums reading only their slab, cached row integrals.
9. `model_files::read` (the app's and the CLI's own read, which train 7 moved there): each solid's
   display mesh on all cores, taking the read's kept mesh where it has one. VF-2 4.9 s -> 2.2 s.
10. `forge_geom::budget::isolated(limit, f) -> (spent, T)`: one piece under a budget of its own,
    its spend handed back. KERNEL-STRESS is adding the same primitive on op-budget; same signature.
11. `forge-mesh`: a face of a file's body drawn within at most `FACE` = 2^27 evaluations (see below).
12. `forge-render`: a part with faces that cannot be meshed is drawn without them, not blank —
    **split out of this branch** at TRAIN-CONDUCTOR's request, onto `fix/undrawn-face` (off main),
    because the gap it leaves needs UX-LEAD's marking to ride with it (`ux/undrawn-faces`, stacked
    on it). `build_part_reporting` gives each left-out face as (face id in the part's body, why);
    the border edges are the face's loops' coedges' edges and the box is `body.face_box`.
13. `forge-step` `times` feature (off by default): each read phase's CPU time, for CHECKS.md.

## Open threads, in the order I would pick them up

**(a) One projection pass shared by the speck check and the edge tolerances.** CANCELLED by the
coordinator in the wind-down; not started. Both phases project edge samples onto each face the edge
bounds (speck: 8x `default_segments` per edge, chained round the loop; tolerances: 16 or 256 from
the edge's own start). Sampling each edge once at the finer count and reading both answers from one
pass would remove roughly one of the two, about a quarter of a checked read. It is not
bit-identical: the tolerances would come from other sample points, so their last bits move, and
every change in the number of surface evaluations moves which solids near the budget are refused.
High-rigor process (corpus differential, oracles, adversarial tests, second-agent review, old path
behind a feature for one train).

**(b) Move the speck guarantee to the predicates.** `laid_out_in_a_speck` is 20% of a checked
read's CPU, and all it does is guard the exact predicates' floating-point filters (a face laid out
over a span below 2^-433). If `forge_math::predicates` checked its own precondition in release and
fell back to exact arithmetic, the read's speck check would be unnecessary. That changes behaviour —
such solids are refused today — so it needs David's call, not just rigor. HELD by the coordinator.

**(c) The budget's unit.** This is the deepest thing I found and it is not mine to fix: a read's
budget counts surface evaluations, and a lazy cache's filling evaluations are paid by whichever
caller fills it first. So a solid's spend — and therefore whether it is refused as "too long to
measure" — depended on call order and on how work was spread over threads. KERNEL-STRESS has the
fix (`budget::uncounted`, approved by the coordinator: a lazy fill is charged to nobody) on
op-budget. Until it lands, my parallel phases are exact only away from the budget's edge; at the
edge, `map`/`map_until`'s prefix rule keeps the verdict one-by-one's for the items it takes, and
`map_counted` is exact always (the phases whose answers no budget changes). **When op-budget lands:
rebase `import/speed-ks` onto it, swap my per-item LEFT/LIMITED juggling in budget.rs onto
`isolated`, and rerun the corpus digest — plain and drawn.** That is the one piece of unfinished
business I would not skip.

**(d) Where the remaining time is.** Of the corpus's CPU: mass 31%, speck 20%, edge tolerances 17%,
mesh check 14%, heal 11%, topology 4%, display mesh 2%, parse 1%. Three quarters of every phase is
B-spline surface evaluation (point and derivatives inside a Newton projection), so the phases are
slow for one reason. The worst files are one check each: HAB3232 is 18.0 s of mass integrals on four
free-form faces of its 13.0 s wall, and the RPL UMC-750 cell is 15.1 s of mass and 11.4 s of speck.
A 1 s median needs the checks themselves to get cheaper — (a), (b), or a better quadrature — not
more cores; on 4 cores the parallel work is already spread.

## Risks I would watch

- **The per-face mesh budget's margin.** `FACE` = 2^28 (268 M) evaluations, which is also `MESH`,
  so the cap binds only inside a budget bigger than a lone drawing's — the app's 16x read, the case
  it is for. The heaviest face that is drawn anywhere is HFL-1313-3 #69702's #10 at 141 M (the
  fixture of `forge-step/tests/mesh_budget.rs`); the heaviest in the Haas corpus's own files is
  75 M of 415,000 faces measured at that 16x budget. So the headroom is about 1.9x.
  HRT160-2's #42930 spends 536 M and fails. **A first cap of 2^27 looked safe on the corpus and was
  not**: it would have refused HFL-1313-3's face, and only `mesh_budget.rs` caught it, because that
  fixture was not in my digest corpus list. The lesson for whoever touches this: the corpus
  differential is not enough on its own — build the file list from the repository as well
  (`find . -iname '*.step' -o ...`), and rerun it if the mesher's or the NURBS evaluation's work per
  face changes.
  A face that runs its own budget out ends the drawing and charges everything the drawing had left,
  so the solid is refused exactly as it is today when a face runs the budget out itself — that is
  what keeps the verdicts identical, and it is the part to re-check if anyone edits `face_within`.
- **Parallelism and the budget's edge**: see (c). Anything new that runs a budgeted phase on
  several threads must use the `budget::map*` helpers, not raw `par::map`, or verdicts can move with
  thread timing.
- **`Opened::finish` re-reads the file.** `forge_step::open` draws a model before its checks and
  `Opened::finish` then re-reads it from the text, because the early drawing fills caches whose
  first filler pays, which moved budget verdicts. It is only used where someone asks for the early
  view (`machine_model::set_show_early`, off by default), so it costs nothing today, but it is
  a double read and (c) is what would let it continue from the shown state instead.
- **The app's 16x budget.** `model_files::read` reads display models at 16x `Limits::max_evals`.
  That is why HRT160-2 could spend four minutes on one face: a bigger budget turns a file that is
  refused quickly into one that is read slowly. Worth a look across the corpus if the app ever feels
  slow on a file the kernel reads fast.
- **Measurement**: the `times` feature's own clock reads cost as much as the read until baa63336
  (they are read only while counting now). Any figure measured with that feature before that commit
  is roughly double.

## Where things live

- `research/import-speed/PROFILE.md` — where main's 11.2 s went, phase by phase and by instructions.
- `research/import-speed/RESULTS.md` — what changed, the numbers, and how identity was checked.
- `research/import-speed/CHECKS.md` — the four checks one by one: what each guards, its cost, and
  either a cheaper exact filter or the argument that there is none.
- `research/import-speed/CORPUS.md` — the frozen benchmark set (20 Haas files, tune / held-out).
- `research/import-speed/band-*.txt` — the speed gate's bands for the commits that were gated.
