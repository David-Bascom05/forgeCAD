# Real CAD programs at scale: CADFS, DeepCAD and Fusion 360 Gallery, judged part by part

Each part is a real modelling history translated to `.fg`, built by our kernel, and held to its own source STEP by
the truth rule (truth.md): OCCT's Gauss–Kronrod volume, the area that goes with it, uncertain where OCCT's measures
disagree, the source's exact copies kept as far as the program makes copies. A part matches when both its volume and
its area are within 1e-4 of the source's.

RESULTS

## Data, and its terms

No dataset file is in the repository; this note, the scripts and the compact results are. Everything is read as data
(`python3 -I`), outside the repo, and fetched part by part where it can be: a STEP is read by HTTP range from its
archive, measured and deleted.

| corpus | source | revision / hash | terms | use here |
|---|---|---|---|---|
| CADFS | huggingface.co/datasets/VladPyatov/CADFS: `dataset/featurescript_fp.zip`, `raw/sketch_raw.zip` (downloaded), `dataset/step.zip` (by range), `misc/unique_models.json` | commit dd76a9b891a6921815fd13c0452afb2d61e5f833; sha256 f6c644f3… (fp), 37c91489… (sketch_raw), 42c79eae… (step) | CC-BY-4.0 (the dataset card). The models come from ABC (Onshape), whose models' copyright stays with their creators | research; aggregate results and ids |
| DeepCAD | www.cs.columbia.edu/cg/deepcad/data.tar (`cad_json`); its STEP: ABC's, from CADFS's `raw/step_abc.zip` (by range) | data.tar etag c66f741-5c8dfdc23c228, cad_json.tar.gz sha256 5caea634…; step_abc sha256 3b92c861… | DeepCAD: MIT (code and processed data). ABC: research use, the models' copyright with their creators under Onshape's terms; CADFS redistributes ABC's STEP under CC-BY-4.0 | research; aggregate results and ids |
| Fusion 360 Gallery, reconstruction | fusion-360-gallery-dataset.s3.us-west-2.amazonaws.com/reconstruction/r1.0.1/r1.0.1.zip (by range: each design's .json and its final .step) | r1.0.1, etag e7b0f6dd…-123 | Fusion 360 Gallery Dataset License (in the archive, updated 11/2021): use "only for non-commercial research purposes"; no redistribution of the dataset; no re-association of a model with its creator | non-commercial research only: aggregate results and ids here; no geometry from it is committed; its repros are written in our own numbers |

If ForgeCAD's use stops being non-commercial research, the Fusion runs and their results must go.

## Method

`tools/find-parts/recreate/scale.py`:

- **Ids.** Each corpus's ids are sorted and cut into fixed shards: shard k of S is `ids[k::S]`. CADFS: the 382,609 of
  `unique_models.json` that have a program, a sketch record and a STEP, S = 200, so shard 0 is the sample
  coverage.md ran. DeepCAD: the 196,088 programs that have an ABC STEP, S = 200. Fusion: the 8,625 designs, S = 10.
- **One part.** Its files are fetched into a fresh directory; it is translated (`cadfs.py`, or `deepcad.py` over
  `../fusion.py`), judged (`judge`), and, if it built, its STEP is fetched and measured (`step_truth.measure`, in the
  OCCT interpreter); one line goes to the shard's `results.jsonl`; the directory goes. Each step is its own process
  at nice 19 with 3 GB of address space: translation 300 s of wall clock, the judge 120 s of CPU (run_cadfs.judge),
  the measure 180 s. The `.fg` of a part that did not match is kept, for repros.
- **Resumable.** A part already in `results.jsonl` is skipped, a shard with `DONE` is skipped; two queues of shards
  ran at 2 parts at a time each, 4 processes at most on 4 cores.
- **DeepCAD's program is not always the model.** DeepCAD kept a model's sketches and blind extrudes and dropped the
  rest: a fillet, a cut up to the next face, a revolve, an extrude through all. ABC's STEP is the whole model. So
  each DeepCAD program is checked against CADFS's FeatureScript of the same ABC model (every one of the 196,088 has
  one): where the model has features DeepCAD's program lacks, a part that builds is "partial source", judged
  neither way. On a first 40, every DeepCAD "mismatch" was such a part, and every whole program that built matched.
- **DeepCAD's arcs.** DeepCAD keeps 8 decimals of a metre, so an arc's ends lie up to 1e-5 mm off its circle; the
  sketch solver wants them on it to about 1e-10 (below). Every such arc's centre is refit through its ends. On 400
  parts: 286 match against 271, and 30 SketchNotSolved refusals go. Fusion's arcs keep the data's centre, but its
  joints are now written to 12 decimals like the centres (they were written to 9, which left 9 of 40 parts
  unsolved).
