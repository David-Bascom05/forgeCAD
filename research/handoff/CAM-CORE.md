# CAM-CORE handoff (2026-11-25, wind-down)

CAM-CORE owned forge-cam's core: adaptive clearing, pockets and `clear()`, the engagement model
(engage.rs) and its feeds, the certifier's closed forms (forge-cam-geom), fixtures, and the
forge-cam-lang plumbing around them. This note covers what was in flight when the wind-down came:
the branches, their state, what is unmerged, what to do next, and the risks.
Shas are as pushed, on main c135b274 (train 7) unless said.

## Branches for train 9 (merge-ready, pre-verified or being)

Train 8 closed before these were ready; TRAIN-CONDUCTOR takes them in train 9 (lines made on
c135b274 with tools/preverify 13d78f06 stand). `cam/adaptive-cap-warning` went in train 8.

| branch @ sha | what | state |
|---|---|---|
| `cam/adaptive-cap-warning` @ 38e413d4 | forge-cam-lang warns when adaptive's level cutting goes over its `engagement:` cap (UX-LEAD's wording); forge-cam-quality guard over every benchmark part with 17 known violations (folded into `chip_load_and_equal_work`'s measure; arcs measured whole) | PRE-VERIFY GREEN, sent |
| `cam/engage-conservative` @ 11eb7fcb | the engagement read of the feeds never under-reads (each edge point dilated by half a cell); flange/plate_break .nc regenerated (feed words only, all lower) | re-running (first run: a forge-render timing test under load) |
| `cam/adaptive-ties` @ edc2f3d8 | adaptive reports stock left after every pass and ring | queued |
| `cam/fixture-box-mu` @ f357f65f | a shaped fixture piece's box grows by its faces' mu (it shrank: the machine-sim collision boxes were not conservative); test fails on the old signs | running |
| `cam/clear-degenerate` @ d9c8ddaa | rptu1 `nut` (clear): a line cut again the other way is air (ends matched within 1e-5 mm, not by bits), and same-height rapids go straight. Re-cut 161 → 1 mm; rapids 593 → 397 mm | queued. **Lands after TQ's `cam/crazy-guards` in train 9**; TRAIN-CONDUCTOR applies the KNOWN edit as a fix-up in train 9's assembly (the coordinator's ruling; merge-main-only stands): in verify/forge-cam-quality/src/crazy.rs drop `("rptu1_clear_cam", "nut", "recut", …)` (a listed entry no longer broken fails guards.rs) and set the nut `rapids` entry to 397.0, why "clear round the nut: the slot's far-end trips for the islands' corner arcs (trim_air's box test cannot see they remove nothing; a stock-aware air test)" |

## Unmerged and not ready

- **`cam/adaptive-pass-start` @ f8fb1b4d**: the round-box fix. A level is cleared whatever the
  stock box's numbers:
  - In air, the heading turns by a look-ahead engagement.
  - A level sweeps again while material more than 4r from the walls exceeds 4 cells.
  - Stock left is reported.
  - Tests: interface/forge-cam-lang/tests/adaptive_round_boxes.rs (12 cases, fails on the old code).

  **It needs TOOLPATH-QUALITY's `cam/hop-map-under` in main first**: without it, boss_adaptive's
  links graze stock at the link feed (chip-load guard, hex 1.50 against fz 0.129); with it,
  forge-cam-quality passes. Once that is in main: merge main, pre-verify, send.

  Cost, reported not targeted: deep_boss_n_adaptive t_stop 648 → about 1045 s, same volume removed.
  It is the cap's honest price: holding 40° with r = 3 needs at least ~6,960 mm of cutting where
  main fits 4,333 mm only by breaking the cap, its rings slotting at up to 180°.

  Check the engagement-cap guard's figures (`cam/adaptive-cap-warning`'s CAP_KNOWN) once both are in
  main: this branch moves the adaptive paths. Lower figures should be re-listed, and a higher one
  explained.
- **`cam/adaptive-pass-start-wip` @ 67bf955d**: the pass step-read fix, with a differential test,
  `passes_hold_the_cap_by_an_independent_measure`.
  - The planner read each step's engagement at its end along the chord. The posted path is arcs
    through the step ends, so the heading there is turned further into a turn, and a stock corner
    can meet the edge between ends.
  - Measured passes reached 51.5° under a 40° cap. With the fix the worst is 46.5°, within the
    two models' stated rounding.
  - **Not landing**: it breaks forge-cam-quality's micro-move guard (deep_open_slot_p_zigzag 14 of
    89 cutting moves under 1 mm; was 5). The stricter read stalls passes at the margin ring's
    corners into 2–3-step nibbles.
  - A half-turn end read was tried: it misses both bounds by a hair (micro 11/91, differential
    0.2 mm over).
  - Next: avoid the nibble passes rather than the read, by letting a stalled pass in a corner
    turn on, or by fitting the short polylines.
  - The branch also carries the look-ahead and leftover commits above, and TQ's 68000590; rebase
    only the step-read commit onto `cam/adaptive-pass-start`.
- **`cam/loop-band-budget-wip` @ 9022ad8e**: prototype. G4's leftover-band offset under a
  refined-bits budget (`Ctx::refined_bits`, `offset_within(max_bits)`, ~500k; normal docs peak at
  285,696, the bad case 1,402,624), falling back to the conservative credit with a note.
  - It waits on EXACT-CORE's exact-core/ties in main (566d28e4 or later, not 3098a105).
  - **It edits forge-cam-geom/src/real.rs, which EXACT-CORE asked CAM-CORE not to touch**: move
    `refined_bits` to EXACT-CORE or agree it with them first.
- **`cam/exact-box` @ 543ac97f**: the exact part box from the model (tasks: exact box back in after
  engage-conservative).
  - Held back from train 7 when forge-cam-quality was not run before handoff.
  - Needs main merged (after engage-conservative lands), the quality guards, and a judged
    re-bless of pins and .nc.

## Not started (in scope or promised)

- **Real workholding on about 3 examples** (coordinator's wind-down item (c)).
  - Eight machined examples have no workholding: block_m6, flange, mold_core, mold_core_finish,
    plate_break, plate_mark, slot_plate, well_island.
  - The standard vise is `haas-vise-05-0404`. It is only on WORKHOLDING's `cam/workholding-haas`,
    not in main, so it waits for that.
  - Flange (worked out, not finished): with a 3 mm clamp the profile to −0.5 reaches 0.5 mm into
    the jaws. The realistic plan is two setups:
    - op1 in the vise faces, clears, bores and drills, and profiles only to about z 4;
    - op2 is the part turned over (`setup(f, from: op1, ..)`), gripped by those profiled sides,
      facing the 3 mm below off and profiling the rest.
  - Certifying against the vise's real bodies is slow (over 6 minutes, unfinished, for the
    two-setup flange). Time it, and say in each file which approach it uses.
  - The mould cores want a Haas Tooling fixture plate from WORKHOLDING's inventory.
  - flange and plate_break have tracked .nc: re-bless once, judged.
- **rptu1 `nut` rapids**: still 397 mm against crazy.rs's 300 mm bound. The rest is each level's
  trip to the slot's far end for the islands' corner arcs, which touch the stock's box but remove
  nothing. trim_air's air test is the stock box, so it cannot tell. A stock-aware air test (the
  level's own cut so far), or ordering a level's loops to end where the next level starts, would
  fix it. Tell TOOLPATH-QUALITY to drop the `recut` KNOWN entry and set `rapids` to 397 when
  `cam/clear-degenerate` lands.
- **TQ's rounds hook**: two rounds of sweeps and rings per adaptive level, the wall pass after the
  last. The decision point is the `next_sweep!` macro in adaptive.rs (~line 664). Not done; TQ
  owns the ring redesign (option B).
- **2.5D pocket per-level sections**: the pocket takes one mid-height section for all levels, so on
  the manifold's `well` (26 → 8, island top 20) it leaves a Ø12 column above the island. Use the
  section of each level band. Tell CAM-2D-FINISH, who will re-check `shal`.
- **axisplane tests' cost**: two tests take about 4.7 s each, over the 2 s budget. My guess, not
  measured: the branch and bound running to its 100k budget on ties it cannot decide; give that
  check a smaller budget without weakening it.
- **CAM-SPEED**: retract_link/top0 (43 % of adaptive's time on deep_boss) is theirs, after
  `cam/adaptive-pass-start` lands, byte-identical with a differential test. Levels share one
  stock map, so parallel levels need an all-levels-from-the-stock-vs-chained differential first.
  David has ruled that bit-identity across thread counts is not required, only correctness.

Older branches of mine that show open (`cam/bits-pin`, `move-memo`, `origin-snap`, `sheet-cycle`,
`t5-parity`, `tools-cli`, `ui-cert-words`) are from trains 4–7, believed landed through their
squash merges. They have not been checked one by one: the train records are the source of truth.
`cam/bits-pin` is a stale one-line re-pin.

- **`fixture: plate(NAME, ..)` / `zero_point(NAME, ..)`** (forge-cam-lang, ours): needed for the
  mould cores on a real plate.
  - WORKHOLDING's recipe: resolve the name in the workholding library as `vise_record` does, then
    `forge_workholding::mill::fixture_plate` or `::zero_point` (each returns a `Placed`), then
    `Fixture::from_workholding`.
  - No Haas plate has a library record yet (05-0301…0321 are catalogued in
    research/cam/workholding-haas.md on cam/workholding-haas; a record needs the real mounting
    grid and clamp keep-outs).
  - Until then, a vise with soft jaws or a stand-in plate record is honest, if the doc says which.

## Risks

- **"Held by the fixture" tests boxes, not bodies** (forge-cam-lang lib.rs ~2845, the leftover
  reasons). A leftover band below the jaw top and within clearance of a fixture *box* is excused.
  With a real model (haas-vise-05-0404: 3522 faces), the box is an enclosure that reaches past the
  body, so it excuses more leftover than the body would. That bears on "stock cleared or
  reported". It should test the faces (a box-to-face distance per band, shaped faces included).
  Not small; not done.

- **Adaptive is slower on purpose, and the cap is still broken on most parts.**
  - Rings (plain offsets half a diameter apart: 90° by construction) and the helix's bottom turn
    are what break the cap now; passes hold it to within a few degrees.
  - The cap guard lists 17 ops, the warning reports them, and the bottom turn should be counted as
    entry (agreed with the coordinator, not done; its feed is the slot chip, checked).
  - TQ's ring redesign is the real fix.
- **trim_air's repeat test is now approximate** (1e-5 mm). It can only turn a near-repeat into air,
  which leaves at most a sliver far under tolerance, and what replaces the run is certified. But
  the merged straight rapids now cross where the old waypoints went round. They are certified, so
  a fixture above the retract plane would show as a refusal, not a crash.
- **The pre-verify tool**:
  - Use tools/preverify 13d78f06 or later; earlier versions passed steps that did not compile.
  - Give each run a fresh CARGO_TARGET_DIR: an old main-comparison build left stale binaries with
    a deleted worktree's paths.
  - Keep 12 GB free.
  - Run nothing heavy beside it: forge-render has timing-ratio tests that fail under load.
- **Engagement model vs quality model**: two models with different cells and read radii. The
  differential test on the WIP branch states the tolerance from both cells. Keep such a test in
  any engagement change.
