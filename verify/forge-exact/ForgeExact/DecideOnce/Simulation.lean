import ForgeExact.DecideOnce.Refines

/-!
# A8 L2: the simulation theorem (`core_refines`)

Stage by stage, for `Refines A B σ τ ν κ`:
1. the split checks agree;
2. memberships and keeps agree facet-side by facet-side, so the classify checks agree and the
   results correspond (`σ φ ∈ B.result ↔ φ ∈ A.result`);
3. the touch rule: bad edges and bad verts correspond;
4. unify: faces, edges and verts correspond.
-/

set_option linter.unusedSectionVars false

namespace ForgeExact.DecideOnce

universe u v

namespace IncLedger

variable {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α] (L : IncLedger α)

theorem plus_mem_rooms {φ : α} (h : φ ∈ L.facets) : L.plus φ ∈ L.rooms :=
  (mem_dedup _ _).mpr (List.mem_flatMap.mpr ⟨φ, h, by simp⟩)

theorem minus_mem_rooms {φ : α} (h : φ ∈ L.facets) : L.minus φ ∈ L.rooms :=
  (mem_dedup _ _).mpr (List.mem_flatMap.mpr ⟨φ, h, by simp⟩)

theorem find?_pair' {γ : Type u} {δ : Type v} [BEq γ] [LawfulBEq γ] (f : γ → δ) (k : γ) :
    ∀ (xs : List γ), k ∈ xs → ((xs.map fun x => (x, f x)).find? fun e => e.1 == k) = some (k, f k)
  | [], h => nomatch h
  | x :: xs, h => by
    simp only [List.map_cons, List.find?_cons]
    by_cases e : x = k
    · subst e; simp
    · have : (x == k) = false := by simp [e]
      simp only [this]
      exact find?_pair' f k xs ((List.mem_cons.mp h).resolve_left (Ne.symm e))

theorem keepIn_tab (rule : Rule) {T : α} (hT : T ∈ L.rooms) : keepInI (L.keepTab rule) T = L.keep rule T := by
  simp only [keepInI, keepTab]
  rw [find?_pair' (L.keep rule) T _ hT]; rfl

theorem isPieceOf_iff (i : Nat) (φ : α) : L.isPieceOf i φ = true ↔ ∃ f, (φ, f) ∈ L.pieces ∧ L.bodyOf f = i := by
  simp only [isPieceOf, List.any_eq_true, Bool.and_eq_true, beq_iff_eq]
  constructor
  · rintro ⟨⟨a, f⟩, hp, rfl, hb⟩; exact ⟨f, hp, hb⟩
  · rintro ⟨f, hp, hb⟩; exact ⟨(φ, f), hp, rfl, hb⟩

theorem mem_result {kt : KeepTabI α} {φ : α} :
    φ ∈ L.result kt ↔ φ ∈ L.facets ∧ keepInI kt (L.plus φ) ≠ keepInI kt (L.minus φ) := by
  simp [result, List.mem_filter]

end IncLedger

namespace Refines

variable {α : Type u} {β : Type v} [DecidableEq α] [BEq α] [LawfulBEq α] [DecidableEq β] [BEq β] [LawfulBEq β]
  {A : IncLedger α} {B : IncLedger β} {σ : α → β} {τ ν : α → Option β} {κ : α → Sector α → Sector β}

theorem bodyOf_eq (h : Refines A B σ τ ν κ) (f : Nat) : B.bodyOf f = A.bodyOf f := by
  simp [IncLedger.bodyOf, h.faceBody]

theorem clsOf_eq (h : Refines A B σ τ ν κ) (f : Nat) : B.clsOf f = A.clsOf f := by
  simp [IncLedger.clsOf, h.faceCls]

theorem isPieceOf_eq (h : Refines A B σ τ ν κ) {φ : α} (hφ : φ ∈ A.facets) (i : Nat) :
    B.isPieceOf i (σ φ) = A.isPieceOf i φ := by
  rw [Bool.eq_iff_iff, IncLedger.isPieceOf_iff, IncLedger.isPieceOf_iff]
  simp only [bodyOf_eq h]
  exact ⟨fun ⟨f, hp, hb⟩ => ⟨f, (h.pieces φ hφ f).mpr hp, hb⟩, fun ⟨f, hp, hb⟩ => ⟨f, (h.pieces φ hφ f).mp hp, hb⟩⟩

/-- 1. The coarse split is sound when the fine one is. -/
theorem splitOk (h : Refines A B σ τ ν κ) (hs : A.splitOk = true) : B.splitOk = true := by
  simp only [IncLedger.splitOk, List.all_eq_true, Bool.and_eq_true] at hs ⊢
  intro p hp
  obtain ⟨φ, hφ, hσ⟩ := h.pieces_from p hp
  have hpA : (φ, p.2) ∈ A.pieces := (h.pieces φ hφ p.2).mpr (by rw [hσ]; exact hp)
  obtain ⟨⟨⟨⟨_, hc⟩, hb⟩, _⟩, hq⟩ := hs _ hpA
  have hψ : p.1 ∈ B.facets := hσ ▸ h.facet_mem φ hφ
  refine ⟨⟨⟨⟨by simpa using hψ, ?_⟩, ?_⟩, by simpa using h.sidesB _ hψ⟩, fun q hq' => ?_⟩
  · rw [← hσ, h.carrier φ hφ, clsOf_eq h]; exact hc
  · rw [bodyOf_eq h, h.nb]; exact hb
  · by_cases e1 : q.1 = p.1
    · by_cases e2 : B.bodyOf q.2 = B.bodyOf p.2
      · have hqA : (φ, q.2) ∈ A.pieces := (h.pieces φ hφ q.2).mpr (by rw [hσ, ← e1]; exact hq')
        have := hq _ hqA
        simp only [bodyOf_eq h] at e2
        simp only [e2, beq_self_eq_true, Bool.and_true, Bool.not_true, Bool.false_or] at this
        simp [this]
      · simp [e2]
    · simp [e1]

/-! ### 2. Membership, keep, result -/

theorem keep_plus (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    B.keep rule (B.plus (σ φ)) = A.keep rule (A.plus φ) := by
  simp only [IncLedger.keep, h.nb]
  congr 1; funext i
  by_cases hi : i < A.nb
  · simp [hi, h.memb_plus φ hφ i hi]
  · simp [hi]

theorem keep_minus (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    B.keep rule (B.minus (σ φ)) = A.keep rule (A.minus φ) := by
  simp only [IncLedger.keep, h.nb]
  congr 1; funext i
  by_cases hi : i < A.nb
  · simp [hi, h.memb_minus φ hφ i hi]
  · simp [hi]

theorem keepIn_plus (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    keepInI (B.keepTab rule) (B.plus (σ φ)) = keepInI (A.keepTab rule) (A.plus φ) := by
  rw [B.keepIn_tab rule (B.plus_mem_rooms (h.facet_mem φ hφ)), A.keepIn_tab rule (A.plus_mem_rooms hφ),
    keep_plus h rule hφ]

theorem keepIn_minus (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    keepInI (B.keepTab rule) (B.minus (σ φ)) = keepInI (A.keepTab rule) (A.minus φ) := by
  rw [B.keepIn_tab rule (B.minus_mem_rooms (h.facet_mem φ hφ)), A.keepIn_tab rule (A.minus_mem_rooms hφ),
    keep_minus h rule hφ]

/-- A fine facet is in the result iff its image is. -/
theorem result_iff (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    σ φ ∈ B.result (B.keepTab rule) ↔ φ ∈ A.result (A.keepTab rule) := by
  rw [IncLedger.mem_result, IncLedger.mem_result, keepIn_plus h rule hφ, keepIn_minus h rule hφ]
  exact ⟨fun ⟨_, h2⟩ => ⟨hφ, h2⟩, fun ⟨_, h2⟩ => ⟨h.facet_mem φ hφ, h2⟩⟩

/-- Every coarse result facet is the image of a fine one. -/
theorem result_from (h : Refines A B σ τ ν κ) (rule : Rule) {ψ : β} (hψ : ψ ∈ B.result (B.keepTab rule)) :
    ∃ φ ∈ A.result (A.keepTab rule), σ φ = ψ := by
  obtain ⟨φ, hφ, rfl⟩ := h.facet_onto ψ (B.mem_result.mp hψ).1
  exact ⟨φ, (result_iff h rule hφ).mp hψ, rfl⟩

theorem orient_eq (h : Refines A B σ τ ν κ) (rule : Rule) {φ : α} (hφ : φ ∈ A.facets) :
    B.orient (B.keepTab rule) (σ φ) = A.orient (A.keepTab rule) φ := by
  simp only [IncLedger.orient, keepIn_minus h rule hφ]

/-- The classify checks agree. -/
theorem classifyOk_eq (h : Refines A B σ τ ν κ) : B.classifyOk = A.classifyOk := by
  have hφ : ∀ φ ∈ A.facets, ((List.range B.nb).all fun i =>
      !B.isPieceOf i (σ φ) || (B.memb i (B.plus (σ φ)) != B.memb i (B.minus (σ φ)))) =
      ((List.range A.nb).all fun i => !A.isPieceOf i φ || (A.memb i (A.plus φ) != A.memb i (A.minus φ))) := by
    intro φ hφ
    have hi : ∀ i, i < A.nb → (!B.isPieceOf i (σ φ) || (B.memb i (B.plus (σ φ)) != B.memb i (B.minus (σ φ)))) =
        (!A.isPieceOf i φ || (A.memb i (A.plus φ) != A.memb i (A.minus φ))) := fun i hi => by
      rw [isPieceOf_eq h hφ, h.memb_plus φ hφ i hi, h.memb_minus φ hφ i hi]
    rw [h.nb, Bool.eq_iff_iff, List.all_eq_true, List.all_eq_true]
    exact ⟨fun H i hr => by rw [← hi i (List.mem_range.mp hr)]; exact H i hr,
      fun H i hr => by rw [hi i (List.mem_range.mp hr)]; exact H i hr⟩
  rw [Bool.eq_iff_iff]
  unfold IncLedger.classifyOk
  rw [List.all_eq_true, List.all_eq_true]
  constructor
  · intro hB φ hφ'; rw [← hφ φ hφ']; exact hB _ (h.facet_mem φ hφ')
  · intro hA ψ hψ
    obtain ⟨φ, hφ', rfl⟩ := h.facet_onto ψ hψ
    rw [hφ φ hφ']; exact hA φ hφ'

/-! ### 3. The touch rule at edges -/

theorem aroundE_perm (h : Refines A B σ τ ν κ) (rule : Rule) {e : α} (he : e ∈ A.edges) {E : β}
    (hτ : τ e = some E) :
    ((A.aroundE (A.keepTab rule) e).map σ).Perm (B.aroundE (B.keepTab rule) E) := by
  have hw := (h.edge_wings e he E hτ).filter fun ψ => (B.result (B.keepTab rule)).contains ψ
  simp only [IncLedger.aroundE]
  refine List.Perm.trans ?_ hw
  rw [List.filter_map]
  apply List.Perm.of_eq
  congr 1
  apply List.filter_congr
  intro φ hφ
  have hf := h.edge_facets_A e he φ hφ
  simp only [Function.comp_def]
  rw [Bool.eq_iff_iff]
  simp only [List.contains_iff_mem]
  exact (result_iff h rule hf).symm

theorem badEdge_some (h : Refines A B σ τ ν κ) (rule : Rule) {e : α} (he : e ∈ A.edges) {E : β}
    (hτ : τ e = some E) : B.badEdge (B.keepTab rule) E = A.badEdge (A.keepTab rule) e := by
  simp only [IncLedger.badEdge, ← (aroundE_perm h rule he hτ).length_eq, List.length_map]

theorem badEdge_none (h : Refines A B σ τ ν κ) (rule : Rule) {e : α} (he : e ∈ A.edges)
    (hτ : τ e = none) : A.badEdge (A.keepTab rule) e = false := by
  simp only [IncLedger.badEdge, IncLedger.aroundE]
  apply decide_eq_false
  apply Nat.not_lt.mpr
  exact Nat.le_trans (List.length_filter_le _ _) (h.edge_inner e he hτ).1

/-- A coarse edge is bad iff some fine edge is. -/
theorem bad_edge_iff (h : Refines A B σ τ ν κ) (rule : Rule) :
    (∃ E ∈ B.edges, B.badEdge (B.keepTab rule) E = true) ↔
    (∃ e ∈ A.edges, A.badEdge (A.keepTab rule) e = true) := by
  constructor
  · rintro ⟨E, hE, hb⟩
    obtain ⟨e, he, hτ⟩ := h.edge_onto E hE
    exact ⟨e, he, by rw [← badEdge_some h rule he hτ]; exact hb⟩
  · rintro ⟨e, he, hb⟩
    cases hτ : τ e with
    | none => rw [badEdge_none h rule he hτ] at hb; cases hb
    | some E => exact ⟨E, h.edge_mem e he E hτ, by rw [badEdge_some h rule he hτ]; exact hb⟩

end Refines

end ForgeExact.DecideOnce
