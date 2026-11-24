# Unclaimed toolpath methods, and what the commercial strategies must be doing

2026-11-24. Research only; no code changed. Written for David and counsel.

**NOT legal advice and not a freedom-to-operate opinion.** This is an engineer's reading of public
pages. Patent status comes from Google Patents, which labels its own status and expiry fields
"an assumption, not a legal conclusion". A patent search that finds nothing is evidence, not proof:
Google's inventor and full-text search is noisy (see §A.1), applicants can file under names I
did not try, and CN/JP/KR filings in local script were not searched. Counsel decides.

Companion notes (do not repeat them here): [fto-adaptive.md](fto-adaptive.md) (claim elements of
the live adaptive-clearing families), [patents-fto.md](patents-fto.md) (broad survey; §3.1 lists
lapsed and expired items), [../patents.md](../patents.md), [roughing-linking.md](roughing-linking.md)
§2 (the method history), [feeds-speeds.md](feeds-speeds.md) §6, [surface-finish.md](surface-finish.md) §2.

**Marks.** [R] read this session (page or patent record fetched); [S] search snippet or listing
only; [K] background knowledge, not re-read; [D] derived here. "[R, fto]" means read this session
in a companion note, not at the primary source.

---

## 0. Findings in one page

1. **A general rule that frees most of the classic literature** [D, with K on US law]. A US patent
   that validly claims a method must have been filed no later than one year after the method was
   first published (the US grace period), and its term ends 20 years after filing, plus any term
   adjustment. So a method *exactly as published* before about late 2004 cannot be covered by a live
   US patent today except through unusual term adjustment. Applications filed before 2000-05-29 got
   no PTA of the modern kind [K]. One exception: applications filed before 1995-06-08 could take 17
   years from *grant*, so a long-pending ("submarine") application on a 1994-1995 publication could in
   principle still be live; none was found for the methods below [K, D]. This covers Suresh & Yang 1994, Spence & Altintas 1994, Lin & Koren
   1996, Stori & Wright 2000, Feng & Li 2002, Choy & Chan 2002-3, Bieterman & Sandstrom 2003,
   Kakino's 2001-2 work, Ibaraki 2003-4, Pateloup 2004, Ren/Yau/Lee 2004, and MATHSM (online 2004).
   It does not cover *later improvements* that someone else patented, and it does not apply outside
   the US. Searches were still run for each (below), as the brief asked.
2. **New patent finds not in the companion notes** [R]:
   - **Boeing US 6,591,158 B1** is the Bieterman & Sandstrom curvilinear (Laplace-morphed) spiral.
     It **lapsed for an unpaid 12th-year fee on 2015-07-08**. The method is free.
   - **Ibaraki/Yamaji JP 2005-050255 A** (priority 2003-07-31) claims Ibaraki's constant-engagement
     method (shape each earlier pass so the finishing pass's engagement stays at or below a set
     value). **Refused by the JPO in 2005; Japan only; no US/EP/WO family.** Free, and it is dated
     prior art against later "reshape the preceding path" claims (Delcam/Autodesk US 9,489,339 cl.18).
   - **Kakino et al. US 6,942,436 B2** (priority 2002-03-11; Mori Seiki, Yasda): spiral pocketing that
     holds the maximum chip thickness and arc of contact constant, and trochoidal slotting with the
     feed varied *within each turn* to hold force. **Expired for unpaid fees (2013).** Free.
   - **Kakino et al. US 6,772,039 B2** (priority 2001-12-04): per-tooth feed from a cutting-force
     response surface over chip thickness and engage angle, with linear feed ramps in transition
     zones before straight/arc changes. **Expired 2023.** Free.
   - **Kakino/Nakagawa JP 2007-272706 A** (2006): replace curved contour segments by ≥4th-order
     smooth curves so the load changes gradually. **Refused 2011; Japan only.** Free.
   - **EPFL US 2014/0297021 A1** (Aggarwal, Dhanik, Xirouchakis; priority 2011-02-11): corner-looping
     high-speed pocketing plus a genetic-algorithm choice of chatter-free parameters. **US abandoned
     2016; WO "ceased"; EP "withdrawn".** Free, and published prior art from 2012.
   - **Boeing US 9,164,503 B2** (Bolin, Freeman; priority 2012-07-13; **active to 2034-02-22**, 4th and
     8th fees paid): cut a pre-roughing slot along a medial-axis curve, with a circular recess at each
     end, then rough with a constant-radial-depth spiral outward from the slot. **This reads close to
     Fusion's "slot clearing" option and to any "medial-axis slot then spiral" design.** Not in
     fto-adaptive.md: it should be added to the claim matrix.
   - **The coordinator's nine unchecked applications** (§A.3): four were abandoned (Hexagon
     2021/0026327, Autodesk 2020/0201284, Siemens 2014/0316751; plus MERL 2013/0262066), one became
     an expired patent (Delcam US 6,832,876), and five became live patents: Surfware **US 9,317,030**
     (stepped helical entry, to 2031-11-10; not in the companion notes), Truemill 9,931,703, DP
     **US 10,877,463** (jerk-threshold multi-axis kinematics, to 2039), Siemens US 8,923,999
     (volumetric cut planning; claims not read) and Autodesk US 12,427,615 (machine wear).
3. **Autodesk US 10,108,172 (Abrahamsen)**: both independent claims (1, 12) need a polygon with a
   polygonal hole and a Voronoi diagram modified to have exactly one cycle, plus a "wave" model.
   Claim 10 (dependent) treats a zero-area skeleton as the hole for simply connected pockets. Live to
   2036-12-18, 8th-year fee paid 2026-03-19 [R]. Claimed-live; Held & Spielberger's 2009 spiral is a
   different construction and appears unclaimed.
4. **Top-ranked unclaimed methods for toolpath quality** (§A.4): (1) Ibaraki-style engagement-capped
   offsets with the cap certified on the exact stock, plus the Kakino/GE per-move feed; (2) the
   Bieterman-Sandstrom morphed spiral for near-convex pockets; (3) Held-Spielberger bounded-width
   spirals with Voronoi decomposition; (4) iso-scallop (Suresh-Yang / Feng-Li / Tournier-Duc) and
   heat-method geodesic finishing; (5) local corner smoothing (Zhao-Zhu-Ding, Tulsyan-Altintas,
   Beudaert) inside the certified tolerance; (6) Choy-Chan / EPFL corner looping for corners.
5. **Main inferences about the commercial products** (Part B): Fusion Adaptive is a stock-aware
   stepping controller that caps a *radial width* (not an angle), with constant feed; Mastercam
   Dynamic caps engagement by path and gets "constant chip" from a per-operation thinning factor;
   hyperMILL MAXX roughing is Celeritive's VoluMill engine (OPEN MIND says so), so it is an MRR cap
   with feed as the actuator; PowerMill Vortex matches the shape of Delcam/Autodesk US 9,489,339
   (live to 2035); the iMachining Wizard is a mechanistic power model inverted per segment, with
   levels 2-7 interpolated between two computed sets.

---

# Part A. Methods that appear to be free to use

## A.1 How the searches were done

- Google Patents' public JSON query endpoint (`patents.google.com/xhr/query`, the URL the search
  page itself calls), with a plain `urllib` client, default user agent, 4-6 s between calls, and
  waits after HTTP 503. Individual patent pages were fetched the same way, and some through the
  WebFetch tool. No sign-in, no UA change.
- Queries were inventor names (with and without topic words) and topic phrases. The listing shows
  the top 8 hits per query; I record the hit count and whether any top hit was a toolpath patent.
- **Limits.** Inventor search matches any person with that surname (e.g. 1,831 "Barton" hits), and
  `q=` phrase terms matched loosely (an `"iso-scallop"` query returned semiconductor and dental
  patents). So "no relevant hit in the top 8" is weak evidence for common names. Chinese and
  Japanese inventors were searched only in Latin script. Espacenet was not used.
- The 21-year rule of §0.1 is the stronger argument for anything published before late 2004.

## A.2 Candidate by candidate

Verdicts: **unclaimed** (no patent found and/or the 21-year rule applies), **claimed-expired**,
**claimed-live**, **UNKNOWN**.

| # | Method (one line) | Date | Searches done → result | Status | Verdict |
|---|---|---|---|---|---|
| 1 | **Stori & Wright**: spiral-in path for convex pockets that tracks engagement, curvature and entry/exit angle (*J. Manuf. Syst.* 19(3)) | 2000 | inventor "Jerome Stori" + milling/machining/tool path → 0; inventor "Stori" → 37, none machining in top 8; inventor "Wright" + "constant engagement" → 7, none relevant [R] | none found; 21-year rule | **unclaimed** |
| 2 | **Ibaraki et al.** constant-engagement offsets: Algorithm I moves offset points along the normal; Algorithm II reshapes the *previous* pass so the next sees constant φ | JP filing 2003; papers 2004-2006 | inventor "Soichi Ibaraki" (+tool path/milling) → 6; "Ibaraki Soichi" → 10. Hit: **JP 2005-050255 A** (Yamaji, Ibaraki; Manufacturing Technology Institute), claims exactly this (each preceding stage generated so the finishing engagement stays ≤ a set value) [R] | JPO notice of refusal 2005-07, **decision of refusal 2005-11-02**; Google still shows "Pending"; no foreign family [R] | **unclaimed** (refused; JP-only) |
| 3 | **Kakino et al.** canned cycles: spiral pocket at constant max chip thickness and arc of contact; trochoidal slot with feed varied within each turn; helical boring | priority 2002-03-11 | inventor "Kakino" + "tool path" → 13 hits; **US 6,942,436 B2** (Mori Seiki, Yasda, Graphic Products) [R] | **Expired - fee related**; USPTO "expired due to nonpayment" entries 2013-10 [R] | **claimed-expired** |
| 4 | **Kakino et al.** feed per tooth from a force response surface (chip thickness, engage angle, axial depth), linear feed ramps before segment changes | priority 2001-12-04 | same query → **US 6,772,039 B2** [R] | Expired - lifetime, 2023-01-30 [R] | **claimed-expired** |
| 5 | **Kakino/Nakagawa**: approximate curved contour segments by ≥4th-order curves so load changes gradually | 2006 | same query → **JP 2007-272706 A** (FANUC, Mori Seiki) [R] | decision of refusal 2011-04-05; JP only [R] | **unclaimed** |
| 6 | **Elber, Cohen, Drake MATHSM**: C¹ path of maximal inscribed circles along the medial axis joined by bitangents/biarcs | online 2004, *CAD* 2005 | inventor "Gershon Elber" → 18, none machining (Technion AM/truss/volumetric); "Elaine Cohen" + machining → 2: US 5,726,896 (Utah, 1995, CNC spline interpolation, expired by age) and a 2016 trim-curve patent [R] | none on MATHSM; 21-year rule | **unclaimed** (but see #26 for medial-axis slot + spiral) |
| 7 | **Choy & Chan** corner looping: bow-like loops appended at concave corners remove corner stock progressively | 2002-2003 | inventor "Choy" + pocket/milling → 0; "Choy" + corner → 113, none machining in top 8 (Nvidia, Avago) [R]; web search for an HKU patent → none [S] | none found; 21-year rule | **unclaimed** |
| 8 | **EPFL corner looping + GA parameter choice** (Aggarwal, Dhanik, Xirouchakis) | priority 2011-02-11 | phrase "corner looping" → 7, incl. **US 2014/0297021 A1** [R]; also CN 103042262 B (NUAA, 2013) and CN 104865892 B (Zhejiang, 2015) [S, listing only] | US **abandoned** 2016-11-29 (no response to office action); WO 2012/107594 "ceased"; EP 2673678 "withdrawn" [R]. CN items: foreign family not checked | **unclaimed** in US (published, abandoned) |
| 9 | **Bieterman & Sandstrom** curvilinear spiral: level sets of an elliptic PDE (Laplace) morphing from a near-circle at the centre to the wall | priority 2000-06-09; paper 2003 | inventor "Bieterman" → 1; "Sandstrom" + Boeing → same: **US 6,591,158 B1** "Methods and apparatus for defining a low-curvature tool path" (Boeing) [R] | 4th and 8th fees paid; **LAPS 2015-07-08, FP effective 2015-07-08**; Google adjusted expiry 2021-10-02 anyway; FR 2809978 B1 family, status not read [R] | **claimed-expired** (fee lapse) |
| 10 | **Held & Spielberger** spiral: non-self-intersecting, G¹ (G² smoothed) spiral with cutting width bounded above and away from zero; 2014 Voronoi decomposition for islands | 2009, 2014, 2018 | inventor "Martin Held" + spiral/pocket/machining → 9, "Held" + tool → 5, "Held" + spiral → 324 (other Helds): none machining in top 8; "Spielberger" → 418, none machining in top 8 [R] | none found | **unclaimed** (weak search for a common name; see #11) |
| 11 | **Abrahamsen** spiral from a modified Voronoi diagram with one cycle and a "wave" model | priority 2014-10-13 | inventor "Mikkel Abrahamsen" → 1: **US 10,108,172 B2** (Autodesk) [R] | active to **2036-12-18**; 4th fee 2022-03-22, 8th fee 2026-03-19 [R]. Independent cl. 1, 12 need a polygon *with a polygonal hole*; cl. 10 (dependent) makes a zero-area skeleton the hole [R] | **claimed-live** |
| 12 | **Abrahamsen & Thorup** "How to cut corners": the unique largest sub-region whose boundary never turns more sharply than a unit circle, O(n log n) simply connected | arXiv 2016 | same inventor query → only #11 [R] | none found | **unclaimed** |
| 13 | **Wang (GE)** solid-model adaptive feed: per-move MRR → power → force → deflection, feed set within limits | filed 1987-08-14 | not re-searched; US 4,833,617 read by the companion notes [R, fto; feeds-speeds §6] | lapsed 1993 (4th-year fee) [R, fto] | **claimed-expired** |
| 14 | **Spence & Altintas** CSG engagement + mechanistic force, inverse feed under force/torque/error limits | 1994 | inventor "Yusuf Altintas" → 1 (WO 2019/191848, tool vibration, UBC); "Altintas" + corner/toolpath/trajectory → 162, top 8 all a different Altintas (Toyota); "Spence" + milling/feedrate → 305, none machining in top 8 [R] | none found; 21-year rule | **unclaimed** |
| 15 | **Erdim, Lazoglu, Ozturk** MRR-based vs force-based feed scheduling for ball-end 3D milling | 2006 | inventor "Lazoglu" → 17, none machining (implants, pumps, damper) ; "Erdim" → 14: Erdim's later MERL patents US 9,892,215 (feed by engagement-function segments, **live to 2035**), US 8,935,138, US 9,513,623, US 2013/0262066, US 9,575,486 (lapsed) [R] | the 2006 Koç-university method: none found | **unclaimed** for the 2006 method; MERL '215 is a separate live claim (fto §1.8) |
| 16 | **Suresh & Yang** constant scallop height (iso-scallop) for free-form surfaces (*J. Eng. Ind.* 116(2)) | 1994 | "Krishnan Suresh" + scallop/tool path → 10, none toolpath; "Suresh" + scallop → 12, none [R]. The 1994 author's identity with Krishnan Suresh is not established (UNKNOWN) | 21-year rule | **unclaimed** |
| 17 | **Lin & Koren** efficient free-form tool-path planning (iso-scallop family) | 1996 | "Yoram Koren" + tool path → 8, all reconfigurable manufacturing; "Koren" + scallop → 2 unrelated [R] | 21-year rule | **unclaimed** |
| 18 | **Feng & Li** constant scallop-height tool paths for 3-axis sculptured surfaces (*CAD* 34(9)) [K citation] | 2002 | inventor "Hsi-Yung Feng" → 1 (UBC, 2016, simulation geometry); "Feng" + scallop → 93, top 8 unrelated [R] | 21-year rule | **unclaimed** |
| 19 | **Tournier & Duc** iso-scallop via a "machining surface" (*IJAMT* 19:318-324) | 2002 | inventor "Christophe Tournier" → 12, all exhaust/AM; "Lavernhe" → 28, AM and filters [R] | 21-year rule | **unclaimed** |
| 20 | **Tulsyan & Altintas** local smoothing of 5-axis linear paths by quintic/septic micro-splines, C³ | 2015 | inventor "Sneha Tulsyan" → 0 [R]; "Altintas" as above | none found | **unclaimed** |
| 21 | **Zhao, Zhu, Ding** real-time look-ahead with curvature-continuous 5-control-point B-spline corner transitions (*IJMTM* 65) | 2013 | "Han Ding" + corner + interpolation → 1: US 11,200,351 (HUST 2019, robot path curve, different) [R]; "Huan Zhao" + interpolation → 0 [R]. CN filings in Chinese script: **not searched** | no US patent found | **unclaimed in US**; CN **UNKNOWN** |
| 22 | **Beudaert, Lavernhe, Tournier** 5-axis local corner rounding of G1 discontinuities | 2013 | "Xavier Beudaert" → 4 (chatter, threading, dampers); "Lavernhe", "Christophe Tournier" → AM only [R] | none found | **unclaimed** |
| 23 | **Pateloup, Duc, Ray** corner rounding of pocket offsets for HSM; B-spline approximation (2010) | 2004, 2010 | "Vincent Pateloup" → 7, ceramics/AM; "Emmanuel Duc" → 3, brake switches and a pump [R] | none found; 21-year rule for 2004 | **unclaimed** (but DP US 9,778,649 claims a *chained engagement-derived* corner radius, live to 2034: fto §1.4) |
| 24 | **Rauch, Duc, Hascoët** trochoid generation under process constraints, trochoid vs plunge | 2007-2009 | "Hascoet" + milling → 13, all AM/repair in top 8; "Hascoet" + trochoidal → 0; "Matthieu Rauch" → 1 (repair) [R] | none found | **unclaimed** (plain trochoids; Kakino's trochoid with in-turn feed variation is expired, #3) |
| 25 | **Freesteel / libactp** stepwise adaptive clearing against a cleared contour | headers 2004; public by 2009 | "Julian Todd" + machining/tool path → 0 relevant (one 1986 regenerator patent); "Martin Dunschen" → 0 [R] | none found; GPL code (read only, never port) | **unclaimed** |
| 26 | **Medial-axis pre-roughing slot, then outward constant-radial-depth spiral** | priority 2012-07-13 | phrase "medial axis" + pocket + spiral → 142, incl. EP 2685332 B1 → **US 9,164,503 B2** (Boeing; Bolin, Freeman) [R] | **active to 2034-02-22**; fees paid 2019, 2023 [R]. All three independents need: medial-axis curve → pre-roughing slot cut with a circular recess at *each* end, then a constant-radial-depth roughing path starting inside the slot and spiralling outward [R] | **claimed-live** |
| 27 | **Ren, Yau, Lee** contraction-tool clean-up (pencil) paths on polyhedral models (*Comput. Ind.* 54) | 2004 | "Hong-Tzong Yau" → 17, dental; "Yuan-Shin Lee" + tool path → 4 unrelated; phrase pencil + "tool path" + bitangent → 0 [R] | 21-year rule | **unclaimed** |
| 28 | **Kimmel & Sethian** fast-marching geodesics; **Crane et al.** heat method, as used by Zou et al. 2020 for near-iso-scallop paths | 1998; 2013/2017; 2020 | "Ron Kimmel" + tool path/machining → 50, none machining in top 8; "Kimmel" + "tool path" → 0; "heat method" + "tool path" → 3 unrelated; geodesic + "tool path" + scallop → 8: none US toolpath, two recent CN (NUAA 2024 five-axis constant residual height; CAS 2025 heat-transfer path planning) [R] | none in US | **unclaimed** in US; the two CN items are **UNKNOWN** (no family check) |
| 29 | **Bartoň et al.** curvature-adapted 5-axis motion planning by second-order line contact (*ACM TOG* 40(4)) | 2021 | "Barton" + milling → 1,831 (noise), "Michael Barton" + milling → 36, none machining in top 8; "Pottmann" + milling → other Pottmanns [R]. "Bartoň" with diacritic and TU Wien/KAUST/BCAM as assignee: **not searched** | not established | **UNKNOWN** (and 5-axis, out of scope for now) |
| 30 | **Steep-and-shallow** finishing split by slope threshold | 1990s practice [K] | steep + shallow + "tool path" + finishing → 42, top 8 unrelated; one 2024 CN on region integration (CN 119828600 A) seen in another query [S] | basic split: old practice | **unclaimed** for the basic split; CN refinement UNKNOWN |

Already established elsewhere and not repeated: the Surfware/Truemill 2004 family (expired except
US 7,577,490, two-arc construction, to about 2026-12-17), MTU US 7,237,989 (expired 2024), GE
US 5,828,574 (expired 2017), Surfware US 6,704,611 (expired 2022), MERL US 9,575,486 (lapsed 2021)
[R, fto §0; patents-fto §3.1].

## A.3 Published US applications the patent agent did not check (coordinator's list)

patents-fto.md §3.1 listed nine published applications whose fate it did not check. All read on
Google Patents 2026-11-24 [R]:

| Application | Subject | Fate | What it means |
|---|---|---|---|
| US 2011/0150592 A1 (Surfware; Diehl, Patterson; priority 2010-12-20) | material entry tool path | **granted as US 9,317,030 B2** (2016-04-19), active, adjusted expiry **2031-11-10**, 4th and 8th fees paid (small entity). Cl.1: two sets of descending circling segments about one vertical axis, the second starting near the first's depth and going deeper, leaving a ledge wider than one chip. Parent of US 9,931,703 | **claimed-live**: stepped-diameter helical entry. A single helix, or a ramp, avoids it [D] |
| US 2016/0107246 A1 (Truemill) | starting hole | **granted as US 9,931,703 B2** (2018-04-03), 8th fee paid 2025-10-03 | already in patents-fto (to 2031-06-28) |
| US 2019/0171189 A1 (DP; Petruzzi, Sona; priority 2017-12-06) | toolpath virtualization and optimization | **granted as US 10,877,463 B2** (2020-12-29), active, adjusted expiry 2039-01-03; EP 3721304 B1, JP 7011064 B2, CN 111819504 B. Cl.1: derive pivot, velocity, acceleration and jerk from tool-end position, tool length and tool-axis orientation; check against machine/tool capability; adjust to keep jerk under a threshold | **claimed-live**, multi-axis kinematics. A 3-axis jerk-limited feed plan does not obviously involve pivot/axis-orientation elements; check before 5-axis [D] |
| US 2021/0026327 A1 (Hexagon; Thenoz) | predictive machining engine | **abandoned** 2023-09-19 (no response after final rejection); WO 2021/021689; EP 4004662 A1 and CN 114245884 B status not read | US free (published 2021: prior art); EP/CN UNKNOWN |
| US 2020/0201284 A1 (Autodesk; Edwards, Rogers) | automatic CAM instruction generation | **abandoned** 2023-11-27 after a Board of Appeals decision | free in US; published prior art |
| US 2022/0080545 A1 (Autodesk; Sanders, Abu Bakar) | evening out machine-component wear | **granted as US 12,427,615 B2** (2025-09-30); continuation US 2026/0070176 A1 pending | **claimed-live**; irrelevant unless forgeCAD varies paths to spread machine wear |
| US 2013/0060368 A1 (Siemens PLM; Balkenende et al.) | volumetric cut planning | **granted as US 8,923,999 B2** (2014-12-30), 4th and 8th fees paid; EP 2753993 B1, JP 6150805 B2, CN 103814334 B | **claimed-live**; claim text not read: counsel to check against stock-aware level planning |
| US 2014/0316751 A1 (Siemens PLM; Nanson, Collins, Zhou) | swept volume of a spun tool | **abandoned** 2015-12-28 (no response to office action); WO 2014/172364 | free in US |
| US 2002/0119020 A1 (Delcam; Hobbs) | machine tools | **granted as US 6,832,876 B2** (2004-12-21), "Expired - Lifetime"; GB 2374562 B | **claimed-expired** |

Also checked while there: MERL **US 2013/0262066 A1** (Erdim, Sullivan; engagement surfaces in
machining simulation) is **abandoned** (2015-10-13); JP 2013-206459 A status not read [R].

## A.4 Ranking the unclaimed methods by toolpath quality

Ranked by expected effect on cycle time, tool-load uniformity and finish for forgeCAD's users
(mostly 3-axis, aluminium and steel, small shops), given what forgeCAD already has (exact line/arc
regions, exact stock, a certifier). All reasons are [D] from the cited results unless marked.

1. **Engagement-capped offsets on the exact cleared region (Ibaraki Algorithm II + Stori & Wright),
   with per-move feed from the real engagement (Kakino US 6,772,039 / GE US 4,833,617 / Spence &
   Altintas).** Biggest roughing gain available without touching live adaptive claims: the corner
   spike that today reaches 127-180° (roughing-linking §1) is capped by reshaping the *inner*
   passes, and the feed then holds the chip at h* on every move. Ibaraki measured φ held at 11.5°
   instead of 19° and force variation within 17 N [R, roughing-linking §2.1]. Kakino's transition-zone
   ramps solve the servo-lag problem Ibaraki reported. Risk note: Delcam/Autodesk US 9,489,339 cl.18
   claims *increasing the radius of the preceding path's curve* when predicted engagement exceeds a
   limit (with offset subsets). JP 2005-050255 (2003) is dated prior art for that idea; counsel should
   compare. A design that reshapes by *moving points along the normal* (Algorithm I) or caps by the
   stepping controller avoids the "radius of preceding curve" wording [D].
2. **Bieterman-Sandstrom morphed spiral** (US 6,591,158 lapsed). One continuous path, no step-over
   marks, low curvature at the centre, curvature growing only near the wall; Boeing reported about
   30 % less time in aluminium and about twice the tool life in titanium [S, roughing-linking §2.1].
   Limited to near-convex pockets: combine with #3 for others. forgeCAD can solve the Laplace problem
   on the exact region and certify the engagement of the result (row 1 of the README plan).
3. **Held-Spielberger spiral with Voronoi decomposition.** Bounded cutting width everywhere, no
   self-intersection, G² after smoothing, works with islands by decomposition; Held reports shorter
   paths and less curvature and engagement variation [R, roughing-linking §2.1]. Lower cycle time
   than offsets because there are no links inside a sub-pocket. Keep clear of Abrahamsen '172
   (one-cycle modified VD + wave) and Celeritive '040 (synthetic VD branches) [D].
4. **Iso-scallop finishing (Suresh-Yang, Lin-Koren, Feng-Li, Tournier-Duc) and heat-method geodesic
   parallels (Zou et al. 2020 with Crane's heat method).** The shortest finishing path for a scallop
   target; with forgeCAD's exact curvature the stepover is exact rather than mesh-estimated
   (surface-finish §1.4). The size of the gain over constant-stepover passes was not read in any
   source this session: UNKNOWN, to be measured on the B5 hemisphere and slope set.
5. **Local corner smoothing (Zhao-Zhu-Ding 2013; Tulsyan-Altintas 2015; Beudaert 2013) and Pateloup
   corner rounding, plus Kakino JP 2007-272706 higher-order contour blends.** Removes the feed dips
   and dwell marks at G1 corners; the reachable feed on a radius ρ is about √(a_max·ρ) (roughing-linking
   §1.1). forgeCAD's version: blend only inside the certified tolerance band, then re-certify (S4/S6 in
   surface-finish §8).
6. **Corner looping (Choy-Chan 2003; EPFL 2011, abandoned).** A cheap fix for corners of plain
   offsets: one loop per concave corner caps engagement without reshaping the whole path. Slightly
   longer path than #1; simpler. Avoid Truemill US 8,560,113 (series of corner arcs joined by short
   transitions, live to 2031) by using one loop per corner per pass and checking against its claim [D].
7. **MATHSM medial-axis circles** for long narrow regions: lowest radial acceleration, C¹. Good in
   slots; wasteful in round regions. Do **not** combine it as "cut a medial-axis slot with end
   recesses, then spiral out at constant radial depth": that is Boeing US 9,164,503 (live).
8. **Plain trochoids (Rauch-Duc-Hascoët) with the in-turn feed variation of Kakino US 6,942,436.**
   For slots narrower than about 2× tool diameter. Low MRR but the safest load; the feed variation
   within the turn is the expired Kakino idea.
9. **Abrahamsen-Thorup bounded-curvature region** as the exact tool for "minimum cutting radius":
   not a path by itself, but it makes #1-#3 better near walls with a proof of what is left as rest.

---

# Part B. What the commercial strategies must be doing (inference from public material)

Method: public help pages, vendor pages, trade press and patents only. Every inference is [D]
and lists the evidence it rests on. No software was decompiled or run.

## B.1 Fusion / HSMWorks Adaptive Clearing

**Public evidence.**
- "Optimal load" is the maximum radial engagement, entered as a distance; the stepover varies at or
  below it; help suggests 10-40 % D; one user reports a 40 % D default [R, fusion-benchmark §2].
  Autodesk help claims it "guarantees a maximum tool load at all stages" [R, patents.md §2.5 snippet;
  fusion-benchmark].
- Minimum cutting radius blends all inside corners and leaves rest material; optional slot clearing
  (a centre slot, then spiral out) "by default ... spiral"; stay-down level 0-100 % where more costs
  more calculation time; max stay-down distance; lift height; no-engagement feedrate; Feed
  optimization only slows at sharp corners; machine-shallow-areas inserts Z levels [R, fusion-benchmark].
- Lineage: libactp describes itself as the GPL algorithm shown in Freesteel; its roughing code
  changes direction "according to the amount of material cutting" with a cleared-contour check
  [R, fto §2]; forum claims link Freesteel to HSMWorks Adaptive [S].
- Output: Autodesk staff describe adaptive paths as an extreme case of linearisation; arcs appear only
  with tight tolerance, and loose tolerance can produce spiral arcs that need a post option [S, forum].
- No Autodesk patent on Adaptive Clearing was found by the companion notes; Autodesk holds '172
  (Abrahamsen) and '339 (Delcam) [R, fto].

**Inferences.**
- [D1] The planner is a *stepping controller against an in-process 2D stock per Z level*: at each step
  it measures the material the tool would remove and turns to hold the radial width at or below the
  cap. Rests on: the libactp lineage, "stock-aware" behaviour, and the guarantee wording. It cannot be
  a pure offset method, because offsets cannot hold any cap in corners (roughing-linking §1).
- [D2] The capped quantity is a radial *width* (distance), not the engagement angle. Rests on: the
  parameter's unit. On a concave arc the same width gives a larger angle (feeds-speeds §2.3), so the
  "guarantee" is a width guarantee; whether Fusion converts width to angle by path curvature is
  UNKNOWN.
- [D3] With feed constant (no engagement-based feed), cycle time is only good if the width stays
  *near* the cap: the controller targets the cap from below, and the time lost is in the places where
  it cannot (corners, entries). Rests on: Feed optimization being geometric only.
- [D4] Minimum cutting radius exists because an unconstrained stepping controller produces tiny
  radii where material piles up in corners; Fusion clips the region by a radius and leaves rest.
  This is what Abrahamsen-Thorup computes exactly (#12).
- [D5] Stay-down level is a search budget for link paths through the cleared region (more = more
  calc time, fewer retracts). Slot clearing exists because spiral-out from one start point links badly
  in long pockets.
- [D6] The path is computed as a dense polyline and arcs are fitted afterwards (or not).

**How forgeCAD reproduces and beats it.** A stepping controller on the *exact* cleared region
(line/arc), capping the engagement **angle** (or both width and angle), certified per op; per-move
feed from the chip (Fusion has none); links proved inside the cleared region; arcs emitted natively
from the exact construction, not fitted; deterministic, with no time budget.
**Patent flags.** The single-cap stepping controller is fto design A. A min/max band reads toward
SolidCAM '224/'704; asymmetric spiral loops with cyclic engagement toward '645/'861/'445; link
*transition areas* toward Celeritive '972/'212. **New:** Fusion-style *slot clearing* along a medial
axis followed by an outward spiral is close to **Boeing US 9,164,503** (live to 2034); forgeCAD's
slot option should not cut circular recesses at both slot ends at set depths, or should not derive the
slot from the medial axis, or should not follow it with a constant-radial-depth outward spiral, as
counsel advises [D].

## B.2 Mastercam Dynamic Motion

**Public evidence.**
- "Maintain a constant chip load" by adapting motion to stock; "material-aware", "stock-aware";
  claims 25-75 % cycle-time cuts and "often up to 3-5×" tool life; "proprietary"; no patent named [R,
  mastercam.com/dynamic].
- The blog says Dynamic Motion "continuously adjusts the toolpath trajectory to maintain consistent
  cutter engagement" [R].
- Mastercam 2024 added a **Maximum engagement** option on Dynamic Mill, Face and OptiRough: it
  "moves the tool towards the entered Stepover as quickly as possible" and "produces more nominal
  chips and minimizes the number of small chips" [R, my.mastercam.com].
- Linking: Gap size (distance or % D); moves shorter than the gap stay down with a Micro lift distance
  and its own feed; longer moves retract under rules (never / boundary / distance / OR / AND) [R,
  caminstructor blog]. Back feed rate [S].
- RCTF (radial chip thinning factor) is a per-operation feed scale from the stepover [R, fusion-benchmark §3].
- CNC Software's one live patent found is an architecture patent (US 12,663,778) [R, fto §1.9].

**Inferences.**
- [D7] "Constant chip load" is engagement-capped *path* plus a *per-operation* thinning factor, not a
  per-move feed. Rests on: RCTF being per-op and no feed-modulation parameter being documented. So the
  chip is constant only where engagement equals the nominal stepover.
- [D8] Before 2024 the controller let engagement sit well below the stepover over long stretches
  (many "small chips"); the new option makes it close the gap quickly. So the default controller is a
  one-sided cap with a slow approach.
- [D9] Linking is a fixed distance rule, not a time estimate. Rests on: gap size and retract rules.

**forgeCAD.** Beat it with a true per-move chip (the chip *is* constant, not nominal) and with a
certified cap. The fixed-rule linking is public and safe (fto design B); a *time-estimated* choice
between clearance-plane and stay-down links is in SolidCAM's pending US 2025/0284264 [R, fto §1.1].

## B.3 hyperMILL MAXX Machining (high-performance roughing)

**Public evidence.**
- OPEN MIND: "Uses proven HPC technology from Celeritive"; spiral and trochoidal paths; full cuts and
  abrupt direction changes prevented; feed adjusted along the path by local conditions; min and max
  feed for approach, plunge and in-plane cutting; maximum step-down, then bottom-up intermediate steps
  [R, openmind-tech.com].
- 2010: hyperMAXX "powered by Celeritive Technologies' Volumill toolpath engine"; feed and infeed
  adjusted to control the volume removed; constant radial depth; feeds adjusted at corners, entries,
  connections; side-cutting-only option up to 100 % D lateral infeed; 50 %+ savings "typical" (vendor)
  [R, American Machinist].
- VoluMill manual: MRR never above that of a straight cut at the set Z step, width and feed; min
  toolpath radius default 45 % D; repositions inside cleared area lift by a floor clearance on a helix
  at high feed [R, roughing-linking §2.3].

**Inferences.**
- [D10] The controlled quantity is MRR, with **feed as the actuator**: v_f = Q*/(a_e·a_p) clipped to
  [F_min, F_max]. The min/max feed fields are the clip. Rests on: the 2010 "volume" wording, the
  VoluMill manual, the min/max feed fields.
- [D11] Links are VoluMill's; if VoluMill's links are the claimed transition-area construction
  (Celeritive '972/'212, to 2029-2030), OPEN MIND is presumably licensed. **Do not imitate VoluMill
  linking.**
- [D12] "Max step-down then bottom-up intermediate steps" is a step-up rest scheme; SolidCAM '282
  claims a bounded step-up (between an inner offset surface and an outer scallop surface, step count
  minimizing material) [R, fto §1.1]. Fixed intermediate heights set by the user are safer [D].

**forgeCAD.** MRR-capped feed is the GE 1987 idea (expired) and Kakino 2001 (expired); but an MRR cap
ignores chip size (feeds-speeds §6 table), so use the chip-constant feed capped by power: better tool
life than VoluMill's rule in thin cuts.

## B.4 PowerMill / FeatureCAM Vortex (Delcam, now Autodesk)

**Public evidence.**
- Engagement "never exceeds, by more than 15 %, that produced by a straight line cut with a given
  stepover"; near the maximum it "changes to a trochoidal path"; deeper cuts allowed; Step Cutting
  limits terraces; output "Tolerance and keep arcs" [R, PowerMill help 2022].
- Feed: use the manufacturer's side-cut feed as the *minimum*; non-cutting default 2× cutting feed;
  the target radius increases with the feed; minimum point spacing from the model [R, PowerMill help].
  MachineDNA sets minimum radius and point spacing from the feed [S].
- Delcam US 9,489,339 (Mackman, priority 2011-12-12, to 2035-03-07): offset subsets with a larger
  first stepover and backfill paths; or increase the radius of the *preceding* path's curve where the
  predicted engagement exceeds a set angle. fto-adaptive.md believes this is Vortex [K there].
- Vortex was "patent pending" at launch 2013-14 [S, patents.md].

**Inferences.**
- [D13] The cap is φ_lim = 1.15·φ_straight(a_e) with φ_straight = arccos(1 − 2a_e/D). Vortex computes
  predicted engagement on curved portions of an offset, and where it exceeds φ_lim it enlarges the
  earlier pass's radius or falls back to trochoids/backfill. Rests on: the 15 % rule, the trochoid
  fallback, the '339 claim wording. This is the '339 construction: **live to 2035**.
- [D14] "Target radius increases with feed" is the centripetal limit ρ ≥ v²/a_max; "minimum point
  spacing" is the block-rate limit Δs ≥ v·t_block. MachineDNA must measure a_max and the block time.
  Both are free physics.

**forgeCAD.** Use the 15 % idea only as a *parameter* of a different construction (stepping
controller or Ibaraki normal moves), never "enlarge the preceding curve when predicted engagement
exceeds a limit". Beat it with per-axis acceleration and jerk (Ruckig bound) instead of one radius rule.

## B.5 SolidCAM iMachining Technology Wizard

**Public evidence.**
- The Wizard gives "synchronized values of feed rate, spindle speed, axial depth of cut, cutting
  angles and chip thickness" for "every segment"; inputs are stock and tool material properties and
  machine limits; an 8-level slider for fixture, holder and machine rigidity; "patented" [R,
  solidcam.com wizard page].
- Overview: cutting angle kept "between defined minimum and maximum values"; feed adjusted as the
  width of cut changes to hold spindle load; when the angle is reduced, feed goes up; level 1 = minimum
  MRR, level 8 = maximum, **levels 2-7 interpolated**; 70 % and 5× claims; morphing spiral and moating
  patented [R, solidcam.com overview]. A SolidCAM blog gives an angle range of about 10°-80° [S].
- Claims: '224 cl.1 (min and max angle, gradual variation); '607 cl.1 (available spindle power →
  automatic max depth and width, with an asymmetric spiral) [R, fto §1.1].

**Inferences.**
- [D15] The Wizard inverts a mechanistic power model per segment: from a material table it picks a
  target chip h*, sets f_z(φ) = h*/sin φ (φ ≤ 90°), and chooses a_p, a_e and the angle band so that
  P = k_c(h_m)·a_p·a_e·v_f stays at or below the machine's power times a level factor. Rests on: the
  listed inputs and outputs, "feed follows width", "angle down → feed up".
- [D16] Only levels 1 and 8 are computed; 2-7 are linear interpolations of the two parameter sets.
  Rests on: "interpolated".
- [D17] "Constant spindle load" and "constant chip" cannot both hold across a wide angle band. With
  the peak chip h held constant, f_z = h/sin φ and a_e = r(1 − cos φ), so MRR per unit a_p scales as
  a_e·v_f ∝ (1 − cos φ)/sin φ = tan(φ/2): from 10° to 80° that is tan 5° = 0.087 to tan 40° = 0.84,
  a factor of about 9.6 [D, closed form]. So either the band is much narrower than 10°-80° in a given
  cut, or the spindle load is not constant, or the chip is not. Which one SolidCAM gives up is UNKNOWN;
  the marketing claims both.

**forgeCAD.** A per-move chip-constant feed with power, torque and deflection caps at the interval
upper bounds (feeds-speeds §6) does what the Wizard does, with a proof of the chip bound. **Patent
flags:** automatic depth and width *from spindle power* is '607 element (2) (with an asymmetric
spiral); a min/max angle band is '224; the Wizard itself is called patented and its number is
UNKNOWN. Keep power as a *check* that refuses or warns, not as an automatic choice of a_p/a_e, until
counsel reviews.

## B.6 Summary of inferred methods against live claims

| inferred method | product | live claim it may read on | forgeCAD choice |
|---|---|---|---|
| stepping controller, single width cap, 2D stock | Fusion | none found if single target (fto §3 A) | angle cap, exact stock, certified |
| medial-axis centre slot, then outward spiral | Fusion slot clearing | **Boeing US 9,164,503 (2034)** | counsel; avoid end recesses + MA slot + constant-width spiral together |
| per-op thinning factor, gap-size linking | Mastercam | none found | per-move chip; fixed-rule links |
| MRR cap, feed as actuator, VoluMill links | hyperMILL MAXX | Celeritive '972/'212 (links) | GE/Kakino feed; own links |
| 15 % engagement cap via enlarged preceding curve | Vortex | **Delcam/Autodesk '339 cl.18 (2035)** | stepping or normal-move reshaping; JP 2005-050255 as prior art |
| power → depth/width, min/max angle band | iMachining | **SolidCAM '607, '224 (2031)** | power as check, single cap |

---

## References

Patents and applications (Google Patents pages, read 2026-11-24 unless marked):
1. US 6,591,158 B1, Bieterman & Sandstrom, Boeing. https://patents.google.com/patent/US6591158B1/en [R]
2. US 9,164,503 B2, Bolin & Freeman, Boeing; family EP 2685332, JP 6334105, CA 2818645, CN 103543696. https://patents.google.com/patent/US9164503B2/en [R]
3. US 10,108,172 B2, Abrahamsen, Autodesk. https://patents.google.com/patent/US10108172B2/en [R]
4. US 2014/0297021 A1, Aggarwal, Dhanik, Xirouchakis, EPFL; WO 2012/107594; EP 2673678. https://patents.google.com/patent/US20140297021A1/en [R]
5. JP 2005-050255 A, Yamaji & Ibaraki. https://patents.google.com/patent/JP2005050255A/en [R]
6. JP 2007-272706 A, Nakagawa & Kakino. https://patents.google.com/patent/JP2007272706A/en [R]
7. US 6,942,436 B2, Kakino et al. https://patents.google.com/patent/US6942436B2/en [R]
8. US 6,772,039 B2, Kakino et al. https://patents.google.com/patent/US6772039B2/en [R]
9. US 4,833,617 A, Wang, GE [R, fto-adaptive.md; feeds-speeds.md §6]
10. Erdim (MERL) patents US 9,892,215; 8,935,138; 9,513,623; 9,575,486; US 2013/0262066 [S, listing]
11. Google Patents query listings for every inventor/term query in §A.2 (patents.google.com/xhr/query) [R]
12. Coordinator-list applications, §A.3 [R]

Papers (citations as in roughing-linking.md, feeds-speeds.md and surface-finish.md unless marked):
13. Stori & Wright 2000, *J. Manuf. Syst.* 19(3):172-184 [S]
14. Ibaraki et al. 2004 (JUSFA) and Uddin et al. 2005 (LEM21) [R, roughing-linking]
15. Elber, Cohen, Drake 2005, *CAD* 37(2):241-250 [R, roughing-linking]
16. Choy & Chan 2002 *IJAMT*; 2003 *CAD* 35(2):155-166 [S]; HKU repository records https://hub.hku.hk/handle/10722/35296 [S]
17. Bieterman & Sandstrom 2003, *J. Manuf. Sci. Eng.* 125(4):709-715 [S]; MMS article https://mmsonline.com/articles/curvilinear-tool-paths-for-pocket-machining [S]
18. Held & Spielberger 2009 *CAD* 41(7); 2014 *CAD&A* 11(3); Held & de Lorenzo 2018 *JCDE* 5(3) [R project page, roughing-linking]
19. Abrahamsen arXiv:1412.5034; Abrahamsen & Thorup arXiv:1603.02080 [R, roughing-linking]
20. Spence & Altintas 1994, *J. Eng. Ind.* 116(1):61-69 [S]
21. Erdim, Lazoglu, Ozturk 2006, *IJMTM* 46:747-757 [S] https://research.hub.ku.edu.tr/entities/publication/5df9d00b-f26c-46bb-9aff-b702f4fd42f2
22. Suresh & Yang 1994, *J. Eng. Ind.* 116(2):253-259 [S]; Lin & Koren 1996, *J. Eng. Ind.* 118:20-28 [S]
23. Feng & Li 2002, *CAD* 34(9) [K]; Tournier & Duc 2002, *IJAMT* 19:318-324, https://hal.archives-ouvertes.fr/hal-01076666 [S]
24. Tulsyan & Altintas 2015, *IJMTM*, doi:10.1016/j.ijmachtools.2015.04.014 [S]
25. Zhao, Zhu, Ding 2013, *IJMTM* 65:88-98, doi:10.1016/j.ijmachtools.2012.10.005 [S]
26. Beudaert, Lavernhe, Tournier 2013, 5-axis local corner rounding, https://hal.archives-ouvertes.fr/hal-00843635 [S]
27. Pateloup, Duc, Ray 2004 *IJMTM* 44; 2010 *CAD* 42(9) [S]
28. Rauch, Duc, Hascoët 2009, *IJMTM* 49(5):375-383, doi:10.1016/j.ijmachtools.2008.12.006 [S]
29. Ren, Yau, Lee 2004, *Comput. Ind.* 54:17-33 [S]
30. Kimmel & Sethian 1998 *PNAS*; Crane, Weischedel, Wardetzky 2017 *CACM*; Zou, Wang, Feng 2020 arXiv:2009.02660 [R/S, surface-finish]
31. Bartoň, Bizzarri, Rist, Sliusarenko, Pottmann 2021, *ACM TOG* 40(4), doi:10.1145/3450626.3459837 [S]
32. libactp-old, https://github.com/Heeks/libactp-old [R, fto]

Vendor material:
33. Autodesk Fusion help (Adaptive, Feed optimization, Slot clearing, Stay-down) [R, fusion-benchmark.md]
34. Autodesk PowerMill help, Vortex: https://help.autodesk.com/cloudhelp/2022/ENU/PWRM-ReferenceHelp/files/GUID-77122AF8-A489-43EB-A542-E21EAA660A0D.htm and GUID-FD08A473-9992-4E0D-A283-A4946A2BA0BB.htm [R]
35. Mastercam Dynamic Motion, https://www.mastercam.com/dynamic [R]; blog https://www.mastercam.com/community/blog/how-dynamic-motion-optimizes-tool-engagement-automatically/ [R]; What's new 2024, Maximum engagement https://my.mastercam.com/whats-new-2024/whats-new-2024_maximum_engagement [R]; camInstructor, Dynamic milling part 3 https://blog.caminstructor.com/mastercam-dynamic-milling-part-3 [R]
36. OPEN MIND, hyperMILL MAXX high-performance milling https://openmind-tech.com/en/cam/hypermill-maxx-machining/roughing-hpc/high-performance-milling [R]; American Machinist 2010-11-18, https://www.americanmachinist.com/cad-and-cam/article/21894769/new-cam-strategy-for-highperformance-roughing [R]
37. SolidCAM, iMachining Technology Wizard https://solidcam.com/imachining/imachining-technology-wizard/ [R]; overview https://solidcam.com/imachining/imachining-overview/ [R]
38. Autodesk forum threads on arcs from adaptive/3D paths, https://forums.autodesk.com/t5/fusion-manufacture-forum/post-processing-3d-contour-as-g2-g3-arcs-is-not-working-on-a/td-p/10622222 [S]

## Not reached or not done

- mmsonline.com and ctemag.com returned 403; the OPEN MIND brochure PDF returned 503; HKU repository
  returned 403.
- Not searched: CN/JP inventors in native script; Espacenet; Bartoň with diacritic or by assignee
  (TU Wien, KAUST, BCAM); Crane as inventor; Zou/Wang/Feng 2020; Sona trochoid patents (patents.md).
- FR 2809978 B1 (Boeing family) status not read. CN 103042262 B, CN 104865892 B, CN 119828600 A,
  CN 122308268 A: foreign families not checked.
- Claim text of US 6,591,158, US 6,942,436 and US 6,772,039 was not parsed (status only, plus
  abstracts/summaries); irrelevant to use since all are expired, but relevant if quoted as prior art.
- No sample NC from Fusion, Mastercam, hyperMILL, PowerMill or SolidCAM was found in open sources;
  inferences rest on help text and patents only.
