# Record: photos and videos of any environment — design note

Status: proposal, 2026-11-24. Owner: RENDER-STUDIO. Asked by David: "for all environments we should
be able to record the entire simulation, and output video or photos." A shared capability in the
environments framework (ENVIRONMENTS' `shell/env/`): every environment gets it with no work of its
own beyond exposing a timeline if it has one.

## 1. What a user gets
- **Photo** (every environment, and the plain design view): the view as PNG at a chosen size, from
  the live raster picture (instant) or from the path tracer at N samples (the Studio's, same
  appearances). Save; copy to the clipboard; in the browser, download.
- **Record** (environments with a timeline: CAM simulation, bend simulation, machine simulation,
  robot jog, thermoform, assembly motion): the whole run, stepped frame by frame **in simulation
  time, not wall clock** (deterministic: same input, same frames), at a chosen fps, size and
  quality (raster or traced at N spp), with the camera following the view as it is or along a
  recorded camera path (keyframes, interpolated). Optional overlays: time, the step's label, the
  environment's legend.
- **Outputs**: PNG sequence (a zip), animated GIF (quick shares), and video.
- **CLI**: `forgecad record F --env bendsim --out run.webm --fps 30 --size 1920x1080 [--traced N]`
  and `forgecad photo F [--env E] --out view.png --size WxH [--traced N]`, headless (lavapipe), for
  CI and agents.
- **UI**: a Capture group (Photo, Record, settings drop-down) on every environment's ribbon, from
  the framework; a progress strip with Cancel while recording (UX-LEAD's standard, their OK first).

## 2. The framework hook (with ENVIRONMENTS)
```rust
/// An environment whose state moves through time.
pub trait Timeline {
    /// Seconds of simulation time; its steps, for overlays and chapter marks.
    fn duration(&self, s: &Shell) -> f64;
    fn label_at(&self, s: &Shell, t: f64) -> Option<String> { None }
    /// Put the environment's state (and what the view shows) at time t; the GUI then draws.
    fn seek(&self, s: &mut Shell, t: f64);
}
// on Environment: fn timeline(&self) -> Option<&dyn Timeline> { None }
```
The recorder lives in the GUI (it owns the GPU): per frame it calls `seek(t_k)`, lets the GUI
rebuild what changed (meshes, poses) through its normal path, draws the view offscreen at the
output size (raster) or traces it (Studio tracer, fixed seed per frame), reads it back (async, as
the Studio's queue does, so the browser never blocks) and hands it to the encoder. One frame per
GUI tick, so the window stays live and Cancel works.

## 3. Encoders and licences (nothing GPL ships)
| Output | Native | Browser | Licence |
|---|---|---|---|
| PNG | `png` (in tree) | same (wasm) | MIT/Apache |
| PNG sequence | zip, stored (our own writer: CRC-32 + headers, ~100 lines) | same, download | ours |
| GIF | our own encoder: median-cut palette per run, `weezl` LZW (already in the lock) | same | MIT/Apache |
| Video | AV1 via `rav1e` into WebM (our own Matroska muxer), behind a cargo feature `video` (on in releases, off in quick dev builds); without it, the PNG sequence with a note | WebCodecs `VideoEncoder` (VP9, else AV1, else H.264 where the browser has it) into the same WebM muxer; MediaRecorder only as a last resort (it is wall-clock) | rav1e BSD-2-Clause; WebCodecs is the browser's |
H.264/MP4 natively is left out: the open encoders are GPL (x264) or patent-encumbered binaries
(OpenH264); WebM/AV1 plays in every current browser and in VLC/ffmpeg.

## 4. Determinism and tests
- Frame k is at t = k / fps exactly; the tracer's seed is (seed, k); nothing reads a clock.
- Tests: same input twice → identical PNG sequence bytes (native, lavapipe); a GIF of a 3-frame
  timeline decodes (with `gif`'s reader in dev-deps, MIT) to the frames' palettised pixels; the
  WebM muxer's output parses (EBML structure test) and, in the browser test, a 1 s VP9 WebM made
  by WebCodecs plays in a `<video>` element (duration and dimensions checked); CI records a 1 s
  headless clip of the bend simulation.

## 5. Order of work
1. Photo (raster + traced) from every environment, save/clipboard/download; CLI `photo`.
2. Timeline trait with ENVIRONMENTS; recorder → PNG zip; CLI `record`; determinism test.
3. GIF; WebM muxer; browser WebCodecs video; native rav1e behind `video`.
4. Camera paths and overlays.
