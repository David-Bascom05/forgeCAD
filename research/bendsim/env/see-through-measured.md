# See-through, measured (load tray)

David asked for see-through that is view-dependent and minimal, an order of magnitude better.
`see_through_measured_on_the_load_tray` (in `interface/forge-render/tests/bendsim_env.rs`, run
with `BENDSIM_METRIC=1`) measures it from what is drawn, with the same code on both versions:

- **watched seen**: the share of the watched points (the bend zone's top along its length, the
  punch tip's contact edge, any check's witness) that no solid body hides from the eye;
- **view ghosted**: the share of the 3D view the ghosts cover (a 140 x 86 raster of their
  triangles);
- **frame**: the mean wall time of a frame (headless, lavapipe, release build).

Each value is the mean over the tray's six bends, mid-stroke, at the default view turned by 0,
60, 120 and 180 degrees.

| turn | watched seen, before | after | view ghosted, before | after | frame, before | after |
|---|---|---|---|---|---|---|
| 0 | 96.8 % | 99.3 % | 12.4 % | 5.3 % | 36.3 ms | 32.4 ms |
| 60 | 97.7 % | 99.5 % | 11.0 % | 4.5 % | 33.6 ms | 35.1 ms |
| 120 | 84.5 % | 99.5 % | 7.5 % | 7.3 % | 37.9 ms | 38.8 ms |
| 180 | 71.1 % | 99.3 % | 9.4 % | 5.2 % | 40.7 ms | 36.5 ms |

Before: bendsim/next 7373c3f0. Each body was tested against a screen rectangle bounding the bend
zone, which a bend line seen at an angle makes most of the picture. After: from the live eye,
exact rays to each watched point; a body is seen through only while it hides at least 4 % of the
points (and two at least).

- The share of the bend hidden falls from 12.5 % to 0.6 % (about 21 times less).
- The view ghosted falls from 10.1 % to 5.6 %.
- The frame time is unchanged.

The sheet is seen through piece by piece (each wall, each flange), the bend being made and the
walls either side of it never. The press and the tools are seen through whole. The die and the
bed are seen through only when they hide the bend, and what is drawn red keeps its red.
`only_what_hides_the_bend_is_seen_through` holds this at 97 % seen and 20 % ghosted.

Clips (12 s, the tray's six bends):

- `shots/load-tray-threequarter.mp4`: the default view;
- `shots/load-tray-profile.mp4`: end on, down the bend line (yaw 262 degrees, pitch 12 degrees);
- `shots/load-tray-orbit.mp4`: the eye turning a degree a frame round the subject.

Each clip has a 3 x 3 strip beside it.
