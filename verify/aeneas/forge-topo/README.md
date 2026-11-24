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
| shells (`check_shell_faces`) | `topo_shell::shell_faces` | `forge_topo.shell_faces_refines` | L4 |
| edge uses (`check_edges`) | `topo_edges::edges` | `forge_topo.edges_refines` | L4 |
| loops (`check_loops`) | `topo_edges::loops` | `forge_topo.loops_refines` | L4 |
| shell Euler counts (`check_shell_euler`) | `topo_euler::shell_euler` | `forge_topo.shell_euler_refines` | L4 |
| fans (`check_fans`) | `topo_fans::fans` | `forge_topo.fans_refines_base` | L4 |

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

`shell_faces_refines` (`lean/ForgeTopo/ShellFaces.lean`): for a body whose links are sound
(`checkLinks = []`, as `check_known` runs it), any instance that `ShellReads` it (as `Reads`, and
the shells and each face's shell; shell ids below `u32::MAX`, the flood's mark for none, face ids
u32) and any sink that appends what it is handed, the translated `shell_faces` returns ok having
appended exactly `Body.checkShellFaces`. The three passes compute the model's `bad`
(`shell_pass_spec`, `face_listed_spec`, `edge_split_spec`); the flood is a depth-first search
whose invariant (`DFS`: what is stamped is reachable, the stack holds stamped faces, a stamped
face off the stack has its neighbours stamped, stack plus unstamped bounded by the faces) gives,
once the stack is empty, stamped = reachable; the model's flood reaches exactly that too
(`verify/forge-topo-check/lean/TopoCheck/Flood.lean`, `reached_iff`).

`edges_refines`, `loops_refines` (`lean/ForgeTopo/Edges.lean`): for a body with sound links and an
instance that `EdgeReads` it (as `Reads`, and each coedge's sense; every loop's coedges and their
edges in range, those of loops a face no longer lists too), the translated `edges` and `loops`
append exactly `Body.checkEdges` and `Body.checkLoops`. `check_edges`'s one geometric test (an
edge's curve degenerate: floats) is the view's `edge_degenerate`, taken as given: the problem it
adds is not one the model decides.

The translation is overflow-checked (translate.sh's default): each `+` and `-` the code makes on
a count is Aeneas's checked op, which fails on overflow. `lean/ForgeTopo/Checked.lean` steps
through them as the wrapping ops (`usize_add_wspec`, `u32_add_wspec`, `usize_sub_wspec`) once the
proof shows the bound (counts below `usize::MAX`, from the view's hypotheses), so `ok` in each
theorem above also says: no overflow.

`shell_euler_refines` (`lean/ForgeTopo/Euler.lean`): for a body with sound links, an instance
that `ShellReads` it and a sink that appends what it is handed, the translated `shell_euler`
appends exactly `Body.checkShellEuler`. The edges and vertices of a shell are counted by stamping
them with the shell (`Stamped`: the stamp marks exactly the ids met in this shell, every other
mark is an earlier shell's or none), which counts the distinct ids as the model's `eraseDups`
does (`distinct_snoc`). Hypothesis `EulerSmall`: each shell's listed faces, loops and coedges fit
`i64` (so `f`, `l`, `e`, `v` and V − E + 2F − L do not overflow). A shell's face id past the faces
counts as a face with no loops, as the model reads it (until this test moved here, `check`
panicked on such a body before `check_shell_faces` could report it).

`fans_refines` (`lean/ForgeTopo/Fans.lean`): for a body with sound links, an instance that
`FanReads` it and a sink that appends what it is handed, the translated `fans` appends exactly
`Body.checkFans corners`, where `corners` is what the view's `corners` returns (geometry: the
corner table, taken as given). The leaving coedges of each vertex are built by a counting sort
(`CountInv`, `vpre`, `PlaceInv`: the coedges of vertex `w` are `items[vpre w .. vpre (w+1)]`, in
the model's order); each fan walk keeps its seen coedges in a reused buffer whose first `sl`
entries are the model's `seen` (`walk_spec`; the model's fuel is past every use, `walk_fuel`).
`fans_refines` takes the table's contract `CornersOK` (the entries pair distinct coedges of the
face with a use that closes the corner) as a hypothesis; `fans_refines_base` discharges it. The
corner table is now translated code (`topo_fans::corners`, the walk of `Body::corners`): it pairs
each coedge of face `f` leaving `v` with the arriving use the geometry picks. Only the geometry is
a view method (`FanTopo::corner_frame`, whether the face has a frame at `v`; `closing`, the index
of the arriving use that closes a corner, or none), and only its answering is assumed (`FanBase`:
`frame_ok`, `closing_ok`), not what it answers. `corners_spec` proves every table it returns is a
`CornerList` (`CornersOK`'s entry contract), whatever the geometry answers; `cornersOf` is the table
the view computes, so `fans_refines_base` appends exactly `Body.checkFans (cornersOf ..)`.
Hypotheses: `FanBase` (the topology read as the model's, `nf_lt`: face ids below `u32::MAX`, and
geometry that answers), sound links, `FansSmall` (the vertex count plus one and the number of edge
uses fit `usize`).

`lean/ForgeTopo/Axioms.lean` fails the build if a theorem uses more than `propext`,
`Classical.choice` and `Quot.sound`.

Build: `cd lean && lake build` (shares Mathlib and Aeneas's library with
`verify/forge-mass/lean/.lake/packages`; the f64 bridge `verify/aeneas/bridge` is required but
unused, the topology has no floats).

Speed (T.fans, judged exception, the coordinator's call): against main 4846170a, exact callgrind
on one VM, the strict total is −0.0377% and every strict line but one is within +0.03%; gearbox
edit is +0.0399% (450,709,672 vs 450,529,868 Ir). Callgrind attributes it to heap layout, not to
the fan test: in that run `check_fans` costs ~150k Ir less (Body::fans_by −58k, Body::leaving −23k,
their mallocs −100k, against topo_fans +95k), while memcpy is +384k, from the boolean's reused
`scratch.pieces` (Vec<Piece>) growing by copy where it had grown in place (edge_split's grow_one
100k → 400k). Layout variants tried (capacity hints, allocation order, reserving `scratch.pieces`)
gave +0.037..+0.089% on that line.

Next: `check_known`'s combinatorial part composed from these (T.all).
