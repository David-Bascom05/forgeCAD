# kernel/forge-math through Aeneas: the L4 pilot

`kernel/forge-math/src/predicates.rs` is translated as it is, built as release, with no source
change (`translate.conf`, `tools/aeneas/translate.sh forge-math`). The proofs in `lean/ForgeMath/`
are about that translation.

## Proven (`lean/ForgeMath/Refine.lean`, `Predicates.lean`)

Each statement says that the shipped function returns `ok`, so it does not panic, overflow or
diverge, and what its value is. Axioms: `propext`, `Classical.choice` and `Quot.sound` only
(`Axioms.lean` fails the build otherwise).

| Rust | Lean | statement |
|---|---|---|
| `two_sum` | `two_sum_eq`, `two_sum_exact` | equals the model's `twoSum`; for finite `a`, `b` with `a ⊕ b` and `s ⊖ a` finite, `s + e = a + b` exactly (Knuth) |
| `split` | `split_eq`, `split_veltkamp` | equals the model's `split SPLITTER`; Veltkamp's properties: `hi + lo = a`, the grids and the sizes |
| `two_product` | `two_product_eq`, `two_product_exact` | equals the model's `twoProduct SPLITTER`; under `product_exact` (the kernel's debug precondition), `x + y = a · b` exactly (Dekker) |
| `CROSS_BOUND`, `CCW_BOUND` | `CROSS_BOUND_eq`, `CCW_BOUND_eq` | the real code computes `(2 + 8·EPS)·EPS` and `(3 + 16·EPS)·EPS`; evaluated, they are the literals the ieee session's bounds use |
| `estimate` | `slice_sum`, `estimate_eq` | the shipped `e.iter().sum()` is the left fold of the model's `add` from `-0.0` (std's `Sum for f64`, defined in the bridge's `Core.lean`); `estimate` returns that sum when its sign agrees with the last component's, else the last component |
| `cross_sign` | `cross_sign_of_fast`, `cross_sign_fast` | if the fast test passes, the result is finite, nonzero and has the exact sign of `a·b − c·d`, for every finite input |
| `orient2d` | `orient2d_of_fast`, `orient2d_fast` | if the fast test passes and the three filter products are `Relative` (`check_filter`), the result is finite, nonzero and has the exact orientation's sign |

`cross_sign_fast` and `orient2d_fast` are the ieee session's theorems
(research/kernel-audit/float-verification), carried over. They were proved for a copy of the
code respelled for Aeneas. `*_of_fast` shows that the real function returns the same value as
the copy whenever the fast test passes, so their conclusions hold for the shipped code. The copy
is no longer needed for these two.

## Opaque

`research/aeneas/proposals/predicates.md` applied six Aeneas-friendly rewrites to `predicates.rs`.
They change control flow only: bits are identical, and the kernel's callgrind lines are at or
below base. With them every slow path translates: `sum_into`, `scale_into`, `exact_diffs`,
`cross_two`, `orient2d_adapt`/`_exact`, `incircle`, and `incircle_exact_diffs`/`_general`. That is 41
definitions. What stays opaque (`lean/ForgeMath/FunsExternal.lean`, Lean `opaque` constants, no
axioms) is the cold integer fallback, run only when a slow path's expansion overflows to NaN:
`cross_big`, `orient2d_big`, `dot3_big`, `incircle_big` (iterator chains, `rposition`, `u128`).

Next: Shewchuk's expansion invariants for `sum_into` and `scale_into` (nonoverlapping, increasing
magnitude, exact sum), and from them the slow paths and `incircle`.

## Rebuilding

```
tools/aeneas/setup.sh                     # once: the pinned toolchain (~40 min)
tools/aeneas/translate.sh forge-math      # Types.lean, Funs.lean, the templates
cd verify/aeneas/forge-math/lean && lake exe cache get && lake build
```
