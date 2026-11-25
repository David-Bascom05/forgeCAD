# Merge ledger

Kept by TRAIN-CONDUCTOR. One row per branch. The status values are: queued, pre-verify, in train N, merged, bounced (with the reason), and held (with what it needs).
Rules: each item needs a GREEN pre-verify on the train's main, made with tools/preverify ≥13d78f06 or proven clean by a record check or my own test build. UI changes need UX-LEAD's OK. Branches with the flagged history land squashed.
This branch (coord/trains) is never merged into main.

Main: 247a26942e5cc29a2ddc6b80ae43d6fe1751f45a (train 10, #64). Before it: d6820393 (train 9, #62), 5767cdb4 (gate-memfns alone, #63), 10d87778 (train 8, #61).
Train 11 ([#65](https://github.com/David-Bascom05/forgeCAD/pull/65)) is in gates. Exact head and gate evidence are below.
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
| perf/ellipse-foot | CURVED-SITES | 22b612b9 | in train 11 | volute noise waived, bits change accepted |
| perf/intcurve-eval-memo | FALLBACK | a7d773c5 | held | Reverted at f02a0bbf. Net −3.27%, valve app +6.09%, shaft nary1 +4.90%. Needs a repeat-heavy-path implementation and fresh gates. |
| perf/varblend-rail-uv | FALLBACK | 55d9d68d | in train 11 | review SAFE; STEP pcurves of vary-blend springs may move last bits |
| flat/subdivide-reuse | FLAT-SITES | f0adb8dc02155c285b73188df7cc3aa44547afb4 | in train 11 | Gated and retained. |
| FLAT-SITES series | FLAT-SITES | see below | queued | Reverted at 5975a755. Each needs a gate line. Cone-apex held for manifold parity (209 → 212 faces). |
| perf/asm-overlap-walks | HOSTED-DESIGN | 62d85634 | in train 11 | |
| ci/corpus-guard-budget | FALLBACK | 55791412 | in train 11 | |
| cam/speed-bull | CAM-SPEED | 7faf830a | in train 11 (content over 3a3fe6d0) | |
| kernel-stress | KERNEL-STRESS | 85601f88 | in train 11 | op-budget, mass-cap, face-box, import-sheets to follow |
| hosted/release-sketches | HOSTED | a7668e22 | in train 11 | |
| cam/clear-degenerate | CAM-CORE | 56ca6ad8 | in train 11 | |
| fix/query-interference-refusal | HOSTED-DESIGN | fbe2959b | in train 11 | |
| cam/workholding-any2 (vise, haas) | WORKHOLDING | e53f6e78 | in train 11 | |
| machines/real-collision | CAM-MACHINE | 93b50f50 | in train 11 | |
| cam/sim-vise-haas | CAM-SIM | cac0d489 | in train 11 (content only) | UI OK coordinator; supersedes cam/sim-vise 614e9808 (dropped) |
| ci/determinism | HOSTED-BUILD | f6f92445 | in train 11 | |
| claude/expect-words | LANG-PROOFS | 2096b4a7 | in train 11 | |
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
