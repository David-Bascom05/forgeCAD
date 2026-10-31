# When Auto uses the GPU

`crates/forge-gpu/src/lib.rs`, `AutoBatch`: the n-ary regen's region batch on the CPU or the GPU.

## Measured

- **The crossover**: d0c75dc measured it once with `bench.rs` (kept here exactly as it was;
  RTX 5060 Laptop, 4 threads): the GPU's round trip is ~85 µs up to a few hundred points and it won
  from 256 points on every suite part (torture: 128 points cpu 75 / gpu 83 µs, 256 points cpu 175 /
  gpu 85). That made `CROSSOVER = 256`. c23af9e replaced the constant with a probe per process
  (`probe`: both timed on the batch's points at sizes doubling from 32); `examples/backends` prints
  the crossover the probe found.
- **128 points before a probe** (`crossover().unwrap_or(128)`, bd69b3a): below 128 the CPU won on
  every part in that measurement, so smaller batches never count toward opening the GPU.
- **OPEN_COST = 0.3 s**: what opening the GPU costs a process. `open_cost.rs` (recreated
  2026-11-01, the original timing was not kept) measured 160-350 ms, median ~250 ms, over five cold
  processes. Auto opens the GPU once the CPU time spent in batches it could take reaches that cost,
  so it never spends more than twice the better choice (ski-rental break-even). Checked end to end
  with `LOOP=2000 backends`: Auto opened near regen 550 and the batch call dropped ~470 -> ~330 µs.

## Design, not measured

The single-precision pass answers only points clear of every boundary by 1e-3, and the double
pass's margin is tol::LINEAR a hair wider than the CPU's (`MARGIN`); `crates/forge-model/tests/gpu.rs`
proves the verdicts equal the CPU's on every suite part plus 20k random points.

## How to rerun

```
cp research/gpu-auto/open_cost.rs crates/forge-model/examples/
fbuild cargo build --release -p forge-model --example open_cost && ./target/release/examples/open_cost 5
LOOP=2000 ./target/release/examples/backends       # after building the backends example
```
`bench.rs` builds only at d0c75dc, where it was `crates/forge-gpu/examples/bench.rs`
(`cargo run --release -p forge-gpu --example bench`); `AutoBatch::new` has changed since. All need
a CUDA GPU.
