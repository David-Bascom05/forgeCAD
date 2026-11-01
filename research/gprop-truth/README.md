# Why OCCT's BRepGProp and the mesh of the same WHUCAD STEP disagree

Research only. No kernel code uses this. It decides how the progress page scores the WHUCAD parts
whose truth is in doubt. That is `rate.py`'s line "truth unreliable": the file's gprop volume or
area is more than 0.3% from its own mesh. In occt-rates/whucad/truthcheck.jsonl that holds for
**184 of 3868 files**. The 181 of 3846 quoted elsewhere is the same test counted over the judged
parts only.

## How BRepGProp integrates (OCCT master)

Source files: `BRepGProp.cxx`, `BRepGProp_Gauss.cxx`, `BRepGProp_Face.cxx`.

- The volume is ⅓∮(p − L)·n dA over the faces. L is `roughBaryCenter`, the mean of all vertex
  points.
- Each face is integrated in its own (u, v):
  - Each edge's **pcurve** gives the boundary.
  - For every Gauss point l on an edge, the inner integral runs in u from the surface's first
    u (`BU1`) to the pcurve's u(l), weighted by dv/dl.
  - The 3D edge curve is never used.
- Loops are not closed. Nothing compares one face's pcurve images with the next face's.
- With `Eps < 1` the routine returns a quadrature error only.

So BRepGProp measures the solid bounded by each face's pcurve images, about a point that depends
on the vertices.

- If those images do not meet, the answer depends on L and on `BU1`. The total vector area
  **A** = Σ∫n dA is the exact measure of how much: V(L + d) = V(L) − ⅓ d·**A**.
- `tc3.py` computes **A** by re-running each face's integral about four points.
- BRepMesh shares each edge's discretisation between its two faces, and those points come from the
  3D curve. The mesh volume did not move when the origin moved (to 1e-15) on every part where it
  closed.

**No WHUCAD file carries a pcurve.** `grep -l PCURVE` over all 8816 finds none. Every pcurve that
gprop integrates was made by OCCT's STEP reader, by projecting the 3D curves. The reader works to
the file's precision (`read.precision.mode` 0), which is CATIA's 0.005 on parts normalised to
about size 1.

## The 184, sorted by cause

Rerun: `drive.py IDS OUT 3` (one child per part running `tc3.py`, 150 s each), then
`cls2.py OUT ours.<sha>.json`, then `prec2.py` for the re-read. All run under fbuild with the OCCT
venv. The numbers are filled in below.

```
G curve ends off their vertices: open pcurve loops         58  |  trust mesh 28 none 30        | vs OCCT build  39: mesh closer  21, within .1% mesh  10 gprop   6 | vs ours  35: mesh closer  19, within .1% mesh  13 gprop   5
M the mesh is open or misses faces                         29  |  trust gprop 1 none 28        | vs OCCT build  17: mesh closer   8, within .1% mesh   2 gprop   2 | vs ours  12: mesh closer   5, within .1% mesh   1 gprop   2
N the truth mesh (deflection 5e-4) is not converged        22  |  trust gprop 3 none 19        | vs OCCT build  13: mesh closer   2, within .1% mesh   0 gprop   7 | vs ours  15: mesh closer   3, within .1% mesh   0 gprop   8
P pcurves off their 3D edges by up to the edge tolerance   51  |  trust mesh 51                | vs OCCT build  47: mesh closer  40, within .1% mesh  31 gprop   4 | vs ours  40: mesh closer  36, within .1% mesh  30 gprop   3
S several solids in the file                               21  |  trust none 21                | vs OCCT build  13: mesh closer   6, within .1% mesh   2 gprop   0 | vs ours  12: mesh closer   5, within .1% mesh   2 gprop   0
U unexplained                                               1  |  trust mesh 1                 | vs OCCT build   0 | vs ours   0
X census timed out                                          2  |  trust none 2                 | vs OCCT build   0 | vs ours   1: mesh closer   0, within .1% mesh   0 gprop   0
{'mesh': 80, 'none': 100, 'gprop': 4}
```

(trust: the rule below. vs OCCT build / vs ours: of the parts where that volume exists, how many the file's
mesh or gprop matches within 0.1%. Ours is never used to decide, only shown.)

- **P: pcurves off their 3D edges by up to the edge tolerance.** The loops are closed in uv, but
  each face's pcurve image strays from the 3D edge by as much as OCCT's tolerance (5e-4 to 5e-3).
  - Example, 100000014 face 42: the file's B-spline edge lies on the plane exactly and up to
    5.4e-4 off the cylinder. OCCT's pcurve on the cylinder is a straight line (2 poles, degree 1)
    that leaves the 3D curve by 8.6e-4. Its edge tolerance is 8.7e-4.
  - The area of that face agrees with the mesh, but its volume term is −6.3e-6 against the mesh's
    −1.8e-7. That is the whole 0.15% gap on this thin-walled part (volume 0.0041 against area
    1.08).
  - On OCCT's own build of 100000014 (tools/occt, o14.json), the build has edges of tolerance
    5.5e-4 whose pcurves leave them by 5.2e-4. Its gprop faces fail to close by
    |**A**| = 8.1e-5, which can move the volume by up to 0.3%. That is the same mechanism, and
    the reason "BRepGProp reads 0.1% low on a shape OCCT built itself". OCCT's mesh of that build
    is itself open (it misses faces), so it is not a check there.
  - A plain OCCT boolean of exact cylinders (`occtbuilt.py`: a thin tube cut across) has edge
    tolerance 1e-7 and agrees with its converged mesh to 3e-6. So BRepGProp on its own is sound.
    The trouble is the tolerant pcurves.
- **G: curve ends off their vertices.**
  - The 3D curves stop short of the vertices they share (research/tolerant-step). OCCT's projected
    pcurves then do not meet, so the loops are open and gprop depends on the origin (100003655:
    7.1% low).
  - The mesh goes through the vertices and gives the intended part exactly on 100003655.
  - In general the shape is only defined to within the gap. Where OCCT's build from the feature
    list and ours agree (to 1e-9), the file's mesh is often 0.3-1.3% from both, so some of these
    are translation differences and not truth errors.
- **N: the truth mesh is not converged.** truthcheck meshes at an absolute 5e-4 with angle 0.1,
  which is too coarse on small curved parts. The mesh moves by more than 0.1% between that and
  1e-4/0.03.
- **M: the mesh is open or misses faces.** BRepMesh failed on some faces.
- **S: several solids in the file.** These are copies or overlaps, the assembly question, not
  measurement.

## Re-reading at a tight precision (`prec2.py`)

The same files read with `read.precision.mode` 1 and `read.precision.val` 1e-6:

```
P  51 files: gprop now within 0.1% of the mesh on 43, still over 0.3% on 4
G  56 files: gprop now within 0.1% of the mesh on 16, still over 0.3% on 35
```
So P is the reader's doing: OCCT's pcurves projected to 0.005 on parts of size 1. G is in the files: the curve ends really
are off their vertices.

## Which truth to score against

Decide for each part from the file alone, never from ours. The rule in `cls2.py`, `trust()`:

0. **None** when a curve end lies more than 0.1% of the diagonal from its vertex: the shape is undecided there.
1. **The mesh** when it is whole and settled:
   - one solid;
   - every face meshed, no free edges;
   - the volume does not move when the origin moves;
   - the volume moves by less than 0.1% between deflection 5e-4 and 1e-4.

   In P, OCCT's build from the feature list agrees with the mesh within 0.1% on 31 of 47 and with
   gprop on 4. Ours agrees with the mesh on 30 of 40 and with gprop on 3.
2. Otherwise **gprop**, but only when its faces close (|**A**|·diag/3 ≤ 1e-4 V) and its pcurves lie
   on their edges (deviation ≤ 1e-6).
3. Otherwise **no trusted truth.** Score against OCCT's build from the features if OCCT built it
   valid, or leave the part unscored. Never score it as wrong.

On the 184: mesh 80, gprop 4, none 100. "None" covers the 47 with an open or unsettled mesh and gprop not closed (M 28, N 19), the 21 with
several solids, the 30 whose curve ends lie more than 0.1% of the diagonal from their vertices (the shape itself is
undecided at the judge's 0.3% there) and 2 the census could not finish in 150 s (100000301, 100004369). Per-part lists: occt-rates/whucad/truth-causes.json (`cls2.py`'s causes.json).

Better than both: regenerate truthcheck and the truth itself with the reader at a tight
precision. `prec2.py` shows that one setting removes cause P from gprop, while the mesh stays as
it was. That is a change to `step_truth.py` in occt-rates, not to the repo.

## Files

- `tc3.py`: the census for one part.
- `drive.py`: one child per part.
- `cls2.py`: cause and trust.
- `prec2.py`: the re-read.
- `pf.py`: gprop against the mesh face by face.
- `occtbuilt.py`: OCCT's own exact boolean.
- `ob14.py`: OCCT's build of a part from its feature JSON. The input (`o14.json`, made by the forgecad export the oracle uses) is dataset geometry and stays out of the repo. The loop topology check is in research/tolerant-step/loops.py.

The datasets and the results stay in occt-rates. Per-part lists are in the message to main.
