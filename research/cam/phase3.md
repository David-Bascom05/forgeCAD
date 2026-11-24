# CAM Phase 3: 3-axis surfacing (2026-11-24)

Branch cam/3axis, stacked on claude/repo-review-mbdrlj-cam-lean (#15/#16). User doc: docs/cam.md.
Design: design.md §7 phase 3. Not built: adaptive / constant-engagement clearing (design.md §8,
patent counsel first); z-level roughing here is offset clearing.

## Built

| piece | where | how it is exact or sound |
|---|---|---|
| the tool's column | forge-cam-geom/drop.rs `Column` | a cutter's lower profile b(ρ) on [0, R] from its chain (lines, lower-right arcs), the column {ρ ≤ R, z ≥ b(ρ)} also as a revolved solid (`solid`, for its signed distance). `Column::of` refuses a chain that does not widen to R; `holds_or_is` says which bodies lie in it |
| drop-cutter | drop.rs `drop`, `drop_above` | CL = sup over the face of y.z − b(ρ). Per face a closed form gives an upper bound for the untrimmed surface and a contact point; the contact, evaluated in intervals where proven inside the face, gives a lower bound. Faces whose bounds do not meet go to an interval branch and bound over the face's parameters (mean-value form per profile piece, capped by the monotone bound and the closed form). Result [lo, hi] with hi − lo ≤ res, each face widened by its mu |
| closed forms | drop.rs `closed`, `closed_t`, `disk_drop`, `bowl_drop`, `plane_support`, `wall_drop` | planes (any tilt: the profile's support against the slope); spheres, cylinders with a horizontal axis and tori about a vertical axis reduce to one circle in the vertical plane through the tool's axis (ring_bound / line_bound, Drop.lean), solved per profile piece: tangency (dome: external; bowl: internal), rings, flat tops, arc ends. Bowls (the face below its centre/axis/tube centre, decided from the parametrisation) take the face's lateral range. Vertical walls (prisms): z1 − b(D), D slab.rs's proven 2D distance |
| the gap of a move | drop.rs `gap_move` | the least signed distance of the face to the column over the move (`Gap::Clear { lower }`, `Over` with a real face point, `Unknown`). Closed form over pieces of s (mean-value form in s) first; the sphere bound S − sd(centre) for a convex column; for walls R − D and z1 − b(D) − tip; else branch and bound over (p, q, s) with the column's own interval signed distance capped by the vertical and radial depths. A proven bound holds for every body in the column (Drop.lean `sd_body_ge_of_column`) |
| a dome cylinder's move, exact | drop.rs `cyl_chord`, `section`, `Column::sq_convex` | a horizontal cylinder above its axis against a convex column: across the axis its untrimmed CL is a sup-convolution of two concave functions (the disk's top, the profile upside down), so concave; along a line move the tip is affine in the lateral place d, so the move's worst point is the unconstrained maximum r √(1 + λ²) + B(λ) (the disk's and the profile's conjugates) if the maximiser d* = e* − u* lies within the move's d, else the nearer end (Concave.lean `move_clear_of_outside`). Past the face's end along its axis by a: the column's section a off its axis (a ball's is a ball, a flat end's a flat end) or, for profiles with b(√·) convex, b(a) − b(0) lower. Used per piece of s in `gap_move` before the mean-value form: one piece for most moves |
| the checker's route | forge-cam ops.rs `certify_with` | `certify_fixtured` unchanged; with `column` on (the surfacing ops), the cutter and a shank inside the column are checked against the part by `gap_move` first; `Over` is checked as the cutter's own signed distance at the witness point (a real violation when below the limit); everything else goes to the existing branch and bound. A move that has failed is not checked further (its failure is reported) |
| lowering | forge-cam lower3d.rs | every face `lower` takes, as it takes it; a sphere or a revolved face trimmed by other curves as a superset (the whole sphere or torus, a cylinder or cone between its edges' heights): sound for every check, stricter; procedural faces refused by kind |
| ops | forge-cam surf.rs | `parallel` (zig-zag rows, all cores), `waterline` (marching squares on a CL grid, crossings by certified bisection: the push-cutter), `zlevel` (offset clearing of the free tool-centre region per level, passes ≤ the tool's radius apart, its voids recorded), `pencil` (the CL surface's creases), `rest` (rows over the positions whose reach holds what an earlier tool left, by grid envelopes), `scallop` (level sets of the distance along the CL surface from the area's edge) |
| generators' self-check | surf.rs `Surfacer` | every move is checked by `gap` (gap_move, so the dome cylinder's exact form above) at half the op's tolerance and split (a new drop at its middle) until proven; steep pieces become a step (up, then across); paths are made with the column widened 0.2371·tol past the rim (no exact wall tangency) and the leave as the cutter grown by it (`Column::grow`) |
| 3D in-process stock | surf.rs `ZMap` | a height map whose cells are lowered only to a height every point of the cell is proven cut down to (the farthest corner's reach), so it holds the true stock; its faces for the checker. The machining document uses the 2.5D in-process stock, to which z-level roughing adds its voids |
| machining document | forge-cam-lang | `parallel`, `waterline`, `zlevel`, `pencil`, `scallop`, `rest`; certification through the column route; examples/machined/mold_core(.fg, _cam.fg) |

## Measurements (this VM, 4 cores, release)

| what | before | after |
|---|---|---|
| a ball in a concave fillet of its own radius, one move along it | 24 s (630k cells) | 12 µs (closed form) |
| mould core, parallel, 4 mm rows (2.8k moves), made and self-checked | 34 s | 0.13 s |
| a ball beside a corner sphere, a level move (side contact) | 1.2 s | 24 µs |
| the dome test's deliberately gouging path, certified (refused) | 9 s | 0.17 s |
| mould core machining document, five ops, 15.4k moves, every move certified (G1–G6, full) | 222 s | 34 s |
| drop-cutter, mould core, 775 points | — | 0.15 s (≈ 0.2 ms/point, most in one cell) |
| mould core document, the generators' self-check (CPU, all ops) | 6.06 s (5.46 s of it dome cylinders: fillets) | 0.79 s (dome cylinders 0.05 s) |
| mould core machining document, wall | 17.5 s (after main's merge) | 13.2 s (the certifier uses gap_move too), then 6.1 s (below) |
| the in-process stock's faces, per later op (z-level's voids) | 1.8 s (9 voids, 170 faces) | 0.52 s (5 voids: a level's void equal to the one above it is that one's floor lowered, exactly the same stock) |
| waterline on the mould core (7 levels, grid 0.5), wall | 4.9 s | 2.1 s (the push-cutter's probes stop once CL is proven above or below z: `drop_decide`) |

The document now (6.1 s wall, the posted program byte-identical to the 13.2 s run's): waterline
2.1 s, rest 1.1 s, parallel 0.5 s, z-level 0.45 s (its grid 0.09 s, offsets 0.26 s), the stock's
faces 3 × 0.52 s, the certificates 0.13 s in all. The earlier 13.7 s put on z-level was mostly
the stock built from its voids, not its own offsets.

## L6

- tests/drop_vs_sampling.rs (forge-cam-geom): the drop against dense sampling of the face and
  against the branch and bound alone (closed forms off) on random planes, spheres (domes and
  bowls), cylinders (horizontal and tilted), tori (any axis, and about a vertical one as round or
  foot), cones, for five cutters: every sample under hi, the two certified answers overlapping,
  resolved to res. A cleared move holds at every sampled face point and instant; an `Over` is
  confirmed by the drop at its instant. Each cutter lies in its column (20k sampled points).
- tests/surf.rs (forge-cam): every op on the dome certified; the parallel path moved 0.05 into the
  part refused (gouge); z-level certified, a stepover over the tool's radius refused; the z-map
  holds the stock (each lowered cell's random points within the tool's reach at their foot); a
  digest of 3D paths pinned, equal native and wasm32-wasip1.
- tests/cam.rs (forge-cam-lang): the mould core roughed and finished, certified full; without the
  roughing the finishing is refused (its shank in stock nobody removed).

## The generators' self-check: why it stays, and what was proven away

David asked for the check to be proven away (its own lemma instead of a check at generate time).
From a move's two end drops alone it cannot be: a face can enter the column through its rim
between two drop points (a needle, a ridge crossing the row) and nothing about the ends bounds
it; a chord-sag bound needs the face. What is proven instead is per face kind: for the faces
whose closed form is concave along a move (a horizontal cylinder above its axis with a convex
column, the faces the mould core's check spent 90% on), the move's worst point is found exactly
(Concave.lean), so the check there is a closed form, not a search. The check stays in
generation (it decides where a row is split or stepped) and its time there fell 6.06 s → 0.79 s
on the mould core; the certifier (refuse-only, unchanged in what it accepts) gets the same speed.

## L1 (verify/forge-cam/ForgeCam/Drop.lean, 10 theorems, standard axioms)

`sd_antitone`, `column_compl_nonempty`, `sd_body_ge_of_column` (a body in the column is at least as
far: the column's gap transfers to the cutter and a narrow shank), `sd_column_ge_vertical`,
`sd_column_ge_radial` (the two depth caps), `drop_clear` (a tip above the drop's upper bound keeps
the column off the face), `ring_rho_ge`, `ring_bound`, `line_rho_ge`, `line_bound` (spheres,
horizontal cylinders and tori about a vertical axis reduce to the plane through the tool's axis).

Concave.lean (12 theorems, standard axioms): `disk_conj`, `disk_conj_at` (the disk's conjugate and
where it is attained), `top_concave`, `neg_profile_concave` (b convex and non-decreasing),
`end_of_outside`, `move_clear_of_outside`, `move_le_unconstrained` (a maximiser outside the move:
its nearer end decides; else the unconstrained maximum), `end_rho_ge`, `section_mono`,
`ball_section` (past the face's end: the column's section), `superadd`, `profile_superadd`
(b(√·) convex: b(a) − b(0) lower).

## Open (hypotheses and gaps; none is assumed proven)

1. **The 1D closed forms' candidates** (`disk_drop`, `bowl_drop`, `plane_support`): that each
   piece's stationary points and ends include the maximum. L6-tested; Lean open.
   Likewise `conj`'s maximiser ρ* (the profile's slope point; the interval it returns holds one)
   and its bound B(λ): L6-tested against sampling (`a_dome_cylinders_chord_bound_is_its_sampled_maximum`,
   within 1e-3 of the sampled maximum; `a_move_by_a_dome_cylinders_end_is_cleared_only_where_it_is`);
   Lean has the theorems that use them (Concave.lean), not the candidate enumeration.
2. **The sphere bound S − sd(centre)** needs a convex column (`Column::convex` checks the profile);
   the supporting half-space argument is not in Lean yet.
3. **Column containment** of each cutter (`Column::of`; `sd_body_ge_of_column` takes T ⊆ column as
   a hypothesis): L6-tested.
4. **lower3d's supersets** rest on the kernel's edge boxes (`CurveGeom::bbox`) for cylinders and
   cones; a superset makes checks stricter, never looser, but a Hit found on its extra part is
   not a real violation (the op is refused, never wrong).
5. **G4 (leftover) for 3D parts** is not computed: the section is 2.5D (`section` refuses
   non-prism faces), and the reply says so (`leftover.computed: false`). Its `certification:
   "full"` is G1–G3, G5 and G6, as for any document whose leftover is not computed.
6. **ZMap** (the 3D stock): tested, not in Lean; the machining document still uses the 2.5D stock
   (finishing ops record no voids, so a later op may be refused where stock is already gone).
7. **Push-cutter** is certified bisection of the drop (closed forms per evaluation), not closed-form
   fibre intervals; scallop and rest plan on grids (planning only: their paths are certified).
8. **Speed**: z-level roughing's exact offsets of staircase regions; tori about non-vertical axes and
   cones go to the branch and bound; a drop exactly at a rim tangency is left unresolved at its
   (safe) upper bound after 20k cells.
9. **L5**: the interval arithmetic against verify/ieee754, as for the 2.5D checker.
