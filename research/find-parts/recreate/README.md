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

## Reproduce

@@REPRO@@
