# The 52 bits.txt lines the sound Interval::sin/cos moved

The branch moves 52 lines (seq and nary) of tools/speed/bits.txt, from 8250ebb (this branch's base)
to 244408c4. This table compares each part's result on the two builds, made by
`bits measures` and `benchparts measures`: validity (forge_topo::check plus a closed mesh), the
meshed volume and area (chord 0.02, max angle 0.5), and the face count.

**Why each moved.** A rebuild with the region snap switched off gives the attribution:
- *sound boxes*: the part still differs from base with the snap off;
- *snapped region*: it matches base with the snap off;
- *both*: it differs either way, and the two changes stack.

The sound boxes are the absolute 2^-52 widening of sin and cos (an arc that ends on a plane now has
a box that touches it). The snapped region is ssi::searched's region pushed out to a 2^-36 grid, so
a pair is traced on a hair-larger region and its chain points move.

**Result.**
- All 52 are valid on both builds, and every face count is unchanged.
- Volume and area agree to within 3e-8 relative, with one exception, frame.
- frame: seq −1.45 mm³ (3.4e-6) and area +0.50 mm²; nary −1.25 mm³. This is the mesh, not the
  geometry. Remeshed finer, the gap between base and head shrinks with the chord, while the volume
  converges to 422164.75:

  | chord | seq gap | nary gap | base's own seq − nary |
  |---|---|---|---|
  | 0.02 | 1.447 | 1.255 | 0.360 |
  | 0.005 | 0.259 | 0.256 | 0.138 |
  | 0.001 | 0.017 | 0.017 (sign flips) | 0.034 |

  At chord 0.001 the change is smaller than base's own seq/nary difference. frame's traced curves
  differ by their chain points (an IntCurve is exact by its Newton step), and its faces mesh
  differently along them.
- **Exact reference** (verify/forge-exact). It applies to polyhedral parts only, and every changed
  part has curved faces whose edges came from the changed boxes or traces, so it is not applicable
  here.
- Scoreboard 0 worse; FUZZ_SEEDS=150 at 150/150.

| part | path | valid base/head | volume base | volume head | Δvol (rel) | area base | area head | Δarea (rel) | faces base/head | why |
|---|---|---|---|---|---|---|---|---|---|---|
| plate | seq | valid/valid | 1398311.002046 | 1398311.002046 | +0.000e+00 (+0.0e+00) | 206094.437722 | 206094.437722 | +0.000e+00 (+0.0e+00) | 510/510 | snapped region |
| plate | nary | valid/valid | 1398311.001930 | 1398311.001930 | +0.000e+00 (+0.0e+00) | 206094.437578 | 206094.437578 | +0.000e+00 (+0.0e+00) | 544/544 | snapped region |
| shaft | seq | valid/valid | 270989.873441 | 270989.875066 | +1.625e-03 (+6.0e-09) | 68921.335128 | 68921.333379 | -1.749e-03 (-2.5e-08) | 664/664 | sound boxes + snapped region |
| shaft | nary | valid/valid | 270990.087019 | 270990.086898 | -1.210e-04 (-4.5e-10) | 68921.958292 | 68921.958275 | -1.700e-05 (-2.5e-10) | 697/697 | sound boxes + snapped region |
| housing | seq | valid/valid | 431422.446704 | 431422.446704 | +0.000e+00 (+0.0e+00) | 241368.859973 | 241368.859973 | +0.000e+00 (+0.0e+00) | 648/648 | snapped region |
| housing | nary | valid/valid | 431422.450787 | 431422.450787 | +0.000e+00 (+0.0e+00) | 241368.858995 | 241368.858995 | +0.000e+00 (+0.0e+00) | 681/681 | snapped region |
| mixed | seq | valid/valid | 850720.884570 | 850720.884587 | +1.700e-05 (+2.0e-11) | 126511.618197 | 126511.618197 | +0.000e+00 (+0.0e+00) | 743/743 | snapped region |
| mixed | nary | valid/valid | 850720.921633 | 850720.921647 | +1.400e-05 (+1.6e-11) | 126511.241908 | 126511.241908 | +0.000e+00 (+0.0e+00) | 743/743 | sound boxes + snapped region |
| sweep | seq | valid/valid | 196500.828276 | 196500.828276 | +0.000e+00 (+0.0e+00) | 29290.003929 | 29290.003929 | +0.000e+00 (+0.0e+00) | 27/27 | snapped region |
| sweep | nary | valid/valid | 196500.828276 | 196500.828276 | +0.000e+00 (+0.0e+00) | 29290.003929 | 29290.003929 | +0.000e+00 (+0.0e+00) | 34/34 | snapped region |
| coil | seq | valid/valid | 44700.785558 | 44700.785851 | +2.930e-04 (+6.6e-09) | 23573.712954 | 23573.712954 | +0.000e+00 (+0.0e+00) | 37/37 | snapped region |
| coil | nary | valid/valid | 44700.785558 | 44700.785851 | +2.930e-04 (+6.6e-09) | 23573.712954 | 23573.712954 | +0.000e+00 (+0.0e+00) | 37/37 | snapped region |
| loft | seq | valid/valid | 102728.510076 | 102728.507028 | -3.048e-03 (-3.0e-08) | 19976.162969 | 19976.162988 | +1.900e-05 (+9.5e-10) | 27/27 | snapped region |
| loft | nary | valid/valid | 102728.510076 | 102728.507028 | -3.048e-03 (-3.0e-08) | 19976.162969 | 19976.162988 | +1.900e-05 (+9.5e-10) | 27/27 | snapped region |
| torture-2 | seq | valid/valid | 126336.521483 | 126336.521483 | +0.000e+00 (+0.0e+00) | 82463.882451 | 82463.882451 | +0.000e+00 (+0.0e+00) | 164/164 | sound boxes + snapped region |
| torture-2 | nary | valid/valid | 126336.521426 | 126336.521426 | +0.000e+00 (+0.0e+00) | 82463.882469 | 82463.882469 | +0.000e+00 (+0.0e+00) | 164/164 | sound boxes + snapped region |
| torture | seq | valid/valid | 1398311.002046 | 1398311.002046 | +0.000e+00 (+0.0e+00) | 206094.437722 | 206094.437722 | +0.000e+00 (+0.0e+00) | 510/510 | snapped region |
| torture | nary | valid/valid | 1398311.001930 | 1398311.001930 | +0.000e+00 (+0.0e+00) | 206094.437578 | 206094.437578 | +0.000e+00 (+0.0e+00) | 544/544 | snapped region |
| fuzz1 | seq | valid/valid | 751535.424716 | 751535.424716 | +0.000e+00 (+0.0e+00) | 96001.497591 | 96001.497591 | +0.000e+00 (+0.0e+00) | 253/253 | snapped region |
| fuzz1 | nary | valid/valid | 751535.412550 | 751535.412550 | +0.000e+00 (+0.0e+00) | 96001.484502 | 96001.484502 | +0.000e+00 (+0.0e+00) | 259/259 | snapped region |
| fuzz2 | seq | valid/valid | 599736.991426 | 599736.991430 | +4.000e-06 (+6.7e-12) | 113451.130047 | 113451.130047 | +0.000e+00 (+0.0e+00) | 306/306 | snapped region |
| fuzz2 | nary | valid/valid | 599737.041658 | 599737.041658 | +0.000e+00 (+0.0e+00) | 113451.082847 | 113451.082847 | +0.000e+00 (+0.0e+00) | 309/309 | sound boxes + snapped region |
| fuzz3 | seq | valid/valid | 243841.516191 | 243841.516191 | +0.000e+00 (+0.0e+00) | 72195.098086 | 72195.098086 | +0.000e+00 (+0.0e+00) | 356/356 | sound boxes + snapped region |
| fuzz3 | nary | valid/valid | 243841.512911 | 243841.512911 | +0.000e+00 (+0.0e+00) | 72195.087167 | 72195.087167 | +0.000e+00 (+0.0e+00) | 359/359 | sound boxes + snapped region |
| fuzz4 | seq | valid/valid | 74189.836777 | 74189.836777 | +0.000e+00 (+0.0e+00) | 27132.466367 | 27132.466367 | +0.000e+00 (+0.0e+00) | 344/344 | snapped region |
| fuzz4 | nary | valid/valid | 74189.842483 | 74189.842483 | +0.000e+00 (+0.0e+00) | 27132.442075 | 27132.442075 | +0.000e+00 (+0.0e+00) | 352/352 | sound boxes + snapped region |
| fuzz5 | seq | valid/valid | 243413.448801 | 243413.448801 | +0.000e+00 (+0.0e+00) | 47088.362944 | 47088.362944 | +0.000e+00 (+0.0e+00) | 366/366 | snapped region |
| fuzz5 | nary | valid/valid | 243413.447783 | 243413.447783 | +0.000e+00 (+0.0e+00) | 47088.360227 | 47088.360227 | +0.000e+00 (+0.0e+00) | 369/369 | sound boxes + snapped region |
| fuzz6 | seq | valid/valid | 431424.320290 | 431424.320290 | +0.000e+00 (+0.0e+00) | 65343.979714 | 65343.979714 | +0.000e+00 (+0.0e+00) | 297/297 | snapped region |
| fuzz6 | nary | valid/valid | 431424.315861 | 431424.315861 | +0.000e+00 (+0.0e+00) | 65343.960147 | 65343.960147 | +0.000e+00 (+0.0e+00) | 304/304 | sound boxes + snapped region |
| fuzz7 | seq | valid/valid | 494644.195482 | 494644.195482 | +0.000e+00 (+0.0e+00) | 73530.366259 | 73530.366259 | +0.000e+00 (+0.0e+00) | 304/304 | sound boxes + snapped region |
| fuzz7 | nary | valid/valid | 494644.203544 | 494644.203544 | +0.000e+00 (+0.0e+00) | 73530.366282 | 73530.366282 | +0.000e+00 (+0.0e+00) | 308/308 | snapped region |
| fuzz8 | seq | valid/valid | 507845.532775 | 507845.532775 | +0.000e+00 (+0.0e+00) | 74440.239528 | 74440.239528 | +0.000e+00 (+0.0e+00) | 289/289 | snapped region |
| fuzz8 | nary | valid/valid | 507845.530559 | 507845.530559 | +0.000e+00 (+0.0e+00) | 74440.244265 | 74440.244265 | +0.000e+00 (+0.0e+00) | 298/298 | sound boxes + snapped region |
| fuzz9 | seq | valid/valid | 343033.669740 | 343033.669736 | -4.000e-06 (-1.2e-11) | 95882.839815 | 95882.839815 | +0.000e+00 (+0.0e+00) | 383/383 | sound boxes + snapped region |
| fuzz9 | nary | valid/valid | 343033.630040 | 343033.630040 | +0.000e+00 (+0.0e+00) | 95882.741298 | 95882.741298 | +0.000e+00 (+0.0e+00) | 385/385 | sound boxes + snapped region |
| fuzz10 | seq | valid/valid | 94972.916579 | 94972.916576 | -3.000e-06 (-3.2e-11) | 30393.810685 | 30393.810685 | +0.000e+00 (+0.0e+00) | 382/382 | sound boxes + snapped region |
| fuzz10 | nary | valid/valid | 94972.921300 | 94972.921300 | +0.000e+00 (+0.0e+00) | 30393.803295 | 30393.803295 | +0.000e+00 (+0.0e+00) | 386/386 | sound boxes + snapped region |
| crank | seq | valid/valid | 116622.633887 | 116622.633887 | +0.000e+00 (+0.0e+00) | 28856.682824 | 28856.682824 | +0.000e+00 (+0.0e+00) | 119/119 | snapped region |
| crank | nary | valid/valid | 116622.644075 | 116622.644075 | +0.000e+00 (+0.0e+00) | 28856.683021 | 28856.683021 | +0.000e+00 (+0.0e+00) | 119/119 | sound boxes + snapped region |
| frame | seq | valid/valid | 421439.832030 | 421438.384828 | -1.447e+00 (-3.4e-06) | 498602.908538 | 498603.411212 | +5.027e-01 (+1.0e-06) | 178/178 | sound boxes + snapped region |
| frame | nary | valid/valid | 421439.471767 | 421438.216851 | -1.255e+00 (-3.0e-06) | 498603.282344 | 498603.453340 | +1.710e-01 (+3.4e-07) | 182/182 | snapped region |
| heatsink | seq | valid/valid | 82402.370660 | 82402.370660 | +0.000e+00 (+0.0e+00) | 62757.441850 | 62757.441850 | +0.000e+00 (+0.0e+00) | 438/438 | snapped region |
| heatsink | nary | valid/valid | 82402.370660 | 82402.370660 | +0.000e+00 (+0.0e+00) | 62757.441850 | 62757.441850 | +0.000e+00 (+0.0e+00) | 438/438 | snapped region |
| jack | seq | valid/valid | 54040.377013 | 54040.377004 | -9.000e-06 (-1.7e-10) | 15690.306318 | 15690.306318 | +0.000e+00 (+0.0e+00) | 333/333 | sound boxes + snapped region |
| jack | nary | valid/valid | 54040.377401 | 54040.376862 | -5.390e-04 (-1.0e-08) | 15690.306318 | 15690.306318 | +0.000e+00 (+0.0e+00) | 335/335 | sound boxes + snapped region |
| manifold | seq | valid/valid | 884324.563973 | 884324.556111 | -7.862e-03 (-8.9e-09) | 343212.180829 | 343212.180740 | -8.900e-05 (-2.6e-10) | 78/78 | snapped region |
| manifold | nary | valid/valid | 884324.563974 | 884324.556112 | -7.862e-03 (-8.9e-09) | 343212.180830 | 343212.180740 | -9.000e-05 (-2.6e-10) | 86/86 | snapped region |
| valve | seq | valid/valid | 302983.826188 | 302983.826188 | +0.000e+00 (+0.0e+00) | 46241.231219 | 46241.231219 | +0.000e+00 (+0.0e+00) | 204/204 | snapped region |
| valve | nary | valid/valid | 302983.826206 | 302983.826206 | +0.000e+00 (+0.0e+00) | 46241.231219 | 46241.231219 | +0.000e+00 (+0.0e+00) | 206/206 | snapped region |
| volute | seq | valid/valid | 802763.643266 | 802763.643188 | -7.800e-05 (-9.7e-11) | 263254.076255 | 263254.076255 | +0.000e+00 (+0.0e+00) | 70/70 | sound boxes + snapped region |
| volute | nary | valid/valid | 802763.643266 | 802763.643188 | -7.800e-05 (-9.7e-11) | 263254.076255 | 263254.076255 | +0.000e+00 (+0.0e+00) | 70/70 | sound boxes + snapped region |

## Against main, after the merge (3bd27beb against origin/main ec328182)

The merge session asked for a per-line check that the 52 lines move by rounding only: topology,
volume and verdicts unchanged. Both builds printed every body's validity, its vertex, edge and
face counts, and every vertex point and edge range to the bit. That is `bits points` and
`benchparts points`, a temporary mode added by `research/kernel-audit/bits-points-patch.py`. It
is not committed: `benchparts` is a gate binary, and the mode's code would shift its counts.

**Result.**
- **Topology and verdicts.** In all 52, the same single body is valid on both builds, with equal
  vertex, edge and face counts. Every regeneration succeeds on both.
- **Vertices.** Points move by at most 1.06e-12 mm (volute), except one vertex in frame nary,
  which moves 1.70e-10 mm. That is about 10^4 ulps of its coordinates, and far below LINEAR.
  Most vertices do not move at all.
- **Edge ranges** move by up to 8.5e-7 (shaft seq). This is a parameter, not geometry. The range
  ends sit at vertices that moved by 1e-10 mm or less, so on a line or a circle the range could
  move only by rounding. The edges that move are traced curves, whose parameter runs along their
  chain points, and those points moved (§ above: the snapped region and the sound boxes).
- **Volumes and areas** are as in the table above, against the branch base: within 3e-8
  relative, frame's being the mesh.

| part | path | main: valid V E F | head: valid V E F | vertices moved | max vertex move (mm) | max edge-range move |
|---|---|---|---|---|---|---|
| plate | seq | valid 937 1398 510 | valid 937 1398 510 | 19 of 937 | 1.45e-13 | 1.86e-07 |
| plate | nary | valid 1045 1540 544 | valid 1045 1540 544 | 7 of 1045 | 2.64e-13 | 1.92e-07 |
| shaft | seq | valid 1351 2026 664 | valid 1351 2026 664 | 359 of 1351 | 1.46e-13 | 8.48e-07 |
| shaft | nary | valid 1553 2262 697 | valid 1553 2262 697 | 235 of 1553 | 7.22e-13 | 2.86e-07 |
| housing | seq | valid 1132 1698 648 | valid 1132 1698 648 | 9 of 1132 | 1.52e-14 | 5.51e-08 |
| housing | nary | valid 1287 1886 681 | valid 1287 1886 681 | 5 of 1287 | 8.75e-14 | 6.55e-08 |
| mixed | seq | valid 1574 2297 743 | valid 1574 2297 743 | 503 of 1574 | 7.52e-13 | 2.72e-07 |
| mixed | nary | valid 1574 2297 743 | valid 1574 2297 743 | 344 of 1574 | 7.38e-13 | 2.07e-07 |
| sweep | seq | valid 43 66 27 | valid 43 66 27 | 2 of 43 | 2.25e-14 | 6.12e-08 |
| sweep | nary | valid 50 80 34 | valid 50 80 34 | 2 of 50 | 2.25e-14 | 6.12e-08 |
| coil | seq | valid 58 90 37 | valid 58 90 37 | 8 of 58 | 1.36e-14 | 1.20e-08 |
| coil | nary | valid 58 90 37 | valid 58 90 37 | 8 of 58 | 1.36e-14 | 1.20e-08 |
| loft | seq | valid 38 60 27 | valid 38 60 27 | 4 of 38 | 7.32e-15 | 1.76e-08 |
| loft | nary | valid 38 60 27 | valid 38 60 27 | 4 of 38 | 7.32e-15 | 1.76e-08 |
| torture-2 | seq | valid 228 371 164 | valid 228 371 164 | 0 of 228 | 0.00e+00 | 3.03e-08 |
| torture-2 | nary | valid 231 374 164 | valid 231 374 164 | 0 of 231 | 0.00e+00 | 2.79e-08 |
| torture | seq | valid 937 1398 510 | valid 937 1398 510 | 19 of 937 | 1.45e-13 | 1.86e-07 |
| torture | nary | valid 1045 1540 544 | valid 1045 1540 544 | 7 of 1045 | 2.64e-13 | 1.92e-07 |
| fuzz1 | seq | valid 440 652 253 | valid 440 652 253 | 30 of 440 | 2.63e-13 | 9.49e-08 |
| fuzz1 | nary | valid 509 727 259 | valid 509 727 259 | 25 of 509 | 3.71e-14 | 9.49e-08 |
| fuzz2 | seq | valid 545 811 306 | valid 545 811 306 | 85 of 545 | 2.72e-13 | 1.47e-07 |
| fuzz2 | nary | valid 637 906 309 | valid 637 906 309 | 71 of 637 | 1.91e-13 | 1.40e-07 |
| fuzz3 | seq | valid 680 1016 356 | valid 680 1016 356 | 67 of 680 | 1.14e-13 | 7.06e-08 |
| fuzz3 | nary | valid 818 1157 359 | valid 818 1157 359 | 59 of 818 | 5.89e-13 | 7.48e-08 |
| fuzz4 | seq | valid 613 924 344 | valid 613 924 344 | 37 of 613 | 4.27e-13 | 4.69e-08 |
| fuzz4 | nary | valid 757 1077 352 | valid 757 1077 352 | 45 of 757 | 4.55e-14 | 3.71e-08 |
| fuzz5 | seq | valid 643 965 366 | valid 643 965 366 | 12 of 643 | 1.43e-14 | 3.31e-08 |
| fuzz5 | nary | valid 743 1068 369 | valid 743 1068 369 | 10 of 743 | 1.71e-14 | 3.31e-08 |
| fuzz6 | seq | valid 495 738 297 | valid 495 738 297 | 20 of 495 | 5.08e-13 | 4.28e-08 |
| fuzz6 | nary | valid 593 843 304 | valid 593 843 304 | 20 of 593 | 4.26e-14 | 1.78e-07 |
| fuzz7 | seq | valid 517 768 304 | valid 517 768 304 | 19 of 517 | 1.28e-14 | 5.36e-08 |
| fuzz7 | nary | valid 584 839 308 | valid 584 839 308 | 13 of 584 | 1.46e-14 | 3.25e-08 |
| fuzz8 | seq | valid 481 716 289 | valid 481 716 289 | 11 of 481 | 2.95e-14 | 2.88e-08 |
| fuzz8 | nary | valid 550 794 298 | valid 550 794 298 | 10 of 550 | 2.95e-14 | 2.33e-08 |
| fuzz9 | seq | valid 722 1081 383 | valid 722 1081 383 | 68 of 722 | 8.70e-14 | 5.87e-08 |
| fuzz9 | nary | valid 890 1251 385 | valid 890 1251 385 | 52 of 890 | 6.89e-13 | 1.16e-07 |
| fuzz10 | seq | valid 709 1058 382 | valid 709 1058 382 | 48 of 709 | 3.04e-14 | 1.15e-07 |
| fuzz10 | nary | valid 849 1202 386 | valid 849 1202 386 | 48 of 849 | 3.04e-14 | 6.25e-08 |
| crank | seq | valid 203 314 119 | valid 203 314 119 | 9 of 203 | 3.14e-14 | 7.07e-08 |
| crank | nary | valid 203 314 119 | valid 203 314 119 | 9 of 203 | 3.07e-14 | 2.53e-08 |
| frame | seq | valid 325 505 178 | valid 325 505 178 | 35 of 325 | 8.10e-14 | 1.07e-07 |
| frame | nary | valid 352 536 182 | valid 352 536 182 | 39 of 352 | 1.70e-10 | 6.27e-08 |
| heatsink | seq | valid 756 1168 438 | valid 756 1168 438 | 2 of 756 | 8.88e-16 | 2.83e-08 |
| heatsink | nary | valid 758 1170 438 | valid 758 1170 438 | 3 of 758 | 8.88e-16 | 2.83e-08 |
| jack | seq | valid 626 940 333 | valid 626 940 333 | 4 of 626 | 2.77e-14 | 3.02e-08 |
| jack | nary | valid 684 1000 335 | valid 684 1000 335 | 8 of 684 | 7.14e-14 | 3.02e-08 |
| manifold | seq | valid 121 192 78 | valid 121 192 78 | 9 of 121 | 1.42e-14 | 1.25e-08 |
| manifold | nary | valid 142 221 86 | valid 142 221 86 | 9 of 142 | 1.42e-14 | 1.25e-08 |
| valve | seq | valid 343 524 204 | valid 343 524 204 | 1 of 343 | 5.33e-15 | 1.26e-08 |
| valve | nary | valid 353 536 206 | valid 353 536 206 | 0 of 353 | 0.00e+00 | 1.23e-08 |
| volute | seq | valid 101 165 70 | valid 101 165 70 | 22 of 101 | 1.06e-12 | 3.99e-08 |
| volute | nary | valid 101 165 70 | valid 101 165 70 | 22 of 101 | 1.06e-12 | 3.99e-08 |
