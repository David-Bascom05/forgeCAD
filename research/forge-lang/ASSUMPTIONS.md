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

`Sim.lean` holds the one induction that carries most of the rest. A run that finished is repeated
exactly (same traces, ids and log) under more fuel, and under any literal table that agrees on the
logged sites. Four more inductions of the same shape:
- `Weak.lean`: more bindings never change a run that finished.
- `Fv.lean`: a run depends only on the names its code mentions, plus the names that function and
  procedure bodies mention (`Defs.reads`).
- `Ren.lean`: renaming sites commutes with evaluation when no code mentions the renamed sites.
- `Sites.lean`: a trace's sites come only from the code, the definitions or the environment.

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
    (61 nonterminals, 47 terminals, 135 rules).
  - `grammar_ll1`: `check grammar certT = true`, run by `decide +kernel`. The kernel re-checks
    everything, with no `native_decide` and no compiler trusted. It takes about 3 s.
  - Zero conflicts, which agrees with the prototype's `tests/grammar.rs`. No Forge syntax change
    is needed for LL(1).
  - `forge_unambiguous`, `forge_roundtrip`, `forge_lossless` are the general theorems instanced.
  - `ambiguous_rejected`: `e := e '-' e | NUM` fails the check, so the check is not vacuous.
- **A syntax change is a data change.** Re-run `gen_grammar.py` and `lake build`. Nothing in
  `LL1/Proof.lean` depends on Forge.
- **Assumed, not proven: the lexer.** The theorems are about token strings. What the lexer must
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
| `Counter/*.lean` | every counterexample named above, proven by evaluation (`rfl`/`decide`) or induction |
