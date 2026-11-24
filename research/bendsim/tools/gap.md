# Bend-sim tooling: what we model, what shops use, the gaps ranked

Status: Phase 1 report, 2026-11-24, BENDSIM-TOOLS (branch `bendsim/tools`, from main 0de84ac1).
Question (David): "more sophistication from the tools we use in sim. Do we have the weird bent tools
which move on an angle to do really deep bends? Like the tool itself moves? Do we have tool change?
Do we use realistic tools?" Press-brake program export (G-code/CNC) is out of scope.

Rules kept throughout: every check certified (Clear = proven >= limit, Near = within the
resolution, a warning), cited data only (UNKNOWN stays UNKNOWN and refuses or is "not checked"),
zero forge-* speed regressions.

## 0. What the code does today (verified on main 0de84ac1)

| area | as built | where |
|---|---|---|
| punch kinds | `Straight`, `Gooseneck` (throat `Relief {depth, z0, z1}`), `Acute`; tip radius, tip angle, tip height, body width, height, segments, kN/m | `tooling.rs` |
| sections | **generated** from those parameters (an envelope; a gooseneck's throat a polygon); an explicit `profile` is supported but **no catalog record has one** (0 of 316) | `tooling.rs` `body_section`, `data/tooling/*.toml` |
| catalog | 7 makers, **168 punches + 148 dies = 316 tools** (the ~2,800 figure is the cited values, not the tools), 32 presses; every value cited; UNKNOWN working geometry refuses (`UnknownGeometry`) | `catalog.rs`, `data/` |
| usable catalog punches | known geometry: amada 0/17 (width unknown), bystronic 15/15, mate 8/21, rolleri 6/14, trumpf 13/19, wila 33/47, wilson 2/35. Goosenecks with no published throat depth are refused (trumpf 5, wila 14, mate 5, rolleri 8) | tally of the TOML |
| default tooling | CLI, MCP and app default to the **stand-in** `std-eu` (our own round numbers); a real press only with `bendsim press P [punches ..] [dies ..]`; with a press and no tools named, **every** known tool of the press maker (`Catalog::default_tools`) | `forge-lang/src/bendsim.rs`, `catalog.rs` |
| tool choice | per bend: both orientations x punches in order straight, gooseneck, acute (then id) x dies by V nearest `v_rule * t` (8 t); first passing setup taken; `choice` says "a gooseneck punch: every straight punch failed (rules ..)" with up to 24 rejected setups | `plan.rs`, `forge-lang` `choice()` |
| tool height | punch and die heights set the tip at top dead centre; `open_height`, `stroke`, `bed_length`, `segments` (fits between standing flanges), `tonnage` are checked | `sim.rs` |
| stations | `Tooling.stations` (punch, die, y, lengths): all stations' tools on the press and in the collision check; **only when the bending document states them**; the planner never lays stations out | `tooling.rs`, `forge-bend-lang` |
| tool change | `Program` counts `setups` (distinct punch/die pairs) and `changes` (steps whose pair differs) and ranks by them; **no time**, no tool-change or station-shift stage in the scene | `plan.rs` `Program`, `scene.rs` `STAGES` |
| ATC | Amada HG-1003ATC is in presses.toml (cited: AMTS III, 15 punch / 18 die stockers); nothing reads it | `presses.toml` |
| moving tools | none: the ram moves in z, the backgauge in x/r; every tool is a fixed y-prism | `sim.rs`, `scene.rs` |
| hems | refused: `HemNotSimulated` | `model.rs` |
| Z/offset, bump, radius | none (radius only as `punch_radius` vs inside radius) | |
| crowning, deflection | none | |
| horns / box segments | segments are lengths only; no relieved-end (horn/ear) segments | |

### Measured on the showcase (examples/sheetmetal/load_tray.fg, 6 bends, release build)

| tooling | result | time |
|---|---|---|
| default `std-eu` (stand-in) | feasible; all six bends on `std-eu/P85-R08-H120` + `V12-88`: straight punch "the first choice" | 3.8 s |
| `press trumpf-trubend-5130` (every known Trumpf tool) | **infeasible, budget exhausted** (4000 setups); its "best attempt" is a R4 straight punch on a 30° die, failing `punch_radius` | 11.6 s |
| TruBend 5130 + the 5 Trumpf goosenecks with a published throat + V10/V16 84° | **infeasible, proven** (1582 setups): `closes_on_punch` against the punch body (-7 to -16 mm) | 10.2 s |

So the honest answer to "do we use realistic tools?" is: the data are real and cited, but the default
is the stand-in, and on real tools the showcase part fails. Two causes: the bodies are envelopes that
err large (fat by design, since no drawn sections are held), and the search is unordered over a whole
maker's catalogue, so it burns its budget on dead tools (30° dies, R4 punches against an R1.5 bend).

## 1. Gaps ranked by value

Effort: S <= 1 day, M 2-4 days, L 1-2 weeks, XL longer. "Prove" = what the tests and the certified
contract must establish.

### Rank 1. (a) Real tools by default: real sections, a real shop kit, names shown

What a shop sees: its own press and the 6-12 tools it keeps on the rack, by catalogue number, drawn
as they look.
- **Real sections.** Add `profile` (exact lines and arcs) to the most common tools from their
  published dimensioned drawings: for each maker's standard straight punch, its main gooseneck, a
  30° acute, and the 84-90° dies V6-V16. Where a drawing does not dimension a flank, that flank
  stays an estimate that errs large (as now), marked per segment. Envelope-only tools stay usable,
  labelled "envelope" in the UI.
- **Shop kit.** A cited `[[kit]]` per maker in the TOML: the press plus a small set (straight,
  gooseneck, acute; V 6/8/10/12/16 at the maker's standard angle), each tool id taken from the
  maker's own "basic tooling package" where one is published, else UNKNOWN and not offered as a
  kit. The default becomes a named real press + its kit; `std-eu` stays, named a stand-in.
- **Search order on a big set.** Prune before the search: drop a die whose angle cannot reach the
  overbend, a punch whose tip radius exceeds the inside radius or whose angle is too wide (closed
  form, already rules), so 30° dies and R4 punches never enter it. This makes the result exact
  rather than budget-bound, and is a speed gain.
- **Names.** Steps, Checks and the 3D labels show maker + order number + kind
  ("Trumpf OW203-S straight, H220 R4 60°").
- Effort: M (data 2 days for ~30 sections; code 1 day). Prove: each profile is a closed loop with
  the tip at the origin, its published dimensions reproduced to 0.01 mm (test per record);
  load_tray feasible on the default kit; programs identical on the stand-in sets (fixtures pinned);
  pruning never removes a setup that passes (test: prune on vs off, same plan, fewer setups).
- Data: makers' dimensioned drawings (WILA, Wilson, Trumpf, Bystronic catalogues already cited in
  `research/bendsim/env/data/maker-tooling.md`); kit lists UNKNOWN until found.

### Rank 2. (b) Deep parts: the right tall or gooseneck punch, and why

Today the order is by kind only; height and throat are not compared to the part, and "why" names the
rule, not the place.
- Rank goosenecks by need: for the flange that closes on the punch, the throat depth and window
  (z0..z1) it needs follow in closed form from the done flanges' heights at the bend's end angle;
  try punches whose throat covers that first, and tall punches (height) where the ram is hit.
- Explain with the witness: "straight punch P85 failed closes_on_punch: right_lip.tip.face 7.3 mm
  into the body at 61°; G88 has a 50 mm throat, the lip needs 14 mm". The witness is already in the
  check (`at`, `phi`, `part`, `tool`); this phrases it in the reply's `choice` and the Checks panel.
- Effort: S-M. Prove: the closed-form need is a lower bound (a punch that fails it fails the
  certified check: tested over the return-lip fixtures); plans unchanged where a straight punch passes.

### Rank 3. (c) Tool change and stage bending

- **Planner lays out stations**: one station per distinct (punch, die) the program needs, packed
  along the bed left to right with a gap >= the part's widest extent, every station's stack length
  by the existing segment rule; the program ranks by flips/turns, then stations, then changes (as now),
  then time.
- **Every bend says its station**, the part moves between stations (a y shift in the scene, as a
  handling stage), and every station's tools are already checked against the part (sim does this
  when stations exist).
- **Tool change** where the stations do not fit the bed: a `ToolChange` stage in the scene
  (punch and die out, new ones in) and counted.
- **Time**: per step, approach + bend + return from the press's cited speeds (`PressRec.speeds`,
  UNKNOWN where not published: then "not timed"); tool change and station shift times UNKNOWN unless
  cited: shown as counts, not seconds. ATC presses (Amada HG-ATC: "loads even the most complex tool
  layout within three minutes", Amada brochure; Trumpf ToolMaster: "a few seconds" per tool, no
  number published) take the cited figure, marked as the maker's claim.
- Effort: M (3-4 days, with UX-LEAD review of the scene stages). Prove: a station layout fits the
  bed and the segment counts the catalogue holds (sum <= available); the part vs every station's
  tools, certified, through every step; a single-station program is unchanged (fixtures).
- Data: Amada HG-ATC brochure 2021 E056-EU03-EN (amada.eu); Trumpf ToolMaster Bend page
  (trumpf.com); change times otherwise UNKNOWN.

### Rank 4. (d1) Hemming, two stages

Removes `HemNotSimulated`, the one refusal a shop meets most.
- Stage 1: an acute pre-bend to about 30° included (WILA: "the first step when creating a hem is to
  make a 30° bend"), on the existing acute punch and die: an ordinary step.
- Stage 2: flattening between a flat punch face and a flattening die (or a combined spring-action
  hemming die), the flange closed from the pre-bend to the hem's gap. New: the flatten contact (the
  flange between two parallel faces: closed form, like the shoulder contacts), the fold state taken
  to phi near pi with the hem's inside radius and gap, and the force rule from the maker's
  flattening tonnage table (cited or "not checked").
- New collision checks: none new in kind (same B&B over the flatten's phi interval).
- Effort: M-L (model.rs hem joint refold near pi needs care: sinc/versine forms already hold).
  Prove: the refold at the end equals the built hem (to the bit, like the other bends); the
  flatten's contact rule; flattening dies added to the catalogue with cites.
- Data: WILA hemming application page and catalogue (flattening top tool BIU-051 and bottom tool
  OZU-083 named in the WILA GB catalogue), Wilson Tool "hemming / flattening die" catalogue page.

### Rank 5. (d2) Z / offset (joggle) tools

Two bends in one stroke, the offset fixed by the tool.
- Model: a step that drives two joints with coupled angles as functions of the ram's stroke s; the
  B&B cell takes s in place of phi (both angles enclosed from s by interval arithmetic).
- Rules: the part's offset equals the tool's (within tolerance); the flange between the two bends
  fits the tool's step; tonnage per the maker's table.
- Effort: M. Prove: at the end of the stroke the two-bend state equals the folded part; the coupling
  is monotone in s (so the interval enclosure holds).
- Data: Wilson Tool offset punch-and-die sets (offsets 6, 7.9, 11.1 mm listed on its open catalogue);
  other makers UNKNOWN.

### Rank 6. (e1) Box tooling: horns and segment layout round flanges

- Segments with relieved ends (horns/ears) so a box's last side bends between standing flanges;
  each segment a separate y-prism with its own section; the layout chosen to put horns at the
  flanges. Today's `segments` rule only fits lengths.
- Effort: M. Prove: the stack covers the bend less its allowed ends; every segment's section
  checked (certified) against the standing flanges.
- Data: horn segments' drawings (WILA, Trumpf) UNKNOWN in our notes; to be cited.

### Rank 7. (d3) Tools that move: a folding (swing-beam) machine

This is the "machine where the tool itself moves on an angle", and it does very deep boxes: the
sheet is clamped flat between the lower and upper (clamping) beams and the folding beam swings
about the bend line, up (and on up/down folders, down).
- New machine type (`Folder`): the part stays put; the folding beam's section rotates with phi
  about the bend axis; the clamping beam's tool ("goat's foot") is the punch analogue.
- New collision checks: part vs a rotating prism (the rotation enters the interval enclosure exactly
  like the part's own phi does today) and vs the clamping beam's tool; clamping-beam stroke (opening)
  and folding-beam adjustment as rules.
- Effort: L. Prove: the rotating body's enclosure (as the part's: sin/cos with ulp bounds); a
  folder's sequence planner (no flips needed for up/down folders).
- Data: Schröder Group catalogues on directindustry (clamping beam stroke 350-650 mm, folding beam
  85°/s, per model); body geometry largely UNKNOWN: envelopes that err large.

### Rank 8. (d4) Radius and bump bending

- Radius punches: already a radius rule; add large-radius punches from catalogues.
- Bump (step) bending: a large radius made of n small air bends; the planner emits n sub-steps
  along the zone; a rule bounds the polygon's deviation from the designed cylinder (closed form:
  sagitta of each facet).
- Effort: M. Prove: the n-facet fold state's deviation bound; ordinary checks per sub-step.

### Rank 9. (d5) Rotary benders (rocker in a saddle on the ram)

- A rocker that rotates as it meets the sheet, wrapping the flange over the die's edge: tool
  kinematics coupled to stroke (rocker angle from contact), a new machine-tool pairing (rocker +
  anvil die), the same certified B&B with a rotating body (shared with rank 7).
- Kinematics: generic, from first principles (a cylinder rolling on a line, its angle from the
  stroke and the contact).
- Effort: L. Data: tool dimensions UNKNOWN.

### Rank 10. (d6) Panel benders

- Blankholder + upper/lower blades with interpolated motion, a manipulator rotating the part. A new
  machine type with multi-axis blade paths; the blade paths and tool shapes are not published:
  UNKNOWN. Only the envelope is citable (Salvagnini P4 brochure: max bend height
  203 mm (P4-2120/2520), 254 mm (P4-2225/3125); four universal tools: upper and lower blade,
  counterblade, blankholder).
- Effort: XL. Recommend: a feasibility rule only (box height vs the cited maximum, part length vs
  the model), not a motion sim.

### Rank 11. (e2) Crowning, deflection

- Crowning compensates beam deflection for one load (The Fabricator, "Better bend accuracy on the
  press brake through crowning"). We have no cited deflection data per press; a sim of the angle
  along the bend would be invented. Recommend: a "not checked: crowning" note for bends longer
  than half the bed at high load, and the tonnage check per metre already in place.
- Effort: S. Data: UNKNOWN.

Already covered (no work): tool height against open height and stroke (`open_height`, `stroke`, the
tip at top dead centre from punch + die heights); bed length and housing gap; tonnage per tool and
press; backgauge reach and finger collisions.

## 2. Proposed Phase 2 order

1. Rank 1 pruning + names (S), then the shop kit and real sections for one maker first (Trumpf,
   to match the TruBend clips), then WILA; load_tray feasible on it; clip.
2. Rank 2 (S-M): gooseneck need in closed form, witnesses in `choice`; clip of a return-lip box
   choosing a gooseneck with the reason on screen.
3. Rank 3 (M): stations laid out, stage bending, tool-change stage, timed where cited; clip.
4. Rank 4 hemming (M-L), Rank 5 Z-tools (M); clips.
5. Rank 7 folder (L) as the first truly moving tool, if David wants it before 6/8.

Scene/renderer changes (station shift, tool change, hem flatten, folder beam) are BENDSIM-ENV's
files: coordinated with them before touching forge-render; kernel scene.rs stages are mine.

## Sources

- WILA hemming: https://www.wilatooling.com/en-us/applications/hemming/ ; WILA GB catalogue,
  https://catalog.wilatooling.com/WILA-Catalog-GB/80/
- Wilson Tool hemming/flattening die: https://wilsontool.com/en-gb/products/bending/european-precision/standard/hemming-flattening-die
- Wilson Tool offset sets: https://wilsontool.com/en-us/products/bending/european-precision/standard/offset-punch-and-die-set
- Amada HG-ATC brochure: https://www.amada.eu/fileadmin/Repository/Products/Brochures/HG-ATC_brochure_2021_E056-EU03_EN.pdf
- Trumpf ToolMaster Bend: https://www.trumpf.com/en_INT/products/machines-systems/automation/automation-for-bending-machines/toolmaster-bend/
- Schröder folders: https://directindustry.com/prod/schroder-group/product-33465-1392597.html ;
  https://www.directindustry.com/prod/schroeder-group/product-33465-937505.html
- Salvagnini P4: https://www.ucimu.it/uploads/ucimu/azienda/Salvagnini_Highlights__short_EN.pdf
- Crowning: https://thefabricator.com/article/bending/better-bend-accuracy-on-the-press-brake-through-crowning

All read 2026-11-24. Figures above are as those pages state them; each enters the TOML only with a
`[[cite]]` (status published / estimate with basis / unknown).
