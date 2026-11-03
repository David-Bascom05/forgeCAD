# How far a fillet carries on past a kink

A fillet picked on one edge rounds on along the edges that continue it. The kernel took an edge as
continuing only when its tangent was within LINEAR over the radius of straight on (0.0003 degrees at r 2).
Between that and about 0.1 degrees the round could neither carry on nor end: the next wall meets the
ball too shallowly, and the fillet refused with Overflow. Other kernels treat such a kink as tangent.

## What other kernels do
- **OpenCascade, measured** (`occt_tol.py`, output `occt_tol.out`): `BRepFilletAPI_MakeFillet` takes the next
  edges into the contour up to a kink of 5.72 degrees and not at 5.74, inward and outward alike: 0.1 rad.
- **CATIA V5, inferred** from recreated parts held against their CATIA STEP files: where a picked edge met an
  unpicked one at a kink of 0.03 to 0.50 degrees (11 vertices), CATIA rounded the next edge too; at 1.08 to
  7.27 degrees (4 vertices) it did not, and carried the next face on past the vertex to end the round on it.
  The threshold lies between 0.50 and 1.08 degrees, which fits CATIA's default tangency angle of 0.5 degrees.
  Carrying on at 1.2 degrees or more turned 2 of those parts from matching their source to not.

## Used by
`fillet.rs`, `CARRY_COS`: a sharp edge within half a degree of straight on carries the fillet on (CATIA's
figure, the stricter of the two). `tools/find-parts/recreate/cadfs.py`, `CARRY_COS`: the same test when the
importer grows a pick along tangents. Test: `interface/forge-agent/tests/fillet_kinks.rs` (its volume at 0.03
degrees is OpenCascade's from `occt_tol.out`).
