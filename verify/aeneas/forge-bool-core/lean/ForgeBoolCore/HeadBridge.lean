import ForgeBoolCore.UFModel
import ForgeExact.DecideOnce.Core

/-!
# Least position = the Lean's `head`

For a list `xs` without repeats and pairs `cr` of its members, the classes of `adjOf cr` over `xs`
(`Conn`) are the classes of the positions' pairs (`Eqv`). So a union–find naming each class by its
least position (`Least`) names it by its first member in list order, which is
`head (adjOf cr) xs`.
-/

namespace ForgeBoolCore.UF

open ForgeExact.DecideOnce ForgeExact.DecideOnce.Ledger

/-- The pairs by position. -/
def posPairs (xs : List Cov) (cr : List (Cov × Cov)) : List (Nat × Nat) :=
  cr.map fun p => (xs.idxOf p.1, xs.idxOf p.2)

theorem adjOf_symm (cr : List (Cov × Cov)) (x y : Cov) : adjOf cr x y = adjOf cr y x := by
  unfold adjOf
  apply List.any_congr
  intro p _
  cases (p.1 == x) <;> cases (p.2 == y) <;> cases (p.1 == y) <;> cases (p.2 == x) <;> rfl
where
  List.any_congr {l : List (Cov × Cov)} {f g : Cov × Cov → Bool} (h : ∀ p ∈ l, f p = g p) :
      l.any f = l.any g := by
    induction l with
    | nil => rfl
    | cons a t ih =>
      simp only [List.any_cons]
      rw [h a (List.mem_cons_self ..), ih fun p hp => h p (List.mem_cons_of_mem _ hp)]

theorem adjOf_true {cr : List (Cov × Cov)} {x y : Cov} (h : adjOf cr x y = true) :
    ∃ p ∈ cr, (p.1 = x ∧ p.2 = y) ∨ (p.1 = y ∧ p.2 = x) := by
  unfold adjOf at h
  obtain ⟨p, hp, hh⟩ := List.any_eq_true.mp h
  refine ⟨p, hp, ?_⟩
  simpa using hh

theorem conn_eqv {xs : List Cov} {cr : List (Cov × Cov)} {a b : Cov}
    (h : Conn (adjOf cr) xs a b) : Eqv (posPairs xs cr) (xs.idxOf a) (xs.idxOf b) := by
  induction h with
  | refl => exact .refl _
  | step _ _ hbc ih =>
    obtain ⟨p, hp, ⟨e1, e2⟩ | ⟨e1, e2⟩⟩ := adjOf_true hbc
    · have : Eqv (posPairs xs cr) (xs.idxOf p.1) (xs.idxOf p.2) :=
        Eqv.edge (e := (xs.idxOf p.1, xs.idxOf p.2)) (List.mem_map.mpr ⟨p, hp, rfl⟩)
      rw [e1, e2] at this; exact ih.trans this
    · have : Eqv (posPairs xs cr) (xs.idxOf p.1) (xs.idxOf p.2) :=
        Eqv.edge (e := (xs.idxOf p.1, xs.idxOf p.2)) (List.mem_map.mpr ⟨p, hp, rfl⟩)
      rw [e1, e2] at this; exact ih.trans this.symm

theorem posPairs_in {xs : List Cov} {cr : List (Cov × Cov)} (hcr : ∀ p ∈ cr, p.1 ∈ xs ∧ p.2 ∈ xs) :
    ∀ e ∈ posPairs xs cr, e.1 < xs.length ∧ e.2 < xs.length := by
  intro e he
  obtain ⟨p, hp, rfl⟩ := List.mem_map.mp he
  exact ⟨List.idxOf_lt_length_iff.mpr (hcr p hp).1, List.idxOf_lt_length_iff.mpr (hcr p hp).2⟩

theorem eqv_conn {xs : List Cov} {cr : List (Cov × Cov)} (hcr : ∀ p ∈ cr, p.1 ∈ xs ∧ p.2 ∈ xs)
    {y z : Nat} (h : Eqv (posPairs xs cr) y z) (hy : y < xs.length) (hz : z < xs.length) :
    Conn (adjOf cr) xs xs[y] xs[z] := by
  have hin := posPairs_in hcr
  induction h with
  | refl => exact .refl
  | edge he =>
    rename_i e
    obtain ⟨p, hp, rfl⟩ := List.mem_map.mp he
    simp only [List.getElem_idxOf]
    refine Conn.step .refl (hcr p hp).2 ?_
    unfold adjOf
    exact List.any_eq_true.mpr ⟨p, hp, by simp⟩
  | symm h' ih =>
    exact Conn.symm (adjOf_symm cr) (List.getElem_mem _) (ih hz hy)
  | trans h1 _ ih1 ih2 =>
    have hm := (Eqv.range hin h1).elim (fun e => e ▸ hy) (fun e => e.2)
    exact (ih1 hy hm).trans (ih2 hm hz)

/-- **The least position names the class as `head` does.** -/
theorem head_least {xs : List Cov} (hnd : xs.Nodup) {cr : List (Cov × Cov)}
    (hcr : ∀ p ∈ cr, p.1 ∈ xs ∧ p.2 ∈ xs) {R : Nat → Nat} (hl : Least (posPairs xs cr) xs.length R)
    {t : Nat} (ht : t < xs.length) :
    head (adjOf cr) xs xs[t] = some (xs[R t]'(hl t ht).1) := by
  obtain ⟨hR, hcon, hmin⟩ := hl t ht
  unfold head
  simp only
  rw [List.find?_eq_some_iff_getElem]
  have hmem : xs[t] ∈ xs := List.getElem_mem _
  refine ⟨?_, R t, hR, rfl, fun j hj => ?_⟩
  · rw [@List.contains_iff_mem _ instBEqOfDecidableEq, cls_spec _ hnd hmem]
    exact eqv_conn hcr hcon ht hR
  · rw [Bool.not_eq_true']
    cases hcb : @List.contains _ instBEqOfDecidableEq (cls (adjOf cr) xs xs[t]) xs[j]
    · rfl
    exfalso
    have hc := (cls_spec _ hnd hmem _).mp ((@List.contains_iff_mem _ instBEqOfDecidableEq).mp hcb)
    have := conn_eqv hc
    rw [List.Nodup.idxOf_getElem hnd, List.Nodup.idxOf_getElem hnd] at this
    have := hmin j this
    omega

end ForgeBoolCore.UF
