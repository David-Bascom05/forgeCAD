# Open-source CAM for toolpath quality: what to read, what to port, what to run as an oracle

Research only, session 2026-11-24, branch research/toolpath-quality. Goal under study: CAM
toolpaths "Fusion level or better" in adaptive roughing, feeds and speeds, finishing and linking.
This note extends research/cam/opensource.md (read it first; it is not repeated here) and is about
three questions per project: (a) what its quality-relevant algorithm actually does, (b) whether we
may READ it, PORT it into `interface/`, or neither, and (c) whether it can be a COMPARISON ORACLE
in `verify/`, with what metric and what harness.

Marks: **[R]** read this session (source file, LICENSE, git log, PyPI/crates.io/Debian JSON, or a
command we ran); **[S]** search snippet or listing only; **[K]** background knowledge, not
re-checked (a lead, not a citation). Clones are shallow (`--depth 1`) in the session scratchpad,
under `oss/<name>`; the commit each was read at is given. Nothing from them was copied into the repo.

Policy recap (from the brief and build.md §1): `kernel/` takes no third-party code or crates;
`interface/` may take licence-compatible crates; `verify/` may run external tools as oracles.
forgeCAD is MPL-2.0 (LICENSE line 1 [R]).

## 0. Corrections to earlier notes and to the brief

- **Adaptive's author is Kresimir Tusek (GitHub `kreso-t`), not "Kreso Pavlovic".** Adaptive.cpp
  and Adaptive.hpp line 4: `Copyright (c) 2018 Kresimir Tusek <kresimir.tusek@gmail.com>` [R];
  the first commit's author is `kreso-t <kresimir.tusek@gmail.com>` [R] (§1.1).
- **Adaptive.cpp is not BSD.** libarea is per file: `Area.cpp`, `AreaClipper.cpp`, `pyarea.cpp`
  are `SPDX-License-Identifier: BSD-3-Clause` (line 1) [R]; `Adaptive.cpp/.hpp` are
  `LGPL-2.1-or-later` (line 1) [R].
- **OpenCAMLib on PyPI has no wheel for Python ≥ 3.12.** The latest release is 2023.1.11 with
  cp37–cp311 wheels only [R, PyPI JSON]. opensource.md §4 says "3.9–3.14 wheels"; that is wrong
  for PyPI. `pip install opencamlib` fails on the container's Python 3.13 [R, ran it]; it works in
  a uv-made 3.11 venv [R, ran it, §2.3].
- **The `g-code` crate is `MIT OR Apache-2.0`** (v0.7.0, crates.io API) [R]; opensource.md had
  it as unverified.
- **Kiri:Moto's licence has a wrinkle**: license.md is MIT [R], but 256 of 274 `src/**/*.js`
  files open with `/** Copyright Stewart Allen <sa@grid.space> -- All Rights Reserved */` [R,
  grep count]. See §3.1.

## 1. FreeCAD CAM

Read at FreeCAD `main` 8fef74c9 (2026-11-23), sparse clone of `src/Mod/CAM` [R]; history from a
treeless clone back to 2018-06 [R].

### 1.1 Adaptive (`src/Mod/CAM/libarea/Adaptive.cpp`, 4 360 lines; `.hpp`, 295 lines) [R]

**Licence.** Line 1 of both: `// SPDX-License-Identifier: LGPL-2.1-or-later` [R]. The prose
block below it (lines 9–11) says "GNU Library General Public License as published by the Free
Software Foundation; either version 2 of the License, or (at your option) any later version" [R],
i.e. LGPL-2.0-or-later wording, under an LGPL-2.1-or-later SPDX tag. Both are "or later", so
LGPL-2.1+ covers it; neither permits copying into MPL-2.0 files without LGPL obligations.
Verdict: **read for ideas, reimplement clean; do not port, not even into `interface/`** (a port is
a "work based on the Library" under LGPL §2 and would have to be LGPL itself).

**Origin and dates (prior-art evidence).** First commit `31cbf6f6531f88832c1c38427409a9a639820349`,
`Sun Aug 19 01:05:45 2018 +0200`, author kreso-t, "Path adaptive operation added", adding
`src/Mod/Path/libarea/Adaptive.cpp` (1 451 lines), `Adaptive.hpp` (109), `PathAdaptive.py` (421)
[R, git show]. Follow-ups by kreso-t through Sept 2018: linking-move optimisation (547ebe4f,
2018-08-28), stock to leave (791435b0), cylindrical stock (3bf7fb24), clearing from outside the
stock inwards (365227a0, 2018-09-01) [R, git log]. This is a dated, public, source-level
disclosure of engagement-controlled clearing with keep-tool-down linking, useful next to
Freesteel's 2004 headers (§3.3) for patents.md. Whether either anticipates any particular claim is
for counsel.

Recent history matters because the oracle will be a moving target [R, git log / GitHub search]:
Clipper2 port of Adaptive merged 2026-11-06 (PR #33739, fc2c96c1; FreeCAD vendors Clipper2 2.0.1,
e1a940f9 2026-05-08); rest machining (#27908, fixes #33909); fractional stepover (0bc72d62,
2026-06-17); finishing passes skipped in pre-cleared regions (#31053); helix uses RampFeed
(#33938, a8015579, 2026-11-19); an "adaptive generator version" is now passed in (2a184e86, f2aaffb4).

**Algorithm as implemented** (line numbers at 8fef74c9) [R unless marked]:
1. *Integer world.* Inputs are scaled to Clipper2 `Point64` with
   `scaleFactor = MIN_STEP_CLIPPER / tolerance / min(1, stepOverFactor·toolDiameter)` (cpp:1749),
   `MIN_STEP_CLIPPER = 48` (hpp:279, with a derivation in hpp comments: 48 > 38 so a 1 %-stepover
   protrusion is never rounded to zero area).
2. *Target.* A reference area is the tool disc minus the disc shifted by R/2 (cpp:1778–1783);
   `optimalCutAreaPD = 2·stepOverFactor·referenceCutArea / R` (cpp:1784) is the target **removed
   area per unit of advance** ("PD" = per distance). Engagement is held as removed area per step,
   not as an explicit engagement angle.
3. *Cleared area.* A `ClearedArea` object holds the union of all tool sweeps, with a cached
   bounded window around the tool for local queries (class near cpp:1060).
4. *Cut area of a candidate step.* `CalcCutArea(c1, c2, cleared)` (cpp:1235) intersects the
   new-disc-minus-old-disc crescent with the uncleared region by scanning circle/polygon
   crossings and summing circular segments (sector − triangle, cpp:1451–1474). It also returns
   the fraction cut conventionally; a step is "conventional" if ≥ 0.51 of it is (cpp:3396).
5. *Angle search.* Per step (`ProcessPolyNode`, cpp:3278) the heading change is searched in
   [−π/4, +π/4] (cpp:1080–1081): iteration 0 tries the angle predicted from the last
   `ANGLE_HISTORY_POINTS = 3` turns, iteration 1 tries −π/4 (max engagement), then a bracketed
   secant/bisection hybrid (`interpolateAngle` clamps the secant fraction to [0.2, 0.8],
   cpp:1162–1170) until |area − target| ≤ `AREA_ERROR_FACTOR = 0.05` of target (hpp:281) or
   `MAX_ITERATIONS = 30` (hpp:280). Climb is preferred: a conventional point is dropped from the
   bracket when a climb point exists (`addPoint`, cpp:1094–1150).
6. *Step length.* 1× to 8× `MIN_STEP_CLIPPER`, shorter near the boundary or entry and when turning
   hard (step ≈ MIN_STEP / |angle|), clamped to ≤ R/4 (cpp:3366–3385).
7. *Smoothing.* Heading is averaged over `DIRECTION_SMOOTHING_BUFLEN = 3` "gyro" points
   (hpp:284, cpp:3327); link and lead-out paths get `SmoothPaths(…, 0.1·stepover, 1, 4)`
   (cpp:588, 2981, 3087). There is no curvature or G2-continuity guarantee; output is polylines.
8. *Entry.* `FindEntryPoint` (cpp:2360) binary-searches the largest helix in
   [max(D/8, helixMin), min(D, helixTarget)] (cpp:1763–1771) that fits the region
   (cpp:2408–2442); the helix disc seeds the cleared area. When an engage point cannot be found
   at the area-derived protrusion (θ from the circular-segment Taylor expansion
   θ = (12A/R²)^(1/3), cpp:3854–3862), it helixes down again (cpp:3905).
9. *Linking.* Each new pass starts at an engage point; the link from the previous pass end is
   kept at depth if `IsClearPath` holds: the tool-radius(+clearance) offset of the link minus the
   cleared area has area < 1 (integer units²) (cpp:2481–2491). `ResolveLinkPath` (cpp:2550) splits
   blocked links recursively and gives up when the total exceeds `keepToolDownDistRatio`
   (default 3.0, hpp:122) times the direct distance (cpp:2663, 2717), when it self-intersects, or
   **on a wall-clock limit** `max(ratio, 3)·CLOCKS_PER_SEC/6` (cpp:2602). `IsAllowedToCutTrough`
   lets a link cut material if the cut area per step stays ≤ 1.5× target (cpp:2494–2547).
   Output segments are typed `mtCutting / mtLinkClear / mtLinkNotClear` (hpp:49), so the Python
   side retracts only on `LinkNotClear`.
10. *Finishing.* An optional finishing profile at `stepover/10` (FINISHING_THICKNESS_SCALE,
    hpp:289) around the region.
11. *Diagnostics.* `AdaptiveOutput` carries `UnclearedAreaRemains`, `TooManyFailedEngagements`,
    `LeadPathFailed` and friends (hpp, struct AdaptiveOutput) [R] — a good model for forgeCAD's
    typed warnings.

**Parameters exposed** (Path/Op/Adaptive.py:492–505 [R]): StepOverPercent, tool diameter, helix
target/min diameter, keep-tool-down ratio, stock to leave, tolerance, force inside-out, finishing
profile, op type (clearing/profiling × inside/outside). Plus helix angle/cone and RampFeed in the
op (see the wiki summary in opensource.md §2).

**Known weaknesses** [R, GitHub issue search API, 2026-11-24]:
- #34143 (open, 2026-11-05): "some moves with too high a cut load" — heavy cut and chatter when a
  loop closes, e.g. end of an adaptive profile; #34144 is a proposed fix to the next-point search,
  developed with a visualiser that colours segments by load against a 5 % band.
- #29885 (closed): short slotting move into an inner 90° corner, "violates step over".
- #27227 (closed): regression leaving uncleared areas, logging "some cuts may be above optimal
  step-over".
- #33470 (closed by #33938): helix entry used the clearing feed; no separate ramp feed.
- #30629 (closed): finishing profile went round the whole stock perimeter.
- #28486 (open): no 3D adaptive; proposal is slice-and-repeat like 3D Pocket.
- From the code: the wall-clock limit in linking (cpp:2602) makes output **machine-speed
  dependent**, so it cannot be a bit-exact oracle; compare statistics only.

**What forgeCAD should take.** (1) "Removed area per unit advance within ±5 %" is a measurable
quality spec, independent of how the path is made; we should state ours the same way and prove
or measure it. (2) Typed link classes and a keep-down ratio. (3) Its gaps are our targets:
bounded load at loop closure and inner corners, G2-smooth output (arcs, not polylines),
determinism (no clocks), 3D adaptive.

### 1.2 Other quality-relevant FreeCAD pieces [R, file listing and headers at 8fef74c9]

All files below carry `SPDX-License-Identifier: LGPL-2.1-or-later` at line 1 [R for those
opened]: read only.
- **3D Surface / Waterline** import `ocl` or `opencamlib` (Op/Surface.py:38–40,
  Op/Waterline.py:38–40); generators `surface_dropcutter.py` (`batch_dropcutter`,
  `path_dropcutter`, `adaptive_path_dropcutter`) and `surface_waterline.py` (`waterline`,
  `adaptive_waterline`, `waterline_stack`). There are now also `surface_zlevel`, `surface_pattern`,
  `surface_mesh`, `surface_postprocess` and rotary drop-cutter generators (2026-08 "3D surface
  toolpath generators, C++ accelerator", c6782580).
- **Ramp entry** (Base/Generator/ramp_entry.py:58–92): methods 0 (helix, closed paths only),
  1 (ramp along the path, then back), 2 (travel at depth then ramp backwards), 3 (half-depth
  zig-zag), 4 (ramp along a closed loop and finish the loop).
- **Lead-in/out** (leadinout.py:49–50): Arc, Line, Perpendicular, Tangent, Arc3d, ArcZ, ArcZFollow.
- **Linking** (linking.py, 2025): `get_linking_moves`, `check_collision`,
  `is_travel_collision_free` sweep a tool shape along travel wires on OCC.
- **Dressups**: Array (Linear1D/Linear2D/Polar with jitter: Dressup/Array.py:128), Boundary,
  DogboneII, Tags; GUI adds AxisMap, Dragknife, LeadInOut, Mirror, PlungeMilling, RampEntry, ZCorrect.
- **Feeds and speeds in core** (new, Path/Tool/FeedsSpeeds/, 2026, sliptonic): a resolver chaining
  providers (tool presets → material machinability → tool default chipload); `rpm =
  Vc·1000/(π·d)` and `feed = rpm·z·fz` (providers.py:117–127); spindle-limit clamp rescales feed to
  keep chipload (resolver.py:147). **No radial chip thinning, no power or deflection check** [R,
  grep]. Unit tests are pure functions (CAMTests/TestFeedsSpeedsResolver.py).
- **FeedsAndSpeeds addon** (github.com/dubstar-04/FeedsAndSpeeds @99dd304, 2025-12-22): LICENSE is
  LGPL-2.1 text; CAMFeedsAndSpeeds.py:1 `SPDX-License-Identifier: LGPL-2.1-or-later` [R]. Adds a
  power estimate `Pc = Kp·C·Q·W`, motor power `Pc/0.8` (CAMFeedsAndSpeeds.py:177–231), with the
  feed factor C table "from Machineries Handbook 28, Table 2" (lines 43–47) [R]. Material cards
  (`Materials/*.FCMat`) are "(c) 2022 Daniel Wood (CC-BY 3.0)" with `SurfaceSpeed_HSS`,
  `SurfaceSpeed_Carbide`, `Kp`, `Kd` [R]. Caution: values transcribed from a copyrighted
  handbook; do not import the table, cite primary data instead.

## 2. OpenCAMLib (OCL)

Read at aewallin/opencamlib 1a450b7 (2026-10-17) [R].

### 2.1 Licence
`COPYING` is the LGPL-2.1 text [R]; README.rst:55 "From August 2018 OpenCAMLib is released under
LGPL license" [R]; file headers (e.g. src/algo/waterline.cpp:8–11) say "GNU Lesser General Public
License … either version 2.1 of the License, or (at your option) any later version" [R]. SPDX:
**LGPL-2.1-or-later**. Read only; may be run as an oracle.

### 2.2 Algorithms and cutters [R, source tree]
- Drop-cutter per triangle: `MillingCutter::vertexDrop / facetDrop / edgeDrop / singleEdgeDrop /
  dropCutter / dropCutterSTL` (src/cutters/millingcutter.cpp:36–412), overridden per cutter.
- Batch and path variants: `BatchDropCutter`, `PathDropCutter`, `AdaptivePathDropCutter`
  (src/dropcutter/).
- Push-cutter on fibres and weaves: `fiberpushcutter`, `batchpushcutter`, `weave`,
  `simple_weave`, `smart_weave`; `Waterline` and `AdaptiveWaterline` (src/algo/), the latter
  subdividing fibres between `sampling` and `min_sampling` (default sampling/10) with a flatness
  predicate `cosLimit` (adaptivewaterline.hpp).
- Cutters exported to Python [R, `dir(opencamlib)`]: CylCutter, BallCutter, BullCutter,
  ConeCutter and composites BallConeCutter, BullConeCutter, ConeConeCutter, CylConeCutter,
  CompBallCutter, CompCylCutter.

### 2.3 As a verify/ oracle: drop-cutter CL points — tried and working

Availability [R]: PyPI `opencamlib` 2023.1.11, wheels cp37–cp311 (manylinux x86_64/aarch64,
macOS, Windows); Debian has source package `opencamlib` 2023.01.11 in trixie (-5) and forky/sid
(-7) [R, sources.debian.org API]; not found in Ubuntu 24.04 listing [S]. On this container:

```sh
uv venv -p 3.11 $VENV && VIRTUAL_ENV=$VENV uv pip install opencamlib==2023.1.11
$VENV/bin/python -I tools/camcheck/ocl_drop.py      # (proposed path)
```

Smoke test run this session [R]: a plane z = 0.5x as two triangles, cutters ⌀6 ball and flat,
`BatchDropCutter` at x ∈ {−10, 0, 7.25, 20}; closed forms tip z = m·x + r√(1+m²) − r (ball) and
m(x + r) (flat). Every OCL result agreed to ≤ 4.4e−16 (e.g. ball at x = 0: 0.354101966 both;
flat at 7.25: 5.125 both), `ocl.version()` = 2023.01.11.

Harness idea: for each `.fg` test part, tessellate at a tight tolerance (`forgecad export F stl
tol 0.001`), drop our cutters at a grid and along our finishing path's XY, and compare our CL z
(computed on exact B-rep) with OCL's (on the mesh). Metric: max and p99 |Δz|; expected bound is
the mesh chordal error plus 1e−9; any point where ours is *lower* than OCL by more than that bound
is a gouge candidate and fails the test. Also compare waterline loops (Hausdorff distance between
our and OCL's loops at the same z, via Clipper2 offset bands, §4.1).

## 3. Other projects

### 3.1 Kiri:Moto (GridSpace/grid-apps @d138275, 2026-07-26)
- Licence: license.md is the MIT text, "Copyright 2014-2018 Stewart Allen" [R]; package.json:6
  `"license": "MIT"` [R]. But 256/274 source files carry an "All Rights Reserved" header line
  [R]. MIT from the copyright holder grants the rights regardless, but the conflicting notice
  should be cleared with the author (or counsel) **before any port**. Reading is fine.
- CAM ops: `src/kiri/mode/cam/work/op-*.js` — rough, outline, contour, pocket, area, trace,
  drill, helical, lathe, level, shadow, xray [R, listing]. Roughing = `OpArea` in `mode: 'clear'`
  with `over: op.step`, `down`, `leave_xy`, `leave_z` on the expanded shadow of the part
  (op-rough.js:28–56) [R]: offset clearing, **no engagement control**. A Clipper 1 JS copy is in
  src/ext/clip2.js [R]. The only "adapt" in CAM UI code is a laser setting (cl-ops.js:713) [R].
- Oracle use: weak (browser app; G-code export only). Not on the list.

### 3.2 PyCAM, Fabex (BlenderCAM), jscut, HeeksCNC
- **PyCAM** (SebKuzminsky/pycam @55e3129, 2021-11-25): LICENSE.TXT / COPYING.TXT GPL-3 text;
  headers "either version 3 … or (at your option) any later version" (pycam/Toolpath/__init__.py)
  [R] → **GPL-3.0-or-later**. Read only. Dormant.
- **Fabex** (SpectralVectors/blendercam @a2abe3a, 2026-11-16, pyproject name `blendercam` 3.1.6):
  LICENSE GPL-3 text [R]. Strategies enumerated in its properties include PARALLEL, CROSS, BLOCK,
  SPIRAL, CIRCLES, OUTLINEFILL, CARVE, PENCIL, MEDIAL_AXIS, WATERLINE, CUTOUT, POCKET, DRILL,
  CURVE, PROJECTED_CURVE [R, grep]; no adaptive clearing found [R, grep]. Optional OCL use
  (fabex/utilities/strategy_utils.py imports it) [R]. Read only.
- **jscut** (tbfleming/jscut @e22fb34, 2023-02-02): COPYING GPL-3 text; js/Cam.js "Copyright 2014
  Todd Fleming … either version 3 … or later" [R] → GPL-3.0-or-later (now confirmed upstream).
- **HeeksCNC** (Heeks/heekscnc): COPYING says "HeeksCAD is covered by the new BSD license,
  Copyright (c) 2008, Daniel Heeks" [R] — the repo's licence file names HeeksCAD, not HeeksCNC, so
  treat HeeksCNC as BSD-3-Clause-*intended*; per-file checks before any port. `actp_funcs.py`
  drives libactp (`actp.makerough(stl_file)`) [R]; `roughing_funcs.py`, `ocl_funcs.py`,
  `area_funcs.py` wire the others.

### 3.3 libactp / Freesteel: origin of adaptive clearing
Heeks/libactp-old @eb97a62 (last commit 2010-03-27) [R]:
- Headers: "FreeSteel -- Computer Aided Manufacture Algorithms / Copyright (C) 2004 Julian Todd
  and Martin Dunschen … GPL … either version 2 … or (at your option) any later version"
  (freesteel/src/cages/PathX.cpp:1–20) [R]. freesteel/fslicense.txt:1–3 "released under the
  General Public License"; it says some low-level parts "may be re-released under the FreeBSD
  license at a later date" [R] (no evidence it happened). debian/copyright: packaging 2009 by
  Joachim Steiger, "Upstream Author: Dan Heeks", GPL-2+, from code.google.com/p/libactp [R].
  SPDX: **GPL-2.0-or-later**. Read only.
- The roughing core is `CoreRoughGeneration` (freesteel/src/pits/CoreRoughGeneration.{h,cpp}):
  `GrabberAlg`, `ChangeBearing`, steering parameters `dchangright`, `dchangrightoncontour`,
  `dchangleft`, `dchangefreespace`, `sidecutdisplch`, plus `BuildLink` for linking (h:50–113,
  cpp:482–512, 671–690) [R]. `ChangeBearing` finds the arc of the tool circle touching stock
  (`CircleIntersectNew`) and turns by how much stock it touches (cpp:678–690) [R]: a
  contact-arc (engagement-angle) controller, different from FreeCAD's area-per-advance search.
- Dates: 2004 copyright in headers; a 2009 Debian packaging; this repo to 2010 [R]. Claims that
  Freesteel showed adaptive clearing ~2004 and that Todd then worked on HSMWorks are forum-level
  [S] (opensource.md §3).

### 3.4 Geometry libraries usable as oracles (and as `interface/` crates)
- **Clipper2** (AngusJohnson/Clipper2 @4be4655, 2026-10-29): LICENSE "Boost Software License -
  Version 1.0" [R]; clipper.version.h:4 `CLIPPER2_VERSION = "2.0.1"` [R]. **BSL-1.0**: may be
  ported or linked in `interface/` with the notice. Python: `pyclipr` 0.1.8, BSL-1.0, Clipper2
  wrapper, cp39–cp314 wheels [R, PyPI]. (`pyclipper` 1.4.0 wraps Clipper **1**, MIT [R, PyPI].)
  Rust: `clipper2-rust` 1.3.0 BSL-1.0 "pure Rust port", `clipper2` 0.6.1 MIT OR Apache-2.0 FFI
  wrapper [R, crates.io].
  Smoke test [R, ran it]: inward offset −3 of a 40×20 rectangle has area 476.0 (exact 476);
  outward +3 with round joins 1188.20 vs exact 1188.27 — round joins are polygonised, so as an
  oracle Clipper2 needs a stated arc tolerance in the comparison.
- **cavalier_contours** (jbuckmccready @5268bea, 2026-10-21): LICENSE-MIT ("Copyright (c) 2021
  Jedidiah Buck McCready") and LICENSE-APACHE [R]; crates.io 0.9.0, `MIT OR Apache-2.0`, updated
  2026-08-20 [R]. Arc-native offsets: the right oracle for *our* arc offsets (no polygonisation
  error), usable as a Rust dev-dependency of a `verify/` crate.
- **OpenVoronoi** (aewallin/openvoronoi @24a967d): COPYING LGPL-2.1; src headers "version 2.1 …
  or later" [R] → LGPL-2.1-or-later. Not on PyPI, not in Debian [R, APIs]; build from source
  with Boost.Python (CMake). Second choice for medial axis.
- **Boost.Polygon Voronoi** via `pyvoronoi` 1.2.8 (MIT wrapper, "Cython wrapper for the Boost
  Voronoi library (version 1.59.0)", cp310–cp314 wheels) [R, PyPI]. Boost itself is BSL-1.0 [K].
  Smoke test [R, ran it]: segment Voronoi of a 40×20 rectangle gives an interior spine on y = 10
  of length 20 (40 counting both half-edges), as expected.
- **libarea core** (Heeks/libarea @2752cd9, 2019): Area.cpp "released under the BSD license. See
  the file COPYING" but there is **no COPYING at the repo root**; debian/copyright gives
  "Files: * … License: BSD 3-Clause" [R]. The repo also contains **kbool**, which is "dual
  licensed. The GPL applies for open source" with GPL-3 text (kbool/kboollicense.txt) [R]; do not
  touch that directory.

### 3.5 Simulation and motion planning
- **CAMotics** (CauldronDevelopmentLLC/CAMotics @e84665f, 2026-03-23): LICENSE "either version 2
  … or (at your option) any later version", Copyright 2011-2021 Joseph Coffland [R] →
  **GPL-2.0-or-later**. Three CLIs are built (SConstruct:279 `progs = 'gcodetool planner camsim'`)
  [R]: `camsim <project|gcode|tpl> <output.stl>` with `--time`, `--resolution`,
  `--render-mode`, `--threads`, `--reduce` (src/camsim.cpp) [R]; `planner` reads G-code and a JSON
  config and prints a plan as JSON and "Total time" (src/planner.cpp:36–99) [R]. Its planner is
  an S-curve (jerk-limited) one: `PlannerConfig.h:38–43` defaults maxVel, maxAccel, maxJerk,
  `junctionDeviation = 0.05`, junctionAccel [R]. Build needs C! (cbang) + scons (README.md:49–90)
  [R]; not in Debian [R, API]. **Oracle use**: (a) stock: `camsim` our G-code on a box stock and
  compare its STL against our in-process stock (volume difference, Hausdorff via our own mesh
  tools); (b) cycle time: `planner` on our posted G-code vs our estimate. Not built this session.
- **LinuxCNC tp** (sparse clone @65f7467, 2026-11-23, VERSION 2.10.0~pre2): src/emc/tp/tp.c,
  blendmath.c "License: GPL Version 2" [R] → treat as **GPL-2.0-only** (no "or later" in the
  header). New in 2.10: `sp_scurve.c` "Ruckig-based S-curve trajectory planning" and a vendored
  `cruckig` (pure C99 port of Ruckig), whose LICENSE is **MIT**, "Copyright (c) 2025 Yang Yang …
  Based on Ruckig … Copyright (c) 2021 Lars Berscheid" [R]. Tests: tests/trajectory-planner/
  circular-arcs/ drive a real LinuxCNC (`import linuxcnc`, hal) [R], i.e. a full sim install, not
  a library. Debian ships `linuxcnc` 2.9.4 (trixie) / 2.9.9 (sid) [R, API]. Heavy as an oracle;
  use Ruckig directly instead (below).
- **Ruckig** (pantor/ruckig): PyPI `ruckig` 0.19.4, MIT, cp39–cp314 wheels, uploaded 2026-07-15
  [R, PyPI]. Time-optimal jerk-limited trajectories. Smoke test [R, ran it]: 1-axis 100 mm rest to
  rest with v = 100, a = 1000, j = 50 000 gives T = 1.12 s, equal to the closed form
  d/v + v/a + a/j = 1.12. As an oracle it gives a **lower bound** on any jerk-limited
  controller's time for a segment, and its MIT licence would even permit use in `interface/`.
- **Grbl** (gnea/grbl @bfb67f0, 2021-01-22): COPYING "either version 3 … or later" [R] →
  GPL-3.0-or-later. Planner junction speed
  `v² = a·δ·sin(θ/2)/(1 − sin(θ/2))` (grbl/planner.c:448–449) [R]; trapezoidal (no jerk) [K].
  A Grbl simulator (`grbl-sim`) exists [K, not checked]; the formula alone is enough to write
  our own estimator and test it against hand cases. Read only.

### 3.6 Rust CAM crates (crates.io API, 2026-11-24) [R metadata only, code not read]
- `svg2gcode` 0.4.1 MIT (pen/laser, not milling); `gcode` 0.7.0 MIT OR Apache-2.0 (parser);
  `g-code` 0.7.0 MIT OR Apache-2.0 (parse + emit); `i_overlay` 9.2.0 MIT OR Apache-2.0 (booleans).
- Young CAM-shaped crates, all 2026: `cutrail` 0.1.1 Apache-2.0 ("material removal simulation …
  mesh-based stock … 3- and 5-axis sweeps"); `gsim-rs` 2.0.0 MIT OR Apache-2.0 (G-code
  simulator); `chipbreaker` 0.1.1 GPL-3.0-or-later ("machining verification engine" CLI);
  `openchuck-path` 0.55.0 **AGPL-3.0-or-later** (4-axis pathing); `BREP_cam` 0.17.0 licence
  "non-standard". Downloads are tens to hundreds. Worth a later read of `cutrail` as a possible
  Apache-2.0 stock-simulation oracle; nothing here is a mature reference.
- `cnc` and `nc-gcode` crates do not exist [R, crates.io 404]. Searching "toolpath" returns
  unrelated AI-provenance crates [R].

### 3.7 Not open, or not found
Carbide Create (closed, [K]); Machinery's Handbook (copyrighted; the FeedsAndSpeeds addon's
C-table is transcribed from it, §1.2); CAMlab, "Mesh CAM" as OSS: not found (opensource.md §7).
No openly licensed, sourced cutting-data dataset (vc, fz by material and diameter, kc1.1, mc) was
found this session beyond the CC-BY-3.0 FreeCAD material cards; forge-tooling's own sourced TOML
(build.md §7) remains the plan.

## 4. Licence and use matrix

"Port" = copy or translate code. MPL-2.0 is compatible with permissive licences (keep notices);
LGPL/GPL code may not be ported into our MPL files. Running a program as a separate process in
`verify/` (not shipped with forgeCAD) is use, which neither GPL nor LGPL restricts [K, GPL-2 §0 /
LGPL-2.1 §0 wording "The act of running … is not restricted", the latter read this session in
OCL's COPYING]. Python oracles live in a venv outside the repo, like tools/meshcheck.

| Project | SPDX (evidence) | Read | Port to interface/ | Port to kernel/ | Oracle in verify/ (metric) |
|---|---|---|---|---|---|
| FreeCAD Adaptive.cpp | LGPL-2.1-or-later (cpp:1) | yes | no | no | engagement histogram, cleared %, path length, link stats |
| FreeCAD other CAM py | LGPL-2.1-or-later (line 1) | yes | no | no | ramp/lead geometry cases |
| FeedsAndSpeeds addon | LGPL-2.1-or-later (py:1); cards CC-BY-3.0 | yes | no | no | rpm/feed formulas only (trivial) |
| libarea core | BSD-3-Clause (FreeCAD copies, line 1; debian/copyright) | yes | yes, with notice | no (policy) | — |
| kbool (in Heeks/libarea) | GPL-3 / proprietary dual | yes | no | no | — |
| OpenCAMLib | LGPL-2.1-or-later (COPYING, headers) | yes | no | no | **CL points vs drop-cutter; waterline loops** |
| OpenVoronoi | LGPL-2.1-or-later | yes | no | no | medial axis (if built) |
| Boost Voronoi / pyvoronoi | BSL-1.0 [K] / MIT (PyPI) | yes | yes | no | **medial axis vertices/edges** |
| Clipper2, clipper2-rust, pyclipr | BSL-1.0 (LICENSE, crates.io, PyPI) | yes | yes | no | **offset area/Hausdorff, cleared-area booleans** |
| cavalier_contours | MIT OR Apache-2.0 | yes | yes | no | **arc-native offsets** |
| Kiri:Moto | MIT (license.md) + "All Rights Reserved" headers | yes | after clearing headers | no | weak |
| libactp/Freesteel | GPL-2.0-or-later | yes | no | no | — (2004 prior art) |
| HeeksCNC | BSD-3 intended (COPYING names HeeksCAD) | yes | after per-file check | no | — |
| PyCAM, Fabex, jscut, dxf2gcode | GPL-3.0-or-later | yes | no | no | weak |
| CAMotics | GPL-2.0-or-later | yes | no | no | **stock STL; planner time** |
| LinuxCNC tp | GPL-2.0-only (header) ; cruckig MIT | yes | cruckig only | no | heavy; prefer Ruckig |
| Ruckig | MIT (PyPI) | yes | yes | no | **jerk-limited time lower bound** |
| Grbl | GPL-3.0-or-later | yes | no | no | formula-level hand cases |

Policy reminder: even permissive code goes to `interface/` only; `kernel/` gets our own
implementations, which the oracles then check.

## 5. Top five oracles to wire into verify/ first

Proposed shape, following tools/meshcheck: `tools/camcheck/setup.sh` makes a uv venv with pinned
wheels outside the repo; `tools/camcheck/run.sh` builds forge-cli, exports cases, runs
`python -I` checkers; results recorded in `verify/forge-cam/README.md` as L6 "differential" rows
next to the ℚ oracle. Nothing here enters `kernel/`.

1. **OpenCAMLib drop-cutter (3-axis finishing CL points).** Already proven to run here (§2.3).
   Setup: `uv venv -p 3.11` + `opencamlib==2023.1.11`. For each finishing test part and each
   cutter kind we support (flat, ball, bull; cone when we add it), export the part as STL at
   tol 0.001 mm and our CL points as JSON (`query F cam` or a test binary), run OCL
   `BatchDropCutter` at the same XY, and report max/p99 |Δz| and every point where ours is below
   OCL's by more than the tessellation bound (gouge) or above by more than bound + 1 µm
   (undercut, i.e. scallop/material left). Add closed-form planes, cylinders and spheres where
   both must match the formula to 1e−9. Then extend to `Waterline`/`AdaptiveWaterline` loops at
   fixed z (compare loops by Hausdorff distance). This is the cheapest, strongest oracle on the
   list: one wheel, no build, exact metric.

2. **Clipper2 + cavalier_contours (2D offsets and the engagement meter).** Two uses. (a) Offsets:
   compare our exact region offsets (build.md §3) with cavalier_contours (arc-native, as a Rust
   dev-dependency of a `verify/forge-cam-diff` crate) and Clipper2 (`pyclipr`, with a stated arc
   tolerance): symmetric-difference area and Hausdorff distance, at many radii, on random and
   corpus profiles. (b) An **independent engagement meter** for any toolpath, ours or FreeCAD's:
   replay a path over the stock polygon with Clipper2, at each step compute removed area per
   advance and the engagement arc (tool circle ∩ uncleared region), and emit a histogram plus the
   max. This is the yardstick for "Fusion level": e.g. "p99 load ≤ 1.05 × target, no full-slot
   step outside declared entries", which FreeCAD's #34143/#29885 show is a real failure mode.
   Writing the meter on Clipper2 keeps it independent of our kernel's offset code.

3. **FreeCAD Adaptive as a quality baseline (not a correctness oracle).** Run FreeCAD's own
   `area.Adaptive2d` headlessly on the same 2D regions, through the engagement meter of (2).
   Availability: conda-forge `freecad` 2026.09.16, LGPL-2.1-or-later, linux-64 and others [R,
   anaconda.org API]; Debian/Ubuntu packages are older [K]. Sketch (not run this session):
   `micromamba create -p $ENV -c conda-forge freecad` then `freecadcmd` with a script that does
   `import area; a = area.Adaptive2d(); a.toolDiameter = 6; a.stepOverFactor = 0.1;
   r = a.Execute(stock_paths, region_paths, [], lambda p: False)` — the binding signature is
   pyarea.cpp:337–368 and Adaptive.hpp `Execute(stockPaths, paths, clearedPaths, cb)` [R];
   TestPathAdaptive.py:283–312 does the same in FreeCAD's tests [R]. Metrics side by side: p50/p99/
   max load vs target, cleared fraction (and `UnclearedAreaRemains`), cutting length, number of
   retracts vs keep-down links, compute time. Because its linking has a wall-clock limit
   (cpp:2602), pin the FreeCAD build and compare distributions with tolerance, not bytes.

4. **Ruckig (and CAMotics `planner`) for cycle time.** Our cycle-time estimate should be bracketed:
   Ruckig (MIT, `pip install ruckig`, verified §3.5) gives the jerk-limited time lower bound per
   segment and per corner given machine v/a/j; our estimator must never report less than it, and
   should be within a stated factor of a real controller's planner. For the "real planner" side,
   CAMotics' `planner` CLI (GPL-2.0-or-later, separate process) prints total time for G-code with
   junction deviation and jerk; build cbang + CAMotics once in CI cache. Metric: relative error of
   our time vs planner per program, and per-feature breakdown on the machined examples; flag any
   program where ours < Ruckig's bound.

5. **Boost.Polygon Voronoi (pyvoronoi) for the medial axis.** For V-carve, trochoidal slot
   centrelines, "rest" detection and the adaptive entry heuristic we will need a medial axis.
   `pyvoronoi` (MIT wrapper of BSL-1.0 Boost) installs as a wheel (verified §3.4); it takes
   integer-scaled segments, matching our exact-input style. Metric: for every corpus region,
   every vertex of our medial axis lies within ε of a Boost Voronoi vertex and has equal clearance
   radius (distance to the nearest site) to 1e−9 relative; edges matched by endpoints; spine
   length totals compared. OpenVoronoi (LGPL, source build) can be a second opinion later.

Next after these: CAMotics `camsim` stock-vs-our-stock (volume and Hausdorff of the residual) once
the planner build exists; a closed-form scallop check (ball of radius r, stepover s on a plane:
h = r − √(r² − s²/4)) using OCL CL points; and a read of `cutrail` (Apache-2.0) as a permissive
stock-simulation reference.

## 6. What this means for "Fusion level or better"

- The open baseline for adaptive is FreeCAD's 2018 area-per-advance controller, still being
  fixed in 2026 for load spikes at loop closure and inner corners, with polyline output and
  clock-dependent linking. A deterministic, arc-output adaptive whose load bound is *measured* by
  an independent meter (oracle 2) and *beats FreeCAD on the same regions* (oracle 3) is a concrete
  "or better" target that does not depend on access to Fusion.
- Open feeds-and-speeds code is shallow (rpm/feed from Vc and fz, optional power from handbook
  constants; no chip thinning in FreeCAD core). Chip thinning, power and deflection checks in
  forge-tooling would already exceed the open state of the art; there is no open oracle for them,
  so they need closed-form unit tests and sourced data.
- 3-axis finishing has a strong open oracle (OCL) and should be the first quality area with a
  differential test.

## References

All read 2026-11-24 unless marked.
1. FreeCAD, src/Mod/CAM/libarea/Adaptive.cpp, Adaptive.hpp, Area.cpp, AreaClipper.cpp, pyarea.cpp,
   CMakeLists.txt @8fef74c9 — https://github.com/FreeCAD/FreeCAD/tree/main/src/Mod/CAM/libarea [R]
2. FreeCAD commit 31cbf6f6 "Path adaptive operation added", 2018-08-19, and `git log --grep
   adaptive` 2018-2026 (treeless clone) [R]
3. FreeCAD src/Mod/CAM/Path/Op/Adaptive.py, Surface.py, Waterline.py; Path/Base/Generator/
   ramp_entry.py, leadinout.py, linking.py, surface_dropcutter.py, surface_waterline.py;
   Path/Dressup/Array.py; Path/Tool/FeedsSpeeds/*.py; CAMTests/TestPathAdaptive.py,
   TestFeedsSpeedsResolver.py @8fef74c9 [R]
4. FreeCAD issues/PRs #34143, #34144, #33470, #33938, #33739, #30629, #29885, #28486, #27227 —
   via https://api.github.com/search/issues (repo:FreeCAD/FreeCAD adaptive in:title) [R]
5. dubstar-04/FeedsAndSpeeds @99dd304: LICENSE, CAMFeedsAndSpeeds.py, Materials/*.FCMat [R]
6. aewallin/opencamlib @1a450b7: COPYING, README.rst, src/algo, src/cutters, src/dropcutter [R];
   https://pypi.org/pypi/opencamlib/json [R]; https://sources.debian.org/api/src/opencamlib/ [R]
7. aewallin/openvoronoi @24a967d: COPYING, src/voronoidiagram.cpp [R]
8. GridSpace/grid-apps @d138275: license.md, package.json, src/kiri/mode/cam/** [R]
9. SebKuzminsky/pycam @55e3129; SpectralVectors/blendercam @a2abe3a; tbfleming/jscut @e22fb34;
   Heeks/heekscnc (COPYING, actp_funcs.py); Heeks/libarea @2752cd9 (debian/copyright,
   kbool/kboollicense.txt) [R]
10. Heeks/libactp-old @eb97a62: debian/copyright, freesteel/fslicense.txt,
    freesteel/src/cages/PathX.cpp, freesteel/src/pits/CoreRoughGeneration.{h,cpp} [R]
11. AngusJohnson/Clipper2 @4be4655 (LICENSE, clipper.version.h); jbuckmccready/cavalier_contours
    @5268bea (LICENSE-MIT, LICENSE-APACHE) [R]
12. CauldronDevelopmentLLC/CAMotics @e84665f: LICENSE, SConstruct, src/camsim.cpp,
    src/planner.cpp, src/gcode/plan/PlannerConfig.h, SCurve.cpp, README.md [R]
13. LinuxCNC/linuxcnc @65f7467 (sparse): src/emc/tp/tp.c, blendmath.c, sp_scurve.c,
    cruckig/LICENSE, tests/trajectory-planner/circular-arcs [R];
    https://sources.debian.org/api/src/linuxcnc/ [R]
14. gnea/grbl @bfb67f0: COPYING, grbl/planner.c [R]
15. PyPI JSON for ruckig, pyclipr, pyclipper, pyvoronoi [R]; crates.io API for cavalier_contours,
    clipper2-rust, clipper2, svg2gcode, gcode, g-code, i_overlay, cutrail, gsim-rs, chipbreaker,
    BREP_cam, openchuck-path [R]; https://api.anaconda.org/package/conda-forge/freecad [R]
16. Session runs (scratchpad, not in repo): OCL drop-cutter vs closed form; Ruckig vs closed
    form; Clipper2 offset areas; Boost Voronoi rectangle spine [R]
17. research/cam/opensource.md, research/cam/build.md, verify/forge-cam/README.md,
    tools/meshcheck/run.sh (this repo) [R]
