# Q1 L2: the executable curved ledger, and the Rust plan (2026-11-23)

Context: research/decide-once/curved-design.md (§2, §4 Q1). This note splits Q1's L2, the exact
`curvedLedger` in core Lean over `Alg`, into modules with their theorems. It also gives the plan for
`decide_once_curved.rs`. Q1's Mathlib proofs (Lemma D, `Faithful`, `xNonManifold_iff_top`) belong
to the q1 session (branch suffix `dol-curved-q1`). This side builds the executable and proves it
equal to its spec.

## 1. The shape: the flat sampler, generalised

The flat L2 is `sampleLedger ps ss (sampler ps)` (Sample.lean, Sampler.lean). It has three parts:
- *sample points* that hit every realised cell lying in a carrier (`Covers`, `sampler_covers`);
- *the ledger read off them*: the cells are the samples' distinct covectors, the pieces come from a
  flip on both sides, and the edges and verts are the listed cells of dimension 1 and 0;
- *exactness* (`sample_exact`) from `Covers` and the input's semantics.

The curved L2 has the same three parts. On the formula route (D2) the second part needs no
point-in-solid test.

**Samples, never walls.** The enumerator sweeps in order to *find* cells, as the flat sampler
does. Its sweep planes are sampling devices, not carriers. Only the separators of design §2.3
enter the covector. With `Faithful` (classes connected), a covector names its cell, so a sample
set that hits every cell is all the enumerator needs.

## 2. Modules (core Lean, verify/forge-exact/ForgeExact/DecideOnce)

| # | Module | Content | Main theorem | Status |
|---|---|---|---|---|
| 1 | `QLine.lean` | `Quadric.evK` (a quadric's value in any model K), `linePt`, `covOn`: the exact covector of the point that a number `n` names on a rational line | `alongLine_ev`, **`covOn_spec`**: the decided covector is `covOfK` of the point, in every model | **done** |
| 2 | `QSep.lean` | Tier-0 seams from `QFace` data (`Quadric.ofPlane`): the cylinder's and cone's axial plane, the cone's nappe plane, the sphere's centre plane; contact-curve planes from `QRef.contactAt` | `seam_through_axis` and the like: each seam holds the axis or the apex exactly; the seams carry no face | **done**: `axisPlane_on`, `axisPlane_nz`, `nappePlane_on`, `centrePlane_on`, `equatorPlane_on`, `noScaled_seams` (contact-curve planes still to add) |
| 3 | `QEdge.lean` (the plane-section RUR) | edge samples: a point where `q_j = q_k = 0` crosses a rational plane `x = c`, given by `y = α` (a root of `R`) and `z = −N(α)/D(α)`. The data come from untrusted elimination and are checked by identities: `homog q = D²·q(c, y, −N/D)` must equal `R·Q` for both quadrics. Vertices (three quadrics) follow the same pattern | **done** (sound half): `homog_ev`, `onCurve_spec` (both quadrics vanish at the sample), `edgeCov_spec` (the decided covector is the sample's). A `#guard` runs the unit sphere against z = 0. **Completeness done**: `bezout_spec`: a Bezout certificate `A·q_j + B·q_k = R` in ℚ[y][z], checked by `bzero`, makes every crossing of the curve with the plane a root of `R` (with `#guard`). **Vertices done** (`QVert.lean`): `homog3_ev`, `onVert_spec` (all three quadrics vanish), `vertCov_spec` (the decided covector is the vertex's), with `#guard`s at (1, 0, 0). **Plane-and-two-quadrics proposer done** (`QVertPlane.lean`, mirroring the Rust's Milestone 1d): `proposeVertPlane_complete`, every vertex of a plane (`lz ≠ 0`) and two quadrics is a proposed sample's point, with its `y`; `#guard`s incl. the tilted plane. The y/z exchange done (`proposeVertPlaneAny_complete`). **Three-quadric proposer done** (`QVert3.lean`, mirroring the Rust's Milestone 1e: two projections, a checked gcd, the linear remainder for `y`, `N + D z` for `z`): `proposeVert3_complete`; `#guard` on two spheres and a cylinder (4 vertices). **Vertex completeness done**: `bezout3_spec`: `A·q_a + B·q_b + C·q_c = R(x)` over ℚ[x][y][z] (`TPoly`, `zHom`), so every vertex's `x` is a root of `R` (with `#guard`). **Edge proposer done** (`QPropose.lean`): closed-form `R`, `N`, `D` and linear Bezout cofactors (`elim`); `proposeEdge_complete`: every crossing of the curve with the plane is a proposed sample's point. **The shear** (`shear_ev`): two crossings sharing a `y` make `D ≡ 0` and the proposal is refused; every carrier is then sheared `(x, y, z) ↦ (x, y + λz, z)`, with `λ` the first of the fixed list `shears` for which every proposal succeeds (the decided rule; past it, `Skip::Budget`). **Next**: the vertex proposer (three-quadric elimination), then QSweep |
| 4 | `QSweep.lean` | the sweep's events, all reduced to vertex samples: a curve's turning points are vertices of `q_j, q_k, critQ q_j q_k` (`critQ` = `(∇q_j × ∇q_k)ₓ`, a quadric); a surface's critical points are vertices of `q, ∂_y q, ∂_z q` (two planes); and the carriers' own vertices. Event values are the vertices' `x`-roots, ordered by `Num.cmp`, with rational sample planes between them | **events' meaning done**: `critQ_ev`, `gradPlaneY_ev`, `gradPlaneZ_ev` (and `dyK_grad`, `dzK_grad` against `Quadric.grad`), with `#guard`s on the unit circle's turning point. **Waits on** the vertex proposer (from the curved Rust session's elimination) for the events' completeness |
| 5 | `QSample.lean` | the samples: on each sweep plane x = c and each rational line y = c' in it, the roots on carrier j (`covOn`, facet samples). Points of j∩k on sweep planes (RUR, edge samples). Vertices (RUR) | **`qsampler_covers`**: every realised cell lying in a carrier is some sample's covector. This is the curved `sampler_covers`, and L2's main theorem. It needs the delineability of the sweep (between critical values the curves on j are graphs), which is where the q1 session's Lemma D and this module meet | |
| 6 | `QLedger.lean` | `curvedLedger`: cells are the samples' distinct covectors. Pieces: φ is a piece of body i iff i's formula differs on φ.plus and φ.minus (`SForm.onCov`, `QForm.lean`). Inside: formula evaluation. Edges: covectors hit by an edge sample. Verts: those hit only by vertex samples | **`curved_exact`**: `ExactK` (and, with the q1 session's theorem, `CurvedTarget`) for every input in the Q1 domain. The dimension rule is sound because a sample on a sweep plane between critical values is never a vertex, and a nearby point of its arc has its covector | |

**Fuel.** `Num.sign` and the isolations take fuel. Spent fuel answers `none` and never a wrong
sign. The decided rule, accepted by the master on 2026-11-23: one fixed fuel constant per call
site, recorded here and in the module. An input whose decision runs out of fuel is
`Skip::Budget`, goes to the old path, and is counted.

| Constant | Value | Call sites | Defined in |
|---|---|---|---|
| `signFuel` | 200 (`Num.sign` bisections, as exact-curved's contact decisions use) | `covOn`, `edgeCov`, `ESample.dOk` | QEdge.lean |
| `rootFuel` | 60 (`QRef.realRoots`' default) | root isolation of `R` and of the line restrictions | QEdge.lean |
| (sizes) | three-quadric test, the curved Rust session (172a608): R_A of degree 8 with coefficients up to 93 bits; A, B, C 569 rationals of at most 84 bits; ~0.4 s in release through `Big`'s public API. No cap needed at this size; re-measure on larger families | `proposeVert3` | — |
| `shears` | `[0, 1, 2, 3, −1, 1/2, 5, −3, 7, 1/3]` | the generic projection, one `λ` per boolean | QPropose.lean |

**Root isolation is certified, not trusted** (`RootCover.lean`). `QRef.realRoots` is incomplete:
an interval still being split at the fuel limit is dropped silently (`if d ≥ fuel then continue`),
losing its roots. The curved ledger takes its roots from a checked cover instead:
- breakpoints, with each gap root-free or isolating one root;
- both rays root-free by Descartes on the shifted polynomial.

`cover_complete` proves every root in any model is in `coverRoots`. `certifiedRoots` is the
proposer, and it answers `none` when the cover does not check (`Skip::Budget`). Square-free inputs come from `proposeSqfree`,
checked both ways (`sp^k = p·w` and `p = sp·h`; `sqfree_roots`). **`rootsOf_complete`**: if
`rootsOf p fuel = some ns`, every root of `p` in any model is denoted by some number in `ns`.

**The sweep statements** are committed by the LemmaD session (46369a9 on `dol-curved-lemmad`,
ForgeCurved/Sweep.lean): `Slab`, `Transverse`, `NoTriple`, `NoCrit`, `ZeroGraphs`, `CrossesAll`,
`GraphLemma`, `FacetCrossing`. `crossesAll_edges` (edge cells cross every sweep plane) is proven.
They hold over any real normed space with a linear sweep, so the sweep-plane version is an
instance. `qsampler_covers` is stated against them.

**Correction to the critical set: properness.** Zeros over compact sub-slabs must be bounded
(`Proper`). xy = 1 in z = 0 against the sweep x has no critical point, yet its branches are graphs
over only part of a slab containing x = 0. The direction rule therefore also requires that no curve
has an asymptotic direction within a level plane, a leading-form test checked exactly.

**Obligation from A8 (the master, 2026-11-23): every arc at a vertex is found.** The face
builder's sectors need it, at transversal vertices from the tangents' exact order, and at
tangential vertices from the crossings on a certified box's faces (`proposeEdgeAny_complete`).

**Order of work:**
1. QSep (small);
2. RUR (medium; it is the exact core of edges and vertices);
3. QSweep;
4. QSample;
5. QLedger.

`qsampler_covers` is proven last, with the q1 session's delineability lemma as its one geometric
input.

## 3. The Rust: `kernel/forge-bool/src/decide_once_curved.rs`

The core is `forge_bool_core`, unchanged. Its `Ledger`, `Sgn`, `CellId`, `Rule` and `Outcome` are
the interface. The bridge mirrors the Lean modules one to one, so that L4 can refine each
function to its Lean counterpart.

| Rust | Lean | Filter / exact |
|---|---|---|
| `carriers` (QFace → `Quadric` with dyadic coefficients, `canon`) | `QRef.Faces`, `QRef.canon` | exact (dyadic) |
| `seams` | `QSep` | exact |
| `crit` (critical values, isolated) | `QSweep` | float roots with certified separation; `Alg` isolation as the fallback |
| `samples` | `QSample` | interval Krawczyk boxes (trace's `Interval`); RUR as the fallback |
| `cov_of` (a sample's covector) | `covOn`, `rur_spec` | interval sign on the sample's box; `Num.sign` (Big-based) as the fallback |
| `formula` (`SForm` eval on a covector) | `SForm.onCov` | exact, combinatorial |
| `build` (cells, pieces, inside, edges, verts → `Ledger`) | `QLedger` | exact, combinatorial; Aeneas L4 target |
| `write` (fresh B-rep: faces from carriers, edges from arcs, vertices rounded once) | — | checked by `forge_topo::check` and the exact comparison, as F3 is |

**Aeneas style** (until research/aeneas/STYLE.md lands, the rules of
research/kernel-audit/decide-once/aeneas/README.md):
- no struct holds a borrow;
- no `break`, and no `return` inside a loop;
- no `Ordering` (comparisons return −1, 0 or 1);
- no local named like a generated namespace.

**Numerics (D6):** the bignum and the RUR are written in the same style, with inline limbs, as
the decide-once Rust session is doing for `Big`. They are interim L4 for now, checked by a
differential test against the Lean L2 with a committed digest, and the blocker is named:
Aeneas and bignum arithmetic.

**Answered by the decide-once Rust session (2026-11-23):**
1. **The bignum.** `forge_math::exact::Big` (kernel/forge-math/src/exact.rs, feature `exact`, as of
   b6a9dd4) has one representation per value:
   - `S(i128)` below 2^126;
   - `M { neg, n, w: [u64; 8] }` up to 8 inline limbs;
   - `L { neg, mag: Vec<u64> }` beyond that.

   Its API: `zero`, `from_i64`, `is_zero`, `signum`, `neg`, `abs`, `add`, `sub`, `mul`, `shl`, `bits`,
   `cmp_big`. `Dy` (dyadic) builds on it. There is no division, gcd or sqrt: the curved side adds
   them (rationals over `Big`, then `Alg::Num`).

   It is not yet in the Aeneas style (`scratch` takes a closure, and `cmp_mag` returns `Ordering`);
   a restyle is promised to the IEEE session. **The curved side writes against the API above, never
   the internals.**
2. **Where.** In `forge_math::exact`, behind the `exact` feature. The RUR and algebraic numbers go
   in a submodule, `forge_math::exact::alg`. There is one bignum.
3. **Width.** Ledger cells are `Vec<Vec<Sgn>>` of any one width. `LedgerWF` bounds counts, not
   width, so near surfaces plus seams as classes is fine.

The flat bridge is being restructured for speed (a body-local read, then a sparse arrangement).
`decide_once_curved.rs` shares code with it only through `forge_math::exact` and
`forge_bool_core`, never through `decide_once.rs` internals.

## 4. Rust status (branch `dol-curved-rust`)

**Milestone 1 (2026-11-23): the numeric core and the four checked modules.** Feature
`forge-bool/decide_once_curved`, off by default. Nothing in `boolean.rs` reaches it yet, so on or
off every boolean is the old path's, bit for bit; the gates hold trivially (no gate line runs new
code).

| Rust | Lean | State |
|---|---|---|
| `forge_math::exact::alg`: `Q` (rationals over `Big`, normalised, `x/0 = 0`), `Poly` ops (`padd` … `sqfree`), `moebius`, `no_pos_root`, `one_pos_root`, `Iso::{ok, zero_at}`, `Num::{cmp_rat, bisect, refine, sign_nz, sign, cmp}`, `real_roots`, `between`, `simplest`, `root_bound`, `rat_sqrt` | Alg/Poly, Alg/Model, Alg/Real, QRef/Decide | done, line for line (list lengths included) |
| `decide_once_curved`: `V3`, `Plane::{eval, canon}`, `Quadric::{eval, grad, quad, canon}`, `QFace::{quadric, nappe, wf}`, `centred` | Ref/Geom, Quad/Basic, QRef/Faces | done |
| `along_line`, `sgn_of_int`, **`cov_on`** | QLine | done |
| `off_axis`, `axis_plane`, `nappe_plane`, `centre_plane`, `equator_plane`, `seams_of`, `classes_of`, `seams` | QSep, Classes | done |
| `restr`, `homog`, `ESample::{on_curve, d_ok}`, **`edge_cov`**, `restr_b`, `bpadd`/`bpmul`/`bzero`, **`bezout`**; the proposer `propose_edge` | QEdge | done; the proposer is new (below) |
| `homog3`, `VSample::{on_vert, d_ok}`, **`vert_cov`** | QVert | checker done; proposer next |
| `SForm::{on_cov, below}` | QForm | done |
| QSweep, QSample, QLedger, write | — | next, in that order |

`Skip` has `OutOfDomain`, `Uncertified`, `Ambiguous` and `Budget`. Fuel: `SIGN_FUEL = 200`,
`ROOT_FUEL = 60` (§2), and `REAL_ROOTS_STEPS = 4000` (the Lean's loop bound).

**Milestone 1b: RootCover and QPropose mirrored** (after the master's correction).
- `QRef.realRoots` is no longer mirrored. It drops an interval still being split when its fuel
  runs out, so it can lose roots silently.
- Roots now come from `alg::roots_of`, a mirror of RootCover.lean (`propose_sqfree`, checked both
  ways, then `certified_roots`, checked by `cover_ok`). Its answer is complete or `None`.
- `propose_edge` is QPropose.lean's, with the same `elim` (`R`, `N`, `D`, and the cofactors
  `U`, `V`), `mk_edge` (quotients `a₂`, `b₂`) and checks (`bezout`, `on_curve`, `d_ok`). Its
  answer is complete (`proposeEdge_complete`) or `None`, which the bridge counts as
  `Skip::Budget`.
- `Quadric::shear` and `shears()` mirror the shear rule.
- `bezout3` and `restr_t` mirror QVert's completeness check.
- The polynomial operations are generic over a coefficient ring (`alg::Coef`: `Q`, and lists of
  a `Coef`), so `Poly`, `BPoly` and `TPoly` share one `padd`/`pmul`, as the Lean's do.

**Float filters** (`FILTERS` in the module, for the IEEE session):
- `sign_filter_iv`: Horner in `forge_math::Interval` (one ulp outward per operation), with each
  rational coefficient enclosed by floats checked exactly (`Q::lo_f64 ≤ v ≤ Q::hi_f64`, by
  `Q::cmp_f64`, a cross-multiplied comparison). An enclosure excluding 0 is the sign. Precondition:
  every bound finite (checked; otherwise the exact path).
- `tighten`: the enclosure of an isolated root, bisected in floats. It keeps `a ≤ lo < hi ≤ b`
  with filter-decided opposite signs of `p` at `lo` and `hi`, so the root (the only one in
  `(a, b)`, by `Iso.ok`) stays inside. A midpoint the filter cannot sign ends it.
- A filter's answer is the true sign, so it equals `Num.sign`'s whenever that one answers
  (`sign_spec`); debug builds check this at every call. Where `Num.sign` would run out of fuel,
  the filter's answer stands. **For L4** the Lean side needs that rule stated: a `signF` that is
  "the filter's sign, else `Num.sign`", with the filter abstracted as any correct partial sign
  oracle. Then `covOn_spec` and the like hold for it unchanged.

**L6 / interim L4.** `digest::exact_digest` hashes the exact path's outputs over a fixed family
(66 quadric pairs × a sweep plane: `propose_edge`'s samples and their covectors over 12
carriers; `roots_of` on 40 random cubics and quartics). The constant is committed, and the test passes natively and on
wasm32-wasip1 (native = wasm). The differential against exact-curved's QRef families waits for
QSample/QLedger; it needs `lake` here (not installed in this container).

**Found on the way, for the Lean side (dol-curved):** a rule for the filter's sign (above). The
non-square-free gap of milestone 1 is closed by RootCover's `proposeSqfree`. Tier 1's parallel
walls are withdrawn (design §2.3, 57b537a), and the Rust builds none of Tier 1 until the
replacement is agreed.

**Cost, measured** (release, the digest family, milestone 1): the 66 edge proposals took 143 ms,
about 2 ms each, and the 150-odd covectors over 12 carriers took 42 ms. With RootCover the whole
digest test runs in 0.36 s. Of that, 76% is `gcd` normalising
rationals, done through `Big`'s public API (Euclid with a float-estimated quotient, corrected
exactly). The fix is in `Big`: a `divmod` and a `gcd` (or an inline-word accessor). They are
asked of the decide-once Rust session, which owns it.

**Milestone 1c (2026-11-23).** The linear edge case is mirrored: `elim_lin`, `propose_edge_lin`
and `propose_edge_any` (QPropose.lean at 29e00be), with its `#guard` as a test. The digest is
re-committed and equal natively and on wasm. **Retarget (the master, same day):** the runtime
ledger moves to A8, the face ledger (curved-design.md §7). Separators, Tier 1 included, move into
the reference, which never runs. So the covector-specific parts here (`cov_on`, `edge_cov`,
`vert_cov`, `classes_of`, `SForm::on_cov`) are paused until the v2 interface is fixed. The exact
numerics and the certified proposers carry over as the builder's exact fallback.

**Milestone 1d: the vertex proposer, one plane and two quadrics** (drafted in Rust for
dol-curved's Lean `proposeVert`; untrusted, checked by `bezout3`, `roots_of`, `on_vert`, `d_ok`).
For a plane `P = n·p − c` with `n.z ≠ 0`, and quadrics `q_a`, `q_b`:
- substitute `z = Z(x, y) := c/n_z − (n_x/n_z) x − (n_y/n_z) y` (`subst_z`, Horner in `z` over
  `ℚ[x][y]`). This gives conics `f_a(x, y) = q_a(x, y, Z)` and `f_b`, quadratic in `y` over `ℚ[x]`
  (trailing zero coefficients trimmed);
- `elim_of(f_a, f_b)` is QPropose's `elim` on these restrictions, with `y` in place of `z` and `x`
  in place of `y`. It gives `R(x)`, `N`, `D` (so `y = −N/D`) and `U`, `V` with
  `U f_a + V f_b = R`;
- the quotient by `P` (`quot_plane`): `q = K P + f` with `K = k₀ + k₁z`, `k₁ = t₂/n_z` and
  `k₀ = (t₁ − k₁p₀)/n_z`, where `q = t₀ + t₁z + t₂z²` over `ℚ[x][y]` and `P = p₀ + n_z z`;
- **the Bezout certificate**: `R = U q_a + V q_b − (U K_a + V K_b) P`, so `A = U`, `B = V` and
  `C = −(U K_a + V K_b)`, each with `z`-degree ≤ 1;
- **the representation**: `N_y = N` and `D_y = D`. From `z = [(c − n_x x) D + n_y N] / (n_z D)`,
  `N_z = −((c − n_x x) D + n_y N)` and `D_z = n_z D`. The quotients `Q_a`, `Q_b`, `Q_c` are
  `pdiv(homog3 q, R)`, checked by `on_vert`. In closed form, `Q_a = n_z² D² a₂` with `a₂` the
  `y²` coefficient of `f_a`, and `Q_c = 0`;
- `n_z = 0 ≠ n_y`: the same with `y` and `z` exchanged (`Quadric::swap_yz`, `tswap_yz` on the
  cofactors), then mapped back (`N_y ↔ N_z`, `D_y ↔ D_z`). All checks run on the original
  quadrics;
- `n = (n_x, 0, 0)` (a plane `x = const`), or `D ≡ 0` (two vertices with one `x`): `None`. The
  decided shear or rotation rule takes these.

**Milestone 1e: the vertex proposer, three quadrics** (drafted in Rust for dol-curved's Lean
mirror; untrusted, checked by `bezout3` twice, a gcd identity, `roots_of`, `on_vert` and `d_ok`).
- `elim_gen` is QPropose's `elim` over any coefficient ring (`elim_of` is its `Poly` instance,
  unchanged; the digest holds). Over `ℚ[x][y]` it gives `R₁ = Res_z(q_a, q_b) = U₁q_a + V₁q_b`
  and `R₂ = Res_z(q_a, q_c) = U₂q_a + V₂q_c`, along with `D z + N` for `(q_a, q_b)`.
- **The chain** (`chain`): a pseudo-remainder sequence in `y` that tracks cofactors, `f = sR₁ + tR₂`.
  Each step is `f ← lc(g) f − lc(f) yᵏ g`, the same for `s` and `t`. Then the common content in
  `ℚ[x]` of `f`, `s` and `t` is divided out (exactly; anything else fails `bezout3`). It ends at
  `R(x) = S R₁ + T R₂`, free of `y`. So `A = S U₁ + T U₂`, `B = S V₁`, `C = T V₂`. The last
  remainder of `y`-degree 1, `p₁y + p₀`, gives `y = −p₀/p₁`. A vanishing remainder (a common
  factor: a shared curve) answers `None`. At most `CHAIN_STEPS = 8` steps.
- **Two projections.** A on the quadrics as given, and B on the quadrics sheared by the first
  `λ ≠ 0` of `shears()` whose chain exists and checks. The shear keeps `x`, so every vertex's `x`
  is a root of `R_A` and of `R_B` (`bezout3` on each, B's on the sheared quadrics, by `shear_ev`).
- **The narrowing**: `G = gcd(R_A, R_B)`, checked as `G = u R_A + v R_B` (`eqv`), so every
  vertex's `x` is a root of `G`. `deg G = 0` gives no vertices.
- **The samples**, on `roots_of(sp)` with `sp = propose_sqfree(G)`:
  - `N_y = p₀`, `D_y = p₁`;
  - `N_z = N(x, −p₀/p₁)·p₁^m` and `D_z = D(x, −p₀/p₁)·p₁^m`, homogenised (`hom_y`), with `m`
    the larger `y`-degree of `N` and `D`;
  - `Q_a = pdiv(homog3 q_a, sp)` and likewise for `b`, `c`;
  - every sample must pass `on_vert` and `d_ok`, or `None`. An extraneous root common to both
    projections is `None`, never wrong.
- Two unit spheres (centres 0 and (1/2, 1/3, 1/5)) and the z-cylinder of radius 3/4 at
  (1/4, 1/8): `R_A` of degree 8, 4 vertices, every covector `(0, 0, 0)`. It takes 352 ms in
  release, through `Big`'s public API.

**Milestone 1f: the plane vertices mirror QVertPlane.lean (4279e5d).** `subst_z_q`, `plane_z_of`,
`on_plane`, `mk_vert_p`, `quot_p`, `propose_vert_plane (qa, qb, P)` and
`propose_vert_plane_any` (the exchange, with `on_vert`/`d_ok` rerun on the originals) replace
the 1d draft. `VSample::swap_back`, `Quadric::swap_yz` and `gcd_ok` mirror the Lean, `elim_of` is
renamed `elim_t` (QPropose's `elimT`), and `propose_vert3` checks its gcd by `gcd_ok`. All the
`#guard`s are tests, and so is kind (c): the sphere with `∂_y`, which has `lz = 0` and goes through
the exchange, and `∂_z`. **Sizes** on the three-quadric test: `R_A` has degree 8, with
coefficients up to 93 bits; `A`, `B`, `C` hold 569 rationals of at most 84 bits.

**Milestone 1g: three-quadric vertices mirror QVert3.lean (5da8e4e).** `zcoef`, `elim_z`, `bdeg`,
`blc`, `bzero_q`, `bcontent`, `bdivc`, `prem` (content divided out at every step, `PREM_FUEL = 64`),
`chain` (`CHAIN_FUEL = 16`, the last linear remainder trimmed), `proj`, `comb3`, `lin_ok`,
`hom_sub`, `bpad`, `try_shear`, `second_proj` and `propose_vert3` replace the 1e draft. The roots
come from `roots_of G`, the samples' `R` is `sqfree G`, and `len L ≤ 2` is required. Same test,
same answer (4 vertices), same certificate sizes. It takes 0.89 s in release, against the draft's
0.44 s, for the per-step content gcds and the two extra checks. The `Big` divmod/gcd request is
the lever.

**Milestone 1h: `Big` division and gcd, owned here (the master, 2026-11-23).** These are
additions to `forge_math::exact`; no existing function is changed:
- `Big::divrem`: truncating; i128 when both operands are inline, else Knuth's algorithm D on the
  32-bit limbs (`divrem_mag`);
- `Big::gcd`: binary on `u128` once both fit, else Lehmer's algorithm L on the leading 64 bits,
  falling back to a full `divrem` when a run is empty. The 2×2 matrix is unimodular, so the gcd
  is kept whatever the run.

In `alg`, `divmod_nat` and `gcd_big` call them, and `Q::add` and `Q::mul` normalise by Henrici
(gcds of the small operands only). The result is the same normal form, so the digest is
unchanged. Oracles: 3000 random cases of up to 600 bits against shift-and-subtract division and
Euclid, and 2000 against `Q::ratio`, native and wasm.

The three-quadric vertex case dropped from 0.89 s to 37–74 ms wall (noisy on 4 shared cores).
In exact callgrind counts, `propose_vert3` went from 8.57 G to 0.39 G instructions. Part of that
is that `homog3` is built once per quadric, with `on_vert`'s identity checked on it once per
proposal, since every sample shares it. **Gate:** the `.text` of the speed gate's binaries
(`regen1`, `bits`, `benchparts`) is byte-identical before and after, so every gate line and
every bit is unchanged.

**Milestone 1i: the second axial seam and the event assembly.**
- `axis_plane2` (QSep.lean b626391, the master's decision) is mirrored: cylinders seam on
  `[axis_plane, axis_plane2]`, cones on `[axis_plane, axis_plane2, nappe_plane]`.
- QSweep.lean's event assembly (096b703) is mirrored: `grad_y`, `grad_z`, `aff_mul`, `qsub`,
  `crit_q`, `grad_plane_y/z`, `is_plane_q`, `propose_triple` (the Lean's role order), `tri`,
  `pairs`, `event_triples`, `is_const_q`, `det3_q`, `num_x_q`, `propose_triple_x`, `raw_events`,
  `sort_nums` (QRef's), `separate`, `iso_ok_n`, `sep_ok`, `spread_e`, `check_events`,
  `pick_events`, `events_of` and `planes_of`. The `#guard`s are tests: for
  `[sphere, y = 0, z = 0]` the events are `(−1, −1), (1, 1)` and the planes `−2, 0, 2`.
- QSample.lean and QLedger.lean are covector-specific, and paused per the A8 retarget. Whether to
  mirror them as the L6 reference executable is asked of the master.

**Milestone 1j: the executable L2 reference** (the master: mirror QSample and QLedger as the L6
reference, kept out of the production path). `kernel/forge-bool/src/decide_once_curved_ref.rs`,
compiled only for tests and under the new `curved_reference` feature (fuzzing), never reached from
`boolean.rs`. It holds:
- QSweep's event assembly, moved here from the bridge;
- QSample (`y_turn`, `turns_on`, `edges_on`, `line_samples`, `edge_sample`, `plane_samples`,
  `num_y_q`, `num_z_q`, `vert_sample`, `triple_samples`, `vert_samples`, `samples_at`,
  `samples_of`);
- QLedger (`cov_le`, `zero_idx`, `is_facet`, `plus`, `minus`, `dedup`, `RefLedger` as Lean's
  `Ledger` with `facets`, `topes` and `split_ok`, `form_of`, `curved_ledger`, `sides_ok`,
  `qledger`);
- QArc (`swap_xy`, `swap_xz`, `Axis`, `axis_perm`, `face_crossings`, `box_faces`,
  `box_crossings`, `face_cov`).

Every Lean `#guard` is a test and agrees. For `[sphere, y = 0, z = 0]`, the samples give 27
distinct covectors, one of them the vertex `(0, 0, 0)`. With body `lit 0 neg`, the ledger has 4
facets, 4 edges and 1 vertex. The unit circle against the box `[−1/2, 3/2] × [−1/2, 1/2]²` crosses
the faces `[2, 0, 2, 2, 0, 0]` times, all with `λ = 0`.

**`Big` merged** (d3d3f6e): forge-math's `exact` is now the decide-once branch's `S/M/L` `Big`
(33b461f), with `divrem` and `gcd` ported onto its 64-bit limbs. On it the three-quadric vertex
takes 31 ms and 0.27 G instructions.
