# .fg speed work: handoff

Goal (the owner's rule): every bench part's `.fg` build is no slower than the retired `.fc` engine
was. Measured as callgrind instructions, x86-64-v2, the app's options (`checks: false, expects:
false`), against `tools/speed/fc-reference.txt`. "Done" means every row of fgcount's fga column is
<= 0, full and edit. Constraints:
- geometry stays bit-identical (`tools/speed/bits.txt`);
- no reliability loss;
- no third-party crates in `kernel/`;
- commits are signed (`git config commit.gpgsign true`; never use `git commit-tree` without `-S`).

Out of scope for now:
- Kernel numerics (root finding). They are being replaced by certified root isolation;
  see research/kernel-audit/README.md, item 4.
- Jack's kernel-side gap. It is revisited after that rewrite.
- Feature ids. They stay hashes of the feature path: positional ids would break rename-stable
  naming.

## Where it stands

Committed steps, gates passed (see each commit message for its measurements):
- Earlier, merged into claude/repo-review-mbdrlj: fast.rs (then with a fallback), the lineage
  map by name, work features sharing the stage before, dof as a check, step_solids_plain,
  the reserved-word scan only on refusal, fgcount.
- Step 1a (9d293e7): params by name (`ParamIx`), shared key sets, the lexer's byte classes,
  one name map for builtins and constants.
- Step 1b (9ff5a92):
  - fn bodies are an Rc;
  - `or`'s operand fast path;
  - lineage walks each solid's faces, with no joined, compacted copy;
  - lineage origins are an inline `Origin`, not a Vec per face;
  - `Body::face_box_unkept`, so lineage on a stage of several solids leaves the kernel's
    cached boxes alone.

Committed, gates pending or partly run:
- The standalone parser (fast.rs = Driver.lean's rdS/rdF; no fallback). See "fast.rs" below.
- Step 1c:
  - a work datum is worked out once;
  - `forge_model::finished_suffices`: no copy of the unfinished body for a part-changing
    feature;
  - sketch regions are kept on the solve;
  - built-in names are a compile-time table;
  - plain numbers are lexed exactly.

Uncommitted at the time of writing: step 1d:
- the shadowing check borrows names instead of copying them;
- call scopes are presized;
- `values::Keys` is a sorted Vec, not a BTreeSet.

Gate notes:
- forge-render's 4 chat/conversation tests need a GPU adapter. Install mesa-vulkan-drivers
  (lavapipe) in a fresh container.
- tools/speed/baseline-valgrind.txt predates the kernel merges. Some kernel-only seq/nary1
  lines read +0.2-0.3% over it even at the base commit 6979fa5. Compare against the parent
  commit's gate output, not the file.
- A bench part's seq line is `run` minus `resolve`. When a step makes the language cheaper,
  seq can rise with no kernel change:
  - `run` keeps the same language work but its regen meets a different heap;
  - on valve at 1b, realloc grew in place less often: +424k of copying;
  - `run` itself was flat (+9k) while `resolve` fell 370k.

## fast.rs (standalone)

- One function per nonterminal of grammar.ll1. Each starts with `pick`:
  - TABLE's rule, through a padded `Option<P>` table (`PICK`);
  - else note the miss (FIRST, with fk if nullable; the stmtsrest/stmts flags) and take EMPTY;
  - or stop.
- `fk` is inherited. `after(P, i, fk)` reads `AFTER`, per-rule suffix FIRST/NULLABLE tables
  worked out at compile time. Use a constant `P` at call sites so it folds.
- Missed mode never shifts.
- The five action checks sit where `enter`/`reduce` make them.
- `reserved_tok` is said first on a refusal.
- The operand fast path in `or` is guarded:
  - the lookahead starts an atom;
  - after the postfix chain it is in `TAILS_END`, the lookaheads on which every operator tail
    takes its empty rule from TABLE.
- tests/parse_diff.rs compares Debug of the whole answer with `parse_table`, errors included.
  FG_DIFF_LONG=1 finds no mismatch, but stops at the missing Lean digest for its longer
  corpora.
- Not done: the Aeneas translation and its proof against `rdS`. The core avoids trait objects
  and unsafe; it uses Box<Error>, Vec and String.

## Latest table

fgcount, before step 1a (the committed state). Percent vs fc-reference.txt; edit = cold + edit.

| part | fc full | fga full | fc edit | fga edit |
|---|---|---|---|---|
| gearbox | 263.8 | +0.4% | 524.7 | +0.2% |
| volute | 1140.0 | -1.9% | 1140.0 | +97.1%* |
| crank | 290.5 | +0.1% | 578.8 | +0.1% |
| valve | 126.8 | +2.4% | 248.1 | +2.8% |
| heatsink | 94.3 | +1.8% | 183.6 | +2.4% |
| quadarm | 51.3 | -0.1% | 100.8 | +0.1% |
| conrod | 59.8 | +2.2% | 117.5 | +3.3% |
| jack | 372.8 | +1.1% | 581.1 | +1.7% |
| frame | 436.4 | +1.8% | 731.6 | +3.1% |
| manifold | 248.4 | +0.7% | 558.7 | +1.4% |
| bracket | 10.8 | +1.4% | 20.4 | +2.2% |
| insert | 27.0 | +4.5% | 47.0 | +5.1% |
| clamp | 12.2 | +5.7% | 23.1 | +7.5% |
| hub | 26.1 | -4.7% | 47.9 | -4.5% |
| guard | 55.5 | +3.1% | 109.9 | +3.5% |

\*volute.fc had no params, so its "edit" changed nothing. It is not comparable.

The exact fc counts, more digits than the reference file:
| part | full | edit |
|---|---|---|
| gearbox | 263836739 | 524731792 |
| volute | 1139954252 | 1139953778 |
| crank | 290492878 | 578765918 |
| valve | 126844126 | 248138314 |
| heatsink | 94310850 | 183573531 |
| quadarm | 51260660 | 100781990 |
| conrod | 59779822 | 117509046 |
| jack | 372769759 | 581075448 |
| frame | 436389730 | 731585602 |
| manifold | 248417119 | 558719590 |
| bracket | 10758489 | 20384292 |
| insert | 26989906 | 46978783 |
| clamp | 12247778 | 23083624 |
| hub | 26076535 | 47902654 |
| guard | 55510136 | 109931047 |

## How to measure

```
RUSTFLAGS="-C target-cpu=x86-64-v2" CARGO_TARGET_DIR=$PWD/target-v2 cargo build -q --release \
  -p forge-lang --example fgcount --config 'profile.release.package.forge-lang.debug="line-tables-only"'
cp target-v2/release/examples/fgcount ./fgcount-vNN      # same-length names for every binary
sh research/speed/tools/count.sh ./fgcount-vNN cg vNN fga full [parts..]   # or: fga edit, fg full
```

- Copy each binary to a path of the same length before counting. ld.so's work changes with the
  binary's path length, by tens of thousands of instructions.
- Counts vary about ±20k between runs of one binary, because hash seeds are random per process.
  Don't judge a change under ~30k on one run: run each binary 2-3 times.
- `--toggle-collect` misses work on other threads, so use whole-program counts.
- Add `--config 'profile.release.package.forge-math.debug="line-tables-only"'` (or another crate)
  to get line-level kernel profiles.

The analysis helpers in research/speed/tools read profiles from `$CG` (default `cg/`):
- `byfn.py CG REGEX [N]`: exclusive cost by function, inlined code folded into its caller.
- `callers.py V PART REGEX`: a function's inclusive cost by caller, profile V against the
  `before.PART.fc.full.cg` profiles.
- `cats.py V PARTS..`: cost by category (parse, lang, sketch, kernel, malloc, std), fg against fc.
- `fnall.py PART V REGEX`.
- `tab.py`: the table from count.sh outputs.

The old `.fc` profiles (`before.*.fc.full.cg`) are in the old session's scratchpad. They can't be
regenerated: the .fc engine is deleted. Without them, compare fg profiles against each other.

## What the gaps are made of (callgrind, fg against fc)

- **Small parts** (clamp, insert, bracket, guard): the front end, spread thin.
  - clamp: lex+parse +0.32M, interpreter +0.47M (this includes font_bytes 0.28M, which fc
    counted on its side too), malloc +0.18M, kernel about 0.
  - No single function is above ~0.1M. fc's whole front end was ~0.3M.
- **frame** (+7.6M before step 1a): fg's front end is ~7.4M against fc's 3.1M.
  - Parse is 2.1M for 13.7 KB (~150 instr/byte, much of it Box/alloc per AST node).
  - lang is 4.9M:
    - ParamKey sets: mostly fixed by step 1a;
    - name lookups: `Eval::lookup` about 0.38M, scope hash per scope;
    - lexer: 0.48M;
    - `origin_upto` lineage: 0.27M;
    - fn bodies cloned at definition: 0.14M.
  - Kernel is +1.2M (nary_body and inside_polygon). That comes from the port's features and
    ids, not from the language.
- **jack** (+1.1%): the kernel's conic_quadric roots get about 2.3M more bisection steps, because
  names decide tie-breaks and so tiny floating-point differences. Out of scope (see the top).
- **valve, conrod, heatsink, manifold, gearbox**: each is a mix of parse/lang (0.5-1.7M) and a
  kernel part.

Ideas measured and dropped, all noise or worse:
- the parser checking terminal bytes instead of strings: -8k;
- memoising reads by Arc address: no hits, since every expression makes a fresh set;
- Newton-first root finding: kernel numerics, out of scope.

## Next

1. Gate and commit step 1a.
2. More of step 1, on frame and the small parts:
   - look scope names up by one hash, not one per scope (a prehashed key, or a per-scope filter);
   - make `StmtKind::Fn`'s body an `Rc` so a definition doesn't clone it;
   - fewer allocations in the parser: AST nodes in an arena or a flat vector, or at least no
     Box for leaves;
   - `origin_upto`.
3. The standalone fast.rs task (the coordinator's, verbatim intent):
   - The Lean model research/forge-lang/lean/ForgeLang/LL1/Driver.lean (`rdS`/`rdF`, theorem
     `rd_eq_run`) proves that recursive descent with an inherited follow set `fk` gives the
     table parser's accepts and errors on every input.
   - Make fast.rs standalone, with no fallback:
     1. Each function takes an inherited `fk: TermSet`. At a call site for B in `… B β`, pass
        `FIRST(β) | (NULLABLE(β) ? fk : ∅)`, with per-call-site constants generated into
        table.rs.
     2. `F<T>` becomes a Result that carries the error.
     3. On a nonterminal with no rule for the lookahead:
        - record the miss: the position, the expected set `FIRST[A] | (NULLABLE[A] ? fk : ∅)`,
          and missed = [A];
        - continue with TABLE's rule, or EMPTY, pushing each nonterminal missed;
        - never shift again: every later terminal ends the parse, with that symbol as stop.
     4. A terminal mismatch before any miss gives the set {a}.
     5. Render errors as `Parser::expected` does: "end of statement" if stmtsrest was missed,
        "a value" if stmts was, else what(stop), with toks[k]'s span.
     6. Raise the five action-check errors at the same tokens `enter` does.
     7. Run `reserved_in` first.
     8. Extend tests/parse_diff.rs to compare errors on refused inputs too.
     9. Then delete the fallback to the table parser.
   - It must stay bit-identical and no slower.
   - Keep the parser core in Aeneas's supported Rust subset (no trait objects, no unsafe; loops
     and plain enums are fine), so it can be proven equal to `rdS` directly.
     opam.ocaml.org is reachable, so Aeneas can be installed.
4. Report progress with the updated table after each committed step.

## Notes a fresh agent would rediscover

- Builder stages are `Rc<Stage>` with OnceCell view/finished/shape/joined.
  - Param-like features (sketch, param, plane, work) share the previous stage.
  - A selection's view finish is reused by the next kernel step (`finished_alone`), so it is
    not wasted work.
- The sketch solve cache is keyed by sketch id and checked with `same_bits`.
- `Solved.dof` is a `Cell<Option<usize>>`, filled when a checks run needs it.
- The app first runs `checks: false`, then a checks run on the same cache (forge-render/src/fg.rs).
  Anything moved behind `checks` must still come out of that second run.
- Each forge-render test run needs ~6 GB of target/ and over an hour. Keep at most one release
  target plus target-v2 for callgrind; disk is tight.
- `cargo test -p forge-lang --test parse_diff` compares fast.rs with the table parser (the Debug
  of the whole answer) on every generated input and every .fg file in the repository.
