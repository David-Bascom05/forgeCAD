# fix-tidy gate evidence

Head merged with claude/repo-review-mbdrlj f636156. Speed: `GATE_COUNTER=callgrind tools/speed/gate.sh`
against a baseline saved from f636156 in a worktree at a path of the same length.

- `gate.txt`: every line, against base.
- `speed-diff.txt`: per-function callgrind self cost, this head against f636156, under the gate's own
  conditions (binaries copied to one /tmp/forge-gate.XXXXXX path, `env -i`, fixedrandom.so,
  `taskset`, `setarch -R`, `--fair-sched=yes`), for every line that was over base before the
  clawback. Functions of unify.rs are summed into one row (their names differ between the two
  builds). "kernel functions above base" lists each forge function whose own count rose.
