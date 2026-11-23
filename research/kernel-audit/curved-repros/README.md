# Curved repros: point contacts the kernel builds

Each file is a part whose result is non-manifold at one point, by the exact curved reference
(verify/forge-exact, research/kernel-audit/exact-comparison-curved.md). On
claude/repo-review-mbdrlj-fix-curved at 19e1b6c the kernel builds every one as a single valid
body. It must refuse with ZeroThickness. They are the six kernel-wrong scoreboard family cases.
Each file's header gives the reference's verdict and the kernel's volume.

| file | scoreboard case | kernel builds | reference |
|---|---|--:|---|
| ball_on_box_top.fg | degen_plane_sphere/0 | 32 523.6 | sphere and top plane touch at a point |
| cone_apex_on_box_top.fg | degen_plane_cone/0 | 32 261.8 | apex on the top: link not a disc |
| ball_on_cylinder_cap.fg | degen_cylinder_sphere/2 | 6 806.8 | sphere and cap touch at a point |
| cones_apex_to_apex.fg | degen_cone_cone/2 | 4 188.8 | two cones' apexes meet |
| ball_on_cone_apex.fg | degen_cone_sphere/2 | 2 618.0 | ball on the apex |
| balls_touching.fg | degen_sphere_sphere/2 | 4 712.4 | two spheres touch at a point |

The balls in these files have a horizontal axis, so each contact is on a ball's equator.
With a vertical axis (the contact at a pole) the kernel refuses correctly. So the touch test
misses sphere contacts away from the poles, as well as cone apexes.

Check one: `forgecad build F.fg`, which should fail with ZeroThickness. Against the reference:
`cargo run --release -p forge-exact-fuzz --bin exact-curved -- fg:F.fg`. On the sequential path
the two files with a work plane read as NoEntity: the harness's convex sub-history drops the
work plane. The n-ary path and `forgecad build` are unaffected.
