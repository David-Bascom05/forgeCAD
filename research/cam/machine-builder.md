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

## 5. Tests and screenshots

- crate tests: writer round trips (every built-in and template), CLI end to end (template at
  project level with every value cited unknown, byte-stable file, parts in envelopes; clone at
  user level with `--own`; shadowing in `list`; refusals; a setup's simulation on the project
  machine, and the user's narrower X travel turning the same program into an overtravel).
- app: headless (native) screenshots of the workspace per template, jogging into a collision, the
  validation panel; the same in the browser build (Playwright); under
  research/cam/machine-builder/.
