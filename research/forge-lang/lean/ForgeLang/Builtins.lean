import ForgeLang.Core
import ForgeLang.LL1.Impl

/-!
# Built-in names: adding some changes no document that binds none of them

A name read in a document means its binding in scope, else the built-in of that name, else nothing
(the error `Unknown`). A binding may not take a built-in's name: that is the refusal
`ShadowsBuiltin` (forge-lang's `NAMES` table, `builtins.rs`). So adding built-ins can do exactly
two things to an existing document:

* refuse it, where it binds one of the new names (`shadows_append`: that is the only way);
* give a meaning to a name it reads but never bound, which before was `Unknown`
  (`resolve_append_new`): no document that ran reads such a name.

`extend_conservative` puts these together: for a document binding none of the added names, the
refusals are the same, and every name it binds or reads that is not new resolves as before.
`weld_conservative` is that theorem for the weldment built-ins (`members`, `end_trim`, `end_cap`,
`corner_plate`); no part in the repository binds any of them (forge-lang's test
`weld::builtins_bound_nowhere` checks every `.fg` file), so every part means what it meant.

The new names change no grammar: built-ins are identifiers called like any other, and
`weld_not_keywords` checks that none of them, nor a member's anchors (`start`, `end`, `top`,
`bottom`, `left`, `right`, read after a dot), is one of the lexer's keywords (`Impl.keywords`, the
list the proven LL(1) parser is built on), so every one lexes as an identifier.
-/

namespace ForgeLang
namespace Builtins

/-- What a name means where it is read. -/
inductive Meaning (α : Type) where
  | bound : α → Meaning α
  | builtin : Name → Meaning α
  | unknown : Meaning α
  deriving DecidableEq

/-- A name's meaning: its binding first, then a built-in of the list `B`. -/
def resolve {α : Type} (B : List Name) (ρ : List (Name × α)) (x : Name) : Meaning α :=
  match ρ.lookup x with
  | some v => .bound v
  | none => if x ∈ B then .builtin x else .unknown

/-- The `ShadowsBuiltin` refusals of a document's bound names: each one in the list. -/
def shadows (B : List Name) (binds : List Name) : List Name :=
  binds.filter fun x => decide (x ∈ B)

/-- A document is accepted (as far as names go) when none of its bindings takes a built-in's name. -/
def Accepted (B : List Name) (binds : List Name) : Prop := shadows B binds = []

/-- Adding names refuses no more bindings than before, so long as none binds an added name. -/
theorem shadows_append (B N binds : List Name) (h : ∀ x ∈ binds, x ∉ N) :
    shadows (B ++ N) binds = shadows B binds := by
  unfold shadows
  apply List.filter_congr
  intro x hx
  have := h x hx
  simp [List.mem_append, this]

theorem accepted_append (B N binds : List Name) (h : ∀ x ∈ binds, x ∉ N) :
    Accepted (B ++ N) binds ↔ Accepted B binds := by
  unfold Accepted; rw [shadows_append B N binds h]

/-- And a binding of an added name is refused: the only new refusal. -/
theorem shadows_new (B N binds : List Name) (x : Name) (hb : x ∈ binds) (hn : x ∈ N) :
    x ∈ shadows (B ++ N) binds := by
  unfold shadows
  simp [List.mem_filter, hb, hn]

/-- A bound name means its binding, whatever the built-ins. -/
theorem resolve_bound {α : Type} (B : List Name) (ρ : List (Name × α)) (x : Name) (v : α)
    (h : ρ.lookup x = some v) : resolve B ρ x = .bound v := by
  simp [resolve, h]

/-- Adding names leaves the meaning of every name that is not one of them. -/
theorem resolve_append_off {α : Type} (B N : List Name) (ρ : List (Name × α)) (x : Name)
    (h : x ∉ N) : resolve (B ++ N) ρ x = resolve B ρ x := by
  unfold resolve
  cases ρ.lookup x with
  | some v => rfl
  | none => simp [List.mem_append, h]

/-- An added name that a document reads unbound meant nothing before (such a document did not
run); now it is the built-in. -/
theorem resolve_append_new {α : Type} (B N : List Name) (ρ : List (Name × α)) (x : Name)
    (hN : x ∈ N) (hn : x ∉ B) (hu : ρ.lookup x = none) :
    resolve B ρ x = .unknown ∧ resolve (B ++ N) ρ x = .builtin x := by
  simp [resolve, hu, hn, hN]

/-- Every binding resolves as before: a name bound in the environment means its binding under
either list. -/
theorem bindings_resolve_as_before {α : Type} (B N : List Name) (ρ : List (Name × α)) (x : Name)
    (v : α) (h : ρ.lookup x = some v) : resolve (B ++ N) ρ x = resolve B ρ x := by
  rw [resolve_bound _ _ _ _ h, resolve_bound _ _ _ _ h]

/-- The whole of it: for a document whose bindings avoid the added names, the same refusals, every
binding resolving to itself, and every read name that is not new meaning what it meant. -/
theorem extend_conservative {α : Type} (B N : List Name) (binds : List Name) (ρ : List (Name × α))
    (h : ∀ x ∈ binds, x ∉ N) :
    shadows (B ++ N) binds = shadows B binds ∧
      (∀ x v, ρ.lookup x = some v → resolve (B ++ N) ρ x = resolve B ρ x) ∧
      (∀ x, x ∉ N → resolve (B ++ N) ρ x = resolve B ρ x) :=
  ⟨shadows_append B N binds h, fun x v hv => bindings_resolve_as_before B N ρ x v hv,
    fun x hx => resolve_append_off B N ρ x hx⟩

/-- The weldment built-ins. -/
def weld : List Name := ["members", "end_trim", "end_cap", "corner_plate"]

/-- A member's anchors, read after a dot (`top[0].top`). -/
def memberAnchors : List Name := ["start", "end", "top", "bottom", "left", "right"]

/-- `extend_conservative` for the weldment built-ins added to any list of built-ins. -/
theorem weld_conservative {α : Type} (B : List Name) (binds : List Name) (ρ : List (Name × α))
    (h : ∀ x ∈ binds, x ∉ weld) :
    shadows (B ++ weld) binds = shadows B binds ∧
      (∀ x v, ρ.lookup x = some v → resolve (B ++ weld) ρ x = resolve B ρ x) ∧
      (∀ x, x ∉ weld → resolve (B ++ weld) ρ x = resolve B ρ x) :=
  extend_conservative B weld binds ρ h

/-- A name as the lexer's keyword lists spell it: its characters' codes. -/
def codes (s : Name) : List Nat := s.toList.map Char.toNat

/-- None of the new names is a keyword: each lexes as an identifier, so the grammar and the proven
parser are unchanged. -/
theorem weld_not_keywords : ∀ n ∈ weld ++ memberAnchors, codes n ∉ LL1.Impl.keywords := by
  decide

end Builtins
end ForgeLang
