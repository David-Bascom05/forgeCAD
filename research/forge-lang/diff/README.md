# Evaluator and edit ops against the core calculus

`core_diff.py` checks the Rust evaluator and edit ops (`fg`) against the Lean core calculus
(`../lean/ForgeLang/Core.lean`). Findings it produced are in
`~/projects/forgecad-gate/unified-doc/impl-proof-findings.md` (E1 to E10).

How: a random program is built as a small AST (shared params, counts, links, lets, functions,
procedures, range and keyed loops, `if`, list selection, ternaries, components and occurrences)
and printed twice, as a `.fg` and as a Lean `Doc`. Lean runs the Docs (`run`, fuel 4000) and
prints each model (id, number, handle, sites) and log. `fg run|check --json` reports features with
their provenance and which params are structural. Per program, edit ops are generated (SetParam,
DeleteFeature, MoveFeature, AddFeature, ReplaceFeature, and in components SetArg on a default,
AddComponentParam as SetArg of a new param, AddFeature at `c.`, ReplaceFeature and DeleteFeature
of `c.x`); Lean runs the edited Doc, `fg edit` applies the op and the result is run again. A
refusal must meet a Lean error and an applied op a Lean success, the edited text must be the
expected text, and the re-run is compared as before.

Fixed cases (in the same run): every counterexample of `Counter/*.lean` that maps to a `.fg`
(`concrete_cache_wrong` and its param-set form, `key_needs_shared`, `ControlUntracked`, fan-out,
zero counts, keyed ids, an observed link, index handles, `make_reads_local`, `delete_refused`,
call-path ids), static rules (`comp_scope_differs`, `use_cycle`, `dead_param`, `delete_silent`,
`index_shift`, `ref_dangles`, negative refs, `down_budget`, nested placement, defaults that read a
caller name, `use` of a component), and cross-run cache keys (`key_needs_literals`,
`key_needs_shared`, `key_needs_defs`) in one `fg serve` session, numbers and volume per version.

## Mapping

- A literal shared param (`shared w = 3`, `count(2)`) is a Lean site, and so is a component
  param's default (`cup.d`, its own handle); a loose literal is `cst`.
- A shared param whose value is an expression is a Lean `let`. fg names it in provenance
  (`driving: q`), the model names its sites: category `link`, not a finding (SetParam on it
  is InlineExpr then SetParam, both proven).
- `x = extrude(plan, e)` is `feat x e`; its depth is the number. `driving p` is the handle `p`,
  `driven [ps]` the trace's sites, `const` no sites.
- Ids: `for` has no name in fg (`e[2]`, `e[lf]`), a procedure call `l.h`, an occurrence `o.b` is
  the evaluation's `cup.b` with `o` in its occurrences.
- fg lists are strict and the model's `sel` is lazy: `strict-list` (fg errors or observes more).

## Run

```
python3 core_diff.py test --fg PATH/fg            # the fixture (seed 1, 60 programs + fixed), ~3 s
python3 core_diff.py fixture --seed 1 --count 60  # rewrite fixture/ (needs Lean: lake build first)
python3 core_diff.py long --seed 2 --seeds 5 --count 150 --fg PATH/fg   # fresh seeds (needs Lean)
```

`interface/forge-lang/tests/core_diff.rs` runs `test` with the crate's own `fg` in the normal
suite. A difference in a category that is not known (a filed finding or the mapping) fails it,
with an example. Lean is an offline tool: `fixture/expected.jsonl` holds its results and
`fixture/meta.json` the hash of the Lean file they came from, so a change to the generator that
is not followed by `fixture` is refused.
