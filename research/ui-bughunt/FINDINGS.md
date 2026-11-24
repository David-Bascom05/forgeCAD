# UI bug hunt: findings

The browser build (wasm32-unknown-unknown, forge-render's web shell) and the app around it, driven
headless (Chromium, SwiftShader WebGPU). Each entry: symptom, repro, root cause, fix, test.
Tools: `tools/ui-hunt/drive.cjs` (a long-lived headless app taking clicks, keys and shots over
HTTP, for exploring by hand).

## Gates

- a4dcf98 (forge-git, kernel): callgrind exact band vs main f04c533 on this VM: worst strict
  line +0.0002% (gearbox app), strict total +0.0000%; bits identical.

## 1. An idle page draws every frame for good; frame read-back never answers (superseded on main by fb7a67fa, the renderer's pick waker; merged in favour of it)

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

## 6. Parts past ~45 mm drawn as a black silhouette (ambient occlusion)

- Symptom: in the page, a box 50 mm or more across draws as a flat black hexagon (no shading, no
  edges); 40 mm and smaller draw shaded. View > Ambient Shadows off brings the shading back.
- Repro: `?open=c.fg&offscreen=1280x800` with an 80 x 80 x 40 box.
- Root cause: `render.wgsl` `ao_fs` kept each pixel's view distance in mm in the green channel of
  the `Rg16Float` occlusion target; half floats end at 65504. A parallel view stands
  `PARALLEL_BACK` = 1000 radii off (camera.rs), so a part of radius ~35 mm or more was past it:
  the distance stored as inf, `abs(inf - inf)` made every smoothing weight NaN, the smoothed
  occlusion came out 0, and the multiply pass blacked out faces and edges. Native (lavapipe)
  happened to clamp the conversion; SwiftShader (and IEEE conversion on GPUs generally) gives
  inf, so this is not only a SwiftShader quirk.
- Found by bisecting the AO passes with debug shader outputs: forcing `ao_fs` to 1 still black,
  blur passthrough fine, blur weights 0, the stored distance ~76 000 for the 40 mm box.
- Fix: the distance kept in units of the near plane's (`p.z / near`, i.e. `1 / depth`: bounded,
  scale-free; the passes compare it only relatively, 1%), `ao_apply_fs` compares the same.
  Where it worked before, the frame is the same (max difference 1 level in the page, 4 natively).
- Shots: `shots/ao-80mm-before.png`, `shots/ao-80mm-after.png`, `shots/cover-before.png`,
  `shots/cover-after.png`. Confirmed by CAM-UI (black flange), MACHINE-BUILDER (machine bodies),
  ENVIRONMENTS/UX-LEAD (thermoform cover.fg 2242 -> 0 black samples, conic boss.fg 2214 -> 0).
- Test: `interface/forge-render/web/tests/shade.cjs` (30, 80 and 400 mm boxes: old build "the
  80 mm box drawn dark: mean 0.0, 1400 of 1400 black"; new: all shaded).

## 7. Thermoform > Check Forming crashes the page (a lock taken twice)

- Symptom: Check Forming on a thermoformed part (examples/thermoform/cover.fg) kills the page:
  `cannot recursively acquire mutex` (std's single-threaded Mutex), then `unreachable`.
- Root cause: `thermo_view.rs` `want_heat` (and `want_sim`) held the `made` lock while calling
  `plat::spawn` with a job that takes the same lock. Natively the job runs on another thread and
  waits; in the single-threaded page `plat::spawn` runs it before returning, on the same thread.
  Wasm stack from the panic (V8's `Error.stackTraceLimit` raised in drive.cjs): `want_heat` <-
  `Gui::apply_fg` <- `Gui::frame`. The same pattern in `gui_thermo.rs`: the re-simulation list
  (`RESIM`, Check Forming's suggested fixes) and an assembly's thermoformed part (`ASM_PART`).
- Fix: each guard dropped before `plat::spawn` (re-taken after where it's used again).
- Test: `nopanic.cjs` opens cover.fg and clicks Thermoform > Check Forming (old: the panic). The
  two `gui_thermo.rs` paths have no browser test yet (an assembly with a thermoform, and a
  suggested fix's re-simulation).

## 9. forge-lang kept every run's sketches (ui/bughunt-2)

- The Rc cycle of open finding 8 (below): `Sk.named` held the sketch's own entities, each holding
  the sketch. Fix: `sketch::detach` keeps them in `named` pointing at a thread-local stand-in,
  `sketch::attach` gives them the sketch back on member access (`plan.outline`); anchors and
  `sketch_view` read only ids and loops.
- Numbers (tools/ui-hunt/leak): 2000 runs 11 MB -> 16 KB, 2000 edits 44 MB -> 2 KB, flat.
- Band (callgrind exact, main 4846170a vs 319b39a3, host vm): strict total -0.0000%, worst strict
  line +0.0000% (volute edit); bits identical.
- Test: `forge-lang` `sketch::no_cycle::a_run_frees_its_sketches` (a sketch is freed with its
  evaluation; names still read through it). Fails before: "the sketch outlived its evaluation".

## 10. Versions under `forgecad web`, and in any page (ui/bughunt-2)

- Server (`forge-cli/src/web.rs`): the served root's own `.git/...` and `.gitignore` are project
  paths (written, read, listed, bundled), never `.git/hooks`, no other or nested dot path; a
  `.git/config` only with keys git reads as settings (refused: fsmonitor, hooksPath, sshCommand,
  alias, filters, includes: a page must not make the machine's own git run anything); a `.git`
  over 64 MiB stays on the machine.
- Page: a write the storage refuses reaches the app (`app_store_failed`): "Not kept: PATH: why"
  in the status, and Versions says the history isn't kept until a reload.
- Found on the way: in a page no save could be signed (no git identity), so history kept nothing
  under "History is on: every save is kept". Now the failure is the status, and Versions asks for
  a name and an email for the project's saves (`History::set_identity`).
- Test: forge-cli web tests; `web/tests/history.cjs` (writable: kept on the machine and back after
  a reload; read only: never "History is on").

## 11. Keys typed around a click were lost in the page (ui/bughunt-2)

- Symptom: typing a name, clicking the next field and typing on: "Test User" came out "Test",
  "test@example.com" "st@example.com"; "Jo Do", a click, "xy" kept only "Jo ".
- Root cause: egui moves keyboard focus between frames (a press away from a field takes the focus
  at the frame's start, the clicked field takes it at the frame's end), so keys in the same frame
  as a click reach no field. A page's frame waits for whatever ran before it on its one thread, so
  input typed meanwhile arrives with the click in one frame.
- Fix: `inputcut::frame_cut`: a frame's events end where a mouse button and the keyboard meet;
  the rest is the next frame's, drawn at once. Shots: `shots/typing-after.png`.
- Test: `inputcut` unit test; `history.cjs` types each field right after clicking it.

## 12. The Agent tab in a page (ui/bughunt-2)

- Nothing in it can run in a page (Claude Code and Codex are programs on the computer, speech its
  engine, the OpenAI-compatible path returns "doesn't reach endpoints yet"). The tab now says so
  plainly, with no Send to fail and no microphone. Shots: `shots/agent-tab-page-{before,after}.png`.

## 13. A selection window killed the page (ui/bughunt-2)

- Symptom: any left drag across the view (a window selection) panicked: `render.rs "it's been
  waited for": getMappedRange failed`, then `unreachable`.
- Root cause: `Renderer::view_ids` drew the view's ids and waited with `device.poll(wait)`; a
  page can't wait for the GPU (mapAsync answers in a later task).
- Fix: `view_ids_start` / `view_ids_take` (woken by the pick's waker); natively `view_ids` still
  waits; in the page the window is chosen when the ids are back (`Gui::picks`); Markup's marks go
  without their entities there. Test: `nopanic.cjs` drags a window (old build: the panic).

## 14. LAG: the page fell seconds behind the mouse (ui/bughunt-2)

Measured with `tools/ui-hunt/perf.cjs` (headless Chromium, SwiftShader WebGPU, 4-core VM):

| | before | after |
|---|---|---|
| small part: a frame read back right after a 1.3 s orbit | 12 844 ms | 503 ms |
| rack18u/asm/rack.fg (860 occurrences): same | 27 116 ms | 951 ms |
| rack.fg opened right after another document in the same browser | 15-28 s | 2.5-3.5 s |

- Root cause: the web host drew a frame for every input event with no limit on frames in flight;
  the desktop's presenter waits for its last present. On a slow adapter the GPU queue grew without
  bound: the screen showed what the mouse did seconds before, and the backlog (in the browser's
  shared GPU process) slowed the next page too.
- Fix: `webapp.rs` paces frames to the GPU: at most `IN_FLIGHT` = 2 out
  (`queue.on_submitted_work_done`); a frame asked for meanwhile is drawn when one is done (a
  second without a done callback draws anyway).
- Pacing made fewer frames, and two input bugs showed (both possible before on any long frame):
  two clicks in one frame (a tab, then the button its panel shows) lost the second, and a move
  after a release lost the click (egui takes a click where the pointer is at the frame's end).
  `inputcut::frame_cut` now ends a frame at a button release, and holds a press for one frame
  when the frame before changed the layout (it asked for another pass at once).
- Tests: `web/tests/pace.cjs` (old: 12.8 s), `inputcut` unit test, `history.cjs` (clicks a tab
  then its panel's button). `e2e.cjs` waits for one frame after a rebuild, not two (paced frames
  coalesce what's asked meanwhile; its pixel check shows the new part).
- Other numbers (after): open small 0.5 s, rack 2.5 s (threaded) / 16 s single-threaded (longest
  main-thread task 1.4 s single, 123 ms threaded; native: 8 s on 4 threads, 26 s on 1); orbit
  ~20 ms per input event before pacing, adapter-bound after; click to frame 5-25 ms.

## forge-render's full cargo test (lavapipe), ui/bughunt-2

All pass but five, and the same five fail on main 4846170a (none mine): ribbon tips "no tip for
tools,Machine Builder"; cam_ui `post_process_shows_the_program_and_writes_it`; camsim's three
flange tests; glyphs "drawn as boxes: '▸' shell/cam/mod.rs, '└' shell/machine.rs" (the same
list on main: this branch's new strings add none).

## 15. A clicked face was never drawn selected; an origin plane lit up instead (ui/bughunt-3)

- Symptom, desktop and page alike: clicking a face said "Picked: base.end" and selected it, but
  nothing showed it: once the pointer left, the frame was the one before the click; and with the
  pointer on the selected face the XZ origin plane behind it lit up green.
- Root causes (gui.rs): the frame's marks were filtered to `!m.selected`, so what was selected was
  never drawn; and the origin planes counted as hit wherever every mark was a selected one (a
  stand-in for "nothing of the part under the pointer"), which is so exactly when the face under
  the pointer is selected (its hover mark is left out then).
- Fix: selected marks drawn (the theme's chosen colour over the face); a plane is hit only where
  `nav.hover_hit()` says nothing of the part is under the pointer. Shots:
  `shots/face-pick-before.png`, `shots/face-pick-after.png` (native).
- Tests: `tests/selection.rs` (native, headless: the face's pixel warm after the pointer leaves,
  the plane's pixel unchanged with the pointer on the face; both fail before) and
  `web/tests/select.cjs` (the same in the page; fails before).
- Not done: an outline along the selected face's edges (the tint is the selection's mark now).

## Open (reported, not fixed here)


- 8. Memory kept by every forge-lang run (a reference cycle), natively too. A long session grows
  without bound: in the page, ~30 KB per edit of a 137-byte part (16 -> 63 MB over 1500
  SetParam edits alternating two values; idle and plain rebuilds stay flat). Natively
  `tools/ui-hunt/leak` (its own workspace): `forge_lang::run` of the same text keeps 5.5 KB per
  run (11 MB after 2000), `edit::apply_checked_in` 22 KB per edit (44 MB after 2000).
  Root cause: `eval.rs` `sketch_inner` stores the sketch block's names in `Sk.named`
  (`Vec<(String, Value)>`); a named entity is `Value::Ent(Ent { sk, .. })` whose `sk` is the same
  `Rc<RefCell<Sk>>`, so every sketch with a named entity is an Rc cycle and is never freed
  (massif: the retained heap is all `Sk::finish`'s sketch clones, profiles, `sketch_block`'s
  values). Confirmed: with `named` emptied the same 2000 runs hold 15 KB, flat.
  Not fixed here: `named` is read after the run (member access `plan.outline`, `sketch_view`,
  anchors) and values cross `Eval`s (used files, components), so the fix belongs in how `named`
  holds its entities (e.g. by id and loop, the `Ent` rebuilt from the sketch at access), and it
  is on forge-lang's run path, so the bench's app/edit/cli lines need the band (freeing each
  run's sketches adds deallocation work).

- 6. Agent tab in a page defaults to "Claude Code" and offers the microphone, both desktop only
  (docs/web.md); a send says "operation not supported on this platform". UX decision.
- 7. Versions under `forgecad web`: the page keeps `.git` in its memory, but the server refuses
  every dot path (by design, docs/web.md Security), so each version write logs `400 .git/...: not
  a project path` to the console only; the status says "History is on: every save is kept" and the
  history is gone on reload. Needs a decision: server allows `.git` writes, or the page says
  versions aren't kept on that backend.

## Swept, nothing found (this round)

In the page (single-threaded build unless said): opening parts and examples; sketching on a face
(Start Sketch, pick, Rectangle, Finish); SetParam edits, Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z, Ctrl+S
(saves, browser default suppressed); Versions turn on (after fix 3); Agent send (after fix 4);
Thermoform tab and Check Forming's panel (Stretch, Webbing, Draft, Simulated wall, probe, Play;
after fix 7); sheet metal Flat Pattern, DXF export (written beside the part on the machine);
with origin/bendsim/env merged locally: DFM, and the Environments tab's Bend Simulation, Flat
Pattern, Thermoform, Render Studio and Drawing (no panics); idle frames, hover picks; HiDPI
(hidpi.cjs) and first frame (firstframe.cjs) pass; idle/shade/nopanic pass on the threaded
artifact. Not yet: CAM, URDF/robot panel and `?env=KEY` (not on main), Assemble, Weldments,
Inspect/Measure, Drawing editing, Markup, file import (STEP/DWG) from the page's picker.
