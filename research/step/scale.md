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
