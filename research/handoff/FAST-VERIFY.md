# FAST-VERIFY handoff (merge-speed infrastructure)

What exists, where it is, how to use it, and what's still open. Measurements and evidence are in
research/fast-verify/PHASE1.md and PHASE2.md (branch fast-verify/phase1).

## Tools (branch tools/preverify; never merged to main; owners fetch it)

| file | what |
|---|---|
| run.py | the owner's pre-verify. One command from a branch checkout after merging main. Prints `PRE-VERIFY GREEN/RED/GREEN-EXCEPT-RENDER <sha> on main <sha>: steps…; tool <sha>` |
| affected.py | which packages a diff reaches: Cargo graph, reads map, Cargo.lock, global files; whether the gate is reached |
| readsmap.py | on main: strace of every test binary, giving the repo files each package's tests read (about 12 min warm) |
| stage.py | one train stage on an exact sha, whole, for the verifier sessions (native / render / wasm / clippy / guards / gate) |
| reparent.sh | pipelined trains: train N+1's verified tree as one new commit on main after N lands (refused unless main's tree is N's) |
| pvitems.py | the log parser and `stays_red()` (one place decides what may never be re-judged) |
| test_pvitems.py | regression tests: compile-error and no-test failures stay RED |
| publish.py | data to preverify/maps. Unused: pushing it needs David's own permission, and it was never given |

Owner command:
`git worktree remove --force /tmp/pv; git worktree prune; git fetch origin tools/preverify && git worktree add /tmp/pv FETCH_HEAD && python3 /tmp/pv/tools/preverify/run.py`

### Judgement rules (agreed with the coordinator and TRAIN-CONDUCTOR)
- Patent grep, guards, removed_checks, model-licences and every_hidden_input_is_reviewed run whole for every branch.
- Clippy, native, render (release, as CI's gpu job) and wasm run on the selected packages, plus the gate when its binaries can change.
- A failed step is re-judged against main (the same packages, built in a separate target: <cache>/base-target):
  - main has the same failure → "main-identical", not RED;
  - a build or compile error, a failure no test line accounts for, or no disk → RED, never re-judged;
  - time budget: 2 s per test (15 s for `corpus_guard_*`). An overrun in a package the branch doesn't change is a warning. One in a package it does change is RED if new (over 2× the budget alone), or if over 20% slower than main's same test when both are timed back to back.
- Render with no Vulkan driver, or no wasm32-unknown-unknown target, is NOT RUN (GREEN-EXCEPT-RENDER). TRAIN-CONDUCTOR asks VERIFY-RENDER to run that sha.
- Accept a PRE-VERIFY line only if it ends "tool <sha>" (b9318989 or later). Older lines predate the false-GREEN fixes.

## Verifier sessions (sharding, step B)
- VERIFY-NATIVE session_01WJtawxtF4mNnfyqiqgAFuk
- VERIFY-RENDER session_01WJtawyot9sDyxbN8Z7HzFB
- VERIFY-WASM session_01WJtawxTXmP6VqVjazrKuzx
Each takes `verify <40-hex sha>` from TRAIN-CONDUCTOR, runs its stage whole in a warm worktree, and replies
`VERIFY <stage> <sha> exit N <s> [toolchain] failed: …`. None of them publishes anything.
Measured on train 7's head: identical verdicts both ways, about 28 min in parallel (near-cold) against about 77 min serial.

## Speed gate (tools/speed, FALLBACK's)
- tools/gate-jobs-3 (GATE_JOBS, GATE_MEMO) and tools/gate-src-sync-2 (sync the build source instead of re-extracting it): approved by
  FALLBACK. They land after FALLBACK's gate-memfns re-baseline, rebased onto it, with the S/4/3/memo identity table. On one VM:
  serial 391–407 s, 4 workers 119 s, memo hit 1 s, all byte-identical.

## Open items
- gate-memfns (FALLBACK) → then rebase and land gate-jobs-3 and gate-src-sync-2.
- forge-sheet contour fuzz_random_profiles: one case of 250 takes 322 s (the folded-clash search refines an
  at-the-limit stretch down to 2^-48). Diagnosed and with forge-sheet's owner, via the coordinator.
- tests/faster-slow (font race fix, reparse 2×, mill_with in parallel with bit-identical results) rides train 10.
- Published reads maps and main results (preverify/maps) would make pre-verify narrower and faster, but need David's
  permission for a session to push data there.
- Pipelined trains (D): mechanics agreed, reparent.sh ready, starting from 8→9.
