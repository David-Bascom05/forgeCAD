# Fundamentals: where the work stopped (paused 2026-11-23)

Paused by the owner's ruling: prove only what active features need, not the whole kernel. Branch
`claude/repo-review-mbdrlj-fundamentals`; head at the pause: the commit adding this file (after
03c2da0). Accepted by the master through 9ce9c83. The proof levels are research/PROOF-STANDARD.md's.

## The validity check (`check_known`, kernel/forge-topo/src/check.rs)

L1 and L3: `check_sound` (verify/forge-topo-check/lean/TopoCheck/Proof.lean), `check` returning
no problem implies `Valid` (Spec.lean). L6: the differential test, 1159 kernel bodies
(verify/forge-topo-check/tests/diff.rs).

**At L4** (verify/aeneas/forge-topo, Aeneas translation of the shipped code refined to the model;
`tools/aeneas/translate.sh forge-topo [--check]`, `cd lean && lake build`):

| test | Rust | theorem |
|---|---|---|
| stray vertices | `topo::strays` | `forge_topo.strays_refines` |
| back-links | `topo::links` | `forge_topo.links_refines` |
| shells: lists, joined | `topo_shell::shell_faces` | `forge_topo.shell_faces_refines` |
| edge uses and senses | `topo_edges::edges` | `forge_topo.edges_refines` |
| faces without loops, empty and broken loops | `topo_edges::loops` | `forge_topo.loops_refines` |

How it is built, for the next test: the tests read a body through traits (`Topo`, with
supertraits `ShellTopo`, `EdgeTopo`) implemented by check.rs's `View` (the body's arrays as
slices: through `&Body` the compiler reloads lengths, Body holding cells) and write through a
`Sink` (`Vec<Problem>`). Each new test goes in a module of its own (rustc places generic code by
module: added to topo.rs, `links`'s code moved). Keep allocations the sizes the old code made
(a u8 or usize vector where the old was u32 moved malloc's size classes and cost thousands of
instructions). The speed gate is exact callgrind; lines that rise with no forge function rising
(libc start-up, the loader) pass the master's layout rule with a per-function diff
(callgrind_annotate --inclusive=no, base and new built side by side).

The proofs' hypotheses: `Reads` (counts and fields by index, every entity alive: `check` looks at
compact bodies), `ShellReads` (shell ids below u32::MAX, face ids u32), `EdgeReads` (coedge senses;
every loop's coedges and their edges in range, leftover loops too), `checkLinks = []` for every
test after `links` (check_known returns after `check_links` otherwise). Model change made for the
shells: the flood pushes each neighbour once, fuel `faces.size + 1`, and is proven complete
(TopoCheck/Flood.lean, `reached_iff`).

**Left** (not started):
- `check_shell_euler`: V − E + 2F − L per shell by stamps (no sort). The model is `shellValue`
  with `distinct` (eraseDups); the refinement needs distinct-count by stamping = eraseDups length.
- `check_fans`: the vertex fans, with the corner table from geometry (`Body::corners`, angles in
  floats) entering as an opaque view method, as `edge_degenerate` does; the guard (`closesOk`)
  then is topology and translates.
- Then the whole of `check_known`'s combinatorial part composed (`Model.check`), and with the flat
  boolean's end-to-end theorem and decide-once switched on, the post-build check leaves release
  (research/PROOF-STANDARD.md, "The validity check after a proven operation").

## The rest of the fundamentals list

| item | state | next |
|---|---|---|
| Mass properties (forge-topo mass.rs) | L1: rules exact; error bounds proven (`centroid_error_within_err`, `inertia_entry_bound`, `face_sum_err` derived γ_n rounding); fixes merged | the in-face rounding allowance (1e-15) is a rule of thumb: derive it via the f64 bridge (verify/aeneas/bridge); then L4 |
| Rigid transforms and patterns | L1 L3 (verify/forge-transform) | L4 |
| Tessellation | L1: closed, oriented (verify/forge-mesh-check: `closed_of_open_edges`, pinch counterexample, chord sag) | handed to fix-mesh-asm (tessellation L1–L5, extending MeshCheck); item 4 redo after its crossing pre-pass |
| Sketch solver | — | L1; repro of `solve_after` reporting Solved over a reused unsolved group: verify/forge-sketch-check/tests/repro.rs |
| Sheets in check (ValidSheet) | surfaces' spec research/surfaces/lean/Sheet.lean | the IEEE session may be proving it; if not, prove it against the model as now (Model.lean has changed since 3d17b13) |

Tools here: the gates (tools/speed/gate.sh with GATE_COUNTER=callgrind; examples bits,
reliability, suite and nary with FUZZ_SEEDS=150; wasm bits via examples/bits on wasm32-wasip1);
the Aeneas toolchain (tools/aeneas/setup.sh, FORGE_AENEAS).
