# Render Studio › Model: audit and redesign

David, 2026-11-24: "the Model tab is like unusable. It's just a list of dropdowns that grows forever."

## Audit (main 0de84ac1; before-*.png, 1400 × 860)
- One full-width ComboBox per **unique part mesh** in the scene (`StudioUi::looks`, by mesh index), each
  over a tiny caption with its file stem. The rack (rack18u/asm/rack.fg: 860 occurrences, 68
  meshes) gets 68 of them; arm2 gets 3; a single part gets 1.
- Each opens the whole library (46 appearances), unsearchable. The default reads "Material's own"
  without saying which material.
- No link to the view: nothing says which row is which part, and a part can't be picked in the view.
  No multi-assign: 31 fasteners take 31 dropdowns.
- Lighting, Camera and Output come after the list: on the rack they are 68 rows down.
- A bug on the way: the default appearance comes from the **document's** material, so every part of
  an assembly renders with the assembly's material, not its own.

What a user does there: give parts their finish (frame painted, panels powder-coated, fasteners
zinc), check what a part is, then set the light, camera and output.

## Design (mock.png)
- An **appearance brush**: one active swatch, chosen from a searchable palette grouped by family;
  **click parts in the view to apply** (the primary flow); Esc stops.
- **Parts**: the model's tree, collapsed, filterable, virtualised; a chip per row (dashed ring = its
  own material's look); Ctrl/Shift multi-select → Apply / Reset; a row's chip opens the palette.
- **Overrides**: only what changed, with Reset.
- Lighting, Camera, Output fold to one-line summaries.

## Built (after-*.png, same files and size)
- `shell/studio_model.rs`: the brush (a chip, the name, Paint / Stop), the palette (search; metal and
  paint open, "Show N more" for the rest; metals' swatches as their F0 under a studio's average
  light, so they tell apart), the parts' tree (grouped by the part files' folders, collapsed,
  filterable, `show_rows`; chip, name, count per row; a row's chip opens the palette for it; a
  swatch dragged onto a row gives it that look; Ctrl/Shift multi-select → Apply / Reset / Clear),
  Overrides (only those, `show_rows`, × each, Reset all). Lighting, Camera, Output fold to a line.
- `gui_studio.rs`: a click in the view while painting gives the clicked part the brush's look (the
  view's own pick, spent); each part's default is its own material (`Occ::material`, from the
  assembly's part documents); the picture is made again whenever the looks change, however.
- Frame time (debug build, the tab alone, every group open): 250 parts 0.48 ms, 2,500 parts
  1.03 ms (`many_parts_keep_the_frame_flat`).
- Tests: shell::studio_model::tests (tree, filter, multi-select, palette search, frame time);
  tests/studio_model.rs (a click in the view paints, many parts one look, the filter, the rack's
  parts keep their own materials).
