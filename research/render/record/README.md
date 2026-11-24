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

Known: on this branch Bend Simulation draws its press view with the window's painter, which a photo
(the renderer's picture) doesn't include — 06 shows the part alone. BEND-SIM's own branch draws the
press through the renderer, so its photos and recordings will show it.
