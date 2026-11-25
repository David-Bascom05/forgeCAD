# Opening a big imported model: results

Goal: the Haas VF-2 STEP (59 MB, 37 solid roots: 27 read, 10 refused, about 15k faces), file on
disk to model shown, in at most 1 s cold, every result bit-identical to main's importer; then the
other Haas models. Where the time went before: PROFILE.md.

Machine: the cloud VM, 4 cores (x86-64-v3 build), 15 GB. Native numbers are `forge-step`'s
`examples/stepopen` (`forge_step::open`, its shown meshes, then `Opened::finish`); the app's own
function is timed by `forge-render`'s `examples/model_open` (`machine_model::file_tris`).

## VF-2

| | main | import/speed |
|---|---:|---:|
| native, 4 cores, cold: **model shown** | 11.2 s | **0.99–1.05 s** |
| native, 4 cores, cold: every check done (`Imported` identical) | 11.2 s | 2.1 s |
| app (`machine_model::file_tris`), cold: shown / final | 11.2 s | 1.15 s / 2.4 s |
| app, re-open (content-hash cache) | 11.2 s | **0.18 s** |
| native, 1 thread: shown / all | 11.2 s | 3.4 s / 7.6 s |
| wasm32-wasip1 under Node, 1 thread (the page build's arithmetic): shown / all | 13.9 s | 4.2 s / 9.2 s |
| instructions, 1 thread (callgrind) | 149 G | 74 G (before the two-phase split) |

The shown set is the solids not refused so far: 28 solids, 1.39 M triangles; when the checks finish
(about 1.1 s later) the speck check has refused one more (113 k triangles), and the final 27 solids,
1.27 M triangles, replace them. The single-threaded page build runs the read inline, so it shows the
final model at once (9.2 s); a threaded page build runs the same parallel paths on Web Workers
(`forge_par`'s spawner).

Per phase (wall, 4 cores, shown path): parse 105 ms (245 ms on one thread; pieces on all cores);
topology 70 ms; then the solids, largest first, with the largest (2,249 faces) the critical path:
heal 158 ms, edge tolerances 335 ms, display mesh 238 ms, ending at about 0.92 s.

## The other Haas models

The 40 heaviest of the corpus (by main's read time; main took 20–60 s on each in the corpus runs,
which shared the 4 cores three ways), on c9e27a50, 4 cores, one at a time: wall seconds to shown and
to every check done, then solids shown / solids in the final read / roots refused.

| file | shown s | all s | shown / final / refused |
|---|---:|---:|---|
| COBOT 30-3_2025.STEP  | 0.34 | 2.97 |  28  /  28  /  0 |
| HC10_5_2025.STEP  | 0.68 | 2.19 |  58  /  58  /  0 |
| GM-1XT_4_2025.STEP  | 0.80 | 2.06 |  24  /  24  /  2 |
| HAB3232_6_2026.STEP  | 0.89 | 10.94 |  19  /  19  /  0 |
| cobot_pkg-7_st-20_09-2026.STEP  | 0.89 | 1.93 |  71  /  71  /  4 |
| EC-1600ZT- Pallet Pool -10_2025.STEP  | 1.05 | 2.21 |  140  /  140  /  1 |
| UMC-1000_4_2025.STEP  | 1.11 | 3.36 |  61  /  60  /  4 |
| DT-2_4_2025.STEP  | 1.14 | 2.80 |  48  /  48  /  0 |
| HRP1_STEP_10-2020.STEP  | 1.16 | 2.92 |  166  /  166  /  1 |
| ST-28L- 2_2025.STEP  | 1.16 | 2.37 |  97  /  97  /  1 |
| desktop_lathe_solid_models_08_2024.STEP  | 1.16 | 1.92 |  21  /  20  /  1 |
| UMC-500 + 120 PKT TC_4_2026.STEP  | 1.23 | 3.13 |  65  /  65  /  1 |
| DS-30Y_STEP__12_2022.STEP  | 1.24 | 2.85 |  103  /  102  /  4 |
| ST-30Y-35Y_STEP_2020-03.STEP  | 1.33 | 2.08 |  172  /  172  /  1 |
| GM-1XT-5AX-07-2026.STEP  | 1.35 | 2.93 |  152  /  152  /  1 |
| GM-1-5AX 07-01-2026.STEP  | 1.36 | 2.95 |  152  /  152  /  1 |
| UMC-750_120 PKT_4_2026.STEP  | 1.37 | 3.95 |  67  /  67  /  2 |
| UMC-750_30_50_70PKT_11_2025.STEP  | 1.37 | 4.13 |  69  /  69  /  1 |
| UMC-1250_2021-03 updated.STEP  | 1.39 | 3.16 |  277  /  277  /  3 |
| st-10y_st-15y_solid_models_10_2023.STEP  | 1.41 | 4.50 |  50  /  50  /  0 |
| ds-28y_solid_models_10-2026.STEP  | 1.43 | 4.36 |  60  /  60  /  1 |
| mm_solid_models_01_2024.STEP  | 1.47 | 3.34 |  159  /  159  /  0 |
| VF-4_4SS_STEP__1_2022.step  | 1.56 | 3.51 |  98  /  98  /  6 |
| rpl_modular_umc-500_09-2026.STEP  | 1.59 | 3.41 |  208  /  208  /  2 |
| VF-10_40_STEP_8in-EXT_7_2022.step  | 1.63 | 3.73 |  61  /  60  /  6 |
| HFL-6512T Tube Laser 08-21-2026.STEP  | 1.66 | 3.29 |  83  /  83  /  5 |
| VF-3_SS_STEP__2_2023.STEP  | 1.76 | 3.33 |  59  /  61  /  8 |
| VF-5_40_SS_TR_STEP__3_2022.step  | 1.76 | 5.46 |  99  /  99  /  4 |
| ds-35y_solid_models_10-2026.STEP  | 1.76 | 3.44 |  68  /  68  /  0 |
| ST-28:28Y_10000224585.STEP  | 1.85 | 2.99 |  90  /  91  /  2 |
| ds-40y_solid_model_5_2026.STEP  | 1.85 | 2.87 |  67  /  67  /  0 |
| VF-6_50_TR_EXTENDED Z-AXIS CLEARANCE_STEP__2_2022.step  | 1.93 | 4.14 |  44  /  45  /  5 |
| HRP-2_STEP__1_2022.step  | 2.04 | 6.12 |  216  /  216  /  6 |
| HDC-3-5AX-MLM_STEP__5_8_2024.step  | 2.17 | 4.55 |  37  /  38  /  7 |
| Fanuc 30kg w RPL_10_2025.STEP  | 2.46 | 3.87 |  27  /  27  /  1 |
| VM-6_STEP__1_2022.step  | 2.49 | 5.59 |  43  /  44  /  5 |
| rpl_modular_umc-750_09-2026 1.STEP  | 2.56 | 6.65 |  211  /  211  /  2 |
| 08-1949_STEP_04_02_2024.STEP  | 2.63 | 5.85 |  164  /  164  /  2 |
| Haas 5kg Cobot Package.STEP  | 3.25 | 7.86 |  173  /  173  /  11 |
| st-10_st-15_solid_models_10_2023.STEP  | 3.33 | 6.10 |  184  /  184  /  3 |

Shown at 0.34–3.3 s (median 1.4 s), every check at 1.9–11 s. The heavy ones are heavy in their
display meshes (ds-40y: 4.8 M triangles) and in mass (HAB3232: 22 s on four faces, after display).

## The frozen corpus, every check done

The twenty files of CORPUS.md (frozen before this round; the split is tune / held-out there), on 4
cores of the cloud VM with nothing else running, import/speed-ks. "all s" is the whole read in one
pass, file on disk to every check done and every solid drawn (`forge_step::import_drawn`, what the
app waits for); "app s" is the app's and the CLI's own path (`forge_machine_builder::model_files::read`,
at its 16x budget, each solid's display mesh on all cores). Then the CPU seconds of each check,
summed over the file's solids and threads (`forge-step --features times`, `examples/steptimes`).

| half | file | all s | app s | heal | speck | devs | mass | mesh check |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| tune | VF-2 | 2.28 | 2.19 | 0.8 | 1.8 | 1.3 | 1.5 | 1.2 |
| tune | GM2-5AX lightweight | 3.16 | 3.12 | 0.9 | 1.5 | 1.0 | 3.1 | 1.4 |
| tune | VF-6/50 TR | 4.59 | 4.66 | 2.4 | 3.3 | 2.5 | 3.7 | 1.6 |
| tune | ST-10/ST-15 | 6.90 | 6.94 | 2.5 | 4.5 | 5.9 | 7.7 | 2.2 |
| tune | DS-30Y | 4.75 | 3.78 | 1.8 | 3.3 | 3.3 | 3.3 | 2.9 |
| tune | RPL UMC-750 cell | 13.22 | 13.21 | 5.2 | 11.4 | 9.2 | 15.1 | 3.3 |
| tune | UMC-500 + 20 PP | 4.49 | 4.66 | 1.8 | 2.9 | 2.2 | 3.6 | 2.5 |
| tune | TRT160 | 1.24 | 1.27 | 0.6 | 0.8 | 0.4 | 0.7 | 0.6 |
| tune | HAB3232 | 13.03 | 12.28 | 1.6 | 3.7 | 3.0 | 18.0 | 0.4 |
| tune | ds-40y | 3.24 | 2.96 | 0.8 | 1.7 | 1.1 | 1.7 | 2.2 |
| held-out | VM-6 | 5.69 | 5.73 | 3.2 | 3.7 | 3.1 | 6.4 | 1.9 |
| held-out | VF-5/40 SS TR | 6.52 | 9.83 | 1.9 | 3.6 | 3.0 | 3.5 | 1.8 |
| held-out | ST-28/28Y | 5.74 | 5.92 | 2.2 | 3.2 | 5.0 | 4.0 | 2.6 |
| held-out | ST-20LY/25LY | 5.32 | 3.17 | 1.4 | 4.3 | 2.5 | 3.2 | 6.9 |
| held-out | UMC-1250 | 3.55 | 3.53 | 1.3 | 3.8 | 3.0 | 3.2 | 1.3 |
| held-out | GM-1-5AX | 2.83 | 3.24 | 1.4 | 2.1 | 1.6 | 2.4 | 1.9 |
| held-out | HRT160SS | 0.67 | 0.69 | 0.2 | 0.8 | 0.3 | 0.5 | 0.4 |
| held-out | TRT210 | 0.61 | 0.60 | 0.2 | 0.6 | 0.3 | 0.4 | 0.2 |
| held-out | Haas 5kg cobot | 9.17 | 9.41 | 2.5 | 4.7 | 3.3 | 14.4 | 7.4 |
| held-out | HFL-3015 | 1.49 | 1.48 | 0.5 | 0.5 | 0.3 | 0.7 | 1.8 |

Median and worst: **tune 4.54 s / 13.22 s, held-out 4.44 s / 9.17 s** (the app's path 4.22 / 13.21
and 3.39 / 9.83). Every file opens; none hangs or runs out of memory; two of the twenty are under
1 s. Bits: the whole Haas corpus and the repository's fixtures, 350 files and 12,241 digest lines,
identical to main's read through `import` and through `import_drawn` (and the app's path identical
piece for piece over all 348 STEP/IGES files).

Where the rest of the time is: mass 31% of all the CPU time, speck 20%, edge tolerances 17%, mesh
check 14%, heal 11%, topology 4%, display mesh 2%, parse 1%. Two of the three slowest files are one
check each (HAB3232: 18.0 s of mass; the RPL UMC-750 cell: 15.1 s of mass and 11.4 s of speck).
research/import-speed/CHECKS.md takes the checks one by one: what each guards, and either a cheaper
exact filter or why there is none.

## What changed (each a commit on import/speed)

1. `forge_step::nurbs`: the basis's work arrays sized to the degree (they were zeroed at degree 15,
   2 KB per evaluation, 7% of all instructions); evaluation specialised by degree (to 3 each way) and
   by the derivatives wanted; a Newton step's last point, `Procedural::point`, `deriv1`, `normal`,
   `value_near`, `implicit` and `gradient` without derivatives nobody reads; each basis quotient
   once (the second of function j is the first of j + 1). The same floating-point operations in
   the same order for every number made.
2. Solids read in parallel: every root's topology first, then the rest largest first
   (`forge_math::par::map_eager`: every thread from the start), answers and stats in file order.
3. `forge_geom::budget::map` / `map_until`: a phase's items on all cores under a read's eval budget,
   with the answers one by one would give. Each item runs under what was left less what finished
   items spent; answers are taken in order while the spends summed in order stay below what was
   left (then, one by one, that item started with more than it spent: no budget test was true in
   either run), and the rest run one by one. `map_until` stops where one by one stops (a speck
   found; a face's integrals failing) and charges only what one by one spends. Used by the speck
   check, the edge deviations, `mass_rel` under a budget, `heal::tighten` and the mesher.
4. The check's mesh kept as the display mesh (`Solid::mesh`, `Solid::display_mesh`) where it is the
   same computation (no budget test true, and less spent than a display mesh's own budget,
   `tessellate::MESH` since #58).
5. Part 21 in pieces (files over 4 MB), taken only when every piece ends exactly where the next
   starts, renumbered as one pass numbers; else one pass (which also gives the exact error).
6. `forge_step::open` / `Opened::finish`: phase 1 (heal, edge tolerances, display mesh) for
   showing, phase 2 (speck check, mass, volume check) after, with the serial budget accounting:
   the speck check on healing's budget left, the tolerances' spend charged after it, the display
   mesh taken as the check's mesh where that is the same computation, the void's other way where
   refused. `finish()` is exactly `import_drawn` (= `import` + every solid's display mesh). A
   solid's first drawing is made within what its check could have at most, so solids that will be
   refused as too long to measure are not shown first (a drawing that runs out is not kept; one that
   does not is `MESH`'s, bit for bit).
7. `heal::tighten`: each edge planned from the edges as read, on all cores, applied in edge order.
8. `forge_mesh`: a file body's faces (16 or more) meshed on all cores, each into its own part (an
   `Out` trait over `TriMesh` or a face's part), merged in face order with new vertices renumbered.
9. `forge_render::machine_model`: shows a STEP/IGES model as soon as `open` draws it, finishes the
   checks on a job and swaps in the final triangles (`poll`, `pending`, `generation`; the machine
   view redraws on `generation`); display meshes cached on disk by content hash and
   `FORGE_KERNEL_HASH`.

## Identical results: how checked

`forge-step/examples/stepdigest` hashes, per file and per solid: the body (`Debug`, with lazily
filled `OnceLock` caches taken out), the mass properties, the display mesh, every refusal (entity,
kind, words) and the read's `Stats`; `STEPDIGEST_DRAWN=1` reads through `import_drawn`
(`open().finish()`), its kept drawings compared with main's drawn afresh. Over the whole Haas corpus
(`haas-3d-models`: 146 archives, 317 STEP/IGES files, 6.6 GB) and the 33 STEP, IGES and BREP files
in the repository: 350 files, 11,870 lines. Every step was diffed against main's digest (main
50af9c42 for steps 1-8, then main 6cd93533 after #58 was merged, which itself changed 19 lines):
0 lines differ, through `import` and through `open().finish()`. Tests: forge-step, forge-mesh,
forge-geom's budget (`map_answers_as_one_by_one`), forge-math's par, part21's
`pieces_read_as_one_pass`, forge-render's cache and flattening, forge-model's hidden-inputs review.

Speed gate (tools/speed/gate.sh, exact callgrind, main and branch counted on this VM, band.py):
c0df293b vs main 50af9c42 strict total +0.0009%, worst strict line +0.0152%; 5456dd5b (mesh refactor,
main 6cd93533 merged) vs main 6cd93533 strict total +0.0011%, worst +0.0155% (heatsink edit); both
PASS, bits.txt identical (band-*.txt here). Later commits touch only the import and the app.

## What is left

- On these 4 cores the shown path is CPU-bound (about 0.85 s of 3.4 s of work on 4 cores, plus
  105 ms of parse): VF-2 sits at the 1 s line, not under it with margin. The largest solid's
  edge-tolerance check (`record_deviations`, 256 points per free-form edge onto each face) is the
  next cost; it must stay before the mesh (the mesher reads edge tolerances).
- Bigger Haas models are shown in 1–2 s and checked in 3–13 s (below). HAB3232's check is 22 s of
  mass integrals on four B-spline faces; splitting one face's integral over its loops' pieces would
  bring that down.
- The single-threaded page build does not show early (no second thread to finish on); a page that
  should, needs `Opened` to own its bytes so `finish` can run on a later frame.
