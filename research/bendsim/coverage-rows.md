# COVERAGE.md rows for the bending document (for research/forge-lang/lean/COVERAGE.md)

COVERAGE.md lives on claude/lang-proofs (not on main yet); these rows go in with the merge of
either branch (whichever lands second adds them), as the `machining`/`cam` rows are written.

Headers:

| Kind | Item | Names | Rust site | Lean |
|---|---|---|---|---|
| header | `bending` | | forge-bend-lang `is_bending` (first statement the bare name) | GAP until LANG-PROOFS: as `LL1.Documents.milling_doc_accepted` (headers that are names add no syntax: the grammar is unchanged); semantics as `Families.run_ext_accepted` (closed namespace) |

Built-in families:

| Kind | Item | Names | Rust site | Lean |
|---|---|---|---|---|
| builtins | `bend` | `part`, `press`, `tooling`, `station`, `bend`, `setup_sheet`, `program` | `interface/forge-bend-lang/src/lib.rs` `NAMES` (the `read` match arms); closed namespace | GAP until LANG-PROOFS: `Families.run_ext_accepted`, `Families.stmt_new_refused`, `Families.families_not_keywords` as for `cam` (no name here is a keyword) |

Arguments:

| Kind | Item | Names | Rust site | Lean |
|---|---|---|---|---|
| args | `bend` | `sheet` (part), `punch`, `die`, `y`, `length`, `punch_length`, `die_length` (station), `at`, `rotate` (bend), `format` (program) | `interface/forge-bend-lang/src/lib.rs` `known(..)` per built-in: an argument not listed is refused | `FamiliesArgs.checkArgs_ok_iff`, `FamiliesArgs.misspelled_refused` (the list exists: every call is checked) |

Edit ops: none new (MoveFeature, SetArg, AddFeature, DeleteFeature, ReplaceFeature, Undo/Redo
on the document's statements).
