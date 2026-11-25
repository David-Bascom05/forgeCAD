# forge-asm check: handoff from HOSTED-DESIGN to ASM-CHECK

HOSTED-DESIGN did the overlap-check speed work (P1–P6) and the query-interference abort fix,
then went back to hosted work. This file is the state, the numbers and where everything is.

## Landing (in progress when written)

| branch | what | state |
|---|---|---|
| perf/asm-overlap-walks | P1+P3, P4, P2 (forge-asm `check.rs`, one line in forge-lang `geomemo.rs`) | corpus 164/164 identical; awaiting the second-agent review the coordinator arranges, then pre-verify and a train |
| fix/query-interference-refusal | `query interference` no longer aborts on one refused boolean (forge-lang `query.rs`) | pre-verify running |

The old walks stay behind the cargo feature `forge-asm/two-walks` for one train (comparison
builds only: `cargo build --release -p forge-cli -p forge-asm --features forge-asm/two-walks`).
**Removing it after that train is yours**: delete `two_walks()`, `narrow` (keep `enclosed`),
`near_surface`'s callers in `enclosed`, `Bvh::new_sorted` + `SELECT_SPLIT` (keep the test's
reference build in the test), and the old-path arms in `interference`. `distance` stays
(clearances use it).

## What changed and why it is exact

- **P1+P3 `narrow_near`**: one walk over the node pairs that either narrow's crossing walk or
  distance_kept's walk would visit. Each test carries its own prune flag down the pair tree, in
  its own frame (narrow puts the smaller mesh in the larger's frame; distance puts B in A's).
  So each test sees exactly its old leaf pairs and coordinates. Neither answer depends on
  visiting order: any crossing; is_some, which a shrinking-reach walk answers like a fixed one.
  `Lazy` places positions and node boxes when first reached (the same `point_to_world` and
  `moved_box` bits). NaN prunes as before (`unpruned`).
- **Memo**: NARROWS keeps (verdict, near). Learned codes 3 (clear, near) and 4 (clear, far) are
  new; old readers skip them. Code 0 (old caches) falls back to the kept gap. The geomemo merge
  prefers the depth-knowing record.
- **P4 `Bvh::new`**: `select_nth_unstable_by` on the same total order, leaves sorted in their
  parent's order. It builds the same tree (`bvh_select_tests`). Mesh BVH builds: 880 M → 512 M Ir.
- **P2 `near_all`**: `enclosed` asked `near_surface` once per sampled triangle. Now one walk of
  the tested mesh's tree against the other mesh's tree, its boxes refitted bottom-up to the
  sampled triangles' exact boxes (no moved-box rounding). Same leaf expression, and box_gap is
  monotone, so the answers are bit-identical (`near_all_answers_as_near_surface`). A first
  version built a fresh BVH per call and cost 3.9 G of tree builds; the refit costs O(nodes).

## Numbers (callgrind Ir, x86-64-v2 build, FORGECAD_CACHE=off, whole `build test.fg --json`)

rack18u/frame/test.fg: main 29.42 G → P1+P3 26.11 G → +P4+P2 21.19 G (−28%).
narrow+near: 25.07 G → 17.21 G. enclosed: 11.06 G → 6.43 G.
What is left in narrow_near: tri_distance 4.9 G (now mostly the near test's and near_all's
leaf tests), enclosed's parity rays (`inside`), crosses/pierces.

Valgrind cannot decode some x86-64-v3 instructions ("Illegal instruction", truncated counts):
build with `CARGO_TARGET_DIR=/var/tmp/tgt-v2 RUSTFLAGS="-C target-cpu=x86-64-v2"`.

Earlier profile on main (before P1–P4; CURVED-SITES tip 39252dd7):
- frame/test (2.4 s overlap, 300 exact runs, 0 interferences): narrow 57% (crosses/pierces 20%,
  near_surface 26%, Placed::new+moved_box 11%, BVH build ~3%), distance_kept 28%, exact
  booleans 5%.
- rack.fg (9.8 s overlap, 294 G Ir): distance_kept 28%, narrow 28%, exact 1.8%, the expects'
  sub-assembly runs 12%.

Wall times for frame/test and rack are still to be measured once a quiet CPU is free.
HOSTED-DESIGN sends them to the coordinator if it gets to them first.

## Open items (yours)

- **P6: the expects' sub-assembly runs (12% of rack).** UNKNOWN what they recompute. Look at
  forge-lang `asm.rs` `measure` ("interferences" calls `checker.interference` again, ~line
  5141) and how sub-assembly docs are re-run per expect. A memo keyed by (doc text, at-params)
  may cover it. Measure first.
- **P5 (convex-hull pair skipping): measured, not worth it.** 47,963 pair tests over the
  example assemblies: hull-prunable pairs are 5,426, but cost 3.6 s of 392.9 s (0.9%), and the
  hull test itself costs 2.0 s. Prunable pairs are already cheap, since the BVH walk drops them
  near the root. Data: `research/asm-speed/p5_hull_instrument.patch` + `p5_run.py`.
- **The query has no mesh stage.** `query interference` runs an exact clash on every
  box-overlapping pair. After the fix, a refused pair is listed unconfirmed (boxes overlap,
  boolean refused) rather than mesh-flagged. A narrow-phase gate (forge-asm's meshes) would
  drop box-only false positives and save the booleans. On frame/test the query takes 4m10s:
  60 pairs, 44 unconfirmed, almost all pan_nut × screw `Internal(RetryInvalid)` refusals.
  Those refusals are CURVED-SITES'.
- After the refusal fix, these docs answer `query interference` where they used to fail
  outright: post_kit, hanger, post_pan, latch_base_stud, caster_plain, sub_fits, sub_drive.

## Scripts and data

- `research/asm-speed/corpus_equal.py BIN_A ENV_A BIN_B ENV_B OUT [JOBS]`: the 164-build
  corpus (48 example assemblies; each of the first four `shared` numbers ×1.05; the two thread
  repros at 6.3). Compares full `build --json` with timings stripped. Runs so far: two-walk vs
  one-walk 164/164; main c135b274 vs P1+P3+P4 164/164; main vs P1–P4 (see NOTES.md).
- `research/asm-speed/NOTES.md`: the measurements above in more detail.
- On design/hosted: `research/hosted/bench/overlap_corpus.py` (per-doc overlap ms, exact runs,
  interferences; data in `research/hosted/data/overlap-corpus-*.jsonl`),
  `research/hosted/proto/src/bin/warmrepro.rs` (edit-then-rerun warm/cold timing),
  `valcost.rs`.
- Regression tests for the thread-overlap path: `interface/forge-lang/tests/asm_thread_repros.rs`
  (from test/overlap-repros, merged via CURVED-SITES).
