/-!
# Classes by closure: the one union–find of the decide-once core

Every grouping the core makes is the transitive closure of a decided relation over a finite list,
never a greedy clustering by distance:

* the flood (cells joined across a facet that is not on the body's boundary),
* the fans round a vertex (result facets joined across an edge round it),
* unify (result facets of one surface and sense joined across a dissolved edge).

This is `Snap.cls` (the carriers' classes) made generic: `cls r xs a` is what a union–find over the
decided pairs computes, `cls_spec` says it is the closure (`Conn`), and for a symmetric relation the
classes partition the list (`cls_eq`). `head r xs a`, the first member of `a`'s class in list
order, is the class's name: it depends on the class alone (`head_eq`), so a class's name is fixed
by the list's (arena) order and nothing else.

Pure, terminating, no fuel that can run out (`closeN_spec`): the Rust is a loop over the list.
-/

set_option linter.unusedSectionVars false

namespace ForgeExact.DecideOnce

universe u

variable {α : Type u} [DecidableEq α]

/-- Connected by a chain of decided pairs of members of `xs`. -/
inductive Conn (r : α → α → Bool) (xs : List α) (a : α) : α → Prop
  | refl : Conn r xs a a
  | step {b c : α} : Conn r xs a b → c ∈ xs → r b c = true → Conn r xs a c

namespace Conn

variable {r : α → α → Bool} {xs : List α}

theorem trans {a b c : α} (h1 : Conn r xs a b) (h2 : Conn r xs b c) : Conn r xs a c := by
  induction h2 with
  | refl => exact h1
  | step _ hc hbc ih => exact Conn.step ih hc hbc

theorem mem {a b : α} (ha : a ∈ xs) (h : Conn r xs a b) : b ∈ xs := by
  induction h with
  | refl => exact ha
  | step _ hc _ _ => exact hc

theorem symm (hr : ∀ x y, r x y = r y x) {a b : α} (ha : a ∈ xs) (h : Conn r xs a b) :
    Conn r xs b a := by
  induction h with
  | refl => exact Conn.refl
  | @step b c hab hc hbc ih =>
    have hcb : Conn r xs c b := Conn.step Conn.refl (Conn.mem ha hab) (by rw [hr]; exact hbc)
    exact hcb.trans ih

/-- An invariant carried along every decided pair is carried along a chain. -/
theorem invariant {P : α → Prop} {a b : α} (h : Conn r xs a b) (ha : P a)
    (hstep : ∀ x y, P x → y ∈ xs → r x y = true → P y) : P b := by
  induction h with
  | refl => exact ha
  | step _ hc hbc ih => exact hstep _ _ ih hc hbc

/-- A relation-preserving map: a chain whose steps all keep `f` keeps `f`. -/
theorem congr_list {ys : List α} (h : ∀ x, x ∈ xs → x ∈ ys) {a b : α} (hab : Conn r xs a b) :
    Conn r ys a b := by
  induction hab with
  | refl => exact Conn.refl
  | step _ hc hbc ih => exact Conn.step ih (h _ hc) hbc

/-- Changing the relation where it is never asked changes nothing. -/
theorem congr_rel {r' : α → α → Bool} (h : ∀ x y, y ∈ xs → r x y = true → r' x y = true)
    {a b : α} (hab : Conn r xs a b) : Conn r' xs a b := by
  induction hab with
  | refl => exact Conn.refl
  | step _ hc hbc ih => exact Conn.step ih hc (h _ _ hc hbc)

end Conn

/-- One round: every member decided-related to the set so far, not yet in it. -/
def closeStep (r : α → α → Bool) (xs s : List α) : List α :=
  s ++ xs.filter fun b => !s.contains b && s.any fun a => r a b

def closeN (r : α → α → Bool) (xs : List α) : Nat → List α → List α
  | 0, s => s
  | n + 1, s => if (closeStep r xs s).length = s.length then s else closeN r xs n (closeStep r xs s)

/-- The class of `a`: what a union–find over the decided pairs gives. -/
def cls (r : α → α → Bool) (xs : List α) (a : α) : List α :=
  closeN r xs (xs.length + 1) [a]

/-- A set closed under the relation within `xs`. -/
def Closed (r : α → α → Bool) (xs s : List α) : Prop :=
  ∀ x ∈ s, ∀ c ∈ xs, r x c = true → c ∈ s

theorem closed_of_fix {r : α → α → Bool} {xs s : List α}
    (h : (closeStep r xs s).length = s.length) : Closed r xs s := by
  intro x hx c hc hxc
  apply Classical.byContradiction
  intro hcs
  have hnil : (xs.filter fun b => !s.contains b && s.any fun a => r a b) = [] := by
    simp only [closeStep, List.length_append] at h
    exact List.length_eq_zero_iff.mp (by omega)
  have : c ∈ xs.filter fun b => !s.contains b && s.any fun a => r a b := by
    simp only [List.mem_filter, Bool.and_eq_true, Bool.not_eq_true', List.any_eq_true]
    exact ⟨hc, by simp [hcs], x, hx, hxc⟩
  rw [hnil] at this; exact nomatch this

theorem closeStep_sound {r : α → α → Bool} {xs : List α} {a : α} {s : List α}
    (hs : ∀ x ∈ s, Conn r xs a x) : ∀ x ∈ closeStep r xs s, Conn r xs a x := by
  intro x hx
  simp only [closeStep, List.mem_append, List.mem_filter, Bool.and_eq_true, List.any_eq_true] at hx
  rcases hx with hx | ⟨hc, _, y, hy, hyx⟩
  · exact hs x hx
  · exact Conn.step (hs y hy) hc hyx

theorem closeStep_nodup {r : α → α → Bool} {xs s : List α} (hxs : xs.Nodup) (hs : s.Nodup) :
    (closeStep r xs s).Nodup := by
  unfold closeStep
  rw [List.nodup_append]
  refine ⟨hs, hxs.filter _, fun x hx y hy => ?_⟩
  simp only [List.mem_filter, Bool.and_eq_true, Bool.not_eq_true', List.contains_eq_mem,
    decide_eq_false_iff_not] at hy
  rintro rfl; exact hy.2.1 hx

theorem closeStep_sub {r : α → α → Bool} {xs s : List α} (hs : s ⊆ xs) : closeStep r xs s ⊆ xs := by
  intro x hx
  simp only [closeStep, List.mem_append, List.mem_filter] at hx
  rcases hx with hx | ⟨hx, _⟩
  · exact hs hx
  · exact hx

/-- With fuel past the list's length, the rounds reach a closed set of connected members. -/
theorem closeN_spec {r : α → α → Bool} {xs : List α} {a : α} (hxs : xs.Nodup) :
    ∀ (n : Nat) (s : List α), s.Nodup → s ⊆ xs → xs.length < s.length + n →
      (∀ x ∈ s, Conn r xs a x) → a ∈ s →
      Closed r xs (closeN r xs n s) ∧ (∀ x ∈ closeN r xs n s, Conn r xs a x) ∧ a ∈ closeN r xs n s
  | 0, s, hn, hsub, hlen, _, _ => by
    have := List.Nodup.length_le_of_subset hn hsub; omega
  | n + 1, s, hn, hsub, hlen, hconn, ha => by
    simp only [closeN]
    split
    · exact ⟨closed_of_fix (by assumption), hconn, ha⟩
    · rename_i hne
      have hgrow : s.length + 1 ≤ (closeStep r xs s).length := by
        simp only [closeStep, List.length_append] at hne ⊢; omega
      exact closeN_spec hxs n _ (closeStep_nodup hxs hn) (closeStep_sub hsub) (by omega)
        (closeStep_sound hconn) (List.mem_append_left _ ha)

/-- **The class is the transitive closure.** -/
theorem cls_spec (r : α → α → Bool) {xs : List α} (hxs : xs.Nodup) {a : α} (ha : a ∈ xs)
    (b : α) : b ∈ cls r xs a ↔ Conn r xs a b := by
  obtain ⟨hcl, hsound, hmem⟩ := closeN_spec (r := r) (a := a) hxs (xs.length + 1) [a]
    (List.nodup_cons.mpr ⟨by simp, List.nodup_nil⟩) (fun x hx => by simp at hx; exact hx ▸ ha)
    (by simp; omega) (fun x hx => by simp at hx; exact hx ▸ Conn.refl) (List.mem_singleton_self a)
  refine ⟨hsound b, fun h => ?_⟩
  induction h with
  | refl => exact hmem
  | step _ hc hbc ih => exact hcl _ ih _ hc hbc

theorem self_mem_cls (r : α → α → Bool) {xs : List α} (hxs : xs.Nodup) {a : α} (ha : a ∈ xs) :
    a ∈ cls r xs a := (cls_spec r hxs ha a).mpr Conn.refl

/-- For a symmetric relation, connected members have one class (as a set). -/
theorem cls_eq (r : α → α → Bool) (hr : ∀ x y, r x y = r y x) {xs : List α} (hxs : xs.Nodup)
    {a b : α} (ha : a ∈ xs) (hab : Conn r xs a b) (c : α) :
    c ∈ cls r xs a ↔ c ∈ cls r xs b := by
  have hb := Conn.mem ha hab
  rw [cls_spec r hxs ha, cls_spec r hxs hb]
  exact ⟨fun h => (Conn.symm hr ha hab).trans h, fun h => hab.trans h⟩

/-- The name of `a`'s class: its first member in list order. -/
def head (r : α → α → Bool) (xs : List α) (a : α) : Option α :=
  let c := cls r xs a
  xs.find? fun b => c.contains b

theorem find?_congr {p q : α → Bool} : ∀ {xs : List α}, (∀ x ∈ xs, p x = q x) →
    xs.find? p = xs.find? q
  | [], _ => rfl
  | x :: xs, h => by
    simp only [List.find?_cons]
    rw [h x (List.mem_cons_self ..), find?_congr fun y hy => h y (List.mem_cons_of_mem _ hy)]

/-- **The name depends on the class alone.** -/
theorem head_eq (r : α → α → Bool) (hr : ∀ x y, r x y = r y x) {xs : List α} (hxs : xs.Nodup)
    {a b : α} (ha : a ∈ xs) (hab : Conn r xs a b) : head r xs a = head r xs b := by
  unfold head
  dsimp only
  apply find?_congr
  intro x _
  simp only [List.contains_eq_mem]
  exact decide_eq_decide.mpr (cls_eq r hr hxs ha hab x)

/-- A member's class has a name, in the class. -/
theorem head_some (r : α → α → Bool) {xs : List α} (hxs : xs.Nodup) {a : α} (ha : a ∈ xs) :
    ∃ h, head r xs a = some h ∧ h ∈ xs ∧ Conn r xs a h := by
  unfold head
  dsimp only
  cases hf : xs.find? fun b => (cls r xs a).contains b with
  | none =>
    have := List.find?_eq_none.mp hf a ha
    simp [self_mem_cls r hxs ha] at this
  | some h =>
    have hm := List.mem_of_find?_eq_some hf
    have hp := List.find?_some hf
    simp only [List.contains_eq_mem, decide_eq_true_eq] at hp
    exact ⟨h, rfl, hm, (cls_spec r hxs ha h).mp hp⟩

/-- Looking a key up in a table made by mapping its keys. -/
theorem find?_pair {γ β : Type} [BEq γ] [LawfulBEq γ] (f : γ → β) (k : γ) :
    ∀ (xs : List γ), k ∈ xs → ((xs.map fun x => (x, f x)).find? fun e => e.1 == k) = some (k, f k)
  | [], h => nomatch h
  | x :: xs, h => by
    simp only [List.map_cons, List.find?_cons]
    by_cases e : x = k
    · subst e; simp
    · have : (x == k) = false := by simp [e]
      simp only [this]
      exact find?_pair f k xs ((List.mem_cons.mp h).resolve_left (Ne.symm e))

/-! ## Lists without repeats, kept in first-seen order -/

def dedupAux : List α → List α → List α
  | [], _ => []
  | x :: xs, seen => if x ∈ seen then dedupAux xs seen else x :: dedupAux xs (x :: seen)

/-- The list with later repeats dropped: first occurrences kept, in order. -/
def dedup (xs : List α) : List α := dedupAux xs []

theorem mem_dedupAux : ∀ (xs seen : List α) (y : α), y ∈ dedupAux xs seen ↔ y ∈ xs ∧ y ∉ seen
  | [], _, _ => by simp [dedupAux]
  | x :: xs, seen, y => by
    simp only [dedupAux]
    split
    · rw [mem_dedupAux xs seen y]
      constructor
      · rintro ⟨h1, h2⟩; exact ⟨List.mem_cons_of_mem _ h1, h2⟩
      · rintro ⟨h1, h2⟩
        rcases List.mem_cons.mp h1 with rfl | h1
        · exact absurd (by assumption) h2
        · exact ⟨h1, h2⟩
    · rw [List.mem_cons, mem_dedupAux xs (x :: seen) y]
      constructor
      · rintro (rfl | ⟨h1, h2⟩)
        · exact ⟨List.mem_cons_self .., by assumption⟩
        · exact ⟨List.mem_cons_of_mem _ h1, fun h => h2 (List.mem_cons_of_mem _ h)⟩
      · rintro ⟨h1, h2⟩
        by_cases e : y = x
        · exact Or.inl e
        · exact Or.inr ⟨(List.mem_cons.mp h1).resolve_left e,
            fun h => (List.mem_cons.mp h).elim e h2⟩

theorem nodup_dedupAux : ∀ (xs seen : List α), (dedupAux xs seen).Nodup
  | [], _ => List.nodup_nil
  | x :: xs, seen => by
    simp only [dedupAux]
    split
    · exact nodup_dedupAux xs seen
    · refine List.nodup_cons.mpr ⟨fun h => ?_, nodup_dedupAux xs _⟩
      exact ((mem_dedupAux xs _ x).mp h).2 (List.mem_cons_self ..)

theorem mem_dedup (xs : List α) (y : α) : y ∈ dedup xs ↔ y ∈ xs := by
  simp [dedup, mem_dedupAux]

theorem nodup_dedup (xs : List α) : (dedup xs).Nodup := nodup_dedupAux xs []

end ForgeExact.DecideOnce
