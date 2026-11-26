# A8's builder on the old path's structure: the mapping (one page)

As of ed26629, face_fast.rs builds each near plane's facets and holders at 1.45M instructions per
flat boolean. That is the builder alone, on coplanar seq, with facets equal to the coarsening on
all 75 flat booleans.

The old path's flat boolean costs about 1.1M in total: plan_once_in's pair loop, then trim, then
split_face and classify. A8 must beat that in wall-clock at equal threads.

This page maps the old path's pieces onto the builder. Each piece is either reused as it is,
wrapped (the same structure, with its tolerance decisions replaced by the ledger's exact or
screened ones), or replaced.

## Reused as they are (structure and code)

| old path | where | in A8 |
|---|---|---|
| `classify::Index::new`, `face_box` | classify.rs:391–460 | Candidate pruning. Face boxes replace face_fast's class boxes and the per-line `touches` scan. |
| `near_b` (A's faces overlapping B's bbox) and the `fb` loop over overlapping face boxes | boolean.rs:1088–1099 | The pair loop. It produces the flat face pairs (fa, fb) that cut each other: different bodies only, which is the "bodies" statement in §3c. |
| `Scratch` (cuts, segs and pieces reused across booleans), bool-speed's S1–S3 (across_memo, packed sorts) | boolean.rs:791 | The arena. One scratch per thread holds segs, ps, the per-plane CSRs and the meets table, and is reset per boolean, not freed. |
| ordering by face name (`near_b.sort_unstable_by_key(name)`) | boolean.rs:1092 | A fixed pair order, so that ids and the merge order are a function of the input. |

## Wrapped: the same structure, with exact or screened decisions

| old path | its tolerance decision | A8's decision |
|---|---|---|
| `ssi::surfaces_in(sa, sb)` for two planes: a line, or Coincident | parallel or coincident within LINEAR | The class line C∩K, exact. Parallel = `d == 0`. Coincident = the same class. |
| `trim`: cut the curve to inside both faces, by parity on each face's edges | point-on-edge and touch within LINEAR, snapping | Per pair: points = the ends on the line, plus strict end-sign crossings (`crossing`). Then sort along the line (`along`), keep windows inside both faces (`mid_in` over the edge index), and cache by face (`inside cache`). **One trace per pair serves both planes.** Today face_fast computes C∩K for plane C and again for K. |
| the `coplanar` list (Coincident pairs) | coincidence within LINEAR | C faces of both bodies on one class. Each face's boundary edges cut the other's plane (`covered` and `bodies` already handle this). |
| `split_face` (pieces of a face along the cuts) and `classify` (piece inside/outside) | point-in-face with tolerance, near-vertex snapping | The per-plane graph: sub-edges, angle sort, cycles, holes, then holders by propagation. Holders replace classify. Inside/outside per room comes later from the witnesses and the core, not from the builder. |

## Replaced

- The tolerances: LINEAR snapping, `touches`, `undecided_touch`, `pinched`, `refuse_strays`. The ledger's single decisions replace them (keys, exact or screened signs). A touch becomes a refusal (`ZeroThickness*`) in the core, not a heuristic.
- Names: the write keeps bool-speed's names (unchanged).

## Estimate per phase (per flat boolean, coplanar)

| phase | face_fast today | after mapping |
|---|---|---|
| setup: point shadows, the edge index for faces > 16 edges | ~0.20M | ~0.10M (Index, and shadows only for faces in a pair) |
| pairs and traces, one per pair, both planes | ~0.76M (lines plus edge scans) | ~0.30M (pairs from face boxes, each trace once) |
| graph per plane | ~0.42M | ~0.20M (arena CSRs, one key sort per line, no Vec<Vec>) |
| whole-plane shortcut for planes no other-body face meets | 1.2 of 8 planes taken | ~3 of 8 taken (walls clip by one pass per face) |
| **builder total** | **1.45M** | **~0.6M** |

Threads: face pairs, and then planes, are independent. A parallel run merges in pair order, then
plane order, so the ledger is unchanged. It's worth it only past bool-research's ~1M break-even
per boolean at equal threads, so it's not planned for the builder alone.

## Provability (owner's rule)

Every reused or wrapped piece keeps its decision a function of exact data. Each step's statement
goes into face-arrangement.md §3c before the step is relied on, and the Lean coordinator reviews
it. The pair loop lives in forge-bool, outside the translated crate, so it is proven through what
it hands the core: the facets and holders, checked in debug against the coarsening on all 75.
