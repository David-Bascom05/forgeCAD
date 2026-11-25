# Merge ledger

Kept by TRAIN-CONDUCTOR. One row per branch. The status values are: queued, pre-verify, in train N, merged, bounced (with the reason), and held (with what it needs).
Rules: each item needs a GREEN pre-verify on the train's main, made with tools/preverify ≥13d78f06 or proven clean by a record check or my own test build. UI changes need UX-LEAD's OK. Branches with the flagged history land squashed.
This branch (coord/trains) is never merged into main.

Main: c135b274 (train 7, #60). Next: tools/gate-memfns alone, then a baseline re-save, then train 8.

Test-build check (mine, after the preverify false-ok bug): forge-render `cargo check --tests` and workspace native + wasm32-wasip1 `--tests`, 0 errors on main c135b274 and on 02e52375 a77d6380 ae8d6144 859a9c5e 38e413d4 6cb5d0f6 adcd26bc b31d58a5 6d610f35 c9e83517 9a1037e3 6a7b384d. Positive control 3a73019c caught E0063.

| branch | owner | sha | status | notes |
|---|---|---|---|---|
| tools/gate-memfns | FALLBACK | (local 0d9585a2) | pre-verify | lands alone first; carries the gate font-race fix; + corpus_guard 15 s class, + wasm32-unknown-unknown in toolchain (in or after) |
| agent/tab-8 | AGENT-TAB | b31d58a5 | queued (train 8, item 1) | GREEN, record clean, security clear + UX OK carry |
| agent/paste | AGENT-TAB | 9a53ec43 | pre-verify | platforms.yml green on train; arboard in docs |
| render/studio-model | RENDER-STUDIO | 02e52375 | queued (train 8) | GREEN, record clean, UX OK |
| render/big-models | BIG-RENDER | adcd26bc | queued (train 8) | GREEN, UX OK carries (only main merged) |
| cam/narrow-alternate | CAM-UI | a77d6380 | queued (train 8) | GREEN, record clean, UX OK; carries export-honest |
| cam/narrow-cube | CAM-UI | c9e83517 | pre-verify | UX OK conditional on GREEN; after narrow-alternate |
| cam/sim-fixes | CAM-SIM | ae8d6144 | queued (train 8, squash) | GREEN, record clean; e3148c8b in its history |
| ux/part-box | UX-LEAD | 859a9c5e | queued (train 8) | GREEN, UX OK |
| ux/narrow-browser | UX-LEAD | 1cc6792a | pre-verify | UX OK |
| ux/standard-focus | UX-LEAD | 9a1037e3 | queued (train 8) | GREEN, UX OK |
| ui/audit-scrollbars | BUTTON-AUDITOR | 6d610f35 | pre-verify | UX OK; carries audit-b6; render check by VERIFY-RENDER queued |
| cam/adaptive-cap-warning | CAM-CORE | 38e413d4 | queued (train 8) | GREEN, record clean, UX OK |
| cam/engage-conservative | CAM-CORE | 11eb7fcb | pre-verify | early in order |
| cam/adaptive-ties | CAM-CORE | edc2f3d8 | pre-verify | |
| cam/fixture-box-mu | CAM-CORE | f357f65f | pre-verify | coordinator's safety fix; merges clean with workholding-vise |
| cam/workholding-vise | WORKHOLDING | 4eacb184 | pre-verify | UX OK; needs the tool line |
| cam/hop-map-under | TOOLPATH-QUALITY | 6a7b384d | queued (train 8) | GREEN, record clean |
| cam/adaptive-only | TOOLPATH-QUALITY | e6343dd0 | pre-verify | squash delta (contains hop-map) |
| cam/crazy-guards | TOOLPATH-QUALITY | 8faf6aa1 | pre-verify | per-test 15 s ruling |
| cam/crazy-2d | CAM-2D-FINISH | a5399977 | held | main merge after memfns, then pre-verify; on crazy-guards |
| cam/speed | CAM-SPEED | 3a3fe6d0 | pre-verify | GREEN-EXCEPT-RENDER; VERIFY-RENDER running |
| alloc/threaded-stress | CAM-PERF | 6cb5d0f6 | queued (train 8) | GREEN, gate judged |
| tests/faster-slow | FAST-VERIFY | 75659407 | pre-verify | band on train (removal.rs) |
| kernel-stress | KERNEL-STRESS | 12c7d280 | pre-verify | body.rs revert verified |
| op-budget / mass-cap / face-box | KERNEL-STRESS | d023bee7 / e1fa7285 / 56e0c452 | pre-verify | |
| import/speed-ks | IMPORT-SPEED | ffecc5b6+ | held | UX: the drawing of a bad face needs marking (Issues + amber outline + shots); squash after kernel-stress |
| fix/nurbs-stall | FALLBACK | da29417d | held | pre-verify on t8 base after memfns; carries torus-seam |
| fix/far-line-torus | — | 8231923b | held | right after nurbs-stall |
| fix/asm-shape-first | HOSTED-DESIGN | 1af8e6da | pre-verify | 3a73019c was a false GREEN (E0063), retracted |
| proof/curved-sites-screw-flip | CURVED-SITES | c1042ae5 | pre-verify | UX OK |
| FLAT-SITES series (12 patches) | TRAIN-CONDUCTOR | — | held | I rebase it after memfns; patches in my workspace, trees verified |
| exact-core/ties | EXACT-CORE | e01c8e30 | held | real.rs add/add vs main; resolve on the memfns merge; old GREEN unconfirmed |
| parasolid/xt-reader | PARASOLID | 5b7c742f / 37712623 | held | UX OK; main merge after memfns |
| sheet/near-limit | (owner) | dd48635e | pre-verify | main merge after memfns |
| bendsim/showcase | BENDSIM | c8e8b058 | held | UX OK; SQUASH only (flagged history); supersedes bendsim/next 8c2d8aa5 |
| claude/lang-det | LANG-PROOFS | cb88d331 | held | main merge after memfns + new tool, then VERIFY-RENDER; security wording only |
| fix/overlap-interference-speed | — | — | dropped | remote kept for David |
| machines/models-cli, builder-off-reasons | — | — | not riding | already in main |

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

Status: the series (all but subdivide-reuse) goes in train 10, rebased by the conductor onto train 9's main. subdivide-reuse goes in the speed train after gate-memfns (a).
