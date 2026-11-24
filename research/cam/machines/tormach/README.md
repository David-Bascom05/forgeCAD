# Tormach machines in forge-machine

GENERATED: `kernel/forge-machine/data/machines/tormach.toml` and `parts/tormach/` come from
`tools/machines/tormach.py` (re-run after editing its facts). Source: tormach.com's machine pages
(first party, read 2026-11-23). The spec blocks as read are kept, restated as key/value pairs, in
`tormach-com-2026-11-23.json`.

| record | kind | notes |
|---|---|---|
| tormach-pcnc-440, -770m, -770mx, -1100m, -1100mx, -1500mx | vmc3 | bed mill: table X on saddle Y, head Z; column face from "spindle centerline to machine column" |
| tormach-24r | vmc3 | gantry router (Y gantry, X carriage, Z), table fixed |
| tormach-xstech, -xstech-pro | vmc3 | small routers; layout UNKNOWN (taken as gantry) |
| tormach-8l | lathe | quick-change tool post as a one-station turret |
| tormach-15lx-turret | lathe | with the optional 8-station turret (the gang-tool version needs FORMAT-GAPS #5) |

Not modelled: PCNC 1100 (older), 15L Slant-PRO (discontinued per its page), 15LX gang, the
optional ATCs (pockets, time and tool weight not on the pages), 4th axes, accessories.

Published and cited: travels, per-axis max feed (Tormach gives one rate, used for rapid and feed),
max rpm (and min where given: 1500MX 250, 24R 10,000, 8L 180), taper/nose, table size,
nose-to-table clearance, spindle-centreline-to-column, swing, the recommended 15LX chuck.

Estimated or unknown (each cite says which): where machine zero sits in the travel, the axis
layout (not stated on spec pages), head/nose/carriage sizes, table thickness, tool limits, turret
geometry, router nose height, the 1100M's table and clearance (not on its page). No power curve is
declared: Tormach publishes power without torque. Control: PathPilot is LinuxCNC-based, so the
mills use the `linuxcnc` dialect (an estimate) and the lathes the Fanuc-style lathe dialect.
