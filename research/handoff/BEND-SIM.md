# BEND-SIM handoff (BENDSIM-ENV: the press brake environment in forge-render)

This session owned the Bend Simulation environment in forge-render. That covers the 3D scene
use, the camera, the see-through view, the panel, the ribbon, the station UI and the showcase
clips. BENDSIM-TOOLS owned the kernel side (forge-brake, forge-lang bendsim, forge-bend-lang),
and still does.

## Branches

| Branch | State |
|---|---|
| `bendsim/showcase` | The train-8 bendsim item. It replaced `bendsim/next`, which it contains, together with `bendsim/contact-tint` and `bendsim/hems`. See its head for the sha. |
| `bendsim/next` | 8c2d8aa5. Superseded by `bendsim/showcase`. |
| `bendsim/next-after-t7` | 8c2d8aa5, the same commit. Superseded. |
| `bendsim/contact-tint` | 22d30cc2. Merged into `bendsim/showcase`. |

UX-LEAD signed off on `bendsim/showcase` at c8e8b058: the camera, the Why row, the station-change
caption, the stack chips and one tool name everywhere. The commits after that are not yet
signed off: the collision tint, the station pickers, the hems and offsets, and the showcase
clips.

## What was done in this run

- **The punch drawn through the part.** David saw the tool and the bend out of line. On the
  TruBend 5130, OW300-S on EV00x needs 400 mm of daylight against 385 mm, so top dead centre
  stood under the sheet. The scene now clamps the stroke's top. The open-height check now fails
  when the tools alone overfill the daylight, instead of reporting "not checked". Test:
  `the_punch_meets_the_bend_every_frame`.
- **The camera follows every station** (David: "the camera only moves on the last tool"). A
  station change pans the eye along the bed once the handling is over. Test:
  `the_eye_goes_to_every_station`. It fails without the fix: at step 3 the eye was at −261 for a
  station at −50.
- **The panel explains the tools.**
  - A "Why" row under each bend's tooling: "chosen from N setups tried: …", with the rejected
    setups on hover, or "kept: station K's tools pass".
  - A caption at each station change.
  - Stack lengths on the overview chips. These are why identical tools sit at two stations.
  - One tool name everywhere, the library's own designation.
- **The overview.** Press Overview frames the tool row from the operator's side. The press body
  in front of it is seen through. Station chips name each station; clicking one goes to that
  station's first bend.
- **Collisions** (David: exact collision areas, tinted). This uses BENDSIM-TOOLS'
  `Scene::contact_bodies` (the cells' own footprint):
  - the region is drawn on the part, red where proven and amber where near or undecided;
  - the tools met stay red;
  - a "Collisions" toggle in the View group switches it.

  Test: `a_collision_is_drawn_on_the_part_where_it_is`.
- **Station pickers** (David: set up or change the tools by hand). In Machine & Tools, each
  station has a punch picker and a die picker over the press library. Add station and Remove
  (for an unused station) are there too. Every change is an edit of the bending document,
  simulated again at once, and undoable. Test: `a_stations_tools_are_changed_by_hand`.
- **Hems and offsets** (BENDSIM-TOOLS' bendsim/hems, merged).
  - Bends done are counted with `Step::mark`.
  - The stroke is named in the panel and the caption: pre-bend, flatten, or offset with another
    bend.
  - Rule names match on `Rule::word()` with a fallback, so new rules don't break this crate.
- **Smaller fixes.**
  - Envelope handling now reads "not proven"; the certified handling reads "meets" or "near".
  - No dead buttons: First and Previous are off at bend 1. "No feasible order: bend N fails …"
    appears in the status bar.
  - A 480 px window no longer panics.

## Showcase clips (`research/bendsim/env/shots/`, all 1920×1080)

1. `showcase-1-load-tray-trubend-stations-1080p.mp4`: the load tray on four planned stations.
   The eye goes to each station, with captions and the Why row.
2. `showcase-2-narrow-channel-collision-1080p.mp4`: the narrow channel on straight punches,
   right then left. The second bend closes the flange on the punch, and the region is drawn
   red.
3. `showcase-3-hem-joggle-wila-trubend-1080p.mp4`: the hem and joggle strip on WILA tools. It
   shows an offset stroke, a pre-bend and a flatten.
4. `showcase-4-station-pickers-1080p.mp4`: station 1's punch is picked by hand, so bend 1 meets
   the ram (shown red). Simulate Again plans around the stations and the new program plays.

## Fixed late

- **Red first frame.** The caches were keyed by the simulation's address, which a re-plan can
  reuse. They are now keyed by `BendSimUi::made`. Collision regions now come only from checks
  that ran. A check that could not run reads `pass: false`; BENDSIM-TOOLS is fixing
  `contacts_of` on their side. Test: `a_replans_first_frame_draws_its_own_plan`.
- **Duplicate tools.** The same tool at two stations that are not next to each other was
  rejected as a duplicate (BENDSIM-TOOLS, 2d35af2d). The pickers test passes.

## Unfinished, and next steps

- **UX-LEAD sign-off.** UX-LEAD OK'd a2079733. After it: the caption change (one line per tool
  by title, key on hover), the red-frame fix, and the clips.
- **Wind-down.** PRE-VERIFY with `/tmp/pv` at 13d78f06 or later, on main c135b274, line sent
  to TRAIN-CONDUCTOR, who takes the band on the train.
- **UX-LEAD P3: handling framing.** During a handling step the eye can sit on the upper tooling,
  or the long stacks can fill the view. Frame the part and the tool it passes at 60–75 %.
- **UX-LEAD P3: station labels.** On the Machine & Tools bed strip, the labels of stations
  close together overlap ("s3s4").
- Not started: planning in the background with progress (BENDSIM-TOOLS' resumable
  `begin`/`step`/`finish`).

## Risks

- **Codegen band.** Train 7 bounced bendsim on bracket edit (+0.0309 %): forge-lang's bendsim
  module changed the eval path's codegen. With `codegen-units = 1` (FALLBACK's gate-memfns), it
  measured +0.0005 %. If gate-memfns lands differently, the band can go red again. BENDSIM-TOOLS'
  fallback is out-of-line registration (`#[inline(never)]` or cold), or its own crate.
- **Slow tests.** PRE-VERIFY's native step had four bendsim tests over the time budget.
  BENDSIM-TOOLS split them (f2a8ee37). The deep-box planner test (3.0 s) and the self-collision
  test (3.4 s) are still over the 2 s warning line, though under SLOW_RED.
- **The see-through and camera tests depend on the planner's layout.** If stage() picks other
  stations or tools, `the_eye_goes_to_every_station` and the overview tests still hold. Shots
  and clips would need re-rendering.
