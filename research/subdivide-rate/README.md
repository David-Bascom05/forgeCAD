# Volute's regen after the smooth-loft knot change

The knots by the tenth-percentile chord (0515256a) made volute's regen +9.7% seq. Profiled with
`perf record --call-graph=fp` on a frame-pointer build of `benchparts run volute seq`, the old
knots switched back in a throwaway build: 1134.1M old, 1259.5M new; +121M of the +125M is the
boolean's trim, `curve_surface_in` -> `subdivide_pieces`, a traced curve against the casing's end
planes and cylinder that the new wall crosses more shallowly (pieces kept 17187 -> 31661, leaves
2687 -> 5568).

## Shipped: runs of chords boxed in `loftmeet::follow`

A seed was compared with every chord of every traced branch (6.7% of volute). Chords are boxed in
runs of 16; a run whose box is farther than the nearest chord so far, or than 1.5 steps, is passed
over. The nearest chord, and the first of equals, are the same, so nothing changes but the cost:
volute seq 1030.9M -> 972.5M (-5.7%).

## Dropped: a pruning bound by the rate along the normal

Bounding how fast a traced curve climbs a plane's (or cylinder's) normal, as the chain cubic's rate
there plus twice the sag, dropped pieces holding no leaf and took volute to -12%, bits unchanged.
It was pulled because the allowance for the step from the cubic onto both surfaces is not proved.
Measured on 5502 chain segments of traced sphere-cylinder curves, |P' - h'| reached 0.063 sag and the
speed 0.66 of the speed bound (1.5 chord + 2 sag) `chain_pieces` already prunes by: both hold there,
neither is proved, and the speed bound rests on the same step. Next: certify each chain segment
(interval Newton over a box) so both bounds are sound, then bring the rate bound back on that.
