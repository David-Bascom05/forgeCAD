# Resume: STEP import at scale (paused by owner priority, 2026-11-23)

Branch claude/repo-review-mbdrlj-scale-step. What was done, what is left, and how to go on. The results and causes in
full are in scale.md; the per-file rows are in scale/CORPUS/SHARD.jsonl.gz.

## Where it stands

- 15 533 real STEP files judged against OCCT 8.0.1 under the truth rule. No crash. Match of certain truths: CADFS 97.4%,
  ABC 96.6%, AutoMate 92.5%, Fusion 360 98.4%, NIST 90.3%.
- Reader judged: claude/repo-review-mbdrlj-import-robust at e27953b, plus `stepread`'s mesh field.
- Two reader fixes made here, after the run and measured apart (scale.md, "Fixed here"): an edge on a closed B-spline at
  its closing point (the top refusal), and the near-flat cone. Gates passed: 31 better, 0 worse on 3020 files; speed
  and bits identical.

## Corpora and licences

| corpus | source | licence | done (shards of 500) |
|---|---|---|---|
| cadfs | huggingface.co/datasets/VladPyatov/CADFS, cadfs/dataset/step.zip (450 306) | CC-BY-4.0 | 0, and 18+45i for i 0..7 |
| abc | the same, cadfs/raw/step_abc.zip (450 307; ABC's STEP) | Onshape ToU 1.g.ii; benchmarking only, nothing redistributed | 9+45i, i 0..6 |
| automate | zenodo.org record 7776208, step.zip (447 442) | CC0-1.0 | 45i, i 0..7 |
| fusion | Fusion 360 Gallery s2.0.1_extended_step.zip (42 912) | Autodesk's non-commercial licence: **testing only, not redistributed, owner approved 2026-11-23**. No file or excerpt in the repository, ever | 4+9i, i 0..6 |
| nist | NIST-PMI-STEP-Files.zip (33) | no restrictions | all |
| MFCAD++ | pure.qub.ac.uk | CC BY | blocked: Cloudflare challenge (not worked around) |
| WHUCAD | gitee fred926/whucad-brep | none stated | blocked: gitee.com fails at the proxy |

formats-corpus (claude/repo-review-mbdrlj-formats-corpus) holds a smaller set: 250 files of ABC chunk 0000 and 44 NIST
files. Leave those out of new counts.

## How to go on

```
tools/occt/setup.sh
cargo build --release -p forge-step --example stepread
cp target/release/examples/stepread /some/bin/stepread       # pin it: rebuilding under a run changes the judge
export SCALE_SCRATCH=/some/scratch STEPREAD=/some/bin/stepread
python3 -I research/step/scale.py index cadfs abc automate fusion nist   # the full index, outside the repo, ~10 s each
python3 -I research/step/scale.py run cadfs 378 900 --stride 45 --jobs 4 # this session's sample, from where it stopped
python3 -I research/step/scale.py run abc 324 900 --stride 45
python3 -I research/step/scale.py run automate 360 895 --stride 45
python3 -I research/step/scale.py run fusion 67 86 --stride 9
python3 -I research/step/scale.py report
```

- **Order and shards.** Ids are sorted as strings and cut into shards of 500; shard K of a corpus is always the same
  files. Another session can take other offsets (for example `run cadfs 1 900 --stride 45`) without overlap.
- **Resuming.** A finished shard is never run again. A shard cut off part way resumes from its partial file in
  `SCALE_SCRATCH`.
- **Queues.** `scale-queue.sh` interleaves the corpora by round. Run it from a copy, not the checkout: sh reads a script
  as it runs. Two queues of 3 jobs each keep 4 cores busy through a shard's slow tail.
- **The fix's effect.** Rerun shards into another directory and compare:
  `SCALE_OUT=/tmp/x STEPREAD=new scale.py run …` then `scale.py compare /tmp/x`.
- **Studying a cause.** `scale.py examples CORPUS CAUSE` lists the smallest files of a cause. `scale.py fetch CORPUS DIR
  ID..` stages files outside the repo. `stepread --faces F` and `occt_faces.py F` give the areas face by face.
  `repro/mk.py` writes the hand-made repros and `repro/check.sh` judges them.

## Causes, routed, and what is unfixed

| cause | files | repro | owner | state |
|---|---|---|---|---|
| closed face with one rim (two edges on one closed spline) | 124 | split_rim_bspline_{t,f} | here | fixed; 105 of these are AutoMate, re-measure the rest |
| near-flat cone | 2 | near_flat_cone | here | fixed |
| FaceWrongSide | 119 | none; ids in scale.md | import-robust | open |
| off past 1e-3 | 98 | none; mostly files whose edges miss their faces within tolerance | import-robust and the judge | open: needs a third witness or a rule for tolerant files |
| mesh and integrals disagree | 66 | spool, extruded_periodic | fix-mesh-asm (meshes right on its branch, with its reader fixes d08ce5b and ba4fba0) | merge those, then re-measure |
| volume cannot be worked out | 41 | none (NIST ftc_06_rd face 89) | import-robust | open |
| OFFSET_SURFACE | 26 | offset_plane | import-robust | open |
| point outside the model box | 24 | far_line_origin | formats-corpus | in hand there |
| whole torus or spline with no edges | 24 | whole_torus | import-robust | open |
| the budget | 15 | | | open |
| OuterNotFirst, no place on the rim for a seam | 4, 5 | | import-robust | open |
| the judge: OCCT's lemon tori | 36 | lemon_ring | truth.md (claude/repo-review-mbdrlj-import) | scale.py counts them uncertain. truth.md should say so: it is not on this branch |

## Head

The head is the commit that adds this file; see `git log -1` on the branch.
