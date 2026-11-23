# Proposal: the predicates' slow paths, written so Aeneas takes them

`predicates-slow-paths.patch` changes `kernel/forge-math/src/predicates.rs` (base 820176b) so that
every slow path of `cross_sign`, `orient2d`, `dot3` and `incircle` translates. Only the cold
integer fallback (`Big`, `*_big`) stays opaque. **Applied**, after the master accepted it as the
layout-only case below (kernel lines all ≤ base; the whole-process differences are libc
`memcpy`/`memcmp` alignment, `predicates-function-diff.txt`).

## The change

All of it is control flow and naming. No float operation and no operand order changes, so the
bits are the same by construction (and measured, below).

| function | before | after |
|---|---|---|
| `sum_into` | a `next` closure capturing `ei`, `fi` mutably | `next_component(e, f, ei, fi) -> (x, ei', fi')`, `#[inline]`, and a `while` loop |
| `scale_into` | `let Some((&first, rest)) = e.split_first() else { return 0 }`, `for &ei in rest` | `if e.is_empty() { return 0 }`, `e[0]`, `while i < e.len()` |
| `exact_diffs` | `for (o, (a, b)) in out.iter_mut().zip(pairs)` with `return None` inside | `while exact && i < N`, then `if exact { Some(out) } else { None }` |
| `cross_two` | `for v in &mut q[..nq] { *v = -*v }` | `while i < nq { q[i] = -q[i]; i += 1 }` |
| `incircle`, stage B | `[(a.x, d.x), ..].iter().all(\|&(p, q)\| two_diff(p, q).1 == 0.0)` | the six tests joined by `&&`, in the same order (same short-circuit) |
| `incircle_exact_general` | a local `lift` (shadows Aeneas's `lift`), `for &c in cr` | `sq`, `while k < cr.len()` |

`estimate` is not changed. Its `e.iter().sum()` translates as it is, and the bridge defines
std's `Iterator::sum` and `Sum for f64` (`AeneasF64/Core.lean`, a fold from `-0.0`).
`verify/aeneas/forge-math/lean/ForgeMath/Estimate.lean` proves the shipped `estimate` against
that definition today. A first version rewrote `estimate` as an index loop. That was bit-identical
but cost up to +0.40%: LLVM stopped inlining `angle_cmp` into `split_face`. So it was dropped.

With the patch, `tools/aeneas/translate.sh forge-math` (with `sum_into`, `scale_into`,
`exact_diffs`, `incircle` and `incircle_exact_general` removed from translate.conf's opaque list)
gives 41 definitions. Only the four `*_big` remain opaque, and the Lean builds.

## The gates (measured on the patch, against base 820176b)

- **Bits:** `tools/speed/bits.txt` identical, every line.
- **native = wasm:** `bits` on wasm32-wasip1 (Node, `tools/wasi-run.mjs`) equals native's (54
  lines).
- **Reliability scoreboard:** 0 worse, 0 better (`reliability --baseline` against base's).
- **`FUZZ_SEEDS=150`:** 150/150 histories of 80 features regenerate valid.
- **forge-math tests:** release and debug, all pass.
- **Speed** (`GATE_COUNTER=callgrind tools/speed/gate.sh`), base and change each measured
  against the same baseline in the same session (`predicates-gate-base.txt`,
  `predicates-gate-change.txt`):
  - kernel lines (the 22 synthetic regens and the bench parts' `seq`/`nary1`, whose collection is
    toggled on the regen call): **0 above base**, 47 equal, 5 below (−22, −59, −60);
  - language lines (`app`, `edit`, `cli`: the whole process, the `.fg` front end included): 30
    of 45 are above base, by +1 to +910 instructions (at most 0.00004%);
  - `volute edit`, the largest, was profiled function by function in the gate's exact environment.
    No Rust function runs more instructions. The +2,484 of that run is all in libc's
    `memcpy` (+1,676) and `memcmp` (+855), whose cost depends on buffer alignment: the heap
    layout moved;
  - variance: the unmodified base, run twice, differs by up to +361 on these lines. The change
    run twice differs by 0 to +3.

So the kernel executes no more instructions anywhere. The whole-process lines move by layout, by
up to about 2.5 times the base's own run-to-run spread. Under the strict rule ("no line above
base") the patch did not pass. The master accepted it as layout only.

## Applying it

```
git apply research/aeneas/proposals/predicates-slow-paths.patch
# drop sum_into, scale_into, exact_diffs, incircle, incircle_exact_general from translate.conf's --opaque
tools/aeneas/translate.sh forge-math
# drop the same from lean/ForgeMath/FunsExternal.lean, and the MutASlice into_iter opaque
cd verify/aeneas/forge-math/lean && lake build
```
