# Gap tables: every Fusion toolpath against forgeCAD, with a ranked plan

2026-11-24, branch `research/toolpath-quality`. Research only: no kernel or interface code was
changed, nothing committed. Written for David. It builds on README.md (§4 ranked plan),
fusion-benchmark.md (checklist items R/F/S/L/O), roughing-linking.md, feeds-speeds.md,
surface-finish.md, and on ../fusion-workflow-2d-3d.md, ../fusion-advanced.md, ../turning.md,
../phase1.md, ../build.md. Their citations are reused, not repeated.

**Marks.**
- **[R]**: read this session, by me. Code is cited as `path:line` on `main` at 1a1da65 unless a
  branch is named. Fusion turning pages were fetched today (§5 sources).
- **[N]**: taken from a sibling note whose author marked it [R]. I read the note, not the source
  page. Fusion keys such as `[PAR]` resolve in fusion-benchmark.md "Sources" or
  fusion-workflow-2d-3d.md "Sources".
- **[S]** search snippet. **[K]** background knowledge, not re-read. **[D]** derived here.
- Anything not read is UNKNOWN.

**Column legend.**
- **forgeCAD**: `main`, a branch name (not merged), or `absent`, with file evidence.
- **Fusion**: the key parameters, with their source.
- **Best method**: from the literature notes.
- **Metrics**: what to measure and how. Unless a row says otherwise, measure with the harness on
  `cam/quality` (verify/forge-cam-quality; §0.2):
  - *cycle* = `t_stop`;
  - *air* = the `air` %;
  - *load* = φ max, % `thick`;
  - *retr* = retracts;
  - *scallop* = max cusp from a height map against the exact part, later certified (S3).
- **Impact** H/M/L: how much a real user's part, time or tool life changes.
- **Effort**:
  - S: one focused session;
  - M: a few sessions;
  - L: weeks, or a new subsystem.

---

## 0. What forgeCAD has, read in the code

### 0.1 On main [R]

- **Milling ops.** Exposed to `.fg` at interface/forge-cam-lang/src/lib.rs:928: `facing`,
  `contour`, `pocket`, `clear`, `drill`, `engrave`, `slot`, `thread_mill`, `edge_break`. The
  kernel builds:
  - `contour`: kernel/forge-cam/src/ops.rs:600;
  - `pocket` (offset passes, innermost first, optional helix): ops.rs:652;
  - `facing` (zig-zag at 0.7 D): ops.rs:798;
  - `holes` (canned cycles): ops.rs:857.
- **Interface-built ops.**
  - `clear` (a pocket with the part's section as islands): lib.rs:2141.
  - `engrave` / `slot` (along sketch curves; slot ramps along the first line): lib.rs:1163.
  - `thread_mill` (helix in a drilled bore, cosmetic Δ): lib.rs:1441.
  - `edge_break` (45° chamfer of the top edges with a 90° chamfer mill, Δ): lib.rs:1682.
  - Hole plans (spot, drill or peck past 3 D, tap): lib.rs:2386, :2556.
- **Linking is by retract only.**
  - Every loop ends with a `Phase::Link` *feed* move up to the retract height: ops.rs:632, :775,
    :835.
  - The module doc says so: ops.rs:6.
- **Feeds.**
  - `clear`, `contour`, `slot` and `engrave` take their feeds at an assumed ae = 0.1 D;
    `pocket` at its stepover; `facing` at 0.7 D (lib.rs:2106–2110).
  - Plunge = 0.3 × cut feed, and 300/100 mm/min with no cutting data (lib.rs:2111–2114).
  - Power uses kc at the peak chip, `hm = hex.max(1e-4)` (kernel/forge-tooling/src/feeds.rs:126).
    This is README §2 bug 1.
- **Reply time.** length ÷ feed, with rapids at a fixed 10,000 mm/min
  (interface/forge-cam-lang/src/lib.rs:3243, :3263, :3266).
  - forge-machine has a better timeline: `t_min` and an exact-stop trapezoidal `t_stop`
    (kernel/forge-machine/src/sim.rs:15, :193).
  - It also has a height-map removal model (kernel/forge-machine/src/removal.rs:1–7).
  - The reply does not use either.
- **Domain.**
  - The lowering takes planes, and cylinders, cones and tori about z. Other faces are refused,
    `Unsupported` (kernel/forge-cam/src/lower.rs:11, :22).
  - docs/cam.md:127–132: "Not yet: adaptive clearing, 3-axis surfacing, 3+2 … 5-axis".
- **Post.**
  - `CycleKind` = Drill, Dwell, Peck, ChipBreak, Tap, Ream (kernel/forge-post/src/lib.rs:117),
    i.e. G81, G82, G83, G73, G84, G85 (kernel/forge-post/data/fanuc.toml:31–36).
  - No G74/G76/G86/G87/G89 template (grep of fanuc.toml, haas.toml).
  - Decimals: 3 for fanuc, haas and grbl; **4 for linuxcnc** (data/*.toml:6).
    *Correction to README §1.3 and §2.3, which say "3 decimals" for all.*
  - Hard-coded `G187 P2` (haas.toml:16) and `G64 P0.01` (linuxcnc.toml:16).
  - No G41/G42 anywhere in forge-cam-lang or the dialects (grep).
- **Turning** (kernel/forge-turn/src/ops.rs):
  - `face` :196; `rough` (G71-like, expanded) :307; `bore_rough` :395.
  - `finish`: the exact nose-radius offset H ⊕ B_r, posted as the imaginary tip with G40 (:527).
  - `groove` (plunges ≤ 0.8 blade apart) :647.
  - `part_off` (one feed through the centre; CSS cap hard-coded 2000 rpm) :696, :722–725.
  - `thread` (ISO 60° external, radial infeed of equal chip area) :739.
  - `drill` (peck) :800.
  - The default CSS cap is hard-coded at 3000 rpm (:187–191).

### 0.2 On branches ahead of main [R: `git log origin/main..origin/<b>`]

| branch | ahead | what matters here |
|---|---|---|
| `cam/quality` | 1 (aec87afa) | **verify/forge-cam-quality**, a measurement harness over every `examples/machined/*_cam.fg` plus five benchmark parts (pocket_islands, thin_wall, deep_slot, open_step, boss). Per op it reports: forge-machine `t_stop`/`t_min`; height-map engagement φ (max, mean, histogram); climb/conventional/slot time; hex vs fz target (thin/thick %); MRR; air; rapids, retracts, plunges, ramps, corners >90°; Kienzle Fc and power. `tests/pinned.rs` runs the same on native and wasm. **It is README row 1's measuring half, already written**, but not certified and not in the reply. |
| `cam/train3`, `cam/train4` | 12, 15 | peck cycles certify, re-entry as a link (c9cacb9e); GRBL tool change at clearance (2a51df75); offset tie fixes (5e8d0201, 86cfb146); plane half-space bound in `shadow` (e258a867, edge break 105k → 5k cells). No new toolpath strategy. |
| `cam/sim` | 56 (contains train4) | the app's toolpath and stock simulation (`interface/forge-cam-lang/src/sim.rs` header: "Nothing here decides anything"); `Cam::check`; turning UI. No new strategy. |
| `claude/cam-turn-3` | 8 | internal grooving and threading (07ae3717); left-hand tools and `finish_up` (1ed59a72); tailstock and steady rest as fixtures (2df255b5); in-process stock as columns, certify 267 → 31 ms (fcfc16e6, fd35a561). |
| `claude/cam-turn-4` | 13 (contains turn-3) | trigon inserts, left-hand boring bars (e1c4f2a6); real vendor tools, holders and inserts, cited (d748fada, 6e8440b4). |
| `cam/pocket-islands`, `cam/machine`, `cam/path-arc-ends` | 0 | already merged. |

**No branch has 3-axis surfacing, adaptive clearing, stay-down linking or engagement feeds.**
Checked by `git grep -iE "drop.cutter|waterline|scallop|raster|parallel finish"` over kernel/,
interface/ and the harness on cam/sim, cam/train4, claude/cam-turn-4 and cam/quality. The only
hits were in unrelated crates (forge-dwg, forge-step, a bench .fg).

### 0.3 What the harness already measured (cam/quality baseline.txt, 2026-11-24) [R]

1. **Pocket re-entry.** Each pocket ring re-enters from the retract height with a full helix at
   plunge feed.
   - thin_wall `wells`: 355 of 522 s of cutting at plunge feed.
   - Chip under half the target for 70% of the time (well_island 74%, pocket_islands 73%).
2. **Overloaded chip.** `clear`/`contour`/`slot` feeds assume ae = 0.1 D, so the chip is
   1.5–1.7× the target for 55–95% of the time (flange top 92%, contours 91–95%).
3. **Straight plunges.** Into material in `clear`: flange top 24, boss rough 40.
4. **Air.** `clear` cuts air 43–54% of its feed time; facing 10–27%.
5. **Refusal.** `clear` refuses an open step whose wall is one L-shaped face (`NotPrism`).

These numbers ground the "Impact" column below: they are forgeCAD's own paths, measured.

---

## 1. 2D milling

| toolpath | forgeCAD | Fusion offers | best known method | metrics (how) | impact | effort |
|---|---|---|---|---|---|---|
| **Face** | **main**: zig-zag at 0.7 D over the stock box ± (r + 1), levels, retract at feed (ops.rs:798–855) | pass direction, extension, stock offset, stepover (doc "95% D − r_c"), both ways/climb/conventional, *use chip thinning* = roll-on entry, multiple depths + finishing step, extend before retract, transitions [N: FACE-REF, FACE] | one-way climb or zig-zag with **arc turnarounds** outside the stock; stepover 60–75% D so the cutter stays in climb; roll-on entry (no edge impact) (roughing-linking.md §4.4) [N] | cycle; air (10–27% today [R baseline]); hex at the turn; entry impact (φ jump at entry) | M: every job starts with it; air and the sharp reversals cost time | S |
| **2D adaptive** | **absent** (build.md §9; docs/cam.md:130) | optimal load (help: 10–40% D), min cutting radius, slot clearing, both ways + other-way load/feed, stepdown/even, order by depth/area, stay-down level, lift, no-engagement feed, ramps [N: 2DA, SLOTCLR, LIFT] | cleared-area simulation steering to one capped engagement (FreeCAD Adaptive: area per advance ±5%, heading search); engagement certified on the exact cleared region; stay-down links proved inside it (roughing-linking.md §2.4, §6) [N] | load: φ max ≤ cap × 1.10 everywhere incl. corners (R1); slot length (R2); φ ≥ 0.6 cap share (R3); cycle; link length; generation time (R18) | **H**: full-flute depth at low ae is the biggest roughing gain; users expect it | L |
| **2D contour** | **main**: offset by r + leave, inside or outside, at levels; plunge at the start, full loop, retract at feed (ops.rs:600–650). No leads, no G41/G42, no tabs, no finishing passes, no roughing stepovers (grep) | compensation in computer/control/wear/inverse wear, radius allowance, outer-corner mode (roll, sharp, loop), roughing passes, finishing passes, lead-end distance, chamfer mode, tabs, thin wall, both ways [N: CONTOUR-REF, TABS] | tangent lead-in/out arcs that overlap slightly to hide the witness mark; a spring pass; wear comp for the shop (roughing-linking.md §4.4) [N]; corner rounding outside is free (offset arcs) | wall error from deflection (later: feeds-speeds.md §4); witness mark (a lead exists, tangent G1); hex in inside corners (91–95% `thick` today [R baseline]) | **H**: every part has a profile; no wear comp and no tabs block real shops and routers | M |
| **2D pocket (offset)** | **main**: offset passes innermost-first, each loop plunged (or helixed per ring) and retracted at feed; `clear` = pocket with part islands (ops.rs:652–796; lib.rs:2141) | max stepover, allow stepover cusps, morphed spiral, smoothing deviation, finishing passes + overlap, wall taper, min cutting radius, rest machining, all ramp types [N: 2D-POCKET-REF] | the same offsets linked **staying down** across the cut band (proved inside the cleared region), corners rounded to a min radius, one entry per level, climb order (roughing-linking.md §4.1 fix list 1–3, §1 corner formula) [N] | cycle; retr; plunge-feed time (68% today on thin_wall [R baseline]); φ max (127° per 90° corner at 10% stepover [N README §1.1]); air | **H**: the default rougher until adaptive lands; today's linking wastes most of its time | M |
| **Slot** | **main**: an end mill along sketch curves, the tool's width, ramp along the first line (lib.rs:1163–1210; docs/cam.md:63) | backoff distance, repeat finishing pass, multiple depths, tangential extension, ramps, predrill [N: SLOT-REF] | full-width slots in levels (axial ≤ ~0.5–1 D) with feed cut for 180° engagement; or trochoidal / peel slotting for deep slots (roughing-linking.md §2.1) [N] | load (slot 100% of time today, φ = 180° [R baseline slot_plate]); hex vs target (13% thick today); cycle | M | S (levels, feed); M (trochoid) |
| **Trace** | **main as `engrave`**: the tip along sketch lines, arcs and circles at levels (lib.rs:1163; docs/cam.md:64). No axial offset, no side compensation, sketch only (no model edges), plane +z only | axial offset, left/right compensation, 3D curves, multiple depths, repeat, pass extension, max angle, up/down milling [N: TRACE-REF] | a curve followed by the tool centre, with optional offset by the exact curve offset (forge-cam-geom offset) [D] | contour error (certified G1 already); lifts | M | S |
| **Bore (helical)** | **partial on main**: `pocket(SETUP, PART.HOLE)` clears a hole's circle by offset rings with a helix entry (docs/cam.md:61; ops.rs:652). No helical-bore op with pitch and finishing pass | pitch or ramp angle, finishing passes, stepovers, repeat, comp type, lead to centre [N: BORE-REF] | a helix at a constant pitch ≤ the tool's ramp limit, then a full circle at the bottom, then a finish pass (roughing-linking.md §4.3 per-tool limits) [N] | hole size and cylindricity (simulated); cycle; ramp angle ≤ tool max (R17) | M: holes over the largest drill are common | S |
| **Circular** | **absent** as its own op (the pocket of a hole covers it) | depths, XY stepover, repeat, comp, lead to centre [N: CIRCULAR-REF] | ramp-on/ramp-off arcs per pass [N] | as Bore | L | S |
| **Thread mill** | **main**: from a hole's recorded thread; down the bore, in, one helix of the pitch (G3, right hand), out; certified against P ∖ Δ (lib.rs:1441–1450) | manual/automatic definition, hand, pitch, pitch-diameter offset, multi-start, multiple passes, repeat, comp, helical leads [N: THREAD-REF] | **radial passes** (2–3 for steel), helical arc lead-in, climb direction from the hand, comp (wear) for size; OD threads too [K] | thread gauge size (the declared Δ is exact); tool deflection (passes); cycle | M: one pass is often too much for steel and small mills | S |
| **Chamfer / deburr** | **main as `edge_break`**: 45° on the top edges of a part's top only, vertical walls only, a 90° chamfer mill, tip offset e = 0.2 mm (lib.rs:1669–1681). Arcs meeting lines at sharp corners refused | chamfer width, tip offset, clearance from model, include setup model, comp, extensions, leads; 2D Contour chamfer mode; Deburr (extension) for any convex edges [N: CHAMFER-REF, DEBURR] | edges picked by B-rep crease (`edges(crease: "convex")`), any plane facing the tool, any chamfer angle, contact moved up the flank for tool life (fusion-workflow-2d-3d.md §3.11) [N] | chamfer width (exact, Δ); tip wear position; edges missed (count) | M: deburr is on nearly every drawing | M |
| **Engrave (V-carve)** | **absent**: `engrave` follows curves at a fixed depth, no medial axis (lib.rs:1163; build.md §1 lists "trace/engrave" only) | medial line with the tool rising and falling with width; sharp corner angle [N: ENGRAVE-REF] | medial axis (Voronoi of the region) and depth = half-width / tan(α/2) (opensource-oracles.md: Boost Voronoi as an oracle) [N] | width error along the stroke; corner sharpness | L (signage, labels; niche for machining) | M |
| **Drilling cycles** | **main**: plans spot, drill (G81), peck (G83 past 3 D), tap (G84) from the hole feature (lib.rs:2386–2560). Post also has G82 dwell, G73 chip-break, G85 ream (forge-post lib.rs:117; fanuc.toml:31–36), **not reachable from `.fg`** (no ChipBreak/Dwell in forge-cam-lang, grep). Peck certification on **cam/train3/4** (c9cacb9e) | drill G81, counterbore G82, chip-break G73 (peck, reduction, min peck, accumulated depth, break distance, dwell), deep G83, break-through, gun drill, tap G84/G74, left/right tap, tap with chip break, ream G85, bore G89/G86, fine bore G76, back bore G87, control cycles; hole ordering (optimised, by X/Y, depth); retraction avoid-collisions vs preserve-cycles [N: DRILL-REF, DRILL-CYC] | cycles from the hole's own intent (no recognition step); peck schedule shrinking with depth; G73 for ≤ ~4 D, G83 deeper [K]; ordering a greedy + 2-opt over holes (roughing-linking.md §4.2) [N] | cycle (rapid path between holes); chip evacuation proxy (peck count × depth); tap synch (F = pitch × rpm exact) | **H** for chip-break, left tap, ream, ordering (daily operations); L for back bore and gun drill | S (expose G73/G82/G85, left tap G74, rigid tap property, ordering); M (G76/G86/G87 with certified shift and orientation) |

Rigid vs floating tapping: Fusion's current help has no separate "rigid tap" cycle; in many posts
it is a post property [N, fusion-workflow-2d-3d.md §3.12, itself K]. forgeCAD: absent (no M29 or
G84.2 template in any dialect; grep).

---

## 2. 3D roughing

| toolpath | forgeCAD | Fusion offers | best known method | metrics (how) | impact | effort |
|---|---|---|---|---|---|---|
| **3D adaptive** | **absent** | optimal load, min cutting radius, machine cavities, slot clearing, max stepdown, **fine stepdown** (upward), flat area detection, machine shallow areas, order by depth/area, fillets, stay-down level, reduce air cutting, bottom-up stages from flute length [N: 3DA, FLATDET, SHALLOW, AIRCUT] | 2D adaptive per Z level on the exact in-process stock section; deep first step at flute length, then intermediate levels; flats exactly (forgeCAD knows planar faces' z) (roughing-linking.md §3) [N] | R1–R3 load; terrace height (R9); stock on flats (R10); air (R13–14); cycle; generation time | **H** for mould and freeform parts; needs 3D lowering first | L (after 2D adaptive and 3-axis lowering) |
| **3D pocket (offset)** | **partial on main**: `clear` and `pocket` are Z-level offset roughing over a section taken at each op's mid height (lib.rs:2144–2148): exact for prisms only; faces outside the 2.5D domain are refused (lower.rs:22) | stepover max/min, min diameter, morphed spiral, allow cusps, smoothing deviation, fine stepdown, flat area detection, shallow areas, lift height, ramps [N: 3D-POCKET-REF] | per-level exact section of the part (and in-process stock), offset rings linked staying down, flats at their exact heights (roughing-linking.md §3, §4.1) [N] | terrace height; stock on flats; retr; air; cycle | **H** once parts are not prisms; today forgeCAD refuses them | M (per-level section) + L (lowering of freeform faces) |
| **Rest roughing** | **absent** as an option. The 2.5D stock is a box minus proven-cleared voids (kernel/forge-cam/src/stock.rs; docs/cam.md:122–125). `clear` is not restricted to where stock remains; the bracket's `rest` op is a plain `clear` (baseline: 43% air [R]) | from previous ops, chosen ops, tool, file, bodies, setup stock; ignore stock less than; reduce air cutting [N: 3D-POCKET-REF, AIRCUT] | the exact in-process stock section ∩ the region to cut, less slivers under a threshold; regions ordered greedy + 2-opt (Inui 2026: most gain is in not cutting air) (roughing-linking.md §4.2) [N] | air % (R13: ≤ 10% of the cut length); cycle; regions skipped | **H**: every two-tool pocket; air is 43–54% today | M (the stock model exists as voids; the per-level section of the stock is new) |

---

## 3. 3D finishing

**Common prerequisite for every row here: none exists.**
- The lowering refuses any face that is not a plane or a cylinder, cone or torus about z
  (lower.rs:11). docs/cam.md:130 says 3-axis surfacing is not yet built.
- There is no drop-cutter, push-cutter or CL surface anywhere (grep, §0.2).
- So every forgeCAD cell in this section is **absent**. The shared base is:
  1. faces lowered as exact surfaces (or as certified triangulations with an error bound);
  2. an exact drop-cutter for ball, bull and flat tools (surface-finish.md §3.1);
  3. a scallop-driven stepover (S1–S2);
  4. certification of the swept tool (G1 exists for 2.5D; S3 is new).

Effort below is *after* that base, which is itself L (README row 8). Impact assumes the base
exists.

| toolpath | Fusion offers | best known method | metrics (how) | impact | effort |
|---|---|---|---|---|---|
| **Parallel** | pass direction, stepover, horizontal stepover, one way/both ways, machine steep areas + add perpendicular passes, min steep stepover, max steep stepdown, slope limits, up/down milling [N: PAR] | XY rows dropped onto the CL surface (drop-cutter); stepover from the target cusp, exact ball bound s²/(8R) ≤ h ≤ s²R/(8R² − s²); add cross passes where θ > 45° (surface-finish.md §1.1, §1.3) [N] | scallop max and p95 vs target on faces ≤ 60° (S2); steep-face scallop (S3); chord error vs exact surface (S1); cycle | **H**: the first 3D finisher everyone uses | M |
| **Contour / waterline** | min diameter, min cutting radius, max stepdown, machine shallow areas, flat area detection, bottom-up, by islands, thin wall, smooth ramps [N: CONTOUR3D-REF] | push-cutter fibres in x and y woven into loops (OCL), or exact slices of the CL surface; stepdown from cusp on the local slope; shallow insertion (surface-finish.md §3.2) [N] | terrace width on shallow areas (S5); scallop on 45–85° walls; lifts; cycle | **H**: steep walls, moulds | M |
| **Scallop / constant stepover** | inside/outside, link inside to outside, limit stepovers, stepover, machine areas using boundaries [N: SCAL] | 3D offsets of the boundary on the surface (constant stepover); true iso-scallop with the curvature correction, or the length-optimal scalar field (Zou 2020) (surface-finish.md §2.1, §1.4) [N] | scallop uniformity (p95/max ≤ 1.2); offset corner apex spacing (S9); lifts | M: finest shallow finish; harder to make robust | L |
| **Pencil** | bitangency angle, overthickness, number of passes, stepover, order [N: PENCIL] | bitangent CL locus (two contacts) in concave creases, exact on the B-rep's fillet edges (forgeCAD knows the fillets); extra offsets either side (surface-finish.md §2.1) [N] | residual in creases after finishing (S14); lifts | M | M |
| **Steep and shallow** | threshold angle, overlap, wall clearance, smooth offsets, remove cusps at junctions, continuous (spiral), constant cusp stepdown, parallel optimisation, own smoothing (fit arcs or evenly spaced points) [N: SS, SS-SMOOTH, SS-PAROPT] | split at θ* with overlap; contour on steep, parallel or scallop on shallow; with stepover = stepdown the crossover is 45° [D in surface-finish.md §1.3] [N] | max scallop over all slopes ≤ target (S7–S8); junction marks (S9); lifts (S10) | **H**: the one finisher that fits most real 3D parts | M (given Parallel and Contour) |
| **Spiral** | spiral mode (spiral, spiral + circles, concentric), direction, inner/outer limit, stepover [N: SPIRAL] | an Archimedean spiral in XY dropped on the surface; spacing grows on slopes (s/cos θ) [N surface-finish.md §2.1] | scallop vs slope; lifts (none in one spiral) | L: domes and round cavities only | S |
| **Radial** | angular step, angle range, inner/outer radius, min stepover, machine straight on [N: RADIAL] | spokes dropped on the surface; add spokes as the radius grows (scallop grows outward) [N] | outer-edge scallop; centre overlap | L | S |
| **Morphed spiral** | stepover, direction, inside/outside [N: MSPIRAL] | a spiral interpolated between boundary offsets; boundary-conforming spirals by scalar-field optimisation (Shen et al. 2025, arXiv:2512.22502, reports +14% efficiency) [N surface-finish.md §2.1] | scallop uniformity; lifts; G2 continuity | M: smooth organic shallow areas with no stepover marks | M |
| **Flat** | types offset/parallel/profile, open pockets, spiral, stepover, finishing pass, multiple depths, over/into holes, rest machining, detection limit, extension; inclined flats (extension) [N: FLAT] | **exact**: forgeCAD knows which faces are planes ⟂ z and their z; clear each with a flat or bull at that z by the 2.5D pocket (h = 0 for s ≤ 2R_f) (surface-finish.md §1.2) [N] | floor stock = 0 ± tol on every flat (S16); air | **H** for prismatic parts (most of them) | **S**: the 2.5D pocket at each planar face's z, with the part above as islands, mostly exists (`pocket` from/to, docs/cam.md:62) |
| **Ramp** | max stepdown, min diameter, min cutting radius, shallow areas, bottom-up, contact point boundary [N: RAMP] | a waterline whose levels are joined by a continuous descent, so the tool never leaves the cut [N] | lifts (zero per region); scallop as Contour | L (brittle materials, hard milling) | S (given Contour) |
| **Horizontal** | stepover, morphed spiral, allow cusps, depths, lift, plunge outside stock [N: HORIZ] | superseded by Flat [N fusion-workflow-2d-3d.md §4] | as Flat | L | S |
| **Rest / corner finishing** | Corner (extension): reference tool, detection limit, steep/shallow pass types (along, across), priority, cusp; rest sources on finishers [N: CORNER, CORNER-CHOICE] | region = {CL_ref − CL_tool > δ}, a sign set of two certified drop-cutter functions; split steep/shallow (surface-finish.md §2.1, exact version [D]) [N] | leftover after both tools ≤ tol (S15); air (only where the ref tool could not reach) | M | M |
| **Blend** | between two drive curves over drive surfaces, offset passes, stepover from tip or on surface, entry/exit passes, 5-axis option [N: BLEND] | morph between curves; the scallop must be checked, not assumed [N] | scallop max (must be measured); lifts | L | M |
| **Project** | axial offset, depths, rest material options [N: PROJECT] | a 2D curve dropped onto the surface: Trace on the surface [N] | groove depth along the curve | L | S (given drop-cutter) |
| **Flow** | U or V of the faces, stepover, cusp, extension; multi-axis (extension) [N: FLOW] | iso-parametric on the **exact** NURBS (forgeCAD owns it), spacing still from the cusp bound (surface-finish.md §2.1) [N] | scallop uniformity (iso-parametric bunches); lifts | M: blades, fillets, single-face blends; a differentiator on an exact kernel | M |
| **Morph** | morph mode, contour offset, stepover or passes count, extension [N: MORPH] | interpolation between chained curves; check scallop [N] | scallop max | L | M |

Not in the task list but in Fusion's 3D menu: Deburr (row "Chamfer/deburr" above), Geodesic (passes
by true surface distance [N: GEO-OV]; heat method, Crane et al. 2017 [N surface-finish.md]),
3+2 Roughing (§4 tool orientation).

---

## 4. Common machinery

| item | forgeCAD | Fusion offers | best known method | metrics (how) | impact | effort |
|---|---|---|---|---|---|---|
| **Linking** | **main**: retract to the retract height *at cutting feed* after every loop, rapid across, plunge (ops.rs:6, :632, :775, :835); no stay-down, no minimum retraction, no high-feed G1 mode | retraction policy full/minimum/shortest; high feedrate mode (6 kinds); allow rapid retract; keep tool down, max stay-down distance; stay-down level; lift height; transition types; safe distance [N: 3DA, STAYDOWN, LIFT, TRANS] | stay down on a straight or arc link whose swept disc lies in the exact cleared region (a proof, not a check); else minimum retraction; G0 retracts; microlift at floor + δ certified against the in-process stock; dog-leg G0 either certified or emitted as G1 (roughing-linking.md §4.1) [N] | retr (count); link length; vertical feed time; cycle (L1–L5, L10–L12) | **H**: the largest time sink today (§0.3 item 1) | M |
| **Leads and ramps** | **main**: helix of radius D/4 per *ring* in `pocket`/`clear` when `ramp:` is given and the disk fits (ops.rs:686–760); ramp along the first line in `slot` (lib.rs:1197–1210); otherwise straight plunges; ramp angle ≤ 30° for every tool (lib.rs:2101–2105); no lead arcs, no per-tool ramp limits (forge-tooling has no ramp field; grep) | horizontal/vertical lead radii; ramp types (outside, zig-zag, predrill, profile, plunge, smooth profile, helix), ramp angle, taper, max stepdown per rev, ramp clearance, helix diameter and minimum [N: 2DA, 3DA] | one entry per level (not per ring); per-tool `ramp_max_deg`, `helix_d_min/max`, centre-cutting flag, refused by the checker like G5; predrill from the `holes` op; outside entry for open regions; tangent lead arcs (roughing-linking.md §4.3–4.4) [N] | plunges into material (count); ramp angle ≤ tool max (R16–R17); plunge-feed time | **H**: tool breakage and the 68% plunge-feed time | S–M |
| **Rest material** | **partial on main**: the 2.5D in-process stock is the box minus proven-cleared voids, used for G2/G3 safety (stock.rs; docs/cam.md:122–125); G4 leftover listed with reasons (phase1.md gaps 2). Not used to restrict where an op cuts | previous ops, chosen ops, tool, file, bodies, setup stock; ignore-less-than; reduce air cutting [N] | the exact stock ∩ region per level; the same body drives rest roughing and the engagement meter (roughing-linking.md §5) [N] | air %; cycle | **H** (with Rest roughing above) | M |
| **Smoothing and arc fitting** | **main**: offsets are already lines and arcs (G2/G3 out), so 2.5D needs no fitting; no fitting exists for polylines (none to fit yet); output decimals 3 (fanuc, haas, grbl) or 4 (linuxcnc) (data/*.toml:6) | smoothing off by default, fit arcs within a tolerance, ≥ 50% block reduction; S&S evenly spaced points [N: SMOOTH, SS-SMOOTH] | biarc fitting with re-certification; constant-arc-length redistribution; finishing posts at 4 decimals; the control's tolerance mode from the op's budget (surface-finish.md §4.3–4.5, §5) [N] | blocks/s vs the control's block rate (O2); fitted-arc deviation (S17); facet marks (cross-pass point alignment, S19) | M now (2.5D is arcs already); **H** with 3D finishing | S (post decimals, tolerance line from the op); M (fitting) |
| **Feed optimisation** | **absent**: one cut feed and one plunge feed per op, at an assumed ae (lib.rs:2106–2114); power at the peak chip (feeds.rs:126) | geometric corner slowdown only: max direction change, reduced-feed radius and distance, reduced feed, inner corners only [N: FEEDOPT] | per-move feed so that h_max ≤ h* from the real engagement on the in-process stock (solid-model feed scheduling, feeds-speeds.md §6), capped by power, torque and deflection at interval upper bounds; corner engagement per feeds-speeds.md §2.3 (README rows 2–3) [N] | hex vs target (thin/thick % per op, harness); spindle power ≤ curve; cycle | **H**: chips 1.5–1.7× target today (§0.3 item 2); this is the headline "better than Fusion" | S (choose ae from the op; hm fix) → M (per move) |
| **Tool orientation / 3+2** | **absent**: setups are +z only, or the part turned over about x (`from:`, docs/cam.md:55) | per-op tool orientation and WCS; 3+2 roughing with automatic orientation (extension) [N fusion-advanced.md §1.1] | a per-op frame posted as a tilted work plane (G68.2 / CYCLE800 / PLANE SPATIAL); automatic indexing = an exact visibility cover (fusion-advanced.md §1.1 MUST/DIFF) [N] | faces reachable per orientation; setups saved; G5 travel per frame | M (5-axis owners); L for 3-axis users | M |
| **Stock awareness / in-process stock** | **main**: exact 2.5D stock as voids, per setup, carried across a turned-over setup (stock.rs; docs/cam.md:55); forge-machine height-map removal from the posted program (removal.rs:1–7); the harness uses a height map (cam/quality) | automatic IPS; roughers stock aware, finishers not [N: IPS, 3D-OV] | exact stock section per level for roughing; height map (or dexels) for measurement and 3D; certified superset for safety (as today) (roughing-linking.md §5) [N] | air; φ from the stock; stock-vs-final gap | **H** as the base of rest, adaptive and feed rows | M |
| **Machining-time estimate** | **main**: length ÷ feed and 10,000 mm/min rapids in the reply (lib.rs:3243–3266); a better exact-stop trapezoid exists in forge-machine (sim.rs:15) and is used by the cam/quality harness only | feed + rapid + tool change, feed scale [N: MTIME]; users report 2–10× errors on 3D [S] | trapezoid now, jerk-limited next (Ruckig lower bound), FIR interpolator model (Ward et al. 2021) later (feeds-speeds.md §7) [N] | estimate vs Ruckig bound (never below); vs a measured run when available (O5–O6) | M: quoting and planning depend on it | S (switch the reply to forge-machine `t_stop` with `t_min` as a bound) |
| **Multi-axis outlook** | **absent** | swarf, multi-axis contour/finishing/roughing, rotary, tool-axis control (lead/tilt, through point/curve), collision tilt and retract (extension) [N fusion-advanced.md §1.3–1.6] | tool-axis control as smooth fields; collision avoidance by tilting on the exact kernel; certified 5-axis sweep is open research (surface-finish.md §6) [N] | gouge (certified); axis reversals; kinematic singularities | L now | L |

---

## 5. Turning

Fusion parameters in this section were fetched today, 2026-11-24 [R]. They are listed under
"Sources (new)".

| toolpath | forgeCAD | Fusion offers | best known method | metrics (how) | impact | effort |
|---|---|---|---|---|---|---|
| **Face** | **main**: passes ≤ ap from the bar's front to Z, nose centre to the centre line (forge-turn ops.rs:196) | no geometry needed; front offset; outside→inside or inside→outside; multiple passes, finishing passes [R: TURN-FACE-READ] | CSS with a max rpm from the machine; a finish pass at a lower ap [K] | face flatness/finish (nose-radius feed mark h = f²/(8 r_ε), [K]); cycle | L (works today) | S |
| **Profile rough** | **main**: axial passes (G71-like, expanded) at every ap and every cylinder of the part, to H ⊕ B(r_ε + leave), retract at 45° (ops.rs:300–307; turning.md "ops"); ID via `bore_rough` (:395) | vertical / horizontal / back-cutting cycles; groove suppression; undercut options; **even depths of cut**; max depth of cut; **skip wall pass**; **angled entry** (gouge-checked); **use pecking** to break long cuts; machine multiple regions [R: TURN-ROUGH-READ, TURN-ROUGH-STEPS] | constant ap with even depths, a chip-breaking interruption (peck) on long passes in gummy materials, angled entry, roughing that follows the profile at the end of each pass (G71 type II) [K] | cycle; chip length proxy (uninterrupted pass length); insert load (ap × f vs insert data) | M | S (even depths, angled entry, pecking) |
| **Profile finish** | **main**: the nose centre on the exact offset of the part by the nose radius; posted as the imaginary tip with G40 (ops.rs:527; turning.md). `finish_up` for flanks facing the chuck on **claude/cam-turn-3** (1ed59a72) | front-to-back (default, "climb"), back-to-front, both ways; holder/insert clearance for undercuts; groove suppression [R: TURN-FINISH-READ] | exact nose-radius compensation in CAM (forgeCAD's; certified), or G41/G42 on the programmed contour with wear in the control [K] | profile error (certified exact); Ra from the feed per rev (f²/(8 r_ε), [K]) | M | S (direction options) |
| **Groove** | **main**: plunges no wider apart than 0.8 blade, the first and last at the walls; exit at feed (ops.rs:647–690). ID grooving on **claude/cam-turn-3** (07ae3717). No pecking, no finish pass | OD/ID/face, tool orientation for angled grooves; roughing + finishing; **max grooving stepover**; **use pecking** [R: TURN-GROOVE-READ] | plunge, then a finish pass down each wall and across the floor; peck for deep grooves; the last plunge offset so the walls are cut by the blade's side [K] | wall finish; chip evacuation proxy; blade load | M | S |
| **Part-off** | **main**: one feed from outside to past the centre and back out (ops.rs:696–736); CSS cap hard-coded 2000 rpm (:722–725). No pecking, no reduced feed near the centre, no edge break | no geometry; **edge break type** (chamfer or fillet on the back edge); **use pecking**; finishing passes; distance below inner radius; **use reduced feed and speed** before the end [R: TURN-PART-READ, TURN-PART-STEPS] | reduced feed over the last ~1–2 mm to the centre (the blade sees rising load and falling speed); peck in gummy steels; rpm cap from the machine/part [K] | blade load near the centre (vc → 0); pip size; cycle | M: broken blades are the common turning crash | S |
| **Thread** | **main**: ISO 60° external, radial infeed of equal chip area, declared Δ (ops.rs:739–760). Internal threading on **claude/cam-turn-3** (07ae3717). No flank/modified-flank infeed, no first-pass depth, no spring pass, no G76/G92 cycle output (turning.md gaps 7) | OD/ID, cylindrical and conical; manual/automatic/standard definition; right/left hand; **infeed mode**; **first pass depth**; **spring pass**; **use cycle** (canned) [R: TURN-THREAD-READ, TURN-THREAD-STEPS] | modified flank infeed (3–5° under the flank) for less chatter and better chips; decreasing depths (constant chip area, as now); spring pass; G76 for small programs [K] | thread size (Δ exact); chip area per pass constant (exact); program length | M | S (first pass, spring pass, flank infeed); M (canned G76 with read-back certification) |
| **Drill / bore on lathe** | **main**: drill on the centre line with pecking (ops.rs:800–838); `bore_rough` + `finish` inside (turning.md gaps 2) | Drill (shared with milling; cycles as §1) [N: DRILL] | peck schedule; G74/G83 on the lathe; boring passes with a finish pass and a spring pass [K] | as milling drill rows | L (works) | S |
| **Chip breaking / feed interruption** | **absent** in rough, groove and part-off (they are single continuous feeds; ops.rs:307, :647, :696). The drill pecks | "use pecking" on profile roughing, groove and part [R: pages above] | periodic interruption (a short retract or dwell every L mm) or a feed oscillation where the control has it (LFV, [K]); otherwise insert choice [K] | longest uninterrupted feed in material (mm); chip-length proxy | M (bird-nesting in soft steel and aluminium on unattended lathes) | S |
| **Insert nose-radius comp** | **main, done exactly**: the nose centre on H ⊕ B_{r_ε+leave}, imaginary tip, G40 (ops.rs:527; turning.md). No G41/G42 output for in-control wear compensation (grep: no G41/G42 in forge-post data) | compensation types on turning ops: UNKNOWN (not read today; the 2D Contour types are in-computer/in-control/wear/inverse wear [N: CONTOUR-REF]) | CAM comp exact (forgeCAD's) **plus** a wear-comp mode (G41/G42 with the computed path) so the operator can adjust size [K] | size after wear offset; certification of the comp path for a radius range (Fusion's "compensation radius allowance" idea [N]) | M: shops expect to hold size with wear offsets | M (certify for a radius interval, not a point) |

Also missing against Fusion turning [R: TURN-OPERATIONS list in the TOC]: Turning Adaptive
Roughing, Groove Roughing/Finishing as separate ops, Single Groove, Turning Chamfer, Turning Trace,
and part handling (sub-spindle grab and return, bar pull). Also missing: a CSS max rpm from the
machine instead of the hard-coded 3000/2000 (ops.rs:187–191, :722–725). G5 refuses a cap above
the machine's (ops.rs:1224), so the constants are safe but arbitrary.

---

## 6. Ranked plan: top 15 by user impact

Ranked by user impact per unit of effort. "README §4 row" refers
to the ranked plan in README.md.

| # | item | why (evidence) | depends on | README §4 row |
|---|---|---|---|---|
| 1 | **Merge the cam/quality harness and put its numbers in the reply**: a per-op `quality` block, and cycle time from forge-machine `t_stop` (with `t_min`) instead of length ÷ feed with 10,000 mm/min rapids | Every other row needs a yardstick. The harness exists (1 commit, aec87afa) and already exposed the four biggest faults (§0.3). The reply's time is a crude bound (lib.rs:3263). | forge-machine (on main) | 1 |
| 2 | **Stay-down and minimum-retract linking, G0 retracts, one entry per level** in `pocket`/`clear`/`contour` | Retract-at-feed after every ring (ops.rs:632, :775). Plunge-feed time is 68% of cutting on thin_wall. Retracts 12–50 per op [R baseline]. Largest time loss, no adaptive needed. | #1 to measure; exact boolean (forge-cam-geom, on main) for the link-in-cleared proof | 4 (+7 for entries) |
| 3 | **Feeds from the op's real width of cut + the hm power fix** | `clear`/`contour`/`slot` assume ae = 0.1 D (lib.rs:2109), so chips run 1.5–1.7× target for 55–95% of the time [R baseline]. Power reads 12–18% low (README §2). A small change with a large tool-life effect. | none | 2 |
| 4 | **Per-tool entry limits and entry choice**: ramp_max, helix diameter range, centre-cutting flag, refused like G5; predrill from `holes`; outside entry for open regions | Straight plunges into material (24–40 per `clear` op [R baseline]); a 30° cap for every tool (lib.rs:2103) is wrong for most. | forge-tooling fields (new, cited) | 7 |
| 5 | **2D contour completeness**: tangent lead arcs, finishing/spring passes, roughing stepovers, wear compensation (G41/G42 with the computed path, certified over a radius interval), tabs | The most common op. It lacks the shop basics (§1 row). Wear comp and tabs are table stakes [N fusion-workflow-2d-3d.md §3.5]. | dialect templates for G41/G42 | new |
| 6 | **Hole cycles exposed and ordered**: G73 chip-break and G82 dwell (already in the post), ream G85, left tap G74 and a rigid-tap property, a peck schedule; greedy + 2-opt hole order; merge cam/train3/4 peck certification | Daily ops. The post has G73/G82/G85 but `.fg` cannot reach them (§0.1). | cam/train3/4 merged | new (ordering part of 9) |
| 7 | **Per-move feed from the real engagement** (GE 1987 method) on the in-process stock, capped by power, torque and deflection, with the proven chip bound | Removes corner overloads (+46% chip in a 10 mm corner [N feeds-speeds.md]). The headline "better than Fusion". | #1, #3; stock section per level (#10) | 3 |
| 8 | **Corner rounding / minimum cutting radius** on offset passes, engagement checked | 127° → about 52° spikes at 10% stepover [N README §1.1]; the achievable feed is √(a·ρ). | #1 (φ certified), #2 | 5 |
| 9 | **Flat finishing on planar faces** (exact, by the existing 2.5D pocket at each plane's z) and **rest roughing from the in-process stock** | Flats are exact in forgeCAD and cheap. Rest `clear` cuts 43% air (bracket `rest` [R baseline]). | stock section per level | 10 (rest); new (flat) |
| 10 | **3-axis base**: lowering of freeform faces, exact drop-cutter, certified sweep; then **Parallel** with a scallop-driven stepover | forgeCAD refuses every non-prismatic part today (lower.rs:22). The biggest *capability* gap. Ranked under #2–#9 only because it is L. | lowering (L); OCL oracle in verify/ | 8 |
| 11 | **Contour (waterline) + Steep-and-Shallow**, then pencil and reference-tool rest finishing | Covers most real 3D parts once Parallel exists. | #10 | 8 |
| 12 | **Finishing output**: per-op decimals (4 for finishing), the control tolerance mode (G187/G64/CYCLE832/Cycle 32) from the op's tolerance, constant-arc-length points, arc fitting with re-certification | `G187 P2` and `G64 P0.01` are hard-coded (haas.toml:16, linuxcnc.toml:16). Matters most with #10. | #10 for fitting; post decimals now | 6 |
| 13 | **Turning quality pack**: CSS cap from the machine; part-off reduced feed and speed near the centre, pecking and edge break; groove pecking and a finish pass; thread first-pass depth, spring pass, modified flank infeed; chip-break interruption on long rough passes; merge claude/cam-turn-3/4 | Each item is in Fusion [R today] and missing here (§5). Part-off is the common lathe crash. | claude/cam-turn-4 merged | new (turning.md gaps 7 for cycles) |
| 14 | **Ordering**: greedy + deterministic 2-opt/Or-opt over regions, levels and holes | Air time. Inui 2026: most of the gain comes from greedy + not cutting air [N]. | #1, #9 | 9 |
| 15 | **2D adaptive clearing**: one capped engagement bound certified on the exact cleared region, open one-direction peel passes, links through the cleared area, deterministic | The largest single roughing gain (full-flute depth at low radial width). When it lands, #1/#2/#7 make it better than Fusion's. | #1, #2, #7, stock per level | 11 |

Just outside the top 15:
- 3+2 per-op frames (M);
- Thread mill radial passes (S);
- Chamfer on any convex edge (M);
- Bore op with pitch and finish pass (S);
- V-carve (M);
- Spiral/Radial/Morphed spiral/Flow/Blend/Morph/Project (after #10);
- 3D adaptive (after #10 and #15);
- multi-axis (L).

---

## 7. Hand-off packets (start today)

Each packet is self-contained. It names its files, an acceptance metric and its tests. Every
packet must keep:
- the certification gates (G1–G6 pass on all `examples/machined/*_cam.fg`);
- native = wasm bits (kernel/forge-cam/tests/bits.rs);
- no third-party crates in kernel/;
- the callgrind speed band (phase1.md "Gates").

### P1. The quality harness into main, and an honest time in the reply (rank 1)
- **Files**:
  - merge `origin/cam/quality` (verify/forge-cam-quality/*; forge-cam-lang read-only
    accessors);
  - interface/forge-cam-lang/src/lib.rs:3212–3266: replace the 10,000 mm/min rapid and
    length ÷ feed with forge-machine's timeline on the posted program (`t_stop`, with `t_min`
    as the lower bound);
  - docs/cam.md "The reply": add a `time: {t_min, t_stop}` and a `quality` block (retracts,
    plunges, air %, φ max/mean, hex max vs fz target, thin/thick %), labelled "measured (height
    map), not certified".
- **Acceptance**:
  - for every example, `t_min ≤ reply t_stop`;
  - the reply's numbers equal the harness's;
  - baseline.txt unchanged.
- **Tests**: verify/forge-cam-quality/tests/pinned.rs (as is); a forge-cam-lang test that the
  `quality` block exists and that `t_min ≤ t_stop`.

### P2. Stay-down links and one entry per level in `pocket`/`clear` (rank 2)
- **Files**: kernel/forge-cam/src/ops.rs:652–796 (`pocket`); :600–650 (`contour`, G0 retract);
  forge-cam-geom boolean.rs (link containment).
- **Method**:
  - at each level, after ring k, link to ring k+1's start by a straight (or arc) move;
  - emit it only when the link's swept disc of radius r lies in the region cleared so far at
    that level (exact `boolean::Diff` empty, as the helix test at ops.rs:690 does);
  - otherwise retract to the minimum safe height;
  - retract with `Rapid`, not `Line … Phase::Link` at cut feed;
  - helix or ramp only for the first ring of each level.
- **Rule**: fixed and deterministic: stay down when the link is inside the cleared region, else
  retract.
- **Acceptance** (harness): thin_wall `wells` and well_island `well`:
  - plunge-feed time −80% or better;
  - retracts per level ≤ number of disjoint regions;
  - `t_stop` −30% or better;
  - all ops still certify.
- **Tests**:
  - a unit test that every emitted link lies in the cleared region (re-checked by the exact
    boolean);
  - re-pin pinned.txt with the before/after in the commit message.

### P3. Feeds from the real width of cut, and the power fix (rank 3)
- **Files**:
  - interface/forge-cam-lang/src/lib.rs:2106–2110 (`feeds_for` ae per kind): `clear`/`pocket`
    use their stepover; `contour` uses leave + finishing width (or the roughing stepover);
    `slot` uses ae = D; `engrave` uses the tip;
  - kernel/forge-tooling/src/feeds.rs:120–131: hm = f_z·(cos φ_in − cos φ_out)/(φ_out − φ_in),
    kc at hm; correct the module doc (k is the peak-chip factor).
- **Acceptance**:
  - harness `thick` ≤ 10% on every `clear`/`contour`/`slot` op;
  - power at ae = 10% D within 1% of the closed form (feeds-speeds.md §11 worked example).
- **Tests**:
  - feeds.rs unit tests for slot (φ = π), ae = 0.1 D and ae = 0.5 D against hand-computed hm;
  - the existing `thinning_and_ball_diameter` test still passes.

### P4. Per-tool entry limits (rank 4)
- **Files**:
  - kernel/forge-tooling/src/lib.rs (tool record: `ramp_max_deg`, `helix_d_min`, `helix_d_max`,
    `centre_cutting`; required for end mills, sourced, "verify with the vendor");
  - the built-in data;
  - forge-cam-lang lib.rs:2101 (drop the blanket 30° cap; refuse a ramp above the tool's
    maximum with a typed fix giving the largest allowed);
  - ops.rs:686 (helix radius from the tool's range, not D/4).
- **Acceptance**:
  - no straight plunge with a non-centre-cutting tool, anywhere;
  - harness `plng` = 0 where a ramp fits.
- **Tests**:
  - a refusal test (ramp too steep → typed error + fix);
  - one test for each of helix, line ramp and plunge choice.

### P5. Hole cycles reachable from `.fg`, and ordering (rank 6)
- **Files**:
  - merge cam/train3/4 (peck certification);
  - forge-cam-lang `plan_holes` (lib.rs:2386–2560): `cycle: "chipbreak"|"dwell"|"ream"`,
    `peck:` schedule;
  - forge-post: G74 left tap and a dialect flag for rigid tapping (M29 / G84.2: read each
    control's manual first; UNKNOWN today);
  - a greedy + 2-opt order over the places, deterministic ties (exact squared lengths).
- **Acceptance**:
  - each cycle posts, reads back (G6) and certifies on all four dialects;
  - flange bolt circle rapid length ≤ the input order's.
- **Tests**: kernel/forge-post/src/tests.rs round trips per cycle; forge-cam-lang tests for the
  plan choices.

### P6. Turning quality pack, the S items (rank 13)
- **Files**:
  - kernel/forge-turn/src/ops.rs:187–191, :722–725 (CSS cap from the machine record);
  - `part_off` :696 (reduced feed over the last 1–2 mm before ρ = 0; optional pecking with a
    retract in its own groove as a feed, which turning.md shows is needed; optional back-edge
    chamfer);
  - `groove` :647 (peck; finish pass down the walls);
  - `thread` :739 (first-pass depth, spring pass, modified-flank infeed: the tool path offset
    along the flank by tan(30° − α) per pass).
- **Acceptance**:
  - every new path certifies (G1–G5) on the stepped shaft (tests/job.rs);
  - thread chip area per pass is constant within 1% (except the spring pass);
  - the part-off feed in the last zone equals the set fraction.
- **Tests**: kernel/forge-turn/tests/job.rs and refuse.rs (a spring pass on a thread with no
  leave, a pecking retract with no room); posted read-back on the three lathe dialects.

### P7. Finishing output in the post (rank 12, the S half)
- **Files**:
  - kernel/forge-post/src/lib.rs (dialect `decimals` per op class, `finish_decimals`);
  - data/haas.toml:16 and linuxcnc.toml:16 (`G187 P{p}` and `G64 P{tol} Q{q}` as templates
    filled from the op's tolerance; add Siemens CYCLE832 and Heidenhain Cycle 32 only when a
    dialect file exists for them).
- **Acceptance**:
  - a finishing op posts at 4 decimals on fanuc and haas;
  - the G64 P equals the op's tolerance budget split (surface-finish.md §5);
  - the round trip (G6) is still exact.
- **Tests**: forge-post tests.rs: digits at k = 4, and template fill.

### P8. 3-axis spike: exact drop-cutter and Parallel on one face kind (rank 10, first step)
- **Files**:
  - new module kernel/forge-cam/src/drop.rs;
  - lower.rs:11 (admit spheres and tori about z first: closed-form drop-cutter for a ball
    against a sphere, plane, cylinder about z, and torus about z);
  - verify/: OCL drop-cutter as the oracle (opensource-oracles.md §5: it already matched a
    closed form to 4.4e-16).
- **Acceptance**:
  - drop-cutter equals OCL within 1e-9 mm on a hemisphere and a torus;
  - a Parallel pass over a hemisphere certifies G1;
  - the height-map scallop ≤ the target from s²R/(8R² − s²).
- **Tests**: closed-form vs drop.rs unit tests; an OCL differential test in verify/
  (separate-process, LGPL stays out of kernel/).

---

## Sources (new this note)

Fusion turning help, fetched 2026-11-24 [R] at
`https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/<ID>.htm`:

| ID | title (paraphrased) | used for |
|---|---|---|
| TURN-ROUGH-READ | Turning Profile Roughing strategy | cycles, groove suppression, even depths, max depth, skip wall pass, angled entry |
| TURN-ROUGH-STEPS | Generate a Turning Profile Roughing toolpath | multiple regions, use pecking, angled entry on the Linking tab |
| TURN-FINISH-READ | Turning Profile Finishing strategy | directions, holder/insert clearance |
| TURN-GROOVE-READ | Turning Groove strategy | max grooving stepover, pecking, OD/ID/angled |
| TURN-PART-READ, TURN-PART-STEPS | Turning Part strategy / steps | edge break, pecking, finishing passes, cut below inner radius, reduced feed and speed |
| TURN-THREAD-READ, TURN-THREAD-STEPS | Turning Thread strategy / steps | definition methods, hand, first pass, infeed mode, spring pass, use cycle |
| TURN-FACE-READ | Turning Face strategy | direction, multiple and finishing passes |
| toctree.json | help TOC (https://help.autodesk.com/view/fusion360/ENU/data/toctree.json) | the turning op list and IDs |

Not read: TURN-COMMON-FEATURES (only 626 characters came back, a stub); turning compensation
types (UNKNOWN); the Turning Adaptive pages (fusion-advanced.md F28 has them).

Code and branches read this session: the files and commits cited inline; `git ls-remote origin`
for the branch list; `git log origin/main..origin/<b>` for cam/train3, cam/train4,
cam/pocket-islands, claude/cam-turn-3, claude/cam-turn-4, cam/sim, cam/quality, cam/peck-cycles,
cam/machine, cam/path-arc-ends; `git show origin/cam/quality:verify/forge-cam-quality/{README.md,baseline.txt}`.
