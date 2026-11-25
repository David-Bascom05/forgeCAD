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

- **HRT160-2 (the app's 16x read spends minutes on one face) is NOT fixed.** I wrote a per-face
  mesh cap and withdrew it: real faces spend up to 226 M (tl-3b_mld_07-2026 #52) and 141 M
  (HFL-1313-3 #69702 #10, `forge-step/tests/mesh_budget.rs`) at that budget, the bad face 536 M,
  so no cap has margin. My first try (2^27) refused both real faces — the app silently lost a solid
  of tl-3b — and was caught only by that test and by the app-path differential, not by the digest
  corpus, whose file list lacked the repository's newer fixtures. Lesson: build the differential's
  file list from the repository as well as the Haas archives, and always run the app's 16x path
  too. CHECKS.md has the numbers and the two routes I would try instead.
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

## Faces that fail to mesh (branch `import/faces`, off `import/speed-ks`)

David's direction: no caps and no faster refusals — find why faces fail and fix the cause. A sweep
of the 237 Haas machine STEP files (every solid read without its measures, meshed at display
quality; `examples/facesweep` in my scratch worktree, the same as `solidinfo` plus a mesh) finds 15
failing solids, 13 distinct. The mesher stops at a solid's first failing face, so rerun the sweep
after each fix. Four causes:

| cause | solids | where | state |
|---|---:|---|---|
| A. a loop's slit of no width (out and back along two edges lying on each other within the file's uncertainty) | 1 (HRT160-2 #13299) | reader | **fixed**: `heal.rs cancel_slits`, 52c2877c; unit test incl. a 1e-3 slit left alone; HRT160-2 now 5 solids, 0 refused, app read 211 s -> 0.24 s; corpus differential queued |
| B. a small-radius cylinder gridded to its radius (`interior_steps`: rows 2·r·du apart even when r ≪ chord: 50k–100k rows per fillet) | 10 (ST-30-35Y x2, GM-1-5AX, GM-1XT-5AX, DM-1, EC-1600ZT, HFL-3015D-6, HFL-6512T, HRP-2, cobot_pkg-7) | mesher | waiting on the coordinator's rigor ruling: the fix (no row spacing below the chord tolerance) changes every such fillet's mesh |
| C. an intersection-curve edge sampled one point per chain link at least (`edge_segments`: 2,000 links over 6 mm -> 4,845 boundary points on a 1 mm-chord plane; CDT recovery thrashes) | 1 (VF-5_40_SS_TR #252821) | mesher | same family and same ruling as B |
| D. a face whose surface the file puts 5 m from its edges (B-spline patch read correctly; edges 5,040 mm off it) | 1 (VS-3 #478011) | the file | **done** on `import/faces`: refused before healing with the true reason (`FAR_FACE_MM`/`FAR_FACE_PART`, `far_gap`); see below |

Tools: `tools/kernel-stress/extract.py FILE ENTITY OUT` takes one solid out of a file; then read it
with the measures skipped (a scratch patch of `finish`, env `FORGE_NOCHECK`) and mesh it per solid
to find its first failing face; log the interior grid sizes and per-stage spends in `tessellate.rs`
to see where the budget goes. Whatever is fixed needs the full differential with the repository's
fixtures and the app's 16x path, as above.

### D's differential

D refuses a solid when a face's edges lie farther than both 1 mm and 1% of the part's diagonal
from its surface. The quick measure (`measure_gaps`, `surface.distance`) can be fooled: a NURBS
nearest-point search may settle on a far sheet of the surface. So a face it calls far is measured
again (`far_gap`: a 64 x 64 parameter grid, then a projection from the nearest grid point), and
only that is believed (05c6e6d4). Corpus and fixtures: VS-3 #478011 (4.2 m; refused before, now
saying why), VS-3 #738262 (3.9 m) and Cobot #226368 (face #1170897, 3.0 mm, the same on a
600 x 600 grid: a thin strip with one edge's end 3 mm off it) newly refused; TRT100 #225539 (9.4 mm
by the quick measure alone) was the false alarm that led to `far_gap`, and reads as before.

The coordinator's condition for landing D: a solid it refuses must not vanish from the app. So
`forge_step::draw_far_refused` (f5171b10) re-reads, for a viewer only, the solids D refused: each
healed without the far face, its edges' tolerances recorded, drawn under what healing left of the
read's budget. `import` is untouched. `model_files::read` draws them, and `read_noting` gives
attach() the note "<part> is drawn without face #N, which can't be read: ...". Shots in
research/import-speed/far-face/. Two traps: drawing the far face itself (VS-3 #478011) takes
minutes and fails, and so does drawing without the edges' tolerances (the mesher meets an edge off
its faces only by them). VS-3 #478011 is still not drawn, as before D, in 7.4 s.

### STEP's Unicode escapes are never decoded (its own change, agreed with the coordinator)

`part21::File::str_of` returns a string's bytes as written, so `\X2\00d600e1...\X0\` (UTF-16),
`\X4\` (UTF-32), `\X\hh` (latin-1) and `\S\c` (the upper half) show raw in every name the app
takes from a file: piece names, the Split list, the drawn-without note (the Cobot package's
"HC5 Joint 5 / 14\X2\00d600e1...\X0\"). A decoder at the parser is small, but it changes piece
names and so the app digests of every file whose names use it: its own differential, not done.

### Gap rule tier 2 (not written yet)

Agreed with KERNEL-STRESS and the coordinator: one gap rule, held on `import/faces`. Tier 1 is
done (a face whose edges lie farther than both 1 mm and 1% of the part from its surface refuses
the solid; wording UX-LEAD-approved, test asserts it). Tier 2 keeps the solid but warns, for a gap
over k x the file's stated uncertainty (and over LINEAR): ds-40y #249171's edge #57090 lies 0.96 mm
off face #5840 where the file claims 1e-5 (evidence: tools/kernel-stress/edge_fit.py on
import-sheets). Plan: Proto `edge_ids` like `face_ids`; measure_gaps reports the worst
(edge, face, gap); choose k from the corpus's distribution of worst gap / uncertainty, so that
accepted solids don't fill with warnings; join it with measures_agree's own `unverified`
words (the gap first). Wording, UX-LEAD verbatim (distances through `far()`): "edge #E lies 0.96 mm
off face #F, though the file claims 0.00001 mm accuracy: that face may look wrong and isn't checked
there (exporting the file again may help)" -- drop "and isn't checked there" if the solid's checks
still run on that face.
