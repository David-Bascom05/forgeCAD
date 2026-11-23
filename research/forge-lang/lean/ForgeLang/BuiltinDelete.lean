import ForgeLang.Builtins

/-!
# `delete`: a conservative built-in, and what it removes

`delete(BODIES)` removes bodies from the part (forge-lang's `bodies::delete`, the kernel's
`BodyFeature::Delete`). Its name is new (`wired::NAMES`): by `Builtins.extend_conservative`
adding it changes no document that binds no `delete` (`delete_conservative`); forge-lang's test
`delete::delete_bound_nowhere` holds every `.fg` in the repository to binding none, so every part
means what it meant (`delete_faces` is another name). It is no keyword (`delete_not_keyword`).

What it does, as the kernel does it on the part's bodies (a list of bodies, each by the feature
that made it):

* `remove_gone`: a deleted body is in the part no more, so no later feature can reach it (the
  language refuses a name of it, `Deleted`; the kernel, a feature naming it, `NoBody`);
* `remove_keeps`: every other body is there as it was, in the same order (`remove_order`);
* `remove_refuses`: a body not there, or a delete of every body, is refused, and nothing changes.
-/

namespace ForgeLang
namespace Builtins

def deleteNames : List Name := ["delete"]

theorem delete_conservative {α : Type} (B : List Name) (binds : List Name) (ρ : List (Name × α))
    (h : ∀ x ∈ binds, x ∉ deleteNames) :
    shadows (B ++ deleteNames) binds = shadows B binds ∧
      (∀ x v, ρ.lookup x = some v → resolve (B ++ deleteNames) ρ x = resolve B ρ x) ∧
      (∀ x, x ∉ deleteNames → resolve (B ++ deleteNames) ρ x = resolve B ρ x) :=
  extend_conservative B deleteNames binds ρ h

theorem delete_not_keyword : ∀ n ∈ deleteNames, codes n ∉ LL1.Impl.keywords := by
  decide

/-! ## The bodies a delete leaves -/

/-- The part's bodies: each the id of the feature that made it, and the body. -/
abbrev Solids (β : Type) := List (Nat × β)

/-- The kernel's delete: refused (none) if an id names no body or every body would go; else the
bodies whose ids are not deleted, in order. -/
def remove {β : Type} (ds : List Nat) (ss : Solids β) : Option (Solids β) :=
  if ds.all (fun d => ss.any (fun s => s.1 == d)) && !(ss.all (fun s => ds.contains s.1)) then
    some (ss.filter fun s => !ds.contains s.1)
  else none

theorem remove_gone {β : Type} {ds : List Nat} {ss out : Solids β} (h : remove ds ss = some out)
    (d : Nat) (hd : d ∈ ds) : ∀ s ∈ out, s.1 ≠ d := by
  unfold remove at h
  split at h
  · cases h
    intro s hs he
    have := (List.mem_filter.mp hs).2
    simp [he, hd] at this
  · cases h

theorem remove_keeps {β : Type} {ds : List Nat} {ss out : Solids β} (h : remove ds ss = some out)
    (s : Nat × β) (hs : s ∈ ss) (hn : s.1 ∉ ds) : s ∈ out := by
  unfold remove at h
  split at h
  · cases h
    exact List.mem_filter.mpr ⟨hs, by simp [hn]⟩
  · cases h

/-- The bodies left keep their order: they are a sublist of the part's. -/
theorem remove_order {β : Type} {ds : List Nat} {ss out : Solids β} (h : remove ds ss = some out) :
    out.Sublist ss := by
  unfold remove at h
  split at h
  · cases h; exact List.filter_sublist
  · cases h

/-- A delete naming a body not there is refused. -/
theorem remove_refuses {β : Type} (ds : List Nat) (ss : Solids β) (d : Nat) (hd : d ∈ ds)
    (hn : ∀ s ∈ ss, s.1 ≠ d) : remove ds ss = none := by
  unfold remove
  have : ds.all (fun d => ss.any (fun s => s.1 == d)) = false := by
    apply List.all_eq_false.mpr
    exact ⟨d, hd, by simp; intro x hx; exact hn (d, x) hx rfl⟩
  simp [this]

/-- A delete leaves a body: never every one. -/
theorem remove_leaves_one {β : Type} {ds : List Nat} {ss out : Solids β}
    (h : remove ds ss = some out) : out ≠ [] := by
  unfold remove at h
  split at h
  · rename_i hc
    cases h
    simp only [Bool.and_eq_true, Bool.not_eq_true', List.all_eq_false] at hc
    obtain ⟨_, s, hs, hns⟩ := hc
    intro he
    have : s ∈ ss.filter fun s => !ds.contains s.1 := List.mem_filter.mpr ⟨hs, by simpa using hns⟩
    rw [he] at this; cases this
  · cases h

end Builtins
end ForgeLang
