# Machine-definition format gaps (from the vendor library work)

Found while restating makers' published specifications into docs/cam-machine.md's format. Each gap
says what the makers publish, what the format can hold today, what the library does meanwhile, and
the extension asked for. Extensions go through the coordinator to CAM-MACHINE (owner of the format).
Workers append; keep one entry per gap, list the machines that hit it.

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
