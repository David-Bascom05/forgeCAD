# Machine definitions (forge-machine)

A machine tool as data: a kinematic tree of joints from the static base, the spindles, turrets
and tool changer, the mounts where tools and parts attach, and collision bodies. Read by
`kernel/forge-machine` (`MachineDef::parse_all`); built-in machines are in
`kernel/forge-machine/data/machines/<vendor>.toml`, one file per vendor (`generic.toml` holds our
placeholder machines; build.rs embeds every file in the directory, generic first, then by name;
a machine name may appear once across them), each body backed by a real part under
`kernel/forge-machine/parts/<vendor>/<machine>/`. Search order, as for every CAM library: the project's
`cam/machines/*.toml`, then the user's `~/.config/forgecad/cam/machines/`, then the built-in set; a
name found earlier hides a later one.

The file is our TOML subset (forge_tooling::toml). Nothing is defaulted silently: a missing field a
kind needs is a typed error naming the record, the field and the fix. Fields marked *optional* say
what their absence means.

## Frames and units

- mm, degrees, mm/min (deg/min for rotaries), seconds, rpm, kW.
- A program may be posted in inches (forge-post `PostOptions::inch`; a setup's `units = "inch"`,
  `Control::in_inches`). Its words are read at the dialect's inch decimals, and a linear word
  w becomes w·254·10^-(k+1) mm at the boundary, exactly as an integer before the one division.
  Everything else stays in mm: the machine, offsets, tools, stock. G20 is refused in a mm
  program and G21 in an inch one: a program does not switch units. A feed move that turns a
  rotary axis is refused in inches, because F would mix inches and degrees.
- All geometry is in the **base frame with every joint at 0** (the home pose).
- A **linear** joint at value q translates its link and everything below it by q·`axis`.
- A **rotary** joint at value q turns its link by q degrees about the line through `pivot`
  along `axis`, right-handed.
- A joint's value is what the control's machine coordinate for its letter reads, except a lathe X
  with `diameter = true`: the program's X word is the diameter, the joint moves half of it.
- Table-side joints move the part: give `axis` as the direction the *table* moves for +word. A VMC
  whose table moves −x for +X has `axis = [-1, 0, 0]`; then the tool moves +x relative to the part,
  as the program means.

## `[[machine]]`

| field | | meaning |
|---|---|---|
| name | required | unique in its level |
| kind | required | `vmc3` `vmc4` `table-table` `head-table` `head-head` `lathe` `mill-turn` `hmc` `gantry` (3 linear joints all on the head, a dual-drive gantry axis as one joint, up to two head rotaries) `turn-mill` (a live-turret lathe: X Z, optional Y and C, a turret whose `live` names the milling spindle that drives it; no B head) |
| control | required | the forge-post dialect its control speaks (machine and control kept apart) |
| source | required | where the numbers come from; `taken`, `note` optional |
| clearance | required | least gap (mm) the collision check holds between bodies |
| length_joint | mills: required | the linear joint G43 moves: a VMC's head Z, or an HMC's table Z (the table moving to the column; its axis is the table's motion for +word, so H acts the same); lathes use X/Z offsets instead |
| coolants | optional | kinds present: flood mist through air (empty: none declared) |
| peck_clear, chip_lift | optional | G83's peck clearance and G73's chip-break lift, mm: the control's parameters (Fanuc 5115/5114, Haas settings, Siemens _ZSFR, ...), citable as `machine.peck_clear` / `machine.chip_lift`. Absent: forge-post's 0.5 / 0.2 mm are used and every program with G83/G73 reports them not checked (`sim::unchecked_cycles`) |
| real | optional | true: a real maker's model; every value must be cited (Provenance). false if absent |

## `[[machine.joint]]`

| field | | meaning |
|---|---|---|
| name | required | unique; also the letter unless `letter` is given |
| letter | optional | X Y Z U V W A B C, optionally one channel digit (Z2, C2). A–C rotary only |
| parent | required | `"base"` or a joint: the tree (mill-turn has several branches) |
| side | required | `head` (on the path to a tool mount) or `table` (to a part mount); checked |
| type | required | `linear` (ISO 10303-105 prismatic) or `rotary` (revolute) |
| axis | required | direction, normalised on read |
| pivot | rotary: required | a point on the axis, home pose; refused on a linear joint |
| range | required | [lo, hi] travel; a cyclic rotary's output range |
| home | optional | in range; 0 if absent |
| rapid, max_feed | required | per axis, max_feed ≤ rapid |
| accel | optional | mm/s² (deg/s²); absent: time estimates assume infinite acceleration and say so |
| cyclic, tcp | optional | rotary only; false if absent |
| preference | optional | −1, 0, 1 (createAxis semantics); 0 |
| reset | optional | never op-start rewind both; never |
| diameter | optional | lathe X only: the word is a diameter |
| ref | optional | `plus` or `minus`: the end of the travel where the machine's zero (reference point) sits, as makers publish it |

## `[[machine.spindle]]`

`name`, `link` (the link it rides on), `kind` (`milling` turns a cutter, `turning` turns the part),
`axis`, `nose` (a point on the spin axis: gauge line or spindle face), `rpm = [min, max]`, `max =
[length, diameter]` (milling) or `[swing, part length]` (turning). Optional: `curve_rpm`/`curve_kw`
(piecewise-linear power, rpm increasing; absent: no power check), `interface`, `c_joint` (the rotary
joint that positions this spindle as a C axis), `ratings_kw` / `ratings_duty` (duty ratings as
published: `cont`, `30min`, `10min`, `1min`, `S3-25%`, ...) and `torque_nm = [max, continuous]`.
With no curve, a continuous rating and continuous torque give the feeds engine the conservative
curve: torque-limited to the base speed P / T, then the continuous power.

## `[[machine.turret]]`

`name`, `link`, `stations`, `index_s` (seconds per station), `center`, `index_axis`; optional `live`
(the milling spindle that drives live tools).

## `[[machine.pallets]]` (at most one)

An automatic pallet changer (an HMC's APC): `count` (≥ 2), `change_s` (seconds per change; may be
omitted when cited unknown). Optional: `max_kg` (per pallet), `size = [x, y]`, `max_work =
[diameter, height]`, and `position_joints` / `position` (where M60 sends the machine first, as the
changer's). On a machine with pallets, M60 swaps the pallet: the timeline marks it and adds its
time, after the position move (a checked rapid). M60 elsewhere is refused. Kind `hmc` reads as
`vmc4` (3 linear and a table rotary) and says what the machine is.

## `[[machine.gang]]`

A gang tool post (Swiss and gang lathes): `name`, `link` (the slide), `t = [..]`, and three numbers
per T in `at` (the station's gauge point, home pose) and `dir` (gauge to tip). Each station is a
tool mount named `NAME.T<n>`; T n puts the tool there (no index time, no stop: the change is the
next move). A lathe takes a gang instead of a turret.

## `[[machine.toolpost]]`

A manual tool post (quick-change holders; a Sherline's or a Tormach 8L's post): `name`, `link`,
`center` (the holder's datum, home pose); optional `change_s` (the operator's swap, seconds; absent:
0 in `t_min`, reported not checked). A lathe takes it instead of a turret. Any T number; a T that
changes the tool is a program stop for the swap.

## `[[machine.changer]]` (at most one)

`pockets`, `change_s` (chip to chip), `max_tool_kg`. Optional: `kind = "manual"` (M6 stops the
program and the operator swaps the tool: no pockets and no `max_tool_kg`, no pocket check; the
timeline marks the stop), else automatic. Optional: `position_joints` and `position`,
where M6 moves the machine by itself before the swap (joint values, e.g. `position_joints = ["Z"]`,
`position = [0]` for a Haas, whose M06 sends Z to machine zero). The simulation makes that move as
a rapid with the old tool, timed and collision-checked; without it the move is not simulated and
every tool change reports `changer.position` not checked.

## `[[machine.mount]]`

`name`, `kind` (`tool` | `part`), `link`, `at`, `dir`. A part mount may give `max_kg` (the most the
table, pallet or rotary carries) and `max_inertia` (kg·m², rotaries); a setup's `work_kg` is checked
against `max_kg` (over it: refused; either one absent: reported not checked). A tool mount's `at` is the gauge point and
`dir` points from it to the tool tip. A part mount's `at` is the fixture datum (table centre, chuck
face centre), `dir` the part's +Z and `xdir` its +X (orthogonal). Every machine has at least one of
each; a mill-turn has two part mounts (main, sub).

## `[[machine.body]]`

`name`, `link`, one or more `[[machine.body.box]]` (`min`, `max`, home pose) or
`[[machine.body.cyl]]` (`base`, `axis`, `r`, `h`). Optional: `part` (the real `.fg` part the
primitives envelope), `ignore` (bodies it touches by construction: ways, neighbours), `clearance`
(its own least gap, mm, in place of the machine's for any pair it is in; the smaller of two bodies'
wins), `passes = "stock"` (the bar runs through it by construction, as through a Swiss guide
bushing: stock against it is never a contact). Bodies on one
link are never checked against each other.

## Probing: `[[machine.probe]]`, `[[machine.setter]]`, `[[machine.homing]]`

The devices themselves are kernel/forge-probe records (work probes, contact and laser tool
setters, each value cited from the maker). A machine record links them:

| table | fields | meaning |
|---|---|---|
| `[[machine.probe]]` | `name`, `device`, `offer`, optional `t` | a spindle work probe the maker ships (`offer = "standard"`) or offers (`"option"`); `t` the pocket/T it is kept in when stated. In the spindle it is a tool: the setup gives its SimTool (stylus and body). Cite `probe.NAME` |
| `[[machine.setter]]` | `name`, `device`, `body`, `at`, `dir` | a tool setter whose envelope is the ordinary `[[machine.body]]` named `body` (so the collision checks and the self-collision sweep cover it unchanged); the body rides the table side (a table joint, or the base). `at`: the trigger point a tool is brought to; `dir`: the device's up; base frame, home pose. Cite `setter.NAME.at`, `setter.NAME.dir` |
| `[[machine.homing]]` | `order` | at most one: groups of joints homed together, the groups in turn (`[["Z"], ["X", "Y"]]`); each joint goes toward its `ref` end. Joints it does not name home afterwards one at a time (a maker may publish only "Z first"): their order is reported not checked (`homing.rest`). Only where the maker publishes the sequence; cite `homing`. Absent: a homing is a generic reference return (every joint together), reported not checked (`homing`). `homing::home` gives the motion as rapids of the timeline (to each joint's `ref` end of travel; a joint without `ref` goes to its `home` value, `joint.N.ref` not checked); the search, switch and back-off are not modelled. The machine homes from wherever it is: the sweep from the given pose is reported not checked (`homing.start`) unless the caller says it knows that pose |

## Provenance: `[[machine.cite]]`

Where each value comes from. A record with `real = true` cites every value `MachineDef::cite_paths`
lists: per joint `joint.N.range`, `.rapid`, `.max_feed` (and `.accel`, `.pivot` when present); per
spindle `spindle.N.rpm`, `.nose`, `.max` (and `.curve` with a power curve); per turret
`turret.N.stations`, `.index_s`, `.center`; `changer.pockets`, `.change_s`, `.max_tool_kg`; every
`mount.N.at`; every `body.N` (its envelope). Other values may be cited too (`joint.N.axis`,
`.home`, `spindle.N.interface`, `machine.<anything>` for record-wide facts); a cite naming nothing
in the record is an error. A path may be cited more than once (sources that agree, or conflict).

| field | | meaning |
|---|---|---|
| path | required | the value, as above |
| status | required | `published` (the source states it), `estimate` (not published: derived as `basis` says), `unknown` (nothing to derive it from), `conflict` (sources disagree), `user` (the user's own measured or entered value; source and taken optional) |
| source | published, conflict: required | URL or document; restate, never copy |
| taken | with a source: required | YYYY-MM-DD the source was read |
| value | optional | the source's statement, restated with its units ("30 in (762 mm)") |
| basis | estimate, unknown: required | how it was derived, and which way it errs |
| note | conflict: required | what disagrees and which value the record holds |

**Unknown numbers may be omitted.** A required number (a joint's `range`, `rapid`, `max_feed`; a
spindle's `rpm`, `max`; a turret's `stations`, `index_s`; the changer's `pockets`, `change_s`,
`max_tool_kg`; a body's envelope) may be left out if and only if a cite names it with
`status = "unknown"`; leaving one out otherwise is an error that says so. Structural fields (a
joint's type, axis, parent, pivot; a mount's link and `at`; a spindle's nose) stay required.

**Never a silent pass on a stand-in.** Every check that needs an omitted value, or one cited
`unknown` or `estimate`, is reported as *not checked* (`sim::unchecked`; the setup reply's
`not_checked`, each value once with the first line that needed it), not as OK: overtravel against
a range, the feed clamp against a max feed, rapid time against a rapid rate, the rpm range, pocket
and station counts, tool change and index times, and collision of a body with no envelope (left
out of the check). Time bounds stay sound: an omitted rate is unbounded and an omitted change or
index time is 0, so `t_min` is still a lower bound. Values given with an `estimate` cite are
still used (the record's number, chosen conservatively as `basis` says) and the checks against
them are listed as estimated.

## `[[machine.zone]]`

A region of joint space the control forbids: a stored stroke check (Fanuc stroke limits 2 and 3),
an interference zone. Machine data like any other, stated in the record and citable as
`zone.NAME` (often `user`: the values set in the control).

| field | | meaning |
|---|---|---|
| name | required | unique |
| joints | required | the joints it bounds; every other joint is free |
| min, max | required | per joint, min < max: the zone is the open box min < q < max (its boundary is travel) |
| note | optional | |

The simulation reports a program entering one (violation `zone`, at the first point found: exact
on straight moves, by bisection on arcs); the self-collision check below leaves zones out of the
travel. The generic mill-turn declares the zones that keep its B head off the headstock, chuck and
sub-spindle.

When the maker does not publish its crash zones, the record cites `zones` (the path, as a whole)
`unknown`. A real lathe's turret does reach its chuck, and an HMC's spindle its pallet, within the
published travel. In such a record, a contact between the machine's own bodies in a run is
reported with the contact as "possible contact: crash zones not published" (not checked: never
clear). The built-in vendor records whose envelopes meet within travel cite it. Their contacts are
pinned in tests/collide.rs: a new one fails, and so does one that goes away.

## Self-collision through travel

A machine's own bodies must not collide anywhere inside the declared travel outside its zones: a
pose the joints can reach where they do is wrong data (fix the geometry, the travel, or declare
the zone the control enforces). tests/collide.rs sweeps every built-in: for each pair of bodies, each
joint that moves one relative to the other through its whole range, with the others at every
combination of their ends; no Hit and no Near.

## Variants and options

One record per variant (spindle, magazine, Y axis, high column): a maker's option sheet becomes as
many records as builds you use, each named for its build (`haas-vf2-15k`).

## Structural checks (`MachineDef::validate`)

A `table-table` machine may carry one turning spindle on its table's C (`c_joint` naming a table
rotary): a mill that also turns, the milling spindle holding the stationary tool; its X is a
radius.


Unique names and letters; the tree is acyclic and rooted at the base; every link named exists;
every joint lies on some mount's chain, head joints only on tool chains, table joints only on part
chains; the kind fits its joint, rotary, spindle and turret counts (table-table and head-head need
the second rotary carried by the first); lathes have a diameter X, mills a length joint.

## For lathe and mill-turn posts (CAM-TURN)

- The X word is a diameter: joint value = X/2 on the joint with `diameter = true`.
- The turning spindle's `rpm[1]` is the G50 clamp ceiling; G96 (CSS) stays under it.
- Turret stations bound T numbers on a lathe; `index_s` feeds the timeline.
- `control` names the dialect. The built-in lathe and mill-turn say `fanuc-lathe`; the dialect is
  CAM-TURN's.
- Mill-turn: part mounts `main` (link C) and `sub` (link C2 on Z2); the milling head's tool mount is
  `bhead` on B.

## Your own machines (the machine builder)

`interface/forge-machine-builder` writes and checks machines so nobody has to write this TOML by
hand (research/cam/machine-builder.md). The format is the one above; the builder's files are
canonical (reading one and writing it again gives the same bytes).

```
forgecad machine list [--json]              every machine the search order sees: level, file,
                                            and the machines of the same name it hides
forgecad machine new NAME --template KIND   KIND: vmc3 trunnion head-table lathe mill-turn gantry
forgecad machine new NAME --from MACHINE    a clone of any machine listed (built-in, user, project)
     [--project | --user]                   where it is saved (default: the user's folder)
     [--own | --keep-cites]                 copied values: cited unknown (default), the source's
                                            cites only, or no cites (the user's own)
forgecad machine check FILE.toml|NAME [--json] [--no-parts]
```

A new machine's copied values are cited `status = "unknown"` with a basis naming where they came
from: they are not this machine's. A value the user enters carries no cite; in a user or project
file an uncited value is reported as the user's own (status `user`). A clone of a built-in machine
brings its body parts (`parts/NAME/BODY.fg` beside the file).

`check` reports: the reader's validation; every pair of bodies at home; every pair through travel
(`proven` clear for all joint values, `hit` with the joint values and a point, `near`, or
`sampled`: lines swept clear but the poses between them not proven); each body's part inside its
envelope; the provenance of every value, listing those cited unknown as not checked. Exit 0 ok,
1 problems, 2 usage.

## Simulation (posted G-code)

`forge-machine` reads the posted text itself, not the toolpath it came from, and drives the joints
with it. That is the check Fusion's simulation skips: Fusion simulates the toolpath before the
post.

```
forgecad machine list                       the machines (project, user, built in)
forgecad machine SETUP.toml [--json]        run a setup: exit 0 ok, 1 refused, 2 error
```

The reply (`forgecad.machine/1`): the timeline (`forgecad.machine-sim/1`: every segment's line,
kind, start, `t_min` and `t_stop`, end joints, tool; marks for tool changes, turret indexes,
dwells, spindle and coolant; violations: overtravel including an arc's extremes, feed clamps,
unknown tools, rpm out of range, pockets), the collisions, the rapids into material, the tool
pieces into material, the removal summary, and what is claimed.

| part | what it is |
|---|---|
| interpreter (forge-machine-core, gcode.rs) | words to control events in exact integers. The block core (kernel/forge-machine-core: motion, modal words, arcs, canned cycles, G28) is proven, through Aeneas, to be the Lean spec's `step` and, over a program, its `runFrom` (verify/aeneas/forge-machine-core, L4, within 2^29 units); the spec refines forge-cam's post model, so posted programs do the planned motion (verify/forge-machine). The shell (text to words, tools, spindle, offsets) agrees with forge-post's reader on its subset (L6: random programs on all 11 dialects, also under the post options: fewer decimals, line numbers on and off, no safe start, inches; with probe moves on the dialects that cite `probe_move` and the probing macros on those that cite them) |
| skip moves (gcode.rs shell, sim.rs) | G38.2–G38.5 / G31, only where the dialect's `probe_move` cites the code (otherwise refused as outside the subset). Outside the proven core: a skip move stops somewhere unknown along its segment, so the collision and removal checks sweep the whole segment as if it ran to its end, its `t_min` is 0 (it may trigger at once), the moved axes' program positions become unknown (an incremental word from them is refused) and the motion mode is cleared. The timeline marks it (`skip`); the moves after it start from the segment's end, or from the stock model's trigger when `removal::mill_probing` found one (a simulation-level estimate for time and display, not the machine's stop), and are always reported not checked (`skip.end`: unknown, or estimated with a model trigger). Collision checks them for every stop: each move after a skip, until every probed axis is set by an absolute word, has the joints still at the stop depend on u ∈ [0, 1], and collide checks it over (u, s) ∈ [0, 1]² with interval poses (`check_after_skip`). When every such move is proven clear, `skip.end` narrows to the material removal after it. It is not narrowed when a move is not one straight segment (an arc, a dogleg rapid, a tool change), or when a skip follows a skip before its axes are set again (`skip_unmodelled`). |
| probing macros (gcode.rs shell, sim.rs) | G65 P<n> only where the dialect's `probe_on` / `probe_off` / `probe_protected` / `probe_surface` templates cite the macro (Renishaw Inspection Plus on Haas: P9832, P9833, P9810, P9811), modelled from the macros' published descriptions, outside the proven core. Probe on/off: a small Z move of unpublished size, not simulated (`probe.switch` not checked; Z unknown to the interpreter afterwards). Protected positioning: a feed move to the target at its own modal F (a probe contact on it is a crash). Single-surface measure: the search along its axis to `Q` past the surface, swept like a skip move (the trigger from the stock as cut), then back to the start, so the position after it is known; its time is not modelled (`probe.measure_time` not checked). Positions in G91 are refused. The switch macros' rules (H-2000-6222 p. 9-23) are enforced here and in forge-post: no tool change while the probe is on, and the move after switching it off is a G28 reference return |
| SINUMERIK 840D native (siemens.rs) | the `siemens-840d` control (data/controls/siemens-840d.toml): a subset of 840D native programs read, each block rewritten into the shell's words, outside the proven core; anything outside the subset refused by name (see "SINUMERIK 840D programs" below) |
| cutter compensation (comp.rs) | G41/G42 … G40 on the controls that state what they do: the SINUMERIK 840D native control (PGsl ch. 10: NORM, G450/G451; see "SINUMERIK 840D programs") and LinuxCNC (its interpreter's documented source, v2.9.9 `interp_convert.cc`, cited in forge-post's `linuxcnc` dialect as `comp_corner = "arc"`, `comp_entry = "perpendicular"`, `comp_corner_tolerance = 0.05`). Outside the proven core: the programmed path between G41/G42 and G40 is replaced by the tool centre's before the simulation, from each tool's `radius` register (wear mode: the wear; control mode: radius + wear; a negative value the other side). LinuxCNC: the entry move ends perpendicular to itself (an error unless longer than the radius) and meets the first block at a corner; an outside corner gets a circle about it, an inside one the intersection; a turn within 0.05 rad neither (the next line starts where the last ended); the move after G40 runs straight from the compensated end to the programmed point and is not an arc. Other dialects refuse G41/G42 (their corner behaviour is not published). Refused everywhere: rapids or blocks with no X/Y motion while on, arcs outside G17, tool or offset changes while on, blocks the compensation shortens past their start (the control's look-ahead is not modelled), arcs the tool does not fit |
| kinematics (kin.rs) | joint values to link placements, dmath rotations (exact at multiples of 90°), and the same over intervals |
| timing (sim.rs, lookahead.rs) | `t_min` a lower bound (feed and axis limits; G95 with G96 integrated exactly on lines, the clamp's bound on arcs); `t_stop` exact stops with trapezoidal acceleration, an estimate; `t_la` (and `seg_la` per segment) a lookahead estimate: grbl v1.1h's planner restated (planner.c, motion_control.c): each block's acceleration limited by every axis along it, a junction's speed v² = a δ sin(θ/2)/(1 − sin(θ/2)) from the junction deviation δ, trapezoids planned by a reverse and a forward pass over a window of the next blocks ending at rest, arcs cut into chords by the arc tolerance; each block's cruise is the simulation's own feed bound (`t_min`'s). The plan stops at tool changes, dwells, spindle and coolant changes and jumps; the program's `t_la` and `t_scurve` add the time `t_min` spends off the segments (tool changes, dwells, indexes, pallet changes), so neither comes out under `t_min` on a program. Defaults grbl's (15 blocks, δ 0.01 mm, arc tolerance 0.002 mm); a joint with no `accel` takes 1000 mm/s² or 3600 deg/s² (`accel_missing` says so). `lookahead::plan` takes other parameters (acceleration scale, δ, window). An estimate of one class of control, not a bound; on lines t_min ≤ t_la ≤ t_stop. `t_scurve` (and `seg_scurve`): the cycle-time ESTIMATE, a jerk-limited planner (`lookahead::plan_scurve`): per-axis acceleration and jerk along each block, S-curve speed changes, corners smoothed within a tolerance (SINUMERIK G642, the blend radius by grbl's geometry), arcs held to √(A r) and ∛(J r²), passes over each run between stops; cited: SINUMERIK SOFT (PGAsl 03/2013 10.3.1 p. 457, PGsl 11.2 p. 315), G642 (PGsl 11.2 p. 309), CYCLE832 (PGZ 11/2006 3.17). Its assumptions are a stated DEFAULT set (`SCurve::default()`: 1000 mm/s², 20,000 mm/s³, 0.02 mm; a record's `accel` where it has one; jerk is in no record), to be replaced by the machine's values; records keep both UNKNOWN. Shown beside `t_min`, the guaranteed lower bound, never alone. Against the one real machine seen (research/cam/timing/rptu-u5-620: three programs on a Spinner U5-620, SINUMERIK 840D) a parameter set fitted on one program predicts the other two programs' totals within 11 %, per operation within 12-50 %; the parameters are not identified across programs. Open: on that machine 3+2 arcs under TRAORI ran at about 40 % of F (CYCLE832's tolerance on rotary axes is a candidate), not modelled |
| collision (collide.rs) | every pair of bodies on different links through every segment: interval poses and separating-axis lower bounds prove Clear; a penetration witness gives Hit; otherwise Near (refused). A Hit's witness is a point in the base (machine) frame at the segment parameter `s` |
| removal (removal.rs) | a height map (milling, tool on the part's +Z) or a radius map (turning) at grid nodes, in floating point: rapids into material and tool pieces (shank, holder) into material, with the stock as cut so far. A touch probe (`probe = r`) removes nothing: on a skip move its ball's first contact with the stock as cut is the trigger (`triggers`: line, path parameter, ball tip, node; none = it ran to its end without contact, which a probe alarms on), on any other move a contact is a crash (`probe_hits`) |

Native and wasm32 give the same bits (tests/bits.rs).

### Removal: the tip's two forms

Milling removal finds, at each grid node, the lowest the cutter comes over a segment, which takes
the tip (in the part's frame) at many points s along the segment. Every segment removal accepts
keeps its rotaries in place, so the tip is an affine function of the linear joints. There are two
ways to compute it:

- **chained** (`Setup::tip_along`): at each s, the pose chain (every link's `then` from the base),
  the tool's tip through its link, and the inverse of the part frame. This was the only form
  before; it still checks the affine one.
- **affine** (`Setup::tip_affine`, what `mill` uses): the chained tip once, at the segment's start
  q0, plus, for each moving linear joint j, (q_j(s) − q0_j)·c_j. The column c_j is the joint's
  axis carried by the rotations above it into the part's frame: `+` when the joint is on the tool's
  chain, `−` when it is on the part's, and none when it is on both (it moves both). It is
  about 5.7 times faster (flange pocket, 0.5 mm grid: 20 s to 3.5 s).

**The bound.** Both forms are measured against T\*: the same chain in exact arithmetic, using
the rotation matrices as computed. Those matrices are the same bits in both forms and at every s,
because a linear joint's `then` multiplies by the identity exactly. With the rotations held to
those values, T\* is exactly affine, and c_j is its exact derivative evaluated in floating point.

Terms, all coordinates in the ∞-norm, and first order in u (O(u²) dropped):

- u = 2⁻⁵³.
- k: the number of joints on the tool's chain plus the number on the part's chain.
- n: the number of moving linear joints.
- M: twice the sum of the tool point's magnitude (mount point plus tip), the part mount's point,
  each linear chain joint's largest |q| over the segment (an arc's centre plus its radius), and
  3·|pivot| for each rotary chain joint. M bounds every vector and partial sum in either form.
  A computed rotation's row has an absolute sum of at most √3 < 2.

- chained: each `then` or `apply` (a 3-term dot plus a translation) adds at most
  4u(2M + M) + 2uM = 14uM, and an error carried in passes through times at most 2. The tool's
  side contributes (k_t + 1)·14uM, the part frame's side (k_p + 1)·14uM, and the inverse and the
  last apply add 18uM. So |chained − T\*| ≤ **(28k + 74)·u·M**.
- affine: the start tip is a chained tip, with the bound above. Each c_j is two 3-term products of
  a unit axis, so it is within 24u of exact and |c_j| ≤ 2. The sum base + Σ d_j·c_j adds
  (n + 1)(4n + 1)·u·M, and the errors in d_j and c_j add 52n·u·M. So |affine − T\*| ≤
  **(28k + 74)·u·M + ((n + 1)(4n + 1) + 52n)·u·M**.
- Their difference is at most the sum of the two:
  **2(28k + 74)·u·M + ((n + 1)(4n + 1) + 52n)·u·M**, which is about 1e-10 mm on the built-in mills.

tests/removal_tip.rs checks every segment of the flange-sized pocket (CAM-SIM's grid, 265
segments, 1025 values of s each). It runs on the 3-axis mill and on the trunnion turned to
C−121.25°, where the sine and cosine are not exact:

| machine | largest tip gap | largest bound | largest height gap |
|---|---|---|---|
| 3-axis mill | 3.6e-15 mm | 8.7e-11 mm | 0 |
| trunnion at C−121.25° | 1.4e-14 mm | 1.0e-10 mm | 7.7e-14 mm |

**The nearest point in closed form.** Most moves keep the tip at one height. That covers a line
whose ends have the same z, and an arc whose two joints' columns are a unit vector and its
quarter turn in the part's xy plane with no other joint moving. On such a move the cutter's
profile only rises with distance, so a node's lowest cutter height is z + g(d_min), with d_min
the distance from the node to the path's nearest point. `mill` finds it in closed form:

- for a line, the distance to the segment;
- for an arc, the candidates are, for each turn of the node's own direction that lies within the
  sweep, the radial distance (a circle) or a few Newton steps on the squared distance (a spiral:
  a rounded end radius makes the control's radius change linearly along the arc), and the two
  ends.

Every other move is searched as before: a ternary search to 1e-13 in s on a line, 64 samples per
eighth of a turn and then a ternary search on an arc. The flange pocket goes from 2.8 s to 19 ms.

Against the reference (chained tip and the search), a height differs by at most
**δ + δd + √(2·corner·δd)**:

- δ is the tips' bound above;
- δd = 2·1e-13·L + 2δ + 16·u·Mxy bounds how far the two nearest-point distances can be apart.
  L is the path's xy length, so 1e-13·L covers the search's last bracket; Mxy bounds the
  coordinates.
- The profile g has slope 0 under the flat, and the corner's part, c − √(c² − e²), is ½-Hölder
  with constant √(2c).

That comes to about 2e-5 mm on the flange. The real gaps are far smaller: tests/removal_tip.rs
checks the closed form and the affine search against the reference on the flange pocket and on
arcs that cut fresh stock (part turns, full turns, both senses, rounded ends), on the 3-axis mill
and the trunnion at C−121.25°:

| program | machine | closed form vs reference | affine search vs reference | bound |
|---|---|---|---|---|
| flange pocket | 3-axis mill | 3.6e-15 mm | 0 | 2.0e-5 mm |
| flange pocket | trunnion at C−121.25° | 1.3e-13 mm | 7.7e-14 mm | 2.2e-5 mm |
| rings | 3-axis mill | 1.7e-10 mm | 7.1e-15 mm | 1.9e-5 mm |
| rings | trunnion at C−121.25° | 3.9e-14 mm | 1.6e-14 mm | 2.0e-5 mm |

The closed form changes the removal heights' bits, so tests/bits.rs is re-pinned (0x8329c333c7dd69f8,
native and wasm32). The kernel's bits.txt is untouched.

A height follows the tip with slope 1 under the cutter's flat. Under its corner it follows with
the corner's slope. At a node that the cutter's rim only grazes (distance r from the path), it is
not continuous in the tip or the distance at all: a gap of δ can add or drop that node's cut. The
test asserts the heights are within the bound on its grids, and they are; that says nothing about
other grids. The affine form alone left the pinned digests (tests/bits.rs, and the flange pocket's
0xa9118efcb77c7167) unchanged; the closed form moves both.

## SINUMERIK 840D programs

`dialect = "siemens-840d"` reads the 840D native programs of the Spinner U5-620 dataset
(tests/data/spinner-u5, CC BY 4.0, ElMoe production data set @b8003660): the subset they use,
from Siemens' published manuals. The control's `[[dialect.cite]]` entries give the page of each
word: PGsl (SINUMERIK 840D sl Fundamentals 03/2013, 6FC5398-1BP40-3BA1), PGAsl (Job Planning
03/2013), PGZ (Cycles 11/2006). Each block is rewritten into the shell's words and run by the
same core; what the subset does not hold is refused with the block's line and the word named.

| read | as |
|---|---|
| `;` comments, `N` numbers, `MSG("…")` | comments (PGsl 14.1 p. 367) |
| `DEF REAL a, b`; a block of only assignments `a=1.5 b=-2.` | declared variables with number values (PGAsl 1.1.4 p. 24); a block that mixes assignments and words, an expression, or an undeclared name is refused |
| `X=a` (an axis letter `=` a variable or number) | the word with the value; `AC(…)`/`IC(…)` refused |
| G0 G1 G2 G3 G17 G90 G94 G4 | the core's (PGsl 9.4–9.6, 8.2, 8.3.1, 7.1, 14.7); `TURN=0` read, `TURN=n` (n > 0) refused (PGsl 9.7 p. 211) |
| `G710` | `G21` (metric, PGsl 8.3.5 p. 165) |
| `G60 G601 FNORM COMPOF TRAFOOF` | no effect here: exact stop (what `t_stop` models), the default feed, no compressor, no transformation (PGsl 11.1; PGAsl 10.2, 4.4, 6.11) |
| `SUPA` | `G53` for the block (PGsl 8.1 p. 149, 12.9 p. 354). SUPA suppresses frames, not the tool offset: a SUPA block with Z and an offset in force (D ≠ 0) is refused |
| `T="name"`, `T0`, `M6` | the setup tool of that `name`; T0 M6 empties the spindle (PGsl 4.2.2 pp. 58–59). An unknown name is refused |
| `D0`, `D1` | G49 / the loaded tool's length (G43 H of its number) (PGsl 5.6 p. 73). After M6 the length is machine data until a D is programmed: a Z move before it is refused. A D with no tool loaded by the program selects an offset the manual does not state: reported not modelled, and a Z move under it is refused. D2 and up refused |
| `MCALL CYCLE81(RTP, RFP, SDIS, DP)` … `MCALL` | modal drilling: after each later block with path motion, rapid to RFP + SDIS (toward RTP), feed to DP, rapid to RTP (PGZ 2.1.3 pp. 42–44; PGAsl 1.24.3.4 p. 191). All four given; DPR and other cycles refused |
| `G41`/`G42` … `G40` with `NORM` and `G450`/`G451` | cutter radius compensation (comp.rs, PGsl 10.1–10.3): each line shifted by the register's radius R to the side (G41 left of travel, G42 right; a negative R the other side), each arc about its centre with radius ∓ R; the G41/G42 block straight to the first contour block's start displaced perpendicular to it (NORM), the G40 block straight from the last one's displaced end; an outside corner on a circle of radius R about it (G450) or to the equidistants' intersection (G451), an inside one to the intersection. R is the tool's `radius` in the setup: with the path compensated in CAM, the wear; with the contour programmed, the radius plus the wear. NORM and G450/G451 must be programmed (their defaults are machine data, MD20150). Refused: KONT/KONTC/KONTT, a rapid, a block with no X/Y motion, an arc outside G17, a D or tool change while on; equidistants that do not meet within their blocks (a bottleneck: the control's look-ahead, CDON/CDOF, PGsl 10.5, is not modelled); an arc whose compensated radius is ≤ 0. Positions rounded to the control's units (each within half a unit of the exact) |
| `CYCLE832(…)` | not modelled (`unchecked`: `cycle832`): its tolerance and mode select the machine's smoothing, compression and velocity control (PGZ 3.17 pp. 265–268), so the time under it is not `t_min`'s; `t_min` stays a lower bound only up to that, `t_stop` assumes exact stops |
| `CYCLE800()` | taken as no motion, reported not modelled (`cycle800`): it deselects the swivel data record (PGZ 3.16 p. 216); a machine's cycle settings may retract |
| an M in the setup's `m_no_motion` | no motion, reported not modelled (`M27` …) |

3+2 machining (the dataset's Coordinate_system_representation_Achskreuz):

| read | as |
|---|---|
| `TRAORI` (alone in its block), with B and C positioned before it and held | the orientation transformation (PGAsl 6.2 pp. 307-310): every point after it is taken through the programmable frame about the work offset's zero into the table's frame, then through the table's pose to the machine; the linear joints are solved for the tool tip there (sim.rs `traori_q`) |
| `AROT X…` / `Y…` / `Z…` (one axis per block) | an additive rotation of the programmable frame about its current axis, right-handed (PGsl 12.4 pp. 333-334): post-multiplied. Checked on all 21 TRAORI operations of the program: each frame's Z is the tool's axis at its B and C (the alternatives match 0 to 5 of them) |
| `TRANS X0 Y0 Z0` | resets the programmable frame (PGsl 12.3.1 p. 329: TRANS resets every frame component); TRANS with an offset is refused |
| `TRAFOOF` | the transformation off |

Under TRAORI arcs are read in G17 only when the frame's Z is the tool's axis (the circle stays one in
X and Y). Refused under TRAORI: a rotary axis moving (simultaneous 5-axis), SUPA, G53 or G28, a tool
change, cutter compensation, a modal cycle call, a work offset on a rotary axis. Frames outside
TRAORI are refused. tests/traori.rs runs the program on the U5-620's kinematics fitted from the
same dataset's log and finds the machine where the log has it: within 0.004-0.28 mm on two
operations (B90 C0, and B90 C90 with a different frame chain).

Refused by name (not in the subset): TRAORI with arguments, ATRANS, ROT, AROTS, MIRROR, SCALE,
COMPON and the other compressors, G64x, FFWON/FFWOF, SOFT/BRISK, other transformations, ORIWKS/ORIMKS,
IF/GOTO and any token that is not a single address letter with a number.

## Several channels (`multi`)

A Swiss lathe, a twin-turret lathe, a mill-turn's sub-spindle: each channel's program runs on its
own letters (the joints with that channel digit: Z2, C2 for channel 2; a setup's `channel`), and the
channels hold for each other at waits (the dialect's `wait_m = [lo, hi]`, e.g. Fanuc's M100–M199).
`multi::run(def, channels, work)` returns each channel's timeline, each segment's time window,
each channel's own collision report, the contacts between bodies of different channels, and where
the time bounds were open. A wait some channel never reaches is refused (deadlock), naming the
lines; a body moved by joints of two channels is refused.

What is claimed, for every timing the bounds allow:
- A move takes at least its `t_min` and at most its exact-stop time (the trapezoid under the
  declared rates and accelerations, at 100 % override). With a moving joint whose acceleration or
  rate is not declared there is no upper time: the windows after it are open, and the run says so.
- Each channel's clock is an interval [earliest, latest]; a wait releases its channels at the
  latest of their earliest arrivals and the latest of their latest ones.
- Each segment is checked as its motion, under way at some time in [earliest start, latest end],
  and its rest at the end pose until the latest start of the channel's next segment (the last
  rests to the end; the start pose rests until the first move).
- Two pieces of different channels are checked against each other, each over its whole motion
  and independently of the other (any relative timing), whenever their windows meet and no wait
  both channels share orders them. A contact reported this way may need a timing a real control
  would not take; a Clear holds for every timing within the bounds.

Not modelled: superimposition and synchronised spindles (a transfer is a wait with both chucks
closed on the part).

A setup of several channels gives `[[setup.channel]]` blocks; each is a channel's setup (`n`, its
`program`, its `wcs`, `tool`, `geo`, `part_mount`, `start_tool`, ...) on top of `[setup]`'s keys.
The stock and fixtures in `[setup]` belong to the first channel; another channel has only those it
gives itself. `forgecad machine SETUP.toml` then answers `forgecad.machine-multi/1`: per channel its
timeline, segment windows, own collisions and not-checked values; the cross-channel contacts; the
whole run's [earliest, latest] end; the open windows (`conservative`); a deadlock as
`{"deadlock": [{channel, line, wait}]}`. Exit 0 with no violation and no contact, 1 refused.

```toml
[setup]
machine = "generic-mill-turn"
dialect = "millturn-2ch"        # a dialect with wait_m = [100, 199]

[[setup.channel]]
n = 1
program = "millturn_ch1.nc"

[[setup.channel]]
n = 2
program = "millturn_ch2.nc"
part_mount = "sub"
```

Examples: examples/machine/millturn_2ch.toml (the head leaves before the sub-spindle comes in, at a
wait: ok) and millturn_2ch_nowait.toml (no wait: some timing within the bounds meets the head:
refused).

## Setup files

```toml
[setup]
machine = "generic-vmc3"
dialect = "fanuc"          # a forge-post dialect, or NAME.toml beside the setup
units = "inch"             # optional ("mm"): the program is in inches (G20)
program = "op1.nc"         # beside the setup
step = 0.5                 # removal grid, mm (optional: 1)
start_tool = 1             # the tool in the spindle at the start (optional)
m_no_motion = [27, 29]     # optional: machine-specific M codes the maker documents as moving
                           # nothing (each reported not modelled; any other unknown M is refused)
work_kg = 40               # part, fixture and pallet top on the part mount (optional)
# optional: channel = 1, tool_mount, part_mount, dogleg = false, feed_combined = true

[[setup.wcs]]              # per axis letter: the joint value at word 0
n = 54
z = -500

[[setup.tool]]
t = 1
name = "e10"               # optional: the name a 840D program's T="e10" selects
tip = [0, 0, -100]         # gauge point to tip, home pose
body = [5, 0, 30, 20, 30, 100]   # (radius, from, to) from the tip; the first is the cutting part
mill = [5, 0]              # radius, corner (or nose = [rn, dr, dz] for an insert, or probe = r:
                           # a touch probe's ball radius: it removes nothing)
h = 100                    # its length register (mills)
radius = 0                 # optional: its cutter radius register for G41/G42 (840D: D1's radius;
                           # LinuxCNC: half its tool-table diameter): the wear in "wear" mode
                           # (CAM compensated the path), radius + wear in "control" mode

[[setup.geo]]              # lathe offset registers, per letter (T0101 → n = 1)
n = 1
x = 120

[[setup.stock]]            # a box in the part frame, or bar = [r0, z0, z1]
min = [-50, -40, 0]
max = [50, 40, 10]

[[setup.fixture]]          # workholding, part frame: one box given here ...
min = [-60, -50, -20]
max = [60, -40, 5]

[[setup.fixture]]          # ... or a named body of boxes and cylinders (reported by name)
name = "vise"
[[setup.fixture.box]]
min = [-80, -60, -60]
max = [80, -45, 10]
[[setup.fixture.cyl]]
base = [70, 0, 0]
axis = [0, 0, 1]
r = 8
h = 25
[[setup.fixture.obox]]     # a block at an angle: o + s u + t v + r w, 0 ≤ s ≤ size[0], ...
o = [90, -10, 0]           # (u, v, w orthonormal)
u = [0.7071067811865476, 0.7071067811865476, 0]
v = [-0.7071067811865476, 0.7071067811865476, 0]
w = [0, 0, 1]
size = [40, 20, 15]
```

Examples: examples/machine (a slot, the same with a crash at N130, a faced and turned bar).
