# forgeCAD coordinator handoff (2026-11-23, from the laptop session)

You are taking over as David's MAIN coordinating agent for forgeCAD (this repo, David-Bascom05/forgeCAD:
his personal Rust CAD kernel with Lean proofs; keep it out of any work systems: no Linear/Slack/Notion).
Until now a Claude Code session on his laptop ran this. It hands over because heavy work must not run
on the laptop. You run in a cloud VM: run the workers here (subagents in this VM, or further
`claude --cloud` sessions if available), keep the merge queue moving, and report to David.
This file lives on branch coord/handoff only: never merge it to main.

## 1. David's rules (all standing)
- **Proofs**: prove the SHIPPED boolean (kernel/forge-bool) end to end: `boolean_correct`
  (research/decide-once/old-path-statements.md §E/§D/§G; ledger verify/forge-bool-proof/LEDGER.md;
  research/PROOF-STANDARD.md). His words: "start agents on proving what needs to get proven. If they find
  something that is actually wrong, fix it. The fallback should trigger only when there are large gaps
  in existing proofs, and if possible we should be able to prove the fallback is not even needed. Use what
  existing agents have already done in branches." A8 decide-once = fallback only (2.6x slower), ideally
  proven unreachable.
- **Curved code**: "prove curve code, find bugs or gaps, and see how useful it is to close the gaps
  (obv fix bugs)".
- **No assuming**: "we should be careful assuming like that. if it is easy to close gaps in a provable way
  do it." Anything not settled by a proof or the exact reference is UNKNOWN (measured margins, "fired 0
  times", "rounding artifact" are evidence, not verdicts). Hypotheses stay hyp/open in the ledger, never
  proven. Easy provable gaps: close directly. Hard or speed-costly gaps: bring to David with numbers.
- **Speed**: "verify that speed (instructions) stay the same or better before we merge bug fixes (fight to
  get speed back if we are noticeably worse)". Accepted noise band (he said "accept it"): exact callgrind
  gate (tools/speed/gate.sh, GATE_COUNTER=callgrind, exact mode), main and branch counted on the SAME
  machine with the same valgrind; each strict line (regen1 synthetic parts; bench app/edit/cli) ≤ +0.03%
  over main and the strict-line total ≤ +0.03%; bench seq/nary1 lines diagnostic. Checker:
  tools/speed/band.py (on branch proof/forge-topo; copy below in §7). gate.sh's exact mode runs valgrind
  under `env -i PATH=/usr/bin:/bin`, so valgrind must be at /usr/bin/valgrind. Applies to every NEW PR
  (bug fixes, gap closures, refactors). Code-layout noise moves every line ~±0.03% for any binary change.
- **Kernel values**: analytic geometry only (no NURBS unless the data is one), every op ms (<10 ms goal,
  100 ms max), robustness; native = wasm32 bit-identical; no proofs at runtime (preconditions debug-only).
- **bits.txt** (tools/speed/bits.txt) changes only for a judged bug fix (exact reference / proof /
  invalid topology) or David's explicit OK.
- **CAM**: no adaptive / constant-engagement clearing until patent counsel (research/cam/design.md §8).
- **CI**: the older PRs (#13–#22, from the earlier round) merge on CI alone; NEW work (agents' PRs) also
  needs the band result in the PR.
- **Merging**: only YOU merge, one PR at a time, and only a PR whose head already contains current main
  (update the branch via `gh api -X PUT repos/David-Bascom05/forgeCAD/pulls/N/update-branch` and wait for its
  CI) with every CI run on that head green. That way CI tested exactly the tree that lands. Merge commits
  (`gh pr merge N --merge`). Never force-push others' branches; resolve conflicts with merge commits.
- **Workspace**: since #29, Cargo.toml `members` names the Rust crates under verify/ explicitly; Lean-only
  verify/ dirs need no entry; a new Rust crate under verify/ must be added to members.
- Commits end with "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"; PR bodies end with
  "🤖 Generated with [Claude Code](https://claude.com/claude-code)".

## 2. Merge queue (state at handoff, main = db5c0759)
In order; each: bring up to date with main, wait for CI, merge.
1. **#31** ci/main-cache-only (push-to-main runs only refresh caches; checks run on the PR). Jumps the line.
2. **#15** CAM code, **#16** CAM Lean (stacked on #15), **#14** fillet census — all three just updated to
   main db5c0759 (old PRs: CI only).
3. **#32** proof/curved-sites (curved-sites 1; band passed vs 103dfb8b; kernel unchanged since).
4. **#19** sketch text: its Lean heartbeat failure was just fixed (cbed8d09; aeneas run in progress).
5. **#33** proof/fix-torus-loops: seed-295 wrong-Ok fix (band passed vs 1c48925d; full workspace test run
   still pending — run it).
6. **#25** proof/spine-bodymodel, **#26** proof/spine-lean (stacked; #26 needs origin/main merged and
   LEDGER rows of all agents kept), **#27** proof/flat-sites (G14; append conflicts in ForgeCurved.lean /
   Axioms.lean after #26: keep both).
7. Older PRs needing real merge work (MERGE worker, §4.6): **#13** (WIP on branch merge13-wip), **#20**,
   **#21**, **#18**, **#22**.
CI facts: #30 made Lean jobs restore main's caches (warm since 20:33 UTC). wasm (~21 min) is the long pole
for Rust PRs. A push to a PR cancels its older runs (#23). Don't cancel main runs (cache saves).
GitHub free-tier concurrent-job limit: don't start CI on many PRs at once.

## 3. Bugs found so far
- **Confirmed**: fuzz seed 295 f69 cylinder×torus loop lost (wrong Ok) → #33 interim (not provably
  complete); forge-git wasm usize overflow (#13, fixed); CAM wasm temp-dir tests (#15, fixed).
- **Confirmed (cloud, TRANSLATE, exact reference)**: tilted thin wedge, A=box [0,L]x[0,10]x[0,10] vs
  prism whose top wall goes (0,y0)→(L,y1). B1: plane_plane decides Empty at q.frame.origin with
  ANGULAR 1e-11/LINEAR 1e-5 → unsound for extents 1e6..1.6e7 (L=1.6e7, y0=-2e-5, y1=6e-5: intersect =
  Ok with INVALID body (EdgeUses uses:1); union/subtract refuse ZeroThickness falsely; also asymmetric
  A,B vs B,A). B2 (independent, plane_plane right): gap ±1.2e-5 at L=1e5..2e6 → same wrong Ok; at
  1e3/1e4 Internal refusals; root cause downstream of D2, UNKNOWN. Repro branch proof/tilt-wrong-ok.
- **Not bugs**: fuzz24 (certifier artifact), G14 box-only accept (proven harmless).
- **Open**: G9 cache staleness (two mechanisms found by reading; corpus count pending); fuzz55 f21 G4
  UNDECIDED reachable, exact verdict open; TRANSLATE's possible plane_plane soundness issue beyond ~1e6 from
  q's origin vs InputSize 1.6e7 (check with the exact reference).

## 4. Workers to run (each was mid-task; full handoffs below). Restart each from its branch.
Shared VM setup: rustup stable + wasm32-wasip1 + node; elan (Lean v4.36.0) + `lake exe cache get`
(Mathlib); tools/aeneas/setup.sh for Charon/Aeneas (pins inside; OCaml via opam); python3.12 (forge-lang
core_diff); valgrind at /usr/bin. Charon/Aeneas on boolean.rs needs >12 GB RAM.

### 4.1 SPINE (statements, ledger, forge-topo)
- #24 merged (plan + LEDGER). #25 = proof/spine-bodymodel @789ae988. #26 = proof/spine-lean @6ba798a9
  (verify/forge-bool-proof: Statement, Contracts (9-step Pipeline, D1–D9, ArrOK, ClassAt, keepRule,
  BoundsFaces, ApplyPost), Compose, KeepRule (keepRuleBd_proof, boolean_correct_of_steps), Bridge
  (readInPlace, ShapeR, BooleanCorrectR, booleanCorrectR_of_steps); workflow forge-bool-proof.yml).
  Needs main merged (keep all agents' LEDGER rows; take main's Cargo.toml, drop its exclude entry).
- proof/forge-topo @209ddb2e WIP (no PR): forge-topo topo.rs/topo_shell.rs/topo_edges.rs (check_strays,
  links, shell_faces, edges, loops via Topo/ShellTopo/EdgeTopo views + Sink), check.rs View/ShellView,
  verify/aeneas/forge-topo (translation + refinements), verify/forge-topo-check/lean Flood.lean;
  PROOF-STANDARD "The speed rule" + tools/speed/band.py. With OVERFLOW_CHECKS=false all 5 refinements
  built; with checks ON `translate.sh forge-topo --check` passes (61 defs, 0 opaque) but proofs: Strays
  builds; Links fixes unbuilt (pass bounds explicitly: `step with usize_add_wspec e 1#usize (by scalar_tac)
  as …`); Checked.lean usize_sub_eq omega fix unbuilt; ShellFaces/Edges never built vs checked. If it won't
  close: OVERFLOW_CHECKS=false (commented line in verify/aeneas/forge-topo/translate.conf), keep T.ovf an
  open GAP, say so in the PR. Then gate (no valid numbers yet; keep vector sizes as old code), tests, wasm,
  PR (David approved landing it once gates pass).
- Then: D8.math (BoundsFaces for apply's output; FaceLocal.frontier_iff_bd etc. exist); T.euler, T.fans,
  T.all (rebase-fundamentals research/fundamentals/RESUME.md); D5.zone.
- Ledger (SPINE): proven E.comp, E.seq, E.R, D7.math, D8.derived, E.fb composition, D0 composition,
  B.model/F1/rooms, B.clusters/phi, S3, S4. Stated E, E.ref, D1–D9, D0, G1. Open E.names, E.size,
  D5.zone, D7.table, D8.math, T.ovf, T.euler, T.fans, T.all, T.valid. Hyp H.fin.
- Gotchas: forge-bool-proof packagesDir ../forge-curved/.lake/packages (cache get there first); forge-topo
  Lean uses ../../../forge-mass/lean/.lake/packages; Aeneas Result is an ITree (WP specs).

### 4.2 TRANSLATE (Aeneas L4)
- proof/translate @4eb03060 (main 103dfb8b merged): tools/aeneas from the aeneas branch + pre-pass
  `lower_loop_exits` (aeneas-robust.patch); setup.sh patch-set hash; translate.sh `DEPS=(…)` per-crate
  chaining, FORGE_KEEP; survey-modules.sh env vars. Old 5 translations byte-identical. New: forge-math +
  vec/frame/interval/tol; verify/aeneas/forge-geom (plane_surface, plane_plane, planes_coincident,
  spindle_in); verify/aeneas/forge-bool (ssi::surfaces_in, closed_or_flush, closed_form, nearly_flush,
  turned_flush, spread_apart). Bridge additions (f64 min/max with opaque F64.zeroTie, next_up/down, ceil,
  rem_euclid, …). ssi.rs rewrites (same float ops/order). research/aeneas/forge-bool.md + fn_survey.py.
- Next: gates (clippy, tests, wasm, exact fuzz, band — ssi rewrites must be in band) → PR "TRANSLATE 1";
  D2 proof (D2Flat.lean WIP: spindle_in Plane Plane = ok false (unfold, rw any's equation twice); then
  surfaces_in never reaches searched; F1 for plane_plane); POSSIBLE FINDING: plane_plane Empty/Coincident
  decided at q.frame.origin with ANGULAR 1e-11/LINEAR 1e-5, sound only within ~1e6 — check with the
  exact reference; forge-topo types under DEPS (Cached --opaque; coordinate with SPINE's forge-topo
  translation); classify.rs for FLAT-SITES (info opaque); after FALLBACK's flags PR: plan_once_in pair
  loop, trim, crossings_near, meet_region (Charon OOM: --start-from per function group; split the three
  giants into named steps = E.shape's Pipeline steps); imprint/nary Not_found: OCAMLRUNPARAM=b.
- Gotchas: Charon sees only optimized MIR of other crates (include types only; translate deps under
  DEPS); another crate's const = opaque Result global, bind in FunsExternal; regenerate dependency
  FunsExternal_Template when sources move; slice patterns need source rewrites.
- L4 gap rows: L4.proc, L4.intcurve, L4.cached, L4.searched, L4.geomcurved, L4.big, L4.zero (hyp by
  design); easy close: Array.map length check.

### 4.3 FLAT-SITES (classify / point_in_face)
- No flat bugs (exact-fuzz agree 463, wrong 0). #27 G14. Branches: proof/flat-sites-size @f56b4fd8
  (debug-only InputSize check; ready once band measured; ledger E.size.rs GAP (release) — David picks:
  creation-path checks or release refusal); proof/flat-sites-pif-at @21a6e127 (point_in_face_at with a
  `near` hook, separate #[inline(never)]; CURVED-SITES builds on it — gate + PR first); proof/flat-sites-
  onplane @b55858ef (classify On requires exact on_plane; measure cost); proof/flat-sites-g13 @5f0e901e
  (face_point own-face check; recover cost if over band); proof/flat-sites-1a @46b486c5 (Lean FlatSites/:
  Where, G15, OnPlane, G13; stacked on #26+#27; PR after #26; drop "rounding artifact" wording);
  proof/flat-sites-probes (tools only, not for merge).
- Next: gate + PR pif-at, size, onplane, g13; 1a after #26; D5 proof (needs TRANSLATE's imprint/
  place_cuts/split_face); ranked gaps #2 (36 pattern-copy points UNKNOWN; exact key fix only if
  speed-neutral), #4 ray branch (hard), #5 flat G11 (provenance proofs).
- Gotchas: never wrap point_in_face_past (inlining +0.13%); cfg-out debug-only code entirely.

### 4.4 FALLBACK (G2–G10, A8)
- #28 merged (verify/forge-bool-fallback). #33 open (seed 295). proof/fallback @0d7b91f7 flags PR NOT
  opened: OVER BAND (patterns seq +0.31%, coplanar seq +0.28%, torture/plate +0.19%; total +0.023%);
  callgrind: trim_in SELF +674k, point_in_face_past SELF +232k (inlining shift). Try: drop `let rung =
  tried.rung;` in trim_in and read tried.rung.X at each use; trim_in #[inline(never)]; compare
  regen1 patterns seq profiles. proof/g9-cache-wip @639a3296 (WIP): cache_ok check + G9 fix
  (recompute reach when a dropped edge's sag ≥ reach; clear query of a rotated face) — measure first.
- Next after flags: G10 (MemberSound leaf IEEE screens open); A8 as fallback (invert: today's path
  first, A8 only on Internal/undecided from ORIGINAL inputs; port by cherry-pick from
  claude/repo-review-mbdrlj-decide-once, never merge); blockers edge-name parity
  (research/booleans/bool-speed-open.md item 2) and random_programs fuzz (item 6); unreachability proof
  (fuzz55 f21 G4 exact verdict open).
- Counters: `cargo build --release -p forge-model --example regen1 --features decisions`;
  FORGE_DECISIONS_SITES=1 (TRACE=1 for rungs); results suite 17 + fuzz 1–300: all rungs/ODD/pinches/
  strays/D9/D3 0; G4 2 (fuzz55); verdict 3 2,447.

### 4.5 CURVED-SITES (curved sites, #7 certifier)
- #32 open. proof/curved-sites-absorb @24dd8472 WIP (fix-curved + exact-curved absorbed; 29 bits lines
  change; V/E/F identical etc. — that judges nothing: attribute each to a fix-curved commit with
  evidence (bisect) or bring to David). Don't delete fix-curved/fix-touch/exact-curved yet.
- Next: #1 point_in_face curved callers on pif-at (9 near-misses; #[cold]); absorb PR; #7 runtime
  (Krawczyk certifier: 477 Certified / 2 Undecided / 0 Missed / 90 cone hosts; median 2.6 ms/call → +0.3 s
  on mixed, ~1000× over band; try centred-form provable sample spacing, per-pair certificate cache,
  cheaper grid; Missed → add the event; only Undecided → typed refusal; if nothing fits: option (c)
  test-and-log, M.ct UNKNOWN — David decides); cone hosts M.cone; #3 B:2910 (η_trace bound); #4
  place_cuts provenance keys. For David: C.slid, 16 quartic separations, 36 unkeyed coincidences.
- Run: regen1 with `--features decisions`, FORGE_DECISIONS_DUMP=1; curved_one example (`locate`).

### 4.6 MERGE (older PRs)
- #13: merge13-wip @deb88039 (conflicts resolved: forge-lang lib.rs, cdt.rs Aeneas-style + #13 checks
  + Fault variants, tessellate.rs budget, nurbs.rs = main; hidden-inputs entries). test-wasm.sh FAILS on
  forge-read-fuzz corpus: (1) brep/line-edge-from-far-away.case → cdt.rs:1486 debug in_range on
  super-triangle corners (fix: FarFromOrigin also refuses out-of-range corners); (2)
  brep/cylinder-at-1e-308.case → predicates.rs:89 orient2d product underflow (find caller with a
  backtrace; option: refuse nonzero coords below 2^-485). Then merge current main (Cargo: add
  verify/forge-read-fuzz to members), gates, bits, push to #13's branch.
- Then #20 (hidden-inputs union; add verify/forge-certify to members), #21 (forge-math interval.rs keep
  both; CURVED's meet_cert uses Interval), #18 (aeneas-robust.patch keep both; TRANSLATE also changes
  tools/aeneas), #22 (regenerate the forge-weld translations).

### 4.7 CAM (new, David asked: "at least through 3-axis surfacing and turning, with simulation and
modeling of full machines")
After #15/#16 merge. Four workers: CAM-CORE (phase-1 gaps in research/cam/phase1.md: G4 leftover
classification, slot/chamfer/engrave/islands/helical ramps, checker speed f64 filter with proven bound,
G5 runtime limits, close verify/forge-cam RESUME.md hypotheses, keep shared types stable); CAM-3AXIS
(phase 3: closed-form drop/push cutter for analytic surfaces, certified B&B for tori/sweeps/splines,
z-level (not adaptive), parallel, waterline, scallop, pencil/rest, 3D in-process stock, Lean L1);
CAM-TURN (phase 5: lathe setup, facing/profile/groove/part/thread/drill, exact nose-radius comp, insert
library, lathe posts, G1–G6); CAM-MACHINE (research/cam/machines.md machine-definition format FIRST —
VMC, trunnion 5-axis, lathe, mill-turn — machine library as real parts, posted-G-code simulation against
kinematics (closes Fusion's gap), collision of all bodies through the motion, material removal, Lean
spec of the interpreter, timeline output). Read research/cam/*.md and verify/forge-cam/RESUME.md first.

## 5. Waiting on David
- #7 runtime rule for the certified Poly4 event search (options a/c/d with numbers).
- E.size release guarantee (creation-path checks vs release refusal).
- curved-sites-absorb bits changes that can't be tied to a judged fix.
- Hard curved gaps (C.slid tangent roots, quartic separations).
- Pending: a code-coverage report of the corpus (~/Documents/forgecad-coverage-2026-11-23.md on the
  laptop; the laptop session will push it to this branch as research/coverage-2026-11-23.md when done).

## 6. Pitfalls learnt today
- Agent `isolation: "remote"` silently ran locally on the laptop; check `hostname` in every worker.
- GitHub won't run pull_request CI on a conflicting PR. Stale digests (verify/forge-exact/fuzz/tests/
  sweep-diff.digest) and Aeneas translations (translation-fresh) are common CI failures: regenerate.
- Keep each worker's scratch files private; never use generic names in shared dirs.

## 7. tools/speed/band.py (copy)
See branch proof/forge-topo, or: strict lines = regen1 parts (torture plate shaft housing coplanar patterns
mixed hole-grid tour upto upto-flat) and bench app/edit/cli; fail if any strict line > +0.03% over base or
the strict sum > +0.03%; bench seq/nary1 diagnostic.

## 8. Cloud coordinator state (from 2026-11-23 21:20 UTC)
Coordinator: cloud session session_01WJtawwACx4hZuk77VMadNk (hostname `vm`, 4 cores, 15 GB).
Workers (each its own cloud session, tag `forgecad-worker`; they report via send_message):
- SPINE        session_01WJtawvzNrcD4yRNQVEeHda
- TRANSLATE    session_01WJtawzk4EzQbVx9RRkma3R
- FLAT-SITES   session_01WJtawx9MPCJQJvxtMfau59
- FALLBACK     session_01WJtawvTfttkoAtbqGu3AmW (first: #33 full workspace tests)
- CURVED-SITES session_01WJtawyv7xKrBkRdL3dwDcS
- MERGE        session_01WJtawyh9MKsEbjQPJ9FL4j
- CAM-CORE     session_01WJtax12n5PFR2L9uPMb7is (owns shared CAM types)
- CAM-3AXIS    session_01WJtawx5yEFpe6hUFTao6CQ
- CAM-TURN     session_01WJtawvePKaogMxErDhFCRF
- CAM-MACHINE  session_01WJtawzvkvyfqNLd7PgZKGf (owns machine-definition format)
- CAM-UI       session_01WJtawySQLKzpnK7g6t2CVB (Manufacture workspace UI in forge-render, gui_cam*)
- CAM-SIM      session_01WJtawyn7zd9gKZdPzHRCgQ (toolpath + stock/machine simulation views, gui_camsim*)
  David: "a full sim that shows the toolpath and Fusion 360 level UI + features for the current scope".
  #20 decision (David): keep verify/forge-certify in members + certify-off -p test runs in CI/wasm.
  CAM workers started early at David's request: they branch from #16's branch
  (claude/repo-review-mbdrlj-cam-lean) and open PRs only after #15/#16 merge.
Queue: #31 CI running on 924bc4d0 (contains main db5c0759).

## 9. Merge log + David's relaxed rule (cloud coordinator)
David (22:0x UTC): "feel free to merge stuff as long as it will not mess things up". Applied as: a PR
whose CI is green may merge WITHOUT update-branch + re-CI only if `git merge-tree` of main + PR head,
diffed against the PR head, touches only files that cannot affect what the PR's CI tested (CI yaml,
disjoint research/ or examples of crates the PR doesn't touch). Kernel-touching PRs keep the full rule.
Merged: #31 (dee3b0d0), #14 (940ba39d), #15 (ee512a7c), #16 (2953130b). #32 updated → 73342d50, CI running.
#33: full workspace test passed (FALLBACK, 2206/0 failed at 8655b848); needs update-branch + CI.

## 10. Workers added later (cloud coordinator)
- CAM-UI session_01WJtawySQLKzpnK7g6t2CVB; CAM-SIM session_01WJtawyn7zd9gKZdPzHRCgQ (listed in §8)
- MACHINE-LIB-A (Haas, Tormach, Hurco, Fadal; now real tool/holder/collet library) session_01WJtawyyoT43CAnAu4XnE9P
- MACHINE-LIB-B (industrial machines; now post dialects) session_01WJtawycAi3RSnznUMrGpJE
- MACHINE-LIB-C (prosumer/hobby/Swiss + Taiwanese/Korean job-shop) session_01WJtawvUaxqFt9zG8dZq2pV
- MACHINE-BUILDER (custom machines + shared library-builder UI/CLI) session_01WJtawxzwPxyVa8T89FuUER
- URDF (export + robot editor) session_01WJtawweJtJyywYtq1m68FH
- WORKHOLDING (kernel/forge-workholding, real products first, soft jaws etc.) session_01WJtawz4GamJmXtW7gNNCqN
- ENVIRONMENTS (Environments tab / environment framework in forge-render) session_01WJtawyAbKnCzfjrk8XLyUr
- RENDER-STUDIO (PBR path tracer, appearances, HDRI) session_01WJtawz369LCQvS1hpKDMaJ
David's standing directions added today: prove code instead of runtime checks (keep checks only for
input-dependent answers; checkers become test oracles + on-demand); real data only (openly published,
cited per value, no sign-ups, no UA spoofing, unknown = UNKNOWN; unknown rates held high so time bounds
stay sound, limit checks report "not checked"); machine vendors' own tooling/workholding first; custom
machines/tools/workholding via a shared library builder; URDF linked to the assembly but overridable;
merge sensibly (merge trains: one PR/CI run per batch; see §9).
Merged: train 1 = PR #38 (e8150caa). Train 2 = PR #40 (CAM-CORE 1, CAM-MACHINE, CAM-TURN, fallback-coverage,
forge-topo #39) in CI. Train 3 candidates: #13, #20, #41 (cam/ui), cam/sim, #42, ci/axioms-strict, machine
library stacks (machines/fadal→haas-2, industrial-b, vendor-c; union sweep-diff digest lines), post/haas-audit,
CAM-CORE stack to cam/sym-ties, cam-turn follow-up, spine-euler, curved-sites-pif/absorb, TRANSLATE 1 + B1,
urdf-export, machines/builder, posts/dialects.

### §10b (2026-11-24) new workers, delegation
- TRAIN-CONDUCTOR session_01WJtawwrcJMe9MW7GeYQtHw now owns ALL merge trains: assembly, conflicts, PRs and CI. That is David's direction: "make a session handle merge nonsense so you don't block on it". Workers send train-ready shas to it and cc the coordinator.
- UI-BUGHUNT session_01WJtax11GnJCjfyG5XiZG3a: wasm/browser UI bugs. Lag first (measured), then dead clicks, then visual glitches (screenshots). Branch ui/bughunt-*.
- Adaptive clearing: David rejects a blanket ban. TOOLPATH-RESEARCH is writing a claim-level FTO with design-arounds (research/cam/toolpath-quality/fto-adaptive.md). TOOLPATH-QUALITY may build engagement measurement and engagement-based feed now; it builds the path generator once a design-around is named.
- tools/test-wasm.sh now runs the test binaries in parallel (train 3).

### §10c (2026-11-24) more workers, rules
- FEEDS-SPEEDS session_01WJtawvkuz1LZKSnUTEjphE: feeds, speeds and engagement as correct, tunable op parameters. Defaults are layered: library → material → machine clamp → user, with each value's source shown. It also owns the feeds.rs power bug fix (kc at the mean chip) and the limit checks. TOOLPATH-QUALITY owns path geometry, linking and engagement measurement.
- CAM-UI top priority: NC export, i.e. an NC Program object in the doc, Save As/download, post-all, setup sheet, and CLI `forgecad cam F post -o`.
- RULE (G9, FALLBACK): never write Body.vertices[..].tol directly in shipped code; use add_tolerant_vertex / Body::grow_vertex_tol.
- RULE: a change to a source file the Aeneas translations cite (e.g. forge-geom intersect.rs) needs regenerated translations (TRANSLATE runs translate.sh) in the same train.
- Curved search (#7, M.ct): option 1: keep today's search, run the certifier as a CI oracle, M.ct UNKNOWN.
- #21: waiting for David's OK on its 52 bits lines.
- BAND EXCEPTION (judged, coordinator): B1 near-parallel fix (FLAT-SITES 6f45e5fb on B2, with TRANSLATE cb344640). Patterns seq is about +0.034%, 0.004% over the per-line limit. Accepted because it fixes a false refusal on valid input inside InputSize. Open item: prove an edge-parameter bound so the cost can be recovered.
- Decisions 2026-11-24: climb is the default cut direction; an unknown op argument is an error with a closest-name fix; forge-machine's own digest may be re-pinned for an exact-in-reals speedup (kernel bits.txt untouched); David: judged bug fixes that change bits.txt merge without asking him.
- RULE (David, 2026-11-24): ONE materials library. Every material property (mechanical, thermal, machinability: iso513, hardness, kc1_1/mc, sheet-forming values, appearance colour) lives in kernel/forge-material/data/materials.toml, cited per key (source_<key>), absent = unknown. No parallel material tables in other crates. Tool-side data (cutting tables by group and substrate, tooling, presses) stays in its own crate.
- RULE (gate): band numbers count only when produced by tools/speed/gate.sh (fixed /tmp/forge-gate.XXXXXX dir form). Instruction counts move with the gate dir's path LENGTH, via argv/env on the stack, so ad-hoc scripts are not comparable. tools/gate-startup (FALLBACK) enforces the dir form, subtracts start-up, and has GATE_SELFTEST.
- Judged exceptions so far: B1 chain patterns seq +0.042%; spine-fans gearbox edit +0.04%.
- CAM FTO: adaptive is one-direction peels only (TOOLPATH-RESEARCH engineering OK, fto-adaptive.md §9), with a winding test in CI. Closed pockets use offset pocket. Adaptive ships only once it beats offset clear.
- Data: the Fusion 360 Gallery may be used (David), fetch-only into a git-ignored cache, never committed, with a CI guard.

## 10d. Decisions 2026-11-24 (David)
- SPEED OVER PROVABILITY: prefer the faster implementation; never reshape Rust for Aeneas if it costs anything;
  unprovable-now code stays opaque/modelled ("to prove" LEDGER rows). Bits identical + band still required.
- forge-num (EXACT-CORE session_01WJtawxSK6fJAPG2zYS3k9u): option (b) of research/kernel-merge/report.md —
  one dependency-free core for exact numbers, Ival, Shewchuk, 2D meets; domain crates stay separate.
  ZERO speed loss (no strict line slower than main); forge-math predicates/Shewchuk are the reference, moved
  verbatim with #[inline]; predicates' Big<N> untouched unless benchmarked ≥ as fast. "When in doubt, don't move."
- Speed-hunting sessions don't do work forge-num will replace (CAM-PERF dropped real.rs/f64-filter → EXACT-CORE).
- NO ACTIVE PATENTS in the repo (docs, comments, commit msgs). Expired patents may be cited as method sources
  with expiry date. No FTO/design-around text. TRAIN-CONDUCTOR greps every train. Old research/toolpath-quality
  branch never merges; use research/toolpath-quality-clean.
- Adaptive clearing ships OPT-IN (default stays clear); verdict: wins in steel on t_la across accel ×0.5–2,
  machine-dependent in aluminium.
- Vendor machine models: commit only with explicit redistribution licence (none found yet); user-supplied
  "Attach model…" + local cache; CI guard needs licence record. Haas terms need a human read (asked David).
- ffmpeg: built in CI from source (LGPL + OpenH264 pinned by git commit 8c7008ae…); FIRST WORKFLOW RUN /
  release needs David's go. Installers fetch it; PNG fallback.
- CAM fuzz/reliability session: WAIT until the forge-num rebuild lands (David).
- FreeCAD/Kiri baselines blocked: this session's sandbox refuses running third-party binaries; David's options
  pending (permission rule for /opt/bench, run them himself, or skip).
- CAM SCOPE CAPPED AT 3D (David, 2026-11-24): no 3+2 or 5-axis generation. 840D/TRAORI reading+sim stays for validation only.
- FEA: research only for now (FEA-RESEARCH session_01WJtawwqWL5HTwiC8HTrB6k, branch research/fea). Build waits on forge-num; needs orient3d/insphere in the shared core.
- Cycle time UI: always t_min (bound) beside t_scurve (estimate), never the estimate alone.
- CAM-UI adaptive entry: blocked until David retires "no adaptive until counsel" in CAM-UI's own session.
