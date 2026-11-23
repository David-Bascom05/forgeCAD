# CAM libraries: machines, posts, materials and feeds, fixtures and stock

2026-11-23. Research only. No kernel or interface code was changed. This note is the
"libraries" half that design.md §6 points to. It covers what Fusion and the others store, how they
store it, and what forgecad should store.

**Source marks.** [R] read directly today. [S] search-result snippet only, not the page itself.
[K] known from prior experience or the literature, not re-read today. Every [R] and [S] was taken on
2026-11-23 unless a date is given. Vendor text is restated, never copied. Where a vendor's file or
manual is under copyright ("All rights reserved"), only its facts are used here.

**Downloaded material** (Autodesk post guide PDF, five library `.cps` posts, the NIST report, the
Fusion simulation guide) was read in the session scratchpad and never saved to the repository.

---

## 0. Summary

- **Fusion separates three things**: the **machine definition** (kinematics tree, limits, spindle,
  post link, simulation model), the **post** (a JavaScript `.cps` with entry functions, run by
  Autodesk's engine), and **cutting data** (per-tool presets, with no link to material). These are
  the right seams. Fusion lacks cutting data tied to material and a documented file format.
- **Fusion's posts are already half data.** The current library posts open with tables: coolant
  codes, work-offset ranges (`wcsDefinitions`), smoothing levels, subprogram blocks, the
  `workPlaneMethod` flags and createAxis records. Code runs only after those tables. A
  **data-driven dialect table with a small fixed engine** in our Rust kernel covers the same ground.
  It needs no script interpreter and no third-party crate, and its output is bit-identical on native
  and wasm (§5.3).
- **Material cutting data** belongs in forge-material, as dotted `cam.*` keys beside `uts`. That
  data is the ISO group, kc1 and mc (Sandvik publishes ranges per group), and vc and fz by tool
  material and diameter. This is the "material drives feeds" feature that Fusion users ask for and
  do not get.
- **Fixtures** are forgecad parts plus a small `[[fixture]]` record that gives the datums and jaw
  range. Fusion has no fixture library. It has only a "Fixture" selection in the setup.

---

## 1. Machine library

### 1.1 Kinematic types

The axis chains below are written from the bed (static) out to each end of the chain.

| type | chain, table side / head side | notes |
|---|---|---|
| 3-axis VMC | static / X Y Z head (or X Y table, Z head) | No rotaries. The tool axis is always +Z. Fusion's 3-axis example has XYZ on the head and the stock still [R a]. |
| 4th axis (rotary on table) | static, A about X (or B about Y) / XYZ | A non-TCP 4th axis needs the origin on the rotary centreline, at the face where the part mounts [R b]. |
| 5-axis table-table (trunnion) | static, A or B tilt, C rotate / XYZ head | Fusion's table-table example has B and C on the stock [R a]. createAxis gives the order: table carrier, then table rider [R c]. |
| head-table | B or A tilt on head / C on table | The head offset is measured from the spindle face to the head pivot [R c]. |
| head-head | static, X Y table / Z, C, B (or A) head | Fusion's example has Z, C and B moving the head [R a]. The carrier offset runs from the rider pivot to the carrier pivot [R c]. |
| nutating head or table | rotary axis not orthogonal, e.g. axis (0, .7071, .7071) | createAxis accepts any axis vector [R c]. |
| mill-turn, B-axis head, sub-spindle | turret or B head with live spindle; main spindle C; sub-spindle Z2, C2 | Fusion has head, turret, main spindle and sub spindle tree items. A sub spindle can be marked "supportive operations" only, and has a "Reverse Z axis" flag [R d]. |
| lathe, live tools, Y | turret X Z (Y); main spindle C | The turret has a station count, attach points, and a live-tooling flag with min and max rpm [R d]. |

Sources: a = Fusion "Machine kinematics" help, help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-MACHINE-KINEMATICS.htm;
b = "Milling Machine Simulation Setup Guide", rev. 2025-03-06, cam.autodesk.com/posts/posts/guides/Milling%20Machine%20Simulation%20Setup%20Guide.pdf;
c = "CAM Post Processor Guide" (Training Guide), revision 4/2/25, §8.1.3, cam.autodesk.com/posts/posts/guides/Post%20Processor%20Training%20Guide.pdf;
d = "Work with machine kinematics", help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-MACHINE-EDIT-KINEMATICS.htm.

### 1.2 Fusion: Machine Library, machine definition, Machine Builder

**Library folders [R]** (MFG-MACHINE-LIBRARY-SELECTION.htm). These are a read-only *Fusion
Library*; *My Machines* subfolders, where you create, import and paste; *Cloud*, after a preference
is turned on; and *Linked*, a folder elsewhere on disk. Definitions are exported and imported as
files. The help does not name the file extension or describe its contents. The API can create a
machine from a "local machine file" (`MachineFromFileInput`, April 2023), and the help does not
name an extension there either [R] (Fusion-360-API/files/cam_MachineFromFileInput.htm). A forum post
calls these files `.mch` [S]. They are believed to be JSON text, but Autodesk documents no
schema [K]. **There is no public format to import from**: verify against a real export before
writing an importer.

**What a definition holds [R]** (MFG-MACHINE-DEFINITION.htm; guide §1.5.3): capabilities (milling,
turning, cutting), kinematics, multi-axis options, a required post link plus the NC output folder,
and the simulation model link. Per the guide, a definition lets one generic post serve many machines. It is required for machine
simulation and for operation properties. A post supports definitions if it has an `activateMachine`
function. The Fanuc, Haas Next Generation, Heidenhain, Hurco, Siemens and Tormach posts do.

**Kinematics tree items and their settings [R]** (MFG-MACHINE-EDIT-KINEMATICS.htm):

| item | settings |
|---|---|
| linear or rotary axis | coordinate (X Y Z, or A B C); home position; rapid feed; max feed; orientation. Rotary only: preference (direction), TCP checkbox, range. Advanced: resolution (continuous or stepped), axis function, offset (XYZ), reset. |
| head | spindle checkbox (clear it for a static turning tool); min and max speed; max tool length and diameter; orientation |
| turret | stations; attach points; live tooling with min and max speed; max tool length and diameter |
| spindle (turning) | min and max speed; **power; peak torque; peak-torque speed**; reverse Z (sub spindle) |
| static | base and enclosure |

The help gives no units for these settings. The API gives units for MachineAxis: cm for linear axes,
radians for rotary. `homePosition`, `toolChangePosition` and `physicalRange` read NaN when they are
not set. `resolutionStepSize` must be greater than 0 [R] (Fusion-360-API/files/cam_MachineAxis.htm).
**Spindle power is three numbers (P, Tpeak, n at Tpeak), not a curve** [R d].

**Simulation model [R b].** The machine CAD is a Fusion design, best obtained from the maker. Its
components must be grouped and named only X-axis, Y-axis, Z-axis, A-axis, B-axis, C-axis and
Base/Static/Enclosure. Surfaces are not collision-checked, so every part must be a solid. Moving
parts need small gaps between them; remove bearings and rails, or run Interference to check. The
origin goes at the table centre. For a non-TCP 4-axis or 5-axis machine with offset rotary centres,
put it at the rotary intersection and give the pivot distance as the axis offset (the guide's
example is 51.308). The Machine Builder's "Set Up Machine Model" dialog pairs the components with
the tree and picks the tool and part attach points. "Preview machine movement" then tests them. The
definition's other tabs: **Post Processing** (default post and output folder); **Multi-axis**
(retract-and-reconfigure on a limit, feedrate method, adjust singularities); and a per-axis
**collision** flag, which is slow if set on many axes.

### 1.3 The post-side machine model (createAxis), which is documented

Section 8.1.3 of the Fusion post guide [R c] is the one *documented* machine schema at Autodesk. A
post can define the machine in code; otherwise the definition is passed to it.

| createAxis field | meaning |
|---|---|
| `table` | true for a table axis, false for a head. Default true. |
| `axis` | rotation vector, any direction. Tables follow the right-hand rule and heads the left-hand rule. Required. |
| `offset` | the pivot point. For tables, measured from the setup's part origin. For a rider head, from the spindle face to its pivot. For a carrier head, from rider pivot to carrier pivot. It can be omitted with TCP. |
| `coordinate` | X, Y or Z (0, 1, 2). Required. |
| `cyclic` | continuous axis. Its range limits the output values, and it never triggers a rewind. |
| `tcp` | the control supports TCP for this axis. Default true. |
| `range` | [lower, upper]: physical limits, or output limits if cyclic |
| `preference` | -1 negative, 0 none, +1 positive, at the start of an operation |
| `reset` | 0 keep the angle; 1 zero it at operation start; 2 zero it at rewind; 3 both |
| `resolution` | actuator step, in degrees |

The order of axes in `new MachineConfiguration(...)` is head rider, head carrier, table carrier,
table rider. Multi-axis feeds are chosen with `setMultiAxisFeedrate`: FPM with TCP, otherwise
inverse time or DPM, with a maximum inverse-time value (9999.99) and a tolerance. The engine
computes the tool tip's real arc when the rotaries move, not the chord. The guide recommends
inverse time when the control has both [R c §8.7]. The library has a sample multi-axis post for
testing these settings ("RS-274D Sample Multi-axis Post Processor") [R c §8.1].

### 1.4 Other systems

| system | machine and control files | what they say |
|---|---|---|
| Mastercam | machine definition `.mmd` (mill), `.lmd` (lathe), `.rmd` (router), `.wmd` (wire); newer versions use the `.mcam-mmd` and `.mcam-control` names; control definition `.control` (`.mcam-control`); post `.pst` | [S] Scribd copies of "Working with MD/CD" and a 2023 Thermwood guide that names a `.mcam-mmd` file. One machine definition, one control definition and one post form a set, and the post names its control definition. [R] Mastercam 2027 blog (2026-07-28): a new panel-based Machine Definition Manager whose kinematic tree stays in sync with the graphics and is validated live, so a definition cannot be saved with an invalid axis combination. Local definitions are stored in the part file and refreshed only on an explicit reload. Definitions can be password-protected read-only. mastercam.com/community/blog/mastercam-2027-modernizes-machine-definitions/ |
| Siemens NX | Machine Tool Builder: a kinematic model of components and junctions; MCF (machine configuration) files; CSE (common simulation engine) controller models for 840D, Fanuc and Heidenhain | [S] The NXOpen `KinematicConfigurator` API (NX 12) has methods to create kinematic chains, channels and junctions, and to import an MCF. A Siemens KB article says to clear "NC Axis" on junctions that are not NC axes. NX 2512 added AI-assisted kinematics-tree creation in Machine Kit Wizard. Siemens docs need a login, so nothing was read directly. |
| VERICUT | `.mch` machine file (XML: construction, kinematics, properties); `.ctl` control file (XML: which G-codes it recognises, word formats, grouping and macro rules); `.xmch` and `.xctl` encrypted forms; `.tls` tool library (ASCII); `.VcProject` (XML); fixture and stock as `.fix`/`.stk` polygon files, or STL/STEP | [R] help.vericut.com/4_vericut_help/2.3_vericut_file_descriptions. Keep the original `.mch`, because the encrypted form cannot be expanded. **The separation of machine and control is the model to copy**: the same machine can run under different controls. |
| STEP ISO 10303-105:2019 (kinematics) | kinematic_link (a vertex) and kinematic_joint (an edge between two links); kinematic_pair subtypes: revolute, prismatic, cylindrical, screw, spherical, universal, gear, rack-and-pinion, low_order_kinematic_pair_with_range (lower/upper rotation and translation per axis), actuated_kinematic_pair; mechanism_representation; tree_structure must be acyclic | [R] downloads.steptools.com/stds/smrl/data/resource_docs/kinematics/sys/5_schema.htm and 6_schema.htm. Swapping a joint's ends negates its pair values and swaps its limits. AP242 carries it (the standard's hinge example is AP242). **This is the import and export route for machine kinematics**, and it fits forge-step. |
| LinuxCNC | kinematics modules: trivkins (Cartesian; `coordinates=` maps joints to letters, repeated letters for gantries), corexykins, xyzac-trt-kins and xyzbc-trt-kins (table-rotary-tilting, switchable with trivkins), 5axiskins (bridge mill XYZBCW), maxkins (B head, C table), genhexkins, pentakins, genserkins (DH parameters), scarakins, pumakins, tripodkins, rotatekins, rosekins | [R] linuxcnc.org/docs/html/man/man9/kins.9.html. [R] linuxcnc.org/docs/html/motion/kinematics.html: a module provides `kinematicsForward` (joints to pose), `kinematicsInverse`, `kinematicsType` (IDENTITY, BOTH, FORWARD_ONLY, INVERSE_ONLY) and `kinematicsHome`; modules may be switchable. **TCP on LinuxCNC is done in kinematics, by switching modules, not with a G-code** [K]; the Autodesk linuxcnc post sets `supportsTCP: false` [R, post grep]. |

**Recorded machine data, then (union of the above):** identity (vendor, model, control); the
kinematic chain, each joint with type, axis vector, pivot, range, home, tool-change position,
resolution, max rapid and max feed (and acceleration and jerk for time estimates, which none of
the sources above has); TCP support per axis; the spindle(s), with speed range and power and
torque; heads and turrets, with max tool length and diameter; the tool changer (pockets, max tool
weight, length and diameter, change time); coolant kinds supported; the post or dialect link; and
collision bodies.

---

## 2. Post processors

### 2.1 Fusion's JavaScript post engine

Sources: entry function reference, cam.autodesk.com/posts/reference/entry_functions.html [R], and
the post guide (rev. 4/2/25) [R c].

**Entry functions [R]**, in call order: onOpen (once, header); per section (usually one operation)
onSection, the motion and cycle calls, onSectionEnd; onClose (footer); onTerminate (once; the output
file is unlocked only here).

| group | functions (arguments) |
|---|---|
| motion | onRapid(x, y, z); onLinear(x, y, z, f); onCircular(clockwise, cx, cy, cz, x, y, z, f); onRapid5D and onLinear5D (x, y, z, dx, dy, dz [, f]) |
| state | onMovement(type), onRadiusCompensation(), onFeedMode(mode), onSpindleSpeed(s), onToolCompensation, onPower (jets), onDwell(sec), onOrientateSpindle(angle) |
| cycles | onCycle(), onCyclePoint(x, y, z) per hole, onCycleEnd() (an onCycleEnd is guaranteed after each onCycle); onCyclePath and onCyclePathEnd for turning cycles |
| commands and text | onCommand(id), onMachineCommand, onComment, onParameter(name, value) (the "CLD" stream), onPassThrough, onManualNC |
| multi-axis rewind | onRewindMachineEntry, onMoveToSafeRetractPosition, onRotateAxes, onReturnFromSafeRetractPosition |
| additive | onLayer, onLinearExtrude, onExtruderTemp, onBedTemp, onFanSpeed, ... |
| linking | onConnectionValidate (can return CONNECTION_IMPOSSIBLE) |

The script cannot replace entry functions after onOpen.

**Kernel settings [R c §5.1.1]** are globals the engine reads: `capabilities` (MILLING, TURNING,
JET, SETUP_SHEET, INTERMEDIATE, MACHINE_SIMULATION), `extension`, `tolerance`,
`linearOutputAccuracy` (digits; must match the xyz format), `highFeedMapping`/`highFeedrate`
(rapids mapped to G1), `mapToWCS`, `allowFeedPerRevolutionDrilling`, and arc control:
`allowedCircularPlanes` (bitmask of XY, ZX, YZ; 0 linearises all arcs), `allowHelicalMoves`,
`allowSpiralMoves`, `maximumCircularRadius` (larger arcs linearised), `maximumCircularSweep`,
`circularInputTolerance` (snaps near-helices to arcs), `circularMergeTolerance`, and
`circularOutputAccuracy` (moves centre and end so the rounded numbers lie on one circle).

**Formats and variables [R c §5.1.8 to 5.1.10]:** `createFormat({prefix, decimals, minDigitsLeft,
minDigitsRight, forceSign, scale, offset, base, maximum, minimum, separator, type})`. `type` is one
of FORMAT_INTEGER, FORMAT_REAL, FORMAT_LZS (no decimal point, leading zeros suppressed) and
FORMAT_TZS (trailing zeros suppressed). `createOutputVariable({control: CHANGED | FORCE | NONZERO,
type: ABSOLUTE | INCREMENTAL | DIRECTIONAL, cyclicLimit, cyclicSign, tolerance, onchange, prefix,
current}, format)`. This is what makes words modal: a word is printed only when its formatted text
changes. `createModalGroup({strict, force}, [[codes]...], format)`, e.g. M10/M11 and M12/M13 clamp
groups.

**Properties [R c §5.1.2 to 5.1.4]** are a `properties = {...}` table: title, description, group,
type (boolean, enum, number, integer, string), value, and scope (`post`, `machine`, `operation`).
The scope with the final say is the operation, then the post, then the machine. A machine-scope
value becomes the post dialog's default.

**Built-in settings tables in the current library posts [R c ch. 4]:**
- `coolant.coolants`: id → on and off codes, e.g. flood 8/9, through-tool 88/89, and
  flood+through-tool [8, 88]/[9, 89]. Coolant ids are FLOOD, MIST, THROUGH_TOOL, AIR,
  AIR_THROUGH_TOOL, SUCTION, FLOOD_MIST, FLOOD_THROUGH_TOOL and OFF. An id with no `on` code is
  unsupported and errors if an operation asks for it.
- `smoothing`: a level per roughing / semi / semi-finishing / finishing, picked by stock-to-leave or
  tolerance thresholds; also `retract`, `parametricFeeds`, `unwind`, `machineAngles`.
- `workPlaneMethod`: `useTiltedWorkplane` (G68.2, G254, PLANE SPATIAL, CYCLE800); `eulerConvention`
  (e.g. EULER_ZXZ_R, or undefined when the control takes rotary positions, as Haas does);
  `cancelTiltFirst`; `useABCPrepositioning`; `prepositionWithTWP`.
- `subprograms`: initial number, minimum cycle points, number format, and start, end and call
  blocks for external files and for embedded subprograms (M98 P / M99 in the sample), with a
  `%currentSubprogram` placeholder.
- `wcsDefinitions`: [R, from the posts]. Fanuc: G54–G59, then G54.1 P1–300. Haas NG: G154 P1–99.
  Siemens 840D: G54–G57, then G505–G599. LinuxCNC: G54–G59, then G59.1–G59.3. GRBL: G54–G59 only.

**Cycles [R c §5.29].** onCyclePoint receives the hole *bottom*. The post writes a canned cycle on the
first point and XY moves after that, or it calls `expandCyclePoint(x, y, z)` and the engine writes
plain moves. Expanding requires the post to handle the spindle and orientation commands the cycle
needs. The engine reads `machineParameters.drillingSafeDistance` and `spindleSpeedDwell`.

Cycle types and their usual codes: drilling G81, counter-boring G82, chip-breaking G73,
deep-drilling G83, tapping / left / right G84/G74, reaming G85, boring G86, stop-boring G87,
fine-boring G76, back-boring G77. Usually expanded: break-through-drilling, gun-drilling,
tapping-with-chip-breaking, circular-pocket-milling, thread-milling; bore-milling has no code given.
Cycle parameters include clearance, retract, stock, depth, bottom, incrementalDepth,
accumulatedDepth, dwell, the break-through feeds and speeds, and backBoreDistance.

**Licence [R].** Each library post's header reads "Copyright (C) 2012-2026 by Autodesk, Inc. All rights
reserved" (fanuc.cps, rev. 44242, 2026-09-07). forgecad must not copy them. Their facts (codes,
ranges) can be cross-checked.

### 2.2 The Fusion post library: numbers

The index the library page loads, cam.autodesk.com/posts/posts/posts-website.json [R], lists **595
entries, 473 of them visible**. Its newest entry is dated 2026-11-13. Visible posts by capability: 358 MILLING, 202
MACHINE_SIMULATION, 153 TURNING, 42 ADDITIVE, 41 JET, 19 INTERMEDIATE, 11 INSPECTION, 8 SETUP_SHEET,
5 CASCADING.

There are 207 vendors. The largest: Haas 53, Mazak 51, Autodesk 29, Jyoti 18, Siemens 11, Heidenhain 9,
Fanuc 7, Hurco 7. Generic and control posts found by name:

| control | posts (file names) |
|---|---|
| Fanuc | fanuc, fanuc compact, fanuc incremental, fanuc turning, fanuc inspection (plus 22 machine-specific posts that name Fanuc) |
| Haas | haas (classic), haas next generation, haas ec, haas desktop mill, haas st-10 and other lathes, haas inspection |
| Siemens | siemens-802d, 808d, 810d, 828d, 840c, 840d, siemens sinumerik one, siemens mill-turn, siemens turning |
| Heidenhain | heidenhain (conversational), heidenhain iso, 145, 155, 407, 426, heidenhain turning, millplus |
| Mazak, Okuma | mazak plus about 50 machine posts; okuma, okuma turning, okuma lb3000 mill-turn |
| LinuxCNC | linuxcnc, linuxcnc turning, emc |
| GRBL | grbl, grbl laser, grbl turning, carbide3d, boxzy |
| Tormach | tormach, tormach pathpilot turning, tormach 8l pathpilot turning, tormach jet |
| Datron | datron next, datron mcr, datron c5, datron iso, datron next inspection |
| neutral | rs274, rs274 multi-axis, apt, dump (prints every entry-function call, for debugging) |

### 2.3 Mastercam MP and FreeCAD

**Mastercam MP [S/R].** The `.pst` file is written in MP. "Postblocks" are named code blocks that are
triggered by NCI records (the neutral intermediate file; e.g. NCI G-code 81 means drilling).
Formats are declared with `fs2 <id> <lead>.<trail>` (with flags for leading and trailing zeros,
incremental and delta) and `fmt <address> <format> <variable>`; prompts are declared with `fq`.
Postblocks seen in a public v9 post (MasterCam_MACH3B.pst, machsupport.com) [R]: psof, peof,
pheader (file); ptlchg (tool change); prapid, plin, pcir (motion); pdrill, ppeck, pchpbrk, ptap,
pbore1 (drilling); psub_call_*, psub_st_*, psub_end_* (subprograms); pcant_* (canned text).

The MP Reference Guide (vols. 1 to 3) reaches users through resellers. Only Scribd copies were found,
so it is marked unreachable.

**FreeCAD CAM [R].** The posts are Python files in src/Mod/CAM/Path/Post/scripts. They come as
`*_post.py`, older `*_legacy_post.py` files beside them, and `*_pre.py` importers. There are about
45 files, among them centroid, dynapath, estlcam, fanuc, generic, generic_multi_axis_industrial,
grbl, haas, heidenhain, linuxcnc, mach3_mach4, marlin, masso_g3, opensbp, smoothie, svg and dxf
(github.com/FreeCAD/FreeCAD/tree/main/src/Mod/CAM/Path/Post/scripts). The new style is a class
that subclasses `PostProcessor` (Path.Post.Processor) and overrides schema methods:
`get_common_property_schema` (extension, preamble, postamble, safety block) and
`get_property_schema` (e.g. LinuxCNC's blend mode EXACT_PATH, EXACT_STOP or BLEND with a
tolerance). It reads a `Machine` model, and it supports tilted work planes by a `PlaneCommand`
(G68.2 for LinuxCNC). The licence is LGPL-2.1-or-later (linuxcnc_post.py, read via
raw.githubusercontent.com).
**FreeCAD is moving from free-form Python scripts to a schema plus hooks.** We could start there.
The FreeCAD wiki page CAM_Postprocessors was behind a bot wall and is unreachable.

### 2.4 NC dialects: the standards

- **ISO 6983-1:2009** [S] (iso.org/standard/34608.html): the word-address program format for
  positioning, line motion and contouring controls. Clauses cover the program format, blocks and
  words, and address indexing. It was confirmed in 2025 and replaced the 1982 edition. Paywalled;
  not read.
- **EIA RS-274-D** (1979) is the US base [R, cited in the NIST report §1.2.2].
- **NIST RS274/NGC Interpreter, Version 3.** T. Kramer, F. Proctor, E. Messina, NISTIR 6556,
  August 17, 2000, nvlpubs.nist.gov/nistpubs/Legacy/IR/nistir6556.pdf [R]. It specifies the
  language subset that EMC (later LinuxCNC) implements, built on the NCMS 1994 NGC
  part-programming specification. It turns code into "canonical machining functions". **Table 8,
  the order of execution within one block, is the reference our modal model should copy:**
  comment, feed mode, F, S, T, M6, spindle, coolant, overrides, dwell, plane, units, cutter comp,
  length comp, coordinate system, path mode, distance mode, retract mode, home or G10 or G92, then
  motion, then stop.
- **LinuxCNC G-code reference** [R] (linuxcnc.org/docs/html/gcode/g-code.html). It is the RS274NGC
  superset. Arcs: IJK is preferred over R. R>0 means a sweep under 180° and R<0 over 180°. Avoid R
  near 180° and 360°. The end-radius mismatch tolerance is 0.5 mm, or 0.005 mm plus 0.1% of the
  radius. G64 P (blend tolerance) and Q (collinear merge). G41.1/G42.1 take the diameter directly.
  G43.1 sets a dynamic offset and G43.2 adds to it. G93 is inverse time, G95 feed per revolution.
  Canned cycles: G73 and G81–G89 with R, Q, P, L; G98 retracts to the old Z, G99 to R.

### 2.5 Program skeleton and dialect table

| element | Fanuc / Haas (ISO) | Siemens 840D | Heidenhain TNC | LinuxCNC | GRBL 1.1 |
|---|---|---|---|---|---|
| program start and number | `%`, `O1001`; integer names (`programNameIsInteger` in fanuc.cps [R]) | `%_N_NAME_MPF` [K] | `BEGIN PGM NAME MM` / `END PGM` [K] | `%` optional; any name [K] | no number |
| comment | `( ... )` | `; ...` | `; ...` | `( ... )` or `;` [K] | `( ... )` or `;` [K] |
| line numbers | `N10` | `N10` | block numbers are mandatory and the control writes them [K] | `N` optional | `N` ignored [K] |
| safe start | `G90 G94 G17 G49 G40 G80` (the Fanuc post's onOpen block, [R] line grep), plus G21/G20 | `G17 G90 G94 G40 G71` [K] | `BLK FORM`, then `TOOL CALL` [K] | the same as Fanuc, plus G64 P | `G90 G94 G17 G21` (no G49, G40 or G80 needed) |
| tool change | `T1 M6`, `G43 H1` | `T1 D1 M6` (T and D) [K] | `TOOL CALL 1 Z S5000` [K] | `T1 M6 G43` (H defaults to T) [K] | no M6 support (the Autodesk grbl post has an "Output M6" option, off for most machines [R]) |
| work offsets | G54–G59; Fanuc G54.1 P1–300 (option, [R] post table); Haas G154 P1–99, and G110–G129 = G154 P1–20 [S] | G54–G57, G505–G599 [R post] | presets: `CYCL DEF 247` [K] | G54–G59.3 | G54–G59 |
| arcs | G2/G3 IJK or R; G17/18/19 | G2/G3 I J K or CR=; also CIP, CT [K] | `CC` then `C X Y DR-` (conversational) [K] | IJK preferred | G2/G3 IJK or R |
| canned cycles | G73 G81 G82 G83 G84 G74 G85 G86 G76; Q peck, P dwell, R plane; G98/G99 | `CYCLE81`–`CYCLE89`, `MCALL` [K] | `CYCL DEF 200` drilling, `203` universal drilling, `207` rigid tapping, ... [K] | G73, G81–G89 | **none**: the Autodesk grbl post defines no onCyclePoint, so holes become G0/G1 [R grep] |
| cutter comp | G41/G42 D | G41/G42 (D in T/D) | `RL`/`RR` on the approach block [K] | G41/G42 [D]; G41.1 | **none** (G40 only [R]); the post refuses radius comp [R grep] |
| high-speed mode | Fanuc `G05.1 Q1 [R1-10]` AICC, `G05.1 Q0` off [R post; S forum]; Haas `G187 P1/P2/P3 E<tol>` (rough, medium, finish; E overrides Setting 85; cancelled by reset and M30) [S Haas] | `CYCLE832(tol, mode)`, e.g. `CYCLE832(0.01, 11200<level>)` in the Autodesk post [R] | `CYCL DEF 32 TOLERANCE`, `32.1 T<tol>`, `32.2 HSC-MODE:<0 finish / 1 rough>` [R post]; DIN/ISO G62 [S] | `G64 P<tol> Q<tol>` | none |
| 5-axis TCP | Fanuc `G43.4 H` (`G43.5` with tool vector) [R post]; Haas `G234 H` (TCPC; rotaries at 0 first) [S] | `TRAORI` [K] | `M128` / `M129`, or `FUNCTION TCPM F TCP AXIS POS PATHCTRL AXIS` [R post] | kinematics switching, not a G-code [K] | none |
| 3+2 work plane | Fanuc `G68.2` [K]; Haas `G254`/`G255` DWO (positioning only; the work offset's B must be 0; G28, G29, G53 and M6 ignore it) [S] | `CYCLE800` [R post] | `PLANE SPATIAL ...` [R post] | `G68.2` (FreeCAD) [R] | none |
| optional stop, block delete | `M01`, `/` | `M01`, `/` | `M01` [K], `/` | `M1`, `/` | `M1`; no `/` [K] |
| coolant | M8 flood, M7 mist, M9 off; Haas M88/M89 through-spindle (the Fusion guide's sample [R c §4.1]) | M8/M9 | M8/M9 | M7 M8 M9 | M7 (compile option), M8, M9 [R] |
| subprograms | `M98 P<n>`, `M99` | `L<n>` or a call by name, `M17` [K] | `CALL LBL n` / `LBL n`, `CALL PGM` [K] | `o<n> sub` / `endsub`, `o<n> call` [K] | none |
| end | `M30`, `%` | `M30` | `END PGM` | `M2`/`M30` | `M2`/`M30` |

GRBL 1.1's whole set [R] (github.com/gnea/grbl/wiki/Grbl-v1.1-Commands): G0 G1 G2 G3, G38.2–G38.5,
G80; G17–G19; G90/G91 and G91.1; G93/G94; G20/G21; G54–G59; G43.1/G49; G40; non-modal G4, G10
L2/L20, G28/G30 (and .1), G53, G92 (and .1); M0 M1 M2 M30, M3 M4 M5, M7 M8 M9. G92 and G43.1
offsets are lost on reset.
**GRBL is a strict subset, so the dialect table must be able to say "not supported, expand or
refuse".**

**Heidenhain conversational vs ISO [S].** The TNCs take Klartext (conversational: `L X+10 R0 FMAX`,
`CC`, `C`, `CYCL DEF`) and DIN/ISO, with separate manuals (DIN/ISO manual ID 670 391-xx). Fusion
has separate `heidenhain` and `heidenhain iso` posts. Conversational is not G-code: every block is
a typed record. It fits a data table only if the table can say "this record kind renders as this
template".

**Siemens frames [K].** TRANS and ATRANS (offsets), ROT and AROT (rotations), SCALE and MIRROR are
programmable frames chained on G54. CYCLE800 sets a swivel frame from a "swivel data record" (the
Autodesk post has properties for the CYCLE800 mode and the data-record label [R]). The CYCLE832
`_TOLM` digits select machining type, transformation (TRAORI) and the G642 mode [S, Siemens
840D slides at ad.siemens.com.cn].

---

## 3. Material and cutting data

### 3.1 Material groups (ISO 513 letters) and Sandvik's kc1 [R]

Source: Sandvik Coromant, "Workpiece materials",
sandvik.coromant.com/en-gb/knowledge/materials/workpiece-materials, and "Specific cutting force"
(.../specific-cutting-force). kc1 is the force in the cutting direction for a chip 1 mm wide and
1 mm thick, at 0° rake; a more positive rake lowers it. The MC code is the ISO letter, the group,
the sub-group, the process letter (Z wrought, C cast, S sintered, U unspecified), and the heat
treatment (AN, HT, AG, UT, HA). Example: P1.2.Z.AN. Each code carries a nominal hardness, kc1, and
mc, which is typically 0.25.

| ISO | materials | machinability, chips | kc1 N/mm² (Sandvik) | carbide vc, orientation only [K] |
|---|---|---|---|---|
| P | unalloyed, low and high alloy steel, cast steel; ferritic and martensitic stainless is P5 | good, varies with C and hardness; long chips | 1400–3100 (MC codes about 1180 free-cutting to 3200) | 150–350 m/min |
| M | austenitic, super-austenitic, duplex, PH stainless | work-hardening, BUE, heat; machinability about 60% (austenitic) and about 30% (duplex) | 1800–2850 | 100–250 |
| K | grey, malleable, nodular, CGI, ADI iron | short chips, abrasive (SiC) | 790–1350 (MC about 680 CGI to 1650) | 150–350 |
| N | Al (<12–13% Si), Al-MMC, Mg, Cu, brass, bronze, Zn | long chips, easy if alloyed; pure Al is sticky; >12% Si is abrasive | Al about 350–700; Cu alloys about 550–1350 | Al 300–1000+ |
| S | Fe, Ni and Co HRSA; Ti (CP, α, α-β, β) | segmented chips; Ti conducts heat poorly and galls | HRSA 2400–3100; Ti 1300–1400 | Ni 20–60; Ti 40–90 |
| H | hardened steel 45–68 HRC, chilled iron, hard coatings | mostly finishing; CBN or ceramic for turning, carbide for milling to about 60 HRC | 2550–4870 (MC about 3090 at 50 HRC to 4750 at 63 HRC) | 50–150 |

[S] Other tables disagree. An ISCAR ISO 513 table gives stainless 1875–2150. Inch catalogues use a
0.0016 in² reference section, so their values are not interchangeable with metric ones. **Store
kc1 and mc with their source and reference chip thickness, never as a bare number.**

### 3.2 Formulas (standard; derivations are ours)

The symbols: D cutter diameter; z flutes; ae radial and ap axial engagement; κ entering angle (90°
for a square end mill); fz feed per tooth.

| quantity | formula | mark |
|---|---|---|
| spindle speed | n = 1000·vc / (π·D) [rpm; vc in m/min, D in mm] | [K] |
| table feed | vf = fz · z · n [mm/min] | [K] |
| engagement angle | φ = arccos(1 − 2·ae/D) for ae ≤ D; φ = π for a full slot | derived |
| max chip thickness | hex = fz · sin κ · sin φ = fz · sin κ · √(1 − (1 − 2ae/D)²) = 2 · fz · sin κ · √((ae/D)(1 − ae/D)) when ae < D/2; hex = fz · sin κ when ae ≥ D/2 | derived (same geometry as the user's form) |
| chip-thinning feed | fz = hex_target / √(1 − (1 − 2ae/D)²) when ae < D/2. Harvey and Helical say thinning applies below 50% radial engagement [R, "Combat Chip Thinning", harveyperformance.com; the formula on that page is an image] | derived + [R] threshold |
| mean chip thickness | hm = (1/φ)∫₀^φ fz·sin κ·sin θ dθ = fz · sin κ · (1 − cos φ)/φ = 2 · fz · sin κ · ae/(D·φ) | derived. Check it against Sandvik's printed form, which on that page is an image. |
| ball nose, effective diameter (no tilt) | Deff = 2·√(ap·(D − ap)) for ap < D/2; use Deff in n = 1000·vc/(π·Deff) | [S] mmsonline, nachiamerica, dapra (agree) |
| ball nose, tilted β (toward the feed) | Deff = D · sin(β + arccos(1 − 2ap/D)), capped at D | derived [K]; check against Mitsubishi's "inclined" formula (not reachable) |
| specific force | kc = kc1 · hm^(−mc) · (1 − γ0/100) (γ0 in degrees) | [K] Sandvik form; the page renders it as an image |
| net power, milling | Pc [kW] = ap · ae · vf · kc / (60 · 10⁶) [mm, mm/min, N/mm²]; the spindle needs Pc/η | [K] |
| torque | Mc [N·m] = Pc · 30 000 / (π · n) | [K] |
| removal rate | Q [cm³/min] = ap · ae · vf / 1000 | [K] |
| deflection, cantilever | δ = F·L³ / (3·E·I), I = π·De⁴/64, with De ≈ 0.8 D in the fluted part; F ≈ tangential force from kc | [K] (G-Wizard and HSMAdvisor compute deflection; their models are not published) |
| tool life, Taylor | v·T^n = C; extended: v·T^n·f^a·d^b = C. n is about 0.08–0.2 for HSS, 0.2–0.5 for carbide, 0.5–0.7 for ceramic; sources disagree | [S] course notes (KSU), Lund assessment paper |

**HSM and HEM rules [S/R]:** light ae with heavy ap (up to the flute length) and constant engagement
(adaptive) toolpaths, which are what make the chip-thinning feed increase safe. Use 5 to 7 flutes
for HEM [S, Harvey tag archive]. Helical gives 10–15% of D for "controlled slice" slotting [S,
guidebook excerpt]. Higher ap needs lower ae for chip evacuation [S, Autodesk blog "constant cutter
forces"]. The Helical HEM guidebook PDF (a mae.ufl.edu mirror) returned HTML, not the PDF:
unreachable.

### 3.3 Fusion: presets and feed optimisation

- **Presets [R]** (MFG-CREATE-TOOL-PRESET.htm): each tool has a "Cutting data" tab with several
  named presets. The first is a copy of the default. They are computed from surface speed and feed
  per tooth or per revolution, and can be driven by expressions of the tool geometry; a preset
  pasted onto another tool recomputes its expressions. The help suggests one preset per material,
  machine or rough/finish case. **Material does not drive feeds in Fusion.** An Autodesk forum
  reply says so; users make one preset per material, or one library per material [S].
- **Tool library JSON [S].** A tool has `geometry` (DC diameter, LCF, NOF flutes, ...: ISO 13399
  names) and `start-values.presets[]` with `n`, `v_c`, `f_z`, `v_f`, plunge, ramp, coolant, a guid
  and a name. A vendor says the current schema is "tools.json version 36". Autodesk's schema was
  not found.
- **Feed optimization [R]** (help GUIDCFB956EA-...htm): on the Passes tab. Its settings: maximum
  directional change (angle), reduced feed radius (the minimum radius before the feed is cut),
  reduced feed distance (how far before the corner), reduced feedrate, and only inner corners.
  Forum reports [S] say dense line segments hide a corner from it unless smoothing fits arcs. This
  is *geometric* corner slowdown, not engagement-based or force-based feeds.

### 3.4 Tool-maker and third-party calculators

| tool | inputs | outputs and methods | mark |
|---|---|---|---|
| Harvey/Helical Machining Advisor Pro (MAP) | a Harvey or Helical tool (over 27,000 SKUs), material, toolpath, machine; RDOC can be locked while ADOC is varied | speed and feed, adjustable ±20% in 1% steps; it uses the tool's full geometry; web, offline and mobile | [S] harveyperformance.com/in-the-loupe/machining-advisor-pro-1-5-improvements, MAP for Harvey Tool |
| Sandvik CoroPlus ToolGuide and Tool Library | task or operation, machine (spindle speed, power and torque limits), material from Sandvik's list, with its hardness | recommended cutting data; tool assemblies exported to CAM; a Fusion add-in saves cutting data as Fusion presets, with the data mapping done on Sandvik's servers | [S] sandvik.coromant.com .../coroplus-tool-guide, .../fusion-360-plugin |
| Kennametal NOVO | machining feature (e.g. a blind hole), material, tolerance | a rules-based Tool Advisor; tool assemblies built with "fits-with" logic; ISO 13399 export of models, dimensions and feeds to CAM (Mastercam, Esprit, Vericut, ...) | [S] kennametal.com/us/en/resources/novo/select.html |
| G-Wizard (CNCCookbook) | machine profile (rpm, feed, HP, power curve), material (1232, by family, alloy and condition), tool (type, D, z, stickout, geometry), ap and ae, and a "Gas Pedal" slider from tool life to MRR | rpm, feed, SFM, IPT, IPR, chip thinning and AFPT below 50% ae, ball-nose compensation, MRR, HP (it lowers feed if over the limit), predicted deflection, and a Cut Optimizer (minimise deflection) | [R] s3.amazonaws.com/s3.cnccookbook.com/GWCalcFeedsSpeeds.htm |
| HSMAdvisor | tool, material (over 300 grades [S]), toolpath, spindle and holder | a virtual tool model for maximum deflection and torque; chip thinning and HSM; ap and ae suggested from tool and machine power; torque/speed curves and a machine force limit [S]; a Mastercam plugin and an SDK | [R] hsmadvisor.com |
| FSWizard (same developer) | material, operation (mill, drill, tap, turn), tool, engagement | SFM, IPT, rpm, feed, power, optimal ap and ae, chip thinning and HSM data | [R] zero-divide.net/?page=fswizard |

**What they all have that Fusion lacks:** a power and torque check against the machine curve, a
deflection estimate, chip-thinning compensation, and material as an input.

---

## 4. Fixtures and stock

### 4.1 Commercial workholding facts (restated)

| product | facts | mark |
|---|---|---|
| Kurt DX6 vise | 6" jaw width, 9.08" max opening, 1.735" jaw depth, 16.81" long; 7,342 lb listed, likely the clamp force (unlabelled); the DX6H hydraulic version lists 6,700 lb | [S] penntoolco.com, kbctools.ca |
| Orange Vise 6" | dual station, 17.5", 20", 22" and 26" bodies; GEN4; the 26" converts to single station; max opening not found | [S] maritool.com listings |
| Mitee-Bite | Pitbull edge clamps (inch and metric; knife-edge or blunt; cap screw plus O-ring in a milled slot) and TalonGrip serrated fixed-side grips that hold on 1.5 mm (0.060") of material; e.g. 33150: 25.4×25.4×12.7 mm, M8, 1.5–5.6 mm range | [S] miteebite.com catalog 2024, haastooling.com |
| LANG Quick-Point | zero-point grids at 52 mm (16 mm studs, 4 t pull-down) and 96 mm (20 mm studs, 6 t); repeatability 0.005 mm; combo 52/96 plates, 5-axis risers, twin bases rotating 4×90° | [S] lang-technik.de FAQ and product pages, mmsonline |
| 5th Axis RockLock | 52 and 96 mm bases; repeatability <8 µm; the PS16F stud is required on 52 mm; self-centring vises ±0.0127 mm | [S] 5thaxis.com product pages |
| soft jaws | machined to the part's profile, a per-job part in practice | [K] |

### 4.2 Fusion's handling

- **Fixtures [R]** (MFG-MM-MODEL-EX1.htm). Make a *manufacturing model*, derive the vise components
  into it, position them, then pick them in the setup's **Fixture** group. Every toolpath in the
  setup then avoids them, and collision checking covers the tool, the holder and the fixtures. **No
  fixture or vise library is built in.** There are sample vises in the Fusion samples, a forum idea
  asking for a library [S], and partner add-ins (5th Axis "Workholding Intelligence", Renishaw
  fixturing) [S; the Autodesk blog returned 403]. Soft jaws are modelled by hand [K].
- **Stock [R]** (MFG-REF-SETUP-STOCK.htm). Modes: fixed size box (W, D, H; position per axis: left,
  centre, right, model origin); relative size box (offsets: none, sides plus top/bottom, or each
  side; rounded up to an increment); fixed and relative cylinder (diameter and length, or radial and
  front/back offsets); fixed and relative tube (OD, ID, length); from solid (e.g. a forging); from
  bounding solid; and **from preceding setup**, the stock transfer for rest machining ("continue
  rest machining" is for turning only). Stock material is not set here; no stock library is
  documented.

---

## 5. Recommendations for forgecad

Constraints: zero third-party crates in the kernel; native and wasm bit-identical; every value
carries its source (forge-material's rule); vendor data restated with source and date, never
copied. These match design.md §5.5–§6.

Marks: **M** must-have (phase 1–2), **D** differentiator, **L** later.

### 5.1 Where each library lives

| library | built-in (read-only) | user | project | crate |
|---|---|---|---|---|
| machines | `kernel/forge-tooling/data/machines/*.toml` | `~/.config/forgecad/cam/machines/` | `cam/machines/` | forge-tooling |
| dialects (posts) | `kernel/forge-post/data/dialects/*.toml` | `~/.config/forgecad/cam/posts/` | `cam/posts/` | forge-post |
| cutting data | `kernel/forge-material/data/materials.toml` (`cam.*` keys) plus `cam/cutting.toml` for tool-material factors | `~/.config/forgecad/materials/` (already exists) | `materials/` (already exists) | forge-material |
| fixtures | `kernel/forge-tooling/data/fixtures/*.toml` plus `.fg` parts | `~/.config/forgecad/cam/fixtures/` | `cam/fixtures/` | forge-tooling |
| stock sizes | `kernel/forge-tooling/data/stock.toml` | the same pattern | the same pattern | forge-tooling |

The search order is project, then user, then built-in, as `forge_material::Libraries::load` does
now. All files use forge-material's TOML subset (`[[record]]` blocks of `key = value`, where a
value is a string, a number or a flat list). That reader should be lifted into a shared module so
that every library parses the same way (**M**).

### 5.2 Machine file (sketch)

```toml
# cam/machines/generic-trunnion-bc.toml -- our own placeholder numbers, no vendor's model copied
[[machine]]
name = "generic trunnion BC"
kind = "table-table"          # vmc3 | vmc4 | table-table | head-table | head-head | lathe | mill-turn
control = "fanuc"             # a dialect name (5.3): machine and control kept apart, as VERICUT does
units = "mm"
source = "forgecad generic; placeholders, not a real machine"

[[joint]]                     # bed outward; `parent` makes a tree (mill-turn has two branches)
name = "X"
parent = "base"
side = "head"                 # head | table, as in Fusion's tree
type = "linear"               # linear | rotary = ISO 10303-105 prismatic | revolute pair
axis = [1, 0, 0]
range = [-300, 300]           # mm (deg for rotary)
rapid = 30000                 # mm/min; also max_feed, home, tool_change
accel = 4000                  # mm/s^2: no source above exposes it; cycle time needs it

[[joint]]
name = "B"                    # then C with parent = "B", axis = [0, 0, 1], cyclic = true
parent = "base"
side = "table"
type = "rotary"
axis = [0, 1, 0]
pivot = [0, 0, -50]           # from the table-top centre, as the Fusion simulation guide places the origin
range = [-120, 120]
preference = 1                # -1 | 0 | 1 and reset = never | op-start | rewind | both: createAxis semantics
tcp = true

[[spindle]]
rpm = [100, 12000]
curve_rpm = [0, 1500, 12000]  # piecewise-linear power curve: constant torque, then constant power
curve_kw = [0, 11, 11]        # Fusion keeps only P, Tpeak and n at Tpeak
max_tool = [250, 125]         # length, diameter; plus taper = "BT40"

[[changer]]
pockets = 24                  # plus max_tool_kg, change_s
coolants = ["flood", "mist", "through", "air"]   # kinds present; their codes live in the dialect

[[body]]                      # optional collision bodies, as forge-asm occurrences
joint = "B"
part = "bodies/trunnion.fg"
```

**M**: kind, joints with ranges and tcp, spindle rpm, control link (what a 3-axis or 3+2 post and
the limit checks need). **D**: the power curve, accelerations, collision bodies, and certified limit
and collision checks through forge-brake; Fusion stores three spindle numbers, we store the curve
and check every cut against it (§5.4). **L**: mill-turn channels, sub-spindle transfer, ISO
10303-105 import/export via forge-step, and Fusion `.mch` import once checked against a real export.

### 5.3 Post: a data-driven dialect table, not a script

**Why not scripting.** Fusion and FreeCAD use a general language (JavaScript, Python). An embedded
interpreter in kernel/ would be a third-party crate or one we build ourselves. Either way, the
output would depend on a language runtime, which makes native/wasm bit-identity and the Lean
round-trip proof (design.md G6) harder.

**Why data is enough.** The evidence is §2.1: Fusion's own current posts reduce most variation to
tables (coolant, wcs ranges, smoothing, subprograms, workPlaneMethod, properties). FreeCAD's new
posts reduce it to a schema plus a few hooks. What remains between dialects is word formats,
which codes exist, block templates, and "supported, expand or refuse": all data.

**The engine (Rust, fixed):** a modal state machine in the NIST order of execution (Table 8);
Fusion-style output variables (a word prints only when it changes, forced after a reset); an arc
policy (planes, IJK or R, max sweep and radius, helix and spiral, else linearise to tolerance); a
cycle policy per kind (native template, expand, refuse); comment escaping, line numbering,
program-number rules; and numbers printed from integer-quantised values (round to the dialect's
resolution in exact integer arithmetic, then print digits), so the text is identical on every
target (**M**).

```toml
# kernel/forge-post/data/dialects/fanuc.toml
[[dialect]]
name = "fanuc"
extension = "nc"
program = "O{num:04}"                  # integer 1..9999
program_open = ["%", "{program} ({title})"]
program_close = ["M30", "%"]
comment = "({text})"                   # with comment_strip = "()"
line_start = 10                        # line_step = 10, line_max = 99999 (flat keys: the subset has no inline tables)
safe_start = "G90 G94 G17 G49 G40 G80 {units}"   # units_mm = "G21", units_in = "G20"
decimals = [3, 4, 3, 1]                # mm, inch, angle, feed
source = "restated: Fanuc-style ISO practice, NIST RS274NGC v3 Table 8; codes cross-checked against Autodesk fanuc.cps (facts only)"

[[code]]                               # what exists; anything absent is refused or expanded
key = "tool_change"
text = "T{t} M06"                      # likewise length_comp "G43 H{h}", tcp_on "G43.4 H{h}",
                                       # smooth_on "G05.1 Q1 R{level}", smooth_off "G05.1 Q0"
[[wcs]]                                # ordered ranges, like Fusion's wcsDefinitions
format = "G54.1 P{n}"                  # after format = "G{n}", range = [54, 59]
range = [1, 48]                        # 48 standard, 300 with the option: a machine property

[[arc]]
planes = ["xy", "zx", "yz"]
mode = "ijk"                           # ijk | r | both; plus max_sweep, helix, spiral

[[cycle]]
kind = "peck-full"                     # drill | spot-dwell | peck-chipbreak | peck-full | tap | tap-left | ream | bore | fine-bore | back-bore
first = "G98 G83 X{x} Y{y} Z{z} R{r} Q{q} F{f}"
next = "X{x} Y{y}"
cancel = "G80"
missing = "expand"                     # native | expand | refuse, per cycle

[[coolant]]
kind = "through"
on = "M88"                             # Haas style; absent -> a typed NeedCoolant refusal, not silence
off = "M89"
```

GRBL then differs by data alone: no `tool_change` (a typed `ManualToolChange` pause, or a refusal
per job), no `[[cycle]]` (every cycle expanded), no cutter-comp code (always compensated in CAM),
one `[[wcs]]` range.

Heidenhain conversational differs in kind. The same engine covers it with per-record templates
(`L X{x} Y{y} R0 F{f}`, `CC`/`C`), because the engine already emits typed records. It is **L**.

**M**: Fanuc-generic, Haas, LinuxCNC, GRBL; the reader that parses our output back (design.md G6).
**D**: typed refusals with fixes ("dialect lacks G83; SetArg cycle=expand"); `query F post` diffing
two dialects on one program; dialects tested by round trip. **L**: Siemens (CYCLE800, CYCLE832 as
templates), Heidenhain ISO and conversational, Mazak, Okuma, and multi-axis inverse-time feeds
(computed in Rust from the machine chain, as Fusion's engine does).

**Escape hatch (L).** A dialect may name a `.fg`-language function for one block kind. That keeps
customisation inside our own deterministic interpreter, not a new scripting runtime.

### 5.4 Cutting data: extend forge-material

The existing dotted extra keys (`thermo.window`, `source_thermo.window`) are the pattern. Add `cam.*`
keys per material, each with its own `source_cam.*`:

```toml
[[material]]
name = "6061-T6"
# ... existing density, youngs, yield, uts, cte, source ...
cam.iso = "N"
cam.mc_code = "N1.2.Z.AG"                     # Sandvik-style code, if the source gives one
cam.kc1 = 700                                  # N/mm^2 at hm = 1 mm, rake 0
cam.mc = 0.25
source_cam.kc1 = "Sandvik Coromant, Workpiece materials, N group: aluminium kc1 350-700 N/mm2, https://www.sandvik.coromant.com/en-gb/knowledge/materials/workpiece-materials (read 2026-11-23)"
kind_cam.kc1 = "cited"
cam.vc_carbide = [300, 1000]                   # m/min; [K] placeholder: must be sourced before it ships
cam.fz_per_d = [0.004, 0.010]                  # fz/D, carbide end mills; [K] placeholder; vendor tables win
```

- `cam/cutting.toml` (forge-tooling) holds what depends on the tool rather than the material:
  factors per tool material and coating, flute-count and helix limits, and a ramp and plunge
  fraction.
- A vendor's tool `[[tool.preset]]` (design.md §6) overrides the material default for that tool.
  Precedence: tool preset, then material and tool-material data, then group defaults. The reply
  names the source of every number, so an agent can see why the feed is what it is (**D**).
- **The engine** computes n and vf from vc and fz. It adjusts for chip thinning (§3.2, hex) and for
  ball-nose Deff at the actual ap and tilt. It checks the power against the spindle curve
  (kc → Pc → spindle kW at n) and the torque. It estimates deflection against a limit. It reports
  Taylor tool life only when C and n are sourced; otherwise it does not report it.
  **M**: n, vf, chip thinning, ball nose. **D**: power, torque and deflection checks on every cut,
  with fixes such as "SetParam ae ...", from the engagement the exact offset engine already knows
  (Fusion's feed optimisation looks only at corner geometry). **L**: tool life, cost, wear.
- Numbers are kept as sourced ranges and never invented. A material with no `cam.*` data gives
  `NeedCuttingData` with a fix that adds a cited range (**M**).

### 5.5 Fixtures and stock

```toml
# cam/fixtures/generic-vise-150.toml
[[fixture]]
name = "generic 150 mm vise"
kind = "vise"                          # vise | plate | zero-point | chuck | soft-jaw | clamp
part = "generic-vise-150.fg"           # a forgecad part or assembly: the collision body
jaw_width = 150
open = [0, 230]                         # the jaw range in mm; the moving jaw is a joint (forge-asm)
jaw_height = 44
datum = "fixed_jaw.face"               # the anchor the part locates on
mount = "zero-point-96"                # optional: what it sits on
source = "forgecad generic; dimensions are not a copy of any product"

[[fixture]]
name = "zero-point plate 96"
kind = "zero-point"
grid = 96                               # stud pitch, mm (LANG and 5th Axis use 52 and 96)
stud_d = 20
repeat_mm = 0.005                       # as a vendor states it; a placeholder until sourced per product
part = "zp-plate-96.fg"
```

**M**: a fixture is a forgecad part placed in the setup with a "fixture" role; toolpaths avoid it
and the checker proves clearance against it as for the part. Built-ins: a generic 150 mm vise, a
fixture plate, a 3-jaw chuck. **D**: soft jaws as a parametric feature: the part's own profile plus
a clearance cut into a jaw blank (a boolean the kernel already does), associative, so the jaws
re-cut when the part changes (Fusion users model them by hand). **L**: vendor products (Kurt,
Orange Vise, Mitee-Bite, LANG, 5th Axis) as restated, sourced profiles with "verify before
ordering", as forge-vendor does.

**Stock (M)** is in the document: `box` and `cylinder` (fixed or relative, with Fusion's offset
modes and round-up), `tube`, `from: solid`, and `from: setup` (the previous setup's in-process
stock: exact where the stock boolean allows; Fusion's is a mesh). A `[[stock]]` list of the shop's
bar and plate sizes lets the planner choose the smallest that fits (**D**).

### 5.6 Checklist

| item | mark |
|---|---|
| Shared TOML-subset reader lifted from forge-material | M |
| Machine: kind, joints (range, home, tcp), spindle rpm, control link | M |
| Machine: power curve, accelerations, collision bodies, certified limit checks | D |
| Machine: STEP 105 kinematics import and export; Fusion .mch import once its format is checked | L |
| Dialect engine with integer-quantised number output, NIST execution order, arc and cycle policies | M |
| Dialects: Fanuc-generic, Haas, LinuxCNC, GRBL; round-trip reader | M |
| Typed refusals with fixes for unsupported codes | D |
| Siemens, Heidenhain ISO and conversational, Mazak, Okuma; multi-axis inverse-time feeds | L |
| `cam.*` keys in forge-material: iso, kc1, mc, vc ranges, fz/D, each with a source | M |
| Feeds engine: n, vf, chip thinning, ball-nose Deff | M |
| Power, torque and deflection checks per cut from true engagement | D |
| Taylor tool life (only when sourced) | L |
| Fixture role in setups; generic vise, plate, chuck | M |
| Associative soft jaws | D |
| Vendor fixture profiles | L |
| Stock modes, including from-setup | M |
| Shop stock-size list | D |

---

## 6. Unreachable or not read

| page | why |
|---|---|
| Autodesk Fusion blog: "Learn How Machine Builder Works", "How To Use the Machine Builder", "5th Axis Workholding Intelligence add-in" | HTTP 403; the WP-JSON mirror was not fetched |
| Fusion `.mch` format | not documented in the help or the API pages read |
| Haas G-code pages (G187, G154, G234, G254) and "mill - g codes" | HTTP 403; facts above are from search snippets of those pages |
| FreeCAD wiki CAM_Postprocessors | bot wall (Anubis) |
| GitHub API for FreeCAD | session access not enabled; the tree and raw files were read through the web instead |
| Sandvik milling formulas page | the formulas are images; only the variable list was read |
| MMC Carbide milling power formula page | HTTP 403; the cutting-conditions page has no formulas in its text |
| Helical Machining Guidebook (mae.ufl.edu mirror) | returned HTML, not the PDF; Helical's own URL returned 404 |
| Mastercam MP Reference Guide, "Working with MD/CD" | reseller-distributed; only Scribd copies, which were not used |
| Siemens NX Machine Tool Builder and SINUMERIK manuals | login or manualslib only; snippets used |
| Heidenhain TNC manuals | PDFs found, not read; facts from Autodesk post code and snippets |
| ISO 6983-1, ISO 513 | paywalled |
| HSMAdvisor user manual | not found; the home page was read |
| Orange Vise max opening | not found |
