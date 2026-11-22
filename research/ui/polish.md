# ForgeCAD UI polish: audit and design system

Screenshots in `research/ui/before/` come from `tests/ui_tour.rs`
(`FORGE_UI_SHOTS=DIR cargo test --release --test ui_tour`), 1600×1000 at 1×, dark theme unless
named. After-images go in `research/ui/after/` under the same names.

## What reads as unfinished, ranked

Ranked by how often a person meets it × how much it costs the app's credibility.

### 1. Launch is an empty void — `01-empty`
The window opens to a blank view, a blank browser and a blank status bar, while the whole ribbon
stays lit as though a part were open. Nothing says what to do next.
**Fix:** an empty state in the view: the app's name, *New part* / *Open…* buttons, the recent
files (already kept by the File menu) and three shortcut hints (Ctrl+N, Ctrl+O, the command
search). The browser says "No document open". Ribbon tools that need a document draw disabled.

### 2. Spacing is zero by default, so controls touch — `06-measure`, `12-error-issues`
`theme::apply` sets `item_spacing = (0, 0)` globally. Everywhere egui lays out widgets itself
the buttons butt together: the Measure filter pills (Any|Faces|Edges…), *Copy all*|*Restart*,
the Issues window's fix buttons, combo boxes of different heights in one row.
**Fix:** an 4-px spacing scale (below); egui's default `item_spacing` becomes `(S2, S1)` = (8, 4),
`button_padding` (S2, S1); the hand-placed layouts that relied on 0 set it locally.

### 3. Three accents compete — every screen
Green (*File*, Settings' *Close*, the active document tab's edge), orange (the active ribbon
tab's rule, focus rings, the active selection slot) and blue (title bar, selection, toggles) all
claim "this is the important thing". Nothing tells a person which colour means *primary action*.
**Fix:** one brand accent (blue, the title bar's family) for primary buttons, the File button,
selected tabs and selection. Orange is kept for exactly one meaning: *the slot that takes your
next click* (an active selection row, a text field with focus). Green becomes `success` only.

### 4. Errors are a wall of coloured text — `11-error`, `12-error-issues`
The Issues window prints severity, code, location and message as runs of 11–12 px red/orange text
with no rows, no icons and no separation; the fix buttons are glued together. Meanwhile a part
that didn't build leaves the view empty with no word in it, so the failure is only visible in a
small status chip.
**Fix:** each issue a row: severity icon + coloured pill (`Error`, `Warning`), the message in body
ink, `feature · line 7:9` in secondary ink, fixes as small secondary buttons with S2 gaps, rows
divided by a hairline. A failed build puts a banner at the top of the view: "round failed:
the kernel refused it · Show issues".

### 5. Type has ten sizes — whole app
`grep` finds sans at 10, 10.5, 11, 11.5, 12, 12.5, 13, 14, 15, 16 and the three weights mixed
freely (e.g. the Measure title semi 14, the Issues title medium 14, the tool dialog's semi 15).
Half-point steps are invisible as hierarchy but visible as inconsistency.
**Fix:** a five-step scale (below); half sizes folded to the nearest step.

### 6. Floating panels each have their own frame — `04`, `06`, `12`
The tool dialog, Measure, Issues and Settings use different radii (4/6), paddings (8–14), title
sizes, close-button sizes and shadows (some none). The tool dialog sits over the browser with a
soft shadow; Measure and Issues sit over the view with almost no separation.
**Fix:** one `panel_frame(t)` (radius R2, padding S3, border `line`, shadow `elev1`) and one
`panel_title(icon, text, close)` row of fixed 28 px height used by all of them.

### 7. Settings is a modal without a scrim, over the ribbon — `08`, `09`
The window's top edge lands on the ribbon (y≈97), so ribbon labels peek out beside it, and the
app behind stays at full contrast. One preset is labelled with another product's name ("Inventor
grey").
**Fix:** a 40 % scrim over the app while Settings is open; the window centred in the area under
the ribbon; the preset renamed "Neutral grey". The footer: *Reset all* as a quiet text button on
the left, *Done* as the primary on the right.

### 8. The status bar: stray separators and cramped text — every screen
"`| mm`" always has a leading bar even when nothing is to its left; the left text starts 10 px in
with no vertical rhythm; the issues chip's hover fill is the only affordance that it's clickable.
**Fix:** status items separated by S3 gaps and 1-px dividers drawn *between* items only; units as
a chip; the issues chip always has its pill background (so it reads as a button).

### 9. Browser inconsistencies — `02` vs `13`
A part's browser is 270 px with no filter; an assembly's is 340 px with a filter field. The side
tab strip squeezes the file name ("part_tools") into the tab row where it collides at narrow
widths. The orange "changed" dots beside rows have no legend or tooltip and sit off the eye
column's alignment.
**Fix:** one width (the person's last), the filter in both; the document name on its own header
row (semi 13) above the rows; the dot gets a tooltip ("edited since the last save") and aligns to
the icon column.

### 10. Mode prompts are hard to read — `05-sketch-pick`
"Pick a plane, a planar face, or a sketch to edit" is ink2 on a near-black chip over a dark view;
the same words repeat in the status bar.
**Fix:** the prompt chip in `raised` with body ink and an accent left rule; status bar shows only
the shortcut hint ("Esc to cancel").

### 11. 3D view details
- The origin axes draw through the part (`02`: the blue Z line crosses the boss). Draw them
  depth-tested, or faded where hidden, as the grid already is.
- The view cube's face words are ~7 px and low contrast in dark; the cube ring is heavier than
  the cube itself.
- The grid is the same weight as model edges at this zoom; major/minor lines unweighted.

### 12. Document tabs at the bottom — every screen
22 px tall, monospace names, a green border marks the active tab.
**Fix:** 26 px, sans 12, the active tab in `raised` with the accent top rule (as the ribbon tabs).

### 13. Smaller items
- Measure: "Selection 1/2" labels in orange/green differ from the selection colours the view
  uses; the help text at 10 px.
- Tool dialog: the name field (`fillet2`) is mono 12 in a sans header; *OK* is a plain button.
  *OK* becomes primary.
- Light theme: `ink2` on `raised` is 5.9:1, fine; but `line` (#D5D9D3) is warm against the cool
  blues; harmonise to a cool grey.

## Design system

Lives in `src/shell/theme.rs` as constants; every panel takes its numbers from there.

### Spacing (4-px grid)
| token | px | use |
|---|---|---|
| `S1` | 4 | icon-to-label, tight rows |
| `S2` | 8 | between controls in a row, button padding x |
| `S3` | 12 | panel padding, between groups |
| `S4` | 16 | between sections |
| `S5` | 24 | empty-state blocks |

Control heights: `CONTROL_H` 26 (buttons, fields, combos), `ROW_H` 26 (browser/tree rows),
`TITLE_ROW_H` 28 (panel titles).

### Type
| token | font | use |
|---|---|---|
| `CAPTION` | sans 11 | group labels, secondary meta, hints |
| `SMALL` | sans 12 | dense UI: tree details, status bar, tables |
| `BODY` | sans 13 | labels, menus, dialog fields |
| `TITLE` | semi 14 | panel and dialog titles |
| `HEADING` | semi 18 | empty-state and settings page headings |
| `CODE` | mono 12 | names, values, file names in code context |

Weights: regular for text, *medium* for emphasis inside a row, *semi* for titles only.

### Colour tokens (added to `Theme`)
Existing: `bar bg surface raised ink ink2 line hover field view on on_ink focus error selection`.
New:
- `primary` / `on_primary`: the one accent for primary buttons and the File button
  (dark #2F6FB3 / white, light #1D5FA8 / white).
- `warn`: today's `fg::WARN`, moved in.
- `success`: today's green accent, kept only for "done/valid".
- `scrim`: black 40 % (dark 55 %).
- `shadow`: `elev1` = 0,4 blur 16, black 35 % dark / 14 % light.
`accent` stays as an alias of `primary` so nothing breaks.

### Radii
`R1` 4 (controls), `R2` 6 (panels, dialogs, chips).

### Component rules
- **Primary button**: one per panel, `primary` fill, white text, R1, height 26. Secondary:
  `raised` fill, `line` border. Quiet: text only, `ink2`, hover `hover` fill.
- **Panels/dialogs**: `panel_frame`; title row 28 px with icon 16, TITLE text, close × 20 px hit
  target on the right; body padding S3; footer buttons right-aligned, primary rightmost.
- **Focus**: 1-px `focus` ring only on the control taking keyboard input or the active
  selection slot; never as decoration.
- **Errors**: severity = icon + colour + word, never colour alone.
- **Empty states**: a heading, one sentence, the primary action, at most two secondary.
- **Hover**: `hover` fill on everything clickable; pointer cursor on custom-painted buttons.

## Status

Before/after pairs share names in `before/` and `after/`.

| # | item | state |
|---|---|---|
| 1 | Empty state | done: `shell/empty.rs`; "No document open" in the browser (`01-empty`). Ribbon tools that need a document: not yet |
| 2 | Spacing | done in every panel that uses `dialog_head` (Measure, Issues, inspect windows, asm dialogs); the global default is left at 0 because the hand-laid chrome depends on it |
| 3 | One accent | done: `accent` is the primary blue; the ribbon/side tab rules and the active document tab use it; the tool dialog's OK is primary |
| 4 | Issues rows | done (`12-error-issues`); the in-view "build failed" banner: not yet |
| 5 | Type scale | tokens in `theme.rs`; panels move to them as they're touched |
| 6 | One panel frame | done: `theme::panel_frame` behind `fgasm::dialog_frame`, the tool dialog and the mode bar; title row 28 px with a hover on ✕ |
| 7 | Settings | done: scrim, centred, "Neutral grey" (`08`, `09`) |
| 8 | Status bar | done: units chip, the issues chip always a chip |
| 12 | Document tabs | done: sans 12, the accent rule on top of the active tab |
| 9, 10, 11, 13 | | not yet |

Known, not from this work: `cargo clippy --lib --target wasm32-unknown-unknown` fails at
`errno` before reaching forge-render, through `zbus-secret-service-keyring-store` (not gated to
native targets in Cargo.toml). The same happens on the base branch.
