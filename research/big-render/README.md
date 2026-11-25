# Big models: frame time after load

Whether a massive model, once open, keeps every frame within 16.7 ms (p99), and the one place
it did not. Measured 2026-11-25 on branch `render/big-models` (from main 50af9c42) in a cloud
container: 4 vCPU, **no GPU** (Vulkan through lavapipe, Mesa's software rasterizer). So the
numbers here are the CPU's share of each frame, the part ForgeCAD's own code spends; the GPU's
share has to be read on a real GPU (below).

## Running it

From the checkout root, after `cargo build --release --bin forge-render --bin forge-app` in
`interface/forge-render`:

```
sh research/big-render/bench.sh OUT path/to/A.step path/to/B.step ..   # the viewport, per interaction
sh research/big-render/app.sh path/to/A.step [WxH]                     # the app, an assembly holding it
```

- `bench.sh` opens each STEP file as `forge-render FILE.step --bench`: every solid meshed as an
  assembly's part is (`tess::build_part_coarse`), placed where the file puts it, then 240 frames
  of a fixed orbit under each interaction: `orbit`, `hover` (a GPU pick under a wandering pixel
  every frame), `section` (x), `explode` (1), `wire` (wireframe), `moving` (8 bodies swinging every
  frame, as a simulation's do). Per run it prints `cpu submit` (p50/p95/p99 of the frame up to
  its hand-off to the GPU, and frames over 16.7 ms), `headless ... frame` (the whole frame with
  the GPU's wait) and, with hover, `pick cpu`.
- `app.sh` opens `fg/asm_big.fg` (a plate beside the model) headless in forge-app and runs an
  orbit drag, a hover, a click and two parameter edits with `--frame-log`: each step's UI-thread
  frames, p50/p95/p99 of the whole frame and of its `ui`, `tess`, `view` and `draw` parts. The
  app's own steps are in `forge-app --help`; `--frame-log` and `--set PARAM VALUE` are new.

**On a real GPU** (David's RTX 5060 laptop, say) read the `headless ... frame p99` lines of
`bench.sh` and the `frame` lines of `app.sh`: those include the GPU. On lavapipe they don't mean
anything: a redraw of even the small `bracket` part spends 20 ms in `draw` at 1600x900 there,
where the wait for the software rasterizer lands (`queue.submit`).

## The corpus

IMPORT-SPEED's corpus (research/import-speed/CORPUS.md) was not out when this ran; these are the
biggest Haas models from the `haas-3d-models` branch:

| model | file | solids | faces | triangles (finest) | read |
|---|---|---|---|---|---|
| VF-2 | VF-2_STEP__1_2023.step, 59 MB | 27 | 9,500 | 265k | 10.0 s |
| UMC-500 20-station PP | UMC-500 w 20 Station PP_3_2026.STEP, 56 MB | 105 | 16,471 | 332k | 11.8 s |
| ST-10/15 | st-10_st-15_solid_models_10_2023.STEP, 201 MB | 184 | 48,165 | 700k | 33.8 s |
| VF-2TR | VF-2TR_STEP__1_2023.STEP, 92 MB | does not read: `Parse(... line 135619: an instance number that is empty or over 32 bits)` | | | |

## Baseline: the viewport (`bench.sh`), CPU share per frame

p99 in ms over 240 frames at 1920x1080; every run had 0 frames over 16.7 ms. The p50s are
0.24-0.50 ms throughout. The VF-2 and UMC-500 p99s above 2 ms (and ST-10's explode and moving)
ran beside a build at low priority; they are noise on a 4-core box, not load.

| model | orbit | hover (pick cpu) | section | explode | wire | moving 8 |
|---|---|---|---|---|---|---|
| VF-2 | 0.40 | 2.04 (1.45) | 1.39 | 4.37 | 4.44 | 3.84 |
| UMC-500 | 6.15 | 6.66 (5.28) | 5.09 | 3.91 | 4.44 | 5.62 |
| ST-10/15 | 0.48 | 0.81 (0.37) | 0.52 | 4.57 | 0.44 | 4.74 |

The CPU's work per frame (cull, upload of poses, encode) is about 0.3 ms: 53 draws for the VF-2,
the picks on the GPU (the ID buffer), level of detail and culling per solid already in place.
The GPU's load is 0.27-0.70 M triangles and as many edge segments, against the 11.5 M triangles
of the 5,000-placement stress scene that holds 6.5 ms p99 on the RTX 5060 (research/render).

## Baseline: in the app (`app.sh`), the VF-2 in an assembly

p99 ms of the UI thread's frame parts (the drag, hover and click at 1600x900, the edit at
960x540):

| step | ui | tess | view | edit lands |
|---|---|---|---|---|
| orbit drag | 0.64 | 4.23 | 0.30 | |
| hover, click | 0.62 | 0.20 | 0.29 | |
| parameter edit, rebuilding (1,530 frames) | 0.40 | 0.19 | 0.00 | **one frame of 222-236 ms** |

The rebuild runs on the worker, but when it landed, the assembly's restage cloned every mesh
(10 ms for the VF-2's 22 MB) and made a whole new `Renderer` on the UI thread (156 ms: about 120
ms of pipelines, 33 ms packing and uploading every mesh), then the next submit took 65 ms more:
one hitch per edit, growing with the import.

## The fix (d77538d9)

- The view's renderer is restaged in place (`Renderer::restage`): new placement buffers and
  the settings `new` starts from, the same pipelines and targets.
- An assembly's meshes only grow (`fg::AsmMeshes`), so they carry a lineage. The renderer made
  at open (on the worker) gets buffers with room to grow, and each later restage packs and writes
  only the new meshes (`grow_parts`), by `queue.write_buffer` into that room.
- The meshes are moved, not cloned, to the UI thread. Any other case (meshes not of that lineage,
  no view yet) takes the old path.

Before and after on the same box, the same script (`--set L 420`, `--set T 25`, a click, an edit
with a part selected), p99 of the frame the edit lands in:

| assembly | ui before | ui after | frame before | frame after |
|---|---|---|---|---|
| VF-2 | 151-169 ms | 0.8-1.1 ms | 207-249 ms | 35-39 ms |
| UMC-500 | 159-185 ms | 0.9-1.2 ms | 216-260 ms | 44-48 ms |

The frame after is one ordinary lavapipe redraw (an orbit frame at 960x540 is 18-34 ms in `draw`
there); its own work is the 1 ms of `ui`.

Same pixels: the seven snapshots of that script (open, each edit, hover, click, the edit with a
selection, the last frame) are byte-identical between c944e6b8 and d77538d9 on both assemblies,
and the steps do change the picture: each first edit moves 729-921 pixels, the click on the
UMC-500 selects a body (29,660), the edit with it selected 226-40,423 (the `T` edit and the
VF-2's click change nothing seen from there). The assembly, pick,
selection and measure tests (result pending) (`cargo test --release --test fg_asm --test asm_drag --test pick
--test selection --test reselect --test headless --test asm_tools --test measure`). The kernel's
speed gate does not build forge-render, so it cannot move.

Two existing quirks were kept as they were, so the pixels stay the same: a restage puts the view's
occlusion back on after `O` turned it off, and drops a background image until its setting
changes. Each is a one-line fix in `Renderer::restage` if wanted.

## Keeping it

A cheap CI check, not built: on the `platforms` job's Linux runner, with Mesa's lavapipe
installed, run `forge-render FILE.step --bench --frames 120` on one mid-size model kept as an LFS
artifact (the UMC-500, 56 MB, 12 s to read), and `app.sh` on it at 640x360; fail when `cpu
submit` p99 or the edit step's `ui` worst passes 8 ms. Both are CPU numbers, steady on a
software rasterizer; together they take about two minutes. The GPU side stays a manual run on
real hardware before a release.
