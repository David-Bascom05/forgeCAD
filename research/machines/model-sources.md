# CNC machine-body CAD model sources: Haas VF-2 and UMC-750

Research date: 2026-11-24. Scope: public pages only. No login, no sign-up, no bot-check workarounds, no model files downloaded (one HTTP HEAD request to confirm an Autodesk download URL serves a file without login).

Verdict key:
- **A**: may be committed to an open repo (explicit redistribution licence).
- **B**: a user may load it locally ("bring your own machine model").
- **C**: neither, or unclear.

Disclosure: my first few reachability probes sent `curl -A "Mozilla/5.0"`. I stopped after the coordinator's note. Every source used below was re-checked with curl's default user agent and was still reachable. The hosts listed as blocked returned 403 or a challenge with both user agents.

---

## TL;DR

| Source | VF-2 | UMC-750 | Format | No-login download | Verdict |
|---|---|---|---|---|---|
| Haas (haascnc.com "Download 3D Machine Models") | yes (stated) | yes (stated) | STL + Parasolid X_T (per a 2013 tutorial); "reboot" UMC model reported Y-up | unknown (site is behind a Cloudflare challenge) | **B** at best; terms not readable, so formally **C** until a human reads them |
| Haas via MachiningCloud (since 2018/19) | yes | yes | GDML (simplified geometry + kinematics) | unknown; MachiningCloud is a sign-in service | **B** (intended for customers' CAM/simulation use); terms not checked |
| Autodesk Fusion Machine Library (cam.autodesk.com/machines) | yes: `haas vf-2.mch` + VF-2 TRT100/TRT160, VF-2 TR, VF-2YT, VF-2SSYT | yes: `haas umc-750.mch` (Classic), `haas umc-750 reboot.mch`, `haas umc-750ss.mch` | `.mch` with `simulationModel` `.f3d` (Fusion archive) | **yes** (download.php serves the file with no login) | **B** (project rule: Autodesk data is local-only, never committed). Terms not readable by our tools; David to check |
| CGTech VERICUT library | `haas_vf2.mch` (3-axis) | not listed | VERICUT `.mch` | ships with licensed install only | **C** for us (only inside a licensed install) |
| CIMCO Machine Simulation (trial) | "Haas VF-2 HRT210" | "Haas UMC750" | proprietary; trial install | trial download; terms not read | **C** |
| camInstructor (Mastercam) | (VF-3 example) | UMC-750 MD/CD/post/sim package | Mastercam MD/CD + MachSim | sold with a course | **C** |
| NexGenCAM on GrabCAD (HSMWorks/HSMXpress) | no (VF-5 instead) | "Haas UMC-750 for Machine Simulation" | HSMWorks sim (not verified) | GrabCAD requires login; host blocked to us | **B** (GrabCAD "private use") |
| Mastercam, SolidCAM, hyperMILL, ESPRIT, NX, ModuleWorks, CAMWorks, BobCAD | no public Haas VF-2/UMC-750 model found | none found | n/a | n/a | **C** / not found |
| Sketchfab "HAAS - UMC-750" (FrikiWiki3d) | n/a | yes, 1,608 faces (visual only) | Sketchfab download (glTF etc.) | Sketchfab downloads require login | **A by licence (CC BY 4.0)**, but low fidelity and provenance risk |
| Sketchfab VF-2 models (several) | yes | n/a | mixed | login required | see per-model list; **A** by licence for the CC BY ones, provenance risk |
| 3dcadbrowser.com "Haas VF-2 CNC Rotary Mill" | yes | n/a | MAX/FBX/OBJ/BLEND/C4D/STEP AP214/IGES | account required | **B**; "Redistribution of source files is not allowed" |
| Printables / Thingiverse | VF-4 only (plus accessories) | none found | STL | hosts blocked to us | **C** / not relevant |
| TraceParts / 3D ContentCentral | none found | none found | n/a | n/a | not found |

**Best realistic path:** build our own parametric machine bodies (VF-2 and UMC-750) from Haas's public published dimensions, such as the layout and installation drawings, travels, table size and the UMC B/C ranges, in ForgeCAD itself, and commit those as **A** (our own work). Separately, support "bring your own machine model" (**B**): the user points the simulator at a file they obtained, most simply a Fusion library `.mch`/`.f3d`, which needs no login (Fusion is needed to open or convert an `.f3d`), or Haas's own X_T/STL from haascnc.com. No high-fidelity VF-2 or UMC-750 model with an explicit redistribution licence was found anywhere.

---

## 1. Haas Automation itself

**What exists.**
- Haas publishes a "Download 3D Machine Models" how-to in its *Haas Apps Minute* video series: https://www.haascnc.com/video/haas-apps-minute/PbqsWSmYnFs.html. The search-engine snippet says: "Bryan walks us through the easy steps for downloading a 3D model of your machine from our website." A translated-page snippet says the models are on each machine's "Product Definition Page" on HaasCNC.com. I could not read the page itself because of the Cloudflare challenge.
- Formats: an older Haas "3D Model Tutorial" (dated January 2013, mirrored on Scribd: https://www.scribd.com/document/402628146/HAAS-3D-Model-Tutorial and https://www.scribd.com/document/453248048/3D-Model-Tutorial) is quoted in search snippets as: "Haas provides machine models in both STL Stereo Lithography and X_T Parasolid formats", packaged as ZIPs, with file names like "VF-3_BASE_DG.STL = VF-3 base that is dark grey in color, STL file type". It says to place them using the Machine Layout Drawings. I read this only through search snippets because Scribd's page did not render for WebFetch.
- UMC-750 "reboot" model: the Autodesk Fusion forum thread "HAAS UMC 750 new machine model for simulation" (https://forums.autodesk.com/t5/fusion-manufacture/haas-umc-750-new-machine-model-for-simulation/td-p/11000588) is reported in search snippets as saying the newer UMC-750 model "can be downloaded from HAAS's website" and covers several variants (Classic, SS, PP, CT, HSK). It reports that "it's quite difficult to set it all up to get a working simulation" and that Haas modeled it Y-up. The forum is 403 to our tools, so this comes from snippets only.
- A third party confirms the downloads exist. camInstructor (https://blog.caminstructor.com/virtual-haas-in-mastercam-part-1) says: "Haas, being the super swell people that they are, have all the files you need directly on their website." and "Simply find the machine and rotary combo you are after, and click download." It notes the files contain "extra solids that are not necessary for simulation". It names no formats and no terms.
- MachiningCloud partnership (2018/2019): Worcon, 2019-03-15 (https://blog.wor-con.com/en/machines/cnc-machines/haas-automation-is-now-available-on-machiningcloud/), says verbatim: "The files are in GDML format and contain simplified 3D models of the machine tool along with the machine's kinematics making them ready for use in your CAM and simulation software. For this initial release, GDML product data is available for the following machines: VF-2, VF-2SS, VF-3, VF-3SS, VF-4, VF-4SS, VF-5/40, VF-5SS, VF-6/40, VF-6SS, VF-7/40, UMC-750, UMC-750SS, Mini Mill, Super Mini Mill, Mini Mill 2, Super Mini Mill 2, Mini Mill-EDU, VM-2, VM-6, EC-400, ST-20Y, DS-30Y." The same article quotes Haas's VP of Sales and Marketing, Scott Gasich: "Haas Automation offers 3D CAD models for their complete line of CNC machines, allowing our customers to easily program and simulate their manufacturing processes offline in their shop software."
- Haas also gave Autodesk the geometry. See section 2: the Fusion library carries Haas VF-2 and UMC-750 simulation models, and the forum thread reports that Haas provided a link.

**Licence or terms.** I could not read them. haascnc.com answers automated clients with a Cloudflare challenge ("Just a moment…"), and I did not work around it. I found no quote of Haas's terms for model downloads anywhere else. The Gasich quote shows the intended use (customers' offline programming and simulation) but is not a licence.

**No-login download:** unknown.

**Verdict:** formally **C (unclear)** until a person reads the haascnc.com download page and its terms in a browser. Likely **B**: a customer loads it locally, which matches Haas's stated intent. Not **A**: no redistribution grant was found.

## 2. Autodesk Fusion Machine Library

**What exists.** The public page https://cam.autodesk.com/machines/ ("Machine Library for Fusion") is fed by the JSON index https://cam.autodesk.com/machines/machines/machines.json. Both load without login. The index has 1,208 machines, 297 of them under vendors HAAS, HAAS_machine_default or HAAS_rotary_default, and 87 visible HAAS entries with a `simulationModel`. Relevant visible entries, with fields copied from the index:

| model | description | filename | simulationModel | axes (mm / deg) |
|---|---|---|---|---|
| VF-2 | "Haas VF-2" | `haas vf-2.mch` | `haas vf-2.f3d` | X ±381, Y ±203 (table); Z −508..0 (head) |
| VF-2 | "Haas VF-2 with TRT100 tilting rotary table" | `haas vf-2 trt100.mch` | `haas vf-2 trt100.f3d` | + A ±110, C unlimited |
| VF-2 | "Haas VF-2 with TRT160 tilting rotary table" | `haas vf-2 with trt160.mch` | `haas vf-2 with trt160.f3d` | + A ±120, C unlimited |
| VF-2 TR | "Haas VF-2 TR" | `haas vf-2 tr.mch` | `haas vf-2 tr.f3d` | + A ±120, C unlimited |
| VF-2YT / VF-2SSYT | | `haas vf-2yt.mch` / `haas vf-2ssyt.mch` | `.f3d` | Y ±254 |
| UMC-750 | "Haas UMC-750 Classic (legacy design)" | `haas umc-750.mch` | `haas umc-750.f3d` | X −762..0, Y −508..0, Z −508..63.5 (head); B −35..110, C unlimited (table) |
| UMC-750 Reboot | "Haas UMC-750 Reboot (new design)" | `haas umc-750 reboot.mch` | `haas umc-750 reboot.f3d` | same |
| UMC-750SS | "Haas UMC-750SS" | `haas umc-750ss.mch` | `haas umc-750ss.f3d` | same |

Every entry's post is `haas next generation.cps`. There are also hidden `HAAS_machine_default` entries (`VF-2.machine`, `UMC-750.machine`, and others) and a hidden `UMC-750 SS 2020.machine` with a `.f3d`. These look like the definitions Haas machines push to Fusion (`haasDriver` post).

- **Format:** `.mch` (Fusion machine definition) that references a `.f3d` simulation model, which is a Fusion archive and not STEP. Opening or converting the `.f3d` needs Fusion. Help page: https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-MACHINES.htm. A "Milling Machine Simulation Setup Guide" PDF is linked from the same page: https://cam.autodesk.com/posts/posts/guides/Milling%20Machine%20Simulation%20Setup%20Guide.pdf.
- **Access:** inside Fusion (Manufacture > Machine Library), or from the web page via `https://cam.autodesk.com/machines/download.php?name=<filename>`. A HEAD request for `haas vf-2.mch` returned `HTTP/2 200`, `content-disposition: attachment; filename="haas vf-2.mch"`, with no login. Not downloaded.
- **Quality notes from the forum (snippets only):** an Autodesk community manager said of the older UMC-750 library entry: "it's not the exact one, since it doesn't have the cover on the top, but I believe the kinematics are the same." The "Reboot" entry was added later.
- **Terms:** not readable by our tools; David to check. The page footer reads "© 2025 Autodesk Inc. All rights reserved" and links to https://www.autodesk.com/company/legal-notices-trademarks, which answers automated clients with an Akamai 403. I found no licence text on cam.autodesk.com/machines itself.
- **Verdict: B** (per our rule: Autodesk data local only, never committed). The `.mch` axis limits and kinematic chain in machines.json are useful cross-checks for our own models (facts, not geometry), but they are still Autodesk data, so treat them as local reference unless David clears them.

## 3. Other CAM vendors' machine libraries

- **CGTech VERICUT.** The library documentation (https://help.vericut.com/vdaf_help/2.4_library_files) lists "HAAS VF2, 3-axis vertical mill: `haas_vf2.mch`", "HAAS MiniMill: `haas_minimill.mch`" and "HAAS VB-1 (`hass_vb1.mch`)". No UMC-750 is listed. The page describes these as "example NC machines intended to help you configure Vericut". No licence text is on the page. Files come only with a licensed VERICUT install. **C** for us.
- **CIMCO Machine Simulation.** https://www.cimco.com/?p=10825 says the free 90-day trial "currently includes the following machines": "Haas UMC750: 5-Axis Vertical Mill", "Haas UMC1000", "Haas VF-2 HRT210: 4-Axis Vertical Machining Center with HRT210 Rotary Table" (added in v8.00.48). Proprietary, inside the trial software. **C**.
- **Mastercam.** I found no official list naming a Haas VF-2 or UMC-750 machine-sim model. camInstructor's lesson (https://learn.caminstructor.com/course/293/1968/21107/2020/5Axis/5-Axis-Lesson-8.pdf, snippet) ships a UMC-750 "Control Definition, Machine Definition, Post, and Machine Simulation files" package with its paid course. Mastercam's "Haas 5X Mill" page (https://www.mastercam.com/?p=541) is a post processor, not a model. Tech Exchange was not reached. **C**.
- **SolidCAM.** Only an install guide (https://hs.us.solidcam.com/knowledge-base/machine-simulation-installation-guide): machine sims are delivered as .zip packages and extracted into `...\Tables\Mach-Sim\xml`. No public Haas package found. **C**.
- **hyperMILL, ESPRIT, NX CAM machine library / Machine Tool Builder, CAMWorks, BobCAD:** no public Haas VF-2 or UMC-750 model found in search. These are typically supplied by the vendor or reseller under the software licence. **C** / not found.
- **ModuleWorks:** an SDK with a kinematic machine builder (https://www.moduleworks.com/software-components/simulation/). No public Haas models. **C**.
- **TopSolid (Missler):** a UMC-750 post plus joint simulation tests with Haas Europe (https://mmsonline.com/news/missler-software-and-haas-automation-europe-collaborate-on-five-axis-machining). No public model. **C**.

## 4. Community models

### GrabCAD (host blocked: grabcad.com and account.grabcad.com return a CloudFront 403 "Request blocked" to our tools)
Models known only from search snippets; I could not open their pages:
- "Haas UMC-750 for Machine Simulation" by NexGenCAM (https://grabcad.com/nexgencam-1/models): "can be used for Machine Simuation in HSMXpress or HSMWorks". NexGenCAM also has a Haas VF-5 model.
- "HAAS VF-2SS Table" (a table only, "for reference and fixturing purposes"), seen in a user's likes list (https://print.grabcad.com/marius--21/likes). Author not confirmed.
- No full Haas VF-2 machine model was confirmed.

**GrabCAD terms.** The full Terms of Use (https://grabcad.com/terms) could not be read (403). Quoted verbatim from the GrabCAD Help Center article "How can models be used and shared?" (https://help.grabcad.com/article/246-how-can-models-be-used-and-shared, which is reachable):

> "When you become a GrabCAD member, you are given access and permission to use millions of CAD models from the GrabCAD Library. … You could make local changes to the model. As long as you have the appropriate software to do so, you are welcome to make changes or additions to the local copy of the model that you downloaded, as long as those changes remain local to your computer. … As stated above, models downloaded from GrabCAD are generally for private use. … For non-commercial public use: … Please make sure to attribute the model's original creator, and include a web link to the original model that inspired your design. Examples include sharing on social media, including in a student presentation, or uploading to a free CAD library like GrabCAD. For commercial public use: If you intend to exchange this model or resign for money, goods or services, or you wish to use this model in some way for your business or company, then this applies to you. You will need to ask the model's original designer for their explicit permission, and then you will need to properly attribute both GrabCAD and the user. However, if a company's title, logo, or other intellectual property is present anywhere in the model, its title or its description, then we would strongly urge you to consider not utilizing this model in a commercial sense. Also, if for any reason you do not get any response from the model's creator, then you cannot use it for commercial purposes."

The Community guidelines (https://help.grabcad.com/article/149-community-values-and-guidelines), as quoted via WebFetch: "all models on GrabCAD are free for personal use"; "charging for GrabCAD models is not allowed."

**Verdict for GrabCAD models: B** (a user's own local, private use; a GrabCAD login is needed to download). Not A: there is no redistribution licence, and changes must "remain local to your computer".

### Sketchfab (web pages return a 202 challenge; the public API at api.sketchfab.com/v3 answered normally). Downloading any Sketchfab model requires a login.
Licence objects as returned by the Sketchfab API: CC BY = `"fullName": "Creative Commons Attribution", "requirements": "Author must be credited. Commercial use is allowed.", "url": "http://creativecommons.org/licenses/by/4.0/"`.

| Model | Author | Licence (Sketchfab field) | Downloadable | Notes |
|---|---|---|---|---|
| [HAAS - UMC-750](https://sketchfab.com/3d-models/haas-umc-750-3191125f65534f54820145e1bb64c74a) | FrikiWiki3d | CC Attribution (BY 4.0) | yes (login) | 1,608 faces, 2019; description "Maquina inyección de plastico grupo HAAS, modelo UMC-750". Low-poly visual only |
| [HAAS - VF-2](https://sketchfab.com/3d-models/haas-vf-2-28713faa4bc245dd893aa4799b26e9d2) | FrikiWiki3d | none (not downloadable) | no | 1,003 faces |
| [VF-2_STEP__1_2023](https://sketchfab.com/3d-models/vf-2-step--1-2023-6725da93906048a48bf5edff7a5e1b84) | agnes_shao | CC Attribution (BY 4.0) | yes (login) | 83,140 faces, 2026-02; description "Haas VF-2 2023 CNC". The name suggests a re-upload of Haas's own CAD, so the uploader may not hold the rights to license it |
| [HAAS VF-2SS CNC](https://sketchfab.com/3d-models/haas-vf-2ss-cnc-c5543433d12944d482357697c396f7c8) | Earnect | CC Attribution (BY 4.0) | yes (login) | 11,136 faces, 2026-07, visualization model |
| [VF-2 SS TR 2020-01](https://sketchfab.com/3d-models/vf-2-ss-tr-2020-01-20b0a2d6e9ab4d3d823a7324bee7b6c4) | tonycam14 | none | no | 1.32 M faces (likely Haas CAD) |
| [HAAS - Vf-3](https://sketchfab.com/3d-models/haas-vf-3-104c4f4adeab47048766fb52cf3fca03) | FrikiWiki3d | CC Attribution | yes (login) | VF-3, not VF-2 |

**Verdict:** the CC BY ones are **A by their stated licence**, but I do not recommend committing them. They are visual-grade with no kinematic structure, the models carry Haas trademarks, and at least one looks like a re-upload of Haas-originated CAD whose uploader may not have the right to relicense it. Downloading also needs a Sketchfab login.

### 3dcadbrowser.com
"[Haas VF-2 CNC Rotary Mill](https://www.3dcadbrowser.com/3d-model/haas-vf-2-cnc-rotary-mill)", "Uploaded by Adam Szechy on November 2013". Formats: MAX, FBX, OBJ, BLEND, C4D, STEP AP214, IGES. The licence text on the page, verbatim: "Royalty-Free License. Commercial and editorial use according to site license terms. Redistribution of source files is not allowed." Also: "You need a 'Free with uploads' or Paid account to download this 3D model." **B** (not committable: redistribution is forbidden).

### CGTrader
"milling mashine haas 750" (https://www.cgtrader.com/3d-models/industrial/tool/milling-mashine-haas-750). The page returned a 202 bot challenge, so I could not read its licence. Unverified; **C**.

### Printables / Thingiverse (both 403 to our tools)
- Only near matches turned up: "Haas VF4 model" (https://printables.com/model/199909-haas-vf4-model, "imported from Thingiverse") and accessories that fit a VF-2 (a rear shower, coolant nozzle extensions). No VF-2 or UMC-750 machine model was found. Licence not read. **C** / not relevant.

### TraceParts / 3D ContentCentral
No Haas machine-tool models were found by search. Haas does not appear to publish there. Not found.

---

## Blocked hosts (not worked around)
- **haascnc.com**: Cloudflare challenge ("Just a moment…").
- **grabcad.com, account.grabcad.com**: CloudFront 403 "Request blocked". help.grabcad.com is reachable.
- **forums.autodesk.com**: 403. Read via search snippets only.
- **autodesk.com legal pages**: Akamai 403 (per the coordinator). Terms not readable by our tools; David to check.
- **printables.com, thingiverse.com, tenlinks.com**: 403.
- **sketchfab.com (web), cgtrader.com**: 202 challenge. The Sketchfab public API was used instead.
- **scribd.com**: the page did not render for the fetcher. Snippets only.

## Recommendations
1. **Commit (A):** our own ForgeCAD-modelled VF-2 and UMC-750 bodies (base, column, saddle/table, spindle head, enclosure envelope, and the UMC trunnion with B/C), dimensioned from published numbers such as travels, table size, B/C ranges and layout drawings. That is facts, not copied geometry, so licensing is clean.
2. **Bring your own (B):** a loader for user-supplied STEP/X_T/STL, mapped to our kinematic chain (with an explicit up-axis option, since the Haas UMC-750 "reboot" model is reported Y-up). Document how to get Haas's own model (haascnc.com product page) and Fusion's library `.mch`/`.f3d` (cam.autodesk.com/machines, no login, but `.f3d` needs Fusion to convert).
3. **Ask Haas** (an applications engineer or a Haas Factory Outlet) for written permission to redistribute simplified machine envelopes. That is the only route to an A-grade official model. MachiningCloud GDML terms are also worth a human check.

## Proposed import path: "bring your own machine model" (nothing committed)

A user who holds a licence to a machine model (Haas's own STEP/X_T/STL from haascnc.com, a Fusion
`.mch`/`.f3d` exported through Fusion, a GrabCAD model under its private-use terms) loads it
locally; forgecad stores nothing of it in the repository.

1. `forgecad machine model import NAME FILE.step|.stl|.obj [--up y|z] [--mm|--in]` reads the file
   (forge-step for STEP, forge-mesh for meshes; X_T is not read: export STEP from it), and lists
   its bodies/solids with their bounding boxes and a thumbnail.
2. The user maps bodies to the record's links (`--map 'saddle=Y,table=X,head=Z,base=base'`, or in
   the Machine Builder by clicking each body), and gives the model-to-machine transform: the table
   top centre at home (our frame origin), picked on the model, plus the up axis.
3. Stored at the user or project level, outside the repo:
   `<config>/forgecad/cam/machines/<NAME>/model/` (or `<project>/cam/machines/<NAME>/model/`) with
   a `model.toml`: source file name + SHA-256, the user's licence note (free text, required), the
   link map and transform. The record itself (kinematics, limits, cited envelopes) is unchanged.
4. Use: the simulator draws the user's bodies on their links (display), while collision keeps
   using the record's cited envelope boxes, unless the user opts in to checking against their mesh
   (then the mesh must be closed: forge-mesh's watertight check; refused otherwise, with why).
5. Guard: tools/ci/no-vendor-tool-libraries.py's pattern extends to `cam/machines/*/model/` and to
   large STEP/STL files outside `parts/`, so a user's model is never committed by mistake.

The committed default stays our own envelope bodies, dimensioned from Haas's published layout
drawings and spec data (research/cam/machines/haas/layout-drawings-2026-11-24.json).
