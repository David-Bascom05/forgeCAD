# Fillets: hand-off (2026-11-22)

This is the state of step 0 of research/fillets/general-method.md §6: the Tier A membership oracle,
and the census labelled against it. Step 1 (the crease-graph builder) is **not** to be started. It
waits for certified surface-intersection tracing (research/kernel-audit/README.md, items 4 and 6).

No kernel code changed in this work. Every change is under verify/fillet-oracle,
research/fillets/oracle, and one example, verify/forge-proof/examples/cliffcheck_fg.rs.

## 1. State

### The oracle: verify/fillet-oracle (crate `forge-fillet-oracle`)

**Its question.** "Is this point under some admissible ball?" For concave picks a point outside the
body is filled exactly when no clear ball of radius r covers it.

**How it answers** (`covered`). Branch and bound over the ball's possible centres in B(x, r),
best-first by an upper bound on clearance. The obstacle is an `Obstacle`: a 1-Lipschitz signed
distance with an error bound, so a box of centres with middle m and half-diagonal h has
clearance within sd(m) ± (err + h).

- **Out**: a witness centre c with sd(c) − err ≥ r − τ and |c − x| ≤ r.
- **In**: every box's upper bound falls below r − τ.
- **Unknown**: the search reached its smallest box, or its budget, without either.

τ = 1e-7 is the spec's tolerance (§1.4).

**Obstacles.**

- `Field` gives analytic solids with counted error bounds: half-spaces, round bars or their holes,
  capped rods, blocks, and unions. The bound is 80 u (|p|∞ + size), twice a 40-flop forward-error
  count. It is not yet machine-checked: that is obligation O9.
- `Kernel`, in examples/label_body.rs, takes a kernel body as the obstacle: the least distance to its
  faces (the foot where the face holds it, else its edges), and its side from forge-bool's `Index`.
  Labels from it are only as good as the kernel's projections and trims, and are marked
  `"obstacle":"kernel"`.

**Spec** (`Spec`, `Zone`). The whole closing away from the vertex table. Within ρ_v of a vertex
where picked and unpicked edges meet, it uses row 2 of the table: filled when no picked edge's
wedge closing (its two faces' surfaces carried on, as a union of half-solids) covers the point,
within the cap half-spaces. ρ_v is just under the distance to the nearest generator not at v.

Unit tests: a right-angle wall pair (points across the arc; the strip's area r²(1 − π/4)), and the
rod and block distances.

### The labellers

**examples/label_pipe.rs: the pipe set, certified obstacle.**

- The obstacle is the part's two solids, exactly, from the .fc twin. The kernel's own body must agree
  with it at 2000 random points, else the part is `unjudged`.
- The kernel's result is judged at points δ = max(1e-3 r, 1e-4) either side of every new face, and
  at 20000 random points round the picked edges (box grown by 2r).
- A disagreement counts only if both the kernel and the oracle give the same answer at the six
  points δ round it.
- Labels:
  - `wrong` when any such disagreement remains: `over` means the kernel has material the spec does
    not; `under` is the reverse;
  - `unsure` when the only disagreements are body points the kernel's result (or its `Index`
    classification) reports outside (`lost`);
  - otherwise `right`, or `refused` / `invalid` as the kernel left it.
- The fill volume by 100k oracle-only points is printed too. Its σ is 3–6%, too wide to judge
  volume by. Use the point labels.

**examples/label_body.rs: any .fg part, every fillet feature in it, kernel obstacle.**

- The spec is restricted to the rounded edges: the sharp edges of the body before whose names
  are gone after.
- These are not counted:
  - within 3r of a table vertex, any disagreement (the table's mitre lies outside the closing);
  - within 3r of an unpicked edge of the round's own kind, a disagreement where the spec changes
    more than the kernel (the whole closing or opening reaches edges the restricted spec does not).
- A fillet of several radii, of both kinds, or with setbacks is `unjudged`.

### The census, as labelled so far

**Pipe set** (gen.py seeds 0–199, the kernel at fe8e0f1b): 170 of 200 labelled before the machine
moved; all 200 are now in research/fillets/oracle/pipe-labels.jsonl.

| label | parts |
|---|---|
| right | 125 |
| wrong | 13 |
| unsure | 4 (p00007 p00071 p00137 p00167) |
| refused | 28 |
| not yet labelled | p00168, p00170, p00172–p00199 |

**Of the kernel's "ok" parts labelled, 13 of 142 are wrong against the oracle:**

- `over`: p00006 (3), p00010 (295), p00029 (328), p00072 (1), p00094 (4), p00111 (60), p00124 (6),
  p00133 (66), p00164 (181);
- both: p00056 (o3 u1), p00080 (o18 u1), p00128 (o9 u1);
- `under`: p00023 (u13, plus 185 lost).

The large `over` counts are material hanging past a rim (see p00080 below).

**Superseded by research/fillets/census.md (2026-11-22):** OCCT's classifier overrules forge-bool's
`Index` at every counted point of 14 of these parts, p00080 included, and of all 5 unsure. The pipe set
ends at 161 right, 8 wrong, 31 refused.

**FreeCAD set** (this session's run of the library: 547 translated bodies, 576 fillet features in
the parts). A first label_body run got through 236 fillets, but it excused the wrong direction for
**convex** rounds: it counted the opening's rounding of unpicked convex edges as kernel errors. The
fix is in the committed code, checked on 0cda5e35.Body028 (now right) and 47000b9e.Pocket (one real
disagreement). **The FreeCAD labels must be rerun: nothing from the first run stands.** Rerun in census.md: all 33
wrongs, 47000b9e.Pocket's included, are the oracle demanding a ball wider than the wall; OCCT's own
fillet equals ours.

The 45 FreeCAD fillet refusals, by cause (filletwhy on each;
research/fillets/oracle/freecad/refusals-by-cause.txt):

| cause | count |
|---|---|
| one model's seam-split cone-to-sphere rims, with the strip face shorter than r (10 copies × 3 bodies) | 30 |
| the ball wider than a short or thin face | 7 |
| a spring over a sharp rim (planes, cylinders) | 8 |

**CADFS and WHUCAD were not labelled** (since done: census.md; CADFS came from Hugging Face, no Onshape key). Their runs (~/forgecad-data/{cadfs,whucad}/runs/...,
which census.py reads) are not on this machine. /home/user/datasets/CADFS is a blobless clone with
no models, and CADFS needs Onshape access to fetch. The source-STEP comparison for CADFS (the
coordinator's ask) is therefore still to do.

## 2. p00080 and p00137

The fillet-only added volume (body + boss + round, less body + boss):

| part | ours (fe8e0f1b) | OCCT | closing_pipe, step 0.05 | closing_pipe, step 0.1 |
|---|---|---|---|---|
| p00080 | 62.751 | 62.564 | 60.510 ± 0.204 | 60.448 ± 0.201 |
| p00137 | 41.524 | 41.466 | 39.741 ± 0.157 | 44.051 ± 0.174 |

**p00080: the construction is wrong, and the closing (cliff) fixes it.**

- Ours and OCCT agree to 0.3%: both build the face-face rolling ball.
- OCCT's round less the body, classified against the closing's candidate cells (0 < d < r, step
  0.1, every 8th cell), agrees with the closing there: 60.16 in both, 0.016 OCCT-only, 0.264
  closing-only, which is grid noise at the arc.
- OCCT's grid volume of its own cut is 62.56, so about 2.4 of OCCT's fill lies **r or more from the
  body**, where no closing fills.
- The oracle finds the same in our result, with points such as (7.81, −17.45, 21.50) held
  robustly. That point is 4.8 below the box's side face y = −12.6153, and the round has material
  there.
- So the face-face spring on the face x = 12.6153 runs past the box's vertical edge. The kernel
  carries the plane on past its rim (the README's "carried supports") and closes material in the
  air beyond the side face.
- The spec has the ball rest on that edge instead: a cliff (a crease onto the rim's tube).
- p00006 is the same past the body's bottom rim, and so, in larger counts, are p00010, p00029,
  p00111, p00133 and p00164.

**p00137: the reference failed there, not the round.**

- The grid closing moves 11% between step 0.05 and step 0.1, far outside its ambiguity band. Its
  centre search (gradient ascent from 7 starts) can miss a clear ball. Its "covered" answers are
  certified, its "added" answers are not. So its 2–4% gap was no evidence.
- The oracle gives `unsure` for p00137: no shape disagreement, but 10 body points that the kernel's
  `Index` classifies outside the result. Either the result lost body material, or its
  classification is wrong there. Its volume (41.52 added) suggests the classification.

## 3. The fallback inventory: the list of things to retire

Line numbers are on fe8e0f1b. "R" marks a retry or preference order, the things to retire. "G"
marks a geometric rule with its own condition, which becomes a predicate or vertex type in general-method.md. "N" marks a numerical
alternative for one and the same surface or curve, which is not a choice of shape.

### kernel/forge-ops/src/fillet.rs

| line | item | what it decides | kind |
|---|---|---|---|
| 375 `fillet` | `check_against` problems → rerun with `seams: true`; then if `carries`, rerun `straight` | the rerun is kept when its check is clean | R |
| 425 `fillet_with` | refused and `carries` → `fillet_tried(straight)` | a chain carried past a kink < 0.5° ends at the kink instead | R |
| 443–454 `fillet_with` | Overflow/Corner/Mitre → `fillet_cliff::retry`; Ok but `!counts_sound` → cliff retry, else keep | the cliff replaces a refusal, or a result whose Euler count is unsound | R |
| 458–530 `fillet_tried` | Overflow → `across` (roll_across); → `seams: true`; Overflow/Corner → walk tries `(seams, rims)` in a fixed order, bounded by `walk_spend`; `only` → rerun without `only` to name a FreeEnd | the first try whose body checks clean and has mass | R (the FreeEnd rerun is a diagnosis only) |
| 532 `fillet_picked` / 1111 `minority_first` | picks of both kinds meeting at a vertex: the kind with fewer edges there is rounded first, the rest roll over its faces | the order of two roundings | G (a convention; see §2.7) |
| 566–590 | `SplitFirst` loop (≤ 4): an edge is split where asked and the round rerun | topology prep requested by the geometry | G |
| 583–588 | built, but notched over a rim and back (`notched`) → cliff retry, else keep the notch | notch versus cliff | R |
| 596–602 / 749 `reseamed` | Overflow/Corner → seams moved round, rerun | the result depends on where the seam was | R |
| 606–619 / 624 `joined_pieces` | Corner/Mitre/Overflow → edges split by a needless vertex joined, rerun, kept if check and mass pass | topology normalization run as a retry | R |
| 989–1130 pass loop | `rim_split` only when `opts.rims`; `fillet_cliff::split` only when `opts.cliff`; refused at pass 2 | whether a spring crossing a rim splits the edge depends on which retry is running | R |
| 1042–1057 `across` | `crossing_pairs` / `roll_across` (1853 / 1906): one blend across a strip face whose springs cross | only inside the `across` retry | R, wrapping a G construction |
| 1772 `roll` | `blend_of(near)` → `rolled_centre` start | the start of the trace | N |
| 1801–1811 `roll` | `switch_support` (smooth edge: rest on the face beyond), then `cliff` only if none switched | smooth before sharp | G when the test is "is the rim G1" (it is, `switch_support` 2124) |
| 1951 `cliff` | rails other than lines and circles only when `rims`; `lean`: the ball holds at the edge's middle, else at `near` only when `rims` | which rims the ball may rest on depends on the retry | R |
| 2316 `rim_split` | skips rims through the edge's own vertices | a mixed-convexity vertex's rim is never cut (the A case) | R (an unstated rule) |
| 2821–2843 `walk_tests` / `walk_spend` | a budget of foot tests across retries | a refusal that depends on work done | R |
| 2990–3040 `Builder::build` | `open_first` (walk only): each vertex beside a short blend edge is resolved on a snapshot, and the stuck ones are walked over, (a, b) then (b, a) | pass over a short edge, or not | R |
| 3018–3020 | resolve errors: kept for a walk only when `opening`, the error is Corner, and a short edge is near | R |
| 3069–3092 | loops that cross, touch or come near on a face; a spring across another loop → Overflow | guard refusals | G (each states its condition) |
| 3109 `without_spurs` | only when the Euler count is unsound: drop a seam walked out and back | cleanup applied only after a failure | R (should be unconditional) |
| 3177–3186 | `closed_up` / `unsure` and check fails → Overflow | "a path that might be wrong" refused after the fact | R |
| 3330 `resolve`, one hot | `switched_out` if it rests beyond; else `run_out`, and on Overflow, `roll_over` | run out across the faces, or roll on over the next one | R |
| 3366–3404 joint | `joint(a, b)` → `joint(b, a)` → `walk_joint(a, b)` → `walk_joint(b, a)` (walks only when `opening`, and loft faces bounded by `walk_spend`) | which blend's section ends the joint | R |
| 3405–3418 / 5322 `mitres` | ring of 3, same convexity, plain → `mitre`; if it refuses and the edges are tangent here (`continues_here`), `joint` | **mitre-first at a mixed-convexity vertex**, whether or not the far point exists | R + G |
| 5340 `mitre` | `nearest_hit` far points; `beside` → `crossing_edge`, else the nearest hit | which hit is the far point | N/G |
| 5738 `corner` | `corner::setbacks` closed form, else `ball_setbacks` | two setback rules for the n-blend patch | R (unless they agree, see §2.6) |
| 3870–3895 `run_out` | `cross_curve`, else `towards_touch` walk a→b, else b→a | the cross curve is closed form or traced | N |
| 4412/4444/4460/5117 | `unsure = true` on swallowed-strip paths | feeds the after-the-fact refusal at 3182 | R |
| 6036–6057 | `excursions`: a spring off over a rim and back is notched (the round trimmed by the top carried on); `notched` set for the cliff retry | notch | R (the notch is the fallback) |
| 6153 `rims_in_part` | a cliff's spring on part of a rim cuts the rim | G |
| 7415 `cross_curve` | `closed_form`, else traced | N |

### kernel/forge-ops/src/fillet_cliff.rs

| line | item | what it decides | kind |
|---|---|---|---|
| 23 `retry` | `retry_whole`, else `joined_loop` (seam-split loop made whole) then `retry_whole` | R |
| 33–64 `retry_whole` | closed concave picks only; `cliff_seams` false then true; kept when check and mass pass | seams left in place, or all moved | R |
| 72–112 `split` | `seam_off` when the stretch wraps the edge's vertex; `rim_seam_off` when the rim's vertex is in the stretch; else Overflow | seam handling, decided by where the seam lies | R |
| 118–168 `clear` | guards: the cliff ball clear of the rim's face and its neighbours; each face-face piece's springs on their faces | G. **Bug:** it walks from t0 to t1 the shorter way round a closed spine (`along`), so a face-face piece longer than half the loop is sampled through the cliff and refused. Seen on p00015. |
| 191–313 `find` | one off-stretch only (`let [i0] = starts`); with stretches off both faces, the one begun first | G (narrow) |
| 498 `counts_sound` | feeds the unsound → cliff retry | R |

### kernel/forge-model/src/lib.rs (`apply_fillet`, 1631)

| line | item | what it decides | kind |
|---|---|---|---|
| 1681–1696 | built, but `blend_through` (the ball comes out on the part's far side) and one kind → Overflow | G (a refusal with its condition) |
| 1697–1699 → 1717 `overlapping_rounds` (1849) | Overflow/Corner/Mitre → each vertex-connected group of picks rounded alone, the added or removed parts booleaned | R (the boolean is the construction this design generalises) |
| 1702–1712 | built but Euler-unsound → Invalid | G (validity refusal) |

### kernel/forge-geom/src/blend.rs and forge-ops/src/corner.rs

`Blend::new`: closed forms (plane–plane, axis, parallel cylinders, coaxial; 413, 499–500), else a
traced pipe. `Pipe::rail` / `rail_polish`. corner.rs:81 `trim_curve` fallback; corner.rs:304
`close_hole`, else `close_hole_rational`. All of these are N: the same surface or patch reached two
ways. They are not choices of shape. **No `Quality` retry exists in the fillet path**: `Quality`
appears only in forge-model/fillets.rs's mesh tests.


## 4. How to run everything

The scratchpad did not come along. Everything below rebuilds from the repo and the datasets.

### Build

```
cargo build --release -p forge-fillet-oracle --examples
cargo test --release -p forge-fillet-oracle
cargo build --release -p forge-proof --example cliffcheck_fg --example filletwhy
```

### Pipe set

The parts come from the generator and need no download:

```
python3 -I research/fillets/oracle/fc_twin.py PIPE 0 200
```

This writes PIPE/pNNNNN.fg, byte for byte what gen.py writes, and the .fc twins.

- Kernel results, one line a part: `sh research/fillets/oracle/pipe_run.sh target/release/examples/cliffcheck_fg PIPE out.jsonl`.
  research/fillets/oracle/pipe-kernel-fe8e0f1b.jsonl is fe8e0f1b's run.
- Oracle labels: `SAMPLES=20000 VOL_SAMPLES=100000 JOBS=3 target/release/examples/label_pipe PIPE PIPE labels.jsonl [IDS..]`.
  This takes about 30 s a part. Progress goes to stderr, one line a part.
- Why a fillet refuses: `target/release/examples/filletwhy PART.fg`. Summarise a directory of its
  reports with `python3 research/fillets/oracle/whyclass.py DIR`.

**OCCT references.** The venv is set up by tools/occt/setup.sh (it was at
/root/.local/share/forgecad-occt/venv). Each script takes the .fc twin:

- `occt_pipe.py FILE.fc`: body + boss filleted by OCCT; its added volume and validity;
- `occt_against_closing.py FILE.fc R DUMP STRIDE`: OCCT's round classified at closing_pipe's
  candidate cells;
- `occt_grid_volume.py FILE.fc R STEP`: OCCT's cut, its GProp volume against a classified grid.

**closing_pipe.rs** is the old grid closing. Build it with `rustc -O`, then run
`closing_pipe FILE.fc STEP`. It is superseded by the oracle, and kept to compare with.

### FreeCAD set

**Sources.**

- The FreeCAD library: `git clone https://github.com/FreeCAD/FreeCAD-library` (it was at
  /home/user/datasets/FreeCAD-library).
- The 245 files with fillets were staged by hash under /home/user/datasets/scratch-census/freecad/files.
  research/fillets/oracle/freecad/map.tsv has hash → library path: the first column is the file
  name (HASH.FCStd), the fourth the path in the library.
- research/fillets/oracle/freecad/fc_files.txt lists the 245.

**Translation and judging against each body's own shape:**

```
python3 tools/find-parts/recreate/run_freecad.py run fc_files.txt --files FILES --out RUN \
  --judge target/release/examples/judge --topo target/release/examples/topo \
  --venv OCCT_VENV/bin/python --jobs 2
python3 tools/find-parts/recreate/run_freecad.py report RUN
```

This needs the judge and topo examples of forge-proof. It took about 45 minutes.

**Oracle labels:**

```
grep -l 'fillet(' RUN/parts/*.fg > list.txt
PER_FACE=40 RANDOMS=600 JOBS=2 target/release/examples/label_body out.jsonl list.txt
```

Lines also go to stderr as each fillet is labelled.

### CADFS and WHUCAD

- `research/fillets/census.py JUDGE RUN OUT` reads ~/forgecad-data/{whucad,freecad,cadfs}/runs/RUN.
- tools/find-parts/recreate/run_cadfs.py and run_whucad.py make those runs.
- WHUCAD's source is https://github.com/fazhihe/WHUCAD, as cloned at /home/user/datasets/WHUCAD-full (1.3 GB).
- CADFS (https://github.com/VladPyatov/CADFS) holds only the tooling. Its models need Onshape API
  credentials, whose environment's .env.example names them.

label_body takes any of their .fg parts as they are. For CADFS, compare the result with the
source STEP as well: step_truth.py in tools/find-parts/recreate, and the README's 1e-4 match rule.

## 5. What's next

1. Finish step 0:
   - label the rest of the pipe set (p00168, p00170, p00172–p00199);
   - rerun label_body on the whole FreeCAD set with the convex fix;
   - stage the CADFS and WHUCAD runs and label them, CADFS against its source STEP too;
   - report how many of today's "ok" are wrong.
2. Possible sharpening, if wanted:
   - certify the `Kernel` obstacle for planes and quadrics: exact face distances, and a trimming
     test from the loops' exact curves;
   - narrow the 3r excusal radii by computing ρ_v as label_pipe does;
   - a stratified volume estimate (the uniform one's σ is 3–6%).
3. **Stop before step 1.**
