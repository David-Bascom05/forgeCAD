# The read's checks: what each guards, what it costs, what could make it cheaper

David's ruling: a model is shown only once every check has run (no provisional view), and the work
goes into making the checks themselves cheaper or showing that one is not needed. This note takes
the checks one by one. Every result here is held to the bit-identical rule for imports (bodies,
faces, edges, tolerances, meshes, mass, every refusal and its words, the read's stats).

## Where a checked read's time goes

Per phase, summed over solids and threads (`cargo run --release -p forge-step --features times
--example steptimes FILE`; the `times` feature counts nothing unless built with it). CPU seconds,
4-core VM, import/speed-ks:

PHASE_TABLE

The checks (speck, deviations, mass, mesh check) are most of it. Three quarters of every phase's
instructions are B-spline surface evaluations (point + derivatives in a Newton projection: PROFILE.md),
and every one of those evaluations is what the read's budget counts (`forge_geom::budget::spend`
is called in one place, `forge_step::nurbs`'s evaluation). That ties every idea below to one rule:

**A check that does fewer surface evaluations changes what the read's budget counts, and so which
solids near the budget are refused as "its surfaces would take too long to measure".** A cheaper
check can only be bit-identical where no solid is near the edge; to be identical everywhere it must
evaluate the same surfaces at the same points the same number of times. So the identical-results
speedups are the ones already taken:

- each evaluation cheaper (degree-sized, derivative-specialised B-spline basis: the same floating
  point operations, 2x fewer instructions);
- the checks' independent pieces on all cores (speck per face, deviations per edge, mass per face
  and per loop piece, the check's mesh per face), with the budget charged what one by one charges;
- the check's mesh kept as the display mesh where it is the same computation (one mesh, not two).

What is left needs a decision on the budget's unit or on the checks' specification; the options
are below, check by check, with what each would save.

## The checks

### Speck (`laid_out_in_a_speck`) — refuses a solid

Guards: the exact predicates' floating-point filters. `orient2d`'s filter (and
`forge_math::predicates::check_filter`) holds for coordinates that are 0 or at least 2^-433 in size.
A face whose parameter loops hold a coordinate below that (a surface's frame 1e308 from its face
lays the face out over a span of 1e-308) would make a later boolean's predicates wrong without a
word. The check draws every face's loops as finely as `forge_topo::check` draws them (8x
`default_segments` per edge) and looks at every coordinate.

Cost: the largest single check (35% of main's instructions on VF-2; SPECK_ROW). Almost all of it is
B-spline projection of the loops' samples on free-form faces (`uv_loop` → `project_near`).

Cheaper exact filter? Looked for, not found in general:
- an analytic face's parameters are its points in the surface's frame (a plane's u is
  `(p - o)·x`); exact products and sums of numbers that are each 0 or at least 2^-k are 0 or at
  least 2^-(2k+104), so a certificate on the sample points' and frame's coordinates (each 0 or at
  least 2^-164) would prove no speck. But those faces are cheap already (no surface evaluations).
- a free-form face's parameters come from Newton steps (`uv - step`, then clamped to the
  domain): a step that lands at 1e-300 by the corner u = 0 of a [0, 1] knot domain is a speck, and
  there is no bound short of doing the projection. A domain that keeps away from 0 (both ends of
  one sign, at least 2^-433 from it) proves it for that direction, but knot domains are [0, 1]
  almost always.
- the samples are the same points every other check draws only if their counts agree; they don't
  (deviations: 16 or 256 per edge from the edge's own start; speck: 8x `default_segments`, chained
  round the loop).

Proposals (each changes what the budget counts, so verdicts at the edge move; under the rigor
rule):
1. **One projection pass for speck and deviations.** Both project edge samples onto each face
   the edge bounds. Sampling each edge once at the finer of the two counts, projecting once per
   face, and reading both the speck coordinates and the gaps from it removes one of the two passes
   (deviations: DEV_ROW). Deviations' values would change slightly (other samples, other Newton
   starts), so tolerances change in their last bits.
2. **Speck checked where it can matter**: at the predicates. The filters' preconditions are
   checked where a boolean feeds them (`check_filter` in debug only today); a release check there
   that falls back to exact arithmetic when a coordinate is tiny would make the read's speck check
   unnecessary (the import would no longer refuse such a solid: a behaviour change, to be agreed).

### Edge tolerances (`record_deviations`) — sets each edge's tolerance

Guards: a file's edges lie off their faces by up to the file's own accuracy; a boolean, the mesher
and `check` must know by how much (an edge's `tol`), or they see gaps that are not there. 16 points
per edge onto each face, 256 where free-form or off, and around the three worst peaks 32 more.

Cost: DEV_ROW. Needed: its numbers are part of the body. Cheaper exact? The 256 samples are fixed by
the specification; the projections chain from point to point already (`project_near`). Proposal 1
above is the one saving.

### Mass (`mass_rel` to 1e-6) — the solid's mass properties, and the volume check's reference

Guards: `Solid::mass` (volume, area, centroid, inertia) is part of the result, and it is what the
mesh is checked against. Cost: MASS_ROW; HAB3232 is the extreme (four free-form faces).

Done: faces on all cores; one face's integral split over its loop pieces on all cores (the same
shares summed in the same order); Chebyshev tile sums looking only at their slab. Cheaper exact?
The integrals' quadrature is fixed by the 1e-6 target and its error control; a different rule gives
other bits. Proposal: none bit-identical beyond what is done.

### Volume check (`mesh_agrees`) — a warning, never a refusal (except too long)

Guards: on a file's geometry the integrals can go wrong with nothing to show for it; a mesh at
5e-4 of the diagonal whose volume and area disagree with them by more than its chord explains marks
the body `unverified`. Cost: MESHCHECK_ROW, but it is the display mesh too (kept where it is the
same computation), so it is no extra work for a viewer.

Proof that it could go: none — its verdict (`Body::unverified`) is part of the result. It is the
cheapest of the four once the display mesh is counted.

## The budget's unit

Every proposal above moves verdicts at the budget's edge, because the budget counts surface
evaluations. KERNEL-STRESS is changing that unit already (approved by the coordinator): a lazy
cache's filling evaluations go uncounted (`budget::uncounted`), so an op's spend no longer depends on
what ran before it or in which thread order. That change also makes the parallel phases' prefix
rule exact at the edge. The proposals here would go through the same rigor process: a full-corpus
differential run with every moved verdict explained, oracles and bounds, adversarial tests, an
independent review, and the old path behind a switch for one train.

## Fully checked VF-2: where it stands

ALL_CHECKED_ROW
