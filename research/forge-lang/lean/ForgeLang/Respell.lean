import ForgeLang.Struct

/-!
# Respell (`interface/forge-lang/src/edit_ops.rs`, `apply_respell`)

`Respell {span, to}` writes `to` over the bytes at `span`: an `UnknownName` error's fix, the read
of a misspelt name. `apply_respell` checks only that the bytes at `span` spell an identifier and
that `to` is one and not a keyword.

* `respell_read_binders`: respelling reads changes no binder, so no feature id that comes from a
  name (P5), and no statement but the one respelt.
* `respell_text`: on the text the op is one splice: every byte outside the span is kept.

| Assumption | Counterexample without it |
|---|---|
| The span is a read, not a binder | `respell_binder_renames`: respelling `base` in `base = ..` renames the feature: its id changes from `base` to `plate`, and the op is not refused. Checked in fg (`Respell` at a binder's span, `ok: true`). Rename is the op for binders |
| The span is a whole token | `respell_subtoken`: a span inside `base` (`as`) respelt `x` gives `bxe`, a third name. Checked in fg: `bxe = extrude(..)` |
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

end ForgeLang
