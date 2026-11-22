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

## What is proven

The proofs are about the Aeneas translation of the copy, with `f64` as the model's binary64. They
use only `propext`, `Classical.choice` and `Quot.sound`. `FloatVerification/Axioms.lean`
fails the build otherwise.

1. **`cross_sign_fast`.** Take any finite doubles `a b c d`. Suppose the fast test of
   `cross_sign(a, b, c, d)` passes, that is `|l − r| > CROSS_BOUND·(|l| + |r|)` with `l = a*b`
   and `r = c*d` rounded. Then `cross_sign` returns `l − r`, and it is finite, nonzero and has
   the sign of the exact `a·b − c·d`.
   - No range hypothesis is needed. Overflow and underflow are included.
   - No error analysis is needed either. Rounding is monotone (`round_mono`), so the order of
     `l` and `r` is never the wrong way round. A rounded difference has the sign of the exact
     difference or is zero. The test only has to exclude zero, NaN and overflow, and the proof
     shows it does: if `l − r` overflows, so does `|l| + |r|`, and the bound becomes `+∞`.
   - So `CROSS_BOUND` matters for speed, not soundness. Any bound `≥ 0` would be sound.
2. **`orient2d_fast`.** Take finite coordinates `a b c`. Assume no underflow: the products
   `(a.x − c.x)·(b.y − c.y)` and `(a.y − c.y)·(b.x − c.x)` of the rounded differences, and the
   exact `CCW_BOUND · detsum`, are each zero or at least `2^-1022` in magnitude. If the fast test
   `det > bound || −det > bound` passes, `orient2d` returns `det`, finite, nonzero and with the
   sign of the exact determinant of the input doubles.
   - The core is `shewchuk`: Shewchuk's constant `(3 + 16ε)ε` suffices, proven over ℚ from the
     error of each step relative to its computed value. Those errors are the model's
     `sub_err_res`, `mul_err_res` and `add_err_res`, new in `verify/ieee754/IEEE754/Computed.lean`.
   - Overflow needs no hypothesis: the test fails whenever anything overflows.
3. **The no-underflow hypothesis cannot be dropped.** `orient2d_wrong_sign` exhibits inputs where
   the test passes with the wrong sign. It is checked by evaluation (`native_decide`), the one
   result that relies on the compiler, and nothing depends on it.

## Which kernel functions are verified

- The fast path of `forge_math::predicates::cross_sign`, for all finite inputs. `cross2` is the
  same function.
- The fast path of `forge_math::predicates::orient2d`, when the products don't underflow.

The proofs are about the copy in `predicates/`. That the copy equals the kernel is tested, not
proven: `matches_the_kernel` compares the two on 2,000,000 random and near-degenerate inputs,
bit for bit, and they agree.

The exact paths are translated (`cross_exact`, `sum_into`, `estimate`, `orient2d_adapt`,
`orient2d_exact`, `cross_two`, `scale_into`) and bound to the model, but **not proven**. They
were tested instead with `predicates/examples/dump.rs` and `check_exact.py`. Each case has its
sign checked against exact rationals, and about half go past the fast test into the adaptive or
exact path:

| coordinates / operands | cases | past the fast test | wrong signs |
|---|---|---|---|
| scale 2^-40 … 2^40 | 300,000 | 156,254 / 146,843 | 0 |
| scale 2^-480 … 2^480 | 200,000 | 103,881 / 97,891 | 0 |
| scale 2^-540 … 2^-520 (products underflow) | 200,000 | 199,940 / 200,000 | 369,335 |
| scale 2^505 … 2^515 (products overflow) | 200,000 | 105,645 / 113,033 | 67,546 |

## Findings about the kernel

- **`orient2d` answers a wrong sign when the products underflow.** The inputs are
  `a = (5.394455326111622e-162, 1.6245062719943995e-164)`,
  `b = (1.3762009331530288e-157, 4.1443425003634566e-160)` and
  `c = (-4.608837897596252e-178, 0)`.
  - The exact determinant is positive, but `orient2d` returns `-5e-324` from its fast path.
    The same points scaled by 2^600 come out positive.
  - Cause: the two rounded products are subnormal and straddle a rounding midpoint in the wrong
    order. `bound` underflows to 0, so the test passes. Shewchuk's error bound assumes no
    underflow.
  - Checked on the kernel itself (`orient2d_underflow_counterexample` in `predicates/`) and
    proven of the translation (`orient2d_wrong_sign`).
- **The exact paths answer 0 for nonzero determinants under underflow.** For example,
  `cross_sign(1.4u, u, 1.3u, u)` with `u = 2^-537` returns `0`, though `a·b − c·d > 0`.
  `orient2d((3t, 5t), (5t, 8t), (0, 0))` with `t = 2^-540` also returns `0`, though the
  determinant is `−t² ≠ 0`. `two_product`'s error term is not representable when the product
  underflows.
- **They answer NaN when a product overflows.** `cross_sign(1e300, 1e10, 1e300, 1e9)` gives NaN,
  and so does `orient2d((1e300, 0), (0, 1e300), (−1e300, −1e300))`. Both `> 0.0` and `< 0.0` are
  false for NaN, so callers read it as "collinear". Also, `split` overflows for `|a| > 2^996`.
- So the module's contract, "the returned value always has the correct sign; … zero exactly when
  it is zero", holds only in a range. The tests found no wrong sign with operands within 2^±480.
  The fast paths are proven within the stated range: `cross_sign` everywhere, `orient2d` without
  underflow. CAD coordinates in millimetres are far inside that range, so this is a documented
  limit rather than an urgent bug.
- Possible fixes:
  - document the range;
  - or check it in the slow path (`if !(det.is_finite()) || detsum < 2^-900 …`) and rescale
    the inputs by a power of two before the exact computation. Rescaling by `2^k` is exact
    and multiplies the determinant by `2^2k`.

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

**The exact paths are not proven.** The Lean translation of the exact paths is done. To prove
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
its top component's sign. The table above shows these theorems would need a range hypothesis
too.

## Good next targets

- **`cross_sign`'s exact path**, then `orient2d_adapt`/`orient2d_exact`: through (a)–(c) above.
  (a) and (b) are standard and self-contained. They are a good addition to `verify/ieee754`.
- **`angle_cmp`**: its half-plane classification is exact comparisons, and the rest is
  `cross2 = cross_sign`. Once `cross_sign` is fully verified, `angle_cmp` is a total order
  proof on top of it.
- **`inside_polygon`**: parity of crossings decided by `orient2d` signs. Given exact orientation
  it is a combinatorial proof.
- **`certainly_in_circle`**: a pure filter with the bound `ICC_BOUND = (10 + 96ε)ε`. Its proof is
  the incircle analogue of `shewchuk`, under the same no-underflow hypothesis. The function only
  claims "certainly inside", so the filter is all there is to prove.
- **`dot3` and `triple`**: their filters, `DOT3_BOUND` and `TRIPLE_BOUND`, are derived in
  research/predicate-bounds/ but not proven. They are a direct extension of `shewchuk`, with a
  sum of three products. Unlike `cross_sign`, their sign can flip under rounding, so the bound
  matters.
