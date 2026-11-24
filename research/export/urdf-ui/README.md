# Robot workspace: screenshots

From `interface/forge-render/tests/robot_ui.rs`, headless (1600×1000 at 1×, dark theme, Mesa
lavapipe in the cloud VM):

    cd interface/forge-render && FORGE_UI_SHOTS=../../research/export/urdf-ui cargo test --test robot_ui

- `01-robot-open.png`: `examples/robot/arm2_robot.fg` opened: its source assembly as the
  document, the Robot panel beside it (links and joints from the root, masses, a slider per
  moving joint; the frozen grip and the merged finger are the robot's own).
- `02-robot-driven-joint.png`: shoulder 50°, elbow driven to 300° and held at the robot's own
  ±90° limit (an override of the assembly's ±135°); the forearm placed by the robot's forward
  kinematics; the elbow selected: each field, and the overridden ones marked with the source's
  value.
- `03-robot-link.png`: the forearm link selected (bodies, mass, centre of mass, collisions).

- `04-robot-override-limits.png`: the shoulder's limits overridden from the panel (±45°): the
  robot document gained `joint(src.shoulder, limit: (-45deg, 45deg), limit_was: (-170deg,
  170deg))`; the field marked overridden with the source's value; Reset to linked and Apply to
  assembly beside it; effort, velocity, damping and friction as the export writes them.
- `05-robot-conflict.png`: the source's elbow limit changed under the robot's override: the
  export refused with the conflict, Keep and Reset on its row.

The panel's edits are edits of the robot document's text (`forge_lang::robot::edit`), each
undoable; Apply to assembly writes the source's joint (`SetArg`, one undo step there) and drops
the robot's override; Export URDF writes NAME_urdf/ beside the document.
