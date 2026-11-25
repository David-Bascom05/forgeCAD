# BUTTON-AUDITOR handoff

David asked for a button and tool auditor: click every command on every screen, see what comes up, judge it against UX-LEAD's rules, and redo the tools that look bad. This note covers what exists, where it is, what is unmerged, and what's left.

## What exists

- **The crawler**: `interface/forge-render/tests/ui_audit/` (`main.rs`, `snap.rs`, `judge.rs`).
  - It enumerates every command from the app's own registry: ribbon tools on every tab and environment, the File menu, browser row menus, side-panel buttons and the tabs.
  - It runs headless on lavapipe in 12 fixture documents. Each click is classed as DIALOG, PANEL, WORKSPACE, MODE, ACTION, HOST, DISABLED, ERROR TEXT, NOTHING VISIBLE or CRASH.
  - The judge then checks:
    - layout: off-screen, clipped (side-cut only; a list's scroll edge is fine), overlap, tiny text, contrast;
    - words: raw or debug text, missing units, label rules;
    - behaviour: a reason on hover for off controls, an exit (Cancel/Close/"Back to …", or Esc), focus per STANDARD §9;
    - newest: scroll bars visible.
  - Its env vars are listed at the top of `main.rs`: `UI_AUDIT=full`, `UI_AUDIT_ONLY`, `UI_AUDIT_TAB`, `UI_AUDIT_SIZE`, `UI_AUDIT_OUT`.
  - CI runs the smoke subset (`smoke_every_3d_model_tool_shows_something`, about 35 s in debug) against `tests/ui_audit/known.txt`, which is empty. The full crawl is `tools/ui-audit/crawl.sh`, about 47 minutes at 1400 on 4 cores.
- **The report**: `tools/ui-audit/report.py CRAWL_DIR [NARROW_DIR]` writes `research/ui-audit/report.md`, `thumbs.md` and `research/button-audit/SCORES.md`. `research/ui-audit/status.tsv` says where each finding stands.
  - Regenerating drops the hand-written caveat paragraph at the top of report.md, so put it back.
- **The guards**:
  - `tests/narrow_reach.rs`: every tab and ribbon command can be clicked at 480 px.
  - `tests/focus_rule.rs`: STANDARD §9's digit hand-off, Esc after a digit, and no hand-off mid-drag.
  - `tests/scrollbars.rs`: handle against track at least 3:1, both themes, solid and floating.
  - `tests/status_paths.rs`: Manufacture's "Wrote …" says the file by its name.
- **The page crawl**: `interface/forge-render/web/tests/ui_audit.cjs`. It works on the plate document; a full page run was never done.

## The fix batches

Each is UX-LEAD OK'd. READMEs and before/after shots are in `research/button-audit/`.

| Batch | What |
|---|---|
| 1 (shell) | dead buttons |
| 2 (words) | error codes as words |
| 3 | Draft/Split/Combine/Delete Face 480 px crash; Update / Zoom All |
| 4 (units) | lengths in the part's unit through `ux::show`, inch formatting, the depth field's words |
| 5 (narrow) | 480 px: tabs behind », every window fits and is a bottom sheet below 720 px, the menus' sizing-frame bug, Material Browser list/detail, MIN_TEXT 11, Insert Hardware units |
| 6 | New Drawing units, sheet edge choices, the Versions empty state, Create Version without history, focus per STANDARD §9, Punch Tool and other words |
| 7 (scrollbars) | handle in the ink; no "Off:" before reasons; merged status bar |

## Unmerged (train 8)

- The branches are stacked: ui/audit → ui/audit-shell → ui/audit-words → ui/audit-words3 → ui/audit-units → ui/audit-narrow → ui/audit-b6 → **ui/audit-scrollbars**. The last carries all of them, plus main c135b274.
- **The head to merge is `ui/audit-scrollbars`** (6d610f35 is PRE-VERIFY GREEN on main c135b274; this note rides on top of it).
- UX-LEAD has OK'd every commit up to 6d610f35.
- `audit/fg-demo-refusal` on origin is a cancelled task. Its delete was refused (403). Ignore or delete it.

## Left to do (no new audits were started)

1. **A fresh full crawl after train 8**, at 1400 and at 480, then `report.py` and `status.tsv`.
   - The last full crawl was on ui/audit-b6 before the focus, judge and scrollbar fixes.
   - Since then, in the first crawl's terms:
     - ERROR TEXT 28 → 0
     - raw-text 55 → 11
     - tiny 30 → 0
     - off-screen 17 → 0
     - no-unit 14 → 0
     - contrast 6 → 0
   - The 14 NOTHING VISIBLE results were in owned areas whose fixes are on their branches.
2. **Typed-first windows not yet focused on open** (STANDARD §9): Parameters (part and assembly), Import, and Capture Settings. Use `dialog::focus_on_open(ctx, key, id, text)` before the field, with the TextEdit's `.id(id)`.
3. **Owners' fixes to re-crawl once in main**:
   - Agent tab (agent/tab-8: resume, switch, markup, "+")
   - Machine Builder's Home Axes (machines/builder-off-reasons)
   - CAM (cam/export-honest, cam/narrow-fixes: the 480 sim panel crash at gui_camsim_panel.rs:475, Tool Library scroll bars, the "Wrote" note)
   - Bend Simulation (BENDSIM-ENV)
   - Record Turntable's pill (render/studio-model)
4. **CAM-UI's `said_words` shortener** can switch to `shell::brief` once it is in main, so there is one function.
5. **Run the page crawl in full** (`web/tests/ui_audit.cjs` on a web build) and add it to the report.

## Risks and notes

- **`rustfmt` on `shell/mod.rs` reformats every submodule**, and several shell files were not rustfmt-clean to begin with. Format single files through stdin (`rustfmt --emit stdout < f`), or you'll touch other owners' code.
- **Crawler binaries are about 1.3 GB each**, and forge-render's debug target is about 22 GB. The container's disk fills fast, so clear `target/debug/incremental` and old test executables.
- **egui's first frame of a new Area or Window is an unseen sizing pass.**
  - Places recorded then are wrong. They are now truncated in the » menu, the ribbon group menus, and `Placed::show_placed`.
  - Focus requested then doesn't hold, which is why `focus_on_open` asks over the first few frames.
- **The judge's text-field count** (`text_fields`, from egui's TextEditState count) tells the judge when a window has nothing to type into. A window that reuses a TextEdit id from earlier in the run can read as "no field".
- **Esc is checked by the dialog's own (largest) layer going away.** A dialog whose biggest layer stays up after closing would read as closed.
