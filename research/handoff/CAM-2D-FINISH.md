# CAM-2D-FINISH handoff

## Branches

| Branch | Sha | State |
|---|---|---|
| cam/2d-finish-2 | d9526bae | Merged to main. It holds leads, tabs, helical bore, the language ARGS and the COVERAGE rows. |
| cam/post-comp-2 | b0fbbdd2 | Merged in substance: train 5 squashed it in, and main's forge-post comp has moved past it since. Do not merge it again. |
| cam/crazy-2d | this branch | Fixes the manifold crazy ops. Waiting on pre-verify and TRAIN-CONDUCTOR. |

## cam/crazy-2d: what it does

The three manifold crazy ops are fixed, and their KNOWN entries are gone from `verify/forge-cam-quality/src/crazy.rs`.

| Op | Before | After |
|---|---|---|
| crn | air 0.995, links 2963 | air 0.58, links 114, retracts 8 |
| shal | air 0.76 | air 0.33 |
| grv | re-cut 0.33 | re-cut 0 |

11 of 14 ops are certified, the same as before.

The changes:

- **Rest machining by plain morphology** (lang `rest_region`):
  - Tool centres are C = (rg ⊖ r) ∩ (M ⊕ r).
  - M ⊕ r is bounded by stock ⊕ r minus each void ⊖ (r − 1e-3).
  - When offsetting hits a cusp spike, it falls back to grow(inside), then to boxes.
  - `edge_only` means (rg ⊖ 2r) ∩ M is empty. Such a piece is cut by one pass along the wall, in `forge-cam/src/rest.rs` `rest_pass`.
- **Stay-down linking in `ops::pocket_in`.** With one loop and no ramp, the tool goes straight down to the next level instead of retracting.
- **Posted-path staging.** It now also stages at a Plunge that has nothing above it. Without this, stay-down was falsely refused for the shank.
- **Engrave and slot ramp.** It always has an even number of legs, which removes the return-to-start that re-cut the groove.

## Risks

- `rest_region`'s fallbacks (`boxes_of`, grow) are supersets. They stay safe because certification still checks every move, but on odd geometry they can cut extra air.
- `chord_inside` in rest.rs is a sampled polygon test. Certification is what actually guards it.
- shal: CAM-CORE plans per-level sections, starting with the well's 6 mm column over the island. Once those land, check shal's air again.

## Next (not started; out of wind-down scope)

- The CAM side of cutter comp: `comp: computer|wear|control` and `comp_range`, keyed on main's comp_corner and comp_entry.
  - A lead arc that curves toward the comp side needs a radius larger than the largest register value.
  - Comp turns on and off on straight G1 moves in XY.
- Deferred:
  - a same-block comp template, waiting for a corner cite;
  - vertical leads;
  - trace on model edges.
