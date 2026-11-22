# forge-topo-check: what `check` proves about a body's topology

Everything downstream trusts `forge_topo::check`. This directory proves what its combinatorial part
guarantees, ties the proof to the kernel's code with a differential test, and lists what it does not
guarantee. Line references are to `kernel/forge-topo/src/{body,check}.rs` at 8250ebb.

## The spec and the proof (`lean/`, core Lean 4.36, no Mathlib)

- `TopoCheck/Model.lean`: `check`'s combinatorial part as a Lean program, line by line. It covers:
  - `check_edges`' use count and sense test;
  - `check_loops`;
  - `check_shell_euler`;
  - `check_fans` through `leaving`, `fans` and `fans_by`;
  - the gate in `check_known` that runs the last two only when no edge's use count is wrong and
    no loop is broken.

  The geometric corner pairing `Body::corners` is a parameter.
- `TopoCheck/Spec.lean`: `Valid`, a closed, oriented, manifold combinatorial map:
  - every edge has exactly two uses, distinct, with opposite senses;
  - every face has a loop, and every loop is a non-empty closed walk;
  - `partner` is an involution that reverses direction (oriented);
  - round each vertex, the rotation `rot c = partner (prev c)` maps the leaving coedges one to one
    into themselves, and the fan walk reaches all of them from the first (one fan);
  - each shell's V − E + 2F − L is even and at most 2.

  Also `WF`, the back-links `add_loop` sets up, which the proof assumes.
- `TopoCheck/Proof.lean`:
  - `check_sound`: `WF b → b.check corners = [] → b.Valid corners`, for any corner table.
  - `one_cycle`: with no corner table, the rotation round every vertex is a single cycle
    through all its leaving coedges (pigeonhole on the period), so the faces there form one disc.
  - `one_cycle_by`: the same for the corner route, given that the corner walk is one to one.

  All three use only `propext`, `Classical.choice` and `Quot.sound`. No `sorry`.

## The tie to the Rust (`tests/diff.rs`; `./diff.sh` builds the Lean model and runs everything)

- `model_agrees`: 861 bodies. Each is a benchmark or fuzz part, or a mutant of one: a flipped sense,
  swapped ends, a dropped or repeated use, uses traded between edges, merged vertices, merged
  shells, a dropped loop. For each, `src/lib.rs` writes the topology and every `Body::corners`
  table, and the compiled Lean model's problems must equal the kernel's combinatorial problems in
  order. All 861 agree. 752 have problems, and every rule is reached: 97 `vertex-fans`, many through
  the corner route.
- `kernel_bodies_have_back_links`: every part's body passes `check` and satisfies `WF`.

## Assumed, not proven

- `WF`: `check` never tests it, and bodies built through `add_loop` have it.
- Euler: the shell value equals the Euler characteristic 2 − 2g only if each face is a planar
  domain and each edge an arc. That is geometry, so the theorem stops at the counts.
- The corner route: `Body::corners` is transcribed in `src/lib.rs` for the test, not modelled.

## What check accepts that `Valid` alone does not exclude (repros in `tests/diff.rs`)

1. `repro_disconnected_shell_passes`: a shell need not be connected. A plate with a hole (value 0)
   and an overlapping plate (value 2) as one shell pass `check`:
   - `check_shells` (the overlap test) runs only with two shells or more;
   - `genus` reads 0.

   As two shells, the overlap is found. Bodies whose shells came from `rebuild_shells`/`compact`
   are grouped by connectivity, so this needs a body built or edited by hand. A connectivity test
   per shell, or always running `check_shells`, would close it.
2. `repro_back_links_unchecked`: two loops that no face lists and whose coedges no edge lists pass.
3. `repro_stray_vertices_pass`: two vertices on no edge pass, on a body with a hole (the global
   value goes from 0 to 2).
4. `repro_seam_corners_not_one_to_one`: `Body::corners` is not one to one. At a cylinder's seam
   vertex, a closed edge both leaves and arrives, and both leaving coedges get the seam's arriving
   use. Across the test's bodies, 104 720 corner tables do this. The route is taken only where loop
   order already finds two fans, so on valid bodies it never decides. No body was found that it
   wrongly accepts:
   - `pinched_pins_rejected`: two tangent pins sharing their seam vertices are rejected;
   - merged-vertex mutants it reads as one fan fail other tests.
