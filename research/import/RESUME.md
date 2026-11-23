# Resuming the real-programs sweep (paused 2026-11-23 by owner priority)

The import of real CAD programs to `.fg` at scale was paused after the first 5k. This is what someone needs to pick
it up. The method and the 5k report are in `scale.md`, the compact results are in `scale-results/`, and the kernel
repros are in `scale-repros/`.

**Head at the pause:** branch `claude/repo-review-mbdrlj-scale-programs`. The runs used merge `11a1436` for CADFS
(its `cadfs.py` is the importer's `1892d2d`) and merge `57c0c8e` for DeepCAD and Fusion. The kernel is the same in
both. The commit that adds this note is the final head.

## Corpora and their terms

| corpus | terms | use |
|---|---|---|
| CADFS (VladPyatov/CADFS on Hugging Face, commit dd76a9b) | CC-BY-4.0; the models are ABC's (Onshape), and their copyright stays with their creators | research; ids and aggregate results are committed |
| DeepCAD (cs.columbia.edu/cg/deepcad/data.tar, `cad_json`), judged against ABC's STEP from CADFS's `raw/step_abc.zip` | MIT; ABC's models under Onshape's terms | research; ids and aggregate results are committed |
| Fusion 360 Gallery reconstruction r1.0.1 | non-commercial research only | **test-only, not redistributed, owner-approved 2026-11-23.** Only verdicts, causes and synthetic repros are committed, never its files, excerpts or measures |

The URLs and hashes are in `scale.md`. No dataset file is in the repository.

## Where the shards stopped

| shard | judged | of | state |
|---|---:|---:|---|
| cadfs 1 of 200 | 1914 | 1914 | done |
| cadfs 2 of 200 | 664 | 1914 | partial |
| deepcad 0 of 200 | 981 | 981 | done |
| deepcad 1 of 200 | 981 | 981 | done |
| deepcad 2 of 200 | 464 | 981 | partial |
| fusion 0 of 10 | 837 | 863 | partial |

The whole run is CADFS shards 1–199 of 200 (shard 0 is the importer's sample, which coverage.md ran), DeepCAD
shards 0–199 of 200, and Fusion shards 0–9 of 10. With 4 cores it ran at about 3.5 parts/s.

## Restarting

The container that ran this is gone, so the data has to be staged again, outside the repo, under `~/data`
(`FORGECAD_DATA` sets another place):

```
B=https://huggingface.co/datasets/VladPyatov/CADFS/resolve/dd76a9b891a6921815fd13c0452afb2d61e5f833/cadfs
mkdir -p ~/data/cadfs ~/data/deepcad/dl ~/data/idx
curl -sL -o ~/data/cadfs/featurescript_fp.zip $B/dataset/featurescript_fp.zip      # 786 MB
curl -sL -o ~/data/cadfs/sketch_raw.zip $B/raw/sketch_raw.zip                       # 2.6 GB
curl -sL -o ~/data/cadfs/unique_models.json $B/misc/unique_models.json
curl -sL -o ~/data/deepcad/dl/data.tar https://www.cs.columbia.edu/cg/deepcad/data.tar
# DeepCAD's cad_json repacked as one zip, ~/data/deepcad/cad_json.zip (members cad_json/NNNN/ID.json)
(cd ~/data/deepcad/dl && tar xf data.tar data/cad_json.tar.gz && mkdir -p ../json && tar xzf data/cad_json.tar.gz -C ../json)
(cd ~/data/deepcad && python3 -I -c "import zipfile,glob,os; z=zipfile.ZipFile('cad_json.zip','w',zipfile.ZIP_DEFLATED); [z.write(p, os.path.relpath(p,'json')) for p in sorted(glob.glob('json/cad_json/*/*.json'))]" && rm -rf json dl/data.tar)
```

The STEPs (CADFS `dataset/step.zip` and `raw/step_abc.zip`, and Fusion's `r1.0.1.zip`) are never downloaded whole.
`httpzip.py` reads them by range and caches each central directory in `~/data/idx`.

Next, build and freeze. Run from a frozen copy, because each part re-reads the importer:

```
cargo build --release -p forge-proof --example judge --example topo
sh tools/occt/setup.sh                      # the OCCT venv for the truth measures
F=~/scale/frozen-$(git rev-parse --short HEAD); mkdir -p $F/bin
cp -r tools/find-parts $F/ && cp target/release/examples/{judge,topo} $F/bin/
```

Seed each shard with the ids already judged, so they are skipped. `scale.py run` skips any id already in a shard's
`results.jsonl`, and ids from other shards do no harm:

```
for c in cadfs deepcad fusion; do zcat research/import/scale-results/$c.jsonl.gz > ~/scale/seed-$c.jsonl; done
# for each partial shard: mkdir -p ~/scale/runs/C-K-of-N && cp ~/scale/seed-C.jsonl ~/scale/runs/C-K-of-N/results.jsonl
# for each done shard, do the same and also: echo done > ~/scale/runs/C-K-of-N/DONE
```

Then run the queues: one line per shard (`corpus k of`), run in turn, `DONE` shards skipped, 4 processes in all:

```
S=$F/find-parts/recreate/scale.py; export FORGECAD_EXE=$F/bin
while read c k n; do R=~/scale/runs/$c-$k-of-$n; [ -f $R/DONE ] && continue
  python3 -I $S run $c --shard $k --of $n --out $R --jobs 2 --tmp ~/scale/tmp; done < QUEUE
```

The queues used were `deepcad 0..199 200` (2 jobs), `cadfs 1..199 200` (2 jobs) and `fusion 0..9 10` (1 job).
To report, run `python3 -I tools/find-parts/recreate/scale.py tally ~/scale/runs/C-*`. To export, run
`scale.py export C ~/scale/runs/C-* --out research/import/scale-results/C.jsonl.gz`; Fusion rows are verdicts only.

If `cadfs.py` changes, refreeze and rerun the CADFS shards from zero, so that one corpus's rows come from one
importer.

## Rules a resumer must keep

- **The truth rule** (`truth.md`, `truth_rule.py`): the volume is OCCT's Gauss–Kronrod volume; the area is taken
  from the measure that agrees with it; a source is "uncertain" where OCCT's measures disagree; and a source's
  copies count as far as the program makes copies (`program_bodies`). A part matches within 1e-4 on both volume
  and area.
- **Partial source (DeepCAD):** DeepCAD kept only sketches and blind extrudes, while ABC's STEP is the whole
  model. Each DeepCAD program is checked against CADFS's FeatureScript of the same ABC id
  (`deepcad.py --whole`). Where the model has another feature kind, an extrude to a bound other than blind or
  symmetric, or a different number of extrudes, a part that builds is "partial source" and is judged neither way.
  That was about 21% of DeepCAD parts, and every whole DeepCAD program that built in the first trial matched.
- **DeepCAD's arcs:** the data keeps 8 decimals of a metre, so every arc whose ends are off its circle by more
  than 1e-10 mm has its centre refit through its ends (`deepcad.py`, `fusion.ON`).
- **DeepCAD's bodies:** a new body stays separate, a join goes into the body made last, and a cut acts on all
  bodies. Joining new bodies instead matched fewer: 279 against 286 of 400.

## Kernel-cause repros, and where they went

All are in `scale-repros/`, in our own numbers, and each fails with its signature on the head.

| repro | cause (parts in the first 5k) | sent to |
|---|---|---|
| `fillet-overflow.fg` | fillet Overflow (41, CADFS) | fillet-fix, session_01Q19LzwXbbjQjNh98T8EXra |
| `chamfer-toolarge.fg` | chamfer TooLarge (16, CADFS) | fillet-fix |
| `classify-fillet-cut.fg` | FAIL Boolean InconsistentClassification, cut (3, CADFS) | fix-curved, session_01Q19LzvdaUxW8JrKx13TvPk |
| `classify-near-tangent.fg` | FAIL Boolean InconsistentClassification, join (8, DeepCAD) | fix-curved |
| `zerothick-groove.fg` | Boolean ZeroThickness, a kernel gap (40 across the corpora, all paths) | fix-curved |

Open work: split the 40 ZeroThickness parts into correct refusals and kernel gaps. A correct refusal is where the
source's own result touches or is non-manifold. One shrank to a cylinder joined to a plate whose hole is the
cylinder's own size, so the two meet along one circle, which is correct. The OCCT oracle (`tools/occt/oracle.py`)
cannot decide this, because its exporter takes only parts that build. Test the reported point against the
source STEP instead: a gap is where the source is one manifold solid there. The next-largest kernel causes have no
repro yet: fillet Corner (10), shell Rebuild(Collapsed) (6), cut Extent Ambiguous (5), shell Collapses (5), shell
SelfIntersecting (5), SketchProfile SelfIntersection (9, DeepCAD) and NotClosed (5, a fail).

## Language gaps and translator causes, ranked (not sent: the master asked for kernel items only)

CADFS untranslated, in the first 2,578 parts: a region the query does not settle 163; imprint edge bounds nothing
67; an entity the sketch no longer has 53; an unequal chamfer 37 (a form `.fg` lacks); a pattern of many bodies 27;
imprint edge source 24; deleteBodies 23; a face the query does not settle 23; no sketch record 19; sketch text beside
other curves 12; sketch text inside a region 8. The importer session (session_01Q19LzynPtKpbkE4EH9HJj8) has these
counts and example ids. DeepCAD's: extrude from a face 48. Fusion's: an elliptical arc without angles 1.

The arc rule is a language gap. `arc(center:, from:, to:)` is refused with SketchNotSolved when its ends are 3e-10 mm
or more off its circle. It was 5 of 739 Fusion parts, and 30 of 400 DeepCAD parts before the centre refit. The
master proposed this rule: the radius from `from`, `to` projected onto the circle, a refusal only beyond a stated
relative tolerance, and a lint with a fix. It needs a Lean extension.

On the translator side, 29 DeepCAD mismatches make more bodies than the source has: `fusion.py` writes one
extrude's touching regions as separate bodies where Onshape fused them.
