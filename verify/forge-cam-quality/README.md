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

## Where we stand (baseline.txt, 2026-11-24)

1. **Pocket rings each re-enter from the retract height with a full helical ramp.** Every
   concentric ring of a pocket level retracts, rapids, plunges to the top and ramps down again
   at the plunge feed (30% of the cut feed), with full engagement: thin_wall's pocket spends 355
   of 522 s of cutting at plunge feed, 70% of its time at a chip under half the target
   (well_island 74%, pocket_islands 73%, bracket bore 93%).
2. **clear / contour / slot feeds assume ae = 0.1 D** (chip thinning ×1.67) while those ops run at
   0.4 D stepover, full slots and corners: the chip is 1.5–1.7× the target for 55–95% of the time
   (flange top 92%, boss rough 92%, contours 91–95%, slots 13%).
3. **Straight plunges into material**: `clear` enters every offset loop by plunging (flange top 24,
   boss rough 40, plate_break side 25); pockets without `ramp` plunge to full step.
4. **Air**: clear ops cut air 43–54% of their feed time; facing 10–27%.
5. `clear` refuses parts whose side wall is one L-shaped face (an open step flush with the
   outline: `NotPrism`, parts/open_step_cam.fg).
