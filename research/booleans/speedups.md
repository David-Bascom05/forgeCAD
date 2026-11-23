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
