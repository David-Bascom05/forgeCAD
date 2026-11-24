import ForgeLang.Families

/-!
# A built-in's arguments: a misspelled one is refused

A call's arguments are its keywords and its positionals (`#0`, `#1`, ..). `named` checks them
against the built-in's own list (forge-cam-lang's per-built-in argument lists, which CAM-UI's
dialog schema is checked against): the first one the list lacks is the typed error
`unknownArg f k`, before the built-in runs. Before such lists, a misspelled keyword
(`stepovr:`) was read by nothing and the built-in ran on its default, silently.

* `checkArgs_ok_iff`: a call is taken exactly when every argument it gives is in the list.
* `misspelled_refused`: an argument not in the list is refused, with its name.
* `checkArgs_extend`: a built-in that gains arguments takes every call it took before (an
  extension refuses nothing).
* `checkArgs_new`: and a call it takes now but refused before gives one of the new arguments.
-/

namespace ForgeLang
namespace Families

/-- `named`'s check for built-in `f` with argument list `allowed`. -/
def checkArgs (f : Name) (allowed gs : List Name) : Except Err Unit :=
  match gs.find? (fun k => !(allowed.contains k)) with
  | some k => .error (.unknownArg f k)
  | none => .ok ()

theorem not_mem_of_contains {allowed : List Name} {k : Name} (h : (!(allowed.contains k)) = true) :
    k ∉ allowed := by
  intro hm
  simp at h
  exact h hm

theorem checkArgs_ok_iff (f : Name) (allowed gs : List Name) :
    checkArgs f allowed gs = .ok () ↔ ∀ k ∈ gs, k ∈ allowed := by
  unfold checkArgs
  cases h : gs.find? (fun k => !(allowed.contains k)) with
  | some k =>
    have hk := not_mem_of_contains (List.find?_some (p := fun k => !(allowed.contains k)) h)
    have hm := List.mem_of_find?_eq_some h
    constructor
    · intro e; cases e
    · intro hall; exact absurd (hall k hm) hk
  | none =>
    constructor
    · intro _ k hk
      have := List.find?_eq_none.1 h k hk
      simpa using this
    · intro _; rfl

theorem misspelled_refused (f : Name) {allowed gs : List Name} {k : Name} (hg : k ∈ gs)
    (hk : k ∉ allowed) :
    ∃ k', k' ∈ gs ∧ k' ∉ allowed ∧ checkArgs f allowed gs = .error (.unknownArg f k') := by
  unfold checkArgs
  cases h : gs.find? (fun k => !(allowed.contains k)) with
  | some k' => exact ⟨k', List.mem_of_find?_eq_some h, not_mem_of_contains (List.find?_some (p := fun k => !(allowed.contains k)) h), rfl⟩
  | none =>
    have := List.find?_eq_none.1 h k hg
    simp at this
    exact absurd this hk

theorem checkArgs_extend (f : Name) {allowed gs : List Name} (more : List Name)
    (h : checkArgs f allowed gs = .ok ()) : checkArgs f (allowed ++ more) gs = .ok () := by
  rw [checkArgs_ok_iff] at h ⊢
  intro k hk
  exact List.mem_append_left _ (h k hk)

theorem checkArgs_new (f : Name) {allowed gs : List Name} (more : List Name)
    (h : checkArgs f allowed gs ≠ .ok ()) (h' : checkArgs f (allowed ++ more) gs = .ok ()) :
    ∃ k ∈ gs, k ∈ more ∧ k ∉ allowed := by
  rw [checkArgs_ok_iff] at h'
  rw [Ne, checkArgs_ok_iff] at h
  apply Classical.byContradiction
  intro hno
  apply h
  intro k hk
  rcases List.mem_append.1 (h' k hk) with h1 | h1
  · exact h1
  · exact Classical.byContradiction fun hn => hno ⟨k, hk, h1, hn⟩

end Families
end ForgeLang
