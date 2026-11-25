# CAM-3D-FINISH handoff

## What I was working on

Rest finishing (`rest(..)`, forge-cam-3d `rest.rs`) on `examples/machined/mold_core_cam.fg`'s op `left`.
The crazy-path guards had flagged it: 76 % of its cutting length was in air, with 35 retracts for 85 mm
of cut.

**Cause.** The region was where the earlier tool (`after:`, flat-10) could not reach in principle.
The ball-6's own parallel, waterline and pencil ops had already finished nearly all of that region. On
top of that, each patch was grown by the 3 mm default margin into finished surface.

**Fix** (branch `cam/rest-air`):

- `rest::region_in`: the rest region also requires the stock to stand more than `min` above what this
  tool leaves. The stock is modelled from the setup's earlier built ops: their paths are swept with
  each tool's profile (`HeightMap::sweep_path`) over the stock's top less its voids, on cells centred
  on the region's grid nodes.
- `rest::trim`: each pass is cut to its run from the first to the last segment that meets that model.
  The model is lowered as the passes cut it.
- The memo key for `rest` carries the stock's bottom, top and voids and the earlier paths. Those are
  what the model reads. The stock's sides are not read, so `finish3d_memo_tests`' "stock beside the
  part" case still comes from the memo.
- `forge_cam_quality::crazy::KNOWN`: mold_core_cam/left's air and retracts entries are removed.

**Result on `left`:**

| | Before | After |
|---|---|---|
| Cut | 85.3 mm | 0.9 mm |
| Air at feed | 267 mm | 8.5 mm |
| Retracts | 35 | 11 |
| Links | 214 mm | 128 mm |
| t_min | 129.9 s | 42.7 s |

The other ops are unchanged. Certification is unchanged ("full except G4 (leftover not computed)"),
so there is no gouge, and the program round-trips.

## State

- Every earlier branch of mine is in main by content (trains squash):
  - cam/arc-frame
  - cam/drape-budget (GAP_BUDGET 1000)
  - cam/3d-hops
  - cam/3d-fit-perf
  - cam/3axis-fit-2
  - cam/speed-ledger (rows)
  - the mould fuzz script
- `cam/rest-air` is the one unmerged branch: cam/crazy-guards @8faf6aa1 (train 8's base for
  TOOLPATH-QUALITY) plus the commits above.

## Open item for whoever lands second

CAM-SPEED's `cam/speed` adds `Cam::prefetch`, which runs each 3D op's generator ahead of the normal
run. `rest`'s key now holds earlier ops' results. A prefetched rest therefore sees no earlier ops: it
is stored under a key the real run never asks for. That is wasted work, not wrong output.

Agreed with CAM-SPEED: **whichever of cam/speed and cam/rest-air lands second adds the skip** while
merging main. In the prefetch, record a job only when `kind != "rest"`, with a comment that rest's key
holds earlier cuts.

## Risks

- The stock model reads cell centres (`sweep`, not `sweep_under`), so it can understate a sliver
  thinner than a cell (0.1 mm or finer). Rest may then skip a cusp of about that size. It is used only
  to choose where to cut. Every kept move is draped and certified as before.
- `rest` now costs a sweep of the setup's earlier paths. That is milliseconds on mold_core: roughly the
  measured length over 0.1 mm cells.
- A `rest` in a setup whose earlier ops are refused models only the built ones. That overstates the
  stock, which makes the region a superset: safe, and possibly some air.
