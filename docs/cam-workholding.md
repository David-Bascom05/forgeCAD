# Workholding library (`kernel/forge-workholding`)

Design: research/cam/workholding.md. Records are `[[workholding]]` tables in our TOML subset,
read from the built-in set (`kernel/forge-workholding/data/*.toml`), then the user's
`<config>/cam/workholding/*.toml`, then the project's `cam/workholding/*.toml`; a name in an upper
level hides the same name below, and a name twice in one level is an error.

```toml
[[workholding]]
name = "kurt-dx6"            # unique
kind = "vise"                # see the table below
vendor = "Kurt"              # a maker's product: every value cited
product = "DX6 CrossOver"
unknown = ["jaw_thickness"]  # not published: no value; a body that needs it is refused
jaw_width = 152.4            # mm (N for forces, kPa for vacuum, degrees for angles)
...
[[workholding.cite]]         # which fields, from where, read when
fields = ["jaw_width", "max_opening"]
source = "https://..."
taken = "2026-11-23"
[[workholding.part]]         # optional: the real .fg part a body envelopes (parts/…)
body = "base"
file = "generic-vise-150/base.fg"
```

Stand-ins say `standin = true` and give their `basis`; a user's own values cite `source = "user"`. Vises whose jaw thickness or height is unpublished get a conservative envelope (see research/cam/workholding.md §6). A size the job measures is
given with the device (`Record::with("jaw_thickness", 22)`); the reply names the field when one
is missing.

| kind | needs | may give |
|---|---|---|
| vise, self_centring | jaw_width jaw_height jaw_thickness max_opening body_length body_width height (table to jaw top) | min_opening clamp_force grip repeatability weight |
| dovetail | body_length body_width height grip angle min_width max_width (tenon) | repeatability clamp_force |
| soft_jaws | width height thickness | |
| parallels | heights (list) width length | accuracy |
| strap_clamp | length width thickness stud_d stud_above | slot reach |
| step_block | step steps tread width | |
| toe_clamp, edge_clamp | body_length body_width height | range_min range_max clamp_force torque |
| fixture_plate | length width thickness pitch hole_d hole_offset | dowel_d dowel_h |
| zero_point | pitch module_d module_h stud_d | pull_force hold_force repeatability |
| riser | d height | |
| tombstone | width height depth | faces |
| vacuum | length width thickness vacuum seal_inset | flow |
| custom | `[[workholding.body]]` with `[[..box]]` min/max, `[[..cyl]]` base/axis/r/h | |
| chuck | jaws body_d body_len jaw_height jaw_len grip_min grip_max | max_rpm through_hole stroke grip_force |
| collet | range_min range_max nose_d nose_len | max_rpm collet_len collet_d |
| face_driver | body_d body_len range_min range_max | pins |
| centre | angle body_d point_len | quill_d max_rpm |
| steady_rest | range_min range_max frame_r width | |

Placing (`mill::*`, `lathe::*`) sizes a device from the stock and poses it in the part frame;
a `Workholding` gives `faces()` (the certified checker, G2), `prims()` (the machine simulation),
`held()` (the leftover report), `reach()` (the least stick-out over the bodies); the lathe gives
envelopes of revolution (`lathe::to_region` for forge-turn), `check_path`, `stick_out` and the
`overhang` guideline (3 × the gripped diameter, a machine maker's manual: a note, not a refusal).
Checks are typed errors with the fix: opening, clamp depth, parallels in the set, tenon in the
dovetail's range, strap nose/heel/stud placement, dowels off the stock, a stock on its plate or
face, the vacuum hold-down rule μ·Δp·A ≥ SF·F_side (Schmalz states F_h = A·Δp·μ, μ ≈ 0.2–0.5:
https://www.schmalz.com/en/support/know-how/glossary/vacuum-clamping-technology, read
2026-11-23; the inputs are the user's), the bar in a chuck's or collet's range.
