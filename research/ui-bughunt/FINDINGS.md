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

## 3. Versions > "Turn on history" crashes the page

- Symptom: clicking "Turn on history" in the Versions tab kills the app: `forge-app panicked: ...
  library/std/src/sys/time/unsupported.rs: time not implemented on this platform`, then
  `RuntimeError: unreachable`.
- Root cause: `forge_history::History::turn_on` -> `forge_git::Repo::init` -> `fsutil::write_new`
  -> `temp_beside`, which named its temp file with `std::time::SystemTime::now()` (panics on
  wasm32-unknown-unknown). Found with `twiggy paths` on the release wasm (every reachable caller
  of `<std::time::SystemTime>::now`): also `Odb::write`, so every version save.
- Fix: `kernel/forge-git/src/fsutil.rs`: in a browser the time part of the temp name is the
  epoch (the sequence number keeps names apart: one process, one memory). The native lines keep
  their line numbers (the shim sits at the file's end); callgrind band in the commit message.
- Test: `interface/forge-render/web/tests/nopanic.cjs` (clicks Versions > Turn on history; the
  old build panics, the new one says "History is on").

## 4. Agent > Send crashes the page

- Symptom: typing a request in the Agent tab and pressing Send kills the app (same panic).
- Root cause: `agent::conversations::Conversation::new` stamped the conversation id with
  `std::time::SystemTime::now()`; `agent::trash_path` (undoing an agent's new file) likewise.
- Fix: `forge_math::clock::system_now()` (the page's `Date.now()` in a browser, std natively).
  Now the page says "operation not supported on this platform" (Claude Code and Codex are desktop
  only: see 6).
- Test: `nopanic.cjs` (types and sends in the Agent tab).

## 5. A wasm-only clippy config for std's browser panics

`interface/forge-render/web/clippy/clippy.toml` (the root's rules plus `std::time::Instant::now`,
`SystemTime::now`, `std::thread::spawn`, `thread::Builder::spawn`, `thread::sleep`,
`std::env::temp_dir`), used only by the wasm32 lint run (`CLIPPY_CONF_DIR`, in web.yml and
ci.sh), so native-only code behind `cfg(not(target_arch = "wasm32"))` is not flagged. It flagged
18 calls in forge-render: the ones above, `load::bench_dir` and `claude::clean_cwd`
(`std::env::temp_dir` -> `forge_dirs::temp_dir`, the same natively), and the desktop-only
modules (agent/claude.rs, codex.rs, mcp.rs, speech) which a page never starts: allowed there
with a note. Limits: clippy lints only forge-render (its workspace's one member); kernel crates
reached from the page are covered by the twiggy check above, not by the lint.

## Open (reported, not fixed here)

- 6. Agent tab in a page defaults to "Claude Code" and offers the microphone, both desktop only
  (docs/web.md); a send says "operation not supported on this platform". UX decision.
- 7. Versions under `forgecad web`: the page keeps `.git` in its memory, but the server refuses
  every dot path (by design, docs/web.md Security), so each version write logs `400 .git/...: not
  a project path` to the console only; the status says "History is on: every save is kept" and the
  history is gone on reload. Needs a decision: server allows `.git` writes, or the page says
  versions aren't kept on that backend.
- 8. Rendering: an 80 mm box at the origin renders as a black silhouette (no shading, no edges)
  in the page on SwiftShader, while a 30 mm box renders shaded. Under investigation.
