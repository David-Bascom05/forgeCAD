# Workholding specs from router and hobby machine makers (published data only)

Accessed: 2026-11-23 (every row). Rules: values only from public maker pages/PDFs reached without login; search
snippets were never used as a value source; "UNKNOWN" = not found on a reachable maker source. mm column is an
exact conversion of the published inch value (1 in = 25.4 mm) unless the maker printed its own mm (then quoted as given).
Where a maker's own document is hosted by a third party, that is said in Notes.

## Sites that refused or could not be read

| Site | What happened |
|---|---|
| grizzly.com product pages (e.g. /products/...g1076, /products/...g1075) | HTTP 403 Forbidden to the fetch tool. Not bypassed. Grizzly's own manual PDFs on cdn0.grizzly.com were reachable (HTTP 200) and are used instead. |
| support.syil.com | Did not resolve from this session (outside the session's allowed network). Not read. |
| onefinitycnc.com QCW frame product pages | HTTP 404 (pages gone). T-track Oops clamp pages returned only the title (page body not rendered). |
| sherline.com blue./green. subdomains | blue.sherline.com returned HTTP 503. www.sherline.com pages worked. |
| downloadcenter.rolanddg.com MDX-50 / MDX-40A user manuals | Guessed URLs returned 404; no MDX vise spec found. |
| www.shopbottools.com/ShopBotDocs/files/MiniVacuumTable.pdf | Old URL returns "Multisite support not enabled"; current copy at shopbottools.com/wp-content/uploads/2024/01/ was used. |
| Carbide 3D shapeoko5pro-specs page | Reachable, but "T-Slot Spacing" label had no value in the page text. |

Note: Sherline pages carry a notice that Sherline suspended new orders effective October 19, 2026 (pending a sale).

---

## 1. Carbide 3D (Shapeoko, Nomad)

### 1a. Crush-It Essential Clamp Set
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| clamp width | "25mm wide" | 25 | https://shop.carbide3d.com/products/crush-it-essential-clamp | 2026-11-23 | fits fully within Hybrid Table slots |
| min height above table | "as low as 11mm" | 11 | same | 2026-11-23 | when seated in a Hybrid Table slot |
| jaw throw | 2 mm at 45 deg | 2 | same | 2026-11-23 | |
| fastener thread | "M6 hardware" | M6 | same | 2026-11-23 | 5 mm Allen key drives all |
| clamp length | UNKNOWN | | same | 2026-11-23 | not on page |
| material | glass-filled nylon, stainless hardware | | same | 2026-11-23 | |
| contents | 4 Crush-It clamps (serrated), 4 flat jaws, 4 stops, 4 Essential Clamps, 1 Nutz 'n Bolts kit | | same | 2026-11-23 | |

### 1b. Crush-It Nutz 'n Bolts
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| bolt thread | "M6 x 1.0mm pitch" | M6x1 | https://shop.carbide3d.com/products/crush-it-nutz-bolts | 2026-11-23 | compatible with Teez Nutz |
| bolt lengths | 16, 20, 25, 30, 35, 40 mm (6 each, 36 total) | same | same | 2026-11-23 | |
| T-nuts (Teez Nutz) | qty 12; dimensions UNKNOWN | | same | 2026-11-23 | |

### 1c. Hybrid Table (Shapeoko 4 / Pro / 5 Pro)
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| T-slot spacing, Shapeoko 5 Pro | "4.04", instead of 100mm" | 102.616 | https://carbide3d.com/blog/introducing-shapeoko-5-pro/ | 2026-11-23 | earlier tables: 100 mm (same sentence). A community post quotes 102.6 mm from the specs page, but the specs page text showed no value |
| T-nut thread | "Qty 12, M6 Teez-Nutz T-Nuts" | M6 | https://carbide3d.com/shapeoko/shapeoko5pro-specs/ | 2026-11-23 | |
| MDF slat thickness | "3/4" MDF", stands proud of T-slots "approximately 3mm" | 19.05 (proud ~3) | https://carbide3d.com/shapeoko/hybrid-table/ | 2026-11-23 | |
| slot width / depth | UNKNOWN | | | 2026-11-23 | only community-measured values found (not used) |
| S5 slat reference drawing | values printed: 18.00 mm / 0.75"; hole Ø6.60 / Ø11.00 (Ø9/32" / Ø7/16"); 25.40 mm (1.000") edge pitch; slat 4x2/2x2 width 75.00 mm (3.00"), 37.50 (1.50"); lengths 1219.20/1244.60 mm (48/49") for 4x4, 609.6/635.00 mm (24/25") for 4x2/2x2 | as printed | https://guides.carbide3d.com/files/pdf/CS-S5-MDF-REFERENCE.pdf | 2026-11-23 | "S5 ... MDF Filler" drawing; text extraction loses which feature each dim belongs to. Assignments above (width/length) are from layout and must be verified on the drawing before use |
| threaded-insert grid on table | UNKNOWN | | | 2026-11-23 | |

### 1d. Threaded Inserts (Qty 100)
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| thread | "M6x1mm" (metric) / "1/4-20" (inch) | M6x1 / 1/4-20 | https://shop.carbide3d.com/products/threaded-inserts-qty-100 | 2026-11-23 | |
| length | "13mm (~1/2")" | 13 | same | 2026-11-23 | |
| drive | "6mm Hex" | 6 | same | 2026-11-23 | both versions |
| max install hole | "9mm (0.354", Letter T Drill)" | 9 | same | 2026-11-23 | |
| OD | UNKNOWN | | same | 2026-11-23 | |

### 1e. Vacuum Table 24x24 / 24x12
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| size | 24" x 24" | 609.6 x 609.6 | https://shop.carbide3d.com/products/vacuum-table-24x24 | 2026-11-23 | |
| size (smaller) | "24x12" (product name, units not stated) | UNKNOWN until confirmed | https://shop.carbide3d.com/collections/workholding | 2026-11-23 | product page not opened |
| thickness | "20mm thick" | 20 | https://shop.carbide3d.com/products/vacuum-table-24x24 | 2026-11-23 | |
| hose / fittings | 8 mm hose fitting, plug for 2nd port, 5 m of 8 mm hose, 5 m of 5x5 mm gasket | 8; 5x5 | same | 2026-11-23 | |
| mounting | "mount to the T-slots in the Hybrid Table" | | same | 2026-11-23 | |
| grid / zones / vacuum level | UNKNOWN | | same | 2026-11-23 | |
| pump | "Vacuum Pump 5 CFM" (separate product) | n/a | https://shop.carbide3d.com/collections/workholding | 2026-11-23 | flow, not a length |

### 1f. Nomad Threaded Table
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| thickness | "approximately 1/2" thick" | ~12.7 | https://shop.carbide3d.com/products/nomad-threaded-table | 2026-11-23 | |
| thread | "M6x1 threaded holes" | M6x1 | same | 2026-11-23 | |
| hole count | "over 50" | | same | 2026-11-23 | |
| hole pitch | UNKNOWN | | | 2026-11-23 | community posts say 1"; not an official value |
| plate size, material | UNKNOWN | | same | 2026-11-23 | |

### 1g. Nomad Low Profile Vise
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| jaw width | 2" | 50.8 | https://shop.carbide3d.com/products/low-profile-vise | 2026-11-23 | |
| length | 8" | 203.2 | same | 2026-11-23 | |
| max clamping width | 6.5" | 165.1 | same | 2026-11-23 | |
| height, thread, weight | UNKNOWN | | same | 2026-11-23 | mounts with included hardware to Nomad table |

---

## 2. Avid CNC
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| spoilboard project: crossmember spacing | rear two crossmembers "360mm (14-3/16") apart (or 400mm (15-3/4") center to center)" | 360 / 400 | https://www.avidcnc.com/support/instructions/machineSetup/spoilboard/fullsize/ | 2026-11-23 | PRO / Standard machines |
| spoilboard fasteners | "M8 Button Head Cap Screws", roll-in T-nuts | M8 | same | 2026-11-23 | |
| default tooling in project | counterbores/slots/drill patterns 1/4" tool; surfacing 1/2" tool | 6.35 / 12.7 | same | 2026-11-23 | tool sizes, not hole sizes |
| spoilboard thickness | UNKNOWN | | same | 2026-11-23 | not stated on the page |
| dog-hole diameter / grid pitch | UNKNOWN | | same | 2026-11-23 | set in Fusion/Vectric files, not stated on page |
| T-slot / threaded insert grid | UNKNOWN | | | 2026-11-23 | no Avid product page found |
| vacuum table / plenum | UNKNOWN | | | 2026-11-23 | no Avid vacuum product found (forum only) |
| hold-down clamps | UNKNOWN | | | 2026-11-23 | none found |

---

## 3. ShopBot

### 3a. Universal Vacuum Hold Down Deck (Desktop / Desktop MAX)
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| vacuum source | "Fein Turbo II Vac" | | https://www.shopbottools.com/products/holddown | 2026-11-23 | kit: plywood plenum, MDF spoilboard, Fein vac |
| smallest part without tabs | "about 10 square inches" | ~6452 mm^2 | same | 2026-11-23 | |
| deck | "1" thick wooden deck with piping" | 25.4 | https://shopbottools.com/wp-content/uploads/2024/01/SBG00230-Installing-Vacuum-Table.pdf | 2026-11-23 | SBG00230, Dec 14 2017 |
| plenum | "3/4" plywood with channels cut in it" | 19.05 | same | 2026-11-23 | channel size/pitch UNKNOWN |
| fasteners | M6x50 socket head cap screws | M6x50 | same | 2026-11-23 | |
| spoilboard attach | 100% silicone caulk, ~1/4" bead around perimeter | 6.35 bead | same | 2026-11-23 | |
| vacuum level (inHg) | UNKNOWN | | | 2026-11-23 | |
| zones | UNKNOWN | | | 2026-11-23 | |

### 3b. Mini Vacuum Table (gasketed, Desktop)
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| pump | "Becker Vacuum Pump 0.3 hp" | | https://shopbottools.com/wp-content/uploads/2024/01/MiniVacuumTable.pdf | 2026-11-23 | May 2 2016 |
| gauge | "Vacuum Gauge 0-30" Hg" | | same | 2026-11-23 | range of gauge, not achieved level |
| plate stock | "HDPE Sheets (24 by 18 inch)" | 609.6 x 457.2 | same | 2026-11-23 | |
| gasket slot | 1/4" wide x 3/16" deep, inset ~1/8" from part edge, corner radius >= 1/2" | 6.35 x 4.76; 3.18; 12.7 | same | 2026-11-23 | |
| plenum grid | 1/4" bit, 1/16" deep, spacing "approximately 1"" | 6.35; 1.59; 25.4 | same | 2026-11-23 | |
| Desktop T-slot spacing | "1 1/2 inch spacing" | 38.1 | same | 2026-11-23 | |
| tubing | 1/4 blue poly | 6.35 | same | 2026-11-23 | |

### 3c. ShopBot clamps
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| T-slot deck clamps | "toggle clamps mounted in the t-slots, wedge clamps" | UNKNOWN | https://www.shopbottools.com/products/holddown | 2026-11-23 | no dimensions |

---

## 4. Onefinity
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Oops clamp (long) | overall 84 mm, adjustment 51 mm | 84 / 51 | https://www.onefinitycnc.com/product-page/oops-clamps-for-threaded-inserts-large-set | 2026-11-23 | page writes "51mm's" |
| Oops clamp (short) | overall 65 mm, adjustment 32 mm | 65 / 32 | same | 2026-11-23 | |
| hardware | "standard off the shelf M6 or 1/4-20"; jackscrew M6 only | | same | 2026-11-23 | |
| set contents (45 pc) | 4 standard + 4 long clamps, 4 tips, M6x16, M6x30, M6x45 button screws (8 each), washers, Allen key | | same | 2026-11-23 | |
| QCW T-track bolt | UNKNOWN (page 404) | | https://www.onefinitycnc.com/product-page/woodworker-qcw-frame-secure-from-beneath-version | 2026-11-23 | search snippet said 1/4-20 hex; not usable as source |
| T-track pitch / count | UNKNOWN | | | 2026-11-23 | |
| threaded insert grid | UNKNOWN | | | 2026-11-23 | |
| T-track Oops clamp sets | UNKNOWN | | https://www.onefinitycnc.com/product-page/t-track-oops-clamps-small-set | 2026-11-23 | body did not render |

---

## 5. DATRON

### 5a. Vacuum clamping plates (Accessories Catalog)
Source for this table: https://www.datron.com/wp-content/uploads/2022/11/DATRON_Accessories_Catalog.pdf (redirects to datron.com/...), accessed 2026-11-23.

| field | published value | mm | notes |
|---|---|---|---|
| MS-VP-A / MS-VP-A-3 | 500 x 500 x 40 mm, ~20.8 kg, 2 connections | 500 x 500 x 40 | art. 0A01621C (M7), 0A01621B (M8); screw-down or vacuum-suctioned |
| MS-VP-B-1 (M8/M85) | 1,000 x 500 x 60 mm, ~42.0 kg, 2 segments | | 0A01672D |
| MS-VP-B-2 (M8/M85) | 1,000 x 700 x 60 mm, ~58.0 kg, 2 segments | | 0A01672E |
| MS-VP-B-3 (M7/ML) | 500 x 500 x 60 mm, ~19.5 kg, 1 segment | | 0A01672P |
| MS-VP-B-4 (ML/M10/M8Cube) | 1,000 x 500 x 60 mm, ~39.0 kg, 2 segments | | 0A01672Q |
| meandering-groove plate | 1,000x500 (ML), 1,000x700 and 700x500 (M10/M8Cube); "100 x 100 mm fields which can be individually disabled"; two fields | | groove section UNKNOWN |
| VacuPlate | 1,000 x 200 mm (VacuPlate 1000, 1000-AS), 500 x 200 mm (VacuPlate 500) | | |
| VacuCard | 650x450x0.7, 1,000x500x0.7, 1,000x700x0.7 mm; 500 g/m^2 | | |
| pumps VP-B-1/2/3 | 100 / 200 / 250 m^3/h; end pressure 20 / 10 / 10 mbar; 3.0 / 5.5 / 5.5 kW | n/a | rotary vane |
| T-slot module plate MS-TN | 10 T-slots "8.1 for T-nut M6 DIN 508, 25 mm grid size in longitudinal direction"; 460 or 760 x 250 mm; height 20 mm (Type A) / 83 mm (Type B with fixed jaw) | | contact pressure "6 N/cm^2 at -0.6 bar vacuum suction" |
| MS-TN for ML/M7/MV/M8Cube/M10 | 500 / 700 / 1,000 x 250 mm, h 20 or 83 mm; max clamping length with short-stroke clamp 263 / 460 / 760 mm | | |
| module plates MS-P-1/2/3 | 280x180x15, 380x280x15, 500x500x15 mm | | |

### 5b. DATRON vacuum cards (shop)
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| Standard Vacuum Card | 500 mm x 400 mm x 0.7 mm, 50/box | 500x400x0.7 | https://shop.datron.com/product/vacuum-cards | 2026-11-23 | SKU 0A01617K |
| Adhesive Vacuum Card | 500 x 400 x 0.7 mm, 5 mm x 5 mm grid, 20/box | | same | 2026-11-23 | 0A01618W |
| Bulk card | 24" x 36" x .023" | 609.6 x 914.4 x 0.584 | same | 2026-11-23 | DV001002 |

### 5c. DATRON USA work-holding page
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| vacuum table sizes | 500x200 (19.68"x7.87"), 500x500, 500x700, 500x1,000, 1,000x1,000 mm | as given | https://www.datron.com/cnc-machine-options/work-holding/ | 2026-11-23 | holes within meandering grooves in aluminum plate |
| hole pattern / grid pitch / vacuum level | UNKNOWN | | same | 2026-11-23 | |
| quick module plates | conical inserts on 100 mm centers | 100 | same | 2026-11-23 | |
| 4th axis center height | 75 mm (2.95") | 75 | same | 2026-11-23 | |
| 4th axis 3-jaw chuck capacity | up to 3.125" dia | 79.375 | same | 2026-11-23 | |
| 4th axis centric clamp capacity | up to 2.5" wide | 63.5 | same | 2026-11-23 | |
| 4/5-axis mounting plate | 135 mm dia; tilt max 120 deg | 135 | same | 2026-11-23 | |

### 5d. DATRON vises (Accessories Catalog, URL as 5a)
| field | published value | mm | notes |
|---|---|---|---|
| KZS compact centric clamp: jaw width | 100 mm | 100 | art. 0A01088D |
| KZS clamping range | 0-160 mm (clamping depth max 14 mm); 0-75 mm (clamping depth max 32 mm) | | single clamping |
| KZS clamping force | 20 kN at 65 Nm torque (total) | | |
| KZS body L x W x H | 150 x 140 x 105 mm; 9 kg | | |
| MFS multifunctional clamp: jaw width | 100 mm | 100 | 0A01088C |
| MFS clamping range | single 0-310 mm (depth max 32.5) / 0-207 mm (depth max 49); double 0-145 / 0-103 mm | | |
| MFS force / body | 20 kN per jaw; 330 x 118 x 109.5 mm; 16 kg | | |
| fixed jaw ZFB lengths | 250, 1,000, 1,250, 2,500 mm | | for T-slot plates |

---

## 6. Roland DG (MDX / DWX)
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| MDX-50 ZCL-50 rotary centre vice capacity | "Thickness: 10 to 65 mm (0.39 to 2.56 in.) or diameter of 20 to 68 mm (0.79 to 2.68 in.)" | 10-65 / 20-68 | https://www.rolanddg.eu/tr/urunleri/3d/modela-mdx-50/accessories | 2026-11-23 | resins only per page |
| MDX vise jaw width / opening | UNKNOWN | | | 2026-11-23 | no MDX vise found |
| MDX fixture plate | UNKNOWN | | | 2026-11-23 | |
| DWX-52D disc (clamp) | step 98.5 mm dia x 10 mm; body 95 mm dia x 60 mm max | 98.5 / 95 | https://downloadcenter.rolanddg.com/contents/manuals/DWX-52D_USE_EN.pdf | 2026-11-23 | workpiece size the clamp takes |
| DWX-52D pin-type workpiece | pin diameter 6 mm (ø5.9 ± 0.02) | 6 / 5.9 | same | 2026-11-23 | |

---

## 7. Sherline

### 7a. Mill vise P/N 3551
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| jaw width | 2.00" | 50.8 | https://sherline.com/product/3551-milling-vise | 2026-11-23 | page gives "(50.8mm)" |
| jaw depth | 1.00" | 25.4 | same | 2026-11-23 | |
| max opening | 2.00" | 50.8 | same | 2026-11-23 | |
| fixed jaw | horizontal and vertical V-groove | | https://www.sherline.com/wp-content/uploads/2014/07/5325_catalog_web.pdf | 2026-11-23 | 11th ed. catalog p.35 |
| mounting | 2 hold-down clamps (P/N 35580), 10-32 T-nuts (30561), 10-32 x 5/8" SHCS (40330) | | sherline 3551 page | 2026-11-23 | |
| height, overall length | UNKNOWN | | | 2026-11-23 | |
| rotating base #3570 | raises vise 1/2" | 12.7 | catalog (above) | 2026-11-23 | |

### 7b. Hold-down sets
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| T-slot pitch (mill & lathe table) | 1.5" | 38.1 | https://www.sherline.com/standard-dimensions-of-sherline-tools/ | 2026-11-23 | |
| T-slot top width | .25" | 6.35 | same | 2026-11-23 | |
| T-slot bottom (T) width | .40" | 10.16 | same | 2026-11-23 | |
| T-slot depths | upper .10", T .10", total .20" | 2.54 / 2.54 / 5.08 | same | 2026-11-23 | |
| table T-slots | 2 slots on 2.75" x 13.0" table (2.75" x 18" on 5800) | 69.85 x 330.2 (Sherline prints 70 x 330 mm) | catalog 5325 | 2026-11-23 | |
| 3013 step block set: studs | 10-32 x 1, 1.5, 2, 2.5, 3, 3.5" (2 each) | 25.4, 38.1, 50.8, 63.5, 76.2, 88.9 | https://www.mikestools.com/download/Sherline-Instructions/3013inst.pdf | 2026-11-23 | Sherline's instruction sheet (dated 7/24/00) hosted by a dealer |
| 3013 contents | extended T-nut (2), standard 10-32 T-nut (2), step clamp (2), anodized step block (2) + 1 unanodized, nut + radiused washer (2) | | same | 2026-11-23 | |
| 3013 step block heights / step pitch | UNKNOWN | | | 2026-11-23 | figure says blocks cut to 8, 10, 12 steps; no heights |
| 3012 standard set | 10-24 x 1-1/4" carriage bolts; 10-32 T-nuts; 10-32 SHCS 7/8, 1, 1-1/4, 1-1/2"; max height adjustment "about 1-3/16"" | 31.75; 22.2-38.1; ~30.2 | https://www.sherline.com/product/3012-standard-hold-down-set/ | 2026-11-23 | clamp dimensions UNKNOWN |

### 7c. Mill tooling plates
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| 3560 size | 4" x 10" x 1/2" | 101.6 x 254 x 12.7 | https://www.sherline.com/?p=2109 | 2026-11-23 | 6061-T6, not anodized |
| 3560 hardware | 10-32 T-nuts (6), 10-32 x 7/16" SHCS (6) | | same | 2026-11-23 | |
| 3562 / 3563 | 7" x 13" / 7" x 18", 3/4" thick, 3 T-slots, 4 dowel pins | 177.8 x 330.2 / 177.8 x 457.2, 19.05 | catalog 5325 | 2026-11-23 | |
| tapped holes | "drilled and tapped for 10-32" | | catalog 5325 | 2026-11-23 | |
| hole pattern pitch | UNKNOWN | | | 2026-11-23 | |

### 7d. Lathe chucks (spindle nose 3/4-16, #1 Morse; ER-16 option 22 x 1.5 mm)
| chuck | dia | normal capacity | reverse capacity | through hole | thread | source | notes |
|---|---|---|---|---|---|---|---|
| 1041 3-jaw self-centering | 2.5" (63.5) | 3/32"-1-3/16" (2-30 mm) | 1/4"-2-1/4" (6-56 mm) | .687" (17 mm) | 3/4-16 (also 1068 12x1, 1070 1/2-20, 1031 14x1) | https://www.sherline.com/product/1041-3-jaw-self-centering-chuck-2-5/ | |
| 1040 3-jaw self-centering | 3.1" (78.74) | 3/32"-1-1/2" (2-38 mm) | 1/4"-3-1/4" (6-82.5 mm) | .687" (17 mm) | 3/4-16 | https://www.sherline.com/product/3-jaw-self-centering-chuck-3-1/ | 2014 catalog gives reverse max 2.75" (70 mm): conflict |
| 1042 3-jaw ER-16 | 3.1" | 3/32"-1-1/2" (2-38 mm) | 3/32"-3-1/4" (2-82.5 mm) | .687" (17 mm) | 22 x 1.5 mm | https://www.sherline.com/product/1042-er-16-3-jaw-self-centering-chuck-3-1/ | ER-16 headstock only |
| 1044 4-jaw independent | 2.5" | 3/32"-1-3/16" (2-30 mm) | 5/16"-2-1/4" (8-56 mm) | .687" (17 mm) | 3/4-16 (also 1071, 1073, 1035) | https://www.sherline.com/product/4-jaw-independent-chuck-3-1/ | URL slug says 3-1 but page is the 2.5" |
| 1030 4-jaw independent | 3.1" | 3/32"-1-1/2" (2-38 mm) | 5/16"-2-3/4" (8-70 mm) | .687" (17 mm) | 3/4-16 | https://www.sherline.com/product/4-jaw-independent-chuck/ | page text elsewhere says 3-1/4" (82.5) reversed: conflict |
| 1078 4-jaw self-centering ER-16 | 3.1" | 3/32"-1-1/2" (2-38 mm) | 1/4"-2-3/4" (6-70 mm) | .687" (17 mm) | 22 x 1.5 mm | https://www.sherline.com/product/1078-er-16-4-jaw-self-centering-chuck-3-1/ | |
All accessed 2026-11-23. Body thickness and weight: UNKNOWN for all.

### 7e. Collet systems for Sherline lathe
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| WW adapter mount | into headstock #1 Morse taper | | https://www.sherline.com/product/11611163-ww-8mm-collet-adapter-and-drawbar/ | 2026-11-23 | P/N 1161 (WW), 1163 (8 mm) |
| WW through capacity | "up to 3/16"" | 4.76 (catalog states 4.5 mm) | same; catalog 5325 | 2026-11-23 | |
| WW collet sizes | 1/64"-5/16" by 64ths; 0.3-8.0 mm by 0.1 | 0.4-7.94; 0.3-8.0 | same | 2026-11-23 | |
| WW collet thread / shank | ".275-40"; shank .312-.313" | 6.985 dia; 7.92-7.95 | same | 2026-11-23 | |
| WW nose dims | UNKNOWN | | same | 2026-11-23 | |
| ER-16 headstock nose thread | 22 x 1.5 mm external | 22 | https://www.sherline.com/product/sherline-er-16-spindle-headstocks/ | 2026-11-23 | 16 deg internal taper |
| ER-16 spindle bore | .406" | 10.31 | same | 2026-11-23 | |
| ER-16 collet range | ".020-.394", 0.5-10mm" AND ".020-.393" / .5-13 mm" | 0.5-10 or 0.5-13 | same | 2026-11-23 | conflict on the same page; collets not sold by Sherline |
| standard spindle nose | 3/4-16 external, #1 Morse internal; tailstock #0 Morse; center height 1.75" over bed | 44.45 | https://www.sherline.com/standard-dimensions-of-sherline-tools/ | 2026-11-23 | |

---

## 8. Grizzly
grizzly.com product pages refused (HTTP 403). Values below are from Grizzly's own owner's-manual PDFs on cdn0.grizzly.com.

### 8a. Clamping kits
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| G1076 name in older manual | "G1076—52-PC. Clamping Kit" | | https://cdn0.grizzly.com/manuals/g0705_m.pdf | 2026-11-23 | same item later sold as "58-Pc." |
| G1076 T-slot / stud | fits 5/8" T-slots, 1/2"-13 studs | 15.875 slot | https://cdn0.grizzly.com/manuals/g0793_m.pdf | 2026-11-23 | |
| G1076 stud lengths | 24 studs, four each 3, 4, 5, 6, 7, 8" | 76.2-203.2 | same | 2026-11-23 | |
| G1076 other contents | 12 step blocks (3 sizes), 6 T-nuts, 6 flange nuts, 4 coupling nuts, 6 end hold-downs | | same | 2026-11-23 | |
| step block heights | UNKNOWN | | | 2026-11-23 | not in manuals read |
| G1075 58-pc | "Made for 1/2" T-slots"; 24 studs, 6 step block pairs, 6 T-nuts, 6 flange nuts, 4 coupling nuts, 6 end hold-downs | 12.7 slot | https://cdn0.grizzly.com/manuals/g0731_m.pdf (also g0808_m.pdf) | 2026-11-23 | stud thread UNKNOWN |
| T26485 58-pc | fits 7/16" T-slots, 3/8"-16 studs; studs four each 3-8" | 11.1 slot; 76.2-203.2 | https://cdn0.grizzly.com/manuals/g0795_m.pdf | 2026-11-23 | |
| G9511 T-slot nuts | 7/16" slot, 3/8"-16 | | https://cdn0.grizzly.com/manuals/g0704_m.pdf | 2026-11-23 | |
| G0704 mill table | 3 T-slots, 7/16" size, 2-1/2" centers | 11.1; 63.5 | same | 2026-11-23 | |

### 8b. Mill vises
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| G7156 / G7154 / G7155 | "4" (3 5/8")", "5" (4 1/2")", "6" (5 5/8")" Precision Milling Vise | 101.6 (92.1) / 127 (114.3) / 152.4 (142.9) | https://cdn0.grizzly.com/manuals/g0758_m.pdf | 2026-11-23 | meaning of the bracketed figure is not stated; opening/height UNKNOWN |
| T10253 mini self-centering vise | 2" jaw width, 2-1/16" jaw opening | 50.8 / 52.4 | https://cdn0.grizzly.com/manuals/g0727_m.pdf | 2026-11-23 | swivel base |
| T10063 / T10064 milling vise | "12 5/16" x 6 9/16"" / "17 1/8" x 8 3/4"" | 312.7 x 166.7 / 435.0 x 222.3 | https://cdn0.grizzly.com/manuals/g0731_m.pdf | 2026-11-23 | which dims these are is not stated |
| G9002-G5975 swivel vises | 2-1/2, 3-1/2, 4, 5, 6, 8" | | https://cdn0.grizzly.com/manuals/g0463_m.pdf | 2026-11-23 | nominal sizes only |

### 8c. Lathe chucks supplied with Grizzly lathes
| lathe | 3-jaw | 4-jaw | faceplate | spindle nose | bore / taper | source | accessed |
|---|---|---|---|---|---|---|---|
| G0602/G0752 (10x22) | 5" (127) | 6-1/2" independent (165.1) | 8" (203.2) | threaded, "Spindle Size 1 3/4"", "8 TPI" (44.45) | 25 mm / MT#4 | https://cdn0.grizzly.com/manuals/g0602_m.pdf | 2026-11-23 |
| G0709 (14x40) | 6" (152.4) reversible jaws | 8" independent (203.2) | 11" (279.4) | D1-5 Camlock | 1.57" (39.9) / MT#5 | https://cdn0.grizzly.com/manuals/g0709_m.pdf | 2026-11-23 |
| G0768/G0769 (8x16) | 4" (101.6) | 4" (parts list) | 6-3/4" (171.5) | "Intrinsic Back Plate" | 0.787" (20 mm) / MT#3 | https://cdn0.grizzly.com/manuals/g0768_m.pdf | 2026-11-23 |
Chuck capacities (jaw ranges): UNKNOWN in these manuals.

---

## 9. SYIL
| field | published value | mm | source URL | accessed | notes |
|---|---|---|---|---|---|
| vise/fixture included | none listed | | https://us.syil.com/x7 | 2026-11-23 | options: 125 mm 4th-axis rotary table, tailstock |
| X7 table | 27.56" x 11.81" (700 x 300 mm) | 700 x 300 | same | 2026-11-23 | |
| X7 T-slots | 3 x 14 mm x 100 mm (number x size x pitch) | 14 / 100 | same | 2026-11-23 | one line shows "0.12" for the count, a typo; comparison table gives 3 |
| X7 max table load | 331 lb (150 kg) | | same | 2026-11-23 | |
| SYIL spares store | "L2 Spare jaws", "L3 Spare jaws" (L-series lathe), no dimensions | UNKNOWN | https://store.syil.com/en-US/spares | 2026-11-23 | |
| SYIL vise specs | UNKNOWN | | | 2026-11-23 | support.syil.com unreachable from this session |
