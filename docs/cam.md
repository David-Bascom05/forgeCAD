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
| `adaptive(SETUP, tool:, from:, to:, step:, engagement:, step_length:, leave:, sweep:, passes:)` | **opt-in, never the default**: roughing around the part (clear's region) with the radial engagement held under one cap (`engagement`, degrees, default 60) by simulating the cleared area step by step. Passes are one-direction peels within 75° of `sweep` (degrees from +x), or with `passes: "two_way"` a zigzag across the sweep. Links stay at depth where the cleared area allows: straight, or back along what the tool already cut at that depth then straight (within a fixed length cap), else a retract. Then plain offset rings only where material is left, and a wall pass. Closed pockets are not offered (use `pocket`). **A tool-load option, not a time-saver: it reduces and steadies tool load (engagement capped); check cycle time on your machine.** Measured on our benchmark (verify/forge-cam-quality, deep_boss: an open level 2.5 D deep, Ø6, generic-vmc3): it cuts faster than clear (steel t_min 399 s two-way vs 462 s) but its path has about 5× the moves, 3× the sharp corners and about 90 retracts against 3, so on a jerk-limited machine model it was slower than clear: +5 to +14 % in steel, +21 to +42 % in aluminium (four acceleration/jerk sets fitted from a Spinner U5-620's logs). On shallow levels and thin bands round an island (RPTU parts) clear is faster too. The voids it claims are only cells provably inside the tool's flat end's sweep |
| `contour(SETUP, tool:, side:, from:, to:, step:, leave:, leave_axial:, direction:, finish:, finish_step:, spring:, lead:, lead_radius:, lead_sweep:, lead_length:, entry:, overlap:, tabs:, tab_spacing:, tab_at:, tab_width:, tab_height:, tab_shape:)` | the part's outline (`side: "outside"`) or its holes (`"inside"`) at the section |
| `pocket(SETUP, PART.HOLE, tool:, stepover:, step:, leave:, leave_axial:, ramp:)` | a hole feature's circle, to its depth |
| `pocket(SETUP, tool:, from:, to:, stepover:, step:, leave:, leave_axial:, ramp:)` | every hole and slot of the part's section between the heights, each less the part's section there (islands standing in it, a boss in a well, are kept: examples/machined/well_island_cam.fg) |
| `slot(SETUP, PART.SKETCH, tool:, depth:, step:, from:, ramp:)` | an end mill (flat or bull) along the sketch's lines, arcs and circles (not construction), on a plane facing +z, in levels down to `depth`: the groove the tool's width, modelled in the part (G1 against the part). `ramp: DEG` enters along each curve's first line, back and forth at that angle from the level above; else a plunge. The stock records each curve's band of the tool's radius as cleared (above the corner radius for a bull). examples/machined/slot_plate_cam.fg |
| `engrave(SETUP, PART.SKETCH, tool:, depth:, step:, from:, offset:, axial_offset:)` | the tool's tip along the sketch's lines, arcs and circles (not construction), on a plane facing +z, in levels down to `depth` below it. The groove must be in the model (G1 is checked against the part, never a blanket tolerance); the stock records each line's and circle's band of the tip's radius as cleared (a V tool clears more, so G4 may list the rest of the groove). Trace: `offset:` moves the tool off each curve to its left (> 0) or right (< 0) as it runs (a line along its normal, an arc or circle to the concentric radius; corners between curves are not joined), `axial_offset:` raises (> 0) or lowers the tip; `slot` takes both too. The moved path is certified against the part like any other |
| `thread_mill(SETUP, PART.HOLE, tool:, length:, passes:, spring:, hand:, starts:, lead:)` | a hole's recorded thread (`thread:`, or `screw:` with `fit: "tap"`), the model not carrying it: declares a cosmetic thread Δ per place (the ring from the bore to the major diameter over `length`, default the depth less two pitches), listed in the op's `removals`, and is certified against P ∖ Δ (never a blanket tolerance). Down the bore, in to the radius, up a helix of the pitch (G3: right hand), out. `passes: n` radial passes splitting the depth from the bore to the major evenly (default 1), `spring: n` the last again, `hand: "left"` climbs down from the top (G3), `starts: n` a helix of n pitches per turn once per start, each a pitch higher, `lead: "arc"` in and out on a half circle tangent to the helix (default `"line"`) |
| `bore(SETUP, PART.HOLE, tool:, pitch:, ramp:, finish_leave:, spring:, leave:, leave_axial:)` | helical bore of a round hole along −z: the tool's centre on the hole's radius less its own, down a helix of `pitch:` per turn (default a 3° ramp; or `ramp:` degrees, at most 30), G3 (climb), a full circle at the bottom, back to the axis, up. `finish_leave:` leaves that much for a second helix at the final radius; `spring: n` repeats it. The tool must be at least half the hole across (no core left), else refused with the fix (`pocket(..)` the hole). Certified against the part |
| `edge_break(SETUP, tool:, size:, tip_offset:, at:)` | chamfer/deburr of the part's top edges with a chamfer mill or spot drill of 30° to 150°: a chamfer `size` wide at half the tool's angle from the wall (45° for a 90° tool), its tip `tip_offset:` below the chamfer's bottom (default 0.2) so the flank, not the tip, cuts. The removal is declared (Δ, in `removals`) and the op certified against P ∖ Δ. In this phase: the part's top, over vertical walls; sharp corners between lines only |
| `drill(SETUP, PART.HOLE, plan: "auto")` | the plan from the hole feature: spot, drill (peck past 3 D), then its counterbore (a counterbore tool of its diameter, else the largest end mill under it pocketing the circle) or countersink (a countersink, chamfer or spot tool of its angle, plunged until the cone is its diameter at the mouth), tap for a thread; `plan: "drill"` leaves the thread (to `thread_mill`). A hole with an ISO class in the part (`hole(.., d: 6, fit: "H7")`) is drilled under size (the library's drill 0.1 to 0.4 under, nearest 0.2 under) and reamed (a reamer of its size, G85); a plain hole of a dowel's size with no class is said. A blind hole with a flat floor is drilled with the drill's point on the floor; the ring that leaves is said |
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

### Feeds and speeds are params

Every milling op (facing, clear, contour, pocket, slot, engrave, thread_mill, edge_break) and
every `tool(..)` takes these args; an op's value wins over its tool's (kernel/forge-tooling/src/resolve.rs):

| arg | unit | what |
|---|---|---|
| `vc:` or `rpm:` | m/min, rpm | cutting speed (at D_eff for a ball), or the spindle speed (wins) |
| `chip:`, `fz:` or `feed:` | mm, mm, mm/min | the peak chip to hold (thinned for ae < D/2); a feed per tooth used as given (not thinned); or the feed |
| `ap:` (= `step:`) | mm | axial depth per level (one of the two) |
| `ae:` (= `stepover:`) or `max_angle:` | mm, degrees | radial width (clear and pocket cut their stepover wide), or the largest engagement angle |
| `tilt:` | degrees | the tool's tilt toward the feed: a ball's contact moves out to D sin(α + tilt) |
| `plunge_feed:`, `ramp_feed:`, `lead_in_feed:` | mm/min | entry feeds (default 30%, 30%, 100% of the feed: labelled "no data behind it"). Posted today: the plunge feed (plunges and helical ramps) and the feed; `ramp_feed` and `lead_in_feed` are resolved and reported but not yet posted on their own (forge-cam's `Cut` carries two feeds) |
| `max_power:`, `max_torque:` | kW, N·m | limits: the feed (only the feed) is lowered to meet them; P ∝ fz^(1−mc), so exactly |
| `max_deflection:` | mm | the tool's tip deflection limit: the feed (only) is lowered to meet it. δ = F/(3E)[(L³ − b³)/I_shank + b³/I_flute] (stepped cantilever, derived), F = 2 Mc/D the mean tangential force at the tip, L the stick-out, b the flute length, E and the flutes' equivalent diameter from the substrate's cited `[[substrate]]` record; the holder and spindle taken rigid (their compliance not checked). Reported as `derived.deflection_mm` |
| `min_feed:`, `max_feed:` | mm/min | the feed range (engagement feed stays in it) |

Thread milling: the width is the thread's own engagement on its arc in the bore (concave arc,
feeds-speeds.md §2.3): cos φ = (r_b² − r_c² − r_t²)/(2 r_c r_t), r_b the bore, r_c = r_m − r_t the
centre path, held as the straight cut of the same angle (source `default`, said in the note); the
chip is held at the edge, and the centre is fed r_c/r_m of the edge's feed (plunge, ramp and
lead-in alike). A user `ae:`/`max_angle:` or programmed `fz:`/`feed:` is used as given.

Op classes (cutting.toml `[[op_class]]`, cited: medians of the makers' own rough and finish
columns against their slot column for the same tool, verify/feeds-vs-makers): a tool preset named
for the op's class (`N rough`, `P slot`, ...) is taken first, then a `general` one. With generic
data, slot and general hold the slot chip; finish holds 0.75× it; rough holds 1.466× it only where
engagement feed measures a sample engaged under 90° (ae < D/2): the op's static feed, which also
runs entries and unmeasured moves, keeps the slot chip.

Engagement feed (clear, pocket, contour, slot, facing; not when the user programmed `fz:`/`feed:`):
TOOLPATH-QUALITY's measurement on the in-process stock gives each FIXED-LENGTH sample (one tool
diameter) its engaged angle, ap and ae; the feed holds the resolved chip through radial thinning,
then is lowered (only the feed) so the sample's power and torque stay within `max_power:`,
`max_torque:` and the spindle's power at the rpm, inside [`min_feed:`, `max_feed:`]. Samples are
joined only at the same clamp, never by engagement band; no material-abrasion term.

The stock material is `part(.., material:)` (a forge-material name; stock is often not the
part's own record), else the part document's material. Its ISO 513 group is `part(.., group:)`,
else its `[[stock]]` record's in cutting.toml, else its family's class (steel P, stainless M,
iron K, aluminium and copper N, titanium S); nothing defaults. A `[[stock]]` record with measured
Kienzle rows (Ti-6Al-4V and C45: Klippel et al. 2022, CC BY 4.0) gives kc1.1/mc at the cut's speed
for power and torque. The tool's substrate (`material`: carbide, HSS, ...) picks the group's
speed column; cermet, ceramic, CBN and PCD have none, so the maker's preset or the user's speed
is needed; a coating moves no generic number (the maker's preset carries it). With no speed or
chip from any layer the op is refused, with the fix.

Plastics (forge-material family `plastic`) are group O: not an ISO 513 class, the makers' own
grouping. Its speed and chip by diameter are Harvey's plastics finisher sheet (acrylic, acetal,
nylon, PEEK ... unfilled), cited; no Kienzle data is published, so power and torque read `not
checked`, and no HSS speed (an HSS tool needs the user's speed). Plastics melt: supervise the speed.

**A tool with maker presets** uses them first: `Tool::preset_for_material` takes the maker's row
that names the part's stock material (its alloy in the maker's text, else its family pattern such
as 6xxx or C3xxxx; the slowest hardness class among equals), else `Tool::preset_for` the class's
representative ("<group> <class>"). A preset's fz is the maker's programmed feed at its own
stated width and depth, so its chip is fz·k(ae/D)·sin κ. The op's class (`Tool::class_for`): a
slot is slot; clearing, pocketing and facing rough; a contour at a finishing width (≤ 10% D, or
the tool's finish preset's ae where wider) finish, else rough; the rest general.

**A tool with no maker data** takes the **10th percentile (P10) of its class** from
kernel/forge-tooling/data/generic.toml (generated from the makers' cited rows by
verify/feeds-vs-makers, `cargo run -p feeds-vs-makers -- --generic`): a class is material group ×
material family (the part's forge-material family: N splits aluminium / copper, S titanium /
nickel) × op class × maker tier (material-specific / general-purpose, an estimate from the
makers' product lines) × flutes × coating × helix band, written only with ≥ 10 rows from ≥ 2
makers; where the tool's class has fewer, the lookup opens helix, then tier, coating, flutes, op
class, and last the family (the note names the class used). A part that names only its group
gets the whole group (in N, copper's speeds among them). P10, not the median, because the costs are asymmetric: too fast breaks tools, too
slow only costs time. Speed is the class's P10 speed; the chip is the class's P10 slot chip relative to
the group's slot chip at the diameter; a rough or finish op's factor is the class's own P10
op/slot chip ratio (else cutting.toml `[[op_class]]`). The value's `spread` (JSON below) gives the
class's min / P10 / P25 / median / max so the user can push up knowingly. Carbide only (every maker row
is); HSS and groups with no class use cutting.toml's group columns. The hold-out per maker (verify/feeds-vs-makers/REPORT.md) gives how often P10 is above a maker's
own number.

Each value is resolved in layers, each over the one before: `default` → `material` (the
stock's data, else its group in cutting.toml) → `library` (the tool's `[[tool.preset]]` for the group, with its cite) →
`user`. Then only two things lower a value, and both are reported with the reason:
`machine-clamped` (rpm range; max feed, held 1 mm/min under it) and `limited` (the user's own
limits). A user's value is never changed silently. Formulas (feeds.rs, with sources): chip thinning
hex = fz sin φ, φ = acos(1 − 2ae/D); the mean chip hm = fz (1 − cos φ)/φ for Kienzle's
kc = kc1.1 hm^(−mc); Pc = ap ae vf kc / 6e7 kW; Mc = Pc 30e3/(π n) N·m. Contact (feeds::contact): for ap < re (bull re, ball D/2), D_eff = D − 2re + 2√(re² − (re − ap)²) and the axial thinning sin κ = √(1 − (1 − ap/re)²) (a ball: D_eff/D, its width cut at D_eff); a tilted ball D_eff = D sin(min(α + tilt, 90°)), α = acos(1 − 2ap/D). Thinning together is capped at 3×: past it the chip is held under the target.

Checks per op: rpm (a clamp warns), power and torque against the machine's spindle curve at the
rpm (over: the op is refused, with the numbers), feed against the machine's maximum, the peak chip
against the tool's own chip (over: warns). Missing data is `not checked`, never a pass: no power
curve, an rpm outside the curve (no extrapolation), no kc data for the material, no maximum feed,
no tool data.

Drilling (`drill(..)`: spot and drill) is its own model, forge_tooling::resolve::drilling: `vc:`|`rpm:`,
`f:` (mm/rev)|`feed:` (mm/min), `peck:` (mm, 0 none), `max_power:`, `max_torque:` on the op. Layers:
the group's `[[drill]]` row for the drill's substrate (cutting.toml: vc and the feed per rev by
diameter, each row one maker's cited series held for the substrate: Guhring 5510 carbide and 651
HSS, OSG EX-SUS-GOLD HSS for N and M) → the drill's own preset (its maker's vc and mm/rev) → the user.
Sandvik's drilling formulas: n = 1000 vc/(π Dc), vf = fn n, Q = Dc fn vc/4, Pc = fn vc Dc kc/240e3,
Mc = Pc 30e3/(π n), kc at (fn/z) sin κr. Peck: the user's, else PMPA's schedule (Cribsheet #116: peck past 3–4 D; 3.5 D, 2 D, then 1 D),
held at its 1 D for every peck from 3 D (one G83 Q; errs slow). A spot drill borrows the twist-drill row at the diameter it cuts
to (said; no spot data read). Reply: ops[].drilling[] {role, tool, values, derived, checks}. Taps (`drill(.., plan: "auto")` on a threaded hole) run at `tap_vc:`|`tap_rpm:`, else the
tap's preset, else the group's `[[tap]]` row (OSG A Brand cut-tap tables, List 16505/16515: the low
end of the slower list; none for S or H), clamped to the machine; the feed is the pitch per
revolution. Reply rows: role spot | drill | tap.

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

## Probing

A touch probe is a tool that cuts nothing, and probing ops measure with it.

```text
p   = probe("renishaw-omp40-2", t: 20, ball: 6, length: 50, stem: 3, body: [(20, 0, 69)])
z0  = probe_wcs(op1, tool: p, kind: "surface", at: (25, 25, 20), wcs: 54)
w   = probe_wcs(op1, tool: p, kind: "web", at: (0, 0, 10), size: 86, axis: "x")
c   = probe_wcs(op1, tool: p, kind: "corner", at: (43, 43, 10), side: (1, 1))
b   = probe_wcs(op1, tool: p, kind: "bore", at: (0, 0, 8), size: 20)
chk = inspect(op1, tool: p, at: (43, 0, 10), normal: (1, 0, 0), nominal: 43, tol: 0.05)
```

- `probe(DEVICE, t:, ball:, length:, stem:, body:)`: the stylus ball's diameter, the stylus
  length (mounting face to ball centre), the stem's diameter, and the probe body's rings
  `(r, from, to)` above the mounting face (its envelope, needed to check it). The device's
  record (kernel/forge-probe) fills these once that library is on this base.
- `probe_wcs(SETUP, tool:, kind:, at:, ...)`: `surface` (`at` is where the ball's tip meets the
  surface), `bore`, `boss` (diameter `size:`), `web`, `pocket` (width `size:` across `axis:`),
  `corner` (`side:` the corner's free side). For the features, `at` is the centre (or corner) at
  the height the ball's centre probes at. `wcs:` names the work offset the measurement is for.
- `inspect(SETUP, tool:, at:, normal:, nominal:, tol:)`: one point along its outward normal.
- Each takes `approach:` (start this far off the surface, default 5), `over:` (how far past the
  nominal contact the search may go, default 1), `entry:` (the contact window starts this far
  before the contact, default twice the setup's clearance, at least 0.5), `feed:` (the search,
  default 100 mm/min) and `link_feed:` (the moves between, default 1000 mm/min).

The approach must exceed `entry` by more than the clearance, so every probe move has a
checked approach before its window. The window's soundness assumes the control stops within
`over` of the first trigger: that is the probe's overtravel specification, so `over` must not
exceed it (the device records give it as `overtravel`). Inside the window the ball is exempt
against every face, not only the probed one (verify/forge-cam RESUME, H.probe): a second face
there would give a wrong measurement, not a crash.

Each probe move is certified as its whole search: from its start to `over` past the nominal
contact, as if the probe never triggered. Every body of the probe keeps the setup's clearance
from the part and the stock on every move. The one exception is the ball inside the contact
window, from `entry` before the contact to `over` past it. A probe is never spun. It posts as
the dialect's cited probe move: G38.2 on linuxcnc and grbl, then a move straight back to where
the search started. A control with no openly published probe move refuses the program. The
reply's op carries `probe`: the kind, what it measures, its points (each surface point and the
direction of travel), the work offset, `sets_wcs` (false: a G38.2 post leaves the result in
the control's variables and writes no offset), and the inspection's nominal and tolerance.

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
           feeds_speeds {spread {class, rows, makers[], vc {min, p10, p25, median, max}, chip {min, p10, p25, median, max}} | null,
                    values {vc, rpm, chip, fz, feed, ap, ae, plunge_feed, ramp_feed, lead_in_feed,
                    min_feed, max_feed: {v, unit, source (default|material|library|user|machine-clamped|limited), note}},
                    derived {hex, hm, engagement_deg, d_eff, mrr_cm3_min, power_kw, power_pct, torque_nm, torque_pct},
                    checks[] {what (rpm|power|torque|feed|chip), verdict (pass|warn|fail|not checked), value, limit, unit, why}} | null,
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
3+2, turning, 5-axis (adaptive clearing is the opt-in `adaptive(..)` above); chamfer mills,
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
