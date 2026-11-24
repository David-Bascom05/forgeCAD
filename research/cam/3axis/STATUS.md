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
- examples/machined/mold_core_cam.fg: 5 ops, 15.4k moves, every move certified, 6.1 s (17.5 s
  before the dome cylinder's exact form, the merged voids and the deciding push; the posted
  program unchanged by the last two; G4 not computed for 3D parts).
- A dome cylinder's move is bounded exactly (`cyl_chord`, Concave.lean): the generators'
  self-check on the mould core 6.06 s → 0.79 s CPU. The check is not dropped: a face entering the
  rim between two drops is invisible to the ends (phase3.md).
- Lean: verify/forge-cam/ForgeCam/Drop.lean (10 theorems) and Concave.lean (12).

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
- Generation's own gap self-check: was ≈ 40% of generation CPU (the dome cylinders' search);
  now ≈ 6 ms of it (exact form).
- Drops: ≈ 15%.
- Certifying the emitted path: ≈ 0.1 s per op.
- Whole document, 6.1 s wall (was 34 s, 17.5 s after main's merge, 13.2 s):
  - waterline 2.1 s, rest 1.1 s, parallel 0.5 s, zlevel 0.45 s;
  - the in-process stock's faces 3 × 0.52 s (stock.rs, built once per later op: a cache across
    ops would save 1 s; CAM-CORE's code);
  - certificates 0.13 s in all.
