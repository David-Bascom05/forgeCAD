/-!
# P6: an ambiguous concrete syntax gives one text two meanings

Infix minus with no precedence or associativity rule: `1 - 2 - 3` is two trees with two values.
Method calls without a binding rule: `a - b.move(5)` is `(a - b).move(5)` or `a - (b.move(5))`.
Each is a text with two models, so P1 fails at the level of text even though evaluation of each
tree is a function. Fixed by any unambiguous grammar: a precedence table, or LR(1)/LL(1) checked by a
generator, or a PEG. LR(1) is sufficient, not necessary (`ASSUMPTIONS.md`).
-/

namespace ForgeLang.Counter

inductive A where
  | n : Nat → A
  | sub : A → A → A
  | move : A → Nat → A

def flat : A → List String
  | .n k => [toString k]
  | .sub a b => flat a ++ ["-"] ++ flat b
  | .move a d => flat a ++ [".move(", toString d, ")"]

def ev : A → Int
  | .n k => k
  | .sub a b => ev a - ev b
  | .move a d => ev a + d

theorem minus_ambiguous :
    flat (.sub (.sub (.n 1) (.n 2)) (.n 3)) = flat (.sub (.n 1) (.sub (.n 2) (.n 3))) ∧
      ev (.sub (.sub (.n 1) (.n 2)) (.n 3)) = -4 ∧ ev (.sub (.n 1) (.sub (.n 2) (.n 3))) = 2 := by decide

theorem method_ambiguous :
    flat (.move (.sub (.n 9) (.n 4)) 5) = flat (.sub (.n 9) (.move (.n 4) 5)) ∧
      ev (.move (.sub (.n 9) (.n 4)) 5) = 10 ∧ ev (.sub (.n 9) (.move (.n 4) 5)) = 0 := by decide

end ForgeLang.Counter
