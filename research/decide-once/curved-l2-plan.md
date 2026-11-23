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
| 3 | `QEdge.lean` (the plane-section RUR) | edge samples: a point where `q_j = q_k = 0` crosses a rational plane `x = c`, given by `y = α` (a root of `R`) and `z = −N(α)/D(α)`. The data come from untrusted elimination and are checked by identities: `homog q = D²·q(c, y, −N/D)` must equal `R·Q` for both quadrics. Vertices (three quadrics) follow the same pattern | **done** (sound half): `homog_ev`, `onCurve_spec` (both quadrics vanish at the sample), `edgeCov_spec` (the decided covector is the sample's). A `#guard` runs the unit sphere against z = 0. **Completeness done**: `bezout_spec`: a Bezout certificate `A·q_j + B·q_k = R` in ℚ[y][z], checked by `bzero`, makes every crossing of the curve with the plane a root of `R` (with `#guard`). **Next**: vertices, then an untrusted proposer (resultant, subresultant, Bezout cofactors) |
| 4 | `QSweep.lean` | for each carrier j: critical values in a fixed rational direction (`Num`s: d-critical points of j and of each curve j∩k, crossings), and rational midpoints between consecutive values by `QRef.between`. The candidates are the flat `cand1` with algebraic roots | `cand_ok`: for every critical-free interval, one candidate lies in it (the curved `cand1_ok`) | |
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
