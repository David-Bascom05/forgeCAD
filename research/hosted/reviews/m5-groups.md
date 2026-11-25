# M5 review: apply_all as one atomic group

Reviewed: `hosted/m5-groups` @ f0eb1579 (diff from `hosted/server-hardening`) and the forge-render part of `hosted/m5-clients` @ f3b60093.

Every finding marked **[repro]** was run against f0eb1579 in a throwaway worktree, which has since been removed. The tests are in `/tmp/claude-0/-home-user/60f48a14-bad0-5fdf-86bf-74c7519f97a2/scratchpad/m5-review-repro.rs` and go in `interface/forge-sync/tests/`. To run them: `cargo test --release -p forge-sync --test zz_review -- --nocapture`. **[read]** means the finding comes from reading the code end to end, without running it.

## Summary

The server side does what the design asks. A group is rebased per op through other clients' ops only. It is checked once, on the result. It is written as one record and deduplicated as one. A refusal leaves nothing behind. I found no partial commit on any path.

The problems are elsewhere:
- **Unbudgeted inverse work.** `group_inverse` costs about 2.6 times the budgeted check and runs on the actor thread.
- **Client side.** The app sends edits made before the first Welcome with base 0. Groups are never checked against the caps before sending. Undo has three faults.

---

## H1. An edit made before the first Welcome is sent with base 0, not its real base. It lands on the wrong name, or is refused for no reason. [repro]

- **Where:**
  - `forge-sync/src/client.rs:213-216`: `local_group` uses `base: self.seq`, which is 0 until the first Welcome. `restore` also starts at seq 0.
  - `forge-render/src/fg.rs:2118`: `attach` runs as soon as the mark matches, before any Welcome.
  - `fg.rs:2424` and `fg.rs:2436`: `l.local` and `l.local_group` run with no `ready()` gate.
- **What happens:** the app shows and edits the file's text. That file is the server's text at some seq N (its mark is verified). The op goes out with base 0, and the server rebases it through every entry from 1 to N, reading old ones with `read_range`. Each of those rebases is wrong.
- **Scenario** (`review_edit_before_welcome_lands_on_another_name`):
  1. Bob renames `w` to `width`, then adds a new `shared w = 5`.
  2. Ada opens the part. Her file is that text. She sets `w = 7` before her Welcome arrives: offline, or during the connect.
  3. The op is sent with base 0.
  4. The head becomes `shared width = 7`, and `shared w = 5` is untouched. No one is told.
- **A second case** (`..._spurious_rename_conflict`):
  1. The history holds `w→width` and then `width→w`.
  2. Ada renames `w` to `wide` before the Welcome.
  3. She gets a RenameConflict refusal, though her rename is valid on her text.
- **Other effects:**
  - Any span op (LiftLiteral, Respell) made before the Welcome is StaleSpan as soon as the document has any history.
  - If the history has been compacted away, the edit is refused with RebaseGone.
- **Why it matters:** this breaks the agreed rule that every op carries its real base. It is the mirror image of the earlier base-rewrite finding (N1-N9 rereview §1, clients-rereview §1): now that bases are kept, a base of 0 for text at seq N goes wrong the other way. It hits every reopen of a served part during the connection window, and all offline editing after a reopen.
- **Severity:** High (a silent edit on the wrong target).
- **Fix:** do not create ops with an unknown base. Either:
  - hold or refuse typed edits in `Session::apply`/`apply_all` while `!l.ready()` (say "connecting…"), or
  - record the seq of the verified text with the mark (and in `.pending`) and have `Client` use it as `seq` while joining.

  Add the scenario above as a test.

## M1. `group_inverse` is unbudgeted O(ops × |text|) work on the document actor, about 2.6 times the check's budgeted cost. [repro]

- **Where:** `forge-sync/src/sequencer.rs:408` and `sequencer.rs:512-525`.
- **What happens:** for a group of n ops, `group_inverse` applies each op again and calls `inverse`. Each `inverse` call runs another `apply`, plus `handles()` for SetParam. That is roughly 3n full parses, after the checker's `VALIDATE_BUDGET` and outside it, on the actor's thread.
- **Measured** (`review_group_inverse_cost`, release build):

  | Document | Ops | `validate` (inside the budget) | `group_inverse` (outside it) |
  |---|---|---|---|
  | 135 KB, 8,000 `shared pN = 1` lines | 256 SetParam | 2.74 s | 7.19 s |
  | 349 KB | 256 SetParam | 10.3 s | 25.9 s |

  At 349 KB the check itself would be over budget and refused. Near 200 KB, a group that passes its 5 s check holds the actor for about 12 s more.
- **Scenario:** one account sends such a group every few seconds (`RATE` allows it). Every other collaborator on the document stalls, and `MAX_ABANDONED` never triggers, because the check finished in time.
- **Before M5:** an op's inverse cost about 3 parses.
- **Severity:** Medium. It breaks the "one VALIDATE_BUDGET per group" rule in effect.
- **Fix:** compute the inverse inside the timed job. Have `validate` return the intermediate texts (it already applies the ops in order), or have it return `inv`. Count the whole cost against the budget.

## M2. The client never applies `check_group` or `UP_MAX` before sending a group. An empty, oversize or over-long apply_all stops sync or loops forever, and a restore then drops every pending op. [read + repro]

- **Where:**
  - `fg.rs:2436` calls `local_group` directly.
  - `client.rs:213` (`local_group`) and `forge-remote/src/lib.rs` (`Session::local_group`) have no checks. Only `edit_group` calls `check_group`.
- **Over 256 ops or 0 ops:**
  1. `Up::parse` fails on the server (`wire.rs:151`, `ops_from`).
  2. The server runs `out.stop("a bad message")` (`forge-server/src/lib.rs:932`), which sends `Closing{retry:false}`.
  3. The client gets Stopped, and `pause_sync` stops syncing the part.
  4. The bad group was already written to `.pending` (`keep_pending`).
  5. On the next open, `Client::restore` returns None on that entry (`client.rs:149-150`), so every kept op is lost, the good ones included (repro: `review_restore_with_empty_group_drops_all_pending`).

  Realistic triggers:
  - Over 256 ops: Delete or Suppress of a component with many joints (`asmtools::delete_ops`/`suppress_ops`), a harness import (`AsmOps`), or a sheet "shop" `SetArg` for every sheet feature.
  - 0 ops: `AsmApplyMoves` when `moved` has no pose (`gui_asm.rs:99-117`).

  Locally, all of these are legitimate edits.
- **Over `UP_MAX` (512 KiB, for example three 200 KiB AddFeatures):**
  1. `forge_ws` rejects the frame and the reader does `Err(_) => break` (`lib.rs:922`), with no Closing message.
  2. The client reconnects and sends the same group first again.
  3. It is dropped again, forever. Every later edit queues behind it, and the user only sees "reconnecting".
- **Severity:** Medium.
- **Fix:**
  - Before calling `local_group`, have `Session::apply_all` refuse with the same caps the server applies: `check_group`, and the message size against `UP_MAX`. Say why, as for other refusals.
  - Make `Client::local_group` reject the group too (`debug_assert`, or a Result).
  - Make `restore` skip a bad entry rather than discard the whole client.

## M3. A group with one op that has no inverse gets `inv: None`, and the next undo silently takes back an older edit. [repro]

- **Where:** `sequencer.rs:512` (None for the whole group), `client.rs:364` (only `Some` is pushed) and `client.rs:255` (pops the last group that has an inverse).
- **Which ops have no inverse** (`inverse.rs:82`): `SetArg`, AddLocalParam, MakeShared, SplitInstance, LiftLiteral, Respell, and an AddFeature that binds 0 or 2 or more names. `SetArg` alone is what Apply pose, the sheet "shop" edits and others send.
- **Scenario** (`review_undo_skips_a_group_without_inverse`):
  1. Ada sets `h = 13`.
  2. Ada's apply_all `[w = 41, SetArg base.new = true]` commits with `"inv":null`.
  3. Ada presses Undo. `h` goes back to 12, `w = 41` stays, and the result is `Some(Ok)`.
  4. The app (`fg.rs:2513`) reports a successful undo of the wrong edit.
- **Why it matters:** `inverse.rs:7` promises "An op with no exact inverse has none (undo says so)". The behaviour was the same for single ops before M5, but groups make it far more common: one SetArg in a group of 50 makes the whole apply_all impossible to undo, and turns Undo into undoing something else.
- **Severity:** Medium.
- **Fix:** push `(seq, None)` as well. When the top entry is None, `undo()` returns `Err("this edit cannot be undone")` (pop it, or keep it as a barrier). Separately, give `SetArg` an inverse, which is the same changed-region approach as ReplaceFeature.

## M4. Hosted undo toggles: the second Undo re-applies the first undo instead of going one step further back. There is no redo. [repro]

- **Where:** `client.rs:364`. An undo's own Ack carries an inverse, which is pushed onto `self.undo` like any edit's.
- **Scenario** (`review_second_undo_redoes_instead_of_going_back`):
  1. Ada makes a group edit (`w = 41`, `h = 13`), then `h = 20`.
  2. The first Undo gives `h = 13`.
  3. The second Undo gives `h = 20` again. The group can never be undone.
- **Redo:** `fg.rs` turns redo off when served (`redo()` returns false), so this toggle is the only "redo" there is.
- **Introduced by M5?** No, the logic predates it. It is in scope for (4): with `attach` clearing the local undo, a served user gets exactly one undo step.
- **Severity:** Medium.
- **Fix:**
  - Mark ops sent by `undo()` (a flag in `Pending`).
  - When their Ack comes, push the inverse onto a redo stack rather than `undo`.
  - Clear the redo stack on any new local edit.

## Checked and found correct (questions 1 to 7)

1. **Rebase** (`rebase.rs:23-50`):
   - `rebase_group` maps each op through `since` on its own and skips the author's own entries, so no op passes through its own group.
   - A Rename inside the group reaches the group's later ops exactly once: the server applies them in order, and nothing is transformed through own ops.
   - A concurrent Rename in `since` (or several, inside another client's group, applied in order: chains and swaps come out right) is applied to every op, because `since.clone()` is used per op.
   - The cases I worked through all came out consistent: name reuse within a group (Delete `a`, then Add `a`) against a concurrent `a→z`, and a group rename against a concurrent same-name rename (RenameConflict).
   - StaleSpan: other clients' entries are refused in `rebase`, and the author's own earlier entries are refused by `!since.is_empty()`. The group's own earlier ops are never in `since`. So no group that should commit is refused, and no moved span gets through.
2. **Atomicity:**
   - `order()` returns before `store.append` on every refusal.
   - It appends one record, then updates the head, `in_log`, the log and recent texts in that order.
   - `compact` only drops whole entries.
   - Replay on open runs `apply_all` per record, and a torn record fails the whole open.
   - The Room sends the Ack and broadcasts only after a commit, and not for a duplicate.
   - Build hashes are per seq.
   - A refused group leaves only its id in `refused`. That is intended: a retry is checked again.
3. **Dedup:**
   - (client, id) returns the stored entry even if the contents differ. That is the intended semantics, and a correct client never reuses an id.
   - A refused id is checked again, never AlreadyCommitted.
   - The high-water mark is unchanged and correct, because one group takes one id.
   - The PROTOCOL 2 `op`/`inv` read is confined to `entry_from`.
4. **Undo:** see M3 and M4. Rebasing an undo through a collaborator's later ops works: the group's inverse is rebased locally through `recent` and sent with base = head.
5. **Limits:**
   - The 256 cap is enforced in `ops_from` (parse) and in `check_group` (sequencer). An Up with 0 or more than 256 ops is a protocol error that stops the connection.
   - `UP_MAX` bounds the bytes.
   - The check runs under one budget.
   - Quadratic or near-quadratic cost: `group_inverse` (M1). Also `rebase_group` costs n ops × the renames in `since`, through `apply_rename`, before the budget. That is small in practice and not measured.
6. **App:**
   - Single edits still go out as groups of one (`local` calls `local_group(vec![op])`).
   - The optimistic view is the app's checked text and the pending view is the client's `replay`: a group that no longer applies is skipped whole.
   - `.pending` stores `ops` arrays.
   - The web build (`websync.rs`) implements `local_group`, uses only `pub` fields (`client.joining`, `outbox`), and adds the needed web-sys features. By reading, it should compile.
   - Gap: `tests/served.rs` never calls `apply_all` on a served part.
7. **Tests:** all seven forge-sync tests and the forge-server test assert what their names claim. They are weak in three places:
   - `undo_of_a_rename_group_after_a_collaborator_uses_the_new_name`: Bob's op renames nothing, so rebasing the inverse through a concurrent rename is never exercised. Its own comment admits this.
   - No test makes a concurrent Rename rewrite two or more ops of one group (`a_group_straddling...` rewrites only one).
   - `a_refused_group_leaves_head_and_log_untouched` refuses only at apply, never at validation of the result.
