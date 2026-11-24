# Translating a new kernel crate with Aeneas (L4)

A short how-to. The full rules for writing translatable Rust are in research/aeneas/STYLE.md, and
the toolchain and float bridge are described in verify/aeneas/README.md.

## Once per machine
- `tools/aeneas/setup.sh` builds the pinned Charon and Aeneas (with OCaml through opam) into
  `~/.cache/forge-aeneas`. It takes about 30 min the first time and is idempotent;
  `setup.sh --check` prints the pins.
- elan with Lean `v4.36.0`, then `lake exe cache get` in the package (Mathlib comes through the
  Aeneas library).
- Memory: Charon needs more than 12 GB on a large file (forge-bool's boolean.rs). Use
  `--start-from` per function group, not the whole crate.

## A new crate CRATE (kernel/CRATE)
1. `verify/aeneas/CRATE/translate.conf`, shell variables (copy verify/aeneas/forge-geom's):
   - `PKG=ForgeCrate`: the Lean library name.
   - `DEPS=(forge-math forge-geom)`: the kernel crates it calls that have their own translation
     here. Their functions are then defined once, by that translation. Charon sees only the
     optimized MIR of other crates, so another crate's code is reached only through DEPS. List a
     dependency's types with `--include` (whole, when you read their fields).
   - `CHARON_ARGS=(--start-from crate::module::fn ...)`: the roots to translate. Use
     `--opaque path` for what cannot or should not translate (trait objects, caches, curved
     code). Each opaque item is an open L4 gap and goes in verify/forge-bool-proof/LEDGER.md (or
     your crate's ledger).
   - `BRIDGE=none` if the crate has no `f64`.
2. `verify/aeneas/CRATE/lean/`: `lakefile.toml` that requires the DEPS' lean packages, or
   `../../bridge` (AeneasF64) when it has no DEPS. Add `lean-toolchain`, `PKG.lean` (imports
   Funs and Axioms) and `PKG/Axioms.lean` (`#standard_axioms` over your definitions and
   theorems; copy forge-bool's).
3. `tools/aeneas/translate.sh CRATE` writes `PKG/{Types,Funs}.lean` and the
   `*External_Template.lean` files. Hand-write `PKG/FunsExternal.lean` and `TypesExternal.lean`,
   which define every opaque item the template lists, unless the bridge or a DEP already defines
   it. translate.sh fails and names any item that has no definition.
3a. If CRATE is a new Rust crate under verify/ (not kernel/), add it to Cargo.toml `members`. The
   Lean-only verify/aeneas dirs need no entry.
4. `lake build` in `verify/aeneas/CRATE/lean`. CI (.github/workflows/aeneas.yml) runs
   `translate.sh --all --check` and builds every `verify/aeneas/*/lean`, so a translation cannot
   go stale against the code. When kernel sources move, regenerate the translation, and also a
   DEP's FunsExternal_Template if its sources moved.

## Gotchas met so far
- **Floats.** `+ - * /`, comparisons, `as` and literals are IEEE binary64 through the bridge.
  Library calls (`floor`, `rem_euclid`, `min`/`max`, `next_up` …) are defined in
  `verify/aeneas/bridge/AeneasF64/Std.lean`; add new ones there as definitions, never axioms.
  `f64::min`/`max` of ±0 is the opaque `F64.zeroTie` (L4.zero): a theorem must not depend on
  the sign of a zero.
- **Exact numbers.** Integers (`i64`/`i128`/`u*`) translate with overflow checks on, so a proof
  that a function returns `ok` also rules out overflow. Heap big-integers and rationals
  (Vec-backed) translate only as far as their Vec code does. Prefer fixed-width integers or
  `[u64; N]` limbs.
- **Source shapes Aeneas rejects** (STYLE.md has the rewrites): iterator adaptors and closures
  (write index loops), slice patterns (`cs[..]` → `len()` and an index), `or_else` chains
  (nested `match`), `dyn Trait`. Rewrites must keep bits and speed: run the band
  (COORDINATOR §1, tools/speed/band.py).
- **Constants.** Another crate's `const` is an opaque `Result` global; bind it in your
  FunsExternal.lean.
- **Errors.** For a `Not_found` from Aeneas, run with `OCAMLRUNPARAM=b` to get the OCaml
  backtrace. Debug-only code (`#[cfg(debug_assertions)]`) is not translated, because the crate
  is translated as release.
- **Proofs on the translation.** `Result` is an interaction tree. Do-blocks normalize with
  `Std.bind_assoc` and `bind_ok` (not `bind_tc_ok`). Unfold well-founded loops with
  `rw [f.eq_def]`. To show what values a function can return, use
  verify/aeneas/forge-bool/lean/ForgeBool/D2Flat.lean's `Only P m` (composes through bind and
  if): `repeat first | exact Only.ok (by simp) | apply Only.ite | (apply Only.bind_any; intro _)`.
