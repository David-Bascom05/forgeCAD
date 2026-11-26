# A8 face arrangement: bool-research's first draft and measurements (2026-11-23)

> Superseded as the design by the Lean coordinator's research/decide-once/face-arrangement.md
> (decide-once-lean, 2ff687bd). This file is kept for the measurements in §0 and §5 and the cost
> table in §6. The review of the coordinator's design is research/booleans/a8-review.md.

The master decided on 2026-11-23 to move decide-once from the plane arrangement to a face
arrangement (A8 in research/booleans/small-booleans.md §5).

This is a draft for the Lean coordinator (session_01Q19Lzx8iDqzvcMQsS9ACqs) from bool-research. It
covers:
- the cell definition;
- the explicit incidence the core reads;
- what carries over from the current proofs;
- the new core's proof size;
- expected per-phase numbers;
- the measured cell counts on the gate parts.

Branch `claude/repo-review-mbdrlj-bool-research`. Base: decide-once-lean @ 556b178.

## 0. Corrections first

1. **The old path costs ≈1.3M per flat boolean, not 3.0M.** small-booleans.md §2 was wrong on
   this.
   - Measured here with callgrind (x86-64-v2), coplanar seq with the switch off:
     `forge_bool::boolean::boolean_into` takes 200.6M inclusive over 210 booleans.
   - Of those 210, the 140 NotFlat booleans cost 111M (decide-once Rust's figure). That leaves
     89.6M for the 70 flat ones, **1.28M each**.
   - Decide-once at bool-speed's head spends 573M on those 70, which is **8.2M each, 6.4× the old
     path**.
   - Old-path split per boolean (all 210, inclusive): trim 0.31M, split_face 0.15M, imprint 0.11M,
     classify 0.07M, apply 0.06M.
   - **The old path's write is about 0.06M,** because `boolean_into` edits the target in place.
     Decide-once's write is 0.68M.
2. **The "~300 → ~40 cells" estimate was wrong in kind.** The ledger never held 300 cells, and the
   arrangement *process* is what is large. Measured per boolean (§5):
   - the plane-arrangement ledger has about 231 cells: 47 pieces, 72 edges, 30 vertices, and the
     rest topes;
   - the face-arrangement piece estimate is about 30;
   - the 300 were the *section* cells `arrange` builds (9.5 crossing classes × 33.6 cells × 9
     sections), most of which are never pieces.

   So A8 cuts the arrangement work about 10×, and the ledger and core cells about 1.5×. Explicit
   incidence then makes each core step O(degree) rather than an O(classes) covector scan.
3. **On the gate, only coplanar (70 booleans), housing (1) and upto (4) reach the flat decide-once
   path.** The 15 real bench parts have none: every one of their booleans is NotFlat. The flat
   path's speed shows on three gate lines. The curved path is where the real parts are.

## 1. Cells

The data, inside the open region `R°` (decide-once-spec §3a; far faces are handled by
`far_unchanged`):
- the near faces `f` of all bodies, each with its closed polygon `P_f`, clipped to the region's
  box, on its carrier class (after the snap, as now);
- for each `f`, its **cut set**
  `X_f = ∂P_f ∪ ⋃ { P_f ∩ P_g : g near, cls g ≠ cls f } ∪ ⋃ { P_f ∩ ∂P_g : g near, cls g = cls f, body g ≠ body f }`.
  The last term is the imprint of a coplanar overlap. The box's walls clip the polygons; they are
  not carriers.

The cells:
- **Vertices**: the finite set of points where `X_f`'s segments end or meet, over all `f`. Each is
  the exact meet of three carriers (a wall counts as a carrier here), or a polygon vertex; that is
  `meet3`, as now.
- **Edges**: the components of `⋃_f X_f` minus the vertices: open segments, each on the line of
  two carriers, or of one carrier and a wall.
- **Facets**: the components of `relint P_f ∖ X_f`, one per region of the face cut by its cut set.
  Facets of different faces with the same point set (coplanar overlap) are **one facet**, a piece of
  both faces, as now. A facet is a polygon that may have holes and need not be convex. The core
  never asks for discs (decide-once-spec §6).
- **Sides**: each facet `φ` has two facet-sides `(φ, +)` and `(φ, −)`, by its carrier's
  orientation. Around each edge `e`, the facets whose closure contains `e` have an exact **radial
  order** about `e`'s line.
  - Consecutive facet-sides in that order face one wedge, so they are joined.
  - A **side id** is a class of facet-sides under the join, computed by union–find. It replaces a
    tope.
  - Two side ids may lie in one geometric volume, for example the outside of an inner shell and the
    inside of an outer one, which share no edge. That is harmless: each class gets its own witness,
    and the witnesses are exact.
- The radial order is the one new predicate. At edge `e` on the line of carriers `a` and `b`, a
  facet on carrier `c` leaves `e` in direction `±(n_c × d_e)` within `c`, and two such half-planes
  are ordered by a sign of a degree-2 expression in the plane coefficients. The same predicate
  appears in trueform (§2.4, radial structure "by exact predicates on the original input planes")
  and in Lévy (radial sort with orient and Norient). It is exact, and filtered like `side_at`.

Two things are no longer cells: the full 3D cells of the plane arrangement, and the parts of a
carrier plane outside every face. That is exactly where the plane arrangement's cost went.

## 2. The ledger the core reads: explicit incidence

```lean
structure LedgerF where
  nb        : Nat
  faceBody  : List Nat            -- per input face, its body
  faceCls   : List Nat            -- per input face, its carrier class
  facetCls  : List Nat            -- per facet id, its carrier
  pieces    : List (Nat × Nat)    -- decided: facet φ lies in face f
  sides     : List (Nat × Nat)    -- per facet id: (side id of +, side id of −)
  inside    : List (Nat × Nat)    -- decided: side id T is inside body i (one witness per class)
  edges     : List (List Nat)     -- per edge id: the facets round it, every one, kept or not
  verts     : List (List Nat × List Nat)  -- per vertex id: the edges round it, the facets round it
```

The core is today's Core.lean with three primitives replaced. Every stage keeps its shape:

| Today (covectors) | Face arrangement |
|---|---|
| `φ.isFacet`, `φ.zeroIdx` | `facetCls φ` |
| `φ.plus`, `φ.minus` | `sides φ` |
| `topes` | the side ids |
| `Cov.le e φ` (edge or vertex below facet) | `φ ∈ edges e` / `φ ∈ (verts v).2` |
| `Cov.le v e` | `e ∈ (verts v).1` |
| `around kt x` | the result facets in `x`'s list |
| `fanRel v c d` | some `e ∈ (verts v).1` with `c, d ∈ edges e` |
| `joinRel` | same carrier, same orientation, sharing an edge |

The stages that keep their shape: `crossings`, `memb`, `keep`, `keepTab`, `result`, `orient`,
`badEdge`, `badVert`, `unify` (`erel`, `heads`, `faceOf`) and `core`.

**Recommendation: state the core once, over an interface.** Define
`structure Inc (C : Type) where carrier : C → Nat; plus minus : C → C; le : C → C → Bool`. The
current `Cov` core is the instance `⟨zeroIdx, plus, minus, Cov.le⟩`, and the face arrangement is
the table instance. Then Closure.lean, the stage definitions and every proof that uses only the
interface's laws are shared, and the old instance keeps its theorems as corollaries.

The laws the proofs need, which `Cov` has and the tables must satisfy (part of `ExactF`):
- `carrier` is defined on facets;
- `le` is transitive;
- the two sides of a facet differ;
- an edge's facets round it are its listed ones.

## 3. Spec and correctness (L1, L3)

**L1 is unchanged:** `reg_kept`, `split_at_region` and `far_unchanged`. Only the decomposition the
correctness theorem uses changes.

`ExactF L ps A` replaces `Exact`. Its clauses:

| Clause | Statement | From |
|---|---|---|
| `pieceFacet`, `oneFace` | as now, with `facetCls` | unchanged in form |
| `cover` | each face's closure is the union of its facets' closures (with the cut set) | new, and easy: `X_f` is finite segments |
| `compat` | every point just off facet `φ` on side `s` has the membership of its side id, for every body | **`contains_const_local`** (JordanLocal.lean, proven): parity is constant along a segment that meets no face. Plus a **wedge lemma**: for consecutive facets round an edge, the arc between points just off them, near the edge, meets no face. The wedge lemma is the one new geometric proof. |
| `witness` | `inside` lists exactly the side ids, per body, whose witness point is inside | as now: one exact parity per class (`Witness.lean`'s shape, B-rep route) |
| `boundary` | `Flip φ i ↔ isPieceOf i φ` | from `compat` plus the face's orientation, as now |
| `complete` | every boundary point of `reg` lies in the closure of some facet | new form: a boundary point lies on some face's polygon (the inputs are B-reps), so it lies in some facet's closure by `cover` |
| `edges`, `verts` | the incidence lists are the true ones; `edges e` holds every facet whose closure contains `e`, in radial order | new: the radial predicate's exactness |

For the touch rule, `XNonManifold` ↔ `ZeroThickness` needs the explicit incidence to hold *every*
facet round an edge. That is the `edges` clause, which closes the gap curved-design.md §1.3 (iii)
found for `Cov.le`. The face arrangement gets the property by construction, which also helps the
curved route.

What carries:
- unchanged: Topology, Regular, Frontier, Closure, Snap, SnapSym, Ledger.lean (the decided-once
  tables, a function of keys), Nef.lean (`opRule`), and Jordan and JordanLocal;
- generalise, with the interface: Core, Coverage (`splitOk_of` and `classifyOk_of`, which use
  only the laws), Correct (`correct`, `zeroThickness_iff` over `ExactF`), Metric (`collapse` over
  side ids), Local and LocalFinal (`ExactIn` becomes `ExactFIn`);
- replaced: Sample, Sampler, SampleLocal, Witness, and the box files. These are the plane
  sampler and its exactness (about 2.2k lines). In their place go the face-arrangement builder's
  exactness (`cover`, `edges`/`verts`, the wedge lemma).

Estimate: about 2–3k lines new or changed in verify/forge-exact. About half is the builder's
exactness, and the wedge lemma is the hardest single piece.

## 4. Rust and Aeneas (L4)

**The new core.** `forge_bool_core` has 1,338 lines today. The covector machinery goes:
- `cov`, `find`, `intern`;
- the row index;
- `around`/`between` with their `ENUM_ZEROS` enumeration;
- the position-to-covector bridges.

Its replacements are id tables: `Vec<u32>` per edge and vertex, `[u32; 2]` per facet. These are
loops over plain arrays, which suit Aeneas. The style rules stay: no borrowed struct, no
`break`/`return` in loops, no `Ordering`.

**The refinement proofs** are about 14k lines today, excluding Funs.lean:

| Group | Effect |
|---|---|
| UF, Sort, SortU32, UFModel, Unify* (about 3.3k) | reusable, with the cell type changed |
| Memb, Ledger, Core (about 0.9k) | reworked |
| Split, SplitOk, Around, Between, Fan, Cells, Collapse (about 5.5k) | much simpler: membership in a list replaces a covector comparison |
| Encode, EndToEnd | restated over `LedgerF` |

Estimate: about 50–60% of the original effort, which was spread over parallel groups (SPLIT.md),
so the same split works.

**The bridge's builder** is the face-arrangement construction:
1. For each near pair of different classes, the exact segment `P_f ∩ P_g`: the line `meet` of two
   planes, clipped by both polygons' edge planes, as in EMBER's plane-based clipping.
2. For each face, the planar arrangement of its cut segments: a few segments per face, so a simple
   sweep or incremental insertion.
3. The radial sort at each edge, and the union–find of sides.

It lives in the bridge, written in the Aeneas style, and takes the planned `build_ledger` L4
route.

## 5. Measured cells (callgrind build, x86-64-v2, branch base 556b178 plus scratch instrumentation)

All figures are means per decide-once boolean. They were produced in a scratch worktree, not
committed. "FA pieces (est.)" is 1 plus the number of other-body faces crossing it, summed over near
faces, plus the coplanar overlaps. That is a rough upper estimate (a slit adds nothing, and crossing
segments can add more).

| Part | Booleans | Faces | Near | Classes (with walls) | Ledger cells | Pieces | Edges | Verts | Cross segments | Coplanar overlaps | FA pieces (est.) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| coplanar | 70 | 183 | 10.0 | 15.0 | 230.9 | 46.7 | 72.4 | 30.1 | 9.3 | 0.9 | 29.5 |
| housing | 1 | 12 | 7.0 | 13.0 | 103.0 | 19.0 | 28.0 | 12.0 | 4.0 | 0 | 15.0 |
| upto | 4 | 12 | 11.8 | 17.8 | 276.5 | 54.0 | 103.5 | 53.2 | 4.0 | 0 | 19.8 |

The other gate parts have no flat decide-once boolean: torture, plate, patterns, hole-grid, tour,
upto-flat, shaft, mixed, and the 15 bench parts.

The ledger shrinks least on housing (19 → 15 pieces) and most on upto (54 → 20). The arrangement
work shrinks most everywhere: from 9.5 crossing classes over whole sections, to 9 crossing segments
over the near faces.

## 6. Expected per-phase numbers (coplanar, per boolean; estimates)

| Phase | Decide-once now (bool-speed head) | A8 | How |
|---|---|---|---|
| arrangement | 3.46M | 0.4M | about 9 exact segments, each a two-plane meet clipped by about 8 edge planes, plus per-face insertion of 1–3 segments; vertices are `meet3` behind a filter |
| sample (witnesses) | 0.41M | 0.15M | one parity per side class, against about 10 near faces |
| ledger | 1.60M | 0.35M | about 150 cells, not 231; id tables, no `Vec<Sgn>` interning |
| core | 1.63M | 0.35M | O(degree) list lookups in place of 16-wide covector scans |
| write | 0.68M | 0.15M | **in place**, as the old path's `apply` (0.06M) does: edit the target's near faces only, and leave the rest untouched |
| rest | 0.42M | 0.25M | exact planes for near faces only (`inputs` converts all 183 today) |
| **total** | **8.2M** | **≈1.65M** | against the old path's **1.28M** |

Even with A8, decide-once is likely about 1.3× the old path per flat boolean. Two consequences:
- **The in-place write and the near-only `inputs` are needed, as well as the face arrangement.**
  Together they are about 0.8M of the 8.2M today, and the old path's whole boolean is 1.28M.
- **The last 0.4M has to come from constants.** For example: fuse sample into the builder, since
  the witness parity can reuse the builder's segment clipping, and avoid HashMaps in the bridge.

At coplanar seq level that is about +25M on 209M, +12%. The gate's 0.5% stays red on coplanar
until the constants close, while housing and upto are 1 and 4 booleans each.

## 7. Open questions for the Lean side

1. The interface route (§2): one core over `Inc`, or a second core? I recommend one.
2. The wedge lemma's statement, in terms of JordanLocal's `Clear`: for consecutive facets `φ`, `ψ`
   round edge `e`, and points `p` (just off `φ`) and `q` (just off `ψ`) within `δ` of a point of
   `e`, there is a polyline from `p` to `q` all of whose segments are `Clear` for every face.
3. Walls: today they are carriers with no face. Here they only clip polygons, so `wall_region` and
   `samplesIn_of` need a restatement for clipped faces.
4. The metric stage (stage 4b, `collapse`): thin side cells become thin side classes. Is the
   inradius test still per class?
