# Round 3 fixes: handoff

Four bugs the Lean proofs found (ASSUMPTIONS.md, "Round 3"). All four are fixed in the Rust
code and in the Lean model. The commit is "wip" because some gates have not run yet (listed
below).

## 1. Suppress drops neighbours on the same line

**Done.**
- `interface/forge-lang/src/edit_ops.rs`, `apply_suppress` / `apply_unsuppress`: Suppress now
  takes out exactly the statements it names. A statement that shares a line with a statement
  that stays goes on a line of its own. Its tag records each line break Suppress put in:
  `#~ID<` for the break before it, `#~ID>` for the break after it. The old tag `#~ID ` is
  unchanged. Unsuppress takes those breaks out again, so the text comes back byte for byte.
  When a break borders another feature's suppressed line, Unsuppress keeps the break and moves
  its mark onto that line. Two features suppressed on one line can therefore be put back in
  either order.
- `interface/forge-render/src/shell/fg.rs`, `suppressed()`: strips the `<` and `>` marks from
  the id.
- `src/reference.md`: the tag forms are documented.
- Lean, `ForgeLang/Suppress.lean`: `same_line_drops`, `dropLines`, `lines_eq_stmts` and
  `suppress_lines` are removed. In the new text model a line is a list of pieces (statement
  fragments and gaps).
  - `suppress_live`: the parser reads exactly the pieces of the statements that stay, in order,
    whatever shares a line.
  - `suppress_text_closure` and `suppress_stmts`: those pieces make up `keepBy (suppFrom ..)`.
  - `unsuppress_suppress`: Unsuppress after Suppress gives every line back.
  - `readTag_render`: a tagged line reads back as its marks and its text.
  - `suppress_complete` is unchanged.

  All of these build with no warnings.
- Tests: `tests/fc_parity.rs` `suppress_takes_one_statement_of_a_line` (every single
  statement, and pairs put back in both orders) passes, and so does `suppress_and_unsuppress`.
  `research/forge-lang/diff/core_diff.py` no longer lists `suppress-line` as a known finding,
  and it checks Suppress and Unsuppress on a shared line. `tests/core_diff.rs` passes.

**Left.** The Lean model has one tag only, so the mark moved onto another feature's line is not
modelled. This is documented in ASSUMPTIONS.md.

## 2. Respell renames a binder, or edits inside a token

**Done.**
- `edit_ops.rs`, `apply_respell`: parses the text and accepts the span only when it is exactly
  a `Name` node's span, using `edit::reads_at`. Anything else is the new typed error
  `Error::NotARead {span, text}` in `error.rs`. `edit.rs` now has `names_expr` and
  `names_stmts`, and `mentions` is rebuilt on them.
- Lean, `ForgeLang/Respell.lean`:
  - `respellChecked` is the op as it now is.
  - `respellChecked_read`: what the op accepts is a read token's span.
  - `respell_refuses_binder`: a binder's span is refused.
  - `respell_refuses_inside`: a span inside a token is refused. Both hold for any lexed token
    list.
  - `respell_binder_refused` and `respell_subtoken_refused`: the two old counterexamples, now
    refused.
- Tests: `fc_parity.rs` `respell_refuses_a_binder_and_a_part_of_a_token` (binder, subtoken,
  member, argument name, prefix of a read) passes. `core_diff.py` no longer lists
  `respell-binder` as known, and it checks the subtoken case too.

## 3. history_id collisions

**Done.**
- `eval.rs`: `Eval` has a new map `hids: HashMap<u64, usize>` (FNV `NameHasher`) from each
  kernel name to the feature that made it. `push_made` does one entry lookup per feature. A
  different id with the same kernel name gives `Error::HistoryIdClash {span, id, other, first}`.
  The error is also exempt from the poisoned-call swallowing in `eval.rs`, as `DuplicateId` is.
  Every feature goes through `push_made`: `push_feature`, sheet steps, thermoform and unfold.
- Audit of other places that key on `history_id`, in ASSUMPTIONS.md:
  - The builder caches key on content and hid together, so they cannot alias.
  - Sheet unfold salts use the same hid, so the new check covers them.
  - Assembly `asm::hid` on instance and joint ids: forge-asm already reports duplicates
    (`DuplicateItem`), but the message names only one of the two ids. Not changed.
- Lean, `ForgeLang/HistoryId.lean`:
  - `hidCheck` is the check `push_made` makes.
  - `checked_distinct`: with the check, no two ids share a kernel name.
  - `checked_complete`: the check refuses only a real clash.
  - `run_kernel_names`: a run that passes the check has distinct kernel names.
  - `clash_refused`: the colliding pair is refused.
  - `Axioms.lean` lists the new theorems.
- Tests: `fc_parity.rs` `a_history_id_clash_is_refused` passes.

**Left.** The callgrind gate. Build the base (6979fa5c) and the fix at equal-length paths,
then:
1. `GATE_COUNTER=callgrind GATE_REPS=3 sh tools/speed/gate.sh --save` in the base.
2. Copy the base's `tools/speed/bits.txt` into the fix's copy.
3. Run `gate.sh <base>/tools/speed/baseline-valgrind.txt` in the fix's copy.

This was started and then stopped for the move; it has not been run.

## 4. Typo fix prefers a builtin over a param

**Done.**
- `edit_ops::nearest` ranks by distance first, then puts the part's own names (params,
  features, locals, functions) before built-ins (`BUILTINS`, `shapes::NAMES`), then sorts
  alphabetically.
- Test: `fc_parity.rs` `a_typo_respells_to_the_parts_name_before_a_builtin` (`tt` with
  `shared t` gives `t`, not `pt`) passes.

## Gates

- Passed:
  - `cargo fmt --all`.
  - `cargo clippy --workspace --all-targets --release -- -D warnings`.
  - The `fc_parity` and `core_diff` release tests.
  - `lake build` of `ForgeLang.Suppress`, `ForgeLang.Respell` and `ForgeLang.HistoryId`, which
    builds all their dependencies.
- Not yet run:
  - The full `cargo test --release -p forge-lang` (it was stopped while compiling).
  - `grammar` and `parse_diff`.
  - The full `lake build`: `ForgeLang.LL1.Impl` and `ForgeLang.LL1.ImplUnifiedDoc` one at a
    time, then `lake build`, `lake build fgparse fglex`, `lake env lean CheckAxioms.lean` and
    `lake env lean Axioms.lean`.
  - The callgrind gate.
  - `cd interface/forge-render && cargo build --release`.
- Then merge `claude/repo-review-mbdrlj` again right before the final commit. Another agent is
  editing `eval.rs` and the parser, and this fix touches `eval.rs` in three small places (the
  field, its init, and `push_made`) plus the poisoned-call match.
