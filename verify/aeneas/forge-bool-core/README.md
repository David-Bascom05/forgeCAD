# kernel/forge-bool-core in Lean, by Aeneas, and its refinement proof (started)

The decide-once core (`kernel/forge-bool-core`) is translated by Charon and Aeneas, and the
translation is proven equal, function by function, to the Lean core it implements,
`ForgeExact.DecideOnce.core` (verify/forge-exact/ForgeExact/DecideOnce/Core.lean). That is the
obligation in decide-once-spec.md §4.

- Translation: `tools/aeneas/translate.sh forge-bool-core` (the toolchain: `tools/aeneas/setup.sh`),
  configured by `translate.conf`: the whole crate, release with overflow checks, and `BRIDGE=none`
  (no f64, so no float bridge). It writes `lean/ForgeBoolCore/{Types,Funs,FunsExternal_Template}.lean`.
  CI (`.github/workflows/aeneas.yml`) re-translates and fails if the committed files differ, then
  builds the Lean.
- `lean/`: a lake project. It requires the Aeneas Lean library at the pinned commit, and
  verify/forge-exact by path. `lake exe cache get && lake build` checks the translation, the
  refinement lemmas and the axiom report.

## What translates

All of it. That is 97 functions and 310 Lean definitions, with no error, no partial file and
nothing left opaque that the core itself defines. Getting there took a style pass on the Rust:
- No struct holds a borrow. The cells the core adds are an owned `Ext`, passed beside `&Ledger`.
  ("joining region abstractions containing shared loans whose shared value contains borrows" was
  the error.)
- No `break`, and no `return` inside a loop. Loops exit through their guard, using a flag.
- No `Ordering` (it gave an internal error): `cmp_cov` returns -1, 0 or 1.
- No local variable named `uf`, because it shadowed the generated namespace `uf`. The module is
  now `union_find`.

`FunsExternal.lean` defines the seven std functions Aeneas leaves opaque, none as an axiom:
`Bool::ne`, the tuple `eq` and `ne` (std's bodies) and `Vec::default` (the empty vector) are
`def`s; the two `Hash` impls and the tuple `Debug` are Lean `opaque`s. The core's proven path uses
none of them.
`sort_triples` now compares field by field (`le3`), so the tuple `PartialOrd` axiom is gone.

## What is proven

Every lemma says the Rust returns `ok`, so it does not panic, overflow or diverge, and returns the
Lean core's value. Proofs use only the standard axioms (`lean/ForgeBoolCore/Axioms.lean`) and contain
no `sorry`. The obligations are stated in `lean/ForgeBoolCore/Specs.lean`; how they are split across
sessions is in `lean/SPLIT.md`.

- **Every stage**, `unify` and `collapse` included:
  - the index, `cov`, `find` and `intern`. `find` looks up through the hashed index `Index::find_h`
    (`RefineHash.lean`). The hash need not be injective: `find_h` binary-searches the run of entries
    with the key's hash, then compares covectors (`same`);
  - `split` and `split_ok` (group S);
  - `membership`, the classify check, and the keep table;
  - `around`, `between`, `edges_between` and `bad_vert`;
  - `core_k`'s loops and the four position-to-covector bridges (group R);
  - `collapse` (group K) and `unify` (group U);
  - `new_ext`, `core_k`, `keep_on`, `classify_on` and `core` (`RefineCore.lean`).
- **End to end** (`EndToEnd.lean`, `Integrate.lean`):
  - **`end_to_end_proof : EndToEnd`**, with no hypotheses. On a Rust ledger that reads as
    the sampler's ledger of a flat input, `forge_bool_core::core` answers the exact regularised
    boolean: the body is `Valid`, its faces are exactly the boundary facets, its kept cells give
    `reg` of the boolean, and `ZeroThickness` ↔ `XNonManifold`.
  - `end_to_end_encode_proof`: the same for the Rust ledger `encode` builds from any sampler ledger
    within `LFits`.
- **Size limits** (`LedgerWF`, `LFits`):
  - fewer than `u32::MAX` cells (the ledger's plus two new sides per piece), edges and verts;
  - ids below `u32::MAX`;
  - `3·n ≤ usize::MAX` for the sorts;
  - distinct edge cells;
  - `nb > 0`;
  - at most 12 zeros (`ENUM_ZEROS`) in the enumerating branches.

## Open obligations

None for the core itself. What remains is the frontier optimisation:

1. **The frontier (local ledger, decide-once-spec §3a).** Proven in verify/forge-exact:
   - F1: `far_unchanged`;
   - F2: `frontier_local`;
   - the global split: `split_at_region` (`DecideOnce/Frontier.lean`);
   - (c′) in the open region for any ledger satisfying `ExactIn` (`DecideOnce/Local.lean`);
   - every `ExactIn` clause but compatibility, for the bridge's local ledger: it is the sampler's
     ledger of the region's samples, `sampleLedger ps ss (regionX ps X In)` (`local_exact`), and
     `local_correct` is (c′) for it (`DecideOnce/SampleLocal.lean`).

   - (proven) `LocalCompat`, the localized Jordan–Brouwer (group dol-jordan):
     - `Jordan.contains_const_local`: ray parity is constant along a segment that meets no face, or
       only faces whose box it stays outside;
     - `local_compat`: compatibility for near/far B-reps.
   - (proven) the six walls' region facts (`wall_region`, `wall_up`, R° open), and `SamplesIn` for
     the local arrangement's sampler (`samplesIn_of`, from `sampler_covers`).

   **Closed:** `local_correct_final hnz ws hws (h : LocalInput ps ss ws) rule`
   (`DecideOnce/LocalFinal.lean`) is (c′) for the local ledger the sampler builds. It has no hypothesis
   beyond the input's domain:
   - carriers with nonzero normals;
   - valid wall indices;
   - closed flat B-reps whose faces are near (plane a carrier) or far (a nonzero normal, and the
     region outside the face's box).

   With F1, F2 and `split_at_region`, this proves the whole of §3a except the stitching (F3), which
   is the bridge's own obligation.
