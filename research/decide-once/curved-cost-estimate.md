# The curved builder's cost: measured, and estimated against today's curved boolean

The master asked for this on 2026-11-23, under the owner's rule that wall-clock is compared at equal
threads and the fastest wins. This page covers plate, patterns and housing, the parts where every
flat decide-once boolean is skipped as NotFlat.

## Today's curved boolean, measured

Method:
- `FORGE_CURVED_STATS=1 regen1 PART seq`, built with `forge-bool/decide_once_curved`. With the
  feature off, the hook is not compiled.
- Each boolean through `boolean_into` prints the operands' sizes and the old path's wall-clock
  (`imprinted`, then `general`, then `refuse_strays`).
- Each figure is the best of 3 runs, on one thread, on a 4-core container.
- A boolean is "curved" when a near pair of faces from different bodies includes a curved face.
- The figures are upper bounds: the stats work runs between booleans and cools the caches. For
  scale, the whole regen (best of 5, stats off) is 23.9 ms for plate seq, 30.2 ms for patterns seq
  and 26.3 ms for housing seq.

| part (seq) | booleans | curved | old path per curved boolean, mean / median / max | near pairs per curved boolean: plane–plane / plane–curved / curved–curved |
|---|---|---|---|---|
| plate | 220 | 212 | **0.144 / 0.102 / 0.913 ms** | 0.75 / 2.90 / 0.87 |
| patterns | 196 | 190 | **0.184 / 0.041 / 0.841 ms** | 5.24 / 3.99 / 0.63 |
| housing | 186 | 168 | **0.155 / 0.047 / 1.412 ms** | 1.90 / 3.48 / 0.83 |

Today's curved boolean costs about **0.15 ms**. The near faces per curved boolean are 4.7, 7.3 and
5.5 on average.

## The exact pieces, measured (release, one thread)

`tests/curved_cost.rs`, ignored by default:

| piece | wall-clock |
|---|---|
| plane–cylinder crossing curve (`pair_curve`: events, slabs) | 1.16 ms |
| its event nodes (`curve_nodes`, stage (b) part 2) | 2.64 ms |
| cylinder–cylinder crossing curve (cross hole, r 4 against r 3; 10 events, 11 slabs) | **549 ms** |
| its event nodes (from the stage (b) test) | tens of seconds to minutes |
| vertex of plane, plane, cylinder (`propose_triple`, tilted frame) | 0.90 ms |
| vertex of plane, cylinder, cylinder | 1.33 ms |
| vertex of three non-planar quadrics (earlier milestone) | 37–74 ms |

## Estimate: the builder as designed, exact on the common path

Per curved boolean (plate's means):
- 2.9 plane–curved pairs at about 3.8 ms each: about 11 ms.
- 0.87 curved–curved pairs at at least 0.55 s each: about 0.5 s, and seconds once the nodes are
  counted.
- Trim vertices at about 1 ms each: about 5–10 ms.

That is about **0.5–2 s per curved boolean against today's 0.15 ms: 3,000–10,000 times slower.**
Even with no curved–curved pair, the plane–curved work alone is about 20 ms, over 100 times
slower. That is far past the 2× bound, so **the exact-on-the-common-path design is withdrawn**, as
the owner's ruling (floats, proven through the IEEE 754 model) already requires.

Where the time goes:
- Resultants and subresultant chains over rationals whose size grows with every frame shear.
- Root isolation for every slab and every separator plane.
- `between` separators: bisection midpoints with growing denominators make each new sample plane
  dearer.

None of this is needed where floats with a proven error bound decide.

## Estimate: the float route (curved-builder-mapping.md)

The common path reuses today's float pieces: pair loop, face boxes, the closed-form plane–quadric
curves, marching traces, the arena. It adds proven-bound screens. The added work per curved
boolean is:
- one screened sign per (trace point, face) incidence, for crossings and inside-face tests,
  costing nanoseconds each;
- an angle screen at each vertex;
- a cycle-area screen per facet.

By analogy with flat decide-once (face_fast is 1.45M instructions per flat boolean today, about
0.6M estimated after its mapping, against the old 1.1M), the curved builder's own work should be
about **0.5–1×** today's curved boolean. The exact fallback runs only on what no screen decides:
true tangencies, coincident surfaces, a vertex where three surfaces meet within the bound. The
fallback costs are the table above. They are acceptable only at a rate of about 1 in 10³–10⁴
booleans, and the fallback rate will be measured on the gate lines before any claim is made.
