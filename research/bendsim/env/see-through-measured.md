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

## With the operator's camera (David: still, panning down the line)

Follow bend now keeps the eye's orientation and distance:

- the working view is framed once, on the part's motion at the tools over every step at its
  station;
- the part's small motions (the slide to the stop, the lift, the ram's stroke, a flange's swing,
  a turn over) leave the eye still;
- the eye pans along the bed only when the part's middle leaves the frame's middle 60 %, and
  brings the part back only when it nears the frame's edge;
- it draws back once at most, and only if the part cannot fit;
- the operator's handling starts no move;
- every move is a smoothstep over 15 frames, from rest to rest, with no cut.

The same measure, with the watched points now also including the free edge of the flange
swinging up, and a body seen through at 3 % of them:

| turn | watched seen | view ghosted | frame |
|---|---|---|---|
| 0 | 99.8 % | 7.2 % | 32.3 ms |
| 60 | 99.8 % | 7.6 % | 36.6 ms |
| 120 | 99.5 % | 7.2 % | 36.4 ms |
| 180 | 100.0 % | 8.0 % | 37.8 ms |

Smoothness of the clips, measured the coordinator's way: the frame-to-frame mean luma difference
at 160 px wide (ffmpeg `tblend` difference, `signalstats` YAVG).

| clip | max, before | after | median, before | after | frames over 5, before | after | duplicates, after |
|---|---|---|---|---|---|---|---|
| three-quarter | 24.5 | 7.6 | 0.30 | 0.18 | 50 | 12 | 0 |
| profile | 30.1 | 2.1 | 0.28 | 0.15 | 23 | 0 | 0 |
| orbit | 18.6 | 10.2 | 1.09 | 1.02 | 26 | 8 | 0 |

The camera's cuts at each step boundary and its re-framing back and forth are gone. The
three-quarter clip's remaining frames over 5 fall at each step's t ≈ 0.32 and 0.93: the start of
the ram's approach and the end of its return. That is the press moving fast through the picture
at twice real speed (24 s of program in a 12 s clip), and the cut-away taking the ram away or
putting it back in one frame. A fade for the cut-away is the next step.

Each frame is rendered once, at its own instant, with no clock read, so no frame is duplicated.
Frames repeat only where nothing in the picture moves.

## Five stations on the TruBend 5130 (BENDSIM-TOOLS' staged program)

`shots/load-tray-5-stations-trubend.mp4` and its strip: the load tray on a Trumpf TruBend 5130,
the planner's five stations along the bed. They run from y −335 to 380:

| station | punch | die | bends |
|---|---|---|---|
| 1 | OW300-S | EV004 30 degrees | 1, 2 |
| 2 | OW300-S | EV004 30 degrees | 3 |
| 3 | OW300-S | EV001 30 degrees | 4 |
| 4 | OW222-K | EV001 30 degrees | 5 |
| 5 | OW222-K | EV001 30 degrees | 6 |

It was rendered from bendsim/next d3ea3f8a with bendsim/tools b359d1be merged (in a scratch
checkout, BENDSIM-TOOLS' `a_staged_clip`). With the operator's camera, the eye stands still at a
station and eases along the bed as the part moves to the next. The other stations' tools are seen
through only where they stand between the eye and the bend.

Jerk, measured the same way: max 8.7, median 0.25, 2 frames over 5, no duplicates.

## The cut-away faded, ghosts keeping their shadows

The cut-away now fades in and out over the ghosts' 9 frames, so no press part vanishes in a
frame. A part wholly cut away stays in the scene as an unseen ghost. Seen-through and cut-away
parts keep casting their shadows (the scene's `ghost_shadows`, on only for the bend simulation),
so the floor's light does not jump as they go.

The clips were re-rendered on bendsim/next with bendsim/tools 6fa149ea merged.

| clip | max | median | frames over 5 | duplicates |
|---|---|---|---|---|
| three-quarter | 4.9 | 0.17 | 0 | 0 |
| 5 stations, TruBend 5130 | 8.2 | 0.26 | 3 | 0 |

The five-station clip's three frames over 5 are at its first frame (8.2), and at the start of the
ram's approach on bends 1 and 5 (7.6 and 5.2), where the whole floor's shading changes at once.
That is not the cut-away, nor a ghost losing its shadow: the numbers did not move with either.
The cause is still open; the next suspect is the sun's shadow fit following the press's moving
bounds.
