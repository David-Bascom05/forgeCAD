# Swiss-type (sliding headstock) lathes: published specs (MACHINE-LIB C)

Read 2026-11-23 from the makers' own pages; restated, not copied. Held here, not in the machine
library, until the format can say a guide bushing, a gang slide and several paths
(FORMAT-GAPS row 10) and because per-axis travels are unpublished for nearly every model.
"—" = not published on the pages read (UNKNOWN). Where a maker's pages disagree, both are given.

Sources (keys): CMJ = cmj.citizen.co.jp/english/product/cincom/cincom_{l20b5,l32,a20,m32}.html;
MCC = marucit.com (L20 page; L32 2019 brochure PDF; A20 2016 brochure PDF); DE = citizen.de L20
brochure PDF; STAR = star-m.jp/eng/products/lathe/swiss_type/<model>.html; STAR-US =
starcnc.com/product/<model>/ (**fetched with a browser User-Agent before the no-spoofing rule
reached the research agent: values marked † must be re-sourced before use**); TA =
tsugamiamerica.com machine pages and Focus brochure PDFs; NOM = nomurads.co.jp (the official
site; nomura-ds.co.jp is now unrelated).

## Citizen (Cincom)

| model | bar (main/sub) | stroke per chucking | axes | main / sub spindle | rapids | control | mass |
|---|---|---|---|---|---|---|---|
| L20 (VIII/IX/X/XII) | 20 (25 opt) / 20 | GB 200 (188 at 25 mm); GBL 50 | X1 Y1 Z1 X2 Z2; Y2 on X, XII; B on XII gang | 10,000 rpm 2.2/3.7/5.5 kW; 10,000 rpm 1.5/2.2/3.7 kW | 32 m/min (Y2 8) | Mitsubishi M820VW (M850LUC-V on XIIB5) [DE] | 2,630–2,680 kg |
| L32 (VIII/IX/X/XII) | 32 (38 opt) / 32 | GB 320; GBL 2.5 D | as L20; X1 stroke 124 (tooling drawing) [MCC] | 8,000 3.7/7.5 kW; 8,000 2.2/3.7 kW | 32 (Y2 24) | Cincom System M70LPC-VU (Mitsubishi) [MCC 2019] | 2,900 kg |
| A20 | 20 / 20 | 200, Z1 stroke 205 (188/193 at 25 mm) | X1 X2 Y1 Z1 Z2 | 10,000 (8,000 pull chuck) 2.2/3.7; 8,000 1.1/1.5 | X1 24 (18 in 2016 PDF), others 32 | "dedicated NC unit" | 2,200 kg |
| M32 (V/VII/VIII) | 32 (38 opt) / 32 | 320 | X1 Y1 Z1 Z2 X3 Z3 X2 Y2 (+Y3, B1 on VIII); 10-station turret | 8,000 5.5/7.5 both | 32 (X2 18, Y2 12) | — | 4,300 kg |

Tools (L20): gang 6 turning 12 mm (13/16), gang rotary 21–25 at 6,000 rpm 1.0 kW (S3 option
9,000 rpm 2.2 kW; DE brochure lists 9,000/2.2 for all), front drilling 3–6, back drilling 4–8;
guide bushing switchable GB/GBL, synchronous GB standard [CMJ, DE]. Max tool counts disagree
between CMJ (38/34/45/41), MCC (38/45/46) and DE (20+α … 31+α).

## Star (FANUC on the US pages †; star-m.jp gives the same headline values unless noted)

| model | bar | headstock stroke (GB / RMGB / non-GB) | axes, paths | main / sub | rapids | control | mass |
|---|---|---|---|---|---|---|---|
| SR-20J II A/B | 20 | 205 / 160 / 50 | X1 Y1 Z1 X2 Z2 C1 C2 (+Y2 on B); 2 paths | 10,000 3.7/2.2 both | 35 m/min (Y2 24) | FANUC 32i-B † | 2,750 kg |
| SR-32J III A/B | 32 | 320 / 286.5 / 80 | as SR-20J II | 8,000 7.5/11; 8,000 3.7/5.5 | 35 (Y2 24) | FANUC 32i-B († and brochure p7) | 4,100 kg |
| SR-20R IV type E | 20 | 205 / 160 / 2.5 D ≤ 50 | +B1; 5 + 4 axes | 10,000 3.7/2.2 both | 35 (Y2 15; B1 27,000°/min) | FANUC 31i-B | 2,600 kg |
| SB-20R II | 20 (25.4 opt) | 205 (196 MR25) / — / 2.5 D ≤ 50 | 4 + 3 | 10,000 3.7/2.2; 10,000 2.2/1.5 | 35 | FANUC 0i-TF Plus or Mitsubishi M80V (brochure) | 2,000 kg |
| SW-20 | 20 | 205 | 11 axes, 3 paths (4/4/3) | 10,000 2.2/3.7 both | 35 / 20 per axis | FANUC 31i-B5 † | 3,400 kg |

C1/C2 rapid 198,000°/min †. SB-16 is discontinued (SB-16R Type G).

## Tsugami (convertible guide bushing / chucker on all)

| model | bar | stroke (GB / no GB) | axes | main / back | rapids | control | mass |
|---|---|---|---|---|---|---|---|
| B0205-VR | 20 / 20 | 170 rotary DD GB (210 stationary, 80 carrier) / 45 | X1 Z1 Y1 X2 Z2 | 10,000 3.7/5.5; 12,000 2.2/3.7 | X2 Z2 36, Y1 Z1 32, X1 24 m/min | FANUC 0i-TF Plus (brochure) vs 0i-TF (page) | 1,800 kg |
| B0206-VR | 20 | as B0205 | + Y2 | as B0205 | 36 m/min | FANUC 0i-TF Plus | 1,850 kg |
| B0326-III | 32 / 32 | 330 / 70 | X1 Z1 Y1 X2 Y2 Z2 C1 C2; 2 paths | 200–8,000 3.7/5.5 both | 35 (X1 Y1 Y2 24) | FANUC 32i-B | 4,000 kg |
| B0386-III | 38 | 320 / 70 (brochure also 330) | as B0326 | 200–6,000 7.5/11; 200–7,000 3.7/5.5 | 32 (X1 Y1 Z1 Y2 24) | FANUC 32i-B | 4,850 kg |
| SS20 / SS26 | 20 / 26 | 220 / 45; 270 / 50 | — | 200–10,000; 2.2/3.7 / 3.7/5.5 kW | — | FANUC 32i-B | 3,400 kg |

## Nomura DS

| model | bar | stroke | spindles | rapids | resolution | control | mass |
|---|---|---|---|---|---|---|---|
| NN-20UB8 | 20 (25 opt) | 200 | 10,000 / 6,000 rpm; 2.2/3.7 kW, back 0.75 kW | 30 m/min | 0.0001 mm, C 0.0001° | Mitsubishi M70A | 2,135 kg |
| NN-32YB5 | 32 | 190 | 8,000 both; 3.7/5.5, 1.5/3.7 kW | Z1 Y1 Z2 Y2 28, X1 20, X2 25 m/min; C 144,000°/min | 0.0001 mm, C 0.0001° | Mitsubishi M80 | 3,030 kg |
| NN-10EX3 | 11 / 10 | RB 110 / FB 155 | 10,000 both; 1.5/3.7, 0.75/1.1 kW | X1 Y2 14, Y1 Z1 X2 Z2 30 m/min | 0.0001 mm, C 0.0001° | Mitsubishi M80 | 1,960 kg |

## What a library record still needs (all UNKNOWN from the makers' pages)
Per-axis travels (X1, Y1, X2, Y2, Z2 …), the gang slide's tool spacing, guide-bushing face to
gang centre, back post geometry, C-axis index resolution (Citizen, Star), channel wait-code sets.
These are in operator/programming manuals, not public pages.
