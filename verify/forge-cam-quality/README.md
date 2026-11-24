# forge-cam-quality: how good forge-cam's paths are to run

forge-cam certifies its paths (no gouge, no collision, the result as claimed: G1–G6). This crate
measures what the certificate does not: cycle time, engagement, chip, removal, air, links, load.

```
cargo run --release -p forge-cam-quality -- [--json OUT] [--machine NAME] [--cell MM] [FILE..]
```

After the table it prints coverage: per document, the ops certified of all and each refusal
with its reason (turning documents, `*_turn.fg`, are read by forge-turn-lang and give coverage
only: the path measures are milling's). The shop parts manifold and hub (examples/machined,
SHOP-PARTS.md) are coverage targets: a refusal there is a gap.

With no files it runs `examples/machined/*_cam.fg`, `*_turn.fg` and the benchmark parts in `parts/`
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

Since (cam/quality-5 on train 4b, 2026-11-24): pocket and clear entries below the first level
come down to the level above at the cutting feed as a link, not at the plunge feed (the tool
stood at that very point at that depth one level up: ops.rs `air_down`); `clear` enters from
beside the stock where it can (rapid down clear of the stock's box, the last mm at the plunge
feed, then in sideways: ops.rs `side_entry`): straight plunges into material in clear ops → 0,
chip unchanged (hex max, p99, 0% overload), φmax higher at those entries (the feed follows).
All programs of latest.txt: 2846.3 → 2763.0 s (−2.9%); manifold refuses the same ops as before.
Then `trim_air` (above): 2763.0 → 2631.3 s (−4.8%), air in clear ops about halved (block_m6 37 →
14%, boss 19 → 11%), the chip unchanged, coverage unchanged.
tests/pinned.txt re-pinned for it (well_island 81.0 → 78.4 s, deep_slot 92.7 → 88.2 s).

Changes (cam/quality): stay-down linking between pocket/clear passes (greedy ready-innermost
order, links tested exactly against the tool-centre region); feeds from engagement
(forge_cam::engage, offline feed optimisation from a stock model); honest planned ae for base feeds; contours
climb and cut each loop through its levels; feeds held ≥ D/2 per move.

Still to do: air in
`clear` (its outer ring is in air by design: a lemma that air rings may be dropped); retracts at
the cut feed (G0 would need G3 against the op's own cleared voids); contour lead-in/out arcs;
rest machining; scallop for 3-axis; engagement-controlled clearing (built since: forge_cam::adaptive;
first design in research/cam/engagement-clearing.md).

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
ops and the same tool: face 1 mm, the surrounding step (`clear`), the holes and
pockets (`pocket`), all certified; part 1 its through slot by `clear` (a part with no sketch has no
`slot` op to use). Parts 7–10 have freeform (3-axis finishing).

### Like for like (2026-11-24)

**1. The same end state.** `--part FILE.fg DX DY DZ` rasterises the part's top surface (its
tessellation) into the stock's grid and compares each program's final stock against it over the
part's footprint (src/endstate.rs). Their programs are in their own frame: G54 at the corner of
an 81 × 81 CAM stock for parts 1–4, 82 × 82 for 5–6, the part's top at Z −1 (NOTICE says how that
was read and checked). Left above the part / cut below it, mm³ (cell 0.1 mm):

| part | theirs: left / below | ours: left / below |
|---|---|---|
| 1 | 0 / 0 | 0 / 0 |
| 2 | 1 / 1 | 2 / 1 |
| 3 | 0 / 3 | 3 / 2 |
| 4 | 2 / 23 | 5 / 2 |
| 5 | 1 / 13 | 4 / 2 |
| 6 | 31 / 2 | 2 / 10 |

Both leave the part: what remains is the cells straddling vertical walls (each a full wall
height deep, a few mm³ in all). Neither has a separate finishing op; their 2D pocket and our
pocket each finish their walls in the same op (tests/foreign.rs pins part 3's).

Ops, theirs → ours: PLANEN (facing, one-way, a lift per pass) → `facing`; 2D ADAPTIVE (the
surrounding step: three closed offset rings round the boss) → `clear`; 2D-TASCHE (holes, a
shallow helix to depth then rings) → `pocket`; part 1's NUT (the slot) → `clear`; part 6's
2D ADAPTIVE4 (circular pocket) and 2D-TASCHE2 (holes) → one `pocket`.

**2. The same feeds.** `--feeds NAME=CUT,PLUNGE` runs an op at given feeds, in the planned moves
and in the posted text the timeline reads (straight-down moves at the plunge feed). Ours at
theirs: facing and step 2000 mm/min, pockets and the slot 1000, plunges 333.33 (part 6: its one
pocket at 1000, though theirs cut the circular pocket at 2000). t_stop, generic-vmc3:

| part | theirs | ours, our feeds | ours, their feeds |
|---|---|---|---|
| 1 (through slot) | 194.4 s | 108.7 s | 105.6 s |
| 2 (4 holes) | 315.7 s | 102.6 s | 89.0 s |
| 3 (+ rectangular step) | 474.4 s | 141.7 s | 125.6 s |
| 4 (+ circular step) | 514.7 s | 205.0 s | 170.9 s |
| 5 (+ mixed step) | 463.8 s | 186.5 s | 162.2 s |
| 6 (+ circular pocket) | 681.8 s | 243.5 s | 266.5 s |

At the same feeds the difference is the paths: their facing is one-way with a lift and a 5 mm
plunge at 333 mm/min per pass (part 3: 172.8 s vs ours 66.6 s at their feed); their holes go
down a 0.22 mm-per-turn helix (part 3: 238.5 s vs 26.8 s). Their step (63.2 s) and ours (44.9 s
at their feed) are close. Part 1's slot was where ours was worse (clear went round both halves
of the part, mostly outside the stock: 47.5 s, 66% air); clear now drops level runs whose tool
disk stays at least r from the stock's box, and straight runs that repeat a segment already cut
at their depth (ops.rs `trim_air`: they remove nothing), so the slot is its centreline, entered
from the side: 21.4 s against their 23.0 s, the same part left. Part 6 at their feeds loses on
the circular pocket (one pocket op held to their hole feed). The numbers are the authors'
programs: an academic benchmark's settings, not Autodesk's best.

**3. What the times are.** generic-vmc3's rates and accelerations are placeholders (kernel/
forge-machine/data/machines/generic.toml), so these are times on one model for both programs,
not either program's time on the RPTU machine. The model is still to be checked against a
measured log (the Spinner U5-620 set, tools/bench: its programs are Siemens 840D with variables,
which forge-machine's interpreter does not read yet).

## Measured machine times: Bílek et al. 2025 (bilek.txt)

Bílek, Řezníček, Matras, Solařík, Macků, *Materials* 18:1669 (2025), DOI 10.3390/ma18071669,
CC BY 4.0 (research/cam/benchmarks/cam-comparisons.md §2.2, branch research/toolpath-quality):
a 60 × 46 × 12 mm pocket with 4 mm corner radii in a 76 × 76 × 40 GFRP block (parts/bilek/
bilek.fg, from the stated dimensions), cut on a DMG Mori DMU 50 with a Ø6 end mill.
NX Cavity Mill at vf 1000 mm/min, ae 1.8 mm, ap 1.5–3.5 mm: measured 5 min 27 s (327 s), NX's
estimate 5 min 26 s. NX Adaptive at vf 2652, ae 0.42, ap 12: measured 4 min 11 s (251 s), NX's
estimate 3 min 49 s (−9 %).

Ours (parts/bilek/bilek_cam.fg): the offset pocket at their conventional data (ae 1.8, ap 3, run
at vf 1000 with `--feeds well=1000,1000`; their ramp feed is not stated): t_stop 351.8 s, t_min
344.7 s on generic-vmc3, certified, the part left (end state). That is our path's time, not
theirs: their Cavity Mill path is not published, so this is a check of magnitude, not of our
timing model (+7.6 % on 327 s, the paths differing). generic-vmc3's acceleration is a
placeholder and there is no DMU 50 record. Adaptive: forge-cam's adaptive does not take closed
pockets (one-direction peels cannot open an entry hole under the cap), so their Adaptive run has no counterpart here.

Ward et al. 2021 (arXiv 2102.02062): its timed pockets (cases 1–2, Table 1) are not dimensioned
in the paper; the dimensioned pocket (case 4, 40 × 60 × 10 open) has forces, not a time. Not
rebuilt.
