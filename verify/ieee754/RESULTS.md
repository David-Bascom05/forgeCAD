# Results

## Part 1: the IEEE 754 model

### What is proven

Every theorem listed in the README is proven, with no `sorry`. `IEEE754/Axioms.lean` checks
that each rests only on `propext`, `Classical.choice` and `Quot.sound`, and fails the build
otherwise. A test file containing a theorem proven by `sorry` and listed in `#standard_axioms`
fails with "rests on the axiom sorryAx". In summary:

- **op = round(exact):** `+ − × ÷ fma` on finite operands are `round` of the exact rational
  result. `sqrt` is `round` of the real square root. The square root is computed with an
  integer square root, and `sqrtPos_eq` proves that algorithm equal to rounding `Real.sqrt`.
- **Relative error:** `|fl(x∘y) − x∘y| ≤ 2^-prec · |x∘y|` when `|x∘y| ≥ 2^emin` and the result
  is finite, for every operation including `sqrt`. For binary64 the constants are `2^-53` and
  `2^-1022` (`F64.*_rel_err`).
- **Subnormal range:** `|fl(r) − r| ≤ 2^(qmin−1)` (`2^-1075` for binary64), and no overflow
  there. Addition is exact there (`add_exact_of_small`).
- **Monotonicity:** `round_mono`, in the IEEE order, with overflow to ±∞ included.
- **Sterbenz:** `y/2 ≤ x ≤ 2y` ⇒ `x ⊖ y = x − y`.
- **Integers:**
  - `|n| ≤ 2^prec` converts exactly and back;
  - sums and products of such integers are exact while they stay within `2^prec`.
- **Idempotence:** rounding a representable value gives it back. `round_exact` covers any
  `k·2^e`.
- **Identities:** `x·1 = x`, `1·x = x`, `x/1 = x`, `x + (−0) = x`, `x − (+0) = x`,
  `fma(x, y, −0) = x·y`, and commutativity of `+` and `·`, all for every datum including NaN
  and ±∞. Also `x + (+0) = x` unless `x = −0`, and `x − x = +0` for finite `x`.
- **Comparisons:**
  - on finite data they are the order of the exact values, so `−0 == +0`;
  - NaN compares false.
- **Encoding:** `ofBits ∘ toBits = id`, and `toBits ∘ ofBits = id` on non-NaN patterns.

The rounding theorems are proven once, for any linearly ordered field with a floor, and used at
`ℚ` and at `ℝ`. They are also proven for any binary format (`Format`), not only binary64.

**Mathlib.** The package uses it, at `v4.36.0`, the same pin as Aeneas's Lean library. `ℝ` and
`Real.sqrt` are needed to state that square root is correctly rounded. The floor, `Int.log` and
`zpow` lemmas carry the proofs. The executable model computes over `ℚ` only.

### Hardware validation

`check/` (Rust) against `ieee754-eval` (the Lean model compiled), on an Intel Xeon @ 2.10GHz
(x86-64 with FMA; `mul_add` compiled to `vfmadd`):

```
cargo run --release -- --n 1000000
ieee754-check: 8783237 queries (fma on this CPU: vfmadd)
    op      cases   mismatch
   abs         84          0
   add    1255528          0
   div    1255528          0
    eq     318036          0
   f2i     160214          0
   fma    1250000          0
   i2f     156150          0
    le     318036          0
    lt     318036          0
   mul    1255528          0
   neg         84          0
  sqrt    1250084          0
   sub    1255528          0
float results: 544939 zero, 151537 subnormal, 5562569 normal, 677227 infinite, 734542 NaN
total 8783237 cases, 0 mismatches, model time 122.6s (fma: hardware vfmadd)
```

That is 8.78 million cases and **no mismatch**, with these inputs:
- binary64, and binary32 at a quarter of the volume;
- every pair of the 42 edge values per format;
- 151,537 subnormal results and 677,227 overflows;
- in a 20,000-operand sample, 4.5% of the binary64 sums and products were exact ties. The
  harness counts them with TwoSum and FMA.

An earlier run with a different operand mix, 5,283,584 cases, also had none. CI
(`.github/workflows/ieee754.yml`) builds the package and the proofs, then runs about 270,000
cases.

## Part 2: the kernel's predicates

See `research/kernel-audit/float-verification.md`. In brief:
- the Aeneas translation of `cross_sign` and `orient2d`, from `kernel/forge-math/src/predicates.rs`,
  runs on this model;
- the fast path of `cross_sign` is proven right for every finite input;
- the fast path of `orient2d` is proven right when nothing underflows, and is wrong otherwise
  (counterexample included);
- the exact paths are translated but not yet proven.
