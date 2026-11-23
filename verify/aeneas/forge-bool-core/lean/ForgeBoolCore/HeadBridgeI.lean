import ForgeBoolCore.UFModel
import ForgeExact.DecideOnce.IncCore

/-!
# Least position = `head`, for the A8 core

HeadBridge.lean for `IncLedger.adjOf` over any cell type: a union-find naming each class of the
positions' pairs by its least position names it by its first member in list order, which is
`head (adjOf cr) xs`.
-/

namespace ForgeBoolCore.UF

open ForgeExact.DecideOnce

universe u

variable {α : Type u} [DecidableEq α] [BEq α] [LawfulBEq α]

/-- The pairs by position. -/
def posPairsI (xs : List α) (cr : List (α × α)) : List (Nat × Nat) :=
  cr.map fun p => (xs.idxOf p.1, xs.idxOf p.2)

theorem adjOf_symmI (cr : List (α × α)) (x y : α) : IncLedger.adjOf cr x y = IncLedger.adjOf cr y x := by
  unfold IncLedger.adjOf
  apply List.any_congr
  intro p _
  cases (p.1 == x) <;> cases (p.2 == y) <;> cases (p.1 == y) <;> cases (p.2 == x) <;> rfl
where
  List.any_congr {l : List (α × α)} {f g : α × α → Bool} (h : ∀ p ∈ l, f p = g p) :
      l.any f = l.any g := by
    induction l with
    | nil => rfl
    | cons a t ih =>
      simp only [List.any_cons]
      rw [h a (List.mem_cons_self ..), ih fun p hp => h p (List.mem_cons_of_mem _ hp)]

theorem adjOf_trueI {cr : List (α × α)} {x y : α} (h : IncLedger.adjOf cr x y = true) :
    ∃ p ∈ cr, (p.1 = x ∧ p.2 = y) ∨ (p.1 = y ∧ p.2 = x) := by
  unfold IncLedger.adjOf at h
  obtain ⟨p, hp, hh⟩ := List.any_eq_true.mp h
  refine ⟨p, hp, ?_⟩
  simpa using hh

theorem conn_eqvI {xs : List α} {cr : List (α × α)} {a b : α}
    (h : Conn (IncLedger.adjOf cr) xs a b) : Eqv (posPairsI xs cr) (xs.idxOf a) (xs.idxOf b) := by
  induction h with
  | refl => exact .refl _
  | step _ _ hbc ih =>
    obtain ⟨p, hp, ⟨e1, e2⟩ | ⟨e1, e2⟩⟩ := adjOf_trueI hbc
    · have : Eqv (posPairsI xs cr) (xs.idxOf p.1) (xs.idxOf p.2) :=
        Eqv.edge (e := (xs.idxOf p.1, xs.idxOf p.2)) (List.mem_map.mpr ⟨p, hp, rfl⟩)
      rw [e1, e2] at this; exact ih.trans this
    · have : Eqv (posPairsI xs cr) (xs.idxOf p.1) (xs.idxOf p.2) :=
        Eqv.edge (e := (xs.idxOf p.1, xs.idxOf p.2)) (List.mem_map.mpr ⟨p, hp, rfl⟩)
      rw [e1, e2] at this; exact ih.trans this.symm

theorem posPairsI_in {xs : List α} {cr : List (α × α)} (hcr : ∀ p ∈ cr, p.1 ∈ xs ∧ p.2 ∈ xs) :
    ∀ e ∈ posPairsI xs cr, e.1 < xs.length ∧ e.2 < xs.length := by
  intro e he
  obtain ⟨p, hp, rfl⟩ := List.mem_map.mp he
  exact ⟨List.idxOf_lt_length_iff.mpr (hcr p hp).1, List.idxOf_lt_length_iff.mpr (hcr p hp).2⟩

theorem eqv_connI {xs : List α} {cr : List (α × α)} (hcr : ∀ p ∈ cr, p.1 ∈ xs ∧ p.2 ∈ xs)
    {y z : Nat} (h : Eqv (posPairsI xs cr) y z) (hy : y < xs.length) (hz : z < xs.length) :
    Conn (IncLedger.adjOf cr) xs xs[y] xs[z] := by
  have hin := posPairsI_in hcr
  induction h with
  | refl => exact .refl
  | edge he =>
    rename_i e
    obtain ⟨p, hp, rfl⟩ := List.mem_map.mp he
    simp only [List.getElem_idxOf]
    refine Conn.step .refl (hcr p hp).2 ?_
    unfold IncLedger.adjOf
    exact List.any_eq_true.mpr ⟨p, hp, by simp⟩
  | symm h' ih =>
    exact Conn.symm (adjOf_symmI cr) (List.getElem_mem _) (ih hz hy)
  | trans h1 _ ih1 ih2 =>
    have hm := (Eqv.range hin h1).elim (fun e => e ▸ hy) (fun e => e.2)
    exact (ih1 hy hm).trans (ih2 hm hz)

/-- **The least position names the class as `head` does.** -/
theorem head_leastI {xs : List α} (hnd : xs.Nodup) {cr : List (α × α)}
    (hcr : ∀ p ∈ cr, p.1 ∈ xs ∧ p.2 ∈ xs) {R : Nat → Nat} (hl : Least (posPairsI xs cr) xs.length R)
    {t : Nat} (ht : t < xs.length) :
    head (IncLedger.adjOf cr) xs xs[t] = some (xs[R t]'(hl t ht).1) := by
  obtain ⟨hR, hcon, hmin⟩ := hl t ht
  unfold head
  simp only
  rw [List.find?_eq_some_iff_getElem]
  have hmem : xs[t] ∈ xs := List.getElem_mem _
  refine ⟨?_, R t, hR, rfl, fun j hj => ?_⟩
  · rw [@List.contains_iff_mem _ instBEqOfDecidableEq, cls_spec _ hnd hmem]
    exact eqv_connI hcr hcon ht hR
  · rw [Bool.not_eq_true']
    cases hcb : @List.contains _ instBEqOfDecidableEq (cls (IncLedger.adjOf cr) xs xs[t]) xs[j]
    · rfl
    exfalso
    have hc := (cls_spec _ hnd hmem _).mp ((@List.contains_iff_mem _ instBEqOfDecidableEq).mp hcb)
    have := conn_eqvI hc
    rw [List.Nodup.idxOf_getElem hnd, List.Nodup.idxOf_getElem hnd] at this
    have := hmin j this
    omega

end ForgeBoolCore.UF
