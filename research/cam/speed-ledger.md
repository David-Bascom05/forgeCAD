# CAM speed ledger

One place for every CAM change made for speed. Started 2026-11-24, as the CAM-wide convention.

**The rule.** A CAM speed change that moves any output must come with a row here, in the same PR:
- paths, blocks, cycle time, cells, or the certificate's numbers;
- a row in **Changes that move outputs** saying what moved and why it stays sound.

A change that keeps every output byte-identical goes in **Identical output**, with how that was
checked. TRAIN-CONDUCTOR expects a row in any CAM speed PR that moves outputs.

**A row says:**
- **what**: the change and where;
- **moves**: which outputs move, and by how much on the parts measured;
- **sound**: why every path is still proven and certified;
- **pinned by**: the test that would fail if the change went wrong.

"Mould" is examples/machined/mold_core_cam.fg and mold_core_finish_cam.fg. "Fuzz" is
`tools/reliability/mould_fuzz.sh`: both moulds, each part parameter of mold_core.fg at -5..+5 %,
84 runs.

## Changes that move outputs

### SurfDrop's drape gap budget: 2000 -> 1000 cells (CAM-3D-FINISH)

- **What.** `kernel/forge-cam-3d/src/surfdrop.rs` `GAP_BUDGET`: the cells one face's gap
  search may spend when the drape (and links, and fitted pieces) test a chord. Branch
  `cam/drape-budget` (6ef05bc9), in `cam/3d-hops`.
- **Moves.** Measured on the mould, by budget:

  | budget | steep_shallow gen | rest gen | steep_shallow cycle (plan_scurve) | steep_shallow blocks | rest blocks |
  |---|---|---|---|---|---|
  | 2000 | 8.50 s | 2.83 s | 1975.2 s | 9956 | 885 |
  | **1000** | **6.49 s** | **2.41 s** | **1975.2 s** | **9972** | **889** |
  | 500 | 5.60 s | 2.03 s | 1977.2 s | 9990 | 889 |
  | 250 | 4.86 s | 2.13 s | 1984.7 s | 10026 | 889 |

  At 1000:
  - every op's cycle time is unchanged;
  - the program totals are 2329.6 s and 3975.3 s;
  - every op certifies;
  - the fuzz refuses only the 4 cases it refuses at 2000.

  500 and below begin to cost cycle time.
- **Sound.** The search's verdict for the generator is "Clear" or "not proven". A chord not
  proven within the budget is split, and its pieces are tested again (short pieces resolve by the
  closed forms). So the budget moves only where splits happen, never what is accepted: an
  accepted chord was proven Clear by the same search, at any budget. Every posted path is then
  certified by forge-cam's checker independently.
- **Pinned by.** `forge-cam-3d` drape and strategy tests (every chord proven), the mould
  certification tests in `forge-cam-lang/tests/cam.rs`, and the fuzz.

### Hops on the 3D ops' links, and the way up at cut feed (CAM-3D-FINISH, with TOOLPATH-QUALITY)

- **What.**
  - `hop_links` (forge-cam-lang) runs `forge_cam::hop::hops` on parallel, waterline, zlevel and
    the finish3d ops too.
  - The stock map is carried from op to op within a setup.
  - An entry's landing is judged by the tool's profile (hop.rs `entry_top`).
  - In forge-cam-3d (link.rs, path.rs) a link's way straight up runs at the cut feed, not the
    plunge feed.
  - Branch `cam/3d-hops`.
- **Moves.** The mould's full program, by plan_scurve, went 2835.2 -> 2329.5 s:
  - rough 1758.2 -> 1329.0 s;
  - walls 206.8 -> 154.5 s;
  - left 156.3 -> 132.7 s;
  - corner 58.0 -> 57.4 s;
  - fin unchanged.

  The 2.5D examples are byte-identical.
- **Sound.**
  - A hop keeps the tip CLEAR (0.5 mm) above the material its map holds within r + 2h at every
    sample.
  - An entry's landing is CLEAR above where the tool's profile first meets a map cell. The
    distance is read as d - 2h, to cover the sampling.
  - The map only ever holds at least the true stock.
  - Rising straight up from a certified position meets no material: the stock is a height
    field, and every body of the tool is already clear of it where it stands.
  - Every hop is posted with `(HOP n)` and checked against the plan on read-back, and certified
    as a link.
- **Pinned by.** hop.rs's tests, including:
  - `a_ball_lands_over_a_slope_by_its_profile_and_into_no_cell`;
  - `a_spike_by_the_rim_is_kept_off_by_the_sampling_margin`, which fails without the 2h margin;
  - `a_flat_end_lands_where_the_disk_does`.

  Also the `hop_label` read-back refusal, and the mould certification tests.

TOOLPATH-QUALITY: please add the 2.5D hops and the clear corner blends here. CAM-PERF: please add
the adaptive caches if any of them moves an output.

## Identical output

Each was checked byte-identical (the reply's JSON and the posted programs) on all 14 machining
examples against the build before it.

| change | where | gain | by |
|---|---|---|---|
| `gap_clear`: the gap search stops once "Clear" is impossible (a cell too small to split, or the cells over the limit cannot all be split within the budget) | forge-cam-geom drop.rs | mould cold 7.02 -> 6.40 s wall | CAM-3D-FINISH; test `gap_clear_is_gap_moves_clear` (each search's own spend as budget; fails with the bound off by one split) |
| In `gap_clear` mode a cell's bound stops at the first of its terms at or below the limit (before sd_rz and the dual-number form) | drop.rs `cell_upper_side` | mould 6.45 -> 5.69 s wall | CAM-3D-FINISH; same test (fails with the stop 1e-3 above the limit) |
| The drape's split point reuses the midpoint drop already made | forge-cam-3d drape.rs `refine` | about 1 % | CAM-3D-FINISH |
| `TrimRef::classify` returns at the first trim piece the box may touch | forge-cam-geom check.rs | with the memo below: the trim test was 36 % of the rest op's instructions | CAM-PERF (b021cf5c) |
| `Face::holds` memoised per face by the box's bits | check.rs | as above | CAM-PERF (4ef931ed, 0f171d3e) |
