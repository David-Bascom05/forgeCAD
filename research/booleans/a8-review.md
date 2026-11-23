# Review of the A8 design (research/decide-once/face-arrangement.md @ 2ff687bd), 2026-11-23

This is bool-research's review, as the Lean coordinator asked. The coordinator wanted three things
looked at:
- (a) the §1 interface, against what a face-arrangement builder naturally produces;
- (b) §3's cell counts and costs, against the fill data;
- (c) §5's open questions.

The measurements are in research/booleans/face-arrangement-measure.md (§0, §5, §6).

**Verdict.** The design is sound and better than my draft. Coarsening the never-run plane
reference, with a simulation theorem, keeps every proven conclusion and confines the new geometry
to `room_const`. I recommend sign-off with the changes below. Changes 1–3 are needed; 4–7 are
smaller.

## 1. Changes needed

**1. The "Why" paragraph has the wrong numbers.**
- The old path costs **1.28M per flat boolean**, not 3.0M. Measured with callgrind, switch off:
  `boolean_into` takes 200.6M over 210 booleans. The 140 NotFlat ones take 111M, leaving 89.6M for
  the 70 flat ones.
- The plane ledger does not hold about 300 cells. Measured per boolean on coplanar:
  - ledger: 231 cells, of which 47 pieces, 72 edges and 30 verts;
  - face-arrangement estimate: about 30 facets.
- The "~300" are the section cells `arrange` builds and throws away.

| | Plane arrangement | Face arrangement (estimate) |
|---|---|---|
| coplanar pieces | 47 | 30 |
| housing pieces | 19 | 15 |
| upto pieces | 54 | 20 |

So A8 cuts the **arrangement** work about 10×, but the **ledger and core** cell counts only
1.5–2.5×. The core still gains, because each step is a Csr read, not a 16-wide covector scan.

Expected total, per flat boolean on coplanar, as estimated in face-arrangement-measure.md §6:
**≈1.65M against 1.28M**. §4's switch rule ("R4 beats the old path on all three benches") will not
hold unless two Rust items join the plan:
- **R5, an in-place write.** The old path's `apply` is 0.06M per boolean, because it edits the
  target's near faces. Decide-once's write is 0.68M, even after A1.
- **R6, exact planes and points for near faces only.** `inputs` converts all ~183 faces today.

Please add both to §4, with R4 measured after them.

**2. Rooms from edge-radial union–find are not always components, so (r)'s ρ can be ill-defined.**
- The natural builder joins facet-sides by radial order round edges. Two shells that share no edge
  (a tool wholly inside the target's near part, giving a cavity, or a hole's floor boss) give two
  side classes that lie in **one** component of `R° ∖ faces`.
- A reference tope between them (a convex cell touching facets of both shells) then has no single
  room. ρ : topes → rooms fails.

Two fixes:
- (a) **Nesting pass.** One exact ray per shell component that the radial union–find left
  separate merges side classes into true rooms. This is Lévy 2024's "one ray per connected
  component" and Zhou et al.'s nesting step. Its cost is about one ray per extra component, which
  is rare on the gate.
- (b) **Make ρ a relation.** Let ρ ⊆ topes × rooms be total and onto, with equal membership on
  related pairs. The core never needs rooms to be components: memb, keep, result, the touch rule
  and unify read facets, edges and verts. Only the classification needs each class's membership
  constant, and `room_const` gives that per side class.

I recommend (b). It keeps the builder local, with no global step, and costs one quantifier in
`Refines` (r). With (b), a tope may relate to several rooms, all with one membership vector. The
simulation theorem's room clauses are then stated through "some related room", and memberships
agree because the memberships are equal.

**3. W1 seams are frequent in flat input, not rare.**
- A slit is a crossing segment `P_f ∩ P_g` that ends inside `P_f`, where an edge of `g` pierces
  `f`'s interior. Every slit makes a wing: the facet meets the slit from both sides.
- That is not only the touch case §1 lists. It is any tool edge passing through a face's interior:
  a corner of a box poking through a plate, or a pocket's wall ending inside the face.
- The cost is small: one seam per slit, along the reference line of `g`'s plane, to the facet's
  boundary or the next cut. But the builder needs it as a core step, not an edge case.
- My piece estimate (1 + crossings per face) already counts one piece per segment, so the numbers
  above include the seams to first order.
- Please name slits in §1's W1 paragraph, as the common source of wings, and give the seam rule
  for a slit. Two candidate rules:
  - extend along the line of the slit's carrier until the first boundary point or cut, on the
    side away from the slit's end; or
  - both ways, if the slit's end is a vertex that other cuts meet.

## 2. Interface (a): what a builder produces naturally

| §1 item | Builder's view | Comment |
|---|---|---|
| walls bound rooms, not facets | the builder clips each near face to the box, and wall edges are facet-boundary edges with one facet round them | Fine. Keep walls as non-facets (§5 Q1). A pseudo-body would add facets the write must drop. `enclosed`: a room is enclosed iff none of its facet-sides' facets has an edge on a wall. That is a table the builder fills, so add `room_touches_wall: Vec<bool>` (or derive `enclosed` in the core from wall-edge flags in `edge_facets`) rather than make it geometric in the core. |
| facets with holes | arise whenever a closed loop of cuts lies inside a face (a boss or pocket footprint) | **Fine for the write.** `write` already gives holes to their outer loop (`inside_loop`, decide_once.rs step 3). No need to cut facets to simply connected (§5 Q3). |
| `facet_rooms` by the class normal's sign | the builder knows each facet's carrier and normal | Fine |
| `pieces`: every facet in one or more faces | coplanar overlap: the builder overlays both faces' cut sets on the class plane, and facets with one point set are one facet, a piece of both | It needs the imprint term `P_f ∩ ∂P_g` (same class, other body) in the cut set. Name it in §1. |
| `edge_facets`, `vert_*` | from the per-face planar arrangements, joined across faces by the exact edge and vertex keys: a pair or triple of carrier ids, as in trueform's "two-level identity" | Fine. The keys are the reference cells' zero sets, which makes τ and ν easy to define. |
| radial order (needed for rooms) | not in the core's interface: it is the builder's business | Fine, but `Refines` (e) should say the builder's rooms respect it. That is a consequence of (r) plus `room_const`. |
| what the write needs and the core does not | facet loops, with exact vertex points, for the write | Outside the core, as §1 says. |

## 3. Witnesses (§5 Q2)

**Keep the flood.**
- Today, `witness_inside` already answers most memberships without a ray: when a side touches a
  piece of the body, the face's orientation gives it. Only sides touching no piece of that body
  cast a ray. With about 15–25 rooms and 2 bodies, that is a handful of rays.
- The flood's union–find costs nothing at this size, and its proofs port.
- Per-room witnesses with no flood would need the orientation shortcut proven as a separate lemma
  (it is `boundary` restated), with no speed gain.

## 4. Smaller points

4. **Naming (§2.2a item 6).** "The least input-face id over all pieces" changes which name a
   merged face gets wherever today's head-first rule differs. Today that is only behind the
   switch, but the bits gate hashes names once the switch is on. Either:
   - show on the gate that the two rules agree (check it with the coarsens check), or
   - mark the bits change as intended, which needs the owner's OK under "nothing that works now
     fails".

   The old path's naming rule should be checked too, since R4 compares against it.
5. **(v) and many planes through one vertex.** At coplanar's T-junctions and cube corners, a vert
   can have 5–6 carriers through it. The "onto, fibres connected through τ = none" condition holds.
   But the debug coarsens check must enumerate fine stars, which the reference ledger lists only
   up to `ENUM_ZEROS = 12` before scanning. That is fine for a check, which should be noted, not
   needed.
6. **Debug coarsens check cost.** It runs the full plane fill: about 8M per boolean today, and
   about 5–6× slower in a debug build. It belongs in CI on the gate corpus, not in every debug
   test.
7. **§3's "~40 cells, well under 0.6M".** Use the measured counts: about 30 facets, 60–70 edges, 40
   verts and 15–25 rooms. The core then costs about 0.35M by my estimate, which is consistent with
   "under 0.6M".

## 5. Q4 and Q5

- **Q4, a direct topological L1 over the face ledger.** It is worth having for curved, where the
  reference's `Faithful` is the expensive part. For flat it is not needed. I have no objection to
  deferring it to q1.
- **Q5, curved.** The same floor appears there, and the scheme transfers, as the draft says. The
  one addition is that change 2's relational ρ matters more in curved: shells touching only along
  contact curves are common there.

## 6. The ≤1.28M target: a bottom-up budget (2026-11-23)

The master set the target on 2026-11-23: A8 costs at most the old path's **1.28M per flat
boolean**, measured on coplanar, housing and upto.

Two estimates bound where A8 lands:
- **Top-down, about 1.65M** (face-arrangement-measure.md §6): today's phase costs, scaled by the
  measured cell ratios. This assumes the bridge keeps today's constant factors (HashMaps, a `Vec`
  per cell, all faces converted).
- **Bottom-up, about 0.75M**: below, from per-operation costs measured on today's code (decide-once
  Rust's profile) and the measured counts per coplanar boolean. Those counts are:
  - ≈10 near faces, about 25 near face pairs of different bodies;
  - 9.3 crossing segments;
  - about 30 facets, 60–70 edges, about 40 verts and about 20 rooms.

The gap between the two is constant factors, so the design must fix them as rules.

### 6.1 Per-operation costs (measured today, or bounded from measurements)

| Operation | Cost | Source |
|---|---|---|
| a filtered sign of a plane at a point | about 20–40 instructions | side_at's f64 path; 98% of signs stop there (decide-once Rust) |
| a vertex with a full 16-sign covector | about 1.3k | decide-once Rust, `vertex()` |
| a vertex as a carrier-triple key plus a filtered f64 point (no covector) | about 0.3–0.4k | `meet3`'s f64 shadow plus one bound check; exact only on failure (about 2%) |
| an exact `meet3` or `side` (fallback) | about 2–5k | `Big::S`/`M` with no allocation (after b6a9dd4) |
| one HashMap insert or lookup on a `Vec<Sgn>` key | about 150–300 | 16M of hashing for about 50k operations per run |
| a ray parity against about 10 near faces | about 5–10k | `witness_inside` through `pierced` |

### 6.2 Budget per flat boolean (coplanar)

| Phase | Work | Bottom-up | **Budget** | Rule that makes it hold |
|---|---|---|---|---|
| inputs, region, snap | box scan of all faces (183 × about 50); planes, snap and points for near faces only | 0.08M | **0.12M** | R6: no exact `DyPlane` or vertex for a far face |
| segments | 25 pairs: polygon-against-plane sides (about 6 vertices × 30), then 9.3 crossings clipped by both polygons (about 2k) | 0.03M | **0.06M** | filtered signs; exact only on failure |
| per-face arrangement and seams | about 10 faces, 1–3 cuts each plus W2 sector seams; about 40 vertices as keyed `meet3` | 0.06M | **0.10M** | vertices keyed by a carrier triple in a dense table indexed by near-class id (at most about 32 classes per region), not a HashMap |
| radial order and rooms | about 70 edges × 2–4 facets, one degree-2 filtered sign per comparison; union–find over about 60 facet-sides | 0.03M | **0.05M** | sort small rows in place |
| witnesses (fused) | about 20 rooms. The rooms bounded by a piece of body *i* take membership from its orientation, as `witness_inside` does today; about 3–5 rays remain | 0.04M | **0.06M** | fused into the builder: rays reuse its per-face edge planes |
| ledger (Csr) | about 150 cells, written into reused buffers | 0.02M | **0.04M** | one arena per boolean, cleared and not freed (thread-local), as EMBER and trueform keep their working sets |
| core v2 | flood, keep, result, touch rule and unify over Csr rows: about 150 cells × degree about 3 × about 30 instructions, plus unify's classes | 0.10M | **0.25M** | Csr only; no sort or hash in the core (the coordinator's v2 already drops them) |
| collapse (metric) | per room, about 20 inradius tests from the builder's facet loops | 0.02M | **0.04M** | |
| write, in place | delete the target's near faces, add the result facets' loops, reuse the far topology untouched | 0.12M | **0.20M** | R5: edit like the old path's `apply` (0.06M); naming by the least face id over pieces |
| **total** | | **≈0.50M** | **≈0.92M** | against the target of 1.28M |

The coordinator's v2 core already drops sort and hash. The rules that must be in the design, not
left to the implementation:
1. **Near-only conversion (R6).** Far faces are never converted to exact form.
2. **No HashMap in the bridge.** Keys are small integer tuples over near-class ids, so dense tables
   or sorted arrays do. HashMaps today cost about 16M per run in hashing alone, plus their
   allocations.
3. **One reused arena per boolean.** Today, malloc and free are about 15% of all instructions.
4. **Covector-free vertices.** A vertex is a carrier-triple key, with its point built exact only on
   filter failure. A 16-sign row per vertex is 1.3k each today.
5. **Witnesses fused into the builder,** with the orientation shortcut first.
6. **The in-place write (R5).**

With all six, the budget is 0.92M, 28% under the target. That leaves room for the bottom-up figures
to be off by 1.8× (0.50M → 0.92M) and the target still holds. Without rules 2 and 3, the top-down
1.65M is the realistic figure.

### 6.3 How to check the budget before the full build

1. **R4-early.** Once R3's builder exists, behind its debug coarsens check, callgrind its phases on
   the three benches before the v2 core is proven. The budget's rows are the pass lines. Any row
   more than 1.5× over its budget is fixed before the core work goes on.
2. **Write first.** R5 can land and be measured on today's v1 path. It changes only `write`, and
   the bits gate checks it.
3. **Upto and housing** have 12 faces, so the inputs and write rows shrink and the builder rows
   dominate. Their budget is the same per-boolean 1.28M, which they should clear easily (19.8 and
   15 estimated facets).

### 6.4 Priorities (the master's note)

Real parts go through the curved path. On the gate, only 75 booleans (coplanar 70, upto 4,
housing 1) reach the flat path. A8's main value for the product is therefore that its scheme
transfers to curved:
- the coarsening of a never-run carrier reference;
- explicit incidence;
- the relational rooms;
- the six rules above.

The flat A8 is the place to prove the scheme and measure it. The curved path is where the time
goes.
