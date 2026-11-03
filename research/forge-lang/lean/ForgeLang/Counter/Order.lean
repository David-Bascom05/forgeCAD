import ForgeLang.Core

/-!
# P1: what "one meaning" needs, and what it does not

Counterexample: a loop over an unordered collection (a set, a hash map) with ids by position. Two
legal iteration orders give two models that disagree on what id 0 is.

Not essential: a deterministic *evaluation order*. If ids are keys (unique, not positions), every
order of the same entries is the same model as a map from id to feature (`perm_lookup`). Parallel
evaluation, or iterating a map in hash order, is fine when the ids do not come from the order.
-/

namespace ForgeLang.Counter

def byIndex (xs : List Int) : List (Nat × Int) := (List.range xs.length).zip xs

theorem two_orders :
    [1, 2].Perm [2, 1] ∧ (byIndex [1, 2]).lookup 0 = some 1 ∧ (byIndex [2, 1]).lookup 0 = some 2 :=
  ⟨List.Perm.swap 2 1 [], rfl, rfl⟩

theorem perm_lookup {α β : Type} [BEq α] [LawfulBEq α] {m m' : List (α × β)} (hp : m.Perm m')
    (hn : (m.map Prod.fst).Nodup) (a : α) : m.lookup a = m'.lookup a := by
  induction hp with
  | nil => rfl
  | cons x p ih =>
    obtain ⟨k, v⟩ := x
    simp only [List.map_cons, List.nodup_cons] at hn
    simp only [List.lookup]
    split
    · rfl
    · exact ih hn.2
  | swap x y l =>
    obtain ⟨kx, vx⟩ := x
    obtain ⟨ky, vy⟩ := y
    simp only [List.map_cons, List.nodup_cons, List.mem_cons, not_or] at hn
    have hne : ky ≠ kx := hn.1.1
    simp only [List.lookup]
    by_cases h1 : a = ky
    · subst h1
      have : (a == kx) = false := by simp [hne]
      simp [this]
    · have : (a == ky) = false := by simp [h1]
      simp [this]
  | trans p1 p2 ih1 ih2 =>
    rw [ih1 hn, ih2 ((p1.map Prod.fst).nodup_iff.1 hn)]

end ForgeLang.Counter
