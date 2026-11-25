# CAM speed baseline

Regenerate: `python3 research/cam-speed/baseline.py` (from the checkout root; `--reps 3` keeps the least of 3). Commit d1f60eb8 (merge-train/7 b3b3cae1 + the profiler), with cam_bench as of e09d864f, native release (x86-64-v3), 4 cores, FORGECAD_CACHE=off, one fresh process per document. Seconds of wall unless said.

**cold**: `forge_cam_lang::run` + the reply's JSON (the app's `run_unmeasured`), first run in the process. **rerun**: the same text again. **edit**: one change, then run, in the same process (the app reruns the whole text on each edit): `last op` = the document's last op's stepover/step/leave/to/depth changed, `first op` = its first op's (every later op of the setup sees new stock), `tool` = the last tool's stickout +1, `part` = the part's first number x1.02. **cpu/wall** = CPU seconds over wall (1.0 = one core busy). **thr** = most threads alive.

Raw results: research/cam-speed/baseline.jsonl. Re-render with `--from-json`.

Split (fixed, by a hash of the path): **T** = tune on, **H** = held out (never tuned on).

## Per document

| doc | set | cold | cpu/wall | thr | rerun | edit last op | edit first op | edit tool | edit part | ok |
|---|---|---|---|---|---|---|---|---|---|---|
| examples/machined/block_m6_cam.fg | T | **0.150** | 0.87 | 3 | 0.007 | - | 0.016 | 0.008 | 0.023 | True |
| examples/machined/bracket_cam.fg | T | **0.381** | 1.08 | 5 | 0.050 | 0.077 | 0.073 | 0.063 | 0.375 | True |
| examples/machined/collar_turn.fg | T | **0.171** | 1.05 | 2 | 0.017 | - | - | - | 0.077 | True |
| examples/machined/flange_cam.fg | T | **0.284** | 1.02 | 6 | 0.032 | - | 0.073 | 0.040 | 0.167 | True |
| examples/machined/hub_turn.fg | T | **0.604** | 1.01 | 2 | 0.017 | - | - | - | 0.371 | True |
| examples/machined/manifold_cam.fg | H | **0.714** | 1.82 | 13 | 0.057 | - | 0.056 | 0.055 | 0.562 | False |
| examples/machined/mold_core_cam.fg | H | **5.512** | 2.95 | 15 | 0.178 | 2.885 | 0.676 | 0.296 | 6.616 | True |
| examples/machined/mold_core_finish_cam.fg | T | **10.173** | 2.64 | 9 | 0.339 | 1.949 | 0.764 | 0.545 | 9.634 | True |
| examples/machined/nut_turn.fg | H | **0.169** | 1.00 | 2 | 0.017 | - | - | - | 0.061 | True |
| examples/machined/plate_break_cam.fg | H | **0.208** | 1.01 | 3 | 0.025 | - | 0.038 | 0.032 | 0.098 | True |
| examples/machined/plate_mark_cam.fg | H | **0.106** | 0.94 | 2 | 0.005 | 0.005 | 0.013 | 0.005 | 0.023 | True |
| examples/machined/shaft_turn.fg | T | **0.169** | 1.01 | 2 | 0.016 | - | - | - | 0.055 | True |
| examples/machined/slot_plate_cam.fg | T | **0.124** | 0.97 | 2 | 0.004 | 0.018 | 0.016 | 0.008 | 0.040 | True |
| examples/machined/well_island_cam.fg | H | **0.171** | 0.88 | 2 | 0.005 | 0.036 | 0.021 | 0.008 | 0.049 | True |
| verify/forge-cam-quality/parts/bilek/bilek_cam.fg | H | **0.169** | 1.06 | 5 | 0.005 | 0.083 | 0.006 | 0.009 | 0.081 | True |
| verify/forge-cam-quality/parts/boss_adaptive_cam.fg | H | **1.010** | 0.99 | 3 | 0.210 | 0.267 | 0.247 | 0.691 | 1.121 | True |
| verify/forge-cam-quality/parts/boss_cam.fg | H | **0.207** | 0.96 | 2 | 0.023 | 0.028 | 0.045 | 0.030 | 0.127 | True |
| verify/forge-cam-quality/parts/deep_boss_n_adaptive_cam.fg | T | **2.903** | 1.01 | 2 | 0.084 | 0.090 | 0.118 | 2.845 | 4.390 | True |
| verify/forge-cam-quality/parts/deep_boss_n_clear_cam.fg | T | **0.290** | 1.00 | 3 | 0.019 | 0.024 | 0.053 | 0.027 | 0.147 | True |
| verify/forge-cam-quality/parts/deep_boss_n_zigzag_cam.fg | H | **2.379** | 1.01 | 2 | 0.038 | 0.044 | 0.072 | 2.192 | 4.888 | True |
| verify/forge-cam-quality/parts/deep_boss_p_adaptive_cam.fg | H | **2.960** | 1.01 | 2 | 0.086 | 0.094 | 0.124 | 2.798 | 4.308 | True |
| verify/forge-cam-quality/parts/deep_boss_p_clear_cam.fg | H | **0.261** | 0.96 | 3 | 0.020 | 0.024 | 0.054 | 0.032 | 0.159 | True |
| verify/forge-cam-quality/parts/deep_boss_p_zigzag_cam.fg | H | **2.464** | 1.01 | 2 | 0.038 | 0.044 | 0.072 | 2.278 | 5.007 | True |
| verify/forge-cam-quality/parts/deep_islands_p_adaptive_cam.fg | H | **10.577** | 1.01 | 4 | 0.181 | 20.307 | 0.244 | 10.051 | 9.784 | True |
| verify/forge-cam-quality/parts/deep_islands_p_clear_cam.fg | T | **0.426** | 1.01 | 5 | 0.046 | 0.313 | 0.091 | 0.066 | 0.287 | True |
| verify/forge-cam-quality/parts/deep_islands_p_zigzag_cam.fg | T | **12.332** | 1.01 | 5 | 0.122 | 23.426 | 0.164 | 12.135 | 11.054 | True |
| verify/forge-cam-quality/parts/deep_open_slot_p_adaptive_cam.fg | H | **0.431** | 1.00 | 2 | 0.008 | 0.692 | 0.031 | 0.314 | 0.257 | True |
| verify/forge-cam-quality/parts/deep_open_slot_p_clear_cam.fg | T | **0.173** | 1.38 | 5 | 0.021 | 0.062 | 0.045 | 0.050 | 0.090 | True |
| verify/forge-cam-quality/parts/deep_open_slot_p_zigzag_cam.fg | T | **0.678** | 1.02 | 2 | 0.010 | 1.074 | 0.033 | 0.512 | 0.727 | True |
| verify/forge-cam-quality/parts/deep_slot_cam.fg | H | **0.114** | 0.96 | 2 | 0.003 | 0.009 | 0.025 | 0.006 | 0.031 | True |
| verify/forge-cam-quality/parts/open_step_cam.fg | H | **0.223** | 1.79 | 5 | 0.020 | 0.130 | 0.040 | 0.093 | 0.139 | True |
| verify/forge-cam-quality/parts/pocket_islands_cam.fg | H | **0.156** | 0.96 | 2 | 0.004 | 0.052 | 0.037 | 0.012 | 0.071 | True |
| verify/forge-cam-quality/parts/rptu/rptu1_clear_cam.fg | T | **0.877** | 2.01 | 5 | 0.008 | 0.013 | 0.043 | 0.728 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu2_none_cam.fg | T | **0.114** | 0.96 | 4 | 0.003 | 0.007 | 0.041 | 0.007 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu3_adaptive_cam.fg | H | **0.473** | 0.99 | 3 | 0.017 | 0.022 | 0.054 | 0.350 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu3_clear_cam.fg | H | **0.138** | 1.02 | 5 | 0.008 | 0.013 | 0.045 | 0.014 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu3_zigzag_cam.fg | T | **0.678** | 1.00 | 3 | 0.014 | 0.018 | 0.050 | 0.592 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu4_adaptive_cam.fg | H | **1.874** | 1.01 | 5 | 0.035 | 0.039 | 0.082 | 1.666 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu4_clear_cam.fg | T | **0.217** | 1.06 | 8 | 0.032 | 0.028 | 0.071 | 0.032 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu4_zigzag_cam.fg | T | **1.880** | 1.02 | 4 | 0.038 | 0.043 | 0.084 | 1.719 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu5_adaptive_cam.fg | T | **1.004** | 1.01 | 5 | 0.020 | 0.025 | 0.066 | 0.826 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu5_clear_cam.fg | H | **0.199** | 1.05 | 7 | 0.024 | 0.030 | 0.093 | 0.023 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu5_zigzag_cam.fg | T | **1.008** | 1.01 | 5 | 0.016 | 0.021 | 0.061 | 0.832 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu6_adaptive_cam.fg | T | **0.552** | 1.05 | 6 | 0.019 | 0.060 | 0.057 | 0.363 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu6_clear_cam.fg | T | **0.200** | 1.10 | 6 | 0.010 | 0.053 | 0.049 | 0.026 | - | True |
| verify/forge-cam-quality/parts/rptu/rptu6_zigzag_cam.fg | H | **0.728** | 1.06 | 6 | 0.015 | 0.059 | 0.058 | 0.583 | - | True |
| verify/forge-cam-quality/parts/thin_wall_cam.fg | T | **0.187** | 1.01 | 6 | 0.005 | 0.077 | 0.035 | 0.013 | 0.096 | True |

Total cold: 66.82 s over 47 documents; 13 over 1 s.

## Cold phases per document

`post` is the programs' wall; `cert` and `rt` are inside it, `cert` summed over the threads that ran it. `other` is the wall outside every span: parsing, evaluation, and in a fresh process the built-in library's first load. The turning documents have no spans (forge-turn-lang).

| doc | cold | part | setup | gen | hops | feeds | stock | post | cert | rt | g4 | json | other |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| examples/machined/block_m6_cam.fg | 0.150 | 0.003 | 0.02ms | 0.005 | 0.002 | 0.011 | 0.01ms | 0.009 | 0.011 | 0.001 | 0.001 | 0.26ms | 0.117 |
| examples/machined/bracket_cam.fg | 0.381 | 0.002 | 0.008 | 0.120 | 0.038 | 0.078 | 0.01ms | 0.037 | 0.033 | 0.003 | 0.014 | 0.28ms | 0.084 |
| examples/machined/collar_turn.fg | 0.171 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.05ms | 0.171 |
| examples/machined/flange_cam.fg | 0.284 | 0.003 | 0.02ms | 0.060 | 0.025 | 0.068 | 0.01ms | 0.012 | 0.023 | 0.001 | 0.002 | 0.23ms | 0.114 |
| examples/machined/hub_turn.fg | 0.604 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.07ms | 0.604 |
| examples/machined/manifold_cam.fg | 0.714 | 0.010 | 0.15ms | 0.081 | 0.009 | 0.267 | 0.03ms | 0.220 | 0.636 | 0.013 | 0.040 | 0.001 | 0.086 |
| examples/machined/mold_core_cam.fg | 5.512 | 0.004 | 0.02ms | 4.847 | 0.159 | 0.00ms | 0.01ms | 0.411 | 1.227 | 0.012 | 0.001 | 0.29ms | 0.090 |
| examples/machined/mold_core_finish_cam.fg | 10.173 | 0.003 | 0.02ms | 9.302 | 0.260 | 0.00ms | 0.01ms | 0.526 | 0.944 | 0.016 | 0.001 | 0.22ms | 0.081 |
| examples/machined/nut_turn.fg | 0.169 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.04ms | 0.169 |
| examples/machined/plate_break_cam.fg | 0.208 | 0.003 | 0.02ms | 0.038 | 0.020 | 0.017 | 0.01ms | 0.020 | 0.020 | 0.001 | 0.005 | 0.20ms | 0.105 |
| examples/machined/plate_mark_cam.fg | 0.106 | 0.003 | 0.02ms | 0.004 | 0.001 | 0.007 | 0.00ms | 0.004 | 0.003 | 0.22ms | 0.003 | 0.15ms | 0.085 |
| examples/machined/shaft_turn.fg | 0.169 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.05ms | 0.169 |
| examples/machined/slot_plate_cam.fg | 0.124 | 0.003 | 0.05ms | 0.001 | 0.00ms | 0.017 | 0.00ms | 0.005 | 0.005 | 0.25ms | 0.015 | 0.12ms | 0.082 |
| examples/machined/well_island_cam.fg | 0.171 | 0.004 | 0.03ms | 0.005 | 0.00ms | 0.031 | 0.01ms | 0.010 | 0.009 | 0.001 | 0.010 | 0.23ms | 0.112 |
| verify/forge-cam-quality/parts/bilek/bilek_cam.fg | 0.169 | 0.002 | 0.02ms | 0.011 | 0.00ms | 0.056 | 0.00ms | 0.010 | 0.008 | 0.001 | 0.007 | 0.09ms | 0.083 |
| verify/forge-cam-quality/parts/boss_adaptive_cam.fg | 1.010 | 0.004 | 0.02ms | 0.484 | 0.203 | 0.163 | 0.01ms | 0.038 | 0.035 | 0.002 | 0.027 | 0.19ms | 0.089 |
| verify/forge-cam-quality/parts/boss_cam.fg | 0.207 | 0.002 | 0.02ms | 0.022 | 0.019 | 0.066 | 0.01ms | 0.010 | 0.008 | 0.001 | 0.004 | 0.18ms | 0.084 |
| verify/forge-cam-quality/parts/deep_boss_n_adaptive_cam.fg | 2.903 | 0.001 | 0.01ms | 2.669 | 0.080 | 0.053 | 0.01ms | 0.010 | 0.008 | 0.001 | 0.007 | 0.16ms | 0.082 |
| verify/forge-cam-quality/parts/deep_boss_n_clear_cam.fg | 0.290 | 0.002 | 0.02ms | 0.057 | 0.014 | 0.080 | 0.01ms | 0.013 | 0.010 | 0.002 | 0.004 | 0.17ms | 0.121 |
| verify/forge-cam-quality/parts/deep_boss_n_zigzag_cam.fg | 2.379 | 0.001 | 0.01ms | 2.168 | 0.045 | 0.057 | 0.01ms | 0.016 | 0.014 | 0.002 | 0.011 | 0.17ms | 0.081 |
| verify/forge-cam-quality/parts/deep_boss_p_adaptive_cam.fg | 2.960 | 0.001 | 0.02ms | 2.720 | 0.079 | 0.053 | 0.01ms | 0.010 | 0.008 | 0.001 | 0.007 | 0.18ms | 0.089 |
| verify/forge-cam-quality/parts/deep_boss_p_clear_cam.fg | 0.261 | 0.001 | 0.02ms | 0.052 | 0.014 | 0.079 | 0.01ms | 0.015 | 0.013 | 0.002 | 0.004 | 0.17ms | 0.094 |
| verify/forge-cam-quality/parts/deep_boss_p_zigzag_cam.fg | 2.464 | 0.001 | 0.02ms | 2.243 | 0.033 | 0.052 | 0.01ms | 0.016 | 0.014 | 0.002 | 0.012 | 0.17ms | 0.107 |
| verify/forge-cam-quality/parts/deep_islands_p_adaptive_cam.fg | 10.577 | 0.002 | 0.01ms | 10.125 | 0.173 | 0.106 | 0.00ms | 0.039 | 0.033 | 0.004 | 0.015 | 0.15ms | 0.117 |
| verify/forge-cam-quality/parts/deep_islands_p_clear_cam.fg | 0.426 | 0.002 | 0.02ms | 0.128 | 0.060 | 0.126 | 0.01ms | 0.018 | 0.014 | 0.003 | 0.002 | 0.15ms | 0.090 |
| verify/forge-cam-quality/parts/deep_islands_p_zigzag_cam.fg | 12.332 | 0.002 | 0.02ms | 11.942 | 0.114 | 0.113 | 0.01ms | 0.053 | 0.043 | 0.007 | 0.018 | 0.16ms | 0.090 |
| verify/forge-cam-quality/parts/deep_open_slot_p_adaptive_cam.fg | 0.431 | 0.001 | 0.02ms | 0.281 | 0.005 | 0.017 | 0.00ms | 0.033 | 0.032 | 0.38ms | 0.008 | 0.13ms | 0.086 |
| verify/forge-cam-quality/parts/deep_open_slot_p_clear_cam.fg | 0.173 | 0.001 | 0.02ms | 0.009 | 0.018 | 0.027 | 0.00ms | 0.030 | 0.029 | 0.001 | 0.005 | 0.13ms | 0.083 |
| verify/forge-cam-quality/parts/deep_open_slot_p_zigzag_cam.fg | 0.678 | 0.002 | 0.02ms | 0.486 | 0.007 | 0.024 | 0.01ms | 0.033 | 0.032 | 0.001 | 0.008 | 0.14ms | 0.118 |
| verify/forge-cam-quality/parts/deep_slot_cam.fg | 0.114 | 0.002 | 0.02ms | 0.001 | 0.00ms | 0.017 | 0.00ms | 0.005 | 0.004 | 0.35ms | 0.006 | 0.13ms | 0.083 |
| verify/forge-cam-quality/parts/open_step_cam.fg | 0.223 | 0.003 | 0.01ms | 0.016 | 0.017 | 0.029 | 0.00ms | 0.074 | 0.073 | 0.001 | 0.001 | 0.13ms | 0.083 |
| verify/forge-cam-quality/parts/pocket_islands_cam.fg | 0.156 | 0.003 | 0.02ms | 0.007 | 0.00ms | 0.044 | 0.00ms | 0.009 | 0.008 | 0.001 | 0.008 | 0.14ms | 0.084 |
| verify/forge-cam-quality/parts/rptu/rptu1_clear_cam.fg | 0.877 | 0.001 | 0.02ms | 0.002 | 0.006 | 0.021 | 0.00ms | 0.761 | 0.760 | 0.42ms | 0.004 | 0.12ms | 0.082 |
| verify/forge-cam-quality/parts/rptu/rptu2_none_cam.fg | 0.114 | 0.002 | 0.01ms | 0.002 | 0.00ms | 0.020 | 0.00ms | 0.006 | 0.007 | 0.42ms | 0.002 | 0.11ms | 0.083 |
| verify/forge-cam-quality/parts/rptu/rptu3_adaptive_cam.fg | 0.473 | 0.002 | 0.02ms | 0.332 | 0.013 | 0.028 | 0.01ms | 0.009 | 0.012 | 0.001 | 0.007 | 0.15ms | 0.082 |
| verify/forge-cam-quality/parts/rptu/rptu3_clear_cam.fg | 0.138 | 0.002 | 0.01ms | 0.011 | 0.004 | 0.027 | 0.01ms | 0.008 | 0.012 | 0.001 | 0.003 | 0.17ms | 0.083 |
| verify/forge-cam-quality/parts/rptu/rptu3_zigzag_cam.fg | 0.678 | 0.002 | 0.02ms | 0.538 | 0.009 | 0.028 | 0.01ms | 0.008 | 0.011 | 0.001 | 0.007 | 0.15ms | 0.085 |
| verify/forge-cam-quality/parts/rptu/rptu4_adaptive_cam.fg | 1.874 | 0.002 | 0.01ms | 1.630 | 0.028 | 0.042 | 0.01ms | 0.037 | 0.041 | 0.001 | 0.052 | 0.20ms | 0.083 |
| verify/forge-cam-quality/parts/rptu/rptu4_clear_cam.fg | 0.217 | 0.002 | 0.01ms | 0.055 | 0.017 | 0.041 | 0.01ms | 0.012 | 0.017 | 0.001 | 0.006 | 0.25ms | 0.083 |
| verify/forge-cam-quality/parts/rptu/rptu4_zigzag_cam.fg | 1.880 | 0.002 | 0.01ms | 1.639 | 0.032 | 0.042 | 0.01ms | 0.036 | 0.041 | 0.001 | 0.046 | 0.22ms | 0.082 |
| verify/forge-cam-quality/parts/rptu/rptu5_adaptive_cam.fg | 1.004 | 0.002 | 0.02ms | 0.801 | 0.014 | 0.043 | 0.01ms | 0.019 | 0.024 | 0.001 | 0.035 | 0.20ms | 0.088 |
| verify/forge-cam-quality/parts/rptu/rptu5_clear_cam.fg | 0.199 | 0.002 | 0.01ms | 0.045 | 0.010 | 0.036 | 0.01ms | 0.012 | 0.022 | 0.001 | 0.010 | 0.26ms | 0.083 |
| verify/forge-cam-quality/parts/rptu/rptu5_zigzag_cam.fg | 1.008 | 0.002 | 0.02ms | 0.823 | 0.011 | 0.035 | 0.01ms | 0.017 | 0.022 | 0.001 | 0.035 | 0.20ms | 0.085 |
| verify/forge-cam-quality/parts/rptu/rptu6_adaptive_cam.fg | 0.552 | 0.003 | 0.02ms | 0.349 | 0.013 | 0.064 | 0.01ms | 0.020 | 0.027 | 0.001 | 0.009 | 0.15ms | 0.095 |
| verify/forge-cam-quality/parts/rptu/rptu6_clear_cam.fg | 0.200 | 0.002 | 0.02ms | 0.017 | 0.007 | 0.062 | 0.01ms | 0.020 | 0.028 | 0.001 | 0.004 | 0.15ms | 0.088 |
| verify/forge-cam-quality/parts/rptu/rptu6_zigzag_cam.fg | 0.728 | 0.002 | 0.02ms | 0.545 | 0.009 | 0.056 | 0.01ms | 0.020 | 0.028 | 0.002 | 0.011 | 0.17ms | 0.085 |
| verify/forge-cam-quality/parts/thin_wall_cam.fg | 0.187 | 0.003 | 0.03ms | 0.004 | 0.00ms | 0.068 | 0.00ms | 0.013 | 0.016 | 0.002 | 0.011 | 0.15ms | 0.089 |

## Edit phases per document (where the edit's time goes)

| doc | edit | wall | part | setup | gen | hops | feeds | stock | post | cert | rt | g4 | json | other |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| examples/machined/block_m6_cam.fg | rerun | 0.007 | 0.001 | 0.02ms | 0.002 | 0.002 | 0.12ms | 0.00ms | 0.001 | 0.001 | 0.06ms | 0.10ms | 0.26ms | 0.13ms |
| examples/machined/block_m6_cam.fg | edit_first_op: skim (facing) | 0.016 | 0.001 | 0.02ms | 0.002 | 0.002 | 0.008 | 0.00ms | 0.002 | 0.001 | 0.001 | 0.09ms | 0.17ms | 0.12ms |
| examples/machined/block_m6_cam.fg | edit_tool: t6 stickout +1 | 0.008 | 0.001 | 0.01ms | 0.002 | 0.002 | 0.09ms | 0.00ms | 0.003 | 0.003 | 0.05ms | 0.08ms | 0.17ms | 0.09ms |
| examples/machined/block_m6_cam.fg | edit_part: part shared L x1.02 | 0.023 | 0.001 | 0.01ms | 0.003 | 0.002 | 0.008 | 0.00ms | 0.008 | 0.010 | 0.38ms | 0.001 | 0.22ms | 0.08ms |
| examples/machined/bracket_cam.fg | rerun | 0.050 | 0.001 | 0.40ms | 0.003 | 0.039 | 0.001 | 0.01ms | 0.004 | 0.002 | 0.32ms | 0.49ms | 0.29ms | 0.20ms |
| examples/machined/bracket_cam.fg | edit_last_op: rest (clear) | 0.077 | 0.001 | 0.40ms | 0.019 | 0.038 | 0.009 | 0.01ms | 0.008 | 0.006 | 0.001 | 0.001 | 0.28ms | 0.19ms |
| examples/machined/bracket_cam.fg | edit_first_op: skim (facing) | 0.073 | 0.001 | 0.44ms | 0.003 | 0.038 | 0.022 | 0.01ms | 0.008 | 0.005 | 0.001 | 0.47ms | 0.27ms | 0.22ms |
| examples/machined/bracket_cam.fg | edit_tool: t2 stickout +1 | 0.063 | 0.001 | 0.45ms | 0.003 | 0.038 | 0.001 | 0.01ms | 0.019 | 0.017 | 0.31ms | 0.49ms | 0.28ms | 0.22ms |
| examples/machined/bracket_cam.fg | edit_part: part shared L x1.02 | 0.375 | 0.001 | 0.013 | 0.166 | 0.039 | 0.095 | 0.01ms | 0.045 | 0.040 | 0.003 | 0.016 | 0.27ms | 0.29ms |
| examples/machined/collar_turn.fg | rerun | 0.017 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.02ms | 0.017 |
| examples/machined/collar_turn.fg | edit_part: part shared d x1.02 | 0.077 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.03ms | 0.077 |
| examples/machined/flange_cam.fg | rerun | 0.032 | 0.002 | 0.05ms | 0.002 | 0.025 | 0.35ms | 0.00ms | 0.002 | 0.001 | 0.18ms | 0.39ms | 0.21ms | 0.11ms |
| examples/machined/flange_cam.fg | edit_first_op: skim (facing) | 0.073 | 0.002 | 0.01ms | 0.002 | 0.025 | 0.038 | 0.01ms | 0.005 | 0.003 | 0.002 | 0.38ms | 0.25ms | 0.13ms |
| examples/machined/flange_cam.fg | edit_tool: t1 stickout +1 | 0.040 | 0.002 | 0.01ms | 0.002 | 0.025 | 0.36ms | 0.00ms | 0.010 | 0.020 | 0.20ms | 0.40ms | 0.23ms | 0.13ms |
| examples/machined/flange_cam.fg | edit_part: part shared side x1.02 | 0.167 | 0.002 | 0.01ms | 0.054 | 0.026 | 0.071 | 0.01ms | 0.012 | 0.023 | 0.001 | 0.002 | 0.22ms | 0.15ms |
| examples/machined/hub_turn.fg | rerun | 0.017 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.03ms | 0.017 |
| examples/machined/hub_turn.fg | edit_part: part shared ri x1.02 | 0.371 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.05ms | 0.371 |
| examples/machined/manifold_cam.fg | rerun | 0.057 | 0.008 | 0.14ms | 0.014 | 0.009 | 0.002 | 0.01ms | 0.019 | 0.019 | 0.002 | 0.002 | 0.001 | 0.003 |
| examples/machined/manifold_cam.fg | edit_first_op: skim (facing) | 0.056 | 0.008 | 0.13ms | 0.014 | 0.009 | 0.002 | 0.01ms | 0.018 | 0.018 | 0.002 | 0.002 | 0.001 | 0.003 |
| examples/machined/manifold_cam.fg | edit_tool: t6 stickout +1 | 0.055 | 0.007 | 0.13ms | 0.014 | 0.009 | 0.002 | 0.01ms | 0.018 | 0.021 | 0.002 | 0.002 | 0.001 | 0.002 |
| examples/machined/manifold_cam.fg | edit_part: part shared L x1.02 | 0.562 | 0.007 | 0.15ms | 0.046 | 0.009 | 0.250 | 0.03ms | 0.207 | 0.559 | 0.014 | 0.038 | 0.001 | 0.003 |
| examples/machined/mold_core_cam.fg | rerun | 0.178 | 0.002 | 0.02ms | 0.005 | 0.156 | 0.00ms | 0.01ms | 0.014 | 0.011 | 0.001 | 0.39ms | 0.29ms | 0.20ms |
| examples/machined/mold_core_cam.fg | edit_last_op: left (rest) | 2.885 | 0.002 | 0.02ms | 2.666 | 0.160 | 0.00ms | 0.01ms | 0.056 | 0.037 | 0.017 | 0.49ms | 0.40ms | 0.22ms |
| examples/machined/mold_core_cam.fg | edit_first_op: rough (zlevel) | 0.676 | 0.004 | 0.07ms | 0.455 | 0.154 | 0.000 | 0.01ms | 0.062 | 0.048 | 0.012 | 0.48ms | 0.27ms | 0.26ms |
| examples/machined/mold_core_cam.fg | edit_tool: t2 stickout +1 | 0.296 | 0.004 | 0.02ms | 0.005 | 0.164 | 0.000 | 0.01ms | 0.122 | 0.279 | 0.001 | 0.48ms | 0.27ms | 0.22ms |
| examples/machined/mold_core_cam.fg | edit_part: part shared w x1.02 | 6.616 | 0.003 | 0.02ms | 4.571 | 0.154 | 0.000 | 0.01ms | 1.886 | 2.027 | 0.011 | 0.001 | 0.28ms | 0.24ms |
| examples/machined/mold_core_finish_cam.fg | rerun | 0.339 | 0.002 | 0.02ms | 0.004 | 0.312 | 0.00ms | 0.01ms | 0.019 | 0.010 | 0.002 | 0.42ms | 0.22ms | 0.23ms |
| examples/machined/mold_core_finish_cam.fg | edit_last_op: plate (flat) | 1.949 | 0.002 | 0.02ms | 1.653 | 0.274 | 0.00ms | 0.01ms | 0.019 | 0.010 | 0.002 | 0.47ms | 0.24ms | 0.22ms |
| examples/machined/mold_core_finish_cam.fg | edit_first_op: rough (zlevel) | 0.764 | 0.002 | 0.02ms | 0.461 | 0.254 | 0.000 | 0.01ms | 0.045 | 0.023 | 0.015 | 0.46ms | 0.20ms | 0.19ms |
| examples/machined/mold_core_finish_cam.fg | edit_tool: t2 stickout +1 | 0.545 | 0.002 | 0.02ms | 0.004 | 0.262 | 0.000 | 0.01ms | 0.276 | 0.469 | 0.002 | 0.001 | 0.26ms | 0.20ms |
| examples/machined/mold_core_finish_cam.fg | edit_part: part shared w x1.02 | 9.634 | 0.003 | 0.02ms | 9.113 | 0.262 | 0.00ms | 0.01ms | 0.255 | 0.293 | 0.015 | 0.001 | 0.28ms | 0.25ms |
| examples/machined/nut_turn.fg | rerun | 0.017 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.02ms | 0.017 |
| examples/machined/nut_turn.fg | edit_part: part shared d x1.02 | 0.061 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.03ms | 0.061 |
| examples/machined/plate_break_cam.fg | rerun | 0.025 | 0.001 | 0.02ms | 0.002 | 0.019 | 0.32ms | 0.00ms | 0.002 | 0.001 | 0.11ms | 0.19ms | 0.20ms | 0.11ms |
| examples/machined/plate_break_cam.fg | edit_first_op: skim (facing) | 0.038 | 0.001 | 0.02ms | 0.002 | 0.019 | 0.011 | 0.00ms | 0.004 | 0.002 | 0.001 | 0.12ms | 0.19ms | 0.11ms |
| examples/machined/plate_break_cam.fg | edit_tool: t7 stickout +1 | 0.032 | 0.001 | 0.02ms | 0.002 | 0.020 | 0.32ms | 0.00ms | 0.009 | 0.008 | 0.11ms | 0.12ms | 0.22ms | 0.11ms |
| examples/machined/plate_break_cam.fg | edit_part: part shared L x1.02 | 0.098 | 0.002 | 0.02ms | 0.034 | 0.020 | 0.017 | 0.00ms | 0.020 | 0.020 | 0.001 | 0.005 | 0.20ms | 0.14ms |
| examples/machined/plate_mark_cam.fg | rerun | 0.005 | 0.001 | 0.02ms | 0.002 | 0.001 | 0.07ms | 0.00ms | 0.45ms | 0.26ms | 0.03ms | 0.07ms | 0.16ms | 0.28ms |
| examples/machined/plate_mark_cam.fg | edit_last_op: mark (engrave) | 0.005 | 0.001 | 0.01ms | 0.002 | 0.001 | 0.07ms | 0.00ms | 0.001 | 0.001 | 0.24ms | 0.07ms | 0.14ms | 0.26ms |
| examples/machined/plate_mark_cam.fg | edit_first_op: skim (facing) | 0.013 | 0.001 | 0.01ms | 0.002 | 0.001 | 0.007 | 0.00ms | 0.001 | 0.001 | 0.28ms | 0.09ms | 0.21ms | 0.32ms |
| examples/machined/plate_mark_cam.fg | edit_tool: t5 stickout +1 | 0.005 | 0.001 | 0.01ms | 0.002 | 0.001 | 0.07ms | 0.00ms | 0.001 | 0.001 | 0.03ms | 0.07ms | 0.17ms | 0.27ms |
| examples/machined/plate_mark_cam.fg | edit_part: part shared L x1.02 | 0.023 | 0.001 | 0.01ms | 0.004 | 0.001 | 0.009 | 0.00ms | 0.004 | 0.004 | 0.32ms | 0.003 | 0.14ms | 0.44ms |
| examples/machined/shaft_turn.fg | rerun | 0.016 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.02ms | 0.016 |
| examples/machined/shaft_turn.fg | edit_part: part shared d x1.02 | 0.055 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.04ms | 0.055 |
| examples/machined/slot_plate_cam.fg | rerun | 0.004 | 0.002 | 0.02ms | 0.001 | 0.00ms | 0.08ms | 0.00ms | 0.001 | 0.39ms | 0.04ms | 0.13ms | 0.12ms | 0.43ms |
| examples/machined/slot_plate_cam.fg | edit_last_op: slots (slot) | 0.018 | 0.002 | 0.02ms | 0.001 | 0.000 | 0.011 | 0.00ms | 0.004 | 0.003 | 0.30ms | 0.16ms | 0.11ms | 0.42ms |
| examples/machined/slot_plate_cam.fg | edit_first_op: skim (facing) | 0.016 | 0.002 | 0.02ms | 0.001 | 0.000 | 0.011 | 0.00ms | 0.002 | 0.001 | 0.32ms | 0.14ms | 0.11ms | 0.001 |
| examples/machined/slot_plate_cam.fg | edit_tool: t2 stickout +1 | 0.008 | 0.002 | 0.02ms | 0.001 | 0.000 | 0.08ms | 0.00ms | 0.004 | 0.004 | 0.04ms | 0.14ms | 0.13ms | 0.46ms |
| examples/machined/slot_plate_cam.fg | edit_part: part shared L x1.02 | 0.040 | 0.002 | 0.02ms | 0.001 | 0.000 | 0.017 | 0.00ms | 0.005 | 0.005 | 0.26ms | 0.014 | 0.12ms | 0.47ms |
| examples/machined/well_island_cam.fg | rerun | 0.005 | 0.002 | 0.08ms | 0.001 | 0.00ms | 0.24ms | 0.00ms | 0.001 | 0.47ms | 0.11ms | 0.14ms | 0.20ms | 0.12ms |
| examples/machined/well_island_cam.fg | edit_last_op: well (pocket) | 0.036 | 0.001 | 0.02ms | 0.005 | 0.00ms | 0.025 | 0.00ms | 0.005 | 0.004 | 0.001 | 0.18ms | 0.18ms | 0.13ms |
| examples/machined/well_island_cam.fg | edit_first_op: skim (facing) | 0.021 | 0.001 | 0.02ms | 0.001 | 0.00ms | 0.015 | 0.00ms | 0.002 | 0.001 | 0.001 | 0.12ms | 0.12ms | 0.15ms |
| examples/machined/well_island_cam.fg | edit_tool: t2 stickout +1 | 0.008 | 0.001 | 0.02ms | 0.001 | 0.000 | 0.17ms | 0.00ms | 0.006 | 0.006 | 0.07ms | 0.12ms | 0.12ms | 0.08ms |
| examples/machined/well_island_cam.fg | edit_part: part shared L x1.02 | 0.049 | 0.001 | 0.01ms | 0.004 | 0.00ms | 0.029 | 0.00ms | 0.008 | 0.007 | 0.001 | 0.007 | 0.13ms | 0.10ms |
| verify/forge-cam-quality/parts/bilek/bilek_cam.fg | rerun | 0.005 | 0.001 | 0.02ms | 0.001 | 0.00ms | 0.42ms | 0.00ms | 0.002 | 0.001 | 0.15ms | 0.14ms | 0.11ms | 0.10ms |
| verify/forge-cam-quality/parts/bilek/bilek_cam.fg | edit_last_op: well (pocket) | 0.083 | 0.001 | 0.02ms | 0.009 | 0.00ms | 0.063 | 0.00ms | 0.009 | 0.007 | 0.002 | 0.15ms | 0.09ms | 0.09ms |
| verify/forge-cam-quality/parts/bilek/bilek_cam.fg | edit_first_op: well (pocket) | 0.006 | 0.002 | 0.02ms | 0.001 | 0.00ms | 0.001 | 0.00ms | 0.002 | 0.001 | 0.24ms | 0.14ms | 0.07ms | 0.11ms |
| verify/forge-cam-quality/parts/bilek/bilek_cam.fg | edit_tool: t1 stickout +1 | 0.009 | 0.001 | 0.01ms | 0.001 | 0.000 | 0.35ms | 0.00ms | 0.007 | 0.006 | 0.16ms | 0.13ms | 0.06ms | 0.10ms |
| verify/forge-cam-quality/parts/bilek/bilek_cam.fg | edit_part: part shared L x1.02 | 0.081 | 0.001 | 0.01ms | 0.010 | 0.000 | 0.055 | 0.00ms | 0.009 | 0.007 | 0.001 | 0.006 | 0.07ms | 0.09ms |
| verify/forge-cam-quality/parts/boss_adaptive_cam.fg | rerun | 0.210 | 0.001 | 0.02ms | 0.002 | 0.204 | 0.001 | 0.01ms | 0.002 | 0.001 | 0.20ms | 0.23ms | 0.20ms | 0.12ms |
| verify/forge-cam-quality/parts/boss_adaptive_cam.fg | edit_last_op: fin (contour) | 0.267 | 0.001 | 0.01ms | 0.002 | 0.252 | 0.007 | 0.01ms | 0.006 | 0.002 | 0.003 | 0.22ms | 0.24ms | 0.12ms |
| verify/forge-cam-quality/parts/boss_adaptive_cam.fg | edit_first_op: skim (facing) | 0.247 | 0.001 | 0.02ms | 0.002 | 0.209 | 0.029 | 0.01ms | 0.005 | 0.002 | 0.002 | 0.19ms | 0.17ms | 0.15ms |
| verify/forge-cam-quality/parts/boss_adaptive_cam.fg | edit_tool: t1 stickout +1 | 0.691 | 0.001 | 0.01ms | 0.479 | 0.203 | 0.001 | 0.00ms | 0.008 | 0.007 | 0.20ms | 0.17ms | 0.20ms | 0.11ms |
| verify/forge-cam-quality/parts/boss_adaptive_cam.fg | edit_part: part shared L x1.02 | 1.121 | 0.001 | 0.01ms | 0.655 | 0.264 | 0.157 | 0.01ms | 0.025 | 0.022 | 0.002 | 0.020 | 0.18ms | 0.12ms |
| verify/forge-cam-quality/parts/boss_cam.fg | rerun | 0.023 | 0.001 | 0.02ms | 0.001 | 0.019 | 0.37ms | 0.00ms | 0.001 | 0.001 | 0.15ms | 0.10ms | 0.15ms | 0.09ms |
| verify/forge-cam-quality/parts/boss_cam.fg | edit_last_op: fin (contour) | 0.028 | 0.001 | 0.01ms | 0.001 | 0.019 | 0.003 | 0.00ms | 0.003 | 0.001 | 0.001 | 0.10ms | 0.15ms | 0.08ms |
| verify/forge-cam-quality/parts/boss_cam.fg | edit_first_op: skim (facing) | 0.045 | 0.001 | 0.01ms | 0.001 | 0.019 | 0.020 | 0.00ms | 0.004 | 0.002 | 0.001 | 0.12ms | 0.15ms | 0.10ms |
| verify/forge-cam-quality/parts/boss_cam.fg | edit_tool: t1 stickout +1 | 0.030 | 0.001 | 0.01ms | 0.001 | 0.019 | 0.33ms | 0.00ms | 0.008 | 0.007 | 0.15ms | 0.10ms | 0.15ms | 0.11ms |
| verify/forge-cam-quality/parts/boss_cam.fg | edit_part: part shared L x1.02 | 0.127 | 0.001 | 0.01ms | 0.027 | 0.020 | 0.066 | 0.01ms | 0.009 | 0.008 | 0.001 | 0.004 | 0.15ms | 0.13ms |
| verify/forge-cam-quality/parts/deep_boss_n_adaptive_cam.fg | rerun | 0.084 | 0.001 | 0.02ms | 0.001 | 0.080 | 0.001 | 0.00ms | 0.002 | 0.001 | 0.17ms | 0.12ms | 0.18ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_boss_n_adaptive_cam.fg | edit_last_op: fin (contour) | 0.090 | 0.001 | 0.02ms | 0.001 | 0.080 | 0.004 | 0.00ms | 0.004 | 0.001 | 0.001 | 0.14ms | 0.16ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_boss_n_adaptive_cam.fg | edit_first_op: skim (facing) | 0.118 | 0.001 | 0.01ms | 0.002 | 0.080 | 0.030 | 0.01ms | 0.006 | 0.003 | 0.002 | 0.12ms | 0.16ms | 0.12ms |
| verify/forge-cam-quality/parts/deep_boss_n_adaptive_cam.fg | edit_tool: t1 stickout +1 | 2.845 | 0.001 | 0.01ms | 2.753 | 0.081 | 0.001 | 0.00ms | 0.010 | 0.009 | 0.18ms | 0.12ms | 0.16ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_boss_n_adaptive_cam.fg | edit_part: part shared L x1.02 | 4.390 | 0.001 | 0.01ms | 4.161 | 0.107 | 0.081 | 0.01ms | 0.031 | 0.026 | 0.003 | 0.009 | 0.16ms | 0.12ms |
| verify/forge-cam-quality/parts/deep_boss_n_clear_cam.fg | rerun | 0.019 | 0.001 | 0.01ms | 0.001 | 0.014 | 0.46ms | 0.00ms | 0.002 | 0.001 | 0.24ms | 0.09ms | 0.14ms | 0.08ms |
| verify/forge-cam-quality/parts/deep_boss_n_clear_cam.fg | edit_last_op: fin (contour) | 0.024 | 0.001 | 0.01ms | 0.001 | 0.014 | 0.003 | 0.00ms | 0.004 | 0.002 | 0.002 | 0.09ms | 0.14ms | 0.08ms |
| verify/forge-cam-quality/parts/deep_boss_n_clear_cam.fg | edit_first_op: skim (facing) | 0.053 | 0.001 | 0.01ms | 0.001 | 0.014 | 0.030 | 0.00ms | 0.007 | 0.004 | 0.002 | 0.09ms | 0.14ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_boss_n_clear_cam.fg | edit_tool: t1 stickout +1 | 0.027 | 0.001 | 0.01ms | 0.001 | 0.014 | 0.44ms | 0.00ms | 0.010 | 0.009 | 0.21ms | 0.09ms | 0.17ms | 0.08ms |
| verify/forge-cam-quality/parts/deep_boss_n_clear_cam.fg | edit_part: part shared L x1.02 | 0.147 | 0.001 | 0.01ms | 0.041 | 0.014 | 0.071 | 0.00ms | 0.015 | 0.013 | 0.002 | 0.004 | 0.15ms | 0.10ms |
| verify/forge-cam-quality/parts/deep_boss_n_zigzag_cam.fg | rerun | 0.038 | 0.001 | 0.01ms | 0.001 | 0.033 | 0.001 | 0.00ms | 0.002 | 0.001 | 0.19ms | 0.12ms | 0.15ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_boss_n_zigzag_cam.fg | edit_last_op: fin (contour) | 0.044 | 0.001 | 0.01ms | 0.001 | 0.033 | 0.004 | 0.00ms | 0.004 | 0.002 | 0.002 | 0.12ms | 0.15ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_boss_n_zigzag_cam.fg | edit_first_op: skim (facing) | 0.072 | 0.001 | 0.01ms | 0.002 | 0.033 | 0.030 | 0.00ms | 0.006 | 0.003 | 0.002 | 0.13ms | 0.16ms | 0.10ms |
| verify/forge-cam-quality/parts/deep_boss_n_zigzag_cam.fg | edit_tool: t1 stickout +1 | 2.192 | 0.001 | 0.01ms | 2.148 | 0.033 | 0.001 | 0.00ms | 0.009 | 0.008 | 0.20ms | 0.13ms | 0.17ms | 0.12ms |
| verify/forge-cam-quality/parts/deep_boss_n_zigzag_cam.fg | edit_part: part shared L x1.02 | 4.888 | 0.001 | 0.01ms | 4.683 | 0.088 | 0.080 | 0.01ms | 0.028 | 0.023 | 0.003 | 0.007 | 0.16ms | 0.12ms |
| verify/forge-cam-quality/parts/deep_boss_p_adaptive_cam.fg | rerun | 0.086 | 0.001 | 0.01ms | 0.001 | 0.082 | 0.001 | 0.00ms | 0.002 | 0.001 | 0.19ms | 0.16ms | 0.21ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_boss_p_adaptive_cam.fg | edit_last_op: fin (contour) | 0.094 | 0.001 | 0.02ms | 0.001 | 0.082 | 0.005 | 0.00ms | 0.005 | 0.002 | 0.002 | 0.18ms | 0.21ms | 0.14ms |
| verify/forge-cam-quality/parts/deep_boss_p_adaptive_cam.fg | edit_first_op: skim (facing) | 0.124 | 0.001 | 0.02ms | 0.001 | 0.080 | 0.036 | 0.01ms | 0.006 | 0.003 | 0.002 | 0.11ms | 0.15ms | 0.13ms |
| verify/forge-cam-quality/parts/deep_boss_p_adaptive_cam.fg | edit_tool: t1 stickout +1 | 2.798 | 0.001 | 0.01ms | 2.707 | 0.080 | 0.001 | 0.00ms | 0.010 | 0.009 | 0.16ms | 0.12ms | 0.19ms | 0.10ms |
| verify/forge-cam-quality/parts/deep_boss_p_adaptive_cam.fg | edit_part: part shared L x1.02 | 4.308 | 0.001 | 0.01ms | 4.086 | 0.104 | 0.081 | 0.01ms | 0.029 | 0.025 | 0.003 | 0.008 | 0.17ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_boss_p_clear_cam.fg | rerun | 0.020 | 0.001 | 0.02ms | 0.001 | 0.015 | 0.001 | 0.00ms | 0.002 | 0.001 | 0.25ms | 0.10ms | 0.15ms | 0.10ms |
| verify/forge-cam-quality/parts/deep_boss_p_clear_cam.fg | edit_last_op: fin (contour) | 0.024 | 0.001 | 0.01ms | 0.001 | 0.014 | 0.003 | 0.00ms | 0.004 | 0.002 | 0.002 | 0.10ms | 0.15ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_boss_p_clear_cam.fg | edit_first_op: skim (facing) | 0.054 | 0.001 | 0.01ms | 0.001 | 0.014 | 0.031 | 0.00ms | 0.008 | 0.004 | 0.002 | 0.10ms | 0.16ms | 0.10ms |
| verify/forge-cam-quality/parts/deep_boss_p_clear_cam.fg | edit_tool: t1 stickout +1 | 0.032 | 0.001 | 0.01ms | 0.001 | 0.014 | 0.46ms | 0.00ms | 0.015 | 0.014 | 0.25ms | 0.11ms | 0.19ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_boss_p_clear_cam.fg | edit_part: part shared L x1.02 | 0.159 | 0.001 | 0.01ms | 0.041 | 0.015 | 0.080 | 0.01ms | 0.018 | 0.016 | 0.002 | 0.004 | 0.18ms | 0.14ms |
| verify/forge-cam-quality/parts/deep_boss_p_zigzag_cam.fg | rerun | 0.038 | 0.001 | 0.01ms | 0.001 | 0.033 | 0.001 | 0.00ms | 0.002 | 0.001 | 0.21ms | 0.13ms | 0.16ms | 0.10ms |
| verify/forge-cam-quality/parts/deep_boss_p_zigzag_cam.fg | edit_last_op: fin (contour) | 0.044 | 0.001 | 0.02ms | 0.001 | 0.034 | 0.004 | 0.00ms | 0.004 | 0.002 | 0.002 | 0.13ms | 0.16ms | 0.10ms |
| verify/forge-cam-quality/parts/deep_boss_p_zigzag_cam.fg | edit_first_op: skim (facing) | 0.072 | 0.001 | 0.01ms | 0.001 | 0.033 | 0.030 | 0.01ms | 0.006 | 0.003 | 0.002 | 0.13ms | 0.16ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_boss_p_zigzag_cam.fg | edit_tool: t1 stickout +1 | 2.278 | 0.001 | 0.01ms | 2.233 | 0.034 | 0.001 | 0.00ms | 0.009 | 0.008 | 0.22ms | 0.14ms | 0.17ms | 0.13ms |
| verify/forge-cam-quality/parts/deep_boss_p_zigzag_cam.fg | edit_part: part shared L x1.02 | 5.007 | 0.001 | 0.02ms | 4.803 | 0.088 | 0.079 | 0.01ms | 0.028 | 0.023 | 0.003 | 0.008 | 0.18ms | 0.14ms |
| verify/forge-cam-quality/parts/deep_islands_p_adaptive_cam.fg | rerun | 0.181 | 0.001 | 0.01ms | 0.001 | 0.173 | 0.002 | 0.00ms | 0.004 | 0.002 | 0.48ms | 0.22ms | 0.14ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_islands_p_adaptive_cam.fg | edit_last_op: rough (adaptive) | 20.307 | 0.001 | 0.01ms | 19.608 | 0.360 | 0.168 | 0.00ms | 0.146 | 0.134 | 0.008 | 0.023 | 0.23ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_islands_p_adaptive_cam.fg | edit_first_op: skim (facing) | 0.244 | 0.002 | 0.07ms | 0.001 | 0.173 | 0.056 | 0.01ms | 0.012 | 0.005 | 0.005 | 0.23ms | 0.14ms | 0.16ms |
| verify/forge-cam-quality/parts/deep_islands_p_adaptive_cam.fg | edit_tool: t1 stickout +1 | 10.051 | 0.001 | 0.02ms | 9.847 | 0.175 | 0.002 | 0.00ms | 0.026 | 0.024 | 0.47ms | 0.24ms | 0.18ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_islands_p_adaptive_cam.fg | edit_part: part shared L x1.02 | 9.784 | 0.001 | 0.02ms | 9.447 | 0.178 | 0.107 | 0.00ms | 0.035 | 0.029 | 0.004 | 0.015 | 0.15ms | 0.14ms |
| verify/forge-cam-quality/parts/deep_islands_p_clear_cam.fg | rerun | 0.046 | 0.001 | 0.02ms | 0.001 | 0.039 | 0.001 | 0.00ms | 0.004 | 0.002 | 0.44ms | 0.18ms | 0.14ms | 0.14ms |
| verify/forge-cam-quality/parts/deep_islands_p_clear_cam.fg | edit_last_op: rough (clear) | 0.313 | 0.001 | 0.01ms | 0.160 | 0.042 | 0.084 | 0.00ms | 0.026 | 0.021 | 0.003 | 0.18ms | 0.14ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_islands_p_clear_cam.fg | edit_first_op: skim (facing) | 0.091 | 0.001 | 0.02ms | 0.002 | 0.039 | 0.039 | 0.01ms | 0.011 | 0.005 | 0.004 | 0.18ms | 0.14ms | 0.13ms |
| verify/forge-cam-quality/parts/deep_islands_p_clear_cam.fg | edit_tool: t1 stickout +1 | 0.066 | 0.001 | 0.02ms | 0.001 | 0.039 | 0.001 | 0.00ms | 0.024 | 0.022 | 0.39ms | 0.18ms | 0.14ms | 0.12ms |
| verify/forge-cam-quality/parts/deep_islands_p_clear_cam.fg | edit_part: part shared L x1.02 | 0.287 | 0.001 | 0.02ms | 0.106 | 0.042 | 0.106 | 0.01ms | 0.031 | 0.026 | 0.003 | 0.002 | 0.13ms | 0.14ms |
| verify/forge-cam-quality/parts/deep_islands_p_zigzag_cam.fg | rerun | 0.122 | 0.001 | 0.02ms | 0.001 | 0.112 | 0.002 | 0.00ms | 0.005 | 0.002 | 0.001 | 0.24ms | 0.14ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_islands_p_zigzag_cam.fg | edit_last_op: rough (adaptive) | 23.426 | 0.001 | 0.02ms | 22.884 | 0.222 | 0.169 | 0.00ms | 0.125 | 0.111 | 0.010 | 0.024 | 0.16ms | 0.12ms |
| verify/forge-cam-quality/parts/deep_islands_p_zigzag_cam.fg | edit_first_op: skim (facing) | 0.164 | 0.001 | 0.02ms | 0.001 | 0.108 | 0.039 | 0.01ms | 0.014 | 0.005 | 0.006 | 0.25ms | 0.15ms | 0.15ms |
| verify/forge-cam-quality/parts/deep_islands_p_zigzag_cam.fg | edit_tool: t1 stickout +1 | 12.135 | 0.001 | 0.02ms | 11.985 | 0.110 | 0.002 | 0.00ms | 0.036 | 0.033 | 0.001 | 0.37ms | 0.22ms | 0.13ms |
| verify/forge-cam-quality/parts/deep_islands_p_zigzag_cam.fg | edit_part: part shared L x1.02 | 11.054 | 0.002 | 0.06ms | 10.773 | 0.110 | 0.111 | 0.01ms | 0.043 | 0.037 | 0.005 | 0.015 | 0.15ms | 0.18ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_adaptive_cam.fg | rerun | 0.008 | 0.001 | 0.01ms | 0.001 | 0.006 | 0.13ms | 0.00ms | 0.001 | 0.31ms | 0.05ms | 0.10ms | 0.12ms | 0.10ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_adaptive_cam.fg | edit_last_op: rough (adaptive) | 0.692 | 0.001 | 0.01ms | 0.633 | 0.014 | 0.012 | 0.00ms | 0.032 | 0.032 | 0.49ms | 0.18ms | 0.14ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_adaptive_cam.fg | edit_first_op: skim (facing) | 0.031 | 0.001 | 0.02ms | 0.001 | 0.005 | 0.021 | 0.00ms | 0.003 | 0.002 | 0.001 | 0.12ms | 0.12ms | 0.10ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_adaptive_cam.fg | edit_tool: t1 stickout +1 | 0.314 | 0.001 | 0.02ms | 0.277 | 0.006 | 0.21ms | 0.00ms | 0.031 | 0.031 | 0.06ms | 0.12ms | 0.13ms | 0.10ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_adaptive_cam.fg | edit_part: part shared L x1.02 | 0.257 | 0.001 | 0.01ms | 0.202 | 0.001 | 0.016 | 0.00ms | 0.028 | 0.028 | 0.35ms | 0.008 | 0.13ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_clear_cam.fg | rerun | 0.021 | 0.001 | 0.01ms | 0.001 | 0.018 | 0.20ms | 0.00ms | 0.001 | 0.50ms | 0.09ms | 0.13ms | 0.12ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_clear_cam.fg | edit_last_op: rough (clear) | 0.062 | 0.001 | 0.01ms | 0.007 | 0.019 | 0.019 | 0.00ms | 0.015 | 0.014 | 0.001 | 0.11ms | 0.13ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_clear_cam.fg | edit_first_op: skim (facing) | 0.045 | 0.001 | 0.02ms | 0.001 | 0.018 | 0.020 | 0.00ms | 0.005 | 0.004 | 0.001 | 0.10ms | 0.15ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_clear_cam.fg | edit_tool: t1 stickout +1 | 0.050 | 0.001 | 0.01ms | 0.001 | 0.018 | 0.26ms | 0.00ms | 0.029 | 0.029 | 0.10ms | 0.11ms | 0.13ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_clear_cam.fg | edit_part: part shared L x1.02 | 0.090 | 0.001 | 0.02ms | 0.009 | 0.018 | 0.027 | 0.01ms | 0.030 | 0.029 | 0.001 | 0.005 | 0.13ms | 0.11ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_zigzag_cam.fg | rerun | 0.010 | 0.001 | 0.02ms | 0.001 | 0.007 | 0.18ms | 0.00ms | 0.001 | 0.37ms | 0.07ms | 0.11ms | 0.13ms | 0.08ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_zigzag_cam.fg | edit_last_op: rough (adaptive) | 1.074 | 0.001 | 0.01ms | 1.010 | 0.017 | 0.015 | 0.00ms | 0.032 | 0.031 | 0.001 | 0.17ms | 0.16ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_zigzag_cam.fg | edit_first_op: skim (facing) | 0.033 | 0.001 | 0.02ms | 0.001 | 0.007 | 0.021 | 0.00ms | 0.003 | 0.002 | 0.001 | 0.12ms | 0.13ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_zigzag_cam.fg | edit_tool: t1 stickout +1 | 0.512 | 0.001 | 0.01ms | 0.473 | 0.007 | 0.20ms | 0.00ms | 0.031 | 0.030 | 0.07ms | 0.12ms | 0.13ms | 0.09ms |
| verify/forge-cam-quality/parts/deep_open_slot_p_zigzag_cam.fg | edit_part: part shared L x1.02 | 0.727 | 0.001 | 0.02ms | 0.655 | 0.012 | 0.018 | 0.00ms | 0.033 | 0.032 | 0.48ms | 0.008 | 0.13ms | 0.10ms |
| verify/forge-cam-quality/parts/deep_slot_cam.fg | rerun | 0.003 | 0.001 | 0.01ms | 0.001 | 0.00ms | 0.10ms | 0.00ms | 0.001 | 0.26ms | 0.08ms | 0.10ms | 0.11ms | 0.07ms |
| verify/forge-cam-quality/parts/deep_slot_cam.fg | edit_last_op: slot (pocket) | 0.009 | 0.47ms | 0.01ms | 0.001 | 0.000 | 0.006 | 0.00ms | 0.001 | 0.001 | 0.34ms | 0.09ms | 0.11ms | 0.06ms |
| verify/forge-cam-quality/parts/deep_slot_cam.fg | edit_first_op: skim (facing) | 0.025 | 0.001 | 0.01ms | 0.001 | 0.000 | 0.020 | 0.00ms | 0.003 | 0.002 | 0.001 | 0.09ms | 0.12ms | 0.08ms |
| verify/forge-cam-quality/parts/deep_slot_cam.fg | edit_tool: t2 stickout +1 | 0.006 | 0.001 | 0.01ms | 0.001 | 0.000 | 0.10ms | 0.00ms | 0.004 | 0.004 | 0.05ms | 0.09ms | 0.12ms | 0.08ms |
| verify/forge-cam-quality/parts/deep_slot_cam.fg | edit_part: part shared L x1.02 | 0.031 | 0.001 | 0.01ms | 0.001 | 0.000 | 0.017 | 0.00ms | 0.005 | 0.005 | 0.38ms | 0.006 | 0.13ms | 0.09ms |
| verify/forge-cam-quality/parts/open_step_cam.fg | rerun | 0.020 | 0.001 | 0.01ms | 0.001 | 0.017 | 0.22ms | 0.00ms | 0.001 | 0.40ms | 0.08ms | 0.06ms | 0.11ms | 0.10ms |
| verify/forge-cam-quality/parts/open_step_cam.fg | edit_last_op: stp (clear) | 0.130 | 0.001 | 0.01ms | 0.023 | 0.022 | 0.022 | 0.00ms | 0.062 | 0.061 | 0.001 | 0.08ms | 0.12ms | 0.10ms |
| verify/forge-cam-quality/parts/open_step_cam.fg | edit_first_op: skim (facing) | 0.040 | 0.001 | 0.02ms | 0.001 | 0.017 | 0.018 | 0.01ms | 0.003 | 0.002 | 0.001 | 0.07ms | 0.12ms | 0.10ms |
| verify/forge-cam-quality/parts/open_step_cam.fg | edit_tool: t1 stickout +1 | 0.093 | 0.001 | 0.01ms | 0.001 | 0.018 | 0.22ms | 0.00ms | 0.073 | 0.072 | 0.08ms | 0.12ms | 0.17ms | 0.11ms |
| verify/forge-cam-quality/parts/open_step_cam.fg | edit_part: part shared L x1.02 | 0.139 | 0.001 | 0.02ms | 0.012 | 0.024 | 0.035 | 0.01ms | 0.066 | 0.065 | 0.001 | 0.001 | 0.12ms | 0.13ms |
| verify/forge-cam-quality/parts/pocket_islands_cam.fg | rerun | 0.004 | 0.001 | 0.01ms | 0.001 | 0.00ms | 0.21ms | 0.00ms | 0.001 | 0.44ms | 0.09ms | 0.14ms | 0.12ms | 0.08ms |
| verify/forge-cam-quality/parts/pocket_islands_cam.fg | edit_last_op: well (pocket) | 0.052 | 0.002 | 0.02ms | 0.007 | 0.00ms | 0.038 | 0.00ms | 0.005 | 0.004 | 0.001 | 0.22ms | 0.23ms | 0.10ms |
| verify/forge-cam-quality/parts/pocket_islands_cam.fg | edit_first_op: skim (facing) | 0.037 | 0.002 | 0.02ms | 0.001 | 0.00ms | 0.030 | 0.00ms | 0.003 | 0.002 | 0.001 | 0.16ms | 0.12ms | 0.14ms |
| verify/forge-cam-quality/parts/pocket_islands_cam.fg | edit_tool: t1 stickout +1 | 0.012 | 0.001 | 0.01ms | 0.001 | 0.000 | 0.20ms | 0.00ms | 0.009 | 0.008 | 0.10ms | 0.16ms | 0.12ms | 0.10ms |
| verify/forge-cam-quality/parts/pocket_islands_cam.fg | edit_part: part shared L x1.02 | 0.071 | 0.001 | 0.01ms | 0.006 | 0.00ms | 0.045 | 0.01ms | 0.009 | 0.008 | 0.001 | 0.008 | 0.13ms | 0.12ms |
| verify/forge-cam-quality/parts/rptu/rptu1_clear_cam.fg | rerun | 0.008 | 0.34ms | 0.04ms | 0.001 | 0.005 | 0.15ms | 0.00ms | 0.001 | 0.36ms | 0.09ms | 0.09ms | 0.11ms | 0.08ms |
| verify/forge-cam-quality/parts/rptu/rptu1_clear_cam.fg | edit_last_op: nut (clear) | 0.013 | 0.29ms | 0.04ms | 0.001 | 0.005 | 0.004 | 0.00ms | 0.002 | 0.001 | 0.42ms | 0.09ms | 0.11ms | 0.07ms |
| verify/forge-cam-quality/parts/rptu/rptu1_clear_cam.fg | edit_first_op: skim (facing) | 0.043 | 0.29ms | 0.01ms | 0.001 | 0.005 | 0.032 | 0.00ms | 0.004 | 0.003 | 0.001 | 0.10ms | 0.11ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu1_clear_cam.fg | edit_tool: t1 stickout +1 | 0.728 | 0.36ms | 0.01ms | 0.001 | 0.005 | 0.14ms | 0.00ms | 0.721 | 0.720 | 0.09ms | 0.12ms | 0.12ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu2_none_cam.fg | rerun | 0.003 | 0.48ms | 0.01ms | 0.001 | 0.000 | 0.12ms | 0.00ms | 0.001 | 0.001 | 0.08ms | 0.16ms | 0.10ms | 0.07ms |
| verify/forge-cam-quality/parts/rptu/rptu2_none_cam.fg | edit_last_op: holes (pocket) | 0.007 | 0.38ms | 0.01ms | 0.001 | 0.000 | 0.003 | 0.00ms | 0.002 | 0.001 | 0.42ms | 0.16ms | 0.10ms | 0.12ms |
| verify/forge-cam-quality/parts/rptu/rptu2_none_cam.fg | edit_first_op: skim (facing) | 0.041 | 0.34ms | 0.01ms | 0.001 | 0.000 | 0.032 | 0.00ms | 0.006 | 0.005 | 0.001 | 0.19ms | 0.11ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu2_none_cam.fg | edit_tool: t1 stickout +1 | 0.007 | 0.45ms | 0.01ms | 0.001 | 0.000 | 0.12ms | 0.00ms | 0.005 | 0.007 | 0.08ms | 0.17ms | 0.11ms | 0.08ms |
| verify/forge-cam-quality/parts/rptu/rptu3_adaptive_cam.fg | rerun | 0.017 | 0.49ms | 0.01ms | 0.002 | 0.013 | 0.29ms | 0.00ms | 0.001 | 0.001 | 0.12ms | 0.24ms | 0.14ms | 0.11ms |
| verify/forge-cam-quality/parts/rptu/rptu3_adaptive_cam.fg | edit_last_op: holes (pocket) | 0.022 | 0.49ms | 0.01ms | 0.002 | 0.013 | 0.003 | 0.00ms | 0.003 | 0.002 | 0.001 | 0.26ms | 0.14ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu3_adaptive_cam.fg | edit_first_op: skim (facing) | 0.054 | 0.001 | 0.01ms | 0.002 | 0.013 | 0.032 | 0.01ms | 0.006 | 0.004 | 0.001 | 0.28ms | 0.15ms | 0.11ms |
| verify/forge-cam-quality/parts/rptu/rptu3_adaptive_cam.fg | edit_tool: t1 stickout +1 | 0.350 | 0.001 | 0.01ms | 0.329 | 0.013 | 0.33ms | 0.00ms | 0.007 | 0.010 | 0.12ms | 0.28ms | 0.15ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu3_clear_cam.fg | rerun | 0.008 | 0.001 | 0.01ms | 0.002 | 0.004 | 0.18ms | 0.00ms | 0.001 | 0.001 | 0.10ms | 0.23ms | 0.16ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu3_clear_cam.fg | edit_last_op: holes (pocket) | 0.013 | 0.43ms | 0.01ms | 0.002 | 0.004 | 0.003 | 0.00ms | 0.003 | 0.002 | 0.001 | 0.22ms | 0.13ms | 0.08ms |
| verify/forge-cam-quality/parts/rptu/rptu3_clear_cam.fg | edit_first_op: skim (facing) | 0.045 | 0.42ms | 0.01ms | 0.002 | 0.004 | 0.032 | 0.00ms | 0.006 | 0.005 | 0.001 | 0.23ms | 0.17ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu3_clear_cam.fg | edit_tool: t1 stickout +1 | 0.014 | 0.001 | 0.01ms | 0.002 | 0.004 | 0.18ms | 0.00ms | 0.007 | 0.011 | 0.13ms | 0.25ms | 0.14ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu3_zigzag_cam.fg | rerun | 0.014 | 0.001 | 0.01ms | 0.002 | 0.009 | 0.26ms | 0.00ms | 0.001 | 0.001 | 0.11ms | 0.26ms | 0.14ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu3_zigzag_cam.fg | edit_last_op: holes (pocket) | 0.018 | 0.47ms | 0.01ms | 0.002 | 0.009 | 0.003 | 0.00ms | 0.003 | 0.002 | 0.001 | 0.27ms | 0.14ms | 0.08ms |
| verify/forge-cam-quality/parts/rptu/rptu3_zigzag_cam.fg | edit_first_op: skim (facing) | 0.050 | 0.49ms | 0.01ms | 0.002 | 0.009 | 0.032 | 0.00ms | 0.006 | 0.004 | 0.001 | 0.28ms | 0.14ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu3_zigzag_cam.fg | edit_tool: t1 stickout +1 | 0.592 | 0.50ms | 0.01ms | 0.575 | 0.009 | 0.35ms | 0.00ms | 0.007 | 0.010 | 0.12ms | 0.30ms | 0.15ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu4_adaptive_cam.fg | rerun | 0.035 | 0.001 | 0.02ms | 0.002 | 0.028 | 0.001 | 0.01ms | 0.002 | 0.001 | 0.18ms | 0.46ms | 0.18ms | 0.11ms |
| verify/forge-cam-quality/parts/rptu/rptu4_adaptive_cam.fg | edit_last_op: holes (pocket) | 0.039 | 0.001 | 0.02ms | 0.002 | 0.028 | 0.004 | 0.01ms | 0.004 | 0.002 | 0.001 | 0.45ms | 0.21ms | 0.15ms |
| verify/forge-cam-quality/parts/rptu/rptu4_adaptive_cam.fg | edit_first_op: skim (facing) | 0.082 | 0.001 | 0.02ms | 0.002 | 0.028 | 0.042 | 0.01ms | 0.008 | 0.006 | 0.001 | 0.50ms | 0.18ms | 0.14ms |
| verify/forge-cam-quality/parts/rptu/rptu4_adaptive_cam.fg | edit_tool: t1 stickout +1 | 1.666 | 0.001 | 0.02ms | 1.621 | 0.028 | 0.001 | 0.01ms | 0.015 | 0.018 | 0.19ms | 0.48ms | 0.19ms | 0.12ms |
| verify/forge-cam-quality/parts/rptu/rptu4_clear_cam.fg | rerun | 0.032 | 0.001 | 0.02ms | 0.003 | 0.025 | 0.38ms | 0.00ms | 0.002 | 0.001 | 0.24ms | 0.38ms | 0.15ms | 0.13ms |
| verify/forge-cam-quality/parts/rptu/rptu4_clear_cam.fg | edit_last_op: holes (pocket) | 0.028 | 0.001 | 0.01ms | 0.002 | 0.018 | 0.004 | 0.00ms | 0.004 | 0.002 | 0.001 | 0.29ms | 0.15ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu4_clear_cam.fg | edit_first_op: skim (facing) | 0.071 | 0.001 | 0.02ms | 0.002 | 0.017 | 0.042 | 0.01ms | 0.009 | 0.007 | 0.001 | 0.34ms | 0.15ms | 0.16ms |
| verify/forge-cam-quality/parts/rptu/rptu4_clear_cam.fg | edit_tool: t1 stickout +1 | 0.032 | 0.001 | 0.01ms | 0.002 | 0.017 | 0.30ms | 0.00ms | 0.012 | 0.019 | 0.16ms | 0.30ms | 0.16ms | 0.13ms |
| verify/forge-cam-quality/parts/rptu/rptu4_zigzag_cam.fg | rerun | 0.038 | 0.001 | 0.01ms | 0.002 | 0.032 | 0.001 | 0.00ms | 0.002 | 0.001 | 0.15ms | 0.42ms | 0.18ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu4_zigzag_cam.fg | edit_last_op: holes (pocket) | 0.043 | 0.001 | 0.01ms | 0.002 | 0.032 | 0.004 | 0.00ms | 0.004 | 0.002 | 0.001 | 0.42ms | 0.17ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu4_zigzag_cam.fg | edit_first_op: skim (facing) | 0.084 | 0.001 | 0.01ms | 0.002 | 0.032 | 0.042 | 0.01ms | 0.007 | 0.005 | 0.001 | 0.42ms | 0.17ms | 0.12ms |
| verify/forge-cam-quality/parts/rptu/rptu4_zigzag_cam.fg | edit_tool: t1 stickout +1 | 1.719 | 0.001 | 0.01ms | 1.667 | 0.036 | 0.001 | 0.01ms | 0.014 | 0.021 | 0.16ms | 0.46ms | 0.22ms | 0.13ms |
| verify/forge-cam-quality/parts/rptu/rptu5_adaptive_cam.fg | rerun | 0.020 | 0.001 | 0.01ms | 0.002 | 0.015 | 0.45ms | 0.00ms | 0.002 | 0.001 | 0.14ms | 0.41ms | 0.17ms | 0.11ms |
| verify/forge-cam-quality/parts/rptu/rptu5_adaptive_cam.fg | edit_last_op: holes (pocket) | 0.025 | 0.001 | 0.01ms | 0.002 | 0.015 | 0.004 | 0.00ms | 0.004 | 0.002 | 0.001 | 0.38ms | 0.17ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu5_adaptive_cam.fg | edit_first_op: skim (facing) | 0.066 | 0.001 | 0.01ms | 0.002 | 0.014 | 0.040 | 0.00ms | 0.009 | 0.007 | 0.001 | 0.38ms | 0.22ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu5_adaptive_cam.fg | edit_tool: t1 stickout +1 | 0.826 | 0.001 | 0.01ms | 0.800 | 0.015 | 0.46ms | 0.00ms | 0.010 | 0.018 | 0.16ms | 0.38ms | 0.17ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu5_clear_cam.fg | rerun | 0.024 | 0.001 | 0.02ms | 0.003 | 0.017 | 0.40ms | 0.01ms | 0.003 | 0.001 | 0.26ms | 0.44ms | 0.21ms | 0.12ms |
| verify/forge-cam-quality/parts/rptu/rptu5_clear_cam.fg | edit_last_op: holes (pocket) | 0.030 | 0.001 | 0.02ms | 0.003 | 0.016 | 0.006 | 0.01ms | 0.005 | 0.003 | 0.001 | 0.41ms | 0.19ms | 0.12ms |
| verify/forge-cam-quality/parts/rptu/rptu5_clear_cam.fg | edit_first_op: skim (facing) | 0.093 | 0.001 | 0.02ms | 0.003 | 0.010 | 0.070 | 0.01ms | 0.010 | 0.008 | 0.001 | 0.30ms | 0.15ms | 0.15ms |
| verify/forge-cam-quality/parts/rptu/rptu5_clear_cam.fg | edit_tool: t1 stickout +1 | 0.023 | 0.001 | 0.01ms | 0.002 | 0.010 | 0.25ms | 0.00ms | 0.011 | 0.018 | 0.14ms | 0.29ms | 0.17ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu5_zigzag_cam.fg | rerun | 0.016 | 0.001 | 0.02ms | 0.002 | 0.011 | 0.41ms | 0.00ms | 0.002 | 0.001 | 0.13ms | 0.38ms | 0.18ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu5_zigzag_cam.fg | edit_last_op: holes (pocket) | 0.021 | 0.001 | 0.01ms | 0.002 | 0.011 | 0.004 | 0.00ms | 0.003 | 0.002 | 0.001 | 0.35ms | 0.17ms | 0.11ms |
| verify/forge-cam-quality/parts/rptu/rptu5_zigzag_cam.fg | edit_first_op: skim (facing) | 0.061 | 0.001 | 0.01ms | 0.002 | 0.011 | 0.040 | 0.01ms | 0.007 | 0.005 | 0.001 | 0.38ms | 0.18ms | 0.13ms |
| verify/forge-cam-quality/parts/rptu/rptu5_zigzag_cam.fg | edit_tool: t1 stickout +1 | 0.832 | 0.001 | 0.01ms | 0.810 | 0.011 | 0.40ms | 0.00ms | 0.010 | 0.015 | 0.15ms | 0.40ms | 0.18ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu6_adaptive_cam.fg | rerun | 0.019 | 0.001 | 0.01ms | 0.002 | 0.013 | 0.43ms | 0.00ms | 0.002 | 0.001 | 0.22ms | 0.29ms | 0.14ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu6_adaptive_cam.fg | edit_last_op: holes (pocket) | 0.060 | 0.001 | 0.01ms | 0.003 | 0.013 | 0.030 | 0.00ms | 0.012 | 0.009 | 0.002 | 0.34ms | 0.15ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu6_adaptive_cam.fg | edit_first_op: skim (facing) | 0.057 | 0.001 | 0.01ms | 0.002 | 0.013 | 0.033 | 0.01ms | 0.009 | 0.006 | 0.002 | 0.35ms | 0.15ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu6_adaptive_cam.fg | edit_tool: t1 stickout +1 | 0.363 | 0.001 | 0.02ms | 0.329 | 0.013 | 0.41ms | 0.00ms | 0.020 | 0.028 | 0.23ms | 0.34ms | 0.16ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu6_clear_cam.fg | rerun | 0.010 | 0.001 | 0.01ms | 0.002 | 0.004 | 0.32ms | 0.00ms | 0.002 | 0.001 | 0.21ms | 0.29ms | 0.14ms | 0.09ms |
| verify/forge-cam-quality/parts/rptu/rptu6_clear_cam.fg | edit_last_op: holes (pocket) | 0.053 | 0.001 | 0.01ms | 0.003 | 0.004 | 0.032 | 0.01ms | 0.013 | 0.010 | 0.002 | 0.30ms | 0.18ms | 0.13ms |
| verify/forge-cam-quality/parts/rptu/rptu6_clear_cam.fg | edit_first_op: skim (facing) | 0.049 | 0.001 | 0.02ms | 0.002 | 0.004 | 0.033 | 0.01ms | 0.009 | 0.006 | 0.002 | 0.35ms | 0.15ms | 0.13ms |
| verify/forge-cam-quality/parts/rptu/rptu6_clear_cam.fg | edit_tool: t1 stickout +1 | 0.026 | 0.001 | 0.02ms | 0.002 | 0.004 | 0.34ms | 0.00ms | 0.019 | 0.026 | 0.22ms | 0.35ms | 0.19ms | 0.13ms |
| verify/forge-cam-quality/parts/rptu/rptu6_zigzag_cam.fg | rerun | 0.015 | 0.001 | 0.02ms | 0.002 | 0.009 | 0.39ms | 0.00ms | 0.002 | 0.001 | 0.21ms | 0.32ms | 0.14ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu6_zigzag_cam.fg | edit_last_op: holes (pocket) | 0.059 | 0.001 | 0.01ms | 0.003 | 0.009 | 0.031 | 0.00ms | 0.014 | 0.011 | 0.002 | 0.40ms | 0.19ms | 0.10ms |
| verify/forge-cam-quality/parts/rptu/rptu6_zigzag_cam.fg | edit_first_op: skim (facing) | 0.058 | 0.001 | 0.02ms | 0.002 | 0.009 | 0.036 | 0.01ms | 0.009 | 0.006 | 0.002 | 0.37ms | 0.19ms | 0.14ms |
| verify/forge-cam-quality/parts/rptu/rptu6_zigzag_cam.fg | edit_tool: t1 stickout +1 | 0.583 | 0.001 | 0.01ms | 0.552 | 0.011 | 0.45ms | 0.01ms | 0.018 | 0.026 | 0.24ms | 0.38ms | 0.17ms | 0.13ms |
| verify/forge-cam-quality/parts/thin_wall_cam.fg | rerun | 0.005 | 0.001 | 0.02ms | 0.001 | 0.00ms | 0.45ms | 0.00ms | 0.002 | 0.001 | 0.22ms | 0.11ms | 0.12ms | 0.09ms |
| verify/forge-cam-quality/parts/thin_wall_cam.fg | edit_last_op: wells (pocket) | 0.077 | 0.001 | 0.01ms | 0.003 | 0.000 | 0.060 | 0.00ms | 0.013 | 0.010 | 0.002 | 0.15ms | 0.14ms | 0.09ms |
| verify/forge-cam-quality/parts/thin_wall_cam.fg | edit_first_op: skim (facing) | 0.035 | 0.001 | 0.02ms | 0.001 | 0.00ms | 0.026 | 0.01ms | 0.006 | 0.003 | 0.002 | 0.13ms | 0.13ms | 0.11ms |
| verify/forge-cam-quality/parts/thin_wall_cam.fg | edit_tool: t2 stickout +1 | 0.013 | 0.001 | 0.02ms | 0.001 | 0.000 | 0.43ms | 0.00ms | 0.010 | 0.015 | 0.18ms | 0.15ms | 0.13ms | 0.09ms |
| verify/forge-cam-quality/parts/thin_wall_cam.fg | edit_part: part shared L x1.02 | 0.096 | 0.001 | 0.01ms | 0.004 | 0.00ms | 0.070 | 0.00ms | 0.011 | 0.013 | 0.001 | 0.011 | 0.13ms | 0.12ms |

## Cold, per op

gen = the op's statement less hops, feeds, stock; cert = its certification when posted (G1-G3, fixture, G5).

| doc | op | kind | gen | hops | feeds | stock | cert |
|---|---|---|---|---|---|---|---|
| examples/machined/block_m6_cam.fg | skim | facing | 0.001 | 0.00ms | 0.004 | 0.00ms | 0.001 |
| examples/machined/block_m6_cam.fg | side | clear | 0.003 | 0.002 | 0.007 | 0.00ms | 0.004 |
| examples/machined/block_m6_cam.fg | tapd | drill | 0.20ms | 0.00ms | 0.000 | 0.00ms | 0.003 |
| examples/machined/block_m6_cam.fg | thr | thread_mill | 0.001 | 0.000 | 0.000 | 0.00ms | 0.003 |
| examples/machined/bracket_cam.fg | skim | facing | 0.40ms | 0.00ms | 0.014 | 0.00ms | 0.002 |
| examples/machined/bracket_cam.fg | side | clear | 0.021 | 0.021 | 0.009 | 0.00ms | 0.005 |
| examples/machined/bracket_cam.fg | bore | pocket | 0.001 | 0.000 | 0.011 | 0.00ms | 0.005 |
| examples/machined/bracket_cam.fg | cutj | pocket | 0.077 | 0.00ms | 0.036 | 0.00ms | 0.010 |
| examples/machined/bracket_cam.fg | rest | clear | 0.021 | 0.017 | 0.008 | 0.00ms | 0.011 |
| examples/machined/flange_cam.fg | skim | facing | 0.39ms | 0.00ms | 0.026 | 0.00ms | 0.002 |
| examples/machined/flange_cam.fg | top | clear | 0.056 | 0.024 | 0.028 | 0.00ms | 0.008 |
| examples/machined/flange_cam.fg | prof | contour | 0.002 | 0.001 | 0.009 | 0.00ms | 0.004 |
| examples/machined/flange_cam.fg | bore | pocket | 0.001 | 0.000 | 0.005 | 0.00ms | 0.005 |
| examples/machined/flange_cam.fg | bolts | drill | 0.001 | 0.000 | 0.000 | 0.00ms | 0.003 |
| examples/machined/manifold_cam.fg | skim | facing | 0.001 | 0.00ms | 0.037 | 0.00ms | 0.001 |
| examples/machined/manifold_cam.fg | prof | contour | 0.010 | 0.009 | 0.040 | 0.00ms | 0.107 |
| examples/machined/manifold_cam.fg | well | pocket | 0.007 | 0.00ms | 0.073 | 0.00ms | 0.048 |
| examples/machined/manifold_cam.fg | crn | pocket | 0.016 | 0.00ms | 0.035 | 0.00ms | 0.159 |
| examples/machined/manifold_cam.fg | shal | pocket | 0.009 | 0.00ms | 0.009 | 0.00ms | 0.024 |
| examples/machined/manifold_cam.fg | grv | slot | 0.003 | 0.00ms | 0.005 | 0.00ms | 0.018 |
| examples/machined/manifold_cam.fg | bored | drill | 0.14ms | 0.000 | 0.000 | 0.00ms | 0.003 |
| examples/machined/manifold_cam.fg | dwl | drill | 0.008 | 0.000 | 0.000 | 0.00ms | 0.063 |
| examples/machined/manifold_cam.fg | tap | drill | 0.014 | 0.000 | 0.000 | 0.00ms | 0.089 |
| examples/machined/manifold_cam.fg | brk | edge_break | 0.005 | 0.000 | 0.000 | 0.000 | 0.020 |
| examples/machined/manifold_cam.fg | face2 | facing | 0.001 | 0.00ms | 0.058 | 0.00ms | 0.002 |
| examples/machined/manifold_cam.fg | rel | pocket | 0.006 | 0.00ms | 0.011 | 0.00ms | 0.006 |
| examples/machined/manifold_cam.fg | cbh | drill | 0.23ms | 0.00ms | 0.000 | 0.00ms | 0.097 |
| examples/machined/manifold_cam.fg | rough | clear | 0.001 | 0.000 | 0.000 | 0.000 | 0.000 |
| examples/machined/manifold_cam.fg | thr | thread_mill | 0.001 | 0.000 | 0.000 | 0.000 | 0.000 |
| examples/machined/mold_core_cam.fg | rough | zlevel | 0.529 | 0.110 | 0.000 | 0.00ms | 0.031 |
| examples/machined/mold_core_cam.fg | fin | parallel | 0.353 | 0.025 | 0.00ms | 0.00ms | 0.362 |
| examples/machined/mold_core_cam.fg | walls | waterline | 1.096 | 0.005 | 0.000 | 0.00ms | 0.306 |
| examples/machined/mold_core_cam.fg | corner | pencil | 0.586 | 0.003 | 0.000 | 0.00ms | 0.265 |
| examples/machined/mold_core_cam.fg | left | rest | 2.282 | 0.016 | 0.00ms | 0.00ms | 0.264 |
| examples/machined/mold_core_finish_cam.fg | rough | zlevel | 0.518 | 0.109 | 0.000 | 0.00ms | 0.024 |
| examples/machined/mold_core_finish_cam.fg | ss | steep_shallow | 6.371 | 0.082 | 0.000 | 0.00ms | 0.477 |
| examples/machined/mold_core_finish_cam.fg | top | spiral | 0.680 | 0.057 | 0.00ms | 0.00ms | 0.128 |
| examples/machined/mold_core_finish_cam.fg | plate | flat | 1.733 | 0.011 | 0.000 | 0.00ms | 0.315 |
| examples/machined/plate_break_cam.fg | skim | facing | 0.46ms | 0.00ms | 0.006 | 0.00ms | 0.001 |
| examples/machined/plate_break_cam.fg | side | clear | 0.034 | 0.020 | 0.010 | 0.00ms | 0.007 |
| examples/machined/plate_break_cam.fg | bore | pocket | 0.001 | 0.000 | 0.001 | 0.00ms | 0.002 |
| examples/machined/plate_break_cam.fg | brk | edge_break | 0.003 | 0.000 | 0.000 | 0.000 | 0.009 |
| examples/machined/plate_mark_cam.fg | skim | facing | 0.42ms | 0.00ms | 0.003 | 0.00ms | 0.001 |
| examples/machined/plate_mark_cam.fg | side | clear | 0.002 | 0.001 | 0.003 | 0.00ms | 0.002 |
| examples/machined/plate_mark_cam.fg | mark | engrave | 0.001 | 0.000 | 0.000 | 0.00ms | 0.27ms |
| examples/machined/slot_plate_cam.fg | skim | facing | 0.38ms | 0.00ms | 0.006 | 0.00ms | 0.001 |
| examples/machined/slot_plate_cam.fg | slots | slot | 0.001 | 0.00ms | 0.011 | 0.00ms | 0.003 |
| examples/machined/well_island_cam.fg | skim | facing | 0.001 | 0.00ms | 0.008 | 0.00ms | 0.002 |
| examples/machined/well_island_cam.fg | well | pocket | 0.004 | 0.00ms | 0.023 | 0.00ms | 0.007 |
| verify/forge-cam-quality/parts/bilek/bilek_cam.fg | well | pocket | 0.011 | 0.00ms | 0.056 | 0.00ms | 0.008 |
| verify/forge-cam-quality/parts/boss_adaptive_cam.fg | skim | facing | 0.35ms | 0.00ms | 0.011 | 0.00ms | 0.001 |
| verify/forge-cam-quality/parts/boss_adaptive_cam.fg | rough | adaptive | 0.481 | 0.202 | 0.148 | 0.00ms | 0.033 |
| verify/forge-cam-quality/parts/boss_adaptive_cam.fg | fin | contour | 0.002 | 0.001 | 0.004 | 0.00ms | 0.001 |
| verify/forge-cam-quality/parts/boss_cam.fg | skim | facing | 0.40ms | 0.00ms | 0.010 | 0.00ms | 0.001 |
| verify/forge-cam-quality/parts/boss_cam.fg | rough | clear | 0.020 | 0.018 | 0.054 | 0.00ms | 0.007 |
| verify/forge-cam-quality/parts/boss_cam.fg | fin | contour | 0.002 | 0.001 | 0.002 | 0.00ms | 0.001 |
| verify/forge-cam-quality/parts/deep_boss_n_adaptive_cam.fg | skim | facing | 0.34ms | 0.00ms | 0.015 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/deep_boss_n_adaptive_cam.fg | rough | adaptive | 2.667 | 0.079 | 0.035 | 0.00ms | 0.006 |
| verify/forge-cam-quality/parts/deep_boss_n_adaptive_cam.fg | fin | contour | 0.002 | 0.001 | 0.002 | 0.00ms | 0.001 |
| verify/forge-cam-quality/parts/deep_boss_n_clear_cam.fg | skim | facing | 0.001 | 0.00ms | 0.026 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/deep_boss_n_clear_cam.fg | rough | clear | 0.054 | 0.013 | 0.052 | 0.00ms | 0.008 |
| verify/forge-cam-quality/parts/deep_boss_n_clear_cam.fg | fin | contour | 0.002 | 0.001 | 0.002 | 0.00ms | 0.001 |
| verify/forge-cam-quality/parts/deep_boss_n_zigzag_cam.fg | skim | facing | 0.35ms | 0.00ms | 0.016 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/deep_boss_n_zigzag_cam.fg | rough | adaptive | 2.166 | 0.044 | 0.039 | 0.00ms | 0.011 |
| verify/forge-cam-quality/parts/deep_boss_n_zigzag_cam.fg | fin | contour | 0.002 | 0.001 | 0.002 | 0.00ms | 0.001 |
| verify/forge-cam-quality/parts/deep_boss_p_adaptive_cam.fg | skim | facing | 0.32ms | 0.00ms | 0.016 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/deep_boss_p_adaptive_cam.fg | rough | adaptive | 2.718 | 0.079 | 0.035 | 0.00ms | 0.006 |
| verify/forge-cam-quality/parts/deep_boss_p_adaptive_cam.fg | fin | contour | 0.002 | 0.001 | 0.002 | 0.00ms | 0.001 |
| verify/forge-cam-quality/parts/deep_boss_p_clear_cam.fg | skim | facing | 0.30ms | 0.00ms | 0.016 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/deep_boss_p_clear_cam.fg | rough | clear | 0.050 | 0.014 | 0.061 | 0.00ms | 0.010 |
| verify/forge-cam-quality/parts/deep_boss_p_clear_cam.fg | fin | contour | 0.002 | 0.001 | 0.002 | 0.00ms | 0.001 |
| verify/forge-cam-quality/parts/deep_boss_p_zigzag_cam.fg | skim | facing | 0.30ms | 0.00ms | 0.015 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/deep_boss_p_zigzag_cam.fg | rough | adaptive | 2.241 | 0.033 | 0.034 | 0.00ms | 0.011 |
| verify/forge-cam-quality/parts/deep_boss_p_zigzag_cam.fg | fin | contour | 0.002 | 0.001 | 0.002 | 0.00ms | 0.001 |
| verify/forge-cam-quality/parts/deep_islands_p_adaptive_cam.fg | skim | facing | 0.34ms | 0.00ms | 0.019 | 0.00ms | 0.003 |
| verify/forge-cam-quality/parts/deep_islands_p_adaptive_cam.fg | rough | adaptive | 10.125 | 0.173 | 0.087 | 0.00ms | 0.030 |
| verify/forge-cam-quality/parts/deep_islands_p_clear_cam.fg | skim | facing | 0.29ms | 0.00ms | 0.020 | 0.00ms | 0.003 |
| verify/forge-cam-quality/parts/deep_islands_p_clear_cam.fg | rough | clear | 0.127 | 0.060 | 0.106 | 0.00ms | 0.011 |
| verify/forge-cam-quality/parts/deep_islands_p_zigzag_cam.fg | skim | facing | 0.43ms | 0.00ms | 0.019 | 0.00ms | 0.005 |
| verify/forge-cam-quality/parts/deep_islands_p_zigzag_cam.fg | rough | adaptive | 11.942 | 0.114 | 0.093 | 0.00ms | 0.038 |
| verify/forge-cam-quality/parts/deep_open_slot_p_adaptive_cam.fg | skim | facing | 0.29ms | 0.00ms | 0.011 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/deep_open_slot_p_adaptive_cam.fg | rough | adaptive | 0.281 | 0.005 | 0.006 | 0.00ms | 0.031 |
| verify/forge-cam-quality/parts/deep_open_slot_p_clear_cam.fg | skim | facing | 0.28ms | 0.00ms | 0.010 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/deep_open_slot_p_clear_cam.fg | rough | clear | 0.009 | 0.018 | 0.016 | 0.00ms | 0.027 |
| verify/forge-cam-quality/parts/deep_open_slot_p_zigzag_cam.fg | skim | facing | 0.46ms | 0.00ms | 0.016 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/deep_open_slot_p_zigzag_cam.fg | rough | adaptive | 0.486 | 0.007 | 0.008 | 0.00ms | 0.030 |
| verify/forge-cam-quality/parts/deep_slot_cam.fg | skim | facing | 0.39ms | 0.00ms | 0.011 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/deep_slot_cam.fg | slot | pocket | 0.001 | 0.00ms | 0.006 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/open_step_cam.fg | skim | facing | 0.38ms | 0.00ms | 0.009 | 0.00ms | 0.001 |
| verify/forge-cam-quality/parts/open_step_cam.fg | stp | clear | 0.016 | 0.017 | 0.021 | 0.00ms | 0.072 |
| verify/forge-cam-quality/parts/pocket_islands_cam.fg | skim | facing | 0.39ms | 0.00ms | 0.014 | 0.00ms | 0.002 |
| verify/forge-cam-quality/parts/pocket_islands_cam.fg | well | pocket | 0.007 | 0.00ms | 0.030 | 0.00ms | 0.006 |
| verify/forge-cam-quality/parts/rptu/rptu1_clear_cam.fg | skim | facing | 0.39ms | 0.00ms | 0.017 | 0.00ms | 0.003 |
| verify/forge-cam-quality/parts/rptu/rptu1_clear_cam.fg | nut | clear | 0.002 | 0.006 | 0.004 | 0.00ms | 0.758 |
| verify/forge-cam-quality/parts/rptu/rptu2_none_cam.fg | skim | facing | 0.33ms | 0.00ms | 0.017 | 0.00ms | 0.003 |
| verify/forge-cam-quality/parts/rptu/rptu2_none_cam.fg | holes | pocket | 0.001 | 0.00ms | 0.003 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu3_adaptive_cam.fg | skim | facing | 0.35ms | 0.00ms | 0.017 | 0.00ms | 0.003 |
| verify/forge-cam-quality/parts/rptu/rptu3_adaptive_cam.fg | step | adaptive | 0.331 | 0.013 | 0.008 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu3_adaptive_cam.fg | holes | pocket | 0.001 | 0.00ms | 0.003 | 0.00ms | 0.005 |
| verify/forge-cam-quality/parts/rptu/rptu3_clear_cam.fg | skim | facing | 0.45ms | 0.00ms | 0.017 | 0.00ms | 0.003 |
| verify/forge-cam-quality/parts/rptu/rptu3_clear_cam.fg | step | clear | 0.009 | 0.004 | 0.007 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu3_clear_cam.fg | holes | pocket | 0.001 | 0.00ms | 0.003 | 0.00ms | 0.005 |
| verify/forge-cam-quality/parts/rptu/rptu3_zigzag_cam.fg | skim | facing | 0.36ms | 0.00ms | 0.017 | 0.00ms | 0.003 |
| verify/forge-cam-quality/parts/rptu/rptu3_zigzag_cam.fg | step | adaptive | 0.536 | 0.009 | 0.008 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu3_zigzag_cam.fg | holes | pocket | 0.001 | 0.000 | 0.003 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu4_adaptive_cam.fg | skim | facing | 0.35ms | 0.00ms | 0.021 | 0.00ms | 0.005 |
| verify/forge-cam-quality/parts/rptu/rptu4_adaptive_cam.fg | step | adaptive | 1.628 | 0.028 | 0.017 | 0.00ms | 0.031 |
| verify/forge-cam-quality/parts/rptu/rptu4_adaptive_cam.fg | holes | pocket | 0.002 | 0.00ms | 0.004 | 0.00ms | 0.006 |
| verify/forge-cam-quality/parts/rptu/rptu4_clear_cam.fg | skim | facing | 0.35ms | 0.00ms | 0.022 | 0.00ms | 0.005 |
| verify/forge-cam-quality/parts/rptu/rptu4_clear_cam.fg | step | clear | 0.053 | 0.017 | 0.016 | 0.00ms | 0.006 |
| verify/forge-cam-quality/parts/rptu/rptu4_clear_cam.fg | holes | pocket | 0.002 | 0.00ms | 0.003 | 0.00ms | 0.007 |
| verify/forge-cam-quality/parts/rptu/rptu4_zigzag_cam.fg | skim | facing | 0.35ms | 0.00ms | 0.022 | 0.00ms | 0.005 |
| verify/forge-cam-quality/parts/rptu/rptu4_zigzag_cam.fg | step | adaptive | 1.638 | 0.032 | 0.017 | 0.00ms | 0.030 |
| verify/forge-cam-quality/parts/rptu/rptu4_zigzag_cam.fg | holes | pocket | 0.001 | 0.000 | 0.004 | 0.00ms | 0.006 |
| verify/forge-cam-quality/parts/rptu/rptu5_adaptive_cam.fg | skim | facing | 0.41ms | 0.00ms | 0.028 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu5_adaptive_cam.fg | step | adaptive | 0.799 | 0.014 | 0.012 | 0.00ms | 0.013 |
| verify/forge-cam-quality/parts/rptu/rptu5_adaptive_cam.fg | holes | pocket | 0.002 | 0.00ms | 0.003 | 0.00ms | 0.007 |
| verify/forge-cam-quality/parts/rptu/rptu5_clear_cam.fg | skim | facing | 0.36ms | 0.000 | 0.020 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu5_clear_cam.fg | step | clear | 0.043 | 0.010 | 0.013 | 0.00ms | 0.005 |
| verify/forge-cam-quality/parts/rptu/rptu5_clear_cam.fg | holes | pocket | 0.002 | 0.000 | 0.003 | 0.00ms | 0.012 |
| verify/forge-cam-quality/parts/rptu/rptu5_zigzag_cam.fg | skim | facing | 0.32ms | 0.00ms | 0.020 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu5_zigzag_cam.fg | step | adaptive | 0.821 | 0.011 | 0.011 | 0.00ms | 0.012 |
| verify/forge-cam-quality/parts/rptu/rptu5_zigzag_cam.fg | holes | pocket | 0.002 | 0.000 | 0.003 | 0.00ms | 0.006 |
| verify/forge-cam-quality/parts/rptu/rptu6_adaptive_cam.fg | skim | facing | 0.49ms | 0.00ms | 0.027 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu6_adaptive_cam.fg | step | adaptive | 0.345 | 0.013 | 0.008 | 0.00ms | 0.005 |
| verify/forge-cam-quality/parts/rptu/rptu6_adaptive_cam.fg | holes | pocket | 0.003 | 0.00ms | 0.028 | 0.00ms | 0.017 |
| verify/forge-cam-quality/parts/rptu/rptu6_clear_cam.fg | skim | facing | 0.41ms | 0.00ms | 0.023 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu6_clear_cam.fg | step | clear | 0.013 | 0.007 | 0.010 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu6_clear_cam.fg | holes | pocket | 0.003 | 0.00ms | 0.029 | 0.00ms | 0.020 |
| verify/forge-cam-quality/parts/rptu/rptu6_zigzag_cam.fg | skim | facing | 0.34ms | 0.00ms | 0.017 | 0.00ms | 0.004 |
| verify/forge-cam-quality/parts/rptu/rptu6_zigzag_cam.fg | step | adaptive | 0.542 | 0.009 | 0.008 | 0.00ms | 0.005 |
| verify/forge-cam-quality/parts/rptu/rptu6_zigzag_cam.fg | holes | pocket | 0.003 | 0.000 | 0.030 | 0.00ms | 0.019 |
| verify/forge-cam-quality/parts/thin_wall_cam.fg | skim | facing | 0.001 | 0.00ms | 0.013 | 0.00ms | 0.003 |
| verify/forge-cam-quality/parts/thin_wall_cam.fg | wells | pocket | 0.003 | 0.00ms | 0.054 | 0.00ms | 0.013 |
