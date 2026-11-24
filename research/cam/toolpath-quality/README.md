# Toolpath quality: how forgeCAD gets to "Fusion level or better"

2026-11-24. Research only: no kernel or interface code was changed. Written for David.

David's ask: *"we need to make [the toolpaths] GOOD — that is the unlock. Fusion level or better:
adaptive toolpaths, feeds and speeds, surface finish. Start a research agent and prior art."*

This file is the summary, the ranked plan, and an index. Each topic note has the detail and the
sources. They extend the earlier notes in `research/cam/` (literature.md, opensource.md,
fusion-*.md); they do not repeat them.

| note | what |
|---|---|
| [roughing-linking.md](roughing-linking.md) | Exact engagement geometry, roughing state of the art (Freesteel → Fusion, Stori & Wright, Ibaraki, trochoids, spirals, VoluMill, iMachining, Vortex, ProfitMilling, FreeCAD Adaptive read from source), step-down, linking and ordering, metrics, what can be proved |
| [feeds-speeds.md](feeds-speeds.md) | Chip thinning (radial, ball, lead angle, bull nose), engagement in corners, Kienzle and mechanistic force, power, deflection, chatter lobes, feed scheduling, acceleration and jerk, tool-maker data, a worked example |
| [surface-finish.md](surface-finish.md) | Scallop formulas (exact and bounds), steep walls, curvature, Ra; finishing strategies; drop and push cutter; corner smoothing, arc fitting, NURBS; controller tolerance modes; 5-axis outlook; statements S1–S7 that can be proved |
| [fusion-benchmark.md](fusion-benchmark.md) | What Fusion offers per area, with its parameters; user-reported weaknesses; competitors; **a testable 69-item "Fusion-level" checklist** (17 items go beyond Fusion); benchmark parts |
| [gap-tables.md](gap-tables.md) | **The whole Fusion 2D / 3D-rough / 3D-finish / common / turning set**: one row per toolpath (forgeCAD on main, on a branch, or absent, from the code; Fusion; best known method; metrics; impact; effort), a ranked top 15, 8 hand-off packets |
| [opensource-oracles.md](opensource-oracles.md) | FreeCAD Adaptive (from its source), OCL, Kiri:Moto, libactp, Clipper2, cavalier_contours, Boost Voronoi, Ruckig, CAMotics, LinuxCNC tp, Grbl: licences with file evidence, what we may read or port, and **five oracles for verify/** (two already run here) |

**Source marks**, as in every CAM note: **[R]** read this session; **[S]** search snippet only;
**[K]** known, not re-read (a lead); **[D]** derived here and checked numerically, but not proven
and not taken from a source. Sources the fetcher could not reach are listed at the end of each
note. No bot wall was bypassed and no account was made.

---

## 0. The answer in one page

**Where forgeCAD is today (read in the code, 2026-11-24).** The toolpaths are *safe*: every move
is certified against the exact part, fixtures and stock. They are not yet *good*:

- pockets are plain offsets, linked by retracting at cutting feed after every pass;
- there is no stay-down linking, no corner rounding, and no engagement computation;
- each op has one feed;
- cycle time is length ÷ feed, with no acceleration model;
- the feeds engine has a power bug (§2, item 1).

(roughing-linking.md §0 and §7, feeds-speeds.md §0.)

**What "Fusion level" means in measurable terms.** fusion-benchmark.md §4 turns Fusion's
documented behaviour into 69 tests. Each has a benchmark part, a metric and a pass threshold.
The thresholds are proposals for forgeCAD; Fusion publishes none. The core ones:

- the engagement never exceeds the cap ("optimal load") anywhere, inner corners included;
- no full-width slot moves except declared entries;
- stay-down linking inside cleared stock;
- the finishing scallop is at most the target on the slopes each strategy claims;
- smoothing and arc output within tolerance;
- a cycle time that is honest.

**Where Fusion is weak, from its own help [R] plus user reports [S]:**

| area | weakness | source |
|---|---|---|
| feeds | No feed driven by engagement or chip thinning. "Feed optimization" only slows down at sharp corners. | [R] |
| output | No NURBS or spline output: the post engine has only line and arc motion. | [R] |
| accuracy | The accuracy model is triangulated: toolpath tolerance + triangulation tolerance, then smoothing on top. No bound against the exact surface. | [R] |
| cycle time | The estimate is feed time + rapid time + tool-change time, with no acceleration. Users report 2–10× errors on 3D paths. | help [R]; users [S] |
| adaptive | Users report corner and sliver artefacts and slow, single-threaded generation. | [S] |

Each of these is a place where forgeCAD can be *better*, not just equal.

**The thesis.** "Fusion or better" is reachable in three moves:

1. **Measure.** Give every op a certified quality report: max engagement, chip thickness,
   scallop, air-cut %, retracts, minimum path radius, and an honest cycle time. Nothing commercial
   publishes a certified maximum for engagement, chip or scallop. Our exact geometry makes each one
   a computed bound, not a sampled estimate. This report is also the yardstick for every later
   step and for the Fusion comparison.
2. **Fix the cheap, big wins.** Each needs no adaptive clearing:
   - feeds from the real engagement, scheduled per move from the material removed in the
     in-process stock;
   - stay-down links proven to lie inside the cleared region;
   - corner rounding with the engagement checked;
   - per-tool ramp limits;
   - ordering;
   - an acceleration-aware cycle time;
   - 4-decimal finishing output with the controller's tolerance mode set from the op;
   - a scallop-driven stepover;
   - steep-and-shallow finishing.
3. **Adaptive clearing on the exact cleared region.** Control a single capped engagement bound
   and certify it on the exact cleared region, so tool load is bounded everywhere, inner corners
   included. Output open one-direction peel passes, link through the cleared area by straight
   moves checked against it (otherwise retract), and adapt the feed to the engagement. Every
   step is deterministic, so the same part gives the same path.

---

## 1. Findings by topic (the short version)

### 1.1 Roughing (roughing-linking.md)
- **Sharp corners of plain offset passes** [D]: the tool's engagement jumps to
  min(180°, φ_straight + the corner's turning angle), where φ_straight = arccos(1 − 2a_e/D).
  - forgeCAD's offset pockets see 127° in every 90° corner at 10% stepover, and 180° at 50%.
  - No stepover makes plain offsets constant-engagement.
  - Rounding the path corners to the tool radius (each inner pass's radius smaller by the
    stepover) cuts the 10% spike from 127° to about 52° [D, closed form checked to 0.01°].
- **Area control is not engagement control** [D]. Removed area per unit length is
  r(sin θ₂ − sin θ₁) over the engaged arc. The vendors bound different things:

  | target | tools | sources |
  |---|---|---|
  | engagement | Fusion; iMachining (a min/max band); Vortex (within 15% of straight) | [R], [S] |
  | removal rate (MRR) | VoluMill | its manual [R] |
  | chip load, by adjusting feed | Mastercam, ESPRIT | [S] |
  | area per unit advance | FreeCAD Adaptive | its source [R] |

- **FreeCAD Adaptive** (Kresimir Tusek; first commit 2018-08-19; LGPL-2.1+), from its source [R]:
  - a target area per unit advance, within ±5%;
  - a heading search over ±45° by guarded secant steps, at most 30 iterations;
  - links checked against the cleared polygons, with a wall-clock time limit;
  - open issues: load spikes when a loop closes (#34143) and short slots into inner corners
    (#29885).
- **Ibaraki 2004/2005** [R]: shaping the path held engagement at 11.5° instead of 19°.
  Regulating feed alone was limited by servo lag. Path shape beats feed for engagement; feed is
  the right tool for the *chip*.
- **Linking.**
  - The reachable feed on a path radius ρ is about √(a_max·ρ) [D; LinuxCNC documents the
    related F²/A rule, R]. So corner radius limits speed, not just tool load.
  - Inui et al. 2026 [R]: most of the ordering gain comes from not cutting air plus a greedy
    order; 2-opt and iterated local search add little.
  - LKH/GLKH is free for non-commercial use only, so we cannot use it.

### 1.2 Feeds and speeds (feeds-speeds.md)
- **Chip thinning** [R]: Helical's guidebook p. 22 and Sandvik's formulas page. Also covered:
  ball, lead-angle and round-insert forms [R], and bull nose and tilted ball [D].
- **Corners overload the chip.** At a_e = 10% D going into a 10 mm wall radius, engagement rises
  from 36.9° to 60.9°. A thinning-adjusted feed set on the straight then overloads the chip by
  46% in the corner [D]. Fusion, and any per-op feed, has this problem.
- **Cutting data.**
  - Openly licensed measured data: Klippel et al. 2022 (CC BY 4.0) gives kc1.1 and mc against
    cutting speed for Ck45 and Ti6Al4V [R]. mc ranges 0.17–0.51, so store it as an interval.
  - No openly licensed measured table for aluminium was found.
  - Helical's 6061 data (H45AL-3) [R]: values can be cited per value, not embedded as a table
    ("may not be reproduced").
- **Feed scheduling** [R]: per-move feed from the material removed in a solid model, under force
  and deflection limits (Spence & Altintas 1994; Erdim et al. 2006). It fits our exact stock
  directly.
- **Cycle time.** Ward et al. 2021 (arXiv:2102.02062) [R]: FIR models of the controller's
  interpolator predict cycle time to within about 90%. Ruckig (MIT) gives a jerk-limited lower
  bound, and was run here.
- **Chatter.** Lobes need a measured tool-point FRF. Without one, give advice only.
- **Provable.** Chip ≤ h* from an engagement bound (monotonicity), plus power and torque caps
  with interval coefficients. Taking upper ends is sound because h^(−mc) increases with mc for
  chips under 1 mm.

### 1.3 Surface finish (surface-finish.md)
- **Ball scallop** [D]: s²/(8R) ≤ h ≤ s²R/(8R² − s²). The common s²/(8R) *under-states* the
  scallop.
- **Bull nose on a slope** behaves like a ball of the corner radius r, not R [D].
- **Steep walls.** Parallel passes fail there because the spacing along the surface is s/cos θ.
  With stepover equal to stepdown, the crossover with waterline is 45° [D]. That matches the
  slope where Fusion adds perpendicular passes.
- **Ra** = 4h/(9√3) ≈ 0.257h for a parabolic cusp [D]: the origin of "Ra ≈ h/4". Real Ra
  depends on runout and vibration; a 2024 model is off by up to 13.5%. Predicted Ra stays a
  warning, not a guarantee.
- **Controllers** [R]: Heidenhain Cycle 32 and Siemens CYCLE832 manuals, LinuxCNC G64 P/Q, GRBL
  $11/$12. Each wants the CAM chord error well below the control tolerance and enough decimals.
  forge-post's fanuc, haas and grbl dialects print **3 decimals**; linuxcnc prints 4
  (`kernel/forge-post/data/*.toml:6`). The dialects hard-code `G187 P2` (Haas) and `G64 P0.01`
  (LinuxCNC). Finishing should post 4 decimals on every dialect, and the tolerance mode should come
  from the op.
- **Provable**: S1–S7, of which S3 is the differentiator:
  - S1, S4: scallop and chord bounds;
  - S2: the curvature bound the planner uses to pick the stepover;
  - S3: a certified maximum scallop from the swept tools, with the existing B&B checker;
  - S7: the controller tolerance as a named hypothesis.

### 1.4 Fusion (fusion-benchmark.md)
See §0 for its weaknesses.
- **"Optimal load"** is a cap on the radial width of cut. Help suggests 10–40% of D and claims
  the strategy "guarantees a maximum tool load at all stages" [R]. That claim is the first thing
  we test, on ours and theirs.
- **Defaults.** The help states almost no numeric defaults, so the checklist tests behaviour.
- **Parity numbers need Fusion.** Link lengths and cycle times on the same parts can only come
  from someone running Fusion (David, or a machinist). This is open question 1 in §5.

### 1.5 Open source (opensource-oracles.md)
- **Read only, never port: GPL code.** libactp (Freesteel, headers dated 2004, GPL-2+), CAMotics
  (GPL-2+), Grbl, PyCAM and jscut (GPL-3+), LinuxCNC tp (GPL-2).
- **LGPL, only in interface/ or verify/**: FreeCAD Adaptive, OpenCAMLib and OpenVoronoi are
  LGPL-2.1+. They are usable as separate-process oracles.
- **Permissive** (still never in kernel/, by our rule): Clipper2 (BSL-1.0), cavalier_contours
  (MIT/Apache-2.0), Ruckig (MIT), Boost Voronoi.
- **Kiri:Moto.** Its licence file says MIT, but 256 of 274 source files are headed "All Rights
  Reserved". Do not port.
- **Oracles for verify/, in order.** Run here so far: OCL drop-cutter (matched the closed form to
  4.4e-16), Ruckig, Clipper2 (pyclipr) and Boost Voronoi (pyvoronoi).
  1. OCL drop-cutter (finishing CL points);
  2. Clipper2 + cavalier_contours (offsets) and an independent **engagement meter** built on
     Clipper2;
  3. FreeCAD Adaptive run headless (conda-forge) as the open baseline to beat;
  4. Ruckig and the CAMotics planner for cycle time;
  5. Boost Voronoi for the medial axis.
- **opensource.md corrections**: the Adaptive author is Tusek, not Pavlovic; Adaptive.cpp is
  LGPL, not BSD; and PyPI `opencamlib` has wheels only for Python 3.7–3.11.

---

## 2. Bugs and corrections found in the repo

1. **`kernel/forge-tooling/src/feeds.rs`: power uses kc at the peak chip, not the mean**
   (verified in code: `let hm = hex.max(1e-4)`).
   - kc falls as the chip thickens, so power comes out about 12% low in a slot and about 18% low
     at a_e = 10% D. That is the unsafe direction for a spindle-power check.
   - The module docstring also calls the thinning factor k = 2√(r − r²) the factor for the
     "mean" chip; it is the factor for the peak chip (h_ex = f_z·sin φ).
   - Fix: hm = f_z·(cos φ_in − cos φ_out)/(φ_out − φ_in) (radians) over the engaged arc, kc at hm.
     forge-tooling's speed gate and bits apply.
2. **`cutting.toml` values have no per-value source**, against the standing "real data, cited per
   value" rule. Replace them with cited values (Klippel CC BY; Sandvik ranges cited by page),
   held as intervals.
3. **forge-post finishing precision.** 3 decimals on fanuc, haas and grbl (linuxcnc already prints
   4), plus the hard-coded `G187 P2` and `G64 P0.01` (surface-finish.md §5).
5. **Feeds at an assumed width of cut** (verified: `interface/forge-cam-lang/src/lib.rs:2109`).
   - `clear`, `contour`, `slot` and `engrave` take their feeds at ae = 0.1·D, whatever the op
     really cuts.
   - A slot cuts at ae = D. With the thinning factor for 0.1·D applied, the chip runs about 1.5–1.7×
     the target for 55–95% of the time (gap-tables.md, from the cam/quality harness).
   - This is a tool-life and breakage risk, and it is now the most urgent feeds fix.
6. **Linking at cutting feed** (`ops.rs`): every retract runs at cutting feed, and the pocket's
   helix or plunge repeats for every ring. On `thin_wall`, 355 of 522 s of cutting is at plunge
   feed.
7. **Cycle time in the reply** is length ÷ feed, with rapids at a fixed 10 m/min
   (`lib.rs:3263`). forge-machine on main already has an exact-stop trapezoid (`t_stop`) that the
   reply does not use.
4. **Corrections to research/cam notes:**
   - Adaptive's author and licence (above).
   - The opencamlib Python versions (above).
   - Stori & Wright 2000 appeared in *J. Manuf. Systems*.
   - Choy & Chan 2003 is corner *looping* (CAD 35(2)).
   - Fusion now filters cutting-data presets by stock material, which corrects
     libraries-machines-posts-feeds.md §3.3.

---

## 4. The ranked plan

**Where the code is** (gap-tables.md, read in the code):
- **No 3-axis surfacing exists on any branch.** The lowering takes planes, plus cylinders, cones
  and tori about z only. All 16 Fusion finishing strategies are absent.
- **Absent:** adaptive clearing (2D and 3D), rest roughing, contour leads, wear compensation (no
  G41/G42), tabs, V-carve and 3+2.
- **Hole cycles:** G73, G82 and G85 exist in the post, but a `.fg` file cannot reach them.
- **Turning:** part-off has no reduced feed near the centre and no pecking; threading has no
  first-pass depth, spring pass or flank infeed; the CSS speed caps are hard-coded.
- **Unmerged:** `origin/cam/quality` (aec87afa) already has the measuring half of row 1: a
  harness for cycle time, engagement, chip, MRR, air cutting, links and load over every example
  part, with a baseline.

gap-tables.md ranks the whole set; its top 5 agree with rows 1, 4, 2, 7 below and add **2D
contour basics** (lead arcs, finishing and spring passes, wear compensation certified over a
radius range, tabs). Its full top 15 also places the **3-axis base (Parallel)** at #10. It is
ranked there only because it is weeks of work: it is the largest capability gap.

Ranked by real-world gain (cycle time, tool life, finish) per unit of work.

Columns:
- **P** = what can be proved: theorems in Lean, so no runtime check is needed for that property;
  the certifier stays for input-dependent answers.
- **Ch.** = Fusion-checklist items in fusion-benchmark.md §4.

| # | build | gain | P (proof instead of check) | Ch. |
|---|---|---|---|---|
| 1 | **Quality report per op** in the `forgecad.cam/1` reply. Certified max engagement φ and chip h_max on the exact cleared region; air-cut %; retracts; min path radius; MRR profile; scallop max (S3). Plus a cycle time with per-axis accel/jerk (trapezoid now, FIR per Ward et al. next), checked never below Ruckig's bound. | Makes "good" measurable; it is the yardstick for every row below and for Fusion parity. A certified engagement, chip and scallop bound is something no commercial CAM publishes. | Engagement is exactly decidable on line/arc regions (L2). The removed-area identity; the corner formula (roughing-linking.md §6) | R1–R4, O-items, BETTER items |
| 2 | **Fix feeds.rs power (hm)**, with cited intervals for kc1.1 and mc; sin κr, bull-nose and tilted-ball D_eff | Correct power and torque limits; tool life | Monotonicity lemma: interval upper ends are sound | F-items |
| 3 | **Per-move feed from the real engagement** (material-removal feed scheduling on the exact stock): keep h_max ≤ h* on the in-process stock; cap by power, torque and a deflection limit at the interval upper bounds; corner and arc engagement per feeds-speeds.md §2.3 | Large: the corner overloads (46%+) go away; straights run faster. Fusion lacks this. | **The headline theorem**: every emitted move's chip ≤ h* (engagement bound + monotonicity). No commercial CAM claims it. | F-items (BETTER) |
| 4 | **Stay-down links** inside the cleared region; minimum retraction otherwise; G0 retracts; optional microlift | Cuts today's 2 × loops × levels vertical moves at cutting feed. Big cycle-time win on every pocket. | A link is inside the cleared region (exact containment) → G1-safe by construction; proved once for the construction | L-items |
| 5 | **Corner rounding** of offset passes with a user-set minimum radius, concentric; engagement *checked* (row 1), leftovers to rest | Corner spike 127° → about 52° at 10% stepover; higher reachable feed (√(aρ)) | Corner-formula theorem; the coverage theorem for offsets with stepover ≤ r | R-items |
| 6 | **Finishing output**: 4 decimals for finishing; control tolerance (G187/G64 P Q/CYCLE832/Cycle 32) from the op's tolerance budget; point redistribution at constant arc length; arc fitting with re-certification | Finish marks, faceting and dwell marks go away; controllers can look ahead | S4 chord bound, S6 rounding (G6 already), S7 as a named hypothesis | S/O-items |
| 7 | **Per-tool entry limits** (ramp angle, helix diameter range, centre-cutting) in forge-tooling, refused by the checker; predrill and outside entry | Tool breakage avoided | Entry-limit check proven complete over all entries an op emits | R/L-items |
| 8 | **3-axis finishing done right** (with CAM-3AXIS): exact drop-cutter, scallop-driven stepover (S2 per parameter box), Parallel + Contour + **Steep-and-Shallow** with overlap, then pencil and reference-tool rest; OCL as the L6 oracle | Finish quality at Fusion level | S1–S3; G1 on every segment (existing) | S-items |
| 9 | **Ordering**: greedy + deterministic 2-opt/Or-opt over regions and levels, exact comparisons (native = wasm) | Moderate (air time) | Containment order respected (proved) | L-items |
| 10 | **Stock-aware levels**: the exact in-process stock section and exact flat-face levels | Fewer air cuts in rest roughing | Exact | R-items |
| 11 | **Adaptive clearing**: a single capped engagement bound on the exact cleared region, certified by row 1; open one-direction peel passes; links through the cleared area; arc output; deterministic (same input, same path) | The biggest single roughing gain (full-flute depth at low radial width: tool life and MRR) | Engagement ≤ cap certified per op; links in the cleared region; coverage | R-items |
| 12 | Chatter advice (user FRF → zero-order and semi-discretisation lobes); NURBS output where the dialect supports it; 3+2, then 5-axis tool-axis control | Advanced | Lobes are advice, not provable | BETTER items |

**Why this order.** Row 1 is the measuring stick: without it no row can show it helped. Rows 2–5
are the largest real gains (feeds, links, corners) with no adaptive clearing at all. Row 3 is the
headline "better than Fusion": a proven chip bound and a feed that follows real engagement. Row
11 builds on rows 1, 3 and 4, which make it better than Fusion's when it lands.

**Benchmarks to prove "Fusion level or better"** (fusion-benchmark.md §4–5, opensource-oracles.md §5):

- **Parts:**
  - B1–B7: our own `.fg` parts with closed-form answers (L-pocket with 90° inner corners, a slot,
    an island pocket, a ramped floor, a hemisphere, a 30/45/60/75° slope set, a fillet set);
  - an ISO 10791-7-style test piece that we model ourselves (the standard is paid);
  - the NIST PMI test cases (free, acknowledgement requested);
  - the FreeCAD CAMTests models (LGPL).
- **Metrics:**
  - engagement p50/p99/max against the cap;
  - chip h_max against h*;
  - air-cut %, retracts, link length;
  - min path radius;
  - certified scallop max and p95 against the target;
  - point-spacing statistics;
  - cycle time from our planner, bracketed by Ruckig (lower bound) and the CAMotics planner;
  - generation time.
- **Comparisons:**
  - FreeCAD Adaptive on the same regions, measured by the *independent* Clipper2 engagement meter
    (open baseline: we must beat it);
  - OCL CL points (finishing correctness);
  - Fusion's numbers on the same parts, once someone runs Fusion (open question 1).
- **Pass:** every checklist item's threshold. Beyond that, a "better" claim needs a certified
  bound where Fusion offers only a nominal parameter.

---

## 5. Open questions for David

**Decided by the coordinator (2026-11-24):**
- (2) **Yes.** Default cutting data = cited tool-maker values, each value with its source (numbers
  with citations, not copies of documents), plus Klippel et al. 2022 (CC BY 4.0), with
  attribution in the data files and docs.
- (3) **Yes.** Machine acceleration and jerk stay UNKNOWN, with a conservative cycle-time range
  (lower and upper bound) until measured or published.

Item 1 below is with David.

1. Can you (or a machinist friend) run Fusion on the B1–B7 parts and export the toolpaths? Without
   that, "Fusion level" is tested against Fusion's documented behaviour, not its numbers.
2. Cutting data: is Klippel (CC BY) + cited tool-maker values (cited, not embedded) acceptable as
   the default data, with "UNKNOWN" where nothing open exists (e.g. measured aluminium kc)?
3. Machine dynamics: per-axis acceleration and jerk are rarely published. Should the machine
   library hold them as UNKNOWN, with a conservative cycle-time range, until a user measures them?
