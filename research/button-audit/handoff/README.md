# Dead buttons handed to their owners

From the UI audit's first full native crawl (tests/ui_audit, lavapipe, 1400x860). Each shot is
the window right after the click: nothing on it changed from before the click (no popup, no
status line, no change in the view).

| Item | Command | Document and state | Owner | What UX-LEAD asks |
|---|---|---|---|---|
| D | Machine Builder › Home Axes | no machine loaded (start screen, Manage › Machine Builder) | MACHINE-BUILDER | P1 dead: home with feedback, or off with the reason |
| E | Bend Simulation › Suggest Order, Simulate Again | examples/sheetmetal/load_tray.fg: no feasible order | BENDSIM-ENV | P1: say "No feasible order: <the first reason>" |
| E | Bend Simulation › First Bend, Previous Bend | same, at bend 1 | BENDSIM-ENV | off at bend 1 (nowhere to go), with the reason on hover |
| F | Manufacture › Export NC | examples/machined/bracket_cam.fg, 5/5 ops certified | CAM-UI | P1 dead: if it refuses, cam/ui-said's refusal should show |

Re-run one: `UI_AUDIT=full UI_AUDIT_ONLY=sheet UI_AUDIT_OUT=DIR cargo test --test ui_audit full_crawl -- --nocapture`
(records in DIR/records.tsv, shots in DIR/shots).
