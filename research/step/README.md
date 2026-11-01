# STEP: what real files use

`survey.py` scans STEP files as text (nothing in them runs) and counts entities, what faces and
edges stand on, units, tolerances and originating systems; `report.py` prints the counts. Used to
choose what `kernel/forge-step` reads first and how it represents NURBS surfaces.

Rerun: `survey.py OUT.json --zip Z --sample 3000` for each of cadfs, whucad and fusion (the datasets stay outside the repo).

Sampled 2026-10-31 (3000 files each; seed 1):

    dataset  faces   plane cyl  cone sph  torus bsurf(rational)  system
    cadfs    136k    58%  26%  1.8% .6%  2.3%  11.7%(.9)         Onshape AP242
    whucad   33k     71%  21%  3.7% .2%  3.3%  .22%(0)           CATIA AP203
    fusion   47k     73%  23%  .2%  0    0     3.3%(.6)          Autodesk AP214

Edges: LINE 63-76%, CIRCLE 20-23%, B_SPLINE_CURVE 3-13%, ELLIPSE to 1%. B-spline surfaces are
degree 3x1 in 95% of Onshape's and 98% of Fusion's, mostly declared SURF_OF_LINEAR_EXTRUSION;
CATIA's are few, to degree 5x5. Control nets mostly 16 or fewer points. WHUCAD also has 150
SURFACE_OF_REVOLUTION. No PCURVE or SEAM_CURVE in these samples, so the reader builds from 3D
curves. Lengths come in metres (Onshape), centimetres (Fusion), mm and inches; uncertainty from
1e-8 to 1e-2. 12-22% of files hold more than one solid.

## Export: a fit's blind spot, found through a reader

OCCT read frame and jack as invalid, with edges at a thread's run-out tip a turn away and 0.9 to
1.5 mm off, and we first took that for OCCT re-projecting our pcurves. It was our fit: a thin
wedge of a face (the run-out) can pass between a coarse cell's samples, so the cell was never
refined and the surface was 2 mm wrong there, with an error of 3e-5 reported. The fit now also
checks points along the loops.

## Import: wrong with nothing to show for it

Sweeping real files against OCCT (`rate.py`), twelve read as valid and 10% or more off. Compared with
OCCT's adaptive mass, OCCT's own mesh and ours, face by face:

- Two read wrong: `net_of` divided rational control points by their weights twice (`bezier_data` gives
  each its own), so surfaces moved from arcs missed their edges, 0.27 mm on an arc of radius 6. Fixed.
- Several were `mass` going wrong near a pole on a revolved face with edges a little off it: the angle
  wanders there, a Chebyshev fit was read far outside its box and came to 1e58. A face whose loop
  leaves the fit's box is measured again by columns alone, and `mass` refuses a negative face area or a volume past the box. What is left
  of this can stay a few percent out, so `measures_agree` in `read.rs` holds the integrals against
  a mesh on import. Past 1% (plus what a chord can stray over its curved area), or where either cannot
  be worked out, the body is imported with a warning in words (`Body::unverified`, shown in the check as
  `import-not-verified`) and never counted as matching by `rate.py`; only a budget run out refuses it
  (David, 2026-11-01: "import with warning").
- Some were the judge: copies dropped on one side only, a truth from OCCT's non-adaptive volume (the
  CADFS truth was re-run), or OCCT disagreeing with itself (adaptive against its own mesh).

Rerun: `rate.py` and `tools/occt/step_read.py`, with the datasets outside the repo.

## Export: what a file says of its accuracy

Each fit keeps within its tolerance of what it stands for, and a fitted edge and a fitted face still
missed each other by far more where the edge runs past the part of the surface the fit was checked on:
0.01 mm on jack against a stated 1e-4, confirmed by OCCT's own projection. `export` now measures this
with the reader's measure and states the larger number (`Export::uncertainty`). The fit itself is
not tightened yet.

## Import: refused for the mesher's sake (2026-11-01)

The check that integrals and mesh agree refused 203 of 6514 files; most of them were good. Four causes, and
none was what the refusal text suggested. Each was found by comparing per face with OCCT's adaptive
area and OCCT's mesh; neither of those is truth on its own.

- A linear extrusion's net reaches 1e6 past any part, and `Nurbs::bbox` took it whole. A body's box was
  2e6 across, so a chord of `5e-4 * diagonal` was 1000 mm. The box now takes the v asked for.
- Healing's `limit` was `20 * uncertainty`. A normalised part keeps the figure of its far larger
  original, 0.1 on a part of size 1, and `closed_form_edge` moved edges to where two planes met 0.06 away,
  further than the edges were long. Then `collapse_slivers` dropped their neighbours and faces lost their
  area in the mesh only, as `mass` integrates curves and the mesh uses vertices. The limit is now also at most
  0.2% of the part. That figure was tried once and not tuned.
- At a pole in all but rounding, `line_curvatures` divided noise by a square of noise and asked for a
  million grid steps both ways. `1e-8` is well above rounding and well below any real ratio of |S_u| to |S_v|.
- Crossing chords, chords through points, two points in one place: a file's edges meet only within their
  tolerance. `triangulate_within(.., tangled)` cuts chords where they cross and takes what an odd number of
  chords part from the outside. An earlier try merged the nearest ends of what crossed, as far as 2% of
  the face, and was dropped: 49 files still failed, with a guessed reach. `tessellate` stays strict, so a
  kernel bug on our own parts still shows; only import's check and the viewers use `tessellate_tolerant`.

`Limits::max_evals` is 1 << 25, spent by healing and the check of one solid; only `Nurbs::eval_all` counts,
so no part made here spends any. Counted over 6514 files of Fusion, CADFS and WHU: p99 3e5 evaluations, p99.9
1.9e6, then 2e7, 4.6e7 and 9e7 on three small WHU files, the last two past the budget. Our own largest export
(valve, 25 MB) spends 1.2e7. An evaluation costs 200 to 500 ns on a loaded laptop, so the budget is 7 to 17 s of work; the
two files that hit it are refused in 8 and 10 s. It is not a bound on the whole read: parsing and what does not
evaluate a spline surface are not counted (see the sweep's read times). The first try, surface points in `mass`, bounded nothing: a file took 36 s
on 20 000 points, each a whole search of the surface.
To count again, print `budget::left` after `measures_agree` over the sweep.

A revolved spline's mass was 2-6% over, the mesh matching OCCT: 18 of the 35 refusals left. `foot`'s Newton
stepped past the end of a surface that holds at its ends (`Nurbs::wrap` clamps) and, its point no longer
moving, stopped there, so a loop's parameters ran up to 7% of the face past the end and the integrals
counted area that is not there. `foot` now holds such a point at the end; a surface that really does run on
past its domain is left alone. Parts made here match Pappus by hand (`measure mass`, rectangle, circle,
ellipse and a 90 degree turn, to the digits printed), so only files were affected.

Two findings from the files left. `LoopsCross` flagged 48 WHU files once `check` was drawn finely; over each
crossing's depth (an end of each chord past the other) 38 were a corner on another loop's line to rounding, or
past it by less than the kernel's tolerance or the edges' own: not a fault, and the check now takes that band.
10 remain real, with holes crossing by up to 4e-4, four of them bad to OCCT as well. And `chain_edge`
made intersection curves through surfaces that all but touch between the chain's points: a middle's
refine went 0.01 away on links of 0.006, the curve wandered (0.089 long sampled coarsely, 0.15 finely) and a
mesh could come out five times too large. Such an edge now keeps the file's curve.

Round trip (`interface/forge-agent/tests/bench_roundtrip.rs`): a fit's edges lie up to `Options::tol` (1e-4) off it,
past `tighten`'s 5e-6, so each became a chain with a search at each point, on nets of a thousand spans: crank took over
80 s to read back. An edge within 20 times the kernel's tolerance of a spline surface now keeps its curve: 0.9 s.
And `fit_surface` checked its loops by the distance to the surface anywhere, and a fit can slide along the
surface and leave a loop off itself: on jack 0.0103 mm, by OCCT's projection as well as ours. Along the loops it now goes
by parameters: 1e-4 on every bench part.

A body whose mass cannot be worked out is refused: the check has nothing to compare with, and that is no pass.
`mass` keeps its own refusal of an integral that did not settle for parts made here; on a file's body the bound holds
the gaps the file leaves, and the mesh is what checks it.

## OpenCascade .brep

`kernel/forge-step/src/brep.rs` reads OCCT's text `.brep` (sniffed from "CASCADE Topology"), written from what the
files hold with OCCT as test oracle only, and shares `finish` with STEP. Offset surfaces and curves, scaled or mirrored
locations and edges with no 3D curve are refused by name. Corpus: 3206 FreeCAD-stored files, outside the repo.

The budget also counts mesh work on such files: `find_edge` round a fan and constraint recovery, where a chord crosses
thousands of triangles. Plain analytic faces took over 60 s there with no surface evaluation at all; now 3 to 5 s and a
refusal. A file's face stops being cut finer after 3 rounds, not 9, and goes to the tangled draw.

## Assemblies are not read

Each solid is read once, where it stands in its own part: NEXT_ASSEMBLY_USAGE_OCCURRENCE, MAPPED_ITEM and
CONTEXT_DEPENDENT_SHAPE_REPRESENTATION are not applied, so copies are missing and parts may be out of place, valid all
the same. `assembly_asks` counts what a file asks that this leaves undone, placements that move and parts used twice, and
such a body is flagged `assembly-not-placed` in the check. A placement onto itself asks nothing: all 59 Onshape files of the CADFS
set with a MAPPED_ITEM are these, and match OCCT. Reading placements is David's call.

## IGES

`kernel/forge-step/src/iges.rs` reads IGES 5.3's boundary-representation entities (186 and down), written from the
specification, sharing `finish` with STEP. A file of trimmed surfaces (144, 143) has no topology and is refused by name;
sewing them is not done. Measured only against OCCT-written files: 300 written by `IGESControl_Writer` in BREP mode from STEP files of
the sets above, so a real system's IGES is untested. A loop runs round on the surface's own side, so a face against its
surface takes it reversed. OCCT writes clockwise arcs as a mirrored matrix on the curve.
