# The one tolerance

`crates/forge-math/src/tol.rs`.

## Derived

- `MODEL_HALF_SIZE` = 500 m (a 1 km cube): a design bound. f64 spacing there is ~1.2e-10 mm, so
  `LINEAR` = 1e-5 mm (10 nm) leaves about five orders of magnitude for computed error at the far
  corner.
- `ANGULAR` = LINEAR / 1 km = 1e-11 rad, so an angle error never moves a point inside the model
  by more than LINEAR.

## Measured

`MIN_LENGTH = LINEAR` and dropping the 10 * LINEAR bands in the booleans (b842031), then bounding
tolerant vertices and the checker's side probe (ee4cda0), were decided on the fuzz histories and
the adversarial hunt:

```
b842031  fuzz 150: seq 145 -> 146, nary 145 -> 147;  nhunt 40: seq 520 -> 532, nary 528 -> 532
ee4cda0  fuzz 150: seq 146 -> 150;                    nhunt 40: seq 532 -> 553, nary 532 -> 541
```

## How to rerun

```
FUZZ_SEEDS=150 fbuild cargo run --release -p forge-model --example suite
fbuild cargo run --release -p forge-model --example nhunt -- 40
```
Counts on today's main are higher (later fixes); to compare the choice itself, change the
constant or band and rerun both on the same commit.
