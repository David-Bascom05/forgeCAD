# Vendor press-brake tooling (Trumpf, Amada, Bystronic) - collected 2026-11-24

All values restated from the makers' openly published catalogues; numbers only, prose not copied.
Status per field: published / estimate (basis) / unknown / conflict (note). 'published (drawing ...)' = a number printed on
the catalogue drawing whose reference lines were read by eye; the number is the maker's, the attribution to a feature is ours.
Decimal commas in Trumpf statements are the catalogue's own ('R1,0 mm' = 1.0 mm).

## 1. Sources

| # | URL | What | Date read | Fetch |
|---|-----|------|-----------|-------|
| 1 | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools / ACB tool catalog, Edition 01/2025, Mat.No. 2968160, 324 pp. (EN). Main Trumpf source | 2026-11-24 | ok (curl, default UA): 200, PDF 16.5 MB |
| 2 | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EHT-DE.pdf | TRUMPF EHT bending tools catalog (DE); downloaded, not used for blocks | 2026-11-24 | ok |
| 3 | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-TruBend-Cell-7000-DE.pdf | TRUMPF TruBend Cell 7000 tool catalog (DE); downloaded, not used | 2026-11-24 | ok |
| 4 | https://online.flippingbook.com/view/795609261/ | AMADA Tooling Catalogue 2023, 166 pp., Amada's own FlippingBook publication; the viewer pages carry a text layer (codes, angles, radii, t/m) but the drawings are images | 2026-11-24 | ok for text layer (viewer HTML, pages 1-166); page images are behind signed URLs - not fetched |
| 5 | https://online.flippingbook.com/view/772006724/240/ | Amada Product Handbook (FlippingBook) p.240: air-bending tonnage chart + V-width rule | 2026-11-24 | ok (text layer) |
| 6 | https://cdn.amada.eu/fileadmin/user_upload/NEWAFHStandard_Complete.pdf | Amada NEW AFH Standard catalogue PDF | 2026-11-24 | REFUSED: HTTP 403 Cloudflare 'Attention Required' (curl and WebFetch); not bypassed |
| 7 | https://cdn.amada.eu/fileadmin/Repository/Products/Brochures/Catalog_AFH_EN_2021.pdf | Amada AFH catalogue 2021 PDF | 2026-11-24 | REFUSED: 403 Cloudflare |
| 8 | https://cdn.amada.eu/fileadmin/user_upload/price-lists/AMADA_Tooling_Price_List__-_Bending.pdf | Amada bending tooling price list PDF | 2026-11-24 | REFUSED: 403 Cloudflare |
| 9 | https://www.amada.eu/fileadmin/user_upload/HighTonnage_Catalog.pdf | Amada high-tonnage catalogue PDF | 2026-11-24 | REFUSED: 403 Cloudflare |
| 10 | https://www.amada.eu/uk-en/products/tooling/bending-tools/deep-bending-tooling-solutions/afh-amada-fixed-height-punches/ | Amada AFH punches web page | 2026-11-24 | REFUSED: 403 (WebFetch) |
| 11 | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (EN, file 2502436, created 2026-05-19), 176 pp. RF-A upper/bottom tools | 2026-11-24 | ok: 200, PDF 4.0 MB |
| 12 | https://www.accurpress.com/tooling/ | Accurpress tooling page | 2026-11-24 | ok, but no dimensional data or catalogue published (directs to phone) |
| 13 | https://www.lvdgroup.com/en/bending/press-brake-tooling | LVD tooling (guessed URL) | 2026-11-24 | 404; web search found no LVD-published tooling catalogue |

## 2. Clamping systems and general rules (cited)

```
trumpf_clamp = "head-bearing tools (OW/K) and shoulder-bearing tools (OW/S); Modufix clamp max surface pressure 2500 kN/m head-bearing, 3000 kN/m shoulder-bearing (3000-6000 kN/m 8000 series)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28
trumpf_tang_dims = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38-116) | tang drawn on every punch drawing but never dimensioned
trumpf_die_width_rule = "s 0.5-2.6 mm: w = 6 x s; 3.0-8.0: 8 x s; 9.0-12.0: 10 x s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.300) | 'Sheet thickness s / Die width w' table
trumpf_die_selection_table = "s:min/opt/max w (mm): 0.5:4/4/8 0.75:4/6/12 1:5/8/16 1.25:6/10/20 1.5:6/12/20 1.75:8/12/24 2:10/16/24 2.5:12/20/30 3:16/24/40 3.5:16/30/50 4:20/30/60 4.5:20/40/70 5:30/40/80 6:30/50/90 7:40/60/100 8:50/70/120 10:70/80/150 12:80/100/150 15:90/120/150"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.179) | selection of the die width table
trumpf_min_flange = "minimum side length ~ 0.75 x die width"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.179) | 'Minimum side length = ~ 0.75 x die width'
trumpf_coining_force = "coining force at least 3x air-bending force"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.178) | p.178 die 90°
trumpf_force_table = "90° air bending, 450 N/mm2 steel, kN per m; e.g. s=1/W8: 91; s=2/W16: 164; s=3/W24: 238; s=6/W50: 443; punch radii assumed W4-5 R0,5, W6-40 R1, W50-150 R4"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.301) | press force table (recommended values); also 700 N/mm2 stainless table
trumpf_box_height = "SHI = (OWH - B)/1.414 x 0.95; OWH 120->64, 140->78, 160->91, 220->132, 300->185, 400->253 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.299) | box internal height table
amada_v_rule = "t 0.5-2.5: V = 6t; 3.0-8.0: 8t; 9.0-10.0: 10t; >=12: 12t"   # published | https://online.flippingbook.com/view/795609261/112/ and https://online.flippingbook.com/view/772006724/240/ | V-width relationship table
amada_tonnage_chart = "mild steel 45-50 kg/mm2, tonnes per 1 m; V/b/ir header e.g. V6: b 4, ir 1; V8: b 5.5, ir 1.3; V12: b 8.5, ir 2; V25: b 17.5, ir 4; V50: b 35, ir 8"   # published | https://online.flippingbook.com/view/772006724/240/ | Air bending tonnage chart header rows V / b / ir
amada_tang_dims = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/33/ | distance pieces ZI/ZII page lists numbers 13, 20, 7, 100, 120, 50 without readable labels
bystronic_rf_a = "RF-A tang 20 mm wide, 62 mm tang height, clamping height 60 mm; head support <=1600 kN/m, shoulder >3200 kN/m; max tool weight 50 kg/m. RF-A Pro: 1800 kN/m head; RF-A HD: 2500 head / 3200 shoulder"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8-10) | Clamping RF-A pages
bystronic_die_insert = "bottom tools: die insert 13 mm, tang 20 mm high (drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | 'Die insert 13 mm'; drawing 13 / 20
bystronic_v_rule = "t 0.5-2.5: 6t; 3-8: 8t; 9-10: 10t; >12: 12t; F = Rm x t^2 x (1 + 4t/V)/V ... (formula as printed, reference data); Al x0.65, stainless x1.6"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.6) | Calculation formulas
bystronic_force_table = "Rm 420 N/mm2, kN per m; e.g. t1/V6: 115 (a=5), t2/V12: 235? -> see note"   # published (cell alignment of the extracted table uncertain: verify on page) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.4) | capacity chart for air bending; internal radius ~16% of V (p.5)
bystronic_min_inner_radius = "t:minR 1:1 2:2 3:3 4:5 5:6 6:8 8:12 10:16 12:20 16:28 18:36 20:40"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.6) | conversion for minimal internal radii
```

## 3. Tools

### Trumpf punches

[[punch]]
id = "trumpf/OW200-K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | catalogue type OW200/K
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW200/K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | header 'OW200/K'
clamping = "TRUMPF head-bearing (OW/K), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "gooseneck ('highly offset', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 120   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | 'Height: 120 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | 'Radius: R1,0 mm'
tip_angle_deg = 86.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | 'Angle: 86°'
max_load = "800 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | technical data load icon: 800 kN/m
weight_kg_per_m = 28.9   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | '28,9 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 64.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | '64 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "10 mm - 300 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | '10 mm - 300 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
dim_body_overall_mm = 55   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.40) | horizontal 55: rear body face to front face of tang/clamp line (read from drawing)
neck_thickness_mm = 23   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.40) | horizontal 23 across the neck
tip_flank_length_mm = 8   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.40) | dimension 8 along the tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.40) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.40) | tang drawn but not dimensioned
segments_mat_no = "500:0823781 300:0823780 200:0823779 100:0824474 HornL100:0824472 HornR100:0824473 50:0824471 45:0824470 40:0824469 35:0824468 30:0824467 25:0824466 20:0838231 15:0838230 10:0838229"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050/3050/2550/2050/1250/1050; VAR.A 2050/1250/1050/550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.
notes = "more radii p.39 (radius:height mm): R0,5:119,8 R1,6:119,2 R2:119,2 R2,3:119 R2,5:119 R3:118,7 R3,2:118,6 R4:118,3 R5:117,8 R6:117,3; horn drawing p.38: horn 100, 31 and 30, 30°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.38) | more radii p.39 (radius:height mm): R0,5:119,8 R1,6:119,2 R2

[[punch]]
id = "trumpf/OW280-K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | catalogue type OW280/K
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW280/K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | header 'OW280/K'
clamping = "TRUMPF head-bearing (OW/K), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "gooseneck ('highly offset', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 140   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | 'Height: 140 mm'
tip_radius_mm = 0.5   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | 'Radius: R0,5 mm'
tip_angle_deg = 80.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | 'Angle: 80°'
max_load = "400 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | technical data load icon: 400 kN/m
weight_kg_per_m = 30.8   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | '30,8 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 78.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | '78 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "10 mm - 300 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | '10 mm - 300 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
dim_body_overall_mm = 55   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.43) | horizontal 55 rear body face to tang front line
neck_thickness_mm = 21   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.43) | horizontal 21 across neck
tip_flank_length_mm = 5   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.43) | dimension 5 along tip flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.43) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.43) | tang drawn but not dimensioned
segments_mat_no = "500:0994464 300:0994463 200:0994462 100:0994465 50:0994471 45:0994470 40:0994469 35:0994468 30:0994467 25:0994466 20:1465755 15:1465754 10:1293086"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250; SET 150"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.42) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW223-K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | catalogue type OW223/K
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW223/K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | header 'OW223/K'
clamping = "TRUMPF head-bearing (OW/K), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "gooseneck ('highly offset', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 160   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | 'Height: 160 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | 'Radius: R1,0 mm'
tip_angle_deg = 86.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | 'Angle: 86°'
max_load = "430 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | technical data load icon: 430 kN/m
weight_kg_per_m = 34.4   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | '34,4 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 91.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | '91 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "10 mm - 300 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | '10 mm - 300 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
neck_thickness_mm = 22   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.47) | horizontal 22 rear body face to throat
throat_depth_mm = 23   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.47) | horizontal 23 from throat to tang centre line (tip lies on that line in the drawing)
dim_body_overall_mm = 55   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.47) | horizontal 55 rear face to tang front line
tip_flank_length_mm = 5   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.47) | dimension 5 along flank
dim_tip_offset_mm = 4   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.47) | horizontal 4 near tip, reference line unclear
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.47) | tang drawn but not dimensioned
segments_mat_no = "500:2895275 300:2895274 200:2895273 100:2895269 50:2895268 45:2895267 40:2895266 35:2895265 30:2895264 25:2895263 20:2895272 15:2895271 10:2895270"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.46) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW243-K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | catalogue type OW243/K
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW243/K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | header 'OW243/K'
clamping = "TRUMPF head-bearing (OW/K), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "gooseneck ('highly offset', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 160   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | 'Height: 160 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | 'Radius: R1,0 mm'
tip_angle_deg = 86.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | 'Angle: 86°'
max_load = "430 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | technical data load icon: 430 kN/m
weight_kg_per_m = 43.2   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | '43,2 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 91.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | '91 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "10 mm - 300 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | '10 mm - 300 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
neck_thickness_mm = 27   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.49) | horizontal 27 rear face to throat
throat_depth_mm = 43   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.49) | horizontal 43 throat to tang centre line
dim_body_overall_mm = 80   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.49) | horizontal 80 rear face to tang front line
tip_flank_length_mm = 5   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.49) | dimension 5 along flank
dim_tip_offset_mm = 4   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.49) | horizontal 4 near tip, reference unclear
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.49) | tang drawn but not dimensioned
segments_mat_no = "500:2894434 300:2894433 200:2894432 100:2894428 HornL100:2894436 HornR100:2894437 50:2894427 45:2894426 40:2894425 35:2894424 30:2894423 25:2894422 20:2894431 15:2894430 10:2894429"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.48) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW255-K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | catalogue type OW255/K
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW255/K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | header 'OW255/K'
clamping = "TRUMPF head-bearing (OW/K), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "gooseneck, acute ('highly offset', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 160   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | 'Height: 160 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | 'Radius: R1,0 mm'
tip_angle_deg = 55.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | 'Angle: 55°'
max_load = "700 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | technical data load icon: 700 kN/m
weight_kg_per_m = 34.7   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | '34,7 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 91.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | '91 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "10 mm - 300 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | '10 mm - 300 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
dim_body_overall_mm = 70   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.51) | horizontal 70 rear face to tang front line (upper body)
throat_depth_mm = 44.3   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.51) | horizontal 44.3 throat to tang front line
tip_flank_length_mm = 6   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.51) | dimension 6 along flank
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.51) | tang drawn but not dimensioned
segments_mat_no = "500:2894987 300:2894986 200:2894985 100:2894981 50:2894980 45:2894979 40:2894978 35:2894977 30:2894976 25:2894975 20:2894984 15:2894983 10:2894982"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.50) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW201-K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | catalogue type OW201/K
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW201/K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | header 'OW201/K'
clamping = "TRUMPF head-bearing (OW/K), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "straight, slightly offset ('slightly offset', p.11)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 120   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | 'Height: 120 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | 'Radius: R1,0 mm'
tip_angle_deg = 86.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | 'Angle: 86°'
max_load = "800 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | technical data load icon: 800 kN/m
weight_kg_per_m = 24.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | '24,0 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 64.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | '64 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "10 mm - 500 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | '10 mm - 500 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
body_width_mm = 30   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.54) | horizontal 30 body width
front_relief_mm = 12   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.54) | horizontal 12: front face set back from tang front line
tip_flank_length_mm = 10   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.54) | dimension 10 along flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.54) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.54) | tang drawn but not dimensioned
segments_mat_no = "500:0823908 300:0823907 200:0823905 100:0824548 50:0824539 45:0824537 40:0824534 35:0824531 30:0824525 25:0824523 20:0855800 15:0855799 10:0855798"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.
notes = "OW201/K H120 R1 also on https://trumpf.com/en_US/landing-pages/us/trubend-2100/tooling per search snippet (not fetched)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.52) | OW201/K H120 R1 also on https://trumpf.com/en_US/landing-pag

[[punch]]
id = "trumpf/OW202-K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | catalogue type OW202/K
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW202/K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | header 'OW202/K'
clamping = "TRUMPF head-bearing (OW/K), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "straight acute ('pointed', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 120   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | 'Height: 120 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | 'Radius: R1,0 mm'
tip_angle_deg = 28.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | 'Angle: 28°'
max_load = "600 kN/m; 800 kN/m (second value, shown with a hemming/flattening icon: interpretation)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | technical data load icon: 600 kN/m / 800 kN/m (second value, shown with a hemming/flattening icon: interpretation)
weight_kg_per_m = 19.5   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | '19,5 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 64.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | '64 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "10 mm - 500 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | '10 mm - 500 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
dim_a_mm = 14   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.58) | horizontal 14 in upper body (reference lines unclear)
dim_b_mm = 10   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.58) | horizontal 10 from centre line to tang front line (reading)
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.58) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.58) | tang drawn but not dimensioned
segments_mat_no = "500:0823820 300:0823819 200:0823818 100:0824603 HornL100:0824599 HornR100:0824601 50:0824598 45:0824597 40:0824596 35:0824595 30:0824594 25:0824593 20:0827316 15:0838227 10:0838228"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.56) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW222-K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | catalogue type OW222/K
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW222/K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | header 'OW222/K'
clamping = "TRUMPF head-bearing (OW/K), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "straight acute ('pointed', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 166   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | 'Height: 166 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | 'Radius: R1,0 mm'
tip_angle_deg = 20.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | 'Angle: 20°'
max_load = "600 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | technical data load icon: 600 kN/m
weight_kg_per_m = 22.9   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | '22,9 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 95.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | '95 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "10 mm - 500 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | '10 mm - 500 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
body_width_mm = 24   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.61) | horizontal 24 upper body
front_relief_mm = 10   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.61) | horizontal 10 front face to tang front line
tip_relief_height_mm = 20   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.61) | vertical 20 tip step height
tip_blade_mm = 2   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.61) | horizontal 2 at tip
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.61) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.61) | tang drawn but not dimensioned
segments_mat_no = "500:2895454 300:2895453 200:2895452 100:2895448 50:2895447 45:2895446 40:2895445 35:2895444 30:2895443 25:2895442 20:2895451 15:2895450 10:2895449"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.60) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW282-K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | catalogue type OW282/K
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW282/K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | header 'OW282/K'
clamping = "TRUMPF head-bearing (OW/K), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "straight acute ('pointed', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 166   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | 'Height: 166 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | 'Radius: R1,0 mm'
tip_angle_deg = 28.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | 'Angle: 28°'
max_load = "600 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | technical data load icon: 600 kN/m
weight_kg_per_m = 16.6   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | '16,6 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 95.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | '95 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "10 mm - 500 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | '10 mm - 500 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
blade_thickness_mm = 6   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.63) | horizontal 6 lower blade
dim_a_mm = 1   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.63) | horizontal 1 near tang centre line
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.63) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.63) | tang drawn but not dimensioned
segments_mat_no = "500:2894657 300:2894656 200:2894655 100:2894651 50:2894650 45:2894649 40:2894648 35:2894647 30:2894646 25:2894645 20:2894654 15:2894653 10:2894652"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.62) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW203-K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | catalogue type OW203/K
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW203/K"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | header 'OW203/K'
clamping = "TRUMPF head-bearing (OW/K), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "straight ('even', p.11)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 120   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | 'Height: 120 mm'
tip_radius_mm = 4.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | 'Radius: R4,0 mm'
tip_angle_deg = 60.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | 'Angle: 60°'
max_load = "1800 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | technical data load icon: 1800 kN/m
weight_kg_per_m = 23.8   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | '23,8 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 64.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | '64 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "10 mm - 500 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | '10 mm - 500 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
body_width_mm = 20   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.66) | horizontal 20 body width (same as tang)
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.66) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.66) | tang drawn but not dimensioned
segments_mat_no = "500:0825378 300:0825377 200:0825376 100:0825394 50:0825391 45:0825390 40:0825389 35:0825388 30:0825387 25:0825386 20:1247241 15:1465780 10:1465757"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.64) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW210-S-H140"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | catalogue type OW210/S
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW210/S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | header 'OW210/S'
clamping = "TRUMPF shoulder-bearing (OW/S), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "straight acute, shoulder-bearing ('pointed', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 140   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | 'Height: 140 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | 'Radius: R1,0 mm'
tip_angle_deg = 28.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | 'Angle: 28°'
max_load = "400 kN/m; 1300 kN/m (second value, separate icon: interpretation hemming/flattening)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | technical data load icon: 400 kN/m / 1300 kN/m (second value, separate icon: interpretation hemming/flattening)
weight_kg_per_m = 28.7   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | '28,7 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 78.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | '78 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "25 mm - 300 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | '25 mm - 300 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
shoulder_width_mm = 60   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.74) | horizontal 60 shoulder
shoulder_thickness_mm = 10   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.74) | vertical 10 shoulder
neck_thickness_mm = 24.5   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.74) | horizontal 24.5 body
blade_thickness_mm = 12.5   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.74) | horizontal 12.5 lower blade
relief_height_mm = 70   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.74) | vertical 70 front relief from tip
flank_angle_deg = 14   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.74) | 14° rear flank
dim_a_mm = 1   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.74) | horizontal 1 front face to centre line
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.74) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.74) | tang drawn but not dimensioned
segments_mat_no = "500:0823995 300:0823993 200:0823991 100:0824653 50:0824650 45:0824649 40:0824648 35:0824647 30:0824646 25:0824641 20:0836712 15:0836711 10:0836710"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.72) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW210-S-H240"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | catalogue type OW210/S
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW210/S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | header 'OW210/S'
clamping = "TRUMPF shoulder-bearing (OW/S), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "straight acute, shoulder-bearing ('pointed', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 240   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | 'Height: 240 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | 'Radius: R1,0 mm'
tip_angle_deg = 28.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | 'Angle: 28°'
max_load = "400 kN/m; 1300 kN/m (second value, separate icon)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | technical data load icon: 400 kN/m / 1300 kN/m (second value, separate icon)
weight_kg_per_m = 47.5   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | '47,5 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 145.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | '145 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "25 mm - 200 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | '25 mm - 200 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
shoulder_width_mm = 60   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.112) | horizontal 60
shoulder_thickness_mm = 10   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.112) | vertical 10
neck_thickness_mm = 24.5   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.112) | horizontal 24.5
blade_thickness_mm = 12.5   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.112) | horizontal 12.5
relief_height_mm = 70   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.112) | vertical 70
flank_angle_deg = 14   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.112) | 14°
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.112) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.112) | tang drawn but not dimensioned
segments_mat_no = "500:0824033 300:0824032 200:0824031 100:0824679 50:0824675 45:0824672 40:0824669 35:0824667 30:0824663 25:0824659 20:0842994 15:0842993 10:0842992"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.110) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW200-S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | catalogue type OW200/S
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW200/S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | header 'OW200/S'
clamping = "TRUMPF shoulder-bearing (OW/S), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "gooseneck, shoulder-bearing ('highly offset', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 220   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | 'Height: 220 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | 'Radius: R1,0 mm'
tip_angle_deg = 86.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | 'Angle: 86°'
max_load = "800 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | technical data load icon: 800 kN/m
weight_kg_per_m = 65.8   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | '65,8 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 132.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | '132 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "25 mm - 200 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | '25 mm - 200 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
neck_thickness_mm = 33   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.78) | horizontal 33
throat_depth_mm = 22   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.78) | horizontal 22 throat to tang centre line
dim_body_overall_mm = 85   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.78) | horizontal 85 rear face to shoulder front
tip_flank_length_mm = 10   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.78) | dimension 10 along flank
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.78) | tang drawn but not dimensioned
segments_mat_no = "300:0823799 200:0823798 100:0824549 50:0824480 45:0824479 40:0824478 35:0824477 30:0824476 25:0824475 20:0848757 15:0837118 10:0837117"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.76) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW201-S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | catalogue type OW201/S
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW201/S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | header 'OW201/S'
clamping = "TRUMPF shoulder-bearing (OW/S), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "straight, slightly offset, shoulder-bearing ('slightly offset', p.11)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 220   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | 'Height: 220 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | 'Radius: R1,0 mm'
tip_angle_deg = 86.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | 'Angle: 86°'
max_load = "800 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | technical data load icon: 800 kN/m
weight_kg_per_m = 56.5   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | '56,5 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 132.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | '132 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "25 mm - 200 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | '25 mm - 200 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
shoulder_width_mm = 60   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.98) | horizontal 60
body_width_mm = 28   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.98) | horizontal 28
tip_flank_length_mm = 10   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.98) | dimension 10 along flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.98) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.98) | tang drawn but not dimensioned
segments_mat_no = "300:0823959 200:0823958 100:0824580 50:0824577 45:0824574 40:0824572 35:0824569 30:0824568 25:0824566 20:0842991 15:0842990 10:0842989"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.96) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW202-S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | catalogue type OW202/S
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW202/S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | header 'OW202/S'
clamping = "TRUMPF shoulder-bearing (OW/S), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "straight acute, shoulder-bearing ('pointed', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 220   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | 'Height: 220 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | 'Radius: R1,0 mm'
tip_angle_deg = 28.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | 'Angle: 28°'
max_load = "600 kN/m; 800 kN/m (second value, separate icon)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | technical data load icon: 600 kN/m / 800 kN/m (second value, separate icon)
weight_kg_per_m = 49.9   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | '49,9 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 132.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | '132 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "25 mm - 200 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | '25 mm - 200 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
shoulder_width_mm = 60   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.104) | horizontal 60
body_width_mm = 30   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.104) | horizontal 30
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.104) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.104) | tang drawn but not dimensioned
segments_mat_no = "500:0823859 300:0823858 200:0823857 100:0824628 50:0824624 45:0824623 40:0824622 35:0824618 30:0824617 25:0824614 20:0846380 15:0837116 10:0837115"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.102) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW203-S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | catalogue type OW203/S
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW203/S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | header 'OW203/S'
clamping = "TRUMPF shoulder-bearing (OW/S), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "straight, shoulder-bearing ('even', p.11)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 220   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | 'Height: 220 mm'
tip_radius_mm = 4.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | 'Radius: R4,0 mm'
tip_angle_deg = 60.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | 'Angle: 60°'
max_load = "4000 kN/m (max. continuous load for R4-R10 up to 4000 kN/m)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | technical data load icon: 4000 kN/m (max. continuous load for R4-R10 up to 4000 kN/m)
weight_kg_per_m = 51.7   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | '51,7 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 132.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | '132 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "25 mm - 200 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | '25 mm - 200 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
shoulder_width_mm = 60   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.108) | horizontal 60
shoulder_thickness_mm = 10   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.108) | vertical 10
body_width_mm = 25   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.108) | horizontal 25
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.108) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.108) | tang drawn but not dimensioned
segments_mat_no = "300:0825630 200:0825629 100:0825639 50:0825636 45:0825635 40:0825634 35:0825633 30:0825632 25:0825631 20:1431078 15:1431066 10:1394894"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.106) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW300-S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | catalogue type OW300/S
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW300/S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | header 'OW300/S'
clamping = "TRUMPF shoulder-bearing (OW/S), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "gooseneck, shoulder-bearing ('highly offset', p.11)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 300   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | 'Height: 300 mm'
tip_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | 'Radius: R1,0 mm'
tip_angle_deg = 86.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | 'Angle: 86°'
max_load = "800 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | technical data load icon: 800 kN/m
weight_kg_per_m = 98.8   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | '98,8 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 185.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | '185 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "25 mm - 100 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | '25 mm - 100 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
neck_thickness_mm = 37   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.92) | horizontal 37
dim_body_overall_mm = 101   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.92) | horizontal 101 rear face to shoulder/tang front line
tip_flank_length_mm = 10   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.92) | dimension 10 along flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.92) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.92) | tang drawn but not dimensioned
segments_mat_no = "200:1420630 100:1420613 50:1420625 45:1420623 40:1420621 35:1420619 30:1420617 25:1420615"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.90) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW302-S-R3"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | catalogue type OW302/S
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW302/S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | header 'OW302/S'
clamping = "TRUMPF shoulder-bearing (OW/S), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "straight acute, shoulder-bearing ('pointed', p.10)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 310   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | 'Height: 310 mm'
tip_radius_mm = 3.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | 'Radius: R3,0 mm'
tip_angle_deg = 28.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | 'Angle: 28°'
max_load = "1800 kN/m (R1 version: max. continuous load up to 1000 kN/m)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | technical data load icon: 1800 kN/m (R1 version: max. continuous load up to 1000 kN/m)
weight_kg_per_m = 66.2   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | '66,2 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 192.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | '192 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "25 mm - 200 mm"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | '25 mm - 200 mm' next to vertical-change icon; Safety-Click lengths per p.20-24
dim_rear_to_centre_mm = 25   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.116) | horizontal 25 rear face to centre line
dim_body_overall_mm = 50   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.116) | horizontal 50 rear face to shoulder front
dim_a_mm = 2   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.116) | horizontal 2 centre line offset (two places: upper body and tip)
throat_depth_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.116) | not dimensioned on drawing
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.116) | tang drawn but not dimensioned
segments_mat_no = "300:2868725 200:2868724 100:2868720 50:2868719 45:2868718 40:2868717 35:2868716 30:2868715 25:2868714"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "VAR.B 4050..1050; VAR.A 2050..550; SET 250"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.114) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

[[punch]]
id = "trumpf/OW400-S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | catalogue type OW400/S
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF LASERdur Bending Tools catalog, Edition 01/2025, Mat.No. 2968160; read 2026-11-24
name = "OW400/S"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | header 'OW400/S'
clamping = "TRUMPF shoulder-bearing (OW/S), Modufix clamp"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.28) | p.28 machine/tool table: head-bearing tools (OW/K), shoulder-bearing tools (OW/S); Modufix clamp
kind = "gooseneck, shoulder-bearing ('highly offset', p.11)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.10-11) | overview groups: pointed / highly offset / slightly offset / even
height_mm = 400   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | 'Height: 400 mm'
tip_radius_mm = 4.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | 'Radius: R4,0 mm'
tip_angle_deg = 81.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | 'Angle: 81°'
max_load = "1100 kN/m"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | technical data load icon: 1100 kN/m
weight_kg_per_m = 223.1   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | '223,1 kg/m'
material = "42 Cr Mo 4, 1080-1180 N/mm2, ~60 HRC"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | technical data '42 Cr Mo 4', '1080 - 1180 N/mm²', '~60 HRC'
max_box_internal_height_mm = 253.0   # published (meaning from icon + p.299 SHI table) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | '253 mm' next to box icon; p.299 table maps punch height to box internal height
safety_click_length_range = "UNKNOWN (no range printed)"   # published (meaning from icon: estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | 'UNKNOWN (no range printed)' next to vertical-change icon; Safety-Click lengths per p.20-24
neck_thickness_mm = 65   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.95) | horizontal 65
throat_depth_mm = 143   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.95) | horizontal 143 throat to tang centre line
dim_body_overall_mm = 232   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.95) | horizontal 232 rear face to front of top block
tip_flank_length_mm = 20   # published (drawing; reference lines read by eye) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.95) | dimension 20 along flank (figure scale 1:2)
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.95) | tang drawn but not dimensioned
segments_mat_no = "100:2869505"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | SINGLE lengths [mm] : Mat. No.
sectioned_sets = "single 100 only"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.94) | VAR. B / VAR. A / SET totals in mm, each with own Mat. No.

### Trumpf dies

[[die]]
id = "trumpf/EV-S-W4-30deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | catalogue type 'EV/S-W4/30°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV/S-W4/30°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | header 'EV/S-W4/30°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 4   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | 'Die width: 4 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 30   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | 'Angle: 30°'
shoulder_radius_mm = 0.6   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 184 drawing) | R 0.6 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | 'Height: 100 mm'
die_width_mm = 15   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | body width 15 on drawing
max_load_kN_per_m = 150   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184; p.300) | '150 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 14.1   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | '14,1 kg/m'
min_flange_mm = "3,3 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | '3,3 mm' next to flange icon
min_flange_at_thickness = "4,9mm (s=0,9mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | '4,9mm (s=0,9mm)' next to flange/sheet icon
segments_mat_no = "100:0994476 50:0994482 25:0994477 10:1236598; VAR.B 2550:1205132; SET 150"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | lengths [mm] : Mat. No.
other_dims = "drawing: lower step 5 and 10 (vertical) at foot"   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | drawing: lower step 5 and 10 (vertical) at foot

[[die]]
id = "trumpf/EV-S-W4-84deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | catalogue type 'EV/S-W4/84°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV/S-W4/84°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | header 'EV/S-W4/84°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 4   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | 'Die width: 4 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 84   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | 'Angle: 84°'
shoulder_radius_mm = 0.6   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 184 drawing) | R 0.6 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | 'Height: 100 mm'
die_width_mm = 8   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | body width 8 on drawing
max_load_kN_per_m = 250   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184; p.300) | '250 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 10.2   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | '10,2 kg/m'
min_flange_mm = "3,1 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | '3,1 mm' next to flange icon
min_flange_at_thickness = "3,8mm (s=1mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | '3,8mm (s=1mm)' next to flange/sheet icon
segments_mat_no = "500:1206609 100:1206600 50:1206606 25:1206601 10:1315231"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | lengths [mm] : Mat. No.
other_dims = "drawing: lower body 20 wide, step height 28,5, foot 8"   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.184) | drawing: lower body 20 wide, step height 28,5, foot 8

[[die]]
id = "trumpf/EV001_30deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | catalogue type 'EV001 30°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV001 30°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | header 'EV001 30°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 6   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | 'Die width: 6 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 30   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | 'Angle: 30°'
shoulder_radius_mm = 0.6   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 188 drawing) | R 0.6 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | 'Height: 100 mm'
die_width_mm = 20   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | body width 20 on drawing
max_load_kN_per_m = 400   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188; p.300) | '400 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 17.2   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | '17,2 kg/m'
min_flange_mm = "4,8 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | '4,8 mm' next to flange icon
min_flange_at_thickness = "9,8mm (s=1,5mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | '9,8mm (s=1,5mm)' next to flange/sheet icon
segments_mat_no = "500:0823843 300:0823842 200:0823841 100:0823840 50:0823839 45:0823838 40:0823837 35:0823836 30:0823835 25:0823830 20:0836703 15:0836702 10:0836701 25-with-groove:0932559; VAR.B 4050:0823872 ...; SET 250:0823844"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV001-H_30deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | catalogue type 'EV001/H 30°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV001/H 30°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | header 'EV001/H 30°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 6   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | 'Die width: 6 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 30   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | 'Angle: 30°'
shoulder_radius_mm = 0.6   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 188 drawing) | R 0.6 in overview table and on drawing
die_height_mm = 150   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | 'Height: 150 mm'
die_width_mm = 20   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | body width 20 on drawing
max_load_kN_per_m = 400   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188; p.300) | '400 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 26.1   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | '26,1 kg/m'
min_flange_mm = "4,8 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | '4,8 mm' next to flange icon
min_flange_at_thickness = "9,8mm (s=1,5mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | '9,8mm (s=1,5mm)' next to flange/sheet icon
segments_mat_no = "500:0823893 300:0823889 200:0823886 100:0823885 50:0823883 25:0823877"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | lengths [mm] : Mat. No.
other_dims = "drawing: lower body 30 wide, foot step 10"   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.188) | drawing: lower body 30 wide, foot step 10

[[die]]
id = "trumpf/EV040_90deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | catalogue type 'EV040 90°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV040 90°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | header 'EV040 90°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 6   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | 'Die width: 6 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 90   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | 'Angle: 90°'
shoulder_radius_mm = 0.6   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 192 drawing) | R 0.6 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | 'Height: 100 mm'
die_width_mm = 20   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | body width 20 on drawing
max_load_kN_per_m = 1100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192; p.300) | '1100 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 18.4   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | '18,4 kg/m'
min_flange_mm = "4,5 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | '4,5 mm' next to flange icon
min_flange_at_thickness = "9,8mm (s=1,5mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | '9,8mm (s=1,5mm)' next to flange/sheet icon
segments_mat_no = "500:0824515 100:0824512 50:0824511 25:0824506"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | lengths [mm] : Mat. No.
other_dims = "coining die; drawing: 30 lower body, 10 foot step, V 6"   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.192) | coining die; drawing: 30 lower body, 10 foot step, V 6

[[die]]
id = "trumpf/EV002_30deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193) | catalogue type 'EV002 30°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV002 30°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193) | header 'EV002 30°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 8   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193) | 'Die width: 8 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 30   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193) | 'Angle: 30°'
shoulder_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 193 drawing) | R 1.0 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193) | 'Height: 100 mm'
die_width_mm = 20   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193) | body width 20 on drawing
max_load_kN_per_m = 400   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193; p.300) | '400 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 17.2   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193) | '17,2 kg/m'
min_flange_mm = "6,3 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193) | '6,3 mm' next to flange icon
min_flange_at_thickness = "10mm (s=2mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193) | '10mm (s=2mm)' next to flange/sheet icon
segments_mat_no = "500:0823928 100:0823925 50:0823924 25:0823919 10:0839096"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.193) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV_W10-84deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201) | catalogue type 'EV W10/84°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV W10/84°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201) | header 'EV W10/84°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 10   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201) | 'Die width: 10 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 84   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201) | 'Angle: 84°'
shoulder_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 201 drawing) | R 1.0 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201) | 'Height: 100 mm'
die_width_mm = 20   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201) | body width 20 on drawing
max_load_kN_per_m = 500   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201; p.300) | '500 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 17.5   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201) | '17,5 kg/m'
min_flange_mm = "7,1 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201) | '7,1 mm' next to flange icon
min_flange_at_thickness = "10mm (s=2,5mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201) | '10mm (s=2,5mm)' next to flange/sheet icon
segments_mat_no = "500:0857526 100:0857517 50:0857523 25:0857518 10:1429296"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.201) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV004_30deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205) | catalogue type 'EV004 30°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV004 30°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205) | header 'EV004 30°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 12   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205) | 'Die width: 12 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 30   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205) | 'Angle: 30°'
shoulder_radius_mm = 1.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 205 drawing) | R 1.0 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205) | 'Height: 100 mm'
die_width_mm = 25   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205) | body width 25 on drawing
max_load_kN_per_m = 400   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205; p.300) | '400 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 20.3   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205) | '20,3 kg/m'
min_flange_mm = "9,3 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205) | '9,3 mm' next to flange icon
min_flange_at_thickness = "12,4mm (s=2,5m)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205) | '12,4mm (s=2,5m)' next to flange/sheet icon
segments_mat_no = "500:0824075 100:0824068 50:0824067 25:0824050 10:0840388"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.205) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV_W16-84deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213) | catalogue type 'EV W16/84°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV W16/84°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213) | header 'EV W16/84°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 16   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213) | 'Die width: 16 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 84   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213) | 'Angle: 84°'
shoulder_radius_mm = 1.6   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 213 drawing) | R 1.6 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213) | 'Height: 100 mm'
die_width_mm = 30   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213) | body width 30 on drawing
max_load_kN_per_m = 700   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213; p.300) | '700 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 25.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213) | '25,0 kg/m'
min_flange_mm = "11,1 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213) | '11,1 mm' next to flange icon
min_flange_at_thickness = "15,1mm (s=4mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213) | '15,1mm (s=4mm)' next to flange/sheet icon
segments_mat_no = "500:0857538 100:0857529 50:0857535 25:0857530 10:1272089"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.213) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV006_30deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215) | catalogue type 'EV006 30°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV006 30°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215) | header 'EV006 30°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 20   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215) | 'Die width: 20 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 30   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215) | 'Angle: 30°'
shoulder_radius_mm = 2.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 215 drawing) | R 2.0 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215) | 'Height: 100 mm'
die_width_mm = 35   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215) | body width 35 on drawing
max_load_kN_per_m = 600   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215; p.300) | '600 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 26.4   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215) | '26,4 kg/m'
min_flange_mm = "15,7 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215) | '15,7 mm' next to flange icon
min_flange_at_thickness = "17,3mm (s=4mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215) | '17,3mm (s=4mm)' next to flange/sheet icon
segments_mat_no = "500:0827283 100:0827280 50:0827279 25:0827274 10:0840381"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.215) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV_W24-80deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220) | catalogue type 'EV W24/80°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV W24/80°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220) | header 'EV W24/80°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 24   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220) | 'Die width: 24 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 80   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220) | 'Angle: 80°'
shoulder_radius_mm = 2.5   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 220 drawing) | R 2.5 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220) | 'Height: 100 mm'
die_width_mm = 35   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220) | body width 35 on drawing
max_load_kN_per_m = 700   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220; p.300) | '700 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 28.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220) | '28,0 kg/m'
min_flange_mm = "19,0 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220) | '19,0 mm' next to flange icon
min_flange_at_thickness = "18mm (s=5mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220) | '18mm (s=5mm)' next to flange/sheet icon
segments_mat_no = "500:0832839 100:0835119 50:0853232 25:0853227"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.220) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV_W30-80deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222) | catalogue type 'EV W30/80°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV W30/80°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222) | header 'EV W30/80°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 30   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222) | 'Die width: 30 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 80   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222) | 'Angle: 80°'
shoulder_radius_mm = 5.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 222 drawing) | R 5.0 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222) | 'Height: 100 mm'
die_width_mm = 45   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222) | body width 45 on drawing
max_load_kN_per_m = 900   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222; p.300) | '900 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 34.8   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222) | '34,8 kg/m'
min_flange_mm = "23,7 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222) | '23,7 mm' next to flange icon
min_flange_at_thickness = "23,6mm (s=6mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222) | '23,6mm (s=6mm)' next to flange/sheet icon
segments_mat_no = "500:0862760 100:0862751 50:0862757 25:0862752"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.222) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV_W40-30deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223) | catalogue type 'EV W40/30°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV W40/30°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223) | header 'EV W40/30°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 40   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223) | 'Die width: 40 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 30   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223) | 'Angle: 30°'
shoulder_radius_mm = 5.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 223 drawing) | R 5.0 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223) | 'Height: 100 mm'
die_width_mm = 60   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223) | body width 60 on drawing
max_load_kN_per_m = 900   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223; p.300) | '900 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 36.7   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223) | '36,7 kg/m'
min_flange_mm = "31,5 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223) | '31,5 mm' next to flange icon
min_flange_at_thickness = "32mm (s=6mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223) | '32mm (s=6mm)' next to flange/sheet icon
segments_mat_no = "500:0864275 100:0864266 50:0864272 25:0864267"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.223) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV_W50-30deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | catalogue type 'EV W50/30°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV W50/30°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | header 'EV W50/30°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 50   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | 'Die width: 50 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 30   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | 'Angle: 30°'
shoulder_radius_mm = 5.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 225 drawing) | R 5.0 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | 'Height: 100 mm'
die_width_mm = 75   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | body width 75 on drawing
max_load_kN_per_m = 1200   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225; p.300) | '1200 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 43.3   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | '43,3 kg/m'
min_flange_mm = "38,0 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | '38,0 mm' next to flange icon
min_flange_at_thickness = "41,4mm (s=8mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | '41,4mm (s=8mm)' next to flange/sheet icon
segments_mat_no = "500:0866298 100:0866297 50:0871428 25:0871423"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | lengths [mm] : Mat. No.
other_dims = "drawing: vertical 65 from top to V bottom (V depth, reading)"   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | drawing: vertical 65 from top to V bottom (V depth, reading)

[[die]]
id = "trumpf/EV_W50-80deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | catalogue type 'EV W50/80°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV W50/80°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | header 'EV W50/80°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 50   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | 'Die width: 50 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 80   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | 'Angle: 80°'
shoulder_radius_mm = 5.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 225 drawing) | R 5.0 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | 'Height: 100 mm'
die_width_mm = 65   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | body width 65 on drawing
max_load_kN_per_m = 1500   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225; p.300) | '1500 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 46.5   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | '46,5 kg/m'
min_flange_mm = "35,5 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | '35,5 mm' next to flange icon
min_flange_at_thickness = "37mm (s=10mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | '37mm (s=10mm)' next to flange/sheet icon
segments_mat_no = "500:0862772 100:0862763 50:0862769 25:0862764"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.225) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV_W60-80deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227) | catalogue type 'EV W60/80°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV W60/80°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227) | header 'EV W60/80°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 60   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227) | 'Die width: 60 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 80   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227) | 'Angle: 80°'
shoulder_radius_mm = 5.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 227 drawing) | R 5.0 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227) | 'Height: 100 mm'
die_width_mm = 75   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227) | body width 75 on drawing
max_load_kN_per_m = 1500   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227; p.300) | '1500 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 51.7   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227) | '51,7 kg/m'
min_flange_mm = "42,0 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227) | '42,0 mm' next to flange icon
min_flange_at_thickness = "44mm (s=10mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227) | '44mm (s=10mm)' next to flange/sheet icon
segments_mat_no = "100:0848802 50:0848808 25:0848803"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.227) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV_W80-80deg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | catalogue type 'EV W80/80°'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV W80/80°"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | header 'EV W80/80°'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 80   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | 'Die width: 80 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 80   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | 'Angle: 80°'
shoulder_radius_mm = 5.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 228 drawing) | R 5.0 in overview table and on drawing
die_height_mm = 100   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | 'Height: 100 mm'
die_width_mm = 100   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | body width 100 on drawing
max_load_kN_per_m = 1500   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228; p.300) | '1500 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 64.4   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | '64,4 kg/m'
min_flange_mm = "54,0 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | '54,0 mm' next to flange icon
min_flange_at_thickness = "53,5mm (s=12mm)"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | '53,5mm (s=12mm)' next to flange/sheet icon
segments_mat_no = "100:0848842 50:0848848 25:0848843"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | lengths [mm] : Mat. No.

[[die]]
id = "trumpf/EV_W80-40deg_R8"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | catalogue type 'EV W80/40° R8'
maker = "Trumpf"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf | TRUMPF catalog Edition 01/2025; read 2026-11-24
name = "EV W80/40° R8"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | header 'EV W80/40° R8'
clamping = "TRUMPF EV lower tool (die tang/adapter not dimensioned)"   # published (system) / tang dims unknown | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | EV dies; adapters TRUMPF EV/DV H50, EHT/AMADA H40 on p.16
v_opening_mm = 80   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | 'Die width: 80 mm' (TRUMPF 'die width' = V opening)
v_angle_deg = 40   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | 'Angle: 40°'
shoulder_radius_mm = 8.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.14-16, 228 drawing) | R 8.0 in overview table and on drawing
die_height_mm = 120   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | 'Height: 120 mm'
die_width_mm = 115   # published (drawing) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | body width 115 on drawing
max_load_kN_per_m = 3000   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228; p.300) | '3000 kN/m' (also in p.300 max-load table)
weight_kg_per_m = 77.0   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | '77,0 kg/m'
min_flange_mm = "60,0 mm"   # published (icon meaning estimate: min flange length) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | '60,0 mm' next to flange icon
min_flange_at_thickness = "UNKNOWN"   # published (icon meaning estimate) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | 'UNKNOWN' next to flange/sheet icon
segments_mat_no = "100:2912604 50:2912603 25:2912538; VAR.B 2550:2937186"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Services/01_brochures/TRUMPF-bending-tools-catalog-EN.pdf (p.228) | lengths [mm] : Mat. No.

### Amada punches (text layer of the 2023 catalogue only; profile drawings not reachable)

[[punch]]
id = "amada/804"   # published | https://online.flippingbook.com/view/795609261/8/ | 'Tool #804'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/8/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 8); read 2026-11-24
name = "Tool #804 AFH"   # published | https://online.flippingbook.com/view/795609261/8/ | 'Tool #804 ... - 100 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/8/ | section heading 'AFH Punches H=120'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/8/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 120   # published | https://online.flippingbook.com/view/795609261/8/ | section heading 'H=120'
tip_angle_and_radius = "86°: R0.6/R1.5/R3.0; 88°: R0.6"   # published | https://online.flippingbook.com/view/795609261/8/ | angle/radii column: 86°: R0.6/R1.5/R3.0; 88°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/8/ | R0.6
tip_angle_deg = 86.0   # published | https://online.flippingbook.com/view/795609261/8/ | 86°
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/8/ | '100 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 45   # published | https://online.flippingbook.com/view/795609261/8/ | 'Ear tonnage = 45 t/m'
tip_dimension = "9 mm tip"   # published (meaning not stated in text) | https://online.flippingbook.com/view/795609261/8/ | '9 mm tip'
other_drawing_numbers_mm = "35"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/8/ | 35
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/8/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/8/ | not in text layer
codes = "8048061/2/3 8048151/2/3 8048301/2/3 8047061/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/8/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/847"   # published | https://online.flippingbook.com/view/795609261/8/ | 'Tool #847'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/8/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 8); read 2026-11-24
name = "Tool #847 AFH"   # published | https://online.flippingbook.com/view/795609261/8/ | 'Tool #847 ... - 50 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/8/ | section heading 'AFH Punches H=120'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/8/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 120   # published | https://online.flippingbook.com/view/795609261/8/ | section heading 'H=120'
tip_angle_and_radius = "86°: R0.6/R1.5/R3.0; 88°: R0.6"   # published | https://online.flippingbook.com/view/795609261/8/ | angle/radii column: 86°: R0.6/R1.5/R3.0; 88°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/8/ | R0.6
tip_angle_deg = 86.0   # published | https://online.flippingbook.com/view/795609261/8/ | 86°
max_load_t_per_m = 50   # published | https://online.flippingbook.com/view/795609261/8/ | '50 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 45   # published | https://online.flippingbook.com/view/795609261/8/ | 'Ear tonnage = 45 t/m'
tip_dimension = "9 mm tip"   # published (meaning not stated in text) | https://online.flippingbook.com/view/795609261/8/ | '9 mm tip'
other_drawing_numbers_mm = "35"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/8/ | 35
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/8/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/8/ | not in text layer
codes = "8478061/2/3 8478151/2/3 8478301/2/3 8477061/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/8/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/820"   # published | https://online.flippingbook.com/view/795609261/9/ | 'Tool #820'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/9/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 9); read 2026-11-24
name = "Tool #820 AFH"   # published | https://online.flippingbook.com/view/795609261/9/ | 'Tool #820 ... - 50 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/9/ | section heading 'AFH Punches H=120'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/9/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 120   # published | https://online.flippingbook.com/view/795609261/9/ | section heading 'H=120'
tip_angle_and_radius = "86°: R0.6/R1.5; 88°: R0.6"   # published | https://online.flippingbook.com/view/795609261/9/ | angle/radii column: 86°: R0.6/R1.5; 88°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/9/ | R0.6
tip_angle_deg = 86.0   # published | https://online.flippingbook.com/view/795609261/9/ | 86°
max_load_t_per_m = 50   # published | https://online.flippingbook.com/view/795609261/9/ | '50 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 20   # published | https://online.flippingbook.com/view/795609261/9/ | 'Ear tonnage = 20 t/m'
tip_dimension = "8 mm tip"   # published (meaning not stated in text) | https://online.flippingbook.com/view/795609261/9/ | '8 mm tip'
other_drawing_numbers_mm = "25"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/9/ | 25
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/9/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/9/ | not in text layer
codes = "8208061/2/3 8208151/2/3 8207061/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/9/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/846"   # published | https://online.flippingbook.com/view/795609261/9/ | 'Tool #846'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/9/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 9); read 2026-11-24
name = "Tool #846 AFH"   # published | https://online.flippingbook.com/view/795609261/9/ | 'Tool #846 ... - 45 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/9/ | section heading 'AFH Punches H=120'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/9/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 120   # published | https://online.flippingbook.com/view/795609261/9/ | section heading 'H=120'
tip_angle_and_radius = "86°: R0.6; 88°: R0.6"   # published | https://online.flippingbook.com/view/795609261/9/ | angle/radii column: 86°: R0.6; 88°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/9/ | R0.6
tip_angle_deg = 86.0   # published | https://online.flippingbook.com/view/795609261/9/ | 86°
max_load_t_per_m = 45   # published | https://online.flippingbook.com/view/795609261/9/ | '45 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 35   # published | https://online.flippingbook.com/view/795609261/9/ | 'Ear tonnage = 35 t/m'
tip_dimension = "6 mm tip"   # published (meaning not stated in text) | https://online.flippingbook.com/view/795609261/9/ | '6 mm tip'
other_drawing_numbers_mm = "35"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/9/ | 35
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/9/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/9/ | not in text layer
codes = "8468061/2/3 8467061/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/9/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/817"   # published | https://online.flippingbook.com/view/795609261/9/ | 'Tool #817'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/9/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 9); read 2026-11-24
name = "Tool #817 AFH"   # published | https://online.flippingbook.com/view/795609261/9/ | 'Tool #817 ... - 30 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/9/ | section heading 'AFH Punches H=120'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/9/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 120   # published | https://online.flippingbook.com/view/795609261/9/ | section heading 'H=120'
tip_angle_and_radius = "86°: R0.6; 88°: R0.6"   # published | https://online.flippingbook.com/view/795609261/9/ | angle/radii column: 86°: R0.6; 88°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/9/ | R0.6
tip_angle_deg = 86.0   # published | https://online.flippingbook.com/view/795609261/9/ | 86°
max_load_t_per_m = 30   # published | https://online.flippingbook.com/view/795609261/9/ | '30 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 15   # published | https://online.flippingbook.com/view/795609261/9/ | 'Ear tonnage = 15 t/m'
tip_dimension = "4 mm tip"   # published (meaning not stated in text) | https://online.flippingbook.com/view/795609261/9/ | '4 mm tip'
other_drawing_numbers_mm = "10"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/9/ | 10
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/9/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/9/ | not in text layer
codes = "8178061/2/3 8177061/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/9/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/813"   # published | https://online.flippingbook.com/view/795609261/10/ | 'Tool #813'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/10/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 10); read 2026-11-24
name = "Tool #813 AFH"   # published | https://online.flippingbook.com/view/795609261/10/ | 'Tool #813 ... - 45 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/10/ | section heading 'AFH Punches H=120'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/10/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 120   # published | https://online.flippingbook.com/view/795609261/10/ | section heading 'H=120'
tip_angle_and_radius = "86°: R0.6"   # published | https://online.flippingbook.com/view/795609261/10/ | angle/radii column: 86°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/10/ | R0.6
tip_angle_deg = 86.0   # published | https://online.flippingbook.com/view/795609261/10/ | 86°
max_load_t_per_m = 45   # published | https://online.flippingbook.com/view/795609261/10/ | '45 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 12   # published | https://online.flippingbook.com/view/795609261/10/ | 'Ear tonnage = 12 t/m'
other_drawing_numbers_mm = "25, 90.81"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/10/ | 25, 90.81
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/10/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/10/ | not in text layer
codes = "8138061/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/10/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/814"   # published | https://online.flippingbook.com/view/795609261/10/ | 'Tool #814'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/10/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 10); read 2026-11-24
name = "Tool #814 AFH"   # published | https://online.flippingbook.com/view/795609261/10/ | 'Tool #814 ... - 45 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/10/ | section heading 'AFH Punches H=120'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/10/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 120   # published | https://online.flippingbook.com/view/795609261/10/ | section heading 'H=120'
tip_angle_and_radius = "30°: R0.6"   # published | https://online.flippingbook.com/view/795609261/10/ | angle/radii column: 30°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/10/ | R0.6
tip_angle_deg = 30.0   # published | https://online.flippingbook.com/view/795609261/10/ | 30°
max_load_t_per_m = 45   # published | https://online.flippingbook.com/view/795609261/10/ | '45 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 12   # published | https://online.flippingbook.com/view/795609261/10/ | 'Ear tonnage = 12 t/m'
other_drawing_numbers_mm = "25, 15"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/10/ | 25, 15
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/10/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/10/ | not in text layer
codes = "8143061/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/10/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/831"   # published | https://online.flippingbook.com/view/795609261/10/ | 'Tool #831'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/10/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 10); read 2026-11-24
name = "Tool #831 AFH"   # published | https://online.flippingbook.com/view/795609261/10/ | 'Tool #831 ... - 100 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/10/ | section heading 'AFH Punches H=120'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/10/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 120   # published | https://online.flippingbook.com/view/795609261/10/ | section heading 'H=120'
tip_angle_and_radius = "30°: R0.6/R1.5/R3.0/R4.0; 26°: R0.6"   # published | https://online.flippingbook.com/view/795609261/10/ | angle/radii column: 30°: R0.6/R1.5/R3.0/R4.0; 26°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/10/ | R0.6
tip_angle_deg = 30.0   # published | https://online.flippingbook.com/view/795609261/10/ | 30°
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/10/ | '100 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 30   # published | https://online.flippingbook.com/view/795609261/10/ | 'Ear tonnage = 30 t/m'
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/10/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/10/ | not in text layer
codes = "8312061/2/3 8313061/2/3 8313151/2/3 8313301/2/3 8313401/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/10/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/841"   # published | https://online.flippingbook.com/view/795609261/11/ | 'Tool #841'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/11/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 11); read 2026-11-24
name = "Tool #841 AFH"   # published | https://online.flippingbook.com/view/795609261/11/ | 'Tool #841 ... - 70 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/11/ | section heading 'AFH Punches H=120'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/11/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 120   # published | https://online.flippingbook.com/view/795609261/11/ | section heading 'H=120'
tip_angle_and_radius = "60°: R0.6/R1.5"   # published | https://online.flippingbook.com/view/795609261/11/ | angle/radii column: 60°: R0.6/R1.5
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/11/ | R0.6
tip_angle_deg = 60.0   # published | https://online.flippingbook.com/view/795609261/11/ | 60°
max_load_t_per_m = 70   # published | https://online.flippingbook.com/view/795609261/11/ | '70 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 30   # published | https://online.flippingbook.com/view/795609261/11/ | 'Ear tonnage = 30 t/m'
other_drawing_numbers_mm = "35"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/11/ | 35
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/11/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/11/ | not in text layer
codes = "8416061/2/3 8416151/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/11/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/803"   # published | https://online.flippingbook.com/view/795609261/11/ | 'Tool #803'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/11/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 11); read 2026-11-24
name = "Tool #803 AFH"   # published | https://online.flippingbook.com/view/795609261/11/ | 'Tool #803 ... - 100 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/11/ | section heading 'AFH Punches H=120'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/11/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 120   # published | https://online.flippingbook.com/view/795609261/11/ | section heading 'H=120'
tip_angle_and_radius = "60°: R6.0/R5.0"   # published | https://online.flippingbook.com/view/795609261/11/ | angle/radii column: 60°: R6.0/R5.0
tip_radius_mm = 6.0   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/11/ | R6.0
tip_angle_deg = 60.0   # published | https://online.flippingbook.com/view/795609261/11/ | 60°
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/11/ | '100 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/11/ | 'Ear tonnage = 100 t/m'
other_drawing_numbers_mm = "30"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/11/ | 30
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/11/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/11/ | not in text layer
codes = "8036601/2/3 8036501/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/11/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/704"   # published | https://online.flippingbook.com/view/795609261/12/ | 'Tool #704'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/12/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 12); read 2026-11-24
name = "Tool #704 AFH"   # published | https://online.flippingbook.com/view/795609261/12/ | 'Tool #704 ... - 100 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/12/ | section heading 'AFH Punches H=90'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/12/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 90   # published | https://online.flippingbook.com/view/795609261/12/ | section heading 'H=90'
tip_angle_and_radius = "86°: R0.6/R1.5/R3.0; 88°: R0.6"   # published | https://online.flippingbook.com/view/795609261/12/ | angle/radii column: 86°: R0.6/R1.5/R3.0; 88°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/12/ | R0.6
tip_angle_deg = 86.0   # published | https://online.flippingbook.com/view/795609261/12/ | 86°
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/12/ | '100 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 45   # published | https://online.flippingbook.com/view/795609261/12/ | 'Ear tonnage = 45 t/m'
tip_dimension = "9 mm tip"   # published (meaning not stated in text) | https://online.flippingbook.com/view/795609261/12/ | '9 mm tip'
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/12/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/12/ | not in text layer
codes = "7048061/2/3 7048151/2/3 7048301/2/3 7047061/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/12/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section
previous_number = "Was #0S01 (86°), #0S02 (90°)"   # published | https://online.flippingbook.com/view/795609261/12/ | Was #0S01 (86°), #0S02 (90°)

[[punch]]
id = "amada/731"   # published | https://online.flippingbook.com/view/795609261/14/ | 'Tool #731'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/14/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 14); read 2026-11-24
name = "Tool #731 AFH"   # published | https://online.flippingbook.com/view/795609261/14/ | 'Tool #731 ... - 100 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/14/ | section heading 'AFH Punches H=90'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/14/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 90   # published | https://online.flippingbook.com/view/795609261/14/ | section heading 'H=90'
tip_angle_and_radius = "30°: R0.6/R1.5/R3.0"   # published | https://online.flippingbook.com/view/795609261/14/ | angle/radii column: 30°: R0.6/R1.5/R3.0
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/14/ | R0.6
tip_angle_deg = 30.0   # published | https://online.flippingbook.com/view/795609261/14/ | 30°
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/14/ | '100 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 30   # published | https://online.flippingbook.com/view/795609261/14/ | 'Ear tonnage = 30 t/m'
other_drawing_numbers_mm = "20"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/14/ | 20
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/14/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/14/ | not in text layer
codes = "7313061/2/3 7313151/2/3 7313301/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/14/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/904"   # published | https://online.flippingbook.com/view/795609261/15/ | 'Tool #904'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/15/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 15); read 2026-11-24
name = "Tool #904 AFH"   # published | https://online.flippingbook.com/view/795609261/15/ | 'Tool #904 ... - 100 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/15/ | section heading 'AFH Punches H=160'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/15/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 160   # published | https://online.flippingbook.com/view/795609261/15/ | section heading 'H=160'
tip_angle_and_radius = "86°: R0.6/R1.5/R3.0; 88°: R0.6"   # published | https://online.flippingbook.com/view/795609261/15/ | angle/radii column: 86°: R0.6/R1.5/R3.0; 88°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/15/ | R0.6
tip_angle_deg = 86.0   # published | https://online.flippingbook.com/view/795609261/15/ | 86°
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/15/ | '100 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 45   # published | https://online.flippingbook.com/view/795609261/15/ | 'Ear tonnage = 45 t/m'
tip_dimension = "9 mm tip"   # published (meaning not stated in text) | https://online.flippingbook.com/view/795609261/15/ | '9 mm tip'
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/15/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/15/ | not in text layer
codes = "9048061/2/3 9048151/2/3 9048301/2/3 9047061/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/15/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/947"   # published | https://online.flippingbook.com/view/795609261/15/ | 'Tool #947'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/15/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 15); read 2026-11-24
name = "Tool #947 AFH"   # published | https://online.flippingbook.com/view/795609261/15/ | 'Tool #947 ... - 50 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/15/ | section heading 'AFH Punches H=160'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/15/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 160   # published | https://online.flippingbook.com/view/795609261/15/ | section heading 'H=160'
tip_angle_and_radius = "86°: R0.6/R1.5/R3.0; 88°: R0.6"   # published | https://online.flippingbook.com/view/795609261/15/ | angle/radii column: 86°: R0.6/R1.5/R3.0; 88°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/15/ | R0.6
tip_angle_deg = 86.0   # published | https://online.flippingbook.com/view/795609261/15/ | 86°
max_load_t_per_m = 50   # published | https://online.flippingbook.com/view/795609261/15/ | '50 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 45   # published | https://online.flippingbook.com/view/795609261/15/ | 'Ear tonnage = 45 t/m'
tip_dimension = "9 mm tip"   # published (meaning not stated in text) | https://online.flippingbook.com/view/795609261/15/ | '9 mm tip'
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/15/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/15/ | not in text layer
codes = "9478061/2/3 9478151/2/3 9478301/2/3 9477061/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/15/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section
previous_number = "Was #472 (88°)"   # published | https://online.flippingbook.com/view/795609261/15/ | Was #472 (88°)

[[punch]]
id = "amada/914"   # published | https://online.flippingbook.com/view/795609261/16/ | 'Tool #914'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/16/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 16); read 2026-11-24
name = "Tool #914 AFH"   # published | https://online.flippingbook.com/view/795609261/16/ | 'Tool #914 ... - 45 t/m'
clamping = "AMADA AFH (Amada Fixed Height) system"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/16/ | section heading 'AFH Punches H=160'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/16/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 160   # published | https://online.flippingbook.com/view/795609261/16/ | section heading 'H=160'
tip_angle_and_radius = "30°: R0.6"   # published | https://online.flippingbook.com/view/795609261/16/ | angle/radii column: 30°: R0.6
tip_radius_mm = 0.6   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/16/ | R0.6
tip_angle_deg = 30.0   # published | https://online.flippingbook.com/view/795609261/16/ | 30°
max_load_t_per_m = 45   # published | https://online.flippingbook.com/view/795609261/16/ | '45 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 12   # published | https://online.flippingbook.com/view/795609261/16/ | 'Ear tonnage = 12 t/m'
other_drawing_numbers_mm = "25"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/16/ | 25
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/16/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/16/ | not in text layer
codes = "9143061/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/16/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section

[[punch]]
id = "amada/604"   # published | https://online.flippingbook.com/view/795609261/17/ | 'Tool #604'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/17/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 17); read 2026-11-24
name = "Tool #604"   # published | https://online.flippingbook.com/view/795609261/17/ | 'Tool #604 ... - 100 t/m'
clamping = "AMADA standard (H=70 range, not AFH)"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/17/ | section heading 'AFH Punches H=70'/'Punches H=70'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/17/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 70   # published | https://online.flippingbook.com/view/795609261/17/ | section heading 'H=70'
tip_angle_and_radius = "88°: R0.2/R0.6/R0.8/R1.5/R3.0"   # published | https://online.flippingbook.com/view/795609261/17/ | angle/radii column: 88°: R0.2/R0.6/R0.8/R1.5/R3.0
tip_radius_mm = 0.2   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/17/ | R0.2
tip_angle_deg = 88.0   # published | https://online.flippingbook.com/view/795609261/17/ | 88°
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/17/ | '100 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 45   # published | https://online.flippingbook.com/view/795609261/17/ | 'Ear tonnage = 45 t/m'
tip_dimension = "9 mm tip"   # published (meaning not stated in text) | https://online.flippingbook.com/view/795609261/17/ | '9 mm tip'
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/17/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/17/ | not in text layer
codes = "6047021/2/3 6047061/2/3 6047081/2/3 6047151/2/3 6047301/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/17/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section
previous_number = "Was #4 (88°), #16 (90°)"   # published | https://online.flippingbook.com/view/795609261/17/ | Was #4 (88°), #16 (90°)

[[punch]]
id = "amada/631"   # published | https://online.flippingbook.com/view/795609261/18/ | 'Tool #631'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/18/ | AMADA Tooling Catalogue 2023 (FlippingBook viewer text layer, page 18); read 2026-11-24
name = "Tool #631"   # published | https://online.flippingbook.com/view/795609261/18/ | 'Tool #631 ... - 100 t/m'
clamping = "AMADA standard (H=70 range, not AFH)"   # published (system name); tang dims unknown | https://online.flippingbook.com/view/795609261/18/ | section heading 'AFH Punches H=70'/'Punches H=70'
kind = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/18/ | profile is only in the page image, which is not reachable as text; not stated in text
height_mm = 70   # published | https://online.flippingbook.com/view/795609261/18/ | section heading 'H=70'
tip_angle_and_radius = "30°: R0.52"   # published | https://online.flippingbook.com/view/795609261/18/ | angle/radii column: 30°: R0.52
tip_radius_mm = 0.52   # published (first listed; catalogue highlights the standard radius in colour, which the text layer loses) | https://online.flippingbook.com/view/795609261/18/ | R0.52
tip_angle_deg = 30.0   # published | https://online.flippingbook.com/view/795609261/18/ | 30°
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/18/ | '100 t/m' (allowable tonnage per metre, legend p.3/4)
ear_load_t_per_m = 30   # published | https://online.flippingbook.com/view/795609261/18/ | 'Ear tonnage = 30 t/m'
other_drawing_numbers_mm = "20"   # published number, meaning UNKNOWN (drawing not readable) | https://online.flippingbook.com/view/795609261/18/ | 20
throat_depth_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/18/ | not in text layer
body_width_at_clamp_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/18/ | not in text layer
codes = "6313051/2/3"   # published (code-to-radius pairing by listing order: estimate) | https://online.flippingbook.com/view/795609261/18/ | codes ending 1/2/3 = 835 mm / 415 mm / Sec
segments = "solid 835, solid 415, sectionalised 100(ear),10,15,15,20,40,50,150,300,100(ear)=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Punch (100 mm are ears): 100, 10, 15, 15, 20, 40, 50, 150, 300, 100 = 800 mm'
material = "through hardened 47 HRC, surface 45~50 HRC, body strength 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Punch Quality section
previous_number = "Was #103"   # published | https://online.flippingbook.com/view/795609261/18/ | Was #103

### Amada dies

[[die]]
id = "amada/0848"   # published | https://online.flippingbook.com/view/795609261/34/ | 'Tool #0848'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/34/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/34/ | AFH
v_opening_mm = 4   # published | https://online.flippingbook.com/view/795609261/34/ | 'V4'
v_angle_deg = 86   # published | https://online.flippingbook.com/view/795609261/34/ | '86°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/34/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/34/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/34/ | not in text layer
max_load_t_per_m = 75   # published | https://online.flippingbook.com/view/795609261/34/ | '75 t/m'
codes = "0848101/2/3"   # published | https://online.flippingbook.com/view/795609261/34/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0847"   # published | https://online.flippingbook.com/view/795609261/34/ | 'Tool #0847'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/34/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/34/ | AFH
v_opening_mm = 4   # published | https://online.flippingbook.com/view/795609261/34/ | 'V4'
v_angle_deg = 88   # published | https://online.flippingbook.com/view/795609261/34/ | '88°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/34/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/34/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/34/ | not in text layer
max_load_t_per_m = 75   # published | https://online.flippingbook.com/view/795609261/34/ | '75 t/m'
codes = "0847101/2/3"   # published | https://online.flippingbook.com/view/795609261/34/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0708"   # published | https://online.flippingbook.com/view/795609261/35/ | 'Tool #0708'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/35/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/35/ | AFH
v_opening_mm = 6   # published | https://online.flippingbook.com/view/795609261/35/ | 'V6'
v_angle_deg = 86   # published | https://online.flippingbook.com/view/795609261/35/ | '86°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/35/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/35/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/35/ | not in text layer
max_load_t_per_m = 80   # published | https://online.flippingbook.com/view/795609261/35/ | '80 t/m'
codes = "0708151/2/3"   # published | https://online.flippingbook.com/view/795609261/35/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0703"   # published | https://online.flippingbook.com/view/795609261/35/ | 'Tool #0703'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/35/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/35/ | AFH; was #14006
v_opening_mm = 6   # published | https://online.flippingbook.com/view/795609261/35/ | 'V6'
v_angle_deg = 30   # published | https://online.flippingbook.com/view/795609261/35/ | '30°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/35/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/35/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/35/ | not in text layer
max_load_t_per_m = 50   # published | https://online.flippingbook.com/view/795609261/35/ | '50 t/m'
codes = "0703101/2/3"   # published | https://online.flippingbook.com/view/795609261/35/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0706"   # published | https://online.flippingbook.com/view/795609261/35/ | 'Tool #0706'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/35/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/35/ | AFH; was #170
v_opening_mm = 6   # published | https://online.flippingbook.com/view/795609261/35/ | 'V6'
v_angle_deg = 60   # published | https://online.flippingbook.com/view/795609261/35/ | '60°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/35/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/35/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/35/ | not in text layer
max_load_t_per_m = 38   # published | https://online.flippingbook.com/view/795609261/35/ | '38 t/m'
codes = "0706061/2/3"   # published | https://online.flippingbook.com/view/795609261/35/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0718"   # published | https://online.flippingbook.com/view/795609261/36/ | 'Tool #0718'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/36/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/36/ | AFH
v_opening_mm = 8   # published | https://online.flippingbook.com/view/795609261/36/ | 'V8'
v_angle_deg = 86   # published | https://online.flippingbook.com/view/795609261/36/ | '86°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/36/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/36/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/36/ | not in text layer
max_load_t_per_m = 90   # published | https://online.flippingbook.com/view/795609261/36/ | '90 t/m'
codes = "0718151/2/3"   # published | https://online.flippingbook.com/view/795609261/36/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0713"   # published | https://online.flippingbook.com/view/795609261/37/ | 'Tool #0713'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/37/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/37/ | AFH
v_opening_mm = 8   # published | https://online.flippingbook.com/view/795609261/37/ | 'V8'
v_angle_deg = 30   # published | https://online.flippingbook.com/view/795609261/37/ | '30°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/37/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/37/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/37/ | not in text layer
max_load_t_per_m = 60   # published | https://online.flippingbook.com/view/795609261/37/ | '60 t/m'
codes = "0713151/2/3"   # published | https://online.flippingbook.com/view/795609261/37/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0728"   # published | https://online.flippingbook.com/view/795609261/38/ | 'Tool #0728'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/38/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/38/ | AFH
v_opening_mm = 10   # published | https://online.flippingbook.com/view/795609261/38/ | 'V10'
v_angle_deg = 86   # published | https://online.flippingbook.com/view/795609261/38/ | '86°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/38/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/38/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/38/ | not in text layer
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/38/ | '100 t/m'
codes = "0728201/2/3"   # published | https://online.flippingbook.com/view/795609261/38/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0723"   # published | https://online.flippingbook.com/view/795609261/38/ | 'Tool #0723'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/38/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/38/ | AFH; was #14106
v_opening_mm = 10   # published | https://online.flippingbook.com/view/795609261/38/ | 'V10'
v_angle_deg = 30   # published | https://online.flippingbook.com/view/795609261/38/ | '30°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/38/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/38/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/38/ | not in text layer
max_load_t_per_m = 70   # published | https://online.flippingbook.com/view/795609261/38/ | '70 t/m'
codes = "0723201/2/3"   # published | https://online.flippingbook.com/view/795609261/38/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0748"   # published | https://online.flippingbook.com/view/795609261/39/ | 'Tool #0748'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/39/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/39/ | AFH
v_opening_mm = 12   # published | https://online.flippingbook.com/view/795609261/39/ | 'V12'
v_angle_deg = 86   # published | https://online.flippingbook.com/view/795609261/39/ | '86°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/39/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/39/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/39/ | not in text layer
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/39/ | '100 t/m'
codes = "0748251/2/3"   # published | https://online.flippingbook.com/view/795609261/39/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0743"   # published | https://online.flippingbook.com/view/795609261/40/ | 'Tool #0743'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/40/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/40/ | AFH; was #142
v_opening_mm = 12   # published | https://online.flippingbook.com/view/795609261/40/ | 'V12'
v_angle_deg = 30   # published | https://online.flippingbook.com/view/795609261/40/ | '30°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/40/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/40/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/40/ | not in text layer
max_load_t_per_m = 80   # published | https://online.flippingbook.com/view/795609261/40/ | '80 t/m'
codes = "0743251/2/3"   # published | https://online.flippingbook.com/view/795609261/40/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0778"   # published | https://online.flippingbook.com/view/795609261/41/ | 'Tool #0778'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/41/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/41/ | AFH
v_opening_mm = 16   # published | https://online.flippingbook.com/view/795609261/41/ | 'V16'
v_angle_deg = 86   # published | https://online.flippingbook.com/view/795609261/41/ | '86°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/41/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/41/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/41/ | not in text layer
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/41/ | '100 t/m'
codes = "0778251/2/3"   # published | https://online.flippingbook.com/view/795609261/41/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0773"   # published | https://online.flippingbook.com/view/795609261/42/ | 'Tool #0773'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/42/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/42/ | AFH; was #14206
v_opening_mm = 16   # published | https://online.flippingbook.com/view/795609261/42/ | 'V16'
v_angle_deg = 30   # published | https://online.flippingbook.com/view/795609261/42/ | '30°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/42/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/42/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/42/ | not in text layer
max_load_t_per_m = 60   # published | https://online.flippingbook.com/view/795609261/42/ | '60 t/m'
codes = "0773301/2/3"   # published | https://online.flippingbook.com/view/795609261/42/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0798"   # published | https://online.flippingbook.com/view/795609261/43/ | 'Tool #0798'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/43/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/43/ | AFH
v_opening_mm = 20   # published | https://online.flippingbook.com/view/795609261/43/ | 'V20'
v_angle_deg = 86   # published | https://online.flippingbook.com/view/795609261/43/ | '86°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/43/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/43/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/43/ | not in text layer
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/43/ | '100 t/m'
codes = "0798301/2/3"   # published | https://online.flippingbook.com/view/795609261/43/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0793"   # published | https://online.flippingbook.com/view/795609261/43/ | 'Tool #0793'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/43/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/43/ | was #143
v_opening_mm = 20   # published | https://online.flippingbook.com/view/795609261/43/ | 'V20'
v_angle_deg = 30   # published | https://online.flippingbook.com/view/795609261/43/ | '30°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/43/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/43/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/43/ | not in text layer
max_load_t_per_m = 60   # published | https://online.flippingbook.com/view/795609261/43/ | '60 t/m'
codes = "0793401/2/3"   # published | https://online.flippingbook.com/view/795609261/43/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0828"   # published | https://online.flippingbook.com/view/795609261/44/ | 'Tool #0828'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/44/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/44/ | AFH
v_opening_mm = 25   # published | https://online.flippingbook.com/view/795609261/44/ | 'V25'
v_angle_deg = 86   # published | https://online.flippingbook.com/view/795609261/44/ | '86°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/44/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/44/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/44/ | not in text layer
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/44/ | '100 t/m'
codes = "0828301/2/3"   # published | https://online.flippingbook.com/view/795609261/44/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0823"   # published | https://online.flippingbook.com/view/795609261/45/ | 'Tool #0823'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/45/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/45/ | was #14306
v_opening_mm = 25   # published | https://online.flippingbook.com/view/795609261/45/ | 'V25'
v_angle_deg = 30   # published | https://online.flippingbook.com/view/795609261/45/ | '30°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/45/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/45/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/45/ | not in text layer
max_load_t_per_m = 60   # published | https://online.flippingbook.com/view/795609261/45/ | '60 t/m'
codes = "0823401/2/3"   # published | https://online.flippingbook.com/view/795609261/45/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0874"   # published | https://online.flippingbook.com/view/795609261/45/ | 'Tool #0874'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/45/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/45/ | AFH 1V
v_opening_mm = 32   # published | https://online.flippingbook.com/view/795609261/45/ | 'V32'
v_angle_deg = 84   # published | https://online.flippingbook.com/view/795609261/45/ | '84°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/45/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/45/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/45/ | not in text layer
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/45/ | '100 t/m'
codes = "0874401/2/3"   # published | https://online.flippingbook.com/view/795609261/45/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/0884"   # published | https://online.flippingbook.com/view/795609261/45/ | 'Tool #0884'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/45/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA (AFH 1V die)"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/45/ | AFH 1V
v_opening_mm = 40   # published | https://online.flippingbook.com/view/795609261/45/ | 'V40'
v_angle_deg = 84   # published | https://online.flippingbook.com/view/795609261/45/ | '84°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/45/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/45/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/45/ | not in text layer
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/45/ | '100 t/m'
codes = "0884401/2/3"   # published | https://online.flippingbook.com/view/795609261/45/ | codes; suffix 3 = 'Sec'
segments = "solid 835, solid 415, sectionalised 100,10,15,15,20,40,50,150,400=800"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/35"   # published | https://online.flippingbook.com/view/795609261/46/ | 'Tool #35'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/46/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/46/ | Heavy Duty 1V; other numbers on page: 32, 60, 60 (meaning UNKNOWN)
v_opening_mm = 32   # published | https://online.flippingbook.com/view/795609261/46/ | 'V32'
v_angle_deg = 85   # published | https://online.flippingbook.com/view/795609261/46/ | '85°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/46/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/46/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/46/ | not in text layer
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/46/ | '100 t/m'
codes = "0355401/2"   # published | https://online.flippingbook.com/view/795609261/46/ | codes; suffix 3 = 'Sec'
segments = "835 / 415 (codes ...1/...2)"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

[[die]]
id = "amada/37"   # published | https://online.flippingbook.com/view/795609261/46/ | 'Tool #37'
maker = "Amada"   # published | https://online.flippingbook.com/view/795609261/46/ | AMADA Tooling Catalogue 2023 text layer; read 2026-11-24
clamping = "AMADA"   # published (series); tang dims unknown | https://online.flippingbook.com/view/795609261/46/ | Heavy Duty 1V; other numbers: 50, 60, 60 (meaning UNKNOWN)
v_opening_mm = 50   # published | https://online.flippingbook.com/view/795609261/46/ | 'V50'
v_angle_deg = 85   # published | https://online.flippingbook.com/view/795609261/46/ | '85°'
shoulder_radius_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/46/ | not in text layer
die_height_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/46/ | not in text layer
die_width_mm = "UNKNOWN"   # unknown | https://online.flippingbook.com/view/795609261/46/ | not in text layer
max_load_t_per_m = 100   # published | https://online.flippingbook.com/view/795609261/46/ | '100 t/m'
codes = "0375401/2"   # published | https://online.flippingbook.com/view/795609261/46/ | codes; suffix 3 = 'Sec'
segments = "835 / 415 (codes ...1/...2)"   # published | https://online.flippingbook.com/view/795609261/4/ | 'Sectionalised Die: 100, 10, 15, 15, 20, 40, 50, 150, 400 = 800 mm'
material = "AMANIT, surface HRC 60-65, body 1150 N/mm2"   # published | https://online.flippingbook.com/view/795609261/4/ | Die Quality section

### Bystronic punches (RF-A)

[[punch]]
id = "bystronic/P2_RF-A_H175_R0.5"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | page title 'P2 RF-A H175 R0.5'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P2 RF-A H175 R0.5"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "offset (light gooseneck; drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | '[H] Height (mm) 175'
tip_radius_mm = 0.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | '[R] Radius (mm) 0.5'
tip_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | 'Nose angle 85°'
max_load_kN_per_m = 850   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | 'F max = 850 kN/m'
weight_kg_per_m = 24.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | '24.0 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
dim_body_overall_mm = 30   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | 30: tang rear face to body front face
throat_depth_mm = 2.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | 2.5: tip line to neck rear (relief) face
neck_thickness_mm = 17.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | 17.5 neck
tip_flank_length_mm = 10   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | 10 along flank
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0002.5145 25:65206.0002.5148 100:65206.0002.5163 200:65206.0002.5076 515:65206.0002.5035 1030:65206.0002.3017 FSW550:65206.0002.9003 FSC1000:65206.0002.0037 Horn100:65206.0002.7018"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.13) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P2_RF-A_H250_R0.5"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | page title 'P2 RF-A H250 R0.5'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P2 RF-A H250 R0.5"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "offset (light gooseneck; drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | profile drawing
height_mm = 250   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | '[H] Height (mm) 250'
tip_radius_mm = 0.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | '[R] Radius (mm) 0.5'
tip_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | 'Nose angle 85°'
max_load_kN_per_m = 850   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | 'F max = 850 kN/m'
weight_kg_per_m = 34.4   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | '34.4 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
dim_body_overall_mm = 30   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | 30
throat_depth_mm = 2.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | 2.5
neck_thickness_mm = 17.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | 17.5
tip_flank_length_mm = 10   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | 10
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0002.5166 25:65206.0002.5169 100:65206.0002.5184 200:65206.0002.5078 515:65206.0002.5055 1030:65206.0002.3016 FSW550:65206.0002.9005 FSC1000:65206.0002.0027 Horn100:65206.0002.7030"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.14) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P2_RF-A_H175_R0.5_Nose_7_mm"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | page title 'P2 RF-A H175 R0.5 Nose 7 mm'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P2 RF-A H175 R0.5 Nose 7 mm"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "offset (light gooseneck; drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | '[H] Height (mm) 175'
tip_radius_mm = 0.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | '[R] Radius (mm) 0.5'
tip_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | 'Nose angle 85°'
max_load_kN_per_m = 500   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | 'F max = 500 kN/m'
weight_kg_per_m = 24.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | '24.0 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
dim_body_overall_mm = 30   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | 30
throat_depth_mm = 2.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | 2.5
neck_thickness_mm = 17.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | 17.5
tip_flank_length_mm = 7   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | 7 along flank ('Nose 7 mm')
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0002.5187 25:65206.0002.5190 100:65206.0002.5205 200:65206.0002.5077 515:65206.0002.5075 1030:65206.0002.3018 FSW550:65206.0002.9004 FSC1000:65206.0002.0029 Horn100:65206.0002.7033"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.15) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P2_RF-A_H175_R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | page title 'P2 RF-A H175 R1.0'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P2 RF-A H175 R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "offset (light gooseneck; drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | '[H] Height (mm) 175'
tip_radius_mm = 1.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | '[R] Radius (mm) 1.0'
tip_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | 'Nose angle 85°'
max_load_kN_per_m = 1000   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | 'F max = 1000 kN/m'
weight_kg_per_m = 24.1   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | '24.1 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
dim_body_overall_mm = 30   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | 30
throat_depth_mm = 2.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | 2.5
neck_thickness_mm = 17.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | 17.5
tip_flank_length_mm = 10   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | 10
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0002.5108 25:65206.0002.5111 100:65206.0002.5126 200:65206.0002.5088 515:65206.0002.5001 1030:65206.0002.3004 FSW550:65206.0002.9001 FSC1000:65206.0002.0003 Horn100:65206.0002.7001"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.17) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P3_RF-A_H175_R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | page title 'P3 RF-A H175 R1.0'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P3 RF-A H175 R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "offset (light gooseneck; drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | '[H] Height (mm) 175'
tip_radius_mm = 1.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | '[R] Radius (mm) 1.0'
tip_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | 'Nose angle 85°'
max_load_kN_per_m = 1300   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | 'F max = 1300 kN/m'
weight_kg_per_m = 24.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | '24.5 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
dim_body_overall_mm = 30   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | 30
throat_depth_mm = 2.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | 2.5
neck_thickness_mm = 17.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | 17.5
tip_flank_length_mm = 12   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | 12 along flank
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0003.5097 25:65206.0003.5100 100:65206.0003.5115 200:65206.0003.5116 515:65206.0003.5005 1030:65206.0003.3005 FSW550:65206.0003.9001 FSC1000:65206.0003.0002 Horn100:65206.0003.7006"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.19) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P3_RF-A_H175_R3.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | page title 'P3 RF-A H175 R3.0'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P3 RF-A H175 R3.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "offset (light gooseneck; drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | '[H] Height (mm) 175'
tip_radius_mm = 3.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | '[R] Radius (mm) 3.0'
tip_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | 'Nose angle 85°'
max_load_kN_per_m = 1100   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | 'F max = 1100 kN/m'
weight_kg_per_m = 24.2   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | '24.2 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
dim_body_overall_mm = 30   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | 30
throat_depth_mm = 2.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | 2.5
neck_thickness_mm = 17.5   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | 17.5
tip_flank_length_mm = 12   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | 12
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0003.5140 25:65206.0003.5143 100:65206.0003.5158 200:65206.0003.5159 515:65206.0003.5027 1030:65206.0003.3006 FSW550:65206.0003.9008 FSC1000:65206.0003.0021 Horn100:65206.0003.7015"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.21) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P4_RF-A_H175_R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | page title 'P4 RF-A H175 R1.0'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P4 RF-A H175 R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "gooseneck (drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | '[H] Height (mm) 175'
tip_radius_mm = 1.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | '[R] Radius (mm) 1.0'
tip_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | 'Nose angle 85°'
max_load_kN_per_m = 600   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | 'F max = 600 kN/m'
weight_kg_per_m = 32.2   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | '32.2 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
dim_body_overall_mm = 60   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | 60: tang rear face to body front (top)
throat_depth_mm = 22   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | 22: tip line to throat
neck_thickness_mm = 28   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | 28 neck
tip_flank_length_mm = 9   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | 9 along flank
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0004.5366 25:65206.0004.5369 100:65206.0004.5382 200:65206.0004.5224 515:65206.0004.5013 1030:65206.0004.3011 FSW550:65206.0004.9027 FSC1000:65206.0004.0006 Horn100:65206.0004.7006"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.25) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P4_RF-A_H250_R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | page title 'P4 RF-A H250 R1.0'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P4 RF-A H250 R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "gooseneck (drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | profile drawing
height_mm = 250   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | '[H] Height (mm) 250'
tip_radius_mm = 1.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | '[R] Radius (mm) 1.0'
tip_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | 'Nose angle 85°'
max_load_kN_per_m = 600   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | 'F max = 600 kN/m'
weight_kg_per_m = 48.7   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | '48.7 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
dim_body_overall_mm = 60   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | 60
throat_depth_mm = 22   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | 22
neck_thickness_mm = 28   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | 28
tip_flank_length_mm = 9   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | 9
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0004.5384 25:65206.0004.5245 100:65206.0004.5274 200:65206.0004.5176 515:65206.0004.5012 1030:65206.0004.3027 FSW550:65206.0004.9019 FSC1000:65206.0004.0008 Horn100:65206.0004.7007"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.26) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P5_RF-A_H175_R0.5"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | page title 'P5 RF-A H175 R0.5'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P5 RF-A H175 R0.5"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "straight acute (drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | '[H] Height (mm) 175'
tip_radius_mm = 0.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | '[R] Radius (mm) 0.5'
tip_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | 'Nose angle 30°'
max_load_kN_per_m = 700   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | 'F max = 700 kN/m'
weight_kg_per_m = 19.3   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | '19.3 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
top_width_mm = 22   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | 22 at top
blade_thickness_mm = 12   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | 12 blade
tip_flank_length_mm = 10   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | 10 along flank
throat_depth_mm = "UNKNOWN"   # unknown | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | not dimensioned
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0005.5182 25:65206.0005.5211 100:65206.0005.5224 200:65206.0005.5169 515:65206.0005.5046 1030:65206.0005.3023 FSW550:65206.0005.9011 FSC1000:65206.0005.0050 Horn100:65206.0005.7040"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.31) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P5_RF-A_H250_R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | page title 'P5 RF-A H250 R1.0'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P5 RF-A H250 R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "straight acute (drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | profile drawing
height_mm = 250   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | '[H] Height (mm) 250'
tip_radius_mm = 1.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | '[R] Radius (mm) 1.0'
tip_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | 'Nose angle 30°'
max_load_kN_per_m = 750   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | 'F max = 750 kN/m'
weight_kg_per_m = 26.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | '26.0 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
top_width_mm = 22   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | 22
blade_thickness_mm = 12   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | 12
tip_flank_length_mm = 10   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | 10
throat_depth_mm = "UNKNOWN"   # unknown | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | not dimensioned
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0005.5012 25:65206.0005.5096 100:65206.0005.5044 200:65206.0005.5078 515:65206.0005.5008 1030:65206.0005.3005 FSW550:65206.0005.9002 FSC1000:65206.0005.0004 Horn100:65206.0005.7001"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.34) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P6_RF-A_H175_R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.39) | page title 'P6 RF-A H175 R1.0'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P6 RF-A H175 R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.39) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "straight (drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.39) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.39) | '[H] Height (mm) 175'
tip_radius_mm = 1.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.39) | '[R] Radius (mm) 1.0'
tip_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.39) | 'Nose angle 30°'
max_load_kN_per_m = 1500   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.39) | 'F max = 1500 kN/m'
weight_kg_per_m = 23.3   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.39) | '23.3 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
throat_depth_mm = "UNKNOWN"   # unknown | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.39) | not dimensioned
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0006.5045 25:65206.0006.5048 100:65206.0006.5063 200:65206.0006.5064 515:65206.0006.5001 1030:65206.0006.3003 FSW550:65206.0006.9001 FSC1000:65206.0006.0001 Horn100:65203.0006.7002 (sic)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.39) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P7_RF-A_H175_R3.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.41) | page title 'P7 RF-A H175 R3.0'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P7 RF-A H175 R3.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.41) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "straight (drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.41) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.41) | '[H] Height (mm) 175'
tip_radius_mm = 3.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.41) | '[R] Radius (mm) 3.0'
tip_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.41) | 'Nose angle 30°'
max_load_kN_per_m = 1600   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.41) | 'F max = 1600 kN/m'
weight_kg_per_m = 24.2   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.41) | '24.2 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
throat_depth_mm = "UNKNOWN"   # unknown | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.41) | not dimensioned
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0007.5157 25:65206.0007.5158 100:65206.0007.5082 200:65206.0007.5103 515:65206.0007.5002 1030:65206.0007.3004 FSW550:65206.0007.9001 FSC1000:65206.0007.0003 Horn100:65206.0007.7001"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.41) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P7_RF-A_H175_R6.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.45) | page title 'P7 RF-A H175 R6.0'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P7 RF-A H175 R6.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.45) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "straight (drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.45) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.45) | '[H] Height (mm) 175'
tip_radius_mm = 6.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.45) | '[R] Radius (mm) 6.0'
tip_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.45) | 'Nose angle 30°'
max_load_kN_per_m = 1600   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.45) | 'F max = 1600 kN/m'
weight_kg_per_m = 25.7   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.45) | '25.7 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
throat_depth_mm = "UNKNOWN"   # unknown | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.45) | not dimensioned
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0007.5227 25:65206.0007.5230 100:65206.0007.5245 200:65206.0007.5246 515:65206.0007.5019 1030:65206.0007.3009 FSW550:65206.0007.9006 FSC1000:65206.0007.0014 Horn100:65206.0007.7017"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.45) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P8_RF-A_H175_R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | page title 'P8 RF-A H175 R1.0'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P8 RF-A H175 R1.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "gooseneck (drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | '[H] Height (mm) 175'
tip_radius_mm = 1.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | '[R] Radius (mm) 1.0'
tip_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | 'Nose angle 85°'
max_load_kN_per_m = 1000   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | 'F max = 1000 kN/m'
weight_kg_per_m = 27.2   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | '27.2 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
dim_body_overall_mm = 40   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | 40: tang rear face to body front
throat_depth_mm = 10   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | 10: tip line to throat
neck_thickness_mm = 20   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | 20 neck
tip_flank_length_mm = 18   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | 18 along flank
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0008.5062 25:65206.0008.5065 100:65206.0008.5080 200:65206.0008.5041 515:65206.0008.5002 1030:65206.0008.3003 FSW550:65206.0008.9001 FSC1000:65206.0008.0001 Horn100:65206.0008.7001"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.53) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

[[punch]]
id = "bystronic/P9_RF-A_H175_R4.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.55) | page title 'P9 RF-A H175 R4.0'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue (file 2502436, PDF created 2026-05-19); read 2026-11-24
name = "P9 RF-A H175 R4.0"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.55) | title
clamping = "Bystronic RF-A (tang 20 mm wide x 62 mm; clamping height 60 mm)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | RF-A drawing: 20, 62, 60; 'Head support <=1600 kN/m, Shoulder supp. >3200 kN/m'
kind = "straight (drawing)"   # estimate (Bystronic gives no kind name; read from drawing) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.55) | profile drawing
height_mm = 175   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.55) | '[H] Height (mm) 175'
tip_radius_mm = 4.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.55) | '[R] Radius (mm) 4.0'
tip_angle_deg = 60   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.55) | 'Nose angle 60°'
max_load_kN_per_m = 2000   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.55) | 'F max = 2000 kN/m'
weight_kg_per_m = 33.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.55) | '33.5 kg/m'
material = "58 - 62 HRC"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.3) | 'hardness: 58 - 62 HRC' (catalogue icon legend)
body_width_mm = 30   # published (drawing; reference lines read by eye) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.55) | 30 body width
throat_depth_mm = "UNKNOWN"   # unknown | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.55) | not dimensioned
body_width_at_clamp_mm = 20   # published (RF-A system drawing p.8) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.8) | tang width 20
segments_part_no = "10:65206.0009.5161 25:65206.0009.5164 100:65206.0009.5046 200:65206.0009.5043 515:65206.0009.5006 1030:65206.0009.3003 FSW550:65206.0009.9001 FSC1000:65206.0009.0001 Horn100:65206.0009.7001"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.55) | standard length : part number (all of 10..100 in 5 mm steps also listed)
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+100HL+100HR"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.11) | Segmentations of punches

### Bystronic dies (RF-A bottom tools)

[[die]]
id = "bystronic/V4-30deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | page title 'V4/30° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V4/30° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 4   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | title 'V4'
v_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | '[a] V-Angle 30°'
shoulder_radius_mm = 0.75   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | '[R] Radius (mm) 0.75'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 20   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | '[B] Width (mm) 20'
v_depth_mm = 5.3   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | '[D] Depth (mm) 5.3'
max_load = "F max 30° = 220 kN/m; F max 90° = 600 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | 'F max 30° = 220 kN/m; F max 90° = 600 kN/m'
weight_kg_per_m = 11.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | '11.0 kg/m'
segments_part_no = "10:64201.0004.5008 50:64201.0004.5019 100:64201.0004.5028 200:64201.0004.5029 515:64201.0004.5004 FSW550:64201.0004.9001 FSC1000:64201.0004.0001 1030:64201.0004.3001"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 version p.75"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | H100 version p.75
foot_width_mm = 25   # published (drawing of V4 pages) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.74) | foot width 25 on narrow-die drawings (p.74/75); confirm per page

[[die]]
id = "bystronic/V4-85deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | page title 'V4/85° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V4/85° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 4   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | title 'V4'
v_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | '[a] V-Angle 85°'
shoulder_radius_mm = 0.75   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | '[R] Radius (mm) 0.75'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 20   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | '[B] Width (mm) 20'
v_depth_mm = 1.8   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | '[D] Depth (mm) 1.8'
max_load = "F max 90° = 600 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | 'F max 90° = 600 kN/m'
weight_kg_per_m = 11.1   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | '11.1 kg/m'
segments_part_no = "10:64201.0004.5010 50:64201.0004.5060 100:64201.0004.5070 200:64201.0004.5071 515:64201.0004.5074 FSW550:64201.0004.9003 FSC1000:64201.0004.0003 1030:64201.0004.3003"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 version p.77"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | H100 version p.77
foot_width_mm = 25   # published (drawing of V4 pages) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.76) | foot width 25 on narrow-die drawings (p.74/75); confirm per page

[[die]]
id = "bystronic/V6-30deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | page title 'V6/30° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V6/30° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 6   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | title 'V6'
v_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | '[a] V-Angle 30°'
shoulder_radius_mm = 0.75   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | '[R] Radius (mm) 0.75'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 20   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | '[B] Width (mm) 20'
v_depth_mm = 9.3   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | '[D] Depth (mm) 9.3'
max_load = "F max 30° = 280 kN/m; F max 90° = 800 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | 'F max 30° = 280 kN/m; F max 90° = 800 kN/m'
weight_kg_per_m = 10.9   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | '10.9 kg/m'
segments_part_no = "10:64201.0006.5034 50:64201.0006.5030 100:64201.0006.5008 200:64201.0006.5023 515:64201.0006.5004 FSW550:64201.0006.9001 FSC1000:64201.0006.0001 1030:64201.0006.3008"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.83"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | H100 p.83
foot_width_mm = 25   # published (drawing of V4 pages) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.82) | foot width 25 on narrow-die drawings (p.74/75); confirm per page

[[die]]
id = "bystronic/V6-85deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | page title 'V6/85° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V6/85° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 6   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | title 'V6'
v_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | '[a] V-Angle 85°'
shoulder_radius_mm = 0.75   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | '[R] Radius (mm) 0.75'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 20   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | '[B] Width (mm) 20'
v_depth_mm = 3.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | '[D] Depth (mm) 3.0'
max_load = "F max 90° = 800 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | 'F max 90° = 800 kN/m'
weight_kg_per_m = 11.1   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | '11.1 kg/m'
segments_part_no = "10:64201.0006.5055 50:64201.0006.5064 100:64201.0006.5215 200:64201.0006.5216 515:64201.0006.5005 FSW550:64201.0006.9006 FSC1000:64201.0006.0005 1030:64201.0006.3011"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.85"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | H100 p.85
foot_width_mm = 25   # published (drawing of V4 pages) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.84) | foot width 25 on narrow-die drawings (p.74/75); confirm per page

[[die]]
id = "bystronic/V8-30deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | page title 'V8/30° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V8/30° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 8   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | title 'V8'
v_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | '[a] V-Angle 30°'
shoulder_radius_mm = 0.75   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | '[R] Radius (mm) 0.75'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 20   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | '[B] Width (mm) 20'
v_depth_mm = 11.2   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | '[D] Depth (mm) 11.2'
max_load = "F max 30° = 300 kN/m; F max 90° = 1000 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | 'F max 30° = 300 kN/m; F max 90° = 1000 kN/m'
weight_kg_per_m = 10.8   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | '10.8 kg/m'
segments_part_no = "10:64201.0008.5053 50:64201.0008.5096 100:64201.0008.5010 200:64201.0008.5032 515:64201.0008.5012 FSW550:64201.0008.9001 FSC1000:64201.0008.0001 1030:64201.0008.3005"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.91"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | H100 p.91
foot_width_mm = 25   # published (drawing of V4 pages) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.90) | foot width 25 on narrow-die drawings (p.74/75); confirm per page

[[die]]
id = "bystronic/V8-85deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | page title 'V8/85° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V8/85° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 8   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | title 'V8'
v_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | '[a] V-Angle 85°'
shoulder_radius_mm = 1.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | '[R] Radius (mm) 1.0'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 20   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | '[B] Width (mm) 20'
v_depth_mm = 4.1   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | '[D] Depth (mm) 4.1'
max_load = "F max 90° = 850 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | 'F max 90° = 850 kN/m'
weight_kg_per_m = 11.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | '11.0 kg/m'
segments_part_no = "10:64201.0008.5302 50:64201.0008.5224 100:64201.0008.5234 200:64201.0008.5235 515:64201.0008.5304 FSW550:64201.0008.9014 FSC1000:64201.0008.0034 1030:64201.0008.3018"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.93"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | H100 p.93
foot_width_mm = 25   # published (drawing of V4 pages) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.92) | foot width 25 on narrow-die drawings (p.74/75); confirm per page

[[die]]
id = "bystronic/V10-30deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | page title 'V10/30° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V10/30° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 10   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | title 'V10'
v_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | '[a] V-Angle 30°'
shoulder_radius_mm = 1.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | '[R] Radius (mm) 1.0'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 20   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | '[B] Width (mm) 20'
v_depth_mm = 14.9   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | '[D] Depth (mm) 14.9'
max_load = "F max 30° = 200 kN/m; F max 90° = 700 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | 'F max 30° = 200 kN/m; F max 90° = 700 kN/m'
weight_kg_per_m = 10.6   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | '10.6 kg/m'
segments_part_no = "10:64201.0010.5018 50:64201.0010.5120 100:64201.0010.5040 200:64201.0010.5043 515:64201.0010.5016 FSW550:64201.0010.9001 FSC1000:64201.0010.0004 1030:64201.0010.3006"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.99"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | H100 p.99
foot_width_mm = 25   # published (drawing of V4 pages) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.98) | foot width 25 on narrow-die drawings (p.74/75); confirm per page

[[die]]
id = "bystronic/V10-85deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | page title 'V10/85° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V10/85° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 10   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | title 'V10'
v_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | '[a] V-Angle 85°'
shoulder_radius_mm = 1.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | '[R] Radius (mm) 1.0'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 20   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | '[B] Width (mm) 20'
v_depth_mm = 5.2   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | '[D] Depth (mm) 5.2'
max_load = "F max 90° = 800 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | 'F max 90° = 800 kN/m'
weight_kg_per_m = 10.6   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | '10.6 kg/m'
segments_part_no = "10:64201.0010.5217 50:64201.0010.5225 100:64201.0010.5235 200:64201.0010.5236 515:64201.0010.5017 FSW550:64201.0010.9010 FSC1000:64201.0010.0010 1030:64201.0010.3009"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.101"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | H100 p.101
foot_width_mm = 25   # published (drawing of V4 pages) | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.100) | foot width 25 on narrow-die drawings (p.74/75); confirm per page

[[die]]
id = "bystronic/V12-30deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | page title 'V12/30° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V12/30° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 12   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | title 'V12'
v_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | '[a] V-Angle 30°'
shoulder_radius_mm = 1.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | '[R] Radius (mm) 1.5'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 25   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | '[B] Width (mm) 25'
v_depth_mm = 18.6   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | '[D] Depth (mm) 18.6'
max_load = "F max 30° = 250 kN/m; F max 90° = 850 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | 'F max 30° = 250 kN/m; F max 90° = 850 kN/m'
weight_kg_per_m = 11.8   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | '11.8 kg/m'
segments_part_no = "10:64201.0012.5018 50:64201.0012.5124 100:64201.0012.5122 200:64201.0012.5043 515:64201.0012.5007 FSW550:64201.0012.9001 FSC1000:64201.0012.0002 1030:64201.0012.3009"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.107"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.106) | H100 p.107

[[die]]
id = "bystronic/V12-85deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | page title 'V12/85° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V12/85° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 12   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | title 'V12'
v_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | '[a] V-Angle 85°'
shoulder_radius_mm = 1.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | '[R] Radius (mm) 1.5'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 25   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | '[B] Width (mm) 25'
v_depth_mm = 6.3   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | '[D] Depth (mm) 6.3'
max_load = "F max 90° = 900 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | 'F max 90° = 900 kN/m'
weight_kg_per_m = 12.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | '12.5 kg/m'
segments_part_no = "10:64201.0012.5197 50:64201.0012.5205 100:64201.0012.5079 200:64201.0012.5215 515:64201.0012.5008 FSW550:64201.0012.9013 FSC1000:64201.0012.0007 1030:64201.0012.3014"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.109"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.108) | H100 p.109

[[die]]
id = "bystronic/V16-30deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | page title 'V16/30° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V16/30° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 16   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | title 'V16'
v_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | '[a] V-Angle 30°'
shoulder_radius_mm = 2.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | '[R] Radius (mm) 2.0'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | '[B] Width (mm) 30'
v_depth_mm = 20.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | '[D] Depth (mm) 20.5'
max_load = "F max 30° = 300 kN/m; F max 90° = 1000 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | 'F max 30° = 300 kN/m; F max 90° = 1000 kN/m'
weight_kg_per_m = 13.3   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | '13.3 kg/m'
segments_part_no = "10:64201.0016.5053 50:64201.0016.5020 100:64201.0016.5113 200:64201.0016.5038 515:64201.0016.5011 FSW550:64201.0016.9001 FSC1000:64201.0016.0001 1030:64201.0016.3006"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.115"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.114) | H100 p.115

[[die]]
id = "bystronic/V16-85deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | page title 'V16/85° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V16/85° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 16   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | title 'V16'
v_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | '[a] V-Angle 85°'
shoulder_radius_mm = 2.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | '[R] Radius (mm) 2.0'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | '[B] Width (mm) 30'
v_depth_mm = 8.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | '[D] Depth (mm) 8.5'
max_load = "F max 90° = 1200 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | 'F max 90° = 1200 kN/m'
weight_kg_per_m = 14.4   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | '14.4 kg/m'
segments_part_no = "10:64201.0016.5182 50:64201.0016.5190 100:64201.0016.5200 200:64201.0016.5062 515:64201.0016.5023 FSW550:64201.0016.9004 FSC1000:64201.0016.0009 1030:64201.0016.3008"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.117"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.116) | H100 p.117

[[die]]
id = "bystronic/V20-30deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | page title 'V20/30° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V20/30° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 20   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | title 'V20'
v_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | '[a] V-Angle 30°'
shoulder_radius_mm = 2.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | '[R] Radius (mm) 2.5'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | '[B] Width (mm) 30'
v_depth_mm = 28   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | '[D] Depth (mm) 28'
max_load = "F max 30° = 180 kN/m; F max 90° = 600 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | 'F max 30° = 180 kN/m; F max 90° = 600 kN/m'
weight_kg_per_m = 12.2   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | '12.2 kg/m'
segments_part_no = "10:64201.0020.5038 50:64201.0020.5043 100:64201.0020.5024 200:64201.0020.5025 515:64201.0020.5002 FSW550:64201.0020.9001 FSC1000:64201.0020.0002 1030:64201.0020.3007"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.123"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.122) | H100 p.123

[[die]]
id = "bystronic/V20-85deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | page title 'V20/85° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V20/85° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 20   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | title 'V20'
v_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | '[a] V-Angle 85°'
shoulder_radius_mm = 2.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | '[R] Radius (mm) 2.5'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | '[B] Width (mm) 30'
v_depth_mm = 10.4   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | '[D] Depth (mm) 10.4'
max_load = "F max 90° = 1400 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | 'F max 90° = 1400 kN/m'
weight_kg_per_m = 14.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | '14.0 kg/m'
segments_part_no = "10:64201.0020.5087 50:64201.0020.5095 100:64201.0020.5123 200:64201.0020.5078 515:64201.0020.5013 FSW550:64201.0020.9004 FSC1000:64201.0020.0009 1030:64201.0020.3012"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.125"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.124) | H100 p.125

[[die]]
id = "bystronic/V24-30deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | page title 'V24/30° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V24/30° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 24   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | title 'V24'
v_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | '[a] V-Angle 30°'
shoulder_radius_mm = 3.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | '[R] Radius (mm) 3.0'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 40   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | '[B] Width (mm) 40'
v_depth_mm = 29.8   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | '[D] Depth (mm) 29.8'
max_load = "F max 30° = 330 kN/m; F max 90° = 1100 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | 'F max 30° = 330 kN/m; F max 90° = 1100 kN/m'
weight_kg_per_m = 15.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | '15.5 kg/m'
segments_part_no = "10:64201.0024.5047 50:64201.0024.5061 100:64201.0024.5058 200:64201.0024.5030 515:64201.0024.5010 FSW550:64201.0024.9001 FSC1000:64201.0024.0001 1030:64201.0024.3005"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.127"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.126) | H100 p.127

[[die]]
id = "bystronic/V24-85deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | page title 'V24/85° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V24/85° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 24   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | title 'V24'
v_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | '[a] V-Angle 85°'
shoulder_radius_mm = 3.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | '[R] Radius (mm) 3.0'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 40   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | '[B] Width (mm) 40'
v_depth_mm = 12.6   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | '[D] Depth (mm) 12.6'
max_load = "F max 90° = 1700 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | 'F max 90° = 1700 kN/m'
weight_kg_per_m = 18.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | '18.0 kg/m'
segments_part_no = "10:64201.0024.5012 50:64201.0024.5081 100:64201.0024.5091 200:64201.0024.5092 515:64201.0024.5011 FSW550:64201.0024.9006 FSC1000:64201.0024.0002 1030:64201.0024.3007"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.129"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.128) | H100 p.129

[[die]]
id = "bystronic/V32-30deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | page title 'V32/30° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V32/30° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 32   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | title 'V32'
v_angle_deg = 30   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | '[a] V-Angle 30°'
shoulder_radius_mm = 4.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | '[R] Radius (mm) 4.0'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 48   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | '[B] Width (mm) 48'
v_depth_mm = 33.6   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | '[D] Depth (mm) 33.6'
max_load = "F max 30° = 340 kN/m; F max 90° = 1100 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | 'F max 30° = 340 kN/m; F max 90° = 1100 kN/m'
weight_kg_per_m = 16.7   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | '16.7 kg/m'
segments_part_no = "10:64201.0032.5053 50:64201.0032.5062 100:64201.0032.5020 200:64201.0032.5039 515:64201.0032.5010 FSW550:64201.0032.9001 FSC1000:64201.0032.0002 1030:64201.0032.3006"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.131"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.130) | H100 p.131

[[die]]
id = "bystronic/V32-85deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | page title 'V32/85° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V32/85° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 32   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | title 'V32'
v_angle_deg = 85   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | '[a] V-Angle 85°'
shoulder_radius_mm = 4.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | '[R] Radius (mm) 4.0'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 48   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | '[B] Width (mm) 48'
v_depth_mm = 16.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | '[D] Depth (mm) 16.5'
max_load = "F max 90° = 2500 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | 'F max 90° = 2500 kN/m'
weight_kg_per_m = 20.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | '20.5 kg/m'
segments_part_no = "10:64201.0032.5012 50:64201.0032.5099 100:64201.0032.5067 200:64201.0032.5059 515:64201.0032.5011 FSW550:64201.0032.9005 FSC1000:64201.0032.0003 1030:64201.0032.3009"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.133"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.132) | H100 p.133

[[die]]
id = "bystronic/V40-60deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | page title 'V40/60° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V40/60° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 40   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | title 'V40'
v_angle_deg = 60   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | '[a] V-Angle 60°'
shoulder_radius_mm = 5.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | '[R] Radius (mm) 5.0'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | '[B] Width (mm) 55'
v_depth_mm = 29.6   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | '[D] Depth (mm) 29.6'
max_load = "F max 60° = 1100 kN/m; F max 90° = 1800 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | 'F max 60° = 1100 kN/m; F max 90° = 1800 kN/m'
weight_kg_per_m = 20.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | '20.5 kg/m'
segments_part_no = "10:64201.0040.5035 50:64201.0040.5083 100:64201.0040.5010 200:64201.0040.5022 515:64201.0040.5003 FSW550:64201.0040.9001 FSC1000:64201.0040.0002 1030:64201.0040.3007"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.135 (F max 90° = 2000 kN/m there)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.134) | H100 p.135 (F max 90° = 2000 kN/m there)

[[die]]
id = "bystronic/V50-60deg_H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | page title 'V50/60° H55'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V50/60° H55"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 50   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | title 'V50'
v_angle_deg = 60   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | '[a] V-Angle 60°'
shoulder_radius_mm = 5.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | '[R] Radius (mm) 5.0'
die_height_mm = 55   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | '[H] Height (mm) 55' (drawing: H excludes the 20 mm tang below)
die_width_mm = 70   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | '[B] Width (mm) 70'
v_depth_mm = 30.3   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | '[D] Depth (mm) 30.3'
max_load = "F max 60° = 1100 kN/m; F max 90° = 1800 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | 'F max 60° = 1100 kN/m; F max 90° = 1800 kN/m'
weight_kg_per_m = 24.5   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | '24.5 kg/m'
segments_part_no = "10:64201.0050.5027 50:64201.0050.5063 100:64201.0050.5033 200:64201.0050.5016 515:64201.0050.5003 FSW550:64201.0050.9001 FSC1000:64201.0050.0001 1030:64201.0050.3003"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.137"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.136) | H100 p.137

[[die]]
id = "bystronic/V60-60deg_H65"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | page title 'V60/60° H65'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V60/60° H65"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 60   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | title 'V60'
v_angle_deg = 60   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | '[a] V-Angle 60°'
shoulder_radius_mm = 7.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | '[R] Radius (mm) 7.0'
die_height_mm = 65   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | '[H] Height (mm) 65' (drawing: H excludes the 20 mm tang below)
die_width_mm = 80   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | '[B] Width (mm) 80'
v_depth_mm = 39   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | '[D] Depth (mm) 39'
max_load = "F max 60° = 1000 kN/m; F max 90° = 1600 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | 'F max 60° = 1000 kN/m; F max 90° = 1600 kN/m'
weight_kg_per_m = 31.4   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | '31.4 kg/m'
segments_part_no = "10:64201.0060.5033 50:64201.0060.5045 100:64201.0060.5037 200:64201.0060.5029 515:64201.0060.5005 FSW550:64201.0060.9001 FSC1000:64201.0060.0002 1030:64201.0060.3005"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.139"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.138) | H100 p.139

[[die]]
id = "bystronic/V80-80deg_H65"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | page title 'V80/80° H65'
maker = "Bystronic"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf | Bystronic bending tool catalogue; read 2026-11-24
name = "V80/80° H65"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | title
clamping = "Bystronic RF-A bottom tool, die insert 13 mm (tang 13 wide x 20 high on drawing)"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | 'Die insert 13 mm'; drawing 13 and 20
v_opening_mm = 80   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | title 'V80'
v_angle_deg = 80   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | '[a] V-Angle 80°'
shoulder_radius_mm = 10.0   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | '[R] Radius (mm) 10.0'
die_height_mm = 65   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | '[H] Height (mm) 65' (drawing: H excludes the 20 mm tang below)
die_width_mm = 100   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | '[B] Width (mm) 100'
v_depth_mm = 38.7   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | '[D] Depth (mm) 38.7'
max_load = "F max 80° = 1400 kN/m; F max 90° = 1600 kN/m"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | 'F max 80° = 1400 kN/m; F max 90° = 1600 kN/m'
weight_kg_per_m = 38.6   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | '38.6 kg/m'
segments_part_no = "10:64201.0080.5035 50:64201.0080.5058 100:64201.0080.5043 200:64201.0080.5025 515:64201.0080.5002 FSW550:64201.0080.9001 FSC1000:64201.0080.0001 1030:64201.0080.3005"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | standard length : part number
sectioned_sets = "FSC 1000 = 10+15+20+30+50+100+200+275+300; FSW 550 = 20+25+30+35+40+200+2x100"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.73) | Bottom tool segmentation
notes = "H100 p.141"   # published | https://www.bystronic.com/sites/default/files/2026-06/2502436_Bending_Werkzeugkatalog_ENG_compressed.pdf (p.140) | H100 p.141

## 4. Conflicts noted

- Trumpf die loads: per-die page values agree with the p.300 max-load table for every die listed here (e.g. EV/S-W4/30° 150 kN/m both places, EV W50/30° 1200 kN/m both). No conflict found.
- Trumpf EV001 30° shoulder radius: overview p.14 'R 0,6' and drawing p.188 'R0,6' agree.
- Bystronic V40/60°: F max 90° = 1800 kN/m at H55 (p.134) vs 2000 kN/m at H100 (p.135); V12/85° 900 kN/m both heights; V12S/85° 700 (H55, p.112) vs 900 (H100, p.113). Different tools, recorded as published; hold each page's value for its own height.
- Amada vs Trumpf/Bystronic V rules agree (6t/8t/10t/12t); Trumpf stops at 10 x s for 9-12 mm.

## 5. Not reachable / not collected

- Amada EU PDFs (NEW AFH Standard, AFH 2021, bending price list, high-tonnage) and amada.eu product pages: HTTP 403 behind a Cloudflare challenge for both curl and WebFetch. Not bypassed. Amada data therefore come only from the text layer of Amada's FlippingBook 2023 catalogue: punch kind, throat, body/tang widths, die heights and shoulder radii are UNKNOWN because they live in the page images, which load only through signed URLs.
- LVD: no openly published LVD tooling catalogue found (lvdgroup.com tooling URLs 404; search returned only third-party 'LVD-style' makers, which are out of scope).
- Accurpress: tooling page reachable but publishes no dimensions (points to phone/dealer; a tonnage calculator on dealersupport.accurpress.com was not used).
- Trumpf tang (OW/K, OW/S) and EV die tang dimensions: not dimensioned anywhere in the catalogue.
- Bystronic heavy-duty / holder upper tools (pp.57-69) and hemming tools skipped (no tip radius/angle).
- Not collected for lack of time: Trumpf 84°/86° narrow dies beyond those listed, Trumpf radius-insert tools (OW320, OWB09), ACB tools; Bystronic H100 die variants are referenced by page in each die's notes but not given their own block.
