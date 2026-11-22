# Float verification: the kernel's sign predicates, through Aeneas, against the IEEE 754 model

Part 2 of the IEEE 754 work. Part 1 is the model, `verify/ieee754`, with
`verify/ieee754/RESULTS.md`. The files of this part are in `float-verification/`:

| | |
|---|---|
| `predicates/` | `cross_sign` and `orient2d` with all their helpers, copied from `kernel/forge-math/src/predicates.rs` and spelled for Aeneas. Its tests check it against the kernel bit for bit. |
| `gen.sh` | Charon → Aeneas → `lean/FvPredicates/{Types,Funs}.lean` |
| `lean/FvPredicates/` | the translation, plus `F64.lean` and `FunsExternal.lean`, which bind `f64` to the model |
| `lean/FloatVerification/` | the theorems |
| `check_exact.py`, `predicates/examples/dump.rs` | the kernel's exact paths against exact rationals |

## The kernel fix (`kernel/forge-math/src/predicates.rs`)

The findings below showed that Shewchuk's bounds, and the expansion arithmetic of the exact
paths, assume no underflow and no overflow. This is the fix as decided: option (1) of the
discussion, after a first version that made every predicate exact for every finite input but
cost up to +0.14% on the deterministic speed-gate lines.

1. **`orient2d` and `dot3` fast filters: an absolute slack.** `bound = BOUND · detsum + SLACK`,
   with `SLACK = 16 · 2^-1074`: one add, no comparison.
   - It covers the absolute rounding error, up to `2^-1075`, of a subnormal product, so a fast
     decision is right for every finite input (`orient2d_fast`, proven).
   - `cross_sign`'s filter needs nothing: it is sound by monotonicity (proven).
   - The slack changes no decision unless the bound is below about `2^-1017`, and never a
     returned value.
2. **The window is each predicate's precondition, checked by `debug_assert!`.**
   - **Windows.** The expansion arithmetic is exact when every nonzero input lies in the
     predicate's window: `[2^-480, 2^500]` for `cross_sign`, `orient2d` and `dot3`,
     `[2^-300, 2^330]` for `triple`, `[2^-210, 2^250]` for `incircle` and
     `certainly_in_circle`.
   - **Why exact.** Inside the window all inputs are multiples of `2^(lo−52)`. So every product
     of `deg` of them is a multiple of `2^-1074`, and every rounding error is representable,
     even when subnormal. Nothing reaches `2^996` (Veltkamp's split) or overflows.
   - **Where the asserts are.** For the degree-2 predicates the assert is on the slow path only,
     since a fast decision needs no window.
   - **The amplified predicates.** `triple`, `incircle` and `certainly_in_circle` multiply
     rounded products, which amplifies an absolute error. For them the window is the whole
     predicate's precondition, asserted on entry. Inside it no product's rounding error is
     absolute, so their original filters are sound without slack.
   - **Release builds.** The asserts compile away, so outside the windows release builds are
     unchecked, as before the fix. For example `cross_sign(1.4u, u, 1.3u, u)`, `u = 2^-537`,
     still answers 0.
3. **NaN fallback, cold path only.** A slow-path result that is NaN, because a product
   overflowed, is recomputed exactly with fixed-width integers (`Big<N>`, 68 to 136 limbs). A
   double is an integer times `2^-1074`, so each determinant is an integer polynomial. Overflow
   now gives the exact sign, with magnitude clamped to `[2^-1074, f64::MAX]`, instead of NaN.
   On the measured parts this check costs nothing (below).
4. **Tests in predicates.rs:**
   - `lattice_at_every_scale` and `degree2_at_window_edges`: near-degenerate integer
     configurations for all five predicates. The scales run from `2^-210` to `2^228` (inside
     every window), and from `2^-480` to `2^478` for the degree-2 ones. Exact signs come from
     i128.
   - Release-only, since outside the windows: `overflow_is_exact` (the overflow cases, including
     the mixed-scale `orient2d((1e300, 1e300), (2e300, 2e300), (0, 1e-300)) > 0`), and
     `orient2d_underflow_counterexample` (the fast filter no longer answers the wrong sign;
     it now gives 0).

Gates: pending (the 5-run A/B speed gate and the debug-assertions survey are running; recorded in the next commit).

## What is proven

The proofs are about the Aeneas translation of the copy, with `f64` as the model's binary64. They
use only `propext`, `Classical.choice` and `Quot.sound`. `FloatVerification/Axioms.lean`
fails the build otherwise.

1. **`cross_sign_fast`.** Take any finite doubles `a b c d`. Suppose the fast test of
   `cross_sign(a, b, c, d)` passes, that is `|l − r| > CROSS_BOUND·(|l| + |r|)` with `l = a*b`
   and `r = c*d` rounded. Then `cross_sign` returns `l − r`, and it is finite, nonzero and has
   the sign of the exact `a·b − c·d`.
   - No range hypothesis is needed. Rounding is monotone (`round_mono`), so the order of `l` and
     `r` is never the wrong way round. A rounded difference has the sign of the exact difference
     or is zero.
   - The test only has to exclude zero, NaN and overflow, and the proof shows it does. So
     `CROSS_BOUND` matters for speed, not soundness.
2. **`orient2d_fast`, with no hypothesis.** Take any finite coordinates. If the fixed fast test
   `det > bound || −det > bound` passes, with `bound = CCW_BOUND·detsum + SLACK`, then
   `orient2d` returns `det`, finite, nonzero and with the sign of the exact determinant of the
   input doubles.
   - The core is `shewchuk`, over ℚ. Each rounding error is relative to its computed value,
     plus `η = 2^-1075` for the three products, which may be subnormal. The errors come from the
     model's `sub_err_res`, `add_err_res` and `mul_err_abs`.
   - Shewchuk's `(3 + 16ε)ε` absorbs the relative part, and `SLACK = 32η` absorbs the absolute
     part. The constants are checked by `norm_num`.
   - Overflow fails the test (`finite_of_bound_slack`).
   - The fix is what removes the earlier no-underflow hypothesis.
3. **The old test was unsound.** `orient2d_cex` evaluates the counterexample's three facts by
   `native_decide`, the one result that relies on the compiler, and nothing depends on it:
   - the test without `SLACK` passes with `det < 0` though the exact determinant is positive;
   - a product underflows;
   - the fixed test does not pass.

## Which kernel functions are verified

- The fast path of `forge_math::predicates::cross_sign`, for all finite inputs. `cross2` is the
  same function.
- The fast path of `forge_math::predicates::orient2d`, for all finite inputs.

The proofs are about the copy in `predicates/`. That the copy equals the kernel is tested, not
proven: `matches_the_kernel` compares the two, bit for bit, on 2,000,000 random and
near-degenerate inputs at scales from 2^-1000 to 2^1000, and they agree.

The exact paths are translated (`cross_exact`, `sum_into`, `estimate`, `orient2d_adapt`,
`orient2d_exact`, `cross_two`, `scale_into`) and bound to the model. They are **not proven**.
Their theorem would be stated under the window hypothesis. The integer fallback for NaN results
(`big::*`, verbatim from the kernel) is kept opaque and given no meaning in Lean: it is defined
to fail, so no theorem can claim anything about the inputs that reach it. These paths are tested
instead: the lattice tests, and `check_exact.py`.

## Findings about the kernel (fixed)

- **`orient2d` answered a wrong sign when the products underflowed.** The inputs are
  `a = (5.394455326111622e-162, 1.6245062719943995e-164)`,
  `b = (1.3762009331530288e-157, 4.1443425003634566e-160)` and
  `c = (-4.608837897596252e-178, 0)`. The exact determinant is positive; the old code returned
  `-5e-324` from its fast path.
  - The two rounded products are subnormal and straddle a rounding midpoint in the wrong order.
  - `bound` underflowed to 0, so the test passed.
  - The same flaw was in `dot3`, `triple`, `incircle` and `certainly_in_circle`.
- **The exact paths answer 0 for nonzero determinants under underflow.** This is outside
  the windows, so it is now a precondition violation, not fixed. Examples:
  - `cross_sign(1.4u, u, 1.3u, u)` with `u = 2^-537`;
  - `orient2d((3t, 5t), (5t, 8t), (0, 0))` with `t = 2^-540`.

  `two_product`'s error term is not representable when the product underflows.
- **They answered NaN when a product overflowed.** For example
  `cross_sign(1e300, 1e10, 1e300, 1e9)`. Callers read NaN as "collinear". This is now fixed by
  the NaN fallback.
- The fast-filter underflow (`orient2d`, `dot3`) is fixed for every finite input.

## What blocked, and what kernel/ would need

Aeneas (2e3d448) and Charon (9e193f0) translate the copy completely: no `sorry` and no admitted
definition. But they needed these changes, and kernel/ would need the same to be translated as
written:

1. **`f64` arithmetic and comparison operators**: Aeneas fails with "Invalid inputs for binop" on
   `a + b`, `a * b`, `a > b` and the like for floats. Calls of the trait impls (`a.add(b)`,
   `a.gt(&b)`, `a.ne(&b)`) are translated as opaque functions, so the copy writes every
   operator that way. The same arithmetic in the same order: `matches_the_kernel` checks bit
   equality.
2. **Float literals**: Aeneas cannot extract them ("Float … literals are unsupported"). The copy
   writes `f64::from_bits(0x…)` and binds `from_bits` to the model's `ofBits`.
3. **Closures and iterators**: `sum_into`'s `next` closure is inlined. `for` loops over ranges
   and slices become `while` loops over an index. `e.iter().sum()` becomes an explicit fold from
   `-0.0`, as `Sum for f64` does. `e.last().copied().unwrap_or(0.0)` and `exact_diffs`
   (const-generic, `iter_mut().zip`) are written out.
4. **`Point2`** is a local struct. `Debug`/`PartialEq` derives are dropped, so no opaque `fmt`.
5. **On the Lean side, `gen.sh` does three things:**
   - it strips Aeneas's `module`/`public import` syntax, because the model is not a module and a
     module cannot import it;
   - it imports `FvPredicates/F64.lean`, since Aeneas leaves the type `F64` undefined. Here it
     is `IEEE754.F64`;
   - it checks that the template's list of opaque functions is the one
     `FvPredicates/FunsExternal.lean` defines. Each one is defined as the model's operation, not
     axiomatized.

Better than rewriting the kernel: teach Aeneas that `f64` binops are calls of those opaque
functions, and that a float literal is `from_bits` of its encoding. Both are local changes in
Aeneas's symbolic interpreter and extraction.

**The exact paths are not proven,** and neither are TwoSum and TwoProduct in `verify/ieee754`
(requested, not done in this round). The Lean translation of the exact paths is done. To prove
that they compute the exact sign, three things are missing:
- **(a)** exactness of TwoSum, for all finite inputs without overflow;
- **(b)** exactness of Veltkamp splitting and Dekker's TwoProduct, without underflow or
  overflow;
- **(c)** Shewchuk's theorem that `sum_into` (his FAST-EXPANSION-SUM with zero elimination)
  keeps expansions strongly nonoverlapping under round-to-even, and that the top component of
  a nonoverlapping expansion then has the sign of the sum.

Part (c) is about two pages of Shewchuk and the bulk of the work. It needs ulp and
"multiple of 2^k" reasoning that the model has the pieces for (`val_repr`, `round_exact`,
`uround_err`) but not yet the lemmas. `estimate` itself is easy: it always returns a value with
its top component's sign. With the fix, such theorems would carry the window as their
hypothesis. Outside it is the integer fallback, which would need its own translation or proof.

The model already has the pieces TwoSum and TwoProduct would build on:
- `round_exact`: a representable value rounds to itself;
- `val_repr`: every double is `k·2^e`;
- `sterbenz`;
- `round_mono`;
- `uround_err`: half-ulp error.

Two pieces are missing: the "nearest" characterization of rounding (`|round r − r| ≤ |y − r|`
for every representable `y`) and the case analysis of Knuth's proof.

## Good next targets

- **`cross_sign`'s exact path**, then `orient2d_adapt`/`orient2d_exact`: through (a)–(c) above.
  (a) and (b) are standard and self-contained. They are a good addition to `verify/ieee754`.
- **`angle_cmp`**: its half-plane classification is exact comparisons, and the rest is
  `cross2 = cross_sign`. Once `cross_sign` is fully verified, `angle_cmp` is a total order
  proof on top of it.
- **`inside_polygon`**: parity of crossings decided by `orient2d` signs. Given exact orientation
  it is a combinatorial proof.
- **`certainly_in_circle`**: a pure filter with the bound `ICC_BOUND = (10 + 96ε)ε` and now the
  `TINY`/`SLACK` terms. Its proof is the incircle analogue of `shewchuk`. The function only
  claims "certainly inside", so the filter is all there is to prove.
- **`dot3` and `triple`**: their filters, `DOT3_BOUND` and `TRIPLE_BOUND`, are derived in
  research/predicate-bounds/ but not proven. They are a direct extension of `shewchuk`, with a
  sum of three products. Unlike `cross_sign`, their sign can flip under rounding, so the bound
  matters.
