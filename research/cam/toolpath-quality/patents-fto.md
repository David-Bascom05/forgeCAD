# Toolpath-quality patents: broad survey (extends ../patents.md)

**Research, not legal advice. Every item needs patent counsel before forgeCAD ships the
feature.** This file extends `../patents.md` (2026-11-23). The claim-by-claim analysis,
element matrix and counsel questions for adaptive clearing and adaptive feed are in
`fto-adaptive.md` (this folder); read that first.

Date of reading: **2026-11-24**. Marks: **[R]** = the Google Patents page (claims section,
legal events, family tables) was fetched and read directly this session. **[S]** = search
result or listing only. **[K]** = background knowledge, not re-read. "Expiry" = Google's
adjusted or anticipated expiration (Google: "an assumption, not a legal conclusion"). A
terminal disclaimer is UNKNOWN for every patent below, because Google does not reliably show
them. Each patent's URL is `https://patents.google.com/patent/<number>/en`.

Risk column: the likelihood that a plain forgeCAD 2.5D/3-axis roughing and finishing feature
set would come near the claims. **H** = a natural implementation could read on it; **M** =
only certain constructions read on it; **L** = a different subject, or it is expired.

## 1. Summary table

| Patent | Title (short) | Assignee (current) | Priority | Expiry | Status 2026-11-24 | Would block | Risk | Src |
|---|---|---|---|---|---|---|---|---|
| US 7,577,490 B2 | Engagement milling | Truemill | 2004-04-29 (CIP of 2005-03-02 appl.) | 2026-12-17 (maybe 2026-10-27, see fto §4) | Active; fees 4/8/12 paid | two-arc passes into a 3-sided region | L (weeks) | R |
| US 7,451,013 B2 | Engagement milling | Truemill | 2004-04-29 | 2025-05-23 | Expired | (was broad: shape passes to a stored max engagement) | free | R |
| US 7,831,332 B2 | Engagement milling | Surfware | 2004-04-29 | 2025-03-02 | Expired | transition arcs from stepover/max engagement | free | R |
| US 8,000,834 B2 | Engagement milling | Surfware | 2004-04-29 | 2025-03-02 | Expired | isoloop spiral blending | free | R |
| US 8,694,149 B2 | Circular tool passes | Truemill | 2004-04-29 | 2025-10-17 | Expired; also fee lapse (FP 2026-04-08) | circular passes with engagement cap | free | R |
| US 8,560,113 B2 | Milling an interior region | Truemill | 2010-04-13 | 2031-09-03 | Active (12th fee paid late 2025-10-03) | corner arcs + transition passes back to first arc | M | R |
| US 9,931,703 B2 | Starting hole for milling | Truemill | 2009-12-21 | 2031-06-28 | Active | entry: stepped holes, larger first then smaller | M (entry) | R |
| US 6,704,611 B2 | Rough milling | Truemill (ex-Surfware) | 2001-08-21 | 2022-08-21 | Expired | — | free | R |
| US 8,295,972 B2 | High performance milling | Celeritive | 2008-10-07 | 2030-06-23 | Active; 4/8/12 paid | transition areas + trimmed cuts + links inside them | M | R |
| US 8,880,212 B2 | High performance milling | Celeritive | 2008-10-07 | 2029-10-07 | Active; 12th paid 2026-04-28 | same, no trimming | M | R |
| US 9,946,245 B2 | Non-concentric milling | Celeritive | 2011-07-25 | 2033-09-28 | Active | VD with added branches → faces → path | M | R |
| US 10,108,176 B2 | Non-concentric milling | Celeritive | 2011-07-25 | 2033-07-05 | Active | replace an arc of an existing path with one closer to boundary | **H** (broad) | R |
| US 10,579,040 B2 | Non-concentric milling | Celeritive | 2011-07-25 | 2032-07-25 | Active | modified VD; shortened branch + 2nd VD | M | R |
| US 10,987,774 B2 | Multi-axis milling | Celeritive | 2012-05-03 | 2033-10-11 | Active | 5-axis orientation via domain mapping | L | R |
| US 8,489,224 B2 | Computerized tool path generation | SolidCAM | 2011-02-28 | 2031-08-20 | Active; 4/8/12 paid | min/max engagement band; asymmetric spiral; moat; channels | **H** | R |
| US 9,052,704 B2 | same | SolidCAM | 2011-02-28 | 2031-02-28 | Active | method-of-machining versions of above | **H** | R |
| US 9,690,282 B2 | same (CIP) | SolidCAM | 2011-02-28 | 2033-01-04 | Active | 3D step-up rest roughing between offset/scallop surfaces | M (3D) | R |
| US 9,823,645 B2 | same | SolidCAM | 2011-02-28 | 2031-10-22 | Active | machine: asymmetric spiral, engagement cycles ≥2× per loop | **H** | R |
| US 10,324,445 B2 | Object ... asymmetric spiral | SolidCAM | 2011-02-28 | 2031-03-22 | Active | product-by-process versions | M | R |
| US 10,620,607 B2 | same | SolidCAM | 2011-02-28 | 2031-05-26 | Active | DOC/WOC from spindle power + asymmetric spiral | M | R |
| US 10,895,861 B2 | Automated machine ... | SolidCAM | 2011-02-28 | 2031-05-23 | Active | machine: asymmetric spiral with engagement swing | **H** | R |
| US 11,625,019 B2 | same | SolidCAM | 2011-02-28 | 2031-05-02 | Active | thin-wall two-stage strategy | L | R |
| US 12,248,293 B2 | same | SolidCAM | 2011-02-28 | 2031-05-07 | Active | choose trochoid vs spiral by estimated time | M | R |
| US 2025/0284264 A1 | same | SolidCAM | 2011-02-28 | (pending) | Pending, docketed 2025-06-05 | choose clearance-plane vs stay-down link by estimated time | M | R |
| US 11,048,224 B2 / 11,841,693 B2 | Chatter-free CNC program | SolidCAM | 2017-03-20 | 2037-03-20 | Active | DOC/rpm chatter map from history | L | R / S |
| US 9,778,649 B2 | Profit milling | Hexagon (ex-DP Technology) | 2012-06-01 | 2034-02-03 | Active; 4/8 paid | TEA-based blend radius, centre area, MRR cap | M | R |
| US 9,958,859 B2 | Profit milling | Hexagon | 2012-06-01 | 2034-10-13 | Active | iterative PSR smoothing of offset groups | M | R |
| US 10,488,847 B2 | Profit milling | Hexagon | 2012-06-01 | 2033-09-15 | Active | smoothed offsets linked by trochoids in narrow areas | M | R |
| US 8,977,382 B2 | Channel cavities (Sona) | Hexagon | 2012-05-11 | 2033-07-27 | Active; 12th paid 2026-03-30 | 5-axis channel trochoid + polishing pass | L | R |
| US 9,423,788 B2 | Channel cavities | Hexagon | 2012-05-11 | 2032-05-11 | Active | 5-axis flank positions + upper/lower pseudo-circles | L | R |
| US 9,927,801 B2 | Channel cavities (CIP) | Hexagon | 2012-05-11 | 2032-05-11 | Active | 5-axis coupled flank positions + trochoid | L | R |
| US 7,237,989 B2 | Milling structural components | MTU Aero Engines | 2003-06-19 | 2024-06-11 | Expired | (curvature for max contact) | free | R |
| US 9,489,339 B2 | Material remover cutting paths | Autodesk (ex-Delcam) | 2011-12-12 | 2035-03-07 | Active; 4/8 paid | offset subsets+backfill; enlarge previous path radius on high engagement | **H** for offset roughing | R |
| US 10,108,172 B2 | Spiral toolpaths, polygonal pockets | Autodesk (inv. Abrahamsen) | 2014-10-13 | 2036-12-18 | Active; 4/8 paid | spiral from modified VD (one cycle, convex faces) + wave model, pocket with hole | M | R |
| US 11,176,291 B2 | Roughing tool sequences | Autodesk | 2018-08-21 | 2040-04-04 | Active | automatic roughing tool-set choice | L | R |
| US 11,782,396 B2 / 12,487,565 B2 | RL toolpath generation | Autodesk | 2020-06-22 | 2041 | Active | ML toolpaths | L | R |
| US 10,884,392 B2 | Deflection-based stock | Autodesk | 2018-08-23 | 2038-10-20 | Active | stock allowance from deflection simulation | L | R |
| US 8,676,372 B1 | Tool path generation | Boeing | 2011-07-19 | 2032-06-11 | Active; 4/8/12 paid | engagement chosen from axial-vs-radial DOC plot → path | M | R |
| US 9,892,215 B2 | Feedrates of machining tools | MERL | 2013-09-23 | 2035-04-16 | Active; 4/8 paid | feed per constant-engagement segment incl. abrasion parameter | **H** for adaptive feed | R |
| US 9,575,486 B2 | Oscillator pocket path | MERL | 2014-03-26 | (2035) | Expired (fee lapse 2021-02-21) | — | free | R |
| US 11,567,470 B2 | Optimization of NC machining | Siemens AG | 2019-01-28 | 2039-12-19 | Active | force model → toolpath corrected by machine stiffness | L | R |
| US 12,663,778 B1 | Tool path generation using CAM | CNC Software (Mastercam) | 2025-06-27 | ~2045-06-27 | Active (granted 2026-06-23) | DAG pipeline of generator/de-gouge/link/post nodes | M (architecture) | R |
| US 12,485,511 B2 | Machining path, multi-axis | OPEN MIND Technologies AG | 2021-08-18 | 2044-06-20 | Active | multi-axis links interpolated per axis subset | L (3-axis) | R |
| US 11,351,618 B2 | Fillet machining, conical-convex tool | OPEN MIND | 2016-09-22 | 2038-01-11 | "Active - Reinstated" after 2026 fee lapse | barrel-tool fillet machining | L | R |
| US 10,564,625 B2 | High speed tool path | individual (S. Singh) | 2014-02-21 | 2035-05-25 | Active | wall cutting + corner passes rule (>90% flat width) + spring pass | L | R |
| US 5,828,574 A | Adaptive feedrates from geometry modeling | GE | 1997-05-19 | 2017-05-19 | Expired | — | free | R |
| US 4,833,617 A | Solid-model adaptive feedrate | GE | 1987-08-14 | — | Lapsed 1993 (fees) | — | free | R |

## 2. Families in detail

### 2.1 Surfware / Truemill
- **Claim 1s [R].** '013 cl.1: store an engagement parameter; use the tool size and that value
  to shape passes so engagement is controlled; circular region, concentric circles sized for
  equal engagement. Cl.14 is the broadest: concentric indexed circles at two or more points,
  joined into "isoloops", blended into a path. '332 cl.1: transition portions between offset
  passes built from four arcs, with radii from stepover and max engagement. '834 cl.1: isoloop
  spiral. '149 cl.1: circular passes advancing into a region from its first side, centre and
  radius set so engagement does not exceed a set value. '490: see fto §4. '113: fto §1.3.
- **Status.** Expired: '013 (2025-05-23), '332 and '834 (2025-03-02), '149 (adjusted 2025-10-17,
  with a fee lapse recorded 2026-05-11, effective 2026-04-08). '834's 12th-year fee was paid
  late with surcharge in 2023. Live: '490 (to 2026-12-17 or earlier), '113 (2031-09-03),
  '703 (starting hole, 2031-06-28). The family has a Darts-ip "litigation" flag [R].
  EP 1745400 B1, CA 2564404 and KR 101070714 are Expired/Fee-related per Google's family
  table [R].
- **Design-around.** For '113, do not join corner arcs back to a first arc with transition
  passes. For '703, enter by helix or ramp, not by milling a large hole and then a smaller,
  deeper hole on the same axis.
- **Prior art.** Stori & Wright 2000 [S]; MTU '989 2003 [R].

### 2.2 Celeritive (VoluMill)
- Claims: fto §1.2. The new members found this session are **US 9,946,245 and US 10,108,176**
  (not in patents.md). '176 cl.1 is the broadest live Celeritive claim: revise an existing path
  by replacing an arc with curves containing an arc nearer the boundary.
- Design-around: links by rule (fto C); no transition areas; plain medial axis; never rewrite
  arcs of a finished path outward.
- Prior art: MATHSM 2004/2005 (maximal inscribed circles on the medial axis) [S]; Held 1991 [K].
- No later continuation was found in Google's family table (last filing 2018-10-19) [R].

### 2.3 SolidCAM (iMachining)
- Claims: fto §1.1. The family on Google: US 8,489,224 → div. 9,052,704 → cont. 9,823,645 →
  10,324,445 → 10,895,861; and CIP 9,690,282 → div. 10,620,607 → 11,625,019 → 12,248,293 →
  US 2025/0284264 (pending; published claims 38/74 on link-time choice) [R]. The chain is still
  being continued, so new claims may be drafted against competitors until 2031.
- Expiry: 2031-02-28 to 2031-10-22 for the 2011-priority members; 2033-01-04 for '282 [R]. The
  spread of dates comes from PTA.
- Design-around: fto §3 (single target, no min band, no cyclic-engagement spiral loops, no moat
  pass, no time-based strategy or link choice, no power-derived DOC/WOC).

### 2.4 DP Technology / Hexagon (ESPRIT)
- Profit milling: fto §1.4. The newly found member is **US 9,958,859**. Sona channel patents
  are 5-axis: '382 cl.1 flank positions + trochoid + polishing pass; '788 cl.1 flank positions
  + upper/lower pseudo-circular trochoid patterns; '801 cl.1 extrapolated limit-curve points →
  flank positions → coupled trochoid [R]. They are not relevant to 3-axis roughing.
- Others listed only [S]: US 10,317,879 (CNC operations engine), US 10,007,254 (CAM-integrated
  CNC), US 9,869,990 (positioning moves), US 7,894,930 (2008, composite machining by path
  pattern types).

### 2.5 Trochoidal milling
- MTU US 7,237,989 [R]: expired 2024. It sets the curvature of successive paths so the
  circumferential contact stays at the permissible maximum. This is a broad, dated (2003) and now
  free form of engagement-held curved paths.
- Live trochoid claims are all narrow: SolidCAM (trochoid only for the remainder after a spiral,
  or chosen by time), DP '847 (trochoids in narrow areas of smoothed offsets), Sona 5-axis.
- A Google Patents search for `trochoidal` + `toolpath`, US grants with priority ≥2006, gave 14
  hits; the relevant ones are listed in §1. Also noted: Kyocera SGS US 11,850,669 "Slot milling
  sequence" (2017) [S, not read]. Check it before shipping trochoidal slotting.
- **Safe form:** a user-parameterised trochoid (circle radius, advance per loop) along a slot
  centreline or medial axis, used where the cap would be exceeded. No time-based choice against
  spirals.

### 2.6 Adaptive feed rate
- Free: GE '617 (1987; claims per-move MRR, force and deflection by off-line solid simulation,
  adjusted feed under user constraints, lapsed 1993) and GE '574 (1997, expired 2017) [R].
- Live: MERL '215 (fto §1.8) [R]; Boeing '372 (engagement from axial/radial DOC plot) [R];
  Siemens '470 (stiffness-based path correction) [R]; Makino US 10,401,823 (real-time monitoring
  against pre-process simulation) [S, not read]; SolidCAM '224 chatter [R].
- Vendor products without a patent number found: CGTech OptiPath/Force (CGTech's US grants: only
  a fibre-placement patent [R list]); ModuleWorks (0 US results under that assignee [R list]);
  CloudNC (only GB201801021D0, "A method for machining", 2018 [S]); Productivity
  Inc/MachiningCloud, Sandvik CoroPlus and Kennametal milling feed patents were not found by the
  searches run (Sandvik hits were turning only: US 9,925,594, 12,172,218, 12,204,307,
  12,455,550 [S]).
- Safe form: fto candidate D.

### 2.7 Autodesk / Delcam / OPEN MIND / CNC Software / Siemens
- An Autodesk "toolpath" search returned 93 hits [R list]. CAM-relevant: 9,489,339
  (Delcam/Vortex-like), 10,108,172 (Abrahamsen spiral), 11,176,291, 11,782,396 / 12,487,565,
  10,884,392, 10,838,400 (toolpath by demonstration) [S], 11,307,559 (holding tabs) [S],
  11,809,160 (multi-tooltip) [S], US 2020/0201284 (automatic CAM generation) [S]. No patent
  titled "adaptive clearing" was found. HSMWorks Adaptive appears unpatented, or patented under
  another title (UNKNOWN).
- Delcam assignee search [R list]: 9,489,339 and 10,105,766 (blades/blisks); the rest is
  metrology and support structures.
- OPEN MIND [R list]: US 12,485,511 (assignee confirmed), US 11,351,618, US 9,977,421 (blank
  machining, 2013) [S], EP 1146408, EP 1590712, DE102005001600 (older, 1998-2004 priority).
  The 5-axis and barrel-tool items are out of scope for v1.
- CNC Software: one US patent, 12,663,778 (2025 filing, granted 2026-06-23) [R]. It claims a
  node pipeline architecture, not Dynamic Motion. No Dynamic Motion patent was found.
- Siemens Industry Software: 69 "milling" hits, none on adaptive clearing [R list]; US
  11,199,831 (adjust toolpath from 3D scan) [S].

### 2.8 Surface finish / smoothing (CAM side)
- Celeritive '176 (arc replacement) is the most relevant to corner smoothing done in CAM [R].
- Samarinder Singh US 10,564,625 [R]: wall finishing with a corner-pass rule (an extra pass if
  corner leftover exceeds 90% of the flat width) plus a spring pass, entry and exit at an angle,
  and overlap at the entry point. Avoid that exact combination of rules.
- A CPC G05B19/4103 search (corner smoothing, ≥2008) mostly returned controller-side patents
  (Fanuc 11,644,813; Mitsubishi 11,327,466; Shin Nippon Koki 8,478,438; Delta 8,121,720; MERL
  9,513,623; Makino 12,416,907) [S, not read]. These are controller interpolation patents;
  forgeCAD emitting G1/G2/G3 is unlikely to practise them. Read Makino 12,416,907 (a CAM-side
  "tool path generation method") before adding path smoothing.
- Arc fitting and G2/G3 output is decades-old practice [K]. NURBS output (G6.2/G5.2) was not
  searched.

## 3. Clearly free (expired or lapsed, with the prior art they supply)

- Surfware/Truemill 2004 family except '490: engagement-controlled passes, concentric/isoloop
  spirals, transition arcs, circular passes into a region (expired 2025; '149 also fee-lapsed)
  [R].
- MTU US 7,237,989 (2003): path curvature from the previous path to hold maximum contact,
  expired 2024 [R].
- GE US 4,833,617 (1987) and US 5,828,574 (1997): simulation-based per-move adaptive feed from
  MRR, force and deflection [R].
- Surfware US 6,704,611 (2001), expired 2022 [R].
- MERL US 9,575,486 (oscillator pocket path), lapsed 2021 [R].
- Never patented, or public before the live families: Stori & Wright 2000 constant-engagement
  spiral [S]; Bieterman & Sandstrom 2003 curvilinear spiral [S]; Choy & Chan 2003 corner swept
  angle [S]; MATHSM (Elber/Cohen/Drake) 2004-2005 [S]; Uddin/Ibaraki 2006 [S]; Held 1991 [K];
  trochoidal HSM 1990s [K]; Freesteel/libactp (2004 headers, GPL-2.0+; public by 2009) [R];
  FreeCAD Adaptive 2018 (LGPL-2.1+) [R/S].

### 3.1 Lapsed, fee-expired, ceased or abandoned: list for cross-reference (unclaimed-methods.md)

All statuses are as Google Patents showed them on 2026-11-24. "Fee" means the USPTO recorded an
unpaid maintenance fee (codes EXP./LAPS/FP), or Google shows "Expired - Fee Related".

| Document | Subject | Event | Src |
|---|---|---|---|
| US 8,694,149 B2 (Truemill) | circular passes advancing into a region, engagement ≤ cap | 12th-yr fee unpaid: LAPS 2026-05-11, FP effective 2026-04-08 (term also ended 2025-10-17) | R |
| US 9,575,486 B2 (MERL) | oscillator-based pocket path | 8th-yr fee unpaid: FP effective 2021-02-21 | R |
| US 4,833,617 A (GE) | solid-model adaptive feedrate | 4th-yr fee unpaid: lapsed 1993-05-23 | R |
| US 11,351,618 B2 (OPEN MIND) | barrel-tool fillet machining | fee lapse 2026-06-07, then shown "Active - Reinstated" (so NOT free) | R |
| KR 101070714 B1, CA 2564404 C (Surfware 2004 family) | engagement milling | "Expired - Fee Related" | R (family table) |
| WO 2005/111909 A1 (Surfware) | engagement milling PCT | "Ceased" | R (family table) |
| WO 2013/181392 A1 (DP profit milling) | profit milling PCT | "Ceased" | R (family table) |
| Expired at full term (not fee) | US 7,451,013; 7,831,332; 8,000,834; 7,237,989 (MTU); 6,704,611; 5,828,574 (GE) | term ended | R |

Published US applications seen in listings whose fate I did **not** check. Each may be
abandoned or may have become a patent; the other agent should verify:
- US 2011/0150592 A1 (Surfware, "Material Entry Tool Path", 2009). Possibly the parent of Truemill US 9,931,703, unverified.
- US 2016/0107246 A1 (Truemill, starting hole).
- US 2019/0171189 A1 (DP, toolpath virtualization and optimization, 2017).
- US 2021/0026327 A1 (DP, predictive machining engine, 2019).
- US 2020/0201284 A1 (Autodesk, automatic CAM generation, 2018).
- US 2022/0080545 A1 (Autodesk, evening out machine wear, 2020).
- US 2013/0060368 A1 (Siemens PLM, volumetric cut planning, 2011).
- US 2014/0316751 A1 (Siemens PLM, swept volume of a spun tool, 2013).
- US 2002/0119020 A1 (Delcam, machine tools, 2001).
- EP 4137899 A1 (OPEN MIND, counterpart of US 12,485,511) and DE 10 2021 121 398 A1: "Pending" [R family table].

All [S]: listings only.

## 4. Needs counsel

All the H and M rows in §1, and especially:
- SolidCAM '224 / '704 cl.1 and the '645 / '861 / '445 machine and product claims.
- Delcam/Autodesk '339 cl.18.
- MERL '215.
- Celeritive '176.
- Boeing '372.
- CNC Software '778 (architecture).
- The '490 expiry date.
- The pending SolidCAM '264.

The exact questions are in fto-adaptive.md §5.

## 5. A design that probably stays clear (summary; detail in fto §3)

1. Clearing: a cleared-area simulation with step-by-step steering to **one** target (removed area
   per step), capped, with no minimum band. Output is open passes plus links, not spiral loops
   with designed stepover variation. Earlier passes are never revised.
2. Optional plain-offset or medial-axis modes, kept to their classic forms: uniform stepover, an
   unmodified medial axis, no TEA-based blend radius, no previous-path radius enlargement, no
   backfill subsets.
3. Links: a straight move inside the cleared area, otherwise retract, decided by a fixed rule.
   No transition areas, no time comparison.
4. Feed: chip-thinning from engagement per fixed-length sample, using the user's chipload. No
   material-abrasion parameter, no partitioning by engagement variation. Follows GE 1997.
5. Islands cleared by the same simulation (no moat pass). No automatic channel, trochoid-vs-spiral
   or power-based DOC/WOC choices. No ML toolpath generation.
6. Entry: helix or ramp (avoid Truemill '703's stepped holes).
7. Keep this dated research and the design notes as a record of independent development.

Questions for counsel: fto-adaptive.md §5 (12 questions).

## 6. Unreachable or not done

- Patent Center and ppubs (PTA figures, terminal disclaimers, file wrappers): not attempted; JS
  application.
- Google Patents rate-limited (HTTP 503 "automated queries") several times; every page was read
  after waiting. No user-agent change was made.
- Espacenet: not used. EP and DE status comes from Google's family tables only.
- GitHub API not enabled for this session, so the FreeCAD commit date is unconfirmed. libactp
  was read by a public `git clone`.
- Full text of the papers was not read; citations come from search results.
- Not searched: Hexagon/Vero (Edgecam/WorkNC) under their own names; Esprit under Hexagon
  beyond DP; NURBS-output CAM patents; EP-only and DE-only families.
