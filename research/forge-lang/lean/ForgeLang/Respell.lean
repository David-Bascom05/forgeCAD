import ForgeLang.Struct

/-!
# Respell (`interface/forge-lang/src/edit_ops.rs`, `apply_respell`)

`Respell {span, to}` writes `to` over the bytes at `span`: an `UnknownName` error's fix, the read
of a misspelt name. `apply_respell` takes the span only when it is exactly a read of a name, one
whole identifier token (`edit::reads_at`: a `Name` node of the parsed text); otherwise it is the
typed error `NotARead` (`respellChecked`).

* `respell_read_binders`: respelling reads changes no binder, so no feature id that comes from a
  name (P5), and no statement but the one respelt.
* `respell_text`: on the text the op is one splice: every byte outside the span is kept.
* `respellChecked_read`: what the op accepts is a read token's span exactly.
* `respell_refuses_binder`: a binder's span is refused (Rename is the op for binders), and so is
  every span `respell_refuses_inside`: one that starts inside a token, or at its start and ends
  elsewhere, in a lexed text (tokens nonempty and apart).
* `respell_binder_refused`, `respell_subtoken_refused`: the two counterexamples of the unchecked
  op, `respell_binder_renames` (`base` respelt `plate`, a feature id changed) and
  `respell_subtoken` (`as` in `base` respelt `x`, giving `bxe`), refused. Both were found in fg
  (`ok: true`) before the check.
-/

namespace ForgeLang

/-- A statement with every read of `x` in its own expressions respelt `y` (nested blocks kept). -/
def Expr.respell (x y : Name) : Expr → Expr
  | .var z => .var (if z = x then y else z)
  | .add a b => .add (a.respell x y) (b.respell x y)
  | .mul a b => .mul (a.respell x y) (b.respell x y)
  | .bin o a b => .bin o (a.respell x y) (b.respell x y)
  | .eq a b => .eq (a.respell x y) (b.respell x y)
  | .lt a b => .lt (a.respell x y) (b.respell x y)
  | .ite c a b => .ite (c.respell x y) (a.respell x y) (b.respell x y)
  | .call f a => .call f (a.respell x y)
  | e => e

def Stmt.respell (x y : Name) : Stmt → Stmt
  | .let_ z e => .let_ z (e.respell x y)
  | .set z e => .set z (e.respell x y)
  | .feat z e => .feat z (e.respell x y)
  | .forN z i c b => .forN z i (c.respell x y) b
  | .forK z i kvs b => .forK z i kvs b
  | .ifs c a b => .ifs (c.respell x y) a b
  | .use z p a => .use z p (a.respell x y)
  | .occ z c args => .occ z c args

theorem Stmt.respell_binder (x y : Name) (s : Stmt) : (s.respell x y).binder = s.binder := by
  cases s <;> rfl

/-- Statement `i` changed by `f`. -/
def modAt (f : Stmt → Stmt) : Nat → List Stmt → List Stmt
  | _, [] => []
  | 0, s :: ss => f s :: ss
  | i + 1, s :: ss => s :: modAt f i ss

/-- Respelling reads in statement `i` keeps every binder, so every name id. -/
theorem respell_read_binders (x y : Name) : ∀ (i : Nat) (body : List Stmt),
    binders (modAt (Stmt.respell x y) i body) = binders body
  | 0, [] => rfl
  | _ + 1, [] => rfl
  | 0, s :: ss => by
    simp only [modAt, binders, List.filterMap_cons, Stmt.respell_binder]
  | i + 1, s :: ss => by
    have := respell_read_binders x y i ss
    simp only [binders] at this
    simp only [modAt, binders, List.filterMap_cons, this]

/-- The text op: the bytes before and after the span are kept as they are. -/
def respellText (src : List Char) (lo hi : Nat) (to : List Char) : List Char :=
  src.take lo ++ to ++ src.drop hi

theorem respell_text (src : List Char) (lo hi : Nat) (to : List Char) (h : lo ≤ hi) (hh : hi ≤ src.length) :
    (respellText src lo hi to).take lo = src.take lo ∧
    (respellText src lo hi to).drop (lo + to.length) = src.drop hi := by
  have : (src.take lo).length = lo := by simp; omega
  constructor
  · simp [respellText, this]
  · rw [respellText, show lo + to.length = (src.take lo ++ to).length by simp [this]]
    exact List.drop_left

/-- A binder respelt: a different feature id, no error. -/
theorem respell_binder_renames :
    run 4 { body := [.feat "base" (.lit 0)], σ := fun _ => 6 } = .ok ([([.nm "base"], .site 0)], []) ∧
    run 4 { body := [.feat "plate" (.lit 0)], σ := fun _ => 6 } = .ok ([([.nm "plate"], .site 0)], []) :=
  ⟨rfl, rfl⟩

/-- A span inside a token: `base` with `as` respelt `x` is `bxe`. -/
theorem respell_subtoken : respellText "base".toList 1 3 "x".toList = "bxe".toList := by decide

/-! ## The check -/

inductive SpanKind where
  | read | binder | other
  deriving DecidableEq

/-- A token of the text: its span and what it is (a read of a name, a binder, anything else). -/
structure SpanTok where
  lo : Nat
  hi : Nat
  kind : SpanKind

/-- `apply_respell`: the splice, when the span is a read token's exactly; refused otherwise. -/
def respellChecked (toks : List SpanTok) (src : List Char) (lo hi : Nat) (to : List Char) : Option (List Char) :=
  if toks.any (fun t => t.lo == lo && t.hi == hi && t.kind == .read) then some (respellText src lo hi to)
  else none

/-- What a lexer gives: tokens nonempty, no two overlapping. -/
def Lexed (toks : List SpanTok) : Prop :=
  (∀ t ∈ toks, t.lo < t.hi) ∧ ∀ t ∈ toks, ∀ u ∈ toks, t ≠ u → t.hi ≤ u.lo ∨ u.hi ≤ t.lo

theorem respellChecked_read {toks src lo hi to out} (h : respellChecked toks src lo hi to = some out) :
    out = respellText src lo hi to ∧ ∃ t ∈ toks, t.lo = lo ∧ t.hi = hi ∧ t.kind = .read := by
  unfold respellChecked at h
  split at h
  · rename_i ha
    obtain ⟨t, ht, e⟩ := List.any_eq_true.1 ha
    simp only [Bool.and_eq_true, beq_iff_eq] at e
    exact ⟨(Option.some.inj h).symm, t, ht, e.1.1, e.1.2, e.2⟩
  · cases h

/-- A span inside a token, or at its start and ending elsewhere, is no token's: refused. -/
theorem respell_refuses_inside {toks src lo hi to} (hl : Lexed toks) {t : SpanTok} (ht : t ∈ toks)
    (hin : t.lo ≤ lo ∧ lo < t.hi) (hne : ¬ (lo = t.lo ∧ hi = t.hi)) :
    respellChecked toks src lo hi to = none := by
  cases h : respellChecked toks src lo hi to with
  | none => rfl
  | some out =>
    obtain ⟨-, u, hu, e1, e2, -⟩ := respellChecked_read h
    by_cases tu : t = u
    · subst tu; exact absurd ⟨e1.symm, e2.symm⟩ hne
    · have := hl.1 u hu
      rcases hl.2 t ht u hu tu with h1 | h1 <;> omega

/-- A binder's span is refused: no read token has it. -/
theorem respell_refuses_binder {toks src to} (hl : Lexed toks) {t : SpanTok} (ht : t ∈ toks)
    (hb : t.kind = .binder) : respellChecked toks src t.lo t.hi to = none := by
  cases h : respellChecked toks src t.lo t.hi to with
  | none => rfl
  | some out =>
    obtain ⟨-, u, hu, e1, e2, ek⟩ := respellChecked_read h
    by_cases tu : t = u
    · subst tu; rw [hb] at ek; cases ek
    · have := hl.1 u hu
      have := hl.1 t ht
      rcases hl.2 t ht u hu tu with h1 | h1 <;> omega

/-- `base = 0`: `base` a binder at 0..4, `=` at 5..6, `0` at 7..8. -/
def baseToks : List SpanTok := [⟨0, 4, .binder⟩, ⟨5, 6, .other⟩, ⟨7, 8, .other⟩]

theorem baseToks_lexed : Lexed baseToks := by
  refine ⟨by decide, ?_⟩
  intro t ht u hu tu
  simp only [baseToks, List.mem_cons, List.not_mem_nil, or_false] at ht hu
  rcases ht with rfl | rfl | rfl <;> rcases hu with rfl | rfl | rfl <;> first | exact absurd rfl tu | simp

/-- `respell_binder_renames`, checked: refused. -/
theorem respell_binder_refused : respellChecked baseToks "base = 0".toList 0 4 "plate".toList = none :=
  respell_refuses_binder (t := ⟨0, 4, .binder⟩) baseToks_lexed (by simp [baseToks]) rfl

/-- `respell_subtoken`, checked: refused. -/
theorem respell_subtoken_refused : respellChecked baseToks "base = 0".toList 1 3 "x".toList = none :=
  respell_refuses_inside (t := ⟨0, 4, .binder⟩) baseToks_lexed (by simp [baseToks]) (by decide) (by decide)

end ForgeLang
