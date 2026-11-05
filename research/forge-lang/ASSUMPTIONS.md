# Forge language: what editability needs

A core calculus with every FastCAD-style feature switched on, the properties P1 to P6 stated over it
in Lean 4, proofs under the weakest assumptions found, and a concrete counterexample for every
assumption kept. Lean project: `lean/` (core Lean only, no Mathlib; `lake build` takes seconds, no
`sorry`, axioms are only `propext`, `Classical.choice` and `Quot.sound`).

## The answer in one screen

What editability needs, all checked:

1. **Ids from syntax.** An id is a path of statement names, call names, range indices and entry
   labels. It never comes from evaluation order, output position, a value, a string built at run
   time, or geometry.
2. **Provenance that includes control.** Every number carries the arithmetic that made it, over
   literal sites. Every site whose number was *observed* (a branch, a count, an index, a
   comparison, a solver) is logged. A logged site is a *structural* param: editing it re-runs.
   Any other site is a *data* param: editing it re-reads the traces.
3. **Edits are addressed to sites.** "Make this value 7" has one meaning only when the value is one
   bare site that drives nothing else, and then it is the same as a site edit.
4. **Single assignment**, or structural ops that check for rebinding.
5. **Some bound on evaluation**: a budget, or a type (`Count`) that makes the recursion structural.
6. **An unambiguous grammar.** It does not have to be LR(1).

What it does not need: banning functions, loops, comprehensions, conditionals, general recursion,
expressions in params, method chains or world-coordinate placement. All of them are in the core
calculus and the theorems hold with them. A deterministic *evaluation order* is not needed either.

## The core calculus (`lean/ForgeLang/Core.lean`)

- Expressions: literal (with a site), constant, variable, `+`, `*`, `==`, `c ? a : b`, list param
  indexed by a loop variable `[e0, e1][i]`, value-function call (recursive), feature reference
  `x.rel`, loop-instance reference `x[k].rel`.
- Statements: `let`, `set` (mutable reassignment), `feat`, range loop `x = for i in 0..n {..}`,
  keyed loop `x = for i in {a: e, b: e} {..}`, `if`, procedure call `x = proc(arg)` (recursive,
  makes features).
- Function and procedure bodies see their parameter and the names bound where they are called, so
  they can read top-level params and lets as FastCAD functions do.
- Ids: `Seg.nm name | Seg.ix index | Seg.key label`. A feature's id is the path to it.
- Numbers are traces (`Tr`: site, const, add, mul). Observation points log their sites.
- Evaluation consumes one unit of fuel per call. `run : Nat → Doc → Except Err (Model × Log)` is a
  total function. Id clashes are the typed error `dupId`.
- A document is a body plus a literal table `σ : Site → Int`. Printing writes a skeleton with a hole
  at each site and fills the holes from `σ` (`Print.lean`).
- `.move(d)` is arithmetic on a placement value, so `add` stands for it. Method chaining
  (`a.f(b)` = `f(a, b)`) is grammar, covered under P6.
- Components (unified document, below): `Comp` = params with defaults, a block, and the names of
  the defining document the block sees (`vis`, its shared params). `Stmt.occ x c args` is
  `x = part(c(args))`. The block runs in its own environment: the arguments (evaluated where the
  call is), the defaults left alone (evaluated in the visible names), and the visible names. Its
  features are under `x`.

`Sim.lean` holds the one induction that carries most of the rest. A run that finished is repeated
exactly (same traces, ids and log) under more fuel, and under any literal table that agrees on the
logged sites. Four more inductions of the same shape:
- `Weak.lean`: more bindings never change a run that finished.
- `Fv.lean`: a run depends only on the names its code mentions, plus the names that function and
  procedure bodies mention (`Defs.reads`).
- `Ren.lean`: renaming sites commutes with evaluation when no code mentions the renamed sites.
- `Sites.lean`: a trace's sites come only from the code, the definitions or the environment.

All five, and `Errors`, `Subst`, `Scope` and `Struct`, carry the component case, so every theorem
below holds for documents with component definitions, occurrences and linked modules.

## P1: one meaning

**Proven.** `run` is a total Lean function. `run_unique`: any two budgets that finish give the same
result. `run_ids_unique`: in a model, an id names one feature, and a clash is the typed error `dupId`.

| Assumption | Counterexample without it |
|---|---|
| The model, read as a map from id to feature, does not depend on the order an unordered collection is iterated in | `Counter/Order.two_orders`: a set iterated in two legal orders with ids by position gives two models |
| Id clashes are an error, never "last wins" | `Counter/Ids.no_call_path_clash`: without the call path, two calls of one procedure give two features with one id |
| The text parses one way | `Counter/Grammar.minus_ambiguous`, `method_ambiguous`: one token string, two trees, two values (see P6) |

**Not essential: a deterministic evaluation order.** `Counter/Order.perm_lookup`: when ids are keys,
every order of the same entries is the same map. Parallel evaluation and hash-order iteration are
fine as long as no id comes from the order.

## P2: totality

**Proven.** Evaluation is total by construction (fuel). `run_mono`: a run that finished keeps its
result under any larger budget. `run_error_stable` (`Errors.lean`): a typed error other than `fuel`
stays the same error under any larger budget. So the budget decides only "finished, or `fuel`".

| Assumption | Counterexample without it |
|---|---|
| Some bound on evaluation | `Counter/Diverge.no_value`: `fn f(n) = n == 0 ? 0 : f(n - 1)` has no value at all at `-1` in an unbudgeted semantics. With a budget the same program is the typed error `fuel` (`down_budget`) |

**Not essential: fuel itself.** If recursion and loop bounds take a `Count` (a natural number),
descent is structural and termination is a property of the program. `downNat` is the same function
as `f`, and Lean accepts it as total without a budget.

**Found:** a *global* budget makes the `fuel` error non-local. Raising a count in one feature can
exhaust the budget of a later feature that has nothing to do with it. Budget per feature, and report
the error at the feature that ran out.

## P3: one driver

**Proven.** A value's handle is `some p` exactly when its trace is the bare site `p`
(`Tr.handle`). `handle_honest`: editing `p` to `v` makes that value `v`. `sole_driver`: no literal
outside a value's trace changes it. `driven_no_handle`: a value whose trace is not one bare site has
no handle. Both edit theorems need `p ∉ log`, i.e. a data param.

| Assumption | Counterexample without it |
|---|---|
| Provenance records control dependencies, not only data | `Counter/ControlUntracked`: `feat a = c ? 5 : 7`. Without logging, `a`'s trace is the bare literal `5` and the log is empty, yet editing `c` changes `a` to 7. The handle lies |
| Params are not reassigned | `Counter/Rebind.dead_param`: `param w = 1; w = 2; feat f = w`. The param literal drives nothing, whatever it is set to |

Fan-out (one site driving many values, such as a literal in a loop body) is **not** a P3 failure.
The handle is the site, and editing it changes every value it drives.

## P4: round trip

**Proven.**

- SetParam on a data param: `setParam_roundtrip`. Evaluating the edited document gives the same
  traces, ids and log; the new numbers are the old traces re-read with the new table, without a
  re-run. Text side: `setLit_text`. Exactly one token changes and the rest is byte-identical,
  given that the site occurs once in the text (free when a site is the literal's path).
- AddFeature and DeleteFeature, both halves.
  - Sound: `delete_sound`, which reads in both directions. If the edited block evaluates, its model
    is the old model with that statement's fragment added or removed.
  - Complete: `delete_complete` and `add_complete`. If no later statement and no function body
    mentions the name the statement binds (`NotRead`), the edited block does evaluate. For Add, the
    new statement must also evaluate where it is inserted.
  - Text side: `body_text_insert`.
- MoveFeature (swap two neighbours), both halves.
  - Sound: `move_sound`. The model has the two fragments swapped.
  - Complete: `move_complete`. If neither statement mentions what the other binds, and they bind
    different names, the swap evaluates.
  - Text side: `body_text_swap`.
- SplitInstance, in general (`Split.lean`, `Sites.lean`).
  - The op: in `x = for i in 0..n { let w = p; rest }`, replace `p` with fresh literals
    `[q0, q1, ...][i]`, each set to `p`'s number.
  - `split_doc`: for any surrounding document, any `rest`, any count expression and any
    definitions, the split document evaluates, to the same ids and numbers. Its traces are the old
    ones with `q j` in place of `p` in instance `j`.
  - Hypotheses: no code mentions the `q j`, and the count fits the list (`CountFits`); otherwise
    the result is the typed error `range`.
  - `split_independent`: no feature of instance `k` carries another instance's literal.
  - `split_edit_one`: editing `q j` leaves every other instance's numbers unchanged.
- InlineExpr and MakeShared, both halves (`Shared.lean`, on `Subst.lean`).
  - InlineExpr freezes a driven param: `let x = e` becomes `let x = r`, with `r` a fresh literal
    set to the value `e` had. `inlineExpr_complete`: if the original evaluates, the frozen one
    does, and its model with `r` written as `e`'s trace is the original model.
    `inlineExpr_sound`: if the frozen one evaluates, the original does, with the same relation.
    `inlineExpr_nums`: no number moves. After the op, `x` has a handle.
  - MakeShared links a param to another: `let b = p` becomes `let b = a`, where `a` holds a value
    equal to `p`'s. `makeShared_complete` and `makeShared_sound`, in both directions: the linked
    model is the original with `p` written as `a`'s trace. `makeShared_nums`: no number moves.
    After the op, `b` and everything built from it are driven by what drives `a`.
  - Hypotheses: the literal that disappears (`p`) or appears (`r`) is mentioned by no other code.
    For MakeShared, the values are equal. For InlineExpr, `e` evaluates where it stands.
  - Both rest on `substEq`: substituting traces for sites in the environment commutes with
    evaluation, as an equation (same success or same typed error).
- A structural edit, such as a loop count, is still one meaning: evaluation is a function. The
  model op is a re-run, and P5 says what stays stable.
- If the edited document does not evaluate, the edit is refused with that run's typed error.

| Assumption | Counterexample without it |
|---|---|
| Edits are addressed to sites, not model values | `Counter/ValueEdit`. `fan_no_site_edit`: "make instance 1 differ" has no site edit at all. `two_meanings`: `a = p + q`, "make `a` 12" has two site edits with different models. `dbl_unreachable`: `a = 2 * p`, "make `a` 3" has none |
| Soundness: single assignment of the names the op touches (`FreshAfter`) | `Counter/Rebind.move_silent`: `let a = 1; a = 2; feat f = a`. Moving `f` above the reassignment evaluates fine and silently changes `f`. `delete_silent`: deleting a shadowing `let` changes `f` though the deleted statement made no feature. In single assignment the same deletion is the typed error `unbound` (`delete_refused`) |
| Completeness: nothing mentions the binding the op removes or moves (`NotRead`) | Not needed for soundness. Without it the op is refused (`delete_refused`: `unbound`), never silently wrong |
| Edited literal not observed (for the re-read form) | `Counter/ControlUntracked`, and every loop count. Those edits are re-runs, not re-reads |

SplitInstance is the op that turns a fan-out value edit into a site edit. `Counter/ValueEdit` has
the concrete instance; `Split.lean` has the general theorem.

**Found:** neither half of Add, Delete and Move needs a dependency graph.
- Soundness uses only single assignment: the edit removes bindings from what later statements
  see. With rebinding, removing a binding can uncover an older one.
- Completeness uses only a name lookup in the source (`NotRead`).
- So the editor's whole pre-check is: is the name mentioned after this point, or in a function
  body?

## P5: stable ids

**Proven.**

- SetParam on a data param changes no id (`setParam_ids`).
- Add, Delete and Move keep every other fragment, ids and traces, exactly (from the P4 theorems).
- Changing a range loop's count keeps every surviving instance exactly and only adds or drops the
  tail (`Loops.count_prefix`).
- Inserting an entry anywhere in a keyed loop keeps every other instance exactly
  (`Loops.keyed_insert`).

| Assumption | Counterexample without it |
|---|---|
| Index ids only over ranges, labels over lists that users edit | `Counter/Ids.index_shift`: inserting in front of a list moves the old `h[1]` to `h[2]`, and `h[1]` now holds another literal. `key_stable`: the same with labels keeps every id |
| Ids not from evaluation order | `position_ids_shift`: toggling a conditional feature renumbers the feature after it. Name ids do not (`name_ids_stable`) |
| Ids not from output position after a filter | `filter_renumbers`: this is FastCAD's `part ball[] = [for .. if ..]`. Checked with `fastcad eval`: `[for i in 0..4 if i != 1]` gives `ball_1, ball_2, ball_3`, and `i = 2` is `ball_2` |
| Keys are syntactic labels, not values | `value_ids_move`: ids keyed by value rename the feature when its number is edited |
| The call path (or call name) is in the id | `no_call_path_clash` |
| Ids structured, not strings | `string_ids_collide`: `"ball_" ++ "1" = "ball_1"`. FastCAD reports `part 'ball_1' declared twice`. That is fine for P1, but whether it clashes depends on a count, so a SetParam can newly fail |
| References absolute, with typed errors | `ref_dangles`: `h[2]` with two instances is `dangling`, `h[-1]` is `range`. `relative_ref_retargets`: "the last instance" is a different feature at each count, with no error |
| Selections not by world position | `nearest_retargets`: "the edge nearest this point" changes target when an unrelated feature moves (FastCAD `fillet(at=..)`) |

## P6: unambiguous grammar

**Proven, for any grammar the check accepts; checked for the Forge prototype's grammar.**
- **Grammar as data** (`LL1/Core.lean`): rules over numbered terminals and nonterminals.
  - `check G T` is a Boolean function. It verifies that the tables `T` (nullable, FIRST, FOLLOW)
    are closed under the inclusions that define them, and that every choice is made on one token.
  - The tables are a certificate: however they were computed, an unconverged table makes the check
    fail. It cannot make the check lie.
- **The theorems** (`LL1/Proof.lean`), all from `check G T = true`, whatever the grammar:
  - `unambiguous`: two parse trees of the start symbol with the same leaves are the same tree.
  - `parse_sound`: whatever the derived LL(1) parser returns is a parse tree. Lossless: its
    leaves followed by the rest of the input are the input.
  - `parse_complete`: every parse tree is found from its leaves, given enough fuel and a legal
    continuation.
  - `roundtrip`: printing a tree as its leaves and parsing gives the tree back.
  - `lossless`: a parse of a whole input has exactly that input as its leaves.
- **The Forge grammar** (`LL1/Forge.lean`).
  - Generated by `gen_grammar.py` from `interface/forge-lang/grammar.ll1` on branch `forge-lang`
    (63 nonterminals, 47 terminals, 139 rules).
  - `grammar_ll1`: `check grammar certT = true`, run by `decide +kernel`. The kernel re-checks
    everything, with no `native_decide` and no compiler trusted. It takes about 3 s.
  - Zero conflicts, which agrees with the prototype's `tests/grammar.rs`. No Forge syntax change
    is needed for LL(1).
  - `forge_unambiguous`, `forge_roundtrip`, `forge_lossless` are the general theorems instanced.
  - `ambiguous_rejected`: `e := e '-' e | NUM` fails the check, so the check is not vacuous.
- **A syntax change is a data change.** Re-run `gen_grammar.py` and `lake build`. Nothing in
  `LL1/Proof.lean` depends on Forge.
- **The lexer** (now proven of a model, `LL1/Lex.lean`, and tested against Rust; see "The
  implementation against the proof"). The theorems are about token strings. What the lexer must
  do:
  - It is a function from text to tokens, with maximal munch.
  - The prototype's lexer is context-sensitive: a leading `-` joins a number only where no value
    precedes it, and NL counts only outside `( )` and `[ ]`. This is fine as long as it is
    deterministic.
  - Tokens keep their byte spans, and whitespace and comments are kept as trivia between them.
    Then concatenating tokens with their trivia gives back the source bytes, and with
    `lossless` that is the byte-identical-edit claim.
  - Each literal a SetParam writes (sign, unit, shortest round-trip float) lexes back as one NUM
    token.

- **Essential: unambiguity** (one tree per text) and **print/parse round trip** on a lossless
  syntax tree, so that a SetParam's new token lexes as one literal. Counterexamples:
  `minus_ambiguous`, `method_ambiguous`.
- **Not essential: LR(1).** It is sufficient. A PEG is unambiguous by construction (ordered choice).
  So is an operator-precedence grammar with a total precedence and associativity table, or GLR with
  disambiguation filters. Choose LR(1) or LL(1) for the *check*: a generator such as lalrpop or
  grmtools reports conflicts, and zero conflicts proves unambiguity mechanically. A PEG can silently
  accept a parse other than the intended one.
- FastCAD hazards to avoid:
  - `:` serves both the comprehension body and the ternary (`[for i in r : c ? a : b]`).
  - `in` is both the loop keyword and the inch unit (`2in`).
  - `0..4` versus a float `0.`.
  - unary `-` before a literal: SetParam to a negative number must replace the sign too.
  - `a - b.move()` needs "methods bind tightest" (FastCAD has that rule).

## Unified document: components, calls, modules (2026-11-04)

Checks `forgecad-gate/unified-doc/design.md` ("Parts as functions, files as modules", "UI
round-trip") before its language changes merge. Findings for the implementation are also in
`forgecad-gate/unified-doc/proof-findings.md`.

### P1 to P5 with components

**Proven, no theorem changed shape.** `component NAME(params) { body }` is `Comp`, an occurrence
is `Stmt.occ`, modules are linked into `Defs.comps`. Every induction (`Sim`, `Weak`, `Fv`,
`Errors`, `Ren`, `Subst`, `Sites`, `Scope`, `Struct`) has the new case, so `run_unique`,
`run_mono`, `run_error_stable`, `handle_honest`, `sole_driver`, `setParam_roundtrip`,
`setParam_ids`, the Add/Delete/Move halves, SplitInstance, InlineExpr, MakeShared and
`scope_equiv` hold with components. The Add/Delete/Move completeness theorems now take any start
environment (a block runs in its own).

- **Weakening** holds for occurrences because the block starts from `only vis ρ`: a binding the
  block cannot see cannot change it. This is the formal reason "a component sees only the shared
  params" makes components safe to move.
- **Scope** (`scope_equiv`, extended): call-site and lexical scope agree for blocks under the rule
  that a component's params and the names its block binds are local, so no function may read
  them. `comp_scope_differs`: a param `w` named like a shared `w` that a function reads; the two
  scopes give different parts. **Rule:** a component param may not reuse a shared param's name
  (scope error).

### Memoisation (`Memo.lean`, `Counter/Memo.lean`)

**Proven.** The cache entry for a call is the *parametric* run: the block with each input name
(argument, default, visible shared param) as a fresh slot site, under a table that gives each
slot the input's number.

- `memo_inst`: a call's run is its parametric run with each slot replaced by the call's input
  trace. Both directions: one evaluates exactly when the other does.
- `memo_key`: the parametric run depends only on the key.
- `memo_sound`: an entry built for one call gives every call with an equal key exactly its own
  run, with its own provenance. The calls may be in one document (instancing, one BOM row) or in
  two (the instant-load build cache).
- `memo_nums`: instantiated traces read, under the call's table, as the entry reads under the
  entry's table, so the geometry built once is every instance's geometry.

What the key must hold, each shown necessary:

| Key part | Counterexample without it |
|---|---|
| The block's text with its literal *numbers*, not just the tree | `key_needs_literals`: same skeleton, `d * 5` vs `d * 3` |
| The definitions the block reaches (function and component bodies) | `key_needs_defs`: same block, `f(x) = x` vs `f(x) = x + x` |
| The input numbers, *including the shared params the block sees* | `key_needs_shared`: same block and argument, `W = 1` vs `W = 2` |
| The entry stored parametric (or, if concrete, the inputs' traces in the key) | `concrete_cache_wrong`: `c1 = part(cup(d: small))`, `c2 = part(cup(d: big))`, `small = big = 80`. A value-keyed concrete entry gives `c2.b` the trace of `small`: today's numbers agree, its handle is `small`, and SetParam `big = 90` leaves it at 80 while the document moves it |

Not needed: the inputs' provenance (with a parametric entry); which call site it is. Within one
run the design's key (definition, resolved arguments) is enough, since shared params and
definitions are fixed in a run. Across runs it is not. The key may drop visible names the block
and functions do not read (`indep`). The kernel version is outside the model and must be in the
key too.

### Refactors (`Refactor.lean`, `Counter/Refactor.lean`)

- **ExtractComponent and InlineComponent: proven, as one equation** (`extract_occ`).
  - An occurrence of the inline block and the same occurrence of the file form give the same
    bindings, model, ids, traces and log.
  - The file form has the passed names as params, nothing visible, and calls with `W: W` added.
  - So `OCC.NAME` references and feature ids are unchanged, and no reference is rewritten.
  - Side conditions (`ExtractPair`):
    - every visible name the block or a function reads is passed;
    - the passed names are fresh as params;
    - defaults are literals.

  | Assumption | Counterexample without it |
  |---|---|
  | Read shared params are passed (`part(FILE, with: {.., W: W})`) | `extract_naive_unbound`: the file form cannot see `W`, refused. `extract_copy_cuts_link`: copying `W`'s value into the file keeps today's numbers, but SetParam `W` then moves the inline part and not the extracted one. Silent. `extract_passes`: passing `W` is the identical run |
  | Functions the block calls are copied, not reached by `use` of the source file | `use_cycle`: the new file would `use` the file that uses it |

- **MakeComponent: proven** (`makeComponent`).
  - Moving `ms` into `component N() { ms }` and writing `o = part(N)` in their place gives the
    old model with `ms`'s fragment under `o`. Every other fragment and the log are unchanged.
  - The moved features' **ids change** (`make_ids_move`: `lid` becomes `L1.lid`). So it is a
    structural op, not an id-stable one.
  - Side conditions:
    - no later statement or function reads what moved (the design's refusal);
    - the new occurrence name is unread;
    - **missing from the design:** no moved statement reads a document name the component
      cannot see.

  | Assumption | Counterexample without it |
  |---|---|
  | Moved statements read only shared names (or the op lifts them to params) | `make_reads_local`: a moved feature reads a document `let`: `unbound` inside the block |
  | No reader outside the file | `make_breaks_placer`: a file that places this one and refers to `x.lid` now gets `dangling`. The in-file check cannot see it. The same holds for drawings that name the moved features' edges (not modelled) |

### Scoped edit ops (`Scoped.lean`)

**Proven.**

- `defs_congr`: changing one component's definition changes nothing that does not place it. The
  hypothesis `NoPlace` (no procedure or block places it) is the design's `NestedComponent` rule.
- `scoped_occ`: an occurrence of the edited component evaluates as the occurrence of a component
  whose block *is* the edited statement list. So every block op is the unscoped op on the block,
  as the design says ("no op changes shape").
- `scopedDelete_ok` and `scopedAdd_ok` treat DeleteFeature `{id: "cup.x"}` and AddFeature
  `{before: "cup.y" | "cup."}` as functions.
  - Each refuses with a typed `Refusal` (`noComp`, `noFeature`, `stillRead`, `taken`) or returns
    the edited definitions.
  - When it returns, every occurrence that evaluated before evaluates after, to its old fragment
    with the statement's part removed or added, under that occurrence. All occurrences change
    together.
- `addParam_occ`: AddComponentParam with a literal default leaves every occurrence's result
  unchanged, when no call names the new param yet and nothing reads it.
- SetArg with a literal is SetParam on that literal's site: an occurrence argument's site is in
  the body, a default's in `D.code`. `setParam_roundtrip` covers both, with its condition that the
  site is not observed. A default the block uses as a count is structural, and editing it is a
  re-run.
- `scoped_text_insert`: inserting into block `c` changes the printed document by exactly the new
  statement's tokens inside the block. P6 says the text parses back to the edited document.

### Modules (`Modules.lean`)

**Proven.**

- `load_total`: with a budget of one more than the number of files, loading the `use` graph never
  runs out. It links, or stops with the typed error `useCycle` or `noFile`. With `run`'s own budget
  (P2), evaluation with `use` is total. `use_cycle` is the concrete cycle.
- `link_inert`: linking a module's components and default part that the document does not place
  changes no run. `use` adds definitions, never statements, so a used file's top level is not
  placed.
- `ids_rooted`: every feature id starts with the name of one of the document's statements. A used
  file's features appear only under an occurrence that this document names.
- `with_unknown_param`: `part(FILE, with: {k: ..})` with `k` not a shared param of the file is the
  typed `noParam`.

### Grammar (`unified.ll1`, `LL1/Unified.lean`, `LL1/UnifiedImpl.lean`)

`gen_grammar.py` now takes a module name and an examples file. Each example is checked by kernel
`decide`: `accept` (the derived parser reads it; `parsesAll_tree` gives the parse tree), `reject`,
or `nopredict` (a table fact).

- **The proposed grammar is LL(1)** (`Unified.grammar_ll1`), so it is unambiguous and round-trips.
  It is the main grammar plus:
  - `stmt := 'component' ID '(' args ')' block`;
  - `stmt := 'drawing' drawtail`, `drawtail := ID dargs block | %empty`,
    `dargs := '(' args ')' | %empty`;
  - `arg := .. | 'component' ':' expr`.
- Accepted:
  - `component cup(d: 80, h: 95) { .. }` and `component lid(at: .., ground: false) { }`;
  - `part(cup(d: small), at: pose(at: (0, 0, 0)))` and `part(cup, at: p)`;
  - `t = use("f.fg")` then `part(t.cup(d: 90))`;
  - `part("mug.fg", with: {d: 70})`;
  - `revolute(c2.body.side(c2.s.o), c1.body.end)`;
  - `drawing dwg901(title: ..) { .. part(component: "lid") }` and `drawing NAME { }`;
  - a drawing file's `drawing` header line;
  - `part("box.fg", component: "lid")`.
- Rejected: `x = component c() {}`. A definition is not an expression.
- `part` and `component` do not conflict, because `part` stays an identifier (a call) and
  `component` is a statement keyword. `use(..)` member access is ordinary postfix `.ID(args)`, and
  `with:` is an identifier keyword argument. Zero conflicts confirms all three.
- **The branch's grammar** (`unified-impl.ll1`, equal to unified-doc's working tree on 2026-11-04)
  is LL(1), but it **rejects `part(component: "lid")`** (`UnifiedImpl.reject_component_kwarg`).
  - `component` is a keyword there, and no alternative of `args` or `arg` is chosen on it
    (`nopredict_*`).
  - Fix: `arg := .. | 'component' ':' expr`, as for `at`.
  - Drawing blocks are not in it yet. When `drawing` becomes a keyword, the header line must stay
    a statement (`drawtail := .. | %empty`), or every drawing file stops parsing.

### Verdicts

| Question | Verdict |
|---|---|
| P1 to P5 with components, calls, occurrences, modules | **Hold**, every theorem, no new hypothesis. Weakening is *stronger*: a block cannot see what it is not given |
| Memoisation soundness | **Proven**, with the key: block text and literal numbers, reachable definitions, input numbers including visible shared params, kernel version; entry stored parametric. Provenance not needed. A value key with concrete entries is unsound |
| Id stability, Extract/Inline | **Proven**: same model, ids and traces. Read shared params must be passed, not copied |
| Id stability, MakeComponent | **Fails by design**: moved ids gain the occurrence prefix. Model otherwise equal. Needs the second refusal rule (moved statements read only shared names) |
| Edit-op round trip, scoped | **Proven** for Delete and Add (refuse or apply, at every occurrence), the general scoping theorem for any block op, AddComponentParam, SetArg with literals, text locality |
| Modules: termination, cycles, top level not placed | **Proven** |
| Grammar LL(1) | **Proven** for the proposed grammar. The branch's grammar rejects `part(component: ..)` |

### Recommendations for the syntax and the ops

1. Add `'component' ':' expr` to `arg` (or rename the drawing kwarg).
2. Keep the `drawing` header a statement when `drawing` becomes a keyword.
3. A component param may not reuse a shared param's name (`comp_scope_differs`).
4. The build cache key holds the definition text with literal numbers, every definition it
   reaches, input numbers including the shared params read, and the kernel version. Store entries
   parametric, or put the inputs' traces in the key.
5. ExtractComponent passes read shared params (`with: {W: W}`) and copies called functions.
   InlineComponent drops the `W: W` arguments.
6. MakeComponent refuses (or lifts to a param) when a moved statement reads a non-shared document
   name. Its dialog says the moved features get new ids. Refuse it when other files or drawings
   reference the moved bodies, or rewrite them.
7. Keep `NestedComponent` (a block places no component) until procedures and nested placement
   are modelled. The scoped-op theorems use it.

### Not mechanised

- No-parse of `part(component: ..)` under the branch's grammar is the derived parser's verdict at
  fuel 400 plus the table fact `nopredict_*`. That every tree would need an `args` node starting
  with `component` is not proven.
- Lexer: `component`, `drawing` and `use` as keywords or identifiers, maximal munch (as before).
- MoveFeature and ReplaceFeature in a block are covered by `scoped_occ` with the unscoped
  theorems, but not stated as op functions like Delete and Add.
- Scoped ops on components that other components or procedures place (needs `NestedComponent`).
- Defaults that read names. Defaults are literals in the Extract theorem, and in the semantics a
  default does not see earlier params.
- Module qualification of names (`file#comp` as one string), diamond `use` loaded twice, and
  `use` paths fenced by the session root.
- Body components (a body named by a joint), joints, poses, BOM rows beyond "one row per key",
  drawing block contents, the kernel.

## The implementation against the proof (impl-proof, 2026-11-04)

What ties the Rust in `interface/forge-lang` to the theorems above. Mismatches found are in
`forgecad-gate/unified-doc/impl-proof-findings.md` (P for the parser, E for the evaluator).

### Parser: one grammar, the table proven (`LL1/Impl.lean`)

- **One source.** `tests/grammar.rs` writes `src/syntax/table.rs` and `LL1/Impl.lean` from the
  same `Grammar` read from `grammar.ll1`. `the_lean_grammar_is_current` fails when Impl.lean is
  stale, as `the_table_is_current` does for table.rs. Impl.lean holds table.rs's LHS, RHS,
  NULLABLE, FIRST, FOLLOW, FOLLOW_END and TABLE literally, and the lexer's KEYWORDS and OPS.
- `Impl.grammar_ll1`: `check grammar certT = true`, with `certT` table.rs's own sets. The sets the
  parser ships are a valid certificate, so `impl_unambiguous` and `impl_roundtrip` hold for the
  parser's grammar (`decide +kernel`, about 90 s).
- `Impl.table_choose`: `∀ A < NN, ∀ a < NT, choose grammar certT A (some a) = tableAt A a`. On
  every cell, TABLE names the rule the proven parser `parse` expands by, and NONE exactly where it
  has none.
- `Impl.source_to_tree`: lexing with the model (below) and parsing the tokens gives a parse tree
  with exactly those tokens, and the lexed pieces are the source byte for byte.
- Checked on de9bb4ee's grammar too (generated the same way; all theorems hold).

### Lexer (`LL1/Lex.lean`)

A model of `lex_into` on bytes, parametrised by the keyword and operator lists.
- `lex_lossless`: `lex C s = .ok ps → (ps.map Piece.text).flatten = s`.
- `word_maximal`: a name or keyword piece is never followed by a name byte.
- `num_literal`: a literal `-`? digits (`.` digits)? (`e` `-`? digits)? unit, followed by a byte
  that is not a digit, letter or dot, is one number piece with that unit, and the rest is left
  as it was. A negative literal needs a place no value precedes (always so in the grammar's
  literal positions). With `tests/lexer.rs` (SetParam writes only this shape) this is the P6
  lexer assumption, now proven of the model.
- Determinism is free (a function).

### Differential (`tests/parse_diff.rs`)

What the proofs cannot reach, the stack driver in `syntax.rs` (chains, kept values, error mode),
its actions and the Rust lexer, is run against the proven objects:
- the Lean parser (`lake exe fgparse`, `Oracle/Parse.lean`: the proven `parse`, nothing else) and
  the lexer model (`lake exe fglex`);
- in the suite through a reference port of `parse` whose answers are the oracle's by digest
  (`tests/parse_diff.digest`); `FG_LEAN=1` re-asks Lean answer by answer and rewrites it;
- inputs: every token string of up to 4 tokens whose proper prefixes are viable (99018 on main,
  102893 on de9bb4ee: every sentence that short and every first error), 3000 random sentences
  (seed 0x5eed_f00d_cafe_0001) and one random edit of each, every .fg file in the repository with
  a token cut at 8 places; for the lexer every 1 to 3 byte text over a 30-byte alphabet, 8000
  random texts and 2000 random programs (37931 texts). `FG_DIFF_LONG=1`: 5 tokens, 40000
  sentences, 4 bytes, 100000 texts;
- required: the parser accepts exactly the oracle's sentences, except for the checks its actions
  add (assignment and keyword to a name, positional after keyword, a param's owner, an integer
  instance index), and its tree equals the tree of the text printed from the oracle's tree with
  every operation bracketed (left-associated chains, right-associated `^`), spans aside. The
  lexer's pieces, kinds and lengths, or its error and span, equal the model's.
- Result: no difference, on main and on de9bb4ee. On de9bb4ee also the long run, answer by
  answer against Lean: 1303029 strings of up to 5 tokens, 70030 sentences and edits, 939931
  lexer texts (about 18 minutes). Two AST quirks (P1, P2) are noted. `LL1/ImplUnifiedDoc.lean`
  is de9bb4ee's generated file, kept until that branch merges.

### Evaluator and edit ops (`diff/`, `ScopedMore.lean`, `Counter/Qualify.lean`)

`diff/core_diff.py` prints one random program as a `.fg` and as a core-calculus `Doc` and
compares `fg run --json` / `fg edit` with Lean's `run`: ids, numbers, handles, structural
params, refusals, edited text (`diff/README.md`). Every counterexample that maps to a `.fg` is a
fixed case. It found E1 to E10 (component evaluations shared by value, observed links, defaults
seeing the caller, `use` of components, zero-count loops, and others).

Closed in Lean:
- `move_down`, `move_up`, `scopedMove`/`scopedMove_ok`: MoveFeature inside a block, as an op
  function, refusing or giving every occurrence a permutation of its fragment and log.
- `replace_complete`, `scopedReplace`/`scopedReplace_ok`: ReplaceFeature with the same binder.
- `occ_body`: any block op that preserves "evaluates, related by Rel" holds at every occurrence
  (under `NoPlace`).
- `default_no_capture`: a default that reads a caller's local is `unbound` (E10 is Rust's side).
- `occ_caller_indep`: an occurrence depends only on its arguments' names and `Defs.reads`.
- `unqualified_shadow`, `qualify_string_collides`: unqualified module names silently take the
  first module; `file#comp` as one string collides unless `#` is barred from names.

Rerun on unified-doc 27252794 and 72aa22c8: E1 to E8 and E10 fixed as the model says, E9 a
typed `NotYet` for now; the parser and lexer diff unchanged and clean (long run on 27252794). The
harness now gives component defaults their own sites (E6's fix: a default is its own handle).
One new low-severity point: fg refuses an edit only on an error kind the document lacked.

### Still not mechanised

- The Rust stack driver itself (tested, not proven), and the action checks that refuse some
  sentences.
- Scoped ops under nested placement (only fg's `InComponent` refusal is tested).
- MakeComponent, Extract and Inline as Rust ops (fg has none yet).
- Diamond `use` and `use` paths fenced to the session root.

## Round 2: nesting and in-context references (`Nested.lean`, `Counter/Nested.lean`)

The core already holds both: a block may place components, and an argument may be a reference to
an occurrence written above. Findings also in `forgecad-gate/unified-doc/proof-findings.md`.

- `defs_congr_closed`: editing `c0` changes nothing that places only components of a set closed
  under placement that excludes `c0` (`Avoids`), at any depth. `NoPlace` (NestedComponent) is the
  special case (`noPlace_avoids`); `avoids_reach` builds the set from reachability.
- `scoped_occ_closed`: an occurrence of the edited component at any depth runs as the edited block.
- `occ_pre`, `pre_pre`, `nested_ids`: ids are path-joined (`box1.lid1.plate`).
- `context_trace`, `context_key_needs_ref`: a reference argument's provenance is the referenced
  geometry's sites, and its value is an input in the key (memo_sound as for numbers).
- `context_forward_unbound`, `context_loop_unbound`: a reference reaches only what is written
  above, so no context cycle can be written (ContextCycle is that refusal).
- Rules forced: `place_cycle_fuel` (a self-placing component never finishes: a static
  `ComponentCycle` check, `rank_acyclic`), `nested_delete_dangles` (a scoped delete must check
  references through occurrence paths in every placing block).
- Not yet: scoped ops at depth as op functions (the relational lift with that check); poses.

## Verdicts on the hypotheses

| Hypothesis | Verdict |
|---|---|
| Essential: deterministic evaluation | **Refined.** What is essential is a unique model as an id → feature map. Evaluation order is not (`perm_lookup`). Determinism of each primitive (kernel ops, solvers) is assumed, as in `research/rebuild` |
| Essential: provenance, unique source site | **Refined, stronger.** Data provenance alone is not enough: control dependencies must be recorded (`ControlUntracked`). "Unique site" is the definition of a handle, not an assumption, and one site may drive many values |
| Essential: ids from names, call path, keys | **Confirmed**, with detail. Index keys are fine for ranges (prefix-closed) and break for edited lists. Keys must be labels, not values. Ids must be structured, not strings |
| Essential: edits defined on source sites | **Confirmed.** Value-addressed edits fail in three ways. When they work, they are site edits |
| Essential: fuel or bounds | **Refined.** Some bound is essential; fuel is not (`Count` makes recursion structural) |
| Not essential: forbidding loops, functions, comprehensions, conditionals, recursion | **Confirmed.** All are in the core and every theorem covers them. A recursion depth or loop count is a structural param |
| Not essential: LR(1) | **Confirmed.** Unambiguity is essential; the grammar class is not |
| Unsure: mutable variables | They make values no more driven than `let` does, and P1, P2, P3 and SetParam hold with `set`. They **break** Add, Delete and Move (silent change) and param honesty. Safe only if params cannot be reassigned and the structural ops check `FreshAfter` |
| Unsure: index-keyed loops | Inserting into a list shifts every later id (`index_shift`). Ranges are fine (`count_prefix`) |
| Unsure: references to loop instances when counts change | Safe when references are absolute and out-of-range or negative is a typed error. Relative references (`last`, `-1`) re-target silently |
| Unsure: conditionals that add or remove features | Safe with name ids (`name_ids_stable`). A reference to an absent feature is a typed error. Position ids break |
| `.move([x,y,z])` placement | **Touches no property.** It is a value (driven when chained), and the edit has one meaning. *Selection* by coordinates is what breaks P5, for references |

**Found beyond the hypotheses:**

- A ternary in *value* position can be a trace operation (the value is driven) instead of a control
  point (the condition is structural). Both are sound. Only "the trace of the chosen branch" is
  unsound.
- Solvers (`solve(f, lo, hi, target=)`, sketch constraints with guesses) observe their inputs, so
  every param feeding them is structural. They must be deterministic (fixed iteration, no
  thread-order reductions).
- FastCAD's "last top-level expression becomes `main`" is a positional id. Adding a statement after
  it renames the part (the same shape as `position_ids_shift`).
- DESIGN's rule "numeric literals only in params" is not needed for editability. In the core,
  literals sit anywhere and every theorem holds, because a literal's site is its path. Keep the rule
  if it is wanted for intent.

## FastCAD language features

| Feature | Verdict | Where |
|---|---|---|
| `param x = 80;` | safe | site with a handle (`handle_honest`) |
| `let x = 3;` (literal) | safe | the literal is a site (its path) |
| `let x = expr` / `param a = b*2` | safe, value driven, no handle | `driven_no_handle`, `dbl_unreachable` |
| `let` shadowing (accepted: `let a = 1; let a = 2;` evaluates, checked) | breaks P4/P5 for Delete and Move (silently) | `delete_silent`. Safe if forbidden, or if ops check `FreshAfter` |
| `fn f(x) = e`, block bodies, default args | safe | core value functions; a default literal is a site with fan-out |
| recursive `fn` (crank's `round_each`) | safe with a bound | `down_budget`; the depth argument is structural |
| `part name = e` | safe | name id |
| `part n[] = [for i in 0..k : ..]` → `n_1..n_k` | breaks P5 (output position, strings) | `filter_renumbers`, `string_ids_collide`. Safe as `n[i]` with structured ids |
| `[for x in [a, b] : ..]` over a literal list | breaks P5 under insertion | `index_shift`. Safe with labelled entries (`key_stable`) |
| `[for i in 0..n : ..]` | safe | `count_prefix` |
| comprehension `if` filter | safe iff ids come from the iteration key | `filter_renumbers` |
| `c ? a : b`, comparisons, `&&`, `||`, `!` | safe with control tracking | `ControlUntracked` |
| method chaining `box(10).move(z=5)` | safe; grammar needs "methods bind tightest" | `method_ambiguous` |
| `.move([x,y,z])`, `.rotate`, `.orient(at=)` | safe | values; driven when chained |
| `fillet(at=[..])`, `inside=[box]`, `chamfer(at=)` | breaks P5 for references | `nearest_retargets` |
| `v[-1]` on numbers | safe (the index is observed) | |
| `holes[-1]` / relative references to features | breaks P5 | `relative_ref_retargets` |
| `part.at("hole_1")`, `at("hole_*")` (kernel-numbered anchors) | breaks P5 if numbering follows evaluation or geometry order | safe if the anchor id is the hole feature's id |
| `"plunger_" + i` built names | breaks P1/P5 (collisions, renames) | `string_ids_collide` |
| `"ball_*"` globs | safe as a query, not a reference | |
| `solve(..)`, sketch constraint solve with guesses | safe if deterministic; inputs become structural | |
| implicit `main` = last expression | breaks P5 | positional id |
| `use "vendor"` | safe if imported ids carry the module path | |
| units `2in`, floats `0.5`, ranges `0..4` | grammar hazards | P6 |
| `expect` | safe; it observes, so its inputs are structural | |

## Recommendations for the Forge syntax

1. **Ids.**
   - `name`; `name[i]` for `for i in 0..n`; `name[label]` for `for p in {nw: .., ne: ..}`.
   - The binding name of a call is a path segment: `l = boss(..)` gives `l.b`.
   - Structured, never built from strings.
   - Nothing positional: no implicit `main`, no output-position numbering.
2. **Lists users insert into carry labels.** Unlabelled `for x in list` is fine only for computed
   lists; give it no per-element handles.
3. **Single assignment.**
   - No `set` and no shadowing.
   - Accumulators become folds or comprehensions.
   - Add, Delete and Move are then sound with no dependency analysis.
   - If mutation is kept, params cannot be reassigned and the ops check `FreshAfter`.
4. **Trace every number; log observations.**
   - A param in the log is shown as structural (count, flag, index, solver input); editing it
     re-runs.
   - Any other param is re-read: a per-feature cache can update its dimensions without re-running
     control.
   - This is the formal version of DESIGN's "driving/driven disjoint".
5. **Edits name sites.**
   - A UI drag on a value resolves to its bare site when the fan-out is 1.
   - With fan-out above 1, it offers "changes N instances" or SplitInstance.
   - On a driven value, it offers InlineExpr or a typed refusal. Never a heuristic inverse.
6. **References.**
   - Absolute: `holes[2]`, `holes[nw]`.
   - Out of range is `dangling` and negative is an error. No `last` or `-1` for features.
   - Edges and faces by meaning (DESIGN already says so), never by coordinates.
7. **Termination.**
   - `Count` for loop bounds and recursion arguments: structural, no budget needed.
   - General recursion only with a per-feature budget, reported at the feature.
8. **Grammar.**
   - Keyword-led statements; a precedence table with method calls tightest.
   - Do not reuse `:` for both the ternary and the comprehension.
   - No trailing-dot floats; unit suffixes that are not keywords (or none: DESIGN has `Length`).
   - A negative literal is one token in literal position.
   - A lossless CST, so that SetParam replaces exactly the literal's span (sign and unit included).
   - Shortest round-trip float printing.
   - Check with an LR(1) generator (zero conflicts) and a property test `parse(print(t)) = t`.
9. **Functions** stay FastCAD-like: pure, and they may read top-level names. `Scope.lean` proves
   `scope_equiv`: call-site scope and lexical scope (bodies see the top-level environment) give
   the same result or the same typed error, under one rule. The rule: no local name (loop
   variable, parameter, anything bound in a nested block or a procedure body) is a name any
   function body reads. That also forces bodies to mention only parameters and top-level names.
   Top-level rebinding does not matter to this theorem. `scope_differs`: without the rule, a loop
   variable `w` captures a function's top-level `w` under call-site scope. Make the rule a
   scope error.

## Open (not mechanised)

- SplitInstance on other shapes (a literal deep inside an expression, splits across nested loops).
  The proven form is the canonical `let w = p` at the head of the loop body. Other shapes rewrite to
  it.
- P6 lexer properties (above). The core calculus's own printer (`Print.lean`) is not run
  through `LL1`: the real grammar is.
- AddLocalParam and other ops not yet named.
- Features inside an `if` block are not referenceable after it (block scope in the core).
- SetParam on a structural param is stable only at loop level (`count_prefix`), not as a statement
  about whole documents.
- Geometry: one number per feature. Kernel ops and solvers are assumed to be functions.

## File map

| File | What |
|---|---|
| `Core.lean` | syntax, traces, ids, evaluator, `run` |
| `Sim.lean` | `sim`: more fuel and agreeing tables repeat a run |
| `Weak.lean` | `weak`: more bindings repeat a run |
| `Edits.lean` | P1 `run_unique`, `run_ids_unique`; P2 `run_mono`; P3 `handle_honest`, `sole_driver`; P4 `setParam_roundtrip`; P5 `setParam_ids` |
| `Errors.lean` | P2 `run_error_stable` |
| `Struct.lean` | P4/P5 soundness: `delete_sound` (Add and Delete), `move_sound` |
| `Fv.lean` | `indep`: the free-name lemma |
| `Complete.lean` | P4/P5 completeness: `delete_complete`, `add_complete`, `move_complete` |
| `Ren.lean` | `ren`: site renaming commutes with evaluation |
| `Split.lean` | SplitInstance: `split_doc` |
| `Sites.lean` | `sitesIn`; `split_independent`, `split_edit_one` |
| `Subst.lean` | `substEq`: substitution commutes with evaluation (equation) |
| `Shared.lean` | InlineExpr, MakeShared: `inlineExpr_complete`/`_sound`/`_nums`, `makeShared_complete`/`_sound`/`_nums` |
| `Scope.lean` | lexical evaluator `runL`; `scope_equiv`, `scope_differs` |
| `LL1/Core.lean` | grammars as data; `check` (closure of the tables, LL(1)); `conflicts` for reports |
| `LL1/Proof.lean` | P6: `unambiguous`, `parse_sound`, `parse_complete`, `roundtrip`, `lossless` |
| `LL1/Forge.lean` | generated: the prototype's grammar, certificate tables, `grammar_ll1` by kernel `decide` |
| `gen_grammar.py` | regenerates `LL1/Forge.lean` from `grammar.ll1` |
| `Loops.lean` | P5 `count_prefix`, `keyed_insert` |
| `Print.lean` | P4 text: `setLit_text`, `body_text_insert`, `body_text_swap` |
| `Memo.lean` | memoisation: `memo_inst`, `memo_key`, `memo_sound`, `memo_nums` |
| `Refactor.lean` | `extract_occ` (Extract/Inline), `makeComponent`; argument-list lemmas |
| `Scoped.lean` | `defs_congr`, `scoped_occ`, `scopedDelete_ok`, `scopedAdd_ok`, `addParam_occ`, `scoped_text_insert` |
| `Modules.lean` | `load_total`, `link_inert`, `ids_rooted`, `use_cycle`, `with_unknown_param` |
| `LL1/Unified.lean` | generated from `unified.ll1` + `unified.examples`: the proposed grammar, `grammar_ll1`, examples |
| `LL1/UnifiedImpl.lean` | generated from `unified-impl.ll1`: the branch's grammar, `reject_component_kwarg` |
| `LL1/Impl.lean` | generated by `interface/forge-lang/tests/grammar.rs`: the parser's grammar, sets and table; `grammar_ll1`, `table_choose`, `source_to_tree` |
| `LL1/Lex.lean` | the lexer model: `lex_lossless`, `word_maximal`, `num_literal` |
| `Oracle/*.lean` | `lake exe fgparse`, `lake exe fglex`: the oracles of `tests/parse_diff.rs` |
| `ScopedMore.lean` | `scopedMove_ok`, `scopedReplace_ok`, `occ_body`, `default_no_capture`, `occ_caller_indep` |
| `Counter/Qualify.lean` | `unqualified_shadow`, `qualify_string_collides` |
| `Nested.lean` | `defs_congr_closed`, `scoped_occ_closed`, `avoids_reach`, `rank_acyclic`, `occ_pre` |
| `Counter/Nested.lean` | `nested_ids`, `context_*`, `place_cycle_fuel`, `nested_delete_dangles` |
| `Axioms.lean` | `lake env lean Axioms.lean`: the axioms of every main theorem |
| `Counter/*.lean` | every counterexample named above, proven by evaluation (`rfl`/`decide`) or induction |
