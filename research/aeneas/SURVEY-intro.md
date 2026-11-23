# What of the kernel Aeneas can take, and what it takes to get the rest

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
