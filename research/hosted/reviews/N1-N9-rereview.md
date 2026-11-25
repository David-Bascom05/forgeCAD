# Re-review: hosted/server-hardening fixes for the N1–N9 review (2026-11-25)

Scope: `git diff 266a07f3 292494d1`, `git show 5c8eb061` and `git show 470e7e90`, in
interface/forge-server, interface/forge-sync and interface/forge-remote. Line numbers are at
470e7e90 (origin/hosted/server-hardening).

Build and tests: I built in a detached worktree at 470e7e90 (removed afterwards).
`cargo test --release -p forge-sync -p forge-server -p forge-remote` passes: remote 2, server
unit 5, close 5, e2e 3, security 16, sync unit 7, converge 10.

I wrote two repro tests for findings 1 and 2. Both pass at 470e7e90, so both bugs reproduce. They
are kept, uncommitted, at
`/tmp/claude-0/-home-user/60f48a14-bad0-5fdf-86bf-74c7519f97a2/scratchpad/n19-rereview-repro.rs`.
To run them, drop the file into `interface/forge-sync/tests/`. I confirmed finding 5 by adding
an `eprintln!` to the code temporarily.

## New findings

### 1. Medium-High: a snapshot Welcome sets each pending op's base to the snapshot's seq, so the server skips its rebase. An edit can land on the wrong target.

Where: `forge-sync/src/client.rs:220-224` (`for p in &mut self.pending { p.base = seq; }`), added
in 470e7e90.

The rewrite claims the op was made on the snapshot's text. So the server's `order()` finds
`since(S)` empty and applies the op as it was written: no rename rebase, and no StaleSpan
guard. A snapshot Welcome is not rare. It comes whenever the client is behind the floor (now
after every close, N1) or more than `CATCH_UP` (1024 entries or 1 MB) behind.

Scenario, reproduced by `snapshot_welcome_base_rewrite_hits_the_new_name`:
1. Ada edits `SetParam w = 41` at seq 0, and her link drops before it is sent.
2. Bob renames `w` to `width`, adds a new `shared w = 5`, and makes 1100 small edits.
3. Ada reconnects and gets a snapshot Welcome at seq 1102. Her op goes out with base 1102.
4. The server commits it as `SetParam w = 41`, on Bob's new `w`. The result is
   `shared width = 40` and `shared w = 41`.

The same op sent with its real base 0 is rebased to `width = 41`; the test checks both. Before
470e7e90 the server rebased it correctly (catch-up limit case) or refused it with BadBase
(compaction case). Neither of those was a silent wrong edit.

Span ops (`Respell`, `LiftLiteral`) are hit the same way. `rebase.rs` says they are "valid only
on the text they were made on". After the rewrite, a stale span is applied to a text it was not
made on.

Ops made before the first Welcome get the same treatment: base 0 becomes S.

Fix: keep `p.base` as it is. Keep the history-preserving replay locally, but send the real base.
When the real base is below the server's floor, let the server refuse with BadBase. Better, now
that M1 keeps the ops in the store, have `order()` read the missing entries from the store and
rebase through them. If resending with base S stays, do it only for ops that rebase to
themselves (no names, no spans), and refuse span ops locally.

### 2. Medium: a Resync Welcome resends ops already in flight on the same connection. A first copy that was refused is checked again from base S and can commit after the user was told it was refused.

Where: `client.rs:232-241` (every Welcome resends every pending op, 470e7e90),
`sequencer.rs:322-333` (a refused id is ordered again), and `client.rs:244-246` (an Ack for an op
no longer pending is ignored).

Scenario, reproduced by `refused_then_committed_after_resync_welcome`:
1. Ada is welcomed at seq 0. Bob commits at seq 1.
2. Ada's Resync reaches the server and is answered at once with a snapshot at S=1.
3. Ada then sends `Respell` with base 0. The server refuses it with StaleSpan, so id 1 goes into
   `refused`.
4. Ada reads the Welcome first. It rewrites the op's base to 1 and resends it.
5. Ada reads the Refused. The op leaves `pending` and the user is told it was refused.
6. The resend reaches the server. Because id 1 is in `refused` it is ordered again, from base 1.
   There is no StaleSpan now, so it commits at seq 2.
7. Ada gets the Ack, ignores it, and stays at seq 1 without the edit. The server's head has it.

Ada's next Applied or Ack hits the `seq != self.seq + 1` check, so she diverges and Resyncs
again.

M2's deferral makes the window wider. A deferred Resync is answered from `idle()`, after any ops
sent meanwhile. Every op still queued behind it is sent twice.

Fix: on a Welcome to a connection that is already welcomed (a Resync's), resend nothing. Each
in-flight op gets its own answer on this connection, in order.

The case 470e7e90 set out to fix is an op left pending after its Ack failed the seq or hash
check. Handle it in the Ack branch: drop that pending op, because the snapshot the Resync asked
for will hold it. Alternatively, have the server answer a retry of a refused id with its refusal
again (store the kind), not check it again.

### 3. Low: one connection shed while its join waits in `closing` breaks the member count. A live member's document can then be closed under it.

Where: `forge-server/src/lib.rs:1135` (a shed conn's Join is skipped, but the conn stays in
`conns`) and `:1080` (its later Leave decrements `members` of the new actor).

Scenario:
1. Doc D is closing. Y joins and X joins; both joins wait in `evs`.
2. X sends Ups before its Welcome until `evs` reaches `DOC_INBOX`. A real client never does
   this; only a misbehaving authenticated one would. X is shed.
3. `Ended` reaches the hub before X's Leave does. The new actor gets Y's join, so `members` is 1.
   X's join is skipped.
4. X's Leave arrives. `members` drops to 0 and `close_actor` runs while Y is still connected.
5. Y's later Ups land in `evs` with no Join, so the next actor drops them as "not joined".
6. Y hears nothing, not even Pongs, until its client gives up after three pings (about 60 s) and
   reconnects.

The same miscount sits in the L5 branch (`:1145`), but that branch can't be reached while
`evs` is at most `DOC_INBOX`.

Fix: when a conn's join is skipped (shed) or not delivered, `conns.remove(&k)` it. Or record per
conn whether it holds a member place, and decrement only then.

### 4. Low: after a reopen, mark LRU order is alphabetical, not recency.

Where: `sequencer.rs:180-185` (`snap.seen` is sorted by client key at `:421` and replayed through
`mark()` in that order).

Client keys are random per session (`forge-remote client_id()`), and a single active account
fills its 1000 places over time. After a reopen, each new client therefore evicts a random old
one, which can be a client that is offline with pending ops.

That client's retry of an op committed before the floor then finds no mark. With finding 1, it
is sent with base S and commits a second time. That shows up as a spurious NameTaken refusal, or
a duplicate SetParam. Without finding 1, it gets a false BadBase.

Fix: replay `seen` in ascending `seq` (the last commit), or store the LRU order.

### 5. Low (L2 is only partly fixed): `forget_abandoned` races the actor's drop and often keeps the entry.

Where: `lib.rs:1184-1237` and `:1165`.

`ended()` sends `Ended` while the thread still holds `a: Actor`. Its `Timed` checker holds the
per-name `Arc`. The hub can run `forget_abandoned` before the thread drops `a`, so it sees
`strong_count == 2` and keeps the entry.

Measured: 2 of 6 runs of `a_close_keeps_the_op_history` printed `strong_count=Some(2)`.
Entries are also never pruned after `NeverOpened` or `Quarantined`, or when an abandoned run
outlives the close.

Fix: drop `a` before `ended(...)` (call `drop(a)` explicitly). Or prune lazily in `open_doc` and
on a periodic sweep.

### 6. Low/Medium (performance): with M1, every open reads and parses the whole history, and every close appends a full copy of the text.

Where: `store.rs:147` (`read_to_end` the whole log, then JSON-parse every record) and `:280` (a
snapshot of up to `MAX_DOC`, 4 MB, plus marks, appended on every close with seq > floor).

Example: a 200 KB part closed 5000 times means about 1 GB read into memory and parsed on each
open. This runs on the actor thread, not the hub, but joins wait for it.

Fix: find the latest snapshot without a full parse. Either scan records backwards from the end,
keep a small side index of the latest snapshot's offset, or keep snapshots in a separate file.
The original review already deferred retention to its own policy.

## Checked, no bug found

**(a) Deadlock.**
- The hub never blocks on an actor: every send is a `try_send` and it calls no `join` while
  serving.
- The stop path drains `rx` while each actor thread finishes.
- Connection threads blocked on `tx.send` are unblocked by that drain.
- A hub panic drops `rx`, so `hub.send` returns an error.
- No actor holds `stats` or `abandoned` across `ended()`.

**(b) Out-of-order ids.**
- A client sends in id order on each connection.
- Pre-Welcome ops are held.
- A shed conn's suffix is dropped whole.
- The actor's slow-drop (`room.leave`) also drops a whole suffix.
- An old conn's late Up after the reconnect's resend is answered from `in_log`.
- I found no path where a never-seen id below `hwm` arrives. The one exception is the restart
  case: a reused client key with `next_id` reset to 1 would be falsely deduplicated, but
  forge-remote makes a fresh key per session.

**(b) Refused cap.** A retry of an evicted refused id needs more than 1000 newer refused ops
from the same client while it is still pending. Pending ops are resent in order on each Welcome,
so I could not build that case. Not reported.

**(b) Old snapshots.** Triples load as marks, but ids refused under the old code were never
recorded, so after the upgrade they read as committed. A retry needs a Refused lost across the
upgrade. Negligible.

**(c) Catch-up Welcome.**
- The resend plus the catch-up Ack does not double-apply: the first Ack commits, and the
  duplicate is ignored.
- The snapshot case shows the op twice in the view only until its Ack or AlreadyCommitted
  arrives. 470e7e90's test covers that.
- A reconnect while a Resync is deferred drops the deferral with the conn. The client's seq has
  not moved past the diverging entry, so the catch-up detects the divergence again.

**(d) The closing flag versus a join.** `members` counts every Join already in the actor's inbox,
and the flag is set only when `members` reaches 0. A Join after that goes to `closing.evs`.
Draining in the closing state handles a Join/Up/Leave of a conn that already left. Outside
finding 3, I found no race.

## (e) Do the tests exercise the race they name?

- `an_op_sent_while_the_document_closes_is_committed`: yes. The Join and Up both go through
  `closing.evs`, and the ≥800 ms check guards against a false pass. It does not cover Ups still in
  the old actor's inbox when the flag is set (the drain path).
- `a_lost_close_still_ends_the_actor`: yes, through the hook that drops Close. It does not fill a
  real inbox.
- `a_second_resync_is_deferred_not_dropped`: yes. Nothing covers a deferred Resync with ops
  behind it, which is finding 2.
- `a_build_past_its_budget_frees_its_slot_once`: yes.
- `a_close_keeps_the_op_history`: yes.
- converge `retries_are_answered_by_marks_and_refused_ids`: covers the mark and refused logic
  across a compaction and a reopen. It does not cover eviction (LRU or refused cap) or load
  order.
- `a_snapshot_welcome_resends_pending_and_a_committed_one_clears`: covers only an op that was
  already committed. Nothing covers an uncommitted op resent with a rewritten base (finding 1),
  or a first copy refused while the resend is in flight (finding 2).
- No test covers `shed`, the L5 branch, or the stop path with a full hub inbox.

## Original findings

| Finding | Status | Note |
|---|---|---|
| H1 | Partly fixed | The server side is right: `evs` keeps order and Welcome gating works. The client's "every Welcome resends, base rewritten" adds findings 1 and 2. |
| H2 | Fixed | Flag before `try_send`, checked every turn, inbox drained, then save. |
| H3 | Fixed | Blocking send; the stop path drains. No deadlock found. |
| M1 | Fixed | Snapshot appended, load starts from the latest. The cost is finding 6. |
| M2 | Fixed | Deferred in the actor, answered in `idle()`. It interacts with finding 2. |
| M3 | Fixed | Marks, refused ids, `in_log`. Finding 4 is minor. |
| L1 | Fixed | `End` enum. |
| L2 | Partly fixed | Finding 5. |
| L3 | Fixed | |
| L4 | Fixed | Both sides canonicalized. |
| L5 | Partly fixed | Closed and shed, but the conn stays counted (finding 3). |
