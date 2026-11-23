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

### Exactness

Fold states are the exact partial refolds of the folded part (the neutral layer kept). Distances
are certified, not sampled: each piece is enclosed over boxes of its parameters and of the motion
by outward-rounded interval arithmetic and branch and bound, so a pass holds at every instant of
the bend (to the stated resolution: distance >= limit - res), and a fail names a real point whose
distance, its error budget added, is under the limit. Anything else is `undecided`, never a pass.
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
