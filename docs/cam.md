# CAM: machining documents

A machining document is a `.fg` file whose first statement is the bare name `machining`. It
names a part, a machine, tools, setups and operations. Every toolpath it makes is posted to NC
text, read back, and the path the text means is **certified** against the exact part:
- **gouge**: the cutter stays within `tol` of the part on cuts;
- **holder_collision**: the shank and holder keep `clearance` from the part and the stock;
- **rapid_clear**: rapids keep `clearance` from the part and the stock;
- **fixture_collision**: every body of the tool, the cutter too, keeps the fixture's clearance
  (default 1 mm) from the vise, on every move;
- **axis_limit** (G5): every posted position within the machine's travel, joint by joint (the
  spindle's Z is the tip's plus the tool's length: stick-out and holder gauge), for a setup placed
  with `at:` (the work offset in machine coordinates); an unplaced setup has each axis's span
  checked against its travel, and the reply's warnings say the positions were not.
- **machine_limit** (G5, runtime): every spindle speed (S) the posted text commands within the
  machine's `rpm` range, and every feed (F) at most its `max_feed`, as read back from the text
  (decided exactly: the posted integers against the limits scaled in outward-rounded arithmetic).
  Placed or not.

An operation whose checks are not all proven is **refused**, with the check that failed, where,
and a fix. A program is written only when every operation in it is certified.

```
forgecad cam F.fg [--json] [-o PROGRAM]     the reply, and the program if released; exit 0 certified
                                             in full, 3 released but certified except G5 (a setup
                                             not placed with `at:`), 1 refused
forgecad cam F.fg sheet -o SHEET.html        the setup sheet
```
The document layer is `interface/forge-cam-lang` (`forge_cam_lang::run`); forge-lang itself only
gains `hole_view`, so a part's build links and runs none of CAM.
Over MCP the tool is `cam` (`file`, and `what: sheet`).

## The document

```
machining
shared tol = 0.01
f     = part("flange.fg", group: "N")          # the part; its ISO 513 material group (P M K N S H)
m     = machine("generic-vmc3")
t1    = tool("flat-10", t: 1, stickout: 32, holder: "bt40-er32")
op1   = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: "top-center", tol: tol)
skim  = facing(op1, tool: t1, to: 18, step: 1)
top   = clear(op1, tool: t1, from: 18, to: 12, step: 3, stepover: 4)
prof  = contour(op1, tool: t1, side: "outside", from: 12, to: -0.5, step: 4)
bore  = pocket(op1, f.bore_h, tool: t1, stepover: 2.5, step: 3)
bolts = drill(op1, f.bolts, plan: "auto")
prog  = program(op1, post: "fanuc", number: 1001)
```

| built-in | arguments |
|---|---|
| `part(FILE, group:)` | a part file beside the document; `group` (default `N`) picks the cutting data |
| `machine(NAME)` | from the library (`generic-vmc3`, `generic-router`, or the project's) |
| `tool(ID, t:, stickout:, holder:)` | a library tool, its T number, stick-out (tip to holder nose), holder |
| `setup(PART, machine:, stock:, origin:, wcs:, at:, tol:, clearance:, clear:, retract:, fixture:, from:)` | `stock: box(margin: (x, y, z))` or `box(min:, max:)`; `origin: "top-center"`, `"top-corner"` or `(x, y, z)` in the part's frame; heights above the stock top; `from: SETUP`: the part turned over about x, starting from the stock that setup provably left (turned) |
| `vise(NAME, across:, clamp:, parallel:, clearance:)` | a library vise (`generic-vise-150`, `generic-vise-100`) closed on the stock along `across` (`"x"` or `"y"`), gripping `clamp` mm of it, on parallels `parallel` wide (default 6); `clearance` default 1 |
| `soft_jaw(vise(NAME, across:), blank:, from: SETUP, grip:, fit:, tool:, relief: "corners", min_contact:)` | soft jaws cut to what `from:` left, turned over, `grip` deep: the pocket is the opening of that outline by `tool`'s radius, grown by `fit` (default 0.02), with a relief of radius r + fit at each corner if asked. Refused unless what `from:` left seats in the pocket (exactly), the seat is at least `min_contact` mm² (default a quarter of the largest section), the blank keeps 3 mm of wall and the part stands more than the fixture clearance above the jaws. The value is a part (the jaws closed, pocketed: `setup(jaws, ..)` cuts them, held by the vise) and a fixture (`setup(f, from: op1, fixture: jaws)`) |
| `facing(SETUP, tool:, to:, step:)` | zig-zag over the stock down to `to` |
| `clear(SETUP, tool:, from:, to:, step:, stepover:, leave:, ramp:)` | everything between the stock outline and the part's section, as offset passes |
| `contour(SETUP, tool:, side:, from:, to:, step:, leave:)` | the part's outline (`side: "outside"`) or its holes (`"inside"`) at the section |
| `pocket(SETUP, PART.HOLE, tool:, stepover:, step:, leave:, ramp:)` | a hole feature's circle, to its depth |
| `pocket(SETUP, tool:, from:, to:, stepover:, step:, leave:, ramp:)` | every hole and slot of the part's section between the heights, each less the part's section there (islands standing in it, a boss in a well, are kept: examples/machined/well_island_cam.fg) |
| `slot(SETUP, PART.SKETCH, tool:, depth:, step:, from:, ramp:)` | an end mill (flat or bull) along the sketch's lines, arcs and circles (not construction), on a plane facing +z, in levels down to `depth`: the groove the tool's width, modelled in the part (G1 against the part). `ramp: DEG` enters along each curve's first line, back and forth at that angle from the level above; else a plunge. The stock records each curve's band of the tool's radius as cleared (above the corner radius for a bull). examples/machined/slot_plate_cam.fg |
| `engrave(SETUP, PART.SKETCH, tool:, depth:, step:, from:)` | the tool's tip along the sketch's lines, arcs and circles (not construction), on a plane facing +z, in levels down to `depth` below it. The groove must be in the model (G1 is checked against the part, never a blanket tolerance); the stock records each line's and circle's band of the tip's radius as cleared (a V tool clears more, so G4 may list the rest of the groove) |
| `thread_mill(SETUP, PART.HOLE, tool:, length:)` | a hole's recorded thread (`thread:`, or `screw:` with `fit: "tap"`), the model not carrying it: declares a cosmetic thread Δ per place (the ring from the bore to the major diameter over `length`, default the depth less two pitches), listed in the op's `removals`, and is certified against P ∖ Δ (never a blanket tolerance). Down the bore, in to the radius, up a helix of the pitch (G3: right hand), out |
| `drill(SETUP, PART.HOLE, plan: "auto")` | the plan from the hole feature: spot, drill (peck past 3 D), tap for a thread; `plan: "drill"` leaves the thread (to `thread_mill`) |
| `program(SETUP, post:, number:)` | `fanuc`, `haas`, `linuxcnc`, `grbl` (default: the machine's control) |

`ramp: DEG` (over 0, at most 30) enters each pass by a helix of half the tool's radius at that angle,
about the pass's start, where that disk lies in the tool centres' region (decided exactly); else
the pass is plunged. A helix is posted as G2/G3 with Z and checked as its arc swept over each
piece's heights (a superset), in closed form against prism faces; next to any other face it is
refused as undecided.

Holes are read from the part's history (`hole(d:, depth:, thread:, ...)`), never recognised from
the shape: `f.bolts` is the `bolts` hole feature with its diameter, depth, thread and fit.

Two setups and soft jaws (examples/machined/bracket_cam.fg): op1 holds the stock in a vise and cuts
the top and the outline down to just above the jaws; `soft_jaw(.., from: op1)` makes jaws to what
op1 left; `opj` cuts the jaws' pocket; op2 holds the part turned over in them and cuts the rest.
Edit the part and every step follows: the jaws are recomputed from the new outline.

## Libraries

Built in: generic tools, conservative holders, generic machines, generic vises and soft-jaw blanks
(`[[vise]]`, `[[jaw_blank]]`, kernel/forge-tooling/data/fixtures.toml) and per-group cutting data, all
our own numbers, marked "verify with the vendor". A project adds its own in `cam/*.toml` beside
the document (formats: research/cam/libraries-tools.md §5). A Fusion tool library converts with
`forge_cam_import::fusion` (interface/forge-cam-import): our TOML, read back before it is written.

## The reply: `forgecad.cam/1`

```
ok, schema: "forgecad.cam/1", certification (full | except G5 | refused), errors[], warnings[],
setups[]   name, part, machine, wcs {origin, axes, offset}, stock {kind, min, max}, tol, clearance, at,
           heights {clear, retract}, fixture {kind, name, across, jaw_top, clamp, clearance,
           pieces[] {name, min, max}} | null
ops[]      name, kind (facing|clear|contour|pocket|drill|engrave|slot|thread_mill|edge_break), setup, status (built|refused),
           tool {id, T, H, type, d, holder, stickout} | null, plan[] (hole plans),
           feeds {rpm, vc, fz, hex, f, power_kw} | null,
           path {moves, length_cut, length_rapid, time_s},
           certificate {verdict (pass|fail), certified, pairs, cells, least_cutter_distance, against},
           checks[] (failures only) {rule, verdict (fail|undecided), certified, value, limit, unit,
                    at, tool_part (cutter|shank/holder), other, phase (cut|plunge|link|rapid), move, path (which of a hole plan's paths `move` counts in)},
           removals[] (declared Δ, e.g. cosmetic threads), leftover {status}, refusal | null
blocking[] {op, why}      fixes[] {op, fix}      suggestions[] {kind: NeedTool, op, why}
programs[] {name, post, lines, round_trip, certified (full | except G5 (unchecked: no `at:`) | no),
           error | null}
leftover   {computed, rule, regions[] {setup, z, boxes, why, r, below}}: decided at the end of each
           chain of setups (a setup another continues with `from:` hands its stock on). `why`, each
           decided exactly, else "uncut" (which claims nothing):
           - "held by fixture": in the jaws' grip, within the fixture's clearance of a piece;
           - "below every tool tip": the band's top is at or under `below`, the lowest tip of
             every posted move of the setup (a setup that continues none);
           - "tool radius": at the band's own section, no disk of radius `r` (the chain's smallest
             tool) inside the free region reaches it (inside corners, narrow slots). Computed as
             the opening at r − 1e-6 grown back by r + 1e-6, a superset of the opening by r over
             the offsets' 2⁻³² mm grid rounding; no claim where the centres' region has a
             one-dimensional part
scope      what is proven and what is not
```

The `scope` line is the contract's honesty: cutting physics, the machine's fidelity, and the
leftover (G4) are not claimed in this phase. The in-process stock used for G2 and G3 is the box
minus what each op provably cleared (kernel/forge-cam/src/stock.rs), so it contains the true one.

## What this phase covers

2.5D on parts whose faces are planes, and cylinders, cones and tori about z (others are refused by kind, typed):
facing, clearing, contours, hole pockets and hole plans, with linking by retract. Not yet:
adaptive clearing (patents first), 3-axis surfacing, 3+2, turning, 5-axis; chamfer mills,
threads modelled as helicoidal faces (`thread(hole:)`): the lowering refuses them, so a modelled thread is milled as a declared Δ only when the model records it instead.
