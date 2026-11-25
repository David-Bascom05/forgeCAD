# HOSTED-DESIGN: handoff (kept current)

Last updated 2026-11-25, at wind-down. Role: design authority for hosted, multi-user forgeCAD. Design only —
code lives with HOSTED-BUILD (session_01WJtaww9gw31F5Az3YPjoqW). Coordinator:
session_01WJtawwACx4hZuk77VMadNk.

## Where things are

| what | where | state |
|---|---|---|
| the design | `research/hosted/DESIGN.md` (§0 one-page answer; §17 phase 2) | phase 1 built; phase 2 decided with measurements |
| numbers | `research/hosted/MEASUREMENTS.md` (§p2 for phase 2), `research/hosted/data/` | done |
| cited sources | `research/hosted/PRIOR-ART.md`, `PRIOR-ART-PHASE2.md` | fetched 2026-11-25 |
| reviews | `research/hosted/reviews/` (N1-N9, its re-review + repro, clients, its re-review + repros) | second round answered at 5df9cf74; client items pending |
| users/orgs/auth plan | `research/hosted/USERS-ORGS-AUTH.md` (sources `PRIOR-ART-AUTH.md`) | written 2026-11-25; design only |
| bench scripts | `research/hosted/bench/` (`phase2_pg.py`, `phase2_ha.py`, `server_load.rs`, `runleak.rs`, older ones) | run; see MEASUREMENTS §p2 |

## State at wind-down (2026-11-25, WOUND DOWN)

HOSTED-DESIGN is wound down. Nothing is open on its side.

**Landing.** Three branches have accepted GREEN pre-verify lines with TRAIN-CONDUCTOR. Watch for
them in a train; they need nothing more from this role.
- fix/asm-shape-first 0ebc123f
- perf/asm-overlap-walks 62d85634, which rides with the speed branches after gate-memfns, with its
  band re-measured on the new gate
- fix/query-interference-refusal fbe2959b

ASM-CHECK was cancelled. `research/asm-check/HANDOFF.md` is notes only (P6 and the query's missing
mesh stage, if anyone picks up forge-asm).

**HOSTED-BUILD's server runs full bore** (hosted/server-hardening → hosted/m5-groups 4cf2da68,
hosted/m5-clients):
- Memory: the two bugs found here are fixed. These were unreleased `forge_keep` sketches and a
  65,536-slot outbox. 240 docs went from 792 MB to 56 MB. In the 50-min soak (359,635 edits, 0
  failures) memory is flat once each document's log reaches its cap: ~1.2 MB live, ~5 MB RSS
  high-water per hot document.
- Faults: `tests/faults.rs` (SIGKILL mid-op: 59,888 edits, 19 kills, every acked op present once,
  all clients converged), `tests/diskfull.rs` (append and compaction on a full disk: nothing acked
  lost, seqs without a gap) and `security.rs` (a connection dropped mid-frame commits nothing).
  Nothing uses wall time.
- Reviews: all highs from four review rounds are fixed and their repros now fail
  (`research/hosted/reviews/`). Still open on HOSTED-BUILD's list, none blocking: first client
  review M3 (hash reports only for builds of exactly the seq's text) and M4; clients-rereview #3
  (two instances, one .pending: a lock planned) and #5 (a re-auth Closing kind); the token scoped
  to its ?sync= address; the UI-thread socket write.

**Design deliverables** (design/hosted):
- `research/hosted/DESIGN.md`: §0–§16 phase 0/1; §4.2 with the resend/rebase and group rules; §17
  phase 2, decided with measurements.
- `research/hosted/USERS-ORGS-AUTH.md`: design only, no implementation (coordinator's rule).
- MEASUREMENTS.md, including §p2.
- PRIOR-ART*.md.
- bench/ and data/.

**For whoever picks hosted design up next:**
1. Phase 2 is §17.9's plan. Start with PgStore behind the existing `Store` trait. `phase2_pg.py`'s
   checks (fence, viewer refused, dedup index) become its tests.
2. Offline editing from a cold open needs the local text's base kept on disk with the text (§4.2,
   phase 4). Until then a served part is read-only until its server first answers.
3. Ask forge-lang's owner for a pub `forge_lang::release_sketches()`. HOSTED-BUILD has a branch
   for it, so check it landed. Every new host (e.g. a compute worker) must call it between builds.
4. Threaded wasm stays off in hosted browser clients until CAM-PERF finds the realloc crash's
   root cause (§8, §17.8).

## Decisions a successor must not undo without new evidence

- Server-ordered typed-op log with rebase + re-validation; Rename transform; LWW SetParam (§4).
- Clients rebuild locally; compute workers for heavy results keyed by content hash + kernel id
  (§3). Determinism is enforced (§13).
- Postgres for the op log, accounts and permissions; ops never deleted; one writer per document
  by the `head_seq` fence in the append; synchronous replication only (an acked op that vanishes
  frees its seq for a different op: silent divergence) (§5, §17).
- The client resends pending ops on its connection's first Welcome, always with the real base;
  never re-base an op to a snapshot (§4.2).
- LOCAL projects never get slower: no hosted code on the local edit path (§7, B5).
