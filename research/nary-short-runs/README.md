# Short n-ary runs go sequential

`kernel/forge-model/src/lib.rs`, `SHORT_RUN` and `short` (and `bodies.rs`, `Run::flush`): a run
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
cp research/nary-short-runs/nx.rs kernel/forge-model/examples/
fbuild cargo build --release -p forge-model --example nx
sh research/nary-short-runs/cross.sh $PWD/target/release/examples/nx out.txt
python3 research/nary-short-runs/crosssum.py out.txt
sh research/nary-short-runs/wall.sh wall.txt       # with SHORT_RUN at 8, so 12 and 16 run n-ary
```
(`nx.rs` is not kept in examples.)

## Tried and dropped (nary-speed-7)

- Runs holding a pattern, a thread or a text handed off too when they have at most `SHORT_RUN`
  tools: conrod's n-ary cost what seq does, but heatsink +23.7%, manifold +0.9%, quadarm and
  bracket +1.3% (instructions:u, nary1). Kept as it was.
- A thread cut into a bore (on a hole, or an internal one on an axis) starting a run of its own,
  threads right after it sharing it, as in the sequential regen: nary1 instructions:u conrod
  -21% (to seq's), valve -6%, frame -3%, gearbox -2%; but the thread's pass has one or two tool
  pairs, so it runs on one thread, and wall time at 4 threads went up (best of three, load ~7:
  valve 69 -> 99 ms, frame 395 -> 452, gearbox 104 -> 113, conrod 33 -> 41). Dropped for the
  cause it worked round: a tapped hole's mouth lying on the bottom of the clearance hole above it,
  which the n-ary pass now skips as a face that bounds nothing (`Shape::plane_in`).
- In the n-ary pass's bookkeeping: `coincident` taking the jobs' pieces with repeats and sorting
  once (+0.9-1.4%: more to sort than the two sorts it saved); the outside-the-result box test asking
  the grid for nearby tools (+3-8%: the grid's list for a big box costs more than the walk);
  settling a new vertex against each edge through it once rather than once per cut found on it
  (+0.4% on housing and torture, and changed bits).
