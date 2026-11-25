# Sweep diff: sweep-139eafb804-h47742fc00a-n0d0-ddd8a4b5 -> sweep-5b17456489-h47742fc00a-n0d0-ddd8a4b5

Ops in both: 3343 (only in A: 0, only in B: 14). Solids in both: 324. Op time in both: 290 s -> 235 s (-19.0%).

| A \ B | ok | refused | wrong | hang | harness |
|---|---:|---:|---:|---:|---:|
| ok | 3109 | 0 | 0 | 0 | 0 |
| refused | 27 | 196 | 1 | 0 | 0 |
| wrong | 0 | 0 | 10 | 0 | 0 |
| hang | 0 | 0 | 0 | 0 | 0 |
| harness | 0 | 0 | 0 | 0 | 0 |

## 1. NEW WRONG ANSWERS (B Ok and wrong): 1

- umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #87260 box_sub: refused Unsupported(NoClosedForm { a: FaceRef { name: #15ecdeb88677f4ca, surface: 'nurbs', near: Point3 { x: 114.44311 -> wrong mesh not watertight: open 3 vol 161076389.82488403

## 2. OK IN BOTH, ANOTHER ANSWER (show each right): 0


## 3. NEW HANGS, OUT OF MEMORY, CRASHES: 0


## 4. NEWLY REFUSED (ok in A): 0


## 5a. FIXED (wrong or hung in A): 0


## 5b. REFUSED IN A, OK IN B: 27

- HA5C-T_11-2019 / HA5C-T_STEP_11-2019.STEP #51342 box_int: refused Internal(SplitCrossed { face: FaceRef { name: #748de1584c37f9ae, surface: 'plane', near: Point3 { x: -73.87829 -> ok
- HA5C-T_11-2019 / HA5C-T_STEP_11-2019.STEP #51342 box_sub: refused Internal(SplitCrossed { face: FaceRef { name: #748de1584c37f9ae, surface: 'plane', near: Point3 { x: -73.87829 -> ok
- HA5C-T_11-2019 / HA5C-T_STEP_11-2019.STEP #80156 int_over: refused Unsupported(NoClosedForm { a: FaceRef { name: #502cff7de045a0c8, surface: 'cone', near: Point3 { x: -103.759, -> ok
- HA5C-T_11-2019 / HA5C-T_STEP_11-2019.STEP #80156 union_over: refused Unsupported(NoClosedForm { a: FaceRef { name: #502cff7de045a0c8, surface: 'cone', near: Point3 { x: -103.759, -> ok
- hvs-1_solid_models_07_2025 / HVS-1.STEP #1620 int_pair10: refused Unsupported(NoClosedForm { a: FaceRef { name: #2cf156be46afe4e7, surface: 'plane', near: Point3 { x: 853.88933 -> ok no faces
- hvs-1_solid_models_07_2025 / HVS-1.STEP #1620 int_pair16: refused Unsupported(NoClosedForm { a: FaceRef { name: #2cf156be46afe4e7, surface: 'plane', near: Point3 { x: 853.88933 -> ok no faces
- hvs-1_solid_models_07_2025 / HVS-1.STEP #1620 union_pair10: refused Unsupported(NoClosedForm { a: FaceRef { name: #2cf156be46afe4e7, surface: 'plane', near: Point3 { x: 853.88933 -> ok
- hvs-1_solid_models_07_2025 / HVS-1.STEP #1620 union_pair16: refused Unsupported(NoClosedForm { a: FaceRef { name: #2cf156be46afe4e7, surface: 'plane', near: Point3 { x: 853.88933 -> ok
- hvs-1_solid_models_07_2025 / HVS-1.STEP #17693 int_over2: refused Unsupported(NoClosedForm { a: FaceRef { name: #ce8c6080711acebc, surface: 'nurbs', near: Point3 { x: 898.88933 -> ok
- hvs-1_solid_models_07_2025 / HVS-1.STEP #17693 union_over2: refused Unsupported(NoClosedForm { a: FaceRef { name: #ce8c6080711acebc, surface: 'nurbs', near: Point3 { x: 898.88933 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 bore_int: refused Unsupported(NoClosedForm { a: FaceRef { name: #2c13e890211d6250, surface: 'nurbs', near: Point3 { x: 507.99612 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 bore_sub: refused Unsupported(NoClosedForm { a: FaceRef { name: #2c13e890211d6250, surface: 'nurbs', near: Point3 { x: 507.99612 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 box_int: refused Unsupported(NoClosedForm { a: FaceRef { name: #2c13e890211d6250, surface: 'nurbs', near: Point3 { x: 507.99612 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 box_sub: refused Unsupported(NoClosedForm { a: FaceRef { name: #2c13e890211d6250, surface: 'nurbs', near: Point3 { x: 507.99612 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 int_over: refused Unsupported(NoClosedForm { a: FaceRef { name: #2c13e890211d6250, surface: 'nurbs', near: Point3 { x: 507.99612 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 int_over2: refused Internal(InconsistentClassification { a: FaceRef { name: #554b48a272cb2a5e, surface: 'cylinder', near: Point3 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 union_over: refused Unsupported(NoClosedForm { a: FaceRef { name: #2c13e890211d6250, surface: 'nurbs', near: Point3 { x: 507.99612 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #645206 union_over2: refused Internal(InconsistentClassification { a: FaceRef { name: #554b48a272cb2a5e, surface: 'cylinder', near: Point3 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #663724 bore_int: refused Unsupported(NoClosedForm { a: FaceRef { name: #17825c9abe25e070, surface: 'nurbs', near: Point3 { x: 271.54758 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #663724 bore_sub: refused Unsupported(NoClosedForm { a: FaceRef { name: #17825c9abe25e070, surface: 'nurbs', near: Point3 { x: 271.54758 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #663724 box_int: refused Unsupported(NoClosedForm { a: FaceRef { name: #17825c9abe25e070, surface: 'nurbs', near: Point3 { x: 271.54758 -> ok
- st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #663724 box_sub: refused Unsupported(NoClosedForm { a: FaceRef { name: #17825c9abe25e070, surface: 'nurbs', near: Point3 { x: 271.54758 -> ok
- tr160-2_2020-03 / TR160-2_STEP_2020-03.STEP #163122 int_pair17: refused Unsupported(NoClosedForm { a: FaceRef { name: #3da33767c6c55bc8, surface: 'nurbs', near: Point3 { x: 76.033501 -> ok
- tr160-2_2020-03 / TR160-2_STEP_2020-03.STEP #163122 union_pair17: refused Unsupported(NoClosedForm { a: FaceRef { name: #3da33767c6c55bc8, surface: 'nurbs', near: Point3 { x: 76.033501 -> ok
- umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #87260 box_int: refused Unsupported(NoClosedForm { a: FaceRef { name: #15ecdeb88677f4ca, surface: 'nurbs', near: Point3 { x: 114.44311 -> ok
- umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #275604 box_int: refused Unsupported(NoClosedForm { a: FaceRef { name: #2980009e2cacedbb, surface: 'nurbs', near: Point3 { x: -442.3000 -> ok
- umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #275604 box_sub: refused Unsupported(NoClosedForm { a: FaceRef { name: #2980009e2cacedbb, surface: 'nurbs', near: Point3 { x: -442.3000 -> ok

## 6. SOLIDS AND IMPORTS

Solids valid in A, not in B: 0; the other way: 0. Imports ok in A, not in B: 0; the other way: 0.

## 7. TIME: the ops that moved most

- +3.7 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #259140 union_over2 (83 -> 3777 ms)
- +3.5 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #87260 union_over (358 -> 3812 ms)
- +3.4 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #259140 int_over2 (48 -> 3493 ms)
- +3.4 s: st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #647540 union_over (56 -> 3486 ms)
- +3.1 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #57887 union_over (542 -> 3680 ms)
- +3.1 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #264610 union_over2 (214 -> 3307 ms)
- +3.1 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #264610 union_over (202 -> 3267 ms)
- +2.8 s: st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #647540 union_over2 (559 -> 3391 ms)
- +2.8 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #74491 union_over2 (251 -> 3001 ms)
- +2.6 s: tr160-2_2020-03 / TR160-2_STEP_2020-03.STEP #83171 union_over2 (113 -> 2685 ms)
- -22.6 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #167334 int_over2 (23353 -> 712 ms)
- -22.0 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #167334 union_over2 (26335 -> 4347 ms)
- -19.4 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #167334 int_over (20284 -> 900 ms)
- -18.7 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #167334 union_over (23645 -> 4963 ms)
- -18.4 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #60006 int_over (19039 -> 595 ms)
- -18.1 s: TR160_10-2019 / TRT160_STEP_10-2019.STEP #60006 union_over (20963 -> 2900 ms)
- -1.2 s: st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #673016 int_pair17 (1402 -> 225 ms)
- -1.1 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #275604 union_over (1867 -> 742 ms)
- -1.1 s: st-28l_solid_models_02_2025 / ST-28L- 2_2025.STEP #673016 int_pair66 (1862 -> 755 ms)
- -1.1 s: umc-500_ss_solid_models_04_2026 / UMC-500_4_2026.STEP #275604 int_over (1766 -> 682 ms)

## The bound's trade (coordinator's ruling, 2026-11-25)

`budget::cap_total(CARRIED)`, CARRIED = 2^24, set the first time an op ends a branch at a spline
patch's rim. Measured spends (st-28l, umc-500_ss, TR160 with op-budget): the answers the rim
carry gives spend at most 5.6M, but for UMC-500 #264610's four (about 21M: given up, back to
refused); ST-28L #673016 union_pair66, which answered before, carries and spends 14.5M; the ops
refused after it spend 5.3M to 359M uncapped (up to 33 s). A cap of 6 * 2^20 would end every such
refusal in under 2 s but refuse union_pair66; 2^24 keeps it, and those refusals end in 2 to 3.8 s.
Ruled: keep 2^24 (a real answer before 1.8 s off a refusal; net -19% op time, the 33 s gone).
