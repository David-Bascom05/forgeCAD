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

## The user-latency gate

The instruction gate counts instructions on one core under valgrind. It cannot see threads: they
are serialised there, and the instructions they add count as a cost even when the user waits
less. It also measures instructions, not time. `latency.sh` measures what the user waits for:
wall-clock time, and beside it the CPU time (user+sys of every thread) the wait cost.

| file | what |
|---|---|
| `latency.sh` | the latency gate: head against a base built in the same session, or head alone (`record`), or the results at 1, 2 and N threads (`bits`) |
| `latstat.py` | its statistics (stdlib only): paired rounds, bootstrap CIs, signed-rank p, the verdicts |
| `../../interface/forge-lang/examples/latbench.rs` | the harness: one round of the ops on one part, each sample `OP REP WALL_NS CPU_NS` |
| `latency-baseline.txt` | the integration branch's latency on the container it was measured on: a record, not a comparison point |

### What a line is

A line is PART, THREADS, OP. THREADS is `t1` (FORGECAD_THREADS=1, pinned to one core), `tN`
(every core, FORGECAD_THREADS unset: what a user gets), or `tK` with `--threads` (FORGECAD_THREADS=K
on every core, e.g. `t16` as regen1's nary16). The parts are the gate's bench parts. The ops, each the user-facing operation:

| op | what the user did |
|---|---|
| `open` | opened the part: the cold build with checks, as `forgecad build` (no disk cache) |
| `edit` | changed one dimension: the rebuild from the open's cache, a new value every rep |
| `step` | exported STEP (AP214) |
| `mesh` | exported 3MF, each solid checked watertight |
| `draw` | drew four views (front, top, right, iso): hidden-line removal |
| `nary` | the kernel's n-ary boolean regen of the part's features, at the process's thread count |
| `k-NAME kseq`, `k-NAME nary` | the kernel suite part NAME (regen1's parts: coplanar, upto, shaft, mixed, patterns, plate) regenerated alone, sequentially and n-ary at the thread count (`--kparts`) |
| `cam exec` | the CAM certify go/no-go, a whole process (only where the checkout has `forge-cam-lang`) |

`draw` is measured on the parts whose four views take under about a second (`LAT_DRAW_PARTS` in
`latency.sh`); on the others it takes 2 to 33 s at all cores (`--heavy` adds them).

### How it is measured

- Base and head are built in the same session, with the normal release profile, each in its own
  target dir (the base from a git worktree, with head's `latbench.rs`, so both run the same ops).
  A wall-clock number from another day or machine is not comparable, so the gate never compares
  against a saved number: every comparison is A/B now.
- Every run under `env -i` with `FORGECAD_CACHE=off`, from `/`, the parts read from one folder.
  `t1` runs are pinned (`taskset -c $LAT_CORE`, default the last core); `tN` runs get every core.
- A warm-up pass over every part on both sides is discarded. Then rounds: per part and threads,
  a base process and a head process back to back, the order flipped every round (AB BA AB ..),
  so drift (other tenants, clocks) falls on both sides of a pair alike.
- In a process, each op runs once untimed, then up to `--reps` times (stopping once its reps
  took `LATBENCH_BUDGET` s); the round's value is the median of its reps (CPU: their mean, as
  `/proc/self/stat` gives it in 10 ms ticks). The round, not the rep, is the unit: a process's
  heap layout and placement are part of the noise a user sees, so reps in one process are not
  independent samples.
- Per line: the paired ratios head/base over the rounds; the estimate is their median, with a
  99% percentile bootstrap CI (5000 resamples of the pairs) and a Wilcoxon signed-rank p.

### Verdicts

- `REGRESSED`: the wall CI lies wholly above 1, the estimate is over **+1%**, and p < 0.01. Both
  tests must agree and the effect must be larger than the threshold: a container's noise does not
  fail a change, and a real 1% does not hide behind "significant but tiny".
- `IMPROVED`: the mirror.
- `cpu+` / `cpu-`: the same test on CPU time against **3%** (CPU is quantised to 10 ms, so a
  wider threshold). A warning, not a failure: it marks a change that keeps more cores busy for the
  same wait, or for less.

### How a change passes: wall-clock is the gate

The owner: "I care about speed. If flat bool can be made faster with the same number of threads I
don't really care how many instructions it is. If it's just faster because of better threading in
that algo it should win." And on fairness: "the comparison has to run on equal thread counts. 16
vs single thread is not fair, unless one algo is inherently limited on taking advantage of multi
threading." And: "If it's 16 threads vs 1 and 1 is winning still take the 1. Fastest wins."

The base rule stays: **nothing user-facing gets slower**. A change passes when, from
`latency.sh --base BASE`:

1. **No line is `REGRESSED`** in wall-clock, at `t1` (head at one thread against base at one
   thread) and at `tN` (head at all cores against base at all cores). Head and base are only ever
   compared at the same thread setting, N the same on both sides and printed in the report; never
   head at N against base at 1. A base whose algorithm is sequential still runs at N threads in
   the `tN` comparison and simply does not gain: that is the fair case.
2. **No new `THREADS HURT`**: on head, every line at all cores (the default, what a user gets)
   is not slower than the same build at one thread, by the same test. Fastest wins: where
   threads make a line slower, the code stays serial there (an adaptive start, a cutoff). A line
   that is so on base too is listed `(base too)` and does not fail the change, but is owed a fix.
3. **Bits**: every result identical to `bits.txt`, native the same as wasm (`wasm.sh`), and no
   result depends on the thread count: `latency.sh bits` regenerates every part (seq and n-ary at
   the thread count, and the drawing's views) at FORGECAD_THREADS=1, 2 and N, and runs the gate's
   bit suites at each; all must be identical.

A change that is faster only through better threading passes. The tN/t1 speed-up per line
(`record`) is information, never a pass.

**The instruction gate is diagnostic** (`gate.sh`): reported with every change, an early
warning (it is exact, so it sees a 0.1% single-thread change the wall clock cannot), and the
place to find *where* a change costs (`phases.sh`); it never fails a change. `cpu+` from the
latency gate (more CPU burnt for the same or less wait) is a warning, stated with the change.

### Validation

`latency.sh --selftest` is not a mode: the knobs are `HEAD_ENV` on a `--base self` run:

- `HEAD_ENV="LATBENCH_SLOW=jack:open:0.02,heatsink:edit:0.02"`: head spins until 1.02x on
  those two lines: both must be `REGRESSED`, nothing else;
- `HEAD_ENV="LATBENCH_PARVIEWS=conrod,heatsink"`: head draws those parts' four views on all
  threads at once (the same views): `draw tN` `IMPROVED` with `cpu`-busy, `draw t1` not
  `REGRESSED`;
- `--base self` three times: no line flagged.

The results are in `latency-validation.txt`.
