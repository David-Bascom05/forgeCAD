# Public toolpath, G-code and machining-process datasets for benchmarking forgeCAD's CAM

2026-11-24, branch research/toolpath-quality. Research only: no dataset file was added to the repo
and no code was changed. Companion to [datasets-parts.md](datasets-parts.md) (part models) and
[../toolpath-quality/opensource-oracles.md](../toolpath-quality/opensource-oracles.md) (licences
of the open CAM projects, which are reused here and not repeated).

**What it answers.** Which public NC programs, controller logs and measured machining results
can test four things in forgeCAD:

- **(a) Reader/post round-trip:** parse real-world dialect G-code (Fanuc-style, Siemens 840D,
  Heidenhain-derived, LinuxCNC, Grbl, Mach3) and re-emit it unchanged in meaning.
- **(b) Cycle-time model:** needs a program **and** a measured run time (whole program, or per
  block). Our estimate today is length ÷ feed with no acceleration (toolpath-quality/README.md
  line 63 [R]).
- **(c) Toolpath quality:** the same part machined from another CAM's path, so ours can be compared
  (length, air-cut, link count, engagement).
- **(d) Surface-finish or wear model:** measured Ra, flank wear VB or forces for known paths.

**Marks.** **[R]** read this session (file, API record, page or command output named);
**[S]** search snippet only; **[K]** background knowledge, not re-read. **UNKNOWN** means not
read; no guess is given. Licence readings are mine, not legal advice. forgeCAD is MPL-2.0.

**Store or fetch.** Same rule as datasets-parts.md: store a few files in-repo (with the licence
and a NOTICE) only if public domain, CC0, CC-BY or MIT, with no non-commercial clause, and small;
everything else is fetched by script with a pinned commit, DOI version and hash into a cache
outside git (proposed: `~/.local/share/forgecad-bench/gcode/<name>/`). GPL/AGPL/LGPL program
files are fetch-only: using them as test **inputs** at run time is fine, copying them into the
MPL tree is not wanted.

**Access refusals this session (recorded, not worked around).** smstestbed.nist.gov: proxy 502,
then DNS failure (host outside the allowed network) [R]; data.nist.gov: 502/503 [R]; Kaggle
dataset page: JS-only, no content without a session [R]; IEEE DataPort PHM 2010: "requires an
IEEE DataPort Subscription" [R]; Hugging Face AI4Manufacturing/PHM2010: gated, "agree to share
your contact information" [S]; GitHub REST API for repos not attached to the session: 403 [R]
(read via `git clone --filter=blob:none` instead).

---

## 0. Summary

| # | Source | NC dialect | Measured | Licence (SPDX) | Size | Store? | a | b | c | d |
|---|---|---|---|---|---|---|---|---|---|---|
| 1.1 | NIST SMS TDP / MTC assembly | ISO 6983 | MTConnect, QIF CMM | NIST licence (not read) | UNKNOWN | — unreachable | (✓) | (✓) | | |
| 1.2 | mtconnectR extdata (NIST test bed) | Mastercam → Heidenhain-derived G-code | MTConnect positions + block line, µs timestamps | AGPL-3.0 (package) | 0.14 MB | fetch | ✓ | ✓ | | |
| 2.1 | FreeCAD CAM post tests | 13+ post dialects (expected blocks) | — | LGPL-2.1-or-later | 3.4 MB dir | fetch | ✓ | | | |
| 2.2 | CAMotics examples | Vectric, PyCAM, TPL, OpenVoronoi, misc | — | GPL-2.0-or-later | 15.7 MB NC | fetch | ✓ | | (✓) | |
| 2.3 | LinuxCNC nc_files + TP tests | RS274NGC (O-words, params) | — | GPL-2.0-or-later (gcmc GPL-3+) | 3.5 + 5.7 MB | fetch | ✓ | | | |
| 2.4 | cncjs / UGS / bCNC examples | Grbl-style, CamBam, misc | — | MIT / GPL-3.0 / GPL-2.0 | small | fetch (cncjs: store ok) | ✓ | | | |
| 3.1 | PHM 2010 milling | none | force, vib, AE 50 kHz; VB per cut | none stated / CC-BY (DataPort) | ~800 MB per cutter | fetch | | | | ✓ |
| 3.2 | NASA/UC Berkeley mill | none | VB, AE, vib, current | US Gov work | 15.1 MB | fetch | | | | ✓ |
| 3.3 | U. Michigan SMART (Kaggle) | program not included | 100 ms axis cmd/actual, sequence no. | CC0 (2nd-hand) | 11.6 MB | fetch | | (✓) | | (✓) |
| 3.4 | Bosch CNC_Machining | none | 2 kHz vibration per op | CC-BY-4.0 (data) | 1 702 h5 | fetch | | | | |
| 3.5 | Spinner U5-620 production (Engelmann 2025) | Siemens 840D (NX) | 1 Hz: line no., line text, cycle time, axes | CC-BY-4.0 | 79 MB + 0.18 MB NC | NC store; data fetch | ✓ | ✓ | | |
| 3.6 | TUGMCL BasicMilling (Graz) | Siemens 840D (NX), CLSF pre-post | 500 Hz Edge data | CC-BY-NC-SA-4.0 | 989 MB zip | fetch only | ✓ | ✓ | (✓) | |
| 3.7 | KIT multimodal milling ×2 | Siemens 840D | 500 Hz controller, 10 kHz force | CC-BY-4.0 | 44.6 + 55.2 GB tar | fetch (subset?) | ✓ | ✓ | | (✓) |
| 3.8 | Malyer & Çaydaş Ti-6Al-4V pockets | Fanuc-style (NX) | Ra, tool life | CC-BY-4.0 (data), MIT (code) | 37.9 MB | NC store; zip fetch | ✓ | (✓) | (✓) | ✓ |
| 3.9 | Myhovych St3ps end milling ×3 | Fanuc-style | forces; Ra profiles | CC-BY-4.0 | 15 + 12 + 10 MB | fetch | ✓ | (✓) | | ✓ |
| 3.10 | RPTU 10-part benchmark | Autodesk post, LinuxCNC-style | — (parts photographed) | CC-BY-4.0 | 5.3 MB | store | ✓ | | ✓ | |
| 3.11 | Hannover tool wear (Mendeley) | none | force 25 kHz, drive 500 Hz, VB | CC-BY-4.0 | 10.6 GB | fetch | | | | ✓ |
| 3.12 | Ward et al. 2021 / Sun et al. | NX → TNC640 / 840D | cycle times in paper tables only | data on request | — | — | | (✓) | | |

✓ = serves directly; (✓) = partly or with work. Sections below give the evidence.

---

## 1. NIST

### 1.1 Smart Manufacturing Systems (SMS) Test Bed and the MTC assembly dataset

- **Catalog [R]** (https://catalog.data.gov/dataset/technical-data-packages-tdps-from-the-smart-manufacturing-systems-sms-test-bed):
  three channels, a volatile MTConnect stream, a queryable repository (MDCS), and pre-compiled
  Technical Data Packages; formats "CAD, CAM, TXT, XML"; licence field
  `https://www.nist.gov/open/license`; modified 2016-06-06.
- **MTC paper [R]** (Hedberg et al., J. Res. NIST 124:124004, 2019,
  https://doi.org/10.6028/jres.124.004, text extracted from the PDF): data DOI
  10.18434/T4/1503154; "Instrument: Computer numerically controlled machine tool and coordinate
  measurement system"; folders `NC-Code` ("machining programs for operations steps for each part
  in accordance with ISO 6983"), `MTConnect` (v1.3 XML), `Raw`, `Clean` (pipe-delimited), `Split`
  (by operation per part instance), `FAIR` and `QIF-IncomingInspection` (QIF 2.1, "20 instances of
  the same part"); licence pointer https://www.nist.gov/director/licensing (not read).
- **Access: unreachable [R].** The data DOI redirects to https://smstestbed.nist.gov/ (302); that
  host returned proxy 502 and then failed DNS in this session. Machine model, material, file
  sizes, and whether time stamps let per-operation durations be computed: UNKNOWN.
- **Would serve (a)(b)** if reached: real ISO 6983 programs plus MTConnect logs of the same runs.
  Needs smstestbed.nist.gov allowed in the environment's network settings.

### 1.2 NIST test-bed data inside the `mtconnectR` R package — reachable

- **Source [R]:** CRAN archive https://cran.r-project.org/src/contrib/Archive/mtconnectR/
  `mtconnectR_1.2.1.tar.gz` (2019-01-07, 443 KB), SHA-256
  `06d84654ced6876c4bb9ce59cf3e5d0332af1dec91f60af3e358d6320a348442`. Package archived from CRAN
  2022-10-03 [R]. DESCRIPTION: `License: AGPL-3` [R].
- **Contents [R]** (`inst/extdata/`):
  - `raw_gcode.NC` (23 677 B, 1 177 lines; SHA-256 in §7): header
    `( BEGIN PGM 827-9999-904 INCH)`, `(;MASTERCAM - X)`, `(;MCX FILE - C:\…\MIKRON\MONEER\…)`,
    `(;MATERIAL - ALUMINUM INCH - 2024)`, date MAR-22-2016, tool list (9/16" end mill, 3" face
    mill, drills, reamer, chamfer). Heidenhain Klartext lines (`BLK FORM`, `TOOL CALL 11 Z
    S12000`, `CYCL DEF 247`, `PLANE RESET STAY`) are kept as comments in parentheses, and the
    motion is plain `G0/G1/G2/G3 … I J` G-code. A converted dialect, useful precisely because it
    is odd.
  - `data_truncated.bz2` (98 720 B; 14 838 lines, `timestamp|key|value`): 2016-03-22
    12:45:00 → 12:59:59 UTC, keys `path_pos` (3 620), X/Y/Z/A/C positions, `line` (712), `Fovr`,
    `execution` [R].
  - `Devices.xml`: MTConnect devices `nist_testbed_Mazak_QT_1` and `nist_testbed_GF_Agie_1`
    [R]. Which device ran this program: UNKNOWN (the MCX path names "MIKRON", a GF brand [K]).
- **Serves (a)** a real CAM-posted program with converted Heidenhain blocks; **(b)** block-level
  measured timing from `line` events with µs time stamps over 15 minutes. Fetch only (AGPL-3).

## 2. Open CAM projects' and senders' sample programs

Licences of FreeCAD, Kiri:Moto, CAMotics, LinuxCNC and Grbl: opensource-oracles.md §1–3 [R there].

- **FreeCAD CAM tests** (`src/Mod/CAM/CAMTests`, FreeCAD @`8fef74c9…`, 2026-11-23) [R]: 152 entries,
  3.4 MB. Post tests for Centroid, Dynapath, EstlCam, Fanuc, Grbl, Haas, Heidenhain, KineticNC/
  Beamicon2, LinuxCNC, Mach3/4, Marlin, Masso G3, OpenSBP, Snapmaker, SVG, DXF (file names) [R].
  Expected output is **asserted blocks inside Python** (e.g. TestHeidenhainPost.py:149
  `"0 BEGIN PGM MOCKJOB MM"`, "Klartext programs are .h files") [R], not stand-alone reference
  `.nc` files. Fixtures: 5 `.FCStd` documents, and `Fixtures/mtconnect/probe_*.xml` MTConnect probe
  documents (xyz mm/inch, xyzac trunnion, multi-device) [R]. Headers `SPDX-License-Identifier:
  LGPL-2.1-or-later` [R]. **Serves (a)** as a spec of what each dialect's post must emit (we can
  port the *assertions* as our own tests, written fresh). Fetch only.
- **Kiri:Moto** (GridSpace/grid-apps @`d138275b…`) [R]: no bundled G-code. `web/obj/` has
  `box.stl`, `cube.stl`, `pocket-test.stl`; `src/cli/` has `kiri-cam-device.json`,
  `kiri-cam-process.json` (roughing/outline settings), `kiri-cam-tools.json`. **Serves (c)** only
  by running its CLI on our parts (oracle use, opensource-oracles.md §3.1).
- **CAMotics** (@`e84665f2…`) [R]: `examples/` 26 MB; 26 NC files (15.7 MB: `.nc`, `.ngc`,
  `.gcode`) and 14 `.tpl` (TPL/JavaScript CAM) programs, each with a `.camotics` project giving
  tool and stock (e.g. bear: ballnose 0.125 in, stock 80×80×20 mm) [R]. Writers seen in headers
  [R]: Vectric "for Mach2/3" (aztec_calendar.nc, 5.6 MB), PyCAM (cat), "TTT++ / OpenVoronoi"
  V-carve (vcarve.ngc), a lithophane generator (warrior.nc, 6.3 MB), N-numbered Fanuc-style
  (cameo.nc), LinuxCNC named parameters `#<z_safe>` (heart), O-word encoder program by L. Glaister
  2007 (genes-encoder.ngc). debian/copyright `Files: *` GPL-2+ [R]; no per-example notices.
  **Serves (a)** (varied real writers), **(c)** via `camsim` stock comparison. Fetch only.
- **LinuxCNC** (@`65f74679…`, sparse) [R]: `nc_files/` 3.5 MB, 75 top-level entries, 247 `.ngc`
  in all: subroutines, O-word control flow, G76 threading, lathe G7x cycles, probing, NURBS (G5/G6),
  remaps, plasma. `tests/trajectory-planner/circular-arcs/nc_files/` 5.7 MB, 169 `.ngc`
  (`performance/` has 3D_Chips, Cereal, short-segment and spiral stress paths). Licence: COPYING
  GPL-2 text; debian/copyright `Files: *` … `License: GPL-2+`, and lists nc_files contributors;
  `nc_files/gcmc_lib/{involute-gear,trochoid-path,cpocket}.gcmc` GPL-3+ [R]. File notices: 3D_Chips.ngc
  "copyright of Rab Gordon, Gary Drew, and Paul Corner… released here under a GPL" [R];
  cds.ngc "Circle Diamond Square Program, Tom Kramer, 26-Sep-1994", 4×4×2 in stock [R].
  **Serves (a)** hardest-dialect coverage and short-segment stress for (b)-style planners.
- **Grbl** (@`bfb67f0`) [R]: no G-code examples (only `doc/script/*.py` streamers, `doc/csv`).
- **cncjs** (@`71844227…`, 2026-11-23) [R]: LICENSE "The MIT License (MIT) Copyright (c)
  2015-2017 Cheton Wu"; `examples/gcode/` 23 files: arcs in XY/XZ/YZ planes, `arc-ijk`,
  `helical-thread-milling`, `Circle_Diamond_Square_Calibration.gcode` ("Credit to Shapeoko Wiki"),
  `tball/*` pocket/engrave. MIT → **storable** (keep the notice; the CDS file credits a third
  party, so check before storing that one).
- **Universal G-Code Sender** (@`76cc42b5…`) [R]: COPYING GPL-3 text; `test_files/` 22 programs
  (e.g. `Gates_combined_R12.nc` "Made using CamBam"), plus arc-expansion fixtures in
  `ugs-core/test/resources/gcode/fixtures/arc/` with expected outputs (`outArc`, `outArcCoarse`,
  `outArcMesh`) [R]. Those arc fixtures are a ready (a) check for arc linearisation. Fetch only.
- **bCNC** (@`24540c5c…`) [R]: one sample (`tests/static/sample.gcode`); LICENSE.md is the GPL-2
  text and a LICENSE.MIT also exists; which covers what: UNKNOWN.

## 3. Academic and industrial machining datasets

### 3.1 PHM Society 2010 data challenge (milling tool wear)
- **Page [R]** (https://phmsociety.org/phm_competition/2010-phm-society-conference-data-challenge/):
  "6mm ball nose tungsten carbide cutter"; 10 400 rpm, 1 555 mm/min, radial 0.125 mm, axial
  0.2 mm; 50 kHz/channel; 7 columns force X/Y/Z (N), vibration X/Y/Z (g), AE-RMS (V); records
  c1…c6, c1/c4/c6 with a wear file "in 10^-3 mm" after each of ~300 cuts; "~800 MB each" bzip2;
  links `cdn.teamholistic.net/phmconference.org/c1.zip` … (302, not followed). **No licence or
  terms stated** on the page [R].
- **IEEE DataPort [R]** (DOI 10.21227/jdxd-yy51): licence "Creative Commons Attribution", but
  subscription required and "Files have not been uploaded".
- **Mirror [R]** katulu-io/uniwear-dataset README: downsampled c1/c4/c6 (+ NUAA Ti), says
  workpiece "Stainless steel (HRC52)". Its licence: UNKNOWN (repo API refused). Machine: Röders
  RFM760 [K].
- **No G-code or toolpath.** **Serves (d)** only, for a constant-engagement wear curve.

### 3.2 NASA / UC Berkeley milling data (Agogino & Goebel)
- **Catalog [R]** (https://catalog.data.gov/dataset/milling-wear): licence
  `https://www.usa.gov/government-works`; download https://data.nasa.gov/docs/legacy/mill.zip.
- **File [R]** (range requests only): 15 140 674 B, ETag `81d821fdef812183a7d38b6f83f7cefa`,
  Last-Modified 2025-03-26; zip holds `Readme.pdf` and `mill.mat` (72.6 MB unpacked).
- **Readme [R]:** 1×167 struct; 16 cases; Matsuura MC-510V; cast iron and "stainless steel J45";
  KC710 inserts; 200 m/min (826 rpm); DOC 1.5/0.75 mm; feed 0.5/0.25 mm/rev (413/206.5 mm/min);
  workpiece 483×178×51 mm; signals AC/DC spindle current, table/spindle vibration and AE (RMS,
  250 Hz); VB measured at irregular intervals. A 2025 data paper notes it "lacks information on
  toolpaths" [S, PMC12621317 via search].
- **Serves (d)** only (face-milling wear vs conditions). Fetch (US Gov work).

### 3.3 University of Michigan SMART "CNC Mill Tool Wear" (Kaggle)
- Kaggle page could not be read (JS only) [R]. **Mirrors read [R]** by blobless clone:
  flyinginte/toolwear_cncmilling @`86fea60` (2019-10-21) and
  SaeedShurrab/Tool-Wear-Detection-in-CNC-Milling-Operartions @`e4f4852` (2020-02-18), each with
  `experiment_01…18.csv`, `train.csv`, `test_artifact.jpg`, and (second) the original README.txt.
  Neither repo has a LICENSE file [R, tree].
- **README [R]:** "April 2018", 2"×2"×1.5" **wax** blocks, "S" carved in the top face; 18
  experiments, feed 3–20 mm/s, clamp 2.5/3/4 bar, 8 unworn / 10 worn; 100 ms sampling of X/Y/Z/S
  command and actual position, velocity, acceleration, current, power; `M1_sequence_number`
  "line of G-code being executed", `M1_CURRENT_FEEDRATE`, `Machining_Process` ("Layer 1 Up"…).
- **The G-code itself is not in the mirrors** [R], and the Kaggle file list is UNKNOWN.
  Licence "CC0: Public Domain" per autonlab/pmx_data README [R, second-hand]; size 11.6 MB raw
  [R, makinarocks README]. Machine model: UNKNOWN.
- **Serves (b)** weakly: measured duration per experiment and commanded vs actual motion at
  100 ms, but the program must be reconstructed from command positions.

### 3.4 Bosch CNC Machining (Tnani, Feil, Diepold, Procedia CIRP 107, 2022)
- boschresearch/CNC_Machining @`d60581d6…` (2024-06-06) [R]: 3 machines (M01–M03), 15 processes
  OP00–OP14, `good`/`bad` labels, 1 702 `.h5` files of tri-axial acceleration at 2 kHz, Oct 2018 –
  Aug 2021. "dataset… licensed under… CC-BY-4.0"; code BSD-3-Clause [R, README]. No NC programs,
  materials or tools [R, README]; total size UNKNOWN (not fetched). **Serves none of (a)–(d)
  directly**; only op durations for a fixed, unknown program.

### 3.5 Spinner U5-620 production data (Engelmann, Schmitt, Martinez) — best (b) find
- **Zenodo 10.5281/zenodo.15735480** v1.0.1 (2025-06-25) [R]: `data_v1_0_1.csv` 78 734 331 B,
  md5 `5a758c263562595416007d554ea824da`, `cc-by-4.0`; "Siemens 840D-SL… five-axis milling machine
  tool of type 'Spinner U5-620'"; three products, 30 sessions with changeovers. Data paper
  https://doi.org/10.1038/s41597-025-05294-0 (not read).
- **Supplement [R]** github.com/ElMoe/Production-Data-Set-for-Five-Axis-CNC-Milling-with-Multiple-Changeovers
  @`b8003660…` (2025-08-08): LICENCE is CC BY 4.0 text; `NC_code/` Keychain (644 lines `.mpf`),
  Coordinate_system_representation (3 073 lines), Bottle_opener (2 792 lines `.txt`); tool list
  (Horn end mills, Garant deburrers and drill, Tungaloy D50 face mill with order codes) [R].
- **NC [R]:** `CYCLE800()`, `CYCLE832(_camtolerance,1,1)`, `DEF REAL`, `SUPA G0 Z=_Z_HOME`,
  `T="ENTFRAESER_D8_N_90GRAD"`, `G710`, `;Operation : CURVE_DRIVE` (NX operation names).
- **Log [R]:** 182 columns at 1 s, including `Program_line_number`, `Line_Content`,
  `Cycle_time_program`, `Net_running_time_program`, `Program_status`, `Feed_override`, axis
  MCS/WCS positions and following errors, spindle current/torque. Material: UNKNOWN.
- **Serves (a)** Siemens 840D/NX dialect; **(b)** measured program and per-block time (1 s
  resolution, with override recorded). Earlier series set 10.5281/zenodo.10853254 (13 CSVs,
  21 MB, CC-BY-4.0, NC variables and phase labels, no NC code stated) [R].

### 3.6 TUGMCL_BasicMilling (Trabesinger, Mücke, Hanna, Klünsner; TU Graz)
- **Zenodo 10.5281/zenodo.7753181** v0.1.0 (2023-03-23) [R]: `FairMillData.zip` 989 340 538 B,
  md5 `a3ac027b1991f7332c49666e9d2e0451`; `License.txt` is **CC BY-NC-SA 4.0** [R] (Zenodo field
  says `other-at`).
- **Contents [R]** (zip directory read by range request, 490 entries): Spinner U5 630,
  "SINUMERIK 840D sL", SIMATIC IPC227E edge device, workpiece "1.2083 (X42Cr 13)", NX 12.0.2.9;
  four operations (face, end, plunge, slot milling), each with STEP CAD, NX strategy reports,
  **toolpath before post (`Toolpath_CLSF_standard/advanced.txt`, `Toolpath_GOTOs.txt`)**, NC after
  post (`.mpf`), NC after edit at the machine (`.MPF`), 500 Hz `*_df.h5` data (94 files), photos.
- **Serves (a)** 840D, **(b)** 500 Hz timing, and an unusual **post check**: CLSF in, MPF out,
  MPF as actually run. NC-SA: fetch only, never store; members can be pulled by HTTP range.

### 3.7 KIT multimodal CNC milling (Ströbel et al., Data in Brief 63:112207, 2025)
- Article [R via WebFetch summary, PMC12621317]: DMC 60 H, SINUMERIK 840D with Edge; Al 2007 T4,
  S235JR, POM-C; NC `.nc` files, STEP, toolpath figures; 500 Hz encoder and commanded positions,
  contour deviation, torque, current; 10 kHz force and acceleration; process duration per trial.
- DataCite [R]: 10.35097/hvvwn1kfwf7qt48z "44,6 GB", tar, `cc-by-4.0`; companion
  10.35097/vnnu3n9z7ndsnhfd (DMG Mori CMX 600V, 54 trials) "55,2 GB". Per-file download on RADAR:
  UNKNOWN. **Serves (a)(b)**, but only if the NC and timing files can be fetched without the tar.

### 3.8 Malyer & Çaydaş, Ti-6Al-4V pocket entries (Zenodo 10.5281/zenodo.23755091 v1.1, 2026-10-29)
- Zip 37 892 409 B, md5 `8736d0c37bdd2b99cde7a02fad2c3200` [R]. README [R]: EL5490 desktop router,
  DSP 0501 controller; Ø4 mm 4-flute AlTiCrN carbide, 3 820 rpm; 8 mm deep quarter-circle pockets
  in 14 levels of 0.571 mm; helical / ramp / plunge entries at 366, 732, 1 098 mm/min; nine NX
  Cavity Mill programs "unaltered as run" (`nc_programs/*.PRG`, 124–151 KB each); every Ra
  reading (Mitutoyo SJ-301); completed entries before tool failure; tool images; Python
  kinematic feed planner (junction deviation, trapezoid, FIR jerk) and z-map. "Controller's own
  look-ahead… not known". Licence: data and NC CC BY 4.0, code MIT [R, LICENSE].
- NC [R]: `O0001`, `G91 G28 Z0.0`, `G43 Z5. H00`, `G03 … I J` (incremental centres), Turkish
  comments. **Serves (a)**, **(d)** Ra and tool life vs entry strategy, **(c)** entry strategies
  to compare with ours; **(b)** only against their simulation, no measured time.

### 3.9 Myhovych (KPI Kyiv), St3ps end milling, three Zenodo sets, all `cc-by-4.0` [R]
- **Cutting force** 10.5281/zenodo.21297035 (15.1 MB, md5 `2fe9abd1…`): four source G-code
  programs with a **feed changed per block** (F122–F673) along lines and R16 arcs, raw dynamometer
  CSV, segments by regime; Ø12 4-tooth HSS (P6M5) TiAlCrN, ae 5 mm, ap 1.5 mm [R, README and
  `f122_to_f404_s740_gcode.txt`]. Force sampling rate: UNKNOWN.
- **Ra, straight** 10.5281/zenodo.21518387 (9.8 MB) and **Ra, R10 arcs** 10.5281/zenodo.21590342
  (12.5 MB): 45 and 30 profilograms, Ra/Rq/Rz, programmed feed and speed [R, descriptions].
- **Serves (d)** (Ra and force vs feed on lines and arcs) and **(a)** small Fanuc-style programs.

### 3.10 RPTU 10-part 3-axis benchmark (Zenodo 10.5281/zenodo.17035762 v1)
- Covered in datasets-parts.md §11. New here [R, `2.ngc` head]: the G-code header reads
  `(MASCHINE)(  HERSTELLER AUTODESK)(  MODEL GENERIC 3-AXIS)`, `G91.1`, `G64 P0.01 Q0.01`,
  `G18 G3 … I K` lead arcs, so it was posted from an **Autodesk CAM machine definition**
  (which Autodesk product: UNKNOWN). NC 3 880 627 B, STEP 429 923 B, PNG 1 027 881 B [R, API].
- **Serves (c)** directly (same STEP, a commercial CAM's path, machined) and **(a)**. Storable.

### 3.11 Other sets read in passing
- **Hannover tool wear** Mendeley 10.17632/zpxs87bjt8.3 [R, API]: 9 cutters to VB ≈ 150 µm,
  cast iron 600-3/S, three 5-axis centres, force 25 kHz, drive data 500 Hz, 6 419 files,
  10.6 GB, CC BY 4.0; no NC listed. (d) only.
- **FFF + hybrid CNC Ra** Mendeley 10.17632/rf7rhwsh4v.2 [R, API]: CC BY 4.0, 437 files,
  620 MB, 64 `.gcode` (mostly printer) and 10 `.nc`, Ra PDF reports. PLA milling; weak (d).
- **MachiningQA-Bench** Zenodo 10.5281/zenodo.24156893 (2026-11-12) [R, API]: 150 Al 7075 blades,
  regional Ra/Rz and profile error, `cc-by-4.0`, but **0 files** listed in the API record;
  access: UNKNOWN.
- **Ward et al. 2021** (arXiv:2102.02062) [R]: DMG Mori eVo 40, Heidenhain TNC640, contour and
  trochoidal pockets from Siemens NX; predicted within 3 % of measured (5.52 % worst) vs CAM
  0.22–54.99 % error; trochoidal 10 µm tolerance 14.40 → 30.99 s. "Data are available… upon
  reasonable request" [R]. **Sun et al.** (arXiv:2106.09719) [R]: Siemens 840D; no public data
  statement found. Not datasets; the tables are sanity numbers for (b).

## 4. Test programs from control makers and standards

- **ISO 10791-7** (M1–M5 test pieces): paid standard; no open NC found [S]. See
  datasets-parts.md §3 [R there]. Plan stays: model our own ISO-style piece and post it ourselves.
- **NAS 979 circle-diamond-square (CDS):** STEP Tools hosts `cds_ccat.238` [800k], "a three axis
  version planned by CCAT", STEP-NC AP238 with stock, tools, roughing and finishing toolpaths
  [R, https://downloads.steptools.com/docs/stepncfiles/cds/]; no licence beyond the site's "All
  Rights Reserved" footer [R]. G-code CDS programs exist in LinuxCNC (`cds.ngc`, GPL-2+) and cncjs
  (`Circle_Diamond_Square_Calibration.gcode`, MIT repo) [R]. The 5-axis cone frustum: no open NC
  found [S].
- **Ballbar test programs:** generated per controller by the ballbar vendor's software [K]; not a
  public dataset. An NC program for a cone-frustum-equivalent ballbar path is noted as hard to make
  in conventional CAM [S].
- **Heidenhain / Siemens / Fanuc samples:** the programming manuals (e.g. TNC 640 Klartext and
  DIN/ISO user's manuals, content.heidenhain.de) contain example programs [S]; vendor terms
  apply [S]. Use for reading the dialect only, write our own tests.

## 5. G-code corpora

- **Hugging Face [R, API 2026-11-24]:** `ablam/gcode` and `r3lax/gcode` are Prusa-sliced **3D
  printing** G-code from Printables (11 GB, no licence tag); `kpoc/FDM-gcodes` FDM, `license:other`;
  `BreyAIrev/cnc-gcode-expert` (cc-by-4.0, <1K) and `cnc-gcode-multi-dialect` (gated, other) are
  instruction→G-code pairs, generated text, not run programs; `jungter/augmented-g-code` (MIT)
  "augmented". None has measured outcomes. Useful at most as (a) parser fuzz input.
- **Slice-100K** (arXiv:2407.04180, NeurIPS 2024 D&B): >100 000 G-code files with STL, FDM only;
  cites an earlier "Greater G-code" set of 860 files [S]. Not milling.
- **GitHub:** no curated milling corpus found; the sender/CAM repos in §2 are the practical corpus.

## 6. Findings that matter for the plan

1. **Cycle time (b) is now testable on real controllers**, open and CC-BY: Spinner U5-620 840D
   logs with line numbers and `Cycle_time_program` (§3.5), plus NIST block timing (§1.2). The
   toolpath-quality README's "honest cycle time" goal can be scored against measured runs, not
   only against Ruckig/CAMotics oracles.
2. **(c) has one clean source:** RPTU's STEP + Autodesk-posted G-code (§3.10). TUGMCL adds NX CLSF
   before post (§3.6) but is NC-SA.
3. **(d) sources with known paths are small**: Malyer Ti pockets (Ra, tool life) and Myhovych
   (Ra, force). PHM 2010, NASA, Hannover give wear with no path.
4. **(a) coverage:** Siemens 840D (Spinner, TUGMCL, KIT), Fanuc-style (Malyer, Myhovych),
   Heidenhain-derived (NIST), RS274NGC (LinuxCNC, RPTU), Grbl/CamBam/Vectric (cncjs, UGS,
   CAMotics). Real Heidenhain Klartext `.h` files with logs: none found open.

## 7. Ranked shortlist (top 8) and fetch plan

Destination root: `~/.local/share/forgecad-bench/gcode/` (cache, outside git); "store" means a
proposed in-repo copy under `verify/forge-cam/fixtures/gcode/<name>/` with licence and NOTICE.
Record SHA-256 on first fetch where only an md5 or commit is given.

| rank | name | URL | pin | dest | size | uses |
|---|---|---|---|---|---|---|
| 1 | spinner-u5 | github.com/ElMoe/Production-Data-Set-for-Five-Axis-CNC-Milling-with-Multiple-Changeovers ; zenodo.org/records/15735480/files/data_v1_0_1.csv | commit `b800366089c196c22c6eb66c69a1e9cfa781a061`; csv md5 `5a758c26…24da` (repo copy SHA-256 `af0d15b5f976455e6f5bd20835f60ea370bcc914b043de4492187a18968514a0`) | NC → store; csv → cache `spinner-u5/` | NC 0.18 MB; csv 78.7 MB | a, b |
| 2 | rptu-10 | zenodo.org/records/17035762 | DOI v1; per-file md5 from API (e.g. `2.ngc` `4220f98e…`) | store `rptu-10/` | 5.3 MB (NC 3.9 MB) | a, c |
| 3 | nist-mtconnectR | cran.r-project.org/src/contrib/Archive/mtconnectR/mtconnectR_1.2.1.tar.gz | tar SHA-256 `06d84654ced6876c4bb9ce59cf3e5d0332af1dec91f60af3e358d6320a348442`; `raw_gcode.NC` `a1d92ee74eed44755fa28bc3cddc129cac6be104346fa02d92ff7eafc5dc9d1d`; `data_truncated.bz2` `6b7843b728d73a03df91afeca61820132923a48849a082a48ae00eb01e82627c` | cache `nist-mtconnectR/` (AGPL) | 0.44 MB | a, b |
| 4 | malyer-ti | zenodo.org/records/23755091/files/Malyer_Caydas_TiPocketMilling_data.zip | v1.1, md5 `8736d0c37bdd2b99cde7a02fad2c3200` | NC + CSVs → store; zip → cache | 37.9 MB (NC 1.26 MB, CSVs 0.02 MB) | a, c, d |
| 5 | linuxcnc-ngc | github.com/LinuxCNC/linuxcnc (sparse: nc_files, tests/trajectory-planner) | commit `65f7467903029174dae789b7f89d5251ccdf8650` | cache `linuxcnc/` | 3.5 + 5.7 MB | a, b-stress |
| 6 | tugmcl | zenodo.org/records/7753181/files/FairMillData.zip | v0.1.0, md5 `a3ac027b1991f7332c49666e9d2e0451` | cache only (NC-SA); pull NC, CLSF and one `.h5` per op by HTTP range | 989 MB whole; members ≲ 15 MB | a, b, c |
| 7 | camotics-ex | github.com/CauldronDevelopmentLLC/CAMotics `examples/` | commit `e84665f2fa9d1151f03282ac7e01320bc65e015b` | cache `camotics/` | 26 MB | a, c |
| 8 | freecad-posttests | github.com/FreeCAD/FreeCAD `src/Mod/CAM/CAMTests` | commit `8fef74c9a1ebb360917e6408daa702c886926f66` | cache `freecad-camtests/` (read, then write our own) | 3.4 MB | a |

Next after these: Myhovych force/Ra (§3.9, CC-BY, 37 MB), cncjs + UGS fixtures (§2), KIT if
per-file access exists (§3.7), NASA mill (§3.2) and PHM 2010 (§3.1) for a future wear model.

**Why this order.** 1 and 3 are the only open sources with a program **and** its measured
timing on an industrial controller; 2 is the only open same-part comparison with a commercial
CAM path; 4 links paths to Ra and tool life; 5–8 widen dialect coverage and post checks.

## References (all read this session unless marked)

1. Catalog record, NIST SMS TDP: https://catalog.data.gov/dataset/technical-data-packages-tdps-from-the-smart-manufacturing-systems-sms-test-bed
2. Hedberg et al. 2019, https://doi.org/10.6028/jres.124.004 ; data DOI 10.18434/T4/1503154 (unreachable)
3. mtconnectR 1.2.1, https://cran.r-project.org/src/contrib/Archive/mtconnectR/
4. FreeCAD @8fef74c9, CAMotics @e84665f2, LinuxCNC @65f74679 (+debian/copyright), grid-apps
   @d138275b, Grbl @bfb67f0 — clones from opensource-oracles.md
5. cncjs @71844227, UGS @76cc42b5, bCNC @24540c5c — blobless clones, https://github.com/{cncjs/cncjs,winder/Universal-G-Code-Sender,vlachoudis/bCNC}
6. PHM 2010: https://phmsociety.org/phm_competition/2010-phm-society-conference-data-challenge/ ;
   https://ieee-dataport.org/documents/2010-phm-society-conference-data-challenge ;
   https://github.com/katulu-io/uniwear-dataset (README)
7. NASA mill: https://catalog.data.gov/dataset/milling-wear ; https://data.nasa.gov/docs/legacy/mill.zip (Readme.pdf)
8. Michigan: mirrors github.com/flyinginte/toolwear_cncmilling, github.com/SaeedShurrab/Tool-Wear-Detection-in-CNC-Milling-Operartions;
   READMEs in github.com/makinarocks/awesome-industrial-machine-datasets and github.com/autonlab/pmx_data
9. Bosch: https://github.com/boschresearch/CNC_Machining ; Tnani et al. https://doi.org/10.1016/j.procir.2022.04.022 (not read)
10. Spinner: https://doi.org/10.5281/zenodo.15735480 ; https://doi.org/10.5281/zenodo.10853254 ; GitHub supplement above
11. TUGMCL: https://doi.org/10.5281/zenodo.7753181
12. KIT: PMC12621317 (https://pmc.ncbi.nlm.nih.gov/articles/PMC12621317/); DataCite 10.35097/hvvwn1kfwf7qt48z, 10.35097/vnnu3n9z7ndsnhfd
13. Malyer & Çaydaş: https://doi.org/10.5281/zenodo.23755091
14. Myhovych: https://doi.org/10.5281/zenodo.21297035 , .21518387 , .21590342
15. RPTU: https://doi.org/10.5281/zenodo.17035762
16. Mendeley API: https://data.mendeley.com/public-api/datasets/zpxs87bjt8 , .../rf7rhwsh4v ; MachiningQA-Bench https://doi.org/10.5281/zenodo.24156893
17. Ward et al. arXiv:2102.02062 ; Sun et al. arXiv:2106.09719
18. STEP Tools CDS: https://downloads.steptools.com/docs/stepncfiles/cds/
19. ISO 10791-7 previews and NAS 979 papers [S]; Slice-100K arXiv:2407.04180 [S]; Heidenhain TNC 640 manuals [S]
20. Hugging Face API: https://huggingface.co/api/datasets?search=gcode (and card READMEs)
