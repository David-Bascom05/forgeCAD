# STEP import at scale: tens of thousands of real parts against OCCT

`scale.py` streams whole datasets through the reader a shard at a time (fetch 500 files by HTTP range, judge each,
append a compact row, delete the files), so disk holds one shard. Ids are sorted and cut into fixed shards of 500;
`scale-queue.sh` is this session's sample (every 45th shard of each large corpus, rounds interleaved), and another
session can take other offsets of the same manifest. Each file runs in its own process twice, time and memory
capped: OCCT 8.0.1 under the truth rule (`occt_truth.py`; research/import/truth.md on claude/repo-review-mbdrlj-import:
Gauss–Kronrod volume at 1e-9, uncertain where OCCT's measures disagree) and ours (`kernel/forge-step/examples/stepread.rs`,
which also reports whether the strict mesh, else the tolerant one, meshes each solid). Judged at 1e-3 on volume and
area, as rate.py does.

Results: `scale/CORPUS/SHARD.jsonl.gz`, one row per file (id, size, CRC32, class, cause, OCCT's five measures summed
over all and over distinct solids, our solids' measures, problems, warnings, refusals, mesh). `scale.py report`
judges the rows again under the rule as it stands, so the rule can change without re-reading. No dataset file is in
the repository; `scale/CORPUS.manifest.json` names each source, its licence, its count and a hash of its sorted ids.

## Corpora (checked 2026-11-23)

| corpus | files | licence / terms | used |
|---|---|---|---|
| CADFS `step.zip` (Hugging Face VladPyatov/CADFS): Onshape, rebuilt from FeatureScript | 450 306 | CC-BY-4.0 | yes |
| ABC (Koch et al. 2019) STEP, through CADFS's `raw/step_abc.zip` mirror (same ids as CADFS) | 450 307 | models' rights their creators' under Onshape's terms 1.g.ii; benchmarking only, nothing redistributed | yes |
| AutoMate (Jones et al. 2021, Zenodo 7776208): parts of real Onshape assemblies | 447 442 | CC0-1.0 | yes |
| NIST MBE PMI test models (NIST-PMI-STEP-Files.zip): CATIA, NX, Creo, SolidWorks | 33 | "used without any restrictions" | yes, whole |
| Fusion 360 Gallery, Segmentation s2.0.1 extended STEP set (`s2.0.1_extended_step.zip`) | 42 912 | Fusion 360 Gallery Dataset License (non-commercial research). Used for testing only, not redistributed; owner approved 2026-11-23. No file or excerpt of one is in the repository | yes |
| MFCAD++ (pure.qub.ac.uk, DOI 10.17034/d1fec5a0-…) | 59 665 | CC BY | **blocked**: Cloudflare challenge on pure.qub.ac.uk; not worked around |
| WHUCAD B-rep (gitee fred926/whucad-brep) | ~130k | no licence on the brep repository (main repo MIT) | **blocked**: gitee.com fails at the proxy; no other mirror found |

The Fusion 360 Gallery's other sets (Reconstruction, Assembly) are not used yet. ABC's original host (archive.nyu.edu) takes no range requests and serves 0.6–1.7 GB 7z chunks; the mirror is used
instead. MFCAD's original 15 488 (github hducg/MFCAD, MIT) and the Fusion sets are candidates for later shards.

## Results (paused by owner priority, 2026-11-23)

15 533 files: every 45th shard of CADFS, ABC and AutoMate from offsets 18, 9 and 0 (rounds 0–7, 0–6, 0–7, and CADFS shard 0), every 9th
of Fusion from 4 (rounds 0–6), and NIST whole. Reader: claude/repo-review-mbdrlj-import-robust at e27953b (the fixes
below came after and are measured apart). `scale/report.txt` is the full report.

| corpus | files | match | mismatch | uncertain | refused | crash | timeout | empty | match of certain truths |
|---|---|---|---|---|---|---|---|---|---|
| CADFS (Onshape) | 4500 | 4306 | 81 | 56 | 30 | 0 | 2 | 25 | 97.4% |
| ABC (Onshape, Parasolid-written) | 3500 | 3272 | 68 | 99 | 47 | 0 | 1 | 13 | 96.6% |
| AutoMate (Onshape assembly parts) | 4000 | 3591 | 138 | 118 | 144 | 0 | 8 | 1 | 92.5% |
| Fusion 360 Segmentation | 3500 | 3428 | 57 | 15 | 0 | 0 | 0 | 0 | 98.4% |
| NIST PMI (CATIA, NX, Creo, SolidWorks) | 33 | 28 | 1 | 2 | 2 | 0 | 0 | 0 | 90.3% |

No crash in 15 533. Mesh: of 21 620 solids read, 21 295 meshed strictly, 317 only by the tolerant mesher, 8 not at all.

### Causes, ours, all corpora

| files | cause | repro (repro/) | routed to |
|---|---|---|---|
| 119 | FaceWrongSide | none isolated; 12x more often with a DEGENERATE_TOROIDAL_SURFACE than all files. Ids: CADFS 00000723, 00000736; ABC 00011984, 00012060 | import-robust (its next item) |
| 124 | refused "a closed cylinder / spline / torus / cone face with one rim and no pole" | `split_rim_bspline_{t,f}`: two edges on one closed B-spline make a rim | **fixed here**, below |
| 98 | off: volume or area past 1e-3 (20 past 1e-2) | none: 73% of these files (of the first 52) have tolerant edges (edges off their faces past LINEAR), against 9.6% of matches. We heal edges onto the surfaces, OCCT integrates the curves as written, and the two differ by more than 1e-3 where a file's edges miss its faces by its tolerance. Seen face by face (`stepread --faces`, `occt_faces.py`): AutoMate 0035331a… cylinder strip bounded by splines meeting at a tip, file tolerance 0.28 mm, 0.9% on that face; CADFS 00001563 fillet tori r 0.6 whose spline ends are 9 µm off (1.5% of r), 0.7% per face, OCCT's agreeing with an independent integral | import-robust, to study; partly the judge's |
| 66 | warned: integrals and mesh disagree | `spool` (a huge torus's inner side, 5.7%), `extruded_periodic` (an unclamped periodic B-spline extruded, 9.4%) | fix-mesh-asm: both mesh within tolerance on its branch (reader fixes d08ce5b, ba4fba0 there); import-robust to merge |
| 41 | warned: the volume cannot be worked out (a face's loops draw no region, or its area negative) | none isolated (sphere caps through a pole and corner blends read right: tried). NIST ftc_06_rd face 89 | import-robust |
| 26 | OFFSET_SURFACE not read | `offset_plane` | import-robust |
| 24 | CARTESIAN_POINT outside the model box | `far_line_origin`: a LINE based 4.3 km along itself | formats-corpus (its #5) |
| 24 | whole torus, or a whole spline surface, with no edges | `whole_torus` | import-robust |
| 15 | "its surfaces would take too long to measure" (the budget) | | budget |
| 11 | timeout (60 s) | | |
| 5 | "no place on the rim for a seam" | | import-robust |
| 2 | near-flat CONICAL_SURFACE refused | `near_flat_cone` | **fixed here** |

The judge's own: OCCT's volumes disagree 88, OCCT times out 57, OCCT invalid 64, areas disagree 38, lemon tori 36.

### Fixed here (kernel/forge-step/src/read.rs, tests/at_scale.rs)

- **An edge on a closed B-spline** (clamped, closed in fact, not periodic) that starts or ends at the curve's closing
  point: `edge_range` mapped that point to the curve's start whichever end of the edge it was, so of two edges A to C
  and C to A one ran over the other's half and the rim was counted wrong. The start now takes the first parameter and
  the end the last.
- **A cone within 1e-6 rad of a right angle** (ABC writes flat rings so) reads as the plane square to its axis, its
  normal against the axis as the cone's is; its edges are measured against it as any edge is.

Gates: the 3020 files of eight shards judged again with the fix (`scale/fix-gate.txt`): 31 better, 0 worse
(19 AutoMate files refused before now match, 2 ABC cones match); speed and bits (`GATE_COUNTER=callgrind
tools/speed/gate.sh`): no row past the baseline, bits identical; `cargo test -p forge-step` and the fuzz corpus
(`-p forge-read-fuzz`) pass.

## The judge

The truth rule as truth.md has it, and two more grounds for an uncertain truth:

- OCCT's shape is invalid, or OCCT fails or takes past 120 s;
- the file has a degenerate torus's inner part (DEGENERATE_TOROIDAL_SURFACE, select_outer .F., a lemon). OCCT measures
  such a face as if it were on the whole torus: `repro/lemon_ring.step`, a tube capped by a lemon ring, is 2398.876 by
  its closed form and by ours, and 2683.12 by OCCT's Gauss–Kronrod volume. Rows from before the census told lemons
  from apples count any degenerate torus.

## Rerun

```
tools/occt/setup.sh                                  # OCCT 8.0.1 venv
cargo build --release -p forge-step --example stepread
export SCALE_SCRATCH=/some/scratch                   # holds the full index and one shard
python3 -I research/step/scale.py index cadfs abc automate nist
research/step/scale-queue.sh                         # or: scale.py run CORPUS K [K2] [--stride S]
python3 -I research/step/scale.py report
```
