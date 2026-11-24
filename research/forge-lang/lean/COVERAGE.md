# forge-lang: what the Lean covers, item by item

Every built-in, document header, keyword and edit op the Rust accepts, with the Lean that covers
it or a GAP that says what is not proven. `interface/forge-lang/tests/lang_coverage.rs` reads the
Rust and fails when an item has no row here, when a row's item is gone, or when a theorem a row
cites does not exist: a language extension must bring its row, even if the row says GAP.

Conventions (the test parses the tables): `Kind` is `builtins`, `header`, `keywords`, `op` or
`not-names`; `Names` lists a family's names in backticks; a Lean cell cites `File.theorem` in
backticks, the file under `ForgeLang/` with `/` written `.` (`LL1.Documents.milling_doc_accepted`
is `theorem milling_doc_accepted` in `ForgeLang/LL1/Documents.lean`), or says GAP. Build and
axioms: `lake build`, `lake env lean CheckAxioms.lean` (standard axioms only, no `sorry`, no
`native_decide`).

## Three ways names resolve

* **The part's one namespace** (parts, assemblies' part-built-ins): a binding first (a param, a
  name in scope, a shared param, a user `fn`), then a constant, then a built-in; binding a
  built-in's or constant's name with `=`, `shared`, `param`, `for` or a sketch/path name is the
  refusal `ShadowsBuiltin` (`Eval::bind`, `builtins::NAMES`). A `fn`'s name and its params are
  bindings without that refusal: they are read before any built-in, so a built-in that comes later
  never changes them. `Builtins.extend_conservative`: adding a family `N` refuses only documents
  that bind a name of `N` and changes the meaning of no other name. Its hypothesis, for every
  family here, is `lang_coverage::shadow_families_bound_nowhere` (every `.fg` that is not a
  machining document or drawing; files already refused with `ShadowsBuiltin` for that name are
  counted, not failed: `research/kernel-audit/audit2/ell.fg` binds `cut`).
* **Closed** (`machining` documents: forge-cam-lang, forge-turn-lang; `robot` documents,
  forge-lang `robot.rs`, which add bare statement calls from a table of their own): a call's head is looked up
  among the built-ins only, a read among the bindings only; no refusal for binding a built-in's
  name (`drill = drill(..)` is fine). `Families.run_ext_accepted`: adding match arms changes no
  accepted document; `Families.stmt_new_refused`: a document calling a new name was refused
  before. No hypothesis on the documents is needed.
* **After bindings, not refused** (assembly joints and mates, `asm::constant`; a harness block's built-ins): read after every binding and constant, and a binding may take the name: `Builtins.extend_unrefused`, no hypothesis. Drawings are **closed** (`Families.drawing_conservative`).

Routing (forge-cli `fg.rs`): a `machining` document with a `lathe_setup(..)` statement goes to
forge-turn-lang, any other to forge-cam-lang. It reads only those two names, never the tables, so
no family extension changes where a document goes.

## Built-in families

| Kind | Item | Names | Rust site | Lean |
|---|---|---|---|---|
| builtins | `core` | `use`, `sqrt`, `div`, `mod`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `abs`, `min`, `max`, `count`, `ratio`, `len`, `polar`, `pt`, `near`, `line`, `arc`, `circle`, `ellipse`, `earc`, `rect`, `slot`, `hull`, `disc`, `poly`, `spline`, `tarc`, `close`, `tangent`, `coincident`, `horizontal`, `vertical`, `parallel`, `perpendicular`, `equal`, `midpoint`, `distance`, `radius`, `angle`, `fix`, `concentric`, `collinear`, `symmetric`, `hdistance`, `vdistance`, `extrude`, `cut`, `revolve`, `fillet`, `chamfer`, `shell`, `loft`, `offset`, `rotate`, `edges`, `faces`, `shift`, `hline`, `vline`, `meet`, `mid`, `plane`, `properties` | `interface/forge-lang/src/builtins.rs` `BUILTINS`; part namespace (`builtins::NAMES`, ShadowsBuiltin) | `Builtins.extend_conservative` (with `N` the family; its hypothesis, no refusal-subject binding of the names, checked by lang_coverage::shadow_families_bound_nowhere), `Builtins.resolve_append_off`, `Builtins.shadows_new`; the base list `B` |
| builtins | `shapes` | `sweep`, `coil`, `draft`, `offset_faces`, `delete_faces`, `replace_face`, `move_faces`, `thicken`, `rib`, `facefillet`, `fullround`, `ball`, `sphere`, `dome` | `shapes.rs` `NAMES`; part namespace (`builtins::NAMES`, ShadowsBuiltin) | `Builtins.extend_conservative` (with `N` the family; its hypothesis, no refusal-subject binding of the names, checked by lang_coverage::shadow_families_bound_nowhere), `Builtins.resolve_append_off`, `Builtins.shadows_new` |
| builtins | `constants` | `pi`, `X`, `Y`, `Z`, `xy`, `xz`, `yz`, `thru`, `thru_all`, `next`, `axis`, `sym`, `two`, `upto`, `intersect` | `builtins.rs` `CONSTANTS`, `MORE`; part namespace (`builtins::NAMES`, ShadowsBuiltin) | `Builtins.extend_conservative` (with `N` the family; its hypothesis, no refusal-subject binding of the names, checked by lang_coverage::shadow_families_bound_nowhere), `Builtins.resolve_append_off`, `Builtins.shadows_new` |
| builtins | `sheet` | `sheet`, `flange`, `cutout`, `unfold`, `contour`, `hem`, `kfactor`, `ktable`, `bend_table`, `per_t`, `stock`, `shop`, `obround`, `circular_relief`, `square_relief`, `open_corner`, `overlap_corner`, `corner_relief`, `constant_width_corner`, `seam`, `corner_round`, `punch`, `form`, `sketched_bend`, `jog`, `hardware`, `pem`, `relief_rule`, `vendor` | `sheet.rs` `BUILTINS`; part namespace (`builtins::NAMES`, ShadowsBuiltin) | `Builtins.extend_conservative` (with `N` the family; its hypothesis, no refusal-subject binding of the names, checked by lang_coverage::shadow_families_bound_nowhere), `Builtins.resolve_append_off`, `Builtins.shadows_new` |
| builtins | `sheet-constants` | `tear`, `no_relief`, `closed_corner`, `tear_corner`, `trim_to_bend` | `sheet.rs` `CONSTANTS`; part namespace (`builtins::NAMES`, ShadowsBuiltin) | `Builtins.extend_conservative` (with `N` the family; its hypothesis, no refusal-subject binding of the names, checked by lang_coverage::shadow_families_bound_nowhere), `Builtins.resolve_append_off`, `Builtins.shadows_new` |
| builtins | `thermo` | `thermoform`, `thermo_material`, `thermo_shop` | `thermo.rs` `BUILTINS`; part namespace (`builtins::NAMES`, ShadowsBuiltin) | `Builtins.extend_conservative` (with `N` the family; its hypothesis, no refusal-subject binding of the names, checked by lang_coverage::shadow_families_bound_nowhere), `Builtins.resolve_append_off`, `Builtins.shadows_new` |
| builtins | `weld` | `members`, `end_trim`, `end_cap`, `corner_plate` | `weld.rs` `BUILTINS`; part namespace (`builtins::NAMES`, ShadowsBuiltin) | `Builtins.weld_conservative`, `Builtins.weld_not_keywords`; hypothesis checked by weld::builtins_bound_nowhere and lang_coverage::shadow_families_bound_nowhere |
| builtins | `wired` | `hole`, `thread`, `pattern`, `lin`, `circ`, `mirror`, `follow`, `places`, `workplane`, `workaxis`, `workpoint`, `ucs`, `union`, `subtract`, `combine`, `split`, `splitface`, `copybody`, `movebody`, `slide`, `spin`, `place`, `text`, `import`, `project`, `entity`, `vary`, `verts` | `wired.rs` `NAMES`; part namespace (`builtins::NAMES`, ShadowsBuiltin) | `Builtins.extend_conservative` (with `N` the family; its hypothesis, no refusal-subject binding of the names, checked by lang_coverage::shadow_families_bound_nowhere), `Builtins.resolve_append_off`, `Builtins.shadows_new` |
| builtins | `assembly` | `part`, `subassembly`, `pose`, `fastened`, `revolute`, `slider`, `cylindrical`, `pin_slot`, `planar`, `ball`, `coincident`, `on_plane`, `on_line`, `mate`, `flush`, `concentric`, `insert`, `parallel`, `perpendicular`, `angle`, `distance`, `lock`, `gear`, `rack_pinion`, `screw`, `linear`, `fit`, `fit_rule`, `kind`, `std`, `ground` | `asm.rs` `NAMES`; own | `Builtins.extend_unrefused` (resolved after every binding, outside the refusal table: a binding may take the name and wins, so adding one changes no document, no hypothesis), `Builtins.unrefused_new`; the order among families: a name in two (a sketch constraint, `ball`) is dispatched by `asm::call` on its context, not modelled |
| builtins | `assembly-part` | `anchor`, `anchor_kind` | `asm.rs` `PART_NAMES`; own | `Builtins.extend_unrefused` (resolved after every binding, outside the refusal table: a binding may take the name and wins, so adding one changes no document, no hypothesis), `Builtins.unrefused_new` |
| builtins | `assembly-kinds` | `external_thread`, `internal_thread`, `shaft`, `bore` | `asm.rs` `KINDS`; own | `Builtins.extend_unrefused` (resolved after every binding, outside the refusal table: a binding may take the name and wins, so adding one changes no document, no hypothesis), `Builtins.unrefused_new` |
| builtins | `harness` | `wire_type`, `cable_type`, `connector`, `node`, `segment`, `wire`, `cable`, `twist` | `harness.rs` `BUILTINS`; own | `Builtins.extend_unrefused` (resolved after every binding, outside the refusal table: a binding may take the name and wins, so adding one changes no document, no hypothesis), `Builtins.unrefused_new`, per context: inside a `harness` block the list has them, outside it does not (`harness::constant`) |
| builtins | `drawing` | `part`, `assembly`, `sheet`, `base_view`, `projected`, `section`, `detail`, `flat_view`, `dim`, `angle`, `radius`, `diameter`, `hole_note`, `ordinate`, `baseline`, `chain`, `center_mark`, `center_marks`, `centerlines`, `centerline`, `leader`, `note`, `balloon`, `auto_balloons`, `fcf`, `datum`, `datum_target`, `surface`, `weld`, `edge`, `parts_list`, `e`, `f` | `drawing/doc.rs` `FUNCTIONS`; closed namespace (the drawing evaluator) | `Families.drawing_conservative` (closed: `x = f(..)` and a nested call from `FUNCTIONS` alone, reads from bindings alone), `Families.run_ext_accepted`, `Families.stmt_new_refused` |
| builtins | `cam` | `part`, `machine`, `tool`, `setup`, `facing`, `contour`, `pocket`, `clear`, `drill`, `engrave`, `slot`, `thread_mill`, `edge_break`, `soft_jaw`, `program` | `interface/forge-cam-lang/src/lib.rs` `Cam::call` arms; closed namespace | `Families.cam_conservative`, `Families.run_ext_accepted`, `Families.run_ext_avoid`, `Families.stmt_new_refused`, `Families.families_not_keywords`; `LL1.Documents.milling_doc_accepted` |
| builtins | `cam-expr` | `box`, `vise` | `Cam::eval` (`fname == ..`); closed namespace | `Families.cam_conservative`, `Families.run_ext_accepted`, `Families.run_ext_avoid`, `Families.stmt_new_refused`, `Families.families_not_keywords` |
| builtins | `turn` | `part`, `lathe_tool`, `lathe_setup`, `turn_face`, `turn_rough`, `bore_rough`, `turn_finish`, `turn_groove`, `turn_thread`, `part_off`, `turn_drill`, `lathe_program` | `interface/forge-turn-lang/src/lib.rs` `Turn::call` arms; closed namespace | `Families.turn_conservative`, `Families.run_ext_accepted`, `Families.run_ext_avoid`, `Families.stmt_new_refused`, `Families.families_not_keywords`; `LL1.Documents.turning_doc_accepted` |
| builtins | `turn-expr` | `chuck` | `Turn::eval` (`match fname.as_str()`); closed namespace | `Families.turn_conservative`, `Families.run_ext_accepted`, `Families.run_ext_avoid`, `Families.stmt_new_refused`, `Families.families_not_keywords` |
| builtins | `robot-assign` | `source`, `link`, `joint`, `frame` | `interface/forge-lang/src/robot.rs` `run`'s `match fname` (then `Reader::value`); closed namespace | `Families.robot_conservative`, `Families.run_ext_accepted`, `Families.run_ext_avoid`, `Families.stmt_new_refused`, `Families.exec_new_refused`, `Families.families_not_keywords`; Rename `FamiliesRename.rename_run` |
| builtins | `robot-stmt` | `root`, `cut`, `freeze`, `merge`, `drop`, `visual`, `collision`, `collisions`, `actuation`, `inertial`, `joint`, `transmission` | `robot.rs` `statement` (a bare `f(..)` statement, `St.exec`); closed namespace | `Families.robot_conservative`, `Families.run_ext_accepted`, `Families.run_ext_avoid`, `Families.stmt_new_refused`, `Families.exec_new_refused`, `Families.families_not_keywords`; Rename `FamiliesRename.rename_run` |
| builtins | `robot-value` | `pose`, `box`, `cylinder`, `capsule`, `sphere`, `mesh` | `robot.rs` `Reader::value` (nested, or an assignment's fall-through); closed namespace | `Families.robot_conservative`, `Families.run_ext_accepted`, `Families.run_ext_avoid`, `Families.stmt_new_refused`, `Families.exec_new_refused`, `Families.families_not_keywords`; Rename `FamiliesRename.rename_run` |
| builtins | `robot-constants` | `hull`, `none` | `robot.rs` `RESERVED`: read before any binding, and binding one is refused (`ReservedName`) | `Builtins.extend_conservative`: no accepted robot document binds them, so reading them before or after bindings is the same, and the part namespace's argument applies (a new reserved word refuses only documents that bind it); `Families.families_not_keywords` |

## Document headers and keywords

The grammar (`interface/forge-lang/grammar.ll1`) is proven LL(1) as generated into
`LL1/Impl.lean` (`the_lean_grammar_is_current` keeps it current); the Rust parser is tied to the
model answer for answer by `tests/parse_diff.rs` (`FG_LEAN=1`, in the lean workflow) on random
token strings, random programs and edits, and every `.fg` in the repository, the machining
documents in `examples/machined/` included. Headers that are names add no syntax.

| Kind | Item | Names | Rust site | Lean |
|---|---|---|---|---|
| header | `machining` | | forge-cam-lang `is_machining`, forge-turn-lang `is_turning` (first statement the bare name; turning when a statement calls `lathe_setup`) | `LL1.Documents.milling_doc_accepted`, `LL1.Documents.turning_doc_accepted`, `LL1.Documents.header_glued_refused`, `Families.families_not_keywords`; semantics `Families.run_ext_accepted` |
| header | `drawing` | | forge-lang `drawing::doc::declared` (and the `drawing` keyword's statement) | `LL1.Unified.accept_drawing_header`, `LL1.Unified.accept_drawing_empty` (on the unified prototype grammar; on `Impl` by `LL1.Impl.grammar_ll1` and parse_diff); semantics `Families.drawing_conservative` |
| header | `assembly` | | forge-render `is_assembly` (optional first line; an assembly is a file with components or joints) | `LL1.Documents.assembly_header_accepted`, `Families.families_not_keywords`. Semantics: GAP (assembly namespace, see `assembly`) |
| header | `robot` | | forge-lang `robot::is_robot` (first statement the bare name) | `LL1.Documents.robot_doc_accepted` (its own example lexes and parses), `Families.families_not_keywords`; semantics `Families.robot_conservative`; the grammar is the one parse_diff checks on every file |
| keywords | `syntax::KEYWORDS` | `shared`, `param`, `fn`, `for`, `in`, `if`, `else`, `return`, `expect`, `and`, `or`, `not`, `true`, `false`, `sketch`, `path`, `at`, `component`, `drawing`, `harness` | `interface/forge-lang/src/syntax.rs` (`RESERVED`: `at`, a keyword that texts used as a name) | `LL1.Impl.grammar_ll1`, `LL1.Impl.impl_unambiguous`, `LL1.Impl.source_to_tree`, `LL1.Impl.impl_roundtrip`, `LL1.Driver.rd_eq_run`, `LL1.Lex.word_maximal`, `LL1.Lex.lex_lossless`, `LL1.LexUnits.in_keyword_after_number` |

## Arguments

A built-in's arguments are its keywords and positionals (`#0`, `#1`, ..). Where a crate lists
them (`pub const ARGS: &[(&str, &[&str])]`, the list `named` checks and CAM-UI's dialog schema is
checked against), each built-in has a row here whose names are its list's; until then the kind
has one row, a GAP. The check: `FamiliesArgs.checkArgs_ok_iff` (a call is taken exactly when every
argument it gives is listed), `FamiliesArgs.misspelled_refused`, and for an extension
`FamiliesArgs.checkArgs_extend` (a built-in that gains arguments takes every call it took) and
`FamiliesArgs.checkArgs_new`.

| Kind | Item | Names | Rust site | Lean |
|---|---|---|---|---|
| args | `cam` | | `interface/forge-cam-lang/src/lib.rs` `named`: no list yet | GAP: a keyword no built-in reads is ignored (`stepovr:` runs on the default); the check, once a list exists, is `FamiliesArgs.misspelled_refused` |
| args | `turn` | | `interface/forge-turn-lang/src/lib.rs` `named_args`: no list yet | GAP: as for `cam`; `FamiliesArgs.misspelled_refused` once a list exists |

## Edit ops

The theorems are about the core model of a part document (`Core.lean`). The text-level ones
(`Print`, `Respell.respell_text`, `Suppress.suppress_text_closure`, `LL1.ReparseModel`) hold of
any document. **Machining documents** (CAM-UI sends exactly SetArg, AddFeature, DeleteFeature,
MoveFeature, ReplaceFeature, Suppress, Unsuppress and Rename to them, through
`forge_lang::edit::apply`), and robot documents likewise: a call's head there is a built-in,
never a read (`edit_ops::calls_closed`, used by every precheck, Suppress's closure, Respell's reads
and Rename),
which is the model's `Families.readsE`. Proven there: Rename (`FamiliesRename.rename_run`); every
op leaves the statements before the first it changes exactly as they were, results, state and
environment (`FamiliesRename.edit_prefix`, `FamiliesRename.run_append`); the run is a function of
the text. GAP, and no theorem of the parts' form can hold: what later statements compute, since
their built-ins read the state earlier ones leave (an op the in-process stock, `program` a setup's
ops); the text-level theorems (`Print`, `Suppress.suppress_text_closure`) are the core printer's.

| Kind | Item | Names | Rust site | Lean |
|---|---|---|---|---|
| op | `SetParam` | | `edit.rs` `apply`; text: `syntax::reparse_number` | `Edits.setParam_roundtrip`, `Edits.setParam_ids`, `Edits.sole_driver`, `Print.setLit_text`, `LL1.Lex.num_literal`, `LL1.ReparseModel.reparse_number_correct`, `LL1.Reparse.reparse_parse`; observed literal: a re-run (`Loops.count_prefix`, `Loops.keyed_insert`) |
| op | `AddLocalParam` | | `edit.rs` (`param F.name = v`) | a name nothing reads: adding its binding is `Complete.add_complete` (a literal binding adds no fragment: the same model; `Scoped.addParam_occ` for a component's). A name `F` already reads (a local param over a shared one of that name): `F` now reads `v`, a ReplaceFeature of `F` keeping its name, so `EditClosure.edit_closure` (everything outside the closure of `F`'s name unchanged). GAP: the local scope itself (one feature's params) is not in the core; that the edited document evaluates |
| op | `MakeShared` | | `edit.rs` | `Shared.makeShared_complete`, `Shared.makeShared_sound`, `Shared.makeShared_nums` |
| op | `SplitInstance` | | `edit.rs` | `Split.split_doc`, `Sites.split_independent`, `Sites.split_edit_one`; counterexample `Counter.ValueEdit.split_edit_one` |
| op | `AddFeature` | | `edit.rs` | `Complete.add_complete`, `Scoped.scopedAdd_ok`, `Print.body_text_insert`, `Scoped.scoped_text_insert`; sound half `Struct.execS_append` |
| op | `DeleteFeature` | | `edit.rs` | `Struct.delete_sound`, `Complete.delete_complete`, `Scoped.scopedDelete_ok`; Counter/Rebind.lean shows single assignment is needed |
| op | `ReplaceFeature` | | `edit.rs` | `ScopedMore.replace_complete`, `ScopedMore.scopedReplace_ok` (the statement keeps its name and nothing after it reads it: complete, every other fragment unchanged); a statement that is read: `EditClosure.edit_closure` (both documents evaluate: every fragment before it, and after it outside the closure of its name, unchanged; `EditClosure.closure_runs`). GAP: that the edited document evaluates when the closure's readers accept the new value (a re-run); in a component's block (scoped) |
| op | `MoveFeature` | | `edit.rs` | `Struct.move_sound`, `Complete.move_complete`, `ScopedMore.move_down`, `ScopedMore.move_up`, `ScopedMore.scopedMove_ok`, `Print.body_text_swap` |
| op | `LiftLiteral` | | `edit.rs` | shared scope: `LiftLiteral.liftLiteral_body` (if the document evaluates, the lifted one evaluates to the same model and log: every id, trace and observed site as it was, the literal's site now the param's), `LiftLiteral.lift_nested`; `Local` scope: `LiftLiteral.liftLiteral_at` (the binding right before the feature that holds the literal, the statements before it untouched; a fresh name, so seen by that feature alone or by every later one reads the same). GAP: the text splice |
| op | `SetArg` | | `edit_ops.rs` `apply_set_arg` (a call's argument, or `arg: text` added), `apply_component_arg` | a literal argument or component default set to a literal: SetParam on its site (`Edits.setParam_roundtrip`, as Scoped.lean notes); any text: a ReplaceFeature keeping the name, so `ScopedMore.replace_complete` (nothing reads it) and `EditClosure.edit_closure` (it is read: everything outside the closure of its name unchanged). GAP: that the edited document evaluates (a re-run of the closure); the text splice parsing back to the one-argument change (checked by reparsing at run time, not proven); machining documents (see above) |
| op | `Rename` | | `edit_ops.rs` `precheck_rename`, `apply_rename`, `ast_rename` (token-level: the binding and its reads, in the document and in its function, procedure and component bodies; not members, labels or params' names; in a machining or robot document, `calls_closed`, not a call's head) | part documents: `RenameDefs.rename_defs` (definitions renamed with the document, as the op does: the renamed document evaluates at the same fuel to the same traces and log, every binding renamed and every feature's id renamed in its first segment, `a`'s features becoming `b`'s), `RenamePart.renFirst_keep` (no other id changes), `RenamePart.rename_body` (the case no definition reads either name), `RenameDefs.renD_nested`, `RenameDefs.renD_top`. Machining and robot documents: `FamiliesRename.rename_run`, `FamiliesRename.rename_accepted`. GAP: a name bound inside a loop's body (Rust's `bound` lists those at top level; the core nests them under the loop's id), a definition's parameter named `a` or `b`, the run's duplicate-id check after renaming, and the text splice parsing back |
| op | `SetProperty` | | `props.rs` `apply_set` (a value reading no name, in the one `properties(..)` statement, which binds nothing and makes no feature) | as a ReplaceFeature of a statement nothing reads: `ScopedMore.replace_complete` and `EditClosure.edit_closure` (every feature's id, trace and log unchanged). GAP: what the properties feed (title block, BOM, the material's density in `Run::physical`) is outside the core |
| op | `Suppress` | | `edit_ops.rs` `apply_suppress` | `Suppress.suppress_complete`, `Suppress.suppress_stmts`, `Suppress.suppress_text_closure`, `Suppress.suppress_live`; ids `HistoryId.historyId_stable` |
| op | `Unsuppress` | | `edit_ops.rs` | `Suppress.unsuppress_suppress` |
| op | `Respell` | | `edit_ops.rs` `apply_respell` | `Respell.respell_read_binders`, `Respell.respell_text`, `Respell.respellChecked_read`, `Respell.respell_refuses_binder` |
| op | `Undo` | | `undo.rs`; CLI `{"op":"Undo"}` (forge-cli `fg.rs`) | `Undo.undo_write`, `Undo.redo_undo`, `Undo.step_refused`, `Undo.history_keep`; `Undo.unchecked_loses_edit` (the check is needed) |
| op | `Redo` | | `undo.rs`; CLI | `Undo.undo_redo`, `Undo.step_refused`, `Undo.history_keep` |
| op | `MakeComponent` | | `refactor.rs` | `Refactor.makeComponent`; Counter/Refactor.lean |
| op | `ExtractComponent` | | `refactor.rs` | `Refactor.extract_occ`, `Modules.link_inert`; Counter/Refactor.lean |
| op | `InlineComponent` | | `refactor.rs` | `Refactor.extract_occ` (the same equation read the other way) |
| op | `robot::Set` | | `robot.rs` `edit` (`field: text` on a statement, `field_was` too) | a SetArg on a robot statement: `FamiliesRename.edit_prefix`, `FamiliesRename.run_append` (every statement before the first changed one as it was). GAP: later statements (stateful built-ins); the text is re-parsed at run time, not proven |
| op | `robot::Reset` | | `robot.rs` `edit` (a field off; a statement left with its target alone goes) | `FamiliesRename.edit_prefix`, `FamiliesRename.run_append` (every statement before the first changed one as it was). GAP: later statements (stateful built-ins); the text is re-parsed at run time, not proven |
| op | `robot::Add` | | `robot.rs` `edit` (a statement at the end) | the old document is the prefix: every statement keeps its result and the state it leaves (`FamiliesRename.edit_prefix` with the old statements as `pre`). GAP: the added statement's own result |
| op | `robot::Remove` | | `robot.rs` `edit` (the statement on a line) | `FamiliesRename.edit_prefix`, `FamiliesRename.run_append` (every statement before the first changed one as it was). GAP: later statements (stateful built-ins); the text is re-parsed at run time, not proven; no read check (a removed binding read later is that reader's typed error) |
| op | `robot::Put` | | `robot.rs` `edit` (the one `call(..)` statement replaced, or added) | `FamiliesRename.edit_prefix`, `FamiliesRename.run_append` (every statement before the first changed one as it was). GAP: later statements (stateful built-ins); the text is re-parsed at run time, not proven |
| op | `drawing::SetArg` | | `drawing/ops.rs` (an item's argument; a drawing file or block) | `FamiliesRename.edit_prefix` (every item before the first changed one as it was). Each op is checked by Rust: the result must read with no more errors than the source (`apply_checked`). GAP: what later items compute (views read the part's state) |
| op | `drawing::AddItem` | | `drawing/ops.rs` (a line `name = func(..)`) | `FamiliesRename.edit_prefix` (every item before the first changed one as it was). Each op is checked by Rust: the result must read with no more errors than the source (`apply_checked`). GAP: what later items compute (views read the part's state) |
| op | `drawing::DeleteItem` | | `drawing/ops.rs` (refused when another item reads it: `Breaks`) | `FamiliesRename.edit_prefix` (every item before the first changed one as it was). Each op is checked by Rust: the result must read with no more errors than the source (`apply_checked`). GAP: what later items compute (views read the part's state) |

## Lists of strings that are not names a document resolves

Every other `const` list of strings in forge-lang, forge-cam-lang and forge-turn-lang, and why it
is not a family (keyword arguments and members are words after `name:` or `.`, not names in
scope).

| Kind | Item | Names | Why | Lean |
|---|---|---|---|---|
| not-names | `interface/forge-lang/src/anchors.rs::FACE_ROLES` | | members a feature's faces are found by (read after `.`), not names in scope | not a namespace |
| not-names | `interface/forge-lang/src/asm.rs::METRICS` | | an assembly's `expect` metrics: the word after `expect`, not a name in scope | not a namespace |
| not-names | `interface/forge-lang/src/asm.rs::PLACES` | | calls that make a file an assembly (`has_components`), a subset of the assembly family | not a namespace |
| not-names | `interface/forge-lang/src/bench.rs::NAMES` | | benchmark part files | not a namespace |
| not-names | `interface/forge-lang/src/bendsim.rs::LENGTHS` | | keys of a vendor file (TOML), not .fg | not a namespace |
| not-names | `interface/forge-lang/src/builtins.rs::CONTROL` | | keyword arguments of `spline` | not a namespace |
| not-names | `interface/forge-lang/src/builtins.rs::PURE` | | the built-ins that are functions of numbers alone (a subset of `core`), exempt from the magnitude bound (`Bounds.runK_of_run`) | not a namespace |
| not-names | `interface/forge-lang/src/builtins.rs::SLOPES` | | keyword arguments of `spline` | not a namespace |
| not-names | `interface/forge-lang/src/eval.rs::WORDS` | | depth words, a subset of the `constants` family | not a namespace |
| not-names | `interface/forge-lang/src/explode.rs::SCREWS` | | standard screw file names | not a namespace |
| not-names | `interface/forge-lang/src/fuzz.rs::UNREAD` | | error kinds (the fuzzer's) | not a namespace |
| not-names | `interface/forge-lang/src/harness.rs::CSV_COLUMNS` | | wire-list CSV columns | not a namespace |
| not-names | `interface/forge-lang/src/harness.rs::PLAIN` | | harness library CSV columns | not a namespace |
| not-names | `interface/forge-lang/src/holes.rs::HOLE_ARGS` | | keyword arguments of `hole` | not a namespace |
| not-names | `interface/forge-lang/src/holes.rs::THREAD_FORMS` | | keyword arguments of `thread` | not a namespace |
| not-names | `interface/forge-lang/src/media.rs::FONT_KINDS` | | file extensions | not a namespace |
| not-names | `interface/forge-lang/src/media.rs::SOLID_KINDS` | | file extensions | not a namespace |
| not-names | `interface/forge-lang/src/props.rs::STATES` | | values of the `state` property (strings) | not a namespace |
| not-names | `interface/forge-lang/src/shapes.rs::SOLID` | | feature kinds that make solid, a subset of the `shapes` family | not a namespace |
| not-names | `interface/forge-lang/src/sheet.rs::METHODS` | | methods of sheet values (`S.edge(..)`, `U.of(..)`), read after `.`, not names in scope | not a namespace |
| not-names | `interface/forge-lang/src/syntax/table.rs::TERMS` | | the grammar's terminals (generated; `LL1.Impl.grammar_ll1`) | not a namespace |
| not-names | `interface/forge-lang/src/syntax/table.rs::NONTERMS` | | the grammar's nonterminals (generated; `LL1.Impl.grammar_ll1`) | not a namespace |
| not-names | `interface/forge-lang/src/syntax.rs::OPS` | | the lexer's operators (`LL1.Impl.grammar_ll1` holds Impl.ops; `LL1.Lex.lex_lossless`) | not a namespace |
| not-names | `interface/forge-lang/src/syntax.rs::RESERVED` | | words that became keywords after texts used them as names (`at`): a lint, in the `keywords` row | not a namespace |
| not-names | `interface/forge-lang/src/thermo.rs::ARGS` | | keyword arguments of `thermoform` | not a namespace |
| not-names | `interface/forge-lang/src/thermo_kept.rs::STAGES` | | thermoform stage names (strings) | not a namespace |
| not-names | `interface/forge-lang/src/units.rs::FUNCS` | | functions a GUI number field may call (units.rs), not a .fg document | not a namespace |
| not-names | `interface/forge-lang/src/weld.rs::ALIGN` | | values of `members(.., align:)` (strings) | not a namespace |
| not-names | `interface/forge-lang/src/work.rs::PLANE_FORMS` | | keyword arguments of `workplane` | not a namespace |
| not-names | `interface/forge-lang/src/work.rs::AXIS_FORMS` | | keyword arguments of `workaxis` | not a namespace |
| not-names | `interface/forge-lang/src/work.rs::POINT_FORMS` | | keyword arguments of `workpoint` | not a namespace |
| not-names | `interface/forge-lang/src/work.rs::UCS_FORMS` | | keyword arguments of `ucs` | not a namespace |
| not-names | `interface/forge-lang/src/work.rs::KEYS` | | keyword arguments of the work features | not a namespace |

## Findings

* forge-cam-lang dropped every `param OWNER.NAME` of a machining document (the owner keyed by the
  path's Debug text, matched by suffix): fixed, an exact id match, as `Families.St.bindCall`
  models it (test cam::a_statements_params_are_its_own).
* Rename on a machining document rewrote a call's head when a binding shared a built-in's name
  (`drill = drill(..)` became `holes = holes(..)`, an unknown built-in): fixed
  (`edit_ops::calls_closed`; test edits::rename_in_a_machining_document_leaves_call_heads);
  `FamiliesRename.renSt` is the fixed op.
* The same cause, for the other ops on a machining document: DeleteFeature and MoveFeature of
  `drill = drill(..)` were refused (`StillRead`) because a later `d2 = drill(..)` only calls the
  built-in, AddFeature of such a call above the binding likewise, and Suppress took `d2` out with
  it: fixed (`edit::mentions_in`; test edits::a_call_head_in_a_machining_document_is_not_a_read).
* robot.rs read `hull` and `none` before bindings without refusing a binding of either (accepted,
  never read): fixed by URDF (`ReservedName`, test urdf_robot::a_reserved_name_is_not_bound).
* A machining document passes over a bare call statement silently (`facing(op1, ..)` with no
  name does nothing, reports nothing): reported to CAM-CORE / CAM-TURN.
* A `fn`'s name and params may take a built-in's name (no `ShadowsBuiltin`): harmless, they are
  read before built-ins, so no family extension changes them.

## Pending

* Workholding / tool-library built-ins (WORKHOLDING, MACHINE-LIB): rows when they reach a
  document.

## Language limits (PR #13)

`Bounds.lean`: the magnitude bound where values enter the kernel (`builtins.rs`
`within_bounds`, `eval::MAX_MAGNITUDE`/`MAX_ANGLE`) is a check on the finished model's numbers
(`runK`), not an observation, so P2, P4 and P5 hold for it as for `run` (`runK_of_run`,
`runK_mono`, `runK_unique`, `runK_error_stable`, `setParam_bounded`, `setParam_bounded_ids`); the
other limits are fuel or `range`; nesting is counted at calls and the evaluator's recursion is at
most `maxDepth + nestBudget` = 600 levels (`stack_depth_le`).

## Also proven (not per item)

P1 to P6 and the core calculus (`Sim`, `Errors`, `Weak`, `Fv`, `Scope`, `Scoped*`, `Nested*`),
memoisation (`Memo`, `OpMemo`), modules and `use` (`Modules`), file reads (`Fence`), loops
(`Loops`), kernel names from ids (`HistoryId`), fit sizes (`Fit`), comparisons (`Compare`), and
the counterexamples under `Counter/` that show each hypothesis is needed.
