import ForgeLang.LL1.Impl

/-!
# Sheets and thin walls: three argument names, and no new built-in

`extrude(.., surface: true)`, `revolve(.., surface: true)` and `extrude(.., thin: t, side: "out")`
(research/surfaces/design.md) add no built-in function, no keyword and no grammar production:
`surface`, `thin` and `side` are argument names, read as `IDENT ':' expr` inside a call's
parentheses like every other argument (`side` was one already, on `rib` and `thicken`). The side's
values ("out", "in", "both") are strings. `"in"` is quoted, so the keyword `in` is never lexed from
it.

So the built-in list (`Builtins`, where it exists, and forge-lang's `NAMES`) is unchanged: no
document's bindings are refused that were not before, and every name resolves as before. Of that,
nothing here is left to prove. What could change the grammar is a new argument name that lexes as
a keyword. `sheet_args_not_keywords` checks that none does, so each lexes as an identifier, and the
proven LL(1) parser (`Impl.keywords`) parses every document as before.

forge-lang's evaluation of the new arguments (a `Sheet` feature, `NotWith` refusals) is the
kernel's, as for every feature. The language's scope, edits and undo do not look inside a call's
argument list by name.
-/

namespace ForgeLang
namespace SheetArgs

/-- The argument names sheets and thin walls add to `extrude`, `cut` and `revolve`. -/
def args : List String := ["surface", "thin", "side"]

/-- A name as the lexer's keyword lists spell it: its characters' codes. -/
def codes (s : String) : List Nat := s.toList.map Char.toNat

/-- None of them is a keyword: each lexes as an identifier, so the grammar and the proven parser
are unchanged. -/
theorem sheet_args_not_keywords : ∀ n ∈ args, codes n ∉ LL1.Impl.keywords := by
  decide

end SheetArgs
end ForgeLang
