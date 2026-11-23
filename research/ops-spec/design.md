# The exact specification of shell, draft, hole, rib, chamfer and pattern: design note

2026-11-23. Design stage. This note comes before any Lean or kernel code. It states, for each of
the six features, the one set the feature produces (L1), the exact refusal conditions, the decided
rule that replaces each retry or fallback, and the plan for L2–L6
(research/PROOF-STANDARD.md). It reuses definitions and does not redefine them. The kernel code
was read at 5de0a5f; file:line references are to that commit.

## 0. Common ground

**Where it lives.** `verify/forge-exact`, in a new tree `ForgeExact/Ops/`. It uses core Lean only,
like the rest of forge-exact. Everything is stated over any ordered field `K`, with the class
context the fillet spec uses (`Fillet/Basic.lean`: `Field`, linear order, `OrderedRing`). So ℚ, the
real algebraic numbers and ℝ all satisfy every theorem. Distances are written squared, and no
square root appears in any statement.

**What it imports. Nothing below is defined twice.**

| from | what |
|---|---|
| `Topology`, `Regular` (on base) | `Pred`, `reg`, `combine`, `Decomposition`, `reg_kept`, `reg_reg` |
| `Fillet/Basic`, `Fillet/Closing` (fillet-spec) | `V K`, `d2`, `Free`, `closing`, `opening`, `cut` |
| `Fillet/PlanePlane`, `Fillet/Revolve` (fillet-spec) | `PlanePair`, the springs, `free_rev`/`closing_rev` (meridian form) |
| `Flat` (base) | `HForm`, the half-space topology, `flat_boolean` |
| `Quad`, `Alg` (exact-curved) | `Quadric`, `QForm`, `arrangement`, `Num` |
| `verify/forge-transform` (fundamentals) | rigid motions and mirrors over ℝ; it is cited, not re-proven |

**Merges.** fillet-spec is based on 1abd5bc, which is an ancestor of this branch, so I merge it in
(a plain merge). exact-curved and fundamentals branch from 8250ebb. I take only their files under
`ForgeExact/Alg`, `ForgeExact/Quad` and `verify/forge-transform` when step 3 needs them, again by a
plain merge. If the master would rather merge them into `claude/repo-review-mbdrlj` first, I
re-base nothing and simply merge the base.

**Angles carry no trigonometry.** The kernel's `draft`, `chamfer(angle:)`, countersinks, drill
points and circular patterns take angles in degrees. In the exact layer an angle enters as a
direction pair `(c, s)` with `c > 0` (or `c² + s² > 0`). It is not normalised, because a plane's
normal needs no unit length. The reference reads the kernel's own f64 `cos`/`sin` as exact
rationals. The degree-to-pair step is the one rounding, with an L5 bound
(`|c − cos a|, |s − sin a| ≤ u`), so the spec is exact in `(c, s)`. Where a unit vector is
unavoidable, as in a cone's half-angle, the field is extended by `√(c² + s²)` through `Alg`.

**Two shapes of definition.** The six features fall into two groups.

- **Boolean features** are hole, rib and pattern, and chamfer and shell where they cut. The result
  is `reg (combine op [S, T₁, …])` for an exact tool `T`. So `reg_kept` applies unchanged, and
  decide-once is the L3 algorithm.
- **Re-carrier features** are draft, the inner skin of a shell, and the faces a chamfer trims. The
  result keeps the body's face lattice, gives some faces new carriers, and recomputes vertices and
  edges as the intersections of the new carriers. This is what `rebuild`/`wavefront` already do.
  The spec defines it as a set (§0.1), states exactly when it is valid, and refuses exactly when it
  is not.

### 0.1 `Recarrier`: the definition shared by draft, shell and chamfer

A polyhedral body is given as a formula over its faces' closed half-spaces. For a planar B-rep this
is the local formula at each edge and vertex: convex means `∩`, concave means `∪`. Re-carrying
substitutes each changed face's half-space in that formula:

  `recarrier S σ := reg (F_S (σ H₁) … (σ Hₘ))`  where `S = reg (F_S H₁ … Hₘ)`.

`F_S` is read off the lattice: it is the "convex/concave tree" of the edges, which is the
Nef/Peterson formula. It is valid (`Valid S σ`) when the substituted formula realises the *same*
lattice. That is:

1. every vertex is still the common point of its faces' new planes, and a vertex of degree > 3
   has its new planes still concurrent;
2. no edge's direction reverses: the new edge vector · the old one > 0;
3. no new contact appears: the realised faces meet only along the lattice's edges. This is decided
   by the boolean's arrangement.

Theorem R1 (`recarrier_valid`): when `Valid S σ` holds, `recarrier S σ` is the solid bounded by
the re-carried faces with the same lattice, and it is independent of which formula of `S` was read.
Without validity the formula depends on its choice, so the spec refuses exactly then. For curved
faces the same holds, with the carriers' `QForm`s in place of half-spaces and the decide-once
curved core (`CurvedC`, as the fillet spec states it) in place of `flat_boolean`.

## 1. Pattern (`pattern.rs`; lib.rs:1997–2028, 1613–1633)

The kernel moves each source's tool by the copy's motion, renames it, and makes **one** n-ary
boolean of the part with all the copies.

**L1.** With motions `g₀ = id, g₁ … gₙ` (forge-transform: each is a rigid motion or a mirror, hence
a homeomorphism):

  `pattern S T op gs := reg (combine op' [S, g₀ T, …, gₙ T])`

Here `op'` is `S ∨ ⋁ gᵢT` for Add and `S ∧ ¬⋁ gᵢT` for Cut. A pattern of several features is the
same formula over all their tools, in feature order. `of: whole` is `⋃ gᵢ S`.

**Theorems.**
- P1 `reg_image`: a homeomorphism commutes with `reg`, so a copy of a solid is a solid.
- P2 `cut_seq` and `add_seq`: for regular closed operands,
  `reg (reg (A \ B) \ C) = reg (A \ (B ∪ C))` and `reg (reg (A ∪ B) ∪ C) = reg (A ∪ B ∪ C)`. So
  the n-ary pass, any one-at-a-time order, and the `may_empty` fallback (lib.rs:953) give **one**
  result. That fallback is order-free by theorem, not by testing.
- P3 `dedup`: dropping a copy equal to an earlier one (`distinct`, lib.rs:1062) does not change the
  set. The rule decided here is that equality is of the **exact** motion. Today the key is float bits
  of the frame origin, which is sound but incomplete: it can miss an exact duplicate, though never a
  false one. It costs nothing.
- P4: copy `k` of a circular pattern is `turn (k·step)`, and `n` copies at `2π/n` close up. These
  are forge-transform's `turn_turn` and `turn_full`, cited.

**Refusals.** `BadLayout` is exact as it stands (zero direction, `n = 0`, non-finite). A copy that
misses the part is **not** a refusal: the set is defined. A copy that overlaps another is not one
either: P2 covers it. The only refusals are the boolean's own `XNonManifold` and `Empty`. These are
the boolean's L3 (`zeroThickness_iff`), which is already proven.

**Decided rule.** None of the kernel's choices change the set (P2, P3). Instance naming is outside
the set spec, and the fundamentals branch covers it.

## 2. Hole (`hole.rs:146–227`, `standard.rs`, `extent.rs:573–632`)

**L1.** The hole's tool is the solid of revolution of the kernel's profile polygon `P` in the
meridian half-plane `(ρ, z)`, with `z` down the drill:

  `H := {x : (ρ²(x), z(x)) ∈ P̂}`

Here `P̂` is `P` written in `ρ²`. A wall piece is `ρ² ≤ r² ∧ z₀ ≤ z ≤ z₁`. A cone piece
(countersink or drill point) is `ρ² ≤ (a + b z)² ∧ 0 ≤ a + b z`, with `b = (c, s)`, the half-angle's
direction pair. A counterbore is the union of two wall pieces, a countersink a cone and a wall, and
a point a wall and a cone. This is fillet-spec's `Revolve` meridian form, reused. The result is
`reg (S \ ⋃_places H_p)`, which is a pattern of translations (§1).

- **thru** is defined by the bodies the hole names, and by no box. It is the half-infinite wall
  `H∞ := {ρ² ≤ r², z ≥ 0} ∪ (the counterbore/countersink pieces)`.
- **blind `d`** is the wall to `z = d` measured from the mouth point. A point angle adds the cone
  beyond.
- **upto(F)** and **next** trim `H∞` by the face's side: `H∞ ∩ {z ≤ z_F(ρ, θ)}`.
- **next** is defined exactly as follows: `z_F(q)` is the first exit from `S` along the drill ray
  through each disc point `q`.
- **`hole_table`**: the table (`standard.rs:58–85`) is a finite function from size, fit and
  convention to numbers. The spec is a Lean copy of the table, generated from the Rust, and L2 is
  that they are equal.
- **thread:** is cosmetic (hole.rs:19). The spec says so: it adds no geometry, only the record.
  The helical `thread` feature is not this item.

**Theorems.**
- H1 `thru_finite`: if every point of `S ∩ H∞` has `z < L`, then `S \ H∞ = S \ H_L`. That is the
  only obligation on the kernel's `hi − lo + 1` (hole.rs:156–167). It needs `reach` to be a sound
  enclosure of `S` along the axis. **Today it is not:** `reach` silently drops an add feature whose
  extent fails to resolve (hole.rs:298), and it grows the box only for the features it lists
  (hole.rs:351–359). The decided rule is that `L` comes from a proven enclosure of the **built
  body** (the forge-topo bbox, which must be sound for curved faces), not from a hull of feature
  tools. This can lengthen a thru hole. It cannot change a correct result, by H1.
- H2 `next_one_face`: `next` is defined iff `z_F` is attained on one face's closure for every disc
  point. `Ambiguous` is refused exactly otherwise. Today the kernel tests 5 rays (extent.rs:582),
  which is necessary but not sufficient, and `verify_next` checks the bottom afterwards. The exact
  test is that the face's trimmed patch covers the disc's projection, decided in the arrangement.
- H3 `profile_valid` gives `BadSize` exactly when `P` is not a simple meridian polygon:
  - `r > 0`;
  - for a counterbore, `Dc/2 > r` and `0 < dc < depth`;
  - for a countersink, `Dc/2 > r` and its depth `< depth`;
  - `0 < angle < π` as a pair `c > 0, s > 0`.

  These are the kernel's conditions (hole.rs:149–197), now with a proof that they are exactly the
  simple-polygon condition.
- H4 `no_target`: `reg (S \ H_p) = S` iff `int S ∩ int H_p = ∅`. **New refusal:** `NoTarget{place}`
  exactly then, as the brief asks. Today such a hole silently does nothing. This changes answers:
  every repo part whose hole misses will be listed before the refusal is enabled, so that the
  master can decide between a refusal and a lint.

## 3. Shell (`shell.rs`, `wavefront.rs`, lib.rs:1496–1541)

The kernel builds the inner skin directly. Each face is offset by `t` (`offset`, shell.rs:852) and
the vertices are re-solved where offset faces meet: a **sharp (intersection) join**. The topology
events are handled by a depth-12 search (wavefront.rs:323–378).

**L1.** The cavity is a re-carrier (§0.1) of `S`. Each closed face `f` gets its inward offset
carrier `off_t f` (plane moved by `t`, cylinder or sphere radius `∓t`, torus minor `∓t`, cone
`r ∓ t/cos`). Each open face keeps its own carrier, which is what puts the rims on the open face's
surface (shell.rs:426–493). Then

  `shell S t O := reg (S \ C)` with `C := recarrier S (off_t on closed faces, id on O)`

and the outward shell is `reg (C⁻ \ S)`.

**Its relation to the erosion (the ball spec).**
- S1 `sharp_eq_erosion`: at an edge between two closed faces that is convex as seen from inside,
  `C` agrees locally with the erosion `{x ∈ S : Free K t x}`, where `K` is the closed faces and
  `Free` is the fillet's.
- S2 `sharp_sub_erosion`: at a reflex edge, `C` is the erosion minus the fillet spec's plane–plane
  closing region of the offset pair. So the sharp shell is the erosion shell with its inner corner
  left unrounded, and the wall there is thicker by exactly that region.

The spec **chooses the sharp join**. It is what the kernel builds, and it is what every shelled
repo part has now. The erosion is the named alternative (`join: round`) and is not adopted. So the
spec changes no answer here.

**Refusals, exactly.**
- `TooThin`: `t ≤ 0`. The kernel's `≤ MIN_LENGTH` is the snap `τ`; under the snapped input it is
  the same condition.
- `AllOpen`, and `NoSuchFace`: both already exact.
- `Focal{face}`: some closed face's offset carrier degenerates within the face's trimmed patch. That
  is a convex radius `≤ t`: cylinder or sphere `ρ ≤ t`, torus minor `≤ t`, or a cone whose
  `r(z) − t/cos ≤ 0` on the patch (`reaches_apex`). It is decided exactly on the patch's extreme
  vertices for quadrics.
- `Event{kind, at}`: `Valid S σ` fails (§0.1).

**The decided rule for events.** This replaces the depth-12 search.
- Events are taken in order of the **offset distance** `τ ∈ (0, t]` at which they occur, as in a
  straight skeleton.
- At each event the lattice change is the unique one for a degree-3 event:
  - an edge's length reaching 0 is an edge flip;
  - a face's area reaching 0 drops the face;
  - a vertex meeting a non-adjacent face is a split.
- Equal-τ events, or an event of degree > 3, are refused as `Undecided{at}`. (The 3D straight
  skeleton is not unique there, so the spec does not pick.)
- The kernel's `collapse` (a fillet strip thinner than the wall) and `islands` are such events,
  with their own guards.
- The search's `dead` memo and its budget go. This can change an answer only where today's search
  found a lattice that is not the first-event one. Each such part will be listed.

The thread pre-pass (`threaded_shell`, lib.rs:1522–1541) is a boolean followed by this definition.
Its fallback to the plain shell when `offset_solid` is None is replaced by `Unbuilt{strip}` unless it
is proven equal. Its volume check becomes a debug assertion of a proven identity.

**Coordination.** The shell-fix session (spline walls) must achieve that a spline face's
`off_t f` is the exact offset carrier. `Offset::surface_over` is an approximation, so a spline-wall
shell is L1-exact only once that is proven or bounded, and it is refused as `Unbuilt{spline}`
otherwise.

## 4. Draft (`draft.rs:50–291`)

**L1.** This is a re-carrier.
- A drafted **plane** `f` with outward normal `n` and pull `d` (the neutral plane's `z`) gets the
  plane through the neutral line `L = f ∩ N`, with normal `n' = c·h + s·d`. Here `h` is `n` with its
  `d`-component removed, and the angle is the pair `(c, s)`. This is the kernel's rule
  (draft.rs:170–189): the drafted face makes angle `a` with the pull, whatever its old inclination.
  The spec takes it as stated, an absolute angle.
- A **cylinder** with axis ∥ `d` becomes the cone through the circle `f ∩ N` with half-angle pair
  `(c, s)`, opening by `sign(outward · a)`. A **cone** with that axis is treated the same way.

`draft S F N a := recarrier S σ_a`.

**Refusals, exactly.**
- `BadAngle`: `c ≤ 0` or `s = 0`, which is `|a| ≥ 90°` or `a = 0`. The kernel's `π/2 − 1e-6` is the
  snap. Any face turned through vertical has `c ≤ 0`.
- `NoNeutralLine`: `h = 0`, which is `n ∥ d`, the face parallel to `N`. **The kernel has two errors
  for one condition** (`AlongPull` at :178 and `NoNeutralLine` at :184). They become one, or the
  order between them is fixed.
- `ThroughApex`: the cone's apex lies on the face's patch, decided on the patch's z-range.
- `NoExactSurface`: an axis not ∥ `d`, or a sphere, torus or spline. These are honest gaps, and they
  stay refusals.
- `Tangent{face}`: the tangency closure of `F` meets a face it cannot draft. The closure is a set,
  so it is independent of BFS order. The theorem says so, and the test permutes `F`.
- `Event`: `Valid` fails.

**Decided rules.**
- When several faces are refused, the error reported is that of the least face name. Today it is
  the first in list order (:91–101).
- `taper::tapered`'s fallback to stacked slabs (taper.rs:416–451) is an extrude feature, which is
  outside this item. It is to be stated as its own definition (the slab union) by its owner, and is
  flagged here.

**Coordination.** The drafted-lid session's fixes must make `draft = recarrier σ_a` hold on its
parts. Any lid that draft builds today and that fails `Valid` is a wrong result today.

## 5. Chamfer (`chamfer.rs`)

The kernel cuts nothing by boolean. It inserts a face for each picked edge, through contact curves
on the two faces, and re-solves.

**L1.** For an edge `e` between faces `A` and `B`, with distances `(d_A, d_B)`, the contacts are
`p_A(s) ∈ A` and `p_B(s) ∈ B`. Each is at distance `d` from the edge point `e(s)` in the plane
normal to `e` at `s`. This is the kernel's three-surface solve (:905–911) as an equation.
- The chamfer face is the ruled surface `[p_A(s), p_B(s)]`. It is a plane for line edges, and a
  plane, cylinder or cone for coaxial circle edges (:925–1002).
- The result is the re-carrier that inserts this face and trims `A` and `B` at the contacts. As a
  set, for a convex edge, it is `reg (S \ W_e)`, where `W_e` is the region of `S` beyond the chamfer
  surface and between the contacts.
- **two-distance**: `d` on `face`, `d2` on the other face.
- **distance–angle**: `d_other = d·s/(sin of interior + angle)`. Since `sin(θ + a) = s_θ c + c_θ s`,
  this is polynomial in the pairs, with `θ` taken from the face normals.

**Vertices.**
- k = 1 is the chamfer face run out to the untouched edges, intersected by `along`.
- k = 2 is the mitre: the two chamfer surfaces meet, cut by their intersection.
- k = 3 in `Point` mode is the intersection of the three chamfer half-spaces.
- k = 3 in `patch` mode adds the plane through the three points where adjacent chamfer planes meet
  the face between them.

Each is a re-carrier, valid or refused.

**Its duality with the round (the fillet spec).**
- C1 `chamfer_through_springs`: for a plane pair with interior dihedral `θ` and a convex round of
  radius `r`, the springs lie at distance `d = r·cot(θ/2)` from the edge. Written without trig, that
  is `d·(1 − cos θ) = r·sin θ` in the pairs. The equal chamfer with this `d` is the plane through the
  round's two springs.
- C2 `round_cut_sub_chamfer_cut`: `cut S r ⊆ W_e`. The chamfer removes the round's cut plus the
  segment between the chord and the arc. So `opening S r ⊇ chamfer S d`, with equality exactly on
  the springs.

**Refusals, exactly.**
- `BadSize`: `d ≤ 0`, or the angle pair is out of `(0, π − θ)`.
- `Tangent`: `θ ∈ {0, π}`, meaning the faces are tangent.
- `TooLarge`: a contact does not exist on the face patch. That is, the circle of radius `d` about
  `e(s)` in the normal plane misses `A`'s trimmed patch for some `s`. This is decided by the
  patch's boundary. Today it is "Gauss–Newton did not converge" (:918), which is not a condition.
  The exact one replaces it as the meaning of the error, and the solver stays as the computation,
  checked against it.
- `Overflows{face}`: `Valid` fails by a new contact.
- `Unsupported`: honest gaps, listed, unchanged.

**Decided rules (the fallbacks).**
- `along`'s retry on a range extended by `±64·width` (:463–476) is replaced by the definition: the
  hit is the first intersection of the untouched edge's **carrier line** with the chamfer surface
  from the vertex. Unbounded, it is one rule, and the retry was an approximation of it.
- The k = 1 rule "within LINEAR of the far vertex, take the edge whole" (:546–553) is the snap of an
  exact event (the hit at the far vertex). It is kept as a snap with τ, as Snap.lean states for the
  boolean.
- `Point` mode's silent fallback to a patch (:678–680) becomes the mode's own definition. `Point`
  is valid iff the meet point is below all three faces, and otherwise refused as `PointNotBelow`.
  Today's fallback output is still available as `corner: patch`. **This changes answers** on parts
  that ask for `point` and get a patch today. They will be listed. If any exist, the master decides
  whether to keep `point` as "point where valid, else patch", stated as one rule.
- The interior angle is taken at the mid-parameter only (:870). That is exact for line and coaxial
  circle edges, where `θ` is constant (theorem), and wrong for spline edges. For those the spec
  takes `θ(s)` per point, and the kernel refuses distance–angle on spline edges until it does
  likewise.

## 6. Rib (`rib.rs:193–300`)

**L1.**
- Let `Γ` be the path with its ends extended **to infinity** along their end tangents. Let `W` be
  the closed side of `Γ` toward the fill normal: the plane region `Γ` bounds, intersected with the
  sketch plane.
- Let `Slab = {lo ≤ n·x ≤ hi}`, by `side`. Let `R` be the connected component of
  `int ((W × Slab) \ S)` that has the germ of the path's middle segment on its fill side: the points
  `mid + εℓ + ((lo+hi)/2) n` for all small `ε > 0`.

`rib S := reg (S ∪ cl R)` when `R` is bounded. **web** is the same with `W` the thickened path
`{q : dist(q, Γ) ≤ t/2}`, extruded to the far side. A draft re-carries the caps or walls (§4).

**Refusals, exactly.**
- `RunsOff`: `R` is unbounded.
- `NoRoom`: the germ lies in `S`.
- `BadPath`, `Corner` and `TooTight`: exact as they stand. `TooTight` is an offset arc radius
  `≤ 0`, with the snap.

**Theorems.**
- B1 `runsOff_iff_box`: a bounded `R` lies in `bbox(S ∪ path)`. The extensions end on `S` and the
  box is convex. So the kernel's test "inside `bound.grow(margin)`" with `far > diag + margin` is
  exactly "bounded", and the box test is proven rather than heuristic.
- B2 `probe_rule`: the germ replaces the fixed `1e-3` probe (:258, :286). The kernel's choice
  agrees with the germ whenever no face of `S` lies within `1e-3` of the middle segment. The decided
  rule is to probe at `ε = ½ · (distance from the segment to S, in the slab)`, computed. This
  changes nothing except parts where `1e-3` picks the wrong component, which are wrong today.
- B3: "first `Inside` shell in shell order" (:291–293) is unique by B2, since the components are
  disjoint, so it is order-free.

## 7. Levels and order of work

| feature | L1 | L2 reference | L3 | L4 | L5 | L6 |
|---|---|---|---|---|---|---|
| pattern | `pattern`, P1–P4 | the boolean's | P2 ⇒ the boolean's L3 | interim: differential | the boolean's | the part census, plus random patterns |
| hole | `H`, H1–H4 | the profile over ℚ and the pairs, then the boolean | H1 ⇒ boolean L3; H2 in Quad | interim | `thru` depth, the enclosure | the census, random holes |
| chamfer | plane pair: C1, C2; circle edges | the plane through exact contacts over ℚ/Alg | `recarrier_valid` | interim | the contact sign tests | the census, random edges |
| draft | planes, cylinders | exact re-carrier over ℚ (planes) and Alg (cones) | `recarrier_valid` | interim | `c > 0`, `h ≠ 0` | the census, random |
| shell | `recarrier`, S1, S2 | flat: exact; quadric: Alg | R1 plus the event rule | interim | `Focal` tests | the census, random boxes and bores |
| rib | `R`, B1–B3 | the component over ℚ | B1, B2 | interim | the germ distance | the census, random |

**L4.** The plan is Aeneas only where the decision code is separable. Candidates are the hole
profile builder (`hole.rs:146–227`, pure arithmetic), `pattern::fixed` (the motions) and the
chamfer two-distance/angle selection (:878–891). The solver-heavy parts (`rebuild`, `wavefront`) do
not go through Aeneas. There a differential test against L2, with a committed digest, is the
interim, marked interim.

**Order.**
1. `Ops/Pattern.lean` (P1–P3) and `Ops/Hole.lean` (H1, H3, H4). Both are general topology and
   meridian algebra, and depend only on the base and fillet-spec.
2. `Ops/Chamfer.lean` (C1, C2 on `PlanePair`) and `Ops/Draft.lean` (planes). Then
   `Ops/Recarrier.lean` (R1 for flat bodies, through `flat_boolean`).
3. `Ops/Shell.lean` (S1, S2, the flat events) and `Ops/Rib.lean` (B1, B2).
4. The L2 references, a driver `forge-ops-ref`, and a harness `verify/forge-ops-check`:
   - the harness dumps, for every repo part and a seeded random generator, each feature's inputs and
     the kernel's result faces (exact rationals of the f64s);
   - the reference re-derives them;
   - the digest is committed;
   - a CI job is added alongside `forge-exact.yml`.
5. Kernel changes, one decided rule at a time, each gated:
   - hole thru enclosure;
   - `along` as a definition;
   - the rib probe;
   - the draft error order;
   - the shell event order;
   - `NoTarget` and `PointNotBelow`, after the listing.

**Gates for any kernel change.**
- The reliability scoreboard 0 worse.
- `FUZZ_SEEDS=150` at 150/150.
- `GATE_COUNTER=callgrind tools/speed/gate.sh` against a baseline on 5de0a5f, with user-facing and
  synthetic lines ≤ base.
- Native = wasm bit-identical.
- No crates in `kernel/`.

No `.fg` language change is planned. `join: round` for shell is named and not added.

## 8. Questions for the master

1. Hole `NoTarget`: a refusal, as the brief says, or a lint? I will list the parts it changes
   before enabling it either way.
2. Chamfer `corner: point`: refuse when not valid, or keep "point, else patch" as one stated rule?
3. Merging exact-curved and fundamentals: into my branch (only the Lean trees I use), or will the
   base get them first?
4. The regen-level `careful` blunting retry (lib.rs:641–659) applies to holes and patterns through
   the boolean. I take it to be the boolean owners' to decide. Please confirm.
