# CAM in forgecad: the design

2026-11-23. Research only: no kernel or interface code was changed. This note is the "how" half of
research/cam/README.md. Repository facts are cited by path, with the branch where a crate is not
yet on `claude/repo-review-mbdrlj`. The prior art behind each choice is in the sibling notes
(commercial.md, opensource.md, literature.md, libraries-*.md, fusion-*.md).

## 0. The idea in one paragraph

forgecad already has what most CAM systems bolt on afterwards:
- an exact B-rep;
- exact offsets of lines, arcs and quadrics;
- a proven boolean;
- certified distance by interval branch and bound through a motion (forge-brake);
- a design history that *knows* its holes, threads, fits, chamfers and fillets;
- typed, agent-readable replies;
- one result on every target.

CAM is then three things:
1. a **tool–part distance and swept-volume engine** in the kernel, which makes every toolpath
   provably gouge-free and collision-free, or refuses it with a name;
2. **toolpath generators** on top of it, 2.5D first, where the geometry is lines and arcs and
   everything is exact;
3. **data and output** in the interface: libraries as our own TOML, posts as data-driven
   dialects, setup sheets, and a `machining` document in .fg.

Nobody else can claim (1). Commercial CAM checks gouges by sampling a mesh at a tolerance
(literature.md §6, §7). This is the differentiator to build first.

## 1. Where things live

The kernel rule: exact results, zero third-party crates, native = wasm bits (README.md). The
precedent is `kernel/forge-brake`. It is an application, a press-brake simulator, yet it lives in
kernel/ because its answers are certified geometry. It depends only on forge-math, forge-ops and
forge-sheet (bendsim branch, kernel/forge-brake/Cargo.toml). Data crates sit in kernel/ too:
forge-material (`data/materials.toml`) and forge-vendor (bendsim/sheet-vendors branches).

| crate | where | what | depends on |
|---|---|---|---|
| `forge-cam-geom` | kernel/ | Tool shapes as revolved line/arc profiles. Exact 2D region offsets of line/arc loops. Swept regions and swept volumes of a tool along a line, an arc or a helix. Cutter-location (CL) computation (drop-cutter, push-cutter) by closed form on planes, cylinders, cones, spheres and tori, and by certified B&B elsewhere. Engagement angle. | forge-math, forge-geom, forge-topo |
| `forge-cam` | kernel/ | Setups, stock and the in-process stock model. Operations as pure functions from (stock, target, tool, params) to a toolpath. The linker. The checks: gouge, holder, rapid-through-stock, fixture and axis limits, certified by forge-brake's `bb` pattern. Cycle time. | forge-cam-geom, forge-bool, forge-ops, forge-mesh, forge-asm (fixtures), forge-material |
| `forge-tooling` | kernel/ | Tools, holders, machines, fixtures and cutting data as data: our TOML, a read-only built-in library, then the user's and the project's libraries, as forge-material and forge-vendor do. | forge-math |
| `forge-post` | kernel/ | The toolpath to NC text through a dialect table. A Lean-modelled G-code subset, and a reader that parses our own output back, for the round-trip proof (§5). | forge-cam |
| CAM in `forge-lang` | interface/ | The `machining` document, its builtins, `query F cam`, the edit ops and fixes, the reply schema `forgecad.cam/1`. | forge-cam, forge-post, forge-tooling |
| `forge-cam-import` | interface/ | Importers: Fusion tool library JSON, GTC packages (ISO 13399 class data), FreeCAD toolbits, LinuxCNC tool tables, Mastercam (only what is documented). The output is our TOML. Importers may use third-party crates (zip, XML) because they live in interface/. | forge-tooling |
| app views | interface/forge-render | Setup, toolpath and stock views, the operation list, the machine. | — |

Why the post is in kernel/: its output is part of what is proven. The NC text must read back to
the motion within a stated rounding (§5 G6), and text must be the same bits in the browser and
native. Numbers are printed from integer-quantised values: each coordinate is rounded to the
dialect's resolution in exact integer arithmetic, then its digits are printed
(libraries-machines-posts-feeds.md §5.3). The output then depends on no formatting library, and a
native-vs-wasm text test pins it.

Why not one `forge-cam` crate: the geometry half is what gets proven and goes through Aeneas
(L4). Keeping it small and free of policy (link order, defaults) keeps that proof tractable. The
same split exists between forge-bool-core and forge-bool (PROOF-STANDARD.md, L4 row of the
boolean).

## 2. What it reuses

| existing piece | where | what CAM uses it for |
|---|---|---|
| Exact 2D offsets: lines, arcs and circles offset to lines, arcs and circles, joined by extension or a corner arc | kernel/forge-dwg/src/draft/offset.rs (410 lines) | The seed of contour offsetting. It lacks region offsetting with self-intersection removal and topology change, which a pocket needs. |
| Wavefront topology events: an edge shrinks away, a face vanishes, a vertex splits | kernel/forge-ops/src/wavefront.rs, research/shell-topology | The same events are the topology changes of a 2D offset front (straight-skeleton / Voronoi events; Held, literature.md §1). |
| Exact offset of every face surface (a plane moves, a cylinder, cone, sphere or torus changes radius) | kernel/forge-ops/src/shell.rs | The 3D offset surface for a ball tool is the part offset by r. The CL surface of a ball end mill is exactly the offset body (literature.md §4), so drop-cutter for a ball is "where a vertical line meets the r-offset body", the face offsets shell already makes. |
| Offset body and its crease graph; the fillet as a morphological closing | research/fillets/general-method.md | Rest machining is the same mathematics. What a tool of radius r cannot reach is the target minus its opening by a ball of radius r (the tool's shape for a ball end). A fillet smaller than the tool shows up as a crease in the offset body: that is where the pencil and rest passes go. |
| The boolean, sequential and n-ary, proven for flat faces, with quadrics in progress | kernel/forge-bool, PROOF-STANDARD.md | The exact reference for stock removal: stock − ⋃ swept volumes. For 2.5D the swept volumes are extrusions of line/arc regions, so this is inside the flat-plus-cylinder domain the boolean already handles. |
| Certified least distance by interval branch and bound over a piece's parameters **and the motion** | forge-brake `bb.rs` (bendsim branch): the `Problem` trait (`may_hold`, `enclose`, `witness`, `sd`, `lower`) | The gouge and collision checker is this trait applied to a new problem: the tool (or holder) moving along a toolpath segment, s ∈ [0, 1], against the part (or fixture, or in-process stock). Verdicts pass, fail or undecided, with undecided refused; this is the bendsim contract (docs/bendsim.md "Exactness"). |
| Certified intersection tracing (Krawczyk step, proven in Lean) | verify/krawczyk (trace branch), forge-geom certify.rs | Waterline: a z-plane's slice of the offset surface; flow lines; the contact curve of a pencil pass. All are traced curves with certified uniqueness per box. |
| Tessellation with a chord bound | kernel/forge-mesh/src/tessellate.rs (`Quality::chord`) | Display of toolpaths and stock. The dexel stock for fast preview. **Never** the safety answer: a mesh's chord error is a tolerance, and the safety answer is certified. |
| Assemblies, joints, interference | kernel/forge-asm | Fixtures and the machine as an assembly: the vise, the plate, the part placed by a joint. Machine kinematics are a joint chain. |
| Material library with sources per value | kernel/forge-material | Cutting data per material group: `cam.vc`, `cam.fz`, `kc1_1` and `mc` as dotted keys, each with its own `source_` (the format already allows `thermo.window`-style keys). |
| Vendor profiles: restated public data, `source`, `taken`, `note`, "verify before ordering" | kernel/forge-vendor/data/*.toml (sheet-vendors branch) | The exact template for tool-maker catalogues: Harvey, Helical, Sandvik and so on, restated with source and date, never copied. Also shop profiles (the machines and tools a job shop has), the CAM twin of a vendor's `[tooling]`. |
| bendsim's typed reply, `forgecad.bendsim/1` | docs/bendsim.md (bendsim branch) | The shape of `forgecad.cam/1`: checks with `rule`, `value`, `limit`, `verdict`, `certified`, `bound`, `at`, `phase`; `blocking[]`; `fixes[]` as edit ops; `suggestions[]` as `NeedTool`. |
| Design history: holes with d, depth, thread, fit; chamfers; fillets with radius; GD&T | forge-lang `hole(...)`, examples/machined/flange.fg, flange_dwg.fg (`fit: "H7"`) | Feature *knowledge*, not feature *recognition*. A `hole(d: 6.6, depth: thru)` is a drill op. An `H7` bore is drill, then bore or ream. A thread is drill, then tap or thread mill. A 2 mm fillet sets the largest corner radius a finishing tool may have. Fusion must recognise these from the B-rep (fusion-advanced.md §6); we read them. Imported STEP still needs recognition, which is the later path. |
| Drawings as a document kind: `drawing` first line, `part("flange.fg", material: ...)` | examples/machined/flange_dwg.fg | The `machining` document follows the same pattern (§3). |
| Persistent names (`e("f1358be8de13ff6a")`), anchors (`base.end`, `hole.bore`) | AGENTS.md | Operations pick geometry by the part's own names, so they survive part edits. This is Fusion's weakest point: its toolpaths go stale and lose selections when the model changes (commercial.md). |

## 3. The .fg surface: a `machining` document

A sketch, not a grammar. Its numbers live in params; its names are the part's own.

```
machining
# flange.fg on a 3-axis VMC in a 6" vise: op 1 top, op 2 flip.
shared stock_xy = 84
shared stock_z = 22
param rough.load = 1.2           # adaptive optimal load (radial engagement), mm
param rough.down = 12
param finish.leave = 0

f    = part("flange.fg", material: "6061-T6")
m    = machine("haas-vf2")                     # forge-tooling: built in, user, project
lib  = tools("shop.tools.toml")                # T-numbers, holders, stick-out
vise = fixture("kurt-dx6", jaw: "soft", open: stock_xy)

op1 = setup(f, machine: m,
            wcs: (origin: f.plate.end, x: +x, z: +z),        # an anchor of the part
            stock: box(size: (stock_xy, stock_xy, stock_z), at: (0, 0, -8)),
            fixture: vise, probe: wcs_probe(corner: "xy-", top: true))

face   = facing(op1, tool: lib.T1, down: 1, leave: 0)
rough  = adaptive(op1, tool: lib.T2, load: load, down: down, leave: 0.3)
bore   = drill(op1, f.bore_h, plan: auto)       # from the hole's d, depth and fit: spot, drill, bore
bolts  = drill(op1, f.bolts, cycle: peck(q: 2))
prof   = contour(op1, edges(of: f.plate, crease: "convex"), tool: lib.T3, leave: leave, comp: "wear")
deburr = chamfer_mill(op1, edges(of: f.boss, crease: "convex"), tool: lib.T6, width: 0.3)

op2 = setup(f, from: op1, flip: x,                           # stock carried over from op1
            wcs: (origin: f.plate.start, z: -z), fixture: vise)
skim = facing(op2, tool: lib.T1, to: f.plate.start)

prog = program([op1, op2], post: "haas-ngc", number: 1001, sheet: true)
```

- An operation is a feature: named, in order, built as it is made, each with status and dims.
  `build`, `check`, `diff`, `edit` and `--at FEAT` work as they do for parts. `--at rough` is the
  in-process stock after `rough`.
- An operation's geometry argument is any selector the part language has. `plan: auto` reads the
  hole's own feature (diameter, depth, thread, fit) and the shop's tools.
- Templates are .fg text: a `machining` file with `part(...)` left as a param, inserted by
  `AddFeature`. No separate template format.
- `fixture(...)` and `machine(...)` instances are forge-asm instances, so `query F interference`
  works on a setup.

## 4. Outputs: `forgecad.cam/1` and the tools

The reply follows bendsim's shape (docs/bendsim.md).

```
ok, schema: "forgecad.cam/1", part, machine, tools_source,
setups[]   name, wcs {origin, axes}, stock {kind, size, from}, fixture, probe
ops[]      name, kind, setup, tool {id, T, H, D, holder, stickout}, status (built|refused|stale),
           feeds {rpm, vc, fz, f, ae, ap, hex (chip thickness after thinning)},
           path {moves, length_cut, length_rapid, time_s, removed_mm3, min_engagement, max_engagement},
           checks[]   rule, verdict (pass|fail|undecided), certified, value, limit, unit, at (WCS),
                      s (where along the path), tool part (cutter|shank|holder|extension), other
                      (part face by name | fixture | stock | machine), phase (cut|link|rapid|plunge)
           leftover {max_mm, where[], why: "tool radius" | "reach" | "undercut" | "rest"}
blocking[]  op, checks[]          fixes[]  typed edit ops (SetArg stickout, SetArg tool, ...)
suggestions[]  {kind: NeedTool, op, why, what: {type, d, r, reach}} | {kind: AddSetup, why}
programs[]  name, post, file, lines, warnings[]
```

The rules a check can carry:

| rule | value vs limit | certified by |
|---|---|---|
| `gouge` | least distance, cutter to target (with the leave) along the cut, vs −tol | B&B over (tool profile, s), as forge-brake |
| `holder_collision` | least distance, holder, shank or extension to the in-process stock, vs clearance | B&B |
| `rapid_through_stock` | least distance, the tool on a rapid to the in-process stock, vs clearance | B&B |
| `fixture_collision`, `machine_collision` | least distance to the fixture or the machine's moving and fixed bodies | B&B |
| `axis_limit` | each joint's travel vs the machine's limits | interval, exact for 3-axis |
| `reach` | the needed flute length and stick-out vs the tool's | closed form |
| `corner_radius` | the tool's radius vs the smallest concave radius it must finish | closed form, from the feature history |
| `engagement` (warning) | max radial engagement, ae or angle, vs the op's load | exact for lines and arcs |
| `leftover` | the largest distance from the final stock to the target-plus-leave, vs tolerance | exact for 2.5D, B&B for 3D |
| `chip_thickness`, `power`, `deflection` (warnings) | vs the tool's and the machine's data | arithmetic on sourced data |

MCP and CLI: `query F cam` (all of it), `query F cam --op rough`, `render F --at rough` (the in-process
stock), `export F prog [-o file.nc]`, `export F setup-sheet`. These are the existing tools with a
new document kind, not new tools. Agents get typed refusals and fixes they can send back
unchanged, as for parts.

## 5. The owner's rules, and the proof standard

### 5.1 Exact geometry, never a wrong result

A toolpath is shipped only when its checks pass with `certified: true`. Otherwise the operation
is refused, with the failing check named. Approximations exist (dexel preview, mesh render), but
no verdict comes from them. This is bendsim's contract, unchanged.

### 5.2 The Lean spec (L1)

Over ℝ³ with regular closed sets. The objects:
- T is the cutter: the cutting part of the tool, the revolved profile up to the flute length.
- N is the non-cutting part: the rest of the tool, the shank, holder and extensions, all
  revolved profiles.
- F is the fixture and machine bodies, in the frame they hold at each instant.
- A path is a continuous map g : [0, 1] → SE(3). For 3-axis it is a translation; for 5-axis, a
  rigid motion. It comes in pieces, each tagged cut, link, rapid or plunge.
- The swept set of a set X along g is Sw(X, g) = ⋃_{s ∈ [0,1]} g(s)·X.
- P is the target part. δ ≥ 0 is the stock to leave and ε > 0 the tolerance. S₀ is the stock.
  S(s) is the in-process stock: S₀ minus the cutter swept up to s.

The obligations for one operation:

- **G1, no gouge.** Sw(T, g) ∩ int(P ⊕ B_δ) = ∅. Equivalently, dist(g(s)·T, P) ≥ δ for every s.
  With a tolerance on the inside (the op's `tol`, Fusion's "tolerance" split into inner and
  outer), it is Sw(T, g) ∩ (P ⊕ B_δ) ⊖ B_{ε_in} = ∅. The tolerance is an explicit part of the
  statement, never a sampling gap.
- **G2, no non-cutting contact.** For every s, (g(s)·N) ∩ int S(s) = ∅ and (g(s)·N) ∩ F(s) = ∅.
  The holder never touches stock, and nothing touches the fixture.
- **G3, no cutting on a non-cutting move.** On every rapid and link piece, (g(s)·T) ∩ int S(s) = ∅.
- **G4, the result.** S_end = S₀ ∖ int ⋃_ops Sw(T, g). Then P ⊕ B_δ ⊆ S_end, which follows from
  G1, and S_end ⊆ (P ⊕ B_δ) ⊕ B_{ε_out} ∪ L. L is the stated leftover: regions the reply lists in
  `leftover.where`, each with its reason (tool radius larger than the concave radius, reach,
  undercut). This is the "stock − swept volume = target within tolerance" of the brief, made
  honest. A one-sided Hausdorff bound, plus the named places where it is not met.
- **G5, the machine.** Every joint value of the inverse kinematics of g(s) is within the machine's
  limits, and the kinematics are continuous along the piece (no flip at a singularity).
- **G6, the post.** Let I be the Lean model of the G-code subset we emit (modal state, G0/G1/G2/G3,
  planes, G90/G91, units, the canned cycles we use expanded to motion; after Kramer's RS274/NGC,
  literature.md §9; Tekriwal & Sottile's Coq/Why3 RS274 semantics is the one mechanised precedent, literature.md §10). Then I(post(g)) = g′, where g′ is g with every coordinate rounded to the
  dialect's digits, and G1 to G5 hold for g′. The obligation is checked on g′, so the rounding is
  inside the proof, not an excuse after it.

What it does not claim: cutting physics (deflection, chatter, tool wear), or that the machine
follows its program. Those stay warnings from sourced data. That boundary is stated in every
reply's schema doc.

### 5.3 The six levels for CAM

| level | for CAM |
|---|---|
| L1 | G1 to G6 above, in Lean. The swept set and Minkowski facts come from Mathlib's convex/metric API; the tool is a solid of revolution. |
| L2 | An exact reference. 2.5D: a flat or bull tool along lines and arcs at fixed z sweeps a body whose z-slices are Minkowski sums of a disk (or annulus) with a segment or arc. These are bounded by lines and arcs, with coordinates in ℚ(√d). The forge-exact Alg/Quad numbers already cover this. Stock removal is then the flat-plus-cylinder exact boolean. 3D: the exact reference is per-primitive closed forms (ball vs plane, cylinder, sphere and so on) in algebraic numbers, sampled exactly at rational s as an oracle for the B&B. |
| L3 | Correctness for every input. The certified B&B is sound: a pass is a proven lower bound, a fail is a real witness. That is the forge-brake argument, which needs its distance error budget proven (docs/bendsim.md says `sd_err` is still an assumption). CAM inherits that open item: prove it once, for both. Refusal happens exactly when a check is fail or undecided. |
| L4 | forge-cam-geom and forge-post go through Aeneas, as forge-bool-core does. forge-cam's planner (which op, which order, the linker) need not be proven correct, only safe: anything it makes passes the L3 checker. This is the forge-brake split: the plan is a search, and the checker is the proof. |
| L5 | The predicates: orient2d and incircle for arcs and lines, which exist, and the interval enclosures, whose ieee754 proof exists. Plus `sd` for each tool profile against each surface type. |
| L6 | Differential: the 2.5D generator against the exact L2 removal on a fuzz corpus of pockets (random line/arc regions, islands, thin walls). The 3D checker against L2 closed forms at rational s. Real corpora: the bench parts, plus NIST's and others' test parts (literature.md). |

### 5.4 No slower

CAM is a new document kind. It never runs in a part's build, so no part's gate line moves. Its own
lines join the speed gate when they land: 2.5D adaptive on the bench pockets, drill planning on
the flange, the checker per 1,000 moves. The design target is interactive regeneration. An
operation regenerates alone, keyed by content hash, and the stock carries forward per op, as
`--at` already does for features. The certified checks cost more than sampling. The B&B clears
most of a path in one cell per move, because a cutting move far from the part bounds itself
immediately. The cost is spent only near contact. forge-brake already runs this way at
interactive speed (bendsim). Measure before promising: the first gate line is the checker's
cells per move on a real adaptive path.

### 5.5 Native = browser, bit-identical

- No threads in wasm. Native may run ops in parallel, but each op is a pure function of its inputs
  and results merge in op order. The n-ary boolean's rule (README.md) carries over.
- Trig only through `forge_math::dmath`. Helices and arcs are evaluated with it.
- No FMA, and no platform math library.
- G-code numbers are printed from integer-quantised values, pinned by a native-vs-wasm text test.
- Feeds and speeds arithmetic uses the same IEEE operations, so the same bits everywhere.

### 5.6 No third-party crates in kernel/

All of the kernel side is our own: the offsets, the B&B, the dialect tables and the TOML subset
reader, which forge-material already has. Importers (zip, XML for GTC, JSON for Fusion) live in
interface/forge-cam-import, where crates are allowed. **No open-source CAM code is ported into
kernel/.** Ideas are learned from it, with licence notes in opensource.md. The
algorithms we need are textbook (Held; drop-cutter; Van Hook), and our versions are exact, which
none of theirs are.

## 6. Libraries: where they live and their format

All are our TOML, in the forge-vendor style: a built-in, read-only set, then the user's
(`~/.config/forgecad/cam/*.toml`), then the project's (`cam/*.toml`), searched in that order.
Every record has `source`, `taken`, `note`, and per-value `source_<key>` where it differs.
Details and the field lists are in libraries-tools.md §5 and libraries-machines-posts-feeds.md §5.

| library | file | built-in content | importers (interface/) |
|---|---|---|---|
| tools | `*.tools.toml`: `[[tool]]` type, geometry (d, r, flute length, shoulder, OAL, taper, tip, number of flutes, hand), shaft segments, material and coating, `[[tool.preset]]` cutting data per material group | a generic set in our own numbers: flat, ball and bull 1 to 20 mm, a drill series, taps in M and UNC/UNF, a spot drill, a chamfer mill, a thread mill | Fusion JSON (`.json` / `.tools`), GTC (ISO 13399 classes), FreeCAD `.fctb`, LinuxCNC `.tbl` |
| holders | `[[holder]]` taper (CAT40, BT30, HSK-A63, ...), revolved profile segments `[[z, d]]`, gauge line | standard tapers and ER collet chucks, as profiles from the published standards' dimensions (restated) | Fusion holder JSON, GTC |
| assemblies | a tool + holder + extensions + stick-out = `T`, with H and D offsets, in the shop's `tools(...)` file | — | Fusion library (it carries holders) |
| machines | `[[machine]]` kind (vmc3, vmc4, trunnion AC/BC, head-table, head-head, lathe, mill-turn), a kinematic chain of joints with limits, spindle (rpm, power curve), tool changer, coolant codes, a link to the post dialect, and optional bodies (`.fg` assembly) for simulation | generic 3-axis, 4-axis and 5-axis trunnion, a 2-axis lathe; no vendor's model copied | Fusion `.mch` if documented; otherwise hand-written |
| posts | `[[dialect]]` a data table: word formats, the safe-start block, tool change, work offsets, arc mode (IJK or R, quadrant split), the canned cycles it supports (or expand), the high-speed mode codes, comments, line numbers, program number rules | Fanuc-generic, Haas, LinuxCNC, GRBL first; Siemens and Heidenhain (ISO) next; Heidenhain conversational later | none: posts are written for us. Fusion's posts are JavaScript under Autodesk's terms (licence notes). |
| cutting data | forge-material's dotted keys: `cam.group` (ISO P M K N S H), `cam.vc`, `cam.fz` by diameter, `cam.kc1_1`, `cam.mc` | per group, from public tool-maker guides, restated with sources | tool-maker data as vendor-style profiles |
| fixtures | `.fg` parts and assemblies (a vise is a part) plus a `[[fixture]]` record: jaw width and open range, its datum | a generic 6" vise, soft jaws as a parameterised part, a fixture plate | STEP via forge-step |
| stock | in the document (`box`, `cylinder`, `tube`, `from: solid`, `from: setup`), plus a `[[stock]]` shop list of bar and plate sizes the planner may choose from | common metric and inch bar and plate | — |

## 7. The phased plan

Ordered so that a usable, proven 2.5D CAM comes first. The riskiest piece is built first.
Estimates are in **session-weeks**: one focused agent session's week of work, with review, as the
repository's branches are run now. They are rough, ±50%.

| phase | what | why in this order | effort |
|---|---|---|---|
| **0. The risk** | Exact 2D region offsetting of line/arc regions with islands, each pass offset from the *original* boundary so every vertex stays degree 2 over the input (literature.md, Implications §1), through topology change: the offset front, with its events from wavefront.rs and Held's Voronoi method. The swept region of a disk along a line/arc path and its exact union. The L1 spec (G1, G3, G4 for 2.5D). The certified checker, a forge-brake `Problem` for a flat, ball or bull tool along a line or arc against faces, and its `sd_err` budget proven. | Everything 2.5D stands on robust offsets. Open-source CAM offsets in floats (libarea) or on an integer grid (Clipper2), never exactly (opensource.md). The certified checker is the claim no one else makes; if it is too slow, the design changes. A go/no-go gate, measured on bench pockets. | 6–10 |
| **1. Usable 2.5D** | The `machining` document; setups, WCS, box/cylinder/from-solid stock. Ops: facing, 2D contour (comp in computer and in control), pocket (offset), slot, trace, engrave (forge-font glyphs exist), 2D chamfer, bore and circular, drilling with the standard canned cycles, and drill plans read from hole features (spot, drill, peck, tap, ream, from `fit`). Leads, ramps (helix, profile), retract planes. Posts: Fanuc-generic, Haas, LinuxCNC, GRBL. The setup sheet (HTML, from the reply). Tool and holder TOML, the built-in set, the Fusion JSON importer. A stock preview by dexel; the exact L2 for checks. | This is what most parts need (job-shop work is mostly 2.5D and drilling: commercial.md). With hole knowledge from the history, it is already better than recognising holes. | 10–14 |
| **2. Adaptive clearing** | Constant-engagement roughing (2D adaptive), built from the exact engagement computation, with a clean-room design, the minimum cutting radius and slot clearing. 2D rest machining from the in-process stock. Feeds and speeds with chip thinning, from the cutting-data library. Cycle-time estimates with the machine's accelerations. | Adaptive is the one 2D op shops pick a CAM for (commercial.md). It needs phase 0's exact stock and offsets. | 6–10 |
| **3. 3-axis surfacing** | Drop-cutter and push-cutter in closed form for planes and quadrics, with certified B&B for tori, sweeps and splines. Ops: 3D adaptive roughing (z-level slices of the part), parallel, contour/waterline (the traced slice of the offset), scallop, pencil (the offset body's creases), steep-and-shallow, flat, and rest finishing from the ball opening (research/fillets). 3D gouge checking on the full part. | It needs the curved boolean's L2 (ROADMAP item 4) for exact stock, so it comes after that matures. Until then the 3D checker is B&B against the part, which needs no stock boolean. | 12–20 |
| **4. 3+2 and 4-axis** | Tool orientation per setup or per op, 4th-axis indexing and wrapping, a trunnion machine's kinematics, axis-limit checks, holder and fixture collision with the machine's bodies (forge-asm). Posts with work-plane codes (G68.2, CYCLE800, PLANE SPATIAL), as dialect rows. | Most "5-axis" job-shop work is 3+2 (commercial.md). The checker is unchanged: g(s) is still a rigid motion. | 6–10 |
| **5. Turning** | A lathe setup; face, profile rough and finish, groove, thread, part-off, drill on centre; insert and holder libraries (ISO insert codes). All 2D and exact (lines and arcs in the XZ plane). | It is cheaper than simultaneous 5-axis and reuses phase 0 wholesale. | 6–8 |
| **6. Simultaneous 5-axis, machine sim** | Swarf, multi-axis contour and flow, tool-axis control (lead/lag, tilt, toward a point or line), collision avoidance by tilting, and a full machine simulation with kinematics. | It is the hardest, and the area where hyperMILL, NX and PowerMill lead (commercial.md). Do it once the checker is proven and fast. | 20–30 |
| **later** | Mill-turn with synchronised channels; probing cycles (WCS probing in phase 1 as post output; inspection later); additive and hybrid; Heidenhain conversational; GTC and ISO 13399 import; on-machine verification. | | — |
| **in parallel from phase 1** | The agent layer: `query F cam`, fixes, `NeedTool` suggestions, plans from the history (`plan: auto`). It is cheap, because it is the same typed reply. It is the second differentiator: an agent can program a part end to end and every path it emits is proven safe. | | with each phase |

Total to a phase-3 CAM that covers 2.5D, adaptive and 3-axis, proven: about 35–55 session-weeks.
The go/no-go is phase 0's checker speed. If certified checking of a 10⁵-move adaptive path takes
more than about a second per op natively, the plan keeps the certificate as a background
`query F cam --certify` and shows `undecided: pending` until it lands, never `pass`.

## 7b. Fixtures, soft jaws and declared removals (2026-11-23, the owner's requirements)

**G2, corrected.** For every instant s of every piece (cut, plunge, link, rapid):
g(s)·(T ∪ N) stays out of the open c-neighbourhood of the fixture, {x : dist(x, F(s)) < c} (the open form, the master's decision 2026-11-23: a graze at exactly c is allowed; Lean: G2, G2open_of_closedForm, 31a7900), with c > 0 the setup's fixture clearance (default 1 mm). The
cutter is kept off the vise as strictly as the holder; G1 protects the part, G2 the fixture.
Together with the stock: (g(s)·N) ∩ int S(s) = ∅ (N clear of the in-process stock; a shank no
wider than the cutter is held to the cutter's rule, §8), and G3 on rapids.

**The vise** is F. A generic vise is a parameterised .fg part (body, fixed and moving jaw: jaw
height, width, open range; plain or soft jaws), placed so the jaws close on the stock. The setup's
`clamp` is the depth the stock sits in the jaws. In the checker the jaws are prism faces (boxes, or
soft jaws' pockets): `fixture_collision` is the certified check of T ∪ N against F ⊕ B_c, the same
verdicts, undecided refused. The part's region inside the jaws (below the jaws' top, between them)
goes into G4's leftover L with reason "held by fixture". An op whose target cannot be reached without
entering F ⊕ B_c is refused with a fix: raise the stock, change the clamp depth, or split the setup.

**Soft jaws.** `soft_jaw(vise, blank: (w, h, t), from: SETUP | PART, grip_depth:, clearance:)` is a
pair of .fg parts (fixed, moving). Each jaw's pocket is the held body's profile at the jaw plane
(the stock, or the result of an earlier setup) offset by the clearance and cut to the grip depth:
pocket = (held ⊕ B_clearance) ∩ {depth ≤ grip_depth}. It is associative (an edit of the part
updates the jaws). Inside corners the jaw tool's radius cannot reach get a relief (a drilled
corner) or are listed as leftover. Machining the jaws is its own setup on the blanks in the vise,
with the same ops, checker (G1–G5) and post. In a setup that uses them, F is the cut jaws and G2
applies unchanged. A seating check: the held part's contact area with the pockets ≥ a stated
minimum, and the pocket no deeper than the part; refused with a fix otherwise. Jaw blanks are a
library of presets (aluminium and steel, generic 6" vise), sourced and dated, "verify with the
vendor before ordering", no logos.

**Declared removals.** Deburr, chamfer milling and thread milling remove material the target may
not show. (1) Preferred: the model carries the geometry (`chamfer()` faces, a modelled `thread()`),
and G1 checks against it. (2) Otherwise a declared, named removal Δ (`edge_break(edges(..), 0.3)`,
`cosmetic_thread(hole.bore, M6)`): an explicit finite region of the part, each with its name; G1 and
G4 use P' = P ∖ Δ, and the reply lists each Δ with what it removed. Never a blanket tolerance. A Δ
lies within a stated distance of the edges or faces it names, or it is refused.

**The checker's statements, as the Lean session formalises them** (verify/forge-cam on
claude/repo-review-mbdrlj-cam-lean):
- body: a ρ-hypograph {z ∈ Z, ρ ≤ w(z)}; sd non-decreasing in ρ (sd_hyp_mono, proven);
- prism faces and planar or vertical moves: Clear{lower} when D ≤ |σ − γ(s)| for all σ, s (D the
  low end of slab::dist) and lower ≤ sdRZ(D, h − z_t) over H − z_t (z_t an interval for a vertical
  move), each lowered face within mu of the true one, the claim being lower − mu;
- G2: fixture_collision at limit = fclear (Clear iff lower ≥ fclear + mu) proves the open form
  exactly;
- G1–G3 from faces need one point of the moved tool outside the set: the tool's top above every
  face checked (ops::certify checks it on every move, else `containment` undecided);
- G4 as S_end ⊆ (P' ⊕ B_{δ+ε_out}) ∪ L, L listed; G6 as read(print q) = normalize(motions q) with
  |quantize − g| ≤ ½·10^−d, absolute coordinates (G90) only, I and J on every arc, X and Y on every
  arc, other axes and F only on change, a block's G only when the motion kind changes and only with
  the block written (the null-move bug the Lean model found, fixed in b6ec16c).

## 8. Decision log

Every CAM decision, dated, with the commit that records or implements it. Newest last. The
build notes are build.md (the design as built), phase0.md (the go/no-go) and phase1.md (status).

| date | decision | by | commit |
|---|---|---|---|
| 2026-11-23 | Crates: forge-cam-geom (kernel, no dependencies, the Aeneas target: exact numbers, regions, offsets, booleans, tools, checker), forge-cam (lowering, section, stock, ops), forge-tooling (TOML libraries, feeds), forge-post (dialects, quantised printing, reader), interface/forge-cam-lang (the machining document, reply, sheet), interface/forge-cam-import (Fusion JSON). forge-cam-geom depends on nothing, departing from §1, so Aeneas can take all of it. | cam session; approved by the master | 8df5c49 (build.md) |
| 2026-11-23 | The B&B is reimplemented in forge-cam-geom to forge-brake's `Problem` contract, sound form (Clear only if the proven bound ≥ limit; Hit only with a real witness; else Undecided, refused). **When bendsim merges (round 6), forge-brake switches to forge-cam-geom's checker: one certified B&B for the kernel.** | cam session; approved, with the plan, by the master | 8df5c49 |
| 2026-11-23 | Exactness: inputs are f64 dyadics; constructed values an expression DAG; a sign from an f64-interval filter, then big-float intervals to the BFMSS separation bound. Offsets: one corner rule (an r-arc at each reflex vertex), events exact, keep tests at rational points. | cam session | 20602cb |
| 2026-11-23 | The checker's float arithmetic rounds outward everywhere (interval extension, no assumed `sd_err` budget). Its correctness against verify/ieee754 is an **L5 obligation**, for the IEEE session (session_01Q19LzyWBpQpFWC3Y7bkDcp). | master | this log |
| 2026-11-23 | .fg: a machining document is a .fg whose first statement is the bare name `machining`; no grammar change; built-ins are abstract IDs. | cam session; approved | 8df5c49 |
| 2026-11-23 | Library formats as libraries-tools.md §5.2–5.4 and libraries-machines-posts-feeds.md §5.2–5.4: one TOML subset reader (forge-tooling/toml.rs); required fields refused with a typed error and a fix; every record `source`, `taken`, `note`; vendor data "verify with the vendor"; dialects as `[dialect]` tables. | cam session | 18b74ba |
| 2026-11-23 | Proof statements G1–G6 as §5.2. In the checker G1 reads min over faces and motion of sd_T ≥ δ − ε_in plus one tool point outside P; each lowered face carries mu (≥ tol::LINEAR) by which a clearance must hold more and a hit fail more. G6: the checks run on motions(quantize(g)), the path the posted text means. | cam session | b1cca26, 5a2d9c8 |
| 2026-11-23 | **Phase 0 go/no-go.** General interval B&B on flange.fg finishing passes: 100–250 ms/move: no-go. The pre-registered alternative, a closed form for prism faces (sd of a ρ-hypograph body is non-decreasing in ρ, so min = sd(D, Δ)): 32 µs/move, 10⁵ moves in 3.2 s, 0 refused: **conditional go**. Ops up to ~3·10⁴ moves certify inline; larger ones in the background as `undecided: pending`, never `pass`. The B&B stays for non-prism faces and non-planar, non-vertical moves. | cam session; accepted by the master | b1cca26 (phase0.md) |
| 2026-11-23 | Hash-consing uses a sorted index (BTreeMap), not a HashMap: no seed, deterministic everywhere, and one Aeneas blocker fewer. | master | this commit |
| 2026-11-23 | The owner's rule "no runtime proof for what is proven": certifying each toolpath against the part stays (it depends on the input). Wherever a property holds for every toolpath an op generates, prove it in Lean (G1–G6) and drop that runtime check. Candidates: the round trip of the post (G6, for every program), the opening clearance of an offset pocket with stepover ≤ r (stock.rs), the in-span arc pieces of the lowering. | master | this log |
| 2026-11-23 | In-process stock for G2/G3 is the box minus voids each op provably cleared (openings for stepover ≤ r, contour bands, drilled circles): it contains the true stock, so a clearance against it is sound. | cam session | b2105d1 |
| 2026-11-23 | forge-lang stays byte-identical to base: the machining document and the hole reading live in interface/forge-cam-lang. Reason: any byte added to forge-lang moves the bench binaries' callgrind counts by tens of instructions (layout), and the gate's strict lines must not exceed base. | cam session | 155e2f2 and before |
| 2026-11-23 | Order of work after Phase 1: Lean G1–G6 and the ρ-monotonicity lemma (blocked here: no Lean toolchain, its install refused by the permission policy). Adaptive clearing after phase 1. | master | this log |
| 2026-11-23 | A shank no wider than the cutter is held to the cutter's rule (within tol of part and stock), not to the clearance: it rides in the slot its own flutes cut, where no clearance is possible. The holder, and any shank wider than the cutter, keep the clearance. G2 is checked against the stock as the op's own completed levels leave it (levels go top-down, so each level's cleared region holds down to its depth). | cam session | this commit |
| 2026-11-23 | Cones and tori are lowered as surfaces of revolution about z (a torus's tube angle measured about its face's middle); they take the general B&B. Ops that no `program(..)` posts are `unposted`, never called certified. | cam session | this commit |
| 2026-11-23 | `pocket(SETUP, from:, to:, tool:)` with no hole named pockets every inner loop (hole, slot) of the part's section between the heights, islands kept; a slot is such a loop (a stadium exactly 2r wide comes out as a there-and-back path). A uniform grid over face boxes picks the faces near each move (the certified clear is still `apart` on each). | cam session | this commit |
| 2026-11-23 | **Open, for the master:** deburring, chamfer milling and thread milling remove material the target model does not show (an unmodelled edge break, a cosmetic thread). G1 as stated refuses them. Proposed: they are allowed only where the model carries the geometry (a `chamfer()` gives cone/plane faces, now lowered; a modelled `thread()`), or with a declared, named removal Δ that G1 and G4 subtract (P' = P − Δ), never a blanket tolerance. Not built until decided. | cam session | this commit |
| 2026-11-23 | G2 corrected: T ∪ N clear of F ⊕ B_c on every piece (fixture clearance c, default 1 mm); vises and custom soft jaws in phase 1; declared removals Δ (P' = P ∖ Δ) for deburr, chamfer and thread milling where the model lacks the geometry (§7b). | owner via the master | this commit |
| 2026-11-23 | The null-move printer bug found by the Lean model of G6 fixed; ops::certify checks the one-point-outside hypothesis (tool top above every face) on every move. | Lean session; cam session | b6ec16c, this commit |
| 2026-11-23 | Vises (G2 corrected, built): `setup(.., fixture: vise(NAME, across:, clamp:, parallel:, clearance:))` places a library vise (forge-tooling `[[vise]]`, generic, "verify with the vendor") with jaws closed on the stock's faces across the clamp axis, gripping `clamp` of it, on two parallels, the bed `jaw_height` below the jaw top. Every piece is a box; the fixture is their union, and every body of the tool, the cutter included, on every phase, keeps the fixture clearance c (default 1 mm) from each piece through the same certified checker (rule `fixture_collision`). Refusals carry a fix: the largest clamp that clears the lowest offending move, or split the setup. G4 splits its bands at the jaw top and names the bands under it "held by fixture". A clamp outside (0, min(stock height, jaw height)], a stock wider than the opening, or parallels that do not fit are refused at the setup. | cam session | this commit |
| 2026-11-23 | Soft jaws (built): `soft_jaw(vise(..), blank:, from: SETUP, grip:, fit:, tool:, relief:)`. The jaws grip what the earlier setup provably left in the grip band (its in-process stock, turned over), not the part's ideal outline: that is what is there, and it contains the part. Pocket M = (G ⊖ r) ⊕ (r + fit), r the jaw tool's radius (so the tool cuts it), plus reliefs of radius r + fit at convex corners (a relief of exactly r is cut only by a plunge at its exact centre). Seating is exact (every band region ⊆ M); contact ≥ min_contact; a 3 mm wall; the part stands more than the fixture clearance above the jaws. The jaws are a part (closed, pocketed) for their own setup, held by the vise's hard jaws and bed, and a fixture for the setup that holds the part. | cam session | this commit |
| 2026-11-23 | A later setup is `setup(PART, from: SETUP)`: the part turned over about x by a proper rotation of the lowered faces (lower::turned, conventions kept by renaming parameters), its stock the earlier one's in-process stock turned over (voids cleared from below up to a ceiling). G4 is decided at the end of each chain of setups. Holes are drilled only in a setup that holds the part the way they open. | cam session | this commit |
| 2026-11-23 | Every region the exact core gives out has its vertices and centres on the 2⁻³² mm grid; a loop within four grid cells is a point (no area; as tool centres, a plunge); a piece whose ends coincide on the grid is dropped. One rule, far below every face's mu (≥ 10⁻⁹). Reason: exact work on rounded inputs leaves sub-ulp slivers and subnormal coordinates (5·10⁻³²⁴ was seen) whose literals put millions of bits into separation bounds (minutes per op). The native = wasm digest is re-pinned (0x0dd2b72f82979ec6) and equal on both. | cam session | this commit |
| 2026-11-23 | Exact meets without ties where the construction knows the answer: neighbours' shared vertex is a meet by identity and the other meet follows rationally (Vieta; the mirror in the line of centres); an offset piece and its corner arc touch only at their shared end (tangent by construction) and are not solved; offset ends are built as vertex + normal so junctions are one expression; boolean takes the same circle by identity. Offset fuzz: 2.3 s → 0.36 s; the bracket's two-setup program with soft jaws: over 10 min → 0.9 s. | cam session | 0a4c189, this commit |
| 2026-11-23 | Leftover regions say why per region: "held by fixture" when the band is under the jaws' top and the region lies within the fixture clearance of a fixture piece, else "uncut". A clear op's region is the stock grown by the tool's diameter, so a strip between part and stock narrower than the tool is cleared from outside the stock. | cam session | this commit |
| 2026-11-23 | Per-check speed, first step (results bit-identical, the digest unchanged): a move's distance to a face's section is computed once per move and face and shared by the tool's bodies (kept with the cutoff it was computed to; a body that needs more recomputes); the tool profile's and the section's point distances compute only the branch they return; a profile segment whose box gap reaches the running upper bound is skipped (it changes neither end of the minimum); a region's ray cast is skipped when the point is outside its box; a distance's candidates live on the stack. The go/no-go line: certification instructions 1.63·10⁹ → 0.97·10⁹ (callgrind, 5000 moves, −40%); 10⁵ moves 2.7 s here. Next: an f64 evaluation with a proven a-priori error bound in front of the interval one (a static filter), since the remaining cost is interval arithmetic itself (≈150 instructions per operation). | cam session | this commit |
| 2026-11-23 | A control decides a full circle from X and Y alone. quantize: an arc meant as less than a whole turn whose end's X and Y round onto its start's is, under a half turn, the line it rounds to (a G1 with its Z: no full helix for a vanishing ramp piece, the Lean session's 7a47f6a), over a half turn a whole turn; each quantised arc carries `full` (meant as a whole turn). The reader sets `full` as a control does (end X, Y equal to start's), so round_trip compares what the control executes, not text. Repro on all four dialects. | Lean session; cam session | this commit |
| 2026-11-23 | soft_jaw also requires nothing the earlier setup left above the part's top inside the blank's footprint (turned over it would sit in the jaws below the seat; softJaw_needs_grip): refused with the facing that fixes it. | Lean session; cam session | this commit |
| 2026-11-23 | Checks on all cores (the owner's rule: wall clock at equal thread counts): ops::certify_fixtured checks chunks of 64 moves with forge_math::par (each with its own scratch; check::Grid keeps no interior state, its caller passes a `Seen`), and merges per move in order replaying the sequential rules (stop at a containment failure, or after the 21st failing check, counts as of that check), so the summary is identical at any thread count (the bracket's reply is byte-identical at 1 and 4 threads; wasm runs 1). The 10⁵-move line: 2.75 s at t1, 1.36 s at t2, 0.75 s at t4 (4 cores here). | master; cam session | this commit |
| 2026-11-23 | L6 against the proven oracle: slab::dist's seg–seg intervals hold the Lean segSegLo exactly on 2000 cases (examples/oracle_segseg.rs at 2ec3c15; Oracle.lean d7d8bab+, run by the CAM Lean session: 0 outside, 832 exact zeros). segSegLo_le_all (44802d3) covers degenerate segments too. seg–arc and arc–arc are proven sound against slab.rs's candidates (DistArc.lean, 75f0606), for arcs under a half turn (debug-asserted in slab::dist, d6ded08). | CAM Lean; cam session | this commit |
| 2026-11-23 | Walls of revolution are lowered in pieces of at most a quarter turn, and Prism::of gives a wall a closed form only when its arc is strictly under a half turn (else the face takes the general branch and bound): every arc slab::dist sees is strictly under a half turn, the precondition of DistArc.lean (debug-asserted). | CAM Lean; cam session | this commit |
| 2026-11-23 | Static filter measured and not adopted: an f64 running-error-bound form of the hottest kernel (point to segment) cut its instructions 13% but certification only 2.5% (callgrind, go/no-go line, t1): on exact f64 inputs an interval operation costs about what a running-error one does. A second, separately proven arithmetic is not worth 2.5%. The wall-clock target is met by the parallel checks (0.75 s at t4 for 10⁵ moves); further single-thread speed would come from fewer evaluations (the B&B cells and the face candidates per move), not cheaper ones. | cam session | this commit |
| 2026-11-23 | Helical ramps (`ramp: DEG` on pocket and clear): each pass is entered by a helix of a quarter of the tool's diameter about its start where that disk lies in the tool centres' region C₀ = R ⊖ r (exact boolean), down from the level above (in air: the disk is in the opening cleared there), else plunged. The checker takes a helical piece as `Move::Helix`: its arc swept over the piece's heights, widened past rounding (a superset of the machine's helix, whose height is linear in the angle); closed form against prism faces (the tip height an interval, as for vertical moves); the branch and bound refuses it (undecided). | master; cam session | this commit |
| 2026-11-23 | Soundness fix found by the ramps: a trim arc's stored radius may differ from its ends' distance to its centre by ~1e-11 (stock.rs's arc pieces take their ends from f64 sin/cos), so a trim was not exactly closed and a ray through that gap flipped parity: Trim::classify answered "surely inside" for a point outside (a jaw block's top face). A trim arc is now its circle's arc plus the radial pieces to its stored ends: a ray within that gap of an end is undecided in that direction, the box test subtracts the gap, and the arc's box holds its ends. Regression test with the real trim. | cam session | this commit |
| 2026-11-23 | Engraving: `engrave(SETUP, PART.SKETCH, tool:, depth:, step:)` follows the sketch's lines, arcs and circles (construction left out) with the tool's tip, on a plane facing +z, in levels. Per the ruling the groove must be in the model: G1 is checked against the part, so a mark whose line runs into a groove's square ends is refused (the V tool's width at depth cuts the end walls; examples/machined/plate_mark.fg stops the line half a groove short). The sketch's frame comes from forge-lang's public query (`frame NAME`, rounded to 1e-9: placement only, the path is certified as posted), its curves from `sketch_view`: forge-lang unchanged. The stock records each line's and circle's band of the tip's radius as cleared from the floor up (a subset of what a V tool clears). | cam session | this commit |
| 2026-11-23 | Thread milling (the declared-removal route of the ruling): `thread_mill(SETUP, PART.HOLE, tool:, length:)` on a hole whose thread is recorded, not modelled. Δ per place: the ring from the bore to the major diameter over the thread's length, listed in the op's `removals` ("cosmetic_thread M6x1 of `m6` at (..): the ring .. from z .. to .."). The op is certified against a target bounding a superset of P ∖ Δ: the bore's wall kept below the band, Δ's outer wall (the bore's sectors at the major radius) and floor ring added, flat faces at the band's heights with Δ's disk taken out (a face that cannot be cut stays whole: never less strict). The stock records the major-diameter disk cleared from the band's bottom. `drill(.., plan: "drill")` leaves the thread. A modelled thread (`thread(hole:)`, helicoidal faces) is refused by the lowering in this phase. examples/machined/block_m6.fg. | master; cam session | this commit |
| 2026-11-23 | Arc pieces' stored ends lie up to g = arc_gap off their circle (trims from stock.rs; cylinder walls' ends evaluated in f64): the end candidates may exceed the truth by g and the radial pieces lie up to g nearer, so slab's low end for an arc piece is lowered by 2g, clamped at 0 (ρ ≥ 0 for the monotonicity lemma), in dist_region and for walls (CAM Lean, Trim.lean 54494d1, option (a)). Digest re-pinned 0x6d4640dca7c17ed1, native = wasm. | CAM Lean; cam session | this commit |
| 2026-11-23 | G5 (axis limits, the master's safety priority): `setup(.., at: (x, y, z))` gives the work offset (the WCS origin's machine coordinates, as G54 holds them; tool length by H). Every posted position, an arc by its circle's box (a superset), is checked joint by joint against the machine's travel in outward-rounded arithmetic; the spindle's Z is the tip's plus the tool's length (stick-out + holder gauge length). A violation is a refusal (rule `axis_limit`) with a fix, and no program is written. An unplaced setup gets the necessary check only (each axis's span within its travel's width) and a warning in the reply that positions were not checked. | master; cam session | this commit |
| 2026-11-23 | Certification status (the master's ruling on unplaced setups): the reply's `certification` is "full" (every check, G5 on every position), "except G5" (released, but a posted setup has no `at:`: G5 checked the spans only) or "refused"; each program says `certified: full | except G5 (unchecked: no `at:`) | no`. `forgecad cam` exits 0 for full, 3 for except G5, 1 for refused. bracket, block_m6 and plate_mark are placed (full); the flange is left unplaced to show the other state. | master; cam session | this commit |
| 2026-11-23 | Product-only speed check (the master's): the go/no-go's three contours as op paths through ops::certify_fixtured (examples/cam_product.rs: the same 4984 checker moves, 15696 body–face checks, cells and verdicts as cam_gonogo), callgrind, t1, both tools: certify_fixtured inclusive 1.182·10⁹ at b7b6981 (before the arc fixes) → 1.120·10⁹ at 07199a4 (−5.3%). The product already computed each body's swept box once per move, so the harness hoist was never in this number; the arc-fix cost is clawed back on the product side alone. | master; cam session | this commit |
| 2026-11-23 | Edge break, option (a), branch and bound only (built; speed open): `edge_break(SETUP, tool:, size: s, at: z)` with a 90° chamfer mill. Δ = the top's band below z' = z − s + d (d the distance inside the top's outline R), listed in `removals`. Target P ∖ Δ: the top eroded to E = R ⊖ s (exact offset), the vertical walls kept below z − s, and per piece of E the chamfer surface down to the piece moved out by s (a trimmed 45° plane per line, with a sharp corner's two planes meeting on its bisector; a cone per arc piece of at most a quarter turn). The path: the axis on R ⊕ c, c = r_tip + 0.2, tip at z − s − 0.2, where the flank is the chamfer surface; every point of R ⊕ c is c from R, so |p − a| ≥ c + d(p) and the flank is never below the surface. Refused: a top that is not the part's top, a face through the band that is not a vertical wall (a rectangle or a vertical cylinder), a sharp corner where an arc meets, a feature narrower than 2s (E loses or splits a loop), a hole the axis path loses. examples/machined/plate_break.fg, 0.3 mm with chamfer-10-90: certified, 140 checks, 644,450 cells (24 plane checks 269k, 38 cone checks 375k, 32k–65k each, under the 200k budget per check), 113 s on one thread (19 moves: one chunk). The same path 0.05 mm into the part is refused (gouge). Reported to the master before any closed form. | master; cam session | this commit |
| 2026-11-23 | Merge-readiness (the owner's priority): origin/main ec32818 merged; fmt and clippy -D warnings clean with no allow attributes (too_many_arguments removed by ops::Cut, the tool, slot, heights and feeds, and ops::Span, the levels; negated comparisons spelt out with NaN still refused; tests without libm or approximate constants); 2188 workspace tests, bits identical, digest unchanged native = wasm, scoreboard 0 worse, fuzz 150/150, callgrind gate all 97 lines equal to a baseline on ec32818. The gate's baseline must be counted from a checkout path of the same length: a longer one moved whole-process lines by ~2k instructions. | master; merge session; cam session | 030ebd9 |
| 2026-11-23 | Closed forms by symmetry (shadow.rs, the master's go-ahead), for a plane with a horizontal direction e and a cone with a vertical axis. The checker's quantity is the least over tool positions A and face points P of sd_rz(|P − A| horizontally, Δz), non-decreasing in its first argument, so a lower bound on |P − A| over a cell bounds it: for the plane √(gap across e² + gap along e²) between the face cell's box and the move's in the frame (n, e); for the cone ||P − o| − |A − o||. The whole move is taken exactly (its line or arc), a piece of it by Move::at's box; the face's slope parameter is split to the resolution first, the move only under a face cell already there. It only proves clearances; whatever it cannot is decided by the branch and bound as before. Exact in the section when the move follows the face's symmetry (an edge break's lines and corner arcs). plate_break: 644,450 cells, 113 s → 105,228 cells, 0.52 s for the whole program; the path moved in 0.05 and 0.02 mm refused (gouge), 0.012 refused (undecided, 0.0015 inside tol), 0.005 certified. L6: tests/shadow_vs_bb.rs, 300 random faces and moves, 474 clearances proven, none contradicted by the branch and bound or by sampling; the edge break's own flank-on-chamfer case proven in under 2000 cells and its 0.05 mm gouge a hit. The product line (cam_product, 5000 moves): 1.119·10⁹ (was 1.120·10⁹). The digest is unchanged. The proof statement went to CAM Lean as a paused note. | master; cam session | this commit |
| 2026-11-24 | 3-axis surfacing is checked through the tool's *column* (its lower profile up to R, infinite above): the gap of a move is the column's least signed distance to a face over the move, which bounds every body inside the column (the cutter, a shank no wider than it; Drop.lean `sd_body_ge_of_column`). Closed forms (planes, vertical walls, spheres, horizontal cylinders, tori about a vertical axis, dome or bowl) first, else interval branch and bound; opt-in route `ops::certify_with(.., column: true)`, `certify_fixtured` unchanged. | CAM-3AXIS | research/cam/phase3.md |
| 2026-11-24 | Z-level roughing is offset clearing of each level's free tool-centre region (passes ≤ the tool's radius apart, passes_cover), not adaptive / constant engagement (§8 patent rule). | CAM-3AXIS | phase3.md |
| 2026-11-24 | Faces outside the 2.5D lowering (spheres, revolved faces trimmed by other curves) are lowered for surfacing as supersets (whole sphere or torus; a cylinder or cone between its edges' heights): sound, stricter. Procedural faces refused by kind. | CAM-3AXIS | phase3.md |
| 2026-11-24 | Paths are made with the column widened 0.2371·tol past its rim, never at exact wall tangency (where the closed footprint makes the drop jump and intervals cannot decide ρ = R); they are certified with the real column. | CAM-3AXIS | phase3.md |
