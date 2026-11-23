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

| 8 | Length joint (G43) on the table side: HMCs move the table on Z | (B) every HMC | allow a table-side `length_joint` (G43 shifts it by -H along its axis); see B10 below | open |
| 9 | Required fields often unpublished: rpm min, rapids, max feed, change/index time | (B) Mazak, Okuma, DMG MORI, DN lathes | optional with absence = no check, said in the reply; see B2, B11 | open |
| 10 | Duty-rated power (cont / 30-min / 10-min / S3) and max torque instead of a curve | (B) all makers | `ratings = [{kw, duty}]`, `torque_nm = {max, cont}`; see B3 | open |
| 11 | Live-turret lathes with C (and Y) don't fit `lathe` or `mill-turn` | (B) Lynx LMA, Puma Y, NLX, CLX, Spinner TC | turret `live` spindle allowed on a lathe with C; see B12 | open |
| 12 | Turning spindle on a mill's table rotary (stationary tool in the milling spindle) | (B) Brother M140X2/M200X3 | turning spindle with `c_joint` on table-table; see B5 | open |
| 13 | Changer: tool-to-tool and chip-to-chip, max diameter with empty neighbours; table/pallet load | (B) all | `tool_to_tool_s`, `max_dia_empty_adjacent`, part mount `max_kg`; see B8, B9 | open |

## Worker B detail (rows 2, 6, 8-13)

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
