# HOSTED-DESIGN: handoff (kept current)

Last updated 2026-11-25. Role: design authority for hosted, multi-user forgeCAD. Design only —
code lives with HOSTED-BUILD (session_01WJtaww9gw31F5Az3YPjoqW). Coordinator:
session_01WJtawwACx4hZuk77VMadNk.

## Where things are

| what | where | state |
|---|---|---|
| the design | `research/hosted/DESIGN.md` (§0 one-page answer; §17 phase 2 being written) | phase 1 built; phase 2 drafted |
| numbers | `research/hosted/MEASUREMENTS.md`, `research/hosted/data/` | phase-2 Postgres/HA runs pending a quiet machine |
| cited sources | `research/hosted/PRIOR-ART.md`, `PRIOR-ART-PHASE2.md` | fetched 2026-11-25 |
| reviews | `research/hosted/reviews/N1-N9.md`, `clients.md` | N1-N9 fixes at 5c8eb061/470e7e90 under re-review |
| users/orgs/auth plan | `research/hosted/USERS-ORGS-AUTH.md` | in progress (coordinator's ask of 05:05) |
| bench scripts | `research/hosted/bench/` (`phase2_pg.py`, `phase2_ha.py`, `server_load.rs`, older ones) | smoke-tested |

## Open items

1. **Land** `perf/asm-overlap-walks` @ c72c6125 (second-agent review SAFE, notes applied) and
   `fix/asm-shape-first` @ 1af8e6da (the false GREEN of 3a73019c retracted). Pre-verifies queued
   on the fixed tool (`/tmp/pv` ≥ ccd7ef7d); send each line to TRAIN-CONDUCTOR
   (session_01WJtawwrcJMe9MW7GeYQtHw). `fix/query-interference-refusal` @ a025df8c: its run
   started on the old tool — check its record with the coordinator's script, rerun if needed.
   The asm work then ends (ASM-CHECK cancelled; `research/asm-check/HANDOFF.md` stays as notes).
2. **Re-review** HOSTED-BUILD's N1-N9 fixes (hosted/server-hardening 5c8eb061 + 470e7e90).
3. **Phase-2 measurements**: `phase2_pg.py`, `phase2_ha.py` on a quiet machine (the chain
   `/var/tmp/chain.sh` runs them after the pre-verifies); fill §17's ⟨…⟩ markers; write
   MEASUREMENTS §p2.
4. **Server memory**: RSS grows ~100 KB per committed op under `server_load.rs` (not glibc
   arenas: same with MALLOC_ARENA_MAX=2) and 3.1 MB per connection. Heap profile running
   (heaptrack); send the culprit to HOSTED-BUILD.
5. **USERS-ORGS-AUTH.md** (design only), then send the path to the coordinator and stop.

## Decisions a successor must not undo without new evidence

- Server-ordered typed-op log with rebase + re-validation; Rename transform; LWW SetParam (§4).
- Clients rebuild locally; compute workers for heavy results keyed by content hash + kernel id
  (§3). Determinism is enforced (§13).
- Postgres for the op log, accounts and permissions; ops never deleted; one writer per document
  by the `head_seq` fence in the append; synchronous replication only (an acked op that vanishes
  frees its seq for a different op: silent divergence) (§5, §17).
- The client resends every pending op on every Welcome (both reviews arrived at it; §4.2).
- LOCAL projects never get slower: no hosted code on the local edit path (§7, B5).
