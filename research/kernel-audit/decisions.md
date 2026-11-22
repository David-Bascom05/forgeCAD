# Decision log, phase 0: notes (by hand)

Measured on claude/repo-review-mbdrlj at 6979fa5c plus the instrumentation (feature `decisions`,
off by default). Everything below the line is the runner's output, unedited except for the output
path in its command line (`verify/forge-proof/examples/decisions.rs` at nightly meta depth: 4300
runs, 81 s on 3 threads).

Gates, feature off (state in DECISIONS-HANDOFF.md):
- callgrind gate against the base built at a same-length path: bits identical; no line over 0.5%;
  synthetic parts within 12 instructions (start-up), bench lines -0.31% to +0.39% around the base,
  within their run-to-run spread (heatsink nary1: 0.28% between two runs of one binary, +0.02% least
  to least; kernel functions net +1400 of 170M);
- release tests of forge-bool, forge-model, forge-lang and forge-proof: 1082 passed, 0 failed
  (126 binaries); the same with the feature on and FORGE_DECISIONS_TRACE: 1082 passed;
- fmt; workspace clippy `--all-targets -D warnings`, feature off and on.

## Site names

`on_edge <- boolean.rs:N` means `on_edge` called from line N of boolean.rs at this commit:

| line | caller |
|---|---|
| 2441 | `trim`: a plane face's curve meets the edge's own line or circle (`plane_meet`) |
| 2751 | `trim`: a crossing (or touch) of the curve with the surface across the edge |
| 3244-3245 | `trim`: a stretch of the curve along one boundary edge (its middle and both ends on it) |
| 4380 | `imprint`: a coplanar face's edge crossing an edge of the other face |

Sites added in this round:
- `curve_surface: hair roots (forge-geom)`: a conic's turn within LINEAR of the surface with roots
  beside it that never leave the tolerance; 0 (one touch) under ABSORB_HAIR_ROOTS, else 2 (the
  roots kept as crossings). Recorded at trim's curve against the edge's surface across, with
  `crossings_near`'s count at the same point beside it;
- `check: edge_off_face, 16 points`, `check: a stray edge, 256 points`, `certify: 256 points`:
  an edge on a face of its, in a "result check" ledger (refuse_strays, the RETRIED check,
  verify::stage, the runner's validity check);
- `unify: SAME (half LINEAR)` beside `unify: ssi::coincident (the boolean's test)` on the same
  two faces (planes or cylinders, facing the same way, within 100 LINEAR), in a "unify" ledger;
- `n-ary place: one reach` (the sweep's own pair test) and `n-ary place: clusters` (joined
  through others), for every two cut points within four tolerances; `n-ary place: a cut at its
  edge's end`.

## Headlines

1. **Every retry rung holds at least one regression test.** The corpus says otherwise, so the
   corpus is not enough to delete any. `FORGE_DECISIONS_TRACE` over the release tests (feature
   on), rescues only:

   | rung | tries ok | test |
   |---|---:|---|
   | CAREFUL | 6 | forge-bool tests/booleans.rs `a_hole_on_a_rounds_centre_circle_at_any_angle` (after ABSORB_HAIR_ROOTS and CHORDS each lost its way again) |
   | ABSORB_HAIR_ROOTS | 2 | forge-lang tests/kernel_fusion_parts.rs `disc_rim_tangent_to_a_flat_it_sits_on` |
   | CHORDS | 3 | forge-lang tests/kernel_build_failures.rs `a_tilted_cutter_hovering_over_a_face` |
   | FENCES | 1 | forge-lang tests/kernel_misc.rs `a_rounded_blade_stands_on_a_spline_hub` |
   | n-ary -> sequential: knife edge | 1 | forge-lang tests/kernel_fusion_parts.rs `disc_in_a_scoop_with_knife_edges` |

   The CAREFUL, CHORDS and ABSORB_HAIR_ROOTS results passed the RETRIED result check (11, all ok). NO_RUN_ON fired in
   no test; on the corpus it rescues `near_tolerance/20`. So the wip note "the soonest to delete
   are FENCES, CAREFUL, CHORDS and ABSORB_HAIR_ROOTS" is withdrawn: deleting any of them fails a
   test today. The decide-once rewrite has to make each of these six inputs pass without its
   rung. They are its acceptance tests.
2. **The near-coincident family still holds nearly every contradiction that goes with a lost
   way.** Its counts are unchanged from the wip run. Point on edge: trim's crossing (2751) says
   on, imprint's (4380) says off, 2316 facts. Vertex on surface: the coincident-pair implication
   against trim's edge-box test (2112) and its run-on end test (1157). Of the 1198 sequential
   plans that ended `Internal`, 1180 (98.5%) had a contradiction first. Of plans without one,
   18 of 96 784 did (0.02%).
3. **New: the hair-root choice disagrees with `crossings_near` (4379 facts) and with curve_surface's
   touching root (1420), and none of it predicts a failure.** Where forge-geom keeps a turn's roots
   as crossings (ABSORB off), `crossings_near` at the same point counts no crossing. In the examples
   the roots sit at the turn itself (the farthest root beside it is 0 to 1e-7 away), so the dedup by
   point downstream leaves one hit. These are double roots of a tangency reported twice. They come
   from many families: ulp suite 1548, ulp fuzz 455, exact_patterns 400, ball_poles, the bench parts
   jack, frame, quadarm and manifold. The sequential plans that ended `Internal` are the same 1180
   as before, so every plan this adds to "with a contradiction" came out (or was refused otherwise).
   Hence the share of contradicted plans that end `Internal` falls from 68% to 42% (1180 of 2812). A
   contradiction in the near-coincident kinds predicts a lost way. One in touch or crossing does
   not, on this corpus.
4. **New: unify's SAME (0.5 LINEAR) against the boolean's `ssi::coincident` (LINEAR): 20 facts,
   all near_tolerance.** Two planes 6e-6 apart are one surface to the boolean and two to unify.
   So unify leaves an edge between faces that the boolean imprinted as coplanar. No plan failed
   on this; it is the same threshold split one step later.
5. **New, no disagreement:**
   - the n-ary place step's one-reach test and its clusters never disagreed (2.0M pair
     decisions): transitivity did not merge anything the direct test would split. Both
     disagree with trim's dedup on 144 facts, the same kind of split as sequential's (item 6);
   - check's edge-on-face: 468k answers within 100 LINEAR, all from the 16-point site and none
     contradicted. The stray-edge site gave no answer. certify's is not collected here: the meta
     `certify` property calls it outside a `checked` ledger.
6. **Two cuts one vertex** (659 facts, no failures): trim's dedup (which also asks the curve's
   middle to lie at the point) against place_cuts' clusters (515), trim's shallow pass (522), and
   the n-ary clusters (144). Mostly a curve coming back to a point it passed, by design.

## Retry rungs (corpus)

| rung | fired | rescued on the corpus | rescues in the regression tests |
|---|---:|---|---|
| FENCES | 0 | — | 1 (`a_rounded_blade_stands_on_a_spline_hub`) |
| `bends` (crossing_on_face) | 0 | — | 0 |
| ABSORB_HAIR_ROOTS | 301 | none | 2 (`disc_rim_tangent_to_a_flat_it_sits_on`) |
| CHORDS | 301 | none | 3 (`a_tilted_cutter_hovering_over_a_face`) |
| CAREFUL | 2 | none | 6 (`a_hole_on_a_rounds_centre_circle_at_any_angle`) |
| NO_RUN_ON | 297 | `near_tolerance/20` (2 tries, both ok, RETRIED check passed) | 0 |
| n-ary -> sequential | 1 | none (ProbeOnToolEdge on coincident_walls/1, refused again) | 1 knife edge (`disc_in_a_scoop_with_knife_edges`); 41 "names missing" (Other) |

---

# The boolean's decision log: contradictions and retries

`cargo run --release -p forge-proof --features decisions --example decisions -- threads 3 out research/kernel-audit/decisions.md`: 4300 runs in 81 s. Written by that run (the notes above it are by hand).

## Per fact kind

| fact | decisions | facts | facts decided at 2+ sites | contradictions | of which by one site only |
|---|---:|---:|---:|---:|---:|
| point on edge | 3298947 | 1832464 | 347506 | 2321 | 2 |
| point at vertex | 1187653 | 105849 | 15309 | 2 | 2 |
| vertex on surface | 2574309 | 528868 | 151013 | 2945 | 0 |
| curve along edge | 206956 | 199774 | 0 | 0 | 0 |
| two cuts one vertex | 17333905 | 884664 | 309218 | 659 | 0 |
| two faces one surface | 223576 | 65916 | 62495 | 20 | 0 |
| touch or crossing | 40300 | 14238 | 14238 | 4401 | 0 |
| edge on face | 468340 | 468340 | 0 | 0 | 0 |

Decisions recorded outside any plan (an n-ary pass's helper threads): 30873.

## Plans: does a contradiction go with a lost way?

| path | plans | with a contradiction | of those ok / internal / other / unknown | without: ok / internal / other / unknown |
|---|---:|---:|---|---|
| n-ary | 1373 | 78 | 49 / 0 / 29 / 0 | 1223 / 0 / 72 / 0 |
| result check | 101004 | 0 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 101004 |
| sequential | 99596 | 2812 | 1542 / 1180 / 90 / 0 | 96449 / 18 / 317 / 0 |
| unify | 8442 | 12 | 0 / 0 / 0 / 12 | 0 / 0 / 0 / 8430 |

## Site pairs that disagree

Per fact kind, the pairs of sites that gave one fact two answers, most facts first.

| fact | site | other site | facts | paths | families |
|---|---|---|---:|---|---|
| point on edge | `on_edge <- boolean.rs:2751` | `on_edge <- boolean.rs:4380` | 2316 | sequential 2316 | hair-under-notched-top ulp 2304, rel near_tolerance 8, meta ulp near_tolerance 4 |
| point on edge | `on_edge <- boolean.rs:3244` | `on_edge <- boolean.rs:4380` | 770 | sequential 770 | hair-under-notched-top ulp 768, rel near_tolerance 2 |
| point on edge | `on_edge <- boolean.rs:2441` | `on_edge <- boolean.rs:4380` | 8 | sequential 8 | rel near_tolerance 8 |
| point on edge | `on_edge <- boolean.rs:2751` | `point_in_face: nearest edge` | 7 | sequential 7 | rel near_tolerance 4, meta ulp near_tolerance 3 |
| point on edge | `on_edge <- boolean.rs:2441` | `point_in_face: nearest edge` | 6 | sequential 6 | rel near_tolerance 4, meta ulp near_tolerance 2 |
| point on edge | `on_edge <- boolean.rs:3245` | `on_edge <- boolean.rs:4380` | 2 | sequential 2 | rel near_tolerance 2 |
| point on edge | `point_in_face: nearest edge` | `point_in_face: nearest edge` | 2 | sequential 2 | rel near_tolerance 2 |
| point at vertex | `place_cuts: a cut at its edge's end` | `place_cuts: a cut at its edge's end` | 2 | sequential 2 | rel near_tolerance 2 |
| vertex on surface | `coincident faces => their vertices (derived)` | `trim: edge box misses the curve's` | 2112 | sequential 2112 | hair-under-notched-top ulp 2104, rel near_tolerance 8 |
| vertex on surface | `coincident faces => their vertices (derived)` | `trim run-on: the edge's end on the curve` | 1157 | sequential 1157 | hair-under-notched-top ulp 1137, rel near_tolerance 12, meta ulp near_tolerance 8 |
| vertex on surface | `trim run-on: the edge's end on the curve` | `trim: edge box misses the curve's` | 372 | sequential 372 | hair-under-notched-top ulp 368, rel near_tolerance 4 |
| vertex on surface | `trim run-on: the edge's end on the curve` | `trim run-on: the edge's end on the curve` | 2 | sequential 2 | rel near_tolerance 2 |
| two cuts one vertex | `trim shallow: within 1.2 tolerances, left` | `trim: a curve's cuts deduplicated` | 522 | sequential 389, n-ary 133 | meta backends suite 104, meta boolean suite 84, meta ulp suite 42, rel suite 42, +29 more |
| two cuts one vertex | `place_cuts: clusters and chains` | `trim: a curve's cuts deduplicated` | 515 | sequential 515 | meta boolean suite 100, meta ulp suite 50, meta mirror suite 47, meta rigid suite 45, +42 more |
| two cuts one vertex | `n-ary place: clusters` | `trim: a curve's cuts deduplicated` | 144 | n-ary 144 | meta backends suite 93, rel suite 24, meta backends fuzz 8, bench jack 6, +6 more |
| two cuts one vertex | `n-ary place: one reach` | `trim: a curve's cuts deduplicated` | 144 | n-ary 144 | meta backends suite 93, rel suite 24, meta backends fuzz 8, bench jack 6, +6 more |
| two cuts one vertex | `trim: a curve's cuts deduplicated` | `trim: a curve's cuts deduplicated` | 136 | sequential 117, n-ary 19 | meta boolean suite 16, meta backends suite 15, meta mirror suite 12, meta order suite 8, +26 more |
| two faces one surface | `unify: SAME (half LINEAR)` | `unify: ssi::coincident (the boolean's test)` | 20 | unify 20 | rel near_tolerance 20 |
| touch or crossing | `crossings_near` | `curve_surface: hair roots (forge-geom)` | 4379 | sequential 3605, n-ary 774 | meta ulp suite 1548, meta ulp fuzz 455, rel exact_patterns 400, bench jack 244, +43 more |
| touch or crossing | `curve_surface: a touching root` | `curve_surface: hair roots (forge-geom)` | 1420 | sequential 1084, n-ary 336 | rel exact_patterns 400, rel ball_poles 204, meta backends ball_poles 150, meta boolean ball_poles 120, +26 more |
| touch or crossing | `crossings_near` | `curve_surface: a touching root` | 22 | sequential 21, n-ary 1 | rel degen_cylinder_cone 7, meta boolean suite 4, meta mirror suite 2, meta order suite 2, +5 more |

An example of each (input [path; retry flags] entities and points: each site's answer, its measure vs its limit):

- point on edge / `on_edge <- boolean.rs:2751` vs `on_edge <- boolean.rs:4380`: near_tolerance/8 [sequential; plain] E6 (37.080889, 23.134408, 10.000010): `on_edge <- boolean.rs:2751` says 1 (0.000e0 vs 1.000e-5), `on_edge <- boolean.rs:4380` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `on_edge <- boolean.rs:3244` vs `on_edge <- boolean.rs:4380`: near_tolerance/8 [sequential; plain] E6 (37.080889, 26.895624, 10.000000): `on_edge <- boolean.rs:3244` says 1 (1.000e-5 vs 1.000e-5), `on_edge <- boolean.rs:4380` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `on_edge <- boolean.rs:2441` vs `on_edge <- boolean.rs:4380`: near_tolerance/8 [sequential; plain] E6 (37.080889, 23.134408, 10.000010): `on_edge <- boolean.rs:2441` says 1 (0.000e0 vs 1.000e-5), `on_edge <- boolean.rs:4380` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `on_edge <- boolean.rs:2751` vs `point_in_face: nearest edge`: near_tolerance/8 [sequential; plain] E26 (37.080889, 23.134408, 10.000000): `on_edge <- boolean.rs:2751` says 1 (2.000e-6 vs 1.000e-5), `point_in_face: nearest edge` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `on_edge <- boolean.rs:2441` vs `point_in_face: nearest edge`: near_tolerance/8 [sequential; plain] E26 (37.080889, 23.134408, 10.000000): `on_edge <- boolean.rs:2441` says 1 (2.000e-6 vs 1.000e-5), `point_in_face: nearest edge` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `on_edge <- boolean.rs:3245` vs `on_edge <- boolean.rs:4380`: near_tolerance/8 [sequential; plain] E6 (37.080889, 23.134408, 10.000000): `on_edge <- boolean.rs:3245` says 1 (1.000e-5 vs 1.000e-5), `on_edge <- boolean.rs:4380` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `point_in_face: nearest edge` vs `point_in_face: nearest edge`: near_tolerance/23 [sequential; plain] E11 (50.000005, 10.189440, 11.000000): `point_in_face: nearest edge` says 0 (1.500e-5 vs 1.000e-5), `point_in_face: nearest edge` says 1 (7.500e-6 vs 1.000e-5)
- point at vertex / `place_cuts: a cut at its edge's end` vs `place_cuts: a cut at its edge's end`: near_tolerance/18 [sequential; plain] V32 (50.000001, 15.985226, 9.999990): `place_cuts: a cut at its edge's end` says 0 (1.345e-5 vs 1.000e-5), `place_cuts: a cut at its edge's end` says 1 (1.000e-5 vs 1.000e-5)
- vertex on surface / `coincident faces => their vertices (derived)` vs `trim: edge box misses the curve's`: near_tolerance/8 [sequential; plain] V16 F0 (37.743516, 25.015016, 9.999998): `coincident faces => their vertices (derived)` says 1 (1.200e-5 vs NaN), `trim: edge box misses the curve's` says 0 (1.200e-5 vs 1.000e-5)
- vertex on surface / `coincident faces => their vertices (derived)` vs `trim run-on: the edge's end on the curve`: near_tolerance/18 [sequential; plain] V32 F0 (50.000010, 15.985226, 10.000000): `coincident faces => their vertices (derived)` says 1 (1.000e-5 vs NaN), `trim run-on: the edge's end on the curve` says 0 (1.345e-5 vs 1.000e-5)
- vertex on surface / `trim run-on: the edge's end on the curve` vs `trim: edge box misses the curve's`: near_tolerance/22 [sequential; plain] V10 F5 (50.000010, 15.722265, 0.000000): `trim run-on: the edge's end on the curve` says 1 (1.000e-5 vs 1.000e-5), `trim: edge box misses the curve's` says 0 (1.000e-5 vs 1.000e-5)
- vertex on surface / `trim run-on: the edge's end on the curve` vs `trim run-on: the edge's end on the curve`: near_tolerance/18 [sequential; plain] V32 F0 (50.000010, 15.985226, 10.000000): `trim run-on: the edge's end on the curve` says 0 (1.345e-5 vs 1.000e-5), `trim run-on: the edge's end on the curve` says 1 (1.000e-5 vs 1.000e-5)
- two cuts one vertex / `trim shallow: within 1.2 tolerances, left` vs `trim: a curve's cuts deduplicated`: coaxial/8 [sequential; plain] E68 E69 (30.057775, 22.925376, -1.000000) (30.057775, 22.925376, -1.000000): `trim shallow: within 1.2 tolerances, left` says 1 (0.000e0 vs 1.000e-5), `trim: a curve's cuts deduplicated` says 0 (0.000e0 vs 1.000e-5)
- two cuts one vertex / `place_cuts: clusters and chains` vs `trim: a curve's cuts deduplicated`: coplanar_stack/5 [sequential; plain] E1 E73 (6.766903, 36.149773, 11.000000) (6.766903, 36.149773, 11.000000): `place_cuts: clusters and chains` says 1 (0.000e0 vs 1.000e-5), `trim: a curve's cuts deduplicated` says 0 (0.000e0 vs 1.000e-5)
- two cuts one vertex / `n-ary place: clusters` vs `trim: a curve's cuts deduplicated`: patterns [n-ary; plain] E1 E1 (100.619776, 203.743236, 19.000000) (100.619776, 203.743236, 19.000000): `n-ary place: clusters` says 1 (0.000e0 vs 1.000e-5), `trim: a curve's cuts deduplicated` says 0 (0.000e0 vs 1.000e-5)
- two cuts one vertex / `n-ary place: one reach` vs `trim: a curve's cuts deduplicated`: patterns [n-ary; plain] E1 E1 (100.619776, 203.743236, 19.000000) (100.619776, 203.743236, 19.000000): `n-ary place: one reach` says 1 (0.000e0 vs 1.000e-5), `trim: a curve's cuts deduplicated` says 0 (0.000e0 vs 1.000e-5)
- two cuts one vertex / `trim: a curve's cuts deduplicated` vs `trim: a curve's cuts deduplicated`: coplanar_stack/5 [sequential; plain] E1 E73 (6.766903, 36.149773, 11.000000) (6.766903, 36.149773, 11.000000): `trim: a curve's cuts deduplicated` says 0 (0.000e0 vs 1.000e-5), `trim: a curve's cuts deduplicated` says 1 (0.000e0 vs 1.000e-5)
- two faces one surface / `unify: SAME (half LINEAR)` vs `unify: ssi::coincident (the boolean's test)`: near_tolerance/7 [unify; plain] F45 F47: `unify: SAME (half LINEAR)` says 0 (6.000e-6 vs 5.000e-6), `unify: ssi::coincident (the boolean's test)` says 1 (6.000e-6 vs NaN)
- touch or crossing / `crossings_near` vs `curve_surface: hair roots (forge-geom)`: ball_poles/0 [sequential; plain] F0 F1 E12 (26.724129, 18.123464, 10.000000): `crossings_near` says 0 (NaN vs NaN), `curve_surface: hair roots (forge-geom)` says 2 (0.000e0 vs 0.000e0)
- touch or crossing / `curve_surface: a touching root` vs `curve_surface: hair roots (forge-geom)`: ball_poles/0 [sequential; plain] F0 F1 E12 (26.724129, 18.123464, 10.000000): `curve_surface: a touching root` says 0 (NaN vs NaN), `curve_surface: hair roots (forge-geom)` says 2 (0.000e0 vs 0.000e0)
- touch or crossing / `crossings_near` vs `curve_surface: a touching root`: cylinder_cone/apex_on_top_rim/Add [sequential; plain] F0 F2 E1 (10.000000, 0.000000, -0.000000): `crossings_near` says 1 (NaN vs NaN), `curve_surface: a touching root` says 0 (NaN vs NaN)

## Retry rungs

Each try a rung made, by how it ended (ok: the plan came out; internal: lost its way again; other: refused otherwise; unknown: a rung that only marks the boolean as retried).

| rung | tries | ok | internal | other | unknown | by section (tries/ok) | inputs (first 12) |
|---|---:|---:|---:|---:|---:|---|---|
| ABSORB_HAIR_ROOTS | 301 | 0 | 301 | 0 | 0 | hair-under-notched-top 294/0, lang 4/0, meta ulp 1/0, rel 2/0 | lang seed 2; lang seed 22; near_tolerance/20; ulp adv near_tolerance 2; ulp seed 0; ulp seed 1; ulp seed 2; ulp seed 3; ulp seed 4; ulp seed 5; ulp seed 6; ulp seed 7 |
| CAREFUL | 2 | 0 | 2 | 0 | 0 | lang 2/0 | lang seed 2; lang seed 22 |
| CHORDS | 301 | 0 | 301 | 0 | 0 | hair-under-notched-top 294/0, lang 4/0, meta ulp 1/0, rel 2/0 | lang seed 2; lang seed 22; near_tolerance/20; ulp adv near_tolerance 2; ulp seed 0; ulp seed 1; ulp seed 2; ulp seed 3; ulp seed 4; ulp seed 5; ulp seed 6; ulp seed 7 |
| NO_RUN_ON | 297 | 2 | 295 | 0 | 0 | hair-under-notched-top 294/0, meta ulp 1/0, rel 2/2 | near_tolerance/20; ulp adv near_tolerance 2; ulp seed 0; ulp seed 1; ulp seed 2; ulp seed 3; ulp seed 4; ulp seed 5; ulp seed 6; ulp seed 7; ulp seed 8; ulp seed 9 |
| RETRIED result check | 2 | 2 | 0 | 0 | 0 | rel 2/2 | near_tolerance/20 |
| n-ary -> sequential: ProbeOnToolEdge | 1 | 0 | 0 | 1 | 0 | rel 1/0 | coincident_walls/1 |

## Retry rungs by family

| rung | family | tries | ok | internal | other | unknown |
|---|---|---:|---:|---:|---:|---:|
| ABSORB_HAIR_ROOTS | hair-under-notched-top ulp | 294 | 0 | 294 | 0 | 0 |
| ABSORB_HAIR_ROOTS | lang fuzz | 4 | 0 | 4 | 0 | 0 |
| ABSORB_HAIR_ROOTS | meta ulp near_tolerance | 1 | 0 | 1 | 0 | 0 |
| ABSORB_HAIR_ROOTS | rel near_tolerance | 2 | 0 | 2 | 0 | 0 |
| CAREFUL | lang fuzz | 2 | 0 | 2 | 0 | 0 |
| CHORDS | hair-under-notched-top ulp | 294 | 0 | 294 | 0 | 0 |
| CHORDS | lang fuzz | 4 | 0 | 4 | 0 | 0 |
| CHORDS | meta ulp near_tolerance | 1 | 0 | 1 | 0 | 0 |
| CHORDS | rel near_tolerance | 2 | 0 | 2 | 0 | 0 |
| NO_RUN_ON | hair-under-notched-top ulp | 294 | 0 | 294 | 0 | 0 |
| NO_RUN_ON | meta ulp near_tolerance | 1 | 0 | 1 | 0 | 0 |
| NO_RUN_ON | rel near_tolerance | 2 | 2 | 0 | 0 | 0 |
| RETRIED result check | rel near_tolerance | 2 | 2 | 0 | 0 | 0 |
| n-ary -> sequential: ProbeOnToolEdge | rel coincident_walls | 1 | 0 | 0 | 1 | 0 |

## Inputs

| family | runs | failed (error, invalid, finding) | panicked | plans with a contradiction |
|---|---:|---:|---:|---:|
| bench bracket | 1 | 0 | 0 | 0 |
| bench clamp | 1 | 0 | 0 | 0 |
| bench conrod | 1 | 0 | 0 | 0 |
| bench crank | 1 | 0 | 0 | 0 |
| bench frame | 1 | 0 | 0 | 8 |
| bench gearbox | 1 | 0 | 0 | 5 |
| bench guard | 1 | 0 | 0 | 0 |
| bench heatsink | 1 | 0 | 0 | 0 |
| bench hub | 1 | 0 | 0 | 0 |
| bench insert | 1 | 0 | 0 | 0 |
| bench jack | 1 | 0 | 0 | 2 |
| bench manifold | 1 | 0 | 0 | 4 |
| bench quadarm | 1 | 0 | 0 | 6 |
| bench valve | 1 | 0 | 0 | 0 |
| bench volute | 1 | 0 | 0 | 0 |
| hair-under-notched-top ulp | 400 | 316 | 0 | 1267 |
| lang fuzz | 34 | 2 | 0 | 19 |
| meta backends axis_revolve | 6 | 0 | 0 | 0 |
| meta backends ball_poles | 6 | 0 | 0 | 75 |
| meta backends coaxial | 6 | 0 | 0 | 5 |
| meta backends coincident_walls | 6 | 0 | 0 | 0 |
| meta backends coplanar_stack | 6 | 0 | 0 | 5 |
| meta backends degen_cone_cone | 6 | 0 | 0 | 0 |
| meta backends degen_cone_sphere | 6 | 0 | 0 | 0 |
| meta backends degen_cone_torus | 6 | 0 | 0 | 0 |
| meta backends degen_cylinder_cone | 6 | 0 | 0 | 0 |
| meta backends degen_cylinder_cylinder | 6 | 0 | 0 | 0 |
| meta backends degen_cylinder_sphere | 6 | 0 | 0 | 0 |
| meta backends degen_cylinder_torus | 6 | 0 | 0 | 0 |
| meta backends degen_plane_cone | 6 | 0 | 0 | 0 |
| meta backends degen_plane_cylinder | 6 | 0 | 0 | 5 |
| meta backends degen_plane_plane | 6 | 0 | 0 | 0 |
| meta backends degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta backends degen_plane_torus | 6 | 0 | 0 | 0 |
| meta backends degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta backends degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta backends degen_torus_torus | 6 | 0 | 0 | 0 |
| meta backends disjoint | 6 | 0 | 0 | 0 |
| meta backends ellipse_walls | 6 | 0 | 0 | 0 |
| meta backends enclosing | 6 | 0 | 0 | 0 |
| meta backends exact_patterns | 6 | 0 | 0 | 5 |
| meta backends fuzz | 150 | 1 | 0 | 10 |
| meta backends loft_ends | 6 | 0 | 0 | 0 |
| meta backends near_tolerance | 6 | 0 | 0 | 0 |
| meta backends remove_all | 6 | 0 | 0 | 0 |
| meta backends repeated | 6 | 0 | 0 | 5 |
| meta backends rounded_everywhere | 6 | 0 | 0 | 0 |
| meta backends slivers | 6 | 0 | 0 | 0 |
| meta backends spline_revolve | 6 | 0 | 0 | 0 |
| meta backends spline_sweep | 6 | 0 | 0 | 0 |
| meta backends spline_tangent | 6 | 0 | 0 | 0 |
| meta backends spline_walls | 6 | 0 | 0 | 0 |
| meta backends suite | 16 | 3 | 0 | 45 |
| meta backends tangent | 6 | 0 | 0 | 0 |
| meta backends through_vertex | 6 | 0 | 0 | 0 |
| meta backends tilted | 6 | 0 | 0 | 0 |
| meta boolean axis_revolve | 6 | 0 | 0 | 1 |
| meta boolean ball_poles | 6 | 0 | 0 | 60 |
| meta boolean coaxial | 6 | 0 | 0 | 4 |
| meta boolean coincident_walls | 6 | 0 | 0 | 0 |
| meta boolean coplanar_stack | 6 | 0 | 0 | 4 |
| meta boolean degen_cone_cone | 6 | 0 | 0 | 0 |
| meta boolean degen_cone_sphere | 6 | 0 | 0 | 0 |
| meta boolean degen_cone_torus | 6 | 0 | 0 | 0 |
| meta boolean degen_cylinder_cone | 6 | 0 | 0 | 0 |
| meta boolean degen_cylinder_cylinder | 6 | 0 | 0 | 0 |
| meta boolean degen_cylinder_sphere | 6 | 0 | 0 | 0 |
| meta boolean degen_cylinder_torus | 6 | 0 | 0 | 0 |
| meta boolean degen_plane_cone | 6 | 0 | 0 | 0 |
| meta boolean degen_plane_cylinder | 6 | 0 | 0 | 4 |
| meta boolean degen_plane_plane | 6 | 0 | 0 | 0 |
| meta boolean degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta boolean degen_plane_torus | 6 | 0 | 0 | 0 |
| meta boolean degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta boolean degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta boolean degen_torus_torus | 6 | 0 | 0 | 0 |
| meta boolean disjoint | 6 | 0 | 0 | 1 |
| meta boolean ellipse_walls | 6 | 0 | 0 | 0 |
| meta boolean enclosing | 6 | 0 | 0 | 0 |
| meta boolean exact_patterns | 6 | 0 | 0 | 5 |
| meta boolean fuzz | 150 | 0 | 0 | 23 |
| meta boolean loft_ends | 6 | 0 | 0 | 0 |
| meta boolean near_tolerance | 6 | 0 | 0 | 0 |
| meta boolean remove_all | 6 | 0 | 0 | 0 |
| meta boolean repeated | 6 | 0 | 0 | 4 |
| meta boolean rounded_everywhere | 6 | 0 | 0 | 0 |
| meta boolean slivers | 6 | 0 | 0 | 0 |
| meta boolean spline_revolve | 6 | 0 | 0 | 0 |
| meta boolean spline_sweep | 6 | 0 | 0 | 0 |
| meta boolean spline_tangent | 6 | 0 | 0 | 0 |
| meta boolean spline_walls | 6 | 0 | 0 | 0 |
| meta boolean suite | 16 | 0 | 0 | 124 |
| meta boolean tangent | 6 | 0 | 0 | 0 |
| meta boolean through_vertex | 6 | 0 | 0 | 0 |
| meta boolean tilted | 6 | 0 | 0 | 0 |
| meta certify axis_revolve | 6 | 0 | 0 | 0 |
| meta certify ball_poles | 6 | 0 | 0 | 15 |
| meta certify coaxial | 6 | 0 | 0 | 1 |
| meta certify coincident_walls | 6 | 0 | 0 | 0 |
| meta certify coplanar_stack | 6 | 0 | 0 | 1 |
| meta certify degen_cone_cone | 6 | 0 | 0 | 0 |
| meta certify degen_cone_sphere | 6 | 0 | 0 | 0 |
| meta certify degen_cone_torus | 6 | 0 | 0 | 0 |
| meta certify degen_cylinder_cone | 6 | 0 | 0 | 0 |
| meta certify degen_cylinder_cylinder | 6 | 0 | 0 | 0 |
| meta certify degen_cylinder_sphere | 6 | 0 | 0 | 0 |
| meta certify degen_cylinder_torus | 6 | 0 | 0 | 0 |
| meta certify degen_plane_cone | 6 | 0 | 0 | 0 |
| meta certify degen_plane_cylinder | 6 | 0 | 0 | 1 |
| meta certify degen_plane_plane | 6 | 0 | 0 | 0 |
| meta certify degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta certify degen_plane_torus | 6 | 0 | 0 | 0 |
| meta certify degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta certify degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta certify degen_torus_torus | 6 | 0 | 0 | 0 |
| meta certify disjoint | 6 | 0 | 0 | 0 |
| meta certify ellipse_walls | 6 | 0 | 0 | 0 |
| meta certify enclosing | 6 | 0 | 0 | 0 |
| meta certify exact_patterns | 6 | 0 | 0 | 5 |
| meta certify fuzz | 150 | 0 | 0 | 2 |
| meta certify loft_ends | 6 | 0 | 0 | 0 |
| meta certify near_tolerance | 6 | 0 | 0 | 0 |
| meta certify remove_all | 6 | 0 | 0 | 0 |
| meta certify repeated | 6 | 0 | 0 | 1 |
| meta certify rounded_everywhere | 6 | 0 | 0 | 0 |
| meta certify slivers | 6 | 0 | 0 | 0 |
| meta certify spline_revolve | 6 | 0 | 0 | 0 |
| meta certify spline_sweep | 6 | 0 | 0 | 0 |
| meta certify spline_tangent | 6 | 0 | 0 | 0 |
| meta certify spline_walls | 6 | 0 | 0 | 0 |
| meta certify suite | 16 | 0 | 0 | 31 |
| meta certify tangent | 6 | 0 | 0 | 0 |
| meta certify through_vertex | 6 | 0 | 0 | 0 |
| meta certify tilted | 6 | 0 | 0 | 0 |
| meta mirror axis_revolve | 6 | 0 | 0 | 0 |
| meta mirror ball_poles | 6 | 0 | 0 | 24 |
| meta mirror coaxial | 6 | 0 | 0 | 2 |
| meta mirror coincident_walls | 6 | 0 | 0 | 0 |
| meta mirror coplanar_stack | 6 | 0 | 0 | 1 |
| meta mirror degen_cone_cone | 6 | 0 | 0 | 0 |
| meta mirror degen_cone_sphere | 6 | 0 | 0 | 0 |
| meta mirror degen_cone_torus | 6 | 0 | 0 | 0 |
| meta mirror degen_cylinder_cone | 6 | 0 | 0 | 0 |
| meta mirror degen_cylinder_cylinder | 6 | 0 | 0 | 0 |
| meta mirror degen_cylinder_sphere | 6 | 0 | 0 | 0 |
| meta mirror degen_cylinder_torus | 6 | 0 | 0 | 0 |
| meta mirror degen_plane_cone | 6 | 0 | 0 | 0 |
| meta mirror degen_plane_cylinder | 6 | 0 | 0 | 2 |
| meta mirror degen_plane_plane | 6 | 0 | 0 | 0 |
| meta mirror degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta mirror degen_plane_torus | 6 | 0 | 0 | 0 |
| meta mirror degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta mirror degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta mirror degen_torus_torus | 6 | 0 | 0 | 0 |
| meta mirror disjoint | 6 | 0 | 0 | 0 |
| meta mirror ellipse_walls | 6 | 0 | 0 | 0 |
| meta mirror enclosing | 6 | 0 | 0 | 0 |
| meta mirror exact_patterns | 6 | 0 | 0 | 5 |
| meta mirror fuzz | 150 | 11 | 0 | 2 |
| meta mirror loft_ends | 6 | 0 | 0 | 0 |
| meta mirror near_tolerance | 6 | 0 | 0 | 0 |
| meta mirror remove_all | 6 | 0 | 0 | 0 |
| meta mirror repeated | 6 | 0 | 0 | 2 |
| meta mirror rounded_everywhere | 6 | 0 | 0 | 0 |
| meta mirror slivers | 6 | 0 | 0 | 0 |
| meta mirror spline_revolve | 6 | 0 | 0 | 0 |
| meta mirror spline_sweep | 6 | 0 | 0 | 0 |
| meta mirror spline_tangent | 6 | 0 | 0 | 0 |
| meta mirror spline_walls | 6 | 0 | 0 | 0 |
| meta mirror suite | 16 | 0 | 0 | 60 |
| meta mirror tangent | 6 | 0 | 0 | 0 |
| meta mirror through_vertex | 6 | 0 | 0 | 0 |
| meta mirror tilted | 6 | 1 | 0 | 0 |
| meta order axis_revolve | 6 | 0 | 0 | 0 |
| meta order ball_poles | 6 | 0 | 0 | 26 |
| meta order coaxial | 6 | 0 | 0 | 3 |
| meta order coincident_walls | 6 | 0 | 0 | 0 |
| meta order coplanar_stack | 6 | 0 | 0 | 2 |
| meta order degen_cone_cone | 6 | 0 | 0 | 0 |
| meta order degen_cone_sphere | 6 | 0 | 0 | 0 |
| meta order degen_cone_torus | 6 | 0 | 0 | 0 |
| meta order degen_cylinder_cone | 6 | 0 | 0 | 0 |
| meta order degen_cylinder_cylinder | 6 | 0 | 0 | 0 |
| meta order degen_cylinder_sphere | 6 | 0 | 0 | 0 |
| meta order degen_cylinder_torus | 6 | 0 | 0 | 0 |
| meta order degen_plane_cone | 6 | 0 | 0 | 0 |
| meta order degen_plane_cylinder | 6 | 0 | 0 | 0 |
| meta order degen_plane_plane | 6 | 0 | 0 | 0 |
| meta order degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta order degen_plane_torus | 6 | 0 | 0 | 0 |
| meta order degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta order degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta order degen_torus_torus | 6 | 0 | 0 | 0 |
| meta order disjoint | 6 | 0 | 0 | 0 |
| meta order ellipse_walls | 6 | 0 | 0 | 0 |
| meta order enclosing | 6 | 0 | 0 | 0 |
| meta order exact_patterns | 6 | 0 | 0 | 5 |
| meta order fuzz | 150 | 0 | 0 | 4 |
| meta order loft_ends | 6 | 0 | 0 | 0 |
| meta order near_tolerance | 6 | 0 | 0 | 0 |
| meta order remove_all | 6 | 0 | 0 | 0 |
| meta order repeated | 6 | 0 | 0 | 2 |
| meta order rounded_everywhere | 6 | 0 | 0 | 0 |
| meta order slivers | 6 | 0 | 0 | 0 |
| meta order spline_revolve | 6 | 0 | 0 | 1 |
| meta order spline_sweep | 6 | 0 | 0 | 0 |
| meta order spline_tangent | 6 | 0 | 0 | 0 |
| meta order spline_walls | 6 | 0 | 0 | 0 |
| meta order suite | 16 | 0 | 0 | 54 |
| meta order tangent | 6 | 0 | 0 | 0 |
| meta order through_vertex | 6 | 0 | 0 | 0 |
| meta order tilted | 6 | 0 | 0 | 0 |
| meta rigid axis_revolve | 6 | 0 | 0 | 0 |
| meta rigid ball_poles | 6 | 0 | 0 | 15 |
| meta rigid coaxial | 6 | 0 | 0 | 2 |
| meta rigid coincident_walls | 6 | 0 | 0 | 0 |
| meta rigid coplanar_stack | 6 | 0 | 0 | 2 |
| meta rigid degen_cone_cone | 6 | 0 | 0 | 0 |
| meta rigid degen_cone_sphere | 6 | 0 | 0 | 0 |
| meta rigid degen_cone_torus | 6 | 0 | 0 | 0 |
| meta rigid degen_cylinder_cone | 6 | 0 | 0 | 0 |
| meta rigid degen_cylinder_cylinder | 6 | 0 | 0 | 0 |
| meta rigid degen_cylinder_sphere | 6 | 0 | 0 | 0 |
| meta rigid degen_cylinder_torus | 6 | 0 | 0 | 0 |
| meta rigid degen_plane_cone | 6 | 0 | 0 | 0 |
| meta rigid degen_plane_cylinder | 6 | 0 | 0 | 3 |
| meta rigid degen_plane_plane | 6 | 0 | 0 | 0 |
| meta rigid degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta rigid degen_plane_torus | 6 | 0 | 0 | 0 |
| meta rigid degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta rigid degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta rigid degen_torus_torus | 6 | 0 | 0 | 0 |
| meta rigid disjoint | 6 | 0 | 0 | 0 |
| meta rigid ellipse_walls | 6 | 0 | 0 | 0 |
| meta rigid enclosing | 6 | 0 | 0 | 0 |
| meta rigid exact_patterns | 6 | 0 | 0 | 5 |
| meta rigid fuzz | 150 | 0 | 0 | 4 |
| meta rigid loft_ends | 6 | 0 | 0 | 0 |
| meta rigid near_tolerance | 6 | 0 | 0 | 0 |
| meta rigid remove_all | 6 | 0 | 0 | 0 |
| meta rigid repeated | 6 | 0 | 0 | 3 |
| meta rigid rounded_everywhere | 6 | 0 | 0 | 0 |
| meta rigid slivers | 6 | 0 | 0 | 0 |
| meta rigid spline_revolve | 6 | 0 | 0 | 0 |
| meta rigid spline_sweep | 6 | 0 | 0 | 0 |
| meta rigid spline_tangent | 6 | 0 | 0 | 0 |
| meta rigid spline_walls | 6 | 0 | 0 | 0 |
| meta rigid suite | 16 | 0 | 0 | 57 |
| meta rigid tangent | 6 | 0 | 0 | 0 |
| meta rigid through_vertex | 6 | 0 | 0 | 0 |
| meta rigid tilted | 6 | 0 | 0 | 0 |
| meta ulp axis_revolve | 6 | 0 | 0 | 3 |
| meta ulp ball_poles | 6 | 0 | 0 | 52 |
| meta ulp coaxial | 6 | 0 | 0 | 36 |
| meta ulp coincident_walls | 6 | 0 | 0 | 2 |
| meta ulp coplanar_stack | 6 | 0 | 0 | 7 |
| meta ulp degen_cone_cone | 6 | 0 | 0 | 0 |
| meta ulp degen_cone_sphere | 6 | 0 | 0 | 0 |
| meta ulp degen_cone_torus | 6 | 0 | 0 | 0 |
| meta ulp degen_cylinder_cone | 6 | 0 | 0 | 0 |
| meta ulp degen_cylinder_cylinder | 6 | 0 | 0 | 0 |
| meta ulp degen_cylinder_sphere | 6 | 0 | 0 | 0 |
| meta ulp degen_cylinder_torus | 6 | 0 | 0 | 0 |
| meta ulp degen_plane_cone | 6 | 0 | 0 | 0 |
| meta ulp degen_plane_cylinder | 6 | 0 | 0 | 2 |
| meta ulp degen_plane_plane | 6 | 0 | 0 | 0 |
| meta ulp degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta ulp degen_plane_torus | 6 | 0 | 0 | 0 |
| meta ulp degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta ulp degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta ulp degen_torus_torus | 6 | 0 | 0 | 0 |
| meta ulp disjoint | 6 | 0 | 0 | 0 |
| meta ulp ellipse_walls | 6 | 0 | 0 | 0 |
| meta ulp enclosing | 6 | 0 | 0 | 0 |
| meta ulp exact_patterns | 6 | 0 | 0 | 5 |
| meta ulp fuzz | 150 | 0 | 0 | 57 |
| meta ulp loft_ends | 6 | 0 | 0 | 0 |
| meta ulp near_tolerance | 6 | 1 | 0 | 10 |
| meta ulp remove_all | 6 | 0 | 0 | 0 |
| meta ulp repeated | 6 | 0 | 0 | 8 |
| meta ulp rounded_everywhere | 6 | 0 | 0 | 0 |
| meta ulp slivers | 6 | 0 | 0 | 0 |
| meta ulp spline_revolve | 6 | 0 | 0 | 0 |
| meta ulp spline_sweep | 6 | 0 | 0 | 0 |
| meta ulp spline_tangent | 6 | 0 | 0 | 0 |
| meta ulp spline_walls | 6 | 0 | 0 | 0 |
| meta ulp suite | 16 | 0 | 0 | 361 |
| meta ulp tangent | 6 | 0 | 0 | 0 |
| meta ulp through_vertex | 6 | 0 | 0 | 0 |
| meta ulp tilted | 6 | 0 | 0 | 0 |
| rel axis_revolve | 24 | 0 | 0 | 0 |
| rel ball_poles | 24 | 3 | 0 | 106 |
| rel coaxial | 24 | 0 | 0 | 8 |
| rel coincident_walls | 24 | 10 | 0 | 0 |
| rel coplanar_stack | 24 | 0 | 0 | 2 |
| rel degen_cone_cone | 24 | 5 | 0 | 0 |
| rel degen_cone_sphere | 15 | 3 | 0 | 0 |
| rel degen_cone_torus | 12 | 3 | 0 | 0 |
| rel degen_cylinder_cone | 24 | 4 | 0 | 7 |
| rel degen_cylinder_cylinder | 51 | 12 | 0 | 13 |
| rel degen_cylinder_sphere | 24 | 9 | 0 | 0 |
| rel degen_cylinder_torus | 21 | 6 | 0 | 0 |
| rel degen_plane_cone | 30 | 6 | 0 | 0 |
| rel degen_plane_cylinder | 48 | 13 | 0 | 44 |
| rel degen_plane_plane | 45 | 12 | 0 | 0 |
| rel degen_plane_sphere | 33 | 6 | 0 | 0 |
| rel degen_plane_torus | 27 | 11 | 0 | 0 |
| rel degen_sphere_sphere | 30 | 4 | 0 | 0 |
| rel degen_sphere_torus | 12 | 2 | 0 | 0 |
| rel degen_torus_torus | 21 | 9 | 0 | 0 |
| rel disjoint | 24 | 0 | 0 | 0 |
| rel ellipse_walls | 24 | 0 | 0 | 0 |
| rel enclosing | 24 | 0 | 0 | 0 |
| rel exact_patterns | 24 | 24 | 0 | 60 |
| rel fuzz | 24 | 0 | 0 | 0 |
| rel loft_ends | 24 | 0 | 0 | 0 |
| rel near_tolerance | 24 | 1 | 0 | 24 |
| rel remove_all | 24 | 24 | 0 | 0 |
| rel repeated | 24 | 0 | 0 | 2 |
| rel rounded_everywhere | 24 | 0 | 0 | 0 |
| rel slivers | 24 | 0 | 0 | 0 |
| rel spline_revolve | 24 | 0 | 0 | 0 |
| rel spline_sweep | 24 | 0 | 0 | 0 |
| rel spline_tangent | 24 | 2 | 0 | 0 |
| rel spline_walls | 24 | 0 | 0 | 0 |
| rel suite | 16 | 0 | 0 | 34 |
| rel tangent | 24 | 24 | 0 | 0 |
| rel through_vertex | 24 | 0 | 0 | 0 |
| rel tilted | 24 | 0 | 0 | 0 |
| suite fuzz80 | 150 | 0 | 0 | 20 |

## Decisions per site

| fact | site | decisions |
|---|---|---:|
| point on edge | `coincide: an edge's end on the other` | 156428 |
| point on edge | `on_edge <- boolean.rs:2441` | 375664 |
| point on edge | `on_edge <- boolean.rs:2683` | 5234 |
| point on edge | `on_edge <- boolean.rs:2704` | 9165 |
| point on edge | `on_edge <- boolean.rs:2751` | 2243896 |
| point on edge | `on_edge <- boolean.rs:2787` | 372 |
| point on edge | `on_edge <- boolean.rs:2890` | 10442 |
| point on edge | `on_edge <- boolean.rs:3030` | 1920 |
| point on edge | `on_edge <- boolean.rs:3192` | 168 |
| point on edge | `on_edge <- boolean.rs:3231` | 30432 |
| point on edge | `on_edge <- boolean.rs:3243` | 35575 |
| point on edge | `on_edge <- boolean.rs:3244` | 35479 |
| point on edge | `on_edge <- boolean.rs:3245` | 35479 |
| point on edge | `on_edge <- boolean.rs:4380` | 117591 |
| point on edge | `on_edge <- boolean.rs:4726` | 149 |
| point on edge | `on_edge <- nary.rs:1226` | 8856 |
| point on edge | `point_in_face: nearest edge` | 232097 |
| point at vertex | `imprint: an edge's end at the other's` | 15978 |
| point at vertex | `n-ary place: a cut at its edge's end` | 104945 |
| point at vertex | `on_edge <- boolean.rs:2441` | 65 |
| point at vertex | `on_edge <- boolean.rs:2751` | 125 |
| point at vertex | `on_edge <- boolean.rs:4380` | 8 |
| point at vertex | `place_cuts: a cut at its edge's end` | 565624 |
| point at vertex | `share_cuts: a cut at an end` | 497448 |
| point at vertex | `trim: a touch at the edge's corner` | 3460 |
| vertex on surface | `coincident faces => their vertices (derived)` | 2324068 |
| vertex on surface | `trim run-on: the edge's end on the curve` | 240373 |
| vertex on surface | `trim: edge box misses the curve's` | 9868 |
| curve along edge | `trim run-on: line along a straight edge` | 86051 |
| curve along edge | `trim run-on: the curve on the surface across` | 120905 |
| two cuts one vertex | `n-ary place: clusters` | 2013191 |
| two cuts one vertex | `n-ary place: one reach` | 2013191 |
| two cuts one vertex | `place_cuts: clusters and chains` | 13019987 |
| two cuts one vertex | `trim shallow: within 1.2 tolerances, left` | 71496 |
| two cuts one vertex | `trim: a curve's cuts deduplicated` | 216040 |
| two faces one surface | `n-ary pair: closed_or_flush` | 12462 |
| two faces one surface | `pair: surfaces_in` | 21043 |
| two faces one surface | `trim run-on: the face across the edge` | 115615 |
| two faces one surface | `unify: SAME (half LINEAR)` | 37228 |
| two faces one surface | `unify: ssi::coincident (the boolean's test)` | 37228 |
| touch or crossing | `crossings_near` | 20150 |
| touch or crossing | `curve_surface: a touching root` | 11461 |
| touch or crossing | `curve_surface: hair roots (forge-geom)` | 8689 |
| edge on face | `check: edge_off_face, 16 points` | 468340 |
