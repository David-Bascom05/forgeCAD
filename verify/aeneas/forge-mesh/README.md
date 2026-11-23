# kernel/forge-mesh's CDT through Aeneas (L4)

`kernel/forge-mesh/src/cdt.rs`, the constrained Delaunay triangulator, translated as it is, built
as release, from `forge_mesh::cdt` (`translate.conf`). No function is left out but `triangulate`,
the one shell that names a `dyn Refine`. `lean/ForgeMesh/Funs.lean` has 257 definitions.

## What it took

cdt.rs was rewritten to research/aeneas/STYLE.md (on the -aeneas branch), one rewrite per commit
on claude/repo-review-mbdrlj-cdt-l4: no `break` or `return` inside a loop, no mutating
closure, no `Ordering` sort (its own `sort_keys`), index loops for iterator adaptors, a ring for
the `VecDeque`, a sorted `Vec` for the `BTreeMap`, `R: Refine + ?Sized` for `&dyn Refine`,
`CdtError::Internal(Fault)` for `Internal(&'static str)`, comparisons for `f64::min`/`max`.
Each kept the kernel's gates: tools/speed/bits.txt identical; the callgrind gate's user-facing
and synthetic lines at or below base; and, since neither of those reaches the mesher,
`kernel/forge-model/examples/meshbits` (every mesh of the suite, fuzz and bench parts at three
qualities, and 400 random CDT regions, tangled ones too) identical to base at every commit, and
native = wasm. The CDT's own instruction count over meshbits is 1.9% under base.

## Bound, not assumed (`lean/ForgeMesh/FunsExternal.lean`)

- `forge_math::predicates::orient2d` is verify/aeneas/forge-math's translation, so its
  theorems there (`ForgeMath/Predicates.lean`) are about the function the CDT calls.
- forge-math's `vec` and `rng`, `certainly_in_circle`, `ICC_BOUND` and `EPS` are translated
  here, with the CDT (`--include`).
- std's `Vec::pop`, `clear`, `remove`, `split_off`, `Default for Vec`, `mem::take` and
  `f64 == f64` are written as std documents them over Aeneas's list model of `Vec`.
- `forge_geom::budget::{over, spend_many}`, a thread-local work counter that belongs to the
  shell, are Lean `opaque` constants: a theorem about the CDT holds whatever they answer.

`ForgeMesh/Axioms.lean` fails the build if `within`, `zip`, `triangulate_within` or
`triangulate_as_far` rest on any axiom but Lean's three standard ones.

## Preconditions (L5)

The predicates' proofs hold for finite inputs whose products do not overflow. cdt.rs checks,
in debug builds only, that every point its predicates see is within `RANGE` = 2^200
(`locate`, and the super-triangle).

## Building

Needs the -aeneas branch's toolchain, bridge and forge-math translation (`tools/aeneas`,
`verify/aeneas/bridge`, `verify/aeneas/forge-math`):
```
tools/aeneas/translate.sh forge-mesh      # Types.lean, Funs.lean, the templates
cd verify/aeneas/forge-mesh/lean && lake build
```

## Next

The statements of research's tessellation plan (M1/M2, with fix-mesh-asm): refinement ends or
refuses typed (`Unsettled`, `TooFine`); every constraint edge is an edge of the result; the
triangles are counter-clockwise, interior-disjoint, and cover the even-odd region of the loops.
