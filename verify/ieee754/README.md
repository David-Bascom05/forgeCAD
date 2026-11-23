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
| error-free sums | `add_err_representable`: `a + b − (a ⊕ b)` is representable; `fast2sum`: `(a ⊕ b) ⊖ a` is exact when `\|b\| ≤ \|a\|`; `fast_two_sum_exact`, `two_sum_exact_of_le`: Fast2Sum, and TwoSum when `\|b\| ≤ \|a\|`, give `s + e = a + b`; `two_sum_exact`: TwoSum (`forge-math`'s `two_sum`) gives `s + e = a + b` for all finite `a`, `b` with `a ⊕ b` and `s ⊖ a` finite (Knuth's case `\|a\| < \|b\|` by Sterbenz on `b ⊖ bv` and `s ⊖ bv`; precision at least 3) |
| Veltkamp's split | `veltkamp`: `forge-math`'s `split` with `SPLITTER = 2^σ + 1` gives finite `hi`, `lo` with `hi + lo = a`, `hi` on the grid `2^(ε+σ)` and `\|hi\| ≤ 2^(ε+p)`, `lo` on `2^ε` and `\|lo\| ≤ 2^(ε+σ−1)` (`ε` the binade of `a`), both on every grid of `a`; precision at least `σ + 1` |
| Dekker's product | `two_product_exact`: `forge-math`'s `two_product` gives `x + y = a · b` exactly at precision `2σ − 1` with `SPLITTER = 2^σ + 1`, when `SPLITTER · a`, `SPLITTER · b` are finite, `\|a · b\| < 2^(emax−1)`, and `a`, `b` lie on grids `2^la`, `2^lb` with `la + lb ≥ qmin` (the kernel's `product_exact`); `F64.two_product_exact` is the `f64` case, `σ = 27` |
| `dmath::rem_pio2` | `Dmath.remPio2_spec`: for finite `\|x\| ≤ 2^20`, `y0`, `y1` finite, `\|n\| ≤ 667546`, `\|y0 + y1 − (x − n·π/2)\| ≤ 2^-56`, `\|y1\| ≤ 2^-53·\|y0\| + 2^-62`, `\|y0 + y1\| ≤ 0.78541` (every branch of the exponent tests; `π` to 30 digits, `Dmath.pi_gt_d30`, `Dmath.pi_lt_d30`, from witnesses for Mathlib.s `pi_lower_bound`); `Dmath/Consts.lean` holds `dmath.rs`'s constants, each with its Rust bit pattern |
| `dmath::k_sin`, `k_cos` polynomials | `Dmath.ksin_poly`, `Dmath.kcos_poly`: for rational `\|x\| ≤ 0.7855`, `\|x·P(x²) − sin x\| ≤ 2^-57 + 10^-27` and `\|Q(x²) − cos x\| ≤ 2^-57 + 10^-29`, with `P`, `Q` the Rust coefficients (kernel-checked interval certificates against Mathlib's Taylor bounds) |
| `dmath::k_sin`, `k_cos` in floating point | `Dmath.ksin_eval`, `Dmath.kcos_eval`: for finite `\|x\| ≤ 0.78542`, `\|y\| ≤ 10^-16`, the result is finite and within 1.36e-16 of `x·P(x²) + y·(1 − x²/2)` (sin), 1.63e-16 of `Q(x²) − x·y` (cos); `Dmath/Approx.lean` is the error calculus |
| `dmath::sin_cos`, `sin`, `cos` (contract C1) | `Dmath.sinCos_err`, `Dmath.sin_err`, `Dmath.cos_err`: for finite `\|x\| ≤ 2^20`, both results are finite and within `2^-52` of `sin x`, `cos x` (absolute; every quadrant and the `\|x\| < 2^-27` branch). `Dmath.ksin_err`, `Dmath.kcos_err`: `k_sin`, `k_cos` within 1.45e-16, 1.79e-16 of `sin(y0+y1)`, `cos(y0+y1)` |
| `dmath::atan` polynomial | `Dmath.katan_poly`: for rational `\|t\| ≤ 0.4375`, `\|t − t·(s1+s2) − arctan t\| ≤ 2^-57 + 10^-19` with the Rust `AT` coefficients; `Dmath.atan_taylor`: `\|arctan t − T_N(t)\| ≤ \|t\|^(2N+1)` (mean value theorem) |
| `dmath::atan` | `Dmath.atan_err`: for finite `x`, `atan(x)` is finite and within `3·10^-16` of `arctan x` (every branch: `\|x\| ≥ 2^66`, `< 2^-27`, the polynomial on `[0, 7/16)` and the four reductions `red_t0`–`red_t3` with `reduced_err`); `Dmath.round_ulp`: a finite rounding of `\|q\| < 2^k` is within `2^(k−54)` |
| `dmath::atan2` (contract C2) | `Dmath.atan2_err50`: for finite `y`, `x`, not both zero and not `y = −0` with `x < 0` (where Rust returns `−π`), `atan2(y, x)` is finite and within `2^-50` of `atan2R y x`, the angle of `(x, y)` in `(−π, π]` (every branch: zero operands, the exponent-gap shortcuts, `atan(\|y/x\|)` and the four quadrants); `Dmath.atan2_err50` is the contract, `Dmath.atan2_err` the sharper `3.56e-16` for `x > 0`, `6.9e-16` otherwise |
| `dmath::asin`, `acos` (contract C3) | `Dmath.asin_err`, `Dmath.acos_err`: for finite `x ∈ [−1, 1]`, `asin(x)`, `acos(x)` are finite and within `2^-50` of `arcsin x`, `arccos x`. `Dmath.sqrtArg_err`: `√((1 − x)(1 + x))` in floating point is `+`-signed and within `2.78e-16` (relative) of `√(1 − x²)` (Sterbenz near `±1`); `pert_y`, `pert_x` move the angle by half the relative perturbation |
| decide-once `side_err`, `side_at` (screens 2, 3) | `Screen.side_err_sound`, `Screen.side_at_sound`: with the `PlaneF` coefficients within `2ε` of the exact plane's and the point within `err` (resp. `HPoint::to_f64`'s 2 ulps) of the exact point, `\|e\| > bound` (resp. `> 1e-11·mag`) gives `e` the sign of the exact `N·P − C`, nonzero; no underflow (`mag`, `Σ\|n\| ≥ 2^-900`). `Screen.lin3_err`: `((a0b0 + a1b1) + a2b2) − c` within `14ε·M + 9η` |
| decide-once interval screens (6, 8, 10) | `Screen.gap_sound`: `lo`, `hi` one rounding off `X`, `Y` and `\|lo − hi\| > 1e-12·(1 + \|lo\| + \|hi\|)` in f64 give `lo − hi` the sign of `X − Y` (no underflow hypothesis). `Screen.snap_gap_sound` (snap's y/z gap: the exact grown boxes are disjoint), `Screen.region_sound` (`region_f`: `rlo > rhi` decides the exact comparison; rounding is monotone so `max`/`min` commute with it), `Screen.wall_sound` (the wall clash: the exact offset is not `n_a·x`) |
| decide-once `meet_f` (screen 1) | `Screen.meet_sound`: for f64 plane shadows within `2ε` of the exact planes, in the `Some` branch (`\|det\| > 4·edet`) and with every product and quotient zero or normal, the exact `Det ≠ 0` and each `p_k` is within its error term `e_k` of `Num_k/Det`, which lies on all three exact planes (`Screen.cramer`). `Screen/Track.lean`: tracked errors `\|v − X\| ≤ kεM`; float addition never underflows (`Screen.add_rel0`) |
| decide-once `plainly_apart` (screen 7, two-sided) | `Screen.two_sided`: `near(a,b) ∧ near(b,a)` at a point gives `min(\|δa − δb\|, \|δa + δb\|) ≤ 2d` (`δ` the signed distances). `Screen.dist_sound`: the f64 `(n·c − ca)/√(n·n)` is within `200ε·(1 + Σ\|C_i\|)` of the exact distance (plane within `2(Σ\|n_i\|)(1 + Σ\|C_i\|)` of the origin). `Screen.apart_sound`: when the screen fires at a corner, `tol_eq`'s two-sided `near` fails there (`d ≤ 2^20`, `2d ≤ 2.0*k*l`) |
| decide-once `Pts::first_meet_f` (screen 5) | `Screen.searchF_sound`: the scan skips repeated positions, and passes a refused triple only when its exact `det = 0`. So its answer comes from the first exactly independent triple in `vertex_at`'s order, the triple whose `meet3` `ensure` stores. `Screen.meetF_sound`: `meet_f`'s `Some` (with `det.abs() <= 4.0*edet` read as f64, and `4.0*edet` exact) gives `det ≠ 0`, each `\|p_k − X_k\| ≤ err`, and `err` finite, under item 1's side conditions. `Screen.tolF_hi`: `1e-13*(1.0 + max\|p_k\|)` ≤ `(1 + 3ε)·c13·(1 + max\|p_k\|)`. `Screen.first_meet_f_sound`: combined, with vertices below `2^1022` |
| decide-once `near_f` (screen 9, abs-dot `m`, ffb810a) | At a corner `c` one rounding off the exact `C`, with normals exact and offsets `2ε` off. `Screen.e_close`: the f64 `e = tc·ns2 − sc·nts` is within `1e-12·m + η` of the exact `E = T(C)\|nS\|² − S(C)(nT·nS)` (the tracked error is `38ε·M`; the abs-dot `Σ\|s_i\|\|t_i\|` in `m` covers `sc`'s error, where `\|nts\|` did not). `Screen.r_close`: the f64 `r = d·√nt2·ns2` is within `40ε·R` of `R = d·\|nT\|·\|nS\|²`. `Screen.near_f_past`: `\|e\| − err > r·(1.0 + 1e-12)` gives `¬ near` at `C`. `Screen.near_f_within`: `\|e\| + err < r·(1.0 − 1e-12)` gives `near` at `C`. Products are zero or normal; `\|nS\|², \|nT\|², d ≥ 2^-100` |
| decide-once `crosses_f` (screen 4) | The f64 `q, a, b` in axes `(s, t)` are each within `δ ≤ 2·10^-13·scale` of the exact points, and are at most `scale ≥ 1`. That needs `scale` to take every coordinate, the drop axis included: `pf`'s bound is relative to all three. `Screen.above_sound`: `\|x − q_t\| > 1e-12·scale` in f64 gives the exact `X − Q_t ≠ 0` with the f64 sign. `Screen.d0_close`: when the two `above`s differ, `\|d0 − D0\| ≤ 1e-12·mag` (`\|b_t − a_t\| > 1.95·10^-12·scale` bounds the `δ²` term). `Screen.crosses_f_sound`: an answer of `crosses_f` is the exact `crosses`, and the `(ε, ε²)` tie rule is never reached |
| decide-once `crosses_f` scale, fix (a) (5eb8e9d) | `Screen.scale_facts`: for `scale = 1.0 + m3(q).max(m3(a)).max(m3(b))`, `1 ≤ scale`, every coordinate's magnitude `≤ scale`, and `(1 − ε)(1 + M) ≤ scale`. `Screen.pf_delta_ok`: so item 5's `pf` bound, `(1 + 3ε)·1e-13·(1 + max\|p_k\|)`, is under `2·10^-13·scale`: `crosses_f_sound`'s hypotheses hold |
| decide-once `interior_f` (screen 12, 8f1b276) | `Screen.proj_close` (claim A): the f64 projection coordinate `p_a = (n.c − n_s m_s − n_t m_t)/n_a` is within the f64 `e = γ(\|n.c\| + \|n_s m_s\| + \|n_t m_t\|)/\|n_a\| + γ\|p_a\|` of the exact `(C − n_s m_s − n_t m_t)/n_a`. Normals are f64 exactly, `n.c` is `2ε` off `C`, and products and quotients are `NZ`. The error is `8ε·A/\|n_a\| + 2ε\|p_a\|`, against `γ = 128ε`. `Screen.e_nonneg`: `e ≥ 0`, so the two exact coordinates are within `e` as well. Claim B is `Screen.side_err_sound` (item 2) with `err = e` |
| decide-once `face_fast` screens (A8, f885b18) | `Screen/Face.lean`. Each point has f64 `f` and a bound `e`, with `\|f_k − P_k\| ≤ e` per axis (`pt_diff_close`, `esum_le` turn that into each screen's hypotheses). `Screen.axis_sound` (`cmp_axis`, `same`): `\|f_i − f_j\| > (e_i + e_j)(1 + 1e-12) + slack` gives `P_i ≠ P_j` with the f64 sign. `Screen.midpoint_ok`: `mid_in`'s midpoint keeps `crosses_f_sound`'s hypothesis. `Screen.crosses_line_sound`: `\|l.f·n\| > 1e-13·(l.ab·\|n\|)` gives `d·n ≠ 0`. `Screen.along_sound`: `along`'s `\|v\| > b` gives `(P_i − P_j)·d ≠ 0` with `v`'s sign. `Screen.orient_sound`, `orient_e`: `orient`'s `\|v\| > bound` gives the exact orientation `≠ 0` with `v`'s sign. Not covered: the angle screen (unsound as written, fix (a) proposed), `touches`, the cycle-area screen, and `mid_in`'s prefilter |
| forge-math `Interval::sin`/`cos`/`sin_cos` (trace, 80a5f54c) | `Interval.sin_encloses`, `Interval.cos_encloses`: for finite ends with `\|a\|, \|b\| ≤ 2^20`, every `sin x` (`cos x`), `x ∈ [a, b]`, lies in `trig_at`'s `[lo.max(−1), hi.min(1)]`. `next_down(y)` is modelled as any finite float below `y`, and `next_up` likewise; the loop sets `hi = 1` (`lo = −1`) exactly when an even (odd) integer lies in `[⌈fl(qa − half)⌉, ⌊fl(qb − half)⌋]`. Pieces: `below_round` and `above_round` (rounding to nearest), `crit_counted` (no extreme is missed: `qa − half` need not be exact, monotone rounding suffices), `Dmath.sin_err` and `cos_err` at the ends, and the trace session's ℝ lemmas (`Interval/Trig.lean`, from Krawczyk/Trig.lean at 67796e71). `sin_cos`'s values are `sinCos`'s, the same as `sin`'s and `cos`'s |
| forge-math `exact.rs` `Big` (decide-once's S/M/L, b6a9dd4 onward) | `Exact/Limbs.lean`: each limb loop modelled index by index, with u64 overflow explicit. `add_into_spec`, `sub_into_spec` (`b ≤ a`, final borrow zero), `mul_into_spec` (every u128 intermediate `< 2^128`), `shl_into_spec` (`\|` is `+` on disjoint bits), `trimLen_spec`, and `cmp_mag_spec` (on trimmed slices, the order of the magnitudes). `Exact/Big.lean`: `Canon` is the one-representation invariant (S under `2^126`; M with 1–8 trimmed limbs, zero padding, not S-sized; L with more than 8 limbs). `fromMag_spec` and `view_spec`; `neg`, `add`, `sub`, `mul` and `shl` are the integer operations and stay canonical; `signum_spec` (`signum = Int.sign val`), `isZero_spec` and `cmpBig_spec` (`= compare`). The i128 fast paths are modelled with their guards; `shl`'s u32 `used + bits` is assumed not to wrap. `limbs_unique` and `eq_iff_val`: under `Canon`, structural equality (so the derived `Eq`/`Hash`) is value equality |
| Shewchuk expansions (sheet-spec, 78b21c02 and 2757ee19) | `Expansion.lean`, generic over `FP f`. `NO k K l` is nonoverlapping, bottom up; zero elimination is free (`NO_sublist`). `NO_sum`, `NO_top_dominates`, `NO_sign`: an expansion's sign is its topmost nonzero component's. `round_mult_grid`, `two_sum_facts`, `grow_step`, `grow_chain_NO` and `grow_nonoverlapping`: Grow-Expansion keeps nonoverlapping (Shewchuk Thm 10), with `grow_chain_sum`; `ChainOk` holds each step's finiteness and exactness. `ExpansionSum.lean`: `round_tie_even`, `err_large`, and Shewchuk's Corollaries 8 and 9 for TwoSum (`two_sum_cor8a`, `two_sum_cor8b`, `two_sum_cor9`). Shewchuk's Thm 13 and Cor 22 are false as stated for strongly nonoverlapping (research/expansion-theory/README.md) and are not used |
| dol-curved-rust `Big::divrem` / `divrem_mag` (exact.rs, 5aca447) | `Exact/DivRem.lean`, `Exact/DivRem2.lean`. `shortDiv_spec` (one-limb divisor). Knuth's algorithm D: `Step.corr_spec` (Theorem B and the refined test: the corrected `qhat` is `q` or `q + 1`, at most `B`; u128 exact), `dStep_spec` (one digit: multiply-subtract, and add back on a borrow), `knuthRun_spec` (the outer loop keeps `A = rem + V·q`, the windows under `B·V`), `norm_v` (the shift by `leading_zeros`: the top limb dropped is 0, `v[n−1] ≥ 2^63`), `rshift_spec` (the remainder shifted back exactly), `knuthDiv_spec` and `divremMag_spec`: `a = q·b + r`, `r < b`. `Big.divrem_spec`: canonical results, truncated (`x = q·y + r`, `\|r\| < \|y\|`, `r` with `x`'s sign), `(0, x)` for `y = 0`. The digit step is an `irreducible_def` so the kernel never unfolds it |
| dol-curved-rust `Q` (exact/alg.rs, 5aca447), on ℤ fields | `Exact/QHenrici.lean`. `Normal`: `d > 0`, `gcd(n, d) = 1`. `QZ.ratio_spec` (the normaliser, `x/0 = 0`), `QZ.add_spec` (Henrici's sum: `g = gcd(d₁, d₂)`, `t = n₁(d₂/g) + n₂(d₁/g)`, `h = gcd(t, g)`, `(t/h)/((d₁/g)(d₂/h))` is normal and the exact sum), `QZ.mul_spec` (Henrici's product), `QZ.neg_spec`, and `QZ.normal_unique` (so `Q`'s derived `Eq`/`Hash` is rational equality, with `Big.eq_iff_val`). `Big`'s `divrem` (truncated) and `gcd` are taken at their integer specs, which are still to be proven on the limbs |
| dol-curved-rust `gcd`, `gcd_u128` (div.rs, 1afff77) | `Exact/Gcd.lean`. `gcdU128_spec`: the binary gcd is `gcd(a, b)` (`stein_spec` for the loop, `tz_or`: `tz(a\|b) = min`), and `gcdU128_le`: at most `max(a, b)`, so `<< shift` drops nothing. Lehmer: `Inv` (the corner images `(u+a, v+c)`, `(u+b, v+d)` in `[0, 2^64]²`, alternating signs) is kept by every pass (`step_inv`); `step_bounds`: no `i128` value past `2^66`; `run_brk`: the run stops within `2^65` passes. `cone` + `mediant`: when the corner quotients agree, `q` is the Euclid quotient of the true pair `M·(x, y)`, so every pass is an exact Euclid step (`run_euclid`). `lehmerStep_spec`: an outer pass keeps the gcd and makes the smaller strictly smaller (termination); `gcdL_spec`, `gcdBig_spec`: `gcd(a, o) = gcd(\|a\|, \|o\|)`. Values only: `top_at`, `bits`, and the `Big` arithmetic are taken at their exact values |
| dol-curved-rust `Q` on `Big` fields (forge-alg lib.rs, 1afff77) | `Exact/QBig.lean`. `QB` is `Q` with its `Big` fields, each operation written with the `Big` calls the Rust makes (`bcmp = signum(a − b)`, `divrem(·).0`, `abs`, `gcd` any `G` of value `gcd(\|a\|, \|b\|)`). `divq_spec`: `divrem`'s quotient is `Int.tdiv` (also at `0`). `ratio_tie`, `add_tie`, `neg_tie`, `sub_tie`, `mul_tie`, `div_tie`, `inv_tie`: on canonical fields the result is canonical and reads as `QZ`'s operation; `cmp_tie`: `cmp_q` is `sign(n₁d₂ − n₂d₁)`. With `QHenrici`: `qadd_spec`, `qsub_spec`, `qmul_spec`, `ratio_spec'` (normal form, exact value). `floor_tie`: `Q::floor` is `n / d` for `d > 0` (`ediv_eq_floor`: Lean's `⌊n/d⌋`); `qmid_spec`: `Q::mid` is `(x + y)/2`, normal |
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
