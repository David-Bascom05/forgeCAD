# Batch 6: words, focus, and what the fresh crawl found

Branch `ui/audit-b6`, stacked on `ui/audit-narrow`. It starts from a fresh full crawl at 1400 × 860 on this branch (862 commands). Compared with the first crawl of main, ERROR TEXT went from 28 to 0, raw-text from 55 to 11, tiny-text from 30 to 0, off-screen from 17 to 0, no-unit from 14 to 0 and contrast from 6 to 0.

| # | What | Before | After |
|---|------|--------|-------|
| 1 | New Drawing gives each sheet size in its own unit, muted beside the name: ANSI sizes in inches ("A 11 × 8.5 in"), ISO sizes in millimetres ("A4 297 × 210 mm"). Esc closes it. | before-1-new-drawing-1400 | after-1-new-drawing-units-1400, after-1-new-drawing-units-480 |
| 2 | Sheet metal dialogs list every edge choice while there are 24 or fewer. Past that, the list scrolls with its bar always drawn. Before, a hidden floating bar cut off the last choices with no sign that they existed. | before-2-flange-choices (ends at right_lip.tip) | after-2-flange-choices |
| 3 | With no part open, Versions says so, and points to Project. "This part" and "Show saves" are off, with the reason on hover. | before-3-versions-no-part ("No saves yet.") | after-3-versions-no-part-hover |
| 4 | Focus follows STANDARD §9 (see below). | | after-4-create-version-focus, after-4-iproperties-focus |
| 5 | Words: the ribbon button and status line now say "Punch Tool", not "PunchTool". Create Harness labels its field "Slack (ratio)". The Feature Control Frame tolerance shows "mm". | | after-5-punch-tool |
| 6 | Create Version and Release in a folder with no history no longer open a dialog whose save could only fail (with an error nobody saw). They open Versions, where "Turn on history" is, and the status line says why. With no part open, the panel's own Create Version and Release are off, with the reason on hover. Create's name hint is the next version's ("v3"), which is what an empty name becomes. | | after-6-create-version-no-history |

## Focus (STANDARD §9, ux/standard-focus)

- **Pick-first dialogs.** The pick slot that is taking picks is the focus, and keys go to the view. It is published as the place `focus:pick`, which the judge accepts. This covers the shared pick slot (feature dialogs and Machine Builder), the sheet metal dialogs' slot button, and the Joint/Constrain step.
- **Digit hand-off.** In a pick-first feature dialog, a digit (or `.` or `-`) typed while nothing has focus starts an entry in the dialog's first number field. Nothing is handed off mid-drag, or while a modifier or another key is held. Esc right after the digit puts the old value back and returns the keys to the pick; a second Esc closes the dialog.
- **Typed-first dialogs.** Their first field is focused when they open, with its text selected. Focus is requested over the window's first few frames, because egui's first frame is an unseen sizing pass. This covers feature dialogs with no pick, Create Version (Name), Release (Reason), the Material Browser (search), Place (filter) and iProperties (first field). The shared helper is `dialog::focus_on_open`.
- **Esc** now also closes New Drawing, Section, Explode, Create Version and Release.
- Tests: `tests/focus_rule.rs` covers the digit hand-off, Esc after a digit, and a digit typed mid-drag.

## Judge

- A window that adds no text field, such as a report or a list, has nothing to focus, so it passes. The crawler counts egui's saved text-field states before and after the click to tell.
- Text cut only at the top or bottom of its clip is a scrolling list's edge, so it is not counted as clipped. Text cut at the side is still `!clipped`.
- Elided text (egui's truncate, with the whole line on hover) is now the note `?truncated`.
- The judge accepts µm as a unit, and ratios such as K-factor need none.

## Routed

- AGENT-TAB: Resume and Switch (no exit), and Markup (empty). Fixed on agent/tab-8 cb0cf8c0.
- CAM-UI: Tool Library (no exit, clipped columns).
- Legitimate, so left as they are: "Build failed" after a rollback or suppress in sheet/thermo. It is the document's own `expect` failing, named in the banner with "Show issues".
