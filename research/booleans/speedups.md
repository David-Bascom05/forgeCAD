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
| **S1** | **A line × line case in `plane_meet`.** A planar face's line edge against the cut line: one 2D solve in the face plane, with the same tolerance tests, and no `across` surface, generic `curve_surface_in`, `crossings_near` or allocation. | EMBER and the octree BSPs (Trettner et al. 2021/22) clip lines against polygon edge planes directly; Cherchi 2020 has per-triangle cached minors | **−15 to −20M (−7 to −10%)**. Of trim's 38.4M flat, `across`, `curve_surface_in`, closure#55 and `crossings_near` are about 22M. A 2D solve costs about 0.2k per edge, against about 2–3k now. | **Real.** The result must equal the general path's hit (`t`, point, contact) to the bit. Do it by evaluating *the same* float expressions in the same order: the `across` plane's normal from the same `cross`, then `line_plane`'s formula, inlined. Not a "nicer" 2D solve. Names are untouched (cuts are named by face pairs). | **Refinement by expression identity.** On `(Line, Line)` in a planar face, the fast path's float term is syntactically the general path's term after inlining `across` and `line_plane`. This is an Aeneas-level equality, with floats as opaque operations. Needs the aeneas session's float bridge to state; no real-number reasoning. | S (~200–300 lines) |
| **S2** | **Allocation reuse in the hot loops.** Extend `Scratch` (boolean.rs:791, already holding `cuts`, `segs`, `pieces`, `specs`) to: trim's `ts`, `along`, `fenced` and `hits`; split_face's `half`, `by_start` and `dirs`; and imprint's `f64` buffers. Clear them; don't drop them. | Manifold's arrays over allocation; EMBER and trueform's per-thread arenas | **−8 to −12M (−4 to −6%)**, flat and rods. malloc is about 31M on the line, and trim's dealloc alone is 3.2M flat. | None. Values are identical; only capacity is kept. | A cleared `Vec` is the empty list in Aeneas's model, so `v.clear(); v.extend(xs)` refines `xs`. A one-line lemma per buffer. | S (~50) |
| **S3** | **Unstable sorts on packed keys.** `by_start.sort()` (split_face) and `by_ends.sort()` (pieces) become `sort_unstable_by_key` on `VKey::packed()` (u64) and a u128 pair. The second field is a distinct index, so stable and unstable sorts give the same list. | Standard: radix or pattern-defeating unstable sorts on integer keys | **−5 to −7M (−3%)**. The sorts are about 11.4M at their top frames over both kinds. Packed-key unstable sort is about 2–3× cheaper. | None, given distinct keys (asserted in debug). `packed` is order-preserving (boolean.rs:224). | **A sort is unique on a strict order:** two sorted permutations of a duplicate-free list are equal. Plus `packed` is monotone. ForgeBoolCore's `RefineSort` already has this style. | S (~100) |
| **S4** | **Classify once per region, not once per piece.** Pieces of one face that are joined without crossing the other body's surface share their inside/outside. Classify one piece per group by ray (`point_in_face_past`, `planar_parity`) and propagate the rest by union–find over shared uncut edges. | Cherchi 2022 (one ray per patch); Lévy 2024 (combinatorial propagation, one ray per component); Zhou 2016 (winding numbers over cells); decide-once's own flood | **−6 to −9M (−3 to −4%)**. Classification is about 15M flat inclusive (`point_in_face_past`, `planar_parity`, `Index::classify`). It drops to about one ray per group. | **Medium.** Exactly, the propagated answer equals the ray's. In floats, the old path's tolerant ray could differ on a near-degenerate piece. Such a difference is an inconsistency today, but it would still change bits. Run L6 (fuzz plus the bits gate) and log every piece where a sampled ray disagrees with the propagation. | Exact statement: **`JordanLocal.contains_const_local`** (proven): parity is constant along a segment that meets no face. Two pieces sharing an uncut edge are joined by such a segment. The float path needs the predicate filters' L5 bounds (`orient2d`/`dot3`) to hold at the piece's interior point. | M (~300 for the propagation lemma over the old path's piece graph) |
| **S5** | **Caches per body across a regen.** `Index::new` for the target, `edge_box`/`face_box` and `rebuild_shells`/`connected` are recomputed every boolean over all ~183 faces. Keep them on the body, and patch only the faces `apply` touched. | Manifold's lazy CSG tree (results reused); EMBER's octree (only changed cells rebuilt); SolidWorks' local update (US 8,305,376) | **−5 to −8M (−3%)**. `Index::new` is 4.1M, `edge_box` 5.0M, `face_box` 1.6M, `rebuild_shells` 4.6M and `connected` 3.8M on the line, about 19M, of which about half is avoidable. | Low. The cache must be invalidated exactly where `apply` edits, and a stale entry is a silent wrong answer, so debug builds recompute and compare. | **The index is a pure function of the body:** patching it after an edit equals rebuilding it. The invariant is `idx = Index::new(body)` after every `apply`. | M (~250) |
| S6 | `uv_dir` for planes: a plane's (u, v) direction is a projection onto the frame, without the generic `Surface::project`/`normal` | — | −1 to −2M | Low (same expressions, as S1) | expression identity, as S1 | S |
| S7 | `std::thread` over face pairs | EMBER's work stealing; Manifold's `autoPolicy` | **0 on the gate:** it counts instructions, and threads add them. At about 1.2M instructions (≈0.3 ms) per boolean, spawn and join cost about as much as they save. Manifold runs serially below 4,096 elements for this reason. | — | — | not recommended |
| S8 | Integer or fixed-point plane coordinates for flat parts (EMBER, trueform) | EMBER (26-bit input, 256-bit predicates); trueform (int32/int64 grids) | — | **Changes geometry.** Snapping vertices to a grid changes results that work today. | — | rejected for the shipped path |
| S9 | BVH or sort-and-sweep candidate pruning | Manifold's BVH; Cherchi's octree | about 0 at our size. The face-pair loop is about 10 near × the tool's faces with a box test, and `trim` already skips edges by box. | — | — | not needed below about 1k faces |

**The first four together: about −35 to −45M on coplanar seq (−17 to −22%).** On a flat boolean,
1.21M → about 0.75M, if S1–S4 land as estimated.

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

1. **S1, feature `flat_line_meet`.** Add the `(Line, Line)` arm to `plane_meet`, inlining exactly
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
