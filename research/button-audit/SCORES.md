# Button audit scores

Every live command, worst first: verdict (good / fix / redo), P-level (STANDARD severity), the rules it breaks. Full detail and shots: research/ui-audit/report.md.

| Command | What a click does | Verdict | P | Score | Problems | Status |
|---|---|---|---|---|---|---|
| plate/3D Model/Draft | DIALOG | redo | P1 | 103 | no-focus: nothing has the keyboard when it opens; crash@narrow: min > max, or either was NaN. min = 356.5, max = 328.0 | fixed: ui/audit-words3 (batch 3, 480 px crash; = ui/bughunt-3 e90d49ec) |
| part/3D Model/Draft | DIALOG | redo | P1 | 103 | no-focus: nothing has the keyboard when it opens; crash@narrow: min > max, or either was NaN. min = 376.21875, max = 328.0 | fixed: ui/audit-words3 (batch 3, 480 px crash; = ui/bughunt-3 e90d49ec) |
| part/3D Model/Split | DIALOG | redo | P1 | 103 | no-focus: nothing has the keyboard when it opens; crash@narrow: min > max, or either was NaN. min = 378.46875, max = 328.0 | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| part/3D Model/Combine | DIALOG | redo | P1 | 103 | no-focus: nothing has the keyboard when it opens; crash@narrow: min > max, or either was NaN. min = 374.21875, max = 328.0 | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| part/3D Model/Delete Face | DIALOG | redo | P1 | 103 | no-focus: nothing has the keyboard when it opens; crash@narrow: min > max, or either was NaN. min = 378.875, max = 328.0 | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| start/Machine Builder: Machine Builder/Home Axes | NOTHING VISIBLE | redo | P1 | 100 | nothing: nothing a person can see happens; nothing@narrow: no popup, panel, status line or change in the view | fixed: machines/builder-off-reasons (MACHINE-BUILDER) |
| sketch/menu File/Save | NOTHING VISIBLE | redo | P1 | 100 | nothing: nothing a person can see happens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-shell (batch 1, state_status) |
| sketch/menu Browser: sketch row/delete | NOTHING VISIBLE | redo | P1 | 100 | nothing: nothing a person can see happens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-shell (batch 1, off with "Finish or cancel the sketch first") |
| sketch/menu Browser: sketch row/down | NOTHING VISIBLE | redo | P1 | 100 | nothing: nothing a person can see happens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-shell (batch 1, off with "Finish or cancel the sketch first") |
| sketch/menu Browser: extrude row/up | NOTHING VISIBLE | redo | P1 | 100 | nothing: nothing a person can see happens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-shell (batch 1, off with "Finish or cancel the sketch first") |
| sketch/Assemble/Pattern | NOTHING VISIBLE | redo | P1 | 100 | nothing: nothing a person can see happens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-shell (batch 1, off until a part is chosen) |
| sketch/Assemble/Mirror | NOTHING VISIBLE | redo | P1 | 100 | nothing: nothing a person can see happens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-shell (batch 1, off until a part is chosen) |
| sketch/Assemble/Copy | NOTHING VISIBLE | redo | P1 | 100 | nothing: nothing a person can see happens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-shell (batch 1, off until a part is chosen) |
| sheet/Flat Pattern: Flat Pattern/Bend Simulation | WORKSPACE | redo | P1 | 100 | crash@narrow: Negative width makes no sense, but got: -167 | with BENDSIM-ENV (480 px crash, bendsim_env.rs:347) |
| drawing/Manage/Update | NOTHING VISIBLE | redo | P1 | 100 | nothing: nothing a person can see happens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-words3 (batch 3) |
| drawing/Manage/Zoom All | NOTHING VISIBLE | redo | P1 | 100 | nothing: nothing a person can see happens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-words3 (batch 3) |
| plate/Assemble/Content Center | DIALOG | redo | P2 | 63 | tiny-text: "O-ring" at 9.5 px; tiny-text: "Plain washer, type A" at 9.5 px; tiny-text: "Hex cap screw" at 9.5 px; tiny-text: "Hex nut" at 9.5 px; tiny-text: "Button head socket cap screw" at 9.5 px; tiny-text: "Flat head socket cap screw" at 9.5 px; tiny-text: "Socket head cap screw" at 9.5 px; tiny-text: "Retaining ring for shafts" at 9.5 px; tiny-text: "Retaining ring for bores" at 9.5 px; tiny-text: "Parallel key, form A" at 9.5 px; no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, 11 px two-line family text) |
| sketch/Assemble/Content Center | DIALOG | redo | P2 | 63 | tiny-text: "O-ring" at 9.5 px; tiny-text: "Plain washer, type A" at 9.5 px; tiny-text: "Hex cap screw" at 9.5 px; tiny-text: "Hex nut" at 9.5 px; tiny-text: "Button head socket cap screw" at 9.5 px; tiny-text: "Flat head socket cap screw" at 9.5 px; tiny-text: "Socket head cap screw" at 9.5 px; tiny-text: "Retaining ring for shafts" at 9.5 px; tiny-text: "Retaining ring for bores" at 9.5 px; tiny-text: "Parallel key, form A" at 9.5 px; no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, 11 px two-line family text) |
| assembly/Assemble/Content Center | DIALOG | redo | P2 | 63 | tiny-text: "O-ring" at 9.5 px; tiny-text: "Plain washer, type A" at 9.5 px; tiny-text: "Hex cap screw" at 9.5 px; tiny-text: "Hex nut" at 9.5 px; tiny-text: "Button head socket cap screw" at 9.5 px; tiny-text: "Flat head socket cap screw" at 9.5 px; tiny-text: "Socket head cap screw" at 9.5 px; tiny-text: "Retaining ring for shafts" at 9.5 px; tiny-text: "Retaining ring for bores" at 9.5 px; tiny-text: "Parallel key, form A" at 9.5 px; no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, 11 px two-line family text) |
| sheet/Sheet Metal/Insert Hardware | DIALOG | redo | P2 | 63 | raw-text: "right_lip" (a raw key); raw-text: "left_lip" (a raw key); no-unit: "4.22" in the row of "M3"; no-unit: "4.8" in the row of "M3"; no-unit: "5.41" in the row of "M4"; no-unit: "6.9" in the row of "M4"; no-unit: "6.35" in the row of "M5"; no-unit: "7.1" in the row of "M5"; no-unit: "4.2" in the row of "M3"; no-unit: "5.4" in the row of "M4"; no-unit: "7.2" in the row of "M5"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, units via ux::show, what and source on hover) |
| sketch/Manage/Save | NOTHING VISIBLE | redo | P1 | 60 | nothing: nothing a person can see happens | fixed: ui/audit-shell (batch 1, state_status) |
| sheet/Bend Simulation: Bend Simulation/Suggest Order | NOTHING VISIBLE | redo | P1 | 60 | nothing: nothing a person can see happens | with BENDSIM-ENV (after the train-7 freeze; 480 px crash sent) |
| sheet/Bend Simulation: Bend Simulation/Simulate Again | NOTHING VISIBLE | redo | P1 | 60 | nothing: nothing a person can see happens | with BENDSIM-ENV (after the train-7 freeze; 480 px crash sent) |
| sheet/Bend Simulation: Bend Simulation/First Bend | NOTHING VISIBLE | redo | P1 | 60 | nothing: nothing a person can see happens | with BENDSIM-ENV (after the train-7 freeze; 480 px crash sent) |
| sheet/Bend Simulation: Bend Simulation/Previous Bend | NOTHING VISIBLE | redo | P1 | 60 | nothing: nothing a person can see happens | with BENDSIM-ENV (after the train-7 freeze; 480 px crash sent) |
| cam/Manufacture/Export NC | NOTHING VISIBLE | redo | P1 | 60 | nothing: nothing a person can see happens | fixed: cam/export-said (CAM-UI) |
| cam/Manufacture/Tool Library | DIALOG | redo | P2 | 59 | clipped: "Material"; clipped: "carbide"; overlap: "bt40-er32: BT40 taper and flange from the inter…" over "Ø6 · Flat end mill"; overlap: "bt40-er32: BT40 taper and flange from the inter…" over "flat-6 · 3 fl · 18 mm flute · carbide"; overlap: "bt40-er32: BT40 taper and flange from the inter…" over "T2"; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| sheet/Sheet Metal/PunchTool | DIALOG | redo | P2 | 51 | raw-text: status "PunchTool: Select a face (its wall) or a featur…" (an error code); raw-text: "right_lip" (a raw key); raw-text: "left_lip" (a raw key); no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/menu Browser: sketch row/delete | ERROR TEXT | redo | P2 | 45 | raw-text: status "Delete plan refused: StillRead: plate reads pla…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| plate/menu Browser: sketch row/down | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move plan to the end refused: StillRead: plate …" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| plate/menu Browser: extrude row/up | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move plate above plan refused: StillRead: plate…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: sketch row/delete | ERROR TEXT | redo | P2 | 45 | raw-text: status "Delete sk1 refused: StillRead: extrude1 reads s…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: sketch row/down | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move sk1 above sk2 refused: StillRead: extrude1…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: extrude row/delete | ERROR TEXT | redo | P2 | 45 | raw-text: status "Delete extrude1 refused: StillRead: sk2 reads e…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: extrude row/down | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move extrude1 above extrude2 refused: StillRead…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: extrude row/up | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move extrude1 above sk1 refused: StillRead: ext…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: hole row/delete | ERROR TEXT | redo | P2 | 45 | raw-text: status "Delete hole1 refused: StillRead: pattern1 reads…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| sheet/menu Browser: sketch row/delete | ERROR TEXT | redo | P2 | 45 | raw-text: status "Delete plan refused: StillRead: tray reads plan…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| sheet/menu Browser: sketch row/down | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move plan above right refused: StillRead: tray …" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| sheet/menu Browser: sheet row/delete | ERROR TEXT | redo | P2 | 45 | raw-text: status "Delete tray refused: StillRead: right reads tra…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| sheet/menu Browser: sheet row/down | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move tray above left refused: StillRead: right …" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| sheet/menu Browser: sheet row/up | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move tray above plan refused: StillRead: tray r…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| sheet/menu Browser: flange row/delete | ERROR TEXT | redo | P2 | 45 | raw-text: status "Delete right refused: StillRead: right_lip read…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| sheet/menu Browser: flange row/up | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move right above tray refused: StillRead: right…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| thermo/menu Browser: sketch row/delete | ERROR TEXT | redo | P2 | 45 | raw-text: status "Delete footprint refused: StillRead: body reads…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| thermo/menu Browser: sketch row/down | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move footprint above round refused: StillRead: …" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| thermo/menu Browser: extrude row/delete | ERROR TEXT | redo | P2 | 45 | raw-text: status "Delete body refused: StillRead: round reads bod…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| thermo/menu Browser: extrude row/down | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move body above cover refused: StillRead: round…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| thermo/menu Browser: extrude row/up | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move body above footprint refused: StillRead: b…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| thermo/menu Browser: fillet row/up | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move round above body refused: StillRead: round…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| weld/menu Browser: sketch row/delete | ERROR TEXT | redo | P2 | 45 | raw-text: status "Delete top refused: StillRead: fl reads top: th…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| weld/menu Browser: sketch row/down | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move top above bl refused: StillRead: fl reads …" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| weld/menu Browser: row row/delete | ERROR TEXT | redo | P2 | 45 | raw-text: status "Delete fr refused: StillRead: foot reads fr: th…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| weld/menu Browser: row row/down | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move fr above foot2 refused: StillRead: foot re…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| weld/menu Browser: row row/up | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move fr above bl refused: StillRead: fr reads b…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| weld/menu Browser: end_cap row/up | ERROR TEXT | redo | P2 | 45 | raw-text: status "Move foot above fr refused: StillRead: foot rea…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| plate/3D Model/Hole | DIALOG | redo | P1 | 43 | no-focus: nothing has the keyboard when it opens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| plate/3D Model/Thread | DIALOG | redo | P1 | 43 | no-focus: nothing has the keyboard when it opens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| part/3D Model/Hole | DIALOG | redo | P1 | 43 | no-focus: nothing has the keyboard when it opens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| part/3D Model/Thread | DIALOG | redo | P1 | 43 | no-focus: nothing has the keyboard when it opens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| part/3D Model/Direct | DIALOG | redo | P1 | 43 | no-focus: nothing has the keyboard when it opens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| sketch/3D Model/Hole | DIALOG | redo | P1 | 43 | no-focus: nothing has the keyboard when it opens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| sketch/3D Model/Thread | DIALOG | redo | P1 | 43 | no-focus: nothing has the keyboard when it opens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| assembly/3D Model/Hole | DIALOG | redo | P1 | 43 | no-focus: nothing has the keyboard when it opens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| assembly/3D Model/Combine | DIALOG | redo | P1 | 43 | no-focus: nothing has the keyboard when it opens; nothing@narrow: no popup, panel, status line or change in the view | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| cam/Manufacture/Thread Mill | DIALOG | redo | P1 | 43 | off-screen: 422x396 at (990,174) leaves the 1400x860 window; raw-text: "thread_mill2" (a raw key); no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| sheet/menu Browser: flange row/edit | DIALOG | redo | P2 | 42 | clipped: "right_lip.start_side"; clipped: "right_lip.end_side"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; clipped@narrow: "right_lip.start_side"; clipped@narrow: "right_lip.end_side" | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| sheet/Sheet Metal/Flange | DIALOG | redo | P2 | 42 | clipped: "right_lip.start_side"; clipped: "right_lip.end_side"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; clipped@narrow: "right_lip.start_side"; clipped@narrow: "right_lip.end_side" | open |
| sheet/Sheet Metal/Hem | DIALOG | redo | P2 | 42 | clipped: "right_lip.start_side"; clipped: "right_lip.end_side"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; clipped@narrow: "right_lip.start_side"; clipped@narrow: "right_lip.end_side" | open |
| plate/View/Section | ACTION | redo | P1 | 40 | nothing@narrow: not placed on screen: nothing to click | fixed: ui/audit-narrow (batch 5, tabs behind » reachable) |
| sketch/View/Section | ACTION | redo | P1 | 40 | nothing@narrow: not placed on screen: nothing to click | fixed: ui/audit-narrow (batch 5, tabs behind » reachable) |
| cam/Manufacture/Record Turntable | DIALOG | fix | P2 | 38 | clipped: "skim · T1 flat-10 · rapid · F 24000 mm/min · X …"; no-focus: nothing has the keyboard when it opens; no-exit: no Cancel or Close, and Esc leaves it open | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| assembly/Assemble/Bill of Materials | DIALOG | fix | P2 | 36 | raw-text: "load_tray" (a raw key); raw-text: "rack_face" (a raw key); no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Sheet Metal/Fold | DIALOG | fix | P2 | 36 | raw-text: "right_lip" (a raw key); raw-text: "left_lip" (a raw key); no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Sheet Metal/Jog | DIALOG | fix | P2 | 36 | raw-text: "right_lip" (a raw key); raw-text: "left_lip" (a raw key); no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Sheet Metal/Corner Seam | DIALOG | fix | P2 | 36 | raw-text: "right_lip" (a raw key); raw-text: "left_lip" (a raw key); no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Sheet Metal/Corner Relief | DIALOG | fix | P2 | 36 | raw-text: "right_lip" (a raw key); raw-text: "left_lip" (a raw key); no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Flat Pattern: Flat Pattern/Bend Table | DIALOG | fix | P2 | 36 | raw-text: "left_lip" (a raw key); raw-text: "right_lip" (a raw key); no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/menu File/New Drawing... | DIALOG | fix | P1 | 33 | label: "New Drawing...": three dots for "…"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 594x227 at (0,220) leaves the 480x860 window; clipped@narrow: "E  1117.6 x 863.6"; clipped@narrow: "A0  1189 x 841" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| sketch/menu File/New Drawing... | DIALOG | fix | P1 | 33 | label: "New Drawing...": three dots for "…"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 594x227 at (0,220) leaves the 480x860 window; clipped@narrow: "E  1117.6 x 863.6"; clipped@narrow: "A0  1189 x 841" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| assembly/menu File/New Drawing... | DIALOG | fix | P1 | 33 | label: "New Drawing...": three dots for "…"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 594x227 at (0,220) leaves the 480x860 window; clipped@narrow: "E  1117.6 x 863.6"; clipped@narrow: "A0  1189 x 841" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| plate/Inspect/Materials | DIALOG | fix | P1 | 32 | clipped: "stand-in"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 1162x662 at (0,140) leaves the 480x860 window; clipped@narrow: "stand-in" | fixed: ui/audit-narrow (batch 5, list/detail at 480) |
| sketch/Inspect/Materials | DIALOG | fix | P1 | 32 | clipped: "stand-in"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 1162x662 at (0,140) leaves the 480x860 window; clipped@narrow: "stand-in" | fixed: ui/audit-narrow (batch 5, list/detail at 480) |
| sheet/Sheet Metal/Vendor | DIALOG | fix | P1 | 30 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 566x353 at (0,222) leaves the 480x860 window; clipped@narrow: "Source: https://sendcutsend.com/guidelines/ ; h…"; clipped@narrow: "Does: laser, waterjet, bend, countersink, tap, …" | fixed: ui/audit-narrow (batch 5, fits, Esc) |
| cam/Manufacture/New Setup | DIALOG | fix | P1 | 28 | off-screen: 422x446 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Face | DIALOG | fix | P1 | 28 | off-screen: 422x382 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/2D Clear | DIALOG | fix | P1 | 28 | off-screen: 422x396 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/2D Contour | DIALOG | fix | P1 | 28 | off-screen: 422x382 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Pocket | DIALOG | fix | P1 | 28 | off-screen: 422x396 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Chamfer | DIALOG | fix | P1 | 28 | off-screen: 422x396 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Engrave | DIALOG | fix | P1 | 28 | off-screen: 422x382 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Adaptive | DIALOG | fix | P1 | 28 | off-screen: 422x396 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Drill | DIALOG | fix | P1 | 28 | off-screen: 422x288 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Bore | DIALOG | fix | P1 | 28 | off-screen: 422x382 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/3D Rough | DIALOG | fix | P1 | 28 | off-screen: 422x396 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Parallel | DIALOG | fix | P1 | 28 | off-screen: 422x382 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Waterline | DIALOG | fix | P1 | 28 | off-screen: 422x396 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Pencil | DIALOG | fix | P1 | 28 | off-screen: 422x396 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Scallop | DIALOG | fix | P1 | 28 | off-screen: 422x396 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Rest Finish | DIALOG | fix | P1 | 28 | off-screen: 422x382 at (990,174) leaves the 1400x860 window; no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| plate/Robot: Robot/Joint axes | MODE | fix | P1 | 23 | label: "Joint axes": not Title Case (axes); silent-mode: a mode with no prompt, outside the allowed list | open |
| start/Manage/Settings | DIALOG | fix | P1 | 22 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 902x682 at (0,82) leaves the 480x860 window; clipped@narrow: "A pastel per part" | fixed: ui/audit-narrow (batch 5, fits at 480) |
| plate/Inspect/Mass | DIALOG | fix | P1 | 22 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 540x598 at (0,186) leaves the 480x860 window; clipped@narrow: "Browse…" | fixed: ui/audit-narrow (batch 5, every window fits; a bottom sheet at 480) |
| sketch/Inspect/Mass | DIALOG | fix | P1 | 22 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 540x598 at (0,186) leaves the 480x860 window; clipped@narrow: "Browse…" | fixed: ui/audit-narrow (batch 5, every window fits; a bottom sheet at 480) |
| sheet/Bend Simulation: Bend Simulation/Play | MODE | fix | P1 | 20 | silent-mode: a mode with no prompt, outside the allowed list | with BENDSIM-ENV (after the train-7 freeze; 480 px crash sent) |
| weld/menu Browser: end_cap row/edit | DIALOG | fix | P2 | 18 | raw-text: "t_foot" (a raw key); no-focus: nothing has the keyboard when it opens | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| assembly/Inspect/Materials | DIALOG | fix | P2 | 16 | clipped: "stand-in"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, list/detail at 480) |
| plate/menu File/Save | ACTION | fix | P2 | 15 | raw-text: status "Saved /tmp/fg-ui-audit-3127/plate/run/plate.fg" (a temporary path) | fixed: ui/audit-shell (batch 1, … and the off reason) |
| plate/Manage/Save | ACTION | fix | P2 | 15 | raw-text: status "Saved /tmp/fg-ui-audit-3127/plate/run/plate.fg" (a temporary path) | fixed: ui/audit-words (batch 2, "Saved plate.fg") |
| assembly/menu File/Save | ACTION | fix | P2 | 15 | raw-text: status "Saved /tmp/fg-ui-audit-3127/assembly/run/tray_a…" (a temporary path) | fixed: ui/audit-shell (batch 1, … and the off reason) |
| assembly/Manage/Save | ACTION | fix | P2 | 15 | raw-text: status "Saved /tmp/fg-ui-audit-3127/assembly/run/tray_a…" (a temporary path) | fixed: ui/audit-words (batch 2, "Saved plate.fg") |
| cam/menu File/Save | ACTION | fix | P2 | 15 | raw-text: status "Saved /tmp/fg-ui-audit-3127/cam/run/bracket.fg" (a temporary path) | fixed: ui/audit-shell (batch 1, … and the off reason) |
| cam/Manufacture/Setup Sheet | ACTION | fix | P2 | 15 | raw-text: status "Wrote /tmp/fg-ui-audit-3127/cam/run/bracket_cam…" (a temporary path) | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| thermo/menu Browser: thermoform row/Edit… | ACTION | fix | P2 | 15 | raw-text: status "NoForgeForm: Edit Feature: thermoform has no di…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| thermo/menu Browser: thermoform row/edit | ACTION | fix | P2 | 15 | raw-text: status "NoForgeForm: Edit Feature: thermoform has no di…" (an error code) | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| plate/Thermoform: Thermoform/Capture Settings | DIALOG | fix | P1 | 14 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 374x321 at (271,234) leaves the 480x860 window | fixed: ui/audit-narrow (batch 5, every window fits; a bottom sheet at 480) |
| plate/Robot: Robot/Capture Settings | DIALOG | fix | P1 | 14 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 374x321 at (271,234) leaves the 480x860 window | fixed: ui/audit-narrow (batch 5, every window fits; a bottom sheet at 480) |
| plate/Render Studio: Render Studio/Capture Settings | DIALOG | fix | P1 | 14 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 374x321 at (271,234) leaves the 480x860 window | fixed: ui/audit-narrow (batch 5, every window fits; a bottom sheet at 480) |
| sheet/Flat Pattern: Flat Pattern/Capture Settings | DIALOG | fix | P1 | 14 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 374x321 at (271,234) leaves the 480x860 window | fixed: ui/audit-narrow (batch 5, every window fits; a bottom sheet at 480) |
| weld/Weldments/Cut List | DIALOG | fix | P1 | 14 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open; off-screen@narrow: 801x341 at (-321,455) leaves the 480x860 window | fixed: ui/audit-narrow (batch 5, fits, scrolls) |
| plate/Assemble/Joint | DIALOG | fix | P3 | 9 | contrast: "1" 2.2:1; no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, ON_FOCUS badge) |
| plate/Assemble/Constrain | DIALOG | fix | P3 | 9 | contrast: "1" 2.2:1; no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, ON_FOCUS badge) |
| sketch/Assemble/Joint | DIALOG | fix | P3 | 9 | contrast: "1" 2.2:1; no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, ON_FOCUS badge) |
| sketch/Assemble/Constrain | DIALOG | fix | P3 | 9 | contrast: "1" 2.2:1; no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, ON_FOCUS badge) |
| assembly/Assemble/Joint | DIALOG | fix | P3 | 9 | contrast: "1" 2.2:1; no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, ON_FOCUS badge) |
| assembly/Assemble/Constrain | DIALOG | fix | P3 | 9 | contrast: "1" 2.2:1; no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, ON_FOCUS badge) |
| assembly/Harness/Create Harness | DIALOG | fix | P2 | 9 | no-unit: "0.03" in the row of "Slack"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Sheet Metal/Sheet Metal Defaults | DIALOG | fix | P2 | 9 | no-unit: "0.42" in the row of "K-factor"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Sheet Metal/Rules | DIALOG | fix | P2 | 9 | no-unit: "0.44" in the row of "K-factor"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| cam/menu File/New Drawing... | DIALOG | fix | P3 | 9 | label: "New Drawing...": three dots for "…"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-shell (batch 1, … and the off reason) |
| cam/Manufacture/Document Text | DIALOG | fix | P2 | 9 | error-text: machining # bracket.fg in two setups. op1: in t…; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| drawing/Annotate/Feature Control Frame | DIALOG | fix | P2 | 9 | no-unit: "0.1" in the row of "Ø"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| drawing/Annotate/Surface | DIALOG | fix | P2 | 9 | no-unit: "1.6" in the row of "Ra (µm)"; no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/3D Model/Convert to Thermoform | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/Assemble/Bill of Materials | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/Assemble/Parameters | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/Assemble/Explode | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/Weldments/Cut List | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, fits, scrolls) |
| plate/Inspect/Section | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/Inspect/iProperties | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/View/Capture Settings | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, every window fits; a bottom sheet at 480) |
| plate/Manage/Parameters | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/Manage/Create Version | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/Manage/Release | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/Manage/Settings | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, fits at 480) |
| plate/Environments/Drawing | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| plate/Robot: Robot/Centres of mass | ACTION | good | P3 | 6 | label: "Centres of mass": not Title Case (mass); unsure: only the view changed, and the view animates by itself: no other sign the click was taken | open |
| part/3D Model/Shell | DIALOG | good | P2 | 6 | error-text: Build: shell1: the kernel refused it: shell; no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Convert to Thermoform | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| part/Environments/Drawing | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sketch/3D Model/Convert to Thermoform | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sketch/Assemble/Bill of Materials | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sketch/Assemble/Parameters | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sketch/Assemble/Explode | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sketch/Weldments/Cut List | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, fits, scrolls) |
| sketch/Inspect/Section | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sketch/Inspect/iProperties | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sketch/View/Capture Settings | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, every window fits; a bottom sheet at 480) |
| sketch/Manage/Parameters | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sketch/Manage/Create Version | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sketch/Manage/Release | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sketch/Manage/Settings | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, fits at 480) |
| sketch/Environments/Drawing | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| assembly/3D Model/Shell | DIALOG | good | P2 | 6 | error-text: Build: shell1: the kernel refused it: nothing t…; no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Convert to Thermoform | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| assembly/Assemble/Parameters | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| assembly/Assemble/Explode | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| assembly/Weldments/Cut List | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, fits, scrolls) |
| assembly/Inspect/Analyze Interference | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| assembly/Inspect/Mass | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, every window fits; a bottom sheet at 480) |
| assembly/Inspect/Section | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| assembly/Inspect/iProperties | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| assembly/View/Explode | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| assembly/View/Capture Settings | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, every window fits; a bottom sheet at 480) |
| assembly/Manage/Parameters | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| assembly/Manage/Create Version | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| assembly/Manage/Release | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| assembly/Manage/Settings | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-narrow (batch 5, fits at 480) |
| assembly/Environments/Drawing | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/menu Browser: sheet row/edit | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| sheet/Sheet Metal/Face | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Sheet Metal/Contour Flange | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Sheet Metal/Cut | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Sheet Metal/Corner Round | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Sheet Metal/Unfold | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Sheet Metal/Check (DFM) | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Environments/Drawing | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Flat Pattern: Flat Pattern/Check (DFM) | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Flat Pattern: Flat Pattern/Save Copy As DXF/DWG | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| sheet/Bend Simulation: Bend Simulation/Capture Settings | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | with BENDSIM-ENV (after the train-7 freeze; 480 px crash sent) |
| cam/Manufacture/Capture Settings | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| thermo/Environments/Drawing | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| drawing/Place Views/Base | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| drawing/Annotate/Datum Identifier | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| drawing/Annotate/Welding | DIALOG | good | P3 | 6 | no-focus: nothing has the keyboard when it opens; esc: Esc leaves it open | open |
| start/menu File/Export 3MF... | DISABLED | good | P3 | 3 | label: "Export 3MF...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| start/menu File/Export OBJ... | DISABLED | good | P3 | 3 | label: "Export OBJ...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| start/menu File/Export STL... | DISABLED | good | P3 | 3 | label: "Export STL...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| start/menu File/New Drawing... | DISABLED | good | P3 | 3 | label: "New Drawing...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| start/menu File/Open... | HOST | good | P3 | 3 | label: "Open...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| plate/menu File/Export 3MF... | HOST | good | P3 | 3 | label: "Export 3MF...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| plate/menu File/Export OBJ... | HOST | good | P3 | 3 | label: "Export OBJ...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| plate/menu File/Export STL... | HOST | good | P3 | 3 | label: "Export STL...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| plate/menu File/Open... | HOST | good | P3 | 3 | label: "Open...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| plate/menu Browser: extrude row/edit | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| plate/3D Model/Loft | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Sweep | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Coil | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Rib | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Emboss | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Import | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Fillet | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Chamfer | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Shell | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Split | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| plate/3D Model/Combine | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| plate/3D Model/Thicken | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Direct | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| plate/3D Model/Delete Face | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| plate/3D Model/Replace | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Plane | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Axis | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Point | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/UCS | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Rectangular | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Circular | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Sketch | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Mirror | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Move | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/3D Model/Copy | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/Assemble/Place | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/Assemble/Free Move | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/Assemble/Free Rotate | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/Weldments/Members | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/Weldments/End Trim | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/Weldments/End Cap | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/Weldments/Corner Plate | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/Inspect/Measure | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/Inspect/Wall | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| plate/Robot: Robot/Make root | DISABLED | good | P3 | 3 | label: "Make root": not Title Case (root) | open |
| plate/Robot: Robot/Merge into parent | DISABLED | good | P3 | 3 | label: "Merge into parent": not Title Case (parent) | open |
| plate/Robot: Robot/Freeze here | DISABLED | good | P3 | 3 | label: "Freeze here": not Title Case (here) | open |
| plate/Robot: Robot/Add frame | DISABLED | good | P3 | 3 | label: "Add frame": not Title Case (frame) | open |
| plate/Robot: Robot/Collision: hull | DISABLED | good | P3 | 3 | label: "Collision: hull": not Title Case (hull) | open |
| plate/Render Studio: Render Studio/Render Image | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | with RENDER-STUDIO (render.png relative to the working directory) |
| plate/Render Studio: Render Studio/Render Turntable | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | with RENDER-STUDIO (render.png relative to the working directory) |
| plate/Render Studio: Render Studio/Floor | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | open |
| plate/Render Studio: Render Studio/Denoise | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | open |
| plate/Render Studio: Render Studio/Start Again | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | open |
| plate/Render Studio: Render Studio/Depth of Field | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | open |
| plate/Render Studio: Render Studio/Perspective | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | open |
| plate/Render Studio: Render Studio/Fit All | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | open |
| part/menu Browser: sketch row/rename | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: extrude row/edit | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: extrude row/rename | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: hole row/edit | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: hole row/rename | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: fillet row/edit | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/menu Browser: fillet row/rename | ACTION | good | P3 | 3 | unsure: only the view changed, and the view animates by itself: no other sign the click was taken | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| part/3D Model/Loft | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Sweep | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Coil | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Rib | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Emboss | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Import | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Fillet | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Chamfer | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Thicken | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Replace | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Plane | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Axis | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Point | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/UCS | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Rectangular | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Circular | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Sketch | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Mirror | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Move | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| part/3D Model/Copy | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/menu File/Export 3MF... | HOST | good | P3 | 3 | label: "Export 3MF...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| sketch/menu File/Export OBJ... | HOST | good | P3 | 3 | label: "Export OBJ...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| sketch/menu File/Export STL... | HOST | good | P3 | 3 | label: "Export STL...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| sketch/menu File/Open... | HOST | good | P3 | 3 | label: "Open...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| sketch/menu Browser: extrude row/edit | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-shell (batch 1, off with "Finish or cancel the sketch first") |
| sketch/3D Model/Loft | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Sweep | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Coil | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Rib | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Emboss | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Import | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Fillet | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Chamfer | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Shell | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Draft | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-words3 (batch 3, 480 px crash; = ui/bughunt-3 e90d49ec) |
| sketch/3D Model/Split | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| sketch/3D Model/Combine | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| sketch/3D Model/Thicken | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Direct | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| sketch/3D Model/Delete Face | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| sketch/3D Model/Replace | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Plane | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Axis | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Point | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/UCS | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Rectangular | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Circular | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Sketch | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Mirror | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Move | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/3D Model/Copy | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/Assemble/Place | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/Assemble/Free Move | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/Assemble/Free Rotate | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/Weldments/Members | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/Weldments/End Trim | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/Weldments/End Cap | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/Weldments/Corner Plate | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/Inspect/Measure | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sketch/Inspect/Wall | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/menu File/Export 3MF... | HOST | good | P3 | 3 | label: "Export 3MF...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| assembly/menu File/Export OBJ... | HOST | good | P3 | 3 | label: "Export OBJ...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| assembly/menu File/Export STL... | HOST | good | P3 | 3 | label: "Export STL...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| assembly/menu File/Open... | HOST | good | P3 | 3 | label: "Open...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| assembly/3D Model/Extrude | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Revolve | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Loft | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Sweep | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Coil | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Rib | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Emboss | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Import | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Fillet | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Chamfer | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Draft | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-words3 (batch 3, 480 px crash; = ui/bughunt-3 e90d49ec) |
| assembly/3D Model/Thread | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| assembly/3D Model/Split | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| assembly/3D Model/Thicken | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Direct | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-narrow (batch 5, menus' sizing frame; 480 reachable) |
| assembly/3D Model/Delete Face | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-words3 (batch 3, 480 px crash) |
| assembly/3D Model/Replace | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Plane | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Axis | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Point | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/UCS | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Rectangular | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Circular | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Sketch | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Mirror | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Move | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/3D Model/Copy | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/Assemble/Place | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/Assemble/Free Move | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/Assemble/Free Rotate | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/Weldments/Members | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/Weldments/End Trim | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/Weldments/End Cap | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/Weldments/Corner Plate | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/Inspect/Measure | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| assembly/Inspect/Wall | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| sheet/menu Browser: sheet row/rollback | PANEL | good | P2 | 3 | error-text: Build failed | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| sheet/menu Browser: flange row/rollback | PANEL | good | P2 | 3 | error-text: Build failed | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| sheet/menu Browser: flange row/suppress | PANEL | good | P2 | 3 | error-text: Build failed | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| cam/menu File/Export 3MF... | HOST | good | P3 | 3 | label: "Export 3MF...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| cam/menu File/Export OBJ... | HOST | good | P3 | 3 | label: "Export OBJ...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| cam/menu File/Export STL... | HOST | good | P3 | 3 | label: "Export STL...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| cam/menu File/Open... | HOST | good | P3 | 3 | label: "Open...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| cam/Manufacture/Post Process | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/NC Programs | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Quality | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| cam/Manufacture/Check Now | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | with CAM-UI (480 px crash gui_camsim_panel.rs:468; provenance box over the view; Record Turntable) |
| thermo/menu Browser: extrude row/edit | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| thermo/menu Browser: fillet row/down | PANEL | good | P2 | 3 | error-text: Build failed | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| thermo/menu Browser: fillet row/edit | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| thermo/menu Browser: thermoform row/Make Mould | PANEL | good | P2 | 3 | error-text: Build failed | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| thermo/menu Browser: thermoform row/up | PANEL | good | P2 | 3 | error-text: Build failed | fixed: ui/audit-shell..words3 (batches 1-3: no error codes; off with "Used by …"; Select) |
| weld/Weldments/Members | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| weld/Weldments/End Trim | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| weld/Weldments/End Cap | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| weld/Weldments/Corner Plate | DIALOG | good | P3 | 3 | no-focus: nothing has the keyboard when it opens | open |
| drawing/menu File/Export 3MF... | DISABLED | good | P3 | 3 | label: "Export 3MF...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| drawing/menu File/Export OBJ... | DISABLED | good | P3 | 3 | label: "Export OBJ...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| drawing/menu File/Export STL... | DISABLED | good | P3 | 3 | label: "Export STL...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| drawing/menu File/New Drawing... | DISABLED | good | P3 | 3 | label: "New Drawing...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| drawing/menu File/Open... | HOST | good | P3 | 3 | label: "Open...": three dots for "…" | fixed: ui/audit-shell (batch 1, … and the off reason) |
| start/menu File/New | WORKSPACE | good |  | 0 |  | open |
| start/menu File/New Sheet Metal | HOST | good |  | 0 |  | open |
| start/Manage/Open | HOST | good |  | 0 |  | open |
| start/Machine Builder: Machine Builder/New from Template | ACTION | good |  | 0 |  | open |
| start/Machine Builder: Machine Builder/Derive Copy | ACTION | good |  | 0 |  | open |
| start/Machine Builder: Machine Builder/Check | PANEL | good |  | 0 |  | open |
| start/Machine Builder: Machine Builder/Finish Machine Builder | WORKSPACE | good |  | 0 |  | open |
| plate/menu File/New | WORKSPACE | good |  | 0 |  | open |
| plate/menu File/New Sheet Metal | HOST | good |  | 0 |  | open |
| plate/menu Browser: sketch row/edit-sketch | DIALOG | good |  | 0 |  | open |
| plate/menu Browser: sketch row/rename | ACTION | good |  | 0 |  | open |
| plate/menu Browser: sketch row/rollback | ACTION | good |  | 0 |  | open |
| plate/menu Browser: sketch row/suppress | PANEL | good |  | 0 |  | open |
| plate/menu Browser: extrude row/delete | ACTION | good |  | 0 |  | open |
| plate/menu Browser: extrude row/rename | ACTION | good |  | 0 |  | open |
| plate/menu Browser: extrude row/rollback | ACTION | good |  | 0 |  | open |
| plate/menu Browser: extrude row/suppress | PANEL | good |  | 0 |  | open |
| plate/3D Model/Start Sketch | DIALOG | good |  | 0 |  | open |
| plate/3D Model/Extrude | DIALOG | good |  | 0 |  | open |
| plate/3D Model/Revolve | DIALOG | good |  | 0 |  | open |
| plate/3D Model/Convert to Sheet Metal | PANEL | good |  | 0 |  | open |
| plate/Assemble/Pattern | ACTION | good |  | 0 |  | open |
| plate/Assemble/Mirror | ACTION | good |  | 0 |  | open |
| plate/Assemble/Copy | ACTION | good |  | 0 |  | open |
| plate/Weldments/Start Sketch | DIALOG | good |  | 0 |  | open |
| plate/Inspect/CG Marker | ACTION | good |  | 0 |  | open |
| plate/View/Visual Style | PANEL | good |  | 0 |  | open |
| plate/View/Ground Plane | ACTION | good |  | 0 |  | open |
| plate/View/Reflections | ACTION | good |  | 0 |  | open |
| plate/View/Ground Shadows | MODE | good |  | 0 |  | open |
| plate/View/Object Shadows | MODE | good |  | 0 |  | open |
| plate/View/Ambient Shadows | MODE | good |  | 0 |  | open |
| plate/View/Orbit | MODE | good |  | 0 |  | open |
| plate/View/Pan | MODE | good |  | 0 |  | open |
| plate/View/Zoom | MODE | good |  | 0 |  | open |
| plate/View/Fit All | ACTION | good |  | 0 |  | open |
| plate/View/Photo | DIALOG | good |  | 0 |  | open |
| plate/View/Record Turntable | DIALOG | good |  | 0 |  | open |
| plate/Manage/Open | HOST | good |  | 0 |  | open |
| plate/Manage/Versions | ACTION | good |  | 0 |  | open |
| plate/Manage/Compare | ACTION | good |  | 0 |  | open |
| plate/Manage/Machine Builder | WORKSPACE | good |  | 0 |  | open |
| plate/Environments/Machine Builder | WORKSPACE | good |  | 0 |  | open |
| plate/Environments/Thermoform | WORKSPACE | good |  | 0 |  | open |
| plate/Environments/Robot | WORKSPACE | good |  | 0 |  | open |
| plate/Environments/Render Studio | WORKSPACE | good |  | 0 |  | open |
| plate/Thermoform: Thermoform/Photo | DIALOG | good |  | 0 |  | open |
| plate/Thermoform: Thermoform/Record Turntable | DIALOG | good |  | 0 |  | open |
| plate/Thermoform: Thermoform/Finish Thermoform | WORKSPACE | good |  | 0 |  | open |
| plate/Robot: Robot/Photo | DIALOG | good |  | 0 |  | open |
| plate/Robot: Robot/Record Turntable | DIALOG | good |  | 0 |  | open |
| plate/Robot: Robot/Finish Robot | WORKSPACE | good |  | 0 |  | open |
| plate/Render Studio: Render Studio/Photo | DIALOG | good |  | 0 |  | open |
| plate/Render Studio: Render Studio/Record Turntable | DIALOG | good |  | 0 |  | open |
| plate/Render Studio: Render Studio/Finish Render Studio | WORKSPACE | good |  | 0 |  | open |
| part/menu Browser: sketch row/edit-sketch | DIALOG | good |  | 0 |  | open |
| part/menu Browser: sketch row/rollback | ACTION | good |  | 0 |  | open |
| part/menu Browser: sketch row/suppress | PANEL | good |  | 0 |  | open |
| part/menu Browser: extrude row/rollback | ACTION | good |  | 0 |  | open |
| part/menu Browser: extrude row/suppress | PANEL | good |  | 0 |  | open |
| part/menu Browser: hole row/down | ACTION | good |  | 0 |  | open |
| part/menu Browser: hole row/rollback | ACTION | good |  | 0 |  | open |
| part/menu Browser: hole row/suppress | PANEL | good |  | 0 |  | open |
| part/menu Browser: hole row/up | ACTION | good |  | 0 |  | open |
| part/menu Browser: fillet row/delete | ACTION | good |  | 0 |  | open |
| part/menu Browser: fillet row/down | ACTION | good |  | 0 |  | open |
| part/menu Browser: fillet row/rollback | ACTION | good |  | 0 |  | open |
| part/menu Browser: fillet row/suppress | PANEL | good |  | 0 |  | open |
| part/menu Browser: fillet row/up | ACTION | good |  | 0 |  | open |
| part/3D Model/Start Sketch | DIALOG | good |  | 0 |  | open |
| part/3D Model/Extrude | DIALOG | good |  | 0 |  | open |
| part/3D Model/Revolve | DIALOG | good |  | 0 |  | open |
| part/3D Model/Convert to Sheet Metal | PANEL | good |  | 0 |  | open |
| part/Environments/Machine Builder | WORKSPACE | good |  | 0 |  | open |
| part/Environments/Thermoform | WORKSPACE | good |  | 0 |  | open |
| part/Environments/Robot | WORKSPACE | good |  | 0 |  | open |
| part/Environments/Render Studio | WORKSPACE | good |  | 0 |  | open |
| sketch/menu File/New | WORKSPACE | good |  | 0 |  | open |
| sketch/menu File/New Sheet Metal | HOST | good |  | 0 |  | open |
| sketch/menu Browser: sketch row/edit-sketch | ACTION | good |  | 0 |  | open |
| sketch/menu Browser: sketch row/rename | ACTION | good |  | 0 |  | open |
| sketch/menu Browser: sketch row/rollback | ACTION | good |  | 0 |  | open |
| sketch/menu Browser: sketch row/suppress | PANEL | good |  | 0 |  | open |
| sketch/menu Browser: extrude row/delete | ACTION | good |  | 0 |  | open |
| sketch/menu Browser: extrude row/rename | ACTION | good |  | 0 |  | open |
| sketch/menu Browser: extrude row/rollback | ACTION | good |  | 0 |  | open |
| sketch/menu Browser: extrude row/suppress | PANEL | good |  | 0 |  | open |
| sketch/3D Model/Start Sketch | DIALOG | good |  | 0 |  | open |
| sketch/3D Model/Extrude | DIALOG | good |  | 0 |  | open |
| sketch/3D Model/Revolve | DIALOG | good |  | 0 |  | open |
| sketch/3D Model/Convert to Sheet Metal | PANEL | good |  | 0 |  | open |
| sketch/Weldments/Start Sketch | DIALOG | good |  | 0 |  | open |
| sketch/Inspect/CG Marker | ACTION | good |  | 0 |  | open |
| sketch/View/Visual Style | PANEL | good |  | 0 |  | open |
| sketch/View/Ground Plane | MODE | good |  | 0 |  | open |
| sketch/View/Reflections | MODE | good |  | 0 |  | open |
| sketch/View/Ground Shadows | MODE | good |  | 0 |  | open |
| sketch/View/Object Shadows | ACTION | good |  | 0 |  | open |
| sketch/View/Ambient Shadows | ACTION | good |  | 0 |  | open |
| sketch/View/Orbit | ACTION | good |  | 0 |  | open |
| sketch/View/Pan | ACTION | good |  | 0 |  | open |
| sketch/View/Zoom | ACTION | good |  | 0 |  | open |
| sketch/View/Fit All | ACTION | good |  | 0 |  | open |
| sketch/View/Photo | DIALOG | good |  | 0 |  | open |
| sketch/View/Record Turntable | DIALOG | good |  | 0 |  | open |
| sketch/Manage/Open | HOST | good |  | 0 |  | open |
| sketch/Manage/Versions | ACTION | good |  | 0 |  | open |
| sketch/Manage/Compare | ACTION | good |  | 0 |  | open |
| sketch/Manage/Machine Builder | WORKSPACE | good |  | 0 |  | open |
| sketch/Environments/Machine Builder | WORKSPACE | good |  | 0 |  | open |
| sketch/Environments/Thermoform | WORKSPACE | good |  | 0 |  | open |
| sketch/Environments/Robot | WORKSPACE | good |  | 0 |  | open |
| sketch/Environments/Render Studio | WORKSPACE | good |  | 0 |  | open |
| assembly/menu File/New | WORKSPACE | good |  | 0 |  | open |
| assembly/menu File/New Sheet Metal | HOST | good |  | 0 |  | open |
| assembly/3D Model/Start Sketch | DIALOG | good |  | 0 |  | open |
| assembly/3D Model/Convert to Sheet Metal | ACTION | good |  | 0 |  | open |
| assembly/Assemble/Pattern | ACTION | good |  | 0 |  | open |
| assembly/Assemble/Mirror | ACTION | good |  | 0 |  | open |
| assembly/Assemble/Copy | ACTION | good |  | 0 |  | open |
| assembly/Weldments/Start Sketch | DIALOG | good |  | 0 |  | open |
| assembly/Inspect/CG Marker | ACTION | good |  | 0 |  | open |
| assembly/View/Visual Style | PANEL | good |  | 0 |  | open |
| assembly/View/Section | ACTION | good |  | 0 |  | open |
| assembly/View/Relationships | MODE | good |  | 0 |  | open |
| assembly/View/Ground Plane | ACTION | good |  | 0 |  | open |
| assembly/View/Reflections | ACTION | good |  | 0 |  | open |
| assembly/View/Ground Shadows | MODE | good |  | 0 |  | open |
| assembly/View/Object Shadows | ACTION | good |  | 0 |  | open |
| assembly/View/Ambient Shadows | ACTION | good |  | 0 |  | open |
| assembly/View/Orbit | MODE | good |  | 0 |  | open |
| assembly/View/Pan | MODE | good |  | 0 |  | open |
| assembly/View/Zoom | MODE | good |  | 0 |  | open |
| assembly/View/Fit All | PANEL | good |  | 0 |  | open |
| assembly/View/Photo | DIALOG | good |  | 0 |  | open |
| assembly/View/Record Turntable | DIALOG | good |  | 0 |  | open |
| assembly/Manage/Open | HOST | good |  | 0 |  | open |
| assembly/Manage/Versions | ACTION | good |  | 0 |  | open |
| assembly/Manage/Compare | ACTION | good |  | 0 |  | open |
| assembly/Manage/Machine Builder | WORKSPACE | good |  | 0 |  | open |
| assembly/Environments/Machine Builder | WORKSPACE | good |  | 0 |  | open |
| assembly/Environments/Robot | WORKSPACE | good |  | 0 |  | open |
| assembly/Environments/Render Studio | WORKSPACE | good |  | 0 |  | open |
| sheet/menu Browser: sketch row/edit-sketch | DIALOG | good |  | 0 |  | open |
| sheet/menu Browser: sketch row/rename | ACTION | good |  | 0 |  | open |
| sheet/menu Browser: sketch row/rollback | ACTION | good |  | 0 |  | open |
| sheet/menu Browser: sketch row/suppress | PANEL | good |  | 0 |  | open |
| sheet/menu Browser: sheet row/rename | ACTION | good |  | 0 |  | open |
| sheet/menu Browser: sheet row/suppress | PANEL | good |  | 0 |  | open |
| sheet/menu Browser: flange row/down | ACTION | good |  | 0 |  | open |
| sheet/menu Browser: flange row/rename | ACTION | good |  | 0 |  | open |
| sheet/Sheet Metal/Start 2D Sketch | DIALOG | good |  | 0 |  | open |
| sheet/Sheet Metal/Go to Flat Pattern | WORKSPACE | good |  | 0 |  | open |
| sheet/Sheet Metal/Convert to Part | ACTION | good |  | 0 |  | open |
| sheet/Environments/Machine Builder | WORKSPACE | good |  | 0 |  | open |
| sheet/Environments/Thermoform | WORKSPACE | good |  | 0 |  | open |
| sheet/Environments/Flat Pattern | WORKSPACE | good |  | 0 |  | open |
| sheet/Environments/Bend Simulation | WORKSPACE | good |  | 0 |  | open |
| sheet/Environments/Robot | WORKSPACE | good |  | 0 |  | open |
| sheet/Environments/Render Studio | WORKSPACE | good |  | 0 |  | open |
| sheet/Flat Pattern: Flat Pattern/Extents | ACTION | good |  | 0 |  | open |
| sheet/Flat Pattern: Flat Pattern/Bend Lines | ACTION | good |  | 0 |  | open |
| sheet/Flat Pattern: Flat Pattern/Tangents | ACTION | good |  | 0 |  | open |
| sheet/Flat Pattern: Flat Pattern/Punch Centres | MODE | good |  | 0 |  | open |
| sheet/Flat Pattern: Flat Pattern/Go to Folded Part | WORKSPACE | good |  | 0 |  | open |
| sheet/Flat Pattern: Flat Pattern/Photo | DIALOG | good |  | 0 |  | open |
| sheet/Flat Pattern: Flat Pattern/Record Turntable | DIALOG | good |  | 0 |  | open |
| sheet/Bend Simulation: Bend Simulation/Choose Press | PANEL | good |  | 0 |  | open |
| sheet/Bend Simulation: Bend Simulation/Tooling | PANEL | good |  | 0 |  | open |
| sheet/Bend Simulation: Bend Simulation/Write Program | ACTION | good |  | 0 |  | open |
| sheet/Bend Simulation: Bend Simulation/Next Bend | ACTION | good |  | 0 |  | open |
| sheet/Bend Simulation: Bend Simulation/Last Bend | ACTION | good |  | 0 |  | open |
| sheet/Bend Simulation: Bend Simulation/Press Overview | PANEL | good |  | 0 |  | open |
| sheet/Bend Simulation: Bend Simulation/Follow Bend | PANEL | good |  | 0 |  | open |
| sheet/Bend Simulation: Bend Simulation/See Through | PANEL | good |  | 0 |  | open |
| sheet/Bend Simulation: Bend Simulation/Photo | DIALOG | good |  | 0 |  | open |
| sheet/Bend Simulation: Bend Simulation/Record | DIALOG | good |  | 0 |  | open |
| sheet/Bend Simulation: Bend Simulation/Finish Bend Simulation | WORKSPACE | good |  | 0 |  | open |
| cam/menu File/New | WORKSPACE | good |  | 0 |  | open |
| cam/menu File/New Sheet Metal | HOST | good |  | 0 |  | open |
| cam/Manufacture/Generate | ACTION | good |  | 0 |  | open |
| cam/Manufacture/Checks | ACTION | good |  | 0 |  | open |
| cam/Manufacture/Photo | DIALOG | good |  | 0 |  | open |
| thermo/menu Browser: sketch row/edit-sketch | DIALOG | good |  | 0 |  | open |
| thermo/menu Browser: sketch row/rename | ACTION | good |  | 0 |  | open |
| thermo/menu Browser: sketch row/rollback | ACTION | good |  | 0 |  | open |
| thermo/menu Browser: sketch row/suppress | PANEL | good |  | 0 |  | open |
| thermo/menu Browser: extrude row/rename | ACTION | good |  | 0 |  | open |
| thermo/menu Browser: extrude row/rollback | ACTION | good |  | 0 |  | open |
| thermo/menu Browser: extrude row/suppress | PANEL | good |  | 0 |  | open |
| thermo/menu Browser: fillet row/delete | ACTION | good |  | 0 |  | open |
| thermo/menu Browser: fillet row/rename | ACTION | good |  | 0 |  | open |
| thermo/menu Browser: fillet row/rollback | ACTION | good |  | 0 |  | open |
| thermo/menu Browser: fillet row/suppress | PANEL | good |  | 0 |  | open |
| thermo/menu Browser: thermoform row/Check Forming | ACTION | good |  | 0 |  | open |
| thermo/menu Browser: thermoform row/delete | ACTION | good |  | 0 |  | open |
| thermo/menu Browser: thermoform row/rename | ACTION | good |  | 0 |  | open |
| thermo/menu Browser: thermoform row/rollback | ACTION | good |  | 0 |  | open |
| thermo/menu Browser: thermoform row/suppress | PANEL | good |  | 0 |  | open |
| thermo/Environments/Machine Builder | WORKSPACE | good |  | 0 |  | open |
| thermo/Environments/Thermoform | WORKSPACE | good |  | 0 |  | open |
| thermo/Environments/Robot | WORKSPACE | good |  | 0 |  | open |
| thermo/Environments/Render Studio | WORKSPACE | good |  | 0 |  | open |
| weld/menu Browser: sketch row/edit-sketch | DIALOG | good |  | 0 |  | open |
| weld/menu Browser: sketch row/rename | ACTION | good |  | 0 |  | open |
| weld/menu Browser: sketch row/rollback | ACTION | good |  | 0 |  | open |
| weld/menu Browser: sketch row/suppress | PANEL | good |  | 0 |  | open |
| weld/menu Browser: row row/rename | ACTION | good |  | 0 |  | open |
| weld/menu Browser: row row/rollback | ACTION | good |  | 0 |  | open |
| weld/menu Browser: row row/suppress | PANEL | good |  | 0 |  | open |
| weld/menu Browser: end_cap row/delete | ACTION | good |  | 0 |  | open |
| weld/menu Browser: end_cap row/down | ACTION | good |  | 0 |  | open |
| weld/menu Browser: end_cap row/rename | ACTION | good |  | 0 |  | open |
| weld/menu Browser: end_cap row/rollback | ACTION | good |  | 0 |  | open |
| weld/menu Browser: end_cap row/suppress | PANEL | good |  | 0 |  | open |
| weld/Weldments/Start Sketch | DIALOG | good |  | 0 |  | open |
| drawing/menu File/Close Drawing | WORKSPACE | good |  | 0 |  | open |
| drawing/menu File/New | WORKSPACE | good |  | 0 |  | open |
| drawing/menu File/New Sheet Metal | HOST | good |  | 0 |  | open |
| drawing/Place Views/Projected | DIALOG | good |  | 0 |  | open |
| drawing/Place Views/Auxiliary | DIALOG | good |  | 0 |  | open |
| drawing/Place Views/Section | DIALOG | good |  | 0 |  | open |
| drawing/Place Views/Detail | DIALOG | good |  | 0 |  | open |
| drawing/Place Views/Break | DIALOG | good |  | 0 |  | open |
| drawing/Place Views/Crop | DIALOG | good |  | 0 |  | open |
| drawing/Place Views/New Sheet | PANEL | good |  | 0 |  | open |
| drawing/Annotate/Dimension | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Baseline | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Ordinate | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Chain | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Hole and Thread | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Datum Target | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Edge | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Text | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Leader Text | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Automated Centerlines | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Center Mark | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Centerline | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Bisector | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Parts List | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Balloon | DIALOG | good |  | 0 |  | open |
| drawing/Annotate/Auto Balloon | DIALOG | good |  | 0 |  | open |
| drawing/Sketch/Text | DIALOG | good |  | 0 |  | open |
| drawing/Sketch/Leader Text | DIALOG | good |  | 0 |  | open |
| drawing/Manage/Export PDF | ACTION | good |  | 0 |  | open |
| drawing/Manage/Export DXF | ACTION | good |  | 0 |  | open |
| drawing/Manage/Export DWG | ACTION | good |  | 0 |  | open |
