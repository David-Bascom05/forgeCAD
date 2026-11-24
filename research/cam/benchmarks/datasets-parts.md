# Public part databases for benchmarking forgeCAD's CAM and geometry

2026-11-24. Research only. No dataset file was added to the repo, and no code was changed. This is
for the owner and for the toolpath-quality work
([../toolpath-quality/README.md](../toolpath-quality/README.md) §benchmarks).

**What it answers.** Which public part sets can test three things:

- **Geometry robustness:** STEP import, booleans, validity.
- **CAM coverage:** the features there are to machine.
- **Toolpath comparison:** reference G-code or a CAM plan to compare our paths against.

For each set it gives the contents, the licence, the size, and whether a copy may be **stored** in
this MPL-2.0 repo or should only be **fetched by script** with a pin.

**Marks.**

- **[R]**: read this session, from the page or file named.
- **[S]**: search snippet only.
- **[K]**: background knowledge, not re-read.
- **UNKNOWN**: not read. No guess is given.

Licence readings are mine and are not legal advice.

**Store or fetch: the rule used here.**

- **Store** (a few files under `verify/…`, with the licence text and a NOTICE beside them) only
  when the set is all of these:
  - public domain, CC0, CC-BY, MIT, or LGPL kept as separate files;
  - free of any non-commercial or research-only clause;
  - small.
- **Fetch by script** for everything else: with a pinned SHA-256, git commit or DOI version, into a
  cache outside git.
- **Precedent in this repo [R]:** research/find-parts/README.md §"Data and licences" says "No
  dataset file enters this repo. Downloads live in `~/.local/share/forgecad-find-parts/`". This
  note keeps that default and names the few sets where storing would be allowed if the owner wants
  them in CI.

---

## 1. NIST MBE PMI test models (CTC, FTC, STC, HTC, MTC)

**Source [R]:**
https://www.nist.gov/ctl/smart-connected-systems-division/smart-connected-manufacturing-systems-group/mbe-pmi-0

**Contents [R]:**

| set | what |
|---|---|
| CTC | 5 Combined Test Cases, CTC-01 to 05. CTC 02 and 04 form an assembly. |
| FTC | 6 Fully-Toleranced Test Cases, FTC-06 to 11. FTC 07 to 10 form an assembly. |
| STC | 5 Simplified Test Cases: FTC 6 to 10 with less complex PMI and no datum targets. |
| HTC | Hole test cases: counterbore, countersink, through and depth holes. Native CAD only. |
| MTC | A 4-part box assembly, from FTC-07, 08 and 09. Native CAD only. |

**STEP zip (read and hashed this session) [R]:**

- **Link:** `https://www.nist.gov/document/nist-pmi-step-files`, which redirects to
  `.../system/files/documents/noindex/2024/06/19/NIST-PMI-STEP-Files.zip`.
- **Size:** 13,976,599 bytes packed and 56.7 MB unpacked, in 49 entries.
- **SHA-256:** `8fa78429e6d8d9b0d7681d223b6aa9ec98c3772185c55b1a0e3679b21c181911`.
- **Files:**
  - AP203 geometry-only: 11 files, CTC 01-05 and FTC 06-11.
  - AP203 with graphical PMI: CTC 01-05.
  - AP242: CTC and FTC in editions e1 and e2, and STC in e3. FTC-08 has an extra `-tg` file with
    tessellated geometry.
  - PDF drawings.
- **README in the zip:** "These are NOT reference STEP files without any errors. The STEP File
  Analyzer and Viewer will report some Syntax Errors."
- **Why that is useful:** it is a ready robustness set of real exporter output from several CAD
  systems.
- **Also:** the CTC definitions zip, `nist-ctc-test-case-definitions`, is 5,464,750 bytes and holds
  20 files: PDFs and a STEP of each CTC. SHA-256
  `79ebc74d99eb05c0d23de1c1d8730651fa2a24def56f647bac21dd439ec40690` [R]. The FTC/CTC native CAD
  zip is 60,294,624 bytes [R, HEAD only].

**Licence [R]:**

- The page names no SPDX identifier. It says: "The test cases, CAD models, and STEP files can be
  used without any restrictions."
- It also says: "We would appreciate acknowledgement…" The NIST logo may not be used.
- NIST's general policy (https://www.nist.gov/open/license) [R]: works of NIST employees are "not
  subject to copyright protection in the United States", "may be subject to foreign copyright",
  and NIST asks to be acknowledged as the source.
- Closest SPDX: none exact. Treat as US-public-domain with an acknowledgement request
  (`LicenseRef-NIST-PD`).

**Store?** Yes. The geometry-only AP203 set is about 3.2 MB, so it could sit in `verify/` with a
NOTICE crediting NIST. The full zip is better fetched.

**Serves:**

- **Geometry robustness:** high. These are real multi-vendor STEP files with known syntax faults.
- **CAM coverage:** high for prismatic work. They have pockets, bosses, hole patterns, slots,
  counterbores and fillets, and they are machinable parts [K].
- **Toolpath comparison:** none, as they ship no NC.
- **Hole recognition:** HTC and the PMI give a check against stated hole types and tolerances.
- **Earlier note [R]:** research/find-parts/README.md row 6 already lists NIST PMI as "import and
  validity only".

## 2. NIST manufacturing test artifacts

### 2a. Five-axis test pieces (NIST, 2011)

- **Paper [R]:** "Evaluation of Proposed Test Artifacts for Five-Axis Machine Tools" (Moylan,
  Blumenfeld, McGlauflin, Fesperman, Donmez, 2011), https://www.nist.gov/node/594146.
- **What it covers [R]:** a cone frustum and a truncated square pyramid, proposed for the ISO
  five-axis standard.
- **Files:** the page offers the publication only. It gives no dimensions and no CAD.
- **Licence:** the paper is a NIST work, so the US-public-domain policy above applies [R, policy].
- **Use:** we model the pieces ourselves from published dimensions. Those dimensions are UNKNOWN
  until the PDF is read.

### 2b. NAS 979 circle-diamond-square

- **No free NAS 979 drawing or model was found** [S]. NAS 979 is an AIA standard [K].
- **The one open copy found is STEP Tools' STEP-NC sample:**
  - Page [R]: https://www.steptools.com/docs/stepncfiles. The directory
    https://downloads.steptools.com/docs/stepncfiles/cds/ describes "a NAS 979
    circle/diamond/square with an inverted NAS 979 cone test in the center".
  - It "includes geometry for starting piece, final piece, and tools, toolpaths for roughing,
    finishing, and tool parameters" [R].
  - The file is `cds_ccat.238`: 812,174 bytes, Last-Modified 2009-02-14, "a three axis version
    planned by CCAT" [R].
  - The first bytes show a zip holding `iso_10303_28.xml` [R], so it is AP238 in the
    ISO 10303-28 XML encoding.
- **Other samples on the same page [R]:** "Moldy" mold part (3- and 5-axis), "Boxy" gear box,
  titanium impeller (5-axis), and "Fishhead" aerospace part from Airbus (5-axis; three .238 files of
  0.7 to 1.3 MB).
- **Licence [R]:** none is stated on the sample pages. The site copyright page
  (https://www.steptools.com/copyright.html) authorises copying of "the document… for
  informational, non-commercial purposes only", with the notice kept and the right revocable.
  Whether that covers the data files: UNKNOWN.
- **Store?** No. Fetch by script at most, and ask STEP Tools before any wider use.
- **Serves:** toolpath comparison. These are the only open sets found with a **reference CAM plan
  and toolpaths together with the geometry** for classic machine-test parts. Reading them needs an
  AP238 reader, which is real work.

### 2c. NIST additive manufacturing test artifact

- **Page [R]:** https://www.nist.gov/node/438556.
- **Files:** STL, AMF and STEP. The STEP comes as `nist_test_artifact_ver3b.zip`: 74,911 bytes,
  holding `NIST Test Artifact_ver3b.step` of 564,884 bytes. SHA-256 of the zip:
  `ee18b7ff5051614d58eca15fa1aafccbdde493648b52bb3e22cca39d7cf77c37` [R].
- **Licence [R]:** "these files are not subject to copyright protection and are in the public
  domain". Acknowledgement is requested.
- **Store?** Yes. It is tiny and public domain.
- **Serves:**
  - Geometry robustness: one prismatic part with many small features: pins, holes, a staircase,
    fine features [K, from the paper's description].
  - CAM coverage: a dense small-feature 2.5D part.
  - It is not a machining test. It is relevant as geometry only.

### 2d. NIST Smart Manufacturing Systems (SMS) Test Bed data

**Technical Data Packages [R]:**

- **Catalog record:**
  https://catalog.data.gov/dataset/technical-data-packages-tdps-from-the-smart-manufacturing-systems-sms-test-bed
- **Formats:** "CAD, CAM, TXT, XML".
- **Licence:** https://www.nist.gov/open/license.

**The MTC three-component assembly dataset:**

- **Paper [R]:** Hedberg et al., J. Res. NIST 124:124004 (2019),
  https://doi.org/10.6028/jres.124.004. Data DOI 10.18434/T4/1503154.
- **Contents [R]:**
  - The part definitions are SolidWorks 2016 native, per ASME Y14.41.
  - "NC-Code: machining programs… in accordance with ISO 6983".
  - MTConnect logs, raw and split by operation.
  - QIF 2.1 inspection results, "20 instances of the same part" per file.
- **Licence:** the paper points to https://www.nist.gov/director/licensing (not read).
- **Access:** **unreachable.** https://smstestbed.nist.gov/tdp/ and the data DOI both returned
  **HTTP 502 from the proxy** this session. data.nist.gov also returned 502.
- **Serves, if reached:** toolpath comparison against real NC programs that were run, with
  measured results. The CAD is SolidWorks native, so it is not readable by us without a STEP
  export. Whether a STEP export is in the package: UNKNOWN.

## 3. ISO 10791-7 test pieces

- **The standard is paid** [S]: CHF 181 at iso.org.
- **From the free iTeh preview of the 2020 edition [R]**
  (https://cdn.standards.iteh.ai/samples/73814/…/ISO-10791-7-2020.pdf), Table 1 lists five types:

| type | name | sizes |
|---|---|---|
| M1 | positioning and contouring | 80, 160, 320 |
| M2 | face milling | 80, 160 |
| M3 | cone frustum | half-apex 15° or 45° |
| M4 | three-step square | 80, 160, 320 |
| M5 | freeform (Annex A) | one size |

- **Which machines each type is for [R]:**
  - M1 and M2 are for 3- to 5-axis machines.
  - M3 and M5 are for 5-axis machines only.
  - M4 is for 4- and 5-axis machines.
- **What is missing:** the preview stops before the drawings, so the dimensions are UNKNOWN.
- **No openly licensed model of any ISO 10791-7 piece was found** [S, two searches].
- **One open derivative was found and read:** part 8 of the RPTU Zenodo set (§11) "extends the
  benchmark part introduced by Mertes et al. (2024)… which is based on ISO 10791-7" [R]. Mertes
  et al. is https://doi.org/10.1016/j.dte.2024.100024 (not read).
- **Plan (unchanged from toolpath-quality §benchmarks):** model an "ISO 10791-7-style" piece
  ourselves as `.fg`, with our own numbers. Do not copy the drawings.

## 4. FreeCAD CAM test models (src/Mod/CAM/CAMTests)

- **Read [R]** from a blobless clone of https://github.com/FreeCAD/FreeCAD at commit
  `8fef74c9a1ebb360917e6408daa702c886926f66` (2026-11-23).
- **Size of the folder:** 161 files, mostly Python tests (`TestPathAdaptive.py`,
  `TestPathPocket.py`, `TestPathProfile.py`, `TestPathDrilling.py`, `TestPathHelix.py`,
  `TestPathThreadMilling.py`, `TestPathVcarve.py`, `TestPathDressupDogboneII.py`, post tests, and
  others).
- **Geometry fixtures [R]:**

| file | bytes | objects inside |
|---|---|---|
| Drilling_1.FCStd | 266,675 | PartDesign body: 3 pads, 12 pockets, linear and polar patterns |
| test_geomop.fcstd | 79,058 | 7 bodies, boxes and cuts |
| Fixtures/test_28534_truncated_pocket.FCStd | 46,187 | regression fixture |
| Fixtures/test_17748_cam_profile.FCStd | 43,077 | regression fixture |
| dressuptest.FCStd | 41,788 | not inspected |
| boxtest.fcstd | 39,033 | a Part::Box with 12 Path objects, a full job |
| Fixtures/OpHelix_v0-21.FCStd | 30,107 | not inspected |
| test_adaptive.fcstd | 20,661 | 3 boxes, a cut and a multi-fuse |
| test_holes00.fcstd | 17,635 | one body: a sketch and a pad |
| test_profile.fcstd | 17,143 | one body: a sketch and a pad |

- **Also in the folder:** tool-bit, tool-library and tool-shape fixtures, and MTConnect probe XMLs.
- **There is no STEP file in CAMTests [R].**
- **How the shapes are stored [R]:** each FCStd is a zip whose shapes are OCCT `.brp` files (for
  example test_adaptive has 5). To use them we need either an OCCT BREP reader, or a one-time
  STEP export done in FreeCAD. An exported STEP is a derived file, still LGPL.
- **Licence [R]:** the root LICENSE is LGPL-2.1, and the test sources carry
  `SPDX-License-Identifier: LGPL-2.1-or-later`. A per-file licence on the `.fcstd` binaries:
  UNKNOWN. They fall under the project licence [K].
- **Store?** Allowed as separate files under LGPL-2.1-or-later with the licence text. MPL-2.0 sits
  with other licences in a "Larger Work" [K]. Fetching is simpler: pin the commit, about 0.6 MB.
- **Serves:**
  - CAM coverage and parity: the Python tests state the expected results of FreeCAD's operations
    on these shapes. That gives a behaviour baseline per operation: adaptive, pocket, profile,
    drilling, helix, thread milling, vcarve, dogbone.
  - Geometry robustness: low. The shapes are simple.

## 5. MFCAD (Cao et al. 2020)

- **Read [R]** from https://github.com/hducg/MFCAD at commit
  `ef6d58a40164d5192666821ce98d0cc90e379fac` (2020-10-25).
- **Contents:** `dataset/step` holds **15,488 `.step` files and 15,488 `.face_truth` per-face
  label files**. Names look like `0-0-0-0-0-23.step`. There are also pythonocc scripts in `occ/`.
- **Feature classes:** 16 classes [K], not read. The README does not list them.
- **Origin:** synthetic, generated as a stock block with features cut in [K].
- **Licence [R]:** MIT (LICENSE, "Copyright (c) 2020 hducg"). SPDX: `MIT`.
- **Size:** UNKNOWN. Listing blob sizes forces a full fetch, which I stopped.
- **Store?** Yes for a sampled subset (say 50 parts), with the MIT notice. Fetch the whole set by
  commit.
- **Serves:**
  - Feature recognition: per-face ground truth.
  - CAM coverage: synthetic 2.5D prismatic features.
  - Geometry robustness: some, as volume. The pythonocc output is uniform.

## 6. MFCAD++ (Colligan et al., Hierarchical CADNet)

- **Record [R]:**
  https://pure.qub.ac.uk/en/datasets/mfcad-dataset-dataset-for-paper-hierarchical-cadnet-learning-from/
- **DOI [R]:** 10.17034/d1fec5a0-8c10-4630-b02e-b92dc81df823.
- **Contents [R]:**
  - 59,665 STEP models: 41,766 train, 8,950 validation, 8,949 test.
  - The models were generated with PythonOCC.
  - Per-face labels are in `feature_labels.txt`, and HDF5 graphs are included.
  - The record does not list the class set. A paper snippet says 24 machining feature types, with
    planar and non-planar faces [S].
- **Size [R]:** 1.5 GB zip, `https://pure.qub.ac.uk/files/278385243/MFCAD_dataset.zip`. A HEAD
  request from here got **HTTP 403**, so it was not fetched.
- **Licence [R]:** "Licence: CC BY". The version is not given; assume 4.0 and verify. SPDX:
  `CC-BY-4.0`, unconfirmed.
- **Store?** The licence allows it with attribution, but the size does not. Store a small subset
  only. Fetch the rest with a pinned SHA-256 computed on first download.
- **Serves:** feature recognition (the main open benchmark), CAM coverage (24 classes, including
  non-planar), and geometry robustness at volume.

## 7. FeatureNet machining-feature set (Zhang, Jaiswal, Rai 2018)

- **Read [R]** from https://github.com/madlabub/Machining-feature-dataset at commit `a0613216…`
  (2018-06-08).
- **Contents:** 24 classes with 1,000 STL each, in `dataset.rar`, with parameters in
  FeatureList.pdf.
- **Licence [R]:** GPL-3.0 (LICENSE). SPDX: `GPL-3.0-only` or `-or-later`; which one is UNKNOWN.
- **Store?** No. GPL data inside an MPL repo is a needless mixed-licence question. Fetch only.
- **Serves:** little. These are meshes, not B-reps, with one feature per part. Low priority.

## 8. ABC dataset (Koch et al., CVPR 2019)

- **Site [R]:** https://deep-geometry.github.io/abc-dataset/. The abstract says "one million
  CAD models" [R, arXiv 1812.06216].
- **Formats per model [R]:**
  - always present: meta YAML, STEP ("converted from Parasolid"), Parasolid, STL;
  - sometimes present: OBJ with normals, feature YAML, FeatureScript, statistics, images.
- **Packaging [R]:** chunks of 10,000 models per format, in 100 STEP chunks.
- **Size [R], summed from `size.yml`:**
  - STEP 106.7 GB; Parasolid 197.5 GB; OBJ 635.8 GB.
  - The first STEP chunk `abc_0000_step_v00.7z` is 1,594,129,754 bytes, md5
    `695388be7a278c7798c8c8ae239772ac` (from `md5.yml`).
  - Chunk URLs are on archive.nyu.edu, for example
    `https://archive.nyu.edu/rest/bitstreams/88598/retrieve`.
- **Licence [R]:**
  - The site says "The copyright of the CAD models is owned by their creators" and points to
    Onshape's Terms of Use 1.g.ii. The only stated term is no warranty.
  - The GitHub repo (commit `39278911…`) is MIT, but that covers the repo text, not the models [R].
- **Onshape §1.g.ii [R]** (https://www.onshape.com/en/legal/terms-of-use):
  - Public documents of Free Plan users created on or after 2018-08-07, or ones without a LICENSE
    tab, carry a worldwide royalty-free licence "to use, copy, modify, merge, publish, distribute,
    sublicense, and/or sell copies".
  - Older documents, or those of paid users, follow their LICENSE tab.
  - Which ABC models fall under which case: UNKNOWN per model.
- **Store?** No. Fetch one or two STEP chunks by script, pinned by md5 plus our own SHA-256.
- **Serves:** geometry robustness at scale (the best STEP-import stress set), and CAM coverage by
  sampling. The models are not machining-curated.

## 9. Fusion 360 Gallery dataset (Autodesk)

- **Read [R]:** https://github.com/AutodeskAILab/Fusion360GalleryDataset at commit `1084b881…`,
  including LICENSE.md ("Updated 11/2021").

**Contents [R]:**

| set | contents | archive |
|---|---|---|
| Reconstruction r1.0.1 | 8,625 sketch-and-extrude sequences: JSON, SMT, STEP, OBJ, PNG; split 6,900 train, 1,725 test | 2,103,449,157 B (HEAD) |
| Segmentation s2.0.1 | 35,680 parts: STEP, SMT, per-face `.seg`, OBJ, points; 8 classes (ExtrudeSide, ExtrudeEnd, CutSide, CutEnd, Fillet, Chamfer, RevolveSide, RevolveEnd) | 3.1 GB |
| Segmentation extended STEP | 42,912 STEP | 506,333,119 B (HEAD; ETag `7230e2a9…-61`) |
| Assembly | 8,251 assemblies, 154,468 parts | no single archive |
| Joint | 32,148 joints | 2.8 GB |

**Licence [R], custom, no SPDX:**

- Item 1: use "only for non-commercial research purposes".
- Item 2: no redistribution of the whole set.
- Item 3: portions may be passed on only to non-commercial research users, with conditions passed
  down.
- Item 8: indemnity.
- Item 9: Autodesk "reserves the right to terminate this license at any time".
- Item 11: "If you are employed by a for-profit, commercial entity, your employer shall also be
  bound."

**Store?** **No, never.** Fetch by script only, into a cache outside git.

- Whether forgeCAD's use counts as "non-commercial research" is the **owner's decision**, and it
  is UNKNOWN here. forgeCAD is a personal MPL-2.0 project, but the licence looks at the purpose of
  use, not at the repo licence.
- The repo already uses the reconstruction set this way (research/find-parts/README.md: 91.3%
  build rate on 8,625; repros kept in a separate gate repo) [R].

**Serves:**

- Geometry robustness: the extended STEP set is real user parts with fillets and chamfers.
- Feature recognition: the segmentation labels give per-face feature-type truth.
- CAM coverage: by sampling.

## 10. DeepCAD, CADParser, SimJEB

**DeepCAD [R]** (https://github.com/ChrisWu1997/DeepCAD, commit `f7b35079…`):

- **Contents:** "178,238 models and their CAD construction sequences" (arXiv 2105.09492 abstract),
  as Onshape JSON plus vectors. They are sketch-and-extrude only [K].
- **Archive:** `http://www.cs.columbia.edu/cg/deepcad/data.tar`, 208,074,561 bytes, ETag
  `c66f741-5c8dfdc23c228`, 2021-08-06 [R, HEAD].
- **Licence:** the code is MIT [R]. **No data licence is stated** [R]. The data derives from
  Onshape via ABC, so the same per-creator question as ABC applies.
- **Store?** No. Fetch only; the repo already does.
- **Serves:** boolean and extrude robustness through `.fg` translation, as find-parts does. It
  adds nothing for CAM over Fusion reconstruction.

**CADParser** (Zhou et al., IJCAI 2023) [R, https://www.ijcai.org/proceedings/2023/200]:

- **Contents:** "approximately 40,000 models", from about 10k templates plus augmentation, with
  revolve, fillet and chamfer operations.
- **Data:** the link given is "https://drive.google.com/CADParserData", but the paper also says
  "plan to make it publicly available shortly".
- **Licence:** none stated.
- **Access:** Google Drive, not tried. Treat as **unavailable and unlicensed**.

**SimJEB** (Whalen, Beyene, Mueller, CGF 2021):

- **Dataverse record [R]:** https://dataverse.harvard.edu/dataset.xhtml?persistentId=doi:10.7910/DVN/XFUWJG,
  version 4.0, released 2023-10-18.
- **Licence field [R]:** **CC0-1.0** (SPDX given in the API). The description text also says
  "Open Data Commons Attribution License", so the record is inconsistent; attribute anyway.
- **Count:** 381 brackets from the GrabCAD Jet Engine Bracket Challenge [S].
- **Files [R], 19 in all, about 29.3 GB:**
  - `SimJEB_cad_(stp).zip`: 207,566,478 B, md5 `19cdb0f4ca8ff51c367ce0238c332622`, file id
    4639266.
  - Uncleaned CAD: 732 MB.
  - OBJ surface meshes: 441 MB.
  - VTK volume meshes, FEM decks, and CSV/H3D results.
- **A note on rights:** the designs began as GrabCAD challenge entries. The authors released them
  as CC0, but whether the original entrants granted that: UNKNOWN.
- **Store?** CC0 allows it, but keep it fetch-only for the size and the provenance question.
- **Serves:**
  - Geometry robustness: organic and topology-optimised-looking brackets with many blends [S].
  - CAM coverage: 3-axis surfacing and multi-setup work on real-looking shapes.

## 11. Academic machining benchmark part sets

**RPTU 3-axis benchmark: the best find** [R, Zenodo API, https://zenodo.org/records/17035762]:

- **Title:** "Benchmark Dataset of 10 Multi-Feature Models for Single-Setup 3-Axis Milling".
- **Authors:** Schmitz, Mertes, Schillinger, Wagner (RPTU Kaiserslautern-Landau).
- **DOI and version:** 10.5281/zenodo.17035762, v1, published 2025-09-02.
- **Licence:** `cc-by-4.0`.
- **Contents:** 10 parts from an 80 × 80 × 50 mm block, one end mill of at most 6 mm, no
  reclamping. Each part has a `.step`, a `.ngc` G-code program and a `.png` photo of the machined
  part. 30 files, 5,338,431 bytes.
- **Features by part:**

| part | features |
|---|---|
| 1 | slots |
| 2 | 4 holes |
| 3 | 4 holes, rectangular step |
| 4 | 4 holes, circular step |
| 5 | 4 holes, semi-circular and semi-rectangular step |
| 6 | 4 holes, rectangular step, circular pocket |
| 7 | square pocket with a negative freeform |
| 8 | the Mertes et al. ISO 10791-7-based part |
| 9 | 4 holes, circular pocket, several freeforms |
| 10 | everything above, combined |

- **STEP files:** AP214 (`AUTOMOTIVE_DESIGN`) [R], 14 to 146 KB each.
- **G-code:** LinuxCNC-style. It uses `G64 P0.01 Q0.01`, `G43 H1`, `G54`, and tool comments like
  "T1 D=6. CR=0. - ZMIN=-7." [R, 1.ngc head]. The NC files are 7 KB to 1.8 MB. Which CAM wrote
  them: UNKNOWN.
- **The authors say [R]:** the parts "were also manufactured on a three-axis CNC machine tool".
- **Store?** **Yes.** It is CC-BY-4.0, about 5.3 MB in total, or 0.43 MB for the STEP files only.
- **Serves:** all three uses at once:
  - CAM coverage, from simple to freeform;
  - toolpath comparison against real, machined G-code;
  - geometry robustness, to a modest degree.
- **It closes the "no reference toolpaths" gap** in toolpath-quality §5 for 3-axis work, without
  needing Fusion.

**AMRC [R]** (figshare API, https://doi.org/10.15131/shef.data.28631333.v1):

- "AMRC Machining Group Technical Reference Pack - CAD" is a single 3.1 MB PDF.
- Licence: **CC-BY-NC-ND-4.0**.
- It is not a part set. Not usable for storing or deriving.

**Georgia Tech** "3D Manufacturing Feature Dataset in STL and STEP Format" [S]:

- Record: https://repository.gatech.edu/entities/publication/b6597f76-4103-451e-be7d-aa557a123e24.
- The page returned **HTTP 403**, so its contents and licence are UNKNOWN.

**Other leads found but not read** [S]:

- HybridCAD++ (Mendeley Data, m966nfzrhc): hybrid additive-subtractive feature recognition, 161k
  models; licence not seen.
- A Durham CC BY 4.0 deposit tied to a 2023 STEP feature-recognition paper.

**Mold and die:**

- No open "mold and die benchmark" part set was found [S].
- The nearest are STEP Tools' "Moldy" AP238 sample (§2b, non-commercial terms) and the 1999
  Thiebaut, Lartigue and Duc free-form test-part paper (geometry not seen) [S].

**Tormach and Haas sample parts:**

- Tormach's project library points to Vectric files for routers [S]. No licence was seen.
- Nothing was found for Haas.
- Treat both as UNKNOWN and not usable.

## 12. Thingiverse, Printables, GrabCAD, Onshape public documents

**Thingiverse:**

- The legal page https://www.thingiverse.com/legal returned **HTTP 403**.
- The search snippets say [S]:
  - the uploader picks a licence per model, often a CC variant;
  - an older version of the terms limited downloads to "personal, non-commercial use";
  - the 2016 mass-scrape dispute was called a terms violation.
- **Verdict:** per-model licences, mostly meshes (STL), and scraping is contested. Single,
  hand-picked models under CC-BY or CC0 could be fetched with the model's own licence recorded.
  Not a benchmark source.

**Printables:**

- The terms page https://www.printables.com/terms-of-use returned **HTTP 403**.
- The snippets say [S]: CC variants with a "no-commercial" flag, plus Prusa's OCL v1.1 and CERN
  OHL-S v2. Licences are per model, and the models are mostly meshes.
- **Verdict:** same as Thingiverse.

**GrabCAD:**

- Downloads need a login (the brief excludes that).
- Help pages say [S] models are "generally intended for private use", and commercial use needs the
  creator's consent.
- **Verdict:** excluded.

**Onshape public documents:**

- §1.g.ii.2 [R] grants an MIT-like licence for Free-Plan documents made after 2018-08-07. Other
  documents follow their LICENSE tab.
- But §4(b)(9) [R] forbids "any robot, spider, scraper or other automated means to access the
  Service" and "any data mining, data gathering or extraction method".
- **Verdict:** a hand-picked document with a known licence date may be copied by hand. A scripted
  harvest is **not allowed**. ABC, DeepCAD and CADFS are the already-harvested route, each with
  the per-creator caveat.

---

## 13. Summary table

| # | set | parts | format | licence (SPDX) | store? | robustness | CAM coverage | toolpath compare | feature truth |
|---|---|---|---|---|---|---|---|---|---|
| 1 | NIST PMI CTC/FTC/STC | 16 (+HTC, MTC) | STEP AP203/242, native | US PD, ack. asked | yes (small subset) | high | high (prismatic) | – | holes/PMI |
| 2a | NIST 5-axis artifacts | 2 (paper) | none | US PD | – (we model) | – | 5-axis | – | – |
| 2b | STEP Tools CDS/Moldy/Fishhead | 5 | AP238 (XML zip) | non-commercial (site) | no | low | 3/5-axis | **yes** | – |
| 2c | NIST AM artifact | 1 | STEP, STL, AMF | public domain | yes | med | 2.5D small features | – | – |
| 2d | NIST SMS MTC data | 3 parts | SolidWorks, ISO 6983 NC | NIST policy | n/a | – | – | yes (unreachable) | – |
| 3 | ISO 10791-7 | 5 types | paid | © ISO | no | – | yes (we model) | – | – |
| 4 | FreeCAD CAMTests | ~10 docs | FCStd (OCCT BREP) | LGPL-2.1-or-later | allowed; fetch | low | op parity | expectations | – |
| 5 | MFCAD | 15,488 | STEP + labels | MIT | subset | med | 2.5D synthetic | – | **yes** |
| 6 | MFCAD++ | 59,665 | STEP + labels | CC-BY (4.0?) | subset | med | 24 classes | – | **yes** |
| 7 | FeatureNet | 24,000 | STL | GPL-3.0 | no | – | low | – | per part |
| 8 | ABC | ~1M | STEP, Parasolid, mesh | per creator / Onshape | no | **highest** | sampled | – | – |
| 9 | Fusion 360 Gallery | 8,625 / 35,680 / 42,912 | STEP, SMT, JSON | custom NC-research | **never** | high | sampled | – | 8 classes |
| 10 | DeepCAD / CADParser / SimJEB | 178k / 40k / 381 | JSON / ? / STEP | none / none / CC0-1.0 | no / – / fetch | med / – / high | – / – / surfacing | – | – |
| 11 | RPTU 3-axis (Zenodo) | 10 | STEP AP214 + .ngc | CC-BY-4.0 | **yes** | low-med | **2.5D→freeform** | **yes, machined** | features listed |

## 14. Ranked shortlist (top 8)

1. **RPTU 3-axis benchmark (Zenodo 17035762).**
   - It is the only open set read here that has STEP, real G-code that was run on a machine, and a
     licence (CC-BY-4.0) that allows storing it.
   - The parts step up from slots to freeform, so they map straight onto B1-B7 and give a
     reference toolpath for every op class we ship.
2. **NIST PMI STEP files.**
   - Real exporter output from several CAD systems, flagged by NIST as containing syntax errors:
     the best small STEP-import robustness set.
   - Prismatic and machinable, with semantic PMI for hole checks.
   - US public domain.
3. **FreeCAD CAMTests.**
   - The only open set that comes with another CAM's stated expectations per operation (adaptive,
     pocket, profile, drill, helix, thread, vcarve, dogbone). It gives us parity tests against
     FreeCAD.
   - LGPL, tiny, pinned by commit.
4. **MFCAD++.**
   - Largest feature-labelled B-rep set, CC-BY, 24 classes including non-planar. It is the
     feature-recognition benchmark and a source of varied 2.5D CAM stock.
5. **MFCAD.**
   - MIT and in git, so pinning is trivial and a subset can be stored for CI. It is the smaller
     sibling of MFCAD++, with per-face truth.
6. **ABC (one or two STEP chunks).**
   - Real Parasolid-to-STEP models at scale; the broadest robustness stress.
   - Fetch-only, because rights are per creator.
7. **SimJEB CAD (STEP zip).**
   - CC0 field, 208 MB, organic real-world brackets. It covers freeform 3-axis and 3+2 surfacing,
     which the prismatic sets lack.
8. **NIST AM test artifact.**
   - Public domain, 75 KB, a dense small-feature part. It is a cheap CI smoke test for import and
     2.5D coverage.

**Held back:**

- **Fusion 360 Gallery segmentation STEP** would rank about 6 on technical merit: real parts,
  fillet and chamfer truth. Its licence is non-commercial research only, revocable, and binds
  employers, so the owner decides first.
- **STEP Tools AP238 samples** are the only reference 5-axis plans found. They need an AP238
  reader and permission from STEP Tools.

## 15. Fetch plan

**Cache:** `~/.local/share/forgecad-parts/<set>/`, outside git. This mirrors find-parts'
`~/.local/share/forgecad-find-parts/`. Each set gets a manifest `tools/parts/<set>.toml` listing
the URL, the pin and the expected SHA-256 for every file. A fetch script (not written) refuses any
hash mismatch.

**In-repo copies** go in `verify/cam-parts/<set>/` with `LICENSE` and `NOTICE`, and only where §13
says "yes".

| # | set | exact URL | pin | where | size |
|---|---|---|---|---|---|
| 1 | RPTU 3-axis | `https://zenodo.org/records/17035762/files/{1..10}.{step,ngc,png}?download=1` | DOI 10.5281/zenodo.17035762 v1. Per-file md5 from the Zenodo API. SHA-256 of the STEP files computed this session, e.g. 1.step `8c214289…f7c349`, 8.step `bb7f3809…822bb7`, 10.step `91c23f1f…bd01ab` | **`verify/cam-parts/rptu-3axis/`** (STEP + ngc; CC-BY NOTICE crediting Schmitz, Mertes, Schillinger, Wagner) | 5.34 MB all; ~0.43 MB STEP; ~3.98 MB ngc |
| 2 | NIST PMI STEP | `https://www.nist.gov/system/files/documents/noindex/2024/06/19/NIST-PMI-STEP-Files.zip` | SHA-256 `8fa78429e6d8d9b0d7681d223b6aa9ec98c3772185c55b1a0e3679b21c181911` | cache. Optionally the 11 AP203 geometry-only files (~3.4 MB) in `verify/cam-parts/nist-pmi/` with a NIST NOTICE | 14.0 MB zip, 56.7 MB unpacked |
| 3 | FreeCAD CAMTests | `https://github.com/FreeCAD/FreeCAD` sparse path `src/Mod/CAM/CAMTests` | commit `8fef74c9a1ebb360917e6408daa702c886926f66`. blob SHA-256s in the list below | cache (blobless sparse clone) | ~0.6 MB fixtures |
| 4 | MFCAD++ | `https://pure.qub.ac.uk/files/278385243/MFCAD_dataset.zip` | DOI 10.17034/d1fec5a0-8c10-4630-b02e-b92dc81df823. SHA-256 recorded on first download (HEAD got 403 here, so it may need a manual browser download) | cache | 1.5 GB |
| 5 | MFCAD | `https://github.com/hducg/MFCAD` | commit `ef6d58a40164d5192666821ce98d0cc90e379fac` | cache. A 50-part subset may go in `verify/cam-parts/mfcad/` with the MIT notice | UNKNOWN (15,488 STEP + labels) |
| 6 | ABC STEP chunk 0 | `https://archive.nyu.edu/rest/bitstreams/88598/retrieve` (`abc_0000_step_v00.7z`) | md5 `695388be7a278c7798c8c8ae239772ac` (ABC `md5.yml`) plus our SHA-256 | cache only | 1.59 GB (10k models) |
| 7 | SimJEB CAD | `https://dataverse.harvard.edu/api/access/datafile/4639266` (`SimJEB_cad_(stp).zip`) | dataset v4.0. md5 `19cdb0f4ca8ff51c367ce0238c332622` (Dataverse) plus our SHA-256 | cache only | 207.6 MB |
| 8 | NIST AM artifact | `https://www.nist.gov/system/files/documents/2017/10/19/nist_test_artifact_ver3b.zip` | SHA-256 `ee18b7ff5051614d58eca15fa1aafccbdde493648b52bb3e22cca39d7cf77c37` | **`verify/cam-parts/nist-am/`** (public domain, NIST acknowledgement) | 75 KB zip, 565 KB STEP |
| – | Fusion seg. STEP (if approved) | `https://fusion-360-gallery-dataset.s3.us-west-2.amazonaws.com/segmentation/s2.0.1/s2.0.1_extended_step.zip` | version s2.0.1. S3 ETag `7230e2a962d946e0b90db0097e96bd46-61` (multipart, not a content hash) plus our SHA-256 | cache only, never git | 506 MB |
| – | NIST CTC definitions | `https://www.nist.gov/system/files/documents/noindex/2022/07/19/NIST_MBE_PMI_CTC_Definitions.zip` | SHA-256 `79ebc74d99eb05c0d23de1c1d8730651fa2a24def56f647bac21dd439ec40690` | cache | 5.5 MB |

**Full SHA-256s computed this session** (scratch copies; not stored in the repo):

```
rptu 1.step   8c2142896328cb3af5d4bc7ea3ee8d9acec62d0128fab2f06d14fc4d81f7c349
rptu 2.step   e6c3d7d0638ca832e59ab0d5d1c45bacb78c0738df76c19d5a64196cd831db45
rptu 3.step   872917f49b6c8cb458260571e45df3ffd8476c5e2dccb27121fefb16002f2991
rptu 4.step   9779d88de7a615c439a76582341eb8244c2c73acc3e67c64c8332856e19437da
rptu 5.step   2e77b32274a0b7ab62dcf6ccb8fc47077a8968cce0e3cd466de7abad1cd693f7
rptu 6.step   30328dbe66955e34db199c9a36f0674548941c59164c12e48ea3bf4bca5eade7
rptu 7.step   1fdc0a1ccfdf92f6bee6e67c8e29c0308dafee104025923315a58a0998f9cec3
rptu 8.step   bb7f3809b3e5610dfde77aaf4cd388426bd06a47267cf021587f6bca23822bb7
rptu 9.step   3658eb09d100e75cc57e0e18fb7850eb0de456ac141c6b8a17e21141f8edfd32
rptu 10.step  91c23f1fb78ab88b50e9bf46af2f5a08774a56a772d66d8922801a6fbe2001ab
fc Drilling_1.FCStd     a81e23567a56bfcaa7444a9f6081b1908846ff487b7c4e942f0d7bd4c2b03027
fc boxtest.fcstd        8356cbdcb11cb7ee0f0c83fc31b1821c10636d176808a2eef1fb433212d044ec
fc test_adaptive.fcstd  34207b128f433c53b68f09e58a14412eac6eff36bb6f25c7baaff659bf5d7375
fc test_geomop.fcstd    9f56136546eecaa27e715fde335e2addfea75222eefafd3cf729932ea19fac7e
fc test_holes00.fcstd   781032119db8d718e0a9e2cbf90691e1e496f8f1d292abf40cee25fa6ed9b7bb
fc test_profile.fcstd   8691fdf4dd355ec117b415f06e7151ee6e55800ef92e6c08fb6d7e73d3331d1a
```

The RPTU `.ngc` SHA-256s are not computed yet. Use the Zenodo md5 values until the first fetch.

**Order of work:**

1. Add sets 1 and 8 to `verify/` (small, permissive) as the first CI smoke corpus.
2. Next, sets 2 and 3 into the cache, with import and op-parity runs.
3. Then sets 4 to 7 as on-demand stress sets, never in CI.

## 16. What could not be reached (this session)

| site | result |
|---|---|
| smstestbed.nist.gov, and DOI 10.18434/T4/1503154 | proxy 502 |
| data.nist.gov | 502 |
| pure.qub.ac.uk zip | HEAD 403; the record page was readable |
| thingiverse.com/legal | 403 |
| printables.com terms | 403 |
| repository.gatech.edu | 403 |
| webstore.ansi.org preview | 403 |
| GitHub REST API | not enabled for this session; git clone worked |

No login, user-agent change or bot-wall bypass was tried. Not attempted at all: NAS 979 text,
Mertes et al. 2024, CADParser's Google Drive link, and the Moylan 2011 PDF.

## References

- NIST MBE PMI: https://www.nist.gov/ctl/smart-connected-systems-division/smart-connected-manufacturing-systems-group/mbe-pmi-0 ; NIST licence https://www.nist.gov/open/license
- NIST AM artifact: https://www.nist.gov/node/438556 ; five-axis artifacts: https://www.nist.gov/node/594146
- NIST SMS TDPs: https://catalog.data.gov/dataset/technical-data-packages-tdps-from-the-smart-manufacturing-systems-sms-test-bed ; Hedberg et al. 2019, https://doi.org/10.6028/jres.124.004
- STEP Tools samples: https://www.steptools.com/docs/stepncfiles ; https://downloads.steptools.com/docs/stepncfiles/cds/ ; https://www.steptools.com/copyright.html
- ISO 10791-7:2020 preview: https://cdn.standards.iteh.ai/samples/73814/5fa1265941304324a6350d960fc3e315/ISO-10791-7-2020.pdf ; https://www.iso.org/standard/73814.html
- FreeCAD: https://github.com/FreeCAD/FreeCAD (src/Mod/CAM/CAMTests, commit 8fef74c9)
- MFCAD: https://github.com/hducg/MFCAD ; MFCAD++: https://pure.qub.ac.uk/en/datasets/mfcad-dataset-dataset-for-paper-hierarchical-cadnet-learning-from/
- FeatureNet: https://github.com/madlabub/Machining-feature-dataset ; https://doi.org/10.1016/j.cad.2018.03.006
- ABC: https://deep-geometry.github.io/abc-dataset/ ; https://arxiv.org/abs/1812.06216
- Onshape terms: https://www.onshape.com/en/legal/terms-of-use
- Fusion 360 Gallery: https://github.com/AutodeskAILab/Fusion360GalleryDataset (LICENSE.md, docs/segmentation.md, docs/reconstruction.md)
- DeepCAD: https://github.com/ChrisWu1997/DeepCAD ; https://arxiv.org/abs/2105.09492
- CADParser: https://www.ijcai.org/proceedings/2023/200
- SimJEB: https://simjeb.github.io/ ; https://dataverse.harvard.edu/dataset.xhtml?persistentId=doi:10.7910/DVN/XFUWJG
- RPTU benchmark: https://zenodo.org/records/17035762 ; Mertes et al. 2024 https://doi.org/10.1016/j.dte.2024.100024 (not read)
- AMRC pack: https://doi.org/10.15131/shef.data.28631333.v1
- GrabCAD help (snippet): https://help.grabcad.com/article/246-how-can-models-be-used-and-shared
