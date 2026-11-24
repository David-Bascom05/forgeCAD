# Feeds, speeds, forces, chatter and feed scheduling: what "good" toolpaths need

Research note for forgeCAD's CAM (branch research/toolpath-quality, 2026-11-24). It extends
`research/cam/literature.md` §8 and `research/cam/libraries-machines-posts-feeds.md` §3. The goal
is toolpaths that match Fusion or do better: adaptive clearing whose feeds are right, cutting data,
and surface finish.

**Marks.** [R] means read this session, either the primary text itself or a fetched page's text.
Where a page was read only through a fetch summary, the entry says so. [S] means seen only as a
search snippet or index record. [K] means background knowledge not re-read this session: a lead to
check, not a fact. "derived" means the algebra is ours and each step is shown. Anything not read is
UNKNOWN. Values are restated, not copied. Every number in the worked example (§11) is cited or
derived.

---

## 0. What forgeCAD computes today (read in the repo)

- `kernel/forge-tooling/src/feeds.rs` [R] takes an ISO 513 group record and computes:
  - `d_eff`, which is 2√(ap(D−ap)) for a ball nose when ap < D/2 and D otherwise, with no tilt;
  - rpm = 1000·vc/(π·d_eff), clamped to the machine's rpm range;
  - the thinning factor k = 2√(r − r²) with r = ae/D when r < 0.5 (k = 1 otherwise), and
    fz = target/k, capped at `THIN_CAP = 3.0`× the target;
  - hex = fz·k, vf = fz·n·z, and power = ap·ae·vf·kc/6e7 kW.
- `kernel/forge-tooling/data/cutting.toml` [R] holds one record per group P M K N S H, with vc
  for carbide and HSS, fz at D = 1 and D = 20 mm (linear in between), kc1_1 and mc. The source is
  "forgecad generic mid-range, restated from public handbook ranges". The values are not cited per
  number, so the standing rule (cite every value) is **not yet met** for this file.
- The caller is `interface/forge-cam-lang/src/lib.rs` `feeds_for` [R]. It passes the part's group,
  the machine's rpm range, the tool's D, z, ball flag and carbide flag, and ap and ae.
- `kernel/forge-cam/src/ops.rs` [R] gives each op a `feed: (cut, plunge)` pair, so the feed is
  constant along an op. Nothing varies the feed with engagement, corners or ramps.
- `kernel/forge-machine/src/sim.rs` [R] times posted G-code two ways. `t_min` uses each joint's
  maximum feed. `t_stop` uses exact stops with a trapezoidal profile (joint `accel`). Its header
  says this is "an estimate, not a bound: a blending control is faster, a jerk-limited one slower".
- Lean: `verify/forge-cam/ForgeCam/Pocket.lean` [R] proves that an offset pocket covers the region
  for a stepover 0 < s ≤ r. It proves nothing about engagement or chip thickness yet.

**Findings in the code** (derived from the code and §1.3–§3):

1. **The power uses kc at hex, not at hm.** The docstring says "hm the mean chip thickness", but
   the code sets `hm = hex.max(1e-4)`. Because kc = kc1.1·h^(−mc) falls as h grows, and hex > hm,
   the code's kc is too low and the **power is underestimated** by the factor (hex/hm)^mc. With
   mc = 0.25 that is ×1.12 for a full slot and ×1.18 at ae = 0.1D (§11). That is the
   non-conservative direction.
2. There is no entering-angle term (sin κ), no corner-radius or bull-nose Deff, and no tilt.
3. ae and ap are a single nominal pair per op. The adaptive path's real engagement (higher in
   corners, §2.3) never reaches the feeds.
4. The N group's `vc_carbide = 400` m/min is well under Helical's 2100 SFM (640 m/min) for
   wrought aluminium (§8). Both numbers are "starting points", but they should be cited and their
   source shown.

---

## 1. Chip thinning

### 1.1 Radial chip thinning (peripheral milling)

The geometry is derived. The tooth path is a trochoid. To first order in fz/D, the uncut chip
thickness at immersion angle θ (θ = 0 where the tooth is normal to the feed and moving into the
cut) is h(θ) = fz·sin θ. This is the circular-path approximation usually credited to Martellotti
(1941) [K]. A cut of radial width ae covers the immersion arc of width

  φ = arccos(1 − 2ae/D)   (0 ≤ ae ≤ D; φ = π for a slot).   (derived: the chord at depth ae on a circle of radius D/2)

In down (climb) milling the tooth enters at θ_st = π − φ and leaves at θ_ex = π. In up
(conventional) milling it enters at θ_st = 0 and leaves at θ_ex = φ. Either way the arc has width
φ, and the largest chip on it is

  hex = fz·sin φ  when φ ≤ π/2 (ae ≤ D/2),  and hex = fz  when ae ≥ D/2,

because sin θ peaks at θ = π/2, which lies inside the arc once ae ≥ D/2. Substituting
cos φ = 1 − 2ae/D gives sin φ = √(1 − (1 − 2ae/D)²) = 2√((ae/D)(1 − ae/D)). The **radial chip
thinning factor** is RCTF = 1/sin φ for ae < D/2. To keep hex at a target chip h*, program
fz = h*·RCTF.

Sources that agree:
- Helical Machining Guidebook (2016), p. 22 [R]. Mirror:
  https://mae.ufl.edu/designlab/Advanced%20Manufacturing/Helical_Machining_Guidebook.pdf (this
  mirror was unreachable in an earlier session and returned the PDF this time). It gives
  IPT = CT·D / (2·√(D·RDOC − RDOC²)) for RDOC < 50% D. Multiplying the top and bottom by 1/D shows
  this is the same expression. Its stated threshold: "less than 50% of cutter diameter". It also
  warns that the adjustment must be reduced where engagement rises in corners.
- Sandvik Coromant, "Milling formulas and definitions" [R, through a fetch summary]:
  https://www.sandvik.coromant.com/en-gb/knowledge/machining-formulas-definitions/milling-formulas-definitions
  For side milling with ae < Dcap/2 it gives fz = hex·Dcap / (2·sin κr·√(Dcap·ae − ae²)), and for
  ae > Dcap/2 it gives fz = hex/sin κr. This is the same law with the entering angle included.
- Helical "Common Milling Calculations" sheet [R]:
  https://cdn.shopify.com/s/files/1/0612/0733/files/Common_Milling_Calculations.pdf. It gives
  FPTadj = CLPT·(D/2)/√(D·RDOC − RDOC²), the same result.
- CNCCookbook's chip-thinning page [R]: https://s3.amazonaws.com/s3.cnccookbook.com/CCChipThinning.htm.
  It gives no formula. Its worked example (1/2" 4FL, "5%" ae) is internally inconsistent: it gives
  ae = 0.015", which is 3% of D, and its feed ratio of ×5.9 does not match RCTF at either 3% or 5%.
  Do not use it as a check.
- Harvey "Combat Chip Thinning" (harveyperformance.com/in-the-loupe/combat-chip-thinning/) came
  back with an empty body this session: **unreachable**. libraries-machines-posts-feeds.md §3.2
  read it earlier: the threshold is in its text, and its formula is an image.

**Mean chip thickness** (derived; needed for kc in §3):

  hm = (1/φ)∫ fz sin θ dθ over the arc = fz·(cos θ_st − cos θ_ex)/φ.

For down milling with ae ≤ D/2 this is fz·(1 − cos φ)/φ = 2·fz·(ae/D)/φ. The same value holds for
up milling, by symmetry of the arc about π/2 after relabelling. For a slot it is 2fz/π.
Sandvik's page defines hm in words but its formula is not in the page's text (an image) [R], so
our form is unverified against theirs. The textbook form (Altintas 2012, ch. 2) is the same
integral [K].

**Monotonicity** (derived): s(x) = 2√(x(1 − x)) has s′(x) = (1 − 2x)/√(x(1 − x)) > 0 on (0, 1/2).
So hex is strictly increasing in ae for ae < D/2 and constant at fz beyond. This is what lets an
engagement **bound** give a chip **bound** (§10).

### 1.2 Ball nose: effective diameter at depth and with tilt

- With no tilt, the ball cuts at depth ap (ap < D/2) out to the diameter Deff = 2√(ap(D − ap)).
  The sources agree: Helical "Common Milling Calculations" [R] writes it as Deff = 2√(R² − (R − ADOC)²);
  Sandvik [R] writes Dcap = √(D3² − (D3 − 2ap)²), which is the same thing; and CNCCookbook's example
  [R], a 1/2" ball at 0.040" deep, gives Deff = 0.2713". We checked that last number:
  2√(0.04·0.46) = 0.27129. Speed is then set from Deff: n = 1000·vc/(π·Deff).
- Feed for a ball, from Sandvik [R]: when the cutter is centred, fz = D3·hex/Dcap. In side
  milling, fz = D3·hex/√(Dcap² − (Dcap − 2ae)²). This is the radial law applied at Dcap, with a
  thinning from the ball's curvature added on top.
- Tilt β (the tool axis leaning toward or away from the feed), derived. The contact band on the
  sphere runs from polar angle β to β + α, where α = arccos(1 − 2ap/D) is the polar angle of the
  depth line. The largest cutting radius is then (D/2)·sin(min(β + α, π/2)), so
  Deff = D·sin(β + α), capped at D. With β = 0 this reduces to the formula above. A tilt also takes
  the zero-speed tip out of the cut once β ≥ α. Mitsubishi's "inclined" formula, the cross-check,
  is still unread (libraries doc §3.2).

### 1.3 Lead-angle (axial) chip thinning: high-feed cutters, round inserts, bull nose

- Straight edge at entering angle κr: the chip measured normal to the edge is hex = fz·sin κr
  (×sin φ when ae < D/2). Sandvik's fz = hex/sin κr [R]. A 45° face mill therefore needs
  fz = 1.41·hex, and a high-feed cutter with κr ≈ 10–17° [K] needs 3.4–5.8·hex. Sandvik also gives
  the effective diameter Dcap = Dc + 2ap/tan κr [R].
- Round insert of diameter iC at ap < iC/2, from Sandvik [R]:
  fz = hex·iC·Dcap / (4·√(ap·iC − ap²)·√(Dcap·ae − ae²)). The first root is the axial thinning of
  the arc edge (the local κ at depth ap); the second is the radial term.
  Dcap = Dc + √(iC² − (iC − 2ap)²).
- Bull nose (corner radius rε, ap < rε), derived by the same construction as the round insert: the
  local entering angle at the bottom of the cut is κ = arccos(1 − ap/rε), the cut reaches out to
  Deff = D − 2rε + 2√(rε² − (rε − ap)²), and the axial factor is sin κ = √(1 − (1 − ap/rε)²). When
  ap ≥ rε, the straight flank takes over and κ = 90°.
- G-Wizard says it compensates for lead angle [R, CNCCookbook chip-thinning page]. Its method is
  not published.

---

## 2. Engagement models

### 2.1 Engagement angle and MRR

- φ = arccos(1 − 2ae/D), from §1.1. The mean number of teeth in the cut is z·φ/(2π). It decides
  whether the force is smooth (≥ 1 tooth engaged) or interrupted [K, Altintas 2012].
- Removal rate: Q = ae·ap·vf (mm³/min), given as Q = ap·ae·vf/1000 cm³/min by Sandvik [R] and as
  MRR = RDOC × ADOC × IPM by Helical [R]. For a path whose ae varies, the exact Q is the rate at
  which the swept volume minus the in-process stock changes. forgeCAD's stock model can compute it
  exactly (§10). GE's 1987 patent does this with a solid modeller (§6).

### 2.2 Up versus down milling

In down milling (climb) the chip starts at its thickest and runs to zero at exit. In up milling it
starts at zero and grows, which gives rubbing at entry [K]. Helical lists "climb mill in lieu of
conventional" among the ways to reduce deflection, p. 69 [R]. The engagement arc formula is the
same either way. Only θ_st and θ_ex change, and so does the direction of the force (§3.2).

### 2.3 Arc of contact on curved paths and in corners

Derived. The tool centre runs on a circle of radius Rp about O, and the boundary of the uncut
material is a circle of radius Rb about the same centre.
- **Concave (inside) arc**, with material outside Rb, as in a pocket corner. The tool reaches
  Rp + r > Rb, and the engaged half-arc runs from the outward radial to the point where the tool's
  circle crosses Rb:
  cos φ = (Rb² − Rp² − r²)/(2·Rp·r).
- **Convex (outside) arc**, with material inside Rb, as on an island:
  cos φ = (Rp² + r² − Rb²)/(2·Rp·r).
- Check: in the straight-line limit (Rp → ∞, Rb = Rp ± (r − ae)), both give cos φ → 1 − 2ae/D.
- Example: r = 6.35 mm, ae = 1.27 mm (0.1D), finishing into a 10 mm wall radius, so Rp = 3.65 and
  Rb = 8.73. Then φ = 60.9° against 36.9° on the straight, and hex rises from 0.600·fz to
  0.873·fz, which is +46% (computed). **An RCTF-boosted feed on the straight becomes a 46% chip
  overload in the corner.** Helical warns of exactly this, p. 22 [R], and Fusion's adaptive
  "Minimum Cutting Radius" exists for the same reason [S, help 3D-ADAPTIVE pages].
- Feed at the contact point compared with the centre (derived): on an arc the centre moves at
  angular rate ω = F_c/R_path. On an *outside* (convex part) arc of part radius ro, the centre path
  is ro + R and the edge at ro moves slower, so holding the edge feed at IPM needs
  F_c = IPM·(ro + R)/ro. On an *inside* arc ri the centre path is ri − R and the edge moves faster,
  so F_c = IPM·(ri − R)/ri. Helical's sheet [R] prints Fo = IPM·(ro + R/2)/ro and
  Fi = IPM·(ri − R/2)/ri, with "R = tool radius" in its key. That is half the geometric
  correction. It may be a deliberate mid-way compromise or a key error: UNKNOWN. It needs a check
  against p. 102–103 of the Helical catalogue, which was not read.

---

## 3. Force and power

### 3.1 Kienzle

The specific cutting force is kc = kc1.1·h^(−mc), and the force is Fc = kc·b·h = kc1.1·b·h^(1−mc).
Kienzle, O. (1952), "Die Bestimmung von Kräften und Leistungen an spanenden Werkzeugen und
Werkzeugmaschinen", *VDI-Z* 94(11):299–305. The citation is [R], from the reference list of
Klippel et al. below; the paper itself is [K].
- Sandvik defines kc1 as the force for a 1 mm × 1 mm chip at 0° rake, and says that more positive
  rake lowers it [R]: https://www.sandvik.coromant.com/en-gb/knowledge/materials/specific-cutting-force.
  The page gives **no** numeric kc1 or mc in its text; its curves are images. The per-group kc1
  ranges in libraries-machines-posts-feeds.md §3.1 come from Sandvik's materials pages, read
  earlier. Their mc is "typically 0.25".
- **Openly licensed measured values.** Klippel, Süssmaier, Kuffa, Wegener, "Dry Cutting
  Experiments Database Ti6Al4V and Ck45", ETH Zürich IWF, arXiv:2209.04197 (2022), **CC BY 4.0**
  [R]: https://arxiv.org/abs/2209.04197. Its Appendix C gives kc1.1, mc, kf1.1 and mf against vc,
  from orthogonal dry cutting.
  - Ck45 (AISI 1045), Table C.17: kc1.1 = 1935 N/mm² at vc = 10 m/min, 1672 at 100, 1707 at 150,
    1851 at 300 and 1368 at 450. The mc values are 0.19, 0.32, 0.28, 0.22 and 0.29, and the full
    spread over vc is 0.17–0.51.
  - Ti6Al4V, Table C.16: kc1.1 = 1299 at 11 m/min, 1253 at 40, 932 at 100, 749 at 254 and 662 at
    500. The mc values are 0.21, 0.22, 0.33, 0.37 and 0.41.
  - **Lesson:** mc is *not* a constant 0.25. It moves by ×2–3 with vc for one material and one
    tool. Any model must carry kc1.1 and mc as intervals with their vc, their rake and their
    source. CC BY lets forgeCAD embed these numbers with attribution.
- A rake correction kc·(1 − γ0/100) appears in Sandvik's printed formula [K]. On the web page it is
  an image (libraries doc §3.2).
- **Aluminium (6061 and similar):** we found no openly licensed measured kc1.1/mc table this
  session. The Sandvik range of about 350–700 N/mm² (libraries doc §3.1) is the only figure in
  hand, so §11 carries it as an interval.

### 3.2 Power and torque

Pc [kW] = ae·ap·vf·kc/(60·10⁶), with kc taken at hm, and Mc [N·m] = Pc·30·10³/(π·n). Both are from
Sandvik [R]. The spindle must supply Pc/η; Sandvik names η but does not define it in the page
text [R]. The torque check matters more than the power check at low rpm, where a spindle's power
curve falls off. The machine's power-versus-rpm curve is needed, and that is a machine-library
item (libraries doc §1).

### 3.3 Mechanistic (Altintas) model

- On a differential axial element dz at immersion θ, the forces are
  dFt = (Ktc·h(θ) + Kte)·dz, dFr = (Krc·h + Kre)·dz and dFa = (Kac·h + Kae)·dz. The K·c
  coefficients are shearing terms and the K·e terms are edge (ploughing) terms. A helix angle β
  lags the immersion along z by tan β·z/(D/2), and the forces are summed over the teeth and z
  [K, Altintas, *Manufacturing Automation*, 2nd ed., CUP 2012, ch. 2]. The six coefficients are
  identified from average slot forces measured at several feeds, by linear regression of F̄ on fz
  [K]. Rivière-Lorphèvre et al. (Procedia CIRP, 20th CMMO 2025) [R, abstract and §2] instead
  identify them by inverse analysis over 57 Ti6Al4V tests, with runout. They report a 5–20% lower
  RMS error than identification from a single test:
  https://orbi.umons.ac.be/bitstream/20.500.12907/52295/1/PROCIR_CMMO%202025_Riviere_VanHee_Beuscart_Ducobu.pdf
- Al7075-T6 textbook coefficients, roughly Ktc ≈ 796, Krc ≈ 168 N/mm² and Kte ≈ 28, Kre ≈ 31 N/mm,
  are [K] only. A search did not confirm them [S, no hit], so they must not be used until read.
- **Orthogonal-to-oblique transformation:** Budak, Altintas & Armarego (1996), "Prediction of
  milling force coefficients from orthogonal cutting data", *J. Manuf. Sci. Eng.* 118(2):216–224,
  doi:10.1115/1.2831014 [S]. A database of shear angle, friction and shear stress from orthogonal
  tests, passed through oblique-cutting geometry (helix as the inclination angle, plus the rake),
  predicts the K's for any cutter geometry. It was verified on Ti6Al4V [S]. This is the route by
  which one material test serves many tools. It matters for forgeCAD because tool geometry (helix,
  rake) is already in the tool record.
- The helix angle spreads each tooth's engagement over a lag angle ψ = 2·ap·tan β/D, which smooths
  force peaks. When ψ ≥ the pitch angle 2π/z, the force is nearly constant [K]. Kienzle with hm
  gives the *average* only. The mechanistic model gives the peaks, and the peaks are what deflection
  and breakage depend on.

---

## 4. Deflection

- Cantilever: δ = F·L³/(3·E·I), with I = π·De⁴/64. Helical's "rule of thumb" is that deflection
  goes with L³ and stiffness with D⁴, p. 69 [R].
- **Effective diameter of the fluted section.** Kops & Vo (1990), "Determination of the equivalent
  diameter of an end mill based on its compliance", *CIRP Annals* 39(1) [S]. Snippets say that the
  equivalent solid diameter is about 80% of D for two- and four-flute end mills. A vendor guide
  gives 0.7–0.8·D [S, mmc-carbide.com length-of-cut course]. Better models split the beam into a
  shank part and a fluted part, each with its own I [S, the same search]. The original paper is
  unread, so treat 0.8 as [S].
- **Carbide modulus.** E ≈ 550–650 GPa for WC with 5–15 wt% Co, falling as Co rises [S, US7985470
  and other patents]. Use the grade's datasheet when there is one.
- **Holder and spindle.** The cantilever ignores the compliance of the holder–spindle joint, which
  can dominate on a long assembly. Receptance coupling (§5.3) is the published way to include it
  [S, Schmitz]. The GE patent uses a cantilever with a thrust force of about 0.5× the cutting force
  [R, US4833617].
- **Runout.** A radial runout e moves the chip load between teeth: in the simplest model one tooth
  cuts fz + e and another fz − e [K, Altintas 2012]. Rivière-Lorphèvre et al. identify runout as a
  parameter [R]. At small fz, for example on finishing passes, runout can take the chip of one
  tooth to zero, which means rubbing.
- **Limit for finishing.** No standard number was read. The CNCCookbook chatter page's "0.001""
  is a minimum chip load for carbide, not a deflection limit [R]. A search snippet attributes
  "chatter can start when deflection exceeds 0.001"" to a vendor [S], which is unverified. For
  forgeCAD the honest limit is that the predicted wall error must be within the feature's
  tolerance. The wall error is the deflection component normal to the wall, which comes from the
  radial force (Krc, Kre) and not from Ft alone.

---

## 5. Chatter

### 5.1 Theory

- Regenerative chatter (Tobias; Tlusty & Polacek, 1963) [K]. The chip thickness depends on the
  present vibration and the vibration one tooth period earlier. Tlusty's oriented 1-DOF limit is
  b_lim = −1/(2·Ks·μ·Re[G]min) [K]. Below the minimum of the lobes, a cut is stable at any speed.
- **Altintas & Budak (1995)**, "Analytical prediction of stability lobes in milling", *CIRP Annals*
  44(1):357–362, doi:10.1016/S0007-8506(07)62342-7 [S, as in literature.md §8]. The zero-order
  (average) Fourier term of the time-varying directional coefficients reduces the problem to an
  eigenvalue problem in the frequency domain. Its result has the form
  a_lim = −2π·Λ_R/(N·Kt·(1 + κ²)), where κ = Λ_I/Λ_R, and the spindle speed for lobe k is
  n = 60·ωc/(N·(ε + 2kπ)) [K form]. Budak & Altintas (1998) added multi-frequency terms, which
  matter at low radial immersion [S, literature.md].
- **Semi-discretization:** Insperger & Stépán (2002), "Semi-discretization method for delayed
  systems", *Int. J. Numer. Methods Eng.* 55:503–518, doi:10.1002/nme.505 [S]. Book: *Semi-
  Discretization for Time-Delay Systems*, Springer 2011, doi:10.1007/978-1-4614-0335-7 [S]. It
  works in the time domain on a discretised map, and it captures the period-doubling (flip) lobes
  of low immersion that the zero-order method misses [K]. Discretising the delay term gives a
  linear map whose spectral radius must be below 1. That is a *finite* eigenvalue problem, the
  easiest of these to bound rigorously.

### 5.2 Inputs that a CAM program does not have

Every stability method needs the tool-point frequency response function (FRF) in x and y, and the
cutting coefficients. The FRF depends on the machine, spindle, holder, stick-out and tool, and it
changes with spindle speed and wear. It is **measured**, not looked up.
- **Tap testing:** an instrumented hammer and an accelerometer give the FRF, and software gives
  the lobes. MSC MillMax, built on Manufacturing Laboratories Inc. software and developed with Oak
  Ridge National Laboratory, presents the result as red, yellow and green rpm zones [S,
  mscdirect.com "Let's Talk About Tap Testing" / MillMax pages]. CutPro, by MAL Inc. from Altintas's
  UBC laboratory, has the MalTF tap-test module, analytical and time-domain lobes, and coefficient
  identification [S]: https://www.malinc.com/wp-content/uploads/2016/02/CutPro-11-User-Manual.pdf
  (not read). "MetalMax" (MLI) is a lead only [K].
- **Receptance coupling (RCSA):** Schmitz & Donalson (2000), "Predicting high-speed machining
  dynamics by substructure analysis", *CIRP Annals* 49(1):303– [S]. Schmitz, Davies & Kennedy
  (2001), "Tool point frequency response prediction for high-speed machining by RCSA", *J. Manuf.
  Sci. Eng.* [S]. Open copy: https://mtrc.utk.edu/wp-content/uploads/sites/45/2019/09/rcsa.pdf
  (not read). Measure the spindle and holder once, model the tool as a beam analytically, and
  couple the two to predict any stick-out with no new tap test. **This is the only route that
  scales to a CAM library**, given one measured spindle FRF per machine.
- **Harvey MAP:** the user guide says it "takes into account all of the tool's geometry" [S]. No
  public description of a stability or deflection model was found, so its approach is UNKNOWN.
  **CoroPlus ToolGuide** takes the machine's rpm, power and torque limits [S, libraries doc §3.4].
  No stability model was found for it either: UNKNOWN. **G-Wizard** predicts deflection [R,
  libraries doc §3.4]. The CNCCookbook chatter page does not say that G-Wizard computes stability
  [R].

### 5.3 What is practical without measured FRFs

1. Keep the depth below a *conservative* absolute-stability estimate. This needs only a damping
   ratio and a stiffness. The stiffness can come from the cantilever of §4. The damping is
   UNKNOWN; the user supplies it or a deliberately low value is assumed and labelled. The output
   is a warning, not a guarantee.
2. Offer an rpm *search* rather than a prediction: the "tooth-passing frequency equals the natural
   frequency" heuristic [K] places the first lobe at n ≈ 60·fn/(N·(k + 1)). Without fn this is
   empty, so expose it as an input.
3. Accept a user's FRF file (tap-test export) and compute the lobes by zero-order and
   semi-discretization methods. That is research-grade but well defined.
4. Never claim "chatter-free". Literature.md §8 already labels stability as an empirical
   eigenproblem.

---

## 6. Adaptive feedrate and feed scheduling

- **GE, US 4,833,617** (Weiping Wang, General Electric), "Solid modeling based adaptive feedrate
  control for NC machining". Filed 14 Aug 1987, granted 23 May 1989, **expired** (lapsed for
  fees, per Google Patents) [R]: https://patents.google.com/patent/US4833617. It runs the NC
  program through a solid modeller that subtracts each swept volume. Per move it computes MRR =
  volume/time, then spindle horsepower = MRR × a unit-horsepower index, then force, then a
  cantilever deflection with thrust ≈ ½ the force. The feed is set to the largest value that keeps
  force and deflection within limits, bounded to 25–400% of the baseline, and written back into
  the program. It reports a time saving of about 30% in one example. **This is free to implement
  and fits forgeCAD's stock model directly.** (The "GE 1997 Spence" in the brief did not resolve
  to a paper. The GE work found is this 1987 Wang patent.)
- **Spence & Altintas (1994)**, "A solid modeller based milling process simulation and planning
  system", *J. Eng. Ind.* 116(1):61–69, doi:10.1115/1.2901810 [S]. CSG engagement plus a
  mechanistic force model, then an inverse solution for the feed under force, torque and
  dimensional-error constraints. Spence's UBC thesis (1992) is at circle.ubc.ca/handle/2429/3062
  [S].
- **Erdim, Lazoglu & Ozturk (2006)**, "Feedrate scheduling strategies for free-form surfaces",
  *Int. J. Mach. Tools Manuf.* 46:747–757, doi:10.1016/j.ijmachtools.2005.07.036 [S]. Ball-end 3D
  milling, comparing MRR-based with force-based scheduling. MRR-based scheduling gives *higher*
  feeds, while their force-based scheduling cut machining time substantially in tests [S]. The
  numbers are UNKNOWN; the full text was not read.
- **VERICUT Force** (CGTech) [S]: https://cgtech.com/force-faq, https://vericut.com/products/force-optimization.
  It is physics-based. It keeps the chip thickness at a constant maximum, and slows when the force
  or spindle power is too high, splitting moves as needed. It writes a separate `.opti` file.
  Force material files come from dynamometer tests (a Kistler dynamometer plus calibration
  software), and the catalogue has over 100 materials. Users can calibrate their own. The vendor
  claims time savings of up to 25% in turning and "40% or more" in milling; no independent
  benchmark was found. OptiPath, the older MRR-based module, is in literature.md §8 [S].
  cgtech.com/products/product-line/force returned 404.
- **Fusion** (§9): geometric corner slowdown only. There is no engagement-based or force-based
  feed.
- **Controller-side adaptive control.** Siemens SINUMERIK "intelligent feedrate control" adapts
  the feed to the cutting conditions in real time [S]. The older ACM product sheet says it uses the
  measured spindle load to regulate the feed between 0 and 150% of programmed, on an 840D [S]:
  https://assets.new.siemens.com/siemens/assets/api/uuid:df586986-1746-4e55-9baf-4bbdd3b56ea7/acm_productinformation052018en.pdf
  (not read). FANUC's "AI" functions found (AICC) are look-ahead contouring (§7), not load
  adaptation [S]. A FANUC load-adaptive function was not found: UNKNOWN.

**The three schedules compared** (derived from §1–§3):

| target held constant | rule on a move with engagement φ(s) | needs |
|---|---|---|
| chip (hex) | fz(s) = h*/sin φ(s) for φ ≤ π/2, else h* | geometry only (provable, §10) |
| MRR | vf(s) = Q*/(ae(s)·ap(s)) | geometry only; ignores chip size, so slots are overfed and thin cuts underfed |
| force or torque | solve Σ F(h(θ; fz), K) = F* for fz | K coefficients (empirical), monotone in fz, so a bisection is exact up to the model |

Recommended for forgeCAD: compute the chip-constant feed exactly. Then take the minimum with the
force, torque and power caps, which are evaluated with *upper* interval bounds on the
coefficients. Add the arc and corner engagement of §2.3, and ramp and plunge limits. Output the
feed schedule per move, together with the engagement trace that justifies it.

---

## 7. Why the programmed feed is not achieved: acceleration, jerk, look-ahead

- The control plans the feed along the path under per-axis velocity, acceleration and jerk limits
  and a contour (blend) tolerance. Short segments and corners force decelerations. CAM that times
  moves at F alone is optimistic.
- **Ward, Sencer, Jones & Ozturk (2021)**, "Accurate prediction of machining feedrate and cycle
  time considering interpolator dynamics", *Int. J. Adv. Manuf. Technol.*,
  doi:10.1007/s00170-021-07211-2. Open copy arXiv:2102.02062 [R, abstract and §1–2]. They model
  the NC interpolator as cascaded first-order FIR filters: one filter turns a velocity pulse into
  a trapezoidal profile, and two filters give piecewise-constant jerk. The time constants set the
  acceleration and jerk. They model corner blending of linear segments, and derive analytically
  the lowest cornering feed that meets both the TCP tolerance and the kinematic limits, for 3-axis
  paths. Over 4 case studies the cycle time is predicted to within 90%, "greatly outperforming
  CAM-based predictions". They cite Heidenhain, Mitsubishi and Siemens as using FIR/IIR filtering
  for look-ahead [R].
- **Erkorkmaz & Altintas (2001)**, "High speed CNC system design. Part I: jerk limited trajectory
  generation and quintic spline interpolation", *Int. J. Mach. Tools Manuf.* 41:1323–1345,
  doi:10.1016/S0890-6955(01)00002-5 [S]. It gives the jerk-limited (S-curve) feed profile, which is
  7 segments in the general case [K]. A follow-on, Erkorkmaz & Altintas (2005), "Quintic spline
  interpolation with minimal feed fluctuation", *J. Manuf. Sci. Eng.*, doi:10.1115/1.1830493 [S].
- **FANUC AI Contour Control** (G05.1 Q1): FANUC's brochure says it looks ahead in the program to
  cut the acceleration, deceleration and servo delays on short segments and corners [S,
  fanucamerica.com fanuc_30ib_low.pdf]. Third-party pages disagree on look-ahead depth (40 against
  200+ blocks) and parameter numbers [S]. Treat these as UNKNOWN.
- **What CAM should do:**
  1. Report three times: `t_min` (at F), `t_stop` (exact stop, trapezoid; forgeCAD has both, §0),
     and a **blended, jerk-limited** estimate. The third is FIR-filter based per Ward et al., with
     the machine's per-axis a, j and the blend tolerance in the machine file.
  2. Lower bound: no control is faster than its per-axis velocity and acceleration limits allow,
     so time ≥ the time-optimal trapezoid per segment with blended corners. A provable floor
     needs a statement of the corner model.
  3. Feed scheduling (§6) must take the minimum of the feed the process allows and the feed the
     machine reaches. Otherwise the chip-constant feed raised on a short arc is never reached, and
     the chip in that corner is not what was planned. The direction of that error is usually safe
     (less feed, thinner chip), but it raises the risk of rubbing.

---

## 8. Tool-maker data

**Read this session [R]:**
- Helical Solutions (a Harvey Performance company), **H45AL-3 / H45AL-RN-3** sheet: 3 flutes, 45°
  helix, aluminium. https://ecomm.productivity.com/customer/docs/skudocs/SF_H45AL-3.pdf
  - Format: SFM per material group, and IPT per diameter (1/8 to 1") × operation (Slot, Rgh, Fin).
  - Wrought aluminium (2014, 5052, 6061, 7050, 7075, 7475): **SFM 2100**. At 1/2" the IPT is
    **Slot 0.0032, Rgh 0.0059, Fin 0.0032**. Cast aluminium: SFM 1400, with 1/2" Slot 0.0049 and
    Rgh 0.0092. Copper alloys: SFM 770.
  - The ADOC and RDOC per operation are in §2.4. The sheet refers to Machining Advisor Pro for
    "more accurate" values.
  - Derived: the Fin column (RDOC 4–6%) is **not** chip-thinning adjusted. RCTF at 5% is 2.29, but
    Fin IPT = Slot IPT. The Rgh column at 35% RDOC (RCTF 1.05) is 1.84× the slot value, so the
    roughing IPT is not "the slot chip, thinned". The columns are separate empirical choices, and
    a CAM program must not re-thin a number that is already a programmed feed.
- Helical **HAL-C-3 and H35AL-3** catalogue pages, from the 2016 catalogue, p. 22 ff.
  https://cdn.shopify.com/s/files/1/0612/0733/files/Helical_Aluminum_Parameters.pdf
  - Wrought (2024, 2219, 5052, 7050, 6061, 7075): SFM **1600–2400**.
  - FPT at 1/2" is given low–high: S **0.0035–0.0047**, HR **0.0047–0.0063**, LR
    **0.0058–0.0079**. The S row is for slotting 1×D deep, with the note to "use 70% of maximum
    machine RPM".
  - Cast aluminium: SFM 720–1080.
  - **This shows parameterisation by operation class (S/HR/LR), with FPT rising as RDOC falls.**
- Helical Machining Guidebook (2016) [R]: thinning on p. 22, HEM on p. 24 (use the whole flute
  length at lower RDOC; work and heat spread along the edge), deflection on p. 69.
- **Licence.** The guidebook says: "All rights reserved. This book or any portion thereof may not
  be reproduced or used in any manner whatsoever without" permission (p. 1, [R]). The SF sheets
  carry no licence; the Helical parameters PDF calls its figures "theoretical ... for planning
  purposes" (per a search summary [S]). **Embedding tables is not licensed.** Citing single values
  as facts with URL (as here), or letting the user import a vendor's file they hold, is the safe
  route. Facts are restated and the tables are not reproduced.

**Not read: UNKNOWN, leads only.** Harvey Tool (small-diameter tools; its per-tool PDFs have the
same layout per [K]), Kennametal (NOVO, ISO 13399 export, libraries doc §3.4 [S]), Sandvik
(CoroPlus; per-insert data in catalogues), Iscar (ITA online advisor [K]), Garr Tool, OSG. The
format and licence of each are UNKNOWN. **ISO 13399 / GTC** is the vendor-neutral path for tool
geometry. Cutting data in GTC packages is vendor-specific [K].

**Typical parameterisation across vendors** ([R] Helical; others [K]): vc or SFM per material
group and condition (hardness), fz per diameter (a table or a linear fit), an operation class
(slot, rough or HEM, finish) that fixes ae and ap ranges, multipliers for length of cut or reach,
and a "use thinning below 50% ae" rule. forgeCAD's `fz_d1/fz_d20` linear fit is the same idea with
one operation class.

---

## 9. Fusion and the third-party calculators

- **Fusion Feed & Speed fields** [R], from https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-REF-COMMON-TOOL.htm:
  Preset, Spindle Speed, Surface Speed, Ramp Spindle Speed, Cutting Feedrate, Feed per Tooth,
  Lead-In and Lead-Out Feedrate, Transition Feedrate, Ramp Feedrate (helical ramps), Plunge
  Feedrate, and Feed per Revolution (plunge). There are drilling variants and Coolant. The page
  says **nothing about chip thinning or material**, and nothing about computing feeds
  automatically. Only presets supply cutting data.
- **Feed Optimization** [R], from
  https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/GUIDCFB956EA-793B-4F8A-B487-028219913D96.htm
  and the 2D Pocket reference: Maximum Directional Change, Reduced Feed Radius, Reduced Feed
  Distance, Reduced Feedrate and Only Inner Corners. These are purely geometric corner rules. The
  2D Pocket page also has Finish Feedrate, Other Way Feedrate and High Feedrate (G1 rapids) [R].
  Forum users report that dense segments hide corners unless smoothing fits arcs [S].
- **Fusion adaptive** [S]: "Optimal Load" is the maximum stepover. "Minimum Cutting Radius" keeps
  the path out of sharp corners so that a higher feed can be held, leaving stock for a later op.
  No feed formula is tied to either. Engagement is held by path shape, not by varying the feed.
- So Fusion has **no chip-thinning computation, no material-driven data, no power or torque
  check, and no engagement-based feed** as far as the read pages show. A forum statement that
  material does not drive feeds is in the libraries doc §3.3 [S]. Its strength is the adaptive
  path itself.
- **G-Wizard** [R, CNCCookbook pages]: radial chip thinning below 50% ae, ball-nose speed and
  feed compensation, lead-angle compensation, and a rubbing warning (the CNCCookbook page cites
  Ingersoll for a carbide floor of about 0.004"). It also does power limits and deflection (libraries
  doc §3.4 [R]). **HSMAdvisor and FSWizard** (zero-divide): a deflection- and torque-limited
  virtual tool, chip thinning, and ae and ap suggestions (libraries doc §3.4 [R]). None publishes
  its models, so their outputs can be compared against ours but not reproduced.

---

## 10. What is provable or exact in forgeCAD, and what stays data

**Exact geometry (provable in Lean, G-level statements):**

1. *Engagement*: for a path P and the in-process stock S (forge-cam `stock.rs`), the engaged arc
   at a point is the set of θ where the tool's circle lies in S. On offset and adaptive paths
   built with a known stepover, a lemma of the form "∀ moves, φ ≤ φ̄(s, r, corner radius bound)"
   follows from the §2.3 formulas. `Pocket.lean`'s offset levels are the setting for it. An open
   item: offset pockets do **not** bound engagement at concave corners unless the corner radius is
   bounded below, which is exactly the §2.3 example.
2. *Chip bound from engagement bound*: with the first-order chip h(θ) = fz·sin κ·sin θ, if
   φ ≤ φ̄ then hex ≤ fz·sin κ·sin(min(φ̄, π/2)), by the monotonicity of §1.1. This is a clean real
   analysis lemma. The **trochoidal correction** (exact tooth paths) is second order in fz/D. A
   proved bound including it, h_true ≤ fz·sin θ + c·fz²/D for an explicit c, is research work
   that has **not** been done: UNKNOWN whether a tidy c exists in the literature. Martellotti 1941
   [K] is the place to start.
3. *Mean chip and MRR*: hm = fz·sin κ·(1 − cos φ)/φ exactly under the same model. MRR per move is
   exactly the volume of the swept solid minus the stock, as an exact computation on forgeCAD's
   stock (the GE patent's method, §6, which has expired).
4. *Feed monotonicity*: the force (Kienzle or mechanistic) is increasing in fz for mc < 1, because
   F ∝ h^(1−mc). So "largest fz with F ≤ F*" is well defined and bisection is sound. Proof: the
   derivative (1 − mc)·h^(−mc) > 0 for 0 ≤ mc < 1.
5. *Interval coefficients*: for h < 1 mm, ln h < 0, so h^(−mc) is **increasing in mc**. Then
   kc ≤ kc1.1_hi·h^(−mc_hi), and P and T are increasing in kc. A conservative power or torque cap
   is the formula evaluated at the upper endpoints. This is provable and it is the right shape for
   Klippel's data, where mc spans 0.17–0.51.
6. *Kinematic floor on time*: per-axis v and a limits give a lower bound on the time of each move.
   It is provable once the corner and blend model is stated. Upper bounds depend on the control's
   planner (UNKNOWN per control).

**Empirical data. Never "proved", always labelled with its source and scatter:**
vc, fz, kc1.1, mc, the K coefficients, Taylor tool life, E for carbide (550–650 GPa), the De/D
ratio of about 0.8, damping, FRFs, the stability lobes (a numerical eigenproblem on measured data),
spindle power curves, and vendor operation classes.

---

## 11. Worked example: 6061, 1/2" 3-flute carbide, adaptive (ae = 10% D) against a slot

**Inputs.**
- Tool: Helical H45AL-3, 1/2" (D = 12.7 mm), z = 3, carbide, non-reached, 2.5×D LOC. Data from
  SF_H45AL-3.pdf [R].
- Material: 6061, wrought, "SFM 2100" [R].
- Assumed machine: max 10,000 rpm. This is an **assumption** made for the example, not data.
- kc1.1 = 350–700 N/mm² (Sandvik N-group aluminium range, libraries doc §3.1), with mc = 0.25
  ("typically", same source). Both are carried as intervals.
- E = 550–650 GPa [S]. De = 0.8·D [S]. Stick-out L = 40 mm, an assumption.

**Speed.** n = 12·2100/(π·0.5) = 16,043 rpm, which Helical writes as 3.82·SFM/D [R]. That is above
10,000, so n = 10,000 rpm and vc = 399 m/min. (forgeCAD's N group asks for 400 m/min. The
machine, not the data, binds here.)

**Slot** (ae = D, ap = 1·D = 12.7 mm, which is in Helical's 75–125% band [R]):
- fz = 0.0032 in = 0.0813 mm [R]. φ = π, hex = fz = 0.0813 mm, and hm = 2fz/π = 0.0517 mm
  (derived).
- vf = 0.0813·3·10,000 = 2438 mm/min (96 ipm). Q = 12.7·12.7·2438 = **393 cm³/min**.
- kc = kc1.1·0.0517^(−0.25) = 2.10·kc1.1, which gives 734–1468 N/mm².
- Pc = Q·kc/6e7 = **4.8–9.6 kW**, and the torque is 4.6–9.2 N·m.
- The mean tangential force is T/r = 723–1447 N. As a tip load on a 40 mm cantilever with
  De = 10.16 mm, δ = 45–107 µm. This is the mean force: peaks are higher, and most of it acts in
  the feed direction, so it is not wall error.

**Adaptive, ae = 1.27 mm (10% D), ap = 25.4 mm (2×D, within a 2.5×D LOC; HEM uses the flute
length, Helical p. 24 [R]):**
- φ = arccos(0.8) = 36.87°, sin φ = 0.600, and RCTF = 1.667 (derived).
- (a) Holding the slot chip: fz = 0.0813/0.6 = 0.1355 mm (0.00533 in). Then hex = 0.0813 and
  hm = fz(1 − 0.8)/0.6435 = 0.0421 mm. vf = 4064 mm/min (160 ipm), so Q = **131 cm³/min**.
  kc = 773–1545 N/mm², so Pc = **1.7–3.4 kW** and the torque is 1.6–3.2 N·m. The mean Ft is
  254–508 N, so δ = 16–38 µm.
- (b) Holding the *roughing* chip: the Rgh 0.0059 in at 35% RDOC gives hex = 0.0059·0.954 =
  0.00563 in, so fz = 0.00938 in = 0.238 mm. Then vf = 7148 mm/min (281 ipm), Q = **231 cm³/min**,
  Pc = 2.6–5.2 kW and δ = 24–58 µm. This extrapolates a vendor's roughing chip to a thinner
  engagement, which Helical's own table does not do (§8). **Flag it as an extrapolation.**
- Corner check, from §2.3: at a 10 mm concave wall radius φ climbs to 60.9°, so hex rises to
  0.873·fz. In case (a) that is 0.118 mm, which is 1.46× the planned chip, unless the feed is
  scheduled down (§6) or the path keeps a minimum cutting radius.

**Reading the numbers.** On equal maximum chip, this adaptive cut removes *less* volume per minute
than the full slot (131 against 393 cm³/min). Its gains are elsewhere: about 1/3 of the force,
torque and deflection; heat and wear spread over 2×D of flute; and no full-slot chip packing. The
MRR advantage appears only if the higher spindle speed or the roughing chip (b) is used, or a
larger tool is run at the same power, which is what HEM tool makers advise. Any claim of "X% faster"
must state which baseline chip it holds.

**forgeCAD today**, on the same inputs: its power would use kc at hex, not hm, and so understate
Pc by 12% for the slot and 18% for the adaptive cut (§0 finding 1). Its fz would come from
`cutting.toml` N (`fz_d20 = 0.12`, interpolated to 0.0777 mm at 12.7 mm), which is close to
Helical's slot 0.0813 mm, but neither value is cited.

---

## 12. Recommendations (ordered)

1. Fix the hm/hex use in `feeds.rs` power, and carry kc1.1 and mc as intervals with a source per
   value. Replace the uncited `cutting.toml` numbers with cited ones: Klippel CC BY for Ck45 and
   Ti6Al4V, and Sandvik ranges cited by page.
2. Add sin κr, bull-nose and tilted-ball Deff (§1.2–1.3).
3. Feed scheduling per move: the chip-constant feed from the *actual* engagement measured on the
   in-process stock (GE method, now expired), capped by power, torque and a deflection limit
   evaluated at the interval upper bounds. Corner and arc engagement per §2.3.
4. A machine file with per-axis jerk and a blend tolerance, and a third, FIR-based time estimate
   (Ward et al.). Then schedule the minimum of the process feed and the reachable feed.
5. Lean: the engagement-to-chip lemma (§10.2) and the interval monotonicity (§10.5). These are
   small, and they make "the chip never exceeds h*" a theorem about every path an op emits, which
   no commercial CAM claims.
6. Chatter: accept a user FRF, compute zero-order and semi-discretization lobes, and present them
   as advice. RCSA is a later step.

---

## 13. Unreachable or not read

| item | status |
|---|---|
| Harvey "Combat Chip Thinning" (harveyperformance.com/in-the-loupe/combat-chip-thinning/) | fetched; empty body this session |
| cgtech.com/products/product-line/force | HTTP 404; snippets of cgtech.com/force-faq and vericut.com used |
| MDPI J. Manuf. Mater. Process. 8(2):83 (Al7075 coefficients?) | HTTP 403 |
| Altintas *Manufacturing Automation* (2012); Altintas & Budak 1995; Budak et al. 1996; Insperger & Stépán 2002; Erkorkmaz & Altintas 2001; Erdim et al. 2006; Spence & Altintas 1994; Schmitz & Donalson 2000; Kops & Vo 1990 | not read: paywalled or not attempted; bibliographic data from index snippets [S] |
| Sandvik formula images (hm, kc with rake) | images; text not available |
| Helical catalogue pp. 102–103 (arc feed) and 108–109 (ball nose) | not found |
| CutPro 11 manual; UTK RCSA PDF; Siemens ACM sheet; FANUC 30i-B brochure | found, not read |
| Kennametal, Iscar, OSG, Garr speeds and feeds; Harvey MAP internals; CoroPlus internals | not read: UNKNOWN |
| Mitsubishi inclined ball-nose formula | not read (as before) |
