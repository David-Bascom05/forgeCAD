# ieee754: IEEE 754 binary floating point in Lean 4

A model of the IEEE 754 binary formats (binary64 = `f64` in particular), with proofs and with an
executable that is checked against hardware bit for bit. It does not depend on ForgeCAD. License
MPL-2.0.

## The model

All in namespace `IEEE754`.

- **Formats.** `Format` is an exponent width and a precision: `binary64 = ⟨11, 53⟩`,
  `binary32 = ⟨8, 24⟩`, `binary16 = ⟨5, 11⟩`. Its constants are `emax`, `emin`, `qmin` (the
  least subnormal is `2^qmin`) and `qmax`.
- **Data.** `FP f` is one of:
  - `zero s` or `inf s`, signed;
  - `nan` (one NaN: payloads are not modelled);
  - `fin s m e h`, the value `(-1)^s · m · 2^e`.

  `h : Canon f m e` makes the significand and exponent canonical: `0 < m < 2^prec`,
  `qmin ≤ e ≤ qmax`, and `m ≥ 2^(prec-1)` (normal) or `e = qmin` (subnormal). So equality of
  `FP` values is equality of IEEE data. `FP.val : FP f → ℚ` is the exact value. `F64` is
  `FP binary64`.
- **Rounding.** `round f zs r` rounds to nearest, ties to even, over any linearly ordered field
  with a floor (`ℚ` and `ℝ`). Overflow goes to ±∞. `zs` is the sign an exact zero result gets.
  A nonzero result that underflows to zero keeps the sign of `r`.
- **Operations:** `add sub mul div fma sqrt neg abs`. Each one handles NaN, the infinities and
  the sign of zero as IEEE 754 says. On finite operands it is `round` of the exact rational
  result. `sqrt` is computed with an integer square root, and `sqrt_eq_round` proves it equal to
  `round` of the real square root.
- **Comparisons:** `cmp` (`none` when unordered), `eq`, `lt`, `le`.
- **Conversions:**
  - `ofInt`: from an integer, rounded to nearest;
  - `toInt?`: to an integer, truncating;
  - `toIntSat lo hi`: Rust's saturating `as` cast.
- **Encoding:** `toBits` and `ofBits` (sign, biased exponent, fraction). NaN encodes as the
  quiet NaN.

Everything is computable: `ieee754-eval` (Main.lean) runs the model on queries from stdin.

## Theorems

No `sorry`. Only `propext`, `Classical.choice` and `Quot.sound`: `IEEE754/Axioms.lean` checks
every theorem below and fails the build otherwise.

| | |
|---|---|
| op = round(exact) | `add_eq_round`, `sub_eq_round`, `mul_eq_round`, `div_eq_round`, `fma_eq_round`, `sqrt_eq_round` (over ℝ) |
| relative error | `round_rel_err`: `2^emin ≤ |r|` and the result finite ⇒ `|round r − r| ≤ 2^-prec·|r|`; per op `add_rel_err` … `fma_rel_err`, `sqrt_rel_err`; for binary64, `F64.add_rel_err` …, with `2^-53` and `2^-1022` |
| subnormal range | `round_abs_err`: `|r| < 2^emin` ⇒ finite and `|round r − r| ≤ 2^(qmin−1)` (`2^-1075` for binary64); per op `add_abs_err` …; `round_err` combines both bounds; `add_exact_of_small`: addition is exact there |
| monotonicity | `round_mono`: `r₁ ≤ r₂` ⇒ `(round r₁).le (round r₂)`, overflow included; `sqrt_mono` |
| idempotence | `round_val`: rounding a representable value gives it back; `round_exact`: `k·2^e` with `|k| ≤ 2^prec`, `e ≥ qmin` and no overflow is exact |
| Sterbenz | `sterbenz`: `y/2 ≤ x ≤ 2y` ⇒ `x − y` is exact |
| integers | `ofInt_exact` (`|n| ≤ 2^prec`), `toInt_ofInt`, `add_int_exact`, `mul_int_exact`, `F64.ofInt_exact` (`|n| ≤ 2^53`) |
| identities | for every datum, NaN and infinities included: `mul_one`, `one_mul`, `div_one`, `add_neg_zero` (`x + −0 = x`), `neg_zero_add`, `sub_zero`, `add_comm`, `mul_comm`, `fma_neg_zero` (`fma x y −0 = x·y`), `neg_neg`. Also `add_zero` (`x + +0 = x` unless `x = −0`) and `sub_self` (`x − x = +0` for finite `x`) |
| nearest | `round_nearest`: a finite rounding of `r` is at least as close to `r` as every finite datum |
| error-free sums | `add_err_representable`: `a + b − (a ⊕ b)` is representable; `fast2sum`: `(a ⊕ b) ⊖ a` is exact when `\|b\| ≤ \|a\|`; `fast_two_sum_exact`, `two_sum_exact_of_le`: Fast2Sum, and TwoSum when `\|b\| ≤ \|a\|`, give `s + e = a + b` |
| error-free sums | `add_err_representable`: `a + b − (a ⊕ b)` is representable; `fast2sum`: `(a ⊕ b) ⊖ a` is exact when `\|b\| ≤ \|a\|`; `fast_two_sum_exact`, `two_sum_exact_of_le`: Fast2Sum, and TwoSum when `\|b\| ≤ \|a\|`, give `s + e = a + b`; `two_sum_exact`: TwoSum (`forge-math`'s `two_sum`) gives `s + e = a + b` for all finite `a`, `b` with `a ⊕ b` and `s ⊖ a` finite (Knuth's case `\|a\| < \|b\|` by Sterbenz on `b ⊖ bv` and `s ⊖ bv`; precision at least 3) |
| Veltkamp's split | `veltkamp`: `forge-math`'s `split` with `SPLITTER = 2^σ + 1` gives finite `hi`, `lo` with `hi + lo = a`, `hi` on the grid `2^(ε+σ)` and `\|hi\| ≤ 2^(ε+p)`, `lo` on `2^ε` and `\|lo\| ≤ 2^(ε+σ−1)` (`ε` the binade of `a`), both on every grid of `a`; precision at least `σ + 1` |
| Dekker's product | `two_product_exact`: `forge-math`'s `two_product` gives `x + y = a · b` exactly at precision `2σ − 1` with `SPLITTER = 2^σ + 1`, when `SPLITTER · a`, `SPLITTER · b` are finite, `\|a · b\| < 2^(emax−1)`, and `a`, `b` lie on grids `2^la`, `2^lb` with `la + lb ≥ qmin` (the kernel's `product_exact`); `F64.two_product_exact` is the `f64` case, `σ = 27` |
| comparisons | `lt_iff`, `le_iff`, `eq_iff` (the order of exact values on finite data), `zero_eq_zero` (`−0 == +0`), `nan_lt` … |
| encoding | `ofBits_toBits` (every datum), `toBits_ofBits` (every non-NaN pattern below `2^width`), `toBits_lt` |
| error relative to the result | `round_rel_err_res`: `|round r − r| ≤ 2^-prec · |round r|` outside the subnormal range; `add_err_res`, `sub_err_res` (always, for two finite data), `mul_err_res` (unless the product underflows), as forward error analyses use them; finiteness of operands of a finite result (`isFinite_of_add` …) |

The rounding theorems are generic over the field. The arithmetic operations use them at `ℚ`, and
`sqrt` uses them at `ℝ`.

## Mathlib

The package uses Mathlib (tag `v4.36.0`, on Lean `v4.36.0`). It needs `ℝ` and `Real.sqrt` to
state square root, and `Int.log`, floors and `zpow` lemmas for the proofs. The pin is the one
Aeneas's Lean library uses, so a project can require both. `lake exe cache get` fetches the
prebuilt Mathlib.

## Building

```
lake exe cache get
lake build            # the proofs, the axioms check and ieee754-eval
```

## Hardware check

`check/` is a Rust harness that compares the model with this CPU. It generates operands of these
kinds:
- every pair of special and boundary values: ±0, least and greatest subnormal, least normal,
  greatest finite, ±∞, NaNs, 1, 1 ± ulp, 2, 3;
- uniformly random bit patterns;
- operands with tiny, near-overflow and near-1 exponents;
- short significands, whose products are exact or ties;
- addends about half an ulp of the other operand (ties in addition);
- near-negations (cancellation);
- `fma` addends equal to minus the rounded product (they expose the product's error);
- integers near `2^53` and `2^63`, and `as i64` near the saturation bounds.

For each operand it computes `add sub mul div fma sqrt neg abs`, `== < <=`, `i64 as f64` and
`f64 as i64` with `f64`, and a quarter as many cases with `f32`. `fma` is `mul_add`, compiled to
`vfmadd` when the CPU has FMA. It sends the same queries to `ieee754-eval` and compares the
answers bit for bit. A NaN matches any NaN.

```
cd check && cargo run --release -- --n 1000000     # ≈ 8.8 million cases
```

The output counts cases per operation, how the results split by kind (zero, subnormal, normal,
infinite, NaN), and how many binary64 sums and products are exact ties.

Results on an Intel Xeon (x86-64, FMA) are in `RESULTS.md`.
