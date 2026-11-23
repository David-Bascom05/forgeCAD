import ForgeLang.LL1.Impl
import ForgeLang.LL1.ImplUnifiedDoc
import ForgeLang.LL1.Driver

/-!
# An edit of one number: the parse moves, it does not change

`syntax::reparse_number` (interface/forge-lang/src/syntax.rs) keeps the tree of the text before,
sets the edited number's value, and moves every span from the number's end on by the change in
length. It is right if the new text's parse is exactly that tree.

* The lexer (`Lex.reparse_lex`): the new text lexes to the same pieces before the number, the new
  number, trivia, and the same pieces after.
* So the parser's input, the terminals, is the same (`terminals_eq`). The LL(1) parse is a
  function of the terminals alone (`parse`, `Driver.rd` on `List Tok`), so it is the same: the
  same rules in the same order, accepted or refused alike (`Impl.reparse_parse`).
* Only the leaves' bytes and the offsets move (`reparse_tokens`): the tokens before the number
  stay where they were, the number stays where it starts and has the new text, and every token
  after it moves by `|L₂ T₂| − |L₁ T₁|`.

`Edit` is what `reparse_number` checks, on bytes:
* `L₁`, `L₂`: `lit` at the number's start in each text. A sign as it was, digits, maybe `.`
  digits, maybe `mm` or `deg`, and then no word byte or `.` (`Stops`).
* `T₁`, `T₂`: `trivia`, the changed bytes after each literal (`a[hi..old_end]`,
  `b[new_hi..new_end]`): spaces, tabs or CRs, then maybe `#` and anything but a line break
  (`TrivLine`).
* `S`: the bytes from `old_end` on, the same in both texts.
* `hash`: the condition the code as written does not check. Both trivia hold a `#` or neither
  does, or else the rest of `S`'s line is trivia. Without it, `"a = 5 #+ 2"` edited to
  `"a = 6 + 2"` is patched, but the new text parses to a sum.
-/

namespace ForgeLang.LL1.Reparse
open Lex

/-- What `reparse_number` accepts: the old text `P ++ L₁ ++ T₁ ++ S`, the new `P ++ L₂ ++ T₂ ++ S`. -/
structure Edit (T₁ T₂ S : Bytes) (L₁ L₂ : Lit) : Prop where
  ok₁ : L₁.Ok
  ok₂ : L₂.Ok
  neg : L₁.neg = L₂.neg
  stops : Stops (T₂ ++ S)
  triv₁ : TrivLine T₁
  triv₂ : TrivLine T₂
  hash : (HASH ∈ T₁ ↔ HASH ∈ T₂) ∨ ∃ V S₁, S = V ++ S₁ ∧ TrivLine V ∧ (S₁ = [] ∨ S₁.head? = some NL)

/-- The pieces with the byte offsets they start at, from `b`. -/
def placed : Nat → List Piece → List (Piece × Nat)
  | _, [] => []
  | b, p :: ps => (p, b) :: placed (b + p.text.length) ps

/-- The tokens, trivia left out, with their offsets: what the parser's spans are made of. -/
def tokens (b : Nat) (ps : List Piece) : List (Piece × Nat) :=
  (placed b ps).filter fun q => !isTrivia q.1.kind

theorem placed_append : ∀ (b : Nat) (xs ys : List Piece),
    placed b (xs ++ ys) = placed b xs ++ placed (b + ((xs.map Piece.text).flatten).length) ys
  | b, [], ys => by simp [placed]
  | b, x :: xs, ys => by
    simp only [List.cons_append, placed, placed_append (b + x.text.length) xs ys, List.map_cons,
      List.flatten_cons, List.length_append, Nat.add_assoc]

theorem placed_shift : ∀ (b k : Nat) (ps : List Piece),
    placed (b + k) ps = (placed b ps).map fun q => (q.1, q.2 + k)
  | _, _, [] => rfl
  | b, k, p :: ps => by
    simp only [placed, List.map_cons]
    rw [show b + k + p.text.length = (b + p.text.length) + k by omega, placed_shift]

theorem tokens_append (b : Nat) (xs ys : List Piece) :
    tokens b (xs ++ ys) = tokens b xs ++ tokens (b + ((xs.map Piece.text).flatten).length) ys := by
  simp [tokens, placed_append, List.filter_append]

theorem tokens_trivia : ∀ (b : Nat) (tr : List Piece), (∀ p ∈ tr, isTrivia p.kind = true) → tokens b tr = []
  | _, [], _ => rfl
  | b, p :: tr, h => by
    have hp := h p (by simp)
    have := tokens_trivia (b + p.text.length) tr (fun q hq => h q (by simp [hq]))
    simp only [tokens, placed, List.filter_cons, hp, Bool.not_true, Bool.false_eq_true, ite_false] at this ⊢
    exact this

theorem tokens_tok {p : Piece} (b : Nat) (ps : List Piece) (hp : isTrivia p.kind = false) :
    tokens b (p :: ps) = (p, b) :: tokens (b + p.text.length) ps := by
  simp [tokens, placed, hp]

/-- Moving where the count starts moves every token by as much. -/
theorem tokens_shift (b k : Nat) (ps : List Piece) :
    tokens (b + k) ps = (tokens b ps).map fun q => (q.1, q.2 + k) := by
  simp only [tokens, placed_shift, List.filter_map]
  rfl

/-- The tokens' terminals, for any map of kinds to terminals that drops trivia and does not look
at a number's unit (as `Impl.terminalOf` and `ImplUnifiedDoc.terminalOf`). -/
theorem filterMap_eq {term : Kind → Option Nat} (htriv : ∀ k, isTrivia k = true → term k = none)
    (hnum : ∀ u u', term (.num u) = term (.num u')) {pre tr₁ tr₂ rest : List Piece} {u₁ u₂ : Lex.Unit}
    {t₁ t₂ : Bytes} (h : ∀ p ∈ tr₁ ++ tr₂, isTrivia p.kind = true) :
    (pre ++ ⟨.num u₁, t₁⟩ :: (tr₁ ++ rest)).filterMap (fun p => term p.kind) =
      (pre ++ ⟨.num u₂, t₂⟩ :: (tr₂ ++ rest)).filterMap (fun p => term p.kind) := by
  have hz : ∀ tr : List Piece, (∀ p ∈ tr, isTrivia p.kind = true) → tr.filterMap (fun p => term p.kind) = [] :=
    fun tr ht => List.filterMap_eq_nil_iff.2 fun p hp => htriv _ (ht p hp)
  simp only [List.filterMap_append, List.filterMap_cons, hnum u₁ u₂,
    hz tr₁ (fun p hp => h p (List.mem_append_left _ hp)), hz tr₂ (fun p hp => h p (List.mem_append_right _ hp))]

/-- **The lexer's answer to an edit, as the parser sees it.** The new text lexes; its terminals
are the old ones (for every `term` as in `filterMap_eq`); its tokens are the old ones with the
number's piece replaced, those before it where they were, and those after it moved by the change
in length (`base₁ + |L₂ T₂| = base₂ + |L₁ T₁|`). -/
theorem reparse_tokens {C : Cfg} (hC : DigitFree C) {P T₁ T₂ S : Bytes} {L₁ L₂ : Lit}
    (he : Edit T₁ T₂ S L₁ L₂) {pre post : List Piece} {u₁ : Lex.Unit}
    (hlex : lex C (P ++ L₁.text ++ T₁ ++ S) = .ok (pre ++ ⟨.num u₁, L₁.text⟩ :: post))
    (hpre : (pre.map Piece.text).flatten = P) :
    ∃ ps', lex C (P ++ L₂.text ++ T₂ ++ S) = .ok ps' ∧
      (∀ term : Kind → Option Nat, (∀ k, isTrivia k = true → term k = none) →
        (∀ u u', term (.num u) = term (.num u')) →
        ps'.filterMap (fun p => term p.kind) = (pre ++ ⟨.num u₁, L₁.text⟩ :: post).filterMap (fun p => term p.kind)) ∧
      ∃ base₁ base₂ rest,
        tokens 0 (pre ++ ⟨.num u₁, L₁.text⟩ :: post) =
          tokens 0 pre ++ (⟨.num u₁, L₁.text⟩, P.length) :: tokens base₁ rest ∧
        tokens 0 ps' = tokens 0 pre ++ (⟨.num L₂.unit, L₂.text⟩, P.length) :: tokens base₂ rest ∧
        base₁ + (L₂.text ++ T₂).length = base₂ + (L₁.text ++ T₁).length := by
  obtain ⟨tr₁, tr₂, rest, S₀, hpost, hnew, htr, hf₁, hf₂⟩ :=
    reparse_lex hC he.ok₁ he.ok₂ he.neg he.stops he.triv₁ he.triv₂ he.hash hlex hpre
  subst hpost
  refine ⟨_, hnew, fun term ht hn => (filterMap_eq ht hn htr).symm, P.length + L₁.text.length + (T₁ ++ S₀).length,
    P.length + L₂.text.length + (T₂ ++ S₀).length, rest, ?_, ?_, ?_⟩
  · rw [tokens_append, hpre, Nat.zero_add, tokens_tok _ _ rfl, tokens_append, hf₁,
      tokens_trivia _ tr₁ (fun p hp => htr p (List.mem_append_left tr₂ hp)), List.nil_append]
  · rw [tokens_append, hpre, Nat.zero_add, tokens_tok _ _ rfl, tokens_append, hf₂,
      tokens_trivia _ tr₂ (fun p hp => htr p (List.mem_append_right tr₁ hp)), List.nil_append]
  · simp only [List.length_append]; omega

end ForgeLang.LL1.Reparse

namespace ForgeLang.LL1.Impl
open Lex Reparse

theorem lexCfg_digitFree : DigitFree lexCfg := by unfold DigitFree lexCfg; decide

theorem terminalOf_trivia : ∀ k, isTrivia k = true → terminalOf k = none := by
  intro k hk; cases k <;> simp_all [isTrivia, terminalOf]

/-- **`reparse_number` is the parse of the edit** (on the parser's grammar and lexer). Where the
edit is one that `Edit` describes, the new text lexes, its terminals are the old ones, so the
proven parser (`parse`) and the table driver (`Driver.rd`, which is `Driver.run`) answer exactly
as on the old text: the same rules in the same order. The tokens differ only in the number's
bytes and in the offsets after it (`reparse_tokens`). -/
theorem reparse_parse {P T₁ T₂ S : Lex.Bytes} {L₁ L₂ : Lit} (he : Edit T₁ T₂ S L₁ L₂)
    {pre post : List Piece} {u₁ : Lex.Unit}
    (hlex : lex lexCfg (P ++ L₁.text ++ T₁ ++ S) = .ok (pre ++ ⟨.num u₁, L₁.text⟩ :: post))
    (hpre : (pre.map Piece.text).flatten = P) :
    ∃ ps', lex lexCfg (P ++ L₂.text ++ T₂ ++ S) = .ok ps' ∧
      terminals ps' = terminals (pre ++ ⟨.num u₁, L₁.text⟩ :: post) ∧
      (∀ n, parse grammar certT n (.n grammar.start) (terminals ps') =
        parse grammar certT n (.n grammar.start) (terminals (pre ++ ⟨.num u₁, L₁.text⟩ :: post))) ∧
      (∀ n, Driver.rd grammar certT n (terminals ps') =
        Driver.rd grammar certT n (terminals (pre ++ ⟨.num u₁, L₁.text⟩ :: post))) := by
  obtain ⟨ps', hnew, hterm, _⟩ := reparse_tokens lexCfg_digitFree he hlex hpre
  have ht : terminals ps' = terminals (pre ++ ⟨.num u₁, L₁.text⟩ :: post) := by
    unfold terminals
    rw [hterm terminalOf terminalOf_trivia (fun _ _ => rfl)]
  exact ⟨ps', hnew, ht, fun n => by rw [ht], fun n => by rw [ht]⟩

end ForgeLang.LL1.Impl

namespace ForgeLang.LL1.ImplUnifiedDoc
open Lex Reparse

theorem lexCfg_digitFree : DigitFree lexCfg := by unfold DigitFree lexCfg; decide

theorem terminalOf_trivia : ∀ k, isTrivia k = true → terminalOf k = none := by
  intro k hk; cases k <;> simp_all [isTrivia, terminalOf]

/-- `Impl.reparse_parse` on the unified-doc snapshot's grammar. -/
theorem reparse_parse {P T₁ T₂ S : Lex.Bytes} {L₁ L₂ : Lit} (he : Edit T₁ T₂ S L₁ L₂)
    {pre post : List Piece} {u₁ : Lex.Unit}
    (hlex : lex lexCfg (P ++ L₁.text ++ T₁ ++ S) = .ok (pre ++ ⟨.num u₁, L₁.text⟩ :: post))
    (hpre : (pre.map Piece.text).flatten = P) :
    ∃ ps', lex lexCfg (P ++ L₂.text ++ T₂ ++ S) = .ok ps' ∧
      terminals ps' = terminals (pre ++ ⟨.num u₁, L₁.text⟩ :: post) ∧
      (∀ n, parse grammar certT n (.n grammar.start) (terminals ps') =
        parse grammar certT n (.n grammar.start) (terminals (pre ++ ⟨.num u₁, L₁.text⟩ :: post))) ∧
      (∀ n, Driver.rd grammar certT n (terminals ps') =
        Driver.rd grammar certT n (terminals (pre ++ ⟨.num u₁, L₁.text⟩ :: post))) := by
  obtain ⟨ps', hnew, hterm, _⟩ := reparse_tokens lexCfg_digitFree he hlex hpre
  have ht : terminals ps' = terminals (pre ++ ⟨.num u₁, L₁.text⟩ :: post) := by
    unfold terminals
    rw [hterm terminalOf terminalOf_trivia (fun _ _ => rfl)]
  exact ⟨ps', hnew, ht, fun n => by rw [ht], fun n => by rw [ht]⟩

end ForgeLang.LL1.ImplUnifiedDoc
