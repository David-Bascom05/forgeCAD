# CAM: machining documents

A machining document is a `.fg` file whose first statement is the bare name `machining`. It
names a part, a machine, tools, setups and operations. Every toolpath it makes is posted to NC
text, read back, and the path the text means is **certified** against the exact part:
- **gouge**: the cutter stays within `tol` of the part on cuts;
- **holder_collision**: the shank and holder keep `clearance` from the part and the stock;
- **rapid_clear**: rapids keep `clearance` from the part and the stock.

An operation whose checks are not all proven is **refused**, with the check that failed, where,
and a fix. A program is written only when every operation in it is certified.

```
forgecad cam F.fg [--json] [-o PROGRAM]     the reply, and the program if released
forgecad cam F.fg sheet -o SHEET.html        the setup sheet
fg cam F.fg [--json] [-o PROGRAM] [--sheet SHEET.html]
```
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
| `setup(PART, machine:, stock:, origin:, wcs:, tol:, clearance:, clear:, retract:)` | `stock: box(margin: (x, y, z))` or `box(min:, max:)`; `origin: "top-center"`, `"top-corner"` or `(x, y, z)` in the part's frame; heights above the stock top |
| `facing(SETUP, tool:, to:, step:)` | zig-zag over the stock down to `to` |
| `clear(SETUP, tool:, from:, to:, step:, stepover:, leave:)` | everything between the stock outline and the part's section, as offset passes |
| `contour(SETUP, tool:, side:, from:, to:, step:, leave:)` | the part's outline (`side: "outside"`) or its holes (`"inside"`) at the section |
| `pocket(SETUP, PART.HOLE, tool:, stepover:, step:, leave:)` | a hole feature's circle, to its depth |
| `drill(SETUP, PART.HOLE, plan: "auto")` | the plan from the hole feature: spot, drill (peck past 3 D), tap for a thread |
| `program(SETUP, post:, number:)` | `fanuc`, `haas`, `linuxcnc`, `grbl` (default: the machine's control) |

Holes are read from the part's history (`hole(d:, depth:, thread:, ...)`), never recognised from
the shape: `f.bolts` is the `bolts` hole feature with its diameter, depth, thread and fit.

## Libraries

Built in: generic tools, conservative holders, generic machines and per-group cutting data, all
our own numbers, marked "verify with the vendor". A project adds its own in `cam/*.toml` beside
the document (formats: research/cam/libraries-tools.md §5). A Fusion tool library converts with
`forge_cam_import::fusion` (interface/forge-cam-import): our TOML, read back before it is written.

## The reply: `forgecad.cam/1`

```
ok, schema: "forgecad.cam/1", errors[], warnings[],
setups[]   name, part, machine, wcs {origin, axes, offset}, stock {kind, min, max}, tol, clearance,
           heights {clear, retract}
ops[]      name, kind (facing|clear|contour|pocket|drill), setup, status (built|refused),
           tool {id, T, H, type, d, holder, stickout} | null, plan[] (hole plans),
           feeds {rpm, vc, fz, hex, f, power_kw} | null,
           path {moves, length_cut, length_rapid, time_s},
           certificate {verdict (pass|fail), certified, pairs, cells, least_cutter_distance, against},
           checks[] (failures only) {rule, verdict (fail|undecided), certified, value, limit, unit,
                    at, tool_part (cutter|shank/holder), other, phase (cut|plunge|link|rapid), move},
           leftover {status}, refusal | null
blocking[] {op, why}      fixes[] {op, fix}      suggestions[] {kind: NeedTool, op, why}
programs[] {name, post, lines, round_trip, error | null}
scope      what is proven and what is not
```

The `scope` line is the contract's honesty: cutting physics, the machine's fidelity, and the
leftover (G4) are not claimed in this phase. The in-process stock used for G2 and G3 is the box
minus what each op provably cleared (kernel/forge-cam/src/stock.rs), so it contains the true one.

## What this phase covers

2.5D on parts whose faces are planes and cylinders along z (others are refused by kind, typed):
facing, clearing, contours, hole pockets and hole plans, with linking by retract. Not yet:
adaptive clearing (patents first), 3-axis surfacing, 3+2, turning, 5-axis; slots, chamfer mills,
engraving and thread milling as ops (the tools exist); helical ramps; the leftover check.
