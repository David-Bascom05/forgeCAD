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

Also in this batch:

- `theme::MIN_TEXT = 11`: the sans, medium, semi and mono helpers clamp to it. The assembly glyph's 9 px count badge is the one deliberate exception.
- `Window::show_sizing` caps a dialog at the area minus 8 px and keeps it inside the window, so no dialog can leave a narrow app.
- Test `tests/narrow_reach.rs` runs at 480 px on every tab. It checks that every ribbon command (131 on the plate) has its own on-screen button. If its group is folded, it opens the group's menu and checks the command's row is on screen. It fails with the list of commands that are lost.

Still open in this area: (c) the Insert Hardware rows should read "tap drill 4.2 mm" through `ux::show`, with the standard on hover. Only the column headers carry units so far.
