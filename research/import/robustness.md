# Import: robustness, STEP quality, new formats (2026-11-23)

Three questions: does every reader of a file end in a value or a typed error, in bounded time and
memory, on any input; how many real STEP files import as valid bodies of the right volume, and
why the rest do not; and which other formats are worth reading.

Branch claude/repo-review-mbdrlj-import-robust. Coverage of features (FCStd, WHUCAD, CADFS histories as
.fg) is another branch's (research/import/coverage.md there) and is not touched here.

## 1. Robustness

### The fuzzer: verify/forge-read-fuzz

`read-fuzz` feeds every reader mutants of real files and keeps what goes wrong.

| target | reader | seeds in the repository |
|---|---|---|
| step | `forge_step::import`, then `check` and the tolerant mesh (what `fg check` and the viewer do next) | examples/**/*.step |
| iges | the same, sniffing IGES | forge-step's test file; seeds/cam-*.igs (B-rep and trimmed-surface, written by OCCT from an example) |
| brep | the same, sniffing OpenCascade `.brep` | seeds/*.brep (written by OCCT from examples) |
| fcstd | `forge_import::fcstd::translate` (zip, XML, `.brp`, the part built) | three documents made by the harness, stored and deflated |
| dwg | `forge_dwg::file::read` | kernel/forge-dwg/tests/data |
| fg | `forge_lang::run`, stopping before the kernel (`fg check`) | every .fg in the repository |
| fg-build | the same, built (`fg build`); not in `all`, it is the kernel's fuzzing | the same |
| ttf, shx | forge-font and forge-dwg's font readers | the repository's fonts (no SHX: none in the repository) |
| json, material | MCP requests and edit ops; material libraries | made; materials.toml |
| inflate, git | forge-git's zlib, trees, commits, tags, index, pack deltas | made |
| stl, obj, 3mf | the new mesh readers (§3) | made |

**Mutations** are a function of (seed, case number), so `--seed S` and a case number make any
input again. Half are byte edits (flips, inserts, deletes, truncation, a run repeated up to 1000
times, a little-endian count or offset made 0, 1, 2^31 or 2^32 − 1, a splice of another seed).
Half know the format:

- **STEP:** a number made extreme (0, −1, ±1e308, 1e400, NaN, 2^31, 2^63), a reference pointed at
  itself, at #0 or at another kind, an instance dropped or repeated, a list nested 10 000 deep or
  10 000 long, a keyword swapped, an argument dropped.
- **IGES:** an 8-column directory field or a parameter made extreme, in place, so the reader goes
  on past the card.
- **.fg:** counts made huge, nesting 100 000 deep (brackets, minus, blocks), lines moved, names
  swapped, strings left open.
- **XML (FCStd, 3MF):** an element repeated 1000 times, entities, a tag cut.
- **Zip:** unpacked, one member made wrong, packed again stored or deflated; members dropped,
  renamed (`../../x.brp`) or made a 16 MB bomb.

**Supervision.** A worker process runs a stride of the cases and says before each which it starts.
The supervisor applies the per-case timeout. Through `ulimit -v` it caps memory. A case that hangs,
aborts (stack overflow, failed allocation) or panics is kept as a `.bin` and `.txt` in `--out`, and
the worker restarts past it. Peak memory per case comes from VmHWM, reset by `clear_refs`. A case
past `--big-mb` and 16 times its own size is a finding. Findings are grouped by kind and panic site.
The run exits 1 if there are any. It uses no third-party crate and no unsafe code; the `fuzz`
profile is release with overflow checks and debug assertions on.

```
cargo build --profile fuzz -p forge-read-fuzz
target/fuzz/read-fuzz run all --seconds 1200 --jobs 4 --seed $RANDOM
target/fuzz/read-fuzz run step,iges --seconds 3600 --jobs 4 --seeds DIR_OF_STEP_FILES   # long runs: a dataset as seeds
target/fuzz/read-fuzz replay step target/read-fuzz/step-hang-s1-c12697.bin              # time, peak memory, outcome
READ_FUZZ_TRACE=1 target/fuzz/read-fuzz replay step F                                   # each stage's time
```

**CI** (.forgejo/workflows/ci.yml, job `fuzz`): the corpus test, then `run all` for 4 minutes on a
PR and 20 nightly, with a fresh seed each run (printed, so a failure replays), 30 s per case and
3 GB.

**Long runs** take a dataset as seeds (`--seeds`; nothing of it is kept but the findings, which
are written outside the repository) and an hour or more per target. A finding from one becomes a corpus
recipe on an in-repository seed, or a handmade input, never the dataset file.

**Corpus** (tests/corpus/TARGET/): each finding fixed. A `.case` file is a recipe ("from FILE in
the repository, replace X with Y", `repeat N:` for runs) and a plain file is the input itself.
`cargo test --release -p forge-read-fuzz` replays them all and needs each to end in its time
without a panic. The .fg inputs are named `.input`, because forge-lang's parse_diff reads every
`.fg` in the repository with a recursive reference parser.

### What it found, and the fixes

Run on the repository's seeds, about 5 minutes per target on 4 cores, then again after each fix.
All of these are fixed at the root.

| reader | finding | cause | fix |
|---|---|---|---|
| step (mesh) | 50 s on one face: a hole's circle made radius 1e6 | `forge_geom::budget` counted spends made with no limit set down from `u64::MAX`, so `left() == u64::MAX` ("no budget live") was never true after the first spend; the tolerant mesh of a file's body never opened a budget and its point walks were not counted | `budget::limited()`; `tessellate_tolerant` opens `budget::FILE` when none is live; `locate` steps count (1/16 each, a step being ~5 ns against a spline evaluation's 200–500); refused in 1 s. The same `left() == u64::MAX` test was in `mass` and forge-lang's measures (their parallel paths), now `limited()` |
| step | 3 GB and an abort: an EDGE_LOOP naming its edges 100 times | the mesher cut the repeated edges finer each round | `finish` (STEP, IGES, .brep): a face's loops go along an edge at most once each way |
| iges (mesh) | index `u32::MAX` in cdt.rs | a loop's count cut to 1; one face's points far from the origin for their spread, so the super triangle rounded to nothing and a point fell on its side | points the super triangle cannot hold are an error; inserting on the hull is an error, not a missing neighbour |
| fg | stack overflow at 5000 brackets, a 30 000-term sum, 5000 unary minuses | the recursive descent parser, evaluation and drop of a tree that deep | the parser refuses depth past 400 (each nested expression, block and prefix operator, and each link of a chain counts) with `Syntax(TooDeep)` at its span; evaluation nesting (calls included) is held at 600, as `Fuel`. Both fit a thread's 2 MB stack (checked under `ulimit -s 2048`) |
| fg | shift overflow in the fast parser | a nonterminal past 64 used as a terminal's bit | `checked_shl` |
| fg | i64 overflow (range end + 1), capacity overflow, an assert in `Frame::rotated_about_z`, i64 overflow in the sheet's vertex grid | numbers no part has: count(1e26), count(2^63 − 1), −1e308 deg, a flange leg of 2^63 | a number not finite or past 1e12 (angles: 1e5 rad) is `TooLarge` where a builtin that is not arithmetic takes it (a feature's, a sketch's or a placement's argument; §Language below); a range, and an index, holds at most 2^20. (`dmath::sin_cos` past its reduction's 2^20 π/2 gives no unit pair; making it take whole turns off first changed the exact results of the speed gate's fuzz parts, which pass such arguments, so the kernel is left as it is and the language's bound keeps files from it) |
| fg | 309 GB asked at once; 2 GB in the crossing sweep and the sketch | polar(n: 2^32), polar(n: 1e6) | a count that sizes geometry (a polygon's sides, a pattern's copies) is at most 100 000 (`TooLarge`); the sweep's margin is rounding's (1e-6), not 1e-3 |
| fg | 1.5 GB in `fg check` | a loop made 65 536 sketches and cutouts | a run executes at most 10^7 statements and makes at most 20 000 features (`Fuel`) |
| fg | 4.7 GB asked at once | a hull of 3000 discs: the sketch's freedom took a dense rank of one block | a block past 2^21 entries is taken as of full rank (its freedom not counted) |
| fg | 78 s | a hexagon nut of 65 535 sides: `chain_loops`' node search, `check_crossings`' every pair, `seg_id`'s search were each quadratic | a grid, a sweep over boxes (pairs judged in the old order, so the same error comes first), one lookup: 0.3 s |
| brep | add overflow in `default_segments`, then minutes in the check's crossing sweep | a circle's edge given the range −1e308..π/2: 65 536 samples, all overlapping | `finish`: an edge runs at most once round a closed curve, over a finite range |
| brep | index `u32::MAX` in cdt.rs's constraint walk | a line's edge from −1e308: the walk stepped through the hull | running off the hull is an error |
| brep, step | minutes in one constraint walk, then in `legalize`'s flips | a cylinder moved 1e6 off its edges; a sliver face (CADFS seed) | the walk checks the budget every 64 steps; flips count with the walks: refused in 1.4 s and 10 s (within the budget's 7–17 s) |
| brep | 116 s | a cylinder moved to y = −1e308: its face laid out over a span of 1e-308, every coordinate subnormal, every predicate on subnormals | subnormal points are refused (interior ones dropped) |
| step | 35 s, refused only at the end | a hole's circle made radius 2^64: constraint recovery looked at the budget every 1024 crossings (each a search of thousands of steps), its Delaunay rounds never | every 64, and each round: 2 s |
| step | index `u32::MAX` in tessellate.rs (CADFS seed) | a grid's staircase over a node never placed | the face fails cleanly |
| dwg | index out of bounds (2 sites) | a file cut short in its first 0x15 bytes; an object map ending where its CRC is | `get` |
| git | slice index panic | a signature with `>` before `<` | the `>` searched after the `<` |
| git | abort allocating | a pack delta declaring a result of 2^50 bytes | capacity bounded by what the delta's ops can make |

No finding remained in the last runs of json, material, inflate, ttf, stl, obj or 3mf. Five
findings were the harness's own and are fixed there: memory an earlier case left resident counted
against every case after it (a case's memory is now its peak less what was resident before it),
its own unzip inflating 256 MB members of mutants, a mutant grown past the memory cap inside the
mutator, a 64 MB deflate bomb whose making outran the timeout, and quadratic repeats.

**Seeds from the datasets** (a long run of step on the 1531 CADFS files) found the last two rows'
CADFS cases; their recipes would need the dataset file, so the fix stands on the in-repository
cases and the rerun.

**Speed** (`GATE_COUNTER=callgrind tools/speed/gate.sh`): passes against the stored baseline, bits
identical. Against the base branch's own gate run on the same machine, every kernel row (seq,
nary1) is within ±0.4%, the most +0.25% (bracket app). Two changes were taken back to keep that:

- `dmath::sin_cos` taking turns off huge arguments changed the exact results of the gate's fuzz
  parts.
- The profile's grid, sweep and lookup cost small sketches up to 1.1%; they now start past 64
  items.

The run before the fixes, about 2 minutes per target: dwg 426 panics in 20 s of cases, git 11,
step 1 abort and 6 hangs, iges 4 panics, fg 16 058 findings (stack overflows, shift overflows,
memory and hangs), fcstd none.

### Budgets: a refusal, not a hang

Bounds are counted work, not wall time, so a file is refused or not alike on every machine and in
the browser build. Each reader's:

- **STEP/IGES/.brep:**
  - `Limits` bound the bytes (2 GB), values (64 M) and instances (16 M); nesting is bounded too.
  - `budget::FILE` (2^25) bounds one solid's spline evaluations, its mesh's fans and point walks,
    healing and the measures' check.
  - Viewers and `fg check` mesh a file's body under the same budget.
- **.fg:**
  - Parse depth 400.
  - Evaluation nesting 600; calls 100 000 per statement and 10^6 per run.
  - 10^7 statements and 20 000 features per run; components 2000.
  - Numbers within 1e12 (angles within 1e5 rad) where they enter the kernel; ranges and indices
    within 2^20 items; a count that sizes
    geometry within 100 000.
  - The sketch's dense rank is capped at 2^21 entries.
- **Zips (FCStd, 3MF):** members are capped before they inflate: 256 MB each, 1 GB in all,
  1000:1 past 64 kB.
- **STL/OBJ/3MF:** 64 M triangles; a binary STL's count must agree with its length; 3MF copies
  are counted through components before any is made.

The fuzzer's wall-clock timeout is how missing budgets are found, as the first finding was.

### Language: the limits, modelled in Lean

The owner's rule: a branch that changes the .fg language extends research/forge-lang/lean in the same
branch. Here:

- **Parse depth** (`Syntax(TooDeep)` past 400): a check the parser's actions make beyond the
  grammar, like `AssignToNonName`. The grammar, the lexer and the Lean parse are unchanged.
  `FG_LEAN=1 cargo test --release -p forge-lang --test parse_diff` passes with the digest as it was.
- **The magnitude bound** is checked where values enter the kernel (`builtins::within_bounds`, in
  `Args::bind`, on every builtin but the pure numeric ones), not on every expression. Arithmetic,
  user functions and `sqrt` .. `len` may carry larger values. A check on every expression would
  observe every number, and every literal would become structural: a data edit to 1e13 would
  re-read fine and re-run as an error.
  - In Lean (ForgeLang/Bounds.lean), the bound is `runK`: the run, then `Bounded` on the finished
    model's numbers.
  - `setParam_bounded` (P4): a data edit's re-read checks the same numbers a re-run does.
  - `setParam_bounded_ids` (P5), and `runK_mono`, `runK_unique`, `runK_error_stable` (P1, P2).
- **Ranges and indices**: at most 2^20 items. `Core.toIx` bounds them at `maxList`, and Rust refuses
  an index past it too. Split.lean's `split_let` takes the bound (new: `split_let_ok`), and Sites.lean's
  `split_independent` is re-proved.
- **Nesting is counted at calls only**: a call adds its callee body's static depth
  (`syntax::body_depth`) to the run's nest, refused past 200 (`NEST_BUDGET`); a statement is at
  most 400 deep (the parser). There is no per-expression wrapper, so the evaluator's hot path is as
  it was. In Lean, `stack_depth_le` bounds the recursion at 400 + 200 = 600 levels, the same bound
  the per-expression count gave. `admits_some` and `admits_zero` give the guard's invariant: the
  stack's body depths sum within the budget. Calls themselves stay at 128 deep (`DEPTH_BUDGET`).
- **The other limits are fuel**: statements 10^7, features 20 000. They count further work as the
  core's fuel counts calls, and run out as `Fuel`.
- **Checks:** `lake build` (each target the rule names); Axioms.lean lists the new theorems;
  CheckAxioms: 2264 theorems, axioms only propext, Classical.choice, Quot.sound.
- **Nothing that builds is refused.** Every repository .fg (301), the FreeCAD library's 547
  translated parts and 980 translated CADFS parts (research/cadfs's cadfs.py on the importer branch)
  were built on the base branch and on this one, each in its own process with 3 GB and 300 s. Their
  errors are identical message for message, with 0 parts newly refused. One repository thermoform part
  runs out of the 3 GB on both.
- **Speed.** The functions these limits touch are net at or below the base on every part the
  gate's language rows run (callgrind, per function): the per-expression check is gone, and the
  evaluator's list and tuple building no longer grows through a fallible collect.

## 2. STEP import quality

3146 real files:

- 1531 CADFS models: Onshape, AP242, Parasolid underneath. Every 250th of the 382 609 in
  unique_models.json, offset 7, fetched by HTTP range from the public Hugging Face dataset
  VladPyatov/CADFS (cadfs/dataset/step.zip, with research/fillets/oracle/rangezip.py on the fillets
  branch).
- 1615 WHUCAD models: CATIA V5, AP203. Every 4th of the hand-drawn folders 0100–0103 (1115),
  and 500 drawn at random (seed 1) from every 4th file of every 10th DeepCAD-derived folder,
  from gitee fred926/whucad-brep.

Judged by `research/step/rate.py` against OCCT 8.0.1 (`tools/occt/step_read.py`). **Matching**
means valid, not warned, with volume and area within 1e-3 of OCCT's. No dataset file is in the
repository.

| | base (claude/repo-review-mbdrlj) | this branch |
|---|---|---|
| read a solid | 3124 (99.3%) | 3124 (99.3%) |
| valid | 3038 (96.6%) | 3082 (98.0%) |
| matching | 2900 (92.2%) | 2939 (93.4%) |
| crash / timeout | 0 / 0 | 0 / 0 |
| problems (files) | LoopBroken 55, Euler 43, FaceWrongSide 33, LoopsCross 9 | FaceWrongSide 37, LoopsCross 13, VertexTolerance 7, ShellsOverlap 5 |

By system (this branch): CATIA 1610 files, 1594 valid; Onshape 1525, 1488 valid.

### The failures, by cause (this branch, 207 files)

| files | cause | whose |
|---|---|---|
| 115 | valid, volume or area off past 1e-3 | see below |
| 27 | FaceWrongSide (8 with LoopsCross, 1 with OuterNotFirst): a seam through a hole or a slot in a periodic face | ours: next |
| 12 | a whole torus with no edges, or a closed torus face with one rim and no pole, refused | ours: a form not read yet |
| 11 | no solid in the file (surface models, empty) | right: OCCT finds none either |
| 11 | warned: the volume cannot be worked out | ours |
| 10 | warned: integrals and mesh 1.2–9 times apart | ours |
| 5 | OCCT itself invalid | judge |
| 5 | VertexTolerance | ours |
| 5 | ShellsOverlap | ours |
| 4 | a solid refused: its surfaces would take too long to measure (the budget) | budget |
| 2 | LoopsCross | ours |

**The volumes off.** The 118 such files of the run before the seam fallback (they include these 115) were each held against OCCT's own mesh as well: BRepMesh at relative deflection
5e-4, its volume summed. By volume:

- 63: ours equals OCCT's mesh to 1e-3, and OCCT's integral, the truth, disagrees with its own
  mesh. The judge is wrong (research/step/README.md met this before).
- 4: OCCT's mesh is itself broken.
- 13: ours equals both OCCT's mesh and its integral; these miss on area only.
- **38: ours is genuinely off, 0.1–1.5% from both.** 32 of them are CATIA. This is the largest
  real cause left.

Taking the judge's own disagreement out (a file matches if it matches either of OCCT's
measures), about 3006 of the 3135 files that hold a solid match: **95.9%**.

### Fixed here

- **A seam's end stranded** (all 54 LoopBroken/Euler files; Onshape).
  - The cause: a closed rim (a circle) shared by two periodic faces, say a sphere and a cone, each
    needing a seam to it. The second face moved the circle's one vertex to where its own seam
    met the rim, leaving the first face's seam ending off the rim.
  - `pick_vertex_on` now cuts the circle there instead, once another edge uses its vertex.
  - 00003679.step: valid, volume 209805.97381 against OCCT's 209805.97384.
- **A seam along a slot's side** (00583642.step).
  - When both vertices of a rim sit on lines that edges of either rim run along (a symmetric
    slot), the fallback cut the rim at its longest edge's middle, again on such a line.
  - It now takes the point of the rim furthest round from every such line.

No file got worse: the six whose problems changed went from LoopBroken/Euler to a lesser
problem, five of them to FaceWrongSide.

### Next, by size

1. **The 38 volumes off 0.1–1.5%** (CATIA): find which faces, against OCCT's per-face area, as
   research/step/README.md did for the earlier ones.
2. **FaceWrongSide, 26 files:**
   - A seam must also keep clear of a hole that the rim does not bound (00088299: a slot through
     a cylinder wall, the seam through it).
   - `seam_start`'s `clear` takes a hole's parameter range from its sampled edges; this case
     escapes it.
3. **Tori with no edges or one rim (11):** a whole torus needs two seams, one per period.
4. **The judge:** rate.py should keep OCCT's mesh beside its integral, so "matching" stops
   counting OCCT's own disagreement.

Assemblies: the sample's multi-solid files read each solid once, as before (research/step/README.md).
No CADFS file and no file of these 3146 asks for a placement that moves.

## 3. New formats, ranked

| | value | done |
|---|---|---|
| STL, OBJ, 3MF as a mesh | high: printing, scans, vendor parts, measurement | **yes** |
| IGES | medium: the reader exists (B-rep, entity 186); real IGES is mostly trimmed surfaces (144/143), refused by name | no change: sewing is next |
| Parasolid x_t | medium–high, hard | assessed, below |
| ACIS sat | medium, hard | assessed, below |

### STL, OBJ, 3MF

These are read as a mesh, and only as a mesh. Faces, edges and exact surfaces don't come of it,
and nothing builds on it.

- `forge_mesh::read::{stl, obj}`:
  - STL binary and ASCII, told apart by length and first word; corners welded where equal to the
    bit.
  - OBJ: `v` and `f`, negative indices, fans for polygons.
- `forge_import::threemf::read`:
  - The core spec: the model from `_rels/.rels`, its unit, objects, components with transforms
    (mirrors keep their triangles facing out), and build items.
  - A component in another model file (production extension) is refused by name.
- Every error says where: byte and line, or the XML node's line.
- `forge_mesh::read::summary` gives triangles, vertices, closed (every edge shared once each way),
  volume (only when closed), area and box.
- `forgecad mesh FILE.stl|obj|3mf [--json]` prints that, and says it is a mesh, not a B-rep:

```
cube.obj: a mesh (triangles, not a B-rep: for reference and measurement), 12 triangles, 8 vertices
  closed; volume 1000.000000 mm3
```

Not done:

- A mesh as a reference body in a .fg part (`mesh("x.stl")` beside `import`): it needs a mesh
  kind of body the language can show and measure but not build on.
- 3MF materials and colours.

### IGES

Read today: entity 186 (manifold solid B-rep) on 190–198 (plane, cylinder, cone, sphere, torus),
read exactly, and 128 (rational B-spline) surfaces, through the same healing and checks as STEP.

What real systems write is mostly 144 (trimmed surface) on 128 or analytic surfaces, with 142
(curve on surface) boundaries and no topology. Reading those as a solid means sewing:

- match boundary curves within tolerance into shared edges;
- orient the faces;
- refuse what does not close.

Each 128 surface that is provably a plane, cylinder, cone, sphere or torus can be read exactly. A
rational B-spline whose control net lies exactly on such a surface with the conic weights is
recognisable within rounding; the rest stay B-splines as STEP's do.

This is the next IGES step, about the size of `heal.rs`'s seams. It was not started here.

### Parasolid x_t (text)

- **Value.** Parasolid is the kernel under SolidWorks, NX, Solid Edge, Onshape (the CADFS set)
  and others; x_t is their native exchange, often sent where STEP loses fidelity.
- **Format.** Siemens publishes the XT format reference. A text x_t is a schema-driven graph of
  nodes:
  - body, region, shell, face, loop, fin, edge, vertex;
  - geometry: plane, cylinder, cone, sphere, torus, B-curve, B-surface, offset, swept, spun and
    blend surfaces.
- **Feasibility.**
  - The topology maps closely onto `Proto` (a fin is a coedge), and `finish` would take it as
    it takes STEP.
  - The schema is either embedded or named (`SCH_*`). Named schemas need Siemens' schema files,
    which can't be shipped: only files with an embedded schema, or of a version whose layout is
    written down, could be read.
  - Rolling-ball blend surfaces carry their spine and radius, not a surface. Their exact
    evaluation is a kernel feature in itself, so such faces would be refused by name, and blends
    are common.
- **Estimate:** 2–3k lines for analytic and B-surface bodies.
- **Recommendation:** worth doing after IGES sewing, for analytic and B-surface parts only.

### ACIS sat

- **Value.** ACIS is the kernel under SpaceClaim, older Inventor and AutoCAD's 3DSOLID (whose
  data is SAT inside the DWG, so forge-dwg would read it too) and others.
- **Format.** Text SAT, documented by Spatial for its early versions:
  - records of body, lump, shell, face, loop, coedge, edge and vertex, with attributes and history;
  - geometry: analytic surfaces, and `spline` surfaces given procedurally (offset, rolling ball,
    skin, sweep), each with a B-spline approximation `bs3_surface` attached.
- **Feasibility.**
  - Analytic bodies are straightforward.
  - Procedural splines are exact only through their procedure, so a reader would use the
    approximation, which is not exact: refuse by name, as for Parasolid blends.
  - Versions change the record layout often, and post-7.0 files need care.
- **Recommendation:** lower than Parasolid, except through DWG's 3DSOLIDs, where it is the only
  3D data there is.

## Rerun

1. Fuzz: §1.
2. STEP rate:
   1. Stage the files as above (scripts in the session scratchpad; the fillets branch's
      `rangezip.py` for CADFS, gitee's API for WHUCAD).
   2. OCCT truth: `tools/occt/step_read.py --jobs 2 @LIST`.
   3. `research/step/rate.py BIN OUT --truth T --jobs 4`.
   4. `rate.py --report OUT`.
3. The mesh check of the off files:
   - BRepMesh at relative deflection 5e-4, its triangles' volume summed;
   - one process per file under `ulimit -v`, since an absolute deflection ran out of memory on
     the large parts.
