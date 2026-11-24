# Open-source CAM: prior-art survey for forgeCAD

Research only. Read 2026-11-23 unless noted. Source marks: **[R]** page read directly,
**[S]** search-result snippet only, **[K]** known from background, not re-read this session
(treat [K] as a lead to verify, not a citation).

Constraints this survey is judged against: forgeCAD is MPL-2.0; kernel crates take zero
third-party crates; results must be bit-identical native vs wasm. So the questions per project
are (a) what algorithm is worth knowing, (b) may we read it, port it, or link it, and
(c) where it could live: anything ported or linked goes in `interface/`, never `kernel/`.

## 1. Summary table

| Project | Does | Lang | Licence (SPDX) | Verdict for forgeCAD |
|---|---|---|---|---|
| FreeCAD CAM (ex-Path) workbench | Full 2.5D/3D CAM, posts, tool bits, sim | Python + C++ | LGPL-2.1-or-later (FreeCAD) | Read for ideas; port only via LGPL nuance (see 3) |
| FreeCAD `libarea/Adaptive.cpp` | Engagement-limited adaptive clearing | C++ | LGPL-2.1-or-later | Read for ideas; reimplement clean |
| libarea (Heeks) core: `Area.cpp`, kurve | Pocket offsets, curve/arc ops | C++ | BSD-3-Clause | Can port with attribution |
| libactp (Freesteel) | Early adaptive clearing | C++ | GPL (version unverified) | Ideas only; do not port |
| HeeksCNC / HeeksCAD | CAD/CAM front end | C++/Python | BSD-3-Clause (HeeksCAD confirmed; HeeksCNC likely) | Can port with attribution once file headers are checked |
| OpenCAMLib (OCL) | Drop-cutter, push-cutter, waterline | C++ (+Py, wasm) | LGPL-2.1 (from Aug 2018; earlier GPL-3.0) | Read for ideas; reimplement |
| OpenVoronoi | Segment Voronoi, medial axis, offsets | C++ | LGPL-2.1 (from Jul 2018; earlier GPL-3.0) | Read for ideas; reimplement |
| Clipper2 (C++) / `clipper2-rust` | Polygon booleans and offsets | C++ / Rust | BSL-1.0 | Can port with attribution; link in `interface/` |
| cavalier_contours | Offsets of line+arc polylines | Rust | MIT OR Apache-2.0 | Can port with attribution; link in `interface/` |
| Kiri:Moto (Grid.Space) | Browser slicer + up-to-4-axis CAM | JavaScript | MIT | Can port with attribution |
| PyCAM | 3-axis toolpaths from STL/DXF/SVG | Python | GPL-3.0 (-only vs -or-later unverified) | Ideas only |
| Fabex (ex-BlenderCAM) | Blender CAM add-on | Python | GPL-3.0 ("unless otherwise indicated") | Ideas only |
| LinuxCNC | Machine controller, RS274/NGC interp, kinematics | C/C++/Python | GPL-2.0 + LGPL-2.1 mix | Ideas only (dialect reference) |
| CAMotics | 3-axis G-code cutting simulation | C++ | GPL-2.0-or-later (README "2+") | Ideas only |
| GRBL | Microcontroller G-code subset | C | GPL-3.0 | Ideas only (dialect reference) |
| bCNC | GRBL sender, autolevel, simple CAM | Python | GPL-2.0 | Ideas only |
| dxf2gcode | 2D DXF to G-code | Python | GPL-3.0 | Ideas only |
| jscut | Browser 2D CAM (pocket, v-carve) | JS | GPL-3.0 (via mirror; upstream unverified) | Ideas only |
| Estlcam | Hobby CAM + controller | Win | Proprietary (paid, unlimited trial) | Not open source |
| MakerCAM | Flash 2D CAM | AS3 | Defunct; licence unverified | Historical only |
| CAMlab | — | — | Not found | No evidence it exists as an OSS project |
| `gcode` crate | `no_std` G-code parser | Rust | MIT OR Apache-2.0 | Can link in `interface/` |
| `g-code` crate | Parse + emit G-code | Rust | not verified (see 9) | Check before use |
| truck | Rust B-rep kernel | Rust | Apache-2.0 | No CAM |
| Fornjot | Rust B-rep kernel (discontinued) | Rust | 0BSD | No CAM |
| PrusaSlicer / CuraEngine | Additive slicers (reference only) | C++ | AGPL-3.0 [K] | Ideas only |

## 2. FreeCAD CAM workbench (formerly Path)

**What it does.** The CAM workbench is the most complete open-source CAM. The mirror of the
FreeCAD wiki lists [R] (https://reqrefusion.github.io/FreeCAD-Documentation-html/wiki/CAM_Workbench.html,
2026-11-23; the live wiki.freecad.org was behind an Anubis bot wall):
- 2.5D ops: Profile, Pocket Shape, Helix, Adaptive, Slot, Drilling, Engrave, Deburr, Vcarve,
  Mill Facing, Thread Milling.
- 3D ops: 3D Pocket, 3D Surface, Waterline; 3D Surface and Waterline still flagged experimental.
- Tools: ToolShape -> ToolBit -> ToolBit Library -> ToolController (feeds/speeds bound to a
  bit). Since release 26.3 more ops honour the bit's shape; before, most assumed a flat endmill.
- Posts: the job stays in a machine-independent internal path format until Post Process; posts
  are per-controller scripts, default metric, flags like `--inches` for LinuxCNC.
- Checks: Sanity Check (missing values in a job), Inspect (raw path, mm/s units), SetupSheet
  (expressions for default op properties, e.g. step-down from tool diameter).
- Simulators: a new CAM Simulator since 1.0, plus the legacy one.
- Dressups: Array (1.1), Axis Map, Boundary, Dogbone, Drag Knife, Lead In/Out, Mirror (26.3),
  Ramp Entry, Tag (holding tabs), Z Depth Correction (probe map).
- The CAM development roadmap names the toolpath libraries: libarea/patharea + Clipper, OCCT,
  OpenCAMLib [S] (https://wiki.freecad.org/CAM_Development_Roadmap, 2026-11-23).

**Adaptive.** The task brief credited "Kirill Kokotov"; the source says otherwise. The file
header of `src/Mod/CAM/libarea/Adaptive.cpp` reads `SPDX-License-Identifier: LGPL-2.1-or-later`,
`Copyright (c) 2018 Kresimir Tusek` [R]
(https://raw.githubusercontent.com/FreeCAD/FreeCAD/main/src/Mod/CAM/libarea/Adaptive.cpp,
2026-11-23). The wiki credits kreso-t / FreeCAD_Mod_Adaptive_Path as the origin [R]
(https://reqrefusion.github.io/FreeCAD-Documentation-html/wiki/CAM_Adaptive.html, 2026-11-23).
No source connected Kokotov to it.

How it works, from reading the code [R, same URL]:
- Works in integer coordinates and keeps a `ClearedArea` polygon set: every tool sweep is
  unioned in; windows around the tool are cached for fast local queries.
- `CalcCutArea` estimates how much new material a tool disc at a candidate position would take,
  by sweeping vertical slices through circle/polygon crossings and integrating the inside spans.
- An angle search (±π/4 about the current heading) interpolates on the error between that
  area and a target area, so each step holds the cut area near the target. This is "engagement
  as removed area per step", not an explicit engagement-angle computation.
- Clipper2 does all offsets, booleans, simplification and point-in-polygon; Z-callbacks on
  vertices mark where finishing passes are needed.
- User knobs [R, wiki]: helix entry (angle, cone angle, diameter limit, emit as G2 arcs),
  step-over as % of diameter, tolerance (speed vs accuracy), force inside-out, stock to leave,
  keep-tool-down ratio. Known limits: not all tool shapes honoured; stray paths, worked
  around by a 0.001 mm stock-to-leave.

**3D Surface / Waterline** use OpenCAMLib when installed ("OCL Drop Cutter"), or an
"Experimental" waterline built on FreeCAD's own Path.Area [S]
(https://wiki.freecad.org/CAM_Waterline, https://wiki.freecad.org/OpenCamLib, 2026-11-23).

**Maturity.** Broad, used in production by hobbyists and small shops; 3D ops remain
experimental; quality varies per op [S][R as above].

**Learn.** (1) Separate the machine-neutral path from the post: forgeCAD's `.fg` features
should produce a typed, unit-carrying path IR, and posts should be pure functions of it.
(2) SetupSheet-style defaults are expressions, which maps onto forgeCAD params. (3) Sanity
Check is exactly the shape of forgeCAD's `check`: typed errors with fixes. (4) Dressups as
composable transforms of a path (tabs, lead-in, ramp, dogbone) fit forgeCAD's feature model.

## 3. libarea, libactp, HeeksCNC

- **libarea** (Dan Heeks): pocketing/profile library with Python module [R]
  (https://github.com/Heeks/libarea, 2026-11-23). The README says it derives from kbool
  (Klaas Holwerda, GPL) [R]; per a search snippet Dan Heeks rewrote it onto Clipper to escape
  kbool's GPL [S] (https://www.pro-linux.de/cgi-bin/DBApp/jump.cgi?ID=20807, 2026-11-23). The
  FreeCAD copy of `Area.cpp` carries `SPDX-License-Identifier: BSD-3-Clause`, "Copyright 2011,
  Dan Heeks" [R] (https://raw.githubusercontent.com/FreeCAD/FreeCAD/main/src/Mod/CAM/libarea/Area.cpp).
  The kurve code (Geoff Hawkesford) was confirmed New BSD in a 2012 email exchange reproduced
  in the project notes [S] (same pro-linux page). Note the `Adaptive.cpp` that sits beside it is
  LGPL, so licence is per file in that directory.
- **libactp**: C library of the adaptive clearing algorithm shown in Freesteel; the repository
  describes it as GPL [S] (https://github.com/Heeks/libactp-old, 2026-11-23). Freesteel (Julian
  Todd, Martin Dunschen) claim adaptive clearing from ~2004 and Todd later worked on HSMWorks
  [S] (https://www.practicalmachinist.com/forum/threads/adaptive-clearing.298696/, forum claim).
  Value to us: published 2004-2006 work on engagement-limited clearing.
- **HeeksCNC**: CAM add-on to HeeksCAD; Zig Zag via OCL, Pocket via libarea [S]
  (https://github.com/90PlusWorks/heekscnc). HeeksCAD is New BSD [S]
  (https://github.com/Heeks/heekscad/wiki, https://launchpad.net/heekscad); HeeksCNC's own
  licence was not confirmed. Largely dormant [K].

## 4. OpenCAMLib (OCL)

[R] https://github.com/aewallin/opencamlib (2026-11-23).
- Algorithms: **drop-cutter** (lower a cutter at fixed XY until it touches a triangle mesh;
  gives raster/zig-zag 3D finishing) and **push-cutter** (push a cutter along a horizontal
  "fiber" at fixed Z; intervals are woven into waterline contours) [R]. Under the hood [K]: a
  kd-tree over triangles, per-cutter closed-form contact tests against vertex, edge and facet,
  and a "weave" graph to stitch fiber intervals into loops.
- Cutters: CylCutter (flat), BallCutter, BullCutter (torus), ConeCutter, CompositeCutter [R].
- Bindings: C++, Python (`opencamlib` on PyPI, 3.9-3.14 wheels), JS/wasm via Emscripten
  (`@opencamlib/opencamlib`, "not 100% feature complete") [R].
- Licence: "From August 2018 OpenCAMLib is released under LGPL license"; repo metadata says
  LGPL-2.1 [R]. Debian marks it LGPL-2.1-or-later and notes the GPL-3 era blocked shipping with
  FreeCAD [S] (https://metadata.ftp-master.debian.org/changelogs/main/o/opencamlib/stable_README.Debian).
  Exact "-only" vs "-or-later" should be checked in COPYING before any use.
- Maturity: the reference open implementation of mesh 3-axis finishing; ~1,260 commits,
  CI, docs [R].
- **Learn.** The cutter-vs-triangle contact math per cutter class is the core of 3-axis
  finishing and is textbook (Chuang/Yau-style "drop" and "push" contacts). forgeCAD should
  reimplement it in its own Rust from the papers, on exact B-rep where possible (drop a cutter
  onto analytic faces, not just tessellation), so results stay deterministic. Do not port OCL
  code into `kernel/`.

## 5. OpenVoronoi

[R] https://github.com/aewallin/openvoronoi (2026-11-23). Incremental topology-oriented 2D
Voronoi for point and line-segment sites (Sugihara-Iri / Held); arc sites "being worked on";
offset generation and an approximate medial-axis filter; C++ with Boost.Python. LGPL-2.1 since
July 2018 (was GPL-3.0). Few releases; slow activity.

**Learn.** Voronoi-based offsets are exact and give the medial axis for V-carve, trochoidal
slot centrelines, and "pocket from the middle". Held's papers (and his VRONI book) are the
reading; topology-first robustness with floats is relevant to bit-identical output because it
keeps combinatorics consistent even when numbers wobble.

## 6. Clipper / Clipper2 and cavalier_contours

- **Clipper2** (Angus Johnson): polygon booleans and offsets (miter/square/bevel/round joins).
  Licence BSL-1.0 [K; the port below states the same]. A pure-safe-Rust port `clipper2-rust` (MatterHackers) claims a direct port
  of `clipper.offset.cpp` and declares `license = "BSL-1.0"` [S]
  (https://docs.rs/crate/clipper2-rust/1.0.3). The `clipper2` and `clipper2c-sys` crates are
  FFI wrappers, MIT OR Apache-2.0 for the wrapper [S] (same search). Clipper is integer-based:
  that is a strength for bit-identical results across native and wasm.
- **cavalier_contours** (Buck McCready): Rust; polylines with true arc segments (no
  linearisation), parallel offsets on open, closed and self-intersecting polylines, booleans;
  `license = "MIT OR Apache-2.0"`; v0.7.0, MSRV 1.88 [S]
  (https://docs.rs/crate/cavalier_contours/0.7.0, https://lib.rs/crates/cavalier_contours).
- **Learn / reuse.** Both licences are MPL-2.0 compatible (permissive; keep notices). Either
  could sit behind a trait in `interface/` today. For `kernel/` (zero third-party crates) the
  path is a clean in-house offsetter informed by both: arc-native offsets
  (cavalier's approach) matter for G2/G3 output and for avoiding chord error before posting.
  Floating-point determinism of cavalier_contours under wasm was not checked; Clipper's
  integer core sidesteps that.

## 7. Browser and hobby CAM

- **Kiri:Moto** (Grid.Space, Stewart Allen): browser slicer + CAM up to 4 axes, local
  processing, MIT [S] (https://grid.space/about.html, https://github.com/GridSpace/grid-apps,
  2026-11-23). Ops include roughing, outline, contour (X or Y raster over organic surfaces),
  pocket (from selected faces), trace [S] (https://docs.grid.space/kiri-moto/CAM/ops). Uses
  slice-then-offset on meshes [K]. JavaScript, actively developed. **Verdict:** the one
  full-featured CAM whose code we could legally port (with notice); also the closest analogue
  to forgeCAD's wasm target and a model for in-browser, no-cloud CAM UX.
- **jscut** (Todd Fleming): browser 2D CAM (pocket, profile, v-carve) on Clipper; GPL-3.0 per
  a mirror [S] (https://gitee.com/mirrors_cheton/jscut); upstream LICENSE not read.
- **MakerCAM**: Flash 2D CAM, offline since its domain lapsed [S]
  (https://forums.maslowcnc.com/t/makercam-is-no-more-what-now/15655).
- **CAMlab**: no search hit for an open-source CAM of that name; treat as unconfirmed.
- **Estlcam**: not freeware - €49 / $59 licence with unlimited free trial [S]
  (https://www.estlcam.de/estlcam.php). Proprietary.
- **dxf2gcode**: 2D DXF to G-code, Python, GPL-3.0 [S]
  (https://sourceforge.net/projects/dxf2gcode/, Debian README).

## 8. PyCAM, Fabex (BlenderCAM)

- **PyCAM**: 3-axis toolpaths from STL, 2D from DXF/SVG; GPL-3 per FAQ [S]
  (https://pycam.sourceforge.net/faq/). Long dormant upstream (SebKuzminsky/pycam) [S].
  Historically pioneered open drop-cutter/waterline in Python before OCL [K].
- **Fabex / BlenderCAM**: Blender add-on, many 3-axis strategies, uses OCL optionally [K];
  "licensed under GPLv3, UNLESS OTHERWISE INDICATED" [S]
  (https://spectralvectors.github.io/blendercam/faq.html). Renamed for Blender trademark
  reasons; active fork at SpectralVectors/blendercam [S].

## 9. Machine side: LinuxCNC, GRBL, bCNC, CAMotics, G-code crates

- **LinuxCNC**: controller with the RS274/NGC interpreter, trajectory planner and pluggable
  kinematics modules for non-Cartesian machines; "entirely licensed under the GPL and LGPL"
  [S] (https://linuxcnc.org/docs/html/getting-started/about-linuxcnc.html); Wikipedia lists
  GPLv2 + LGPLv2.1 [S]. **Learn:** its G-code manual is the de facto spec for the NGC dialect
  (modal groups, G41/42 cutter comp, canned cycles G81-G89, G64 path blending, O-word
  subroutines); forgeCAD's LinuxCNC post should be tested against that spec, not its code.
- **GRBL**: GPL-3.0 [S] (https://github.com/gnea/grbl-Mega/). Subset: G0-G3, G4, G10 L2/L20,
  G17-19, G20/21, G28/G30, G38.x probing, G53, G54-59, G80, G90/91, G91.1, G92, G93/94; no
  macros, variables, or most canned cycles; no multi-turn P-word arcs [S]
  (https://github.com/gnea/grbl/wiki). **Learn:** a GRBL post must expand drilling cycles and
  avoid canned cycles and subroutines; this is a good "lowest common denominator" target.
- **bCNC**: GRBL sender with autolevel and simple CAM ops; GPL-2.0 [S]
  (https://www.linuxlinks.com/bcnc-cross-platform-python-application-cnc-machines/).
- **CAMotics** (Joseph Coffland, Cauldron Development): 3-axis G-code cutting simulation;
  README says GPL "version 2+" [S]
  (https://awesome.ecosyste.ms/projects/github.com%2FCauldronDevelopmentLLC%2FCAMotics). It
  simulates, does not generate paths; 5-axis/lathe listed as planned [S]. Uses an octree/
  voxel stock with surface extraction [K]. **Learn:** stock-removal simulation by swept tool
  volume on a sparse grid is the practical approach; for forgeCAD, an exact B-rep subtract of
  swept volumes is possible in principle but far slower - a dexel/height-field simulator in
  `interface/` is the pragmatic first step.
- **Rust G-code crates.** `gcode` (Michael-F-Bryan): `no_std`/wasm-friendly parser, optional
  zero-alloc visitor, spans, MIT OR Apache-2.0 [R] (https://docs.rs/crate/gcode/latest).
  `g-code` (sameer): parse and emit, PEG parser, formatting, checksums [S]
  (https://libraries.io/cargo/g-code) - licence not read. `ngc`: LinuxCNC-dialect parser,
  parser-only [S] (https://docs.rs/crate/ngc/latest). No Rust crate found that generates
  toolpaths as a library; GCodeKit5 is a GTK app with a toolpath "designer" crate [S]
  (https://github.com/thawkins/gcodekit5). **Learn:** forgeCAD will be among the first Rust
  CAM engines; a parser crate is useful in `interface/` for round-trip tests of our own posts.

## 10. Rust CAD kernels

- **truck** (RICOS): Apache-2.0 [S] (https://github.com/ricosjp/truck). No CAM found.
- **Fornjot**: 0BSD, "No longer in development" [S] (https://github.com/hannobraun/fornjot).
  No CAM.
- CADmium builds on truck under Elastic-2.0 (not OSI) [S]. No CAM.

## 11. Additive reference only

PrusaSlicer (AGPL-3.0, from Slic3r) and CuraEngine (AGPL-3.0) [K] - read-only references for
perimeter offsetting, Arachne variable-width walls (a medial-axis method), and G-code flavour
handling. AGPL is incompatible for combined distribution with MPL-2.0 code.

## 12. Licence table (for MPL-2.0 forgeCAD)

MPL-2.0 is file-level copyleft. Combining with permissive code (MIT, Apache-2.0, BSD-3,
BSL-1.0, 0BSD) is fine: keep their notices. Apache-2.0 is compatible one way (Apache into MPL
project). GPL code cannot be copied into MPL files we distribute as MPL. MPL-2.0 has a
"Secondary Licenses" clause that lets MPL code be combined into a GPL work, not the reverse.
LGPL: linking an unmodified LGPL library dynamically is allowed, but porting LGPL source into
our files makes those files LGPL obligations (and static linking into wasm brings relinking
duties); in practice, treat LGPL as "read for ideas, reimplement clean" for a Rust/wasm
product.

| Category | Projects |
|---|---|
| **Can port with attribution** (permissive; keep notice; `interface/` or rewritten into kernel by our own hand) | libarea core + kurve (BSD-3-Clause), HeeksCAD (BSD-3-Clause; HeeksCNC to verify), Clipper2 / clipper2-rust (BSL-1.0), cavalier_contours (MIT OR Apache-2.0), Kiri:Moto (MIT), `gcode` crate (MIT OR Apache-2.0), truck (Apache-2.0), Fornjot (0BSD) |
| **Link only, or read and reimplement** (LGPL) | FreeCAD CAM incl. Adaptive.cpp (LGPL-2.1-or-later), OpenCAMLib (LGPL-2.1), OpenVoronoi (LGPL-2.1), LinuxCNC LGPL parts |
| **Read for ideas only** (GPL/AGPL; incompatible for porting into MPL-2.0) | libactp (GPL), PyCAM (GPL-3.0), Fabex (GPL-3.0), LinuxCNC GPL parts (GPL-2.0), CAMotics (GPL-2.0-or-later), GRBL (GPL-3.0), bCNC (GPL-2.0), dxf2gcode (GPL-3.0), jscut (GPL-3.0, unverified), PrusaSlicer / CuraEngine (AGPL-3.0) |
| **Not open source / unknown** | Estlcam (proprietary), MakerCAM (defunct), CAMlab (not found), `g-code` crate (licence unread) |

Caveat: "read for ideas" means learning the published algorithm and writing our own code
without copying structure or text. Before any port, verify each COPYING file
rather than repository metadata.

## 13. What forgeCAD should take away

1. **No open project combines exact B-rep with CAM in Rust.** FreeCAD does on OCCT; everyone
   else works on meshes or 2D polygons. forgeCAD can query exact faces (planes, cylinders,
   cones) for depths, floors, and hole cycles instead of rediscovering them from meshes.
2. **2D core first.** Offsets (arc-native), booleans, and a medial axis drive pocket, profile,
   adaptive, v-carve, engrave. Write these in-house for `kernel/`; Clipper2's integer approach
   is the best model for bit-identical native/wasm results.
3. **Adaptive clearing** is well documented in the open (Freesteel 2004-2006, FreeCAD 2018).
   Design ours as "cleared-area state + target removed area per step".
4. **3-axis finishing** = drop-cutter and push-cutter/waterline per cutter class; reimplement
   from the literature (OCL is the open reference).
5. **Posts are data.** A typed path IR -> per-controller post; GRBL (subset) and LinuxCNC (NGC)
   as the first two dialects; round-trip-parse our own output in tests.
6. **Checks and dressups** map naturally onto forgeCAD's typed errors with fixes and on
   features that transform features (tabs, lead-in, ramp, dogbone).
7. **Simulation** belongs in `interface/` as a height-field/dexel model first; CAMotics is the
   behavioural reference.
