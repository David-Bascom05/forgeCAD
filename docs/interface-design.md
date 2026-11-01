# Interface design

The kernel can build real parts. This is the plan for what sits on top of it: the app, the
browser client, the host and the protocol between them. Status: agreed design; the renderer
(`interface/forge-render`), the tools and op log (`forge_agent::tools`, `forge_doc::wire`) and the live
loop (`forge_agent::live`, `forge_render::live`) are built.

The one rule: **no lag.** Where a choice trades anything against speed, speed wins. If an
existing tool is too slow, we write our own, as we did with the kernel.

## How it runs

The same program runs in three roles:

- **App, hosting:** the desktop app is client and server at once. Its UI calls the kernel
  in-process, with no protocol and no serialization. Other clients can connect to it.
- **App or browser, client:** connects to a host somewhere else.
- **Headless host:** `forgecad host` serves projects with no window, for example on node1. It
  is the same host code as the app's.

A browser with no connection runs the kernel itself as wasm. It does this only when it isn't
connected to a host.

Clients and hosts may be on the same machine, on one LAN, or across the internet. All three
are supported the same way.

## Who computes what

Every capable client rebuilds the part itself, so an edit never waits on the network. The host
is the authority: it orders edits and stores the log. Native and wasm results are bit-identical
(CI checks this), so a client's result equals the host's and reconciling is a hash comparison.

Geometry never goes over the wire, only edits. Sending finished shapes to devices too weak to
rebuild (old phones, huge parts) stays possible later: nothing in the protocol may assume every
client rebuilds. Not now.

## Tools

The UI and agents use one command layer: extrude, fillet, select, measure, place in assembly,
and so on.

- A button press is a tool call. The UI has no private path to the document.
- A tool produces document ops, which are what sync and what the log records.
- Agents get the same tools, plus extras that make them cheaper to use: batching, selectors,
  compact answers.
- Anything one can do, the other can. Every UI action can be replayed as tool calls in a test.

Agents join a live session as ordinary clients, so you watch their edits appear. Talking to an
agent from inside the UI comes later.

As built: `tools::Call` is the typed call; `Part::call` commits calls, `Part::act` commits the same
calls as text lines, all or none. A commit appends one `wire::Entry` to `PART.fc.log`, the ops by
stable id (with what undoes them, the file's hash, its face count and volume) and rewrites the `.fc`
file; each op prints back as a text line. The first entry holds the file's text, as does an entry for
a hand edit, where undo stops. The agent surface is five MCP tools (`act query measure render help`),
replies are diffs, and the grammar is behind `help TOPIC`, so AGENTS.md stays short.

## Live loop

A worker thread owns the part. The UI thread swaps the newest value into a slot and takes what is
ready, and so never waits on the kernel; a value overtaken before it started is dropped. The worker
rebuilds from the changed feature and meshes only the faces whose key changed (their name, surface
and boundary: `facemesh::key`), at a level coarse enough for a drag. Each face has its own span of the
GPU buffers, so an edit writes only its faces. Releasing commits one line and brings the full mesh back.
`forge-render --drag-bench` is the 16 ms benchmark: value set to frame done, for each bench part.

## Protocol

Our own compact binary format.

- **Edits** are ops keyed by stable feature id: "feature 17, parameter 2 = 12.5" is a few bytes.
- **Transport:** raw QUIC between native peers, WebTransport (the same QUIC) from browsers,
  WebSocket only where QUIC is blocked.
- **Live values** while dragging go as unreliable datagrams: a lost one is superseded by the next.
- **Committed edits** go on reliable ordered streams.
- **Co-editing** is live, Google Docs style, over an append-only op log.
- **Identity:** the protocol carries an identity slot. Accounts are not designed yet.

## Speed targets

- **Live drag:** the part rebuilds on every frame while you drag, skipping any rebuild already
  stale. 16 ms (60 fps) per rebuild is the kernel's real-world benchmark, and this will show
  which parts are too slow.
- **Big assemblies:** 5,000 parts, some of them complex. The first open may take a few seconds;
  every open after that is instant.
- **Cache:** built results are stored by content hash of the part and kernel version. A part
  rebuilds only when edited or when its cache entry is missing.
- **Kernel benchmarks never use the cache.** The speed gate measures the kernel itself.
- **Hardware:** this laptop's NVIDIA GPU first. Integrated graphics is the stretch goal. CUDA is
  allowed wherever it helps.

## Rendering

One Rust renderer on wgpu: native on the GPU in the app, compiled to wasm on WebGPU in the
browser. There is no webview and no JavaScript in the drawing path, chrome included.

The app shell is winit plus wgpu, with the kernel running natively in-process. Chrome and viewport
are drawn into one frame, so input reaches the pixels in one frame, and nothing is drawn when nothing
moves. The chrome is egui (with egui-wgpu and egui-winit), restyled to Commonty's colours and type,
with our own ribbon and tree widgets; the viewport, view cube and dimension arrows stay ours. The
browser runs the same UI on one canvas (WebGPU, wasm), so `forge_render::shell` knows nothing of the
window or the worker: it reads a document and says what was asked.

Why these (David, 2026-10-31), measured on the bracket under load on the laptop's RTX 5060:

```
option                 crates added   release build   fit
egui 0.37                  35           +121 s        wgpu 30, winit 0.30; all of the below
cosmic-text + glyphon      30           +41 s         text only: widgets, menus, IME all ours
iced 0.14                  61           +112 s        needs wgpu 27: a second wgpu
slint 1.18                225           +231 s        its own language, an async stack
```

The added crates are over forge-render's 95. Click to GPU done is 0.7 ms at the median, 2.6 ms p95; an
orbit frame with chrome and viewport 0.7 ms, 2.5 ms p95; idle asks for no frame at all. egui shapes
with harfrust, so kerning is real. System clipboard through smithay-clipboard (3 crates); file dialogs
through rfd's portal (2 crates, no async runtime). AccessKit stays an optional feature: egui's widgets
and ours report their role and label from the start.

Why not a webview over wgpu: WebKitGTK has no WebGPU, a frame shipped into a page takes 30 ms, and web
chrome over a wgpu subsurface can't promise one frame for both.

## Storage

- Parts stay plain `.fc` text files: the most token-efficient form for agents, and readable and
  diffable.
- Speed comes from binary files beside them, the op log and the build cache, which agents never
  need to read.
- The op log being append-only keeps every history option open (undo across sessions, scrubbing,
  versions, branches). How versions look in the UI is not decided.

## UI

An Inventor-style layout: ribbon, browser tree, viewport, in Commonty's colours and type (Public Sans,
IBM Plex Mono, bundled in `forge-render/fonts`). It holds to desktop standards, not a game UI's: text
selection, caret, clipboard, IME, keyboard focus, tooltips, kinetic scrolling, context menus, fractional
scales, system file dialogs. The `.fc` text stays in sync but isn't
shown; it is for agents and files. Assemblies are in from the start and get optimized later.

## Dependencies

The kernel stays at zero third-party crates. The interface may use a limited set of vetted ones
(wgpu, winit, a QUIC library), each pinned, and each replaced by our own if it can't hit the
speed targets.

## Milestone 1: viewer and protocol

No modeling UI yet. A really good viewer, on all four of:

- **Speed:** 5,000 parts at 60+ fps while orbiting, with instancing, culling, level of detail
  and streaming.
- **Visual quality:** crisp edges and silhouettes, good shading and materials, ambient
  occlusion, anti-aliasing.
- **Precise interaction:** exact picking of faces, edges and vertices, hover highlight, measure,
  section views, explode.
- **Navigation:** Inventor-style orbit, pan and zoom, view cube, zoom to fit or selection,
  smooth animated transitions.

Plus the protocol carrying real edits between clients, with co-editing.

## Decided later

- Accounts and sign-in: one central service or per host.
- How versions and branches appear in the UI.
- Talking to agents in the UI.
- Sending finished shapes to weak devices.

## Open questions

- Co-editing conflicts: two people changing the same feature at once.
- A pending kernel question: fuzz seed 3, where one n-ary entry point returns success without
  the fillet while the other errors. That needs naming against the original tool faces.
