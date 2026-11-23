# ForgeCAD kernel audit: where we patch edge cases instead of having one definition

Read-only audit, repo `/home/user/forgeCAD`, branch `claude/repo-review-mbdrlj` (HEAD 25808ab5), 2026-11-22.
Line numbers are for HEAD unless a commit is named. No code was changed.

## 0. Short answer

Yes. Outside fillets, the pattern is concentrated in **one place: how the boolean decides
near-tolerance contact.** "Is this point on that edge?", "does this curve run along that edge or
cross it?", "are these two roots one touch or two crossings?", "are these two cuts one vertex?",
"are these faces coincident?" Each of these is computed again, in a different formula, at every
site that needs it. Each site compares a distance against LINEAR, 1.2×, 2×, 4× or 10× LINEAR, or
against unrelated constants (1e-6, 1e-9, 0.3 mm). Contradictions between the sites surface late as
`InconsistentClassification`, `SplitCrossed` or an invalid body. A retry ladder of thread-local
mode flags then tries the other readings, up to about 8 plans per boolean. Most of the open
failures in `tools/proof/known.txt`, `interface/forge-lang/tests/fuzz_known.txt` and the
reliability baseline come from this one root cause.

The second-largest pattern is **two evaluators of one definition**: the sequential and n-ary
booleans and regens. They are reconciled by falling back from n-ary to sequential on five error
classes.

The other areas are mostly fine. Some are legitimate exact special cases. Others are honest
refusals where a general method is missing (shell or offset with topology change, general
chamfer), which is a gap, not a patch.

Evidence from today: a bounded meta run (`meta seeds 20 count 3 adv 2 only
rigid,mirror,order,boolean,backends,ulp`, 110 parts) found:
- `mirror: redundant vertices differ` on a 2-feature part: a box plus one tilted cylinder. Mirrored,
  it gives V11 E16 instead of V10 E15, so the decision is not invariant under an exact symmetry.
- `backends: nary 1: redundant vertices differ` on 3 parts, the smallest a shaft at 18 features.

The output is in `scratchpad/meta.txt`.

---

## 1. Ranked list of patched areas

Ranking is by how much open failure each explains.

| # | area | class | explains | principled fix | fits constraints |
|---|---|---|---|---|---|
| 1 | Boolean near-tolerance incidence (trim, imprint, apply, place_cuts, share_cuts, coincide) | **b** | M1 (rigid/mirror/order/backends redundant vertices), M5, M9, U1, ball_poles; fuzz_known `InconsistentClassification`; ulp 242/400 on adv near_tolerance 2; baseline near_tolerance, coincident_walls, ball_poles disagreements | decide each incidence **once**, keyed symbolically, with one tolerant rule; derive every related decision from that one (topological consistency + tolerant entities) | yes; likely faster |
| 2 | Boolean retry ladder (`plan` → `plan_retried` → `plan_fenced` → `plan_once`; CAREFUL, CHORDS, ABSORB_HAIR_ROOTS, NO_RUN_ON, FENCES, RETRIED) | **b** (symptom of 1 and 5) | the same failures; every rung is a second reading of an undecided predicate | removed by 1 and 5: no second reading exists if the predicate is decided once | yes; faster |
| 3 | Sequential vs n-ary booleans and regens, reconciled by fallbacks | **b** | known.txt M1–M3, M6, M7, M10, "nary no shapes", nary-chamfer/fillet MissingEdge; baseline 5 × `reject ok`; 3 backends hits today | one face-pair kernel and one incidence store for both; body identity and names defined on the result, not the evaluation order | yes |
| 4 | Contact type (crossing vs touch) by sampling: `crossings_near`, conic "hair roots", tangent-boundary probes | **b** | ball_poles, U1, SplitCrossed retries, tangent family rejects | certified univariate root isolation on g(t) along the curve, plus one tolerance-band rule | yes; Bernstein machinery is already proven in Lean |
| 5 | Knife-edge "blunting" tried only after a failure | **b** | the "knife edge" census row (30 Fusion failures held one); `lost_its_way` fallbacks | always canonicalise a profile under the one tolerance: material thinner than LINEAR is zero thickness | yes |
| 6 | Surface–surface tracing (predictor–corrector with shortened-step retries, stalls, CAREFUL box) | **b** (missing certification) | baseline `torus_torus/horn` error ×3, `cylinder_sphere/centre_on_rim` error ×3, `cone_cone/perpendicular_apex_to_apex` invalid; the silent wrong-mass bug in the census | certified tracing (interval Newton or Krawczyk tube per step), with singular points found first (topology-first SSI) | yes, as a filter after the float step |
| 7 | Screw `track()` run-on, halved when it touches the next turn | **b** (small) | fuzz seed 0 (114 404-point chains, 237 s) → InconsistentClassification | one clearance rule computed in closed form, or trace in the face's own domain | yes |
| 8 | Shell, offset, draft and chamfer by same-topology rebuild; chamfer constructions per curve type | mostly **a** + missing method | Shell Rebuild 8, Chamfer TooLarge 12 / Unsupported 13 (history census) | general offset with topology change (offset surfaces + boolean, or local op with face loss); general ruled chamfer | yes, later |
| 9 | Fillets (context only; another agent covers them) | **b** | Corner 28, Overflow 14, NoBlend 28 | — | — |

These are fine as they are (class a, or rare-case and speed patches the owner accepts). They are
listed in section 3.

---

## 2. Each area in detail

### 2.1 Boolean near-tolerance incidence (rank 1)

**What happens.** One geometric fact is decided several times, by different formulas and
thresholds:

- **On-edge test.** `on_edge` (boolean.rs:283) uses `d ≤ edge.tol`, else the end vertex's tol.
  `coincide` (boolean.rs:4464) projects vertices onto the other body's edge with `≤ vx.tol`.
  `share_cuts` (4492) re-asks `on_edge` for a cut found on another edge.
- **Run-on test.** Is the curve along a straight edge, or crossing it? trim (2265…) decides this
  with `cross().length() ≤ 1e-6` for parallel lines (about line 2450). That is an angular threshold
  unrelated to `ANGULAR = 1e-11` (forge-math/src/tol.rs:16). The test also needs both edge ends
  within `vx.tol` of the curve.
- **Same-place test.** `place_cuts` (4561) clusters cuts greedily by distance ≤ max tol. It then
  merges heads "chained" within **2 × reach** (4573, 4595–4620). This is a distance clustering, so
  it is not transitive and depends on order.
- **Shallow crossing.** A curve leaving an edge at a shallow angle "crosses anywhere along a
  stretch". trim moves cuts when `gap ≤ 1.2 × tol || gap > 0.3` (2846). 0.3 mm is an absolute
  length, justified by "under a third of a mm unless R is over a metre". The census records this
  floor being "lowered in steps against the failing list" from 4 to 1.2 tolerances
  (research/build-failures/README.md, second batch).
- **Probing and other margins.** `clear_step` stops at 2 × LINEAR (6188). `RIM = 10 × LINEAR`
  (279). `4.0 * LINEAR` chord tests (3006, 3023). `lean.abs() ≤ 1e-9`, a raw sine (5930).
  classify.rs:1142 uses `d.abs() > 1e-6`.
- **Elsewhere in the kernel.**
  - `unify::SAME = 0.5 × LINEAR`, with the comment that "on one surface is not transitive"
    (unify.rs:252–254).
  - `check` vertex tolerance at **1.1 × band** (forge-topo/src/check.rs:691). The census says
    1.001 was not enough, and that 137053 is 1.017×.

**The latest instance** is commit 94fc9fb6, on unmerged branch `worktree-agent-a2b4568cfb2a1a516`.
The pair decides a box bottom and a plate top **coincident**, but three other decisions recompared
3D distances to LINEAR, and a few-ulp nudge flipped them. The fix gives each decision a 2× allowance:

- apply: an edge carries its offset as tolerance "up to twice LINEAR" (94fc9fb6:boolean.rs:1981);
- trim: `beside_coplanar` accepts ends within `ends_within(2.0)` (2487–2532), and a stretch along a
  "beside" edge is boundary at `≤ 2.0 * tol` (3074);
- imprint: hit re-asked on the face's surface within `2.0 * LINEAR` (4276).

This moves the cliff from 1× to 2×. It does not remove it. A box at 2·LINEAR − 4e-16 fails the same
way. Its own measurement shows how far from robust this is: on the part's first three features,
**316 of 400 ulp seeds failed before and 242 after**.

**The commit history shows the same churn**, each fix for one failing part:

| commit | change |
|---|---|
| 20ae535c | "Cuts under two tolerances apart are one place" |
| 1ffa7f25 | "Back to chains only for cuts: merging heads under two tolerances broke two regressions" |
| 2a59cde9 | "Shallow crossings past two tolerances move to their neighbour" |
| 9269c3e5 | "shallow floor at 1.2 tolerances" |
| 25af07ac | "a hair chord only past four tolerances from its ends" |
| 3ba9c151 | scoreboard near_tolerance/20 |
| 02b0418d | WHUCAD 100003369 |
| af9747fd | WHUCAD 100003288 |
| 3e17f31e | part 30974 |

The census itself lists about 15 "fixed" rows of this kind: probe on a touch, chain of cuts within
tolerance, curve reaching a smooth edge, rim tangent to a flush face's edge, hair kink, and others.

**Root cause.** There is no single incidence structure. A fact such as "point P lies on edge E" or
"face F and face G are one surface" is re-derived locally wherever it is needed. Each site uses
its own formula: 3D distance, distance after projection, a parameter-space test, an angle. Tolerance
decisions are not transitive (A≈B, B≈C, A≉C), so the sites disagree near the threshold. The
decisions also depend on coordinates, so a mirror or a rigid motion flips them (M1, reproduced
today).

The existing edge and vertex `tol` fields and `Plan::grown` (boolean.rs:740) are the start of a
tolerant model. But the rule "decide once, then derive" is not enforced.

**Principled method.**

1. **Edge–face first.** Compute the intersections of every edge of A with every face of B (and the
   reverse) once, giving vertices keyed symbolically by (edge, face) or (vertex, face). Face–face
   curves then only connect these vertices. They never re-find crossings by intersecting the curve
   with the "across" surface.

   This is the classic B-rep boolean order: Hoffmann, Hopcroft & Karasick, "Robust set operations
   on polyhedral solids", IEEE CG&A 9(6), 1989. It is also Hoffmann, *Geometric and Solid Modeling*,
   1989, ch. 4. The consistency then holds by construction: both faces of an edge see the same cut.
   trim now gets that effect through "for both of its faces alike" (boolean.rs:353–356),
   `share_cuts` and `coincide`.
2. **Topological consistency.** Every predicate is evaluated once per symbolic tuple and stored in
   a ledger. Related decisions are derived from it, never re-measured: if F ~ G is coplanar and edge
   e lies on F, then e's relation to G follows. See Sugihara & Iri on the topology-oriented approach
   (e.g. "Construction of the Voronoi diagram for one million generators in single-precision
   arithmetic", Proc. IEEE 80(9), 1992), and Milenkovic, "Verifiable implementations of geometric
   algorithms using finite precision arithmetic", Artif. Intell. 37, 1988.
3. **Tolerant entities, one rule.** Entities x and y are incident iff dist(x, y) ≤ tol(x) + tol(y).
   The rule is applied only when an incidence is created. Merges go by union–find on *decided
   incidences*, and the merged vertex's tolerance grows to cover its members, up to a fixed cap.
   Beyond the cap the boolean refuses with a location (never a wrong result).

   This is Parasolid's tolerant modelling (Jackson, "Boundary representation modelling with local
   tolerances", ACM Solid Modeling '95). See also Segal, "Using tolerances to guarantee valid
   polyhedral modeling results", SIGGRAPH '90. It replaces place_cuts' 2× chaining, unify's 0.5×,
   check's 1.1× and 94fc9fb6's 2× allowances with one rule.
4. **Why symbolic perturbation does not fit here.** Edelsbrunner–Mücke, "Simulation of Simplicity",
   ACM TOG 9(1), 1990 perturbs coincidences away. Here coincidences are intended (a user's flush
   faces must merge), so SoS would create slivers. Use SoS only for leftover tie-breaks with no
   geometric meaning: order of equal parameters along an edge, which copy of a seam.

**Constraints.** These are combinatorial changes on f64 values, using the existing filtered
predicates (forge-math/src/predicates.rs). Results stay bit-identical across native and wasm
because decisions are keyed by names, not arena order. It should be no slower, because crossings are
computed once instead of per face pair, plus retries. The cap-then-refuse rule keeps "never a wrong
result".

**Lean specification (owner's requirement).**

1. *Specification over the reals.*
   - Exact inputs: the output point set is the regularised Boolean of the input point sets, as a
     valid manifold B-rep. If that set is non-manifold, the output is `ZeroThickness` at a point of
     the non-manifold locus.
   - Tolerant inputs: the output is valid, and lies within Hausdorff distance c·LINEAR of the
     regularised Boolean of *some* exact realisation consistent with the incidence ledger.
2. *Finite predicates to branch on, each a sign in {−, 0, +}, with 0 meaning "within tolerance":*

   | predicate | meaning |
   |---|---|
   | `side(v, F)` | vertex against face surface |
   | `on(v, e)` | vertex on edge |
   | `cross(e, F)` | edge against face: sign change, touch, along |
   | `coinc(F, G)` | same surface, same or opposite sense |
   | `order(p, q; e)` | order of points along an edge |
   | `inside(p, F)` | point in face, against its loops |

   For planar input every one is an orient3d/orient2d sign, or a sign of a low-degree polynomial in
   the coordinates.
3. *Per-branch obligations.*
   - (i) Ledger closure: the stored signs are closed under the derivation rules (transitivity of 0
     through union–find; `coinc(F, G) ∧ e ⊂ F ⇒ side(ends(e), G) = 0`).
   - (ii) Arrangement validity: for **any** closed, consistent sign assignment, the face splits are
     planar subdivisions and the classification flood has no conflict. This makes
     `InconsistentClassification` provably unreachable.
   - (iii) The output satisfies `forge_topo::check` (Euler, edge uses, faces on their surfaces within
     their tolerances).
   - (iv) Tolerance growth stays within the cap, or the boolean refuses.
4. *Floating-point bridge.* Each predicate is computed with a certified error bound δ: the Shewchuk
   filter, or an exact expansion for polynomials.
   - **One rule for undecided signs:** |v| ≤ ε_tol − δ gives 0; |v| ≥ ε_tol + δ gives the sign of v.
   - Inside the band, the sign is chosen once, for example by deterministic SoS on the symbolic key.
     It is stored, and every later question derives from the stored answer.
   - Theorem shape: correctness holds for *every* consistent assignment. So a ulp nudge may change
     the topology but never produces an invalid body. The `ulp` property should then test
     "valid either way", not "same".
   - The repo already has a rounding model to build on (research/loft-ssi/shipped/LoftMeet/Float.lean).

**Proof difficulty.**
- Planar faces, line edges: **medium**. Polynomial predicates, finite enumeration of sign patterns
  per face pair, HHK-style arrangement argument.
- With quadrics: **high**. Algebraic degree, and the predicates over conic edges need root
  isolation (see 2.4).
- With procedural surfaces: **very high**. It needs certified tracing (2.6).

### 2.2 The boolean's retry ladder (rank 2)

**Evidence.**
- Flags: boolean.rs:853–870. `plan` retries on `Internal` when either body has a torus or a
  procedural surface, with `CAREFUL` (889–907).
- `plan_retried` (910–947) catches `InconsistentClassification | SplitCrossed`. It sets `RETRIED`
  and tries `(absorb, chords)` in two orders, then `NO_RUN_ON`.
- `plan_fenced` (949–962) reruns with `FENCES` when a closed curve crossed springs an odd number
  of times.
- `general` (584–586) re-checks a retried body and returns `Lost::RetryInvalid`.
- The flags change decisions deep inside trim and imprint: CAREFUL at 2345, 2585, 2593 and 6279;
  CHORDS at 2603, 2833, 3005, 4411 and 4446; NO_RUN_ON at 2448; FENCES at 2673–2676; and
  `forge_geom::intersect::ABSORB_HAIR_ROOTS` (intersect.rs:559, 580, 1238).
- History: 1e3d61da "tries each reading in turn", 7722020a "two readings, not four … (bracket
  +3%)", 3ba9c151, 06631015 "a pair on a torus lost its way, once more…".

**Classification: b.** Each rung is the other answer to a predicate that sits in the undecided band:
- "two roots a hair apart" = crossing pair or touch;
- "hovering edge" = own edge or not;
- "line along an edge" = run-on or crossing;
- "spring crossing" = shallow or not.

The ladder searches the sign assignments by failure. Searching by failure is unsafe because
accepting a result is not the same as the result being correct (hence `RetryInvalid`). It is also
slower: the failing trials cost extra (7722020a).

**Principled fix.** Same as 2.1 and 2.4. Once each predicate is decided once (stored band rule),
there is no alternative reading to retry. CAREFUL is an accuracy mode: the torus box was too coarse
for the tracer's cells. It is replaced by certified enclosure refinement (2.6), which refines where
the certificate fails instead of on a global retry.

**Lean.** No separate proof. The ladder is deleted when 2.1 and 2.4 land; their obligation (ii)
covers it.

### 2.3 Sequential vs n-ary: two evaluators of one definition (rank 3)

**Evidence.**
- nary.rs is 3524 lines, and `nary_body` alone runs from line 622 to 2800 with its own face-pair
  loop. It imports trim, imprint, lean and crossings_near from boolean.rs (nary.rs:16–20) but
  places, classifies and floods separately. It even keeps its own `UNDECIDED` handling (nary.rs:561,
  1087).
- research/tangent-cuts/README.md: "the original branch changed only the sequential loop".
- Fallbacks to sequential in `nary_regen_on` (forge-model/src/lib.rs:1287–1310): on any
  `Internal`, on `ProbeOnToolEdge`, on `names_missing` (counted by `NAME_FALLBACKS`, 1199–1212), and
  on `lost_its_way` with knife edges. Also `any_touching` → sequential (1278), short runs →
  sequential (1340, 1370), and `may_empty` → one at a time.
- Multi-body: bodies.rs:1070–1095, plus `may_be_empty` (1099). Commits e5e74b40 and ed8abb16 note
  that "an add after a cut goes one at a time".
- Known failures: known.txt lines for `backends: nary 1` and `nary no shapes` (M1, M2, M3, M6, M7,
  M10, nary-fillet-after-pattern, nary-chamfer-after-fillet, test-gen/r14). Baseline:
  `coincident_walls/16`, `near_tolerance/8` and `ball_poles/10, 16, 18` are `reject ok` (the
  sequential path rejects, the n-ary path builds). Today's meta run: 3 backends hits.

**Root cause.** The n-ary boolean re-implements the pair logic rather than sharing one
decision store, so the two paths make different tolerance decisions (2.1). On top of that,
**body identity and naming depend on evaluation order**: a body emptied mid-run, or a face split
into several kept pieces, comes out differently in the two orders.

**Principled fix.**
- One face-pair kernel and one incidence ledger keyed by (feature-face, feature-face). The n-ary
  path is the same decisions, evaluated in parallel and memoised.
- The result's bodies and names are a function of the final regularised set plus lineage. One way
  is to define bodies as the connected components of the result and name faces by their source face
  and split rank. That way, order of evaluation cannot change them.
- `may_be_empty` (a conservative box test) can stay as a cost guard.

**Lean.** *Spec:* n-ary(tools, ops) = fold of the sequential boolean, up to the result's canonical
naming. *Predicates:* the same as 2.1. *Obligation:* each pair decision depends only on its key, not
on the order of evaluation, and the decisions commute. Then n-ary equals sequential, by induction on
the fold. **Difficulty: medium** once 2.1 exists. Before that, it is not provable because the two
paths really decide differently.

### 2.4 Contact type by sampling (rank 4)

**Evidence.**
- `crossings_near` (boolean.rs:4404–4462) takes 64 samples in a window. Samples with
  `|implicit| ≤ 1e-9` count as "dead": that is the raw implicit value, not a distance, so its scale
  depends on the surface. The window widens to 2 × LINEAR only under CHORDS, and "in and out again
  by less than a tolerance" is dropped unless CHORDS is set.
- Conic–quadric "hair roots": intersect.rs:520–580. Research/tangent-cuts/README.md says: "eight
  iterations and 0.05-radian displacement cap … are guards, not a convergence proof", and the
  probe fractions (0.001, 0.01, 0.05) and normal threshold 1e-6 "are search heuristics, not
  bounds".
- forge-bool/src/ssi.rs:439 lists offsets `[0.0, -5.7e-14, -1e-9, 1e-9, -0.9 * LINEAR]`, a test
  sweep that names the cases that broke.
- Known failures: known.txt `ulp: … ZeroThickness Point3 # adv ball_poles … nudged by ulps
  nothing at the touch can decide`. The baseline's `ball_poles` disagreements.

**Classification: b.** It is a non-robust predicate (sampling) plus a mode-dependent rule.

**Principled fix.** Along a curve c(t) and surface S, define g(t) = S.implicit(c(t)), normalised to
a distance (divide by |∇S| or use the signed distance).
- For conic × quadric, g is a trig polynomial of degree 2 (research/tangent-cuts: Q(t) = k0 + …).
  With u = tan(t/2) it becomes a quartic. Isolate its roots with Descartes' rule of signs or a
  Bernstein sign test. See Mourrain & Pavone, "Subdivision methods for solving polynomial
  equations", J. Symb. Comput. 44, 2009, or Lane & Riesenfeld 1981. The repo has this machinery,
  proven: research/loft-ssi/shipped/LoftMeet/Bernstein.lean.
- The *tolerance band* is B = {t : |g(t)| ≤ LINEAR}, a union of intervals that root isolation on
  g ± LINEAR certifies.
- **One rule:** each component of B is a single contact. It is a crossing iff g has opposite signs
  at the component's two ends, a touch if the signs are the same, and "along" if the component
  covers the whole range.
- This one rule replaces both readings of ABSORB_HAIR_ROOTS and CHORDS.

**Constraints.** Only IEEE ops on polynomial coefficients. Deterministic. Cheap, because a quartic
replaces 64 samples plus bisection.

**Lean.**
1. *Spec:* the contacts of a conic with a quadric, as components of B, with their types.
2. *Predicates:* the signs of the Bernstein coefficients of g − LINEAR and g + LINEAR on each
   subinterval, and the sign of g at component ends.
3. *Obligations:* (a) the subdivision terminates at a fixed depth or reports "undecided";
   (b) when the coefficients all have one sign, the interval has no root (already proven in
   LoftMeet); (c) the type rule is exhaustive over the end-sign patterns.
4. *FP bridge:* coefficients come with a rounding bound, as in Float.lean. An undecided band end
   goes to the stored-sign rule of 2.1.

**Difficulty: medium-low.** This is the most reusable of the existing Lean work.

### 2.5 Knife-edge blunting tried only after a failure (rank 5)

**Evidence.**
- `step` (forge-model/src/lib.rs:563–583), `careful` (612–638) and `in_turn` (641–659) try every
  extrude plain, then blunted if `lost_its_way`, then all blunted. `rebuilt_blunted` and
  `all_blunted` are at 662–706.
- bodies.rs has its own `careful` and `all_blunted` (340–390).
- `Profile::blunted` is at forge-ops/src/profile.rs:815–830.
- The `lost_its_way` test (lib.rs:708–716) is any ZeroThickness, Internal or Unsupported refusal.
- Census: "Blunted where the sides part, only when the boolean refuses the exact tool … partly:
  fails when a later feature is what trips on it".

**Classification: b.** The same input has two representations, and which one is used depends on
whether a downstream boolean fails. That is history-dependent, and it can mask a real bug as "lost
its way".

**Principled fix.** Canonicalise once, at profile time, under the one tolerance. A stretch where
two sides are within LINEAR is zero-thickness material by definition (tol.rs's "one tolerance"), so
always end the tip where the sides part by more than LINEAR. The blunted tip is within LINEAR of the
exact tip, so the "tip that stays can meet other faces exactly" case still meets them within
tolerance.

**Lean.** *Spec:* the canonical profile is within Hausdorff distance LINEAR of the input, and no
stretch of it is thinner than LINEAR. *Predicates:* `doubles_back(seg_i, seg_{i+1})`, and the parting
distance against LINEAR. *Obligations:* idempotence; the result is a valid simple region or a
reported touch. *FP bridge:* distances between lines and arcs in closed form, with filtered bounds.
**Difficulty: low.**

### 2.6 Surface–surface tracing without certification (rank 6)

**Evidence.**
- forge-geom/src/intcurve.rs:815–870: a predictor–corrector where a step that "jumped" or
  "stalled is retried shorter"; turn retries down to `TURN_LEAST = 1/1024` with
  `TURN_COS = 0.866` (760–761); stalls noted via the thread-local `STALLS` (944–960).
- `ssi::searched` refuses any apex in the region (ssi.rs:170–180).
- `CAREFUL` exists because the torus box set the tracer's cells too coarse (boolean.rs:862–865).
- d9cf4adf: "a root count that changes with no event between means an event was missed".
- Census, mass: "Newton from a far seed landed on the other arm, as exact as the right one, so no
  gap and no error estimate saw it".
- Baseline: `torus_torus/horn/*` error, `cylinder_sphere/centre_on_rim/*` error,
  `cone_cone/perpendicular_apex_to_apex/Add` invalid, `plane_torus/standing_tangent_top_inside/Keep`
  invalid.

**Classification: b (a missing method).** Marching gives no guarantee of staying on one branch, or
of finding every loop and singular point.

**Principled fix.**
- **Topology first:** find the critical and singular points (tangencies, apexes, turning points)
  before marching. See Grandine & Klein, "A new approach to the surface intersection problem",
  CAGD 14, 1997, and Sederberg & Meyers, "Loop detection in surface patch intersections", CAGD 5,
  1988.
- Then march between them with **certified steps**. Each step's tube is proven by interval Newton
  or the Krawczyk operator (Neumaier, *Interval Methods for Systems of Equations*, 1990) to contain
  exactly one branch. See also Beltrán & Leykin, "Certified numerical homotopy tracking", Exp.
  Math. 21, 2012.
- forge-math already has `interval.rs`. Run the float step first and certify after it, refining
  only where the certificate fails. That keeps the cost close to today's.

**Lean.** *Spec:* the traced polyline lies within ε of exactly one branch of {a = 0} ∩ {b = 0} in
the region, and every branch is covered. *Predicates:* the Krawczyk inclusion test per box; the sign
of det(∇a, ∇b, n) for singular points. *Obligations:* uniqueness in each tube, and the tubes chaining
end to end. *FP bridge:* interval enclosures of every surface type, including procedural ones.
**Difficulty: high**, mostly in enclosing the procedural surfaces.

### 2.7 Screw `track()` run-on (rank 7)

**Evidence.**
- forge-geom/src/screw.rs:244–290 builds the track. w is
  `min(0.25·len, r/8, 0.25·gap, 0.25·inward)` (395–398).
- At 411–418 a branch halves the run-on when `TAU·|b| − hh.width() < w`.
- Commit 0cf7a1ad, prompted by fuzz seed 0: 114 404-point chains, 237 s, then
  InconsistentClassification.

**Classification: b (small).** The face's implicit is defined through an arbitrary extension, and
its size is set by fractions plus a branch for one configuration.

**Principled fix.** Either (i) compute the clearance between this track and the next turn's in
closed form, and use one rule, run-on = w = ½·clearance, for every face; or (ii) intersect in the
face's own (u, v) domain with boundary events, so that no extension matters.

**Lean.** *Spec:* the extended zero set is a C¹ surface with no self-contact within w. *Predicates:*
the clearance sign. *Obligation:* the separation lemma. **Difficulty: medium-low**, but the result
is narrow.

### 2.8 Shell, offset, draft and chamfer by same-topology rebuild (rank 8)

**Evidence.**
- forge-ops/src/rebuild.rs:22–60 recomputes vertices by Gauss–Newton "near where it was"
  (`solve_point`) and errors with `Collapsed` or `Vertex` on topology change (152–178).
- The chamfer surfaces are a list of closed forms by edge type: line on plane or cylinder; circle on
  axis-aligned plane, cylinder, cone, sphere or torus; planar spline. Anything else is
  `Unsupported` (forge-ops/src/chamfer.rs:920–1060).

**Classification.** The closed forms are **a**: each is the exact rolling-plane or cone chamfer for
its case. The refusals are a **missing general method**, not a patch: offset with topology change,
like ACIS or Parasolid local operations that let faces vanish, or offset surfaces plus a general
boolean; and a general ruled chamfer between the two foot curves. These are not on the
failure-explaining path, so do them after phase 3.

### 2.9 Fillets (context)

`fillet.rs` is 8036 lines. It has a retry budget (`WALK_LEFT`, 2828–2840),
`fillet_cliff::retry` as the last resort (447, 452, 587) and corner rules (`ball_setbacks`, 5673).
The census open rows are Corner 28, Overflow 14 and NoBlend 28. This is the same pattern; it is
covered by the other agent.

---

## 3. Legitimate or acceptable (keep)

- **Shewchuk predicates** with documented bounds (forge-math/src/predicates.rs:1–30; research/predicate-bounds).
- **The imprint fast path** with `CLEAR = 10 × LINEAR`, which hands anything closer to the general
  boolean (imprint.rs:26) and has a `Check` mode. This is a conservative filter for speed. Fine.
- **CDT point location**: if the visibility walk cycles, it falls back to a scan
  (forge-mesh/src/cdt.rs:138). Walk cycling in constrained triangulations is a known property
  (Devillers, Pion & Teillaud, "Walking in a triangulation", IJFCS 13, 2002). Fine.
- **`ssi` circle–circle tangency** `h² ≤ 2·LINEAR·r` (ssi.rs:292, 384) is the sagitta relation
  h² ≈ 2rδ, so it is exactly "within LINEAR". This is class a, but it should feed the ledger
  rather than be re-asked.
- **`crossing_touch`**: crossing cylinders at r1 + r2 within tolerance give a point (ssi.rs:280–300;
  08d82caf). This is a real degenerate configuration, decided once. Class a.
- **`may_be_empty`** (bodies.rs:1099) is a conservative box guard. It is pessimistic for intersect
  (it always says true), which costs time but is never wrong.
- **The sketch solver**: Levenberg–Marquardt with damping (forge-sketch/src/solve.rs:397–610).
  Standard; no retries or perturbation.
- **Memo caches** (screw.rs:1128, ssi.rs:161, swept.rs:112/170) are keyed by input bits, so they
  are deterministic speed patches.
- **Not audited in depth**: forge-sheet/src/region.rs uses `NEAR = 2 × TOL` and several 2× and 4×
  TOL tests (42, 412–929) in its own 2D arrangement. It is likely the same pattern in 2D, but with
  little failure evidence, so it is low priority. The same incidence-once rule applies, with exact
  orient2d.

---

## 4. Phased plan (riskiest and most valuable first)

**Phase 0: measure (no behaviour change, days).**
- Add a debug-only *decision ledger*: log every incidence decision under its symbolic key and
  assert that the same key never gets two answers. This turns late `InconsistentClassification`
  failures into early "two decisions about one fact disagree" reports with a location.
- Count how often each rung of the retry ladder fires (RETRIED, CAREFUL, CHORDS, ABSORB, NO_RUN_ON,
  FENCES) over the suite, fuzz, reliability and Fusion parts. That ranks which rungs matter.

**Phase 1: incidence once, planar first (highest risk and value).**
- In trim and imprint: compute edge–face intersections first, keyed (edge, face); make place_cuts
  a union–find over decided incidences with one tolerant rule and a cap; propagate coplanar
  decisions.
- Start with plane/plane pairs and the adversarial families near_tolerance, coincident_walls,
  coplanar_stack and plane_plane. Planar parts go through the new path; the others keep the old
  one.
- In parallel, run the **Lean pilot** (section 5).
- Gates: reliability baseline, `meta known`, bits on native and wasm, the callgrind gate.
- Change the `ulp` property to "valid either way".

**Phase 2: conics and quadrics.**
- Certified contact classification (2.4) feeds the same ledger.
- Delete ABSORB_HAIR_ROOTS, CHORDS and NO_RUN_ON one by one as their Phase-0 trigger counts reach
  zero. Then delete `plan_retried`.

**Phase 3: one kernel for sequential and n-ary.**
- nary_body uses the same pair kernel and ledger.
- Define body identity and names on the result.
- Remove the fallbacks in lib.rs:1287–1310 and bodies.rs:1070–1095.

**Phase 4: canonicalise profiles.**
- Always blunt knife edges; remove `careful`, `in_turn`, `all_blunted` and `lost_its_way` retries.

**Phase 5: certified tracing.**
- Topology-first SSI with Krawczyk-certified steps; CAREFUL and FENCES go.
- The screw clearance rule (2.7).

**Phase 6: missing general methods.**
- Offset with topology change; general chamfer. Fillets per the other agent.

---

## 5. Lean pilot recommendation

**Pilot: the boolean's incidence and classification for planar faces (plane × plane pairs,
straight edges), under the tolerant one-rule ledger.**

Why this one and not the fillet corner rule:
- **Most failure.** The open classes (`InconsistentClassification` in fuzz_known, M1/M5/M9/U1 in
  known.txt, the near_tolerance, coincident_walls and coplanar_stack families, the
  `plane_plane/vertex_on_vertex/Add` invalid) are all incidence decisions. The latest patch
  (94fc9fb6) is a purely planar case. The 2-feature mirror failure found today needs a cylinder,
  but its face splits are planar.
- **Bounded.** Every predicate is an orient2d/orient3d sign or a low-degree polynomial in the
  coordinates. Sign patterns per face pair are finite and can be enumerated. HHK 1989 and Segal
  1990 give the arrangement argument.
- **A rounding model already exists.** research/loft-ssi/shipped/LoftMeet/Float.lean and
  check-local/lean (no `sorry`) show the toolchain and the style. The check-local proof
  (`full_of_local`) already covers the validity checker the obligation (iii) targets.
- **The fillet corner rule is a poor first pilot.** Its specification is not settled: the rolling-
  ball envelope at an n-edge vertex with mixed convexity, and which G¹ patch exists. Its
  configuration space is open-ended, and the open rows (Corner 28) come from missing rules, not
  inconsistent ones. Prove the corner rule after its definition is fixed by the fillet work.

**Pilot statement.**
- For any two closed planar polyhedra and any sign assignment to {side, on, cross, coinc, order,
  inside} that is closed under the derivation rules, the algorithm's face splits are valid planar
  subdivisions.
- The flood classification is conflict-free.
- The output passes `check`, or the algorithm refuses with ZeroThickness exactly when the
  regularised result is non-manifold.
- When the signs come from f64 through the filtered band rule, the output lies within c·LINEAR of
  the regularised Boolean of an exact realisation.

**Proof difficulty by area:**

| area | difficulty |
|---|---|
| 2.5 knife-edge canonicalisation | low |
| 2.4 contact classification (conic × quadric) | medium-low (reuses Bernstein.lean) |
| 2.7 screw clearance | medium-low |
| 2.1 incidence, planar (pilot) | medium |
| 2.3 n-ary = sequential, given 2.1 | medium |
| 2.1 with quadrics | high |
| 2.6 certified tracing | high (procedural enclosures) |
| 2.8 offset with topology change | high |
| fillet corners | high (specification not yet settled) |

Note: 94fc9fb6 is on an unmerged branch (`worktree-agent-a2b4568cfb2a1a516`). Its 2× allowances
are not in HEAD. Merging it would add three more independent 2× decisions; the alternative is
phase 1.

---

## 6. An exact reference boolean (source of truth, outside the kernel)

**Where.** `verify/forge-exact/`: a Lean 4 package, compiled to a native executable by `lake build`.
Lean's `Nat` and `Int` are GMP-backed in the compiled runtime, and Mathlib or Batteries provide
`Rat`. A Rust harness in verify/ exchanges parts with it as JSON, the way `tools/occt` and
`oracle` already do. kernel/ gains no dependency. Precedents for the planar stage and for
closed-form quadrics already exist in research/ (`check-local/lean`, `loft-ssi/shipped`).

**Definition (over the reals).** Inputs: solids A and B as B-reps whose carriers are the kernel's
own surfaces. Every f64 the kernel stores is a dyadic rational, so the input is exact as given.
- Let S be the set of all carrier surfaces of both bodies. Take the cell decomposition of R³ by the
  arrangement of S, restricted to the hull of both bounding boxes grown by 1.
- Classify each open cell c as in or out of A, and of B, using one interior point.
  Membership is constant on c because ∂A ∪ ∂B ⊂ ∪S. This is the key lemma.
- Keep c iff op(in_A(c), in_B(c)).
- The result is the regularisation (closure of the interior) of the union of the kept cells.
- Its boundary is made of the facets between a kept cell and a dropped cell, with coplanar
  (cosurface) facets merged.

This is the CGAL Nef-polyhedron formulation: Bieri & Nef; Hachenberger, Kettner & Mehlhorn,
"Boolean operations on 3D selective Nef complexes", CGTA 38, 2007. EXACUS covers the
curved-surface counterpart: Berberich, Hemmer, Kettner, Schömer & Wolpert, "An exact, complete and
efficient implementation for computing planar maps of quadric intersection curves", SoCG 2005; and
Schömer & Wolpert, "An exact and efficient approach for computing a cell in an arrangement of
quadrics", CGTA 33, 2006.

**Verdicts.** The reference answers one of:
- `Solid(R)`;
- `Empty`;
- `NonManifold(locus)` (the regularised set is not a 2-manifold boundary; the kernel must say
  ZeroThickness there);
- `Ambiguous(chain)` (see 6.1).

The fuzz comparison accepts a kernel result iff all three hold:
1. The verdict class matches (the kernel's ZeroThickness ↔ NonManifold, Empty ↔ Empty).
2. For a solid, the Hausdorff distance between the boundaries is ≤ c·LINEAR, and the volumes agree
   to that slack.
3. The topology matches (shells, Betti numbers, faces per carrier class) after the kernel's
   sub-LINEAR features are collapsed.

On `Ambiguous`, any valid kernel result within c·LINEAR of *either* reading passes, and so does a
located refusal.

### 6.1 The canonical tolerance rule (what the kernel's per-case code should implement)

The reference applies this snap to the dyadic inputs before the exact operation. The kernel's
scattered tolerance code (2.1) should be one implementation of it.

1. **Carriers first (dimension 2).** Two carriers S, T, of the same body or of different bodies,
   are *tolerance-equal* iff they are of one kind and their canonical parameters agree within the
   tolerance over the region R where their faces' boxes, grown by LINEAR, overlap:
   - planes: max over the corners of both faces' extent (the hull of their grown boxes) of
     |dist(x, T)| for x on S ≤ LINEAR, R itself not empty. (Measured over R alone, the test merges
     any two faces that meet at an edge: R is then a slab 2·LINEAR across round the edge, where
     each plane is within LINEAR of the other. The reference first did this, and under it every
     solid's faces fell into one class; see exact-comparison.md.)
   - cylinders and cones: the axis lines within LINEAR over R's extent along the axis, and the
     radius or half-angle within LINEAR at R's ends;
   - spheres and tori: centres, axes and radii likewise.

   This is the existing `ssi::nearly_flush` and `coincident` idea, made the definition.

   Take the **transitive closure** (union–find). If a class's spread (the largest pairwise
   distance over R) exceeds a cap K·LINEAR (K = 2 is suggested; measure), the verdict is
   `Ambiguous`. Otherwise every member is replaced by the class representative: the carrier of the
   least (body, face name). Names are invariant under motion and mirroring, so the choice does not
   depend on coordinates.

   Same or opposite sense is kept per face.
2. **Re-derive the topology exactly.** Each body's vertices and edges are recomputed exactly from
   its snapped carriers. For planes, a vertex is the rational solution of its three planes. An
   edge whose ends become closer than LINEAR (MIN_LENGTH) collapses, by the same rule at
   dimension 0 (vertex classes by union–find with the same cap).

   Per-edge import tolerances (`edge.tol`) widen only that edge's tests, never the rule.
3. **Then exact.** Run the arrangement boolean on the snapped, now exactly consistent inputs. A
   cross-body vertex within LINEAR of the other body's carrier, whose own carriers were *not*
   snapped (a corner touching a face at a tilt), is **not** snapped. It is an exact sub-tolerance
   crossing. After the operation, the output's sub-LINEAR features (edges shorter than LINEAR,
   slabs thinner than LINEAR) are reported with their locus. The kernel must then either collapse
   them validly within c·LINEAR or refuse with ZeroThickness there. Silently making an invalid body
   is the one forbidden answer.
4. **Order of decisions:** carriers, then edges, then vertices. Each decision is made once and
   inherited by everything bounded by it. This is the ledger of 2.1. With it, the 94fc9fb6 case is
   decided by step 1 alone. The box bottom and the plate top are one carrier, so the notch-wall
   edge lies exactly in it. No 2× allowance is needed anywhere.

This rule is deterministic and independent of evaluation order. It is invariant under exact
symmetries, which settles the M1 mirror failure by definition. Under inexact motions it can flip
only at the snap threshold, and then both sides are valid answers.

**G5b merge rule vs Snap tolEq over extent (a known divergence).** The G5b fix (branch
claude/repo-review-mbdrlj-fix-draft, `ssi::turned_flush`) merges two planes turned past NEAR
when, over the faces' boxes grown by LINEAR once and overlapped (wider than 4·LINEAR on two axes),
each plane is within LINEAR of the other at the projection of every corner. That is the rule as the
first reference specified it, region plus `wide`, not Snap `tolEq` as it now stands, which compares
the planes over `extent` (the hull of both grown boxes). Under `extent` the drafted lid's end and
the block's side would not merge: across the block's whole side the end lies about 10·θ off. The
answers still agree. Asked directly (the lid and block as polyhedra, all three ops), the reference
under `snap` gives a solid at the exact volume for the lid at 0.006°, 0.05°, 0.1° and 0.2°, and the
kernel builds that volume each time (the merged result is 50·d under exact, at most 3.3e-7
relative: the block's side keeps the dipped corner, d off its plane, as a vertex). Under `strict`
the reference calls the G5b band ambiguous. They part at a knife edge just under d = LINEAR,
d ∈ (LINEAR·(1 − θ), LINEAR]: the drafted lid at 0.08099°–0.08103°, `cap` at θ from 1.998e-3 to
2e-3. There the region reaches LINEAR past the dipped corner, where the gap is d + θ·LINEAR >
LINEAR, so the pair is kept apart, while the corner itself is decided on the other face's edge. The
kernel refuses (UntouchedOnOther, or ZeroThickness at d = LINEAR) where the reference answers solid.
verify/forge-coverage/tests/g5.rs `g5_knife_edge_at_linear` pins it. The decide-once rewrite takes
`extent` as its definition.

### 6.2 Staging and the exact-arithmetic pieces each stage needs

| stage | surfaces | pieces | Lean difficulty |
|---|---|---|---|
| **1** | planes (straight edges), over ℚ | rationals (GMP); 3×3 determinants; exact orient3d; arrangement by BSP or plane-by-plane cell splitting; exact point-in-polyhedron (ray with a symbolically perturbed direction); facet merging; union–find snap of 6.1 | **medium**: convex cells, the constant-membership lemma, regularisation; Mathlib has affine subspaces and convexity |
| **2** | + cylinders, cones, spheres (the kernel's analytic quadrics; circles, ellipses, lines, parabolas, hyperbolas as edges) | implicit quadrics with **rational coefficients** taken from the stored f64 frames without normalising (a cylinder is \|d\|²\|p−o\|² − ((p−o)·d)² − r²\|d\|² = 0, exact); **real algebraic numbers** (square-free integer polynomial + isolating interval); **root isolation** (Descartes / Vincent–Collins–Akritas, or Sturm sequences); **resultants and subresultants** to project pairs and triples of quadrics (cylindrical algebraic decomposition, Collins 1975), or exact quadric-intersection parametrisation (Dupont, Lazard, Lazard & Petitjean, "Near-optimal parameterization of the intersection of quadrics", J. Symb. Comput. 43, 2008); sign of a polynomial at an algebraic point (refine intervals, or exact arithmetic in ℚ(α)) | **high** but bounded: the degree is at most 4 per curve; the Bernstein and Descartes lemmas can reuse LoftMeet/Bernstein.lean |
| **3** | + tori; offsets of the analytic class (closed: a sphere, cylinder, cone or torus offsets to the same kind) | degree-4 implicits; intersection curves up to degree 8 (torus–quadric) and 16 (torus–torus); multivariate resultants, singular-point and branch-point detection on the curves; high-degree algebraic numbers | **very high**; the performance of the reference is the risk, not its correctness |
| **4** | procedural surfaces (sweeps, lofts, screws, blends): transcendental through dmath trig, so not exactly decidable | a **certified** reference instead of an exact one: interval enclosures of each surface, Krawczyk-certified curves (2.6), and an `Undecided(locus)` verdict where certification fails | high; the reference becomes "certified or undecided", never wrong |

Note on angles: where the kernel derives a parameter through dmath (sin or cos of a cone angle, a
rotated frame), the reference takes the **stored f64 result** as exact. So it decides the exact
surface the kernel holds, and dmath's correctness stays outside its scope.

### 6.3 What stage 1 would already adjudicate

From the current evidence, these are planar or block-only cases:
- **tools/proof/known.txt**:
  - `ulp: fails: Boolean Internal "face # U1`, a tool face a hair over the tolerance from a
    coplanar face (adv near_tolerance);
  - `backends: nary no shapes: … Unsupported "region # M7`, adv near_tolerance 1, a sliver two
    tolerances tall. Stage 1 decides whether the sliver is a NonManifold, `Ambiguous` or a solid
    verdict, so the kernel's answer can be judged;
  - the planar instances of M1 `redundant vertices differ`. After the snap, the reference has no
    redundant vertices, so the property becomes "kernel topology = reference topology".
- **The 94fc9fb6 case** (adv near_tolerance 2, seed 482, box on a notched plate): 316 of 400 ulp
  variants failed before that fix and 242 after. Stage 1 gives every variant one verdict.
- **The reliability baseline**:
  - `plane_plane/vertex_on_vertex/Add invalid`;
  - the 12 `plane_plane/*` rejects (is ZeroThickness or Empty correct, for example `edge_on_edge`
    and `identical/Cut`?);
  - the block seeds of `near_tolerance/8`, `coincident_walls/*` (10 rejects, plus the
    `coincident_walls/16` reject/ok disagreement), `coplanar_stack`, `through_vertex`,
    `exact_patterns` and `remove_all`. Their doc comments describe blocks; their rod and hole seeds
    wait for stage 2.

  This replaces the OCCT oracle's judgement for these, which the census notes "still needs judging".

Stage 2 is needed for:
- `fuzz_known.txt` `InconsistentClassification`, which is on a hole and a rib, so a cylinder;
- the M1 mirror repro found today (box + tilted cylinder);
- the `shaft` backends repro (revolve: cylinders and planes);
- `ball_poles` (spheres, including `ulp: … ZeroThickness Point3`);
- the tangent family.

Not adjudicated by any stage: M11 and the nary MissingEdge and NoSuchEdge failures (naming, not
geometry), and M8 (mesh certification).

**Use in CI.** Run stage 1 as a nightly property, `exact: kernel ≡ reference under 6.1`, over the
planar adversarial families and the planar fuzz histories. Every disagreement it shrinks is a
kernel bug with a definite right answer. Phase 1 of section 4 is then "make the kernel implement
6.1", with this property as its gate.

---

## 7. Scope of "everything is polynomial": every surface and curve kind the kernel has

Sources: `Surface` (forge-geom/src/surface.rs:95–103) and `Curve` (curve.rs:95–105) enums, and
every `impl Procedural` / `impl ProceduralCurve` in forge-geom.

The reference decides **point sets**, not parametrisations. A kind counts as algebraic if its
image is the zero set (or a semialgebraic piece) of a polynomial.

Degrees below are implicit degrees. "≤" is the generic bound (Bézout, or 2mn for a rational
tensor patch of bidegree (m, n)); particular cases often factor lower. Degree sets the cost of the
reference: resultant sizes and algebraic-number degrees grow with the products of these numbers.

### 7.1 Algebraic: exact reference, stages 1–3

| kind | where | implicit degree |
|---|---|---|
| Plane | surface.rs | 1 |
| Cylinder, Cone, Sphere | surface.rs | 2 |
| Torus (ring, horn, spindle, lemon/apple) | surface.rs | 4 |
| offsets of the above (shell, thicken) | forge-ops/src/shell.rs:851 | the same kind and degree: natural quadrics and tori are closed under offset |
| `Offset` of a procedural surface | blend.rs:156 | algebraic iff the base is; the degree grows sharply (a general ellipsoid's offset is already degree 12) |
| `Extruded` (a plane rational spline of degree d, swept straight) | spline/extruded.rs | ≤ d (an ellipse wall: 2) |
| `Spun` (a rational section of degree d, revolved) | spline/spun.rs | ≤ 2d (r = √(x²+y²), squared out) |
| `Swept` on a **planar** spine | spline/swept.rs:1–7 | algebraic (the frame is rational in T/\|T\|, so a square root): ≤ about twice the rational bound; certified tier at first |
| `Pipe` (ball blend about an exact spine), `Tube` (constant distance from a rim) | blend.rs:1659, tube.rs | canal surfaces: algebraic when the spine is. Spine = the intersection of two offsets: degree ≤ 4 for two natural quadrics, 8 for quadric × torus. Eliminating the spine point gives a bound in the tens (a circle spine gives the torus, 4) |
| `HolePatch` (corner) | corner.rs:2193 | a rational blend Σ R_i B_i of algebraic sides: algebraic in principle, degree impractical → certified tier |
| `Ruled`, `Skinned` with **rational rails only** (line, spline, parabola) | ruled.rs, skinned.rs | Ruled: rational bidegree (max(a, b), 1) → ≤ 2·max(a, b). Skinned: per span (d, 3) → ≤ 6d with common denominators |
| Line; Circle, Ellipse, Parabola, Hyperbola | curve.rs | 1; 2 (a plane ∩ a quadric) |
| `SplineCurve` (rational B-spline as Bézier pieces) | spline.rs:1–4 | d per piece |
| `Intersection` / `ChordCurve` (a re-parametrised intersection) | intcurve.rs, smooth.rs:377 | ≤ d1·d2: conic 2, quadric·quadric 4, quadric·torus 8, torus·torus 16 |
| `Spring` (the ball's foot on a surface) | blend.rs:1052 | algebraic when the spine and surface are (foot equations are polynomial) |

**A finding that affects lofts.** `Ruled` matches its rails "by the fraction t along" each
(ruled.rs:21–44), and an `Arc` rail is parametrised by **angle**: start + t·sweep. A loft from a
circle to a line or a spline therefore pairs cos(α + βt) with a polynomial in t. The image is
**transcendental**, not algebraic. Two arcs are algebraic only if their angular rates are equal (or
in a small rational ratio). `Skinned` matches its sections the same way.

If the owner wants lofts inside the exact reference, the **definition** must change: parametrise
arcs rationally, for example as a rational quadratic Bézier, as Parasolid- and NURBS-style lofts
do. That changes which points are matched, so it changes the geometry. It is a definition decision,
to be made once, not a numeric fix.

### 7.2 Transcendental

| kind | where | why |
|---|---|---|
| `Screw` (a section line or arc under a screw motion with drift a and rise b) | screw.rs:657 | z = b·atan2(y, x)-type relations. With a = b = 0 it is a revolution, so algebraic (plane, cone, cylinder, torus) |
| `Helix` and coaxial `Round` curves | screw.rs:857, screw/coaxial.rs:368 | the screw's helices, conical helices, flat spirals |
| threads, coils, `Swept` along a helix | spline/swept.rs `Spine::Helix` | the same screw motion |
| `Wrap` curves, wrapped ruled walls (text on a cylinder, a hole across a sheet-metal bend: a right conoid) | wrap.rs:572 | the angle s/unroll appears inside cos and sin |
| `Swept` along a **spatial** spine (rotation-minimising frame) | spline/swept.rs:5–7, 75–110 | R' = [w]R is an ODE. Note that the kernel's surface is **its Magnus table** ("tabulated by Magnus steps at build and stepped from the nearest entry"), not the exact RMF |
| `Ruled` and `Skinned` with an `Arc` or `Wrap` rail | ruled.rs:21–44 | the angle matching of 7.1 |
| `VarBlend` | varblend.rs:2499 | the radius law runs along a non-rational spine parameter: defined by solving, not by a formula |
| `Fence` surfaces (screw.rs:1028, blend.rs:1208, varblend.rs:2777) | — | search aids near springs, never boundary: the reference ignores them |

### 7.3 How the reference handles the transcendental kinds

1. **The cylindrical-cover reduction (exact, and covers the common cases).** Write points as
   (ρ, φ, z) with φ **unwrapped**, a real coordinate on the universal cover of the axis
   complement. This is the reduction `Screw::nearest_turn` already performs: (ρ − a·u, z − b·u)
   with u = φ + 2πm (screw.rs:468–493).

   In those coordinates the following are all **polynomial in (ρ, φ, z)**:
   - every screw face with that axis: G(ρ − aφ, z − bφ) = 0, with G the section line or arc, of
     degree 1 or 2;
   - helices;
   - wrapped curves and walls on a cylinder about that axis;
   - coaxial planes ⊥ the axis, cylinders, cones, spheres centred on the axis, and tori.

   The faces span bounded φ ranges (`ue`, screw.rs:356–362), and screw faces stay off the axis
   (`rmin > LINEAR`, screw.rs:382–384). So a **coaxial** configuration, which covers a thread or
   coil cut into its shaft and its flat ends, is a semialgebraic problem in the cover. The stage-2
   exact machinery then applies unchanged. `screw/coaxial.rs` and `screw/pair.rs` already exploit
   this in closed form, so the reference would match their domain exactly.
2. **Non-coaxial configurations** include a thread against a cross hole, a tilted plane, a wrap
   against a free plane, a spatial RMF sweep, and angle-matched lofts. Here the equations mix φ
   with cos φ and sin φ (exp-polynomial or Pfaffian), and the RMF table adds exp of bounded
   arguments.
   - **Finiteness still holds.** Over the bounded model box every such set is definable in
     ℝ_an (restricted analytic functions; Denef & van den Dries 1988; van den Dries, *Tame
     Topology and O-minimal Structures*, 1998). It has finitely many components and a cell
     decomposition.
   - **No effective decomposition is known.** Decidability of ℝ_exp is conditional on Schanuel's
     conjecture (Macintyre & Wilkie 1996). Khovanskii's fewnomial bounds bound how many components
     there are, but do not find them.
   - So the reference here is a **δ-complete decision procedure** (Gao, Avigad & Clarke,
     "δ-complete decision procedures for satisfiability over the reals", IJCAR 2012; dReal). It
     uses interval subdivision with Krawczyk inclusion and exclusion over the bounded domain, with
     δ ≪ LINEAR (say LINEAR/1000). Every question is either decided or within δ of a tolerance
     threshold; the latter yields `Ambiguous`.

     This matches the tolerance semantics exactly: the kernel is only ever asked to decide up to
     LINEAR, and the reference never claims more than δ-precision there.
3. **Which surface is "the" surface.** For the spatial RMF sweep the reference takes the kernel's
   Magnus-table formula as the definition, exact on the stored f64 table. A separate certified
   bound relates it to the true RMF (the 1e-13 step agreement in `frame_table` is a test, not a
   bound). The same rule as for dmath applies: decide the surface the kernel holds.

---

## 8. Refinement: proving each fast path against the reference

**The obligation.** For every optimised path P (the fast kernel, the imprint fast path, n-ary,
memo caches, float filters), on the reference's input domain, one of these must hold:
- (i) **P(x) ≈ Ref(x)**: the comparison relation of section 6, meaning the same verdict class,
  boundary within c·LINEAR, and the same topology modulo sub-LINEAR features; or
- (ii) P is **total** and refuses only under a stated condition C(x), and C is a theorem about
  Ref(x). For example: ZeroThickness only if Ref is NonManifold or Ambiguous; Unsupported only for
  the listed kinds (stage-4 surfaces); never Internal.

The proof splits along the boundary that the ledger of 2.1 creates:

```
f64 inputs ──numeric predicates──▶ sign vector (ledger) ──combinatorial core──▶ B-rep
              (certified bridge)                            (Aeneas → Lean)
Lean:  bridge ⊢ ledger is a consistent sign assignment for the real inputs (band rule)
       core   ⊢ ∀ consistent ledger, core(ledger) = RefAlgo(ledger)        (2.1 obligation ii)
       RefAlgo ⊢ = regularised Boolean spec (section 6)
```

### 8.1 Aeneas for the combinatorial code

Aeneas (Ho & Protzenko, "Aeneas: Rust verification by functional translation", ICFP 2022; Charon
extracts MIR) translates safe Rust to pure Lean 4, with loops becoming recursive functions and
panics becoming errors. Production use includes Microsoft's SymCrypt Rust code.

From the published work (check the current release before committing):
- *Supported:* `Vec`, structs and enums, borrows, traits.
- *Unsupported or limited:* interior mutability (`Cell`, `RefCell`, thread-locals), concurrency,
  trait objects (`Arc<dyn …>`), unsafe code, and heavy closure or iterator-adapter chains.
- *Floats:* opaque, with no IEEE semantics.

How the kernel fits:
- **Good fits.** forge-topo's arena B-rep (ids as indices, no borrowed graph) and its rules (`check`,
  already proven local≡full in research/check-local/lean). Also the post-refactor union–find over
  decided incidences, face splitting from a ledger, the `classify_specs` flood (boolean.rs:5861),
  `unify` joins, and naming.
- **Blockers today, all of which the refactor removes anyway:**
  - the thread-local retry flags (boolean.rs:853–870, intersect.rs:580);
  - numeric and combinatorial work interleaved in `trim` (2265–3169);
  - `Arc<dyn Procedural>` surfaces inside topology code;
  - `par_map` threads (nary.rs:51); prove the sequential core and treat parallelism as a separate,
    order-independence lemma.
- **Recommendation.** Extract the ledger-driven core as its own crate with no floats in its
  signatures (it takes signs and symbolic keys), and run Aeneas on that. Where Aeneas falls short,
  write the Lean model by hand and tie it to the Rust by differential testing on random ledgers
  (Lean-compiled versus Rust). That is weaker than extraction, but practical. Kani can check
  totality and panic-freedom of that crate on small bounds.

### 8.2 The certified-predicate bridge for the numeric code

These are proven in Lean directly over a floating-point model, not through Aeneas:

- **IEEE-754 as axioms.** Correct rounding to nearest-even for + − × ÷ and sqrt: fl(a∘b) =
  (a∘b)(1+δ) + η, with |δ| ≤ u and |η| ≤ underflow bound. This is the model
  research/loft-ssi/shipped/LoftMeet/Float.lean already uses. Native and wasm bit-identity is
  consistent with it, since both use the same correctly rounded ops and no FMA contraction.
- **Shewchuk filters and exact expansions** (forge-math/src/predicates.rs). Prove the stated bounds
  (`CCW_BOUND`, `ICC_BOUND`, `CROSS_BOUND`, `DOT3_BOUND`, `TRIPLE_BOUND`; research/predicate-bounds),
  and that the expansion arithmetic (two_sum, two_product) is exact under the axioms.
- **Interval filters** (forge-math/src/interval.rs). Rust has no rounding modes, so outward rounding
  is one-ulp widening (next_up/next_down). Prove enclosure from the rounding axiom.
- **dmath trig** (an fdlibm port, forge-math/src/dmath.rs). State per-function axioms
  |dmath::sin(x) − sin x| ≤ k·ulp on stated ranges, and the same for cos, atan2 and sin_cos. Justify
  them from fdlibm's published bounds plus exhaustive or MPFR differential testing; or replace them
  later with proven correctly-rounded kernels (CORE-MATH). The reference never needs these axioms
  for stored data (6.2: it takes the stored results as exact). They are needed only where the
  kernel *computes* a decision through trig.
- **What needs the bridge:**
  - the closed-form surface–surface meets (ssi.rs; meet.rs, meet_gen.rs from SymPy; Cardano and
    quartic solvers in forge-math/src/poly.rs): each returns curves and contact types, which must
    come with root-isolation certificates (2.4);
  - `point_in_face` and ray classification (classify.rs): planar faces already use exact
    predicates, curved faces need filters;
  - `settle` (boolean.rs:4660 ff.): the vertex's grown tolerance must be a *proven* bound on its
    distance to every curve;
  - the tracer (intcurve.rs): Krawczyk certificates (2.6);
  - `track()` clearance (2.7).
- **Fast paths proven by keyed purity.** These are proven equal to the path they shortcut, not to
  the reference:
  - the memo caches (screw.rs:1128, ssi.rs:161, swept.rs:112/170): equal iff the key holds every
    input bit, which is the op-memo audit's rule;
  - the imprint fast path (imprint.rs:26): equal on its CLEAR domain; its `Check` mode is the test
    form of that theorem;
  - `may_be_empty` (bodies.rs:1099): a conservative guard, proven by box containment.

**Proof cost**, smallest first:
1. keyed-purity fast paths;
2. Shewchuk filter bounds (textbook);
3. the combinatorial core under a consistent ledger (the 2.1 pilot);
4. the closed-form meets with root-isolation certificates;
5. the tracer (Krawczyk) and the dmath axioms' justification.
