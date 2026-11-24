# CAM edit and cold-start timing (2026-11-24)

How long a machining document takes to solve: cold, and after a typical edit. Measured by
CAM-CORE on merge-train/4b @f59e678e merged with cam/parity (f462f977; it has zlevel, the mould
core and CAM-TURN's hub).

## Method

- A harness (`examples/edit_timing.rs`, not committed) runs documents in order in one process,
  as the app does: forge-render's cam shell calls `forge_cam_lang::run` on the whole text at
  every edit.
- Each line in the tables is one full run.
  - **Cold**: the first run in a fresh process.
  - **Rerun**: the same text again.
  - **Edit columns**: each edited text, run after the previous runs.
- The split:
  - **certify**: time in `certify_posted`, the posted path's checks;
  - **post**: `program()` less its certification;
  - **generate**: everything else, which includes the part's build and lowering, each op's plan,
    the in-process stock and the G4 leftover.
- **Native** is a release build on x86-64-v3, single thread.
- **wasm** is wasm32-wasip1 (+simd128) under Node 22's V8: the engine and the single thread of a
  browser page.
- The edits:
  - **stepover**: one op's stepover;
  - **part dim**: one shared of the part file, which the document re-reads;
  - **tool**: one tool's stickout, or a different insert for the hub;
  - **add op**: one op before the program.

## Totals, seconds (cold / rerun / stepover / part dim / tool / add op)

| document | native | wasm |
|---|---|---|
| flange (5 ops) | 0.21 / 0.19 / 0.20 / 0.19 / 0.22 / 0.33 | 0.34 / 0.23 / 0.24 / 0.24 / 0.23 / 0.36 |
| bracket (5 ops, 3 setups) | 0.37 / 0.21 / 0.21 / 0.35 / 0.21 / **7.09** | 0.52 / 0.28 / 0.28 / 0.42 / 0.27 / **7.78** |
| plate_break (4 ops) | 0.17 / 0.12 / 0.14 / 0.15 / 0.12 / 0.14 | 0.30 / 0.16 / 0.18 / 0.18 / 0.16 / 0.17 |
| hub (turning, 14 ops) | 0.57 / 0.46 / 0.54 / 0.53 / 0.60 / 0.48 | 0.75 / 0.60 / 0.65 / 0.64 / 0.70 / 0.60 |
| mold core (3D, 5 ops) | 11.8 / 11.2 / 8.9 / 10.5 / 11.2 / 11.7 | 30.5 / 29.6 / 23.4 / 28.3 / 29.5 / 31.5 |
| manifold (15 ops, 2 setups) | 18.5 / 13.6 / 13.5 / 13.7 / 13.6 / 13.5 | 22.5 / 17.2 / 17.2 / 17.3 / 17.2 / 17.2 |

The bracket's added op is an `edge_break`, which takes 6.9 s of certification by itself.

## Split, cold run, seconds (generate / certify / post)

| document | native | wasm |
|---|---|---|
| flange | 0.193 / 0.017 / 0.002 | 0.298 / 0.033 / 0.006 |
| bracket | 0.342 / 0.022 / 0.003 | 0.448 / 0.063 / 0.009 |
| plate_break | 0.158 / 0.013 / 0.001 | 0.262 / 0.031 / 0.005 |
| hub (turning) | 0.57 (forge-turn-lang is not split) | 0.75 |
| mold core | 9.43 / 2.40 / 0.02 | 23.05 / 7.44 / 0.04 |
| manifold | 1.75 / 16.75 / 0.02 | 3.80 / 18.64 / 0.03 |

- On the 2.5D parts, "generate" is mostly the part build and the G4 leftover; each op's own plan
  is ~5–50 ms (FORGE_CAM_TIME).
- The mould core's generate is the drop-cutter surfacing.
- The manifold's certify is branch-and-bound cells: shal 59k, rough 16k, crn 17k, brk 10k.

## What is cached today

- **Per op: nothing.** Every run re-plans every op, rebuilds the in-process stock and
  re-certifies every op. An edit to op 5 regenerates and re-certifies ops 1–4. A part edit
  redoes everything. The app reruns the whole text on each edit.
- **Process-wide memos, by content** (forge-cam-geom). They are why a rerun is 20–40% faster
  than cold:
  - offsets and booleans, by a hash of their input regions and distance (MADE, 512 entries);
  - validated exact regions (VALID, 4096 entries);
  - each region's y-cuts.
- Nothing is keyed per op or per (move, face) pair. Certification verdicts are never reused.

## What would make edits instant

All of these keep the guarantees, because a reused verdict is the same function of the same
inputs.

1. **Per-op cache with keyed invalidation** (forge-cam-lang). Key an op's plan on its resolved
   arguments, its tool's record and shape, its setup and the part's face content hash.
   - Key its certification on the posted path's text hash, the part faces' hashes, the stock
     before it, the tool shape and the fixture.
   - An edit to op k re-plans op k only.
   - Later ops re-certify only their stock checks (rapid, holder) and only if op k's voids
     changed. Their gouge checks against the part are untouched.
   - Stock only ever loses material. So a later op's Clear against the stock stays valid when
     the new voids contain the old ones; only checks that were not Clear need re-running.
2. **Pair-level verdict memo**, keyed by (move, face content, body, limit). A part edit changes a
   few faces; every pair on an unchanged face hits.
   - The manifold's 12–17 s would drop to the changed faces' share. Most edits touch one
     feature.
   - Sound, since a verdict is a pure function of those inputs. The cache must key on exact
     bits, not rounded values.
3. **Provisional path at once, certification in the background.** 2.5D generation is
   0.1–0.4 s and the turning ops 0.5 s, so the path can be shown immediately and marked
   "certifying".
   - NC export stays gated on "certified".
   - In the browser, a Web Worker runs certification. On a 3D document generation itself is the
     long pole (mould core: 23 s in wasm), so surfacing needs (4) as well.
4. **Parallel certification natively** (rayon over pairs, ops and setups) and in the browser
   (Web Workers with SharedArrayBuffer; needs COOP/COEP headers). Pairs are independent. The
   manifold has ~4k pairs per big op, so 8 cores give near 8×.
5. **The sinks themselves:**
   - B&B cells on curved faces: closed forms from CAM-3D-FINISH and CURVED-SITES, and CAM-PERF's
     work;
   - zero-gap walls: plans already avoid them where an op of its own exists;
   - drop-cutter surfacing in the mould core: CAM-3AXIS;
   - edge_break's certification on the bracket: 6.9 s for one op.

Proposed order: (3), then (1), then (2), then (4). (3) makes every 2.5D edit feel instant today.
(1) and (2) make re-certification proportional to the change. (4) multiplies what is left.
