# The inc (A8 v2) core's refinement: the split

Target: `inc.IncCoreSpec` (ForgeBoolCore/IncSpecs.lean). On a well-formed v2 ledger (`LedgerWF2`)
and metric ledger (`MetricWF2`), `inc::core l r m` returns `ok`, and `outLF` of its outcome is
`coreMI (toLF l)` on `m`'s thin and enclosed rooms. With no thin rooms, `coreMI` is `coreI`
(`coreMI_nil`), and `endToEndF_of` gives `EndToEndF` at once, through `flat_refines`.

The Lean it refines is verify/forge-exact/ForgeExact/DecideOnce/IncCore.lean (`coreI` and its
stages). Funs.lean's `inc.*` is about 3.8k lines. The groups below follow the Rust functions, one
spec each, as v1's SPLIT.md did. Each group states its specs in its own file and proves them with
standard axioms and no sorry.

| group | Rust fns (Funs.lean `inc.*`) | Lean target (IncCore) | reuse from v1 |
|---|---|---|---|
| T, tables | `starts_at_zero`, `csr_ok`, `rows`, `row`, `count_in`, `ledger_wf` | `ledger_wf l = true ↔ LedgerWF2 l`; `row` = `rowL` | RefineCells-style loop specs |
| S, split | `body_of`, `cls_of`, `split_ok`, `split` (piece_of, rooms, room_pos) | `splitOk`, `rooms` (first-seen dedup), `isPieceOf` | RefineSplitOk's shape, DedupModel |
| C, classify | `flood`, `witnesses`, `membership`, keep (`apply_rule`) | `memb` (flood head = least position), `classifyOk`, `keep`, `keepTab` | RefineUF, UFModel, RefineMemb, RefineClassify, RefineKeep, RefineRule |
| K, touch | `around_e`, `around_v`, `fan_rel`, `fans`, `bad_vert`, `core_k` loops | `result`, `orient`, `aroundE`, `aroundV`, `badEdge`, `fanRel`, `badVert`, `coreK` | RefineFan, RefineUF |
| U, unify | `same_set`, `dedup`, `unify` loops | `unify` (`joinRel`, `heads`, `faceOf`, `faceKey`, `atE`/`atV`, `outV`, `ecs`, `erel`) via `IncLedger.unify_eq` | RefineUnify* (most carries over), RefineUF |
| M, collapse | `collapse` and its loops (since c2a0294) | `collapse` (MetricI) | RefineCollapse |
| E, core | `core`, `core_k` | `coreMI`, `coreK`; then `IncCoreSpec` and `EndToEndF` | the coordinator |

Size limits added: `LedgerWF2.small` now also has `n_rooms * nb ≤ Usize.max`, which `membership`'s table needs. It always holds on 64-bit targets, since both are below 2^32. It also has `nrowsL edge_facets < U32.max` and `nrowsL vert_edges < U32.max`, so the refusals' `as u32` casts name the right cell. It also has `3 * nfacets ≤ Usize.max`, which the rim sort needs (v1 had it as a hypothesis too). And it has `vert_sectors.sec.len < U32.max`, for `fans`' union-find. `sectors_csr` (the sector rows' bounds) is a new field. `ledger_wf` checks `sectors_csr` already. It checks `sectors_nodup` and every size limit too, so `ledger_wf ↔ LedgerWF2` holds (`ledger_wf_spec`).

Points to agree within each group:
- **Lists vs sets.** The Rust orders faces by head, edges by erel root, and verts in listed order,
  as the Lean does. Where an order differs, state the spec up to `List.Perm`, and tell E, which
  then weakens `IncCoreSpec` to a body correspondence.
- **Union-find heads are least positions**, which is the Lean's `head … xs`, as in v1's UF proofs.
- **Size limits** come from `LedgerWF2.small` and the Csr bounds.

## Status

| group | state | file |
|---|---|---|
| T | done: `rows_spec`, `row_spec`, `csr_ok_spec`, `ledger_wf_spec` (`ledger_wf l ⦃ b => b = true ↔ LedgerWF2 l ⦄`; every check, loop and size limit) | RefineIncTables, RefineIncWF, RefineIncWFProp |
| S | done: `body_of_spec`, `cls_of_spec`, `split_ok_spec` (`= splitOk`), `split_spec` (`SplitOK`: piece table = `isPieceOf`, `rooms` = `(toLF l).rooms`, `room_pos` inverts it) | RefineIncTables, RefineIncSplit, RefineIncRooms |
| C | done: `flood_spec`; `membership_spec` (row `i`, position `t` = `(toLF l).memb i (room rooms[t])`, via `memb_bridge` and the generic `head_leastI`); `core_loop0_spec` with `cAll_eq` (= `classifyOk`); `core_loop1_spec` with `keep_eq` (= `keep (ruleL nb r)` room by room) | RefineIncFlood, HeadBridgeI, RefineIncMemb, RefineIncClassify |
| K | done: `core_k_loop0_spec` (AInv) with `result_eq`, `orient_eq`; `around_e_spec`, `aroundE_len`, edge scan `core_k_loop1_spec` with `edges_find_eq`; `around_v_spec`, `fans_spec`, `bad_vert_spec` (= `badVert`, via the generic `fan_bad`), vert scan `core_k_loop2_spec` | RefineIncAssemble, RefineIncTouch, FanBridgeI, RefineIncFan |
| E | done: `core_k_spec` (= `coreK`, given `UnifySpec`), `core_spec` (= `coreMI`, given `UnifySpec` and `CollapseSpec`), `incCoreSpec_of`. The K lemmas now take any keep vector with `KeepRel` to a Lean table, so they serve the collapsed table too | RefineIncCore |
| U | done: `unify_specI` (= `UnifySpec`, no hypotheses added). Loops re-proven for the v2 shapes: join (`join_spec`, sound and complete for `joinRel`), faces (`faces1_spec`, `faces2_spec`), `at_e` (`ate_spec`), verts (`verts_spec`), `ecs` (`ecs_spec`), erel (`erel_spec`), edges (`edges_spec`); v1's `has`/`same_set`/`faces_at` specs and its generic class lemmas (`u_eqv_iff`, `u_cls_filter`, `u_key`) reused. Orders match the Lean's. Then `incCoreSpec_proof` (`IncCoreSpec`) and `endToEndF_proof` (`EndToEndF`) | RefineIncUnifyJoin, RefineIncUnifyFaces, RefineIncUnifyAt, RefineIncUnifyErel, RefineIncUnifyEdges, RefineIncUnifyLean, RefineIncUnify |
| M | done: `collapse_specI` (= `CollapseSpec`). Loops 0, 1, 3, 5, 6 are v1's own, by definitional equality, read through `vS`; loops 2, 4, 7, 8 re-proven (RefineIncCollapse); the Lean side by positions (`KI.collapse_pos`, RefineIncCollapseLean). `incCoreSpec_of_unify`: IncCoreSpec now needs only `UnifySpec` | RefineIncCollapseLean, RefineIncCollapse |
