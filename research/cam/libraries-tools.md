# CAM libraries: cutting tools, holders, and their data formats

2026-11-23. Research only: no code changed, no downloaded data saved in the repository (samples
were read in a scratch directory outside the repo and discarded). This note is the "tools and
holders" half of the library research that research/cam/design.md §6 points at. Autodesk Fusion
is the reference product.

Tags on sources: **[R]** read directly (page or file fetched and read), **[S]** search-result
snippet only, **[K]** known from prior knowledge, not re-read for this note. Every source was read
or searched on 2026-11-23 unless noted. Source keys (F1, I3, ...) are listed in §7. Everything
here is restated in my own words. Where the Autodesk schema text derives from ISO 13399 wording,
I give the meaning, not the text.

---

## 0. Summary

- Fusion's tool record is a flat JSON object. Its `type` is one of about 40 strings, plus
  `geometry` (ISO 13399-style symbols such as `DC`, `LCF`, `OAL`, `RE`, `TA`, `SIG` and `TP`,
  plus Autodesk names such as `shoulder-length`, `tip-diameter` and `assemblyGaugeLength`),
  `holder` and `shaft` (stacked cone frusta: `height`, `upper-diameter`, `lower-diameter`),
  `post-process` (number, length/diameter offset, turret, live, manual change, break control),
  and `start-values.presets[]` (feeds and speeds with a material filter). Autodesk publishes a
  JSON Schema for it (draft-07, about 2.6 MB) at cam.autodesk.com [R F6]. It is the most
  practical interchange format in the hobby and job-shop world, and it is documented. **Importing
  it is a must-have.**
- ISO 13399 is the neutral standard, a dictionary of classes and properties (DC, APMX, LF,
  DCON, ...). It is carried in STEP Part 21 files plus STEP models. GTC (Generic Tool Catalog) is
  its packaging and classification layer: a ZIP holding an XML assortment, p21 property files
  and STEP/DXF models. Sandvik, Iscar, Kennametal and Walter publish GTC. NX/Teamcenter,
  Mastercam, Fusion (properties only, not the STEP model) and CoroPlus import it [R F4, G1–G4,
  M1]. The standard text is paywalled. GTC working documents are behind a member area.
- Mastercam `.tooldb`, the SolidCAM ToolKit and NX `tool_database.dat` are closed or
  semi-documented. FreeCAD `.fctb`/`.fctl` (JSON) and the LinuxCNC `tool.tbl` (text) are open
  and simple [R O1–O4].
- Holders are, for collision checking, a revolved profile: Fusion stacks frusta below a gauge
  line [R F7, F8]. Taper standards (ISO 7388-1/-2/-3, ISO 12164 HSK, ISO 26623 Capto, ISO 15488
  ER) fix the interface, not the chuck's outside. The outside comes from the maker's
  catalogue or STEP.
- For forgeCAD: `[[tool]]` and `[[holder]]` TOML in mm, ISO 13399 symbol names where they exist,
  and holder profiles as `[[z, d]]` polylines with optional arcs. Import Fusion JSON first, then
  FreeCAD and LinuxCNC (export as well), and GTC/p21 later (§6).

---

## 1. Cutter types and their parameters

### 1.1 Fusion's type list

Fusion's "Supported tools reference" [R F3] and the schema's `type` enums [R F6] give these
types:

| family | Fusion `type` strings |
|---|---|
| milling | `flat end mill`, `ball end mill`, `bull nose end mill`, `face mill`, `tapered mill` (subtype `tapered-type`: `tapered_ball` / `tapered_bull_nose`), `radius mill` (corner rounding), `chamfer mill` (UI: engrave/chamfer), `corner chamfer end mill`, `dovetail mill`, `lollipop mill`, `slot mill` (also slitting saws and keyseat cutters, by vendor convention [S V1]), `thread mill`, `form mill`, `circle segment barrel`, `circle segment lens`, `circle segment oval`, `circle segment taper` |
| hole making | `drill`, `center drill`, `spot drill`, `counter sink`, `counter bore`, `reamer`, `boring bar`, `tap right hand`, `tap left hand` |
| turning | `turning general`, `turning boring`, `turning grooving`, `turning threading` |
| other | `probe`, `holder`, `shaft`, `tool block` (lathe/mill-turn block), `waterjet`, `laser cutter`, `plasma cutter`, depositing types (`electric arc wire`, `laser powder`, `laser wire`) |

There is no T-slot or key-cutter type. Users model T-slot and key cutters as a `slot mill` with
a reduced neck (`shoulder-diameter` < `DC`) [S F9 forum threads]. There is no separate "tap type"
(cut, form, spiral point) field. Taps differ only by hand [R F6]. Tap type lives in the
description, and in ISO 13399 as a thread-form/chamfer property (`THFT`, `THCHT`) [R I4].

### 1.2 Common rotating-tool geometry (Fusion Cutter tab, schema keys)

From the Cutter tab reference [R F1] and the schema [R F6]. "CAM use" is what the field drives.

| UI name | JSON key | meaning | CAM use |
|---|---|---|---|
| Diameter | `DC` | cutting diameter at the cutting reference point | offsets, stepover, compensation, rpm from Vc |
| Max cutting diameter | `DCX` | largest swept diameter (face mills, dovetails) | stock/collision envelope |
| Shaft diameter | `SFDM` | clamped shank diameter | holder fit, collision |
| Overall length | `OAL` | whole tool | simulation and the assembly |
| Length below holder | `LB` | stick-out from the holder. The schema says it was formerly "body length", so LB in Fusion is not ISO's body length | reach and collision. With the holder gauge length it gives `assemblyGaugeLength` |
| Shoulder length | `shoulder-length` | tip to the start of the shank, also called LBS/LBX | reach of a reduced-neck tool; where the neck ends |
| Shoulder diameter | `shoulder-diameter` | neck diameter between the flutes and the shank | neck collision in deep walls |
| Flute length | `LCF` | length of cutting edge | maximum axial engagement; non-cutting body collision above it |
| Corner radius | `RE` | corner radius in the profile | bull nose, scallop, rest machining |
| Taper angle | `TA` | Fusion UI: *half* the included angle. Schema/ISO: included angle between the generators | tapered and chamfer tools |
| Tip angle / point angle | `SIG` | point angle of drills and spot drills | drill-through breakthrough, spot depth for a chamfer |
| Tip diameter | `tip-diameter` | flat at the tip of angled tools | chamfer and engrave depth |
| Tip length | `tip-length` | centre drill: tip to the start of the countersink | centre-drill depth |
| Tip offset | `tip-offset` | form mill: tip to the compensation point | contour height |
| Number of flutes | `NOF` | flute count | feed per tooth to feed |
| Clockwise rotation | `HAND` (bool) | right-hand when true | M3/M4 |
| Coolant supply | `CSP` (bool) | through-tool coolant | coolant mode choice |
| Thread pitch | `TP` | taps and thread mills. Inch is 1/TPI | tap feed = pitch × rpm; helix pitch |
| Number of teeth | `NT` | thread mill teeth per edge | multi-pitch thread mills |
| Thread pitch min/max | `TPN`, `TPX` | range a thread mill or insert can cut | validation |
| Thread profile angle | `thread-profile-angle` | usually 60° | thread form |
| Thread tip type, radius, width | `thread-tip-type` (flat, point, round), `thread-tip-radius`, `thread-tip-width` | | thread root |
| Upper radius, lower radius, profile radius, axial distance | `upper-radius`, `lower-radius`, `profile-radius`, `axial-distance` | circle-segment (barrel) tools: the large profile arc, its centre height above the tip, and the corner radii. `lensType` for lens | 5-axis finishing with large effective radius |
| Chamfer angle and width | `chamfer-angle`, `chamfer-width` | corner chamfer end mill | |
| Form profile | `profile` (array) | form mill: a closed half-profile revolved about the axis | 2D ops. 3D paths use an approximating end mill [R F5] |
| Assembly gauge length | `assemblyGaugeLength` | gauge line to tip of the whole assembly | tool length offset prediction |
| Material | `BMC` | `unspecified`, `hss`, `ti coated`, `carbide`, `ceramics` | presets, Vc defaults |
| (legacy) grade | `GRADE` | the schema says it no longer does anything | — |

The schema fixes which keys each type allows [R F6, digest S1]. Notable per-type sets:
`center drill` adds `SIG, TA, tip-diameter, tip-length`. `counter sink` and `spot drill` add
`SIG, tip-diameter`. `dovetail mill` adds `RE, TA`. `face mill` adds `RE, TA, upper-radius`.
`radius mill` adds `RE, tip-length`. Taps add `TP` and have no `HAND` (the type carries the hand).
`thread mill` adds `NT, TP, TPN, TPX` and the thread-tip keys. `probe` has only `DC, LB, SFDM,
assemblyGaugeLength`. `form mill` has `profile, tip-offset`. Units are per tool (`unit`:
`millimeters` or `inches`) [R F6, sample libraries S2].

What is missing in Fusion compared with ISO 13399: helix angle (`FHA`), coating as a field (only
the `ti coated` material value), neck diameter separate from shoulder, usable length `LU`, rpm
max `RPMX`, tool weight, and balance class. These exist in ISO 13399 [R I4].

### 1.3 Per-type notes (what CAM needs from each)

| tool | defining parameters | notes |
|---|---|---|
| flat end mill | DC, LCF, NOF, shoulder, SFDM | RE = 0 |
| ball | DC (RE = DC/2 implied) | Fusion's ball type has no RE key: the ball is implied |
| bull nose / torus | DC, RE | FreeCAD calls it "torus" too [R O2] |
| chamfer / engrave / V-bit | DC, TA, tip-diameter | Fusion's TA here is the half angle in the UI. FreeCAD `CuttingEdgeAngle` is the included angle [R O2] |
| spot drill | DC, SIG, tip-diameter | chamfer-by-spot uses SIG |
| centre drill | DC (body), tip-diameter (pilot), tip-length, SIG, TA | two-stage profile |
| drill | DC, SIG, LCF | the breakthrough uses the tip length, DC/(2·tan(SIG/2)) |
| tap | DC, TP, hand, LCF | FreeCAD adds `TipAngle` (chamfer) and flutes [R O2] |
| thread mill | DC, TP or TPN..TPX, NT, profile angle, tip type | FreeCAD: `Crest`, `NeckDiameter`, `NeckLength` [R O2] |
| reamer | DC, LCF | |
| boring bar (hole making) | DC, LCF, shoulder | the rotating boring head, not the lathe bar |
| counterbore | DC, shoulder (pilot not modelled) | |
| countersink | DC, SIG, tip-diameter | |
| slot / key / T-slot / saw | DC, LCF (= width), RE, shoulder-diameter (neck) | FreeCAD has `Keyway` (`LowerCuttingHeight`, `NeckCuttingHeight`) and `SlittingSaw` (`BladeThickness`, `CapDiameter`, `CapHeight`) [R O2] |
| dovetail | DC, DCX, TA, RE, LCF | FreeCAD `Dovetail`: `CuttingEdgeAngle`, `NeckDiameter`, `NeckHeight`, `TipDiameter` [R O2] |
| lollipop | DC (ball), shoulder-diameter (neck) | undercuts |
| tapered | DC (tip), TA, RE, LCF | tapered ball and tapered bull subtypes |
| circle segment (barrel, oval, taper, lens) | DC, profile-radius, axial-distance, upper/lower radius, TA (taper form) | 5-axis "barrel" finishing |
| form | closed half-profile + compensation point | Fusion warns 3D strategies approximate it [R F5] |
| probe | DC (stylus ball), SFDM, LB | used by probing cycles |

### 1.4 Turning tools

Fusion's turning tool reference [R F2] and the schema's insert/holder keys [R F6]:

**Insert (`geometry`)**

| key | meaning |
|---|---|
| `SC` | insert shape code. Standard set: A B C D E K L M R S T V W. Also `custom`, `groove square`, `groove round`, `thread iso double`, `thread iso triple` |
| `INSD` | insert size as inscribed circle (IC). Alternatively, `SIZE_SPECIFICATION_MODE` = `IC` or `CE` (cutting-edge length) |
| `S` | thickness |
| `RE` | nose radius |
| `RA` | relief (clearance) angle, the 2nd letter of the ISO 1832 code |
| `TC` | tolerance class: C H E G M U |
| `SCTY` | cross-section (hole, chip breaker): A B C F G H J M N Q R T U W X |
| `EPSR` | included angle, for custom inserts |
| `tool_grooveWidth`, `tool_insertWidth` | grooving width (cutting) and body width |
| `DRVA`, `stickout` | drive angle; stick-out from a tool block |
| threading | `TPN`, `TPX`, `tool_internalThread`, `thread-profile-angle`, `thread-tip-type`/radius/width |

ISO 1832 shape letters [K, structure confirmed S I8]: **C** 80° rhombic, **D** 55° rhombic,
**V** 35° rhombic, **W** 80° trigon, **T** 60° triangle, **S** 90° square, **R** round, **K** 55°
parallelogram, **E** 75° and **M** 86° rhombic, **A/B** 85°/82° parallelograms, **L** 90°
rectangle, **H/O/P** hexagon, octagon, pentagon. The full code reads shape, clearance (N = 0°,
B = 5°, C = 7°, P = 11°, ...), tolerance (M, G, ...), type (hole/chipbreaker), IC size,
thickness, nose radius. Example: CNMG 120408 is 80° rhombic, 0° clearance, M tolerance, G
section, 12 mm edge, 4.76 mm thick, 0.8 mm radius.

**Holder (`holder`)**: `THSC` (holder style code: ISO letters A–Y for straight and offset
shanks; `boring bar f/g/j/k/l/p/q/u`; `groove external/internal/face`; `thread straight/offset/
face`; `custom`), `HAND` (R/L/N), `MTP` (clamping: C, D, M, P, S, the 1st letter of an ISO 5608
holder code [K]), `CW` (cutting width/head width), `LH` (head length), `OAL`, `W` (shank width),
`H` (shank height), `tool_roundShank` (boring bars). **Setup**: orientation angle,
`compensation` (`tip`, `tip tangent`; the UI also has tip centre and insert centre) and
`tool_shaftAxisAngle` [R F2, F6].

Turning presets add `use-constant-surface-speed` (G96) and feeds per revolution (`f_n`,
`f_n_leadIn`, `f_n_leadOut`) [R F6, S2].

ISO 13399 covers all of this more richly (`KAPR` lead angle, `PSIR`, `LF` functional length,
`WF` functional width, `CDX` maximum cut depth, `APMX`) [R I4].

---

## 2. Tool assembly: holder, extensions, offsets, presets

### 2.1 Fusion's assembly model

- **Holder** (`holder`, also a library item of `type: holder`): `segments[]` of `{height,
  lower-diameter, upper-diameter}`, stacked from the tool end upward, plus `gaugeLength`,
  description, vendor, product-id and link [R F6, F8, holder samples S3]. Gauge length is the sum
  of segment heights below the gauge line. A segment flagged "above gauge line" (taper and
  flange) is excluded. A user value may override it if 0 < value ≤ total height [R F7].
- **Shaft** (`shaft`): the same segment list, for extensions and the visible shank between tool
  and holder [R F1, F6].
- **Assembly gauge length** (`assemblyGaugeLength`): gauge line to tool tip, which is about the
  holder gauge length plus `LB`, edited on the Cutter tab [R F7]. This is what a presetter
  measures and what goes into the H register.
- **Post Processor tab** (`post-process`): `number` (T), `length-offset` (H), `diameter-offset`
  (D), `turret`, `comment`, `manual-tool-change`, `live` (live-tool lathe), `break-control`
  (post checks for breakage). Turning tools have a single `compensation-offset` [R F1, F2, F6].
- **Presets** (`start-values.presets[]`): each has `name`, `guid`, `material` {`category` (all,
  metal, wood, plastic, glass, ceramic, stone, fabric), `query` (a stock-material name),
  `use-hardness`, `minimum-hardness`, `maximum-hardness`}, `stock-materials[]`, optional
  `strategies` {roughing[], finishing[]}, then cutting data: `n`, `n_ramp`, `v_c`, `f_z`, `f_n`,
  `v_f`, `v_f_plunge`, `v_f_ramp`, `v_f_leadIn`, `v_f_leadOut`, `v_f_transition`,
  `v_f_retract`, `ramp-angle`, `stepdown`/`use-stepdown`, `stepover`/`use-stepover`,
  `tool-coolant` [R F6, S2]. Coolant enum: `disabled`, `flood`, `mist`, `flood mist`, `tool`
  (through-tool), `flood tool`, `air`, `air through tool`, `suction` [R F6]. Presets are copied
  between tools of the same type only [R F10].
- **Expressions**: any field may be an expression over named parameters (`tool_diameter*.10`),
  stored next to the values in `expressions` {param: text}. Expressions are not exported to CSV
  [R F11, F4].
- Fusion has **no tool-life or wear field** in the tool record [R F6]. Wear and life belong to
  the machine/control or to tool management (TDM, CoroPlus). LinuxCNC can track usage through
  its `db_program` interface [R O4].

### 2.2 Other systems

- **LinuxCNC** `tool.tbl`: one line per tool, `T<n> P<pocket> [X..W offsets] [D dia] [I front
  angle J back angle Q orientation 0–9 (lathe)] ;comment`, in machine units. Pocket 0 is the
  spindle. `G43 Hn` uses entry n's length. `G41/G42` use D. Random and non-random changers differ
  in how P is rewritten [R O3]. `db_program` replaces the file with a process speaking a
  line protocol (`g`, `p`, `l`, `u`) and allows usage-based tool choice [R O4].
- **Mastercam**: tool assemblies are a tool plus one or more holder components plus projection
  length, kept in a `.tooldb`. Inch and metric can coexist in one file [S M2]. CoroPlus
  assemblies appear directly in Mastercam 2018+ [S M3].
- **FreeCAD**: the tool bit carries the geometry. A tool controller (in the job) carries
  feeds, speeds and spindle direction. The library gives the tool number [R O1].

---

## 3. Data formats and catalogues

### 3.1 ISO 13399 (cutting tool data representation and exchange)

Parts verified or partly verified (titles from ISO, BSI and DIN catalogue listings, which are
[S] because iso.org returned 403):

| part | content |
|---|---|
| 1 (2006) | overview, fundamental principles, general information model [S I1, I2] |
| 2 | reference dictionary for cutting items (inserts, edges). 2021 edition, confirmed 2025 [S I3] |
| 3 | reference dictionary for tool items (solid tools, bodies) [S I3] |
| 4 | reference dictionary for adaptive items (holders, adaptors, extensions) [S I3] |
| 5 | reference dictionary for assembly items and auxiliary items [S I3] |
| 50 | reference dictionary for reference systems and common concepts (coordinate systems, connections) [S I1] |
| 60 | reference dictionary for connection systems [S I1] |
| 70 / 71 / 72 | graphical data layout: layer settings for tool layout; documents for 2D graphical product info; drawing header properties as XML [S I5] |
| 80 (2017) | creation and exchange of 3D models, overview and principles (simplified models) [S I6] |
| 100 | definitions, principles and methods for reference dictionaries [S I1] |
| 150 | usage guidelines [S I1] |
| 2xx / 3xx / 4xx | 3D model construction rules per item kind, e.g. 201 regular inserts, 301/302 taps and solid drills, 314 cartridges [S I7] |

How it works: every item belongs to a class in the dictionary. A class has properties
identified by a code and a preferred symbol (DC, APMX, LF, DCON, ...), with a defined unit and
value domain. The dictionaries follow the ISO 13584 (PLIB) and IEC 61360 dictionary model [K].
Exchange is by a STEP Part 21 file (ISO 10303-21), STEP-XML or SDAI database access [S I1]. The
3D models are simplified STEP solids with named coordinate systems: PCS (primary) at the
cutting reference and MCS/CSW at the connections, so a CAM system can stack items
[K; scope S I6]. Item types: cutting item (insert), tool item (body or solid tool), adaptive
item (holder, adaptor, extension), assembly item, auxiliary item (screws, clamps) [S I3].
The symbol list published by Dormer Pramet (about 140 symbols) is a good public crib [R I4]. Key
ones: DC/DCX cutting diameter, APMX maximum depth of cut, LF functional length, LU usable
length, LCF flute length, LB body length, LH head length, OAL, DCON connection diameter, DN
neck diameter, RE, KAPR, SIG, NOF, FHA helix, TP/TPI, CW, IC, S, BMC/SUBSTRATE/COATING, CNSC
coolant entry, RPMX, WT.

Open or not: the text is copyrighted and sold by ISO [R I9]. The symbol names and meanings are
widely reproduced by vendors. Fusion's schema reuses the symbols and paraphrases of their
definitions [R F6].

### 3.2 GTC (Generic Tool Catalog)

- Developed by Siemens PLM with Sandvik Coromant, Iscar and Kennametal. CIMSOURCE and
  MachiningCloud took part. It adds a vendor-neutral classification hierarchy and a package
  structure on top of ISO 13399 without contradicting it [R G1, 2016 post].
- A GTC package is a ZIP of item data: properties (p21) and graphics (STEP 3D, DXF 2D) for one
  or more items or assemblies [R G2; R F4]. The assortment and classification are in
  `package_assortment.xml`, whose items carry `gtc_generic_class_id`. A class starting `INS` is
  an insert (consumable) [R G3]. GTC 2.0 is current in third-party importers [R G3].
- The specification ("GTC Current Working Documents") is published through gtc-tools.com, run
  for the GTC group. Parts of it sit in a member area. A free P21 viewer is offered [R G4]. I did
  not download the working documents.
- Who publishes: Sandvik Coromant (free with a web account; compatible with CoroPlus ToolLibrary
  2.0+ and Teamcenter MRL 11.4+) [R G5], Iscar and Walter (on request), Kennametal [R G1].
- Who imports: Teamcenter MRL / NX CAM since 10.1.4 [R G1]. Mastercam (milling, and lathe from
  2026) into `.tooldb`, with a validation step [R M1]. Fusion (GTC .zip and bare .p21; maps p21
  geometry to Fusion parameters, ignores the STEP model, rejects double-insert holders, angled
  groovers and PrimeTurning; missing values become errors and defaulted values become warnings)
  [R F12]. CoroPlus ToolLibrary [R G6]. ToolsUnited exports to ISO 13399/GTC and DIN 4000 [S G7].

### 3.3 Fusion tool library files

- **Formats** [R F4]: import `.json`/`.tools` (Fusion), `.hsmlib` (Inventor CAM/HSMWorks), `.csv`,
  GTC `.zip`, `.p21`. Export `.tools` (the same JSON, zipped), `.json`, `.hsmlib`, `.csv`. GTC
  and p21 are import-only. CSV keeps the exported header names, needs consecutive tool indexes,
  and creates presets by row [R F13]. `.hsmlib` is XML [K].
- **JSON structure** [R F6; sample libraries S2]: `{ "data": [tool, ...], "version": int }`. The
  schema's minimum version is 11. Observed versions are 1 (old), 26, 35 and 36 (current Fusion
  writes 36 per a vendor page [S V1]). Required per tool: `guid` (UUID, or `<...>` placeholders
  for internal use), `type`, `unit`, `BMC`, `geometry`, `post-process`, `start-values`. Optional:
  `description`, `vendor`, `product-id`, `product-link`, `holder`, `shaft`, `expressions`,
  `tool-block`, `setup` (turning), `errors`, `last_modified`, `reference_guid`. Holder-only
  libraries use `type: holder` records with `segments` [S3]. The schema is published as
  `ToolLibrary.schema.json` and as a zip on the "Tools for Autodesk Fusion" page [R F6, V2].
- **Library locations** [R F14, F15]: Documents (in the design file; editable), Local, Cloud
  (being replaced by Hub Libraries), the Fusion Library (read-only; samples, holders in inch and
  metric, probes, turning) and Vendor libraries (read-only; rolling out). If they are missing,
  libraries can be downloaded from the "Tools for Autodesk Fusion" site and imported.
- **Vendor catalogue**: the index behind cam.autodesk.com/hsmtools lists 52 libraries from 32
  vendors [R V2]. Among them: Harvey Tool (3), Helical Solutions (2), Sandvik Coromant (solid end
  mills, solid hole making, turning inch/metric), Kennametal (turning) and Kennametal Tools, ISCAR
  (6: hole making, solid end mills, turning), Garr Tool, Guhring, OSG, Mitsubishi Materials,
  KYOCERA SGS, IMCO, M.A. Ford, Dormer Pramet, Widia and Ingersoll (turning), Valor, NexGen,
  Destiny, SwiftCARB, DATRON, Delta Tools, Fraiser, XEBEC, Bits & Bits, Tormach and Haas Tooling
  starter sets. Holders come from Haas, Haimer and Schunk; probes from Renishaw and Blum. Harvey
  and Helical also ship App Store add-ins [S V3].
- API: Fusion's CAM API reads and writes tool libraries and tools as this JSON [K; the API page
  returned 404].

### 3.4 Mastercam

`.tooldb` is the tool library since Mastercam 2017. It holds tools, holders and assemblies, in
inch and metric [S M2]. Vendors describe it as closed with no published spec [R M4]. A forum
claim that it is SQLite is unverified [S]. The older format was `.tools` (pre-2017) [K]. Import
routes: GTC (mill; lathe from 2026) [R M1], CoroPlus [S M3], MachiningCloud/ToolsUnited plugins
[S G7, R M4], CSV paste [R M4].

### 3.5 SolidCAM, NX, others

- **SolidCAM**: the classic `.tab` tool tables are converted into ToolKit libraries (2021+) [S
  C1]. Vendors integrate through "Import Tools from Third Party" (Kennametal) [S C2]; ToolsUnited
  has a SolidCAM download path [S C1]. The format is not public.
- **NX**: the ASCII `tool_database.dat` (pipe-delimited `FORMAT` and `DATA` lines per `#CLASS`
  block, e.g. `END_MILL_NON_INDEXABLE`, with `MATREF` material refs). Units come from the
  english/metric library folder, and `UGII_CAM_LIBRARY_TOOL_ASCII_DIR` relocates it [R N1; S N2].
  Siemens discourages hand edits [S N2]. With Teamcenter, MRL stores GTC-imported items and
  assemblies [R G1].
- **FreeCAD CAM** (main branch, read 2026-11-23) [R O1, O2]:
  - A tool bit is JSON, `.fctb`: `version` 2, `name`, `shape` (an `.fcstd` parametric body),
    `shape-type`, `parameter` {quantities as strings with units, e.g. `"5.0000 mm"`, `"60 °"`},
    `attribute` {}.
  - A library is JSON, `.fctl`: `{"tools":[{"nr":1,"path":"x.fctb"},...],"version":1}`. The
    number belongs to the library, not the bit.
  - Shipped shapes: ballend, bullnose (alias torus), chamfer, dovetail, drill (subtypes jobber,
    stub, spot, ...), endmill, keyway, lollipop, probe, radius, reamer, slittingsaw, tap,
    taperedballnose, thread-mill, v-bit, custom.
  - Parameter names by shape: endmill `Diameter, CuttingEdgeHeight, Length, ShankDiameter,
    Flutes`. Bull adds `CornerRadius`. V-bit `CuttingEdgeAngle, TipDiameter`. Tap `Pitch,
    TipAngle, CuttingEdgeLength`. Thread mill `Crest, NeckDiameter, NeckLength`. Tapered ball
    `TaperAngle, TaperDiameter`. Dovetail `CuttingEdgeAngle, NeckDiameter, NeckHeight,
    NeckCuttingHeight, TipDiameter`. Probe `ShaftDiameter`.
  - Bit-level `SpindleDirection` (Forward, Reverse, None) and `Material` (HSS, Carbide).
  - The new asset code also has a YAML serializer for bits and a LinuxCNC `.tbl` exporter
    (export-only; writes `T<n> P0 ... D<dia> ;label`).
  - Licence LGPL-2.1+ [K]: format readable, code not portable into kernel/.
- **LinuxCNC**: §2.2. Open, trivial, export-worthy.

### 3.6 Cloud catalogues

| service | what | formats | status |
|---|---|---|---|
| Sandvik CoroPlus ToolLibrary | build assemblies from any ISO 13399 supplier; >900k items from >40 suppliers via a ToolsUnited account | GTC export, STEP AP214/AP242 (basic or detailed), DXF. CAM add-ins: Fusion, NX, Mastercam, Cimatron, GibbsCAM, DELMIA | paid, 12-month subscription [R G6] |
| Sandvik tool data download | per-product GTC packages | GTC | free with web account [R G5] |
| ToolsUnited (CIMSOURCE) | multi-vendor catalogue, StOB schema mapping DIN 4000 ↔ ISO 13399 | >20 CAM/TMS interfaces, ISO/GTC, DIN 4000, STEP | pay-per-use or flat rate [S G7] |
| MachiningCloud (listed under Hexagon) | multi-vendor app with CAM plugins (Mastercam, Vericut, hyperMILL since 2025) | plugin-based | Kennametal and Widia catalogues announced as removed [S G8, H1] |
| Kennametal NOVO | assembly builder, ISO 13399 export | — | discontinued 25 May 2025; users moved to MachiningCloud [S G8] |

### 3.7 Openness

| format | open spec? | notes |
|---|---|---|
| Fusion JSON / `.tools` | **yes**: a published JSON Schema [R F6] | Autodesk terms on the schema not reviewed; implementing a reader from a schema is ordinary interoperability |
| ISO 13399 p21 + STEP | standard is paywalled; symbols public | needs a STEP p21 reader; the class dictionary is needed to interpret |
| GTC zip | working docs on gtc-tools.com, member area | XML and p21 inside |
| FreeCAD `.fctb`/`.fctl` | yes (source, LGPL) | shapes are `.fcstd` bodies; read parameters only |
| LinuxCNC `.tbl` | yes (docs) | geometry is only D |
| Mastercam `.tooldb`, SolidCAM, NX `.dat` | no / partly | not worth importing directly; users can go via GTC or Fusion |
| HSM `.hsmlib` | no public spec | Fusion converts it |

---

## 4. Holder library

### 4.1 Spindle interfaces

| interface | standard | key facts |
|---|---|---|
| 7/24 steep taper (SK, ISO, "DIN") | ISO 7388-1 (replaces DIN 69871; DIN ISO 7388-1:2014) [S H2] | 7 in 24 taper (16.594° included) [R H3]. Sizes 30/40/45/50/60. Gauge diameters 31.75 / 44.45 / 57.15 / 69.85 mm for 30/40/45/50 [K; 40 = 44.45 S H2]. Pull stud M12/M16/M20/M24 for 30/40/45/50 [K] |
| CAT (V-flange) | ANSI B5.50 → ISO 7388-3 [K; R H3] | the same taper, inch pull-stud threads (CAT40 5/8-11) [K]. Not interchangeable with BT [R H3] |
| BT | MAS 403 / JIS B 6339 → ISO 7388-2 [R H3] | Japanese derivative. Thicker, symmetric flange; metric studs [K] |
| NMTB | ANSI B5.18 | manual-change 7/24, drawbar-threaded [R H3] |
| HSK | ISO 12164-1 / DIN 69893-1 (forms A, C) and others [S H4] | hollow taper shank about 1:10 with face contact. Forms A (ATC), B (larger flange, high torque), C/D (manual), E/F (high speed, no drive slots), T (turning, tight keys). Size = flange diameter 25–160 (HSK-A63) [R H3] |
| Capto (PSC) | ISO 26623 (2008; C10 added 2013) [S H5] | polygon taper 1:20, flange contact. C3/C4/C5/C6/C8/C10 = 32/40/50/63/80/100 mm [S H5; R H3] |
| R8 | Bridgeport | 16°51′ cone, 1.25 in OD, 0.949 in locating diameter, 7/16-20 drawbar [R H3] |
| Morse | ISO 296 | about 1:20; MT0–MT7 [R H3] |
| Kwik-Switch | Command/Kwik-Switch 200/300 quick-change for R8 knee mills | not re-read [K] |

### 4.2 Clamping types and what geometry they need

- **ER collet chuck** (ISO 15488, formerly DIN 6499; Rego-Fix 1972): ER8/11/16/20/25/32/40/50.
  The number is the collet seat opening in mm. A collet grips from its nominal size to 1 mm below
  (0.5 mm for small sizes; 2 mm for ER50) [R H6]. Kennametal gives 0.5 mm collapse for ER8/11 and
  1 mm for ER16–40. Set coverage is ER11 1–7, ER16 1–10, ER20 1(2)–13, ER25 1–16, ER32 2–20,
  ER40 3–26 mm [S H7]. Geometry: the nut OD and length dominate collision. Nut OD for ER32 is
  about 50 mm (slim nuts are smaller) [K, maker-specific].
- **Shrink-fit**: slim nose with a 4.5° (or 3°) nose taper, about 20–30% slimmer than ER [K]. The
  profile is maker data, e.g. Haimer's HSK-A63 shrink chucks with 80–160 mm gauge lengths [S H4].
  Haimer offers a Fusion holder library [R V2].
- **Hydraulic**: wider body, good runout. Profile from the maker.
- **Side-lock / Weldon** (DIN 1835-B / DIN 6359 shanks [K]): a cylinder with a radial screw boss,
  which is asymmetric. Model it as revolved at the boss radius for safety.
- **Shell-mill arbor, drill chuck, tapping attachment, boring head, extensions**: all the same
  revolved stack.

### 4.3 Geometry for collision checking

Fusion represents every holder and extension as a stack of truncated cones (`height`,
`lower-diameter`, `upper-diameter`). Steps are zero-height or tiny segments, and segments above
the gauge line are marked [R F8, S3]. A real BT40-ER11-100 holder sample has about 20 segments,
down to 0.14 mm tall, at 14–63 mm diameters [S3]. This is a revolved polyline profile. Arcs (nut
radii) are faceted. ISO 13399-80 and the 4xx parts define simplified STEP models for adaptive
items with connection coordinate systems [S I6]. GTC packages carry those STEP files [R F12].
For forgeCAD's exact checker (design.md §1), a revolved profile of lines **and arcs** is exactly
what the distance code wants. A STEP holder is reduced to its revolved profile by sectioning
through the axis and taking the outer envelope.

### 4.4 Where dimensions come from

- The standards (ISO 7388, ISO 12164, ISO 26623, ISO 15488) fix the taper, flange and seat. They
  are paywalled; sample pages exist (iteh previews) [S H2].
- The chuck nose is not standardised. Get it from maker catalogues and their STEP/DXF downloads:
  Haimer, Schunk and Haas (Fusion holder libraries [R V2]), Rego-Fix (ER catalogue [S H7]),
  Sandvik/Kennametal/Iscar (GTC with STEP [R G1, G5]), Hoffmann and Kemmler shop pages (STEP per
  item [S H4]), and Fusion's own read-only holder library (inch and metric) [R F14].
- Open-source holder profiles exist in user GitHub repos (Fusion holder JSON) [S3]. Their licence
  is unclear, so use them as a cross-check only.

---

## 5. Recommendations for forgeCAD

### 5.1 Principles

1. **Our own TOML is the truth.** Importers translate into it, in the forge-vendor style:
   `source`, `taken` (date), `note`, and `source_<key>` per value (design.md §6;
   kernel/forge-material/data/materials.toml shows the pattern).
2. **mm and degrees inside**, always. A record may say `unit = "in"` for its numbers (vendor
   catalogues), and it is converted on load. Angles are degrees in the file and **included
   angles** everywhere. A field named `half_angle` is the only half angle. This kills Fusion's TA
   ambiguity (half in UI, included in schema text).
3. **ISO 13399 symbols as field names** where one exists (`dc`, `dcx`, `re`, `lcf`, `lu`, `lb`,
   `oal`, `dcon`, `dn`, `sig`, `tp`, `nof`, `kapr`, `ic`, `s`), with long aliases accepted
   (`diameter`, `corner_radius`, ...). Every importer then maps to one vocabulary.
4. **Geometry is a profile.** Each tool type is a generator of a revolved line/arc profile
   (forge-cam-geom). `type = "profile"` takes a profile directly (form tools, barrels, holders,
   STEP-derived). The checker sees only profiles.
5. **Separate the tool, holder, assembly and cutting data**, as Fusion half-does. The assembly
   (T number, H, D, stick-out) belongs to the shop/machine, not the catalogue item.

### 5.2 Tool record (`*.tools.toml`)

```toml
[[tool]]
id = "flat-6-3fl-alu"            # required, unique in the file; referenced by assemblies
type = "flat"                    # flat ball bull chamfer spot center_drill drill tap
                                 # thread_mill reamer bore counterbore countersink slot
                                 # dovetail lollipop tapered barrel form probe
                                 # turn_general turn_boring turn_groove turn_thread
dc = 6.0                         # required for rotating tools (mm)
re = 0.0                         # bull/torus; ball implies dc/2
nof = 3
lcf = 18.0                       # flute length, required
lu = 22.0                        # usable length below the neck end (Fusion shoulder-length)
dn = 5.7                         # neck diameter (Fusion shoulder-diameter), optional
dcon = 6.0                       # shank (Fusion SFDM), required
oal = 57.0                       # required
hand = "right"                   # right | left | neutral(turning)
coolant_through = false
material = "carbide"             # hss | carbide | cermet | ceramic | pcd | cbn
coating = "ZrN"                  # free text, optional
helix = 45.0                     # degrees, optional
# type-specific, validated per type:
# sig (point/included angle), tip_d, tip_len, angle (included), tp (pitch mm), tpi,
# tpn/tpx, nt, thread_angle, thread_tip = "flat|point|round", tip_r, tip_w,
# barrel: profile_r, axial_dist, upper_r, lower_r, form: profile = [[z, r], ...] with arcs
# turning: insert = "CNMG120408", sc, ic, s, re, ra(clearance), tc, kapr, holder_code, cw, lh, h, b
vendor = "Example Tool Co"
product_id = "EX-123"
url = "https://..."
source = "Fusion library import, <file>, version 36"
taken = "2026-11-23"
note = "verify before ordering"

[[tool.shaft]]                   # optional segments between tool and holder, like Fusion
h = 10.0
d_lower = 6.0
d_upper = 6.0

[[tool.preset]]                  # optional; per material group, like Fusion presets
name = "aluminium rough"
group = "N"                      # ISO P M K N S H (forge-material cam.group)
material = "6061-T6"             # optional, a forge-material name
vc = 300.0                       # m/min
fz = 0.04                        # mm/tooth (or fn mm/rev for drills/turning)
ap = 6.0                         # optional, mm
ae = 0.6                         # optional, mm
ramp_angle = 2.0
coolant = "flood"                # off flood mist through air through_air mql suction
```

Required by type: rotating tools need `id, type, dc, lcf, oal, dcon` plus the type's own keys
(ball none; bull `re`; chamfer `angle`; drill `sig`; tap `tp|tpi` and `hand`; thread mill `tp`
or `tpn..tpx`; form `profile`). Turning tools need `id, type, insert|sc+ic+s+re`, holder style,
`hand` and the orientation. Anything else is optional. A missing value is refused with a typed
error and a fix ("give lcf"), never defaulted silently. Fusion's import errors and warnings show
why: vendor data is incomplete [R F12].

### 5.3 Holder record (`*.holders.toml`)

```toml
[[holder]]
id = "bt40-er32-70"
interface = "BT40"               # BT30 BT40 BT50 CAT40 CAT50 SK40 (ISO 7388-1) HSK-A63 ... C6 R8 MT3 ER-straight
clamp = "er32"                   # er<size> | shrink | hydraulic | weldon | sidelock | milling_chuck | arbor | drill_chuck
bore = [2.0, 20.0]               # clamping range, mm
gauge_length = 70.0              # gauge line to the nose face
profile = [[0, 50.0], [26, 50.0], [26, 42.0], [70, 42.0]]   # [z from nose, diameter]; z up
arcs = [{ at = 1, r = 2.0 }]     # optional fillet/arc on a profile vertex (exact for the checker)
above_gauge = [[70, 44.45], [135, 25.0]]   # taper/flange, for machine-sim only
max_rpm = 20000
balanced = "G2.5 @ 25000"
source = "..."
taken = "2026-11-23"
note = "nose profile restated from maker catalogue; verify before use"
```

Extensions use the same record with `clamp` and a male interface. The built-in set should be
generic tapers (ISO 7388-1/-2/-3 flange envelopes) plus ER chucks with a conservative nut
envelope, all labelled "generic, conservative". Real holders come from maker imports.

### 5.4 Assembly (in the shop/machine file)

```toml
[[assembly]]
t = 12                            # tool number (T)
h = 12                            # length offset register (defaults to t)
d = 12                            # diameter/radius comp register (defaults to t)
pocket = 12                       # optional
turret = 0                        # lathes: station
tool = "flat-6-3fl-alu"
holder = "bt40-er32-70"
extensions = []
stickout = 24.0                   # LB: nose face to tip; required
gauge = 94.0                      # optional measured value; must match holder.gauge_length + stickout ± tol, else a lint
spindle = "cw"                    # cw | ccw | off (probe), derived from hand by default
coolant = ["flood"]               # allowed modes on this machine
live = false
manual_change = false
life = { minutes = 120, warn = 0.8 }   # optional; tracked outside the file
```

### 5.5 Importers and exporters

| format | direction | priority | why |
|---|---|---|---|
| Fusion `.json` / `.tools` (zip) | import (and export later) | **must-have, phase 1** | largest public pool: 52 vendor libraries from 32 makers [R V2], plus every hobby library on GitHub. Published schema [R F6]. Maps nearly 1:1 to §5.2. Holder `segments` map to `profile`. `post-process` maps to an assembly. `start-values.presets` map to presets (material query is free text). Turning: `SC/INSD/S/RE/RA/TC/SCTY` map to the insert, `THSC/MTP/HAND/CW/LH/W/H` to the holder |
| LinuxCNC `.tbl` | export (import D/Z only) | **must-have, phase 1** | the LinuxCNC post needs it. Trivial |
| FreeCAD `.fctb` / `.fctl` | import and export | **should, phase 1–2** | open, simple JSON. FreeCAD users are the natural audience of an MPL CAD. Map `Diameter→dc`, `CuttingEdgeHeight→lcf`, `ShankDiameter→dcon`, `Length→oal`, `CornerRadius→re`, `CuttingEdgeAngle→angle`, `TipDiameter→tip_d`, `Pitch→tp`, `TipAngle→sig`, `Flutes→nof`, `NeckDiameter→dn`, `NeckHeight/NeckLength→lu` (approximately). Parse quantity strings with units |
| Fusion CSV | import | later | header names are Fusion-internal; JSON is better |
| GTC `.zip` / ISO 13399 `.p21` | import | **differentiator, phase 2–3** | the real vendor-neutral path for inserts, turning holders, indexable mills and holder STEP. Needs a p21 reader (forge-step has STEP parsing) and a class-to-type table (the GTC hierarchy). Use the STEP model to get exact holder profiles by sectioning, which Fusion does not do [R F12] |
| HSM `.hsmlib`, Mastercam `.tooldb`, SolidCAM, NX `.dat` | none | not planned | closed. Users export to Fusion JSON or GTC first |
| Vendor-profile restatements | built-in data | ongoing | design.md's plan: restated, sourced, dated |

### 5.6 Must-have, differentiator, later

- **Must-have (phase 1)**: types flat/ball/bull/chamfer/spot/drill/tap/thread mill/reamer/
  counterbore/countersink/boring head/slot/probe. Holder profile records, assembly records with
  T/H/D, presets by ISO group. The Fusion JSON importer with per-field typed errors. The
  LinuxCNC exporter. The generic built-in set (design.md §6).
- **Differentiators**: (a) exact line/arc profiles for tools and holders, feeding the certified
  collision checker. No commercial system's holder model is exact; Fusion facets arcs into
  frusta. (b) `query F tools` replies listing why a tool cannot do an op (reach = `lu` vs depth,
  neck vs wall, holder vs fixture) with fixes ("use a 30 mm stick-out" as a SetProperty). (c)
  Gauge-length consistency lints: holder + stick-out vs measured gauge. (d) GTC STEP to exact
  revolved profile.
- **Later**: turning inserts and holders (phase 5, ISO 1832 / ISO 5608 decoding), circle-segment
  and form tools beyond 2D, tool blocks for mill-turn, life and wear tracking (machine-side),
  CoroPlus or ToolsUnited online connectors (paid, accounts).

---

## 6. Gaps and cautions

- The ISO 13399 part titles beyond 1–5, 50–80 and 100/150 are from catalogue listings, not from
  the standard. The PLIB link and the PCS/MCS coordinate-system names are [K].
- The internal GTC layout beyond `package_assortment.xml` (folder names, the p21 per item) was not
  verified from the specification. Open a real Sandvik package before writing the importer.
- The taper gauge diameters and pull-stud threads are [K]. Check them against ISO 7388 sample
  pages before any value goes into a built-in profile.
- Fusion's TA semantics conflict (UI help says half angle [R F1]; schema text says included angle
  between generators [R F6]). Test against a real exported chamfer mill. The SpeTool sample has a
  90° V-bit with `TA: 45`, which suggests half angle in the file [S2].
- Fusion `LB` is "length below holder" in current files, not ISO body length [R F6]. Importers
  must not map Fusion LB to ISO LB.

---

## 7. Sources

All read or searched 2026-11-23.

**Autodesk Fusion**
- F1 [R] Milling or Hole Making tool reference (Cutter, Shaft, Holder, Post Processor, Cutting
  Data tabs). https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/GUID-A0197982-0082-45AF-B848-8D62CAF57C49.htm
- F2 [R] Turning tool reference. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/GUID-6C532684-30CC-4C00-B983-ACE28DBFBDDE.htm
- F3 [R] Supported tools reference. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-TOOL-LIBRARY-SUPPORTED-TOOLS.htm
- F4 [R] Import and export tools and tool libraries. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-REF-LIBRARY-EXP-IMP.htm
- F5 [R] Form mill tools. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/FORM-MILL-OVERVIEW.htm
- F6 [R] Autodesk Tooling JSON Schema, `ToolLibrary.schema.json` (draft-07, $id
  cam.autodesk.com/tool-data/RawInputToolLibrary.schema.json, about 2.6 MB), fetched and parsed
  field by field. https://cam.autodesk.com/tools/tools/Tooling%20Schema/ToolLibrary.schema.json
- F7 [R] Work with the gauge length of tool holders and tool assemblies. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-TL-MILL-HOLDER-GAUGE-LINE.htm
- F8 [R] Holders reference. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/GUID-12B90889-DE86-49A8-A1F2-950D7042E590.htm
- F9 [S] Autodesk forum threads on tool lengths and on T-slot/dovetail definition (snippets only;
  the forum returned 403). https://forums.autodesk.com/t5/fusion-manufacture/cam-tool-lengths-flute-vs-shoulder-vs-body-vs-overall/td-p/6271883
- F10 [R] Work with tool presets. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-CREATE-TOOL-PRESET.htm
- F11 [R] Expressions in the Tool Library. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-REF-LIB-EXPRESSION.htm
- F12 [R] Import tools from P21/GTC. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-TOOL-LIBRARY-IMPORT.htm
- F13 [R] Edit tools in a CSV file. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-REF-LIBRARY-EDIT-TOOLS-CSV.htm
- F14 [R] Tool Library overview. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-TOOL-LIBRARY-OVERVIEW.htm
- F15 [R] Types of tool libraries. https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-TOOL-LIB-TYPES.htm
- also [R] Work with tools (renumber, copy/paste holder), Extract a tool holder:
  …/MFG-WORK-WITH-LIB-TOOLS.htm, …/MFG-LIBRARY-EXTRACT-HOLDERS.htm
- V1 [S] RobbJack Fusion download page (slot-mill convention for saws; "version 36").
  https://robbjack.com/downloads/fusion-360
- V2 [R] Tools for Autodesk Fusion vendor index (the JSON behind the page: 52 entries, 32
  vendors). https://cam.autodesk.com/hsmtools and https://cam.autodesk.com/tools/tools/tools.json
- V3 [S] Autodesk App Store listings for Harvey and Helical add-ins; Autodesk blog on vendor
  sections. https://www.autodesk.com/products/fusion-360/blog/finding-favorite-cutting-tools-just-got-easier/

**Sample data (read in scratch, not saved)**
- S1 [R] toolpath/ui-packages `schemas/fusion/digest.json`, a third-party digest of F6
  (retrievedAt 2026-09-12), used only to cross-check per-type key sets.
  https://github.com/toolpath/ui-packages
- S2 [R] Public Fusion tool libraries on GitHub: carlbass/fusion_tool_libraries (Haas ST30SSY,
  turning, version 26), ezpeople/FUSION360 (version 35), rr1706/RRFusionLibrary (version 36),
  jamwaffles/FusionToolTranslator (version 1).
- S3 [R] Fusion holder JSON samples: rahul-sha/Fusion360-Posts "BT40 Holders.json",
  X3msnake/Ouplan-CNC-Fusion-360-Post-Processor tools/HOLDER.json.

**ISO 13399 and GTC**
- I1 [S] BSI Knowledge listings of ISO 13399 parts (dictionaries, principles 100, usage 150).
  https://knowledge.bsigroup.com/products/cutting-tool-data-representation-and-exchange-reference-dictionary-for-tool-items
- I2 [S] ISO 13399-1 catalogue page (403 on fetch). https://www.iso.org/standard/36757.html
- I3 [S] ISO/TS 13399-2:2021 page (403 on fetch; scope snippet names parts 3, 4 and 5).
  https://www.iso.org/standard/81109.html
- I4 [R] Dormer Pramet, "Cutting tool parameters according to ISO 13399" (symbol table).
  https://www.dormerpramet.com/ISO-13399
- I5 [S] ISO/TS 13399-70/-71/-72 listings. https://www.iso.org/standard/66793.html,
  https://shop.standards.ie/en-ie/standards/iso-ts-13399-70-2016-606595_saig_iso_iso_1390366
- I6 [S] ISO/TS 13399-80:2017 (AFNOR, ISO) and [R] BSI page for the withdrawn 2014 edition.
  https://www.iso.org/standard/68886.html, https://knowledge.bsigroup.com/products/cutting-tool-data-representation-and-exchange-creation-and-exchange-of-3d-models-overview-and-principles-1
- I7 [S] GlobalSpec/Accuris listing (13399-201, -301, -302); iteh sample of 13399-314.
- I8 [S] ISO 1832 editions (iteh samples), insert designation structure.
- I9 [R] Wikipedia, ISO 13399. https://en.wikipedia.org/wiki/ISO_13399
- G1 [R] Siemens NX Manufacturing blog, "Generic Tool Catalog revolutionizes cutting tool data
  exchange" (3 Feb 2016). https://blogs.sw.siemens.com/nx-manufacturing/generic-tool-catalog-revolutionizes-cutting-tool-data-exchange/
- G2 [R] Sandvik Coromant, Export GTC package. https://www.sandvik.coromant.com/en-us/support/coroplus-tool-library-support/export-gtc-package
- G3 [R] CRIBWISE help, GTC package import (package_assortment.xml, GTC 2.0). https://cribwise.com/help-center/gtc-package-import/
- G4 [R] gtc-tools.com home page. https://gtc-tools.com
- G5 [R] Sandvik Coromant tool data download. https://sandvik.coromant.com/en-gb/tools/tool-data
- G6 [R] Sandvik CoroPlus ToolLibrary FAQ. https://www.sandvik.coromant.com/fr-fr/support/coroplus-tool-library-support/faq-coroplus-tool-library
- G7 [S] ToolsUnited flyer and home page. https://info.toolsunited.com/wp-content/uploads/2023/09/ToolsUnited-Flyer-EN.pdf
- G8 [S] MachiningCloud "NOVO Phase Out" notice and PR Newswire release (the notice itself was 404
  on fetch). https://www.machiningcloud.com/?p=10490, https://www.prnewswire.com/news-releases/machiningcloud-to-offer-free-subscription-for-existing-novo-customers-302408684.html

**Other CAM systems**
- M1 [R] Mastercam 2026 What's New, Import Lathe GTC Tool Data (first 100k characters). https://my.mastercam.com/whats-new-2026/whats-new-2026_lathegtc/
- M2 [S] Mastercam 2017 Tool Manager guide (scribd) and CloudNC help pages on .tooldb.
- M3 [S] Mastercam, CoroPlus ToolLibrary assemblies in Mastercam 2018. https://www.mastercam.com/?p=9697
- M4 [R] RobbJack Mastercam download page (.tooldb closed; CSV path). https://robbjack.com/downloads/mastercam
- C1 [S] SolidCAM 2021 What's New (ToolKit imports .TAB); UPenn MEAM SolidCAM tool table guide;
  ToolsUnited SolidCAM tutorial.
- C2 [S] Kennametal, How to import tools with SolidCAM. https://kennametal.com/us/en/resources/tutorials/how-to-import-tools-with-solidcam.html
- N1 [R] RobbJack NX download page (tool_database.dat structure). https://www.robbjack.com/downloads/nx
- N2 [S] Siemens support articles on tool_database.dat (HOFF field, conversion). https://support.sw.siemens.com/en-US/okba/PL8541906/tool-library-file-field-for-holder-insertion-distance/index.html
- O1 [R] FreeCAD `src/Mod/CAM/Tools/README.md`, `Tools/Library/Default.fctl`, `Tools/Bit/*.fctb`,
  `Tools/Shape/shape_aliases.json` (main branch, sparse clone in scratch). https://github.com/FreeCAD/FreeCAD/tree/main/src/Mod/CAM/Tools
- O2 [R] FreeCAD `src/Mod/CAM/Path/Tool/` (shape models, toolbit base, serializers fctb/yaml,
  library serializers fctl/linuxcnc), main branch.
- O3 [R] LinuxCNC, Tool compensation and tool table. https://linuxcnc.org/docs/stable/html/gcode/tool-compensation.html
- O4 [R] LinuxCNC 2.9, Tool database interface (db_program). https://linuxcnc.org/docs/2.9/html/tooldatabase/tooldatabase.html

**Holders and standards**
- H1 [S] Hexagon Nexus MachiningCloud page. https://nexus.hexagon.com/home/product/machining-cloud
- H2 [S] DIN Media, DIN ISO 7388-1:2014 replacing DIN 69871-1; iteh ISO 297/ISO 7388-1 samples;
  Kemmler ISO 7388-1 catalogue. https://dinmedia.de/en/standard/din-iso-7388-1/205582777
- H3 [R] Wikipedia, Machine taper (7/24, CAT, BT, NMTB, HSK, R8, Morse, Capto). https://en.wikipedia.org/wiki/Machine_taper
- H4 [S] HSK-A63 listings (ISO 12164-1 / DIN 69893-1 / ASME B5.62; Haimer, Hoffmann shrink and
  Weldon chucks). https://shop.haimer.com/en/Shrink-Fit-Chuck-Standard-Version-DIN-69893-1-HSK-A63/A63.140.25
- H5 [S] Capto C3–C10 sizes and ISO 26623 history (Sandvik catalogue PDF, Thomasnet). https://news.thomasnet.com/fullstory/polygon-shank-coupling-size-100-coromant-capto-c10-added-to-iso-dis-26623-standard-20013075
- H6 [R] Wikipedia, Collet (ER series, DIN 6499, Rego-Fix 1972). https://en.wikipedia.org/wiki/Collet
- H7 [S] Cromwell ER set ranges; Kennametal ER collet pages; Rego-Fix ER catalogue PDF.

**Could not reach** (refused or empty): Autodesk forums (403); iso.org standard pages (403);
ANSI webstore preview PDFs for ISO 13399-1 and -100 (403); Mitsubishi Materials ISO 13399
property page (403); Seco "ISO 13399 standards on catalogs" (JavaScript shell, no content);
Kemmler ISO 13399 page (404); FreeCAD wiki CAM_ToolBit (bot wall, so I used the source
instead); MachiningCloud NOVO notice (404 on fetch); Fusion API ToolLibrary page (404); GTC
working documents (member area, not attempted); `cam.autodesk.com/tools/tools/` directory listing
(403; individual files fetched fine).
