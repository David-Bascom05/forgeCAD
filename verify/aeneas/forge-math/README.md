# kernel/forge-math through Aeneas: the L4 pilot

`kernel/forge-math/src/predicates.rs` is translated as it is, built as release, with no source
change (`translate.conf`, `tools/aeneas/translate.sh forge-math`). The proofs in `lean/ForgeMath/`
are about that translation.

## Proven (`lean/ForgeMath/Refine.lean`, `Predicates.lean`, `Estimate.lean`, `Expansion.lean`)

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
| `next_component`, `sum_into` | `next_spec`, `sum_into_spec` (`Expansion.lean`) | with `e.len() + f.len() ≤ h.len()`, no panic, and `h[..n]` is `sumIntoL e f`: the merge by magnitude (`mergeL`), the twoSum chain keeping nonzero errors (`growL`), and the last sum unless it is zero with something before it |
| `scale_into` | `scale_into_spec` | with `2·e.len() ≤ h.len()`, no panic, and `h[..n]` is `scaleL e b` (Scale-Expansion with zero elimination, over `twoProduct`/`twoSum`) |
| `cross_sign` | `cross_sign_of_fast`, `cross_sign_fast` | if the fast test passes, the result is finite, nonzero and has the exact sign of `a·b − c·d`, for every finite input |
| `orient2d` | `orient2d_of_fast`, `orient2d_fast` | if the fast test passes and the three filter products are `Relative` (`check_filter`), the result is finite, nonzero and has the exact orientation's sign |

`cross_sign_fast` and `orient2d_fast` are the ieee session's theorems
(research/kernel-audit/float-verification), carried over. They were proved for a copy of the
code respelled for Aeneas. `*_of_fast` shows that the real function returns the same value as
the copy whenever the fast test passes, so their conclusions hold for the shipped code. The copy
is no longer needed for these two.

## Proven: `dmath` (`lean/ForgeMath/Dmath.lean`, `Atan.lean`)

Each translated function returns `ok` and equals the ieee session's model, written operation
for operation from the same Rust (verify/ieee754/IEEE754/Dmath). The models' contracts then hold
for the shipped code.

| Rust | equals the model (`*_eq`, `*_spec`) | contract, for the shipped code |
|---|---|---|
| `k_sin`, `k_cos` | `ksin`, `kcos` | (the models' `ksin_eval`, `kcos_eval`) |
| `exponent`, `rem_pio2` | the exponent field `expo`, `remPio2` | |
| `sin_cos`, `sin`, `cos` | `sinCos`, `sin`, `cos` | C1 `sin_cos_err`, `sin_err`, `cos_err`: for finite `\|x\| ≤ 2^20`, within `2^-52` |
| `atan` | `atan` (for every input: NaN, `±∞`, all branches) | `atan_err`: finite `x`, within `3·10^-16` of `arctan x` |
| `atan2` | `atan2` (every special case, then the exponent gap) | C2 `atan2_err50`: within `2^-50` of the angle |
| `asin`, `acos` | `asin`, `acos` (`RangeInclusive::contains` through f64's `PartialOrd`) | C3 `asin_err`, `acos_err`: on `[−1, 1]`, within `2^-50` |

## Opaque

`research/aeneas/proposals/predicates.md` applied six Aeneas-friendly rewrites to `predicates.rs`.
They change control flow only: bits are identical, and the kernel's callgrind lines are at or
below base. With them every slow path translates: `sum_into`, `scale_into`, `exact_diffs`,
`cross_two`, `orient2d_adapt`/`_exact`, `incircle`, and `incircle_exact_diffs`/`_general`. That is 41
definitions. What stays opaque (`lean/ForgeMath/FunsExternal.lean`, Lean `opaque` constants, no
axioms) is the cold integer fallback, run only when a slow path's expansion overflows to NaN:
`cross_big`, `orient2d_big`, `dot3_big`, `incircle_big` (iterator chains, `rposition`, `u128`).

Next: Shewchuk's invariants (nonoverlapping, exact sum) are stated against `sumIntoL` and `scaleL`
in verify/ieee754's `Expansion.lean` (sheet-spec); with them, the slow paths and `incircle`.

## Rebuilding

```
tools/aeneas/setup.sh                     # once: the pinned toolchain (~40 min)
tools/aeneas/translate.sh forge-math      # Types.lean, Funs.lean, the templates
cd verify/aeneas/forge-math/lean && lake exe cache get && lake build
```
