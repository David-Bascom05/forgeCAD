# verify/aeneas/forge-topo: `check`'s topology, as shipped, refines the model

`kernel/forge-topo/src/topo.rs` holds the float-free tests `check` makes of a body's topology,
written over a read-only view `Topo` (implemented by `check.rs`'s `View`, a body's arrays as
slices) and an output `Sink` (implemented by `Vec<Problem>`). Charon and Aeneas
translate that file — the real code, built `--release` — to `lean/ForgeTopo/{Types,Funs}.lean`
(`tools/aeneas/translate.sh forge-topo`; `--check` fails if the committed translation is stale).
The proofs in `lean/ForgeTopo/` show the translation computes exactly what
`verify/forge-topo-check/lean`'s model computes, so `TopoCheck.Body.check_sound` speaks of the
shipped code for each test moved here.

| test | Rust | refinement theorem | level |
|---|---|---|---|
| stray vertices (`check_strays`) | `topo::strays` | `forge_topo.strays_refines` | L4 |
| back-links (`check_links`) | `topo::links` | `forge_topo.links_refines` | L4 |

`strays_refines`: for any `Topo` instance whose counts and edge ends are the model body's, and
any `Sink` that appends the problem it is handed, the translated `strays` returns (no panic, no
overflow: Aeneas's `Result` is `ok`) having appended exactly `Body.checkStrays` of the model. The
hypotheses are the two instances: `View`'s methods read the arrays the encoding
(`verify/forge-topo-check/src/lib.rs`, checked on 1159 kernel bodies by `tests/diff.rs`) writes,
and `Vec<Problem>`'s `stray` is one `push`.

`links_refines` (`lean/ForgeTopo/Links.lean`): for any instance that `Reads` the model body (its
counts, and by index every field `links` reads: edge ends and uses, coedge edge and loop, loop
span and face, face loops; every entity alive, as in the compact bodies `check` looks at,
check.rs:181–183) and any sink that appends what it is handed, the translated `links` returns ok
having appended exactly `Body.checkLinks`. On the way: the face pass's `listed` is the model's
`listed` saturated at 2 (`Kept`, `face_pass_spec`), the edge pass gives `edgeOk`
(`edge_pass_spec`), the loop pass `loopSkip || loopOk` (`loop_pass_spec`; a run of coedges past
the array fails `loopOk` too, `all_coOk_unfit`).

`lean/ForgeTopo/Axioms.lean` fails the build if a theorem uses more than `propext`,
`Classical.choice` and `Quot.sound`.

Build: `cd lean && lake build` (shares Mathlib and Aeneas's library with
`verify/forge-mass/lean/.lake/packages`; the f64 bridge `verify/aeneas/bridge` is required but
unused, the topology has no floats).

Next: the shell tests (`check_shell_faces`: listing, flood), then the edge-use and loop-closure
tests, the shell Euler counts and the fans move to `topo.rs` the same way, until `check`'s whole
topological part is translated.
