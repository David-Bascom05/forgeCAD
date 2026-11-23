# STEP export: a spline wall's fit near a pole

Repro: `a_spline_dome_exports_within_its_stated_accuracy` in
interface/forge-lang/tests/kernel_bench_roundtrip.rs (ignored), on
interface/forge-lang/tests/kernel_parts/shell_spline_dome.fg. The dome's wall is a centripetal
spline from (30, 0) through (24, 20) to (0, 30), square to the axis there.

Exported at tol 1e-4 (the default), the file states its accuracy (`Export::uncertainty`) as 12.39
mm, both for the dome alone and for it shelled 2. Some edge fitted near the pole lies that far off
its fitted face. The spline cup without a pole (shell_revolved_spline.fg's body) states 2.0e-4 mm.

What the file gives, against the exact volume (an independent 30-digit integration of the section
about the axis; the dome alone 59629.338687932 mm3, shelled 2 10994.9163243004 mm3):

| | dome alone | dome shelled 2 |
|---|---|---|
| our reader (trims by the file's edges) | 59629.33054, -1.4e-7 | 10994.93631, +1.8e-6 |
| OpenCascade 8.0 (BRepGProp on the file) | 59627.96898, -2.3e-5 | 10993.63757, -1.2e-4 |

Both read the file as valid. The OpenCascade figures do not shrink with the tolerance: at 1e-3
the dome shelled 2 reads 10995.05728 (+1.3e-5), at 1e-4 10993.63757 (-1.2e-4). The kernel's own
answer is exact (its mass on the dome shelled 2 agrees with the 30-digit figure to every printed
digit), so this is the export's fit, not the geometry. It predates the dome shell: the
unshelled dome's file reads the same before 0ab77a8 as after.

OpenCascade's BRepOffsetAPI_MakeThickSolid fails on the dome body (2 inward, its bottom face
removed), so it gives no reference for the shell here.

## Our shell's validity on the dome, where OpenCascade gives no reference

Checked apart from the shell's own code, on six domes (three wall shapes, t 0.0254 to 10):

- `forge_topo::check`: no problems on any.
- Every vertex of the tessellation of each spline wall (up to 525k uses, chord 1e-3), at its
  distance from the dome's section, the section's own nearest point in the vertex's meridian
  plane (Spline::closest, not the Spun's or the offset's code): the outer wall at 0 within
  2.5e-14 mm, the inner wall at t within 3.3e-14 mm.
- The tessellation's volume converges on the exact one from below as the chord shrinks
  (dome 2: -3.5e-4 at the default chord 0.01, -3.1e-5 at 1e-3), as an inscribed mesh does.
- The mass matches the 30-digit volume and area to every printed digit (the tests).

Also seen near a pole, before and after the dome shell alike: the tessellator refuses a spline wall
reaching the axis at chord 1e-4 ("refinement did not settle"), the unshelled dome as much as the
shelled one; at 1e-3 and the default it meshes.
