# Press-brake machine specifications (openly published sources)

Collected 2026-11-24 for the bend simulator's machine model (side frames, ram/upper beam, bed,
backgauge). Every value carries: status | source URL | date read | the source's own statement with its
units. Status is `published` (read in the source), `estimate (basis)` (derived or interpreted by us),
`unknown`, or `conflict (note)` (sources disagree; the value given is the cited source's, the note
gives the other). Units are kept as the source states them (kN, metric t, US tons, mm, in, ipm).
Where a maker writes `a / b`, the second value is an option (enlarged / increased open height)
as that source's footnote says.

Field meanings: `open_height` = the source's open height / daylight / distance table-ram / Q size
(the source's own term is quoted in the statement; Trumpf also gives `table_to_beam`, the raw
beam-to-table distance, and a smaller "usable open height"). `table_height` = working height / bed
height above floor. `bg_x_range` = the largest stop distance or range, `bg_x_travel` = axis travel.

No sign-ups, no user-agent changes, no bot-challenge workarounds were used. Downloaded files were
only read as text or rendered to images to read rotated tables.

## 1. Sources

| key | URL | what | read | result |
|---|---|---|---|---|
| TB | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | TRUMPF bending machines brochure, 60 pp, print code 202402; technical data pp. 50-55 (TruBend 1000/3000/5000/7000/8000) | 2026-11-24 | fetched (PDF). Same path on host `Trumpf.com` (no www) returned the home page instead |
| TW5 | https://www.trumpf.com/en_GB/products/machines-systems/bending-machines/trubend-series-5000/ | TruBend 5000 product page, model table | 2026-11-24 | fetched (WebFetch). "Technical data sheet" download link not present in page content |
| TW7 | https://www.trumpf.com/en_GB/products/machines-systems/bending-machines/trubend-series-7000/ | TruBend 7000 product page, model table | 2026-11-24 | fetched |
| TW3 | https://www.trumpf.com/en_GB/products/machines-systems/bending-machines/trubend-series-3000/ | TruBend 3000 product page (newer generation 3100/3130/3170/3230) | 2026-11-24 | fetched, table only partly readable (cells misaligned); used only for conflicts |
| - | https://www.amada.eu/... (HG, HG-ATC, HRB pages), https://cdn.amada.eu/fileadmin/Repository/Products/Brochures/HG_series_8_pages_2021_E050-EU07_EN.pdf, HG-ATC_brochure_2021_E056-EU03_EN.pdf, HRB_F155-EU03en_brochure_2023_01.pdf | Amada Europe pages and brochures | 2026-11-24 | REFUSED: HTTP 403, Cloudflare "Sorry, you have been blocked" (curl and WebFetch). Not retried |
| - | https://www.amada.com/america/hg-series/ and https://www.amada.com/media/digital-b/HG_Series/html5/ | Amada America HG page; specs only inside a FlipViewer e-book | 2026-11-24 | page fetched (series range only: 56 to 247 tons, 55 to 169.3 in); e-book content is obfuscated/keyed, not read |
| AJ | https://products.amada.co.jp/products/product/?productid=id000055&language=2 | Amada Japan HG Series product page, spec table | 2026-11-24 | fetched |
| AJE | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | HG series catalogue E050-HQ02en Nov.2019, p.8 (dimensions, machine spec, AMNC 3i spec), page image served to the public e-book viewer | 2026-11-24 | fetched, read from image |
| AJP7 | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page007.jpg | same catalogue p.7 (tool holders, backgauge fingers) | 2026-11-24 | fetched |
| AJA | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | HG-1003ATC catalogue F007-HQ02en May 2016, p.4 | 2026-11-24 | fetched, read from image |
| BX | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | Bystronic Xpert Pro Technical Data (4 pp) | 2026-11-24 | fetched (en_US version also fetched, same layout) |
| BS | https://www.bystronic.com/che/en/b/bending-machine-bybend-star-40-80 | ByBend Star 40/80 product page | 2026-11-24 | fetched by curl (WebFetch failed: header overflow); only the 80 t table present |
| - | https://pdf.directindustry.com/pdf/bystronic-laser/xact-smart-technical-data/15383-980948.html | Xact Smart technical data on aggregator | 2026-11-24 | REFUSED: HTTP 403 |
| LE | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | LVD Easy-Form page, spec tables | 2026-11-24 | fetched |
| LEP | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | Easy-Form brochure (PDF created 2024-01) | 2026-11-24 | fetched |
| LP | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | LVD PPEB page, spec tables (PPEB_2021_EN.pdf also fetched) | 2026-11-24 | fetched |
| LD | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | LVD Dyna-Press page (Dyna-Press_2022_EN.pdf also fetched, same values) | 2026-11-24 | fetched |
| CG | https://www.e-ci.com/goform-electric-press-brakes | Cincinnati GOFORM page, spec table | 2026-11-24 | fetched |
| CX | https://www.e-ci.com/xform-series-press-brake | Cincinnati XFORM page, spec tables (XF and XFe) | 2026-11-24 | fetched |
| CI | https://www.e-ci.com/xi-series-press-brake | Cincinnati Xi page | 2026-11-24 | fetched |
| AC | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | Accurpress Accell brochure (24 pp, PDF dated 2020-09), hosted on a dealer site (machineryhost); restates Accurpress's own brochure | 2026-11-24 | fetched. accurpress.com is reachable (HTTP 200) but was not searched for a newer sheet |
| BM | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | Baileigh BP-3305CNC operator manual 08/2019, Technical Specifications | 2026-11-24 | fetched |
| BW | https://www.wurthmachinery.com/skin/common_files/files/specs/JPWBA9-1020680-SpecSheet.pdf | dealer (Wurth Machinery) spec sheet for BP-3305CNC (JPW item BA9-1020680), restating Baileigh/JPW | 2026-11-24 | fetched; used for conflicts |
| EP | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | Ermaksan Power-Bend Pro brochure (PDF created 2014-10), hosted by dealer Prestige Equipment; Ermaksan's own document | 2026-11-24 | fetched; table read from rendered page 14 |
| DU | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | Durma AD-R catalogue (PDF created 2018-07, Turkish/English), hosted by US dealer Productivity Inc.; Durma's own document | 2026-11-24 | fetched; table read from rendered page 9 |
| SU | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | SafanDarley E-Brake 35T-130T Premium brochure 10-2018 US | 2026-11-24 | fetched; table read from rendered page 19 |
| SI | https://cdn.shopify.com/s/files/1/0638/7833/2592/files/E-Brake35-130T-E092013_INT.pdf | SafanDarley E-Brake 35-130T brochure 09-2013 INT (metric, older generation) | 2026-11-24 | fetched; used for metric values and conflicts |
| - | https://www.machinetools.com/..., https://india.machinetools.com/... (Amada spec sheets) | used-machine listings | 2026-11-24 | REFUSED: HTTP 403 |
| - | http://www.sheetworks.info/bending/hg/specifications.html | Amada GmbH reseller page for HG | 2026-11-24 | not reachable: host outside this session's allowed network |
| - | https://prestigeequipment.com/Brakes-Press-Hydraulic/Amada-HG1303/32344/i, https://machinehub.com/listings/434986-... | dealer listings for Amada HG-1303 | 2026-11-24 | fetched, item not found / no spec values |

## 2. Machines

32 machines: Trumpf 7, Amada 4, Bystronic 4, LVD 4, Cincinnati 4, Accurpress 2, Ermaksan 2,
Durma 2, SafanDarley 2, Baileigh 1. Range 33 US ton (Baileigh) to 3200 kN / 400 t; electric
(TruBend 7000, GOFORM, Dyna-Press, E-Brake), hybrid (Amada HG), hydraulic (rest).

```toml
[[press]]
id = "trumpf-trubend-5085"
maker = "Trumpf"
model = "TruBend 5085"
tonnage = "850 kN"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "TRUMPF bending machines brochure 202402, Technical data p.53: Press force kN 850"
drive = "hydraulic with On-Demand Servo Drive"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "The On-Demand Servo Drive thus saves up to 42% in energy compared to traditional hydraulics (TruBend Series 5000 page)"
bending_length = "2210 / 2720 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Bending length mm 2210 / 2720"
housing_gap = "1750 / 2260 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Width between columns mm 1750 / 2260"
open_height = "385 / 615 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Usable open height mm 385 / 615 ([1] second value = enlarged design, option)"
table_to_beam = "505 / 735 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. table/press beam distance mm 505 / 735"
stroke = "215 / 445 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis stroke mm 215 / 445"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "420 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Throat depth mm 420 (web page TW5 also 420 mm)"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "1095-1115 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Working height mm 1095-1115, [2] with lower tool height of 100 mm"
speed_approach = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis rapid traverse mm/s 220"
speed_working = "max. 25 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis press operation mm/s max. 25 [4] with working speed of 10 mm/s"
speed_return = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis return speed mm/s 220"
backgauge_axes = "up to 6 CNC axes (axis names not listed)"   # published | https://www.trumpf.com/en_GB/products/machines-systems/bending-machines/trubend-series-5000/ | read 2026-11-24 | "up to 6 CNC-controlled backgauge axes"
bg_x_range = "860 / 1000 mm max stop range"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. stop range in X [5] mm 860 / 1000, [5] depending on the selected backgauge"
bg_x_travel = "600 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis traverse path mm 600"
bg_r_range = "250 mm traverse"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis traverse path mm 250"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "1000 / 1750 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis [5] mm/s 1000 / 1750"
bg_r_speed = "330 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis mm/s 330"
bg_z_speed = "1750 / 2500 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Z axis [5] mm/s 1750 / 2500"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "3020 x 1800 x 2375 mm (std)"   # conflict (web page TW5 gives height 2840 mm for 5085 and 5085 (S)) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Length x width mm 3020 x 1800 / 3530 x 1800[1]; Height mm 2375 / 2835[1]"
weight = "8000 / 8700 kg"   # conflict (web page says 8300 kg (5085) / 9100 kg (5085 S); brochure 8000 / 8700) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Weight kg 8000 / 8700[1]"
control = "Touchpoint TruBend"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "User interface Touchpoint TruBend"
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
# note: Second value of each pair is the enlarged design (option) per brochure footnote [1]. Web page TW5 lists 5085 (S) as 2720 mm bending length, 2260 mm between columns, 615 mm usable open height.
```

```toml
[[press]]
id = "trumpf-trubend-5130"
maker = "Trumpf"
model = "TruBend 5130"
tonnage = "1300 kN"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "TRUMPF bending machines brochure 202402, Technical data p.53: Press force kN 1300"
drive = "hydraulic with On-Demand Servo Drive"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "The On-Demand Servo Drive thus saves up to 42% in energy compared to traditional hydraulics (TruBend Series 5000 page)"
bending_length = "3230 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Bending length mm 3230"
housing_gap = "2690 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Width between columns mm 2690"
open_height = "385 / 615 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Usable open height mm 385 / 615 ([1] second value = enlarged design, option)"
table_to_beam = "505 / 735 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. table/press beam distance mm 505 / 735"
stroke = "215 / 445 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis stroke mm 215 / 445"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "420 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Throat depth mm 420 (web page TW5 also 420 mm)"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "1095-1115 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Working height mm 1095-1115, [2] with lower tool height of 100 mm"
speed_approach = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis rapid traverse mm/s 220"
speed_working = "max. 25 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis press operation mm/s max. 25 [4] with working speed of 10 mm/s"
speed_return = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis return speed mm/s 220"
backgauge_axes = "up to 6 CNC axes (axis names not listed)"   # published | https://www.trumpf.com/en_GB/products/machines-systems/bending-machines/trubend-series-5000/ | read 2026-11-24 | "up to 6 CNC-controlled backgauge axes"
bg_x_range = "860 / 1000 mm max stop range"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. stop range in X [5] mm 860 / 1000, [5] depending on the selected backgauge"
bg_x_travel = "600 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis traverse path mm 600"
bg_r_range = "250 mm traverse"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis traverse path mm 250"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "1000 / 1750 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis [5] mm/s 1000 / 1750"
bg_r_speed = "330 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis mm/s 330"
bg_z_speed = "1750 / 2500 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Z axis [5] mm/s 1750 / 2500"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "3980 x 1800 x 2375 / 2835 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Length x width mm 3980 x 1800; Height mm 2375 / 2835 ([6] basic machine without options)"
weight = "10700 / 11800 kg"   # conflict (web page TW5 lists 11800 kg and width 1900 mm, height 2840 mm) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Weight kg 10700 / 11800[1]"
control = "Touchpoint TruBend"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "User interface Touchpoint TruBend"
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
```

```toml
[[press]]
id = "trumpf-trubend-5230"
maker = "Trumpf"
model = "TruBend 5230"
tonnage = "2300 kN"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "TRUMPF bending machines brochure 202402, Technical data p.53: Press force kN 2300"
drive = "hydraulic with On-Demand Servo Drive"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "The On-Demand Servo Drive thus saves up to 42% in energy compared to traditional hydraulics (TruBend Series 5000 page)"
bending_length = "3230 / 4250 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Bending length mm 3230 / 4250"
housing_gap = "2690 / 3680 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Width between columns mm 2690 / 3680"
open_height = "615 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Usable open height mm 615 ([1] second value = enlarged design, option)"
table_to_beam = "735 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. table/press beam distance mm 735"
stroke = "445 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis stroke mm 445"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "420 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Throat depth mm 420 (web page TW5 also 420 mm)"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "1095-1115 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Working height mm 1095-1115, [2] with lower tool height of 100 mm"
speed_approach = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis rapid traverse mm/s 220"
speed_working = "max. 25 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis press operation mm/s max. 25 [4] with working speed of 10 mm/s"
speed_return = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis return speed mm/s 220"
backgauge_axes = "up to 6 CNC axes (axis names not listed)"   # published | https://www.trumpf.com/en_GB/products/machines-systems/bending-machines/trubend-series-5000/ | read 2026-11-24 | "up to 6 CNC-controlled backgauge axes"
bg_x_range = "860 / 1000 mm max stop range"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. stop range in X [5] mm 860 / 1000, [5] depending on the selected backgauge"
bg_x_travel = "600 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis traverse path mm 600"
bg_r_range = "250 mm traverse"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis traverse path mm 250"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "1000 / 1750 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis [5] mm/s 1000 / 1750"
bg_r_speed = "330 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis mm/s 330"
bg_z_speed = "1750 / 2500 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Z axis [5] mm/s 1750 / 2500"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "4150 x 2055 x 3200 mm"   # conflict (web page TW5 says height 3000 mm for 5230 and 5230 (S)) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Length x width mm 4150 x 2055 / 5140 x 2055[1]; Height mm 3200"
weight = "17200 / 19850 kg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Weight kg 17200 / 19850"
control = "Touchpoint TruBend"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "User interface Touchpoint TruBend"
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
```

```toml
[[press]]
id = "trumpf-trubend-5320"
maker = "Trumpf"
model = "TruBend 5320"
tonnage = "3200 kN"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "TRUMPF bending machines brochure 202402, Technical data p.53: Press force kN 3200"
drive = "hydraulic with On-Demand Servo Drive"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "The On-Demand Servo Drive thus saves up to 42% in energy compared to traditional hydraulics (TruBend Series 5000 page)"
bending_length = "4420 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Bending length mm 4420"
housing_gap = "3680 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Width between columns mm 3680"
open_height = "615 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Usable open height mm 615 ([1] second value = enlarged design, option)"
table_to_beam = "735 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. table/press beam distance mm 735"
stroke = "445 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis stroke mm 445"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "420 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Throat depth mm 420 (web page TW5 also 420 mm)"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "1110-1130 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Working height mm 1110-1130, [2] with lower tool height of 100 mm"
speed_approach = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis rapid traverse mm/s 220"
speed_working = "max. 25 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis press operation mm/s max. 25 [4] with working speed of 10 mm/s"
speed_return = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis return speed mm/s 220"
backgauge_axes = "up to 6 CNC axes (axis names not listed)"   # published | https://www.trumpf.com/en_GB/products/machines-systems/bending-machines/trubend-series-5000/ | read 2026-11-24 | "up to 6 CNC-controlled backgauge axes"
bg_x_range = "860 / 1000 mm max stop range"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. stop range in X [5] mm 860 / 1000, [5] depending on the selected backgauge"
bg_x_travel = "600 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis traverse path mm 600"
bg_r_range = "250 mm traverse"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis traverse path mm 250"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "1000 / 1750 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis [5] mm/s 1000 / 1750"
bg_r_speed = "330 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis mm/s 330"
bg_z_speed = "1750 / 2500 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Z axis [5] mm/s 1750 / 2500"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "5180 x 2055 x 3200 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Length x width mm 5180 x 2055; Height mm 3200 ([6] basic machine without options)"
weight = "23400 kg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Weight kg 23400"
control = "Touchpoint TruBend"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "User interface Touchpoint TruBend"
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
```

```toml
[[press]]
id = "trumpf-trubend-7036"
maker = "Trumpf"
model = "TruBend 7036"
tonnage = "360 kN"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Press force kN 360"
drive = "direct drive, torque motor (electric)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "the directly driven torque motor produces a high torque even at a low revolution speed"
bending_length = "1020 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Bending length mm 1020"
housing_gap = "932 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Width between columns mm 932"
open_height = "295 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Usable open height mm 295"
table_to_beam = "420 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. table/press beam distance mm 420"
stroke = "120 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis stroke mm 120"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "150 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Throat depth mm 150"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "1150 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Working height mm 1150 [7] with 100 mm die height"
speed_approach = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis rapid traverse mm/s 220"
speed_working = "max. 25 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis press operation max. 25 [8] dependent on die width, application, local regulations"
speed_return = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis return speed mm/s 220"
backgauge_axes = "6-axis backgauge (axis names not listed)"   # published | https://www.trumpf.com/en_GB/products/machines-systems/bending-machines/trubend-series-7000/ | read 2026-11-24 | "6-axis backgauge"
bg_x_range = "500 mm max stop range"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. stop range in X mm 500"
bg_x_travel = "240 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis traverse path mm 240"
bg_r_range = "153 mm traverse"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis traverse path mm 153"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "1500 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis mm/s 1500"
bg_r_speed = "750 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis mm/s 750"
bg_z_speed = "2200 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Z axis mm/s 2200"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "1900 x 1340 x 2380 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Length x width mm 1900 x 1340; Height mm 2380 (web TW7 same)"
weight = "3500 kg"   # conflict (web page TW7 says 3300 kg) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Weight kg 3500"
control = "Touchpoint TruBend"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "User interface Touchpoint TruBend"
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
```

```toml
[[press]]
id = "trumpf-trubend-7050"
maker = "Trumpf"
model = "TruBend 7050"
tonnage = "500 kN"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Press force kN 500"
drive = "direct drive, torque motor (electric)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "the directly driven torque motor produces a high torque even at a low revolution speed"
bending_length = "1530 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Bending length mm 1530"
housing_gap = "1305 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Width between columns mm 1305"
open_height = "385 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Usable open height mm 385"
table_to_beam = "505 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. table/press beam distance mm 505"
stroke = "215 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis stroke mm 215"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "250 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Throat depth mm 250"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "1150 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Working height mm 1150 [7] with 100 mm die height"
speed_approach = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis rapid traverse mm/s 220"
speed_working = "max. 25 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis press operation max. 25 [8] dependent on die width, application, local regulations"
speed_return = "220 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis return speed mm/s 220"
backgauge_axes = "6-axis backgauge (axis names not listed)"   # published | https://www.trumpf.com/en_GB/products/machines-systems/bending-machines/trubend-series-7000/ | read 2026-11-24 | "6-axis backgauge"
bg_x_range = "500 mm max stop range"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. stop range in X mm 500"
bg_x_travel = "240 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis traverse path mm 240"
bg_r_range = "153 mm traverse"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis traverse path mm 153"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "1500 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis mm/s 1500"
bg_r_speed = "750 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis mm/s 750"
bg_z_speed = "2200 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Z axis mm/s 2200"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "2420 x 1805 x 2580 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Length x width mm 2420 x 1805; Height mm 2580 (web TW7 same)"
weight = "5500 kg"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Weight kg 5500 (web TW7 same)"
control = "Touchpoint TruBend"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "User interface Touchpoint TruBend"
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
```

```toml
[[press]]
id = "trumpf-trubend-3100"
maker = "Trumpf"
model = "TruBend 3100 (generation in 202402 brochure)"
tonnage = "1000 kN"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Technical data p.52, TruBend 3100: Press force kN 1000"
drive = "hydraulic"   # estimate (Series 3000 is the hydraulic line; not stated in table) |  | read 2026-11-24 | ""
bending_length = "3060 mm"   # conflict (web page TW3 (newer generation) lists TruBend 3100 max. bending length 3230 mm) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Bending length mm 3060"
housing_gap = "3384 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Width between columns mm 3384"
open_height = "350 / 500 mm (manual/hydraulic clamp); 430 / 580 mm (TRUMPF QuickClamp)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Usable open height with manual and hydraulic tool clamp mm 350 / 500[1]; with TRUMPF QuickClamp mm 430 / 580[1]"
table_to_beam = "470 / 620 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. table/press beam distance mm 470 / 620[1] ([1] increased open height, option)"
stroke = "200 / 350 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis stroke mm 200 / 350[1]"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "none (open frame without throat limit stated)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Throat depth mm – (dash printed for 3066/3100/3170)"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "1049-1069 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Working height [2] mm 1049 – 1069, with lower tool height of 100 mm"
speed_approach = "200 mm/s"   # conflict (web page TW3 gives max. rapid feedrate 170 mm/s for TruBend 3100) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis rapid traverse [4] mm/s 200 ([4] with BendGuard option)"
speed_working = "15 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. Y axis operation mm/s 15 (web TW3: max. working speed 15 mm/s)"
speed_return = "200 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Y axis return speed mm/s 200"
backgauge_axes = "2-, 4- or 5-axis"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "The 2-, 4- or 5-axis backgauge makes the machine quick and precise (Series 3000 page)"
bg_x_range = "860 mm max stop range"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Max. stop range in X direction mm 860"
bg_x_travel = "600 mm"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis traverse path mm 600"
bg_r_range = "150 mm traverse"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis traverse path mm 150"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "500 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "X axis mm/s 500"
bg_r_speed = "200 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "R axis mm/s 200"
bg_z_speed = "1000 mm/s"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Z axis mm/s 1000"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "manual or hydraulic clamp, or TRUMPF QuickClamp (option)"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Usable open height with manual and hydraulic tool clamp / with TRUMPF QuickClamp"
overall_lwh = "3607 x 1644 x 2370 (2720 enlarged) mm"   # conflict (web page TW3 newer 3100: 4220 x 2003 x 2375 mm) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Length x width mm 3607 x 1644; Height mm 2370 / 2720[1]"
weight = "7700 / 8300 kg"   # conflict (web page TW3 newer 3100: 9200 kg) | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Weight kg 7700 / 8300[1]"
control = "T3500T"   # published | https://www.trumpf.com/filestorage/TRUMPF_Master/Products/Machines_and_Systems/02_Brochures/TRUMPF-bending-machines-brochure-EN.pdf | read 2026-11-24 | "Control T3500T"
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
# note: The 2024 brochure and the current web page describe different TruBend 3000 generations; the web page table was only partly readable (misaligned), so the brochure is used.
```

```toml
[[press]]
id = "amada-hg-5020"
maker = "Amada"
model = "HG-5020"
tonnage = "500 kN"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "HG series catalogue E050-HQ02en Nov.2019 (Japanese domestic market), p.8: Tonnage capacity kN 500"
drive = "hybrid drive (servo pump hydraulics)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page007.jpg | read 2026-11-24 | "Hybrid drive system, High speed & high accuracy press brakes"
bending_length = "2150 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Table length mm 2150"
housing_gap = "1700 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Distance between frames 1700 (Unit: mm)"
open_height = "520 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Open height 520 (Unit: mm), one value for all HG models; holder condition not stated"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "250 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Stroke length mm 250 (also product page AJ, cell spans all models)"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "UNKNOWN"   # unknown | - | read 2026-11-24 | "not on the JP catalogue page; amada.eu pages refused automated reads (Cloudflare 403); search-engine snippets attribute 450 mm / 455 mm to amada.eu, unverified"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "950 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Bed height 950 (Unit: mm)"
speed_approach = "220 mm/s"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Approach speed mm/s 220"
speed_working = "20 mm/s"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Bending speed mm/s 20"
speed_return = "250 mm/s"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Return speed mm/s 250"
backgauge_axes = "L1, L2, Z1, Z2 (+ optional LS1, LS2); D1, D2 ram; Y1, Y2"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "AMNC 3i Axes under CNC control: 8-axis: D1, D2, L1, L2, Y1, Y2, Z1, Z2 (LS1, LS2) optional"
bg_x_range = "700 mm measurement length (L axis)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Backgauge measurement length mm 700"
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "250 mm (30 to 280 mm) finger height"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Backgauge finger height range mm 250 (30 to 280)"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "30 m/min (L axis)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Feed rate m/min L-axis: 30, Y-axis: 90, Z-axis: 20"
bg_r_speed = "90 m/min (Y axis = finger height in Amada naming)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Feed rate m/min ... Y-axis: 90"
bg_z_speed = "20 m/min"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Feed rate m/min ... Z-axis: 20"
bg_fingers = "standard detachable / step-tipped fingers; options listed"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page007.jpg | read 2026-11-24 | "Backgauge fingers: Detachable fingers, Step-tipped finger, 90 mm support, 300 mm stopper"
clamping = "AGRIP (Amada holders), AMTS hydraulic option; 1V die holders"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page007.jpg | read 2026-11-24 | "AGRIP Amada standard holders for punches ... AGRIP-M manual, AGRIP-A hydraulic; Amada modular tooling system (AMTS)"
overall_lwh = "machine width 3070, total width 4435, height 2810, depth 2400 (2615 with L-shift) mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Machine width L 3070; Total width L (Maximum footprint) 4435; Total height H 2810; Depth W (with L-shift) 2400 (2615)"
weight = "5500 kg"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Machine mass kg 5500"
control = "AMNC 3i"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "AMNC 3i specifications: 18.5 in wide multi-touch LCD screen"
crowning = "CC axis (crowning) 0 to 500%"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Least input increment ... CC-axis: 0 to 500%; Motor power ... (CC)"
# note: Tilt range (left-right) L-axis +/-320 mm per catalogue. Japanese-market specification (catalogue footnote).
```

```toml
[[press]]
id = "amada-hg-1303"
maker = "Amada"
model = "HG-1303"
tonnage = "1300 kN"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "HG series catalogue E050-HQ02en Nov.2019 (Japanese domestic market), p.8: Tonnage capacity kN 1300"
drive = "hybrid drive (servo pump hydraulics)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page007.jpg | read 2026-11-24 | "Hybrid drive system, High speed & high accuracy press brakes"
bending_length = "3110 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Table length mm 3110"
housing_gap = "2700 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Distance between frames 2700 (Unit: mm)"
open_height = "520 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Open height 520 (Unit: mm), one value for all HG models; holder condition not stated"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "250 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Stroke length mm 250 (also product page AJ, cell spans all models)"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "UNKNOWN"   # unknown | - | read 2026-11-24 | "not on the JP catalogue page; amada.eu pages refused automated reads (Cloudflare 403); search-engine snippets attribute 450 mm / 455 mm to amada.eu, unverified"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "950 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Bed height 950 (Unit: mm)"
speed_approach = "220 mm/s"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Approach speed mm/s 220"
speed_working = "20 mm/s"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Bending speed mm/s 20"
speed_return = "250 mm/s"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Return speed mm/s 250"
backgauge_axes = "L1, L2, Z1, Z2 (+ optional LS1, LS2); D1, D2 ram; Y1, Y2"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "AMNC 3i Axes under CNC control: 8-axis: D1, D2, L1, L2, Y1, Y2, Z1, Z2 (LS1, LS2) optional"
bg_x_range = "700 mm measurement length (L axis)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Backgauge measurement length mm 700"
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "250 mm (30 to 280 mm) finger height"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Backgauge finger height range mm 250 (30 to 280)"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "30 m/min (L axis)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Feed rate m/min L-axis: 30, Y-axis: 90, Z-axis: 20"
bg_r_speed = "90 m/min (Y axis = finger height in Amada naming)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Feed rate m/min ... Y-axis: 90"
bg_z_speed = "20 m/min"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Feed rate m/min ... Z-axis: 20"
bg_fingers = "standard detachable / step-tipped fingers; options listed"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page007.jpg | read 2026-11-24 | "Backgauge fingers: Detachable fingers, Step-tipped finger, 90 mm support, 300 mm stopper"
clamping = "AGRIP (Amada holders), AMTS hydraulic option; 1V die holders"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page007.jpg | read 2026-11-24 | "AGRIP Amada standard holders for punches ... AGRIP-M manual, AGRIP-A hydraulic; Amada modular tooling system (AMTS)"
overall_lwh = "machine width 4030, total width 5455, height 3070, depth 2400 (2615 with L-shift) mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Machine width L 4030; Total width L (Maximum footprint) 5455; Total height H 3070; Depth W (with L-shift) 2400 (2615)"
weight = "11500 kg"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Machine mass kg 11500"
control = "AMNC 3i"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "AMNC 3i specifications: 18.5 in wide multi-touch LCD screen"
crowning = "CC axis (crowning) 0 to 500%"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Least input increment ... CC-axis: 0 to 500%; Motor power ... (CC)"
# note: Tilt range (left-right) L-axis +/-500 mm per catalogue. Japanese-market specification (catalogue footnote).
```

```toml
[[press]]
id = "amada-hg-2204"
maker = "Amada"
model = "HG-2204"
tonnage = "2200 kN"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "HG series catalogue E050-HQ02en Nov.2019 (Japanese domestic market), p.8: Tonnage capacity kN 2200"
drive = "hybrid drive (servo pump hydraulics)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page007.jpg | read 2026-11-24 | "Hybrid drive system, High speed & high accuracy press brakes"
bending_length = "4300 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Table length mm 4300"
housing_gap = "3760 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Distance between frames 3760 (Unit: mm)"
open_height = "520 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Open height 520 (Unit: mm), one value for all HG models; holder condition not stated"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "250 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Stroke length mm 250 (also product page AJ, cell spans all models)"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "UNKNOWN"   # unknown | - | read 2026-11-24 | "not on the JP catalogue page; amada.eu pages refused automated reads (Cloudflare 403); search-engine snippets attribute 450 mm / 455 mm to amada.eu, unverified"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "950 mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Bed height 950 (Unit: mm)"
speed_approach = "220 mm/s"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Approach speed mm/s 220"
speed_working = "20 mm/s"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Bending speed mm/s 20"
speed_return = "250 mm/s"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Return speed mm/s 250"
backgauge_axes = "L1, L2, Z1, Z2 (+ optional LS1, LS2); D1, D2 ram; Y1, Y2"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "AMNC 3i Axes under CNC control: 8-axis: D1, D2, L1, L2, Y1, Y2, Z1, Z2 (LS1, LS2) optional"
bg_x_range = "700 mm measurement length (L axis)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Backgauge measurement length mm 700"
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "250 mm (30 to 280 mm) finger height"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Backgauge finger height range mm 250 (30 to 280)"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "30 m/min (L axis)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Feed rate m/min L-axis: 30, Y-axis: 90, Z-axis: 20"
bg_r_speed = "90 m/min (Y axis = finger height in Amada naming)"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Feed rate m/min ... Y-axis: 90"
bg_z_speed = "20 m/min"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Feed rate m/min ... Z-axis: 20"
bg_fingers = "standard detachable / step-tipped fingers; options listed"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page007.jpg | read 2026-11-24 | "Backgauge fingers: Detachable fingers, Step-tipped finger, 90 mm support, 300 mm stopper"
clamping = "AGRIP (Amada holders), AMTS hydraulic option; 1V die holders"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page007.jpg | read 2026-11-24 | "AGRIP Amada standard holders for punches ... AGRIP-M manual, AGRIP-A hydraulic; Amada modular tooling system (AMTS)"
overall_lwh = "machine width 5220, total width 6590, height 3220, depth 2400 (2615 with L-shift) mm"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Machine width L 5220; Total width L (Maximum footprint) 6590; Total height H 3220; Depth W (with L-shift) 2400 (2615)"
weight = "20000 kg"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Machine mass kg 20000"
control = "AMNC 3i"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "AMNC 3i specifications: 18.5 in wide multi-touch LCD screen"
crowning = "CC axis (crowning) 0 to 500%"   # published | https://products.amada.co.jp/media/products/id000055/2/ebook/E050-HQ02en_Nov2019_HG-series/images/book2/page008.jpg | read 2026-11-24 | "Least input increment ... CC-axis: 0 to 500%; Motor power ... (CC)"
# note: Tilt range (left-right) L-axis +/-500 mm per catalogue. Japanese-market specification (catalogue footnote).
```

```toml
[[press]]
id = "amada-hg-1003atc"
maker = "Amada"
model = "HG-1003ATC"
tonnage = "1000 kN"   # published | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "HG-1003ATC catalogue F007-HQ02en May 2016 (Japanese domestic market), p.4: Capacity kN 1000"
drive = "hybrid"   # published | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "hybrid digital press brake HG with ATC (product page id000056)"
bending_length = "3110 mm"   # published | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "Bending length mm 3110; drawing: (table length) 3110"
housing_gap = "3143 mm"   # estimate (unlabelled dimension 3143 between frame lines in the front-view drawing) | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "drawing dimension 3143 (unit mm) above the table"
open_height = "601 mm"   # estimate (unlabelled vertical drawing dimension 601 between table and beam) | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "drawing dimensions 984 / 601 / 2974 (unit mm), front view"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "620 mm"   # published | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "Stroke length mm 620"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "984 mm"   # estimate (unlabelled vertical drawing dimension from floor to table) | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "drawing dimension 984 (unit mm)"
speed_approach = "220 mm/s"   # published | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "Rapid closing speed mm/s 220"
speed_working = "20 mm/s"   # published | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "Bending speed mm/s 20"
speed_return = "250 mm/s"   # published | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "Opening speed mm/s 250"
backgauge_axes = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "AMTS III (ATC)"   # published | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "ATC specifications: Tool type AMTS III (ATC); 18 dies, 15 punch stockers; tool layout length 15 to 2495 mm"
overall_lwh = "approx. 5640 (3997+1643) x 3006 x 2974 mm"   # published | https://products.amada.co.jp/media/products/id000056/2/ebook/F007-HQ02en_May2016_HG-1003ATC/images/book2/page004.jpg | read 2026-11-24 | "drawing: 386 / 3997 / 1643 widths, depth 3006, heights 2974 and 2800 (unit mm)"
weight = "UNKNOWN"   # unknown | - | - | not found in the sources read
control = "UNKNOWN"   # unknown | - | - | not found in the sources read
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
# note: Stroke 620 mm is printed in the 2016 catalogue (much longer than HG 250 mm); kept as published.
```

```toml
[[press]]
id = "bystronic-xpert-pro-100-3100"
maker = "Bystronic"
model = "Xpert Pro 100/3100"
tonnage = "1000 kN"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Bystronic Xpert Pro Technical Data (tp007, en_GB): Tonnage 1,000 kN"
drive = "hydraulic (ByTower / Servo Dynamic supply options listed)"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Oil capacity ... Electrical supply ByTower ... Electric supply (Servo Dynamic)"
bending_length = "3100 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Bending length 3,100 mm"
housing_gap = "2750 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Distance between side frames 2,750 mm"
open_height = "640 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Open height 640 mm"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "250 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Standard stroke 250 mm"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "400 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Throat 400 mm"
table_width = "74 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Table Width 74 mm"
table_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_approach = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_working = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_return = "UNKNOWN"   # unknown | - | - | not found in the sources read
backgauge_axes = "2/4/5/6-axis variants; X, R, Z named"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "X-axis ... (2/4/5-axis); Z-axis speed for 6-axis"
bg_x_range = "1050 mm max range"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Max. back gauge range X-axis (2/4/5-axis) 1,050 mm"
bg_x_travel = "600 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "X-axis travel (2/4/5-axis) 600 mm"
bg_r_range = "250 mm travel"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "R-axis travel (2/4/5-axis) 250 mm"
bg_z_range = "1900 mm travel"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Z-axis travel 1,900 mm"
bg_x_speed = "650 mm/s"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "X-axis speed for 2/4/5/6-axis (up to) 650 mm/s"
bg_r_speed = "200 mm/s"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "R-axis speed for 2/4/5-axis (up to) 200 mm/s"
bg_z_speed = "1500 mm/s (4/5-axis), 1200 mm/s (6-axis)"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Z-axis speed for 4/5-axis (up to) 1,500 mm/s; for 6-axis 1,200 mm/s"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "4180 x 2000 x 2820 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Length 4,180 mm; Width 2,000 mm; Height 2,820 mm; footprint approx. 6,000 x 4,500 mm"
weight = "12.2 t"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Weight 12.2 t"
control = "UNKNOWN"   # unknown | - | - | not found in the sources read
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
# note: Bystronic notes: technical data can vary in different countries, according to local security rules and configuration.
```

```toml
[[press]]
id = "bystronic-xpert-pro-ext-150-3100"
maker = "Bystronic"
model = "Xpert Pro Extended 150/3100"
tonnage = "1500 kN"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Bystronic Xpert Pro Technical Data (tp007, en_GB): Tonnage 1,500 kN"
drive = "hydraulic (ByTower / Servo Dynamic supply options listed)"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Oil capacity ... Electrical supply ByTower ... Electric supply (Servo Dynamic)"
bending_length = "3100 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Bending length 3,100 mm"
housing_gap = "2750 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Distance between side frames 2,750 mm"
open_height = "840 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Open height 840 mm"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "450 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Standard stroke 450 mm"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "400 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Throat 400 mm"
table_width = "74 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Table Width 74 mm"
table_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_approach = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_working = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_return = "UNKNOWN"   # unknown | - | - | not found in the sources read
backgauge_axes = "2/4/5/6-axis variants; X, R, Z named"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "X-axis ... (2/4/5-axis); Z-axis speed for 6-axis"
bg_x_range = "1050 mm max range"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Max. back gauge range X-axis (2/4/5-axis) 1,050 mm"
bg_x_travel = "600 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "X-axis travel (2/4/5-axis) 600 mm"
bg_r_range = "250 mm travel"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "R-axis travel (2/4/5-axis) 250 mm"
bg_z_range = "1900 mm travel"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Z-axis travel 1,900 mm"
bg_x_speed = "650 mm/s"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "X-axis speed for 2/4/5/6-axis (up to) 650 mm/s"
bg_r_speed = "200 mm/s"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "R-axis speed for 2/4/5-axis (up to) 200 mm/s"
bg_z_speed = "1500 mm/s (4/5-axis), 1200 mm/s (6-axis)"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Z-axis speed for 4/5-axis (up to) 1,500 mm/s; for 6-axis 1,200 mm/s"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "4180 x 2000 x 3420 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Length 4,180 mm; Width 2,000 mm; Height 3,420 mm; footprint approx. 6,000 x 4,500 mm"
weight = "13.2 t"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Weight 13.2 t"
control = "UNKNOWN"   # unknown | - | - | not found in the sources read
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
# note: Bystronic notes: technical data can vary in different countries, according to local security rules and configuration.
```

```toml
[[press]]
id = "bystronic-xpert-pro-320-4300"
maker = "Bystronic"
model = "Xpert Pro 320/4300"
tonnage = "3200 kN"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Bystronic Xpert Pro Technical Data (tp007, en_GB): Tonnage 3,200 kN"
drive = "hydraulic (ByTower / Servo Dynamic supply options listed)"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Oil capacity ... Electrical supply ByTower ... Electric supply (Servo Dynamic)"
bending_length = "4300 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Bending length 4,300 mm"
housing_gap = "3750 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Distance between side frames 3,750 mm"
open_height = "640 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Open height 640 mm"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "250 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Standard stroke 250 mm"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "400 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Throat 400 mm"
table_width = "74 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Table Width 74 mm"
table_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_approach = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_working = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_return = "UNKNOWN"   # unknown | - | - | not found in the sources read
backgauge_axes = "2/4/5/6-axis variants; X, R, Z named"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "X-axis ... (2/4/5-axis); Z-axis speed for 6-axis"
bg_x_range = "1050 mm max range"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Max. back gauge range X-axis (2/4/5-axis) 1,050 mm"
bg_x_travel = "600 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "X-axis travel (2/4/5-axis) 600 mm"
bg_r_range = "250 mm travel"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "R-axis travel (2/4/5-axis) 250 mm"
bg_z_range = "2900 mm travel"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Z-axis travel 2,900 mm"
bg_x_speed = "650 mm/s"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "X-axis speed for 2/4/5/6-axis (up to) 650 mm/s"
bg_r_speed = "200 mm/s"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "R-axis speed for 2/4/5-axis (up to) 200 mm/s"
bg_z_speed = "1500 mm/s (4/5-axis), 1200 mm/s (6-axis)"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Z-axis speed for 4/5-axis (up to) 1,500 mm/s; for 6-axis 1,200 mm/s"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "5260 x 2000 x 3170 mm"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Length 5,260 mm; Width 2,000 mm; Height 3,170 mm; footprint approx. 7,000 x 4,500 mm"
weight = "24.3 t"   # published | https://www.bystronic.com/sites/default/files/2026-03/tp007-BE-Techdaten-Xpert%20Pro_en_GB_0.pdf | read 2026-11-24 | "Weight 24.3 t"
control = "UNKNOWN"   # unknown | - | - | not found in the sources read
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
# note: Bystronic notes: technical data can vary in different countries, according to local security rules and configuration.
```

```toml
[[press]]
id = "bystronic-bybend-star-80"
maker = "Bystronic"
model = "ByBend Star 80"
tonnage = "80 t"   # published | https://www.bystronic.com/che/en/b/bending-machine-bybend-star-40-80 | read 2026-11-24 | "Technical specifications ByBend Star 80: Tonnage 80 t"
drive = "UNKNOWN"   # unknown | - | - | not found in the sources read
bending_length = "1530 mm"   # published | https://www.bystronic.com/che/en/b/bending-machine-bybend-star-40-80 | read 2026-11-24 | "Bending length 1530 mm"
housing_gap = "UNKNOWN"   # unknown | - | - | not found in the sources read
open_height = "515 mm"   # published | https://www.bystronic.com/che/en/b/bending-machine-bybend-star-40-80 | read 2026-11-24 | "Open height 515 mm"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "200 mm"   # published | https://www.bystronic.com/che/en/b/bending-machine-bybend-star-40-80 | read 2026-11-24 | "Standard stroke 200 mm"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_approach = "250 mm/s"   # published | https://www.bystronic.com/che/en/b/bending-machine-bybend-star-40-80 | read 2026-11-24 | "Max. high speed 250 mm/s"
speed_working = "25 mm/s"   # published | https://www.bystronic.com/che/en/b/bending-machine-bybend-star-40-80 | read 2026-11-24 | "Max. working speed 25 mm/s"
speed_return = "UNKNOWN"   # unknown | - | - | not found in the sources read
backgauge_axes = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "RFA hydraulic fast clamping (option)"   # published | https://www.bystronic.com/che/en/b/bending-machine-bybend-star-40-80 | read 2026-11-24 | "RFA clamping (Fast clamping System) ... selection of hydraulic clamping systems"
overall_lwh = "UNKNOWN"   # unknown | - | - | not found in the sources read
weight = "UNKNOWN"   # unknown | - | - | not found in the sources read
control = "BySoft Cell Control Bend"   # published | https://www.bystronic.com/che/en/b/bending-machine-bybend-star-40-80 | read 2026-11-24 | "BySoft Cell Control Bend"
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
# note: Page read by curl; WebFetch of the same page failed (header overflow). Only the 80 t table was in the page HTML.
```

```toml
[[press]]
id = "lvd-easy-form-135-30"
maker = "LVD"
model = "Easy-Form 135/30"
tonnage = "1350 kN"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Pressing force kN 1350"
drive = "hydraulic"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "LVD hydraulic press brake brochure; pump regulates flow rate"
bending_length = "3050 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Working length mm 3050"
housing_gap = "2600 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Distance between uprights mm 2600"
open_height = "400 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Distance table/ram mm 400"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "200 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Stroke mm 200"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "400 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Gap mm 400 (LVD term for throat)"
table_width = "120 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Table width mm 120"
table_height = "970 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Working height mm 970"
speed_approach = "180 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Approach speed* mm/s 180"
speed_working = "22 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Working speed** mm/s 22"
speed_return = "200 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Return speed mm/s 200"
backgauge_axes = "2-axis X,R (manual Z) on Easy-Form 6; 5-axis X,R,Z1,Z2,X' on Easy-Form 9; 6-axis X1,R1,Z1,X2,R2,Z2 option"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "Standard two-axis backgauge (X, R) with manual Z-axis on Easy-Form 6; Five-axis backgauge (X, R, Z1, Z2, X') on Easy-Form 9"
bg_x_range = "gauging to 1000 mm with material support"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "Side view standard backgauge with three gauge positions. Allows gauging to 1000 mm with material support."
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "three-point gauging fingers"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "The three-point gauge fingers allow automatic calculation ... Top view standard three-point gauging finger 98 / 35 / 50 / 7,5"
clamping = "quick-acting hydraulic clamping on ram and table (option)"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "options ... quick-acting hydraulic clamping on ram and on table, hardened clamping"
overall_lwh = "UNKNOWN"   # unknown | - | - | not found in the sources read
weight = "9500 kg"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Weight kg 9,500"
control = "Touch-B 19 in"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "The 19 in Touch-B control is user-friendly"
crowning = "CNC crowning (V-axis)"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "CNC CROWNING ... tailor-made V-axis crowning system"
```

```toml
[[press]]
id = "lvd-easy-form-220-30-plus"
maker = "LVD"
model = "Easy-Form 220/30 Plus"
tonnage = "2200 kN"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Pressing force kN 2200"
drive = "hydraulic"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "LVD hydraulic press brake brochure; pump regulates flow rate"
bending_length = "3050 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Working length mm 3050"
housing_gap = "2600 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Distance between uprights mm 2600"
open_height = "570 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Distance table/ram mm 570"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "300 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Stroke mm 300"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "400 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Gap mm 400 (LVD term for throat)"
table_width = "200 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Table width mm 200"
table_height = "1000 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Working height mm 1000"
speed_approach = "120 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Approach speed* mm/s 120"
speed_working = "21 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Working speed** mm/s 21"
speed_return = "200 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Return speed mm/s 200"
backgauge_axes = "2-axis X,R (manual Z) on Easy-Form 6; 5-axis X,R,Z1,Z2,X' on Easy-Form 9; 6-axis X1,R1,Z1,X2,R2,Z2 option"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "Standard two-axis backgauge (X, R) with manual Z-axis on Easy-Form 6; Five-axis backgauge (X, R, Z1, Z2, X') on Easy-Form 9"
bg_x_range = "gauging to 1000 mm with material support"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "Side view standard backgauge with three gauge positions. Allows gauging to 1000 mm with material support."
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "three-point gauging fingers"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "The three-point gauge fingers allow automatic calculation ... Top view standard three-point gauging finger 98 / 35 / 50 / 7,5"
clamping = "quick-acting hydraulic clamping on ram and table (option)"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "options ... quick-acting hydraulic clamping on ram and on table, hardened clamping"
overall_lwh = "UNKNOWN"   # unknown | - | - | not found in the sources read
weight = "14500 kg"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/easy-form | read 2026-11-24 | "Weight kg 14,500"
control = "Touch-B 19 in"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "The 19 in Touch-B control is user-friendly"
crowning = "CNC crowning (V-axis)"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "CNC CROWNING ... tailor-made V-axis crowning system"
```

```toml
[[press]]
id = "lvd-ppeb-320-40"
maker = "LVD"
model = "PPEB 320/40"
tonnage = "3200 kN"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Pressing force kN 3200"
drive = "hydraulic"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "LVD hydraulic press brake brochure; pump regulates flow rate"
bending_length = "4000 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Working length mm 4000"
housing_gap = "3150 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Distance between uprights mm 3150"
open_height = "570 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Distance table/ram mm 570"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "300 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Stroke mm 300"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "400 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Gap mm 400 (LVD term for throat)"
table_width = "200 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Table width mm 200"
table_height = "1000 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Working height mm 1000"
speed_approach = "120 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Approach speed* mm/s 120"
speed_working = "14 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Working speed** mm/s 14"
speed_return = "130 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Return speed mm/s 130"
backgauge_axes = "2-axis X,R (manual Z) on Easy-Form 6; 5-axis X,R,Z1,Z2,X' on Easy-Form 9; 6-axis X1,R1,Z1,X2,R2,Z2 option"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "Standard two-axis backgauge (X, R) with manual Z-axis on Easy-Form 6; Five-axis backgauge (X, R, Z1, Z2, X') on Easy-Form 9"
bg_x_range = "gauging to 1000 mm with material support"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "Side view standard backgauge with three gauge positions. Allows gauging to 1000 mm with material support."
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "three-point gauging fingers"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "The three-point gauge fingers allow automatic calculation ... Top view standard three-point gauging finger 98 / 35 / 50 / 7,5"
clamping = "quick-acting hydraulic clamping on ram and table (option)"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "options ... quick-acting hydraulic clamping on ram and on table, hardened clamping"
overall_lwh = "UNKNOWN"   # unknown | - | - | not found in the sources read
weight = "23000 kg"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/ppeb | read 2026-11-24 | "Weight kg 23,000"
control = "Touch-B 19 in"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "The 19 in Touch-B control is user-friendly"
crowning = "CNC crowning (V-axis)"   # published | https://www.lvdgroup.com/sites/default/files/uploads/downloads/easy-form-2023_en.pdf | read 2026-11-24 | "CNC CROWNING ... tailor-made V-axis crowning system"
```

```toml
[[press]]
id = "lvd-dyna-press-40-15"
maker = "LVD"
model = "Dyna-Press 40/15"
tonnage = "400 kN"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Dyna-Press 40/15 Pressing force 400 kN"
drive = "electric drive"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Dyna-Press (LVD electric-drive compact press brake range)"
bending_length = "1530 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Working length 1530 mm"
housing_gap = "1535 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Distance between uprights 1535 mm"
open_height = "400 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Distance table/ram 400 mm"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "200 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Stroke 200 mm"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "100 mm (left side)"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Gap (left side) 100 mm"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "900 mm"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Working height 900 mm"
speed_approach = "100 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Approach speed* 100 mm/s"
speed_working = "25 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Working speed** 25 mm/s"
speed_return = "100 mm/s"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Return speed 100 mm/s"
backgauge_axes = "X, R, Z1, Z2"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Backgauge X, R, Z1, Z2"
bg_x_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "manual; top universal/W style, bottom universal/LVD style"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Top tooling universal/W style; Bottom tooling universal/LVD style; Clamping manual"
overall_lwh = "height 2500, depth 1205 mm (length not given)"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Total height 2500 mm; Total depth 1205 mm"
weight = "3750 kg"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Weight 3750 kg"
control = "15 in Touch-B"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "Control 15 in Touch-B"
crowning = "no"   # published | https://www.lvdgroup.com/en/press-brakes/stand-alone-press-brakes/dyna-press | read 2026-11-24 | "CNC crowning no"
```

```toml
[[press]]
id = "cincinnati-goform-40gx3"
maker = "Cincinnati"
model = "GOFORM 40GX3"
tonnage = "40 US tons"   # estimate (model number prefix (Cincinnati names models by tonnage); table has no tonnage column) | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "Model GOFORM 40GX3"
drive = "electric, planetary roller screw"   # published | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "this machine gets it done with a planetary roller screw design"
bending_length = "40 in"   # published | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "Overall Die Surface 40 in"
housing_gap = "UNKNOWN"   # unknown | - | - | not found in the sources read
open_height = "17 in"   # published | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "Open Height (in.) 17"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "10 in"   # published | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "Stroke Length (in.) 10"
closed_height = "7 in"   # published | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "Closed Height (in.) 7"
throat = "8 in"   # published | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "Throat (in.) 8"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_approach = "400 in/min"   # published | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "Approach Speed (ipm) 400"
speed_working = "60 in/min"   # published | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "Forming Speed (ipm) 60"
speed_return = "400 in/min"   # published | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "Return Speed (ipm) 400"
backgauge_axes = "6-axis"   # published | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "The Goform has a 10 in stroke and 6-axis back gauge"
bg_x_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "UNKNOWN"   # unknown | - | - | not found in the sources read
weight = "6,400 lb (domestic shipping)"   # published | https://www.e-ci.com/goform-electric-press-brakes | read 2026-11-24 | "Domestic Shipping Wgt. (lbs) 6,400"
control = "UNKNOWN"   # unknown | - | - | not found in the sources read
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
# note: Open height = closed height + stroke (7 + 10 = 17 in) as printed.
```

```toml
[[press]]
id = "cincinnati-xform-135xf10"
maker = "Cincinnati"
model = "XFORM 135XF10"
tonnage = "135 US tons"   # estimate (model number prefix (Cincinnati names models by tonnage); table has no tonnage column) | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Model XFORM 135XF10"
drive = "hydraulic"   # estimate (page describes hydraulic cylinders with clevis brackets; drive type word not in table) |  | read 2026-11-24 | ""
bending_length = "12 ft"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Overall Die Surface 12 ft"
housing_gap = "UNKNOWN"   # unknown | - | - | not found in the sources read
open_height = "21 in"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Open Height (in.) 21"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "14 in"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Stroke Length (in.) 14"
closed_height = "7 in"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Closed Height (in.) 7"
throat = "18 in"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Throat (in.) 18"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_approach = "650 in/min"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Approach Speed (ipm) 650"
speed_working = "55 in/min"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Forming Speed (ipm) 55"
speed_return = "540 in/min"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Return Speed (ipm) 540"
backgauge_axes = "5-axis or 6-axis"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "5-Axis Backgage ... 6-Axis Backgage: depth, position, angle, and height"
bg_x_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "UNKNOWN"   # unknown | - | - | not found in the sources read
weight = "UNKNOWN"   # unknown | - | - | not found in the sources read
control = "24 in touch screen PC control"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "high resolution, 24 in touch screen monitors with 3D graphics"
crowning = "programmable (CNC and Auto Crown)"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Programmable Crowning: Our intelligent crowning systems (CNC and Auto Crown)"
```

```toml
[[press]]
id = "cincinnati-xform-350xf12"
maker = "Cincinnati"
model = "XFORM 350XF12"
tonnage = "350 US tons"   # estimate (model number prefix (Cincinnati names models by tonnage); table has no tonnage column) | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Model XFORM 350XF12"
drive = "hydraulic"   # estimate (as 135XF10) |  | read 2026-11-24 | ""
bending_length = "14 ft"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Overall Die Surface 14 ft"
housing_gap = "UNKNOWN"   # unknown | - | - | not found in the sources read
open_height = "24 in"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Open Height (in.) 24"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "16 in"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Stroke Length (in.) 16"
closed_height = "8 in"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Closed Height (in.) 8"
throat = "18 in"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Throat (in.) 18"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_approach = "300 in/min"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Approach Speed (ipm) 300"
speed_working = "30 in/min"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Forming Speed (ipm) 30"
speed_return = "330 in/min"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Return Speed (ipm) 330"
backgauge_axes = "5-axis or 6-axis"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "5-Axis Backgage ... 6-Axis Backgage"
bg_x_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "UNKNOWN"   # unknown | - | - | not found in the sources read
weight = "UNKNOWN"   # unknown | - | - | not found in the sources read
control = "UNKNOWN"   # unknown | - | - | not found in the sources read
crowning = "programmable (CNC and Auto Crown)"   # published | https://www.e-ci.com/xform-series-press-brake | read 2026-11-24 | "Programmable Crowning"
```

```toml
[[press]]
id = "cincinnati-xi-90xi8"
maker = "Cincinnati"
model = "Xi 90Xi8"
tonnage = "90 US tons"   # estimate (model number prefix (Cincinnati names models by tonnage); table has no tonnage column) | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "Model Xi 90Xi8"
drive = "hydraulic"   # estimate (page: closed-loop hydraulic control, servo-proportional valves) |  | read 2026-11-24 | ""
bending_length = "10 ft"   # published | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "Overall Die Surface 10 ft"
housing_gap = "UNKNOWN"   # unknown | - | - | not found in the sources read
open_height = "15 in"   # published | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "Open Height (in.) 15"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "8 in"   # published | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "Stroke Length (in.) 8"
closed_height = "7 in"   # published | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "Closed Height (in.) 7"
throat = "16 in"   # published | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "Throat (in.) 16"
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_approach = "400 in/min"   # published | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "Approach Speed (ipm) 400"
speed_working = "24 in/min"   # published | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "Forming Speed (ipm) 24"
speed_return = "240 in/min"   # published | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "Return Speed (ipm) 240"
backgauge_axes = "standard backgage, optional 5-axis"   # published | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "a versatile 24-inch range backgage comes standard on the Xi ... optional 5-Axis backgage"
bg_x_range = "24 in range"   # published | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "a versatile 24-inch range backgage comes standard"
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "UNKNOWN"   # unknown | - | - | not found in the sources read
weight = "UNKNOWN"   # unknown | - | - | not found in the sources read
control = "24 in touchscreen PC control"   # published | https://www.e-ci.com/xi-series-press-brake | read 2026-11-24 | "24 in Touchscreen Control"
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
```

```toml
[[press]]
id = "accurpress-accell-412510"
maker = "Accurpress"
model = "Accell E 412510"
tonnage = "125 US tons"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Accell brochure Accell-EU-20200810 (Accurpress, hosted on dealer site machineryhost), ACCELL E Specifications p.22: TONS 125"
drive = "hydraulic"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Hydraulic Ram Clamping; HYDRAULIC press brake brochure context"
bending_length = "120 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "A (Length of Bed) 120 (inches)"
housing_gap = "102 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "B (Distance Between Frames) 102 (inches)"
open_height = "18 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "RAM OPEN HEIGHT 18"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "10 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "STROKE LENGTH 10"
closed_height = "8 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "RAM CLOSED HEIGHT 8"
throat = "12 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "D (Throat Depth) 12; text: 12 in Throat Depth on machines up to 250 Tons"
table_width = "4 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "TOP BED WIDTH 4"
table_height = "34 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "BED HEIGHT 34"
speed_approach = "490 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "APPROACH 490 (IN/MIN)"
speed_working = "24 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "BEND 24 (IN/MIN)"
speed_return = "320 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "RETURN 320 (IN/MIN)"
backgauge_axes = "Premium: X, R, optional Z1/Z2; Elite: X1/X2, R1/R2, Z1/Z2"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Accell E models include the Premium CNC X, R, (optional Z1/Z2) backgauge; Elite 6-axes X1/X2, R1/R2, Z1/Z2"
bg_x_range = "24 in - 39 in (609.6 - 990.6 mm) X travel (Premium)"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "PREMIUM SPECIFICATIONS Axis Travel X-Axis 24 in - 39 in 609.6 - 990.6 mm"
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "10 in (254 mm) R travel (Premium)"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "R-Axis 10 in 254 mm"
bg_z_range = "distance between side frames minus 16 in (Premium)"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Z1/Z2-Axes Distance between side frames minus 16 in"
bg_x_speed = "900 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Positioning Speed 900 ipm (X, R, Z1/Z2)"
bg_r_speed = "900 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Positioning Speed 900 ipm"
bg_z_speed = "900 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Positioning Speed 900 ipm"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "Wila hydraulic ram clamping for American tooling"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "All E-models are equipped with Wila Hydraulic Ram Clamping to suit American tooling"
overall_lwh = "C 124 in wide x E 60 in deep x F 100 in high"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "OVERALL DIMENSIONS (INCHES) C 124, E 60, F 100"
weight = "16,000 lb (approx. shipping)"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "APPROX. SHIPPING WEIGHT (LB) 16,000"
control = "Vision or ETS3000"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "sold with a CNC Accurcrown ... Vision or ETS3000 control"
crowning = "CNC Accurcrown"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Accell-E and U models are sold with a CNC Accurcrown"
# note: Column meanings from the brochure dimension drawing: A bed length, B distance between frames, C overall width, D throat depth, E depth, F height.
```

```toml
[[press]]
id = "accurpress-accell-425012"
maker = "Accurpress"
model = "Accell E 425012"
tonnage = "250 US tons"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Accell brochure Accell-EU-20200810 (Accurpress, hosted on dealer site machineryhost), ACCELL E Specifications p.22: TONS 250"
drive = "hydraulic"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Hydraulic Ram Clamping; HYDRAULIC press brake brochure context"
bending_length = "144 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "A (Length of Bed) 144 (inches)"
housing_gap = "126 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "B (Distance Between Frames) 126 (inches)"
open_height = "20 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "RAM OPEN HEIGHT 20"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "12 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "STROKE LENGTH 12"
closed_height = "8 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "RAM CLOSED HEIGHT 8"
throat = "12 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "D (Throat Depth) 12; text: 12 in Throat Depth on machines up to 250 Tons"
table_width = "5 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "TOP BED WIDTH 5"
table_height = "36 in"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "BED HEIGHT 36"
speed_approach = "320 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "APPROACH 320 (IN/MIN)"
speed_working = "24 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "BEND 24 (IN/MIN)"
speed_return = "260 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "RETURN 260 (IN/MIN)"
backgauge_axes = "Premium: X, R, optional Z1/Z2; Elite: X1/X2, R1/R2, Z1/Z2"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Accell E models include the Premium CNC X, R, (optional Z1/Z2) backgauge; Elite 6-axes X1/X2, R1/R2, Z1/Z2"
bg_x_range = "24 in - 39 in (609.6 - 990.6 mm) X travel (Premium)"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "PREMIUM SPECIFICATIONS Axis Travel X-Axis 24 in - 39 in 609.6 - 990.6 mm"
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "10 in (254 mm) R travel (Premium)"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "R-Axis 10 in 254 mm"
bg_z_range = "distance between side frames minus 16 in (Premium)"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Z1/Z2-Axes Distance between side frames minus 16 in"
bg_x_speed = "900 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Positioning Speed 900 ipm (X, R, Z1/Z2)"
bg_r_speed = "900 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Positioning Speed 900 ipm"
bg_z_speed = "900 in/min"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Positioning Speed 900 ipm"
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "Wila hydraulic ram clamping for American tooling"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "All E-models are equipped with Wila Hydraulic Ram Clamping to suit American tooling"
overall_lwh = "C 150 in wide x E 65 in deep x F 106 in high"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "OVERALL DIMENSIONS (INCHES) C 150, E 65, F 106"
weight = "27,000 lb (approx. shipping)"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "APPROX. SHIPPING WEIGHT (LB) 27,000"
control = "Vision or ETS3000"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "sold with a CNC Accurcrown ... Vision or ETS3000 control"
crowning = "CNC Accurcrown"   # published | https://s3.amazonaws.com/f.machineryhost.com/797bf842c8ce7e38d28c9c93b9d7cbe0/Accell-EU-20200810%20(2).pdf | read 2026-11-24 | "Accell-E and U models are sold with a CNC Accurcrown"
# note: Column meanings from the brochure dimension drawing: A bed length, B distance between frames, C overall width, D throat depth, E depth, F height.
```

```toml
[[press]]
id = "baileigh-bp-3305cnc"
maker = "Baileigh"
model = "BP-3305CNC"
tonnage = "33 US tons (29.9 metric t)"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Operator manual 08/2019 TECHNICAL SPECIFICATIONS: Maximum Pressure 33 tons (29.9metric tons)"
drive = "hydraulic"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Hydraulic Pump Axial Piston ... Maximum Hydraulic Pressure 3190psi (22MPa)"
bending_length = "63 in (1600 mm) table"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Table Length x Width 63 in x 7 in (1600 x 176mm)"
housing_gap = "51.3 in (1304 mm)"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Distance Between Housings 51.3 in (1304mm); dealer sheet BW: 51.25"
open_height = "11.8 in (300 mm)"   # conflict (Wurth dealer sheet BW: Distance From Table To Ram 13.325 in) | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Distance From Table To Ram 11.8 in (300mm)"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "3.9 in (100 mm)"   # conflict (dealer sheet BW: Distance Per Stroke (In.) 3.15 (matches manual's Adjustment of Ram Travel 3.1 in (80 mm))) | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Stroke Distance 3.9 in (100mm); Adjustment of Ram Travel 3.1 in (80mm)"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "7.87 in (200 mm)"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Throat Depth 7.87 in (200mm); BW: 8 in"
table_width = "7 in (176 mm)"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Table Length x Width 63 in x 7 in (1600 x 176mm)"
table_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
speed_approach = "3.5 in/s (90 mm/s)"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Approach Speed 3.5 in/sec. (90mm/sec.)"
speed_working = "0.394 in/s (10 mm/s)"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Working Speed .394 in/sec. (10mm/sec.)"
speed_return = "3.3 in/s (85 mm/s)"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Return Speed 3.3 in/sec. (85mm/sec.)"
backgauge_axes = "X (CNC), plus Y (bend depth) motor"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Back Gauge Motor (X Axis) 3/4hp; Bend Depth Motor (Y Axis) 1/8hp"
bg_x_range = "21.5 in (546 mm) back gauge length"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Back Gauge Length 21.5 in (546mm)"
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "97 x 91 x 82 in (2464 x 2312 x 2083 mm) footprint"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Footprint Dimensions 97 in x 91 in x 82 in (2464 x 2312 x 2083mm)"
weight = "4000 lb (1800 kg) shipping"   # published | https://baileigh.com/media/akeneo_connector/asset_files/B/P/BP_3305CNC_1of2_08_2019_98c3.pdf | read 2026-11-24 | "Shipping Weight 4000lbs. (1800kg)"
control = "UNKNOWN"   # unknown | - | - | not found in the sources read
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
```

```toml
[[press]]
id = "ermaksan-powerbend-pro-1270-40"
maker = "Ermaksan"
model = "Power-Bend Pro 1270-40"
tonnage = "40 Ton (metric t assumed)"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "Ermaksan Power-Bend Pro brochure (2014 PDF, hosted by dealer Prestige Equipment), Technical Features p.14: BENDING POWER 40 Ton"
drive = "hydraulic (Y1, Y2 synchronised)"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "Y1, Y2, X, R + motorized crowning; OIL CAPACITY lt."
bending_length = "1270 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "BENDING LENGTH (A) 1270 mm"
housing_gap = "1050 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "DISTANCE BETWEEN COLUMNS 1050 mm"
open_height = "387 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "DAYLIGHT (D) 387 mm"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "170 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "STROKE (C) 170 mm"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "350 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "THROAT DEPTH (E) 350 mm"
table_width = "90 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "TABLE WIDTH (G) 90 mm"
table_height = "850 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "TABLE HEIGHT (F) 850 mm"
speed_approach = "140 mm/s"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "Y RAPID SPEED 140 mm/s"
speed_working = "17 mm/s"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "Y WORKING SPEED* 17 mm/s (*max 10 mm/s on CE machines, EN12622)"
speed_return = "170 mm/s"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "Y RETURN SPEED 170 mm/s"
backgauge_axes = "X motorized, R manual (3-axis machine Y1,Y2,X); 4- and 6-axis options"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "3 Axes (R Manual) CNC Press Brake; motorized X axis back gauge and manual R axis as standard; options 4 axis (X, R, Z1, Z2), 6 axis (X, R, Z1, Z2, X1, X2)"
bg_x_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_travel = "800 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "TRAVEL IN X AXIS 800 mm"
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "230 mm/s"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "SPEED OF X AXIS 230 mm/s"
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "2"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "BACKGAUGE FINGER BLOCKS Amount 2"
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "2150 x 1650 x 2300 mm (L x W x H)"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "LENGTH (L) 2150, WIDTH (W) 1650, HEIGHT (H) 2300 mm"
weight = "3050 kg"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "WEIGHT 3050 kg."
control = "UNKNOWN"   # unknown | - | - | not found in the sources read
crowning = "none (-)"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "CROWNING none (-)"
```

```toml
[[press]]
id = "ermaksan-powerbend-pro-3100-400"
maker = "Ermaksan"
model = "Power-Bend Pro 3100-400"
tonnage = "400 Ton (metric t assumed)"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "Ermaksan Power-Bend Pro brochure (2014 PDF, hosted by dealer Prestige Equipment), Technical Features p.14: BENDING POWER 400 Ton"
drive = "hydraulic (Y1, Y2 synchronised)"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "Y1, Y2, X, R + motorized crowning; OIL CAPACITY lt."
bending_length = "3100 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "BENDING LENGTH (A) 3100 mm"
housing_gap = "2550 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "DISTANCE BETWEEN COLUMNS 2550 mm"
open_height = "650 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "DAYLIGHT (D) 650 mm"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "375 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "STROKE (C) 375 mm"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "510 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "THROAT DEPTH (E) 510 mm"
table_width = "240 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "TABLE WIDTH (G) 240 mm"
table_height = "1000 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "TABLE HEIGHT (F) 1000 mm"
speed_approach = "110 mm/s"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "Y RAPID SPEED 110 mm/s"
speed_working = "8,4 mm/s"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "Y WORKING SPEED* 8,4 mm/s (*max 10 mm/s on CE machines, EN12622)"
speed_return = "130 mm/s"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "Y RETURN SPEED 130 mm/s"
backgauge_axes = "X motorized, R manual (3-axis machine Y1,Y2,X); 4- and 6-axis options"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "3 Axes (R Manual) CNC Press Brake; motorized X axis back gauge and manual R axis as standard; options 4 axis (X, R, Z1, Z2), 6 axis (X, R, Z1, Z2, X1, X2)"
bg_x_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_travel = "1000 mm"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "TRAVEL IN X AXIS 1000 mm"
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "230 mm/s"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "SPEED OF X AXIS 230 mm/s"
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "2"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "BACKGAUGE FINGER BLOCKS Amount 2"
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "4550 x 2650 x 3470 mm (L x W x H)"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "LENGTH (L) 4550, WIDTH (W) 2650, HEIGHT (H) 3470 mm"
weight = "20840 kg"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "WEIGHT 20840 kg."
control = "UNKNOWN"   # unknown | - | - | not found in the sources read
crowning = "Motorized"   # published | https://www.prestigeequipment.com/wp-content/uploads/2017/06/POWERBENDPRO-ENG.pdf | read 2026-11-24 | "CROWNING Motorized"
```

```toml
[[press]]
id = "durma-ad-r-30135"
maker = "Durma"
model = "AD-R 30135"
tonnage = "135 Ton (metric t assumed)"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Durma AD-R Serisi Teknik Detaylar (Durma catalogue 2018, hosted by US dealer Productivity Inc.) pp.16-17: Bending Force (Ton) 135"
drive = "hydraulic"   # estimate (table gives motor power 15 kW and Y1/Y2 speeds but does not name the drive; AD-R is sold as a hydraulic press brake) | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Motor Power 15"
bending_length = "3050 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Bending Length (mm) A 3050"
housing_gap = "2600 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Distence Between Columns (mm) B 2600"
open_height = "530 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Daylight (mm) D 530"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "265 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Stroke (mm) C 265"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "410 mm (* 750 mm option)"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Throat Depth (mm) E 410; Optional Throat Depth * 750 mm option"
table_width = "104 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Table Width (mm) G 104"
table_height = "900 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Table Height (mm) F 900"
speed_approach = "160 mm/s"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Y Rapid Speed (mm/sec) 160"
speed_working = "10 mm/s"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Y Working Speed (mm/sec) 10"
speed_return = "120 mm/s"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Y Return Speed (mm/sec) 120"
backgauge_axes = "X, R (manual or motorized)"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "X Axes Working Speed; R Axes Working Distence (Manuel) 140; (Motorized) 250"
bg_x_range = "650 mm standard (1000 mm option)"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "X Axes Distance 650 S, 1000 O (S standard, O option)"
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "140 mm manual / 250 mm motorized"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "R Axes Working Distence (Manuel) 140; (Motorized) 250"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "500 mm/s"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "X Axes Working Speed (mm/sec)*** 500"
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "4200 x 1680 x 2750 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Length (mm) L 4200; Width (mm) W 1680; Height (mm) H 2750"
weight = "9170 kg"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Weight Approx. (kg) 9170"
control = "UNKNOWN"   # unknown | - | - | not found in the sources read
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
```

```toml
[[press]]
id = "durma-ad-r-40320"
maker = "Durma"
model = "AD-R 40320"
tonnage = "320 Ton (metric t assumed)"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Durma AD-R Serisi Teknik Detaylar (Durma catalogue 2018, hosted by US dealer Productivity Inc.) pp.16-17: Bending Force (Ton) 320"
drive = "hydraulic"   # estimate (table gives motor power 37 kW and Y1/Y2 speeds but does not name the drive; AD-R is sold as a hydraulic press brake) | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Motor Power 37"
bending_length = "4050 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Bending Length (mm) A 4050"
housing_gap = "3600 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Distence Between Columns (mm) B 3600"
open_height = "630 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Daylight (mm) D 630"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "365 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Stroke (mm) C 365"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "410 mm (* 750 mm option)"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Throat Depth (mm) E 410; Optional Throat Depth * 750 mm option"
table_width = "154 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Table Width (mm) G 154"
table_height = "900 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Table Height (mm) F 900"
speed_approach = "160 mm/s"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Y Rapid Speed (mm/sec) 160"
speed_working = "10 mm/s"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Y Working Speed (mm/sec) 10"
speed_return = "140 mm/s"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Y Return Speed (mm/sec) 140"
backgauge_axes = "X, R (manual or motorized)"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "X Axes Working Speed; R Axes Working Distence (Manuel) 140; (Motorized) 250"
bg_x_range = "650 mm standard (1000 mm option)"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "X Axes Distance 650 S, 1000 O (S standard, O option)"
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "140 mm manual / 250 mm motorized"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "R Axes Working Distence (Manuel) 140; (Motorized) 250"
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "500 mm/s"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "X Axes Working Speed (mm/sec)*** 500"
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "UNKNOWN"   # unknown | - | - | not found in the sources read
overall_lwh = "5300 x 1910 x 3230 mm"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Length (mm) L 5300; Width (mm) W 1910; Height (mm) H 3230"
weight = "20040 kg"   # published | https://www.productivity.com/wp-content/uploads/2023/03/PIC-Durma-AD-R-Series.pdf | read 2026-11-24 | "Weight Approx. (kg) 20040"
control = "UNKNOWN"   # unknown | - | - | not found in the sources read
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
```

```toml
[[press]]
id = "safandarley-e-brake-35-1250"
maker = "SafanDarley"
model = "E-Brake 35-1250 Premium"
tonnage = "39 US ton"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "E-Brake 35T-130T Premium brochure 10-2018 US, Technical specifications p.19: Pressing force in US Ton 39"
drive = "electric servo, belt-pulley (E-Brake)"   # published | https://cdn.shopify.com/s/files/1/0638/7833/2592/files/E-Brake35-130T-E092013_INT.pdf | read 2026-11-24 | "The flexible belts, which are only 3 mm thick and 50 mm wide (INT 2013 brochure); E-Brake = electric"
bending_length = "50 in"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Working length in inches 50"
housing_gap = "UNKNOWN"   # unknown | - | - | not found in the sources read
open_height = "23 in (Q-dimension)"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Q-dimension in inches 23; text: standard Q size (Opening) is 23 inches, option 26, 27 or 31 inches"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "12 in"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Maximum stroke in inches 12"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "36.6 in"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "side-view drawing: 36.6 (inches) from floor to table, depth 71.6"
speed_approach = "449 in/min"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Closing speed in inch/min 449"
speed_working = "48 in/min (CE: 24 in/min)"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Bending speed max.* in inch/min 48*; * CE version: Max. bending speed 24 inch/min"
speed_return = "449 in/min"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Return speed in inch/min 449"
backgauge_axes = "X, R standard; Z1+Z2 and Delta X optional"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "CNC-controlled back gauge (X-axis) ... CNC-controlled R- axis; options CNC-controlled Z1-Z2 axis, CNC-controlled Delta X-axis"
bg_x_range = "reach 1000 mm (INT 2013)"   # published | https://cdn.shopify.com/s/files/1/0638/7833/2592/files/E-Brake35-130T-E092013_INT.pdf | read 2026-11-24 | "CNC-controlled X- and R-axes come as standard, as well as hinging backgauge pins and a reach of 1000 mm; positioning speed of 350 mm/sec"
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "350 mm/s (INT 2013)"   # published | https://cdn.shopify.com/s/files/1/0638/7833/2592/files/E-Brake35-130T-E092013_INT.pdf | read 2026-11-24 | "positioning speed of 350 mm/sec"
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "New Standard Pro MC mechanical (std); NSCL-I HC hydraulic, Wila or European style optional"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "the E-Brake comes with the New Standard Pro MC mechanical tool clamp system ... NSCL-I HC Premium hydraulic ... Wila clamps or the European Style"
overall_lwh = "B 98 in wide, C 99 in high, D 104 in high (to console), depth 71.6 in"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "A in inches 50, B 98, C 99, D 104 (drawing letters)"
weight = "10.800 lb"   # conflict (INT 2013 brochure (older generation): 350 kN, 3250 kg, closing/return 180 mm/s) | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Weight in lb 10.800 (European thousands separator)"
control = "EC10 (std), EC20 optional"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "EC10 controls, single split-screen display; EC20 2D-graphic CNC controls (accessory)"
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
# note: Throat depth is not published in either brochure. INT 2013 (metric, older generation) gives stroke 300 mm, Q 590 mm (option 650/690 mm), table height drawing 930 mm.
```

```toml
[[press]]
id = "safandarley-e-brake-100-3100"
maker = "SafanDarley"
model = "E-Brake 100-3100 Premium"
tonnage = "110 US ton"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "E-Brake 35T-130T Premium brochure 10-2018 US, Technical specifications p.19: Pressing force in US Ton 110"
drive = "electric servo, belt-pulley (E-Brake)"   # published | https://cdn.shopify.com/s/files/1/0638/7833/2592/files/E-Brake35-130T-E092013_INT.pdf | read 2026-11-24 | "The flexible belts, which are only 3 mm thick and 50 mm wide (INT 2013 brochure); E-Brake = electric"
bending_length = "120 in"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Working length in inches 120"
housing_gap = "UNKNOWN"   # unknown | - | - | not found in the sources read
open_height = "23 in (Q-dimension)"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Q-dimension in inches 23; text: standard Q size (Opening) is 23 inches, option 26, 27 or 31 inches"
table_to_beam = "UNKNOWN"   # unknown | - | - | not found in the sources read
stroke = "12 in"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Maximum stroke in inches 12"
closed_height = "UNKNOWN"   # unknown | - | - | not found in the sources read
throat = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_width = "UNKNOWN"   # unknown | - | - | not found in the sources read
table_height = "36.6 in"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "side-view drawing: 36.6 (inches) from floor to table, depth 71.6"
speed_approach = "177 in/min"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Closing speed in inch/min 177"
speed_working = "48 in/min (CE: 24 in/min)"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Bending speed max.* in inch/min 48*; * CE version: Max. bending speed 24 inch/min"
speed_return = "177 in/min"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Return speed in inch/min 177"
backgauge_axes = "X, R standard; Z1+Z2 and Delta X optional"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "CNC-controlled back gauge (X-axis) ... CNC-controlled R- axis; options CNC-controlled Z1-Z2 axis, CNC-controlled Delta X-axis"
bg_x_range = "reach 1000 mm (INT 2013)"   # published | https://cdn.shopify.com/s/files/1/0638/7833/2592/files/E-Brake35-130T-E092013_INT.pdf | read 2026-11-24 | "CNC-controlled X- and R-axes come as standard, as well as hinging backgauge pins and a reach of 1000 mm; positioning speed of 350 mm/sec"
bg_x_travel = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_r_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_range = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_x_speed = "350 mm/s (INT 2013)"   # published | https://cdn.shopify.com/s/files/1/0638/7833/2592/files/E-Brake35-130T-E092013_INT.pdf | read 2026-11-24 | "positioning speed of 350 mm/sec"
bg_r_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_z_speed = "UNKNOWN"   # unknown | - | - | not found in the sources read
bg_fingers = "UNKNOWN"   # unknown | - | - | not found in the sources read
clamping = "New Standard Pro MC mechanical (std); NSCL-I HC hydraulic, Wila or European style optional"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "the E-Brake comes with the New Standard Pro MC mechanical tool clamp system ... NSCL-I HC Premium hydraulic ... Wila clamps or the European Style"
overall_lwh = "B 171 in wide, C 99 in high, D 104 in high (to console), depth 71.6 in"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "A in inches 120, B 171, C 99, D 104 (drawing letters)"
weight = "16.535 lb"   # conflict (INT 2013 brochure (older generation): 1000 kN, 6350 kg, closing/return 75 mm/s) | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "Weight in lb 16.535 (European thousands separator)"
control = "EC10 (std), EC20 optional"   # published | https://us.safandarley.com/wp-content/uploads/2024/05/E-Brake-35T-130T-Premium-102018-US.pdf | read 2026-11-24 | "EC10 controls, single split-screen display; EC20 2D-graphic CNC controls (accessory)"
crowning = "UNKNOWN"   # unknown | - | - | not found in the sources read
# note: Throat depth is not published in either brochure. INT 2013 (metric, older generation) gives stroke 300 mm, Q 590 mm (option 650/690 mm), table height drawing 930 mm.
```
## 3. Gaps

- Amada: amada.eu (pages and cdn brochures) blocks automated reads (Cloudflare 403); the Amada America
  e-book is obfuscated. Only the Japanese-market HG and HG-1003ATC catalogues were readable. No
  throat depth for any Amada model; EG, HRB and HD series not found in any readable maker source
  (Amada JP product ids 1-260 checked; only HG, HG-ATC and HG robot cells exist there). Search-engine
  snippets attribute 450 mm (web) / 455 mm (brochure) throat and 520 mm open height (400 mm with
  holders) to amada.eu; these could not be verified and are not used.
- HG-1003ATC: open height, frame gap and table height are interpretations of unlabelled drawing
  dimensions (601, 3143, 984 mm): estimates.
- Bystronic: Xact / Xact Smart, Beyeler and older Xpert 40 data not readable (directindustry 403; no
  bystronic.com tech-data PDF found). ByBend Star page gives only tonnage, length, open height, stroke,
  speeds. Xpert Pro sheet has no table height, Y speeds, clamping, crowning or control name.
- Trumpf: no tool-clamp type, table width or crowning in the tables; backgauge axis names not given
  (only "up to 6 axes"). TruBend 3000: the brochure (202402) and the current web page describe
  different generations; throat depth printed as a dash. Brochure vs web conflicts on weights and
  heights (recorded per machine).
- LVD: no R-axis or Z-axis ranges, no X travel figure (only "gauging to 1000 mm with material support");
  no overall length/width for Easy-Form/PPEB; Easy-Form page shows no control/clamping per model.
- Cincinnati: no distance between housings, backgauge ranges (except Xi 24 in), dimensions (except
  GOFORM shipping weight); tonnage is read from the model number (estimate).
- Accurpress: values from a 2020 brochure copy on a dealer host; accurpress.com not searched.
- Baileigh: maker manual and dealer sheet disagree on table-to-ram (11.8 vs 13.325 in) and stroke
  (3.9 vs 3.15 in); no R axis.
- Ermaksan, Durma: catalogues are 2014 and 2018 copies on US dealer hosts; tonnage unit printed only
  as "Ton" (metric t assumed, marked); Ermaksan R axis manual, range not given; Durma Z axis absent.
- SafanDarley: no throat depth in either brochure; E-Brake stroke 12 in (US 2018) vs 300 mm (INT 2013);
  drawing letters A-D not all defined.
- Across all makers: backgauge finger geometry (depth, height, width) and ram/bed cross-section
  envelopes (front/back offsets of the beams from the tool line) are not published in any source read;
  closed height is published only by Cincinnati and Accurpress.
