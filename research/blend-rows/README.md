# Variable blends: how close the rows are kept

`kernel/forge-geom/src/varblend.rs`, `predict`: a VarBlend keeps rows of solved ball centres along
its guide, each predicted by the last within `predict` = a thousandth of the smallest radius
(5123c9e; it was a ten-thousandth). Rows only seed the exact solves and bound searches, so the
spacing moves cost, never the surface: every point is still solved to rounding.

## Measured

One of the speed changes in 5123c9e, measured together: `random_fillets` 50 s -> 1.9 s, with
instruction counts per eval, projection and implicit from `examples/varblend` (79ecbe8). This
knob's own share was not recorded.

## How to rerun

```
fbuild cargo build --release -p forge-geom --example varblend
perf stat -e instructions:u ./target/release/examples/varblend project 1000   # minus `build 0`
fbuild cargo test --release -p forge-model --lib random_fillets
```
To isolate the knob, set the 1e-3 back to 1e-4 and compare both.
