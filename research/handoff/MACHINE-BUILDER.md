# MACHINE-BUILDER handoff (2026-11-25, wind-down)

## What landed

All of this is on main; no branch of mine holds code that main lacks.

- **Library Builder as an environment**: machines, cutting tools, holders, collets, workholding,
  and press-brake punches, dies and presses. Trains 4b/5 (`machines/library-env`).
- **Library P1** (`machines/library-p1`, train 5):
  - the sheet shows the same values the view draws (conflict/estimate chips);
  - product-name titles and coatings in words;
  - a Project/User/Built-in level filter;
  - 48 px thumbnail rows;
  - a die-segments table;
  - checks in words.
- **Spinner U5-620 record** (train 5), with CAM-MACHINE's pivot fit and tool change 5.8 s.
- **Attached machine models** (`machines/attach-model`, train 5):
  - the user's own STEP/IGES/STL, kept in their library;
  - placed on a body's envelope and checked against it.
- **Model CLI** (`machines/models-cli`, train 7): `forgecad machine model
  attach|list|check|remove|split`.
  - The source note is required.
  - The split dialog has thumbnails, a suggested body, and click-in-view selection.
  - A display-only body's report is labelled.
  - CAM-HAAS's `haas-model-import` builds on it: shipped Haas models in LFS, `ship`/`fetch`.
- **Builder off-reasons** (`machines/builder-off-reasons`, train 7): the builder's tools say why
  they're off. Home Axes is off while the axes are at home (BUTTON-AUDITOR's item D).

## Unfinished (UX-LEAD's P1 list; not started on any branch)

1. **Narrow views** (< 1200 px of view width):
   - collapse the side-column labels into small numbered markers, with the text on hover and a
     "Notes (n)" chip;
   - keep the framed subject clear of the view's nav bar (at 1024 px a long die's far end
     runs under it: `web-1024x700-die.png`).

   Where: `shell/machine.rs` `overlay()` (labels), `gui_machine.rs` `frame_preview()` (framing:
   pass the nav bar's rect as an obstacle to `frame::frame`). Keep BUTTON-AUDITOR's
   `.show_fit` on the three `egui::Window`s.
2. **Sheet adoption**: move the library sheets (tooling, workholding, brake, machine) onto the
   shared `shell::ux::Sheet`, now on main. Today they are `egui::Grid`s built in each
   `machine_*.rs` `editor()` with `lib_form` helpers.
3. **Carbide**: draw a cutting tool's fluted part with RENDER-STUDIO's Carbide appearance
   (`appearance_for("carbide")`, on main since train 5) instead of black oxide.
   `library_mesh::Look::Cutting` → its `appearance()`.
4. **Machines' list rows**: these still use the old schematic icons, not `ux::thumbnail` like
   the other libraries. `lib_form::record_row` plus a machine preview (its bodies at home) would
   do it.
5. **Web "Upload model…"** (the coordinator's follow-up): in the browser, `plat::pick_file` only
   offers project files. Writing an upload into the user's library storage needs the browser
   host's support.

## Risks and notes

- **Formatting**: never run `cargo fmt` on the whole forge-render crate. It reformats other
  teams' files (it did once: fb929a84, reverted at the train-6 merge). Use `rustfmt` on your
  own files.
- **Fonts**: run `tests/glyphs.rs` before every push. A `→` in the split reply failed train 7.
- **File I/O**: everything goes through `forge_dirs::fs`, so the browser build works (UI-BUGHUNT's
  wasm rule).
- **Shipped models**: the user library and forgecad's shipped models interact (CAM-HAAS's
  `models_shipped`). Two follow-ups I sent them are unconfirmed:
  - removing every shipped model makes them come back (an empty user index is treated as
    "none");
  - opening Split copies the shipped models into the user library even on Cancel.
- **Pre-verify**: the record of my 7c9114d3 GREEN matches the tool bug the coordinator described
  (its clippy log has main's forge-topo "could not compile"). It is superseded by e68cb8ed's,
  which is clean, and the branch's content is on main.
- **Forge-exact digest**: sweep-diff digest lines for new machine parts need Lean (`lake
  build`), which wasn't available in this container. TRAIN-CONDUCTOR regenerates them.
