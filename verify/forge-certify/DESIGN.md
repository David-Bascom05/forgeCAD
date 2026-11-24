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

As proven (ForgeCertify/Theorem.lean, Frames.lean; `Opd` is an operand's faces with, face by
face, its triangles, the completed certificate):

```
def opSet (op : List Bool → Bool) (ins : List Opd) : Pred V3 :=
  fun x => op (ins.map fun o => (Solid.mk "" o.faces.toArray).contains x) = true

theorem check_sound (op) (ins : List Opd) (res : List Face) :
    check op ins res = true → reg (den res) = reg (opSet op ins)

theorem check_sound_bool (o : Op) (A B : Opd) (res : List Face) :
    check (binOp o) [A, B] res = true → reg (den res) = regBool o (den A.faces) (den B.faces)

theorem check_sound_xyz (op) (ins) (res) :
    (∀ k < 3, check op (ins.map (cycOpdN k)) (res.map (cycFaceN k)) = true) →
    ∀ k < 3, reg (denAxis k res) = reg (opSetAxis k op ins)
```

n-ary from the start: the binary boolean is n = 2; a whole history is another `op`. `check` itself
checks `op (all false) = false` (nothing far away is kept). `check_sound_xyz` is the direction-free
form: `denAxis k` is ray parity along axis k (`cyc : (x,y,z) ↦ (y,z,x)` applied k times, shown a
homeomorphism, so `reg` commutes with it), and the driver runs exactly these three checks. Axis 0
is `check_sound` itself (`denAxis_zero`).

No assumption about the inputs: if an input is not closed the checker cannot certify (it needs
closedness, §3 step 2) and rejects; the theorem never relies on the one assumption RefFacts.lean
records for the reference (ray parity constant on cells). Proven in core Lean, no `sorry`, only
`propext`, `Classical.choice`, `Quot.sound`, enforced by ForgeCertify/Axioms.lean (`#certify_axioms`,
forge-exact's elaborator) in the default build.

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
completion): each piece is triangulated exactly over ℚ by vertical slabs in its plane's 2-D frame,
split at every vertex and along every operand edge lying in that plane (so a tool touching a face
along a line inside a piece gives triangles that meet the other operand's triangles edge to edge,
or not at all). Each vertex key's exact point is the meet of the snapped planes of every piece and
face that uses it; when those planes span only a line or one plane (a vertex on a seam between two
faces of one surface, or one splitting an edge), the kernel's point put exactly on them.

**Checked** (all exact, all of it covered by the theorem):

1. *Pieces tile their faces.* For each operand face f: `loops(f) + Σ ∂(triangles of f's pieces) = 0`
   as a mod-2 1-chain, and every triangle lies on f's plane.
2. *Operands closed.* `Σ ∂(all triangles of an operand) = 0`.
3. *Atoms.* On each plane P (n.x ≠ 0), the triangles of all operands are pairwise identical or
   interior-disjoint (2-D separating axis); identical ones are one atom τ with a multiplicity per
   operand `α_τ ∈ {0,1}^n`.
4. *Each atom avoids every other surface.* For every atom τ ⊂ P and every operand triangle u off P:
   open τ ∩ u = ∅, shown by a separating plane (3-D separating axis test: a plane at u's extreme
   along each of the coordinate axes, the two normals, edge × normal and edge × edge; soundness is
   convexity only).
5. *Each atom classified.* A witness w ∈ open τ (the first of 13 fixed positive weightings that is
   untied), generic (off every other triangle's plane, and its yz shadow off every vertex and, for
   a triangle not along x, off its edge lines); per operand, `a⁺_i(τ)` = parity of that operand's
   triangles off P hit by the +x ray from w.
6. *The result on every plane.* With `J(τ) = op(a⁺ ⊕ α) ⊕ op(a⁺)` (does the boolean change across
   τ along +x):  `loops(result faces on P) + Σ_{τ ⊂ P} J(τ) ∂τ = 0`. Every plane carrying a result
   face or an atom is checked, so a missing, extra or misplaced face is caught.
7. *The other two frames.* Steps 1–6 again with the coordinates cycled once and twice: a result
   face on a plane along x (invisible to +x rays) is seen along +y or +z (`check_sound_xyz`).

"= 0 as a mod-2 1-chain": every non-degenerate segment is keyed by its line (direction normalised to
first nonzero coordinate 1, base point at that coordinate 0) and each (line, endpoint) must occur an
even number of times. This is subdivision-invariant, so T-junctions (an input edge split where the
other solid crosses it) need no split lists.

A rejection names the first check that failed and where: which operand, face, atom, plane, point.

## 4. The proof

Generic points: off a finite union of planes built from the data (`walls`, Walls.lean: per
segment end, z = const; per segment, the plane through it along +x; per pair of planes, the plane
through their meet along +x; per plane P and triangle plane Q, Q lifted along +x onto P; per atom
witness w and triangle vertex v, the planes along +x through w and v and through w and the point of
Q at v's level). `gen_of_walls`: off them, when the checker accepts, nothing is tied. `Flat.wall_nowhereDense` makes it nowhere dense, and `reg_eq_of_sdiff` reduces the
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
* **Faces along x (PV).** A triangle u with n.x = 0 is hit by no ray; across it the sweep [e, w]
  is handled in u's own 2-D frame (`pt_vert`), so the constancy above covers every operand face.
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
boolean.

**Where it runs** (owner's ruling: what is proven for all cases needs no proof at runtime). The
decide-once boolean is proven end to end, so re-checking every boolean at runtime would add cost
and no assurance. The checker runs only in tests, the fuzzers (forge-exact-fuzz, the box family)
and CI, as an independent cross-check of the proof's assumptions about the shipped code, and in an
explicit on-demand command (e.g. `forgecad check --certify F`). Never on a normal build, edit or
render. The checker itself stays Lean-proven. The kernel side costs nothing when the feature is off (cfg'd
out, proven by the callgrind gate) and, when on, one copy of the plan's pieces per boolean (<10% of
the boolean). An index on the witness rays (sort by projected box) would make steps 4–5
O(T log T + hits); left for later, as it changes only speed.

## 6. Kernel emission (`certify` feature, off by default)

`forge-bool` gains `[features] certify = []`. Only while `certify::record(true)` is set: before
and after `plan` (the general path) the operands, op and pieces go to a thread-local sink, the
result is attached when the boolean returns, and `forge_bool::certify::take()` hands them out; the
local-imprint path and the n-ary boolean note that they gave no certificate
(`certify::take_skipped()`). Nothing the kernel decides reads the sink.

The hooks are line-neutral: each call site names a hook on its own line (`planned`,
`general_into`, `took_imprint`, `nary_noted`), and the hooks are defined at the end of
boolean.rs and nary.rs. With the feature off they are `use` aliases of the plain functions, so
every line of the off build is the line it was (a hook statement inside a function moved every
panic location below it, which changed LLVM's internal symbol names and, through the symbol
table's length, valgrind's startup layout by a few instructions). Measured: the off build's
`regen1` and `bits` are machine-code identical to base (every loaded section byte for byte; only
the build id and symbol names differ), `benchparts` too once forge-lang's kernel-source hash
(which changes with any kernel file, by design) is the same, and the callgrind gate at equal paths
gives all 93 lines equal to base and the bits identical.

## 7. The harness (verify/forge-certify)

A Rust crate runs every flat boolean of the forge-exact fuzz corpus (families, plane_plane,
notched with its 400 ulp stages, suite FUZZ_SEEDS=150, lang) with the feature on, collects each
boolean's certificate, runs the compiled checker (one process, one JSON line per boolean, numbers
as exact `[m, e]`), and reports accepted / rejected (with reasons) / out of scope, plus the
checker's time against the kernel's per boolean. A boolean done by a path that gives no
certificate (the local-imprint path, the n-ary boolean) is counted out of scope with that path's
name, never dropped. `repros` runs every part and pair of verify/forge-exact/fuzz/tests/repros.rs
and the drafted lid (coverage G5); `--mutate` sends each accepted result again wrong on purpose (its
first and last face dropped, the op changed) and counts the verdicts, so the checker is seen to
reject as well as accept. Every rejection is cross-checked against
verify/forge-exact/fuzz/tests/repros.rs (bugs 1–4) and the near-coincident ulp family; a rejection
that is not a kernel bug is a checker bug, and is fixed.

## 8. Scope and limits

* Flat faces only (every edge a line, every face a plane); anything else is out of scope.
* The theorem is about the point set under ray parity; with the three frames (the default) it holds
  along each axis, so no plane is a blind spot. One frame (`"frames": "x"`) is a third of the cost
  and proves `check_sound` alone.
* The relation is exact equality with the snapped inputs. A vertex whose snapped planes do not meet
  in one point (the kernel merged a near-coincidence within LINEAR, so the exact boolean has a
  sliver there that the kernel's result has not) is out of scope, as in forge-exact's reference
  (`Ref/Input.lean`, the same note).
* Completeness is not proven: a correct result can be rejected if the kernel's pieces are not a
  common refinement or a separating plane is not found. Each such case is a checker bug to fix.

## 9. Results (this branch, three frames)

`cargo run -p forge-certify --release --bin certify -- --time` (families, plane_plane, notched
with its 400 ulp stages, suite FUZZ_SEEDS=150, lang 192 seeds): **2073 booleans: 1363 accepted,
0 rejected, 710 out of scope**: 339 the kernel refused (no result to check), 194 a curved edge, 134
done by the n-ary boolean (not instrumented), 43 a vertex the kernel settled by tolerance (6 whose
snapped planes do not meet in one point, 37 a crossing snapped onto an edge it is not exactly on):
there the snapped inputs' exact boolean has a sliver the kernel's result has not.

`certify repros plane_plane --mutate`: every repro of verify/forge-exact/fuzz/tests/repros.rs and the
drafted lid. **One rejection, a real kernel bug**: `near_tolerance_45` boolean 4, the kernel keeps a
wall at x = 50.00002 whose class snaps to 50.000005 (1.5e-5 off, over LINEAR): repros.rs
`near_tolerance_45_chained_walls_are_one`. The others: the corner-touch pinch, `near_tolerance_24`
and `_53`, the two- and three-axis bands, `coincident_walls_1`'s last stage and the notched part at
ulp 0 are refused by the kernel (out of scope: nothing to certify); the slit edge, the touching
subtract (its extra imprint face is the right point set), the notched part at ulp 13, the n-ary
lost-face part (now fixed) and the drafted lid at 0.1° and 0.2° are accepted; the drafted lid at
0.05° (G5b, a corner within LINEAR of the block's side) is out of scope as a tolerance merge.
Mutants (each accepted result sent with its first or last face dropped, or claimed for another op):
240 of 242 rejected; the 2 accepted are `identical`'s A∪A claimed as A∩A and back, the same set.

Cost (`--time`, the checker's process against the kernel's boolean, 3 reps): 1406 booleans checked
in 3201 s against the kernel's 0.12 s: 2.3 s per boolean on average, per-boolean ratio median 7515×,
p90 74229×, slowest check 307 s. Far over §5's estimate: the checker is all-pairs in exact
rationals with no spatial index and runs three frames. It runs where §5 says (tests, fuzzers, CI,
an on-demand command), never on the build path.

`certify suite --mutate` and `certify families more plane_plane notched lang --mutate` on 9880fd4a
(this checker as merged; release; two runs, both exit 0):

| run | accepted | rejected | out of scope | face-dropped mutants | op-changed mutants |
|---|---|---|---|---|---|
| suite | 244 | 0 | 0 | 488 rejected | 488 rejected |
| families, more, plane_plane, notched, lang | 1524 | 0 | 785 | 2976 rejected | 3045 rejected, 3 accepted |

Out of scope in the second run: 342 the kernel refused, 267 an edge not straight, 131 the n-ary
boolean, 45 a vertex not one point of the planes (and the edge) round it. **Open**: those 45 vertices
are listed by coordinate in the run's output and are handed to FLAT-SITES, who owns vertex
placement. Each one is either a tolerance merge (out of scope by design) or a vertex the kernel put
in the wrong place. Which, case by case, is UNKNOWN.

The 3 accepted mutants are all A op A on equal solids, where union and intersection are the same
point set (equivalent mutants, not a checker that accepts a wrong certificate):
`plane_plane/identical/Add` (union claimed as Intersect), `plane_plane/identical/Keep` (inter
claimed as Union), and `degen_plane_plane/5`. The last is seed 5 of `degenerate("plane", "plane")`:
tool `5 % 15` = index 5 of `degen_tools("plane", "plane")`, `identical` (the base block,
(-20,-20,-20) 40×40×20), with op `DEGEN_OPS[(5 / 15) % 3]` = Add (n = 15 tools)
(kernel/forge-model/src/adversarial.rs).

**Next steps, for resuming** (stopped here by the owner's priorities):
1. Instrument the n-ary boolean (`nary_with`: its regions as pieces), so its 134 booleans and the
   n-ary lost-face family are certified instead of counted out of scope.
2. A spatial index for the checker: sort the witness rays and the separation pairs by projected
   box, making steps 4–5 O(T log T + hits) instead of all pairs. Speed only: nothing proven
   changes.
3. The on-demand command (`forgecad check --certify F`) and a CI job running
   `certify --suite 150` (and `repros --mutate`).
