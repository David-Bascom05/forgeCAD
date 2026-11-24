# Toolpath quality: surface finish

Research only; nothing in the kernel uses it yet. Goal (David, 2026-11-24): CAM toolpaths "Fusion level or
better: adaptive toolpaths, feeds and speeds, surface finish". This note covers **surface finish**:
the geometry of scallops, the finishing strategies, how CL points are computed, path smoothness,
what the controller does with the path, the 5-axis outlook, metrics, and what forgeCAD can prove.
It extends research/cam/literature.md §4, §5, §6 and §9 (not repeated here) and the 3D strategy
table in research/cam/fusion-workflow-2d-3d.md §4. It uses the G1–G6 obligations of
research/cam/design.md §5.2. Sources were looked up on 2026-11-24.

Marks (the same as literature.md, plus one):
- **[R]**: read this session (the page, the PDF, or the passage cited).
- **[S]**: search-engine summary or bibliographic record only.
- **[K]**: background knowledge, not looked up again. A lead: check it before relying on it.
- **[D]**: derived in this note from the stated definitions. Not a citation. It is a proof
  obligation for §8, and needs checking before code depends on it.

Notation: R is the tool radius (ball, or the outer radius of a bull nose), r is the corner radius of a
bull nose, R_f = R − r is the radius of its flat, s is the stepover (the distance between adjacent
passes), h is the scallop (cusp) height, f_z is the feed per tooth, κ is the surface's normal
curvature across the feed (positive convex), θ is the surface slope from horizontal, and ε is a tolerance.

What exists today (read with `ls` and the module docs this session) [R]: `forge-cam-geom` has exact
tool bodies of revolution (`tool.rs`: a (ρ, z) chain of lines and arcs, interval signed distance),
a certified move-vs-face checker (`check.rs`: faces are plane, cylinder, cone, sphere and torus with
rational parametrisations; moves are line, planar arc and helix; `certify` gives a `Verdict`), and
2D exact offsets and booleans. `forge-cam` has 2.5D ops; `forge-post` prints by exact integer
quantisation into four dialects (fanuc, haas, linuxcnc, grbl: 3 decimals, linuxcnc 4; arcs IJK;
haas starts with `G187 P2`, linuxcnc with `G64 P0.01`). No 3D finishing, scallop model or
arc/spline fitting exists yet.

---

## 1. Scallop (cusp) height

### 1.1 Ball end mill on a flat floor

- **Exact.** Two adjacent passes leave a ridge where two circles of radius R, whose centres are s
  apart, cross: **h = R − √(R² − s²/4)**, and the inverse is **s = 2√(2Rh − h²)** [K]; also given
  by calculator pages found in search [S] (https://calculator.academy/scallop-height-calculator/).
- **Rational bounds, no square root** [D]. With a = s/2, h = a²/(R + √(R² − a²)). Since
  R − a²/R ≤ √(R² − a²) ≤ R for a ≤ R:
  **s²/(8R) ≤ h ≤ s²R/(8R² − s²)**. The usual approximation h ≈ s²/(8R) is therefore always an
  *under*-estimate, and its relative error is below s²/(8R² − s²). So a stepover chosen from the
  approximation overshoots the target h. A planner that must guarantee h ≤ h_max should use the
  exact inverse or the upper bound. Example [D]: R = 3 mm, s = 0.3 mm gives h = 3.752 µm by the
  exact formula and 3.750 µm by the approximation.

### 1.2 Bull nose and flat end on a flat floor

- **Bull nose, axis normal to the floor** [D]. The section across the feed is a flat of half-width
  R_f with quarter-arcs of radius r. If s ≤ 2R_f, the flats overlap and **h = 0**. Otherwise
  **h = r − √(r² − (s/2 − R_f)²)**, valid for s/2 − R_f ≤ r. This is why a bull nose clears floors
  with a large stepover. The advantage holds only for a floor exactly normal to the axis: at any
  slope the contact moves onto the torus (§1.3).
- **Flat end** [D]: h = 0 on a normal floor for s ≤ 2R. The real floor finish then comes from
  feed marks, runout and tool dish angle (§1.6), not from the path geometry.

### 1.3 Inclined walls, 3-axis: why parallel finishing fails on steep walls

Take a fixed vertical axis and a plane at slope θ. Raster passes run along the contour (horizontal),
with horizontal stepover s_xy across the slope.

- **Distance along the surface between passes** [D]: s_xy / cos θ for raster passes, and
  d_z / sin θ for waterline passes with stepdown d_z.
- **Ball** [D]. The section across the feed is still a circle of radius R, so
  **h(θ) = R − √(R² − (s_xy / (2 cos θ))²)**. This grows without limit as θ → 90°: the passes stop
  meeting at all once s_xy/cos θ > 2R. Waterline is the mirror case: h grows as θ → 0. The two
  give equal surface spacing where tan θ = d_z / s_xy. That is 45° when d_z = s_xy. Fusion's
  Parallel "optimisation" adds perpendicular passes to areas steeper than 45° [R]
  (https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/3D-STEEP-AND-SHALLOW-PARALLEL-PASSES-OPTIMIZATION.htm).
  Its Parallel "Machine Steep Areas" adds passes where the surface is steep across the machining
  direction, with a "Minimum Steep Stepover" [R]
  (https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/GUID17CC1F90-A2A5-4CC3-80EB-B7972E327E0F.htm).
- **Bull nose on a slope, feed along the contour** [D]. The section plane across the feed contains
  the tool axis, so the section is exactly the tool profile. Once θ > 0 both contacts are on the
  corner arc, and h = r − √(r² − (s_xy/(2 cos θ))²): it behaves like a *ball of radius r*, not R.
  For a feed direction that is not horizontal, the section is not through the axis, and the
  effective radius is the curvature radius of the torus silhouette in that plane. That is the
  "effective cutter radius" idea of Lee & Ji 1997 and Lo 1999 (cited by Zou 2020, below) [R
  citation only]. Lee & Ji: *IJPR* 35(1):225–252; Lo: *CAD* 31(9):557–566.
- **Flat end on a slope** [D]. Each pass leaves a horizontal tread s_xy wide and a riser
  s_xy·tan θ high. The deviation normal to the plane is **h = s_xy·sin θ**. It is linear in s, not
  quadratic, which is why flat ends finish only flats and walls.

### 1.4 Curved surfaces: curvature-corrected stepover

- **Second-order formula** [R]: Zou, Q. (2020). "Robust and efficient tool path generation for
  poor-quality triangular mesh surface machining." arXiv:2004.09136, §3, eq. 1. It cites Lin & Koren
  1996 ([17] there):
  **h = (R_s + r)/(8 R_s r) · ‖p₂ − p₁‖² + O(‖p₂ − p₁‖³)**, with R_s the surface's radius of
  curvature at the contact point across the feed (positive convex, negative concave), r the cutter
  or effective cutter radius, and ‖p₂ − p₁‖ the distance between the contact points. Its eq. 2
  gives κ_t = v_tᵀ T v_t, where T is the curvature tensor κ₁d₁d₁ᵀ + κ₂d₂d₂ᵀ. Equivalently,
  h ≈ (κ_t + 1/r)·s²/8. **Convex surfaces need a smaller stepover, concave ones allow a larger
  one.** If κ_t → −1/r (a concave radius equal to the tool's), the cutter fits the surface. If
  κ_t < −1/r, the tool cannot reach (gouge or leftover, §3.4).
- **Exact circle-on-circle** [D], for checking the approximation and as an L2 reference. Take a
  convex section circle of radius ρ about O, and tool centres at distance ρ + R from O, separated by
  angle φ (the contact arc is ρφ). The ridge is at distance d = (ρ+R)cos(φ/2) −
  √(R² − (ρ+R)²sin²(φ/2)) from O, and h = d − ρ. Concave (tool inside, centres at ρ − R):
  d = (ρ−R)cos(φ/2) + √(R² − (ρ−R)²sin²(φ/2)), and h = ρ − d. With the tan-half-angle
  parametrisation forgeCAD already uses for arcs, these are in ℚ(√·).
- **Iso-scallop literature**, a lineage read from Zou 2020's reference list [R citation]:
  Suresh & Yang 1994 *J. Eng. Ind.* 116(2):253–259 (the first constant-scallop method, in the
  parameter domain); Lin & Koren 1996 *J. Eng. Ind.* 118:20–28 (efficiency); Yoon 2005 *IJPR*
  43(23):4989–4998; Sarma & Dutta 1997 "The geometry and generation of NC tool paths" *J. Mech.
  Des.* 119(2):253–258 (3D, accuracy); Feng & Li 2002 *CAD* 34(9):647–654,
  doi:10.1016/S0010-4485(01)00136-1 (3D; DOI from the record [S]); Kim 2007 "Constant cusp height
  tool paths as geodesic parallels on an abstract Riemannian manifold" *CAD* 39(6):477–489. Tournier &
  Duc also have a 3-axis iso-scallop paper (HAL hal-01076666) [S]; the PDF was behind a bot wall
  (Anubis) and was not read.

### 1.5 Scallop along the feed: tooth marks

- Each tooth removes a crescent. Along the feed, the ridges are spaced f_z apart, and their height
  is **h_f ≈ f_z² / (8 R_e)**, with R_e the radius of the cutting edge's trace in the feed plane
  [K]. It is the same circle-chord geometry as §1.1. Example [D]: f_z = 0.1 mm and R_e = 3 mm give
  0.42 µm, against 3.8 µm from s = 0.3 mm. In 3-axis ball finishing the stepover usually
  dominates. A search summary says cusps from the feed can exceed stepover scallops as f_z rises with
  faster machines [S] (Springer s00170-023-11001-3 and others in that search).
- **Ball centre on a flat floor**: with the axis vertical, contact at the ball's tip has zero
  cutting speed (rubbing, poor finish) [K]. This is the classic reason to tilt the tool (lead angle)
  or to keep shallow areas off the tip (§6).

### 1.6 Ra from cusp height, and real roughness

- **Parabolic profile** [D]. With a periodic profile y = h·u², u ∈ [−1, 1] (the small-s limit of a
  circular cusp), the mean line is h/3 and
  **Ra = h·∫₀¹|u² − 1/3| du = 4h/(9√3) ≈ 0.257 h**, with Rt = h. So "Ra ≈ h/4" is the parabolic
  result. The turning analogue Ra = f²/(32 r_ε), Rt = f²/(8 r_ε) (ratio 1/4) appears in a patent and
  in calculators [S] (US 6733365, https://vibromera.eu/calculators/turning-surface-finish). The
  patent excerpt says measured Ra from a sharp tool comes in under twice the theoretical value [S].
  A full cusp topography (stepover cusps × feed cusps) gives an areal Sa (ISO 25178) rather than
  a profile Ra (ISO 4287) [K].
- **What the geometry leaves out.** Wang, Zhao, Tan, Wan (2024). "Modelling and Optimization of
  Machined Surface Topography in Ball-End Milling Process." *Materials* 17(7):1533.
  doi:10.3390/ma17071533 [R]. Their swept-edge model includes runout (eccentricity e, phase υ₀)
  and lead/tilt angles. Predicted Sa against measured Sa differs by 0.85%, −2.17% and 13.48% on
  three trials, at Sa ≈ 0.8–1.9 µm. Other effects in the same search (deflection, minimum chip
  thickness, ploughing) [S]. **Consequence for forgeCAD:** a geometric scallop is a *proven* number.
  Ra and Sa predictions are models with ±10–100% error, so they are warnings from sourced data,
  as design.md §5.2 already separates.

---

## 2. Finishing strategies: state of the art

### 2.1 The strategies

- **Parallel / raster (iso-planar).** Planes cut the surface, or the XY rows are dropped onto it.
  Simple and robust. Spacing is uniform in XY, not on the surface (§1.3), and there is redundant
  overlap on shallow areas. Zou 2020 §2 traces it to Huang & Oliver 1994 *IJAMT* 9(5):281–290, and
  on meshes to Yang & Feng 2008 [R citation].
- **Iso-parametric / Flowline.** Fix u, step v. Spacing follows the parametrisation, so it bunches
  where the map is distorted. Zou says its "most serious limitation is the machining inefficiency
  caused by redundant machining" [R]. Fusion's Flow follows U or V of the faces [R via
  fusion-workflow-2d-3d.md §4]. forgeCAD owns exact NURBS/UV, so the parametrisation is exact,
  but the spacing must still be set from §1.4, not from Δv.
- **Waterline / Z-level / Contour.** Slices of the CL surface at constant z (literature.md §4).
  Best on steep walls, worst on shallow ones (§1.3). OpenCAMLib builds a waterline from two batch
  push-cutter runs along "fibers" in x and y, then weaves them into loops; `AdaptiveWaterline`
  adds a minimum sampling and a "cosine limit" for a flat() predicate [R]
  (https://opencamlib.readthedocs.io/en/stable/operations.html).
- **Steep and shallow.** Contour above a threshold slope, parallel or scallop below it.
  Fusion: the threshold is measured from horizontal, 0–90°. The overlap distance is how far each
  side extends over the other ("e.g. twice the stepover"). Options: Smooth Offsets (replaces sharp
  turns with curves, to avoid dwell marks), Remove Cusps at Junctions (an extra pass through the
  apexes), and cusp height driving stepover and stepdown [R]
  (https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/GUID-A6933B73-D98F-43AD-BCFD-A450D91C4E61.htm).
  An Autodesk blog suggests 30–45° typical thresholds [S]
  (https://www.autodesk.com/products/fusion-360/blog/why-steep-and-shallow/). That blog returned
  403 to direct fetch.
- **Iso-scallop / constant-scallop / "3D offset".** Each pass is placed so that h = h_max against
  the previous one (§1.4). It gives the shortest path with no redundant overlap, but it needs a
  first path, accumulates offset singularities (loops and self-intersections), and its passes can
  have sharp turns [R: Zou 2020 §2; Zou, Wang, Feng 2020 below]. Fusion's "Scallop" is a
  constant-*stepover* 3D offset from the boundary or walls [R via fusion-workflow-2d-3d.md §4]. That
  is iso-scallop only where the curvature is constant.
- **Geodesic parallels / heat method on meshes.** On a surface of constant κ_t, constant-scallop
  passes are geodesic parallels. In general they are parallels in a metric with T + T_c (Zou 2020
  §4 derives this; Kim 2007 frames it as an abstract Riemannian manifold) [R citation]. Zou 2020
  computes the distance by the heat method, with two sparse linear solves: Crane, Weischedel,
  Wardetzky (2017) "The heat method for distance computation," *CACM* 60(11):90–99 [R citation].
  The older route is fast marching: Kimmel & Sethian 1998 "Computing geodesic paths on manifolds,"
  *PNAS* [S] (https://pmc.ncbi.nlm.nih.gov/articles/PMC21092). A survey notes that FMM distances
  only approximate mesh geodesics [S] (arXiv:0904.2550). Fusion's Geodesic finishing lays passes by
  "true surface distance" [R via fusion-workflow-2d-3d.md §4].
- **Length-optimal / field-based.** Zou, Wang, Feng (2020). "Length-optimal tool path planning for
  freeform surfaces with preferred feed directions." arXiv:2009.02660 [R abstract]. The paths are
  level sets of one scalar field from a Poisson problem that trades constant scallop against a
  preferred feed-direction field. It cites Kumazawa et al. for the result that iso-scallop and
  maximum-strip-width feed directions are generally incompatible [R]. The implicit representation
  avoids self-intersections by construction.
- **Spiral (from centre, morphed, boundary-conforming).** Spirals remove the step-over moves, and so
  the marks they leave. Fusion has Spiral, Radial and Morphed Spiral [R via fusion doc §4]. Recent
  work: Shen, Xu, Qi, Zhang, Yan, Ding (2025, rev. 2026), "Topology-Preserving Scalar Field
  Optimization for Boundary-Conforming Spiral Toolpaths on Multiply Connected Freeform Surfaces,"
  arXiv:2512.22502 [R abstract]. Against conformal slit mapping it reports +14.24% machining
  efficiency, +5.70% scallop uniformity and over 10% lower impact-induced vibration. Earlier spirals
  on meshes are refs [28]–[31] in Zou 2020 [R citation].
- **Radial.** Spokes from a centre. Spacing grows with radius, so the scallop grows outward unless
  spokes are added (Fusion's "min stepover") [R via fusion doc].
- **Morph between curves / Blend.** Passes interpolated between two drive curves. The spacing is a
  blend, so the scallop must be checked, not assumed [R via fusion doc] [D].
- **Pencil.** Runs along concave creases where the tool touches two surfaces at once (bitangency).
  Fusion: bitangency angle, and overthickness (an inflated tool to catch larger fillets) [R via
  fusion doc]. Ren, Yau, Lee (2004), "Clean-up tool path generation by contraction tool method for
  machining complex polyhedral models," *Computers in Industry* 54:17–33,
  doi:10.1016/j.compind.2003.09.003 [S]. The same group has a rolling-ball method for finding
  pencil regions [S] (https://core.ac.uk/download/357329768.pdf).
- **Rest / corner finishing with a reference tool.** Machine only where a larger reference tool
  could not reach. Fusion Corner splits the result into steep and shallow by a threshold [R via
  fusion doc]. Exact version [D]: the region is (CL surface of the reference tool − CL surface of the
  current tool) > δ_rest. Both are drop-cutter max functions, so the region is a sign set of a
  difference of certified functions.
- **Projection.** A 2D pattern is dropped onto the part along z: Parallel, Spiral and Radial are
  all this, through drop-cutter (§3.1).
- **Flat.** Detects areas normal to the tool and clears them with a flat or bull nose (h = 0 for
  s ≤ 2R_f, §1.2) [R via fusion doc].

### 2.2 Trade-offs

Ratings are this note's judgement [D] from §1 and the sources above. "Robustness" means it always
produces a valid path on messy input.

| Strategy | Finish uniformity | Path length | Smoothness | Robustness | Compute cost |
|---|---|---|---|---|---|
| Parallel | good on shallow, fails above ~45° | long (overlap) | straight rows; stepover marks at row ends | very high | low (drop-cutter on a grid) |
| Waterline | good on steep, fails on shallow | medium | closed loops; one step-down mark per loop | high | medium (2D offsets per z) |
| Steep and shallow | good both, seam at the threshold | medium | needs overlap and cusp clean-up | high | medium+ |
| Iso-parametric / flowline | depends on the parametrisation | long where bunched | smooth (follows u, v) | per-face; multi-face needs alignment | low |
| Iso-scallop (offset) | best (h = h_max everywhere) | shortest of the offsets | sharp turns and loops at singularities | medium | high (offset curves on surfaces) |
| Geodesic / heat method | near-iso-scallop | short | smooth level sets | high on noisy meshes (Zou) | 2 sparse solves |
| Field/Poisson (length-optimal) | trades h against feed direction | optimal for its energy | smooth, no self-intersections | high | sparse solve + contouring |
| Spiral / morphed spiral | good | short, no step-overs | best (no direction reversal) | topology issues on holes | medium–high |
| Radial | varies with radius | medium | spokes; centre crowding | high | low |
| Pencil | n/a (clean-up) | short | follows creases; branches | medium (bitangency is sensitive) | high |
| Rest / corner | n/a (clean-up) | short | fragments | medium | high (two CL surfaces) |

**Fusion-level for forgeCAD 3D finishing**: Parallel with steep-area passes, Contour, Steep and
Shallow with overlap, Scallop (3D offset), Pencil, Corner with a reference tool, Flat, all with
cusp-height-driven stepover and smoothing. **Better than Fusion**: a *certified* max scallop per pass
(§8) and true iso-scallop from exact curvature (forgeCAD has exact surfaces, and κ comes from the
exact form, not from a mesh fit).

---

## 3. Cutter location (CL) computation

### 3.1 Drop-cutter

- OpenCAMLib: drop the cutter at a fixed (x, y) until it touches. Per triangle it tests the 3
  vertices, the facet and the 3 edges, and raises cl.z when any would be cut; `PathDropCutter` adds a
  z "floor" [R] (https://opencamlib.readthedocs.io/ and
  https://opencamlib.readthedocs.io/en/stable/operations.html). The cutter types and the
  bull-against-edge quartic are in literature.md §4 [S].
- Generalised APT cutter vs STL: Yau, Chuang, Lee 2004 *IJPR* 42(13) [S] (literature.md §4).
- **Exact against analytic faces** [D], extending literature.md §4. Ball vs plane: the centre
  lies on the plane offset by R (linear). Ball vs sphere and cylinder: offset sphere and cylinder.
  Ball vs torus: an offset torus (major radius kept, minor radius r ± R). Ball vs cone: the offset is
  a cone shifted along the axis, plus a sphere at the apex. Flat and bull against curved faces need
  the contact point where the face normal meets the tool profile. That is a root of a polynomial
  system, to isolate with forgeCAD's existing interval Newton. Since drop-cutter is a max over
  contacts, a certified upper bound on each contact's z gives a gouge-safe z_CL (literature.md §4).
- **Between samples.** The CL path is the polyline through the sampled z_CL values. Between two
  samples the true CL surface can rise above the chord (a convex bump) and gouge. Two remedies:
  (a) adaptive sampling until the chord deviation is ≤ ε_path (OpenCAMLib's "adaptive" classes
  exist [R, name only]); (b) the certified swept check of design.md §5.2 G1 on every segment,
  which forge-cam-geom `certify` already does for line moves against plane, cylinder, cone, sphere
  and torus faces [R].

### 3.2 Push-cutter and waterline

Push-cutter moves the cutter along a fiber at constant z and returns the interval where it
intersects each triangle [R] (OpenCAMLib docs above). Waterline = weave of x- and y-fibers into
loops [R]. For exact B-rep [D]: waterline at z is the 2D offset of the part's slice at z by the
tool's radius function at the contact height. For a ball, each slice of a face offset by R is a
curve of the offset surface. forgeCAD's 2D exact offset (`forge-cam-geom/offset.rs`) covers the
line/arc slices of planar, cylindrical and conical faces. Spheres, tori and NURBS give conics and
higher curves, which need certified approximation within ε_path.

### 3.3 Offset-surface methods and accuracy budget

- **Offset surface / CL surface.** For a fixed axis, the CL surface is the upper envelope of the part
  ⊕ inverted tool (literature.md §4 [S]). Fusion's "machining tolerance is the sum of the tolerances
  used for toolpath generation and geometry triangulation. Any additional filtering tolerances must
  be added to this tolerance to get the total tolerance" [R] (Fusion Parallel reference, URL in
  §1.3). Very tight tolerances cause "longer toolpath calculation times, large G-code files, and very
  short line moves", and short moves at high feed cause "data starving" [R] (same page).
- PowerMill: the Mesh Factor (0–1) sets the triangulation relative to the machining tolerance [R]
  (https://help.autodesk.com/cloudhelp/2024/ENU/PWRM-ReferenceHelp/files/GUID-0A68D751-66AA-4189-91E5-6739FFFD05CB.htm).
- **forgeCAD's budget** [D]. With no triangulation (exact faces), the budget is
  ε_total = ε_path (CL chord or fitting) + ε_round (post quantisation, at most ½·10^-decimals per
  axis, so √3/2·10^-d in 3D) + ε_ctrl (the controller's smoothing, §5). Only the first two are in
  forgeCAD's program; ε_ctrl is a stated assumption. G6 already checks the rounded path. At 3
  decimals the rounding error per axis is up to 0.5 µm, against 1–4 µm finishing chord errors
  (Heidenhain, §5). That is a real fraction of the budget, so **finishing should post 4 decimals**,
  as Heidenhain recommends [R].

### 3.4 Gouge avoidance

Local gouge: concave curvature tighter than the tool (κ_t < −1/R_e, §1.4). Global gouge: another face
under the tool. Drop-cutter's max handles both for a fixed axis. In 5-axis the local condition
involves the tilted tool's curvature (§6). forgeCAD's rule stays G1, on every move, certified.
Literature for checking is in literature.md §6.

---

## 4. Path smoothness for good finish

### 4.1 Why linear G1 paths mark the part

- Siemens: a CAM program of many short G1 moves is a polyhedron, whose facets "can be visibly
  mapped on the surface". At block transitions, linear interpolation gives step changes in axis
  velocity, which excite resonance and show as a bevelled pattern or vibration marks [R]. Siemens
  (05/2009), *SINUMERIK 5-axis machining* manual, §1.4 and §2,
  https://cache.industry.siemens.com/dl/files/454/37335454/att_110322/v1/SIN_WF5_0509_en.pdf.
- FANUC: Smooth Tolerance+ Control targets unevenness from many short segments. A 5 µm CAM tolerance
  gives smaller programs but visible facets; 1 µm gives good surfaces without longer machining time
  [S] (the FANUC fine-surface flyer; its URL returned 410 when fetched).
  (https://www.fanuc.eu/~/media/files/pdf/products/cnc/flyers/mfl-03005-fa%20fine%20surface%20technology/fine-surface-technology-flyer-en.pdf)

### 4.2 Corner smoothing (local blending)

- **3-axis.** Zhao, Zhu, Ding (2013). "A real-time look-ahead interpolation methodology with
  curvature-continuous B-spline transition scheme for CNC machining of short line segments." *IJMTM*
  65:88–98. doi:10.1016/j.ijmachtools.2012.10.005 [S]. It blends adjacent lines with a 5-control-point
  B-spline and is G2 continuous, with analytic curvature extrema, a bound on the approximation
  error, and a jerk-limited look-ahead. Tajima & Sencer (2016) "Kinematic corner smoothing for high
  speed machine tools," *IJMTM* 108:27–43 [S].
- **5-axis.** Tulsyan & Altintas (2015). "Local toolpath smoothing for five-axis machine tools."
  *IJMTM* 96:15–26. doi:10.1016/j.ijmachtools.2015.04.014 [S]. It inserts quintic (tip) and septic
  (orientation) micro-splines and is C³ (the review wording). Beudaert, Lavernhe, Tournier (2013)
  "5-axis local corner rounding of linear tool path discontinuities," *IJMTM* 73:9–16 [S]. Bi et al.
  2015 dual-Bézier, *IJMTM* 91:96–108; Shi et al. 2015 dual PH curves, *IJMTM* 88:223–236; Yang & Yuen
  2017, *IJMTM* 123:22–35; Sun & Altintas 2021 G3, *CIRP JMST* 32:529–549 [S]. All these citations
  were read from the reference list of the survey: Ma, Shen, Jiang, Zou, Yuan (2023), "A survey of
  path planning and feedrate interpolation in CNC," arXiv:2303.01368 (*J. Graphics* 43(6), in
  Chinese) [R]. That survey separates local corner adjustment from global (C-space) orientation
  optimisation [R].
- **In CAM** (rather than in the controller). Fusion's Steep and Shallow "Smooth Offsets" rounds
  sharp turns so the tool does not dwell [R]. forgeCAD [D]: round an offset corner with an arc of
  radius ρ ≥ ρ_min *only where the rounded path stays inside the safe CL region*. For 2D offsets the
  exact offset already gives arcs at convex corners. For concave corners, a "min cutting radius"
  (Fusion has one in Contour/Adaptive [R via fusion doc]) leaves material, which must then be
  reported as leftover (G4).

### 4.3 Arc fitting

- **Fusion smoothing**: "replace those linear moves with arcs (G2/G3 moves) within a given
  tolerance". It fits in any plane that suits the strategy; the post linearises arcs a control
  cannot take; it is on the Passes tab and off by default [R]
  (https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/GUID5A951723-4797-4F5F-A7A3-45DCBCA2D26F.htm).
  The Parallel reference says it removes "excessive points" and fits arcs "to reduce code size
  without sacrificing accuracy" [R]. An Autodesk blog (403 on fetch) says the smoothing tolerance
  *adds* to the machining tolerance, and suggests 50–200% of it (100% a good start) for finishing
  [S] (https://www.autodesk.com/products/fusion-360/blog/best-settings-smoothing-tolerance/).
- **Biarcs and arc splines.** Meek & Walton (1995), "Approximating smooth planar curves by arc
  splines," *J. Comput. Appl. Math.* 59(2):221–231, doi:10.1016/0377-0427(94)00029-Z [S]. Yeung &
  Walton 1994 *CAD* 26(11) [K] (literature.md §9). Robust biarc construction: arXiv:1711.00935 [S].
- **forgeCAD fit** [D]. Fit arcs to a *verified* polyline only when the Hausdorff distance between
  the arc and the sub-path is ≤ ε_fit. Then **re-certify the arc move** (forge-cam-geom has `Move::Arc`
  in z = const). The total error is ε_path + ε_fit, as Fusion states. Arc output is only in G17/18/19
  planes, so 3D finishing passes that are not planar stay G1 unless a spline word exists (§4.4).
- **PowerMill point distribution** [R] (URL in §3.3): "Tolerance and keep arcs" (removes unneeded
  points), "Tolerance and replace arcs" (polylines for controls bad at arcs), "Redistribute"
  ("ensures a constant distance between points, only inserting extra points if they are necessary to
  keep tolerance"; may cost compute but reduce machine time), and "Fit arcs" (3-axis only). The
  Tolerance Factor (0–1) sets how many points are removed. There are also a maximum point
  separation distance and, with Redistribute, an angle limit (useful near gimbal lock).

### 4.4 NURBS / spline output

- LinuxCNC: G5.2/G5.3 NURBS blocks (weights P, order L, default 3), marked experimental and not fully
  tested; G5 cubic spline in XY only [R] (https://linuxcnc.org/docs/html/gcode/g-code.html).
- Siemens: `BSPLINE` (order ≤ 3, SD, PL knot intervals, PW weights) and `POLY` (order ≤ 5); `COMPCAD`
  compresses G1 into a B-spline in the tolerance band [R] (Siemens manual, §7.1 and §3.6.1).
- FANUC: NURBS interpolation (G06.2) and G05.1 modes [K] (literature.md §9); no FANUC manual was
  openly reachable this session.
- **Recommendation** [D]: G1 (+ arcs) is the portable floor and what G6 already proves. A spline
  word is a dialect feature for later. Its proof needs the control's spline semantics, which are
  public only for LinuxCNC.

### 4.5 Point distribution, and marks that are not scallops

- **Even spacing.** Cimatron's "Fine Surface Quality" distributes points equally, "resulting in
  smoother machining with better surface quality", with a Preferred Points Distance [R]
  (https://help.cimatron.com/en/2026/Parameters/Motion_Parameters/Tool_Trajectory/tt_toolpath_smoothness.htm).
  PowerMill's Redistribute does the same (above). A PowerMill forum thread on banding and waviness in
  moulds blames unstable point distribution from very small tolerances [S, anecdotal]
  (https://forums.autodesk.com/t5/powermill-forum/surface-finish-defects-on-mould-banding-waviness-powermill/td-p/14119127).
- **Mechanism** [D, hypothesis to test]. Chord-error-driven decimation puts points where the
  curvature is, so adjacent passes on a surface of slowly varying curvature get breakpoints at
  different places. The facets of neighbouring passes then do not line up, and the controller's
  velocity dips at each breakpoint (§5) fall in an irregular pattern across passes: banding.
  Points at a fixed arc length, or on a grid shared across passes (so pass k's i-th point lies on the
  same cross-curve as pass k+1's), make the facets and dips coherent. That is the
  "keep points on a grid" idea. No primary source was found that states it, so it is [D]: measure it
  in the §7 benchmark before relying on it.
- **Feed direction.** Fusion Parallel offers one way / both ways / other way, and up or down
  milling; 3D Adaptive is climb only [R via fusion doc]. Climb versus conventional changes
  deflection direction, so in zig-zag finishing alternate passes deflect differently and leave a
  visible pattern [K].
- **Lead-in/out and dwell marks.** Each stop or slowdown in contact leaves a mark (Fusion: "avoid
  dwell marks" [R], Steep and Shallow page). Spirals and stay-down linking minimise the count, so
  count them as a metric (§7).

---

## 5. The controller side

The controller does not run the program's polyline. It smooths within its own tolerance and slows
where the look-ahead sees curvature or too many short blocks. CAM output should be shaped for that.

| Control | Command | What it does | Source |
|---|---|---|---|
| LinuxCNC | `G64 P- Q-` | P: the path stays within P of the programmed endpoint, slowing down if needed; without P, best speed whatever the deviation. Q: the "naive CAM detector" merges near-collinear XYZ G1 at the same feed (within Q) into one line; Q omitted → P; Q0 off. In G17, an arc within P of a line becomes two lines. `G61` exact path, `G61.1` exact stop | [R] linuxcnc.org/docs/html/gcode/g-code.html |
| LinuxCNC planner | P and Q | in the newer planner, `G64 P0.005` = `G64 P0.005 Q0`; worst case ≈ P + Q with Q on | [S] forum threads (forum.linuxcnc.org/20-g-code/47953) |
| GRBL | `$11`, `$12` | junction deviation, default 0.010 mm: corner speed between segments (higher is faster, risks lost steps). Arc tolerance, default 0.002 mm: arcs are drawn as line segments no worse than this | [R] github.com/gnea/grbl/wiki/Grbl-v1.1-Configuration |
| Haas | `G187 Pn Ennnn` | P1 rough, P2 medium (factory default via Setting 191), P3 finish; E = max corner rounding (overrides Setting 85); cancelled by reset, M30, M02 | [S] haascnc.com G187 page (403 on fetch) |
| Siemens 840D | `CYCLE832(tol, mode)` | finishing / pre-finishing / roughing / deselection. "Chord tolerance should be taken from the CAM system or weighted with a factor of 1.0 to 1.5". Default axis tolerances 0.01 mm/0.08° finishing, 0.05/0.4 pre-finishing, 0.1/0.8 roughing. COMPCAD (G1 → B-spline, constant acceleration, free-form), COMPCURV (jerk-free, circumferential), G642 smoothing with axis tolerances (fixed with COMPCAD). "Tolerances that are lower than the one specified [in CAM] are not practical." Example `CYCLE832(0.005,112101)` | [R] Siemens 5-axis manual 05/2009 §3.6 (URL in §4.1) |
| Heidenhain TNC | `CYCL DEF 32 TOLERANCE` T, HSC-MODE, TA | T = path tolerance. Finishing: T ≥ 110–300% of the CAM chord error, which is "usually 1–4 µm"; T 5 µm (accuracy) to 20 µm (smoothness). Roughing: CAM chord 4–30 µm, T 0.05–0.3 mm. HSC-MODE 0 favours accuracy, 1 speed. TA: rotary axis tolerance, with the TCP error compensated within T. Output 4 decimals (vectors 7) | [R] HEIDENHAIN Klartext 61, "Cycle 32 TOLERANCE", https://www.heidenhain.us/addl-materials/enews-pro/pdf/KLARTEXT_61_Cycle_32.pdf |
| FANUC | `G05.1 Q1` (AICC), Smooth Tolerance+, Nano Smoothing | AI contour control: look-ahead and accel/decel before interpolation. Smooth Tolerance+: smooths short-segment paths. Nano smoothing: "interpolation for generating smooth curves on a nanometer scale" | [S] FANUC flyer and pages; [R abstract] Sudo (2007) "Advanced Control Technologies for 5-Axis Machining," *IJAT* 1(2):108–112, doi:10.20965/ijat.2007.p0108; block counts and the R1–R10 levels are third-party blogs [S] |

**How to shape CAM output for these controls** [D, from the sources above]:
1. **Split the tolerance explicitly.** CAM chord ε_path small (1–4 µm finishing, per Heidenhain);
   control tolerance 1.1–3× of it (Heidenhain), or 1.0–1.5× (Siemens). The post should *emit* the
   control's tolerance from the op's tolerance: CYCLE832(tol), Cycle 32 T, G64 P, G187 P3 E. Today the
   haas and linuxcnc dialects hard-code `G187 P2` and `G64 P0.01` [R]. For finishing these should come
   from the op. The reply should report ε_ctrl as an assumption outside the proof.
2. **Even chord lengths.** Compressors fit splines through point runs, and uneven spacing gives
   uneven fits. Use constant arc-length spacing ≤ L_max, with extra points only for tolerance (the
   PowerMill Redistribute rule).
3. **Don't over-sample.** Short blocks at high feed starve the control (Fusion [R]). The block
   rate is F / L. For a control processing N blocks/s, L ≥ F / N. N is machine data and not public
   for most controls, so make it a dialect field with a sourced default.
4. **Decimals ≥ 4 for finishing** (Heidenhain [R]; Siemens says ≥ 3 for 3-axis and 5 for 5-axis
   [R]). forge-post's dialects today print 3 (fanuc, haas, grbl) or 4 (linuxcnc) [R].
5. **Collinear runs as one block, or not at all.** LinuxCNC's Q detector merges them [R]. A CAM
   that emits exactly collinear points adds nothing but block rate.

---

## 6. 3-axis → 5-axis outlook

- **3+2 indexing.** Fixed tilts let a ball avoid tip-cutting and a bull reach steep walls. Fusion
  has 3+2 roughing with automatic orientation [R via fusion doc]. Proof cost: same as 3-axis in a
  rotated frame. The exact rotation is a rational point on the circle (literature.md §5).
- **Lead/tilt (sturz).** A flat end tilted by lead λ along the feed has an elliptical section
  across the feed, with radius of curvature **R_e = R / sin λ** at its lowest point [D]
  (semi-axes R and R·sin λ; curvature radius a²/b). A large R_e gives a large stepover for the same
  h (§1.1). Vickers & Quan 1989 [K]; Lee & Ji 1997 [R citation]. A bull's R_e follows the same
  algebra with the torus.
- **Barrel / circle-segment cutters, tangent-plane machining.** OPEN MIND (hyperMILL MAXX): conical
  barrel cutters, radii "1,000 mm and larger" (conventional barrels 50–100 mm); in their test a 3 mm
  ball and a 350 mm barrel gave nearly the same surface with 90% less time [R vendor claim]
  (https://openmind-tech.com/en/cam/hypermill-maxx-machining/finishing/conical-barrel-cutter).
  Trade press: 6–8 mm line spacing and 70% cycle-time cuts in shop use [S]. **Check by §1.1** [D]:
  at equal h, s ∝ √R, so 350 mm vs 3 mm gives √(350/3) ≈ 10.8× stepover. That is about a 91% cut in
  path length for equal feed, consistent with the claim. It holds only where the surface is flat or
  convex enough for the segment to fit (local gouge, §3.4). Academic: Bartoň, Bizzarri, Rist,
  Sliusarenko, Pottmann (2021) *ACM TOG* 40(4), doi:10.1145/3450626.3459837 [S]; the ACM page
  returned 403.
- **Swarf / flank milling.** Only developable ruled surfaces are exact (literature.md §5).
- **Tool-axis smoothness and singularities.** Linear interpolation of rotary axes (Siemens ORIAXES)
  makes inclined flank-milled walls non-flat; ORIVECT/ORIPLANE interpolates the orientation on a
  great circle [R] (Siemens manual §3.5). Singular-cone avoidance: Affouard et al. 2004
  (literature.md §5). Heidenhain TA smooths the rotary axes and compensates the TCP error within
  T [R]. PowerMill's point-separation angle adds points near gimbal lock [R].
- **Fusion level vs leader level** [D, from the sources]. Fusion's multi-axis finishing is in the
  paid extension (Multi-Axis Finishing, Swarf, Geodesic with lollipop tools) [R via fusion doc].
  Leader level (hyperMILL MAXX, PowerMill) adds barrel-cutter tangent-plane strategies, automatic
  tilting for collision avoidance, and orientation smoothing tied to the machine's kinematics. For
  forgeCAD the order would be: 3+2 first (cheap to prove), then lead/tilt with a certified G1 check
  in the tilted frame, then barrel cutters (the tool body is already a (ρ, z) chain of lines and
  arcs, which covers a circle segment).

---

## 7. Metrics: a finish-quality benchmark

Measure from exact geometry where possible, and from simulation otherwise. Every metric names its
evaluator.

1. **Scallop field.** h(p) = distance from the design surface (+δ) to the in-process stock along the
   normal, sampled densely on each finished face. Report max, mean, p95 and the share above h_target.
   Exact form [D]: per face, the stock is S₀ minus a union of swept tools; h(p) = max over t ≥ 0
   such that p + t·n is outside the swept union. This is a ray query against the certified checker,
   so h_max can be certified as an upper bound by interval B&B (the same machinery as G1).
2. **Uniformity.** The coefficient of variation of h over the face; iso-scallop should approach 0
   on its region. Shen et al. 2025 report scallop uniformity as a percentage [R abstract].
3. **Gouge.** min(h) ≥ −ε_in: G1 already proves it.
4. **Point spacing.** Per pass: the distribution of segment lengths (min, CV); the share below
   L_min = F/N (§5.3); cross-pass alignment = phase offset between pass k's and k+1's breakpoints
   along the cross-curve (§4.5).
5. **Path smoothness.** Count of tangent discontinuities above an angle threshold; curvature
   jumps between blocks (G1 vs G2 joins); count of direction reversals; for arcs, radius vs ρ_min.
6. **Marks.** Count of lead-ins/outs, retracts and step-overs *inside the finished region* (each
   is a potential mark), and the total length of feed slowdowns from a simple jerk-limited
   look-ahead model (Erkorkmaz & Altintas 2001, literature.md §9) with the dialect's limits.
7. **Predicted roughness.** Ra across the feed from h (§1.6, ≈ 0.257·h) and along the feed from f_z
   (§1.5). Label it a model, not a guarantee (§1.6).
8. **Efficiency.** Path length, estimated time (with the look-ahead model), and redundant-overlap
   area (area cut by two or more passes beyond what h_target needs).

**Benchmark set** [D]: a flat floor with a bull nose (h = 0 expected); a 45° inclined plane with
parallel vs contour vs steep-and-shallow (§1.3 closed form for checking); a sphere cap and a
spherical bowl (exact §1.4 circle formulas, convex and concave); a torus fillet (pencil and corner);
and a NURBS freeform (iso-scallop vs parallel). Each row has a closed-form h_max to check the
simulator against, as design.md §5.3 L6 does for the 2.5D generator.

---

## 8. What is provable in forgeCAD

These are statements of the G1–G6 style [D]. They are sketches, not Lean. Each is over ℝ, with the
tool a solid of revolution.

- **S1 (flat ball scallop).** For a ball of radius R at CL heights z = R over the plane z = 0, along
  two lines s ≤ 2R apart, every point of the plane between them is within h(s) = R − √(R² − s²/4)
  of the swept union, and h(s) ≤ s²R/(8R² − s²). This is elementary (§1.1); it is the base case.
- **S2 (scallop from a curvature bound).** Let a face F be C² with normal curvature κ_t ≤ κ̄ across
  the feed in a region U, and let adjacent CC curves be at surface distance ≤ s, with tool effective
  radius r. Then h ≤ (κ̄ + 1/r)·s²/8 + C·s³, with C given by bounds on the third derivatives over U.
  forgeCAD has exact surfaces, so it can *enclose* κ_t over a parameter box by intervals (its dual
  numbers in `dual.rs` give derivatives). The theorem then becomes a certified bound per box, and the
  planner chooses s from it. The O(s³) term must be made explicit; the exact circle formulas (§1.4)
  give the constant-curvature case without remainder.
- **S3 (scallop from the stock, direct).** The stronger alternative to S2: certify h_max directly
  from the swept tools (§7.1) with the B&B checker. Then no curvature theorem is needed, only
  soundness of the B&B, which is design.md's L3 item. S2 is the *planner's* tool; S3 is the *proof*.
- **S4 (chordal deviation of a line).** For a C² planar curve with curvature ≤ k̄ = 1/ρ on a piece
  of chord length L ≤ 2ρ, the distance from the chord to the curve is at most the circle's sagitta
  ρ − √(ρ² − L²/4) ≤ L²ρ/(8ρ² − L²). Note that k̄L²/8 is only the leading term and *under*-states it
  (§1.1). This sets ε_path for a polyline. The comparison with the circle is classical [K]; prove it
  once in Lean (space curves need the same with curvature and the chord's plane).
- **S5 (arc fit).** If an arc A replaces a sub-polyline P with Hausdorff d_H(A, P) ≤ ε_fit, then
  every point of A's swept tool is within ε_fit of P's. So G1 on P with margin ε_fit implies G1 on
  A; or simply re-certify A, since `Move::Arc` exists. Prefer the re-check: it needs no lemma.
- **S6 (rounding).** As G6: the quantised program is what is checked. For finishing, the reply states
  ε_round = √3/2·10^-d and that it is in the budget.
- **S7 (controller, stated assumption).** If the control keeps the path within ε_ctrl of the
  programmed path (the meaning of LinuxCNC's G64 P [R]; Heidenhain's T [R]; Siemens' tolerance [R]),
  then the machined surface is within h + ε_path + ε_round + ε_ctrl of the design surface in the
  outward direction, and the gouge margin is reduced by ε_ctrl. forgeCAD cannot prove what the
  controller does. It can make ε_ctrl a named hypothesis in the reply, and post the matching
  command (§5.1).
- **Not provable** (state as such): Ra and Sa (process physics, §1.6), chatter, deflection, and the
  banding mechanism of §4.5 (an empirical claim).

---

## 9. Suggested order for forgeCAD (from this note)

1. Scallop library: exact h for ball, bull and flat on a floor, a slope, and a circle section; the
   rational bounds; tests against closed forms (S1, S4).
2. Drop-cutter on exact faces (ball first), with adaptive sampling to ε_path and the G1 certify on
   every segment; Parallel with steep-area passes; Contour from the exact 2D offset.
3. The scallop metric from the stock (§7.1) as a certified max: this is the differentiator (S3).
4. Cusp-height-driven stepover: S2 per parameter box, then Steep and Shallow with overlap.
5. Point redistribution at constant arc length, and arc fitting with re-certification; post 4
   decimals for finishing; emit the control tolerance from the op (§5).
6. Iso-scallop / geodesic on exact surfaces; pencil and reference-tool rest; then 3+2.

## Sources not reached

- Autodesk blog "best settings smoothing tolerance" and "why steep and shallow": HTTP 403 (search
  summaries only, [S]).
- Haas G187 page: HTTP 403 ([S]). FANUC fine-surface flyer: HTTP 410 ([S]). MMS "ballnose
  alternative" article: 403. ACM TOG (Bartoň 2021): 403. Tournier & Duc HAL PDF: Anubis bot wall,
  not bypassed. PowerMill support article on Constant Z step marks: 403 ([S] from the search).
- The primary papers of Suresh & Yang, Lin & Koren, Feng & Li, Sarma & Dutta, Kim, Ren–Yau–Lee,
  Zhao–Zhu–Ding, Tulsyan & Altintas and Beudaert et al. were not read: only their citations, through
  Zou 2020 and Ma et al. 2023, or search records. Their methods as described above are from those
  secondary texts.
