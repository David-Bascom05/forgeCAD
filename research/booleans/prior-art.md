# Fast, exact booleans: prior art, and what to adopt (2026-11-23)

The owner asked for this on 2026-11-23: "if the bool agent is having speed issues, then lets do a
prior art search on faster ways to do bools and try to prove those algorithms." The boolean must be
"crazy fast, since it is in basically everything". Everything must meet research/PROOF-STANDARD.md
(L1–L6), and nothing may get slower than today's kernel.

Branch `claude/repo-review-mbdrlj-bool-research`, based on decide-once-lean @ 556b178. Read for
this note:

- verify/forge-exact/ForgeExact/DecideOnce (Ledger, Core, Frontier, Local, LocalFinal);
- research/kernel-audit/decide-once/aeneas/README.md;
- `research/decide-once/curved-design.md` on dol-curved;
- kernel/forge-bool/src/decide_once.rs, in particular `arrange`, `side_at`, `build_ledger`,
  `fill` and `write`;
- kernel/forge-math/src/exact.rs (`Big`, `Dy`, `DyPlane`, `HPoint`, `meet3`).

The one new proof is `research/booleans/lean/Sparse.lean`. It is core Lean v4.36.0-rc2 (the
forge-exact toolchain) and compiles with `lean Sparse.lean`. Its theorems use only the standard
axioms:

- `skip_pos` and `skip_neg`: a plane with one strict sign at every vertex of a convex cell has that
  sign on the whole cell, for homogeneous integer points;
- `mul_fits`, `add_fits` and `int_mul_fits`: bit budgets compose.

Every measured number below is the authors' own, on their own hardware, unless it is marked
otherwise. Numbers for our kernel are instruction counts per boolean on the coplanar bench, from the
master's brief.

## 0. Summary

1. **We already have the fastest papers' representation.** EMBER, the octree BSPs and trueform
   are fast because a vertex is the meet of three planes and every predicate is a sign of a
   polynomial in plane coefficients. No constructed coordinate is ever stored. Our `HPoint`
   from `meet3` over `DyPlane` is the same idea. What they have and we lack:
   - **locality**: work only where planes actually cross;
   - **fixed-width arithmetic**: no allocation, with widths known statically;
   - **lazy construction**: filter first, and build the exact number only when the filter
     fails;
   - **untouched faces copied, not rebuilt.**
2. **Nothing in the literature beats a decided-once exact ledger on robustness.** Each fast
   method decides something differently:
   - Manifold perturbs (symbolic, on float ties);
   - trueform votes (a weighted majority over observations that disagree);
   - Cherchi re-casts rays with `nextafter`;
   - EMBER and trueform quantise the input to an integer grid.

   None of those four ideas suits us. Each one changes the answer on some input that works today,
   or is a rule our L3 could not state.
3. **Ranked levers** (§2). Each keeps the ledger bit-identical, so the proven core and
   `end_to_end_proof` are untouched:
   - **A2 sparse arrangement**: the skip test runs once per plane on the section's corners, and
     covector entries are inherited;
   - **A1 splice write**: far faces are copied, not re-chained;
   - **A3 lazy vertices**: indirect predicates with a proven f64 filter;
   - **A4 fixed-width exact arithmetic**, with a decided fallback to `Big`;
   - **A5 flat layout**: packed covectors and arena tables in the bridge.

   That is the order of §2's table. Estimated together (§2.2): **11.65M → about 4.6M instructions per boolean (2.5×)**. That is
   still about 3.5× the old path's 1.3M.
4. **The owner's rule cannot be met by these levers alone** on the coplanar bench (§4). Closing the
   rest needs either A6/A7 (§2.1), or decide-once staying behind the old path, as it does now,
   until it is no slower line by line.

The levers come from the brief's list as follows: "sparse arrangement" is A2, "incremental write"
is A1, "inline-limb Big" is A4, and "body-local reading via hulls" is A6.

## 1. Comparison

Legend:
- Exact: E means exact predicates and exact topology; D means decided by a stated rule; F means
  floating point with no guarantee.
- Ledger: can our proven core consume the method's output as a decide-once ledger (cells with
  covectors, facets, memberships)?
- Lean: an estimate of the L1–L5 effort if we adopt it: S is days, M is a week or two, L is
  months.

| Method (source, date) | Algorithm | Measured speed (source) | Robustness / exact? | Degeneracies (coplanar, touching) | Ledger? | Lean L1–L5 |
|---|---|---|---|---|---|---|
| **EMBER**: Trettner, Nehring-Wirxel, Kobbelt, "EMBER: Exact Mesh Booleans via Efficient & Robust Local Arrangements", ACM TOG 41(4) art. 39, July 2022. [pdf](https://www.graphics.rwth-aachen.de/media/papers/339/ember_exact_mesh_booleans_via_efficient_and_robust_local_arrangements.pdf) | Adaptive kd-type subdivision of the scene box, with early-outs for subproblems that cannot affect the result. In leaves, each polygon gets a local BSP of only the segments that cut it. A reference point with a known winding-number vector is carried down the recursion, and polygons are classified by tracing at most three segments to it. | Thingi10K pairs: geometric mean **1.6 ms** per boolean at about 20k faces. 1.2M triangles: 34 ms on 8 cores, against QuickCSG 1010 ms and Zhou et al. 141 s. Fig. 13: about 10× faster than the fastest *inexact* method. | E, provided the input is 26-bit integers or EMBER's own output (§3.3, §6 of the paper). | Exact plane predicates throughout. Polygons on a split plane go "left". Coplanar overlaps are handled after the BSP. | Partly. A leaf's polygons lie in cells of the leaf's planes, and the WNV is per-operand membership (`memb`). Covectors need one sign per class plane, as our `arrange` computes. | L (subdivision, WNV tracing). The pieces we adopt (A2, A4) are S–M. |
| **Octree-embedded BSPs**: Nehring-Wirxel, Trettner, Kobbelt, "Fast Exact Booleans for Iterated CSG using Octree-Embedded BSPs", CAD 135:103015, 2021. [arXiv 2103.02486](https://arxiv.org/abs/2103.02486) | A global octree with a BSP in each cell, so edits stay local. Plane-based geometry with fixed-width integers, and static bit budgets per expression. | Up to **2.5M mesh–plane cuts/s on one core** (i7-7700K). Table 1, cycles per operation, 128/192/256-bit against CGAL lazy and gmp: intersect three planes **23/160/402 against 4200/6380**; classify vertex **13/103/142 against 710/1540**; plane from points 226/512/761 against 2770/9100. | E within the bit budget: 26-bit input for 256-bit arithmetic. Their table: 0.22 mm resolution at 128-bit, 1.58 µm at 192-bit, 11.5 nm at 256-bit. | Exact. | As EMBER. | The number layer is S–M (A4). |
| **trueform**: Sajovic, Knez, "trueform: fast and robust mesh CSG via topological aggregation", arXiv 2607.15905, 17 July 2026. [pdf](https://arxiv.org/pdf/2607.15905) | Input is snapped to an int32 or int64 grid. Pairwise contacts are typed VV/VE/VF/EE/EF by exact predicates on the input planes, with "exact without exact constructions". Each cut face is arranged in its own plane. Radial order around edges is decided by exact orient. Where observations of one topological unit disagree, a weighted majority vote decides. One build answers any n-ary expression as a per-domain bit test. | 1000 Thingi10K pairs (M-series, TBB): median **15.7 ms**. Geometric-mean slowdowns of the others against it: MeshLib 5.5×, Manifold 7.6×, EMBER 10.4×, Cherchi 28.7×, CGAL 32.1×, Geogram 100.4×. A 22M-triangle, 201-operand union arranged in 0.5 s. A re-query costs 13–37× less than the build. | E for predicates on the grid. The vote is a D rule over inconsistent readings of materialised geometry. | Coplanar packs are collapsed. Contact types are exact. SoS is deliberately excluded, and used only as a fallback for unseeded domains. | Its domains with operand bitvectors are exactly our topes with `memb`. The build-once, query-many split is our ledger/core split. | The vote has no L3 statement in our terms. The parts we adopt (typed contacts as table keys, A5) are S. |
| **Cherchi, Pellacini, Attene, Livesu**, "Interactive and Robust Mesh Booleans", ACM TOG 41(6), Dec 2022. [arXiv 2205.14151](https://arxiv.org/abs/2205.14151) | The arrangement of Cherchi et al. 2020 ("Fast and robust mesh arrangements using floating-point arithmetic", TOG 39(6)), with an octree, parallel structures and linear ear-cut. Points are implicit (indirect predicates). orient3d caches per-triangle 3×3 minors, so each call is a 4-term dot product. Labels come from one ray per patch, cast through a float-then-exact cascade. | Rotation demo: **20 ms at 25k triangles, 140 ms at 200k** (M1 Pro), against libigl's Zhou et al. at 390 ms and 1.50 s. Thingi10K, 3,814 booleans: 4.5 min against 28.3 min. Labelling about 11× faster on average. These figures come from a text extraction of the arXiv HTML; check them against the paper before quoting. | E arrangement. The ray is re-cast with `nextafter` on degenerate hits, and the authors say their list of such cases is not exhaustive. | The exact arrangement handles coplanar faces. Rays perturb. | The arrangement gives cells and facets. Ray labels give `memb` per patch. | M for indirect predicates (A3). The ray re-cast rule is not L3-statable. |
| **Indirect predicates**: Attene, "Indirect predicates for geometric constructions", CAD 126:102856, 2020. [arXiv 2105.09772](https://arxiv.org/abs/2105.09772) | A predicate on a constructed point is evaluated from the primitives that define it, through static, semi-static and interval filters, then expansions. | The authors report it faster than lazy exact approaches (CGAL `Lazy_exact_nt`). It is the arithmetic of Cherchi 2020/2022 and of Diazzi et al. 2023 (CDT). | E. | n/a (arithmetic). | n/a: it is how `side_at` should be computed. | M (A3: a forward error bound, with verify/ieee754). |
| **Guo, Fu**, "Exact and Efficient Intersection Resolution for Mesh Arrangements", ACM TOG 43(6) art. 165, Dec 2024. [project](https://mangoleaves.github.io/projects/mesh-arrangements), doi 10.1145/3687925 | Indirect predicates reformulated for lower float error, so filters succeed more often, plus localised, dimension-reduced sorting and deduplication of intersection points. | The project page reports faster than Cherchi 2020. I did not read the full text, so no numbers are given here. | E. | Exact. | As Cherchi. | Its filter reformulation feeds A3 (lower error means fewer exact fallbacks). |
| **Lévy**, "Exact predicates, exact constructions and combinatorics for mesh CSG", arXiv 2405.12949, v1 May 2024, v2 June 2025 (Geogram). [arXiv](https://arxiv.org/abs/2405.12949) | Corefinement with an AABB tree and a per-triangle CDT, a Weiler 3-map, and combinatorial classification propagated across darts. One exact ray per connected component. Expansions or multi-precision floats, both behind interval filters. | Coplanar cases: 0.9–11.5 s; it beats Zhou on all, and Cherchi 2022 on two of four, the other two of which Cherchi 2022 crashed on or got wrong. ThingiCSG: Manifold always fastest, but wrong on coplanar cases (nasty_gear_1). The paper quotes EMBER's published 4.5 ms for 20rods, against its own 864 ms. These figures come from a text extraction; check them before quoting. | E. | The CDT is unique, so coplanar overlaps coincide. Incircle uses SoS. Rays are randomly re-launched. | The combinatorial propagation is our flood, `memb` across facets (`flipB`). | Little to adopt for speed: it is slower than EMBER. |
| **Zhou, Grinspun, Zorin, Jacobson**, "Mesh Arrangements for Solid Geometry", ACM TOG 35(4), 2016 (libigl) | A global arrangement of all triangles with exact rationals (CGAL Epeck). Winding numbers are propagated over cells. | EMBER Fig. 1: **141 s** where EMBER takes 34 ms. Cherchi: 28.3 min on Thingi10K. | E. | Exact. | Yes: cells with winding numbers are topes with `memb`. It is the closest in spirit to our flat L2 reference. | Its slowness is the warning: global arrangements plus exact rationals. |
| **CGAL Nef polyhedra**: Hachenberger, Kettner, Mehlhorn, "Boolean operations on 3D selective Nef complexes", CGTA 38, 2007. **CGAL corefinement**: Loriot, CGAL Polygon Mesh Processing, 2017+ | Nef: sphere maps at every vertex, a global overlay, and exact lazy rationals. Corefinement: refine only along the intersection, Epeck, and label by connected component. | Lévy, ThingiCSG 17_Tree: **Nef 1721 s, corefinement 11 s, Manifold 2 s**. ochafik, quoted in [manifold#383](https://github.com/elalish/manifold/discussions/383): fast-csg 30–150× faster than Nef, and Manifold a further 5–30×. trueform: CGAL 32× slower than trueform. | E. | Nef: full generality, including non-manifold and lower-dimensional sets. | Nef is a selective complex, exactly a ledger of all cells with marks. | Why Nef is slow: it represents **every** cell of the overlay globally, with an exact number per coordinate and allocation per operation. Our current `arrange` repeats the "every cell, every plane" part (§2, A2). |
| **Manifold**: Lalish, [github.com/elalish/manifold](https://github.com/elalish/manifold), wiki edited Feb 2024. Based on Smith, Dodgson, "A topologically robust algorithm for Boolean operations on polyhedral shapes using approximate arithmetic", CAD 39(2), 2007 | Float geometry and exact integer topology. Each geometric question is computed once, from earlier answers ("decide once", in its own words). Symbolic perturbation on ties: the first operand moves along its normal for union, and against it for difference or intersection. A parallel BVH (Thrust/TBB). | Blog, Mar 2022: 100–1000× faster than the OpenSCAD release (CGAL) on GPU. trueform: 7.6× slower than trueform (median 118 ms). Lévy: fastest on ThingiCSG. | D: the topology is manifold by construction. The geometry is only "ε-valid", which the wiki says "cannot be mathematically proven". | Ties are perturbed, so nothing is ever "on". This is wrong on some coplanar inputs (Lévy). The roadmap says the coplanar rule needs work. | No covectors (no zeros by design). | Its consistency rule is our Theorem (a). SoS contradicts `reg_kept` on touching n-ary inputs, so we do not adopt it. |
| **BSP CSG**: Naylor, Amanatides, Thibault, SIGGRAPH 1990; Bernstein, Fussell, "Fast, exact, linear booleans", SGP 2009; Campen, Kobbelt, EG 2010 (plane-based, exact) | Booleans as BSP merges. Bernstein–Fussell introduced plane-based polygons with filtered predicates. Campen–Kobbelt made them exact. | Superseded by EMBER, which builds on these. | E (plane-based). | Exact. | A BSP leaf is a convex cell with a covector, like ours. | — |
| **Integer and fixed-precision schemes**: Fortune, "Polyhedral modelling with multiprecision integer arithmetic", CAD 29(2), 1997; Nehring-Wirxel 2021; EMBER; trueform's int32→128 and int64→256 ladders | Quantise to a grid, bound the degree of every predicate, and pick a static width. | See the octree row's Table 1 (two orders of magnitude over gmp). | E on the grid. Quantisation changes the input. | Exact. | n/a. | The width lemmas are here (`mul_fits`). The fixed-limb L4 is M (A4). |
| **Simulation of simplicity**: Edelsbrunner, Mücke, ACM TOG 9(1), 1990 | Symbolic ε-perturbation, so no predicate is ever zero. | Cheap. | D, but the answer is that of a perturbed input. | It removes coplanar and touching cases rather than deciding them. | No: the zeros are what our covectors are made of. | Rejected: it would change `reg` (`XNonManifold`, touching results). |
| **B-rep kernels**: Parasolid (Jackson, "Boundary representation modelling with local tolerances", ACM SMA 1995, [pdf](https://ftp1.cs.wisc.edu/pub/users/prem/jackson-SM-95.pdf)); ACIS (tolerant modelling, proprietary); OCCT Boolean Operations Algorithm ([spec, 8.0](https://dev.opencascade.org/doc/overview/html/specification__boolean_operations.html)) | Per-entity tolerances, with every coincidence decided against them. OCCT's General Fuse "pave filler" computes the interferences V/V, V/E, E/E, V/F, E/F, F/F once, in increasing dimension, into one data structure, then splits only faces that have interferences, then classifies. Fuzzy mode adds a global tolerance. | Nothing systematic is published. The OCCT docs show fuzzy mode 0.047 s against 2.09 s on one example. | D: tolerance-based, with tolerances that grow (OCCT raises vertex tolerances: `Tol(V)=max(Tol(V), D+Tol(E))`). | Tolerances. | The pave filler's interference table is a ledger by another name: one decision per pair, every later use a lookup. | Its "split only faces with interferences" is A1's prior art. Growing tolerances are what our snap avoids (Snap.lean). |
| **Lazy exact arithmetic**: Shewchuk, "Adaptive precision floating-point arithmetic and fast robust geometric predicates", DCG 18, 1997; Meyer, Pion, FPG, 2008; CGAL `Lazy_exact_nt`; LEDA `real` (Burnikel, Fleischer, Mehlhorn, Schirra, 1999–2000); CORE (Karamcheti, Li, Pechtchanski, Yap, 1999) | Filter cascades: a static or semi-static float bound, then an interval, then exact. LEDA and CORE use separation bounds for algebraic expressions. | Shewchuk: orient3d costs a small factor over plain float when the filter succeeds. Octree Table 1: CGAL lazy costs 10–100× fixed width. | E. | n/a. | n/a. | We have orient2d, cross_sign, dot3, incircle, TwoSum and TwoProduct at L5 (ieee). The filters for A3 are the same kind of statement. |
| **Local or incremental re-evaluation**: SolidWorks' local update; Pariente et al., CAD 1996; the octree BSPs' locality | Re-evaluate only features or regions a change can affect. | Not quantified publicly. | — | — | Our F1 `far_unchanged` and F2 `frontier_local` are the proven version for one boolean. | A1 uses them. A7 (§2.1) would extend them across rebuilds. |

What is new since 2022, and beats the methods above: **trueform** (July 2026), 10.4× EMBER by its
authors' measurement. Its speed comes from three things: per-face local arrangements of only the
pairs that touch; integer predicates on input planes without constructions; and flat graph data
with no global structure. **Guo & Fu (2024)** improves the filter success rate of indirect
predicates. **Lévy (2024/25)** is exact and robust, but slower than EMBER. None of these changes
the ranking. They confirm that locality, constructions-free predicates and flat data are where the
speed is.

## 2. Recommendation, ranked by expected speed-up per unit of proof effort

The profile, per boolean, on the coplanar bench (from the brief):
- fill 4.9M: arrangement 1.5M, the rest of `build_ledger` 1.6M, inputs, snap and vertices 1.2M,
  witnesses 0.5M;
- core 1.45M;
- write 5.3M.

The total is 11.65M, against the old path's ~1.3M. upto takes 65M against 41M.

The estimates below are mine, from reading the code. I asked decide-once (Rust) and bool-speed for
the splits they rest on (the near-face share, the planes crossing each section, `Big`
allocations); §2.2 says which number each estimate depends on.

| Rank | Lever | Prior art | Phase it hits | Expected | Proof effort | Why this rank |
|---|---|---|---|---|---|---|
| 1 | **A2 sparse arrangement**: per class plane, the skip test runs once per other plane, on the corners of the box section. Planes that do not cross the section keep one strict sign, written into every vertex covector without `side_at`. | EMBER's local BSPs and early-outs; trueform's per-face arrangements; Cherchi's octree candidate pairs | arrangement 1.5M, and part of `build_ledger` | arrangement **1.5M → 0.4–0.6M** | **S.** The ledger is bit-identical by `skip_pos`/`skip_neg` (done, §3.1). L4 is the bridge's `arrange` in Aeneas style. | The lemma is already proven. No change to cells, order or names. |
| 2 | **A1 splice write**: far faces are copied with their loops, not re-pooled as segments and re-chained. Only near faces and the core's faces go through `chain`, `area`, `inside_loop` and `build_body`. | OCCT GF (only faces with interferences are split); trueform (non-intersected faces reuse the input's links); EMBER's early-outs; Manifold's Compose | write 5.3M | **5.3M → 1.3–2.5M**, scaling with the near-face share | **S–M.** L1/L3 come from F1 (`far_unchanged`) plus one lemma: re-chaining a far face's own coedges gives back its loops. L4 is the splice, in Aeneas style. | The largest phase, and the spec already exists. Its bits must be shown identical first (§4). |
| 3 | **A5 flat layout**: covectors packed as two `u64` masks (nonzero, positive; 64 classes or fewer, with the current `Vec<Sgn>` beyond that), `HashMap<Vec<Sgn>,_>` replaced by sorted arenas, `Vx.cov` without its own `Vec`. The core's interface is unchanged: rows are unpacked at `encode`. | trueform's bit tests per domain; Manifold's arrays over hash tables (its #383 notes hash tables were much slower) | the rest of `build_ledger` 1.6M; the arrangement | **1.6M → 0.8–1.0M** | **S.** A data refactor of the bridge, which is not at L4 yet. Its L4 becomes part of the planned `build_ledger` L4 (`LedgerWF` on the skeleton). | Constant factors. No geometry. |
| 4 | **A3 lazy vertices (indirect predicates)**: a vertex is its three plane ids plus an f64 homogeneous shadow computed straight from the plane coefficients (no `to_f64` of a `Big` ratio), with a proven error bound. The exact `HPoint` is built only when a filter fails, or for `cell_point` and output. | Attene 2020; Cherchi 2020/2022; Guo & Fu 2024; EMBER/trueform's "exact without constructions" | inputs, snap and vertices 1.2M; witnesses 0.5M; vertex creation in the arrangement | **1.7M → 0.7–0.9M** | **M.** L5: a forward error bound for `n·X − cW` with X, W the 3×3 minors, against verify/ieee754 (the same kind as orient3d). L3: the sign equals the exact one whenever the bound certifies it. | It needs a new L5 lemma, and the ieee session owns that machinery. |
| 5 | **A4 fixed-width exact numbers**: per working region, clear the dyadic exponents to integers with one shared shift, compute each coefficient's width, and run `meet3` and `side` in fixed limbs (i128/i192/i256) when the static budget fits, else in `Big`. The width check is exact, so the values are identical. | Nehring-Wirxel 2021, Table 1 (23 against 6380 cycles for a three-plane meet); EMBER; trueform; Fortune 1997 | every exact fallback that remains after A3; `meet3` | Today, on what remains after A3: **0.3–0.6M**. More if the `Big` profile shows allocation dominating. | **M.** L3: the budget lemmas (done, §3.3). L4: Aeneas on fixed-limb add and mul (u64 limbs, loops without `break`). | It overlaps the planned "inline-limb Big", and its value depends on what A3 leaves. |
| — | Rejected: SoS, majority votes, input quantisation, global Nef-style overlays, and ray re-casting by `nextafter` | — | — | — | Each would change results or has no L3 statement. | §4 |

### 2.1 Beyond the five

- **A6: body-local reading through hulls, and EMBER-style subdivision of the region into leaves.**
  - What it does: split the near region into boxes, each with its own six walls and local ledger,
    and union the results.
  - The proof is a generalisation of `split_at_region` and `local_correct` from one region to a
    partition of regions. Stitching at the shared walls is the hard part.
  - Size: L (weeks to months). The asymptotic gain is large for big near sets. It is small on the
    coplanar bench, which has about 23 classes. Do it when parts with hundreds of near classes
    appear.
- **A7: reuse across rebuilds.**
  - The decided ledger is a pure function of the exact inputs and the op. Caching the boolean's
    result under a key of the exact input bits needs no new proof, provided the key is compared in
    full (no hash-only hit).
  - A parametric edit that moves one feature changes the near region. `far_unchanged` then says
    which part of the previous result can be kept.
  - First check what research/replay and research/rebuild already cache at the feature level, then
    measure upto's share of repeated booleans.

### 2.2 Expected totals against the profile

| Phase | Today | After A2+A1+A5 | After A3+A4 too | Depends on |
|---|---|---|---|---|
| arrangement | 1.5M | 0.5M | 0.4M | planes crossing each section (asked) |
| rest of `build_ledger` | 1.6M | 0.9M | 0.9M | hash and alloc share (asked) |
| inputs, snap, vertices | 1.2M | 1.2M | 0.6M | the share of exact `meet3` and `to_f64` (asked) |
| witnesses | 0.5M | 0.5M | 0.3M | the same |
| core | 1.45M | 1.45M | 1.45M | proven; packing it would need a new Aeneas proof |
| write | 5.3M | 1.8M | 1.8M | near-face share (asked) |
| **total** | **11.65M** | **≈6.4M** | **≈5.5M** | |

The core is now the largest single phase. The next step, after the bridge, is packed covectors
inside `forge_bool_core`. That re-runs the translation, and the refinement lemmas for `cov`,
`find`, `split` and `membership` change their row type, which is an M-sized proof. With it, the
core goes 1.45M → about 0.6M and the total ≈ 4.6M. That is 2.5× today, and still about 3.5× the
old path.

## 3. Proof plans for the top three

All three keep the ledger the bridge hands to `forge_bool_core::core` bit for bit. So `CoreSpec`,
`end_to_end_proof` and `local_correct_sampler` are reused unchanged. Each lever's obligation is
only "the new bridge computes the same ledger, or the same body" (L3), plus its own L4 and L5.

### 3.1 A2, sparse arrangement

- **L1.** No new statement. The spec is the existing one: the arrangement of class plane `c`
  within the box is the list of its cells (`covCell`, Correct.lean).
- **L3.** The new `arrange` returns the same `Plane2` as the current one, which gives the same
  ledger. The argument:
  - The current loop skips plane `j` iff every vertex of every current cell is strictly on one
    side.
  - The cells tile the box section `S`, which is convex. The corners of `S` are cell vertices, and
    every cell vertex lies in `S`, so it is a nonnegative combination of `S`'s corners.
  - So by `skip_pos`/`skip_neg` (Sparse.lean, proven): if every corner is strictly on one side,
    so is every vertex. The converse holds because the corners are vertices.
  - The skip decisions are therefore identical. The test moves from cell vertices to `S`'s 3–6
    corners, and its answer is computed once per `(c, j)`, not once per pass.
  - Inherited covector entries: if `j` is skipped with sign `s`, every vertex `h` in `S` has
    `side(j, h) = s` by the same lemma. So `covector` can write `s` without calling `side_at`,
    which gives the same row.
  - What remains is to state `S`'s corners as combinations: `S` is clipped from the u/v quad by
    walls 4 and 5. In Lean this is `section_` in Ref/Arrange.lean, so the lemma is "every point of
    `section_` is `comb` of its corners", which is S-sized.
- **L4.** `arrange` is rewritten in the Aeneas style (README "What translates": no borrowed
  struct, no `break` or `return` in loops, no `Ordering`). The new loop over `j` takes a
  precomputed `skip: Vec<i8>` (0 = crossing, ±1 = constant sign). The refinement lemma is
  `arrange_rs = arrange_lean` with `skip` given by corner signs. Until the bridge's L4 lands, a
  debug assertion compares every inherited entry with `side_at`, and an L6 differential compares
  both versions' `Plane2` on forge-exact-fuzz and the repros.
- **L5.** None new. The corner signs use `DyPlane::side` (exact) behind the same `side_at`
  filter, whose bound is the L5 item the ieee session has.
- **Reuse.** The core, the sampler proofs and `LedgerWF` are untouched. Only the bridge's
  `build_ledger`/`arrange` obligation gets the lemma above.

### 3.2 A1, splice write

- **L1.** No new statement. `far_unchanged` (F1, Frontier.lean) says the result agrees with the
  input outside the region. `split_at_region` says it is the far part plus the local result.
- **L3.** The spliced body equals `write`'s body. Two lemmas:
  - (i) **Re-chaining is the identity on a far face.** A far face's segments in `Grp::Far(g)` are
    exactly its own coedges, so `chain` returns its loops: the same cycles, up to rotation, and
    `build_body` normalises the start vertex and the order of loops. The spliced copy must
    reproduce that normalisation exactly. If `build_body` sorts loops by area or starting
    vertex, the splice applies the same key.
  - (ii) **Faces at the frontier are untouched.** A face is far iff `!reg.near[g]`, and the walls
    cut only near faces. So far faces meet near or core faces only at original vertices
    (`PK::Orig`), and the shared vertex and edge ids are the same in both constructions.
  - (iii) A far face's name and orientation are copied, which is what `unique` already assigns
    when there is no clash. The splice runs `unique` over the far names in the same order.
- **L4.**
  - The splice itself (copy loops, remap ids) is plain array code and goes through Aeneas.
  - `write`'s near half stays as it is. It is f64 output assembly, not yet in any L4 survey row,
    and it is marked interim.
  - The **L6 gate is decisive here:** callgrind, plus bits identical on every bench, fuzz and repro
    line, before the splice is switched on.
- **L5.** None: the splice does no arithmetic.
- **Reuse.** F1, F2 and `split_at_region` are proven. This is their first use for speed.

### 3.3 A5, flat layout (A4's width lemmas included)

- **L1.** None.
- **L3.**
  - **Packing.** `pack : Vec<Sgn> → (u64, u64)` is injective on rows of width 64 or less, and
    `unpack ∘ pack = id`. That is S in Lean: the bit lemmas are core Nat facts. Every hash and
    equality on packed rows then agrees with the unpacked ones, so `by_cov` deduplicates
    identically.
  - The cell order is set by the insertion order, which is unchanged.
  - Rows wider than 64 keep the `Vec<Sgn>` path. That is a decided rule, on the width, which is
    exact.
  - **Widths.** For A4, `mul_fits`, `add_fits` and `int_mul_fits` (proven here) give the static
    budget of `meet3` (degree 3 in plane coefficients, with W of degree 3) and of `side` (degree 4).
    If the coefficients fit in `b` bits after the shared shift, then `meet3`'s coordinates fit in
    `3b + 3` bits and `side` in `4b + 5`. Those are the constants to check against the fixed limb
    count.
- **L4.** A5 is part of the bridge's planned `build_ledger` L4 (`LedgerWF` on the Aeneas
  skeleton). Packed rows make that proof easier: no nested `Vec` in the translated state.
  `encode` unpacks, and its `EncodeSpec` is unchanged.
- **L5.** None.

## 4. Risks to "nothing slower, nothing that works now fails"

1. **The levers do not close the gap alone.** Even at ≈4.6M, decide-once is about 3.5× the old
   path's 1.3M on the coplanar bench. So the route that is shipped today stays: decide-once runs
   behind the old path, or where the old path fails, until the callgrind gate (no line more than
   0.5% worse) passes line by line.
   - The literature's remaining factor is allocation-free flat data end to end, as in trueform and
     EMBER: no `Vec` per number, covector, polygon or HashMap key.
   - So after A1–A5, the next measurement is the allocation count. `Big::L` and `Dy` are where to
     look.
2. **Bit identity of the splice (A1).** If `write` today normalises far faces in a way the splice
   does not reproduce (loop order, start vertex, dropped zero-length segments, names through
   `unique`), output bits change. That breaks "nothing that works now fails" for any downstream
   name reference.
   - Mitigation: lemma (i) first, then the bits gate on every corpus, and the splice switched on
     only with both.
3. **A2's corners must be the exact section corners.** If the corner list were taken from the f64
   box rather than the exact clipped quad, a plane passing exactly through a corner would be
   misjudged. The rule is the exact `side` at the exact corners. Zero counts as crossing, which
   is conservative: then it splits as today.
4. **The A3 filter's precondition.** `plane_f` rounds `c = n·o`, which is a dyadic, not an f64. The
   error bound must include that rounding, and debug builds check the precondition (`|c|`
   finite, no subnormal). A filter that is wrong in one ulp case would silently change a sign, so
   A3 ships only with its L5 lemma, not before.
5. **A4's width fallback.** Normals with noise components of about 1e-17, common after rotations,
   widen the aligned coefficients past 256 bits. Those inputs take the `Big` path, the same as
   today. Cost: one exact width check per plane per region, about 10 instructions. The fixed path
   must never be taken past its budget: the width check is exact (`Big::bits`), and Aeneas proves
   no overflow under it.
6. **Rejected ideas stay rejected:**
   - SoS (Manifold) would turn exact touching into an arbitrary side, and change the
     `XNonManifold` refusals that L3 ties to `reg`;
   - votes (trueform) answer an inconsistency that decide-once cannot produce;
   - quantisation to a grid (EMBER, trueform) moves input vertices by up to half a grid step. That
     changes results that work today and bypasses the snap's tolerance classes.

## 5. Sources

Fetched or checked 2026-11-23:
- EMBER: https://www.graphics.rwth-aachen.de/media/papers/339/ember_exact_mesh_booleans_via_efficient_and_robust_local_arrangements.pdf
- Octree-embedded BSPs: https://arxiv.org/abs/2103.02486
- trueform: https://arxiv.org/abs/2607.15905
- Cherchi et al. 2022: https://arxiv.org/abs/2205.14151
- Lévy: https://arxiv.org/abs/2405.12949
- Attene, indirect predicates: https://arxiv.org/abs/2105.09772
- Guo & Fu 2024: https://mangoleaves.github.io/projects/mesh-arrangements
- Manifold:
  - https://github.com/elalish/manifold/wiki/Manifold-Library
  - https://github.com/elalish/manifold/wiki/Performance-Considerations
  - https://elalish.blogspot.com/2022/03/manifold-performance.html
  - https://github.com/elalish/manifold/discussions/383
- Parasolid tolerant modelling (Jackson 1995): https://ftp1.cs.wisc.edu/pub/users/prem/jackson-SM-95.pdf
- OCCT Boolean Operations: https://dev.opencascade.org/doc/overview/html/specification__boolean_operations.html

Cited from the literature, not re-fetched:
- Zhou et al. 2016;
- Hachenberger et al. 2007;
- Shewchuk 1997;
- Edelsbrunner and Mücke 1990;
- Smith and Dodgson 2007;
- Fortune 1997;
- Naylor et al. 1990;
- Bernstein and Fussell 2009;
- Campen and Kobbelt 2010;
- Meyer and Pion 2008;
- LEDA and CORE.

No third-party code was copied, and nothing was added to kernel/.
