# Milling workholding: published specs (data gathering)

All values accessed **2026-11-23**. "Published value" is quoted in the source's own unit; "mm" column is a
plain unit conversion (1 in = 25.4 mm, 1 lbf = 4.44822 N, 1 ft-lb = 1.35582 N·m, 1 inHg = 3.38639 kPa,
1 psi = 6.89476 kPa). Where the source already prints mm, that is used as-is. UNKNOWN = not found in a public,
no-login source. Letter-coded catalog tables are flagged where the letter-to-feature mapping is my reading,
not printed beside the number.

---

## 1. Kurt DX6 / 3600V (D688 not found)

Kurt's own site (kurtworkholding.com) returned HTTP 403. Kurt's catalog PDF is mirrored by a distributor
(Mars Supply), and Penn Tool (distributor) prints labelled values.

- KC = Kurt catalog PDF, https://www.marssupply.com/userfiles/products/documents/k/KURT_DX6_Catalog.pdf (DX6 "Dimensional Data" p.12; 3600 series table)
- PT = https://www.penntoolco.com/kurt-dx6-crossover-precision-machine-vise-61-421-171/
- KF = https://www.nextind.com/customer/docs/skudocs/dx6-featurespdf.pdf (Kurt DX6 features sheet, distributor-hosted)

### DX6

| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| jaw width | 6" (PT); B = 6.000 (KC) | 152.40 | PT, KC | 2026-11-23 | |
| max opening | 9.08" (PT); AA = 9.080 (KC) | 230.63 | PT, KC | 2026-11-23 | KC text: "Large 9" opening", "9.0" versus 8.8" (D688)" |
| other opening positions | BB 12.117, CC 13.805, DD 16.842 (in) | 307.77 / 350.65 / 427.79 | KC | 2026-11-23 | under "Jaw Openings" drawing; positions (jaw plate placements) not labelled in text, my reading |
| jaw height (depth) | 1.735" (PT); H = 1.735 (KC) | 44.07 | PT, KC | 2026-11-23 | |
| overall length | 16.81" (PT); C = 16.810 (KC) | 426.97 | PT, KC | 2026-11-23 | |
| overall height | 4.61" | 117.09 | PT | 2026-11-23 | PT also lists "Vise height 4-3/8"" (111.13 mm); difference not explained |
| bed height (base to bed) | 2.875" (PT); A = 2.875 (KC) | 73.03 | PT, KC | 2026-11-23 | KC/KF: "Same bed height as D688" |
| body width | 7.39" (PT); I = 7.391 (KC) | 187.73 | PT, KC | 2026-11-23 | |
| clamp force | 6,884 lbf @ 80 ft-lb (table: 10 ft-lb 786 lbf ... 80 ft-lb 6,884 lbf) | 30.62 kN @ 108.5 N·m | KC | 2026-11-23 | max torque in table is 80 ft-lb |
| mounting thread | P = 1/2-13, P Metric = M12 | M12 | KC | 2026-11-23 | |
| weight | 65 lbs (KC); 69 lbs (PT) | 29.5 kg / 31.3 kg | KC, PT | 2026-11-23 | sources conflict |

### 3600V (same Kurt catalog, "Standard (3600) and Short (3620)" table; catalog also prints an mm column for 3600VM)

| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| jaw width | B = 6.000 in | 152.400 (printed) | KC | 2026-11-23 | |
| overall length | C = 15.060 in | 382.524 (printed) | KC | 2026-11-23 | 3620V short: 12.940 in |
| jaw height | H = 1.735 in | 44.069 (printed) | KC | 2026-11-23 | |
| A (likely bed height) | 3.187 in | 80.949 (printed) | KC | 2026-11-23 | label not in text; mapping UNCONFIRMED |
| opening positions | BB 9.500, CC 11.250, DD 14.750 in | 241.3 / 285.75 / 374.65 | KC | 2026-11-23 | catalog mm column is misaligned in this row block; letters UNCONFIRMED |
| clamp force | 6,356 lbf @ 80 ft-lb | 28.27 kN @ 108.5 N·m | KC | 2026-11-23 | |
| overall height, base width | UNKNOWN | | | | |
| **D688** (all fields) | UNKNOWN | | | | not in this catalog except as comparison ("opening 8.8"", 69 lbs) |

---

## 2. Orange Vise 6" dual station

Manufacturer page loads but publishes almost no geometry; it offers Parasolid models
(/content/OV6-160DS5.x_t ... OV6-260DS5.x_t) from which geometry could be measured.

- OV = https://www.orangevise.com/6x20-dual-station-vise/
- MT = https://www.maritool.com/p17494/6%22-x-20%22-Dual-Station-Orange-Vise/product_info.html
- MTJ = https://www.maritool.com/p20784/Jaws-for-Talon-Grip™-6x1.687x1-(Flush)/product_info.html

| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| jaw width | 6" (product name only) | 152.4 | OV | 2026-11-23 | nominal |
| body lengths offered | 16", 17.5", 20", 22", 26" | 406.4 / 444.5 / 508 / 558.8 / 660.4 | OV | 2026-11-23 | |
| max opening per station | UNKNOWN | | OV, MT | 2026-11-23 | only min gaps quoted (22" vise "2.5" gap in single station") |
| jaw height | 1.687" (flush dovetail jaw "flush with Orange Master Jaws") | 42.85 | MTJ | 2026-11-23 | accessory jaw 6" x 1.687" x 1"; implies master-jaw top, not a vise spec |
| overall height | UNKNOWN | | | | |
| max clamp force | 7200 lbs (120 ft lbs torque); ratio 600 lbs / 10 ft-lbs | 32.03 kN @ 162.7 N·m | OV, MT | 2026-11-23 | |
| bolt-on jaw pattern | 1/2"-13 in 3.875" x 0.9375" pattern | pattern 98.43 x 23.81 (thread is inch 1/2-13) | OV | 2026-11-23 | |
| weight (OV6-200DS5) | 110.00 LBS | 49.9 kg | OV | 2026-11-23 | |

---

## 3. Mitee-Bite

- MBP = https://www.miteebite.com/wp-content/uploads/2020/03/Pitbull-Clamp-Instr-METRIC-1.pdf
- MBC = https://www.miteebite.com/2024%20CATALOG-FINAL_8-24.pdf (2024 catalog; TalonGrip p.19-20, vacuum p.38-41)

### Pitbull clamps, metric (MBP; all mm). A = tap offset from workpiece edge, B/C = slot to mill, D* = minimum recommended clamp height, E = per drawing (unlabelled in text).

| part | A | B | C (clamp width) | D* min clamp height | E | screw | max torque N·m | max holding force N | total throw mm |
|---|---|---|---|---|---|---|---|---|---|
| 56000/56010 steel | 3.81 | 3.55 | 9.52 | 1.90 | 6.60 | M2.5 | 1.8 | 2,800 | 0.190 |
| 56020/56030 steel | 5.08 | 4.75 | 12.70 | 2.54 | 9.90 | M4 | 5.6 | 6,600 | 0.406 |
| 56050/56060 steel | 7.62 | 7.11 | 19.05 | 3.81 | 14.48 | M6 | 22.5 | 16,000 | 0.610 |
| 56070/56075 steel | 10.16 | 11.43 | 25.40 | 6.35 | 18.03 | M10 | 40.6 | 26,000 | 1.270 |
| 56080/56085 steel | 15.24 | 16.26 | 38.10 | 9.52 | 19.56 | M12 | 145.0 | 50,000 | 1.900 |

Source MBP, accessed 2026-11-23. Brass versions (56015/56040/56065) have lower torque/force. "Clamping range" as such
is not published; the clamp's range is the total throw above. Machinable versions 56077 (M10, 26,000 N) and 56088
(M12, 50,000 N) in MBC p.18; catalog note: "Maximum torque is not necessary. Normally half that value is sufficient."

### TalonGrip (MBC p.19-20)

| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| min grip | "clamping on as little as .060 (1.5mm)" | 1.5 | MBC | 2026-11-23 | |
| grip 33050 (metric) | A 19.05, B 12.7, C 6.35, M5; grip height 1.5-1.9 mm | same | MBC | 2026-11-23 | |
| fixture grip 33075 / 33150 | 19.05x19.05x7.92 M5, 1.5-3.0 mm; 25.4x25.4x12.7 M8, 1.5-5.6 mm | same | MBC | 2026-11-23 | |
| 6" jaw set 32066 | A 6.0 (150), B 1.73 (43.94), C 1.0 (25.4) | 150 / 43.94 / 25.4 | MBC | 2026-11-23 | letters per drawing; B likely jaw height (UNCONFIRMED) |
| long grip 33056 | 5.832" x 2.625" x .500", gripping height .050-.125" | 148.13 x 66.68 x 12.70; 1.27-3.17 | MBC | 2026-11-23 | gripping height .117" (2.97 mm) on standard jaws |

### Mitee-Bite fixture/grid plate

| field | published value | mm | source | accessed | notes |
|---|---|---|---|---|---|
| Multi-Power Vacuum Pallet | 14" x 16", grid plate tapped M6 | 355.6 x 406.4 | MBC | 2026-11-23 | grid pitch UNKNOWN |

---

## 4. LANG Technik

- LMG77 = https://lang-technik.de/en/products/48120-77-makro-grip-77-5-axis-vise-jaw-width-77-mm-clamping-range-0-120-mm
- LMG125 = https://lang-technik.de/en/products/48155-77-makro-grip-125-5-axis-vise-jaw-width-77-mm-clamping-range-0-155-mm
- LQP52 = https://lang-technik.de/en/products/45151-quick-point-52-single-plate-150-x-116-x-27-mm-without-mounting-bores (+ data sheet https://www.lang-technik.de/sites/default/files/139/pdf/45151_en_6.pdf)
- LQP96 = https://lang-technik.de/en/products/45400-quick-point-96-single-plate-192-x-156-x-27-mm-bores-150-x-100-50-mm-and-clamping-edge
- LFAQ = https://lang-technik.de/en/service/faq/zero-point-clamping-system

| field | published value | mm | source | accessed | notes |
|---|---|---|---|---|---|
| Makro-Grip 77 (48120-77) jaw width | 77 mm (3.03") | 77 | LMG77 | 2026-11-23 | other 77 lengths: 0-85, 0-160, 0-200 ranges |
| Makro-Grip 77 clamping range | 0 - 120 mm | 0-120 | LMG77 | 2026-11-23 | |
| Makro-Grip 77 L x W x H | 130 x 77 x 65 mm | height 65 | LMG77 | 2026-11-23 | |
| Makro-Grip 77 clamping depth | "clamping depth of just 3 mm" | 3 | LMG77 | 2026-11-23 | form-fit (pre-stamped serration) |
| Makro-Grip 77 clamping force | 14.000 N @ 60 Nm max | 14 kN | LMG77 | 2026-11-23 | |
| Makro-Grip 77 grid / stud | 52 mm; stud 16 mm (M8 threaded pin) | 52 / 16 | LMG77 | 2026-11-23 | |
| Makro-Grip 125 (48155-77) jaw width | 77 mm | 77 | LMG125 | 2026-11-23 | this item is the 125 body with 77 jaw; a 125-mm-jaw item was not checked |
| Makro-Grip 125 clamping range | 0 - 155 mm | 0-155 | LMG125 | 2026-11-23 | page also carries a stray "0 - 205 mm" (another length) |
| Makro-Grip 125 L x W x H | 160 x 125 x 89 mm | height 89 | LMG125 | 2026-11-23 | |
| Makro-Grip 125 clamping depth | 3 mm | 3 | LMG125 | 2026-11-23 | |
| Makro-Grip 125 clamping force | 20.000 N @ 100 Nm | 20 kN | LMG125 | 2026-11-23 | centering accuracy ±0,02 mm |
| Makro-Grip 125 grid / stud | 96 mm; stud 20 mm | 96 / 20 | LMG125 | 2026-11-23 | |
| Quick-Point 52 pitch | 52 mm | 52 | LFAQ | 2026-11-23 | |
| Quick-Point 52 stud dia | 16 mm (0.63") | 16 | LQP52 | 2026-11-23 | |
| Quick-Point 52 single plate | 150 x 116 x 27 mm, max torque 30 Nm, 3.3 kg | height 27 | LQP52 | 2026-11-23 | |
| Quick-Point 96 pitch / stud | 96 mm / 20 mm | 96 / 20 | LFAQ, LMG125 | 2026-11-23 | |
| Quick-Point 96 single plate | 192 x 156 x 27 mm | height 27 | LQP96 (title) | 2026-11-23 | from product URL/title; page body not fetched |
| repeatability | "0.005 mm" | 0.005 | LFAQ | 2026-11-23 | |
| pull-in force | "Depending on the zero point plate type, up to 14 kN" | 14 kN | LFAQ | 2026-11-23 | per-plate values UNKNOWN |

---

## 5. 5th Axis

- 5AC = https://5thaxis.com/wp-content/uploads/2022/09/2022-Summer-5th-Axis-Catalog.pdf (2022 Summer catalog)
- 5AD = https://5thaxis.com/wp-content/uploads/2021/04/V75100X-1.pdf (V75100X drawing/tech data)

| field | published value | mm | source | accessed | notes |
|---|---|---|---|---|---|
| RockLock 52 stud spacing | 52 mm | 52 | 5AC | 2026-11-23 | |
| RockLock 52 low-profile base RL52A-4/RL52S-4 | Ø116mm x 25mm | height 25 | 5AC | 2026-11-23 | RL52A-44 square: 103.5 x 103.5 x 25 mm |
| RockLock 96 base RL96A-56 | 158mm x 126mm x 30mm | height 30 | 5AC | 2026-11-23 | |
| RockLock repeatability | "+/- 8 μm" / "less than 8 μm" | 0.008 | 5AC | 2026-11-23 | |
| PS16F | 52mm pull stud, S7 tool steel | | 5AC | 2026-11-23 | stud diameter / length UNKNOWN in text (CAD available) |
| PS20F | 96mm pull stud | | 5AC | 2026-11-23 | dims UNKNOWN |
| V75100X jaw width | 2.95" (75mm) | 75 | 5AD, 5AC | 2026-11-23 | |
| V75100X size | 75mm x 100mm x 56mm | height 56 | 5AC | 2026-11-23 | drawing also shows 56.1 (unlabelled) |
| V75100X clamping range | 0 - 76.3mm | 0-76.3 | 5AC | 2026-11-23 | |
| V75100X clamping force | 5000 lbf @ 45 ft-lb (22 kN @ 60 Nm) | 22 kN | 5AD | 2026-11-23 | a distributor quotes 13.5 kN @ 60 Nm (not verified); use OEM drawing |
| V75100X centering | ± 0.0005" (± 12.7 μm) | 0.0127 | 5AD | 2026-11-23 | 4x fits PS16F at 52 |
| V510X (96 mm, closest to "V110"; no V110 found) | 125.7mm x 254mm x 73.6mm; jaw 125.7mm; fwd 0 - 184.7mm; rev 60.3mm - 234.7mm; 25.1 kN @ 75 Nm | height 73.6 | 5AC | 2026-11-23 | overview table says overall range 0 - 254.8mm |
| V562X (96 mm) | 126mm x 152mm x 74mm; overall range 0 - 153.6mm | height 74 | 5AC | 2026-11-23 | |
| vise jaw dovetail angle | "45° dovetail" | | 5AC, 5AD | 2026-11-23 | not 60° |
| dovetail fixture angle | "45° dovetail for increased clamping force" | | 5AC p.53 | 2026-11-23 | |
| dovetail depth required | DM12: 2.54mm; DM25: 2.54mm; DM50: 5.08mm | 2.54 / 5.08 | 5AC p.53 | 2026-11-23 | DM12 21x32x19 mm stock 12-25 mm; DM25 37x41x27 mm stock 25-75 mm; DM50 50x50x38 mm |
| R52-DM12 / R52-DM25 on 52 base | 75 x 75 x 51 mm / 75 x 75 x 55 mm | | 5AC | 2026-11-23 | |

---

## 6. Jergens

jergensinc.com product pages returned HTTP 403 (WebFetch). GlobalSpec datasheets: 403. DirectIndustry PDF viewer: 403.
One Jergens PDF on the same domain was downloadable:
JM = https://www.jergensinc.com/PDF_Catalogs/Jergens%20Mazak%20Ball%20Lock%20Guide%20Final%20-%20Metric.pdf

| field | published value | mm | source | accessed | notes |
|---|---|---|---|---|---|
| Ball Lock change-over repeatability | "within +/- 0.3 repeatability" | UNKNOWN unit | JM p.4 | 2026-11-23 | unit not printed |
| Ball Lock subplate thickness (Mazak kits) | 900 X 430 X 28 MM; 1300 X 550 X 32 MM | 28 / 32 | JM | 2026-11-23 | machine-specific subplates |
| Ball Lock grid pitch | UNKNOWN | | | | product page 403 |
| toe clamps, strap clamps, step block increments | UNKNOWN | | | | 403 |
| Fixture-Pro | UNKNOWN | | | | not reached |

---

## 7. Schunk

- SNSE3 = https://schunk.com/us/en/workpiece-clamping-technology/quick-change-pallet-systems/quick-change-pallet-system-for-general-milling-applications-and-heavy-duty-cutting/individual-modules-nse3/nse3-138-v1/p/000000000001313723
- SNSEM = https://schunk.com/us/en/workpiece-clamping-technology/quick-change-pallet-systems/quick-change-pallet-system-for-general-milling-applications-and-heavy-duty-cutting/individual-modules-nse-plus/nse-m-plus-138/p/000000000000471140
- SNSEC = https://schunk.com/us/en/workpiece-clamping-technology/quick-change-pallet-systems/quick-change-pallet-system-for-general-milling-applications-and-heavy-duty-cutting/individual-modules-nse-plus/c/PGR_4051
- SKSC = https://schunk.com/de/en/workpiece-clamping-technology/manual-clamping-systems/centric-clamping-vises/ksc3/ksc3-125-235/p/000000000001514242

| field | published value | mm | source | accessed | notes |
|---|---|---|---|---|---|
| NSE plus 138 (pneumatic) – all fields | UNKNOWN | | SNSEC | 2026-11-23 | category page lists no data; Schunk datasheet PDFs (dev.stb.cloud.schunk.com) 403 |
| NSE plus family repeatability | "< 0.005 mm"; system pressure 6 bar; turbo "up to 300%" | 0.005 | SNSEC | 2026-11-23 | |
| NSE3 138-V1 (current successor) dims | Ø138 x Ø110 x 55,7 mm | Ø138, h 55.7 | SNSE3 | 2026-11-23 | |
| NSE3 138-V1 pull-down force | 8 kN; 28 kN with turbo | | SNSE3 | 2026-11-23 | unlocking pressure 6 bar; 4.4 kg; repeat < 0.005 mm |
| NSE-M plus 138 (manual) | Ø138 x Ø110 x 43 mm; 2.5 kN; < 0.005 mm; 15 Nm | h 43 | SNSEM | 2026-11-23 | |
| KONTEC KSC3 125-235 | 125 x 235 x 50 mm; width 125.0 mm; 35 kN; 100 Nm; 9.5 kg | h 50 (body) | SKSC | 2026-11-23 | delivered without jaws |
| KSC jaw width / clamping range / jaw height | UNKNOWN | | SKSC | 2026-11-23 | depends on system jaws; not on page |

---

## 8. Generic standards

### DIN 650 / ISO 299 T-slots
| field | published value | mm | source | accessed | notes |
|---|---|---|---|---|---|
| slot 14 / 18 / 22: head width, head depth, throat depth | UNKNOWN | | https://6sku3uaup9f9.din.de/en/standard/din-650/1472525 | 2026-11-23 | standard is paywalled (DIN 650:1989-10). Engineers Edge ISO 299 table (https://engineersedge.com/hardware/dimension_table_for_tslots_bolts_and_nuts_13508.htm) has only 12/20 mm rows and its scraped columns were unaligned; direct curl was 403 |
| slot width tolerance | "H12 (H7 or H8 on request)" | | Scribd copy (not used; login-gated) | | not counted |

### DIN 508 T-slot nuts (Elesa/Ganter sheet) – EL = https://www.elesa.com/siteassets/PDF/PDF_DE/DIN%20508.pdf
a = tenon width (= slot width DIN 650), e = head length/width, h = overall height, k = tenon height (per drawing).

| slot | a | e | h | k | source | accessed |
|---|---|---|---|---|---|---|
| 14 | 14 (-0.3/-0.5) | 22 (-0.5) | 16 | 8 (-0.5) | EL | 2026-11-23 |
| 18 | 18 | 28 | 20 | 10 | EL | 2026-11-23 |
| 22 | 22 | 35 | 28 | 14 | EL | 2026-11-23 |

Cross-check McMaster DIN 508 table (https://www.mcmaster.com/products/t-slot-nuts/specifications-met~din-508/): 14/M12: base 22 x 22 x 8, overall ht 16; 18/M16: 28 x 28 x 10, 20; 22/M20: 35 x 35 x 14, 28. Agrees.

### DIN 6314 straight strap clamps – Kipp K1516 "Clamp straps DIN 6314 straight" (distributor-hosted)
KP = https://www.marek.eu/data/katalogove-listy/kipp/K1516.pdf (mm; F in kN). Letter meanings only on drawing: L length, A thickness (likely), B2 width (likely), B1 slot width (likely); UNCONFIRMED.

| order no. | L | A | B1 | B2 | B3 | E1 | E2 | F kN | screw |
|---|---|---|---|---|---|---|---|---|---|
| K1516.06 | 50 | 10 | 7 | 20 | 8 | 13.5 | 13 | 4.82 | M6 |
| K1516.08 | 60 | 12 | 9 | 25 | 10 | 14.5 | 13 | 8.77 | M8 |
| K1516.10 | 80 | 15 | 11 | 30 | 12 | 20.5 | 19 | 13.9 | M10 |
| K1516.12 | 100 | 20 | 14 | 40 | 14 | 28 | 26 | 20.2 | M12/M14 |
| K1516.14 | 125 | 20 | 14 | 40 | 14 | 28 | 36 | 20.2 | M12/M14 |
| K1516.16 | 125 | 25 | 18 | 50 | 18 | 35 | 27 | 37.8 | M16/M18 |
| K1516.18 | 160 | 25 | 18 | 50 | 18 | 35 | 47 | 37.8 | M16/M18 |
| K1516.20 | 160 | 30 | 22 | 60 | 22 | 41 | 38 | 58.8 | M20/M22 |
| K1516.24 | 200 | 30 | 26 | 70 | 26 | 48 | 54 | 84.7 | M24 |

Accessed 2026-11-23. **DIN 6315** (forked clamp): UNKNOWN (standard paywalled; no public table found).

### Parallels and 1-2-3 block
| field | published value | mm | source | accessed | notes |
|---|---|---|---|---|---|
| 1-2-3 block (Starrett 706) | 1 x 2 x 3" (25 x 50 x 75mm) | 25.4 x 50.8 x 76.2 | https://starrett.com/dms/flipbooks/Cat-33/inc/html/366.html | 2026-11-23 | Starrett's own mm is rounded |
| 1-2-3 parallelism / squareness / flatness | .0001" / .0001 in/in / .0001" | 0.003 | same | 2026-11-23 | RC 63-65; holes 11/32" thru, c'bore 17/32" |
| 1-2-3 size tolerance | UNKNOWN | | | | |
| parallel set (Fowler 52-437-125-0) | 10 pairs, 6" length, 1/8" | 152.4 / 3.175 | https://www.fowlerprecision.com/524371250 | 2026-11-23 | |
| parallel heights | 1/2", 5/8", 3/4", 7/8", 1", 1-1/8", 1-1/4", 1-3/8", 1-1/2", 1-5/8" | 12.7 ... 41.275 (3.175 steps) | same | 2026-11-23 | "Absolute heights are nominal" |
| parallel tolerance | parallelism ±.0002"; paired in height ±.00015" | ±0.005 / ±0.0038 | same | 2026-11-23 | 58-63 HRC |

---

## 9. Vacuum workholding

| field | published value | metric | source | accessed | notes |
|---|---|---|---|---|---|
| Schmalz rule | "For every 100 mbar difference in pressure, a vertical clamping force of approximately 1 N/cm² results." | 10 kPa → 0.01 MPa | https://www.schmalz.com/en/support/know-how/glossary/vacuum-clamping-technology | 2026-11-23 | i.e. F = Δp × A |
| Schmalz lateral hold | Fh = A × Δp × μ; μ ≈ 0.2 to 0.5 | | same | 2026-11-23 | |
| Schmalz example | 600 cm², 800 mbar, μ 0.25 → 1,200 N; "sufficient for ... approximately 400 cm³/min during roughing" | 80 kPa | same | 2026-11-23 | "illustrative purposes only" |
| Schmalz Matrix-Plate operating vacuum | -750 mbar (guide value); generator warning at -600 mbar | -75 kPa / -60 kPa | https://media.schmalz.com/MAM_Library/Dokumente/Montageanleitung/30/3030/303001/30300103058/15df503c6f01_MONT_30.30.01.03058_en-EN.pdf | 2026-11-23 | |
| Schmalz safety factor (handling, not machining) | S ≥ 1.5 smooth/dense; ≥ 2.0 porous/rough/oiled | | https://www.schmalz.com/en/vacuum-knowledge/the-vacuum-system-and-its-components/system-design-calculation-example/theoretical-holding-force-of-a-suction-cup | 2026-11-23 | load cases I-III; μ 0.5 metal |
| DATRON vacuum level | 25-29 inHg | 84.7-98.2 kPa | https://www.datron.com/resources/blog/9-things-you-need-to-know-about-vacuum-chucks/ | 2026-11-23 | |
| DATRON example | 81 in² x 14.7 psi = 1,190.7 lbs; 1 in² → 14.7 lbs | 101.4 kPa; 5.30 kN | same | 2026-11-23 | no safety factor / min area given |
| Mitee-Bite pump | "approximately 12-13 psi of vacuum holding force" at dead head | 82.7-89.6 kPa | MBC p.40 | 2026-11-23 | |
| Mitee-Bite gauge example | 12" x 10" = 120" x 12.5 psi = 1500 lbs | 86.2 kPa; 6.67 kN | MBC p.40 | 2026-11-23 | |
| Witte R3 grid plate (via Hoffmann) | 600 x 400 x 32.5 mm; slit spacing 12.5 mm; M10; ZeroClamp 200 mm pitch | | https://www.hoffmann-group.com/GB/en/houk/p/385030-R3 | 2026-11-23 | vacuum level UNKNOWN |
| minimum part area rule | UNKNOWN | | | | none of the sources publishes one |
| Pierson | UNKNOWN | | | | no public spec found |

---

## Sites that refused / could not be used
- kurtworkholding.com (HTTP 403)
- jergensinc.com product pages (HTTP 403); datasheets.globalspec.com (403); pdf.directindustry.com (403)
- dev.stb.cloud.schunk.com datasheet PDFs (403)
- ganternorm.com (403)
- engineersedge.com (403 on direct fetch; WebFetch got text but table unaligned)
- woodworkingnetwork.com (403)
- lang-technik.de guessed URL 404 (product pages under /en/products/ worked)
- Scribd copies of DIN 650/6315 not used (login-gated)
- DIN 650, DIN 6314, DIN 6315, ISO 299 originals: paywalled
