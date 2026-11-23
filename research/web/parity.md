# Web parity: what the app does in a browser today

Audited 2026-11-22 against `claude/repo-review-mbdrlj` (8250ebb). The goal is the desktop app
unchanged in a browser, or served from a machine by `forgecad web DIR`.

## How the app is built today

| layer | what | wasm32-unknown-unknown today |
|---|---|---|
| kernel crates, forge-lang | the whole build: features, sketches, assemblies, sheet metal, thermoform, drawings, STEP, fonts | compiles; the workspace's tests run on wasm32-wasip1 with bit-identical results (CI) |
| `shell/` (forge-render) | the chrome: ribbon, tree, panels, dialogs, sketcher UI, measure UI, issues, settings, versions, agent panel, DWG document | compiles (it is plain egui reading a document and emitting `Action`s) |
| `gui.rs` + `gui_*.rs`, `fg.rs`, `live.rs`, `tess.rs`, `measure.rs`, `thermo_view.rs`, `load.rs`, `cache.rs`, `agent/`, `speech/` | the host: turns `Action`s into tool calls, runs the build worker, meshes, measures, keeps files | **native only** (`#[cfg(not(target_arch = "wasm32"))]` in lib.rs) |
| `web.rs` | a viewer: prebuilt `.mesh` files on a WebGPU canvas | wasm only |
| `webdwg.rs` | the DWG document editor: the shell's DWG layout, egui fed from winit's events | wasm only |

So the browser has the shared chrome compiled in but nothing hosting it except for DWG files. The
host's native-only dependencies are few and mechanical:

- `std::thread` (build worker, whole-mesh thread, background jobs in inspect, docs, versions,
  markup, thermo, agent): no threads on wasm32-unknown-unknown with the stable toolchain (1.99.1;
  shared-memory threads need `-Zbuild-std` with `+atomics`).
- `std::fs` (open/save, settings, the mesh cache, the agent's project files, forge-lang's loaders
  for parts, fonts, imports, libraries, forge-git's object store): every call fails on
  wasm32-unknown-unknown.
- `std::time::Instant`: panics on wasm32-unknown-unknown (`web-time` is already in the tree via
  winit).
- `rfd` file dialogs, `ureq` HTTP, `keyring` stores, child processes (Claude/Codex CLIs, speech):
  native only by nature.
- `egui-winit`: native only; `webdwg.rs` already has its own winit→egui translation.

## Feature by feature

| feature | web today | what it takes |
|---|---|---|
| open / edit / save .fg | **no** (viewer of prebuilt meshes only) | the host on wasm; a storage layer for files |
| live rebuild | **no** | the build worker on wasm (in a Web Worker so the UI never blocks) |
| sketcher | **no** (UI compiled, no host) | the host |
| every feature tool (extrude … rib, pattern, bodies) | **no** | the host; the tools are shell code + forge-lang edits, both compile |
| measure | **no** | `measure.rs` (pure kernel calls) compiled for wasm |
| issues panel | **no** | the host (rows come from the worker) |
| assemblies | **no** | the host; forge-lang's part loader needs the storage layer (it reads sibling files with `std::fs`) |
| sheet metal (flat, DXF) | **no** | the host; DXF is a string, saved via the storage layer / download |
| drawings | **no** for .fg drawings; | the host |
| DWG | **yes**: open, edit, save as download (`?dwg`) | already works; move it under the same storage |
| DXF export | **no** | the host + storage |
| STEP import | **no** | forge-step compiles for wasm; needs the storage layer to read the file |
| STEP export | **no** | forge-step compiles; save via storage / download |
| fonts (text features) | **no** | forge-lang reads `.ttf/.otf` beside the part: the storage layer; UI fonts are embedded already |
| materials | data **yes** (embedded `materials.toml`), UI no | the host |
| standard parts library | data **yes** (embedded CSVs in forge-lang) | the host |
| vendor profiles (sheet DFM) | not landed yet (claude/repo-review-mbdrlj-sheet-vendors) | bundle like materials when it lands |
| settings | **no** (`load`/`save` are native only: a TOML in the config dir) | storage: a settings key in OPFS/IndexedDB/localStorage |
| undo / redo | **no** (forge-lang's undo log lives on disk beside the file) | in-memory undo works; the on-disk log via the storage layer |
| history / versions (forge-git) | **no** | forge-git is pure Rust over `std::fs`: the storage layer gives it a filesystem |
| agent / chat | **no** | OpenAI-compatible endpoints over `fetch` (CORS permitting, key in the page's storage); the Claude/Codex CLI agents and the MCP bridge spawn processes: desktop only, or through `forgecad web` |
| speech | **no** | a native process; out of scope (could be Web Speech later) |
| thermoform | **no** | the host; forge-thermo is pure Rust |
| mesh export (landing from claude/repo-review-mbdrlj-mesh-export) | n/a | through the storage layer / download once it lands |

## Plan

1. **A platform layer** in forge-render (`plat`): `spawn` (a thread natively; on the web, the job
   runs where the page can run it), `Instant` (std natively, `web-time` on the web), file dialogs
   (rfd natively; the page's picker on the web), and `Storage`.
2. **The host on wasm**: compile `gui.rs` and everything it needs for wasm32, driven from a winit
   canvas with the winit→egui translation `webdwg.rs` already has. `Gui::frame` is the same on both.
3. **The build off the UI thread.** The build worker's loop is split into a step (one batch of jobs
   in, messages out) that the native thread loops on unchanged. On the web the step runs in a Web
   Worker holding its own instance of the same wasm; its messages are encoded (meshes already have
   a byte format; rows, issues, measures, anchors are plain data). Without a worker (or before it
   is ready) the step runs on the page between frames: the same bits, since the kernel is
   deterministic. Shared-memory threads are not available on the pinned stable toolchain, so the
   threaded build is not the default; see the notes in `README.md` here once measured.
4. **Storage**: one `Storage` trait used by the host on both targets: the local filesystem
   natively; OPFS (falling back to IndexedDB) in the browser; or `forgecad web DIR`'s HTTP API with
   an ETag per file (a write against a stale ETag is refused as `StaleBase`). forge-lang's loaders
   read through a virtual filesystem the host fills from storage on the web, `std::fs` natively
   (no change, no cost there).
5. **`forgecad web DIR [--port P] [--bind ADDR]`** in forge-cli: static files + the file API;
   localhost by default; a token required for any other bind; paths confined to DIR.
6. **Tests**: Playwright end to end (load, open, edit a parameter, rebuild, volume equals native),
   parity over the bench parts and examples (volume and face counts exact), an API test of
   `forgecad web`; a CI job for all of it.
