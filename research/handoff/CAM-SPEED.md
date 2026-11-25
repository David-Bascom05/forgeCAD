# CAM-SPEED handoff (2026-11-25)

Goal (David): a cold toolpath generation of every example CAM document in ≤ 1 s; an edit that
affects one op in ≤ 100 ms. Baseline and method: research/cam-speed/BASELINE.md (regenerate
with research/cam-speed/baseline.py; per-doc, per-op, cold and edit times, phases, CPU/wall).

## Branches

| branch | sha | state |
|---|---|---|
| cam/speed | 3a3fe6d0 | PRE-VERIFY GREEN-EXCEPT-RENDER (sent to TRAIN-CONDUCTOR; render via VERIFY-RENDER); in train 8 |
| cam/speed-bull | this commit (on cam/speed) | bull-nose rest fix; held for the coordinator's independent review, then PRE-VERIFY |
| cam/speed-wip | (scratch) | old WIP, superseded; may be deleted |

### cam/speed (byte-identical outputs on all 47 docs and every edit run)
- Opt-in phase profiler (forge-cam-lang/src/prof.rs, FORGE_CAM_PROF), cam_bench, baseline.py.
- hop_links memo (HOPPED; carried map keyed by the call number that made it) and adaptive's
  memo keyed on the cutter's record: rerun/tool-edit wins (boss_adaptive rerun 0.21 → 0.006 s,
  deep_islands tool edit 10–12 → 0.2 s). Cold cost +0.06 % Ir (key formatting), accepted.
- Built-in tooling library read once, in parallel, never copied (Library::builtin_shared):
  every fresh process ~0.09 s faster (block_m6 0.150 → 0.067 s).
- Cam::prefetch: a document's 3D generations made side by side into their memos before the
  run (mold_core 5.5 → 4.5 s, mold_core_finish 10.2 → 8.0 s cold).
- Tests: finish3d_memo_tests 144 → 63 s (cases side by side, corner fixture, same cases).

### cam/speed-bull (outputs change for bull-nose cutters only: rewrite protocol)
- contact.rs disc capsule: the closed form for bull-nose cutters (FALLBACK's floor fix
  eb5f279e included; EXACT-CORE reviewed: SAFE).
- drop.rs bull_section: a bull column's section past a horizontal cylinder's end (proof in its
  doc comment; checked by CAM-3D-FINISH and EXACT-CORE; two-sum exactness fix 14df2eb4).
- forge-cam-geom feature `old-closed-form` restores the old path (keep one train).
- Measured (bull-10-r1 rest after a flat-10 zlevel, mold_core corner): 64.6 → 11.0 s, both
  certified (11.4 s after FALLBACK's floor fix), rest 1785 → 425 moves, cut length +0.8 %. The 47 docs and the mould fuzz (84 runs):
  identical apart from wall times. Pending: the coordinator's second review, FALLBACK on
  drop.rs, a speed-ledger row (research/cam/speed-ledger.md) in the landing commit.

## Unfinished (not started or not landed)

1. Prefetch on an unchanged rerun costs ~7 ms (28 → 35 ms on the mould). The coordinator wants
   ≤ 28 ms. Draft (not committed): in Cam::prefetch, a static LAST holding the document's text,
   dir and each part file's text from the last run that prefetched; equal → return. Correct by
   construction (a skipped prefetch only means the run makes them in order).
2. Rest + prefetch: CAM-3D-FINISH's cam/rest-air makes rest's finish3d key hold earlier cuts,
   so a prefetched rest is wasted work. Agreed: whichever of cam/speed / cam/rest-air lands
   second adds `kind != "rest"` to the prefetch while merging main.
3. Adaptive (deep_islands 10–12 s, deep_boss 2.4–3 s, one core): 43 % of the time is
   Field::retract_link → top0, which scans a full disk of map0 for every sample along a retract;
   the disks overlap almost entirely. Agreed with CAM-CORE: skip cells already visited (index as
   HeightMap::top computes it), byte-identical, with a differential test, AFTER CAM-CORE's
   pass-start branch lands (and on TQ's cam/park-descent if landed). Est. 30–40× fewer reads.
4. Mould generators: the drape's gap search on level planes (≈160 cells a chord) and
   budget-exhausted chords (≈3 s on mold_core). A tighter closed form for trimmed level planes
   would change verdicts → rewrite protocol. EXACT-CORE is making the interval arithmetic itself
   faster (exact-core/ival-fast, −29 %); the two stack.
5. Bull rest's remaining 2.4k Unknowns: the torus brushing past a round's end (a ≈ 4.5); the
   stadium section is loose there.
6. Certification (David: make it fast or prove parts redundant, never show before it):
   not started. ~1 s CPU on the mould (check::Move::at, Face::holds).
7. Slow tests (coordinator's list): only finish3d_memo and prefetch_tests done. The 57 s of
   finish3d_memo left is the bull case, which cam/speed-bull brings to ~11 s.
8. wasm numbers: cam/speed measured (block_m6 0.187 s cold, flange 0.408, bracket 0.583 under
   Node); the merge-train/7 wasm baseline was built (target/wasm-t7) but not timed.

## Risks
- HOPPED identifies the carried height map by a never-reused call number: correct only while a
  numbered map is never mutated (true today; pinned by hop_memo_tests).
- Cam::prefetch assumes no 3D generator's memo key holds an earlier op's result (true today
  except rest after cam/rest-air: item 2).
- adaptive/2's key holds only tool.tool: pinned by adaptive_reads_no_other_part_of_the_tool;
  CAM-CORE may make it structural (adaptive() taking &Tool) after pass-start.
- The bull closed forms are new proof-carrying code: keep `old-closed-form` for a train.
