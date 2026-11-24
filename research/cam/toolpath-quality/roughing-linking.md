# Roughing, step-down and linking: what "good" means and how to get there

Research only, 2026-11-24, branch research/toolpath-quality. It extends research/cam/literature.md
§1–3, opensource.md §2–3 and patents.md and does not repeat them. Read those first. The goal comes
from David: toolpaths at Fusion level or better, meaning adaptive clearing, feeds and speeds, and
surface finish. This note covers roughing, step-down, linking and the metrics. Finishing and feeds
are covered elsewhere.

**Marks** are the same as literature.md. **[R]** means read this session (the page, the PDF or the
source code). **[S]** means only a search snippet or a bibliographic record was seen. **[K]** means
background knowledge that was not re-read: treat it as a lead. One mark is added here. **[D]**
means derived in this note and checked numerically this session, with scripts kept in the session
scratchpad (`eng.py`, `arc.py`, `rounded.py`). A [D] result is mathematics, not a citation. It
should still be proven (§6) before anything relies on it.

**Patent gate.** forgeCAD has a binding rule: "no adaptive / constant-engagement clearing until
patent counsel". This note studies the algorithms anyway. Where a choice touches a known patent
family, it says so in one line and points to patents.md. A separate agent handles patents.

**What forgeCAD does today** [R, code read this session]:
- `kernel/forge-cam/src/ops.rs::pocket` builds exact offset loops, each offset from the *original*
  boundary at r + leave + k·s. It cuts them innermost first at each Z level from `levels()`, which
  makes equal steps of at most `step`.
- Entry is a helical ramp of radius D/4 when that disc fits exactly inside the first pass's
  centre region (checked with an exact boolean). Otherwise the tool plunges.
- After **every** loop the tool feeds up to `h.retract`, rapids across, and plunges or helixes down
  again. There are no stay-down links, no lead arcs, no corner rounding, and no engagement
  computation.
- The cycle time in `interface/forge-cam-lang/src/lib.rs` (around line 3212) is length ÷ feed, with
  rapids at a fixed 10 000 mm/min. It has no acceleration or jerk model.
- `forge-cam-geom` already has exact line and arc regions, exact offsets and exact booleans
  (phase1.md). That is the right base for everything in §6.

---

## 1. Engagement geometry (exact formulas)

**Definitions.** A tool of radius r (diameter D = 2r) has its centre at c and moves with velocity v.
Let *Cl* be the region already cleared, meaning earlier passes plus this pass's own sweep so far.
The **engaged set** is the set of points on the tool circle that border uncut material and face
forward: E = {c + r·u(θ) : u(θ)·v > 0, c + r·u(θ) ∉ Cl}. The **engagement angle** φ is the angular
measure of E. When E is a single arc this is the usual "angle from entry to exit" (Kramer 1992
defines minimal engagement by reference to a straight cut at constant step-over [S]; Ibaraki et
al. 2004 Fig. 1 [R]). A forward-moving tool cuts only its front half, so φ ≤ 180°. The value 180°
is a full slot.

**(1.1) A straight cut at radial depth a_e** [K, standard; and [D]]:
φ_s = arccos(1 − a_e/r) = arccos(1 − 2a_e/D).
Typical values are 10 % D → 36.9°, 25 % D → 60°, 50 % D → 90°, and 100 % D → 180°.

**(1.2) Area per unit length** [D]. This identity explains why FreeCAD's control is not the same as
engagement control. Measure θ from the velocity. If the engaged set is one arc [θ₁, θ₂], the area
swept into new material per unit advance is
dA/ds = ∫ cos θ · r dθ = r (sin θ₂ − sin θ₁) =: a_eff,
the "effective radial depth". For a straight cut θ₁ = −90° and θ₂ = −90° + φ, which gives
a_eff = r(1 − cos φ) = a_e. The same a_eff can come from very different arcs, though. An arc
centred on the front has a_eff = 2r sin(φ/2). At a_eff = r/2 (25 % D) that arc spans only 29°
instead of 60°, but its largest chip is the full f_z instead of f_z·sin 60°.

So **constant removed area per step (FreeCAD, VoluMill's MRR) is not constant engagement and not
constant chip thickness.** They agree only on side cuts.

**(1.3) Chip thickness** [K]. On a side cut the largest chip is h_max = f_z·sin φ for φ ≤ 90°, and
f_z for φ ≥ 90°. The mean chip over the arc is h_m = f_z(1 − cos φ)/φ, with φ in radians. On a
centre path of radius ρ, the cutting point moves faster or slower than the centre:
- feed at the contact point = F·(ρ + r)/ρ when the wall is concave (the tool inside an arc wall);
- feed at the contact point = F·(ρ − r)/ρ when the wall is convex.

That correction belongs to feeds. forge-tooling already has chip thinning (phase1.md), so only
the arc factor is missing.

**(1.4) Sharp corners of offset loops** [D]. This is the main result here, and I did not find it
stated this way in any source read.

Setup: offset (contour-parallel) loops made from the original boundary, spiralling out with
step-over s ≤ 2r. At a convex corner of the pocket (an "inside corner" of the wall, such as the
corners of a rectangular pocket), every inward offset keeps a sharp corner on the bisector.
Suppose the tool reaches the corner of loop k, the loop k−1 corner lies inward on the bisector,
and the path turns by the exterior angle β. Then the uncut arc at the corner instant is
**φ_corner = min(180°, φ_s + β)**, with φ_s from (1.1).

The engaged arc starts at the incoming leg's side normal. It ends φ_s past the outgoing leg's side
normal, and the two normals differ by β. Numerical check (`eng.py`, 2·10⁵ samples, error < 0.01°):

| a_e/D | φ_s | β = 30° | 60° | 90° | 120° |
|---|---|---|---|---|---|
| 0.05 | 25.8° | 55.8° | 85.8° | 115.8° | 145.8° |
| 0.10 | 36.9° | 66.9° | 96.9° | 126.9° | 156.9° |
| 0.25 | 60.0° | 90.0° | 120.0° | 150.0° | 180.0° |
| 0.50 | 90.0° | 120.0° | 150.0° | 180.0° | 180.0° |

The commonly quoted "inside corners go to 180°" is the case s = r: at a 90° corner with a 50 %
step-over the corner is a full slot. **forgeCAD's current pocket is in this situation.** A
rectangular pocket at 10 % step-over sees 127° in every corner, 3.4 times its straight-line value.

**(1.5) Arcs** [D, checked against `arc.py` to 0.01°]. The tool centre runs on a circle of radius ρ,
and the previous pass was concentric at ρ − s. That gives a cleared disc of radius R_c = ρ − s + r,
with material outside it, as in a concave wall or a rounded path corner. Then
**φ_arc(ρ) = arccos(((ρ − s + r)² − ρ² − r²) / (2ρr))**, which tends to φ_s as ρ → ∞. With r = 1:

| a_e/D | ρ = 0.5r | ρ = r | ρ = 2r | ρ = 4r | ρ = 10r |
|---|---|---|---|---|---|
| 0.10 | 63.9° | 51.7° | 44.8° | 41.0° | 38.6° |
| 0.25 | 104.5° | 82.8° | 71.8° | 66.0° | 62.5° |
| 0.50 | — | 120.0° | 104.5° | 97.2° | 92.9° |

A convex wall, such as an island or a reflex vertex of the pocket, lowers φ. The same formula holds
with the cleared region and the material swapped.

**(1.6) Rounding the corners** [D] (`rounded.py`, square pocket, 0.5° sampling). The table gives
the largest φ through a 90° corner:

| a_e/D | sharp | every loop rounded to ρ = 0.5r | ρ = r | ρ = 2r | concentric, outer ρ = r | concentric, ρ = 2r |
|---|---|---|---|---|---|---|
| 0.10 | 126.5° | 76.5° | 62.0° | 53.5° | 52.0° | 45.0° |
| 0.25 | 150.0° | 116.5° | 99.5° | 86.5° | 83.0° | 72.0° |

What this shows:
- Rounding the path corners cuts the spike by roughly 2 to 3 times.
- Concentric radii (each inner loop's radius equal to the outer loop's radius minus s) do better
  than one constant radius, because the cleared boundary then runs parallel to the path.
- Concentric radii shrink to 0 after ρ/s loops. The innermost loops are sharp again, and loop k
  with radius ρ_k = s sees arccos(−s/2r) = 90° + arcsin(s/D).
- Rounding leaves uncut cusps in the corners. They have to be covered by a later pass, or the
  coverage bound in §6 must be shown to still hold.
- Patent note: a chain of corner radii in which each radius is the previous plus the step-over,
  chosen from an engagement-change limit, is the claim language of DP Technology US 9,778,649
  (patents.md §2.4). A user-set constant ρ, with engagement only *checked*, is the clearer side.
  Counsel should confirm.

**(1.7) Other spikes.** These are qualitative and not computed here.
- **Topology events**, where an offset loop splits at a neck or a narrow channel, or where a loop
  vanishes: the next loop enters uncut material at up to 180°. Ibaraki, Yamaji, Matsubara (2010)
  locate exactly these "critical cutting regions" from the medial axis [S, literature.md §2].
- **Ramp and plunge entries.** A helix is a full-width cut of up to 180° at a small axial depth.
- **Links that cross uncut material.** FreeCAD's `IsAllowedToCutTrough` refuses a link once the
  area it would cut exceeds a fraction of the optimal area per step [R, Adaptive.cpp ~l.2494].

**(1.8) Reading the numbers.** Commercial "constant engagement" tools state a *tolerance* on φ:
- FeatureCAM Vortex keeps φ within 15 % above the straight-line value at the same step-over, and
  switches to trochoids when it would exceed that [R, Autodesk FeatureCAM help].
- iMachining keeps φ between a user minimum and maximum [R, SolidCAM page].

So "Fusion level" is a bounded φ band, roughly φ_s to 1.15·φ_s, not a constant φ. Plain offsets
spike to φ_s + β.

---

## 2. Roughing: the state of the art

### 2.1 History and lineage

- **1992, Kramer (NIST).** "Pocket milling with tool engagement detection," *J. Manuf. Syst.*
  11(2):114–123 [S, NIST record; citation also in Ibaraki 2004 R]. The algorithm detects
  minimal-engagement cuts, defined against a straight side cut, and resets feed and speed when the
  cut changes type. This is the earliest engagement-aware pocketing I found.
- **Iwabe et al.** (JSPE, year not read): an extra circular loop at a convex corner to cap
  engagement. [S, cited in Ibaraki 2004 and Uddin 2005, both R]
- **2000, Stori & Wright.** "Constant engagement tool path generation for convex geometries," *J.
  Manuf. Syst.* 19(3):172–184 [S, catalogue record; the citation is confirmed in Ibaraki 2004's
  reference list, R]. Note the journal: the brief said *J. Manuf. Sci. Eng.*, and that is wrong.
  - Method: a spiral-in path for convex pockets that tracks engagement, curvature and the entry and
    exit angles. Simulations show near-constant MRR.
  - Limits: convex pockets only. Run as a spiral-out, it leaves corner material [R, Ibaraki 2004].
- **2002–2003, Choy & Chan, corner looping.**
  - "Machining tactics for interior corners of pockets," *IJAMT* (2002).
  - "A corner-looping based tool path for pocket milling," *CAD* 35(2):155–166, 2003,
    doi:10.1016/S0010-4485(02)00049-0.
  - The cutter-swept-angle model, *Int. J. CAD/CAM* 3(1):1–12, 2003.
  - Method: loops added at concave corners remove the corner material progressively [S].
  - The brief called this a "corner trochoid". The papers call it corner looping.
- **2003, Bieterman & Sandstrom (Boeing).** "A curvilinear tool-path method for pocket machining,"
  *J. Manuf. Sci. Eng.* 125(4):709–715 [S, citation in several reviews].
  - Method: the path follows level sets of an elliptic PDE solution on the pocket. Later authors
    describe it as the principal eigenfunction / Laplace problem. The path morphs from a near-circle
    at the centre to the wall.
  - Results: about 30 % less time in aluminium, and about twice the tool life in titanium at a small
    increase in time [S].
  - Limits: nearly convex pockets only [S]. The path is numerical by nature.
- **2003–2005, Elber, Cohen, Drake, "MATHSM."** *CAD* 37(2):241–250, 2005 [R, preprint dated
  2003-12-31].
  - Method: maximally inscribed circles along the medial axis, joined by bitangent lines or biarcs,
    to give a C¹ path made mostly of circular arcs with the largest possible radii, so the radial
    acceleration is lowest.
  - The authors say it suits long, narrow regions and should be combined with standard methods in
    near-circular regions. It can create a "redundant front" where the inscribed radius changes
    fast.
- **2004–2005, Ibaraki et al., Kyoto.** These are the constant-engagement offset papers [R, both
  PDFs from Ibaraki's page]:
  - JUSFA 2004, "Constant engagement tool path generation for two-dimensional end milling": Ibaraki,
    Ikeda, Yamaji, Matsubara, Kakino, Nishida.
  - LEM21 2005, "...to enhance machining accuracy in end milling": Uddin, Ibaraki, Matsubara,
    Nishida, Kakino.
  - Algorithm I moves each point of an offset path along its normal by one Newton step per point,
    with a numerical derivative, until φ reaches the target. The authors say it has no convergence
    guarantee. Spiral-in it is fine. Spiral-out it leaves corner material.
  - Algorithm II instead modifies the *previous* (inner) path, so that the next path, which is kept
    exact, sees a constant φ. That preserves the final contour.
  - Experiment (SKD61 at HRC 53, a 10 mm 4-flute end mill): φ held at 11.5° instead of rising to
    19° on the corner arc. The force variation fell to within 17 N, and the corner error fell from
    14 µm.
  - Feed-rate regulation reached similar force control, but servo lag at the corner entry and exit
    limited it. This is a useful datum for "path shaping beats feed override".
- **2007–2010, trochoidal.**
  - Otkur & Lazoglu (*IJMTM* 47(9), 2007): closed-form chip thickness for trochoidal and double
    trochoidal milling [S].
  - Rauch, Duc, Hascoët, "Improving trochoidal tool paths generation and implementation using
    process constraints modelling," *IJMTM* 49(5):375–383, 2009 [S, reference list], plus their
    2007 and 2008 papers on trochoid versus plunge roughing [S].
  - Ibaraki et al. 2010 *Prec. Eng.*: trochoidal grooving only where the medial axis says an offset
    would overload the tool [S, literature.md].
- **2004–2010, high-speed machining corners: Pateloup, Duc, Ray.**
  - "Corner optimization for pocket machining," *IJMTM* 44(12–13):1343–1353, 2004 [S, cited].
    Corners are rounded with arcs so the feed does not drop at the corner.
  - "B-spline approximation of circle arc and straight line for pocket machining," *CAD*
    42(9):817–827, 2010 [S].
  - The open pages on HAL were behind Anubis (unreachable).
- **2007, Zhao, Wang, Zhou, Qin.** "Pocketing toolpath optimization for sharp corners," *J. Mater.
  Process. Technol.* 192–193:175–180 [S]. Biarc transitions at corners remove the leftover at large
  step-overs and keep the path "second-order continuous", which allows a wider step-over.
- **2008–2018, Held, Spielberger, de Lorenzo, spirals** [R, Held's project page]:
  - Held & Spielberger 2009, *CAD* 41(7):539–550, doi:10.1016/j.cad.2009.04.002. A spiral out from
    a chosen point. It does not cross itself, is G¹ (G² after smoothing), and its cutting width
    stays above zero and below a user threshold everywhere.
  - Held & Spielberger 2014, *CAD&A* 11(3):346–357, doi:10.1080/16864360.2014.863508. A
    Voronoi-based decomposition of pockets with islands into sub-pockets suited to a spiral, plus a
    start-point heuristic.
  - Held & de Lorenzo 2018, *JCDE* 5(3):348–357. A simplified version with multiple spirals.
  - Claims: the experiments show shorter paths and less variation in curvature and engagement. The
    page states no hard engagement bound.
- **2014–2019, Abrahamsen.**
  - Spiral toolpaths, arXiv:1412.5034 [R, literature.md]: G¹, no self-crossing, successive turns at
    most δ apart.
  - Abrahamsen & Thorup, "How to cut corners and get bounded convex curvature," arXiv:1603.02080
    (SoCG 2016; *DCG*) [R, abstract]. For a region bounded by n segments and arcs, it computes the
    **unique largest subset whose boundary never turns left more sharply than a unit circle**, in
    O(n²) time, or O(n log n) when the region is simply connected. This is exactly a "minimum
    cutting radius" done right, and it works on forgeCAD's line-and-arc input.
  - Note: Abrahamsen also holds US 10,108,172, "Spiral toolpaths for high-speed machining of
    polygonal pockets" [S, Google Patents hit]. It is not in patents.md, so the patent agent should
    check it.
- **2012, Banerjee, Feng, Bordatchev.** "Process planning for floor machining of 2½D pockets based
  on a morphed spiral tool path pattern," *Comput. Ind. Eng.* 63(4):971–979 [S]. This is the
  morphed-spiral lineage from Bieterman. A review lists it for globally convex pockets with a
  centred island [S].
- **Patel & Lalwani** (Inderscience, mid-2010s): spirals for 2.5D star-shaped pockets, and
  comparisons of zigzag, contour-parallel and spiral paths [S]. I found no engagement paper by them.
- **Medial-axis families**, beyond Held, Elber and Ibaraki:
  - "Kim/Hatna" and "Huang" were not looked up and are **UNKNOWN**.
  - Celeritive US 10,579,040 builds paths from a Voronoi diagram with synthetic branches (patents.md
    §2.2). Using a plain medial axis is the design-around.

### 2.2 Freesteel → HSMWorks → Fusion: what can be established

- libactp ("Adaptive Clearing Tool Path Library") describes itself as an implementation of "the
  GPL'ed algorithm demonstrated in FreeSteel" [R, GitHub About text]. The page shows no dates and no
  authors.
- A Practical Machinist thread says Julian Todd was a prominent HSMWorks developer, and that the
  Freesteel blog is "where HSMWorks Adaptive clearing came from" [S, forum, undated in the snippet].
- An Autodesk forum post credits Martin Dunschen with the Adaptive Clearing entry positions in
  HSMWorks 2017 [S].
- Autodesk bought HSMWorks ApS (Copenhagen) on 2012-10-01 [S, several press copies].
- The HSMWorks 2014 release notes call Adaptive Clearing a constant-tool-engagement strategy, and
  the 2015 release says it was significantly updated [S].
- Fusion renamed Adaptive Clearing to "2D/3D Adaptive Roughing" [R, Fusion help].
- The **Freesteel blog (freesteel.co.uk/wpblog) returned 403**, and web.archive.org cannot be
  fetched from this session.
- So the "2004–2006" dates rest only on the [S] claims in opensource.md and patents.md. **UNKNOWN**
  until someone reads the blog. This matters for prior art (patents.md §2.1): it needs a human with
  a browser.

### 2.3 Commercial systems: what each publicly says it does

| system | control variable | path shape | how it handles corners and narrow places | links and lifts | source |
|---|---|---|---|---|---|
| Fusion 2D/3D Adaptive (ex-HSMWorks) | "Optimal load" = the largest engagement, by varying step-over | adaptive, stock-aware; optional slot first, then spiral to the walls | "Minimum cutting radius" blends inside corners and leaves rest material; feed optimisation slows at corners | stay-down level 0–100 %, maximum stay-down distance, minimum stay-down clearance, lift height, no-engagement feed; retraction full / minimum / shortest path; G1 at high feed instead of G0 | [R] Fusion 3D Adaptive reference |
| FeatureCAM / PowerMill Vortex (Delcam) | φ no more than 15 % over the straight-cut φ at the step-over | offsets, with trochoids where φ would exceed the limit | "Vortex min radius" for the inner trochoids; "Z lift" during trochoids | retract or raise the feed on non-cutting moves | [R] FeatureCAM help; Vortex step-downs up to 2–3×D are a trade-press claim [S] |
| Mastercam Dynamic | constant chip load from radial chip thinning (marketing) | "material-aware", adjusts the path in real time | — | micro lift distance and feed when a reposition is shorter than the "gap size"; back-feed rate; rules for when to lift | [R] Mastercam blog (vague); [S] tutorials for micro lift and back feed |
| VoluMill (Celeritive, now in GibbsCAM and others) | **constant MRR**: never above the MRR of a straight cut at the set Z step, cut width and feed; slots get a lower feed and/or more Z steps | tangent-continuous; "not trochoidal" [S]; side-mill only or auto side/slot by cycle time | Min Toolpath Radius, default 45 % of D, may leave uncut areas | Floor Clearance: a reposition inside the cleared area lifts along a helix by this much and travels at "High Feedrate" | [R] GibbsCAM VoluMill manual v14 (2020) |
| SolidCAM iMachining | φ between a user minimum and maximum; feed varies with the width of cut to hold the spindle load | "morphing spirals"; "moating" splits regions; trochoid-like moves near walls | the Technology Wizard sets feed, speed, a_p, φ and chip thickness per point; slider levels 1–8 | moves at depth, retracts only when needed; stock tracked | [R] SolidCAM page (marketing; 70 % / 5× claims unbacked) |
| ESPRIT ProfitMilling (DP Tech, now Hexagon) | constant chip load by changing the feed; light radial cut at full depth | spiral out to open space, trochoids in slots, offsets in open zones | "corner picking" for constant φ | "small Z lifts on transitional moves", blended transitions; bottom-up and Z-level roughing | [R] ESPRIT page (75 % / 500 % claims unbacked) |
| FreeCAD Adaptive (Kresimir Tusek 2018, LGPL-2.1+) | **removed area per unit length** = the nominal step-over (see 2.4) | stock-aware marching; helix entry | none explicit; area control covers it | keep-tool-down ratio; links through cleared area checked by `IsClearPath` | [R] Adaptive.cpp and .hpp, wiki mirror |

Note: Kresimir Tusek is the copyright holder in the file header (opensource.md). The brief's
"Kreso Pavlovic" is not supported by the source.

**What "constant X" each one picks, side by side:**
- Constant engagement: Fusion, Vortex, iMachining as a band, Ibaraki.
- Constant MRR: VoluMill, and FreeCAD per unit length.
- Constant chip thickness: Mastercam, ProfitMilling, and the iMachining wizard. Each gets there by
  changing the feed on top of a path that already limits engagement.

By §1.2 these are three different targets. In practice:
- engagement limits radial force and heat;
- MRR limits spindle power;
- chip thickness keeps the edge loaded to its design value.

A Fusion-level system bounds φ by path shape, then sets the feed per move from h_max. That is the
Ibaraki result: path shaping is more robust than feed override.

### 2.4 FreeCAD Adaptive: the open reference algorithm

All of this was read in `Adaptive.cpp` and `Adaptive.hpp` this session [R]. The comment
"bisection on angle" in the brief is close but not exact.

- **Units and robustness.** Integer coordinates scaled so that the minimum step is 48 integer units
  (`MIN_STEP_CLIPPER = 16·3`). The header explains the choice from integer rounding and a smallest
  step-over of 1 %. Clipper2 does every boolean.
- **Target.** `referenceCutArea` is the area of the tool disc minus the disc shifted by r/2.
  `optimalCutAreaPD = 2·stepOverFactor·referenceCutArea / r` is the target **area per unit
  distance**, which is about the step-over a_e (§1.2).
- **Per step.**
  - The step length is 1 to 8 times the base step. It is shorter near the boundary or the entry,
    and shorter at large turning angles.
  - The heading is predicted from the last 3 angles.
  - Up to 30 iterations search the turn angle in [−45°, +45°]. Iteration 1 tries the most-engaging
    side and iteration 2 the least.
  - When the area error brackets zero, it interpolates linearly but keeps the fraction p within
    [0.2, 0.8]. The source calls this "a compromise between binary search … and following linear
    interpolation", a safeguarded regula falsi.
  - It stops when |area − target| ≤ 5 % (`AREA_ERROR_FACTOR`). It tracks conventional versus climb
    and rejects conventional cuts (cutoff 0.51).
  - If nothing is found, it expands the cleared area and steers toward the nearest uncleared point
    inside a ±45° triangle ahead.
- **Area evaluation.** `CalcCutArea` sweeps vertical slices through the crossings of the circle with
  the polygons. The cleared area is unioned in, with cached local windows.
- **Linking.**
  - `ResolveLinkPath` searches a link through the cleared area, accepted while its length is at most
    `keepToolDownDistRatio` times the straight distance, under a time limit.
  - `IsClearPath` checks it against the cleared polygons with a safety clearance.
  - `MakeLeadPath` builds the lead-in.
  - Otherwise it retracts.
- **Finishing.** An optional profile at 1/10 of the step-over.
- **Known issues** [R, wiki]: stray paths, worked around with 0.001 mm of stock to leave; not all
  tool shapes honoured; slow on large areas.

**Read for forgeCAD.** It is a robust, simple controller on a *sampled* state, using integer
polygons. Every decision is a float area compared with a 5 % tolerance. It has no bound on φ
itself: area is not angle, as §1.2 shows. Its structure (cleared-region state, a 1-D search per
step, links checked against the cleared region) is what patents.md §3 calls the probably clearer
design. Counsel still has to confirm.

### 2.5 Trade-offs

Grades: ++ best, + good, 0 neutral, − poor. These are judgements from the sources above, not
measurements.

| family | φ uniformity | path length | compute | smoothness (G1/G2, radius) | robustness (islands, necks) | patent exposure (patents.md) |
|---|---|---|---|---|---|---|
| offsets (forgeCAD today) | − : spikes to min(180°, φ_s+β) (§1.4) | ++ shortest | ++ exact, cheap | − : sharp corners, G0 | ++ any region, exact | none |
| offsets + rounded corners, checked φ | 0/+ : 2–3× lower spikes (§1.6) | + | + | + : G1, with a stated min ρ | + : cusps need coverage proof | low; avoid ESPRIT's chained-radius claim |
| offsets + corner loops (Choy & Chan; Iwabe) | + at corners | − : loops add length | + | 0 | + | old academic; Truemill 8,560,113 for corner arc series, to 2031 |
| constant-engagement offsets (Stori; Ibaraki) | ++ on convex parts | + | 0 : Newton per point | + | − : convex only / leftovers | the basic "cap engagement" family expires 2026-12-17 |
| PDE spiral (Bieterman) | + | + | − : numerical PDE | ++ | − : nearly convex, no islands | none known |
| geometric spiral (Held–Spielberger; Abrahamsen) | + (experiments) | + | 0 : Voronoi | ++ : G1/G2, no self-crossing | + : decomposition for islands | Abrahamsen US 10,108,172 [S], unchecked |
| medial-axis circles (MATHSM) | 0 | − | 0 | ++ : arcs, large radii | + : narrow regions | Celeritive Voronoi only if synthetic branches |
| trochoids (Otkur; Rauch; Ibaraki) | ++ bounded by construction | −− : most of it is air | ++ | + : arcs | ++ | plain trochoid is old prior art; specific constructions patented |
| stock-aware adaptive (Fusion, FreeCAD) | ++ / + | + | −− : sampled, iterative | + : smoothing pass | + : heuristics, stray paths | **high**: SolidCAM, Celeritive, Truemill (patents.md §3) |

---

## 3. Step-down strategies

- **Full flute length, low radial depth.** This is the HSM default for adaptive paths.
  - ProfitMilling: full axial depth with light radial engagement. 27–37 % in hard metals, "just
    under 90 %" in soft ones [R, ESPRIT page].
  - Vortex: up to 2–3×D axial in trade press [S].
  - iMachining: full depth [R].
  - The precondition is a bounded φ. With sharp-corner spikes (§1.4), full-depth cutting breaks
    tools. That is why forgeCAD's current pocket must not offer it.
- **Equal levels.**
  - forgeCAD's `levels()` already splits the depth into equal steps of at most `step` [R].
  - VoluMill also lowers the step so that every pass removes the same material ("Actual" Z step)
    [R].
  - Both support "Depth First", which is Fusion's "Order by area", against level by level, which is
    Fusion's "Order by depth" [R].
- **Step-up / fine step-down.** After deep roughing levels, extra levels are cut *upward* between
  them to take off the terraces.
  - Fusion: "Fine stepdown" [R].
  - Vortex: "step cutting" [R].
  - VoluMill: "Hit flats by adding passes" or "after each pass" [R].
  - Patent note: SolidCAM US 9,690,282 claims step-up cuts that keep all remaining surfaces between
    an inner offset surface and an outer scallop surface (patents.md §2.3). Plain fine step-down to
    fixed heights is older practice [K]. Counsel should confirm.
- **Flat-area detection.**
  - Fusion detects flat areas and peaks and adds levels at those heights, with a minimum step-down
    and a minimum axial engagement. It warns that this is slow [R].
  - The forgeCAD fit is exact. The B-rep lowering already knows every planar face's height
    (lower.rs). A flat face at height z_f between two levels means adding the level z_f + leave.
    This is a set of exact heights, not a detection problem.
- **Machine shallow areas** (Fusion): extra levels where the step-over on the slope would exceed a
  threshold [R]. This belongs to 3D roughing and is out of scope for 2.5D.
- **Stock-aware levels and rest.**
  - Fusion's rest source can be earlier ops, a file, bodies or the setup stock, with "ignore stock
    less than" [R].
  - iMachining tracks the stock dynamically [R].
  - forgeCAD has the exact in-process stock for 2.5D (stock.rs: box minus proven-cleared voids).
    So "cut only where stock remains at this level" is region ∩ stock_section(z), an exact boolean
    that is already available. Air-cutting levels above a facing op or a previous setup then cost
    nothing.

---

## 4. Linking and ordering

### 4.1 Stay down versus retract

- **Stay down through the cleared region.**
  - Fusion: stay-down level, maximum stay-down distance, minimum stay-down clearance, and a
    separate no-engagement feed [R].
  - FreeCAD: link length at most ratio × straight distance, checked against the cleared polygons
    [R].
  - VoluMill: Floor Clearance helix plus High Feedrate [R].
- **Retraction policy.** Fusion has three [R]:
  - *Full*: up to the retract height.
  - *Minimum*: up only to the lowest height that clears, plus a safe distance.
  - *Shortest path*: a straight diagonal. Fusion warns not to use it on machines whose G0 is not
    linear (dog-leg rapids).
- **G0 versus high-feed G1.** Fusion's "High Feedrate Mode" turns rapids into G1 at a set feed, so
  that the motion is predictable (a dog-leg G0 is not a straight line) [R].
  - forgeCAD's checker certifies G0 as the straight segment (phase1.md G3). For a dog-leg
    controller the post must either emit G1 or certify the dog-leg path. The dialect TOML should
    state which.
- **Microlift.** While repositioning inside the cleared area, the tool lifts a little off the floor
  so it does not rub the floor or recut chips.
  - Mastercam: micro lift distance and feed, back-feed rate, and rules for when to lift [S].
  - VoluMill: Floor Clearance [R]. Vortex: Z lift on trochoids [R]. ProfitMilling: small Z lifts
    [R]. Fusion: lift height [R].
  - For forgeCAD, a microlift link is a 3D segment (or a helix) at z_floor + δ, and it must be
    certified against the in-process stock at that moment (G3 with the current stock, not the final
    part).
- **forgeCAD today:** a full retract after every loop, and the retract itself at the cutting feed.
  With 40 loops × 4 levels that is 160 up-and-down cycles. Fixes, in order:
  1. Link consecutive offset loops at the same level with a short in-plane move through the cleared
     region: loop k ends next to loop k+1's start, and the band between them was cut. The exact
     boolean can prove the link segment's swept disc lies in the cleared region plus the current
     cut. This is the FreeCAD `IsClearPath` idea done exactly. Design-around note from patents.md
     §2.2: checking a link against the cleared area is fine; building separate "transition
     toolpaths" that cross the cuts and trimming the cuts against them is Celeritive's claim.
  2. Minimum retraction between regions.
  3. A rapid retract (G0) instead of a feed retract.

### 4.2 Ordering

- **Within an op.** For offset loops the containment tree fixes a partial order: inner before outer
  when spiralling out (Held 1991; literature.md §1). Choose the start point on each loop so it is
  nearest to the previous end, or at the loop's corner bisector to hide the entry mark [K].
- **Across regions and levels.** This is a TSP over regions with entry and exit pairs. Castelino,
  D'Souza & Wright 2003 model it as a **generalised TSP with precedence constraints**: each cell is
  a cluster and its entry/exit pairs are the cities. *J. Manuf. Syst.* 22(3):173–180 [S, record].
  Follow-ups [S]:
  - Oysu & Bingul 2009: genetic algorithm with simulated annealing.
  - Ant-colony variants.
  - A 2019 *JMST* paper claiming up to 40 % less air time.
- **Solvers.**
  - 2-opt and Or-opt are the cheap local improvers.
  - LKH, and GLKH for the equality-GTSP through the Noon–Bean reduction (Helsgaun 2015, *Math.
    Prog. Comp.* 7(3):269–287) [S]. Free for academic, non-commercial use only. **The licence is
    unusable for forgeCAD**, so the ideas would have to be reimplemented.
  - GLNS (Smith & Imeson 2017) is a competitive alternative [S].
- **Recent evidence.** Inui, Nakatsuji, Nishida, *IJAT* 20(4):369–381, 2026 [R]:
  - greedy, then 2-opt, then iterated local search with double-bridge moves, over dexel-detected
    rest regions;
  - semi-finishing time fell from 90.1 to about 22 min, and from 52.9 to about 11 min;
  - local search improved on greedy only slightly.
  - The lesson: most of the gain is in *not cutting air* and in a sensible greedy order, not in
    optimal TSP.
- **Determinism.** forgeCAD requires native and wasm bit-identical. A TSP heuristic with ties broken
  by exact comparisons (exact lengths are algebraic, or compare squared lengths) and no randomness
  meets that. Iterated local search with random double-bridge moves would need a seeded,
  specified PRNG.
- **Turn costs.** Arkin, Bender, Demaine, Fekete, Mitchell, Sethia, "Optimal covering tours with
  turn costs" (arXiv:cs/0309014, *SIAM J. Comput.*) [R, abstract]. Minimum-turn milling is
  NP-complete. Approximations exist, with a PTAS for some versions. Turns cost time on a real
  machine, which supports smooth links (4.4).

### 4.3 Entries

- **Helix** (forgeCAD has one).
  - Parameters [R, Fusion reference]: ramping angle, helix taper angle (a conical helix helps chips
    get out), **maximum ramp step-down per revolution**, ramp clearance height, maximum and minimum
    helix diameter.
  - Fusion's note: a helix near 1.8×D can leave a boss in the centre; about 0.8×D avoids it [R].
    Geometric reason [D]: a centre-cutting end mill on a helix of centre radius ρ_h leaves a core
    post of radius ρ_h − r when ρ_h > r. With r = D/2 and helix *diameter* 1.8D, ρ_h = 0.9D > r,
    so a post of radius 0.4D is left. forgeCAD's helix radius D/4 leaves none.
  - The real limits are per tool, from the maker. ISCAR: helix angle no more than the cutter's
    maximum ramp angle RMPX, and feed per tooth reduced by 30–40 % [S]. Sandvik: the maximum ramp
    angle depends on the insert radius, and the pitch per turn can never exceed the cutter's maximum
    a_p [S, page summary].
  - forge-tooling should carry `ramp_max_deg`, `helix_d_min`, `helix_d_max` and `plunge: bool`
    (centre cutting) per tool, required with no default, sourced, "verify with the vendor".
  - The checker should refuse a helix steeper than `ramp_max_deg`, as G5 does for feeds.
- **Ramp along a path** (zigzag or profile ramp). A linear or along-the-profile ramp at the same
  angle limit, used when no helix disc fits. forgeCAD's `slot` already ramps along the first line
  (phase1.md).
- **Predrill.** Fusion takes predrill positions from existing holes [R]. VoluMill has predrilling
  [R, section heading only]. In forgeCAD this is natural: the `holes` op exists, and the entry can
  be a drilled point inside the region's centre zone, ordered before the pocket.
- **Outside entry.** Fusion's "Machine cavities" off means machining only from the outside in. For
  open pockets, enter from outside the stock in air: no ramp is needed [R].

### 4.4 Leads and smooth transitions

- **Lead arcs.** Fusion has a horizontal and a vertical lead radius [R]. A tangent arc into the cut
  is a line-and-arc construction, so it is exact.
  - Its engagement grows from 0 to the cut's value, so the entry has no spike.
  - On a closed loop, a lead-in arc and a lead-out arc that overlap slightly hide the witness mark.
- **Corner smoothing tied to machine limits.**
  - Centripetal acceleration on radius ρ at feed v is v²/ρ, so the controller limits the feed on an
    arc to v ≤ √(a_max·ρ) [K, kinematics].
  - LinuxCNC: to hold feed F with acceleration A, a move must be at least F²/A long. Blending
    (G64 P tolerance, Q naive-CAM merging) sacrifices path accuracy to keep the feed [R, LinuxCNC
    docs].
  - Consequence: a path's minimum radius **sets its achievable feed**. Example: a_max = 2 m/s² and
    v = 10 m/min (0.167 m/s) need ρ ≥ 13.9 mm.
  - Hence VoluMill's default Min Toolpath Radius of 45 % D, and its warning that "it may leave
    areas uncut" [R]. Fusion's minimum cutting radius is the same idea [R].
- **G1 versus G2 continuity.** Offsets of lines and arcs with rounded corners are G1. Arcs joined to
  lines are G1 with jumps in curvature (G2 discontinuities), which jerk-limited controllers see.
  - Clothoid or B-spline corner blends make G2 (Pateloup et al. 2004/2010 [S]; Zhao et al. 2007
    biarcs [S]; Erkorkmaz & Altintas 2001, quintic splines and jerk-limited feed [S,
    literature.md §9]).
  - B-splines and clothoids leave the line-and-arc algebra. forgeCAD should **keep G1 lines and arcs
    with a certified minimum radius** and let the controller's look-ahead blend.
  - Fusion's "Smoothing" pass also fits tangent arcs and lines within a tolerance [R].

---

## 5. Metrics: how to measure roughing quality

All of these are computable from the posted path plus the exact in-process stock. Each should be in
the `forgecad.cam/1` reply per op, so that "better than Fusion" is a measured claim.

| metric | definition | exact in forgeCAD? | target, from sources |
|---|---|---|---|
| engagement φ: max, p99, histogram | φ(s) along every cutting move against the exact cleared region (§1) | yes, piecewise algebraic (§6) | max ≤ 1.15·φ_s (Vortex [R]); no point at 180° except declared slots and entries |
| effective radial depth a_eff and MRR = a_eff·a_p·F | §1.2 | yes | variance small (VoluMill's constant MRR [R]) |
| h_max and chip-thickness variation | f_z·sin φ (≤ 90°) with the arc factor (§1.3) | yes, given the feed | within the tool's chip band |
| cycle time | planner model: trapezoid or jerk-limited speed per segment, junction speed limits, a_max, j_max, look-ahead; rapids at machine G0 rates per axis | estimate (an empirical controller); state the model | compare with today's length/feed, which overstates speed on short arcs; Monreal & Rodriguez 2003 show that the acceleration model captures even zigzag orientation effects at high feed [S] |
| air-cut % | length of feed moves with φ = 0 ÷ all feed length | yes | Inui et al. 2026: the main lever [R] |
| retracts | count, and total vertical travel | yes | few: stay-down |
| path length | split into cut, link, rapid | yes | — |
| minimum radius of curvature | the smallest arc radius and the number of sharp G1 corners, each compared with √(a_max·ρ) against the commanded feed | yes | ≥ the tool's min ρ (e.g. 45 % D in VoluMill [R]) |
| leftover | the G4 regions after roughing: max thickness above `leave` | yes, already built (phase1.md G4) | ≤ leave + ε |
| entry and ramp compliance | helix angle against `ramp_max_deg`, diameter limits | yes | 100 % |

**Cycle-time model.** The current one is length ÷ F with 10 m/min rapids [R, lib.rs]. A
Fusion-grade estimate needs at least the following. Each should be marked "estimate" in the reply,
like the feeds.
- Per-axis a_max and the rapid rate in the machine TOML.
- A junction-speed limit at every corner, from the angle and the blend tolerance (LinuxCNC G64 P
  [R]).
- The arc limit √(a_max·ρ).
- A forward/backward pass to get the trapezoid profile.

---

## 6. What is exact and provable in forgeCAD's framework

forgeCAD's stance (literature.md, implications 5): generators may be anything; prove the output.
Roughing adds two kinds of property. *Safety* (G1–G5) is already proven. *Quality bounds* are new.
Quality bounds are not needed for safety, but they turn "good" into a theorem or a certified
number.

**(6.1) Coverage theorem: offsets with s ≤ r** [D; a Lean-sized proof sketch].
Let R be a closed region, d(p) = dist(p, ∂R), and L_k = {d = r + k·s} for k = 0..K, the exact
offset loops forgeCAD builds, with K the last nonempty one. If s ≤ r, then
⋃_k (L_k ⊕ B_r) ⊇ open_r(R) = (R ⊖ B_r) ⊕ B_r.

Proof sketch:
1. Take p with r + ks ≤ d(p) ≤ r + (k+1)s. Let b be its nearest boundary point. Along the segment
   [p, b], d falls at exactly rate 1, because d(x) = |x − b| on it. By continuity the segment meets
   L_k at distance d(p) − r − ks ≤ s ≤ r from p.
2. Points beyond L_K are within s of L_K, since L_{K+1} is empty.
3. Take q in the opening with d(q) < r. Then q ∈ B(c, r) for some c with d(c) ≥ r. The segment
   [c, q] crosses L_0 at a point m, and |q − m| ≤ r.

Mathlib has `Metric.infDist` and the 1-Lipschitz lemmas, so this is a small L1 theorem.
- The rate-1 fact is the only geometric lemma.
- The bound s ≤ r is sharp at convex corners. It agrees with "uncut when r < s < 2r" in Park & Choi
  2001 [S].
- This proves forgeCAD's current pocket leaves nothing inside the opening. Today that is checked
  after the fact by G4.

**(6.2) Corner engagement theorem** [D]. Under the local hypotheses of §1.4 (a convex vertex of R
with exterior angle β, loops k−1 and k both having their corner on its bisector, no other cleared
material nearby), the uncut arc at the corner is exactly min(π, arccos(1 − s/r) + β).
- **Corollary.** Every offset pocket with a corner of exterior angle β has max φ ≥ min(π, φ_s + β).
  So no step-over makes plain offsets "constant engagement" in a rectangle. That is a clean,
  provable negative result to put in the docs.
- **Positive form.** If the path is rounded with concentric radii ρ_k ≥ ρ₀ at each corner, then on
  those arcs φ ≤ φ_arc(ρ₀) from (1.5). The arcs are the worst places when ρ is small, and φ_arc is
  monotone decreasing in ρ (checked numerically). The innermost loops, where the concentric radius
  would go below ρ₀, need a separate treatment, for example a trochoid or a smaller tool.

**(6.3) Engagement as an exactly decidable quantity.**
- The cleared region in forgeCAD is an exact line-and-arc region: the union of the tool discs swept
  along lines and arcs, which are offsets. The tool circle meets lines and circles at
  quadratic-extension points.
- Along a *line* move c(t) = c₀ + t·v, each crossing angle θ_i(t) between the tool circle and a
  boundary element is an algebraic function of t. φ(t) is a sum of differences of these angles, and
  the active set changes only at finitely many events where the circle becomes tangent or meets a
  vertex. Those events are roots of degree-2 or degree-4 polynomials in t.
- So "max_t φ(t) ≤ φ_max" is decidable exactly by events plus monotonicity between them. A cheaper
  route is the same branch and bound as check.rs: interval enclosures of cos θ_i over t-cells, with
  `Clear` only when the proven bound holds.
- Compare cos φ, or the chord, never radians: φ ≤ φ_max ⇔ cos φ ≥ cos φ_max (cos falls on [0, π]), as literature.md §2
  says.
- This gives a **certified engagement report**: "max φ = 83.0° ± 0 at s = …", which no commercial
  tool states.

**(6.4) Exact a_eff and MRR.** By §1.2, a_eff = r·(sin θ₂ − sin θ₁) summed over the engaged arcs.
The sines of the crossing angles are quadratic numbers, so MRR per move is exact up to F and a_p.

**(6.5) Links proven inside the cleared region.** "The swept disc of the link segment lies in
Cl ∪ (above-floor clearance)" is an exact region inclusion. It uses the boolean that already has an
L6 fuzz and the region spec. It strengthens G3 for stay-down links: today G3 checks rapids against
the stock, and this proves φ = 0 along the link, so no unintended cut happens.

**(6.6) Minimum radius as a theorem.** For the corner-rounded path, every arc has ρ ≥ ρ_min, and
every line-to-line junction is tangent (G1). Both are syntactic checks of the exact path.
Abrahamsen & Thorup's largest bounded-convex-curvature subset [R, abstract] is the right
*construction* for the centre region:
- replace C_r by its largest subset of convex curvature ≤ 1/ρ_min;
- its boundary is then a valid centre path with min radius ρ_min on the convex side;
- the leftover is exactly C_r minus that subset, which goes to rest machining.

Their relation to the morphological opening C_r ∘ B_ρ: not read, **UNKNOWN**. The opening is easy
in forgeCAD: erode by ρ, then dilate by ρ, both exact offsets.

**(6.7) What stays empirical.** Forces, tool life, chatter, the controller's real cycle time, and
the "right" φ_max for a tool and material. Those are data (literature.md §8). The reply should label
them, as G5 labels the feeds.

---

## 7. Recommendations for forgeCAD, in order

Each item is checkable with the metrics in §5. Items 1–6 need no adaptive clearing and are outside
the patent gate as far as patents.md goes. Counsel still decides.

1. **Engagement and quality report first** (§5, §6.3). Compute φ(s), a_eff, h_max, air-cut %,
   retracts, min ρ and a planner-based cycle time per op, in the reply. Without these, "good" cannot
   be measured. It is also the evidence base for Fusion comparisons on the same parts.
2. **Stay-down links** between consecutive loops and levels, proven inside the cleared region
   (§6.5). Use minimum retraction otherwise, G0 for the retract, and an optional microlift. This
   removes most of today's 2 × loops × levels vertical moves.
3. **Corner rounding with a user ρ_min**, concentric where possible, with the engagement *checked*,
   not derived (§1.6, §6.2). Leftovers go to rest or to a finishing contour. Expect corner spikes to
   fall from 127° to about 52° at 10 % step-over with ρ = r. Avoid the chained, engagement-derived
   radius rule (ESPRIT).
4. **Per-tool entry limits** (`ramp_max_deg`, helix diameter range, centre cutting) in
   forge-tooling, with the checker refusing violations. Add predrill and outside entry.
5. **Ordering:** a greedy nearest-neighbour order with deterministic 2-opt and Or-opt over regions
   and levels, honouring the containment order. Use exact or squared-length comparisons so native
   and wasm stay bit-identical. Do not use LKH or GLKH code (non-commercial licence).
6. **Stock-aware levels:** intersect with the exact in-process stock section; add exact flat-face
   levels; fine step-down to fixed heights (check SolidCAM 9,690,282 first).
7. **Behind the patent gate, for when counsel clears it:** a FreeCAD-style controller on the exact
   cleared region, with the target stated as a bound on **φ** (not area), and the output certified
   by (6.3). The Truemill 2004 engagement-milling family expires 2026-12-17 (US 7,577,490), which
   removes one blocker. SolidCAM, Celeritive, Truemill 8,560,113 and DP Tech remain.
8. **Lean targets**, small and high value: (6.1) the coverage theorem, (6.2) the corner formula
   with its negative corollary, and (1.2) the a_eff identity.

---

## 8. Unreachable, unread and corrected

- **Unreachable:**
  - freesteel.co.uk/wpblog (403);
  - web.archive.org (blocked for this tool);
  - mmsonline.com (403);
  - wiki.freecad.org and hal.archives-ouvertes.fr (Anubis bot walls);
  - core.ac.uk PDFs (returned HTML).
  - The Ibaraki 2004 and Uddin 2005 PDFs and the MATHSM preprint were read.
- **Not read (abstract or record only):** Stori & Wright 2000, Choy & Chan 2002/2003, Bieterman &
  Sandstrom 2003, Pateloup et al. 2004/2010, Zhao et al. 2007, Rauch et al. 2009, Otkur & Lazoglu
  2007, Castelino et al. 2003, Monreal & Rodriguez 2003, Helsgaun 2015, Banerjee et al. 2012, and
  Held & Spielberger 2009/2014 (project page read, papers not).
- **UNKNOWN:** "Kim/Hatna" and "Huang" medial-axis papers; "Voronoi adaptive" as a named method
  (nothing found beyond Celeritive's patent and Ibaraki's medial-axis trochoids); Fusion's internal
  algorithm; the Freesteel dates.
- **Corrected from the brief:**
  - Stori & Wright is *J. Manuf. Syst.*, not *J. Manuf. Sci. Eng.*.
  - Choy & Chan is corner *looping*, in *CAD* 35(2).
  - The FreeCAD Adaptive author in the file header is Kresimir Tusek.
  - The FreeCAD search is a safeguarded regula falsi on the area error, not a pure bisection.
  - Ibaraki's constant-engagement work starts in 2004, not the 2010s; the 2010 paper is trochoidal
    grooving.

---

## References

1. Abrahamsen, M. (2015). Spiral toolpaths for high-speed machining of 2D pockets with or without
   islands. arXiv:1412.5034. https://arxiv.org/abs/1412.5034 [R via literature.md]
2. Abrahamsen, M., Thorup, M. (2016/2022). How to cut corners and get bounded convex curvature.
   SoCG 2016; *Discrete Comput. Geom.*; arXiv:1603.02080. https://arxiv.org/abs/1603.02080 [R abstract]
3. Arkin, E. M., Bender, M. A., Demaine, E. D., Fekete, S. P., Mitchell, J. S. B., Sethia, S.
   (2003/2005). Optimal covering tours with turn costs. arXiv:cs/0309014; *SIAM J. Comput.*
   https://arxiv.org/abs/cs/0309014 [R abstract]
4. Autodesk. Fusion 3D Adaptive Roughing reference.
   https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/GUID09E44604-DAD8-47D6-ADC6-C100869DE724.htm [R];
   2D Adaptive Roughing overview https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/2D-ADAPTIVE-READ.htm [R]
5. Autodesk. FeatureCAM help, Vortex.
   https://help.autodesk.com/cloudhelp/2024/ENU/FCAM/files/GUID-6122477C-AF88-4C92-8664-C0542AE7747D.htm [R]
6. Autodesk (2012). Autodesk acquires HSMWorks technology (press release).
   https://investors.autodesk.com/node/16696 [S]; HSMWorks 2014/2015 notes https://adsknews.autodesk.com/en/?p=707 [S]
7. Banerjee, A., Feng, H.-Y., Bordatchev, E. V. (2012). Process planning for floor machining of 2½D
   pockets based on a morphed spiral tool path pattern. *Comput. Ind. Eng.* 63(4):971–979.
   doi:10.1016/j.cie.2012.06.008 [S]
8. Bieterman, M. B., Sandstrom, D. R. (2003). A curvilinear tool-path method for pocket machining.
   *J. Manuf. Sci. Eng.* 125(4):709–715. doi:10.1115/1.1596579 [S; DOI K]
9. Castelino, K., D'Souza, R., Wright, P. K. (2003). Toolpath optimization for minimizing airtime
   during machining. *J. Manuf. Syst.* 22(3):173–180. [S]
10. Choy, H. S., Chan, K. W. (2003). A corner-looping based tool path for pocket milling. *CAD*
    35(2):155–166. doi:10.1016/S0010-4485(02)00049-0 [S]; (2002) Machining tactics for interior
    corners of pockets, *IJAMT* [S]; (2003) Modeling cutter swept angle at cornering cut, *Int. J.
    CAD/CAM* 3(1):1–12 https://koreascience.kr/article/JAKO200336037347979.page [S]
11. Elber, G., Cohen, E., Drake, S. (2005). MATHSM: medial axis transform toward high speed
    machining of pockets. *CAD* 37(2):241–250. doi:10.1016/j.cad.2004.05.008; preprint
    https://gershon.cs.technion.ac.il/papers/hsm.pdf [R]
12. ESPRIT (Hexagon). High-speed machining / ProfitMilling.
    https://espritcam.hexagon.com/product/high-speed-machining [R]
13. FreeCAD. `src/Mod/CAM/libarea/Adaptive.cpp` and `Adaptive.hpp` (K. Tusek, 2018,
    LGPL-2.1-or-later). https://raw.githubusercontent.com/FreeCAD/FreeCAD/main/src/Mod/CAM/libarea/Adaptive.cpp [R];
    CAM Adaptive wiki mirror https://reqrefusion.github.io/FreeCAD-Documentation-html/wiki/CAM_Adaptive.html [R]
14. GibbsCAM / CAMBRIO (2020). VoluMill manual, version 14. https://bu.edu/epic/files/2023/01/24_VoluMill.pdf [R]
15. Held, M. Project page: high-speed machining (spiral paths).
    https://cosy.sbg.ac.at/~held/projects/hsm/hsm.html [R]; Held & Spielberger (2009) *CAD*
    41(7):539–550, doi:10.1016/j.cad.2009.04.002 [S]; Held & Spielberger (2014) *CAD&A*
    11(3):346–357, doi:10.1080/16864360.2014.863508 [S]; Held & de Lorenzo (2018) *JCDE*
    5(3):348–357, doi:10.1016/j.jcde.2017.11.011 [S]
16. Helsgaun, K. (2015). Solving the equality generalized traveling salesman problem using the
    Lin–Kernighan–Helsgaun algorithm. *Math. Prog. Comp.* 7(3):269–287.
    doi:10.1007/s12532-015-0080-8 [S]
17. Ibaraki, S., Ikeda, D., Yamaji, I., Matsubara, A., Kakino, Y., Nishida, S. (2004). Constant
    engagement tool path generation for two-dimensional end milling. Proc. JUSFA 2004, Denver.
    https://mecdes.hiroshima-u.ac.jp/~ibaraki.h0523/pubs/papers/jusfa04_ibaraki.pdf [R]
18. Ibaraki, S., Yamaji, I., Matsubara, A. (2010). On the removal of critical cutting regions by
    trochoidal grooving. *Prec. Eng.* 34(3):467–473. [S, via literature.md]
19. Inui, D., Nakatsuji, H., Nishida, I. (2026). Toolpath generation based on mathematical
    algorithms with shape simulation for mold machining. *IJAT* 20(4):369–381.
    doi:10.20965/ijat.2026.p0369 https://www.fujipress.jp/ijat/au/ijate002000040369/ [R]
20. ISCAR. Die and mold user guide (helical milling rule).
    https://www.iscar.com/Catalogs/flipview/reference-user-guides/Die_and_Molds_User_Guide_7861458_2017/files/basic-html/page15.html [S]
21. Kramer, T. R. (1992). Pocket milling with tool engagement detection. *J. Manuf. Syst.*
    11(2):114–123. https://www.nist.gov/node/703756 [S]
22. libactp-old (Heeks). https://github.com/Heeks/libactp-old [R, About text]
23. LinuxCNC documentation, user concepts (trajectory control).
    https://linuxcnc.org/docs/html/user/user-concepts.html [R]
24. Mastercam. How Dynamic Motion optimizes tool engagement.
    https://www.mastercam.com/community/blog/how-dynamic-motion-optimizes-tool-engagement-automatically/ [R];
    micro lift and back feed from tutorials https://blog.caminstructor.com/mastercam-dynamic-milling-part-3 [S]
25. Monreal, M., Rodriguez, C. A. (2003). Influence of tool path strategy on the cycle time of
    high-speed milling. *CAD* 35(4):395–401. doi:10.1016/S0010-4485(02)00060-X [S]
26. Otkur, M., Lazoglu, I. (2007). Trochoidal milling. *IJMTM* 47(9). [S]
27. Park, S. C., Choi, B. K. (2001). Uncut free pocketing tool-paths generation using pair-wise
    offset algorithm. *CAD* 33(10):739–746. [S, via literature.md]
28. Pateloup, V., Duc, E., Ray, P. (2004). Corner optimization for pocket machining. *IJMTM*
    44(12–13):1343–1353 [S]; (2010) B-spline approximation of circle arc and straight line for
    pocket machining, *CAD* 42(9):817–827 [S]
29. Patel, D., Lalwani, D. I. Pocket tool path papers (Inderscience). https://www.inderscience.com/filter.php?aid=80530 [S]
30. Practical Machinist forum, "Adaptive clearing" thread.
    https://www.practicalmachinist.com/forum/threads/adaptive-clearing.298696/ [S]
31. Rauch, M., Duc, E., Hascoët, J.-Y. (2009). Improving trochoidal tool paths generation and
    implementation using process constraints modelling. *IJMTM* 49(5):375–383. [S]
32. Sandvik Coromant. Ramping. https://www.sandvik.coromant.com/en-gb/knowledge/milling/milling-holes-cavities-pockets/ramping [S]
33. SolidCAM. iMachining overview. https://solidcam.com/cam-modules/imachining-overview/ [R]
34. Stori, J. A., Wright, P. K. (2000). Constant engagement tool path generation for convex
    geometries. *J. Manuf. Syst.* 19(3):172–184. [S; citation R in ref. 17]
35. Uddin, M. S., Ibaraki, S., Matsubara, A., Nishida, S., Kakino, Y. (2005). Constant engagement
    tool path generation to enhance machining accuracy in end milling. Proc. LEM21, Nagoya.
    https://mecdes.hiroshima-u.ac.jp/~ibaraki.h0523/pubs/papers/lem05_uddin.pdf [R]
36. Zhao, Z. Y., Wang, C. Y., Zhou, H. M., Qin, Z. (2007). Pocketing toolpath optimization for sharp
    corners. *J. Mater. Process. Technol.* 192–193:175–180. [S]
37. Abrahamsen, M. US 10,108,172 B2, Spiral toolpaths for high-speed machining of polygonal pockets.
    https://patents.google.com/patent/US10108172 [S, hit only; for the patent agent]
