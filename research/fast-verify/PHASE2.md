# FAST-VERIFY phase 2: what changed, measured (one 4-core VM, 2026-11-24/25)

## C. The speed gate (tools/speed, FALLBACK's; branches tools/gate-jobs-3, tools/gate-src-sync-2,
on tools/gate-memfns @e702b3cc; land after FALLBACK's re-baseline)

Same binaries (codegen-units=1), same VM. Every row is byte-identical to the serial gate on all 99
lines and bits, in order:

| how | wall |
|---|---|
| gate.sh as before (serial, source re-extracted) | 391–407 s |
| GATE_JOBS=4, source synced | 119 s |
| GATE_JOBS=3 on CPUs 1–3 | 112 s |
| GATE_MEMO hit (an identical build counted here before: main's side of a band, a branch that touches no gate crate) | **1 s** |

Found on the way. All are fixed in gate-memfns, the first two by FALLBACK:
- benchparts rewrote its font at one fixed temp path on every run, so two gates at once could read each
  other's half-written file (gearbox edit off by 364 Ir);
- the first counted run to create the font's folder was 564 Ir cheaper, so line order leaked into gearbox app;
- every gate rebuilt forge-lang and everything after it (~200 s): re-extracting the source gave every
  directory a new mtime, and forge-lang's build script watches one. Now synced; the incremental build's
  counts equal the clean build's.

## A. Owner pre-verify (tools/preverify, branch tools/preverify)

- `readsmap.py`: on main, each test binary run under strace. It records every repo file and directory the tests
  read at run time, plus their dep-info. 74 packages, 395 binaries: 1109 s cold, 706 s warm. It sees
  forge-cam-quality's verify/cam-parts/** and examples/machined/*.fg, and forge-model's hidden-inputs list.
- `affected.py`: the Cargo graph (every dependency kind) plus the reads map plus Cargo.lock entries plus global files.
  Over the 43 branches of trains 6 and 7 it selects **18 of 74 packages on average (25%)**. 13 of the 43 reach
  the gate. `selection-t6-t7.txt` has every branch.
- `run.py`: patent grep, guards, every_hidden_input_is_reviewed (whole), then clippy, native, render and wasm on
  the selection, then the gate (main against branch, same VM, band.py, memo). Its record says whether the branch carries e3148c8b.

Validation on train 7's two failures, each branch alone against the main it was cut from:

| branch | train 7 (old way, after the freeze) | pre-verify (new way, before) |
|---|---|---|
| cam/exact-box @543ac97f | forge-cam-quality: 2 in guards + pinned (bisect, ~40 min, rebuild) | RED native: chip_load_and_equal_work (guards.rs:103), adaptive_cutting_moves_not_micro (guards.rs:204), pinned_subset (pinned.rs:93). 1635 s, whole workspace (no map for that base) |
| ux/part-box before its fix @51cedd80 | forge-render a_sheet_part_shows_flat_and_exports_its_dxf, fg.rs:447 | RED render: a_sheet_part_shows_flat_and_exports_its_dxf, fg.rs:447. 792 s, 0 root packages + forge-render |
| ux/part-box with UX-LEAD's fix @12c1090c | (not in train 7) | GREEN, 1024 s |

The verdicts are identical. Both failures would have been found on the branch, before the freeze.
