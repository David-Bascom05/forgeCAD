# Fitting 3D finishing into long lines and arcs

3D finishing makes the classic micro-moves: drop-cutter points every few tenths of a millimetre.
This note records how the finishing ops in forge-cam-3d now fit their passes into long lines and
G2/G3 arcs, what that guarantees, and what it measured on the mould core.

## Method

There is one fitter, `forge_cam::fit`, shared with adaptive. It is the plain greedy method:

- From each piece's start, it looks for the longest run of input points that one line, or one
  circular arc, stays within the chord tolerance of. The arc is in xy, with its height linear in
  the angle (`PMove::Arc`). The run's end is found by doubling, then bisecting.
- Every candidate piece goes to the caller's `accept` hook. In forge-cam-3d that hook proves the
  piece with the drop-cutter, as a chord is proven: a line by `seg3_below`, an arc by the new
  `arc3_below`. `SurfDrop` proves an arc through `ops::checker_moves` and the column gap test.
- A rejected piece backs off to a shorter one. One input segment is always kept as it is, so the
  output never has more moves than the input.
- The op's path is then certified by forge-cam's checker like any other path.

The bound: every kept piece is within the tolerance of the pass it replaces, both ways.
- A line: each vertex's projection is within tol, and the projections advance along the line.
  The distance between two linearly parametrised segments is convex in the parameter, so it is at
  most the larger of the two vertex distances.
- An arc: the same, with each section's sagitta taken off the tolerance.

`fit::deviation` also measures this by sampling, and each op's plan reports the measurement.

## The budget: the path's tolerance, not the cusp

The plan's proven cusp is a bound on the scallop between ideal passes at the op's spacing. It never
included the path's own deviation, which is bounded separately: chords are proven to dip at most
the slack below the CL surface, and checked to sit at most the drape's `leave` (the setup's `tol`)
above it. The fit is charged to that second budget:
- The passes are draped within `tol − fit` above, and waterline contours are held to `fit`/5.
- So a fitted piece, within `fit` of its pass, stays within the plain op's `tol`.
- The stepover, and so the cusp, is exactly the plain op's.

A first version charged the fit to the cusp instead: the stepover was taken for `cusp − fit` (for
`flat`, the stepover less 2·fit, its pieces being level). That cost 0–6% in cycle time. The table
below compares both against the unfitted op.

## Defaults

- `fit` = 0.25 × the setup's `tol` (0.0025 mm at `tol` 0.01), at most half of it. `fit: 0` turns
  fitting off.
- Arcs are level only: their ends may differ in height by at most `fit`. The checker takes a
  helical arc as the arc swept over its whole height range, which cannot be proven along a
  drafted wall.
- Arc radius 0.05 to 200 mm. The axis-limit check boxed an arc's whole circle, so a near-straight
  arc of radius ~1000 mm failed it. That check now boxes the arc's own sweep.
- Curved xy patterns must be chorded well inside `fit`, or no arc fits their points:
  - The spiral is chorded at `fit`/5 when fitting.
  - Waterline contours (the steep part, ramping) have their vertices and outward sag held to
    `fit`/5, with the proof slack left at the op's.

A contour held to `fit`/5 inward as well (chords cutting a convex corner) was tried and dropped.
It tripled the points of steep_shallow (40,701), gave 432 arcs, and made the path longer than
unfitted.

## Measured on the mould core

The setup:
- Part: `mold_core.fg` in 6061-T6, roughed by zlevel with `flat-10`.
- Each op alone with `ball-6`, cusp 0.01, `tol` 0.01.
- Machine: `generic-vmc3`, posted with `fanuc`.
- Times: `forge_machine::lookahead::plan_scurve` with the default assumption set. The second time
  column assumes a 2 ms block cycle: no published value, a sensitivity check only.
- "≥90% F": the share of cutting time in blocks whose average speed is at least 90% of their
  programmed feed.
- Every row certifies (`cam.ok`).

To reproduce: `cargo run --release -p forge-cam-lang --example fit_report`.

Three runs of each op: unfitted (`fit: 0`), fitted with the cusp budget (fit 0.0005 =
0.05 × cusp, the earlier version), and fitted with the tolerance budget (fit 0.0025, as shipped).

| strategy | fit | blocks | mean move mm | NC bytes | t_scurve s | ≥90% F | stepover / step mm | fitted as | measured max dev mm |
|---|---|---|---|---|---|---|---|---|---|
| steep_shallow | 0 | 13701 | 0.835 | 237455 | 760.9 | 79% | 0.3461 / 0.3461 | | |
| steep_shallow | cusp 0.0005 | 6322 | 1.880 | 136802 | 792.2 | 77% | 0.3374 / 0.3374 | 13257 points as 5141 lines | 0.00000 |
| steep_shallow | tol 0.0025 | 5711 | 2.003 | 124980 | 756.8 | 77% | 0.3461 / 0.3461 | 12903 points as 4423 lines and 143 arcs | 0.00247 |
| spiral | 0 | 1115 | 0.751 | 24686 | 50.9 | 89% | 0.4895 | | |
| spiral | cusp 0.0005 | 253 | 3.578 | 9263 | 55.2 | 87% | 0.4771 | 11284 points as 8 lines and 187 arcs | 0.00050 |
| spiral | tol 0.0025 | 183 | 4.829 | 6524 | 53.8 | 87% | 0.4895 | 4900 points as 7 lines and 120 arcs | 0.00249 |
| radial | 0 | 807 | 2.135 | 17817 | 108.9 | 34% | 0.4895 | | |
| radial | cusp 0.0005 | 415 | 4.290 | 9292 | 111.9 | 71% | 0.4771 | 614 points as 198 lines | 0.00000 |
| radial | tol 0.0025 | 407 | 4.244 | 9144 | 108.9 | 71% | 0.4895 | 593 points as 193 lines | 0.00000 |
| scallop | 0 | 1539 | 0.507 | 22978 | 48.1 | 81% | 0.4895 | | |
| scallop | cusp 0.0005 | 163 | 4.916 | 3032 | 48.5 | 91% | 0.4771 | 1544 points as 144 lines | 0.00000 |
| scallop | tol 0.0025 | 159 | 4.968 | 2964 | 48.0 | 88% | 0.4895 | 1520 points as 140 lines | 0.00195 |
| ramping | 0 | 6110 | 0.609 | 187068 | 225.8 | 92% | — / 0.2447 | | |
| ramping | cusp 0.0005 | 4798 | 0.810 | 137972 | 234.3 | 93% | — / 0.2386 | 7040 points as 4631 lines | 0.00050 |
| ramping | tol 0.0025 | 2220 | 1.687 | 86061 | 226.7 | 94% | — / 0.2447 | 6689 points as 1228 lines and 827 arcs | 0.00245 |
| flat | 0 | 7877 | 0.385 | 122543 | 189.1 | 81% | 0.4895 | | |
| flat | cusp 0.0005 | 351 | 8.676 | 6004 | 191.5 | 75% | 0.4885 | 7444 points as 168 lines | 0.00000 |
| flat | tol 0.0025 | 351 | 8.695 | 5982 | 185.5 | 74% | 0.4895 | 7694 points as 168 lines | 0.00000 |
| rest | 0 | 1971 | 0.588 | 47013 | 102.3 | 76% | 0.4895 | | |
| rest | cusp 0.0005 | 1899 | 0.626 | 45580 | 107.7 | 76% | 0.4771 | 1670 points as 1662 lines | 0.00002 |
| rest | tol 0.0025 | 1915 | 0.605 | 46149 | 102.3 | 75% | 0.4895 | 1734 points as 1678 lines | 0.00209 |
| pencil | 0 | 317 | 0.686 | 5047 | 22.3 | 70% | — | | |
| pencil | tol 0.0025 | 93 | 2.373 | 1721 | 22.3 | 70% | — | 304 points as 80 lines | 0.00000 |

The 2 ms block-cycle column moved by at most 1 s and is left out here (`fit_report` prints it).

## Findings

- **With the tolerance budget the stepover is the plain op's, and cycle time is within −2% to
  +0.4%.** The exception is spiral (+5.7%); see the last point.
- **Blocks and program size fall sharply** wherever a pass is straight in xy or lies in one
  plane:
  - flat: 22× fewer blocks, 20× smaller program;
  - scallop: 9.7× fewer blocks, 7.8× smaller;
  - spiral: 6.1× fewer blocks, 3.8× smaller, mostly arcs;
  - pencil: 3.4× fewer blocks;
  - ramping: 2.8× fewer blocks, 2.2× smaller;
  - steep_shallow: 2.4× fewer blocks, 1.9× smaller.
- **Where they don't:** rest's short passes over concave fillets barely merge.
- **Time does not drop under this planner model.** At these feeds (about 6 to 16 mm/s) a 0.4 mm
  block still lasts 25 ms or more: well above any block cycle, and the planner blends the shallow
  corners at full speed. The gain in time would come from feeds high enough that block
  processing or corner blending limits speed.
- **"≥90% F" is a per-block average, so it penalises long blocks.** A long block that includes
  its own acceleration ramps can average under 90% while cruising most of its length. That is
  why flat drops from 81% to 74% while its time falls. Total time is the better measure.
- **Arcs on tiny radii are slower than chords there.** Near the spiral's centre the planner holds
  an arc to its centripetal acceleration, while it blends the chords' corners within its
  tolerance (spiral +5.7%).
