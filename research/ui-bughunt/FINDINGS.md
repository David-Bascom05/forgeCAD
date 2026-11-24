# UI bug hunt: findings

The browser build (wasm32-unknown-unknown, forge-render's web shell) and the app around it, driven
headless (Chromium, SwiftShader WebGPU). Each entry: symptom, repro, root cause, fix, test.
Tools: `tools/ui-hunt/drive.cjs` (a long-lived headless app taking clicks, keys and shots over
HTTP, for exploring by hand).

## 1. An idle page draws every frame for good; frame read-back never answers

- Symptom: once a part is open, the page draws ~60 frames a second with nothing changing
  (`forgecad.state().frames` climbs: 1802 frames in 30 s idle), and `forgecad.pixels()` never
  resolves. On a slow adapter the hover pick under the pointer never comes back either.
- Repro: open any part offscreen (`?open=a.fg&offscreen=1280x800`), wait 5 s, compare `frames`.
- Root cause: `webapp.rs` `about_to_wait` asked for a redraw on every loop turn while a pick was
  out (`pick_busy()`). The native loop (`app.rs`) polls for the pick's answer
  (`ControlFlow::Poll`) and draws only once it is back. Every frame drawn while the pick's
  `mapAsync` is out queues more GPU work ahead of it; on SwiftShader the queue never drains, so
  the pick (asked for at the cursor whenever the geometry changes) never answers, the page draws
  forever, and the frame read-back's own `mapAsync` starves too.
- Fix: while a pick is out, a 4 ms timer wakes the loop to look for it (`Msg::Poll`), nothing
  drawn; drawn once `picks()` says the answer changed what's marked.
- Test: `interface/forge-render/web/tests/idle.cjs` (before: "an idle page drew 301 frames in
  5 s"; after: 0 frames, the read-back answers, and a hover is still picked and drawn).

## 2. Sheet metal's DFM check never runs in the browser

- Symptom: the DFM panel's report never appears in the page.
- Root cause: `shell/vendorui.rs` `VendorUi::poll` started the check on
  `std::thread::Builder::spawn`, which fails on wasm32-unknown-unknown; the failure was taken
  quietly (`if spawned.is_ok()`), with the source hash already marked checked, so it never
  retried.
- Fix: `crate::plat::spawn` (a thread natively, a Web Worker in the threaded build, inline in
  the single-threaded page).
- Test: the wasm-only clippy config (`interface/forge-render/web/clippy/clippy.toml`) bans
  `std::thread::Builder::spawn` and the other std calls that panic or fail in a browser.
