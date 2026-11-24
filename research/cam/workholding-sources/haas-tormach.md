# Workholding specs: Haas and Tormach (public sources only)

All values were accessed on 2026-11-23. "mm" is the vendor's own metric figure when one is
printed, marked (pub). Otherwise it is inch x 25.4, marked (conv), and no rounding was invented
beyond that arithmetic. UNKNOWN means no open source gave the figure. Search-engine snippets were
used only to find pages. Every value below was read from the page or PDF itself, either from raw
HTML/PDF text (curl + pdftotext, with drawings rendered and checked by eye) or, for tormach.com
(which refuses curl), through the WebFetch tool's page extraction.

Source key:
- HT-xxxx = https://www.haastooling.com/p/xxxx (Haas Automation's own tooling store; the seller is "Haas Automation Inc.")
- UK-… = www.haas.co.uk pages and PDFs. The PDFs are "Haas Technical Publications © Haas Automation" layout drawings.
- PROD = ecomm.productivity.com, a Haas Factory Outlet distributor that rehosts the Haas layout drawing.
- TM = tormach.com, KB = knowledgebase.tormach.com

---

## HAAS

### H1. 6" Single-Station CrossOver Fixed-Jaw Vise, 9" opening, flange style ("Haas Workholding by Kurt"), PN 05-0404
The page says: "the same high-quality DX6 Kurt vise … designed and branded specifically for Haas".

| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Jaw width [JW] | 6.0 in | 150.0 (pub) | https://www.haastooling.com/p/05-0404 | 2026-11-23 | the page prints 150.0 mm, not 152.4 |
| Max opening [VO] | 9.0 in | 225.0 (pub) | same | 2026-11-23 | |
| Inner jaw range [J1] | 0.0-9.08 in | 0.0-230.6 (pub) | same | 2026-11-23 | |
| Outer jaw range [J3] | 7.76-16.84 in | 197.1-427.7 (pub) | same | 2026-11-23 | |
| Jaw height [JH] | 1.735 in | 44.07 (pub) | same | 2026-11-23 | |
| Jaw depth/thickness | UNKNOWN | | same | 2026-11-23 | not on page (the matching soft jaw, H3, is 1.5 in deep) |
| Body length [BL] | 16.81 in | 426.97 (pub) | same | 2026-11-23 | |
| Overall length [OAL] | 18.258 in | 463.75 (pub) | same | 2026-11-23 | |
| Body height [BH] | 2.875 in | 73.03 (pub) | same | 2026-11-23 | |
| Overall height [OAH] | 4.36 in | 110.74 (pub) | same | 2026-11-23 | |
| Overall/body width [OAW] | 7.391 in | 187.73 (pub) | same | 2026-11-23 | |
| Base thickness [BT] | 0.82 in | 20.83 (pub) | same | 2026-11-23 | |
| Height of jaw screw hole [H] | 0.94 in | 23.88 (pub) | same | 2026-11-23 | |
| Jaw screw spacing [S] | 3.875 in | 98.43 (pub) | same | 2026-11-23 | |
| [a] / [b] dimensions | 0.656 in / 5.972 in | 16.66 / 151.69 (pub) | same | 2026-11-23 | labelled only on the line drawing, so what they measure is unknown |
| Jaw screw thread [TP] | 1/2"-13 | – | same | 2026-11-23 | |
| Wrench | 0.75 in (3/4") | – | same | 2026-11-23 | |
| Weight | 78.0 lb | 35.0 kg (pub) | same | 2026-11-23 | |

### H2. Other Haas-branded vises found (all from haastooling.com, Haas Workholding by Kurt unless noted)
| product / field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| **6" Double-Station Short TriLock CarveSmart, PN 05-0958**: jaw width | 6.0 in | 152.4 (pub) | https://www.haastooling.com/p/05-0958 | 2026-11-23 | |
| opening [VO] / J1 | 3.8 in / 0-3.8 in | 96.5 (pub) | same | 2026-11-23 | |
| jaw height | 2.0 in | 50.8 (pub) | same | 2026-11-23 | |
| BL / OAL | 18.7 / 19.0 in | 475.0 / 482.6 (pub) | same | 2026-11-23 | |
| BH / OAH | 3.1 / 5.126 in | 78.7 / 130.2 (pub) | same | 2026-11-23 | |
| OAW | 6.0 in | 152.4 (pub) | same | 2026-11-23 | |
| base thickness / screw spacing | 0.625 / 3.875 in | 15.9 / 98.4 (pub) | same | 2026-11-23 | thread 1/2"-13; 90.0 lb |
| **100 mm Double-Station HD CarvLock, PN 05-0211**: jaw width | 4.0 in | 100.0 (pub) | https://www.haastooling.com/p/05-0211 | 2026-11-23 | metric (ISO) vise |
| opening | 3.0 in | 75.0 (pub) | same | 2026-11-23 | J1 0.0-2.98 in (0.0-75.7 mm); J3 9.25-16.38 in (235.0-416.1 mm) |
| jaw height | 1.235 in | 31.37 (pub) | same | 2026-11-23 | |
| BL / OAL | 16.375 / 17.76 in | 415.92 / 451.1 (pub) | same | 2026-11-23 | |
| BH / OAH | 2.375 / 3.61 in | 60.32 / 91.69 (pub) | same | 2026-11-23 | |
| OAW | 4.0 in | 101.6 (pub) | same | 2026-11-23 | BT 0.7 in (17.78); S 2.5 in (63.5); M10x1.5; 34 lb |
| **5" (130 mm) Self-Centering Vise, PN 05-0004** (not by Kurt; origin "Various"): jaw width | 4.92 in | 125.0 (pub) | https://www.haastooling.com/p/05-0004 | 2026-11-23 | |
| opening [VO] | 5.0 in | 130.0 (pub) | same | 2026-11-23 | J1 0.48-5.19 in (12.0-132.0); J3 1.65-7.72 in (42.0-196.0) |
| jaw height | 1.26 in | 32.0 (pub) | same | 2026-11-23 | |
| BL / OAL | 8.27 / 8.433 in | 210.0 / 214.2 (pub) | same | 2026-11-23 | |
| BH / OAH | 2.24 / 3.504 in | 57.0 / 89.0 (pub) | same | 2026-11-23 | |
| OAW | 4.961 in | 126.0 (pub) | same | 2026-11-23 | a, b 3.78 in (96.0); c 1.77 in (45.0); S 4.0 in (101.6); 4 zero-point connections; M6 / M10 threads; 28 lb |
| Haas "E-Vise 6.0" (HEV-06) | UNKNOWN | | https://www.haascnc.com/content/dam/haascnc/productivity/workholding/e-vise-6-0/HEV-06-9.7.2021.pdf | 2026-11-23 | HTTP 403, refused |

### H3. Haas vise jaws (soft and hard)
| product / field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| **Soft steel 6"x1-1/2" machinable jaws, PN 05-0026 (HMVJ-61.5S)**: width / height / depth | 6.0 / 2.0 / 1.5 in | 152.4 / 50.8 / 38.1 (pub) | https://www.haastooling.com/p/05-0026 | 2026-11-23 | for Haas 6" vises |
| screw spacing / screw-hole height | 3.88 / 0.94 in | 98.4 / 23.8 (pub) | same | 2026-11-23 | cbore Ø0.78 in (19.8); hole Ø0.52 in (13.1) |
| **Aluminum 4"x1-1/2", PN 05-0088**: W / H / D | 4.0 / 1.75 / 1.5 in | 101.6 / 44.5 / 38.1 (pub) | https://www.haastooling.com/p/05-0088 | 2026-11-23 | S 2.5 in (63.5); H 0.69 in (17.5); cbore 0.71 (18.0); hole 0.41 (10.5) |
| **Aluminum 5"x2-1/2" (self-centering vise), PN 05-0049**: W / H / D | 4.9 / 2.0 / 2.5 in | 125.0 / 50.8 / 64.0 (pub) | https://www.haastooling.com/p/05-0049 | 2026-11-23 | S 1.77 in (45.0); H 1.26 (32.0); cbore 0.55 (14.0); hole 0.35 (9.0) |
| **Aluminum 8"x1", PN 05-0236**: W / H / D | 8.0 / 2.2 / 1.0 in | 203.2 / 55.9 / 25.4 (pub) | https://www.haastooling.com/p/05-0236 | 2026-11-23 | S 4.75 in (120.7); H 1.22 (31.0); cbore 1.02 (26.0); hole 0.71 (18.0) |
| **Hard steel 6"x1" Grip-N-Rip, PN 05-0789**: W / H / D | 6.0 / 1.71 / 1.0 in | 150.0 / 43.43 / 25.4 (pub) | https://www.haastooling.com/p/05-0789 | 2026-11-23 | thread 10-32 |
| 6" aluminum soft jaw | UNKNOWN | | – | 2026-11-23 | no Haas page found |

### H4. Haas fixture/tooling plates
| product / field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| **UMC-500/750 rectangle grid steel subplate, PN 05-0581**: L x W | 15.748 x 8.465 in | 400.0 x 215.0 (pub) | https://www.haastooling.com/p/05-0581 | 2026-11-23 | mounts to UMC trunnion |
| thickness | 0.748 in | 19.0 (pub) | same | 2026-11-23 | |
| thread | M12x1.75 | – | same | 2026-11-23 | |
| [a] [b] [c] [d] [e] | 5.874 / 4.0 / 0.984 / 5.5 / 1.969 in | 149.2 / 101.6 / 25.0 / 139.7 / 50.0 (pub) | same | 2026-11-23 | the labels are only on the drawing image. [e] = 50.0 mm is probably the grid pitch, but the page does not say so |
| parallelism | 0.002 in | 0.051 (pub) | same | 2026-11-23 | 26.8 lb |
| **UMC-750 round grid steel subplate, PN 08-1884**: Ø | 19.7 in | 500.4 (pub) | https://www.haastooling.com/p/08-1884 | 2026-11-23 | |
| thickness | 0.748 in | 19.0 (pub) | same | 2026-11-23 | M12x1.75; [a] 9.92 in (252.0), [b] 1.969 in (50.0) unlabelled |
| subplate hardware set, PN 06-0556 | T-nuts for 16 mm slots M12x1.75; SHCS M12x1.75x25/x20; SHCS 1/2-13x3/4" | – | https://www.haastooling.com/p/06-0556 | 2026-11-23 | "for all Haas tooling fixture subplates" |
| haascnc.com UMC fixture plate page | UNKNOWN | | https://haascnc.com/productivity/tooling-and-fixturing/fixture-plate-umc.html | 2026-11-23 | HTTP 403, refused |
| VF-series grid plates / tooling-plate table | UNKNOWN | | – | 2026-11-23 | none found on an open page |

### H5. HRT160 rotary table
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Platter Ø | 6.3 in | 160 (pub) | https://www.haas.co.uk/rotaries/hrt160/ ; https://www.haas.co.uk/wp-content/uploads/2019/10/HRT160_Dimensions.pdf | 2026-11-23 | the PDF is titled "HRT160/SS Layout Drawing", Rev A, Mar 2016 |
| Centre height | 5.000 ± 0.001 in | 127.00 ± 0.03 (pub) | both | 2026-11-23 | base to CL |
| Through bore | 1.5000 (+0.0005/-0) in | 38.100 (+0.013/-0) (pub) | both | 2026-11-23 | |
| T-slots | 6 @ 60° | – | both | 2026-11-23 | |
| T-slot widths (drawing) | slot .562 in TYP; undercut .93-.97 in; depths .36-.39 in and .38 in | 14.27; 23.6-24.6; 9.1-9.9; 9.7 (pub) | PDF | 2026-11-23 | read from the section view |
| Platter bolt circles | Ø3.95 in BC; Ø2.80 in BC | 100.3; 71.1 (pub) | PDF | 2026-11-23 | |
| Overall: length / depth / height | 17.35 / 6.0 / 10.40 in | 441 / 152 / 264 (pub) | PDF | 2026-11-23 | |
| Locating keyway | 0.625 +0.001/-0 in | 15.88 (pub) | PDF | 2026-11-23 | side mounting: 4x 1/2-13 x 0.63 in deep; 2x Ø0.500 in pins |

### H6. HRT210 rotary table
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Platter Ø | 8.3 in | 210 (pub) | https://www.haas.co.uk/rotaries/hrt210/ ; https://www.haas.co.uk/wp-content/uploads/2019/10/HRT210_Dimensions.pdf | 2026-11-23 | drawing Rev A, Mar 2016 |
| Centre height | 6.000 ± 0.001 in | 152.40 ± 0.03 (pub) | both | 2026-11-23 | |
| Through bore | 2.0000 (+0.0005/-0) in | 50.800 (+0.013/-0) (pub) | both | 2026-11-23 | |
| T-slots | 6 @ 60° | – | both | 2026-11-23 | the PDF slot detail repeats .562 in / .93-.97 in / .36-.39 in |
| Bolt circles | Ø4.73 in; Ø3.25 in | 120.1; 82.6 (pub) | PDF | 2026-11-23 | |
| Overall L / depth / H | 18.5 / 7.0 / 12.40 in | 470 / 178 / 315 (pub) | PDF | 2026-11-23 | keyway 0.625 in (15.88) |

### H7. TRT160 trunnion
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Platter Ø | 6.3 in (UK page); Ø6.30 in (drawing) | 160 (pub) | https://www.haas.co.uk/rotaries/trt160/ ; https://ecomm.productivity.com/customer/docs/skudocs/TRT160%20Layout%20Drawing.pdf | 2026-11-23 | the drawing is Haas Rev 1, Oct 31 2019, rehosted by a Haas distributor |
| A-axis centre height | 6.900 in | 175.26 (UK) / 175.3 (drawing) (pub) | both | 2026-11-23 | |
| B-axis platter height | 6.15 in | 156.2 (pub) | drawing | 2026-11-23 | |
| Through bore | Ø1.50 in | 38.1 (pub) | drawing p2 (TR160 platter) | 2026-11-23 | |
| T-slots | "6@ 60°" (UK page) | – | UK page | 2026-11-23 | CONFLICT: the 2019 drawing shows a bolt-hole platter (12x M10x1.5, 6x M6x1.0) with no T-slots |
| Max part swing / platter load | 10.5 in; 55 lb | 267 (pub); 24.9 kg | UK page | 2026-11-23 | |
| Footprint | 21.07 x 12.34 in; height 11.69 in | 535.3 x 313.3; 296.9 (pub) | drawing | 2026-11-23 | |

### H8. TRT210 trunnion
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Platter Ø | 8.3 in (UK, T-slot platter Ø8.27 in); standard hole platter Ø9.06 in | 210 / 230.0 (pub) | https://www.haas.co.uk/rotaries/trt210/ ; https://www.haas.co.uk/wp-content/uploads/2019/10/trt210_04-2021.pdf | 2026-11-23 | drawing Rev 5, Apr 8 2021. Its standard platter is Ø230 holes-only; T-slot, A6 and zero-point platters are optional |
| A-axis centre height | 9.000 ± 0.001 in (UK page) vs 7.65 in "A-Axis Centerline" (2021 drawing) | 228.60 / 194.2 (pub) | both | 2026-11-23 | CONFLICT, probably different generations. Use the drawing for the 2021 unit |
| B-axis platter height | 7.03 in | 178.5 (pub) | drawing | 2026-11-23 | |
| Through bore | Ø2.00 in | 50.8 (pub) | drawing | 2026-11-23 | |
| T-slot platter: slots | 6 @ 60°; slot .56 in, undercut .97 in, depth .39 in / .39 in; platter thickness 1.75 in | 14.3; 24.6; 9.9; 44.4 (pub) | drawing p4 | 2026-11-23 | |
| Max part swing / load | Ø14.5 in (drawing) / 12 in (UK page); 100 lb | 368.3 / 305 (pub); 45.4 kg | both | 2026-11-23 | CONFLICT on swing |

### H9. HA5C 5C indexer
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Centre height | 4.000 ± 0.001 in | 101.60 ± 0.03 (pub) | https://www.haas.co.uk/rotaries/ha5c/ ; https://www.haas.co.uk/wp-content/uploads/2019/10/HA5C_Dimensions.pdf | 2026-11-23 | drawing Rev B, Sep 17 2013 |
| Collet capacity | std 5C 1/64 - 1 1/16 in | 0.4 - 27 (pub) | UK page | 2026-11-23 | |
| Spindle nose thread | 2-3/16-10 NS-2A | – | both | 2026-11-23 | |
| Overall length / height / width | 15.32 / 7.80 (8.3 incl. handle) / 7.93 in | 389.1 / 198.1 (211) / 201.4 (pub) | PDF | 2026-11-23 | head body 7.00 in (177.8) |
| Locating pins | Ø.625 in, 3.0000 in apart | 15.88; 76.200 (pub) | PDF | 2026-11-23 | |

### H10. Haas ST-10 / ST-20 / ST-30 standard chucks
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| ST-10 chuck size | 6.5 in | 165 (pub) | https://www.haas.co.uk/lathes/st-10/ | 2026-11-23 | A2-5 nose; max 6000 rpm (spindle); spindle bore 2.31 in (58.7) |
| ST-10 bar capacity | 1.75 in | 44 (pub) | same | 2026-11-23 | |
| ST-20 chuck size | 8.3 in | 210 (pub) | https://www.haas.co.uk/lathes/st-20/ | 2026-11-23 | A2-6; max 4000 rpm (spindle); bore 3.50 in (88.9) |
| ST-20 bar capacity | 2.5 in | 63.5 (pub) | same | 2026-11-23 | |
| ST-30 chuck size | 10.0 in | 254 (pub) | https://www.haas.co.uk/lathes/st-30/ | 2026-11-23 | A2-6; max 3400 rpm (spindle); bore 3.50 in (88.9) |
| ST-30 bar capacity | 3.0 in | 76 (pub) | same | 2026-11-23 | |
| Chuck type, jaw count, chuck through-hole, chuck max rpm, brand | UNKNOWN | | https://www.haascnc.com/productivity/workholding/chuck-6-5-a2-5.html (also chuck-8-3-a2-6, chuck-10-a2-6) | 2026-11-23 | haascnc.com refused (403). The UK pages give size only |
| (related) 10" manual 3-jaw scroll chuck A2-6, PN 05-0808 | Ø10.0 in; ID 3.0 in; 2500 rpm; OD clamp 0.2-10.2 in; ID clamp 2.8-9.8 in; 46000 N | 254.0; 76.0 (pub) | https://www.haastooling.com/p/05-0808 | 2026-11-23 | for TL-1/TL-2 toolroom lathes, NOT the ST standard chuck |

### H11. Haas lathe collet chucks (5C nose option and quick-change)
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| 5C collet nose / 5C collet chuck A2-5 (haascnc "5c-collet-a2-5") | UNKNOWN | | https://www.haascnc.com/productivity/workholding/5c-collet-a2-5.html | 2026-11-23 | refused (403) |
| **HQCB-42 pull-back, A2-5, PN 04-0299** (ST-10 family): capacity | 0.157-1.65 in | 4.0-42.0 (pub) | https://www.haastooling.com/p/04-0299 | 2026-11-23 | HRG-42 collets, not 5C |
| bore / nose Ø / nose length / OAL | 1.169 / 4.015 / 0.689 / 3.602 in | 29.7 / 102.0 / 17.5 / 91.5 (pub) | same | 2026-11-23 | body Ø 125.0; flange Ø 140.0; 7000 rpm; draw nut M55x2 |
| **HQCBD-52 dead-length, A2-5, PN 05-0331**: capacity | 0.157-2.05 in | 4.0-52.0 (pub) | https://www.haastooling.com/p/05-0331 | 2026-11-23 | HRG-52 |
| bore / nose Ø / OAL | 2.09 / 4.57 / 4.291 in | 53.0 / 116.0 / 109.0 (pub) | same | 2026-11-23 | nose length printed as "39.8 in \ 1.57 mm" (garbled on the source page), so UNKNOWN; 7000 rpm |
| **HQCB-65 pull-back, A2-6, PN 05-0332** (ST-15/20): capacity | 0.157-2.65 in | 4.0-65.0 (pub) | https://www.haastooling.com/p/05-0332 | 2026-11-23 | HRG-65 (2.65 in vs 65.0 mm as printed) |
| bore / nose Ø / nose length / OAL | 2.6 / 4.72 / 0.93 / 4.37 in | 66.0 / 120.0 / 23.5 / 111.0 (pub) | same | 2026-11-23 | 5500 rpm; M75x2 |

---

## TORMACH

### T1. 4 in. CNC Vise, PN 35285
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Jaw width | 4 in. (10.2 cm) | 102 (pub) | https://tormach.com/4in-cnc-vise-35285.html | 2026-11-23 | |
| Max inside opening (std jaws) | 4 in. (10.2 cm) | 102 (pub) | same | 2026-11-23 | |
| Max outside opening | 11 in. (27.9 cm) | 279 (pub) | same | 2026-11-23 | |
| Jaw depth | 1.25 in. (3.2 cm) | 32 (pub) | same | 2026-11-23 | jaw height is not given separately |
| Overall length | 11 in. (27.9 cm) | 279 (pub) | same | 2026-11-23 | |
| Bed height | 2.5 in. (6.4 cm) | 64 (pub) | same | 2026-11-23 | |
| Overall height / width | UNKNOWN | | same | 2026-11-23 | shipping weight 44.1 lb |

### T2. 5 in. (127 mm) CNC Vise, PN 31759
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Jaw width | 5 in. (12.7 cm) | 127 (pub) | https://tormach.com/5-in-cnc-vise-31759.html | 2026-11-23 | |
| Max inside opening | 4 in. (10.2 cm) | 102 (pub) | same | 2026-11-23 | as published |
| Max outside opening | 12 in. (30.5 cm) | 305 (pub) | same | 2026-11-23 | |
| Jaw depth | 1.25 in. (3.2 cm) | 32 (pub) | same | 2026-11-23 | |
| Overall length | 12 in. (30.5 cm) | 305 (pub) | same | 2026-11-23 | |
| Bed height | 2.5 in. (6.4 cm) | 64 (pub) | same | 2026-11-23 | |
| Overall height / width | UNKNOWN | | same | 2026-11-23 | 44.1 lb shipping |

### T3. 5 in. Machinist Vise - Precision Angle Lock, PN 38935 (the "machinist vise" sold for PCNC mills)
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Jaw width | 5 in. (12.7 cm) | 127 (pub) | https://tormach.com/5-in-machinist-vise-38935.html | 2026-11-23 | |
| Max inside opening | 5 in. (12.7 cm) | 127 (pub) | same | 2026-11-23 | |
| Max outside opening | 12 in. (30.5 cm) | 305 (pub) | same | 2026-11-23 | |
| Jaw depth | 1.5 in. (3.8 cm) | 38 (pub) | same | 2026-11-23 | |
| Overall length | 12.5 in. (31.8 cm) | 318 (pub) | same | 2026-11-23 | |
| Bed height | 2.5 in. (6.4 cm) | 64 (pub) | same | 2026-11-23 | |
| Overall height / width | UNKNOWN | | same | 2026-11-23 | 47 lb shipping |

### T4. 6 in. CNC Vise - Precision Angle Lock, PN 51739
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Jaw opening | 6 in. | 152.4 (conv) | https://tormach.com/6-in-cnc-vise-51739.html | 2026-11-23 | |
| Jaw bolt pattern | 3.875 in. x 0.9375 in. | 98.425 x 23.8125 (conv) | same | 2026-11-23 | |
| Workstop hole / base holes | 7/16-14 in fixed jaw; M8 holes on bottom | – | same | 2026-11-23 | |
| Jaw width, jaw height, length, height, width | UNKNOWN | | same | 2026-11-23 | not on page; 68.4 lb shipping |
| Clamp kit for 6 in. vise, PN 52208 | 4x M12x50 screws, 4x T-nut M12, 4x MC-6-38 clamps (18 mm hole) | – | https://tormach.com/clamp-kit-for-6-in-vise-52208.html | 2026-11-23 | |

### T5. Tormach soft / hard jaws
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Std 4 in. aluminum soft jaw set, PN 54343 | 4.2 in. x 2.0 in. x 0.84 in. | 106.68 x 50.8 x 21.336 (conv) | https://tormach.com/standard-4-in-aluminum-soft-jaw-set-pair-54343.html | 2026-11-23 | which value is W, H or D is not labelled; hole spacing UNKNOWN |
| Std 5 in. aluminum soft jaw set, PN 54291 | 5.2 in. x 2 in. x 0.85 in.; bolt centres 3.125 in.; screws 3/8 in. or M10 | 132.08 x 50.8 x 21.59; 79.375 (conv) | https://tormach.com/5-in-aluminum-soft-jaw-set-54291.html | 2026-11-23 | |
| Std 6 in. aluminum soft jaw set, PN 53812 | dimensions UNKNOWN | | https://tormach.com/6-in-soft-jaw-set-2-pack-53812.html | 2026-11-23 | fits PN 51739; 2 jaws |
| Hardened 5 in. steel jaws 31878; 5 in. steel step jaws 31880 | not fetched / UNKNOWN | | https://tormach.com/hardened-5in-steel-jaw-set-31878.html ; https://tormach.com/5-in-steel-step-jaw-set-31880.html | 2026-11-23 | URLs taken from the category page |

### T6. Tormach fixture plates
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| SMW Fixture Tooling Plate Bundle for 1100 mills: plate | 34 in. x 11.95 in. x 0.95 in., anodized 7075 | 863.6 x 303.53 x 24.13 (conv) | https://tormach.com/smw-1100-tool-plate-bundle.html | 2026-11-23 | PCNC 1100/1100M/1100MX; hole grid and thread UNKNOWN |
| SMW bundle for 770 mills: plate | 26 in. x 11.95 in. x 0.95 in., 7075 | 660.4 x 303.53 x 24.13 (conv) | https://tormach.com/smw-770-tool-plate-bundle.html | 2026-11-23 | grid and thread UNKNOWN; 5/8 in. hardware kit |
| SMW bundle for PCNC 440: plate | 18 in. x 8.95 in., 7075 | 457.2 x 227.33 (conv) | https://tormach.com/smw-pcnc-440-tool-plate-bundle.html | 2026-11-23 | thickness, grid and thread UNKNOWN; 3/8 in. hardware kit |
| SMW steel plates: dowel holes | ".5005 ± .0002" for 1/2 in. dowel" | 12.713 ± 0.005 (conv) | https://tormach.com/articles/expanding-the-saunders-machine-works-fixture-plate-line | 2026-11-23 | article also names a "standard ½"-13 hole pattern" (for the Pierson pallet), no pitch |
| xsTECH aluminum plate, PN 53638 | 12 x 5.95 x 0.575 in., 6061; holes 0.75 in. on centre; all tapped 1/4-20 | 304.8 x 151.13 x 14.605; pitch 19.05 (conv) | https://tormach.com/tormach-xstech-aluminum-fixture-tooling-plate-53638.html | 2026-11-23 | for the xsTECH router |
| Aluminum fixture plate, PN 32836 | 18.0 x 8.0 in. x 10 mm; 1/2" locating slots; no holes | 457.2 x 203.2 (conv) x 10 (pub) | https://tormach.com/aluminum-fixture-plate-32836.html | 2026-11-23 | PCNC 770 ("7700" typo on page) |
| Aluminum fixture plate for 1100, PN 32624 (20 x 9.5 in. x 10 mm) | UNKNOWN | | https://tormach.com/aluminum-fixture-plate-32624.html | 2026-11-23 | the URL now redirects to the mill-workholding category, so no product page exists. The size came only from a search snippet, so it is not used as a value |

### T7. Tormach clamping kits / step blocks / toe clamps
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Strap clamp kit 5/8 in. T-slots (58 pcs), PN 32580: T-slot | 5/8 in. (15.9 mm) | 15.9 (pub) | https://tormach.com/clamp-kit-t-slots-58-pcs-32580.html | 2026-11-23 | 1100/770 family |
| studs | 1/2-13, qty 24 | – | same | 2026-11-23 | stud lengths UNKNOWN |
| step blocks | 6 pairs | – | same | 2026-11-23 | step-block sizes and steps UNKNOWN |
| other | 6 T-nuts, 6 flange nuts, 4 coupling nuts, 6 "end hold downs" | – | same | 2026-11-23 | strap clamp lengths UNKNOWN |
| Strap clamp kit 3/8 in. T-slots (58 pcs), PN 35509 | T-nuts for 3/8 in. slots; studs 5/16-18 | 9.525 (conv) | https://tormach.com/clamp-kit-t-slots-58pcs-35509.html | 2026-11-23 | quantities and sizes UNKNOWN |
| Clamp kits for 4 in. / 5 in. CNC vise (37925 / 37924) | not fetched / UNKNOWN | | https://tormach.com/clamp-kit-4-inch-cnc-vise-37925.html ; https://tormach.com/clamp-kit-5in-cnc-vise-37924.html | 2026-11-23 | |
| Toe clamps | UNKNOWN | | – | 2026-11-23 | no Tormach toe-clamp product found |

### T8. 8L lathe workholding
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Spindle nose | 5C | – | https://tormach.com/media/asset/8/l/8l-15l-rapidturn_pkgcomparison_0823.pdf | 2026-11-23 | comparison doc 08/23 |
| Through-spindle bore | 1 in. (25.4 mm); KB: 1" (25 mm) | 25.4 (pub) | same ; https://knowledgebase.tormach.com/8l/machine-specifications | 2026-11-23 | |
| Swing over bed / carriage | 8 in. / 4 in. | 200 / 100 (pub) | both | 2026-11-23 | |
| Packaged workholding | "3-Jaw Reversible 6 in. Lathe Chuck (D1-4 Mount)", "5C Round Collet Set (18 Pieces)", "ER20 Collet Chuck and Collet Set (15 Pieces)" | – | comparison PDF | 2026-11-23 | the package matrix checkmarks were lost in text extraction, so which package includes which item is UNKNOWN |
| 4 in. precision 3-jaw chuck, 5C mount, PN 32855 | Ø4 in.; thru hole 0.875"; ID range 1.18-3.54"; OD range 0.08-3.15" | 101.6; 22.225; 29.97-89.92; 2.03-80.01 (conv) | https://tormach.com/4in-precision-3-jaw-chuck-5c-mount-32855.html | 2026-11-23 | a search snippet said 1 in. through-hole; the page says 0.875" |
| 4 in. precision 4-jaw chuck, 5C mount, PN 50841 | Ø4 in.; thru hole 0.94"; ID 0.9-3.5 in.; OD 0.062-3.0 in. | 101.6; 23.876 (conv) | https://tormach.com/4in-precision-4-jaw-chuck-5c-mount-50841.html | 2026-11-23 | the page's own mm figures (ID "6-45 mm", OD "4-100 mm") contradict its inch figures, so the conv values are used |
| 5C collets | 5C round collet sets 18 pcs (34728) and 35 pcs (34727) | – | https://tormach.com/tooling/workholding/lathe-workholding.html | 2026-11-23 | ranges UNKNOWN (not fetched) |
| ER20 collet chuck, PN 34058 | UNKNOWN | | https://tormach.com/er20-collet-chuck-34058.html | 2026-11-23 | the URL redirects to the collet category (product page gone). Snippet values (3/4 in. shank, M25x1.5) are not used |

### T9. 15L Slant-PRO workholding
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Spindle nose | D1-4 camlock mount, or 5C insert | – | https://tormach.com/media/asset/8/l/8l-15l-rapidturn_pkgcomparison_0823.pdf | 2026-11-23 | |
| Through bore | 1.53 in. with 5C insert removed | 39 (pub) | same | 2026-11-23 | |
| Swing over bed / carriage | 15 in. / 6.2 in. | 381 / 158 (pub) | same | 2026-11-23 | spindle 250-3500 rpm high range |
| 3-jaw reversible 6 in. chuck, D1-4, PN 33156 | Ø6.25 in.; thru hole 1.6 in.; max 2500 rpm | 158.75; 40.64 (conv) | https://tormach.com/3-jaw-reversible-6in-lathe-chuck-d1-type-direct-mount-33156.html | 2026-11-23 | (3) 16 mm cam-lock studs M10x1; 0.003 in TIR; gripping range UNKNOWN; flange not included |
| 4-jaw independent 6 in. chuck, D1-4, PN 33336 | Ø6 in.; thru hole 1.75 in.; max 2000 rpm | 152.4; 44.45 (conv) | https://tormach.com/4-jaw-independent-reversible-6in-lathe-chuck-d1-type-direct-mount-33336.html | 2026-11-23 | gripping range UNKNOWN |
| Automatic collet closer for 15L, PN 38690 | standard 5C collets | – | https://tormach.com/automatic-collet-closer-15l-slantpro-38690.html | 2026-11-23 | capacity, nose dims and air pressure UNKNOWN |
| 15L knowledge-base spec page | UNKNOWN | | https://knowledgebase.tormach.com/15l-slant-pro/machine-specifications | 2026-11-23 | 404 (guessed URL) |

---

## Sites that refused or failed
- **haascnc.com**: HTTP 403 to both curl and WebFetch for every page tried. This includes product pages (/productivity/workholding/chuck-*.html, 5c-collet-a2-5.html, /machines/... HRT160, /productivity/tooling-and-fixturing/fixture-plate-umc.html), /content/dam PDFs (HEV-06 e-vise, pdp_feed/mld TRT210 and ST-10 drawings) and the HA5C operator manual PDF. I did not try to get around this. The Haas chuck type, jaws, chuck rpm, brand, the 5C collet nose and the haascnc.com vise pages therefore stayed UNKNOWN.
- **tormach.com**: HTTP 503 to plain curl. The pages were readable through the WebFetch tool, and the PDF came through WebFetch's saved binary. Several Tormach product URLs (32624 fixture plate, 34058 ER20 chuck) redirect to category pages because the product pages are gone.
- **knowledgebase.tormach.com/15l-slant-pro/machine-specifications**: 404.
- **haastooling.com**: pages readable, but category listings load by script and /api/ is disallowed in robots.txt. Products were found only through search results, so coverage is incomplete (for example, no ST standard hydraulic chuck page).
- **dksh.com** Haas ST-10 factsheet: 404.
