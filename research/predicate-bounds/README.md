# Float filters of the exact predicates

`crates/forge-math/src/predicates.rs` evaluates each predicate in f64 and returns that value when
its error bound proves the sign; otherwise it recomputes exactly with expansions.

## Where each bound comes from

- `CCW_BOUND`, `ICC_BOUND`, `CCW_BOUND_B`, `ICC_BOUND_B`, `SPLITTER`: Shewchuk, "Adaptive
  Precision Floating-Point Arithmetic and Fast Robust Geometric Predicates" (1997), as published.
- `CROSS_BOUND`, `DOT3_BOUND`, `TRIPLE_BOUND` (90902ba): derived here by forward error analysis,
  one rounding per operation, unit roundoff e = 2^-53 (no FMA anywhere in the kernel):
  - a*b - c*d: l = ab(1 + d1), r = cd(1 + d2), det = (l - r)(1 + d3), |di| <= e. The error is at
    most e|ab| + e|cd| + e|l - r| <= (e/(1 - e) + e)(|l| + |r|) = (2 + O(e)) e (|l| + |r|).
  - x + y + z of three products: e per product plus e per sum on at most the sum of magnitudes:
    (3 + O(e)) e (|x| + |y| + |z|).
  - a . (b x c): each component's cross is 2e of its |yz| + |zy|, the multiply by a_i one more e,
    the two sums 2e: (5 + O(e)) e times the permanent.

  The O(e) terms (8e, 32e, 64e) are generous slack for the rounding of the magnitude sums and
  the bound's own product, as Shewchuk does. The derivation was not written down at the time;
  this is a reconstruction.

## How to rerun

```
python3 research/predicate-bounds/check.py
```
Tests each filter against exact rational arithmetic on 200k random and near-cancelling inputs
per predicate and prints the worst error as a fraction of its bound (2026-11-01: cross 0.984,
dot3 0.873, triple 0.691; the cross bound is tight).
