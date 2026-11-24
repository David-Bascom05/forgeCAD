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
| `setup(PART, machine:, stock:, origin:, wcs:, at:, tol:, clearance:, clear:, retract:, fixture:, from:)` | `stock: box(margin: (x, y, z), below: b)` (b mm of stock under the part, default 0) or `box(min:, max:)`; `origin: "top-center"`, `"top-corner"` or `(x, y, z)` in the part's frame; heights above the stock top; `from: SETUP`: the part turned over about x, starting from the stock that setup provably left (turned) |
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
joined only at the same clamp, never by engagement band; no material-abrasion term
(research/cam/toolpath-quality/fto-adaptive.md, design-around D).

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
           feeds {rpm, vc, fz, hex, hm, f, power_kw, torque_nm} | null,
           feeds_speeds {values {vc, rpm, chip, fz, feed, ap, ae, plunge_feed, ramp_feed, lead_in_feed,
                    min_feed, max_feed: {v, unit, source (default|material|library|user|machine-clamped|limited), note}},
                    derived {hex, hm, engagement_deg, d_eff, mrr_cm3_min, power_kw, power_pct, torque_nm, torque_pct},
                    checks[] {what (rpm|power|torque|feed|chip), verdict (pass|warn|fail|not checked), value, limit, unit, why}} | null,
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
