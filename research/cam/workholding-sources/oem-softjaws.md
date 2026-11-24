# Workholding data: machine-maker standard workholding, soft jaws, vises, collets, T-slots

All values were accessed 2026-11-23. Each value was read from the page or PDF named in its row. No value comes from a search snippet.
Inch-to-mm conversions use 25.4 exactly and are mine. Where the source prints mm, that figure is copied.
"Nominal" chuck sizes such as 8" are trade designations, so their mm figure is only an inch-to-mm conversion, not a measured chuck OD.
Lettered dimensions (A, B, C ...) are copied as printed. The meaning of a letter comes from the source's drawing. Where the drawing is an image I could not read, the notes say so and give the meaning only as a likely reading.

## Sites that refused or were not usable

| Site | What happened | Workaround |
|---|---|---|
| kurtworkholding.com (HTML pages) | HTTP 403 to WebFetch and curl | Used Kurt's own catalog PDF (2017 "Kurt Workholding US Catalog") hosted by distributors, a Kurt manual hosted by Travers, and Travers' D688 listing |
| mazak.com | curl got HTTP 406 | WebFetch worked. Mazak values in this file went through WebFetch's summarizer, so check them against the page |
| lyndexnikken.com PDF (R8/5C/3J/16C catalog) | HTTP 403 | Not used |
| dn-solutions.com catalogs | Download needs an e-mail address ("Catalog download authentication") | Not used. Only the public product-page spec tables were read |
| okuma.com/lb3000-exii-brochure | Download form | Used the public PDF at okuma.com/files/documents instead |
| miteebite.com | WebFetch got 403, but plain curl got 200 (no user-agent change) | Used the public catalog PDF |
| glacern.com/gsv-640 | Loads the home page. GSV-640 is not in Glacern's product menu (only GSV-440/550/690) | Gave the GSV-690, Glacern's 6" standard vise, as the closest product |
| issuu.com, scribd.com copies | Not used (JS/login walls, third-party copies) | - |

---

# A. Standard workholding on machine makers' machines

## DMG MORI CLX 350
Source: https://us.dmgmori.com/products/machines/turning/universal-turning/clx/clx-350

| Field | Published value | mm | Source URL | Accessed | Notes |
|---|---|---|---|---|---|
| Max. bar diameter | 65 mm / 2.6 in | 65 | above | 2026-11-23 | Page also says "ø2.56" bar capacity in standard" |
| Max. workpiece diameter | 320 mm / 12.6 in | 320 | above | 2026-11-23 | |
| Max. workpiece length | 530 mm / 20.9 in | 530 | above | 2026-11-23 | |
| Standard chuck size / brand | UNKNOWN | UNKNOWN | above | 2026-11-23 | Not on the public page |

## Mazak QUICK TURN 200 family
Source: https://www.mazak.com/us-en/products/quick-turn/ (read through WebFetch)

| Model | Chuck size | Max machining dia | Max machining length | Bar capacity | Notes |
|---|---|---|---|---|---|
| QUICK TURN 200 | 8" (nominal; 203.2) | Φ13.78" = 350.0 mm | 20.16" = 512.1 mm | UNKNOWN | Page lists no bar capacity or spindle bore |
| QUICK TURN 200M / 200MY | 8" | Φ14.75" = 374.7 mm | 21.28" = 540.5 mm | UNKNOWN | |
| QUICK TURN 200MS / 200MSY | 8" | Φ14.75" = 374.7 mm | 22.70" = 576.6 mm | UNKNOWN | |
| Chuck brand | UNKNOWN | | | | |

## Mazak QT-COMPACT 200
Source: https://www.mazak.com/uk-en/products/qt-compact/ (read through WebFetch)

| Model | Chuck size | Max machining dia | Max machining length | Bar capacity |
|---|---|---|---|---|
| QT-COMPACT 200M L / 200MY L | 8" | Φ340 mm | 532 mm / 1082 mm | UNKNOWN |
| QT-COMPACT 200MS L / 200MSY L | 8" | Φ340 mm | "-" (not given) | UNKNOWN |

## Okuma LB3000 EX II
Source: https://www.okuma.com/files/documents/space_turn_lb3000_ex2_e__18a__700_jan2021.pdf

| Field | Published value | mm | Source URL | Accessed | Notes |
|---|---|---|---|---|---|
| Standard spindle / chuck | JIS A2-6, "8-in chuck, 10-in chuck" | 203.2 / 254 nominal | above | 2026-11-23 | |
| Big-Bore spindle / chuck | JIS A2-8, 10-in or 12-in chuck | | above | 2026-11-23 | |
| Super Big-Bore spindle / chuck | JIS A2-11, 15-in chuck | | above | 2026-11-23 | |
| Spindle bore dia | ø80 {ø91} <ø110> mm (ø3.15 {ø3.58} <ø4.33> in) | 80 / 91 / 110 | above | 2026-11-23 | {} = Big-Bore, <> = Super Big-Bore |
| Standard chucking kit chuck | "Solid 8-in N-08A6"; optional kits "Hollow 8-in BB208A6", "Hollow 8-in B-208A6", "Hollow 10-in B-210A6"; sub-spindle "Hollow 6-in B206" | | above | 2026-11-23 | Chuck brand is not named. The model codes are as printed. The kit-to-model column mapping in the PDF table is hard to read from the extracted text |
| Kit jaws | Soft jaws A: 5, Soft jaws B: 3, Hard jaws: 1 (quantities in chucking kits) | | above | 2026-11-23 | |
| Max turning dia | ø410 (16.14 in) L spec; ø340 (13.39 in) M spec | 410 / 340 | above | 2026-11-23 | |
| Bar capacity | UNKNOWN | UNKNOWN | above | 2026-11-23 | Only the bore is published |

## Okuma GENOS L2000-e / L3000-e
Source: https://d30jruhmdicmo0.cloudfront.net/documents/GENOS_L2000_L3000-e-OAC-E_2b_200_Feb2022.pdf (Okuma CDN, Feb 2022)

| Field | Published value | mm | Accessed | Notes |
|---|---|---|---|---|
| Spindle nose / bore | JIS A2-6, bore ø62 (2.44 in); and JIS A2-8, bore ø80 (3.15 in) | 62 / 80 | 2026-11-23 | Two spec columns. Probably A2-6 = L2000-e and A2-8 = L3000-e, but the extracted column alignment is ambiguous |
| Max turning dia | ø230 / ø200 / ø340 mm (by column) | | 2026-11-23 | |
| Standard chuck size / brand | UNKNOWN | UNKNOWN | 2026-11-23 | Not found in the brochure text |
| Bar capacity | UNKNOWN | | 2026-11-23 | |

## DN Solutions Lynx 2100 and PUMA 2600
Sources: https://www.dn-solutions.com/us/product/turning-center/2-axis-horizontal/lynx-2100-a.do ; https://www.dn-solutions.com/global/product/turning-center/2-axis-horizontal/puma-2600.do

| Model | Chuck size | Max turning dia | Max turning length | Bar capacity | Notes |
|---|---|---|---|---|---|
| Lynx 2100A | 6 inch | 350 mm (14 in) | 330 mm (13 in) | UNKNOWN | The series page says "6"~10" High Performance Compact Turning Center". Lynx 2100 Y/SY: 300 mm / 510 mm |
| PUMA 2600 | 10 inch | 481 mm (19 in) | 790 mm (31 in) | UNKNOWN | The full catalog needs an e-mail address, so it was not used |
| Chuck brand | UNKNOWN | | | | |

## Makino a51nx (HMC)
Source: https://www.makino.com/en-us/machine-technology/machines/horizontal-4-axis/a51nx

| Field | Published value | mm | Accessed | Notes |
|---|---|---|---|---|
| Pallet | 400 x 400 mm / 15.7" x 15.7" | 400 x 400 | 2026-11-23 | |
| Maximum workpiece | ∅ 630 mm x 900 mm / ∅ 24.8" x 35.4" | ∅630 x 900 | 2026-11-23 | |
| Maximum payload | 400 kg / 880 lbs | - | 2026-11-23 | |
| Travels | X 560, Y 640, Z 640 mm | | 2026-11-23 | |
| ATC | 60 tools (40, 134, 219, 311 opt.) | | 2026-11-23 | |
| Standard tombstone / pallet top pattern | UNKNOWN | | 2026-11-23 | |

## Mazak HCN-5000 and HCN-5000 NEO
Sources: https://www.mazak.com/us-en/products/hcn ; https://mazak.com/us-en/detail/japan/horizontal/hn500ne (both read through WebFetch)

| Field | Published value | mm | Notes |
|---|---|---|---|
| HCN-5000 pallet size | 19.69" x 19.69" | 500 x 500 (conversion) | |
| HCN-5000 max workpiece | Φ31.5" x 39.37" | Φ800 x 1000 (conversion) | |
| HCN-5000 max load | UNKNOWN | | Not on the page |
| HCN-5000 NEO pallet | 20" x 20" | 508 (conversion of a rounded figure) | |
| HCN-5000 NEO max workpiece | Φ32" x 39" | Φ812.8 x 990.6 | |
| HCN-5000 NEO max load (evenly distributed) | 1102 lbs | (499.9 kg) | |
| Table indexing / pallet top | UNKNOWN | | |
| Other HCN on the same page | HCN-4000 15.75" sq, Φ24.8" x 35.43"; HCN-6000 19.69" sq, Φ35.43" x 39.37"; HCN-8800 31.5" sq, Φ57.09" x 57.09" | | |

## DN Solutions NHP 5000
Source: https://www.dn-solutions.com/us/product/machining-center/horizontal/nhp-5000.do (spec table in the page HTML)

| Field | Published value | mm | Notes |
|---|---|---|---|
| Pallet size | 500x500 mm / 20X20 inch | 500 x 500 | |
| Maximum workpiece weight | 500 kg / 1102 lb | | |
| Max workpiece dia x height | UNKNOWN | | Not in the public table |
| Travels X/Y/Z | 730 / 730 / 880 mm | | |
| Tool stations | 40 | | |
| Siblings (same table) | NHP 4000: 400 mm pallet, 400 kg. NHP 5000S: 500x600 mm, 500 kg. NHP 5500 2nd Gen: 500x500 mm, 800 kg | | |
| Standard tombstone | UNKNOWN | | |

## Okuma MB-5000H II
Source: https://www.okuma.com/files/documents/mb_5000h2_e__1b__200_jan2019.pdf

| Field | Published value | mm | Notes |
|---|---|---|---|
| Pallet size | 500 x 500 mm (19.69 x 19.69 in) | 500 x 500 | |
| Max load | 500 kg (1,100 lb) | | |
| Max workpiece dimensions | ø800 x 1,000 mm (31.5 x 39.37 in) | | |
| Indexing angle | 1 deg [0.001] | | [ ] = optional 0.001 deg table |
| Pallet top, standard | "Pallet top surface M16 tap" | | Standard specifications list. Options list: "T-slots, inch holes" |
| Spindle center to pallet top | 50 to 810 mm | | |
| Spindle nose to pallet center | 100 to 910 mm | | |

## Brother SPEEDIO S300X2 / S500X2 / S700X2
Source: https://machinetool.global.brother/en-eu/sx2/spec/index.aspx

| Field | Published value | mm | Notes |
|---|---|---|---|
| Table work area size | 800 x 400 (31.5 x 15.7 in); 600 x 400 (23.6 x 15.7 in); 600 x 400 | as printed | Column order as on the page. The other columns (S700X2 800x400; S500X2 and S300X2 600x400) are a reasonable reading but not confirmed |
| Max loading capacity (uniform) | 250 [300] kg (551 [661] lb) | | [ ] = option, per page footnote |
| Standard rotary table / jig | UNKNOWN | | No standard rotary or vise is listed. Brother sells the T-200A / T-200Ad rotary table, but its product page gave no spec text when fetched |

## Hurco / Fadal standard vises or rotary tables
| Field | Value | Notes |
|---|---|---|
| Standard vise / rotary | UNKNOWN | No maker-published standard-workholding data found. A Hurco catalog copy on Scribd lists rotary tables as optional; Scribd was not used as a source |

---

# B. Soft-jaw blanks sold per vise

## Kurt: 6" jaw plates for D688 / DX6 / 3600 ("Fits all Kurt 6" vises")
Source: Kurt Workholding US Catalog (Kurt PDF, created 2017-09-28), pages 66-68: https://cdn.dgisupply.ca/img/product-images/Asset_URL/KURT_DX6_Catalog.pdf (the same file is at https://www.productivity.com/wp-content/uploads/2020/07/Kurt-Workholding-Interactive.pdf)

Standard Individual Jaw Plates table. G = 5.969 is the length and H = 3.875 is the bolt spacing (matches the 3.875" jaw-plate thread spacing). B and C are most likely thickness and height, but the drawing is an image.

| Part # | Description | B (in) | C (in) | G (in) | H (in) | mm (B x C x G) | Weight |
|---|---|---|---|---|---|---|---|
| D60-7 SOFT | Std. Soft Jaw | 0.750 | 1.750 | 5.969 | 3.875 | 19.05 x 44.45 x 151.61 | 2 lb |
| D60-7 | Std. Hard Jaw | 0.750 | 1.750 | 5.969 | 3.875 | 19.05 x 44.45 x 151.61 | 2 lb |
| 60-234-1 | Aluminum ("carve your own pocket") | 0.980 | 1.880 | 5.969 | 3.875 | 24.89 x 47.75 x 151.61 | 1 lb |
| 60-235-1 | 4150 Semi-Hard ("carve your own pocket") | 1.125 | 1.880 | 5.969 | 3.875 | 28.58 x 47.75 x 151.61 | 3.5 lb |
| D40-7 SOFT | 4" Std. Soft Jaw | 0.562 | 1.250 | 3.969 | 2.500 | 14.27 x 31.75 x 100.81 | 1 lb |
| D80-7 SOFT | 8" Std. Soft Jaw | 1.105 | 2.200 | 7.969 | 4.750 | 28.07 x 55.88 x 202.41 | 5 lb |

| Part # | Published value | mm | Notes |
|---|---|---|---|
| XJAB6A-318 | "5.969" L x 1.500" W x 1.874" H", 6061-T6, 2.00 lb, "Fits all Kurt 6" vises", reversible | 151.61 x 38.10 x 47.60 | Machinable aluminum jaw plate |
| D60-315-P (Groove Lock, hard) | 1.735 x 0.7250 x 5.969 in | 44.07 x 18.42 x 151.61 | Standard 6" plate envelope |
| KSJ6 | 6" x 1.735" x 0.725" | 152.4 x 44.07 x 18.42 | |

DoveLock machinable jaw kits (Length / Width / Height as printed):
| Part # | L x W x H (in) | mm | Weight |
|---|---|---|---|
| DJ606AL-JAWKIT | 6.000 x 1.250 x 1.980 | 152.40 x 31.75 x 50.29 | 4 lb |
| DJ607AL-JAWKIT | 7.000 x 1.500 x 1.980 | 177.80 x 38.10 x 50.29 | 5 lb |
| DJ608AL-JAWKIT | 8.000 x 1.500 x 1.980 | 203.20 x 38.10 x 50.29 | 6 lb |
| DJ610AL-JAWKIT | 10.000 x 1.500 x 1.980 | 254.00 x 38.10 x 50.29 | 7 lb |
| DJ606S-JAWKIT (machinable steel) | 6.000 x 1.000 x 1.980 | 152.40 x 25.40 x 50.29 | 7 lb |
| DL406AL-JAWKIT | 6.000 x 1.275 x 1.414 | 152.40 x 32.39 x 35.92 | 3 lb |
| DJ6 Master Jaw Kit | 6.000 x 0.775 x 1.725 | 152.40 x 19.69 x 43.82 | 2 lb |

HD/HDL 6" machinable jaw kits (same catalog, p.38; mm and inch both printed):
| Part # | Description | A | B | H | Weight |
|---|---|---|---|---|---|
| HD6AL-JAWKIT | 1.73 High Aluminum | 43.891 mm (1.728) | 152.400 (6.000) | 349.250 (13.750) | 3 kg / 7 lb |
| HD6AL2.5-JAWKIT | 2.4 High Aluminum | 60.960 (2.400) | 152.400 (6.000) | 349.250 (13.750) | 9 kg / 19 lb |
| HD6C-JAWKIT | 1.73 High Iron | 43.891 (1.728) | 152.400 (6.000) | 349.250 (13.750) | 15 kg / 33 lb |
A and B are likely height and width, and H likely overall length; the drawing is an image. Other letters: C 127.000, D 82.550, E 84.120, F 24.160, G 13.260 (AL2.5: 30.330).

Soft jaws named specifically for the 3600 or D688: none beyond the "all Kurt 6" vises" plates above.

## Orange Vise (MariTool) machinable jaws
| Part # | Published value | mm | Source URL | Notes |
|---|---|---|---|---|
| OV-701-002 (6" machinable sliding jaw blocks) | "Base LxWxH: 4.63" x 6 x 2", 6061-T6, 6 lb; bored for 0.4993" dowel pins | 117.60 x 152.40 x 50.80; dowel 12.68 | https://www.maritool.com/p17501/6-Inch-6061-Aluminum-Machinable-Jaw-Blocks-6x4-5/8x2/product_info.html | For all OV6 generations |
| OV-701-012 (6" machinable center jaws) | "4" x 6" x 2" (LxWxH)", 6061, 5 lb | 101.60 x 152.40 x 50.80 | https://www.maritool.com/p17504/6-Inch-6061-Aluminum-Machinable-Center-Jaws-6x4x2/product_info.html | |
| OV-702-002 (dovetailed soft jaws, Al) | Height 2", Depth 1", Length 6", 6061-T6 | 50.80 / 25.40 / 152.40 | https://www.maritool.com/p20786/2%22-x-1%22-Dovetailed-Soft-Jaws---Aluminum/product_info.html | 3rd/4th-gen dovetail |
| 702-401 (TalonGrip-ready, 1018 steel) | 6" L x 2" H x 1" T | 152.40 x 50.80 x 25.40 | https://www.maritool.com/p20784/Jaws-for-Talon-Grip™-6x1.687x1-(Flush)/product_info.html | M5 tapped |
| OV-702-411 (TalonGrip-ready, flush) | 6" L x 1.687" H x 1" T | 152.40 x 42.85 x 25.40 | same | "flush with Orange Master Jaws" |

## Mitee-Bite TalonGrip vise jaws (1018 steel, not heat-treated)
Source: Mitee-Bite 2024 catalog p.20: https://www.miteebite.com/2024%20CATALOG-FINAL_8-24.pdf

A, B and C are as printed (inch, with mm in parentheses). A is the jaw length; B and C are most likely height and thickness (drawing is an image). Note: the "Vise" column in the extracted text looks shifted by one row against A.

| Part # | Vise (as printed) | A | B | C | D (hole spacing) | Grips / stop |
|---|---|---|---|---|---|---|
| 32044 | 4" (100mm) | 4.0 (100) | 1.48 (37.59) | 1.0 (25.4) | 2.5 (63.5) | 33050 / 33020 |
| 32066 | 4"/6" | 6.0 (150) | 1.73 (43.94) | 1.0 (25.4) | 2.5/3.88 (63.5/98.55) | 33050 / 33020 |
| 32068 | 6" | 8.0 (200) | 1.73 (43.94) | 1.0 (25.4) | 3.88 (98.55) | 33050 / 33020 |
| 32088 | 6"/8" | 8.0 (200) | 2.45 (62.23) | 1.25 (31.75) | 3.87/4.75 (98.3/120.65) | 33075 / 33030 |
| 33044 (no mounting holes) | - | 4.0 (100) | 1.48 (37.59) | 1.0 (25.4) | - | |
| 33066 (no mounting holes) | - | 6.0 (150) | 1.73 (43.94) | 1.0 (25.4) | - | |
| 33068 (no mounting holes) | - | 8.0 (200) | 1.73 (43.94) | 1.0 (25.4) | - | |
Grips 33050: 19.05 x 12.7 x 6.35 mm, M5, gripping height 1.5-1.9 mm. Mitee-Bite does not sell plain aluminum soft-jaw blanks in this catalog: UNKNOWN.

## LANG Makro-Grip contour jaws (aluminium, machinable)
| Item | Published value | mm | Source URL |
|---|---|---|---|
| 48419-77 (outside, 1 pair) | Jaw width 77 mm (3.03") - length 28 mm (1.1") - height 30 mm (1.18"); 0.28 kg | 77 x 28 x 30 | https://lang-technik.de/sites/default/files/630/pdf/48419-77_en_0.pdf |
| 48409-125 (inner side, 1 pair) | Jaw width 125 mm (4.92") - length 33 mm (1.3") - height 26 mm (1.02"); 0.63 kg | 125 x 33 x 26 | https://lang-technik.de/sites/default/files/626/pdf/48409-125_en_0.pdf |
| Other widths (46 mm etc.), item no. 45151 | UNKNOWN | | Not checked |

## 5th Axis machinable jaws
Source: https://5thaxis.com/wp-content/uploads/2022/09/2022-Summer-5th-Axis-Catalog.pdf (same entries in the 2024 catalog at https://suttontools-2022.s3.ap-southeast-2.amazonaws.com/Brochure+and+Catalogue/Brochures/5thAxis/5TH_AXIS_CATALOG_2024.pdf)

| Part # | Published value (mm) | Fits |
|---|---|---|
| VAJ75X (6061 Al) | 75 x 45 x 25 | V75100 / V75100X |
| VAJ52X (6061 Al) | 125.73 x 62.23 x 34.29 | V562X / V510X |
| VAJ52M (6061 Al) | 125 x 50 x 34 | V552M / V562M / V510M |
| VAJ610M (6061 Al) | 151 x 78 x 47 | V6105M |
| VSJ75X / VSJ52M / VSJ610M (1018 steel) | 75x45x25 / 125x50x34 / 151x78x47 | as above |
The catalog does not say which axis each dimension is.

## Glacern soft jaws (6")
Source: https://www.glacern.com/gsv_690
| Part # | Published | mm |
|---|---|---|
| GSJ-061020AL | "Soft Jaws - Al (6x1x2) Small" | 152.4 x 25.4 x 50.8 |
| GSJ-061520AL | "(6x1.5x2) Medium" | 152.4 x 38.1 x 50.8 |
| GSJ-062020AL | "(6x2x2) Large" | 152.4 x 50.8 x 50.8 |

---

# C. Vises

## Kurt D688
| Field | Published value | mm | Source URL | Notes |
|---|---|---|---|---|
| Jaw width | 6" | 152.4 | https://www.travers.com/product/kurt-d688-milling-machine-vise-61-421-170 | Distributor listing (Kurt HTML refused) |
| Max opening | 8.8" | 223.52 | same | Kurt catalog also says "Larger opening (9.0" versus 8.8")" when comparing the DX6 |
| Jaw depth (height) | 1.735" | 44.07 | same | |
| Length | 16.81" | 426.97 | same | |
| Bed height | 2.875" | 73.03 | same | |
| Vise height | 4.375" | 111.13 | same | |
| Body width | 8-3/4" | 222.25 | same | |
| Weight | 81 lbs | (36.7 kg) | same | |
| Clamp force | 7968 lb at 80 ft-lb | | https://docs.travers.com/assets/pdf/61-421-170_manual.pdf | Kurt D688 manual |
| Jaw plates | D60-7 plate, D60-315 grooved plate | | same manual, parts list | |

## Kurt DX6 (D688 replacement), for reference
Source: Kurt catalog p.12 (dgisupply URL above). Letters as printed; drawing is an image.
| Letter | Value (in) | mm | Likely meaning |
|---|---|---|---|
| A | 2.875 | 73.03 | Bed height (matches D688 2.875") |
| B | 6.000 | 152.40 | Jaw width |
| C | 16.810 | 426.97 | Overall length |
| H | 1.735 | 44.07 | Jaw height |
| J | 0.725 | 18.42 | Jaw plate thickness |
| K | 4.000 | 101.6 | |
| AA / BB / CC / DD | 9.080 / 12.117 / 13.805 / 16.842 | 230.63 / 307.77 / 350.65 / 427.79 | Jaw openings for the jaw positions |
| Weight | 65 lbs | | |
Text: "Large 9" opening".

## Kurt HDL6 (double-station, 6")
Source: Kurt catalog p.28 (dgisupply URL above). mm and inch both printed for HDLM6/HDL6. Letters as printed; drawing is an image.
| Letter | mm | in | Likely meaning |
|---|---|---|---|
| A | 79.375 | 3.125 | Height (±0.001" note) |
| B | 152.400 | 6.000 | Jaw width |
| C | 533.400 | 21.000 | Overall length |
| D | 258.877 | 10.192 | |
| E | 82.550 | 3.250 | |
| F | 539.750 | 21.250 | |
| G | 269.875 | 10.625 | |
| N | 43.891 | 1.728 | Jaw height (standard height jaw) |
| O | 152.400 | 6.000 | Body width |
Text: "Full 3 and 4 inch opening in each station with standard hard jaw system". Hard-jaw kit table (6" manual): B and C both 101.600 mm (4.000"), G 44.069 (1.735), F 18.415 (0.725).
Max opening per station: probably 4" for the 6" model (101.6 mm), but the text gives only "3 and 4 inch".

## Glacern GSV-640
| Field | Value | Notes |
|---|---|---|
| All fields | UNKNOWN | Glacern lists no GSV-640. The closest product is the GSV-690 (6" standard) below |

## Glacern GSV-690 (6" standard CNC vise)
Source: https://www.glacern.com/gsv_690
| Field | Published value | mm |
|---|---|---|
| Jaw width | 6" (model class) | 152.4 |
| Capacity, inner jaw plates | 8.95" | 227.33 |
| Capacity, outer jaw plates | 17.00" | 431.8 |
| Bed height | 2.8750" +/- 0.0005" | 73.03 |
| Bolt hole pattern | 3.875" x 0.9375" | 98.43 x 23.81 |
| Jaw plate threads | 1/2"-13 UNC, 3.875" spacing | 98.43 |
| Mounting | (4) 16 mm bushing points; RockLock 96 mm spacing | |
| Shipping weight | 81 lbs | |
| Jaw depth, overall length, height, body width | UNKNOWN | Drawing image only |

---

# D. Collet workholding

## Hardinge collet geometry
Source: Forkardt Hardinge Collet Blocks catalog 2290, p.17: https://forkardthardinge.com/wp-content/uploads/2290_Collet-Blocks-WEB.pdf (mm in parentheses as printed)

| Collet | A back bearing dia | B overall length | C thread | D head dia | Round cap. | Hex cap. | Square cap. |
|---|---|---|---|---|---|---|---|
| 5C | 1.250" (31.75) | 3.281" (83.34) | 1.238" x 20 RH | 1.470" (37.34) | 1-1/16" (26.99) | 29/32" (23.02) | 3/4" (19.05) |
| 16C | 1.889" (47.98) | 4.312" (109.52) | 1.870" x 1.75mm RH | 2.265" (57.53) | 1-5/8" (41.28) | 1-13/32" (35.72) | 1-9/64" (28.97) |
| 3J | 2.000" (50.80) | 3.750" (95.25) | 1.988" x 20 RH | 2.205" (56.00) | 1-3/4" (44.45) | 1-17/32" (38.89) | 1-1/4" (31.75) |
Internal stop threads: 5C 1.041" x 24 RH; 16C 1.687" x 20 TPI RH; 3J 1.79" x 24 TPI RH. Connector-bushing on-center spacing for hydraulic blocks: 5C 3.750", 16C 4.250", 3J 4.250".

## 5C collet blocks (square / hex)
| Product | Published value | mm | Source URL | Notes |
|---|---|---|---|---|
| TTC CB5-H (hex) | 1-3/4" x 1-3/4" x 2-3/4"; capacity 1/16" thru 1-1/8"; parallel .0005"; 1.34 lb | 44.45 x 44.45 x 69.85; cap. 1.59-28.58 | https://www.travers.com/product/ttc-cb5-h-collet-fixture-69-001-008 | TTC is Travers' own brand. Description also covers CB5-S (square), same size |
| Precise 3900-1620 (hex + square set) | Overall 1-3/4" x 1-3/4" x 3-1/4"; capacity 1/16" to 1-1/16" | 44.45 x 44.45 x 82.55; cap. 1.59-26.99 | https://www.penntoolco.com/precise-4-piece-hex-square-5c-collet-block-set-3900-1620/ | Distributor page |
| Hardinge powered 5C blocks | e.g. 7799-00-00-000095: capacity 1-1/16" (26.99) | | Hardinge 2290 PDF above | Hydraulic/pneumatic, not square/hex manual blocks |

## 5C emergency collets
Source: Forkardt Hardinge Spindle Tooling catalog 2348J, p.5: https://forkardthardinge.com/wp-content/uploads/2348J_SpindleTooling.pdf
| Field | Published value | mm | Notes |
|---|---|---|---|
| 5C-E1 (steel), part 10071210000000 | Pilot hole 1/16" | 1.59 | |
| 5C-E2 (steel), part 10071310000000 (3 slots) / 10071310000004 (4 slots) | Pilot 1/8" | 3.18 | |
| 5C-E0 | No pilot, 3 slots | | |
| 5C-EN1 / 5C-EN2 (extended nose) | Pilot 1/4"; nose length A = 1" / 1/2" | 6.35; 25.40 / 12.70 | |
| 5C Plug (solid) | A = 1-3/4" | 44.45 | |
| Max machinable diameter | UNKNOWN | UNKNOWN | Not published. The standard 5C round capacity, 1-1/16" (26.99), is the published figure for regular collets |
| 16C-E / 3J-E pilots | 1/4" (6.35) | | |

---

# E. Standards

## T-slots, DIN 650
Source: Mytri B.V. (maker of cast-iron and granite plates), article S-950: https://www.mytri.nl/en/products/t-slots-to-din-650
Mytri prints the letters only and does not say what each means. A likely reading: a = slot width, b = undercut (head) width, c = undercut height, H = total depth, e = chamfer. Tolerance on slot width H12; H7/H8 on request. All values are mm.

| a | for screws | b | c | H | e |
|---|---|---|---|---|---|
| 10 | M8 | 17.5 - 18 | 8 | 18 | 1.0 |
| 12 | M10 | 20.5 - 21 | 9 | 21 | 1.0 |
| 14 | M12, 1/2" | 23.5 - 24 | 10 | 24 | 1.0 |
| 16 | M14 | 26.5 - 27 | 11 | 27 | 1.0 |
| 18 | M16, 5/8" | 29.5 - 30 | 12 | 30 | 1.5 |
| 20 | M18 | 33.5 - 34 | 14 | 34 | 1.5 |
| 22 | M20, 3/4" | 37.5 - 38 | 16 | 38 | 1.5 |
| 24 | M22, 7/8" | 41.0 - 42 | 18 | 42 | 1.5 |
| 28 | M24, 1" | 47.0 - 48 | 20 | 48 | 1.5 |
| 32 | M27, 1 1/8" | 54 | 22 | 54 | 1.5 |
| 36 | M30, 1 1/4" | 60 | 25 | 61 | 2.0 |
| 42 | M36, 1 1/2" | 70 | 29 | 74 | 2.0 |
ISO 299: not separately checked: UNKNOWN.

## DIN 6315 B forked clamps
Source: Halder EH 23150 datasheet ("Published on: 23.10.2026"): https://halder.com/file-download/object_file_dsag_eh23150_en
b1 = nominal (slot) width, l1 = length, a = height (Hoffmann lists the 22 mm size as height 40). b2, b3 and b4 are from the drawing (image), and their meanings were not read. All values are mm.

| Art. No. | b1 | l1 | a | b2 | b3 | b4 | Screws | Weight |
|---|---|---|---|---|---|---|---|---|
| 23150.0007 | 6.6 | 60 | 12 | 19 | 6 | 3 | M6, 1/4 | 65 g |
| 23150.0009 | 9.0 | 80 | 15 | 25 | 8 | 4 | M8, 5/16 | 141 g |
| 23150.0011 | 11.0 | 100 | 20 | 31 | 10 | 5 | M10, 3/8 | 299 g |
| 23150.0014 | 14.0 | 125 | 25 | 38 | 12 | 6 | M12/M14, 1/2 | 578 g |
| 23150.0015 | 14.0 | 160 | 25 | 38 | 12 | 6 | M12/M14 | 715 g |
| 23150.0016 | 14.0 | 200 | 25 | 38 | 12 | 6 | M12/M14 | 905 g |
| 23150.0018 | 18.0 | 160 | 30 | 48 | 15 | 8 | M16/M18, 5/8 | 1.1 kg |
| 23150.0019 | 18.0 | 200 | 30 | 48 | 15 | 8 | M16/M18 | 1.3 kg |
| 23150.0020 | 18.0 | 250 | 40 | 48 | 15 | 10 | M16/M18 | 2.3 kg |
| 23150.0022 | 22.0 | 200 | 40 | 52 | 15 | 10 | M20/M22, 3/4 | 1.8 kg |
| 23150.0023 | 22.0 | 250 | 40 | 62 | 20 | 10 | M20/M22 | 3.0 kg |
| 23150.0024 | 22.0 | 315 | 40 | 62 | 20 | 10 | M20/M22 | 3.8 kg |
| 23150.0026 | 26.0 | 200 | 40 | 66 | 20 | 10 | M24, 1" | 2.4 kg |
| 23150.0027 | 26.0 | 250 | 40 | 66 | 20 | 10 | M24 | 3.0 kg |
| 23150.0028 | 26.0 | 315 | 40 | 66 | 20 | 10 | M24 | 3.8 kg |
| 23150.0030 | 26.0 | 500 | 50 | 66 | 20 | 10 | M24 | 7.6 kg |
| 23150.0034 | 33.0 | 250 | 50 | 74 | 20 | 12 | M30, 1 1/4 | 3.7 kg |
| 23150.0035 | 33.0 | 315 | 50 | 74 | 20 | 12 | M30 | 4.8 kg |
| 23150.0036 | 33.0 | 400 | 50 | 74 | 20 | 12 | M30 | 6.5 kg |
| 23150.0040 (1) | 40.0 | 400 | 60 | 100 | 30 | 12 | M36, 1 1/2 | 11.0 kg |
| 23150.0041 (1) | 40.0 | 600 | 60 | 100 | 30 | 12 | M36 | 19.0 kg |
(1) Halder: "DIN standards do not include these dimensions."
