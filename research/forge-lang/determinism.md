# forge-lang: bit-identity across platforms (log10, powi) and the SetParam check

Asked by the coordinator for HOSTED-BUILD (2026-11-25): hosted forgeCAD needs the same bits on
x86-64, aarch64 and wasm32.

## 1. `f64::log10`: replaced

`lib.rs` (rounding a refused size's suggested fit to three figures) and `taper_lint.rs` (the
taper lint's fix) took `floor(log10 |x|)` from the platform libm. At a power of ten a libm one
ulp low puts the floor one decade down, so the suggested value would have had a different number
of figures on a different target.

Both now call `units::decimal_exponent(x)`. It returns the exponent of the shortest decimal that
round-trips to `x` (`format!("{:e}")`). That decimal is computed by core's float formatting,
pure Rust with no libm, so it is the same on every target.

- Value: it is the leading digit's power of ten of the value as a person sees it. For a double
  that is the nearest to `10^k` it gives `k`, even when that double is a hair below `10^k`.
- Edge cases: it gives 0 for 0 and for a non-finite input. Before, the floor of `-inf` cast to
  `i32` gave `i32::MIN`, and `2 - i32::MIN` overflows.
- Tests: `tests/decimal_exponent.rs` checks exactness at every power of ten from 10^-307 to
  10^308, negated, one ulp above and below, and at `9.999e(k-1)`. It also checks that the result
  agrees with `log10().floor()` on values away from the powers of ten.
- Lint: the root `clippy.toml` now disallows `f64::log10`, `f64::log2` and `f64::log`. Before,
  the disallowed libm calls were sin, cos, sin_cos, tan, asin, acos, atan, atan2, cbrt, hypot,
  exp, ln, powf and mul_add. The planted check fires.

  CI reads the root file: Forgejo's `ci.yml` (`cargo clippy --workspace --all-targets -D
  warnings`) and GitHub's `clippy.yml` (from train 7, the root workspace and forge-render), so
  each disallowed method is a hard error. web.yml's wasm32 job reads
  `interface/forge-render/web/clippy/clippy.toml`, which disallows the three as well.

  forge-render had four `f64::log10` uses:
  - `shell/fgdim.rs` `tidy` sets a dragged value that is written into the part, and
    `shell/dwgcmd.rs` `grid_step` sets SNAP's step, so saved points land on it. Both now take
    `units::decimal_exponent_ceil`.
  - `shell/sketching.rs`'s drawn grid spacing and `shell/inspect.rs` `fig` (on-screen digits)
    are never saved, so they keep `log10`, allowed with that reason.

## 2. `powi`: the runtime is deterministic; constant folding is not

At runtime, `f64::powi(x, n)` lowers to `llvm.powi.f64`. That intrinsic either becomes a call to
`__powidf2` or, when `n` is a constant, a chain of multiplies built by LLVM's SelectionDAG.

- **`__powidf2` is the same everywhere.** compiler-builtins 0.1.160, `src/float/pow.rs`:
  ```rust
  let mut mul = F::ONE;
  loop { if (pow & 1) != 0 { mul *= a; } pow >>= 1; if pow == 0 { break; } a *= a; }
  if recip { F::ONE / mul } else { mul }
  ```
  This is plain IEEE multiplies and one divide, with no fused operations. libgcc's
  `__powidf2` (`libgcc2.c`) and compiler-rt's (`powidf2.c`), which can win the link on gnu and
  apple targets, do the same multiplies in the same order. So `x.powi(n)` with a runtime `x` and
  `n` gives identical bits on x86-64, aarch64 and wasm32.
- **LLVM's multiply chain for a constant `n`** (`ExpandPowI` in SelectionDAGBuilder) squares and
  multiplies in the same order. Its first factor is the square itself where the runtime has
  `1.0 * a`, which is exact. So it also gives the same bits.
- **Not deterministic: constant folding.** When `x` and `n` are both compile-time constants, after
  inlining too, LLVM's ConstantFolding evaluates `powi` with the build machine's C++ `std::pow`.
  That is one rounding of the exact power, not the runtime's chain of roundings. Measured here on
  x86-64 with glibc:

  | build | `95.65f64.powi(3)` |
  |---|---|
  | `rustc -O` (folded) | `0x412ab4acdfced918` |
  | `rustc -O0`, or a `black_box` operand | `0x412ab4acdfced917` |

  The folded value is one ulp off. Over 200,000 random bases the folded and runtime results differ
  for this share of cases:

  | `powi(n)` | differ |
  |---|---|
  | 2 | 0.08% |
  | 3 | 26% |
  | 4 | 50% |
  | 6 | 66% |

  The bits therefore depend on two things that change across platforms:
  - whether a call is inlined into a constant context (LLVM's cost model differs per target);
  - the build host's libm (glibc, Apple libm, or whatever builds the wasm).

  An exact result is safe either way: integers, powers of two, and powers of ten up to 10^22.
  `10f64.powi(k)` is safe for |k| ≤ 22, and for negative k too (one correctly rounded divide).

**Exposure today.** The interface and kernel sources have 234 `.powi(` calls outside tests. Most are
`powi(2)` of a runtime value, which is safe unless inlined into a constant. Some have a constant
base in source:
- `kernel/forge-weld/src/section.rs:794`: `24.15f64.powi(2) - 20.95f64.powi(2)` happens to match
  `x*x` for these two bases with glibc; another host's `pow` may not.
- `10f64.powi(k)` and `2f64.powi(k)` with a runtime `k`: safe as above.

**Proposed replacement, not done here.** It touches the kernel and moves bits where a fold
happened.
- Add `forge_math::dmath::powi(x, n)`: the compiler-builtins loop above, written in Rust.
  Folding a plain `fmul` is exact IEEE (APFloat), the same on any host. A folded `dmath::powi`
  equals the runtime one on every target.
- Write `powi(2)` as `x * x`.
- Disallow `f64::powi` and `f32::powi` in both clippy configs.
- The migration is mechanical. Its speed band should be flat, since it emits the same multiplies.
  `bits.txt` can change only where a fold happened. HOSTED-BUILD's native/wasm parity run is the
  check.

## 3. The SetParam check: three evaluations, now two

`edit::apply_checked_in`, which backs `forgecad edit`, the app's edits and the agent's tools, ran
the source document twice and the edited one once:
- `apply` → `param_dim` evaluated the source for the param's dimension;
- `checked_in` evaluated the source again for the errors it already had, then the edited text.

Now the source is evaluated once, lazily, in a `OnceCell`, and serves both purposes. An op refused
before evaluation still costs no run. The edited text's run is the third. That takes one of three
evaluations off every SetParam (and every op whose precheck asks a param's dimension); other ops
still take two.

The source run now has the document's `dir`, as the error check always did, so a param's
dimension is read from the same evaluation the check uses.

Further reductions, not done:
- **Two runs to one.** The app (`forge-render/src/fg.rs:1996`) and the CLI already hold the
  source's evaluation. An `apply_checked_from(before: &Run, ..)` would take it and run only the
  edited text.
- **Incremental from the changed param.** Every statement before the first that reads the param,
  directly or through a link, evaluates as before. Lean's `FamiliesRename.edit_prefix` and
  `EditClosure.edit_closure` prove this for the part model. So a run could resume from a snapshot
  of the evaluator taken at that statement. `eval::Eval` keeps no per-statement snapshots, so this
  needs an evaluator checkpoint: a larger change, and its own speed band.

## 4. Edits evaluate inside the document's own root

Every evaluation an edit makes, such as a SetParam's dimension and the check of the edited text,
now has the document's directory and the session's root.
- `edit::At { dir, root }` is passed through `apply_at`, `apply_ast_at`, `apply_checked_at` and
  `apply_all_checked_at`. `apply`, `apply_ast`, `apply_checked`, `apply_checked_in` and
  `apply_all_checked` are the same with no root, as before.
- Callers updated: the refused size's fit search (`lib.rs`, the run's `dir` and `root`), `with:`
  texts (`asm::variant_text`, the placed file's directory and the fence), an assembly's `at`
  texts (`asm::at_text`, the evaluator's), and forge-cli's session (`fg serve`, MCP: its root).
- Not changed: the desktop app has a directory but no root. The hosted server's callers are not in
  this tree and should call the `_at` forms with the session's root. `refactor::apply_checked`
  still checks with no directory.
- Test: `tests/edit_fence.rs`.
