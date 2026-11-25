# Sweep diff: sweep-fd8c9da21f-h47742fc00a-n0d0-ddd8a4b5 -> sweep-2963c65efa-h47742fc00a-n0d0-ddd8a4b5

Ops in both: 3343 (only in A: 0, only in B: 6). Solids in both: 324. Op time in both: 268 s -> 202 s (-24.5%).

| A \ B | ok | refused | wrong | hang | harness |
|---|---:|---:|---:|---:|---:|
| ok | 3109 | 0 | 0 | 0 | 0 |
| refused | 12 | 212 | 0 | 0 | 0 |
| wrong | 0 | 0 | 10 | 0 | 0 |
| hang | 0 | 0 | 0 | 0 | 0 |
| harness | 0 | 0 | 0 | 0 | 0 |

## 1. NEW WRONG ANSWERS (B Ok and wrong): 0


## 2. OK IN BOTH, ANOTHER ANSWER (show each right): 0


## 3. NEW HANGS, OUT OF MEMORY, CRASHES: 0


## 4. NEWLY REFUSED (ok in A): 0


## 5a. FIXED (wrong or hung in A): 0


## 5b. REFUSED IN A, OK IN B: 12

- HA5C-T_11-2019 / HA5C-T_STEP_11-2019.STEP #51342 box_int: refused Internal(SplitCrossed { face: FaceRef { name: #748de1584c37f9ae, surface: 'plane', near: Point3 { x: -73.87829 -> ok
- HA5C-T_11-2019 / HA5C-T_STEP_11-2019.STEP #51342 box_sub: refused Internal(SplitCrossed { face: FaceRef { name: #748de1584c37f9ae, surface: 'plane', near: Point3 { x: -73.87829 -> ok
- HA5C-T_11-2019 / HA5C-T_STEP_11-2019.STEP #80156 int_over: refused Unsupported(NoClosedForm { a: FaceRef { name: #502cff7de045a0c8, surface: 'cone', near: Point3 { x: -103.759, -> ok
- HA5C-T_11-2019 / HA5C-T_STEP_11-2019.STEP #80156 union_over: refused Unsupported(NoClosedForm { a: FaceRef { name: #502cff7de045a0c8, surface: 'cone', near: Point3 { x: -103.759, -> ok
- hvs-1_solid_models_07_2025 / HVS-1.STEP #1620 int_pair10: refused Unsupported(NoClosedForm { a: FaceRef { name: #2cf156be46afe4e7, surface: 'plane', near: Point3 { x: 853.88933 -> ok no faces
- hvs-1_solid_models_07_2025 / HVS-1.STEP #1620 int_pair16: refused Unsupported(NoClosedForm { a: FaceRef { name: #2cf156be46afe4e7, surface: 'plane', near: Point3 { x: 853.88933 -> ok no faces
- hvs-1_solid_models_07_2025 / HVS-1.STEP #1620 union_pair10: refused Unsupported(NoClosedForm { a: FaceRef { name: #2cf156be46afe4e7, surface: 'plane', near: Point3 { x: 853.88933 -> ok
- hvs-1_solid_models_07_2025 / HVS-1.STEP #1620 union_pair16: refused Unsupported(NoClosedForm { a: FaceRef { name: #2cf156be46afe4e7, surface: 'plane', near: Point3 { x: 853.88933 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 bore_int: refused Unsupported(NoClosedForm { a: FaceRef { name: #2c13e890211d6250, surface: 'nurbs', near: Point3 { x: 507.99612 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 bore_sub: refused Unsupported(NoClosedForm { a: FaceRef { name: #2c13e890211d6250, surface: 'nurbs', near: Point3 { x: 507.99612 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 int_over: refused Unsupported(NoClosedForm { a: FaceRef { name: #2c13e890211d6250, surface: 'nurbs', near: Point3 { x: 507.99612 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 union_over: refused Unsupported(NoClosedForm { a: FaceRef { name: #2c13e890211d6250, surface: 'nurbs', near: Point3 { x: 507.99612 -> ok

## 6. SOLIDS AND IMPORTS

Solids valid in A, not in B: 0; the other way: 0. Imports ok in A, not in B: 0; the other way: 0.

## 7. TIME: the ops that moved most

- +3.2 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #259140 union_over2 (83 -> 3277 ms)
- +3.1 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #259140 int_over2 (48 -> 3100 ms)
- +3.0 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #57887 union_over (413 -> 3395 ms)
- +2.8 s: st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #647540 union_over (55 -> 2857 ms)
- +2.8 s: tr160-2_2020-03 / TR160-2_STEP_2020-03.STEP #83171 union_over2 (114 -> 2902 ms)
- +2.8 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #87260 union_over (366 -> 3116 ms)
- +2.7 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #74491 union_over2 (234 -> 2896 ms)
- +2.3 s: st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #647540 union_over2 (545 -> 2823 ms)
- +2.1 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #259140 union_over (85 -> 2145 ms)
- +2.0 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #259140 int_over (48 -> 2051 ms)
- -21.7 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #167334 int_over2 (22366 -> 696 ms)
- -21.3 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #167334 union_over2 (25561 -> 4232 ms)
- -18.0 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #167334 int_over (18655 -> 659 ms)
- -17.8 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #167334 union_over (21927 -> 4158 ms)
- -16.3 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #60006 int_over (16728 -> 427 ms)
- -16.0 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #60006 union_over (18154 -> 2197 ms)
- -1.2 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #275604 union_over (1925 -> 732 ms)
- -1.2 s: st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #673016 int_pair17 (1388 -> 219 ms)
- -1.2 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #275604 int_over (1776 -> 608 ms)
- -1.2 s: st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #673016 int_pair66 (1806 -> 650 ms)

## The bound's trade (coordinator's ruling, 2026-11-25), after the independent review

`budget::cap_total(CARRIED)`, CARRIED = 2^24, set where a branch is carried onto a spline patch's
rim. A rim end with no point to carry it to keeps the stall of before 7be140be (the review's
blocker A: a branch left short of the face's edge, a wrong answer's path); so the recoveries that
needed it (ST-28L #663724, hvs-1 #17693, tr160-2 #163122, UMC-500 #87260 and #275604) are refused
again, and #87260 box_sub's cracked mesh is gone. The verdict near a cap is the op's whole spend's
on any machine (op-budget fd8c9da2, the review's blocker B). Measured spends before the review:
the answers kept spend at most 5.6M; ST-28L #673016 union_pair66, which answered before, spends
14.5M; a cap of 6 * 2^20 would end every refusal past a carry in under 2 s but refuse it. Ruled:
keep 2^24 (a real answer before 1.8 s off a refusal). Here: 12 refused ops answer, none lost, op
time -24.5%, the slowest refusal past a carry 3.4 s (33 s uncapped).
