# Timing against a real machine: the RPTU/Spinner U5-620 programs

The only machine-truth check of forge-machine's timing model so far: the three SINUMERIK 840D
programs of the RPTU/Spinner dataset (ElMoe @b8003660, CC BY 4.0; copies in
kernel/forge-machine/tests/data/spinner-u5) against their own 1 Hz log (Zenodo 15735480,
data_v1_0_1.csv, CC BY 4.0; fetched 2026-11-24, 78,734,331 bytes, sha256 d7227bad…52be580).
Simulated by kernel/forge-machine/tests/timing_u5.rs; measured by the scripts here
(`python3 -I research/cam/timing/rptu-u5-620/logtimes4.py data_v1_0_1.csv` from the repository root).

## The table (seconds per execution of the program)

| | Keychain | Bottle opener | Achskreuz (3+2) |
|---|---|---|---|
| **logged running time**, median [range] | 73 [66, 75] | 236 [228, 238] | 1122 [1120, 1137] |
| executions (override 100 % in ≥ 95 % of samples) | 7 | 8 | 12 |
| of which tool-change subprograms (line −2) | 18 | 23 | 29 |
| **logged, without tool changes** | **55** [55, 58] | **213** [211, 215] | **1094** [1091, 1108] |
| t_min (lower bound) | 28.0 | 138.8 | 783.7 |
| t_stop, A = 1000 / 2000 / 5000 mm/s² | 47.5 / 39.1 / 33.2 | 266.1 / 214.2 / 174.1 | 1061.0 / 951.3 / 865.1 |
| t_la (15 blocks), A = 1000 / 2000 / 5000 | 37.4 / 33.0 / 30.2 | 168.5 / 153.6 / 144.7 | 1022.8 / 929.8 / 855.2 |
| t_la (100 blocks), A = 1000 / 2000 / 5000 | 37.4 / 33.0 / 30.2 | 168.0 / 153.3 / 144.7 | 1022.8 / 929.8 / 855.2 |
| logged ÷ t_min | 1.96 | 1.53 | 1.40 |
| logged ÷ t_la (A 2000, 15 blocks) | 1.67 | 1.39 | 1.18 |
| logged ÷ t_stop (A 2000) | 1.41 | 0.99 | 1.15 |

(sim times include no tool-change time; "logged, without tool changes" is the comparable row)

### Where the logged time goes, and the model's share of it

Each running second (single executions at override 100 %, 4 per program; `speedclass.py`) by path
speed against the programmed feed of its line:

| | Keychain | Bottle opener | Achskreuz |
|---|---|---|---|
| rapids (G0 / SUPA), logged | 12.2 | 14.3 | 244.2 |
| rapids, t_min / t_la (A 2000) | 6.1 / 10.3 | 8.9 / 15.0 | 102.4 / 247.0 |
| cutting, logged: at feed (≥ 90 % F) + reduced (10–90 %) + near stop (< 10 %) | 12.5 + 23.8 + 4.0 = 40.3 | 101.8 + 72.8 + 20.5 = 195.1 | 560.2 + 273.8 + 9.8 = 843.8 |
| cutting (feeds and arcs), t_min / t_la (A 2000) | 21.9 / 22.7 | 129.9 / 138.6 | 681.3 / 682.8 |
| cutting: logged ÷ t_la | 1.78 | 1.41 | 1.24 |
| tool changes, logged (per change) | 21.0 (≈ 6–7) | 24.5 (≈ 6) | 34.0 (≈ 7) |

## What it says

- t_min holds as a lower bound on every program (logged ÷ t_min 1.40–1.96).
- Rapids are modelled well at A = 2000 mm/s²: t_la within 0.85–1.07 of the logged rapid time.
  The log confirms the record's rapid: per-axis G0 speed peaks at 47,999 mm/min (p99), the
  dealers' 48 m/min.
- Cutting is underestimated by t_la: the control spends 24–32 % of the time between 10 % and
  90 % of the programmed feed, which grbl's junction-deviation planner at δ 0.01 mm does not
  reproduce (it adds 1–7 % over t_min for cutting). The shortfall grows with short segments and
  arcs (Keychain's curve drives 1.78, Achskreuz's mostly long passes 1.24). Programmed feeds
  are reached exactly where moves are long (F263 … F3820: p90 of the logged path speed = F).
  So for toolpaths of many short moves (adaptive clearing) t_la is optimistic: a comparison by
  t_la favours them.
- t_stop at A = 2000 lands within 0.99–1.41 of the logged time: exact stops overestimate the
  rapids but the over- and under-estimates partly cancel. It is not evidence that the control
  stops at every block (CYCLE832 runs continuous-path mode).

## What each column assumes

- Machine: spinner-u5-620's kinematics as fitted from this log (machines/spinner-u5-620
  @3576cc70; a test-local copy, tests/data/spinner-u5/u5-620-kinematics.toml): B and C senses,
  C axis, and the B axis 49.3 mm above the table taking B and C to intersect (estimates fitted
  from logs, residual ≤ 0.33 mm). Linear rapid 48 m/min per axis (dealer listing; the log agrees);
  no max cutting feed (UNKNOWN: = rapid); rotary speeds UNKNOWN (high).
- Accelerations: the record has none (UNKNOWN). t_stop and t_la are given at 1000, 2000 and
  5000 mm/s² on X Y Z (3600 deg/s² on B C), the same for every program; A is not fitted.
- t_la: grbl v1.1h's planner (lookahead.rs): junction deviation 0.01 mm, 15 or 100 blocks, arcs
  cut at 0.002 mm. No jerk limit. 100 blocks change nothing here: the moves are long against
  what a window of 15 can stop within at these accelerations.
- Not modelled: CYCLE832 (the programs set tolerance 0.02 mm, continuous-path mode with
  compressor and smoothing: the machine's settings, PGZ 3.17), spindle run-up (each operation's
  first block starts the spindle: 10,000-24,000 rpm), the tool change (record change time
  UNKNOWN: 0 here; the log measures about 6-7 s per change, including the tool-change
  subprogram's moves), dwells (none programmed).
- Setups: G54 per program from the log (Keychain and Bottle opener from MCS minus the program's
  words on one line; Achskreuz from the BCS, tests/traori.rs). Gauge lengths from the fit
  (137.0, 250.1 mm), else 120-123.5 mm (unknown): they shift approach lengths, not cutting.

## The log, as read

- Each logged session runs the program twice back to back (the line number returns to ~19 after
  the end); the times above are per execution. Sessions are split there.
- Running time: the sum of intervals between consecutive samples with status 3 (running) in AUTO,
  each at most 2 s (longer gaps are pauses: one of 15-100 s near each session's start);
  executions with feed override at 100 % in at least 95 % of samples.
- Tool-change time: samples at program line −2 (the tool-change subprogram).
- The per-axis feed columns do not follow the per-axis position changes (during the arc at
  Achskreuz line 1081 the X position moves while the X feed column reads 0 and Z's does not);
  only the path speed (the vector's length, which position changes confirm on sampled lines) is
  used.
- Zenodo's file differs from the sha256 in tools/bench/manifest.tsv (af0d15b5…): reported to
  TOOLPATH-QUALITY.
