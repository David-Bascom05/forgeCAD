# Interface design

The kernel can build real parts. This is the plan for what sits on top of it: the app, the
browser client, the host and the protocol between them. Status: agreed design; only the renderer is built
(`interface/forge-render`).

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
browser. There is no webview or JavaScript in the drawing path. Only the UI chrome around the
3D view is web code.

The app shell is Tauri 2, with that web UI shared between app and browser and the kernel running
natively in-process.

## Storage

- Parts stay plain `.fc` text files: the most token-efficient form for agents, and readable and
  diffable.
- Speed comes from binary files beside them, the op log and the build cache, which agents never
  need to read.
- The op log being append-only keeps every history option open (undo across sessions, scrubbing,
  versions, branches). How versions look in the UI is not decided.

## UI

An Inventor-style layout: ribbon, browser tree, viewport. The `.fc` text stays in sync but isn't
shown; it is for agents and files. Assemblies are in from the start and get optimized later.

## Dependencies

The kernel stays at zero third-party crates. The interface may use a limited set of vetted ones
(wgpu, Tauri, a QUIC library), each pinned, and each replaced by our own if it can't hit the
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
