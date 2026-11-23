# bool-speed: open, parked and rejected items (2026-11-23)

The record of the speed session's items that are not done, for `speedups.md` (on the
bool-research branch) to link. Counts are the exact gate (callgrind on the x86-64-v2 build,
`tools/speed/exact.sh` conditions), switch off unless said; wall-clock is best of N interleaved
on a 4-vCPU VM.

## Landed for round 7 (for reference)

Batch on r7b (`dad4398..4f18b18`, reachable from this branch at `b9dba9e^2`): S1, S2, S3,
line_in_range in place, S1b, planar_parity without lists, by_ends filtered, imprint's lists
reused, crossings without a list and split_face's cycles in one list, point_in_face_past without
a boxed iterator, fuzz seed 29's lineage (option a), the result cache keyed by the kernel's
features, `FORGECAD_CACHE=off` in gate.sh. Against r7b: all 97 lines lower, −292.5M summed;
coplanar seq −16.56%, patterns seq −9.84%. W1/W3 were already in r7b.

The rule that came out of it: on this path, small allocations in hot loops cost far more than
their own instructions (glibc's free-list work between the boolean's larger blocks). Imprint's
31k small blocks cost 8.5M on coplanar seq. Look at allocation churn first, then algorithms.

## Rejected, with numbers

**`CurveSurface::Hits` as an inline list** (`HitList { None, One, Many(Vec) }`, safe code, all 65
uses). It saves the one-hit list of a line through a plane (18k blocks on coplanar seq), about
−1.2M there. But the enum's branch on every slice access, its size in every `CurveSurface` move,
and per-item pushes cost more on curved parts. Against 4f18b18: net +3.1M (61 lines slower,
mixed seq +1.24M); with Vec-like sizing in collect and extend, net +2.4M (60 slower); with the
curved intersectors building it directly, net +16.3M (81 slower). Kept as a stash in the
`forgeRDY` scratch worktree, not committed.

A per-thread pool of hit lists, handed back by trim: no gain (the lists are mostly made
elsewhere). Using `bands.near` in `point_in_face_past`'s `past` check: no gain (rarely reached).

## Open, in order

1. **Incremental FaceInfo (today's path), about −10M on coplanar seq.** `point_in_face_past` is
   26M inclusive on coplanar seq; 10M of that rebuilds FaceInfo (edges, boxes, Bands, Simple) for
   faces each boolean changes (the plate's big face, every pad). The invariant is the Lean
   coordinator's CacheOK (every set `Face::query` equals `FaceInfo::fresh`, the write sites and
   readers listed, GONE entries skipped by every reader, row order independent of insertion
   where a reader depends on it), with debug builds recomputing and comparing after every apply.
   Agree the statement before building.
2. **Edge-name parity, ported into the decide-once write** (A8 ships as the fallback, so its
   names must be today's). Measured with `names` (bd62ca6): geometry identical everywhere; names
   differ in 36 of 334 part/path blocks: edges 1,849 of 284,845 (0.65%), vertices 1,016 of
   205,338, faces 237 of 107,036, most on coplanar seq. Across the repository's 303 .fg files,
   5 builds differ (the seed 29 repros, thermoform's corner_fill). The port: trim's cut name
   (`root.cut.with_set(face wholes)`), each piece by its end vertices (`piece_name`), vertex
   names as the mix of the marks meeting there with ties by place in the surface frame
   (`rank_names`), and `flush_name` for edges on a removed face. Size M–L.
3. **Stitch stage 4** (abdf086: corners, runs, edges in forge-bool-core) goes with item 2, with
   decide-once Rust's fixes taken (`crate::inc::Csr`, no `vec![x]`). Write phase +0.96M, switch on.
4. **forge-lang's async step (deferred, ranked for later).** The app and cli build each feature
   as the program is evaluated, and evaluation reads the stages built so far (selections,
   anchors, `instances`, asm), so W3's build-ahead cannot apply: feature i+1 does not exist while
   step i runs. Instead: `push(i)` would start step i's boolean on a worker and evaluation would go on
   to statement i+1 (building its tools), blocking only where it reads stage i. Expected win: on
   open and edit, up to the share of tool building and evaluation that overlaps the boolean
   chain (upto-like parts, where regen1 shows −17% at t4; app open about the same order).
   Design: `Stage` from `Rc` to `Arc`, every `ev.b.stages` read through one join point (a
   dozen sites in builtins.rs, asm.rs, eval.rs). Risks: lineage, names or diagnostics picking
   up an order from completion; errors must surface at the step that met them; the stage cache
   (`Kept`, the memo) keyed by finished stages only. Size L.
5. **S4′ (the per-stretch flood in trim/imprint), last or not at all.** Realistic ceiling −2 to
   −5M: the expensive `point_in_face_past` calls are stretches along an edge, which its five
   conditions exclude. Its proof statement (the Jordan single-crossing flip) goes to the Lean
   coordinator before any merge.
6. **forge-lang fuzz `random_programs` with the switch on** (delete_faces built 0 of 4): a blocker
   for shipping the fallback; it failed before the lineage change, with the same counts.
7. **C4 in its inc form** (split computed once for `witnesses` and `core`): allowed by the Lean
   coordinator, low priority (A8 is the fallback). C1–C3 (v1 covector core): not allowed; v1
   is an oracle only.
