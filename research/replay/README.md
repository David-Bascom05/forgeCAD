# Plan replay: what it could gain on a drag (2026-10-31)

David's question: on a drag where a feature's structure holds, reuse the boolean's decisions and re-run
only the calculation, exact to the bit, procedural surfaces included. Stopped at the measuring step: on
the complex parts the realistic best is under 2x. Nothing here is used by shipped code.

## Rerun

```
git apply research/replay/measure/probe.patch    # counters and research/rebuild's measure patch: never on main
cp research/replay/measure/{dragplan,warmbits}.rs interface/forge-agent/examples/
RUSTFLAGS="-Cforce-frame-pointers=yes -Cdebuginfo=line-tables-only" CARGO_TARGET_DIR=target/fp \
  fbuild cargo build --release -p forge-agent --example dragplan --example warmbits
# from a scratch directory holding the scripts and drags.txt, for each `part number` line:
dragplan part number 8 > steps-part.txt
perf record -e instructions:u -c 200000 --call-graph fp -o d dragplan part number 16
perf script -i d -F comm,tid,ip,sym --no-inline | gzip > prof/part.txt.gz
python3 cap.py
warmbits part number 0.002; warmbits part number 0.0002
```

## Method

Instructions on main at bf21c471 with names-arena-2 and rebuild-1 on top, 8 steps, 2% swing, skip off.
Face pairs by counters: kept if the pair left a cut or new edge, closed if both surfaces are analytic with a closed
form; the n-ary's "kept" includes edges dropped later, so calculation is on the high side. Everything else by
sampled stacks (they agree with the counters within 10%). "Same value": both surfaces (bits at five parameters) and
the region equal the last frame's. Validity is check::check and selfx. Procedural boxes count as calculation:
a traced curve's chain follows its region (`leaf = size / 64`), so their bits reach the result.

## Measured (Mi per frame)

```
             total  parse  calc      calc   decide  decide  of them   caps: A   B     C
                    model  closed    proc   closed  proc    validity
frame reach    500     20     36       82      61     322        65    3.6  2.5   1.5
heatsink L     559     17     56       93     248     135       124    3.4  1.9   1.7
gearbox W      353     42    122       32      99      61         9    1.8  1.7   1.3
manifold pitch 262      7     25      103      19     118        10    1.9  1.8   1.1
crank len      337      8     35      233      38      34         2    1.2  1.2   1.2
volute core166 898      7     13      392      93     388       266    2.2  1.3   1.3
conrod b+t      52      3      7        6      17      18         0    3.4  3.4   3.3
```

A: every decision free, certificate included. B: validity still runs (a full rebuild fails on it, so no frame may
skip it without proof). C: and moved rejected pairs still run (no sound certificate found). Same-value pairs memoised on
top of C: frame 1.7, heatsink 1.8, gearbox 1.3, manifold 1.1, crank 1.3, volute 1.4, conrod 3.4.

## Procedural: warm start

`warmbits`, traced curves at two values matched by edge name (frame, manifold, crank, volute; heatsink, gearbox and
conrod have none moved):

```
                 Newton steps a node, old node / march predictor     one limit from all starts   a fixed point   spread
                 0.2% move      0.02% move
frame            2.28 / 2.01    1.96 / 2.01                           71/203                     137/203         8e-14 mm
manifold         2.26 / 1.97    1.99 / 1.97                           43/56                      44/56           2e-14
crank            2.21 / 1.77    1.51 / 1.77                           41/80                      60/80           6e-15
volute           1.00 / 2.11    1.00 / 2.11                            0/28                       7/28           3e-13
```

- No gain: a chain is already warm-started, each node from the one before. Its predictor is as good as last frame's node.
- Not the same bits: refine's system is underdetermined (a node may land anywhere along the curve), and with a station
  plane to make it square, Newton from different starts ends on different floats and often cycles. Full convergence
  plus a polish does not give one answer.
- Rounding to a grid (Ziv's test, refuse near a boundary) works in principle, but with a 3e-13 spread and ~5000 nodes
  a frame the grid has to be near 6e-8 mm for about 7% of frames to refuse. It also changes every full rebuild and
  adds a step per node there, against the +0.3% gate. Not tried in code.
- What can be exact without any of that: replaying search results, not values. A trace's seed is a cell centre, a
  subdivision's start a function of the piece, so the same Newton runs from the same start. It saves the search only,
  and the plan's validity test then has to show the search would pick the same one, which is the cost saved.

## Why the certificate is the wall

Most decision time is proving nothing happens: rejected procedural pairs (frame 217 Mi of `searched`, manifold 86, heatsink 60,
gearbox 49), edges tested and not cut. To skip one exactly, every predicate the full rebuild would run must be shown to
go the same way. Re-running them costs what was saved; the octree's pruned cells as a certificate saves about 1/8.
Margins need a sound bound on how far a procedural surface moved, which only rigid tools have.
