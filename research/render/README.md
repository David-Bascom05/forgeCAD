# Renderer measurements

Where the numbers in `interface/forge-render` come from. Measured 2026-10-31 on the Legion Pro 5:
NVIDIA GeForce RTX 5060 Laptop (8 GB), wgpu 30.0.1 on Vulkan, driver as installed.

## The level-of-detail threshold: `LOD_PX` = 1 (`render.rs`)

A placement takes the coarsest level whose chord spans at most this many pixels at its nearest
point. `lod_sweep.sh` tries 0.125 to 8 on the 5,000-placement scene at 1920x1080 and `compare.py`
reads the results. RMS and off>16 compare frames without lines against the finest threshold, so
they measure the shapes alone; tris and segments are the frame with lines on.

```
view threshold  rms  off>16  tris      segments
far  0.125   0.00   0.00%  35958366  42470938
far   0.25   2.00   0.39%  29651304  35875135
far    0.5   2.44   0.51%  13569099   1331910
far      1   2.57   0.58%  11521197         0
far      2   3.19   0.91%   9132258         0
far      4   5.45   2.51%   5844011         0
far      8   6.37   3.28%   5190199         0
mid  0.125   0.00   0.00%  25460382  27451062
mid   0.25   1.80   0.25%  18521160  21173806
mid    0.5   3.17   0.84%  13809230  16332404
mid      1   4.31   1.42%   7843157   5108467
mid      2   5.17   1.88%   5624103    778593
mid      4   5.68   2.38%   4246022    277253
mid      8   7.65   3.86%   3084585    113142

bench  0.125: mean 22.58 p99 37.84 worst 38.42 ms
bench   0.25: mean 17.93 p99 30.70 worst 31.16 ms
bench    0.5: mean 8.95 p99 19.55 worst 20.26 ms
bench      1: mean 3.90 p99 6.54 worst 7.08 ms
bench      2: mean 2.06 p99 3.20 worst 3.87 ms
bench      4: mean 1.34 p99 2.04 worst 2.92 ms
bench      8: mean 1.05 p99 1.54 worst 2.60 ms
```

0.5 misses 16.6 ms. 1 is the finest that holds it, with room for a 2560x1600 panel. 2 halves the
frame again but drops lines from most of the mid view (5.1 M segments to 0.8 M), since clustered
levels draw none.

Rerun, from `interface/forge-render` after `cargo build --release`:
`sh ../../research/render/lod_sweep.sh OUTDIR` (about 2 minutes, GPU busy, no window).

## The level ladder: `CHORDS`, `CLUSTER_STEP`, `CLUSTER_END` (`tess.rs`)

`forge-render --stress 10 --levels` prints it: each part's radius, then chord:triangles/segments per
level. The kernel's tessellation stops getting smaller at a part's topology. Triangles from a first
probe, with the turn at 15 degrees up to chord 0.05 and 90 past it:

```
heatsink  chord 0.01 60230   chord 0.05 58212   chord 0.2 24822   chord 1 23842   chord 5 23842
gearbox   chord 0.01 66336   chord 0.05 24828   chord 0.2 11244   chord 1  6578   chord 5  5210
```

So a part on screen at 20 pixels would still cost its whole topology. Past 1 mm the levels cluster
vertices instead, three times coarser each, until the chord is half the radius. Chord 5 was also dropped
because valve's face 51 fails to mesh there (`ConstraintsIntersect`); `build_part` skips any level
the kernel can't mesh. The 3x step and the 0.5 end weren't tuned: forcing the gearbox onto a clustered
level at 12 and 50 times the threshold (`--lod-px`) still gave the right shape with no holes, so
there was no reason to look further.

The chords' largest turns (15, 22.5, 45, 60 degrees) let the coarse levels shrink on large round
faces, where the default 15 degrees sets the triangle count, not the chord.

## MSAA: `SAMPLES` = 4 (`render.rs`)

Not measured against others: 1 and 4 are the only counts WebGPU guarantees.

## Lines

The fins (mesh edges that can turn into a silhouette) and B-rep edges cost about half of a heavy
frame: the 5,000-placement bench at 1920x1080 has p99 6.25 ms with lines and 3.95 ms with
`--lines none`. Each segment is 6 vertex invocations, most of which the vertex shader drops at once.

## Line drawing: a shared index pattern

Each segment was six vertex shader runs; with one index pattern over four vertices per segment the
GPU's vertex cache makes it four. The 5,000-placement bench at 2560x1600, no occlusion: p99 15.33 ms
drawn plain, 10.57 ms indexed (other changes to the pass being the same). Line width doesn't matter
(a feather of 0.4 or 0.75 pixels: 15.19 and 15.33 ms): the time is in the vertices.

## Line modes: near (default), far, none (`LineMode`, `--lines`, key L)

Near draws edges and silhouettes on tessellated levels, none on clustered ones. Far also draws, on
clustered levels, the B-rep edges of the coarsest tessellated level: all of them, on the 5,000-placement
bench at 2560x1600, p99 21.86 ms. Polylines are stored longest first, so a level can draw just the
start of the list; keeping the edges at least `FAR_EDGE_PX` = 6 pixels long on screen (where a level's
chord is about a pixel) brings it to p99 14.41 ms. Near 11.72 ms, none 6.29 ms on the same run. Far at
that threshold wasn't compared by eye with others.

## Occlusion: `AO_REACH` = 0.06 of the window's height (`render.rs`), `AO_STRENGTH`, `AO_BIAS` (`render.wgsl`)

Set by eye on heatsink (fins and the plate under them) and the stress scene's mid view, then timed.
A reach of 0.025 and 0.08 with a hard cut-off barely darkened the heatsink's plate between fins, since
what hides it is further than the reach; the softer fall-off out to about twice the reach fixed that.
Same bench: p99 9.65 ms without occlusion, 11.53 ms with, and 6.24 ms with occlusion but no lines.
