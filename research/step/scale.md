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
| Fusion 360 Gallery (Reconstruction r1.0.1, Segmentation s2.0.1, extended STEP set, Assembly a1.0.0) | ~200k | Autodesk licence: non-commercial research only; for-profit employers bound | **held**: asked the master whether our use qualifies |
| MFCAD++ (pure.qub.ac.uk, DOI 10.17034/d1fec5a0-…) | 59 665 | CC BY | **blocked**: Cloudflare challenge on pure.qub.ac.uk; not worked around |
| WHUCAD B-rep (gitee fred926/whucad-brep) | ~130k | no licence on the brep repository (main repo MIT) | **blocked**: gitee.com fails at the proxy; no other mirror found |

ABC's original host (archive.nyu.edu) takes no range requests and serves 0.6–1.7 GB 7z chunks; the mirror is used
instead. MFCAD's original 15 488 (github hducg/MFCAD, MIT) and the Fusion sets are candidates for later shards.

## Rerun

```
tools/occt/setup.sh                                  # OCCT 8.0.1 venv
cargo build --release -p forge-step --example stepread
export SCALE_SCRATCH=/some/scratch                   # holds the full index and one shard
python3 -I research/step/scale.py index cadfs abc automate nist
research/step/scale-queue.sh                         # or: scale.py run CORPUS K [K2] [--stride S]
python3 -I research/step/scale.py report
```
