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
| MP4 (H.264) | ffmpeg as a **subprocess** (`record/ffmpeg.rs`): raw RGBA on stdin, `libopenh264` first, else the platform's encoder, `libx264` only in a user's own ffmpeg, else `mpeg4`; found at `FORGE_FFMPEG`, beside the app (the installers put the bundled LGPL build there), then PATH; nothing of ffmpeg is linked | WebCodecs `VideoEncoder` (`avc1`) into our own MP4 muxer (`record/mp4.rs`, checked by ffmpeg decoding what it writes); no `avc1` → PNG zip |
| no ffmpeg | PNG frames and the exact ffmpeg command to make the video | — |
Each recording says which path made it (`Video saved: … (N frames, ENCODER)`). GIF and WebM are
out (no second encoder, no new compiled dependency).

**The bundled ffmpeg** (the coordinator's ruling, 2026-11-24: build it ourselves). `tools/ffmpeg/PIN`
pins the ffmpeg.org tarball and OpenH264's source by sha256; `tools/ffmpeg/build.sh` builds OpenH264
(BSD-2-Clause) static, then ffmpeg with `--disable-gpl --disable-nonfree --disable-autodetect
--disable-everything` and only what recording needs (rawvideo in; libopenh264, mpeg4, png out; mp4
and image2 muxers; scale/pad/format filters), static. `.github/workflows/ffmpeg.yml` runs it on
linux-x86_64, macos-arm64 and windows-x86_64 (MSYS2), checks the result says `--disable-gpl` and
has `libopenh264`, and publishes release `ffmpeg-<version>-<build>` of this repository; it runs only
by hand or when PIN changes on main. Each archive carries `THIRD-PARTY.md` (versions, source URLs
with their sha256 as the LGPL's source offer, the exact configure line) and both licences.
`tools/fetch-ffmpeg.sh` / `.ps1` take the release asset pinned in PIN (`ASSET_*_SHA256`, filled after
the first run), refuse a mismatch, and put `ffmpeg`, `ffprobe`, `ffmpeg-THIRD-PARTY.md` and
`ffmpeg-LICENSE.*` in the install's `bin` beside forge-app, with `ffmpeg.pin` so an update fetches
again only when the pin changes. `install.sh`, `install.ps1` and `forgecad-update` run it; a failed
fetch never fails the install, and says that recording saves PNG frames until `forgecad-update`
succeeds. No GPL build is shipped anywhere (macOS x86_64 and Linux arm64 get none: PATH's, if any).
The binary is never committed.

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
2. Next: the first run of the ffmpeg workflow (David's go), then PIN's asset hashes; the
   environments' timelines (bend simulation first) and a recorded clip of each.
3. Then: traced (path-tracer) photos and recordings; camera paths; overlays (time, step label).
