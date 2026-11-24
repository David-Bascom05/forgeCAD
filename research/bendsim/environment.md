# Bend Simulation environment: design note

Status: proposal, 2026-11-24. Branch `bendsim/env`. Owner: BENDSIM-ENV worker.
Goal (David): "the press brake sim is not what I imagined. It's a tool pop-up now. I want the
environment for sim to model the REAL tools, from the tool lib, and show me in 3D with a real
renderer and visualizer."

What stays: forge-brake's exact partial refolds (model.rs), the certified interval branch and bound
(sim.rs, bb.rs), the planner (plan.rs), the CLI (`query F bendsim`). Their results for the existing
cases do not change (tests pin them); every change below adds data and bodies, or is judged.

## 1. Real tooling library (kernel/forge-brake/data/tooling/<maker>.toml)

One TOML file per maker, our TOML subset (forge_tooling::toml, as forge-machine). Search order as
forge-machine: project `bend/tooling/*.toml`, user `<config>/forgecad/bend/tooling/*.toml`, built in;
a later id is hidden by an earlier one.

```
[[punch]]
id = "trumpf/<order no>"   maker = "Trumpf"   name = "..."   real = true
clamp = "trumpf" | "promecam" | "wila-ns" | "american" | "amada-afh" | ...
kind = "straight" | "gooseneck" | "acute" | "sash"
height = 120   tip_radius = 0.8   tip_angle = 88   width = 26   max_load = 1000  # kN/m
relief = { depth = 50, z0 = 30, z1 = 95 }          # gooseneck throat, when dimensioned
profile = [ {line = [x0, z0, x1, z1]}, {arc = [cx, cz, r, a0, sweep]}, ... ]  # exact outline
segments = [ {length = 100, count = 1}, ... ]  sectioned = "..."
[[punch.cite]]  path = "height"  status = "published"  source = URL  taken = "2026-11-24"
                value = "the source's statement, restated with units"
[[die]] ... v, angle, shoulder_radius, height, width, max_load, profile, segments, cites
```

Provenance exactly as docs/cam-machine.md: `published` / `estimate` (basis, and which way it errs)
/ `unknown` / `conflict` (note). A `real = true` record must cite every value it holds.

UNKNOWN geometry refuses, typed: `ToolingError::Unknown { id, field }`; the planner never picks the
tool, and asking for it by id answers `UnknownGeometry` with the fields. Split:
- **working geometry** (tip radius and angle, height, V, die angle, shoulder radius, die height):
  must be `published` (or an `estimate` derived exactly from published dimensions);
- **body envelope** beyond the working part: may be an `estimate` that errs LARGE (a fatter body
  can only report a collision that is not there, never miss one), basis stated; else unknown.
- **outline**: `profile` from the published drawing's dimensions as exact lines and arcs; without
  one, the existing generated section from the published parameters, marked `estimate` (errs large
  where it must: the section's body is the published width's full rectangle).
- `max_load` unknown: the tonnage check reports "not checked" for that tool (never a pass).

Sources, machine vendors first (Trumpf, Amada, Bystronic, LVD), then tooling makers (WILA, Wilson
Tool, Rolleri, Mate). Research notes with every cite: research/bendsim/env/data/*.md; the TOML is
written from them by hand and reviewed against them (a test checks every `real` record's cites).
The old made-up sets stay under their ids (`std-eu`, `std-us`; vendor profiles and tests use them),
renamed in every reply and UI as "stand-in (forgecad's own numbers)", `real = false`.

## 2. Real press brakes (kernel/forge-brake/data/presses/<maker>.toml)

```
[[press]]
id = "trumpf-trubend-5130"  maker = "Trumpf"  model = "TruBend 5130"  real = true
force = 1300               # kN
bed_length = 3230   housing_gap = 2690   open_height = 615   stroke = 365   throat = 420
table_height = 1000  table_width = 60    clamp = "trumpf"
ram  = { front, back, height }          # upper beam envelope above the clamp
bed  = { front, back, depth }           # lower beam envelope below the die seat
frame = { side_thick, depth, height }   # each side housing; the C-throat cut from it
speeds = { approach, work, return }     # mm/s, for the animation's timing only
[press.backgauge]  fingers = 2  axes = ["X", "R", "Z1", "Z2"]
  x = [lo, hi]  r = [lo, hi]  z = [lo, hi]  finger = { depth, height, width }  retract
[[press.cite]] ...                       # as above, every value
```

Bodies: frame housings (with the throat), ram, bed, backgauge beam and fingers, as conservative
boxes and prisms (like forge-machine's envelopes): a body dimension not published is an estimate
that errs large or is unknown. Unknown working values (open height, stroke, backgauge ranges) make
the matching check report "not checked" with the field, never a pass. A tooling set is then
(press, stations) instead of one press per set; the old `Tooling.press` stays for stand-ins.

New checks in sim.rs, added as new bodies to the same certified B&B (no sampling):
`frame_collision` (part vs side housings incl. the throat, y-limited prisms), `throat` (part depth
behind the tool line inside the housings vs throat depth), `backgauge_axes` (X/R/Z within the
press's published ranges), station-aware `tool_collision` (every station's tools are y-limited
prisms at their places; the part is checked against all of them, not only the working one).
A step's existing checks are unchanged for the existing (single station, no frame) cases.

## 3. Stations along the bed

`Setup { press, stations: [{ y, punch, die, segments: [lengths] }] }`. Default: one centred
station sized by the existing segment rule; the planner may add stations (one per distinct
punch/die pair it needs, left to right, gap ≥ the parts' room) and chooses a station per step.
Station layouts are saved in the part's project (`bend/<part>.setup.toml`), editable.

## 4. The environment (interface/forge-render/src/shell/bendsim/)

Content for ENVIRONMENTS' `EnvId::BendSim` (`impl Environment`); the pop-up is removed.
- **Setup tab**: machine picker (library, filter by maker / force / length, provenance shown per
  value with its badge), tooling picker per station (filter by clamp system matching the press,
  V for the thickness, kind), station layout along the bed (drag, segments built from the tool's
  available lengths), "plan" fills it.
- **Sequence tab**: the steps (drag to reorder, lock a step's station/orientation, planner's
  suggestion beside each with why), re-plan from here.
- **Simulate**: one timeline, steps as blocks; play / pause / step / scrub across all steps and
  within a step. Phases per step: handle (operator flips/rotates the part to the new pose: shown,
  not collision-checked, said so), gauge (backgauge X/R/Z move, part pushed to the fingers),
  approach (ram to pinch), bend (φ 0 → θ + overbend, the exact refold), springback (to θ),
  return (ram up), unload. Ram, backgauge axes and the part animate from the same numbers.
- **Results**: per step every check (pass / fail / not checked), tonnage vs tool and press limit,
  overbend and springback, collisions: the B&B witness drawn at its φ (part face and tool body in
  red, witness point and distance label); click jumps the timeline to that φ and frames it.
  Flat-pattern link: jumps to the FlatPattern environment with the bend line highlighted.
- **3D**: forge-brake gains `scene.rs`, renderer-independent: at (step, t) it returns bodies
  `{ name, mesh (triangles, normals), appearance id, highlight }` for the part (the refold's slabs
  and shells tessellated to a chord tolerance), tools (extruded exact sections), press bodies and
  fingers. The shell draws them through the viewport's opaque world-triangle path (CAM-SIM's
  `Overlay::solid`, cam/sim branch) now, and through RENDER-STUDIO's real-time preview path
  (appearances, split-sum IBL, shadows) when it lands: appearance ids `tool-steel-ground`,
  `paint-machine-gloss` (frame, colour per maker's published livery = UNKNOWN, a neutral grey),
  and the sheet's material mapped by forge-material name. Asks to RENDER-STUDIO: those ids in
  appearances.toml; the preview path's entry point for a list of (mesh, transform, appearance);
  floor shadow and AO in it.
- Browser and native run the same code (no threads needed: plan once, scene meshes per frame are
  cheap; the planner runs off the frame as today).

## 5. Library builder (MACHINE-BUILDER)

forge-brake exports a field schema (`schema.rs`): per record kind (punch, die, press) the fields
`{ path, kind: number|text|choice|profile|segments, unit, range, required, help }` and which are
working geometry (must be published or exact estimate). MACHINE-BUILDER's shared builder renders
the form and writes `bend/tooling/*.toml` / `bend/presses/*.toml` with cites (user-entered values
cite as `status = "published"`, `source = "user"`). I send the schema once it is in code.

## 6. Tests

- every built-in record parses, validates, and has a cite per value (real = true); no unknown
  working geometry in a record the planner can pick;
- certified results unchanged: forge-brake/tests and forge-lang/tests/bendsim.rs as they are;
- end to end: examples/sheet tray on a real press with real tooling (feasible, with its plan), and
  a deliberate collision (a box whose last flange closes on a straight punch from the real set:
  fails `closes_on_punch` with the witness), and a frame/throat case;
- scene: meshes closed, at t = end of a step the part's mesh matches the refold's bodies;
- screenshots: native (headless renderer) and browser (web/tests) under research/bendsim/env/.

## 7. Order of work

1. data format + parser + provenance checks (kernel), research data → TOML;
2. press bodies, stations, new checks (kernel; speed band run, though forge-brake is off the
   strict lines);
3. scene.rs; 4. the environment UI on ui/environments (+ cam/sim's solid path);
5. schema for MACHINE-BUILDER; 6. screenshots, gates, merge-train request.

Open: (a) base for the UI work: ui/environments and cam/sim are not on main yet; I build on main
and merge both branches into mine (no force-push), or wait, as the coordinator decides.
(b) livery colours: unknown unless the maker publishes them; grey until then.

## 8. The bending program editor (David: "with proper tools to reorder and things. sim + manufacturing editor")

The environment is a press-brake **program editor**, not a viewer. Its source of truth is a
companion text document, like CAM's `machining` document and URDF's robot document; the UI only
writes edit ops to it, with undo/redo; every edit re-simulates (certified sim per step) and the 3D
view and timeline follow.

```
bending
# load_tray.fg on a TruBend 5130
part   = part("load_tray.fg", sheet: "tray")
press  = press("trumpf-trubend-5130", crowning: auto)
s1     = station(punch: "trumpf/OW200-S", die: "trumpf/EV_W16-84deg", y: -400,
                 punch_segments: (200, 100, 50), die_segments: (250, 100), horns: (left))
s2     = station(punch: "trumpf/OW222-K", die: "trumpf/EV_W16-84deg", y: 600, length: 300)
b1     = bend("right_lip", at: s1, flip: false, rotate: false,
              gauge: (x: 42.5, r: 0, z: (-100, 100), edge: "right_lip.side"), angle: 90deg)
b2     = bend("left_lip", at: s1, rotate: true)
b3     = bend(("right", "left"), at: s2)          # two bends in one stroke (merged step)
sheet  = setup_sheet(pictures: true)
```

- **Statements** (a closed namespace, as `machining`'s): `part`, `press`, `station`, `bend`,
  `setup_sheet`, `program`. Steps run in statement order: reordering = moving `bend` lines.
- **Edit ops**: the existing ones (MoveFeature, AddFeature, DeleteFeature, SetArg, Rename,
  Undo/Redo) on the document's statements; split/merge = replacing one `bend` with two / two with
  one (ReplaceFeature). No new op kinds.
- **Per step**: the bend(s), station, orientation (flip, rotate end for end, shift along the
  station), backgauge (X, R, Z1/Z2, which fingers, which gauge edge), the angle target and
  overbend, crowning, manual overrides; an override a check refuses stays, shown failing.
- **Planner**: suggestions are offered beside each step and as whole ranked programs (fewest
  flips, then fewest tool changes / stations, then search order); applying one is an explicit
  edit, never silent.
- **Tooling layout editor**: stations dragged along the bed, segment stacks built from the tool's
  published lengths (split to the needed length, horns), checked against the bed length, housing
  gap and tonnage per station (kN/m x stack length vs the tools' and the press's limits).
- **Results** per step: every check, collisions and limit violations listed with click-to-locate
  (jump the timeline to the witness's phi, frame it).
- **Handling** between steps checked for collision: the part's envelope swept through the flip /
  rotation the operator makes (lifted clear of the die, turned about its own centre), against the
  press bodies; reported as its own rule (`handling`), a warning where the motion is the
  operator's choice.
- **Outputs**: a setup sheet (tool layout, step list with a picture per step, as HTML/PNG); a
  press-brake program only in a format whose specification is openly published — none of the
  controls' formats (Trumpf TASC/TruBend, Amada AMNC, Bystronic ByVision, Delem DA) is openly
  documented as far as I know (to be checked, cited): until one is, `program(..)` answers the
  typed error `ProgramFormatUnavailable` with what was looked for.
- **Language proofs**: the `bending` header and the `bend` built-in family (closed namespace)
  get rows in research/forge-lang/lean/COVERAGE.md in the same change (LANG-PROOFS proves them),
  as `machining`/`cam` have: header row + builtins row + args GAP row.

Crate: `interface/forge-bend-lang` (as forge-cam-lang): parse the document with forge-lang's
parser, evaluate the closed namespace into (Model, Tooling with stations, the steps as
`Setup`s and overrides), run forge-brake's `simulate` on the given sequence, answer
`forgecad.bending/1`. forge-cli routes a `bending` document to it (`fg.rs`), MCP the same.

Order of work (coordinator, 2026-11-24): (1) collision-checked handling; (2) ranked automatic
sequence search; then the `bending` document + editor UI on it; (3) RENDER-STUDIO appearances;
(4) presses and tools as a Kind in MACHINE-BUILDER's forge-library builder.
