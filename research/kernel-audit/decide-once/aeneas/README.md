# kernel/forge-bool-core in Lean, by Aeneas, and its refinement proof (started)

The decide-once core (`kernel/forge-bool-core`) is translated by Charon and Aeneas, and the
translation is proven equal, function by function, to the Lean core it implements,
`ForgeExact.DecideOnce.core` (verify/forge-exact/ForgeExact/DecideOnce/Core.lean). That is the
obligation in decide-once-spec.md §4.

- `translate.sh`: Charon (9e193f0) and Aeneas (2e3d4488) on the crate, writing
  `lean/ForgeBoolCore/{Types,Funs}.lean`. The toolchains come from
  research/kernel-audit/coverage/aeneas/{setup,build}.sh. The OCaml build also needs `libgmp-dev`
  (apt) and `opam install odoc`, which those scripts do not install.
- `lean/`: a lake project. It requires the Aeneas Lean library at the same commit, and
  verify/forge-exact by path. `lake build` checks the translation, the refinement lemmas and the
  axiom report. It takes about 5 min once the Mathlib cache is fetched (`lake exe cache get`).

## What translates

All of it. That is 97 functions and 310 Lean definitions, with no error, no partial file and
nothing left opaque that the core itself defines. Getting there took a style pass on the Rust:
- No struct holds a borrow. The cells the core adds are an owned `Ext`, passed beside `&Ledger`.
  ("joining region abstractions containing shared loans whose shared value contains borrows" was
  the error.)
- No `break`, and no `return` inside a loop. Loops exit through their guard, using a flag.
- No `Ordering` (it gave an internal error): `cmp_cov` returns -1, 0 or 1.
- No local variable named `uf`, because it shadowed the generated namespace `uf`. The module is
  now `union_find`.

`FunsExternal.lean` (Aeneas's template, unchanged) declares eight std functions as axioms:
- the derived `PartialEq` and `Debug` of pairs;
- `Hash`;
- `Vec::default`;
- `Bool::ne`;
- the derived `PartialOrd::le` of `(u32, u32, u32)`, which `sort_triples` compares with.

None of them is used by the functions proven so far. The proof of `split_ok` will need the tuple
order. Either define it there, or have `sort_triples` compare field by field, as `cmp_cov` already
does.

## What is proven (`lean/ForgeBoolCore/Refine.lean`)

Each lemma says the Rust returns `ok` (so it does not panic, overflow or diverge) with the value
of the Lean core's function. A slice of signs is read as a covector by `covOf`
(`id ↦ cells[id]`, sign by sign).

| Rust | Lean core | lemma |
|---|---|---|
| `zero_idx` | `Cov.zeroIdx` | `zero_idx_spec` (loop: `zero_idx_loop_spec`) |
| `is_facet` | `Cov.isFacet` | `is_facet_spec` (loop: `is_facet_loop_spec`) |
| `le` | `Cov.le` | `le_spec` (loop: `le_loop_spec`) |

`lean/ForgeBoolCore/Axioms.lean` prints each lemma's axioms: propext, Classical.choice and
Quot.sound only. There is no `sorry`.

## Next, in order

1. `with_set` = `List.set`, `zeros`, the sorted index (`Index::find` returns the id with that
   covector, given no repeats), and `find` / `intern` over the ledger plus `Ext`.
2. The union–find: after the unions of a list of pairs, `find x` is the least member of `x`'s
   class. Then `Closure.head_eq` gives the Lean's `head` (spec §4's note).
3. `split_ok` = `splitOk`, `split` = `facets` / `topes` (dedup in first-seen order), `flood` /
   `membership` = `memb`, and `apply_rule`.
4. `around` = `Ledger.around`. The enumeration finds exactly the `φ` with `x ≤ φ`; sorting
   restores result order. The same for `between` / `edges_between` and `fanRel`.
5. `bad_vert` = `badVert`, then `unify` = `Ledger.unify`, then `core` = `DecideOnce.core`.
