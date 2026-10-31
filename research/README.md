# Research: how the numbers and formulas got here

Every proof, derivation or measurement whose output shipped code uses is kept here, one folder
per topic, with what was derived, how to rerun it, and where the code uses it; the code points
back with a one-line comment. Dead ends get a short note only. Offline tools (SymPy, Lean,
OpenCascade) never become build dependencies; run them through `fbuild`.

## Topics

- `closed-forms/`: quartics and Cardano for surface pairs, quartic root counts, parabolas (text).
  Also `tools/sympy/meet.py`, which generates `kernel/forge-geom/src/meet_gen.rs`.
- `predicate-bounds/`: the float filters' error bounds in forge-math's exact predicates.
- `mesh-bounds/`: chord-sag step sizes and the measured mesh layout choices.
- `upto-proofs/`: the exact arguments that prove an end on a surface and the next face.
- `gpu-auto/`: the GPU crossover, the 128-point floor and the cost of opening the GPU.
- `tolerance/`: LINEAR, ANGULAR and the one-tolerance decision.
- `blend-rows/`: how close a variable blend's rows are kept.
- `nary-short-runs/`: the n-ary regen's hand-off of short runs to the sequential booleans, and the
  fuzz crossover by run length it was measured with.
- `mass-props/`: exact mass properties' speed choices (corner patches' domain rule, procedural
  edge pieces, swept surfaces' columns, chord-walked pipe spines) and the timing harness.
- `feature-costs/`: the ranked cost of every feature kind by prefix difference, and the tools.
- `corner-patch/`: the rational corner quads and their fold proof, from branch `fold-research`
  (not yet on main when this index was written). Its README also covers the four
  `refs/experiments/corner-*` refs.

## The experiment refs

None of them is used by main's corner patch (`HolePatch`); all stay as refs:
- `corner-blend-interval-proof` (a063386): corner-sector interpolant plus a first-order interval
  regularity proof; 45272 cells / 345 ms for one octant patch, and 10 forced-split folds slipped
  past the sampled check. Dropped.
- `corner-fold-mean-value` (5bb82a8): mean-value regularity proof; sound but refused nearly every
  survey patch (bounds 10-170x the truth near corners). Dropped.
- `corner-fold-taylor-models` (7143102): Taylor-model fold proof; proved every survey patch but at
  0.25-6.6 s each. Dropped; its analysis became `docs/fold-proof.md` on `fold-research`.
- `corner-rational-stage1` (4b81f12): the first rational patch with a Bernstein fold proof; the
  quads on `fold-research` superseded it.

Main's `HolePatch` constants (the 16 x 16 fold scan at 60 degrees, 64 x 64 for split pieces,
domain sides by the fourth root of side length, stops at nine tenths of a chain's middle,
`MAX_SIDES` = 12; 1813f3c, 785ba61, 2120083) were tuned against the fillet corpus and the
synthetic 3-, 4- and 5-edge corner tests; no separate instrument was kept. The rational patch is
meant to replace them.

## Already recorded elsewhere

- Speed claims in commit messages are instruction counts from the committed examples
  (`regen1`, `mesh1`, `meshcost`, `fillet1`, `finish1`, `varblend`, `swept`, `cdt_cost`,
  `sketchbench`, `backends`) and the gate (`tools/speed/gate.sh`, `bits.txt`).
- Robustness numbers come from `examples/suite` (fuzz), `nhunt`/`hunt` (adversarial families),
  `reliability` (degeneracy scoreboard) and the OpenCascade oracle (`tools/occt`).
- From the literature, not derived here: dmath's fdlibm kernels, Shewchuk's predicate bounds,
  the hole tables (DIN 336, DIN 974-1, ASME B18.3).
- Test fonts are written at test time by `kernel/forge-font/src/testing.rs`; nothing derived.

## Not on main yet

Nothing to backfill until these merge, and then their branches owe a folder here: the
`regenerate_auto` tool-count threshold (`nary-speed-2`/`-3`), exact mass properties' quadrature
choices (`massprops`), the loft x loft intersection research (forgecad-gate `research/loft-ssi`).
