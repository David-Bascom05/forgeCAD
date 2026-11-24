# Capture shots

Headless (llvmpipe), `CAPTURE_SHOTS=DIR cargo test --release --test capture_shots` in
interface/forge-render, branch render/studio-3.

| File | What |
|---|---|
| 01-ribbon-bendsim-1400/1024.png | Capture in Bend Simulation's ribbon: Photo large; Record (Bend Simulation's timeline) and Capture Settings small |
| 02-ribbon-manufacture-1400/1024.png | the same in Manufacture; at 1024 the ribbon collapses the small tools to icons |
| 03-settings.png | Capture Settings, a dialog like the others (title row, close): size presets, fps, turntable length, MP4 / PNG frames, framing, overlays, folder (its last part, the whole path on hover, Choose…, Reset) |
| 04-recording.png | a recording of the bend simulation's timeline under way: the pill with the frame, a plain bar and Stop; Record swapped for Stop Recording, Photo off |
| 05-photo-saved.png | the result line after a photo, with Open folder |
| 06-the-photo.png | the photo itself (1280 × 720, in Bend Simulation): no UI; an environment's photo keeps the window's composition, refitted to the picture's shape |
| 09-photo-design-view.png | a photo in the design view (1280 × 720): the part framed to fill 70 % of the picture from the view's direction, whatever the window's shape ("Use the view exactly" keeps the camera as it is) |
| 10-web-capture-settings.png | the browser (WebGPU, 1000 × 700): Capture on the View tab, its settings (no folder: the browser downloads) |
| 11-web-recording.png | the browser recording the bend simulation's timeline: the pill, Photo off, Stop Recording in Record's place |
| 07-bendsim-channel-native.mp4 | Record in Bend Simulation (a 100 × 200 channel, 2 bends on the TruBend 5130): its timeline, 8 s, 240 frames at 30 fps, 1280 × 720, by ffmpeg (`forge-app channel.fg --env bendsim --record … --fps 30 --out-size 1280x720`); ffprobe: h264, 240 frames |
| 08-bendsim-channel-web.mp4 | the same in the browser (web/tests/record.cjs): WebCodecs H.264 into the app's MP4 muxer, 640 × 360 at 10 fps; ffprobe: h264, 80 frames |

07 and 08 are from bendsim/timeline d2743fc9 (Follow bend on through the run, the handling followed,
see-through with hysteresis and a fade): the see-through bodies are drawn blended as in the view
(tests/photo.rs a_see_through_body_is_blended_in_a_photo). Not yet in them: the white outline of what
is seen through (the panel draws it over the window; BEND-SIM is moving it into the 3D scene), and
in bend 2 the press's upper beam stands in the top of the picture (as in the live view).
