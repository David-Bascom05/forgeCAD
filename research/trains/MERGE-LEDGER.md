# Merge ledger

Kept by TRAIN-CONDUCTOR. One row per branch. The status values are: queued, pre-verify, in train N, merged, bounced (with the reason), and held (with what it needs).
Rules: each item needs a GREEN pre-verify on the train's main, made with tools/preverify ≥13d78f06 or proven clean by a record check or my own test build. UI changes need UX-LEAD's OK. Branches with the flagged history land squashed.
This branch (coord/trains) is never merged into main.

Main: 247a2694 (train 10, #64). Before it: d6820393 (train 9, #62), 5767cdb4 (gate-memfns alone, #63), 10d87778 (train 8, #61).
Train 11 (merge-train/11) is in gates, on top of 247a2694.
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
| perf/intcurve-eval-memo | FALLBACK | a7d773c5 | in train 11 | 2nd review SAFE; comment fixes + hidden-input row to come |
| perf/varblend-rail-uv | FALLBACK | 55d9d68d | in train 11 | review SAFE; STEP pcurves of vary-blend springs may move last bits |
| flat/subdivide-reuse + FLAT-SITES series | FLAT-SITES (pushed by me) | see below | in train 11 | flush-per-plane, open-shell, cone-apex, check-chords, g11-series, accum-margin, whole-turn |
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
| cam/adaptive-ties, cam/adaptive-pass-start | CAM-CORE | edc2f3d8, b3dacc6a | pre-verify | |
| hosted/server-hardening, client-native, client-web | HOSTED | 07a687db, 1bc1e103, deafa8fb | pre-verify | client UI OK given |
| import/speed-ks, import/faces | IMPORT-SPEED | e16c813a, 0a85e052 | pre-verify | after kernel-stress |
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

Status: the series (all but subdivide-reuse) goes in train 10, rebased by the conductor onto train 9's main. subdivide-reuse goes in the speed train after gate-memfns (a).
