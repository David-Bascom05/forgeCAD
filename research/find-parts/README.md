# Finding parts that will not build

Random histories make unrealistic parts and miss what people really model. This is what finds more,
measured by distinct failures per CPU-hour, and the number that matters most: the percent of real
human parts that build. Tools: `tools/find-parts/`. Nothing here is used by shipped code except the
CI tier (`verify/forge-proof/tests/find_parts.rs`, `tools/find-parts/ci-corpus.txt`).

## The real-parts build rate

```
real-parts build rate: 91.3% (7875 of 8625) on main d33b84aa   the Fusion 360 Gallery reconstruction set, all of it
                       94.7% (8169 of 8625) on reliab-2 3b1d1c47, the first day's fixes
OpenCascade yardstick: 99.0% of 1053 translated parts (every 8th) with the same translation; ours 94.9% on main, 97.8% on reliab-2
```
Built means: regenerates, every stage valid, meshes closed, exact volume within 0.1% of what Fusion recorded (1% for parts whose
ellipses or splines became polylines). A refusal counts against us, because Fusion built the part, with one exception: a final
body truly pinched in one solid cannot exist, so refusing it is right (`correct-refusals.txt`, 2 designs). Rerunning the command
below gives the number; it moves only with the kernel, since the dataset, translation and tolerance are fixed.

Of the 750 not built on main: 205 did not translate (129 tapered extrudes, which the part format lacks; 49 profiles that do not
close; 20 elliptical arcs without angles; 7 others), 545 did. 33 of those are wrong volumes, nearly all translation: OCCT agreed
with our kernel on 31 of the 35 on reliab-2, mostly multi-body joins (a Join merges the bodies it touches; the translation adds to
the last one). The fixed translation also costs OCCT 0.8%, which is roughly the ceiling this importer allows.

DeepCAD, every 20th design: 94.7% build (10184 of 10754), with no volume to check against, so this says only that they build.
HistCAD's 10 open samples (5 Industrial: revolves, fillets): 9 build on main. All 10 build in OCCT. One failed with `inconsistent
classification`, and `industrial_01027399` lost a cut silently (15.33 against OCCT's 14.00); reliab-2 fixes both. 10 parts is a
smoke test, not a rate.


## Which methods find the most

CPU time of the judged parts only, same binary (main d33b84aa), parts run at nice 15 under heavy load. A failure counts if it
is a kernel error, an invalid body, a hang or a crash. A root is the failure with the feature kind dropped. Generator mistakes and
volumes off the dataset's truth are not counted. Small runs that used up all their parts (`std`, `augment`, `noise`) have inflated rates,
so read roots and the first CPU minute together.

| method | parts | CPU h | failing parts | signatures | roots | roots/CPU h | roots in first 60 CPU-s | roots also in real parts | distinct refusals |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| fuzz (baseline) | 1184 | 0.34 | 346 | 92 | 30 | 90 | 12 | 9 | 0 |
| swarm | 528 | 0.19 | 171 | 70 | 28 | 151 | 11 | 9 | 0 |
| DeepCAD sample | 10214 | 0.01 | 187 | 14 | 12 | 1046 | 12 | 8 | 6 |
| Fusion real parts | 7909 | 0.37 | 264 | 16 | 14 | 38 | 9 | 14 | 6 |
| real parts + finishing (augment) | 2091 | 0.08 | 263 | 24 | 16 | 189 | 9 | 12 | 66 |
| real parts mutated | 2167 | 0.09 | 95 | 15 | 13 | 145 | 10 | 10 | 33 |
| real parts + noise (on reliab-2) | 14612 | 0.23 | 320 | 16 | 14 | 60 | 6 | 11 | 7 |
| surface matrix | 15426 | 0.07 | 418 | 15 | 12 | 172 | 9 | 9 | 17 |
| feature pairs (enum) | 21645 | 0.11 | 1845 | 26 | 13 | 116 | 9 | 6 | 89 |
| bench + standard parts mutated | 1643 | 0.34 | 43 | 19 | 14 | 42 | 7 | 6 | 212 |
| bench sweeps | 1376 | 0.17 | 25 | 16 | 8 | 48 | 3 | 4 | 78 |
| coverage-guided (instrumented binary) | 701 | 0.34 | 42 | 6 | 5 | 15 | 1 | 4 | 115 |
| standard parts | 66 | 0.00 | 3 | 1 | 1 | - | 1 | 0 | 2 |
| bench + standard, finishing | 106 | 0.00 | 2 | 1 | 1 | - | 1 | 0 | 23 |
| bench + standard, noise | 135 | 0.00 | 21 | 7 | 5 | - | 5 | 3 | 7 |


What the table shows, and what it cannot:

- **Real parts find what matters, not the most kinds.** Random fuzz still reaches the most failure kinds, mostly n-ary regen
  disagreements on features the real data here does not use. But it says nothing about frequency, and it cannot see a wrong answer. Real
  parts ranked the failures (one case, coaxial equal-radius cylinders butted, was 244 of 545) and fixing that ranking moved the rate
  from 91.3% to 94.7% in a day. Real parts are also cheap: DeepCAD reached 12 roots in 28 CPU-seconds.
- **Only a truth finds silent wrong answers.** Three parts lost a cut with no error, and every method without a truth reported them ok.
  Fusion's volume or OCCT caught them. The cause was float noise of about 3e-7 mm, which is why `noise` exists.
- **Noise is the cheapest fragility test.** On reliab-2, 3e-7 mm of noise stopped 309 of 8188 built real parts (3.8%), and no volume
  moved by more than 0.01%. That threshold cannot see a lost cut smaller than that.
- **Swarm beats plain fuzz** at no cost: nearly the same roots in 56% of the CPU. It should replace the default, with the usual caveat
  that this is one seed range.
- **Feature pairs give the smallest repros** (2-3 lines) and fill gaps: `enum-cells.md`, `matrix-cells.md`. 124 of 1681 pair cells
  never built, so nothing past those is exercised yet.
- **Coverage guidance lost here**: 5 roots in 0.34 h. The mutations mostly reach new refusal branches (115 distinct refusals), the
  instrumented binary is slower, and a new error path is rarely a bug. It might do better from a corpus of real parts; not tried.
- **Sweeps find what edits break, not kernel bugs**: 78 distinct refusals, many a selector left with nothing after a dimension moved
  (81 of 490 refused parts). Whether each is right needs a person or OCCT. `text` refusals there are the harness: no font beside the file.
- Signatures are coarse. One root can hide several bugs, so counts are a lower bound, and one seed range does not give error bars.

Failures go to the reliability agent as shrunk repros in `forgecad-gate/repros/` (`find-*`, `find-synth/`, `find-noise/`). The CI tier
is 39 parts picked by branch coverage per CPU-second, covering 14435 of the 15275 counters the 2000-part sample reached, about 6 s in the test
profile: no part may do worse than recorded.

Next: HistCAD's full set if it becomes reachable, then CADFS, for real fillets, shells and sweeps. All numbers above are sketch and extrude
only, apart from HistCAD's samples, so the rate says nothing yet about finishing features.


## How each method works

Every method judges a part in its own process (`verify/forge-proof/examples/judge`): RAM capped, wall
clock capped, so a hang or a stack overflow costs one case. A part ends ok, refused (a clear error),
fail (a kernel bug by `history::classify`), wrong (built, disagrees with a known truth), timeout, crash
or oom. A failure's signature is its verdict, the feature kind and its message with numbers and names
blanked, plus the panic's source line; its root drops the feature kind. Each signature's first part is
shrunk to a minimal `.fc` by dropping lines and selectors and plainer numbers (`fp.shrink`).

- `fuzz`: `history::run`, the baseline.
- `swarm`: the same, each history drawing from a random third of the feature kinds (Groce et al., "Swarm
  testing", ISSTA 2012).
- `real`: Fusion 360 Gallery and DeepCAD timelines translated to `.fc` (`fusion.py`), judged, and checked
  against the volume Fusion recorded (`realparts.py`).
- `sweep`: each dimension of the bench parts across its range and onto values that make faces meet: equal to
  another dimension, their sum or difference, a hair off those (`gens.sweep`).
- `enum`: base + F1 + F2 for every pair of 40 feature forms on 19 bases (`gens.enumerate_pairs`), so every feature
  kind after every feature kind on each face and edge kind. `cells.py` prints the matrix.
- `matrix`: tool shape x surface x placement (tangent, coaxial, coincident, through a vertex, near each)
  x op x depth x a finishing step after (`gens.matrix`), extending the `adversarial` families to blend, chamfer and
  draft surfaces and to what comes next.
- `std`: flanges (ASME B16.5), bolts and nuts (ISO 4017), shafts with keyways and ring grooves (DIN 6885, 471),
  pillow blocks (6200 series), brackets, housings, enclosures, castings, involute gears, pulleys, manifolds.
- `mutate`: real parts with one or two edits: a feature moved or dropped, an op changed, a selector swapped, a dimension
  moved, a feature from another part spliced in, a finishing step added.
- `cover`: the same mutations steered by branch coverage. The judge is built with `-C instrument-coverage`; a run's raw
  profile is read directly, a part reaching a new branch joins the corpus, and parts holding rare branches get mutated
  most (FairFuzz, Lemieux and Sen, ASE 2018).
- `histcad.py`: HistCAD's samples, whose fillets pick edges by 3D points. `occt_replay.py`: the translated parts through OCCT alone.
- `augment`: a real part, then the finishing a person adds next, sized to it.
- `noise`: a built part with its coordinates moved 3e-7 mm: it must build, to about the same volume. Real data carries this
  much float noise, and two silent wrong answers came from it. No truth needed.
- OCCT differential: `occt_diff.sh` replays parts through `tools/occt/oracle.py`; `occt_tally.py` lists what one kernel
  builds and the other refuses, and every disagreement.

## Rerun

```
fbuild cargo build --release -p forge-proof --example judge
D=<data dir, outside the repo>
python3 -I tools/find-parts/fusion.py $D/fusion/json/*.json --out RUN/parts      # --mm 1000 for DeepCAD (metres)
fbuild python3 tools/find-parts/realparts.py RUN/parts --out RUN/real            # prints the rate, then each signature
fbuild python3 tools/find-parts/realparts.py RUN/parts --out RUN/real --shrink-only
fbuild python3 tools/find-parts/find.py METHOD --out RUN/METHOD --cpu 1200       # sweep enum matrix std mutate fuzz swarm augment noise
fbuild env RUSTFLAGS="-C instrument-coverage -C target-cpu=x86-64-v3" CARGO_TARGET_DIR=$PWD/.scratch/target-cov \
    cargo build --release -p forge-proof --example judge
fbuild python3 tools/find-parts/cover.py --exe .scratch/target-cov/release/examples/judge --out RUN/cover --cpu 1200
python3 tools/find-parts/report.py RUN/fuzz RUN/sweep .. --real RUN/real         # the table
python3 tools/find-parts/cells.py enum RUN/enum                                  # the matrices
```
The CI tier is regenerated by `ci_pick.py`: greedy set cover of branches per CPU-second over a sample of the generators.

## Data and licences

No dataset file enters this repo. Downloads live in `~/.local/share/forgecad-find-parts/`, and importers read them with
`python3 -I`, as data only.

- Fusion 360 Gallery reconstruction, r1.0.1 (https://github.com/AutodeskAILab/Fusion360GalleryDataset), 8625 designs: its
  licence allows use "only for non-commercial research purposes" and limits redistribution. Aggregate results are here.
  Repros shrunk from it stay in `forgecad-gate/repros/`, marked; a test restates the geometry in its own numbers.
- DeepCAD (https://github.com/ChrisWu1997/DeepCAD), about 178k designs from Onshape via ABC, listed as MIT; ABC says "the
  copyright of the CAD models is owned by their creators" under Onshape's terms. Treated the same way.

## More datasets: the survey

Sketch and extrude data does not exercise fillets, chamfers, shells, revolves, sweeps or patterns. Surveyed 2026-10-31 from each
dataset's own pages; effort is an estimate, licences are as stated there and not legal advice, and items marked unverified were
not confirmed. Ranked by distinct real failures expected per unit of importer effort.

| rank | dataset | real? | size | licence | history, kinds | edge references | check against | access | effort |
|---|---|---|---|---|---|---|---|---|---|
| 1 | HistCAD Industrial | human, company parts | 18,352 | code MIT; Industrial keeps its source terms (unverified) | yes: revolve, helix, fillet, chamfer | 3D points | native, STEP (older paper version) | 5 samples open; full data unconfirmed | days |
| 2 | CADFS | human Onshape; text is LLM | about 450k; programs 0.8 GB, STEP 7.4 GB | card CC-BY-4.0, code MIT; ABC leaves copyright with creators | yes: fillet, chamfer, shell, hole, revolve, sweep, loft, pattern, mirror; no draft | Onshape symbolic queries | STEP | open | 2-4 weeks |
| 3 | FreeCAD-library | human | 3,231 .FCStd, about 5 GB | CC-BY 3.0, attribute authors | feature tree, PartDesign share unverified | named elements, format unread | stored .brep (likely per step) | open | 1-2 weeks |
| 4 | FllumaOne | synthetic templates | 100k | unverified | yes: fillet, shell, sweep, loft, patterns | unverified | STEP | "for review" link | 1 week |
| 5 | WHUCAD | 4,244 manual parts are new, the rest augmented DeepCAD | 146k | MIT, sourced from ABC | yes: 18 types, the only one with draft | CATIA names, quantised to 256 levels | mesh, B-rep | open | 3-6 weeks |
| 6 | NIST PMI, AutoMate, Fusion segmentation and assembly | human | 11 cases and others; 34.9 GB; 35,680 and 154,468 | unrestricted; CC0; non-commercial research | none usable | - | STEP; Fusion assembly has volumes | open | small, import and validity only |
| 7 | bd_warehouse, cq_warehouse | human code | limited families | Apache-2.0 | code, needs Python tracing | code selectors | run on OCCT | open | 2+ weeks |
| 8 | CADParser, SldprtNet | human | 40k, 242k | not found, unstated | yes on paper | 3D point; unverified | unverified | no data found | blocked |

Not useful for this gap: ABC `feat` (not a history; `ofs` not inspected, and CADFS's programs from the same source show symbolic
queries), Fusion assembly (no timelines) and segmentation (labels only), MFCAD and MFCAD++ (synthetic, labels), Text2CAD (DeepCAD
operations), Omni-CAD (fillets and chamfers removed), CAD-Recode (synthetic), GenCAD-Code, CADmium, Thingi10K. Onshape's terms forbid
bulk collection through its API. No dataset beyond Fusion, DeepCAD and the HistCAD samples has been downloaded: using them
awaits David's answer on the licences.

Sources: https://deep-geometry.github.io/abc-dataset/ ; https://arxiv.org/abs/2605.01925 , https://huggingface.co/datasets/VladPyatov/CADFS ,
https://github.com/VladPyatov/CADFS ; https://arxiv.org/html/2602.19171 , https://anonymous.4open.science/r/HistCAD-68C2 ;
https://github.com/FreeCAD/FreeCAD-library ; https://www.ijcai.org/proceedings/2023/0200.pdf ; https://arxiv.org/html/2603.13098v1 ;
https://github.com/fazhihe/WHUCAD ; https://arxiv.org/pdf/2606.17696 , https://github.com/Cad-Kernel/FllumaOne-100K ;
https://github.com/AutodeskAILab/Fusion360GalleryDataset ; https://zenodo.org/record/7776208 ; https://github.com/hducg/MFCAD ;
https://www.nist.gov/ctl/smart-connected-systems-division/smart-connected-manufacturing-systems-group/mbe-pmi-0 ;
https://arxiv.org/html/2411.04954v3 ; https://github.com/SadilKhan/Text2CAD ; https://github.com/chandar-lab/CADmium ;
https://arxiv.org/abs/2412.14042 ; https://arxiv.org/abs/2505.14646 ; https://github.com/gumyr/bd_warehouse ;
https://github.com/gumyr/cq_warehouse ; https://www.onshape.com/en/legal/terms-of-use
