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

## Opaque, and what makes each translatable

These are Lean `opaque` constants (`lean/ForgeMath/FunsExternal.lean`): there is no axiom, and
no theorem can use their result. Every theorem above is about a path that does not reach them.

| item | why | the Aeneas-friendly form (bits and speed unchanged) |
|---|---|---|
| `sum_into` | its `next` closure captures `ei`, `fi` mutably | inline `next` as a `while` loop over indices (the ieee copy's form) |
| `scale_into` | Aeneas: "Could not match the contexts" at the `if lo != 0.0 { h[n] = lo; .. }` join. The likely cause is that `rest`, a shared borrow from `split_first`, is live across it. | index `e` directly (`e[0]`, `while i < e.len()`) instead of `split_first` |
| `exact_diffs` | `out.iter_mut().zip(pairs)` and a `return` inside the loop: no `zip`, no early return | a `while` loop over `i` with a `done` flag |
| `incircle` | stage B's `[..].iter().all(..)`: no `all` | six `two_diff(..).1 == 0.0` joined by `&&` (same short-circuit order) |
| `incircle_exact_general` (reached only through `incircle`) | a local named `lift` shadows Aeneas's `lift` | rename the closure |
| `cross_big`, `orient2d_big`, `dot3_big`, `incircle_big`, `Big` | iterator chains, `rposition`, `overflowing_add` on `bool` casts, const generic arrays | the cold integer fallback. Leave it opaque, or rewrite it with index loops. It runs only when a slow path overflows to NaN. |
| the iterator adaptor of `cross_two`'s `for v in &mut q[..nq]` | `IterMut` | `while i < nq { q[i] = -q[i]; i += 1 }` |

`research/aeneas/proposals/predicates.md` has these rewrites as a patch, with every gate measured.
Bits, native = wasm, the scoreboard and `FUZZ_SEEDS=150` all pass, and the kernel's instruction
counts are at or below base on every line. Some whole-process language lines move by up to +910
instructions from heap layout, so it is left for the owner. With them applied, the slow paths of `cross_sign`, `orient2d`, `dot3` and `incircle`
translate too, as 41 definitions with only the `Big` fallback opaque. Then the expansion arithmetic (Shewchuk's `sum_into` and `scale_into` invariants:
nonoverlapping, increasing magnitude, exact sum) can be proven against the model. That is L4 for
the predicates in full. Each rewrite is a kernel change and must pass the owner's gates (bits,
callgrind speed ≤ base, native = wasm, scoreboard, `FUZZ_SEEDS=150`).

## Rebuilding

```
tools/aeneas/setup.sh                     # once: the pinned toolchain (~40 min)
tools/aeneas/translate.sh forge-math      # Types.lean, Funs.lean, the templates
cd verify/aeneas/forge-math/lean && lake exe cache get && lake build
```
