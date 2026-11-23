import ForgeLang.LL1.Impl

/-!
# The `in` unit beside the `in` keyword

`in` is a keyword (`for x in ..`, an assembly's `in OCC`) and, written right after a number with
no space, a length unit: inches (`0.063in`, 1.6002 mm). The lexer decides by the number: a unit is
the letters glued to a number (`numUnit`, maximal munch), so `5in` is always one number in inches,
and `5 in OCC`, with a space, is a number, the keyword and a name. On the parser's own keywords and
operators (`Impl.lexCfg`):

* `inch_literal`: `0.063in` lexes as one number token in inches;
* `inch_glued`: `5in OCC` is the number `5in`, a space and the name `OCC`, never the keyword;
* `in_keyword_after_number`: `5 in OCC` is a plain number, then the keyword `in`, then a name;
* `in_keyword_after_tuple`: `(10, 20) in OCC` keeps `in` the keyword after a bracket;
* `num_literal` (Lex.lean) covers `in` with the other units for every literal SetParam writes.
-/

namespace ForgeLang.LL1.LexUnits

open Lex

/-- A string's bytes (ASCII). -/
def b (s : String) : List Nat := s.toList.map Char.toNat

/-- The kinds of the pieces a text lexes to on the parser's configuration. -/
def kinds (s : String) : Except (Err × Nat × Nat) (List Kind) :=
  (lex Impl.lexCfg (b s)).map (·.map Piece.kind)

/-- The keyword index of `in` in the parser's list. -/
theorem in_is_keyword_4 : Impl.keywords[4]? = some (b "in") := by decide

theorem inch_literal : lex Impl.lexCfg (b "0.063in") = .ok [⟨.num .inch, b "0.063in"⟩] := by rfl

theorem inch_glued : kinds "5in OCC" = .ok [.num .inch, .space, .id] := by rfl

theorem in_keyword_after_number : kinds "5 in OCC" = .ok [.num .none, .space, .kw 4, .space, .id] := by
  rfl

theorem in_keyword_after_tuple :
    kinds "(10, 20) in OCC" =
      .ok [.op 13, .num .none, .op 19, .space, .num .none, .op 14, .space, .kw 4, .space, .id] := by
  rfl

/-- Any unit `in` literal SetParam writes is one number piece, whatever follows that cannot
continue a number (a space, an operator, the end). -/
theorem inch_num_literal (pv : Bool) (d : Nat) (l : Lit) (hl : l.Ok) (hu : l.unit = .inch) (r : Bytes)
    (hr : Stops r) (hpv : l.neg = true → pv = false) :
    ∃ c s, l.text ++ r = c :: s ∧ step Impl.lexCfg pv d c s = .ok (.num .inch, l.text, r, d) := by
  have := num_literal Impl.lexCfg pv d l hl r hr hpv
  rwa [hu] at this

end ForgeLang.LL1.LexUnits
