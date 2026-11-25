# HOSTED-DESIGN: handoff (kept current)

Last updated 2026-11-25 (evening). Role: design authority for hosted, multi-user forgeCAD. Design only —
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

## Open items

1. **Landed?** Three branches have GREEN lines with TRAIN-CONDUCTOR (session_01WJtawwrcJMe9MW7GeYQtHw):
   fix/asm-shape-first 0ebc123f, perf/asm-overlap-walks 62d85634 (rides with the speed branches
   after gate-memfns), fix/query-interference-refusal fbe2959b. Watch for their train; nothing else
   to do on asm (ASM-CHECK cancelled; `research/asm-check/HANDOFF.md` is notes only).
2. **HOSTED-BUILD (session_01WJtaww9gw31F5Az3YPjoqW), until the server runs full bore**:
   - server-hardening 5df9cf74: the wrong-target fix, re-review lows and both memory fixes are in;
     the load test confirmed the memory (DESIGN §17.7). Still open there: ~4.1 KB/op RSS growth
     to explain with a longer soak; their fault-injection tests.
   - client-native: clients-rereview #3 (two instances on one .pending), #5 (re-auth Closing kind,
     saves while paused), the lows; M3/M4/M5 from the first client review. Re-review when they land
     (`reviews/clients-rereview.md` is the checklist).
   - a pub `forge_lang::release_sketches()` (HOSTED-BUILD's branch) so hosts stop copying it.
3. When HOSTED-BUILD's soak and fault tests pass: update this file and send WOUND DOWN to the
   coordinator (session_01WJtawwACx4hZuk77VMadNk). No auth implementation.

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
