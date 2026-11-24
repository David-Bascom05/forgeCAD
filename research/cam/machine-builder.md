# Machine builder (design note)

David: "we should importantly be able to add our own custom machines". The format and the loader
exist (kernel/forge-machine, docs/cam-machine.md, search order project → user → built-in). This
note is how a user makes and keeps a machine without writing TOML: a library crate, three CLI
words, and a Machine Builder workspace in the app (native and browser).

Branch `machines/builder` from `machines/base` @9f956879.

## 1. Layers

| layer | where | owner |
|---|---|---|
| format, reader, validation, kinematics, collision | kernel/forge-machine | CAM-MACHINE |
| writer, levels, templates/clones, check | interface/forge-machine-builder (new) | MACHINE-BUILDER |
| envelope check (needs forge-lang), setup runs | interface/forge-machine-lang | CAM-MACHINE (2 small additions here) |
| CLI words | interface/forge-cli `machine list/new/check` | |
| app workspace | interface/forge-render `gui_machine*.rs`, `shell/machine/` | MACHINE-BUILDER |
| setup dialog's machine picker | forge-render (Manufacture) | CAM-UI, reads `Catalog::load` |
| machine view in the simulation | forge-render | CAM-SIM, may reuse §4's body meshes |

Nothing in kernel/ changes: the builder uses forge-machine's public API only, so no speed band
applies to this branch (forge-lang/build.rs hashes kernel/; this branch leaves kernel/ alone).

## 2. Built (CLI, tested)

- `write`: canonical text in the docs' layout. Tested over every built-in machine and template:
  `parse(write(m)) == m` and `write(parse(write(m))) == write(m)` byte for byte. Strings the reader
  cannot take back (control characters other than \n \t) are refused, not mangled.
- `catalog`: the three levels with shadowing (`hides`), a broken file listed with its error
  instead of hiding every other machine. `forge_machine_lang::library` (what setups use) now goes
  through it, so setups see the user's folder too (before: project and built-in only).
- `make`: templates `vmc3 trunnion head-table lathe mill-turn gantry` (the generic built-ins, plus
  the builder's own head-table and gantry router; the gantry is kind `vmc3` with X, Y, Z all on
  the head side and the part mount on the base: no format change needed) and clones of any listed
  machine. A clone of a built-in brings its `.fg` body parts (embedded at build time) to
  `parts/NAME/BODY.fg` beside the file. `real` is always false on a made machine.
- `check`: validation; every pair at home; every pair through travel; part in envelope; provenance.

### Provenance and UNKNOWN (no silent defaults)

- Copied values (template or clone) are cited `status = "unknown"`, basis "copied from X: not
  measured on this machine and not known to be conservative for it". Never presented as the
  user's values.
- Values the user enters carry no cite; in a user/project file the check reports them as `user`.
  (`--own` takes every copied value as the user's: an explicit choice.)
- The format cannot hold a number that is UNKNOWN: a cite `unknown` still needs a stand-in, and
  the simulation checks against it. Until the format can (request F1 below), `check` lists every
  unknown-cited value as "not checked" and flags collision verdicts whose body envelope is unknown
  (`[envelope UNKNOWN: ...]`). The app shows such a field as `UNKNOWN (stand-in N)`.

### Self-collision through travel (how far it is proven)

The distance between two bodies depends only on the joints in exactly one of their chains
(shared joints move both rigidly). Per pair:
- no such joint: the home verdict holds for all travel;
- one: a line segment of that joint through its range is checked by collide::check, whose
  interval subdivision of a one-joint line covers exactly that range: proven;
- several: each joint swept with the others at both ends of their ranges (a `Hit` there is a
  real collision with its joint values and a point); then the whole box of their travel is proven
  by bisection: a diagonal segment checked in a single span (`Report::spans == 1`, no contact)
  encloses each joint's whole interval; else split the box across its relatively widest joint
  (budget 1024 boxes). Out of budget: `sampled` (lines clear, poses between them NOT proven).

Measured (release, this VM): generic-vmc3 every pair proven, 8 ms; trunnion 0.14 s; mill-turn
0.3 s. Findings on the built-in generic machines (CAM-MACHINE's data), each with joint values:
- generic-trunnion-bc: head reaches the cradle (X=-153.586 Y=-300 Z=-350 B=-120).
- generic-lathe: turret reaches the headstock (Z=-59.996 X=-10).
- generic-mill-turn: bhead reaches headstock, chuck, subchuck, subhead at travel extremes.
These are poses inside the declared travel; whether a real control's soft limits or interlocks
prevent them is a property of the machine, UNKNOWN here. The check reports them as problems.

A whole-box API in collide (no wasted descent along a diagonal) would make `prove_box` cheaper
and is requested (F4); the current one is sound with the public API.

## 3. Requests to CAM-MACHINE (through the coordinator)

- F1: a required number may be omitted when a cite `unknown` names it; consumers report "not
  checked" for limits against it (overtravel, feed, rpm, pockets, timing, collision of a body
  with no envelope).
- F2: cite status `user` (the user's own value; source/taken optional).
- F3: manual tool post (lathe without an indexing turret), per-machine tool-change (M06)
  position. Gantry needs nothing (above).
- F4: `collide::check_box(def, a, b, &[Interval])`.
- F5: a body's display mesh (`mesh = "parts/x.stl"`) for imported meshes that are not solids;
  the envelope primitives stay what collision uses.

## 4. The Machine Builder workspace (app)

Opened from a ribbon tab **Machines** (and from CAM-UI's setup dialog: "New machine..." / "Edit
machine..."). It swaps the view's scene for the machine (the pattern of gui_versions.rs: keep the
part's renderer and scene, restore on close).

```
+-- Library -------------+---------------- 3D ------------------+-- Edit ----------------+
| Project                |                                      | [Machine|Joints|Spindle|
|   shop-vmc        *    |   bodies as boxes/cylinders, tinted  |  Changer|Mounts|Bodies|
| User                   |   by link; parts (.fg) when built;   |  Provenance]           |
|   my-trunnion          |   pairs at Hit: red, Near: amber;    |  fields; copied values |
|   generic-vmc3 (hides  |   mounts as triads; joint axes       |  marked "copied"; an   |
|     built-in)          |                                      |  edit makes it "user"; |
| Built-in               |                                      |  "Unknown" button      |
|   generic-...          |                                      |                        |
| [New from template v]  |  Jog: X [----o----] -300..300  ...   | Validation: errors,    |
| [Clone selected]       |  [Home] [Check travel]               |  check report, per-    |
|                        |                                      |  value provenance      |
| [Save to: user|project]|                                      |                        |
+------------------------+--------------------------------------+------------------------+
```

- **Model**: the workspace edits a `MachineDef` plus the file it came from and its level. Every
  edit re-runs `validate` + `validate_cites` (microseconds) and shows the refusal next to the
  field it names. Editing a value whose cite is `unknown` (copied) removes that cite: it is now
  the user's. "Unknown" on a field adds the cite (basis "not known: entered by the user as a
  stand-in") until F1.
- **Joints as a tree** by parent: add child, delete (refused while a mount, spindle, body or child
  uses it, with the users listed), reparent, type, side, axis, pivot, range, home, rapid,
  max_feed, accel (blank = absent, said as "infinite acceleration assumed"), cyclic, tcp,
  preference, reset, diameter, letter.
- **Spindles** (kind, link, axis, nose, rpm, power-curve points table, max, interface, c_joint),
  **turrets**, **changer** (pockets, change_s, max_tool_kg; M06 position after F3), **mounts**,
  **coolants**, **control** (a forge-post dialect from a list, or typed).
- **Bodies**: per body its link, primitives (box min/max, cylinder base/axis/r/h), `ignore`;
  geometry from (a) primitives only, (b) an existing `.fg` part: "fit envelope" builds it and
  sets a box to its bounds, (c) a STEP file: writes `parts/NAME/BODY.fg` = `import("BODY.step")`
  and fits the envelope, (d) a mesh (STL/OBJ): envelope fitted from its bounds; the mesh shown
  only once F5 exists. The part-in-envelope check runs on save and on demand.
- **Models** (the user's own files, e.g. a maker's STEP; §5 "Attached models"): "Attach model…"
  per body, or one for the whole machine; drawn in the body's place, placed on its envelope, nudged
  (units, quarter turns, angles, move), checked against the envelope. Never in a project, never
  fetched, never committed.
- **Jog**: one slider per joint over its range (the value box beside it); the scene's placements
  follow `kin::pose`. Live self-collision at the jogged pose: `collide::check_pose` on every pair
  each change (cheap). "Check travel" runs §2's check in the background (plat::spawn; synchronous
  in the single-threaded browser build) and lists the pairs; clicking a Hit jogs to its pose.
- **Save** to the user or project folder through `make::save` (refuses a name already at that
  level in another file; reads back what it wrote and refuses if it does not load as the edited
  machine). Files go through forge_dirs::fs: in the browser the project folder persists through
  the page's storage; the user folder there (`/home/.config/...` in memory) is UNKNOWN to persist
  across reloads: the browser offers project level by default and says so.
- **The picker sees it immediately**: `catalog::Catalog::load(project, user)` is cheap (parses a
  few files); CAM-UI's setup dialog calls it when it opens. A save bumps
  `catalog::generation()` so an open picker can reload.

## 5. Status (built)

- CLI and crate: §2, tests in interface/forge-machine-builder/tests.
- App: Manage > Machines > Machine Builder (`Action::MachineBuilder`; files src/gui_machine.rs,
  src/shell/machine.rs, src/shell/machine_parts.rs). Library by level with what each hides; new
  from a template or a clone (copied values cited unknown / source cites kept / taken as mine);
  editor tabs Machine, Joints (tree; add child, delete refused while used, rename follows every
  reference and cite), Spindles (power-curve points), Changer (+ turrets), Mounts, Bodies (boxes,
  cylinders, `ignore`, geometry from a .fg / STEP / STL with "fit envelope"), Check; a provenance
  chip per value (copied, user, UNKNOWN, published, estimate, conflict) and "?" to mark a value
  unknown; jog sliders per joint posing the bodies, live contact at the pose (red hit, amber
  near); Check travel; save to the user or project folder.
- Body geometry: an attached STEP is copied to `parts/NAME/` with a `BODY.fg` that imports it;
  the envelope is fitted to the part's tessellated bounds + 0.05 mm (chord 0.01 mm). An STL gives
  the envelope only (its display waits on F5).
- Attached models (forge_machine_builder::models, src/machine_model.rs; tests/machine_models.rs).
  A maker's model may be committed only where its licence explicitly permits redistribution
  (tools/ci/model-licences.py; the licence findings per maker, 2026-11-24: none does), so the
  user brings their own:
  - Kept in the user's library only: `<config>/forgecad/cam/machines/models/<machine>/` holds a
    copy of each file and `models.toml` (body or whole machine, file, `from`, scale, rotate,
    offset). Nothing goes into a project; nothing is downloaded.
  - Read: STEP and IGES through forge-step (meshed as a file's body is), STL as it is. Parasolid
    (X_T/X_B) and CAD programs' own formats are refused with "save it as STEP (or STL)".
  - Placed: `p ↦ Rz·Ry·Rx·(scale·p) + offset` into the base frame at home, then by the body's
    link at the jogged pose. On attach it sits centred over the body's envelope, its bottom on the
    envelope's bottom; a file 25.4 or 1000 times too small for the envelope (within 2.5x) is read
    as inches or metres, and the message says so. Quarter turns and "Place on envelope" re-place
    it; angles and the move are free.
  - Checked: every point of the model (up to 200 000, spread evenly) against the body's
    primitives, 0.1 mm slack: "Inside the collision envelope", or "Sticks out by up to D mm (n of
    N points). Collisions there are not found." with "Fit envelope to model" (one box around it +
    0.05 mm, the value then the user's). Collisions are still found on the envelope only.
  - Shown: the body is drawn as the model (a whole-machine model as one piece on the base, not
    moving with the axes); on the Bodies tab the chosen body's envelope is dashed over the view
    (the model would hide it) and the farthest point out is marked with its distance.
  - The Part file row takes a forgecad `.fg` only now: a STEP or STL typed there is pointed to
    the Model row, so a maker's file is not copied beside a project's machine.
- Page: `fc.app_machine(cmd)` (open, pick, template, clone, jog, tab, check, save) drives the same
  code; the answer is the state's status line.
- Found while building it: egui's Slider rounds the value it shows into the variable; the jog
  bar slides a copy and writes back only on the user's move.

## 6. Tests and screenshots

- crate tests: writer round trips (every built-in and template), CLI end to end (template at
  project level with every value cited unknown, byte-stable file, parts in envelopes; clone at
  user level with `--own`; shadowing in `list`; refusals; a setup's simulation on the project
  machine, and the user's narrower X travel turning the same program into an overtravel).
- app: headless (native) screenshots of the workspace per template, jogging into a collision, the
  validation panel; the same in the browser build (Playwright); under
  research/cam/machine-builder/.

## 7. Known issue: browser shading of the machine's bodies

In the browser build (Chromium, SwiftShader WebGPU, `web/tests/machine_builder.cjs`) the machine's
bodies are drawn with the right shapes and poses but unlit (black, no edges, no view cube), while
the part before the builder opens is shaded normally, and the native build (lavapipe Vulkan,
`tests/machine_builder.rs`) draws the same meshes shaded with edges and the hit in red. No WebGPU
error reaches the console. Tried without effect: edge segments in every mesh (no empty
storage binding), the view set up as `show_loaded` does (`theme_view`, `ao`). Cause UNKNOWN. On a
real GPU browser: not tried (none here). Everything else works in the page: the template, jog,
live contact (status), check, save to the project folder through the page's storage, and the
native CLI listing the saved machine.
