# Resume note: the CDT at L4/L5 (stopped 2026-11-23)

Stopped by the owner's priority: proofs are for kernel code under active feature work, and the
tessellator's is finished. M3 (L4) and M4 (the binding) are done; L5 is begun. This note is
what a resumption needs.

## Done

- **L4.** kernel/forge-mesh/src/cdt.rs rewritten to research/aeneas/STYLE.md (on -aeneas), one
  rewrite per commit on claude/repo-review-mbdrlj-cdt-l4, then translated: `translate.conf`,
  257 definitions, no `sorry`, no axiom (`ForgeMesh/Axioms.lean`). Only `triangulate`, the
  shell that names a `dyn Refine`, is opaque.
- **The binding.** `forge_math::predicates::orient2d` in the translation *is* ForgeMath's
  translated `orient2d` (`FunsExternal.lean`, field-wise `Point2`), so ForgeMath's theorems are
  about the function the CDT calls. forge-math's `vec`, `rng`, `certainly_in_circle`,
  `ICC_BOUND`, `EPS` are translated with the CDT (`--include`). std's missing `Vec` functions,
  `mem::take` and `f64 ==` are definitions over Aeneas's list model; forge_geom's work budget is
  a pair of Lean `opaque` constants (a theorem holds whatever it answers). Debug-only
  precondition: every point the predicates see is within `RANGE` = 2^200.
- **Statements** (`ForgeMesh/Spec.lean`, Props, no `sorry`): `Ends` (T), `Correct` (C, P),
  `Settled` (the seam with research/tessellation-proof's M2).
- **Proven** ("returns ok", standard axioms only):
  - `Basic.lean`: `nx`, `pv`, `slot`, `least`, `most`, `same_chord`;
  - `Sort.lean`: `sort_keys`, all 7 loops (length kept), for fewer than 2^62 keys;
  - `Curves.lean`: `morton`, `hilbert` (invariant d + 4 s^2 <= 2^32);
  - `Search.lean`: `one`, `find_use`;
  - `Input.lean`: `bad_input`.

## Hypotheses the statements take (not proven anywhere yet)

- **OrientSound**: `orient2d` returns with the exact orientation's sign. ForgeMath proves it
  on the fast path (`orient2d_fast`); the slow path is -aeneas's stated next step.
- **InCircleSound**: `certainly_in_circle a b c d = true` implies the exact incircle
  determinant is positive (one-sided). It is what makes the Lawson flips (`legalize`, the
  constraint recovery's re-legalizing) terminate. It needs `--start-from
  forge_math::predicates::certainly_in_circle` in ForgeMath's translate.conf and the stage-A
  filter's error bound (`ICC_BOUND * permanent`). Not routed to anyone yet (master, 2026-11-23).

## Open, in order

1. `brio_order`: `Rng.below < n`; `sort_keys` permutes (so every `order[k] < n`); `grid <= 65535`
   (cases on the float clamp; `toIntSat` of a value in [0, 65535]).
2. A mesh well-formedness invariant (indices in range, neighbour links symmetric, `vtri` valid,
   counter-clockwise triangles, constraint flags matched across an edge), and under it `locate`,
   `classify`, `flip`, `split_inside`, `split_edge`, `legalize` (InCircleSound), `find_edge`.
3. `insert_constraint` (its budgets bound every loop), `insert_tangled`, `insert_loops`,
   `insert_points`, `flood`, `insert_interior`.
4. `refine` (its rounds and budget bound it), `within`; then `Ends`, `Settled`, `Correct`.

`Ends` also needs size hypotheses: point and triangle counts small enough that the
`usize`/`u32` arithmetic cannot overflow (the sort needs 2n + 16 <= usize::MAX; `n as u32` in
`bad_input` needs fewer than 2^32 points; `CEILING` = 2^23 bounds refinement).

**For `Settled` (fix-mesh-asm's question):** the seam holds for *every* inside triangle,
boundary-only ones included. Round 0 looks at every triangle with `fresh_from = SUPER`, so its
skip "all corners before `fresh_from`" excludes only super-triangle corners, which are outside
anyway. Every later triangle holds a new point (each split and each flip in `legalize` makes
triangles round the point inserted) and is in the next round's `look`. An edge between two
older points that a later round skips was asked in the round it was made. The proof is to come.

## Integration

Since the merge with main (2026-11-23): it builds in-tree against main's bridge and forge-math
translation (`tools/aeneas/translate.sh --all --check` up to date, `lake build` with the axioms
check). What follows is how it stood before that.

The Lean package builds only with -aeneas's `tools/aeneas`, `verify/aeneas/bridge` and
`verify/aeneas/forge-math` beside it (`lakefile.toml` requires `../../forge-math/lean`).
Every build here was checked in an -aeneas worktree. -aeneas's kernel/forge-math/src/predicates.rs
differs from this branch's by its own Aeneas rewrites. Once the branches are merged, rerun
`tools/aeneas/translate.sh forge-mesh` (the translation takes forge-math's signatures only, so
it should not change) and `lake build` here. The merge session handles integration; this
branch merges neither -aeneas nor fix-mesh-asm. fix-mesh-asm's later commits touch
tessellate.rs only and conflict with this branch only in Cargo.toml's `exclude`, where this
branch adds `verify/aeneas`.

## Gates (for latency-gate)

Every commit: tools/speed/bits.txt identical. `kernel/forge-model/examples/meshbits` was
identical to base 9268e0f at every commit, and native = wasm. It hashes every mesh of the suite,
fuzz and bench parts at three qualities, plus 400 random CDT regions; neither the callgrind gate
nor bits.txt reaches forge_mesh. CDT instructions over meshbits
(`--toggle-collect=forge_mesh::cdt::within*` and `zip*`): base 36.94G; zip's rewrite 36.60G;
c9-c14 +3.1% (this branch's sort, slower than std's) until 0714204; final 36.23G (-1.9%).

Callgrind gate lines over base, rerun under identical conditions with a per-function diff:
- synthetic lines: +-4 in the gate's own runs (by GATE_CPU), equal on rerun;
- diagnostic seq/nary1 lines: all in libc `__memcmp_avx2_movbe` (heap layout);
- from c12 (least/most) on, some user-facing app/edit/cli lines (crank, valve, manifold, guard)
  are +100 to +1044 instructions: libc `memcmp`, `strtoul` and unnamed libc/ld internals only, with
  no function whose source or call count changed. They are for latency-gate to judge on wall-clock.
