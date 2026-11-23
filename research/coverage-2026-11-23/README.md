# Code coverage of the boolean corpus (2026-11-23)

Asked by David: "have we proven that's all the cases… is that all the code paths?" Corpus counts are
evidence, not proof; this measures which code the corpus never runs, so those paths can be ledger rows
(proof must cover them) or get fuzz families.

**Setup.** main 103dfb8b (before #33, so fuzz295 fails, as expected), default features, `--release`,
LLVM source-based coverage with branch counting (nightly-2026-09-17 = LLVM of the pinned 1.99.1).
Corpus: (a) gate parts regen1 seq/nary1 + bench parts (52/52 clean); (b) suite fuzz 1–300 seq and nary1
(299/300 each; fuzz295 fails = the seed-295 bug, fixed by #33); (c) exact reference: sweep-diff digest
identical to CI, reference repros 5/5, of the 9 ignored snap repros 6 fail as expected and **3 now pass**
(un-ignore candidates); (d) unit tests for forge-bool/geom/topo/model: 403 passed.

## Totals (corpus a+b+c; tests d separately)

| crate | lines | regions | branch outcomes | tests: regions / branches |
|---|---|---|---|---|
| forge-bool | 77.2% | 75.9% | 72.0% | 88.8% / 82.4% |
| forge-bool without imprint.rs | 88.8% | 87.8% | 82.7% | |
| forge-geom | 56.5% | 55.9% | 54.8% | 89.6% / 75.8% |
| forge-topo | 72.6% | 70.3% | 54.3% | 87.7% / 72.1% |

imprint.rs (the FORGE_IMPRINT column, G8) is 1.3%. decide_once / decisions are not compiled by default.

## Never run by the corpus
1. **Refusals**: gate + fuzz produce no forge-bool refusal and no retry. **17 of 31 refusal sites run
   under nothing, tests included**: boolean.rs RetryInvalid, the undecided_touch refusal, OpenCurve, Uncut,
   NoNormal, DegenerateEdge, SplitOpen, EmptyCycle, HoleOutside, NoInteriorPoint, UntouchedOnOther;
   nary.rs NoClosedForm, ToolVerticesMeet, NoNormal, NoInteriorPoint, EmptyCycle, MergedRegion.
   (refusal-sites.tsv)
2. **Retry ladder (G2–G7)**: the FENCES rung runs under nothing (ODD is set 21× — bench frame, sweep-diff —
   never with an error); CAREFUL never in the corpus; no retried boolean in the corpus returned Ok (G3's
   after-retry check judged only 6 unit-test cases); pinched_new never finds a pinch; refuse_strays never
   refuses.
3. **G14**: trim_in B:2625 box-only accept true all 171,474 times (point_in_face never evaluated there);
   B:2954 664k box-only accepts; ~836k cut decisions rest on the box (proven harmless in #27, given D4).
4. **Degenerate rays**: every degenerate-ray branch in classify's parity (zero denominator, non-finite hit,
   hit near an edge end, conic tangency) is never taken by anything = §1a's all-rays-degenerate path.
5. **Surface pairs**: spun-spline meets, ruled–ruled walls, parabola_wall, spindle tori, cone–cone with a
   shared apex, plane–cylinder a hair off parallel, plane–cone apex and sliver cases.
6. **n-ary**: no corpus n-ary run has an Intersect tool (every any_intersect path skipped); threaded
   par_map arms skipped (only nary1 runs); touches inside n-ary never reached.
7. **Debug switches** (FORGE_DEBUG/SPECS/PHASES) account for many of the ~726 unrun forge-bool regions.

## Hot spots
boolean.rs:289 on_edge 2.78M checks (1.65M true); `edge.tol > LINEAR` at boolean.rs:2645 false all 1.73M
times; tol-sites.tsv lists all 164 tolerance checks: 13 never evaluated, 51 always one side.

## Cheapest corpus additions
The exact-fuzz families in the coverage run; an n-ary Intersect family plus naryN runs; apex/pole/
spun-spline fuzz families; a `--features decide_once` build.

## Files
refusal-sites.tsv, bool-listing.txt (every unrun region/branch by function), geom-listing.txt,
topo-listing.txt, tol-sites.tsv, hot_branches.tsv, hot_fns.tsv, fn/br/rg_uncovered.tsv, attrib.txt (which
part reaches each rare line), crate-totals.txt, tools/ (the scripts). HTML and per-part profiles stayed on
the laptop (~/Documents/forgecad-coverage-2026-11-23/).

**For the coordinator**: each never-run group → a ledger row (owner: FALLBACK for 1–2 and 6, FLAT-SITES
for 4, CURVED-SITES for 5) marked "no corpus evidence: proof must cover it, or add a fuzz family"; and
un-ignore the 3 snap repros that now pass.
