import Mathlib.Data.List.Iterate
import Mathlib.Logic.Function.Iterate
import Mathlib.Data.List.Nodup
import Mathlib.Tactic

/-!
# The walk of `chain`: closed walks of a permutation (the model)

Once the degrees pass (every vertex left as often as reached, at most once), the successor of a
piece, `σ e` (the first piece leaving the vertex `e` arrives at), maps the pieces `0 .. m-1`
into themselves injectively, so it is a permutation. A walk from an unused piece, marking pieces
until it meets a marked one, meets exactly its start (`walk_closes`): the kernel's `at != start`
refusal can never happen; and after it the marked set is again closed under σ both ways
(`closed_add`). So the loops `chain` returns are σ's cycles.
-/

namespace ForgeSheetCore.Walk

variable (σ : Nat → Nat) (m : Nat)

/-- σ maps `0 .. m-1` into itself, injectively. -/
structure Perm : Prop where
  lt : ∀ e, e < m → σ e < m
  inj : ∀ e e', e < m → e' < m → σ e = σ e' → e = e'

variable {σ m}

theorem iter_lt (hp : Perm σ m) {s : Nat} (hs : s < m) : ∀ k, σ^[k] s < m
  | 0 => hs
  | k + 1 => by rw [Function.iterate_succ_apply']; exact hp.lt _ (iter_lt hp hs k)

/-- The path of a walk: `s, σ s, …, σ^(k-1) s`. -/
def path (σ : Nat → Nat) (s k : Nat) : List Nat := (List.range k).map fun i => σ^[i] s

theorem path_succ (s k : Nat) : path σ s (k + 1) = path σ s k ++ [σ^[k] s] := by
  simp [path, List.range_succ]

theorem mem_path {s k e : Nat} : e ∈ path σ s k ↔ ∃ i < k, σ^[i] s = e := by
  simp only [path, List.mem_map, List.mem_range]

theorem path_length (s k : Nat) : (path σ s k).length = k := by simp [path]

/-- **The walk meets its start**: from `s` outside a set `P0` closed under σ's preimage, along a
path of `k ≥ 1` distinct pieces outside `P0`, the first marked piece reached is `s` itself. -/
theorem walk_closes (hp : Perm σ m) {P0 : List Nat}
    (hpre : ∀ e, e < m → σ e ∈ P0 → e ∈ P0) {s k : Nat} (hs : s < m) (hk : 0 < k)
    (hout : ∀ e ∈ path σ s k, e ∉ P0) (hnd : (path σ s k).Nodup)
    (hmet : σ^[k] s ∈ P0 ∨ σ^[k] s ∈ path σ s k) : σ^[k] s = s := by
  obtain ⟨k', rfl⟩ : ∃ k', k = k' + 1 := ⟨k - 1, by omega⟩
  have hlast : σ^[k'] s ∈ path σ s (k' + 1) := mem_path.mpr ⟨k', by omega, rfl⟩
  rcases hmet with h | h
  · rw [Function.iterate_succ_apply'] at h
    exact absurd (hpre _ (iter_lt hp hs k') h) (hout _ hlast)
  · obtain ⟨i, hi, hie⟩ := mem_path.mp h
    rcases Nat.eq_zero_or_pos i with rfl | hpos
    · exact hie.symm
    · exfalso
      obtain ⟨i', rfl⟩ : ∃ i', i = i' + 1 := ⟨i - 1, by omega⟩
      rw [Function.iterate_succ_apply', Function.iterate_succ_apply'] at hie
      have hik := hp.inj _ _ (iter_lt hp hs i') (iter_lt hp hs k') hie
      -- σ^[i'] s = σ^[k'] s at two places i' < k' of a path without repeats
      have hi' : i' < k' := by omega
      have := List.nodup_iff_injective_get.mp hnd
      have e : (path σ s (k' + 1))[i']'(by rw [path_length]; omega) =
          (path σ s (k' + 1))[k']'(by rw [path_length]; omega) := by
        simp only [path, List.getElem_map, List.getElem_range]; exact hik
      have := this (a₁ := ⟨i', by rw [path_length]; omega⟩) (a₂ := ⟨k', by rw [path_length]; omega⟩) e
      simp at this; omega

/-- A cycle: a walk from its first piece that σ brings back to it. -/
def IsCycle (σ : Nat → Nat) (c : List Nat) : Prop :=
  ∃ s k, 0 < k ∧ c = path σ s k ∧ σ^[k] s = s

/-- **The marked set stays closed**: adding a closed walk to a set closed under σ both ways
keeps it closed both ways. -/
theorem closed_add (hp : Perm σ m) {P0 : List Nat}
    (hc : ∀ e, e < m → (e ∈ P0 ↔ σ e ∈ P0)) {s k : Nat} (hs : s < m) (hk : 0 < k)
    (hcyc : σ^[k] s = s) :
    ∀ e, e < m → (e ∈ P0 ++ path σ s k ↔ σ e ∈ P0 ++ path σ s k) := by
  intro e he
  simp only [List.mem_append]
  constructor
  · rintro (h | h)
    · exact Or.inl ((hc e he).mp h)
    · obtain ⟨i, hi, rfl⟩ := mem_path.mp h
      right
      rcases Nat.lt_or_ge (i + 1) k with h1 | h1
      · exact mem_path.mpr ⟨i + 1, h1, by rw [Function.iterate_succ_apply']⟩
      · have : i + 1 = k := by omega
        exact mem_path.mpr ⟨0, hk, by
          simp only [Function.iterate_zero, id]
          rw [← Function.iterate_succ_apply' σ i s]
          show s = σ^[i + 1] s
          rw [this, hcyc]⟩
  · rintro (h | h)
    · exact Or.inl ((hc e he).mpr h)
    · obtain ⟨i, hi, hie⟩ := mem_path.mp h
      right
      -- σ e = σ^[i] s: e is σ^[i-1] s, or σ^[k-1] s when i = 0
      rcases Nat.eq_zero_or_pos i with rfl | hpos
      · obtain ⟨k', rfl⟩ : ∃ k', k = k' + 1 := ⟨k - 1, by omega⟩
        have : σ (σ^[k'] s) = σ e := by
          rw [← Function.iterate_succ_apply' σ k' s, hcyc]; simpa using hie
        have := hp.inj _ _ (iter_lt hp hs k') he this
        exact mem_path.mpr ⟨k', by omega, this⟩
      · obtain ⟨i', rfl⟩ : ∃ i', i = i' + 1 := ⟨i - 1, by omega⟩
        rw [Function.iterate_succ_apply'] at hie
        have := hp.inj _ _ (iter_lt hp hs i') he hie
        exact mem_path.mpr ⟨i', by omega, this⟩

end ForgeSheetCore.Walk
