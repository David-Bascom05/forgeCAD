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

## UX-LEAD's P1s and markup (review-*.png: the rack at 1400 and 480 wide)
- Painting no longer shows noise: until the restarted trace is readable (8 samples) the raster view
  in the same looks stands in (each placement's tint from its look, `viewport_tint`'s packing, in any
  visual style); then the trace fades in over 8 samples more. The count reads "Starting again…"
  meanwhile (after-arm2-painting.png, then -settled).
- The selection shows in the view: the chosen parts' faces in the selection's colour, over the trace
  as over the raster (`cube_only` draws them through the ghost pass); a row under the pointer
  pre-lights its occurrences fainter; the part under the pointer in the view lights its row; a click
  in the view (not applying) chooses its row, opens its group and scrolls it in (after-arm2-selected).
- "Apply to parts" is the brush's toggle (pressed while applying); the selection bar says "Apply X
  to N parts", "Reset looks", "Deselect", its count on one line; "68 parts · 860 in the assembly";
  families "Metal", "Paint & coating", …; hovers: a swatch's name and family (and "selected"), a
  ring's "Material's own: M", a part's file and material, a group's folder, the overrides' ×
  "Reset to the material's own"; each change of looks one Undo step (Overrides › Undo).
- Frame time with the whole palette open and a filter typed: 250 parts 0.39 ms, 2,500 parts
  1.04 ms; the filter compares without allocating for ASCII names.
