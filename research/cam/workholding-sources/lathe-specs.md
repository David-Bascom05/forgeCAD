# Lathe workholding: published dimensional specs

All values were read on 2026-11-23 from public pages or PDFs that needed no login. "UNKNOWN" means no source that could be reached published the value. The mm column is converted from the published unit (1 in = 25.4 mm) and rounded to 0.01 mm; where the published unit is already mm, the value is copied unchanged.

Abbreviations: "pub." means published value. Letter dimensions (A, B, ...) are the source's own labels.

---

## 1. Kitagawa B-200 series power chucks (3-jaw, open centre)

Kitagawa's own site (kitagawa.global) refused automated access, returning 403 to curl and 404 to the fetcher. The Kitagawa Europe catalog mirrors on Cutwel and DirectIndustry also refused. The values below come from a US distributor, Benchmark Tooling, which publishes a spec table for each MPN.

Sources:
- B206: https://benchmarktooling.com/products/67295915
- B208: https://benchmarktooling.com/products/83072389
- B210: https://benchmarktooling.com/products/67295949 (listing is MPN B210 supplied on an A2-8 adapter)

| field | B-206 pub. | B-206 mm | B-208 pub. | B-208 mm | B-210 pub. | B-210 mm | source | accessed | notes |
|---|---|---|---|---|---|---|---|---|---|
| Chuck outside diameter | 152.40 mm / 6 in | 152.40 | 209.95 mm / 8 in | 209.95 | 254.00 mm / 10 in | 254.00 | Benchmark pages above | 2026-11-23 | B-206 figure looks like the nominal 6" size class, not the measured body. Search-engine snippets of kitagawa.global describe the B-206 as "169 mm", but the page could not be opened, so that figure is unverified. |
| Chuck body length/height | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | - | 2026-11-23 | Not on the distributor pages. Kitagawa drawings were not reachable. |
| Through-hole | 45.00 mm (1.7717 in) | 45.00 | 52.00 mm (2.0472 in) | 52.00 | 75.01 mm (2.9530 in) | 75.01 | Benchmark | 2026-11-23 | |
| Jaw stroke (on diameter) | 5.50 mm | 5.50 | 7.40 mm | 7.40 | 8.79 mm (0.3460 in) | 8.79 | Benchmark | 2026-11-23 | Labelled "Jaw Stroke Diameter". |
| Piston (plunger) stroke | 0.5910 in | 15.01 | 0.8660 in | 22.00 | 0.9840 in | 24.99 | Benchmark | 2026-11-23 | Labelled "Piston Stroke Length". |
| Gripping range min | 16.00 mm | 16.00 | 13.00 mm | 13.00 | 31.00 mm | 31.00 | Benchmark | 2026-11-23 | Labelled "Minimum Jaw Capacity". |
| Gripping range max | 169.00 mm | 169.00 | 210.00 mm | 210.00 | 254.00 mm | 254.00 | Benchmark | 2026-11-23 | Labelled "Maximum Jaw Capacity". |
| Max rpm | 6,000 | - | 5000 | - | 4200 | - | Benchmark | 2026-11-23 | |
| Number of jaws | 3 | - | 3 | - | 3 | - | Benchmark | 2026-11-23 | |
| Max static gripping force | 57.00 kN | - | 86.00 kN | - | 111.00 kN | - | Benchmark | 2026-11-23 | |
| Max drawbar pull | 22.00 kN | - | 34.80 kN | - | 43.00 kN | - | Benchmark | 2026-11-23 | |
| Jaw interface | 1.5 mm x 60 serrated | - | 1.5mm x 60 Serrated | - | 1.5mm x 60 Serrated | - | Benchmark | 2026-11-23 | |
| Mount | Plain Back | - | Plain Back | - | A2-8 | - | Benchmark | 2026-11-23 | |
| Matching cylinder | S1246 | - | S1552 | - | S1875 | - | Benchmark | 2026-11-23 | |

### Soft and hard jaws for Kitagawa-style 1.5 mm x 60 deg serration

The only jaw data reachable was from Shars, an aftermarket supplier. No Kitagawa SJ-series drawing could be reached.

| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| 6" round soft jaw (Shars SKU 202-6473): jaw height | 1.57"/40mm | 40 | https://www.shars.com/6-round-soft-steel-chuck-jaw-set-for-kitagawa-1 | 2026-11-23 | Fits 6" Kitagawa-type chucks. Aftermarket part, not Kitagawa. |
| same: groove (key) width | 0.472"/12mm | 12 | same | 2026-11-23 | |
| same: fastener | 3/8"/M10 | - | same | 2026-11-23 | |
| same: bolt hole spacing | 0.787"/20mm | 20 | same | 2026-11-23 | |
| same: jaw width / length | UNKNOWN | UNKNOWN | same | 2026-11-23 | Not printed. |
| 10" round soft jaw (Shars SKU 202-6475): jaw height | 1.968"/50mm | 50 | https://www.shars.com/10-round-soft-steel-chuck-jaw-set-for-kitagawa-1 | 2026-11-23 | |
| same: groove width | 0.63"/16mm | 16 | same | 2026-11-23 | |
| same: fastener | 1/2"/M12 | - | same | 2026-11-23 | |
| same: bolt hole spacing | 1.181"/30mm | 30 | same | 2026-11-23 | |
| same: jaw width / length | UNKNOWN | UNKNOWN | same | 2026-11-23 | |
| B-208 hard jaws (height, width, length) | UNKNOWN | UNKNOWN | https://www.shars.com/products/workholding/lathe-chuck-jaws/8-reversible-double-step-hard-steel-chuck-jaw-set-for-kitagawa-b208-1 | 2026-11-23 | The page returned HTTP 202 with an empty body. A search snippet showed width 1.38, length 3.42 and height 2.01 (in), but the page could not be read, so these are not recorded as verified. |

### 4-jaw chuck (Bison-Bial, DIN 702-4 combination four-jaw, 200 mm)

Source: Hoffmann Group, item 315635-200, brand BISON BIAL: https://www.hoffmann-group.com/IN/en/hin/p/315635-200

| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Outer diameter A | 200 mm | 200 | Hoffmann 315635-200 | 2026-11-23 | Recessed mount. |
| Height without jaws H1 | 96 mm | 96 | same | 2026-11-23 | |
| Height with jaws H2 | 127 mm | 127 | same | 2026-11-23 | |
| Through-hole ("Aperture diameter E") | 55 mm | 55 | same | 2026-11-23 | |
| Centring diameter B | 160 mm | 160 | same | 2026-11-23 | |
| Centring hole depth C | 5 mm | 5 | same | 2026-11-23 | |
| Pitch circle F | 176 mm | 176 | same | 2026-11-23 | |
| Square K (key) | 11 mm | 11 | same | 2026-11-23 | |
| Number of jaws | 4 | - | same | 2026-11-23 | |
| Weight | 19 kg | - | same | 2026-11-23 | |
| Gripping range | UNKNOWN | UNKNOWN | - | 2026-11-23 | Not on the page. The Bison catalog PDF host reset the connection. |
| Max rpm | UNKNOWN | UNKNOWN | - | 2026-11-23 | |

---

## 2. Royal Products

### 2a. Royal Low-Profile Accu-Length CNC collet chucks (dead-length)

Source: Royal Products "RP 2021 Low Profile Workholding Catalog", table "Royal Low-Profile Accu-Length CNC Collet Chucks": https://royalproducts.com/wp-content/uploads/2021/01/RP-2021-Low-Profile-Workholding-Catalog.pdf

Royal's table labels are: A = overall length, B = nose diameter, C = nose length, D = body diameter, E = flange diameter.

| field | 5C A2-5 (#42330) pub. | mm | 5C A2-6 (#42332) pub. | mm | 16C A2-5 (#42338) pub. | mm | accessed | notes |
|---|---|---|---|---|---|---|---|---|
| Capacity (round) | 1-1/16" | 26.99 | 1-1/16" | 26.99 | 1-5/8" | 41.28 | 2026-11-23 | |
| A Overall length | 5.75 in | 146.05 | 5.75 in | 146.05 | 6.50 in | 165.10 | 2026-11-23 | |
| B Nose diameter | 2.43 in | 61.72 | 2.43 in | 61.72 | 3.25 in | 82.55 | 2026-11-23 | |
| C Nose length | 1.00 in | 25.40 | 1.00 in | 25.40 | 1.03 in | 26.16 | 2026-11-23 | |
| D Body diameter | 4.00 in | 101.60 | 4.00 in | 101.60 | 3.80 in | 96.52 | 2026-11-23 | |
| E Flange diameter | 6.45 in | 163.83 | 7.45 in | 189.23 | 6.45 in | 163.83 | 2026-11-23 | |
| Weight | 21 lbs | 9.53 kg | 21 lbs | 9.53 kg | 25 lbs | 11.34 kg | 2026-11-23 | |
| Max drawbar force | 4,000 lbs | - | 4,000 lbs | - | 5,400 lbs | - | 2026-11-23 | |
| Max clamping force | 13,600 lbs | - | 13,600 lbs | - | 18,300 lbs | - | 2026-11-23 | |
| Max rpm | 6,000 | - | 6,000 | - | 6,000 | - | 2026-11-23 | |

### 2b. Royal Low-Profile Pullback CNC collet chucks (same catalog)

| field | 5C A2-5 (#42053) pub. | mm | 5C A2-6 6" (#42057) pub. | mm | accessed | notes |
|---|---|---|---|---|---|---|
| Capacity | 1-1/16" | 26.99 | 1-1/16" | 26.99 | 2026-11-23 | |
| A Overall length | 5.00 in | 127.00 | 6.00 in | 152.40 | 2026-11-23 | |
| B Nose diameter | 2.63 in | 66.80 | 2.63 in | 66.80 | 2026-11-23 | |
| C Nose length | 1.05 in | 26.67 | 1.05 in | 26.67 | 2026-11-23 | |
| D Body diameter | 3.14 in | 79.76 | 3.14 in | 79.76 | 2026-11-23 | |
| E Flange diameter | 6.45 in | 163.83 | 7.45 in | 189.23 | 2026-11-23 | |
| Weight | 12 lbs | 5.44 kg | 19 lbs | 8.62 kg | 2026-11-23 | |
| Max rpm | 6,000 | - | 6,000 | - | 2026-11-23 | |

Catalog footnote, quoted: "Threaded-nose collet chucks are for use with 5C step collets that close on the head, not the 10-degree angle."

### 2c. Royal ER collet chuck for lathes

UNKNOWN. No Royal ER lathe collet chuck spec table turned up in search, and I did not crawl royalproducts.com further.

### 2d. Royal High-Precision Quad-Bearing Live Centers, standard point

Source: Royal "2024 RP CATALOG - LIVE AND DEAD CENTERS", catalog page 93 (PDF page 9): https://royalproducts.com/pdfs/2024%20RP%20CATALOG%20-%20LIVE%20AND%20DEAD%20CENTERS.pdf

I read the letter meanings from the catalog drawing: B = head (body) diameter, G = diameter at the base of the point cone, F = from the point tip to the head face, E = head length from its front face to the start of the taper shank.

| field | 2 MT / 3 MT (#10412/#10413) | mm | 4 MT / 5 MT (#10414/#10415) | mm | 5 MT HD / 6 MT HD (#10445/#10446) | mm | accessed | notes |
|---|---|---|---|---|---|---|---|---|
| Point angle | "Standard point angle is 60° +15' / -0" | - | same | - | same | - | 2026-11-23 | Stated on the quad-bearing pages. |
| B head diameter | 1.70 in | 43.18 | 2.45 in | 62.23 | 3.82 in | 97.03 | 2026-11-23 | |
| E head length | 2.12 in | 53.85 | 2.78 in | 70.61 | 3.89 in | 98.81 | 2026-11-23 | |
| F point projection | 1.07 in | 27.18 | 1.47 in | 37.34 | 2.31 in | 58.67 | 2026-11-23 | |
| G point base diameter | 0.88 in | 22.35 | 1.25 in | 31.75 | 2.00 in | 50.80 | 2026-11-23 | |
| Max suggested rpm | 6,000 | - | 5,000 | - | 3,000 | - | 2026-11-23 | |
| Max workpiece weight | 885 lbs | - | 2,240 lbs | - | 5,240 lbs | - | 2026-11-23 | |
| Thrust load | 1,270 lbs | - | 2,150 lbs | - | 5,300 lbs | - | 2026-11-23 | |
| Runout | "+- 0.00005" TIR guaranteed" | 0.00127 | same | - | same | - | 2026-11-23 | |

---

## 3. Collet standards

### 3a. ER collets (DIN 6499 / ISO 15488)

Sources:
- ISCAR "ER Collet Type DIN 6499" datasheet, "Basic Dimensions" table: https://www.iscar.com/eCatalog/Ecat/datafileENM/INFO/ER.pdf
- Vertex ER instructions (Chronos copy), which give the same A/B values: https://www.chronos.ltd.uk/wp-content/uploads/erinst.pdf
- REGO-FIX "ER System" catalog (COL1125 01/2015; distributor copy): https://www.productivity.com/wp-content/uploads/2020/04/Rego-Fix-_ER_System-3-25-20-Interactive.pdf

| field | ER16 | ER25 | ER32 | ER40 | unit | source | accessed | notes |
|---|---|---|---|---|---|---|---|---|
| Collet OD "A" | 17 | 26 | 33 | 41 | mm | ISCAR, Vertex | 2026-11-23 | |
| Collet length "B" | 27 | 35 | 40 | 46 | mm | ISCAR, Vertex | 2026-11-23 | |
| Smallest collet in range (nominal, clamps down to) | 1.00 – 0.50 | 1.00 – 0.50 | 2.00 – 1.00 | 3.00 – 2.00 | mm | REGO-FIX metric tables | 2026-11-23 | REGO-FIX writes ranges as "max – min". |
| Largest collet in range | 10.00 – 9.00 | 17.00 – 16.00 | 22.00 – 21.00 | 30.00 – 29.00 | mm | REGO-FIX | 2026-11-23 | ER25 17, ER32 21/22 and ER40 27–30 are REGO-FIX extended sizes. The largest marked "*" (included in sets) are ER25 16.00–15.00, ER32 20.00–19.00 and ER40 26.00–25.00. |
| Collapse range per collet | 1.0 (0.5 for sizes up to 1 mm) | 1.0 | 1.0 | 1.0 | mm | REGO-FIX tables (e.g. "6.00 – 5.00") | 2026-11-23 | Read from the tables: each collet spans 1 mm, except the smallest sizes. |
| Collet taper angle | UNKNOWN as text | | | | | ISCAR drawing | 2026-11-23 | The ISCAR drawing carries the annotations "8°" and "30°", but no text says which is which. Not recorded as a verified half-angle. |
| Nut OD "A" (REGO-FIX Hi-Q/ER standard hex & spanner nut) | 28 | 42 | 50 | 63 | mm | REGO-FIX p.32 | 2026-11-23 | The drawing shows A as the outer diameter and B as the nut length. Other nut styles differ, e.g. the ER16 MS mini nut is 22 mm. |
| Nut length "B" | 17.5 | 20.0 | 22.5 | 25.5 | mm | REGO-FIX p.32 | 2026-11-23 | |
| Nut thread "C" | M22 x 1.50 | M32 x 1.50 | M40 x 1.50 | M50 x 1.50 | - | REGO-FIX p.32 | 2026-11-23 | |
| Recommended nut torque (ISCAR std nut) | 7 | 20 | 22 | 25 | Kgxm (as printed) | ISCAR | 2026-11-23 | |

Usage rule (REGO-FIX, quoted): "Insert tool to the full length of the collet for best results if possible. However, never insert the tool less than 2/3 of the collet bore length."

### 3b. 5C collets (and 16C for comparison)

Source: Hardinge "Spindle Tooling" catalog 2348J, "Dimensions and Capacities" table: https://forkardthardinge.com/wp-content/uploads/2348J_SpindleTooling.pdf

| field | 5C published | mm | 16C published | mm | accessed | notes |
|---|---|---|---|---|---|---|
| A back bearing diameter (collet OD) | 1.250" | 31.75 | 1.889" | 47.98 | 2026-11-23 | |
| B overall length | 3.281" | 83.34 | 4.312" | 109.52 | 2026-11-23 | |
| C thread | 1.238" x 20 RH | - | 1.870" x 1.75mm RH | - | 2026-11-23 | |
| D head diameter | 1.468" | 37.29 | 2.265" | 57.53 | 2026-11-23 | |
| Max round capacity | 1-1/16" | 26.99 | 1-5/8" | 41.28 | 2026-11-23 | The PDF text renders 1-1/16 as "11⁄16"; the printed mm value (26.99) confirms it. |
| Max hex capacity | 29/32" | 23.02 | 1-13/32" | 35.72 | 2026-11-23 | |
| Max square capacity | 3/4" | 19.05 | 1-9/64" | 28.97 | 2026-11-23 | |
| Internal stop thread | 1.041" x 24 RH | - | 1.687" x 20 TPI RH | - | 2026-11-23 | |
| Nose taper angle | "the 10-degree angle" (Royal catalog wording, §2b) | - | UNKNOWN | - | 2026-11-23 | Royal does not say whether 10° is the half-angle or the included angle. A Zoro listing reportedly gives "Taper Angle 20 Degrees" for Lyndex-Nikken 5C collets, but Zoro refused access (403), so that is unverified. |

---

## 4. Face drivers

### 4a. Neidlein (now sold under Kitagawa) FSB, movable centre pin, flange mount

Source: NEIDLEIN Face Driver catalog (distributor-hosted copy): https://www.acomee.com.mx/catalogos/catalogosn/NEIDLEIN%20FACE%20DRIVER.pdf

Columns are as printed: d, d1, centre Ø, d2, d3, d5, d6, l1, l2, drive pin (count), fastening screw (type, pc), clamping diameter D1/D2/D3. All in mm. The text does not define d1, d2, d5 or d6, so they are omitted. Note from the catalog: "All face drivers are supplied without drive pins."

| type (cat. no.) | d (body Ø) | centre Ø range | drive pins | clamping Ø D1 / D2 / D3 | l1 | l2 | accessed |
|---|---|---|---|---|---|---|---|
| FSB 01 (73012) | 48 | 0–5 | 3 | 8 / 11 / 17 | 115 | 28 | 2026-11-23 |
| FSB 1 (73002) | 48 | 0–8 | 3 | 13 / 18 / 26 | 115 | 28 | 2026-11-23 |
| FSB 2 (73003) | 70 | 2–14 | 6 | 26 / 31 / 36 | 115 | 23 | 2026-11-23 |
| FSB 3 (73004) | 70 | 2–18 | 6 | 34 / 39 / 44 | 115 | 33 | 2026-11-23 |
| FSB 4 (73005) | 90 | 3–24 | 6 | 39 / 49 / 59 | 115 | 72 | 2026-11-23 |
| FSB 5 (73006) | 132 | 3–35 | 6 | 69 / 84 / 99 | 115 | 74 | 2026-11-23 |
| FSB 6 (73007) | 212 | 3–35 | 6 | 140 / 155 / 170 | 115 | 165 | 2026-11-23 |
| FSB 8 (73016) | 360 | 30–76 | 6 | 270 / 290 / 310 | 190 | 262 | 2026-11-23 |

The SB series, with a Morse taper (MK 3–6) shank, has the same d, centre Ø, pin count and clamping Ø per size (same PDF).

Guideline from the same catalog, quoted: "Ratio D/d should not exceed 2, otherwise it would work inefficiently." Here D is the cutting diameter and d is the clamping diameter.

### 4b. Riten mechanical face drivers

Sources:
- Type 40: https://cdn.dgisupply.ca/img/product-images/Asset_URL/Riten_4003_Catalog.pdf
- Type 47: https://cdn.dgisupply.ca/img/product-images/Asset_URL/Riten_4705_Catalog.pdf

Both are Riten catalog pages hosted by a distributor.

| field | Type 40 pub. | mm | Type 47 pub. | mm | accessed | notes |
|---|---|---|---|---|---|---|
| Driving diameter range | 0.32" - 0.67" | 8.13 – 17.02 | 1.54" - 2.32" | 39.12 – 58.93 | 2026-11-23 | |
| Drive pins required | 3 | - | 6 | - | 2026-11-23 | |
| Centre-hole dia. range (centre points) | 0 - 0.200 (C4001) | 0 – 5.08 | 0.120 - 0.950 (C4701) up to 1.535 - 1.732 (C4706) | 3.05 – 24.13 … 38.99 – 43.99 | 2026-11-23 | Choose the centre point whose range is closest to the centre hole. |
| Mount | #3/#4/#5 Morse Taper, Flange Mount | - | #5/#6 Morse Taper, Flange Mount | - | 2026-11-23 | Spindle adapters for A2-5, A2-6, A2-8 and A2-11. |

---

## 5. Tailstock centres: Morse tapers and 60° point

Sources:
- Little Machine Shop "Dimensions of Standard Tapers" (inch): https://littlemachineshop.com/pages/dimensions-of-standard-tapers
- cgtk.co.uk "Morse Taper Dimensions" (metric; columns not defined on the page): https://www.cgtk.co.uk/metalwork/data/morse
- Wikipedia "Machine taper" (cross-check; cites ISO 296 / DIN 228-1): https://en.wikipedia.org/wiki/Machine_taper

These are not the ISO 296 text itself, which is paywalled. Little Machine Shop's large-end values match cgtk's D1 to within rounding.

| MT | Large end (LMS, in) | mm | D1 (cgtk, mm) | Small end (LMS, in) | Length (LMS, in) | mm | L2 (cgtk, mm) | Taper/ft (LMS, in/ft) | Half-angle (cgtk) | accessed |
|---|---|---|---|---|---|---|---|---|---|---|
| 2 | 0.7000 | 17.78 | 17.780 | 0.5720 | 2.56 | 65.02 | 65.087 | 0.5994 | 1°25'49" | 2026-11-23 |
| 3 | 0.9380 | 23.83 | 23.825 | 0.7780 | 3.19 | 81.03 | 80.962 | 0.6024 | 1°26'16" | 2026-11-23 |
| 4 | 1.2310 | 31.27 | 31.267 | 1.0200 | 4.06 | 103.12 | 103.188 | 0.6233 | 1°29'15" | 2026-11-23 |
| 5 | 1.7480 | 44.40 | 44.399 | 1.4750 | 5.19 | 131.83 | 131.762 | 0.6315 | 1°30'26" | 2026-11-23 |

60° centre point:
- Royal live centers state "Standard point angle is 60° +15' / -0" (source in §2d).
- No public copy of a standard such as DIN 332 / ISO 866 for centre holes was accessed, so the standard itself is UNKNOWN here.

---

## 6. Steady rests

### 6a. Röhm SLZ / SLZB self-centring steady rests

Source: Röhm operating instructions SL - SLZ / SLZB, DC0000335, §1.2 "Clamping range": https://www.roehm.biz/fileadmin/products/media/en/DC0000335.PDF

| size | clamping range (without chip guard) | mm | accessed | notes |
|---|---|---|---|---|
| SLZ 047 | 4 -- 70 | 4–70 | 2026-11-23 | |
| SLZ/SLZB 08105 | 8 -- 105 | 8–105 | 2026-11-23 | |
| SLZ/SLZB 1152 | 11 -- 152 | 11–152 | 2026-11-23 | |
| SLZ/SLZB 1517 | 15 -- 170 | 15–170 | 2026-11-23 | |
| SLZ/SLZB 40200 | 40 -- 200 | 40–200 | 2026-11-23 | |
| SLZ/SLZB 325 | 35 -- 250 | 35–250 | 2026-11-23 | |
| SLZ/SLZB 50315 (531) | 50 -- 315 | 50–315 | 2026-11-23 | |
| SLZ/SLZB 95360 | 95 -- 360 | 95–360 | 2026-11-23 | |
| Height above centreline / centre-to-base | UNKNOWN | UNKNOWN | 2026-11-23 | Not in the manual text. The Röhm e-shop data sheet (qminst.roehm.biz) returned 503. |

### 6b. SMW-Autoblok SLU

All fields are UNKNOWN. The DirectIndustry PDF pages refused access (403). The SMW FlippingBook pages that loaded (394, 406, 414) were other series (SLUB, SR-CL, RX), and their tables came out garbled. No SLU numbers were recorded.

---

## 7. Grip length and overhang guidance (guidelines, not standards)

| rule (quoted exactly) | publisher / document | source URL | accessed | notes |
|---|---|---|---|---|
| "Workpieces longer than 3 times the chucking diameter must be supported by the tailstock or a steady rest." | JET, Geared-Head Lathe GH-1340 Operation and Maintenance Instructions, Part No. M-JT1-2708, Edition 2 02/2026, safety item 33 | https://www.trick-tools.com/common/documentation/gh1340_manual.pdf | 2026-11-23 | GUIDELINE. Machine maker's manual, distributor-hosted copy. |
| "stock protruding more than three times its diameter should be supported by the live center." | Grizzly G4002 lathe manual, p.21 | https://www.manualslib.com/manual/513180/Grizzly-G4002.html?page=23 | 2026-11-23 | GUIDELINE. Same manual: "the tailstock barrel should protrude about 1/2'' and not more than 3''." |
| "Clamping length too short, overhang too long." (marked Incorrect) / "Additional support over center or rest." (marked Correct) | Optimum Optiturn TH 4210 manual, p.130 | https://www.manualslib.de/manual/988006/Optimum-Optiturn-Th-4210.html?page=130 | 2026-11-23 | Qualitative only; no number. |
| "Ratio D/d should not exceed 2, otherwise it would work inefficiently." | NEIDLEIN face driver catalog (D = cutting Ø, d = clamping Ø) | see §4a | 2026-11-23 | GUIDELINE for face drivers. |
| "never insert the tool less than 2/3 of the collet bore length." | REGO-FIX ER System catalog | see §3a | 2026-11-23 | GUIDELINE for minimum grip in ER collets (written for tools; applies to any shank). |

No chuck maker (Kitagawa, SMW, Röhm, Schunk) was found publishing a numeric minimum jaw grip length or maximum overhang ratio. The Haas/MMK chuck manual that might contain one refused access.

---

## Sites that refused or could not be read

| site / URL | result |
|---|---|
| kitagawa.global (product pages, e.g. /b-208a6, /in/products/.../b-206) | 404 to the fetcher; 403 to plain curl on every path, including the home page |
| cutwel.co.uk Kitagawa catalog PDFs | HTTP 405 from the Varnish cache server |
| pdf.directindustry.com, directindustry.com (Kitagawa B-200, SMW SLU) | HTTP 403 |
| shop.butlerbros.com (Kitagawa B210) | HTTP 403 |
| zoro.com (Lyndex-Nikken 5C collet) | HTTP 403 |
| shars.com B208 hard-jaw page | HTTP 202, empty body (likely a bot challenge) |
| lyndexnikken.com ER collet chuck PDF | HTTP 403 |
| haascnc.com MMK ZA chuck manual PDF | HTTP 403 |
| iastatedigitalpress.com (paper on L/D ratios) | Anubis "Access Denied" |
| ieferia.interempresas.net (Bison manual-chuck catalog PDF) | connection reset twice |
| qminst.roehm.biz (SLZN data sheet) | HTTP 503 |
| govets.com (Kitagawa B206) | HTTP 504 |
| mtdcnc.com Kitagawa page | HTTP 404 |
| online.flippingbook.com SMW-Autoblok catalog | loaded, but SLU pages not found and tables garbled |

No user agent was changed and no block was worked around.
