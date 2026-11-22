# Coverage of the flat-face boolean: handoff (2026-11-22)

Task: prove in Lean whether `forge_bool::boolean` covers every consistent sign pattern of flat
faces, and turn each gap into a kernel site, a part and a test. Kernel at 888fc195 (branch
claude/repo-review-mbdrlj). research/kernel-audit/coverage.md is **not written yet**. This file
holds the state.

## What is here

| path | what |
|---|---|
| `Coverage/Model.lean` | Hand model of the flat path on two axis-aligned boxes, line-referenced to boolean.rs and intersect.rs. It covers face pairs (986–1060), `trim`'s edge loop and stretch test (2265–3167), coplanar `imprint` (4109–4274), `coincide`/`share_cuts`/`shared` (4464–4535, 1652–1693), splitting into regions, `classify_specs` (coplanar, lean, flood, ray; 5861–6071), `keep_of`, and the four-use ZeroThickness test (1724–1737). It also holds the exact regularised boolean on cells and the `Outcome` comparison. Core Lean only. |
| `Coverage/Theorems.lean` | The theorems. Kernel-checked: `Rel.complete`, `Rel.count`, `flags_inert`. By `native_decide`: `no_retry`, `sample_inert`, `gaps`, `correct_off_gaps`. |
| `Main.lean` | The `coverage` exe: the model's outcome for every pattern (→ `results/model.tsv`). |
| `results/` | `model.tsv` (Lean model) and `kernel-unit10.tsv` (real kernel) cover all 6591 patterns. `band-*.tsv` hold the tolerance-band sweeps. |
| `aeneas/` | Aeneas toolchain scripts, plus `placecuts/`: `place_cuts`' clustering (boolean.rs:4561–4620) extracted float-free, with the distance tests passed in as Boolean tables. It is the first Aeneas target. |
| `verify/forge-exact/` (Rust crate) | `src/lib.rs`: Allen patterns, box builder, `kernel`/`kernel_on` outcome classifier, exact cell answer `exact`/`exact_boxes`. `examples/box_table.rs`: whole family, kernel vs exact. `examples/box_band.rs`: tolerance band. `examples/box_one.rs`: one pattern with offsets (run with FORGE_DEBUG=1). `tests/box_family.rs`: the gap tests. `parts/corner_touch.fg`: the .fg repro. |

The spec agent's Lean files share `verify/forge-exact`. This crate owns its `Cargo.toml`, and the
coordinator has told the spec agent so.

## Where it stands

**Pattern space.** On two boxes, every predicate the flat path evaluates compares two coordinates
on one axis. So a consistent sign assignment is one of Allen's 13 interval relations per axis
(`Rel.complete` proves the list complete). With three ops that gives 13³ × 3 = 6591 patterns. A
stretch's middle sample adds one free sign (`sv`). It never matters (`sample_inert`).

**Results.**
- `no_retry`: no consistent pattern reaches Lost or Unsupported. The retry ladder is never entered
  on this family.
  - Checked by `native_decide` alone: 305 s interpreted on the throttled machine.
  - The full `Theorems.lean` (4 native_decide theorems) has not been built in one go yet. Each
    takes about 5 min there.
- `flags_inert` is kernel-checked: CAREFUL, CHORDS, NO_RUN_ON, FENCES and ABSORB change nothing a
  flat pair reaches.
  - CAREFUL is set only with a torus or procedural face (896).
  - ODD/FENCES need springs.
  - ABSORB is read only for conic roots (intersect.rs:559).
  - CHORDS reaches flat faces only at `hair` (3005), and `past` is empty on boxes.
  - NO_RUN_ON reaches flat faces only in the `Hits` arm (2448). A box's parallel edge is
    `OnSurface` and takes the run-on arm regardless.
- The model's outcome equals the exact one on 6583 patterns. **8 gap patterns**: meets/metBy on all
  three axes, union. These are two boxes meeting at one corner, the scoreboard's
  `plane_plane/vertex_on_vertex/Add` and its mirror images.
  - Exact answer: ZeroThickness (the union is not a manifold at the corner).
  - Kernel: a pinched body that `check` accepts, volume right (2000 at unit 10).
  - Site: no flat-face site decides a corner touch.
    - The four-use test (boolean.rs:1724–1737) sees edges only.
    - The touch test (1061–1094) is fed only by `touches` (2048–2101), which is empty for two
      planes.
    - `place_cuts` merges the corners (an alias), and `split_old_pinches` (631–653, from
      `general` 571) separates them again.
  - Real parts: the scoreboard row `plane_plane/vertex_on_vertex/Add` reads `invalid` there. That
    is the forge-model regen path, not the bare boolean, and is not yet explained. `forgecad build
    parts/corner_touch.fg` reports "valid".
- **The real kernel agrees with the model pattern for pattern** on all 6591, including the gap set,
  at units 0.001, 1, 7.3, 10 and 1000 (`box_table`).
- Tests: `cargo test --release -p forge-exact` passes 2 tests and ignores 8:
  - `box_family_off_the_gaps_is_exact` pins the 6583;
  - `corner_touch_today_makes_a_pinched_body` pins today's behaviour;
  - the 8 `corner_touch_*` tests are `#[ignore = "coverage gap: … at boolean.rs:1724 …"]` and fail
    today, as intended.

## Tolerance band (inconsistent readings): findings so far

`box_band` takes every pattern with coincident ends and moves B's coincident ends by δ × LINEAR,
outward on both of B's ends. It flags a run whose outcome matches neither reading (the coincident
pattern, or the moved boxes taken exactly) and is not a valid body within 1e-4 of either reading's
volume.

1. **All three axes, or two (BAND_AXES=01): δ ∈ (LINEAR/√2, LINEAR] fails.**
   - Edges found: 0.705 passes, 0.7075 fails; 1.0 fails, 1.05 passes.
   - All axes: 2778 of 5943 runs, mostly `SplitCrossed`; at −0.9, also `Euler`/`VertexFans`/`EdgeUses`
     invalid bodies. Axes 01: 1323 of 5187.
   - Mechanism: each plane reads coincident (|d| ≤ LINEAR, intersect.rs:853–855), while a corner
     moved on two axes is √2·δ > LINEAR from the edge or vertex it should be on (`on_edge`
     283–300, `coincide` 4464, `end_vertex` in `place_cuts`).
   - This is "decide coincidence once, then re-measure distances", the near-coincident.md pattern,
     now with an exact band edge.
2. **One axis (BAND_AXES=0 or 2): |δ| ∈ (LINEAR, ≈4.8 LINEAR) fails; |δ| ≤ LINEAR and ≥ 4.9 pass.**
   - Axis 0: 527 runs for δ>0 (295 invalid `EdgeUses`, 232 spurious ZeroThickness), 302 for δ<0.
     Axis 2: 625.
   - Smallest case: `equal/meets/equal union 1.5 0`. B sits beside A in y and is 1.5e-5 wider
     at both x ends. FORGE_DEBUG shows:
     - in the coplanar imprint, A's corner on B's edge is put on B's end vertex, 1.5e-5 away
       (cuts 4–5, 9, 14–15, 19 keyed to `Old(1, …)`);
     - the "same place" decision comes out one way on one face and the other way elsewhere.
   - The outer edge (~4.8 LINEAR) matches no constant read so far. Candidates: a grown vertex
     tolerance (`settle`, 4646; `grown_tols`, 1997), `edge_box(e).grow(2*slack)` (2373), or the
     `4.0 * LINEAR` sample break (3022). Not yet pinned.
3. Within |δ| ≤ 0.5 LINEAR, and on every axis at |δ| ≥ 5 LINEAR, nothing fails apart from the 8
   corner gaps. The retry rungs needed by these band runs are not observable without the
   decision-log agent's ladder counters (their `decisions` feature, uncommitted in
   worktree-agent-a92b7ce49ed6ca3dc).

## Rebuild

Lean (elan at ~/.elan, toolchain v4.36.0-rc2, no Mathlib):
```
cd research/kernel-audit/coverage
lake build Coverage.Model coverage     # seconds
.lake/build/bin/coverage > results/model.tsv
lake build Coverage.Theorems           # ~20 min throttled: four native_decide evaluations
```
Rust (shared target dir saves disk):
```
CARGO_TARGET_DIR=/home/user/forgeCAD/target cargo build --release -p forge-exact --examples
target/release/examples/box_table 10 [--all]
BAND_AXES=0 target/release/examples/box_band 0.9 1.5 4.8 4.9
FORGE_DEBUG=1 target/release/examples/box_one equal/meets/equal union 1.5 0
cargo test --release -p forge-exact [-- --ignored]
```
Aeneas (clones at /home/user/aeneasverif/{aeneas@2e3d448, charon@b23c297}, shallow):
- `aeneas/setup.sh` does three things:
  - installs opam 2.3.0 from GitHub releases into a local OPAMROOT;
  - creates the OCaml 5.3.0 switch;
  - installs Rust nightly-2026-09-17 with rustc-dev, llvm-tools and rust-src only (the toolchain
    file's eight extra targets and miri are skipped).
- Its `opam install` lines ran with no switch set. `aeneas/build.sh` fixes that with
  `OPAMSWITCH=5.3.0` and installs the charon and aeneas OCaml deps (done).
- **Blocked:** charon's Rust build fails on `crates/miniutil`. It needs the `minirust` git
  submodule (`crates/minirust/tooling/miniutil`), which the shallow clone lacks: run
  `git submodule update --init` in the charon clone, or clone with submodules.
- Also: aeneas pins charon 9e193f0 (`charon-pin`) but the clone is b23c297. build.sh symlinks
  `aeneas/charon` to the clone, which skips the pin check. A mismatch may break the aeneas build.
- Then: `charon cargo --preset=aeneas` in `aeneas/placecuts`, and
  `aeneas -backend lean placecuts.llbc`.

## Next

1. Finish Aeneas on `placecuts`. If it translates, prove in Lean that the greedy clustering plus
   chain merge equals union-find on the `near` relation when `near` is an equivalence (the
   decide-once rule). Find the patterns where it differs when `near` is not transitive, which is
   the 2× chain.
2. Pin the one-axis band's outer edge (~4.8 LINEAR) and its site. Map each band failure to the
   disagreeing pair of sites with the decision-log agent's `decisions` feature once it lands.
   Mark representative band cases as `#[ignore = "coverage gap: band …"]` tests.
3. Explain the scoreboard's `invalid` for vertex_on_vertex on the forge-model regen path. Add
   forge-model as a dev-dependency and run the Part.
4. Correctness theorem against the spec agent's definitions: their `reg_kept` (Regular.lean, on
   Mathlib) makes the cell answer in `exact` the regularised boolean. State
   `kernel = exact ↔ result = reg (op A B)` once their arrangement and tolerance definitions land.
5. Widen the family beyond boxes: a tilted plane (the across `smooth` 1e-3 test at 4085,
   `nearly_flush` NEAR=1e-4 at ssi.rs:66, lean's 1e-9 at 5926, the run-on 1e-6 at 2450).
6. Write research/kernel-audit/coverage.md:
   - what's covered;
   - the gaps (pattern, site, repro, real-part hits);
   - retry rungs reached only by gap or band patterns: on flat faces, all of them, since none is
     entered by a consistent pattern;
   - the float-free functions to extract from kernel/ for Aeneas: `place_cuts` clustering
     (4561–4620), `edge_split` (756–813), `classify_specs`' flood (5933–6031), `keep_of` (6096),
     the four-use count (1724–1737), `split_new_pinches`/`split_old_pinches` fan walks (631–699),
     and the retry ladder control (889–962) with its flags as parameters.
