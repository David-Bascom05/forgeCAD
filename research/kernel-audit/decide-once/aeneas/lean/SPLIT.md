# Splitting the Aeneas refinement of `forge_bool_core`

The goal: the Aeneas translation of `kernel/forge-bool-core` (`ForgeBoolCore/Funs.lean`) refines the
Lean core `ForgeExact.DecideOnce` (`coreM`), within stated size limits. The end-to-end theorem says
the Rust core's answer on a ledger built from the sampler is the exact regularised boolean.

Every remaining obligation is a `def …Spec : Prop` in `ForgeBoolCore/Specs.lean`. That file builds
with no `sorry`, so the groups below can work in parallel and the coordinator can assemble the end to
end from the Props.

Translation: aeneas 2e3d4488, charon 9e193f0, Lean v4.36.0, merged from
`claude/repo-review-mbdrlj-decide-once` at 2cb22d7. That version has `le3` (so no tuple `PartialOrd`
axiom) and the stage-4b `collapse`.

## Rules for every group

- **New files only**, in `ForgeBoolCore/`, named as listed below. Never edit `Funs.lean`,
  `Types.lean`, `FunsExternal.lean`, `Specs.lean`, or another group's files. If a statement in
  `Specs.lean` is wrong, or needs a hypothesis it lacks, send the change to the coordinator. Don't
  work round it locally.
- Prove `theorem foo_spec_proof : FooSpec` for each of your Props. Add your files to
  `ForgeBoolCore.lean` and `#print axioms` lines to `ForgeBoolCore/Axioms.lean`; those are the only
  two shared files you may append to. Expect merge conflicts there to be trivial.
- No `sorry`. Standard axioms only (`propext`, `Classical.choice`, `Quot.sound`).
- Gate every commit on `lake build` showing `Build completed`, with no `error`. Name clashes across
  files have broken the build before, so prefix your helper lemmas with your group letter, e.g.
  `U.`/`u_…`.
- Commits must be signed and end with the session's `Co-Authored-By`/`Claude-Session` lines. Push
  only to your own branch. Send the coordinator (`session_01Q19Lzx8iDqzvcMQsS9ACqs`) the commit
  hash; the coordinator cherry-picks it onto `claude/repo-review-mbdrlj-decide-once-lean`.
- Aeneas idioms that work here are in the proven files: `step as ⟨…⟩`,
  `loop.spec_decr_nat measure inv`, and `scalar_tac`. Watch for the postcondition flattening
  conjunctions and dropping trivial equalities (use a flat `refine ⟨…⟩`, or `trivial`), and for
  `index_mut` returning `⟨w, back, hw, hback⟩` or a pair, depending on the call site.
  `RefineAround.lean` and `RefineFan.lean` are worked examples of nested loops.

## Call graph (core path only; `witnesses` is the bridge's, not the core's)

```
core ─┬─ new_ext ── Index::new                                   [proven: index_new_spec]
      ├─ split_ok ─┬─ cov, is_facet, zero_idx, body_of, ncells   [proven]
      │            ├─ cls_of                                     (S)
      │            └─ sort_triples ── le3                        (S)
      ├─ split                                                   [proven: split_spec]
      └─ classify_on ─┬─ membership                              [proven: membership_spec]
                      ├─ the classify check                      [proven: ckAll_eq]
                      └─ keep_on ─┬─ the keep table (apply_rule)  [proven: keep_loop_spec]
                                  ├─ collapse ─ Uf, sort_u32      (K)
                                  └─ core_k ─┬─ result table      [proven: keep_res_spec]
                                             ├─ is_edge           [proven: is_edge_spec]
                                             ├─ edge scan: around [proven: edge_scan_spec, around_spec]
                                             ├─ vertex scan: around, bad_vert
                                             │                    [proven: vert_scan_spec, bad_vert_spec]
                                             └─ unify ─┬─ edges_between [proven: edges_between_spec]
                                                       ├─ faces_at, same_set, has  (U)
                                                       └─ cov, zeros, zero_idx, Uf [proven]
```

## The groups

The old plan's groups A (index), B (split and membership) and C (around, enum, between,
edges_between) are **done**, and so is `bad_vert` from D. The remaining work regroups as follows.

| group | Props to prove (`Specs.lean`) | may assume | files |
|---|---|---|---|
| **S** split_ok | `SplitOkSpec` | `cov_spec`, `is_facet_spec`, `zero_idx_spec`, `body_of_spec`, `ncells_spec`, `sort_by_cov`-style merge sort (copy the pattern of `RefineSortU32.lean` for `sort_triples` with `le3`) | `RefineSplitOk.lean` |
| **K** collapse | `CollapseSpec` | `SplitOK`, the union–find specs (`union_spec`, `find_spec`, `new_spec`, the `Least` model in `UFModel.lean`), `sort_u32_spec`, `ncells_spec` | `RefineCollapse.lean` |
| **U** unify | `UnifySpec` (with `faces_at`, `same_set`, `has`) | `edges_between_spec`, `zeros_spec`, `zero_idx_spec`, `cov_spec`, the union–find specs; **and** `ResultBridge`, `AroundBridge`, `BadEdgeBridge`, `BadVertBridge` as hypotheses (take them as arguments) | `RefineUnify.lean` |
| **R** bridges | `ResultBridge`, `AroundBridge`, `BadEdgeBridge`, `BadVertBridge` (pure Lean: positions and ids to the Lean's covectors, no `step`) | everything proven, especially `keep_res_spec`, `topes_toL`, `facets_toL`, `side_plus`/`side_minus`, `covE_inj`, `cls_spec`, `around_nodup`, `badVert_iff` | `BridgeResult.lean`, `BridgeTouch.lean` |
| **E** core (coordinator) | `NewExtSpec`, `CoreKSpec`, `ClassifyOnSpec`, `CoreSpec`, `EndToEnd`: **done**, conditional on S, K, U and R; `EncodeSpec`, `NoExtraSpec` open | all the above | `RefineCore.lean`, `EndToEnd.lean` |

**Status of E.** All proven, no `sorry`, standard axioms only:
- `end_to_end (hS : SplitOkSpec) (hK : CollapseSpec) (hU : UnifySpec) (hR : ResultBridge) (hE : BadEdgeBridge) (hV : BadVertBridge) : EndToEnd`;
- `core_spec_of`, `classify_on_spec_of`, `core_k_spec_of`, `keep_on_lemma`, `new_ext_spec_proof`;
- `coreM_nil` (no thin cells: `coreM` is `core`).

So the groups' Props are exactly what remains. `AroundBridge` isn't used by E; it is there for U if U wants it.

Group U is the largest: eight loops and three helpers. Splitting it further is possible:
`faces_at`/`same_set`/`has` are separable from `unify`'s loops, with their own small specs (set
membership of `facesAt`). If U is split, the coordinator adds those Props to `Specs.lean` first.

### Size limits (stated as hypotheses; `LedgerWF` in `Specs.lean`)

`LedgerWF` also requires:
- `0 < nb`, i.e. at least the target body;
- fewer than `u32::MAX` edges and verts;
- the sorts' `3·n` scratch within `usize`.

All of these are assumed by the Props, and the proofs show nothing overflows or goes out of bounds
within them:

- cells (ledger cells plus the core's own) < `u32::MAX` (`NONE`);
- `cells + 2·pieces + 1 < u32::MAX`;
- face, body and class ids < `u32::MAX`;
- `3·n ≤ usize::MAX` for the sorts;
- a cell's zero count ≤ 12 (`ENUM_ZEROS`) in the enumerating branches of `around` and `between`.
  Past that they scan, so shifts stay below 32 bits, at most 12·2¹¹ hits, at most 2¹² lookups.

## The frontier (local ledger, spec §3a, commit 5f12a6e)

**It changes neither the call graph nor the groups.** The core is unchanged; the region is the
bridge's choice of which cells to list.

- **(b′) is (b) verbatim.** `Valid` and `Consistent` (`Coverage.lean`) quantify only over the
  ledger's pieces, topes, facets, edges, verts and result, which are all listed data. The frontier is
  unlisted, so no Prop here mentions it.
- **(c′)** is `Exact` over the local arrangement: the near carriers plus the six walls, the walls
  carrying no face. `flat_correct` proves (c) for any `sampleLedger` of a `FlatInput`. Two pieces
  remain:
  - (c′-1) the local ledger *is* such a sampler ledger restricted to open R, with pieces only from
    near faces and sides in R°;
  - (c′-2) compat per cell holds because no far face meets R°.

  Both are Lean obligations over `ForgeExact.DecideOnce.Sample`, not Aeneas.
- **F1 (far unchanged)** and **F2 (frontier manifold)** are geometry over `ForgeExact.Ref` and
  `Regular.lean`'s `reg`. Neither touches the translation. Together with (c′) they give the global
  statement:
  - reg boolean = its part in R° ∪ (A₀ \ R° or ∅);
  - manifold iff manifold at the cells of R°.

  This could be a group **F**, with files in `verify/forge-exact/ForgeExact/DecideOnce/Frontier.lean`.
  It is independent of all the groups above.
- §6's orientation of a side next to the body's own face is taken as a **stated hypothesis** for
  now.

## Integration

The coordinator:
- cherry-picks each group's commits;
- runs `lake build` and checks the `#print axioms` lines;
- proves group E, assembling `EndToEnd` from `CoreSpec`, `EncodeSpec` and `flat_correct`.

`EndToEnd` is the single theorem that states the result.
