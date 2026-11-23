# Booleans at our size: fixed costs, and which path to ship (2026-11-23)

This is a follow-up to prior-art.md, at the master's request. Our booleans are tiny: about 380 faces
per boolean, about 11 of them near. The papers time meshes of 10⁴–10⁷ triangles.

The question has three parts:
1. What does the literature say about per-boolean fixed costs at this size?
2. Can a proven exact method match a fast floating-point one here?
3. Or should decide-once be the proven reference and the old path the shipped fast path? If so,
   how would the old path get L4?

## 0. Answer

1. **Exactness is not what costs at this size, and the literature shows it.** Filtered exact
   predicates cost about what float predicates do when the filter succeeds (Shewchuk 1997). With
   fixed widths, a three-plane meet costs 23–402 cycles and a vertex classification 13–142
   (Nehring-Wirxel et al. 2021, Table 1). The exact EMBER beats the inexact QuickCSG by about 10×,
   and the exact trueform beats the float Manifold by 7.6×.
   - At 11 near faces, all the exact geometry the boolean needs comes to about 0.1M instructions
     (§2).
   - Decide-once spends ~10M per boolean and the old path ~3.0M. Both are dominated by structure:
     a global arrangement over every class, per-face work over all 380 faces, and allocation.
     Neither is dominated by exact arithmetic.
2. **Recommendation: make decide-once the shipped path, and budget it below the old path.** The
   alternative is to prove the old path, which I estimate at months, with no clear end (§3).
   - The target is **≤ 2.5M instructions per boolean** on coplanar seq, against the old path's
     3.0M. The phase budget is in §2.
   - The old path stays the default until decide-once passes the callgrind gate line by line, as
     now.
3. **The old path is not a practical L4 target.** It is about 16k lines of f64 tolerance code
   with retries, and its decisions are tolerance-based, so there is no exact L1 for it to refine.
   The one sound way to ship old code "proved correct" is as an **untrusted producer behind a
   proven checker** (certifying algorithms, §3.2). At our size, the checker costs about what the
   local decide-once does. So once A2–A5 land, it buys nothing over shipping decide-once.
   Whether "proved correct" may mean "every output checked by proven code" is the owner's call.

## 1. What the literature says about small inputs

None of the papers times inputs as small as ours. What they do show:

| Source | Smallest timed case | Fixed costs named | How small inputs are handled |
|---|---|---|---|
| EMBER (TOG 2022) | Fig. 19: 3–5 ms at about 10⁵ faces per operand. The Thingi10K geometric mean is 1.6 ms at about 2·10⁴ faces, on 8 cores. | Converting every input polygon to the plane representation (plane from points: 226–761 cycles at 128–256 bits, from the octree paper's Table 1). Building the subdivision. Classifying by tracing segments to the reference point, which costs almost twice the pairwise intersections (EMBER §5, timing breakdown). | Early-outs: a subproblem that cannot change the result stops at once. Work-stealing threads; there is no serial small path. |
| Octree BSPs (CAD 2021) | Per-operation cycle counts (Table 1); 2.5M mesh–plane cuts/s, single core | Allocation-free fixed-width arithmetic: CGAL lazy and gmp vary 5–10% from "their internal allocations" | Single-threaded by design |
| trueform (arXiv 2607.15905, 2026) | Volumetric partition in 59 ms. Browser: 12 ms per difference at about 550k triangles (M4 Max). Re-querying a built arrangement costs 13–37× less than building it. | The build: per-face arrangements and graph labelling. The paper calls every stage "a graph problem on graphs it never explicitly constructs", meaning no global structure. | Untouched faces reuse the input's links. The build and query split means a sequence of operations over the same operands costs one build. |
| Manifold ([Performance Considerations](https://github.com/elalish/manifold/wiki/Performance-Considerations), Aug 2023) | Wiki: one sphere union with `GetMesh` in 14 ms, hardware not stated. | A BVH and collider over all triangles. Sorting and reindexing. Triangulation (ear clipping, serial). "Sending messages between threads can be expensive." | `autoPolicy(size)` runs serially below a threshold (`kParallelThreshold = 4096` in csg_tree.cpp; 10⁵ at a `boolean3.cpp` call site, in the manifold-rs vendored copy). Compose for operands whose boxes do not meet. Lazy CSG trees, so a chain is evaluated once. |
| Cherchi et al. (TOG 2022) | 20 ms at 25k triangles (M1 Pro) | Cached 3×3 minors per triangle, so orient3d is a 4-term dot product | — |

So, at our size:
- (a) parallelism is useless, and every library falls back to serial;
- (b) what remains is the per-face fixed work, over all faces, which the fast libraries keep to a
  copy;
- (c) allocation and global structure are the named costs.

**Our two paths spend in the same places.** On coplanar seq (the master's figures; the 0.68M write
per boolean implies about 70 booleans), per boolean:

| Phase | Decide-once | Note |
|---|---|---|
| arrangement | 3.5M (242M) | a full arrangement of every class plane against all ~23 classes plus 6 walls |
| sample | 0.41M (28.6M) | |
| ledger | 1.6M (111.8M) | `HashMap<Vec<Sgn>>`, a `Vec` per covector |
| core | 1.63M (114.2M) | proven; rows are `Vec<Sgn>` |
| write | 0.68M (47.4M) | after A1 (bool-speed) |
| rest | 0.42M (29.4M) | includes `inputs`, which builds an exact `DyPlane` for **all** ~380 faces, near or not |
| **total** | **≈10.0M (700.8M)** | the old path: ≈3.0M (209M) |

## 2. What decide-once should cost here

A floor, from the octree paper's cycle counts and our size (11 near faces, ≤ 11 near classes plus
6 walls, a few planes crossing each section):

- Pairwise meets: at most 55 pairs, each one `meet3`. At 23–402 cycles, that is under 25k.
- Arrangement signs: about 11 sections, each with about 20 vertices and 17 planes. Inherited
  entries cost about 0. The filtered rest are about 20 instructions each: under 0.1M.
- Witnesses and classification: tens of cells, each with one exact parity test against at most 11
  faces: under 0.05M.
- Far faces: a copy, about 0.3M for about 370 faces at about 1k instructions each, which is the
  old path's own floor too.

**The floor is about 0.5M per boolean.** A budget that beats the old path with margin:

| Phase | Now | Budget | Levers (prior-art.md §2) |
|---|---|---|---|
| arrangement | 3.5M | 0.3M | A2 sparse (`skip_pos`, proven), restricted to near classes; A3 lazy vertices; A4 fixed width |
| ledger | 1.6M | 0.4M | A5 packed covectors, arenas; no `Vec` per row |
| core | 1.63M | 0.6M | packed rows in `forge_bool_core` (an M-sized re-proof: `cov`, `find`, `split` and `membership` change row type) |
| sample, rest | 0.83M | 0.4M | exact planes only for near faces (far faces need none: `far_unchanged`); A3 |
| write | 0.68M | 0.6M | done (A1) |
| **total** | **10.0M** | **≈2.3M** | against the old path's 3.0M |

The two largest items are arrangement and core, and each has a lever whose proof is in hand or
scoped. The per-phase split behind these numbers is still being confirmed: allocation counts are
asked of bool-speed and decide-once (Rust), and so are the mean crossing planes per section.

## 3. Proving the old path instead

### 3.1 Directly, by Aeneas: not practical

- **Size.** kernel/forge-bool is boolean.rs 7.3k lines, imprint 2.1k, classify 1.5k, nary 3.6k,
  unify 0.8k and ssi 0.8k, all f64 with tolerances.
- **No L1 to refine.** Its decisions are "within LINEAR" tests made at several sites, and
  research/kernel-audit/decisions.md records where they contradict each other. L3 needs "the
  decisions give exactly L1 on a stated domain". For tolerant f64 code, that domain is inputs
  whose every near-coincidence is either exact or farther than every rounding error. Stating it is
  possible, but proving that 16k lines respect it is months of work, and each retry or fallback
  needs its own decided rule (PROOF-STANDARD).
- **It is not obviously faster once proven.** The proof would force decided rules onto it, which
  is decide-once's design again.

### 3.2 As a certified producer: sound, but no cheaper here

Certifying algorithms: an untrusted program returns its output with a witness, and a small checker,
which is what gets proven, accepts or rejects it. Sources:
- McConnell, Mehlhorn, Näher, Schweitzer, "Certifying algorithms", Computer Science Review 5(2),
  2011;
- Mehlhorn, Näher, Seel, Seidel, Schilz, Schirra, Uhrig, "Checking geometric programs or
  verification of geometric structures", CGTA 12, 1999
  ([record](https://pure.mpg.de/pubman/item/item_1331069_1));
- formally verified checkers: Alkassar, Böhme, Mehlhorn, Rizkallah, "Verification of certifying
  computations", CAV 2011, and the JAR 52 version, 2014
  ([arXiv 1301.7462](https://arxiv.org/abs/1301.7462)).

For us, it would work like this:
- **Rule.** Run the old path. Check its result with a proven checker. On rejection, run
  decide-once. This is one decided rule, so it meets PROOF-STANDARD's rule for fallbacks.
- **L3.** Accepted ⇒ the result is `reg` of the boolean. Rejected ⇒ decide-once's answer, which is
  proven.
- **The checker for a result `O` of `A op B`**:
  - (1) `O` is valid: `check_sound`, which is proven at L1 and L3.
  - (2) The far faces of `O` equal the inputs' far faces. This is a comparison, justified by
    `far_unchanged`.
  - (3) In the region, `O`'s near faces are exactly the boundary facets of `reg(A op B)`.
- **Why (3) is not cheap.** (3) is not a local check. It needs the cells of the near arrangement,
  so that no boundary piece is missing. The old path's split faces can serve as the certificate for
  those cells, and checking a convex subdivision is linear: exact signs at its vertices, plus exact
  area sums for coverage. But the checker still has to sign every certificate vertex against every
  near class, and run the core. That is the local decide-once's cost after A2–A5, about 1.5M.
- **Cost.** The old path's 3.0M plus about 1.5M for checking is 4.5M, which is slower than the
  budgeted decide-once at 2.3M.

So certification is the right L4 story for the old path if decide-once could not be made fast. It
can, by §2. Certification also stays useful for curved inputs, where decide-once is not yet built.
There it is a way to ship old curved code with proven outputs, once a curved checker exists (the
curved formula route of curved-design.md §2.5a gives one).

## 4. Risks

- The budget in §2 assumes the master's phase totals and about 70 booleans in coplanar seq. If the
  allocation counts show something other than alloc and hashing dominating ledger and core, the
  A5 and core-packing estimates move.
- The core re-proof for packed rows is the one non-small proof on the path. Until it lands, the
  core stays at about 1.6M and the total at about 3.3M, which is about 10% slower than the old
  path. So **core packing is on the critical path for "nothing slower"**.
- Restricting `inputs` to near faces changes which `DyPlane`s exist. `snap` must still see every
  face the region could touch, and `region` already decides that set. The rule is exact planes
  for `reg.near` faces only, and the ledger must be bit-identical, checked by the gate.

Not run: a local micro-benchmark of Manifold on a 380-face part (installing the third-party
package here was declined). Every library timing above is the authors' own, at sizes 10²–10⁴ times
ours.

## 5. Update: measured fill data (decide-once Rust, 2026-11-23)

Measured on coplanar seq with callgrind, switch on: total 0.935G against a base of 0.21G. The run
has 70 decide-once booleans and 140 NotFlat ones on the old path (111M). Per boolean: about 380
faces, about 11 near, and about 10 near classes plus 6 walls. Averaged over 625 calls, one
`arrange` sees:

| | count |
|---|---|
| classes, with the 6 walls | 16.0 |
| classes crossing the clipped section | 9.5 |
| cells | 33.6 |
| vertices | 46.4, of which 23.6 are class–class |

Other measurements:
- `arrange` costs about 148k instructions per call, 93M in all (1.33M per boolean).
- `vertex()` costs about 1.3k per new vertex: a full covector of 16 f64-screened signs.
- Only about 0.3% of sign evaluations go exact.
- After the inline-limb `Big` (b6a9dd4), no L-form allocations remain.

**Corrections to prior-art.md and §2 above:**

- **A2 is worth much less than estimated.** 9.5 of 16 classes cross each section, so the skip
  already prunes most of what it can. A2 still saves:
  - the repeated skip scans;
  - about 6.5 inherited covector entries per vertex.

  Revised: `vertex()` 1.3k → about 0.8k, `arrange` 148k → about 110k, so the arrangement goes
  1.33M → about 1.0M per boolean. It stays cheap to prove (`skip_pos`), but it is no longer the
  top lever.
- **A4 (fixed width) is moot.** Exact fallbacks are 0.3%, and the inline-limb `Big` removed the
  allocations.
- **The real cost is the number of cells,** about 9 sections × 34 cells ≈ 300 cells per
  boolean. Three phases scale with it:
  - the arrangement (per vertex);
  - the ledger (CellTab interning, about 22k lookups per run);
  - the core (`around`/`between`, about 20k `find_h`).

  These three are 1.33M + 1.6M + 1.63M ≈ 4.6M of the ≈10M. The old path splits each face only
  where another face actually crosses it, which on this bench means 2–4 pieces per near face.

**The floor is in the spec, not in constants** (decide-once Rust's conclusion, which I share). The
spec's cells are the sign classes of the *plane* arrangement. A line that crosses a section splits
it everywhere, even where no face of that class reaches. Every fast exact method in prior-art.md
instead arranges *faces*: segments within polygons (Zhou et al. 2016, Cherchi 2020/2022, EMBER's
per-polygon BSPs, trueform's per-face arrangements). That is why they, and our old path, cut far
fewer cells.

Two ways to get there, re-ranked as the top two levers:

| Rank | Lever | Cells per boolean | Expected (arrangement + ledger + core) | Proof |
|---|---|---|---|---|
| 1 | **A6′ region partition, within today's spec.** Split the near box into boxes (kd splits at face-free coordinates) until each box holds about 3–4 near faces. Each box's ledger uses only the classes of faces meeting it: `sampleLedger` of the region, exactly as the single-region ledger does now. Lines then extend only across one box. | about 300 → about 100–150 (estimate) | 4.6M → about 2.0M | **M.** L1: `reg_local` (Frontier.lean, proven) gives the result on each open box from its own faces. One new lemma, `reg_dense`: two sets whose regularisations agree on a dense open set have equal regularisations, since `closure (U ∩ W) = closure U` for open `U` and dense open `W`. The walls are nowhere dense (`wall_nowhereDense`). L3: `local_correct_sampler` per box, unchanged. New Rust: the box splitter, and stitching between boxes in the write (today it stitches region to far only). |
| 2 | **A8 face arrangement, a spec change.** Cells become the components of space minus the faces, not the sign classes of the planes. Each near face is split only by the segments where other faces cross it. | about 300 → about 40 | 4.6M → about 0.7M | **L.** L1/L3: membership is constant on each piece by `contains_const_local` (JordanLocal.lean, proven: parity is constant along a segment that meets no face), so the spec side has its key lemma. But the core reads incidence from covectors (`Cov.le`), and face-arrangement cells have no covector over all classes. So the core needs explicit incidence, which means a new `forge_bool_core` and its Aeneas proof redone. This is the Lean side's call. |

Revised per-boolean picture, all estimates:
- **Today:** ≈10.0M.
- **With A1 (done), A5 and A6′:** about 5M.
- **Adding core packing:** about 4M.
- **Only A8 reaches the old path's ≈3.0M with margin:** about 2M with everything.

So "nothing slower" on coplanar seq needs the spec change (A8). That is a decision for the master
and the Lean side. The levers within today's spec (A6′, A5, core packing) get decide-once to about
1.3–1.7× the old path.
