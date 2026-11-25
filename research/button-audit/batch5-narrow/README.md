# Batch 5: the narrow window (480 × 860)

Branch `ui/audit-narrow`. These shots come from the crawler at `UI_AUDIT_SIZE=480x860`, except 2 and 7, which are at 1400 × 860.

| # | What | Before | After | Judge, before → after |
|---|------|--------|-------|-----------------------|
| 1 | Tab row: the tabs that don't fit go behind **»**, and the active tab always stays in the row. | before-1-tabs-480 | after-1-tabs-480-view, after-1-tabs-480-more | tabs off the right edge, none reachable → every tab can be clicked |
| 2 | Content Center cards: the family line wraps to two 11 px lines and is centred, with an ellipsis if it is still too long. | before-2 | after-2 | clipped → none |
| 3 | Settings: the window is at most 16 px narrower than the app, and the side list is 120 px when narrow. | before-3 | after-3 | off-screen → none |
| 4 | Material Browser: below 720 px it shows the list *or* the details, with a "‹ Materials" back link and the libraries folded above the list. Esc steps back from the details first. | before-4 | after-4-480-list, after-4-480-detail; 1400 is unchanged | off-screen 566 px → fits |
| 5 | Sheet Metal Vendor: the width is capped, and the parts stack top-down. Text below 11 px is raised to 11. **Esc** closes the DFM report and then the window. | before-5-vendor-480 | after-5-vendor-480 | off-screen 566×353, 2 × clipped, no Esc → none (no-focus remains) |
| 6 | Cut List: the width is capped at the app's width minus 16, and the table scrolls both ways. | before-6-cutlist-480 (801 px wide, starting at x = −321) | after-6-cutlist-480 (420 px) | off-screen, empty → only "Start" clipped, at the scroll edge (by design) |
| 7 | The Joint step badge has near-black `theme::ON_FOCUS` numbers on amber. | before-7 (white, 2.2:1) | after-7 (over 8:1) | contrast 2.2:1 → none |
| 8 | Insert Hardware: each number carries its unit through `ux::show` ("4.22 mm"), under "Mounting hole" and "Edge, at least". On hover it says what the number is and where it comes from ("The mounting hole for the M3 nut: Ø 4.22 mm. From …", or that the row is a placeholder). Tooltips now wrap at the window width minus 16 px (egui's 500 px default ran off a 480 px window). | before-8-hardware-1400 | after-8-hardware-1400, after-8-hardware-480, after-8-hardware-hovers | 9 × no-unit → none |

Also in this batch:

- `theme::MIN_TEXT = 11`: the sans, medium, semi and mono helpers clamp to it. The assembly glyph's 9 px count badge is the one deliberate exception.
- `Window::show_sizing` caps a dialog at the area minus 8 px and keeps it inside the window, so no dialog can leave a narrow app.
- The ribbon's missing tools (Hole, Thread, Direct, Section…) at 480 px (3ad70935). egui's first, invisible frame of a new menu lays it out at x = 5. The » menu and the folded group menus recorded their rows from that frame, so a click aimed there missed and closed the menu. As a result, the tabs behind » were never reached, and all of Modify read "not placed". Those rows are no longer recorded during a sizing pass. On the start and plate documents at 480 px, "not placed" fell from 54 rows to 0, and "nothing visible" from 30 to 4 (all side-panel items).
- Test `tests/narrow_reach.rs` runs at 480 px on every tab. It checks that every ribbon command (75 with no document, 97 on a part; tabs behind » are reached by clicking ») has its own on-screen button. If its group is folded, it opens the group's menu and checks the command's row is on screen. It fails with the list of commands that are lost.
