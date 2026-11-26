# CAM-SIM handoff

The CAM simulation: `interface/forge-render/src/camsim/` and `gui_camsim*.rs`. It covers the
timeline, the z-map stock, the posted code on a machine, the tools and holders, the workholding
and the checks tab. Wound down on 2026-11-25 at David's direction.

## Branches, in the order they land

| branch | sha | what | state |
|---|---|---|---|
| `cam/sim-fixes` | | David's three bugs (the soft-jaw part, tool changes, the label) | landed in train 8 |
| `cam/sim-machine` | 2d3968e3 | one machine list (forge-cam-lang `machines`), the Haas VF-2 run, the operator's view, fine detail at the tool, the clips, the display grid's stock markers labelled approximate | PRE-VERIFY GREEN on main 10d87778 (tool b9318989), UX OK, train 10 |
| `cam/workholding-any2` | 11c7a3b2 | WORKHOLDING's: any workholding (`vise(NAME)`, `workholding(NAME, at:)`, user imports), `Fixture::table_z`, `Fixture::placed`, `body_source` | WORKHOLDING's, CAM-CORE OK |
| `machines/real-collision` | 93b50f50 | CAM-MACHINE's: the machine bodies checked by their real parts (`real_trees`, `collide::check_with`), the stock as given in the sim's check | CAM-MACHINE's (wound down), GREEN-EXCEPT-RENDER, with TRAIN-CONDUCTOR |
| `cam/sim-vise-haas` | (this branch) | the vise on the table, "Workholding off the table", the no-workholding notice, any placed device drawn from its real parts, shipped models (`drawn_dir`) and "Model not downloaded", Display ▸ Checked geometry, the drawn-vs-checked ratchet, the user-fixture test, the VF-2 clip in the real Haas vise (research/cam/sim/shots/camsim-flange-vf2-vise.mp4) | merges all of the above; lands after them. `cam/sim-vise` (614e9808) is its earlier, superseded state |

Merging train 8 into a branch that carries `cam/workholding-any2` needs three fixes. This branch
has them; WORKHOLDING was told:
- forge-cam-lang `Cam`'s prefetch copy needs `wh: self.wh.clone()`.
- `forge-cam-lang/Cargo.toml` ends up with two `[dev-dependencies]` tables; join them.
- For `Cargo.lock`, take main's and let cargo add the branch's crates.

## What's drawn against what's checked (David's rule)

`tests/camsim.rs::what_is_drawn_against_what_is_checked` covers every body of every built-in
machine. It compares the source the sim draws from with the one `Cam::machine_run`'s check takes
(`camsim::machine::sources`).
- The gap count was 1170 of 1170. With CAM-MACHINE's real trees it is 3.
- The 3 remaining bodies are display-only (`check = false`): the Haas enclosures, whose parts are
  solid blocks, and the VF-2's way covers.
- `GAPS_AT_MOST = 3` may only fall.

Fixtures, any device (`camsim::machine::holding_sources`,
`a_fixture_of_the_users_own_is_drawn_as_it_is_checked`):
- A user's imported STEP plate is drawn from the mesh its compile wrote. That compile is the
  same one as the faces certification checks.
- The machine run still checks a model body by `Fixture::boxes()`, the box about its faces,
  which can reach 6.5 mm past the body. That is counted as a gap (`HOLDING_GAPS_AT_MOST = 1`).

Next, for CAM-MACHINE with WORKHOLDING:
- Pass `Body.exact` faces (forge_cam::fixture::body_source) to `machine_run`'s check.
- Then make `holding_sources` report them, and lower the count to 0.
- WORKHOLDING's rule: draw from `placed` (it is tight); check from the faces; never mix in the
  enclosing boxes.

## Unfinished

- The UMC-750 clip was never made.
- UX-LEAD's notes:
  - From train 7 (P2): change ticks over op labels; the status line's order, with the full text
    on hover.
  - From train 10 (P3): the head's framing, and the Y X Z readout order.
- The z-map stock markers are approximate, and labelled so. A real stock check in the machine
  run is CAM-MACHINE's: "stock (as given)" is now a body in `check_with`, as a superset, never
  claimed a Hit.

## Risks

- `cam/sim-vise-haas` carries the two branches above. If either changes before landing, merge it
  again; don't let both land their own copy.
- The VF-2 test and clip documents give the flange 6 mm of stock under the part. With a real vise,
  the profile's 0.5 mm break-through reaches the jaws (certification catches it). The flange
  example itself has no vise.
- Workholding meshes are found through `FORGECAD_WORKHOLDING_MODELS`, else the build's source tree
  (`camsim::machine::holding_part`). An installed build needs that path, or the meshes installed.
  Without them, a body falls back to its own tight prims.
- Clips stay in history for good (TRAIN-CONDUCTOR), so keep them short.
