# The read's checks: what each guards, what it costs, what could make it cheaper

David's ruling: a model is shown only once every check has run (no provisional view), and the work
goes into making the checks themselves cheaper or showing that one is not needed. This note takes
the checks one by one. Every result here is held to the bit-identical rule for imports (bodies,
faces, edges, tolerances, meshes, mass, every refusal and its words, the read's stats).

## Where a checked read's time goes

Per phase, summed over solids and threads (`cargo run --release -p forge-step --features times
--example steptimes FILE`; the `times` feature counts nothing unless built with it). CPU seconds,
4-core VM, import/speed-ks:

Wall seconds for the whole read (file on disk to every check done and every solid drawn, one pass:
`forge_step::import_drawn`), the app's own path (`model_files::read`, at its 16x budget), and then
CPU seconds per check summed over the file's solids and threads. 4 cores of the cloud VM, nothing
else running, import/speed-ks.

| half | file | all s | app s | heal | speck | devs | mass | mesh check |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| tune | VF-2 | 2.28 | 2.19 | 0.8 | 1.8 | 1.3 | 1.5 | 1.2 |
| tune | GM2-5AX lightweight | 3.16 | 3.12 | 0.9 | 1.5 | 1.0 | 3.1 | 1.4 |
| tune | VF-6/50 TR | 4.59 | 4.66 | 2.4 | 3.3 | 2.5 | 3.7 | 1.6 |
| tune | ST-10/ST-15 | 6.90 | 6.94 | 2.5 | 4.5 | 5.9 | 7.7 | 2.2 |
| tune | DS-30Y | 4.75 | 3.78 | 1.8 | 3.3 | 3.3 | 3.3 | 2.9 |
| tune | RPL UMC-750 cell | 13.22 | 13.21 | 5.2 | 11.4 | 9.2 | 15.1 | 3.3 |
| tune | UMC-500 + 20 PP | 4.49 | 4.66 | 1.8 | 2.9 | 2.2 | 3.6 | 2.5 |
| tune | TRT160 | 1.24 | 1.27 | 0.6 | 0.8 | 0.4 | 0.7 | 0.6 |
| tune | HAB3232 | 13.03 | 12.28 | 1.6 | 3.7 | 3.0 | 18.0 | 0.4 |
| tune | ds-40y | 3.24 | 2.96 | 0.8 | 1.7 | 1.1 | 1.7 | 2.2 |
| held-out | VM-6 | 5.69 | 5.73 | 3.2 | 3.7 | 3.1 | 6.4 | 1.9 |
| held-out | VF-5/40 SS TR | 6.52 | 9.83 | 1.9 | 3.6 | 3.0 | 3.5 | 1.8 |
| held-out | ST-28/28Y | 5.74 | 5.92 | 2.2 | 3.2 | 5.0 | 4.0 | 2.6 |
| held-out | ST-20LY/25LY | 5.32 | 3.17 | 1.4 | 4.3 | 2.5 | 3.2 | 6.9 |
| held-out | UMC-1250 | 3.55 | 3.53 | 1.3 | 3.8 | 3.0 | 3.2 | 1.3 |
| held-out | GM-1-5AX | 2.83 | 3.24 | 1.4 | 2.1 | 1.6 | 2.4 | 1.9 |
| held-out | HRT160SS | 0.67 | 0.69 | 0.2 | 0.8 | 0.3 | 0.5 | 0.4 |
| held-out | TRT210 | 0.61 | 0.60 | 0.2 | 0.6 | 0.3 | 0.4 | 0.2 |
| held-out | Haas 5kg cobot | 9.17 | 9.41 | 2.5 | 4.7 | 3.3 | 14.4 | 7.4 |
| held-out | HFL-3015 | 1.49 | 1.48 | 0.5 | 0.5 | 0.3 | 0.7 | 1.8 |

Medians and worsts: tune 4.54 s / 13.22 s, held-out 4.44 s / 9.17 s (the app's path 4.22 / 13.21
and 3.39 / 9.83). Shares of all the CPU time of the twenty: mass 31%, speck 20%, edge tolerances
17%, mesh check 14%, heal 11%, topology 4%, display mesh 2%, parse 1%. Two of the three slowest
files are one check each (HAB3232: 18.0 s of mass of 13.0 s wall; the RPL UMC-750 cell: 15.1 s of
mass and 11.4 s of speck), so the way to a 1 s median is cheaper checks, not more cores.

(The `times` feature's own clock reads cost as much as the read itself at first; they are read only
while counting since baa63336, and every figure here is from the fixed build.)

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

Cost: the largest single check (35% of main's instructions on VF-2; 20% of the twenty files' CPU time, 11.4 s of the RPL UMC-750 cell alone). Almost all of it is
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
   (deviations: 17% of the twenty files' CPU time). Deviations' values would change slightly (other samples, other Newton
   starts), so tolerances change in their last bits.
2. **Speck checked where it can matter**: at the predicates. The filters' preconditions are
   checked where a boolean feeds them (`check_filter` in debug only today); a release check there
   that falls back to exact arithmetic when a coordinate is tiny would make the read's speck check
   unnecessary (the import would no longer refuse such a solid: a behaviour change, to be agreed).

### Edge tolerances (`record_deviations`) — sets each edge's tolerance

Guards: a file's edges lie off their faces by up to the file's own accuracy; a boolean, the mesher
and `check` must know by how much (an edge's `tol`), or they see gaps that are not there. 16 points
per edge onto each face, 256 where free-form or off, and around the three worst peaks 32 more.

Cost: 17% of the twenty files' CPU time. Needed: its numbers are part of the body. Cheaper exact? The 256 samples are fixed by
the specification; the projections chain from point to point already (`project_near`). Proposal 1
above is the one saving.

### Mass (`mass_rel` to 1e-6) — the solid's mass properties, and the volume check's reference

Guards: `Solid::mass` (volume, area, centroid, inertia) is part of the result, and it is what the
mesh is checked against. Cost: 31% of the twenty files' CPU time, and 18.0 s of HAB3232's 13.0 s wall; HAB3232 is the extreme (four free-form faces).

Done: faces on all cores; one face's integral split over its loop pieces on all cores (the same
shares summed in the same order); Chebyshev tile sums looking only at their slab. Cheaper exact?
The integrals' quadrature is fixed by the 1e-6 target and its error control; a different rule gives
other bits. Proposal: none bit-identical beyond what is done.

### Volume check (`mesh_agrees`) — a warning, never a refusal (except too long)

Guards: on a file's geometry the integrals can go wrong with nothing to show for it; a mesh at
5e-4 of the diagonal whose volume and area disagree with them by more than its chord explains marks
the body `unverified`. Cost: 14% of the twenty files' CPU time, but it is the display mesh too (kept where it is the
same computation), so it is no extra work for a viewer.

Proof that it could go: none — its verdict (`Body::unverified`) is part of the result. It is the
cheapest of the four once the display mesh is counted.

### A face's own budget (`forge_mesh::tessellate::face_within`)

Not a check of the file, but a bound on one: no face of a file's body may spend more than
`FACE` = 2^27 evaluations of a drawing's budget, and never more than that budget has left. A face
that runs its own out ends the drawing, with everything the drawing had left charged, so its caller
refuses the solid as "too long to measure" exactly as it does today when a face runs the budget out
itself. Deterministic, in the budget's own unit, with no clock, so the verdict is the same on every
machine and whatever order the faces are drawn in.

Why: the app reads display models at 16 times a read's budget, and one face of HRT160-2 (#42930)
spent 536 M evaluations and then failed — four minutes of a read of a file the kernel otherwise
reads in 0.9 s. With the cap that read takes 53 s, with the same result. The constant: over 415,000
faces drawn across the corpus files at that 16x budget, the heaviest face that actually draws spends
75 M, so the margin is about 1.8x. It will grow when a lazy cache's fills stop being charged
(KERNEL-STRESS's `uncounted`), since a face's own spend falls.

## The budget's unit

Every proposal above moves verdicts at the budget's edge, because the budget counts surface
evaluations. KERNEL-STRESS is changing that unit already (approved by the coordinator): a lazy
cache's filling evaluations go uncounted (`budget::uncounted`), so an op's spend no longer depends on
what ran before it or in which thread order. That change also makes the parallel phases' prefix
rule exact at the edge. The proposals here would go through the same rigor process: a full-corpus
differential run with every moved verdict explained, oracles and bounds, adversarial tests, an
independent review, and the old path behind a switch for one train.

## Fully checked VF-2: where it stands

2.28 s on 4 cores (main: 11.2 s), of which the checks are about 1.9 s of the 6.7 CPU seconds. The
1 s line is not met for a fully checked VF-2, and cannot be by parallelism alone on 4 cores: the
read's work would have to fall by about half. The mass integrals and the speck check are where
that half is.

A per-face budget for the mesher (2^27 evaluations, charged to the read's own budget, a face that
runs it out ending the drawing as the budget running out does today) stops one bad face from eating
a whole read: HRT160-2's #42930 spent 536 M evaluations on one face, four minutes of the app's read,
where the heaviest face that is actually drawn anywhere in the corpus spends 75 M.
