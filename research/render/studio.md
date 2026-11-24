# Render Studio: design note

Status: proposal, 2026-11-23. Branch `render/studio`. Owner: RENDER worker.

Goal (David): photorealistic renders of parts and assemblies, as an environment in the Environments
tab. Everything here lives in `interface/forge-render`; nothing in `kernel/` changes, so the speed band
does not apply. Native and browser (WebGPU) run the same WGSL.

## 1. Scope, in order of landing

1. **Appearances** (`studio/appearance.rs`, `data/appearances.toml`): physically based, metal/rough
   model, one record per look, each value with a source and a kind (`cited`, `user`, `stand-in`) as
   forge-material does. Built in: aluminium (polished, brushed, bead-blasted, anodised clear / black /
   red / blue), steels (polished, ground, black oxide, galvanised, mild steel mill finish), stainless
   (polished, brushed #4, bead-blasted), brass, copper, titanium, cast iron; plastics (ABS, PC, PMMA
   clear, PA, POM, PP, PTFE, PETG) gloss / matte / textured; glass (clear, frosted); rubber (EPDM,
   NBR, silicone); wood (oak, maple, walnut, birch ply); paint (gloss, satin, powder coat,
   metallic flake as clearcoat). Mapping from forge-material: by material name first
   (`materials = ["6061-T6", ...]`), then by family; the part's own `colour` tints dielectrics when
   no appearance is assigned. User appearances: same TOML in the project's `materials/` and
   `~/.config/forgecad/`, exactly like forge-material's libraries.
   Data: metals' F0 (and edge tint) computed from measured complex IOR at 450/550/650 nm
   (refractiveindex.info: Rakić 1998 Al, Johnson & Christy 1972 Cu/Ag/Au, Werner 2009 Fe/Ti/Cr/Ni,
   brass/steel from the same); dielectric F0 from measured IOR (PMMA 1.49, PC 1.586, ABS 1.53,
   N-BK7 1.5168 Schott datasheet); albedos from the CC0 Physically Based database
   (physicallybased.info) where it has the material. Roughness has no clean published mapping from
   Ra: those values are marked `stand-in` (judged against photographs) until we measure.
2. **Path tracer** (`studio/trace.wgsl` + `studio/tracer.rs`): progressive, one sample per pixel per
   dispatch, megakernel compute shader (8x8 workgroups), accumulation in an f32 storage buffer.
   Scene: the finest LOD of each placement's mesh, flattened to world triangles; a binned-SAH BVH
   built on the CPU (our own, threads via forge-par), flattened to a storage buffer, stack traversal.
   BSDF: Lambert + GGX (Smith height-correlated), VNDF sampling (Heitz 2018 / Dupuy & Benyoub 2023
   spherical caps), Schlick for dielectrics, F0 + edge tint for conductors, clearcoat layer, rough
   dielectric transmission (Walter 2007) for glass, Kulla-Conty multiple-scatter compensation
   (tables computed by us at start-up). Lights: HDRI with importance sampling (marginal/conditional
   CDF over luminance x sin(theta)), studio presets as analytic rect/disk area lights + a procedural
   backdrop, a sun; MIS (power heuristic) between BSDF and light samples; Russian roulette after 3
   bounces; max 8 bounces (16 for glass). Fits WebGPU's default limits (<= 8 storage buffers per
   stage, 128 MB binding): checked at start-up, scenes beyond it are refused with a typed reason.
3. **Denoiser**: our own edge-avoiding a-trous wavelet filter (Dammertz et al. 2010, the paper, no
   code taken) on demodulated irradiance, guided by first-hit albedo / normal / depth AOVs that the
   tracer writes for free; 5 passes, WGSL. Off for final renders unless asked; on for preview.
   (OIDN, Apache-2.0, is the quality option later for native only; it has no wasm build.)
4. **Environment**: HDRIs, a small built-in set from Poly Haven (CC0; each cited in
   `data/hdri/SOURCES.md` with URL, author and date), stored 1024x512 RGBE (~1 MB each, 4-5 files);
   a 1-colour/gradient/transparent backdrop option; ground plane: shadow catcher (shadows + optional
   reflection, invisible otherwise) or a real infinite plane with an appearance; rotation and
   exposure of the environment.
5. **Camera**: perspective (focal length in mm on a 36x24 frame) / orthographic, thin-lens depth of
   field (f-stop, focus distance, click to focus), saved views (in the project), turntable
   (N frames about world z through the scene centre).
6. **Output**: PNG (8-bit sRGB, ACES-fit tone map, exposure) at a chosen resolution and sample
   count, alpha for transparent backdrop; turntable frames numbered `name_0000.png`; a render queue
   (jobs run one after another, progress, cancel). CLI too: `forgecad render F --studio ...`.
7. **Assemblies**: every occurrence's mesh and appearance; exploded view from `gui_explode.rs`'s plan.
8. **Real-time preview in the normal viewport**: the raster `realistic()` path gets the appearance's
   roughness (tints grow a second vec4) and split-sum IBL from the same HDRI (prefiltered mips + a
   2D BRDF LUT, both computed on the GPU at load). Later than the tracer.
9. Later: decals and texture maps (UV-less: triplanar), brushed anisotropy, spectral glass dispersion.

## 2. Workspace

A Studio environment: viewport shows the progressive trace (resets on any change), panels for
Appearance (drag onto parts/faces), Environment, Camera, Output/queue. Until the ENVIRONMENTS
worker's API exists it is a standalone workspace behind `--studio` (and a settings flag in the
browser); its content is a `Studio` struct with `ui(&mut egui::Ui, ...)` and `paint(...)` so it
plugs into whatever trait that API gives.

## 3. Determinism and tests

- Random numbers: PCG hash of (pixel, sample index, bounce, dimension, seed); the seed is a render
  setting (default 0). Same inputs, same device -> same bits; across devices -> tolerance.
- CPU mirror (`studio/bsdf.rs`, `studio/cpu.rs`): the same BSDF, sampling and a small CPU tracer in
  plain Rust (no GPU on CI): unit tests for energy conservation (white furnace <= 1, Lambert = 1,
  GGX conductor F0 = 1 with compensation ~ 1 at all roughness), reciprocity, pdf normalisation and
  sampling-vs-pdf chi-square, Fresnel limits, env-CDF sampling vs pdf, BVH vs brute force.
- Golden image: a fixed scene (part + ground + built-in HDRI, 160x120, 256 spp, seed 0) rendered on
  the GPU, compared to `tests/golden/studio_*.png` by RMSE and per-pixel quantile; native (lavapipe
  or a GPU; skipped with a message if none, like `tests/headless.rs`) and browser (Chromium with
  swiftshader WebGPU, as `web/tests/ci.sh` already runs). The GPU result is also compared to the CPU
  tracer's mean within its Monte Carlo error, so a golden cannot drift into being wrong.
- Renders for David committed under `research/render/studio/`.

## 4. Performance budget

- RTX 5060 laptop class: 1920x1080 preview at >= 30 samples/s (1 spp/frame, 4 bounces, ~1 M tris);
  a 1080p final at 512 spp in < 30 s. Browser: within 2x of native.
- BVH build: < 150 ms per million triangles (native, 4 threads); rebuild only on geometry change,
  placement-only changes refit the top level (two-level BVH: per part + instances).
- The normal viewport is untouched unless the Studio is open (no cost to CAD frame times).
- CI (lavapipe): the golden scene in < 15 s.

## 5. Open questions for the coordinator / David

- HDRI files in the repo: ~5 MB total at 1024x512. Fine, or fetch on first use (browser: from the
  same origin) and keep only one small one in the repo?
- Environments API shape (from the ENVIRONMENTS worker).
