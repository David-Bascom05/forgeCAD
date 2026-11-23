# The curved reference's L2 gaps, proven by hand

PROOF-STANDARD L2 asks that the executable reference be proven equal to its L1 spec. For the
curved reference (verify/forge-exact/ForgeExact/QRef), the Lean proofs cover these checkers:
- `Quad.boxSign` and `member_box`;
- `Quad.pairCheck` and `pairCheck_spec`;
- the formula on patterns (`QForm.den_iff_onSigns`, `member_off_wall`);
- emptiness from patterns (`empty_of_patterns`);
- `Alg.Num.sign` (`sign_spec`).

Two pieces of the executable that use them were not proven: the octree traversal and the
touch/saddle contact test. This note proves both by hand, as the step before Lean.

Notation:
- `R = reg(combine op (F i).den)` is the regularised history (L1).
- `qs` are the arrangement's quadrics, and the wall `W` is the union of their zero sets.
- `κ(σ)` is the history's value on a sign pattern σ (`Arr.keptOn`).

`member_off_wall`: for `x ∉ W`, `x ∈ R ⇔ κ(pattern(x))`.

**Assumption B (boundedness).** Every tool lies in the hull of its faces' boxes. The root cube
holds that hull grown by 1, so every point outside the root cube is outside every tool. The
harness checks each tool against the kernel's classifier, the same reading of a tool as the
intersection of its faces' inner sides. So the history keeps no point outside the root cube:
new/add take the union of tools, cut and keep only remove.

## 1. The octree (`QRef.octree`, `Arr.keptK`, `Arr.anyKept`)

**Now in Lean** (ForgeExact/QRef/BF.lean, Octree.lean; in Axioms.lean). The executable's
history is a formula `BF`. `keptOn`, `keptK` and `anyKept` are `BF.eval2`, `BF.eval3` and
`BF.allFalse`, so the theorems are about the code that runs:
- Lemma 1.1 is `BF.eval3_sound`;
- the enumeration in 1.5 is `BF.allFalse_sound`;
- 1.2 and 1.3 are `in_box` and `out_box`, at points off the wall;
- 1.5 is `empty_of_leaves`, with the cover as a hypothesis;
- `member_off_wall` for the history is `reg_off_wall`.

What stays by hand: that the octree's leaves tile the root cube (it halves boxes), and
assumption B.


The octree splits the root cube into eight closed children, level by level. Each box `B` (centre
`c`, half-width `h`) gets signs `s_k = boxSign q_k c h ∈ {1, −1, 0}`. By `boxSign_spec`,
`s_k = 1` gives `q_k > 0` on all of `B`, `s_k = −1` gives `q_k < 0`, and `0` gives nothing.

**Lemma 1.1 (Kleene soundness).** Let `keptK s = some b`. Then `κ(σ) = b` for every complete
pattern `σ` that agrees with `s` where `s_k ≠ 0`.

*Proof.* `keptK` folds `and3`, `or3` and `not3` (Kleene's strong three-valued connectives) over
the same expression tree as `keptOn` folds `&&`, `||` and `!`. A literal is `some b` exactly when
its quadric's sign is certified, and then equals the literal's value under every completion. By
induction on the tree:
- If every argument with a decided value has that value under all completions, then so does the
  connective's result. This is the defining property of Kleene's tables: `and3 (some false) _ =
  some false` because `false && y = false` for every `y`, and `or3` and `not3` alike.
- The fold's result is `some b` only through such entries. ∎

**Theorem 1.2 (in-boxes).** If `keptK (signsAt B) = some true`, then `B ∖ W ⊆ R`, and the
interior of `B` lies in `R`.

*Proof.* Take `x ∈ B ∖ W`. Its pattern agrees with the certified signs, by `boxSign_spec`. By
Lemma 1.1, `κ(pattern(x)) = true`. By `member_off_wall`, `x ∈ R`.
- `W` is nowhere dense (`wall_nowhereDense`), so `B ∖ W` is dense in `B`.
- `R` is closed: it is a closure.
- Hence `B ⊆ R`, so `int B ⊆ R`. ∎

**Theorem 1.3 (out-boxes).** If `keptK (signsAt B) = some false`, then `int B ∩ R = ∅`.

*Proof.* As in 1.2, every `x ∈ B ∖ W` has `x ∉ R`. Suppose `y ∈ int B ∩ R`.
- Since `R = closure(int(combine))`, the open set `int B` meets `int(combine)` in a nonempty open
  set `U`.
- `U` contains a point `z ∉ W` (`escape`).
- `z ∈ combine` with `z ∉ W` means `κ(pattern(z))` holds (`den_iff_onSigns`).
- But `z ∈ B ∖ W`, so by Lemma 1.1 `κ(pattern(z)) = false`. Contradiction. ∎

**Corollary 1.4 (the enclosure).** Let `I` be the boxes reported in, `U` the leaves left
undecided, and `O` the boxes decided out. Then `Σ_I vol ≤ vol R ≤ Σ_I vol + Σ_U vol`.

*Proof.*
- The boxes of one level have disjoint interiors, and a box is split only while undecided. So
  `I ∪ U ∪ O` tile the root cube with disjoint interiors (decided boxes are never split).
- Box boundaries are null sets.
- Lower bound: by 1.2, `I`'s interiors lie in `R`.
- Upper bound: by 1.3, `R` meets no out-box's interior. By B, `R` lies in the root cube. So up to
  null sets, `R ⊆ ∪I ∪ ∪U`. ∎

**Theorem 1.5 (emptiness).** Suppose that:
- no box is in;
- every exclusion `e = (i, a, j, b)` passed `pairCheck`, so no point has `q_i` of sign `a` and
  `q_j` of sign `b` (`pairCheck_spec`);
- for every undecided leaf `L`, `anyKept (signsAt L) ex = some false`.

Then `R = ∅`.

*Proof.* By `empty_of_patterns` it suffices that `κ(pattern(x))` is false for every `x ∉ W`.
- If `x` is outside the root cube: assumption B.
- If `x` is in some out-box: Lemma 1.1.
- Otherwise `x` lies in a closed undecided leaf `L`, since the tiling covers the cube. Its pattern
  `σ = pattern(x)` agrees with `L`'s certified signs. By `pairCheck_spec`, σ avoids every
  excluded pair. `anyKept` enumerates every completion of `L`'s undecided signs, keeps those that
  avoid every excluded pair, and returns `some false` only if `κ` is false on all of them
  (`some true` on the first that keeps; `none` past 12 undecided, which proves nothing). σ is one
  of them, so `κ(σ)` is false. ∎

**Witness.** The verdict `solid` needs an in-box. By 1.2 it lies in `R`, so `R` has interior.

## 2. The touch/saddle test (`QRef.localTouch`, `pinched`, `saddleBad`)

Setting: quadrics `q_i`, `q_j` and a rational `t ≠ 0`. `M = A_i + t A_j` is the matrix of
`f = q_i + t q_j`. The point `p` is rational, with `(p, 1)` in `M`'s null space and `q_i(p) = 0`.
`S` is the quadratic part of `M`, and `n = ∇q_i(p) ≠ 0`.

**Lemma 2.1.** `f(x) = S(x − p)` for all `x`, `q_j(p) = 0`, and `∇q_j(p) = −n/t`.

*Proof.* `M (p, 1)ᵀ = 0` means `f(p) = (p,1) M (p,1)ᵀ = 0` and `∇f(p) = 2 (M (p,1)ᵀ)_{xyz} = 0`.
A quadric equals its second-order Taylor expansion about any point, so `f(x) = S(x − p)`. Then
`q_j(p) = (f(p) − q_i(p))/t = 0`, and `∇q_j(p) = (∇f(p) − n)/t = −n/t`. ∎

Write `d = x − p = w + s·n` with `w ⊥ n`. To first order, `q_i(x) = s|n|² + O(|d|²)` and
`q_j(x) = −s|n|²/t + O(|d|²)`.

**Theorem 2.2 (a point touch).** Let `S|_{n⊥}` be definite, of sign `σ_Q`. Then there is a
neighbourhood of `p` in which no point has the pattern `E` that `localTouch` returns, while the
other three sign patterns of `(q_i, q_j)` occur in every neighbourhood of `p`.

*Proof.* Take `t > 0` and `σ_Q = +`; the code then returns `E = (q_i < 0, q_j < 0)`, and the
other three cases follow by the symmetries `t ↦ −t` (with `q_j ↦ −q_j`) and `S ↦ −S`.

*`E` is absent near `p`.* Suppose `q_i(x) < 0`, `q_j(x) < 0` and `x ≠ p` is close to `p`.
- Then `f(x) = q_i + t q_j < 0`, so `S(d) < 0`.
- `S(d) ≥ c|w|² − C|s||w| − C s²` with `c > 0`, so `S(d) < 0` forces `|w| ≤ K|s|` and hence
  `|d| ≤ (K + |n|)|s|`.
- Then `q_i(x) = s|n|² + O(s²)` and `q_j(x) = −s|n|²/t + O(s²)`. For `|s|` small these have the
  signs of `s` and `−s`, which are opposite. Contradiction.

*The other three occur.*
- Along `±n`, the patterns `(+, −)` and `(−, +)` occur.
- On surface `i` near `p`, at `x = p + w + O(|w|²)`: `t q_j(x) = S(x − p) > 0`, by
  definiteness. A small push off surface `i` to its positive side gives `(+, +)`. ∎

**Theorem 2.3 (the local topology).** Other quadrics are nonzero at `p`, so by `sign_nbhd` they
keep their signs on a ball `U` about `p`. By `den_iff_onSigns`, the history on `U` is a function
of `(sign q_i, sign q_j)` only, off the wall. Name the regions:
- `a` and `c`: the two patterns differing from `E` in one sign; these are the two sides that
  touch;
- `b`: the pattern opposite `E`, the pinched region between the surfaces.

Then `R ∩ U` is a 3-manifold with boundary near `p` iff the kept set `K ⊆ {a, b, c}` is one of
`∅`, `{a, b, c}`, `{a}`, `{c}`, `{a, b}`, `{b, c}`. `pinched` returns true exactly on
`{a, c}` and `{b}`.

*Proof.*
- **The two surfaces.** Near `p` both are graphs over the tangent plane, by the implicit function
  theorem (`n ≠ 0`, and `∇q_j(p) ∥ n`): `u = g_i(w)` and `u = g_j(w)`, with `u` the coordinate
  along `n`. By 2.2, `g_i − g_j` has strict sign on `w ≠ 0` and vanishes at `0`.
- **The regions** are `{u > max}`, `{u < min}` and `{min < u < max}`. The first two are `a` and
  `c` in some order; the last is `b`.
- **Manifold cases.** These sets and their unions `{u > g_i}`, `{u > g_j}`, `{u < g_i}`,
  `{u < g_j}` are each bounded by one continuous graph, so they are manifolds with boundary.
- **`{a, c}`** is the complement of a closed cusp region. Its link at `p` (its intersection with a
  small sphere) is two disjoint discs, not one, so it is not a manifold at `p`.
- **`{b}`**: its link at `p` is an annulus (the band between the two graphs on the sphere), not a
  disc. Not a manifold at `p`.
- **`∅` and all**: `p` is outside, or interior. ∎

**Theorem 2.4 (a saddle).** Let `S|_{n⊥}` be indefinite. Then all four patterns occur near `p`,
the two between-patterns each over two opposite sectors of the tangent plane, and `saddleBad`
gives exactly the non-manifold kept sets.

*Proof.*
- **The sectors.** On surface `i`, `t q_j = S(d)`, which takes both signs on `n⊥`. Its zero set
  near `p` is two curves crossing at `p` (the surfaces cross along them), splitting a
  neighbourhood into four sectors on which `g_i − g_j` alternates in sign.
- **The regions** are `A = {u > max}`, `B = {u < min}`, `X` (between, where `g_i > g_j`, two
  opposite sectors) and `Y` (between, where `g_j > g_i`, the other two).
- **Manifold cases.** The kept sets with a manifold boundary are those bounded by one continuous
  graph:
  - `∅` and all;
  - `{A} = {u > max}` and `{B}`;
  - `{A, X} = {u > g_j}`, `{A, Y} = {u > g_i}`, `{B, X}`, `{B, Y}`;
  - `{A, X, Y} = {u > min}` and `{B, X, Y}`.
- **Non-manifold cases.**
  - `{X}`, `{Y}`, `{X, Y}`: the kept set is a union of thin wedges meeting only along the
    crossing curves (pairs of opposite sectors touch at `p`; `{X, Y}` is a checkerboard).
  - `{A, B}`, `{A, B, X}`, `{A, B, Y}`: their complements are the sets just listed, and a set's
    boundary is its complement's.
- **The code.** `saddleBad` is true iff (`X` or `Y` kept, and neither `A` nor `B`) or (`A` and
  `B` kept, and not both `X` and `Y`), which is exactly these six. ∎

**Theorem 2.5 (lines of contact).** Along a contact line `P + sD` with `q_i ≡ 0` on it, the
code subdivides the line at:
- the other quadrics' roots;
- the roots of `S(w(s), w(s))` (quadratic in `s`, because `w(s) = ∇q_i(P + sD) × D` is linear);
- the roots of `S(D, w(s))` (linear).

On each open interval none of these vanishes, so the transversal test's outcome (touch along the
line with fixed `σ_Q`, or crossing) and the other quadrics' signs are constant, and one rational
sample decides the interval. The local analysis at a point of the line is 2.2–2.4, applied in the
plane `{D}⊥` through the point (the configuration is a product with the line, to first order
along it).

*This subdivision was missing until this note. The code tested one sample per interval of the
other quadrics' roots, which is unsound if `S(w, w)` changes sign inside an interval. It is
fixed in QRef/Decide.lean together with this note.* ∎

**What the hand proofs assume.** The implicit function theorem and the link criterion for
manifolds with boundary (standard real analysis and topology over ℝ; not formalised here), and
assumption B. Next step: formalise 1.1–1.5 in Lean over `Quad`. They are combinatorial and use
only the proven checkers. 2.1 is algebra. 2.2–2.4 need the IVT-level analysis that
`Alg.Model` provides.

## 3. Cone apexes (`QRef.apexDecide`)

The executable takes the apex `p` exactly, as the cone quadric's singular point (its matrix's
null vector). Let `C` be the tangent cone at `p`: the nappe about `a = s·z`, of half-angle
`α = atan(s/c)`. Let `J` be the smooth surfaces through `p` other than the nappe's own plane,
with gradients `g_j ≠ 0`.

**Lemma 3.1 (separation).** `C̄ ∖ {0} ⊂ {σ g · d > 0}` iff `σ g·a > 0` and
`(g·a)²(s² + c²) > s² |g|² |a|²`.

*Proof.* The condition says the angle between `σg` and `a` is below `π/2 − α`: its cosine
exceeds `sin α = s/√(s² + c²)`, squared. A direction in `C̄` makes an angle at most `α` with `a`,
so its angle with `σg` is below `π/2`. Conversely, if the angle is at least `π/2 − α`, some
generator of `C̄` makes an angle at least `π/2` with `σg`. ∎

**Lemma 3.2.** If every `g_j` separates `C`, then on `C ∩ B(p, ε)`, for small `ε`:
- every `q_j` has the sign `σ_j`;
- the cone quadric has the sign of its quadratic part at `a`;
- every quadric not through `p` has its sign at `p`.

So `C` near `p` carries one pattern, `P_C`.

*Proof.* On the compact link of `C̄`, `σ_j g_j·d ≥ c_j |d|` for some `c_j > 0` (Lemma 3.1).
`q_j(p + d) = g_j·d + O(|d|²)`. The cone quadric is homogeneous of degree 2 about its singular
point, so its sign at `p + d` is `Q(d)`'s, constant on `C`. Quadrics not through `p` keep their
sign near `p` (`sign_nbhd`). ∎

**Which planes count.** The nappe's own plane `N` passes through `p` with normal `a`. When it
serves only as the cone solid's selector, its sign matters only to that solid: off `C ∪ −C` the
cone quadric is outside, so the cone solid is out whatever `N`'s sign, and on `−C` the cone
solid is out by its selector. Then `J` omits `N`. When a face lies on `N`'s carrier, or another
solid's literal uses it, `N` is a surface through `p` like any other and belongs to `J`. This
holds for a box top, a cylinder cap, a base disc, or a second cone's selector. Its normal is the
axis, so it separates `C` (Lemma 3.1). (The first version dropped `N` in every case and then
skipped an empty `J`. That missed five non-manifold apexes. The repro is
`cone_apex_on_a_face_normal_to_its_axis_is_non_manifold`.)

**Lemma 3.3 (the caps).** Lemma 3.2 holds on `−C` as well, with every `q_j` of sign `−σ_j` and
the cone quadric of the same sign as on `C` (its quadratic part is even). So `−C` near `p`
carries one pattern too, `P_−C`. It matters when another solid uses the cone quadric (two cones
sharing an apex and an axis). Otherwise `−C` is out for the cone solid, and lies inside the
sector `anti` (every `J` sign `−σ_j`), where every other solid sees the same signs as on the rest
of `anti`.

**Lemma 3.4 (the cells).** Suppose `|J| ≤ 3` with independent normals. Then the link of `p` is cut
into these cells:
- the caps `D₊` (of `C`) and `D₋` (of `−C`);
- for each of the `2^{|J|}` sectors `τ` of the tangent planes, the cell `O_τ`: the sector's
  link, less the caps.

Every `O_τ` is nonempty and connected. `D₊` lies inside the sector `own`, and `D₋` inside `anti`.
`D₊` meets only `O_own`, along a circle, and `D₋` meets only `O_anti`. Two cells `O_τ` and
`O_τ'` share an arc iff `τ` and `τ'` differ in one bit. Two planes' line meets the link at two
vertices, and round each of them lie the four cells with the other bits fixed. On every cell,
membership is `κ` on one pattern:
- on `D₊`, `P_C`; on `D₋`, `P_−C`;
- on `O_τ`, the cone quadric outside, `τ`'s signs on `J`, and the other quadrics at their signs
  at `p`.

*Proof.* By the implicit function theorem (independent normals, transversal crossings), a
neighbourhood of `p` is homeomorphic to the arrangement of the tangent planes. The caps sit
strictly inside one sector each (Lemma 3.1, applied to `±a`). A polyhedral cone with at most
three faces is not a circular cone, so `own` minus `D₊` is a nonempty annulus. The planes' arcs
lie off the caps, so the adjacencies of the sectors are those of the hyperplane arrangement.
The signs are those of Lemmas 3.2 and 3.3. ∎

**Theorem 3.5′ (`linkDisc`).** The result is a manifold at `p` iff the kept cells' union `X` on
the link is empty, everything, or a closed disc. A disc means: `X` is connected through shared
arcs, so is its complement, and no vertex has its four cells kept as a checkerboard.

*Proof.* Near `p` the result is the cone over `X`, which is a manifold with boundary at `p` iff
`X` is `∅`, `S²` or a disc. Suppose `X` has no checkerboard vertex. The caps' circles carry no
vertex. Then `X` is a compact surface with boundary in `S²`. If it is connected, each boundary
circle bounds one component of its complement (Jordan), so a connected complement means one
boundary circle, and the region is a disc (Schoenflies). Conversely, a disc is connected, its
complement is a disc too, and a pinch vertex makes `X` not a surface. ∎

The earlier rule, Theorem 3.4, `(kC ∧ ¬ownKept ∧ anyOther) ∨ (¬kC ∧ ownKept ∧ ¬allKept)`, is the
case `P_−C ~ anti` of this. 3.5′ also judges the sectors' own pattern at `p`, which 3.4 left to the
other contacts.

**Theorem 3.5 (one surface cutting `C`).** Suppose `J = {j}`, no other solid uses the cone
quadric, and `j`'s tangent plane cuts `C`
transversally: `(g·a)²(s² + c²) < s²|g|²|a|²`. Then the result is non-manifold at `p` iff
exactly two opposite cells of the cycle `cap₊, cap₋, out₋, out₊` are kept. These are the cone
region and its outside, on either side of the plane.

*Proof.*
- **The link.** The link of the arrangement at `p` (in the linear model, by the implicit function
  theorem, both crossings being transversal) is the sphere. A great circle crosses a small circle
  (the cap of `C`) at two points.
- **Cells and arcs.** Four cells: the cap's two halves, and the rest of the two hemispheres, each
  connected. Each consecutive pair in the cycle shares an arc: the chord in the cap, the
  small-circle arcs, and the great-circle arc outside the cap.
- **Vertices.** Round each crossing point the four cells appear in this cyclic order.
- **The disc criterion.** A union of cells is a disc iff it is nonempty, proper, connected, its
  complement is connected, and it has no pinch at a vertex. On this cycle that is exactly one
  contiguous run. Two opposite cells form two pieces meeting at the two vertices. ∎

Anything outside the lemmas' hypotheses is reported undecided, so the verdict is ambiguous:
- a surface through `p` whose tangent plane cuts `C` when there are several, or tangentially;
- a singular second surface;
- more than three surfaces, or dependent normals.

The sampled check (`apexCheck`) is no longer used.

## 4. Curves where three or more surfaces meet, and edges touching surfaces

**Triple curves (`QRef.tripleCurves`, `curveSectors`).** On a plane `a`, two quadrics whose
restrictions are proportional (one conic) meet along that conic, with `a`. Take a rational point
`p` of it, found exactly on a rational line in the plane. Let `T` be the surfaces through `p`,
with gradients `g_j`.

**Theorem 4.1.** Suppose `|T| ≥ 3`, no gradient is zero, and no two are parallel, and all are
orthogonal to `D = g_0 × g_1` (they share the tangent line `D`). Then near `p` the result is a
3-manifold with boundary iff the kept sectors of the line arrangement `{g_j · d = 0}`, in the
normal plane `D⊥`, form one cyclic run (or none, or all).

*Proof.*
- By the implicit function theorem, the surfaces near `p` are transversal in the normal plane: no
  two normals are parallel. So a neighbourhood is homeomorphic to `(−ε, ε) × (a neighbourhood of
  0 in D⊥)`, with the surfaces mapped to the lines `{g_j·d = 0}`.
- Off the lines, membership is the history on the sector's signs; the other quadrics keep their
  signs near `p` (`sign_nbhd`).
- A union of closed sectors of a circle-arrangement is an arc iff it is one cyclic run. Its
  product with an interval is a disc (the link at `p`) iff it is an arc. Otherwise the link has
  several pieces.
- The code orders the `2|T|` rays `± g_j × D` exactly (by the half and the cross product in the
  basis `g_0`, `D × g_0`). It takes each sector's direction as the sum of its bounding rays, and
  counts the changes of kept round the cycle: more than two means more than one run. ∎

If two normals are parallel at `p`, the surfaces are tangent there, and the pencil analysis (§2)
judges it. The triple-curve test stands aside.

**Edges touching surfaces (`QRef.edgeTouches`).** These are detected exactly:
- on a plane `a`, the curve `a ∩ b` tangent to `c`'s trace at a rational point `p`;
- found as a rational root of the plane pencil `det(C_b + t C_c)`, whose member's singular point
  lies on both conics;
- with the normals of `a`, `b`, `c` pairwise non-parallel, and the history varying across the
  eight local sign patterns.

The local verdict needs second order along the edge (the edge and the surface part quadratically
away from `p`), so such a contact is reported undecided, and the verdict is ambiguous. This
replaces the hint check as the reference's own detection of these contacts.

## 5. Polynomial surfaces (tori): `QRef.Poly`

A history with a torus face takes the polynomial path. Every face is a polynomial `P3`: a torus
gives its quartic, the other faces their quadrics, and a cone also gives its nappe.

**In Lean (ForgeExact/Poly3):**
- `escape_one`: restricted to a line, a nonzero polynomial has finitely many roots (`zero_of_roots`).
- `wall_nowhereDense`, `reg_off_wall`.
- `about_bound`: the box bound about the box's centre. `eval_about` says `about p c` is `p`
  expanded about `c`.
- `in_box`, `out_box`, `empty_of_leaves`.

So the octree's volume enclosure and its `empty` verdict rest on these exactly as §1's rest on the
quadric ones: §1's argument holds word for word with `Poly3.boxSign` in place of `Quad.boxSign`.

**The manifold check (`QRef.Poly.check`), by hand.** It runs on each undecided leaf `B` of the
octree. Let `U` be the surfaces whose signs `B` leaves undecided. Every other surface keeps one
sign on `B` (`boxSign_spec`), so on `B` membership is the history on `U`'s signs alone
(`reg_off_wall`). The check accepts `B` in three cases; otherwise it splits `B`, within a budget.

- **One surface `f`, a component of `∇f` with a certified sign on `B`.** Then `∇f ≠ 0` on `B`,
  so `f = 0` is a smooth sheet in `B`. On each side membership is constant, so near every point of
  `B` the result is empty, everything, or a half-space bounded by the sheet: a manifold.
- **Two surfaces `f`, `g`, a component of `∇f × ∇g` with a certified sign on `B`.**
  - The two normals are independent at every point of `B`. So each sheet is smooth, and where they
    meet, `(f, g)` is a submersion. Near a common point `p` the map `x ↦ (f(x), g(x), t(x))`, with
    `t` a coordinate along the curve, is a local diffeomorphism. It sends the four sign sectors to
    the quadrants of the `(f, g)` plane times an interval.
  - So the result near `p` is (union of kept closed quadrants) × interval. That is a half-space
    iff the kept quadrants are one run round the cycle `(+,+), (+,−), (−,−), (−,+)`. With four
    sectors, the only kept sets that are not one run (or none, or all) are the checkerboards
    `{(+,+), (−,−)}` and `{(+,−), (−,+)}`, which `pairOk` rejects.
  - Points of `B` on one sheet only are the first case.
- **Three surfaces, `det(∇f₁, ∇f₂, ∇f₃)` with a certified sign on `B`.**
  - At a common point the map `(f₁, f₂, f₃)` is a local diffeomorphism, so the result is the cone
    over the kept closed octants' trace on the link sphere. The eight octants trace the eight
    triangles of an octahedron.
  - The cone over a closed region of `S²` is a manifold with boundary at the apex iff the region is
    empty, all of `S²`, or a closed disc.
  - A union of triangles is a disc iff three things hold: it is connected through shared edges;
    its complement is too; and round each of the six vertices, the four triangles there are not a
    checkerboard. Without a pinch vertex, the region is a compact surface with boundary. When it is
    connected, its boundary circles correspond to its complement's components, and one circle in
    `S²` bounds a disc (Schoenflies). That is `octantsOk`.
  - Points of `B` on two of the three sheets give one of the six vertices' four-cycles, which the
    same check covers. Points on one sheet give the first case.

**Touches with a torus (`touches`, `judgeTouch`), by hand.** Let `T` be a torus with an
axis-aligned axis, and work in its meridian half-plane: `ρ ≥ 0` is the distance from the axis
and `w` the height along it. The solid torus is the body of revolution of the disc `Δ` about
`(R, 0)` of radius `r`.

- **Coaxial faces.** A face `F` coaxial with `T` is the body of revolution of a region `Φ` of the
  meridian:
  - a plane normal to the axis: a half-plane bounded by `w = c`;
  - a cylinder on the axis: the strip `ρ < c`;
  - a sphere centred on the axis, or a coaxial torus: a disc.

  Two such bodies touch along the revolution of the points where `∂Δ` and `∂Φ` touch, and the
  interiors' relation is that of `Δ` and `Φ`. For a circle and a line, or two circles, tangency
  is an exact identity in ℚ: the distance from the centre to the line equals `r`, or
  `|c₁c₂|² = (r ± s)²`. The contact point is then rational, and it is unique for distinct
  curves. The plane's case depends on its outward normal: the torus lies on the plane's inner
  side iff that normal points away from it.
  - External tangency: `Δ ∩ Φ = ∅`.
  - Internal tangency: one region contains the other.
  
  Both hold globally, so the combination they exclude is empty everywhere. With `ρ_P > 0` the
  contact is a circle.
- **Planes at a point.** The solid torus's support function in direction `n` is
  `h(n) = n·o + R|n_⊥| + r|n|`. The farthest tube centre lies along `n_⊥`, and the tube adds
  `r|n|`. When `n_⊥ ≠ 0` the maximum is attained at exactly one point, `o + R n_⊥/|n_⊥| + r n/|n|`.
  So a plane `n·x = h(n)` touches the torus there and nowhere else, and the torus lies on one
  side of it. The same holds for `−n`. The test is exact when `|n|` and `|n_⊥|` are rational.
- **The other carriers** must keep one nonzero sign on the contact. At a point this is exact
  evaluation. On a circle: with `t` the rational parameter, the carrier times `(1+t²)^D` is a
  polynomial in `t`. If Descartes' rule clears every Möbius interval of `[−B, B]`, with `B` a
  root bound (`noRealRoot`, built on the proven `noPosRoot`), it has no real root. It is then
  nonzero on the circle except perhaps at `t = ∞`, which is checked directly to have the same
  sign.
- **The local rule.** Near a contact point `p` the two surfaces are tangent, and the third
  combination, which the exclusion rules out, does not occur. In the plane normal to the
  contact (for a circle, the meridian) the three occurring combinations are, along a small
  circle round `p`: the side `A`, the pinched region `pin` between the two curves, and the side
  `B`. `pin` is a cusp-shaped region that touches `p`, and the kept set is a union of these
  closures.
  - It is a half-disc, so a manifold, iff the kept set is one run: `∅`, `{A}`, `{B}`, `{A, pin}`,
    `{pin, B}` or everything.
  - It is not a manifold iff the kept set is `{A, B}` (two pieces meeting at `p`) or `{pin}`
    (a region pinched to zero thickness at `p`).
  
  For a circle the picture is this one times an interval, since the other signs are constant
  along it. This is the quadric path's `pinched` rule.
- **Exclusions in the local check.** Inside a leaf `B`, membership is `κ` on the undecided
  signs `U`. The exclusions above remove some patterns of `U` everywhere. If on the patterns
  that remain `κ` is a function of `U ∖ {u}`, then off the wall the boundary of the result in `B`
  lies on the zero sets of `U ∖ {u}`, and the regularisation does not see `u`. So the three
  acceptance tests apply to `U ∖ {u}`. A manifold touch keeps one run, so it depends on only
  one of the two surfaces, and the check accepts the leaves round it. A non-manifold touch
  depends on both.

A check that accepts every undecided leaf proves the result is a 3-manifold with boundary
everywhere off the certified boxes. Inside a certified box the result is all or nothing. Every
point of the wall lies in an undecided leaf, because a box with all signs certified holds no wall
point. Leaves the budget or the depth cut off report `und`, and the verdict is `ambiguous`.

So on this path:
- `solid` is proven: a box is certified in, and the result is a manifold.
- `empty` is proven (`empty_of_leaves`).
- `nonmanifold` comes only from a touch decided exactly as above. A failed check (an undecided
  tangency, a cone apex, a checkerboard, or four undecided surfaces) is not proof of a bad point.
- Two undecided surfaces whose normals at a box's centre are within `1e-2` rad stop the check
  at once. This is a shortcut to `und`, so it can only make the verdict ambiguous.

**What stays unproven in Lean:** the check above, and the executable `norm`, `mul3` and `deriv3`
that build the gradients and the cross products. A wrong derivative could accept a bad box. The
signs of those polynomials are taken with the proven `Poly3.boxSign`.

## 6. Completeness of root finding

Several decisions need every root of a polynomial, not only sound ones. A root that is missed
makes them wrong, not undecided. dol-curved found that `realRoots` dropped an interval still
being split when its fuel ran out. The contacts also found the pencil's rational roots in
floating point: float roots, then the convergents of their continued fractions.

- **`realRoots` now returns `none`** unless its cover is complete. An interval left at the fuel
  limit, or still on the stack at the step budget, gives `none`. Every interval of `[−B, B]`
  must be cleared by Descartes' rule (`noPosRoot`, proven) or isolated (`Iso.ok`). Above degree
  2 it works on the square-free part, whose roots are the same, so double roots no longer
  exhaust the fuel.
- **`rationalRoots`** gives every rational root of `p`, or `none`.
  - Clear `p`'s denominators. A rational root `u/v` in lowest terms then has `v | L`, the
    cleared leading coefficient.
  - Two distinct rationals with denominators dividing `L` differ by at least `1/L²`.
  - Each isolating interval is halved (exact sign bisection) below width `1/L²`. It then holds
    at most one such rational. That candidate is its simplest rational, the one of least
    denominator, and it is tested exactly.

The decisions that need completeness, and what an incomplete cover now gives:

| decision | needs | incomplete gives |
|---|---|---|
| `contacts`: the pencil's rational roots `t` | every rational root of `det(A_i + tA_j)` | an open contact, so ambiguous (its exclusion `⟨i,+,i,−⟩` excludes nothing) |
| contact line subdivision (Theorem 2.5) | every root of the other quadrics, `S(w,w)` and `S(D,w)` along the line | the contact undecided |
| isolated contact points on the pencil's null line | every root of `q_i` there | the contact undecided |
| `edgeTouches`: the plane pencil's rational roots | every rational root | an undecided note |
| conic contacts, triple curves | one rational point | fewer points, so undecided (never wrong) |
| polynomial path: constant signs on a touch circle | no real root (`noRealRoot`, complete by construction) | the touch undecided |

The volume estimate's `froots` and `nearTouch`'s float roots stay heuristic. They feed only the
estimate and near-decision notes, as documented.
