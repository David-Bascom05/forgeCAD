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
| 2 | `QSep.lean` | Tier-0 seams from `QFace` data (`Quadric.ofPlane`): the cylinder's and cone's axial plane, the cone's nappe plane, the sphere's centre plane; contact-curve planes from `QRef.contactAt` | `seam_through_axis` and the like: each seam holds the axis or the apex exactly; the seams carry no face | next |
| 3 | `Alg/RUR.lean` | rational univariate representations for 0-dimensional systems: a conic pair on a rational plane (edge samples), three quadrics (vertices). Each is a separating linear form, its minimal polynomial `R`, and coordinates `Pᵢ(u)/Q(u)`. Subresultants are untrusted; a reduction identity is checked, as in `Alg.Sum.redCert` | `rur_spec`: every root of `R` gives a common zero, and the carrier's sign there is `Num.sign` of a univariate polynomial | |
| 4 | `QSweep.lean` | for each carrier j: critical values in a fixed rational direction (`Num`s: d-critical points of j and of each curve j∩k, crossings), and rational midpoints between consecutive values by `QRef.between`. The candidates are the flat `cand1` with algebraic roots | `cand_ok`: for every critical-free interval, one candidate lies in it (the curved `cand1_ok`) | |
| 5 | `QSample.lean` | the samples: on each sweep plane x = c and each rational line y = c' in it, the roots on carrier j (`covOn`, facet samples). Points of j∩k on sweep planes (RUR, edge samples). Vertices (RUR) | **`qsampler_covers`**: every realised cell lying in a carrier is some sample's covector. This is the curved `sampler_covers`, and L2's main theorem. It needs the delineability of the sweep (between critical values the curves on j are graphs), which is where the q1 session's Lemma D and this module meet | |
| 6 | `QLedger.lean` | `curvedLedger`: cells are the samples' distinct covectors. Pieces: φ is a piece of body i iff i's formula differs on φ.plus and φ.minus (`SForm.onCov`, `QForm.lean`). Inside: formula evaluation. Edges: covectors hit by an edge sample. Verts: those hit only by vertex samples | **`curved_exact`**: `ExactK` (and, with the q1 session's theorem, `CurvedTarget`) for every input in the Q1 domain. The dimension rule is sound because a sample on a sweep plane between critical values is never a vertex, and a nearby point of its arc has its covector | |

**Fuel.** `Num.sign` and the isolations take fuel. Spent fuel answers `none` and never a wrong
sign. The decided rule is: one fixed fuel constant per call site, stated in the module. An input
whose decision runs out of fuel is `Skip::Budget` and goes to the old path, counted. That is one
rule, as D3 asks, not a retry.

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

**Asked of the decide-once Rust session:**
1. `Big`'s API and limb layout, so that `Alg::Num` and the RUR build on it;
2. where the exact numerics live (`forge_math::exact` beside `Dy`?);
3. whether `forge_bool_core`'s `Ledger` constructor (`encode`) takes covectors of any width as
   they are (`LFits.width`). The curved ledger's width is the near surfaces plus the seams.
