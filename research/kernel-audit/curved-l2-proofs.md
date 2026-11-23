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

**Lemma 3.3.** Suppose `|J| ≤ 3` with independent normals. Then off `C̄`, near `p`, membership is
`κ` with the cone quadric outside and the `J` signs those of the point's sector. Every one of
the `2^{|J|}` sectors meets every neighbourhood of `p` off `C̄`.

*Proof.* Independent normals: the surfaces cross transversally at `p`. By the implicit function
theorem, a neighbourhood of `p` is homeomorphic to the linear model, the arrangement of the
tangent planes, and every sign vector is realised by an open cone. `C̄` lies inside one sector
(the separating one, `own`). Removing it leaves every other sector intact, and leaves `own`
nonempty: `C̄` is a proper closed sub-cone of it. ∎

**Theorem 3.4.** Let `kC = κ(P_C)`, `ownKept = κ(own sector, cone outside)`, and let `anyOther`
and `allKept` range over the other sectors and over all sectors. The result is non-manifold at
`p` iff `(kC ∧ ¬ownKept ∧ anyOther) ∨ (¬kC ∧ ownKept ∧ ¬allKept)`.

*Proof.* By 3.2–3.3, near `p` the kept set is `(C if kC) ∪ (the kept sectors, less C̄)`, and its
link at `p` is a union of spherical regions.
- `kC` and `ownKept`: `C` lies inside a kept sector and changes nothing.
- `¬kC` and `¬ownKept`: `C` lies inside a dropped sector and changes nothing.

In both cases the apex adds no non-manifold point. The sectors' own structure at `p` is that of
the other solids, judged by the other contacts.
- `kC` and `¬ownKept`: the link is the cap of `C` together with the links of the kept sectors.
  These are disjoint, since `C̄` lies inside the dropped sector `own`. With another sector kept
  the link is disconnected, so not a disc: non-manifold. With none kept, the link is the cap
  alone, a disc (the cone's own apex): manifold.
- `¬kC` and `ownKept`: the link is the kept sectors' link with an open cap removed from the
  interior of `own`'s. If every sector is kept, that is a sphere minus a cap, a disc: manifold.
  Otherwise the kept sectors' link has nonempty boundary, and removing an interior cap leaves a
  non-disc (an annulus when it was a disc): non-manifold. ∎

Anything outside the lemmas' hypotheses is reported undecided, so the verdict is ambiguous:
- a surface through `p` whose tangent plane cuts `C`;
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
