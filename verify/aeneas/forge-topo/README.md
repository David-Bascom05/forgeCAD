# verify/aeneas/forge-topo: `check`'s topology, as shipped, refines the model

`kernel/forge-topo/src/topo.rs` holds the float-free tests `check` makes of a body's topology,
written over a read-only view `Topo` that `Body` implements (`check.rs`). Charon and Aeneas
translate that file — the real code, built `--release` — to `lean/ForgeTopo/{Types,Funs}.lean`
(`tools/aeneas/translate.sh forge-topo`; `--check` fails if the committed translation is stale).
The proofs in `lean/ForgeTopo/` show the translation computes exactly what
`verify/forge-topo-check/lean`'s model computes, so `TopoCheck.Body.check_sound` speaks of the
shipped code for each test moved here.

| test | Rust | refinement theorem | level |
|---|---|---|---|
| stray vertices (`check_strays`) | `topo::strays` | `forge_topo.strays_refines` | L4 |

`strays_refines`: for any `Topo` instance whose counts and edge ends are the model body's, the
translated `strays` returns (no panic, no overflow: Aeneas's `Result` is `ok`) a list that, as
problems, is `Body.checkStrays` of the model. The hypotheses are the encoding
(`verify/forge-topo-check/src/lib.rs`, checked on 1159 kernel bodies by `tests/diff.rs`): `Body`'s
`Topo` methods read the same arrays the encoding writes.

`lean/ForgeTopo/Axioms.lean` fails the build if a theorem uses more than `propext`,
`Classical.choice` and `Quot.sound`.

Build: `cd lean && lake build` (shares Mathlib and Aeneas's library with
`verify/forge-mass/lean/.lake/packages`; the f64 bridge `verify/aeneas/bridge` is required but
unused, the topology has no floats).

Next: `check_links`, the shell flood and Euler counts, and the fan tests move to `topo.rs` the
same way, until `check`'s whole topological part is translated.
