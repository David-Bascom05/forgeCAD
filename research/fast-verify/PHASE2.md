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

## B. Sharded train verification: train 7's head, both ways (2026-11-25)

The same commit runs both ways: train 7 @8ca1a573, which is b3b3cae1 plus main's #58/#59. The old way is TRAIN-CONDUCTOR's
serial script on one VM. The new way is the three verifier sessions running at once, each on the exact sha. All runs are on the pinned
toolchain, rustc 1.99.1.

| stage | old way (serial) | new way (verifier, in parallel) | verdict |
|---|---|---|---|
| native (native-tests.py --workspace, JOBS=3) | 29 min; 0 failed; local budget notes | 1699 s (near-cold); 0 failed; exit 1 from the 2 s budget only (75 over; main has 76) | same |
| forge-render debug tests | 19 min; 1 failure, cam_ui document_text_is_a_bottom_panel (a known focus race, fixed for train 8) | 1148 s (near-cold); 0 failed | same except the race, which did not recur |
| forge-render release tests (CI's gpu job) | not run locally; CI went RED on studio_model | 1156 s; studio_model render_image_shows_its_progress_at_once_then_its_result FAILED (first frame 3.20 s vs <100 ms) | same as CI, now caught before the PR |
| wasm (wasm-tests.py) | about 15 min; green | 940 s; 403 binaries, 0 failed | same |
| **wall-clock** | **about 77 min** (native + render + wasm + 2 gates, one after another, target/ deleted between stages) | **about 28 min near-cold** (the slowest shard), with the gate on the conductor's VM at the same time (119 s; 1 s on a memo hit) | |

These are near-cold numbers, since each verifier had built main once first. Warm numbers will come from train 8.

## Found and fixed along the way
- Pre-verify judged relative to main. A failure or slow test that main has too is "main-identical", not RED (clippy, native, render, wasm).
  Fixed a hole: under -D warnings, a lint main has stopped clippy at its crate and left the crates after it unlinted. The comparison now runs warned.
- forge-lang tests: the bench font is written once per process and renamed into place (tests/faster-slow @36058798). This was the
  "block.ttf WrongKind" flake, two test threads in one process sharing the file. Pre-verify also gives each run a private TMPDIR.
- Slow tests: main has 76 tests over the 2 s budget on a 4-core VM. contour fuzz_random_profiles (178–322 s) is one
  of its 250 cases, a folded-clash search refining an at-the-limit stretch to 2^-48 (24M+ intervals). The diagnosis went to forge-sheet's
  owner. reparse runs the same edits (18519 patched, 50467 refused) in 7.5 s instead of 16.2 s.

## Slow tests rewritten (branch tests/faster-slow), with David's rigor rule

Every rewrite runs the same cases with the same assertions, and old and new catch the same mutations.

| test | change | same cases | mutations (old / new) | time |
|---|---|---|---|---|
| forge-lang reparse a_number_edited_patches_to_the_parse_of_the_edit | files checked in parallel; each file parsed once, its tree cloned per edit | 18519 patched + 50467 refused, both versions | M1 wrong value: FAIL/FAIL; M2 wrong span end: FAIL/FAIL; M3 comment check dropped: pass/pass (another test covers it, both versions alike) | 16.2 s → 7.5 s |
| forge-machine removal_tip (both) | each comparison's tips and its 3 maps in parallel | the same programs, machines, forms and assertions; identical printed gaps | M4 affine tip +1e-6: FAIL/FAIL; M5 closed height +1e-4: FAIL/FAIL | see mill_with |
| forge-machine removal::mill_with (library; the app's CAM sim too) | each move's rows of cells in parallel, applied in order | 12 maps (2 machines × 2 programs × 3 forms) bit-identical: every height, volume, cut, hit, trigger and probe hit | n/a (bit-identical) | 36.2 s → 21.4 s for the 12 maps, loaded VM |
| forge-lang tests' bench font (common::font_dir) | written once per process and renamed into place | n/a | fixes the "block.ttf WrongKind" flake | |

Diagnosed and sent to its owner, not changed here: forge-sheet contour fuzz_random_profiles. One case of 250 takes 322 s,
the folded-clash search refining an at-the-limit stretch to 2^-48.
