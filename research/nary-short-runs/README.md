# Short n-ary runs go sequential

`crates/forge-model/src/lib.rs`, `SHORT_RUN` and `short` (and `bodies.rs`, `Run::flush`): a run
of at most sixteen features of one tool each is applied with the sequential regen's booleans.

## Measured

Instructions:u of one regen of `suite::fuzz(seed, len)` on each path, seeds 1-8, by `cross.sh`
(through `fbuild`, `FORGE_BATCH=cpu`) with `nx.rs`; `crosssum.py` prints n-ary / seq per length.

- `cross-main.txt`: main at 75003a3, before the hand-off. N-ary cost 13% more than seq at 2, 4
  and 8 tools (median; worst seed 1.21-1.24x), 10% at 12, 7% at 16, about as much from 24 on
  (median 0.99-1.04, summed 0.93-0.97).
- `cross-short-runs.txt`: with the hand-off at 8. Lengths 1-8 cost what seq does (median
  1.000-1.002, the rest is reading FORGE_BATCH once); 12 on unchanged.

The hand-off started at 8 tools (the coordinator's first decision, handoffs/nary-speed-5.md);
12 and 16 tools were still dearer n-ary in instructions on every seed but one or two. The rule
then set was wall time: keep n-ary there only if 4 threads beat seq. `wall-12-16.txt` (by
`wall.sh` with `nx.rs`'s `wall`: best of 25 regens, the first column the load average, 13 on 16
cores) says they never do: at these sizes (0.2-2.7 ms) the threads cost more than they share,
n-ary at 2 and 4 threads ran 1.3-5x seq's time on every seed, and n-ary at 1 thread 0.8-1.35x.
So `SHORT_RUN` is 16: 12 and 16 tools now cost what seq does (1.000, `cross-16.txt`), 24 on
are unchanged.
The one-tool-a-feature condition was measured on the bench parts: heatsink's run after its
fillet (a slot, a pattern of three, a pocket) cost 2073M n-ary and 2572M handed off, because the
sequential regen takes a pattern's copies in an n-ary pass over the whole part anyway.

## How to rerun

```
cp research/nary-short-runs/nx.rs crates/forge-model/examples/
fbuild cargo build --release -p forge-model --example nx
sh research/nary-short-runs/cross.sh $PWD/target/release/examples/nx out.txt
python3 research/nary-short-runs/crosssum.py out.txt
sh research/nary-short-runs/wall.sh wall.txt       # with SHORT_RUN at 8, so 12 and 16 run n-ary
```
(`nx.rs` is not kept in examples.)
