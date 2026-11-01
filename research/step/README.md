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
  a mesh on import and refuses the body past 2% (plus what a chord can stray over the area).
- Some were the judge: copies dropped on one side only, a truth from OCCT's non-adaptive volume (the
  CADFS truth was re-run), or OCCT disagreeing with itself (adaptive against its own mesh).

Rerun: `rate.py` and `tools/occt/step_read.py`, with the datasets outside the repo.

## Export: what a file says of its accuracy

Each fit keeps within its tolerance of what it stands for, and a fitted edge and a fitted face still
missed each other by far more where the edge runs past the part of the surface the fit was checked on:
0.01 mm on jack against a stated 1e-4, confirmed by OCCT's own projection. `export` now measures this
with the reader's measure and states the larger number (`Export::uncertainty`). The fit itself is
not tightened yet.

