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
