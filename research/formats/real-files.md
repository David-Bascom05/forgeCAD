# Every format on real files (2026-11-23)

The owner asked for public databases of real files for every format we read or write.
Until now only STEP (CADFS, WHUCAD) and FreeCAD's .FCStd were tested on real files
(research/import/robustness.md section 2, coverage.md on the import branch). This note covers every
other format, and STEP, IGES and .brep from new sources. Each format has:

- a public corpus: how it was chosen and its licence;
- an independent reference reader;
- a judging rule fixed before any result was looked at;
- the results;
- for writers, a round trip through the reference.

Branch claude/repo-review-mbdrlj-formats-corpus, on import-robust (ad8c3d7), which has the new
STL/OBJ/3MF readers. No dataset file is in the repository. The files were staged outside it, and
`manifest/` lists every file used (source, URL, commit, path, bytes, sha256, licence). Every bug
below is a repro made from nothing (`repro/`), apart from two that are marked.

How results were adjudicated: where the rule called a file a mismatch and the cause was the
reference, a second step settled it. Each such step is a declared function in the judge, run on
the first step's output, and is named in the tables. Where the judge itself was wrong (it used the
wrong quantity, or the reference was used wrongly), it was corrected before its results were
counted, and the correction is written where it was made (in the script and below).

## Summary

| format | read / write | corpus (files) | result |
|---|---|---|---|
| STL | read | Thingi10K every 20th (495), assimp+trimesh tests (36) | **all match** after adjudication (0 refused, 0 crash) |
| OBJ | read | Thingi10K's 42, common-3d-test-models, assimp, trimesh (125) | 110 match (9 after adjudication); 3 refused (1 bug) |
| 3MF | read | 3MF Consortium test suites 2.4.1, 3mf-samples, assimp, trimesh (2361) | positives: 1411 match, 0 mismatch; 69 refused (3 bugs) |
| STL | write | the STL and OBJ above, written back (656) | every STL source bit-equal; OBJ sources equal but for topology STL cannot hold |
| STEP / IGES / .brep | read | NIST PMI CAD models (44), OCCT data (41), FreeCAD (34), pythonocc-demos (13), cadquery, libredwg (2); ABC every 40th of chunk 0 (250) | see §2: 4 bugs, one of them silent |
| STEP | write | the repository's parts (173 with a solid), the real files above | parts: 173/173 match; real files: §2 |
| DWG | read | LibreDWG (142), LibreCAD (25), GDAL (8), ezdxf (5) test files (180) | all 155 read match (4 after adjudication); of the 160 R2000+ files, 4 refused (1 bug) and 1 a truncated test file; 20 pre-R2000 refused by design |
| DWG | write | the 155 read, written back | 155/155 read by dwgread as the original |
| DXF | write | every sheet flat pattern in the repository (39) | 39/39: ezdxf reads them, audit clean, the same entities as the DWG |
| TTF | read | Google Fonts every 20th (197), Adobe Source (188), Unicode text-rendering-tests (85), fonts in FreeCAD/ezdxf/GDAL/cadquery (32) | 388 k code points: all glyph ids and advances equal; outlines equal but for FreeType's side-bearing placement (1 low bug) |
| OTF (CFF) | read | the same | 113 of 502 fonts refused by design (CFF) |
| SHX | read | ezdxf's 4 stroke fonts and 1 shape file (5 files, 21 230 shapes) | 5/5 match |
| FCStd | read | FreeCAD's own test and example documents (59) | 34 of 35 translated bodies match FreeCAD's stored shape; 1 silent bug |

## The bugs, ranked

Ranked by harm. Silent wrong geometry comes first, then refusals of common valid files, then rare
refusals, then messages. Repros are in `repro/` (make.py, make_brep.py, make_font.py,
fcstd_sidetype.rs). Each one runs as the note says, and was checked against both our reader and
the reference.

| # | format | bug | real files | repro | severity |
|---|---|---|---|---|---|
| 1 | FCStd | FreeCAD 1.1's `SideType` (0 one side, 1 two sides, 2 symmetric) is not read. FreeCAD 1.1 deprecates `Midplane` and leaves it false, so a symmetric or two-sided pad or pocket is translated one-sided: wrong geometry, reported as "translated" | FreeCAD's own data/examples/PartDesignExample.FCStd: 36% low, first divergence at its Mirrored | `fcstd_sidetype.rs`: a test in forge-import's fcstd.rs style. Fails today: "symmetric: centroid z 5 vs 0" | high |
| 2 | STEP | An assembly's instances are not expanded: each part is read once, wherever and however often it is placed. No refusal, so the total is silently short | as1-ac-214 / as1-oc-214 / as1_pe_203 (5 of 18 solids, volume −15%), as1-ac-214_small (3 of 5), libredwg's ACEB10 (3 of 7) | `two-instances.step` (XCAF, one box placed twice): ours 1 solid, 1000; OCCT 2, 2000 | high |
| 3 | 3MF, FCStd | ZIP64 packages are refused as "not a zip package". forge-import's zip.rs reads only the classic end record, and FCStd uses the same Zip | consortium positive P_XXX_2202_01 | `tetra-zip64.3mf`: lib3mf and Python's zipfile read it; `tetra.3mf`, the same package without ZIP64, reads | medium |
| 4 | .brep, STEP | Tori whose major radius is not past the minor one (horn R = r, spindle R < r) are refused ("torus: radii 10 and 10"), or read as volume 0 | OCCT's Bottom.brep, Top.brep, mal_vis.brep (refused); MODERN_Sink_1.brep (one of 3 solids refused) | `horn-torus.brep`: ours volume 0, OCCT 19739.21. `spindle-torus.brep`: ours FaceWrongSide and volume 0, OCCT 6516.87. (`horn-torus.step` reads right but warned) | medium |
| 5 | STEP | A LINE whose origin lies more than 1 km along it refuses the whole file ("CARTESIAN_POINT: outside the 1 km model box"). Its edge is near the part, and such a LINE is valid; CATIA writes them | NIST nist_stc_07_asme1_ap242-e3 (origin 169 m out; OCCT: 1 valid solid) | `far-line-origin.step`: a box whose one LINE origin is moved 2 km along the line. Ours refuses; OCCT reads 1000 | medium |
| 6 | 3MF | A `requiredextensions` entry we don't know is read as core. The spec says a consumer must refuse it, and a required extension (displacement, booleans) can change the geometry | consortium negative N_XXX_0428 (read, not refused) | `tetra-required-unknown.3mf`: ours reads it | medium |
| 7 | DWG | R2004+ files written by LibreCAD's DWG writer are refused: "compressed data ends early" (R2004, R2013) or "past the end" (R2007). dwgread reads all four | LibreCAD's tests: ordinary_enc_AC1018/AC1021/AC1027/ac1027_ansi932 | no synthetic repro: making one needs that writer's compressor. The files are public (LibreCAD @954217a, GPL-2), so this is the one bug that points at the source files | medium–low |
| 8 | FCStd, 3MF | Refused rather than read: a sketch's GeomEllipse, AdditiveSphere, a chain through Part::Feature, pad type 3 (FCStd); OFFSET_SURFACE, AP242 tessellated solids (STEP). Named refusals, so gaps rather than faults | FreeCAD's Drilling_1, TestTangentMode3, hole_puzzle, PadTest/PocketTest; ABC 00002480; NIST nist_ftc_08_asme1_ap242-e1-tg | none: features to add (coverage.md's) | low (gaps) |
| 9 | TTF | A glyph is placed by its glyf coordinates where FreeType moves it by hmtx lsb − xMin (the phantom point). It shifts 6875 glyphs in 39 fonts, nearly all by 1 unit (DejaVu, Halant, the Unicode tests' Lana by 91) | §5 | `lsb-not-xmin.ttf`: a 100-unit square at x 100, lsb 90. FreeType 2.13.2 puts it at 90..190, ours at 100..200 | low |
| 10 | 3MF | A build item in another model part (production extension, `p:path`) is refused as "no object 1". Components with p:path are refused by name | 27 consortium positives (P_XPX_0705_01 and others) | `build-item-path.3mf` | low (message) |
| 11 | 3MF | A member past the 256 MB cap is refused with "not a zip package (or one whose members would inflate past the caps)". The cap is by design; the message hides which applies | P_*_0909_04/06 (a 380 MB model) | (by size: none made) | low (message) |
| 12 | OBJ | A line continued with `\` is refused ("\\" is not a vertex index). The format allows it, and trimesh and assimp read it | trimesh's wallhole.obj | `continuation.obj` | low |
| 13 | TTF | cmap format 13 and Mac-only cmaps: "no Unicode cmap of format 4 or 12" | TestCMAP13, TestCMAPMacTurkish | — | low |

The STEP and .brep problems that robustness.md already ranks are not repeated as new bugs. They
are FaceWrongSide/LoopsCross on periodic faces (OCCT's Axis_of_bearing, Ball, screw.step,
pythonocc's KR600 and RC_Buggy, 6 of the ABC files), "loops draw no region" (NIST ftc_06, both
copies), and integrals against mesh (FreeCAD's helix1.brep). They are tallied in §2 for the STEP
owners.

These were sent to import-robust (session_01Q19M119KWVSR9GxEW1HRUD), which owns the readers, for
the master to route. This work wrote no fuzzer and changed no reader.

## 1. Meshes: STL, OBJ, 3MF

**Corpora.**

- **Thingi10K** (Hugging Face Thingi10K/Thingi10K, raw_meshes; 10 000 Thingiverse models):
  - every 20th file id, sorted, offset 0: 495 STL and 5 OBJ;
  - and all 42 OBJ;
  - licences per file, all open (CC-BY-SA 176, CC-BY 154, CC-BY-NC 73, BY-NC-SA 55, GPL 13, PD 8, others): `manifest/thingi10k.jsonl`.
- **Test models** of the readers' own projects, positive and negative:
  - assimp test/models/{STL,OBJ,3MF};
  - trimesh's models/;
  - alecjacobson/common-3d-test-models.
- **3MF Consortium**: test_suites 2.4.1 (2271 cases, positive and negative, suites 1–11) and
  3mf-samples (80); BSD-2.
- Not used:
  - PartNet: registration and its terms;
  - Objaverse: glTF, with per-object licences, so it would need conversion; not needed;
  - ModelNet and Princeton (OFF): not a format we read;
  - ABC's meshes: its STEP is used instead (§2).

**Rule** (`mesh_judge.py`, fixed before the first run). References:

| format | reference |
|---|---|
| STL | numpy-stl, raw facets |
| OBJ | trimesh, `process=False` |
| 3MF | lib3mf 2.5, the build walked through components and transforms, in mm |

Compared:

- the triangles;
- the vertices (STL only, since trimesh re-indexes an OBJ's);
- the area;
- the box of the vertices the triangles use;
- the open edges, by our reader's definition (a directed edge repeated or without its reverse),
  worked out in numpy from the reference's arrays, with STL corners welded where equal to the bit;
- closed, and the volume where closed.

The tolerance is 1e-9 relative. For 3MF it is f32 rounding (2e-6 × (1 + distance/size)), because
lib3mf keeps coordinates as f32: its API's type, known before the run. Outcomes:

- match / mismatch;
- refused: ours refused, the reference read it;
- ref-refused;
- both-refused;
- crash, and timeout at 60 s (4 GB cap).

| corpus | files | match | mismatch → adjudicated | refused | both refused | ref refused | crash / timeout |
|---|---|---|---|---|---|---|---|
| STL, Thingi10K | 495 | 294 | 201 → **201 match** | 0 | 0 | 0 | 0 / 0 |
| STL, test models | 36 | 26 | 7 → **7 match** | 0 | 2 (empty) | 1 | 0 / 0 |
| OBJ, all sources | 125 | 101 | 9 → **9 match** | 3 | 10 (no faces) | 2 (trimesh crashed) | 0 / 0 |
| 3MF positives | 1719 | 1411 | 0 | 69 | 223 | 16 | 0 / 0 |
| 3MF negatives | 552 | 155 read | — | 5 | 191 | 201 | 0 / 0 |
| 3MF samples etc. | 90 | 49 | 0 | 4 | 11 | 26 | 0 / 0 |

Adjudication, declared steps:

- **STL** (`adjudicate`): numpy-stl keeps an ASCII STL's numbers as f32 and reads only its first
  `solid`. Every mismatch was an ASCII file. Against trimesh's f64 parse of every solid, welded the
  same way under the same rule, all 208 match. Two of them hold two solids, where numpy-stl had
  half the triangles.
- **OBJ** (`obj-edges`): every mismatch was in the open edges, because trimesh splits vertices by
  texture coordinate and material. Counted from the file's own `f` indices, all 9 equal ours.

The 3MF refusals:

| count | refusal | why |
|---|---|---|
| 35 | p:path components | production extension, by design |
| 27 | p:path build items | bug 10 |
| 7 | over the member cap | 6 by size, bug 11; 1 is ZIP64, bug 3 |

The 223 "both refused" positives are beam-lattice, boolean, displacement and slice cases with no
mesh in the build, so both sides are right.

**Negatives we read.** Most concern things a mesh reader passes over:

- OPC and content-type rules (N_XXX_02xx, 0402–0407);
- metadata and thumbnails (0410, 0419);
- materials and the slice and production extensions (the SPX/XPM/XPX/SXX/XXM suites);
- mesh rules we read as facts: a triangle repeating a vertex (0411), fewer than 4 triangles (0426),
  inward or negative volumes (0416, 0418), which our summary reports as open or signed.

The one that matters is 0428, the unknown required extension (bug 6).

**Writer** (`rewrite`): our reading of each file is written by `to_binary_stl` and read back by
numpy-stl, then held to the reference's reading of the source by the same rule: an STL source's
own f32 reading, and an OBJ source's at f32 tolerance. (The first version held our own f64 reading
against the f32 file, which is wrong for every ASCII and OBJ source. It was corrected before its
results were counted.)

| source | written | match | mismatch | not read |
|---|---|---|---|---|
| STL | 530 | **528** (bit-equal) | 0 | 2 (empty) |
| OBJ | 125 | 85 | 25 | 13 (+2 trimesh) |

All 25 OBJ mismatches are open-edge counts only; triangles, area and box agree. An STL holds no
indices, and welding f32 corners merges or collapses vertices that an OBJ keeps apart (the
reference finds new degenerate triangles in them). That is the format's limit, not the writer's.

## 2. B-rep: STEP, IGES, OpenCascade .brep

**Corpora** (`manifest/brep.jsonl`, `manifest/abc.jsonl`):

| source | files | what | licence |
|---|---|---|---|
| NIST MBE PMI | 44 STEP | CTC/FTC/STC test models from CATIA, NX, Creo, SolidWorks and Inventor: AP203, AP242, AP242 with PMI, one tessellated. Three zips from nist.gov/document/… | "can be used without any restrictions" |
| OCCT data/ | 41 | .brep, STEP, IGES | LGPL |
| FreeCAD | 34 | .brep test shapes, its STEP test assemblies | LGPL |
| pythonocc-demos | 13 | STEP, IGES, .brep | LGPL |
| cadquery, libredwg | 2 | STEP | — |
| ABC chunk 0000 | 250 STEP | every 40th model, Onshape/Parasolid | Onshape ToU 1.g.ii, the same basis as CADFS |

**IGES:** the only public files found were 4 (OCCT's and pythonocc's), all trimmed surfaces with
no solid. OCCT finds no solid in any of them either. Burkardt's IGES collection
(people.math.sc.edu) did not answer, and the CAx-IF files are for members only. So IGES is barely
judged here.

Fusion 360 Gallery (S3) was reachable, but its licence is for non-commercial research. Its use for
testing a product was left to the owner, so it is not used. MFCAD/MFCAD++ were not reached.

**Rule** (`brep_judge.py`): ours is stepread.rs, as research/step/rate.py runs it. The truth is
OCCT 8.0.1 under truth.md's rule:

- per solid, the Gauss–Kronrod volume;
- the area of the measure that agrees with that volume;
- uncertain where neither agrees.

The outcome, in order:

| outcome | when |
|---|---|
| crash / timeout | 120 s ours, 360 s OCCT |
| both-none | OCCT reads no solid and nor do we |
| refused | OCCT reads a solid, ours none |
| uncertain | the truth rule can't settle OCCT's measure |
| invalid | ours has check problems or a warning |
| match | the sums of \|volume\| and area within 1e-3 (rate.py's rule, so these compare with robustness.md's 93.4%) |
| match6 | the same within 1e-6 |
| mismatch | otherwise |

| corpus | files | match6 | match (1e-3) | mismatch | invalid | refused | uncertain | both none | timeout |
|---|---|---|---|---|---|---|---|---|---|
| NIST | 44 | 29 | 10 | 0 | 2 | 2 | 0 | 0 | 1 (OCCT's) |
| OCCT data | 41 | 18 | 2 | 1 | 3 | 3 | 0 | 14 | 0 |
| FreeCAD | 34 | 27 | 0 | 2 | 2 | 0 | 0 | 3 | 0 |
| pythonocc-demos | 13 | 3 | 2 | 2 | 2 | 0 | 0 | 4 | 0 |
| cadquery, libredwg | 2 | 1 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| ABC chunk 0 | ABC_N | ABC_M6 | ABC_M | ABC_MIS | ABC_INV | ABC_REF | ABC_UNC | ABC_NONE | ABC_TO |

The mismatches and refusals, by cause:

- **Assemblies read once** (bug 2): all 5 assembly mismatches.
- **Tori with R ≤ r** (bug 4): OCCT's Bottom, Top, mal_vis, and MODERN_Sink's missing solid.
- **The far line origin** (bug 5): NIST stc_07.
- **AP242 tessellated solids not read**: NIST ftc_08 e1-tg (bug 8, a gap).
- **ABC**: one OFFSET_SURFACE (a gap); one closed spline face with one rim and no pole; one with
  "no place on the rim for a second meridian". The last two belong to robustness.md's
  "tori/closed faces with no edges" class.
- **ABC 00004440**: ours reads 7 valid solids where OCCT drops one ("Part 7", 80 faces, a normal
  representation). Ours has no problems in it, so this is OCCT's doing as far as can be told.
- **NIST ctc_02 AP203 with PMI**: OCCT timed out on it at 360 s. Ours reads it in 0.3 s, so it
  can't be judged.

**Writer.**

- **The repository's parts** (`write-fg`): `forgecad export F.fg`, read by OCCT, held to the
  part's own volume and area (`query F stat`) to 1e-4. Of the 230 examples:
  - 173 export a solid, and all **173 match**: OCCT reads one valid shape each time.
  - 43 have no solid (sketches, assemblies of references), and 14 don't build.
  - The first run used `build`'s `physical`, which counts only physical solids, and flagged 11
    sheet parts whose flat pattern is a second body. Corrected before counting: those parts were
    right.
- **Real files** (`write`): each solid we read is exported and held, through OCCT, to our own
  body's volume and area to 1e-4. STEPW_REAL.

## 3. DWG and DXF

**Corpora** (`manifest/dwg.jsonl`): 180 DWG files, the test files of:

- LibreDWG: 142, R1.4 to R2018;
- LibreCAD: 25;
- GDAL's OGR CAD driver: 8;
- ezdxf: 5.

ODA/Teigha samples were not used: their terms are a click-through. We read no DXF, so DXF corpora
(ezdxf's 103, LibreCAD's 1412, GDAL's) have nothing to judge. The DXF writer is judged on our
own output below.

**Rule** (`dwg_judge.py`): LibreDWG's dwgread 0.13.3, built from the GNU release, `-O json`.
Objects are keyed as the file types them: a fixed type's number, or a class's DXF name. Compared:

- the count of every key over all objects;
- the count of every key over entities;
- the layer names, as a multiset.

Match means all three are equal.

On the reference side, two corrections were made on first sight of its output: dwgread's JSON
gives some class objects a fixed number (MLEADERSTYLE as 1), and those are keyed by the class
named. The types with both a fixed number and a class (LAYOUT 82, PLACEHOLDER 80, LWPOLYLINE,
HATCH, XRECORD) keep their number.

| | files | match | mismatch → adjudicated | refused | both refused |
|---|---|---|---|---|---|
| read | 180 | 151 | 4 → **4 match** | 24: 20 pre-R2000 (by design: R2000–R2018 read), 4 LibreCAD-written (bug 7) | 1 (truncated test file) |
| write (read, written back, read by dwgread) | 155 | **155** | 0 | — | — |

Adjudication of the 4, by hand:

- **3 files** (example_2018, TS1, lc_multiline_attrib): dwgread's JSON `type` field is wrong for
  ATTDEF, ATTRIB, OLE2FRAME and LIGHT (an ATTDEF given as 1 or 4, LIGHT as 3, OLE2FRAME as 2).
  Its entity names agree with our types one for one.
- **1 file** (GDAL attribs.dwg, codepage 1252): dwgread writes its layer names' bytes into the
  JSON undecoded, so they are not UTF-8. Ours decodes them ("ECL réseaux …").

Every file written back reads the same in ours too.

**DXF writer** (`dxf_judge.py`): every sheet flat pattern in examples/, 39 sheets. `export F dxf ID`
is read by ezdxf (readfile, then its auditor), and `export F dwg ID` by dwgread. Match means:

- both read;
- no audit error;
- the same model-space entity types and counts.

Result: **39/39 match, 0 audit errors.** The first run counted dwgread's model-space BLOCK/ENDBLK
pair, which a DXF keeps in BLOCKS; it was corrected before counting.

## 4. FCStd beyond the library

**Corpus:** every .FCStd in FreeCAD's repository (59, LGPL; `manifest/fcstd.jsonl`): examples, test
fixtures, CAM tool bits. 24 hold no PartDesign body.

**Rule** (`fcstd_judge.py`): each translated body is built and held to the shape FreeCAD stored
for it (its Shape's .brp, measured by OCCT under the truth rule) to 1e-4, coverage.md's tolerance.

| bodies | match | mismatch | untranslated (named) | stopped |
|---|---|---|---|---|
| 45 | **34** | 1 (bug 1) | 9: empty bodies 2, pad type 3 ×4, GeomEllipse, AdditiveSphere, a chain through Part::Feature | 1: a fillet the kernel refused (strange_part_with_holes) |

The mismatch was bisected feature by feature against the stored shapes:

| feature | FreeCAD | ours |
|---|---|---|
| Pad | 436.477 | 436.477 |
| Groove | 433.471 | 433.471 |
| Mirrored (about XY) | 430.465 | 433.471 |

The pad is symmetric (−7..7) in FreeCAD and one-sided (0..14) in ours, so the mirror of the groove
falls outside our part. The cause is FreeCAD 1.1's `SideType = 2` (bug 1).

## 5. Fonts: TrueType, OpenType, SHX

**Corpora** (`manifest/fonts.jsonl`, 502 files):

- Google Fonts: every 20th of its 3924 font files, sorted (OFL, Apache, UFL);
- Adobe Source Sans, Serif and Code Pro: all 188, OTF and TTF (OFL);
- the Unicode text-rendering-tests fonts: 85, test fonts with odd tables;
- the fonts in FreeCAD, ezdxf, GDAL, cadquery and LibreCAD: 32.

SHX: ezdxf's stroke fonts (iso3098, azomix, azomix_i, kst32b) and its ltypeshp.shx. LibreCAD's and
GDAL's .shx files are shapefile indexes, not fonts.

**Rule** (`font_judge.py`). The reference is fontTools 4.68. For every BMP code point in its best
cmap that names a glyph other than .notdef, compared:

- the glyph id;
- the advance (exact);
- the outline through its glyph set, components resolved:
  - contour count;
  - signed area (AreaPen against ours by Green's theorem, exact for quadratics) to 1e-9 of em²;
  - exact bounds (BoundsPen) to 1e-6 units.

Also, forge-dwg's TtfFont must take every font forge-font takes.

Before counting, two uses of fontTools were corrected:

- RecordingPen leaves components undecomposed, so DecomposingRecordingPen is used.
- A one-point contour (an anchor) draws nothing, so contours are counted with a segment. The
  first font met one.

SHX is judged against ezdxf's shape-file parser: the shape numbers, each one's pen end to 1e-9 of
the height, and its stroke box to 1%.

| | fonts | code points | match | mismatch → adjudicated | refused |
|---|---|---|---|---|---|
| TrueType outlines | 389 | 387 899 | 330 | 56 → see below | 2 (cmap 13, Mac-only: bug 13) + 1 corrupt test font both refuse |
| CFF (OTF) | 113 | — | — | — | 113 by design (`FontError::Cff`): every Adobe Source OTF and 16 test fonts |
| SHX | 5 | 21 230 shapes | **5** | 0 | 0 |

Glyph ids and advances agree everywhere. The 56 fonts' 7179 differing code points were each held
to the glyf outline as stored (`adjudicate`: fontTools' coordinates, decoded here, with no
side-bearing shift and lone points dropped):

| code points | what |
|---|---|
| 6875 (39 fonts) | equal but for the hmtx placement that FreeType and fontTools apply and ours doesn't (bug 9) |
| 231 | equal but for fontTools' CFF preference (a test font with both tables) and anchor points in BoundsPen's box |
| 73 | ours drops contours of zero area (two-point loops, a lone on-curve point with an off-curve one). Same area; they draw nothing, so not a fault for text that becomes solids |

## Rerun

Python tools in a venv outside the repository:

- packages: trimesh, numpy-stl, ezdxf, fonttools, lib3mf, py7zr, freetype-py,
  cadquery-ocp==8.0.1.1.0;
- dwgread from libredwg-0.13.3.tar.xz (ftp.gnu.org), `./configure --disable-bindings && make`;
- every script run with `python -I`.

```
cargo build --release -p forge-formats -p forge-cli -p forge-step --example stepread
# stage: git clone the sources in manifest/sources.json at their commits (sparse where large),
# stage/thingi10k.py for Thingi10K, the NIST zips, ABC's abc_0000_step_v00.7z (every 40th .step)
mesh_judge.py run target/release/formats-probe OUT @LIST;  mesh_judge.py adjudicate OUT; mesh_judge.py obj-edges OUT
mesh_judge.py rewrite target/release/formats-probe OUT @LIST
brep_judge.py run target/release/examples/stepread OUT @LIST;  brep_judge.py write target/release/formats-probe OUT @LIST
brep_judge.py write-fg target/release/forgecad OUT @FG_LIST
dwg_judge.py run|write target/release/formats-probe DWGREAD OUT @LIST
dxf_judge.py target/release/forgecad DWGREAD OUT @FG_LIST
font_judge.py run|shx target/release/formats-probe OUT @LIST;  font_judge.py adjudicate PROBE OUT
fcstd_judge.py target/release/formats-probe target/release/forgecad OUT WORK @LIST
repro/make.py DIR; repro/make_brep.py DIR; repro/make_font.py DIR
```

`results/` holds every file's verdict and cause (paths relative to the source), from which the
tables above were counted.
