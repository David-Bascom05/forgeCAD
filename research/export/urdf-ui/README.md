# Robot workspace: screenshots

From `interface/forge-render/tests/robot_ui.rs`, headless (1600×1000 at 1×, dark theme, Mesa
lavapipe in the cloud VM):

    cd interface/forge-render && FORGE_UI_SHOTS=../../research/export/urdf-ui cargo test --test robot_ui

The Robot environment (docs/ux/STANDARD.md §1): its own tab and ribbon (Robot, Drive,
Structure, Exit / Finish Robot), the link and joint tree in the browser, the selected link's or
joint's sheet docked at the right edge at full height, the view what is left, fitted to the robot
on entering (§2: its parts filling 70% of the view's shorter side, from the 3/4 view). Every
value carries its unit (mm or in by the status bar's toggle, degrees, N·m or N, rad/s or m/s)
and a badge for where it came from (§6: `linked`, `yours`, `unknown · Set…`).

- `01-robot-open.png`: `examples/robot/arm2_robot.fg` opened: the tree from the root with tree
  lines, an icon per kind, each link's mass right-aligned, `root` and `massless` badges; the
  sheet with the drive (a slider per moving joint).
- `02-robot-driven-joint.png`: shoulder 50°, elbow driven to 300° and held at the robot's own
  ±90° limit (an override of the assembly's ±135°); the forearm placed by the robot's forward
  kinematics; the elbow's sheet: Joint, Placement (origin X/Y/Z, axis +Z), Limits (yours, the
  assembly's on hover), Actuation.
- `03-robot-link.png`: the forearm link's sheet: Bodies, Mass properties (mass, centre of mass
  X/Y/Z), Geometry (visuals, collision), Structure.
- `04-robot-override-limits.png`: the shoulder's limits overridden from the sheet (±45°): the
  robot document gained `joint(src.shoulder, limit: (-45deg, 45deg), limit_was: (-170deg,
  170deg))`; the limits badged `yours`; Use source and Apply to assembly beside them.
- `05-robot-conflict.png`: the source's elbow limit changed under the robot's override: one
  sentence ("Elbow limit changed in arm2.fg from −135…135° to −135…120°. The robot overrides it
  with −90…90°.") with Keep override and Use source; the robot document's line on hover.
- `06-robot-tree-edited.png`: the tree changed from the ribbon's Structure group and the sheet:
  the elbow frozen at -60°, the forearm merged into the upper arm, the upper arm made the root, a
  frame added, the upper arm's collision its convex hull; the Checks say the robot's own elbow
  override is now moot.

The panel's edits are edits of the robot document's text (`forge_lang::robot::edit`), each
undoable; Apply to assembly writes the source's joint (`SetArg`, one undo step there) and drops
the robot's override; Export URDF writes NAME_urdf/ beside the document.

## In the browser

From `interface/forge-render/web/tests/robot_shots.cjs` (the web build, Chromium with WebGPU on
SwiftShader, 1600×1000), the same document:

    cd interface/forge-render && NO_MESHES=1 sh web/build.sh OUT && FORGE_WEB_SHOTS=../../research/export/urdf-ui node web/tests/robot_shots.cjs OUT

- `web-01-robot-open.png`, `web-02-robot-joint.png` (the elbow selected from the browser),
  `web-03-robot-link.png` (the forearm): the same layout, sheet, badges, named axis arrows and
  status line as the native pictures.
