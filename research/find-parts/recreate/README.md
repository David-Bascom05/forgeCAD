# How well we recreate real parts

Each dataset's own history is translated to `.fc`, built by the kernel, and held against the source CAD's own result.
Tools: `tools/find-parts/recreate/`. No dataset file, and no part derived from one, is in this repo: data lives in
`~/forgecad-data/<dataset>/`, and a regression test restates geometry in its own numbers. Nothing here is used by
shipped code.

## What the numbers mean

Reported separately for every dataset:

- **match**: regenerates, every stage valid, meshes closed, and volume, area and box agree with the source. Tolerance
  0.1% of volume, 0.1% of area, 0.1% of the box's diagonal; 1% where a polyline stands in for an ellipse or spline
  (.fc sketches have no spline row), and those parts are noted `approx`.
- **wrong**: built with no error and disagrees. Triaged by OpenCascade replaying the same `.fc`: OCCT agrees with the
  source and not us is ours and silent, the most serious class; OCCT agrees with us means the `.fc` is not the source's
  part, an importer error.
- **refused**: a clear error. Every part in these sets was built by its source CAD, so every refusal is a wrong refusal
  unless the importer gave the kernel the wrong input.
- **fail, crash, oom, timeout**: a kernel bug by `history::classify`, or the process died. 3 GB, 120 s of CPU per process.
- **not translated**: never reached the kernel, kept apart so the importer cannot flatter or hurt the kernel's number.
  `form` is something .fc or the importer has no form for, `reference` a reference that cannot be resolved, `importer`
  the importer's own bug.

The sets that translate are biased toward simple parts, so a rate is over translated parts of that kind.

## Refused blends: kernel or importer?

`occt_second.py` gives OCCT the same history with the same edges and holds its result against the source's STEP.
`selection_check.py` asks the source's result directly: a blended edge is gone from it, an untouched one is still on it.
`fillet_split.py` joins them per part:

- `kernel-gap`: the edges fit the source's result, or OCCT built them and matched the STEP.
- `importer-suspect`: a picked edge is still sharp in the source's result, or OCCT's result on those edges is off it.
- `undecided`: an unpicked edge is gone, or not checkable. Features after the blend weaken the check to "did not pick too
  much".

OCCT refusing is not evidence of a selection error: Parasolid builds things OCCT does not.

@@NUMBERS@@

## Sets not run

- FllumaOne: its only link is a Google Drive folder "for review", no licence and no public archive until its paper is
  accepted. Synthetic templates in any case.
- bd_warehouse, cq_warehouse: read, not run. Few parametric families, built on OCCT, so they cannot show what OCCT cannot
  do; recording them means running their Python under tracing, and `fastener.py`, `thread.py` (and bd's `flange.py`) call
  `eval()` on strings from their CSV files, so it would need isolation.
- HistCAD: the part finder holds its 5 open samples. The full data sits behind a Hugging Face login that answered 401, so
  it needs someone to ask for or accept access.
- AutoMate, NIST PMI, Fusion segmentation and assembly: final shapes only, so nothing to recreate. With a STEP importer they
  could test reading real B-reps, validity, mass properties against OCCT, meshing, and booleans and blends on imported
  bodies. That importer is not small: AP203/214 parsing, the analytic and NURBS surfaces and trims, sewing and tolerance
  healing; weeks, and not built.

## Limits of the importers

- CADFS: a region query names one curve and a weight for its side. The weight is right about nine times in ten against the
  curve's direction (measured where only one side has a region; neither the raw line direction nor the circle's winding
  explains the rest). Where both sides have regions the weight decides and the part is noted `side-by-weight`, and a wrong
  answer there is an importer miss.
- CADFS: Onshape imprints a sketch on its plane, so faces and earlier sketches in that plane also cut its regions (one part,
  checked by hand, only matches with this). Doing that for every sketch lowered the match count, so it is done only where a
  query names geometry from outside the sketch.
- CADFS: `qSketchRegion` with "exclude inner" is read as the outermost regions; a few sketches of many regions do not fit
  (noted `all-regions`).
- CADFS: a multi-body add goes to the last body made, where Onshape merges all it touches (noted `multibody-join`).
- CADFS STEP files can hold exact copies of a solid; copies are dropped from the truth, and counted.
- CADFS programs can name a sketch entity the program no longer has; those are unresolvable.
- FreeCAD: older files have no BaseFeature; a feature's base is then its blend's Base, its pattern's original, its sketch's
  support, or the feature before it in the document, which can be wrong.
- WHUCAD: only the selection's main name is used, not CATIA's disambiguating faces.

## Reproduce

Data, each in its own directory under `~/forgecad-data/`, read as data with `python3 -I`, nothing in it run:

```
cadfs/dl     huggingface.co/datasets/VladPyatov/CADFS at dd76a9b: featurescript_fp.zip step.zip sketch_raw.zip unique_models.json
freecad/dl   codeload.github.com/FreeCAD/FreeCAD-library master tarball; .FCStd files copied flat by run/survey (data only)
whucad/dl    codeload.github.com/fazhihe/WHUCAD main tarball (data/vec); gitee.com/fred926/whucad-brep archive
```
h5py comes from a venv of its own; OCCT from `tools/occt/setup.sh`.

```
fbuild cargo build --release -p forge-proof --example judge --example topo
fbuild cargo build --release -p forge-model --example oracle
R=tools/find-parts/recreate E=target/release/examples V=~/.local/share/forgecad-occt/venv/bin/python

# CADFS: IDS is every 20th of unique_models.json, sorted
python3 -I $R/run_cadfs.py extract IDS --data DL --work WORK
python3 -I $R/run_cadfs.py run IDS --work WORK --out RUN --judge $E/judge --topo $E/topo --jobs 4
python3 -I $R/run_cadfs.py oks RUN > OK
$V -I $R/step_truth.py DL/step.zip --ids OK --out TRUTH --jobs 4 --tmp TMP
python3 -I $R/run_cadfs.py report RUN TRUTH

# FreeCAD: FILES lists the files holding PartDesign features
python3 -I $R/run_freecad.py run FILES --files DIR --out RUN --judge $E/judge --topo $E/topo --venv $V --jobs 4
python3 -I $R/run_freecad.py report RUN
python3 -I $R/bisect_freecad.py RUN/results.jsonl --files DIR --judge $E/judge --topo $E/topo --venv $V --tmp TMP --out OUT

# WHUCAD: folders 0100-0103 are the manual parts
python3 -I $R/run_whucad.py run IDS --vec VEC --step STP --out RUN --judge $E/judge --topo $E/topo --h5py H5PY --occt $V --jobs 4
python3 -I $R/run_whucad.py report RUN

# wrong and refused parts
THREADS=1 REPS=1 CHORD=0.01 $E/oracle dir:IN > PARTS.json
$V -I $R/triage_wrong.py WRONG.json --export PARTS.json --results RESULTS --truth TRUTH --out OUT
$V -I $R/occt_second.py ROWS RUN/parts --export PARTS.json --topo $E/topo --step DL/step.zip --out SECOND
$V -I $R/selection_check.py ROWS RUN/parts --topo $E/topo --step DL/step.zip --out SELECT
python3 -I $R/fillet_split.py SECOND SELECT --out SPLIT.tsv
JUDGE=$E/judge python3 -I $R/shrink_sigs.py RESULTS RUN/parts --out REPROS --name DATASET
```
Every sweep ran at nice 19, 4 jobs at most, 3 GB and 120 s of CPU per process.
