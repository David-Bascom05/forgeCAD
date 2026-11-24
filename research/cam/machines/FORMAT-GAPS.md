# Machine-definition format gaps (MACHINE-LIB workers → coordinator → CAM-MACHINE)

The format (docs/cam-machine.md) is CAM-MACHINE's. Library workers list what real machines need and
the format can't say; the coordinator routes each to CAM-MACHINE. Status: open / agreed / done.
Add rows; don't rewrite others'.

| # | gap | needed by | proposal | status |
|---|---|---|---|---|
| 1 | Per-value provenance. A record has one `source`/`taken`; real-machine data needs a source, a date and published/estimate/unknown on each value. Required numeric fields can't say UNKNOWN. | all real machines | optional `[[machine.cite]] path = "joint.X.range" source = URL taken = date status = "published" \| "estimate" \| "unknown" note = ...`; a record with `real = true` must cite every required numeric field; an estimate or unknown value is still a number, the conservative one (larger envelope, smaller rate), and its cite says so | done (machines/base: `real`, `[[machine.cite]]`, validate_cites; approved by coordinator 2026-11-23) |
| 2 | Pallet changers (APC): pallets as part mounts that swap, change time | Haas EC-series, Hurco APC options, horizontals | `[[machine.pallet]]` with stations, `change_s`, part mount per pallet | open |
| 3 | Probing: a spindle probe or tool setter as a tool/fixed body with a trigger | most VMCs (Haas WIPS, Renishaw) | `[[machine.probe]]` kind spindle/setter, body, at | open |
| 4 | Gantry routers: two parallel motors on one axis (X1/X2 gantry), moving bridge | Tormach 24R, routers | allow one joint with a `slaves = [...]` note, or a `gantry = true` flag; kinematics unchanged | open |
| 5 | Twin-turret lathes, Swiss guide bushing and gang slides | (B), (C) workers | multiple turrets per channel; gang slide = turret with stations along a line (`index_axis` linear); guide bushing as a fixed part support body | open |
| 6 | Options and variants: one model with option sets (longer Z, Y axis, sub-spindle, 4th axis, high-speed spindle) | Haas VF-2 vs VF-2SS, ST-20 vs ST-20Y | either one record per variant (no format change, recommended for now) or `extends = "base-name"` with overrides | open |
| 7 | Bar capacity, chuck size, tailstock as data (lathes) | Haas ST, Tormach 15L | `[[machine.spindle]]` optional `bar`, `chuck`; tailstock as a joint (`W`/`B`-letter linear on the part side) plus body | open |
| 8 | Tool-change motion: a changer that moves joints before the change. A Haas mill's M06 "automatically moves Z up to the machine zero position ... the Z-axis is left at machine zero" (HFO Mill Programming manual, M06). forge-machine's M6 moves nothing, so a longer next tool starts where the last one's tip was: false hits, and the real Z-up move is never collision-checked | Haas mills (all), most VMCs | `[[machine.changer]]` optional `position = { Z = 0 }` (joint values the change goes to, in machine coordinates) and `left_at = "position"`; the sim adds a rapid there before the change and checks it | open |
| 9 | Letter B (and A) on a linear axis: Haas DS lathes are commonly said to drive the sub-spindle on a linear B (unverified; needs the lathe manual) | haas-ds-30y | allow `letter = "B"` with `type = "linear"` when declared explicitly, or a per-dialect letter map | open |
| 10 | A 4-axis mill whose one rotary is on the head (swivel head B, no table rotary): no kind fits (vmc4 = a table rotary) | Hurco VMX42SWi/60SWi/84SWi | allow vmc4 with the rotary on either side, or a `head4` kind | open |
| C8 | Manual tool change: hobby routers / desktop mills have no changer; sim refuses M6 without `[[machine.changer]]`; the grbl dialect's `M0` tool change carries no T word | (C): Shapeoko, Nomad, Onefinity, Avid, Genmitsu, Sherline, Bantam, MDX | no changer = manual: M6 = operator pause + swap, no pocket limit, time an estimate; grbl tool change `T{t} M6` or a comment the reader honours | agreed (coordinator), → CAM-MACHINE |
| C9 | Gantry routers fit `vmc3` only by counting (XYZ head-side, part on `base`); see also row 4 | (C): routers, Datron neo | `kind = "router3"` label, or confirm vmc3-by-count | CAM-MACHINE decides |
| C10 | Swiss-type: guide bushing body with its own clearance (tools ~0.5–1 mm from its face) and stock-outside-bushing limit; gang post T→station map; 2–3 channels run together with waits (merged timelines for inter-channel collision); headstock Z1 is a table-side linear | (C): Citizen, Star, Tsugami, Nomura | `[[machine.guide_bushing]]`, per-body `clearance`; `[[machine.gang]]` stations; setup with one program per channel + dialect wait codes (extends row 5) | open (smallest-first) |
| C11 | Unbounded rotary (Pocket NC B, ±9999 on the control) | (C): Penta | `cyclic = true` with the control's range | confirm pending |
| C12 | Dialects missing: milltronics (9000-DGI / INSPIRE), siemens (SYIL 828D, Grizzly Sinumerik), mach3 (Grizzly G0951), centroid-cnc12 (Avid EX, Sherline Acorn), syntec (Laguna SmartShop), pathpilot, carbide-motion, buildbotics, mach4, masso, centroid-acorn, datron-next, shopbot (OpenSBP, not G-code), roland RML-1/NC, fanuc-lathe-multipath (+ Citizen/Star/Tsugami M-codes) | (C) | findings only; no posts yet (coordinator) | finding |
| C13 | Manual tool post on a lathe (Sherline 8440, QCTP lathes): `lathe` kind requires a turret; modelled as a 1-station turret | (C) | `[[machine.toolpost]]` (one station, manual change) or turret `stations = 1`, `manual = true` | open |
| C14 | Control by option: many builders sell one model with Siemens, Fanuc, Heidenhain or Mitsubishi; `control` holds one | (C): Hwacheon, Kafo, Fryer, Johnford | `controls = [...]` (the options) with `control` the default, or one record per control | agreed to propose (coordinator) |

## Details: MACHINE-LIB C rows (C8–C12), 2026-11-23

### 8 (C1). Manual tool change (no changer)
Most hobby routers and desktop mills (Shapeoko, Nomad, Onefinity, Avid without ATC, Genmitsu,
Sherline, Bantam, Roland MDX-40A) change tools by hand: the control pauses on M6 (per each
vendor's docs, cited per machine in the library) or the post emits `M0` (our `grbl` dialect).
- Today: `changer` absent → sim refuses `M6` ("M6 on a machine with no tool changer"); the `grbl`
  dialect's `M0` carries no T word, so the simulator cannot know which tool follows.
- Meanwhile: library machines omit `[[machine.changer]]`; sample jobs use a single tool.
- Proposal (coordinator: "changer optional = manual change (M6 pause)"): no `[[machine.changer]]`
  means manual change: M6 = operator pause + tool swap, no pocket limit, its time an operator
  estimate marked as such. And the grbl dialect's tool change emitting `T{t} M6`
  (which grbl-family controls accept it: UNKNOWN, to check per control) or a `(TOOL n)` comment
  the reader honours.

### 4/9 (C2). Gantry routers: what `vmc3` means
Gantry routers (Shapeoko, Onefinity, Avid, ShopBot, Laguna, Datron neo) carry X, Y and Z all on the
head side; the part sits on the static bed (part mount on `base`). `kind = "vmc3"` counts only
"3 linear, 0 rotary", so it validates.
- Proposal (coordinator agreed to propose, 2026-11-23): `kind = "router3"`: X, Y, Z all head-side,
  part mount on `base`, ganged Y1/Y2 as one joint; until CAM-MACHINE adds it the library files
  use `vmc3`, which validates by count. The dual Y motors (Y1/Y2 slaved, squared at homing) are
  one joint kinematically: nothing to add unless racking is modelled. A router's 4th axis (rotary
  along X on the bed, e.g. Onefinity/Avid/ShopBot indexers) is `vmc4` with the rotary on `base`: should
  work, untested.

### 5/10 (C3). Swiss-type (sliding headstock) lathes
Citizen L/A/M, Star SR/SB, Tsugami B0, Nomura NN: the part is held in the main collet and the
**headstock moves in Z1** (a table-side linear joint under C1); the bar passes through a **guide
bushing** fixed to the frame; tools cut at the bushing face. Tools sit on a **gang tool post**
(X1/Y1 slide; a tool is selected by moving Y1, not by indexing), plus front drill sleeves, cross
live tools, and a **back tool post** for the sub-spindle (Z2/X2/C2). Two or three **paths
(channels)** run at once with wait codes.
- Kind: `mill-turn` validates (lin ≥ 3, turning ≥ 1, milling ≥ 1, table rotary ≥ 1) if live tools
  are modelled as a milling spindle; `lathe` refuses any rotary, so C1 forces `mill-turn`. Fine,
  but:
- Gap a, guide bushing: a static body the stock passes through. Collision of tools vs the bushing
  is the main crash on a Swiss (tools work within ~0.5–1 mm of its face) and `clearance` is one
  number per machine; the stock is not a body, so "stock leaves the bushing" (Z1 beyond the
  per-chucking stroke) and "the part is unsupported past N·d from the bushing" cannot be checked.
  Proposal: `[[machine.guide_bushing]] link = "base", at, axis, bore = [min, max]`, an optional
  per-body `clearance` override, and a removal-map limit: the turned length outside the bushing.
- Gap b, gang tool post: stations at fixed offsets on one slide, chosen by T number (each T moves
  the slide so its tool is on centre). Today a tool's position is `setup.tool.tip` from one tool
  mount; one mount per station is possible but the T→station map is not in the machine file.
  Proposal: `[[machine.gang]] link, stations = [[name, at, dir, kind], ...]` (kind turning /
  cross-live / end-live / drill-sleeve), with `live` naming the spindle that drives live stations.
- Gap c, multiple paths: letters carry a channel digit and a setup picks one channel. A Swiss runs
  $1/$2(/$3) together with waits (`M200`-range on Citizen, `!n L` on Fanuc multi-path). Collision
  between the channels needs both programs interleaved. Proposal: a setup lists one program per
  channel; the interpreter reads the dialect's wait codes and the sim merges the timelines.
- Gap d, dialect: Swiss controls are Fanuc multi-path (0i-TF/31i/32i) or Mitsubishi M8 series
  with vendor M-codes (pick-off, cut-off check, bushing). No `fanuc-lathe-multipath` / vendor
  M-code tables yet (CAM-TURN owns lathe dialects).

### 11 (C4). Penta Pocket NC / Solo kinematics
Table-table: A tilts a trunnion that carries B (continuous); X moves the rotary assembly, Y and Z
move the spindle. Fits `table-table` (3 linear on either side). Open only if B's range is
unbounded (±9999° on the Kinetic Control): `cyclic = true` with a large range is our reading.

### 12 (C5). Dialects the library needs and forge-post lacks
Present: fanuc, haas, linuxcnc, grbl. Missing for this vendor set:
- `pathpilot` (Tormach, also Penta's LinuxCNC-based Kinetic Control is close to `linuxcnc`).
- `carbide-motion` (GRBL 1.1 subset + M6 prompt) — maybe a grbl variant (C1).
- `grblhal` (Onefinity Elite/Buildbotics? check), `buildbotics` (Onefinity classic controller:
  its own G-code subset based on LinuxCNC's), `mach4` (Avid, many conversions), `masso`,
  `centroid-acorn`.
- `datron-next` (Datron's control: SimPL programs; ISO G-code import exists — UNKNOWN subset).
- `shopbot` (OpenSBP: not G-code at all; a different post shape).
- `roland-rml1` (RML-1 / Roland NC code on MDX; MDX-50 also takes NC code = ISO subset).
- `fanuc-lathe-multipath` + Citizen/Star/Tsugami M-code tables (C3d).

| B8 | Length joint (G43) on the table side: HMCs move the table on Z | (B) every HMC | allow a table-side `length_joint` (G43 shifts it by -H along its axis); see B10 below | open |
| B9 | Required fields often unpublished: rpm min, rapids, max feed, change/index time | (B) Mazak, Okuma, DMG MORI, DN lathes | optional with absence = no check, said in the reply; see B2, B11 | open |
| B10 | Duty-rated power (cont / 30-min / 10-min / S3) and max torque instead of a curve | (B) all makers | `ratings = [{kw, duty}]`, `torque_nm = {max, cont}`; see B3 | open |
| B11 | Live-turret lathes with C (and Y) don't fit `lathe` or `mill-turn` | (B) Lynx LMA, Puma Y, NLX, CLX, Spinner TC | turret `live` spindle allowed on a lathe with C; see B12 | open |
| B12 | Turning spindle on a mill's table rotary (stationary tool in the milling spindle) | (B) Brother M140X2/M200X3 | turning spindle with `c_joint` on table-table; see B5 | open |
| B13 | Changer: tool-to-tool and chip-to-chip, max diameter with empty neighbours; table/pallet load | (B) all | `tool_to_tool_s`, `max_dia_empty_adjacent`, part mount `max_kg`; see B8, B9 | open |

## Worker B detail (rows 2, 6, B8–B13)

## B1. Pallet changers (APC) — needed for every HMC and APC VMC
- Published: pallet count (2, or more on pools), pallet size, pallet change time (s), max load per
  pallet, sometimes the pallet's max work diameter x height.
- Format: one `[[machine.changer]]` for tools only; a part mount on one link. No pallet event.
- Meanwhile: the HMC's pallet in the machining position is the part mount (on B); the APC numbers
  are in `note` only. Kind `vmc4` (3 linear + 1 table rotary) is used for the HMC.
- Asked: `[[machine.pallets]]` { count, change_s, max_kg, size, max_work = [dia, h] } and a pallet
  change (M60-style, dialect-specific) as a timeline mark; optionally kind `hmc` so "vmc4" isn't
  read as a vertical machine.
- Machines: (B) Makino a51nx, DN Solutions NHP, Mazak HCN, Okuma MB-H, DMG MORI NHX, Kitamura HX.

## B2. Spindle minimum rpm is usually unpublished
- `rpm = [min, max]` is required. Most makers publish only the max (Brother publishes 10).
- Meanwhile: min = 1 with a comment "UNKNOWN (unpublished): 1 = no lower check".
- Asked: `rpm = [max]` or `rpm_min` optional, absence = no lower check, said in the reply.

## B3. Spindle power is published as duty ratings, not curves
- Published: continuous / 30-min / 10-min / 1-min / S3-25% kW, max torque, sometimes rated torque,
  rarely a chart with readable points. Fusion keeps (P, Tpeak, n at Tpeak).
- Format: `curve_rpm`/`curve_kw` (piecewise-linear) or nothing (no power check).
- Meanwhile: no curve unless a chart's numbers are published; the ratings are in `note`.
- Asked: `ratings = [{kw, duty = "cont"|"30min"|"10min"|"1min"|"S3-25%"}]`, `torque_nm = {max, cont}`;
  a check that uses the continuous rating as the conservative bound.

## B4. Several sources per record
- `source` is one string; a machine's numbers often come from a web spec page plus a catalogue PDF.
- Meanwhile: URLs joined with " | " (the first is the primary).
- Asked: `source` may be a list.

## B5. Turning on a milling machine's table rotary (Brother SPEEDIO M140X2 / M200X3)
- A trunnion (A) carries C, a direct-drive turning spindle (2,000 rpm, 55 N.m); the milling spindle
  holds a stationary turning tool (spindle locked). Kind `mill-turn` requires a diameter X; kind
  `table-table` forbids a turning spindle.
- Meanwhile: table-table, C as a rotary joint only; turning noted.
- Asked: allow a turning spindle with `c_joint` on table-table (or a kind `mill-turn-table`), X
  programmed as a radius.

## B6. Machine zero position within the travel
- Makers publish strokes, not where the machine zero (reference point) sits in them.
- Meanwhile: X/Y 0 = mid-travel, Z 0 = top (mills); lathes put X's top at the max turning radius,
  Z's at max turning length + 50 (estimates). A real G54 differs by a constant per axis.
- Asked: nothing structural; a `home`/`zero` note convention, or an optional `ref = "plus"|"minus"`
  per joint so posted G53 moves land right when the maker publishes it.

## B7. Variants and options
- One model = several spindles (10k/12k/15k/20k/24k), magazines (21/30/40/60), tailstocks, Y axes,
  high columns. Published as options on one sheet.
- Meanwhile: the standard build is the record; options listed in `note`.
- Asked: a way to declare an option set that patches a base record (`[[machine.variant]] base =
  "...", spindle.rpm = ...`), or accept one record per variant.

## B8. Changer limits beyond pockets / time / weight
- Published: tool-to-tool and chip-to-chip times, max tool diameter with adjacent pockets full and
  empty, max tool length (sometimes per magazine option), max tool moment.
- Format: `change_s`, `max_tool_kg`, spindle `max = [length, diameter]`.
- Meanwhile: change_s = chip-to-chip (the longer, a fair timeline estimate; tool-to-tool in a
  comment); diameter = adjacent-full.
- Asked: `tool_to_tool_s`, `max_dia_empty_adjacent`.

## B9. Table load
- Published: table / pallet / rotary max load (kg) and sometimes inertia (kg.m^2).
- Asked: `max_kg` (and `max_inertia`) on part mounts, checked against setup stock + fixture mass.

## B10. G43's length joint on the table side (HMCs that move the table on Z)
- Many HMCs move the table (pallet + B) along Z toward a column carrying X and Y. `length_joint`
  must be a linear head joint (validate refuses `Z` on the table side).
- Meanwhile: Z modelled on the head (X -> Y -> Z); the tool-to-part motion is the same, the
  static/moving split of bodies is not (the table becomes static, the head carries Z).
- Asked: allow a table-side length joint (G43 shifts the table joint by -H along its axis).
- Machines: Mazak HCN-5000 (and every HMC in the library).

## B11. Required fields the maker does not publish
- Rapids, max cutting feed, tool change time, turret index time are often absent (Mazak QT-200
  publishes none; Okuma and DMG MORI hide full data behind a sign-up). `change_s`/`index_s` must be
  > 0; `rapid`/`max_feed` required.
- Meanwhile: the value that keeps every check vacuous and t_min a lower bound: rapid 100 m/min
  (rotary 1000 rpm), max_feed = rapid, change/index 0.001 s, each marked "UNKNOWN (unpublished)"
  beside the field.
- Asked: make these optional with absence = "no check / no time", reported in the reply's claims,
  so no stand-in numbers are needed.

## B12. Lathes with live tools, C and/or Y on a turret
- Published widely (DN Lynx LMA, Puma Y/SY, DMG MORI NLX/CLX, Spinner TC, Mazak QT MY): a turret
  with driven tools, a C-axis main spindle, often Y, sometimes a sub-spindle on W/B.
- Format: kind `lathe` forbids rotaries and milling spindles; kind `mill-turn` asks for lin >= 3 and
  a milling spindle (a live turret without Y has lin 2: refused).
- Meanwhile: these are modelled as their 2-axis lathe (X, Z, turret), the rest named in `note`.
- Asked: a turret `live` spindle counted as milling for a `lathe` kind with C (lin 2 or 3), and a
  sub-spindle on a lathe.

## B13. Things the format may want but that makers rarely publish
- Rotary tilt ranges (DMG MORI DMU 65, DN DVF 5000 2nd gen, Okuma MU-5000V, Grob G350, Kern Micro HD,
  Hyundai WIA XF6300, Chiron FZ 16 S: not published on the reachable pages): those machines are left
  out rather than guessed.
- The sense of + on a tilt (which way the table tips): never published; modelled right-handed.
- Pivot height of a trunnion below the table top: never published; estimates named in each note.
