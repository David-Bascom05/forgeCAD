# Haas Tooling workholding models: inventory

David's upload (branch `haas-tooling-step` @ 34adf0c7, research/haas-tooling-workholding/): 299 STEP files named by Haas part
number. David, verbatim: "They are totally ok to include in the repo I checked."

Each part's name and category are from its public product page, https://www.haastooling.com/p/PART (the page's
schema.org Product and BreadcrumbList), fetched 2026-11-25 with plain curl (no sign-in). Sizes are the imported
solids' box (forge-step stepread, release, mm), in the model's own frame: not yet a placement. 299 files are 285
distinct models: inch/metric twins and kits share one file (the "same file as" column).

Import: "ok" = every solid read; "partial" = some solids refused (kinds counted); "none" = no solid read (a
surface or faceted model: KERNEL-STRESS's corpus). Times are the whole read.

## Automation Accessories > Robot Cell Accessories

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0571 | Zero Point System Pull Stud | 35.0 × 32.1 × 32.1 | 1 / 69 | ok | 143 |  |

## Lathe Workholding > Lathe Collets, Chucks, & Liners > Hydraulic Power Chuck

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0930 | 10" (254mm) Through-Hole 3-Jaw Hydraulic Power Chuck, A2-6 Spindle Nose, M85x2 Drawbar Nut | 185.1 × 262.2 × 260.0 | 36 / 4131 | ok | 1494 |  |
| 05-0931 | 18" (457mm) Through-Hole 3-Jaw Hydraulic Power Chuck, A2-11 Spindle Nose, M175x3 Drawbar Nut | 283.0 × 457.5 × 456.0 | 39 / 4715 | ok | 756 |  |
| 05-0932 | 10" (254mm) Through-Hole 3-Jaw Hydraulic Power Chuck, A2-6 Spindle Nose, M75x2 Drawbar Nut | 185.1 × 262.2 × 260.0 | 36 / 4131 | ok | 1416 |  |
| 05-0933 | 12" (305mm) Through-Hole 3-Jaw Hydraulic Power Chuck, A2-6 Spindle Nose, M85x2 Drawbar Nut | 200.0 × 304.1 × 304.0 | 35 / 4137 | ok | 1770 |  |
| 05-0934 | 12" (305mm) Through-Hole 3-Jaw Hydraulic Power Chuck, A2-8 Spindle Nose, M115x2 Drawbar Nut | 210.4 × 316.1 × 314.9 | 36 / 4052 | ok | 1458 |  |
| 05-0935 | 8" (210mm) Through-Hole 3-Jaw Hydraulic Power Chuck, A2-5 Spindle Nose, M60x1.5 Drawbar Nut | 169.0 × 211.5 × 210.0 | 36 / 3542 | ok | 1471 |  |
| 05-0936 | 8" (210mm) Through-Hole 3-Jaw Hydraulic Power Chuck, A2-6 Spindle Nose, M75x2 Drawbar Nut | 159.2 × 211.5 × 210.0 | 33 / 3456 | ok | 1459 |  |
| 05-0937 | 5" (135mm) Through-Hole 3-Jaw Hydraulic Power Chuck, A2-5 Spindle Nose, M35x1.5 Drawbar Nut | 119.5 × 135.5 × 135.0 | 29 / 2426 | ok | 956 |  |

## Lathe Workholding > Lathe Collets, Chucks, & Liners > Quick-Change Collet Chucks

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 04-0298 | HQCBD-65, Dead Length Collet Chuck for A2-6 Spindle Nose, M75x2 Drawbar Nut | 131.5 × 165.0 × 165.0 | 8 / 535 | ok | 125 |  |
| 04-0299 | HQCB-42, Pull Back Collet Chuck for A2-5 Spindle Nose, M55x2 Drawbar Nut | 97.0 × 139.9 × 140.0 | 7 / 281 | ok | 40 |  |
| 05-0331 | HQCBD-52, Dead Length Collet Chuck for A2-5 Spindle Nose, M55x2 Drawbar Nut | 89.0 × 140.0 × 140.0 | 8 / 735 | ok | 377 |  |
| 05-0332 | HQCB-65, Pull Back Collet Chuck for A2-6 Spindle Nose, M75x2 Drawbar Nut | 78.5 × 165.0 × 165.0 | 6 / 545 | ok | 284 |  |
| 05-0334 | HQCBD-65, Dead Length Collet Chuck for A2-6 Spindle Nose, M85x2 Drawbar Nut | 131.5 × 202.0 × 202.0 | 9 / 621 | ok | 452 |  |
| 05-0335 | HQCB-80, Pull Back Collet Chuck for A2-8 Spindle Nose, M115x2 Drawbar Nut | 143.0 × 210.0 × 209.9 | 8 / 563 | partial MANIFOLD_SOLID_BREP 1 | 313 |  |
| 05-0336 | HQCBD-65, Dead Length Collet Chuck for A2-8 Spindle Nose, M115x2 Drawbar Nut | 131.5 × 202.0 × 202.0 | 9 / 634 | ok | 402 |  |
| 08-2075 | HQCB-65, Pull Back Collet Chuck for A2-6 Spindle Nose, M60x1.5 Drawbar Nut | 78.5 × 165.0 × 165.0 | 6 / 545 | ok | 284 | 05-0332 |
| 08-2076 | HQCB-65, Pull Back Collet Chuck for A2-6 Spindle Nose, M85x2 Drawbar Nut | 78.5 × 165.0 × 165.0 | 6 / 545 | ok | 284 | 05-0332 |
| 08-2077 | HQCBD-65, Dead Length Collet Chuck for A2-6 Spindle Nose, M60x1.5 Drawbar Nut | 131.5 × 165.0 × 165.0 | 8 / 499 | ok | 805 |  |

## Lathe Workholding > Lathe Collets, Chucks, & Liners > Scroll Chucks

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0582 | 16" (400mm) 3-Jaw Scroll Chuck, A2-8 Spindle Nose | 400.0 × 400.0 × 200.3 | 1 / 2345 | ok | 395 |  |
| 05-0583 | 20" (500mm) 3-Jaw Scroll Chuck, A2-11 Spindle Nose | 500.0 × 502.5 × 221.3 | 1 / 3191 | ok | 2009 |  |
| 05-0807 | 8" (203mm) 3-Jaw Scroll Chuck, A2-6 Spindle Nose |  | 0 / None | none |  |  |
| 05-0808 | 10" (254mm) 3-Jaw Scroll Chuck, A2-6 Spindle Nose | 165.5 × 260.0 × 250.0 | 1 / 68 | ok | 4 |  |
| 05-0809 | 8" (203mm) 3-Jaw Scroll Chuck, A2-5 Spindle Nose | 203.0 × 203.0 × 51.0 | 1 / 227 | ok | 67 |  |

## Lathe Workholding > Lathe Workholding Accessories > Lathe Jaws

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0131 | Aluminum Machinable Lathe Jaws for 5" (135mm) Chuck, Standard Point, 3 Pc. Set | 38.1 × 25.4 × 63.5 | 1 / 188 | ok | 8 |  |
| 05-0133 | Aluminum Machinable Lathe Jaws for 5" (135mm) Chuck, Round, 3 Pc. Set | 132.0 × 60.3 × 76.2 | 1 / 218 | ok | 11 |  |
| 05-0135 | Aluminum Machinable Lathe Jaws for 6" (165mm) Chuck, Standard Flat, 3 Pc. Set | 31.7 × 38.1 × 76.2 | 1 / 222 | ok | 10 |  |
| 05-0136 | Soft Steel Machinable Lathe Jaws for 6" (165mm) Chuck, Standard Point, 3 Pc. Set | 31.8 × 38.1 × 76.2 | 1 / 218 | ok | 10 |  |
| 05-0137 | Soft Steel Machinable Lathe Jaws for 6" (165mm) Chuck, Standard Flat, 3 Pc. Set | 31.7 × 38.1 × 76.2 | 1 / 222 | ok | 10 |  |
| 05-0138 | Aluminum Machinable Lathe Jaws for 6" (165mm) Chuck, Round, 3 Pc. Set | 129.1 × 60.3 × 73.0 | 1 / 208 | ok | 10 |  |
| 05-0139 | Aluminum Machinable Lathe Jaws for 8" (210mm) Chuck, Standard Point, 3 Pc. Set | 38.1 × 50.8 × 101.6 | 1 / 284 | ok | 13 |  |
| 05-0141 | Soft Steel Machinable Lathe Jaws for 8" (210mm) Chuck, Standard Point, 3 Pc. Set | 38.1 × 50.8 × 101.6 | 1 / 284 | ok | 12 |  |
| 05-0143 | Aluminum Machinable Lathe Jaws for 8" (210mm) Chuck, Round, 3 Pc. Set | 173.2 × 60.3 × 98.4 | 1 / 274 | ok | 13 |  |
| 05-0144 | Aluminum Machinable Lathe Jaws for 10" (254mm) Chuck, Standard Point, 3 Pc. Set | 38.1 × 114.3 × 50.8 | 1 / 316 | ok | 15 |  |
| 05-0145 | Aluminum Machinable Lathe Jaws for 10" (254mm) Chuck, Standard Flat, 3 Pc. Set | 38.1 × 114.3 × 50.8 | 1 / 324 | ok | 15 |  |
| 05-0148 | Aluminum Machinable Lathe Jaws for 10" (254mm) Chuck, Round, 3 Pc. Set | 217.2 × 63.5 × 123.8 | 1 / 340 | ok | 16 |  |
| 05-0149 | Aluminum Machinable Lathe Jaws for 12" (305mm) Chuck, Standard Point, 3 Pc. Set | 50.8 × 50.8 × 139.7 | 1 / 380 | ok | 19 |  |
| 05-0150 | Aluminum Machinable Lathe Jaws for 12" (305mm) Chuck, Standard Flat, 3 Pc. Set | 50.8 × 50.8 × 139.7 | 1 / 392 | ok | 17 |  |
| 05-0152 | Soft Steel Machinable Lathe Jaws for 12" (305mm) Chuck, Standard Flat, 3 Pc. Set | 50.8 × 50.8 × 139.7 | 1 / 392 | ok | 18 |  |
| 05-0153 | Aluminum Machinable Lathe Jaws for 12" (305mm) Chuck, Round, 3 Pc. Set | 261.2 × 63.5 × 149.2 | 1 / 404 | ok | 21 |  |
| 05-0155A | Aluminum Machinable Lathe Jaws for 15" (381mm) Chuck, Standard Flat | 63.5 × 76.2 × 165.1 | 1 / 428 | ok | 19 |  |

## Lathe Workholding > Lathe Workholding Accessories > Live Centers

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 04-0177 | 3MT #3 Morse Taper Live Center, High Precision | 166.0 × 50.0 × 49.9 | 1 / 258 | ok | 468 |  |
| 04-0178 | 4MT #4 Morse Taper Live Center, High Precision | 211.3 × 66.0 × 66.0 | 1 / 283 | ok | 557 |  |
| 04-0179 | 5MT #5 Morse Taper Live Center, High Precision | 270.5 × 80.0 × 80.0 | 1 / 279 | ok | 611 |  |
| 05-0262 | 3MT #3 Morse Taper Live Center, Extended Clearance | 180.0 × 50.0 × 50.0 | 1 / 308 | ok | 728 |  |
| 05-0263 | 4MT #4 Morse Taper Live Center, Extended Clearance | 241.1 × 66.0 × 66.0 | 1 / 317 | ok | 710 |  |
| 05-0264 | 5MT #5 Morse Taper Live Center, Extended Clearance | 294.0 × 80.0 × 80.0 | 1 / 302 | ok | 666 |  |

## Mill Workholding > Mill Chucks & Mounts > 3-Jaw Chucks & Mounts

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0222 | 8" (210mm) 3-Jaw Mill Chuck | 200.0 × 122.0 × 200.0 | 5 / 2062 | ok | 5298 |  |
| 05-0223 | Flat Mill Table Adapter Plate, 8" (210mm) 3-Jaw Mill Chuck |  | 0 / None | none |  |  |
| 05-0224 | 90° Mill Table Adapter Plate, 8" (210mm) 3-Jaw Mill Chuck |  | 0 / None | none |  |  |
| 05-0266 | 6" (165mm) NBK 3-Jaw Thin Mill Chuck | 97.7 × 220.0 × 220.0 | 1 / 2429 | ok | 2070 |  |
| 05-0267 | 8" (210mm) 3-Jaw NBK Thin Mill Chuck | 270.0 × 110.5 × 270.0 | 1 / 2721 | ok | 1508 |  |
| 05-0268 | 10" (254mm) 3-Jaw NBK Thin Mill Chuck | 315.0 × 125.0 × 315.0 | 1 / 2221 | ok | 1716 |  |
| 05-0272 | 12" (305mm) 3-Jaw NBK Thin Mill Chuck | 370.0 × 138.6 × 370.0 | 1 / 2988 | ok | 1713 |  |

## Mill Workholding > Mill Chucks & Mounts > 5C Indexer Chucks

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0779 | 5" (127 mm) 3-Jaw Scroll Chuck for 5C Indexer | 125.0 × 56.5 × 125.0 | 7 / 366 | ok | 235 |  |

## Mill Workholding > Mill Chucks & Mounts > Collet Fixtures

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0592 | 5C Collet Manual Fixture | 131.5 × 121.0 × 122.5 | 13 / 677 | ok | 260 |  |

## Mill Workholding > Mill Workholding Accessories > Adapter Plates

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0090 | Riser Block for 5" (130mm) Self-Centering Vise | 279.4 × 279.3 × 100.0 | 1 / 224 | ok | 22 |  |
| 05-0204 | 310mm Rotary Adapter Plate for 5" (130mm) Self Centering Vise | 243.9 × 35.0 × 244.0 | 1 / 633 | ok | 795 |  |
| 05-0205 | 160/210mm Rotary Adapter Plate for 3" (75mm) Self Centering Vise | 164.9 × 25.0 × 165.0 | 1 / 62 | ok | 6 |  |
| 05-0500 | Riser Block for 130mm (5") Self-Centering Vise | 279.4 × 279.3 × 100.0 | 1 / 166 | ok | 17 |  |
| 05-0556 | 3-Sided Pyramid for 75mm (3") and 130mm (5") Self-Centering Vises | 325.0 × 325.0 × 217.0 | 6 / 410 | ok | 994 |  |
| 05-0564 | 4-Sided Block for 75mm (3") and 130mm (5") Self-Centering Vises | 300.0 × 299.9 × 300.7 | 4 / 309 | ok | 374 |  |
| 05-0576 | 6-Sided Block for 130mm (5") Self-Centering Vises | 325.0 × 325.0 × 290.3 | 4 / 378 | ok | 453 |  |
| 05-0819 | 4 in / 100mm Double Station Vise Adapter Plate for Haas VF T-Slot Tables | 165.0 × 15.0 × 420.0 | 1 / 72 | ok | 6 |  |
| 05-0820 | 6 in / 150mm Double Station Vise Adapter Plate for Haas VF T-Slot Tables | 220.0 × 20.0 × 540.0 | 1 / 72 | ok | 6 |  |
| 05-0830 | 6'' Swivel Base for Milling Vise | 24.9 × 57.4 × 198.0 | 5 / 3557 | partial MANIFOLD_SOLID_BREP 1 | 27484 |  |

## Mill Workholding > Mill Workholding Accessories > Angle Plates

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 09-0586 | Solid Right Angle Plate, Handle Gusset, 5" (125mm) Length x 3-3/4" (95mm) Width x 4" (100mm) Height | 127.0 × 101.6 × 95.2 | 1 / 28 | ok | 2 |  |
| 09-0587 | Solid Right Angle Plate, Handle Gusset, 8" (200mm) Length x 4-1/2" (110mm) Width x 5" (125mm) Height | 203.2 × 127.0 × 114.3 | 1 / 28 | ok | 2 |  |
| 09-0588 | Solid Right Angle Plate, Handle Gusset, 10" (150mm) Length x 5-1/2" (135mm) Width x 7" (175mm) Height | 254.0 × 177.8 × 139.7 | 1 / 28 | ok | 2 |  |
| 09-0589 | Slotted Angle Plate, Webbed Gusset, 5" (125mm) Length x 6" (150mm) Width x 4-1/2" (110mm) Height | 127.0 × 114.3 × 152.4 | 1 / 34 | ok | 2 |  |
| 09-0590 | Slotted Angle Plate, Open-Ended, 5" (125mm) Length x 6" (150mm) Width x 4-1/2" (110mm) Height | 127.0 × 114.3 × 152.4 | 1 / 30 | ok | 2 |  |
| 09-0591 | Slotted Angle Plate, Webbed Gusset, 6" (150mm) Length x 9"(225mm) Width x 7" (175mm) Height | 152.4 × 177.8 × 228.6 | 1 / 64 | ok | 5 |  |
| 09-0592 | Slotted Angle Plate, Open-Ended, 6" (150mm) Length x 9" (225mm) Width x 7" (175mm) Height | 151.0 × 178.0 × 228.0 | 1 / 155 | ok | 8 |  |

## Mill Workholding > Mill Workholding Accessories > Clamp Force Gauges

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 09-0571 | 30-Taper Spindle Drawbar Force Gauge, 50kg/cm2 Range | 172.9 × 72.0 × 103.7 | 7 / 755 | ok | 885 |  |
| 09-0572 | 40-Taper Spindle Drawbar Force Gauge, 150kg/cm2 Range | 174.8 × 72.0 × 103.7 | 7 / 790 | ok | 971 |  |
| 09-0573 | 50-Taper Spindle Drawbar Force Gauge, 250kg/cm2 Range | 172.9 × 96.6 × 117.7 | 7 / 750 | ok | 892 |  |
| 09-0574 | HSK63A Spindle Drawbar Force Gauge, 350kg/cm2 Range | 148.0 × 74.5 × 105.0 | 5 / 808 | ok | 1114 |  |

## Mill Workholding > Mill Workholding Accessories > Tooling Fixture Subplates

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0581 | UMC-500/750 Rectangle Grid Steel Tooling Fixture Subplate, 8-1/2" Width x 15-3/4" Length | 215.0 × 400.0 × 19.0 | 1 / 106 | ok | 11 |  |
| 08-1879 | VF-2YT Rectangle Grid Aluminum Tooling Fixture Subplate, 18" Width x 36" Length | 914.4 × 457.2 × 25.0 | 1 / 248 | ok | 31 |  |
| 08-1880 | VF-2YT Rectangle Grid Steel Tooling Fixture Subplate, 18" Width x 36" Length | 914.4 × 457.2 × 19.0 | 1 / 248 | ok | 23 |  |
| 08-1881 | VF-2YT Rectangle Blank Aluminum Tooling Fixture Subplate, 18" Width x 36" Length | 914.4 × 457.2 × 25.0 | 1 / 68 | ok | 5 |  |
| 08-1882 | VF-2YT Rectangle Blank Steel Tooling Fixture Subplate, 18" Width x 36" Length | 914.4 × 457.2 × 19.0 | 1 / 68 | ok | 6 |  |
| 08-1883 | UMC-750 Round Grid Aluminum Tooling Fixture Subplate, 19.7" Base Diameter | 500.4 × 500.3 × 25.0 | 1 / 312 | ok | 33 |  |
| 08-1884 | UMC-750 Round Grid Steel Tooling Fixture Subplate, 19.7" Base Diameter | 500.4 × 500.3 × 19.0 | 1 / 312 | ok | 32 |  |
| 08-1885 | UMC-750 Round Blank Aluminum Tooling Fixture Subplate, 19.7" Base Diameter | 500.4 × 500.3 × 25.0 | 1 / 36 | ok | 4 |  |
| 08-1886 | UMC-750 Round Blank Steel Tooling Fixture Subplate, 19.7" Base Diameter | 500.4 × 500.3 × 19.0 | 1 / 36 | ok | 4 |  |
| 08-2016 | VF-4 Rectangle Grid Aluminum Tooling Fixture Subplate, 19-1/2" Width x 26" Length x 3/4" Thick | 660.4 × 495.3 × 19.0 | 1 / 188 | ok | 17 |  |
| 08-2017 | VF-4 Rectangle Grid Steel Tooling Fixture Subplate, 19-1/2" Width x 26" Length x 1/2" Thick | 660.4 × 495.3 × 12.7 | 1 / 188 | ok | 17 |  |
| 08-2018 | VF-4 Rectangle Grid Aluminum Tooling Fixture Subplate, 19-1/2" Width x 26" Length x 1/2" Thick | 660.4 × 495.3 × 12.7 | 1 / 188 | ok | 17 | 08-2017 |

## Mill Workholding > Mill Workholding Accessories > Vise Jaws

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0050 | Soft Steel 5" (130mm) x 2-1/2" (64mm) Machinable Vise Jaws | 64.0 × 50.8 × 125.0 | 1 / 32 | ok | 3 |  |
| 05-0054 | Hard Steel 4" (100mm) Non-Machinable Serrated Vise Jaws | 101.6 × 34.5 × 15.0 | 1 / 125 | ok | 31 |  |
| 05-0085 | Soft Steel 4" (100mm) x 1-1/2" (38mm) Machinable Vise Jaws | 101.6 × 44.5 × 38.1 | 1 / 16 | ok | 2 |  |
| 05-0162 | Hard Steel 150mm (6") Non-Machinable Serrated Self-Centering Air Vise Jaws | 150.0 × 34.5 × 100.0 | 1 / 1289 | ok | 103 |  |
| 05-0165 | Hard Steel 75mm (3") Non-Machinable Serrated Self-Centering Air Vise Jaws | 75.0 × 20.5 × 60.0 | 1 / 681 | ok | 98 |  |
| 05-0169 | Soft Steel 4" (100mm) Machinable Self-Centering Hydraulic Vise Jaws - Haas 6" (150mm) Chuck Vise | 101.6 × 43.1 × 84.4 | 1 / 477 | ok | 26 |  |
| 05-0173 | Soft Steel 6" (150mm) Machinable Self-Centering Hydraulic Vise Jaws - Haas 8" (200mm) Chuck Vise | 152.4 × 43.1 × 94.4 | 1 / 533 | ok | 31 |  |
| 05-0175 | Aluminum 8" (200mm) Machinable Self-Centering Hydraulic Vise Jaws - Haas 10" (250mm) Chuck Vise | 203.2 × 55.8 × 104.4 | 1 / 587 | ok | 34 |  |
| 05-0216 | Soft Steel 150mm (6") Machinable Self-Centering Air Vise Jaws, Serrated | 150.0 × 60.0 × 100.0 | 1 / 962 | ok | 56 |  |
| 05-0229 | Hard Steel 4" (100mm) Non-Machinable Dovetail Vise Jaws | 102.0 × 35.0 × 15.0 | 1 / 20 | ok | 2 |  |
| 05-0234 | Soft Steel 8" (200mm) x 1-1/2" (38mm) Machinable Vise Jaws | 203.2 × 55.9 × 38.1 | 1 / 16 | ok | 2 |  |
| 05-0236 | Aluminum 8" (200mm) x 1" (25mm) Machinable Vise Jaws | 203.2 × 55.9 × 25.4 | 1 / 16 | ok | 2 |  |
| 05-0239 | Hard Steel 8" (200mm) Non-Machinable Serrated Vise Jaws | 203.2 × 58.4 × 27.3 | 1 / 184 | ok | 52 |  |
| 05-0240 | Hard Steel 8" (200mm) Non-Machinable Blank Vise Jaws | 203.2 × 55.9 × 27.3 | 1 / 16 | ok | 2 |  |
| 05-0242 | Aluminum Machinable Chuck Jaws - Haas 8" (200mm) 3-Jaw Mill Chuck | 90.0 × 55.0 × 30.0 | 1 / 38 | ok | 3 |  |
| 05-0257 | Hard Steel Non-Machinable Chuck Jaws - Haas 8" (200mm) 3-Jaw Mill Chuck | 80.0 × 45.0 × 27.0 | 1 / 142 | ok | 146 |  |
| 05-0502 | Hard Steel 130mm (5") Non-Machinable Self-Centering Vise Step Jaws | 130.0 × 29.0 × 60.0 | 1 / 56 | ok | 12 |  |
| 05-0770 | Hard Steel 4" (100mm) Non-Machinable V-Groove Vise Jaws | 101.6 × 34.5 × 40.0 | 1 / 94 | ok | 7 |  |

## Mill Workholding > Mill Workholding Accessories > Vise Mounts & Fixtures

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0177 | 16mm T-Slot Hardware, 18mm Toe Clamps, 1/2"-13 Thread - Pack of 4 | 84.0 × 43.9 × 39.0 | 4 / 121 | ok | 252 |  |
| 05-0178 | 16mm T-Slot Hardware, 30mm Toe Clamps, 1/2"-13 Thread - Pack of 4 | 78.6 × 30.2 × 28.6 | 3 / 132 | partial MANIFOLD_SOLID_BREP 1 | 11 |  |
| 05-0180 | Vise Fixture Keys, 16mm T-Slot - Pack of 2 | 15.9 × 17.2 × 27.8 | 2 / 115 | ok | 321 |  |
| 05-0246 | Multi Axis Mill Vise Stop, Medium | 39.8 × 125.0 × 218.0 | 11 / 365 | ok | 1500 |  |
| 05-0247 | Multi Axis Mill Vise Stop, Large | 52.1 × 153.0 × 218.0 | 10 / 492 | ok | 2167 |  |
| 05-0638 | DC-1 Rotary Fixture Key, 1.9" (49.4 mm) Length, 14mm T-Slot | 49.4 × 10.0 × 13.9 | 1 / 21 | ok | 2 |  |
| 05-0639 | DC-1 Rotary Fixture Key, 2" (50.7 mm) Length, 14mm T-Slot | 50.7 × 10.0 × 15.9 | 1 / 20 | ok | 2 |  |
| 05-0641 | DC-1 Rotary Fixture Key,  5.1" (129.5 mm) Length, 14mm T-Slot | 129.4 × 10.0 × 15.9 | 1 / 20 | ok | 3 |  |
| 05-0642 | DC-1 Rotary Locating Pin, 14mm T-Slot | 13.9 × 15.5 × 13.9 | 1 / 16 | ok | 3 |  |
| 05-0643 | DC-1 Vise Fixture Keys, 14mm T-Slot - Pack of 2 | 25.0 × 12.0 × 15.9 | 1 / 15 | ok | 1 |  |
| 06-0123 | 16mm T-Slot Hardware, M12x1.75 Thread - Pack of 2 | 38.5 × 78.5 × 34.1 | 3 / 37 | ok | 4 |  |
| 06-0517 | 14mm T-Slot Hardware, M10x1.5 Thread - Pack of 6 | 30.0 × 23.0 × 40.5 | 2 / 577 | ok | 544 |  |
| 06-0518 | 14mm T-Slot Hardware, M12x1.75 Thread - Pack of 6 | 31.0 × 24.0 × 41.5 | 2 / 572 | ok | 536 |  |

## Mill Workholding > Mill Workholding Accessories > Zero Point System

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0569 | Zero Point System Pallet, Single Station | 145.0 × 30.0 × 145.0 | 1 / 43 | ok | 6 |  |
| 05-0572 | Zero Point System, Dual Station | 383.0 × 125.0 × 190.0 | 4 / 597 | ok | 377 |  |
| 05-0573 | Zero Point System Pallet, Dual Station, Steel |  | 1 / 248 | ok | 246 |  |
| 05-0574 | Zero Point System Pallet, Dual Station, Aluminum |  | 1 / 248 | ok | 248 |  |
| 05-0593 | Zero Point System Locating Pin | 11.0 × 20.5 × 11.0 | 1 / 26 | ok | 4 |  |

## Mill Workholding > Mill Workholding Kits > 3-Jaw Chuck Kits

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 08-1717 | 8"(200mm) 3-Jaw Chuck to Mill Table Adapter Plate Kit, Flat |  | 0 / None | none |  |  |
| 08-1718 | 8"(200mm) 3-Jaw Chuck to Mill Table Adapter Plate Kit, 90 Degree | 200.0 × 122.0 × 200.0 | 5 / 2062 | ok | 5326 |  |

## Mill Workholding > Mill Workholding Kits > Automated Workholding Kits

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 08-1853 | 75mm (3") Pneumatic Self-Centering Manual Vise Kit for Haas TRT210 / UMC Series 350HD/400 | 100.0 × 145.5 × 124.5 | 13 / 2753 | partial MANIFOLD_SOLID_BREP 1 | 2035 | 05-0407 |
| 08-1854 | 75mm (3") Pneumatic Self-Centering Automatic Vise Kit for Haas TRT210 / UMC Series 350HD/400 | 100.0 × 145.5 × 124.5 | 13 / 2753 | partial MANIFOLD_SOLID_BREP 1 | 2035 | 05-0407 |
| 08-1857 | 75mm (3") Pneumatic Self-Centering Vise Kit for Haas VMC | 190.0 × 120.6 × 190.0 | 33 / 4798 | ok | 4487 |  |
| 08-1860 | 75mm (3") Pneumatic Self-Centering Manual Vise Kit for Haas TRT160 / UMC-350 | 125.0 × 111.8 × 125.0 | 29 / 4711 | ok | 4608 |  |
| 08-1861 | 75mm (3") Pneumatic Self-Centering Automatic Vise Kit for Haas TRT160 / UMC-350 | 125.0 × 111.8 × 125.0 | 25 / 4639 | ok | 4477 |  |
| 08-1862 | 6" (150mm) Hydraulic Vise Kit for Haas UMC Series 500/750/1000 | 270.0 × 270.0 × 165.0 | 8 / 464 | ok | 44 | 05-0547 |
| 08-1863 | 8" (200mm) Hydraulic Vise Kit for Haas UMC Series 500/750/1000 | 270.0 × 270.0 × 165.0 | 8 / 534 | ok | 50 | 05-0548 |
| 08-1864 | 10" (250mm) Hydraulic Vise Kit for Haas UMC Series 500/750/1000 | 314.9 × 315.0 × 165.0 | 8 / 562 | ok | 53 | 05-0549 |
| 08-1865 | 100mm (4") Pneumatic Self-Centering Manual Vise Kit for Haas TRT310 |  | 0 / None | none |  |  |
| 08-1866 | 100mm (4") Pneumatic Self-Centering Automatic Vise Kit for Haas TRT310 |  | 0 / None | none |  |  |

## Mill Workholding > Mill Workholding Kits > Rotary Vise Kits

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0577 | 8"(200mm), HRT210 Air Chuck | 180.0 × 260.0 × 260.0 | 44 / 3794 | ok | 1603 |  |
| 08-1931 | 8"(200mm) 3-Jaw Air Chuck and Adapter Kit for HRT210 Rotary | 180.0 × 260.0 × 260.0 | 44 / 3794 | ok | 1603 | 05-0577 |

## Mill Workholding > Mill Workholding Kits > Vise Kits

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 08-1887 | 4-Sided 75mm (3") Self-Centering Vise Block Kit | 300.0 × 299.9 × 415.4 | 14 / 3634 | ok | 4690 |  |
| 08-1888 | 4-Sided 130mm (5") Self-Centering Vise Block Kit | 300.0 × 299.9 × 348.4 | 21 / 3576 | ok | 8094 |  |
| 08-1889 | 6-Sided 130mm (5") Self-Centering Vise Block Kit | 325.0 × 325.0 × 338.0 | 21 / 3644 | ok | 6918 |  |
| 08-1986 | 3-Sided 3" Self-Centering Vise Pyramid Kit | 325.0 × 399.7 × 436.5 | 36 / 6849 | ok | 17830 |  |
| 08-1987 | 3-Sided 5" Self-Centering Vise Pyramid Kit | 325.0 × 402.2 × 463.3 | 16 / 2918 | ok | 8111 |  |

## Mill Workholding > Specialty Fixture Clamps & Rails > DT Slide Clamps by Mitee-Bite

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0929 | DT Slide Clamp, Haas Workholding by Mitee-Bite | 25.4 × 30.8 × 72.0 | 6 / 278 | ok | 436 |  |

## Mill Workholding > Specialty Fixture Clamps & Rails > ID Xpansion™ Clamps by Mitee-Bite

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0744 | Size #00 ID Xpansion Clamp, 0.16" to 0.29" Machinable Range, Haas Workholding by Mitee-Bite | 20.0 × 14.9 × 20.0 | 5 / 354 | ok | 1327 |  |
| 05-0745 | Size #0 ID Xpansion Clamp, 0.28" to 0.49" Machinable Range, Haas Workholding by Mitee-Bite | 29.7 × 30.5 × 29.7 | 5 / 469 | ok | 1983 |  |
| 05-0746 | Size #1 ID Xpansion Clamp, 0.48" to 0.56" Machinable Range, Haas Workholding by Mitee-Bite | 31.5 × 38.1 × 31.5 | 5 / 441 | ok | 1666 |  |
| 05-0747 | Size #2 ID Xpansion Clamp, 0.53" to 0.79" Machinable Range, Haas Workholding by Mitee-Bite | 37.4 × 40.3 × 37.5 | 5 / 458 | ok | 1648 |  |
| 05-0749 | Size #4 ID Xpansion Clamp, 0.9" to 1.39" Machinable Range, Haas Workholding by Mitee-Bite | 55.9 × 51.0 × 56.0 | 5 / 598 | ok | 656 |  |
| 05-0750 | Size #5 ID Xpansion Clamp, 1.15" to 1.65" Machinable Range, Haas Workholding by Mitee-Bite | 69.4 × 66.7 × 69.5 | 5 / 426 | ok | 1504 |  |
| 05-0751 | Size #6 ID Xpansion Clamp, 1.15" to 2.03" Machinable Range, Haas Workholding by Mitee-Bite | 75.5 × 60.3 × 75.5 | 5 / 414 | ok | 1425 |  |
| 05-0752 | Size #7 ID Xpansion Clamp, 1.15" to 3.06" Machinable Range, Haas Workholding by Mitee-Bite | 107.5 × 76.1 × 107.5 | 6 / 478 | ok | 1530 |  |
| 05-0754 | Size #9 ID Xpansion Clamp, 1.15" to 6.89" Machinable Range, Haas Workholding by Mitee-Bite | 175.0 × 76.1 × 175.0 | 8 / 614 | ok | 1951 |  |
| 05-0755 | Size #10 ID Xpansion Clamp, 1.15" to 9.85" Machinable Range, Haas Workholding by Mitee-Bite | 250.2 × 76.1 × 250.2 | 8 / 622 | ok | 2167 |  |
| 05-0756 | Size #00 ID Xpansion Clamp, 4.1mm to 7.4mm Machinable Range, Haas Workholding by Mitee-Bite | 20.0 × 14.7 × 20.0 | 4 / 354 | ok | 1383 |  |
| 05-0757 | Size #0 ID Xpansion Clamp, 7.1mm to 12.4mm Machinable Range, Haas Workholding by Mitee-Bite | 29.7 × 29.0 × 29.7 | 5 / 519 | ok | 2285 |  |
| 05-0759 | Size #2 ID Xpansion Clamp, 13.5mm to 20mm Machinable Range, Haas Workholding by Mitee-Bite | 37.4 × 38.7 × 37.5 | 5 / 502 | ok | 1952 |  |
| 05-0760 | Size #3 ID Xpansion Clamp, 18mm to 27mm Machinable Range, Haas Workholding by Mitee-Bite | 49.9 × 45.8 × 50.0 | 6 / 482 | ok | 1884 |  |
| 05-0762 | Size #5 ID Xpansion Clamp, 29.3mm to 42mm Machinable Range, Haas Workholding by Mitee-Bite | 69.4 × 61.0 × 69.5 | 5 / 533 | ok | 2280 |  |
| 05-0763 | Size #6 ID Xpansion Clamp, 29.3mm to 51.5mm Machinable Range, Haas Workholding by Mitee-Bite | 75.5 × 61.0 × 75.5 | 5 / 533 | ok | 2326 |  |
| 05-0764 | Size #7 ID Xpansion Clamp, 29.3mm to 77.7mm Machinable Range, Haas Workholding by Mitee-Bite | 107.5 × 125.5 × 107.5 | 6 / 536 | ok | 1919 |  |
| 05-0765 | Size #8 ID Xpansion Clamp, 29.3mm to 103mm Machinable Range, Haas Workholding by Mitee-Bite | 132.8 × 75.5 × 132.9 | 6 / 541 | ok | 2105 |  |
| 05-0766 | Size #9 ID Xpansion Clamp, 29.3mm to 175mm Machinable Range, Haas Workholding by Mitee-Bite | 175.0 × 75.5 × 175.0 | 8 / 694 | ok | 2553 |  |
| 05-0767 | Size #10 ID Xpansion Clamp, 29.3mm to 250.2mm Machinable Range, Haas Workholding by Mitee-Bite | 250.2 × 56.7 × 250.2 | 7 / 635 | ok | 1764 |  |

## Mill Workholding > Specialty Fixture Clamps & Rails > Low Profile Spring Clamps & Rails

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0294 | Low Profile Spring Clamp, Flat Edge, Small - Pack of 2 | 34.0 × 30.9 × 17.5 | 11 / 288 | ok | 4999 |  |
| 05-0295 | Low Profile Spring Clamp, Flat Edge, Large | 46.0 × 40.9 × 26.0 | 11 / 296 | ok | 4396 |  |
| 05-0297 | Low Profile Spring Clamp, Serrated Edge, Small - Pack of 2 | 35.3 × 30.9 × 17.5 | 11 / 784 | ok | 4547 |  |
| 05-0298 | Low Profile Spring Clamp, Serrated Edge, Large | 47.3 × 40.9 × 26.0 | 7 / 1328 | ok | 4675 |  |
| 05-0340 | Low Profile Spring Clamp Spacer, Hole Edge, Small 39.5x12mm, Square Block - Pack of 4 | 39.5 × 15.0 × 12.0 | 1 / 32 | ok | 2 |  |
| 05-0341 | Low Profile Spring Clamp Spacer, Hole Edge, Large 30x15mm, Square Block - Pack of 2 | 30.0 × 22.0 × 15.0 | 1 / 22 | ok | 3 |  |
| 05-0342 | Low Profile Spring Clamp Spacer, Hole Edge, Large 49.5x15mm, Square Block - Pack of 2 | 49.5 × 22.0 × 15.0 | 1 / 22 | ok | 4 |  |
| 05-0343 | Low Profile Spring Clamp Spacer, Hole Edge, Large 49.5x25mm, V-Block - Pack of 2 | 25.0 × 49.5 × 22.0 | 1 / 26 | ok | 4 |  |
| 05-0416 | Low Profile Spring Clamp Rail Fixture, 500mm | 500.0 × 50.0 × 50.0 | 3 / 2606 | ok | 796 |  |
| 05-0417 | Low Profile Spring Clamp Rail Fixture, 600mm | 600.0 × 50.0 × 50.0 | 3 / 3093 | ok | 956 |  |
| 05-0550 | Low Profile Spring Clamp Rail Stop, 35mm | 35.0 × 50.0 × 47.3 | 3 / 323 | ok | 23 |  |
| 05-0551 | Low Profile Spring Clamp Rail Stop, 50mm | 60.0 × 50.0 × 27.7 | 1 / 373 | ok | 27 |  |
| 05-0553 | Low Profile Spring Clamp Rail Base Plate | 46.0 × 50.0 × 9.0 | 1 / 28 | ok | 2 |  |
| 05-0554 | Low Profile Spring Clamp Rail T-Nut | 24.0 × 25.0 × 17.0 | 1 / 32 | ok | 2 |  |

## Mill Workholding > Specialty Fixture Clamps & Rails > Pitbull® Clamps by Mitee-Bite

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0418 | 1/2" Knife Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 8 | 16.4 × 10.9 × 23.2 | 3 / 174 | ok | 818 |  |
| 05-0419 | 1/2" Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 8 | 7.4 × 6.9 × 20.5 | 3 / 171 | ok | 696 |  |
| 05-0420 | 3/4" Knife Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 19.1 × 25.4 × 19.2 | 3 / 116 | ok | 493 |  |
| 05-0421 | 3/4" Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 19.1 × 25.5 × 19.1 | 3 / 115 | ok | 495 |  |
| 05-0422 | 1" Knife Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 4 | 34.9 × 24.5 × 47.6 | 3 / 187 | ok | 872 |  |
| 05-0423 | 1" Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 4 | 16.0 × 14.3 × 42.9 | 3 / 176 | ok | 843 |  |
| 05-0424 | 1" Machinable Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 4 | 16.0 × 19.0 × 42.9 | 4 / 188 | ok | 824 |  |
| 05-0425 | 12.7mm Knife Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 8 | 16.4 × 11.0 × 22.4 | 3 / 178 | ok | 784 |  |
| 05-0426 | 12.7mm Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 8 | 7.4 × 7.0 × 19.7 | 3 / 175 | ok | 722 |  |
| 05-0427 | 19.05mm Knife Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 19.1 × 15.9 × 27.6 | 3 / 181 | ok | 788 |  |
| 05-0428 | 19.05mm Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 11.2 × 10.0 × 27.6 | 3 / 187 | ok | 782 |  |
| 05-0429 | 25.4mm Knife Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 4 | 34.9 × 24.5 × 47.6 | 3 / 187 | ok | 935 |  |
| 05-0430 | 25.4mm Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 4 | 16.0 × 14.3 × 42.9 | 3 / 175 | ok | 858 |  |
| 05-0431 | 25.4mm Machinable Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 4 | 16.0 × 14.3 × 42.9 | 3 / 180 | ok | 809 |  |
| 05-0942 | 19.05mm Brass Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 19.1 × 16.2 × 25.6 | 3 / 135 | ok | 589 |  |
| 05-0943 | 12.7mm Brass Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 8 | 11.2 × 10.0 × 27.6 | 3 / 170 | ok | 536 |  |
| 05-0944 | 9.52mm Brass Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 8 | 12.2 × 7.7 × 14.8 | 3 / 139 | ok | 874 |  |
| 05-0945 | 3/4" Brass Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 24.6 × 11.1 × 28.0 | 3 / 99 | ok | 301 |  |
| 05-0946 | 1/2" Brass Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 8 | 16.7 × 7.9 × 18.6 | 3 / 101 | ok | 305 |  |
| 05-0947 | 3/8" Brass Blunt Edge Pitbull Clamp, Haas Workholding by Mitee-Bite - Pack of 8 | 12.2 × 5.6 × 13.7 | 3 / 99 | ok | 300 |  |

## Mill Workholding > Specialty Fixture Clamps & Rails > TalonGrip™ Clamps by Mitee-Bite

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0790 | 1/2" TalonGrip Jaw, by Mitee-Bite  - Pack of 2 | 19.1 × 12.7 × 9.1 | 2 / 101 | ok | 237 |  |
| 05-0791 | 1/2" TalonGrip Stop, by Mitee-Bite | 19.1 × 12.7 × 7.9 | 2 / 74 | ok | 218 |  |
| 05-0792 | 3/4" TalonGrip Jaw, by Mitee-Bite - Pack of 2 | 19.1 × 9.9 × 19.0 | 2 / 137 | ok | 326 |  |
| 05-0793 | 1" TalonGrip Jaw, by Mitee-Bite - Pack of 2 | 19.1 × 10.1 × 25.4 | 2 / 129 | ok | 286 |  |
| 05-0794 | 1" TalonGrip Jaw XL, by Mitee-Bite | 25.4 × 18.7 × 39.7 | 2 / 151 | ok | 367 |  |

## Mill Workholding > Specialty Fixture Clamps & Rails > Uniforce® Clamps by Mitee-Bite

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0432 | Model 500, 1/2" Machinable Uniforce Clamp, Locking Plate, Haas Workholding by Mitee-Bite | 28.6 × 21.4 × 28.0 | 5 / 308 | ok | 579 |  |
| 05-0433 | Model 500, 1/2" Machinable Uniforce Clamp, Haas Workholding by Mitee-Bite | 28.6 × 21.4 × 28.0 | 4 / 292 | ok | 660 |  |
| 05-0435 | Model 750, 3/4" Machinable Uniforce Clamp, Haas Workholding by Mitee-Bite | 38.1 × 30.9 × 40.5 | 4 / 268 | ok | 510 |  |
| 05-0436 | Model 1000, 1" Machinable Uniforce Clamp, Locking Plate, Haas Workholding by Mitee-Bite | 50.8 × 31.7 × 49.2 | 5 / 293 | ok | 535 |  |
| 05-0437 | Model 250, 1/4" Uniforce Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 14.8 × 8.1 × 8.1 | 3 / 82 | ok | 49 |  |
| 05-0438 | Model 375, 3/8" Uniforce Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 9.2 × 11.8 × 15.5 | 3 / 164 | ok | 508 |  |
| 05-0439 | Model 500, 1/2" Uniforce Clamp, Haas Workholding by Mitee-Bite - Pack of 8 | 12.2 × 16.1 × 28.0 | 3 / 164 | ok | 518 |  |
| 05-0440 | Model 750, 3/4" Uniforce Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 18.7 × 23.8 × 40.5 | 4 / 251 | ok | 919 |  |
| 05-0441 | Model 1000, 1" Uniforce Clamp, Haas Workholding by Mitee-Bite - Pack of 4 | 24.9 × 31.3 × 49.2 | 3 / 159 | ok | 466 |  |
| 05-0442 | Model 1000, 1" Machinable Uniforce Clamp, Haas Workholding by Mitee-Bite | 50.8 × 31.3 × 49.2 | 4 / 277 | ok | 554 |  |
| 05-0443 | Model 500, 12.7mm Machinable Uniforce Clamp, Locking Plate, Haas Workholding by Mitee-Bite | 28.6 × 21.4 × 27.9 | 5 / 270 | ok | 897 |  |
| 05-0444 | Model 500, 12.7mm Machinable Uniforce Clamp, Haas Workholding by Mitee-Bite | 28.6 × 161.7 × 15.9 | 3 / 237 | ok | 18 |  |
| 05-0445 | Model 750, 19.1mm Machinable Uniforce Clamp, Locking Plate, Haas Workholding by Mitee-Bite | 38.1 × 30.9 × 40.5 | 5 / 218 | ok | 640 |  |
| 05-0446 | Model 750, 19.1mm Machinable Uniforce Clamp, Haas Workholding by Mitee-Bite | 38.1 × 168.0 × 23.9 | 3 / 212 | ok | 16 |  |
| 05-0447 | Model 1000, 25.4mm Machinable Uniforce Clamp, Locking Plate, Haas Workholding by Mitee-Bite | 50.8 × 31.9 × 53.9 | 5 / 251 | ok | 855 |  |
| 05-0448 | Model 1000, 25.4mm Machinable Uniforce Clamp, Haas Workholding by Mitee-Bite | 50.8 × 174.4 × 31.8 | 3 / 240 | ok | 20 |  |
| 05-0449 | Model 250, 6.4mm Unfiorce Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 14.8 × 8.1 × 8.1 | 3 / 82 | ok | 46 |  |
| 05-0450 | Model 375, 9.5mm Uniforce Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 20.1 × 12.0 × 11.9 | 3 / 82 | ok | 42 |  |
| 05-0451 | Model 500, 12.7mm Uniforce Clamp, Haas Workholding by Mitee-Bite - Pack of 8 | 26.2 × 16.1 × 15.9 | 3 / 74 | ok | 5 |  |
| 05-0452 | Model 750, 19.05mm Uniforce Clamp, Haas Workholding by Mitee-Bite - Pack of 6 | 18.7 × 23.8 × 40.5 | 3 / 153 | ok | 445 |  |

## Mill Workholding > Specialty Fixture Clamps & Rails > VersaGrip™ Clamps by Mitee-Bite

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0795 | 3/4" VersaGrip Jaw, by Mitee-Bite - Pack of 2 | 19.1 × 19.1 × 17.5 | 2 / 203 | ok | 947 |  |
| 05-0796 | 3/4" VersaGrip Jaw Heavy Duty, by Mitee-Bite - Pack of 2 | 19.0 × 15.9 × 27.1 | 2 / 1393 | ok | 829 |  |

## Mill Workholding > Vises > Dovetail Fixtures

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0780 | 3/4" Dovetail Fixture for 160/210mm Rotaries | 51.9 × 50.8 × 96.3 | 12 / 865 | ok | 3323 |  |
| 05-0823 | 3/8" Dovetail Fixture for 70mm Rotaries | 70.5 × 70.5 × 51.0 | 7 / 434 | ok | 1039 |  |
| 05-0824 | 3/4" Dovetail Fixture for 100mm Rotaries | 100.5 × 100.5 × 54.0 | 12 / 747 | ok | 2492 |  |
| 05-0825 | 1-1/2" Dovetail Fixture for 160/210mm Rotaries | 61.9 × 71.0 × 96.3 | 16 / 1085 | ok | 4180 |  |
| 05-0826 | 100mm Rotary Adapter Plate for 3/8" Dovetail Fixture | 76.2 × 12.7 × 76.2 | 1 / 50 | ok | 8 |  |
| 05-0827 | 160/210mm Rotary Adapter Plate for 3/4" Dovetail Fixture | 167.4 × 25.4 × 167.5 | 1 / 62 | ok | 6 |  |
| 05-0828 | 310mm Rotary Adapter Plate for 3/4" and 1-1/2" Dovetail Fixtures | 209.9 × 25.4 × 210.0 | 1 / 62 | ok | 6 |  |
| 05-0829 | 500mm T-Slot Adapter Plate for 3/4" and 1-1/2" Dovetail Fixtures | 279.3 × 76.2 × 279.4 | 1 / 129 | ok | 12 |  |
| 08-2178 | 3/8" Dovetail Fixture and Adapter Kit for 100mm Rotaries | 123.3 × 248.0 × 150.3 | 10 / 961 | ok | 1487 |  |
| 08-2179 | 3/4" Dovetail Fixture and Adapter Kit for 160/210mm Rotaries | 184.0 × 100.5 × 198.9 | 14 / 1210 | ok | 2478 |  |
| 08-2180 | 3/4" Dovetail Fixture and Adapter Kit for 310mm Rotaries | 209.9 × 283.4 × 258.9 | 14 / 1490 | ok | 3642 |  |
| 08-2181 | 3/4" Dovetail Fixture and Adapter Kit for UMC-500 | 279.3 × 283.4 × 293.6 | 14 / 1557 | ok | 3524 |  |
| 08-2182 | 1-1/2" Dovetail Fixture and Adapter Kit for 310mm Rotaries | 209.9 × 543.5 × 258.9 | 18 / 1710 | ok | 4305 |  |
| 08-2183 | 1-1/2" Dovetail Fixture and Adapter Kit for UMC-500 | 279.3 × 543.5 × 293.6 | 18 / 1777 | ok | 4416 |  |

## Mill Workholding > Vises > Fixed-Jaw Vises

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0181 | 6" Double-Station HD™ CarvLock® Fixed-Jaw Vise, 4" Opening, Haas Workholding by Kurt | 570.2 × 123.4 × 152.4 | 20 / 841 | ok | 260 |  |
| 05-0182 | 4" Double-Station HD™ CarvLock® Fixed-Jaw Vise, 3" Opening, Haas Workholding by Kurt | 275.4 × 32.3 × 100.8 | 8 / 1539 | partial BREP_WITH_VOIDS 2 | 1068 |  |
| 05-0209 | 100mm Single-Station CrossOver® Fixed-Jaw Vise, 165mm Opening, Flange Style, Haas Workholding by Kurt | 345.0 × 88.5 × 126.0 | 20 / 1163 | ok | 184 |  |
| 05-0210 | 150mm Double-Station HD™ CarvLock® Fixed-Jaw Vise, 100mm Opening, Haas Workholding by Kurt | 570.2 × 123.4 × 152.4 | 20 / 841 | ok | 260 | 05-0181 |
| 05-0211 | 100mm Double-Station HD™ CarvLock® Fixed-Jaw Vise, 75mm Opening, Haas Workholding by Kurt | 275.4 × 32.3 × 100.8 | 8 / 1539 | partial BREP_WITH_VOIDS 2 | 1068 | 05-0182 |
| 05-0212 | 200mm Single-Station CrossOver® Fixed-Jaw Vise, 265mm Opening, Flange Style, Haas Workholding by Kurt | 609.6 × 119.5 × 202.4 | 39 / 2845 | ok | 3055 |  |
| 05-0213 | 150mm Single-Station CrossOver® Fixed Jaw Vise, 225mm Opening, Flange Style, Haas Workholding by Kurt | 466.3 × 117.1 × 187.8 | 17 / 960 | ok | 320 |  |
| 05-0404 | 6" Single-Station CrossOver® Fixed-Jaw Vise, 9" Opening, Flange Style, Haas Workholding by Kurt | 466.3 × 117.1 × 187.8 | 17 / 960 | ok | 320 | 05-0213 |
| 05-0405 | 4" Single-Station CrossOver® Fixed-Jaw Vise, 6-1/2" Opening, Flange Style, Haas Workholding by Kurt | 345.0 × 88.5 × 126.0 | 20 / 1163 | ok | 184 | 05-0209 |
| 05-0406 | 8" Single-Station CrossOver® Fixed-Jaw Vise, 10-1/2" Opening, Flange Style, Haas Workholding by Kurt | 609.6 × 119.5 × 202.4 | 39 / 2845 | ok | 3055 | 05-0212 |
| 05-0957 | 6" Double-Station TriLock® CarveSmart® Fixed-Jaw Vise, 5" Opening, Haas Workholding by Kurt | 572.5 × 130.2 × 152.4 | 15 / 2474 | ok | 2660 |  |
| 05-0958 | 6" Double-Station Short TriLock® CarveSmart® Fixed-Jaw Vise, 3-3/4" Opening, Haas Workholding by Kurt | 481.6 × 130.2 × 152.7 | 15 / 2449 | ok | 4327 |  |
| 05-0959 | 6" Double-Station Hydraulic TriLock® CarveSmart® Fixed-Jaw Vise, 5" Opening, Haas Workholding by Kurt | 558.1 × 106.1 × 152.4 | 15 / 3570 | partial BREP_WITH_VOIDS 1 | 2135 |  |
| 05-0960 | 4" Double-Station TriLock® CarveSmart® Fixed-Jaw Vise, 3-3/4" Opening, Haas Workholding by Kurt | 450.9 × 111.2 × 101.6 | 15 / 2510 | ok | 1630 |  |
| 05-0961 | 4" Double-Station Short TriLock® CarveSmart® Fixed-Jaw Vise, 3" Opening, Haas Workholding by Kurt | 385.9 × 111.2 × 101.6 | 15 / 2508 | ok | 2142 |  |
| 05-0984 | 6" Single-Station CrossOver® Reverse Fixed-Jaw Vise, 9" Opening, Flange Style, by Kurt | 430.7 × 103.3 × 151.6 | 25 / 1322 | ok | 1958 |  |
| 05-0985 | 8" Single-Station CrossOver® Reverse Fixed-Jaw Vise, 10-1/2" Opening, Flange Style, by Kurt | 588.2 × 116.0 × 202.4 | 19 / 2405 | ok | 4472 |  |
| 05-0986 | 6" Single-Station AngLock® CarveSmart® Fixed-Jaw Vise, 7-3/4" Vise Opening, Flange Style, by Kurt | 465.3 × 123.4 × 186.3 | 22 / 927 | ok | 158 |  |
| 05-0989 | 6" Single-Station Extra Capacity XL™ XtraLong® Fixed-Jaw Vise, 14-1/2" Vise Opening, by Kurt | 604.4 × 123.4 × 152.4 | 23 / 1874 | ok | 2520 |  |
| 05-0990 | 8" Single-Station Extra Capacity XL™ XtraLong® Fixed-Jaw Vise, 21-1/2" Vise Opening, by Kurt | 874.7 × 151.1 × 204.7 | 29 / 2069 | ok | 1296 |  |

## Mill Workholding > Vises > Self-Centering Vises

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0004 | 5" (130mm) Self-Centering Vise, 5" (130mm) Opening | 279.4 × 279.3 × 163.0 | 6 / 1685 | ok | 2819 |  |
| 05-0013 | 3" (75mm) Self-Centering Vise, 2-1/2" (64mm) Opening | 116.5 × 105.0 × 200.0 | 10 / 3328 | ok | 4521 |  |
| 05-0407 | 75mm (3") Pneumatic Self-Centering Vise, 35mm (1-1/2") Opening, Haas TRT210 / UMC-350HD | 100.0 × 145.5 × 124.5 | 13 / 2753 | partial MANIFOLD_SOLID_BREP 1 | 2035 |  |
| 05-0547 | 6" (150mm) Hydraulic Self-Centering Vise, 60mm (2-1/2") Opening, Haas UMC 500/750/1000 | 270.0 × 270.0 × 165.0 | 8 / 464 | ok | 44 |  |
| 05-0548 | 8" (200mm) Hydraulic Self-Centering Vise, 110mm (4-1/2") Opening, Haas UMC 500/750/1000 | 270.0 × 270.0 × 165.0 | 8 / 534 | ok | 50 |  |
| 05-0549 | 10" (250mm) Hydraulic Self-Centering Vise, 150mm (6") Opening, Haas UMC 500/750/1000 | 314.9 × 315.0 × 165.0 | 8 / 562 | ok | 53 |  |
| 05-0773 | 5" (130mm) Self-Centering Vise, 3-1/2" (90mm) Opening | 172.0 × 71.0 × 170.0 | 12 / 2881 | ok | 4410 |  |
| 05-0993 | 4" (101.6mm) MaxLock™ AngLock® Self-Centering Vise, 2" (50.8mm) Opening, Machinable Vise Jaws, by Kurt | 149.2 × 74.7 × 101.6 | 22 / 2707 | ok | 1602 |  |
| 05-0994 | 4" (101.6mm) MaxLock™ AngLock® Self-Centering Vise, 2-1/8" (54mm) Opening, Dovetail Vise Jaws, by Kurt |  | 0 / None | none |  |  |
| 05-0995 | 4" (101.6mm) MaxLock™ AngLock® Self-Centering Vise, 2-1/8" (54mm) Opening, Serrated Vise Jaws, by Kurt |  | 0 / None | none |  |  |
| 05-0996 | 4" (101.6mm) MaxLock™ AngLock® Self-Centering Vise, 4" (101.6mm) Opening, Machinable Vise Jaws, by Kurt | 200.0 × 74.7 × 101.6 | 22 / 2701 | ok | 1718 |  |
| 05-0998 | 4" (101.6mm) MaxLock™ AngLock® Self-Centering Vise, 4-1/8" (104.8mm) Opening, Serrated Vise Jaws, by Kurt | 200.0 × 74.2 × 101.6 | 22 / 3221 | ok | 1955 |  |

## Mill Workholding > Vises > Toolmakers' Vises

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 09-0568 | Toolmaker's Vise, Screwless, 50mm Jaw Width, 80mm Opening | 54.5 × 103.6 × 150.0 | 2 / 430 | ok | 510 |  |
| 09-0569 | Toolmaker's Vise, Screw Guide, 50mm Jaw Width, 63mm Opening | 48.0 × 60.2 × 220.0 | 18 / 1285 | ok | 3044 |  |

## Tombstones > Tombstones & Towers

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0179 | 400mm Cast Iron Square Plain Tombstone | 400.0 × 400.0 × 586.5 | 1 / 215 | ok | 14 |  |
| 05-0220 | 400mm Aluminum Square Plain Tombstone | 400.0 × 400.0 × 586.5 | 1 / 199 | ok | 32 |  |
| 05-0301 | 400mm Cast Iron T Plain Tombstone | 400.0 × 586.5 × 400.0 | 1 / 146 | ok | 14 |  |
| 05-0302 | 400mm Aluminum T Plain Tombstone | 400.0 × 586.5 × 400.0 | 1 / 146 | ok | 15 |  |
| 05-0303 | 400mm Cast Iron Cross Plain Tombstone | 400.0 × 586.5 × 400.0 | 1 / 203 | ok | 30 |  |
| 05-0304 | 400mm Aluminum Cross Plain Tombstone | 400.0 × 586.5 × 400.0 | 1 / 203 | ok | 31 |  |
| 05-0305 | 400mm Cast Iron Hex Plain Tombstone | 400.0 × 586.5 × 400.0 | 1 / 150 | ok | 15 |  |
| 05-0306 | 400mm Aluminum Hex Plain Tombstone | 400.0 × 586.5 × 400.0 | 1 / 150 | ok | 15 |  |
| 05-0307 | 500mm Cast Iron Square Plain Tombstone | 500.0 × 600.0 × 500.0 | 1 / 234 | ok | 24 |  |
| 05-0309 | 500mm Cast Iron T Plain Tombstone | 500.0 × 600.0 × 500.0 | 1 / 206 | ok | 26 |  |
| 05-0311 | 500mm Cast Iron Cross Plain Tombstone | 500.0 × 600.0 × 500.0 | 1 / 249 | ok | 30 |  |
| 05-0313 | 500mm Cast Iron Hex Plain Tombstone | 500.0 × 600.0 × 500.0 | 1 / 164 | ok | 19 |  |
| 05-0315 | 630mm Cast Iron Square Plain Tombstone | 630.0 × 850.0 × 630.0 | 1 / 258 | ok | 28 |  |
| 05-0317 | 630mm Cast Iron T Plain Tombstone | 630.0 × 850.0 × 630.0 | 1 / 198 | ok | 18 |  |
| 05-0319 | 630mm Cast Iron Cross Plain Tombstone | 630.0 × 850.0 × 630.0 | 1 / 195 | ok | 25 |  |
| 05-0321 | 630mm Cast Iron Hex Plain Tombstone | 630.0 × 850.0 × 630.0 | 1 / 142 | ok | 16 |  |
| 05-0814 | 800mm Cast Iron Square Plain Tombstone | 800.0 × 1248.0 × 800.0 | 4 / 413 | ok | 478 |  |
| 05-0815 | 800mm Cast Iron T Plain Tombstone | 800.0 × 1148.0 × 800.0 | 4 / 323 | ok | 462 |  |
| 05-0816 | 800mm Cast Iron Cross Plain Tombstone | 800.0 × 1048.0 × 800.0 | 4 / 366 | ok | 466 |  |
| 05-0817 | 800mm Cast Iron Hex Plain Tombstone | 800.0 × 1048.0 × 800.0 | 4 / 297 | ok | 456 |  |

## Tombstones > Vise Cluster Towers

| part | name | size x × y × z mm | solids / faces | import | ms | same file as |
|---|---|---|---|---|---|---|
| 05-0774 | 4" (100mm) TriLock® CarveSmart® Vise Cluster Tower, 400mm Base Plate, Haas Workholding by Kurt | 400.0 × 479.4 × 400.0 | 141 / 6345 | ok | 1821 |  |
| 05-0775 | 4" (100mm) TriLock® CarveSmart® Vise Cluster Tower, 500mm Base Plate, Haas Workholding by Kurt | 500.0 × 479.4 × 500.0 | 141 / 6345 | ok | 1799 |  |
| 05-0776 | 6" (150mm) TriLock® CarveSmart® Vise Cluster Tower, 400mm Base Plate, Haas Workholding by Kurt | 400.0 × 600.5 × 400.0 | 136 / 6834 | ok | 1529 |  |
| 05-0777 | 6" (150mm) TriLock® CarveSmart® Vise Cluster Tower, 500mm Base Plate, Haas Workholding by Kurt | 500.0 × 600.5 × 500.0 | 136 / 6836 | ok | 1412 |  |
| 05-0778 | 6" (150mm) TriLock® CarveSmart® Vise Cluster Tower, 630mm Base Plate, Haas Workholding by Kurt | 630.0 × 606.9 × 630.0 | 136 / 6824 | ok | 1427 |  |
