# Adaptive clearing and adaptive feed: claim-level FTO notes

**Engineering analysis for David to judge. NOT legal advice and not a freedom-to-operate
opinion.** It was written by an engineer-researcher from public pages. Claim scope is set by the
full claims as a court construes them, read with the specification and prosecution history,
none of which was reviewed here. Expiry depends on PTA, terminal disclaimers, the CIP or
continuation chain, and fees. Google Patents labels its own status and expiry fields "an
assumption, not a legal conclusion". Everything below has to go to patent counsel before any
release decision.

Read on **2026-11-24**. Marks: **[R]** means the Google Patents page (claims section, legal events
and family tables) was fetched and read directly this session. **[S]** means search snippet only.
**[K]** means background knowledge that was not re-read. "Expiry" is Google's adjusted or
anticipated expiration as shown on 2026-11-24. "Fees" are the USPTO maintenance-fee events
Google lists. Google showed no terminal-disclaimer event for any patent below, but Google does
not reliably list terminal disclaimers, so that point is UNKNOWN for all of them. All patent
URLs have the form `https://patents.google.com/patent/<number>/en`.

Companion survey: `patents-fto.md` (this folder). Earlier table: `../patents.md` (2026-11-23).

---

## 0. Findings in one page

1. **Free now (expired, prior art).** The Surfware/Truemill 2004 "engagement milling" family
   except US 7,577,490. US 7,451,013 (store a max engagement, shape passes so engagement is
   controlled) expired 2025-05-23. US 7,831,332 and US 8,000,834 expired 2025-03-02. US 8,694,149
   (circular passes into a three-sided region with engagement capped) lapsed for an unpaid 12th-year
   fee (FP effective 2026-04-08) [R]. Also free: MTU US 7,237,989 (2003 priority; curvature of each
   path set so cutter contact holds at the maximum allowed), expired 2024-06-11 [R]. GE feedrate
   patents US 4,833,617 (lapsed 1993) and US 5,828,574 (expired 2017) [R]. Surfware US 6,704,611,
   expired 2022 [R].
2. **US 7,577,490 (Truemill).** Its claims are narrow: passes of exactly two tangent arcs with a
   radius rule, advancing into a three-sided region (§4). It expires on 2026-12-17 per Google, and
   it may already have expired on about 2026-10-27, because it is a CIP of an application filed
   2005-03-02 (§4) [R + arithmetic]. All fees are paid. It blocks only that two-arc construction.
3. **Live, and closest to "adaptive clearing", ordered by breadth:**
   - **SolidCAM US 8,489,224 cl.1 and US 9,052,704 cl.1 (to 2031).** Preselect a maximum and a
     minimum engagement angle; the path's engagement varies gradually between them.
   - **SolidCAM US 9,823,645 / 10,895,861 / 10,324,445 / 10,620,607 (2031).** Asymmetric spiral
     loops whose engagement swings max to min within each loop. Also the "moat around an island"
     claims (8,489,224 cl.45; 9,052,704 cl.4) and the trochoid-vs-spiral time choice
     (12,248,293). The pending **US 2025/0284264** claims choosing a clearance-plane link or a
     stay-down link by estimated time.
   - **Delcam/Autodesk US 9,489,339 (to 2035-03-07).** Offset subsets plus backfill, or enlarging
     the corner radius of the *preceding* offset when the predicted engagement angle exceeds a
     limit.
   - **Boeing US 8,676,372 (to 2032-06-11).** Engagement level chosen from an axial-DOC vs
     radial-DOC "engagement plot", with the path generated from it.
   - **MERL US 9,892,215 (to 2035-04-16).** Split the path into segments of near-constant
     engagement; feed per segment from an engagement function that includes a material
     "abrasive resistance" parameter. This is the live **adaptive-feed** claim.
   - **Celeritive** VoluMill links (8,295,972 / 8,880,212, to 2029-2030) and modified-Voronoi or
     arc-replacement (9,946,245 / 10,108,176 / 10,579,040, to 2032-2033).
   - **Truemill US 8,560,113** corner arcs (to 2031).
   - **DP/Hexagon profit milling** (9,778,649 / 9,958,859 / 10,488,847, to 2033-2034).
   - **Autodesk US 10,108,172 (Abrahamsen, to 2036-12-18).** Spiral from a specially modified
     Voronoi diagram plus a "wave" model. All 22 claims read: both independents need a hole in
     the pocket; hole-less pockets only via a skeleton as a zero-area hole (dependent cl.10/21).
4. **Recommended design (§3, matrix §3.2).** (A) Step-by-step clearing driven by a cleared-area
   simulation, holding **one** target (removed area per step, upper-bounded). There is no
   min/max engagement band, no spiral loops built to a cyclic engagement pattern, and no
   reshaping of earlier passes. (B) Links: retract, or a straight move checked against the
   cleared area, chosen by a fixed rule. No transition toolpaths or areas, and no time-estimate
   comparison. (C) Feed from engagement per fixed-length sample, using chip-thinning geometry and
   the user's chipload. No material-abrasion term, and no partitioning by engagement variation.
   (D) Islands handled by the same simulation, with no dedicated moat pass. On my reading this
   design avoids at least one required element of every live independent claim listed. The weak
   spots are the SolidCAM machine/product claims (9,823,645 / 10,895,861 / 10,324,445). They need
   no *preselected* band, only engagement that rises and falls within spiral loops. Counsel must
   rule on those.

---

## 1. Live independent claims reduced to required elements [R unless marked]

Notation: `[P#.n]` is element n of patent P's claim. Medium and apparatus claims that repeat a
method claim's elements are merged. Plain-words paraphrase, not claim text.

### 1.1 SolidCAM "Computerized tool path generation" (iMachining), priority 2011-02-28

Inventors Berman, Osovlanski, C. Calderone, A. Calderone. All fees due to date have been paid,
and no lapse appears [R].

| Patent | Expiry | Independents read |
|---|---|---|
| US 8,489,224 B2 | 2031-08-20 | 1, 5, 13, 20, 30, 36, 45, 51, 57, 63, 69, 75 (80 claims) |
| US 9,052,704 B2 (div.) | 2031-02-28 (anticipated) | 1-6 |
| US 9,690,282 B2 (CIP) | 2033-01-04 | 1, 4, 7, 10, 11, 12 |
| US 9,823,645 B2 | 2031-10-22 | 1 |
| US 10,324,445 B2 | 2031-03-22 | 1-4 read; 5, 7 product-by-process versions of the channel claims |
| US 10,620,607 B2 | 2031-05-26 | 1, 4, 7 |
| US 10,895,861 B2 (pub. US 2019/0243331) | 2031-05-23 | 1 |
| US 11,625,019 B2 | 2031-05-02 | 1, 5 |
| US 12,248,293 B2 | 2031-05-07 | 1, 5 |
| US 2025/0284264 A1 | pending (filed 2025-02-12; "docketed new case" 2025-06-05) | published 38, 74 |

- **'224 cl.1 (and cl.5, apparatus):** (1) select a maximum permitted engagement angle; (2)
  select a minimum permitted engagement angle; (3) configure the path so the engagement angle
  varies gradually between them. Cl.5 adds: (4) an asymmetric spiral path; (5) the variation is
  cyclic.
- **'704 cl.1 (method of machining):** (1) direct the tool along a path whose engagement angle
  varies gradually between (2) a *preselected* max and (3) a *preselected* min.
- **'224 cl.13/20:** (1) select a region; (2) an asymmetric spiral with at least two loops; (3)
  each loop's stepover increases and decreases cyclically; (4) the spiral maximizes the portion
  of the region it removes.
- **'224 cl.30/36; '704 cl.2-3; '445 cl.2-3:** (1) an asymmetric (diverging) spiral removes a
  first portion; (2) trochoid-like paths remove only the rest; (3) the split is chosen to minimize
  machining time ('704 cl.2 has (3') "spiral maximizes the portion removed" in place of time).
- **'224 cl.45/51; '704 cl.4; '445 cl.4 (islands):** (1) define islands in the cross-section; (2)
  start the path in a region without islands; (3) when the path meets an island, run a "moat"
  path that surrounds the island.
- **'224 cl.57/63; '704 cl.5; '445 cl.5:** (1) find an open region where clearing it whole takes
  longer than cutting a separating channel and clearing two parts; (2) define or cut that channel
  between two points on the external boundary ('704: with a trochoid-like path).
- **'224 cl.69/75; '704 cl.6; '445 cl.7:** (1) a semi-open region where a trochoidal clear takes
  longer than isolating it with channels; (2) define separating channels to all closed boundary
  segments.
- **'645 cl.1 (machine); '861 cl.1 (machine); '445 cl.1 (object):** (1) a controller or machine
  directs the tool along an asymmetric (in '645/'445 also *non-circular*) spiral; (2) sequential
  loops separated by different distances at different places; (3) within each loop the engagement
  falls gradually from a max to a min and rises again ('645/'445: at least twice per loop). No
  "preselected" values are required.
- **'607 cl.1/4/7:** (1) get the available spindle power; (2) automatically pick a max depth and
  width of cut as a function of that power; (3) layers at that depth and width; (4) an asymmetric
  spiral with loops at varying distances whose engagement swings between a max and a min.
- **'282 cl.1 (and 4, 7, 10-12; 3D rest roughing):** (1) define the finished surface, (2) an offset
  surface (inner limit) and (3) a scallop surface (outer limit); (4) step-up cuts at several Z
  heights; (5) all resulting surfaces lie between the two limits; (6) the number and areas of
  step-ups are chosen to minimize material removed; (7) step heights computed at azimuthal points
  along the step-wall/floor intersection curve (the claim continues with further specifics).
- **'019 cl.1/5 (thin walls):** (1) rough to a thick wall from both sides at first max values of
  depth, width, speed and feed; (2) then thin it from both sides at reduced second values; (3) upper
  portion first, then lower (further specifics follow).
- **'293 cl.1/5:** (1) estimate the time to machine a semi-open region with a trochoidal path; (2)
  estimate the time with a spiral path; (3) automatically pick the shorter; (4) machine with it.
- **'264 (pending) cl.38/74:** (1) multiple toolpaths that need repositioning between them; (2)
  estimate the time of a repositioning path that travels in a clearance plane; (3) estimate the
  time of one that does not; (4) automatically pick the shorter. *Pending: claims may change.*

### 1.2 Celeritive Technologies (VoluMill)

| Patent | Priority | Expiry | Fees |
|---|---|---|---|
| US 8,295,972 B2 | 2008-10-07 | 2030-06-23 | 4/8/12 paid |
| US 8,880,212 B2 (cont.) | 2008-10-07 | 2029-10-07 | 4/8/12 paid (12th 2026-04-28) |
| US 9,946,245 B2 | 2011-07-25 | 2033-09-28 | 4/8 paid |
| US 10,108,176 B2 (div.) | 2011-07-25 | 2033-07-05 | 4/8 paid |
| US 10,579,040 B2 (cont.) | 2011-07-25 | 2032-07-25 | 4 paid |

- **'972 cl.1/17 (cl.15 adds a two-pass order):** (1) receive a main toolpath of cuts; (2)
  compute transition toolpaths, each defining a transition area that intersects cuts; (3) trim
  the cuts at their intersections with transition areas; (4) join the trimmed pieces with
  connecting moves that lie substantially inside transition areas, where the tool is
  substantially disengaged; (5) mill along all of it.
- **'212 cl.1/10/22/26:** elements (1), (2) and (4) as above, without trimming (cl.1: connect the
  end of one cut to the start of the next; cl.10: connect an interruption of a cut to its
  continuation); plus (5) the milling removes material from part of a transition area of a
  computed transition toolpath.
- **'245 cl.1/10/11:** (1) compute a Voronoi diagram of the region; (2) add branches that are
  distinct from any VD branch, creating new faces and boundary elements distinct from the region
  boundary; (3) identify the modified-VD faces; (4) traverse them to create the path.
- **'040 cl.1/8:** (1) a Voronoi diagram; (2) add branches, giving boundary elements distinct
  from the region boundary; (3) path based on faces of the modified VD. **cl.15:** (1) a first VD;
  (2) *shorten* a branch to get a tangent-continuous representation of a sub-region; (3) path from
  faces of a *second* VD of that sub-region.
- **'176 cl.1/5/9 (broad):** (1) receive chains of curves forming a *previously generated*
  toolpath; (2) replace at least one circular arc in it with a sequence of curves that contains a
  new arc *closer to the workpiece boundary*; (3) mill with the revised path.

### 1.3 Truemill US 8,560,113 B2: "Method of milling an interior region"

Priority 2010-04-13; expiry 2031-09-03. The 12th-year fee was paid late with surcharge on
2025-10-03, so the patent is in force [R].
- **cl.1/8/15:** (1) a region with first, second and third sides; (2) a first arc whose
  in-process boundary intersects the first side and is tangent to the second and third sides;
  (3) a succession of second arcs, each intersecting the first side, parallel to the first arc,
  stepping by a set value toward the first side until the remainder is less than that value; (4)
  then a succession of *transition passes connecting each second arc, end to end, with the first
  arc*.

### 1.4 DP Technology / Hexagon "Profit milling" (inventor Jianping Song), priority 2012-06-01

| Patent | Expiry | Fees |
|---|---|---|
| US 9,778,649 B2 | 2034-02-03 | 4 (small), 8 (large, 2024-10-31) |
| US 9,958,859 B2 (filed 2014-11-25; new to our list) | 2034-10-13 | 4, 8 |
| US 10,488,847 B2 (cont. of '649) | 2033-09-15 | 4 paid 2023-01-31 |

Current assignees per Google: Hexagon Innovation Hub GmbH / Hexagon Manufacturing Intelligence.
- **'649 cl.1:** (1) a blend-arc radius from the maximum allowed variation of tool engagement angle
  (TEA); (2) smooth offsets using that radius, or the prior radius plus stepover; (3) limit TEA by
  limiting maximum curvature of the smoothed offset; (4) determine a pocket centre area sized from
  stepover and tool radius; (5) re-create moves inside the centre area with TEA kept in a range; (6)
  path keeps MRR at or below a value at each point; (7) feed change and TEA change kept in ranges.
  **cl.9:** contour-parallel offsets (boundary or VD-based); (1) a "preferred smoothing radius" used
  as blend radius *and* as the inward offset distance; (2) replace narrow areas with blend arcs;
  (3) centre-area re-creation; (4) an iterative smoothing loop; (5) link offsets.
- **'859 cl.1/9:** offsets grouped; smooth groups with the PSR or a minimal smoothing radius,
  using the adjacent inner group as guide; inward offsets at the PSR distance; iterate until no
  offset can be smoothed.
- **'847 cl.1/8/15:** (1) smooth offsets with the blend radius; (2) "trochoidal areas" from
  narrow areas formed by blend arcs and partial offsets; (3) group open and closed offsets
  separately; (4) sort by adjacency; (5) connect sorted offsets by trochoidal moves inside the
  trochoidal areas.

### 1.5 Delcam, now Autodesk: US 9,489,339 B2, "material remover cutting paths" (Mackman)

Priority 2011-12-12; expiry 2035-03-07; 4th and 8th fees paid; assignee now Autodesk Inc [R].
Believed to be Delcam's Vortex [K, not confirmed by the page].
- **cl.1/10:** (1) offsets from the model perimeter at the stepover, per corner region; (2) per
  offset, a *subset* of paths at the stepover, where the first path of a subset has a *larger*
  stepover to the last path of the previous subset; (3) join paths within a subset at each corner
  region; (4) *backfill* paths remove the material left in that larger-stepover zone.
- **cl.18/26:** (1) subsets of offset paths as in (2) above; (2) assess curved portions of the
  current path for a predicted engagement angle above a set angle; (3) if above, *increase the
  radius of the corresponding curve of the preceding path* so the current path stays under it.

### 1.6 Autodesk US 10,108,172 B2: "Spiral toolpaths for high-speed machining of polygonal pockets"

Inventor Mikkel Abrahamsen. Assignee Autodesk Inc (the coordinator's note had Abrahamsen as
holder; the page names him as inventor and Autodesk as assignee). Priority 2014-10-13; expiry
**2036-12-18**; 4th and 8th fees paid (8th on 2026-03-19) [R].
- **cl.1/12:** (1) a polygon P *with a polygonal hole H*; (2) a VD of P's and H's segments; (3)
  modify it so that it has exactly one cycle (the locus equidistant from H and P), trees growing
  from the cycle, a leaf on every corner of P and H, and all faces convex; (4) for each tree a
  "wave" model running from H's leaves (t=0) to P's leaves (t=1); (5) a polyline spiral made by
  travelling around the wave, going around H; (6) mill the pocket along it.
  Cl.12 (medium) repeats cl.1 step for step ("leaf on the boundary of P∖H" in place of "of P or
  H"; no change in substance).
- **All 22 claims read [R].** The only independents are **cl.1 (method) and cl.12 (medium)**;
  cl.2-11 depend on 1 and cl.13-22 mirror them on 12. Every claim therefore needs elements
  (1)-(6) above: a hole H, the modified VD with its single cycle and convex faces, the wave
  model, and a spiral around H. Dependents add: perpendicular edges spaced along the boundary
  (cl.2/13); a bisector replacing double edges at concave corners (cl.3/14); wave times and
  speeds node by node, smoothed around the cycle (cl.4-6/15-17); revolutions set from the user
  stepover, interpolated between wavefronts and rounded to lines and arcs (cl.7-9/18-20).
- **Pockets without a hole.** No independent claim covers them. Dependents **cl.10/21** reach
  them only by *building a skeleton inside P and treating it as a zero-area hole H*; cl.11/22
  join several holes by bridges into one hole. So a hole-less pocket reads only if the method
  still builds the modified VD and wave around such a skeleton.
- **Spirals not built from a modified VD.** No claim covers them: the modified VD (3) and the
  wave model over its trees (4) are in both independents.
- **forgeCAD (§3).** A builds no VD and no spiral, so it avoids (2)-(5). B's plain offsets avoid
  (2)-(5); B's circles along an *unmodified* medial axis avoid (3) (no added edges, no single
  cycle forced, no convex faces required) and (4) (no wave model). Design rule: do not turn a
  medial-axis skeleton into a zero-area "hole" and spiral outward from it with timed
  wavefronts; that is cl.10/21.

### 1.7 Boeing US 8,676,372 B1: "Tool path generation for machining operations"

Priority 2011-07-19; expiry 2032-06-11; 4/8/12 fees paid (12th 2025-09-18) [R].
- **cl.1/19:** (1) identify a desired level of engagement *using an engagement plot of axial
  depth of cut versus radial depth of cut*; (2) generate a path whose levels of engagement along
  it are based on that desired level.
- **cl.17/28:** same plot element; select a path and machine with levels of engagement based on it.
- **cl.14/33:** adds a stability plot at a chosen spindle speed for a particular machine, a
  material from a list, feed selection, and machining an aircraft part.

### 1.8 MERL US 9,892,215 B2: "Determining feedrates of machining tools" (adaptive feed)

Priority 2013-09-23; expiry 2035-04-16; 4th and 8th fees paid (8th 2025-07-30) [R].
- **cl.1:** (1) partition the path into segments within which an engagement function is
  substantially constant, with the partition based on variation of that function staying within
  a set range; (2) the engagement function includes the engagement angle (entry to exit) and
  geometric properties of the engagement surface, **and a parameter for the workpiece material's
  abrasive resistance**; (3) a feed per segment from the function; (4) machine with those feeds.
- **cl.11:** the same, inside a simulation built on distance fields: an object distance field,
  a tool distance field and a swept-volume distance field.
- **cl.13:** the same, with a composite adaptively sampled distance field and ray casting through
  swept volumes.

### 1.9 Other live items checked; low relevance to 3-axis adaptive roughing

- **CNC Software (Mastercam) US 12,663,778 B1**: CAM pipeline as a graph of numbered nodes. All
  three independent claims are now read; see §1.10.
- **OPEN MIND US 12,485,511 B2** (assignee confirmed: Open Mind Technologies AG), priority
  2021-08-18, expiry 2044-06-20 [R]. **cl.1/16/17:** a multi-axis path with two engagement
  segments and a collision-free non-engaging link; (1) an "interpolation group" of 1 to N−1 of
  the N machine axes; (2) per-axis interpolation start and end points at the segment ends; (3) a
  per-axis link function by interpolation; (4) define the link from those functions; (5) machine.
  Relevant to multi-axis linking. A 3-axis link that interpolates XY and Z separately *might* be
  argued to fit; that is UNKNOWN.
- **Celeritive US 10,987,774 B2** (5-axis tool-orientation domain mapping, to 2033-10-11);
  **US 2023/0040385 A1** (turning notch, Google status "Active", expiry 2044). Both are out of
  scope [R].
- **Autodesk US 11,176,291** (choosing roughing tool sequences, to 2040-04-04), **US
  11,782,396 / 12,487,565** (reinforcement-learning toolpaths, to 2041), **US 10,884,392**
  (deflection-based stock, to 2038-10-20). These matter only if forgeCAD adds automatic tool
  selection, ML toolpaths or deflection simulation [R, cl.1 only].
- **Siemens AG US 11,567,470** (force model from tool-workpiece interaction; *corrects the
  toolpath by machine stiffness*, to 2039-12-19) [R]. Avoid toolpath correction from stiffness.
- **SolidCAM US 11,048,224 / 11,841,693** (chatter map from historical data, to 2037) [R/S].
- **MERL US 9,575,486** (oscillator pocket path) lapsed for fees on 2021-02-21 [R], so it is free.
- **ModuleWorks**: Google Patents assignee search returned 0 US results [R, query]. Siemens
  Industry Software: no adaptive-clearing patent among the 69 "milling" hits [R, list only].
  CGTech: only a fibre-placement patent [R, list].

### 1.10 CNC Software (Mastercam) US 12,663,778 B1: "Systems and methods for generating a tool path using computer aided manufacturing for machining"

Published as a B1 (no pre-grant publication; the B2 URL returns 404). Inventors Seth Jackson,
Dylan Gondyke, Mike Johnson, Dave Conigliaro, Carlos Carbonera, Dustin Peirolo; assignee CNC Software LLC. Filed and priority 2025-06-27
(US 19/251,917); granted 2026-06-23; Google's anticipated expiration **2045-06-27**; large entity;
no maintenance fee due yet [R]. 14 claims. Independents: **1** (system), **10** (method), **13**
(computer program); 14 is a carrier of 13. Read in full [R].
- **cl.1 (system of nodes):** (1) a *generator node* that makes and sends a "raw pass packet":
  several passes, each a list of points in space for the tool to traverse; (2) *filter nodes*
  that receive the packet, **modify** it and pass it on, of two kinds, both required: (2a) a
  *de-gouging node* that, for every point, makes the tool hit nothing but the stock and never go
  past the part's surface; (2b) a *linking node* that adds the transition from each pass's end to
  the next pass's start; (3) a *processing node* that turns the packet into machine instructions;
  (4) the nodes are arranged as a **directed acyclic graph**, each node has an **ordering
  number**, and a node may only send to higher-numbered nodes and only receive from
  lower-numbered ones. (Dependents add per-node configuration data, a tool-axis-control node,
  super nodes, wait-for-predecessors, a logic node.)
- **cl.10 (method) and cl.13 (program; cl.14 its carrier):** (1) obtain a model of the part; (2)
  generate a raw pass packet (passes of points); (3) add tool-axis-control data, *for every point*
  a surface normal, a tip position and a tool orientation, giving "Offset Cutter Location Data";
  (4) de-gouge that data (same two conditions as cl.1 (2a)), giving "Safe" data; (5) then add the
  linking between passes, giving the tool path. No node graph is required. **Cl.10 does not
  require machining** (only dependent cl.11 does), so a program that only generates a path could
  practice cl.10 directly; see §5 Q10.
- **forgeCAD's CAM pipeline, as the code stands [R, code read]:** `interface/forge-cam-lang`
  (`run`) evaluates the machining document statement by statement in source order and keeps the
  ops in a plain `Vec<OpOut>` in that order. Each op is one pure function in
  `kernel/forge-cam/src/ops.rs` (`facing`, `contour`, `pocket`, `holes`) that returns an
  `OpPath` whose moves **already include the retract links** (`Phase::Link`); there is no
  separate linking stage. The program is then posted, the G-code read back (round trip), and the
  read-back motion is checked by `certify_fixtured` (G1 gouge, G2 shank and holder, G3 rapids).
  The checker **never moves a point**: an op that is not clear is refused with the check, place
  and a fix. Moves (`PMove`) hold only an XYZ end point (and an arc centre); no per-point normal
  or tool orientation is stored. There are no node objects, no graph structure and no ordering
  numbers. So: **no DAG with ordering numbers**, no de-gouging filter that modifies the path, no
  stand-alone linking filter, no Offset Cutter Location Data.
- **Elements forgeCAD avoids.** cl.1: (2a) (check-and-refuse, not modify), (2b) (links made
  inside the generator, before any check), and on a plain reading (4). Weak point: the document's
  statements run in source order and later statements use names defined earlier, so someone
  could argue that statement order is an "ordering number" and that a linear chain is a trivial
  DAG (Q8). Even so, (2a) is absent. cl.10/13: (3) (no per-point normal and orientation) and (4)
  (no de-gouging step that yields corrected data); the order also differs (links before checking,
  checking after posting). **Design rules:** keep gouge handling as check-and-refuse; if an
  automatic gouge *fix* is ever added, do not run it as a stage that rewrites points ahead of a
  separate linking stage; do not store a per-point normal + tip + orientation record for 3-axis
  output; do not build the CAM pipeline as a user-visible graph of numbered generator / filter /
  processor nodes.

---

## 2. Prior art predating the live families

| Item | Date | What it shows | Mark |
|---|---|---|---|
| M. Held, *On the Computational Geometry of Pocket Machining*, LNCS 500, Springer | 1991 | offset and Voronoi-based pocketing | [K] |
| GE US 4,833,617 (Wang): solid-model adaptive feedrate | filed 1987-08-14; lapsed 1993 | offline simulation, per-move MRR, force, deflection, adjusted feed | [R] |
| GE US 5,828,574 (Robinson, Wang, Nelson) | filed 1997-05-19; expired 2017 | same, from wire-EDM stock | [R] |
| Stori & Wright, "Constant engagement tool path generation for convex geometries", *J. Manufacturing Systems* 19(3):172-184, DOI 10.1016/S0278-6125(00)80010-2 | 2000 | spiral-in path holding engagement near constant; controls curvature and entry/exit where it can't | [S] |
| Surfware US 6,704,611 "System and method for rough milling" | priority 2001-08-21; expired 2022-08-21 | rough milling (claims not parsed) | [R metadata] |
| Bieterman & Sandstrom, "A curvilinear tool-path method for pocket machining", *JMSE* 125(4):709-715 | 2003 | smooth spiral from centre morphing to boundary (Laplace-based) | [S] |
| Choy & Chan, "Modeling cutter swept angle at cornering cut", *Int. J. CAD/CAM* | 2003 | engagement (swept angle) at corners; path modification to limit it | [S] |
| MTU US 7,237,989 (Glaesser) | priority 2003-06-19; expired 2024-06-11 | after reaching max permissible contact, set each path point's curvature from tool radius, pocket and *previous path* so contact stays at the max | [R] |
| Freesteel / libactp, `CoreRoughGeneration.cpp` | headers "Copyright (C) 2004 Julian Todd and Martin Dunschen", GPL-2.0+; Debian packaging 2009-05-05; last commit 2010-03-27 | step-by-step roughing that "change[s] direction according to the amount of material cutting", with a cleared-contour check | [R] (cloned github.com/Heeks/libactp-old). Public date before 2009 UNVERIFIED |
| Surfware publications US 2005/0246052 A1, US 2005/0256604 A1 (2004 family) | published 2005 | engagement-capped passes, concentric/isoloop spirals, transition arcs | [R family list] |
| Elber, Cohen, Drake, "MATHSM: Medial Axis Transform toward High Speed Machining of pockets", *CAD* 37(2):241-250 | online 2004, issue 2005 | C1 path of maximal inscribed circles along the medial axis plus bitangent links | [S] |
| Uddin, Ibaraki et al., *JSME Int. J. Series C* 49(1):43-49 | 2006 | constant-engagement modification of offset paths in pocketing | [S] |
| Trochoidal HSM slotting | 1990s | trochoid = circle + advance | [K] |
| FreeCAD `Adaptive.cpp` (Kresimir Tusek), commit 31cbf6f6 "Path adaptive operation added", by kreso-t | 2018-08-19 per coordinator (date not shown on the page I read) | cleared-area polygons; per-step target cut area; angle search | [R commit page; date S] |

Fit against the live families (engineering view):
- **Against SolidCAM (2011):** Stori & Wright 2000, MTU 2003, Surfware 2005 publications, libactp
  (if it was public by 2009-2010), MATHSM 2005, Uddin/Ibaraki 2006. All of them predate it and
  all control or cap engagement. None found so far *selects a minimum* engagement angle and
  varies between the two; that element is SolidCAM's distinguishing feature.
- **Against Celeritive links (2008):** classic retract or stay-down linking [K]. No dated
  "transition area" art was found.
- **Against Delcam '339 cl.18 (2011):** MTU 2003 shapes each path from the *previous* path to hold
  contact. Uddin/Ibaraki 2006 modifies offsets for constant engagement. Both are close.
- **Against Boeing (2011):** stability-lobe diagrams are decades old [K], but the claim pairs them
  with path generation.
- **Against MERL feed (2013):** GE 1987/1997 (per-move feed from simulated removal) [R]. Feed
  scheduling by radial chip thinning is old shop practice [K].
- **Against Autodesk '172 (2014):** MATHSM 2005 and Held 1991 (VD-based pocketing).

---

## 3. Design-around candidates and claim-element matrix

### 3.1 Candidates (concrete algorithm choices)

- **A. Simulation-driven, single-target step clearing (FreeCAD/libactp style).** Keep a
  cleared-area region (polygon or grid). At each step, choose the heading so that the *newly
  removed area per step* is close to a target, *never above an upper bound*. There is no
  user-selected or computed **minimum** engagement and no deliberate cyclic variation; any dip
  is incidental (free space, breakthrough). Passes are not organised into spiral loops with
  designed stepover variation. Earlier passes are never revised; each step is computed once.
- **B. Offset or medial-axis clearing with bounded engagement by limiting, not reshaping.**
  Plain contour-parallel offsets at a uniform stepover, or circles along an *unmodified* medial
  axis (MATHSM-like). Where the simulation says a step would exceed the cap, insert a trochoid
  loop (a user-set circle-plus-advance) or slow the feed. No blend-radius-from-TEA rule, no
  preceding-path radius enlargement, no larger-stepover subsets with backfill, no added or
  shortened Voronoi branches, no wave model.
- **C. Links by rule.** At the end of a cut: if the straight segment to the next start lies
  inside the cleared area (with clearance), stay down and move straight; otherwise retract to a
  safe Z and rapid. This is a fixed rule, with no estimate of the time of each option and no
  transition toolpaths, transition areas or trimming of cuts against them.
- **D. Adaptive feed from engagement.** After the path is fixed, sample it at a fixed arc-length
  step (or per existing move). At each sample, compute the radial engagement from the cleared
  area and set feed = user chipload × flutes × rpm × chip-thinning factor(engagement), clamped.
  The only material input is the user's chipload. There is no "abrasive resistance" term and no
  partitioning of the path by engagement-variation thresholds. This follows the GE 1997 per-move
  pattern.
- **E. Islands and regions.** The same simulation clears around islands as they come; there is
  no dedicated moat pass on first contact with an island. There is no automatic channel-cutting
  decision driven by time estimates, no trochoid-vs-spiral time choice, and no automatic
  depth/width selection from spindle power.

### 3.2 Matrix

The cell gives the candidate's status against the claim and the element it fails to meet.
"avoids [n]" means element n is absent by design. "n/a-avoids" means the candidate does not do
what the claim covers at all, so a core element is absent. "may read" means every element
*could* be present. "unknown" means it was not fully read or the answer depends on construction.

| Live claim (expiry) | A step clearing | B offset/MA | C links | D adaptive feed | E islands/regions | Combined A+C+D+E |
|---|---|---|---|---|---|---|
| SolidCAM '224 cl.1/5; '704 cl.1 (2031) | avoids [2] no min selected; **unknown** if a tolerance band is construed as a min | avoids [2] | n/a-avoids | n/a-avoids (feed, not path) | n/a-avoids | avoids [2], subject to Q1 |
| '224 cl.13/20 (2031) | avoids [3] no designed cyclic stepover; **may read** if loops emerge with varying stepover and "maximizes removed portion" | avoids [2] (uniform-stepover offsets) | n/a-avoids | n/a-avoids | n/a-avoids | **may read** → Q2 |
| '645 cl.1; '861 cl.1; '445 cl.1 (2031) | avoids [1]/[2] if output is not organised as spiral loops; **may read** if it is, since [3] needs no preselected band | avoids [2] uniform-stepover offsets; MA circles are not one spiral | n/a-avoids | n/a-avoids | n/a-avoids | **may read** → Q2 (weakest point) |
| '224 cl.30/36; '704 cl.2-3; '445 cl.2-3 (2031) | avoids [1]-[2] (no spiral-plus-trochoid split) | avoids [3] if the trochoid is used only where the cap is hit, not chosen to minimize time; '704 cl.2 [3'] **unknown** | n/a | n/a | n/a | avoids [3] |
| '224 cl.45/51; '704 cl.4; '445 cl.4 moat (2031) | avoids [3] no moat pass | avoids [3] if offsets come from boundary+islands together (classic) | n/a | n/a | avoids [3] | avoids [3]; Q3 (is a first offset around an island a "moat"?) |
| '224 cl.57/63/69/75; '704 cl.5-6; '445 cl.5/7 channels (2031) | avoids [1] no time comparison | avoids [1] | n/a | n/a | avoids [1] | avoids |
| '607 cl.1/4/7 (2031) | avoids [1]-[2] DOC/WOC user-set, not from spindle power | avoids [1] | n/a | **avoid** deriving DOC/WOC from power; feed-only power clamp keeps [2] absent | n/a | avoids [2] |
| '282 cl.1 etc. 3D step-up (2033) | avoids [3]-[6] (2.5D only) | avoids | n/a | n/a | n/a | avoids; if forgeCAD adds 3D rest roughing, do not choose step-ups by offset/scallop envelope minimization |
| '019 cl.1/5 thin wall (2031) | avoids [1]-[2] (no automatic two-stage wall strategy) | avoids | n/a | n/a | n/a | avoids |
| '293 cl.1/5 trochoid vs spiral time choice (2031) | avoids [1]-[3] | avoids | n/a | n/a | avoids | avoids |
| '264 pending cl.38/74 | n/a | n/a | avoids [2]-[4] (rule, not time estimate) | n/a | n/a | avoids, but **unknown** until it issues |
| Celeritive '972 cl.1/15/17 (2030) | n/a | n/a | avoids [2]-[4] no transition toolpaths/areas, no trimming | n/a | n/a | avoids [2] |
| Celeritive '212 cl.1/10/22/26 (2029) | n/a | n/a | avoids [2], [5] | n/a | n/a | avoids [2] |
| Celeritive '245 cl.1; '040 cl.1/8 (2032-33) | n/a-avoids (no VD) | avoids [2] unmodified MA/VD; **may read** if branches are added to reach corners | n/a | n/a | n/a | avoids [1] |
| Celeritive '040 cl.15 (2032) | n/a | avoids [2]-[3] (no shortened branch, no second VD) | n/a | n/a | n/a | avoids |
| Celeritive '176 cl.1/5/9 (2033) | avoids [1]-[2] no revision of a generated path | **may read** if a post-pass pushes corner arcs outward; avoid that | n/a | n/a (feed only) | n/a | avoids [1] if no post-pass rewrite |
| Truemill '113 cl.1/8/15 (2031) | avoids [2]-[4] | avoids [4] if corner loops are not joined back to a first arc by transition passes | n/a | n/a | n/a | avoids [4] |
| DP '649 cl.1/9 (2034) | avoids [1]-[5] | avoids [1] (no TEA-derived blend radius), [4]-[5] | n/a | D alone: avoids [1]-[5] | n/a | avoids [1] |
| DP '859 cl.1/9 (2034) | avoids | avoids (no PSR smoothing loop) | n/a | n/a | n/a | avoids |
| DP '847 cl.1/8/15 (2033) | avoids [1]-[3] | avoids [1], [2] | n/a | n/a | n/a | avoids |
| Delcam/Autodesk '339 cl.1/10 (2035) | avoids [2], [4] | avoids [2], [4] (uniform stepover, no backfill) | n/a | n/a | n/a | avoids [2] |
| Delcam/Autodesk '339 cl.18/26 (2035) | avoids [1], [3] (not offset subsets; no preceding-path enlargement) | avoids [3] cap by trochoid or feed, not by enlarging the previous radius | n/a | avoids [3] | n/a | avoids [3] |
| Autodesk '172 cl.1/12, all 22 read (2036) | n/a-avoids (no VD, no spiral around a hole) | avoids [3]-[4] unmodified MA, no wave model; a medial-axis skeleton used as a zero-area "hole" (cl.10/21) still needs [3]-[4] | n/a | n/a | n/a | avoids [3]-[4] |
| Boeing '372 cl.1/17/19/28 (2032) | avoids [1] if no axial-vs-radial engagement plot is used | same | n/a | **may read** if feed/engagement picked from a stability-lobe-style DOC map | n/a | avoids [1] (Q5) |
| Boeing '372 cl.14/33 | avoids [1]; also not an aircraft part | | | | | avoids |
| MERL '215 cl.1/11/13 (2035) | n/a | n/a | n/a | avoids [1] (no engagement-variation partition) and [2] (no abrasive-resistance term); cl.11/13 also avoid distance-field/ADF elements | n/a | avoids [1], [2] (Q6) |
| CNC Software '778 cl.1 (2045) | avoids [2a] (gouge check refuses, never modifies points), [2b] (links made inside the op function, no linking filter) and [4] (no node graph, no ordering numbers; code read §1.10) | same | avoids [2b] | n/a | n/a | avoids [2a], [2b]; [4] subject to Q8 |
| CNC Software '778 cl.10/13 (2045) | avoids [3] (no per-point normal + tip + orientation) and [4] (no de-gouge step producing corrected data); links precede checking | same | same | n/a | n/a | avoids [3], [4] |
| OPEN MIND '511 (2044) | n/a 3-axis | n/a | **unknown** for 3-axis links interpolating a subset of axes | n/a | n/a | out of scope for 3-axis v1; Q9 |
| Siemens '470 (2039) | n/a | n/a | n/a | avoids final element (no path correction from stiffness) | n/a | avoids |

**Net.** Combination A+C+D+E, with B used only in its plain form, avoids at least one required
element of every live independent claim I read. There are two exceptions where I cannot say it
does. (1) The SolidCAM spiral-loop machine/product claims ('645, '861, '445 cl.1, and '224
cl.13) could read on *any* clearing output that forms spiral loops at varying spacing whose
engagement rises and falls within each loop. The safest technical reading: make A's output
*not* a spiral of closed loops (finish each region with open passes and links), and hold
engagement near a single target. Whether "engagement held at a target ± tolerance" counts as
"gradually decreases from a maximum to a minimum" is a construction question (Q1, Q2). (2)
Claims not read in full: '282 tails, '019 tails. Autodesk '172 (all 22) and CNC Software
'778 (all 14; §1.10) are now read; both are avoided.

---

## 4. Truemill US 7,577,490 B2: what it alone blocks, and whether its expiry is confirmed

- **Claims [R]:** 16 claims; independents are 1 (method) and 9 (medium), with identical elements:
  1. a pass made of tangent-continuous arcs, each pass advancing into the region by a set pitch;
  2. a region with three sides: the first is the current in-process material boundary, the
     second and third are part or material boundaries, and the first meets the other two at its
     ends;
  3. a first arc *tangent to the second side* with radius **greater than half** the distance
     between the second and third sides at the points where the first side meets them;
  4. a second arc joined to the first, *tangent to the third side*, radius **less than half** that
     distance;
  5. output NC instructions for that path.
- **What only it blocks until expiry:** the two-arc construction for advancing passes into a
  three-sided region (a wedge or slot end), with one arc wider and one narrower than half the
  opening. The broader 2004 claims (store max engagement, shape passes to control it, circular
  passes into such a region) are in patents that have already expired or lapsed (§0 item 1).
  Candidates A-E do not build passes from this arc pair, so they avoid elements 3-4.
- **Expiry status:**
  - Maintenance fees for years 4, 8 and 12 were paid (2013-02-19, 2017-02-20, 2021-02-18; small
    entity). No lapse event appears [R].
  - Google shows "Adjusted expiration 2026-12-17" and status Active [R].
  - The patent text states it is a **continuation-in-part of application 11/070,430 filed
    2005-03-02** (that application became US 7,451,013) [R]. Under 35 U.S.C. 154(a)(2) the 20-year
    term normally runs from the earliest US non-provisional application whose benefit is claimed
    [K]. That puts the base at 2005-03-02 → 2025-03-02. Google's date equals the patent's own
    filing date (2005-04-22) plus 604 days. If those 604 days are PTA, adding them to the
    2005-03-02 base gives **2026-10-27**, a date already past. This is arithmetic on inferred
    numbers. The PTA figure was **not read** (the PTA notice is not on Google's page). Counsel or
    Patent Center should confirm (Q4).
  - Terminal disclaimer: none shown on Google, but that is UNKNOWN. Patent-term extension does not
    apply to this kind of patent [K].
  - **Bottom line:** it expires by 2026-12-17 at the latest (barring an error in Google's data),
    and possibly earlier. Shipping anything in the two-arc construction should wait until after
    that date.

---

## 5. Exact questions for counsel

1. SolidCAM US 8,489,224 cl.1 and US 9,052,704 cl.1: does a clearing algorithm that targets one
   removed-area-per-step value with an upper cap (no user or computed minimum) meet "selecting a
   minimum permitted engagement angle" or a "preselected minimum"? Does a numerical tolerance
   band (target ± ε) count as a selected min/max?
2. US 9,823,645 cl.1, 10,895,861 cl.1, 10,324,445 cl.1 and 8,489,224 cl.13: if forgeCAD's path
   forms loops around a region at varying spacing and its engagement incidentally varies within a
   loop, is that an "asymmetric spiral" whose engagement "gradually decreases from a maximum ...
   to a minimum ... at least twice in a single loop"? What output structure (open passes plus
   links, no closed loops) would clearly fall outside? Do the product-by-process claims in
   '445 matter for software we distribute?
3. The "moat" claims (8,489,224 cl.45; 9,052,704 cl.4; 10,324,445 cl.4): is a contour offset
   around an island, generated together with the outer boundary's offsets, a "moat tool path"
   configured "upon said tool path encountering an island"?
4. US 7,577,490: what is the recorded PTA, and is the term measured from 2005-03-02 (the CIP
   parent)? Is it already expired before 2026-12-17? Any terminal disclaimer?
5. Boeing US 8,676,372: does letting a user enter axial and radial depth of cut, or a
   chipload/engagement table, amount to "identifying ... using an engagement plot of axial depth
   of cut versus radial depth of cut"? Would a stability-lobe import feature read?
6. MERL US 9,892,215: is a user chipload (per material) a "parameter defining an abrasive
   resistance of a material"? Is splitting moves at a fixed arc length (not by engagement
   variation) outside "partitioning ... such that the variation ... is within a predetermined
   range"?
7. Celeritive US 10,108,176 cl.1 is broad (replace an arc of a previous path with curves
   containing a new arc closer to the boundary). Does any corner post-processing (arc fitting,
   G2/G3 output, smoothing) that moves an arc outward read on it? Is MATHSM 2005 invalidating art?
8. CNC Software US 12,663,778 B1 cl.1: forgeCAD runs machining statements in source order, each
   op one function that emits cuts with links already in them, then posts, reads back and
   *checks* (refuses on a gouge, never modifies points) (§1.10). Does source-statement order, with
   later statements naming earlier ones, count as nodes "arranged as a directed acyclic graph"
   with "ordering numbers"? Does a check that refuses, with no correction, fall outside a
   "de-gouging node ... adapted to ... modify"? Cl.10/13 need no graph and no machining: is
   adding per-point normal, tip and orientation (element 3) the clear line for 3-axis output?
   Is there prior art (OpenCAMLib, FreeCAD Path, Mastercam's own earlier releases, before
   2025-06-27) on the record?
9. OPEN MIND US 12,485,511: does a 3-axis link that interpolates Z separately from XY meet the
   "interpolation group of at least one and no more than N−1 axes"?
10. Liability model: most method claims end with "milling / machining the workpiece". forgeCAD
    only *generates* G-code. Assess direct infringement of the system and medium claims (SolidCAM
    apparatus claims, Celeritive cl.17, MERL cl.11, CNC Software) versus induced infringement
    through users, for open-source (MPL-2.0) distribution with no revenue.
11. Validity: do libactp (2004 headers; Debian 2009-05-05), Stori & Wright 2000, MTU 7,237,989
    (2003) and the Surfware 2005 publications anticipate or render obvious the broad SolidCAM
    cl.1 or Delcam '339 cl.18? Can the libactp public-availability date be established
    (code.google.com archives)?
12. Do any EP, DE or IL counterparts (SolidCAM EP 2681683, EP 3039496; DP EP 2856267; OPEN MIND
    EP 4137899 pending) matter for forgeCAD's users outside the US?

## 6. Not reached or unverified

- USPTO Patent Center and ppubs (PTA, terminal disclaimers, file wrappers): not attempted. They
  need a JS app; the Google data stands in.
- Google Patents returned HTTP 503 ("automated queries") several times; it was retried after
  pauses and every page listed was eventually read. Its JSON search endpoint was used for the
  assignee and keyword lists.
- FreeCAD commit date: the commit page I read did not show it, and the GitHub API is not enabled
  for this session. 2018-08-19 is the coordinator's figure.
- Papers were not read in full (citations from search snippets).
- ModuleWorks, CloudNC (only GB201801021D0 found), Sandvik and Kennametal milling-path patents:
  no relevant US grants surfaced in the searches run; the searches were not exhaustive.

## 7. Addendum (2026-11-24, from unclaimed-methods.md)

Found by the unclaimed-methods search after this note was written. Read on Google Patents; see that
note for queries and sources.

**New live claims to add to the §3 matrix:**
- **Boeing US 9,164,503 (to 2034).** Cut a slot along the medial axis with a circular recess at
  each end, then spiral outward at constant radial depth. This is close to Fusion's slot
  clearing.
  - Design rule: don't combine an end-recessed medial-axis slot with an outward constant-depth
    spiral.
  - Read its independent claims before building any slot-first clearing. **Not yet read claim by
    claim.**
- **Surfware US 9,317,030 (to 2031).** Stepped helical entry. Check before building stepped
  helices; a plain single helix at the tool's ramp limit is the default.
- **Now granted, from published applications:**
  - Truemill 9,931,703 (two-hole entry);
  - DP 10,877,463 (multi-axis jerk limit, to 2039);
  - Siemens 8,923,999 (claims not read);
  - Autodesk 12,427,615 (machine wear).

**Free, and new prior art against the live claims:**
- **JP 2005-050255** (Yamaji, Ibaraki). It claims Ibaraki's constant-engagement method; Japan
  refused it in 2005, and it was filed only in Japan. It is dated prior art against Delcam/Autodesk
  9,489,339 cl.18.
- **Boeing US 6,591,158** (Bieterman & Sandstrom morphed spiral): fee-lapsed 2015.
- **Kakino US 6,942,436** (constant-chip spiral; trochoid with the feed varied within a turn):
  fee-expired.
- **Kakino US 6,772,039** (feed per tooth from a force model): expired 2023.
- **Abandoned in the US:** EPFL US 2014/0297021 (corner looping); Hexagon 2021/0026327; Autodesk
  2020/0201284; Siemens 2014/0316751; MERL 2013/0262066.

**Design note.** To keep engagement-capped offsets clear of Delcam '339 cl.18, reshape the current
pass (move its points along the normal, or round it at a user radius). Never enlarge the
*preceding* pass's corner radius in response to a predicted engagement.

**Counsel question 13.** US 9,164,503: does a slot-clearing mode that cuts along a medial axis
and then offsets outward read on it if the slot ends are not recessed circles?

## 8. Emergent spirals from a cap-driven step rule (question from TOOLPATH-QUALITY, 2026-11-24)

**The case.** kernel/forge-cam/src/adaptive.rs, on cam/quality, implements this rule. From the
current position, take the largest turn toward material whose engagement on the cleared-area grid
is at most one cap. There is no minimum and no band. A pass ends when no turn keeps the cap even
at 1/8 step, or after about two radii in air. In a closed rectangular pocket, entered by a helix
at its widest point, the rule *emergently* produces one continuous outward spiral: about 30
turns, stepover set only by the cap, engagement roughly constant, rising in the corners.

**Reading against '645 cl.1, '861 cl.1, '445 cl.1** (§1.1; engineering analysis, not legal
advice). These are machine and object claims about the path the tool follows. How the path was
computed does not appear in them, so "emergent" is no defence for these claims. Element by
element:
1. **An asymmetric, non-circular spiral.** A rectangular outward spiral is one.
2. **Sequential loops separated by different distances at different places.** Likely met: a
   cap-set stepover is not uniform around a rectangle; it is narrower where the corners raise the
   engagement.
3. **Within each loop, engagement falls from a max to a min and rises again (at least twice per
   loop in '645/'445).** Likely met: four corners per loop give four rises and falls. No
   preselected values are needed.

**Verdict: high risk.** This is the configuration the matrix row marks "may read", made concrete.
'224 cl.13 is also at risk: an asymmetric spiral of at least two loops whose stepover rises and
falls cyclically.

**Would ending a pass at a heading turn of 300° or more help?** Somewhat, but not clearly. Each
pass becomes open and is joined by a fixed-rule link. Whether a series of nearly closed passes
with short links is still "a spiral with sequential loops" is claim construction. The overall
pattern of the machined path is what a machine claim reads on, so a court could look through
short links. Treat it as reducing the risk, not removing it. Passes that end when they meet
their own earlier cut give essentially the same pattern, with the same doubt.

**Constructions that remove a required element structurally** (recommended for closed pockets
until counsel answers Q2):
- **(a) Uniform-stepover offsets.** Constant stepover at every place (removes element 2), with
  corners rounded at a user radius, engagement *certified* rather than steered, and the chip held
  by the per-move feed. Prior art: Held 1991 and every 1990s CAM. Engagement is bounded but not
  constant; the feed takes up the variation.
- **(b) One-direction "peel" passes.** The same cap-driven step rule, but along passes that sweep
  across the region in one general direction and never wind around a centre, each ended at the
  boundary and linked back through the cleared area. No spiral and no loops (removes elements 1
  and 2). It is still constant-engagement per step.
- **(c) Circles or trochoids along the medial axis.** No single spiral; prior art: Elber, Cohen &
  Drake 2005; Rauch et al.
- **Keep the cap-driven rule as is only where its output cannot loop:** open-sided regions, rest
  regions and slots, where it naturally gives open passes.

**Islands.** Outside clearing that produces partial loops around each island also approaches the
"moat" claims ('224 cl.45; '704 cl.4; '445 cl.4: when the path meets an island, run a path that
surrounds it). Keep passes near islands as parts of the region's general sweep. Don't generate a
pass whose purpose is to encircle an island on first contact. Counsel Q3 stands.

**Added to counsel Q2:** the concrete output above (an emergent, cap-driven rectangular spiral
of about 30 loops with corner engagement peaks), and the 300° split variant.

## 9. Review of the peel design (cam/quality @41d99cd3, kernel/forge-cam/src/adaptive.rs), 2026-11-24

Engineering review for David and counsel; **not legal advice**. Asked by TOOLPATH-QUALITY and the
coordinator. The design as described to me, and checked against the module header:
1. Per level, a cleared-area grid. Each step takes the largest turn toward material whose
   engagement is at most ONE cap; no minimum, no band.
2. Every pass heads within ±75° of one fixed, user-set sweep direction, so its total turn is at
   most 150°. A pass ends at the boundary, when no turn keeps the cap, or after about two radii in
   air.
3. Starts are in cleared air with material ahead (an open side first); a plain helix only if
   there is none.
4. Links follow a fixed rule: straight at depth if clear, else retract, rapid, plunge.
5. Afterwards come plain offset rings (offsets of boundary and islands together, uniform spacing),
   only where material remains, then the plain wall ring.
6. Islands are met only by the sweep.
7. Closed pockets are refused (they go to the offset pocket, §8 (a)).
8. Feeds are set per fixed-length sample.

| live claim (§1) | required element the design lacks | verdict |
|---|---|---|
| SolidCAM '224 cl.1/5, '704 cl.1 | a selected or preselected **minimum** (one cap only) | avoids |
| '224 cl.13; '645 cl.1, '861 cl.1, '445 cl.1 | a **spiral** with sequential loops (each pass turns at most 150° about one fixed direction, so it cannot wind round a point) | avoids, if the two-way rule below holds |
| '224 cl.30/36, '704 cl.2-3, '445 cl.2-3 | a spiral portion; a split chosen by time | avoids |
| '224 cl.45/51, '704 cl.4, '445 cl.4 (moat) | a path run "when the path meets an island" to surround it (6: the sweep meets islands; the step-5 rings are a final clean-up of the whole region from boundary and island offsets together, not a response to meeting an island) | avoids, on my reading; **counsel Q3 stays open** (see below) |
| '224 cl.57-75 (channels) | separating channels; a time comparison | avoids |
| '607 | depth and width picked automatically from spindle power | avoids, if power stays a check |
| Celeritive '972/'212 | transition toolpaths or areas | avoids |
| Celeritive '245/'040, Autodesk '172 | a (modified) Voronoi diagram | avoids |
| Celeritive '176 | revising arcs of a previously generated path | avoids (steps are never revised) |
| Delcam/Autodesk '339 cl.1/18 | offset subsets with a larger stepover plus backfill; enlarging the preceding path's radius | avoids |
| DP '649/'859/'847 | a blend radius from the allowed engagement change; trochoidal links | avoids |
| Truemill '113; '490 (to 2026-12-17) | corner arcs plus transition passes; the two-tangent-arc construction | avoids |
| Boeing 8,676,372 | an engagement taken from an axial-vs-radial depth-of-cut plot | avoids, if the UI takes the cap as a number and offers no such plot |
| Boeing 9,164,503 | a medial-axis slot with circular end recesses, then an outward spiral | avoids |
| MERL 9,892,215 | segmentation by engagement variation; an abrasion parameter | avoids (fixed-length samples) |
| SolidCAM US 2025/0284264 (pending) | a link type chosen by estimated time | avoids (fixed rule) |
| Surfware 9,317,030; Truemill 9,931,703 | a stepped helix; a two-hole entry | avoids (plain helix) |
| CNC Software '778 | a de-gouging node that modifies the path, … | avoids (the checker only refuses) |
| OPEN MIND 12,485,511 | Q9 (axis-group interpolation in links) | open, but it applies to every CAM's retracts, not to this design |

**Step 5 (rings around islands).** On my reading the rings avoid the moat element:
- they are a final clean-up of the whole region, from offsets of boundary and islands together
  (classic contour-parallel pocketing with islands, Held 1991);
- they are emitted only where material remains;
- they are not triggered by the path meeting an island.

Two ways to make that stronger, both cheap:
- emit only the arcs of a ring that actually cut material (partial rings), so that no ring is
  generated *in order to* encircle an island;
- make no ring around an island before the sweep has finished the level.

The plain wall ring is a finishing contour (old prior art). Counsel Q3 still decides.

**The two-way variant.** Each pass picks +sweep or −sweep at its start, never both. It stays in
(b), with one condition. Two consecutive passes in opposite directions on opposite sides of an
island, or of any uncut core, together with their links, trace a loop around it. That loop is a
spiral-like and moat-like pattern, made from parts that are each fine.

Rule: order the passes so that their positions across the sweep advance monotonically, like a
zigzag (boustrophedon). Never follow a +sweep pass on one side of an island or core with a
−sweep pass on its other side.

Test: for every island, and for a grid of points in the region, the winding number of the whole
path (passes plus links, closed by its start–end chord) is 0. It must also be 0 for the one-way
variant. Raster zigzag with alternating directions is classic direction-parallel pocketing (prior
art), so with that ordering rule the two-way variant is fine.

**Written OK (engineering, not legal).** The peel design as built at 41d99cd3, with closed
pockets refused, avoids at least one required element of every live independent claim read.
The conditions:
1. Rings around islands may be partial, and are emitted only after the sweep.
2. The two-way variant keeps a monotone cross-sweep order and passes the winding-number test.
3. Power is a check only, and the cap is a number (no ae-vs-ap plot).
Residual for counsel: Q3 (moat), Q9 (applies to all CAM), and Q10 (whether a generator alone can
infringe).

**§9 clarification (2026-11-24, after TOOLPATH-QUALITY ran the winding test).** One-way passes
with their return links each enclose a strip of already-cleared air, where |w| = 1. Classic
one-way raster pocketing ("zig") has the same property, and it is old prior art. What the
spiral-loop and moat claims describe is loops that *nest around a common point* (a spiral) or
encircle an island or uncut material.

So condition (2) reads:
- (i) winding 0 around every island, and around every point of material still uncut when the
  loop is made;
- (ii) |w| ≤ 1 at every point. No two pass-plus-link loops nest or overlap, so the loops progress
  side by side across the sweep, never concentrically;
- (iii) passes start in monotone cross-sweep order. This is what guarantees (ii).

Zero winding everywhere is NOT required. Retracing zigzag returns are not needed for FTO.

**§9 correction (2026-11-24, after TOOLPATH-QUALITY's data on cam/quality-adaptive-wip @b5605f1f).**
The test proposed above, winding of a pass closed by its start–end chord, is a poor proxy, and it
is withdrawn. The chord is not a move the tool makes. An L-shaped peel (along the bottom, then up
the far side, within ±75° of the sweep) closed by its chord "encloses" an island the tool never
goes round. What the spiral-loop and moat claims describe is the tool *travelling round* a point.

The binding reading is now:
- (i) **Per pass:** the total heading change, |Σ Δheading|, is at most 2·PEEL (150°). A single pass
  cannot go round anything.
- (ii) **Whole path:** for the path as travelled (passes, links and rapids, in order, never closed
  by a chord), the running angle swept about every island point, and about every point of material
  uncut at that moment, stays strictly inside (−2π, 2π) at every prefix. The path never completes
  a revolution about it. A spiral or moat accumulates whole turns; peel passes with returns
  oscillate.
- (iii) **Monotone cross-sweep start order** stays, as good practice. Condition (ii) is the test.

PEEL = 75° may stay. An L-peel subtends at most about 180° of an island and its link returns. Lower
PEEL only if (ii) fails.

**On bands round a boss** (RPTU parts 3–6):
- Plain offset rings of the island, outward-in, beat every peel and zigzag variant: part 3 clear
  46 s against Autodesk's adaptive at 63 s, which is itself 3 closed offset rings. Fusion uses
  offsets there too.
- Offsets of boundary plus islands together are classic prior art, under the §9 reading of the
  moat claims (Q3 open).
- So adaptive is for deep, open levels at full flute length, and bands stay with `clear`.

**§9 note: the wall pass and the link fix** (2026-11-24, from cam/quality-adaptive-wip, test
`path_winds_round_nothing`).

Test (ii) first failed for one-way peels. Each L-peel plus its diagonal rapid back to the next
start made a full turn about an island corner (1622° at PEEL 75°; 518° at 45°; 374° about an
uncut point at 30°). The fix keeps PEEL 75°: a retract link returns over the pass it follows, at
retract height, then hops to the next start. With it, all six cases pass (one- and two-way;
sweeps 0°, 90° and 30°).

The final wall pass is excluded from test (ii) on this reading. It is one plain offset of all
walls (boundary and islands together), run after the level's sweep and cleanup: a finishing
contour, which is classic prior art. It is neither a spiral nor a ring generated on meeting an
island.

Conditions, to be asserted in code:
- (a) the wall pass runs only after the level is cleared to stock-to-leave;
- (b) it is one offset at a single distance;
- (c) cleanup rings stay partial: only the arcs that cut material.

Counsel Q3 is unchanged.
