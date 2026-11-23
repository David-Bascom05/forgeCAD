# verify/aeneas: the kernel's Rust, translated to Lean

Level L4 of research/PROOF-STANDARD.md asks that the shipped Rust refine the proven algorithm.
Here the shipped Rust itself is translated, by Charon (Rust → LLBC) and Aeneas (LLBC → Lean), and
the proofs are about that translation. They are not about a hand-written Lean model of it.

| | |
|---|---|
| `tools/aeneas/setup.sh` | the pinned toolchain: aeneas `2e3d448`, charon `9e193f0` (aeneas's `charon-pin`), OCaml 5.3.0 through opam 2.3.0, charon's own Rust nightly, Lean `v4.36.0`. It also applies `tools/aeneas/patches/`. |
| `tools/aeneas/translate.sh CRATE [--check]` | translates `kernel/CRATE`, in place and built as release, into `verify/aeneas/CRATE/lean/PKG/{Types,Funs}.lean`. With `--check` it writes nothing and fails if the committed files differ from a fresh translation. |
| `.github/workflows/aeneas.yml` | runs `translate.sh --all --check` on every change to `kernel/` or to these files, then builds the Lean. A proof cannot go stale against the code. |
| `bridge/` | the float bridge, the Lean library `AeneasF64`: Rust's `f64` is `verify/ieee754`'s binary64 |
| `CRATE/translate.conf` | what to translate (`PKG`, Charon's `--start-from` / `--opaque`) |
| `CRATE/lean/` | the translation, the definitions of its opaque functions, and the proofs |

## The float bridge

Aeneas at the pin has no floating point. It rejects `a + b` on `f64` ("Invalid inputs for binop"),
it cannot write a float literal, and it leaves the type `F64` undefined. Before this, the only way
through was to respell a copy of the code: `a.add(b)`, `f64::from_bits(..)` for each literal
(research/kernel-audit/float-verification/predicates). The proofs were then about the copy.

`tools/aeneas/patches/aeneas-f64.patch` (about 150 lines of OCaml) adds `f32`/`f64` to Aeneas:
- `+ - * / %` and unary `-` on floats are new pure operations that cannot fail;
- `== != < <= > >=` on floats are IEEE 754 comparisons, never Lean's `=`;
- `as` between floats and integers;
- a float literal is its encoding, as rustc computed it (Charon's `float_bits`), so no decimal
  string is parsed anywhere.

The Lean code calls `F64.Ops.add`, `F64.Ops.eq`, `F64.Ops.lt`, `F64.Ops.ofBits n`, `F64.Ops.ofIScalar`,
`F64.Ops.toIScalar .I64` and so on. `bridge/AeneasF64/Ops.lean` defines each one as the
`verify/ieee754` operation: `F64.Ops.add a b := IEEE754.FP.add a b` and so on, with `F64 :=
IEEE754.F64`. Library calls on floats (`abs`, `sqrt`, `to_bits`, `is_nan`, …) stay opaque to
Aeneas, and `bridge/AeneasF64/Std.lean` defines the ones the translated crates use.

These are definitions. The bridge adds no axiom: every theorem about translated float code still
rests on `propext`, `Classical.choice` and `Quot.sound` only, and each package's `Axioms.lean`
checks that.

## The float trusted base

What the definitions assume about the machine is one statement, **T**:

> **T.** On the kernel's two targets, `x86_64-unknown-linux-gnu` (`-C target-cpu=x86-64-v3`) and
> `wasm32-wasip1` (`+simd128`), with the repository's flags (`.cargo/config.toml`, the `release`
> and `wasm` profiles of `Cargo.toml`), rustc and LLVM compile each `f64` operation the bridge
> names to the IEEE 754 binary64 operation with rounding to nearest, ties to even. Subnormals are
> kept, not flushed. There is no contraction into fused multiply–add and no reassociation. And the
> program never depends on a NaN's payload or on the sign of a NaN.

Why T holds:
1. **The language.** Rust's float semantics (RFC 3514, the std docs of `f64`) specify the basic
   operations as IEEE 754 with the default rounding mode. Rust has no fast-math flags, and rustc
   never emits LLVM's `contract` or `reassoc` flags, so LLVM may not fuse or reassociate. The one
   freedom the language keeps is the NaN payload. That is why T excludes code that observes it,
   and why the model has a single NaN.
2. **The flags.** x86-64-v3 has FMA instructions, but LLVM uses them only for `mul_add` or for
   `contract` flags, and rustc never emits those flags. `.cargo/config.toml` says so, and the
   kernel calls no `mul_add`: `kernel/forge-model/tests/hidden_inputs.rs` flags `mul_add` and
   `powi`, and tools/proof/hidden-inputs.txt reviews every hit. On wasm, `+simd128` lanes round
   as the scalar op does, and the kernel does not enable relaxed-simd.
3. **The checks.**
   - `verify/ieee754/check` compares the model with the CPU bit for bit (`RESULTS.md`, about 8.8
     million cases). That covers `add sub mul div fma sqrt neg abs`, the comparisons and the
     conversions, with special values, subnormals, ties and cancellations.
   - The kernel's own results are pinned bit for bit, native against wasm32-wasip1
     (`tools/speed/bits.txt`, `examples/bits`, the platforms CI).

T is not a Lean axiom, and nothing in Lean can state it. It is the same kind of trust as trust in
Charon and Aeneas themselves: that the translation means what the compiled code does. The trusted
base of an L4 proof here is therefore:
- (a) rustc's MIR, Charon and Aeneas (the patch included) translate faithfully;
- (b) T;
- (c) Lean's kernel, with the three standard axioms.

Nothing else. In particular, no rounding behaviour is assumed: every rounding is a computation of
`IEEE754.round`.

### What is outside T

The proofs say nothing about these, and each one is marked where it occurs:
- `to_bits` of a NaN (the payload). The bridge gives the canonical quiet NaN.
- `f64::min`/`max` on `±0`. IEEE leaves the order of `-0` and `+0` open, and LLVM's `minnum`
  may return either. The bridge leaves them undefined until a crate needs them.
- `powi`, and the libm transcendentals (`sin`, `exp`, `ln`, `powf`, …). These are not correctly
  rounded and differ between platforms. The kernel's deterministic replacements are Rust in
  `forge_math::dmath`, and they translate as ordinary code.
- `f32`. The bridge defines only `f64`, so a translation that uses `f32` does not build.
