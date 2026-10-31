# Mesh cost of swept faces (pipes and variable blends)

How the meshing of swept faces (`Procedural::swept`: `Pipe`, `VarBlend`) was measured and
chosen, for `proc_strips` and `Nodes::want(.., shared)` in `crates/forge-mesh/src/tessellate.rs`.

## Rerun

    cargo build --release -p forge-model --example meshcost
    sh research/mesh-cost/cost.sh target/release/examples/meshcost out.txt \
        slant-hole-rims shaft-cross-hole shaft-post boss-rim-periodic boss-foot-periodic

Each line is instructions per mesh: `perf stat -e instructions:u` of `meshcost <case> <chord> 2`
less `meshcost <case> <chord> 0` (regeneration only), halved, at chords 0.01 and 0.25. Cases are
suite parts and the fillet `cases()` / `variable_cases()`. Below ~300k instructions per mesh the
count moves by up to 2% between runs; compare a build against itself first.

## What was chosen, and why

- A swept face square in its parameters is meshed as a grid of columns joined to its boundary by
  a band, every chord and triangle checked by the surface (`strays_from`), the Delaunay path when
  any fails. Every test point of the grid shares its u with a column (the chords' middles across,
  along and corner to corner, the triangles' middles at a third), so one section solve serves a
  whole column of tests.
- Steps share the tolerance between the two ways (half each, or the rest of it where one way is
  held by the largest angle): with each way at the whole tolerance a cell's diagonal sags by both
  and failed on every case at chord 0.01. Equal shares minimise the points for a given sag sum.
- Per cell the diagonal nearer the surface at the cell's middle: twisted cells of the variable
  blends failed along one diagonal and not the other.
- The ring's corners sit at the boundary's corners in the band's walk; placed by their own u or
  v, the corner triangles reached across a whole step and failed.

## Tried and dropped

- Rows at the side edges' samples (as for analytic faces): the sides of a pipe or blend can have
  fewer samples than its middle needs (the section's angle grows between its ends), so chords
  across failed at 0.01; and both ways at the whole tolerance failed the diagonals.
- A secant step along the chord before Gauss-Newton when a middle lies off its parameters'
  middle: with the shared tolerance almost no test needs it; it cost 0.5-2%.
- A cache of points by column for the Delaunay path (all procedural faces): it worked out grid
  points the Delaunay never keeps (loft +21% at 0.01) and its lookups cost more than a pipe's
  evaluation.
- The tube bound (spine curvature, reach) from mesh-speed-5's handoff, as the check for the strip
  triangles: evaluating the tests a column at a time made the checks cheap enough without it,
  and it held only for line, circle and ellipse spines.
