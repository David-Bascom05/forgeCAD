# How well we recreate real parts

Each dataset's own history is translated to a Forge part (`.fg`), built by the kernel, and held against the source CAD's
own result.
Tools: `tools/find-parts/recreate/`. No dataset file, and no part derived from one, is in this repo: data lives in
`~/forgecad-data/<dataset>/`, and a regression test restates geometry in its own numbers. Nothing here is used by
shipped code.

How a blend's edges are named: each importer finds the edges (and a shell's faces) it means on the part as built so
far, by what it knows of them (a FreeCAD `EdgeN` sampled from the stored shape, a CADFS or WHUCAD query's carrier
surfaces), exactly as before. Those edges are then written as the lineage selection that names exactly them on that part:
`edges(of: F, shape: .., r: .., longer: ..)`, `edges(between: (A, B))` of anchors (`F.end`, `F.side(S.l)`), a pattern's
copy `P[k]` or a hole's place `H[k]`; where none of those does, where the faces of the features on either side
meet (`edges(between: (F, G))`, a blend's own faces too), told apart from look-alikes by the anchored faces at their ends
(`touching: F.side(S.l).edges`) or the world axis a line runs along; never by position. forge-proof's `topo` works the selection out
(`forge_import::state`, the same code `forgecad import` names FreeCAD's edges with) and checks the union names those
edges and no others; where no union does, the part is `reference`, not translated. `fgw.py` writes the statements:
every frame the importer worked out is a `plane(at:, normal:, x:)` through helper points, so the kernel gets the frame
and the numbers the retired format gave it.

Checked against the retired importers when they were ported (2026-11-22): the importers as they wrote the retired
format, built by its own judge and topo, and as they write `.fg` now, on the same bodies and the same kernel. Classes
as below; every body that matched still matches, and every body both built has the same volume (to 1e-9) and faces,
except two WHUCAD parts, 1e-6 and 2e-9 apart, where an arc is now written through its ends (the retired format wrote
centre, radius and angles, whose ends sat 1e-10 off the lines beside them).

| FreeCAD, the 245 files with fillets (677 bodies) | retired | `.fg` |
|---|---:|---:|
| match | 482 | 484 |
| wrong | 5 | 5 |
| refused | 53 | 53 |
| fail | 5 | 5 |
| timeout | 1 | 0 |
| not translated: form | 110 | 110 |
| not translated: reference | 21 | 20 |

The two new matches: a body the retired run lost to a wall-clock timeout under load (it builds in both, the same volume
and faces), and a chamfer the retired importer left untranslated (its edges not found) on a part where two bodies of one
Part tree share a chain: a feature stepped twice now gets ids of its own (a Forge id is bound once; the retired format
let the second copy's labels shadow the first's).

| WHUCAD, every 7th manual part with a fillet, chamfer or shell, and 50 more (361 bodies) | retired | `.fg` |
|---|---:|---:|
| built (no STEP here to hold them to) | 243 | 243 |
| refused | 71 | 71 |
| fail | 6 | 6 |
| not translated: form | 19 | 19 |
| not translated: reference | 22 | 22 |

CADFS's data (and WHUCAD's STEP files) were not on the machine the port was checked on, so `cadfs.py` is ported by the
same writer and selection as the two above, untried on its own data.

## What the numbers mean

Reported separately for every dataset:

- **match**: regenerates, every stage valid, meshes closed, and volume, area and box agree with the source. Tolerance
  0.1% of volume, 0.1% of area, 0.1% of the box's diagonal; 1% where a polyline stands in for an ellipse or spline
  (the importers write splines as polylines), and those parts are noted `approx`.
- **wrong**: built with no error and disagrees. Triaged by OpenCascade replaying the same `.fg`: OCCT agrees with the
  source and not us is ours and silent, the most serious class; OCCT agrees with us means the `.fg` is not the source's
  part, an importer error.
- **refused**: a clear error. Every part in these sets was built by its source CAD, so every refusal is a wrong refusal
  unless the importer gave the kernel the wrong input.
- **fail, crash, oom, timeout**: a kernel bug by `history::classify`, or the process died. 3 GB, 120 s of CPU per process.
- **not translated**: never reached the kernel, kept apart so the importer cannot flatter or hurt the kernel's number.
  `form` is something Forge or the importer has no form for, `reference` a reference that cannot be resolved, `importer`
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

## What the formats mean

Learned from the data, each checked against the source's own result. For the Rust importers that come after STEP.

FreeCAD (`.FCStd`, a zip of `Document.xml` and `.brp` shapes; every object keeps its shape, so every step has a truth):

- A sketch's wires: `Sketch::toShape` takes the first edge left and pulls in the first edge in document order that meets the
  wire's last or first vertex (`TopoShape::sort_Edges`, within `Precision::Confusion`), until nothing meets or it closes. At a
  junction the order decides, and a wire may end open.
- A face from wires: the bullseye maker nests wires, not regions. A point inside an odd number of wires is solid. Wires that
  touch, like a dumbbell tangent to its rectangle, cut the plane into more regions without changing the parity.
- A pad's sketch normal is its placement's z; older files have `Sketch` for `Profile`, no `BaseFeature`, and `Support` as a Link,
  LinkSub or LinkSubList. Since 0.19 a pad stores its `Direction`.
- `AddSubShape` is the feature's tool as it was built, so a tool can be checked apart from the booleans.
- A Part workbench solid (`Part::Revolution`, `Part::Box`, `Part::Cylinder`) can be a PartDesign chain's base through a sketch's
  `Support`. 411 of the 451 bodies that read as "first feature cuts" were this.
- A profile that is not a sketch (`Part::Part2DObjectPython`, Draft) has no rows; its stored shape's wires are the profile.

Fusion 360 Gallery (`reconstruction/*.json`, with a STEP per extrude under the stage's `step` name):

- A feature's `bodies` lists every body there is after it; `extrude_bodies` the ones it made or changed.
- A join takes every body it touches, not the last one made. A surviving body keeps its id and the other's face ids, so
  shared face ids name what merged. A body swallowed whole leaves no face behind; a face point inside the tool finds it.
- Regions of one new feature that touch make one body: `extrude_bodies` says how many.
- Profiles of one feature can split a shared curve differently, so cancelling shared curves is not enough: check the area.
- Two-sided extents can both be negative: the two signed distances bound the slab whichever way round they fall.

CADFS (Onshape, `sketch_raw` and programs):

- A region query is `IMPRINT FACE` with `TD([[edge, weight]])`. An edge `IMPRINT EDGE derivedFrom entity` may carry its own
  `TD([[vertex, weight]])`, usually `INTERSECT VERTEX` of two entities, in about 40% of a sample's items. Reading it as "the
  piece beside that vertex" gave 1 match and 16 wrong on the pilot's unsettled parts, whichever end the weight was taken to mean,
  so it is not used and those parts stay untranslated. What it means is open.
- Imprinting a sketch on its plane when an edge bounds nothing alone gave 1 match and 3 wrong on the pilot: not used.
- A profile from a solid's face extrudes along the face's outward normal: 8 match, against 1 the other way.
- A loft's `isStart` is not the array's order: `CAP_FACE`'s OSD names the profile. `capPos` is the section's index.
- A lofted wall's rulings (read from the STEP's B-spline control points): each straight side runs to the arc between the
  points of a circle in line with its corners, at the same fraction along both.
- A circle's seam in a loft is not in the history. Off one axis it twists the wall and moves the volume by tens of percent, so
  those lofts are left untranslated until the seam rule measured on `spline-1` is on main (`research/spline-loops/seam.py`).
- A closed sweep has no caps in the source; two halves meeting at the far side give the same solid.

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

# HistCAD: blends' points resolved to lineage selections on the part so far
python3 -I tools/find-parts/histcad.py SEQ.json .. --out DIR --topo $E/topo

# wrong and refused parts
THREADS=1 REPS=1 CHORD=0.01 $E/oracle dir:IN > PARTS.json
$V -I $R/triage_wrong.py WRONG.json --export PARTS.json --results RESULTS --truth TRUTH --out OUT
$V -I $R/occt_second.py ROWS RUN/parts --export PARTS.json --topo $E/topo --step DL/step.zip --out SECOND
$V -I $R/selection_check.py ROWS RUN/parts --topo $E/topo --step DL/step.zip --out SELECT
python3 -I $R/fillet_split.py SECOND SELECT --out SPLIT.tsv
JUDGE=$E/judge python3 -I $R/shrink_sigs.py RESULTS RUN/parts --out REPROS --name DATASET
```
Every sweep ran at nice 19, 4 jobs at most, 3 GB and 120 s of CPU per process.
