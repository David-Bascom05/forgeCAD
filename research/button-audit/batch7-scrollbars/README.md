# Batch 7: scroll bars you can see, and reasons without "Off:"

Branch `ui/audit-scrollbars`, stacked on `ui/audit-b6`. This was CAM-UI's finding, routed through the coordinator. UX-LEAD chose option (A).

| # | What | Before | After |
|---|------|--------|-------|
| 1 | Solid scroll bars (egui `ScrollStyle::solid()`) drew their handle in `widgets.inactive.bg_fill` over a track in `extreme_bg_color`. Both are `t.field`, so the handle was the track's own colour, 1.0:1, in both themes. The theme now sets `scroll.foreground_color`, so the handle is drawn in the ink: 15.3:1 dark and 16.6:1 light. A floating bar also faded to nothing at rest. Its handle now rests at 0.6 of the ink (5.8:1 dark, 4.4:1 light), so a list that overflows always shows that it scrolls. The three `solid()` sites (iProperties, Settings, the Material Browser) use `theme::solid_scroll()`. | before-1-materials-scrollbar | after-1-materials-scrollbar |
| 2 | A ribbon tool that is off gives its reason alone, as a sentence ("Open or create a part first …"), with no "Off:" before it. Versions' hovers read the same way ("No part open, so there are no saves to list"). The drawing editor's "On/Off: click to turn …" is a state, not a reason, so it stays. | | after-2-reason-without-off |

## Checks that keep this fixed

- `tests/scrollbars.rs` draws a solid bar and a floating always-visible bar with the app's theme applied, in both themes. It reads the colours egui paints for track and handle, and needs at least 3:1. Without the fix the solid bar measures 1.0:1 and the test fails.
- Crawler: the new judge rule `!scrollbar` fails any scroll bar a click brings up whose handle is under 3:1 against its track, or against the panel when the bar floats. On the plate at 1400 without the fix it flags Materials, Settings and Machine Builder (1.0:1). With the fix it flags nothing.
