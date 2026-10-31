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

| crate | what |
|---|---|
| `forge-math` | vectors, frames, deterministic trig, intervals, polynomial roots, small linear algebra |
| `forge-geom` | curves, surfaces, intersections |
| `forge-topo` | B-rep bodies, stable names, the validity checker |
| `forge-sketch` | 2D sketches, geometric constraint solver, profile extraction |
| `forge-ops` | profiles, extrude, revolve, sweep, coil, loft |
| `forge-mesh` | exact predicates, constrained Delaunay, tessellation, STL |
| `forge-model` | part histories and regenerating them |
| `forge-agent` | the `.fc` part file, selectors, measuring, rendering, incremental sessions |
| `forge-cli` | `forgecad build/query/measure/render/check/diff`, `serve`, `mcp` (see AGENTS.md) |

## Building

```
cargo test --workspace                          # native
cargo test --workspace --target wasm32-wasip1   # wasm, run under Node's WASI
```

The toolchain is pinned in `rust-toolchain.toml`; rustup installs it on first use.

## License

MPL-2.0.
