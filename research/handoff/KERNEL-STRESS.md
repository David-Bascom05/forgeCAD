# KERNEL-STRESS handoff

KERNEL-STRESS ran the kernel over the Haas machine models (146 zips, 194 STEP files, about 11,600
solids), fixed what it found in its own area, and routed the rest to owners. The sweep is the
kernel's differential net: `tools/kernel-stress/compare.sh BASE NEW` runs it on two refs and
reports every difference, wrong answers first.

## Branches

| branch | what | state |
|---|---|---|
| kernel-stress | the harness (verify/forge-stress, tools/kernel-stress), the corpus and report (research/kernel-stress), the reader's fixes (forge-step), the section fixes (forge-agent), forge-asm rigid_copy | train 8 |
| op-budget | every top-level op has a work budget (`forge_geom::budget::OP`, MESH for a file body's mesh); par helpers share it; spends do not depend on call order or threads (`uncounted`, memo costs); `isolated` | train 8 |
| mass-cap | forge-topo mass: a cap round a NURBS apex weighs right (it was silently zero) | train 8 |
| face-box | forge-topo uv: a face on a surface closed one way gets a box as far as its loops reach | train 8 |
| op-cancel | on op-budget: cancelling a running build (a newer edit, a close, Cancel), progress, nothing of a stopped build kept (forge-lang, forge-render fg.rs) | after train 8; UI not done |

The heads and their PRE-VERIFY lines are in the last section.

## The tools

- `tools/kernel-stress/run.sh -o OUT -j N [-s haas|tooling] [ZIP...]`: the sweep. `-s tooling` reads
  the Haas Tooling workholding STEP files (branch haas-tooling-step, fetched shallow, never merged).
  `KS_NARY=1` runs booleans through n-ary on all threads, as the app does. `KS_DECISIONS=1` records
  the decision log's named sites.
- `tools/kernel-stress/compare.sh BASE NEW [-j N] [ZIP...]`: builds both refs with this checkout's
  harness and sweeps each once (cached). Writes a report from `diff.py`. Exits 1 on new wrong
  answers, hangs, or changed answers.
- `tools/kernel-stress/corpus.py make|run`: the corpus, the three smallest cases per failure class
  plus `research/kernel-stress/pinned.tsv` (cases named by hand, each with an owner). It is replayed
  from the zips.
- `tools/kernel-stress/extract.py STEP ENT[,ENT] out.step`: one solid (or a pair) out of a file,
  for a fixture or a reproducer.
- A full sweep takes about 3.5 h at -j3 on 4 cores. Imports alone take about 30 min.

## Open work

1. **Budget order independence: the sweep check.** op-budget's `uncounted` and memo-cost change
   (bb96cbcb) makes an op's spend independent of what ran before it and of threads. The tests are in
   forge-step tests/nary_budget.rs. The coordinator asked for a Haas sweep, old against new, with
   every refusal that moves explained.
   - Done: the import A/B, every STEP file read at d023bee7 (before) and bb96cbcb (after). 195
     files read the same, solid for solid. One differs: ds-40y #249171 (138 faces) now reads,
     where before it was refused as "its surfaces would take too long to measure". That refusal
     depended on what was read before it, the order-dependence IMPORT-SPEED found. It reads as
     740337 mm3, and check flags it (FaceWrongSide x8), so nothing wrong is passed as right.
   - Not done: the op-level A/B on the files whose ops spend near OP. Run
     `tools/kernel-stress/compare.sh d023bee7 op-budget` (about 7 h for both sides at -j3).
2. **op-cancel.**
   - Kernel and plumbing are done and tested.
   - Not done: the progress bar and Cancel button (they need UX-LEAD's OK with screenshots).
   - Not done: bringing a cut's unbudgeted work under the budget. On the app's path a slice of
     budgeted work is about 3 s of wall time, so a cancel can take that long; the target is well
     under 1 s.
   - Needs a rebase onto HOSTED-DESIGN's fix/asm-shape-first: put a `halt.stopped()` check after
     each forge_lang::run in the assembly block.
   - WEB-WORKER merges it for the browser build.
3. **Haas Tooling workholding files** (14 of 285 not read fully; `run.sh -s tooling` sweeps them all):
   - Fixed on kernel-stress, each with a synthetic test in tests/stress_haas.rs:
     - 05-0335: a whole torus with no edges, now an O-ring;
     - 05-0182 and 05-0959: directions written with noise where 0 was meant (parts of 7e-161),
       which laid faces out in a 1e-160 span. Such parts are now 0. 05-0182 was the one blocking
       WORKHOLDING's vise record.
   - Genuine sheets, not solids (OPEN_SHELL "a sheet, not a solid"): 05-0807, 05-0994, 05-0995,
     08-1865, 08-1866, and the sheet shells of 05-0223, 05-0224, 08-1717. Their once-used edges
     don't pair up as unsewn edges (checked: matching ends, different middles), so sewing doesn't
     help. Reading them needs sheet bodies, a feature.
   - 05-0407 (a pneumatic vise WORKHOLDING wants next): #90615 "a closed torus face with one rim and
     no pole". A torus face cut along one meridian whose rim is also used by a neighbour.
     Duplicating the rim gave EdgeUses, so it was reverted; it needs a look at the whole shell.
     Two more of its solids read but fail check: #2603 (FaceWrongSide, LoopsCross; 1322 faces) and
     #45516 (FaceWrongSide; measures disagree).
   - 05-0178: "no place on the rim for a second meridian" (heal.rs cap/join).
   - 05-0830: "its surfaces would take too long to measure". It is still refused with uncounted
     cache fills, so the read itself is heavy. IMPORT-SPEED tried a per-face mesh budget (2^27)
     and withdrew it: real faces spend too close to any cap (tl-3b #52: 226M; HFL-1313-3 #69702:
     141M), while HRT160-2's bad face spends 536M. Their suggested routes: a cheaper give-up inside
     the mesher's boundary recovery, or a smaller budget for model_files::read's display read.
4. **The corpus's open classes:**
   - boolean refusals against main's healing: FALLBACK has the list of 21;
   - FaceWrongSide wrong-Oks: rerun after train 8, which has FLAT-SITES' cone-apex fix;
   - forge-mesh's micro-edge crack (K050);
   - import cases: a sliver sphere, a corner patch with a pole, an O-ring as a closed NURBS.
   - They are in research/kernel-stress/haas-notes.md with owners.
5. **The decision log's sites:** once FLAT-SITES' "trim: a long shallow pass" site is on main, run
   `KS_DECISIONS=1 tools/kernel-stress/run.sh` and send them every hit.

## Risks

- **Fills are not bounded.** `uncounted` runs a cache fill unbounded, by design: a fill cut short
  would leave a half-made value in a cache that later builds share (main has that hazard today
  wherever a fill reaches the budget). A runaway inside a fill is not bounded or cancellable; the
  sweep shows none so far.
- **Long refusals.** OP = 2^29 lets a runaway take about 90 s to refuse (about 50 s with
  IMPORT-SPEED's faster NURBS evaluation). The ST-20/30 turret booleans need 289M, so it cannot be
  lower without making them faster.
- **The shared bank.** A nested `scoped()` on a thread that shares an op's bank adds no cap of its
  own; `isolated()` is the private counter. Code that relied on nested caps inside parallel work
  should use `isolated`.
- **forge-stress's box ops** are not the quarter box their comment once named (`Frame::from_z`
  picks its own axes). They are kept for continuity with the sweeps and the corpus.
- **Section areas** are mesh-based (chord 0.01) and move by up to about 1e-2 between kernels. diff.py
  treats a change above 1e-4 of the area (and above 1e-6 of the box squared) as a changed answer. A
  rewrite that changes meshing will show many such rows; each needs a look, but most are the mesh.
