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
| kind | required | `vmc3` `vmc4` `table-table` `head-table` `head-head` `lathe` `mill-turn` |
| control | required | the forge-post dialect its control speaks (machine and control kept apart) |
| source | required | where the numbers come from; `taken`, `note` optional |
| clearance | required | least gap (mm) the collision check holds between bodies |
| length_joint | mills: required | the linear head joint G43 moves (Z); lathes use X/Z offsets instead |
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

## `[[machine.spindle]]`

`name`, `link` (the link it rides on), `kind` (`milling` turns a cutter, `turning` turns the part),
`axis`, `nose` (a point on the spin axis: gauge line or spindle face), `rpm = [min, max]`, `max =
[length, diameter]` (milling) or `[swing, part length]` (turning). Optional: `curve_rpm`/`curve_kw`
(piecewise-linear power, rpm increasing; absent: no power check), `interface`, `c_joint` (the rotary
joint that positions this spindle as a C axis).

## `[[machine.turret]]`

`name`, `link`, `stations`, `index_s` (seconds per station), `center`, `index_axis`; optional `live`
(the milling spindle that drives live tools).

## `[[machine.changer]]` (at most one)

`pockets`, `change_s` (chip to chip), `max_tool_kg`. Optional: `kind = "manual"` (M6 stops the
program and the operator swaps the tool: no pockets and no `max_tool_kg`, no pocket check; the
timeline marks the stop), else automatic. Optional: `position_joints` and `position`,
where M6 moves the machine by itself before the swap (joint values, e.g. `position_joints = ["Z"]`,
`position = [0]` for a Haas, whose M06 sends Z to machine zero). The simulation makes that move as
a rapid with the old tool, timed and collision-checked; without it the move is not simulated and
every tool change reports `changer.position` not checked.

## `[[machine.mount]]`

`name`, `kind` (`tool` | `part`), `link`, `at`, `dir`. A tool mount's `at` is the gauge point and
`dir` points from it to the tool tip. A part mount's `at` is the fixture datum (table centre, chuck
face centre), `dir` the part's +Z and `xdir` its +X (orthogonal). Every machine has at least one of
each; a mill-turn has two part mounts (main, sub).

## `[[machine.body]]`

`name`, `link`, one or more `[[machine.body.box]]` (`min`, `max`, home pose) or
`[[machine.body.cyl]]` (`base`, `axis`, `r`, `h`). Optional: `part` (the real `.fg` part the
primitives envelope), `ignore` (bodies it touches by construction: ways, neighbours). Bodies on one
link are never checked against each other.

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

## Self-collision through travel

A machine's own bodies must not collide anywhere inside the declared travel outside its zones: a
pose the joints can reach where they do is wrong data (fix the geometry, the travel, or declare
the zone the control enforces). tests/collide.rs sweeps every built-in: for each pair of bodies, each
joint that moves one relative to the other through its whole range, with the others at every
combination of their ends; no Hit and no Near.

## Structural checks (`MachineDef::validate`)

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

## Simulation (posted G-code)

`forge-machine` reads the posted text itself, not the toolpath it came from, and drives the joints
with it. That is the check Fusion's simulation skips: Fusion simulates the toolpath before the
post.

```
forgecad machine list                       the machines (built in, then cam/machines/*.toml)
forgecad machine SETUP.toml [--json]        run a setup: exit 0 ok, 1 refused, 2 error
```

The reply (`forgecad.machine/1`): the timeline (`forgecad.machine-sim/1`: every segment's line,
kind, start, `t_min` and `t_stop`, end joints, tool; marks for tool changes, turret indexes,
dwells, spindle and coolant; violations: overtravel including an arc's extremes, feed clamps,
unknown tools, rpm out of range, pockets), the collisions, the rapids into material, the tool
pieces into material, the removal summary, and what is claimed.

| part | what it is |
|---|---|
| interpreter (gcode.rs) | words to control events in exact integers; agrees with forge-post's reader on its subset (L6, 240 programs × 4 dialects); its Lean spec refines forge-cam's post model, so posted programs do the planned motion (verify/forge-machine, L1) |
| kinematics (kin.rs) | joint values to link placements, dmath rotations (exact at multiples of 90°), and the same over intervals |
| timing (sim.rs) | `t_min` a lower bound (feed and axis limits; G95 with G96 integrated exactly on lines, the clamp's bound on arcs); `t_stop` exact stops with trapezoidal acceleration, an estimate |
| collision (collide.rs) | every pair of bodies on different links through every segment: interval poses and separating-axis lower bounds prove Clear; a penetration witness gives Hit; otherwise Near (refused). A Hit's witness is a point in the base (machine) frame at the segment parameter `s` |
| removal (removal.rs) | a height map (milling, tool on the part's +Z) or a radius map (turning) at grid nodes, in floating point: rapids into material and tool pieces (shank, holder) into material, with the stock as cut so far |

Native and wasm32 give the same bits (tests/bits.rs).

## Setup files

```toml
[setup]
machine = "generic-vmc3"
dialect = "fanuc"          # a forge-post dialect, or NAME.toml beside the setup
program = "op1.nc"         # beside the setup
step = 0.5                 # removal grid, mm (optional: 1)
start_tool = 1             # the tool in the spindle at the start (optional)
# optional: channel = 1, tool_mount, part_mount, dogleg = false, feed_combined = true

[[setup.wcs]]              # per axis letter: the joint value at word 0
n = 54
z = -500

[[setup.tool]]
t = 1
tip = [0, 0, -100]         # gauge point to tip, home pose
body = [5, 0, 30, 20, 30, 100]   # (radius, from, to) from the tip; the first is the cutting part
mill = [5, 0]              # radius, corner (or nose = [rn, dr, dz] for an insert)
h = 100                    # its length register (mills)

[[setup.geo]]              # lathe offset registers, per letter (T0101 → n = 1)
n = 1
x = 120

[[setup.stock]]            # a box in the part frame, or bar = [r0, z0, z1]
min = [-50, -40, 0]
max = [50, 40, 10]

[[setup.fixture]]          # boxes, part frame
min = [-60, -50, -20]
max = [60, -40, 5]
```

Examples: examples/machine (a slot, the same with a crash at N130, a faced and turned bar).
