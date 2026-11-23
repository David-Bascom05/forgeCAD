import ForgeExact.DecideOnce.Simulation

/-!
# The flood is right, over rooms (`flood_correct`)

curved-brep-route.md §2. For curved route (iii), and equally for flat A8, a builder fills
`inside` only at flood heads, from orientation seeds. Suppose a true membership
`m : room → body → Bool` flips across exactly the pieces of each body and is constant across every
other facet. Then the core's flood membership of every room is `m`, provided `inside` agrees with
`m` at the head of each flood class. This is Coverage.lean's `memb_eq` over `IncLedger`, with the
witness asked for only at heads.
-/

set_option linter.unusedSectionVars false

namespace ForgeExact.DecideOnce

universe u

namespace IncLedger

variable {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α] {L : IncLedger α}

/-- One step of the flood keeps a membership that flips exactly across pieces. -/
theorem adj_keeps {m : α → Nat → Bool}
    (hflip : ∀ φ ∈ L.facets, ∀ i, i < L.nb → (m (L.plus φ) i ≠ m (L.minus φ) i ↔ L.isPieceOf i φ = true))
    {i : Nat} (hi : i < L.nb) {x y : α} (h : adjOf (L.crossings i) x y = true) : m x i = m y i := by
  simp only [adjOf, crossings, List.any_map, List.any_filter, List.any_eq_true,
    Bool.and_eq_true, Bool.not_eq_true', Bool.or_eq_true, beq_iff_eq, Function.comp_def] at h
  obtain ⟨φ, hφ, hnp, hxy⟩ := h
  have he : m (L.plus φ) i = m (L.minus φ) i := by
    apply Classical.byContradiction; intro hne
    have := (hflip φ hφ i hi).mp hne; rw [hnp] at this; cases this
  rcases hxy with ⟨h1, h2⟩ | ⟨h1, h2⟩
  · rw [← h1, ← h2]; exact he
  · rw [← h1, ← h2]; exact he.symm

/-- **The flood is right.** With `inside` agreeing with `m` at every flood head, the core's
membership of every room is `m`. -/
theorem flood_correct {m : α → Nat → Bool}
    (hflip : ∀ φ ∈ L.facets, ∀ i, i < L.nb → (m (L.plus φ) i ≠ m (L.minus φ) i ↔ L.isPieceOf i φ = true))
    (hhead : ∀ T ∈ L.rooms, ∀ i, i < L.nb → ∀ R, head (adjOf (L.crossings i)) L.rooms T = some R →
      L.inside.contains (R, i) = m R i)
    {i : Nat} (hi : i < L.nb) {T : α} (hT : T ∈ L.rooms) : L.memb i T = m T i := by
  obtain ⟨R, hR, _, hc⟩ := head_some (adjOf (L.crossings i)) (xs := L.rooms) (nodup_dedup _) hT
  unfold memb
  rw [hR]
  simp only
  rw [hhead T hT i hi R hR]
  exact (Conn.invariant (P := fun x => m x i = m T i) hc rfl
    fun x y hx _ hxy => (adj_keeps hflip hi hxy).symm.trans hx)

/-- And the classify check passes. -/
theorem classifyOk_of_flood {m : α → Nat → Bool}
    (hflip : ∀ φ ∈ L.facets, ∀ i, i < L.nb → (m (L.plus φ) i ≠ m (L.minus φ) i ↔ L.isPieceOf i φ = true))
    (hhead : ∀ T ∈ L.rooms, ∀ i, i < L.nb → ∀ R, head (adjOf (L.crossings i)) L.rooms T = some R →
      L.inside.contains (R, i) = m R i) : L.classifyOk = true := by
  simp only [classifyOk, List.all_eq_true, List.mem_range, Bool.or_eq_true, Bool.not_eq_true']
  intro φ hφ i hi
  by_cases hp : L.isPieceOf i φ = true
  · right
    rw [flood_correct hflip hhead hi (L.plus_mem_rooms hφ), flood_correct hflip hhead hi (L.minus_mem_rooms hφ)]
    simpa using (hflip φ hφ i hi).mpr hp
  · left; simpa using hp

end IncLedger

end ForgeExact.DecideOnce
