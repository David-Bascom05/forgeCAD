# Speed-ups for the flat boolean: not proven yet, but provable (2026-11-23)

The owner's directive: "explore ways in which we can speed up the flat bool that are not proven
yet, but should be provable."

This note is the survey and the ranking. Implementation goes to bool-speed
(session_01Q19LzxekVconSXig54WBmi) and decide-once Rust. Part I is today's shipped path, the old
`forge_bool::boolean`, because a win there reaches users at once. Part II is the decide-once A8
builder.

**The rule** (master, 2026-11-23): prototypes may go ahead unproven. Nothing merges to main or
switches on until one of two things holds:
- it is re-proven; or
- the shipped path's L6 evidence holds (fuzz, scoreboard, bits), and speed is at or below base.

## 0. Where the shipped path's flat booleans spend

This is measured here: exact callgrind, x86-64-v2, on coplanar seq at the branch base 556b178.
- Flat-only costs are switch-off minus switch-on per function. With the switch on, the old path
  runs only the 80 rod booleans.
- So "flat" means the 70 flat booleans on the old path.
- Inclusive costs. Recursive sorts are taken at their top frame (`driftsort_main`).

| Function (inclusive) | Flat, 70 booleans | Per flat boolean | Rods, 80 booleans |
|---|---|---|---|
| `boolean::general` (the whole boolean) | **84.7M** | **1.21M** | 115.8M |
| `plan_once_in` | 78.6M | 1.12M | 104.0M |
| `trim`: curves clipped to both faces | **38.4M** | 0.55M | 26.5M |
| ↳ `across` (a surface through each edge) | 7.5M | | |
| ↳ `trim::{closure#55}` | 7.3M | | |
| ↳ `curve_surface_in` (generic line ∩ surface) | 5.4M | | |
| ↳ `__rust_dealloc` | 3.2M | | |
| ↳ `crossings_near`, `edge_box`, `Curve::eval` | 3.9M | | |
| `split_face` | 11.3M | 0.16M | 20.8M |
| ↳ sort of `(VKey, usize)` | 2.7M | | |
| ↳ `uv_dir` | 1.7M | | |
| classification: `point_in_face_past` (from imprint, classify_specs, trim) | 10.9M | 0.16M | 15.3M |
| ↳ `planar_parity` (ray parity) | 6.4M | | |
| `imprint` | 7.5M | 0.11M | 15.8M |
| `classify_specs` | 4.4M | 0.06M | 9.8M |
| sort of `((VKey, VKey), usize)` (`by_ends`) | 1.9M | | 4.3M |
| `apply` (the write) | 3.4M | 0.05M | 9.3M |
| `edge_split` | 2.1M | | 6.2M |
| `Index::new` (classification index, both bodies, every boolean) | 1.1M | | 3.0M |

Over the whole line, both kinds of boolean:
- malloc, free and realloc internals are about 31M of 209M (15%);
- dhat counts about 1,150 blocks per boolean (bool-speed).

The headline: **45% of a flat boolean is `trim`.** Within it, the largest part is general-curve
machinery run on line-against-line cases. The reason is that `plane_meet` (boolean.rs:4032), the
fast path for a curve meeting an edge in a planar face, handles line × circle and circle × circle,
but **not line × line**. So every edge cut on a flat part goes through:
- `across`: build a plane through the edge;
- `curve_surface_in`: the generic line–surface intersection;
- `crossings_near`, and the tolerance closures.

## Part I: the shipped path (ranked)

The ranking is by expected gate win per unit of risk and proof. The win is on **coplanar seq**
(base 209.2M; 70 flat and 80 rod booleans). "Flat" wins also apply to upto (4 flat booleans) and
housing (1). Shared code, such as sorts and allocation, also speeds up the rods and other lines.

| # | Speed-up | Prior art | Est. win, coplanar seq | Bits / naming risk | Why provable | Proof size |
|---|---|---|---|---|---|---|
| **S1** | **Memoise `across` per boolean** (revised, see the correction below). `across(body, f, c)` is a pure function of (side, face, coedge), recomputed for every curve that reaches the edge. Keep it in `Scratch` keyed by (side, coedge), cleared per boolean. Also return `line_in_range`'s hits in an inline small array, not a `Vec<LineHit>` round trip. | EMBER and the octree BSPs (plane per edge stored once); Cherchi 2020 (per-triangle cached minors) | **−6 to −8M (−3 to −4%)**: `across` is 7.5M flat, plus rods; `IntoIter<LineHit>` is 0.7M | **None.** Same values, computed once. | **Memo of a pure function:** `memo_get` and `memo_sound` (Speedups.lean, proven), plus purity of `across` (it reads only `body`, which is immutable during `plan_once_in`) | S (~150 for the Aeneas side) |
| **S2** | **Allocation reuse in the hot loops.** Extend `Scratch` (boolean.rs:791, already holding `cuts`, `segs`, `pieces`, `specs`) to: trim's `ts`, `along`, `fenced` and `hits`; split_face's `half`, `by_start` and `dirs`; and imprint's `f64` buffers. Clear them; don't drop them. | Manifold's arrays over allocation; EMBER and trueform's per-thread arenas | **−8 to −12M (−4 to −6%)**, flat and rods. malloc is about 31M on the line, and trim's dealloc alone is 3.2M flat. | None. Values are identical; only capacity is kept. | A cleared `Vec` is the empty list in Aeneas's model, so `v.clear(); v.extend(xs)` refines `xs`. A one-line lemma per buffer. | S (~50) |
| **S3** | **Unstable sorts on packed keys.** `by_start.sort()` (split_face) and `by_ends.sort()` (pieces) become `sort_unstable_by_key` on `VKey::packed()` (u64) and a u128 pair. The second field is a distinct index, so stable and unstable sorts give the same list. | Standard: radix or pattern-defeating unstable sorts on integer keys | **−5 to −7M (−3%)**. The sorts are about 11.4M at their top frames over both kinds. Packed-key unstable sort is about 2–3× cheaper. | None, given distinct keys (asserted in debug). `packed` is order-preserving (boolean.rs:224). | **A sort is unique on a strict order:** two sorted permutations of a duplicate-free list are equal. Plus `packed` is monotone. ForgeBoolCore's `RefineSort` already has this style. | S (~100) |
| **S4** | **Classify once per region, not once per piece.** Pieces of one face that are joined without crossing the other body's surface share their inside/outside. Classify one piece per group by ray (`point_in_face_past`, `planar_parity`) and propagate the rest by union–find over shared uncut edges. | Cherchi 2022 (one ray per patch); Lévy 2024 (combinatorial propagation, one ray per component); Zhou 2016 (winding numbers over cells); decide-once's own flood | **−6 to −9M (−3 to −4%)**. Classification is about 15M flat inclusive (`point_in_face_past`, `planar_parity`, `Index::classify`). It drops to about one ray per group. | **Medium.** Exactly, the propagated answer equals the ray's. In floats, the old path's tolerant ray could differ on a near-degenerate piece. Such a difference is an inconsistency today, but it would still change bits. Run L6 (fuzz plus the bits gate) and log every piece where a sampled ray disagrees with the propagation. | Exact statement: **`JordanLocal.contains_const_local`** (proven): parity is constant along a segment that meets no face. Two pieces sharing an uncut edge are joined by such a segment. The float path needs the predicate filters' L5 bounds (`orient2d`/`dot3`) to hold at the piece's interior point. | M (~300 for the propagation lemma over the old path's piece graph) |
| **S5** | **Caches per body across a regen.** `Index::new` for the target, `edge_box`/`face_box` and `rebuild_shells`/`connected` are recomputed every boolean over all ~183 faces. Keep them on the body, and patch only the faces `apply` touched. | Manifold's lazy CSG tree (results reused); EMBER's octree (only changed cells rebuilt); SolidWorks' local update (US 8,305,376) | **−5 to −8M (−3%)**. `Index::new` is 4.1M, `edge_box` 5.0M, `face_box` 1.6M, `rebuild_shells` 4.6M and `connected` 3.8M on the line, about 19M, of which about half is avoidable. | Low. The cache must be invalidated exactly where `apply` edits, and a stale entry is a silent wrong answer, so debug builds recompute and compare. | **The index is a pure function of the body:** patching it after an edit equals rebuilding it. The invariant is `idx = Index::new(body)` after every `apply`. | M (~250) |
| S6 | `uv_dir` for planes: a plane's (u, v) direction is a projection onto the frame, without the generic `Surface::project`/`normal` | — | −1 to −2M | Low (same expressions, as S1) | expression identity, as S1 | S |
| S7 | `std::thread` over face pairs | EMBER's work stealing; Manifold's `autoPolicy` | **0 on the gate:** it counts instructions, and threads add them. At about 1.2M instructions (≈0.3 ms) per boolean, spawn and join cost about as much as they save. Manifold runs serially below 4,096 elements for this reason. | — | — | not recommended |
| S8 | Integer or fixed-point plane coordinates for flat parts (EMBER, trueform) | EMBER (26-bit input, 256-bit predicates); trueform (int32/int64 grids) | — | **Changes geometry.** Snapping vertices to a grid changes results that work today. | — | rejected for the shipped path |
| S9 | BVH or sort-and-sweep candidate pruning | Manifold's BVH; Cherchi's octree | about 0 at our size. The face-pair loop is about 10 near × the tool's faces with a box test, and `trim` already skips edges by box. | — | — | not needed below about 1k faces |

**The first four together: about −25 to −33M on coplanar seq (−12 to −16%)** (revised). On a flat
boolean, 1.21M → about 0.85M, if S1–S4 land as estimated.

**Correction to the first version of S1 (2026-11-23).** A deeper split shows three things:
- `curve_surface_in` already sends a line straight to `line_in_range` → `line_surface` →
  `line_plane`. Its 5.4M is volume, not dispatch.
- `trim::{closure#55}` (7.3M flat) is `point_in_face_past` on each hit, so it belongs to S4's
  pool. S4's pool is about 22M flat inclusive.
- A `(Line, Line)` arm in `plane_meet` would **not** be bit-safe. That branch filters hits by
  `on_edge` plus the other face's box and `point_in_face`, while the general branch uses `near`,
  the decided logs and `crossings_near`.

S1 is therefore the `across` memo above. Bool-speed was told before implementing.

## Part II: the decide-once A8 builder (ranked)

These are the six rules of a8-review.md §6, ranked here as speed-ups, plus two new ones. The
reference throughout is the Lean coordinator's design, face-arrangement.md on decide-once-lean
@ 2e732a5f. The budget is 0.92M per flat boolean, against the old path's 1.16M.

| # | Speed-up | Prior art | Est. win per flat boolean (from today's 8.2M) | Bits / naming | Why provable | Size |
|---|---|---|---|---|---|---|
| D1 | Face arrangement in place of the plane arrangement (A8 itself) | Zhou 2016; Cherchi 2020/22; EMBER; trueform | 3.46M → about 0.25M | the design's naming rule (least face id), checked on the gate | `Refines` plus `core_refines` (design §2.2) | L (designed) |
| D2 | Covector-free vertices: a carrier-triple key plus a filtered f64 point, built exact on failure (about 2% today) | Attene 2020 (indirect predicates); Guo & Fu 2024; Shewchuk 1997 (semi-static filters) | −0.5M (1.3k → about 0.35k per vertex) | none: the decided signs are equal | an L5 filter bound for `n·X − cW` with X, W the 3×3 minors, against verify/ieee754, as orient3d | M |
| D3 | One reused arena per boolean; no HashMap in the bridge (dense tables keyed by near-class id) | trueform; EMBER; Manifold | −1.5 to −2M (malloc ~15%; Fx hashing of `Vec<Sgn>` 16M per run) | none | value-identical refactor, inside `build_ledger`'s L4 | S |
| D4 | Exact planes and points for near faces only (R6) | EMBER (only polygons in a leaf are touched) | −0.3M (inputs converts all 183 faces) | none | `far_unchanged` (Frontier.lean): far faces need no decision | S |
| D5 | The in-place write (R5), as the old `apply` (0.05M) | OCCT General Fuse (split only faces with interferences); trueform (reuses untouched links) | −0.5M (0.68M → about 0.15M) | the naming rule; bits gate | `far_unchanged`, plus the write's identity on far faces (a8-review.md §3.2) | S–M |
| D6 | Witnesses fused into the builder, orientation shortcut first | Lévy (one ray per component); `witness_inside`'s own shortcut | −0.3M | none | `boundary` (the shortcut) plus `room_const` | S |
| D7 (new) | **A per-regen ledger cache for unchanged tools.** In a pattern, the tool is translated, and the ledger of a translated pair is the translated ledger. On upto's and coplanar's repeated tools, reuse the combinatorial core's outcome when the pair's near ledger is equal after translation. | Manifold's lazy CSG reuse; trueform's build once, query many | coplanar has 70 similar booleans (DOSTATS rows repeat exactly: 169 cells, 36 pieces, 47 edges), so up to −1.6M per repeated boolean on the core | names are relative to faces, so the mapping must carry names | **The core is a function of the ledger:** equal ledgers give equal outcomes (`core` is pure; `end_to_end_proof` is per ledger). Needs only ledger equality, decided exactly. | S |
| D8 (new) | Integer plane coordinates **inside the builder only** (no input change): clear the dyadic exponents per region to fixed-width integers, with a `Big` fallback by an exact width check | Nehring-Wirxel 2021, Table 1; Fortune 1997 | small (exact work is about 2%) | none | `mul_fits`/`add_fits` (research/booleans/lean/Sparse.lean, proven) | M; low priority |

## Top 4 for prototypes (behind feature flags on bool-speed's branch)

Each is measured on the exact gate (tools/speed/exact.sh: callgrind plus dhat; bits.txt) before
any proof work.

1. **S1, feature `across_memo`** (revised; the plan below is the superseded `flat_line_meet`, kept
   for the record and not to be built). Add the `(Line, Line)` arm to `plane_meet`, inlining exactly
   the float operations `across` and `curve_surface_in` perform for a line edge in a planar face.
   - The test: a debug assertion that compares the fast hit with the general path's on every call.
   - Pass: bits identical on the whole gate, coplanar seq −15M or better, no line worse.
   - If the assertion ever fires, the inlining is not literal; fix it, and don't loosen the
     comparison.
2. **S2, feature `scratch_trim`.** Move trim's and split_face's per-call `Vec`s into `Scratch`.
   - Pass: bits identical; dhat blocks per boolean down at least 30% on coplanar; −8M.
3. **S3, feature `packed_sorts`.** Packed-key `sort_unstable` for `by_start` and `by_ends`.
   (`nary.rs:2928` already sorts unstably.)
   - Debug assertion: the keys are distinct.
   - Pass: bits identical; −5M.
4. **S4, feature `piece_flood`.** Union–find over the pieces of each face across uncut edges, and
   one `point_in_face_past` per group.
   - Debug mode classifies every piece both ways and logs disagreements, with the pair and the
     point.
   - Pass: zero disagreements on the gate, fuzz (forge-exact-fuzz) and the repros; bits identical;
     −6M.
   - Disagreements are the useful output. Each is a place the old path's tolerant classification
     is inconsistent.

D7 is the fifth candidate, on the A8 branch, once A8's core exists. Measure first how many of
coplanar's 70 ledgers are equal after translation.

## Risks to bits and naming

- S1 and S6 change *how* a float is computed. The answer is safe only if the operations are
  literally the same, which the debug assertion enforces.
- S4 can change bits wherever the old path was inconsistent. It ships only with zero
  disagreements on the evidence corpora.
- S5's cache must be exact. Debug builds recompute and compare after every `apply`.
- S2 and S3 cannot change bits, given distinct keys.
- No item above changes names, except the A8 naming rule, which the design already gates.
- The gate counts instructions:u. S7 (threads) can only add instructions, so it is excluded from
  the ranking, not merely ranked low.


## Proof statements for S1–S3 (ready to prove at L4)

The abstract facts are proven in `research/booleans/lean/Speedups.lean`. That file is core Lean on
the forge-exact toolchain v4.36.0-rc2. It compiles with no `sorry`, using only `propext` and
`Quot.sound`. Each change's L4 obligation is its Aeneas translation refining the unchanged function,
citing these facts.

| | Rust change | Abstract fact (proven) | L4 obligation, for the Aeneas session |
|---|---|---|---|
| **S1** `across_memo` | `trim` asks `Scratch.across.get_or(side, coedge, ‖ across(body, f, c))` | `memo_get`: on a sound table the memoised call returns `f k`. `memo_sound`: it keeps the table sound. `memo_nil`: the per-boolean empty table is sound. | (i) `across` is pure: its translation reads only `body`, and `body` is not mutated within `plan_once_in`. (ii) `trim_memo` with a sound table equals `trim`, by `memo_get` at each call. (iii) The table is cleared per boolean (`memo_nil`). The key must include the side: coedge ids of the two bodies collide. |
| **S2** `scratch_trim` | `let ts = &mut scratch.ts; ts.clear(); …` in place of `let mut ts = Vec::new()` | `cleared_fill`: in Aeneas's model a cleared `Vec` is `[]` (capacity is not modelled), so filling it gives the fresh result | Each buffer is cleared at the start of each use, and no reference to it outlives the use (Aeneas's borrow model checks the second). One lemma per buffer: the state after `clear` is the `Vec::new()` state. |
| **S3** `packed_sorts` | `by_start.sort_unstable_by_key(\|x\| (x.0.packed() as u128) << 64 \| x.1 as u128)`, and the same for `by_ends` with a 192-bit or two-word key | `sorted_unique`: two sorted permutations of a list are equal (strict order). `packed_mono`: `VKey::packed` is strictly monotone within the widths (side < 2, ids < 2³²). `sort_packed_eq`: a `lt`-sorted list maps to a `<`-sorted packed list. | (i) The pairs are distinct: the second field is the index `j`, which is unique. (ii) `sort_unstable`'s result is sorted and a permutation. That is std's contract; take it as an axiom of the std model, as FunsExternal does for others, or prove it on a translated sort, as `RefineSort` does. (iii) So both sorts give the one sorted order (`sorted_unique`), and the result is bit-identical. Debug builds assert the width bounds of `packed_mono`. |

S4 is not in this table: its fact is `JordanLocal.contains_const_local`, which is already proven.
Its L4 needs the old path's piece graph modelled. That is M-sized, and waits until S4's prototype
shows zero disagreements.

## Threads: wall-clock speed for the user (2026-11-23)

The owner, relayed by the master: "the goal is speed for the user, not just small binaries".
Threads are therefore ranked here by **wall-clock** win. The instruction gate stays as it is, and a
new session (session_01Q19LzzwaZhmXMt4M3NiseR) is building a wall-clock user-latency gate beside
it. The earlier S7 ("0 on the gate") is about the instruction gate only.

### Measured on this machine (4 vCPUs, a VM; release build x86-64-v2)

**Thread overhead** (std only, a small benchmark in the scratchpad):

| Operation | Cost |
|---|---|
| `thread::spawn` + `join`, 1 thread | 42–50 µs |
| `thread::scope` with 4 threads | about 105 µs |
| a Mutex/Condvar pool waking 4 workers and waiting | about 110 µs (the VM's wake latency, about 25 µs per thread) |

A desktop is typically 2–5× cheaper. The latency gate should re-measure on the target hardware.

**Throughput:** coplanar seq runs 209M instructions in about 28 ms, so about 7.5G instructions/s
on one core. A flat boolean (1.21M) is therefore about **0.16 ms**.

**Where a regen's instructions are** (callgrind, switch off):

| Part | Booleans' share | Tools' share |
|---|---|---|
| shaft | 98% | 2% |
| mixed | 99% | 1% |
| coplanar | 96% | 4% |
| upto | **61%** | **39%** |

**The existing parallel n-ary regen, wall-clock** (best of 5; `regen1 PART nary1` against
`nary16`):

| Part | nary1 | nary16 | Change |
|---|---|---|---|
| coplanar | 27.1 ms | 45.1 ms | **+66%** |
| upto | 7.1 ms | 8.4 ms | **+18%** |
| heatsink | 56.6 ms | 55.7 ms | −2% |
| shaft | 67.6 ms | 50.4 ms | −25% |
| mixed | 229.6 ms | 99.9 ms | **−56%** |

`nary16` asks for 16 threads on 4 vCPUs. `forge-bool/src/nary.rs`'s `par_map` already starts
helpers only after 200 µs of serial work. But `forge-model/src/lib.rs:1126` (tool building) and
`bodies.rs:1052` spawn as soon as `threads > 1`, and nothing caps the count at the cores there are.

### The break-even rule

For a batch of serial work W, split over p threads at fixed overhead O, the parallel time is
W/p + O:
- **Break-even** is at W = O·p/(p−1). With O ≈ 105 µs and p = 4, that is W ≈ 140 µs, about **1M
  instructions**.
- **A 25% saving** of W needs W ≥ 2·O ≈ 210 µs, about **1.6M instructions**.
- **Items:** each should be at least 5–10 µs (about 50k instructions), so that the shared atomic
  counter and the result pushes stay under 5%.

What that means at our sizes:
- **A typical flat boolean** (about 0.16 ms, with the face-pair loop about 70% of it) is below
  break-even. Splitting its pairs gains nothing.
- **A large boolean** gains about (1 − 1/p) of its pair phase. That is one of 5M instructions or
  more (about 0.7 ms): coplanar's largest, with 32 near faces against a median of 10, or the
  curved booleans on mixed.
- **Across a regen:** the n-ary batch (many tools at once) is already large enough on mixed and
  shaft. It is too small on coplanar and upto, where the threads cost more than they save.

### How results stay independent of the thread count

The repository already has the pattern, in forge-math `par::map` and nary's `par_map`:
- workers take items from a shared counter;
- each answer is tagged with its item index;
- the answers are sorted by index at the end;
- everything after that runs in item order.

The proof:
- **`par_map_eq`** (Speedups.lean, proven, core Lean, standard axioms). The collected `(i, f i)`
  list is a permutation of the sequential list. Once sorted by index, it *is* the sequential list,
  whatever the scheduling or the thread count. This rests on `keyed_unique`.
- The precondition is that `f` is pure: it reads shared state only immutably and writes only its
  own answer.

The rules for every threaded site:
1. No floating-point reduction across items. Sums are taken after the sort, in item order.
2. No ids or names allocated inside a worker. Names come from the inputs (as the boolean's pair
   names already do, from face names) or are assigned after the merge.
3. Chunking never decides results. Items are the unit, not thread-sized chunks.
4. A panic in a worker resumes on the caller (`resume_unwind`, as par.rs does).

### Ranked by expected wall-clock win

| # | Item | Win (wall-clock) | Bits / naming | Proof | Size |
|---|---|---|---|---|---|
| **W1** | **Make the existing regen-level threading adaptive and capped.** Cap the thread count at the cores there are. Start serially, and add helpers only once more than about 200 µs of work remains, as nary's `par_map` already does, at `lib.rs:1126` and `bodies.rs:1052`. (`ssi.rs:524` is test code.) | coplanar n-ary **45 → about 27 ms (−40%)**; upto 8.4 → about 7.1 ms (−15%); mixed keeps its −56% | none: answers already in item order; serial and parallel give one result | `par_map_eq` (proven) plus purity of the item function | S |
| **W2** | S1–S4 above (single-thread instruction cuts) | about −12 to −16% on coplanar seq wall-clock, 1:1 with instructions | as listed per item | as listed | S–M |
| **W3** | **Build tools ahead of the boolean chain in the seq regen.** While boolean k runs, a helper builds tools k+1 and beyond, in item order. | upto: tools are 39% of instructions, so about −30% latency on upto; coplanar, shaft and mixed about −1 to −4% | none: tools are pure functions of their features | `par_map_eq`. Each tool depends only on the part's parameters, not on the previous boolean's result. Features that read the body (up-to-face, holes) are excluded by the same `modifies` test the n-ary regen already uses. | S–M |
| **W4** | **Face-pair batches inside one boolean,** adaptive (no helpers below about 1M instructions of pair work). Each pair's `trim` writes into its own buffers (cuts, segs, imprints, touching), concatenated in pair order with index remapping. | typical flat boolean: 0. Large booleans (≥ 5M instructions, mostly curved; coplanar's 32-near ones): up to about 2× on their pair phase with 4 cores. | medium: `trim` today pushes into shared `cuts`/`segs`, with cut indices held in `ts`. The concatenation must remap indices exactly; `rank_pairs` runs after, unchanged. | `par_map_eq`, plus "concat of per-pair outputs = the sequential fold" (an S list lemma), plus purity of `trim` per pair (it reads `bodies` and `index`, and writes only its own outputs once restructured) | M |
| W5 | S5 (per-body caches) | −3% | low | as listed | M |
| W6 | Independent bodies or assembly instances in parallel | already done in forge-asm (`solve.rs`, `check.rs`) | — | — | — |
| — | Threads inside the decide-once/A8 builder | none at about 0.9M per boolean, below break-even | — | — | not recommended |

**Recommended order for the user:** W1 (small, certain, and it removes a regression of the
threaded path), then S1–S4 (W2), then W3 (it decides upto-like parts), then W4 for large
booleans. W1 and W3 should be measured first on the latency gate. The figures here are from a
4-vCPU VM.

### W1 done (2026-11-23, on this branch)

**The change:**
- `forge_math::par::cap`: the requested threads, capped at `par::threads()` (the cores there are,
  or `FORGECAD_THREADS`).
- `forge_math::par::map_adaptive`: nary's `par_map`, moved into forge-math so it can be shared.
  The caller works alone, and helpers start only after `HELPERS_AFTER` = 200 µs, with items left.
- The regen's two tool builders (`forge-model` lib.rs `build_each` and bodies.rs `build_each`) use
  `map_adaptive`. The first spawned at once; the second used fixed chunks. Both now work item by
  item, in item order.
- Every n-ary entry point goes through `wasm_one`, which now caps the thread count.
- nary's `par_map` is a thin call to the shared function.

**Bits:**
- `bits` (54 kernel lines) and `benchparts bits` (30 lines) are identical to tools/speed/bits.txt
  at `FORGECAD_THREADS` = 1, 2, 4 and 16.
- `cargo test --release` passes: forge-math, forge-model (lib), and forge-lang's
  `kernel_determinism`, which compares one core, threads and the batch.

**Wall-clock** (µs, best of 7; 11 for heatsink; `regen1 PART nary1/nary16`, this 4-vCPU VM):

| Part | Base nary1 | Base nary16 | W1 nary1 | W1 nary16 | nary16 change |
|---|---|---|---|---|---|
| coplanar | 24,373 | 32,863 | 24,978 | **22,123** | **−33%**, now faster than nary1 |
| upto | 6,512 | 7,088 | 6,370 | **5,874** | **−17%** |
| heatsink | 52,807 | 53,614 | 53,088 | 53,299 | 0% (noise) |
| shaft | 63,673 | 41,907 | 67,586 | **34,983** | **−17%** |
| mixed | 207,366 | 95,708 | 212,618 | **88,894** | **−7%** |

nary1 is unchanged within noise: with one thread, `map_adaptive` is the serial map. The
instruction gate's nary1 lines therefore run the same work.

**Equal-thread comparison**, as the owner rules (µs, best of 15 runs, base and head
interleaved):

| Part | nary1 base | nary1 head | nary16 base | nary16 head |
|---|---|---|---|---|
| coplanar | 23,731 | 23,987 (noise) | 32,863 | 22,123 (−33%) |
| shaft | 65,158 | 64,974 (noise) | 41,907 | 34,983 (−17%) |
| mixed | 215,076 | 212,939 (noise) | 95,708 | 88,894 (−7%) |

The nary16 columns are best of 7, from the table above. upto nary16: 7,088 → 5,874 (−17%).

**Correctness gates before handoff** (head 6c20812; base is its parent, 70d6aeb):

| Gate | Result |
|---|---|
| reliability scoreboard | against tools/reliability/baseline.txt: **0 worse**, 3 better. Base shows the same 3, so that file is stale. Base and head verdicts are identical (985 lines). |
| suite with `FUZZ_SEEDS=150` | 150/150 histories of 80 features regenerate valid; every suite part ok |
| n-ary with `FUZZ_SEEDS=150` | agree **167/167** (same faces, volume within 0.1%), 4 threads |
| wasm bits | the wasm32-wasip1 `bits` under Node is identical to native (54 lines) |
| bits at `FORGECAD_THREADS` = 1, 2, 4 and 16 | 84/84 lines identical to tools/speed/bits.txt |

## Fastest wins: t1 against tN (2026-11-23)

The owner's rule is that the user gets the faster of threaded and single-threaded on every path.
The master's instruction was to compare, per part and per op, default threads (tN, here 4) against
`FORGECAD_THREADS=1` (t1), on one build, and keep a path serial wherever one thread is faster.

### What was slower, and why

| Cause | Lines | Fix |
|---|---|---|
| fillet's own `par_map` spawned a thread per core at once and ignored `FORGECAD_THREADS` | every part with a fillet: clamp, insert, heatsink, … (8–16 threads each) | it calls `forge_math::par::map` |
| `forge_math::par::map` spawned every thread at once (used by ops and forge-lang) | all ops users | it is `map_adaptive` |
| forge-lang's measure spawned threads for mass and box at once, even with `FORGECAD_THREADS=1` | every `cli` run (hub −9%) | items of one `map_adaptive`; serial at one thread |
| helpers started at 200 µs with any work left, one per core | small n-ary regens | helpers only when the work left, projected from the items done, exceeds `HELPERS_FOR` = 2 ms, and **one helper per 2 ms** of it |
| a new thread starts with cold per-thread caches. `ssi::MET` keeps searched curves per thread, so helpers search again what the caller knows. | jack n-ary +14% | the n-ary pair loop has its own threshold, `PAIR_HELPERS_FOR` = 5 ms |
| the n-ary edge split took its per-edge-list form whenever `threads > 1`, even when no helper came | hole-grid n-ary | the threaded form needs at least 512 edges per thread |
| `map_adaptive` tagged and re-sorted every answer even when no helper joined | all | alone, the caller writes answers straight into place; only items after helpers start are tagged |
| `par::threads()` called `available_parallelism` (cgroup file reads on Linux) at every map | all, small | the core count is cached; `FORGECAD_THREADS` is still read every time |

### The table

Commit `fc9…`, this branch: the line after "W1 fastest-wins" in the log. Median of 11 paired rounds
on a 4-vCPU VM; each round runs t1, then tN, then tN again. A/A is the second tN over the first: it
is this VM's noise, and it reaches ±10% on runs under 15 ms. "Helpers" is the number of threads
the tN run started (strace `clone3`).

| Line | t1 ms | tN ms | tN/t1 | A/A | Helpers | Behaviour |
|---|---|---|---|---|---|---|
| regen1 torture seq | 23.0 | 23.3 | 1.028 | 1.133 | 0 | serial (no helper paid) |
| regen1 torture nary16 | 21.5 | 20.7 | 0.953 | 1.044 | 6 | threaded |
| regen1 plate seq | 21.9 | 21.8 | 1.002 | 0.982 | 0 | serial (no helper paid) |
| regen1 plate nary16 | 21.6 | 21.3 | 0.978 | 0.979 | 7 | threaded |
| regen1 shaft seq | 82.5 | 80.8 | 0.982 | 0.940 | 0 | serial (no helper paid) |
| regen1 shaft nary16 | 68.5 | 47.7 | 0.704 | 0.960 | 16 | threaded |
| regen1 housing seq | 22.4 | 21.7 | 0.969 | 1.059 | 0 | serial (no helper paid) |
| regen1 housing nary16 | 26.6 | 24.8 | 0.951 | 1.000 | 7 | threaded |
| regen1 coplanar seq | 30.5 | 30.0 | 0.988 | 0.990 | 0 | serial (no helper paid) |
| regen1 coplanar nary16 | 23.9 | 25.1 | 1.052 | 0.989 | 6 | threaded, **slower: open** |
| regen1 patterns seq | 26.4 | 26.4 | 1.005 | 0.974 | 0 | serial (no helper paid) |
| regen1 patterns nary16 | 20.1 | 22.7 | 1.086 | 0.996 | 7 | threaded, **slower: open** |
| regen1 mixed seq | 256.7 | 254.0 | 1.006 | 1.020 | 0 | serial (no helper paid) |
| regen1 mixed nary16 | 224.6 | 116.5 | 0.512 | 0.980 | 20 | threaded |
| regen1 hole-grid seq | 18.0 | 18.1 | 1.046 | 1.010 | 0 | serial (no helper paid) |
| regen1 hole-grid nary16 | 16.1 | 18.8 | 1.232 | 0.977 | 7 | threaded, **slower: open** |
| regen1 tour seq | 3.2 | 3.2 | 1.036 | 0.983 | 0 | serial (no helper paid) |
| regen1 tour nary16 | 2.8 | 2.9 | 1.048 | 1.007 | 0 | serial (no helper paid) |
| regen1 upto seq | 7.3 | 7.3 | 1.006 | 0.990 | 0 | serial (no helper paid) |
| regen1 upto nary16 | 5.7 | 5.6 | 0.993 | 0.992 | 0 | serial (no helper paid) |
| regen1 upto-flat seq | 3.9 | 4.0 | 0.991 | 0.953 | 0 | serial (no helper paid) |
| regen1 upto-flat nary16 | 3.3 | 3.4 | 1.016 | 1.016 | 0 | serial (no helper paid) |
| benchparts cli gearbox | 83.0 | 71.5 | 0.849 | 0.999 | 3 | threaded |
| benchparts edit gearbox | 97.7 | 96.5 | 0.969 | 1.073 | 0 | serial (no helper paid) |
| benchparts run gearbox seq | 77.4 | 86.3 | 1.067 | 0.966 | 0 | serial (no helper paid) |
| benchparts run gearbox nary4 | 99.4 | 84.9 | 0.899 | 0.999 | 6 | threaded |
| benchparts cli volute | 200.8 | 164.6 | 0.873 | 0.963 | 5 | threaded |
| benchparts edit volute | 239.0 | 236.7 | 0.973 | 1.017 | 0 | serial (no helper paid) |
| benchparts run volute seq | 182.8 | 171.1 | 0.929 | 1.045 | 0 | serial (no helper paid) |
| benchparts run volute nary4 | 178.1 | 177.4 | 1.006 | 0.999 | 0 | serial (no helper paid) |
| benchparts cli crank | 71.5 | 64.3 | 1.004 | 1.001 | 5 | threaded |
| benchparts edit crank | 70.0 | 69.2 | 0.997 | 0.992 | 0 | serial (no helper paid) |
| benchparts run crank seq | 61.3 | 63.1 | 1.013 | 1.009 | 0 | serial (no helper paid) |
| benchparts run crank nary4 | 66.8 | 66.8 | 0.947 | 0.963 | 5 | threaded |
| benchparts cli valve | 79.7 | 54.6 | 0.636 | 1.028 | 5 | threaded |
| benchparts edit valve | 43.5 | 50.4 | 1.106 | 0.961 | 0 | serial (no helper paid) |
| benchparts run valve seq | 38.1 | 38.7 | 1.017 | 1.014 | 0 | serial (no helper paid) |
| benchparts run valve nary4 | 39.2 | 38.5 | 0.979 | 0.983 | 2 | threaded |
| benchparts cli heatsink | 27.7 | 28.5 | 0.941 | 0.990 | 5 | threaded |
| benchparts edit heatsink | 33.3 | 34.0 | 0.996 | 0.981 | 0 | serial (no helper paid) |
| benchparts run heatsink seq | 28.4 | 29.2 | 1.025 | 0.985 | 0 | serial (no helper paid) |
| benchparts run heatsink nary4 | 27.7 | 27.9 | 0.985 | 0.981 | 1 | threaded |
| benchparts cli quadarm | 15.1 | 16.2 | 1.012 | 0.950 | 0 | serial (no helper paid) |
| benchparts edit quadarm | 22.0 | 20.3 | 0.948 | 1.050 | 0 | serial (no helper paid) |
| benchparts run quadarm seq | 18.1 | 17.7 | 0.954 | 1.123 | 0 | serial (no helper paid) |
| benchparts run quadarm nary4 | 17.9 | 18.1 | 0.997 | 0.988 | 0 | serial (no helper paid) |
| benchparts cli conrod | 29.9 | 24.9 | 0.804 | 1.016 | 3 | threaded |
| benchparts edit conrod | 24.0 | 25.4 | 0.933 | 1.321 | 0 | serial (no helper paid) |
| benchparts run conrod seq | 23.7 | 24.9 | 1.044 | 0.982 | 0 | serial (no helper paid) |
| benchparts run conrod nary4 | 19.2 | 19.6 | 1.023 | 1.010 | 0 | serial (no helper paid) |
| benchparts cli jack | 88.4 | 70.4 | 0.790 | 1.000 | 4 | threaded |
| benchparts edit jack | 77.2 | 76.7 | 1.015 | 1.010 | 0 | serial (no helper paid) |
| benchparts run jack seq | 71.7 | 72.3 | 1.000 | 0.986 | 0 | serial (no helper paid) |
| benchparts run jack nary4 | 69.2 | 62.4 | 0.917 | 1.006 | 2 | threaded |
| benchparts cli frame | 121.8 | 94.5 | 0.759 | 1.008 | 5 | threaded |
| benchparts edit frame | 124.1 | 118.6 | 0.978 | 1.029 | 0 | serial (no helper paid) |
| benchparts run frame seq | 131.4 | 123.8 | 1.000 | 1.030 | 0 | serial (no helper paid) |
| benchparts run frame nary4 | 121.6 | 106.4 | 0.860 | 0.951 | 2 | threaded |
| benchparts cli manifold | 60.5 | 56.6 | 0.946 | 0.994 | 6 | threaded |
| benchparts edit manifold | 97.8 | 93.7 | 0.994 | 0.981 | 2 | threaded |
| benchparts run manifold seq | 65.0 | 68.2 | 0.986 | 0.976 | 2 | threaded |
| benchparts run manifold nary4 | 67.1 | 64.5 | 0.950 | 1.069 | 2 | threaded |
| benchparts cli bracket | 6.2 | 5.9 | 0.979 | 1.104 | 0 | serial (no helper paid) |
| benchparts edit bracket | 9.1 | 7.4 | 0.820 | 1.291 | 0 | serial (no helper paid) |
| benchparts run bracket seq | 6.3 | 6.5 | 1.016 | 0.970 | 0 | serial (no helper paid) |
| benchparts run bracket nary4 | 7.2 | 7.2 | 1.132 | 0.927 | 0 | serial (no helper paid) |
| benchparts cli insert | 10.0 | 9.9 | 0.906 | 1.076 | 0 | serial (no helper paid) |
| benchparts edit insert | 15.4 | 13.6 | 0.934 | 1.070 | 0 | serial (no helper paid) |
| benchparts run insert seq | 13.5 | 13.4 | 0.987 | 1.045 | 0 | serial (no helper paid) |
| benchparts run insert nary4 | 10.9 | 12.5 | 1.043 | 1.056 | 0 | serial (no helper paid) |
| benchparts cli clamp | 9.9 | 10.1 | 1.034 | 1.002 | 0 | serial (no helper paid) |
| benchparts edit clamp | 9.8 | 11.0 | 1.061 | 1.047 | 0 | serial (no helper paid) |
| benchparts run clamp seq | 7.2 | 7.6 | 1.037 | 0.974 | 0 | serial (no helper paid) |
| benchparts run clamp nary4 | 7.2 | 10.1 | 1.060 | 0.991 | 0 | serial (no helper paid) |
| benchparts cli hub | 8.0 | 11.9 | 1.364 | 0.968 | 0 | serial (no helper paid) |
| benchparts edit hub | 12.0 | 11.1 | 1.000 | 1.018 | 0 | serial (no helper paid) |
| benchparts run hub seq | 10.2 | 10.1 | 1.058 | 0.896 | 0 | serial (no helper paid) |
| benchparts run hub nary4 | 9.5 | 9.7 | 1.085 | 1.298 | 0 | serial (no helper paid) |
| benchparts cli guard | 14.8 | 15.3 | 1.016 | 1.007 | 0 | serial (no helper paid) |
| benchparts edit guard | 21.8 | 21.6 | 0.992 | 1.012 | 0 | serial (no helper paid) |
| benchparts run guard seq | 20.2 | 20.3 | 1.013 | 0.975 | 0 | serial (no helper paid) |
| benchparts run guard nary4 | 24.8 | 25.4 | 1.013 | 0.980 | 0 | serial (no helper paid) |

### Reading the table

- **Every line that starts helpers is faster or level, except three small synthetic n-ary
  regens:** coplanar (1.05), patterns (1.09) and hole-grid (1.23), each about 16–25 ms with 6–7
  helpers.
  - All three are all-planar: their items are cheap, so helpers mostly add allocator and cache
    contention.
  - Per-site experiments (each site forced serial in turn) could not separate a cause from this
    VM's drift. Even all sites serial measured 1.03, so I stopped tuning on this VM.
  - **These three are left open for the latency gate's paired statistics** (99% CI, Wilcoxon).
    The candidate rule, if they confirm: helpers only where the mean item costs over about 20 µs,
    since cheap items are bound by memory and the allocator. Planar n-ary items are cheap;
    curved SSI items are not.
- Lines with no helpers (t1 and tN run the same serial work) scatter from 0.82 to 1.36 against an
  A/A that scatters from 0.90 to 1.32. That is noise, not threads. hub `cli` at 1.364 has A/A 0.968
  and no helper, and was 1.02 in an earlier paired run of 15 rounds.
- **The wins:**
  - `cli`: valve 0.64, jack 0.79, frame 0.76, conrod 0.80, gearbox 0.85, volute 0.87.
  - n-ary: shaft 0.70, mixed 0.51, frame 0.86, gearbox 0.90, jack 0.92.

**Gates on this commit:**
- bits: 84/84 identical at `FORGECAD_THREADS` = 1, 2, 4 and 16;
- wasm bits: identical;
- reliability: verdicts identical to base (985 lines);
- `FUZZ_SEEDS=150` suite: 150/150;
- n-ary: 167/167;
- tests pass: forge-math, forge-model lib, forge-lang `kernel_determinism`, forge-ops fillet;
- clippy `-D warnings`: clean on the five crates.
