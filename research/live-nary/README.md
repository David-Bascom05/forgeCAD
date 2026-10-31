# N-ary regen for live drags: tried, dropped

A live drag re-runs the part sequentially from the first changed feature (`Model::rebuild`). The
idea was to use the n-ary regen, with threads, when many features must re-run. Dropped 2026-10-31:
David decided drags stay in the kernel with no preview for now, and n-ary could only be a preview.

## Why it could only be a preview

- The n-ary regen leaves one final result, no per-line stages, and re-runs the whole history. Prefix
  reuse is lost, and a drag re-running few lines gets slower (conrod `bolt.4` re-runs 5 of 24).
- Its names differ from sequential on 4 of the 5 parts measured, on every value (frame up to 66 faces,
  gearbox 102, crank 14, conrod 80; only manifold agrees). So faces re-mesh when the drag starts, and
  release needs a sequential rebuild to restore stages and names.
- Selectors must be frozen at their pre-drag picks, since nothing is built to run them on. A pick that
  should change mid-drag shows only at commit.

## Measured

30 values, a 2% swing, same machine, load average 40-45. Times in ms. At that load wall-clock cannot
show a thread gain, so p50 and p99 are mostly noise; `best` is the fastest of the 30. It was not
repeated on a quiet machine, and sequential `best` was not measured (the first value was a no-op).

```
                  seq p50/p99   nary1 best/p50/p99   nary2 best/p50/p99   nary4 best/p50/p99   lines rerun
frame reach       255/1113      204/237/622          158/241/648          124/277/868          41 of 147
gearbox W         214/496       128/208/473          135/223/860          166/287/554          44 of 57
manifold pitch    201/1167      139/238/844          127/209/973          128/202/1172         17 of 27
crank len         136/327        86/131/277           82/165/586           91/199/745          14 of 18
conrod bolt.4     360/756        12/285/1175          43/299/1564          24/314/2274          5 of 24
```

Over all 150 values at 3 thread counts: no failures, no volume differences, and the feature lists
matched sequential with selectors frozen. No line needed geometry to parse. Only frame showed a gain
from threads; none came near 16 ms.

How: parse every line with each selector taking the names it last resolved to (`own` as identity, a
line needing a face's plane ending the attempt), then `Part::regenerate_nary_solids_ids` on 1, 2 and
4 threads, compared with the sequential `Part::drag` at the same value.
