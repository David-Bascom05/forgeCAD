# Head-to-head comparisons of CAM systems and toolpath strategies, with measured results

2026-11-24. Research only. The brief: find published studies that compare CAM systems, or toolpath
strategies, on a defined test part and report numbers measured on a real machine (cycle time, Ra
or Sa, tool wear, forces, energy or power). For each, say whether forgeCAD could rebuild the part,
generate its own path, and compare a predicted cycle time and metrics against the measured ones.

Marks, as in the rest of `research/cam/`:
- **[R]** read this session (full text, or the abstract or record page where stated);
- **[S]** search-engine snippet only;
- **[K]** background knowledge, not re-read.

Anything not read is UNKNOWN. Open-access sources only. Every site that refused us is listed in §7.

Not repeated here (see the cross-references):
- RPTU 10-part benchmark, Zenodo 17035762: STEP plus G-code posted from an Autodesk machine
  definition, but no measured cycle times or Ra. See [datasets.md](datasets.md) and
  [datasets-parts.md](datasets-parts.md) §11.
- Ibaraki, Stori & Wright, and Inui, Nakatsuji & Nishida (*IJAT* 20(4), 2026: semi-finishing time
  90.1 → about 22 min by cutting less air). See
  [../toolpath-quality/roughing-linking.md](../toolpath-quality/roughing-linking.md) §2 and §4.2.
- Ward et al. 2021 is already cited for its cycle-time *model*
  ([../toolpath-quality/feeds-speeds.md](../toolpath-quality/feeds-speeds.md),
  [../toolpath-quality/README.md](../toolpath-quality/README.md) §1.2). Here it is used for its
  *measured* times (§2.1).

## 0. Summary

- **No open, vendor-independent, multi-CAM benchmark with measured results was found.** The only
  multi-vendor trial found is CETIM's (2003). It survives as a trade-press report: the part is not
  described and no absolute times are given (§4).
- **Two studies can be rebuilt well enough to test forgeCAD's cycle-time and strategy claims
  quantitatively:**
  - Ward et al. 2021 (arXiv, CC BY): contour and trochoidal pockets from NX. Measured time vs the
    CAM estimate at 1000/3000 mm/min and 10/50 µm tolerance. Also an aerostructure program where
    CAM underestimated the time by 36–62 %.
  - Bílek et al. 2025 (Materials, CC BY): a 60 × 46 × 12 mm pocket. NX Adaptive vs Cavity Mill,
    full cutting data, CAM-estimated vs measured cycle time, forces and Ra.
- **Cycle-time pattern across sources.**
  - Adaptive or trochoidal roughing beat offset or conventional roughing by about 20–35 % in
    independent measured studies.
  - Vendor and single-case reports claim 60–80 %.
  - Two studies found no tool-wear or vibration benefit, or worse accuracy, for trochoidal paths
    (Gross 2020; Bílek 2025 on dimensions and burrs).
- **CAM cycle-time estimates are low.** They come closest on simple offsets at low feed (≤ 1 %
  error). They are worst on trochoidal and HSM paths at tight tolerance (−55 % on one pocket,
  −62 % on a whole aerostructure part). This is the strongest single argument for forgeCAD's planned
  acceleration-aware estimate.

## 1. Method and scoring

**Reproducibility score, 1–5** (5 = part, parameters and measurements all published):
- **5:** full drawing or CAD, every cutting parameter, the path settings, and measured numbers in
  tables.
- **4:** part dimensions enough to model; parameters complete; measured numbers in tables; some
  path settings missing.
- **3:** part only partly dimensioned, or results only in figures; parameters mostly given.
- **2:** part shown only as a picture or CAD render; partial parameters.
- **1:** no usable part; or simulation only; or numbers only as percentages.

"Rebuild" means writing the part as a `.fg` file from what the paper states, not from a photo.

## 2. Studies (full detail)

### 2.1 Ward, Sencer, Jones, Ozturk 2021: measured vs CAM cycle time, contour and trochoidal pockets

- **Citation.** "Accurate prediction of machining feedrate and cycle time prediction considering
  interpolator dynamics," arXiv:2102.02062. Journal version DOI 10.1007/s00170-021-07211-2. CC BY
  4.0. **[R, full text]** https://arxiv.org/abs/2102.02062
- **Compared.** A contour-parallel pocket against a trochoidal pocket, both from Siemens NX CAM. NX's
  own time estimate, the measured time, and the authors' FIR-interpolator prediction.
- **Machine.** DMG Mori Universal eVo 40, five-axis, Heidenhain TNC640 [R].
- **Case 4 force test** [R]:
  - open-sided pocket 40 × 60 × 10 mm in Al 7075 blanks of 236 × 30 × 6 mm;
  - 2-flute Ø12 mm solid carbide end mill, HSK-63A;
  - Kistler 9139AA dynamometer.
- **Pocket dimensions for cases 1–2:** UNKNOWN. The cutting-condition table was not extracted;
  figure 15 shows the paths.
- **Measured** [R, Table 1]. Times in seconds; error = CAM vs measured.

| path | F mm/min | tol µm | measured | NX estimate | NX error | FIR prediction |
|---|---|---|---|---|---|---|
| contour | 1000 | 10 | 27.07 | 26.56 | −1.9 % | 27.32 |
| contour | 3000 | 10 | 10.37 | 8.85 | −14.7 % | 10.46 |
| contour | 3000 | 50 | 9.60 | 8.85 | −7.8 % | 9.73 |
| trochoidal | 1000 | 50 | 40.78 | 40.69 | −0.2 % | 41.30 |
| trochoidal | 3000 | 10 | 30.13 | 13.56 | **−55.0 %** | 30.99 |
| trochoidal | 3000 | 50 | 14.04 | 13.56 | −3.4 % | 14.40 |

- **Aerostructure program** [R, Tables 4–5]: roughing plus floor and wall finishing at 8000 mm/min.

| tol µm | measured s | CAM estimate s | CAM error | FIR prediction error |
|---|---|---|---|---|
| 10 | 1694.5 | 637 | −62.4 % | +3.5 % |
| 20 | 1313.0 | 637 | −51.5 % | −5.3 % |
| 50 | 1001.8 | 637 | −36.4 % | −4.7 % |

- **Rebuild.** The pocket of case 4 can be rebuilt. The aerostructure part cannot: it is shown only
  as a picture.
- **Why it matters.** The CAM estimate does not change with tolerance (637 s at every setting).
  The measured time does: it changes by 69 % between 10 and 50 µm.
- **Score: 4** (pocket cases), **2** (aerostructure).

### 2.2 Bílek, Řezníček, Matras, Solařík, Macků 2025: NX Adaptive vs Cavity Mill on a GFRP pocket

- **Citation.** "An experimental investigation into trochoidal milling for high-quality GFRP
  machining," *Materials* 18(7):1669. DOI 10.3390/ma18071669, CC BY 4.0. **[R, full text via
  PMC11990107]**. The last third of the article (about 47 k characters) was not read.
- **Compared.** Siemens NX 12: Adaptive Milling (trochoidal) against Cavity Mill (conventional).
  Six Ø6 mm Seco carbide tools were tested trochoidally; tool T3 was used for both strategies.
- **Part** [R]: GFRP block about 76 × 76 × 40 mm; oblong pocket 60 × 46 × 12 mm with 4 mm corner
  radii; unidirectional GFRP with DT806 epoxy.
- **Machine** [R]: DMG Mori DMU 50 (3rd gen.); Kistler 9129AA dynamometer.
- **Parameters** [R]:

| | trochoidal | conventional |
|---|---|---|
| vc | 220 m/min (11,671 rpm) | 145 m/min (7,692 rpm) |
| vf | 2652 mm/min | 1000 mm/min |
| ap | 12 mm (2D) | 1.5–3.5 mm |
| ae | 0.42 mm (7 %) | 1.8 mm (30 %) |
| MRR stated | 13.37 cm³/min | 2.7–6.3 cm³/min |

- **Measured, tool T3** [R]:

| metric | trochoidal | conventional |
|---|---|---|
| cycle time, NX estimate | 3 min 49 s | 5 min 26 s |
| cycle time, measured | **4 min 11 s** (+9.6 % over CAM) | **5 min 27 s** |
| Ra, floor parallel to fibres | 1.36 ± 0.03 µm | 1.73 ± 0.06 µm |
| Ra, floor normal to fibres | 1.48 ± 0.05 µm | 1.56 ± 0.07 µm |
| long side (nominal 60) | 59.890 mm | 59.965 mm |
| short side (nominal 46) | 45.913 mm | 46.023 mm |
| Fy, RMS / max | 26.6 / 141.0 N | 24.96 / 144.5 N |
| flank wear | < 50 µm | < 50 µm |
| cost index | 100 % | 114.5 % |

- **Result.** Measured time was 23 % shorter with the trochoidal path. Conventional milling gave
  better dimensions, smaller burrs, and the higher weighted score (7.22 against 5.61).
- **Caveats** [R]. The authors' "25 % lower force" claim does not match their T3 table. Wall-Ra
  labels and burr units are inconsistent in the paper.
- **Rebuild: yes.** Block and pocket are fully dimensioned. The NX path settings beyond ae, ap and
  feeds are not given.
- **Score: 4.**

### 2.3 Shajari, Sadeghi, Hassanpour 2014: four finishing strategies on a convex surface

- **Citation.** "The influence of tool path strategies on cutting force and surface texture during
  ball end milling of low curvature convex surfaces," *Sci. World J.* 2014:374526. DOI
  10.1155/2014/374526, CC BY. **[R, full text PMC3951060]**
- **Compared.** Raster, 3D offset, radial and spiral finishing. The CAM system is not named.
- **Part** [R]:
  - block 82 × 60 × 16 mm, convex, slope 0–32° from the y-axis;
  - **curvature radius not reported**;
  - DIN 1.4903 (X10CrMoVNb9-1) stainless steel, 270 HB.
- **Machine and tool** [R]:
  - Hartford 3-axis machining centre, Fanuc 0M control, 8000 rpm, 11 kW;
  - Walter Ø12 mm two-flute TiN ball end mill;
  - emulsion coolant;
  - ap 0.5 mm.
- **Design** [R]. Taguchi L9 over vc 60/120/180 m/min, fz and step-over 0.3/0.7/1.1 mm. The
  paper's fz levels are inconsistent between tables.
- **Measured, test 9** (vc 180, fz 0.18, ae 0.7) [R]:

| | raster | 3D offset | spiral | radial |
|---|---|---|---|---|
| time, min | 1.43 | 2.02 | 1.50 | 5.03 |
| total force, N | 736.6 | 741.6 | 1408.1 | 612.0 |
| roughness, µm (parameter not named) | 1.793 | 1.514 | 2.037 | 1.261 |

- **All nine runs.** Times for all four strategies are tabulated (radial is about 3× raster in every
  run), as are forces. Roughness is given only for test 9.
- **Rebuild: partly.** A cylinder or sphere segment must be guessed to match 0–32° on 82 × 60.
- **Score: 3.**

### 2.4 Globocki-Lakić, Sredanović, Jotić, Gotovac 2022: SolidCAM roughing and finishing combinations

- **Citation.** "A comparative analysis of milling strategies of complex geometry surfaces," *FME
  Transactions* 50(4):623–634. DOI 10.5937/fme2204623G. Free to read; the licence line says
  "All rights reserved." **[R, full text]**
- **Compared** (SolidCAM):
  - roughing: Contour against Hatch;
  - finishing: Constant step-over, Spiral and Linear.
- **Part** [R]:
  - Al 5083 blank 80 × 60 × 40 mm;
  - a freeform surface with concave and convex regions, **shown only as a CAD render**.
- **Machine and tool** [R]:
  - Emco Concept MILL 450, Sinumerik 810D/840D;
  - Dormer S501 two-flute carbide ball mill, Ø12, for both roughing and finishing;
  - emulsion.
- **Parameters** [R]:
  - roughing ae 2, ap 1.5, 400 mm/min, 4000 rpm;
  - finishing ae 1, ap 0.5, 300 mm/min, 6000 rpm;
  - profile tolerance 0.01.
- **Measured** (stopwatch) [R, Table 4]:

| run | strategies | rough h | finish h | total h |
|---|---|---|---|---|
| EXP1 | Contour + Constant step-over | 0.510 | 1.014 | 1.524 |
| EXP2 | Hatch + Spiral | 0.600 | 1.380 | 1.980 |
| EXP3 | Contour + Linear | 0.510 | 1.230 | 1.740 |

- **Other results.**
  - Contour roughing took 15 % less time than Hatch.
  - Ra and Rz appear only in figures. EXP2 was by far the worst; EXP1 was best.
  - A second phase with retuned parameters (EXP4) took 1.99 h but gave 1.5–2× lower Ra.
- **Rebuild: no**, not without the CAD model. Machine, tool and parameters are complete.
- **Score: 2.**

### 2.5 Silva et al. 2024: plunge milling vs conventional side milling of deep slots

- **Citation.** "A comparative study of different milling strategies on productivity, tool wear,
  surface roughness, and vibration," *J. Manuf. Mater. Process.* 8(3):115. DOI
  10.3390/jmmp8030115, CC BY 4.0.
  - Read: abstract via the Crossref API **[R]**.
  - Full text: mdpi.com returned 403 (§7).
- **Compared.** Plunge roughing against conventional side milling of deep slots in AA7050-T7451.
  Both tools carried a DLC-type coating [R].
- **Measured.**
  - Plunge roughing cut cycle time by about 20 % [R abstract], 20.7 % in the conclusions [S].
  - Tip wear did not increase [R abstract]; plunge tools wore slightly less [S].
  - Vibration was lower with conventional milling (3.61 mm/s) [S].
- **Slot dimensions, machine and parameters:** UNKNOWN.
- **Score: UNKNOWN, provisionally 3.**

### 2.6 Amaro, Ferreira, Simões 2020: trochoidal vs conventional, duplex stainless steel

- **Citation.** "Comparative analysis of different cutting milling strategies applied in duplex
  stainless steel," *Procedia Manufacturing* 47. DOI 10.1016/j.promfg.2020.04.132. Open access
  (Procedia, CC BY-NC-ND) [K]. **[S]**. The Coimbra repository copy looped on redirects; RCAAP
  showed a bot challenge (§7).
- **Measured** [S]:

| strategy | vc | tool life | MRR |
|---|---|---|---|
| conventional (best) | 120 m/min | 20 min | 17.16 cm³/min |
| trochoidal | 240 m/min | 20 min | 23.66 cm³/min |

- **Wear** [S]: flank wear, then chipping. A related thesis names notch wear as the cause of
  breakage.
- **Part, machine, tool and CAM:** UNKNOWN.
- **Score: UNKNOWN (≥ 2).**

### 2.7 Gómez et al. 2021: trochoidal, plunge and conventional full-slot roughing

- **Citation.** "Comparison between milling roughing operations in full slotting manufacturing:
  trochoidal, plunge and conventional milling," *IOP Conf. Ser.: Mater. Sci. Eng.* 1193:012003.
  DOI 10.1088/1757-899X/1193/1/012003, open access. **[S]**. iopscience returned 403.
- **Compared.** Production time for full slots by three methods (UPV/EHU) [S].
- **Results, part and whether times were measured on a machine:** UNKNOWN.
- **Score: UNKNOWN.**

### 2.8 Gross, Friedl, Meier, Hanenkamp 2020: linear vs trochoidal, wear and vibration

- **Citation.** "Comparison of linear and trochoidal milling for wear and vibration reduced
  machining," *Procedia CIRP* 90:563–567. DOI 10.1016/j.procir.2020.01.069. **[R, abstract via FAU
  CRIS]**. Full text: sciencedirect returned 403.
- **Result** [R abstract]. Across several component geometries, trochoidal milling was **not** a
  suitable strategy for reducing vibration or tool wear compared with linear milling.
- **Numbers and part:** UNKNOWN.
- **Score: UNKNOWN.** Important as the counterexample.

### 2.9 Energy studies: Aramcharoen & Mativenga 2014; Edem et al. 2017 and 2019; Pervaiz; MSEC 2016

- **Aramcharoen & Mativenga.** "Critical factors in energy demand modelling for CNC milling and
  impact of toolpath strategy," *J. Clean. Prod.* 78:63–74 (2014), DOI
  10.1016/j.jclepro.2014.04.065. **[S]**
  - Builds energy states and power constants per machine.
  - Says that comparing alternative toolpaths "identified major opportunities" for saving energy.
  - Numbers UNKNOWN. No open copy found.
- **Edem, Balogun, Nkanang, Mativenga 2019.** *IJAMT* 103:997–1007. **[S]**
  - hyperMILL true spiral, rectangular spiral and square contour pockets in AISI 1018.
  - True spiral had the lowest energy and time, and a better floor finish.
  - Springer redirected to a login, so open-access status is UNKNOWN.
- **Edem et al. 2017** **[S]**. The most energy-efficient strategy differed between two 3-axis
  machines.
- **Pervaiz, Deiab, Rashid, Nicolescu.** "An experimental analysis of energy consumption in milling
  strategies" (IEEE conference) **[S]**. Zigzag, constant-overlap spiral, parallel spiral and one-way
  pockets in Al 6061 with Ø8 HSS cutters. Numbers UNKNOWN; not open.
- **MSEC2016-8746.** "A comparative analysis between high speed dynamic and traditional pocketing
  toolpaths in precision milling machines" **[S, abstract]**. The dynamic path beat the spiral on
  tool wear, energy and CO₂. Numbers UNKNOWN; ASME, not open.
- **Uhlmann et al. 2013**, GCSM Berlin, Ti-6Al-4V:
  - trochoidal vs conventional milling: average machine power **+6 %**, process time **−35 %**
    [R, as cited in Akhavan Niaki et al. 2018, NSF PAR copy; +6 % also in the Fraunhofer
    abstract, S];
  - so energy per part fell [derived from the two figures, assuming the same volume removed].
  - Fraunhofer Publica returned 503.
- **Score:** 1–2 for all of these as reachable today.

### 2.10 Infante Castillo & Culqui Culqui 2021: adaptive vs conventional HSM roughing, 7075 (Prodax)

- **Citation.** *Lecture Notes in Electrical Engineering* 763. DOI 10.1007/978-3-030-72212-8_9.
  **[S]** Springer chapter; open-access status UNKNOWN.
- **Compared** [S]:
  - conventional: ap 2.188 mm, ae 64 % of D;
  - adaptive: ap 14 mm, ae 10 %.
- **Result** [S]. ANOVA: adaptive had the lowest time, best case **−82.3 %**.
- **Score: UNKNOWN.**

### 2.11 Pleta, Ulutan, Mears 2014–2015: trochoidal vs end milling, IN738

- **Citations.**
  - ASME MSEC 2014, V002T02A058.
  - *Procedia Manufacturing* 1:556–566 (2015), DOI 10.1016/j.promfg.2015.09.032 (open access [K]).
- **Result** [R, as summarised in Akhavan Niaki, Pleta, Mears, *IJAMT* 2018, NSF PAR copy]:
  - compared force, wear and roughness;
  - introduced "material removed per wear" as a metric;
  - reported **up to 600 %** more material removed per unit of wear for trochoidal paths.
- **Not fetched:** the Procedia original. The follow-up papers (Pleta et al. 2019 *J. Manuf.
  Process.* 43:54–62 [R]; Wagih et al. 2023 [R abstract]) are trochoidal-only parameter studies,
  not head-to-head.
- **Score: UNKNOWN (≈ 3 if the slot geometry is given).**

### 2.12 Smaller or weaker items

- **Chuchala et al. 2021**, *Materials* 14:3036, CC BY **[R, Europe PMC full text]**.
  - Three face-milling pass sequences on 60 × 200 mm plates of EN AW-6082-T6, 6, 8 and 12 mm thick.
  - Ø63 face mill, 5 APMT1604 inserts, 1400 rpm, 600 mm/min, full width ae 60. Heidenhain TNC640.
  - Differences between strategies were *not* statistically significant. Cutter-axis position
    mattered more: Ra 0.26–0.29 µm in up-milling, 0.63–0.68 µm in the transition zone.
  - Fully rebuildable, but it tests pass sequencing, not CAM paths. **Score 4, low relevance.**
- **Mikó, Varga, Zębala 2019**, *Materials* 12:4038, CC BY **[R, abstract via Europe PMC]**.
  - Ball-end milling of convex and concave cylinders in 42CrMo4, varying only the feed direction.
  - Measured Ra, dimensional error and form error. Dimensions UNKNOWN: the full text returned 500
    or a browser check.
  - **Score: UNKNOWN**; promising for a finishing-direction test.
- **Bagci & Yüncüoğlu 2017**, *Strojniški vestnik* 63(11):643–656 **[R, abstract on dLib]**.
  - Roughing strategies on sculptured surfaces.
  - Measured dynamic tool deflection (two inductive probes) and forces; machining time;
    engagement and MRR from a B-rep simulation.
  - Strategy names and numbers UNKNOWN. **Score: UNKNOWN.**
- **But & Popescu 2025**, *Acta Tech. Napocensis* 68(Special I) **[R, full text]**.
  - VoluMill-style deep roughing (ap 15–20 mm, ae ≤ 20 % D) against conventional roughing.
  - Forces of up to 800 N and up to 3 kW are *simulated* values. No measured comparison and no
    dimensioned part. **Score 1.**
- **Hrabovský 2014**, Brno University of Technology bachelor thesis **[R, NUŠL record]**.
  - "Comparison of roughing strategies in various CAM software", PowerMILL vs HSMWorks.
  - Punch and die moulds, compared by machine time. Whether the times were measured on a machine
    is UNKNOWN; the record says the full text is in the BUT library but also "no fulltext".
  - **Score 1–2.**
- **Derner 2023**, CTU Prague master thesis **[R, record]**. Open access; thesis licence is personal
  use. https://dspace.cvut.cz/entities/publication/da883e8f-4021-48a0-89cb-e33fe9e060b5
  - "Advanced roughing strategies in CAM for modern milling methods". Keywords: iMachining, SolidCAM,
    Siemens NX.
  - Measured on a real part with a Spike sensor toolholder (forces/torque).
  - **Attachments: NC code, Spike data, CAD models.**
  - The PDF host digilib.cvut.cz is outside this session's allowed network (§7).
  - **Potentially score 4–5. The best lead to follow up.**

## 3. Ranking: how well forgeCAD could reproduce each comparison

The test: rebuild the part in `.fg`, generate our path, predict cycle time and the metric, and
compare with their measured numbers. Ranked by what is reachable and verified today.

| # | study | rebuild | what we can compare | score |
|---|---|---|---|---|
| 1 | Ward et al. 2021 (§2.1) | 40 × 60 × 10 open pocket (Al 7075, Ø12) yes; pockets of cases 1–2 need Table 1 / Fig. 15 | measured time vs CAM estimate across feed and tolerance: a direct test of our acceleration-aware estimate; forces on the trochoidal path | 4 |
| 2 | Bílek et al. 2025 (§2.2) | yes (76 × 76 × 40 block, 60 × 46 × 12 R4 pocket) | adaptive vs offset measured time (4:11 vs 5:27), CAM estimate error (−9 % vs 0 %), forces; our adaptive vs our offset at their ae/ap/vf | 4 |
| 3 | Shajari et al. 2014 (§2.3) | partly (curvature guessed from 0–32° slope) | relative finishing times of raster/offset/spiral/radial over 9 parameter sets; force ranking | 3 |
| 4 | Derner 2023 (§2.12) | likely yes (CAD + NC attached) | iMachining / NX adaptive NC vs ours, measured Spike load | 4–5 if reachable |
| 5 | Globocki-Lakić et al. 2022 (§2.4) | no (render only); a look-alike part only | relative rough/finish times (Contour vs Hatch −15 %; step-over vs spiral) | 2 |
| 6 | Silva et al. 2024 (§2.5) | UNKNOWN (deep slot, dims unread) | plunge vs side roughing time (−20.7 %) | 3? |
| 7 | Chuchala et al. 2021 (§2.12) | yes (60 × 200 plates) | facing pass order vs Ra; tests a facing op, not strategy choice | 4, low value |
| 8 | Amaro et al. 2020 (§2.6) | UNKNOWN | tool life at equal time; MRR (needs a wear model we lack) | ≥ 2 |
| 9 | Gómez et al. 2021 (§2.7) | UNKNOWN | slot strategies, production time | ? |
| 10 | Edem et al. 2019 / Aramcharoen 2014 (§2.9) | UNKNOWN | energy per toolpath type (needs a power model) | 1–2 |

**Practical order for forgeCAD:**
1. **Bílek pocket**: one afternoon to model; numbers in tables.
2. **Ward pockets**: cycle time against feed and tolerance, which is exactly what our planned
   estimate must get right.
3. **Shajari** finishing set, as a relative-time check.
4. Fetch **Derner**'s attachments once digilib.cvut.cz is reachable.
5. RPTU's 10 parts ([datasets.md](datasets.md)) remain the best *path-to-path* comparison, though
   they come with no measured time.

## 4. Vendor-independent benchmark efforts

| effort | what it is | public? | measured results? |
|---|---|---|---|
| CETIM HSM CAM trial (2003, France) | six HSM CAM suppliers program one demonstration part; cut on CETIM's Huron EX 14 from a common tool set; time and finish rated | trade-press report only (Machine Design, 21 Aug 2003) [R] | relative only: PowerMill fastest, others about ⅓ longer; two failed to finish programming in time; finish rated "very good" … "good to medium". Part undescribed. |
| VDI/NCG 5211 sheets 1–3 | NCG test workpieces for HSC and 5-axis *machines* and micromachining | paid standards; several sheets withdrawn or marked for withdrawal [S] | for machines, not CAM |
| ISO 10791-7, NAS 979 | machine acceptance test pieces | paywalled; RPTU part 8 is an open derivative ([datasets.md](datasets.md)) [R earlier] | no |
| RPTU 3-axis benchmark (Zenodo 17035762) | 10 parts, STEP + Autodesk-posted G-code, CC BY | yes | G-code only; no measured time or Ra |
| NIST SMS test bed | Mastercam program + MTConnect events ([datasets-gcode.md](datasets-gcode.md)) | partly (host unreachable earlier) | timing events, one CAM |
| Zaragoza master thesis TAZ-TFM-2019-051 | "benchmark for CAD/CAM systems" in toolmaking, adapted to hot forging; two systems shortlisted | open PDF, but 403 here [S] | UNKNOWN |
| Fraunhofer IPK (euspen 2020) | machine-specific CAM vs manual NC for ultra-precision turning: Ra, accuracy, time | open PDF [S] | yes, but turning, not milling |
| CIMdata | publishes vendor news, e.g. ModuleWorks dental "8 % faster milling" against one rival | vendor-sourced [S] | not independent |
| AMRC, WZL, IPT, MTA SZTAKI, JIMTOF, an "NCG CAM benchmark" | searched | **nothing public found** | — |

**Conclusion.** No standing, open, multi-CAM benchmark with measured cycle time or finish exists
that we could find. The closest are:
- RPTU: open parts plus one CAM's G-code;
- CETIM 2003: measured, but closed and undescribed;
- single-lab academic studies (§2).

forgeCAD could fill the gap by publishing a small open set (RPTU parts plus the Bílek and Ward
pockets) together with our paths, predicted times, and, when David has machine access, measured
times.

## 5. What this tells us

**Roughing time, adaptive/trochoidal/plunge vs offset/conventional**

| source | kind | saving | mark |
|---|---|---|---|
| Bílek 2025, GFRP pocket, NX | measured, independent | −23 % (251 vs 327 s) | R |
| Uhlmann 2013, Ti-6Al-4V | measured | −35 % time, +6 % mean power | R (via citing paper) |
| Silva 2024, AA7050 deep slots, plunge | measured | −20.7 % | R abstract / S |
| Globocki-Lakić 2022, Contour vs Hatch (both conventional) | measured | −15 % | R |
| Infante Castillo 2021, 7075 | measured, best case | −82.3 % | S |
| Tebis practical test, steel | vendor | > −60 % | R (vendor page) |
| Autodesk blog, 1018 steel | vendor, one test | 8:09 → 2:01 (−75 %) | S |
| Seco Tools | vendor claim | −40 to −70 % | S |

- **Independent measured studies: −20 to −35 %.** Vendor and best-case figures: −60 to −80 %.
  Read as "60–80 % against a badly set conventional baseline." The size of the gain depends mostly
  on how deep the conventional step-down was (Bílek: 1.5–3.5 mm against 12 mm full flute;
  Infante: 2.2 against 14 mm).
- **Tool life is mixed.**
  - Positive: 2× cutting speed at the same 20 min life in duplex steel (Amaro [S]); up to 600 %
    more material per unit of wear in IN738 (Pleta [R via citation]).
  - Negative: no wear or vibration benefit (Gross 2020 [R abstract]).
  - Equal: low wear for both strategies with the right tool (Bílek [R]).
  - So forgeCAD should not claim tool-life gains from geometry alone. Engagement control is a
    necessary condition, not a sufficient one.
- **Accuracy and finish trade-off.** Trochoidal and adaptive roughing gave worse dimensions and
  more burr in GFRP (Bílek). The authors propose trochoidal roughing followed by a conventional
  finish, an argument for keeping stock-to-leave and a separate finishing op as the default.
- **Finishing strategies (measured).**
  - Radial gave the lowest force and best roughness but ~3× the time on a gentle convex surface.
    Spiral was worst there (Shajari).
  - On a mixed concave/convex part, constant step-over beat linear and spiral on both time and Ra
    (Globocki-Lakić).
  - So no strategy wins everywhere. This supports surface-finish.md's per-region choice
    ([../toolpath-quality/surface-finish.md](../toolpath-quality/surface-finish.md)).
- **Cycle-time estimates.**
  - CAM estimates are close for offsets at low feed (−0.2 to −2 %).
  - They are poor for short-segment, high-feed or tight-tolerance paths: −15 % on a contour pocket
    at 3000 mm/min; −55 % on a trochoidal pocket; −36 to −62 % on a full aerostructure program
    (Ward [R]); −9 % on NX Adaptive (Bílek [R]).
  - An interpolator-aware prediction got within about 6 %.
  - This is the measurable target for forgeCAD's cycle time
    ([../toolpath-quality/feeds-speeds.md](../toolpath-quality/feeds-speeds.md)).
- **Energy.** Energy per part tracks time far more than power. Trochoidal paths raise mean power
  slightly (+6 %) and cut time more (Uhlmann). The best strategy depends on the machine (Edem
  2017 [S]).

## 6. References (open-access status and mark)

1. Ward R., Sencer B., Jones B., Ozturk E. (2021). arXiv:2102.02062; DOI
   10.1007/s00170-021-07211-2. CC BY 4.0. [R]
2. Bílek O. et al. (2025). *Materials* 18:1669. DOI 10.3390/ma18071669. CC BY. [R] (PMC11990107)
3. Shajari S., Sadeghi M.H., Hassanpour H. (2014). *Sci. World J.* 374526. DOI 10.1155/2014/374526.
   CC BY. [R]
4. Globocki-Lakić G. et al. (2022). *FME Trans.* 50:623–634. DOI 10.5937/fme2204623G. Free access.
   [R]
5. Silva F.J.G. et al. (2024). *JMMP* 8(3):115. DOI 10.3390/jmmp8030115. CC BY. [R abstract; S]
6. Amaro P., Ferreira P., Simões F. (2020). *Procedia Manuf.* 47. DOI
   10.1016/j.promfg.2020.04.132. Open access [K]. [S]
7. Gómez G. et al. (2021). *IOP Conf. Ser. MSE* 1193:012003. DOI 10.1088/1757-899X/1193/1/012003.
   Open access. [S]
8. Gross D., Friedl F., Meier T., Hanenkamp N. (2020). *Procedia CIRP* 90:563–567. DOI
   10.1016/j.procir.2020.01.069. [R abstract]
9. Aramcharoen A., Mativenga P.T. (2014). *J. Clean. Prod.* 78:63–74. DOI
   10.1016/j.jclepro.2014.04.065. Not open (no copy found). [S]
10. Edem I.F. et al. (2019). *IJAMT* 103:997–1007. DOI 10.1007/s00170-019-03604-6. Access UNKNOWN.
    [S]
11. Pervaiz S. et al. "An experimental analysis of energy consumption in milling strategies." IEEE
    conference. [S]
12. "A comparative analysis between high speed dynamic and traditional pocketing toolpaths in
    precision milling machines." ASME MSEC2016-8746, DOI 10.1115/msec2016-8746. [S]
13. Uhlmann E. et al. (2013). GCSM Berlin, pp. 593–598. [R as cited in 16; S]
14. Infante Castillo, Culqui Culqui (2021). LNEE 763. DOI 10.1007/978-3-030-72212-8_9. [S]
15. Pleta A., Ulutan D., Mears L. (2015). *Procedia Manuf.* 1:556–566. DOI
    10.1016/j.promfg.2015.09.032. [R as cited in 16]
16. Akhavan Niaki F., Pleta A., Mears L. (2018). *IJAMT*, DOI 10.1007/s00170-018-1967-0. NSF PAR
    accepted copy. [R]
17. Pleta A. et al. (2019). *J. Manuf. Process.* 43:54–62. NSF PAR copy. [R] (not head-to-head)
18. Wagih M. et al. (2023). *Proc. IMechE C*, DOI 10.1177/09544062231196991. Author-page copy. [R
    abstract] (not head-to-head)
19. Liu D. et al. (2019). *Materials* 12:1937. DOI 10.3390/ma12121937. CC BY. [R] (no baseline)
20. Chuchala D. et al. (2021). *Materials* 14:3036. DOI 10.3390/ma14113036. CC BY. [R]
21. Mikó B., Varga B., Zębala W. (2019). *Materials* 12:4038. DOI 10.3390/ma12244038. CC BY. [R
    abstract]
22. Bagci E., Yüncüoğlu E. (2017). *Stroj. vestn.* 63(11):643–656. [R abstract]
23. But A., Popescu C.-E. (2025). *Acta Tech. Napocensis* 68(Spec. I). [R]
24. Hrabovský A. (2014). BUT bachelor thesis, hdl 11012/31964. [R record]
25. Derner T. (2023). CTU master thesis, hdl 10467/111539. [R record]
26. Korane K. "Study rates CAM systems," *Machine Design*, 21 Aug 2003 (CETIM). [R]
27. Tebis, "Practical test of run time calculation" (HPC milling page). Vendor. [R]
28. Autodesk Fusion blog, "Speed up your milling with constant cutter forces." Vendor. [S]
29. Seco Tools, "Make milling dynamic…" Vendor. [S]
30. Salehi M. et al. (2016). *Procedia CIRP* 46:230–233 (epicycloidal vs trochoidal: about −20 %
    time, +10 % force and vibration). [S; one secondary source says "+20 %"]
31. VDI/NCG 5211 Blatt 1–3 (record pages). [S]

## 7. Unreachable or not read (record, do not retry blindly)

- **mdpi.com** (JMMP 8:115; JMMP 9:171; Materials full pages) and **preprints.org**: HTTP 403.
  Workaround used: Europe PMC REST and Crossref API, both of which are documented public APIs.
- **sciencedirect.com** (Gross 2020; Procedia papers) and **iopscience.iop.org**: 403.
  **link.springer.com**: redirected to a login.
- Bot walls, **not bypassed**:
  - machinery.mas.bg.ac.rs (human verification; Serbian "Analysis of methods for tool path
    generation using commercial CAM systems");
  - comum.rcaap.pt (Anubis proof-of-work; Amaro);
  - pmc.ncbi.nlm.nih.gov (browser check, late in the session).
- Not reachable, other causes:
  - estudogeral.uc.pt: redirect loop.
  - zaguan.unizar.es, core.ac.uk, essuir.sumdu.edu.ua (Dodok et al., Edgecam 2017 strategies),
    yadda.icm.edu.pl (Patil 2014, Waszczuk 2020), doaj.org: 403.
  - dspace.vsb.cz: 429. This blocked a thesis comparing CAM-computed with machine-measured times.
  - publica.fraunhofer.de: 503.
  - gala.gre.ac.uk: PDF over 10 MB.
- **digilib.cvut.cz**: outside this session's allowed network. It must be allowed before Derner
  2023's PDF and attachments can be fetched.
- Europe PMC full text for Mikó 2019: HTTP 500.
- Not found at all: an NCG, VDMA, CIMdata or JIMTOF *CAM* benchmark with published measurements;
  an AMRC, WZL or IPT multi-CAM study.
