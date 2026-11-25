# Assembly overlap check speed (P1–P6)

Branch perf/asm-overlap-walks off main c135b274. Only forge-asm's overlap path changes, plus one
line in forge-lang's geomemo merge.

## P5: skipping pairs by a convex-hull gap. Measured, dropped.

`p5_hull_instrument.patch` on main, then `p5_run.py OUT`: every narrow-tested pair across the 48
example assemblies (47,963 pair tests, FORGECAD_CACHE=off). For each pair it records the exact
hull gap (GJK on the vertex sets), the narrow and near verdicts, and the time to re-run
narrow + distance. 13 docs exit 1, as on main (they report failed expects). 276 of the 47,963
lines were interleaved by sub-assembly checkers writing at the same time and are skipped.

| | pairs | time (narrow + distance) |
|---|---|---|
| all | 47,963 | 392.9 s |
| clear and not near | 25,353 | |
| hull gap > reach (prunable) | 5,426 | 3.6 s (0.9%) |
| hull test itself (uncached) | | 2.0 s |

Every prunable pair was Clear and not near (asserted). The pairs a hull could skip are already
cheap, because the BVH walk drops them near the root. The time is in pairs that come near each
other without touching on non-convex parts. Not worth a proof.

## P1+P3: one lazy walk (`narrow_near`)

There used to be two walks: narrow's crossing walk, and distance_kept's walk, used only for
`.is_some()`. Now one walk carries each test's own pruning flag down the pair tree, in each
test's own frame. Positions and node boxes are moved only when the walk reaches them. Both
answers equal the two walks' answers by construction: each test sees exactly the leaf pairs and
coordinates it saw before, and neither answer depends on the visiting order. The near test
stops at the first pair within reach. The old path is kept for one train behind
FORGECAD_ASM_TWO_WALKS=1.

The memo now keeps the near answer with the narrow verdict. Learned codes 3 and 4 are new; old
readers skip them. Old disk caches' code 0 still answers through their kept gaps.

- Adversarial tests (`one_walk_tests`): touching, nested, gaps of ±1e-12 around 0 and the depth,
  rotations, both mesh-size orders (swap and no swap), and a non-convex comb against a pin.
  Every case matches the two walks.
- Corpus (`corpus_equal.py`, 164 builds: 48 assemblies, their edited variants and the two thread
  repros): two-walk and one-walk `build --json` are identical with timings removed, 164 of 164.
- callgrind, frame/test (x86-64-v2, whole build): 29.42 G → 26.11 G Ir (−11.2%). The walks:
  14.0 G → 10.8 G (−23%). What remains: enclosed/near_surface 11.0 G (the P2 target), the BVH
  build's sort 1.0 G (the P4 target).

## P4: BVH by median selection

`select_nth_unstable_by` on the same total order (centre on the axis, then index). Each leaf is
then sorted in its parent's order. This builds the same tree and leaf order:
`bvh_select_tests` compares nodes, boxes and items with the sorted build on 141 box sets,
including heavy ties.
