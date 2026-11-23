# Writing kernel Rust that Aeneas can translate

This is for code meant for level L4 (research/PROOF-STANDARD.md): Charon and Aeneas translate it
to Lean, and the proofs are about that translation. The toolchain is aeneas `2e3d448` and charon
`9e193f0`, with the float patch (`tools/aeneas/setup.sh`). Every rule below was hit in this
repository:
- by the decide-once session on `kernel/forge-bool-core` (310 definitions);
- by the coverage session on `place_cuts`;
- by this session on `kernel/forge-math` and in the module-by-module survey (research/aeneas/SURVEY.md).

How to check a change: `tools/aeneas/translate.sh CRATE` if the crate has a `translate.conf`,
otherwise `tools/aeneas/survey.sh CRATE`. A function is translatable when Aeneas reports no error
for it *and* the output elaborates in Lean (`tools/aeneas/leancheck.sh`). The second check
matters. Aeneas accepts some code whose Lean then fails to elaborate, for example every iterator
adaptor below.

A rewrite for Aeneas must keep the kernel's gates: identical bits, speed at or below base
(`GATE_COUNTER=callgrind tools/speed/gate.sh` against a baseline on the base commit),
native = wasm, the reliability scoreboard at 0 worse, and `FUZZ_SEEDS=150` at 150/150. The rewrites
below keep bits by construction: they change control flow and naming, never a float operation or
its order. Speed has to be measured, see "Speed" at the end.

## What translates

- `f64` arithmetic `+ - * /`, unary `-`, the comparisons, `as` to and from integers, and float
  literals. Through the float bridge, each is the IEEE 754 binary64 operation of verify/ieee754
  (verify/aeneas/README.md).
- `f64` library calls that the bridge defines: `abs`, `sqrt`, `is_nan`, `is_finite`,
  `is_infinite`, `from_bits`, `clone`, and the constants `EPSILON`, `MIN_POSITIVE`, `MAX`,
  `INFINITY`, `NEG_INFINITY`, `NAN`. Others (`floor`, `ceil`, `trunc`, `round`, `to_bits`,
  `copysign`, `signum`) are exact IEEE functions and can be added to
  `verify/aeneas/bridge/AeneasF64/Std.lean` when a crate needs them.
- Integers, with overflow as failure: a proof shows that it never happens.
- Structs, enums and `match`.
- `Vec`, arrays and slices with indexing.
- `while` loops.
- `for i in a..b` over a range.
- Closures that capture nothing (`incircle_exact_diffs`'s `term`), and plain generic functions
  and traits.
- `Option`/`Result` with `?`, and `if let`.

## Rules

1. **No struct holds a borrow.** A struct with a `&T` or `&mut T` field fails with "joining
   region abstractions containing shared loans whose shared value contains borrows"
   (forge-bool-core). Pass the borrowed thing as a separate argument beside the struct, or own it.
   Bits are unaffected. The speed cost is usually nil: one more argument, the same loads.
2. **No `break`, and no `return` inside a loop.** Exit through the loop condition with a flag:
   `while !done && i < n { .. }`. `exact_diffs`' `return None` inside its `for` is one case. Bits
   are the same as long as the flag adds no float operation.
3. **No `std::cmp::Ordering`, and no `cmp`, `partial_cmp`, `sort_by` or `max_by` that use it.**
   It gives an internal error. Return `-1`/`0`/`1` as `i8` (forge-bool-core's `cmp_cov`), or
   compare with `<` directly.
4. **No derived `PartialOrd` on a tuple or struct used in a comparison.** The derived
   `(u32, u32, u32)::le` became an axiom of the translation (forge-bool-core's `sort_triples`).
   Compare field by field.
5. **No iterator adaptors.** Aeneas's Lean library models `Iterator` without `zip`, `all`,
   `any`, `sum`, `map`, `filter`, `rposition`, `enumerate`, `fold` and `collect`. Aeneas accepts
   the code, and Lean then fails with "`zip` is not a field of structure
   `core.iter.traits.iterator.Iterator`". Iterating `for x in slice` or `for v in &mut s[..n]`
   leaves `into_iter`/`IterMut` opaque. Write index loops instead:
   - `for (o, (a, b)) in out.iter_mut().zip(pairs)` becomes
     `let mut i = 0; while i < N { let (a, b) = pairs[i]; ..; out[i] = ..; i += 1 }`;
   - `e.iter().sum()` becomes a loop that folds in the same order, from the same start value as
     std's `Sum` for `f64`. Check that start value against the pinned std: it decides the sign of
     an empty or all-zero sum;
   - `xs.iter().all(|&x| p(x))` becomes `p(x0) && p(x1) && ..` for a fixed length, or a `while`
     loop with a flag. Both short-circuit at the same element.
6. **No closure that captures mutably.** `sum_into`'s `next` captures `ei` and `fi` by `&mut`:
   "Unimplemented". Inline it, or make it a function that takes the state as arguments and
   returns it.
7. **Do not keep a shared borrow live across a branch that writes through a `&mut`.**
   `scale_into`'s `let Some((&first, rest)) = e.split_first()`, followed by a conditional
   `h[n] = lo`, fails with "Could not match the contexts". Index `e[0]` and loop
   `i in 1..e.len()` instead.
8. **No local names that clash with generated names.** A local `uf` shadowed the generated
   namespace `uf` (forge-bool-core). A local `lift` shadows Aeneas's `lift`, which the
   translation calls after every float operation (forge-math's `incircle_exact_general`). Avoid
   `lift`, `ok`, `fail`, `massert`, `toResult`, and the crate's own module names.
9. **No trait objects (`dyn Trait`, `Box<dyn Any>`).** "Dynamic trait types are not supported
   yet" (forge-math's `par::map`, a panic payload). Use an enum, or a generic.
10. **No `Rc`, `RefCell`, `Cell`, threads or atomics.** Aeneas has no model of shared mutable
    state. Keep these in a shell around a pure core that takes and returns owned values.
11. **No `HashMap` or `HashSet` in a core.** They are not modelled, and their iteration order is a
    hidden input anyway (tools/proof/hidden-inputs.txt). Use a sorted `Vec` with binary search,
    or a dense index array (forge-bool-core's `Index`).
12. **No `unsafe`, raw pointers or provenance casts** (forge-alloc's `usize as *const`: "Invalid
    input for unop: cast_with_exposed").
13. **No `f32` and no `mul_add` in translated code.** The bridge defines only binary64, and the
    kernel bans `mul_add` (hidden_inputs). Transcendental functions come from `forge_math::dmath`,
    which is Rust and translates as ordinary code. Never call libm's `sin`, `exp` or `powf` from a
    core.
14. **Debug-only checks stay `#[cfg(debug_assertions)]`.** The translation is of the release
    build (`cargo --release`), so these checks do not exist in it. Their conditions become the
    theorems' hypotheses: `check_filter` becomes `Relative`, `check_two_product` becomes
    `product_exact`.
15. **No NaN payload dependence.** `to_bits` of a NaN is outside the float trusted base T. Code
    whose result depends on which NaN it got cannot be proven against the model.

## The shape that works: a core and a shell

The decide-once boolean is the model to follow. The float kernel is split from the combinatorial
core:
- **The core** is the code that makes decisions. It takes decided signs, ids and tables, and
  returns a result or a refusal. It is written to the rules above, has no floats, and is proven
  against its Lean specification, function by function.
- **The float kernels** are the predicates and constructions. Since the bridge, they translate
  as well, and are proven against verify/ieee754 (forge-math's `two_product`, `orient2d`).
- **The shell** does everything else: allocation, parallelism, caches, `HashMap`s, logging. It
  calls the core and the kernels and is not translated. Its correctness obligation is small:
  it passes the core what the specification says.

A module is ready for L4 when its core and kernels translate with no opaque item a theorem needs.

## Speed

Index loops can add bounds checks that iterator code did not have. To keep speed at or below
base:
- slice once before the loop: `let h = &mut h[..n];` then `while i < h.len()`. LLVM removes the
  checks when the bound is the slice's own length;
- keep fixed-size arrays (`[f64; 4]`) where the code has them. Their indices are checked against a
  constant;
- measure every rewrite with the exact gate before keeping it:
  `GATE_COUNTER=callgrind tools/speed/gate.sh` against a baseline on the base commit.

A rewrite that costs instructions is not kept. Leave that function opaque and say so.
