# Train 13 instruction profiles

Base `5a3f4875b4d57f23f8e1f653b4f43c84b5c01031`. Original PR head `18169987f63df6c0ee7ece19aec8d9c2b280bfa1`. The base and integrated profiles reproduced all 99 counts from the original CI artifacts. For each benchmark, the isolated open-shell delta plus the isolated per-plane flush delta equals the integrated delta exactly. The finer-chords, G11, accumulation-margin and whole-turn items each returned the same 99 instruction counts as main. All six isolated items matched all 84 geometry hashes. [Integrated profiles](https://github.com/David-Bascom05/forgeCAD/actions/runs/45692504315) · [Six isolated profiles](https://github.com/David-Bascom05/forgeCAD/actions/runs/45693482087)

The following counts are additional instructions against main. CI's threshold is +0.03% per strict line.

| Original failing benchmark | Open-shell `c37bc7e4` | Flush-per-plane `7b61ccae` | Combined |
|---|---:|---:|---:|
| bracket edit | +4,579 | +7,061 | +11,640 |
| conrod app | +18,779 | +5,964 | +24,743 |
| coplanar seq | +334,120 | +0 | +334,120 |
| gearbox app | +48,353 | +23,748 | +72,101 |
| gearbox cli | +48,382 | +110,643 | +159,025 |
| guard cli | +1,508 | +37,434 | +38,942 |
| heatsink app | +6,463 | +110,071 | +116,534 |
| heatsink cli | +6,463 | +46,806 | +53,269 |
| heatsink edit | +12,926 | +300,725 | +313,651 |
| hole-grid nary1 | +0 | +49,049 | +49,049 |
| hole-grid seq | +216 | +50,297 | +50,513 |
| housing seq | +188,497 | +0 | +188,497 |
| hub app | +2,164 | +13,222 | +15,386 |
| hub edit | +4,291 | +15,320 | +19,611 |
| patterns seq | +168,906 | +0 | +168,906 |
| plate seq | +61,381 | +0 | +61,381 |
| quadarm cli | +8,078 | +13,475 | +21,553 |
| shaft seq | +143,077 | +0 | +143,077 |
| torture seq | +61,381 | +0 | +61,381 |
| tour nary1 | +1,814 | +3,765 | +5,579 |
| tour seq | +2,404 | +852 | +3,256 |
| upto seq | +11,891 | +29,429 | +41,320 |
| upto-flat seq | +8,451 | +0 | +8,451 |

For coplanar seq, all 334,120 additional instructions were in `boolean::general`, which checks the surviving uses of edges from removed faces and loops. Housing seq added 188,497 in the same function. These checks prevent handing out an open shell.

For heatsink edit, the per-plane flush change added 300,725 instructions and the open-shell check added 12,926. In the combined per-function profile, glibc's memory copy added 201,842 and `_int_malloc` added 59,761. The instruction count below the edge-sized vector-growth function rose 201,929 despite exactly 432 calls on both sides. These inclusive costs overlap with the glibc self costs. The heap layout changed how often a growing allocation needed to move. The n-ary pair loop also added 23,996 instructions. The differing allocator result on the Debian development VM did not reproduce the hosted heatsink result.

Head `2e7d9a2d` kept both geometry checks. It reused loop storage, pruned disjoint faces before iterating pairs and initialized the per-tool plane-group cache on demand. Coplanar seq measured −0.0991%, housing seq −0.1544%, and the strict total −0.0187% against main. Heatsink edit (+0.1533%) and bracket app (+0.0631%) remained over the limit. Head `73513c86` additionally reserves the known source and bevel topology before constructing a chamfer. [Candidate CI](https://github.com/David-Bascom05/forgeCAD/actions/runs/45695144874) · [Candidate profiles](https://github.com/David-Bascom05/forgeCAD/actions/runs/45695711876)
