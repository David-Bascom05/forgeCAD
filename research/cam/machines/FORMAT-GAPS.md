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
