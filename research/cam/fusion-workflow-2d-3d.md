# Autodesk Fusion Manufacture: workflow, shared parameter tabs, 2D and 3D milling strategies

Research for forgeCAD CAM planning, with Autodesk Fusion as the reference product. Research only:
nothing in forgeCAD was changed. Read on 2026-11-23.

## How this was researched, and how to read the citations

- Primary source: the current Fusion help. `help.autodesk.com/view/fusion360/ENU/?guid=<ID>` is a
  JavaScript viewer that wraps static pages served at
  `https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/<ID>.htm`. I fetched those static
  pages directly (HTTP 200, ~120 pages) and read their text. The full help table of contents came from
  `https://help.autodesk.com/view/fusion360/ENU/data/toctree.json` (5,921 entries; 737 of them in
  the CAM book). That TOC is how I listed every strategy, so the lists below should be complete
  for the help as published on 2026-11-23.
- Every citation is a key such as `[SETUP-MILL]`. The source table at the end gives each key's URL.
  Unless marked otherwise, all were read on 2026-11-23.
- Tags: **[R]** read directly; **[S]** search-result snippet only; **[K]** my own prior knowledge,
  not re-read for this note. Untagged facts are [R].
- Everything here is restated in my own words; no help text is quoted.
- Ratings for forgeCAD: **must-have** (a credible first CAM release needs it), **differentiator**
  (where forgeCAD's text-first, typed, agent-driven design could do better than Fusion), **later**.

### Findings that matter before the detail

1. **Fusion has renamed its roughing strategies.** The help now uses *2D Adaptive Roughing* (was
   2D Adaptive Clearing), *2D Offset Roughing* (was 2D Pocket), *3D Adaptive Roughing* (was
   Adaptive Clearing), *3D Offset Roughing* (was Pocket Clearing) and *3+2 Roughing* (was 3+2
   Clearing) [2D-ADAPT] [2D-POCKET] [3D-ADAPT] [3D-POCKET] [3P2]. Search for the old names will hit
   older pages and forum posts.
2. **"Rest Finishing" is no longer a strategy in the current help.** No TOC entry exists for it.
   Rest work is now done with a *Rest Machining* group on each operation's Geometry tab and with the
   *Corner* strategy, which uses a reference tool [3D-CORNER] [FLAT-REST]. (I recall that older
   Fusion and HSM releases had a separate rest finishing strategy [K]; I could not confirm when it
   was removed.)
3. **The Manufacturing Extension (paid add-on) gates** Steep and Shallow, Corner, Deburr, 3+2
   Roughing, Hole Recognition, all 4/5-axis toolpaths (including multi-axis Contour, Blend, Flow,
   Geodesic), inclined flats, every toolpath modification (trim, delete passes, replace tool, move
   entry, leads and links), live probing connection and advanced inspection [EXT]. Plain 3-axis
   Blend, Flow and Geodesic do not need it; only their multi-axis modes do [BLEND-STEPS]
   [FLOW-STEPS] [GEO-STEPS].
4. **The help contradicts itself on stock transfer between setups.** The Stock tab reference says
   *Continue rest machining* is for Turning setups only [STOCK], while the rest-machining-between-
   setups page says stock transfer currently works only for *milling* setups [REST-SETUPS]
   [CONT-REST]. I could not resolve which is current.

---

## 1. The Manufacture workflow step by step

### 1.1 Overall sequence (the milling "hello world")

Fusion's own milling workflow page goes: set document units, then **Setup > New Setup** (operation
type Milling), then add an operation (for example **2D > Face**) and pick a tool from the Tool
Library. The toolpath generates when you press OK. Then **Actions > Simulate** with stock shown,
then **Create NC Program** with a post from the Post Library, then **Post**, then view the NC file
[MILL-WF].

| Step | What the user does | Source | forgeCAD rating |
|---|---|---|---|
| Setup | Machine, operation type, WCS, model, fixtures, stock, post settings | [SETUP] [SETUP-MILL] | must-have |
| Operation | Pick strategy, tool, then fill Tool/Geometry/Heights/Passes/Linking tabs | [2D-ADAPT-REF] | must-have |
| Generate | Computed on OK by default; becomes out of date when inputs change | [GEN-OV] | must-have |
| Simulate | Stock removal, collisions, with or without a machine model | [SIM] | must-have (stock-only); machine sim later |
| NC Program | Group operations, choose post, post properties, post | [NCP-OV] [NCP-REF] | must-have |
| Setup sheet | HTML operator document | [SHEET-OV] | later |

### 1.2 Setup dialog

A setup is the container for operations. It records what is machined and how: machine and
operation type, the model, the WCS orientation and origin, the stock, and post settings. For
additive, WCS, stock and post settings are replaced by build-plate arrangement [SETUP-REF].

**Operation type** [SETUP-REF] [SETUP]:

| Type | Covers | Note | forgeCAD |
|---|---|---|---|
| Milling | 2 to 5 axis mills | default | must-have |
| Turning or Mill/Turn | 2 to 5 axis lathes, live tooling, main spindle or subspindle per setup | spindle choice is a setup field | later |
| Cutting | plasma, waterjet, laser | | later (a 2D cut profile is cheap once 2D Contour exists) |
| Additive | FFF, SLA/DLP, MJF, SLS, MPBF, binder jet; DED has its own flow | needs a machine selected first | later |

**Machine.** Choosing a machine from the Machine Library sets the operation type automatically.
If the machine's linked post defines *operation properties*, Fusion shows which properties it will
add to which operations, and they then appear on each operation's Post Process tab [SETUP]. A
machine definition holds capabilities and kinematics, can link a post processor, and can link a
3D machine model for machine simulation. Machines are optional for milling, turning and cutting
[MACHINES]. *forgeCAD: later for kinematics; must-have is a minimal machine record (travels, max
rpm, rapid rate, post).*

**WCS orientation** (milling and cutting) [SETUP-MILL]. Z should point away from the face being
machined.

| Option | Meaning |
|---|---|
| Model orientation | use the model's own axes |
| Select Z axis/plane & X axis | a face or edge sets Z, another sets X; each can be flipped 180 degrees |
| Select Z axis/plane & Y axis | same, with Y |
| Select X & Y axes | two picks set X and Y |
| Select coordinate system | take origin and orientation from a user coordinate system in the model |

**WCS origin** [SETUP-MILL]: Model origin; Selected point (vertex or edge); Stock box point (a
point of the stock bounding box, for example a top corner); Model box point (same on the model
box). A setup also offers an **Accessibility Analysis** preview showing what the tool can reach from
the chosen orientation [SETUP-MILL] [SETUP].

*forgeCAD: must-have. A differentiator is to make the WCS a named, typed anchor in the `.fg` text
(like `base.end`), so `query`/`measure` can check it.*

**Model and fixtures** [SETUP-MILL]. All visible bodies are selected by default; you may pick a
subset for multi-part jobs. Any body can be marked as a fixture (vise, clamp, chuck). Collisions
with fixtures are flagged in simulation. *forgeCAD: model selection must-have; fixtures-as-bodies
must-have for collision checks.*

**Stock modes** [STOCK] [BOUNDING]:

| Mode | Parameters | forgeCAD |
|---|---|---|
| Fixed size box | width X, depth Y, height Z (always positive); either *Ground stock at model origin* (then an offset from the model origin) or per-axis model position: offset from -X/+X side, centre, or model origin (likewise Y, Z); *Round up to nearest* increment | must-have |
| Relative size box | box around the part; offset mode: none, sides plus separate top and bottom, or each side separately; round-up increment | must-have (the default) |
| Fixed size cylinder | diameter, length, position: flush front, centre, flush back | later |
| Relative size cylinder | radial, frontside and backside offsets, round-up | later |
| Fixed size tube | OD, ID, length, position | later |
| Relative size tube | radial offset (applied to ID and OD), front and back offsets, round-up | later |
| From solid | any solid body in the design (useful for forgings and castings) | must-have (a forgeCAD body is the natural stock) |
| From bounding solid | a bounding solid made in Design; the setup adopts its coordinate system; lets you place stock in a vise with joints and measure it for a BOM | later |
| From preceding setup | uses the previous setup's stock; *Continue rest machining* checkbox | must-have (two-sided parts) |

**Stock transfer between setups** [REST-SETUPS] [CONT-REST]. Typical use is flipping a part for
the second side. Both setups must reference the same single *component* (not a body, because a
component carries its origin). The second setup picks the same component, sets its WCS (usually Z
flipped), sets stock mode to From preceding setup and ticks Continue rest machining. It only takes
stock from the setup *immediately above* it in the browser. To take stock from an earlier setup, or
from another document, or to reposition it, the help suggests exporting the simulated stock as STL
and using it as solid stock. *forgeCAD: must-have, and a differentiator if transferred stock is an
exact or mesh body addressable by name (`setup1.stock_out`) rather than an STL round-trip.*

**Post Process tab of the setup** [POST-TAB]: Program name/number, program comment, **WCS offset**
(the post maps 1 to G54 and so on), **Multiple WCS offsets** (number of instances, offset increment,
ordering: preserve order, by operation, by tool). The help notes the multi-offset trick is for
fixtures whose exact positions are unknown; evenly spaced parts can use a toolpath pattern instead,
which simulates fully. *forgeCAD: WCS offset must-have; multiple offsets later.*

### 1.3 Probing the WCS (Probe WCS)

Probe WCS makes a probing operation that runs the machine's probing macros to locate stock or
already-cut features and update the setup's work offset. Results can come back from a results file
or live over a machine connection; the live path needs the Manufacturing Extension and a Machine
Connector app [PROBE-OV] [PROBE-WF].

- Tabs: Tool, Geometry, Heights, Settings [PROBE-REF].
- Feeds: link feedrate, lead-in feedrate (first touch), measure feedrate (second touch; must match
  the probe's calibration feed) [PROBE-REF].
- Probe mode: pick faces on the **model** or on the **stock**. The faces picked choose the cycle:
  single surface, angled surface, inner/outer corner, channel, wall pair (web), boss, hole, partial
  hole or boss, rectangular boss (stock centre) [PROBE-REF].
- Cycle families: single axis, corner, block centre, rectangular cavity centre, circular boss
  centre, circular hole centre, cavities with islands, between two walls [PROBE-REF].
- Options: use the exact picked point, approach distance (default twice the probe diameter),
  spacing for angle measurement, overtravel, size and position tolerances that can stop the
  machine with a message, and *Override driving WCS* to drive the probe from a different offset
  [PROBE-REF] [PROBE-OV].
- Drill also has a *Custom/Probe* cycle that only passes parameters to a machine probing cycle,
  with nothing drawn on screen [DRILL-CYC] [DRILL-REF].

*forgeCAD: later; the selection-to-cycle mapping is a neat model for typed probe ops.*

### 1.4 Browser, operation list, folders, patterns

**Browser structure.** Setups contain operations and folders; NC Programs sit in their own node.
New operations go into the *active* setup or folder (radio button); right-click a folder and choose
New Operation to put one there directly [FOLDERS-REF].

**Status icons** [ICONS]: error; child has error; warning; child has warning; out of date; valid
(toolpath and in-process stock both generated); has notes; queued; generating; toolpath done but
IPS off or pending; protected; modified (for example trimmed); optional; suppressed; IPS turned
off. A parent shows its worst child state in the order error, then out of date, then warning.
Hovering shows the message without opening the log [ICONS]. Warnings and errors live in a per
operation log (Show Log) [WARN].

**Operation states set from the context menu:**

| State | Effect | Source | forgeCAD |
|---|---|---|---|
| Suppress | ignored by generate, simulate, post; toolpath cleared | [SUPPRESS] | must-have |
| Protect | not invalidated by model edits; for parts already cut, so later rest work still sees them | [PROTECT] | differentiator (pinning a toolpath to a file `version` fits forgeCAD's edit model) |
| Optional | post may mark the block skippable (block delete), if the post supports it | [OPTIONAL] | later |
| Copy / Duplicate / Create Derived Operation | reuse settings; derived keeps a link to its parent | [DUP] | must-have (duplicate); derived later |
| Compare and Edit | grid of all parameters of several ops or setups; filters (different only, non-default, warnings, editable, modified); colour coding; tools not editable here | [COMPARE] | differentiator (forgeCAD's `diff` already does this in text) |

**Folders.** Folders group operations (for example roughing, semi-finishing, finishing) and do not
change them. Folders nest [FOLDERS-REF] [FOLDERS]. *forgeCAD: must-have and cheap.*

**Toolpath patterns** [PATTERN] [PATTERN-REF]. Select operations, right-click, *Add to New
Pattern*. A pattern folder is created. Types: Linear, Circular, Mirror, Duplication, Component
(assemblies only). Patterns nest. Ordering inside a pattern: preserve order (each instance
complete), by operation, by tool. A pattern folder can override the WCS offset. Only selecting the
pattern folder itself simulates or posts the copies; selecting the inner operations ignores the
pattern. Changes to a pattern apply with no toolpath recompute. *forgeCAD: must-have; it maps
directly onto the existing `pattern(of: ..., layout: lin(...))` feature.*

### 1.5 Templates

- **Store as Template**: select one or more operations, right-click, give name, description and a
  location (Local by default; Cloud or subfolders optional). Several operations become one
  template [TPL-STORE] [TPL-REF].
- **Create From Template**: right-click a setup, or open *Manage > Template Library*, choose and
  press Create Operations; the new operations land in the active setup [TPL-SEL] [TPL-REF].
- **Template Library**: Fusion Library (read-only, includes hole-drilling and additive support
  templates), Local, Cloud (must be enabled in preferences); filters; import and export as
  `*.f3dhsm-template` [TPL-LIB] [TPL-SEL].
- Templates store every value **as an expression**, so values like stepover follow the new tool
  [OPTEMPL].

*forgeCAD: differentiator. A template is just a text fragment of operations with tool-relative
expressions; the `.fg` language already has params and expressions.*

**Parameter defaults and expressions.** Most parameters default to expressions of other
parameters (for example a fraction of tool diameter). Per field: Save as User Default, Reset to
User Default, Reset to System Default, Edit Expression (F2), Copy Parameter Name. Defaults export
and import as `*.f3dhsm-defaults` [DEFAULTS]. *forgeCAD: must-have (tool-relative defaults); a
differentiator if defaults live as plain text.*

### 1.6 Toolpath editing

**Manual NC** inserts machine actions between operations; the post must support each one. Types:
comment, stop (M0), optional stop (M1), dwell, tool break control, measure tool, chip conveyor
start and stop, door open and close, calibrate, verify, clean, generic action, print message,
display message, alarm, alert, pass-through of literal code (flagged as unsafe), force tool change,
call subprogram [MANUALNC] [MANUALNC-REF]. *forgeCAD: must-have for comment, stop, dwell, force
tool change; the rest later.*

**Toolpath modifications** (all need the Manufacturing Extension) [MODIFY] [EXT]:

| Modification | What it does | Source |
|---|---|---|
| Trim | cut a toolpath with a polygon, plane or boundary; keep one side or both (split); new links are gouge-checked | [TRIM] |
| Delete passes or segments | remove picked passes (a pass is cutting motion between leads/links; made of segments) | [DELETE] |
| Replace tool | swap the tool without recompute; allowed changes are feeds/speeds and non-tip geometry (lengths) | [REPLACE] |
| Move entry positions | move entry and exit on closed passes, for example stagger them so marks do not line up | [ENTRY] |
| Leads and links | re-do heights, linking and leads on an existing path without recompute | [LEADS] |

Not modifiable: turning, 2D Adaptive, 2D Offset Roughing, Slot, Thread, Bore, Circular, 2D Chamfer,
Drill, wrapped toolpaths, and in-control compensation. Move start points also excludes open-pass
strategies (Face, Parallel, Ramp, Radial, Morphed Spiral, Spiral unless in concentric circles mode).
3D Adaptive and 3D Offset can be modified after a preference switch. The help warns new links
avoid plunging into stock but may be inefficient, and that the leads-and-links modification
checks only the model, not the stock [MODIFY]. *forgeCAD: later; a differentiator would be to store
each modification as a typed op over the toolpath so edits stay replayable.*

### 1.7 Regenerate, out of date, in-process stock

- Operations generate automatically on creation. A change that affects an operation marks it out
  of date; *Actions > Generate* on a selection, setup or folder brings it back [GEN-OV] [GEN].
- **Automatic IPS** computes stock simulation after each toolpath, so selecting any operation shows
  the stock as it stands after it. It is an on/off switch with no dialog. With it off, simulating
  one operation warns that in-process stock is missing and may be wrong [IPS].
- **CAM Task Manager** and **Machining Time** (feed time, rapid time, tool-change time, feed scale
  what-if) are separate tools [TOC] [MTIME].

*forgeCAD: must-have. forgeCAD's build/check, per-feature status and `--at FEAT` already give
exactly this shape; IPS per operation is the analogue of "the part as it stood after that feature".
That is a differentiator.*

### 1.8 NC Programs and posting

An NC Program is a saved group of operations (from one or many setups) plus a post and its
settings. Simulation and setup sheets can also run on it [NCP-OV].

- **Settings tab** [NCP-REF] [NCP-STEPS]: use the machine's configured post, or pick a post;
  optional *cascading post* (second post run after the first, for verification tools); program
  name/number; file name (can be an expression); comment; output folder; *Post to Fusion Hub*; NC
  extension; output unit (inch, mm, document); open in editor; **post properties** (post-specific:
  home positioning, tool-change positions, block numbering, header, code formatting, limits).
- **Operations tab**: tick setups or single operations; *Reorder to minimise tool changes*
  [NCP-STEPS].
- **Combining setups**: one program file can hold several setups (for example vise 1 at G54 and
  vise 2 at G55). With reordering on, one tool finishes its work in both vises before the next tool
  change; the right offset is emitted per operation [NCP-GROUPS].
- OK saves the program without posting; Post saves and posts. Right-click to re-post, simulate,
  open folder, make a setup sheet, duplicate, edit [NCP-STEPS].
- Posts are JavaScript `.cps` files: personal, installed (overwritten on upgrade) and cloud posts;
  an online Post Library [POST-OV].

*forgeCAD: must-have (one post, program grouping, reorder by tool); a differentiator is a typed,
diffable post configuration and a built-in G-code reader that re-simulates the output.*

### 1.9 Setup sheets

Actions > Setup Sheet on selected setups, folders, operations or an NC program writes an **HTML**
document and opens it in the browser [SHEET-STEPS]. Contents: a screenshot of the current view, WCS
info, stock size and orientation, finished part size, tool count, max Z travel, cutting and rapid
distance, estimated cycle time, and per-operation detail [SHEET-OV]. From an NC program the sheet
can be published to a hub folder and has selectable configurations; sections are Summary, Tool,
Setup and Operations [SHEET-NCP]. An Excel variant exists [TOC]. *forgeCAD: later; trivially
generated from the typed reply JSON.*

### 1.10 Tool library in the document

Libraries: **Documents** (tools in the open design; picking any tool from another library copies
it here), Hub Libraries (replacing Cloud), Cloud, Local, Fusion Library (read-only, standards based)
and Vendor libraries (read-only, rolling out) [TOOL-TYPES]. Tools carry several *cutting data
presets* (feeds and speeds); holders are separate items; custom tools only in Documents, Local and
Cloud [TOOL-LIB]. *forgeCAD: must-have (tools stored in the part file, copied in, like the Documents
library).*

### 1.11 Simulation entry

*Actions > Simulate* (or *Simulate with Machine*) on a setup or selected operations opens a
simulation environment [SIM-NOM] [SIM]. Two halves: **animation** (tool and stock playback) and
**verification** (background collision checks). By default collisions are not checked while
animating, only during verification [SIM]. Without a machine: tool, stock and collisions only,
faster, motion between points may be wrong; GPU mode available for milling with some limits
[SIM-NOM]. With a machine: kinematics from the machine and post, axis overtravel, retract and
reconfigure events, rapid moves into stock, machine collisions [SIM]. Stock display: colour by
operation or by comparison with the model (leftover stock offset and tolerance; blue too little
removed, red too much, green within tolerance), accuracy slider, transparency, stop on collision
[SIM-STOCK]. Issues list with clickable timeline marks [SIM-ISSUES]. *forgeCAD: stock-only
simulation and gouge/compare colouring are must-have; machine simulation later.*

---

## 2. Parameter tabs shared by milling operations

Most milling dialogs have Tool, Geometry, Heights, Passes, Linking tabs (plus Multi-Axis and Post
Process where relevant) [2D-ADAPT-REF] [CONTOUR-REF] [OPS-OV].

### 2.1 Tool tab

| Parameter | Meaning | Source | forgeCAD |
|---|---|---|---|
| Tool | picked from a library; copied into the document | [WF] [TOOL-TYPES] | must-have |
| Coolant | list of coolant modes; whether each works depends on the post | [2D-ADAPT-REF] | must-have |
| Preset | fill feeds and speeds from the tool's cutting data | [DRILL-REF] | must-have |
| Spindle speed (rpm) / Surface speed (SFM or m/min) | linked pair | [2D-ADAPT-REF] | must-have |
| Ramp spindle speed | separate rpm for ramping | [2D-ADAPT-REF] | later |
| Cutting feedrate / Feed per tooth | linked pair | [2D-ADAPT-REF] | must-have |
| Lead-in feedrate, Lead-out feedrate | feeds for leads | [2D-ADAPT-REF] | must-have |
| Ramp feedrate, Plunge feedrate, Feed per revolution (plunge) | entry feeds | [2D-ADAPT-REF] | must-have |
| Retract feedrate (drilling) | feed out when not rapid | [DRILL-REF] | must-have |
| Shaft and holder mode, shaft/holder clearance, use flute, check spindle/table | collision avoidance against holder; for drill: trim, detect tool length, fail, skip colliding holes | [DRILL-REF] [TOOLTAB] | differentiator (exact-kernel holder checks) |
| Tool orientation | per-operation override of WCS orientation and origin, same choices as setup (plus *Setup WCS*) | [2D-ADAPT-REF] | must-have (3+2) |
| Machining type | 3-axis, 5-axis, Wrap, 4-axis, Polar, depending on strategy (multi-axis needs the extension) | [MACH-TYPE] | later |

### 2.2 Geometry tab

| Parameter | Meaning | Source | forgeCAD |
|---|---|---|---|
| Pocket / contour selection | faces, edges or sketches; closed pocket, open pocket, or standing boss | [2D-ADAPT-REF] [CONTOUR-REF] | must-have |
| Extension method (open pockets) | how open chains are closed at the stock: tangent (default), closest boundary, parallel | [2D-ADAPT-REF] | must-have |
| Stock contours / stock selections | cut out to the stock box or a picked boundary; tool may approach from outside; not a containment | [2D-ADAPT-REF] [FACE-REF] | must-have |
| Tangential extension, separate end extension | lengthen open contours at start/end | [CONTOUR-REF] | must-have |
| Machining boundary (3D) | none, bounding box, silhouette, or selection | [3D-ADAPT-REF] [RADIAL-REF] | must-have |
| Tool containment (3D) | tool inside, centre on, or outside the boundary, plus additional offset | [3D-ADAPT-REF] | must-have |
| Contact point boundary / contact only | limit by where the tool touches, not its centre; only cut where in contact | [GEO] [PARALLEL-REF] | later |
| Slope from/to angle | restrict to a slope range | [PARALLEL-REF] | later |
| Rest machining (2D) | manual: give the previous tool's diameter, corner radius, taper, shoulder length; 2D ops are not stock aware | [2D-ADAPT-REF] [CONTOUR-REF] [S: forum] | must-have |
| Rest machining (3D) source | from previous operations, from chosen operations, from tool (manual), from file, from bodies/solids, from setup stock; union of dependent ops; include all previous; ignore stock less than | [3D-POCKET-REF] [3D-ADAPT-REF] | must-have (from previous ops) |
| Include setup model | operation sees the setup model plus picked surfaces | [GEO] | must-have |
| Surface manager | mark surfaces to machine, avoid, ignore, or treat as fixture | [DRILL-REF] [TOC] | later |
| Tabs (2D Contour) | rectangular or triangular, width, height, by distance or count, manual | [CONTOUR-REF] | must-have (router users) |

### 2.3 Heights tab

Five heights, each set **From** a reference plus an **Offset** (positive or negative) [HEIGHTS]
[CLEAR] [RETRACT] [FEEDH] [TOPH] [BOTH]:

| Height | Role | Preview colour |
|---|---|---|
| Clearance | first height the tool rapids to on the way to the toolpath | orange |
| Retract | height it moves up to before the next pass; above feed and top | olive green |
| Feed | rapid stops here and feed begins; also peck retract in drilling; can be *Disabled* (rapid straight to the lead-in) | green |
| Top | top of the cut | light blue |
| Bottom | final depth | dark blue |

**From** references: the other heights, model top/bottom, stock top/bottom, a selection (vertex,
edge, face), origin (absolute). Drill adds hole top, hole bottom, hole segment top/bottom; bottom
height adds *to chamfer width* and *to chamfer diameter* for spot/chamfer drills [CLEAR] [BOTH].
**Highest of / Lowest of** picks automatically from stock, model and fixture top/bottom
[AUTO-HEIGHT]. Heights can be dragged on the canvas [HEIGHTS]. For 4/5-axis, **clearance
geometry** can be a plane, cylinder, sphere or (5-axis) box, with direction and origin choices
[CLEAR-GEO].

*forgeCAD: must-have, and a differentiator: heights `from:` an anchor like `base.end` or
`hole.bore` with an offset is exactly forgeCAD's selection model.*

### 2.4 Passes tab

| Parameter | Meaning | Source | forgeCAD |
|---|---|---|---|
| Tolerance | max chord error when turning curves into lines; tight tolerance means big files and possible data starving on slow controls | [2D-ADAPT-REF] | must-have |
| Smoothing | fit arcs and merge collinear moves within tolerance (G2/G3); off by default; Steep and Shallow uses its own arc fitting and even spacing | [SMOOTH] | must-have |
| Feed optimisation | slow down at corners: max direction change, reduced feed radius, reduced feed distance, reduced feedrate, inner corners only | [2D-ADAPT-REF] | later |
| Stock to leave, radial (wall) and axial (floor) | may be negative (electrodes, spark gap; negative radial must be under tool radius); radial value copies to axial unless set | [2D-ADAPT-REF] | must-have |
| Multiple depths | max stepdown, even stepdowns, finishing stepdowns and finishing stepdown size, order by depth / by area / by islands / by step | [CONTOUR-REF] [2D-ADAPT-REF] | must-have |
| Finishing passes | number, stepover, finish feedrate, leads on every pass, repeat final pass (deflection), finishing overlap, only at final depth, rough final | [CONTOUR-REF] | must-have |
| Direction | climb, conventional; for some ops both ways (with separate load and feed for the other way) | [2D-ADAPT-REF] [FACE-REF] | must-have |
| Sideways compensation | left (climb) or right (conventional) | [CONTOUR-REF] | must-have |
| Minimum cutting radius | blend inside corners so the tool is not buried (leaves rest material) | [2D-ADAPT-REF] | must-have |
| Preserve order | machine features in picking order instead of optimised order | [CONTOUR-REF] | must-have |
| Wall taper angle | tapered walls from a 2D strategy | [CONTOUR-REF] | later |
| Thin wall | cut both sides of walls up to a width together | [CONTOUR-REF] | later |
| Fillets (3D) | round sharp toolpath corners with a radius | [3D-ADAPT-REF] | later |
| Cusp height | derive stepover and stepdown from target scallop height | [CUSP] | must-have for 3D finishing |
| Flat area detection | adjust stepdowns to hit flats so floor stock to leave is exact | [FLAT-DET] | must-have |

### 2.5 Linking tab

| Parameter | Meaning | Source | forgeCAD |
|---|---|---|---|
| High feedrate mode | which rapids stay G0: all, axial and radial only, axial only, radial only, single-axis only, or always G1 at the high feedrate (for machines that dog-leg at rapid) | [2D-ADAPT-REF] | must-have |
| High feedrate | the G1 feed used in place of rapids | [2D-ADAPT-REF] | must-have |
| Allow rapid retract | retract at G0 or at lead-out feed | [2D-ADAPT-REF] | must-have |
| Retraction policy | full retract (to retract height), minimum retract (just clear the part), shortest path | [2D-ADAPT-REF] [3D-ADAPT-REF] | must-have |
| Safe distance | minimum clearance during retracts (after stock to leave) | [CONTOUR-REF] | must-have |
| Keep tool down / max stay-down distance / min stay-down distance / stay-down level | avoid retracts over short gaps; some 2D ops need Keep Tool Down ticked; some ops have no such option | [STAYDOWN] [2D-ADAPT-REF] | must-have |
| Lift height | lift slightly off the floor on stay-down moves (Offset Roughing, Adaptive, Horizontal) | [LIFT] | later |
| No-engagement feedrate | feed for moves not cutting but not retracted (Adaptive) | [2D-ADAPT-REF] | later |
| Leads | lead-in/out on/off, horizontal radius, sweep angle, linear distance (where cutter compensation switches on/off), perpendicular, vertical radius, same as lead-in | [CONTOUR-REF] | must-have |
| Ramp type | plunge outside stock, zig-zag, predrill (needs predrill points), profile, plunge, smooth profile, helix | [2D-ADAPT-REF] | must-have (helix, zig-zag, plunge, predrill) |
| Ramp details | ramping angle, ramp taper angle (conical helix), max ramp stepdown per rev, ramp clearance height, helical ramp diameter, minimum ramp diameter, ramp radial clearance | [2D-ADAPT-REF] [3D-FLAT-REF] | must-have |
| Entry positions, predrill positions | picked points near which to enter, or pre-drilled holes | [2D-ADAPT-REF] | must-have |
| Transition type | no contact (retract), straight line, shortest path, smooth (arcs, limited by ramp angle) | [TRANS] | later |
| Extend before retract | run passes past the stock before lifting (Face) | [FACE-REF] | later |

The Linking tab is also where ramping failures surface: the help's example is a minimum ramp
diameter larger than the room available, which leaves areas uncut with a log warning [WARN].
*forgeCAD differentiator: turn that warning into a typed error with a `fix` (largest ramp diameter
that fits), the way a too-big fillet radius already comes back with a SetParam fix.*

---

## 3. 2D milling strategies

Current 2D menu, from the help TOC: 2D Adaptive Roughing, 2D Offset Roughing, Face, 2D Contour, Slot,
Trace, Thread, Bore, Circular, Engrave, 2D Chamfer; plus drilling: Drill and Hole Recognition [TOC]
[2D-OV] [DRILL-OV].

### 3.1 Face

Usually the first operation: flattens the stock top down to the model top so the part sits flat.
Assumes the whole stock by default; stock contours or sketch/edge selections limit it in XY,
heights in Z [FACE] [FACE-REF]. Specific parameters: *pass direction* (angle), *pass extension*
beyond the boundary, *stock offset* (grow the stock contour outwards), *stepover* (default about
95% of diameter less corner radius), direction (both ways default, climb, conventional), *from other
side*, *use chip thinning*, multiple depths with *both sides* (alternate start side per depth),
finishing step and finishing stepdown and finish feed, *extend before retract*, transition type
(smooth, straight, short, no contact) [FACE-REF]. *forgeCAD: must-have.*

### 3.2 2D Adaptive Roughing (formerly 2D Adaptive Clearing)

Constant-engagement roughing; avoids sudden direction changes and conventional cuts; clears
cavities, open pockets and around bosses; can approach from outside the stock. Help prefers it for
simple pockets and 3D Adaptive for complex shapes [2D-ADAPT] [2D-ADAPT-REF].

- **Optimal load**: maximum engagement held; acts like stepover but varied to avoid overload. Help
  claims about 40% faster removal than legacy pocketing and much deeper stepdowns.
- **Minimum cutting radius**: blends inside corners (leaves rest material).
- **Use slot clearing / slot clearing width**: cut a slot down the middle first, then spiral out.
- Both ways with *optimal load other way* and *other way feedrate*.
- Multiple depths, even stepdowns, order by depth or by area, stock to leave, smoothing, feed
  optimisation.
- Linking: stay-down level, lift height, no-engagement feed, full or minimum retract, ramp types,
  helical ramp diameter, minimum ramp diameter.
- **Flat area detection** belongs to the 3D roughers and 3D Contour, not to 2D Adaptive [FLAT-DET].
- Subpages: open pocket machining, facing with 2D Adaptive [TOC].

*forgeCAD: must-have; the adaptive clearing engine is also the hardest single algorithm here.*

### 3.3 2D Offset Roughing (formerly 2D Pocket)

Offset-of-boundary clearing with optional finishing pass; stays inside its boundary (unlike
adaptive); smaller programs, better for old controls with little memory [2D-POCKET]. Specific
parameters: maximum stepover, *allow stepover cusps* (otherwise stepover is reduced so a radiused
tool leaves no floor cusps), *use morphed spiral machining* (one continuous spiral), smoothing
deviation, finishing passes and overlap, wall taper angle, minimum cutting radius, rest machining,
plus all ramp types [2D-POCKET-REF]. Subpages: circular pocketing, rest roughing [TOC].
*forgeCAD: must-have (simple and robust; first roughing to ship).*

### 3.4 Slot

For straight, arc or curved slots, closed or open on one end, of constant width; tool no wider than
the slot [SLOT]. Parameters: *backoff distance* (move off the wall at the end before retracting),
*repeat finishing pass*, multiple depths, tangential fragment extension, ramp type, predrill and
entry positions, preserve order, safe distance [SLOT-REF] [GEO]. *forgeCAD: later (2D Contour or
Adaptive covers most slots).*

### 3.5 2D Contour

Profile along a closed or open chain, inside or outside, always in a plane; usually finishing;
supports tabs; tapered walls via wall taper angle; 3D Contour is for sloped and organic walls
[CONTOUR] [CONTOUR-REF].

- **Compensation type**: *In computer* (Fusion offsets the path; no G41/G42); *In control* (outputs
  G41/G42 on the programmed contour, operator sets the offset); *Wear* (computed offset **and**
  G41/G42, so the operator enters the wear difference as a negative value); *Inverse wear* (same,
  wear entered as positive). Control compensation applies only to finishing passes. Modifications
  are not allowed on in-control, wear or inverse wear paths [CONTOUR-REF] [MODIFY].
- **Compensation radius allowance**: range of tool radii (from the tool radius up to radius plus
  allowance) for which the path stays safe [CONTOUR-REF].
- **Outer corner mode** (in-computer only): roll around the corner, keep sharp corner, keep sharp
  corner with loop [CONTOUR-REF].
- **Roughing passes**: max stepover, number of stepovers; finishing passes as in 2.4; finishing
  smoothing deviation; lead end distance (start lead-out feed before the end).
- **Chamfer mode**: only with a chamfer tool. Pick sharp edges (give chamfer width) or the bottom
  edge of a modelled chamfer (width computed, optional extra width); *chamfer tip offset* moves
  the contact up the flank [CONTOUR-REF].
- **Tabs**: shape, width, height, by distance or count, manual placement [CONTOUR-REF] [TABS].
- **Thin wall** with thin wall width; order by islands / by step; both ways (only affects depth
  passes on one open contour) [CONTOUR-REF].

*forgeCAD: must-have. Compensation types and tabs are table stakes for real shops and routers.*

### 3.6 Trace

Follows edges or sketch curves, including 3D curves, on the tool centre (sketch) or with left/right
compensation (edges); good for single-line text; smaller code than Engrave [TRACE]. Parameters:
*axial offset* (shift along the tool axis), sideways compensation, multiple depths (max stepdown,
number of stepdowns), repeat passes, pass extension, *maximum angle*, up/down milling (split each
pass into only-down or only-up parts), chamfer width/tip offset options, keep tool down
[TRACE-REF] [GEO]. *forgeCAD: must-have (cheap; covers grooves, engraving, deburr by hand).*

### 3.7 Thread (thread milling)

Helical thread milling in holes or on bosses, straight or tapered; heights come from the picked
circular or conical face [THREAD]. Parameters: *definition method* (manual, or automatic from
Fusion-modelled threads), threading hand (right/left), *thread pitch*, *pitch diameter offset*
(thread depth, needed when the hole is modelled at minor or the boss at major size), *do multiple
threads* / number of threads (multi-start), multiple passes / number of stepovers, repeat passes,
compensation type, helical leads, lead to centre, linear lead length. Selection helpers: same
diameter, holes only/bosses only, diameter range, same depth, same top Z, containment boundary
[THREAD] [THREAD-REF]. *forgeCAD: differentiator; forgeCAD already models threads exactly
(`research/thread-math/`), so automatic definition from the model is natural.*

### 3.8 Bore

Helical milling of holes or bosses with straight or tapered walls; heights from the face; usually
faster than Circular [BORE]. Parameters: pitch/ramp angle (*use ramp angle*), finishing passes,
multiple passes with stepovers, repeat passes, compensation type, lead to centre, linear lead
length, same selection helpers as Thread [BORE-REF]. *forgeCAD: must-have (holes larger than any
drill).*

### 3.9 Circular

Cylindrical holes and bosses only; ramps on and off each pass around the profile; smaller code than
Bore and uses more flute length [CIRCULAR]. Parameters: multiple depths with max stepdown and even
stepdowns, multiple passes with *XY stepover*, repeat passes, compensation type, lead to centre,
selection helpers [CIRCULAR-REF]. *forgeCAD: later (Bore covers it).*

### 3.10 Engrave

V-carve style engraving with a pointed chamfer tool: finds the medial line of the picked region and
moves the tool up and down as the width changes; makes sharp internal corners by running into
corner intersections unless the corner angle exceeds *sharp corner angle* [ENGRAVE]
[ENGRAVE-REF]. Other parameters: contour selections, multiple depths, keep tool down, entry
positions. *forgeCAD: later; it needs a medial-axis routine, which forgeCAD does not have.*

### 3.11 2D Chamfer

Bevels edges with a tapered tool, on modelled or unmodelled chamfers [CHAMFER]. Parameters:
*chamfer width*, *chamfer tip offset* (move contact off the tool tip for life), *chamfer clearance*
(keep away from model geometry not being chamfered), include setup model, compensation type,
finishing overlap, tangential extensions, leads [CHAMFER-REF] [GEO]. 2D Contour can also chamfer
(pick the bottom edge of a modelled chamfer with width 0) [CHAMFER]. *forgeCAD: must-have; edge
selection by `edges(of: base, crease: "convex")` is a clean fit.*

### 3.12 Drill and its cycles

Usually triggers the control's canned cycles; works in milling and turning; supports multi-axis
types Polar, 4-axis, 5-axis, which output expanded moves, so tapping, threading and back boring are
not yet allowed in multi-axis [DRILL] [DRILL-REF].

**Cycle types** (Cycle tab) [DRILL-REF] [DRILL-CYC]:

| Cycle | Typical code | Motion and extra parameters | forgeCAD |
|---|---|---|---|
| Drilling, rapid out | G81 | feed to depth, rapid out; spot and holes under ~3xD | must-have |
| Counterboring, dwell and rapid out | G82 | dwell at bottom (seconds; post usually outputs ms; at least 60/rpm for one rev) | must-have |
| Chip breaking, partial retract | G73 | peck depth, peck reduction per peck, minimum peck, accumulated depth before full retract, chip break distance, dwell before retract | must-have |
| Deep drilling, full retract | G83 | same pecking parameters, retracts to R plane each peck | must-have |
| Break through | expanded | break-through distance from bottom, reduced feed / feed per rev / rpm | later |
| Guided deep drilling (gun drill) | expanded | start depth in pilot hole, positioning rpm and feed, stop spindle, dwelling depth; >20xD holes | later |
| Tapping | G84/G74 | synchronous feed; hand taken from the tool | must-have |
| Left tapping / Right tapping | G74 / G84 | explicit hand | must-have |
| Tapping with chip breaking | control-specific | partial screw-in, reverse, repeat | later |
| Reaming, feed out | G85 | feed in, feed out | must-have |
| Boring, dwell and feed out | G89 | dwell, feed out | later |
| Stop boring | G86 | spindle stop at bottom, rapid out | later |
| Fine boring, shift | G76 | stop, orient, shift off wall, rapid out | later |
| Back boring | G87 | from the far side with a reverse tool; rarely used | later |
| Circular pocket milling | control cycle | incremental depth, direction, use hole diameter, stepover, repeat pass, stock to leave, multiple steps, cycle diameter; nothing drawn on screen | later |
| Bore milling | control cycle | pitch plus the above | later |
| Thread milling | control cycle | pitch, hand, cycle diameter | later |
| Custom / Probe | control cycle | parameters for a machine probing macro | later |

*Rigid tapping* is not a separate cycle in the current help; tapping is described as synchronous.
In many Fusion posts rigid versus floating tapping is a post property [K, not re-read].

**Hole selection** [DRILL-REF] [DRILL]: selection mode *Faces* (cylinders or hole chamfers; keeps
depth and is associative), *Points* (hole edges, sketch points or circles; XY and start Z only),
*Diameter range* (min/max, no picks), *Remaining holes from operation* (fall-back holes another
op skipped or trimmed for collisions). Options: select same diameter (associative: new holes of
that size are picked up on regenerate), check for occlusions, containment boundaries (nested to
include or exclude), *auto-merge hole segments* (start above counterbores), *drill tip through
bottom* (depth to full diameter, for through holes), reset directions.

**Hole ordering**: order selected, optimised, inside to outside, by X, by Y (X/Y only in 3-axis),
order by depth (high to low or reverse), reverse order [DRILL-REF].

**Drill linking**: ignore gouges (fast first pass), retraction policy *avoid collisions* or
*preserve cycles* (keep cycle heights and warn), high feedrate mode, safe distance [DRILL-REF].
Shaft and holder mode for drilling: trimmed, detect tool length, fail on collision, skip colliding
holes [DRILL-REF] [DRILL-COLL].

**Hole Recognition** (extension): scans for cylinders on any plane, assigns each an action (spot,
drill, counterbore, tap and so on) from a hole template, and creates the operations; includes
partial holes [HOLEREC] [EXT]. *forgeCAD: differentiator. forgeCAD's `hole` feature already
knows its own kind, diameter and depth, so a hole can carry its machining intent with no
recognition step.*

---

## 4. 3D milling strategies

Current 3D menu from the TOC: 3D Adaptive Roughing, 3D Offset Roughing, 3+2 Roughing, Steep and
Shallow, Flat, Parallel, Scallop, Contour, Ramp, Pencil, Horizontal, Spiral, Radial, Morphed
Spiral, Project, Blend, Morph, Corner, Flow (with Flow (Old) behind a preference), Deburr, Geodesic
[TOC] [3D-OV]. Roughers are stock aware; finishers are not [3D-OV].

| Strategy | What it does | Specific parameters | Extension | forgeCAD |
|---|---|---|---|---|
| 3D Adaptive Roughing | constant-engagement Z-level roughing; preferred 3D rougher; climb only | optimal load, min cutting radius, *machine cavities*, slot clearing, max roughing stepdown, *fine stepdown* (upward intermediate steps), flat area detection, machine shallow areas (min shallow stepdown, max shallow stepover), order by depth/area, fillets, stay-down and min stay-down clearance, reduce air cutting [3D-ADAPT] [3D-ADAPT-REF] | no | must-have |
| 3D Offset Roughing | Z-level roughing by offsetting the boundary; constant stepover; may cut both directions | manual stepover (max/min), min diameter, morphed spiral, allow stepover cusps, smoothing deviation, fine stepdown, flat area detection, machine shallow areas, lift height, ramp types [3D-POCKET] [3D-POCKET-REF] | no | must-have |
| 3+2 Roughing | roughs from several fixed tilted orientations in one op; styles offset, parallel, adaptive; automatic orientation choice [3P2] | | yes | later |
| Steep and Shallow | hybrid: contour passes on steep areas, scallop or parallel on shallow, split by *threshold angle* measured from horizontal; overlap distance, wall clearance, smooth offsets, remove cusps at junctions, continuous passes, safer ordering on steep walls, avoid flat areas, parallel-pass optimisation, undercut machining, own smoothing (fit arcs, even points) [STEEP] [STEEP-REF] [SMOOTH] | yes | later (Contour plus Scallop separately cover it) |
| Flat | finishes only flat areas normal to the tool; types offset, parallel, profile; better flat detection and fillets than Horizontal | optimise open pockets, spiral, pass direction, stepover, smoothing deviation, profile cut direction, finishing pass, multiple depths, machine over/into holes with max hole size, rest machining with source (previous ops, bodies, setup stock), *detection limit*, *extension*; inclined flats via Multi-Axis tab (extension) [FLAT] [FLAT-REF] [FLAT-REST] [FLAT-INCL] | base; inclined: yes | must-have |
| Parallel | raster rows in XY projected on the surface; best on shallow areas | pass direction, stepover, horizontal stepover, one way / both ways / other way, *machine steep areas* with *add perpendicular passes*, min steep stepover, max steep stepdown, simple ordering, up/down milling, slope limits [PARALLEL] [PARALLEL-REF] | no | must-have |
| Scallop | constant-stepover offsets that follow walls (3D offset) | inside/outside direction, link inside to outside, limit number of stepovers, stepover, *machine areas using boundaries*, boundary overlap, up/down milling [SCALLOP] [SCALLOP-REF] | no | must-have |
| Contour | waterline Z-levels; best on steep walls | min diameter, min cutting radius, max stepdown, machine shallow areas, flat area detection, order bottom-up, order by islands, thin wall, repeat finishing pass, smooth ramps, multi-axis tilting (extension) [CONTOUR3D] [CONTOUR3D-REF] | base; multi-axis: yes | must-have |
| Ramp | helical descent around the walls so the tool never leaves the cut; good for brittle materials | max stepdown, min diameter, min cutting radius, machine shallow areas, order bottom-up, contact point boundary [RAMP] [RAMP-REF] | no | later |
| Horizontal | finds flat areas and clears them with offset passes (older than Flat) | stepover, morphed spiral, allow cusps, multiple depths, lift height, allow plunging outside stock [HORIZ] [HORIZ-REF] | no | later (Flat supersedes) |
| Pencil | runs along internal corners and fillets at or below the tool's corner radius; optional extra parallel passes | *bitangency angle* (detects wall/fold passes), *overthickness* (inflate tool to catch larger fillets), number of passes, stepover, inside/outside order [PENCIL] [PENCIL-REF] | no | must-have |
| Spiral | spiral from a centre point projected down | spiral mode (spiral, spiral with circles, concentric circles), clockwise, inner and outer limit, stepover [SPIRAL] [SPIRAL-REF] | no | later |
| Radial | spokes from a centre projected down | angular step, angle from/to, inner/outer radius, min stepover, machine straight on [RADIAL] [RADIAL-REF] | no | later |
| Morphed Spiral | spiral morphed from the boundary shape, projected; organic shallow surfaces | stepover, clockwise, inside/outside [MSPIRAL] [MSPIRAL-REF] | no | later |
| Project | projects curves onto the surfaces; tool centre follows them | axial offset, multiple depths, rest material resolution/overlap/stock to leave [PROJECT] [PROJECT-REF] | no | later (Trace on surface) |
| Blend | finishing between a start and end drive curve over drive surfaces, optional intermediate curves; can detect undercuts | offset passes using surface boundary or curves, stepover from tool tip or on surface, entry and exit passes with heights; 5-axis option [BLEND] [BLEND-STEPS] | base; multi-axis: yes | later |
| Morph | passes morphed between two or more chained curves | morph mode, contour offset, use stepover or number of passes, number of offset stepovers, pass extension [MORPH] [MORPH-REF] | no | later |
| Corner | rest finishing of corners the *reference tool* could not reach; steep and shallow passes split by threshold angle | reference tool (larger than the cutting tool), detection limit, steep and shallow pass types: along with max stepover, along with constant stepover, across; priority; cusp height [CORNER] [CORNER-CHOICE] [CORNER-STEPS] | yes | must-have (as plain rest finishing) |
| Flow | drives the tool along U or V of the picked faces; best on curved surfaces; auto-aligns faces | stepover, cusp height, tangential fragment extension, multi-axis (extension) [FLOW] [FLOW-STEPS]; Flow (Old) by preference, with isometric direction and number of stepovers [FLOW-OLD] | base; multi-axis: yes | differentiator (forgeCAD owns the exact NURBS/UV parameterisation) |
| Deburr | deburrs sharp or rounded outer edges as chamfer or fillet; strips redundant points | edges to deburr (auto rules), min edge angle, exclude circular holes in a diameter range, exclude vertical edges, include rounded edges, edges to exclude, deburr by chamfer or fillet (fillet needs multiple passes), edge shape, unreachable corners, add loops, 3/4/5-axis, cut at fixed point on tool [DEBURR] [DEBURR-STEPS] | yes | differentiator (B-rep edge knowledge; `edges(crease: "convex")`) |
| Geodesic | finishing of free-form surfaces and undercuts with passes laid out by true surface distance | pass type blend or scallop; offset from machining boundary, drive-surface boundary, centre circle/line, or picked curves; smooth stepovers; straighten cuts on boundaries; ball or lollipop tool; 3/4/5-axis [GEO-OV] [GEO-STEPS] | base; multi-axis: yes | later |

**Rest machining options in 3D** (Geometry tab, Rest Machining group) [3D-POCKET-REF]
[3D-ADAPT-REF] [PENCIL-REF] [FLAT-REST]:

| Source | Meaning |
|---|---|
| From previous operation(s) | rest stock from all earlier operations in the setup (the usual case; uses IPS) |
| From operation(s) | from chosen operations only |
| From tool | manual: diameter, corner radius, taper angle, shoulder length of the earlier tool |
| From file | a rest-material file |
| From solid(s) / From bodies | bodies modelled as the remaining stock |
| From setup stock | the setup stock |
| Union of all dependent operations / Include all previous operations | how operation-based sources combine |
| Ignore stock less than | skip thin slivers of rest material |
| Detection limit, extension (Flat) | ignore small areas (counted on top of stock to leave); overlap neighbouring regions |
| Reference tool (Corner) | the larger tool whose reach defines what is left |

2D strategies are not stock aware; their rest machining is the manual "previous tool" form only
[2D-ADAPT-REF] [S: forum reply in search results]. *forgeCAD: "from previous operations" is the
must-have, and it falls out of keeping an in-process stock body per operation.*

**Multi-axis strategies** (all extension: Rotary Roughing, Swarf, Advanced Swarf, Multi-Axis
Roughing, Multi-Axis Contour, Rotary Parallel, Rotary Contour, Multi-Axis Finishing) are listed in
the TOC but were outside this note's scope [TOC] [EXT]. *forgeCAD: later.*

---

## 5. Summary ratings for forgeCAD

| Area | Must-have | Differentiator | Later |
|---|---|---|---|
| Workflow | setup (milling), WCS, model/fixtures, box/relative/solid stock, from preceding setup, folders, patterns, generate/out-of-date, NC program, one post, tool library in document, stock simulation | named WCS anchors; IPS per operation as `--at`; templates and defaults as text; Compare and Edit as `diff`; protect = pin to version; typed ramp/lead failures with fixes | turning, cutting, additive, probing, manual NC beyond basics, toolpath modifications, setup sheets, machine simulation |
| Shared tabs | Tool feeds/speeds, Geometry selection and containment, five Heights with From+Offset, tolerance, smoothing, stock to leave, multiple depths, finishing passes, direction, leads, ramps, retraction, high feedrate mode, stay-down | heights from model anchors; holder collision checks on the exact kernel | feed optimisation, transitions, lift height, slope limits, clearance geometry |
| 2D | Face, 2D Adaptive, 2D Offset Roughing, 2D Contour (with compensation types and tabs), Trace, Bore, 2D Chamfer, Drill (G81/82/73/83/84/74/85) | Thread from modelled threads; holes that carry machining intent | Slot, Circular, Engrave, advanced and control-side cycles, Hole Recognition |
| 3D | 3D Adaptive, 3D Offset, Flat, Parallel, Scallop, Contour, Pencil, Corner-style rest finishing | Flow and Deburr on exact B-rep | Steep and Shallow, Ramp, Horizontal, Spiral, Radial, Morphed Spiral, Project, Blend, Morph, Geodesic, 3+2 and multi-axis |

---

## Sources

All are Autodesk Fusion help pages, read [R] on 2026-11-23 at
`https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/<ID>.htm` unless a full URL is given.
The same pages show in the viewer at `https://help.autodesk.com/view/fusion360/ENU/?guid=<ID>`.

| Key | ID or URL | Title (paraphrased) |
|---|---|---|
| TOC | https://help.autodesk.com/view/fusion360/ENU/data/toctree.json | Full help table of contents |
| MILL-WF | MFG-OVERVIEW-MILLING-WORKFLOW | Workflow: milling |
| WF | MFG-OVERVIEW-MILLING-WORKFLOW | (same) |
| OPS-OV | MFG-OVERVIEW-PARAMETERS-OVERVIEW | Operation parameters overview |
| SETUP | MFG-CREATE-SETUP | Create a setup |
| SETUP-REF | GUID6614D39D-114D-424A-ABDF-F97BC6D5D88F | Setup reference |
| SETUP-MILL | MFG-REF-SETUP-MILL | Setup tab (milling and cutting) |
| STOCK | MFG-REF-SETUP-STOCK | Stock tab reference |
| BOUNDING | MFG-USE-BOUNDING-SOLID | Bounding solid as stock |
| POST-TAB | MFG-REF-SETUP-POSTPROCESS | Setup Post Process tab |
| REST-SETUPS | MFG-REST-MACHINING-SETUP-STOCK-TRANSFER | Rest machining between setups |
| CONT-REST | MFG-CONTINUE-REST-MACHINING | Continue rest machining |
| MACHINES | MFG-MACHINES | Machines |
| PROBE-OV | MFG-PROBE-WCS-OVERVIEW | Probe WCS strategy |
| PROBE-WF | MFG-HOW-TO-PROBE-WCS-WORKFLOW | Workflow: setting up a part with probing |
| PROBE-REF | GUID-61123A95-8828-4214-A8FE-5A4DD1C2EE4F | Probe WCS reference |
| FOLDERS | LP-READ-IRONFOLDER | Folders in setups |
| FOLDERS-REF | GUIDE8611C29-454F-4ED2-849A-5E4C55ADB45F | Group operations using folders |
| PATTERN | LP-READ-IRONPATTERN | Patterns for toolpaths |
| PATTERN-REF | GUIDE86746AC-03D4-4909-812D-128D855ED02C | New Pattern reference |
| ICONS | MFG-TOOLPATH-STATUS-ICONS | Browser status icons |
| WARN | Fusion_CAM_reference_cam_browser_operation_warnings_html | Warnings and errors |
| SUPPRESS | GUIDBF444CFD-62D5-4D2C-85DE-2A56DF0C7067 | Suppress an operation |
| PROTECT | GUIDD6031B4C-624B-4940-8477-347E7E193BC4 | Protect an operation |
| OPTIONAL | GUID3BDF7F3C-03E2-4842-8A6F-08969D29166C | Make an operation optional |
| DUP | GUID10089C3E-05AE-4F55-9EC4-6CAEE1B5B8DF | Duplicate operation settings |
| COMPARE | GUID0F07C838-1313-46F2-A1F5-13A6CD153014 | Compare and Edit |
| OPTEMPL | GUID7436A4BD-F2F3-412A-91A8-003876CFAA3D | Operation templates |
| TPL-LIB | MFG-TOOL-TEMPLATE-LIBRARY-OVERVIEW | Template Library |
| TPL-SEL | MFG-TEMPLATE-LIBRARY-SELECTION | Create an operation from a template |
| TPL-STORE | MFG-TEMPLATE-LIBRARY-STORE-AS-TEMPLATE | Store operations as a template |
| TPL-REF | MFG-REF-TOOLPATH-TEMPLATE-LIBRARY | Template Library reference |
| DEFAULTS | MFG-PARAMETER-DEFAULTS-OVERVIEW | Defaults for parameters |
| MANUALNC | LP-READ-IRONSTRATEGY_MANUAL | Manual NC |
| MANUALNC-REF | GUID895129FD-6548-414D-8B31-EDBCC5EF4439 | Manual NC entry reference |
| MODIFY | MFG-TOOLPATH-MODIFY-OVERVIEW | Toolpath modifications |
| TRIM | MFG-TRIM-OVERVIEW | Toolpath trimming |
| DELETE | MFG-TOOLPATH-DELETE-OVERVIEW | Deleting passes or segments |
| REPLACE | MFG-TOOLPATH-CHANGE-TOOL-MODIFICATION | Replacing tools |
| ENTRY | MFG-TOOLPATH-MOVE-ENTRY-POSITIONS | Tool entry positions |
| LEADS | LEADS-AND-LINKS-MODIFICATION | Leads and links modification |
| GEN-OV | MFG-GENERATE-OPERATIONS-OVERVIEW | Generating operations |
| GEN | MFG-GENERATE-OPERATIONS | Generate an operation |
| IPS | MFG-IPS-OVERVIEW | Automatic in-process stock |
| MTIME | MFG-MACHINING-TIME-OVERVIEW | Machining time |
| NCP-OV | MFG-NC-PROGRAM-OVERVIEW | NC Programs |
| NCP-STEPS | MFG-NC-PROGRAM-STEPS | Create an NC program |
| NCP-GROUPS | MFG-REF-NC-PROGRAM-GROUPS | Organising operations from multiple setups |
| NCP-REF | MFG-REF-NC-PROGRAM | NC Program reference |
| POST-OV | MFG-POST-PROCESSING-OVERVIEW | Post processing |
| SHEET-OV | MFG-SETUP-SHEET-OVERVIEW | Setup sheets |
| SHEET-STEPS | MFG-SETUP-SHEET-STEPS | Create a setup sheet |
| SHEET-NCP | MFG-REF-NC-PROGRAM-SETUP-SHEET | Setup sheet from an NC program |
| TOOL-LIB | MFG-TOOL-LIBRARY-OVERVIEW | Tool Library |
| TOOL-TYPES | MFG-TOOL-LIB-TYPES | Types of tool libraries |
| TOOLTAB | TOOL-TAB-COMMON-MILLING-FEATURES | Tool tab |
| MACH-TYPE | MACHINING-TYPE-OVERVIEW | Machining type |
| SIM | GUID76F8D8EF-2725-4203-944B-B9345936DDDB | Simulation |
| SIM-NOM | MFG-SIMULATE-OPERATIONS-WITHOUT-MACHINE | Simulating without a machine |
| SIM-STOCK | GUIDB27B568A-3BBF-48A6-B430-BBA619B906D0 | Display stock |
| SIM-ISSUES | MFG-SIMULATE-DISPLAY-ISSUES | Check for issues |
| EXT | https://help.autodesk.com/cloudhelp/ENU/Fusion-Extensions/files/EXT-MANUFACTURING.htm | Fusion Manufacturing Extension |
| HEIGHTS | GUID3EB12C22-C06B-4601-ABFE-7F1ECBABB9CE | Setting machining heights |
| CLEAR | MFG-REF-CLEARANCE-HEIGHT | Clearance height |
| RETRACT | MFG-REF-RETRACT-HEIGHT | Retract height |
| FEEDH | MFG-REF-FEED-HEIGHT | Feed height |
| TOPH | MFG-REF-TOP-HEIGHT | Top height |
| BOTH | MFG-REF-BOTTOM-HEIGHT | Bottom height |
| AUTO-HEIGHT | AUTOMATED-HEIGHT-SELECTION | Highest of / lowest of |
| CLEAR-GEO | MFG-REF-CLEARANCE-GEOMETRY | Clearance geometry |
| TRANS | GUID32FB2A12-1443-43C9-9C33-A4946E36A888 | Transitions between passes |
| LIFT | GUIDE697A20E-3EA7-4149-B158-ACDFA61E136A | Lift height and stay-down |
| SMOOTH | GUID5A951723-4797-4F5F-A7A3-45DCBCA2D26F | Smoothing |
| STAYDOWN | GUID2E5ABA80-7240-4F1A-989C-2410C4A0CCE5 | Stay-down distance |
| CUSP | MFG-CUSP-HEIGHT-OVERVIEW | Cusp height |
| FLAT-DET | GUID1C1BF6D5-F1AD-4C85-9EB4-EBD58448DE14 | Detecting flat areas |
| GEO | (parameter text found across the reference pages below) | used for one-line descriptions of shared parameters |
| 2D-OV | MFG-2D-MILLING-OVERVIEW | 2D milling strategies |
| 2D-ADAPT | 2D-ADAPTIVE-READ | 2D Adaptive Roughing |
| 2D-ADAPT-REF | GUIDA73542E9-ED9C-4BD9-A87D-3A0ECA8BEB41 | 2D Adaptive Roughing reference |
| 2D-POCKET | 2D-POCKET-READ | 2D Offset Roughing |
| 2D-POCKET-REF | GUID67DC60DE-EE42-4BC4-B50D-B28D938D146E | 2D Offset Roughing reference |
| FACE | 2D-FACE-READ | Face |
| FACE-REF | GUID9CEA7D03-83C1-4ADD-B6B7-7FFF9DFC97A9 | Face reference |
| CONTOUR | 2D-CONTOUR-READ | 2D Contour |
| CONTOUR-REF | GUID75B6821B-DE26-4E3B-AF10-4A54131CD9E4 | 2D Contour reference |
| TABS | MFG-REF-2D-CONTOUR-TABS | Tabs reference (downloaded; details also on CONTOUR-REF) |
| SLOT / SLOT-REF | 2D-SLOT-READ / GUID29D1D526-292A-4322-A305-7D4744BEFEB1 | Slot |
| TRACE / TRACE-REF | 2D-TRACE-READ / GUID223F93A7-F88E-4359-B02D-B0C2F11AFA8B | Trace |
| THREAD / THREAD-REF | 2D-THREAD-READ / GUIDEB015068-A98E-48EE-A066-9D6053ECAB4A | Thread |
| BORE / BORE-REF | 2D-BORE-READ / GUIDAC5A8A1B-4259-4183-9E45-24320E513CAF | Bore |
| CIRCULAR / CIRCULAR-REF | 2D-CIRCULAR-READ / GUID331A9934-78C9-4890-8C71-2372FDBFEDF7 | Circular |
| ENGRAVE / ENGRAVE-REF | 2D-ENGRAVE-READ / GUID-7111B68D-F0B0-464E-B9A8-09400F4650B9 | Engrave |
| CHAMFER / CHAMFER-REF | 2D-CHAMFER-READ / GUID-D1F8BDCF-9A4E-4636-AB22-B1721A3CE5FE | 2D Chamfer |
| DRILL-OV | MFG-DRILLING-OVERVIEW | Drilling strategies |
| DRILL | LP-READ-IRONSTRATEGY_DRILL | Drill |
| DRILL-REF | MFG-REF-DRILL-CMD | Drill reference |
| DRILL-CYC | MFG-REF-DRILLING-CYCLES | Drilling cycles reference |
| DRILL-COLL | DRILLING-COLLISION-AVOIDANCE | Avoid collisions when drilling (downloaded, skimmed) |
| HOLEREC | LP-READ-IRONHOLERECOGNITION | Hole Recognition |
| 3D-OV | MFG-3D-MILLING-OVERVIEW | 3D milling strategies |
| 3D-ADAPT / 3D-ADAPT-REF | 3D-ADAPTIVE-READ / GUID09E44604-DAD8-47D6-ADC6-C100869DE724 | 3D Adaptive Roughing |
| 3D-POCKET / 3D-POCKET-REF | 3D-POCKET-READ / GUID63F97CC8-99FE-40B7-AFF1-061E826955B3 | 3D Offset Roughing |
| 3P2 | MFG-3-PLUS-2-READ | 3+2 Roughing |
| STEEP / STEEP-REF | 3D-STEEP-AND-SHALLOW-READ / GUID-A6933B73-D98F-43AD-BCFD-A450D91C4E61 | Steep and Shallow |
| FLAT / FLAT-REF | 3D-FLAT-READ / MFG-REF-3D-FLAT-CMD | Flat |
| FLAT-REST | MFG-3D-FLAT-REST-MACHINING | Flat rest machining |
| FLAT-INCL | 3D-FLAT-READ-INCLINED-FLATS | Inclined flats |
| 3D-FLAT-REF | MFG-REF-3D-FLAT-CMD | (same as FLAT-REF) |
| PARALLEL / PARALLEL-REF | 3D-PARALLEL-READ / GUID17CC1F90-A2A5-4CC3-80EB-B7972E327E0F | Parallel |
| SCALLOP / SCALLOP-REF | 3D-SCALLOP-READ / GUID77067110-3D88-4DA7-B171-6B27F04CF8B3 | Scallop |
| CONTOUR3D / CONTOUR3D-REF | 3D-CONTOUR-READ / GUIDEA850013-D826-408E-B076-C613834A77F3 | Contour |
| RAMP / RAMP-REF | 3D-RAMP-READ / GUIDB4D404D3-F897-4FF2-A684-3121C3DDB062 | Ramp |
| PENCIL / PENCIL-REF | 3D-PENCIL-READ / GUIDD7ECF22F-BEAA-4B6A-A2CE-DA6F2C08D787 | Pencil |
| HORIZ / HORIZ-REF | 3D-HORIZONTAL-READ / GUIDF6889A73-5F67-486B-9E73-9662152D1DAD | Horizontal |
| SPIRAL / SPIRAL-REF | 3D-SPIRAL-READ / GUID429D116E-73E2-4046-A8FD-B7A2281B5DA9 | Spiral |
| RADIAL / RADIAL-REF | 3D-RADIAL-READ / GUID6D249502-C436-4060-AC60-910F442AFEEA | Radial |
| MSPIRAL / MSPIRAL-REF | 3D-MORPHED-SPIRAL-READ / GUID37A40E2E-270D-4BE7-9C72-AA996F9A4B5C | Morphed Spiral |
| PROJECT / PROJECT-REF | 3D-PROJECT-READ / GUID8F9E90A7-4F77-4461-B8C2-B79F9EED4DE0 | Project |
| BLEND / BLEND-STEPS | MFG-MULTI-AXIS-BLEND-READ / MFG-MULTI-AXIS-BLEND-STEPS | Blend |
| MORPH / MORPH-REF | 3D-MORPH-READ / GUID-B7EDA6D6-B7A2-4307-83A5-DA79C4DDA11A | Morph |
| CORNER / 3D-CORNER | 3D-CORNER-READ | Corner |
| CORNER-CHOICE | 3D-CORNER-CHOICE-OF-TOOLPATH | Corner pass types |
| CORNER-STEPS | 3D-CORNER-FINISHING-TASK | Generate a Corner toolpath |
| FLOW / FLOW-STEPS | 3D-FLOW-V2-OVERVIEW / 3D-FLOW-V2-STEPS | Flow |
| FLOW-OLD | 3D-FLOW-READ | Flow (Old) |
| DEBURR / DEBURR-STEPS | MFG-MULTI-AXIS-DEBURR-READ / MFG-MULTI-AXIS-DEBURR-STEPS | Deburr |
| GEO-OV / GEO-STEPS | MFG-MULTI-AXIS-GEODESIC-READ / MFG-MULTI-AXIS-GEODESIC-STEPS | Geodesic |
| S: forum | https://forums.autodesk.com/t5/fusion-manufacture/2d-pocket-rest-machining-incomplete/m-p/7745320 and related threads | [S] snippets only: 2D ops not stock aware; rest machining lives on the Geometry tab |
| S: blog | https://www.autodesk.com/products/fusion-360/blog/from-preceding-setup-fusion-cam/ | [S] From Preceding Setup walk-through (snippet only) |

### Pages I could not reach or did not read

- `https://help.autodesk.com/view/fusion360/ENU/?guid=MFG-SETUP-STOCK` (a guessed ID): the viewer
  returned "page not found". I then used the TOC to find the real IDs and read the static pages.
- Autodesk Learn course units (for example
  `https://www.autodesk.com/learn/ondemand/course/fusion360-cam-design-manufacturing-professional/unit/6jnHnMMx0es9PIaWAcy7AC`):
  HTTP 403 to automated fetch. No learning-path content is used in this note.
- The help's videos (linked from many pages) were not watched.
- Downloaded but only skimmed: hole template editor, Excel setup sheets, post customisation,
  multi-axis strategy pages, turning, additive, cutting.
- Not found in the current help: a "Rest Finishing" strategy, a "Rigid tapping" cycle name (see
  findings 2 and the Drill table; both [K]).
