# CAM: machining documents

A machining document is a `.fg` file whose first statement is the bare name `machining`. It
names a part, a machine, tools, setups and operations. Every toolpath it makes is posted to NC
text, read back, and the path the text means is **certified** against the exact part:
- **gouge**: the cutter stays within `tol` of the part on cuts;
- **holder_collision**: the shank and holder keep `clearance` from the part and the stock;
- **rapid_clear**: rapids keep `clearance` from the part and the stock;
- **fixture_collision**: every body of the tool, the cutter too, keeps the fixture's clearance
  (default 1 mm) from the vise, on every move; with no workholding declared, from the support
  under the stock (the plane of the stock's bottom): nothing cuts below the stock;
- **axis_limit** (G5): every posted position within the machine's travel, joint by joint (the
  spindle's Z is the tip's plus the tool's length: stick-out and holder gauge), for a setup placed
  with `at:` (the work offset in machine coordinates); an unplaced setup has each axis's span
  checked against its travel, and the reply's warnings say the positions were not.
- **machine_limit** (G5, runtime): every spindle speed (S) the posted text commands within the
  machine's `rpm` range, and every feed (F) at most its `max_feed`, as read back from the text
  (decided exactly: the posted integers against the limits scaled in outward-rounded arithmetic).
  Placed or not.

An operation whose checks are not all proven is **refused**, with the check that failed, where,
and a fix. A program is written only when every operation in it is certified. Every tool change is
preceded by Z to the machine's safe position (the dialect's `retract_home`: `G91 G28 Z0.` on
Fanuc and Haas, `G53 G0 Z0` on LinuxCNC; GRBL, which changes tools by hand, goes to the setup's clearance height in work coordinates and tells the operator to set Z for the new tool).

```
forgecad cam F.fg [--json] [-o PROGRAM]     the reply, and the program if released; exit 0 certified
                                             in full, 3 released but certified except G5 or G4 (a setup
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
| `setup(PART, machine:, stock:, origin:, wcs:, at:, tol:, clearance:, clear:, retract:, fixture:, from:)` | `stock: box(margin: (x, y, z), below: b)` (b mm of stock under the part, default 0) or `box(min:, max:)`, a bar `cylinder(d:, margin: (radial, above), below:)` or a tube `tube(d:, id:, margin:, below:)` along z about the part's centre (no `d:`: the part's farthest reach plus the radial margin), or the stock as its own part file `solid("blank.fg")` (a prism along z, in the part's frame); a shaped stock must hold the part, shown band by band between its flat faces (exact booleans), or the setup is refused; `origin: "top-center"`, `"top-corner"` or `(x, y, z)` in the part's frame; heights above the stock top; `from: SETUP`: the part turned over about x, starting from the stock that setup provably left (turned) |
| `vise(NAME, across:, clamp:, parallel:, clearance:)` | a library vise (`generic-vise-150`, `generic-vise-100`) closed on the stock along `across` (`"x"` or `"y"`), gripping `clamp` mm of it, on parallels `parallel` wide (default 6); `clearance` default 1 |
| `soft_jaw(vise(NAME, across:), blank:, from: SETUP, grip:, fit:, tool:, relief: "corners", min_contact:)` | soft jaws cut to what `from:` left, turned over, `grip` deep: the pocket is the opening of that outline by `tool`'s radius, grown by `fit` (default 0.02), with a relief of radius r + fit at each corner if asked. Refused unless what `from:` left seats in the pocket (exactly), the seat is at least `min_contact` mm² (default a quarter of the largest section), the blank keeps 3 mm of wall and the part stands more than the fixture clearance above the jaws. The value is a part (the jaws closed, pocketed: `setup(jaws, ..)` cuts them, held by the vise) and a fixture (`setup(f, from: op1, fixture: jaws)`) |
| `facing(SETUP, tool:, to:, step:, leave_axial:)` | zig-zag over the stock down to `to` |
| `clear(SETUP, tool:, from:, to:, step:, stepover:, leave:, leave_axial:, ramp:)` | everything between the stock outline and the part's section, as offset passes |
| `contour(SETUP, tool:, side:, from:, to:, step:, leave:, leave_axial:, direction:, finish:, finish_step:, spring:, lead:, lead_radius:, lead_sweep:, lead_length:, entry:, overlap:, tabs:, tab_spacing:, tab_at:, tab_width:, tab_height:, tab_shape:)` | the part's outline (`side: "outside"`) or its holes (`"inside"`) at the section |
| `pocket(SETUP, PART.HOLE, tool:, stepover:, step:, leave:, leave_axial:, ramp:)` | a hole feature's circle, to its depth |
| `pocket(SETUP, tool:, from:, to:, stepover:, step:, leave:, leave_axial:, ramp:)` | every hole and slot of the part's section between the heights, each less the part's section there (islands standing in it, a boss in a well, are kept: examples/machined/well_island_cam.fg) |
| `slot(SETUP, PART.SKETCH, tool:, depth:, step:, from:, ramp:)` | an end mill (flat or bull) along the sketch's lines, arcs and circles (not construction), on a plane facing +z, in levels down to `depth`: the groove the tool's width, modelled in the part (G1 against the part). `ramp: DEG` enters along each curve's first line, back and forth at that angle from the level above; else a plunge. The stock records each curve's band of the tool's radius as cleared (above the corner radius for a bull). examples/machined/slot_plate_cam.fg |
| `engrave(SETUP, PART.SKETCH, tool:, depth:, step:, from:, offset:, axial_offset:)` | the tool's tip along the sketch's lines, arcs and circles (not construction), on a plane facing +z, in levels down to `depth` below it. The groove must be in the model (G1 is checked against the part, never a blanket tolerance); the stock records each line's and circle's band of the tip's radius as cleared (a V tool clears more, so G4 may list the rest of the groove). Trace: `offset:` moves the tool off each curve to its left (> 0) or right (< 0) as it runs (a line along its normal, an arc or circle to the concentric radius; corners between curves are not joined), `axial_offset:` raises (> 0) or lowers the tip; `slot` takes both too. The moved path is certified against the part like any other |
| `thread_mill(SETUP, PART.HOLE, tool:, length:, passes:, spring:, hand:, starts:, lead:)` | a hole's recorded thread (`thread:`, or `screw:` with `fit: "tap"`), the model not carrying it: declares a cosmetic thread Δ per place (the ring from the bore to the major diameter over `length`, default the depth less two pitches), listed in the op's `removals`, and is certified against P ∖ Δ (never a blanket tolerance). Down the bore, in to the radius, up a helix of the pitch (G3: right hand), out. `passes: n` radial passes splitting the depth from the bore to the major evenly (default 1), `spring: n` the last again, `hand: "left"` climbs down from the top (G3), `starts: n` a helix of n pitches per turn once per start, each a pitch higher, `lead: "arc"` in and out on a half circle tangent to the helix (default `"line"`) |
| `bore(SETUP, PART.HOLE, tool:, pitch:, ramp:, finish_leave:, spring:, leave:, leave_axial:)` | helical bore of a round hole along −z: the tool's centre on the hole's radius less its own, down a helix of `pitch:` per turn (default a 3° ramp; or `ramp:` degrees, at most 30), G3 (climb), a full circle at the bottom, back to the axis, up. `finish_leave:` leaves that much for a second helix at the final radius; `spring: n` repeats it. The tool must be at least half the hole across (no core left), else refused with the fix (`pocket(..)` the hole). Certified against the part |
| `edge_break(SETUP, tool:, size:, tip_offset:, at:)` | chamfer/deburr of the part's top edges with a chamfer mill or spot drill of 30° to 150°: a chamfer `size` wide at half the tool's angle from the wall (45° for a 90° tool), its tip `tip_offset:` below the chamfer's bottom (default 0.2) so the flank, not the tip, cuts. The removal is declared (Δ, in `removals`) and the op certified against P ∖ Δ. In this phase: the part's top, over vertical walls; sharp corners between lines only |
| `drill(SETUP, PART.HOLE, plan: "auto")` | the plan from the hole feature: spot, drill (peck past 3 D), then its counterbore (a counterbore tool of its diameter, else the largest end mill under it pocketing the circle) or countersink (a countersink, chamfer or spot tool of its angle, plunged until the cone is its diameter at the mouth), tap for a thread; `plan: "drill"` leaves the thread (to `thread_mill`). A blind hole with a flat floor is drilled with the drill's point on the floor; the ring that leaves is said |
| `zlevel(SETUP, tool:, leave:, from:, to:, step:, stepover:, area:, grid:)` | 3-axis roughing, offset clearing (not adaptive): at each level from `from` (the stock's top) down to `to`, `step` apart (and at the part's own top), the tool centres the drop-cutter leaves free on a CL grid `grid` apart, cleared by passes `stepover` apart (at most the tool's radius) from that region's boundary; `area: (x0, y0, x1, y1)` bounds the tool centres. Its flat bottom's sweep is recorded in the in-process stock |
| `parallel(SETUP, tool:, stepover:, angle:, sample:, leave:, area:, floor:)` | 3-axis finishing: zig-zag rows `stepover` apart at `angle` degrees, each dropped onto the part (samples at most `sample` apart, split until each move is proven) |
| `waterline(SETUP, tool:, from:, to:, step:, area:, grid:)` | constant-z contours where the tool's reach crosses each level (marching squares on a CL grid, each crossing by certified bisection) |
| `pencil(SETUP, tool:, area:, grid:, kink:, min_len:)` | along the CL surface's creases, where the tool touches two faces at once (the double-contact locus): concave corners and fillets tighter than the tool. `kink` in 1/mm (a CL second difference: a local minimum across a jump in slope of at least that), default 1/D; `min_len` drops shorter runs |
| `scallop(SETUP, tool:, cusp: \| stepover:, walls:, area:, from:, to:, grid:)` | constant stepover measured along the part: geodesic parallels, the level sets of the distance over the CL surface from the area's edge, or from walls steeper than `walls` degrees; `cusp:` derives the stepover |
| `rest(SETUP, tool:, after: TOOL, after_leave:, min:, cusp: \| stepover:, sample:, margin:, angle:, area:, grid:)` | parallel rows only where `after` (an earlier tool of any end: flat, ball, bull) left more than `min` (default 0.05) that this tool reaches, from the two cutters' lower envelopes; grown by `margin` |
| `steep_shallow(SETUP, tool:, cusp: \| (step:, stepover:), threshold:, overlap:, angle:, area:, from:, to:, grid:)` | waterlines where the surface is steeper than `threshold` (default 45°), parallel rows where it is shallower, each extended `overlap` over the other. With `cusp:` the step-down is d·sin θ and the stepover d·cos θ, d the largest spacing proven to leave at most that cusp, so the cusp holds on both sides of the threshold (on planes; curvature changes it to second order) |
| `spiral(SETUP, tool:, cusp: \| stepover:, center: (x, y), r0:, r1:, direction: "out" \| "in", area:, from:, to:)` | an Archimedean spiral about a point, `stepover` apart per turn, dropped onto the part |
| `radial(SETUP, tool:, cusp: \| stepover:, center: (x, y), r0:, r1:, area:, from:, to:)` | spokes about a point, at most `stepover` apart at `r1`, dropped onto the part |
| `ramping(SETUP, tool:, cusp: \| step:, slope_min:, area:, from:, to:, grid:)` | waterline-like passes that descend continuously from one level to the next (no step-down mark); with `cusp:` the step-down is proven at slopes down to `slope_min` (default 30°) |
| `flat(SETUP, tool:, cusp: \| stepover:, angle:, area:, from:, to:, grid:)` | the flat areas: the heights where the CL surface is flat (found and listed in the plan), each finished by rows at exactly that height where the tool stands on the face |

The 3-axis ops share the 2D ops' meaning of `step:`: the step-down (zlevel, waterline,
steep_shallow, ramping; `down:` is its old name, accepted until the next train with a warning;
both given is refused, and `parallel` refuses `step:`, which used to mean its sample spacing).
The finishing ops (parallel to flat) share: `leave:` (stock to leave), `floor:` (lowest
tip height), `sample:` (the spacing of the drop-cutter's samples along a pass), `stay:` (the longest link that stays down,
proven above the surface) and `lift:` (a lifted link's height over the surface); otherwise a
link retracts. Every move is certified like any op. With `cusp:` the reply's `finish` gives the
cusp asked, the cusp the spacing is proven to bound, the stepover, the step-down and the links
made.
| `program(SETUP or [SETUPS], post:, number:, ops:)` | `fanuc`, `haas`, `linuxcnc`, `grbl` (default: the machine's control); `ops: OP` or `[OPS]` posts only those ops of its setups. Each op is certified on the stock every earlier op of its setup (and of the setup it continues) leaves, so those are posted first, by an earlier program or earlier in this one, or the program is refused with the fix; an op no program posts is not certified (`unposted`). `decimals:` and `feed_decimals:` set the program's number formats (the text is quantised, read back and certified at them), `coolant:` (`"flood"`, `"mist"`, `"through"`, `"off"`) every path's coolant, refused if the machine has none of it. `split: "setup"`: one program per setup, named `PROGRAM/SETUP` and numbered on from `number:` (the reply's program has its `setup`) |

`ramp: DEG` (over 0, at most 30) enters each pass by a helix of half the tool's radius at that angle,
about the pass's start, where that disk lies in the tool centres' region (decided exactly); else
the pass is plunged. A helix is posted as G2/G3 with Z and checked as its arc swept over each
piece's heights (a superset), in closed form against prism faces; next to any other face it is
refused as undecided.

`leave:` is the radial stock to leave (on walls: each pass's offset grows by it) and
`leave_axial:` the axial stock (on floors: the op's lowest level, `to:` or a hole's depth, is
raised by it), both default 0, either may be negative; `facing`, `clear`, `contour` and `pocket`
take `leave_axial:`, the others refuse it. G4 grows the part by the larger of the two.

A contour's `direction:` is `"climb"` (the part on the tool's right, with the spindle M03) or
`"conventional"` (default climb, as pockets and clearing are). `finish: n` (default 1) cuts n radial passes at each level,
`finish_step:` apart (default a tenth of the tool's diameter), the last at the contour's own offset;
`spring: n` (default 0) repeats the last pass n times. Only `contour` takes these and the leads below.

A contour's leads (Fusion's Linking tab): `lead: "arc"` joins each loop at each level on a
circular arc tangent to it (G1-continuous) of `lead_radius:` (default half the tool's diameter)
through `lead_sweep:` degrees (default 90, at most 180), preceded by a tangent line of
`lead_length:` (default 0), and leaves it the same way; `lead: "line"` is the tangent line alone
(`lead_length:`, default half the tool's diameter). Leads lie on the air side (with climb, the
tool's left). With leads the tool comes down at the lead-in's start, off the wall; after each level's
lead-out it goes back at that level to the lead-in's start and down to the next (staying down
through the levels), and up at the end. `entry: (x, y)` starts each loop at its point nearest that point (the piece
there split); `overlap:` cuts on past the start by that length before the lead-out, to hide the
witness mark. Default `lead: "none"`: the path is the one without these. Every lead is certified as
any move is; a lead that does not fit (an arc bigger than the hole it is in) is refused with the
gouge it makes, so make `lead_radius:` smaller. Method: plain tangent arcs and lines.

Holding tabs: `tabs: n` per loop evenly spaced (the first half a spacing from the loop's start),
or `tab_spacing:` (one every that length along the loop), or `tab_at: [(x, y), ..]` (one at each
loop's point nearest each point). `tab_width:` is the bridge (default the tool's diameter; the tool
lifts over the width plus its diameter), `tab_height:` its height above the contour's bottom
(default 1, under the depth), `tab_shape: "rect"` (straight up and down, default) or `"triangle"`
(ramped up to the middle and down). Only the levels below the tabs' top lift. Tabs that overlap,
or reach the loop's start (where it comes down; move them or `entry:`), are refused. The stock
counts the band an outside contour sweeps as cleared only down to the tabs' top.

Heights (`from:`, `to:`) are numbers in the part's frame, or the model's own: a horizontal face or
plane of the part named as in the part's text (`f.plate.end`, `f.boss.end`, `f.plate.start`),
plus or minus a length along the setup's up (`to: f.plate.end + 0.5`). In a setup turned over
(`from:`), the same reference is read in the turned frame. A reference that is not horizontal, or
in any other argument, is refused.

Every built-in takes the arguments listed for it in forge-cam-lang's `ARGS` (the table above);
any other name is refused (`UnknownArg`), with the nearest as the fix: `stepovr` is "did you mean
`stepover`?".

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
ok, schema: "forgecad.cam/1", certification (full | except G5 | full except G4 (leftover not computed) | except G4 and G5 (..) | refused), errors[], warnings[],
setups[]   name, part, machine, wcs {origin, axes, offset}, stock {kind, min, max}, tol, clearance, at,
           heights {clear, retract}, fixture {kind, name, across, jaw_top, clamp, clearance,
           pieces[] {name, min, max}} | null
ops[]      name, kind (facing|clear|contour|pocket|drill|engrave|slot|thread_mill|edge_break|bore), setup, status (built|refused),
           tool {id, T, H, type, d, holder, stickout} | null, plan[] (hole plans),
           feeds {rpm, vc, fz, hex, hm, f, power_kw, torque_nm} | null,
           path {moves, length_cut, length_rapid, time_s},
           certificate {verdict (pass|fail), certified, pairs, cells, least_cutter_distance, against},
           checks[] (failures only) {rule, verdict (fail|undecided), certified, value, limit, unit,
                    at, tool_part (cutter|shank/holder), other, phase (cut|plunge|link|rapid), move, path (which of a hole plan's paths `move` counts in)},
           removals[] (declared Δ, e.g. cosmetic threads), leftover {status}, refusal | null
blocking[] {op, why}      fixes[] {op, fix}      suggestions[] {kind: NeedTool, op, why}
programs[] {name, post, lines, round_trip, certified (full | except G5 (unchecked: no `at:`) | full except G4 (leftover not computed) | except G4 and G5 (..) | no),
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

3-axis surfacing (research/cam/phase3.md): a part outside the 2.5D lowering (spheres; cylinders,
cones and tori of any axis or trimmed by other curves) is lowered with those faces as supersets
(the whole sphere or torus): sound for every check, stricter, and a concave such face may refuse
an op. Each surfacing move is certified by the tool's column (drop.rs): its signed distance to the
part over the move, by closed forms for planes, vertical walls, spheres, cylinders with a
horizontal axis and tori about a vertical axis, else by branch and bound. G4 (leftover) is not
computed for such parts (`leftover.computed: false`). examples/machined/mold_core_cam.fg: z-level
roughing, parallel, waterline, pencil and rest on a filleted core, every move certified.

## What this phase covers

2.5D on parts whose faces are planes, and cylinders, cones and tori about z (others are refused by kind, typed):
facing, clearing, contours, hole pockets and hole plans, with linking by retract. Not yet:
adaptive clearing (patents first), 3+2, turning, 5-axis; chamfer mills,
threads modelled as helicoidal faces (`thread(hole:)`): the lowering refuses them, so a modelled thread is milled as a declared Δ only when the model records it instead.

## In the app: the Manufacture workspace

Open a machining document in the app (natively or in the browser) and it opens on the
**Manufacture** tab, with its part in the view. The document's text stays the source of truth:
the browser, the dialogs and the ribbon make the same edit ops an agent sends (`SetArg`,
`AddFeature`, `DeleteFeature`, `MoveFeature`, `Suppress`, `Rename`), and what they show is this
document's run, the reply above.

- **Setup ▸ New Setup**: machine, origin, work offset, placement (`at:`), fixture, stock (the
  part's box plus offsets, or a fixed box), heights; a new setup comes with its `program(..)`.
- **2D** and **Drilling**: one dialog per op built-in, with Tool, Geometry, Heights, Passes and
  Linking tabs; a hole or sketch is chosen from the part's history or picked in the view.
- **The browser**: setups, their ops and programs, each certified, refused, not posted, stale
  (edited since it was generated) or suppressed; right-click to edit, move, suppress or delete.
  The selected op's tool, feeds, path and certificate show below it, and its failed checks:
  click one to locate its witness in the view.
- **Generate** (and Auto Generate), **Post Process** (dialect, number, the NC text, Write NC
  File), **Setup Sheet**, **Checks**, **Tool Library** (add a library tool with its T number),
  **Document Text** (the text by hand).

Code: interface/forge-render/src/shell/cam/ and src/gui_cam.rs; notes and screenshots in
research/cam/ui/.

## Tool life (prediction and tracking)

`Cam::tool_life` predicts each loaded tool's life use over the job: Taylor's
v T^n f^a ap^b = C from cited rows (`kernel/forge-tooling/data/life.toml`), summed per
fixed-length sample of the posted path (each sample's own speed, chip and depth; Miner's
rule). The part must name its material (`part("p.fg", group: "S", material: "Ti-6Al-4V")`);
a tool × material with no cited row is "not estimated". A program that would use more life
than the loaded tool has left gets a warning with a change point or a sister tool, never an
error. Physical tools are tracked in `cam/tool-instances.toml` (`forgecad tool log ...`), and
logged worn tools calibrate the shop's own C (and n), shown beside the cited values.
Details, sources and the honest validation: research/cam/toollife/model.md, validation.md.
