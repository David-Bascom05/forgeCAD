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
  - The map, read within r + 2h, bounds the true stock: it starts from the in-process stock and
    is lowered only by the moves swept into it.
  - Rising straight up from a certified position meets no material: the stock is a height
    field, and every body of the tool is already clear of it where it stands.
  - Every hop is posted with `(HOP n)` and checked against the plan on read-back, and certified
    as a link.
- **Pinned by.** hop.rs's tests, including:
  - `a_ball_lands_over_a_slope_by_its_profile_and_into_no_cell`;
  - `a_spike_by_the_rim_is_kept_off_by_the_sampling_margin`, which fails without the 2h margin;
  - `a_flat_end_lands_where_the_disk_does`.

  Also the `hop_label` read-back refusal, and the mould certification tests.

### Hops on the 2.5D ops' links (TOOLPATH-QUALITY)

- **What.** `kernel/forge-cam/src/hop.rs` and `hop_links` in forge-cam-lang, for clear, contour
  and adaptive:
  - A retract between two cuts (up, across at the retract height, down) becomes one up-over-down
    run of G1 at the machine's feed limit, as low as the material it crosses allows.
  - Phase `hop`; posted after `(HOP n)`.
  - A run ending above a plunge hops down to just over the material there.
  - A landing with material beside it comes straight down the column the retract came down.
  - Commits on `cam/adaptive-only`: ce72fdeb, 1818a6eb (on by default), 67beb20b (label check),
    27a93e25 (entries and beside-wall landings).
- **Moves.** Rough op, steel, equal volume, every op certified; plan_scurve over four (A, J, δ)
  sets, G0 blended / stopped at every G0:

  | part | op | rapids before → after | time saved, blended | time saved, stopped |
  |---|---|---|---|---|
  | deep_islands | adaptive | 397 → 69 | 5.2–7.0 % | 7.0–9.5 % |
  | deep_boss | adaptive | 157 → 11 | 5.7–7.7 % | 7.7–10.7 % |
  | deep_open_slot | adaptive | 19 → 8 | 0.9–1.4 % | 1.4–2.3 % |
  | deep_islands | clear | 12 → 5 | 1.1–1.4 % | 1.2–1.4 % |
  | deep_boss | clear | 8 → 4 | 0.8 % | 0.8–0.9 % |
  | deep_open_slot | clear | 21 → 5 | 3.9–5.6 % | 4.1–5.9 % |

  No case is slower. examples/machined re-posted.
- **Sound.**
  - **Clearance.** The tip stays 0.5 mm over the highest material within r + 2h at every sample,
    a map cell apart. A point between samples is within h/2 of one, and a cell's centre is within
    h/√2 of any point of it.
  - **Bodies.** Every body above the cutter (shank, holder) clears the highest material in the
    box round the way, widened by its radius.
  - **Ramps and landings.**
    - An end with material above its level within the band climbs or descends straight, as the
      retract did.
    - A landing's own column (r + h) must be clear.
  - **Never higher** than the retract it replaces.
  - **The map.** It starts from the setup's in-process stock (the uncut box if an earlier op
    didn't build), and is swept by this op's moves.
  - **Certificate.**
    - Hops are certified as links.
    - A `(HOP n)` label is believed only where its blocks are the planned hop's own; otherwise
      the op is refused (`hop_label`).
    - The staging treats a hop as ending a level, as the retract did.
  - **Feed.** The feed scheduler reads a hop as one sample: the chip-rule feed if it meets
    material.
- **Pinned by.**
  - hop.rs tests:
    - `a_retract_over_material_becomes_one_hop_clear_of_it`;
    - `a_cut_ending_against_a_wall_climbs_straight_up`;
    - `a_retract_onto_an_entry_hops_down_to_just_over_the_material`;
    - `a_landing_beside_a_wall_comes_straight_down_the_retracts_column`, which also checks that
      a slanted original gives no hop;
    - `a_retract_that_enters_material_keeps_its_descent`.
  - `hop_label_tests`: a label on a cut, past a hop, or over a rapid is refused.
  - `tests/peck.rs`: staging, holder.
  - The guards: chip load, equal volume, micro-move ratchet.

### Facing: the turn between passes a half circle in the air (TOOLPATH-QUALITY)

- **What.** `ops::facing`: the stepover between zigzag passes, two right-angle corners beyond the
  stock, becomes one arc of the stepover's diameter, bulging outward (b40454af).
- **Moves.**
  - Facing ops 0.1–2.5 % faster (plan_scurve set 1, stop at every G0).
  - Facing's accel/decel loss over the harness: 17.9 → 6.6 s.
  - Volumes identical.
  - pinned.txt's no-acceleration bounds rise as they must, since the arc is π/2 × the stepover:
    well_island skim t_stop 36.0 → 36.3 s, deep_slot skim 65.7 → 66.0 s.
- **Sound.**
  - The arc lies further from the stock than the pass's end, which is d/2 + 1 mm past it, so it
    cuts nothing.
  - The passes are unchanged.
  - It is certified as any feed move.
- **Pinned by.** `tests/pinned.rs` (facing ops of well_island and deep_slot) and the guards'
  equal-volume check.

### Clear: the roughing rings' convex corners blended (TOOLPATH-QUALITY)

- **What.** `ops::blend_corners`, called by `pocket_in(.., blend: true)` (clear only; pocket
  passes false), 3be00883. From the second ring in, each outer loop L becomes its opening
  (L ⊖ ρ) ⊕ ρ, which rounds exactly its convex corners to radius ρ. ρ is the largest of r, r/2,
  r/4 that passes the coverage test (Sound).
- **Moves.** Every clear op in the harness, plan_scurve set 1, stop at every G0:

  | op | change |
  |---|---|
  | plate_break side | −9.5 % |
  | flange top | −8.1 % |
  | bracket side / rest | −6.6 % / −4.6 % |
  | boss rough | −5.5 % |
  | open_step | −3.5 % |
  | deep_islands rough | −3.1 % (aluminium −3.7 %) |
  | deep_boss rough | −2.8 % (aluminium −3.3 %) |
  | three others | 0.0 % |
  | **all clear ops** | **2633.8 → 2542.3 s (−3.5 %)** |

  - None is slower.
  - Volume, peak chip load and certification are identical.
  - t_min falls too (flange top 86.8 → 80.7 s): with no sharp corner the engagement falls there,
    and the feed scheduler holds F.
  - Not applied to pocket: there the outer ring is the wall pass, cut last, and its picking up
    the corners' leftovers cost pocket_islands +8.5 %.
- **Sound.** A blend is kept only if L ⊕ r ⊆ (L′ ⊕ r) ∪ (the band the ring outside it sweeps,
  as kept), tested with exact booleans.
  - The inner edge is unchanged: ((L ⊖ ρ) ⊕ ρ) ⊖ r = L ⊖ r for ρ ≤ r.
  - The outermost ring stays sharp.
  - By induction from it, the rings together sweep all they swept before. So the opening, and
    what clear claims cleared, are unchanged.
  - A corner no ρ passes for stays sharp.
- **Pinned by.**
  - The guards' equal-volume and chip-load checks over every benchmark part.
  - The certificate's G1–G3 over the posted paths.
  - The harness certification sweep (identical on all 28 parts).


## Temporary safety exceptions

Speed given up on purpose, for safety, until a named condition is met. Each row says what was
switched off, what it costs, and what turns it back on.

### The threaded page's per-thread block caches off (CAM-PERF), from 2026-11-24

- **What.** `kernel/forge-alloc/src/lib.rs` `THREAD_CACHE = false` (branch `alloc/cache-off`): in
  the threaded browser build every small block goes through the eight shared heaps again, as
  before 34d6cad8. The heaps' quick look first (c0c6143f) stays on.
- **Why.** A threaded page's worker died with "memory access out of bounds" in
  `GlobalAlloc::realloc` (CI #56, parity shard 1/3, two workers, the same stack, building
  examples/rack18u/asm/rack.fg). Not reproduced locally (about 14 runs on 1, 2 and 4 CPUs, a
  256 KiB worker stack, cache on and off), nor by the native threaded stress (`alloc/threaded-stress`).
- **Costs.** About 20 ms of a manifold rerun in the threaded page (worker median about 104 ms with
  the caches, about 127 ms without). Outputs are unchanged: the allocator moves no output.
- **Back on when.** The root cause is found and fixed, and the caches come back with a regression
  test: the threaded realloc stress (`threaded_stress`, alloc/threaded-stress) under the load that
  showed the fault. Owner: CAM-PERF.

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
| The adaptive op's (path, voids, notes) memoised by its arguments' full Debug text (`adaptive/1`: region, the Cut destructured field by field, levels, in-process stock, parameters) | forge-cam-lang lib.rs `adaptive_memo` | adaptive rerun 6.0 s -> 13 ms (deep_islands_p_adaptive), 2.2 s -> 7 ms (boss_adaptive) | CAM-PERF (f9f4a115); test `adaptive_memo_tests::each_input_class_clears_anew` (fails with ip, prm or rg out of the key) |
| A lathe program's certification memoised (`turn-certify/1`: setup, job, posted moves, exact) | forge-turn-lang `certify_memo` | hub_turn rerun 0.37 s -> 25 ms | CAM-PERF (0bf9b496); test `certify_memo_tests::each_input_class_certifies_anew` (fails with any key part out) |
| The quality measure reads the run only through `CamView`, and the last measure is kept by the view's full Debug text | forge-cam-quality lib.rs, forge-render run.rs | a rerun's measure: manifold 2.4 s, mould_finish 1.6 s -> 4-12 ms | CAM-PERF (0ca759d2); JobQ byte-identical on 43 docs; test `the_kept_measure_is_the_fresh_one` |
| Each posted path's read-back and certification run in parallel, applied to the ops in the paths' order; memo state (bypass, counts) carried to the threads | forge-cam-lang `post`, forge-memo `Context` | manifold cold 0.81 -> 0.65 s | CAM-PERF (b73025f9); forge-cam-lang tests with forge-math/force_helpers; `work_on_other_threads_counts_and_bypasses_as_the_callers` |
| An engrave/slot sketch's frame answered from the part's own load (anchors and the cut sketches asked there), not two more runs of the part | forge-cam-lang `frame_of`, forge-lang query.rs `prepare`/`ask_prepared` | manifold rerun 53 -> 46 ms native, about 50 ms in the page | CAM-PERF (fcfea667); test `the_frames_are_the_asked_ones` (word for word against `ask`); debug builds re-ask every frame |
| The built-in machine definitions parsed once per process | forge-machine library.rs | tens of ms per run in the page | CAM-PERF (34343dc8); test `the_kept_builtins_are_the_parsed_ones` |
| The threaded page's allocator: heaps looked at quickly before any is flushed; a per-thread cache of small blocks | forge-alloc | page rerun 0.21 -> 0.12 s (manifold) | CAM-PERF (c0c6143f, 34d6cad8); parity.cjs 55/55 on 4 threads; tests `a_quick_look_neither_flushes_nor_grows`, `a_thread_cache_hands_out_each_block_once`. The per-thread caches are off for now: see Temporary safety exceptions |
| adaptive's start candidates: each disk test's answer kept until a sweep's box comes within 2r + 3h; the stall scan's edge distance and allowed test kept per level | forge-cam adaptive.rs (2bf1ccaf) | deep_islands adaptive 40.4 -> 12.2 s | TOOLPATH-QUALITY; posted programs of all 28 harness parts and examples byte-identical |
| adaptive's disk test answers "no material" from 8 x 8 block maxima over the cells its samples read (kept on adaptive's Field, not HeightMap: a field on HeightMap cost engage 1.7-2.1 % Ir) | adaptive.rs (51500301) | deep_islands 12.2 -> 11.4 s; flange (no adaptive) Ir unchanged | TOOLPATH-QUALITY; as above |

CAM-PERF's rows were checked byte-identical (the reply's JSON and the posted programs) on 43
documents: the 14 machining examples and verify/forge-cam-quality's harness. Train 5 already holds
CAM-PERF's earlier identical-output work (the surfacing, certification, finish3d, engagement,
section and round-trip memos; the height maps' tiles; the edge table; the kept tool library).
None of CAM-PERF's changes moves an output.
