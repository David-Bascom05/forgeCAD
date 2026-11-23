# What of the kernel Aeneas can take, and what it takes to get the rest

> **Status, 2026-11-23: closed.** The owner de-prioritised proving the whole kernel
> (research/PROOF-STANDARD.md, "Proving the actual code"). This page is written up as it stood.
> Modules it did not run module by module are **not surveyed, by the owner's priority**. The
> topology-core proposal (§ "The main proposal") is **deferred**, not approved. The opaque markers
> stay usable in a translate.conf.

Every `kernel/*` module (268 files, about 200,000 lines) was put through Charon and Aeneas as the
code is today (aeneas `2e3d448` + `tools/aeneas/patches`, charon `9e193f0`, release build with
overflow checks), and the output through Lean against Aeneas's library and the float bridge.
This page says, for each module, whether it translates, what stops it, and the smallest
Aeneas-friendly factoring. It ranks the modules by value (what the spec sessions' Lean models
cover) and by effort, and it ends with one L4 target per spec session.

How the numbers were made, and how to remake them:
- `tools/aeneas/survey-modules.sh CRATE..` translates each module on its own
  (`--start-from crate::module`), so one item that fails does not hide the rest. It reports what
  Aeneas attempted and could not translate, and whether the output elaborates in Lean
  (`tools/aeneas/leancheck.sh`).
- `tools/aeneas/survey.sh` does a crate at once and attributes the results by source span. It is
  faster, but cross-module failures cascade there, so where both ran this page uses the module
  run.
- The static columns count, in each module's non-test source, what the style guide
  (research/aeneas/STYLE.md) names as blockers.
- `research/aeneas/survey_report.py` writes the tables below from those outputs.

"Translates" (class A) means that Aeneas reports no failure for the module and the Lean
elaborates. The functions are then ready for refinement proofs like those in
`verify/aeneas/forge-math`.

## What stops the rest, by how much of the kernel it stops

1. **The geometry and topology types (blocks most of the kernel).** `forge_geom::Curve` has
   `Intersection(Arc<IntCurve>)` and `Procedural(ProcCurve)`, a trait object, and `Surface` has
   `Procedural(Proc)`. `forge_topo::Edge`/`Face` hold a `Curve`/`Surface`, a `Cached<Box3>`
   (interior mutability), and `Face::query: Cached<Arc<dyn Any + Send + Sync>>`. Aeneas cannot
   translate these type groups ("Encountered an error when analyzing the following type
   declarations group: forge_geom::curve::Curve"). It then cannot translate `Edge`, `Face` or
   `Body`, nor any function that names one. That is every module in class C below: topology, the
   booleans' shell, the ops, sheet metal, meshing of a body, STEP, the model.
   - **The factoring.** It is the decide-once one, generalised. Split a `Body` into a float-free,
     dyn-free **topology** (vertices, edges, coedges, loops, faces and shells as plain ids and
     incidence arrays, plus each face's surface *kind* and sense) and a **geometry table** indexed
     by face and edge, which stays in Rust and is not opened by translated code.
   - **The core and the kernels.** Algorithms whose decisions are combinatorial take the topology
     and a ledger of decided facts. Examples are the validity check's manifold, orientation and
     loop conditions, unify, names and the uv loop logic. Float kernels take the analytic surface
     structs (`Plane`, `Cylinder`, `Cone`, `Sphere`, `Torus`), which are plain `f64` records and
     translate. Procedural surfaces stay behind the table.
   - **The cost** is a data-structure change in forge-topo. It is not done here. It is the one
     change that would move the most modules from C to A/B, and it needs its own design under
     the gates.
2. **Iterator adaptors (in most class B modules).** `zip`, `map`, `filter`, `all`, `any`,
   `enumerate`, `fold`, `collect` and `rposition` are fields Aeneas's Lean `Iterator` does not
   have. Lean fails with "`zip` is not a field of structure `core.iter.traits.iterator.Iterator`".
   Two ways out:
   - define the adaptor as std does, in the bridge (`AeneasF64/Core.lean` does `sum`);
   - rewrite to index loops (STYLE.md rule 5).

   The second costs speed only where LLVM's inlining changes (STYLE.md, "Speed"), and must be
   measured.
3. **Closures that capture mutably, or that are passed to an adaptor.** "Unimplemented", or a
   type group for the closure that fails with the types it captures (`Body::edge_ids::{closure}`).
4. **Tool limits on the largest modules.** Charon needs more than 8 GB for `forge_topo::mass`,
   `forge_ops::fillet` and `forge_ops::direct`. Aeneas runs out of heap on `forge_ops::chamfer` and
   `forge_ops::loft`. `forge_ops::fillet_cliff` does not compile under Charon's driver. A whole-crate
   run of forge-topo, forge-ops, forge-bool or forge-dwg exceeds 12 GB in Charon.
   - For `mass.rs` the cause is the face integration (`Body`, `Rc<Way<'a>>`, `&'static` rule
     caches). Its quadrature kernels (`fejer`, `chebyshev_integrals`, `lobatto`,
     `coefficient_matrix`, `eigen`, `harmonics`, `root_line`) go through Charon and Aeneas on their
     own, and fail in Lean only on `map`/`sum`.
   - The remedy is the same factoring: small pure kernels, translated on their own with
     `--start-from`.
5. **Aeneas's other limits:**
   - "Mixed-recursive declaration groups are not supported";
   - "Early returns inside of loops" and "Returns inside of nested loops";
   - "a lifetime constraint relating …";
   - "Could not match the contexts" (a shared borrow live across a write);
   - "Joining region abstractions containing shared loans" (a struct holding a borrow, in
     `forge_font::testing`).

   Each is one of STYLE.md's rules.
6. **Not code to translate.** These are I/O, the GPU probe, file system, git, and the test
   harness: forge-material's `save` and forge-gpu's `probe` ("Ignoring the body … because of"
   unsupported I/O), forge-dirs, forge-git, forge-alloc (raw pointers), forge-harness. They are
   outside the proof standard's features and stay untranslated.

## Summary

306 modules (round 6's tree), classified by `survey_report.py`:
- **A, translates, 21.** forge-math `dmath`, `frame`, `vec`, `interval`, `hash`, `rng`, `tol`, `trace`,
  `clock`; forge-topo `name`, `small`; forge-sheet `relief`; forge-ops `roles`; and the crate
  roots. `predicates` is A too since 6a6534d, translated through `verify/aeneas/forge-math`
  (its row below predates that).
- **B, small gaps, 106.** No shared geometry types. The failures are local: iterator adaptors,
  closures, early returns, lifetimes. The rewrites are the style guide's.
- **C, blocked by the shared types, 177.** These name `Body`, `Face`, `Edge`, `Curve`, `Surface`,
  `Solid`, `Feature` or `Part`. They need §1 below first.
- **D, tool limits, 2.** Also several C modules hit Charon/Aeneas memory or crash limits.

Already done at L4 on the real code, in `verify/aeneas` (this branch and the sessions'):
- forge-math: predicates (`two_sum`, `split`, `two_product`, `estimate`, the `cross_sign` and
  `orient2d` fast paths), dmath (`k_sin`, `k_cos`, `rem_pio2`, `sin_cos`, `sin`, `cos`, with
  contract C1), and `frame`/`vec` translated;
- forge-weld (support, moments: weld-spec);
- forge-sheet-core (sheet-spec);
- forge-bool-core (decide-once);
- forge_geom::spline's `lerp`, `tridiagonal`, `decasteljau`, `insert_knot` (spline-spec).

Caveats on the tables:
- forge-geom ran module by module for `blend`, `budget` and `corner` only. Each of its modules
  pulls in about 900 functions through `Curve`/`Surface`, so the rest have the crate run (which
  failed) and the static counts.
- Lean errors reported as `…from_residual` came from a bug in leancheck, now fixed, so the next
  error in those modules is not known.
- The crates round 6 added (forge-weld, forge-sheet-core, forge-par, forge-bool-core, …) have
  static counts only.

## The `dyn` question: a polymorphism bridge, or closed enums?

**What exists.**
- **Charon** translates trait objects. It emits vtables, and `transform_dyn_trait_calls` turns a
  call on `&dyn Trait` into a call through the vtable.
- **Aeneas** at the pin (2e3d448, Nov 2026) already has the existential bundle the owner
  describes. Its Lean library has `structure Dyn (Trait) where self : Type; inst : Trait self;
  value : self`. `Box::new(x) as Box<dyn Trait>` translates to `Dyn.mk _ inst x` (its test
  `tests/src/dyn.rs`). What it does not translate:
  - a method call through the object (trait-ref kind `Dyn`: "Dynamic trait types are not
    supported yet");
  - `dyn` in signatures and back-types;
  - multi-trait objects (`dyn T + Send + Sync`: "Unsupported use of dyn traits");
  - `Arc`, which its library does not model.

  I found no tracking issue. Runtime Verification report using Hax for "some dynamic-dispatch
  shapes" Aeneas can't take. [1]
- **Verus** lists trait objects as not supported (feature guide, 2024). Discussion #1047
  designs them as dictionary datatypes, which must be proof/tracked values so that specs cannot
  fabricate them. [2] [3]
- **Kani** handles dyn by model checking through vtables, with vtable function-pointer
  restrictions (VanHattum et al., ICSE SEIP 2022). [4] [5]
- **Creusot**: nothing found.

**What the kernel's `dyn` is.**
- `forge_geom::ProcCurve(Arc<dyn ProceduralCurve>)` has about 17 implementations, and
  `Proc(Arc<dyn Procedural>)` (surfaces) about 15. They are not closed within forge-geom:
  forge-step (`Nurbs`, heal, fit), forge-thermo (`Scaled`), forge-topo's mass (`Held`) and
  forge-ops' rebuild add their own.
- Both traits have `as_any`, and callers downcast. forge-bool downcasts to `blend::Spring` and
  forge-topo's mass to `corner::HolePatch`.
- `Face::query: Cached<Arc<dyn Any + Send + Sync>>` is downcast to forge-bool's `FaceInfo`.

**What was measured here, with no code change (Charon `--opaque`).**
- Make opaque the two dyn wrappers (`forge_geom::curve::ProcCurve`, `forge_geom::surface::Proc`),
  `forge_geom::intcurve::IntCurve` (the `Arc`) and `forge_topo::body::Cached` with its impls.
- Then **no type group fails**. `Curve`, `Surface`, `Edge`, `Face` and `Body` translate as
  ordinary types with opaque fields, and their Lean types elaborate. 468 of 468 bodies of
  `forge_topo::body` translate, and 540 of 542 of `forge_topo::check`.
- What is left:
  - the known iterator gaps;
  - Aeneas robustness: one more uncaught lookup when extraction projects a field of a type it
    did not translate (two such were fixed in aeneas-robust.patch this round);
  - Aeneas's memory on a 540-function module (more than 13 GB).
- This is the cheapest unblocking: the procedural cases become opaque values, and proofs cover
  the analytic cases. A procedural surface reached in a theorem's computation makes that
  theorem say nothing, exactly as the predicates' `Big` fallback does.

**Recommendation, per blocked type.**

| type | recommendation | why |
|---|---|---|
| `ProcCurve`, `Proc` (`Arc<dyn …>`) | **now:** Charon `--opaque` in each translate.conf. **later, if procedural geometry needs proofs:** a closed `enum` of the forge-geom kinds, plus one `External(Arc<dyn …>)` arm (opaque) for the out-of-crate ones | Patching Aeneas for dyn method calls would translate the call, but a proof about `d.inst.method d.value` can only be generic over every possible implementation. A closed enum gives each kind its own case. Enum dispatch is usually at least as fast as a vtable call, but it changes inlining. Measure it on the exact gate (the predicates patch showed that inlining is where speed moves). |
| `Arc<IntCurve>` | an index into a curve table owned by the body (the geometry table of §1), or `--opaque` now | sharing without `Arc`; clone cost unchanged |
| `Cached<Arc<dyn Any>>` (`Face::query`) | a typed cache field, `Cached<forge_bool::FaceInfo>` behind a crate boundary, or moved out of `Face` into the booleans' own per-face table | downcast is reflection. It has no clean model, and the cache is not part of a face's meaning. |
| `Cached<Box3>` and other `OnceLock` caches | out of the translated types (the table), or `--opaque` | interior mutability, which Aeneas does not model; the cached value is a function of the entity |

A `dyn` patch to Aeneas (method calls through `Dyn.inst`, multi-trait objects, `Arc` as `Box`)
is feasible, the same kind of work as the f64 patch. It does not make the procedural
geometry provable, though: its trusted base is the vtable, and its theorems quantify over all
implementations. I did not prototype it, and I don't recommend it ahead of the opaque markers
plus §1.

## The main proposal: a float-free, dyn-free topology core, and a geometry table

The decide-once boolean already works this way: a `Ledger` of decided facts, and a core that
translates. Generalised to the B-rep:
- **Shape.**
  - `forge_topo::Topo { vertices, edges: Vec<TEdge>, coedges, loops, faces: Vec<TFace>, shells }`,
    where `TEdge { start, end, coedges, alive, name }` and `TFace { loops, shell, same_sense,
    kind: SurfaceKind, alive, name }` are plain ids, bools, names and small enums.
  - `Geom { curves: Vec<Curve>, surfaces: Vec<Surface>, tol: …, caches }` is indexed by edge
    and face id.
  - `Body { topo: Topo, geom: Geom, joins, … }`. Accessors stay as they are
    (`body.edge(id).curve` becomes `body.geom.curves[id]`), so callers change little.
- **What it unblocks.**
  - Everything that decides on incidence alone: the validity check's manifold, orientation and
    loop-closure conditions (fundamentals' `check_sound`); unify's merges (decide-once); names;
    uv loop bookkeeping; the boolean's combinatorial stages beyond forge-bool-core; sheet's strip
    tree over faces; pattern and transform bookkeeping. These are roughly the C modules of
    forge-topo, forge-bool, forge-ops and forge-sheet.
  - Each such function takes `&Topo` (and a ledger of decided geometric facts where it needs
    them), not `&Body`.
  - Float kernels take the analytic surface structs (`Plane`, `Cylinder`, `Cone`, `Sphere`,
    `Torus`), which are plain `f64` records and translate through the bridge.
- **Migration order.** Each step is its own commit under the gates:
  1. introduce `Geom` and move `Curve`/`Surface` and the caches into it, with the accessors
     unchanged (mechanical);
  2. split `Edge`/`Face` into `TEdge`/`TFace` plus the table;
  3. move the validity check's combinatorial part to `&Topo` and translate it;
  4. unify, names and uv;
  5. the booleans' and sheet's combinatorial stages;
  6. only then, if wanted, the closed-enum procedural kinds.
- **Speed risk.** Splitting the structs changes the memory layout. An edge's curve moves from
  inside the `Edge` to a parallel vector, which costs one more cache line per access in loops
  that read both. It can also win where loops touch only topology. Bits are identical by
  construction (no float op changes). Every step is measured on the exact callgrind gate, with
  a per-function profile, and reverted if any kernel line rises. The predicates patch showed
  that a layout-only move in libc `memcpy` is told apart from a real cost that way.

## L4 targets, one per spec session, ranked by value then effort

| session | target | class | effort | why first |
|---|---|---|---|---|
| ieee | the rest of dmath: `atan`, `atan2`, `asin`, `acos` (this branch's `Dmath.lean` pattern), then the predicates' slow paths (now translated: Shewchuk's expansion invariants) | A | S–M | the translations exist and the models are final |
| fundamentals | `forge_math::frame` and `vec` (rigid transforms, A, translated in forge-math); then `forge_topo::check` after §1 step 3; mass's quadrature kernels (`fejer`, `chebyshev_integrals`, `lobatto`, `eigen`), which translate on their own (`--start-from`) except `map`/`sum` | A / C | S / L | transforms now; check needs the topology core |
| sheet-spec | `frame.compose` for the fold side (done); `forge_sheet::relief` (A); `region`/`form` after §1 | A / C | S / L | |
| spline-spec | the rest of `forge_geom::spline` by `--start-from` (as already done for four functions) | B | M | |
| decide-once | forge-bool's `classify`/`unify` combinatorics after §1; `decisions.rs` fails under Charon's driver | C | L | |
| fillet-spec, sweep-spec, ops-spec | the analytic kernels inside `blend`/`corner`/`screw` by `--start-from` on pure functions; the ops themselves need §1 and the opaque markers | C | L | |
| trace | `forge_math::interval` (A) for Krawczyk, then `intersect` after the markers | A / C | S / L | |
| weldments | done (`support_loop`, `line_moments`); `reach`/`cutting` next, same crate | — | M | |
| sketch | `forge_sketch::solve` | B | M | |
| fix-mesh-asm, mesh-export | `forge_mesh::cdt` (no `Body`; iterator gaps only), `stl` | B/C | M | |

## References

[1] https://runtimeverification.com/blog/from-rust-source-code-to-mathematical-proof
[2] https://verus-lang.github.io/verus/guide/features.html
[3] https://github.com/verus-lang/verus/discussions/1047
[4] https://www.amazon.science/publications/verifying-dynamic-trait-objects-in-rust
[5] https://cs.wellesley.edu/~avh/dyn-trait-icse-seip-2022-preprint.pdf
    Aeneas: https://github.com/AeneasVerif/aeneas (backends/lean/Aeneas/Std/Primitives.lean `Dyn`, tests/src/dyn.rs)

## Per module

Columns: lines (non-test); value H/M/L (H: a feature with an L1–L3 model; M: in the proof
standard's table; L: not a geometric feature); class; static counts of the blockers; what Aeneas
and Lean said (module run where there is one, else the crate run).
<!-- generated by research/aeneas/survey_report.py; classes over 306 modules: {'A': 21, 'B': 106, 'C': 177, 'D': 2} -->

### forge-alloc (468 lines; crate run: ok)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `lib.rs` | 468 | L | B | 0 | 0 | 0 | 0 | 0 | 4 | 0 | 16 fns, 2 bodies failed; lean: Funs: Sv/Funs.lean:160:65: error(lean.synthInstanceFai |

### forge-asm (6190 lines; crate run: ok)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `bom.rs` | 106 | M | C | 5 | 0 | 0 | 0 | 5 | 0 | 0 | 20 defs, 0 bodies / 25 sigs failed, 9 Lean errors (crate run) |
| `check.rs` | 1680 | M | C | 9 | 0 | 10 | 3 | 99 | 8 | 0 | 214 defs, 24 bodies / 301 sigs failed, 0 Lean errors (crate run) |
| `diag.rs` | 229 | M | C | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 25 defs, 3 bodies / 31 sigs failed, 0 Lean errors (crate run) |
| `joint.rs` | 274 | M | B | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 14 defs, 7 bodies / 30 sigs failed, 0 Lean errors (crate run) |
| `lib.rs` | 181 | M | B | 0 | 0 | 0 | 0 | 10 | 0 | 0 | 16 defs, 4 bodies / 36 sigs failed, 0 Lean errors (crate run) |
| `model.rs` | 692 | M | C | 10 | 0 | 3 | 0 | 4 | 0 | 0 | 147 defs, 0 bodies / 228 sigs failed, 2 Lean errors (crate run) |
| `plan.rs` | 799 | M | C | 9 | 0 | 0 | 0 | 40 | 5 | 0 | 68 defs, 9 bodies / 106 sigs failed, 0 Lean errors (crate run) |
| `qr.rs` | 144 | M | B | 0 | 0 | 0 | 0 | 15 | 0 | 0 | 17 defs, 1 bodies / 17 sigs failed, 0 Lean errors (crate run) |
| `solve.rs` | 1787 | M | C | 1 | 0 | 1 | 0 | 150 | 5 | 0 | 235 defs, 43 bodies / 299 sigs failed, 0 Lean errors (crate run) |
| `step.rs` | 298 | M | C | 6 | 0 | 0 | 0 | 7 | 2 | 0 | 26 defs, 5 bodies / 32 sigs failed, 59 Lean errors (crate run) |

### forge-bool (18828 lines; crate run: charon: out of memory (12 GB cap))

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `boolean.rs` | 6877 | H | C | 249 | 8 | 16 | 0 | 479 | 23 | 0 | tool: charon: out of memory (over the 8 GB cap) |
| `classify.rs` | 1116 | H | C | 51 | 3 | 3 | 0 | 40 | 0 | 0 | 136 fns, 6 bodies failed; lean: Funs: `«sorry /- ERROR: unknown identifier": trait_dec |
| `decide_once.rs` | 2344 | M | C | 37 | 0 | 4 | 75 | 148 | 10 | 0 | crate run: charon: out of memory (12 GB cap) |
| `decisions.rs` | 531 | H | C | 7 | 0 | 3 | 0 | 10 | 1 | 0 | tool: charon: ERROR Code failed to compile |
| `imprint.rs` | 2085 | H | C | 98 | 4 | 6 | 0 | 89 | 15 | 0 | 496 fns, 21 bodies failed; lean: Types: Sv/Types.lean:951:2: error(lean.unknownIdentifi |
| `lib.rs` | 32 | M | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 fns, 0 bodies failed; Lean ok |
| `nary.rs` | 3611 | H | C | 48 | 3 | 7 | 0 | 332 | 11 | 2 | tool: charon: out of memory (over the 8 GB cap) |
| `shape.rs` | 721 | M | C | 7 | 0 | 4 | 0 | 7 | 0 | 0 | 74 fns, 1 bodies failed; lean: Funs: unexpected token ':='; expected '/' (9x); unexpe |
| `ssi.rs` | 464 | H | C | 58 | 0 | 2 | 1 | 15 | 0 | 0 | 89 fns, 6 bodies failed; lean: FunsExternal: `core.option.Option.Insts.CoreOpsTry_tra |
| `unify.rs` | 1047 | H | C | 71 | 1 | 0 | 0 | 64 | 0 | 0 | 155 fns, 6 bodies failed; lean: FunsExternal: `core.option.Option.Insts.CoreOpsTry_tra |

### forge-bool-core (1338 lines; crate run: not run)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `index.rs` | 43 | M | B | 0 | 0 | 0 | 0 | 0 | 2 | 0 | crate run: not run |
| `lib.rs` | 1113 | M | C | 6 | 0 | 0 | 0 | 12 | 0 | 0 | crate run: not run |
| `sort.rs` | 141 | M | B | 0 | 0 | 0 | 0 | 0 | 1 | 0 | crate run: not run |
| `union_find.rs` | 41 | M | B | 0 | 0 | 0 | 0 | 2 | 0 | 0 | crate run: not run |

### forge-brake (4958 lines; crate run: not run)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `bb.rs` | 266 | M | B | 0 | 0 | 0 | 0 | 8 | 7 | 2 | crate run: not run |
| `geo.rs` | 769 | M | B | 0 | 0 | 0 | 0 | 44 | 3 | 2 | crate run: not run |
| `lib.rs` | 25 | M | C | 1 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: not run |
| `library.rs` | 370 | M | B | 0 | 0 | 0 | 0 | 5 | 0 | 0 | crate run: not run |
| `model.rs` | 416 | M | B | 0 | 0 | 0 | 0 | 19 | 0 | 4 | crate run: not run |
| `num.rs` | 124 | M | B | 0 | 0 | 0 | 0 | 0 | 0 | 1 | crate run: not run |
| `plan.rs` | 517 | M | C | 0 | 0 | 2 | 16 | 20 | 8 | 0 | crate run: not run |
| `sim.rs` | 1742 | M | C | 10 | 2 | 2 | 1 | 49 | 8 | 17 | crate run: not run |
| `tooling.rs` | 579 | M | C | 1 | 0 | 0 | 0 | 22 | 2 | 0 | crate run: not run |
| `view.rs` | 150 | M | B | 0 | 0 | 0 | 0 | 9 | 0 | 0 | crate run: not run |

### forge-dirs (951 lines; crate run: ok)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `fs.rs` | 837 | L | C | 0 | 0 | 6 | 0 | 13 | 0 | 0 | 131 fns, 14 bodies failed; lean: Funs: `take_while` is not a field of structure `core.i |
| `lib.rs` | 114 | L | C | 0 | 0 | 1 | 0 | 8 | 0 | 0 | 8 fns, 5 bodies failed; lean: Types: object file '/home/user/forgeCAD/verify/aeneas/ |

### forge-draw (5619 lines; crate run: aeneas: Fatal error: exception Failure("ty_regions shouldn't be called)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `annot.rs` | 510 | L | B | 0 | 0 | 0 | 0 | 13 | 1 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `dwg.rs` | 146 | L | C | 1 | 0 | 0 | 0 | 5 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `dxf.rs` | 443 | L | C | 1 | 0 | 0 | 0 | 7 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `fits.rs` | 160 | L | B | 0 | 0 | 0 | 0 | 13 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `gdt.rs` | 938 | L | B | 0 | 0 | 0 | 0 | 22 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `geom2.rs` | 344 | L | B | 0 | 0 | 0 | 0 | 19 | 2 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `grid.rs` | 278 | L | B | 0 | 0 | 0 | 0 | 10 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `hlr.rs` | 1837 | L | C | 32 | 3 | 9 | 10 | 111 | 6 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `lib.rs` | 22 | L | B | 0 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `page.rs` | 167 | L | B | 0 | 0 | 0 | 0 | 1 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `pdf.rs` | 191 | L | B | 0 | 0 | 0 | 0 | 8 | 1 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `section.rs` | 138 | L | C | 8 | 0 | 0 | 0 | 11 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `sheet.rs` | 287 | L | B | 0 | 0 | 0 | 0 | 6 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `svg.rs` | 80 | L | B | 0 | 0 | 0 | 0 | 7 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `view.rs` | 78 | L | B | 0 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |

### forge-dwg (29149 lines; crate run: charon: out of memory (12 GB cap))

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `bits.rs` | 417 | L | B | 0 | 0 | 0 | 0 | 4 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/boundary.rs` | 370 | L | C | 14 | 0 | 0 | 1 | 19 | 4 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/commit.rs` | 446 | L | C | 1 | 0 | 0 | 0 | 21 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/curve.rs` | 1715 | L | C | 196 | 6 | 1 | 0 | 32 | 3 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/determinism.rs` | 102 | L | C | 8 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/dim.rs` | 367 | L | C | 3 | 0 | 0 | 0 | 1 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/e2e.rs` | 315 | L | C | 8 | 0 | 0 | 0 | 5 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/entity.rs` | 853 | L | C | 35 | 0 | 0 | 0 | 42 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/fillet.rs` | 583 | L | C | 70 | 0 | 0 | 0 | 6 | 3 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/geom.rs` | 384 | L | B | 0 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/grip.rs` | 166 | L | C | 32 | 0 | 0 | 0 | 7 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/intersect.rs` | 808 | L | C | 30 | 1 | 0 | 0 | 14 | 1 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/make.rs` | 289 | L | C | 20 | 0 | 0 | 0 | 15 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/mod.rs` | 48 | L | C | 2 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/nurbs.rs` | 590 | L | C | 0 | 1 | 0 | 0 | 36 | 1 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/offset.rs` | 288 | L | C | 30 | 0 | 0 | 0 | 11 | 1 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/ops.rs` | 626 | L | C | 78 | 0 | 0 | 0 | 16 | 3 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/snap.rs` | 371 | L | C | 29 | 2 | 0 | 0 | 3 | 4 | 0 | crate run: charon: out of memory (12 GB cap) |
| `draft/xform.rs` | 116 | L | C | 20 | 0 | 0 | 0 | 8 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `edit.rs` | 612 | L | B | 0 | 0 | 0 | 1 | 20 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `file/mod.rs` | 236 | L | B | 0 | 0 | 0 | 0 | 5 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `file/r2000.rs` | 610 | L | B | 0 | 0 | 0 | 0 | 6 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `file/r2004.rs` | 340 | L | B | 0 | 0 | 0 | 0 | 8 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `file/r2007.rs` | 1426 | L | B | 0 | 0 | 0 | 0 | 55 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `fonts/glyph.rs` | 85 | L | B | 0 | 0 | 0 | 0 | 6 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `fonts/hershey.rs` | 755 | L | C | 116 | 0 | 1 | 0 | 10 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `fonts/mod.rs` | 545 | L | C | 7 | 0 | 8 | 4 | 17 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `fonts/shx.rs` | 709 | L | B | 0 | 0 | 0 | 3 | 17 | 0 | 2 | crate run: charon: out of memory (12 GB cap) |
| `fonts/tests.rs` | 255 | L | B | 0 | 0 | 0 | 0 | 9 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `fonts/text.rs` | 96 | L | B | 0 | 0 | 0 | 0 | 3 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `fonts/ttf.rs` | 304 | L | B | 0 | 0 | 0 | 0 | 13 | 0 | 2 | crate run: charon: out of memory (12 GB cap) |
| `header.rs` | 692 | L | B | 0 | 0 | 0 | 0 | 11 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `io.rs` | 600 | L | B | 0 | 0 | 0 | 0 | 2 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `lib.rs` | 174 | L | C | 1 | 0 | 0 | 0 | 0 | 1 | 0 | crate run: charon: out of memory (12 GB cap) |
| `object.rs` | 522 | L | B | 0 | 0 | 0 | 0 | 2 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `scene/build.rs` | 2627 | L | C | 11 | 6 | 4 | 2 | 109 | 1 | 0 | crate run: charon: out of memory (12 GB cap) |
| `scene/color.rs` | 86 | L | B | 0 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `scene/geom.rs` | 589 | L | B | 0 | 0 | 0 | 0 | 16 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `scene/hatch.rs` | 249 | L | B | 0 | 0 | 0 | 0 | 17 | 2 | 0 | crate run: charon: out of memory (12 GB cap) |
| `scene/ltype.rs` | 101 | L | B | 0 | 0 | 0 | 0 | 3 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `scene/mod.rs` | 196 | L | C | 0 | 4 | 0 | 0 | 0 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `scene/text.rs` | 736 | L | C | 0 | 3 | 0 | 0 | 23 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `str.rs` | 129 | L | B | 0 | 0 | 0 | 0 | 7 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `types/entities.rs` | 488 | L | C | 5 | 0 | 0 | 0 | 3 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `types/entities_a.rs` | 1260 | L | C | 5 | 0 | 0 | 0 | 2 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `types/entities_b.rs` | 1064 | L | B | 0 | 0 | 0 | 0 | 1 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `types/mod.rs` | 223 | L | C | 2 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `types/objects.rs` | 226 | L | B | 0 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `types/objects_b.rs` | 943 | L | B | 0 | 0 | 0 | 0 | 8 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `types/tables.rs` | 640 | L | B | 0 | 0 | 0 | 0 | 2 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `types/tables_b.rs` | 1485 | L | B | 0 | 0 | 0 | 0 | 5 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `version.rs` | 42 | L | B | 0 | 0 | 0 | 0 | 1 | 0 | 0 | crate run: charon: out of memory (12 GB cap) |
| `write.rs` | 1250 | L | C | 4 | 0 | 0 | 0 | 23 | 1 | 0 | crate run: charon: out of memory (12 GB cap) |

### forge-font (1857 lines; crate run: aeneas: Fatal error: exception Failure("ty_regions shouldn't be called)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `cff.rs` | 685 | L | B | 0 | 0 | 0 | 0 | 5 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `layout.rs` | 120 | L | B | 0 | 0 | 0 | 0 | 8 | 0 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `lib.rs` | 659 | L | B | 0 | 0 | 0 | 0 | 16 | 2 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |
| `testing.rs` | 393 | L | B | 0 | 0 | 0 | 0 | 22 | 2 | 0 | crate run: aeneas: Fatal error: exception Failure("ty_regions |

### forge-geom (27995 lines; crate run: aeneas: Fatal error: allocation failure during minor GC; last error: [)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `blend.rs` | 2025 | H | C | 148 | 3 | 12 | 0 | 36 | 1 | 0 | 986 fns, 20 bodies failed; lean: FunsExternal: `P.Insts.CoreOpsFunctionFnOncePairF64F64 |
| `budget.rs` | 55 | M | B | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 23 fns, 2 bodies failed; Lean ok |
| `corner.rs` | 3176 | H | C | 78 | 9 | 10 | 0 | 135 | 4 | 4 | 893 fns, 52 bodies failed; lean: no output |
| `corner/rational.rs` | 1415 | H | C | 10 | 0 | 4 | 4 | 43 | 0 | 0 | 340 fns, 14 bodies failed; lean: Types: Field `coremarkerCopyInst` has already been dec |
| `curve.rs` | 599 | H | C | 34 | 4 | 2 | 0 | 2 | 0 | 4 | crate run: aeneas: Fatal error: allocation failure during min |
| `intcurve.rs` | 1053 | H | C | 48 | 0 | 3 | 0 | 48 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `intersect.rs` | 1230 | H | C | 85 | 0 | 2 | 0 | 46 | 3 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `lib.rs` | 28 | M | C | 2 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `loftmeet.rs` | 567 | M | C | 18 | 6 | 0 | 0 | 23 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `meet.rs` | 1750 | H | C | 45 | 0 | 0 | 0 | 59 | 5 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `meet_gen.rs` | 42 | M | B | 0 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `meet_tests.rs` | 212 | M | C | 15 | 0 | 0 | 0 | 2 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `ruled.rs` | 781 | M | C | 13 | 1 | 1 | 0 | 38 | 1 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `ruled/meet.rs` | 338 | M | C | 15 | 0 | 0 | 0 | 10 | 2 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `screw.rs` | 1279 | M | C | 7 | 3 | 3 | 0 | 13 | 1 | 6 | crate run: aeneas: Fatal error: allocation failure during min |
| `screw/along.rs` | 607 | M | C | 19 | 3 | 0 | 0 | 5 | 4 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `screw/coaxial.rs` | 481 | M | C | 17 | 1 | 0 | 0 | 3 | 1 | 2 | crate run: aeneas: Fatal error: allocation failure during min |
| `screw/general.rs` | 347 | M | C | 9 | 0 | 0 | 0 | 15 | 5 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `screw/pair.rs` | 141 | M | C | 2 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `sheet.rs` | 101 | M | C | 14 | 0 | 0 | 0 | 7 | 1 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `skinned.rs` | 1718 | M | C | 8 | 3 | 9 | 0 | 132 | 1 | 2 | crate run: aeneas: Fatal error: allocation failure during min |
| `smooth.rs` | 529 | M | C | 24 | 0 | 1 | 0 | 17 | 1 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `spline.rs` | 1665 | H | C | 4 | 4 | 3 | 0 | 113 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `spline/extruded.rs` | 373 | H | C | 4 | 1 | 3 | 0 | 7 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `spline/fold.rs` | 200 | H | B | 0 | 0 | 0 | 0 | 27 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `spline/meet.rs` | 400 | H | C | 32 | 0 | 0 | 0 | 17 | 2 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `spline/spun.rs` | 334 | H | C | 4 | 1 | 2 | 0 | 9 | 0 | 2 | crate run: aeneas: Fatal error: allocation failure during min |
| `spline/swept.rs` | 697 | H | C | 5 | 3 | 7 | 0 | 11 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `spline/tests.rs` | 816 | H | C | 8 | 1 | 0 | 0 | 39 | 0 | 8 | crate run: aeneas: Fatal error: allocation failure during min |
| `surface.rs` | 1031 | H | C | 77 | 4 | 1 | 0 | 7 | 0 | 12 | crate run: aeneas: Fatal error: allocation failure during min |
| `testing.rs` | 79 | M | C | 22 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `tube.rs` | 237 | M | C | 8 | 1 | 0 | 0 | 3 | 0 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `varblend.rs` | 3052 | H | C | 43 | 1 | 15 | 0 | 116 | 13 | 0 | crate run: aeneas: Fatal error: allocation failure during min |
| `wrap.rs` | 637 | M | C | 6 | 1 | 0 | 0 | 16 | 2 | 4 | crate run: aeneas: Fatal error: allocation failure during min |

### forge-git (4106 lines; crate run: ok)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `config.rs` | 245 | L | B | 0 | 0 | 0 | 0 | 8 | 0 | 0 | 19 defs, 3 bodies / 0 sigs failed, 0 Lean errors (crate run) |
| `diff.rs` | 110 | L | B | 0 | 0 | 0 | 0 | 0 | 9 | 0 | 13 defs, 1 bodies / 0 sigs failed, 0 Lean errors (crate run) |
| `fsutil.rs` | 100 | L | B | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 13 defs, 0 bodies / 0 sigs failed, 1 Lean errors (crate run) |
| `hash.rs` | 341 | L | B | 0 | 0 | 0 | 0 | 5 | 0 | 0 | 106 defs, 1 bodies / 0 sigs failed, 0 Lean errors (crate run) |
| `ignore.rs` | 305 | L | B | 0 | 0 | 0 | 3 | 8 | 0 | 0 | 55 defs, 4 bodies / 0 sigs failed, 0 Lean errors (crate run) |
| `index.rs` | 315 | L | B | 0 | 0 | 0 | 0 | 7 | 0 | 0 | 48 defs, 1 bodies / 0 sigs failed, 1 Lean errors (crate run) |
| `lib.rs` | 85 | L | B | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 11 defs, 2 bodies / 1 sigs failed, 0 Lean errors (crate run) |
| `object.rs` | 363 | L | B | 0 | 0 | 0 | 0 | 13 | 4 | 0 | 113 defs, 9 bodies / 2 sigs failed, 3 Lean errors (crate run) |
| `odb.rs` | 294 | L | C | 0 | 0 | 9 | 4 | 14 | 0 | 0 | 59 defs, 1 bodies / 0 sigs failed, 2 Lean errors (crate run) |
| `pack.rs` | 299 | L | C | 0 | 0 | 1 | 0 | 8 | 4 | 0 | 38 defs, 0 bodies / 0 sigs failed, 2 Lean errors (crate run) |
| `refs.rs` | 245 | L | B | 0 | 0 | 0 | 0 | 9 | 0 | 0 | 45 defs, 1 bodies / 0 sigs failed, 3 Lean errors (crate run) |
| `repo.rs` | 378 | L | B | 0 | 0 | 0 | 0 | 10 | 0 | 0 | 35 defs, 1 bodies / 0 sigs failed, 6 Lean errors (crate run) |
| `tree.rs` | 110 | L | B | 0 | 0 | 0 | 0 | 6 | 0 | 0 | 25 defs, 1 bodies / 0 sigs failed, 1 Lean errors (crate run) |
| `walk.rs` | 123 | L | B | 0 | 0 | 0 | 3 | 4 | 0 | 0 | 95 defs, 1 bodies / 0 sigs failed, 81 Lean errors (crate run) |
| `zlib.rs` | 793 | L | B | 0 | 0 | 0 | 0 | 26 | 0 | 0 | 123 defs, 6 bodies / 0 sigs failed, 0 Lean errors (crate run) |

### forge-gpu (856 lines; crate run: aeneas: Fatal error: exception Not_found; last error: [Error] Unsuppor)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `lib.rs` | 856 | L | C | 1 | 1 | 4 | 0 | 19 | 3 | 0 | crate run: aeneas: Fatal error: exception Not_found; last err |

### forge-harness (2046 lines; crate run: ok)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `lib.rs` | 693 | L | B | 0 | 0 | 0 | 0 | 47 | 0 | 0 | 127 defs, 3 bodies / 117 sigs failed, 34 Lean errors (crate run) |
| `tube.rs` | 44 | L | C | 3 | 0 | 0 | 0 | 2 | 0 | 0 | 3 defs, 1 bodies / 5 sigs failed, 0 Lean errors (crate run) |
| `wires.rs` | 1309 | L | C | 0 | 1 | 0 | 0 | 136 | 10 | 0 | 234 defs, 17 bodies / 246 sigs failed, 54 Lean errors (crate run) |

### forge-material (697 lines; crate run: ok)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `lib.rs` | 697 | L | C | 0 | 0 | 5 | 0 | 53 | 0 | 0 | tool: Ignoring the body of 'forge_material::save' beca |

### forge-math (3422 lines; crate run: ok)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `clock.rs` | 82 | M | A | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 1 fns, 0 bodies failed; Lean ok |
| `dmath.rs` | 508 | H | A | 0 | 0 | 0 | 0 | 1 | 0 | 14 | 70 fns, 0 bodies failed; Lean ok |
| `exact.rs` | 661 | M | B | 0 | 0 | 0 | 0 | 10 | 17 | 0 | crate run: ok |
| `frame.rs` | 143 | H | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 86 fns, 0 bodies failed; Lean ok |
| `hash.rs` | 111 | M | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 22 fns, 0 bodies failed; Lean ok |
| `interval.rs` | 295 | H | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 84 fns, 0 bodies failed; Lean ok |
| `lib.rs` | 26 | M | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 fns, 0 bodies failed; Lean ok |
| `linalg.rs` | 91 | M | B | 0 | 0 | 0 | 0 | 17 | 0 | 0 | 15 fns, 1 bodies failed; lean: Funs: Type mismatch (3x); `«sorry /- ERROR: unknown id |
| `par.rs` | 57 | M | B | 0 | 0 | 0 | 0 | 8 | 1 | 0 | 16 fns, 2 bodies failed; lean: Types: Field `markerCopyInst` has already been declare |
| `poly.rs` | 285 | M | B | 0 | 0 | 0 | 0 | 18 | 1 | 0 | 69 fns, 0 bodies failed; lean: Funs: stuck at solving universe constraint (8x); `skip |
| `predicates.rs` | 814 | H | B | 0 | 0 | 0 | 0 | 14 | 6 | 0 | 67 fns, 3 bodies failed; lean: Funs: expected a product type, got (4x); `zip` is not  |
| `rng.rs` | 30 | M | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 8 fns, 0 bodies failed; Lean ok |
| `tol.rs` | 32 | M | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 11 fns, 0 bodies failed; Lean ok |
| `trace.rs` | 24 | M | A | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 5 fns, 0 bodies failed; Lean ok |
| `vec.rs` | 263 | H | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 93 fns, 0 bodies failed; Lean ok |

### forge-mesh (4848 lines; crate run: ok)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `cdt.rs` | 1114 | H | C | 0 | 3 | 0 | 0 | 47 | 5 | 0 | 128 fns, 7 bodies failed; lean: Funs: `«sorry /- ERROR: unknown identifier": trait_dec |
| `lib.rs` | 20 | M | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 fns, 0 bodies failed; Lean ok |
| `mesh.rs` | 128 | M | B | 0 | 0 | 0 | 0 | 9 | 0 | 0 | 22 fns, 0 bodies failed; lean: Funs: `«sorry /- ERROR: unknown identifier": trait_dec |
| `obj.rs` | 186 | M | B | 0 | 0 | 0 | 0 | 8 | 0 | 0 | crate run: ok |
| `scene.rs` | 392 | M | C | 2 | 0 | 0 | 0 | 28 | 0 | 0 | crate run: ok |
| `stl.rs` | 191 | M | B | 0 | 0 | 0 | 0 | 8 | 0 | 0 | 7 fns, 1 bodies failed; lean: Funs: `zip` is not a field of structure `core.iter.tra |
| `tessellate.rs` | 2121 | H | C | 79 | 1 | 1 | 2 | 132 | 1 | 0 | 436 fns, 14 bodies failed; lean: Funs: `zip` is not a field of structure `core.iter.tra |
| `threemf.rs` | 499 | M | B | 0 | 0 | 0 | 0 | 33 | 0 | 0 | crate run: ok |
| `zip.rs` | 197 | M | B | 0 | 0 | 0 | 0 | 4 | 0 | 0 | crate run: ok |

### forge-model (18772 lines; crate run: ok)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `adversarial.rs` | 1685 | M | C | 108 | 0 | 0 | 0 | 24 | 0 | 0 | 167 defs, 5 bodies / 677 sigs failed, 0 Lean errors (crate run) |
| `bodies.rs` | 1229 | M | C | 145 | 0 | 0 | 4 | 104 | 0 | 0 | 189 defs, 3 bodies / 277 sigs failed, 0 Lean errors (crate run) |
| `edit.rs` | 166 | M | C | 5 | 0 | 0 | 0 | 2 | 0 | 0 | 30 defs, 0 bodies / 26 sigs failed, 0 Lean errors (crate run) |
| `edits.rs` | 515 | M | C | 26 | 0 | 0 | 0 | 12 | 0 | 0 | 24 defs, 1 bodies / 36 sigs failed, 0 Lean errors (crate run) |
| `extent.rs` | 642 | M | C | 49 | 0 | 1 | 0 | 19 | 0 | 0 | 53 defs, 7 bodies / 63 sigs failed, 0 Lean errors (crate run) |
| `fillets.rs` | 1451 | M | C | 87 | 0 | 0 | 0 | 61 | 2 | 0 | 48 defs, 0 bodies / 112 sigs failed, 0 Lean errors (crate run) |
| `finishing.rs` | 419 | M | C | 51 | 0 | 0 | 0 | 25 | 0 | 0 | 35 defs, 0 bodies / 34 sigs failed, 0 Lean errors (crate run) |
| `hole.rs` | 512 | M | C | 39 | 0 | 0 | 0 | 29 | 0 | 0 | 52 defs, 5 bodies / 76 sigs failed, 0 Lean errors (crate run) |
| `import.rs` | 45 | M | C | 4 | 0 | 1 | 0 | 0 | 0 | 0 | 11 defs, 0 bodies / 10 sigs failed, 0 Lean errors (crate run) |
| `lib.rs` | 2350 | M | C | 229 | 5 | 2 | 0 | 113 | 3 | 0 | 180 defs, 12 bodies / 382 sigs failed, 1 Lean errors (crate run) |
| `names.rs` | 1254 | M | C | 148 | 2 | 1 | 2 | 102 | 0 | 0 | 166 defs, 3 bodies / 242 sigs failed, 0 Lean errors (crate run) |
| `params.rs` | 139 | M | C | 2 | 5 | 1 | 0 | 4 | 0 | 0 | 27 defs, 5 bodies / 33 sigs failed, 2 Lean errors (crate run) |
| `path.rs` | 243 | M | C | 7 | 0 | 0 | 0 | 8 | 0 | 0 | 15 defs, 0 bodies / 28 sigs failed, 0 Lean errors (crate run) |
| `pattern.rs` | 523 | M | C | 37 | 0 | 0 | 0 | 20 | 0 | 0 | 58 defs, 5 bodies / 75 sigs failed, 0 Lean errors (crate run) |
| `regions.rs` | 100 | M | C | 7 | 0 | 0 | 0 | 8 | 0 | 0 | crate run: ok |
| `rib.rs` | 300 | M | C | 7 | 0 | 0 | 0 | 19 | 0 | 0 | 44 defs, 0 bodies / 53 sigs failed, 0 Lean errors (crate run) |
| `splinefam.rs` | 565 | M | C | 22 | 0 | 0 | 0 | 6 | 0 | 0 | 33 defs, 1 bodies / 46 sigs failed, 0 Lean errors (crate run) |
| `splitface.rs` | 589 | M | C | 23 | 0 | 0 | 15 | 54 | 4 | 0 | 47 defs, 7 bodies / 88 sigs failed, 0 Lean errors (crate run) |
| `standard.rs` | 302 | M | B | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 20 defs, 0 bodies / 28 sigs failed, 0 Lean errors (crate run) |
| `suite.rs` | 1549 | M | C | 94 | 0 | 0 | 0 | 27 | 0 | 0 | 53 defs, 5 bodies / 88 sigs failed, 0 Lean errors (crate run) |
| `swept.rs` | 647 | M | C | 22 | 0 | 4 | 0 | 33 | 0 | 0 | 74 defs, 15 bodies / 141 sigs failed, 0 Lean errors (crate run) |
| `text.rs` | 257 | M | C | 2 | 0 | 1 | 0 | 25 | 1 | 0 | 41 defs, 2 bodies / 59 sigs failed, 2 Lean errors (crate run) |
| `thread.rs` | 654 | M | C | 19 | 1 | 0 | 0 | 16 | 0 | 0 | 60 defs, 0 bodies / 84 sigs failed, 0 Lean errors (crate run) |
| `torture.rs` | 110 | M | C | 12 | 0 | 0 | 0 | 0 | 0 | 0 | 1 defs, 0 bodies / 5 sigs failed, 0 Lean errors (crate run) |
| `trace.rs` | 117 | M | C | 7 | 0 | 1 | 0 | 11 | 0 | 0 | 7 defs, 1 bodies / 20 sigs failed, 0 Lean errors (crate run) |
| `upto.rs` | 1569 | M | C | 89 | 0 | 0 | 0 | 94 | 2 | 4 | 163 defs, 7 bodies / 231 sigs failed, 0 Lean errors (crate run) |
| `verify.rs` | 103 | M | C | 7 | 0 | 0 | 0 | 9 | 2 | 0 | 21 defs, 1 bodies / 25 sigs failed, 0 Lean errors (crate run) |
| `weld.rs` | 211 | M | C | 16 | 0 | 0 | 0 | 5 | 0 | 0 | crate run: ok |
| `work.rs` | 526 | M | C | 58 | 0 | 0 | 0 | 22 | 0 | 0 | 64 defs, 5 bodies / 92 sigs failed, 85 Lean errors (crate run) |

### forge-ops (19899 lines; crate run: charon: out of memory (12 GB cap))

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `chamfer.rs` | 1072 | M | C | 54 | 0 | 0 | 19 | 53 | 2 | 0 | tool: Fatal error: allocation failure during minor GC |
| `corner.rs` | 490 | M | C | 9 | 1 | 0 | 0 | 34 | 0 | 0 | 69 fns, 0 bodies failed; lean: Types: Sv/Types.lean:366:2: error(lean.unknownIdentifi |
| `direct.rs` | 835 | M | C | 49 | 0 | 0 | 11 | 110 | 0 | 0 | tool: charon: out of memory (over the 8 GB cap) |
| `draft.rs` | 209 | M | C | 20 | 0 | 0 | 0 | 8 | 0 | 0 | 136 fns, 2 bodies failed; lean: Funs: `zip` is not a field of structure `core.iter.tra |
| `extrude.rs` | 365 | M | C | 32 | 0 | 0 | 0 | 18 | 0 | 0 | 116 fns, 8 bodies failed; lean: Funs: unexpected token 'sorry'; expected '}' (21x); `s |
| `fillet.rs` | 7898 | H | C | 198 | 2 | 4 | 0 | 532 | 25 | 0 | tool: charon: out of memory (over the 8 GB cap) |
| `fillet_cliff.rs` | 780 | M | C | 36 | 0 | 0 | 0 | 77 | 1 | 0 | tool: charon: ERROR Code failed to compile |
| `glyph.rs` | 343 | M | C | 18 | 0 | 0 | 0 | 25 | 0 | 0 | 42 fns, 2 bodies failed; lean: Funs: unexpected token 'sorry'; expected '}' (8x); une |
| `lib.rs` | 44 | M | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 fns, 0 bodies failed; Lean ok |
| `loft.rs` | 816 | M | C | 16 | 0 | 1 | 0 | 89 | 3 | 0 | tool: Fatal error: allocation failure during minor GC |
| `profile.rs` | 923 | M | C | 10 | 0 | 1 | 0 | 44 | 0 | 0 | 110 fns, 7 bodies failed; lean: Funs: `zip` is not a field of structure `core.iter.tra |
| `rebuild.rs` | 618 | M | C | 78 | 0 | 0 | 0 | 42 | 1 | 0 | 81 fns, 1 bodies failed; lean: Funs: `zip` is not a field of structure `core.iter.tra |
| `revolve.rs` | 631 | M | C | 28 | 0 | 0 | 0 | 21 | 1 | 0 | 133 fns, 9 bodies failed; lean: Funs: unexpected token 'sorry'; expected '}' (16x); `s |
| `roles.rs` | 59 | M | A | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 9 fns, 0 bodies failed; Lean ok |
| `selfx.rs` | 222 | M | C | 15 | 0 | 3 | 0 | 19 | 0 | 0 | 38 fns, 0 bodies failed; lean: Funs: unexpected token 'sorry'; expected '}' (14x); Fu |
| `shell.rs` | 947 | M | C | 67 | 1 | 0 | 6 | 73 | 0 | 0 | 345 fns, 7 bodies failed; lean: Funs: `zip` is not a field of structure `core.iter.tra |
| `skin.rs` | 321 | M | C | 12 | 0 | 0 | 0 | 28 | 0 | 0 | 126 fns, 11 bodies failed; lean: Funs: unexpected token 'sorry'; expected '}' (32x); `s |
| `strip.rs` | 935 | M | C | 40 | 0 | 0 | 0 | 88 | 1 | 0 | tool: charon: out of memory (over the 8 GB cap) |
| `sweep.rs` | 779 | M | C | 18 | 0 | 4 | 0 | 61 | 0 | 1 | 272 fns, 15 bodies failed; lean: FunsExternal: `P.Insts.CoreOpsFunctionFnOncePairF64F64 |
| `taper.rs` | 937 | M | C | 22 | 0 | 0 | 0 | 66 | 0 | 0 | 315 fns, 12 bodies failed; lean: Funs: `zip` is not a field of structure `core.iter.tra |
| `thicken.rs` | 295 | M | C | 27 | 0 | 0 | 0 | 6 | 0 | 0 | 136 fns, 2 bodies failed; lean: Funs: `zip` is not a field of structure `core.iter.tra |
| `wavefront.rs` | 380 | M | C | 22 | 2 | 0 | 0 | 56 | 0 | 0 | 164 fns, 5 bodies failed; lean: Funs: `zip` is not a field of structure `core.iter.tra |

### forge-par (359 lines; crate run: not run)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `lib.rs` | 223 | M | C | 0 | 3 | 7 | 0 | 0 | 4 | 0 | crate run: not run |
| `tests.rs` | 136 | M | C | 0 | 0 | 1 | 0 | 16 | 6 | 0 | crate run: not run |

### forge-sheet (13599 lines; crate run: aeneas: Fatal error: exception Failure("ty_regions shouldn't be called)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `bendline.rs` | 486 | H | D | 0 | 1 | 0 | 0 | 26 | 0 | 0 | tool: Fatal error: exception Not_found |
| `build.rs` | 4391 | M | C | 34 | 15 | 2 | 11 | 269 | 9 | 9 | 891 fns, 46 bodies failed; lean: FunsExternal: `P.Insts.CoreOpsFunctionFnOncePairF64F64 |
| `checks.rs` | 1281 | M | B | 0 | 0 | 0 | 0 | 99 | 5 | 0 | 1044 fns, 78 bodies failed; lean: FunsExternal: `core.option.Option.Insts.CoreOpsTry_tra |
| `contour.rs` | 498 | M | B | 0 | 0 | 0 | 0 | 9 | 0 | 0 | 105 fns, 5 bodies failed; lean: FunsExternal: `core.option.Option.Insts.CoreOpsTry_tra |
| `corner.rs` | 391 | M | B | 0 | 0 | 0 | 0 | 18 | 2 | 0 | 122 fns, 2 bodies failed; lean: FunsExternal: `core.option.Option.Insts.CoreOpsTry_tra |
| `error.rs` | 900 | M | B | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 129 fns, 5 bodies failed; Lean ok |
| `form.rs` | 1001 | H | C | 5 | 0 | 0 | 0 | 38 | 0 | 4 | 229 fns, 15 bodies failed; lean: FunsExternal: `core.option.Option.Insts.CoreOpsTry_tra |
| `graph.rs` | 920 | M | B | 0 | 0 | 0 | 0 | 44 | 0 | 0 | 227 fns, 5 bodies failed; lean: FunsExternal: `core.option.Option.Insts.CoreOpsTry_tra |
| `hardware.rs` | 98 | M | B | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 28 fns, 0 bodies failed; lean: Funs: `find` is not a field of structure `core.iter.tr |
| `hem.rs` | 378 | M | B | 0 | 0 | 0 | 0 | 11 | 0 | 0 | 131 fns, 4 bodies failed; lean: FunsExternal: `core.option.Option.Insts.CoreOpsTry_tra |
| `lib.rs` | 50 | M | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 fns, 0 bodies failed; Lean ok |
| `pattern.rs` | 862 | M | C | 1 | 0 | 0 | 0 | 22 | 0 | 0 | tool: Joining region abstractions containing shared lo |
| `punch.rs` | 487 | M | B | 0 | 0 | 0 | 0 | 25 | 0 | 0 | 128 fns, 7 bodies failed; lean: FunsExternal: `core.option.Option.Insts.CoreOpsTry_tra |
| `region.rs` | 1231 | H | C | 0 | 0 | 1 | 3 | 57 | 5 | 0 | 115 fns, 7 bodies failed; lean: FunsExternal: `core.option.Option.Insts.CoreOpsTry_tra |
| `relief.rs` | 216 | M | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 50 fns, 0 bodies failed; Lean ok |
| `roles.rs` | 46 | M | B | 0 | 0 | 0 | 0 | 9 | 0 | 0 | 39 fns, 0 bodies failed; lean: Funs: `map` is not a field of structure `core.iter.tra |
| `shop.rs` | 363 | M | B | 0 | 0 | 0 | 0 | 17 | 3 | 0 | 112 fns, 0 bodies failed; lean: Funs: `min_by` is not a field of structure `core.iter. |

### forge-sheet-core (278 lines; crate run: not run)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `lib.rs` | 187 | M | B | 0 | 0 | 0 | 0 | 1 | 1 | 0 | crate run: not run |
| `place.rs` | 50 | M | B | 0 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: not run |
| `union_find.rs` | 41 | M | B | 0 | 0 | 0 | 0 | 2 | 0 | 0 | crate run: not run |

### forge-sketch (4153 lines; crate run: ok)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `dual.rs` | 139 | M | B | 0 | 0 | 0 | 0 | 8 | 0 | 0 | 30 fns, 0 bodies failed; lean: Funs: expected a product type, got (6x); `zip` is not  |
| `lib.rs` | 19 | M | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 fns, 0 bodies failed; Lean ok |
| `profile.rs` | 534 | M | C | 7 | 1 | 1 | 0 | 40 | 0 | 0 | 112 fns, 4 bodies failed; lean: Funs: Type mismatch (9x); Application type mismatch: T |
| `project.rs` | 301 | M | C | 12 | 1 | 0 | 0 | 8 | 1 | 0 | 36 fns, 3 bodies failed; lean: Types: Sv/Types.lean:244:2: error(lean.unknownIdentifi |
| `sketch.rs` | 1263 | M | C | 0 | 0 | 1 | 0 | 48 | 0 | 0 | 188 fns, 6 bodies failed; lean: Funs: Type mismatch (25x); Application type mismatch:  |
| `solve.rs` | 960 | M | B | 0 | 0 | 0 | 2 | 108 | 1 | 0 | 245 fns, 12 bodies failed; lean: Funs: `zip` is not a field of structure `core.iter.tra |
| `tests.rs` | 534 | M | D | 0 | 0 | 0 | 0 | 15 | 0 | 0 | tool: charon: ERROR Code failed to compile |
| `winding.rs` | 403 | M | B | 0 | 0 | 0 | 0 | 1 | 0 | 0 | crate run: ok |

### forge-step (8259 lines; crate run: aeneas: Fatal error: exception Not_found; last error: [Error] Internal)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `brep.rs` | 1684 | M | C | 41 | 0 | 0 | 0 | 63 | 0 | 0 | tool: Internal error: please file an issue |
| `fit.rs` | 696 | M | C | 12 | 0 | 0 | 7 | 28 | 1 | 0 | 83 fns, 4 bodies failed; lean: Funs: unexpected token 'sorry'; expected '}' (12x); Ty |
| `heal.rs` | 1411 | M | C | 65 | 0 | 0 | 3 | 64 | 1 | 0 | 130 fns, 0 bodies failed; lean: FunsExternal: `core.option.Option.Insts.CoreOpsTry_tra |
| `iges.rs` | 676 | M | C | 15 | 0 | 0 | 0 | 16 | 0 | 0 | 346 fns, 17 bodies failed; lean: TypesExternal: Function expected at (1x);  |
| `lib.rs` | 17 | M | A | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 fns, 0 bodies failed; Lean ok |
| `nurbs.rs` | 658 | M | C | 3 | 0 | 7 | 0 | 32 | 0 | 0 | 102 fns, 14 bodies failed; lean: Funs: `«sorry /- ERROR: unknown identifier": trait_dec |
| `part21.rs` | 598 | M | B | 0 | 0 | 0 | 0 | 1 | 5 | 0 | 64 fns, 10 bodies failed; lean: TypesExternal: Function expected at (1x);  |
| `read.rs` | 1567 | M | C | 45 | 0 | 0 | 0 | 41 | 0 | 0 | tool: Internal error: please file an issue |
| `write.rs` | 952 | M | C | 29 | 0 | 0 | 0 | 62 | 2 | 0 | crate run: aeneas: Fatal error: exception Not_found; last err |

### forge-thermo (9015 lines; crate run: ok)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `bench.rs` | 1009 | L | C | 20 | 2 | 0 | 0 | 46 | 0 | 0 | 0 defs, 0 bodies / 0 sigs failed, 0 Lean errors (crate run) |
| `build.rs` | 512 | L | C | 11 | 0 | 0 | 0 | 26 | 0 | 0 | 55 defs, 3 bodies / 80 sigs failed, 1 Lean errors (crate run) |
| `checks.rs` | 360 | L | C | 9 | 0 | 0 | 0 | 7 | 1 | 0 | 19 defs, 1 bodies / 22 sigs failed, 0 Lean errors (crate run) |
| `field.rs` | 410 | L | B | 0 | 0 | 0 | 0 | 28 | 3 | 0 | 53 defs, 6 bodies / 60 sigs failed, 0 Lean errors (crate run) |
| `fit.rs` | 199 | L | B | 0 | 1 | 0 | 0 | 31 | 4 | 0 | 55 defs, 3 bodies / 58 sigs failed, 0 Lean errors (crate run) |
| `lib.rs` | 37 | L | C | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 0 defs, 0 bodies / 0 sigs failed, 0 Lean errors (crate run) |
| `material.rs` | 334 | L | C | 2 | 0 | 0 | 0 | 16 | 1 | 0 | 53 defs, 4 bodies / 69 sigs failed, 0 Lean errors (crate run) |
| `newton.rs` | 843 | L | B | 0 | 1 | 0 | 0 | 44 | 4 | 0 | 74 defs, 7 bodies / 87 sigs failed, 0 Lean errors (crate run) |
| `prepare.rs` | 282 | L | C | 15 | 0 | 0 | 0 | 33 | 3 | 0 | 46 defs, 1 bodies / 74 sigs failed, 0 Lean errors (crate run) |
| `scale.rs` | 262 | L | C | 45 | 0 | 0 | 0 | 3 | 0 | 0 | 10 defs, 1 bodies / 34 sigs failed, 0 Lean errors (crate run) |
| `sdf.rs` | 399 | L | C | 6 | 0 | 0 | 12 | 19 | 0 | 0 | 50 defs, 0 bodies / 54 sigs failed, 0 Lean errors (crate run) |
| `simulated.rs` | 1212 | L | C | 5 | 1 | 0 | 20 | 99 | 3 | 0 | 187 defs, 14 bodies / 256 sigs failed, 0 Lean errors (crate run) |
| `stretch.rs` | 2262 | L | C | 10 | 1 | 5 | 3 | 153 | 6 | 0 | 263 defs, 43 bodies / 311 sigs failed, 0 Lean errors (crate run) |
| `surface.rs` | 144 | L | C | 0 | 1 | 0 | 0 | 8 | 0 | 0 | 32 defs, 3 bodies / 30 sigs failed, 66 Lean errors (crate run) |
| `tests.rs` | 750 | L | C | 5 | 0 | 0 | 1 | 26 | 1 | 0 | 0 defs, 0 bodies / 0 sigs failed, 0 Lean errors (crate run) |

### forge-topo (6396 lines; crate run: charon: out of memory (12 GB cap))

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `body.rs` | 1401 | H | C | 59 | 1 | 13 | 0 | 94 | 1 | 0 | 459 fns, 8 bodies failed; lean: Funs: `«sorry /- ERROR: unknown identifier": trait_dec |
| `check.rs` | 1467 | H | C | 58 | 1 | 3 | 0 | 125 | 2 | 0 | 542 fns, 14 bodies failed; lean: FunsExternal: `P.Insts.CoreOpsFunctionFnOncePairF64F64 |
| `lib.rs` | 19 | M | A | 5 | 0 | 0 | 0 | 0 | 0 | 0 | 0 fns, 0 bodies failed; Lean ok |
| `mass.rs` | 2791 | H | C | 85 | 1 | 18 | 0 | 210 | 2 | 2 | tool: charon: out of memory (over the 8 GB cap) |
| `name.rs` | 86 | M | A | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 23 fns, 0 bodies failed; Lean ok |
| `small.rs` | 95 | M | A | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 10 fns, 0 bodies failed; Lean ok |
| `uv.rs` | 537 | M | C | 29 | 2 | 0 | 0 | 44 | 2 | 0 | 121 fns, 5 bodies failed; lean: Funs: `«sorry /- ERROR: unknown identifier": trait_dec |

### forge-vendor (2030 lines; crate run: not run)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `lib.rs` | 1373 | M | C | 0 | 0 | 5 | 0 | 87 | 1 | 0 | crate run: not run |
| `tests.rs` | 246 | M | B | 0 | 0 | 0 | 0 | 9 | 0 | 0 | crate run: not run |
| `toml.rs` | 411 | M | B | 0 | 0 | 0 | 0 | 14 | 0 | 0 | crate run: not run |

### forge-weld (1842 lines; crate run: not run)

| module | lines | value | class | shared types | dyn | Rc/Arc/Cell | HashMap | iter adaptors | Ordering | libm | what Aeneas + Lean said |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `frame.rs` | 885 | M | C | 0 | 1 | 0 | 0 | 48 | 4 | 0 | crate run: not run |
| `lib.rs` | 12 | M | B | 0 | 0 | 0 | 0 | 0 | 0 | 0 | crate run: not run |
| `library.rs` | 295 | M | B | 0 | 0 | 0 | 0 | 20 | 0 | 0 | crate run: not run |
| `section.rs` | 650 | M | C | 2 | 1 | 0 | 0 | 19 | 0 | 0 | crate run: not run |
