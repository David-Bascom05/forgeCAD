# Decision log, phase 0: notes (by hand), WIP

Measured on claude/repo-review-mbdrlj at 6979fa5c plus this commit's instrumentation (feature
`decisions`, off by default). Everything below the line is the runner's output, unedited except
for the output path in its command line (`verify/forge-proof/examples/decisions.rs` at nightly
meta depth: 4300 runs, 451 s). The callgrind gate with the feature off has **not** been run to the
end yet (see DECISIONS-HANDOFF.md).

## Site names

`on_edge <- boolean.rs:N` means `on_edge` called from line N of boolean.rs at this commit:

| line | caller |
|---|---|
| 2416 | `trim`: a plane face's curve meets the edge's own line or circle (`plane_meet`) |
| 2717 | `trim`: a crossing (or touch) of the curve with the surface across the edge |
| 3209-3211 | `trim`: a stretch of the curve along one boundary edge (its middle and both ends on it) |
| 4346 | `imprint`: a coplanar face's edge crossing an edge of the other face |

## Headlines

1. **Nearly every contradiction comes from one family: a face a hair under a tolerance from a
   coplanar face.** That is near-coincident.md's part (`hair-under-notched-top ulp`) and
   `rel near_tolerance`. They hold 2316 of the 2321 point-on-edge contradictions and all of the
   vertex-on-surface disagreements except 2. The pairs, most first:
   - Point on edge:
     - trim's crossing (2717) says on, imprint's crossing (4346) says off: 2316 facts, at 1.2e-5
       against a limit of 1e-5;
     - trim's along-stretch (3210) against imprint: 770 facts.
   - Vertex on surface:
     - The pair decided the two planes coincident, so every vertex of each face lies on the other.
       Yet trim's edge-box test (2112 facts) and trim's run-on end test (1157) measure the same
       vertex 1e-5 to 1.35e-5 off and say no.
     - Those two tests also disagree with each other: 372 facts.

   These are exactly the six sites near-coincident.md names. The log keys them as one fact.
2. **A contradiction predicts a lost way.**
   - Sequential plans with a contradiction: 1738, of which 1180 ended `Internal` (68%).
   - Plans without one: 97 858, of which 18 ended `Internal` (0.02%).
   - So 98.5% of all `Internal` plans had a recorded contradiction first.
3. **Two cuts one vertex**: 656 facts, spread over suite, fuzz, lang fuzz, and bench jack and
   frame. Three sites disagree about two cuts at the same point:
   - trim's dedup keeps them apart, because it also asks for the curve's middle between the two
     parameters to lie at the point;
   - place_cuts' clustering merges them;
   - trim's shallow-crossing pass leaves any gap under 1.2 tolerances as one place.

   Mostly this is a curve coming back to a point it passed ("a cut each time", by design). None of
   these plans failed.
4. **Touch or crossing**: 22 facts. curve_surface calls a root touching, then crossings_near finds
   a crossing (degen cylinder_cone apex_on_top_rim, suite). This is a refinement by design, but it
   is still counted.
5. **No disagreement found** on two faces one surface (36k facts, 32k decided at 2+ sites) or curve
   along edge (one site each).

## Retry rungs

| rung | fired | rescued |
|---|---:|---|
| FENCES | 0 | — |
| `bends` (crossing_on_face) | 0 | — |
| ABSORB_HAIR_ROOTS | 301 | none |
| CHORDS | 301 | none |
| CAREFUL | 2 | none |
| NO_RUN_ON | 297 | one input: `near_tolerance/20` (2 tries, both ok; the result passed the RETRIED check) |
| n-ary -> sequential | 1 | none (ProbeOnToolEdge on coincident_walls/1, refused again) |

So on this corpus, deleting CAREFUL, ABSORB_HAIR_ROOTS, CHORDS and FENCES changes no outcome.
NO_RUN_ON holds one part. Caveat: the corpus has no Fusion or WHUCAD parts, and the regression
tests' rung counts were not collected yet (DECISIONS-HANDOFF.md, next steps).

---

# The boolean's decision log: contradictions and retries

`cargo run --release -p forge-proof --features decisions --example decisions -- threads 4 out research/kernel-audit/decisions.md`: 4300 runs in 451 s. Written by that run (the notes above it are by hand).

## Per fact kind

| fact | decisions | facts | facts decided at 2+ sites | contradictions | of which by one site only |
|---|---:|---:|---:|---:|---:|
| point on edge | 3293533 | 1828607 | 346385 | 2321 | 2 |
| point at vertex | 1082708 | 89390 | 15287 | 2 | 2 |
| vertex on surface | 2572687 | 528292 | 150814 | 2945 | 0 |
| curve along edge | 206757 | 199575 | 0 | 0 | 0 |
| two cuts one vertex | 13306786 | 746609 | 157029 | 656 | 9 |
| two faces one surface | 148781 | 35971 | 32551 | 0 | 0 |
| touch or crossing | 22916 | 11276 | 11276 | 22 | 0 |

Decisions recorded outside any plan (an n-ary pass's helper threads): 39142.

## Plans: does a contradiction go with a lost way?

| path | plans | with a contradiction | of those ok / internal / other / unknown | without: ok / internal / other / unknown |
|---|---:|---:|---|---|
| n-ary | 1373 | 40 | 40 / 0 / 0 / 0 | 1232 / 0 / 101 / 0 |
| sequential | 99596 | 1738 | 555 / 1180 / 3 / 0 | 97436 / 18 / 404 / 0 |

## Site pairs that disagree

Per fact kind, the pairs of sites that gave one fact two answers, most facts first.

| fact | site | other site | facts | paths | families |
|---|---|---|---:|---|---|
| point on edge | `on_edge <- boolean.rs:2717` | `on_edge <- boolean.rs:4346` | 2316 | sequential 2316 | hair-under-notched-top ulp 2304, rel near_tolerance 8, meta ulp near_tolerance 4 |
| point on edge | `on_edge <- boolean.rs:3210` | `on_edge <- boolean.rs:4346` | 770 | sequential 770 | hair-under-notched-top ulp 768, rel near_tolerance 2 |
| point on edge | `on_edge <- boolean.rs:2416` | `on_edge <- boolean.rs:4346` | 8 | sequential 8 | rel near_tolerance 8 |
| point on edge | `on_edge <- boolean.rs:2717` | `point_in_face: nearest edge` | 7 | sequential 7 | rel near_tolerance 4, meta ulp near_tolerance 3 |
| point on edge | `on_edge <- boolean.rs:2416` | `point_in_face: nearest edge` | 6 | sequential 6 | rel near_tolerance 4, meta ulp near_tolerance 2 |
| point on edge | `on_edge <- boolean.rs:3211` | `on_edge <- boolean.rs:4346` | 2 | sequential 2 | rel near_tolerance 2 |
| point on edge | `point_in_face: nearest edge` | `point_in_face: nearest edge` | 2 | sequential 2 | rel near_tolerance 2 |
| point at vertex | `place_cuts: a cut at its edge's end` | `place_cuts: a cut at its edge's end` | 2 | sequential 2 | rel near_tolerance 2 |
| vertex on surface | `coincident faces => their vertices (derived)` | `trim: edge box misses the curve's` | 2112 | sequential 2112 | hair-under-notched-top ulp 2104, rel near_tolerance 8 |
| vertex on surface | `coincident faces => their vertices (derived)` | `trim run-on: the edge's end on the curve` | 1157 | sequential 1157 | hair-under-notched-top ulp 1137, rel near_tolerance 12, meta ulp near_tolerance 8 |
| vertex on surface | `trim run-on: the edge's end on the curve` | `trim: edge box misses the curve's` | 372 | sequential 372 | hair-under-notched-top ulp 368, rel near_tolerance 4 |
| vertex on surface | `trim run-on: the edge's end on the curve` | `trim run-on: the edge's end on the curve` | 2 | sequential 2 | rel near_tolerance 2 |
| two cuts one vertex | `trim shallow: within 1.2 tolerances, left` | `trim: a curve's cuts deduplicated` | 521 | sequential 389, n-ary 132 | meta backends suite 103, meta boolean suite 84, meta ulp suite 42, rel suite 42, +29 more |
| two cuts one vertex | `place_cuts: clusters and chains` | `trim: a curve's cuts deduplicated` | 515 | sequential 515 | meta boolean suite 100, meta ulp suite 50, meta mirror suite 47, meta rigid suite 45, +42 more |
| two cuts one vertex | `trim: a curve's cuts deduplicated` | `trim: a curve's cuts deduplicated` | 134 | sequential 117, n-ary 17 | meta boolean suite 16, meta backends suite 13, meta mirror suite 12, meta order suite 8, +26 more |
| touch or crossing | `crossings_near` | `curve_surface: a touching root` | 22 | sequential 21, n-ary 1 | rel degen_cylinder_cone 7, meta boolean suite 4, meta mirror suite 2, meta order suite 2, +5 more |

An example of each (input [path; retry flags] entities and points: each site's answer, its measure vs its limit):

- point on edge / `on_edge <- boolean.rs:2717` vs `on_edge <- boolean.rs:4346`: near_tolerance/8 [sequential; plain] E6 (37.080889, 23.134408, 10.000010): `on_edge <- boolean.rs:2717` says 1 (0.000e0 vs 1.000e-5), `on_edge <- boolean.rs:4346` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `on_edge <- boolean.rs:3210` vs `on_edge <- boolean.rs:4346`: near_tolerance/8 [sequential; plain] E6 (37.080889, 26.895624, 10.000000): `on_edge <- boolean.rs:3210` says 1 (1.000e-5 vs 1.000e-5), `on_edge <- boolean.rs:4346` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `on_edge <- boolean.rs:2416` vs `on_edge <- boolean.rs:4346`: near_tolerance/8 [sequential; plain] E6 (37.080889, 23.134408, 10.000010): `on_edge <- boolean.rs:2416` says 1 (0.000e0 vs 1.000e-5), `on_edge <- boolean.rs:4346` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `on_edge <- boolean.rs:2717` vs `point_in_face: nearest edge`: near_tolerance/8 [sequential; plain] E26 (37.080889, 23.134408, 10.000000): `on_edge <- boolean.rs:2717` says 1 (2.000e-6 vs 1.000e-5), `point_in_face: nearest edge` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `on_edge <- boolean.rs:2416` vs `point_in_face: nearest edge`: near_tolerance/8 [sequential; plain] E26 (37.080889, 23.134408, 10.000000): `on_edge <- boolean.rs:2416` says 1 (2.000e-6 vs 1.000e-5), `point_in_face: nearest edge` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `on_edge <- boolean.rs:3211` vs `on_edge <- boolean.rs:4346`: near_tolerance/8 [sequential; plain] E6 (37.080889, 23.134408, 10.000000): `on_edge <- boolean.rs:3211` says 1 (1.000e-5 vs 1.000e-5), `on_edge <- boolean.rs:4346` says 0 (1.200e-5 vs 1.000e-5)
- point on edge / `point_in_face: nearest edge` vs `point_in_face: nearest edge`: near_tolerance/23 [sequential; plain] E11 (50.000005, 10.189440, 11.000000): `point_in_face: nearest edge` says 0 (1.500e-5 vs 1.000e-5), `point_in_face: nearest edge` says 1 (7.500e-6 vs 1.000e-5)
- point at vertex / `place_cuts: a cut at its edge's end` vs `place_cuts: a cut at its edge's end`: near_tolerance/18 [sequential; plain] V32 (50.000001, 15.985226, 9.999990): `place_cuts: a cut at its edge's end` says 0 (1.345e-5 vs 1.000e-5), `place_cuts: a cut at its edge's end` says 1 (1.000e-5 vs 1.000e-5)
- vertex on surface / `coincident faces => their vertices (derived)` vs `trim: edge box misses the curve's`: near_tolerance/8 [sequential; plain] V16 F0 (37.743516, 25.015016, 9.999998): `coincident faces => their vertices (derived)` says 1 (1.200e-5 vs NaN), `trim: edge box misses the curve's` says 0 (1.200e-5 vs 1.000e-5)
- vertex on surface / `coincident faces => their vertices (derived)` vs `trim run-on: the edge's end on the curve`: near_tolerance/18 [sequential; plain] V32 F0 (50.000010, 15.985226, 10.000000): `coincident faces => their vertices (derived)` says 1 (1.000e-5 vs NaN), `trim run-on: the edge's end on the curve` says 0 (1.345e-5 vs 1.000e-5)
- vertex on surface / `trim run-on: the edge's end on the curve` vs `trim: edge box misses the curve's`: near_tolerance/22 [sequential; plain] V10 F5 (50.000010, 15.722265, 0.000000): `trim run-on: the edge's end on the curve` says 1 (1.000e-5 vs 1.000e-5), `trim: edge box misses the curve's` says 0 (1.000e-5 vs 1.000e-5)
- vertex on surface / `trim run-on: the edge's end on the curve` vs `trim run-on: the edge's end on the curve`: near_tolerance/18 [sequential; plain] V32 F0 (50.000010, 15.985226, 10.000000): `trim run-on: the edge's end on the curve` says 0 (1.345e-5 vs 1.000e-5), `trim run-on: the edge's end on the curve` says 1 (1.000e-5 vs 1.000e-5)
- two cuts one vertex / `trim shallow: within 1.2 tolerances, left` vs `trim: a curve's cuts deduplicated`: coaxial/8 [sequential; plain] E68 E69 (30.057775, 22.925376, -1.000000) (30.057775, 22.925376, -1.000000): `trim shallow: within 1.2 tolerances, left` says 1 (0.000e0 vs 1.000e-5), `trim: a curve's cuts deduplicated` says 0 (0.000e0 vs 1.000e-5)
- two cuts one vertex / `place_cuts: clusters and chains` vs `trim: a curve's cuts deduplicated`: coplanar_stack/5 [sequential; plain] E1 E73 (6.766903, 36.149773, 11.000000) (6.766903, 36.149773, 11.000000): `place_cuts: clusters and chains` says 1 (0.000e0 vs 1.000e-5), `trim: a curve's cuts deduplicated` says 0 (0.000e0 vs 1.000e-5)
- two cuts one vertex / `trim: a curve's cuts deduplicated` vs `trim: a curve's cuts deduplicated`: coplanar_stack/5 [sequential; plain] E1 E73 (6.766903, 36.149773, 11.000000) (6.766903, 36.149773, 11.000000): `trim: a curve's cuts deduplicated` says 0 (0.000e0 vs 1.000e-5), `trim: a curve's cuts deduplicated` says 1 (0.000e0 vs 1.000e-5)
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
| bench frame | 1 | 0 | 0 | 2 |
| bench gearbox | 1 | 0 | 0 | 0 |
| bench guard | 1 | 0 | 0 | 0 |
| bench heatsink | 1 | 0 | 0 | 0 |
| bench hub | 1 | 0 | 0 | 0 |
| bench insert | 1 | 0 | 0 | 0 |
| bench jack | 1 | 0 | 0 | 2 |
| bench manifold | 1 | 0 | 0 | 0 |
| bench quadarm | 1 | 0 | 0 | 0 |
| bench valve | 1 | 0 | 0 | 0 |
| bench volute | 1 | 0 | 0 | 0 |
| hair-under-notched-top ulp | 400 | 316 | 0 | 1267 |
| lang fuzz | 34 | 2 | 0 | 11 |
| meta backends axis_revolve | 6 | 0 | 0 | 0 |
| meta backends ball_poles | 6 | 0 | 0 | 0 |
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
| meta backends degen_plane_cylinder | 6 | 0 | 0 | 0 |
| meta backends degen_plane_plane | 6 | 0 | 0 | 0 |
| meta backends degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta backends degen_plane_torus | 6 | 0 | 0 | 0 |
| meta backends degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta backends degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta backends degen_torus_torus | 6 | 0 | 0 | 0 |
| meta backends disjoint | 6 | 0 | 0 | 0 |
| meta backends ellipse_walls | 6 | 0 | 0 | 0 |
| meta backends enclosing | 6 | 0 | 0 | 0 |
| meta backends exact_patterns | 6 | 0 | 0 | 0 |
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
| meta backends suite | 16 | 3 | 0 | 35 |
| meta backends tangent | 6 | 0 | 0 | 0 |
| meta backends through_vertex | 6 | 0 | 0 | 0 |
| meta backends tilted | 6 | 0 | 0 | 0 |
| meta boolean axis_revolve | 6 | 0 | 0 | 0 |
| meta boolean ball_poles | 6 | 0 | 0 | 0 |
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
| meta boolean degen_plane_cylinder | 6 | 0 | 0 | 0 |
| meta boolean degen_plane_plane | 6 | 0 | 0 | 0 |
| meta boolean degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta boolean degen_plane_torus | 6 | 0 | 0 | 0 |
| meta boolean degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta boolean degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta boolean degen_torus_torus | 6 | 0 | 0 | 0 |
| meta boolean disjoint | 6 | 0 | 0 | 1 |
| meta boolean ellipse_walls | 6 | 0 | 0 | 0 |
| meta boolean enclosing | 6 | 0 | 0 | 0 |
| meta boolean exact_patterns | 6 | 0 | 0 | 0 |
| meta boolean fuzz | 150 | 0 | 0 | 14 |
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
| meta boolean suite | 16 | 0 | 0 | 88 |
| meta boolean tangent | 6 | 0 | 0 | 0 |
| meta boolean through_vertex | 6 | 0 | 0 | 0 |
| meta boolean tilted | 6 | 0 | 0 | 0 |
| meta certify axis_revolve | 6 | 0 | 0 | 0 |
| meta certify ball_poles | 6 | 0 | 0 | 0 |
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
| meta certify degen_plane_cylinder | 6 | 0 | 0 | 0 |
| meta certify degen_plane_plane | 6 | 0 | 0 | 0 |
| meta certify degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta certify degen_plane_torus | 6 | 0 | 0 | 0 |
| meta certify degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta certify degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta certify degen_torus_torus | 6 | 0 | 0 | 0 |
| meta certify disjoint | 6 | 0 | 0 | 0 |
| meta certify ellipse_walls | 6 | 0 | 0 | 0 |
| meta certify enclosing | 6 | 0 | 0 | 0 |
| meta certify exact_patterns | 6 | 0 | 0 | 0 |
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
| meta certify suite | 16 | 0 | 0 | 22 |
| meta certify tangent | 6 | 0 | 0 | 0 |
| meta certify through_vertex | 6 | 0 | 0 | 0 |
| meta certify tilted | 6 | 0 | 0 | 0 |
| meta mirror axis_revolve | 6 | 0 | 0 | 0 |
| meta mirror ball_poles | 6 | 0 | 0 | 0 |
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
| meta mirror degen_plane_cylinder | 6 | 0 | 0 | 0 |
| meta mirror degen_plane_plane | 6 | 0 | 0 | 0 |
| meta mirror degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta mirror degen_plane_torus | 6 | 0 | 0 | 0 |
| meta mirror degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta mirror degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta mirror degen_torus_torus | 6 | 0 | 0 | 0 |
| meta mirror disjoint | 6 | 0 | 0 | 0 |
| meta mirror ellipse_walls | 6 | 0 | 0 | 0 |
| meta mirror enclosing | 6 | 0 | 0 | 0 |
| meta mirror exact_patterns | 6 | 0 | 0 | 0 |
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
| meta mirror suite | 16 | 0 | 0 | 42 |
| meta mirror tangent | 6 | 0 | 0 | 0 |
| meta mirror through_vertex | 6 | 0 | 0 | 0 |
| meta mirror tilted | 6 | 1 | 0 | 0 |
| meta order axis_revolve | 6 | 0 | 0 | 0 |
| meta order ball_poles | 6 | 0 | 0 | 0 |
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
| meta order exact_patterns | 6 | 0 | 0 | 0 |
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
| meta order suite | 16 | 0 | 0 | 36 |
| meta order tangent | 6 | 0 | 0 | 0 |
| meta order through_vertex | 6 | 0 | 0 | 0 |
| meta order tilted | 6 | 0 | 0 | 0 |
| meta rigid axis_revolve | 6 | 0 | 0 | 0 |
| meta rigid ball_poles | 6 | 0 | 0 | 0 |
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
| meta rigid degen_plane_cylinder | 6 | 0 | 0 | 0 |
| meta rigid degen_plane_plane | 6 | 0 | 0 | 0 |
| meta rigid degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta rigid degen_plane_torus | 6 | 0 | 0 | 0 |
| meta rigid degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta rigid degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta rigid degen_torus_torus | 6 | 0 | 0 | 0 |
| meta rigid disjoint | 6 | 0 | 0 | 0 |
| meta rigid ellipse_walls | 6 | 0 | 0 | 0 |
| meta rigid enclosing | 6 | 0 | 0 | 0 |
| meta rigid exact_patterns | 6 | 0 | 0 | 0 |
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
| meta rigid suite | 16 | 0 | 0 | 39 |
| meta rigid tangent | 6 | 0 | 0 | 0 |
| meta rigid through_vertex | 6 | 0 | 0 | 0 |
| meta rigid tilted | 6 | 0 | 0 | 0 |
| meta ulp axis_revolve | 6 | 0 | 0 | 0 |
| meta ulp ball_poles | 6 | 0 | 0 | 0 |
| meta ulp coaxial | 6 | 0 | 0 | 2 |
| meta ulp coincident_walls | 6 | 0 | 0 | 0 |
| meta ulp coplanar_stack | 6 | 0 | 0 | 1 |
| meta ulp degen_cone_cone | 6 | 0 | 0 | 0 |
| meta ulp degen_cone_sphere | 6 | 0 | 0 | 0 |
| meta ulp degen_cone_torus | 6 | 0 | 0 | 0 |
| meta ulp degen_cylinder_cone | 6 | 0 | 0 | 0 |
| meta ulp degen_cylinder_cylinder | 6 | 0 | 0 | 0 |
| meta ulp degen_cylinder_sphere | 6 | 0 | 0 | 0 |
| meta ulp degen_cylinder_torus | 6 | 0 | 0 | 0 |
| meta ulp degen_plane_cone | 6 | 0 | 0 | 0 |
| meta ulp degen_plane_cylinder | 6 | 0 | 0 | 0 |
| meta ulp degen_plane_plane | 6 | 0 | 0 | 0 |
| meta ulp degen_plane_sphere | 6 | 0 | 0 | 0 |
| meta ulp degen_plane_torus | 6 | 0 | 0 | 0 |
| meta ulp degen_sphere_sphere | 6 | 0 | 0 | 0 |
| meta ulp degen_sphere_torus | 6 | 0 | 0 | 0 |
| meta ulp degen_torus_torus | 6 | 0 | 0 | 0 |
| meta ulp disjoint | 6 | 0 | 0 | 0 |
| meta ulp ellipse_walls | 6 | 0 | 0 | 0 |
| meta ulp enclosing | 6 | 0 | 0 | 0 |
| meta ulp exact_patterns | 6 | 0 | 0 | 0 |
| meta ulp fuzz | 150 | 0 | 0 | 4 |
| meta ulp loft_ends | 6 | 0 | 0 | 0 |
| meta ulp near_tolerance | 6 | 1 | 0 | 8 |
| meta ulp remove_all | 6 | 0 | 0 | 0 |
| meta ulp repeated | 6 | 0 | 0 | 2 |
| meta ulp rounded_everywhere | 6 | 0 | 0 | 0 |
| meta ulp slivers | 6 | 0 | 0 | 0 |
| meta ulp spline_revolve | 6 | 0 | 0 | 0 |
| meta ulp spline_sweep | 6 | 0 | 0 | 0 |
| meta ulp spline_tangent | 6 | 0 | 0 | 0 |
| meta ulp spline_walls | 6 | 0 | 0 | 0 |
| meta ulp suite | 16 | 0 | 0 | 44 |
| meta ulp tangent | 6 | 0 | 0 | 0 |
| meta ulp through_vertex | 6 | 0 | 0 | 0 |
| meta ulp tilted | 6 | 0 | 0 | 0 |
| rel axis_revolve | 24 | 0 | 0 | 0 |
| rel ball_poles | 24 | 3 | 0 | 4 |
| rel coaxial | 24 | 0 | 0 | 8 |
| rel coincident_walls | 24 | 10 | 0 | 0 |
| rel coplanar_stack | 24 | 0 | 0 | 2 |
| rel degen_cone_cone | 24 | 5 | 0 | 0 |
| rel degen_cone_sphere | 15 | 3 | 0 | 0 |
| rel degen_cone_torus | 12 | 3 | 0 | 0 |
| rel degen_cylinder_cone | 24 | 4 | 0 | 7 |
| rel degen_cylinder_cylinder | 51 | 12 | 0 | 6 |
| rel degen_cylinder_sphere | 24 | 9 | 0 | 0 |
| rel degen_cylinder_torus | 21 | 6 | 0 | 0 |
| rel degen_plane_cone | 30 | 6 | 0 | 0 |
| rel degen_plane_cylinder | 48 | 13 | 0 | 0 |
| rel degen_plane_plane | 45 | 12 | 0 | 0 |
| rel degen_plane_sphere | 33 | 6 | 0 | 0 |
| rel degen_plane_torus | 27 | 11 | 0 | 0 |
| rel degen_sphere_sphere | 30 | 4 | 0 | 0 |
| rel degen_sphere_torus | 12 | 2 | 0 | 0 |
| rel degen_torus_torus | 21 | 9 | 0 | 0 |
| rel disjoint | 24 | 0 | 0 | 0 |
| rel ellipse_walls | 24 | 0 | 0 | 0 |
| rel enclosing | 24 | 0 | 0 | 0 |
| rel exact_patterns | 24 | 24 | 0 | 0 |
| rel fuzz | 24 | 0 | 0 | 0 |
| rel loft_ends | 24 | 0 | 0 | 0 |
| rel near_tolerance | 24 | 1 | 0 | 12 |
| rel remove_all | 24 | 24 | 0 | 0 |
| rel repeated | 24 | 0 | 0 | 2 |
| rel rounded_everywhere | 24 | 0 | 0 | 0 |
| rel slivers | 24 | 0 | 0 | 0 |
| rel spline_revolve | 24 | 0 | 0 | 0 |
| rel spline_sweep | 24 | 0 | 0 | 0 |
| rel spline_tangent | 24 | 2 | 0 | 0 |
| rel spline_walls | 24 | 0 | 0 | 0 |
| rel suite | 16 | 0 | 0 | 25 |
| rel tangent | 24 | 24 | 0 | 0 |
| rel through_vertex | 24 | 0 | 0 | 0 |
| rel tilted | 24 | 0 | 0 | 0 |
| suite fuzz80 | 150 | 0 | 0 | 20 |

## Decisions per site

| fact | site | decisions |
|---|---|---:|
| point on edge | `coincide: an edge's end on the other` | 156612 |
| point on edge | `on_edge <- boolean.rs:2416` | 374122 |
| point on edge | `on_edge <- boolean.rs:2649` | 5230 |
| point on edge | `on_edge <- boolean.rs:2670` | 9161 |
| point on edge | `on_edge <- boolean.rs:2717` | 2240363 |
| point on edge | `on_edge <- boolean.rs:2753` | 372 |
| point on edge | `on_edge <- boolean.rs:2856` | 10442 |
| point on edge | `on_edge <- boolean.rs:2996` | 1914 |
| point on edge | `on_edge <- boolean.rs:3158` | 164 |
| point on edge | `on_edge <- boolean.rs:3197` | 30464 |
| point on edge | `on_edge <- boolean.rs:3209` | 35473 |
| point on edge | `on_edge <- boolean.rs:3210` | 35377 |
| point on edge | `on_edge <- boolean.rs:3211` | 35377 |
| point on edge | `on_edge <- boolean.rs:4346` | 117469 |
| point on edge | `on_edge <- boolean.rs:4692` | 149 |
| point on edge | `on_edge <- nary.rs:1186` | 8944 |
| point on edge | `point_in_face: nearest edge` | 231900 |
| point at vertex | `imprint: an edge's end at the other's` | 15978 |
| point at vertex | `on_edge <- boolean.rs:2416` | 65 |
| point at vertex | `on_edge <- boolean.rs:2717` | 125 |
| point at vertex | `on_edge <- boolean.rs:4346` | 8 |
| point at vertex | `place_cuts: a cut at its edge's end` | 565624 |
| point at vertex | `share_cuts: a cut at an end` | 497448 |
| point at vertex | `trim: a touch at the edge's corner` | 3460 |
| vertex on surface | `coincident faces => their vertices (derived)` | 2322822 |
| vertex on surface | `trim run-on: the edge's end on the curve` | 239999 |
| vertex on surface | `trim: edge box misses the curve's` | 9866 |
| curve along edge | `trim run-on: line along a straight edge` | 86039 |
| curve along edge | `trim run-on: the curve on the surface across` | 120718 |
| two cuts one vertex | `place_cuts: clusters and chains` | 13019987 |
| two cuts one vertex | `trim shallow: within 1.2 tolerances, left` | 71041 |
| two cuts one vertex | `trim: a curve's cuts deduplicated` | 215758 |
| two faces one surface | `n-ary pair: closed_or_flush` | 12310 |
| two faces one surface | `pair: surfaces_in` | 21043 |
| two faces one surface | `trim run-on: the face across the edge` | 115428 |
| touch or crossing | `crossings_near` | 11458 |
| touch or crossing | `curve_surface: a touching root` | 11458 |
