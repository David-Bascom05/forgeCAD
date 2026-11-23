/-!
# Proof statements for speed-ups S1–S3 (research/booleans/speedups.md §"Proof statements")

Core Lean only (no Mathlib), the forge-exact toolchain. Each theorem is the abstract fact the
Rust change relies on. The Aeneas refinement of each change cites it.

* S1 `across_memo`: memoising a pure function returns the function's value, and keeps the table
  sound (`memo_get`, `memo_sound`).
* S2 `scratch_trim`: a buffer cleared before use is the fresh empty buffer, so filling it gives
  what a fresh one gives (`cleared_fill`). The Aeneas model has no capacity.
* S3 `packed_sorts`:
  - a list of distinct keys has one sorted order (`sorted_unique`);
  - `VKey::packed` is strictly monotone within its bounds (`packed_mono`);
  - so sorting by the packed key gives the derived order's sort (`sort_packed_eq`).
-/

namespace BoolResearch.Speedups

/-! ## S1: memoisation of a pure function -/

/-- A memo table: an association list. -/
abbrev Memo (K V : Type) := List (K × V)

def lookup {K V : Type} [DecidableEq K] (m : Memo K V) (k : K) : Option V :=
  (m.find? fun e => e.1 == k).map Prod.snd

/-- Look up `k`, computing and storing `f k` on a miss. -/
def getOr {K V : Type} [DecidableEq K] (f : K → V) (m : Memo K V) (k : K) : V × Memo K V :=
  match lookup m k with
  | some v => (v, m)
  | none => (f k, (k, f k) :: m)

/-- The table only holds values of `f`. -/
def Sound {K V : Type} [DecidableEq K] (f : K → V) (m : Memo K V) : Prop :=
  ∀ k v, lookup m k = some v → v = f k

theorem lookup_cons {K V : Type} [DecidableEq K] (k k' : K) (v : V) (m : Memo K V) :
    lookup ((k, v) :: m) k' = if k = k' then some v else lookup m k' := by
  unfold lookup
  by_cases h : k = k'
  · subst h; simp
  · simp [h]

/-- **S1, value.** On a sound table the memoised call returns `f k`. -/
theorem memo_get {K V : Type} [DecidableEq K] (f : K → V) (m : Memo K V) (hm : Sound f m) (k : K) :
    (getOr f m k).1 = f k := by
  unfold getOr
  cases h : lookup m k with
  | none => rfl
  | some v => exact hm k v h

/-- **S1, invariant.** The memoised call keeps the table sound. -/
theorem memo_sound {K V : Type} [DecidableEq K] (f : K → V) (m : Memo K V) (hm : Sound f m) (k : K) :
    Sound f (getOr f m k).2 := by
  unfold getOr
  cases h : lookup m k with
  | some v => exact hm
  | none =>
    intro k' v' h'
    rw [lookup_cons] at h'
    by_cases e : k = k'
    · subst e; simp at h'; exact h'.symm
    · simp [e] at h'; exact hm k' v' h'

/-- The empty table is sound. A boolean starts with it, which is the per-boolean invalidation. -/
theorem memo_nil {K V : Type} [DecidableEq K] (f : K → V) : Sound f ([] : Memo K V) := by
  intro k v h; simp [lookup] at h

/-! ## S2: a cleared buffer is a fresh one -/

/-- In Aeneas's model a `Vec` is its list of elements. `clear` gives `[]`, and pushing `xs` onto
it gives `xs`, as a fresh `Vec::new()` would. -/
theorem cleared_fill {α : Type} (_old : List α) (xs : List α) :
    ([] : List α) ++ xs = [] ++ xs ∧ ([] : List α) ++ xs = xs := ⟨rfl, List.nil_append xs⟩

/-! ## S3: sorting is unique on distinct keys; the packed key is monotone -/

/-- **Two sorted permutations of one list are equal** (strict order on `Nat`). -/
theorem sorted_unique : ∀ (l₁ l₂ : List Nat), l₁.Perm l₂ →
    l₁.Pairwise (· < ·) → l₂.Pairwise (· < ·) → l₁ = l₂
  | [], l₂, hp, _, _ => List.Perm.nil_eq hp
  | a :: r₁, [], hp, _, _ => absurd (List.Perm.length_eq hp) (by simp)
  | a :: r₁, b :: r₂, hp, h₁, h₂ => by
    have hab : a = b := by
      have ha : a ∈ b :: r₂ := hp.mem_iff.mp (List.mem_cons_self ..)
      have hb : b ∈ a :: r₁ := hp.mem_iff.mpr (List.mem_cons_self ..)
      rcases List.mem_cons.mp ha with e | ha'
      · exact e
      · rcases List.mem_cons.mp hb with e | hb'
        · exact e.symm
        · have x : b < a := (List.pairwise_cons.mp h₂).1 a ha'
          have y : a < b := (List.pairwise_cons.mp h₁).1 b hb'
          omega
    subst hab
    have hr : r₁.Perm r₂ := (List.perm_cons a).mp hp
    rw [sorted_unique r₁ r₂ hr (List.pairwise_cons.mp h₁).2 (List.pairwise_cons.mp h₂).2]

/-- The kernel's `VKey`, `Old(side, vertex) < New(n)`, with lexicographic order on `Old`
(the derived `Ord` of `enum VKey { Old(Side, VertexId), New(u32) }`). -/
inductive VKey
  | old (s v : Nat)
  | new (n : Nat)

def VKey.lt : VKey → VKey → Prop
  | .old s v, .old s' v' => s < s' ∨ (s = s' ∧ v < v')
  | .old _ _, .new _ => True
  | .new _, .old _ _ => False
  | .new n, .new n' => n < n'

/-- Within the kernel's widths: a side is 0 or 1, and vertex ids and new ids are `u32`. -/
def VKey.ok : VKey → Prop
  | .old s v => s < 2 ∧ v < 2 ^ 32
  | .new n => n < 2 ^ 32

/-- `VKey::packed` (boolean.rs:224): `Old(s, v) ↦ s·2³² + v`, `New(n) ↦ 2⁶³ + n`. -/
def VKey.packed : VKey → Nat
  | .old s v => s * 2 ^ 32 + v
  | .new n => 2 ^ 63 + n

/-- **`packed` is strictly monotone** on keys within the widths. -/
theorem packed_mono : ∀ a b : VKey, a.ok → b.ok → a.lt b → a.packed < b.packed
  | .old s v, .old s' v', ⟨_, hv⟩, ⟨_, hv'⟩, h => by
    simp only [VKey.packed]
    rcases h with h | ⟨rfl, h⟩
    · have : s * 2 ^ 32 + 2 ^ 32 ≤ s' * 2 ^ 32 := by
        have := Nat.mul_le_mul_right (2 ^ 32) (Nat.succ_le_of_lt h)
        simpa [Nat.succ_mul] using this
      omega
    · omega
  | .old s v, .new n, ⟨hs, hv⟩, _, _ => by
    simp only [VKey.packed]
    have : s * 2 ^ 32 ≤ 1 * 2 ^ 32 := Nat.mul_le_mul_right _ (by omega)
    omega
  | .new _, .old _ _, _, _, h => absurd h (by simp [VKey.lt])
  | .new n, .new n', _, _, h => by simp only [VKey.packed, VKey.lt] at *; omega

/-- **S3.** Sorting the packed keys of distinct, in-width `VKey`s gives the same list as sorting
by the derived order. Stated on the packed images: a list sorted by `lt` maps to a list sorted by
`<` on `packed`. So `sorted_unique` makes the packed `sort_unstable` order the one sorted order. -/
theorem sort_packed_eq (l : List VKey) (hok : ∀ k ∈ l, k.ok) (hs : l.Pairwise VKey.lt) :
    (l.map VKey.packed).Pairwise (· < ·) := by
  rw [List.pairwise_map]
  exact List.Pairwise.imp_of_mem (fun {a b} ha hb h => packed_mono a b (hok a ha) (hok b hb) h) hs

end BoolResearch.Speedups
