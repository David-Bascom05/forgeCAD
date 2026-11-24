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
* **Closed** (`machining` documents: forge-cam-lang, forge-turn-lang): a call's head is looked up
  among the built-ins only, a read among the bindings only; no refusal for binding a built-in's
  name (`drill = drill(..)` is fine). `Families.run_ext_accepted`: adding match arms changes no
  accepted document; `Families.stmt_new_refused`: a document calling a new name was refused
  before. No hypothesis on the documents is needed.
* **Own** (assembly joints/mates, harness blocks, drawings): resolved by their own code; GAP.

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
| builtins | `assembly` | `part`, `subassembly`, `pose`, `fastened`, `revolute`, `slider`, `cylindrical`, `pin_slot`, `planar`, `ball`, `coincident`, `on_plane`, `on_line`, `mate`, `flush`, `concentric`, `insert`, `parallel`, `perpendicular`, `angle`, `distance`, `lock`, `gear`, `rack_pinion`, `screw`, `linear`, `fit`, `fit_rule`, `kind`, `std`, `ground` | `asm.rs` `NAMES`; own | GAP: resolved by asm.rs (`asm::call` before a part's built-ins in an assembly, the part's where a sketch constraint shares the name, `ball` by its arguments); this namespace is not modelled. Lexing: lang_coverage::no_builtin_is_a_keyword |
| builtins | `assembly-part` | `anchor`, `anchor_kind` | `asm.rs` `PART_NAMES`; own | GAP: `asm::call` (`anchor`, `anchor_kind`, in both kinds of document); not modelled |
| builtins | `assembly-kinds` | `external_thread`, `internal_thread`, `shaft`, `bore` | `asm.rs` `KINDS`; own | GAP: `asm::constant`, read after the part's constants and before its built-ins; not modelled |
| builtins | `harness` | `wire_type`, `cable_type`, `connector`, `node`, `segment`, `wire`, `cable`, `twist` | `harness.rs` `BUILTINS`; own | GAP: known only inside a `harness` block (harness.rs); the block's scope is not modelled |
| builtins | `drawing` | `part`, `assembly`, `sheet`, `base_view`, `projected`, `section`, `detail`, `flat_view`, `dim`, `angle`, `radius`, `diameter`, `hole_note`, `ordinate`, `baseline`, `chain`, `center_mark`, `center_marks`, `centerlines`, `centerline`, `leader`, `note`, `balloon`, `auto_balloons`, `fcf`, `datum`, `datum_target`, `surface`, `weld`, `edge`, `parts_list`, `e`, `f` | `drawing/doc.rs` `FUNCTIONS`; own | GAP: the drawing document's own evaluator (drawing/doc.rs); not modelled beyond its grammar (`LL1.Unified.accept_drawing_header`) |
| builtins | `cam` | `part`, `machine`, `tool`, `setup`, `facing`, `contour`, `pocket`, `clear`, `drill`, `engrave`, `slot`, `thread_mill`, `edge_break`, `soft_jaw`, `program` | `interface/forge-cam-lang/src/lib.rs` `Cam::call` arms; closed namespace | `Families.cam_conservative`, `Families.run_ext_accepted`, `Families.run_ext_avoid`, `Families.stmt_new_refused`, `Families.families_not_keywords`; `LL1.Documents.milling_doc_accepted` |
| builtins | `cam-expr` | `box`, `vise`, `cylinder`, `tube`, `solid` | `Cam::eval` (`fname == ..`); closed namespace | `Families.cam_conservative`, `Families.run_ext_accepted`, `Families.run_ext_avoid`, `Families.stmt_new_refused`, `Families.families_not_keywords` |
| builtins | `turn` | `part`, `lathe_tool`, `lathe_setup`, `turn_face`, `turn_rough`, `bore_rough`, `turn_finish`, `turn_groove`, `turn_thread`, `part_off`, `turn_drill`, `lathe_program` | `interface/forge-turn-lang/src/lib.rs` `Turn::call` arms; closed namespace | `Families.turn_conservative`, `Families.run_ext_accepted`, `Families.run_ext_avoid`, `Families.stmt_new_refused`, `Families.families_not_keywords`; `LL1.Documents.turning_doc_accepted` |
| builtins | `turn-expr` | `chuck` | `Turn::eval` (`match fname.as_str()`); closed namespace | `Families.turn_conservative`, `Families.run_ext_accepted`, `Families.run_ext_avoid`, `Families.stmt_new_refused`, `Families.families_not_keywords` |

## Document headers and keywords

The grammar (`interface/forge-lang/grammar.ll1`) is proven LL(1) as generated into
`LL1/Impl.lean` (`the_lean_grammar_is_current` keeps it current); the Rust parser is tied to the
model answer for answer by `tests/parse_diff.rs` (`FG_LEAN=1`, in the lean workflow) on random
token strings, random programs and edits, and every `.fg` in the repository, the machining
documents in `examples/machined/` included. Headers that are names add no syntax.

| Kind | Item | Names | Rust site | Lean |
|---|---|---|---|---|
| header | `machining` | | forge-cam-lang `is_machining`, forge-turn-lang `is_turning` (first statement the bare name; turning when a statement calls `lathe_setup`) | `LL1.Documents.milling_doc_accepted`, `LL1.Documents.turning_doc_accepted`, `LL1.Documents.header_glued_refused`, `Families.families_not_keywords`; semantics `Families.run_ext_accepted` |
| header | `drawing` | | forge-lang `drawing::doc::declared` (and the `drawing` keyword's statement) | `LL1.Unified.accept_drawing_header`, `LL1.Unified.accept_drawing_empty` (on the unified prototype grammar; on `Impl` by `LL1.Impl.grammar_ll1` and parse_diff). Semantics: GAP (drawing evaluator) |
| header | `assembly` | | forge-render `is_assembly` (optional first line; an assembly is a file with components or joints) | `LL1.Documents.assembly_header_accepted`, `Families.families_not_keywords`. Semantics: GAP (assembly namespace, see `assembly`) |
| keywords | `syntax::KEYWORDS` | `shared`, `param`, `fn`, `for`, `in`, `if`, `else`, `return`, `expect`, `and`, `or`, `not`, `true`, `false`, `sketch`, `path`, `at`, `component`, `drawing`, `harness` | `interface/forge-lang/src/syntax.rs` (`RESERVED`: `at`, a keyword that texts used as a name) | `LL1.Impl.grammar_ll1`, `LL1.Impl.impl_unambiguous`, `LL1.Impl.source_to_tree`, `LL1.Impl.impl_roundtrip`, `LL1.Driver.rd_eq_run`, `LL1.Lex.word_maximal`, `LL1.Lex.lex_lossless`, `LL1.LexUnits.in_keyword_after_number` |

## Edit ops

The theorems are about the core model of a part document (`Core.lean`). The text-level ones
(`Print`, `Respell.respell_text`, `Suppress.suppress_text_closure`, `LL1.ReparseModel`) hold of
any document. **Machining documents** (CAM-UI and the lathe editor send SetArg, AddFeature,
DeleteFeature, MoveFeature, Suppress, Rename to them): GAP for every op's semantics. Their
built-ins carry state (an op registers with its setup; `program` posts every op of a setup), so
no edit of one statement is local to it, and no locality theorem can hold as stated for parts;
what is proven is that the run is a function of the text (`Families.run`), the names facts
above, and Rename (`FamiliesRename.rename_run`).

| Kind | Item | Names | Rust site | Lean |
|---|---|---|---|---|
| op | `SetParam` | | `edit.rs` `apply`; text: `syntax::reparse_number` | `Edits.setParam_roundtrip`, `Edits.setParam_ids`, `Edits.sole_driver`, `Print.setLit_text`, `LL1.Lex.num_literal`, `LL1.ReparseModel.reparse_number_correct`, `LL1.Reparse.reparse_parse`; observed literal: a re-run (`Loops.count_prefix`, `Loops.keyed_insert`) |
| op | `AddLocalParam` | | `edit.rs` (`param F.name = v`) | GAP: no theorem for adding a local param. Closest: `Scoped.addParam_occ` (a component param with a literal default, unread: every occurrence unchanged) and `Complete.add_complete`; the local-param statement itself is not in the core model |
| op | `MakeShared` | | `edit.rs` | `Shared.makeShared_complete`, `Shared.makeShared_sound`, `Shared.makeShared_nums` |
| op | `SplitInstance` | | `edit.rs` | `Split.split_doc`, `Sites.split_independent`, `Sites.split_edit_one`; counterexample `Counter.ValueEdit.split_edit_one` |
| op | `AddFeature` | | `edit.rs` | `Complete.add_complete`, `Scoped.scopedAdd_ok`, `Print.body_text_insert`, `Scoped.scoped_text_insert`; sound half `Struct.execS_append` |
| op | `DeleteFeature` | | `edit.rs` | `Struct.delete_sound`, `Complete.delete_complete`, `Scoped.scopedDelete_ok`; Counter/Rebind.lean shows single assignment is needed |
| op | `ReplaceFeature` | | `edit.rs` | `ScopedMore.replace_complete`, `ScopedMore.scopedReplace_ok` (the statement keeps its name and nothing after it reads it: complete, every other fragment unchanged); a statement that is read: `EditClosure.edit_closure` (both documents evaluate: every fragment before it, and after it outside the closure of its name, unchanged; `EditClosure.closure_runs`). GAP: that the edited document evaluates when the closure's readers accept the new value (a re-run); in a component's block (scoped) |
| op | `MoveFeature` | | `edit.rs` | `Struct.move_sound`, `Complete.move_complete`, `ScopedMore.move_down`, `ScopedMore.move_up`, `ScopedMore.scopedMove_ok`, `Print.body_text_swap` |
| op | `LiftLiteral` | | `edit.rs` | GAP: a literal replaced by a new param holding its value; the expected theorem is `Shared.inlineExpr_sound` read backwards (same numbers, the trace's site renamed, `Ren`/`Subst`), not stated |
| op | `SetArg` | | `edit_ops.rs` `apply_set_arg` (a call's argument, or `arg: text` added), `apply_component_arg` | a literal argument or component default set to a literal: SetParam on its site (`Edits.setParam_roundtrip`, as Scoped.lean notes); any text: a ReplaceFeature keeping the name, so `ScopedMore.replace_complete` (nothing reads it) and `EditClosure.edit_closure` (it is read: everything outside the closure of its name unchanged). GAP: that the edited document evaluates (a re-run of the closure); the text splice parsing back to the one-argument change (checked by reparsing at run time, not proven); machining documents (see above) |
| op | `Rename` | | `edit_ops.rs` `precheck_rename`, `apply_rename`, `ast_rename` (token-level: the binding and its reads, not members, labels or params' names; in a machining document, `calls_closed`, not a call's head) | machining documents: `FamiliesRename.rename_run`, `FamiliesRename.rename_accepted` (`b` fresh, neither name a param's: the same state after every statement, the same statements failing, an accepted document stays accepted). GAP: part documents (ids follow names, P5: the expected theorem is the run with `nm a` renamed `nm b` in every id, not stated); `Respell.respell_refuses_binder` and `Respell.respell_read_binders` cover only Respell's side |
| op | `SetProperty` | | `props.rs` | GAP: document properties (metadata) are not in the model; that they change no feature is unproven |
| op | `Suppress` | | `edit_ops.rs` `apply_suppress` | `Suppress.suppress_complete`, `Suppress.suppress_stmts`, `Suppress.suppress_text_closure`, `Suppress.suppress_live`; ids `HistoryId.historyId_stable` |
| op | `Unsuppress` | | `edit_ops.rs` | `Suppress.unsuppress_suppress` |
| op | `Respell` | | `edit_ops.rs` `apply_respell` | `Respell.respell_read_binders`, `Respell.respell_text`, `Respell.respellChecked_read`, `Respell.respell_refuses_binder` |
| op | `Undo` | | `undo.rs`; CLI `{"op":"Undo"}` (forge-cli `fg.rs`) | `Undo.undo_write`, `Undo.redo_undo`, `Undo.step_refused`, `Undo.history_keep`; `Undo.unchecked_loses_edit` (the check is needed) |
| op | `Redo` | | `undo.rs`; CLI | `Undo.undo_redo`, `Undo.step_refused`, `Undo.history_keep` |
| op | `MakeComponent` | | `refactor.rs` | `Refactor.makeComponent`; Counter/Refactor.lean |
| op | `ExtractComponent` | | `refactor.rs` | `Refactor.extract_occ`, `Modules.link_inert`; Counter/Refactor.lean |
| op | `InlineComponent` | | `refactor.rs` | `Refactor.extract_occ` (the same equation read the other way) |

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
* A `fn`'s name and params may take a built-in's name (no `ShadowsBuiltin`): harmless, they are
  read before built-ins, so no family extension changes them.

## Pending

* `robot` document (URDF worker; research/export/urdf-editor.md on branch claude/urdf-export):
  a header word and its built-ins (`source`, `drop`, ..); rows when it lands.
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
