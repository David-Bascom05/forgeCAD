# Merge ledger

Kept by TRAIN-CONDUCTOR. One row per branch. The status values are: queued, pre-verify, in train N, merged, bounced (with the reason), and held (with what it needs).
Rules: each item needs a GREEN pre-verify on the train's main, made with tools/preverify ≥13d78f06 or proven clean by a record check or my own test build. UI changes need UX-LEAD's OK. Branches with the flagged history land squashed.
This branch (coord/trains) is never merged into main.

Main: 5a3f4875b4d57f23f8e1f653b4f43c84b5c01031 (train CI, #66). Train 11 landed at 568d5f497c3c343b8fafeebe6f1b50e13d9c56e3 (#65). Before it: 247a26942e5cc29a2ddc6b80ae43d6fe1751f45a (train 10, #64), d6820393 (train 9, #62), 5767cdb4 (gate-memfns alone, #63), 10d87778 (train 8, #61).
Train 12 ([#67](https://github.com/David-Bascom05/forgeCAD/pull/67)) is in gates. Train 13 ([#68](https://github.com/David-Bascom05/forgeCAD/pull/68)) is stacked for review.
UI OKs now come from the coordinator (UX-LEAD wound down), judged against docs/ux/STANDARD.md.

| branch | owner | sha | status | notes |
|---|---|---|---|---|
| tools/gate-memfns | FALLBACK | c4158938 | merged (#63) | alone; baseline re-saved with GATE_SELFTEST=1; isolation table 99/99 identical |
| perf/release-one-codegen-unit | FALLBACK | f0d81fd9 | held | CGU=1, not queued |
| tests/faster-slow | FAST-VERIFY | 728f42a8 | merged (train 10) | |
| claude/lang-det, claude/powi | LANG-PROOFS | 8be3bcac, 8d552fef | merged (train 10) | powi/libm bans; 5 newer powi uses and one log10 fixed on the train |
| exact-core/ival-fast | EXACT-CORE | 5c1912c7 | merged (train 10) | ival-strict feature: remove in a later train |
| sheet/near-limit | — | b9abe851 | merged (train 10) | |
| ux/undrawn-faces (+ fix/undrawn-face) | UX-LEAD | 0f37fb14 | merged (train 10) | |
| fix/agent-mcp-release | AGENT-TAB | 494a57c3 | merged (train 10) | |
| ux/handoff, alloc/threaded-stress | UX-LEAD, CAM-PERF | d1f5e0ef, a5a5a2b3 | merged (train 10) | docs |
| cam/engage-conservative | CAM-CORE | 11eb7fcb | merged (train 10) | pins re-blessed with hop-map-under |
| cam/rest-air | TOOLPATH-QUALITY | 1cb27e2c | merged (train 10) | KNOWN: rptu1 only |
| fix/asm-shape-first | HOSTED-DESIGN | 0ebc123f | merged (train 10) | |
| parasolid/xt-reader | PARASOLID | 9e207b79 | merged (train 10) | sampled cubic reverted (coordinator ruling); near misses in handoff |
| cam/sim-machine, cam/adaptive-only | CAM-SIM, TOOLPATH-QUALITY | 2d3968e3, a7d36860 | merged (train 10, content only) | |
| perf/ellipse-foot | CURVED-SITES | 22b612b9 | merged (train 11) | volute noise waived, bits change accepted |
| perf/intcurve-eval-memo | FALLBACK | a7d773c5 | held | Reverted at f02a0bbf. Net −3.27%, valve app +6.09%, shaft nary1 +4.90%. Needs a repeat-heavy-path implementation and fresh gates. |
| perf/varblend-rail-uv | FALLBACK | 55d9d68d | merged (train 11) | review SAFE; STEP pcurves of vary-blend springs may move last bits |
| flat/subdivide-reuse | FLAT-SITES | f0adb8dc02155c285b73188df7cc3aa44547afb4 | merged (train 11) | Gated and retained. |
| FLAT-SITES series | FLAT-SITES | see below | in train 13, cone-apex held | Reverted at 5975a755. Each needs a gate line. Cone-apex held for manifold parity (209 → 212 faces). |
| perf/asm-overlap-walks | HOSTED-DESIGN | 62d85634 | merged (train 11) | |
| ci/corpus-guard-budget | FALLBACK | 55791412 | merged (train 11) | |
| cam/speed-bull | CAM-SPEED | 7faf830a | in train 11 (content over 3a3fe6d0) | |
| kernel-stress | KERNEL-STRESS | 85601f88 | merged (train 11) | op-budget, mass-cap, face-box, import-sheets to follow |
| hosted/release-sketches | HOSTED | a7668e22 | merged (train 11) | |
| cam/clear-degenerate | CAM-CORE | 56ca6ad8 | merged (train 11) | |
| fix/query-interference-refusal | HOSTED-DESIGN | fbe2959b | merged (train 11) | |
| cam/workholding-any2 (vise, haas) | WORKHOLDING | e53f6e78 | merged (train 11) | |
| machines/real-collision | CAM-MACHINE | 93b50f50 | merged (train 11) | |
| cam/sim-vise-haas | CAM-SIM | cac0d489 | in train 11 (content only) | UI OK coordinator; supersedes cam/sim-vise 614e9808 (dropped) |
| ci/determinism | HOSTED-BUILD | f6f92445 | merged (train 11) | |
| claude/expect-words | LANG-PROOFS | 2096b4a7 | merged (train 11) | |
| bendsim/showcase | BENDSIM-ENV | 07b6b19b | in train 11 (content only) | UI OK coordinator; contains exact-core/bb-brake |
| cam/workholding-default2 | WORKHOLDING | dc60f541 | held | undecidable checker case at exact fixture clearance (coordinator ruling) |
| hosted/web-worker | WEB-WORKER (archived) | 4ea27f52 | held | needs a main merge after train 10: move main's set_part_boxes, set_explode and set_lineage into scene_of/view_on, re-run render and browser tests |
| cam/adaptive-ties, cam/adaptive-pass-start | CAM-CORE | edc2f3d84783654d4fbf4b44dd4406f8e5d4eddb, 9cbee5ea9baf4f2134af50cb5268a2ebbce7e2fb | in train 12 | |
| hosted/server-hardening, client-native, client-web | HOSTED | 38663ef76a0aaf7ca41a67ab002f13249bdcfd0b, f8c9038c486b0e0b85c78e59b97099c241e0220e, 5aa6912b9f2674a401866defaaeffe28cf0ca6e6 | in train 12 | client UI OK given |
| import/speed-ks | IMPORT-SPEED | 934102fa6d797045e1adbf8374bdc080d937e2dd | pre-verify | After kernel-stress. Verify evidence against landed main. |
| import/faces | IMPORT-SPEED | ba36e31bc052a47b03736e92e799c04fedf6e8ad | held | Needs independent review and the rigor package on the final candidate. |
| import/mesh-floor | IMPORT-SPEED | 0dbe360a754be73f528cb7f783e5a61d14c91fcb | held | Needs independent review and the rigor package on the final candidate. |
| fix/nurbs-stall, fix/far-line-torus | FALLBACK | — | held | after kernel-stress |

## FLAT-SITES work, pushed by the conductor (FLAT-SITES had wound down and could not push), 2026-11-25 09:15Z
Every branch was rebuilt from FLAT-SITES's format-patch text on its own verified base, and its tree equals the hash FLAT-SITES gave. Commit shas differ from FLAT-SITES's local ones; the trees are what match.

| branch | base | head | tree | contents |
|---|---|---|---|---|
| proof/flush-per-plane | ded2d224 | 7b61ccae | a3bd8102 | (already on origin) n-ary flush per plane |
| flat/open-shell | ded2d224 | c37bc7e4 | 11d2374a | OpenShell refused (25884d3e) |
| flat/cone-apex | ded2d224 | a241bcbe | 3aacadb1 | unify keeps a cone's tip (2c926b0b) |
| flat/check-chords | ded2d224 | ca7739f3 | 8286ca0f | check draws loops finer (68501ad8) |
| flat/g11-series | ded2d224 | df38be77 | fc184da6 | G11 docs ×3, trim-shallow 68f143af, T budget 1310c255, sampler U 7720efa0 |
| flat/accum-margin | ded2d224 | 17867560 | 1a14c639 | δ-accumulation 7e530dc1, shape margin 5dd101a7 + 2af37071 |
| flat/whole-turn | 6cd93533 | d37b853f | aade6d92 | model-size tests 875846c6, exact whole turn 722cad0b |
| flat/subdivide-reuse | 10d87778 | f0adb8dc | 05d57413 | speed: subdivide reuse (bits identical, −1.08%) + research/handoff/FLAT-SITES.md (blob b66e9152) |

Current disposition is in the queue above.

## Live queue reconciliation, 2026-11-25

- [Train 11 #65](https://github.com/David-Bascom05/forgeCAD/pull/65), head `8348be6c5263ff9e111b518477bb7e06af87029a`, base `247a26942e5cc29a2ddc6b80ae43d6fe1751f45a`. The PR records green per-branch pre-verifies and coordinator UI OKs. It attributes intended bit changes to ellipse-foot and the reported over-band lines to the reverted memo. GitHub Actions: all 16 workflows succeeded, including [determinism](https://github.com/David-Bascom05/forgeCAD/actions/runs/45648438485) and [web](https://github.com/David-Bascom05/forgeCAD/actions/runs/45648438301). The PR still lists final local render, WASM and band results as pending and native debug time-budget failures require disposition. Obtain the final records on this head and base before merging under the documented history rules. The Codex VM denies `setarch -R`, which the exact Callgrind gate requires.
- [Train 12 branch](https://github.com/David-Bascom05/forgeCAD/tree/merge-train/12), head `f85ec2aa36ee4ec2715460b6a9d60c0848d4647d`, 88 commits beyond the Train 11 head. Includes `afc58f54` (strict origin parsing and WSS outside loopback) and `7b3a387a1f440e29c7fad09f1545f1e28c21f8a7` (gate jobs). Four-machine reports remain unverified. Verify against the landed main before merge. No branch workflow or PR was visible. Draft creation through the GitHub connector and GitHub CLI returned internal errors, and a direct REST retry returned an empty response. The live PR list still showed no Train 12 PR.
- [#43](https://github.com/David-Bascom05/forgeCAD/pull/43), head `6dbac7aa30d013a4802403f06da2c47941473d8f`. Draft and conflicting. Conflicts are boolean.rs, hidden-inputs.txt and the forge-bool proof ledger. Owner decision pending on the measured +0.0353% strict plate and torture lines. A ready candidate also needs current-base integration and gates.
- [#19](https://github.com/David-Bascom05/forgeCAD/pull/19), head `cbed8d093fa68a98daa8f64f6e7f6d35902f452a`. Conflicting. Prior evidence used `0870264` and reported valve app +0.299% against the then-applicable +0.25% ceiling. Current merge conflicts are Cargo.toml, tools/aeneas/survey-modules.sh and verify/aeneas/bridge/AeneasF64/Core.lean. Resolve alongside the proof dependency and repeat the applicable gates on current main.
- Freeform-surface candidate `ks/nurbs-stall` at `6502ae9aa39f5675f7e4d79ca993b76678a9d32f` includes kernel-stress and the three fix commits from `fix/nurbs-stall` (`da29417d77dfdb5352bbac42030c14cb40849807`). Local integration on the Train 11 candidate (local commit 51f37ff80e954e8f9fb667112c59789764168c9e) passed both release Haas Boolean tests with Rust 1.99.1. Strict Clippy on forge-geom, forge-bool and forge-step initially failed on intcurve.rs. Local commit 981cb808aaab02dbcdcad44d641db2d737c4e57b preserves the slope guard’s behavior and passes strict Clippy, the pre-verify guard step and both release Haas Boolean tests on rerun. Full pre-verify and exact gates still need landed main after Train 11.

TRAIN-CONDUCTOR remains paused.

The ledger write through the GitHub connector returned an internal error. A normal fast-forward push to coord/trains returned HTTP 403 for David-Bascom05. This reconciliation is saved in the Codex workspace.

## Train 11 landed

- [#65](https://github.com/David-Bascom05/forgeCAD/pull/65) merged at `568d5f497c3c343b8fafeebe6f1b50e13d9c56e3`, verified as current main. The user confirmed the prior agent passed the outstanding gates.
- Local Train 12 candidate `3bd9cb3a869599321f783b9227f303d18d8bc39e` integrates the landed main. Its tree `e153bbf082c5e6e8c8b4e7c13ded8b7654959efe` is identical to the existing Train 12 head. The fast-forward push returned HTTP 403. No Train 12 PR is open. The four-machine gate evidence remains unverified.

## New CI and the next trains

- [#66](https://github.com/David-Bascom05/forgeCAD/pull/66) merged at `5a3f4875b4d57f23f8e1f653b4f43c84b5c01031`. [The hosted run](https://github.com/David-Bascom05/forgeCAD/actions/runs/45662062523) passed all seven pre-verify checks and the AMD64 and ARM64 geometry reference checks. The exact Callgrind comparison measured 20,017,033,325 strict instructions on each side and identical geometry hashes.
- [Train 12 #67](https://github.com/David-Bascom05/forgeCAD/pull/67), head `39b2c04df1671d867943c9b25870b11f63763736`, base `5a3f4875b4d57f23f8e1f653b4f43c84b5c01031`. [New CI](https://github.com/David-Bascom05/forgeCAD/actions/runs/45670156710) started. Final hosted review coverage and applicable gate evidence must be confirmed before merge.
- [Train 13 #68](https://github.com/David-Bascom05/forgeCAD/pull/68), draft head `7ed230ec0b554a8b1a3eb3b0bbdd8390edc67aa3`, stacked on `39b2c04df1671d867943c9b25870b11f63763736`. Flush-per-plane, open-shell, check-chords, G11, accumulation margin and whole-turn were integrated locally, including the drift documentation and both regression tests. Cone-apex remains held. Per-item gates and applicable independent reviews are pending against landed main.

- Train 12's first hosted gate passed against `5a3f4875b4d57f23f8e1f653b4f43c84b5c01031` with identical geometry and 20,017,033,325 strict instructions on both sides. Hosted Clippy found four banned test-only integer-power calls. Fixed head `772bbf9c9f22d4d760c1f2efb3578c6dbbd0b011` passed strict pre-verify Clippy locally. [Exact-head CI rerun](https://github.com/David-Bascom05/forgeCAD/actions/runs/45673007725) pending.
- Train 13 passed 32 focused local Boolean regression tests on `7ed230ec0b554a8b1a3eb3b0bbdd8390edc67aa3`.

- Train 12 rerun on `772bbf9c9f22d4d760c1f2efb3578c6dbbd0b011` passed Clippy, WASM and exact Callgrind. The native suite's assertions passed, but test budgets failed. Its baseline run stopped at a candidate-only package, leaving inherited and new slow cases unresolved. Train 12 head `ec2b48aeef9de68163be9347019974f11f6c90f0` corrects that comparison. New tests remain subject to the time budget. [Hosted rerun](https://github.com/David-Bascom05/forgeCAD/actions/runs/45681076188).
- Train 12's six new adaptive round-box assertions pass under the native harness. Five exceeded the comparison's four-second isolated allowance in this VM (4.47–5.21 s). Profiling the round-box case measured 5.26 s in CAM planning and 0.11 s in the stock sweep. The timing issue is reproducible independently of the baseline-comparison bug. Keep #67 open until the new timing failures are resolved and the exact-head CI is green.
- Train 13 can land independently of Train 12. [#69](https://github.com/David-Bascom05/forgeCAD/pull/69) carries the same FLAT-SITES delta as stacked draft #68 directly on main. Head `b6fd30dec9793d35973adf1924cfbf11e181b2b9`, base `5a3f4875b4d57f23f8e1f653b4f43c84b5c01031`. [Hosted train gates](https://github.com/David-Bascom05/forgeCAD/actions/runs/45682069755) queued. Cone-apex remains held. Existing flush-per-plane and open-shell exceptions cover historical measurements against ded2d224. The integrated candidate still needs its current-base gate verdict, per-item evidence, and applicable independent review.
- Standalone Train 13 [#69](https://github.com/David-Bascom05/forgeCAD/pull/69) head `18169987f63df6c0ee7ece19aec8d9c2b280bfa1`, synthetic candidate `7ff1e2bf741e`, base `5a3f4875b4d57f23f8e1f653b4f43c84b5c01031`. Fixed deterministic math in the new regressions. Local Clippy, guards, focused tests and all 84 reference hashes passed. Hosted clippy, guards and hidden-input audit passed. [Exact Callgrind](https://github.com/David-Bascom05/forgeCAD/actions/runs/45682279105/job/139446457720) failed on 23 strict lines against the +0.03% band. Strict total +0.0067%, worst coplanar seq +0.1850%, heatsink edit +0.1733%, housing seq +0.1413%. Hold until the current-base over-band lines are attributed and brought within the band or dispositioned under the documented correctness-exception process. Historical exceptions do not cover every current line.
- Train 13 [#69](https://github.com/David-Bascom05/forgeCAD/pull/69) performance investigation. Base `5a3f4875b4d57f23f8e1f653b4f43c84b5c01031`, prior head `18169987f63df6c0ee7ece19aec8d9c2b280bfa1`. On the same source path and CPU with Callgrind, sequential coplanar added 334,120 instructions inside general and housing added 188,497 there. These match the hosted absolute benchmark deltas. The added pass examines edges of removed target faces/loops to reject an open shell. Local heatsink edit instead improved overall and does not explain the hosted +313,651. [Exact-host instruction profiles](https://github.com/David-Bascom05/forgeCAD/actions/runs/45692504315) requested for all three revisions. Trial head `2e7d9a2d7a4c9029cd8c45ddb744b6e0455757b9`, merge candidate `072cdd674c69791740d97c39f5399111d02e1468`, retains the checks and reuses loop storage, filters disjoint faces before pair traversal and initializes the plane cache lazily. 32 focused tests passed locally. [Hosted CI](https://github.com/David-Bascom05/forgeCAD/actions/runs/45691835560) and [independent review](https://github.com/David-Bascom05/forgeCAD/pull/69#issuecomment-7173228324) pending. Prior head also failed the render-studio browser e2e timing assertion (479 ms, limit 386 ms). Keep draft until the applicable gates and reviews clear.
- Train 13 profiling update. [Hosted profiles](https://github.com/David-Bascom05/forgeCAD/actions/runs/45692504315) reproduce the original gate's full base and candidate count files byte for byte. On heatsink edit, `18169987` added 313,651 instructions. Of these, 201,842 are glibc copies, 59,761 are in `_int_malloc`, 23,996 are in the n-ary pair loop and 12,926 in `boolean::general`. The edge-sized vector growth path ran 432 times on both revisions but cost 201,929 more instructions including its callees. Allocator placement determines whether a growing buffer must move. Sequential coplanar, housing, patterns, plate, shaft, torture and upto-flat regressions are entirely in the new open-shell scan. [Per-item profiles](https://github.com/David-Bascom05/forgeCAD/actions/runs/45693482087) are being collected separately on this base; the isolated finer-chords fix already matched the base strict counts and geometry.
- `2e7d9a2d` brought the exact strict total to −0.0187%, coplanar seq to −0.0991% and housing seq to −0.1544%. 22 of the 23 originally failing lines cleared. Heatsink edit was still +0.1533% and bracket app was newly over at +0.0631%. No findings in the independent review; both geometry-reference architectures passed. The hidden-input inventory needed the two new lazy-cache declarations, now justified and locally tested. [Train 13 PR](https://github.com/David-Bascom05/forgeCAD/pull/69) head `73513c86aab85b90d68259a3fb5867ec9583ff72` preallocates the known chamfer topology to avoid repeatedly moving large edges and faces. Synthetic merge candidate `7a65ee07765affb5b03bdea9dffdf4cf40594900` vs `5a3f4875b4d57f23f8e1f653b4f43c84b5c01031`. [CI](https://github.com/David-Bascom05/forgeCAD/actions/runs/45695144874) and [independent re-review](https://github.com/David-Bascom05/forgeCAD/pull/69#issuecomment-7173710035) pending.
- [Train 13 per-item attribution](./TRAIN-13-PERFORMANCE.md). All 99 integrated benchmark deltas on the original head are the exact sum of the isolated open-shell and flush-per-plane deltas. Four isolated items matched main on every count. All six matched the 84 geometry hashes. The independent review of `73513c86` completed without findings and both architecture reference jobs, Clippy, guards and the updated hidden-input audit passed.
- [Exact profiles on `73513c86`](https://github.com/David-Bascom05/forgeCAD/actions/runs/45695711876) show the heatsink edit and bracket build regressions recovered, with four other strict lines above +0.03%. [Breakdown](./TRAIN-13-PERFORMANCE.md). The PR stays in draft. Testing [the repository's usual growth allowance for chamfer corner entities](https://github.com/David-Bascom05/forgeCAD/actions/runs/45697910868) and [a separately reviewable sampling experiment](https://github.com/David-Bascom05/forgeCAD/actions/runs/45697574462) off the merge branch.
