# The curved face builder's Lean statements, against the float mapping (agreed with curved Rust, 2026-11-23)

Context: curved-builder-mapping.md (dol-curved-rust, b3f88e2), agreed by the master on 2026-11-23.
The common path is today's float code, with each tolerance decision replaced by a screen whose
bound is proven. Exact algebra is the fallback for tangency, coincidence and triple-near cases.

The rule for every statement below: **a theorem's hypotheses are facts over ℝ, never
computations.** Something that discharges a hypothesis is one of these:
- a screened f64 decision whose enclosure is proven (the IEEE contracts 1–5 of the mapping);
- an exact fallback decision.

Both discharge the same hypothesis. Nothing in Lean depends on which one ran.

## What is already proven (forge-curved, standard axioms)

| Lean | states | consumed by |
|---|---|---|
| `FacetSides.sideLocal_of_data` (F5) | `FacetData` ⇒ `SideLocal` | route B's `Obligations` |
| `F6.f6` | bodies bounded by their faces, each piece oriented at one point ⇒ per-room memberships that flip exactly across pieces | `flood_correct` (coordinator) |
| `FaceSense.grad_*` | the gradient of `QFace.quadric` per kind | `same_sense` ⇒ F6's `o` |
| `Nesting.anchored` | crossings along a path, membership flipping at each ⇒ end in face ⇔ count even | holes and nesting (step 3) |
| `Nesting.flip_of_transversal` | the region is one side of `c` near the crossing, and `(c∘γ)' ≠ 0` ⇒ flip | `anchored` |
| `BranchOrder.branch_order` (L1) | no singular zero on the slab ⇒ branches keep their order | the exact fallback's branch matching |
| `Star.star` (L2) | compact box, one point over the event, arms cover the rest ⇒ star | the same |

## Proposed new statements, one per screen of the mapping

**S0. Keyed identities** (added by curved Rust, curved-fallback-rates.md at 7f83451). The most
common questions on every curved boolean are coincidences by construction:
- a curve ∩ edge point lies on that edge;
- two cuts of one curve at one vertex are that vertex;
- faces with equal exact carrier classes share a surface;
- a face's vertex lies on a coincident face.

These are decided by provenance and exact class keys, not by a screen, because no sign screen
can decide a zero. In Lean a keyed decision is the fact it names: equal carriers, or a point that
is the meet of the named carriers. Real near-misses are typed refusals: none on the gate parts,
and 58 decisions in 16 of 6,440 fuzz plans.

**S1. Sign.** Contract 1, `sign_filter_iv`: the enclosure holds `q(p)`. This is the IEEE session's statement. Lean consumes it as `sgn q(p) = s`.

**S2. A crossing along a trim or a conic** (mapping rows `trim` and `split_face`; contract 3).

- Input: a parameter interval `[t₀, t₁]` of a path `γ` (a trim edge, or a sample conic `q_f ∩ {x = c}`). One of the following must be certified:
  - **constant**: the enclosure of `c∘γ` over the interval excludes 0;
  - **one crossing**: `c∘γ(t₀)` and `c∘γ(t₁)` have opposite signs (S1), and the derivative enclosure over the interval excludes 0 (Krawczyk, or a monotonicity check).
- Lean, `crossings_of_cover`: suppose a partition of `[a, b]` into such pieces is certified. Then:
  - the zeros of `c∘γ` on `[a, b]` are exactly one per crossing piece;
  - they are sorted;
  - at each of them `(c∘γ)' ≠ 0`.

  These are exactly `parity_path`'s `ts`, `hoff` and the transversality that `flip_of_transversal` needs. A piece certifying neither is the fallback, which is exact or a counted Skip.

**S3. A vertex** (contract 3/5; mapping `cert_vertices`). Input: a box with:
- a proven enclosure of exactly one common zero of its carriers (interval Newton or Krawczyk);
- the carriers' gradients independent over the box (a determinant enclosure excluding 0).

Lean: the box holds one point of the carriers' intersection, and it is transversal there. This is what route B's `VertModel` and `LocalConn` use (`localGood'_of_transversal`). If the enclosure holds a second root, that is the fallback.

**S4. The angle order at a vertex** (contract 4; mapping `split_face`). Input: per pair of arc ends, the enclosure of the tangent cross product excludes 0.

Lean: the cyclic order of the arcs round the vertex, in the face's tangent plane, is the sorted order. This gives `VertModel`'s sectors: each sector between consecutive arcs is one facet's local cell. If the cross product is within its bound, the fallback is the exact curvature order, or `Skip::Tangent`.

**S5. Holes and nesting** (mapping: "holes by a cycle-area screen").
- On a **plane**, a cycle's signed area is a correct orientation test (flat's area contract). Keep it there.
- On a **curved carrier**, signed area has no meaning: on a closed carrier, a loop bounds two regions. I propose S2 plus `Nesting.anchored`:
  - the count runs along a sample conic, anchored at a trim sample pushed to the face side by `o_f`;
  - each crossing is an S2 piece.

  Today's `split_face` decides holes by uv signed area (boolean.rs at 7f83451, B:5525–5606).
  That chart is injective only inside one period window, and a cycle that wraps the axis is
  skipped there, not decided. **Agreed:** S5 is the statement. The uv area may serve as an
  alternative screen only for non-wrapping cycles on cylinders and cones, with the chart's
  injectivity as its hypothesis.

**S6. Orientation** (mapping `classify_specs`). There is no screen here. `o_f` is `same_sense` combined with the per-kind rule (FaceSense). The hypothesis "the body is bounded by its faces" comes from the input's validity (`forge_topo::check`), as F6 states it. A NoSeed ray's hits are S2 pieces along the ray.

**S7. Rooms** (A8 flood). `Rooms.room_preconnected`: each flood step joins pieces whose closures meet in the room. A step is certified by a shared split facet (combinatorial), so there is no new screen.

**The S2 piece** (agreed):
- on a closed-form conic, the piece is a parameter interval of the closed form, and γ is analytic;
- on a meet chain, it is one Bernstein chart's interval of the certified trace, and γ is a
  polynomial there.

**No runtime fallback to the exact reference** (owner ruling). The reference is the fuzz oracle
only, and an undecided question is a typed refusal. "Fallback" above therefore means a typed
refusal, or an exact keyed fact (S0).

## What the builder must hand to Lean, per face ledger

The proof is stated over these artifacts; it never reruns the floats. What the builder must record:
- the decisions only: signs, the crossing pieces and their order, vertex boxes, angle orders;
- each decision's certificate kind (screen or fallback).

## Order of work (after agreement)

1. S2 `crossings_of_cover`, plus a corollary combining it with `anchored`. This is the nesting step on floats.
2. S4 angle order, to `VertModel` sectors.
3. S3 vertex boxes, to `LocalConn` through transversality.
4. The assembly: the builder's recorded decisions give `FacetData`, F1–F4 and F6's hypotheses, and so route B's theorem.

R1/Faithful follows, but only for the parts curved Rust builds (master, 2026-11-23).
