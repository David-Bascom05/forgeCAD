# UI gaps in the features merged in rounds 5 and 6

What a person meets that is unfinished, by feature, with what was fixed here and what waits on
code that isn't on this branch yet.

## Weldments (this branch)

| gap | state |
|---|---|
| The section list is one dropdown of ~200 designations | **fixed**: Family, then Size in that family (`after/weld-4-section-picker`) |
| A member slot took picks from the browser and chips only | **fixed**: a click on a member in the view takes it; an unnamed edge reads as its face's member (tests/weld_ui.rs) |
| Cut List rows don't light their members in the view | open: the browser lights one feature's faces; a row is several members. Needs a multi-feature highlight |
| End Trim's "To" takes a member or body from the browser only | open: a view click on a member face gives the face (a plane trim), which is also valid; a "the member, not its face" toggle would make the cope reachable from the view |

## Sheet vendors and DFM (claude/repo-review-mbdrlj-sheet-vendors, not merged here)

Seen in its own test's pictures (`gaps/vendor-window.png`, `gaps/vendor-dfm-chosen.png`); the
branch predates this branch's design system, so after the merge it takes the theme (one accent,
units chip) but not these:

| gap | fix |
|---|---|
| Vendor window and DFM panel use their own frame and a "Close" button, not `panel_frame` and `dialog_head` (icon, title, ×) | switch to them |
| Buttons touch (`Use for this part`\|`Check (DFM)`, `Import`\|`Reload`) | `dialog_head` gives the body the S2/S1 spacing |
| Labels after their dropdowns (`5052-H32 aluminum ▾ Material`) | label first, as every dialog |
| Source URLs as one run of raw text | one "Sources" line per link, or a "Sources (4)" hover |
| DFM message numbers unrounded (`1.736872506509 from the bend`) | round to the shop's resolution (0.01 mm) |
| DFM rule shown as a red chip with its code (`HoleNearBend`) | the Issues row form: severity pill, the rule in words, the code in secondary ink |

## Bend simulation (same branch)

No pictures in its tests; to look at once merged.

## Mesh export, sketch text (this branch)

| gap | state |
|---|---|
| Sketch text preview is the UI font, not the chosen one | open: the glyphs come from the build at Finish; a live preview needs the font's outlines in the sketcher |
| Mesh export result only in the status bar | open, small: a "Show in folder" after export (native only) |

## CAM tool library

No CAM code is on the integration branch yet (research/cam only); its UI waits on the
`forgecad.cam/1` schema.
