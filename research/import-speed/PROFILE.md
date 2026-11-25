# Opening a big imported model: where the time goes

Target (David, via the coordinator): the Haas VF-2 STEP, file on disk to model shown, in at most 1 s
cold, every result bit-identical to the current importer.

Stress case: `VF-2_STEP__1_2023.step` from `research/haas-models/vf-2_solid_models_1_2023.zip` on
branch `haas-3d-models`: 59,433,003 bytes, 37 solid roots, 27 read, 10 refused, about 15k faces.

Machine: the cloud VM (4 cores, x86-64, 15 GB). Native release build (`cargo build --release`).

## The open path

The app opens a machine model through `forge_render::machine_model::read`: `forge_step::import`,
then each solid meshed with `tessellate_tolerant` at 5e-4 of its diagonal. `forge_step`'s example
`stepmesh` is that path exactly, and is what is timed here (the `forge-lang` `import(...)` path is
the same read without the mesh).

Baseline (main 50af9c42), wall, single-threaded (the path has no threads today):

| phase (inside `import` unless marked) | wall s | share |
|---|---:|---:|
| part 21 parse (lex + entity table) | 0.25 | 2% |
| build topology (`Build::shell`: entities to the proto body) | 0.49 | 4% |
| `measure_gaps` (stats: how far the file misses itself) | 0.16 | 1% |
| heal: `split_closed_splines`, `add_seams`, `tighten`, `split_fans`, `into_body` | 0.98 | 9% |
| `laid_out_in_a_speck` (refusal check: uv loops at 8x segments, every face) | 3.25 | 29% |
| `record_deviations` (each edge's tolerance, 16 or 256 points onto its faces) | 1.78 | 16% |
| `measures_agree`: `mass_rel` (integrals to 1e-6) | 1.51 | 13% |
| `measures_agree`: mesh at 5e-4 diagonal, volume and area against the integrals | 0.88 | 8% |
| rest of `import` (roots, units, assembly, drops) | 0.90 | 8% |
| **import total** | **10.18** | |
| display mesh (app side, the same mesh as the check's, made again) | 0.86 | 8% |
| **file to triangles** | **11.2** | |

(HOSTED-DESIGN measured 13.8 s wall, 170 MB, through `forgecad` eval + measure on its machine.)

Per solid (import only): the largest is 2.49 s (solid #441390, 2249 faces), then 1.90 s (561
faces), 1.01 s (1258 faces), 0.97 s (a refused one). Every other is under 0.9 s.

## What the instructions are (callgrind, 149.3 G Ir in all)

| function (inclusive) | Ir | share |
|---|---:|---:|
| `forge_step::nurbs::Nurbs::eval_all` (B-spline surface point + 2 derivatives) | 112.0 G | 75.0% |
| of which `basis` (Cox–de Boor + derivatives) | 64.4 G | 43.2% |
| `Nurbs::newton` (projection) | 86.3 G | 57.8% |
| `laid_out_in_a_speck` → `uv_loop` | 52.2 G | 35.0% |
| `record_deviations` | 27.5 G | 18.4% |
| `mass::face_integrals` | 24.9 G | 16.7% |
| `tessellate_with` (both meshes) | 16.2 G | 10.8% |
| `heal::Proto::tighten` | 13.4 G | 9.0% |
| `memset` (self) | 10.3 G | 6.9% |

Self cost: `eval_all` 22.4%, `basis` 18.6%, `basis` closure (derivative ratios) 14.1%, `memset`
6.9%. The memset is the basis's work arrays zeroed at `MAX_DEGREE` = 15 (a 16 x 16 triangle, 2 KB)
on every evaluation, for surfaces of degree 3.

## So

1. Surface evaluation is three quarters of everything. Every phase that is slow is slow because it
   projects points onto the file's B-spline surfaces. Make `Nurbs::eval_all` and `basis` cheap,
   with the same floating-point operations in the same order (so the same bits): arrays sized to
   the degree, no repeated derivative ratios, a point-only evaluation where only the point is used.
2. The display mesh is the check's mesh: same body, same chord, same `Quality`. Reuse it when the
   check's budget never ran out (then no budget test inside the mesher was true in either run, and
   the two runs are the same computation).
3. Solids are independent (each has its own budget scope): read them on all cores, results in
   file order.
4. Inside a solid, the per-face and per-edge checks are independent: run them on all cores under an
   optimistic budget (each worker gets what is left; if the spends together reach it, redo the
   phase one by one, so the answer is the serial one).
5. Re-open: a content-hash cache of the triangles.

Measured results of each step are in RESULTS.md.
