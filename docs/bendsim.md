# Bend simulation: `query F bendsim`

The part's sheet bent on a press brake, bend by bend, on the vendor's tooling: every check
through every bend's motion, a bend order and tooling that pass them all (or why none does), as
typed JSON. Kernel: `kernel/forge-brake`. Design and proofs: `research/bendsim/README.md`.

```
forgecad query F bendsim [--tooling T] [--vendor V] [--sequence S] [--sheet NAME] [--uts MPA] [--json]
MCP: query {file: F, sel: "bendsim", tooling?, vendor?, sequence?}   (or all of it in sel's words)
```

- Default, with no `--vendor` and no `--tooling`: the sheet's own vendor's tooling if its vendor
  profile has a `[tooling]` section, else standard tooling (`std-eu`). For each bend the planner
  takes the die whose V is nearest the set's rule (`v_rule`, 8 t; acute dies last) and a straight
  punch, a gooseneck only where every straight punch fails, then acute; each step's `choice` says
  which and why, with the setups tried before it and the rule each failed.
- `T`: a standard set (`std-eu`, Promecam-style; `std-us`, American) or a tooling file (`.json`,
  beside the part).
- `V`: a vendor id (`sendcutsend`, `jlc`, .., as forge-vendor's profiles name them: built in,
  the user's, the project's) whose `[tooling]` is used; or a `.json` file with a `tooling` field.
  A vendor's tooling only narrows the choice to what it has: a list it gives (`dies`, `punches`)
  is all there is; what it leaves out is the standard set's, and the reply's `tooling_source`
  says which (`kind` standard | vendor | file, `id`, `why`, and for a vendor its `source`, `taken`,
  `note` and `verify`).
- Presets restated from public pages (not official, verify with the vendor before ordering):
  `sendcutsend` (its four published V widths and 44 in maximum bend length; the rest std-us),
  `jlc` (publishes no press-brake tooling: std-eu). Sources and dates in each profile's
  `[tooling]`.
- `S`: the bends in order, comma separated, by name (the flange's feature name) or number (1 is the
  sheet's first bend, as `bends` lists them). Left out, the order is searched.
- `stations auto` (stage bending): the program's tools laid out as stations along the bed, all
  on the press for every step. Each station is a punch, a die and their stacks' lengths; a bend
  works at the station holding its tools whose stacks cover it and stand inside it, so the same
  tools may stand twice (a box's last sides want shorter stacks). Stations go left to right in
  the order the program first uses them, spaced so the part at one stays off its neighbours'
  stacks (else packed tight and checked), on a 5 mm grid, centred on the bed; the whole program
  is then simulated with every station on the press, every check certified. The reply's `stage`:
  `stations[{n, punch, die, kind, y, punch_length, die_length, reach, bends, why, punch_clear}]`,
  `steps[{bend, station, travel [approach, bend, return] mm, seconds}]`, `flips`, `turns`,
  `shifts` (moves between stations), `time {ram_seconds, counted, not_counted}`: the ram's motion
  at the press's published speeds; handling and moves between stations are not timed (no
  published times). `{ok: false, why}` when no layout fits and passes.
- Tools that cannot make a bend at all (the die's or the punch's angle, the punch's radius,
  decided in closed form) are named in `choice.rejected` with their rule, never simulated: the
  search's budget goes to setups that may pass.
- `--uts`: tensile strength for the force check; else the part's material's, if the material
  library has it; else there is no force check.

## Tooling files

Lengths mm, angles degrees. Every field but `id` has a default from the base set.

```json
{ "base": "std-eu", "id": "acme-press-3",
  "clearance": 0.5, "self_clearance": 0, "short_end": 3, "v_rule": 8,
  "punches": [{ "id": "acme/G88", "style": "eu", "kind": "gooseneck", "height": 135,
                "tip_radius": 0.8, "tip_angle": 88, "tip_height": 12, "width": 26,
                "relief": { "depth": 40, "z0": 25, "z1": 105 },
                "segments": [{ "length": 100, "count": 2 }, { "length": 50, "count": 1 }],
                "max_load": 500 }],
  "dies": [{ "id": "acme/V12", "v": 12, "angle": 88, "shoulder_radius": 1.5, "height": 60,
             "width": 40, "segments": [{ "length": 835, "count": 3 }], "max_load": 700 }],
  "press": { "id": "acme-100t", "open_height": 360, "stroke": 220, "bed_length": 3000,
             "housing_gap": 2600, "ram": { "front": 60, "back": 60 }, "bed": { "front": 60, "back": 60 },
             "max_force": 1000,
             "backgauge": { "fingers": 2, "x": [5, 600], "r": [0, 150], "retract": 5,
                            "finger": { "depth": 25, "height": 20, "width": 15 } } },
  "springback": [{ "material": "steel", "angle": 90, "overbend": 1.5 }] }
```

- `profile`: a punch's or a die's section as `[[x, z], ..]`, in place of the generated one.
  A punch has its tip at the origin; a die has its V centred on x = 0 and its top on z = 0.
- A list given (`punches`, `dies`) replaces the base set's; a list left out is the base's.
- In a vendor profile (TOML) the same keys sit under `[tooling]` (`[[tooling.dies]]`, ..), lengths in the
  file's `units` (mm or in), angles in degrees.

## The reply, schema `forgecad.bendsim/1`

```
ok, schema, sheet, tooling, tooling_source, press, material, clearance,
feasible            every bend passes in `sequence`
sequence            bend names in order (as searched, or as given)
bends[]             n, bend, angle (deg), direction (up|down), radius, length
steps[]             n, bend, angle, overbend (deg), punch, die, punch_length, die_length,
                    orientation {flip, rotate, y}, backgauge {x, r, fingers[], stop} | null,
                    travel (ram, mm), force (kN) | null, pass,
                    choice {die, punch (why, in words), v_rule, tried, rejected[{punch, die, rotate,
                    rule}]}, checks[]
blocking[]          (infeasible) bend, after[], punch, die, checks[] (the failing ones)
fixes[]             typed edit ops (SetParam on a flange height's param, else SetArg)
suggestions[]       {kind: LengthenFlange, bend, wall, by} | {kind: NeedTool, bend, rule, what}
explored, exhausted the setups simulated; whether the search hit its budget
contacts[]          where the part and a tool collide (below)
```

A check: `rule`, `pass`, `severity` (error|warning), `value`, `limit`, `compare` (`>=` or `<=`:
how the value is held to the limit), `verdict` (pass | fail | undecided: neither proven clear nor
proven violated, refused as a failure), `limit_lo` (two-sided rules), `unit` (mm|deg), `certified`,
and where it applies: `bound` (a distance's proven lower bound; `value` is met by a real point),
`part` and `other` (`{anchor, strip, name, surface, segment}`), `tool`
(`punch:ID/tip`, `punch:ID/body`, `die:ID`, `ram`, `bed`, `backgauge:fingerN`), `at` (press frame,
mm), `phi` (the bend's turn there, deg), `phase` (static|approach|bend|return).

A part's `anchor` is the part's own text: `W.top`, `W.bottom` (a wall's skins), `F.tip.face`,
`F.start_side.face`, `F.end_side.face` (a flange's edge faces), `F.bend_top`, `F.bend_bottom`
(a bend zone's skins).

Press frame: x across the bed, +x towards the backgauge; y along the bed, 0 at the bend's middle;
z up from the die's top.

### Rules

| rule | value vs limit |
|---|---|
| `tool_collision` | least distance, part to punch, die, ram, bed or finger, vs `clearance` |
| `closes_on_punch` | the same, for an already-bent flange against the punch or ram |
| `self_collision` | least distance, the moving side to the rest of the part, vs `self_clearance` |
| `min_flange` | each leg's straight run vs the die shoulder's contact |
| `v_too_narrow` | the shoulder contact's place along the leg, >= 0 (on the leg, not the zone) |
| `v_depth` | the zone's lowest point above the V's bottom, vs `clearance` |
| `die_angle`, `punch_angle` | the overbent included angle vs the tool's angle |
| `punch_radius` | the tip radius vs the bend's inside radius |
| `backgauge_reach` | the stop's x, within the fingers' range |
| `open_height` | the tip's height at top dead centre vs t + clearance |
| `stroke` | ram travel vs the stroke |
| `bed_length` | the part's length along the bed vs the housing gap (the tooling's vs the bed) |
| `segments` | the shortest tooling length the segments make vs the bend less its allowed ends |
| `tonnage` | air-bending force (1.42 UTS t² / V per metre) vs the press, or kN/m vs a tool |
| `withdraw` | (warning) the punch coming back up past the sprung part |
| `handling` | (warning) least distance, the part carried through the operator's move into the step, to every body of the press, vs `clearance` |
| `tool_rating` | the sheet's thickness vs a tool's published `max_thickness` (only where the maker gives one) |
| `no_flattening` | a hem's pre-bend passes, but no punch and die on the press flatten it (count of passing flattening setups vs 1) |

`handling`: every step after the first. The move is the planned one: lift off the die, pull out
towards the operator until the part's sphere is in front of every body, turn there, push in, set
down. The ram is at top dead centre; the backgauge is at the last step's stop while the part is
lifted and pulled out, at this step's while it is pushed in and set down, and both during the
turn. The part itself (every piece, exactly as the step places it) is carried through the move by
the same certified search as the bends' checks, against every body of the press as it stands (the
station's tools, the other stations', the ram, the bed, the frame, the fingers): a pass is proven
clear, a fail names a real point of the part, at a real instant of the move, proven to meet a body.
First the part's boxes (grown by how far the true surface lies off its samples) and its sphere
are swept through the move, body by body: a body they clear by the clearance is proven clear at a
fraction of the cost (most bodies of most moves); the search settles the rest.
Lifting off (setting down onto) the bed or a die without a profile is not checked against them: a
point above such a body moving straight up only moves away from it, and the part resting on it is
the step's own check. A fail says the planned move meets that body; the operator may still find
another way in (a tilt), so it is a warning.

### Contacts: where it collides, on both bodies

For every step (and every blocked bend's best attempt) whose tool check failed or is Near, every
piece of the part against every tool not proven clear gets its region: the branch and bound's cells
kept down to its leaf size wherever they are not proven at least the clearance away. A cell is
`proven` when a real point of it, at a real instant, is under the clearance; the rest are bounded.
The cells are never fewer than the true contact. A step whose tool checks pass pays nothing.

```
contacts[]  step (null: a blocked bend's attempt), bend, rule, severity, verdict (hit | near |
            undecided), phase, tool (`punch:ID/body`, `die:ID`, `ram`, `backgauge:finger1`,
            `station2:..`), estimate, faces[{anchor, strip, name, surface, segment, cells, proven}],
            depth (the least proven distance of a real point, negative: that deep into the tool),
            lower, limit, when {angle_deg [a, b]} | {ram_mm [a, b]},
            others[{anchor, ..}] (part against part: the faces met; tool is `part`),
            region {points (press frame, up to 64), tool_points (the tool's nearest), bbox,
                    cells, proven, complete, exact}, text
```

`text` says it in a line: "right.top runs 3.252 mm into the punch std-eu/P85-R08-H120's body
50–198.438 mm from the bend's left end, at 0°–90° of step 2 (bend left)".
`query F bendsim collisions [step N]` answers only the contacts (schema
`forgecad.bendsim.collisions/1`), one line each in text; the kernel's `forge_brake::contacts_of`
gives each contact's cells in the piece's own parameters (u, v, w and the motion) for drawing.

### Exactness

Fold states are the exact partial refolds of the folded part (the neutral layer kept). Distances
are certified, not sampled: each piece is enclosed over boxes of its parameters and of the motion
by outward-rounded interval arithmetic and branch and bound, so a pass holds at every instant of
the bend (distance >= limit, proven), and a fail names a real point whose distance, its error
budget added, is under the limit. A distance proven only to within the search's resolution of the
limit (limit - res <= distance, not proven >= limit, and no point found under it) is `near`: a
warning, not a pass, that leaves the program feasible (the reply gives its `resolution`). Anything
else is `undecided`, never a pass.
The double signed distances carry a stated error budget (`forge_brake::bb::sd_err`), an
assumption still to be proven. Contacts the bend is made by (the punch tip on the zone,
the legs on the die's shoulders) are decided by the closed-form rules. The search is deterministic.

### Errors

`{ok: false, schema, error: {kind, text, ..}}`: `NotBuilt` (with the build's own error, its span
and fixes, as `cause`), `NoSheet`, `NoSuchSheet` (`sheets`), `NoSuchTooling` (`ids`), `BadTooling`,
`NoSuchBend` / `BadSequence` (`item`, `bends`), `HemNotSimulated` (`bend`), `NotBend`,
`SplineOutline`, `SheetNotBuilt`.

### In `query F dfm`

`forge_lang::bendsim::simulate_sheet(&SheetOut, dir, &Ask)` and `items(&SheetOut, &Sim)` give the
findings in the DFM report's Item shape: `rule` (the words above), `severity` (violation, warning,
refused for a blocked bend), `feature` (the flange), `measured`, `limit`, `sense`, `unit`,
`location {anchor, at: null}`, `data {step, punch, die, tool, phase, phi_deg, at_press}`, `fixes`.

## Hems

A hem's bend is made in two steps, each saying its `stroke`: `pre-bend`, an air bend to 30 degrees
included (WILA: "The first step when creating a hem is to make a 30° bend"), on tools whose `role`
bends; then at once `flatten`, the part's base flat on a flat die and the leg closed to the hem's
turn by a flat face coming down on the part's highest point. Every other step's `stroke` is `bend`.
The flattening is checked for its stroke, the open height over the pre-bent leg, the tools'
segments and ratings, the part on itself (but the hem's own two layers: their gap is the sheet's)
and every tool through the stroke; its force is not checked (the makers' flattening tables are
read, their columns not yet confirmed). A pre-bend's punch radius is not held to the hem's: the
tool sets the pre-bend's, the flattening closes it. The flattening has no backgauge stop.

Tools carry a `role` in a tooling file: `bend` (the default), `flatten` (a flat punch face or a
flat die), `hem` (a combination die: a V for the pre-bend beside a pad that flattens; not yet
simulated), `offset` with its `z`; and an optional `max_thickness`. The published figures read so
far are in research/bendsim/tools/hems-data.md.
