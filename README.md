# ForgeCAD

An open source parametric CAD tool, written from the ground up in Rust: its own
geometry kernel, a native and a browser client, and a self-hostable server.

## Kernel

Shapes are stored exactly: a cylinder is an axis and a radius, a circle a frame
and a radius. No NURBS; surfaces that have no closed form will be defined by how
they are built (a fillet is a ball rolling between two faces) and evaluated
numerically.

Every result is bit-identical on every target. The kernel uses only IEEE
`+ - * / sqrt` and its own trigonometry (`forge_math::dmath`), never the
platform math library, and CI runs the whole test suite natively and as
wasm32 to prove it.

Units are millimetres. Tolerances live in `forge_math::tol`.

The repository is split by what a thing is:

- `kernel/` is the geometry kernel: exact results, zero third-party crates.
- `interface/` is everything that uses the kernel: part files, the agent layer and the command
  line. The app, host and protocol go here too ([design](docs/interface-design.md)).
- `verify/` is what checks the kernel and is never shipped.
- `research/` records how each derived formula and constant was reached, and `tools/` holds the
  speed gate, the OpenCascade oracle and test runners.

| `kernel/` | what |
|---|---|
| `forge-alloc` | the wasm heap |
| `forge-math` | vectors, frames, deterministic trig, intervals, polynomial roots, small linear algebra |
| `forge-geom` | curves, surfaces, intersections |
| `forge-topo` | B-rep bodies, stable names, the validity checker |
| `forge-sketch` | 2D sketches, geometric constraint solver, profile extraction |
| `forge-ops` | profiles, extrude, revolve, sweep, coil, loft, fillets, chamfers, shell, draft |
| `forge-bool` | booleans, sequential and n-ary |
| `forge-gpu` | the GPU batch backend |
| `forge-font` | glyph outlines for text features |
| `forge-mesh` | exact predicates, constrained Delaunay, tessellation, STL |
| `forge-model` | part histories and regenerating them |

| `interface/` | what |
|---|---|
| `forge-doc` | the document model behind part files, its ops and their binary form |
| `forge-agent` | the `.fc` part file, the tools, selectors, measuring, rendering, incremental sessions, the live loop |
| `forge-cli` | `forgecad act/query/measure/render/help`, `build/check/diff`, `serve`, `mcp` (see AGENTS.md) |
| `forge-render` | the renderer, wgpu native and in the browser; its own workspace, built apart from the kernel |

| `verify/` | what |
|---|---|
| `forge-proof` | metamorphic properties, certificates, point membership against the CSG tree |
| `forge-bench` | benchmarks |

## Building

```
cargo test --workspace                              # native
cargo test --workspace --target wasm32-wasip1       # wasm, run under Node's WASI
cargo build --profile wasm --target wasm32-wasip1   # the browser build: fat LTO, simd128
tools/speed/gate.sh                                 # native instruction counts and bits
tools/speed/wasm.sh                                 # wasm instruction counts, ratio to native
```

wasm is single-threaded (wasm32-wasip1 has no threads), so the n-ary regen runs on one thread
there.

The toolchain is pinned in `rust-toolchain.toml`; rustup installs it on first use.

`interface/forge-render` is its own workspace, so wgpu never enters the commands above. From there:

```
cargo run --release -- gearbox                      # a bench part or a .fc path (`--help` lists keys and mouse)
cargo run --release -- --stress 5000                # the 15 bench parts placed 5,000 times
cargo run --release -- --stress 5000 --bench        # a fixed orbit, frame times on stdout, no window (--window: with one)
cargo run --release -- crank --dims                 # the numbers a part has
cargo run --release -- crank --drag len             # left drag changes one live, release commits it
cargo run --release -- --drag-bench                 # value change to frame, each bench part, no window
cargo test                                          # GPU tests skip where there is no GPU
sh web/build.sh OUT                                 # the browser page and the bench parts' meshes
```

Built meshes are cached in `~/.cache/forgecad/mesh`, by content hash of the part file and the
kernel's sources. Nothing else reads that cache; kernel benchmarks never see it.

## License

MPL-2.0.
