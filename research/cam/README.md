# CAM for forgecad: prior art, the tools best-in-class needs, and how to add it

2026-11-23. Research only: no kernel or interface code was changed. Written for the owner.

The owner's ask: *"a prior art search on CAM. I believe that Fusion is currently the best here. I
want to know how to add it and all the tools it needs to be best in class."*

This file is the answer and the index. The detail, with every source, is in eight notes:

| note | what | lines |
|---|---|---|
| [commercial.md](commercial.md) | Fusion's full toolpath list (base vs Manufacturing Extension); Mastercam, hyperMILL, NX, ESPRIT, SolidCAM, PowerMill, CAMWorks, Onshape and 15 others; market data; "is Fusion best?" | 386 |
| [fusion-workflow-2d-3d.md](fusion-workflow-2d-3d.md) | Fusion's workflow step by step (setup, WCS, stock, probing, templates, NC programs, setup sheets), the five shared parameter tabs, every 2D and 3D strategy with its parameters. About 120 Fusion help pages read. | 840 |
| [fusion-advanced.md](fusion-advanced.md) | Multi-axis and tool-axis control, turning and mill-turn, probing and inspection, additive, simulation, the accuracy model, hole recognition, AI, the CAM API | 712 |
| [libraries-tools.md](libraries-tools.md) | Cutter types and parameters, tool assemblies, ISO 13399, GTC, Fusion's tool JSON schema, Mastercam, FreeCAD, LinuxCNC, holders and tapers; our `[[tool]]`, `[[holder]]` and `[[assembly]]` TOML | 709 |
| [libraries-machines-posts-feeds.md](libraries-machines-posts-feeds.md) | Machine kinematics and definitions, Fusion's post engine and library, the NC dialects (Fanuc, Haas, Siemens, Heidenhain, LinuxCNC, GRBL), cutting data and formulas, the calculators, fixtures and stock; our machine, dialect and fixture TOML | 725 |
| [opensource.md](opensource.md) | FreeCAD CAM, OpenCAMLib, libarea, Kiri:Moto, PyCAM, Fabex, OpenVoronoi, Clipper2, cavalier_contours, LinuxCNC, GRBL, CAMotics, Rust crates; a licence table for MPL-2.0 | 289 |
| [literature.md](literature.md) | The research literature on ten topics, and what can be exact, what needs certified intervals, and the honest specs | 570 |
| [design.md](design.md) | The forgecad design: crates, reuse, the `.fg` surface, the reply schema, the Lean spec, the six levels, libraries, the phased plan | 291 |

**How sources are marked** in every note: **[R]** read directly this session (2026-11-23);
**[S]** search snippet only; **[K]** known, not re-read (treat as a lead). Fusion's help is
plain HTML at `https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/<ID>.htm`. That is where
most [R] Fusion facts come from; below, "Fusion help `<ID>`" means that URL. Autodesk's blogs and
marketing pages, Haas's G-code pages, iso.org, Learn courses and the FreeCAD wiki refused the
fetcher. Each note lists what it could not reach (for example
[libraries-machines-posts-feeds.md §6](libraries-machines-posts-feeds.md),
[fusion-advanced.md "Pages I could not reach"](fusion-advanced.md)). No proprietary text is
copied; everything is restated.

---

## 0. The answer in one page

**Is Fusion best?** Partly. Fusion is the best *value* and the best *integrated* CAD + CAM for
2.5D, 3-axis, 3+2, light 5-axis and turning. It is the reference for small shops, education and
makers. The base subscription costs about US$680 a year and includes 2D, 3D, 3+2, turning,
mill-turn, WCS probing, machine simulation, free editable posts and setup sheets. The paid
Manufacturing Extension adds simultaneous 5-axis, collision tilting, Steep and Shallow, Corner,
Deburr, Hole Recognition, toolpath editing and in-process probing. Sources: Autodesk's comparison
PDF (dated 2025-05-28) [R]
https://damassets.autodesk.net/content/dam/autodesk/www/pdfs/fusion-manufacturing-extension-comparison-updated-v3.pdf
and Fusion help `MFG-MULTI-AXIS-MILLING-OVERVIEW` [R]; prices from G2 [S] (commercial.md §2.1).

Fusion is **not** best at four things:

| area | who leads | evidence |
|---|---|---|
| high-end 5-axis surfacing, barrel/tangent-plane finishing, blades and impellers | hyperMILL (MAXX), NX, PowerMill | vendor docs [R]/[S], practitioner consensus [K]; no independent benchmark exists (commercial.md §4) |
| mold and die automation | WorkNC, Cimatron, PowerMill | [S] |
| multi-channel mill-turn and Swiss | ESPRIT, NX, GibbsCAM | Fusion has no channel synchronisation in its help [R]; NX has a Synchronization Manager [R] (fusion-advanced.md §2.5) |
| G-code-level verification | Vericut, NCSIMUL | Fusion simulates toolpath data, not the posted code [R] (fusion-advanced.md §5.3) |

By installed professional seats, Mastercam is #1. That is vendor-cited CIMdata data and old [R].
In CNCCookbook's 2024 survey, a self-selected sample of about 300, Fusion had the largest share
but was falling and Mastercam was rising [R]
https://www.cnccookbook.com/cnccookbook-2024-cam-software-survey-which-is-best. Confidence in
any "who is best" ranking is medium-low: free data on vendor share is paywalled, vendor-cited or
self-selected (commercial.md §3).

**What no one has.** Fusion defines its accuracy against a triangulated model: machining
tolerance is toolpath tolerance plus triangulation tolerance, with smoothing on top. Fusion help
`GUID09E44604-DAD8-47D6-ADC6-C100869DE724` [R]. Forum users report that 2D toolpaths are not
model-aware, so simulation is the gouge check [S] (fusion-advanced.md §5.4). No vendor read
promises that the real swept tool stays within a tolerance of the exact surface. No paper or
product proves a toolpath generator or a gouge check correct. The one mechanised G-code
semantics is Tekriwal and Sottile's RS274 work in Coq and Why3 (NFM 2025) [S] (literature.md
§10). All of the AI CAM shipping in 2026 produces editable operations that a person must review,
and none claims verified output: Fusion's Autodesk Assistant, CloudNC CAM Assist, Toolpath,
Mastercam Copilot, ESPRIT ProPlanAI and NX's AI suggestions (fusion-advanced.md §6).

**The recommendation.** Build CAM, but do not build "Fusion again". Build the CAM whose every
toolpath is *proven* gouge-free and collision-free against the exact part, or refused with the
reason. Let its users and agents program from the part's own design intent: forgecad's holes,
threads, fits and fillets are features, not shapes to recognise. Three differentiators fall out
of what forgecad already has:

1. **Certified safety.** Gouge, holder, rapid and fixture checks by forge-brake's interval branch
   and bound over the tool and the motion. A pass is proven; a fail names a real point; anything
   else is refused. Nobody else offers this.
2. **Design-intent machining.** `drill(op, f.bolts)` reads the hole's diameter, depth, thread
   and fit (`H7`) and plans spot, drill, peck, tap or ream. Fusion needs the paid extension to
   *recognise* holes (Fusion help `LP-READ-IRONHOLERECOGNITION` [R]). CAMWorks and NX sell
   recognition plus rules databases (commercial.md §2.4, §2.8).
3. **Agent-native and text-native.** A `machining` document is .fg text: diffable, versioned,
   with typed replies and fixes. Fusion's CAM API can already script setups, operations,
   templates, generation and posting from Python [R] (fusion-advanced.md §6.4), and Autodesk has
   a Fusion MCP beta [S]. Our edge over that is that the agent's output is *proven*, not reviewed.

**Build order:** a usable, proven **2.5D** CAM first, with the riskiest piece built before
anything else. That piece is exact line/arc region offsetting plus the certified checker at
interactive speed: it is the go/no-go. Then adaptive clearing, 3-axis surfacing, 3+2, turning,
and last simultaneous 5-axis. About 35–55 session-weeks to a proven 2.5D + adaptive + 3-axis
CAM (§3.6).

---

## 1. Prior art, ranked against Fusion

### 1.1 Commercial

Ranked by what each does best. "vs Fusion" is where it beats or trails Fusion. Detail and
sources: commercial.md §2.

| # | product | best at | vs Fusion | constant-engagement roughing |
|---|---|---|---|---|
| 1 | **Autodesk Fusion** | integrated cloud CAD+CAM at low cost; free, open JavaScript posts (595-entry library index [R], libraries-machines-posts-feeds.md §2.2); machine sim with Machine Builder in base | the reference | 2D/3D Adaptive Roughing |
| 2 | **Mastercam** (Sandvik) | installed base, reseller and post network, breadth (mill, lathe, router, wire, Swiss) | wider machine coverage; posts usually paid; 2026.R2 GPU verification, auto-tilt with bull tools [R] | Dynamic Motion |
| 3 | **hyperMILL** (OPEN MIND) | high-end 5-axis, blades and impellers, barrel cutters with tangent-plane finishing, automatic indexing, tool-length prediction [R] | stronger 5-axis and automation; perpetual, quote-only | MAXX HPC |
| 4 | **Siemens NX CAM** | enterprise and aerospace, feature-based machining, G-code-driven sim, mill-turn sync, closed-loop probing (2606) [R] | far stronger at the high end; about $7k–12k per seat-year [S] | Adaptive Milling |
| 5 | **ESPRIT EDGE** (Hexagon) | multi-channel mill-turn and Swiss; machine-builder-validated posts and twins | the mill-turn reference | ProfitMilling |
| 6 | **PowerMill** (Autodesk) | mold and die, huge models, rich tool-axis modes (lines, fixed direction, blisk, lead-then-lean) [R] | Autodesk's own high end; sold bundled with Fusion | Vortex |
| 7 | **SolidCAM** | in-SOLIDWORKS CAM; iMachining with its Technology Wizard | strongest embedded roughing | iMachining |
| 8 | **CAMWorks / SOLIDWORKS CAM** | automatic feature recognition plus TechDB shop rules | better rules-based automation | VoluMill |
| 9 | **Onshape CAM Studio** (PTC) | cloud-native, 2.5D/3-axis only so far, kinematic sim [R] | narrower; proves a small first release is accepted | Adaptive |
| — | WorkNC, Cimatron, GibbsCAM, BobCAD, TopSolid | mold automation, mold+electrodes, multi-task, low-cost, top satisfaction | niche leaders | Waveform, … |
| — | Vericut, NCSIMUL | G-code machine verification, feed optimisation | the verification standard; Fusion has none of its own | — |
| — | CloudNC CAM Assist, Toolpath | AI program proposals inside Fusion, Mastercam, NX | add-ins; human review required | — |
| — | ModuleWorks | the OEM toolpath and simulation SDK under many vendors (claims more than 60% of leading vendors for 5-axis) [S] | an Autodesk blog title ties it to the Fusion extension [S] | — |

What the industry converged on: every vendor ships constant-engagement roughing under its own
name, and the 3D finishing list is the textbook one (commercial.md §5). That core is table
stakes. The value, and the hard engineering, is in 5-axis, automation and verification, which is
what Fusion puts in its extension.

### 1.2 Open source

The licence column is for porting into MPL-2.0. Detail: opensource.md §1, §12.

| project | what it does best | maturity | licence → use |
|---|---|---|---|
| **FreeCAD CAM** (ex-Path) | the only full open 2.5D/3D CAM on a B-rep (OCCT); posts in Python; toolbits as JSON | the most complete open CAM; mixed robustness | LGPL-2.1+ → read, reimplement |
| FreeCAD `Adaptive.cpp` (Kresimir Tusek, 2018) | engagement-limited adaptive clearing: a cleared-area state plus a target removal per step | good; the best open adaptive | LGPL-2.1+ → read, reimplement |
| **OpenCAMLib** | drop-cutter, push-cutter, waterline for cylinder, ball, bull and cone cutters | the open 3-axis reference | LGPL-2.1 → read, reimplement |
| OpenVoronoi | segment Voronoi, medial axis, offsets | research quality | LGPL-2.1 → read |
| libarea core and kurve (Heeks) | pocket offsets on arcs and lines | dated | BSD-3 → may port, with attribution |
| **Clipper2** | robust polygon booleans and offsets on an integer grid | excellent | BSL-1.0 → may port; integer method is a model for bit-identity |
| cavalier_contours | line+arc polyline offsets, in Rust | good | MIT/Apache-2.0 → may port |
| **Kiri:Moto** | browser CAM (up to 4-axis) and slicer | active | MIT → may port |
| PyCAM, Fabex/BlenderCAM, CAMotics, GRBL, bCNC, dxf2gcode, jscut, libactp | 3-axis from meshes; Blender add-on; G-code cutting sim; the GRBL dialect | various | GPL → ideas only |
| LinuxCNC | RS274/NGC interpreter, kinematics modules; the reference open dialect | mature | GPL/LGPL → ideas, dialect reference |
| truck, Fornjot (Rust kernels) | — | — | no CAM at all |

No open project combines an exact B-rep with CAM in Rust, and none is exact: they offset in floats
or on integer grids and check on meshes. forgecad's kernel code must be our own anyway
(no third-party crates); the algorithms are textbook.

### 1.3 Research literature

Detail and 100+ references: literature.md. The eight that matter most:

1. **Held 1991**, *On the Computational Geometry of Pocket Machining*, LNCS 500,
   doi:10.1007/3-540-54103-9, and Held 1998, CAD 30(4), doi:10.1016/S0010-4485(97)00071-7. Voronoi-based
   offsetting of line/arc boundaries: the basis for exact 2.5D.
2. **Kramer, Proctor, Messina 2000**, NIST RS274/NGC Interpreter v3, NISTIR 6556,
   doi:10.6028/NIST.IR.6556 [R]. The fullest public meaning of G-code. Its order-of-execution
   table is the model for our post's modal engine.
3. **Choi & Jerard 1998**, *Sculptured Surface Machining*, doi:10.1007/978-1-4615-5283-3. The
   CL surface, drop-cutter, Z-map, scallop height.
4. **Elber, Cohen, Drake 2005**, MATHSM, CAD 37(2), doi:10.1016/j.cad.2004.05.008, and
   Abrahamsen 2014, arXiv:1412.5034 [R]. Smooth, bounded-engagement clearing from the medial axis.
5. **Abdel-Malek & Yeh 1997**, doi:10.1016/S0010-4485(96)00097-8, and **Blackmore, Leu, Wang
   1997**, doi:10.1016/S0010-4485(96)00101-7. Swept-volume theory.
6. **Ilushin, Elber, Halperin, Wein, Kim 2005**, CAD 37(9), doi:10.1016/j.cad.2004.09.018. 5-axis
   collision checking, but only at sampled positions.
7. **Sullivan, Erdim, Perry, Frisken 2012**, CAD 44(6), doi:10.1016/j.cad.2012.02.002 [R]. Stock
   simulation by distance fields, claimed better than 1 µm.
8. **Tekriwal & Sottile 2025** (NFM), Coq/Why3 semantics of RS274 [S]. The only mechanised G-code
   semantics found.

What the literature says for an exact implementation (literature.md, "Implications"):
- **Exact:** 2.5D line/arc offsets, if each pass is offset from the original boundary (degree 2
  over the input); rest regions (an opening by a disc); engagement angles; 3-axis swept volumes of
  flat, ball, bull and cone tools along lines and planar arcs (planes, cylinders, cones, spheres,
  tori, all inside forgecad's curved-boolean scope); drop-cutter in closed form for cylinder,
  ball and cone tools.
- **Certified intervals:** bull tools against edges (a quartic), freeform faces, helical moves,
  anything 5-axis.
- **Approximate by nature, so stated as bounds:** non-developable flank milling, cutting physics,
  decimal output, and the machine itself.

---

## 2. The tool list for best in class

**M** = must-have for a credible first release in that area; **D** = differentiator, where exact
geometry, proofs, design intent or agents beat the incumbents; **L** = later. "Fusion" says how
Fusion does it; "Ext" means it needs the paid Manufacturing Extension. Every Fusion row is from
its help pages, read [R]. The IDs and parameters are in the notes named in each section heading.

### 2.1 Libraries, the foundation (libraries-tools.md, libraries-machines-posts-feeds.md)

| item | Fusion | mark | forgecad |
|---|---|---|---|
| **Tool library: cutter types** | About 40 type strings: 15 milling (flat, ball, bull, chamfer, face, slot, dovetail, lollipop, tapered, radius, four circle-segment forms, form mill, thread mill), 9 hole-making (drill, spot, centre, counterbore, countersink, reamer, boring bar, tap right/left), 4 turning, probe, holder, shaft. No T-slot type: users model it as a slot mill with a neck. Schema at https://cam.autodesk.com/tools/tools/Tooling%20Schema/ToolLibrary.schema.json [R] | M | `[[tool]]` with ISO 13399 symbol names (`dc`, `re`, `lcf`, `lu`, `dn`, `dcon`, `oal`, `nof`, `sig`, `tp`); every type is a generator of an exact line/arc revolved profile; `type = "profile"` for form and barrel tools |
| geometry parameters | diameter, corner radius, flute, shoulder, body and overall lengths, taper, tip diameter, flutes, shaft segments, hand, coolant-through, material, coating | M | required fields per type, refused with a typed error if missing, never defaulted silently |
| **turning inserts and holders** | ISO insert codes (shape C, D, V, W, T, S, R …; size, thickness, nose radius, relief, tolerance), holder style, hand | L (phase 5) | ISO 1832 / ISO 5608 decoding |
| **tool assembly** | holder + shaft + stick-out, T number, H and D offsets, turret, live, manual change, break control; presets of feeds and speeds per tool | M | `[[assembly]]` in the shop file: `t`, `h`, `d`, `stickout`, `holder`, `extensions`; a gauge-length consistency lint (D) |
| **holder library** | holders as stacked cone frusta below a gauge line, so arcs are faceted; vendor holder libraries (Haimer, Schunk, Haas) | M | `[[holder]]` exact line/arc profile `[[z, d]]` plus arcs, used by the certified checker (D); generic conservative BT/CAT/HSK/ER built in, restated from the standards (ISO 7388, ISO 12164, ISO 15488); real holders by import |
| **formats and catalogues** | its own JSON (`.json` / `.tools` zip), 52 vendor libraries from 32 makers (Harvey, Helical, Sandvik, Kennametal, Iscar, OSG, Garr …) [R]; GTC import (properties only, not the STEP model) | M | **import Fusion JSON** (phase 1); export a LinuxCNC tool table (phase 1); FreeCAD `.fctb`/`.fctl` both ways (phase 1–2); GTC/ISO 13399 `.p21` with STEP-to-exact-profile (D, phase 2–3). Mastercam `.tooldb`, NX and SolidCAM: closed, not planned. Traps: Fusion `LB` is length below holder, not ISO LB; Fusion `TA` is a half angle in the file, contrary to the schema text (libraries-tools.md §6) |
| **machine library** | kinematics tree (X…C axes, table or head, range, TCP, preference, reset, resolution), spindle as power and peak torque only (no curve), Machine Builder; `.mch` format undocumented [R] | M (3-axis, 4-axis, trunnion) | `[[machine]]` TOML: kind, joint chain with limits and home, spindle rpm and power curve (D), accelerations (D), coolant codes, dialect link, optional `.fg` assembly bodies for sim. Machine and control separate, as Vericut does (`.mch` / `.ctl`) |
| **material and feeds/speeds library** | presets live on each tool; the material does not drive feeds; "feed optimisation" only slows corners by geometry [R] | M | cutting data in forge-material as `cam.*` keys (ISO group P M K N S H, `kc1`, `mc`, `vc`, `fz` by diameter), each value sourced; the feeds engine computes rpm, feed, chip thinning and ball-nose effective diameter (M); power, torque and deflection checks from the *true* engagement (D). Fusion lacks all three checks; HSMAdvisor, FSWizard and G-Wizard have them |
| **post-processor library** | JavaScript `.cps` posts on Autodesk's engine; 595 posts for 207 vendors, free and editable, but "All rights reserved", so facts only [R] | M | a fixed Rust engine reading **data dialects** (`[[dialect]]`), with no embedded script language; numbers printed from integer-quantised values, so native and wasm are identical; a round-trip reader for the proof. First dialects: Fanuc-generic, Haas, LinuxCNC, GRBL (M); Siemens, Heidenhain ISO, Mazak, Okuma (L) |
| **fixture library** | a "Fixture" selection in the setup; no fixture library [R] | M (generic vise, plate, chuck) | fixtures are .fg parts plus a `[[fixture]]` record (jaw range, datum); associative soft jaws cut from the part (D); vendor fixtures (Kurt, Orange Vise, Mitee-Bite, Lang, 5th Axis) restated as profiles (L) |
| **stock library** | stock in the setup (nine modes); no shop stock list | M | stock modes in the document; a `[[stock]]` list of bar and plate sizes the planner may choose from (D) |

### 2.2 Workflow and UX (fusion-workflow-2d-3d.md §1–2)

| item | Fusion | mark | forgecad |
|---|---|---|---|
| **setup** | operation type, machine, WCS orientation (from model, Z+X, Z+Y, X+Y, a coordinate system) and origin (model origin, point, stock-box or model-box point), model, fixtures; accessibility analysis | M | `setup(f, machine:, wcs: (origin: f.plate.end, …), stock:, fixture:)`: the WCS is a named anchor of the part, so it survives edits (D) |
| **stock** | fixed box, relative box, fixed and relative cylinder and tube, from solid, from preceding setup, plus offsets | M | `box`, `cylinder`, `tube`, `from: solid`, `from: setup` |
| **probe the WCS** | Probe WCS: the faces picked choose the cycle (wall, web, pocket, boss, corner, bore, angle) | M | `probe: wcs_probe(...)` emitted as the control's probing macros |
| **operation list** | browser with folders, patterns (five types), suppress, protect, optional, dirty/regenerate state | M | operations are features: named, ordered, `build` status per op; `--at OP` is the in-process stock (D) |
| **templates** | saved templates, a template library, expression-based defaults | M | templates are .fg text, inserted by `AddFeature`; no separate format (D) |
| **toolpath editing** | trim, delete passes, replace tool, move entry, edit leads and links (Ext) | L | edits as ops on the document; Fusion's Compare and Edit is `diff` (D) |
| **heights** | clearance, retract, feed, top, bottom: each a reference plus an offset, with automatic highest/lowest | M | heights from model anchors (D) |
| **linking** | high-feed mode, retraction policy, stay-down distance, lead-in/out arcs, ramps (helix, profile, zigzag, plunge, predrill), transitions | M | the same parameters; a failed ramp or lead comes back as a typed failure with a fix (D) |
| **passes** | tolerance, smoothing, stock to leave, multiple depths, finishing passes, climb/conventional | M | tolerance split inner/outer and part of the proof statement (§3.4) |
| **NC programs** | combine setups, reorder to cut tool changes, one post per program | M | `program([op1, op2], post:, number:)` |
| **setup sheets** | HTML (and Excel) from setups and NC programs | M | from the typed reply; HTML first |
| machining time | estimate per op | M | with the machine's accelerations (D) |

### 2.3 2D (fusion-workflow-2d-3d.md §3)

| item | Fusion (all base) | mark |
|---|---|---|
| Face | facing passes over the stock top | M |
| 2D Adaptive Roughing | constant optimal load, minimum cutting radius, slot clearing, flat-area detection | M (phase 2) |
| 2D Offset Roughing (was 2D Pocket) | offset pocketing | M |
| 2D Contour | compensation in computer, in control, wear, inverse wear; chamfer mode; tabs | M |
| Slot | along a slot centreline | L |
| Trace | follow a curve | M |
| Thread (thread milling) | helical thread milling | M; differentiator from *modelled* threads |
| Bore, Circular | helical boring, circular pockets | M (Bore), L (Circular) |
| Engrave | V-bit engraving | L (forge-font glyphs exist) |
| 2D Chamfer | deburr chamfer with a chamfer mill | M |
| Drill and cycles | drill (G81), dwell (G82), chip-break (G73), deep (G83), tapping (G84/G74), reaming and boring (G85, G86, G76, back boring), circular pocket and thread-mill as cycles, probing; hole sorting | M (the G81/82/73/83/84/74/85 set); others L |
| hole planning from the feature history | Fusion needs Hole Recognition (Ext) | **D**: `plan: auto` from `hole(d, depth, thread, fit)` |

### 2.4 3D (fusion-workflow-2d-3d.md §4)

| item | Fusion | mark |
|---|---|---|
| 3D Adaptive Roughing | stock-aware constant-engagement z-levels; rest sources | M |
| 3D Offset Roughing (was Pocket Clearing) | offset z-levels | M |
| Flat | flat areas only | M |
| Parallel | raster | M |
| Scallop | constant scallop | M |
| Contour | waterline | M |
| Pencil | along concave creases | M (forgecad: the offset body's creases, research/fillets) |
| Rest machining | a Rest group on each op (previous ops, chosen ops, tool, file, bodies, setup stock) plus Corner (Ext); there is no "Rest Finishing" strategy in the current help | M |
| Steep and Shallow | split by slope (Ext) | L |
| Ramp, Horizontal, Spiral, Radial, Morphed Spiral, Project, Blend, Morph | classic finishers | L |
| Flow, Geodesic | along surface parameters or geodesics (3-axis free, multi-axis Ext) | L; D on exact B-rep (forgecad has the exact surface parameters) |
| Deburr | automatic edge break (Ext) | D on exact edges: forgecad knows every convex edge by name |

### 2.5 Multi-axis (fusion-advanced.md §1)

| item | Fusion | mark |
|---|---|---|
| 3+2 positional | per-op tool orientation, tilted-plane output (base) | M (phase 4) |
| automatic indexing (fewest orientations) | not in Fusion; hyperMILL has it [R] | D |
| 4th axis wrap and indexed rotary | base | M |
| simultaneous rotary rough and finish | Ext | L |
| Swarf, Multi-Axis Contour, tiltable 3D finishing | Ext | M (phase 6) |
| flank and barrel tangent-plane finishing with a *proved* deviation | Fusion: none; hyperMILL MAXX [R] | D |
| tool-axis control | Vertical, Lead/Lean, from/to point or curve; preferred-tilt cone; smoothing; min/max tilt (trim or hold) | M |
| collision avoidance by tilting, plus trim | Ext | M |
| certified holder and shank clearance; minimum stick-out | not offered by anyone read | D |
| rewinds and retract-and-reconfigure | Ext | M |
| Geodesic, multi-axis roughing, multi-axis deburr | Ext | L |

### 2.6 Turning and mill-turn (fusion-advanced.md §2)

| item | Fusion | mark |
|---|---|---|
| face, profile rough and finish, adaptive turning, groove, single groove, thread, chamfer, part-off, trace | 12 strategies, base | M (phase 5; all 2D lines and arcs, so exact) |
| sub-spindle grab and return, bar pull, live tooling, Y-axis | base, from the machine definition | L |
| multi-channel synchronisation, Swiss | none in Fusion; NX and ESPRIT have it | L |

### 2.7 Probing and inspection (fusion-advanced.md §3)

| item | Fusion | mark |
|---|---|---|
| Probe WCS | base; posts call the control's Renishaw-style macros | M |
| Probe Geometry, Inspect Surface, part alignment, live connection, reports | Ext | L |
| inspection deviation against the exact B-rep | Fusion measures against its model; forgecad's distances are certified | D |

### 2.8 Additive and hybrid (fusion-advanced.md §4)

| item | Fusion | mark |
|---|---|---|
| FFF, binder jet (base), MPBF, DED (Ext) | full additive workspace | L |
| DED + finish-milling hybrid with true-solid stock | Ext | L (a D later: exact deposited stock) |

### 2.9 Simulation and verification (fusion-advanced.md §5)

| item | Fusion | mark | forgecad |
|---|---|---|---|
| stock simulation | toolpath-driven; comparison colours (blue under-cut, green in tolerance, red gouge); collisions with holder and fixture | M | a dexel preview for display; the verdict from the exact checker |
| **certified gouge and excess check** | none; tolerance is defined against a triangulation [R] | **D** | G1–G4 of §3.4, by B&B |
| machine simulation with kinematics | base, with Machine Builder; reports overtravel, collisions, auto-inserted retracts | L (phase 6) | the machine as a forge-asm assembly; the same checker |
| G-code-driven verification | not in Fusion (CAMplete, being folded in) [S]; Vericut, NCSIMUL | L | we prove the post instead (G6), so our own code needs no separate verification; integrate Vericut for other people's code |

### 2.10 Feature recognition and automation (fusion-advanced.md §6)

| item | Fusion | mark | forgecad |
|---|---|---|---|
| hole recognition with templates | Ext | M | **read from the history** (D); recognition only for imported STEP (L) |
| scripting API | Python/C++ API: setups, ops by strategy name, parameters, templates, generate, post, setup sheets [R] | M | the existing CLI/MCP typed ops; already the model |
| exact feature recognition plus verified auto-programming | none | D | |
| AI program proposals | Autodesk Assistant (conversational), "Instant Prismatic Machining" (exploratory) [S]; CAM Assist, Toolpath, Copilot, ProPlanAI | D | the agent writes the `machining` document; every path it emits passes the certified checker or is refused with a fix |
| ML process planning | ProPlanAI, NX generative suggestions | L | |

---

## 3. How to add it to forgecad

The full design is [design.md](design.md). The essentials:

### 3.1 Architecture

| crate | where | what |
|---|---|---|
| `forge-cam-geom` | kernel/ | Tools as exact revolved profiles; exact 2D region offsets; swept regions and volumes; drop-cutter and push-cutter (closed form on planes and quadrics, certified B&B elsewhere); engagement. This small crate is the one that goes through Aeneas. |
| `forge-cam` | kernel/ | Setups, stock and in-process stock; operations as pure functions; the linker; the certified checks; cycle time |
| `forge-tooling` | kernel/ | tools, holders, machines, fixtures, stock sizes as our TOML (built in, then user, then project) |
| `forge-post` | kernel/ | the dialect engine and tables, plus a reader of our own output, for the round-trip proof |
| CAM in `forge-lang` | interface/ | the `machining` document, `query F cam`, schema `forgecad.cam/1`, edit ops and fixes |
| `forge-cam-import` | interface/ | importers for Fusion JSON, GTC/ISO 13399, FreeCAD, LinuxCNC (third-party crates allowed here) |

The precedent is `kernel/forge-brake` (bendsim branch): an application crate in kernel/ because
its answers are certified geometry; it depends on forge-math, forge-ops and forge-sheet only.

### 3.2 What it reuses

- **Exact offsets.** forge-dwg's line/arc/circle offset (kernel/forge-dwg/src/draft/offset.rs);
  forge-ops' exact face offsets (shell.rs: a plane moves, a cylinder, cone, sphere or torus
  changes radius). For a ball tool, the CL surface *is* the r-offset of the part.
- **The wavefront topology events** of shell (kernel/forge-ops/src/wavefront.rs) are the topology
  changes of a 2D offset front.
- **The boolean** gives the exact stock reference: stock − ⋃ swept volumes. 2.5D sweeps are
  extrusions of line/arc regions, inside the domain the boolean already handles.
- **Fillet offsets.** The ball closing and opening of research/fillets/general-method.md are
  exactly rest machining: what a ball tool cannot reach is the target minus its opening, and
  the offset body's creases are where the pencil passes go.
- **Certified intersection tracing** (the Krawczyk step, verify/krawczyk on the trace branch)
  traces waterlines, flow lines and contact curves.
- **Tessellation** (forge-mesh, `Quality::chord`) is for display and the dexel preview only,
  never for a verdict.
- **forge-brake's patterns:**
  - the `bb::Problem` trait: interval branch and bound over a piece's parameters *and the motion
    s*, with verdicts pass, fail or undecided, undecided refused. It becomes the gouge, holder,
    rapid and fixture checker unchanged in kind;
  - tooling as data with a built-in library "in our own numbers";
  - the planner as search while the checker is the proof;
  - the reply schema `forgecad.bendsim/1` with checks, `blocking[]`, `fixes[]` and
    `suggestions[]` (`NeedTool`).
- **The vendor profiles** (kernel/forge-vendor, sheet-vendors branch) are the template for
  tool-maker catalogues and shop profiles: restated with `source`, `taken`, `note` and "verify
  before use". forge-material's dotted keys (`thermo.*`) are the template for `cam.*` cutting
  data.
- **The design history.** Holes with d, depth, thread and fit; chamfers; fillet radii (which bound
  a finishing tool's corner radius); persistent names, so operations survive edits.

### 3.3 The .fg surface, the reply and the tools

A `machining` document, like `drawing`:

```
machining
f    = part("flange.fg", material: "6061-T6")
m    = machine("haas-vf2")
lib  = tools("shop.tools.toml")
op1  = setup(f, machine: m, wcs: (origin: f.plate.end, z: +z),
             stock: box(size: (84, 84, 22), at: (0, 0, -8)), fixture: fixture("kurt-dx6"))
face = facing(op1, tool: lib.T1, down: 1)
rough = adaptive(op1, tool: lib.T2, load: 1.2, down: 12, leave: 0.3)
bore = drill(op1, f.bore_h, plan: auto)                # spot, drill, bore from the hole's fit
prof = contour(op1, edges(of: f.plate, crease: "convex"), tool: lib.T3, comp: "wear")
prog = program([op1], post: "haas-ngc", number: 1001, sheet: true)
```

The real file keeps its numbers in params (design.md §3).

- **The reply** `forgecad.cam/1`: setups, ops with tool, feeds (rpm, vc, fz, chip thickness after
  thinning), path stats, `checks[]` (rule, verdict, certified, value, limit, where, which tool
  part against what), `leftover` with reasons, `blocking[]`, `fixes[]`, `suggestions[]`,
  `programs[]` (design.md §4).
- **The tools** are the existing ones with a new document kind: `query F cam [--op]`,
  `render F --at OP`, `export F prog`, `export F setup-sheet`, over CLI and MCP.

### 3.4 The owner's rules

- **Exact, never a wrong result.** A path ships only when its checks pass with
  `certified: true`; otherwise it is refused with the failing check named. Dexel and mesh views
  are for display only.
- **The Lean spec (L1).** T is the cutter, N the non-cutting parts (shank, holder, extensions), F
  the fixture and machine, g : [0,1] → SE(3) the motion, Sw(X, g) = ⋃ g(s)·X, P the target, δ the
  stock to leave, ε the tolerance, S(s) the in-process stock.
  - **G1, no gouge:** Sw(T, g) ∩ int(P ⊕ B_δ) = ∅, with the inner tolerance stated in the
    formula, never as a sampling gap.
  - **G2, non-cutting parts:** g(s)·N misses int S(s) and F(s) for every s.
  - **G3, rapids and links:** g(s)·T misses int S(s).
  - **G4, the result:** P ⊕ B_δ ⊆ S_end ⊆ (P ⊕ B_δ) ⊕ B_ε ∪ L, where L is the named leftover
    (tool radius, reach, undercut). This is "stock − swept = target within tolerance", made
    honest: two one-sided inclusions plus the listed places.
  - **G5, the machine:** joint limits are held, and there is no flip through a singularity.
  - **G6, the post:** the Lean model of our G-code subset (after NIST RS274/NGC) maps post(g) to
    g′, the path rounded to the dialect's digits, and G1–G5 are checked on g′.

  Cutting physics and the machine's fidelity are outside the spec, and every reply says so.
- **The six levels.**
  - **L2:** exact algebraic sweeps and removal for 2.5D (ℚ(√d), forge-exact's Alg/Quad);
    closed-form oracles for 3D.
  - **L3:** the B&B is sound (a pass is a proven bound, a fail a real witness). It needs
    forge-brake's `sd_err` budget proven; that is an open item shared with bendsim, to be proven
    once for both.
  - **L4:** forge-cam-geom and forge-post through Aeneas. The planner needs only to be safe,
    because everything it makes passes the checker.
  - **L5:** the predicates and interval enclosures.
  - **L6:** a pocket fuzz corpus against the exact removal, plus the bench parts.
- **No slower.** CAM is a new document kind and never runs in a part's build, so no existing
  gate line moves. Its own lines join the gate (the checker's cells per move on a real adaptive
  path first). Ops regenerate alone, keyed by content hash.
- **Native = browser.** No threads in wasm; parallel ops merge in op order; `dmath` trig; no FMA;
  G-code printed from integer-quantised numbers, pinned by a native-vs-wasm text test.
- **No third-party crates in kernel/.** All kernel code is our own; importers live in
  interface/. No open-source CAM code is ported into kernel/.

### 3.5 Libraries: where and how

All of them are our TOML, using forge-material's reader lifted into a shared module. They are
searched project, then user (`~/.config/forgecad/cam/`), then built in. Every record carries a
source.

| library | file | built in | importers |
|---|---|---|---|
| tools | `*.tools.toml` `[[tool]]`, `[[tool.shaft]]`, `[[tool.preset]]` | generic flat, ball and bull 1–20 mm; drills; M and UNC/UNF taps; spot, chamfer, thread mill | Fusion JSON (M), FreeCAD (M/D), LinuxCNC export (M), GTC/ISO 13399 (D) |
| holders | `*.holders.toml` `[[holder]]` exact profile | generic BT/CAT/HSK + ER, labelled conservative | Fusion holders, GTC STEP to profile |
| assemblies | `[[assembly]]` in the shop file | — | Fusion libraries |
| machines | `cam/machines/*.toml` `[[machine]]` | generic 3-axis, 4-axis, trunnion, 2-axis lathe | Fusion `.mch` once its format is checked; STEP 105 (L) |
| posts | `kernel/forge-post/data/dialects/*.toml` `[[dialect]]` | Fanuc-generic, Haas, LinuxCNC, GRBL | none: written for us |
| cutting data | forge-material `cam.*` keys | per ISO group, from public tool-maker data, restated | — |
| fixtures | `.fg` parts + `[[fixture]]` | generic vise, plate, chuck | STEP |
| stock | in the document; `[[stock]]` shop sizes | common bar and plate | — |

Schemas with every field: libraries-tools.md §5.2–5.4, libraries-machines-posts-feeds.md
§5.2–5.5.

### 3.6 The phased plan

Estimates are in session-weeks (one focused agent session's week, with review), ±50%.

| phase | what | effort |
|---|---|---|
| **0. The risk, first** | Exact 2D line/arc region offsetting with islands, through topology change. Swept regions and their exact union. The L1 spec for 2.5D. The certified checker for flat, ball and bull tools along lines and arcs, with `sd_err` proven. **Go/no-go:** checker speed on bench pockets. | 6–10 |
| **1. Usable 2.5D** | The `machining` document; setups, WCS, stock. Facing, contour with compensation, pocket, slot, trace, engrave, chamfer, bore, drilling cycles, and drill plans from hole features. Leads, ramps, heights. Posts: Fanuc, Haas, LinuxCNC, GRBL. Setup sheets. Tool and holder TOML and the Fusion JSON importer. Dexel preview. | 10–14 |
| **2. Adaptive** | Constant-engagement roughing from exact engagement; 2D rest; feeds with chip thinning; cycle time | 6–10 |
| **3. 3-axis** | Drop-cutter and push-cutter (closed form, then certified); 3D adaptive, parallel, waterline, scallop, pencil, flat, rest from the ball opening; 3D gouge checks. After the curved boolean's L2 matures (ROADMAP item 4). | 12–20 |
| **4. 3+2 and 4-axis** | Orientations, 4th axis, trunnion kinematics, axis limits, holder/fixture/machine collisions, tilted-plane post codes | 6–10 |
| **5. Turning** | Lathe setups and the core turning ops; inserts and holders | 6–8 |
| **6. Simultaneous 5-axis, machine sim** | Swarf, multi-axis contour and flow, tool-axis control, tilting for collisions, full machine simulation | 20–30 |
| later | mill-turn sync, inspection probing, additive and hybrid, Heidenhain conversational, Vericut link | — |
| alongside every phase | the agent layer: typed replies, fixes, `NeedTool`, `plan: auto` | — |

About 35–55 session-weeks to phase 3. If certifying a 10⁵-move adaptive op takes more than
about a second natively, the certificate runs in the background (`--certify`), and the op shows
`undecided: pending`, never `pass`, until it lands.

---

## 4. Licences

**Licences** (opensource.md §12). forgecad is MPL-2.0, which is file-level copyleft.
- **Permissive, may port with attribution:** libarea core (BSD-3), HeeksCAD (BSD-3), Clipper2
  (BSL-1.0), cavalier_contours (MIT/Apache-2.0), Kiri:Moto (MIT), the `gcode` crate. Ported code
  could go only in interface/. In kernel/ we write our own.
- **LGPL, read and reimplement:** FreeCAD CAM including Adaptive, OpenCAMLib, OpenVoronoi.
  Static linking into wasm brings relinking duties, so do not link them.
- **GPL or AGPL, ideas only:** PyCAM, Fabex, CAMotics, GRBL, bCNC, LinuxCNC's GPL parts, jscut,
  libactp.
- **Fusion's posts** are "All rights reserved": use the facts about controls, never the code.
  Fusion's tool JSON schema is published, and reading the format to import it is interoperability.
- ISO 13399 and ISO 6983 are paywalled standards. Restate the facts only.

---

## 5. What I would do next

1. Owner decision: go on phase 0, with its go/no-go on checker speed.
2. Before phase 0, prove forge-brake's `sd_err` budget. It is shared with bendsim, and CAM's L3
   rests on it.
3. Open a real Fusion tool library export and a real Sandvik GTC package to settle the open
   format questions (the `TA` half angle; the GTC layout) before writing importers.

Open questions the research could not settle are listed at the end of each note: Fusion's stock
representation in simulation, the `.mch` format, whether stock transfer between setups works for
milling or turning (the help contradicts itself), and the exact Sandvik mean-chip-thickness
form.
