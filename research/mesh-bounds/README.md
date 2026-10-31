# Mesh step sizes and the chord guarantee

`crates/forge-mesh/src/tessellate.rs` promises every triangle within the chord tolerance at its
centroid and edge midpoints (checked by `crates/forge-model/tests/mesh_quality.rs`).

## Derived (check.py, recreated 2026-11-01; the originals were worked by hand)

- `arc_step`: a chord over angle theta of radius r sags r(1 - cos(theta/2)), so the step is
  2 acos(1 - chord/r).
- Surfaces curved both ways (spheres, tori; 2120083): a triangle sags by its sags along both
  sides, so each direction gets half the tolerance; sag is quadratic in the step, so the step is
  shorter by sqrt 2 (`edge_segments` with `fine`, `interior_steps`).
- A triangle's middle on such a surface sags up to 4/3 of its worst side's middle (equality for
  the equilateral triangle; a5c87f0), which is why refinement tests triangle middles there.

## Measured

- Cylinder and cone rows two columns apart, not four (387a882): with four, refinement by a hole's
  rim left triangles 3.5 chords deep; with two, every cylinder triangle in the fillet corpus was
  within 0.89 chords, for 7% more triangles. To repeat: set the row factor in `interior_steps`
  back to 4.0 and run `fbuild cargo test --release -p forge-model --test mesh_quality -- --nocapture`
  (it reports the worst chord ratio); `meshcost <case> <chord> <times>` counts the triangles' cost.
- The 0.9 under the sqrt 2 grid step on spheres and tori: so a cell's diagonal does not sag by
  nearly the whole tolerance and get split; chosen by eye on the suite, no measurement kept.
- Procedural faces gridded per cell of an 8 x 8 split (`G`, 2120083) and refinement until nothing
  strays (a5c87f0; 24 rounds had left overshoots of up to 3.8 chords): instruction counts with
  `mesh1`/`meshcost` and the worst ratio from `mesh_quality`. The per-variant numbers were not
  kept beyond the commit messages.

## How to rerun

```
fbuild python3 research/mesh-bounds/check.py    # needs sympy; five "ok" lines
fbuild cargo test --release -p forge-model --test mesh_quality -- --nocapture
```
