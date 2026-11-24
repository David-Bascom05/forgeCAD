# forgeCAD UX standard

The bar every environment, panel and 3D preview must clear before it rides a merge train. It is
built from the app as it already looks at its best (the 3D Model tab, Thermoform, the
Manufacture tree, Render Studio) and from what David rejected in the Library Builder and the
Bend Simulation ("Those screenshots are of nonsense", "the tools seem so unwieldy").

Owner: UX-LEAD. Shared widgets that implement these rules live in `interface/forge-render`
(section 12); use them rather than hand-rolling each rule.

Screenshots cited as `branch:path` are the review set this standard was written against.

## 0. The five rules that matter most

1. **The view shows the thing you are working on, and only that.** No floating panels over an
   unrelated part. An environment owns the window.
2. **Never draw an invented solid.** Geometry comes from the record's dimensions. What is unknown
   is drawn as a ghost with a label saying so.
3. **Frame the subject.** On entry, on selection and after every change of subject, the camera
   fits the subject with margin, from a 3/4 view, outside every other body.
4. **Speak like an engineer, not like a TOML file.** Human labels with units; the raw key is a
   hover detail.
5. **Every number says where it came from**, with the same badge everywhere.

## 1. Environment layout

An environment (research/ui/environments.md) is a mode of the window. Its layout is fixed:

| Region | Holds | Rules |
|---|---|---|
| Ribbon | the environment's tab(s), then Inspect / View / Manage | groups named by job (Setup, Machine, Motion, Play, Exit); the last group is **Exit** with one **Finish ⟨Environment⟩** button; icons on every large button; no stand-in or debug actions in the ribbon |
| Browser (left, 270 px) | the environment's own tree: the library, the program, the setups | replaces the part's feature tree while the environment is active; one row per object with a thumbnail or icon, name, short kind/status at the right edge; selection drives the view and the property sheet |
| View (centre) | the subject, framed | only the subject and its context (machine, stock, press); the part's own model hidden unless it is the subject; standard view-cube, nav bar and Axes/Grid/Planes/Anchors chips stay |
| Properties (right, docked, 320–380 px) | the selected object's property sheet | docked to the right edge, full height below the ribbon, scrolls; never a floating window over the view |
| Timeline / transport (bottom of view) | play, step, scrub | only for things that move (CAM sim, bend sim); one row of transport, then the track; labels inside the track segments |
| Status bar | one line: what you are in and what Finish does, or the subject's summary | e.g. `Bend Simulation · TruBend 5130 · 2 bends, feasible`; never developer text |

Do:
- `ui/environments:research/ui/environments/01-environments-tab.png`: tree left with human values
  (`Blank 186 × 146 × 2.0`), subject centred and lit, nothing floating.
- `cam/ui-3:research/cam/ui/cam-2-selected.png`: setups/ops tree left with status ticks, the
  selected op's sheet with units (`12000 rpm`, `3726 mm/min`), transport + coloured op track.

Don't:
- `machines/brake-tabs:research/cam/machine-builder/30-workholding-vise.png`: the Machine Builder
  floats two windows and a "Checks" bar over `part_tools.fg`, whose feature tree (sk1, extrude1,
  fillet1…) stays in the browser and whose volume stays in the status bar. None of it is the
  subject.
- `bendsim/env:research/bendsim/env/shots/native-00-trubend-default-view.png`: the browser is a
  long mixed column (machine picker, specs, program, `up`/`dn` buttons, a check table) cut off at
  270 px; the checks belong in the properties panel or a Checks tab.

## 2. Camera framing

- On entering, on selecting a new subject, and on Fit (F / Home): fit the subject's bounds so the
  subject fills **60–75 % of the view's shorter side**, centred in the view area *not covered by
  docked panels*.
- Default direction: the 3/4 iso the app already uses (front-right-top), perspective,
  look-at = bounds centre. Long thin subjects (a 2.5 m die, a bar) are framed along their length
  from the 3/4 view so their section is readable, plus a section inset (section 4) — never a
  sliver across the screen.
- The eye is **outside every body in the scene**. A press, a machine enclosure or a vise must
  never be clipped by the near plane or fill the screen with one grey wall. If the subject sits
  inside a machine (the sheet between punch and die), frame the working zone from a viewpoint
  that sees both, with the machine's near walls cut away or ghosted.
- World axes and grid are scaled to the subject: grid spacing a round number about 1/10 of the
  subject; axes no longer than the subject's size; never infinite red/green lines crossing the
  whole view over a 50 mm tool.
- Animated subjects (bend, toolpath): the camera does not cut per step; it holds a frame that
  contains the whole motion envelope; "Follow" is opt-in.
- Helper: `camera::frame(bounds, view_rect, Dir::Iso)` (section 12). No environment computes its
  own fit distance.

## 3. Style: Realistic for physical things

- Physical objects that exist in a shop (machines, presses, punches, dies, vises, chucks, tool
  holders, cutters, stock, sheet) are drawn in the **Realistic** visual style with
  **RENDER-STUDIO appearances** chosen from the record's material: steel tooling = brushed/ground
  steel, cast machine frames = the vendor's paint colour if published else neutral machine grey
  paint, carbide = carbide grey, aluminium stock = machined aluminium, sheet = the sheet's
  material. Use the appearance library; never a flat `0xC0C0C0`.
- The design part being worked on keeps the document's own style and appearance.
- One key light + environment, soft ground shadow (as Thermoform and Render Studio already do).
- Colours have meaning and only one meaning: blue = selected / the work piece in motion,
  orange = tool path rapid / warning, red = collision or failure, green = pass. Don't use
  selection blue for the sheet at rest; use its material and highlight it on selection.

Do: `ui/environments:research/ui/environments/01-environments-tab.png` (mould in a warm material,
formed part in white plastic, shadow), `render/studio-ui:research/render/studio/02-rack-assembly.png`.

Don't: every Library Builder preview (flat light grey boxes on a grid), the bend sim's press
(untextured walls, one of which fills the right half of the view).

## 4. Preview fidelity

A preview is a claim that the thing looks like this. So:

1. **Real geometry from the record's dimensions.** A vise is jaws + body + screw at its
   `jaw_width`, `max_opening`, `body_length`, `body_width`. A press brake is frame side housings,
   ram, bed, back gauge at its `throat`, `bed_length`, `housing_gap`, `open_height`, `stroke`. A
   cutter is shank + flutes (fluted helix or at least the cut length shaded differently) at
   `dc`, `dcon`, `lcf`, `oal`. A die is its section swept along its *segment* lengths, not the
   longest one.
2. **Unknown = ghost + label.** A dimension the record does not publish is drawn as a
   translucent, outlined envelope (30 % alpha, dashed silhouette) with a leader label:
   `height · unknown`. A whole object with no geometry is a ghost bounding box with its name and
   "no geometry published".
3. **Never an invented solid.** No placeholder blocks presented as the product, no default sizes
   pulled from nowhere. A "stand-in" is allowed only for the user's own experiments, always shown
   as a ghost with a `stand-in` badge, and never offered first.
4. **Conflict is visible.** A value with conflicting sources is drawn at the value used, with the
   conflict badge, and the sheet says which sources disagree.
5. **Long or small features get an inset.** A die or punch section is shown as a 2D section inset
   (to scale, dimensioned: V, angle, radius, height) in the corner of the view, because a 3D
   sliver can't show it.
6. **Scale context.** A small object (a tool, an insert) is drawn with a scale bar or next to a
   faint 100 mm reference cube toggle; a large one (a press) with a 1.8 m human silhouette toggle
   off by default.

Don't:
- `machines/brake-tabs:research/cam/machine-builder/30-workholding-vise.png`: the Haas 6" vise is a
  tall grey box with two slabs on top: invented proportions (its `height` is in conflict and its
  jaw thickness unknown, yet both are drawn solid).
- `machines/brake-tabs:research/cam/library-builder/brake-0-press.png`: "TruBend 5085" is three
  grey slabs; frame, ram and bed values are UNKNOWN but drawn as solids.
- `machines/brake-tabs:research/cam/machine-builder/20-tool-library.png`: flat-10 is a plain
  cylinder; the flute length is published (`lcf 25`) and not shown.

## 5. Labels, values and units

- Every field has a **human label** in sentence case: `Cutting diameter`, `Shank diameter`,
  `Flute length`, `Overall length`, `Flutes`, `Jaw width`, `Max opening`, `Tonnage`,
  `Throat depth`. The raw key (`dc`, `lcf`, `jaw_width`) is shown **on hover**, in mono, with the
  file path: `dc · builtin/tools.toml`.
- Every number carries its **unit**, from forge-units, in the user's unit system (the status-bar
  mm/in toggle): `10.00 mm`, `850 kN`, `25 m/min`, `30°`. A bare `150.0` or `2550.0` is a defect.
- Numbers are formatted to the precision the source gives (no `44.07 mm` from `1.735 in` unless
  you say "converted"), right-aligned in a fixed column, never truncated. Fields size to their
  content or the panel widens; text that doesn't fit wraps or ellipsises *with* a hover showing
  all of it. `carbic`, `gener`, `forgec`, `2026-` are defects.
- Booleans are checkboxes with a label (`Real product`), enums are dropdowns with human names
  (`Linear`, `Table side`), lists (die segments) are a small table with a header row
  (`Length · mm`, `Count`) and a `+ Add segment` button.
- Group the sheet into titled sections (Size, Cutting, Holder, Ratings, Source); headings in the
  theme's section style, not `RECORD` / `WORKING GEOMETRY` caps typed as text.
- Titles are the product's name (`Haas 6" CrossOver vise · 05-0404`), with the record id as
  secondary text. Don't glue strings (`visebuilt-in`, `pressbuilt-in`, `diebuilt-in`).
- Abbreviated button text is a defect: `up`/`dn` → ↑/↓ icon buttons with tooltips; `x` → a
  delete icon; `|<` `<` `>` → transport icons.

## 6. Provenance badges

One widget, `provenance_badge(ui, Source)`, everywhere a value is shown:

| Source | Badge | Colour |
|---|---|---|
| Published (cited) | `published` + hover: the citation (doc, page, URL, date) | blue |
| Estimate / derived | `estimate` + hover: how it was derived | amber |
| Conflict | `conflict` + hover: the disagreeing sources and values | red |
| Unknown | `unknown` + a `Set…` button | grey italic |
| User | `yours` + hover: when set, and the original value | violet |
| Uncited | `uncited` + hover: "no source given" | grey |

- The badge is a small pill after the value, same width column for every row, never wrapping
  under the field. The `?` buttons go: the badge *is* the hover target.
- Unknown values don't show an empty field *and* a `set` button *and* `not given`: one
  `unknown · Set…`.
- Checks that could not run show `not checked` with the reason (which value is unknown), never
  `pass`.

Do: the CAM op sheet's `Status certified` line (one source line, plain words).
Don't: `machines/brake-tabs:research/cam/library-builder/brake-0-press.png` (four badge
spellings per row: `UNKNOWN`, `set`, `not given`, `published ?`).

## 7. Empty, error and loading states

- **Empty**: every list and view that can be empty says what it is and offers the next action:
  "No user tools yet · Derive from a built-in tool or Import…". Not `PROJECT none / USER none`.
  Empty sections collapse to a single line.
- **Error**: inline, next to the field or object at fault, in plain words, with the fix as a
  button when a fix exists (the kernel's typed fixes). A banner only for a whole-environment
  failure. Never a Rust `Debug` dump.
- **Loading**: anything over 150 ms shows a spinner or progress where the result will appear
  (the view gets a centred spinner; a list gets skeleton rows); the rest of the UI stays live.
  Long jobs (render, sim) show progress and Cancel.
- **Disabled**: a control that can't be used is drawn off with its reason as a tooltip (the
  Environments tab already does this). No buttons that pretend.

## 8. Lists with thumbnails

- Library lists (machines, tools, holders, workholding, punches, dies, presses, materials,
  appearances) show a **48 px thumbnail** per row, rendered from the record's real geometry by the
  shared thumbnail renderer (section 12), cached by record hash. Unknown geometry gets the ghost
  icon of its kind.
- Row = thumbnail · display name (bold) · one secondary line (maker, key size: `Ø10 · 4 fl ·
  carbide`) · provenance dot at the right.
- Group by maker/kind with counts, collapsible; search filters live and highlights the match;
  levels (Project / User / Built-in) are a segmented filter, not three headings with `none`.
- Lists longer than the panel scroll inside the panel; the panel never cuts a row in half
  (`Bystronic (4)` half-clipped in brake-0-press.png).

## 9. Keyboard and undo

- Every environment: Esc = clear selection / close popover (never exits the environment);
  Enter in a field = commit; Tab / Shift-Tab move through the property sheet; F = fit subject;
  Space = play/pause for anything with a transport; ←/→ = previous/next step; Delete = delete the
  selected user record (with undo, not a confirm dialog).
- Ctrl-Z / Ctrl-Shift-Z undo every edit an environment makes (record edits, program order,
  tooling choice), through the document/library history, not a private stack.
- Every command is reachable by a ribbon button, its keyboard shortcut (shown in the tooltip) and
  the agent tool layer (docs/interface-design.md: "a button press is a tool call").

## 10. Browser and native parity

- The wasm build and the native build show the same layout, labels, previews and states for the
  same document; the review screenshot set includes both (`native-*.png`, `web-*.png`) at
  1400×860 and 1600×1000, and one at 1024×700 to prove nothing clips.
- Differences are allowed only where the platform forces them (file dialogs, menus) and are
  listed in the branch's README.
- Fonts: the app's Inter / mono pair at the theme sizes; nothing smaller than 11 px.

## 11. The review gate

A UI-facing branch rides a train only with:
1. A fresh screenshot set (native + web) produced by its own headless test
   (`FORGE_UI_SHOTS=DIR`), committed under `research/<area>/shots/` from the branch head.
2. UX-LEAD's OK on that set against this standard (sent to TRAIN-CONDUCTOR).
3. P0 items (sections 0, 2, 4, 5 units/truncation) all fixed; P1/P2 may be tracked as follow-ups.

## 12. Shared toolkit (interface/forge-render, owned by UX-LEAD)

Adopt these instead of writing your own:

| Widget / helper | Does |
|---|---|
| `ux::PropertySheet` | sectioned label/value/unit/badge grid; raw key on hover; sizes columns; handles unknown/set |
| `ux::provenance_badge` | section 6 |
| `ux::thumbnail` | renders a body list offscreen to a cached 96×96 texture, 3/4 view, Realistic |
| `camera::frame` | section 2: fit bounds into the uncovered view rect, from a direction, eye outside obstacles |
| `ux::ghost` | translucent outlined envelope + leader label for unknown geometry |
| `ux::empty_state` / `ux::loading` | section 7 |
| `ux::LibraryList` | section 8 rows, groups, levels filter, search |
