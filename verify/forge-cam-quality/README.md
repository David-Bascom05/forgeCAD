# forge-cam-quality: how good forge-cam's paths are to run

forge-cam certifies its paths (no gouge, no collision, the result as claimed: G1–G6). This crate
measures what the certificate does not: cycle time, engagement, chip, removal, air, links, load.

```
cargo run --release -p forge-cam-quality -- [--json OUT] [--machine NAME] [--cell MM] [FILE..]
```

With no files it runs `examples/machined/*_cam.fg` and the benchmark parts in `parts/`
(pocket with islands, thin wall, deep slot, open step, boss). `baseline.txt` / `baseline.json`
are the numbers at the commit that made them. `tests/pinned.rs` pins a CI-cheap subset
(well_island, deep_slot; 0.5 s in debug, native and wasm32 the same): re-pin with
`FORGE_BLESS=1` only for a judged change, with the before/after in the commit message.

## How each number is made

- **t_stop, t_min**: the posted program on the machine (forge-machine's timeline): `t_min` a
  lower bound from feeds and rapid rates, `t_stop` exact stops with trapezoidal acceleration. Per
  op: the op's blocks (from its `(name kind)` comment to the next op's), tool change included.
  The machine is the setup's own unless `--machine`; generic-vmc3's rates are placeholders, so
  absolute times are indicative and comparisons (before/after) are what count.
- **cut_s, air**: Σ length/feed of the feed pieces (≤ min(R/4, 0.5) mm, arcs by chords) that
  remove material from the height-map stock, and of those that remove none (air).
- **retr, plng, ramps**: upward z-only moves of > 0.5 mm; straight-down feed moves that remove
  material; ramp entries (a run of descending xy moves, e.g. a helix, counts once).
- **c>90**: direction changes of more than 90° between consecutive xy feed moves (also c>30).
- **phi**: engagement angle φ of the side edge (front half, 1° steps, read at r − 3h/4):
  max and time-weighted mean over cutting pieces; `phi_hist` in the JSON. ae/D = (1 − cos φ)/2.
- **climb / conventional / slot**: M03 assumed. Material at the right extreme of the front half
  (the wall is on the right) = climb; left = conventional; both = slotting.
- **hex**: max chip thickness = fz · max cos θ over the engaged edge (θ from the travel), with fz
  = programmed F / (rpm · flutes): radial chip thinning included. `fzTgt` is the cutting data's
  feed per tooth for the diameter (forge-tooling). **thin**: time with hex < 0.5 fzTgt (rubbing);
  **thick**: hex > 1.25 fzTgt (overload).
- **MRR**: removed volume / cut time (mm³/min).
- **Fc_N, P_kW**: Kienzle, kc = kc1.1 · hm^−mc with hm = fz · mean cos θ: peak tangential force
  per tooth kc1.1 · ap · hex^(1−mc), peak power ap · ae · vf · kc / 6e7 (ae from φ).
- Not yet: scallop/cusp (2.5D floors with flat tools have none; for 3-axis), tool deflection.

The stock model is a height map (cells D/60 in [0.02, 0.1] mm by default): a measurement within
about a cell, not a certificate. Setups that continue another (`from:`) start from their own
stock box, not from the earlier setup's result.

## Where we stood (baseline.txt, 2026-11-24, main f04c533)

1. **Pocket rings each re-entered from the retract height with a full helical ramp** at the
   plunge feed (30% of the cut feed), fully engaged: thin_wall's pocket spent 355 of 522 s of
   cutting at plunge feed; 70–93% of pocket time at a chip under half the target.
2. **clear / contour / slot feeds assumed ae = 0.1 D** (chip thinning ×1.67) while those ops ran
   at 0.4 D stepover, full slots and corners: the chip 1.5–1.7× the target for 55–95% of the time.
3. **Straight plunges into material** at every `clear` loop entry (flange top 24, boss rough 40).
4. **Outside contours were conventional** (counter-clockwise round the part with M03).
5. `clear` refuses parts whose side wall is one L-shaped face (open step flush with the outline:
   `NotPrism`, parts/open_step_cam.fg). Reported to CAM-CORE.

## Where we stand (latest.txt)

| | baseline | latest |
|---|---|---|
| all 13 programs, t_stop (generic-vmc3) | 2627.5 s | 1950 s (−25.8%) |
| side-cutting time with chip > 1.25 fz (overload) | 17.8% | 0.0% |
| side-cutting time with chip < 0.5 fz (rubbing) | 19.6% | 1.4% |
| chip p99 / data fz, worst milling op | 1.7 | 1.00 |
| outside contours, climb | 0% | 100% |

Since (cam/bench-rptu, 2026-11-24): pocket and clear entries below the first level come down
to the level above at the cutting feed as a link, not at the plunge feed (the tool stood at that
very point at that depth one level up: ops.rs `air_down`). The 15 programs of latest.txt
(boss_adaptive and open_step included): 2407.4 → 2335.9 s (−3.0%); plate_break −19.1 s,
bracket −22.2 s over its three programs; tests/pinned.txt re-pinned for it (well_island 81.0 →
78.4 s, deep_slot 92.7 → 88.2 s).

Changes (cam/quality): stay-down linking between pocket/clear passes (greedy ready-innermost
order, links tested exactly against the tool-centre region); feeds from engagement
(forge_cam::engage, GE US 5,828,574-style, expired); honest planned ae for base feeds; contours
climb and cut each loop through its levels; feeds held ≥ D/2 per move.

Still to do: air in
`clear` (its outer ring is in air by design: a lemma that air rings may be dropped); retracts at
the cut feed (G0 would need G3 against the op's own cleared voids); contour lead-in/out arcs;
rest machining; scallop for 3-axis; engagement-controlled clearing (research/cam/
engagement-clearing.md, after counsel).

## Foreign programs: the same parts, another CAM system (rptu-10.txt)

```
cargo run --release -p forge-cam-quality -- --ngc FILE X0 Y0 Z0 X1 Y1 Z1 [--flutes N] [--machine M]
```

measures a program another system posted (src/foreign.rs): the text is read by forge-machine's
interpreter (LinuxCNC dialect, up to 4 decimals and 3 in F), split into its ops at its operation
comments, swept through the stock box given (program frame), and timed on the same machine
model as our own programs. Tools come from the tool comments (`(T1 D=6. CR=0. ...)`); without
`--flutes` the chip columns are per revolution. `tests/foreign.rs` pins part 3.

The parts are RPTU's 10-part 3-axis benchmark (verify/cam-parts/rptu-10, CC BY 4.0, NOTICE
there): STEP models and the Autodesk CAM programs the authors made and machined, one 6 mm flat
end mill. `parts/rptu/` imports parts 2–6 (`import(file: "N.step")`) and machines them with our
ops and the same tool: face 1 mm, the surrounding step (`clear` or `adaptive`), the holes and
pockets (`pocket`), all certified. Part 1 is refused: its side walls are notched by the through
slot (`NotPrism`, as open_step; with CAM-CORE). Parts 7–10 have freeform (3-axis finishing).

Whole program, t_stop on generic-vmc3 (2026-11-24, with `air_down`):

| part | Autodesk | ours, step by `clear` | ours, step by `adaptive` |
|---|---|---|---|
| 2 (4 holes) | 315.7 s | 102.6 s | – |
| 3 (+ rectangular step) | 474.4 s | 150.9 s | 316.3 s |
| 4 (+ circular step) | 514.7 s | 214.3 s | 194.2 s |
| 5 (+ mixed step) | 463.8 s | 195.5 s | 189.5 s |
| 6 (+ circular pocket) | 681.8 s | 252.7 s | 417.9 s |

Read with care: the feeds differ as well as the paths. Theirs: S18000 (generic-vmc3 stops at
12000: one `Rpm` violation, the timeline runs the programmed F), F2000 side cutting (0.111
mm/rev), F1000 in pockets, F333 plunging, facing one-way with a lift per pass, holes by a
shallow helix (0.22 mm per turn) to depth. Ours: the cutting data's fz 0.039 × 3 flutes at 12000
(F1402, raised where the engagement thins the chip). Their adaptive holds the engagement
(φmax 80°, 2% air on part 3's step: 63 s); ours does not yet (part 3: 211 s, 80% air, 109
retracts): that is the work. Their offset pocket for part 6's circular pocket meets 180° as ours
does. The block: the dataset says 80 × 80 × 50; the models are 50–52 mm tall and every program
faces 1 mm off: measured as Z −50..0 with the part's top at Z −1 (NOTICE).
