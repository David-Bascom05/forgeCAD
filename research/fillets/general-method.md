# One definition for ForgeCAD fillets

2026-11-22. Research only: no kernel or interface code was changed.

How sources are marked: **[S]** means read in source this session. **[W]** means confirmed by web search this session, from snippets only. WebFetch could not reach most hosts here (faculty.cc.gatech.edu, scispace.com, docs.nvidia.com and tflex.com did not resolve), so papers I could not open are marked **[K]**: cited from the literature as I know it and not re-read this session. Treat [K] claims as leads to check. Repository facts are cited by path.

---

## 0. Recommendation

**Define the fillet as a ball problem, and compute it from the offset body.** For a radius r, a fillet on concave picks is the part of the **morphological closing** (S ⊕ B_r) ⊖ B_r (Rossignac & Requicha) that the picked edges reach. A round on convex picks is the dual: the **opening**, which is the closing of the complement. "Reach" and the ends of a pick are defined in §1: a short, finite rule set over the reals. The industry's mitre and run-out conventions are one explicit table of vertex types in that rule set, not separate constructions.

**The algorithm follows from the definition.** The centres of the balls that make the closing lie on the boundary of the offset body D = S ⊕ B_r. The blend surfaces are exactly:

- a **pipe** of radius r about each crease of ∂D (where two offset sheets meet);
- a **sphere** about each exposed corner of ∂D (where three or more meet).

Every case the kernel now handles by its own construction is then a **vertex of the crease graph of ∂D** or a **property of one sheet**:

| today's construction | what it is in the crease graph |
|---|---|
| cliff | the crease passes onto a rim tube |
| switch_support | it passes onto a G1 neighbour's sheet |
| `across` / crowding | a sheet is swallowed at a triple point |
| merged / collapse | a triple point where a sheet has zero width |
| Pivot | a sheet degenerates to an axis |
| overlapping_rounds | opposite sheets cross |
| mitre | two pipes meet at a crease that is not exposed |

OCCT's walking states (surface–surface, surface–restriction, restriction–restriction) are the same graph seen one edge at a time.

**What this changes.** Today's kernel picks among constructions by trying them in a fixed order:

- `fillet` → `fillet_with` → `fillet_tried` → `fillet_picked`, with retries for `across`, `seams`, `walk`, `rims`, `straight`, `cliff`, `cliff_seams`, `reseamed` and `joined_pieces` (kernel/forge-ops/src/fillet.rs:375-620);
- then `overlapping_rounds` as a boolean (kernel/forge-model/src/lib.rs:1849).

The result depends on which try succeeded first. In the proposed method there are no retries. The branches are a fixed, finite set of sign predicates (§2) on exactly computed geometry. Each branch has a proof obligation against the real-number spec (§3). If a predicate cannot be certified, the fillet is refused and the refusal names the predicate and the place.

**Keep:**

- the surface kinds: `blend::Pipe`, `tube::Tube`, `blend::offset`, Sphere, Torus;
- the closed-form intersections in `forge-geom/src/meet.rs`, which already include torus–torus as `Form::Trig2`;
- `intcurve` tracing;
- `forge_proof::blends::audit`, as the runtime spec check.

**Replace:** the per-case builders and every fallback.

**Riskiest kernel piece, built first:** a **complete, certified crease-graph builder**. It needs:

- (a) exact sheet–sheet intersection, including pipe–pipe and pipe–quadric, with **nodes at spring crossings**;
- (b) **certified detection of every triple point** along a crease (where a third sheet comes within r).

Everything else is assembly. §6 gives the phases and how to verify each one. There is a slow exact reference outside the kernel (§5.4), and the Lean 4 obligations are in §3.

**Other parts of the report.** Screw faces (threads, coils) and the cost of offsets in degree are in §8. Each fast closed form (plain pipe, Pivot sphere, mitre, full round, cliff) is proven as a refinement of the one definition in §9, with each obligation marked as combinatorial (provable through Aeneas) or numeric (needs certified predicates).

**Where the spec cannot be purely morphological, and why.** The data's truth (Onshape/Parasolid STEP) uses **mitres** where two blends meet beside an unpicked sharp edge. The pure closing would instead roll the ball round that edge. This repository has measured the difference: the closing differs from the mitre by up to 0.335 at r 2 (research/fillets/README.md, "Balls resting on a support carried on past its end"). So the spec has one deliberate convention, the vertex table of §1.3. It is finite, written over the reals, and decided per vertex from its local type. Everything along the length of a blend (smooth roll-over, cliff, crowding, full rounds) is pure closing, with no convention.

---

## 1. The specification, over the reals

### 1.1 Objects

- S ⊂ ℝ³ is a compact regular solid, with boundary stratified into **generators**: open face patches, open edges and vertices.
- Faces that lie on one surface across an edge (seams, or flush splits a boolean left) are **one generator**. This is what makes the result independent of seams by definition, so `reseamed`, `joined_pieces` and `cliff_seams` disappear.
- B(c, r) is the closed ball and B°(c, r) the open ball.
- The picks are E = {(e, r_e)}. They are partitioned into **radius classes**; within one class, either all picks are concave or all are convex.

### 1.2 Whole-body closing, and its crease form

These are standard definitions: Rossignac & Requicha, *Offsetting operations in solid modelling*, CAGD 3 (1986) 129-148 **[W]**, and Rossignac's thesis *Blending and offsetting solid models*, Rochester 1985 **[W]**.

- Free centres: M = { c : B°(c, r) ∩ S = ∅ } = closure of the complement of S ⊕ B°_r.
- Closing: C_r(S) = ℝ³ \ ⋃_{c∈M} B°(c, r). Its fill is K = C_r(S) \ S.
- Contact set: N(c) = B(c, r) ∩ S for c ∈ ∂M.
- Crease set: Γ = { c ∈ ∂M : N(c) does not lie in one generator's closure taken alone }. In general position, Γ is a graph:
  - its **edges** are curves where exactly two sheets are active;
  - its **vertices** are points where three are active, or where a sheet degenerates.
- Cap of c: H(c) = the spherical convex hull of N(c) on ∂B(c, r). It is an arc for two contacts and a spherical polygon for n contacts.
- Shadow of c: Q(c) = the points on rays from c through H(c), beyond distance r and up to their first hit on S.

**Lemma A (crease form of the closing).** K = ⋃_{c∈Γ} Q(c). The new boundary of C_r(S) is ⋃_{c∈Γ} H(c): pipes over Γ's edges and spherical polygons at its vertices. This is the dual of the offset's crease graph. It is obligation O1 in §3.

### 1.3 Fillet on selected edges: the restricted closing

Take one radius class, with concave picks. For a picked edge e with faces F1 and F2, its **seed** is the arc of Γ whose balls touch F1 and F2 near e. Its **wedge** W_e is the solid bounded by F1 and F2 carried on as surfaces.

**Reach Γ_E.** Γ_E is the union of the closed arcs of Γ reached from seeds without crossing:

- an arc whose two contacts both belong to the faces of an **unpicked concave** edge (that edge stays sharp);
- an end vertex of the pick (see "Ends" below).

Arcs whose contacts lie on rims (convex edges) or on G1 neighbours **are** crossed. That makes smooth roll-over and cliff part of the definition, not options.

**Body.** Away from the pick's end vertices:

  R = S ∪ ⋃_{c∈Γ_E} Q(c).

**Ends: a finite table over the local vertex type.** Let v be a vertex of S that ends picked chains, and U_v = B(v, ρ_v) a neighbourhood small enough to meet only v's generators. The *local type* of v is the cyclic word of its edges over the alphabet {picked, unpicked} × {concave, convex, smooth}.

| local type at v | R ∩ U_v | what it is |
|---|---|---|
| every edge at v picked, one radius | S ∪ (C_r(W_v) \ S), where W_v is the union of v's face half-solids | vertex ball (sphere); n faces give an n-gon cap |
| some edge at v unpicked; picked edges e ∈ E_v | S ∪ ((⋂_{e∈E_v} C_r(W_e)) ∩ Cap_v \ S) | mitre (two or more picks); run-out (one pick) |
| other local types | not in the table yet: refuse with the type | — |

Cap_v is the intersection of the end half-spaces of faces at v that are reached across an unpicked convex edge from the support of **no other** pick. It stops a lone blend flush with an end face. It does not cut a mitre: there the face across the sharp edge supports the other pick.

**Why the table gives the right shapes.**

- With two picks the ⋂ gives the mitre: the two pipes, carried on, cut each other. Unequal radii work too, since each closing uses its own r. An independent check is in §5.3.
- With one pick, clipping by S gives OCCT's "run out into the face across the end".
- With all edges picked, the vertex closing gives OCCT's sphere ("three equal blends meet in a sphere", fillet.rs:9).

**Gluing.** Each U_v and the body formula must agree on the overlaps. This is obligation O6. The size ρ_v comes from the geometry: below the distance to the nearest generator not at v.

**Convex picks** are the dual: R = ℝ³ \ fillet(ℝ³ \ S) with the same picks, which gives the opening. Rounds **past half a plate's width** (`overlapping_rounds`) need no rule. The opening of a plate thinner than 2r has a ridge where the inward sheets of opposite faces cross.

**Several radius classes** combine by union of their fills (concave) and intersection of their kept parts (convex), each computed on the same S. **Mixed convexity** is the composition convex∘concave: the concave fill first, then the convex picks named in the result. This fixed order replaces `minority_first`, and it is a convention. Phase 2 checks it against the 4+1 parts that `minority_first` fixed (README, "both kinds meet at a vertex").

**Variable radius.** The closing does not define one. Along the length, the spine is where offset sheets at r(s) meet, as `varblend` already does, and it reads as a rolling ball of varying radius. At corners there is no ball definition: the n-sided patch (`forge-geom/src/corner.rs`) is a *separate* spec, a G1 interpolation, and should be named as such. Equal-radius vertices stay spheres.

### 1.4 Tolerance is part of the spec

The closing is **not** continuous in the input. A slot of width 2r fills up; a slot of 2r + ε does not. Real parts are full of such exact coincidences: "radius = face width or half" was 8 + 5 of the pilot's refusals (README, ranked table). So the spec is stated on **τ-snapped** input:

- A predicate value within τ = tol::LINEAR of zero **is** zero, by definition.
- The result must equal the exact result for some input within τ that has that sign pattern.
- If a computed sign is undecided, because its certified error straddles ±τ, the fillet is refused with that predicate named (§4).
- Symbolic perturbation (Edelsbrunner & Mücke, *Simulation of Simplicity*, ACM TOG 9(1) 1990 **[W]**) is used **only** for ties that carry no design meaning, such as the order of two parameters that coincide. It is never used for radius against width, where the exact degenerate answer (a full round) is the designer's intent.

---

## 2. What the algorithm branches on: the finite predicate set

All predicates are signs, three-valued {−, 0, +} under the τ rule of §1.4. In each, c is a ball centre, g a generator, C a crease.

| # | predicate | sign of | decides |
|---|---|---|---|
| P1 | edge kind | det(n₁, n₂, t) at a point of e (and \|n₁×n₂\| − τ) | concave, convex or smooth; `Smooth` refusal |
| P2 | clearance | d_g(c) − r | whether g blocks the ball (also: foot in face versus on its rim) |
| P3 | event on a crease | roots of s ↦ d_g(C(s)) − r for every generator g not active on C | triple points: cliff onset and return, swallowed face, crowding |
| P4 | exposure | signs of ∇f_i · t_jk at a triple point (the three active sheets f_i and the crease tangents t_jk) | sphere at the vertex, or pipes meeting in a mitre |
| P5 | crease reflexivity | det(∇f_a, ∇f_b, t) | pipe (reflex crease of D) or sharp join (convex crease) |
| P6 | pick label | data | reach Γ_E; vertex table |
| P7 | sheet degeneracy | ρ_g + r for the sheet's own radius (0 means the axis: Pivot); r − κ⁻¹ min (focal) | Pivot sphere; refusal past a focal point |
| P8 | parameter order | s_i − s_j on one crease or spring | assembly order (perturbed when tied) |
| P9 | node branches | definiteness of II₁ − II₂ at a spring crossing | which pipe–pipe branch is the mitre |

There are k = 9 kinds. A local configuration (a crease arc, or a vertex of bounded degree) evaluates a bounded number of them. In general position a crease-graph vertex has three active sheets. The vertex type is therefore a sign vector over ≤ 3 sheets × {P2, P4, P5, P7} plus the P6 labels, which is a few dozen signs. The coverage proof (O3) enumerates **realisable** sign patterns, not all 3^k: most patterns contradict each other, for example P4 forces P5.

---

## 3. Correctness obligations, and which are hard

The spec is stated in Lean 4 over Mathlib's reals:

- `Metric.thickening` gives ⊕;
- closed balls and complements give ⊖;
- generators are explicit semi-algebraic or procedural pieces.

| obligation | statement | difficulty |
|---|---|---|
| O1 | Lemma A: crease form of the closing (K = ⋃ Q(c), new boundary = caps) | **medium**: elementary metric geometry; needs regularity of S (finite stratification, reach > 0 away from edges) |
| O2 | **Locality**: R ∩ B(x, ρ) depends only on S ∩ B(x, ρ + 2r) | **routine** (triangle inequality) |
| O3 | **Vertex coverage**: for every realisable sign pattern of P2/P4/P5/P7 at a triple point, the branch's local surface (sphere cap, two pipes cut, sharp) equals the spec in U_v | **routine for planar generators**, enumeration plus linear algebra; **medium** for quadrics and tori (polynomial sign facts) |
| O4 | **Arc lemma**: between consecutive P3 events, the active pair is constant, so the arc's surface is one pipe | **routine** (continuity and the intermediate value theorem), *given* that P3 found every root, which is O8 |
| O5 | **n edges at a vertex**: the cap of C_r(W_v) is the spherical convex hull of the n contacts; the ⋂ over picks is a finite intersection | **routine to medium**: induction on n, using locality and the correctness of spherical hull insertion |
| O6 | **Gluing**: the vertex table and the body formula agree on U_v ∩ (body) | **medium**: a lemma per table row |
| O7 | **Output is a valid solid**: assembled faces bound R, and R is regular | **medium** for planes, quadrics and tori; **research-hard** in general (isotopy of the computed curve network to the exact one, Plantinga–Vegter style **[K]**) |
| O8 | **Completeness of traced curves and events** (no missed loop, branch or root) on procedural sheets | **research-hard**: needs certified tracing (§4.3) and a Lean theory of validated numerics, which exists in Isabelle (Immler) **[W]** but not, to my knowledge, in Lean 4 |
| O9 | **Float bridge**: every filtered predicate's computed sign equals the exact sign whenever it is not "undecided" | **routine per predicate**; the repository already does exactly this (below) |
| O10 | **Focal offsets**: correctness past a curved face's focal distance (offset sheet with a cuspidal edge) | **research-hard**; refuse instead (P7) |
| — | The vertex table **equals Parasolid** | **not provable**: it is a convention. Provable: the code implements the table. Data in Phase 2 pins the table. |

**Precedents.**

- Pichardie & Bertot, *Formalizing convex hull algorithms* (Coq, TPHOLs 2001) **[W]**.
- Brun, Dufourd & Magaud, *Designing and proving correct a convex hull algorithm with hypermaps in Coq*, CGTA 2012 **[W]**.
- Bertot on triangulations (Coq / MathComp, ICTAC 2018) **[W]**.
- Meikle & Fleuriot, Hoare-logic convex hull in Isabelle **[W]**.
- The verified closest-pair algorithm in Isabelle (Nipkow and co-authors, IJCAR 2020) **[W]**.
- Cohen & Mahboubi, *Formal proofs in real algebraic geometry: from ordered fields to quantifier elimination*, LMCS 8(1:2) 2012. Axiom-free Coq proofs of real closed fields, root counting by pseudo-remainder sequences, and QE **[W]**. This is the route for O3 and O9 on algebraic sheets: exact sign determination of polynomials in f64 coefficients, which are rationals.
- Immler, *A verified ODE solver and the Lorenz attractor* (Isabelle/HOL, JAR 2018) **[W]**. This is the precedent for O8: verified enclosures of traced curves.
- **In this repository**: research/loft-ssi/shipped/*.lean proves loftmeet.rs's hull and Bernstein tests in Lean 4. The f64 error bounds are hypotheses, discharged by derive/proofs/Proofs/Rounding.lean (γ_k bounds), and `#print axioms` is checked (shipped/Axioms.lean). O9 should copy that pattern exactly.

---

## 4. Floating point

### 4.1 Exact or filtered predicates (O9)

- Inputs are f64, so the coefficients of planes, quadrics and tori are dyadic rationals.
- P1, P2, P4, P5 and P8 on these sheets are polynomials in those rationals. Evaluate each in f64 with a proven forward error bound (the loft-ssi pattern). If the result lies inside the bound, re-evaluate in exact expansion arithmetic (Shewchuk-style, DCG 1997 **[K]**), written in-kernel because there are no crates.
- Square sqrt-comparisons away. **No trig in predicates**: dmath trig is used only to build frames, and predicates read the stored f64 frame vectors. This keeps native and wasm bit-identical without any extra work.
- P3 on algebraic creases (line, circle, and the conics of plane against quadric) is a univariate root isolation. Use Sturm or Descartes in exact arithmetic, which is the Cohen–Mahboubi formalisation's subject.

### 4.2 One rule for undecided signs

The rule is §1.4: within τ of zero is zero by the spec. A certified enclosure that straddles ±τ is a refusal ("undecided: P_k at x"). Symbolic perturbation applies only to meaningless ties (P8).

### 4.3 Certified tracing (O8)

Today's tracer (`forge-geom/src/intcurve.rs:533`) is nearly certifiable:

- **Exclusion.** Its octree drops a cell when |f(centre)| > the cell's radius, which is sound because the distance-type implicits are 1-Lipschitz. What is missing is a certified error on f, and a global nearest point for procedural pipes, so that the pipe implicit really is 1-Lipschitz.
- **Uniqueness.** Each step needs an interval-Newton or Krawczyk box proving one crease point per box **[K]**.
- **Completeness.** Needs subdivision until every cell is excluded or holds a certified simple branch (Plantinga–Vegter style **[K]**).
- **Events.** Root isolation of d_g(C(s)) − r along a traced crease, using Taylor-model or interval enclosures of the crease.

All of this uses only IEEE arithmetic and sqrt, with directed rounding emulated by error bounds rather than by rounding modes, so the results are bit-identical on native and wasm.

---

## 5. The kernel operations this needs

### 5.1 Offsets of our kinds, and their degrees

The degree is that of the implicit equation.

| generator | offset sheet | degree | exists today |
|---|---|---|---|
| plane | plane | 1 | yes (`blend::offset`) |
| cylinder | coaxial cylinder | 2 | yes |
| cone | coaxial cone, apex moved; past the apex a sphere about it | 2 | yes (cone part) |
| sphere | concentric sphere | 2 | yes |
| torus | torus of minor + d (spindle past major) | 4 | yes |
| pipe of radius ρ about spine C | **pipe of radius ρ + d about the same C** | as the pipe (no growth) | **no**: goes through the generic `Offset` (blend.rs:84). Add `Pipe::grown`, trivial and exact, as `Tube::grown` already is. |
| straight edge (rim) | cylinder | 2 | yes (radius 0 tube) |
| circular edge | torus | 4 | yes |
| other edge curves | `Tube` about the curve | see below | yes (tube.rs) |
| vertex | sphere | 2 | yes |
| loft, spline, other procedural faces | generic `Offset` (implicit by nearest point) | transcendental | yes, slow |

Pipe degree grows with the spine.

- The spine of two sheets of degrees a and b is algebraic, of degree ≤ ab.
- Pipe about a line: degree 2. Pipe about a circle: degree 4.
- Pipe about an **ellipse** (the spine of a plane offset against a tilted cylinder, or a rim on a tilted cut): degree **≥ 8**. Its section by the ellipse's plane is the ellipse's planar offset curve, which has degree 8 **[K]**.
- The quartic spine of two crossing cylinders gives a pipe of higher degree still.

This is why the kernel should keep pipes **procedural** (spine plus radius), and why the arrangement reference of §5.4 is limited to degree ≤ 4.

### 5.2 Intersections the crease graph needs

| pair | use | state | how to do it exactly |
|---|---|---|---|
| sheet–sheet, quadric or torus pairs | spines | closed forms exist (meet.rs `setup`: cylinder–cylinder, cylinder/cone–torus, torus–plane/sphere, **torus–torus Trig2**) | add certified event isolation (P3) |
| sheet–tube (cliff), tube–tube (rim–rim) | spines | lines and circles: closed form; general: traced | certify per §4.3 |
| crease ∩ third sheet | P3 triple points | `curve_surface` exists | **make it complete**: interval exclusion along the crease, and exact roots on algebraic creases |
| **pipe–pipe** | mitres, ridges where blends cross | no general support; `mitre` is a hand-built special case (fillet.rs:5340) | intersection is singular at its **node**, the spring crossing where both pipes touch the shared face (two cylinders tangent to one plane meet in two ellipses crossing there). Solve in spine parameters: unknowns (s₁, s₂, x) with \|x − C₁(s₁)\| = r₁, (x − C₁)·C₁′ = 0, and the same for 2. Start from the node, known exactly as `springs_meet` on the face and the foot of the triple-point ball. Take branch directions from II₁(v,v) = II₂(v,v) (P9). Continue with certified Newton. |
| pipe–face, pipe–sphere | run-out and caps (vertex table) | traced today | as for pipe–pipe; transversal except at known spring ends |

**Springs are never found by intersection.** A spring is the exact foot curve of a crease on its generator, known by construction. Assembly imprints springs onto the faces first (`forge-bool/src/imprint.rs`), so the boolean only meets **transversal** curves.

### 5.3 Today's closed forms become fast evaluations, not alternatives

| today | in the crease graph | fast path |
|---|---|---|
| plane–plane blend | crease line, pipe = cylinder | as now (OCCT's ChFiKPart_ComputeData_FilPlnPln is the same idea **[S]**) |
| plane–cylinder, plane–cone | crease circle or conic, pipe = torus or procedural | as now (ChFiKPart FilPlnCyl, FilPlnCon **[S]**) |
| `Pivot` (radius = arc's radius) | a P7 = 0 sheet (cylinder offset to its axis); the crease meets the axis at a point | sphere, closed form |
| three equal blends meet | exposed triple point (P4 all +) | sphere |
| `mitre` | non-exposed triple point; pipe–pipe from the node | closed form where both pipes are cylinders (two conics) |
| `merged`, `collapse` (radius = width or half) | a P3 double root: the swallowed sheet has zero width | full round at exactly that width |
| `cliff` | crease onto a rim tube: P3 event for the rim generator | cylinder or torus where the rim is a line or circle |
| `switch_support` | crease crosses a G1 sheet boundary (no event: same r, continuous sheets) | none needed |
| `across` / `Crossed` | triple point where a middle face's sheet ends | — |
| `overlapping_rounds` | crossing of opposite inward sheets (dual) | no boolean |
| `minority_first` | the composition order of §1.3 | — |
| `excursions`, `blocked_beyond` | the P3 event sequence along a rim | — |

### 5.4 The exact reference, outside the kernel: slow and provable

The reference has two tiers, both built outside `kernel/`.

**Tier A: membership oracle.** It is general and should be built first. A point x ∉ S is in the fill exactly when every ball of radius r containing x meets the relevant obstacle, as §1.3 defines it.

- Decide this with certified branch-and-bound over the centre c ∈ B(x, r), using interval bounds of d_S(c) − r.
- For semi-algebraic S it is a first-order formula over the reals, decidable by QE (Cohen–Mahboubi is the formal precedent).
- Answers are {in, out, undecided within ε}.
- Volume comes from certified quadrature of the indicator (stratified points with exact answers). This replaces the voxel `closing*.rs`, whose grid noise is about 2% (README, "Rolling on a strip").

**Tier B: exact B-rep.** It covers planes, quadrics and tori.

- The **arrangement of both bodies' surfaces**: cells classified and regularised, as in CGAL's Nef polyhedra for flat faces (Hachenberger, Kettner & Mehlhorn, *Boolean operations on 3D selective Nef complexes*, CGTA 2007 **[K]**) and EXACUS for quadric intersection curves (Berberich et al., SoCG 2005 **[K]**).
- The fill solid is built from the exact offsets of §5.1, restricted to degree ≤ 4 (cylinder, torus, sphere pipes), then combined with S.
- This covers the prismatic and turned parts where creases are lines and circles. Tori are quartic: beyond EXACUS's quadrics, but within exact algebraic arrangements of low degree.
- Creases of higher degree go to Tier A.

**What the reference should settle first, in this order:**

1. **Convention disputes on the "wrong" rows.** 00307228 (wrong corner, 0.09%); 00057004 and 00309568 (mitre not followed); 100001816 (STEP truth puts the round inside the cavity). The reference says what the spec gives, which separates kernel bugs from convention mismatches.
2. **Closed rounds over a rim**, whose seam-dependent wrong volumes led to commit 0cb9725c (cliff against notch).
3. **Crowding refusals** ("springs crossing or touching a neighbour's: most of what is left", README ranked table), and the parts whose ball crosses several short faces (100000470, FreeCAD 4a66f49a).
4. **Carried supports** on lofted walls: 0.1 to 0.4% against OCCT near corners. "Which is better there is not settled by this" (README). The closing does not carry surfaces, so the reference settles this by the spec.
5. Near-tangent rails (ours 330.630 against OCCT 329.887 at r 6), `blend_through` refusals, and `face_turned` (S-blend) parts.

---

## 6. Phased plan, riskiest first

**Phase 0: spec and reference (outside the kernel).**
- Build Tier A, and Tier B for degree ≤ 4.
- Write the Lean statement of §1 and prove O2.
- Run Tier A on every fillet part the census holds. Label each as spec-match, spec-mismatch or convention.
- *Verify*: the reference agrees with the closed forms in `forge-model/src/fillets.rs` cases (exact volumes) to its certified ε, and with OCCT where OCCT builds (tools/occt/setup.sh).

**Phase 1 (riskiest): certified crease graph in forge-geom.**
- Add `Pipe::grown`.
- Add certified P3 event isolation on every crease kind.
- Add pipe–pipe and pipe–quadric intersection from nodes.
- Make crease ∩ sheet complete.
- *Verify*:
  - fuzz algebraic pairs against meet.rs's closed forms;
  - on random plane/cylinder corner scenes, compare the graph with Tier B's arrangement exactly;
  - check every traced point with the ball test (`forge_proof::blends`, bound 1e-7 as now);
  - check bit-identity on native and wasm;
  - **timing gate**: on ordinary parts the closed-form paths are as fast as now, because only clearance checks against generators within 2r are added, and those go through the face-box index.

**Phase 2: reach and vertex table, no topology yet.**
- Compute Γ_E and its caps for every census part.
- Compare predicted volume (shadow integration) with the current kernel, the reference and the STEP truth.
- Pin the vertex table against the data, including the composition order for mixed convexity.
- *Verify*: no part that matches today mismatches under the prediction.

**Phase 3: assembly.**
- Imprint springs, add pipe, sphere and cap faces, and trim transversals.
- Retire `Builder`'s per-case paths and every retry in fillet.rs and lib.rs, **one class at a time**, each gated by `census.py` / `tally_census.py` with no match lost.
- *Verify*: `forge_topo::check`, the audit, the reference, and OCCT.

**Phase 4: Lean.**
- O1, O3–O6 for planar generators, then quadrics and tori.
- O9 per predicate, as in loft-ssi.
- O7 and O8 stay hypotheses, discharged by runtime certificates, and are stated as such.

**Phase 5: variable radius** on the same graph (VarBlend spines); corner patches under their own named spec.

### Expected effect on README's tables

This is an estimate. Nothing was measured this session.

- **Refused should fall**: crowding, mitre not followed, multi-short-face, and retry-budget timeouts.
- **Stays refused, by stated reason**:
  - focal offsets (P7);
  - uncertifiable signs;
  - vertex types outside the table (e.g. unequal radii at an all-picked vertex, where the industry uses a setback patch);
  - focal-region lofts.
- **Unchanged**: parts "stopped" by an earlier translation, which only re-translation can change.
- **Wrong should fall** where it came from fallback order (seam-dependent results). Wrong rows that the reference shows to be a convention gap are fixed in the table, not in the code.

---

## 7. How the literature and kernels compare

| method | one definition? | corners | overflow | crowding | intersections needed | fits our constraints? |
|---|---|---|---|---|---|---|
| **Morphological closing/opening** (Rossignac & Requicha 1984 **[K]**, CAGD 1986 **[W]**) | **yes**; per-edge selection needs a rule (§1.3) | falls out (spheres) | smooth and cliff fall out; never notch | falls out | offset sheets of every generator against each other, globally (an offset solid); then pipes about creases | yes, if computed as the crease graph (exact procedural pipes, no NURBS) |
| **Rolling-ball envelope / spine tracing** (Choi & Ju, CAD 21(4) 1989 **[W]**; Vida, Martin & Várady survey, CAD 26(5) 1994 **[W]**) | an edge-local definition; corners and overflow are separate topics | separate vertex blends, setbacks, n-sided | separate rules | not defined | offset–offset spine; contact tests | the same primitives; it is our current method |
| **OCCT ChFi3d / BRepBlend** **[S]** | **no**: `PerformFilletOnVertex` dispatches on the number of stripes and the sharp-edge count (one, two, three, more, singular; ChFi3d_Builder.cxx:611-727); walking switches functions surf–surf / surf–rst / rst–rst with `Decroch` flags (ChFi3d_FilBuilder.cxx 776-2177) | case per count; "MoreThreeCorner" as "last chance" | surf–rst walking (cliff) | largely unsupported | walking; blend surfaces **approximated** (AppBlend, ChFi3d_Builder_6.cxx 540-629) | **no**: it stores approximations |
| **Parasolid / T-FLEX / Solid Edge / NX** | rules in order: smooth → cliff → notch; T-FLEX says cliff wins over notch when both are on **[W]** (tflex.com overflow page; soliddna.com rounding overflow) | mitre for same convexity with a sharp edge; automatic vertex blend for mixed convexity **[W]** | user options | — | not public | — |
| **SolidWorks** | Default chooses between Keep edge and Keep surface "depending on the geometry conditions" **[W]** (help.solidworks.com Overflow Type) | — | case choice | — | — | — |
| **ACIS** | mitres; vertex blends with setbacks (repo note prior-art.md, from ACIS R10 docs) | — | — | — | — | Spatial docs unreachable here |
| **Implicit / distance-field / R-function blends; Sellán et al., "Opening and closing surfaces", ACM TOG 39(6) 2020 [W]** | one definition | falls out | falls out | falls out | — | **no** as a kernel (approximated or meshed). It is the right *reference* idea: exact kernels avoid the approximation by keeping the distance-field definition as the spec and building exact surfaces (pipes, spheres) for its boundary, which is §1–§2. |

Braid's *Non-local blending of boundary models* (CAD 29(2) 1997, 89-100 **[W]**, content not read) treats blends that interact away from their edge, i.e. crowding, and is worth obtaining.

The industrial evidence points the same way everywhere: shipped kernels are rule-ordered case handling. A forum note even reports that the Parasolid people keep tweaking the code as new special cases turn up **[W]**, though that is anecdotal. Only the morphological definition is a single object, and only it can be stated over the reals for Lean. The price is that the industrial vertex conventions must be written down explicitly as a finite table.

---

## 8. Algebraic faces, helical faces, and what offsets cost

### 8.1 Every filleted surface is algebraic, except screw faces

These are the implicit degrees, with a rational parametrisation where there is one.

| face | implicit degree | its offset (any d where defined) | offset's degree | rational offset? |
|---|---|---|---|---|
| plane | 1 | plane | 1 | yes |
| cylinder | 2 | cylinder | 2 | yes |
| cone | 2 | cone, apex moved; as a full point set, plus a sphere about the apex | 2 (both pieces) | yes |
| sphere | 2 | sphere | 2 | yes |
| torus | 4 | torus (minor + d); spindle past the major radius | 4 | yes |
| pipe about a line or circle | 2 / 4 | same kind, ρ + d | 2 / 4 | yes |
| pipe about an algebraic spine of degree m | depends on m: ≥ 8 for an ellipse (§5.1) | pipe about the **same** spine, radius ρ + d | **same as the pipe**: offsetting a canal surface does not raise its degree | yes when the spine and radius are rational (canal surfaces have rational offsets, Peternell & Pottmann 1997 **[K]**) |
| loft or spline (procedural) | rational patch, high degree | generic offset | for a general surface of degree n it grows fast (a general ellipsoid's offset has degree 8 **[K]**) | generally not |

**The useful fact.** All our analytic kinds are surfaces of revolution or canal surfaces, and that class is **closed under offset with no growth in degree**.

Degree grows in only two places:

1. **Spines**: two sheets of degrees a and b meet in a space curve of degree ≤ ab.
2. **Pipes about spines that are not lines or circles.**

Both stay procedural in the kernel (spine plus radius, exact by Newton) and implicit in the reference. The **cost model** for the crease graph is therefore:

- closed form when both sheets are in {plane, sphere, cylinder, cone, torus} and the pair is in meet.rs's table;
- otherwise one certified traced curve per crease;
- one certified root isolation per (crease, nearby generator) for P3.

On ordinary prismatic parts (lines and circles everywhere) every step is closed form.

### 8.2 Faces of a screw motion (threads, coils)

`forge-geom/src/screw.rs` sweeps a section by u ↦ ((r + a·u) cos u, (r + a·u) sin u, h + b·u). These faces are transcendental (θ appears both inside trig and linearly), so no polynomial reference exists. The definition does not change: the closing is defined for any closed S. What changes is how its pieces are computed and certified.

- **Pure helical faces (drift a = 0: threads).** The screw motion σ is an isometry. The closing commutes with isometries, so C_r(σS) = σ C_r(S).
  - **In the region where every generator within 2r is σ-invariant**, Γ_E is σ-invariant. The generators that qualify are the helical faces of one axis and pitch, and coaxial cylinders. Planes square to the axis and cones do **not** qualify.
  - Its creases are then helices, or σ-orbits of one point, and the fill is σ-invariant.
  - **Reduce to one turn, or to one point of the section.** Solve the crease's section point once: a 2-D problem in the quotient by σ, with the twisted metric dρ² + ρ² dθ² + (dζ + (b/2π) dθ)². The spine is then the helix through that point, and the pipe about it is a helical pipe, itself a screw face.
  - **Exact under these constraints**: the trig is the kernel's dmath, deterministic and bit-identical.
  - **Certified** by interval enclosures of dmath sin/cos with proven error bounds, the same bridge as §4.1, but with trig in the predicate. This is the only place a predicate contains trig.
- **Where the symmetry breaks** (a thread's run-out into an end face or chamfer, or a coil's end): by locality (O2) only the 2r-neighbourhood of the break is non-invariant.
  - There, P3 events on a helical crease are roots of exponential-polynomial functions of s (z linear in s, x and y trigonometric in s).
  - On a bounded s-range they are finite. Isolate them with certified interval Newton.
  - If an event cannot be certified, **refuse**, naming the predicate.
- **Coils with drift (a ≠ 0).** There is no isometric symmetry (the radius grows with u), so there is no reduction. Use certified tracing (§4.3) throughout, or refuse.

**The reference on screw faces.**

- Tier B (algebraic arrangements) cannot represent them.
- Tier A (membership) still applies, with d_S evaluated by a screw-reduced 1-D certified minimisation over the angle. Its answers are certified to ε, not exact rationals.
- For thread fillets whose neighbourhood is σ-invariant, the reference is the 2-D quotient problem, decided once.

**Lean.** The invariance lemma (the closing commutes with isometries; an invariant S has an invariant fill) is **routine**. Certified trig enclosures are **medium**: a dmath error-bound lemma per function, in the loft-ssi rounding pattern. Completeness of event isolation for exponential-polynomials is **research-hard** (O8).

---

## 9. Fast closed forms as refinements of the one definition

### 9.1 Pattern

Each fast path is a pair (guard G, construction F), and the kernel runs `if G then F else General`. The obligation is a refinement:

> **R(F):** for every τ-snapped input where G holds, F's surface set and topology equal General's, both equal the spec's restricted closing (§1.3), and F does not refuse there.

The second half is **totality on G's domain**: a fast path may never refuse an input the general path would build.

Proofs split two ways:

- **Integer or combinatorial.** Dispatch, the vertex-table lookup, face, loop and edge bookkeeping, naming, and Euler counts. These are proven on the Rust itself through **Aeneas** (safe Rust → Lean; Ho & Protzenko, ICFP 2022 **[K]**). Aeneas's model is integer and ownership based, which fits the topology code.
- **Numeric.** The guards' signs, and the geometric identities. These are proven over ℝ in Lean, plus the certified-predicate bridge (§4) that ties the f64 sign to the real one. Aeneas does not help with f64 semantics.

### 9.2 Obligations, one per closed form

| fast path | guard G (predicates) | F | refinement obligation | kind |
|---|---|---|---|---|
| **plain pipe**: plane–plane → cylinder | P1 concave or convex; both sheets planes; P3: no events on the edge's range | crease = line, pipe = cylinder, springs = lines | real identity: offset planes meet in that line, and the cap is the arc (O1 specialised); G's "no event" is certified P3 | numeric guard; routine algebra; dispatch by Aeneas |
| plain pipe: plane–cylinder, plane–cone, cylinder–cylinder coaxial | as above, with the pair in meet.rs's closed-form table | crease = circle or conic, pipe = torus or procedural | closed-form crease = sheet ∩ sheet (polynomial identity); events none | numeric; routine for circles, medium for conics |
| **Pivot sphere** (`pivot_of`) | P7: \|ρ_cyl − r\| ≤ τ; plane square to the axis within angle τ; the rolled centre on the axis | sphere at axis + n r | on the snapped input the cylinder's offset sheet **is** the axis line; the crease graph is the single point where it meets the offset plane; the cap is that ball's cap; F equals C_r(W) near the arc | numeric guard; routine real proof |
| **three equal blends** → sphere | vertex table row 1; three active sheets; P4 exposed (all +) | sphere cap bounded by three section arcs | cap of C_r(W_v) is the spherical triangle of the three contacts (O5 with n = 3) | numeric guard; routine |
| n equal blends at a vertex | row 1; n sheets | spherical n-gon | O5 by induction on n (spherical hull) | numeric guard; combinatorial hull proven through Aeneas, with its orientation tests certified |
| **mitre** (`mitre`) | row 2; both pipes plain (cylinders or tori); the node exists (springs meet on the shared face) | pipes cut along the branch through the node | ⋂ C_r(W_e) near v equals the region under both pipes; its boundary crease is the P9-selected branch of pipe₁ ∩ pipe₂; for two cylinders the branches are conics (exact). Totality: whenever row 2 holds and the node exists, F builds | numeric (P9 definiteness, node certified); the curve is exact for cylinders; for tori, medium (certified trace from the node) |
| **merged / collapse** (r = width or half) | P3 double root within τ | full round: one pipe from the faces beyond | on the snapped input the middle sheet has zero width along the arc; the crease graph has a degree-4 vertex along it; F equals the spec | numeric; medium (the multiplicity must be certified, not just the sign) |
| **cliff on a line or circle rim** | P3 event onto a rim generator; the rim is a line or circle | crease = offset sheet ∩ cylinder or torus about the rim, closed form; pipe about it | F's spine equals that crease between the onset and return events | numeric; routine for line rims, medium for circle rims |

**Shared combinatorial obligation.** Every F leaves the B-rep in the same topological state General would: the same faces, loops, edge incidences and names. Prove it once on the shared assembly code through Aeneas, and per fast path as "F calls assembly with the same inputs". This is what lets the fast paths stay fast without becoming second constructions.
