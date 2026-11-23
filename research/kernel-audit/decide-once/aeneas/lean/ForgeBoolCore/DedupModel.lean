import ForgeExact.DecideOnce.Closure

/-!
# `dedup`, as the Rust's first-seen table computes it

`dedup (xs ++ [x])` adds `x` exactly when it is new (`dedup_snoc`), and an injective map commutes
with `dedup` (`dedup_map`): ids without repeated covectors dedup as their covectors do.
-/

namespace ForgeBoolCore.Dedup

open ForgeExact.DecideOnce

variable {α β : Type} [DecidableEq α] [DecidableEq β]

theorem dedupAux_congr : ∀ (xs : List α) {s t : List α}, (∀ y, y ∈ s ↔ y ∈ t) →
    dedupAux xs s = dedupAux xs t
  | [], _, _, _ => rfl
  | x :: xs, s, t, h => by
    simp only [dedupAux]
    by_cases hx : x ∈ s
    · rw [if_pos hx, if_pos ((h x).mp hx)]; exact dedupAux_congr xs h
    · rw [if_neg hx, if_neg (fun e => hx ((h x).mpr e))]
      congr 1
      exact dedupAux_congr xs (fun y => by simp [h y])

theorem dedupAux_snoc : ∀ (xs : List α) (x : α) (s : List α),
    dedupAux (xs ++ [x]) s = dedupAux xs s ++ (if x ∈ xs ∨ x ∈ s then [] else [x])
  | [], x, s => by by_cases h : x ∈ s <;> simp [dedupAux, h]
  | y :: ys, x, s => by
    simp only [List.cons_append, dedupAux]
    by_cases hy : y ∈ s
    · rw [if_pos hy, if_pos hy, dedupAux_snoc ys x s]
      congr 1
      by_cases e : x = y
      · subst e; simp [hy]
      · simp [e]
    · rw [if_neg hy, if_neg hy, dedupAux_snoc ys x (y :: s), List.cons_append]
      congr 2
      by_cases e : x = y
      · subst e; simp
      · simp [e]

theorem dedup_snoc (xs : List α) (x : α) :
    dedup (xs ++ [x]) = dedup xs ++ (if x ∈ xs then [] else [x]) := by
  unfold dedup; rw [dedupAux_snoc]; simp

theorem dedup_map (f : α → β) : ∀ (xs : List α), (∀ a ∈ xs, ∀ b ∈ xs, f a = f b → a = b) →
    dedup (xs.map f) = (dedup xs).map f := by
  intro xs
  rw [← List.reverse_reverse xs]
  generalize xs.reverse = zs
  induction zs with
  | nil => intro _; rfl
  | cons y zs ih =>
    intro hinj
    rw [List.reverse_cons] at hinj ⊢
    generalize zs.reverse = ys at hinj ih ⊢
    rw [List.map_append, List.map_singleton, dedup_snoc, dedup_snoc, List.map_append,
      ih (fun a ha b hb => hinj a (by simp [ha]) b (by simp [hb]))]
    congr 1
    by_cases hy : y ∈ ys
    · have : f y ∈ ys.map f := List.mem_map.mpr ⟨y, hy, rfl⟩
      simp [hy, this]
    · have : f y ∉ ys.map f := by
        intro h; obtain ⟨a, ha, he⟩ := List.mem_map.mp h
        exact hy (hinj a (by simp [ha]) y (by simp) he ▸ ha)
      simp [hy, this]

theorem dedupAux_length_le : ∀ (xs s : List α), (dedupAux xs s).length ≤ xs.length
  | [], _ => by simp [dedupAux]
  | x :: xs, s => by
    simp only [dedupAux]
    split
    · have := dedupAux_length_le xs s; simp; omega
    · have := dedupAux_length_le xs (x :: s); simp; omega

theorem dedup_length_le (xs : List α) : (dedup xs).length ≤ xs.length := dedupAux_length_le xs []

theorem idxOf_dedup_snoc_old {xs : List α} {x c : α} (hc : c ∈ xs) :
    (dedup (xs ++ [x])).idxOf c = (dedup xs).idxOf c := by
  rw [dedup_snoc]
  split
  · simp
  · rw [List.idxOf_append, if_pos ((mem_dedup _ _).mpr hc)]

theorem idxOf_dedup_snoc_new {xs : List α} {x : α} (hx : x ∉ xs) :
    (dedup (xs ++ [x])).idxOf x = (dedup xs).length := by
  rw [dedup_snoc, if_neg hx, List.idxOf_append, if_neg (fun h => hx ((mem_dedup _ _).mp h))]
  simp

end ForgeBoolCore.Dedup
