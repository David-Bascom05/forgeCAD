# ForgeCAD in a browser

The browser build is the desktop app: the same `Gui` (ribbon, tree, sketcher, every feature tool,
Measure, the issues panel, assemblies, sheet metal, drawings, versions), and the kernel and
forge-lang compiled to wasm in the page. A part builds to the same bits as natively: the same
volume, faces and errors (`web/tests/parity.cjs` checks the bench parts and every example).

It needs a browser with WebGPU (Chrome or Edge 113+, Safari 26+, Firefox 141+ on Windows).

## Build

From `interface/forge-render`, with the `wasm32-unknown-unknown` target and `wasm-bindgen-cli`
0.2.130 (`cargo install wasm-bindgen-cli --version 0.2.130 --locked`):

```
sh web/build.sh OUT                # the app, the examples, and the viewer's bench meshes
NO_MESHES=1 sh web/build.sh OUT    # without the viewer's meshes (no native build needed)
```

`OUT` is plain static files: `index.html`, `app.js`, `storage.js`, `zip.js`, the wasm and its
JS glue, and `examples/`.

## Run it on a machine's files: `forgecad web`

```
forgecad web DIR [--port P] [--bind ADDR] [--token T] [--app OUT] [--read-only]
```

serves the app (from `--app OUT`, `$FORGECAD_WEB`, or `share/forgecad/web` beside the installed
binary) and a file API over `DIR`. Open the URL it prints: the editor works on the files of `DIR`,
saved back to that machine as you save. It listens on `127.0.0.1:8080` by default.

The API, for scripts too (`/api/...`, JSON):

| call | what |
|---|---|
| `GET /api/info` | `{"server":"forgecad web","root":NAME,"writable":B}` |
| `GET /api/files` | `{"files":[{"path","size","etag"}]}` (no dot files or folders) |
| `GET /api/files/PATH` | the bytes, with an `ETag` |
| `PUT /api/files/PATH` | the bytes, with `If-Match: ETAG` (the version it was made from) or `If-None-Match: *` (a new file); `{"etag"}` |
| `DELETE /api/files/PATH` | with `If-Match: ETAG` |
| `POST /api/rename` | `{"from","to"}`; refused if `to` exists |

An ETag is the hash `forgecad edit --base` takes as a file's version. A write made against an
ETag that is no longer the file's is refused with `412 {"error":"StaleBase","etag":CURRENT}`, so
two editors (two tabs, a tab and an agent, a tab and your text editor) never overwrite each other
unseen; the browser says so and keeps your edit in the tab. A write with no precondition is
refused with 428.

### Security

- It binds to `127.0.0.1` unless `--bind` says otherwise. On loopback it answers only requests
  whose `Host` is a loopback name, so a web page can't reach it through DNS rebinding, and the API
  refuses requests whose `Origin` is not its own.
- Any other bind requires a token: `--token T` (16 characters at least), or one it makes and
  prints in the URL. The page sends it as `Authorization: Bearer T` (it takes `?token=T` from the
  URL once and keeps it for the tab). Use HTTPS in front of it (a reverse proxy) anywhere but a
  trusted network: the token travels with every request.
- Nothing outside `DIR` is reachable: paths with `..`, absolute paths, backslashes, colons and
  empty segments are refused; a link that leads out of `DIR` is not followed; dot files and dot
  folders (`.git`) are neither listed nor served.
- `--read-only` refuses every write.
- Writes are atomic (a file beside it, renamed over it) and one at a time.

## Run it from static files

Serve `OUT` from any static host (a CDN, GitHub Pages, `python3 -m http.server`). With no
`forgecad web` behind it, projects are kept in the browser: the Origin Private File System, or
IndexedDB where there is none. File > Open lists the project's files and takes files, a folder or
a `.zip` from the computer; "Export the project" downloads the project as a `.zip`. A new project
starts with the examples. `?project=NAME` keeps several projects apart.

Serve `.wasm` as `application/wasm`. To allow the threaded build later, also send
`Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`
(`forgecad web` does).

## How it fits together

- `forge_dirs::fs` is the filesystem every crate reads and writes through. Natively (and on WASI)
  it is `std::fs`, re-exported: nothing changes there. In the browser it is an in-memory tree
  (`forge_dirs::fs::mem`) the page fills from the project's storage and that tells the page of
  every change, which `web/storage.js` keeps where the project lives. So assemblies, fonts and
  imported files beside a part, the undo log, and versions all work as on the desktop.
- `forge_math::clock` is the clock (std's natively; the page's `performance.now()` and
  `Date.now()` in the browser, where std's panics). Timings only: nothing built depends on it.
- `forge-render/src/plat.rs` holds what else differs: background work (a thread natively; run in
  turn in the page), file pickers, small values kept between sessions (settings, an endpoint's
  key: `localStorage` in the browser).
- `webapp.rs` is the page's host: winit's canvas, its events to egui (`webinput.rs`), and
  `Gui::frame`, as `app.rs` is the desktop's.

## What differs from the desktop

- Two builds of the app. The default runs the kernel on the page's thread, between frames: a long
  rebuild holds the page for its length. The threaded artifact (`THREADS=1 sh web/build.sh OUT`,
  into `OUT/threads`) runs each document's build on a Web Worker sharing the page's memory, so the
  page never blocks for a build; the page loads it by itself where it is cross-origin isolated
  (`forgecad web` sends the headers; a static host must send
  `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`), and
  `?threads=0` opts out. It is the same source and the same pinned stable toolchain with its
  `rust-src`, std rebuilt with atomics (`-Zbuild-std`, enabled by `RUSTC_BOOTSTRAP` for that build
  only); `web/build.sh` holds the exact flags. Both give the same bits: the parity tests run on
  each. In it the page's main thread never waits on a lock (a wait there traps): the locks it
  shares with the workers spin (`plat::lock`, `forge_dirs::fs`, forge-alloc's heap), kernel work
  runs on workers, and the WebGPU objects stay on the page's thread (the worker hands back a view's
  meshes and scene). The kernel's own parallel paths (n-ary booleans, `forge_math::par`) still run
  on one thread there: std can't start a thread in a browser, so they take their one-thread path,
  as on WASI.
- The Agent tab: the Claude and Codex agents run their CLIs, and the MCP bridge a socket: desktop
  only. OpenAI-compatible endpoints are not yet reached from the page.
- Speech input: desktop only.
- Folder watching: the page's files change only through the app (and `forgecad web`'s writes are
  checked against their versions instead).

## Tests

From `interface/forge-render`, with `OUT` built and `forgecad` and `forge-parity` built release:

```
cargo build --release --bin forge-parity && (cd ../.. && cargo build --release -p forge-cli)
node web/tests/e2e.cjs OUT        # open, edit a parameter, rebuild, save, StaleBase; volume = native
node web/tests/parity.cjs OUT     # bench parts and examples: page vs native, bit for bit
node web/tests/responsive.cjs OUT # (THREADS=1) the threaded page never blocks for a build
THREADED=1 sh web/tests/ci.sh     # (from the root) all of it on the threaded artifact
cargo test -p forge-cli web::     # (from the root) forgecad web's API
```

The browser tests use the Playwright that is installed (`require("playwright")`) and Chromium with
a software WebGPU adapter (SwiftShader), so they run without a GPU.
