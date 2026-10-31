# Threads: why they are slow everywhere, and the exact forms that fix them here

Modelled threads are a known pain in every CAD system. This records what others do, what we
measured on our own path, the math that replaces the search, and where the code uses it.

## What others do

- **Most tools default to cosmetic threads**, because the geometry is expensive: Onshape's own tip
  says each modelled feature adds faces and edges that bog a model down
  ([tech tip](https://www.onshape.com/it/resource-center/tech-tips/tech-tip-to-model-or-not-why-cosmetic-threads-are-usually-the-better-choice)).
  SolidWorks 2017, measured by a reseller: the Thread feature is 96% of a small part's rebuild
  (0.38 s of 0.39 s), a swept thread 1.01 s, a cosmetic one nothing
  ([GoEngineer](https://www.goengineer.com/en-us/blog/solidworks-performance-physical-cosmetic-threads)).
  Its "simplified" advice is to suppress the thread. A PTC forum guess puts the slowness in "the
  math of resolving helical intersections": on our path that guess was right.
- **Kernels sweep a B-spline along a helix**: Bentley's helical surface is a B-spline to a tolerance
  ([docs](https://docs.bentley.com/LiveContent/web/Promis.e%20Help-v9/en/CreateSurfacesConstructHelicalSurface.html));
  STEP has no helicoid, so threads arrive as B-spline faces of 100+ poles
  ([solidcraft #7074](https://github.com/tangency-labs/solidcraft/issues/7074) shows what that does
  to trimming). I found no public statement of how Parasolid, ACIS or CGM represent a thread's
  flank; they are closed. Treat that as unknown, not as evidence.
- **OCCT is the one we can measure** and it is slow and silently wrong: a cut of a bored block with a
  helical sweep returns two solids, one inverted, with no error, for 16-17 of 180 M4-M10 depths
  ([OCCT #1629](https://github.com/Open-Cascade-SAS/OCCT/issues/1629)); a cut can return the input
  unchanged ([OCCT #1543](https://github.com/Open-Cascade-SAS/OCCT/issues/1543)). An independent
  port reports 1 s, 2 s, then failure at 7 turns
  ([ogeom-rs #56](https://github.com/gilbertorconde/ogeom-rs/issues/56)). Our own oracle already
  found a groove cut 336 mm^3 where Monte Carlo says 231 (`tools/occt/build.py`).
- **The math is old and unused for this**: a surface invariant under a screw motion is a helicoidal
  surface ([Generalized helicoid](https://en.wikipedia.org/wiki/Generalized_helicoid)), studied
  through line geometry and kinematic surfaces in Pottmann & Wallner, *Computational Line Geometry*
  (Springer 2001), ch. 4 and 8, and
  [Pottmann, Hofer, Odehnal, Wallner 2004](https://link.springer.com/978-3-540-24670-1_23), which
  recognise such surfaces from their normals; none of them works out kernel intersections. Helical
  projection ([*On Helical Projection and Its Application in Screw Modeling*](https://www.researchgate.net/publication/273489202_On_Helical_Projection_and_Its_Application_in_Screw_Modeling)) reduces a
  screw to a planar closed curve: the same reduction as here, used for modelling. A helicoid cut by
  a coaxial cylinder is a helix and by a cone a conical helix
  ([TU Wien notes](https://www.geometrie.tuwien.ac.at/vis/vis222.html)). I found no published
  boolean that exploits the invariance. SolidWorks' "geometry pattern" and a revolve-and-pattern
  thread are the nearest practice.

What to beat: seconds per thread, silent failures, and a default that tells users not to model them.

## Baseline (main ad33439)

Instructions:u by prefix difference (`research/feature-costs`), `bench-baseline-seq.tsv`,
`stress-baseline-*.tsv`. The 13 real threads cost 2621M of a 13607M bench, 2947M with what they
leave later features. OCCT on `stress/` (`tools/occt/oracle.py`, chord 0.01): ours and OCCT's volume
and area agree on all but two; `fine-long-ext` (200 turns) OCCT takes 166.8 s against our 305 ms,
and disagrees by 1.6e-7 of the volume (1.6x the mesh's bound); `fine-long-int` OCCT times out at 200 s.
`taper-runoff` failed on main ("face split crossed itself"): a tapered groove running off its rod.

## The math (`derive.py` checks it)

In the half plane of a screw surface's turn k, u = phi + 2 pi k, a point reads (rho - a u, z - b u).
A helix, a coaxial circle or a line in an axial plane has rho, phi and z all linear in its parameter
t, so it reads as a point moving **linearly** in t, the turns shifted by 2 pi (a, b):

    q_k(t) = q_0 + v t - 2 pi k (a, b),   v = (a2 - a w, b2 - b w)

The surface shows a fixed section, so each pair is a low-degree solve, and only the turns that can
reach the section are visited: s_k(t) = s_0(t) - step k solves for k directly, O(hits) not O(turns).
A helix of the surface's own motion has v = 0: it never crosses, whatever the turn count. That one
case was 67% of the old search's work. Against any plane a cylinder's helix is
f(t) = A cos(t - th) + G t + D, whose turning points are closed form, with one root between each.

## Where the time went, and what each step won (real threads, instructions)

Old: `curve_surface_subdivide` 66-91% of a thread. Its evaluations by pair: helix x fence 66.9%,
coaxial circle x screw 11.9%, helix x plane/cylinder/cone 13.5%, off-axis plane 3.2%, line x fence 2.9%.
`Fence::value` looped over every turn on every evaluation.

1. `screw/along.rs`: the closed forms above; anything within a few tolerances of an end, a turn's
   end or another meeting is searched for in just that stretch, so a touch stays what the search
   calls one. A run along a section longer than the search's leaf budget is `Overlap` at once.
2. `Fence::value`: nearest turn in O(1), bit-identical.
3. `Surface::singular` skips the normal on a screw face (never singular off the axis).
4. `nary.rs`: an untouched face keeps its box. Every n-ary pass was recomputing every procedural
   face's box: this one line takes heatsink from 2368M to 1423M, and it is not thread-specific.
5. `screw::misses`: an oblique tube that plainly misses the thread's shell (frame's bottom bracket):
   no tracing at all. Bound derived in `derive.py` section 6.

6. `Body::face_box`: on a face straight across in v (a screw face with a straight section, a ruled
   face), a point lies between two boundary points at its u, so the face is in the hull of its
   edges' boxes: no uv drawing, no column boxes. Tight boxes exposed that traced curves ended short of
   the box's sides, so `meet_region` grows the tracer's region by 0.2% for these faces (`fuzz` t00001).
7. `Screw::nearest`: a point of the surface reads its parameters straight off the turn's half plane.

8. `Procedural::implicit_gradient`: one screw `value` for both (the tracer asked twice); the tracer's
   octree tests the cheap surface first.

Tried and dropped:
- A ray along the thread's axis in `classify`: no win once the rest was in, the rays were not the cost.
- The swept section (`meet_general`) before the tracer: "inconsistent classification" on cross holes.
  Roots moving fast along a section are born and die between rows (on a radial flat near the angle
  where the section runs along the cross hole's axis), so whole branches are missed.
- Closed-form seeds for the tracer instead of its octree (rows of section lines, the two rim helices):
  a further 30% on cross holes, but 35 of 400 fuzz runs got worse. The curves it did not seed near were
  on the carried-on track at a turn past the face, off the face; the failure itself was not found.
- A strip mesher for screw faces: the twist of a straight-section flank leaves a gap |dr| du / 4 in a
  quad, above the chord at default quality at the edges' spacing, and the side edges fix the rows.

Where the limit is: a cross hole through a thread still costs what tracing a screw face costs, about
1000 instructions per screw evaluation; 1.6x so far. After the regen, on threaded parts mesh is now
5x the regen. Both are the next things to attack, and neither is a thread feature's own cost.

## Results

Against main ad33439 (`bench-baseline-seq.tsv`), instructions:u, thread feature alone:
```
part#feature     seq old     new
conrod#22            79.5    14.4
crank#13            252.1    10.7
frame#126           698.4    17.2
frame#127           706.2    14.5
frame#138            27.4     3.4
gearbox#33           95.6    22.2
gearbox#53           28.5     3.6
jack#11             158.5    32.2
valve#14            128.0    28.1
valve#30            121.1    12.0
valve#31            174.7    11.7
valve#32            103.1     4.1
valve#33             48.2     4.1
sum                  2621     177   (14.8x)
```
Bench of 15 parts: 13607M to 8240M. Long fine threads (200 turns) 73-77x, and cost per turn is now
flat where it was quadratic. `volume_check.py`: two more turns remove two turns of the form by the
exact mass properties, rel error at most 3.5e-7 on every size (printed volumes have 6 decimals),
which also says OCCT, not us, is off on `fine-long-ext`.

## How it is checked

- `along.rs` test: 500 random groove faces against helices, circles, axial and through-axis lines,
  planes, cylinders, cones; exact on, within and just past a tolerance; ranges ending on a hit.
  Every answer against the search. Rules it had to learn, each a real finding: past its budget the
  search drops true touches (the closed form keeps them); where a turn ends the implicit jumps and
  the search's Lipschitz bound can lose a crossing beside it (the closed form does not).
- `oracle_patch.py`: applied temporarily, compares every closed-form answer inside real regens.
- `fuzz/gen.py`, `fuzz/compare.py`: random threaded parts on main's binary and this one, seq and
  n-ary 2. Found a real bug in the first version (the edge box `near` lost when a closed form
  declined) that no unit test had. Final: 1000 parts, 2000 runs, 0 worse; 11 runs fail on both.
- `forge-proof`'s `meta` on 120 of those parts, each: main fails `ulp` on one, this on none; the first
  version of the box change failed a boolean identity (a screw face against itself reached the tracer,
  whose answer depended on whether a seed landed exactly on it: `meet_screw` now says Empty).

## Lean (`lean/`, no sorry, standard axioms only)

Theorem by theorem, what the code does, and what was not needed:
- `section_along_line`, `own_motion_is_flat`: the reading is linear in t with a fixed step per turn.
- `linear_skip` (needs the same side at both ends; not b != 0, not m >= 0), `linear_root`: the skip
  and the clear crossing in `linear`.
- `miss_margin`, `margin_budget`: a computed value past 1.6 tolerances means the exact one is past the
  search's 1.5, given error at most tol / 10, and what `coaxial` admits plus rounding fits at model
  size. That budget is why `coaxial` tightened from 1e-12 rad to 1e-13: 1e-12 over 1e6 mm does not fit.
- `arc_square`, `arc_skip`, `arc_roots`: the arc pieces.
- `slab`, `helix_plane_deriv`, `one_root_between` (needs only f' != 0 inside; not A > 0, not |G| < A),
  `turning_points`: the helix against any plane.
- `turns_covered` (needs step != 0): no turn left out. `nearest_turn`, `nearest_turn_clamped` (needs
  step != 0): the fence's two candidates, as clamped in the code.
- `between_boundary`, `column_ends`, `face_in_hull_of_boundary`: the face box. Needs only a compact
  uv region and affinity in v: not connectedness, not simple connectivity, not a screw at all.
- `bessel_two`, `shell_reading`: the plain miss. Needs e square to z and g > 0.
- `Counter.lean`: why the needed ones are needed.

Not proved: that the search and the closed form agree inside a stretch (they run the same search),
and the first `misses` test's triangle bound (one line of triangle inequality).

## Rerun

```
~/laya/.venv/bin/python -I research/thread-math/derive.py
cp research/feature-costs/featcost.rs interface/forge-agent/examples/   # not kept in examples
fbuild cargo build --release -p forge-agent --example featcost
python3 research/thread-math/fuzz/gen.py DIR 1 200
python3 research/thread-math/fuzz/compare.py OLD_BIN NEW_BIN DIR
python3 research/feature-costs/measure.py BIN research/thread-math/stress out.tsv seq 4 '^bench/'
cd research/thread-math/lean; ln -s .../packages .lake/packages; lake build    # mathlib as in loft-ssi/shipped
```
