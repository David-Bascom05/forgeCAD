# Record: photos and videos of any environment — design note

Status: building, 2026-11-24 (photo, record, PNG frames, MP4 natively via ffmpeg and in the browser via WebCodecs, the CLI, the ffmpeg build and installers done; waiting: the ffmpeg workflow's first run, timelines from the environments). Owner: RENDER-STUDIO. Asked by David: "for all environments we should
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
- **Outputs**: MP4 (H.264), PNG frames (a folder natively, a zip in the browser), a single PNG.
- **CLI**: `forgecad record F [--env E] [--out run.mp4|DIR] [--fps N] [--size WxH] [--seconds S]
  [--frames]` and `forgecad photo F [--env E] [--out view.png] [--size WxH]`, headless (lavapipe
  in CI): they run `forge-app F --record|--photo OUT ..` beside them, the same Capture code the
  ribbon runs. (`--traced N`, the path tracer's picture, is still to come.)
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

## 3. Encoders and licences (David's decision, 2026-11-24: MP4; ffmpeg the one dependency)
| Output | Native | Browser |
|---|---|---|
| Still | PNG (`png`, in tree) | the same (the project's files, downloadable) |
| PNG frames | a folder of `frame_NNNNN.png` | a zip, our own stored-zip writer (`record/zip.rs`) |
| MP4 (H.264) | ffmpeg as a **subprocess** (`record/ffmpeg.rs`): raw RGBA on stdin, `libx264` first, else `libopenh264`, the platform's encoder, `mpeg4`; found at `FORGE_FFMPEG`, beside the app (the installers put the pinned build there), then PATH; nothing of ffmpeg is linked | WebCodecs `VideoEncoder` (`avc1`) into our own MP4 muxer (`record/mp4.rs`, checked by ffmpeg decoding what it writes); no `avc1` → PNG zip |
| no ffmpeg | PNG frames and the exact ffmpeg command to make the video | — |
Each recording says which path made it (`Video saved: … (N frames, ENCODER)`). GIF and WebM are
out (no second encoder, no new compiled dependency).

**The bundled ffmpeg** (David's plan, 2026-11-24: public prebuilt builds; no build or release of
our own). `tools/ffmpeg/PIN` pins one per platform by its exact versioned URL and the sha256 of the
file as downloaded (computed, never guessed): Linux x86_64 and arm64, John Van Sickle's static
6.0.1 (GPLv3; his archived old-releases, the newest there); Windows x86_64, gyan.dev's 9.0.2
essentials (GPLv3; the sha256 matches gyan.dev's own). macOS: none yet, UNKNOWN (Martin Riedl's
versioned builds are `--enable-nonfree`, which is not redistributable; evermeet.cx, x86_64 only,
could not be downloaded where this was set up). GPL is fine: ffmpeg runs as a separate program over
a pipe and nothing of it is linked. `tools/fetch-ffmpeg.sh` / `.ps1` download, check the sha256,
unpack ffmpeg and ffprobe into the install's `bin` beside forge-app, with the build's licence
(`ffmpeg-LICENSE.txt`) and `ffmpeg-THIRD-PARTY.md` (version, builder, URL, sha256, the link to that
version's source), and an `ffmpeg.pin` stamp so an update fetches again only when the pin changes.
`install.sh`, `install.ps1` and `forgecad-update` run it; a failed fetch never fails the install.
Without one: the user's own (`FORGE_FFMPEG`, PATH), else PNG frames and the command to make the
video. The encoder: libx264 when the ffmpeg has it, else openh264, the platform's, mpeg4.

**The browser**: web/app.js's `forge_video_*` drive WebCodecs' `VideoEncoder` (constrained
baseline, the least level the size and rate fit, AVCC output, no B-frames); the app grows odd sides
to even ones, takes the encoded frames and avcC back, and writes the MP4 with `record/mp4.rs`.
`window.forge_avc1` (from `isConfigSupported` at load) false → a PNG zip, said so.

## 4. Determinism and tests
- Frame k is at t = k / fps exactly; the tracer's seed is (seed, k); nothing reads a clock.
- Tests: same input twice → identical PNG sequence bytes, a photo twice → identical PNG
  (tests/photo.rs, native, lavapipe); an MP4 by ffmpeg that ffprobe reads as H.264 with every
  frame; the muxer's index points at every frame and a real stream round-trips through ffmpeg
  (record/mp4.rs); in the browser (web/tests/record.cjs) a photo downloads at its size and a
  161 × 97 turntable downloads as an MP4 ffprobe reads as `h264,162,98,10`; the fetch refuses a
  hash mismatch and leaves nothing behind. To come: a 1 s headless clip of the bend simulation
  once its timeline lands.

## 5. Order of work
1. Done: photo, record (timeline or turntable) to PNG frames and MP4, natively and in the browser;
   the CLI; the ffmpeg build, fetch and installers.
2. Next: a macOS ffmpeg pin; the environments' timelines (CAM, thermoform) and a clip of each.
3. Then: traced (path-tracer) photos and recordings; camera paths; overlays (time, step label).
