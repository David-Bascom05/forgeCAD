# Sheet metal prototype: how the numbers were made

Evidence for a design decision, not a feature. Nothing here ships. The write-up with the recommendation is outside
the repo: `~/projects/forgecad-gate/sheet-metal/prototype.md`. The formulas it tests are in `prior-art.md` next to that.

`proto/` is its own Cargo workspace (zero third-party crates), so nothing enters the kernel's build, lockfile or CI. One
additive kernel change sits on this branch: `forge_geom::wrap::Flat::Arc`, with its test, and arms that refuse it in
`wrap_wall`, `line_wall`, `meet_wall` and `glyph`. No existing part builds one (the speed gate shows no change).

## What is what

- `proto/src/region.rs`: Option A, n-ary wall minus cuts on `Segment` loops. Split at every meeting, decide each
  sub-piece by what is on its two sides at its midpoint, keep those with the region on exactly one side, chain.
- `proto/src/prism.rs`: Option B, forge-bool on a flat prism (`bseq` one cut at a time, `bnary` all at once), loops read
  back off the top face.
- `proto/src/reference.rs`: the independent area. Exact scanlines over the raw inputs at an angle to the part, shares no
  code with either option. `proto/src/verify.rs` also holds answers against 2000 sampled points and the kernel's own
  `Profile::normalized`.
- `proto/src/cases.rs`: the cases, a function of (kind, n, seed).
- `proto/src/sheet.rs`: direct construction. Cuts a flat region at the bends' tangent lines, traces each strip's faces,
  lays the one topology out through the fold map or the flat map. Uses `forge_topo` body building and existing surfaces
  only, no booleans. Errors are typed enums with entity and numbers (`BuildError`, `RegionError`).
- `proto/src/bin/bracket.rs`: parts against closed forms; `by_features` builds the same bracket with today's extrude,
  union and fillet; `fuzz` runs random chains and cuts through region, build, check, mass and mesh.
- `proto/src/bin/dbgroots.rs`: a standalone reproduction of the `cross_sections` bug below.
- `occt_ref.py`: L and U built in OCCT from prisms and revolves, none of our code.
- `check_arc.py`: SymPy for the circle-across-a-bend claims.
- `summarise.py`: the tables from `results/`.

## Rerun

From `proto/`:

```
fbuild cargo build --release
../run_trim.sh                                 # reliability of a, bseq, bnary: results/trim-*.log
fbuild target/release/trim all nudged 2 0 100 > ../results/trim-nudged.log   # and 20 0 100, 200 0 10, appended
../run_trim_a.sh                               # Option A's final code in place of its earlier log lines
SHEET_FIXED_CROSS=1 target/release/trim verify a KIND N 0 S   # A with the cross_sections fix copy: trim-a-fixed-*.txt
../run_fuzz.sh                                 # 1000 random parts: results/fuzz.txt
../run_cost.sh > ../results/trim-cost.txt      # instructions, generation subtracted
../run_cost2.sh > ../results/construction-cost.txt
target/release/bracket all .scratch/step > ../results/parts.txt
target/release/bracket invariants > ../results/invariants.txt
target/release/bracket refusals > ../results/refusals.txt
fbuild $VENV/bin/python -I ../../../tools/occt/step_read.py --jobs 2 .scratch/step/*.step > ../results/occt-step.jsonl
fbuild $VENV/bin/python -I ../occt_ref.py > ../results/occt-ref.txt
```

`$VENV` is `tools/occt/setup.sh`'s, `~/.local/share/forgecad-occt/venv`. `check_arc.py` needs sympy 1.14.
Native against wasm: build with `--target wasm32-wasip1`, run `bracket bits` and `trim bits` under
`tools/wasi-run.mjs` and native, and `cmp` the two outputs. Then `python3 summarise.py` from this folder.

## The kernel bug this found

`forge_geom::spline::meet::cross_sections` drops real crossings of two planar splines. Its Newton loop accepts only an
absolute step of 1e-13, and about 300 mm from the origin rounding keeps the step circling around that value, so the
leaf is thrown away with no error. `results/cross-sections-bug.txt`: two crossings by polyline, one reported; moved to
the origin, two. It made both Option A and forge-bool refuse the same cases, as an unbalanced chain and as
`InconsistentClassification`. Not fixed here. Scaling the limit by the coordinates, in a temporary build, cleared every
such refusal in both options. `SHEET_FIXED_CROSS=1` runs Option A with that change in a local copy.

## Tried and dropped

- A tolerance band of 2e-5 for "this piece lies along that boundary": it swallowed real pieces beside tangent contacts
  and unbalanced the chain. Now 1e-9 for coincidence, and a long piece side by side within 2e-5 is a typed refusal.
- A level test ray: symmetric sketches put midpoints level with circle centres, where neighbouring arcs' rounded end
  heights straddled the ray and a crossing vanished. Now the ray is turned 0.0137 rad.
- A level reference scanline: edges level with the rows make the row length jump, a first-order error of about 5e-3.
  Now the rows are turned 0.37 rad.
- Filleting both of a bracket's bends at once: `Overflow`, and also before `unify`. The feature-built bracket unifies
  first, then fillets the outside and the inside one at a time.

## Notes on the results

- `trim-a-fixed*.txt` were run before Option A snapped its line ends to their vertices; `fuzz.txt` too. The snap moves
  line ends by less than the tolerance. The `trim-*.log` lines for `a` are the final code.
- The fuzz's volume truth is a scanline with an error near 1e-4 at 400k rows: `fuzz-convergence-1186.txt` shows it
  falling with rows, and each part is also held against Green's area of the exact regions (worst 7.3e-11, under the
  summed `mass` err on every part).

## Where the kernel uses this (kernel/forge-sheet)

- `region.rs` is Option A from `proto/src/region.rs`, rewritten: pinches and near-coincident edges are typed
  refusals naming the input segments; its constants (`ALONG` 1e-9 coincident, `NEAR` 2e-5 near-coincident, the turned
  test ray `RAY`) are the ones the trim runs above settled on (`results/summary.md`, `results/trim-a-fixed*.txt`).
- `tests/region.rs` keeps `cases.rs` and `reference.rs` (scanline) from here, and Option B (forge-bool on a flat
  prism) as the cross-check.
- `build.rs` is `proto/src/sheet.rs` generalised from a chain in global flat x to a tree of walls, each with its own
  coordinates (the prototype's K change moved the folded part by 7e-15; the crate's moves it by nothing,
  `k_moves_nothing_folded_to_the_bit`). Its closed forms (setback, BA, the volume identity) are `prior-art.md`'s,
  checked by `check_fold.py` here (SymPy and mpmath: `python3 check_fold.py`).

`proto/` builds against `forge_geom::wrap::Flat::Arc` (on main since 3776cb18); it stays outside the workspace.
