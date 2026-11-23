# CAM patents to be aware of

**Check with counsel; not legal advice.** This is an engineer's reading list compiled from
public pages, not a freedom-to-operate opinion. Claim gists are paraphrases of what Google
Patents shows (often the abstract or summary, where the claim text was truncated); the scope of
a patent is set by its full claims, as construed, and expiry depends on fees, term adjustment
(PTA), terminal disclaimers and continuations. Google itself labels its status and expiry
fields "an assumption, not a legal conclusion".

Read 2026-11-23 unless noted. **[R]** page read directly, **[S]** search snippet only,
**[K]** background knowledge, not re-read. Expiry: Google's "adjusted/anticipated
expiration" where shown, else an estimate of 20 years from earliest non-provisional US filing.

## 1. Summary table (most relevant first)

| Patent | Title | Assignee | Earliest priority / filing | Expiry (Google) | Status 2026-11-23 | Src |
|---|---|---|---|---|---|---|
| US 8,489,224 B2 | Computerized tool path generation | SolidCAM Ltd | 2011-02-28 / 2011-02-28 | 2031-08-20 | Active | [R] |
| US 9,690,282 B2 | Computerized tool path generation | SolidCAM Ltd | 2011-02-28 / 2013-08-29 | 2033-01-04 | Active | [R] |
| US 12,248,293 B2 | (same family) | SolidCAM Ltd | 2011 family / 2023-03-10 | not read (likely ~2031 + PTA) | Active | [S] |
| US 8,295,972 B2 | High performance milling | Celeritive Technologies | 2008-10-07 / 2009-10-07 | 2030-06-23 | Active | [R] |
| US 8,880,212 B2 | High performance milling | Celeritive Technologies | 2008-10-07 / 2012-09-24 | 2029-10-07 | Active | [R] |
| US 10,579,040 B2 | Non-concentric milling | Celeritive Technologies | 2011-07-25 / 2018-10-19 | 2032-07-25 | Active | [R] |
| US 8,560,113 B2 | Method of milling an interior region | Truemill Inc (ex-Surfware) | 2010-04-13 / 2011-04-12 | 2031-09-03 | Active | [R] |
| US 7,577,490 B2 | Engagement milling | Truemill Inc (ex-Surfware) | 2004-04-29 / 2005-04-22 | 2026-12-17 | Active for ~3 more weeks | [R] |
| US 7,451,013 B2 | Engagement milling | Truemill Inc | 2004-04-29 / 2005-03-02 | 2025-05-23 | Expired | [R] |
| US 7,831,332 B2 | Engagement milling | Surfware Inc | 2004-04-29 / 2008-05-29 | 2025-03-02 | Expired | [R] |
| US 8,000,834 B2 | Engagement milling | Surfware Inc | 2004-04-29 / 2010-03-11 | 2025-03-02 | Expired | [R] |
| US 8,694,149 B2 | Engagement milling using circularly shaped tool passes | Truemill Inc | 2004-04-29 / 2011-02-03 | 2025-10-17 | Expired (fee related) | [R] |
| US 9,778,649 B2 | Profit milling | DP Technology (now Hexagon) | 2012-06-01 / 2013-05-30 | 2034-02-03 | Active | [R] |
| US 10,488,847 | Profit milling (continuation) | DP Technology | 2012 family | not read | presumed active | [S] |
| US 5,828,574 A | Adaptive feedrates from geometry modeling for NC machining | General Electric | 1997-05-19 | 2017-05-19 | Expired | [R] |
| US 4,833,617 A | Solid modeling based adaptive feedrate control for NC machining | not read | 1980s [S] | — | Expired (age) | [S] |
| US 9,423,788 / US 8,977,382 | Automatic method for milling complex channel-shaped cavities (trochoid) | Sona (per snippet) | not read | not read | check | [S] |
| US 7,237,989 | Milling method used for producing structural components (trochoidal) | not read | not read | not read | likely expired, check | [S] |

## 2. Families in detail

### 2.1 Surfware / Truemill "Engagement milling" (TrueMill)
Sources [R]: https://patents.google.com/patent/US7577490B2/en,
https://patents.google.com/patent/US7451013B2/en, https://patents.google.com/patent/US7831332B2/en,
https://patents.google.com/patent/US8000834B2/en, https://patents.google.com/patent/US8694149B2/en,
https://patents.google.com/patent/US8560113B2/en. Inventors Alan Diehl, Robert B. Patterson.
EP counterpart EP 1745400 B1 [S] (https://data.epo.org/gpi/EP1745400B1).

- **Gist.** Store a maximum cutter engagement; build each pass of a clearing path so the
  engagement never exceeds it; specific embodiments use spiral-like paths and transition
  sections made of tangent-continuous arcs sized from step-over and max engagement (8,000,834),
  and series of circular passes advancing into a region between converging/diverging sides
  (8,694,149). US 7,451,013's family is marked "has litigation" on Google [R].
- **US 8,560,113** (the number in the brief; it is Truemill's, not SolidCAM's): milling an
  interior region such as a pocket corner with a series of circular arcs joined by short
  transition passes, to cut non-cutting motion [R]. Active to ~2031-09-03.
- **Status.** All 2004-priority members are expired except **US 7,577,490 (to 2026-12-17)**.
  After that date the basic "cap engagement per pass" family is free in the US; 8,560,113
  remains.
- **Design-around until then.** Simplest: do not ship engagement-limited clearing in the US
  before 2027. Long-term: avoid the corner-arc-series-with-transitions construction of
  8,560,113 for interior corners, or get counsel's view on prior art (Freesteel adaptive
  clearing was shown in 2004-2006 [S], https://github.com/Heeks/libactp-old).

### 2.2 Celeritive VoluMill
Sources [R]: https://patents.google.com/patent/US8295972B2/en,
https://patents.google.com/patent/US8880212B2/en, https://patents.google.com/patent/US10579040B2/en.
Press: the patent covers "the ability to connect safely between cuts at high speeds" [S]
(https://news.thomasnet.com/fullstory/u-s-issues-celeritive-technologies-inc-patent-covering-key-innovations-of-the-volumill-tm-20001887).
Inventors Glenn Coleman, Evan C. Sherbrooke.

- **8,295,972 / 8,880,212 gist.** Given a main toolpath of cuts, compute "transition"
  toolpaths whose areas cross those cuts; trim cuts where they meet the transition areas; join
  cut ends with connecting moves that lie inside the transition areas, where the tool is mostly
  out of material; mill along all of it. The point is fast, safe links between cuts.
- **10,579,040 gist.** Build a Voronoi diagram of the region, add extra branches so the
  boundary elements differ from the original outline, and derive part of the toolpath from the
  faces of that modified diagram.
- **Expiry.** 2029-10-07, 2030-06-23, 2032-07-25.
- **Design-around.** Link between clearing cuts by (a) retract-and-rapid, or (b) links that
  run through already-cleared area computed from the cleared-area state (as FreeCAD Adaptive's
  `IsClearPath` / keep-tool-down does) without first constructing separate "transition
  toolpaths" whose regions cross the cuts and trimming cuts against them. For medial-axis
  clearing, use the plain medial axis/Voronoi of the boundary without synthetic branches, or
  offset-based spirals. Have counsel compare whatever we build to claim 1 of each.

### 2.3 SolidCAM iMachining
Sources [R]: https://patents.google.com/patent/US8489224B2/en,
https://patents.google.com/patent/US9690282B2/en; [S]
https://patentsgazette.uspto.gov/week10/OG/html/1532-2/US12248293-20250311.html. Inventors
Michael Berman, Doron Osovlanski, Christopher Calderone, Anthony Calderone. Large family:
US 9,052,704, 9,823,645, 10,324,445, 10,620,607, 10,895,861, 11,625,019, 12,248,293,
US 2025/0284264; EP 2681683 B1 (SI, HK), EP 3039496 B1, JP, KR, CN, AU, CA, SG [R].
SolidCAM says iMachining is patented and keeps "the cutting angle between defined minimum and
maximum values" [S] (https://solidcam.com/cam-modules/imachining-overview/).

- **8,489,224 gist.** Choose a maximum and a minimum allowed engagement angle; shape the path
  so the engagement angle varies gradually between them. Marketing adds "morphing spirals"
  that follow the region shape, with trochoidal moves near walls not to be cut.
- **9,690,282 gist.** Rough a 3D part by step-up cuts at several Z levels so all left
  surfaces lie between an offset (inner) surface and a scallop (outer) surface.
- **12,248,293 gist.** Estimate machining time for a semi-open region with a trochoidal path
  and with a spiral path, and pick the faster [S].
- **Expiry.** 2031-08-20 (8,489,224), 2033-01-04 (9,690,282); other members follow.
  **This is the family most likely to read on any modern adaptive clearing.**
- **Design-around.** Control a single target (removed area per step, or a single max
  engagement), not a min/max band with deliberate gradual variation; do not auto-choose
  between trochoidal and spiral by estimated time for semi-open regions; avoid the
  inner-offset/outer-scallop bounded step-up construction. Counsel must review claim sets of
  the whole family; continuations can be drafted to cover competitors' variants.

### 2.4 DP Technology (ESPRIT) "Profit milling"
Source [R]: https://patents.google.com/patent/US9778649B2/en; continuation US 10,488,847 [S]
(https://patents.justia.com/patent/10488847).
- **Gist.** Pick a corner-rounding radius from the allowed change in engagement angle; smooth
  successive offsets with that radius (or the previous radius plus step-over); cap curvature to
  limit engagement; define a central area from step-over and tool radius; trochoidal moves may
  replace partial offsets.
- **Expiry.** 2034-02-03.
- **Design-around.** Do not derive offset corner-rounding radii from an engagement-angle
  change limit in a chained way; or don't use offset-smoothing as the engagement control.

### 2.5 Autodesk / HSMWorks Adaptive, Delcam Vortex, Mastercam Dynamic Motion
- Autodesk's help describes Adaptive Clearing as guaranteeing a maximum tool load at all stages
  [S] (https://help.autodesk.com/cloudhelp/2019/ENU/Inventor-HSM/files/GUID-970298C7-2F2E-47E6-AD54-454DEC0BBD7C.htm).
  No Autodesk-assigned adaptive-clearing patent number was found in this session.
- Delcam Vortex (now Autodesk) was "patent pending" at launch 2013-2014, controlling
  engagement angle [S] (https://www.machinery.co.uk/content/news/new-toolpath-strategy-from-delcam-slashes-machining-times-by-up-to-60).
  Number not found; would now be Autodesk's; a 2013 filing would run to ~2033.
- Mastercam Dynamic Motion (CNC Software, since X4 / 2009): marketed as proprietary; no patent
  number found [S] (https://www.mastercam.com/dynamic).
- **Action.** Counsel should run assignee searches for "Autodesk, Inc.", "Delcam", "CNC
  Software" with CPC G05B19/4093, G05B19/4097, G05B2219/35, B23C3/00. Not done here.

### 2.6 OPEN MIND (hyperMILL MAXX) barrel / circle-segment cutters
Trade press: tangent-plane machining in hyperMILL 2016.1; barrel shapes supported since 2002;
co-developed with EMUGE [S] (https://mmsonline.com/products/open-mind-technologies,
https://moldmakingtechnology.com/articles/barrel-cutters-going-mainstream-). No Open Mind CAM
patent was found by web search; a Dec 2025 US 12,485,511 B2 (inventors Josef Koch, Peter
Brambs, Munich) on collision-free linking of engagement path segments in multi-axis machining
came up without an assignee [S]
(https://patentsgazette.uspto.gov/week48/OG/html/1541-1/US12485511-20251202.html); possibly
Open Mind, unverified. Cutter-geometry patents (tool makers like EMUGE) may exist separately.
**Action.** Espacenet/DPMA search for "OPEN MIND Technologies AG"; until then treat 5-axis
barrel strategies as unknown-risk and out of scope for a first release.

### 2.7 Trochoidal milling generally
Trochoidal slotting is decades old (HSM literature of the 1990s [K]), so the basic idea is
prior art; patents claim specific constructions. Found: Sona's channel-cavity trochoid patents
US 9,423,788 / 8,977,382 [S] (https://patents.justia.com/patent/8977382), US 7,237,989 on
curvature of milling paths from tool radius and contour [S], SolidCAM's trochoid-vs-spiral
selection (2.3), DP Technology (2.4). **Design-around.** A plain trochoid (circle + advance)
along a medial axis or offset centreline, with parameters set by the user, is the safest form.

### 2.8 Feed-rate optimisation (Vericut OptiPath / Force and others)
- CGTech OptiPath: simulation learns depth, width and angle per cut, splits moves, rewrites
  feeds to hit target removal rate / chip thickness without changing the path [S]
  (https://cgtech.com/products/product-line/100-optipath.html). Force: physics-based, keeps
  cutting force steady within limits on force, power and deflection [S]
  (https://mmsonline.com/articles/simulation-speeds-cycle-times-on-complex-parts). No CGTech
  patent number found.
- **GE US 5,828,574** [R] (https://patents.google.com/patent/US5828574A/en): offline
  simulation subtracts swept tool volumes from a solid model, computes material removal rate,
  average force and deflection per move, and picks the largest feed within force/deflection
  limits. **Expired 2017-05-19** - useful as dated prior art for exactly the OptiPath idea.
  US 4,833,617 (solid-model adaptive feed for end milling) also expired [S]
  (https://patents.google.com/patent/US4833617).
- **Design-around.** The core concept is free in the old GE/1980s form; avoid copying later
  vendor-specific refinements without a search.

### 2.9 CloudNC
CloudNC (London, 2015) sells CAM Assist (auto strategy/toolpaths, ML + physics feeds and speeds)
and reportedly "filed patent for groundbreaking AI in 2018" [S]
(https://www.joinef.com/companies/cloudnc/). No numbers found. **Action.** Search assignee
"CloudNC Ltd/Limited" (GB and WO first). Relevant if forgeCAD adds automatic strategy choice.

## 3. What this means for forgeCAD's design

1. Expired and usable as background: Surfware engagement-milling 2004 family except
   US 7,577,490 (expires 2026-12-17), GE adaptive feedrate (2017), 1980s solid-model feedrate.
2. Live and closest to an adaptive-clearing feature: **SolidCAM US 8,489,224 family (to
   2031-2033+)**, **Celeritive US 8,295,972 / 8,880,212 (to 2029-2030) and US 10,579,040 (to
   2032)**, **Truemill US 8,560,113 (to 2031)**, **DP Technology US 9,778,649 (to 2034)**.
3. A design that probably stays clear (counsel to confirm): cleared-area state plus a single
   target removed-area per step (FreeCAD-style), links by retract or by straight moves checked
   against cleared area (no transition-toolpath construction), offset or plain medial-axis
   spirals, user-set trochoids, feed optimisation of the GE 1997 kind.
4. Keep a dated record of every source and design decision (this file and opensource.md) as
   evidence of independent development from public prior art.
