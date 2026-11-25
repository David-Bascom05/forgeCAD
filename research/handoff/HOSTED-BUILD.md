# HOSTED-BUILD handoff

The state of the hosted build (research/hosted/DESIGN.md, on design/hosted), kept current by
HOSTED-BUILD. Updated 2026-11-25 (night).

## Branches

All four hosted branches have origin/main (Train 10, 247a2694) merged in. TRAIN-CONDUCTOR
accepted server-hardening bc12d3e0, client-native 84b3ea49 and client-web 2fd9bea8 for T12;
the heads below are those plus the main merge (and, on client-web, the --sync hardening).

| Branch | What | State |
|---|---|---|
| `hosted/server` | forge-sync (sans-I/O protocol), forge-ws (RFC 6455, no new crates), forge-server (actors, hub, file log, git versions), forge-remote (native client, remotes.toml trust) | superseded by server-hardening, which contains it |
| `hosted/server-hardening` | N1–N9, the reviews' fixes, load/soak/fault/disk-full tests | 576d8e34. Pre-verify GREEN at bc12d3e0; later commits are tests, the handoff, and the main merge |
| `hosted/client-native` | the desktop app as a client (forge-render: `sync_link`, `fg::Session`) | f8c9038c. UX-LEAD OK. Main merge conflicts in forge-render (both sides' fields kept) |
| `hosted/client-web` | the browser app as a client (web-sys WebSocket, `?sync=`, `forgecad web --sync`) | 5aa6912b. Security review SAFE; its two hardening fixes are in (afc58f54: wss:// unless loopback, strict origin parse). Pre-verify pending |
| `hosted/release-sketches` | forge-lang's `release_sketches()` / `ReleaseOnDrop`: the kept-sketch leak's root fix | a7668e22, GREEN-EXCEPT-RENDER, gate ok |
| `hosted/lean-rebase` | the Lean model of rebase | LANG-PROOFS' (now inactive) |
| `ci/determinism` | cross-platform bit identity: forge-det + compare.py; folded-vs-runtime `--consts`; the ipow native and wasm runners | d6083ef8. f6f92445 GREEN |

## Protocol (PROTOCOL = 2)

- Every Welcome makes the client resend every pending op, in order. After a snapshot Welcome
  they are sent from the snapshot's seq. The server dedups by (client, op id): per client, a
  high-water mark of committed ids plus the refused ids. A retry gets its Ack again (from the
  log) or AlreadyCommitted (from before it). DESIGN §4.2.
- `Down::Error` is a message only; the connection stays open. `Down::Closing { text, retry }`
  precedes every close. With `retry: false` (a bad token, another kernel or protocol, a
  quarantined or unopenable document, a malformed message), the client stops reconnecting and
  the app pauses syncing.
- A client can be saved and restored (`Client::saved`/`restore`; forge-remote
  `Session::resume`/`saved`). The app keeps it in `.forgecad/sync/<doc>.pending`, so unacked
  ops survive a close.

## Server

- Store: an append-only file log per document. A snapshot is a record appended to it, and
  nothing is deleted (op history is kept; retention is a separate policy, not built).
- A document's actor closes with a flag plus a Close message, and drains its inbox first. Ops
  sent while it closes wait with their joins and go to the new actor in order.
- Limits: validation budget (abandoned runs per document and server-wide), hash-build budget
  (`Config.build_budget`), outbox bytes, per-IP and pre-auth caps, a handshake deadline,
  rate tokens.
- Capacity (HOSTED-DESIGN's bench, 4 vCPU): about 3.2 MB per open document plus connection,
  about 300 documents per GB; about 1.1 ms of server CPU per committed op.

## Load, soak, faults (hosted/server-hardening)

- `tests/faults.rs`: the real binary under SIGKILL mid-op and restart, and sessions closed and
  resumed from their saved client. A smoke run by default (~1.3 s); `--ignored` soak with
  FORGE_SOAK_SECS. 60 s: 59,888 edits, 19 kills, 85 resumes; converged, nothing lost, nothing
  committed twice.
- `tests/load.rs` (`--ignored`, release): steady edits plus connection churn, with /proc memory
  and CPU. 100 documents × 3 sessions, 120 s: 72,000 edits; RSS 35 MB joined, 110 MB at the end
  (680 B per op: the in-memory op log); 0.57 ms server CPU per op; ack p50 3.6 ms, p99 23.5 ms.
  `FORGE_LOAD_WRAP="valgrind --tool=massif ..."` profiles the server's heap.
- Fixed from it: forge-lang's kept sketches (`forge_keep`) are released on every server thread
  and on forge-remote's (`forge_sync::release_sketches`); they were 38 KB per op for good. The
  per-connection outbox no longer pre-allocates 3 MB. Validation runs on one worker per document.

## App

- LOCAL is unchanged: one `Option` test on the edit path. The gate binaries were
  byte-identical to `main` (re-check with the callgrind B5 gate on the client branches).
- SERVED: a first open (no `.forgecad/sync/<doc>.hash` mark) attaches only once the server's
  text equals the file's and the editor's. Otherwise it pauses and the file is never written
  over. A hand edit pauses, and the editor then shows the kept file. Save updates the mark.
  "Syncing paused" is a banner over the view for as long as it holds.

## UX

UX-LEAD OK'd hosted/client-native 1bc1e103 and hosted/client-web deafa8fb (the sync notes'
words, the Syncing paused banner, the shots in research/hosted/shots/). Open, P3: the native
pause reason should read "bracket.fg changed on disk since it last synced, so your edits stay
here and aren't shared. <the action>", with the path on hover (sync_link.rs SyncFile::open and
tick).

## Next

1. HOSTED-DESIGN re-review of server-hardening. Pre-verify (pv ≥ 13d78f06) for each branch,
   sent to TRAIN-CONDUCTOR (session_01WJtawwrcJMe9MW7GeYQtHw).
2. Clients review mediums: M3 (report hashes only for builds of exactly the seq's text), M4
   (hosted undo before an Ack), M5 (apply_all as one atomic group), M6 (done: a re-check right
   before the rename). Web lows (a stopped link stops; the token is scoped to its address).
3. Load, soak and fault injection: done (above), and the disk-full test (tests/diskfull.rs:
   refused as not saved, the log stays whole, a close while full loses nothing acked). The
   ipow wasm runner is in ci/determinism. A 50-minute soak (40 documents, churn) is the
   check on the ~4 KB/op residual: the in-memory log, capped at 1 MiB per document.
4. Not in scope: users, orgs and auth (HOSTED-DESIGN is writing that plan).
