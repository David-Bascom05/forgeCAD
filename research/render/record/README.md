# Capture shots

Headless (llvmpipe), `CAPTURE_SHOTS=DIR cargo test --release --test capture_shots` in
interface/forge-render, branch render/studio-3.

| File | What |
|---|---|
| 01-ribbon-bendsim-1400/1024.png | Capture in Bend Simulation's ribbon: Photo large; Record Turntable (no timeline yet) and Capture Settings small |
| 02-ribbon-manufacture-1400/1024.png | the same in Manufacture; at 1024 the ribbon collapses the small tools to icons |
| 03-settings.png | Capture Settings: size presets, fps, turntable length, MP4 / PNG frames, overlays, folder |
| 04-recording.png | a turntable recording under way: the pill with frame, progress and Stop; Record swapped for Stop Recording, Photo off |
| 05-photo-saved.png | the result line after a photo, with Open folder |
| 06-the-photo.png | the photo itself: the model, its ground and shadow, no UI |
| 07-bendsim-channel-native.mp4 | Record in Bend Simulation (a 100 × 200 channel, 2 bends on the TruBend 5130): its timeline, 8 s, 240 frames at 30 fps, 1280 × 720, by ffmpeg (`forge-app channel.fg --env bendsim --record … --fps 30 --out-size 1280x720`); ffprobe: h264, 240 frames |
| 08-bendsim-channel-web.mp4 | the same in the browser (web/tests/record.cjs): WebCodecs H.264 into the app's MP4 muxer, 640 × 360 at 10 fps; ffprobe: h264, 80 frames |

With BEND-SIM's timeline merged (bendsim/timeline @5ea51ae3) the press draws through the renderer,
so 07 and 08 show it. At the start of bend 2 (07: frames 120–125) the handling move turns the part
out of the frame for about 0.3 s.
