# Batch 7: scroll bars you can see, and reasons without "Off:"

Branch `ui/audit-scrollbars`, stacked on `ui/audit-b6`. This was CAM-UI's finding, routed through the coordinator. UX-LEAD chose option (A).

| # | What | Before | After |
|---|------|--------|-------|
| 1 | Solid scroll bars (egui `ScrollStyle::solid()`) drew their handle in `widgets.inactive.bg_fill` over a track in `extreme_bg_color`. Both are `t.field`, so the handle was the track's own colour, 1.0:1, in both themes. The theme now sets `scroll.foreground_color`, so the handle is drawn in the ink: 15.3:1 dark and 16.6:1 light. A floating bar also faded to nothing at rest. Its handle now rests at 0.6 of the ink (5.8:1 dark, 4.4:1 light), so a list that overflows always shows that it scrolls. The three `solid()` sites (iProperties, Settings, the Material Browser) use `theme::solid_scroll()`. | before-1-materials-scrollbar | after-1-materials-scrollbar |
| 2 | A ribbon tool that is off gives its reason alone, as a sentence ("Open or create a part first …"), with no "Off:" before it. Versions' hovers read the same way ("No part open, so there are no saves to list"). The drawing editor's "On/Off: click to turn …" is a state, not a reason, so it stays. | | after-2-reason-without-off |

## Checks that keep this fixed

- `tests/scrollbars.rs` draws a solid bar and a floating always-visible bar with the app's theme applied, in both themes. It reads the colours egui paints for track and handle, and needs at least 3:1. Without the fix the solid bar measures 1.0:1 and the test fails.
- Crawler: the new judge rule `!scrollbar` fails any scroll bar a click brings up whose handle is under 3:1 against its track, or against the panel when the bar floats. On the plate at 1400 without the fix it flags Materials, Settings and Machine Builder (1.0:1). With the fix it flags nothing.

## 3. The status bar after the train-7 merge (UX-LEAD's check of 0fcbe327)

Main now cuts the line between its parts to fit and puts Manufacture's fix chip beside it. The audit adds one thing on top: a file's path is said by its name, with the whole line on hover.

- **(a) 480 × 860**, a release refusal ("Program prog can't be released: setup op1 isn't placed on the machine."). The chip "Place the setup…" stays whole, with the warning before it. There is too little room for any of the words beside it, so they move to the chip's hover (main's rule: a lone "…" says nothing). Shots: after-3a-status-480-fix-chip, after-3a-…-hover.
- **(b) 760 × 860**, the same refusal. The words are cut short of the chip and end in "…", with the whole line and the rule's detail on hover. Shot: after-3b-status-760-cut-hover.
- **(c) 1400 × 860**, Setup Sheet in a deep folder. "Wrote /tmp/…/customer-acme-industrial/brackets-and-flanges/revision-c-for-the-shop/bracket_cam-sheet.html" reads "Wrote bracket_cam-sheet.html", with the whole path on hover. Shot: after-3c-status-1400-path-hover.
- **Fixed while making these shots:** paths were shortened only in the shell's own line, not in an environment's line such as Manufacture's "Wrote …". Both are shortened now. `tests/status_paths.rs` checks it.
- **Order:** a path is shortened first, and only its folders go: the file name always stays. The line is then cut between its parts from the end, so the message's first words win over anything after them. The refusal (chip) line carries no path, so (a) and (b) cannot show a path. Manufacture writes that line itself every frame.
- **Not this branch's issue:** at 480, a CAM document's simulation panel trips egui's negative-width assertion (gui_camsim_panel.rs:475) a few seconds after it starts. CAM-UI has fixed it on cam/narrow-fixes (d203a64c), which isn't in main yet.
