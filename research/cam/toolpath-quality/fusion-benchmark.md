# Fusion as the toolpath-quality benchmark: what it offers, where users say it falls short, and a testable "Fusion-level" bar

Research only, written 2026-11-24 on branch `research/toolpath-quality`. Nothing in forgeCAD was
changed. Goal (David): CAM toolpaths "Fusion level or better: adaptive toolpaths, feeds and speeds,
surface finish". This note covers toolpath **quality**. It extends, and does not repeat:

- `research/cam/fusion-workflow-2d-3d.md` (every 2D/3D strategy, the shared tabs, the help IDs);
- `research/cam/fusion-advanced.md` (multi-axis, the accuracy model §5.4, simulation);
- `research/cam/commercial.md` (vendors, the constant-engagement name table §5);
- `research/cam/libraries-machines-posts-feeds.md` §3 (presets, calculators, feed optimisation);
- `research/cam/literature.md` §2 (engagement geometry), and `design.md` §5 (the proof levels).

## How to read this

- **[R]** I fetched and read the page in this session (2026-11-24). **[S]** I saw only a search
  snippet or summary. **[K]** background knowledge, not re-read. **[prev R]** read on 2026-11-23 by
  the sibling notes and cited from them, not re-read today. Anything not read is **UNKNOWN**.
- Fusion help pages are `https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/<ID>.htm`; keys
  such as `[3DA]` resolve in the source table at the end.
- **User reports are user reports.** A forum post saying "X is slow" is one person's experience on
  one part and one version, not a fact about Fusion.
- Fetch limits met today: `forums.autodesk.com`, `autodesk.com/products/fusion-360/blog/*`,
  `practicalmachinist.com` and Reddit's search all answered 403 to the fetcher. I did not work
  around them. Everything from those sites is [S] (snippet) only.
- Everything is restated; nothing is quoted at length.

### Findings that matter first

1. **Fusion's help documents almost no numeric defaults.** Across the 20+ reference pages re-read
   today, the only stated defaults are: smoothing **off** [SMOOTH R]; finishing stock to leave 0,
   roughing "a small amount" [2DA R]; 3D Adaptive direction **climb** [CLIMB R]; *Include setup
   model* on [3DA R]; *Contact Only* on [CONTACT R]; Steep and Shallow smoothing tolerance factor
   **0.5 × tolerance** [SS-SMOOTH R]; and a stepover default of "95% of diameter less corner radius"
   that appears word for word on the Parallel, Scallop and Pencil pages [PAR R] [SCAL R] [PENCIL R]
   and is almost certainly text copied from Face (no one runs a ball-nose parallel finish at 95%);
   treat it as a doc error, not a default. Other defaults are expressions of tool geometry that live
   in the product, not the help [prev R: DEFAULTS]. **So the checklist below tests behaviour, not
   default values.**
2. **"Optimal load" is a cap on radial engagement written as a distance.** It is the maximum
   sideways engagement the path should hold; the actual stepover varies and is never more than it
   [3DA R] [3DA-STEPS R]. Help suggests 10–40% of diameter, depending on material, tool and
   rigidity [3DA-STEPS R] [2DA-STEPS R]; the 3D Adaptive reference gives per-material guidance
   (steel ≤20% D sideways at up to flute length deep; hardened steel ≤5%; aluminium 30%, up to 50%;
   halve it for many-flute cutters) [3DA R]. The reference claims the strategy "guarantees a
   maximum tool load at all stages" [3DA R]. That guarantee is the core thing to test.
3. **Fusion has no engagement-driven or chip-thinning feed control.** Its *Feed Optimization*
   only slows the feed at sharp geometric corners: max directional change, reduced feed radius,
   reduced feed distance, reduced feedrate, only inner corners [FEEDOPT R]. Face's *Use chip
   thinning* is a **roll-on entry**, not a feed correction [FACE R]. Tools carry named cutting-data
   presets, which can be expressions of tool geometry [PRESET R] [EXPR R] and can now be associated
   with **stock materials** that a setup picks from the model's physical material [STOCKMAT R]
   [STOCKMAT-SEL R]. That last point corrects the sibling note's "material does not drive feeds"
   (libraries-machines-posts-feeds.md §3.3): material now *filters* presets, but nothing *computes*
   a feed from material, engagement or machine power. I found no Fusion feeds-and-speeds calculator
   or AI feed feature in the help TOC or in search [S, absence]. Mastercam (RCTF), iMachining
   (per-segment Technology Wizard) and hyperMILL MAXX (feed adapted along the path) all have one
   (§3).
4. **Fusion's cycle-time estimate is feed time + rapid time + tool-change time** from programmed
   feeds, path lengths and one machine rapid rate, with a feed-scale what-if [MTIME R]
   [MTIME-HOW R]. Nothing in the help mentions acceleration or jerk. Users report it ignores
   acceleration and can be off by large factors on 3D paths (§2.6) [S]. Beating this is cheap and
   is a clear "better than Fusion" item.
5. **Fusion cannot emit NURBS/spline motion.** The post engine's motion entry points are
   `onRapid`, `onLinear`, `onCircular` and their 5-axis forms; there is no spline entry, and the
   PostProcessor class reference has no occurrence of "spline" or "nurbs" [POSTAPI R]. Smoothing
   means arc fitting (G2/G3) and, in Steep and Shallow only, "evenly spaced points" [SMOOTH R]
   [SS-SMOOTH R]. Autodesk's own route to spline-quality finish is the control's look-ahead
   smoothing (an Okuma Super-NURBS blog post) [S: OKUMA].
6. **The accuracy model is triangulated.** Machining tolerance = toolpath-generation tolerance +
   geometry-triangulation tolerance, and filtering/smoothing tolerance adds on top [3DA R]
   [LEG-SMOOTH R]. There is no documented certified bound against the exact surface
   (fusion-advanced.md §5.4). That is forgeCAD's natural opening.

---

## 1. What Fusion offers, area by area

### 1.1 Roughing

**2D and 3D Adaptive Roughing** (renamed from Adaptive Clearing) [2DA R] [3DA R]:

| Parameter | What it does | Default (documented?) | Source |
|---|---|---|---|
| Optimal load | max radial engagement held; stepover varies at or below it | not documented; help suggests 10–40% D. One forum user says the product default is 40% D and calls it too aggressive [S: OPTLOAD-FORUM] | [3DA R] [3DA-STEPS R] |
| Optimal load other way, other way feedrate | the conventional-direction width and feed when *Both ways* is on (2D, open profiles) | — | [2DA R] |
| Direction | climb, conventional, both ways; not always holdable | climb (3D) | [3DA R] [CLIMB R] |
| Minimum cutting radius | smallest path radius in a sharp inside corner; blends all inside corners, leaving rest material | — | [3DA R] [2DA R] |
| Use slot clearing, slot clearing width | cut a centre slot first, then spiral out; for pockets where the default spiral makes corner linking | off implied ("by default ... spiral") | [SLOTCLR R] |
| Machine cavities (3D) | ramp into enclosed pockets, or only cut from outside in | — | [3DA R] |
| Maximum roughing stepdown | max Z step; last pass may be smaller | — | [3DA R] |
| Use even stepdowns (2D) | split depth into equal steps ≤ max | — | [2DA R] [2DA-STEPS R] |
| Fine stepdown (3D) | intermediate steps *upward* between big steps to cut terracing | — | [3DA R] |
| Flat area detection, minimum stepdown, minimum axial engagement | add levels at flats so axial stock-to-leave is exact on them; optional floor on axial engagement so a flute stays in the cut; "may increase calculation time considerably" | off | [3DA R] [FLATDET R] |
| Machine shallow areas: min shallow stepdown, max shallow stepover | insert extra Z levels where the step would leave too wide a terrace | off | [SHALLOW R] [SHALLOW2 R] |
| Order by depth / order by area | all regions per level, or each region to full depth | — | [3DA R] |
| Stock to leave, radial and axial | negative allowed (electrodes); slope-interpolated between the two on non-vertical, non-flat faces | roughing: "small amount" | [3DA R] [2DA R] |
| Fillets, fillet radius (3D) | round sharp corners of the path | — | [3DA R] |
| Smoothing | arc fitting within the filter tolerance | off | [3DA R] [SMOOTH R] |
| Feed optimization | geometric corner slowdown (§1.2) | off | [FEEDOPT R] |
| Rest machining | from previous ops, file, bodies, setup stock; ignore stock less than; *Reduce air cutting* (3D; help's example 4:44 → 3:20 machining time, at extra calc time) | — | [3DA R] [AIRCUT R] |
| Tolerance | max chord deviation; help warns of data starving (old controls ~40 blocks/s, newer ~1,000) | — | [3DA R] |
| Linking: max stay-down distance, min stay-down clearance/distance, **stay-down level** (Least 0% to Most 100% in 10% steps; more = more calc time), lift height, no-engagement feedrate | — | lift height 0 | [3DA R] [2DA R] [LIFT R] |
| Leads: horizontal/vertical lead-in/out radius | — | — | [3DA R] |
| Ramp: type (2D: plunge outside stock, zig-zag, predrill, profile, plunge, smooth profile, helix; 3D page shows predrill, plunge, helix), ramping angle, ramp taper angle (conical helix), max ramp stepdown per rev, ramp clearance height, helical ramp diameter, minimum ramp diameter, predrill and entry positions | — | one user-facing tip: ramp clearance height defaults to 0.1 in above top [S: SAUNDERS] | [2DA R] [3DA R] |

Notes on semantics, all [R] unless marked:
- 3D Adaptive builds constant-Z layers and clears them bottom-up in stages: the first step at the
  tool's effective flute length, then intermediate layers [3DA].
- Changing the direction (climb/conventional) forces a full recompute for 3D Adaptive, because the
  passes depend on it; other strategies only re-link [CLIMB].
- *Stay-down level* is a cost knob: higher keeps the tool down around obstacles instead of
  retracting, at a "significant" calc-time cost [3DA].

**Offset roughing (2D/3D, ex-Pocket)**: constant stepover, may cut both directions, *allow
stepover cusps*, *morphed spiral*, smoothing deviation, lift height, rest roughing [prev R:
2D-POCKET-REF, 3D-POCKET-REF]. **Face**: stepover, pass direction, pass extension, stock offset,
both ways, *use chip thinning* (roll-on), multiple depths, extend before retract [FACE R].

### 1.2 Feeds and speeds

| Item | What Fusion has | Source |
|---|---|---|
| Tool tab Feed & Speed | preset; spindle rpm ⇄ surface speed; ramp spindle speed; cutting feed ⇄ feed per tooth; lead-in, lead-out, **transition**, ramp, plunge feeds; plunge feed per rev; drilling adds retract feed, feed-per-rev mode | [TOOLTAB R] |
| Linking-tab feeds | high feedrate (G1 stand-in for G0), no-engagement feedrate (adaptive stay-down moves) | [3DA R] |
| Presets | several named cutting-data presets per tool; a new one copies the default; computed from surface speed and feed per tooth/rev; copy-paste to same-type tools recomputes expressions | [PRESET R] |
| Expressions | cutting data (e.g. stepover) can be an expression of tool geometry (`tool_diameter*.10`) | [EXPR R] |
| Stock materials | a stock material groups physical materials (with keywords, hardness); presets are associated with stock materials; the setup picks the stock material from the model's physical material (default Steel → Low-Carbon Steel) and the tool picker groups matching presets first | [STOCKMAT R] [STOCKMAT-ASSOC R] [STOCKMAT-SEL R] |
| Feed optimization | reduce feed where direction change > angle or radius < reduced-feed radius, over a reduced-feed distance before the corner, to a fixed reduced feed; inner corners only option | [FEEDOPT R] |
| Chip thinning | none as feed compensation; Face's option is a roll-on entry | [FACE R] |
| Feeds calculator / AI feeds | none found in help TOC; Autodesk Learn tutorials describe the preset tab as the "Feed and Speed calculator" [S: LEARN-FS]; AI CAM is announced as path generation that learns from edits, not feeds [S: AI-BLOG] | [S] |
| Third-party | CloudNC CAM Assist claims physics-based feeds inside Fusion [prev R: fusion-advanced C1]; Sandvik CoroPlus add-in writes presets [prev S] | — |
| Machining Extension | gates Steep and Shallow, Corner, Deburr, toolpath modifications, multi-axis [prev R: EXT]; I found no feeds feature in it | [prev R] |

### 1.3 Finishing

Strategy list, roles and extension gating are in fusion-workflow-2d-3d.md §4 [prev R]. Below is
what bears on finish quality, re-read today.

| Quality control | What it does | Where | Source |
|---|---|---|---|
| Tolerance | chord error; total = generation + triangulation (+ smoothing) | all | [3DA R] [PAR R] |
| Smoothing (fit arcs) | replaces collinear runs with one line and tangent runs with arcs; off by default; help says code can shrink ≥50%; planar paths (parallel) filter well, Scallop less | most 2D/3D | [SMOOTH R] [3DA R] |
| Legacy smoothing tolerance | an older separate arc-fit tolerance kept on old ops, now only in Compare and Edit; fitted arcs may deviate by it **plus** the cut tolerance | old ops | [LEG-SMOOTH R] |
| S&S smoothing modes | *Fit arcs* or *Evenly spaced points* with a *maximum spacing*; smoothing tolerance factor default 0.5 (editable only via Compare and Edit) | Steep and Shallow | [SS-SMOOTH R] |
| Cusp height | derive stepover/stepdown from target scallop; assumes an inclined plane by default | finishers | [CUSP R] |
| S&S stepdown mode | *Constant stepdown* or *Constant cusp* (steep stepdown from tool, cusp and local slope) | Steep and Shallow | [SS R] |
| Threshold angle | split steep (contour) vs shallow (parallel or scallop), measured from horizontal, 0–90° | Steep and Shallow | [SS R] |
| Overlap distance, wall clearance, smooth offsets, remove cusps at junctions, continuous (spiral) | blend zones; keep shallow passes off steep walls (parallel only); round sharp offset corners that cause dwell marks; extra pass through apexes where offset corners exceed the stepover; replace closed loops with spirals | Steep and Shallow | [SS R] |
| Optimize parallel passes | add perpendicular passes on areas steeper than 45°, drop overlapping ones (extension) | Steep and Shallow | [SS-PAROPT R] |
| Machine steep areas, add perpendicular passes, min steep stepover, max steep stepdown, simple ordering, up/down milling | Parallel's own steep-wall handling | Parallel | [PAR R] |
| Slope confinement | from/to slope angle from horizontal | most 3D finishers | [SLOPE R] |
| Contact only | default on; off extends passes across openings to the boundary | most 3D finishers | [CONTACT R] |
| Contact point boundary | boundary limits the contact point, not the tool centre | finishers | [CPB R] [SCAL R] |
| Surface manager | surface groups: machine, avoid, ignore, fixture, each with radial/axial clearance or stock, added to the tab-wide values | Geometry tab | [SURFMGR R] |
| Scallop: inside/outside, link inside to outside, limit number of stepovers, machine areas using boundaries | constant-stepover offsets along the surface | Scallop | [SCAL R] |
| Max remaining stock | reorder steep passes on nearby walls to alternate by Z | Steep and Shallow | [SS R] |
| Leads normal to surface | S&S leads enter along the surface normal | Steep and Shallow | [SS R] |
| Rest finishing | *Corner* with a reference tool (extension); rest machining sources on finishers | — | [prev R: CORNER, 3D-POCKET-REF] |
| Point randomisation | **not offered**; a forum user notes Mastercam had a point-randomising option Fusion lacked [S: FACET] | — | [S] |

### 1.4 Linking

| Parameter | Options | Source |
|---|---|---|
| Retraction policy | full retraction (to retract height), minimum retraction (just clear the part + safe distance), shortest path (straight line; unsafe on machines that dog-leg G0) | [3DA R] [SCAL R] |
| High feedrate mode | preserve all rapids; axial+radial only; axial only; radial only; single-axis only; always high feed (G1) | [3DA R] |
| High feedrate | the G1 feed used in place of G0 | [3DA R] |
| Allow rapid retract | G0 retracts, or retract at lead-out feed | [3DA R] |
| Keep tool down, max stay-down distance | must be ticked on 2D Contour, 2D Offset Roughing, Trace, Engrave; absent on Thread, Bore, Circular, Chamfer, Rotary, Steep and Shallow; always on elsewhere (set 0 to force retracts) | [STAYDOWN R] |
| Lift height | lift off the floor on stay-down moves (Offset Roughing, Adaptive, Horizontal); default 0 | [LIFT R] |
| Transition type | no contact, straight line, shortest path, smooth (tangent arcs; ramp never steeper than the ramp angle) | [TRANS R] |
| Leads | horizontal and vertical lead-in/out radius; S&S uses surface-normal leads | [3DA R] [SS R] |
| Ramps | see §1.1 | [2DA R] |
| Safe distance | min clearance on retracts | [prev R: CONTOUR-REF] |

### 1.5 Output and control

| Item | What Fusion does | Source |
|---|---|---|
| Arc output | G2/G3 from smoothing in all sensible planes; the post linearises arcs the control lacks | [SMOOTH R] |
| NURBS / splines | no post entry point for splines | [POSTAPI R] |
| Point spacing for modern controls | *Evenly spaced points* in Steep and Shallow only | [SS-SMOOTH R] |
| Data-starving guidance | help names block-rate limits (40 to 1,000+ blocks/s) and warns against tight tolerance at high feed | [3DA R] |
| Cycle time | feed + rapid + tool-change time; feed scale, rapid feed, tool-change time editable in the dialog | [MTIME R] [MTIME-HOW R] |
| Machine definition feed ratio | a feedrate ratio in the machine definition scales the estimate; users tune it by hand (92%, 70% quoted) | [S: TORMACH; fetched R, it is a user forum] |
| Simulation | stock comparison colouring and collisions; motion between points "may be wrong" without a machine | [prev R: SIM, SIM-NOM] |

---

## 2. Strengths and weaknesses from user reports

All of this is **user report**, not fact, unless a help page is cited.

### 2.1 Adaptive roughing

- *Strength, widely repeated:* adaptive is the easiest constant-engagement rougher to drive; in a
  Titans of CNC forum thread an instructor called Fusion "the easiest by far" compared with getting
  the same from Mastercam [S: TITANS].
- *Sharp corners:* users hitting corner problems seemed to use flat cutters with no corner radius
  (HSM ideas thread) [S: HSM-IDEA]. The help's own answer is *Minimum cutting radius*, which trades
  engagement for rest material [2DA R].
- *Thin slivers:* a forum advisor said thin slivers on some open pockets are unavoidable with a
  constant-engagement strategy and unlikely to be fixed inside adaptive [S: SLIVER].
- *Jagged motion under rest machining:* a poster traced jagged 3D Adaptive moves to rest machining;
  removing it gave a smooth path [S: JAGGED].
- *Missed stepdowns:* a thread titled "3D adaptive collision due to missing stepdowns" (2024) [S:
  MISSING-STEP]; content not read.
- *Retract-heavy micro-tool paths:* a knowledge-base post reports many unnecessary retracts with
  micro-tools in mould features [S: RETRACTS].
- *Generation time:* reports of 20–30 min at 52% for a 3D adaptive; 2D adaptive "taking hours" on
  simple geometry; low CPU use (~20%) during 3D adaptive rest machining; one user cut a regen to 189
  s by raising optimal load [S: SLOWGEN] [S: LOWCPU]. An Autodesk staff reply (snippet) says the
  kernel is multicore but adaptive is sequential because each pass depends on stock left by the last
  [S: LOWCPU]. The help itself warns flat-area detection, stay-down level and reduce-air-cutting
  add calc time [3DA R] [AIRCUT R].
- *Default optimal load too high:* one user says the 40% D default is "way too aggressive" even on
  a Haas VF-2 [S: OPTLOAD-FORUM].

### 2.2 Feeds and speeds

- No engagement- or force-based feed; corner slowdown is geometric only [FEEDOPT R]. Forum reports
  that dense linear segments hide corners from feed optimisation unless smoothing first fits arcs
  are cited in the sibling note [prev S].
- A forum user asked whether chip thinning can be applied to adaptive facing; no answer in the
  snippet [S: CHIPTHIN-Q]. An Autodesk blog explains chip-thinning math for manual use [S: FORCES].

### 2.3 Finishing

- *Faceting on curved surfaces:* a user saw facets aligned with toolpath points; a moderator
  advised tightening tolerance and smoothing (0.002 → 0.0002 in) [S: FACET].
- *Steep walls with Parallel:* Fusion's own docs say Parallel suits shallow areas and Contour steep
  ones; the remedies are slope confinement, perpendicular passes, S&S wall clearance and parallel
  optimisation (extension) [PAR R] [SLOPE R] [SS R] [SS-PAROPT R].
- *Scallop corners:* S&S adds *Smooth offsets* and *Remove cusps at junctions* precisely because
  inward offsets grow sharp corners whose apex spacing exceeds the stepover and cause dwell marks
  [SS R]. Plain Scallop does not list those two options [SCAL R]. A 2015 thread is titled
  "Scalloping path not smooth" [S: SCAL-SMOOTH].
- *Continuous passes* are not available in plain Scallop per an Autodesk blog (snippet) [S:
  WHY-SS]; S&S has them [SS R].

### 2.4 Linking

- Shortest-path retraction is unsafe on dog-leg rapids; help itself says so [3DA R].
- Excess retracts on adaptive with micro-tools [S: RETRACTS]; users ask for more keep-tool-down
  control ("Adaptive Clearing, Keep Tool Down", 2025) [S: KTD].

### 2.5 Output

- No NURBS output [POSTAPI R]. Arc-fit quality across planes is not documented beyond "all planes
  that make sense" [SMOOTH R].

### 2.6 Cycle-time estimate

- Thread "Machining time estimates are junk" (2017–2021): an Autodesk product manager (snippet)
  said 3D estimates have been off by 2–10× in his experience, less on 2D; users say Fusion has no
  acceleration model; machining time, simulation time and setup sheet time disagreed on one setup
  (1:54, 2:49, 2:29); one Haas user saw near-perfect agreement; suggested fix is calibrating the
  feed scale (one user ~45%) [S: JUNK].
- Tormach forum: users tune the machine-definition feedrate ratio (92% on a Tormach, 70% on a
  larger machine), say Fusion assumes effectively infinite acceleration, and use the estimate for
  relative comparisons [TORMACH R, user forum].
- A user notes the time ignores machine dynamics, so an adaptive path can look faster in Fusion and
  run slower on the machine for small slots [S: TITANS-DYN].
- Drilling/tapping times reported wrong in a separate thread; Autodesk acknowledged (snippet) [S:
  JUNK2].

### 2.7 Cloud

- Autodesk says CAM generation runs locally and connection speed is not the cause of slow CAM
  commands [S: PERF]. Cloud dependence for licensing and file storage is [K]; not re-read.

### 2.8 Not found

No Practical Machinist, cnczone or Reddit thread content was readable (403 or not in results). No
NYC CNC / Saunders video content was read; only the Autodesk blog's snippet about ramp clearance
[S: SAUNDERS].

---

## 3. What competitors have for toolpath quality that Fusion lacks

| Product | Capability Fusion lacks | Evidence |
|---|---|---|
| Mastercam | **RCTF** (radial chip thinning factor): a tool-page checkbox that raises the feed from the operation's stepover (example: 24.4 → 40.7 ipm at 10%); on 2D HS, Face, Pocket, Slot, Horizontal Area, Dynamic OptiRough, Area Roughing, Circle Mill; ISCAR HEM mode adds a 1–5 factor and changes rpm. It is a per-operation scale, not along the path. | [CAMINSTR R] |
| Mastercam | **Arc feedrate override** (2024): speed up on outside arcs, slow down on inside arcs, by % or absolute (Dynamic Contour, Peel Mill, Model Chamfer) — radius-compensated feed, which Fusion's corner slowdown is not | [MC-ARCFEED R] |
| Mastercam | **Maximum engagement** (2024, Dynamic Mill/Face/OptiRough): reach the programmed stepover as fast as possible for more nominal chips | [MC-MAXENG R] |
| Mastercam | Dynamic Motion marketing: holds chip thickness, smooth entries/exits; no parameters given | [MC-DYN R] |
| Mastercam | micro-lifts and accelerated back-feed on non-cutting moves (trade press) | [S: MC-MICRO] |
| SolidCAM iMachining | **Technology Wizard**: from tool, workpiece material and machine limits it computes feed, rpm, axial depth, engagement angle and chip thickness **for each segment** of the path; 8-level aggressiveness slider for rigidity | [IMACH R] |
| hyperMILL MAXX HPC | feed adapted along the path from local conditions (edges, wrapping); user sets min and max feed per situation (approach, plunge, in-plane); full cuts and abrupt direction changes avoided; Celeritive HPC technology | [HM-HPC R] |
| PowerMill Vortex | engagement capped, switching to trochoidal near the limit; help says engagement stays within about 15% of a straight cut at the same stepover; pair with step cutting; *Tolerance and keep arcs* point distribution | [VORTEX R] (2022 help) |
| PowerMill | MachineDNA point spacing so the machine holds programmed feed [S: VORTEX-S] | [S] |
| VERICUT Force | physics-based feed optimisation on posted NC (vendor claims ~40% cycle time) | [prev R: fusion-advanced V1] |
| Okuma, others | control-side spline smoothing (Super-NURBS) | [S: OKUMA] |

What none of them documents (in what I read): a **certified** bound that the swept tool stays
within ε of the exact surface, or a certified bound on engagement between samples. Both are the
forgeCAD differentiators flagged in design.md §5.

---

## 4. The "Fusion-level" checklist (testable)

Conventions. **Metric** is computed by forgeCAD's own analysis of the path against exact stock
and part (the L2 reference in design.md §5.3), or by a dexel/voxel simulation where marked.
**Threshold** values are *proposals* for forgeCAD, not numbers Fusion publishes; where Fusion's help
states a behaviour, the item reproduces that behaviour, and the threshold is how we check it.
φ(D, a) = arccos(1 − 2a/D) is the straight-cut engagement for radial width a (literature.md §2).
"Bench" names the part from §5. "Src" is the evidence that Fusion has the capability. Items marked
**BETTER** are where Fusion has nothing to match, so they are beyond-Fusion goals.

### 4.1 Roughing (R)

| # | Capability | Test on bench | Metric, pass threshold | Src |
|---|---|---|---|---|
| R1 | Adaptive engagement cap everywhere | B1, B2, B5 at OL = 10%, 25%, 40% D | max engagement angle along path (sampled every ≤0.05 D, plus certified between samples) ≤ φ(D, OL)·1.10, **including inner corners, entries and island approaches** | [3DA] "guarantees max load" |
| R2 | No full-width slotting | B1–B5 | length of cutting moves with engagement ≥ 170° = 0, except inside declared slot clearing, helix ramps, or plunges | [3DA] [SLOTCLR] |
| R3 | Engagement stays near the cap (efficiency) | B1, B2 | fraction of cutting length with engagement ≥ 0.6·φ(OL) ≥ 70% (excluding entries) | [3DA] (varies stepover ≤ OL) |
| R4 | Climb-only option holds | B1 | conventional-cut length / total ≤ 2% with direction = climb | [3DA] [CLIMB] |
| R5 | Both ways with separate load and feed | B3 (open pocket) | conventional passes obey OL_other ≤ cap·1.10 and carry the other-way feed | [2DA] |
| R6 | Minimum cutting radius | B1 (sharp inside corners) | min path curvature radius ≥ set value; rest material reported per corner | [3DA] [2DA] |
| R7 | Slot clearing | B2 (long narrow pocket) | centre slot exists of the set width; total link length ≤ that without slot clearing | [SLOTCLR] |
| R8 | Stepdown, even stepdowns, final pass | B5 | no Z step > max; with even on, all steps equal within 1 µm | [2DA] |
| R9 | Fine stepdown (upward) | B6 (freeform core) | max terrace height left ≤ fine stepdown + axial stock | [3DA] |
| R10 | Flat area detection | B5 (floors at odd heights) | stock on every flat = axial stock-to-leave ± tolerance | [FLATDET] |
| R11 | Machine shallow areas | B6 | max horizontal terrace width ≤ max shallow stepover, or step = min shallow stepdown | [SHALLOW] |
| R12 | Stock to leave, radial and axial, negative allowed | B1, B6 | measured remaining stock on walls/floors = set value ± tolerance; negative radial refused when ≥ tool radius (typed error) | [3DA] [2DA] |
| R13 | Rest machining from previous ops | B1 with 12 mm then 4 mm tool | 4 mm path cuts only where IPS has material > ignore-threshold; air-cut length ≤ 10% of its cut length | [3DA] [AIRCUT] |
| R14 | Reduce air cutting | B7 (overhanging stock, flipped setup) | air-cut length reduced vs. off; help's own example 4:44 → 3:20 | [AIRCUT] |
| R15 | Order by depth / by area | B4 (several cavities) | ordering matches the option | [3DA] |
| R16 | Helix entry fits or refuses with a fix | B2, B4 | helix diameter in [min, max]; if none fits, a typed error with the largest feasible diameter (Fusion only logs a warning) | [2DA] [prev R: WARN] |
| R17 | Engagement-capped ramp | B2 | ramp stepdown per rev ≤ max ramp stepdown; ramp angle ≤ set | [2DA] |
| R18 | Generation time | B6 at OL 15% D, 0.01 mm tol | regen ≤ 10 s native on the bench box; no Fusion figure exists, users report minutes to hours [S] | user reports |
| R19 | Determinism | all | same inputs → bit-identical path native and wasm | design.md §5.5 (BETTER) |

### 4.2 Feeds and speeds (F)

| # | Capability | Test | Metric, pass threshold | Src |
|---|---|---|---|---|
| F1 | Full Tool-tab feed set | any op | rpm⇄Vc, feed⇄fz, ramp rpm, lead-in/out, transition, ramp, plunge (and per-rev) feeds all present and emitted in the right moves | [TOOLTAB] |
| F2 | Named presets with expressions | tool library | preset fields can be expressions of tool geometry and recompute on copy | [PRESET] [EXPR] |
| F3 | Material-filtered presets | setup with Al and steel bodies | setup's stock material selects matching presets first | [STOCKMAT-SEL] |
| F4 | Corner feed reduction | B1 contour | feed reduced over the set distance before every corner with turn > set angle or radius < set radius; inner-only honoured | [FEEDOPT] |
| F5 | Corner detection survives dense lines | B6 finish path before smoothing | corners found on unsmoothed polyline paths (users report Fusion misses them) | [prev S] |
| F6 | No-engagement feed on stay-down moves | B1 adaptive | every non-cutting stay-down move at no-engagement feed | [3DA] |
| F7 | High-feed G1 in place of G0 per mode | all | each of the 6 high-feedrate modes produces exactly the G0/G1 split described | [3DA] |
| F8 | **BETTER** Chip-thinning compensated feed | B1 adaptive | feed per segment so that h_max = fz_target within ±5% given local engagement (Mastercam RCTF does this per op only) | [CAMINSTR] (competitor) |
| F9 | **BETTER** Engagement-driven feed along the path | B1, B5 | feed varies with local engagement; never exceeds a machine feed cap; reported per segment | [IMACH] [HM-HPC] (competitor) |
| F10 | **BETTER** Power/torque check | B5 steel | spindle power from k_c1 model ≤ machine curve everywhere, else typed warning with the place | libraries-machines-posts-feeds.md §3.4 |

### 4.3 Finishing (S)

| # | Capability | Test | Metric, pass threshold | Src |
|---|---|---|---|---|
| S1 | Tolerance honoured against the **exact** surface | B6 parallel and scallop | one-sided distance from swept ball to exact B-rep within [−tol, +tol + stock]; certified, not sampled (Fusion: against a triangulation) | [3DA] (BETTER: exact) |
| S2 | Cusp-height-driven stepover | B6, walls 0–60° | measured scallop (simulated) ≤ target on all faces ≤ 60° for parallel; ≤ target everywhere for scallop | [CUSP] |
| S3 | Parallel with steep-area passes | B6 | with machine-steep-areas on, max scallop on faces 60–85° ≤ 1.5× target | [PAR] |
| S4 | Perpendicular cross passes | B6 | second pass set at 90° exists | [PAR] |
| S5 | Contour (waterline) stepdown and shallow insertion | B6 | stepdown ≤ max; shallow terraces ≤ max shallow stepover | [prev R: CONTOUR3D-REF] [SHALLOW2] |
| S6 | Slope confinement | B6 | only faces with slope in [from, to] cut (contact points) | [SLOPE] |
| S7 | Steep/shallow split with overlap | B6 | regions split at threshold angle; overlap band = set distance | [SS] |
| S8 | Constant-cusp steep stepdown | B6 | steep cusp within ±10% of target across slopes 45–85° | [SS] |
| S9 | Scallop corner quality | B6, B4 | no offset corner with apex spacing > 1.2× stepover unless a junction pass removes it; sharp offset corners smoothed (min radius ≥ set) | [SS] |
| S10 | Continuous spiral finishing | B6 | number of lifts in steep and shallow regions ≤ number of regions | [SS] |
| S11 | Wall clearance on shallow passes | B6 | shallow passes stay ≥ set distance from steep walls | [SS] |
| S12 | Contact only / contact point boundary | B6 | with CPB, contact points stay inside the face boundary; with contact only off, passes cross openings | [CONTACT] [CPB] |
| S13 | Avoid / machine / fixture surface groups | B6 + clamp | avoid surfaces untouched within clearance; fixture clearance from setup added | [SURFMGR] |
| S14 | Pencil along concave creases | B6 fillets | pencil pass on every concave crease with radius ≤ tool radius + overthickness | [prev R: PENCIL-REF] |
| S15 | Rest finishing by reference tool | B6 with 10 mm then 3 mm ball | only regions the reference tool cannot reach are cut; leftover ≤ tol after both | [prev R: CORNER] |
| S16 | Flat finishing exact on flats | B5 | every planar floor finished, floor stock = 0 ± tol | [prev R: FLAT] |
| S17 | Arc fitting in all planes | B6 parallel (XZ planes) | ≥ 50% block reduction on planar paths; fitted arcs within smoothing tol + tol | [SMOOTH] |
| S18 | Evenly spaced points option | B6 S&S | no gap > max spacing on cutting moves | [SS-SMOOTH] |
| S19 | **BETTER** Point-pattern artefacts | B6 parallel | neighbouring passes' point positions not aligned (measured cross-pass alignment); reported as a lint | user report [S: FACET] |
| S20 | Gouge-free | all finishing | certified min distance from tool to part ≥ −1 µm (Fusion has only simulation colouring) | design.md §5 (BETTER) |

### 4.4 Linking (L)

| # | Capability | Test | Metric, pass threshold | Src |
|---|---|---|---|---|
| L1 | Retraction policies | B4 | full: every retract reaches retract height; minimum: retract = local clearance + safe distance; shortest: straight line, refused if machine marks G0 as dog-leg | [3DA] |
| L2 | Safe distance | B4 | min tool-to-part clearance during links ≥ safe distance + stock | [prev R: CONTOUR-REF] |
| L3 | Max/min stay-down distance | B1 | no stay-down move longer than max; none shorter than min | [3DA] [2DA] [STAYDOWN] |
| L4 | Stay-down level trade-off | B4 | retract count decreases monotonically from Least to Most | [3DA] |
| L5 | Lift height | B1 | stay-down moves sit at floor + lift | [LIFT] |
| L6 | Leads: horizontal/vertical radii | B1, B6 | lead arcs exist with set radii; tangent (G1 continuity) at path join | [3DA] |
| L7 | Surface-normal leads | B6 S&S | lead direction = surface normal ± 5° | [SS] |
| L8 | Transition types | B6 | smooth transitions never exceed ramp angle; no-contact transitions never touch part | [TRANS] |
| L9 | Ramp types | B2 | each of plunge-outside, zig-zag, predrill, profile, plunge, smooth profile, helix produced as described | [2DA] |
| L10 | No rapid into stock | all | zero G0 moves intersecting IPS (exact) | [prev R: SIM] |
| L11 | Allow rapid retract off | B1 | retracts emitted at lead-out feed | [3DA] |
| L12 | Link length budget | B1, B4 | total non-cutting length ≤ Fusion's on the same part (measured once by exporting Fusion's NC for the bench part, if David can run Fusion) | comparison |

### 4.5 Output and time (O)

| # | Capability | Test | Metric, pass threshold | Src |
|---|---|---|---|---|
| O1 | Chord tolerance on output | all | every emitted segment within tol of nominal | [3DA] |
| O2 | Block-rate awareness | B6 at 5,000 mm/min | lint when segment time < 1/(control block rate); fix = loosen tol or fit arcs | [3DA] (help's data-starving text) |
| O3 | Arc output linearised for controls without arcs | post config | no G2/G3 when dialect lacks them | [SMOOTH] |
| O4 | Cycle-time estimate at Fusion's level | all | feed + rapid + tool change reported per op, with feed scale | [MTIME] |
| O5 | **BETTER** Jerk-limited estimate | B1, B6 | estimate within 10% of a jerk-limited trapezoid/S-curve simulation (Ward et al. 2021 style) using the machine file's a_max, j_max; and within 15% of a measured run when available | [S: WARD] |
| O6 | Consistency | all | machining time, simulation time and setup sheet time identical (users report Fusion's differ) | [S: JUNK] (BETTER) |
| O7 | **BETTER** Spline output | B6 on a dialect with G6.2/G5.1 | optional NURBS blocks within tol; fallback arcs | [POSTAPI] (Fusion lacks) |
| O8 | Round-trip | all | re-reading posted G-code reproduces the path within dialect digits | design.md G6 (BETTER) |

Count: 19 + 10 + 20 + 12 + 8 = **69 items**; 52 are Fusion-parity, the rest marked BETTER.

---

## 5. Benchmark parts

| ID | Part | Use | Link | Licence | Status |
|---|---|---|---|---|---|
| B1 | **forgeCAD-authored pocket set**: rectangular pocket with sharp (r = 0) and r = 0.6 D inside corners, a round island, a 1.1 D-wide neck between two lobes | R1–R6, R13, F4, F6, L3–L6 | to write as `.fg` | ours | proposed; exact L2 reference possible (lines and arcs) |
| B2 | **Long narrow pocket** 1.6 D wide, 10 D long, closed | R7, R16, R17, L9 | `.fg` | ours | proposed |
| B3 | **Open pocket / step** open on one side, thin wall 0.3 D | R5, sliver behaviour [S: SLIVER] | `.fg` | ours | proposed |
| B4 | **Multi-cavity plate** with cavities at different depths | R15, L1–L4, L8 | `.fg` | ours | proposed |
| B5 | **Stepped floors** at non-multiples of stepdown; one floor steel-load test | R8, R10, S16, F10 | `.fg` | ours | proposed |
| B6 | **Freeform core/cavity**: ruled walls 30–85°, shallow dome, concave fillets r = 2, 5 mm, convex blend | S1–S20, R9, R11 | `.fg` (lofts and fillets) | ours | proposed |
| B7 | **Two-setup part** with overhanging stock for flip | R13, R14 | `.fg` | ours | proposed |
| B8 | **ISO 10791-7 test piece** (circle-diamond-square positioning and contouring part; 160 and 320 mm sizes; 2020 edition adds a 5-axis freeform piece in Annex A) | contour accuracy, corners, cycle time | https://www.iso.org/standard/73814.html ; preview https://webstore.ansi.org/preview-pages/ISO/preview_ISO+10791-7-2020.pdf | ISO copyright; the standard is paid. Model our own part from the published shape class; do not ship ISO's drawing | [S] |
| B9 | **NAS 979** circle-diamond-square | historic equivalent of B8 | Renishaw white paper https://renishaw.com/resourcecentre/en/download/--99071 | inactive for design since 2013-05-31 per Renishaw [S]; ISO 10791-7 and ASME B5.54 carry the test now [S] | reference only |
| B10 | **NIST MBE PMI test cases** (CTC/FTC/STC and "modified" cases remodelled to be easier to make) | real prismatic parts with tolerances; STEP | https://www.nist.gov/el/systems-integration-division-73400/mbe-pmi-validation-and-conformance-testing/download-free-cad ; data record https://data.nist.gov/od/id/276B3C17848DEC5BE0531A570681E6081452 | NIST: use without restriction, acknowledgement requested, no logo use [S]; not stated as public domain | [S] |
| B11 | **FreeCAD CAM test files**: `test_adaptive.fcstd`, `test_profile.fcstd`, `test_holes00.fcstd`, `test_geomop.fcstd`, `boxtest.fcstd`, `dressuptest.FCStd`, `Drilling_1.FCStd`, fixtures `test_28534_truncated_pocket.FCStd`, `test_17748_cam_profile.FCStd` (listed from main at 8fef74c9, 2026-11-23) | differential tests against FreeCAD Adaptive on the same geometry | https://github.com/FreeCAD/FreeCAD/tree/main/src/Mod/CAM/CAMTests | repo LGPL-2.1-or-later [K for the data files; header of Adaptive.cpp is LGPL per opensource.md] | [R listing] |
| B12 | **Fusion sample CAM parts** | side-by-side with Fusion's own output | in-product samples | licence UNKNOWN; do not redistribute | UNKNOWN |

Why mostly our own parts: the checklist needs numbers computed against an exact reference, and our
`.fg` parts (lines, arcs, planes, lofts) give closed forms (design.md L2). B8–B11 are for realism and
differential checks. Fusion parity numbers (L12, R18 comparisons) need someone with Fusion to post
the same bench parts once; that is the only way to get Fusion's link lengths, times and paths, and
I cannot do it here.

---

## 6. Open questions

1. Fusion's real defaults (optimal load, stay-down distance, lead radii, tolerances per strategy):
   only readable from the product or from a saved template/defaults file (`*.f3dhsm-defaults`)
   [prev R: DEFAULTS]. UNKNOWN.
2. Whether Fusion's adaptive cap is held between samples or only at samples: undocumented.
3. Whether the machine definition's acceleration-like fields (if any) feed the estimate: users say
   no [S: TORMACH-S]; no help page found.
4. Practical Machinist / Reddit / NYC CNC material: unread (403); if David wants it, he can paste
   threads.

---

## Sources

Fusion help, read **[R] 2026-11-24** at `https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/<ID>.htm`:

| Key | ID | Title (paraphrased) |
|---|---|---|
| 3DA | GUID09E44604-DAD8-47D6-ADC6-C100869DE724 | 3D Adaptive Roughing reference |
| 3DA-READ | 3D-ADAPTIVE-READ | 3D Adaptive Roughing strategy |
| 3DA-STEPS | 3D-ADAPTIVE-STEPS | Generate a 3D Adaptive toolpath |
| 2DA | GUIDA73542E9-ED9C-4BD9-A87D-3A0ECA8BEB41 | 2D Adaptive Roughing reference |
| 2DA-STEPS | 2D-ADAPTIVE-STEPS | Generate a 2D Adaptive toolpath |
| CLIMB | GUID3DCDDEC0-95C8-4444-8DC4-74E1B20F9413 | Conventional and climb milling (3D Adaptive) |
| SLOTCLR | GUID6A81D2D0-3123-4F08-9872-8A365705CBB1 | Slot clearing |
| SHALLOW | GUIDA90700D1-B58C-4218-BB25-5F7F4DF27325 | Shallow area machining (Adaptive) |
| SHALLOW2 | GUIDC3082500-D32B-489E-A548-4FE2BECE6A7F | Shallow areas |
| AIRCUT | REDUCE-AIR-CUTTING-ADAPTIVE | Reduce air cutting |
| FLATDET | GUID1C1BF6D5-F1AD-4C85-9EB4-EBD58448DE14 | Detecting flat areas |
| FACE | GUID9CEA7D03-83C1-4ADD-B6B7-7FFF9DFC97A9 | Face reference |
| PAR | GUID17CC1F90-A2A5-4CC3-80EB-B7972E327E0F | Parallel finishing reference |
| PAR-STEPS | 3D-PARALLEL-STEPS | Generate a Parallel toolpath |
| SCAL | GUID77067110-3D88-4DA7-B171-6B27F04CF8B3 | Scallop finishing reference |
| SCAL-STEPS | 3D-SCALLOP-STEPS | Generate a Scallop toolpath |
| CONTOUR3 | GUIDEA850013-D826-408E-B076-C613834A77F3 | Contour finishing reference (downloaded; linking text checked) |
| PENCIL | GUIDD7ECF22F-BEAA-4B6A-A2CE-DA6F2C08D787 | Pencil reference (stepover text checked) |
| RAMP | GUIDB4D404D3-F897-4FF2-A684-3121C3DDB062 | Ramp finishing reference (downloaded) |
| SS | GUID-A6933B73-D98F-43AD-BCFD-A450D91C4E61 | Steep and Shallow reference |
| SS-SMOOTH | Fusion_CAM_milling_3d_milling_overview_steep_shallow_overview_steep_shallow_smoothing_html | Smoothing for Steep and Shallow |
| SS-PAROPT | 3D-STEEP-AND-SHALLOW-PARALLEL-PASSES-OPTIMIZATION | Parallel passes optimisation |
| SMOOTH | GUID5A951723-4797-4F5F-A7A3-45DCBCA2D26F | Smoothing reference |
| LEG-SMOOTH | MFG-REF-COMMON-LEGACY-SMOOTHING | Legacy smoothing tolerance |
| CUSP | MFG-CUSP-HEIGHT-OVERVIEW | Cusp height |
| FEEDOPT | GUIDCFB956EA-793B-4F8A-B487-028219913D96 | Feed optimisation |
| STAYDOWN | GUID2E5ABA80-7240-4F1A-989C-2410C4A0CCE5 | Stay-down distance |
| LIFT | GUIDE697A20E-3EA7-4149-B158-ACDFA61E136A | Lift height and stay-down moves |
| TRANS | GUID32FB2A12-1443-43C9-9C33-A4946E36A888 | Transitions between passes |
| SLOPE | GUID544B46E7-E0EC-47C3-B50B-7252CB8303F0 | Slope confinement |
| CONTACT | GUID2042554C-8B7D-4778-A6E5-97030447A71C | Toolpath without surface contact (Contact Only) |
| CPB | GUIDA16162AB-95D5-4086-B54D-926B9A9BB429 | Contact point boundary |
| SURFMGR | GUIDFCF0C83A-507A-48E3-9F08-15F104B20CBF | Surface Manager |
| TOOLTAB | MFG-REF-COMMON-TOOL | Tool tab settings reference |
| PRESET | MFG-CREATE-TOOL-PRESET | Work with tool presets |
| EXPR | MFG-REF-LIB-EXPRESSION | Expressions in the Tool Library |
| STOCKMAT | MFG-STOCK-MATERIALS-CONCEPT | Stock materials |
| STOCKMAT-ASSOC | MFG-STOCK-MATERIALS-ASSOCIATE-PRESETS | Associate a preset with a stock material |
| STOCKMAT-SEL | MFG-CONFIGURE-STOCK-MATERIALS-TOOL-SELECTION | Stock materials for tool selection |
| MTIME | MFG-MACHINING-TIME-OVERVIEW | Machining time |
| MTIME-HOW | GUID0D142E92-5E3F-4E60-AB76-EEA0E5237845 | Display the machining time |
| TOC | https://help.autodesk.com/view/fusion360/ENU/data/toctree.json | help table of contents (used to find IDs) |

Other sources:

| Key | URL | Tag |
|---|---|---|
| POSTAPI | https://cam.autodesk.com/posts/reference/entry_functions.html and .../classPostProcessor.html | [R] |
| TORMACH | https://forums.tormach.com/t/machining-time-feed-rate-ratio-in-fusion/2821 | [R] user forum |
| CAMINSTR | https://blog.caminstructor.com/chip-thinning-rctf-and-iscar-hem | [R] reseller/trainer blog |
| MC-ARCFEED | https://my.mastercam.com/ru/whats-new-2024/whats-new-2024_override_arc_feedrate | [R] |
| MC-MAXENG | https://my.mastercam.com/whats-new-2024/whats-new-2024_maximum_engagement | [R] |
| MC-DYN | https://www.mastercam.com/community/blog/how-dynamic-motion-optimizes-tool-engagement-automatically/ | [R] vendor |
| IMACH | https://solidcam.com/imachining/imachining-technology-wizard/ | [R] vendor |
| HM-HPC | https://openmind-tech.com/en/cam/hypermill-maxx-machining/roughing-hpc/high-performance-milling | [R] vendor |
| VORTEX | https://help.autodesk.com/cloudhelp/2022/ENU/PWRM-ReferenceHelp/files/GUID-77122AF8-A489-43EB-A542-E21EAA660A0D.htm | [R] (2022 help) |
| FREECAD-TESTS | https://github.com/FreeCAD/FreeCAD/tree/main/src/Mod/CAM/CAMTests (sparse git fetch of main) | [R] listing |
| JUNK | https://forums.autodesk.com/t5/fusion-manufacture-forum/machining-time-estimates-are-junk/td-p/7035875 | [S] (403 to fetch) |
| JUNK2 | https://forums.autodesk.com/t5/fusion-manufacture-forum/machining-time-does-it-show-the-correct-time/m-p/13324181 | [S] |
| TORMACH-S | https://forums.autodesk.com/t5/fusion-manufacture/accurate-cycle-time-estimates/td-p/13002605 | [S] |
| WARD | https://eprints.whiterose.ac.uk/185222/ (Ward et al. 2021, feedrate and cycle time with interpolator dynamics) | [S] |
| HSM-IDEA | https://forums.autodesk.com/t5/hsm-ideas/adaptive-machining-improvement/idc-p/7090661 | [S] |
| SLIVER | https://forums.autodesk.com/t5/fusion-360-ideastation-request-a/adaptive-clearing-leaving-thin-walls-when-hsm-gets-scary-and/idi-p/7233288 | [S] |
| JAGGED | https://forums.autodesk.com/t5/fusion-manufacture/3d-adaptive-jagged-cutting-moves-that-should-be-smooth/m-p/12944380 | [S] |
| MISSING-STEP | https://forums.autodesk.com/t5/fusion-manufacture/3d-adaptive-collision-due-to-missing-stepdowns/td-p/13063061 | [S] title only |
| RETRACTS | https://industrialmonitordirect.com/blogs/knowledgebase/fusion-360-3d-adaptive-multiple-tool-retracts-fix | [S] |
| KTD | https://forums.autodesk.com/t5/fusion-manufacture-forum/adaptive-clearing-keep-tool-down/td-p/13772503 | [S] title only |
| SLOWGEN | https://forums.autodesk.com/t5/fusion-support-forum/cam-generation-for-2d-adaptive-clearing-taking-hours-with-fairly/td-p/10543832 and https://forums.autodesk.com/t5/fusion-support-forum/3d-adaptive-clearing-stuck-at-44-9/td-p/7836180 | [S] |
| LOWCPU | https://forums.autodesk.com/t5/fusion-manufacture/low-cpu-utilization-on-cam-path-generation/td-p/8782481 | [S] |
| OPTLOAD-FORUM | https://forums.autodesk.com/t5/fusion-manufacture/optimum-load/m-p/7265596 | [S] |
| CHIPTHIN-Q | https://forums.autodesk.com/t5/fusion-manufacture-forum/quick-tip-adaptive-facing-a-constant-engagement-facing-toolpath/td-p/6636373 | [S] |
| FORCES | https://www.autodesk.com/products/fusion-360/blog/speed-up-your-milling-with-constant-cutter-forces/ | [S] (403) |
| FACET | https://forums.autodesk.com/t5/fusion-manufacture-forum/faceted-finish-on-curved-surface/m-p/10427462 | [S] |
| SCAL-SMOOTH | https://forums.autodesk.com/t5/fusion-manufacture/scalloping-path-not-smooth/td-p/5856232 | [S] title only |
| WHY-SS | https://www.autodesk.com/products/fusion-360/blog/why-steep-and-shallow/ | [S] (403) |
| OKUMA | https://www.autodesk.com/products/fusion-360/blog/fusion-360-and-okuma-super-nurbs-achieve-superior-machined-surface-finishes/ | [S] (403) |
| SAUNDERS | https://www.autodesk.com/products/fusion-360/blog/fusion-community-spotlight-5-ways-to-improve-your-cam-cycle-time-with-john-saunders/ | [S] (403) |
| TITANS / TITANS-DYN | https://titansforum.titansofcnc.com/t/fusion-360-2d-adaptive-vs-mastercam/7856 ; dynamics remark from the same search, source thread not identified | [S] |
| PERF | https://www.autodesk.com/products/fusion-360/blog/how-to-troubleshoot-fusion-360-performance-issues/ | [S] |
| LEARN-FS | https://www.autodesk.com/learn/ondemand/tutorial/cutting-data-feeds-and-speeds | [S] |
| AI-BLOG | https://www.autodesk.com/products/fusion-360/blog/ai-powered-workflows-coming-to-autodesk-fusion/ | [S] |
| MC-MICRO | https://mmsonline.com/articles/dynamic-motion-moves-past-old-boundaries | [S] |
| VORTEX-S | PowerMill help search results (MachineDNA remark) | [S] |
| ISO / NAS / NIST | links in §5 | [S] |

Previously read keys ([prev R], 2026-11-23) resolve in `fusion-workflow-2d-3d.md` (DEFAULTS, WARN,
2D-POCKET-REF, 3D-POCKET-REF, CONTOUR-REF, CONTOUR3D-REF, PENCIL-REF, CORNER, FLAT, SIM, SIM-NOM,
EXT) and `fusion-advanced.md` (C1, V1).
