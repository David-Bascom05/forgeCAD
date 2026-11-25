# Review: hosted/client-native @ 44c5c6d7 and hosted/client-web @ 5fc726bb

By HOSTED-DESIGN, 2026-11-25. A second agent read both diffs end to end; I verified the three
high findings and M2 myself in the code, and corrected one detail (the reconnect backoff). No
build was needed for any of them. Findings are against DESIGN.md §14 (one client library) and
§15 (LOCAL/SERVED, hand edits, detect-and-pause).

## High

**H1. The first open of a served part can delete the file's own text.**
`sync_link.rs:120–148`: with no `.forgecad/sync/<doc>.hash` mark yet (every first open), the
local file's hash is written as the mark and the file is taken as "synced". About a second later
`tick` writes the server's text over it. Any local edit made before the project was served is
gone from disk, from the editor and from undo (undo is cleared on open). §15.2 allows attaching
only when the texts match. Fix: on a first open with no mark, compare the file's text with the
server's; equal, write the mark; different, pause with the hand-edit message and leave the file
alone.

**H2. Unacked edits exist only in memory, and a refused Hello does not pause.**
(a) Nothing persists pending ops; §14 asks for `.forgecad/sync/pending`. Close the app with an
op unacked (offline, or a slow server) and the edit is gone. (b) `fg.rs:1965`:
`Event::Closed(why)` only sets `sync_note`, so a server that refuses the Hello (wrong kernel id,
bad token) leaves the app editing into a pending list forever while forge-remote reconnects for
good (backoff 50 ms doubling to 5 s — not every 50 ms, as first reported). Fix: pause sync on
Closed, say so, and persist pending ops. (c) `fg.rs:2279`: Ctrl+S, and the agent's save before
each turn, writes the view to disk; the next `tick` reads that as a hand edit and pauses. A save
of a served document should write through the link (or update the mark), not behind it.

**H3. "Syncing paused" is never shown.** `gui.rs:2359`: sync notes are shown only while
`s.sync.is_some()`, and `pause_sync` sets `sync` to None before the note is drawn. So the one
message the user must see is the one they cannot, and later edits stay local silently. Fix: keep
the pause reason in its own field, shown whether or not a link is live.

## Medium

**M1.** `fg.rs:1939`: after a pause the session still holds the server's text, so the next Save
overwrites the hand edit the pause said it kept.

**M2. An Ack dropped during a Resync leaves an op pending for ever.** `client.rs:213–236`: an
Ack whose `seq != self.seq + 1` returns `diverged(seq)`, which asks for a snapshot; the pending
entry stays. `Welcome { snap: true }` then replaces `confirmed`, clears `recent` and replays,
but never resends or clears pending. That op is now committed on the server and pending on the
client: it is applied twice in the view, `pending() > 0` for ever (so `settled` is never true
and the file is never written again), and hash reports stop. Fix: resend pending ops after the
snapshot. This is the same fix as my H1 on the server side (research/hosted/reviews/N1-N9.md):
**the client must resend pending ops on every Welcome**; dedup by (client, op id) makes it
idempotent. Two independent reviews landing on it is a good sign it belongs in the protocol, so
§4.2 should say it.

**M3.** `fg.rs:2037`: build hashes are reported against the server's seq even when the build was
rolled back with `until`, when the text was already stale, and for assemblies whose sibling
parts are stale on local disk. Each gives a false DIVERGENCE. Only report a hash for a build of
exactly the text at that seq.

**M4.** Hosted undo before an Ack, or offline, undoes the previous acknowledged op rather than
the user's latest edit.

**M5.** `apply_all` is checked as a bundle locally but sent as separate ops the server checks one
by one, so the server can commit part of a bundle. Either send it as one op or make the protocol
carry an atomic group.

**M6.** An editor save landing between `tick`'s check and its rename is overwritten with no
trace (read, then write, with no re-check).

## Low

- A closed web document reconnects for ever (`websync.rs:159`).
- A stored `sync_token` is sent to a new `?sync=` address (`app.js:147`): scope the token to the
  address it was given for.
- The UI thread writes to the socket under the session lock with a 10 s write timeout: a stalled
  link can block a frame.
- After compaction plus a server restart, an op that did land is reported refused with BadBase
  (the pre-compaction base is gone). Answer such a base with a snapshot, not a refusal.

## Checked and right

- **LOCAL projects**: the edit path gains one `Option` test, with no network, no thread and no
  per-edit work. (The callgrind B5 gate was not run on these branches; it should be, since
  forge-render is in it.)
- The remote comes only from the user's `remotes.toml`; the token never reaches a log or the
  page's URL.
- Origin and Host checks hold on the web client.
- Reconnect resends are deduplicated by (client, op id); acks cannot arrive out of order on one
  connection.
- `room.rs`'s seq-0 hash check, and the `render: true` assembly hash rule, match §13.
