# RENDER-STUDIO handoff

The Render Studio (`interface/forge-render`: `gui_studio.rs`, `shell/studio_env.rs`,
`shell/studio_model.rs`, `studio/`) and capture work from session RENDER-STUDIO, at the
wind-down (2026-11-25).

## Branches

| branch | head | state |
|---|---|---|
| `render/studio-model` | 02e52375 | in train 8 (merge-train/8), TRAIN-CONDUCTOR keeping that sha. PRE-VERIFY GREEN on main c135b274 (tools/preverify @35d9ccbe; its record checks clean of compile errors), UX-LEAD OK. Its later commit 0f230871 (this file) rides with studio-polish. |
| `render/studio-polish` | the commit carrying this line | for train 9: 31d5b85c (UX-LEAD's P2/P3 items from the 11-13 shots, UX-LEAD OK on 31d5b85c) with render/studio-model (0f230871, this file) merged in. Pre-verify with /tmp/pv @13d78f06 on this head; its line goes to TRAIN-CONDUCTOR. |
| `render/studio-model-t7fix` | c69ea889 | landed in train 7 (main c135b274). |
| `render/nickel` | 1fe0102a | landed in train 5. |
| `render/studio`, `-2`, `-3`, `-ui` | | in main (trains 5 and earlier); nothing to merge. |

## What train 8 brings (render/studio-model since train 7)

- The live trace takes a budget of the GPU a frame: a sample is traced a band of rows at a time;
  the picture is denoised and shown at 8, 16, 32, 64 samples, then every 64, the denoise also in
  bands. Natively the rows are fitted to 8 ms from the band's GPU time
  (`on_submitted_work_done`, less the least measured); in the browser from the page's frame times
  (two probe frames in 16, fitted once a 16, growth capped at 1.25x, between h/8 and h rows a frame).
  Traced frame here (lavapipe): median 23-28 ms vs 16-19 stopped (was ~230 ms).
- The badge over the view says it's still refining: "Tracing · N samples" with ux::loading's spinner;
  hover "Refines until N samples; edits restart it"; its own layer so the pointer reaches it.
- Renders go where captures go (`<document>-render-<time>.png`), "Wrote PATH"; the recording pill
  says Cancel and Esc.
- Tests: `the_live_trace_keeps_studio_frames_within_budget` (release), `web/tests/studio_live.cjs`
  (in CI beside studio.cjs), the click-frame test measured against a plain frame (software
  adapters by name/DeviceType::Cpu), the flat-frame lib test as the fastest of 30 frames, the file's
  GPU tests one at a time (`one_at_a_time`), `shots_for_ux` (STUDIO_UX_SHOTS=DIR).
- research/render/studio/11-13 shots.

## Not done / next steps

1. `render/studio-polish` into train 9 (UX OK; its pre-verify line is with TRAIN-CONDUCTOR, or
   re-run it if that's missing). Its items:
   - P2: the Appearance card's words wrapped beside Apply to parts, or the button below them in a
     panel under 300 px.
   - P3: spinner on Preparing / Starting again.
   - P3: hover "The live view refines until N samples; edits restart it. A render uses Output's
     samples."
   - P3: Output's Size/File and Lighting's File in the label column; File's hint shortened to
     "Capture folder" (the long one was cut off at that width: UX-LEAD may prefer the field on its
     own line instead).
   - P3, §2: Fit All on entering the Studio (flange ~20% -> ~65% of the view's height).
2. From the Model tab's review (UX P2, earlier): outline marks for selections of up to 256
   entities over the trace (today the chosen parts are lit see-through through the ghost pass).
3. A final render refining in the view (today a render runs in the background with its card and the
   live view paused).
4. ffmpeg for recording: macOS stays UNKNOWN until David picks a source (Linux/Windows are pinned in
   tools/ffmpeg/PIN; not Martin Riedl's nonfree builds); the real-download install test is the
   coordinator's to run on David's side.
5. CAM-UI's sim readout (`gui_camsim_panel.rs`, truncated on purpose) is flagged as clipped by
   BUTTON-AUDITOR's judge: theirs to settle (CAM-UI / BUTTON-AUDITOR).

## Risks

- The browser's pace is a heuristic on frame times. It is bounded (never more than a sample a
  frame, a sample in at most 8 frames) but on an unusual page (frames throttled in a background tab,
  a very fast GPU with slow CPU) it may sit at either bound. `studio_live.cjs` watches it on
  CI's software adapter only.
- Natively, the pace trusts `on_submitted_work_done` after a non-blocking poll; a driver that
  reports late would read as slow and keep the trace small (safe, slower to clean up).
- Timing tests on shared runners: the budget test (p90 vs median, 30 ms or a quarter of a slow
  frame), the click frame (plain + 20 ms; 100 ms only on hardware GPUs) and the flat-frame lib test
  are written to be robust, but a heavily loaded box can still produce outliers. Their failure
  messages name both numbers.
- `shell/capture.rs`'s Cancel discards a video but keeps PNG frames already written; the hover says
  so.
- The squash-merge history: when merging main into a branch, files main took from your own train
  head can conflict add/add; check main's copy equals your train head before resolving to your
  side (as done for c135b274).
