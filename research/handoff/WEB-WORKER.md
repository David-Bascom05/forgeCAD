# WEB-WORKER handoff: the default browser build's build worker

Branch `hosted/web-worker`. It merges main c135b274 and fix/asm-shape-first 1af8e6da. Its own
commits:

| sha | what |
|---|---|
| a3392591, e3c0be4b | `wire`: the build's data as bytes (field-complete `record!`/`choice!` codec, round-trip tests on real example builds) |
| e0d16d72 | `fg::remote`: the build Worker in a dedicated Web Worker with its own wasm instance (`web/build.js`), the page↔worker protocol, file replication, the view made on the page; Inspect by `Brief` and asks; `Built.hash` stub for hosted |
| 5dff812f | the thermoform view's kernel half kept in the worker (sim, heat map, exports by ask); Measure's same-kind picks resolved on the page at once; `app_build_bench` |
| b26798c3 | behind `?worker=1` (default stays in-page); tests: `buildworker.cjs`, `responsive.cjs` on the default artifact, `FORGE_WORKER=0|1` for every web test |

## State

Done and tested; not the default. `?worker=1` turns it on, and the in-page path is unchanged
behind the default (`?worker=0`).

Measured in headless Chromium (swiftshader WebGPU, 4-core container):

- **Page freeze.** rack18u `asm/rack.fg`, Analyze Interference on, then `y0_front` 50→51.
  On the page's own thread the longest task was 9.9 s and the longest frame gap 9.9 s. On the
  build worker the longest page task was 0 ms (none over Chrome's 50 ms line) and the longest
  frame gap 33 ms.
- **Same build.** The built state is identical both ways (volume and area bits, faces, issues).
- **Build time.** About 1 s slower on the worker (10.7 s against 9.9 s on the rack). Part of that
  is meshing and encoding the assembly's whole mesh set (one 22.9 MB message).
- **Crossing cost per mesh message** (write in wasm, copy out, transfer, copy in, read):
  - 10.7 KB: 0.05 ms total.
  - 3.9 MB: 10.4 ms total (write 3.5, copy out 1.6, transfer 0.02, copy in 1.6, read 3.8).
    The page's share is about 5.3 ms.
- **Cross-document delay.** One build worker serves every open document, so they queue. A small
  part's edit took 282 ms alone and 9.7 s behind another document's rack check.
- **Web tests.** All 17 of ci.sh's web tests pass with `FORGE_WORKER=1` and with
  `FORGE_WORKER=0` (parity: 291 of 291 bit-identical to native, both ways).
- **Native tests.** The measure, inspect, wall_panel, pick and selection tests pass on lavapipe,
  as do the codec round trips (`cargo test --lib wire remote`).

## What remains before it can be the default

1. **UX-LEAD's asks for waiting picks.** A filtered Measure pick of another kind, the section
   plane and face-from-edge can wait on the worker. Past about 300 ms the status line should show
   the shared spinner (`ux::loading`) and "Waiting for the build to take the pick", and Esc should
   drop the waiting pick. The export status should show the spinner too. UX-LEAD wants shots at
   1400 px, dark, worker mode: a waiting pick (force a long check), an export in progress, and the
   Project Geometry refusal with its hover text ("To project it here, open the page with
   ?worker=0 (slower: builds block the page)"; not done yet). Today a waiting click is retried
   each frame until answered, with no visible state.
2. **Project Geometry of a blend's or an intersection's edge** is refused on the worker.
   `Curve::Procedural` and `Curve::Intersection` are `Arc<dyn …>` and can't cross. Fixing it needs
   either a kernel-side serializable form, or flattening in the worker (the sketch re-flattens
   with an `approx` the user can change, so the worker would have to be asked again then).
3. **Cross-document delay.** Use a small pool (2–3 instances, documents assigned round-robin) or
   one worker per document; each instance costs its wasm memory.
4. **Interleaving during a long check.** A query (a refining pick, Measure) waits for the running
   pass to finish. The worker yields between an assembly's shape pass and its checks pass
   (fix/asm-shape-first), but not inside the check. Stepping the asm checker pair by pair would
   let queries in.
5. **Cancel (KERNEL-STRESS, `op-cancel`).** In the unshared worker a stop can only land at a pass
   boundary. When op-cancel merges, add `Msg::Cancelled` to the codec (it won't compile until you
   do), pass the `Running` slot to `Worker::new` in `fg::remote::worker`, and post `(secs,
   spent/OP)` progress as a `ToPage` message after each step.
6. **Hosted (HOSTED-BUILD).** `fg::build_hash` returns None. When forge-sync merges, make it call
   `forge_sync::hash_body` / `hash_placed`. The WebSocket and the sync client stay on the page.
7. **The thermo worker path is untested in a browser.** The simulated part, the wall's heat map
   and the kernel exports as asks of the worker compile, and their functions are the in-page ones,
   but no web test opens a thermoform with `?worker=1`. Add one (examples/thermoform/tray.fg: show
   Simulated, the Thickness analysis, export the mould's STEP) before making the worker the default.
8. **The agent's `Gui::measure` tool** refuses in worker mode (it's synchronous; only native tests
   use it).

## Page-side kernel work still on the page's thread (out of scope; the next follow-up)

These run through `plat::spawn`/`serve`, which in the single-threaded build runs the job
immediately, on the page's thread. Each holds the page for as long as the job takes:

- **Whole document builds** (as long as a build of the document: tens of ms to seconds; the rack
  is about 10 s):
  - `gui_thermo.rs` `thermo_host` and `thermo_asm` (a thermoform part's runs; in an assembly, the
    selected part's);
  - `gui_versions.rs` `vers_spawn` (two versions built to compare);
  - `shell/drawing.rs` `regen` (a drawing's views);
  - `gui_inspect.rs` the section outline (`query::ask … section`);
  - `gui_asm.rs` `asm_act` / `asm_find_snaps`, and `gui.rs` `asm_anchors` (part files built for
    their anchors and snaps);
  - `gui_markup.rs` `markup_names`;
  - `gui_overlay.rs` `ask_extras`;
  - `gui.rs` `take_fg`.
- **CAM** (CAM-UI's): `gui_cam.rs` `cam_host` (3 sites: the CAM run, its quality measure, the
  adaptive's alternative, Check Now), and `gui_camsim.rs` `camsim_follow`. These can take
  seconds on a real part.
- **Meshing and loading:** `live.rs` `mesh_all` (`par_map`) and `load.rs` `open_all` (the viewer's
  part files), `measure.rs` `start`.
- **Thermo, local path only:** `thermo_view.rs` `want_sim` / `want_heat`. These go to the worker
  when `?worker=1`.
- **Not kernel work:** `agent/openai.rs` `send` (network), `gui_agent.rs` `agent_keyring`,
  `gui_speech.rs`, `shell/vendorui.rs` (2), and `webapp.rs` `build_start` (the parity test's).

On `?worker=1`, Locate's interference marks and the wall search moved to the worker; the outline
did not.

## Risks

- **The codec is field-complete by construction.** A field added to any type that crosses fails
  to compile until it's written. That's deliberate, but every co-editor of `Built`'s types will
  hit it. The fix is one line in `src/wire/types.rs` or `src/fg/remote.rs` (a `record!` naming
  the new field).
- **`Frame` crosses by its exact axes** (`Frame::from_axes`, orthonormal to 1e-12). A frame that
  drifted past that would fail to decode (the whole message is then dropped and logged). None has
  in the tests.
- **File replication.** The page forwards every change and load of its in-memory filesystem to the
  worker before the work that may read it, and the worker's writes come back to be kept. A write
  path that bypasses `forge_dirs::fs` on the page would leave the worker stale.
