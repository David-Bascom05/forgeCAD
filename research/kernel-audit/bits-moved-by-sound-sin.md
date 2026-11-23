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
