# tools/speed: the speed gate and the boolean's profile

| file | what |
|---|---|
| `gate.sh` | the gate: instructions per regen of each part on each path, against a baseline (REGRESSED: over 0.5%), and every regen's bits against `bits.txt` |
| `exact.sh` | the exact count's conditions (sourced by `gate.sh` under callgrind, `phases.sh`, `lines.sh`) |
| `fixrand.c` | `getrandom` as a fixed stream (LD_PRELOAD, measurement only) |
| `phases.sh` | the boolean's instructions per phase on every gate line, switch on and off, with a flame profile and call tree per line |
| `cgfold.py` | reads a callgrind out file: inclusive Ir per function, folded stacks, a call tree |
| `lines.sh` | a few gate lines, counted as the gate counts them |
| `baseline-valgrind.txt` | the callgrind baseline (exact) |
| `baseline.txt` | the perf baseline (not exact: perf mode is unchanged) |

## The exact count

`GATE_EXACT=1 tools/speed/gate.sh` (or `GATE_COUNTER=callgrind`) counts with valgrind's callgrind
on the x86-64-v2 build, under `exact.sh`. There a count depends on the code alone: the same binary
gives the same count on every run, bench parts included, and two builds that compile to the same
code count the same, to the instruction. What used to move a count with no code change:

| cause | effect | fix in `exact.sh` |
|---|---|---|
| forge-lang's `build.rs` bakes `FORGE_KERNEL_HASH`, a hash of every kernel source file, into every result-cache key | any kernel edit (a comment, a module behind a feature that is off) moved map orders and so the bench lines' counts, by −55 to +525 | `FORGE_KERNEL_HASH_PIN` (honoured by `build.rs`), `FORGECAD_CACHE=off` so a pinned build never reads a disk cache |
| std's `HashMap` seeds come from `getrandom` | bench lines moved by up to ~0.4% run to run | `fixrand.so` preloaded |
| worker threads (`par`, fillet, forge-asm) interleave their allocations in glibc's malloc | the same | `taskset -c $GATE_CORE` (default 0): one core, `available_parallelism` = 1 |
| reading an environment variable scans the whole environment | the caller's own variables moved every count by hundreds | `env -i` with a fixed `PATH` |

Checked: base 9e78b283 and the decide-once branch with the switch off, built at the same path, have
byte-identical `.text`, `.rodata` and `.data` (regen1, benchparts); the gate gives 0 difference on
every line. Two gates can run side by side on different cores (`GATE_CORE=1`).

The exact baseline is not comparable with a count made without `exact.sh` (one core, fixed seeds):
re-save it (`GATE_EXACT=1 tools/speed/gate.sh --save`) from the code it is the baseline of.

## The switch

`GATE_FEATURES=forge-bool/decide_once` builds with the decide-once switch on (in
`target/callgrind-on`) and compares against the same baseline: the switch against the old path.

## Per phase

`tools/speed/phases.sh [OUT] [PART MODE ..]` (`PHASES_SWITCH=on|off|both`): per gate line the total,
the boolean (the old path's entries, decide-once inside them) and decide-once's phases:
arrangement, sample, ledger, core, write, rest. Each phase is an `#[inline(never)]` function of
`kernel/forge-bool/src/decide_once.rs`. Per line it writes `NAME.cg`, `NAME.folded` (flamegraph.pl's
input) and `NAME.tree`. `python3 tools/speed/cgfold.py F.cg tree FN 4 1` prints the call tree under
`FN`; `incl FN..` its inclusive Ir.
