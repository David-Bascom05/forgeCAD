# Today's flat path: how often a proven screen would be undecided, per guess site

The master asked for this on 2026-11-23: rates first, before any guess site of today's flat
boolean is swapped for a proven f64 screen. The site list is the Lean coordinator's §G
(old-path-statements.md on claude/repo-review-mbdrlj-decide-once-lean).

The method is curved Rust's (curved-fallback-rates.md on claude/repo-review-mbdrlj-dol-curved-rust),
so the two tables compare directly:
- **The decision log.** Feature `decisions`, with `FORGE_DECISIONS_DUMP=1` (curved Rust's dump,
  applied locally) and `FORGE_DECISIONS_TRACE=1` for the retry ladder's rungs. Each logged
  tolerance decision lies within 100·LINEAR of its threshold.
- **Buckets by |measure|.** ≤ 1e-12 is a coincidence: zero in exact terms, decided by
  provenance/keys, never by a sign screen. 1e-12 … 1e-9 is float noise of one fact computed two
  ways. Above 1e-9 and within the limit is a **real near-miss**, where a screen decides it but its
  answer may differ from today's LINEAR answer.
- **Temporary stderr probes** for the sites the log does not record: Index::classify's two `On`
  branches, its ray fallbacks, UNDECIDED (G4), pinched (G6), refuse_strays (G7) and n-ary verdict 3
  (G10). They were measured and then removed, so nothing is committed in the kernel.
- **Runs:** `regen1 PART MODE` on coplanar, upto, upto-flat, hole-grid, tour, torture, mixed and
  shaft (plus plate, patterns and housing for the probes), and `fuzz1`–`fuzz40` seq. Branch at
  f8fce68, release, switch off (today's path only).

## Decision log: decisions in band

| run | in band | ≤1e-12 | 1e-12…1e-9 | 1e-9…limit | plans | plans with a ≤1e-9 decision | plans with a near-miss |
|---|---|---|---|---|---|---|---|
| coplanar seq | 74,538 | 74,444 | 19 | **0** | 421 | 211 | **0** |
| coplanar nary1 | 48,512 | 48,128 | 288 | **0** | 3 | 2 | **0** |
| upto seq | 2,808 | 2,772 | 0 | **0** | 61 | 36 | **0** |
| upto nary1 | 1,483 | 1,467 | 0 | **0** | 27 | 14 | **0** |
| upto-flat seq | 1,820 | 1,800 | 0 | **0** | 37 | 18 | **0** |
| upto-flat nary1 | 1,112 | 1,100 | 0 | **0** | 3 | 2 | **0** |
| hole-grid seq | 14,799 | 14,399 | 0 | **0** | 5 | 3 | **0** |
| hole-grid nary1 | 14,004 | 13,604 | 0 | **0** | 3 | 2 | **0** |
| tour seq | 22,480 | 22,465 | 0 | **0** | 18 | 10 | **0** |
| tour nary1 | 22,424 | 22,410 | 0 | **0** | 8 | 5 | **0** |
| torture seq | 5,224 | 5,207 | 4 | **0** | 441 | 221 | **0** |
| torture nary1 | 6,588 | 6,498 | 78 | **0** | 3 | 2 | **0** |
| shaft seq | 7,683 | 7,406 | 236 | **1** | 349 | 174 | **1** |
| shaft nary1 | 11,493 | 11,316 | 121 | **0** | 3 | 2 | **0** |
| mixed seq | 22,565 | 21,489 | 861 | **44** | 401 | 201 | **8** |
| mixed nary1 | 28,354 | 28,161 | 149 | **1** | 3 | 2 | **1** |
| fuzz 1–40 seq | 121,126 | 120,460 | 477 | **58** | 6,440 | 2,482 | **16** |

The rows don't add up to the total because of decisions past their limit and decisions with no
measure. The fuzz row matches curved Rust's, since it is the same corpus.

Gate parts, summed over all modes, by site:

| fact | site | ≤1e-12 | 1e-12…1e-9 | near-miss | past limit |
|---|---|---|---|---|---|
| vertex on surface | coincident faces ⇒ their vertices (derived) | 73,375 | 27 | 0 | 0 |
| two cuts, one vertex | n-ary place: one reach / clusters | 54,008 (each) | 278 | 0 | 6 |
| point on edge | on_edge ← B:2920 (shared with curved; theirs) | 40,252 | 878 | **44** | 4 |
| two cuts, one vertex | place_cuts: clusters and chains (curved's) | 18,699 | 282 | 1 | 0 |
| point on edge | on_edge ← B:2602 | 8,968 | 0 | 0 | 2 |
| point at vertex | n-ary place: a cut at its edge's end | 6,689 | 0 | 0 | 0 |
| vertex on surface | trim run-on: the edge's end on the curve | 4,154 | 2 | 0 | 0 |
| point on edge | **point_in_face: nearest edge** (classify.rs, mine) | 3,831 | 4 | 1 | 91 |
| two cuts, one vertex | trim: a curve's cuts deduplicated / shallow | 7,172 | 1 | 0 | 0 |
| point at vertex | place_cuts: a cut at its edge's end | 2,203 | 1 | 0 | 1 |
| two faces, one surface | trim run-on, n-ary closed_or_flush, unify SAME / coincident, surfaces_in | 5,096 | 0 | 0 | 0 |
| curve along edge | trim run-on: the curve on the surface across | 844 | 0 | 0 | 1,255 |
| point on edge | on_edge ← B:3212/3374/3413/3425–3427/4562, nary:1260, coincide | 3,849 | 5 | 0 | 1 |
| touch or crossing | crossings_near / curve_surface (curved) | 108 | 0 | 0 | 426 (no measure) |

## The §G sites the log does not record (probes)

| site | coplanar | upto | upto-flat | hole-grid | tour | torture | shaft | mixed | plate | patterns | housing | fuzz 1–40 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **G2/G3** retry ladder: rungs fired (TRACE) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | — | — | — | **0** |
| **G4** UNDECIDED touch: passed / refused | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | **0 / 0** |
| **G6** pinched_new / pinched_old fired | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | **0** |
| **G7** refuse_strays refused | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | **0** |
| **G10** n-ary verdict 3 (undecided), nary1 | 923 | 40 | 32 | 800 | 24 | 35 | 4 | 115 | 35 | 218 | 189 | — |
| classify: `On`, within LINEAR of a face (seq + nary1) | 1,346 | 40 | 32 | 1,598 | 48 | 6 | 3 | 130 | 6 | 218 | 208 | 0 |
| classify: `On`, all rays degenerate | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | **0** |
| classify: a ray rejected (on surface / touching / hit on a boundary) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| point_in_face: `On` near an edge (seq + nary1) | 1,585 | 126 | 92 | 1,600 | 135 | 19 | 15 | 264 | 19 | 1,252 | 439 | 56 |

How far each `On` actually was:
- classify `On`: 3,622 at exactly 0, 13 at ≤ 1e-12, none above.
- point_in_face `On`: 4,411 at exactly 0, 1,176 at ≤ 1e-12, 6 at ≤ 1e-9, and **9 above 1e-9**:
  - shaft seq: 1, at 9.0e-6;
  - fuzz20: 3, at 7.1e-6;
  - fuzz30: 1, at 8.2e-6;
  - fuzz40: 4, at 1.9e-6.

## What this says (flat sites)

1. **The ladder, UNDECIDED, pinched and strays (G2–G4, G6, G7) never fire** on the gate parts or
   fuzz 1–40. Replacing them with screen-then-exact changes no result on these inputs. The swap
   is about provability, not behaviour. It should still be listed per site: the repros and L6
   corpus are the next place to count before switching.
2. **classify's `On` is almost all exact coincidence.** In 3,635 `On`s no point was more than
   1e-12 from a face, and the all-rays-degenerate branch never fired. So the exact `On` (§1a) is
   decided by provenance or keys (the point was made on that face, so its exact form is on it).
   The exact on-face test, then a refusal, is reserved for points of no known provenance, and fires
   0 times here.
3. **point_in_face's nearest-edge `On` has 9 real near-misses** (shaft seq: 1, fuzz: 8, distances
   2e-6 to 9e-6). Those are the results the swap may change and must list with the exact
   reference's verdict. The rest are exactly on the edge they were cut from, so provenance decides
   them.
4. **G10, n-ary verdict 3, fires a lot** (up to 923 per run, on every part's nary1). It is the
   n-ary batch's "undecided" point classification. It is today's normal route to the exact local
   test (D6), not a rare event, so it needs its screen and exact test first, and the cost matters.
5. **Most coincidences sit at curved Rust's sites** (on_edge, place_cuts, unify, ssi::coincident),
   shared with flat faces. Per the split agreed with curved Rust, they own those functions. The
   flat-only sites with real near-misses are on_edge ← B:2920 (44, in mixed: theirs) and
   point_in_face (mine).

The next step is the exact `On` swap (G1/§1a), then G10's verdict 3. Per the order, A8 runs as the
fallback after these.
