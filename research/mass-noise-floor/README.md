# Mass stops refining a trace at the surface's noise floor

`kernel/forge-topo/src/mass.rs`, `refine`, `trace_miss`, `carry_miss`, `NOISE_MISS`.

## What was measured

An edge between two blends on Ruled walls, traced by mass: for each piece, how far the 12-point
interpolant of the trace misses the 36-point trace (`trace_miss`), by refinement depth (8 is the
first piece, 0 the last allowed). Median over 16713 pieces on a lofted-blade part, in the surface's
u and v:

    depth  pieces  width     miss u    miss v
    8      67      8.49e-01  1.44e-10  2.54e-11
    7      95      4.96e-01  1.76e-10  1.31e-11
    6      170     2.44e-01  4.83e-11  3.59e-12
    5      316     1.22e-01  1.36e-11  1.91e-12
    4      600     6.09e-02  9.95e-12  1.33e-12
    3      1141    2.65e-02  9.94e-12  1.11e-12
    2      2161    1.33e-02  1.15e-11  1.10e-12
    1      4104    6.64e-03  1.32e-11  1.18e-12
    0      8059    3.32e-03  1.37e-11  1.22e-12

A smooth trace's miss falls by a large factor per halving. Below depth 4 it does not move: that is
noise from the surface's own value, and `resolved`'s tolerance (1e-3 * rel of the span plus 64
epsilon) sits under it, so about 85% of the pieces were spent chasing it.

## The rule

A piece stops when, in both u and v, its miss is at least half its parent's and at most
`NOISE_MISS` (1e-9, two orders above the floor measured). A miss larger than that which does not
shrink may be the trace's shape, so it still refines.

A piece that stops unresolved, at this floor or at the depth limit, now carries its miss into the
error as a gap between edge and surface. Pieces that stopped at the depth limit carried nothing
before.

## Rerun

Log `trace_miss` per piece and depth in `refine` on a part whose blends meet on Ruled walls;
`kernel/forge-model/tests/mass_noise.rs` builds such a part with an analytic twin.
