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
`--style shaded` (no lines). Each segment is 6 vertex invocations, most of which the vertex shader drops at once.

## Line drawing: a shared index pattern

Each segment was six vertex shader runs; with one index pattern over four vertices per segment the
GPU's vertex cache makes it four. The 5,000-placement bench at 2560x1600, no occlusion: p99 15.33 ms
drawn plain, 10.57 ms indexed (other changes to the pass being the same). Line width doesn't matter
(a feather of 0.4 or 0.75 pixels: 15.19 and 15.33 ms): the time is in the vertices.

## Line modes: near (default), far, none (now visual styles: `VisualStyle`, `--style`, key L)

Near draws edges and silhouettes on tessellated levels, none on clustered ones. Far also draws, on
clustered levels, the B-rep edges of the coarsest tessellated level: all of them, on the 5,000-placement
bench at 2560x1600, p99 21.86 ms. Polylines are stored longest first, so a level can draw just the
start of the list; keeping the edges at least `FAR_EDGE_PX` = 6 pixels long on screen (where a level's
chord is about a pixel) brings it to p99 14.41 ms. Near 11.72 ms, none 6.29 ms on the same run. Far at
that threshold wasn't compared by eye with others.

These became Inventor's visual styles (2026-11-04): Shaded with Edges is near, the wireframes draw far
(or they'd lose far parts entirely), Shaded and Realistic none. Hidden edges are a pass after the
scene that reads its depth: a fragment of an edge is hidden only if the depth a cross 2 pixels out is
all nearer (by 1e-3 of it), so where an edge's width spills over the face it bounds it isn't called
hidden; only B-rep edges that aren't smooth go in it (hidden silhouettes and tangent edges were noise).

## Occlusion: `AO_REACH` = 0.06 of the window's height (`render.rs`), `AO_STRENGTH`, `AO_BIAS` (`render.wgsl`)

Set by eye on heatsink (fins and the plate under them) and the stress scene's mid view, then timed.
A reach of 0.025 and 0.08 with a hard cut-off barely darkened the heatsink's plate between fins, since
what hides it is further than the reach; the softer fall-off out to about twice the reach fixed that.
Same bench: p99 9.65 ms without occlusion, 11.53 ms with, and 6.24 ms with occlusion but no lines.

## The browser: Chrome 150 on the RTX 5060, no canvas

A WebGPU canvas can't be shown on this GPU in Chrome here: a bare clear in plain JavaScript fails at
the canvas's shared image (`vkAllocateMemory` `VK_ERROR_OUT_OF_DEVICE_MEMORY`, `Error creating
wgpu::Texture`), headless or with a window, whichever `--use-angle` was tried. It isn't the renderer's
code. So `bench_offscreen` (`web.rs`, `&offscreen=WxH` in `web/index.html`) draws to a texture and
waits for the GPU each frame, as native headless does, in a separate Chrome on a temporary profile:

`__NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia __EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/10_nvidia.json __VK_LAYER_NV_optimus=NVIDIA_only google-chrome --user-data-dir=$(mktemp -d) --headless=new --enable-unsafe-webgpu --use-webgpu-power-preference=force-high-performance --enable-features=Vulkan --use-angle=vulkan 'URL?stress=5000&moving=2000&bench=600&offscreen=2560x1600&need=nvidia'`

`need=nvidia` draws nothing unless the adapter says so. The same scene and camera path, a pick each frame:

```
                          frame mean  p99   worst   gpu mean  gpu p99
browser 2560x1600 moving    7.80     12.30  180.50    5.90    10.34
browser 2560x1600 moving    7.75     11.90  169.90    5.92    10.51
native  2560x1600 moving    7.23     11.75   11.89    6.43    10.87
browser 1920x1080           4.69      7.20  166.50    3.19     5.27
native  1920x1080           4.08      6.85    7.28    3.51     5.97
browser crank 2560x1600     2.79      4.70   88.20    0.39     0.51
native  crank 2560x1600     0.76      1.00    2.79    0.41     0.56
```

The extra 0.5 to 2 ms a frame is the wait itself, which is a browser callback (see crank); GPU time is
the same. The worst frames weren't traced; a pick reads back one frame later here, so its time is a frame's.
