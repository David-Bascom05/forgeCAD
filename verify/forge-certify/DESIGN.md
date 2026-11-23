# A proven certificate checker for flat-face booleans

Translation validation: the kernel returns a result body and a certificate; a checker written and
proven in Lean 4 (core only, beside verify/forge-exact) accepts only if the result is the exact
regularised boolean of the inputs. The theorem is *accept ⇒ correct* for any kernel, bugs
included: an accepted result is proven, a rejected one is a bug caught and becomes a refusal naming
the reason.

## 1. What is proven

Everything is stated with verify/forge-exact's definitions: `ForgeExact.reg` (closure of interior),
`ForgeExact.combine` (a pointwise boolean of a family), `ForgeExact.regBool`, and a solid's point
set `den S := fun x => S.contains x = true`, where `Ref.Solid.contains` is the reference's own
ray-parity membership (odd number of faces pierced by the +x ray, each face's loops tested in the
yz projection).

```
theorem check_sound (op : List Bool → Bool) (h0 : op (all false) = false)
    (ins : List Solid) (res : Solid) (cert : Cert) :
    check op ins res cert = true →
    reg (den res) = reg (combine (fun m => op (List.ofFn m)) (fun i => den ins[i]))

theorem check_sound_bool (o : Op) (A B R : Solid) (cert : Cert) :
    check o.fn [A, B] R cert = true → reg (den R) = regBool o (den A) (den B)
```

n-ary from the start: the binary boolean is n = 2; the n-ary regen and a whole history (`keptAt`)
are other `op`s. `h0` (nothing far away is kept) holds for union, intersection, difference and
every history.

No assumption about the inputs: if an input is not closed the checker cannot certify (it needs
closedness, §3 step 2) and rejects; the theorem never relies on the one assumption RefFacts.lean
records for the reference (ray parity constant on cells). Proven in core Lean, no `sorry`, only
`propext`, `Classical.choice`, `Quot.sound`, enforced by an `#standard_axioms` module in the
default build, as in forge-exact.

## 2. Tolerance: what the checker's relation is

The kernel snaps within LINEAR, so "the kernel's f64 result equals the exact boolean of its f64
inputs" is false by design. The checker's relation is: **the result's exact interpretation is the
exact regularised boolean of the snapped inputs.**

1. Every operand face's carrier goes through the proven rule (`ForgeExact.Snap`: `tolEq`, `cls`,
   `rep`, `spreadOk`, `snapped`), the same functions forge-exact runs. Past the spread cap the
   boolean is *out of scope (ambiguous)*, never accepted or rejected.
2. Vertices are recomputed exactly from the snapped planes round them (`Snap.vertexAt`); a vertex
   its planes do not settle is out of scope (ambiguous). `snap_result`: the configuration is
   consistent, every on/near decision made in the representative.
3. The result's faces lie on the operands' surfaces (the kernel clones them): each result face's
   plane must equal, bit for bit, an operand face's plane, and takes that carrier's snapped plane;
   a result face on no operand surface is a rejection. The result's vertices are recomputed from
   its own faces' snapped planes (its own topology, not the certificate's) and each must be within
   LINEAR of the kernel's f64 vertex, else rejected.

So `ins` and `res` in the theorem are these exact rational solids; the theorem says the result is
exactly right on them, and step 3 bounds the kernel's numbers to within LINEAR of that. Every
double is a dyadic rational: no floating-point model anywhere.

## 3. The certificate, and what the checker checks

**Emitted by the kernel** (cheap: data it already has). For each operand, the pieces its faces were
split into (the general boolean's `Plan.specs`, the n-ary boolean's regions): per piece, its source
face and its loops as vertex keys (operand vertices, or new vertices shared across operands); faces
the kernel never split come as themselves. No classification, no witness points: the checker finds
those itself, so nothing it needs is taken on trust.

**Completed by the checker** (untrusted step, outside the proof: the theorem holds for every
completion): each piece is triangulated exactly (ear clipping over ℚ). Each vertex key's exact
point is the meet of the snapped planes of every piece that uses it.

**Checked** (all exact, all of it covered by the theorem):

1. *Pieces tile their faces.* For each operand face f: `loops(f) + Σ ∂(triangles of f's pieces) = 0`
   as a mod-2 1-chain, and every triangle lies on f's plane.
2. *Operands closed.* `Σ ∂(all triangles of an operand) = 0`.
3. *Atoms.* On each plane P (n.x ≠ 0), the triangles of all operands are pairwise identical or
   interior-disjoint (2-D separating axis); identical ones are one atom τ with a multiplicity per
   operand `α_τ ∈ {0,1}^n`.
4. *Each atom avoids every other surface.* For every atom τ ⊂ P and every operand triangle u off P:
   open τ ∩ u = ∅, shown by a separating plane (3-D separating axis; soundness is convexity only).
5. *Each atom classified.* A witness w ∈ open τ, generic (its +x ray meets no edge or vertex); per
   operand, `a⁺_i(τ)` = parity of that operand's triangles off P hit by the +x ray from w.
6. *The result on every plane.* With `J(τ) = op(a⁺ ⊕ α) ⊕ op(a⁺)` (does the boolean change across
   τ along +x):  `loops(result faces on P) + Σ_{τ ⊂ P} J(τ) ∂τ = 0`. Every plane carrying a result
   face or an atom is checked, so a missing, extra or misplaced face is caught.
7. *The result closed* (a validity check beside the theorem: faces with n.x = 0 are invisible to
   +x rays, so the theorem says nothing about a result face on such a plane; closedness catches a
   missing or extra one).

"= 0 as a mod-2 1-chain": every non-degenerate segment is keyed by its line (direction normalised to
first nonzero coordinate 1, base point at that coordinate 0) and each (line, endpoint) must occur an
even number of times. This is subdivision-invariant, so T-junctions (an input edge split where the
other solid crosses it) need no split lists.

A rejection names the first check that failed and where: which operand, face, atom, plane, point.

## 4. The proof

Generic points: off a finite union of planes built from the data (face and triangle planes; per
segment line, the plane through it along +x; per pair of planes, the plane through their meet along
+x; per vertex, z = const; per atom witness w and vertex or line-meet m, the plane through w, m
along +x). `Flat.wall_nowhereDense` makes it nowhere dense, and `reg_eq_of_sdiff` reduces the
theorem to: at every generic x, `res.contains x = op(ins_i.contains x)`.

Lemmas, bottom up (all over ℚ, each a finite sign argument):

* **IP (interval parity).** For affine g₁..g_k on [s₀, s₁], nonzero at the ends, no two vanishing at
  one inner point: `[all g > 0 at s₀] ⊕ [all g > 0 at s₁]` is the parity of the inner roots where
  every other g is positive. The one engine of every crossing argument below.
* **Z (zero 1-chains).** If a chain is zero (§3) and a crossing predicate has the form "the point
  φ(L) its line determines lies inside the segment", the predicate's sum over the chain is even
  (each `[q ∈ (a,b)] = [a <_L q] ⊕ [b <_L q]`, and the endpoint multiset is even).
* **Faces to triangles.** At generic x, `Face.pierced f x` = parity of f's triangles hit (crossesYZ is
  such a predicate; a triangle's three crossings are its interior, by IP along the ray). So
  `contains` is the triangle soup's ray parity.
* **Constancy (PT).** For an atom τ ⊂ P, generic e ∈ open τ and the witness w: per operand
  triangle u off P, `hit(u,e) ⊕ hit(u,w) = ⊕_{edges E of u} shadow(E; e, w)` when u misses [e,w]
  (step 4), by IP along [e,w] (cases: u vertical; an edge of u on P, apex on either side; otherwise).
  The shadows sum to zero over a closed operand (Z, step 2; P's own edges cast none). So
  `a⁺_i(e) = a⁺_i(τ)` on all of τ.
* **Along one line (telescoping).** At generic x, the +x line meets distinct planes at distinct
  points; sorting them and telescoping `op` from far away (`op(0) = 0`) gives
  `op(ins(x)) = Σ_P J_P(e_P)`. Steps 1, 3 and Z make the operands' and result's crossings at e_P the
  atom counts; step 6 makes `res.contains x = Σ_P J_P(e_P)`.

The checker the executable runs is these very definitions (as forge-exact's reference is), so the
theorem is about the code that runs.

## 5. Cost

Per boolean, with T_i triangles per operand and t atoms (t ≈ ΣT_i):

| step | work |
|---|---|
| tiling, closedness, result chains (1, 2, 6, 7) | O(ΣT log ΣT): hashing segment keys |
| atoms per plane (3) | Σ_P (atoms on P)², box-culled; small in practice |
| avoidance (4) | t × T, box-culled to the pairs whose boxes meet |
| classification (5) | t × T ray tests (a box reject for nearly all) |

The kernel's own boolean is O((F_A + F_B) log + pairs) in f64. The checker is quadratic in exact
rationals: for the fuzz corpus' parts (tens of faces) a few ms; for a 1000-face part with a small
tool, roughly 10⁵–10⁶ box tests and 10⁴ exact ray tests, ~0.1–1 s, i.e. 100–1000× the kernel's
boolean. **So it runs in CI and tests, plus an optional app switch** (certify every boolean of a
part on demand), never always-on. The kernel side costs nothing when the feature is off (cfg'd
out, proven by the callgrind gate) and, when on, one copy of the plan's pieces per boolean (<10% of
the boolean). An index on the witness rays (sort by projected box) would make steps 4–5
O(T log T + hits); left for later, as it changes only speed.

## 6. Kernel emission (`certify` feature, off by default)

`forge-bool` gains `[features] certify = []`. Under `#[cfg(feature = "certify")]` only: after
`plan` (general path) and after region classification (n-ary path), the pieces are pushed to a
thread-local sink with the operands and op; `forge_bool::certify::take()` hands them out. Nothing
the kernel decides reads the sink, so results are bit-identical; with the feature off the code is
not compiled, so the gate shows no instruction change. Paths not instrumented (the local-imprint
path, off by default) give no certificate: reported out of scope, not accepted.

## 7. The harness (verify/forge-certify)

A Rust crate runs every flat boolean of the forge-exact fuzz corpus (families, plane_plane,
notched with its 400 ulp stages, suite FUZZ_SEEDS=150, lang) with the feature on, collects each
boolean's certificate, runs the compiled checker (one process, one JSON line per boolean, numbers
as exact `[m, e]`), and reports accepted / rejected (with reasons) / out of scope, plus the
checker's time against the kernel's per boolean. Every rejection is cross-checked against
verify/forge-exact/fuzz/tests/repros.rs (bugs 1–4) and the near-coincident ulp family; a rejection
that is not a kernel bug is a checker bug, and is fixed.

## 8. Scope and limits

* Flat faces only (every edge a line, every face a plane); anything else is out of scope.
* The theorem is about the point set under +x ray parity; result validity on faces with n.x = 0 is
  checked by closedness only (running the check in the y and z frames too would make it
  direction-free at 3× the cost: an option).
* Completeness is not proven: a correct result can be rejected if the kernel's pieces are not a
  common refinement or a separating plane is not found. Each such case is a checker bug to fix.
