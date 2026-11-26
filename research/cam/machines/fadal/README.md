# Fadal machines in forge-machine

GENERATED: `kernel/forge-machine/data/machines/fadal.toml` and `parts/fadal/` come from
`tools/machines/fadal.py`, reading `fadal-com-2026-11-23.json` (the spec tables of fadal.com's
model pages, first party, restated). These are the current Fadal models (the brand is in
production again), NOT the pre-2010 classics (VMC 15/3016/4020 of the original company), whose
spec sheets we have no public source for.

Records (vmc3): fadal-vmc-2015h-s-ii (BT30), -vmc-2520, -vmc-3320, -vmc-4020b-ii, -vmc-6032,
-vmc-6032-50t, -vmc-8032, -vmc-8032-50t. The VMC 4022 page lists only travels and table: not
modelled.

Published: travels, nose-to-table range, spindle-centre-to-column (positions the column body),
table, max rpm, taper, rapids (4020B-II, 6032, 8032 families), tool capacity, weight, diameter
(with adjacent tools, the smaller) and length. Starred page values are options: the standard
value is used.

Not published, stated in the cites: the control (the fanuc dialect stands in), tool change time,
cutting feed limits, rapids for the 2015H/2520/3320, power curve (torque and peak power without
rpm), head size, table thickness, where the travel sits on the table. A conflict: the 4020B-II
page's text says 900 IPM rapids, its table 1,000 (the record holds 1,000).

The column body starts 5 mm above the table top: with the travel taken centred, a full-height
column would meet the VMC-6032's table at full Y travel (the self-collision sweep caught it),
which the real machine does not, so the stand-in is wrong at table height and is not drawn there.
