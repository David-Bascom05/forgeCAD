# Re-review: client fixes (hosted/client-native 48c2fc38, server-hardening 12082dfb + dc53dbbb)

2026-11-25. Scope:
- `git log 470e7e90..origin/hosted/server-hardening` (12082dfb PROTOCOL 2 / Closing, dc53dbbb
  saved/restore; 7bf5e019 is docs only);
- `git diff 44c5c6d7 origin/hosted/client-native` in forge-render, forge-remote and forge-sync. The
  client changes are 48c2fc38 (fg.rs, sync_link.rs, gui.rs, shell/banner.rs, tests/served.rs). The
  rest of the diff comes from main merges.
- client-web: fb0afcef is **not pushed**. origin/hosted/client-web is still 5fc726bb and does not
  contain 12082dfb, so the browser client still speaks PROTOCOL 1.

Line numbers are at origin/hosted/client-native (48c2fc38). forge-sync/client.rs is the same at
origin/hosted/server-hardening.

Build: I built a detached worktree at 48c2fc38 (/var/tmp/wt-cr2, with its own target dir; both are
now removed). I ran three scratch tests there:
- the earlier repro, N1-N9-rereview-repro.rs;
- two new forge-sync tests;
- one forge-remote test.

All of them pass, which means each bug reproduces. They are kept, uncommitted, in
`/tmp/claude-0/-home-user/60f48a14-bad0-5fdf-86bf-74c7519f97a2/scratchpad/cr2-repro/`:
- `cr2_repro.rs`: copy into interface/forge-sync/tests/
- `cr2_backoff.rs`: copy into interface/forge-remote/tests/

I did not run the full suites or served.rs.

## Re-review finding 1 (base rewritten to the snapshot's seq): STILL PRESENT, and now on the main path

`forge-sync/src/client.rs:276-278` is unchanged:

```rust
for p in &mut self.pending {
    p.base = seq;
}
```

The repro still reproduces against PROTOCOL 2 at 48c2fc38. It needed no changes:
- `snapshot_welcome_base_rewrite_hits_the_new_name` passes. With the base rewritten, the head is
  `shared width = 40` plus `shared w = 41`. With the real base it is `shared width = 41`.
- Re-review finding 2 (`refused_then_committed_after_resync_welcome`) also still passes. Every
  Welcome still resends every pending op (client.rs:286-295).

dc53dbbb's own test, `a_saved_client_restores_its_pending_ops_and_ids` (converge.rs), asserts the
rewritten base (`base: 5`). See new finding 1 for why this is now worse.

## New findings

### 1. HIGH: every restored `.pending` goes through the base rewrite. An offline edit lands on the wrong target after a collaborator's Rename.

Where:
- `client.rs:139-156`: `restore` builds the client with `Client::new`, so `seq` is 0 and
  `joining` is true.
- `client.rs:183`: Hello carries `seq: None` while joining.
- `room.rs:79-80`: `seq: None` is always answered with a snapshot.
- `client.rs:276-278`: the snapshot rewrites each pending op's base.

So every H2(a) resume sends its kept ops with base = the current head. The real base is in the
`.pending` file (`"base":0`) and is thrown away. The CATCH_UP limit does not need to be exceeded:
two ops in between are enough.

Scenario, reproduced by `restored_pending_lands_on_the_renamed_target`:
1. Ada sets `w = 41` while offline, then closes the app. The kept file is
   `{"client":"ada-c","next":2,"ops":["{…\"base\":0,\"id\":1,…SetParam w 41}"]}`.
2. Bob renames `w` to `width` and adds a new `shared w = 5`.
3. Ada opens the part. The op is resent as `Op { base: 2, id: 1, SetParam w = 41 }`.
4. The head becomes `shared width = 40`, `shared w = 41`. Ada's edit is lost, and Bob's new
   parameter is overwritten. No one is told.

Span ops (`Respell`, `LiftLiteral`) are applied the same way to a text they were not made on.

A related problem: `.pending` holds no seq, head hash or server identity. A kept file restored
against a different history is applied blindly to whatever text the server now has, with no
refusal. That happens when the server's `.forgecad/server/` is wiped, or when remote.toml is
pointed at another server.

Fix:
- Keep `p.base` (re-review fix 1). Save `seq` in `.pending`, and say Hello with it, so the server
  either catches up and rebases or answers BadBase.
- Have the server rebase through stored entries below the floor, or refuse with BadBase so the
  client can show it as a §4.5 branch.
- Save a server/log id (a nonce made when the log is created, sent in Welcome) with the kept ops.
  On a mismatch, refuse to resend them and keep them as a branch.
- Fix the dc53dbbb test to assert the real base.

### 2. MEDIUM: a shared or stale `.pending` reuses op ids. The server takes the new edit as a retry and drops it without telling the user, and the client books someone else's op as its own.

Where: `sync_link.rs:111-116` (every open restores the same client id and `next`),
`client.rs:154`, `sequencer.rs:314-327` (dedup by (client, id)), and `client.rs:297-334` (the
Ack commits the server's op in place of the user's).

Scenario, reproduced by `two_restores_of_one_pending_file_drop_an_edit_silently`:
1. Two app instances open the same part, so both restore the same `.pending`. A copied project
   folder does the same.
2. Instance A sets `w = 41` as id 1. Instance B sets `h = 99`, also as id 1.
3. The server answers B with A's Ack. B commits A's op and its `h = 99` disappears.
4. B gets no `Refused` event (its events are `[Changed]`, with 0 pending). B also pushes A's
   inverse onto its own undo stack, so B's next Undo reverts A's edit.

Before dc53dbbb, every session got a fresh `client_id()`, so this could not happen.

The same id reuse happens when `.pending` falls behind the ids actually sent:
- `keep_pending` (`fg.rs:2078`) runs only on the next `sync_poll`, and nothing writes it on
  `Drop` (`fg.rs:1812`). If the app quits or crashes after an op is sent and acked but before the
  next poll, the next open reuses that id for a new edit, which is then dropped.
- `keep_pending` ignores a failed `put` (`sync_link.rs:237`). A read-only or full disk leaves an
  old `.pending` that is restored again on each open. Its old ops are resent too. If the marks have
  evicted them (re-review finding 4), they commit twice.

Fix:
- Take a lock on `.pending`, for example an O_EXCL lockfile. A second instance then starts a fresh
  client id and does not write the file.
- Reserve ids in blocks: write `next = N+64` before using ids N..N+63, so a stale file never
  reissues an id that went out.
- Write `.pending` in `Drop` as well.
- Report a failed `put` and a failed `restore` to the user, not silently.

### 3. MEDIUM: the reconnect has no backoff after a server close, so a retry close, or either direction of PROTOCOL 2 skew, reconnects about 20 times a second, for good.

Where: `forge-remote/src/lib.rs:448` resets `backoff` to 50 ms on every successful TCP connect.
Line 469 breaks on an unparseable message, and line 512 sleeps for that 50 ms. The same code is
at 44c5c6d7 (lines 410 and 431), so released v1 clients behave this way too.

Measured with `reconnect_rate_after_a_retry_close`, a fake server that answers Hello and closes,
over 2 s:

| The server answers | Connections in 2 s | Events in the app |
|---|---|---|
| `closing` with retry true ("too busy", "too many people") | 39 | 39 Closed notes |
| an unparseable message: what a v1 client sees from a v2 server's `closing` | 39 | 0 |
| a v1 server's `error` "protocol 2: this server speaks 1": a v2 client against an old server | 39 | `Said(...)` |

So:
- **A v1 client against a v2 server.** This includes the browser client, still at 5fc726bb. It
  cannot parse `{"t":"closing"}`, so it says nothing and reconnects at 20 Hz for ever, and the
  user never sees why. The protocol bump does not do its job: the refusal is written in a
  vocabulary only v2 can read.
- **A v2 client against a v1 server.** The refusal comes as `Down::Error`, which is now
  `Event::Said`, "the connection stays". So the v2 client never stops and never pauses.
- **A 21st member of a full document** (`too many people`, retry true), or a busy actor, hammers
  the server at 20 Hz per document.

Fix:
- In `run`, reset the backoff only after a Welcome, and add jitter.
- In the client, treat any `Error` or `Closing` received before this connection's Welcome
  (`!welcomed`) as Stopped when it is an Error, or as a backed-off retry when it is a Closing.
- On the server, send the door refusal for a protocol mismatch as the legacy `error` message, so v1
  clients show it. Better still, send the protocol refusal in both forms.

### 4. MEDIUM: a fixable `retry:false` (an expired or wrong token, a server-side open error) ends in a pause that cannot be undone if the user saves meanwhile. The queued edits are then never sent.

Where:
- `fg.rs:2068`: Stopped calls `pause_sync`.
- `fg.rs:1980`: `pause_sync` sets `sync_file = None`.
- `fg.rs:2394-2397`: `save` updates the mark only through `sync_file`, so a save while paused
  leaves the mark alone.
- `sync_link.rs:187-191`: on the next open, the mark does not match the file, so the part pauses
  as a "hand edit".
- `fg.rs:1931-1934`: a paused part never starts its link.

Scenario:
1. The token in remotes.toml expires. The server sends `Closing{retry:false}` with "a valid token
   is needed", and the part pauses. The banner says edits stay local.
2. The user keeps working and presses Ctrl+S. The agent's save before each turn, at
   `gui_agent.rs:982`, does the same.
3. The user fixes the token and reopens the part. It now reads "was edited outside the app since
   it last synced". The pause is permanent.
4. The ops kept in `.pending` while the token was bad are never sent.

The only way out is to delete `.forgecad/sync/<doc>.hash` by hand. That then goes through the H1
`sync_wait` path. There, `sync_link::open` (`fg.rs:1933`) calls `Session::resume`, which sends the
stale `.pending` ops at the Welcome (with the base rewritten, finding 1) **before** `verify` runs,
even when `verify` then refuses to attach.

Server errors that are only passing are also sent with `retry:false`: `out.stop(&e)` for
`NeverOpened` (forge-server `lib.rs:1225`, `:1174`, `:1141`), where `open_doc` (`:1275-1294`)
fails on any `metadata` or read I/O error or a store error. For example, the server's file can be
briefly missing during a `git checkout` in the server's folder.

Fix:
- While paused for a refusal, keep the `SyncFile` mark tracking: the app's own saves are not hand
  edits.
- Add a Retry action to the banner that re-reads remotes.toml and reopens the link.
- Start the link only after `verify` succeeds, or hold its outbox until then.
- Send I/O open errors as `retry:true`, with backoff (finding 3).

### 5. LOW-MEDIUM: a hand-edit pause throws away the user's unacked edits from the editor.

Where: `fg.rs:1989-2000`. `pause_for_hand_edit` replaces `src` with the disk text whenever they
differ.

Scenario:
1. The user edits while offline, so the ops are pending and shown in the view.
2. A formatter or a `git checkout` touches the file. The next `tick` pauses.
3. The editor now shows the disk text. The user's offline edits vanish from the screen, with no
   undo: hosted undo left with the link.
4. They survive only as JSON in `.pending`, which the now-paused part never sends (finding 4).

Fix: when `pending() > 0`, keep the view text as a side branch, for example
`<doc>.fg.mine`, or as an editor tab. Say so in the banner.

### 6. LOW: `sync_wait` gives no sign while the server is unreachable.

Where: `fg.rs:1954`. Entering the wait sets no note and no banner. `sync_wait_poll`
(`fg.rs:2004-2026`) reports only messages the server sends, and a failed connect sends none.

Scenario: the first open of a freshly served part while the server is down. The part behaves as
LOCAL with nothing on screen. The user edits and saves. When the server comes up, `verify`
refuses (the editor differs), and the user then learns the part was never synced.

Fix: say "Waiting for the server to check this part" while waiting, in the banner or the status
line.

### 7. LOW: the write order after a crash causes a false hand-edit pause, and `.pending` writes are not durable.

Where:
- `sync_link.rs:278-280`: `tick` renames the file, then writes the mark.
- `fg.rs:2390-2397`: `save` does the same.
- `sync_link.rs:246`: the mark is written with a plain, non-atomic `fs::write`.
- `put` (`:127-135`) has no fsync.

Scenarios:
- A crash between the rename and the mark write leaves file = new text and mark = old. The next
  open pauses as "edited outside the app", and finding 4 makes that permanent.
- A power loss can leave `.pending` empty. `restore` then returns None, and the app silently
  starts a new client id, losing the kept ops.

Fix:
- Write the mark atomically, before the file, and accept either the old or the new hash on open.
- fsync `.pending` before the rename.
- Report a `.pending` that fails to parse.

### 8. LOW: M6 is narrowed, not closed.

Where: `sync_link.rs:270-278`. An editor's save that lands between the second read and the
rename, which includes writing the temporary file, is still renamed over with no trace. An editor
that saves by rename (vim, VS Code) is replaced whole.

Fix:
- On Linux, use `renameat2(RENAME_EXCHANGE)` and hash the file that was swapped out. If it is not
  `last`, swap it back and pause.
- Elsewhere, hard-link the old file aside before the rename, and compare it after.

## Original client findings

| Finding | Status | Evidence |
|---|---|---|
| H1 first open overwrites | **Fixed** (see findings 4 and 6) | With no mark, the link goes into `sync_wait` (`fg.rs:1950-1955`). `verify` (`sync_link.rs:206-221`) compares the server's text (with pending), the disk and the editor. A mismatch pauses, and `tick` never writes before `verified`. served.rs checks this. Gap: the link starts, and sends any kept ops, before `verify`. |
| H2(a) pending not persisted | **Partly fixed** | Written each poll (`fg.rs:2078`) and restored on open (`sync_link.rs:111-116`). But every restore goes through the base rewrite (finding 1, High). Ids can collide (finding 2). There is no write on Drop and failures are silent (findings 2 and 7). |
| H2(b) refused Hello does not pause | **Fixed** (see findings 3 and 4) | Closing{retry:false} becomes Stopped. forge-remote stops (`lib.rs:491-493`) and the app pauses (`fg.rs:2068`, `:2008`). Gaps: retry closes have no backoff, PROTOCOL skew in either direction loops, and the pause has no recovery path. |
| H2(c) save seen as hand edit | **Fixed** | `save` calls `SyncFile::wrote` (`fg.rs:2394-2397`, `sync_link.rs:225-230`). Not while paused (finding 4). |
| H3 pause never shown | **Fixed** | `sync_paused` (`fg.rs:1809`) is copied to `FgPanel` every frame (`gui.rs:2388-2392`) and drawn by `sync_banner` (`shell/banner.rs:93-128`, called at `shell/mod.rs:866`). Notes are taken whether or not a link is live. Nothing is shown while `sync_wait` waits (finding 6). |
| M1 Save after pause overwrites hand edit | **Fixed** | `pause_for_hand_edit` loads the kept file. A pause on open never loads the server's text. Side effect: finding 5. |
| M2 Ack lost in a Resync leaves op pending | **Fixed, through a fix that is still wrong** | Every Welcome resends (`client.rs:286-295`). This brings re-review findings 1 and 2, both still reproduced. |
| M3 false build-hash reports | **Not fixed** | The hash block in `fg.rs` was only reformatted. |
| M4 hosted undo before Ack | **Not fixed** | `client.rs:230-253` is unchanged. |
| M5 `apply_all` not atomic | **Not fixed** | |
| M6 save between check and rename | **Partly fixed** | Finding 8. |
| L: web reconnects for ever (`websync.rs`) | **Not fixed** | client-web is unchanged (5fc726bb, PROTOCOL 1). When it takes 12082dfb it must handle `Stopped`, and it becomes a v1 client against a v2 server until then (finding 3). |
| L: `sync_token` not scoped to `?sync=` | **Not fixed** | `app.js:147` is unchanged. |
| L: UI-thread socket write under lock, 10 s timeout | **Not fixed** | `forge-remote lib.rs:375-380` and `:453`. |
| L: BadBase after compaction plus restart | **Superseded** | The client no longer sends a base below the floor, because it rewrites the base (finding 1). |

## The author's server-side claims, checked

- **The client sends no op before Welcome:** true. `send_op` is gated by `welcomed`, and
  `reconnect` clears both it and the outbox.
- **Dedup runs before the base check:** true (`sequencer.rs:314-327`, before `order`).
- **Error is only a message, and Closing precedes every close:** true for the server's own close
  paths (`close`, `stop`, `fail`). The one place `Out::Close` is sent without a message is
  `out.close(None)` at the normal end of a connection.
- **A join racing a failed open waits for Ended:** correct. `TrySendError::Disconnected` means the
  actor's `ended()` blocking send finished before its `rx` dropped, so the Ended is already queued
  behind this Join, and `drop_doc_conns` then reaches the newly inserted conn.
