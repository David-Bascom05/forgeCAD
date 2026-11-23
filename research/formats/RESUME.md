# Resuming the real-files work (research/formats)

Branch claude/repo-review-mbdrlj-formats-corpus, on import-robust (ad8c3d7). The report is
real-files.md; this note is how to pick the work up.

## Done

- **Judged on real files, against an independent reader, by a rule fixed before the results**
  (real-files.md has each format's corpus, rule, adjudication and tables):

  | formats | reference |
  |---|---|
  | STL, OBJ, 3MF | numpy-stl, trimesh, lib3mf |
  | STEP, .brep | OCCT 8.0.1 under research/import/truth.md's rule |
  | DWG | LibreDWG's dwgread 0.13.3 |
  | DXF (written) | ezdxf |
  | TTF | fontTools / FreeType |
  | SHX | ezdxf |
  | FCStd | FreeCAD's stored shapes |

- **Writers round-tripped**: STL (to_binary_stl), STEP (the repository's parts and real files),
  DWG (read and written back), DXF and DWG flat patterns.
- **At scale**: all of Thingi10K (10 000), all of Google Fonts (3924), the slicers' and lib3mf's
  own files (844).
- **Reader fixes**, each with its repro as a committed test, gated:

  | commit | fix |
  |---|---|
  | 8164af8 | far LINE, axis and plane origins |
  | 4687a3d | STEP assembly placements (crate::asm) |
  | 512ddac | tori through their axis; whole faces bounded by a seam |
  | ec20825 | LibreCAD's DWG pages and header |

- **Routed**: bugs 1, 3, 6 and 10–12 to import-robust; 9 to sketch-text.

## Dropped (owner priority)

| item | why | where it stopped |
|---|---|---|
| IGES | too few public files | 4 files, all trimmed surfaces with no solid |
| Google Fonts scale run | sketch text frozen | finished before the ruling; results kept |
| DWG bug 7's rest | low value | LibreCAD's R2007/R2013 objects stay raw: 4–29 data bits past each object's fields, where the decoder asks for none |
| Mass runs of other kernels' files | paused | ABC chunk 0000 STEP at 205/10 000 (results/abc-0000-step-partial.jsonl, OCCT's truth per file); ABC OBJ at 565/7168 (results/abc-0000-obj-partial.jsonl) |

## Left, by format

- **STEP write, bug 14** (ours, medium). OCCT's data/occ/Motor-c.brep solid 13: four degree 7×8
  rational Bézier patches, read as procedural surfaces, are fitted as cubic B-splines; OCCT finds
  one fit invalid. Ours reads the export back right.
  - Next: `formats-probe step-rewrite Motor-c.brep OUT` writes OUT.13.step. Find the face by
    OCCT's BRepCheck (face 14 in OCCT's order), then the patch it came from.
  - Then write such a patch exactly (its own NURBS) rather than fitting it, and make a repro: one
    rational Bézier face on a box, via repro/make_brep.py.
- **STEP read**, not new, robustness.md's: FaceWrongSide on periodic faces (Bottom.brep's
  cylinder, ABC, RC_Buggy); a linear extrusion whose profile lies 500 m along it (ABC
  00007680); a cone of half angle -90 degrees (Top.brep); OFFSET_SURFACE; AP242 tessellated
  solids; NIST ftc_06's "loops draw no region".
- **3MF** (import-robust's):
  - ZIP64, bug 3, high: repro/make.py → tetra-zip64.3mf;
  - unknown requiredextensions: tetra-required-unknown.3mf;
  - p:path build items: build-item-path.3mf;
  - the cap's wording.
- **OBJ**: `\` continuation (continuation.obj, import-robust's); a face of fewer than three
  corners refuses the whole file (bug 16: ABC 5 of 565, trimesh's notenoughindices.obj).
- **STL**: a binary file one triangle short is refused (bug 15).
- **FCStd** (import-robust's): FreeCAD 1.1 SideType (repro/fcstd_sidetype.rs, a test for
  forge-import tests/fcstd.rs).
- **TTF** (sketch-text's): hmtx lsb placement (repro/make_font.py → lsb-not-xmin.ttf); CFF fonts
  not read (113 of 502).
- **DWG**: bug 7's rest (dropped).
- **Runs paused**: the ABC chunk 0000 STEP and OBJ runs, which resume where they stopped (below).

## Rerun

1. **Python.** A venv outside the repository with trimesh numpy-stl ezdxf fonttools lib3mf py7zr
   freetype-py cadquery-ocp==8.0.1.1.0. Run everything with `python -I`.
2. **dwgread.** From libredwg-0.13.3.tar.xz (ftp.gnu.org): `./configure --disable-bindings &&
   make`.
3. **Build.** `cargo build --release -p forge-formats -p forge-cli -p forge-step --example
   stepread`.
4. **Sources.** They are listed in manifest/sources.json (URL, commit, licence), each file used
   in manifest/*.jsonl (path, bytes, sha256):
   - git clone each at its commit (a sparse checkout where large);
   - Thingi10K by stage/thingi10k.py;
   - the NIST zips from nist.gov/document/{nist-pmi-step-files, nist-ftc-ctc-pmi-cad-models,
     nist-stc-pmi-v4};
   - ABC's chunks from archive.nyu.edu (abc-dataset/data/step_v00.txt and obj_v00.txt list them).

   No dataset file is committed.
5. **Judges**, each with a `report` mode: mesh_judge.py, brep_judge.py, dwg_judge.py,
   dxf_judge.py, font_judge.py, fcstd_judge.py. Usage is in each script's docstring and in
   real-files.md's Rerun.
6. **At scale**, streamed and resumable (stream.py; a file already in OUT is skipped):

   ```
   stream.py thingi PROBE OUT META_DIR --batch 100 --jobs 2
   stream.py abc PROBE OUT abc_0000_obj_v00.7z --jobs 2
   stream.py urls font PROBE OUT https://raw.githubusercontent.com/google/fonts/COMMIT/ @PATHS
   stream.py files mesh PROBE OUT @LIST
   stream.py report OUT
   brep_judge.py run STEPREAD OUT @LIST --jobs 4 --timeout 60    # resumable too
   ```

7. **A reader fix's bit gate.** `bits.py BASE_STEPREAD NEW_STEPREAD OUT @LIST` lists the files
   read differently. Pair it with `tools/speed/gate.sh` (GATE_COUNTER=callgrind here) and the
   forge-model reliability example.
8. **Repros, made from nothing.** `repro/make.py DIR`, `repro/make_brep.py DIR` (OCCT),
   `repro/make_font.py DIR`. The committed tests are forge-step's tests/far_origins.rs,
   assembly.rs and tori.rs, and forge-dwg's tests/header_variants.rs and r2004's
   a_page_with_no_terminator_reads.

## Head

The commit adding this note; its id is reported to the master.
