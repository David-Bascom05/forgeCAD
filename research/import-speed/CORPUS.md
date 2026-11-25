# Import benchmark corpus (frozen 2026-11-25)

The files every import-speed and big-model render number is reported on (IMPORT-SPEED, BIG-RENDER).
Source: branch `haas-3d-models`, `research/haas-models/<zip>.zip`; the member named below (the
zip's directory). Sizes in MB; solids / refused and faces are kernel-stress a152c42c's read
(`forge-step/examples/stepdigest`), faces summed over the solids read.

Picked from all 317 STEP/IGES files of the 146 archives: the largest by faces and bytes in each kind
(VMC and gantry mills, trunnion mills, lathes, 5-axis UMCs with pallets and robots, rotaries and
indexers, a cobot, a laser), not one file. Split once, here: **tune** is what speed work may look at
while tuning; **held-out** is only reported, at the end of a round, never tuned on. Honesty notes:
VF-2 (the original stress case), HAB3232 (mass integrals) and ds-40y (meshing) were tuned on before
this split and are in the tune half; the held-out files appeared in the heavy-model timing table of
RESULTS.md (reported, not tuned on).

| half | kind | MB | faces | solids / refused | archive / member |
|---|---|---:|---:|---|---|
| tune | mill (VMC) | 59 | 10221 | 28 / 9 | `vf-2_solid_models_1_2023/VF-2_STEP__1_2023.step` |
| tune | 5-axis gantry mill | 117 | 36580 | 535 / 25 | `GM2-5AX_Solid_Model/10000100371 lightweight v1.STEP` |
| tune | mill + trunnion | 107 | 23622 | 48 / 2 | `vf-6_50_tr_solid_models_2_2022/VF-6_50_TR_EXTENDED Z-AXIS CLEARANCE_STEP__2_2022.step` |
| tune | lathe | 201 | 48211 | 186 / 53 | `st-10_st-15_solid_models_10_2023/st-10_st-15_solid_models_10_2023.STEP` |
| tune | lathe (dual spindle, Y) | 91 | 15413 | 102 / 4 | `ds-30y_solid_models_12_2022/DS-30Y_STEP__12_2022.STEP` |
| tune | UMC 5-axis + robot cell | 90 | 23496 | 211 / 2 | `rpl_modular_umc_750_solid_models_09_2026/rpl_modular_umc-750_09-2026 1.STEP` |
| tune | UMC 5-axis + pallet pool | 55 | 16471 | 105 / 1 | `umc-500_ss_20_station_pp_solid_models_03_2026/UMC-500 w 20 Station PP_3_2026.STEP` |
| tune | rotary (TRT160) | 27 | 7743 | 34 / 0 | `TR160_10-2019/TRT160_STEP_10-2019.STEP` |
| tune | rotary/indexer | 30 | 4680 | 19 / 0 | `hab3232_solid_models_06_2026/HAB3232_6_2026.STEP` |
| tune | lathe (dual spindle, Y) | 26 | 7452 | 66 / 1 | `ds-40y_solid_models_05_2026/ds-40y_solid_model_5_2026.STEP` |
| held-out | mill (VMC) | 134 | 24106 | 44 / 5 | `VM-6_SOLID_MODELS_1_2022/VM-6_STEP__1_2022.step` |
| held-out | mill + trunnion | 71 | 22346 | 100 / 3 | `vf-5_40_ss_tr_solid_models_3_2022/VF-5_40_SS_TR_STEP__3_2022.step` |
| held-out | lathe (Y) | 123 | 25072 | 91 / 6 | `st-28_st-28y_solid_models_07_2023/ST-28:28Y_10000224585.STEP` |
| held-out | lathe (long bed, Y) | 113 | 17331 | 275 / 63 | `ST20LY-25LY_3-2021/ST20LY-25LY_STEP_3-2021.STEP` |
| held-out | UMC 5-axis | 76 | 18828 | 278 / 3 | `UMC-1250_2021-03/UMC-1250_2021-03 updated.STEP` |
| held-out | 5-axis gantry mill | 58 | 21326 | 152 / 1 | `gm-1-5ax_solid_models_07_2026/GM-1-5AX 07-01-2026.STEP` |
| held-out | rotary (HRT160SS) | 22 | 7278 | 183 / 0 | `hrt160ss_step_10-2021/HRT160SS_STEP_10-2021.STEP` |
| held-out | rotary (TRT210) | 18 | 4968 | 48 / 0 | `trt210_4-2021/TRT210_STEP_4-2021.STEP` |
| held-out | cobot package | 144 | 25338 | 173 / 11 | `Haas_5kg_Cobot_Package_Model/Haas 5kg Cobot Package.STEP` |
| held-out | fiber laser | 80 | 27926 | 96 / 17 | `hfl-3015_solid_models_02_2026/HFL-3015_02_2026.STEP` |

Not in it yet:

- Parasolid: the same archives carry `.x_t` exports (VF-2's `VF-2_x_t__1_2023.x_t`, 22 MB;
  `VF-2TR_x_t__1_2023.x_t`, 31 MB; most machines have one). They join this table, the largest
  per kind as above, when PARASOLID's reader (parasolid/xt-reader) is in main.
- Public non-Haas STEP: the repository's fixtures are all small (33 files, 3.1 MB together);
  none is large enough to belong here, and nothing is downloaded from unofficial mirrors.
- Files main cannot read whole are listed with their errors in KERNEL-STRESS's corpus
  (research/kernel-stress/corpus.tsv), e.g. `VF-2TR_STEP__1_2023.STEP` (a reference past 32
  bits, read since kernel-stress a152c42c).

Report per file: wall seconds, cold, file on disk to every check done (`stepopen`'s "all read"),
4 cores of the cloud VM unless said; the median and the worst of each half; and bits against main
(`stepdigest`).
