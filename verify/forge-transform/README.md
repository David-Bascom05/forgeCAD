# forge-transform: rigid motions, mirrors and patterns

What the kernel's transforms preserve, proven over the reals and checked against the code. The
code is `kernel/forge-math/src/frame.rs` and `kernel/forge-model/src/pattern.rs`, with
`bodies.rs`'s `motion`, at 8250ebb.

## The proofs (`lean/`, Lean 4.36 + Mathlib; no `sorry`; `Axioms.lean` fails the build unless all 27 cited theorems use only the standard axioms)

The project builds on verify/forge-mass's frames and surfaces (a path dependency, sharing its
Mathlib checkout).

| file | proven |
|---|---|
| `Rigid.lean` | `complete`: a frame's axes span. `vw_dot`, `vw_cross`: `vec_to_world` keeps dot and cross products: a rotation that keeps orientation. `local_world`, `world_local`: `point_to_local` undoes `point_to_world`, both ways. `compose` is a frame, and `compose_world` is its motion, the composite (frame.rs:89–96). `inverse` is a frame, and `inverse_world` is its motion, `point_to_local` (frame.rs:98–110); so it inverts both ways. `vol_integrand_rigid`: the volume integrand (p − c)·(du × dv) is unchanged at corresponding points. The moments' centre moves with the body (mass.rs:113–117), so volume is kept. |
| `Mirror.lean` | `mv_eq`: `Motion::Mirror`'s linear part is v − 2(v·z)z (pattern.rs:103–121). `mv_dot`, `mv_mv`: an isometry and an involution, as is `mp_mp` for the points. `mv_cross`: it turns cross products back. `keepZ`, `turnZ`: both `Motion::frame` images are right-handed frames (pattern.rs:126–138). `mirror_plane` … `mirror_torus`: each mirrored surface is the image of the old one with v (plane) or u (the rest) negated, and its natural normal is the image of the old normal. So each face keeps its sense (pattern.rs:318). `mirror_circle`, `mirror_line`: each mirrored curve is the image at the same parameter, so edges keep their ranges and ends. `left_kept`: with each loop walked back and every sense flipped (pattern.rs:401–403), the face stays on the left. `vol_integrand_mirror`: the volume integrand is unchanged. |
| `Pattern.lean` | `rot` is `rotated_about_z` (frame.rs:113–126), a frame. `turn_turn`: circular pattern copies (pattern.rs:268, like `Step::Rotate`) compose by adding angles. `turn_axis`: they fix the axis. `turn_full`: n copies at 2π/n close up. |

Assumed, not proven:
- `normalize` and `from_zx` change nothing over the reals here: their inputs are already unit
  and perpendicular.
- The integral of the unchanged integrand over the corresponding domain is the same: a change of
  variables with a uv reflection whose loops are walked back.
- That a mirrored body passes `check`, which `mirrored_bodies_are_valid` tests rather than proves.
  The combinatorial reversal is the one verify/forge-topo-check's model takes; it is not proven
  there.

## The tests (`cargo test -p forge-transform`)

- `frames_compose_and_invert`: 10 000 random frame pairs. `compose`'s motion is the composite,
  `inverse` undoes and equals `point_to_local`, and both stay orthonormal and right-handed to 1e-14.
- `turns_add_and_close`: 2000 random axes. Turns add their angles, fix the axis, and close up
  after n steps of 2π/n, to 1e-11.
- `mirrored_bodies_are_valid`: seven benchmark parts, each mirrored across a random plane clear
  of it. Each result passes `check`. Volume and area double, and the centroid is the mean of the
  original's and its image. Every shell's signed volume (`shell_volume`) appears twice: the copy
  is oriented as the original, not inside out.
- `moved_bodies_keep_mass`: the same parts moved by a random rotation and translation
  (`BodyFeature::Move`). Each passes `check`, keeps its volume, area and principal moments to
  1e-9, and its centroid lands where `body_motion` takes the old one.

No bug found in this item.
