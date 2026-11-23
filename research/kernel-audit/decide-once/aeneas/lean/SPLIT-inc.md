# The inc (A8 v2) core's refinement: the split

Target: `inc.IncCoreSpec` (ForgeBoolCore/IncSpecs.lean). On a well-formed v2 ledger (`LedgerWF2`),
`inc::core` returns `ok`, and `outLF` of its outcome is `coreI (toLF l)`. With it,
`endToEndF_of` gives `EndToEndF` at once, through `flat_refines`.

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
| E, core | `core` | `coreI`; then `IncCoreSpec` and `EndToEndF` | the coordinator |

Size limits added: `LedgerWF2.small` now also has `n_rooms * nb ≤ Usize.max`, which `membership`'s table needs. It always holds on 64-bit targets, since both are below 2^32.

Points to agree within each group:
- **Lists vs sets.** The Rust orders faces by head, edges by erel root, and verts in listed order,
  as the Lean does. Where an order differs, state the spec up to `List.Perm`, and tell E, which
  then weakens `IncCoreSpec` to a body correspondence.
- **Union-find heads are least positions**, which is the Lean's `head … xs`, as in v1's UF proofs.
- **Size limits** come from `LedgerWF2.small` and the Csr bounds.

## Status

| group | state | file |
|---|---|---|
| T | done: `rows_spec`, `row_spec` (`ledger_wf`'s equivalence is still to do) | RefineIncTables.lean |
| S | done: `body_of_spec`, `cls_of_spec`, `split_ok_spec` (`= splitOk`), `split_spec` (`SplitOK`: piece table = `isPieceOf`, `rooms` = `(toLF l).rooms`, `room_pos` inverts it) | RefineIncTables, RefineIncSplit, RefineIncRooms |
| C | started: `flood_spec` (UInv of the non-piece crossings, by room position); `membership_spec` (row `i`, position `t` = `(toLF l).memb i (room rooms[t])`, via `memb_bridge` and the generic `head_leastI`). Next: `core_loop0` (`classifyOk`), `core_loop1` (keep) | RefineIncFlood, HeadBridgeI, RefineIncMemb |
| K, U, E | to do | |
