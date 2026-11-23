# Commercial CAM survey (Fusion as reference point)

Date: 2026-11-23. Scope: prior art for a possible forgeCAD CAM module. Research only.

Source marks: **[R]** read directly this session; **[S]** search-engine snippet only (not opened);
**[K]** background knowledge, not re-read this session (treat as unverified). All URLs dated
2026-11-23 (date read/searched). Vendor pages are marketing; performance claims ("up to 90%")
are the vendor's, not measured. Several Autodesk and Hexagon web pages returned HTTP 403 to the
fetcher, so Fusion marketing/blog claims are [S]; Fusion *help* pages (help.autodesk.com/cloudhelp)
were readable and are the main [R] base for the Fusion toolpath table.

---

## 1. Fusion toolpath list as of 2025-2026, base vs Manufacturing Extension

Method: crawled the Fusion CAM help tree (`MFG-MILLING-OVERVIEW` and children) and flagged each
page that carries the "This feature is part of an extension" banner [R]
(https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-3D-MILLING-OVERVIEW.htm and siblings).
Cross-checked against Autodesk's comparison PDF "Fusion vs. Fusion for Manufacturing", created
2025-05-28 [R] (https://damassets.autodesk.net/content/dam/autodesk/www/pdfs/fusion-manufacturing-extension-comparison-updated-v3.pdf).
And the 2026 release notes [R] (https://help.autodesk.com/cloudhelp/ENU/Fusion-ReleaseNotes/files/FIC-REL-NOTES-INC.htm).

Ext = requires the Manufacturing Extension. "base" = included in a paid Fusion subscription.

| Area | Strategy / feature | Ext? | Notes |
|---|---|---|---|
| 2D mill | 2D Adaptive (Adaptive Roughing) | base | constant-engagement roughing |
| 2D mill | 2D Offset Roughing (was 2D Pocket) | base | |
| 2D mill | Face | base | |
| 2D mill | 2D Contour | base | |
| 2D mill | Slot | base | |
| 2D mill | Trace | base | |
| 2D mill | Thread (thread milling) | base | |
| 2D mill | Bore, Circular | base | helical |
| 2D mill | Engrave | base | |
| 2D mill | 2D Chamfer | base | |
| Drilling | Drill (cycles: drill, peck, chip-break, tap, bore, ream, thread-mill etc.) | base | "automated hole selection and sorting"; one diameter at a time |
| Drilling | Hole Recognition (auto hole recognition + hole templates, multi-axis drilling) | **Ext** | groups holes across planes, multi-tool operations |
| 3D rough | 3D Adaptive Roughing | base | stock-aware, rest machining supported |
| 3D rough | 3D Offset Roughing (was Pocket Clearing) | base | |
| 3D rough | 3+2 Roughing / 3+2 Clearing | **Ext** | 3+2 Clearing added July 2026 |
| 3D finish | Steep and Shallow | **Ext** | hybrid scallop+contour (and parallel) split by slope; also 5-axis |
| 3D finish | Flat | base | |
| 3D finish | Parallel, Scallop, Contour | base | |
| 3D finish | Ramp, Pencil, Horizontal | base | Ramp gained "machine shallow areas" July 2026 |
| 3D finish | Spiral, Radial, Morphed Spiral | base | |
| 3D finish | Project, Blend, Morph | base | |
| 3D finish | Flow (3-axis) | base | multi-axis Flow is Ext |
| 3D finish | Geodesic (3-axis) | base | multi-axis Geodesic is Ext |
| 3D finish | Corner (automatic rest/corner finishing) | **Ext** | finds unmachined stock in internal corners |
| 3D finish | Deburr (edge break, auto edge selection) | **Ext** | 3-5 axis |
| Multi-axis | Tool orientation (turn/tilt/align to view/surface normal), 3+1 / 3+2 positional | base | |
| Multi-axis | 4th-axis Wrap (wrapped 2D toolpaths) | base | help page says base; PDF row "3+1 and 3+2" ticked for both |
| Multi-axis | Multi-axis Contour, Multi-axis Flow, Multi-axis Geodesic | **Ext** | |
| Multi-axis | Swarf, Advanced Swarf | **Ext** | |
| Multi-axis | Multi-Axis Roughing, Multi-Axis Finishing | **Ext** | |
| Multi-axis | Rotary Parallel, Rotary Roughing (Rotary Offset Roughing), Rotary Contour | **Ext** | |
| Multi-axis | 5-axis Steep and Shallow, collision avoidance (tilting), clearance geometry, tool-axis smoothing | **Ext** | |
| Multi-axis | Polar machining options / polar links | base? | added Sept 2026; gating not stated |
| Modify | Toolpath modifications: trim/limit, delete passes, move entry positions, leads-and-links edit, replace tool | **Ext** | |
| Turning | Face, Profile Roughing, Profile Finishing, Adaptive Roughing (turning), Groove, Single Groove, Groove Roughing/Finishing, Thread, Chamfer, Part (part-off), Trace | base | |
| Turning | Part handling: sub-spindle grab/return, bar pull | base | |
| Mill-turn | Turn-mill / mill-turn / live tooling | base | PDF ticks both columns |
| Fabrication | 2D Profile (laser, waterjet, plasma) | base | |
| Fabrication | Basic arrange/nesting | base | multi-sheet "advanced nesting", reports, remnants: Ext |
| Probing | Probe WCS (setup probing), manual inspection | base | |
| Probing | Probe Geometry (in-process), Inspect Surface, Part Alignment, Live Part Alignment (2-axis method added 2026) | **Ext** | |
| Additive | FDM, binder-jet prep, 2D/3D arrange | base | |
| Additive | Metal powder-bed (MPBF) prep, DED incl. multi-axis, PBF process simulation, 3MF export with supports | **Ext** | |
| Simulation | Stock simulation; machine simulation with free machine library; Machine Builder | base | PDF: "Machine simulation" ticked for both |
| Post | Post processing with free, editable JavaScript (.cps) post library | base | |
| Docs | Setup sheets (HTML, Excel) | base | |
| Automation | Templates (operation templates; hole templates) | base / Ext (hole templates) | |
| AI | Autodesk Assistant: conversational milling setup + toolpath creation, tool suggestions | base (gating not stated) | GA Sept 2026 per Autodesk [S] |
| AI | "AI-generated CAM programming", "Instant Prismatic Machining" | announced, not shipped | AU 2026 [S] |
| AI (3rd party) | CloudNC CAM Assist, Toolpath add-ins | separate purchase | |
| API | Fusion Compute MCP (public beta): agents drive setups, CAM templates, toolpaths; Manufacturing Data Model API (metered from Aug 2026) | — | [S] / [R] release notes |

Sources for rows: help pages listed above [R]; Hole Recognition page `LP-READ-IRONHOLERECOGNITION.htm` [R];
Multi-axis overview `MFG-MULTI-AXIS-MILLING-OVERVIEW.htm` (explicit base vs Ext list) [R];
Probing/inspection `MFG-PROBING-OVERVIEW.htm` [R]; Turning `MFG-TURN-OPERATIONS-OVERVIEW.htm` [R];
Setup sheets `MFG-SETUP-SHEET-OVERVIEW.htm` [R]; Post processing `MFG-POST-PROCESSING-OVERVIEW.htm` [R];
Machine Builder `MFG-MACHINE-BUILDER-OVERVIEW.htm` [R]; Geodesic (`?contextId=MFG-MULTI-AXIS-GEODESIC`) [R];
AI items: https://www.autodesk.com/products/fusion-360/blog/fusion-manufacturing-innovations-imts-au/ [S],
https://www.autodesk.com/products/fusion-360/blog/fusion-cam-assistant-milling-toolpaths/ [S],
https://www.autodesk.com/products/fusion-360/blog/ai-powered-workflows-coming-to-autodesk-fusion/ [S].

Observations for forgeCAD:
- The **base** product already covers what most job shops program daily: 2D, 3-axis rough/finish,
  3+2 positional, turning, mill-turn, drilling cycles, posts, machine sim, setup sheets.
- The Extension monetizes: (a) simultaneous 4/5-axis + collision avoidance, (b) automation
  (Steep & Shallow, Corner, Deburr, Hole Recognition), (c) toolpath editing, (d) in-process
  probing/alignment, (e) metal additive.
- Fusion's 3D strategy list is 21 named strategies; most are classic CAM-textbook strategies
  (offset, raster, waterline, scallop, pencil, radial/spiral, morph/flow).

---

## 2. Vendor-by-vendor

Columns for each: best at / roughing / 3D finishing / 5-axis / turning & mill-turn / simulation /
posts / tools / feature recognition / automation & AI / licensing & price.

### 2.1 Autodesk Fusion (+ Manufacturing Extension)
- **Best at:** integrated cloud CAD+CAM at low cost; fastest ramp for small shops; free editable posts.
- **Roughing:** 2D/3D Adaptive (constant engagement, originally HSMWorks tech [K]), Offset roughing,
  3+2 Clearing (Ext). See table.
- **5-axis:** Ext-only; swarf, multi-axis contour/flow/geodesic, rotary, 5-axis Steep & Shallow,
  tilt-based collision avoidance. An Autodesk blog title refers to "ModuleWorks machining strategies"
  for the extension, i.e. some multi-axis strategies are licensed components
  (https://www.autodesk.com/products/fusion-360/blog/fusion-360-machining-extension-future-moduleworks-machining-strategies/) [S].
- **Turning/mill-turn:** base product has full 2-axis turning, sub-spindle part handling, mill-turn.
  No source found on true multi-channel synchronisation; ESPRIT is the usual reference there.
- **Simulation:** stock + machine simulation in base; Machine Builder for custom kinematics;
  "simulation-ready" machines added monthly (2026 release notes) [R].
- **Posts:** hundreds of free, open JavaScript posts; Autodesk doesn't write custom posts itself
  (https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-POST-LIBRARY-OVERVIEW.htm) [S]; post overview [R].
- **Tools:** cloud tool library; vendor libraries (Harvey/Helical 14,000+ end mills, geometry
  only, no feeds/speeds — https://www.autodesk.com/products/fusion-360/blog/fusion-360-supports-14000-end-mills-harvey-performance/) [S];
  Sandvik CoroPlus Tool Library add-in (https://www.sandvik.coromant.com/en-gb/support/coroplus-tool-library-support/fusion-360-plugin) [S].
- **Feature recognition:** Hole Recognition (Ext); pocket/fillet recognition described as exploratory (AU 2026) [S].
- **Automation/AI:** templates; Autodesk Assistant CAM (GA Sept 2026); Toolpath and CloudNC add-ins;
  Autodesk invested in Toolpath on 2025-08-21 (https://adsknews.autodesk.com/en/news/strategic-investment-toolpath/) [S].
- **Price:** Fusion ~US$680/user/yr; Manufacturing Extension ~US$1,465/user/yr per G2; Autodesk
  pages quote ranges ($495+ / $595-1,465) inconsistently (https://www.g2.com/products/autodesk-fusion/pricing,
  https://www.autodesk.com/solutions/autodesk-fusion-subscription-faq) [S]. Subscription only.
  HSMWorks (SOLIDWORKS-embedded sibling) no longer sold; Fusion is the entitlement since 2025-03-25;
  HSMWorks end-of-life 2028-03-25 (https://www.autodesk.com/products/hsmworks) [S].

### 2.2 Mastercam (CNC Software, owned by Sandvik)
- **Best at:** installed base, reseller/post network, breadth (mill, lathe, mill-turn, router, wire, Swiss).
- **Ownership:** Sandvik agreed to buy CNC Software in Aug 2021; ~270,000 licenses/users then
  (https://www.digitalengineering247.com/article/sandvik-to-acquire-cam-software-company-cnc-software) [S].
- **Roughing:** Dynamic Motion family (Dynamic Mill, OptiRough, Dynamic Contour, Peel Mill) [K];
  vendor claims 25-75% cycle-time reduction (https://online.flippingbook.com/view/525404410/) [S].
- **3D finishing:** Accelerated Finishing (barrel/oval-form tools), Equal Scallop, Waterline, Hybrid, Raster, Pencil [K].
- **5-axis:** Multiaxis module; 2026 renamed Multiaxis Roughing to Multiaxis Pocketing (undercut roughing,
  wall/floor finishing); asymmetric deburr; 2027 adds fixture-aware toolpaths and automated rest finishing
  (https://www.mastercam.com/wp-content/uploads/2025/06/Mastercam-2026-Whats-New.pdf,
  https://www.mastercam.com/wp-content/uploads/2026/06/Mastercam-2027-Whats-New.pdf) [S].
  2026.R2 auto-tilt collision control supports bullnose tools [R].
- **Simulation:** GPU-accelerated verification "up to 10x" faster (2026.R2) [R]
  (https://www.mastercam.com/solutions/products/mastercam-2026-r2/).
- **Probing:** Blum tool-setter support across Fanuc/Siemens/Okuma/Heidenhain/Mazak/Makino (2026.R2) [R].
- **Posts:** MP post language; posts typically from resellers (paid) [K].
- **AI:** Copilot (voice/text parameter edits, help search) in 2026.R2 [R]; CloudNC CAM Assist
  supports Mastercam 2025-2027 (https://www.cloudnc.com/mastercam) [S].
- **Licensing:** perpetual or subscription, quote-only; maintenance via Mastercam CONNECT [R].
  Forum-reported (2024) maintenance: Mill 3D >$2,000/yr, +~$1,000 multiaxis, +~$1,000 mill-turn
  (https://mastercamforums.com/forum/index.php?msg=62339) [S]. Weak evidence.

### 2.3 hyperMILL (OPEN MIND)
- **Best at:** high-end 5-axis (aerospace, impellers/blades, molds), barrel-cutter finishing.
- **MAXX Machining** — three separately licensable modules [R]
  (https://openmind-tech.com/en/cam/hypermill-maxx-machining):
  - Roughing: HPC trochoidal / optimized pocket strategies with controlled engagement.
  - Finishing: barrel-cutter strategies incl. 5-axis **tangent plane** machining; conical barrel
    cutters with 250-1,500 mm form radii allowing 6-8 mm stepovers
    (https://openmind-tech.com/en/cam/hypermill-maxx-machining/finishing/conical-barrel-cutter) [S];
    vendor claims up to 90% time saved [S].
  - Drilling: 5-axis helical drilling without predrilling.
- **Other:** 5-axis rest machining, 5-axis automatic tool-axis mode (2026: "minimum clearance angle"),
  right-angle heads in TOOL Builder with collision tracking, "hole brush" deburr tool, CAM Plan Turning
  improvements (https://www.americanmachinist.com/cad-and-cam/product/55389366/updated-cam-increases-accuracy-programming-speed-open-mind-technologies) [S].
- **Simulation:** VIRTUAL Machining Center (G-code-based machine sim + optimizer) [K].
- **Automation:** AUTOMATION Center, feature/macro technology, hyperCAD-S own CAD [K].
- **Price:** not public (https://www.g2.com/products/hypermill/pricing) [S]; perpetual + maintenance typical [K].

### 2.4 Siemens NX CAM / NX X Manufacturing
- **Best at:** enterprise/aerospace, integrated with Teamcenter; turbomachinery, feature-based machining,
  G-code driven integrated simulation [K].
- **Roughing:** Cavity mill, Adaptive Milling [K]; Rough Turn Adaptive for Inconel/Hastelloy new in 2606 [R].
- **2606 (June 2026) items** [R] (https://blogs.sw.siemens.com/nx-manufacturing/whats-new-in-nx-for-manufacturing-2606-june-2026/):
  background toolpath generation, fixture automation (vises, tombstones), expanded planar deburring
  (front/back chamfer, dovetail, spherical tools), face-mill zig-zag across faces, closed-loop
  cut-measure-cut probing (Renishaw, Blum), new CAM-only NX X Manufacturing packages (Standard/Advanced/Premium).
- **Feature recognition:** Feature Based Machining (FBM) with rule-based process assignment [S].
- **AI:** "AI Make Machining Suggestion" (NX X 2512, LLM-based; 2606 adds history-free generative suggestion
  and auto-creates missing parametric tools) [R]; CloudNC add-on and Lambda Function tool-selection plug-in [S]
  (https://blogs.sw.siemens.com/nx-manufacturing/siemens-partners-with-lambda-function-to-explore-ai-assisted-cnc-programming-for-nx-cam/).
- **Price:** search snippet of Siemens catalog: NX X Manufacturing ~$7,080 / $9,480 / $11,880 per seat-year
  (Standard/Advanced/Premium) [S, unverified] (https://plm.sw.siemens.com/ja-JP/product-catalog).

### 2.5 ESPRIT EDGE (Hexagon)
- **Best at:** mill-turn, multi-channel, Swiss, machine-builder-validated posts/digital twins (DMG MORI
  distributes it) (https://en.dmgmori.com/products/digitization/work-preparation/cam-software/hexagon-esprit-edge) [S].
- **Roughing:** ProfitMilling (2.5-5 axis, patented) and ProfitTurning; 2026.2 adds dynamic cutting
  optimisation in Pocketing with steady engagement [S]
  (https://hexagon.com/company/newsroom/press-releases/2026/hexagon-introduces-new-esprit-edge-capabilities-to-reduce-cnc-programming-effort-and-support-high-complexity-machining).
- **Mill-turn:** multi-spindle, multi-channel synchronisation; Swiss support added 2025 [S].
- **AI:** Hexagon Copilot (assistant) and ProPlanAI (automatic process planning) from ESPRIT EDGE 2025.2;
  ProPlanAI extended to turning/mill-turn/Swiss, available March 2026 [S]
  (https://hexagon.com/company/newsroom/press-releases/2025/hexagon-revolutionises-cam-programming-with-swiss-machining-and-ai-powered-automation).
- **Price:** not public (https://www.g2.com/products/esprit/pricing) [S].

### 2.6 SolidCAM (+ iMachining)
- **Best at:** CAM embedded in SOLIDWORKS (also Inventor, NX via iMachining for NX); iMachining roughing.
- **iMachining 2D/3D:** morphing-spiral constant-engagement roughing; Technology Wizard computes cutting
  conditions per segment; 1-8 "machining level" slider for rigidity/MRR [S]
  (https://solidcam.com/imachining/imachining-technology-wizard/).
- **Other:** HSS/HSM 3D, Sim 5-axis, mill-turn, Swiss [K]. 2026: less manual geometry selection in 2.5D,
  multi-workpiece, faster simulator, kinematic machine simulation in beta [S]
  (https://www.solidsolutions.co.uk/whats-new-in-solidcam-2026/). Mentions CAM Assist integration [S].
- **Price:** not public.

### 2.7 Autodesk PowerMill
- **Best at:** complex 3- and 5-axis surface machining for mold/die, large parts; huge-model performance [K].
- **Strategies:** Vortex (constant-engagement roughing), model rest roughing, steep-and-shallow, optimized
  constant-Z, raster, flowline, 5-axis with automatic collision avoidance, toolpath editing [K].
- **Tiers:** sold as "Fusion with PowerMill" Standard/Premium/Ultimate; simultaneous 4/5-axis only in Ultimate [S]
  (https://www.autodesk.com/products/powermill/features).
- **2026 release:** users on Autodesk forum call it mostly fixes [S]
  (https://forums.autodesk.com/t5/powermill-forum/powermill-2026/m-p/13886787). Weak signal of reduced investment vs Fusion.

### 2.8 CAMWorks (HCL) / SOLIDWORKS CAM
- **Best at:** feature-based automation inside SOLIDWORKS / Solid Edge.
- **AFR:** automatic feature recognition of 20+ prismatic feature types, re-recognised on design change;
  **TechDB** stores shop rules to assign tools/strategies [S]
  (https://www.goengineer.com/blog/what-is-feature-based-cam-and-why-is-it-important).
- **Roughing:** VoluMill option (also sold standalone as NEXION) [S] (https://volumill.com/node/62).
- SOLIDWORKS CAM is a CAMWorks-based subset [K].

### 2.9 Onshape CAM Studio (PTC)
- Cloud-native, released (beta) Feb 2025 [S] (https://www.ptc.com/en/blogs/corporate/ptc-releases-cloud-native-cam-studio-in-onshape).
- Feature page [R] (https://www.onshape.com/en/features/cam-studio): 2.5-3 axis, adaptive in all versions,
  interactive machine simulation with full kinematics, customizable tools/machines/posts; included in
  Professional/Enterprise plans (Professional ~$2,500/user/yr per DEVELOP3D [S],
  https://develop3d.com/simulation/first-look-onshape-cam-studio/). **CAM Studio Advanced**
  (3+2, 4/5-axis, turning) still "coming soon"/waitlist on the page read.
- Relevance: a parametric cloud CAD vendor treated CAM as "the last piece" and shipped a deliberately
  small 2.5/3-axis slice first.

### 2.10 Others worth knowing
| Product | Owner | Niche / notable | Source |
|---|---|---|---|
| WORKNC | Hexagon | automatic mold & die; Waveform roughing; 2026.2 multi-directional rest roughing in one op; NCSIMUL link | https://hexagon.com/products/product-groups/computer-aided-manufacturing-cad-cam-software/worknc/release-history [S] |
| Cimatron | Sandvik group [K] | integrated mold design + NC, electrodes; auto hole/stock recognition | https://www.cimatron.com/cimatron-mold/ [S] |
| GibbsCAM | Sandvik Manufacturing Solutions | multi-task/turning; 2026 re-engineered Advanced 3D engine; CAM Assist for 3-axis/3+2 | https://www.gibbscam.com/en/news/gibbscam-2026-next-generation-engine-redefines-advanced-3d-machining [R] |
| BobCAD-CAM | BobCAD | low-cost perpetual CAM, mold 3D finishing, machine sim | https://bobcad.com/v2/mold-machining-software-3d-surface-finishing/ [S] |
| TopSolid | TopSolid | top satisfaction in CNCCookbook 2024 survey | see §3 [R] |
| VERICUT | CGTech | independent G-code machine simulation; Force/Optimizer feed optimization; 9.6 (2025-06-30) adds AI assistant | https://www.automation.com/article/cgtech-announces-vericut-9-6 [S] |
| NCSIMUL | Hexagon | G-code machine simulation; 2026 "Selective Simulation" | https://hexagon.com/about/news-media/press-releases/2026/hexagon-helps-manufacturers-verify-and-optimise-nc-programs-faster-with-ncsimul [S] |
| CAM Assist | CloudNC | AI generates 3-axis/3+2 programs inside Fusion, Mastercam, GibbsCAM, NX; >1,000 shops; claims "up to 80%"; bespoke pricing | https://cloudnc.com/fusion [S] |
| Toolpath | Toolpath Labs | AI DFM + CAM + quoting; Fusion add-in; investors Kennametal, ModuleWorks, Autodesk | https://toolpath.com/blog/toolpath-announces-strategic-investment-by-autodesk [S] |
| CoroPlus Tool Library | Sandvik Coromant | cloud tool assemblies into Fusion, Mastercam, Gibbs, Cimatron, TopSolid, DELMIA | https://www.sandvik.coromant.com/en-us/tools/digital-machining/coroplus-tool-library [S] |
| Machining Advisor Pro | Harvey Performance | free feeds/speeds calculator for its own end mills; no confirmed auto-link into Fusion | https://www.harveyperformance.com/in-the-loupe/map-for-harvey-tool/ [S] |
| ModuleWorks | ModuleWorks | OEM toolpath/simulation SDK; claims >30 CAM vendors and >60% of leading vendors use its 5-axis component | https://www.moduleworks.com/why-moduleworks/ [S] |

---

## 3. Market and survey evidence

| Evidence | What it says | Strength |
|---|---|---|
| CIMdata press release, 2026-06-09 [R] (https://www.cimdata.com/en/news/item/30210-cimdata-publishes-global-cam-market-analysis-report) | CAM software+services market $2.9B (2024) → $3.1B (2025), +9.5%; 2026 forecast +8.3% to $3.4B; vendor ranks only in paid report ($4,995) | Strong for size; no free vendor ranking |
| Mastercam citing CIMdata (2020 release) [R] (https://www.mastercam.com/about/media/cimdata/) | #1 by installed seats 26 yrs running; ~2x seats of nearest competitor; 2022 trade press says 28th year [S] | Vendor-cited, seat-count metric (favours long-installed, edu seats), stale |
| Older CIMdata (2008/2010) via trade press [S] | Dassault + Siemens led by revenue (28.2% combined, 2008); NX led industrial seats shipped, Mastercam installed seats (2010) | Old; shows revenue vs seat rankings differ |
| CNCCookbook 2024 survey [R] (https://www.cnccookbook.com/cnccookbook-2024-cam-software-survey-which-is-best) | ~300+ responses; Fusion 360 overall share leader but down; Mastercam up "fairly significantly", NX up; top satisfaction TopSolid, Carbide Create | Self-selected readership (hobby-heavy, mix unstated); moderate-weak |
| MMS/Gardner Top Shops [S] (https://amtonline.org/article/job-shops-by-the-numbers) | 86% of 400+ US job shops have CAM; no per-vendor split public | No vendor evidence |
| G2 (2022 snapshot via search) [S] (https://www.g2.com/compare_reports/fusion-360-and-mastercam) | Fusion 4.5/5 (383 reviews) vs Mastercam 4.1 (23); hyperMILL ~4.1 (~10); PowerMill ~4.2 | Weak: tiny samples, Fusion reviews include design users; Autodesk republishes it |
| Practitioner forums (Practical Machinist, reddit) | Could not retrieve threads directly (blocked/not indexed). A compiled page reports Fusion praised for learnability, stability complaints across all products [S] (https://ency-product-page-wireframes.pages.dev/research/community-pains.html) | Weak / anecdotal |

Common consensus [K, consistent with above]: Fusion dominates hobby/small-shop/education and is
"good enough" for most 3-axis and 3+2 prismatic work; Mastercam is the default professional
job-shop choice in North America; hyperMILL/NX/PowerMill are preferred for demanding 5-axis,
blades, molds; ESPRIT/NX/Gibbs for complex mill-turn and Swiss; Vericut/NCSIMUL are the
reference for G-code-level verification in aerospace.

---

## 4. Is Fusion best?

**Evidence for (qualified: best value and integration):**
- Price: ~$680/yr base includes 2D, 3D, 3+2, turning, mill-turn, probing WCS, machine sim, posts
  ([S] pricing; [R] PDF/help). Comparable Mastercam/hyperMILL/NX seats are quote-only and,
  where numbers surface, several times higher (Mastercam maintenance alone ~$2k/yr [S]; NX X
  Manufacturing ~$7k-12k/seat-yr [S]).
- One parametric model for design and CAM; CAM is associative to design changes [K].
- Free, open, editable JavaScript posts plus free machine models — unusual; most rivals sell posts
  through resellers [R]/[K].
- Largest share in the broad (hobby + small shop) CNCCookbook sample [R]; highest G2 satisfaction [S].
- Fast cadence: monthly releases with steady CAM additions (3+2 Clearing, polar machining,
  Part Alignment 2-axis, deburr point reduction in 2026) [R]; active AI investment (Assistant GA,
  Toolpath stake, Compute MCP beta) [S].
- Feature breadth with the Extension is now broad: 5-axis, automation, probing, additive [R].

**Evidence against (best at specialist work):**
- Mastercam has the largest installed professional base (CIMdata seats, vendor-cited) and was
  gaining share in 2024 while Fusion slipped (CNCCookbook) [R].
- No independent benchmark shows Fusion matching hyperMILL MAXX (barrel/tangent-plane),
  PowerMill or NX on 5-axis surface quality, blades/impellers, or mold work. Fusion's list lacks
  dedicated blade/port/impeller modules and its barrel-tool finishing is not documented in the
  help pages read [R] (absence, not proof).
- Mill-turn: Fusion covers turning + sub-spindle + live tooling, but multi-channel synchronised
  programming and Swiss is where ESPRIT (and NX, Gibbs) are explicitly positioned [S].
- Machine simulation in Fusion is toolpath/CL-based with Machine Builder; aerospace shops still
  buy Vericut/NCSIMUL for G-code-level verification and feed optimization [S]/[K].
- Many of the most useful automation features (Steep & Shallow, Corner, Deburr, Hole Recognition,
  toolpath editing) are behind the Extension, roughly tripling cost [R]/[S].
- Cloud dependency/subscription only; some shops object [K].

**Verdict:** Fusion is the best *value* and best *integrated CAD+CAM* for 2D/3-axis/3+2 and
light 5-axis and turning, and is the de-facto reference for small shops, education and makers.
It is not the best at high-end 5-axis surfacing (hyperMILL, NX, PowerMill), mold-and-die automation
(WorkNC, Cimatron, PowerMill), complex mill-turn/Swiss (ESPRIT, NX, Gibbs), or G-code machine
verification (Vericut, NCSIMUL). Evidence quality: market-size data is strong; vendor-ranking
data is either paywalled, vendor-cited, stale or from self-selected surveys, so "who is best"
rests mostly on positioning plus practitioner consensus. Medium-low confidence overall.

---

## 5. Strategy-name cross-reference (roughing that keeps tool engagement constant)

| Vendor | Name | Source |
|---|---|---|
| Fusion / HSMWorks / Inventor CAM | Adaptive Clearing (2D/3D), Turning Adaptive Roughing | help [R] |
| Mastercam | Dynamic Motion (Dynamic Mill, OptiRough, Peel Mill) | [S]/[K] |
| SolidCAM | iMachining 2D/3D | [S] |
| hyperMILL | MAXX HPC roughing (trochoidal, optimized pocket) | [R] |
| CAMWorks / standalone | VoluMill (NEXION) | [S] |
| ESPRIT | ProfitMilling / ProfitTurning | [S] |
| PowerMill | Vortex | [K] |
| NX | Adaptive Milling; Rough Turn Adaptive (2606) | [K]/[R] |
| WORKNC | Waveform | [S] |
| Onshape CAM Studio | Adaptive | [R] |

Every major vendor converged on the same idea (bounded radial engagement / chip thinning,
trochoidal or morphing-spiral paths). It is table stakes, not differentiation.

---

## 6. Ten most useful facts for "should forgeCAD build CAM, and how"

1. **The core is commoditised.** Every vendor ships constant-engagement roughing under its own name
   (§5); Fusion's base product's 21 3D strategies are classic textbook methods (offset, raster,
   waterline, scallop, pencil, morph, flow). A credible v1 is 2.5D + 3-axis adaptive + a few finishers.
2. **The price floor is ~$680/yr for full 3-axis + turning + posts + machine sim (Fusion base).**
   A new entrant does not win on 3-axis features or price; it must win on something else
   (scriptability, agent-drivability, text-native/diffable programs, open source).
3. **Money and differentiation sit in 5-axis, automation and verification** — exactly what Fusion
   puts in its Extension (simultaneous 4/5-axis, collision avoidance, Steep & Shallow, Corner,
   Deburr, Hole Recognition, toolpath editing, in-process probing). Those are the hard parts.
4. **Even Autodesk and many others license toolpath/simulation kernels** (ModuleWorks: claims >30
   CAM vendors, >60% of leading vendors for 5-axis; Autodesk blog refers to ModuleWorks strategies
   in the Extension) [S]. Buy-vs-build is a normal industry choice; open-source alternatives should be checked.
5. **Posts are a distribution moat.** Fusion's free, editable JavaScript post library and free
   machine models are a key adoption driver; Mastercam relies on resellers. A forgeCAD CAM needs
   a post format that is simple, open and data-driven from day one.
6. **Feature recognition + rules is the established automation pattern** (CAMWorks AFR + TechDB,
   NX FBM, Fusion Hole Recognition + hole templates, Cimatron auto drilling). forgeCAD's named,
   typed features (`hole`, `pattern`, `extrude`) already carry the intent that others must
   *recognise* from B-rep — a structural advantage for "design-intent-driven CAM".
7. **AI CAM in 2025-2026 is mostly assistants and 3-axis/3+2 proposal generators** (Autodesk
   Assistant, Mastercam Copilot, Hexagon ProPlanAI, NX AI Make Machining Suggestion, CloudNC CAM
   Assist >1,000 shops, Toolpath). All keep a human review step. Autodesk's Fusion Compute MCP
   beta exposes setups/templates/toolpaths to agents — the same agent-driven niche forgeCAD's
   typed-ops/JSON design targets.
8. **Cloud CAD vendors ship CAM in small slices:** Onshape CAM Studio launched (Feb 2025) as
   2.5/3-axis only, with Advanced (5-axis, turning) still waitlisted in late 2026. A narrow,
   correct v1 is an accepted strategy.
9. **Verification is a separate product category** (Vericut, NCSIMUL, plus G-code-based sims in
   hyperMILL/NX/ESPRIT). Shops trust G-code-level simulation over CL-level; forgeCAD's existing
   exact kernel/mesh tools could back stock-removal sim and gouge checks, a measurable correctness story.
10. **Market is ~$3.1B (2025), growing ~8-9%/yr, fragmented and consolidating** (Sandvik owns
    Mastercam, GibbsCAM, Cimatron; Hexagon owns ESPRIT, WORKNC, NCSIMUL; Autodesk folded HSMWorks
    into Fusion by 2028). Room exists for a niche entrant, but not for a me-too general CAM.

---

## Source list (all accessed 2026-11-23)

[R] read: Fusion CAM help pages under https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/
(MFG-MILLING-OVERVIEW, MFG-2D-MILLING-OVERVIEW, MFG-3D-MILLING-OVERVIEW, MFG-MULTI-AXIS-MILLING-OVERVIEW,
MFG-DRILLING-OVERVIEW, MFG-TOOLPATH-MODIFY-OVERVIEW, MFG-TURNING-OVERVIEW, MFG-TURN-OPERATIONS-OVERVIEW,
MFG-PART-HANDLING-OVERVIEW, MFG-PROBING-OVERVIEW, MFG-SETUP-SHEET-OVERVIEW, MFG-POST-PROCESSING-OVERVIEW,
MFG-MACHINE-BUILDER-OVERVIEW, MFG-2D-PROFILE-OVERVIEW, 3D-STEEP-AND-SHALLOW-READ, 3D-FLOW-V2-OVERVIEW,
and every child page linked from them); Fusion 2026 release notes; Autodesk comparison PDF (2025-05-28);
Geodesic help page; CIMdata 2026-06-09 release; Mastercam CIMdata page; Mastercam 2026.R2 page;
OPEN MIND MAXX page; Siemens NX 2606 blog; Onshape CAM Studio page; GibbsCAM 2026 news; CNCCookbook 2024 survey.

[S] snippet only: all other URLs cited inline above (Autodesk blogs and pricing pages returned
HTTP 403; Hexagon and G2 pages returned 403).

[K] not re-read: strategy names for Mastercam, PowerMill, NX, ESPRIT KnowledgeBase, hyperMILL
VIRTUAL Machining/AUTOMATION Center, SolidCAM HSS/HSM, SOLIDWORKS CAM lineage, Fusion Adaptive
origin in HSMWorks, general practitioner consensus.
