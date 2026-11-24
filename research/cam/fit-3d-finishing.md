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
- The spiral's xy pattern is chorded at `tol`/20 (and within `fit`/5) whether fitted or not, so
  arcs fit its points and both runs cover the same area.
- Waterline contours: ramping's are the plain op's (`tol`/2, and `tol`/2 + `fit` is within `tol`);
  steep_shallow's waterlines are left exactly as the plain op's and unfitted, and only its shallow
  rows are fitted. A fitted steep contour has to be traced within `tol − fit` to keep `tol`, which
  made its path 1.2% longer than the plain op's (cut 768.5 s against 760.9 s); unfitted, it cuts
  in 760.2 s. Contours held finer (`fit`/5) let more arcs fit but doubled ramping's generation and
  lengthened both paths; one held to `fit`/5 inward as well tripled steep_shallow's points. All
  were dropped.

## Proving arcs cheaply

- **In the generator:** an arc is proven by its chords. The cutter's signed distance is
  1-Lipschitz in its position, and a level arc is within its chords' sagitta s of them. So each
  chord proven by the closed form at limit + s, or by the column grown by s (`Column::grow`, which
  holds the cutter anywhere within s of the chord), proves the arc. Proving the checker's arc
  pieces by search instead ran refusals to the budget: 175 s of CPU on one waterline.
- **In the certifier:** `contact::clear_arc` clears a level arc by its chords' closed form
  (sagitta ≤ 1e-4 mm), in `check::closed_form` and first on `certify_with`'s column route, prisms
  included. A level arc against a vertical cylinder had cost the search about 1.8 s a pair.
- **In the fitter:** the longest run is found by its geometry alone, then proven once.

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

Each op unfitted (`fit: 0`) and fitted (default `fit` 0.0025), as shipped. Generation is the
op's own time; release is the program's (post, read back, certified).

| strategy | fit | blocks | NC bytes | t_scurve s | generation s | release s | stepover / step | fitted as | measured max dev mm |
|---|---|---|---|---|---|---|---|---|---|
| steep_shallow | 0 | 13701 | 237455 | 760.9 | 9.8 | 0.65 | 0.3461 / 0.3461 | | |
| steep_shallow | 0.0025 | 9362 | 180006 | 760.2 | 9.9 | 0.08 | 0.3461 / 0.3461 | rows fitted, waterlines as plain: 12561 points as 8222 lines | 0.00014 |
| spiral | 0 | 4956 | 108671 | 54.9 | 0.52 | 0.03 | 0.4895 | | |
| spiral | 0.0025 | 230 | 7434 | 53.9 | 0.71 | 0.12 | 0.4895 | 4900 points as 61 lines and 113 arcs | 0.00249 |
| radial | 0 | 807 | 17817 | 108.9 | 0.16 | 0.02 | 0.4895 | | |
| radial | 0.0025 | 407 | 9144 | 108.9 | 0.17 | 0.02 | 0.4895 | 593 points as 193 lines | 0.00000 |
| scallop | 0 | 1539 | 22978 | 48.1 | 0.10 | 0.02 | 0.4895 | | |
| scallop | 0.0025 | 159 | 2964 | 48.0 | 0.12 | 0.01 | 0.4895 | 1520 points as 140 lines | 0.00195 |
| ramping | 0 | 6110 | 187068 | 225.8 | 34.6 | 0.08 | — / 0.2447 | | |
| ramping | 0.0025 | 3452 | 115168 | 225.7 | 34.4 | 0.17 | — / 0.2447 | 5933 points as 3027 lines and 248 arcs | 0.00250 |
| flat | 0 | 7877 | 122543 | 189.1 | 1.97 | 0.10 | 0.4895 | | |
| flat | 0.0025 | 351 | 5982 | 185.5 | 2.07 | 0.03 | 0.4895 | 7694 points as 168 lines | 0.00000 |
| rest | 0 | 1971 | 47013 | 102.3 | 19.5 | 0.05 | 0.4895 | | |
| rest | 0.0025 | 1915 | 46149 | 102.3 | 19.3 | 0.05 | 0.4895 | 1734 points as 1678 lines | 0.00209 |
| pencil | 0 | 317 | 5047 | 22.3 | 0.84 | 0.02 | — | | |
| pencil | 0.0025 | 93 | 1721 | 22.3 | 0.85 | 0.02 | — | 304 points as 80 lines | 0.00000 |

The earlier version that charged the fit to the cusp (stepover for `cusp − fit`, fit 0.0005) was
0–6% slower to cut; it is superseded.

## Findings

- **Generation and release time are the plain op's,** within a few tenths of a second.
- **Cutting time is the plain op's or less on every op.** steep_shallow's waterlines are left as
  the plain op's (see Defaults): fitting them within the tolerance meant a contour within
  `tol − fit`, a path 1.2% longer (`t_min` 682.3 → 690.6 s) and 1.0% slower to cut, though the
  planner's overhead fell (78.7 → 77.9 s).
- **No arc the machine must slow for.** A control holds an arc of radius r to √(a r) (centripetal
  acceleration) and (j r²)^(1/3) (jerk). The fitter's smallest radius is 1.5 × the radius where
  those reach the op's feed (`arc_min_r`, with a and j from the machine record or the planner's
  default assumption set); below it, lines are kept, which the planner blends at the feed.
- **The spiral is like for like.** It is chorded at `tol`/20 whether fitted or not. A coarse
  spiral, clipped to the area chord by chord, loses its ends and covers less: 29 passes and 752 mm
  against 35 and 778 mm. An earlier "+5.7%" was that difference in coverage, not the fitter.
- **Blocks and program size fall sharply** wherever a pass is straight in xy or lies in one
  plane:
  - spiral: 21× fewer blocks, 15× smaller program;
  - flat: 22× fewer blocks, 20× smaller;
  - scallop: 9.7× fewer blocks, 7.8× smaller;
  - pencil: 3.4× fewer blocks;
  - steep_shallow: 1.5× fewer blocks, 1.3× smaller (its rows only);
  - radial: 2.0× fewer blocks;
  - ramping: 1.8× fewer blocks, 1.6× smaller.
- **Where they don't:** rest's short passes over concave fillets barely merge.
- **Time does not drop much under this planner model.** At these feeds (about 6 to 16 mm/s) a
  0.4 mm block still lasts 25 ms or more, and the planner blends the shallow corners at full speed.
- **"≥90% F" is a per-block average, so it penalises long blocks.** Total time is the better
  measure.
