# CAM-UI handoff

CAM-UI owned the Manufacture workspace in `interface/forge-render`: its ribbon, browser, dialogs, NC Programs, the Tool Library page, Document Text, the export paths (native and page) and its tests (`tests/cam_ui.rs`, `web/tests/cam.cjs`). The document text is the source of truth (forge-cam-lang, forge-turn-lang); the UI only edits it.

## State at wind-down (main = c135b274, train 7)

One branch carries everything unmerged: **`cam/narrow-cube`** (on top of `cam/narrow-alternate`, which carries `cam/export-honest`). Train 8 takes `cam/narrow-alternate @a77d6380` (PRE-VERIFY GREEN, UX OK) and then `cam/narrow-cube` (UX OK, PRE-VERIFY sent to TRAIN-CONDUCTOR).

Every other `cam/*` branch of mine is merged (trains 5–7) or contained in the ones above. Do not merge any of them on their own.

| Branch | Head | What | Status |
|---|---|---|---|
| cam/export-honest | 0e4fcf30 | page says an export only when done ("Exported/Downloaded/Export cancelled"); an unanswered save dialog says it is waiting, with Try again; 480 px crash (timeline fold); tool library scroll bars; Document Text keeps typed text until Apply | in cam/narrow-alternate |
| cam/narrow-alternate | a77d6380 | at a phone-wide window the properties column and the view take turns (View tab, a chip back); main c135b274 merged | train 8, GREEN, UX OK |
| cam/narrow-cube | c9e83517 (+ this handoff) | half-size cube below 320 px of view and framing clear of cube and tool strip; "Wrote <file>" by name in ink2, path on hover | train 8 after narrow-alternate, UX OK |

## Known merge points (already told TRAIN-CONDUCTOR)

- `gui_camsim_panel.rs` ~l.589, the simulation tool label's `if`: keep all three conditions `!self.view_hidden`, `!st.playing`, `!dialog` (CAM-SIM's cam/sim-fixes adds `!dialog`).
- The status bar's fix chip: UI-BUGHUNT's train-7 drawing, plus Try again's hover from `cam::status_fix_tip`.

## Small follow-ups not done (none blocking)

1. Re-shoot `research/cam/ui/cam-480-view.png` once `ux/narrow-browser` is on main (the view becomes ~300 px, not 209). UX-LEAD asked for it "so the record matches main".
2. Re-shoot `research/cam/ui/cam-turn-section-framed.png` once `ux/part-box` is on main (the shaft then fills the free area; UX-LEAD wants to see it).
3. Tool Library scroll bars: once BUTTON-AUDITOR's `ui/audit-scrollbars` (theme::solid_scroll, scroll.foreground_color in the theme) is on main, drop the library's own `ScrollStyle { foreground_color: true, ..solid() }` in `shell/cam/ui.rs` (finder and detail_pane) for `theme::solid_scroll()`.
4. `cam::said_words` (Manufacture's notes) and BUTTON-AUDITOR's `shell::brief` (status line) both shorten "Wrote <path>" to the file's name: make one call the other after train 8 (TRAIN-CONDUCTOR's note).

## Risks and things to know

- Every test that clicks by name must wait on a state, not a frame count: use `tap` (waits for the place to be laid out) and loop on the state with a time bound. Fixed counts were the cause of every cam_ui flake (trains 5–7).
- Background jobs (CAM run, quality measure, alternative clear, Check Now) run through `plat::spawn` in `gui_cam.rs::cam_host`, inline in the atomics-less page. Results of jobs that can overlap must not share a single slot: `quality_job` is a Vec for that reason (a stale measure overwrote the current one).
- egui's `debug_assert`s (e.g. negative widths) only fire in debug builds: run cam_ui in debug too when touching layout at narrow widths.
- Shared files touched: `gui.rs` (a view change's last step redrawn), `nav.rs` (half cube), `shell/mod.rs` (narrow layout, status chip), `shell/docs.rs` (three app-wide Shell fields), `shell/env/mod.rs` (`properties_label`). Run the whole forge-render suite when changing them.
- Squash merges on main: merging main into a branch that holds train history conflicts widely; take main's tree and re-apply only the branch's commits after its train sha, one at a time (git apply -3), resolving each.
- Before saying two branches don't conflict, run `git merge-tree --write-tree A B` (I said it twice without checking, and was wrong both times).
