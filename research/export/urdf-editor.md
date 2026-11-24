# URDF editor: design

David: "urdf also needs a full editor so that you can mess w joint and everything", and "it should
be linked to the assembly or part, but not to the point I can't change or simplify the URDF."
Builds on the exporter (research/export/urdf.md, `forge_lang::urdf`).

## 1. Principles
- **Text is the source of truth.** A robot is a `.fg` document of its own (a `robot`
  document, as CAM's is a `machining` document), read by the same parser. The editor only ever
  sends edit ops (`SetArg`, `AddFeature`, `DeleteFeature`, ...) to it, one undo step each; the
  agent tools and the CLI do the same. Nothing lives only in the UI.
- **Linked, not locked.** The robot document names a source (an assembly or a part). Geometry,
  mass properties, joint frames, kinds, limits and values come from it and follow it live. On
  top: explicit, persistent overrides, one statement each. A field with no override is
  *linked*; a field with one is *overridden*. Reset-to-linked deletes the statement.
- **Never silent.** When the source changes, every override is checked against it: what
  changed, and which overrides now conflict, is reported, with fixes as ops (keep, reset, edit).
  An override is never dropped or kept without saying so. Same for the exporter's refusals.
- **Unlinked works too.** `source(none)`: a pure URDF, every link and joint written out; an
  imported `.urdf` becomes one of these.

## 2. The robot document
```
robot
shared eff = 20                          # numbers in params, as everywhere
src = source("arm2.fg")                  # the assembly or part; source(none): unlinked

root(src.base)                           # the root link (default: the grounded group)
fore = link(src.fore, name: "forearm")   # a linked link, renamed
merge(fore, src.finger)                  # several rigid groups as one link (see §4)
drop(src.cable)                          # out of the robot: no visual, no collision, no mass
collision(src.upper, box(size: (144, 24, 10), at: pose(at: (60, 0, 5))))
collision(fore, hull)                    # simplified: box | cylinder | sphere | capsule | hull | mesh("x.stl")
collisions("hull")                       # links with no collision(..): their convex hull (default) or "mesh"
actuation(effort: eff, velocity: 2)      # every moving joint's, unless its joint(..) says
visual(src.base, mesh("meshes/base_lowpoly.stl"))
inertial(fore, mass: 0.42, mass_was: 0.391)    # override (kg), and the linked value it replaced
joint(src.elbow, limit: (-90deg, 90deg), limit_was: (-135deg, 135deg))
joint(src.elbow, effort: eff, velocity: 1.5, damping: 0.1, friction: 0.02)   # URDF-only: no `was`
freeze(src.grip, at: 4)                  # a joint made fixed (here at 4 mm)
cut(src.j3)                              # a closed loop opened here
tool0 = frame(on: fore, at: pose(at: (144, 0, 5)))     # URDF-only fixed child link
cam = frame(on: src.base, at: pose(at: (0, 40, 30), z: (0, -1, 0)))
transmission(src.shoulder, actuator: "m1", reduction: 50)
```
- Names: `src.X` is the source's instance or joint X (stable ids: forge-lang's instance and
  joint names). A source rename shows as a broken reference with a fix (`rename src.old ->
  src.new`), never a silent loss.
- `FIELD_was:` (`limit_was`, `axis_was`, `mass_was`, `com_was`, `inertia_was`) records the linked
  value an override replaced, written by the editor when the override
  is made. It is what conflict detection compares against (§5). URDF-only fields (effort,
  velocity, damping, friction, transmission, names, frames) have no linked value, hence no `was`.
- Where an edit goes: a fact of the mechanism (a joint's limits, axis, drive) can be changed in
  the **source** (the assembly's own `limit:`; the CAD changes too) or **overridden** in the
  robot (the URDF only). The joint panel offers both, labelled; the default is **open question
  for David** (proposal: override in the robot, with "apply to assembly" one click away).
  URDF-only data is only ever in the robot document.

## 3. The model
`forge_lang::urdf` is split in two (no change to the output):
- `model(run, robot_doc) -> Model`: links and joints, every field a `Field<T> { value, from:
  Linked | Overridden { was } | Robot }`, plus validation issues; the exporter's current
  analysis is the linked layer, the robot document's statements apply on top in order.
- `write(&Model) -> Exported`: the XML and meshes, as today.
`forgecad export robot.fg urdf` and the editor's Export button both go through it. `forgecad
build robot.fg` reports links, joints, overrides (linked/overridden), conflicts and issues as
typed JSON, as any document does. `forgecad import x.urdf -o robot.fg` writes an unlinked robot.

## 4. Overrides and what they may do
| statement      | does                                                    | refused when (typed, with fixes)                  |
|----------------|---------------------------------------------------------|---------------------------------------------------|
| `root(L)`      | re-root: the tree re-hung from L (joints turned round as the exporter already does) | L dropped |
| `link(G, name:)` | rename                                                | name taken                                        |
| `merge(L, G..)` | groups as one link, mass composed                     | the groups are joined by a moving joint: it must be frozen first (`joint(J, type: "fixed", at: v)`), said in the error |
| `drop(G)`      | out of the robot                                        | it carries the root, or children (they'd float: listed) |
| `collision/visual(L, geom)` | replace geometry: `box/cylinder/sphere/capsule(..)`, `hull`, `decimate(n)`, `mesh(file)` | a capsule (no URDF type) is written as cylinder + two spheres, said |
| `inertial(L, mass:, com:, inertia:)` | override any of the three             | not physical (mass ≤ 0, inertia not positive definite or breaking the triangle inequality) |
| `joint(J, ...)` | type, axis, origin, limits, effort, velocity, damping, friction, mimic | a type the frames can't carry (e.g. prismatic where the source is a ball) is allowed only as an explicit override, flagged as differing from the mechanism |
| `cut(J)`       | open a loop at J                                        | J not in a loop (warning, not error)             |
| `frame(on:, at:)` | URDF-only fixed link (tool, sensor)                  | name taken                                        |
| `transmission(J, ..)` | `<transmission>`                                 | J fixed                                           |

## 5. When the source changes
On every evaluation the linked layer is rebuilt and each override checked:
- **value conflict**: an override's `was` ≠ the linked value now ("elbow's limit in arm2.fg went
  from ±135° to ±120°; the robot overrides it to ±90°"). Fixes: keep (rewrite `was`), reset
  (delete the override), edit.
- **gone**: an override names something the source no longer has (instance, joint). Fixes:
  delete, or rename to a near name.
- **structure**: a merge now spans a moving joint, a cut is no longer in a loop, a dropped group
  now carries the root. Fixes per case.
- New source joints/instances appear linked, listed as "new in the source".
The editor shows these in a banner and the validation panel; `build` returns them typed. Nothing
is applied or dropped until the user picks a fix.

## 6. The editor (forge-render, native and browser)
A **Robot** workspace (beside Design/Assembly/Manufacture), opened on a robot document or made
from an assembly ("New robot from this assembly" writes `robot` + `source(..)`).
- **Tree** (left): root first; links with their joints between, icons by type; drag a link onto
  another to merge, context menu: rename, make root, drop, add frame. Dummy links shown dimmed.
- **Joint panel**: type, parent/child, origin frame (xyz, rpy), axis, limits lower/upper,
  effort, velocity, damping, friction, mimic; each field with a *linked*/*overridden* chip, the
  linked value shown greyed beside an override, and reset. Limits edit as "in assembly" or
  "override" (§2).
- **Link panel**: inertial (computed value and its bounds shown; override fields), visual and
  collision geometry (mesh / primitive / hull, with a fit-primitive-to-mesh button), material
  colour.
- **3D view**: the robot placed by the **model's** forward kinematics (so robot-only overrides
  show), reusing the assembly view's drawing of occurrences (gui_asm, scene) — not the
  assembly solver, which knows nothing of overrides. Per joint a slider (and drag in the view,
  as gui_asmdrag does), held within limits; toggles: joint axes and frames, limit arcs and travel
  bars, centres of mass (sized by mass), collision geometry (wire), link names. "Back to export
  pose".
- **Validation panel**: the exporter's refusals and warnings live (tree, loops and where to cut,
  missing effort/velocity, zero-mass and massless links, non-physical inertia, open meshes, mimic
  chains, name clashes), each clickable to its link/joint and with its fix.
- **Import** a `.urdf` (meshes referenced, not copied); **Export** (ZIP or folder) with the
  exporter's options.
Look: forgeCAD's existing panels and palette (gui_inspect, palette.rs); no new widget style.

## 7. Tests
- Model: every override kind applied and refused; conflicts for each kind after a source edit;
  reset-to-linked = statement gone and value linked again; re-root gives the same placements
  (FK read-back, as the exporter's tests).
- Round trips: import(export(robot)) ≡ robot (links, joints, numbers); export(import(x.urdf))
  parses with check_urdf and has the same kinematics as x.urdf at sampled q.
- Determinism: pinned hashes native and wasm32.
- UI: the editor's ops as text edits (unit tests on the shell), and browser screenshots of the
  workspace (tree, panels, sliders at two poses, conflict banner) committed under
  research/export/urdf-ui/, from the web build in Chromium as web.yml runs it.

## 7a. Status (claude/urdf-robot)
Built (phase 2a): `forge_lang::robot` (the document, read with every bad line named),
`urdf::export_robot` (tree overrides `root`/`cut`/`freeze` into the walk, which keeps each link's
solved anchor apart from its URDF placement so a frozen joint at another value moves its subtree
rigidly; model overrides `link`/`merge`/`drop`/`frame`/`visual`/`collision`/`collisions`/
`inertial`/`joint`/`transmission`/`actuation`; convex hulls; conflicts with keep/reset fixes;
every override listed with its linked value in the reply), `forgecad build|export ROBOT.fg`.
Units: mm, angles as written (bare = rad), kg, kg mm².
Built (phase 2b): unlinked robots (`source(none)`; `NAME = link(mass:, com:, inertia:)`,
`NAME = joint(parent:, child:, type:, at:, axis:, limit:, effort:, velocity:, damping:,
friction:, mimic: (J, multiplier, offset))`, checked a tree with one root; point masses allowed,
impossible inertias refused), `mesh(file, scale:)` (packaged once when a readable STL/OBJ beside
the document, else referenced as written, said), visual `material:`/`colour:`, and
`forgecad import ROBOT.urdf [-o DIR]` (`forge_lang::urdf_import`: its own XML reader; inertials
turned from their rpy into link axes; what the document cannot carry listed as `# not imported`).
Not yet: `decimate(n)`, edit ops for robot statements (with the editor).

## 8. Order
1. Exporter on main (this branch; merge train 3).
2. `model`/`write` split; robot document (parse, eval, overrides, conflicts); CLI build/export/
   import; tests. No UI.
3. Editor: read-only workspace (tree, panels, 3D with sliders, validation).
4. Editing: ops from the panels, linked/overridden chips, reset, conflict banner, import/export.
5. Screenshots, docs (AGENTS.md, help forge).

## 9. Decisions (coordinator, within David's intent, 2026-11-23)
- Editing a joint's limits or axis in the editor writes a **robot override** by default; the
  assembly stays as it is. An explicit "apply to assembly" action sits next to it.
- `merge` across a moving joint is refused until that joint is frozen by a visible statement,
  `freeze(J, at: v)` (the joint becomes fixed at v); never implicit.
- Collision default for new robots: a **convex hull per link** (it contains the visual mesh, so
  contact is never under-reported, and simulators take it cheaply); visual stays the mesh.
  Box/cylinder/capsule fits and a decimated mesh are the alternatives offered.
