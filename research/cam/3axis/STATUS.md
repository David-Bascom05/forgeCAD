# CAM-3AXIS status (cam/3axis, 2026-11-24)

Main merged (#15/#16 in). Details, measurements and open items: research/cam/phase3.md.

## Done
- Drop-cutter, certified: kernel/forge-cam-geom/src/drop.rs. Cutters: flat, ball, bull, chamfer and
  drill (any revolved profile `Column::of` accepts). Faces:
  - closed form: planes (any tilt), vertical walls, spheres, horizontal cylinders and tori about a
    vertical axis, each dome or bowl;
  - interval branch and bound: everything else (tilted tori, cones, tilted cylinders).
- Gap check of a move (`gap_move`): certified, refuse-only.
- Ops in kernel/forge-cam/src/surf.rs, wired into the machining document (forge-cam-lang).
- examples/machined/mold_core_cam.fg: 5 ops, 15.4k moves, every move certified, G1–G6, 34 s.
- Lean: verify/forge-cam/ForgeCam/Drop.lean, 10 theorems.

## API (stable)
- `drop::Column::of(&Body) -> Option<Column>`: the cutter's column.
  - `.grow(δ)`: the leave.
  - `.widened(ε)`, `.trimmed(r)`, `.convex()`.
  - `.b_at(ρ) -> Ival`.
- `drop::drop(col, faces, idx: &[u32], x, y, res, budget) -> Cl { lo, hi, cells }`:
  lo ≤ CL ≤ hi, hi − lo ≤ res unless the budget runs out. hi = −∞ when no face is within reach.
- `drop::drop_above(.., floor, ..)`: the same, resolved only above `floor`.
- `drop::gap_move(col, face, &Move, limit ≤ 0, res, budget) -> (Gap, cells)`, where `Gap` is one of:
  - `Clear { lower }`
  - `Over { s, by, p, q }`
  - `Unknown`
- `surf::Surfacer::new(faces, &cutter_body, leave, tol, floor)`:
  - `.cl(x, y) -> Cl`: clamped to the floor; uses a grid of faces in xy.
  - `.gap(a, b, limit) -> Gap`: over every face near the move.
  - `.drop_polyline(&[[f64; 2]], step) -> Result<Vec<[f64; 3]>, SurfError>`.
- `surf::ClGrid::new(&sf, area, h)`: `hi` and `lo` per node. `surf::push(sf, a, b, z, step)`.
- Ops, each `-> Result<OpPath, OpError>`:
  - `surf::parallel(sf, cut, area, stepover, angle, step)`
  - `surf::waterline(.., zs, h)`
  - `surf::zlevel` / `zlevel_voids`
  - `surf::pencil`, `surf::rest`, `surf::scallop`
  - `surf::ZMap`
- Lowering: `forge_cam::lower3d::lower_surfacing(&Body)`.
- Checker: `ops::certify_with(.., column: bool)`.
- For CAM-3D-FINISH's `DropCutter` trait: `Band { lo, hi }` = `Surfacer::cl(x, y)`.
  - "None" maps to hi = −∞ before the floor clamp (`drop_above` with floor −∞).

## Document ops → Fusion strategies
| ours | Fusion |
|---|---|
| zlevel | adaptive's place (offset clearing, not adaptive) |
| parallel | parallel |
| waterline | contour |
| pencil | pencil |
| rest | rest |
| scallop (grid geodesic) | scallop |

## Time shares (mould core, parallel, 2.8k moves, 4 cores)
- Generation's own gap self-check: ≈ 80% of generation CPU, of 0.13 s wall.
- Drops: ≈ 15%.
- Certifying the emitted path: ≈ 0.1 s per op.
- Whole document, 34 s:
  - zlevel 13.7 s, mostly the exact offsets of staircase regions;
  - every other op under 8 s.
