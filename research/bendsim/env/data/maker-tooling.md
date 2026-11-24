# Press-brake tooling from openly published catalogues: WILA, Wilson Tool, Rolleri, Mate

Collected 2026-11-24 for the bendsim press-brake simulator. Every value carries the URL it was read from, the
date read (all sources read on 2026-11-24), and the source's own statement with units. Numbers were
transcribed from catalogue tables and from the dimensioned drawings in the catalogues (drawing dimensions
were read off rendered page images); nothing is guessed. `UNKNOWN` = not published in the source read.
`estimate` = derived, with the basis. Catalogue page numbers are the printed page numbers (equal to the PDF
page index for these PDFs).

Conventions used in the field names:
- `height_mm`: the maker's tool height. WILA H and Wilson H run from the tip to the tool shoulder that
  bears on the holder (WILA: section titles 'H=98/120/163/200'); Rolleri 'Working height' and Mate 'H'
  likewise exclude the tang; Rolleri drawings also give the overall height (`total_height_mm`).
- `rear_extent_from_tip_axis_mm` (WILA) and `width_behind_tip_mm` (Rolleri/Mate/Wilson American W): the
  horizontal dimension from the tip axis to the rear face of the body; for a gooseneck it is the full depth
  behind the tip line, not the throat. A throat (relief) depth is recorded only where dimensioned.
- `tip_flank_oblique_dim_mm`: the oblique dimension every maker draws at the tip; its meaning is not stated.
- Max load: kept in the source's unit (t/m, kN/m, or Wilson 'Max. Ton feet [m]' bracket value = t/m).
- `kind` is our classification from the drawing silhouette unless the maker's series name states it.

## 1. Sources

| # | URL | what | date read | result |
|---|-----|------|-----------|--------|
| 1 | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf | WILA catalogue 2024 USA (144 pp; NS Premium, NS Pro, American Style, American Vintage top & bottom tools) | 2026-11-24 | fetched (PDF, 16.5 MB, no login) |
| 2 | https://wilsontool.com/en-us/resources/european-style-press-brake-tooling-catalog | Wilson Tool resource page that links the European catalogue PDF | 2026-11-24 | fetched |
| 3 | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf | Wilson Tool European-Style Press Brake Tooling catalogue B374C, Sept 2025 (150 pp; Fixed Height, European Traditional, Staged, air bending force chart) | 2026-11-24 | fetched (PDF, 12.4 MB, no login) |
| 4 | https://wilsontool.com/en-us/resources/american-precision-press-brake-tooling-catalog | Wilson Tool resource page linking the American Precision PDF | 2026-11-24 | fetched |
| 5 | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf | Wilson Tool American Precision Press Brake Tooling catalogue B350C, Oct 2025 (84 pp) | 2026-11-24 | fetched (PDF, 6.2 MB, no login) |
| 6 | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf | Mate European Precision Style press brake tooling catalogue LIT01119 Rev C (2015, 56 pp) | 2026-11-24 | fetched (PDF, 7.3 MB, no login); 2015 edition |
| 7 | https://www.rolleritools.com/information/catalogues-138942/bending-catalogue-2024.html | Rolleri Bending Catalogue 2024 PDF | 2026-11-24 | REFUSED by rule: download is behind a form asking Company / Name / e-mail; not submitted |
| 8 | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Rolleri online catalogue, type R1 (European/Promecam) punches: 126 products with working height, angle, radius, material, max load | 2026-11-24 | fetched (public HTML) |
| 9 | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Rolleri online catalogue, type R1 dies: holders, 2-V, 1-V, T dies, multi-V | 2026-11-24 | fetched (public HTML) |
| 10 | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/<product>.html (and .../type-r1-dies/<product>.html) | Rolleri product detail pages (article no., length, weight); each block cites its own page | 2026-11-24 | fetched |
| 11 | https://www.rolleritools.com/upload/immagini/img1200/<file>.jpg | Rolleri product drawings (dimensioned JPEG profiles) linked from the detail pages; each block cites its file | 2026-11-24 | fetched |
| 12 | https://rolleri.com/ | unrelated company (landscape supplies), not the tooling maker | 2026-11-24 | fetched; irrelevant |

## 2. Clamping systems, V-selection rules and tonnage charts

```
[[clamping]]
id = "wila/ns-bottom-adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.41 fact table) | Bottom (OZU/DDU/HU/THU) have 13 mm adaption designed for use on all press brakes with a Clamping Slot 13.5x 22 (WxH)
die_tang_width_mm = 13   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.41) | 13 mm adaption
die_clamping_slot_mm = "13.5 W x 22 H"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.41) | Clamping Slot 13.5x 22 (WxH)
die_tang_width_mm_wrb = 13   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.71 Wing Bender table, OZU-WRB-301..303) | Bottom adaption width 13 mm / 0.512"
top_tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf | NS I head load / NS II, V, VI shoulder load and AS I/II adaptions are drawn without dimensions
top_tool_safety = "Safety-Click up to 12.5 kg; heavier tools Safety-Pins or E2M"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.46, p.143) | Safety-Click ... top tools with a weight up to 12.5 kg/28 lbs

[[clamping]]
id = "mate/european-precision-tang"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6 SAFETY GROOVE DATA) | dimensioned tang detail
tang_width_mm = 13   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 13
tang_height_mm = 30   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 30 (top of tang to shoulder)
tang_front_face_to_tool_axis_mm = 7   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 7 (from tang face to dashed tool axis)
safety_groove_depth_mm = 3   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 3
safety_groove_height_mm = 8.4   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 8.4
safety_groove_top_above_shoulder_mm = 13   # published (as drawn; arrow from shoulder line to groove top) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 13

[[clamping]]
id = "wilson/european-traditional-tang"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (pp.36-46 drawings) | 1.101 [27.97] on every European Traditional punch drawing
tang_height_mm = 27.97   # conflict (Mate p.6 gives 30; Rolleri drawings give overall minus working height = 30.0..30.3); Wilson's dimension may end at a different datum | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | 1.101 [27.97]
tang_variants = "straight tang (part no.) and beveled tang (part no. + B)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.33) | All punches are available with a straight or beveled tang
die_foot_width_mm_1V_quick_change = 14   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | .551 [14] at die foot
die_2V_groove = "width 15.1 mm, depth 6.5 mm; V centres 13 mm each side of die centreline"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.74) | .593 [15.1], .256 [6.5], .512 [13] .512 [13]
fixed_height_die_foot_width_mm = "14 or 15 (style dependent)"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.12) | .551 [14] / .591 [15]
american_tang = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf | American Precision drawings show no tang dimension; 'button'/'groove' safety variants only

[[clamping]]
id = "rolleri/r1"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Tools R1 Type Promecam | European Style
tang_height_mm = 30   # estimate (basis: overall height minus working height on Rolleri drawings: 130-100, 120-90, 150-120, 175-145; P.97.88.R08 gives 96.95-66.65 = 30.3) | https://www.rolleritools.com/upload/immagini/img1200/P.130-88-R08_P.130-88-R025-01.jpg | 130/5.12" and 100/3.94"
tang_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | not dimensioned on the drawings read

```

V-selection rules and tonnage charts:

```
[[rule]]
id = "wilson/air-bending-force-chart"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.141) | AIR BENDING FORCE CHART
v_selection = "T <= 2.6 mm: V = 6T; 3.00-8.0 mm: V = 8T; 9.00-12.00 mm: V = 10T; >= 14.00 mm: V = 12T"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.141) | Material Thickness: 2.6mm or Less = T x 6; 3.00mm - 8.0mm = T x 8; 9.00mm - 12.00mm = T x 10; 14.00mm & Thicker = T x 12
v_columns_mm = "4 6 7 8 10 12 14 16 18 20 25 32 40 50 63 80 100 125"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.141) | V (mm) row
min_flange_in = "0.110 0.165 0.193 0.220 0.276 0.331 0.397 0.454 0.510 0.567 0.709 0.945 1.181 1.476 1.860 2.362 2.953 3.789"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.141) | MF row (inch), per V column
inside_radius_in = "0.026 0.039 0.046 0.052 0.066 0.079 0.092 0.105 0.118 0.131 0.164 0.210 0.262 0.328 0.413 0.525 0.656 0.820"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.141) | IR row (inch), per V column
basis = "mild steel 60,000 psi, 88 deg included angle; tons per foot"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.141) | based on mild steel (tensile strength of 60,000 PSI) formed to an included angle of 88
material_factors = "soft Al/brass/Cu x0.35; half-hard Al x0.5; hard Al, mild steel x1.0; half-hard brass/Cu x1.1; stainless x1.5; high strength steel x2.75"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.141) | Tons x 0.35 ... Tons x 2.75
tonnage_cells = "UNKNOWN"   # unknown (not transcribed: the text layer does not keep cells aligned to V columns; read the page image before use) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.141) | grid of tons/ft by gauge

[[rule]]
id = "mate/air-bending-chart"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.4) | METRIC AIR BENDING CHART, FORCE REQUIRED FOR 90 deg AIR BENDING, SHEET RESISTANCE R=45 kg/mm2
v_columns_mm = "6 8 10 12 16 20 25 32 40 50 63 80 100 125 160 200 250"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.4) | V row
min_flange_b_mm = "4 5.5 7 8.5 11 14 17.5 22 28 35 45 55 71 89 113 140 175"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.4) | b row
inside_radius_ir_mm = "1 1.3 1.6 2 2.6 3.3 4 5 6.5 8 10 13 16 20 26 33 41"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.4) | ir row
v_selection = "sheet 0.5-2.5 mm: 6T; 2.5-8: 8T; 8-10: 10T; 10 or more: 12T"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.4) | SHEET THICKNESS (mm) 0.5 - 2.5 / 2.5 - 8 / 8 - 10 / 10 OR MORE; V - width 6T 8T 10T 12T
bending_types = "air bending V 12T-15T, ir 2T-2.5T, springback about +/-45'; bottoming V 6T-12T, ir 1T-2T, +/-30'; coining V 5T, ir 0.5T-0.8T, +/-15'"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.4) | TYPE OF BENDING table
sheet_resistance = "Al 20-25, mild steel 40-45, stainless 65-70 kg/mm2"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.4) | R=20-25 / 40-45 / 65-70 kg/mm2
tonnage_cells = "UNKNOWN"   # unknown (not transcribed: text layer misaligns cells) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.4) | Force in ton/meter grid
force_formula = "UNKNOWN"   # unknown (printed formula not resolved from the text layer; not transcribed) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.4) | CALCULATION OF FORCE FOR AIR BENDING

[[rule]]
id = "wila/tonnage"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf | no tonnage chart or V-selection rule found in the 2024 USA catalogue; dies carry a gauge label per V instead (e.g. V=6 mm '22 Ga', V=24 mm '11 Ga', V=40 mm '3/16"', V=50 mm '1/4"')

```

## 3. Tools

### 3.1 WILA (catalogue 2024 USA)

```
[[punch]]
id = "wila/BIU-001"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | type BIU-001
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | WILA catalogue USA 2024
part_number = "BIU-001"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | BIU-001/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | adaption figure on the same spread: 'NS I head load'
kind = "straight-acute"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | drawing only; the catalogue does not name the kind
height_mm = 98.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | vertical dim on drawing 98 mm (tip to tool shoulder); section title H=98
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | R1 mm
tip_angle_deg = 28.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | alpha=28 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 22.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | oblique dim at tip 22 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | segmented tooling figure
weight_515mm_kg = 7.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | 7.5 kg for 515 mm

[[punch]]
id = "wila/BIU-004"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | type BIU-004
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | WILA catalogue USA 2024
part_number = "BIU-004"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | BIU-004/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | drawing only; the catalogue does not name the kind
height_mm = 98.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | vertical dim on drawing 98 mm (tip to tool shoulder); section title H=98
tip_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | R3 mm
tip_angle_deg = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | alpha=60 deg
max_load_t_per_m = 160.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | Max load 160 t/m (54 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | segmented tooling figure
weight_515mm_kg = 10.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | 10.6 kg for 515 mm

[[punch]]
id = "wila/BIU-002"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | type BIU-002
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | WILA catalogue USA 2024
part_number = "BIU-002"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | BIU-002/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | drawing only; the catalogue does not name the kind
height_mm = 98.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | vertical dim on drawing 98 mm (tip to tool shoulder); section title H=98
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | R1 mm
tip_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | alpha=80 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 25.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | horizontal dim 25 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 10.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | oblique dim at tip 10 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | segmented tooling figure
weight_515mm_kg = 9.1   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | 9.1 kg for 515 mm

[[punch]]
id = "wila/BIU-003"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | type BIU-003
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | WILA catalogue USA 2024
part_number = "BIU-003"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | BIU-003/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | adaption figure on the same spread: 'NS I head load'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | drawing only; the catalogue does not name the kind
height_mm = 98.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | vertical dim on drawing 98 mm (tip to tool shoulder); section title H=98
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | R1 mm
tip_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | alpha=86 deg
max_load_t_per_m = 65.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | Max load 65 t/m (22 t/ft)
rear_extent_from_tip_axis_mm = 50.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | horizontal dim 50 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 8.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | oblique dim at tip 8 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | segmented tooling figure
weight_515mm_kg = 12.4   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.44) | 12.4 kg for 515 mm

[[punch]]
id = "wila/BIU-011"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | type BIU-011
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | WILA catalogue USA 2024
part_number = "BIU-011"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | BIU-011/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | adaption figure on the same spread: 'NS I head load'
kind = "straight-acute"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | drawing only; the catalogue does not name the kind
height_mm = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | vertical dim on drawing 120 mm (tip to tool shoulder); section title H=120
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | R1 mm
tip_angle_deg = 28.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | alpha=28 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 24.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | horizontal dim 24 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 22.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | oblique dim at tip 22 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | segmented tooling figure
weight_515mm_kg = 9.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | 9.5 kg for 515 mm

[[punch]]
id = "wila/BIU-014"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | type BIU-014
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | WILA catalogue USA 2024
part_number = "BIU-014"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | BIU-014/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | drawing only; the catalogue does not name the kind
height_mm = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | vertical dim on drawing 120 mm (tip to tool shoulder); section title H=120
tip_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | R3 mm
tip_angle_deg = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | alpha=60 deg
max_load_t_per_m = 160.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | Max load 160 t/m (54 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | segmented tooling figure
weight_515mm_kg = 12.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | 12.2 kg for 515 mm

[[punch]]
id = "wila/BIU-012"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | type BIU-012
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | WILA catalogue USA 2024
part_number = "BIU-012"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | BIU-012/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | drawing only; the catalogue does not name the kind
height_mm = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | vertical dim on drawing 120 mm (tip to tool shoulder); section title H=120
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | R1 mm
tip_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | alpha=80 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 25.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | horizontal dim 25 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 10.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | oblique dim at tip 10 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | segmented tooling figure
weight_515mm_kg = 9.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | 9.8 kg for 515 mm

[[punch]]
id = "wila/BIU-013"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | type BIU-013
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | WILA catalogue USA 2024
part_number = "BIU-013"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | BIU-013/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | adaption figure on the same spread: 'NS I head load'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | drawing only; the catalogue does not name the kind
height_mm = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | vertical dim on drawing 120 mm (tip to tool shoulder); section title H=120
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | R1 mm
tip_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | alpha=86 deg
max_load_t_per_m = 65.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | Max load 65 t/m (22 t/ft)
rear_extent_from_tip_axis_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | horizontal dim 55 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 8.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | oblique dim at tip 8 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | segmented tooling figure
weight_515mm_kg = 15.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.45) | 15.5 kg for 515 mm

[[punch]]
id = "wila/BIU-015"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | type BIU-015
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | WILA catalogue USA 2024
part_number = "BIU-015"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | BIU-015/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | drawing only; the catalogue does not name the kind
height_mm = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | vertical dim on drawing 120 mm (tip to tool shoulder); section title H=120
tip_radius_mm = 0.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | R0.6 mm
tip_angle_deg = 90.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | alpha=90 deg
max_load_t_per_m = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | Max load 60 t/m (20 t/ft)
rear_extent_from_tip_axis_mm = 25.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | horizontal dim 25 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 8.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | oblique dim at tip 8 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | segmented tooling figure
weight_515mm_kg = 11.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | 11.0 kg for 515 mm

[[punch]]
id = "wila/BIU-016"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | type BIU-016
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | WILA catalogue USA 2024
part_number = "BIU-016"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | BIU-016/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | drawing only; the catalogue does not name the kind
height_mm = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | vertical dim on drawing 120 mm (tip to tool shoulder); section title H=120
tip_radius_mm = 1.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | R1.2 mm
tip_angle_deg = 90.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | alpha=90 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | segmented tooling figure
weight_515mm_kg = 12.4   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.46) | 12.4 kg for 515 mm

[[punch]]
id = "wila/BIU-021"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | type BIU-021
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | WILA catalogue USA 2024
part_number = "BIU-021"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | BIU-021/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | adaption figure on the same spread: 'NS I head load'
kind = "straight-acute"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | R1 mm
tip_angle_deg = 28.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | alpha=28 deg
max_load_t_per_m = 70.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | Max load 70 t/m (24 t/ft)
rear_extent_from_tip_axis_mm = 24.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | horizontal dim 24 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 22.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | oblique dim at tip 22 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | segmented tooling figure
weight_515mm_kg = 12.4   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | 12.4 kg for 515 mm

[[punch]]
id = "wila/BIU-221"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | type BIU-221
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | WILA catalogue USA 2024
part_number = "BIU-221"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | BIU-221/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | adaption figure on the same spread: 'NS I head load'
kind = "straight-acute"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | R3 mm
tip_angle_deg = 28.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | alpha=28 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 24.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | horizontal dim 24 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 22.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | oblique dim at tip 22 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | segmented tooling figure
weight_515mm_kg = 12.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | 12.5 kg for 515 mm

[[punch]]
id = "wila/BIU-029"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | type BIU-029
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | WILA catalogue USA 2024
part_number = "BIU-029"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | BIU-029/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | adaption figure on the same spread: 'NS I head load'
kind = "straight-acute"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | R3 mm
tip_angle_deg = 28.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | alpha=28 deg
max_load_t_per_m = 160.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | Max load 160 t/m (54 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | segmented tooling figure
weight_515mm_kg = 15.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | 15.6 kg for 515 mm

[[punch]]
id = "wila/BIU-024"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | type BIU-024
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | WILA catalogue USA 2024
part_number = "BIU-024"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | BIU-024/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | R3 mm
tip_angle_deg = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | alpha=60 deg
max_load_t_per_m = 160.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | Max load 160 t/m (54 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | segmented tooling figure
weight_515mm_kg = 16.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.47) | 16.0 kg for 515 mm

[[punch]]
id = "wila/BIU-222"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | type BIU-222
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | WILA catalogue USA 2024
part_number = "BIU-222"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | BIU-222/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 6.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | R6 mm
tip_angle_deg = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | alpha=60 deg
max_load_t_per_m = 160.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | Max load 160 t/m (54 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | segmented tooling figure
weight_515mm_kg = 15.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | 15.8 kg for 515 mm

[[punch]]
id = "wila/BIU-137"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | type BIU-137
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | WILA catalogue USA 2024
part_number = "BIU-137"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | BIU-137/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | adaption figure on the same spread: 'NS I head load'
kind = "straight-radius"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 9.53   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | R9.53 mm
tip_angle_deg = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | alpha=60 deg
max_load_t_per_m = 190.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | Max load 190 t/m (63 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | segmented tooling figure
weight_515mm_kg = 16.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | 16.2 kg for 515 mm

[[punch]]
id = "wila/BIU-223"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | type BIU-223
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | WILA catalogue USA 2024
part_number = "BIU-223"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | BIU-223/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | adaption figure on the same spread: 'NS I head load'
kind = "straight-radius"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 12.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | R12.5 mm
tip_angle_deg = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | alpha=60 deg
max_load_t_per_m = 225.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | Max load 225 t/m (76 t/ft)
rear_extent_from_tip_axis_mm = 25.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | horizontal dim 25 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | segmented tooling figure
weight_515mm_kg = 18.9   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | 18.9 kg for 515 mm

[[punch]]
id = "wila/BIU-226"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | type BIU-226
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | WILA catalogue USA 2024
part_number = "BIU-226"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | BIU-226/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | adaption figure on the same spread: 'NS I head load'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | R1 mm
tip_angle_deg = 45.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | alpha=45 deg
max_load_t_per_m = 40.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | Max load 40 t/m (13 t/ft)
rear_extent_from_tip_axis_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | horizontal dim 55 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 8.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | oblique dim at tip 8 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | segmented tooling figure
weight_515mm_kg = 15.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.48) | 15.2 kg for 515 mm

[[punch]]
id = "wila/BIU-022"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | type BIU-022
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | WILA catalogue USA 2024
part_number = "BIU-022"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | BIU-022/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | R1 mm
tip_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | alpha=80 deg
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | Max load 80 t/m (27 t/ft)
rear_extent_from_tip_axis_mm = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | horizontal dim 30 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 10.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | oblique dim at tip 10 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | segmented tooling figure
weight_515mm_kg = 16.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | 16.2 kg for 515 mm

[[punch]]
id = "wila/BIU-026"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | type BIU-026
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | WILA catalogue USA 2024
part_number = "BIU-026"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | BIU-026/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | R3 mm
tip_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | alpha=80 deg
max_load_t_per_m = 140.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | Max load 140 t/m (47 t/ft)
rear_extent_from_tip_axis_mm = 35.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | horizontal dim 35 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 26.8   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | oblique dim at tip 26.8 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | segmented tooling figure
weight_515mm_kg = 19.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | 19.6 kg for 515 mm

[[punch]]
id = "wila/BIU-225"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | type BIU-225
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | WILA catalogue USA 2024
part_number = "BIU-225"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | BIU-225/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | adaption figure on the same spread: 'NS I head load'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | R3 mm
tip_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | alpha=80 deg
max_load_t_per_m = 90.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | Max load 90 t/m (30 t/ft)
rear_extent_from_tip_axis_mm = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | horizontal dim 60 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 10.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | oblique dim at tip 10 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | segmented tooling figure
weight_515mm_kg = 24.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | 24.6 kg for 515 mm

[[punch]]
id = "wila/BIU-025"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | type BIU-025
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | WILA catalogue USA 2024
part_number = "BIU-025"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | BIU-025/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | adaption figure on the same spread: 'NS I head load'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 6.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | R6 mm
tip_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | alpha=80 deg
max_load_t_per_m = 130.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | Max load 130 t/m (44 t/ft)
rear_extent_from_tip_axis_mm = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | horizontal dim 60 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 18.6   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | oblique dim at tip 18.6 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | segmented tooling figure
weight_515mm_kg = 27.7   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.49) | 27.7 kg for 515 mm

[[punch]]
id = "wila/BIU-224"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | type BIU-224
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | WILA catalogue USA 2024
part_number = "BIU-224"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | BIU-224/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 0.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | R0.6 mm
tip_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | alpha=86 deg
max_load_t_per_m = 45.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | Max load 45 t/m (15 t/ft)
rear_extent_from_tip_axis_mm = 25.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | horizontal dim 25 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 6.5   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | oblique dim at tip 6.5 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | segmented tooling figure
weight_515mm_kg = 12.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | 12.2 kg for 515 mm

[[punch]]
id = "wila/BIU-023"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | type BIU-023
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | WILA catalogue USA 2024
part_number = "BIU-023"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | BIU-023/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | adaption figure on the same spread: 'NS I head load'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | drawing only; the catalogue does not name the kind
height_mm = 163.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | vertical dim on drawing 163 mm (tip to tool shoulder); section title H=163
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | R1 mm
tip_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | alpha=86 deg
max_load_t_per_m = 65.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | Max load 65 t/m (22 t/ft)
rear_extent_from_tip_axis_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | horizontal dim 55 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 8.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | oblique dim at tip 8 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | segmented tooling figure
weight_515mm_kg = 19.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.50) | 19.6 kg for 515 mm

[[punch]]
id = "wila/BIU-031"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | type BIU-031
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | WILA catalogue USA 2024
part_number = "BIU-031"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | BIU-031/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | adaption figure on the same spread: 'NS I head load'
kind = "straight-acute"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | R1 mm
tip_angle_deg = 28.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | alpha=28 deg
max_load_t_per_m = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | Max load 60 t/m (20 t/ft)
rear_extent_from_tip_axis_mm = 24.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | horizontal dim 24 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 22.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | oblique dim at tip 22 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | segmented tooling figure
weight_515mm_kg = 15.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | 15.0 kg for 515 mm

[[punch]]
id = "wila/BIU-231"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | type BIU-231
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | WILA catalogue USA 2024
part_number = "BIU-231"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | BIU-231/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | adaption figure on the same spread: 'NS I head load'
kind = "straight-acute"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | R3 mm
tip_angle_deg = 28.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | alpha=28 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 24.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | horizontal dim 24 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 22.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | oblique dim at tip 22 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | segmented tooling figure
weight_515mm_kg = 15.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | 15.2 kg for 515 mm

[[punch]]
id = "wila/BIU-034"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | type BIU-034
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | WILA catalogue USA 2024
part_number = "BIU-034"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | BIU-034/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | R3 mm
tip_angle_deg = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | alpha=60 deg
max_load_t_per_m = 160.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | Max load 160 t/m (54 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | segmented tooling figure
weight_515mm_kg = 18.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | 18.8 kg for 515 mm

[[punch]]
id = "wila/BIU-037"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | type BIU-037
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | WILA catalogue USA 2024
part_number = "BIU-037"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | BIU-037/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 6.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | R6 mm
tip_angle_deg = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | alpha=60 deg
max_load_t_per_m = 160.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | Max load 160 t/m (54 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | segmented tooling figure
weight_515mm_kg = 18.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.51) | 18.8 kg for 515 mm

[[punch]]
id = "wila/BIU-237"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | type BIU-237
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | WILA catalogue USA 2024
part_number = "BIU-237"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | BIU-237/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | adaption figure on the same spread: 'NS I head load'
kind = "straight-radius"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 9.53   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | R9.53 mm
tip_angle_deg = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | alpha=60 deg
max_load_t_per_m = 190.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | Max load 190 t/m (63 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | segmented tooling figure
weight_515mm_kg = 19.3   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | 19.3 kg for 515 mm

[[punch]]
id = "wila/BIU-038"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | type BIU-038
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | WILA catalogue USA 2024
part_number = "BIU-038"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | BIU-038/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | adaption figure on the same spread: 'NS I head load'
kind = "straight-radius"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 12.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | R12.5 mm
tip_angle_deg = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | alpha=60 deg
max_load_t_per_m = 225.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | Max load 225 t/m (76 t/ft)
rear_extent_from_tip_axis_mm = 25.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | horizontal dim 25 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | segmented tooling figure
weight_515mm_kg = 22.7   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | 22.7 kg for 515 mm

[[punch]]
id = "wila/BIU-032"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | type BIU-032
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | WILA catalogue USA 2024
part_number = "BIU-032"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | BIU-032/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | R1 mm
tip_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | alpha=80 deg
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | Max load 80 t/m (27 t/ft)
rear_extent_from_tip_axis_mm = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | horizontal dim 30 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 10.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | oblique dim at tip 10 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | segmented tooling figure
weight_515mm_kg = 19.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | 19.8 kg for 515 mm

[[punch]]
id = "wila/BIU-233"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | type BIU-233
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | WILA catalogue USA 2024
part_number = "BIU-233"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | BIU-233/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | adaption figure on the same spread: 'NS I head load'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | R3 mm
tip_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | alpha=80 deg
max_load_t_per_m = 90.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | Max load 90 t/m (30 t/ft)
rear_extent_from_tip_axis_mm = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | horizontal dim 60 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 10.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | oblique dim at tip 10 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | segmented tooling figure
weight_515mm_kg = 28.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.52) | 28.8 kg for 515 mm

[[punch]]
id = "wila/BIU-235"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | type BIU-235
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | WILA catalogue USA 2024
part_number = "BIU-235"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | BIU-235/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | adaption figure on the same spread: 'NS I head load'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 6.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | R6 mm
tip_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | alpha=80 deg
max_load_t_per_m = 130.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | Max load 130 t/m (44 t/ft)
rear_extent_from_tip_axis_mm = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | horizontal dim 60 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 18.6   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | oblique dim at tip 18.6 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | segmented tooling figure
weight_515mm_kg = 32.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | 32.2 kg for 515 mm

[[punch]]
id = "wila/BIU-033"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | type BIU-033
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | WILA catalogue USA 2024
part_number = "BIU-033"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | BIU-033/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | adaption figure on the same spread: 'NS I head load'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | R1 mm
tip_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | alpha=86 deg
max_load_t_per_m = 50.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | Max load 50 t/m (17 t/ft)
rear_extent_from_tip_axis_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | horizontal dim 55 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 8.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | oblique dim at tip 8 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | segmented tooling figure
weight_515mm_kg = 23.1   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | 23.1 kg for 515 mm

[[punch]]
id = "wila/BIU-035"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | type BIU-035
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | WILA catalogue USA 2024
part_number = "BIU-035"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | BIU-035/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 0.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | R0.6 mm
tip_angle_deg = 90.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | alpha=90 deg
max_load_t_per_m = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | Max load 60 t/m (20 t/ft)
rear_extent_from_tip_axis_mm = 25.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | horizontal dim 25 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 8.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | oblique dim at tip 8(partly cut off) mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | segmented tooling figure
weight_515mm_kg = 17.4   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | 17.4 kg for 515 mm

[[punch]]
id = "wila/BIU-036"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | type BIU-036
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | WILA catalogue USA 2024
part_number = "BIU-036"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | BIU-036/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Premium top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | section 'TOP TOOLS' / 'New Standard Premium TOOLING'
clamping = "WILA New Standard, NS I head load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | adaption figure on the same spread: 'NS I head load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | drawing only; the catalogue does not name the kind
height_mm = 200.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | vertical dim on drawing 200 mm (tip to tool shoulder); section title H=200
tip_radius_mm = 1.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | R1.2 mm
tip_angle_deg = 90.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | alpha=90 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | segmented tooling figure
weight_515mm_kg = 19.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.53) | 19.0 kg for 515 mm

[[punch]]
id = "wila/BIU-401"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | type BIU-401
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | WILA catalogue USA 2024
part_number = "BIU-401"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | BIU-401/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Pro top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | section 'TOP TOOLS' / 'New Standard Pro TOOLING'
clamping = "WILA New Standard (NS) top tool; adaption variant not shown on p.73"   # unknown (variant) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | section header 'New Standard Pro TOOLING'
kind = "straight-acute"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | drawing only; the catalogue does not name the kind
height_mm = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | vertical dim on drawing 100 mm (tip to tool shoulder); section title H=100
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | R1 mm
tip_angle_deg = 28.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | alpha=28 deg
max_load_t_per_m = 65.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | Max load 65 t/m (22 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 22.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | oblique dim at tip 22 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | adaption drawn schematically without dimensions
lengths = "515 mm; 475 mm sectioned; 200 mm sectioned"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | Length column of the table
sectioning = "475 mm set drawn as 100+125+20+25+30+35+40+100 mm (figure p.73); 200 mm set = 100+100"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | segmented tooling figure
weight_515mm_kg = 7.3   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | 7.3 kg for 515 mm

[[punch]]
id = "wila/BIU-403"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | type BIU-403
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | WILA catalogue USA 2024
part_number = "BIU-403"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | BIU-403/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Pro top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | section 'TOP TOOLS' / 'New Standard Pro TOOLING'
clamping = "WILA New Standard (NS) top tool; adaption variant not shown on p.73"   # unknown (variant) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | section header 'New Standard Pro TOOLING'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | drawing only; the catalogue does not name the kind
height_mm = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | vertical dim on drawing 100 mm (tip to tool shoulder); section title H=100
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | R1 mm
tip_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | alpha=86 deg
max_load_t_per_m = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | Max load 60 t/m (20 t/ft)
rear_extent_from_tip_axis_mm = 50.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | horizontal dim 50 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 8.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | oblique dim at tip 8 mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | adaption drawn schematically without dimensions
lengths = "515 mm; 475 mm sectioned; 200 mm sectioned"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | Length column of the table
sectioning = "475 mm set drawn as 100+125+20+25+30+35+40+100 mm (figure p.73); 200 mm set = 100+100"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | segmented tooling figure
weight_515mm_kg = 12.3   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | 12.3 kg for 515 mm

[[punch]]
id = "wila/BIU-404"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | type BIU-404
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | WILA catalogue USA 2024
part_number = "BIU-404"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | BIU-404/1 = 515 mm variant; other suffixes = other lengths
name = "New Standard Pro top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | section 'TOP TOOLS' / 'New Standard Pro TOOLING'
clamping = "WILA New Standard (NS) top tool; adaption variant not shown on p.73"   # unknown (variant) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | section header 'New Standard Pro TOOLING'
kind = "straight-acute"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | drawing only; the catalogue does not name the kind
height_mm = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | vertical dim on drawing 100 mm (tip to tool shoulder); section title H=100
tip_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | R3 mm
tip_angle_deg = 28.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | alpha=28 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | horizontal dim 20 mm drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | adaption drawn schematically without dimensions
lengths = "515 mm; 475 mm sectioned; 200 mm sectioned"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | Length column of the table
sectioning = "475 mm set drawn as 100+125+20+25+30+35+40+100 mm (figure p.73); 200 mm set = 100+100"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | segmented tooling figure
weight_515mm_kg = 10.1   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.73) | 10.1 kg for 515 mm

[[punch]]
id = "wila/BIU-805"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | type BIU-805
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | WILA catalogue USA 2024
part_number = "BIU-805"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | BIU-805/1 = 515 mm variant; other suffixes = other lengths
name = "American Style top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | section 'TOP TOOLS' / 'American Style TOOLING'
clamping = "WILA American Style top tool, AS II shoulder load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | adaption figure p.76: 'AS II shoulder load'
kind = "straight-acute"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | drawing only; the catalogue does not name the kind
height_mm = 98.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | vertical dim on drawing 98.6(3.882") (tip to tool shoulder)
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | R1(0.039")
tip_angle_deg = 28.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | alpha=28 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | horizontal dim 30(1.181") drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 22.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | oblique dim at tip 22(0.866") mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns (no 255 mm listed)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | segmented tooling figure
weight_515mm_kg = 6.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | 6.5 kg for 515 mm

[[punch]]
id = "wila/BIU-808"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | type BIU-808
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | WILA catalogue USA 2024
part_number = "BIU-808"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | BIU-808/1 = 515 mm variant; other suffixes = other lengths
name = "American Style top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | section 'TOP TOOLS' / 'American Style TOOLING'
clamping = "WILA American Style top tool, AS II shoulder load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | adaption figure p.76: 'AS II shoulder load'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | drawing only; the catalogue does not name the kind
height_mm = 100.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | vertical dim on drawing 100.6(3.961") (tip to tool shoulder)
tip_radius_mm = 3.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | R3.2(0.126")
tip_angle_deg = 60.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | alpha=60 deg
max_load_t_per_m = 160.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | Max load 160 t/m (54 t/ft)
rear_extent_from_tip_axis_mm = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | horizontal dim 30(1.181") drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | not dimensioned on this drawing
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns (no 255 mm listed)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | segmented tooling figure
weight_515mm_kg = 11.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | 11.8 kg for 515 mm

[[punch]]
id = "wila/BIU-806"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | type BIU-806
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | WILA catalogue USA 2024
part_number = "BIU-806"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | BIU-806/1 = 515 mm variant; other suffixes = other lengths
name = "American Style top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | section 'TOP TOOLS' / 'American Style TOOLING'
clamping = "WILA American Style top tool, AS II shoulder load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | adaption figure p.76: 'AS II shoulder load'
kind = "gooseneck-small"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | drawing only; the catalogue does not name the kind
height_mm = 100.4   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | vertical dim on drawing 100.4(3.953") (tip to tool shoulder)
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | R1(0.039")
tip_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | alpha=86 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | horizontal dim 30(1.181") drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 10.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | oblique dim at tip 10(0.394") mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns (no 255 mm listed)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | segmented tooling figure
weight_515mm_kg = 8.3   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | 8.3 kg for 515 mm

[[punch]]
id = "wila/BIU-807"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | type BIU-807
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | WILA catalogue USA 2024
part_number = "BIU-807"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | BIU-807/1 = 515 mm variant; other suffixes = other lengths
name = "American Style top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | section 'TOP TOOLS' / 'American Style TOOLING'
clamping = "WILA American Style top tool, AS II shoulder load adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | adaption figure p.76: 'AS II shoulder load'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | drawing only; the catalogue does not name the kind
height_mm = 100.4   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | vertical dim on drawing 100.4(3.953") (tip to tool shoulder)
tip_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | R1(0.039")
tip_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | alpha=86 deg
max_load_t_per_m = 65.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | Max load 65 t/m (22 t/ft)
rear_extent_from_tip_axis_mm = 54.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | horizontal dim 54(2.126") drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 8.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | oblique dim at tip 8(0.315") mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns (no 255 mm listed)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | segmented tooling figure
weight_515mm_kg = 11.4   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.76) | 11.4 kg for 515 mm

[[punch]]
id = "wila/BIU-803"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | type BIU-803
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | WILA catalogue USA 2024
part_number = "BIU-803"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | BIU-803/1 = 515 mm variant; other suffixes = other lengths
name = "American Style top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | section 'TOP TOOLS' / 'American Style TOOLING'
clamping = "WILA American Style top tool (adaption variant not shown on this page)"   # unknown (variant) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | section header 'American Style TOOLING'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | drawing only; the catalogue does not name the kind
height_mm = 100.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | vertical dim on drawing 100.6(3.961") (tip to tool shoulder)
tip_radius_mm = 1.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | R1.5(0.059")
tip_angle_deg = 88.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | alpha=88 deg
max_load_t_per_m = 160.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | Max load 160 t/m (54 t/ft)
rear_extent_from_tip_axis_mm = 45.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | horizontal dim 45(1.772") drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 19.0   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | oblique dim at tip 19(0.748") mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns (no 255 mm listed)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | segmented tooling figure
weight_515mm_kg = 12.4   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | 12.4 kg for 515 mm

[[punch]]
id = "wila/BIU-804"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | type BIU-804
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | WILA catalogue USA 2024
part_number = "BIU-804"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | BIU-804/1 = 515 mm variant; other suffixes = other lengths
name = "American Style top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | section 'TOP TOOLS' / 'American Style TOOLING'
clamping = "WILA American Style top tool (adaption variant not shown on this page)"   # unknown (variant) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | section header 'American Style TOOLING'
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | drawing only; the catalogue does not name the kind
height_mm = 100.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | vertical dim on drawing 100.6(3.961") (tip to tool shoulder)
tip_radius_mm = 0.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | R0.8(0.031")
tip_angle_deg = 90.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | alpha=90 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 65.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | horizontal dim 65(2.559") drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 12.7   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | oblique dim at tip 12.7(0.500") mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns (no 255 mm listed)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | segmented tooling figure
weight_515mm_kg = 15.9   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | 15.9 kg for 515 mm

[[punch]]
id = "wila/BIU-801"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | type BIU-801
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | WILA catalogue USA 2024
part_number = "BIU-801"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | BIU-801/1 = 515 mm variant; other suffixes = other lengths
name = "American Style top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | section 'TOP TOOLS' / 'American Style TOOLING'
clamping = "WILA American Style top tool (adaption variant not shown on this page)"   # unknown (variant) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | section header 'American Style TOOLING'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | drawing only; the catalogue does not name the kind
height_mm = 100.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | vertical dim on drawing 100.6(3.961") (tip to tool shoulder)
tip_radius_mm = 0.4   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | R0.4(0.016")
tip_angle_deg = 90.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | alpha=90 deg
max_load_t_per_m = 45.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | Max load 45 t/m (15 t/ft)
rear_extent_from_tip_axis_mm = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | horizontal dim 30(1.181") drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 6.4   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | oblique dim at tip 6.4(0.250") mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns (no 255 mm listed)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | segmented tooling figure
weight_515mm_kg = 8.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | 8.5 kg for 515 mm

[[punch]]
id = "wila/BIU-802"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | type BIU-802
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | WILA catalogue USA 2024
part_number = "BIU-802"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | BIU-802/1 = 515 mm variant; other suffixes = other lengths
name = "American Style top tool"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | section 'TOP TOOLS' / 'American Style TOOLING'
clamping = "WILA American Style top tool (adaption variant not shown on this page)"   # unknown (variant) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | section header 'American Style TOOLING'
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | drawing only; the catalogue does not name the kind
height_mm = 100.6   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | vertical dim on drawing 100.6(3.961") (tip to tool shoulder)
tip_radius_mm = 0.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | R0.8(0.031")
tip_angle_deg = 90.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | alpha=90 deg
max_load_t_per_m = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | Max load 100 t/m (34 t/ft)
rear_extent_from_tip_axis_mm = 35.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | horizontal dim 35(1.378") drawn from tip axis to rear face of the body (for goosenecks = full depth behind tip line)
tip_flank_oblique_dim_mm = 12.7   # published (meaning not stated) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | oblique dim at tip 12.7(0.500") mm, drawn along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | throat not dimensioned; only rear extent given
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | not dimensioned
tang_dims = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | adaption drawn schematically without dimensions
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 200 mm sectioned with horns (no 255 mm listed)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | Length column of the table
sectioning = "550 mm set drawn as 100+200+20+25+30+35+40+100 mm, horn R=25; 200 mm set = 100+100; any length 20 mm..machine length in 5 mm steps"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | segmented tooling figure
weight_515mm_kg = 10.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.77) | 10.5 kg for 515 mm

[[die]]
id = "wila/OZU-061"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | type OZU-061
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | WILA catalogue USA 2024
part_number = "OZU-061"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | OZU-061/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.54 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 6.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | V=6 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | alpha=30 deg
shoulder_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | R1 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = 16.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | top horizontal dim 16 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | Max load 80 t/m (27 t/ft), column also states alpha=90 deg
gauge_label = "22 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | Length column
weight_515mm_kg = 4.9   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | 4.9 kg for 515 mm

[[die]]
id = "wila/OZU-051"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | type OZU-051
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | WILA catalogue USA 2024
part_number = "OZU-051"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | OZU-051/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.54 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 8.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | V=8 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | alpha=30 deg
shoulder_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | R1 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = 16.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | top horizontal dim 16 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | Max load 80 t/m (27 t/ft)
gauge_label = "20 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | Length column
weight_515mm_kg = 4.9   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | 4.9 kg for 515 mm

[[die]]
id = "wila/OZU-062"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | type OZU-062
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | WILA catalogue USA 2024
part_number = "OZU-062"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | OZU-062/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.54 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 10.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | V=10 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | alpha=30 deg
shoulder_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | R1 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | top horizontal dim 20 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | Max load 80 t/m (27 t/ft)
gauge_label = "18 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | Length column
weight_515mm_kg = 5.4   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | 5.4 kg for 515 mm

[[die]]
id = "wila/OZU-052"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | type OZU-052
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | WILA catalogue USA 2024
part_number = "OZU-052"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | OZU-052/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.54 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 12.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | V=12 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | alpha=30 deg
shoulder_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | R1 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | top horizontal dim 20 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | Max load 80 t/m (27 t/ft)
gauge_label = "16 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | Length column
weight_515mm_kg = 5.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.54) | 5.2 kg for 515 mm

[[die]]
id = "wila/OZU-053"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | type OZU-053
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | WILA catalogue USA 2024
part_number = "OZU-053"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | OZU-053/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.55 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 16.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | V=16 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | alpha=30 deg
shoulder_radius_mm = 1.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | R1.5 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | only the V is dimensioned at the top
body_width_mm = 30.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | lower horizontal dim 30 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | Max load 80 t/m (27 t/ft)
gauge_label = "14 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | Length column
weight_515mm_kg = 7.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | 7.0 kg for 515 mm

[[die]]
id = "wila/OZU-063"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | type OZU-063
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | WILA catalogue USA 2024
part_number = "OZU-063"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | OZU-063/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.55 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | V=20 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | alpha=30 deg
shoulder_radius_mm = 2.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | R2 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | only the V is dimensioned at the top
body_width_mm = 35.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | lower horizontal dim 35 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | Max load 80 t/m (27 t/ft)
gauge_label = "13 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | Length column
weight_515mm_kg = 7.7   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | 7.7 kg for 515 mm

[[die]]
id = "wila/OZU-054"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | type OZU-054
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | WILA catalogue USA 2024
part_number = "OZU-054"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | OZU-054/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.55 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 24.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | V=24 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | alpha=30 deg
shoulder_radius_mm = 2.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | R2.5 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | only the V is dimensioned at the top
body_width_mm = 40.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | lower horizontal dim 40 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | Max load 80 t/m (27 t/ft)
gauge_label = "11 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | Length column
weight_515mm_kg = 8.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | 8.2 kg for 515 mm

[[die]]
id = "wila/OZU-361"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | type OZU-361
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | WILA catalogue USA 2024
part_number = "OZU-361"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | OZU-361/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.55 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 6.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | V=6 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | alpha=30 deg
shoulder_radius_mm = 2.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | R2 mm
height_mm = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | vertical dim 100 mm (overall, incl. adaption, as drawn)
top_width_mm = 16.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | top horizontal dim 16 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | Max load 80 t/m (27 t/ft)
gauge_label = "22 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | Length column
weight_515mm_kg = 7.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.55) | 7.8 kg for 515 mm

[[die]]
id = "wila/OZU-351"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | type OZU-351
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | WILA catalogue USA 2024
part_number = "OZU-351"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | OZU-351/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.56 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 8.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | V=8 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | alpha=30 deg
shoulder_radius_mm = 2.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | R2.5 mm
height_mm = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | vertical dim 100 mm (overall, incl. adaption, as drawn)
top_width_mm = 16.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | top horizontal dim 16 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | Max load 80 t/m (27 t/ft)
gauge_label = "20 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | Length column
weight_515mm_kg = 7.7   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | 7.7 kg for 515 mm

[[die]]
id = "wila/OZU-362"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | type OZU-362
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | WILA catalogue USA 2024
part_number = "OZU-362"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | OZU-362/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.56 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 10.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | V=10 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | alpha=30 deg
shoulder_radius_mm = 2.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | R2.5 mm
height_mm = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | vertical dim 100 mm (overall, incl. adaption, as drawn)
top_width_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | top horizontal dim 20 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | Max load 80 t/m (27 t/ft)
gauge_label = "18 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | Length column
weight_515mm_kg = 9.1   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | 9.1 kg for 515 mm

[[die]]
id = "wila/OZU-352"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | type OZU-352
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | WILA catalogue USA 2024
part_number = "OZU-352"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | OZU-352/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.56 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 12.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | V=12 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | alpha=30 deg
shoulder_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | R3 mm
height_mm = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | vertical dim 100 mm (overall, incl. adaption, as drawn)
top_width_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | top horizontal dim 20 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | Max load 80 t/m (27 t/ft)
gauge_label = "16 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | Length column
weight_515mm_kg = 9.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | 9.0 kg for 515 mm

[[die]]
id = "wila/OZU-353"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | type OZU-353
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | WILA catalogue USA 2024
part_number = "OZU-353"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | OZU-353/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.56 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 16.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | V=16 mm
v_angle_deg = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | alpha=30 deg
shoulder_radius_mm = 3.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | R3.5 mm
height_mm = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | vertical dim 100 mm (overall, incl. adaption, as drawn)
top_width_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | only the V is dimensioned at the top
body_width_mm = 30.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | lower horizontal dim 30 mm across the body
max_load_t_per_m = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | Max load 80 t/m (27 t/ft)
gauge_label = "14 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | Length column
weight_515mm_kg = 12.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.56) | 12.5 kg for 515 mm

[[die]]
id = "wila/OZU-013"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | type OZU-013
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | WILA catalogue USA 2024
part_number = "OZU-013"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | OZU-013/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.61 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 24.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | V=24 mm
v_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | alpha=80 deg
shoulder_radius_mm = 2.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | R2.5 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | only the V is dimensioned at the top
body_width_mm = 35.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | lower horizontal dim 35 mm across the body
max_load_t_per_m = 125.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | Max load 125 t/m (42 t/ft)
gauge_label = "11 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | Length column
weight_515mm_kg = 8.2   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | 8.2 kg for 515 mm

[[die]]
id = "wila/OZU-014"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | type OZU-014
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | WILA catalogue USA 2024
part_number = "OZU-014"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | OZU-014/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.61 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 30.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | V=30 mm
v_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | alpha=80 deg
shoulder_radius_mm = 3.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | R3 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | only the V is dimensioned at the top
body_width_mm = 40.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | lower horizontal dim 40 mm across the body
max_load_t_per_m = 125.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | Max load 125 t/m (42 t/ft)
gauge_label = "10 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | Length column
weight_515mm_kg = 8.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | 8.8 kg for 515 mm

[[die]]
id = "wila/OZU-015"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | type OZU-015
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | WILA catalogue USA 2024
part_number = "OZU-015"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | OZU-015/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.61 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 40.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | V=40 mm
v_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | alpha=80 deg
shoulder_radius_mm = 4.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | R4 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | only the V is dimensioned at the top
body_width_mm = 50.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | lower horizontal dim 50 mm across the body
max_load_t_per_m = 125.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | Max load 125 t/m (42 t/ft)
gauge_label = "3/16'"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | Length column
weight_515mm_kg = 10.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | 10.0 kg for 515 mm

[[die]]
id = "wila/OZU-035"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | type OZU-035
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | WILA catalogue USA 2024
part_number = "OZU-035"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | OZU-035/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.61 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 50.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | V=50 mm
v_angle_deg = 80.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | alpha=80 deg
shoulder_radius_mm = 5.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | R5 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | only the V is dimensioned at the top
body_width_mm = 75.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | lower horizontal dim 75 mm across the body
max_load_t_per_m = 150.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | Max load 150 t/m (50 t/ft)
gauge_label = "1/4'"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | Length column
weight_515mm_kg = 14.9   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.61) | 14.9 kg for 515 mm

[[die]]
id = "wila/OZU-024"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | type OZU-024
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | WILA catalogue USA 2024
part_number = "OZU-024"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | OZU-024/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.64 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 4.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | V=4 mm
v_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | alpha=86 deg
shoulder_radius_mm = 0.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | R0.5 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = 10.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | top horizontal dim 10 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 45.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | Max load 45 t/m (15 t/ft)
gauge_label = "25 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | Length column
weight_515mm_kg = 3.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | 3.5 kg for 515 mm
conflict_note = "V label reads 'V=4 mm/0.472in'; 4 mm = 0.158 in (0.472 in is V12); drawing dim 4/0.158 agrees with 4 mm"   # conflict (inch value) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | V=4 mm/0.472"

[[die]]
id = "wila/OZU-021"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | type OZU-021
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | WILA catalogue USA 2024
part_number = "OZU-021"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | OZU-021/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.64 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 6.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | V=6 mm
v_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | alpha=86 deg
shoulder_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | R1 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = 16.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | top horizontal dim 16 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | Max load 120 t/m (40 t/ft)
gauge_label = "22 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | Length column
weight_515mm_kg = 4.9   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | 4.9 kg for 515 mm
conflict_note = "top width labelled 16/0.623in; 16 mm = 0.630 in as printed elsewhere"   # conflict (minor inch rounding) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | 16/0.623"

[[die]]
id = "wila/OZU-010"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | type OZU-010
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | WILA catalogue USA 2024
part_number = "OZU-010"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | OZU-010/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.64 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 8.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | V=8 mm
v_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | alpha=86 deg
shoulder_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | R1 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = 16.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | top horizontal dim 16 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | Max load 120 t/m (40 t/ft)
gauge_label = "20 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | Length column
weight_515mm_kg = 4.9   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | 4.9 kg for 515 mm

[[die]]
id = "wila/OZU-022"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | type OZU-022
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | WILA catalogue USA 2024
part_number = "OZU-022"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | OZU-022/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.64 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 10.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | V=10 mm
v_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | alpha=86 deg
shoulder_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | R1 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | top horizontal dim 20 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | Max load 120 t/m (40 t/ft)
gauge_label = "18 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | Length column
weight_515mm_kg = 5.7   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.64) | 5.7 kg for 515 mm

[[die]]
id = "wila/OZU-011"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | type OZU-011
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | WILA catalogue USA 2024
part_number = "OZU-011"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | OZU-011/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.65 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 12.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | V=12 mm
v_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | alpha=86 deg
shoulder_radius_mm = 1.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | R1 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | top horizontal dim 20 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | Max load 120 t/m (40 t/ft)
gauge_label = "16 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | Length column
weight_515mm_kg = 5.7   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | 5.7 kg for 515 mm

[[die]]
id = "wila/OZU-012"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | type OZU-012
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | WILA catalogue USA 2024
part_number = "OZU-012"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | OZU-012/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.65 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 16.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | V=16 mm
v_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | alpha=86 deg
shoulder_radius_mm = 1.5   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | R1.5 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | only the V is dimensioned at the top
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | Max load 120 t/m (40 t/ft)
gauge_label = "14 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | Length column
weight_515mm_kg = 6.7   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | 6.7 kg for 515 mm

[[die]]
id = "wila/OZU-023"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | type OZU-023
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | WILA catalogue USA 2024
part_number = "OZU-023"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | OZU-023/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.65 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 20.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | V=20 mm
v_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | alpha=86 deg
shoulder_radius_mm = 2.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | R2 mm
height_mm = 55.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | vertical dim 55 mm (overall, incl. adaption, as drawn)
top_width_mm = "UNKNOWN"   # unknown | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | only the V is dimensioned at the top
body_width_mm = 30.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | lower horizontal dim 30 mm across the body
max_load_t_per_m = 120.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | Max load 120 t/m (40 t/ft)
gauge_label = "13 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | Length column
weight_515mm_kg = 7.7   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | 7.7 kg for 515 mm

[[die]]
id = "wila/OZU-336"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | type OZU-336
maker = "WILA"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | WILA catalogue USA 2024
part_number = "OZU-336"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | OZU-336/1 = 515 mm
name = "New Standard Premium bottom tool (1-V)"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | section 'BOTTOM TOOLS'
clamping = "WILA New Standard, NS VI shoulder load with V-Lock; 13 mm adaption"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (p.65 adaption figure; p.41 fact table) | 'NS VI shoulder load with V-Lock'; 'Bottom (OZU...) have 13 mm adaption ... Clamping Slot 13.5 x 22 (WxH)'
v_opening_mm = 4.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | V=4 mm
v_angle_deg = 86.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | alpha=86 deg
shoulder_radius_mm = 0.8   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | R0.8 mm
height_mm = 100.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | vertical dim 100 mm (overall, incl. adaption, as drawn)
top_width_mm = 10.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | top horizontal dim 10 mm (above the V dim)
body_width_mm = 25.0   # published (meaning as drawn) | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | lower horizontal dim 25 mm across the body
max_load_t_per_m = 45.0   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | Max load 45 t/m (15 t/ft)
gauge_label = "25 Ga"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | gauge label printed under V
lengths = "515 mm; 550 mm sectioned; 200 mm sectioned; 200 mm; 255 mm"   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | Length column
weight_515mm_kg = 5.1   # published | https://www.wilatooling.com/media/l04fzgkk/wila-catalog_2024_usa.pdf (catalogue p.65) | 5.1 kg for 515 mm

```

### 3.2 Wilson Tool (European catalogue B374C 2025, American Precision B350C 2025)

```
[[punch]]
id = "wilson/40002"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Part No. 40002 (beveled 40002B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Wilson Tool catalogue
part_number = "40002"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | 40002
name = "88 deg standard tip punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series heading '400 SERIES - 88 deg STANDARD TIP PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40002B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series name '400 SERIES - 88 deg STANDARD TIP PUNCHES'
height_mm = 66.9   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | H 2.634 in [66.9 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | R .008 in [0.2 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | A 88 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = ".86 lbs/inch [.015 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Weight .86 lbs/inch [.015 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | 1.101 [27.97]
tip_flank_oblique_dim_mm = 9.0   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | .354 [9] drawn obliquely at tip
ear_piece = "35 x 35 mm with R5 corner, 10 mm step"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | EAR PIECE 1.378 [35], 1.378 [35], R .197 [R 5], .394 [10]
throat_depth_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | relief not dimensioned
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | not dimensioned
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40008"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Part No. 40008 (beveled 40008B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Wilson Tool catalogue
part_number = "40008"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | 40008
name = "88 deg standard tip punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series heading '400 SERIES - 88 deg STANDARD TIP PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40008B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series name '400 SERIES - 88 deg STANDARD TIP PUNCHES'
height_mm = 66.6   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | H 2.624 in [66.6 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | R .030 in [0.8 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | A 88 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = ".86 lbs/inch [.015 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Weight .86 lbs/inch [.015 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | 1.101 [27.97]
tip_flank_oblique_dim_mm = 9.0   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | .354 [9] drawn obliquely at tip
ear_piece = "35 x 35 mm with R5 corner, 10 mm step"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | EAR PIECE 1.378 [35], 1.378 [35], R .197 [R 5], .394 [10]
throat_depth_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | relief not dimensioned
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | not dimensioned
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40015"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Part No. 40015 (beveled 40015B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Wilson Tool catalogue
part_number = "40015"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | 40015
name = "88 deg standard tip punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series heading '400 SERIES - 88 deg STANDARD TIP PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40015B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series name '400 SERIES - 88 deg STANDARD TIP PUNCHES'
height_mm = 66.3   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | H 2.612 in [66.3 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 1.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | R .060 in [1.5 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | A 88 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = ".86 lbs/inch [.015 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Weight .86 lbs/inch [.015 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | 1.101 [27.97]
tip_flank_oblique_dim_mm = 9.0   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | .354 [9] drawn obliquely at tip
ear_piece = "35 x 35 mm with R5 corner, 10 mm step"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | EAR PIECE 1.378 [35], 1.378 [35], R .197 [R 5], .394 [10]
throat_depth_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | relief not dimensioned
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | not dimensioned
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40030"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Part No. 40030 (beveled 40030B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Wilson Tool catalogue
part_number = "40030"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | 40030
name = "88 deg standard tip punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series heading '400 SERIES - 88 deg STANDARD TIP PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40030B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series name '400 SERIES - 88 deg STANDARD TIP PUNCHES'
height_mm = 65.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | H 2.586 in [65.7 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 3.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | R .120 in [3.0 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | A 88 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = ".86 lbs/inch [.015 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Weight .86 lbs/inch [.015 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | 1.101 [27.97]
tip_flank_oblique_dim_mm = 9.0   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | .354 [9] drawn obliquely at tip
ear_piece = "35 x 35 mm with R5 corner, 10 mm step"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | EAR PIECE 1.378 [35], 1.378 [35], R .197 [R 5], .394 [10]
throat_depth_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | relief not dimensioned
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | not dimensioned
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40102"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Part No. 40102 (beveled 40102B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Wilson Tool catalogue
part_number = "40102"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | 40102
name = "90 deg standard tip punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series heading '400 SERIES - 90 deg STANDARD TIP PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40102B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series name '400 SERIES - 90 deg STANDARD TIP PUNCHES'
height_mm = 66.9   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | H 2.634 in [66.9 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | R .008 in [0.2 mm]
tip_angle_deg = 90   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | A 90 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = ".86 lbs/inch [.015 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Weight .86 lbs/inch [.015 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | 1.101 [27.97]
tip_flank_oblique_dim_mm = 9.0   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | .354 [9] drawn obliquely at tip
ear_piece = "35 x 35 mm with R5 corner, 10 mm step"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | EAR PIECE 1.378 [35], 1.378 [35], R .197 [R 5], .394 [10]
throat_depth_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | relief not dimensioned
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | not dimensioned
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40108"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Part No. 40108 (beveled 40108B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Wilson Tool catalogue
part_number = "40108"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | 40108
name = "90 deg standard tip punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series heading '400 SERIES - 90 deg STANDARD TIP PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40108B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series name '400 SERIES - 90 deg STANDARD TIP PUNCHES'
height_mm = 66.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | H 2.624 in [66.7 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | R .030 in [0.8 mm]
tip_angle_deg = 90   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | A 90 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = ".86 lbs/inch [.015 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Weight .86 lbs/inch [.015 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | 1.101 [27.97]
tip_flank_oblique_dim_mm = 9.0   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | .354 [9] drawn obliquely at tip
ear_piece = "35 x 35 mm with R5 corner, 10 mm step"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | EAR PIECE 1.378 [35], 1.378 [35], R .197 [R 5], .394 [10]
throat_depth_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | relief not dimensioned
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | not dimensioned
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40130"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Part No. 40130 (beveled 40130B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Wilson Tool catalogue
part_number = "40130"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | 40130
name = "90 deg standard tip punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series heading '400 SERIES - 90 deg STANDARD TIP PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40130B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series name '400 SERIES - 90 deg STANDARD TIP PUNCHES'
height_mm = 65.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | H 2.588 in [65.7 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 3.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | R .120 in [3.0 mm]
tip_angle_deg = 90   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | A 90 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = ".86 lbs/inch [.015 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Weight .86 lbs/inch [.015 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | 1.101 [27.97]
tip_flank_oblique_dim_mm = 9.0   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | .354 [9] drawn obliquely at tip
ear_piece = "35 x 35 mm with R5 corner, 10 mm step"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | EAR PIECE 1.378 [35], 1.378 [35], R .197 [R 5], .394 [10]
throat_depth_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | relief not dimensioned
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | not dimensioned
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40212"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Part No. 40212 (beveled 40212B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Wilson Tool catalogue
part_number = "40212"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | 40212
name = "88 deg thin tip punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series heading '402 SERIES - 88 & 90 deg THIN TIP PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40212B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | series name '402 SERIES - 88 & 90 deg THIN TIP PUNCHES'
height_mm = 66.9   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | H 2.634 in [66.9 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | R .008 in [0.2 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | A 88 deg
max_load_t_per_m = 40   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Max. Ton (LT & ST) feet [m]: [40]; XT [10]
weight = ".86 lbs/inch [.015 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.36) | Weight .86 lbs/inch [.015 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | 1.101 [27.97]
tip_flank_oblique_dim_mm = 4.02   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | .158 [4.02]
ear_piece = "10 mm wide, R5, 5 mm and 10 mm steps"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36) | .394 [10], R .197 [R 5], .197 [5], .394 [10]
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.36 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40302"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Part No. 40302 (beveled 40302B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Wilson Tool catalogue
part_number = "40302"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | 40302
name = "88 deg gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | series heading '403 SERIES - 88 deg GOOSENECK PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40302B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | series name '403 SERIES - 88 deg GOOSENECK PUNCHES'
height_mm = 119.9   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | H 4.721 in [119.9 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | R .008 in [0.2 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | A 88 deg
max_load_t_per_m = 50   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Max. Ton (LT & ST) feet [m]: [50]
weight = "1.68 lbs/inch [.030 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Weight 1.68 lbs/inch [.030 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | 1.101 [27.97]
throat_depth_mm = 58   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | horizontal dim [58] from tip axis to gooseneck throat
tip_flank_oblique_dim_mm = 9   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | [9] oblique at tip
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40308"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Part No. 40308 (beveled 40308B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Wilson Tool catalogue
part_number = "40308"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | 40308
name = "88 deg gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | series heading '403 SERIES - 88 deg GOOSENECK PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40308B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | series name '403 SERIES - 88 deg GOOSENECK PUNCHES'
height_mm = 119.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | H 4.711 in [119.7 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | R .030 in [0.8 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | A 88 deg
max_load_t_per_m = 50   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Max. Ton (LT & ST) feet [m]: [50]
weight = "1.68 lbs/inch [.030 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Weight 1.68 lbs/inch [.030 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | 1.101 [27.97]
throat_depth_mm = 58   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | horizontal dim [58] from tip axis to gooseneck throat
tip_flank_oblique_dim_mm = 9   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | [9] oblique at tip
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40402"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Part No. 40402 (beveled 40402B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Wilson Tool catalogue
part_number = "40402"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | 40402
name = "88 deg gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | series heading '404 SERIES - 88 & 90 deg GOOSENECK PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40402B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | series name '404 SERIES - 88 & 90 deg GOOSENECK PUNCHES'
height_mm = 104.9   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | H 4.130 in [104.9 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | R .008 in [0.2 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | A 88 deg
max_load_t_per_m = 50   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Max. Ton (LT & ST) feet [m]: [50]; XT [47]
weight = "1.54 lbs/inch [.027 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Weight 1.54 lbs/inch [.027 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | 1.101 [27.97]
throat_depth_mm = 58   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | horizontal dim [58] from tip axis to gooseneck throat
tip_flank_oblique_dim_mm = 9   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | [9] oblique at tip
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40415"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Part No. 40415 (beveled 40415B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Wilson Tool catalogue
part_number = "40415"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | 40415
name = "88 deg gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | series heading '404 SERIES - 88 & 90 deg GOOSENECK PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40415B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | series name '404 SERIES - 88 & 90 deg GOOSENECK PUNCHES'
height_mm = 104.3   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | H 4.108 in [104.3 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 1.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | R .060 in [1.5 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | A 88 deg
max_load_t_per_m = 50   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Max. Ton (LT & ST) feet [m]: [50]; XT [47]
weight = "1.54 lbs/inch [.027 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.37) | Weight 1.54 lbs/inch [.027 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | 1.101 [27.97]
throat_depth_mm = 58   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | horizontal dim [58] from tip axis to gooseneck throat
tip_flank_oblique_dim_mm = 9   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37) | [9] oblique at tip
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.37 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40602"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Part No. 40602 (beveled 40602B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Wilson Tool catalogue
part_number = "40602"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | 40602
name = "88 deg standard tip gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | series heading '406 SERIES - 88 deg STANDARD TIP GOOSENECK PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40602B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | series name '406 SERIES - 88 deg STANDARD TIP GOOSENECK PUNCHES'
height_mm = 89.9   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | H 3.539 in [89.9 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | R .008 in [0.2 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | A 88 deg
max_load_t_per_m = 70   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Max. Ton (LT & ST) feet [m]: [70]; XT [47]
weight = "1.29 lbs/inch [.023 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Weight 1.29 lbs/inch [.023 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38) | 1.101 [27.97]
throat_depth_mm = 48.3   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38) | horizontal dim [48.3] from tip axis to gooseneck throat
tip_flank_oblique_dim_mm = 9   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38) | [9] oblique at tip
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40630"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Part No. 40630 (beveled 40630B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Wilson Tool catalogue
part_number = "40630"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | 40630
name = "88 deg standard tip gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | series heading '406 SERIES - 88 deg STANDARD TIP GOOSENECK PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40630B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | series name '406 SERIES - 88 deg STANDARD TIP GOOSENECK PUNCHES'
height_mm = 88.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | H 3.491 in [88.7 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 3.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | R .120 in [3.0 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | A 88 deg
max_load_t_per_m = 70   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Max. Ton (LT & ST) feet [m]: [70]; XT [47]
weight = "1.29 lbs/inch [.023 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Weight 1.29 lbs/inch [.023 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38) | 1.101 [27.97]
throat_depth_mm = 48.3   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38) | horizontal dim [48.3] from tip axis to gooseneck throat
tip_flank_oblique_dim_mm = 9   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38) | [9] oblique at tip
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40712"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Part No. 40712 (beveled 40712B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Wilson Tool catalogue
part_number = "40712"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | 40712
name = "90 deg thin tip gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | series heading '407 SERIES - 90 deg THIN TIP GOOSENECK PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40712B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | series name '407 SERIES - 90 deg THIN TIP GOOSENECK PUNCHES'
height_mm = 89.9   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | H 3.539 in [89.9 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | R .008 in [0.2 mm]
tip_angle_deg = 90   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | A 90 deg
max_load_t_per_m = 50   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Max. Ton (LT & ST) feet [m]: [50]; XT [30]
weight = "1.29 lbs/inch [.023 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.38) | Weight 1.29 lbs/inch [.023 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38) | 1.101 [27.97]
throat_depth_mm = 49   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38) | horizontal dim [49] from tip axis to gooseneck throat
tip_flank_oblique_dim_mm = 6   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38) | [6] oblique at tip
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.38 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/44540"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | Part No. 44540 (beveled 44540B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | Wilson Tool catalogue
part_number = "44540"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | 44540
name = "85 deg gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | series heading '445 SERIES - 85 deg GOOSENECK PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 44540B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | series name '445 SERIES - 85 deg GOOSENECK PUNCHES'
height_mm = 105.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | H 4.134 in [105.0 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 4.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | R .188 in [4.8 mm]
tip_angle_deg = 85   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | A 85 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = "1.68 lbs/inch [.030 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | Weight 1.68 lbs/inch [.030 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.40) | 1.101 [27.97]
throat_depth_mm = 49   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.40) | 1.93 [49]
tip_flank_oblique_dim_mm = 15   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.40) | .590 [15]
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.40 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/40802"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | Part No. 40802 (beveled 40802B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | Wilson Tool catalogue
part_number = "40802"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | 40802
name = "88 deg narrow punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | series heading '88 & 90 deg NARROW PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 40802B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | catalogue section / tang columns
kind = "narrow"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | series name '88 & 90 deg NARROW PUNCHES'
height_mm = 95.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | H 3.740 in [95.0 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | R .008 in [0.2 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | A 88 deg
max_load_t_per_m = 50   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | Max. Ton (LT & ST) feet [m]: [50]
weight = "0.70 lbs/inch [.012 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.40) | Weight 0.70 lbs/inch [.012 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.40) | 1.101 [27.97]
narrow_blade_dims = "2.756 [70] vertical, .236 [6] width"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.40) | 2.756 [70], .236 [6]
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.40 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/41002"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | Part No. 41002 (beveled 41002B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | Wilson Tool catalogue
part_number = "41002"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | 41002
name = "88 deg sash punch - tall"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | series heading '88 deg SASH PUNCHES - TALL'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 41002B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | catalogue section / tang columns
kind = "sash"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | series name '88 deg SASH PUNCHES - TALL'
height_mm = 99.9   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | H 3.933 in [99.9 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | R .008 in [0.2 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | A 88 deg
max_load_t_per_m = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | Max. Ton (LT & ST) feet [m]: [30]
weight = "0.70 lbs/inch [.012 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | Weight 0.70 lbs/inch [.012 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.41) | 1.101 [27.97]
sash_dims = ".984 [25], R .394 [R 10], .394 [10], .788 [20], R .315 [8]"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.41) | dims on drawing
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.41 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/41102"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | Part No. 41102 (beveled 41102B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | Wilson Tool catalogue
part_number = "41102"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | 41102
name = "88 deg sash punch - short"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | series heading '88 & 90 deg SASH PUNCHES - SHORT'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 41102B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | catalogue section / tang columns
kind = "sash"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | series name '88 & 90 deg SASH PUNCHES - SHORT'
height_mm = 69.9   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | H 2.752 in [69.9 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | R .008 in [0.2 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | A 88 deg
max_load_t_per_m = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | Max. Ton (LT & ST) feet [m]: [30]
weight = "0.51 lbs/inch [.009 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.41) | Weight 0.51 lbs/inch [.009 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.41) | 1.101 [27.97]
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.41 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/41908"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | Part No. 41908 (beveled 41908B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | Wilson Tool catalogue
part_number = "41908"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | 41908
name = "30 deg acute punch - tall"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | series heading '419 SERIES - 30 deg ACUTE PUNCHES - TALL'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 41908B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | catalogue section / tang columns
kind = "acute"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | series name '419 SERIES - 30 deg ACUTE PUNCHES - TALL'
height_mm = 104.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | H 4.095 in [104.0 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | R .030 in [0.8 mm]
tip_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | A 30 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = "1.01 lbs/inch [.018 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | Weight 1.01 lbs/inch [.018 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.42) | 1.101 [27.97]
acute_dims = ".788 [20], R .315 [R 8], .472 [12], .709 [18]"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.42) | dims on 419 drawing
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.42 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/41930"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | Part No. 41930 (beveled 41930B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | Wilson Tool catalogue
part_number = "41930"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | 41930
name = "30 deg acute punch - tall"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | series heading '419 SERIES - 30 deg ACUTE PUNCHES - TALL'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 41930B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | catalogue section / tang columns
kind = "acute"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | series name '419 SERIES - 30 deg ACUTE PUNCHES - TALL'
height_mm = 97.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | H 3.846 in [97.7 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 3.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | R .120 in [3.0 mm]
tip_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | A 30 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = "1.01 lbs/inch [.018 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.42) | Weight 1.01 lbs/inch [.018 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.42) | 1.101 [27.97]
acute_dims = ".788 [20], R .315 [R 8], .472 [12], .709 [18]"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.42) | dims on 419 drawing
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.42 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/42610"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | Part No. 42610 (beveled 42610B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | Wilson Tool catalogue
part_number = "42610"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | 42610
name = "30 deg acute punch - short"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | series heading '426 SERIES - 30 deg ACUTE PUNCHES - SHORT'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 42610B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | catalogue section / tang columns
kind = "acute"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | series name '426 SERIES - 30 deg ACUTE PUNCHES - SHORT'
height_mm = 65.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | H 2.579 in [65.5 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | R .030 in [0.8 mm]
tip_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | A 30 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | Max. Ton (LT & ST) feet [m]: [100]
weight = "0.51 lbs/inch [.009 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | Weight 0.51 lbs/inch [.009 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.43) | 1.101 [27.97]
acute_dim = ".718 [18.24]"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.43) | .718 [18.24]
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.43 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/41440"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | Part No. 41440 (beveled 41440B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | Wilson Tool catalogue
part_number = "41440"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | 41440
name = "60 deg acute punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | series heading '414 SERIES - 60 deg ACUTE PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 41440B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | catalogue section / tang columns
kind = "acute"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | series name '414 SERIES - 60 deg ACUTE PUNCHES'
height_mm = 66.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | H 2.607 in [66.2 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 4.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | R .188 in [4.8 mm]
tip_angle_deg = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | A 60 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | Max. Ton (LT & ST) feet [m]: [100]
weight = "1.01 lbs/inch [.018 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.43) | Weight 1.01 lbs/inch [.018 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.43) | 1.101 [27.97]
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.43 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/44001"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | Part No. 44001 (beveled 44001B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | Wilson Tool catalogue
part_number = "44001"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | 44001
name = "75 deg standard punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | series heading '4400 SERIES - 75 deg STANDARD PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 44001B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | series name '4400 SERIES - 75 deg STANDARD PUNCHES'
height_mm = 67.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | H 2.645 in [67.2 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 1.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | R .060 in [1.5 mm]
tip_angle_deg = 75   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | A 75 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = "0.86 lbs/inch [.015 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | Weight 0.86 lbs/inch [.015 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.44) | 1.101 [27.97]
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.44 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/44031"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | Part No. 44031 (beveled 44031B)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | Wilson Tool catalogue
part_number = "44031"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | 44031
name = "75 deg gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | series heading '4403 SERIES - 75 deg GOOSENECK PUNCHES'
clamping = "European Traditional (Promecam-style) punch tang; straight tang part no. / beveled tang part no. 44031B"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | series name '4403 SERIES - 75 deg GOOSENECK PUNCHES'
height_mm = 119.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | H 4.707 in [119.5 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 1.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | R .060 in [1.5 mm]
tip_angle_deg = 75   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | A 75 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | Max. Ton (LT & ST) feet [m]: [100]; XT [50]
weight = "1.68 lbs/inch [.030 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.44) | Weight 1.68 lbs/inch [.030 kg/mm]
tang_height_mm = 27.97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.44) | 1.101 [27.97]
throat_depth_mm = 53   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.44) | 2.09 [53]
lengths = "LT 835 mm, ST 415 mm, XT 835 mm (sectionalized; section widths 10,15,20,40,50,100 L-ear,100 R-ear,200,300 mm per p.33)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.44 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/A0406"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Part No. A0406
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Wilson Tool catalogue
part_number = "A0406"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | A0406
name = "120mm tall standard punch (fixed height, beveled tang)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | series heading '120mm TALL PUNCHES W/ BEVELED TANG (FIXED HEIGHT)'
clamping = "Wilson Fixed Height tooling, beveled European tang"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | series name '120mm TALL PUNCHES W/ BEVELED TANG (FIXED HEIGHT)'
height_mm = 119.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | H 4.717 in [119.8 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.6   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | R .024 in [0.6 mm]
tip_angle_deg = 86   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | A 86 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Max. Ton (LT & ST) feet [m]: [100]; XT [40]
weight = "1.29 lbs/inch [.023 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Weight 1.29 lbs/inch [.023 kg/mm]
lengths = "L 835 mm, S 510 mm, X 370 mm sectionalized (X sections 10(2),15,20,25,40,50,100 mm + ear sections), 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.10 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/A4706"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Part No. A4706
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Wilson Tool catalogue
part_number = "A4706"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | A4706
name = "120mm tall gooseneck punch (fixed height)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | series heading '120mm TALL PUNCHES W/ BEVELED TANG (FIXED HEIGHT)'
clamping = "Wilson Fixed Height tooling, beveled European tang"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | series name '120mm TALL PUNCHES W/ BEVELED TANG (FIXED HEIGHT)'
height_mm = 119.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | H 4.717 in [119.8 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.6   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | R .024 in [0.6 mm]
tip_angle_deg = 86   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | A 86 deg
max_load_t_per_m = 50   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Max. Ton (LT & ST) feet [m]: [50]; XT [40]
weight = ".192 [.034] (as printed)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Weight .192 [.034] (as printed)
throat_depth_mm = 58   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.10) | 2.283 [58]
conflict_note = "weight printed '.192' lbs/inch next to [.034] kg/mm; .034 kg/mm = 1.9 lbs/inch"   # conflict (decimal point) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.10) | .192 [.034]
lengths = "L 835 mm, S 510 mm, X 370 mm sectionalized (X sections 10(2),15,20,25,40,50,100 mm + ear sections), 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.10 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/A4735"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Part No. A4735
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Wilson Tool catalogue
part_number = "A4735"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | A4735
name = "120mm tall gooseneck punch (fixed height)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | series heading '120mm TALL PUNCHES W/ BEVELED TANG (FIXED HEIGHT)'
clamping = "Wilson Fixed Height tooling, beveled European tang"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | series name '120mm TALL PUNCHES W/ BEVELED TANG (FIXED HEIGHT)'
height_mm = 118.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | H 4.373 (as printed) in [118.7 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 3.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | R .118 in [3.0 mm]
tip_angle_deg = 86   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | A 86 deg
max_load_t_per_m = 50   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Max. Ton (LT & ST) feet [m]: [50]; XT [40]
weight = ".192 [.034] (as printed)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.10) | Weight .192 [.034] (as printed)
conflict_note = "H printed 4.373 in but [118.7] mm; 118.7 mm = 4.673 in"   # conflict (inch value) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.10) | 4.373 [118.7]
lengths = "L 835 mm, S 510 mm, X 370 mm sectionalized (X sections 10(2),15,20,25,40,50,100 mm + ear sections), 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.10 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/A1406"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | Part No. A1406
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | Wilson Tool catalogue
part_number = "A1406"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | A1406
name = "85mm tall standard punch (fixed height)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | series heading '85mm TALL PUNCHES W/ BEVELED TANG (FIXED HEIGHT)'
clamping = "Wilson Fixed Height tooling, beveled European tang"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | series name '85mm TALL PUNCHES W/ BEVELED TANG (FIXED HEIGHT)'
height_mm = 84.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | H 3.339 in [84.8 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.6   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | R .024 in [0.6 mm]
tip_angle_deg = 86   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | A 86 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | Max. Ton (LT & ST) feet [m]: [100]; XT [47]
weight = "1.01 lbs/inch [.018 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | Weight 1.01 lbs/inch [.018 kg/mm]
lengths = "L 835 mm, S 510 mm, X 370 mm sectionalized (X sections 10(2),15,20,25,40,50,100 mm + ear sections), 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.9 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/A2306"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | Part No. A2306
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | Wilson Tool catalogue
part_number = "A2306"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | A2306
name = "85mm tall acute punch (fixed height)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | series heading '85mm TALL PUNCHES W/ BEVELED TANG (FIXED HEIGHT)'
clamping = "Wilson Fixed Height tooling, beveled European tang"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | catalogue section / tang columns
kind = "acute"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | series name '85mm TALL PUNCHES W/ BEVELED TANG (FIXED HEIGHT)'
height_mm = 84.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | H 3.339 in [84.8 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.6   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | R .024 in [0.6 mm]
tip_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | A 30 deg
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | Max. Ton (LT & ST) feet [m]: [100]; XT [30]
weight = "0.86 lbs/inch [.015 kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.9) | Weight 0.86 lbs/inch [.015 kg/mm]
lengths = "L 835 mm, S 510 mm, X 370 mm sectionalized (X sections 10(2),15,20,25,40,50,100 mm + ear sections), 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.9 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/50045"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | Part No. 50045 (beveled 50045G)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | Wilson Tool catalogue
part_number = "50045"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | 50045
name = "3.75in acute block punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | series heading '3.75" ACUTE BLOCK PUNCH'
clamping = "Wilson American Precision (American-style punch; straight or 'G' groove/button safety variants)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | catalogue section / tang columns
kind = "acute"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | series name '3.75" ACUTE BLOCK PUNCH'
height_mm = 93.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | H 3.661 in [93.0 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | R .031 in [0.8 mm]
tip_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | A 30 deg
max_load_t_per_m = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | only US tons/ft printed
weight = "gross lbs L/S/X 35/18/35"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | Weight gross lbs L/S/X 35/18/35
max_load_us = "30 US ton/ft (L&S), 40 (X) per column order"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.8) | Max Ton/ft 30 ... 40
lengths = "L 36 in [914.4 mm], S 18 in [457.2 mm], X sectionalized 35.87 in [911.2 mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.8 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/50209"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | Part No. 50209 (beveled 50209G)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | Wilson Tool catalogue
part_number = "50209"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | 50209
name = "5.75in acute gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | series heading '5.75" ACUTE GOOSENECK PUNCH'
clamping = "Wilson American Precision (American-style punch; straight or 'G' groove/button safety variants)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | catalogue section / tang columns
kind = "acute-gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | series name '5.75" ACUTE GOOSENECK PUNCH'
height_mm = 143.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | H 5.661 in [143.8 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | R .031 in [0.8 mm]
tip_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | A 30 deg
max_load_t_per_m = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | only US tons/ft printed
weight = "gross lbs L/S/X 42/21/38"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.8) | Weight gross lbs L/S/X 42/21/38
max_load_us = "30 / 40 US ton/ft"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.8) | Max Ton/ft 30 ... 40
ear_piece = "1.250 [31.8], R.375 [9.5], 1.750 [44.5] ref, .750 [19.1]"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.8) | EAR PIECE - 5.75" TALL ONLY
lengths = "L 36 in [914.4 mm], S 18 in [457.2 mm], X sectionalized 35.87 in [911.2 mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.8 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/50236"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | Part No. 50236 (beveled 50236G)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | Wilson Tool catalogue
part_number = "50236"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | 50236
name = "3.75in large gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | series heading '3.75" LARGE GOOSENECK PUNCH'
clamping = "Wilson American Precision (American-style punch; straight or 'G' groove/button safety variants)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | series name '3.75" LARGE GOOSENECK PUNCH'
height_mm = 94.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | H 3.730 in [94.7 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | R .031 in [0.8 mm]
tip_angle_deg = 75   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | A 75 deg
max_load_t_per_m = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | only US tons/ft printed
weight = "gross lbs L/S/X 46/23/42"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | Weight gross lbs L/S/X 46/23/42
max_load_us = "L&S 28, X 21 US ton/ft"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.10) | 28 / 21
width_behind_tip_mm = 54.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.10) | W Width 2.125 [54.0]
throat_depth_mm = 38.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.10) | 1.496 [38.0] from tip axis to throat
tip_flank_dim_in = ".437"   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.10) | .437
lengths = "L 36 in [914.4 mm], S 18 in [457.2 mm], X sectionalized 35.87 in [911.2 mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.10 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/50184"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | Part No. 50184 (beveled 50184G)
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | Wilson Tool catalogue
part_number = "50184"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | 50184
name = "3.75in large gooseneck punch"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | series heading '3.75" LARGE GOOSENECK PUNCH'
clamping = "Wilson American Precision (American-style punch; straight or 'G' groove/button safety variants)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | catalogue section / tang columns
kind = "gooseneck"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | series name '3.75" LARGE GOOSENECK PUNCH'
height_mm = 94.9   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | H 3.736 in [94.9 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 0.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | R .031 in [0.8 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | A 88 deg
max_load_t_per_m = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | only US tons/ft printed
weight = "gross lbs L/S/X 51/26/47"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.10) | Weight gross lbs L/S/X 51/26/47
max_load_us = "L&S 28, X 21 US ton/ft"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.10) | 28 / 21
width_behind_tip_mm = 60.3   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.10) | W Width 2.375 [60.3]
throat_depth_mm = 38.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.10) | 1.496 [38.0]
lengths = "L 36 in [914.4 mm], S 18 in [457.2 mm], X sectionalized 35.87 in [911.2 mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.10 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[punch]]
id = "wilson/50166"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.14) | Part No. 50166
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.14) | Wilson Tool catalogue
part_number = "50166"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.14) | 50166
name = "3.75in block punch (made to order)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.14) | series heading '3.75" BLOCK PUNCH'
clamping = "Wilson American Precision (American-style punch; straight or 'G' groove/button safety variants)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.14) | catalogue section / tang columns
kind = "straight"   # estimate (basis: series name 'standard'/'block' and drawing) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.14) | series name '3.75" BLOCK PUNCH'
height_mm = 93.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.14) | H 3.660 in [93.0 mm] (drawing: from tool shoulder to tip)
tip_radius_mm = 1.6   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.14) | R .062 in [1.6 mm]
tip_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.14) | A 88 deg
max_load_t_per_m = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.14) | only US tons/ft printed
weight = "gross lbs L/S/X 46/23/43"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.14) | Weight gross lbs L/S/X 46/23/43
max_load_us = "50 / 40 US ton/ft"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.14) | 50 40
width_mm = 31.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.14) | W 1.250 [31.8]
conflict_note = "50166 (R 1.6) H 3.660 [93.0] is lower than 50168 (R 3.2) H 3.695 [93.9]; elsewhere H falls as R grows"   # conflict (possible misprint; not resolved) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.14) | 3.660 [93.0] / 3.695 [93.9]
lengths = "L 36 in [914.4 mm], S 18 in [457.2 mm], X sectionalized 35.87 in [911.2 mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.14 column heads; p.33) | LT 32.87" [835mm], ST 16.34" [415mm], XT 32.87" [835mm]

[[die]]
id = "wilson/42008"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Part No. 42008
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Wilson Tool catalogue
part_number = "42008"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 42008
name = "88 deg quick change 2V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 88 deg QUICK CHANGE 2V DIES
clamping = "European Traditional quick change 2V (groove 15.1 x 6.5 mm in foot)"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | .593 [15.1], .256 [6.5]
v_opening_mm = "V1 4.0; V2 7.0"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | V1 [4.0], V2 [7.0]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | A 88 deg
shoulder_radius_mm = "V1 0.4; V2 0.4"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | R1 [0.4], R2 [0.4]
height_mm = 46   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 1.811 [46]
die_width_mm = 34.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | W Width [34.5]
v_edge_offsets_mm = "X1 3.5 (left face to V1 centre), X2 5.0 (right face to V2 centre)"   # published (meaning as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | X1 [3.5], X2 [5.0]
max_load_t_per_m = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Max. Ton feet [m] [60]
weight = "0.70 [.012]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Weight lbs./inch [kg/mm] 0.70 [.012]
hardness = "Nitrex HRC-70"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Nitrex standard on all 2V dies provides HRC-70
lengths = "LT 835, ST 835 (as printed in the column head), XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 32.87" [835mm] x3

[[die]]
id = "wilson/42007"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Part No. 42007
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Wilson Tool catalogue
part_number = "42007"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 42007
name = "88 deg quick change 2V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 88 deg QUICK CHANGE 2V DIES
clamping = "European Traditional quick change 2V (groove 15.1 x 6.5 mm in foot)"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | .593 [15.1], .256 [6.5]
v_opening_mm = "V1 6.0; V2 10.0"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | V1 [6.0], V2 [10.0]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | A 88 deg
shoulder_radius_mm = "V1 0.4; V2 0.6"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | R1 [0.4], R2 [0.6]
height_mm = 46   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 1.811 [46]
die_width_mm = 37.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | W Width [37.0]
v_edge_offsets_mm = "X1 4.5 (left face to V1 centre), X2 6.5 (right face to V2 centre)"   # published (meaning as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | X1 [4.5], X2 [6.5]
max_load_t_per_m = 70   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Max. Ton feet [m] [70]
weight = "0.70 [.012]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Weight lbs./inch [kg/mm] 0.70 [.012]
hardness = "Nitrex HRC-70"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Nitrex standard on all 2V dies provides HRC-70
lengths = "LT 835, ST 835 (as printed in the column head), XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 32.87" [835mm] x3

[[die]]
id = "wilson/42009"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Part No. 42009
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Wilson Tool catalogue
part_number = "42009"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 42009
name = "88 deg quick change 2V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 88 deg QUICK CHANGE 2V DIES
clamping = "European Traditional quick change 2V (groove 15.1 x 6.5 mm in foot)"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | .593 [15.1], .256 [6.5]
v_opening_mm = "V1 8.0; V2 12.0"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | V1 [8.0], V2 [12.0]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | A 88 deg
shoulder_radius_mm = "V1 0.5; V2 0.8"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | R1 [0.5], R2 [0.8]
height_mm = 46   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 1.811 [46]
die_width_mm = 39.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | W Width [39.0]
v_edge_offsets_mm = "X1 5.5 (left face to V1 centre), X2 7.5 (right face to V2 centre)"   # published (meaning as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | X1 [5.5], X2 [7.5]
max_load_t_per_m = 80   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Max. Ton feet [m] [80]
weight = "0.86 [.015]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Weight lbs./inch [kg/mm] 0.86 [.015]
hardness = "Nitrex HRC-70"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Nitrex standard on all 2V dies provides HRC-70
lengths = "LT 835, ST 835 (as printed in the column head), XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 32.87" [835mm] x3

[[die]]
id = "wilson/42005"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Part No. 42005
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Wilson Tool catalogue
part_number = "42005"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 42005
name = "88 deg quick change 2V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 88 deg QUICK CHANGE 2V DIES
clamping = "European Traditional quick change 2V (groove 15.1 x 6.5 mm in foot)"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | .593 [15.1], .256 [6.5]
v_opening_mm = "V1 12.0; V2 20.0"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | V1 [12.0], V2 [20.0]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | A 88 deg
shoulder_radius_mm = "V1 0.5; V2 0.5"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | R1 [0.5], R2 [0.5]
height_mm = 46   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 1.811 [46]
die_width_mm = 45.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | W Width [45.5]
v_edge_offsets_mm = "X1 7.5 (left face to V1 centre), X2 12.0 (right face to V2 centre)"   # published (meaning as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | X1 [7.5], X2 [12.0]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Max. Ton feet [m] [100]
weight = "0.86 [.015]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Weight lbs./inch [kg/mm] 0.86 [.015]
hardness = "Nitrex HRC-70"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Nitrex standard on all 2V dies provides HRC-70
lengths = "LT 835, ST 835 (as printed in the column head), XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 32.87" [835mm] x3

[[die]]
id = "wilson/42004"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Part No. 42004
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Wilson Tool catalogue
part_number = "42004"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 42004
name = "88 deg quick change 2V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 88 deg QUICK CHANGE 2V DIES
clamping = "European Traditional quick change 2V (groove 15.1 x 6.5 mm in foot)"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | .593 [15.1], .256 [6.5]
v_opening_mm = "V1 14.0; V2 18.0"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | V1 [14.0], V2 [18.0]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | A 88 deg
shoulder_radius_mm = "V1 0.5; V2 0.5"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | R1 [0.5], R2 [0.5]
height_mm = 46   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 1.811 [46]
die_width_mm = 45.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | W Width [45.0]
v_edge_offsets_mm = "X1 8.5 (left face to V1 centre), X2 10.5 (right face to V2 centre)"   # published (meaning as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | X1 [8.5], X2 [10.5]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Max. Ton feet [m] [100]
weight = "0.86 [.015]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Weight lbs./inch [kg/mm] 0.86 [.015]
hardness = "Nitrex HRC-70"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Nitrex standard on all 2V dies provides HRC-70
lengths = "LT 835, ST 835 (as printed in the column head), XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 32.87" [835mm] x3

[[die]]
id = "wilson/42006"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Part No. 42006
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Wilson Tool catalogue
part_number = "42006"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 42006
name = "88 deg quick change 2V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 88 deg QUICK CHANGE 2V DIES
clamping = "European Traditional quick change 2V (groove 15.1 x 6.5 mm in foot)"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | .593 [15.1], .256 [6.5]
v_opening_mm = "V1 16.0; V2 25.0"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | V1 [16.0], V2 [25.0]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | A 88 deg
shoulder_radius_mm = "V1 0.8; V2 0.8"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | R1 [0.8], R2 [0.8]
height_mm = 46   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 1.811 [46]
die_width_mm = 50.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | W Width [50.0]
v_edge_offsets_mm = "X1 9.5 (left face to V1 centre), X2 14.5 (right face to V2 centre)"   # published (meaning as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | X1 [9.5], X2 [14.5]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Max. Ton feet [m] [100]
weight = "0.86 [.015]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Weight lbs./inch [kg/mm] 0.86 [.015]
hardness = "Nitrex HRC-70"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Nitrex standard on all 2V dies provides HRC-70
lengths = "LT 835, ST 835 (as printed in the column head), XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 32.87" [835mm] x3

[[die]]
id = "wilson/42002"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Part No. 42002
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Wilson Tool catalogue
part_number = "42002"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 42002
name = "90 deg quick change 2V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 90 deg QUICK CHANGE 2V DIES
clamping = "European Traditional quick change 2V (groove 15.1 x 6.5 mm in foot)"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | .593 [15.1], .256 [6.5]
v_opening_mm = "V1 8.0; V2 12.0"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | V1 [8.0], V2 [12.0]
v_angle_deg = 90   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | A 90 deg
shoulder_radius_mm = "V1 0.5; V2 0.8"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | R1 [0.5], R2 [0.8]
height_mm = 46   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 1.811 [46]
die_width_mm = 39.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | W Width [39.0]
v_edge_offsets_mm = "X1 5.5 (left face to V1 centre), X2 7.5 (right face to V2 centre)"   # published (meaning as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | X1 [5.5], X2 [7.5]
max_load_t_per_m = 80   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Max. Ton feet [m] [80]
weight = "0.86 [.015]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Weight lbs./inch [kg/mm] 0.86 [.015]
hardness = "Nitrex HRC-70"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Nitrex standard on all 2V dies provides HRC-70
lengths = "LT 835, ST 835 (as printed in the column head), XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 32.87" [835mm] x3

[[die]]
id = "wilson/44103"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Part No. 44103
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Wilson Tool catalogue
part_number = "44103"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 44103
name = "75 deg quick change 2V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 75 deg QUICK CHANGE 2V DIES
clamping = "European Traditional quick change 2V (groove 15.1 x 6.5 mm in foot)"   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | .593 [15.1], .256 [6.5]
v_opening_mm = "V1 12.0; V2 25.0"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | V1 [12.0], V2 [25.0]
v_angle_deg = 75   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | A 75 deg
shoulder_radius_mm = "V1 3.0; V2 5.5"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | R1 [3.0], R2 [5.5]
height_mm = 46   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 1.811 [46]
die_width_mm = 51.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | W Width [51.7]
v_edge_offsets_mm = "X1 9.0 (left face to V1 centre), X2 16.7 (right face to V2 centre)"   # published (meaning as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | X1 [9.0], X2 [16.7]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Max. Ton feet [m] [100]
weight = "1.01 [.018]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Weight lbs./inch [kg/mm] 1.01 [.018]
hardness = "Nitrex HRC-70"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | Nitrex standard on all 2V dies provides HRC-70
lengths = "LT 835, ST 835 (as printed in the column head), XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.74) | 32.87" [835mm] x3

[[die]]
id = "wilson/42120"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Part No. 42120
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Wilson Tool catalogue
part_number = "42120"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 42120
name = "88 deg quick change 1V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 88 deg quick change 1V die
clamping = "European Traditional quick change 1V (in die holder #43030 or #43530)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | section / drawing
v_opening_mm = 6.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | V [6.0 mm]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Angle 88 deg
shoulder_radius_mm = 1.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Radius [1.5 mm]
height_mm = 83.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | drawing height [83.5 mm]
die_width_mm = 14.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | W Width [14.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Max. Ton feet [m]: [100]
weight = "0.51 [.009] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Weight 0.51 [.009] lbs/inch [kg/mm]
foot_width_mm = 14   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | .551 [14]
lengths = "LT 835, ST 415, XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | 32.87" [835mm], 16.34" [415mm], 32.87" [835mm]

[[die]]
id = "wilson/42121"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Part No. 42121
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Wilson Tool catalogue
part_number = "42121"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 42121
name = "88 deg quick change 1V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 88 deg quick change 1V die
clamping = "European Traditional quick change 1V (in die holder #43030 or #43530)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | section / drawing
v_opening_mm = 8.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | V [8.0 mm]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Angle 88 deg
shoulder_radius_mm = 1.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Radius [1.5 mm]
height_mm = 83.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | drawing height [83.5 mm]
die_width_mm = 14.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | W Width [14.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Max. Ton feet [m]: [100]
weight = "0.51 [.009] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Weight 0.51 [.009] lbs/inch [kg/mm]
foot_width_mm = 14   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | .551 [14]
lengths = "LT 835, ST 415, XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | 32.87" [835mm], 16.34" [415mm], 32.87" [835mm]

[[die]]
id = "wilson/42122"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Part No. 42122
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Wilson Tool catalogue
part_number = "42122"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 42122
name = "88 deg quick change 1V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 88 deg quick change 1V die
clamping = "European Traditional quick change 1V (in die holder #43030 or #43530)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | section / drawing
v_opening_mm = 10.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | V [10.0 mm]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Angle 88 deg
shoulder_radius_mm = 2.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Radius [2.0 mm]
height_mm = 83.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | drawing height [83.5 mm]
die_width_mm = 14.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | W Width [14.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Max. Ton feet [m]: [100]
weight = "0.51 [.009] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Weight 0.51 [.009] lbs/inch [kg/mm]
foot_width_mm = 14   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | .551 [14]
lengths = "LT 835, ST 415, XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | 32.87" [835mm], 16.34" [415mm], 32.87" [835mm]

[[die]]
id = "wilson/42104"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Part No. 42104
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Wilson Tool catalogue
part_number = "42104"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 42104
name = "88 deg quick change 1V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 88 deg quick change 1V die
clamping = "European Traditional quick change 1V (in die holder #43030 or #43530)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | section / drawing
v_opening_mm = 12.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | V [12.0 mm]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Angle 88 deg
shoulder_radius_mm = 2.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Radius [2.5 mm]
height_mm = 83.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | drawing height [83.5 mm]
die_width_mm = 17.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | W Width [17.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Max. Ton feet [m]: [100]
weight = "0.70 [.012] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Weight 0.70 [.012] lbs/inch [kg/mm]
foot_width_mm = 14   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | .551 [14]
lengths = "LT 835, ST 415, XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | 32.87" [835mm], 16.34" [415mm], 32.87" [835mm]

[[die]]
id = "wilson/42105"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Part No. 42105
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Wilson Tool catalogue
part_number = "42105"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 42105
name = "88 deg quick change 1V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 88 deg quick change 1V die
clamping = "European Traditional quick change 1V (in die holder #43030 or #43530)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | section / drawing
v_opening_mm = 14.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | V [14.0 mm]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Angle 88 deg
shoulder_radius_mm = 2.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Radius [2.5 mm]
height_mm = 83.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | drawing height [83.5 mm]
die_width_mm = 18.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | W Width [18.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Max. Ton feet [m]: [100]
weight = "0.70 [.012] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Weight 0.70 [.012] lbs/inch [kg/mm]
foot_width_mm = 14   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | .551 [14]
lengths = "LT 835, ST 415, XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | 32.87" [835mm], 16.34" [415mm], 32.87" [835mm]

[[die]]
id = "wilson/42100"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Part No. 42100
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Wilson Tool catalogue
part_number = "42100"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 42100
name = "90 deg quick change 1V die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | 90 deg quick change 1V die
clamping = "European Traditional quick change 1V (in die holder #43030 or #43530)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | section / drawing
v_opening_mm = 6.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | V [6.0 mm]
v_angle_deg = 90   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Angle 90 deg
shoulder_radius_mm = 1.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Radius [1.5 mm]
height_mm = 83.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | drawing height [83.5 mm]
die_width_mm = 14.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | W Width [14.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Max. Ton feet [m]: [100]
weight = "0.51 [.009] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.81) | Weight 0.51 [.009] lbs/inch [kg/mm]
foot_width_mm = 14   # published (as drawn) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | .551 [14]
lengths = "LT 835, ST 415, XT 835 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.81) | 32.87" [835mm], 16.34" [415mm], 32.87" [835mm]

[[die]]
id = "wilson/42200"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Part No. 42200
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Wilson Tool catalogue
part_number = "42200"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | 42200
name = "30 deg quick change 1V die (acute)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | 30 deg quick change 1V die (acute)
clamping = "European Traditional quick change 1V (in die holder #43030 or #43530)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | section / drawing
v_opening_mm = 6.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | V [6.0 mm]
v_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Angle 30 deg
shoulder_radius_mm = 3.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Radius [3.0 mm]
height_mm = 83.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | drawing height [83.5 mm]
die_width_mm = 16.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | W Width [16.0 mm]
max_load_t_per_m = 76   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Max. Ton feet [m]: [76]
weight = "0.51 [.009]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Weight 0.51 [.009]

[[die]]
id = "wilson/42201"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Part No. 42201
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Wilson Tool catalogue
part_number = "42201"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | 42201
name = "30 deg quick change 1V die (acute)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | 30 deg quick change 1V die (acute)
clamping = "European Traditional quick change 1V (in die holder #43030 or #43530)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | section / drawing
v_opening_mm = 8.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | V [8.0 mm]
v_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Angle 30 deg
shoulder_radius_mm = 3.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Radius [3.0 mm]
height_mm = 83.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | drawing height [83.5 mm]
die_width_mm = 19.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | W Width [19.0 mm]
max_load_t_per_m = 90   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Max. Ton feet [m]: [90]
weight = "0.51 [.009]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Weight 0.51 [.009]

[[die]]
id = "wilson/42202"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Part No. 42202
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Wilson Tool catalogue
part_number = "42202"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | 42202
name = "30 deg quick change 1V die (acute)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | 30 deg quick change 1V die (acute)
clamping = "European Traditional quick change 1V (in die holder #43030 or #43530)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | section / drawing
v_opening_mm = 10.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | V [10.0 mm]
v_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Angle 30 deg
shoulder_radius_mm = 3.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Radius [3.0 mm]
height_mm = 83.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | drawing height [83.5 mm]
die_width_mm = 24.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | W Width [24.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Max. Ton feet [m]: [100]
weight = "0.70 [.012]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Weight 0.70 [.012]

[[die]]
id = "wilson/42203"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Part No. 42203
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Wilson Tool catalogue
part_number = "42203"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | 42203
name = "30 deg quick change 1V die (acute)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | 30 deg quick change 1V die (acute)
clamping = "European Traditional quick change 1V (in die holder #43030 or #43530)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | section / drawing
v_opening_mm = 12.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | V [12.0 mm]
v_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Angle 30 deg
shoulder_radius_mm = 5.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Radius [5.0 mm]
height_mm = 83.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | drawing height [83.5 mm]
die_width_mm = 28.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | W Width [28.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Max. Ton feet [m]: [100]
weight = "0.70 [.012]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Weight 0.70 [.012]

[[die]]
id = "wilson/42205"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Part No. 42205
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Wilson Tool catalogue
part_number = "42205"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | 42205
name = "30 deg quick change 1V die (acute)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | 30 deg quick change 1V die (acute)
clamping = "European Traditional quick change 1V (in die holder #43030 or #43530)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | section / drawing
v_opening_mm = 16.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | V [16.0 mm]
v_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Angle 30 deg
shoulder_radius_mm = 5.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Radius [5.0 mm]
height_mm = 83.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | drawing height [83.5 mm]
die_width_mm = 36.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | W Width [36.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Max. Ton feet [m]: [100]
weight = ".086 [0.15] (as printed)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.82) | Weight .086 [0.15] (as printed)

[[die]]
id = "wilson/A8406"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Part No. A8406
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Wilson Tool catalogue
part_number = "A8406"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | A8406
name = "Fixed height 86 deg die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Fixed height 86 deg die
clamping = "Wilson Fixed Height tooling die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | section / drawing
v_opening_mm = 4   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | V [4 mm]
v_angle_deg = 86   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Angle 86 deg
shoulder_radius_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | not stated
height_mm = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | drawing height [60 mm]
die_width_mm = 10.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | W Width [10.0 mm]
max_load_t_per_m = 87   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Max. Ton feet [m]: [87]
weight = "0.33 [.006] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Weight 0.33 [.006] lbs/inch [kg/mm]
lengths = "L 835, S 510, X 370 sectionalized, 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.9, p.12) | 32.87" [835mm], 20.08" [510mm], 14.57" [370mm], 3.93" [100mm]

[[die]]
id = "wilson/A7106"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Part No. A7106
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Wilson Tool catalogue
part_number = "A7106"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | A7106
name = "Fixed height 86 deg die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Fixed height 86 deg die
clamping = "Wilson Fixed Height tooling die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | section / drawing
v_opening_mm = 8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | V [8 mm]
v_angle_deg = 86   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Angle 86 deg
shoulder_radius_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | not stated
height_mm = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | drawing height [60 mm]
die_width_mm = 12.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | W Width [12.0 mm]
max_load_t_per_m = 97   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Max. Ton feet [m]: [97]
weight = "0.33 [.006] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Weight 0.33 [.006] lbs/inch [kg/mm]
lengths = "L 835, S 510, X 370 sectionalized, 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.9, p.12) | 32.87" [835mm], 20.08" [510mm], 14.57" [370mm], 3.93" [100mm]

[[die]]
id = "wilson/A7206"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Part No. A7206
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Wilson Tool catalogue
part_number = "A7206"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | A7206
name = "Fixed height 86 deg die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Fixed height 86 deg die
clamping = "Wilson Fixed Height tooling die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | section / drawing
v_opening_mm = 10   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | V [10 mm]
v_angle_deg = 86   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Angle 86 deg
shoulder_radius_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | not stated
height_mm = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | drawing height [60 mm]
die_width_mm = 14.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | W Width [14.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Max. Ton feet [m]: [100]
weight = "0.51 [.009] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Weight 0.51 [.009] lbs/inch [kg/mm]
lengths = "L 835, S 510, X 370 sectionalized, 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.9, p.12) | 32.87" [835mm], 20.08" [510mm], 14.57" [370mm], 3.93" [100mm]

[[die]]
id = "wilson/A7406"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Part No. A7406
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Wilson Tool catalogue
part_number = "A7406"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | A7406
name = "Fixed height 86 deg die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Fixed height 86 deg die
clamping = "Wilson Fixed Height tooling die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | section / drawing
v_opening_mm = 12   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | V [12 mm]
v_angle_deg = 86   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Angle 86 deg
shoulder_radius_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | not stated
height_mm = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | drawing height [60 mm]
die_width_mm = 16.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | W Width [16.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Max. Ton feet [m]: [100]
weight = "0.51 [.009] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Weight 0.51 [.009] lbs/inch [kg/mm]
lengths = "L 835, S 510, X 370 sectionalized, 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.9, p.12) | 32.87" [835mm], 20.08" [510mm], 14.57" [370mm], 3.93" [100mm]

[[die]]
id = "wilson/A7706"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Part No. A7706
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Wilson Tool catalogue
part_number = "A7706"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | A7706
name = "Fixed height 86 deg die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Fixed height 86 deg die
clamping = "Wilson Fixed Height tooling die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | section / drawing
v_opening_mm = 16   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | V [16 mm]
v_angle_deg = 86   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Angle 86 deg
shoulder_radius_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | not stated
height_mm = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | drawing height [60 mm]
die_width_mm = 20.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | W Width [20.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Max. Ton feet [m]: [100]
weight = "0.51 [.009] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Weight 0.51 [.009] lbs/inch [kg/mm]
lengths = "L 835, S 510, X 370 sectionalized, 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.9, p.12) | 32.87" [835mm], 20.08" [510mm], 14.57" [370mm], 3.93" [100mm]

[[die]]
id = "wilson/A8206"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Part No. A8206
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Wilson Tool catalogue
part_number = "A8206"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | A8206
name = "Fixed height 86 deg die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Fixed height 86 deg die
clamping = "Wilson Fixed Height tooling die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | section / drawing
v_opening_mm = 25   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | V [25 mm]
v_angle_deg = 86   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Angle 86 deg
shoulder_radius_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | not stated
height_mm = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | drawing height [60 mm]
die_width_mm = 29.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | W Width [29.5 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Max. Ton feet [m]: [100]
weight = "0.70 [.012] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Weight 0.70 [.012] lbs/inch [kg/mm]
lengths = "L 835, S 510, X 370 sectionalized, 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.9, p.12) | 32.87" [835mm], 20.08" [510mm], 14.57" [370mm], 3.93" [100mm]

[[die]]
id = "wilson/A5400"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Part No. A5400
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Wilson Tool catalogue
part_number = "A5400"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | A5400
name = "Fixed height 84 deg die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Fixed height 84 deg die
clamping = "Wilson Fixed Height tooling die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | section / drawing
v_opening_mm = 40   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | V [40 mm]
v_angle_deg = 84   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Angle 84 deg
shoulder_radius_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | not stated
height_mm = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | drawing height [60 mm]
die_width_mm = 41.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | W Width [41.0 mm]
max_load_t_per_m = 100   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Max. Ton feet [m]: [100]
weight = "0.79 [.014] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Weight 0.79 [.014] lbs/inch [kg/mm]
lengths = "L 835, S 510, X 370 sectionalized, 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.9, p.12) | 32.87" [835mm], 20.08" [510mm], 14.57" [370mm], 3.93" [100mm]
conflict_note = "84 deg die listed with W 1.614 [41.0] for V 1.575 [40]: 0.5 mm land each side; row grouping in the table may be mis-set"   # conflict (plausibility; transcribed as printed) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.12) | 1.575 [40] 1.614 [41.0]

[[die]]
id = "wilson/A7103"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Part No. A7103
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Wilson Tool catalogue
part_number = "A7103"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | A7103
name = "Fixed height 30 deg die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Fixed height 30 deg die
clamping = "Wilson Fixed Height tooling die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | section / drawing
v_opening_mm = 8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | V [8 mm]
v_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Angle 30 deg
shoulder_radius_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | not stated
height_mm = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | drawing height [60 mm]
die_width_mm = 20.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | W Width [20.0 mm]
max_load_t_per_m = 20   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Max. Ton feet [m]: [20]
weight = "0.51 [.009] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Weight 0.51 [.009] lbs/inch [kg/mm]
lengths = "L 835, S 510, X 370 sectionalized, 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.9, p.12) | 32.87" [835mm], 20.08" [510mm], 14.57" [370mm], 3.93" [100mm]

[[die]]
id = "wilson/A7203"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Part No. A7203
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Wilson Tool catalogue
part_number = "A7203"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | A7203
name = "Fixed height 30 deg die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Fixed height 30 deg die
clamping = "Wilson Fixed Height tooling die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | section / drawing
v_opening_mm = 10   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | V [10 mm]
v_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Angle 30 deg
shoulder_radius_mm = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | not stated
height_mm = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | drawing height [60 mm]
die_width_mm = 25.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | W Width [25.0 mm]
max_load_t_per_m = 26   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Max. Ton feet [m]: [26]
weight = "0.51 [.009] lbs/inch [kg/mm]"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Weight 0.51 [.009] lbs/inch [kg/mm]
lengths = "L 835, S 510, X 370 sectionalized, 100 mm"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.9, p.12) | 32.87" [835mm], 20.08" [510mm], 14.57" [370mm], 3.93" [100mm]

[[die]]
id = "wilson/A7403"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (catalogue p.12) | Part No. A7403
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.12) | Wilson
v_opening_mm = 12   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.12) | .472 [12]
v_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.12) | 30 deg DIES (FIXED HEIGHT)
height_mm = 60   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.12) | 2.362 [60]
die_width_mm = 30.0   # conflict (inch value 1.811 in = 46 mm, mm value 30.0) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.12) | 1.811 [30.0]
max_load_t_per_m = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_c6cljg21n10pt7cqmtlrgudk22/B374C_EuroCatalog_NP_Sept2025.pdf (p.12) | 9 [30]

[[die]]
id = "wilson/50190"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Part No. 50190
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Wilson Tool catalogue
part_number = "50190"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | 50190
name = "Staged block die 88 deg (American Precision)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Staged block die 88 deg (American Precision)
clamping = "Wilson American Precision staged die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | section / drawing
v_opening_mm = 6.4   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | V [6.4 mm]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Angle 88 deg
shoulder_radius_mm = 0.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Radius [0.8 mm]
height_mm = 41.4   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | drawing height [41.4 mm]
die_width_mm = 25.4   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | W Width [25.4 mm]
max_load_t_per_m = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | only US tons/ft printed
weight = "see gross weight in source"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Weight see gross weight in source
v_bottom_radius_mm = 0.4   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | R2 V Rad. [0.4]
tang_dims_as_drawn = ".500 [12.7] and .625 [15.9]"   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | .500 [12.7] .625 [15.9]
lengths = "L 36 in, S 18 in, X 35.87 in"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | 36" 18" 35.87"

[[die]]
id = "wilson/50205"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Part No. 50205
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Wilson Tool catalogue
part_number = "50205"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | 50205
name = "Staged block die 88 deg (American Precision)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Staged block die 88 deg (American Precision)
clamping = "Wilson American Precision staged die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | section / drawing
v_opening_mm = 12.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | V [12.7 mm]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Angle 88 deg
shoulder_radius_mm = 1.6   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Radius [1.6 mm]
height_mm = 44.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | drawing height [44.7 mm]
die_width_mm = 25.4   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | W Width [25.4 mm]
max_load_t_per_m = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | only US tons/ft printed
weight = "see gross weight in source"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Weight see gross weight in source
v_bottom_radius_mm = 0.4   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | R2 V Rad. [0.4]
tang_dims_as_drawn = ".500 [12.7] and .625 [15.9]"   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | .500 [12.7] .625 [15.9]
lengths = "L 36 in, S 18 in, X 35.87 in"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | 36" 18" 35.87"

[[die]]
id = "wilson/50194"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Part No. 50194
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Wilson Tool catalogue
part_number = "50194"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | 50194
name = "Staged block die 88 deg (American Precision)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Staged block die 88 deg (American Precision)
clamping = "Wilson American Precision staged die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | section / drawing
v_opening_mm = 22.2   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | V [22.2 mm]
v_angle_deg = 88   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Angle 88 deg
shoulder_radius_mm = 4.0   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Radius [4.0 mm]
height_mm = 49.6   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | drawing height [49.6 mm]
die_width_mm = 31.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | W Width [31.8 mm]
max_load_t_per_m = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | only US tons/ft printed
weight = "see gross weight in source"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Weight see gross weight in source
v_bottom_radius_mm = 0.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | R2 V Rad. [0.8]
tang_dims_as_drawn = ".500 [12.7] and .625 [15.9]"   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | .500 [12.7] .625 [15.9]
lengths = "L 36 in, S 18 in, X 35.87 in"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | 36" 18" 35.87"

[[die]]
id = "wilson/50033"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Part No. 50033
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Wilson Tool catalogue
part_number = "50033"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | 50033
name = "Staged block die 90 deg (American Precision)"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Staged block die 90 deg (American Precision)
clamping = "Wilson American Precision staged die"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | section / drawing
v_opening_mm = 12.7   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | V [12.7 mm]
v_angle_deg = 90   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Angle 90 deg
shoulder_radius_mm = 1.6   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Radius [1.6 mm]
height_mm = 44.5   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | drawing height [44.5 mm]
die_width_mm = 25.4   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | W Width [25.4 mm]
max_load_t_per_m = "UNKNOWN"   # unknown | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | only US tons/ft printed
weight = "see gross weight in source"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Weight see gross weight in source
v_bottom_radius_mm = 0.4   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | R2 V Rad. [0.4]
tang_dims_as_drawn = ".500 [12.7] and .625 [15.9]"   # published (meaning not stated) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | .500 [12.7] .625 [15.9]
lengths = "L 36 in, S 18 in, X 35.87 in"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | 36" 18" 35.87"

[[die]]
id = "wilson/51026"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (catalogue p.42) | Part No. 51026 staged acute die
maker = "Wilson Tool International"   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | Wilson
v_opening_mm = 25.4   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | V-Opening Rad. Center 1.000 [25.4]; theoretical sharp .856 in
v_angle_deg = 30   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | STAGED ACUTE DIES 30 deg
height_mm = "3.097 in / 90.5 mm"   # conflict (3.097 in = 78.7 mm; mm value printed 90.5) | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | 3.097 [90.5]
die_width_mm = 50.8   # published | https://d2mw1gqvnic7hf.cloudfront.net/asset/636361170875/document_np91faq4rp5ore9gvr7uvh077o/B350C%20American%20Precision_NP_Oct2025.pdf (p.42) | 2.000 [50.8]

```

### 3.3 Rolleri (online catalogue rolleritools.com, type R1 European/Promecam)

```
[[punch]]
id = "rolleri/P.97.88.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9788r08.html | P.97.88.R08
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9788r08.html | rolleritools.com product page
part_number = "P.97.88.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9788r08.html | P.97.88.R08
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "gooseneck (small relief)"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/P.97-88-R08_PK.97-88-R025-01(1).jpg | drawing only
height_mm = 66.65   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=66,65mm
total_height_mm = 96.95   # published | https://www.rolleritools.com/upload/immagini/img1200/P.97-88-R08_PK.97-88-R025-01(1).jpg | drawing 96.95 mm overall (top of tang to tip)
tip_radius_mm = 0.8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=0,8mm
tip_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=88 deg
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=1000kN/m
material = "C45"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=C45
width_at_shoulder_mm = 26   # published | https://www.rolleritools.com/upload/immagini/img1200/P.97-88-R08_PK.97-88-R025-01(1).jpg | 26/1.02" across the body at the shoulder
tip_flank_oblique_dim_mm = 8.9   # published | https://www.rolleritools.com/upload/immagini/img1200/P.97-88-R08_PK.97-88-R025-01(1).jpg | 8.9/0.35" oblique at tip
lengths_and_weights = "835 mm 11.10; 415 mm 5.50; 795 mm FX 10.60; 805 mm F 10.70; 900 mm 11.90"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9788r08.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/P.130.88.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p13088r08-6678.html | P.130.88.R08
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p13088r08-6678.html | rolleritools.com product page
part_number = "P.130.88.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p13088r08-6678.html | P.130.88.R08
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "straight"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/P.130-88-R08_P.130-88-R025-01.jpg | drawing only
height_mm = 100   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=100mm
total_height_mm = 130   # published | https://www.rolleritools.com/upload/immagini/img1200/P.130-88-R08_P.130-88-R025-01.jpg | drawing 130 mm overall (top of tang to tip)
tip_radius_mm = 0.8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=0,8mm
tip_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=88 deg
max_load_kN_per_m = 400   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=400kN/m
material = "C45"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=C45
width_at_shoulder_mm = 26.25   # published | https://www.rolleritools.com/upload/immagini/img1200/P.130-88-R08_P.130-88-R025-01.jpg | 26.25/1.03" at shoulder
tip_flank_oblique_dim_mm = 8.5   # published | https://www.rolleritools.com/upload/immagini/img1200/P.130-88-R08_P.130-88-R025-01.jpg | 8.5/0.33"
lengths_and_weights = "835 mm 13.70; 415 mm 6.80; 795 mm FX 13.00; 805 mm F 13.20"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p13088r08-6678.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/P.135.88.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p13588r08.html | P.135.88.R08
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p13588r08.html | rolleritools.com product page
part_number = "P.135.88.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p13588r08.html | P.135.88.R08
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "gooseneck"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/P.135-88-R08_P.135-88-R025-01.jpg | drawing only
height_mm = 104.65   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=104,65mm
total_height_mm = 134.5   # published | https://www.rolleritools.com/upload/immagini/img1200/P.135-88-R08_P.135-88-R025-01.jpg | drawing 134.5 mm overall (top of tang to tip)
tip_radius_mm = 0.8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=0,8mm
tip_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=88 deg
max_load_kN_per_m = 500   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=500kN/m
material = "C45"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=C45
width_behind_tip_mm = 56   # published | https://www.rolleritools.com/upload/immagini/img1200/P.135-88-R08_P.135-88-R025-01.jpg | 56/2.20" from rear face to tip axis
tip_flank_oblique_dim_mm = 10.6   # published | https://www.rolleritools.com/upload/immagini/img1200/P.135-88-R08_P.135-88-R025-01.jpg | 10.6/0.42"
throat_depth_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/upload/immagini/img1200/P.135-88-R08_P.135-88-R025-01.jpg | throat not dimensioned
drawing_working_height_mm = 104.6   # published | https://www.rolleritools.com/upload/immagini/img1200/P.135-88-R08_P.135-88-R025-01.jpg | 104.6/4.12" (listing: 104,65)
lengths_and_weights = "835 mm 22.70; 415 mm 11.30; 795 mm FX 21.70; 805 mm F 22.00"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p13588r08.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/P.150.88.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p15088r08-6685.html | P.150.88.R08
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p15088r08-6685.html | rolleritools.com product page
part_number = "P.150.88.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p15088r08-6685.html | P.150.88.R08
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "gooseneck"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/P.150-88-R08-01.jpg | drawing only
height_mm = 120   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=120mm
total_height_mm = 150   # published | https://www.rolleritools.com/upload/immagini/img1200/P.150-88-R08-01.jpg | drawing 150 mm overall (top of tang to tip)
tip_radius_mm = 0.8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=0,8mm
tip_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=88 deg
max_load_kN_per_m = 500   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=500kN/m
material = "42Cr"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=42Cr
width_behind_tip_mm = 56   # published | https://www.rolleritools.com/upload/immagini/img1200/P.150-88-R08-01.jpg | 56/2.20"
tip_flank_oblique_dim_mm = 8.5   # published | https://www.rolleritools.com/upload/immagini/img1200/P.150-88-R08-01.jpg | 8.5/0.33"
lengths_and_weights = "835 mm 21.20; 415 mm 10.60; 795 mm FX 20.20; 805 mm F 20.50; 900 mm 22.90"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p15088r08-6685.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/P.120.35.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p12035r08-6727.html | P.120.35.R08
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p12035r08-6727.html | rolleritools.com product page
part_number = "P.120.35.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p12035r08-6727.html | P.120.35.R08
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "acute (straight)"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/P.120-35-R08_P.120-35-R15_P.120-35-R3-01-01.jpg | drawing only
height_mm = 90   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=90mm
total_height_mm = 120   # published | https://www.rolleritools.com/upload/immagini/img1200/P.120-35-R08_P.120-35-R15_P.120-35-R3-01-01.jpg | drawing 120 mm overall (top of tang to tip)
tip_radius_mm = 0.8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=0,8mm
tip_angle_deg = 35   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=35 deg
max_load_kN_per_m = 700   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=700kN/m
material = "C45"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=C45
width_at_shoulder_mm = 26   # published | https://www.rolleritools.com/upload/immagini/img1200/P.120-35-R08_P.120-35-R15_P.120-35-R3-01-01.jpg | 26/1.02" at shoulder
lengths_and_weights = "835 mm 12.30; 415 mm 6.00; 795 mm FX 11.70; 805 mm F 11.90"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p12035r08-6727.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/TOP.175.85.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/top17585r08.html | TOP.175.85.R08
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/top17585r08.html | rolleritools.com product page
part_number = "TOP.175.85.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/top17585r08.html | TOP.175.85.R08
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "gooseneck"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/TOP.175-85-R08_TOP.175.85.R2.jpg | drawing only
height_mm = 145   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=145mm
total_height_mm = 175   # published | https://www.rolleritools.com/upload/immagini/img1200/TOP.175-85-R08_TOP.175.85.R2.jpg | drawing 175 mm overall (top of tang to tip)
tip_radius_mm = 0.8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=0,8mm
tip_angle_deg = 85   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=85 deg
max_load_kN_per_m = 900   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=900kN/m
material = "42Cr"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=42Cr
width_behind_tip_mm = 75   # published | https://www.rolleritools.com/upload/immagini/img1200/TOP.175-85-R08_TOP.175.85.R2.jpg | 75/2.95" rear face to tip axis
throat_depth_mm = 45   # published | https://www.rolleritools.com/upload/immagini/img1200/TOP.175-85-R08_TOP.175.85.R2.jpg | 45/1.77" throat to tip axis
tip_flank_oblique_dim_mm = 10.7   # published | https://www.rolleritools.com/upload/immagini/img1200/TOP.175-85-R08_TOP.175.85.R2.jpg | 10.7/0.42"
lengths_and_weights = "525 mm 20.90; 495 mm FA 19.70; 500 mm FB 19.90"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/top17585r08.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/TOP.175.85.R2"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | TOP.175.85.R2
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | rolleritools.com product page
part_number = "TOP.175.85.R2"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | TOP.175.85.R2
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "gooseneck"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/TOP.175-85-R08_TOP.175.85.R2.jpg | drawing only
height_mm = 145   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=145mm
total_height_mm = 175   # published | https://www.rolleritools.com/upload/immagini/img1200/TOP.175-85-R08_TOP.175.85.R2.jpg | drawing 175 mm overall (top of tang to tip)
tip_radius_mm = "2 (name, data-raggio) / 0.8 (listing text)"   # conflict (see conflict_note) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=2 (name, data-raggio) / 0,8 (listing text)mm
tip_angle_deg = 85   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=85 deg
max_load_kN_per_m = 900   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=900kN/m
material = "42Cr"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=42Cr
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | detail page not read
conflict_note = "name and data-raggio say R2; listing text says 'Radius=0,8mm'"   # conflict | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | name and data-raggio say R2; listing text says 'Radius=0,8mm'

[[punch]]
id = "rolleri/TOP.175.26.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/top17526r08-6596.html | TOP.175.26.R08
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/top17526r08-6596.html | rolleritools.com product page
part_number = "TOP.175.26.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/top17526r08-6596.html | TOP.175.26.R08
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "acute (straight)"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/TOP.175-26-R08-01.jpg | drawing only
height_mm = 145   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=145mm
total_height_mm = 175   # published | https://www.rolleritools.com/upload/immagini/img1200/TOP.175-26-R08-01.jpg | drawing 175 mm overall (top of tang to tip)
tip_radius_mm = 0.8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=0,8mm
tip_angle_deg = 26   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=26 deg
max_load_kN_per_m = 950   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=950kN/m
material = "42Cr"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=42Cr
width_at_shoulder_mm = 27.25   # published | https://www.rolleritools.com/upload/immagini/img1200/TOP.175-26-R08-01.jpg | 27.25/1.07" at shoulder
lengths_and_weights = "525 mm 11.00; 495 mm FA 10.40; 500 mm FB 10.50"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/top17526r08-6596.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/00402"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/00402.html | 00402
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/00402.html | rolleritools.com product page
part_number = "00402"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/00402.html | 00402
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "gooseneck (small relief)"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/00402-01-01.jpg | drawing only
height_mm = 66.9   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=66,9mm
total_height_mm = 96.9   # published | https://www.rolleritools.com/upload/immagini/img1200/00402-01-01.jpg | drawing 96.9 mm overall (top of tang to tip)
tip_radius_mm = 0.2   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=0,2mm
tip_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=88 deg
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=1000kN/m
material = "C45"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=C45
width_at_shoulder_mm = 26   # published | https://www.rolleritools.com/upload/immagini/img1200/00402-01-01.jpg | 26/1.02"
tip_flank_oblique_dim_mm = 9   # published | https://www.rolleritools.com/upload/immagini/img1200/00402-01-01.jpg | 9/0.35"
series = "A Series (Amada-design compatible)"   # published | https://www.rolleritools.com/upload/immagini/img1200/00402-01-01.jpg | listing 'A Series'
lengths_and_weights = "835 mm 11.00; 415 mm 5.50; 835 mm FCA 11.00"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/00402.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/CFH150.86.R02-GNK"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/cfh15086r02-gnk-117324.html | CFH150.86.R02-GNK
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/cfh15086r02-gnk-117324.html | rolleritools.com product page
part_number = "CFH150.86.R02-GNK"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/cfh15086r02-gnk-117324.html | CFH150.86.R02-GNK
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "gooseneck"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/AFH150.86.R02-GNK-01.jpg | drawing only
height_mm = 120   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=120mm
total_height_mm = 150   # published | https://www.rolleritools.com/upload/immagini/img1200/AFH150.86.R02-GNK-01.jpg | drawing 150 mm overall (top of tang to tip)
tip_radius_mm = 0.2   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=0,2mm
tip_angle_deg = 86   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=86 deg
max_load_kN_per_m = 500   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=500kN/m
material = "42Cr"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=42Cr
web_width_at_throat_mm = 25.4   # published | https://www.rolleritools.com/upload/immagini/img1200/AFH150.86.R02-GNK-01.jpg | 25.4/1.0" drawn from rear face to throat
tip_flank_oblique_dim_mm = 9   # published | https://www.rolleritools.com/upload/immagini/img1200/AFH150.86.R02-GNK-01.jpg | 9/0.35"
series = "CFH fixed-height series"   # published | https://www.rolleritools.com/upload/immagini/img1200/AFH150.86.R02-GNK-01.jpg | listing 'CFH Series'
lengths_and_weights = "415 mm 12.50; 800 mm F 24.20; 835 mm 25.20"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/cfh15086r02-gnk-117324.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/P.97.60.R2"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9760r2-6711.html | P.97.60.R2
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9760r2-6711.html | rolleritools.com product page
part_number = "P.97.60.R2"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9760r2-6711.html | P.97.60.R2
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "gooseneck (small relief)"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/P.97-60-R08_P.97-60-R2-01-01.jpg | drawing only
height_mm = 67   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=67mm
total_height_mm = 97   # published | https://www.rolleritools.com/upload/immagini/img1200/P.97-60-R08_P.97-60-R2-01-01.jpg | drawing 97 mm overall (top of tang to tip)
tip_radius_mm = 2   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=2mm
tip_angle_deg = 60   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=60 deg
max_load_kN_per_m = 600   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=600kN/m
material = "C45"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=C45
width_at_shoulder_mm = 26   # published | https://www.rolleritools.com/upload/immagini/img1200/P.97-60-R08_P.97-60-R2-01-01.jpg | 26/1.02"
tip_flank_oblique_dim_mm = 12.4   # published | https://www.rolleritools.com/upload/immagini/img1200/P.97-60-R08_P.97-60-R2-01-01.jpg | 12.4/0.49"
lengths_and_weights = "835 mm 9.00; 415 mm 4.50; 795 mm FX 8.50; 805 mm F 8.70"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9760r2-6711.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/PU.117.26.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/pu11726r08-6735.html | PU.117.26.R08
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/pu11726r08-6735.html | rolleritools.com product page
part_number = "PU.117.26.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/pu11726r08-6735.html | PU.117.26.R08
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "acute (straight, narrow blade)"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/PU.117-26-R08_PU.117-26-R025-01-01.jpg | drawing only
height_mm = 117   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=117mm
total_height_mm = 147   # published | https://www.rolleritools.com/upload/immagini/img1200/PU.117-26-R08_PU.117-26-R025-01-01.jpg | drawing 147 mm overall (top of tang to tip)
tip_radius_mm = 0.8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=0,8mm
tip_angle_deg = 26   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=26 deg
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=1000kN/m
material = "C45"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=C45
width_at_shoulder_mm = 27   # published | https://www.rolleritools.com/upload/immagini/img1200/PU.117-26-R08_PU.117-26-R025-01-01.jpg | 27/1.06"
blade_width_mm = 14   # published | https://www.rolleritools.com/upload/immagini/img1200/PU.117-26-R08_PU.117-26-R025-01-01.jpg | 14/0.55" across the blade
lengths_and_weights = "835 mm 13.60; 415 mm 6.70; 795 mm FX 12.90; 805 mm F 13.10"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/pu11726r08-6735.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/P.120.85.R1"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p12085r1-6696.html | P.120.85.R1
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p12085r1-6696.html | rolleritools.com product page
part_number = "P.120.85.R1"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p12085r1-6696.html | P.120.85.R1
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "straight (relieved)"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/P.120-85-R1-01-01.jpg | drawing only
height_mm = 90   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=90mm
total_height_mm = 120   # published | https://www.rolleritools.com/upload/immagini/img1200/P.120-85-R1-01-01.jpg | drawing 120 mm overall (top of tang to tip)
tip_radius_mm = 1   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=1mm
tip_angle_deg = 85   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=85 deg
max_load_kN_per_m = 800   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=800kN/m
material = "42Cr"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=42Cr
width_at_shoulder_mm = 27.25   # published | https://www.rolleritools.com/upload/immagini/img1200/P.120-85-R1-01-01.jpg | 27.25/1.07"
tip_flank_oblique_dim_mm = 6   # published | https://www.rolleritools.com/upload/immagini/img1200/P.120-85-R1-01-01.jpg | 6/0.24"
lengths_and_weights = "835 mm 13.50; 415 mm 6.70; 795 mm FX 12.90; 805 mm F 19.00 (as printed; out of line with 795 FX 12.90)"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p12085r1-6696.html | Article No. / Length / Weight table

[[punch]]
id = "rolleri/P.97.85.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9785r08-6693.html | P.97.85.R08
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9785r08-6693.html | rolleritools.com product page
part_number = "P.97.85.R08"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9785r08-6693.html | P.97.85.R08
name = "Punch R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | 'Punch R1 European type ...' listing text
clamping = "Rolleri type R1 = European / Promecam style"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | page title 'Tools R1 Type Promecam | European Style'
kind = "gooseneck (small relief)"   # estimate (basis: silhouette of the product drawing) | https://www.rolleritools.com/upload/immagini/img1200/P.97-85-R08-01.jpg | drawing only
height_mm = 66.6   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Working height=66,6mm
total_height_mm = 96.5   # published | https://www.rolleritools.com/upload/immagini/img1200/P.97-85-R08-01.jpg | drawing 96.5 mm overall (top of tang to tip)
tip_radius_mm = 0.8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Radius=0,8mm
tip_angle_deg = 85   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | alpha=85 deg
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Max. load=1000kN/m
material = "C45"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches.html | Material=C45
width_at_shoulder_mm = 26   # published | https://www.rolleritools.com/upload/immagini/img1200/P.97-85-R08-01.jpg | 26/1.02"
tip_flank_oblique_dim_mm = 8.9   # published | https://www.rolleritools.com/upload/immagini/img1200/P.97-85-R08-01.jpg | 8.9/0.35"
lengths_and_weights = "835 mm 11.00; 415 mm 5.50; 795 mm FX 10.40; 805 mm F 10.60"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-punches/p9785r08-6693.html | Article No. / Length / Weight table

[[die]]
id = "rolleri/T80.06.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies/t800688.html | T80.06.88
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.06.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.06.88
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam); T die on a 60 x 18 mm base (drawing)"   # published | https://www.rolleritools.com/upload/immagini/img1200/T80-06-88-TR80-06-88(3).jpg | drawing: base 60/2.36", flange 18/0.71"
v_opening_mm = 6   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=6 (listing data-v attribute / name)
v_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=88 deg
shoulder_radius_mm = 0.4   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=0,4mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
stem_width_mm = 14   # published | https://www.rolleritools.com/upload/immagini/img1200/T80-06-88-TR80-06-88(3).jpg | 14/0.55"
lengths_and_weights = "835 mm 13.00; 415 mm 6.50; 795 mm FX 12.40; 805 mm F 12.50"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies/t800688.html | Article No. / Length / Weight table

[[die]]
id = "rolleri/T80.08.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.08.88
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.08.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.08.88
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | series R1
v_opening_mm = 8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=8 (listing data-v attribute / name)
v_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=88 deg
shoulder_radius_mm = 0.5   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=0,5mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
stem_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | drawing not read
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | detail page not read

[[die]]
id = "rolleri/T80.10.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies/t801088.html | T80.10.88
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.10.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.10.88
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam); T die on a 60 x 18 mm base (drawing)"   # published | https://www.rolleritools.com/upload/immagini/img1200/T80-10-88-TR80-10-88(3).jpg | drawing: base 60/2.36", flange 18/0.71"
v_opening_mm = 10   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=10 (listing data-v attribute / name)
v_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=88 deg
shoulder_radius_mm = 0.8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=0,8mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
stem_width_mm = 18   # published | https://www.rolleritools.com/upload/immagini/img1200/T80-10-88-TR80-10-88(3).jpg | 18/0.71"
lengths_and_weights = "835 mm 14.20; 415 mm 7.10; 795 mm FX 13.50; 805 mm F 13.70"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies/t801088.html | Article No. / Length / Weight table

[[die]]
id = "rolleri/T80.12.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.12.88
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.12.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.12.88
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | series R1
v_opening_mm = 12   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=12 (listing data-v attribute / name)
v_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=88 deg
shoulder_radius_mm = 2.75   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=2,75mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
stem_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | drawing not read
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | detail page not read

[[die]]
id = "rolleri/T80.16.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.16.88
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.16.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.16.88
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | series R1
v_opening_mm = 16   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=16 (listing data-v attribute / name)
v_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=88 deg
shoulder_radius_mm = 2.75   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=2,75mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
stem_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | drawing not read
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | detail page not read

[[die]]
id = "rolleri/T80.20.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.20.88
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.20.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.20.88
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | series R1
v_opening_mm = 20   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=20 (listing data-v attribute / name)
v_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=88 deg
shoulder_radius_mm = 3   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=3mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
stem_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | drawing not read
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | detail page not read

[[die]]
id = "rolleri/T80.25.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies/t802588-6912.html | T80.25.88
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.25.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.25.88
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam); T die on a 60 x 18 mm base (drawing)"   # published | https://www.rolleritools.com/upload/immagini/img1200/T80-25-88(1).jpg | drawing: base 60/2.36", flange 18/0.71"
v_opening_mm = 25   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=25 (listing data-v attribute / name)
v_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=88 deg
shoulder_radius_mm = 3   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=3mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
stem_width_mm = 35   # published | https://www.rolleritools.com/upload/immagini/img1200/T80-25-88(1).jpg | 35/1.38"
lengths_and_weights = "835 mm 20.20; 415 mm 10.00; 795 mm FX 19.20; 805 mm F 19.50"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies/t802588-6912.html | Article No. / Length / Weight table

[[die]]
id = "rolleri/T80.10.85"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.10.85
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.10.85"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.10.85
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | series R1
v_opening_mm = 10   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=10 (listing data-v attribute / name)
v_angle_deg = 85   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=85 deg
shoulder_radius_mm = 2.75   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=2,75mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
stem_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | drawing not read
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | detail page not read

[[die]]
id = "rolleri/T80.06.30"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.06.30
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.06.30"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.06.30
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | series R1
v_opening_mm = 6   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=6 (listing data-v attribute / name)
v_angle_deg = 30   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=30 deg
shoulder_radius_mm = 0.6   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=0,6mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 350   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=350kN/m
stem_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | drawing not read
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | detail page not read

[[die]]
id = "rolleri/T80.10.30"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.10.30
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.10.30"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.10.30
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | series R1
v_opening_mm = 10   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=10 (listing data-v attribute / name)
v_angle_deg = 30   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=30 deg
shoulder_radius_mm = 1   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=1mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 500   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=500kN/m
stem_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | drawing not read
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | detail page not read

[[die]]
id = "rolleri/T80.12.30"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies/t801230-7022.html | T80.12.30
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.12.30"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.12.30
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam); T die on a 60 x 18 mm base (drawing)"   # published | https://www.rolleritools.com/upload/immagini/img1200/T80-12-30.jpg | drawing: base 60/2.36", flange 18/0.71"
v_opening_mm = 12   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=12 (listing data-v attribute / name)
v_angle_deg = 30   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=30 deg
shoulder_radius_mm = 1.5   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=1,5mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 400   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=400kN/m
stem_width_mm = 24   # published | https://www.rolleritools.com/upload/immagini/img1200/T80-12-30.jpg | 24/0.94"
lengths_and_weights = "835 mm 16.00; 415 mm 8.00; 795 mm FX 15.20; 805 mm F 15.40"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies/t801230-7022.html | Article No. / Length / Weight table

[[die]]
id = "rolleri/T80.20.30"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.20.30
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "T80.20.30"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T80.20.30
name = "T die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | series R1
v_opening_mm = 20   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=20 (listing data-v attribute / name)
v_angle_deg = 30   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=30 deg
shoulder_radius_mm = 2.5   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=2,5mm
height_mm = 80   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=80mm
max_load_kN_per_m = 500   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=500kN/m
stem_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | drawing not read
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | detail page not read

[[die]]
id = "rolleri/M60.85.32"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | M60.85.32
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "M60.85.32"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | M60.85.32
name = "1-V die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | series R1
v_opening_mm = 32   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=32 (listing data-v attribute / name)
v_angle_deg = 85   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=85 deg
shoulder_radius_mm = 4   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=4mm
height_mm = 60   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=60mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
die_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | drawing not read
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | detail page not read

[[die]]
id = "rolleri/M60.85.40"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies/m608540-7048.html | M60.85.40
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "M60.85.40"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | M60.85.40
name = "1-V die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/upload/immagini/img1200/M60-85-40.jpg | series R1
v_opening_mm = 40   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=40 (listing data-v attribute / name)
v_angle_deg = 85   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=85 deg
shoulder_radius_mm = 4   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=4mm
height_mm = 60   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=60mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
die_width_mm = 60   # published | https://www.rolleritools.com/upload/immagini/img1200/M60-85-40.jpg | 60/2.36"
v_dim_on_drawing_mm = 40   # published | https://www.rolleritools.com/upload/immagini/img1200/M60-85-40.jpg | 40/1.57"
lengths_and_weights = "835 mm 20.00; 415 mm 10.50; 795 mm FX 19.00; 805 mm F 19.00"   # published (weight unit not printed; kg implied) | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies/m608540-7048.html | Article No. / Length / Weight table

[[die]]
id = "rolleri/M60.85.50"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | M60.85.50
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "M60.85.50"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | M60.85.50
name = "1-V die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | series R1
v_opening_mm = 50   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=50 (listing data-v attribute / name)
v_angle_deg = 85   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=85 deg
shoulder_radius_mm = 4   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=4mm
height_mm = 60   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=60mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
die_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | drawing not read
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | detail page not read

[[die]]
id = "rolleri/AMR60.06.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | AMR60.06.88
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | rolleritools.com product listing
part_number = "AMR60.06.88"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | AMR60.06.88
name = "Self centering 1-V die R1 European type"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing text
clamping = "Rolleri type R1 (European/Promecam)"   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | series R1
v_opening_mm = 6   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | V=6 (listing data-v attribute / name)
v_angle_deg = 88   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | alpha=88 deg
shoulder_radius_mm = 0.8   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Radius=0,8mm
height_mm = 60   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | height=60mm
max_load_kN_per_m = 1000   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | Max. load=1000kN/m
die_width_mm = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | drawing not read
lengths_and_weights = "UNKNOWN"   # unknown | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | detail page not read

[[die]]
id = "rolleri/T120.08.85"   # conflict | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T120.08.85
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | listing
conflict_note = "name says 85 deg V8; listing text says 'alpha=35 deg, Radius=1mm, Max. load=350kN/m' while sibling T120.xx.85 dies read 85 deg, R2.75, 1000 kN/m"   # conflict | https://www.rolleritools.com/tools-6437/press-brakes/tools/type-r1-dies.html | T die R1 European type with height=120mm, alpha=35 deg, Radius=1mm, Material=C45, Max. load=350kN/m

[[accessory]]
id = "rolleri/horn-R1_SCA_1"   # published | https://www.rolleritools.com/upload/immagini/img1200/R1_SCA_1-01.jpg | horn (ear) drawing shown on R1 punch pages
maker = "Rolleri S.p.A."   # published | https://www.rolleritools.com/upload/immagini/img1200/R1_SCA_1-01.jpg | drawing
horn_dims = "relief 20 mm wide, R25 corner, 9 mm step"   # published | https://www.rolleritools.com/upload/immagini/img1200/R1_SCA_1-01.jpg | 20/0.79", R 25/0.98", 9/0.35"

```

### 3.4 Mate Precision Tooling (European Precision Style catalogue, 2015)

```
[[punch]]
id = "mate/11.100"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 11.100
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | LIT01119 Rev C PN 2015
part_number = "11.100"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 11.100
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "straight (relieved)"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | drawing only
height_mm = 66.6   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | H 66.6 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | R 0.8
tip_angle_deg = 90   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | alpha 90 deg
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | T/mt 100
width_at_shoulder_mm = 26   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 26
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | not stated

[[punch]]
id = "mate/11.145"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 11.145
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | LIT01119 Rev C PN 2015
part_number = "11.145"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 11.145
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "straight (relieved)"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | drawing only
height_mm = 66.5   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | H 66.5 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | R 0.8
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | alpha 88 deg
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | T/mt 100
width_at_shoulder_mm = 26   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 26
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | not stated

[[punch]]
id = "mate/11.101"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 11.101
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | LIT01119 Rev C PN 2015
part_number = "11.101"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 11.101
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "straight (relieved)"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | drawing only
height_mm = 65.45   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | H 65.45 (Legend: H = Height)
tip_radius_mm = 3.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | R 3.0
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | alpha 88 deg
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | T/mt 100
width_at_shoulder_mm = 26   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 26
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | not stated

[[punch]]
id = "mate/11.275"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 11.275
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | LIT01119 Rev C PN 2015
part_number = "11.275"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 11.275
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "straight (relieved)"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | drawing only
height_mm = 67.6   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | H 67.6 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | R 0.8
tip_angle_deg = 75   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | alpha 75 deg
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | T/mt 100
width_at_shoulder_mm = 26   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 26
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | not stated

[[punch]]
id = "mate/10.176"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 10.176
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | LIT01119 Rev C PN 2015
part_number = "10.176"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 10.176
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "gooseneck (small relief)"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | drawing only
height_mm = 66.74   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | H 66.74 (Legend: H = Height)
tip_radius_mm = 0.6   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | R 0.6
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | alpha 88 deg
max_load_t_per_m = 35   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | T/mt 35
width_at_shoulder_mm = 26   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 26
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | not stated

[[punch]]
id = "mate/11.870"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 11.870
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | LIT01119 Rev C PN 2015
part_number = "11.870"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | 11.870
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | drawing only
height_mm = 135.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | H 135.0 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | R 0.8
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | alpha 88 deg
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | T/mt 100
width_at_shoulder_mm = 26   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | 26
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.6) | not stated

[[punch]]
id = "mate/11.228"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | 11.228
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | LIT01119 Rev C PN 2015
part_number = "11.228"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | 11.228
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "straight (relieved)"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | drawing only
height_mm = 105.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | H 105.0 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | R 0.8
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | alpha 88 deg
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | T/mt 100
width_at_shoulder_mm = 26   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.7) | 26
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | not stated

[[punch]]
id = "mate/10.262"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | 10.262
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | LIT01119 Rev C PN 2015
part_number = "10.262"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | 10.262
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "straight"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | drawing only
height_mm = 100.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | H 100.0 (Legend: H = Height)
tip_radius_mm = 0.6   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | R 0.6
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | alpha 88 deg
max_load_t_per_m = 50   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | T/mt 50
width_at_shoulder_mm = 26   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.7) | 26
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | not stated

[[punch]]
id = "mate/11.260"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | 11.260
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | LIT01119 Rev C PN 2015
part_number = "11.260"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | 11.260
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "narrow"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | drawing only
height_mm = 95.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | H 95.0 (Legend: H = Height)
tip_radius_mm = 0.6   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | R 0.6
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | alpha 88 deg
max_load_t_per_m = 50   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | T/mt 50
width_at_shoulder_mm = 27   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.7) | 27
blade_width_at_tip_mm = 6   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.7) | 6
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.7) | not stated

[[punch]]
id = "mate/10.504"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | 10.504
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | LIT01119 Rev C PN 2015
part_number = "10.504"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | 10.504
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | drawing only
height_mm = 104.5   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | H 104.5 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | R 0.8
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | alpha 88 deg
max_load_t_per_m = 50   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | T/mt 50
width_behind_tip_mm = 56   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.8) | 56
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | not stated

[[punch]]
id = "mate/10.502"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | 10.502
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | LIT01119 Rev C PN 2015
part_number = "10.502"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | 10.502
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | drawing only
height_mm = 89.7   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | H 89.7 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | R 0.8
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | alpha 88 deg
max_load_t_per_m = 50   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | T/mt 50
width_behind_tip_mm = 53   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.8) | 53
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | not stated

[[punch]]
id = "mate/11.147"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | 11.147
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | LIT01119 Rev C PN 2015
part_number = "11.147"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | 11.147
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | drawing only
height_mm = 104.5   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | H 104.5 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | R 0.8
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | alpha 88 deg
max_load_t_per_m = 50   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | T/mt 50
width_behind_tip_mm = 56   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.8) | 56
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | not stated

[[punch]]
id = "mate/11.146"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | 11.146
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | LIT01119 Rev C PN 2015
part_number = "11.146"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | 11.146
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "gooseneck"   # estimate (basis: silhouette of the catalogue drawing) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | drawing only
height_mm = 89.58   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | H 89.58 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | R 0.8
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | alpha 88 deg
max_load_t_per_m = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | T/mt 60
width_behind_tip_mm = 56   # published (as drawn) | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.8) | 56
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.8) | not stated

[[punch]]
id = "mate/10.505"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.11) | 10.505
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.11) | LIT01119 Rev C PN 2015
part_number = "10.505"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.11) | 10.505
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.11) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.11) | drawing not examined
height_mm = 120.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.11) | H 120.0 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.11) | R 0.8
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.11) | alpha 88 deg
max_load_t_per_m = 50   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.11) | T/mt 50
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.11) | not stated

[[punch]]
id = "mate/11.149"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.12) | 11.149
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.12) | LIT01119 Rev C PN 2015
part_number = "11.149"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.12) | 11.149
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.12) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.12) | drawing not examined
height_mm = 165.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.12) | H 165.0 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.12) | R 0.8
tip_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.12) | alpha 88 deg
max_load_t_per_m = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.12) | T/mt 60
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.12) | not stated

[[punch]]
id = "mate/11.840"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.9) | 11.840
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.9) | LIT01119 Rev C PN 2015
part_number = "11.840"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.9) | 11.840
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.9) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.9) | drawing not examined
height_mm = 185.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.9) | H 185.0 (Legend: H = Height)
tip_radius_mm = 1.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.9) | R 1.0
tip_angle_deg = 85   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.9) | alpha 85 deg
max_load_t_per_m = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.9) | T/mt 60
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.9) | not stated

[[punch]]
id = "mate/10.142"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.14) | 10.142
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.14) | LIT01119 Rev C PN 2015
part_number = "10.142"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.14) | 10.142
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.14) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.14) | drawing not examined
height_mm = 67.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.14) | H 67.0 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.14) | R 0.8
tip_angle_deg = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.14) | alpha 60 deg
max_load_t_per_m = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.14) | T/mt 60
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.14) | not stated

[[punch]]
id = "mate/11.134"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | 11.134
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | LIT01119 Rev C PN 2015
part_number = "11.134"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | 11.134
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | drawing not examined
height_mm = 66.3   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | H 66.3 (Legend: H = Height)
tip_radius_mm = 0.6   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | R 0.6
tip_angle_deg = 45   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | alpha 45 deg
max_load_t_per_m = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | T/mt 80
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | not stated

[[punch]]
id = "mate/10.514"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | 10.514
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | LIT01119 Rev C PN 2015
part_number = "10.514"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | 10.514
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | drawing not examined
height_mm = 80.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | H 80.0 (Legend: H = Height)
tip_radius_mm = 0.6   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | R 0.6
tip_angle_deg = 30   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | alpha 30 deg
max_load_t_per_m = 50   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | T/mt 50
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.16) | not stated

[[punch]]
id = "mate/11.158"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.17) | 11.158
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.17) | LIT01119 Rev C PN 2015
part_number = "11.158"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.17) | 11.158
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.17) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.17) | drawing not examined
height_mm = 80.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.17) | H 80.0 (Legend: H = Height)
tip_radius_mm = 0.6   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.17) | R 0.6
tip_angle_deg = 30   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.17) | alpha 30 deg
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.17) | T/mt 100
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.17) | not stated

[[punch]]
id = "mate/11.540"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.19) | 11.540
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.19) | LIT01119 Rev C PN 2015
part_number = "11.540"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.19) | 11.540
name = "European Precision Style punch"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.19) | catalogue title
clamping = "European Precision Style (Promecam-type) tang with safety groove"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.6) | SAFETY GROOVE DATA drawing (see clamping section)
kind = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.19) | drawing not examined
height_mm = 117.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.19) | H 117.0 (Legend: H = Height)
tip_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.19) | R 0.8
tip_angle_deg = 26   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.19) | alpha 26 deg
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.19) | T/mt 100
lengths = "415 mm (M), 835 mm (S), 805 mm sectioned (F: 370+100+100+100+50+40+20+15+10, horns)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.5) | HALF LENGTH (M) 415, STANDARD (S) 835, SECTIONED (F) 805
weight = "UNKNOWN"   # unknown | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.19) | not stated

[[die]]
id = "mate/20.806"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.806
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.806"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.806
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 6   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 6
v_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 88 deg
shoulder_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 0,8 (Legend R = radius)
v_bottom = "R 0.5"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 0,5 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 14   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 14
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 100
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

[[die]]
id = "mate/20.808"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.808
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.808"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.808
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 8
v_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 88 deg
shoulder_radius_mm = 1.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 1,0 (Legend R = radius)
v_bottom = "R 0.5"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 0,5 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 14   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 14
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 100
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

[[die]]
id = "mate/20.810"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.810
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.810"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.810
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 10   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 10
v_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 88 deg
shoulder_radius_mm = 1.2   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 1,2 (Legend R = radius)
v_bottom = "R 0.5"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 0,5 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 18   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 18
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 100
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

[[die]]
id = "mate/20.371"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.371
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.371"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.371
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 12   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 12
v_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 88 deg
shoulder_radius_mm = 1.5   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 1,5 (Legend R = radius)
v_bottom = "R 0.5"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 0,5 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 18   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 18
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 100
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

[[die]]
id = "mate/20.327"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.327
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.327"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.327
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 16   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 16
v_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 88 deg
shoulder_radius_mm = 2.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 2,0 (Legend R = radius)
v_bottom = "R 0.5"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 0,5 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 24   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 24
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 100
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

[[die]]
id = "mate/20.310"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.310
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.310"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.310
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 20   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 20
v_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 88 deg
shoulder_radius_mm = 2.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 2,0 (Legend R = radius)
v_bottom = "R 1.0"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 1,0 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 30   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 30
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 100
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

[[die]]
id = "mate/20.309"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.309
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.309"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.309
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 25   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 25
v_angle_deg = 88   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 88 deg
shoulder_radius_mm = 3.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 3,0 (Legend R = radius)
v_bottom = "R 1.0"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 1,0 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 35   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 35
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 100
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

[[die]]
id = "mate/20.243"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.243
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.243"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.243
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 6   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 6
v_angle_deg = 90   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 90 deg
shoulder_radius_mm = 0.4   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 0,4 (Legend R = radius)
v_bottom = "R 0.5"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 0,5 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 14   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 14
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 100
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

[[die]]
id = "mate/20.510"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.510
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.510"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.510
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 10   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 10
v_angle_deg = 85   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 85 deg
shoulder_radius_mm = 2.5   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 2,5 (Legend R = radius)
v_bottom = "R 0.5"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 0,5 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 18   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 18
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 100
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

[[die]]
id = "mate/20.525"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.525
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.525"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.525
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 25   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 25
v_angle_deg = 85   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 85 deg
shoulder_radius_mm = 5.0   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 5,0 (Legend R = radius)
v_bottom = "R 1.0"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 1,0 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 35   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 35
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 100   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 100
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

[[die]]
id = "mate/20.300"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.300
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.300"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.300
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 12   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 12
v_angle_deg = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 60 deg
shoulder_radius_mm = 0.8   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 0,8 (Legend R = radius)
v_bottom = "R 0.5"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 0,5 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 18   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 18
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 60
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

[[die]]
id = "mate/20.331"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.331
maker = "Mate Precision Tooling"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | LIT01119 Rev C PN 2015
part_number = "20.331"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 20.331
name = "Pillar die H80"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | PILLAR DIES H80
clamping = "European Precision Style; pillar on a 60 mm wide base, 18 mm base flange (as drawn)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | drawing dims 60 and 18
v_opening_mm = 12   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | V 12
v_angle_deg = 45   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | alpha 45 deg
shoulder_radius_mm = 1.5   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | R 1,5 (Legend R = radius)
v_bottom = "R 0.8"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | F R 0,8 (Legend F = bottom)
height_mm = 80   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | H = 80
pillar_width_mm = 24   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | B 24
base_width_mm = 60   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | 60 (drawing)
max_load_t_per_m = 50   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (catalogue p.38) | T/m 50
lengths = "415 mm, 835 mm, 805 mm sectioned (370+200+100+50+40+20+15+10)"   # published | https://www.mate.com/wp-content/uploads/2015/10/LIT01119_PB_EPS_PN_2015_REV_C_LR.pdf (p.29, p.38) | 415 mm 835 mm 805 mm

```

## 4. Not reachable / not collected, and why

- Rolleri Bending Catalogue 2024 (PDF): gated behind a form asking for company, name and e-mail; not submitted (no sign-ups). The open online catalogue (rolleritools.com listings, detail pages and drawings) was used instead and is cited per value.
- Wilson Tool 'Express Bend' and WT/WILA-style (New Standard style) catalogues: not fetched in the time available; only the European-style and American Precision catalogues were read. The my-account web shop (myaccount.wilsontool.com) was not used.
- WILA top-tool adaption (tang) dimensions: not dimensioned in the 2024 USA catalogue; only the bottom 13 mm adaption and 13.5 x 22 slot are stated.
- WILA American Vintage (pp.90-99), the remaining NS Premium dies (pp.57-60, 62-63, 66-70), NS Pro dies (pp.74-75) and American Style dies (pp.82-89): in the PDF, not transcribed (time). American Style dies p.82 restate the NS VI V6..V12 30 deg dies with the same numbers.
- Mate: only the 2015 European Precision Style PDF was read; Mate's WILA/Trumpf-style and American-style catalogues and mate.com DXF/PDF profile files were not fetched. Mate 1V dies (pp.35-37): V values ambiguous in the text layer; not transcribed. Mate punch kinds not examined for pages 9-19.
- Euromaquina and other Promecam-style generic makers: not attempted (time).
- Tonnage grids (Wilson p.141, Mate p.4): rule rows transcribed; grid cells were not, because the text layer does not keep them aligned to their V columns.
- Body width at clamp and gooseneck throat height: not dimensioned for most tools in any source read; recorded UNKNOWN.

Conflicts recorded in the blocks: WILA OZU-024 inch label, OZU-021 inch rounding; Wilson A4735 H inch value, A4706 weight decimal, A7403 width inch vs mm, A5400 width vs V, 51026 height inch vs mm, 50166 H ordering; Wilson tang height 27.97 vs Mate/Rolleri 30; Rolleri TOP.175.85.R2 radius, T120.08.85 angle/radius/load, P.120.85.R1 805 F weight. Not given a block: Wilson 44206 (75 deg 1V die) W printed 1.260 [30.0] although 1.260 in = 32 mm; Wilson 42114 W .69 [17.0] (0.69 in = 17.5 mm).
