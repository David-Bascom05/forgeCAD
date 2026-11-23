# The kernel against the exact curved reference (planes, cylinders, cones, spheres; tori)

`verify/forge-exact` now decides booleans of solids bounded by planes and the kernel's analytic
quadrics (cylinders, cones, spheres) exactly, over ℚ, with Lean proofs of the parts listed in
"What is proven" below. Its harness, `exact-curved`, gives each case to the kernel and to the
reference and judges the kernel's sequential and n-ary paths. Tori are in step 4 (below); procedural
surfaces are not (README §6.2).

Rebuild: `lake build` in verify/forge-exact, then
`cargo run --release -p forge-exact-fuzz --bin exact-curved -- [degen] [families] [lang] [band] [torus] [torusband]
[--seeds N] [--lang N] [--rule strict|snap] [--threads N] [--only NAME] [--shrink]`.
Repros: `cargo test --release -p forge-exact-fuzz --test repros -- --ignored` (the curved ones are
at the end of tests/repros.rs).

## The reference

### Step 1: real algebraic numbers (ForgeExact/Alg)

Core Lean, not Mathlib. Mathlib's `Polynomial` and `Real` are noncomputable, and the reference is
compiled from the definitions the proofs are about.

- **Semantics.** Every theorem holds in every *model of the reals*: an ordered field with ℚ
  embedded and the intermediate value property for rational polynomials. The model is a
  hypothesis, not an axiom; ℝ is one.
- **Numbers.** A number is a rational, or `(p, a, b)`. Its isolation is certified once, by a sign
  change and by Descartes' one-sign-change case on the Möbius transform. Both Descartes cases are
  proven by comparing terms, with no derivatives.
- **Proven:**
  - the sign of any polynomial at a number (`sign_spec`);
  - comparison with a rational (`cmpRat_spec`);
  - comparison of two numbers (`cmp_spec`);
  - negation and rational shift and scale;
  - sums and products (`add_spec`, `mul_spec`).
- **Certificates.** Every algorithm is certificate-checked. A zero sign needs a checked common
  factor that changes sign. A sum or product comes from a minimal polynomial found by untrusted
  linear algebra, checked by a bivariate reduction identity. A failed check or exhausted fuel
  answers `none`, never a wrong answer.

### Step 2: the arrangement (ForgeExact/Quad)

- **Solids.** A solid is a formula over `q > 0` for quadrics with rational coefficients, written
  from the stored f64 frames without normalising. A cone takes `|z| = 1` in its offset term,
  which moves it by about r·1e-16.
- **The decomposition.** It is the sign-pattern decomposition of the quadrics. A cylindrical
  algebraic decomposition refines it; `reg_kept` needs only compatibility and a nowhere-dense
  wall.
- **Proven:**
  - `box_bound`: an exact Taylor bound of a quadric over a box;
  - `sign_nbhd`: signs are open;
  - `wall_nowhereDense`: the wall is nowhere dense;
  - `quad_boolean`: the decomposition is compatible, so `reg_kept` applies;
  - `member_off_wall`: membership is decided exactly at any point off the wall;
  - `member_box`: a box that certifies every sign lies in one cell;
  - `pairCheck_spec`: S-lemma exclusions, λa + μb = Σ dℓ², checked coefficient by coefficient;
  - `empty_of_patterns`: emptiness from patterns.

All of these are in Axioms.lean's standard-axioms check, with no `sorry`.

### Step 3: the executable (ForgeExact/QRef)

- **Tools.** Each tool is the intersection of its faces' inner sides. The harness sends only such
  tools, and checks each against the kernel's classifier at 400 points.
- **Volume.**
  - A certified enclosure `[lo, hi]` from an octree of boxes that certify every sign. It is loose
    (about ±10% at the 12k-box budget); it is a hard bound only.
  - An estimate: Gauss–Legendre over vertical lines split exactly at the quadrics' roots, between
    the structural breakpoints of the sections. It is about 1e-9 on the degenerate pairs, and
    reports its own error from two resolutions.
- **Contacts.** These come from the rational roots of the pencil's determinant det(A_i + t A_j),
  reduced to the complement of a common null space.
  - A semidefinite member is a proven global exclusion; these give the emptiness proofs.
  - At every rational point of the contact locus (a point, a line, or a conic's rational points),
    a local second-order test on the tangent plane decides between a touch and a saddle. The
    kept local regions then decide non-manifoldness.
  - Algebraic contact points are decided with `Alg.Num.sign` when the member is globally
    semidefinite.
- **Cone apexes** lying on other surfaces get a tangent-cone check sampled over directions. Being
  sampled, it never decides: a flag makes the verdict ambiguous.
- **Near decisions.**
  - Typed gaps (sphere/sphere, parallel cylinders, cylinder/plane, sphere/plane, sphere/cylinder)
    and near-double roots of the pencil flag a near touch.
  - Under the snap rule, planes snap by `Snap.snapped`, and spheres and parallel cylinders merge.
  - A near touch that is left is a *sliver* under snap, and *ambiguous* under strict.
- **Hints.** Where the kernel refuses with a location, the reference examines that point itself:
  - two surfaces nearly tangent there, or three or more through it, make a near touch;
  - two planes crossing there with opposite sectors kept make the result non-manifold along the
    edge.

  It never takes the kernel's verdict.
- **Probes.** A probe is a point whose box of half-width 4·LINEAR certifies every sign; it is
  exactly in or out. The kernel's classifier must agree at every probe: 4·LINEAR is past the
  snap's 2·LINEAR reach. Probes are placed 6 and 60 LINEAR off every tool face, plus 96 spread
  over the box.
- **Verdict:**
  - `nonmanifold` at a decided contact;
  - `empty` when no box is in and every undecided box keeps no pattern its signs and the
    exclusions allow;
  - `solid` with a certified interior box;
  - `ambiguous` when a contact is undecided or neither is shown;
  - `sliver` under snap at a near touch.

### Step 4: tori, by polynomials of any degree (ForgeExact/Poly3, QRef/Poly.lean)

Rounds and pipes are tori, so this is the step that reaches fillets. Offsets of the analytic
surfaces are again planes, cylinders, cones, spheres and tori, so they need nothing more.
- **Lean (Poly3).** Polynomials in x, y, z of any degree, as monomial lists. The quadric results
  hold for them too, all in Axioms.lean:
  - escape: restricted to a line, a nonzero polynomial has at most its degree's roots
    (`zero_of_roots`, by synthetic division), so it is nonzero somewhere in every open set;
  - the box bound, taken about the box's centre (`about_bound`, `eval_about`);
  - sign neighbourhoods, and `wall_nowhereDense`;
  - `reg_off_wall`, `in_box`, `out_box` and `empty_of_leaves`.
- **The polynomial path.** A history with a torus face (a ring torus, 0 < r < R) goes here; the
  quadric path is unchanged.
  - Every face becomes a polynomial: a torus its quartic, the rest their quadrics, a cone its
    nappe as well.
  - The octree is the same as the quadric one, with the proven `Poly3.boxSign`, so `[lo, hi]`
    and `empty` are proven as before. The budget is 40k boxes, because tori need finer boxes.
  - The volume estimate splits vertical slices at the polynomials' roots, which are found in
    floating point. It integrates by Gauss–Legendre between the points where the slices change
    structure, and reports a two-resolution error. That error is about 3e-4 relative on a thin
    hollow torus.
- **Manifold check, by hand (curved-l2-proofs.md §5).** Each undecided leaf is accepted or split
  until one of these holds:
  - it has one undecided surface, with a gradient component of certified sign: a smooth sheet;
  - it has two, with a component of their gradients' cross product of certified sign
    (transversal), and their sectors are not kept as a checkerboard;
  - it has three, with the determinant of their gradients of certified sign, and the kept octants
    form a disc on the link.

  A leaf left unaccepted makes the verdict ambiguous, so `solid` means proven manifold. Two
  undecided surfaces with nearly parallel normals stop the check at once, as a near tangency.
  That shortcut can only make the verdict ambiguous.
- **Touches, exact.** These touches of a torus are decided in ℚ:
  - with a coaxial plane, cylinder, sphere or torus, by tangency in the meridian half-plane,
    which gives a rational contact circle;
  - with a plane at a support point, by the torus's support function `R|n_⊥| + r|n|`, when its
    roots are rational.

  The other carriers' signs are proven constant along the contact (`noRealRoot`). The pinched
  rule then decides the touch, and a non-manifold one makes the verdict `nonmanifold`. Each
  touch's empty sign combination also lets the local check drop a surface the result does not
  depend on there, so manifold touches can be shown `solid`.
- **The estimate** also breaks slices at walls parallel to them, which add no root but change
  the slice. Before this fix, plane_torus/tangent_top_and_bottom/Keep came out 1.5% off: the
  estimate gave 23 314, and the kernel's 22 974.22 matches an independent integral. It now
  uses two pieces per breakpoint interval, about 20 s a case; that case's error is 0.4 and its
  reported error 1.1.
- **Torus pairs: 74 agree, 0 wrong, 16 ambiguous.** The ambiguous ones:
  - Keep cases whose result is empty, which the octree cannot prove without the touches'
    exclusions;
  - cones with tori: coaxial cones are not in the meridian test, and apexes on a torus are
    not decided;
  - standing_tangent_top_inside, a support point inside the box's top.

### What is proven, and what is not (PROOF-STANDARD levels)

- **L1 (spec over ℝ / every model):** done for the quadric boolean over ℚ³. The results are
  `quad_boolean` (`reg_kept`), `member_off_wall`, `member_box`, `pairCheck_spec` and
  `empty_of_patterns`, and the algebraic numbers.
- **L2 (executable proven equal):** partial. The executable uses the proven checkers
  (`boxSign`, `pairCheck`, `Num.sign`, the formula on patterns). These are not proven:
  - the octree traversal (that the in-boxes cover only the result, and that emptiness follows
    from the leaves);
  - the local touch/saddle test and its non-manifold rules;
  - the near-decision tests, the hints and the volume estimate (a report only);
  - the reading of a tool as the intersection of its faces' inner sides (sampled in the harness,
    the analogue of stage 1's Jordan–Brouwer assumption).

## Totals (snap rule, per kernel path)

| source | agree | wrong | refused | ambiguous | out of scope |
|---|--:|--:|--:|--:|--:|
| degenerate pairs (279: plane/cylinder/cone/sphere × configs × Add/Cut/Keep) | 240 | 19 | 3 | 17 | 0 |
| torus pairs (93: plane/cylinder/cone/sphere/torus with a torus × configs × Add/Cut/Keep) | 74 | 0 | 0 | 16 | 3 |
| scoreboard families (36 × 4 seeds, convex-quadric sub-histories) | 86 | 4 | 0 | 5 | 21 |
| forge_lang programs (300, extrude/revolve/ball/hole/pattern, convex-quadric sub-history) | 200 | 0 | 0 | 0 | 100 |
| curved band (279 pairs × tool moved ±0.9, 3 LINEAR on x, y, z = 2 511) | 2 191 | 50 | 102 | 168 | 0 |

The sequential and n-ary paths agree with each other except in the band (n-ary 2 206 / 50 / 87 /
168). The families and lang rows are from the run before the sampled apex check stopped deciding
verdicts; one family "wrong" (degen_cone_sphere/2, an apex contact) is ambiguous under the final
rules. Out of scope:
- 90 cases where the reference took over 30 s (many-carrier family parts; the octree's rational
  arithmetic is the cost);
- 82 where the history combines before any part exists;
- 40 with several bodies;
- 30 with procedural faces.

Under the strict rule the degenerate pairs give 244 agree / 9 wrong / 3 refused / 23 ambiguous:
the four extra ambiguous are near touches, which the snap rule calls slivers.

## Bugs (each a repro at the end of tests/repros.rs)

### Point contacts built instead of ZeroThickness: 19 degenerate pairs and their band variants

The union or difference meets itself at one point. By §6 the result is non-manifold, and the
kernel must refuse with ZeroThickness, as it must for the flat `cube_on_a_corner_vertex`. It
builds a body that passes `forge_topo::check`.

The cases:
- plane_sphere: tangent_top_above/Add, tangent_top_inside/Cut, tangent_top_and_bottom/Cut;
- sphere_sphere: tangent_outside/Add, equal_tangent_outside/Add, tangent_inside/Cut;
- cylinder_sphere: pole_on_cap/Add, pole_on_bottom_inside/Cut;
- cone_sphere/pole_on_base_centre/Add;
- cone apexes on a surface, decided by the exact apex test (curved-l2-proofs.md §3):
  - plane_cone/apex_on_vertex/Add and apex_on_edge/Add;
  - cylinder_cone/apex_on_wall/Add;
  - cone_cone/apex_on_surface/Add;
  - cone_sphere/pole_on_apex/Add;
- cone apexes on a face normal to the axis, and two cones sharing an apex:
  - plane_cone/apex_on_top_from_above/Add and apex_on_top_from_inside/Cut;
  - cylinder_cone/apex_on_cap_centre/Cut;
  - cone_cone/apex_to_apex/Add and inverted_overlap/Cut.

  The reference first said *solid* on these five, because it took the face for the nappe's
  selector and skipped the apex. fix-touch found the gap: its kernel refuses them. The reference
  now decides them from the apex's link.

Tests: `ball_resting_on_the_top_is_non_manifold`,
`cavity_touching_the_top_from_inside_is_non_manifold`, `balls_touching_at_a_point_are_non_manifold`,
`cone_apex_on_a_face_normal_to_its_axis_is_non_manifold`. Owner: the touch-decisions session.

### SplitThroughSingular: a ball centred on a cylinder's rim

cylinder_sphere/centre_on_rim, Add, Cut and Keep, and all 9 band moves. The union is an ordinary
solid: the reference certifies interior boxes and finds no contact. The kernel refuses
`Unsupported(SplitThroughSingular)` on the sphere, whose pole lies on the rim.

Test: `ball_centred_on_a_rim_builds`. Owner: the touch-decisions session.

### The band: features within a few LINEAR of a degenerate position

| kind | cases | test |
|---|---|---|
| a body with NaN volume | cylinder_sphere/tangent_wall_equator/Cut x 3 | `band_ball_on_wall_cut_has_a_volume` |
| a wrong volume: 4 190.78 mm³ kept from a 4 188.79 mm³ ball | tangent_wall_equator/Keep x 3 | `band_ball_on_wall_keep_volume` |
| NotClosed (an edge on one face) | identical cylinders z 3; dome_on_cap x 3, z −0.9; plane_cone base_on_top z −0.9, countersink_rim_in_top z 0.9 | `band_identical_cylinders_moved_close` |
| invalid VertexTolerance (3e-5) | cone_cone identical, tip_shares_surface x 3; cylinder_cone apex_on_cap_centre z 3; plane_cone apex_on_top_from_inside z 3; centre_on_rim x 3 | `band_identical_cones_valid` |
| SplitCrossed on a sphere face | plane_sphere centre_on_edge, centre_on_vertex, centre_on_wall_tangent_top (all ops, ±0.9, 3); cylinder_sphere dome_on_cap, tangent_wall_equator y 3 | `band_ball_on_an_edge_splits` |
| HoleOutside on a cylinder face | cylinder_cylinder/perpendicular_tangent_inside y −0.9 (all ops) | `band_tangent_inside_cylinder_holes` |
| InconsistentClassification, RetryInvalid | plane_cone/lying_on_top/Cut z −0.9; sphere_sphere/through_pole_on_seam/Add y 3 | — |

Owner: the fix-curved session.

### Not bugs, or not decided

- **cylinder_cone/base_rim_on_wall/Cut.** The cone's base rim touches the cylinder wall: an
  edge-on-surface contact the reference does not analyse. The kernel's ZeroThickness is judged
  right only through the hint check (three surfaces through the point). This is recorded as a
  reference gap, to be closed next.
- **coincident_walls 1 and 2** refuse ZeroThickness correctly. The stage is non-manifold along a
  planar edge at z = 7, which the hint check confirms exactly, as the flat comparison found.
- **Ambiguous (11 pairs):**
  - three apex-on-corner intersections whose emptiness pairwise certificates can't prove;
  - three spheres touching on an edge of a third surface;
  - two skew equal cylinders with an algebraic contact point and no global certificate;
  - the apex contacts above.
- **Band, 133 ambiguous:** thin keeps that the octree cannot certify in or prove empty.

## Still open in the reference

- Torus touches still undecided: coaxial cones, a cone apex on a torus, non-axis-aligned tori,
  saddle tangencies (a plane or cylinder at the inner equator from the hole's side), and empty
  Keep results. Proving those empty needs the touches' exclusions in `anyKept`.
- A second singular quadric through a cone's apex (two cones of different angles sharing an
  apex) is undecided. Two cones sharing the apex and the quadric are decided (§3).
- Edge-on-surface contacts (the base_rim_on_wall gap) stay undecided.
- Speed: 90 family cases took over 30 s.
- The families and lang rows predate the apex fix (the degenerate pairs were rerun).
