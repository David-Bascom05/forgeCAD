# forge-topo-check: what `check` proves about a body's topology

Everything downstream trusts `forge_topo::check`. This directory proves what its combinatorial part
guarantees, ties the proof to the kernel's code with a differential test, and lists what it does not
guarantee. Line references are to `kernel/forge-topo/src/{body,check}.rs` at 8250ebb.

## The spec and the proof (`lean/`, core Lean 4.36, no Mathlib)

- `TopoCheck/Model.lean`: `check`'s combinatorial part as a Lean program:
  - `check_links`, which runs first and alone where it fails;
  - `check_edges`' use count and sense test;
  - `check_loops`;
  - `check_shell_euler`;
  - `check_fans` through `leaving`, `fans` (with its guard on corner tables) and `fans_by`;
  - `check_shell_faces` and `check_strays`;
  - the gate in `check_known` that runs shell Euler and fans only when no edge's use count is
    wrong and no loop is broken.

  The geometric corner pairing `Body::corners` is a parameter.
- `TopoCheck/Spec.lean`: `Valid`, a closed, oriented, manifold combinatorial map:
  - its links agree (`WF`: ranges, edges' uses and loops' coedges naming each other, faces' loops
    their own, each listed once);
  - every edge has exactly two uses, distinct, with opposite senses;
  - every face has a loop, and every loop is a non-empty closed walk;
  - `partner` is an involution that reverses direction (oriented);
  - round each vertex, the fan walk check takes (loop order, or the corner table where that is
    one to one) permutes the leaving coedges in one cycle: one fan, a disc;
  - every vertex is an end of an edge;
  - the shells partition the faces, the faces at an edge share a shell, and each shell is joined
    through its edges: the shells are the connected pieces;
  - each shell's V − E + 2F − L is even and at most 2.

  A loop no face lists (as `remove_loop` leaves one) is left over from an in-place edit, no part
  of the body; `compact` drops it.
- `TopoCheck/Proof.lean`:
  - `check_sound`: `b.check corners = [] → b.Valid corners`, for any corner table, with no
    hypothesis.
  - `one_cycle`: with no corner table, loop order's rotation round every vertex is a single cycle.

  `Axioms.lean` fails the build unless both use only `propext`, `Classical.choice` and
  `Quot.sound`. No `sorry`.

## The tie to the Rust (`tests/diff.rs`; `./diff.sh` builds the Lean model and runs everything)

- `model_agrees`: benchmark and fuzz parts and mutants of them. The mutants:
  - a flipped sense, swapped ends, a dropped or repeated use, uses traded between edges;
  - merged vertices, merged shells, a dropped loop;
  - a stray vertex, a face dropped from its shell, a coedge naming another's loop.

  The compiled Lean model's problems must equal the kernel's combinatorial problems, in order,
  with every `Body::corners` table supplied. Every rule is reached.
- `kernel_bodies_have_back_links`: every part's body passes `check` and satisfies `WF`.

## The holes the first model left, closed (check.rs, body.rs; tests in `tests/diff.rs`)

1. **A shell in pieces** (`disconnected_shell_rejected`). A holed plate and an overlapping plate
   as one shell passed: Euler 0 + 2 = 2, and the overlap test runs only between shells. Fixes:
   - `check_shell_faces` now rejects a shell whose faces are not joined through edges, a face
     its shell does not list (or lists twice), and two faces at an edge in different shells.
   - Two ops made such bodies, now regrouped by connectivity:
     - `revolve` of a profile with a hole through a whole turn (the hole sweeps a void);
     - `thicken` of faces apart.
2. **Back-links** (`broken_back_links_rejected`). `check_links` tests `WF` and every id's range
   before anything reads through them. An edge's use swapped for a copy outside every listed
   loop used to pass; two coedges trading loops used to panic `check`.
3. **Stray vertices** (`stray_vertices_rejected`). `check_strays` rejects a vertex no edge
   references. A dead edge's ends count as referenced: they are left for `compact` with it.
4. **Corner tables not one to one** (`seam_corners_one_to_one`).
   - `Body::corners` now takes the other use of a coedge's own edge last: at a seam it leaves the
     same way. The seam vertex's two leaving coedges are then closed as loop order closes them.
   - `Body::fans` walks by a corner table only where it closes each leaving coedge with a
     distinct use, of its edge, arriving at the vertex. Otherwise loop order's fans stand. So one
     fan by either rule is one disc, which `check_sound` proves.

The gates run on the change, against the baselines in tools/:
- the scoreboard (`reliability`): 0 worse, 3 better;
- `FUZZ_SEEDS=150` suite: 150/150;
- nary: agrees, 167/167;
- bits: identical;
- forge-model, forge-lang, forge-topo, forge-bool, forge-ops and forge-mesh tests pass.

Two kernel tests now see a fault as the broken back-link it is: forge-ops `checker_catches_mistakes`
and forge-lang `a_fault_in_any_one_entity_is_found`. The latter gained faults that keep the links,
for `EdgeUses` and `FaceNoLoops`.
