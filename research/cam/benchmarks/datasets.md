# Public benchmark datasets for forgeCAD CAM: parts, toolpaths, G-code

2026-11-24. Research only; no dataset files are in the repo yet. The coordinator's brief (from
David): find public databases of parts and of toolpaths/G-code to benchmark against, with no
sign-ups, paywalls or logins. For each, record its licence and whether we may store copies in the
repo or only fetch them by script with a pinned hash.

Detail and every source are in two notes:
- [datasets-parts.md](datasets-parts.md): part databases.
- [datasets-gcode.md](datasets-gcode.md): toolpath, G-code and machining-process data.

Every claim there is marked [R] (read this session), [S] (search snippet only) or [K] (background
knowledge, not re-read). Anything not read is UNKNOWN. This summary repeats only their
conclusions.

**Store or fetch.**
- **Store in the repo** (licence allows redistribution inside an MPL-2.0 repo, with LICENSE and
  NOTICE files): CC-BY, CC0, MIT, BSD, and US-government public domain. Destinations:
  `verify/cam-parts/<set>/` and `verify/forge-cam/fixtures/gcode/<set>/`.
- **Fetch by script only, with a pinned hash** (cache outside git,
  `~/.local/share/forgecad-bench/<set>/`): GPL, LGPL, AGPL, NC, NC-SA, per-creator rights, or
  unclear licences.

## 1. What each benchmark use needs, and the best open source for it

| use | best open source | licence | store? |
|---|---|---|---|
| **Same part, another CAM's real toolpath** (toolpath-quality comparison) | **RPTU 3-axis benchmark**, Zenodo 10.5281/zenodo.17035762 v1: 10 parts from slots to freeform (part 8 based on ISO 10791-7), STEP AP214 plus G-code run on a machine, posted from an Autodesk machine definition | CC-BY-4.0 | **yes** (5.3 MB) |
| **Cycle-time model against a real controller** | **Spinner U5-620**, Zenodo 15735480 plus a GitHub supplement: Siemens 840D NC posted from NX, with a 1 Hz log of line number, line text and `Cycle_time_program` | CC-BY-4.0 | NC **yes** (0.18 MB); CSV cache (79 MB) |
| Cycle time / block timing (NIST) | A real NIST SMS test-bed program (Mastercam output) with 15 min of MTConnect events at µs time stamps, via the archived R package mtconnectR 1.2.1. smstestbed.nist.gov itself was unreachable (proxy 502, then the host fell outside the allowed network). | AGPL-3 | fetch only |
| **Toolpath → measured Ra / tool life** | Malyer & Çaydaş, Ti-6Al-4V pocket entries (Zenodo 23755091 v1.1): NC as run, every Ra reading, tool life | CC-BY (data), MIT (code) | NC + CSVs **yes** |
| Toolpath → forces / Ra | Myhovych St3ps end milling (three Zenodo sets) | CC-BY-4.0 | yes |
| CAM path before the post, and after edits at the machine | TUGMCL (TU Graz): NX CLSF, the posted 840D program, the program as edited, 500 Hz data | CC-BY-NC-SA-4.0 | fetch only |
| **STEP-import robustness** | **NIST MBE PMI test models** (CTC/FTC/STC, AP203 and AP242). NIST's own README says they contain syntax errors. | US public domain (acknowledgement requested) | yes (14 MB); optionally the AP203 subset only |
| Parity with another CAM's expectations | **FreeCAD CAMTests** (commit 8fef74c9): about 10 FCStd fixtures (shapes as OCCT .brp) plus expected results per operation | LGPL-2.1+ | fetch; read it, then write our own tests |
| Feature recognition / 2.5D coverage at scale | MFCAD++ (59,665 STEP files, 24 classes, 1.5 GB); MFCAD (15,488 STEP files, per-face labels) | CC-BY; MIT | MFCAD++ fetch; an MFCAD 50-part subset may be stored |
| Robustness at scale | ABC (106.7 GB STEP in 100 chunks, md5 per chunk). Rights stay with each creator (Onshape terms). | per creator | fetch 1–2 chunks only |
| Freeform / 3+2 surfacing parts | SimJEB CAD zip (207.6 MB). The licence field says CC0, the description says ODC-By: treat it as ODC-By. | CC0 / ODC-By | fetch |
| Small CI smoke part | NIST AM test artifact (75 KB) | public domain | yes |
| Post/reader dialect coverage | LinuxCNC `nc_files/` (247 programs) + 169 trajectory-planner tests; CAMotics examples (26); cncjs (MIT); UGS arc fixtures | GPL-2+ / GPL-3 / MIT | cncjs **yes**; the rest fetch |

**Not usable, or held:**
- **Fusion 360 Gallery:** custom licence, non-commercial research only, revocable, no full
  redistribution, and it binds employers. Never store it. Whether forgeCAD's use is allowed is
  **David's decision**.
- **DeepCAD** states no data licence. **CADParser** has no working link and no licence.
- **ISO 10791-7 and NAS 979:** no open drawings, models or NC. We model our own. The RPTU part 8
  is an open derivative of ISO 10791-7.
- **STEP Tools AP238 samples** (NAS 979 CDS, Moldy, Fishhead; the only reference 5-axis plans
  found): the site's terms are "All Rights Reserved" / non-commercial.
- **Thingiverse, Printables, GrabCAD, Onshape public documents:** per-model licences, logins,
  or (Onshape §4(b)(9)) a ban on scripted harvesting.
- **Wear data with no G-code:** PHM 2010, NASA/Berkeley mill, Hannover. Only for a future wear
  model.
- **Michigan CNC mill tool wear:** Kaggle needs a login; the GitHub mirrors have no G-code.
- **Ward et al. 2021:** data on request only.
- **Hugging Face "gcode" corpora:** 3D printing or generated text; at most parser fuzz input.

**Correction to datasets-parts.md.** RPTU's "which CAM: UNKNOWN" is now: the G-code headers name
an Autodesk machine definition (an Autodesk post), product still unknown (datasets-gcode.md
§3.10).

**Needs the environment.** The NIST SMS test bed (smstestbed.nist.gov) needs that host allowed in
the environment's network settings. Until then, mtconnectR stands in.

## 2. Ranked shortlist (merged) and fetch plan

| rank | set | what it proves | licence → where | pin | size |
|---|---|---|---|---|---|
| 1 | **RPTU 10-part** | Our toolpaths vs a commercial (Autodesk-posted) path on the same STEP; reader round trip; cycle-time sanity | CC-BY → **store** `verify/cam-parts/rptu-10/` | DOI v1; per-file md5 from the Zenodo API; record SHA-256 at first fetch (datasets-parts.md lists all 10 STEP SHA-256s) | 5.3 MB |
| 2 | **Spinner U5-620** | **Cycle-time model vs measured 840D runs**, per line; 840D dialect reader | CC-BY → NC **store**, CSV cache | GitHub commit `b8003660…`; CSV SHA-256 `af0d15b5…14a0` | 0.18 MB + 79 MB |
| 3 | **NIST PMI STEP** | Import robustness (known-bad syntax), prismatic CAM coverage, hole PMI | public domain → store (or the AP203 subset) | zip SHA-256 `8fa78429…1911` | 14 MB |
| 4 | **Malyer & Çaydaş Ti pockets** | Path → Ra and tool-life model validation; Fanuc-style reader | CC-BY → NC + CSV **store** | v1.1, md5 `8736d0c3…` | 1.3 MB stored (37.9 MB zip) |
| 5 | **NIST mtconnectR** | Block timing on a NIST test-bed machine; Heidenhain-derived dialect | AGPL-3 → cache | tar SHA-256 `06d84654…8442` | 0.44 MB |
| 6 | **FreeCAD CAMTests** | Parity with FreeCAD's per-op expectations (adaptive, pocket, profile, drill, helix, thread, vcarve, dogbone) | LGPL → cache; write our own | commit `8fef74c9` | 0.6–3.4 MB |
| 7 | **MFCAD / MFCAD++** | Feature recognition; many varied 2.5D parts for G1–G6 certification at scale | MIT (subset store) / CC-BY (fetch) | MFCAD commit `ef6d58a4`; MFCAD++ archive hash at first fetch | 1.5 GB |
| 8 | **LinuxCNC nc_files + TP tests; CAMotics examples** | Reader coverage on real programs; trajectory-planner stress for cycle time | GPL → cache | LinuxCNC `65f74679`; CAMotics `e84665f2` | about 35 MB |

Next: TUGMCL (NC-SA, by HTTP range: CLSF before post plus measured data), SimJEB, one ABC chunk,
Myhovych, cncjs (MIT, store), NIST AM artifact (store, CI smoke).

**Fetch mechanics** (both notes §7/§15):
- A `tools/bench/fetch.sh` reads a per-set manifest of URL, pin and SHA-256.
- It downloads into the cache.
- It verifies the hash and refuses on a mismatch.
- It never commits anything.
- Stored sets go in with their LICENSE and NOTICE files (attribution for CC-BY; acknowledgement
  for NIST).
