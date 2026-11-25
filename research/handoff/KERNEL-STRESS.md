# KERNEL-STRESS handoff

KERNEL-STRESS ran the kernel over the Haas machine models (146 zips, 194 STEP files, about 11,600
solids) and the Haas Tooling workholding files (285), fixed what it found in its own area, and
routed the rest to owners. The sweep is the kernel's differential net: `tools/kernel-stress/compare.sh
BASE NEW` runs it on two refs and reports every difference, wrong answers first.

Written 2026-11-25 at wind-down. Main was 247a2694 (train 10). None of the branches below was in
trains 8 to 10.

## Branches, in landing order

| branch | head | what | state |
|---|---|---|---|
| kernel-stress | 85601f88 | the harness (verify/forge-stress, tools/kernel-stress), the corpus and report (research/kernel-stress), the reader's fixes (forge-step: direction noise, whole torus, flat holes filled), the section fixes (forge-agent), forge-asm rigid_copy | PRE-VERIFY GREEN on main 10d87778, tool b9318989; the line is with TRAIN-CONDUCTOR |
| op-budget | fd8c9da2 | every top-level op has a work budget (`budget::OP`, MESH for a file body's mesh); par helpers share it; an op's spend does not hang on order or threads (`uncounted` cache fills, memo costs); an op is cut short only past its budget, whatever its threads hold (`Money`: bank, debt, out); `isolated` | independent re-review running; pre-verify at fd8c9da2 running when this was written |
| gate/nurbs-ops | e5d88107 | the speed gate's first NURBS ops (`stepop1`: the quarter box on two small Haas solids), strict lines | WIP: the baseline lines and the isolation table still to add (Open work 2). Lands before ks/far-line-torus |
| ks/far-line-torus | f1cef02b (code 2963c65e + nested-budget fix) | FALLBACK's two tracer fixes (it contains ks/nurbs-stall 6502ae9a) bounded: `budget::cap_total(CARRIED = 2^24)` once a branch is carried onto a spline rim; a trace stops once its op is over; ssi's memo asks a remembered search's cap again | independent re-review running; lands after op-budget and gate/nurbs-ops |
| mass-cap | a622f957 | forge-topo mass: a cap round a NURBS apex weighs right (it was silently zero) | main 10d87778 merged; merge main again and pre-verify |
| face-box | 6815ce67 | forge-topo uv: a face on a surface closed one way gets a box as far as its loops reach | main 10d87778 merged; merge main again and pre-verify |
| import-sheets | (this file's branch) | open shells read as sheets (`Imported::sheets`), drawn by the machine-model path; heal.rs: a sphere's pole mid-edge, a spindle torus's apex, seams that cross their rims; `tools/kernel-stress/edge_fit.py` | not pre-verified. Conflicts with IMPORT-SPEED's `finish` split (import/faces): rebase onto it once that is on main (agreed) |
| op-cancel | 7166abd0 | on op-budget: cancelling a running build, progress, nothing of a stopped build kept | kernel and plumbing done and tested; UI not done |

Pre-verify: `python3 /tmp/pv/tools/preverify/run.py` from the branch's checkout, with the tool at
the head of `tools/preverify` (lines end "tool <sha>"); at least 2 GB free or a step is RED. Merge
main first. The native step prints FAIL before its verdict; read the verdict line.

## The tools

- `tools/kernel-stress/run.sh -o OUT -j N [-s haas|tooling] [ZIP...]`: the sweep. `-s tooling` reads
  the Haas Tooling workholding STEP files (branch haas-tooling-step, fetched shallow, never merged).
  `KS_NARY=1` runs booleans through n-ary on all threads. `KS_DECISIONS=1` records the decision
  log's named sites.
- `tools/kernel-stress/compare.sh BASE NEW [-j N] [-o DIR] [ZIP...]`: builds both refs with THIS
  checkout's harness (run it from a checkout whose forge-stress records `evals`, op-budget's or
  later, to see spends) and sweeps each once (cached by harness, ref and zip set). Writes a report
  from `diff.py`; exits 1 on new wrong answers, hangs, or changed answers.
- `tools/kernel-stress/corpus.py make|run`: the corpus, plus `research/kernel-stress/pinned.tsv`
  (cases named by hand, each with an owner; HA5C-T #80156 and UMC-500 #69561 are there instead of
  large fixtures).
- `tools/kernel-stress/extract.py STEP ENT[,ENT] out.step [--faces N,..]`: one solid out of a file
  (or some of its faces, as a sheet).
- `tools/kernel-stress/edge_fit.py STEP EDGE_CURVE..`: a file's own edges against its own faces,
  with no forge code: a refusal's evidence (ds-40y #249171: an edge 0.96 mm off a face).
- A full sweep takes about 3.5 h at -j3 on 4 cores; the 8 zips of research/kernel-stress/
  diff-op-budget-far-line-torus.md about 20 min.

## What the last sweeps showed

- **op-budget against main, imports**: 195 of 196 files read the same; ds-40y #249171 now reads
  (it was refused, order-dependently, as too long to measure). The file is wrong there (edge_fit).
- **op-budget's speed gate** (callgrind, at 226e0527): strict total -0.013% against main's baseline,
  bits identical; worst lines insert app/edit/cli +0.36%, heatsink cli +0.28%, clamp app +0.18%,
  tour +0.10%; best bracket -0.41%, coplanar -0.40%, crank -0.32%. Accepted under the correctness
  exception (the budget removes thread-order-dependent verdicts). What remains: `uncounted` around
  cache fills of procedural geometry (insert's fillets: those fills do spend and must be uncounted
  to be order-independent), the per-op `scoped` budget and its drop, and inlining shifts (box
  lookups now inline into callers like `forge_ops::rebuild::solve_point`). Closed-form fills skip
  the wrapper (`Curve/Surface::may_spend`; debug builds assert a plain fill spends nothing).
- **ks/far-line-torus against op-budget fd8c9da2** (8 zips, 3343 ops): 0 new wrong, 0 ok to
  refused, 12 refused ops answer (HA5C-T #51342, #80156; hvs-1 #1620; ST-28L #645206), 15
  answers given only through the old wrong-answer path are refused again, op time -24.5%, the
  slowest refusal past a carry 3.4 s (33 s uncapped). The trade (CARRIED 2^24 keeps ST-28L #673016
  union_pair66's answer at 14.5M; 6 * 2^20 would end refusals under 2 s but refuse it) was ruled by
  the coordinator; it is in the report.

## Open work

1. **Landing op-budget fd8c9da2 and ks/far-line-torus.** The re-review found blockers A and B SAFE
   and one ISSUE: a nested `scoped` reset the outer's TOTAL and CAPPED. That is fixed at f1cef02b,
   with a test. Pre-verify op-budget, then gate/nurbs-ops, then far-line-torus, and send the lines
   to TRAIN-CONDUCTOR.
2. **gate/nurbs-ops**: run `GATE_COUNTER=callgrind tools/speed/gate.sh` on a quiet machine, append
   the `noop stepop1`, `k018-box op` and `ring-box op` lines to tools/speed/baseline-valgrind.txt
   (only those: callgrind counts are exact, every other line must come out unmoved, which is the
   isolation table to attach). Its pre-verify gate stage fails by construction (main's side has no
   such lines), as FALLBACK's gate-machinery branch did; say so with the line.
3. **The mesher's charge** (coordinator, after op-budget): forge-mesh cdt.rs charges constraint
   recovery 16 per step (lines ~697, ~796) where a step costs about 1/16 of an evaluation
   (`insert_interior`'s own rate): EC-1600ZT pallet pool #263173, a valid solid that draws in
   0.28 s, is refused at the plain read's 1x budget. Recalibrate from measured ns per step against
   ns per evaluation (a count still, so order-independent), then the import A/B with every moved
   verdict explained. Not started. The solid: `extract.py "EC-1600ZT- Pallet Pool -10_2025.STEP"
   263173` from ec-1600zt_pallet_pool_solid_models_10_2025.zip.
4. **op-cancel**: the progress bar and Cancel button need UX-LEAD's OK with screenshots; a cut's
   unbudgeted work makes a cancel take up to a slice (about 3 s on the app's path; target well under
   1 s); rebase onto fix/asm-shape-first (on main since train 10) with a `halt.stopped()` check
   after each forge_lang::run in the assembly block; WEB-WORKER merges it for the browser build.
5. **A cracked result's mesh is not said in the view.** Export says "not closed and manifold";
   the viewport (forge-render tess.rs) draws an open mesh silently. ux/undrawn-faces (train 10)
   lists faces left out, not a mesh with open edges; a one-line "mesh not closed" on the part needs
   an `Undrawn` field and the Issues window (UX).
6. **Mesh cracks (K050)** to IMPORT-SPEED: UMC-500 #87260 box_sub and #264610 union_over/over2
   gave Ok solids (check clean, volume oracle right) whose meshes had 3 and 44 open edges (before
   the review's fix A, which refuses #87260 again; #264610 is refused by the cap). The mesher's
   own: B (small-radius cylinders gridded to their radius) and C (intersection chains sampled per
   link) wait on the coordinator's rigor ruling; IMPORT-SPEED holds them.
7. **The gap rule** (ds-40y #249171's true reason): IMPORT-SPEED holds it on import/faces, tier 2
   ("edge #E lies X off face #F (the file's tolerance is T)" as a warning, the solid kept), after
   measuring worst-gap / uncertainty over the corpus. `body.unverified` is overwritten later by
   measures_agree: join the two messages.
8. **Haas Tooling** (285 files): all read now but for 05-0830 (refused as too long to measure; with
   IMPORT-SPEED, likely their cause B). The sheets (05-0994, 05-0995, 08-1865/1866 and the open
   shells of 05-0223/0224 and 08-1717, 51 in all) read as sheets on import-sheets; WORKHOLDING's
   compile (forge-workholding-builder compile.rs) reads `im.solids` by index and must take
   `im.sheets` to use them. Two single sliver faces (05-0994 #109088, 05-0995 #43971, about
   0.3 x 0.025 mm, their loops open in the file) mesh to nothing.
9. **The decision log's sites**: once FLAT-SITES' "trim: a long shallow pass" site is on main, run
   `KS_DECISIONS=1 tools/kernel-stress/run.sh` and send them every hit.

## Risks

- **A budget edge the re-review noted, not fixed:** certify's TRACED memo (feature certified_trace,
  off by default) keeps a trace that gave up on the budget as None, answered so after.
- **Fills are not bounded.** `uncounted` runs a cache fill unbounded, by design: a fill cut short
  would leave a half-made value in a cache later builds share. A runaway inside a fill is neither
  bounded nor cancellable; no sweep has shown one.
- **Long refusals.** OP = 2^29 lets a runaway take about 90 s to refuse (about 50 s with
  IMPORT-SPEED's faster NURBS). Past a rim carry the cap holds it to about 3.4 s. The ST-20/30
  turret booleans need 289M, so OP cannot be lower without making them faster.
- **Past a budget, threads run on owing** until their debt is at least what the others hold: at
  most threads x SLICE more work before an op past its budget stops. That is what makes the
  verdict the same on any machine.
- **`isolated` now returns what f spent, past its limit too** (it returned at most the limit).
  Nothing calls it on these branches; IMPORT-SPEED's `map_until` sums it.
- **forge-stress's box ops** are not the quarter box their old comment named (`Frame::from_z` picks
  its own axes); kept for continuity with the sweeps and the corpus.
- **Section areas** are mesh-based (chord 0.01) and move by up to about 1e-2 between kernels; diff.py
  calls a change past 1e-4 of the area (and 1e-6 of the box squared) a changed answer.
- **Disk.** Pre-verify, sweeps and gate runs each build a full target; this VM ran under 3 GB free
  more than once. Delete branch targets between runs (they rebuild).
