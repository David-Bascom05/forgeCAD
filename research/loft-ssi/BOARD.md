# Loft x loft intersection: shared research board
Goal (David, 2026-11-01): an exact formula (even a complex one) for where two smooth lofts
(forge-geom Skinned surfaces) meet, replacing numeric marching; the pump volute's loft-core
cut costs ~116 G instructions (12.6 s).
Agents: prior-art, lean-derive, novel-math, plus loft-speed (profiling/optimizing the current tracer).
Append findings here (dated, signed with your role), and message each other directly for anything
that changes another's direction.

## The Skinned surface, exactly (prior-art, 2026-10-31; from crates/forge-geom/src/skinned.rs, forge-ops/src/loft.rs, skin.rs)
NOTE: the blend across sections is NOT a local Hermite blend. It is a global natural cubic spline
(C2; or end-clamped when the loft has start/end slopes) with chord-length knots s_0 < ... < s_{n-1}
(s_k = cumulative distance between profile centroids). n <= 14 sections. Each face is ONE span.

On span i (s in [s_i, s_{i+1}], h = s_{i+1}-s_i, a = (s_{i+1}-s)/h, b = 1-a), datum d's weight:
    w_d(s) = a*[d==i] + b*[d==i+1] + h^2/6 * ((a^3-a) M[d][i] + (b^3-b) M[d][i+1])
M[d][.] = second derivatives of the cardinal spline of unit datum d (tridiagonal solve, constant per loft).
=> every w_d is a CUBIC POLYNOMIAL in s on the span (nonzero for all d: global support), sum_k w_k = 1.
The face is carried on half a span past each end in s (same cubic) and up to 0.25 past [0,1] in t.

    S(t,s) = sum_k w_k(s) C_k(t)  +  sum_j w_{n+j}(s) D_j  +  sum_g beta_g(t) E_g(s)
 - C_k(t): the matched segment j of profile k (prepare() splits a circle into two arcs of sweep +-pi):
     Line: C_k(t) = A_k + (B_k-A_k) t                         (degree 1 in t)
     Arc:  C_k(t) = O_k + R_k (X_k cos(a_k + sg_k t) + Y_k sin(a_k + sg_k t))
   (Rail::Parabola / Rail::Wrap exist in the type but loft only builds Line and Arc.)
 - D_j: end-slope vectors (planes[k].z * weight), constant.
 - Guides (Shift): beta_g(t) in {1-t, t, 1}; E_g(s) = G(p(s)) - sum_k w_k(s) V_k - sum_j w_{n+j}(s) D_j,
   with p(s) = sum_k m_k(s) p_k a cubic (spline of arc lengths) and G a line/arc chain BY ARC LENGTH:
   on an arc piece G(p) = O + R(x cos(p/R + c) + y sin(p/R + c)) => cos/sin of a CUBIC in s: transcendental in s.

### What class each wall is in (per span)
 1. Line walls, no guides: S = P(s) + t Q(s), P,Q cubic polynomial vectors => a RULED surface,
    polynomial bidegree (1,3); implicit degree <= 2*1*3 = 6.
 2. Arc walls, no guides, all sections' |sweep| equal (always true for circle profiles: sweep = +-pi):
    with phi = sg*t, cos(a_k+phi) = cos a_k cos phi - sin a_k sin phi, so
        S = c(s) + X(s) cos phi + Y(s) sin phi,   c, X, Y cubic polynomial vectors in s.
    Every s = const section is an ELLIPSE (centre c(s), conjugate semi-diameters X(s), Y(s)).
    With u = tan(phi/2): rational bidegree (2,3), weight (1+u^2): implicit degree <= 2*2*3 = 12.
 3. Arcs with different sweeps, or a line matched with an arc at the same segment index:
    a mix of t and cos(sg t), or cos(sg1 t), cos(sg2 t): NOT algebraic (exponential-polynomial / trig);
    only commensurate sweeps (p/q) make it algebraic, via Chebyshev, at much higher degree.
 4. Any guide whose path has an arc: transcendental in s (cos of a cubic). Line-only guides stay cubic.
So the exact-algebra route (implicitization, resultants) only applies to classes 1 and 2.

### The benchmark (repros/vf6.fc): volute (9 circle profiles) cut by core (10 circle profiles)
Profiles 1..9 share the SAME frames and centres in both lofts; core radii = volute radii - 5 for the
first 7, then 28.5 vs 33.5, 32 vs 37, and the core adds a 10th profile at y=125 (r=33).
Each profile: frame x = (cos th, sin th, 0) radial, z tangent; so Y_k = frame y = (0,0,+-1) constant for all k
=> Y(s) = R(s) * (0,0,+-1) with R(s) = sum w_k R_k, and X(s) = sum w_k R_k x_k horizontal, X . Y = 0:
each section is an ellipse with perpendicular axes, |X(s)| < R(s) (averaged unit vectors shrink) - nearly circles.
The two lofts' spline weights DIFFER (core has 10 knots, natural spline is global) even on shared spans.
Both are class 2: c(s) + X(s) cos phi + Y(s) sin phi.

## Prior art (prior-art, 2026-10-31)
Today's tracer (forge-geom/src/intcurve.rs) marches and refines in 3D with implicit_gradient on both
surfaces; for Skinned that value is a signed distance found by a nearest-point Newton projection
(Skinned::solve -> polish), so every step and every later IntCurve evaluation pays two projections.
No production kernel or paper intersects two parametric surfaces that way; all of them below work
in the surfaces' own parameters.

Notation: A(s,phi) = c(s) + X(s) cos phi + Y(s) sin phi, B(r,psi) = d(r) + U(r) cos psi + V(r) sin psi
(class 2). For class 1 replace cos/sin by the line parameter t.

### M1. Parametric system + certified subdivision + parametric marching (the standard; works for ALL classes)
 F(s,phi,r,psi) = A(s,phi) - B(r,psi) = 0: 3 equations, 4 unknowns, solution = the intersection curve.
 - Marching: Barnhill & Kersey, "A marching method for parametric surface/surface intersection",
   CAGD 7 (1990); Patrikalakis & Maekawa, Shape Interrogation for CAD/CAM (Springer 2002) ch. 5
   (free: https://web.mit.edu/hyperbook/Patrikalakis-Maekawa-Cho/); OCCT IntWalk_PWalking /
   IntPatch_PrmPrmIntersection (parametric start points from a polyhedral pre-pass, then walking).
 - Completeness (every branch, every closed loop): subdivision with normal/tangent-cone tests.
   Sederberg & Meyers, "Loop detection in surface patch intersections", CAGD 5 (1988);
   Kriezis, Patrikalakis & Wolter, "Topological and differential-equation methods for surface
   intersections", CAD 24(1) (1992) (a loop forces a point with collinear normals);
   Hanniel & Elber, "Subdivision termination criteria in subdivision multivariate solvers using dual
   hyperplanes", CAD 39 (2007) (cone test: <= 1 root in a box);
   Barton, Elber & Hanniel, "Topologically guaranteed univariate solutions of underconstrained
   polynomial systems via no-loop and single-component tests", CAD 43(8) (2011) 1035-1044:
   No-Loop Test = the tangent image of the solution curve over the box lies in a half-sphere
   (from cones of the gradients); Single-Component Test = exactly two boundary crossings. A box passing
   both holds exactly one arc, which is then traced numerically with guaranteed topology.
   Mizrahi & Elber, CAD 58 (2015) (2D-solution analogue; https://gershon.cs.technion.ac.il/papers/MVBivarSolver.pdf).
 - What it needs from us: point+first derivatives (Skinned::deriv already gives them), and BOUNDS of F and of
   its gradients over a parameter box. Elber gets bounds from Bernstein/B-spline control points (needs
   polynomial/rational form: class 1, 2 via tan(phi/2), bidegree (3,2,3,2), 4*3*4*3 = 144 coefficients per
   coordinate). For classes 3-4 (trig of t or of a cubic) use interval or affine arithmetic instead:
   Skinned already has Spline::weight_box; sin/cos of an interval is trivial. Affine arithmetic is much
   tighter for this: Lin, Qin, Liao & Xiong, "Affine arithmetic-based B-spline surface intersection with
   GPU acceleration", IEEE TVCG 20(2) (2014).
 - Degrees: none to expand. Cost: one deriv() of each surface (~100-200 flops for our walls) + a 3x4
   min-norm solve per Newton iteration; subdivision boxes cost an interval evaluation each.
   Expected: well under 1 ms per face pair once start points are found (vs 12.6 s today).
 - Robustness: topology guaranteed away from tangential contact (cone tests fail to terminate at
   tangencies: stop at a size floor and hand that box to a singular-point handler, as Barton et al.).
 - Certified tracing alternative to cone tests: interval Krawczyk over a whole step:
   Burr, Byrd & Lee, "Certified algebraic curve projections by path tracking", ISSAC 2025, arXiv:2502.05357;
   Duff & Lee, "Certified homotopy tracking using the Krawczyk method", ISSAC 2024;
   Burr, Hauenstein & Lee, arXiv:2602.07718 (2026). Needs only interval extensions of F and its Jacobian
   (analytic, NOT necessarily polynomial), so it covers the trig classes too.
 - Topology-guaranteed B-spline SSI, the current state of the art, both CAS (Chinese Academy of Sciences):
   Yang, Jia & Yan, "Topology guaranteed B-spline surface/surface intersection", ACM TOG 42(6) (SIGGRAPH Asia 2023),
   doi:10.1145/3618349 (beats SISL, OCCT, ACIS on hard topology at practical speed);
   Cheng, Zhang, Xiao & Li, "Topology-driven approximation to rational SSI via interval algebraic topology
   analysis", SIGGRAPH 2023 (4D interval box, critical points classified into 4 cases; beats IRIT, SISL, Parasolid).
   Both need polynomial/rational input; their ideas (critical points first, then trace) carry over.

### M2. Family-of-sections reduction ("ringed surface" method): an exact bivariate formula
 Heo, Hong, Seong, Kim & Elber, "The intersection of two ringed surfaces and some related problems",
 Graphical Models 63(4) (2001) 228-244, doi:10.1006/gmod.2001.0553; predecessor for lines:
 Heo, Kim & Elber, "The intersection of two ruled surfaces", CAD 31(1) (1999) 33-50;
 general sweeps: Seong, Kim, Kim, Elber & Martin, "Intersecting a freeform surface with a general swept
 surface", CAD 37(5) (2005) 473-483. Idea: a surface swept by a one-parameter family of simple curves meets
 another such surface exactly where section A(s) meets section B(r); eliminate the in-section parameters in
 closed form and you get ONE scalar equation lambda(s,r) = 0 in the plane of the two sweep parameters,
 with a closed-form (rational) map from each solution back to the 3D point.
 - Ruled vs ruled (our class 1 vs class 1, rulings along t): exactly Heo 1999:
     delta(s,r) = det(Q_A(s), Q_B(r), P_A(s) - P_B(r)) = 0, polynomial bidegree (6,6) for our cubic P,Q;
   s-line and r-line parameters then follow by a 2x2 linear solve (rational in s,r). Redundant solutions only
   where rulings are parallel (|Q_A x Q_B| = 0), classified in the paper.
 - Ellipse family vs ellipse family (class 2 vs class 2), derived here for us:
     plane of A(s): n(s) = X x Y (degree 6); B(r)'s ellipse crosses it where
       p cos psi + q sin psi = -e,  p = n.U, q = n.V, e = n.(d - c)        (closed form, 0 or 2 psi, as
       GuidePath::crossings already does for arcs)
     then the crossing point Q = B(r,psi) - c(s) lies on ellipse A(s) iff
       h = (Q.(Y x n))^2 + (Q.(n x X))^2 - |n|^4 = 0.
     lambda(s,r) = h(psi+) * h(psi-) is symmetric in the two crossings, so it is a polynomial in the data
     (a resultant); expanded it has bidegree about (84,24) in (s,r) - never expand it, evaluate it in closed form.
     Or keep psi: two equations g(s,r,psi) = 0, h(s,r,psi) = 0 in 3 unknowns, a regular space curve.
   Volute special case (every section plane contains the z axis direction, Y = R(s) z-hat, X horizontal):
     plane: (Q_h x X) . z = 0 (degree 6 in s), ellipse: (Q.X)^2 R^2 + Q_z^2 |X|^2 - |X|^2 R^2 = 0
     (degree 18 in s, 6 in r, trig degree 2 in psi).
 - Ruled vs ellipse family: Heo 2001 section 5 (line vs circle per (s,r)); same construction.
 - What it needs: the sweep structure (section curves of the family are lines/conics with closed-form
   intersection). Holds for classes 1 and 2 and for guide pulls along lines; NOT for class 3 (sections are
   not conics when sweeps differ / lines mix with arcs). Class 4 (arc guides) is still fine: the guide only
   moves c(s), and the reduction never needs c to be polynomial - it only needs, for fixed s, a planar
   conic section. (Check: with Along::Start/End guide pulls the section is NOT a conic, beta(t) mixes in.)
 - Robustness: the answer is a plane curve in (s,r), so all certified plane-curve machinery applies:
   start points = boundary roots (1D, closed form or univariate solve) + turning points (lambda = lambda_r = 0,
   2x2 systems isolated by interval Newton/Krawczyk); between critical s-values every branch is an
   x-monotone arc (Arnon/CAD style). Degenerate cases: n.U = n.V = 0 (B's ellipse parallel to A's plane,
   coplanar sections) - classified like Heo's parallel rulings.
 - Speed: one lambda evaluation ~ a few hundred flops (cubic weights, 2 cross products, atan2/acos);
   tracing a 1D implicit curve in 2D needs a 1-unknown Newton per point. Fastest exact option for class 2.
 - "Exact formula": yes in the ForgeCAD sense - the curve is {lambda(s,r) = 0} with an explicit lift.

### M3. Algebraic: implicitize one surface, substitute the other, then plane-curve topology
 Implicitization: Sederberg & Chen, "Implicitization using moving curves and surfaces", SIGGRAPH 1995;
 Cox, Goldman & Zhang, "On the validity of implicitization by moving quadrics for rational surfaces with no
 base points", JSC 29 (2000); Busé & Chen, "Determinantal tensor product surfaces and the method of moving
 quadrics", arXiv:2006.16655. Matrix representations (no expansion, rank drop on the surface):
 Busé, "Implicit matrix representations of rational Bezier curves and surfaces", CAD 46 (2014);
 Busé & Luu Ba, "The surface/surface intersection problem by means of matrix based representations",
 CAGD 29 (2012); Shen, Busé, Alliez & Dodgson, line/trimmed NURBS via matrix representations, CAGD 2016.
 Resultant/eigen formulations: Manocha & Canny, "A new approach for surface intersection", IJCGA 1(4) (1991);
 Krishnan & Manocha, "An efficient surface intersection algorithm based on lower-dimensional formulation",
 ACM TOG 16(1) (1997) 74-106 (projection of the curve as a plane algebraic curve, start points + turning
 points by eigenvalue problems, then tracing).
 Plane-curve topology (certified): Gonzalez-Vega & Necula, "Efficient topology determination of implicitly
 defined algebraic plane curves", CAGD 19 (2002); Alberti, Mourrain & Wintz, "Topology and arrangement
 computation of semi-algebraic planar curves", CAGD 25 (2008) (Bernstein subdivision);
 Kerber & Sagraloff, "A worst-case bound for topology computation of algebraic curves", JSC 47 (2012);
 Berberich, Emeliyanenko, Kobel & Sagraloff, "Exact symbolic-numeric computation of planar algebraic
 curves", TCS 491 (2013) (CGAL).
 - For us: class 1: implicit degree 6; class 2: degree 12 (2mn for bidegree (2,3)). Substituting B
   (bidegree (2,3)) into A's degree-12 implicit: plane curve of bidegree (24,36) in (tan(psi/2), r):
   25*37 = 925 coefficients, coefficient growth makes f64 unusable: needs exact rationals (a bigint we
   would have to write: zero dependencies). Base points / non-generic cases make moving-quadric matrices fail.
 - Classes 3-4: not applicable at all.
 - Speed: implicitization per surface pair ms..s in exact arithmetic; topology of a degree-~60 curve
   seconds in CGAL-class code. Exact and certified, but slower than today's 12.6 s target only with a lot of
   engineering. Useful offline (Lean/SymPy) to CHECK M1/M2 results, not as the kernel path.

### M4. Bezier clipping / pure subdivision without tracing
 Sederberg & Nishita, "Curve intersection using Bezier clipping", CAD 22(9) (1990); Nishita, Sederberg &
 Kakimoto, "Ray tracing trimmed rational surface patches", SIGGRAPH 1990. Quadratic convergence for
 transversal points; for curves (1D solution sets) it produces strips of boxes (Lin et al. 2014 thin them).
 Needs Bernstein form (classes 1-2 only). Good as the box pre-pass of M1, not on its own.

### Kernels
 OCCT: IntPatch_PrmPrmIntersection + IntWalk_PWalking (parametric walking from sampled start points, no
 topology guarantee: Yang 2023 shows failures). SISL: s1859 subdivision + s1313/s9iterate marching.
 ACIS / Parasolid: procedural sweep/loft surfaces carry a B-spline approximation; SSI is done on it and the
 intersection curve is stored procedurally (pairs of parameter curves, "SP-curves" in Parasolid,
 Jackson, "Boundary representation modelling with local tolerances", SMA 1995). None ship an exact formula
 for loft-loft; all are numeric marching + B-spline fits. Nothing found for intersecting two cubic-spline
 skins of conic sections specifically: M2 applied to elliptic sections appears to be new work.

## 2026-10-31 loft-speed: where volute's 116 G go (seq, instructions:u)
Per feature (prefix differences): loft new 1.0G, casing extrude 6.9G, flange 0.7G, seal 0.1G,
**core loft cut 72.0G**, chamber cut (cyl) 22.5G, dbolt pattern 1.1G, vent 2.2G, foot 3.5G, chamfer 5.4G, rest <0.5G.
Per function: 93% of all instructions is Skinned::implicit/gradient = a nearest-point projection
(Skinned::solve: seed scan over 153 grid points + Newton `polish`), ~14.5k instructions per solve,
7.44M solves per regen. Data::point 45% self, solve (seed scan) 17%, Spline::weights 10%, memmove of
the 384-byte Weights 12%.
Inside intcurve::trace (core cut prefix): octree seeding 3.73M solves (64%), seeds 299k; refine+march+
finish 2.06M solves for only 16.4k chain points (~125 solves per kept point: most go to refining seeds
that never converge / lie on the extensions).
Pairs: the core cut calls surfaces_in on 188 Skinned x Skinned pairs (149 distinct), 8.5 s of the run.
126 return empty; 62 return 1-4 curves, but every one of those curves lies on the carried-on extensions
(e.g. volute span s in [0,74.8] vs core span [151.9,231.2]; curves at |z|~31 on the extended walls),
i.e. no SkxSk curve the boolean keeps. Confirms prior-art: the walls never meet, the time is spent
finding nothing on the extensions plus proving nested 5 mm walls disjoint by octree.
Next (loft-speed): prune trace seeds to the faces' own parameter domain (patch boxes), then cheaper
Skinned evaluation (per-span polynomial form), then parametric marching in (u1,v1,u2,v2).

## Where the vf6 time goes, and a certified exclusion that removes it (novel-math, 2026-10-31)
Measured (scratch log around intcurve::trace in forge-bool ssi.rs, release, vf6.fc 10.7 s):
 - 393 traced pairs in the core cut take 9.9 s. 268 of them are Skinned x Skinned (8.6 s, median 30 ms,
   max 115 ms), plus 130 Skinned x plane/cylinder (1.3 s). perf: about 95% is in Skinned::solve, the
   nearest-point Newton behind implicit(), which the octree calls at every cell centre.
 - The face-box prefilter keeps 268 of about 288 possible core x volute wall pairs.
 - In the TRUE geometry the walls never meet (py/gap.py): every core point has elliptic radius at most 0.879
   in the volute's section through it. The 76 "non-empty" traces are crossings of the CARRIED-ON
   extensions (half a span past each face), which trimming then throws away.
So in the benchmark the cost is entirely in proving emptiness.

Certificate (py/certify.py): a piece s in I, a in J of S = c(s) + X(s) cos a + Y(s) sin a is EXACTLY a
rational Bezier patch of bidegree (3,2): control points c_i + X_i cx_j + Y_i sy_j (the Bernstein points of
the cubics over I, times the arc's rational quadratic control points over J, weights 1, cos h, 1 > 0).
It therefore lies in the convex hull of its 12 points. Test two hulls against each other with a separating
axis (x, y, z, both piece normals, the centroid difference); if no axis separates them, split the bigger
piece along its longer side.
Result on the volute (face domains, all 8 x 9 span pairs x 4 x 4 quarter arcs): ALL certified disjoint,
5432 hull tests in total (16 per far pair; the worst pair, both span 5, needed 394). That is about 2k
instructions per test, roughly 10-20 M instructions in all, against ~100 G. Exact: no sampling and no
tolerance in the proof. Candidate pieces, where no axis separates them down to depth 14, would go on to
the exact solver (next section).
Prototypes: research/loft-ssi/novel/py/{loft.py,gap.py,certify.py} (numpy only).

### Finding: in vf6 the two lofts' walls never meet (prior-art, 2026-10-31)
research/loft-ssi/prior-art/proto/m2_volute.py, m2_probe.py (numpy, rebuilt from the skinned.rs math):
knots 0, 74.83, ..., 592.45 for both lofts (the core adds 607.45). For every s in the volute's faces, the core
points lying in the volute's section plane sit inside the volute's section ellipse with radial margin 30% at
s=0 falling to 13.5% at s=592 (about 5 mm); the section-pair function lambda(s,r) never changes sign on a
400x400 grid. The core leaves through the volute's end cap plane (y=110) and shares its start plane.
=> the loft-vs-loft cost in the benchmark is proving an EMPTY intersection. Also: shared section planes
make M2's plane-crossing angle ill-conditioned (B's ellipse almost lies in A's plane); M2 needs a
coplanar-family branch (2D conic vs conic in the shared plane).

### Ranking for our case (prior-art)
1. M1 with interval/affine box bounds: (a) march in (s,phi,r,psi), no projections; (b) prove empty pairs
   and find all start points by subdivision with the cone / no-loop / single-component tests
   (Barton-Elber-Hanniel 2011). Covers every class incl. transcendental; f64 only; the vf6 case becomes a
   handful of box rejections.
2. M2 section-family reduction (Heo et al. 1999/2001) for classes 1-2 (+ Whole guides): the exact formula
   lambda(s,r) = 0 with a closed-form lift; fastest per point; needs the coplanar branch and Krawczyk-isolated
   turning points for certified topology.
3. M3 implicitize + plane algebraic curve topology: exact, certified, but degree 12 implicit / bidegree
   (24,36) curves need exact rationals; classes 1-2 only. Use offline (Lean/SymPy) to check 1 and 2.

## README background draft (for research/loft-ssi/README.md in the repo; prior-art)
Background. ForgeCAD's smooth lofts are cubic-spline skins: on one span S(t,s) = sum_k w_k(s) C_k(t)
with cubic cardinal-spline weights and line or arc sections, so a circle-profile wall is
c(s) + X(s) cos phi + Y(s) sin phi, a cubic family of ellipses (rational bidegree (2,3), implicit degree 12).
Methods considered:
- Implicitize and substitute, then compute the topology of the plane algebraic curve [Sederberg-Chen 1995;
  Busé-Chen 2020; Busé-Luu Ba 2012; Manocha-Canny 1991; Krishnan-Manocha 1997; Gonzalez-Vega-Necula 2002;
  Kerber-Sagraloff 2012]. Dropped as the kernel path: degree 12 implicits and bidegree (24,36) plane curves need
  exact big rationals, and lofts with unequal arc sweeps, line/arc pairs or arc guides are not algebraic at all.
- Section-family reduction [Heo-Kim-Elber 1999; Heo-Hong-Seong-Kim-Elber 2001; Seong et al. 2005]: two
  surfaces swept by conics meet where section A(s) meets section B(r), one scalar equation lambda(s,r) = 0
  with a closed-form lift. Exact and fast for line/ellipse walls; degenerate when the families share planes.
- Parametric system A(s,phi) = B(r,psi) with certified subdivision and marching [Barnhill-Kersey 1990;
  Sederberg-Meyers 1988; Kriezis-Patrikalakis-Wolter 1992; Hanniel-Elber 2007; Barton-Elber-Hanniel 2011;
  Yang-Jia-Yan 2023; Cheng-Zhang-Xiao-Li 2023], with interval/affine bounds [Lin et al. 2014] or Krawczyk
  certificates [Duff-Lee 2024; Burr-Byrd-Lee 2025] so it holds for non-polynomial walls too.
- Bezier clipping [Sederberg-Nishita 1990]: needs Bernstein form; at most a pre-pass.
Kernels (OCCT IntWalk_PWalking, SISL, ACIS, Parasolid) march numerically in parameters on B-spline
approximations of procedural surfaces; none has a closed form for loft-loft.
(Full citations with DOIs/links: BOARD.md "Prior art" section.)

### Prior art for novel-math's lambda-line resultant (prior-art, 2026-10-31)
No paper found with this exact elimination: null line of the 3x4 system, then the resultant of two quadratics in lambda.
The closest is Heo et al. 2001 (eliminates through the plane-crossing angle). Pencil-of-quadrics work is related
but different: Levin 1976; Wang, Goldman & Tu, ACM TOG 22(2) 2003; Dupont, Lazard, Lazard & Petitjean, JSC 43 2008.
Phantom real zeros of F need a complex-conjugate common root, so the two quadratics must be proportional.
That makes them generically isolated points (acnodes), not branches.
Two remedies:
 - Trace the lifted real curve Q1 = Q2 = 0 in (s, sigma, lambda) instead of F (Krishnan & Manocha 1997);
   use F only to project and to find critical s-values.
 - Where the first subresultant S1 vanishes on F = 0, check whether a real lambda exists, with lambda* = -S0/S1
   elsewhere (Gonzalez-Vega & Necula 2002; Seidel & Wolpert 2005).

## Derivation (lean-derive, 2026-10-31) - scripts in derive/ (venv/bin/python <script>)
Agreed definition: prior-art's "Skinned surface, exactly" above (checked against skinned.rs/loft.rs/skin.rs).
Volute/core structure used below: every section frame has y = -e_z, centre on z = 0, circles split at angle 0/pi
with the SAME phi = pi*u (+pi) in every section. So per span, in the local tau in [0,1]:
    S(tau, phi) = P(tau) + cos(phi) A(tau) - sin(phi) rho(tau) e_z,   P, A horizontal (R^2) cubics, rho scalar cubic.
(sin phi >= 0 on the first half face, <= 0 on the second; z^2 = rho^2 (1 - c^2) covers both halves at once.)

### R1. Exact intersection, section-pair projection: F(s1, s2), bidegree (12, 12)   [derive/volute.py]
 2D cross u^v = u.x v.y - u.y v.x.  D = P2(s2) - P1(s1), W = A1^A2, a1 = D^A1, a2 = D^A2.
    F = rho1^2 (W^2 - a2^2) - rho2^2 (W^2 - a1^2)
 Theorem (Lean, in progress): for W != 0, rho1,rho2 != 0: the two walls share a point with section params (s1,s2)
 <=> F = 0 and W^2 >= a2^2; then cos phi1 = a2/W, cos phi2 = a1/W, sin phi1 rho1 = sin phi2 rho2 (sign: same z).
 No spurious branch: the z -> -z mirror maps the squared branch onto the real one. Degree is tight (fixed ellipse 2
 in a vertical plane meets the degree-12 implicit of wall 1 in 24 points = 12 mirror pairs).
 Volute knots coincide (0, 74.83, ..., 592.45; core adds 607.45), so F vanishes at every shared-knot corner
 (s,s) (coplanar concentric sections: W = D = 0), and along the whole diagonal s1 ~ s2 the two plane families
 nearly coincide: F, W, a1, a2 are all O(eps^2) there, real F = 0 branches (complex z) sit within 1e-3 of it.
 Bernstein subdivision of F alone does NOT terminate on diagonal span pairs (40k boxes, 8k left at depth 24).
 => F is the right exact formula for transversal walls, the WRONG certificate for nested same-family walls.

### R2. Foliation form (what to ship for the volute): 3 variables (s1, s2, c2), c2 = cos phi2 in [-1, 1]
 Wall-1 sections are vertical planes; a point (h, z) lies on section s1 of wall 1 iff
    g   = A1 ^ (h - P1) = 0                                                  (on the plane)
    Psi = rho1^2 ((h - P1).A1)^2 + |A1|^4 (z^2 - rho1^2) = 0                  (on the ellipse in it)
 Substitute wall 2's point h = P2 + c2 A2, z^2 = rho2^2 (1 - c2^2):
    g(s1,s2,c2)   = A1^D + c2 W                                          degree (3+3, 3, 1)  = (6,3,1)
    Psi(s1,s2,c2) = rho1^2 (A1.(D + c2 A2))^2 + |A1|^4 (rho2^2 (1-c2^2) - rho1^2)   degree (18, 6, 2)
 Walls meet on span pair <=> g = Psi = 0 has a root in [span1] x [span2] x [-1,1] (Lean lemma, in progress).
 Psi < 0 means wall 2's point is INSIDE wall 1's section ellipse in that plane (the 13% margin shows up here).
 Certificate: Bernstein boxes; a box is clear if g's coefficients have one strict sign or Psi's do.
 Volute x core, all 72 span pairs (face domains), f64, coefficients cleared only beyond 1e-12 x max|coef|:
    616 boxes in total, 0 unresolved (diagonal pairs ~55 boxes each, neighbours ~20, far pairs 1).
 Coefficients: g 7x4x2 = 56, Psi 19x7x3 = 399. Cost estimate: setup ~5e4 flops per pair, ~5e3 per box
 => ~1e7 instructions for the whole volute x core wall set, vs 72 G today.
 If a box has g changing sign and Psi changing sign at the depth floor, it holds a real crossing: trace the
 curve {g = 0, Psi = 0} (regular space curve in (s1,s2,c2)), or equivalently F = 0 in (s1,s2).

### Prior art for lean-derive's R2, the moving-frame form (prior-art, 2026-10-31)
Closest: Seong, Kim, Kim, Elber & Martin, CAD 37(5) 2005, doi:10.1016/j.cad.2004.10.006.
They treat the sweep as a family of section curves (rational or implicit) and intersect the other surface with each.
That gives a zero set in parameter space. I verified the abstract only, so cite it as "following the approach of".
Ancestors: Heo, Kim & Elber 1999 (ruled); Heo et al. 2001 (ringed).
Keeping a section parameter as an unknown instead of eliminating it: Kim & Elber, "Problem reduction to parameter space",
Math. of Surfaces IX (2000); Krishnan & Manocha 1997 (lift then trace).
Nothing found on sweep vs sweep with nearly coincident section-plane families, so R2's fix appears new.

## End to end with the certificate, and the exact F(s, sigma) (novel-math, 2026-10-31)
Scratch patch (research/loft-ssi/novel/wt, not committed): Skinned::ring() returns c, X, Y when every rail
is an arc with one start and sweep and there are no guides (end slopes allowed). rings_disjoint() runs the
Bezier-hull SAT on the two FACE domains, clipped to the region, needing a gap of more than tol::LINEAR, and
ssi::surfaces_in returns Empty when it certifies. Release build:
  vf6.fc        10.7 s -> 2.09 s (Skinned x Skinned traces 268 -> 0; what is left is 125 Skinned x plane/cyl
                traces, 1.45 s). Valid. Same 46 faces. 122 edges / 77 verts instead of 144 / 99: the 22 extra
                vertices in today's result are imprints where carried-on curves cross face edges (checked:
                e.g. (110.845, 0, 28.956) sits on the volute's r=30 circle, 5 mm from the core).
                Volume 695713.2 vs 695715.9 (approx, tessellation follows the edge splits).
  vfx.fc        (novel/vfx.fc: vf6's two lofts, core raised 8 mm in z so the walls really cross; the top
                opens along two lips): today's build FAILS ("surface pair without a closed form": the tracer
                returns None on a pair the certificate now removes). With the certificate: valid, 2.36 s,
                26 real Skinned x Skinned crossings left at about 65 ms each (1.7 s).
Exact condition (py/pencil.py): F(s, sigma) = Res_mu of two quadratics on the null line of
[X1 Y1 -X2 -Y2] z = c2 - c1. Checked numerically: the column-j form equals q_j^4 F for every j.
lean-derive is deriving the j-free form and its degrees.
Caveat, measured on vfx: when the two lofts share section planes (the usual shell/core construction),
the (s, sigma) chart is the WRONG chart. The two lips project to nearly the same line (sigma = s +- 0.05)
and cross at every shared knot, where F vanishes to high order (|F| ~ 1e-75 against 1e25 elsewhere). The
two branches are only well separated in a chart on one surface, (s, a1). So for shared planes, use F to
certify and locate, and trace in (s, a1) or in the 4D parametric form S1(s, a1) = S2(sigma, a2), never in (s, sigma).

## Measured: certificate + closed-form marching in the face's own parameters (novel-math, 2026-10-31)
Scratch (novel/wt, forge-geom skinned.rs bottom + forge-bool ssi.rs, env RING_MEET=1):
 - ring x ring (both arc-section spans): Bezier-hull SAT certifies empty pairs. If a pair isn't empty,
   the same subdivision runs on to leaves of 1/64 of the region; leaf pairs are seeds for a minimum-norm Newton on
   S1(s,a) - S2(sigma,b) = 0 (4 unknowns, 3 equations, closed-form points and derivatives, no nearest-point
   solves), marched along n1 x n2 at the tracer's step. The output is an ordinary IntCurve.
 - ring x quadric (plane, cylinder, cone, sphere): Q(S) w^2 is EXACTLY a polynomial of bidegree (6,4) on a
   piece. Its 7x5 Bernstein coefficients come in closed form from the 12 homogeneous control points
   (products of Bernstein polynomials). When they all have one sign, the piece is certified empty. Otherwise
   subdivide to leaves, then Newton and march in the face's (s, a) chart on h = Q(S(s,a)) (2 unknowns, 1 equation).
Instructions for the whole build (perf stat), release:
                     today        + certificate    + certificate + ring marchers
   vf6.fc           101.7 G         17.4 G            5.1 G     (10.7 s -> 2.09 s -> 0.50 s)
   vfx.fc            60.9 G FAIL    21.2 G            4.9 G     (fails  -> 2.36 s -> 0.45 s)
   per pair: ring x ring with real crossings, about 1 ms against 65 ms traced; ring x quadric, 125 pairs in
   11 ms against 1.45 s.
All results valid (forgecad check). Curves agree with today's: total curve-edge length 3593.341 vs 3593.328
(vf6) and 3638.624 vs 3638.628 (vfx). Matching edges agree in length to 1e-3 mm. Edge counts are lower
(vf6 118 vs 144) because the spurious extension imprints are gone.
What's left in vf6's 5.1 G: Data::point/Spline::weights (Skinned::new grids, tessellation) and Skinned::solve
from downstream IntCurve evals. loft-speed's closed-form ring evaluator and a closed-form project cover those.

### R3. Recipe: `skinned::disjoint(a, b) -> bool` (lean-derive, 2026-10-31; reference impl derive/cert.py)
 Case test (else return false): every rail of both spans is an Arc, all with the same start (0 or pi) and
 sweep; no shifts; one unit e with every section frame's y = +-e, frame x . e = 0, (origin - o0) . e = 0
 and slopes . e = 0 for BOTH walls (same e, same plane). Fold the sign of y_k (and a start of pi) into r_k.
 1. Per wall, span j (h = k_{j+1} - k_j, m = the spline's second derivatives, Spline::m), datum d's weight in
    the cubic Bernstein basis of tau: [y_j, (2y_j+y_j1)/3 - h^2(2m_dj+m_dj1)/18, (y_j+2y_j1)/3 - h^2(m_dj+2m_dj1)/18, y_j1].
    P, A, rho coefficients = sum_d w_d (O_d, r_d x_d, r_d) (2D coordinates in the plane square to e; slopes add to P).
 2. Bernstein tensors over (tau1, tau2, u), c2 = 2u - 1 ([-1, 1]):  D_ij = P2_j - P1_i (partition of unity),
    E = D + c2 A2,  g = A1 ^ E  (7x4x2),  Psi = rho1^2 (A1.E)^2 + |A1|^4 (rho2^2 * [0,2,0]_u - rho1^2)  (19x7x3).
    Products: (p q)_k = sum_{a+b=k} C(m,a) C(n,b) / C(m+n,k) p_a q_b per axis; degree elevation where sums
    meet. Carry a magnitude tensor beside each (same formulas on |.|).
 3. Clear a box when g's or Psi's coefficients all exceed K u mag_max in one sign (K = 256, u = 2^-53).
    Else halve (de Casteljau, axis by depth mod 3, or the widest) down to a depth cap (36); hitting it -> false.
 4. Face pairs whose 3D boxes are apart need none of this (52 of the 72 volute pairs clear in 1 box anyway).
 Volute x core: 616 boxes, all 72 pairs certified, f64. Cost ~3e4 flops setup per pair + ~1e4 per box
 (both tensors and their magnitudes) => ~1e7 instructions for the whole wall set (vs 1.4 G for the SkxSk
 pairs after loft-speed's prune, 72 G before).
 Where walls DO cross (volute class): trace F(s1,s2) = rho1^2(W^2-a2^2) - rho2^2(W^2-a1^2) = 0 in the span pair,
 evaluated in factored form from the cubics (~150 flops with gradient), lift with cos phi1 = a2/W, cos phi2 = a1/W,
 sin phi2 = sin phi1 rho1/rho2. Starts: roots of F on the four box edges (degree-12 univariate Bernstein) and turning
 points (F = F_s2 = 0); near s1 ~ s2 with nearly coincident planes, switch to the (s1, s2, c2) system {g = 0, Psi = 0}.

### R4. General arcs (no common axis), with novel-math (lean-derive, 2026-10-31; derive/general_*.py)
 n_i = X_i x Y_i, l = n1 x n2, w = |l|^2, W0 = (n1.c1)(n2 x l) + (n2.c2)(l x n1) (= w * a point of L = plane1 cap plane2).
 q_i(t) = |V_i x X_i|^2 + |V_i x Y_i|^2 - w^2 |n_i|^2, V_i = W0 + t l - w c_i: q_i(t) = 0 <=> (W0 + t l)/w on ellipse i.
 Res_t(q1, q2) = |n1|^4 |n2|^4 |l|^8 F, and novel-math's pencil Res_j = q_j^4 F for every column j: the same F,
 degree exactly 24 in each loft's s for cubic data (bidegree (24,24), tight). Checked with exact rationals at 30
 random points (Schwartz-Zippel), not proved in Lean. Same near-diagonal ill-conditioning applies to nested walls.

## Production patch handed to loft-speed (novel-math, 2026-10-31)
research/loft-ssi/novel/loft-speed.patch, against loft-speed 5a0b79f: crates/forge-geom/src/loftmeet.rs
(exact rational Bezier hulls of span pieces; SAT for span x span; Bernstein sign test of Q(S) w^2 for span x
quadric; seeds from the leaves; minimum-norm Newton and marching in the surfaces' own parameters), called in
ssi::surfaces_in before the tracer, which remains the fallback. Instructions: vf6 3.65 -> 1.88 G, vfx 5.07 ->
2.04 G, bench-parts/volute.fc 10.03 -> 5.79 G; the other bench parts are unchanged; part tests give the same
pass/fail set. R17 now fails on 5a0b79f with or without it; its earlier pass came from stray vertices
0.07-1.2 mm off the loft (see novel/README.md).
Exact formula status: lean-derive proved F (j-free, symmetric form via L = plane1 cap plane2), bidegree
(24,24) tight, closed-form lift, Lean proof for the volute class (bidegree (12,12)). It is NOT the tracing
chart for lofts that share section planes; it is fine for certifying a pair and counting its branches.

### R5. Proof status (lean-derive, 2026-10-31): derive/proofs, `lake build` clean, no sorry
 Section.lean: on_section_iff (point on a section ellipse <=> g = 0 and Psi = 0), sections_meet_iff (R2's
   reduction to (s1, s2, c2)), pair_iff (R1: F = 0 and a2^2 <= W^2 <=> meet, with the lift).
   The certificate only needs the forward directions, which hold with no nondegeneracy hypotheses.
 Degree.lean: degrees_s1 (g <= 6, Psi <= 18, F <= 12 in s1), degrees_s2 (g <= 3, Psi <= 6, F <= 12 in s2).
 Bernstein.lean: bb_sum, bb_nonneg, bern_ge/bern_le (1D enclosure), bern3_pos (3D: all coefficients >= m > 0
   => positive on the box), rounded_pos, bb_half (de Casteljau: B^n_k(t/2) = sum_j B^n_j(t) C(j,k) 2^-j).
 Not formal: that K = 256 ulps x magnitude bounds the f64 coefficient error (standard gamma_n argument, not
   written out); the right half of de Casteljau (mirror of bb_half); R4's general identities (Schwartz-Zippel).
 Kernel cross-check (loft-speed): six points of volute/core walls from my model vs Skinned::deriv agree to the
   6-decimal input precision (2.4e-7).

## 2026-10-31 loft-speed: landed on branch loft-speed (rebased on main 6b402b9)
volute.fc (bench part, resolved), instructions:u seq / nary1:
 real-parts 116.31G / 119.88G
 face-domain patch boxes in trace      25.50G / 28.59G
 per-span cubic form (Data::poly)      15.32G / 17.16G
 cubic interval boxes                  11.05G / 11.39G
 packed seed scan (bit-identical)      10.22G
 warm draw_loop projections             9.64G / 10.12G
 ruled warm draw_loop (crank -0.5%)     9.64G
 seed-grid cell fix (no false 'sharp')  7.75G / 8.07G
 novel-math loftmeet (hull SAT, Bernstein sign, parametric march)  4.34G / 4.66G
 check: fine loops only on a miss (R13/R17/R32 now hold)          4.34G
 IntCurve evals start from chain uv (value_near)                  3.94G / 4.09G
 loftmeet Newton weighted by |S_u|,|S_v|                          3.12G / 3.24G (~370 ms seq)
lean-derive's R2 certificate not built: the SkxSk certification left is ~0.25G including quadric
sign tests; the remaining cost is diffuse (draw_loop projections 22%, IntCurve evals 15%, selfx 15%).
