import ForgeExact.DecideOnce.BRep

/-!
# The witness ray is right

`Exact.witness` asks each witness answer to be the solid's membership in the side cell. The
witness is the reference's ray parity (`Ref.Solid.contains`) at one point of the cell. Two facts
make that the membership of the whole cell:

* **a point of the side cell** (`side_point`): from any point `m` of a facet (a cell with one
  zero, at carrier `P`), the step `m ± δ n_P`, with `δ` under every other carrier's slack at `m`
  (`stepBound`), lies in the facet's plus (minus) side cell. This is RefFacts' step, for all the
  carriers at once;
* **the ray is constant on the cell** (`brep_compat`, from Jordan–Brouwer).

`witness_of_samples`: if the ledger's witness table holds exactly the side cells (and bodies) at
whose sample point the reference's ray says inside, every witness answer is the membership, i.e.
`Exact.witness` holds. With `side_point` the bridge has its samples: one point inside each piece
(which `pieceReal` already asks for), stepped off its carrier.
-/

namespace ForgeExact.DecideOnce

open ForgeExact ForgeExact.Ref Flat Ledger

/-- The largest step off `P` from `m` that crosses no other carrier (and at most 1). -/
def stepBound (P : Plane) (m : V3) : List Plane → Rat
  | [] => 1
  | q :: qs => qmin (if q.eval m = 0 then 1 else qabs (q.eval m) / (2 * (qabs (P.n.dot q.n) + 1)))
      (stepBound P m qs)

theorem stepBound_pos (P : Plane) (m : V3) : ∀ qs, 0 < stepBound P m qs
  | [] => by unfold stepBound; decide
  | q :: qs => by
    unfold stepBound
    have := stepBound_pos P m qs
    have h1 : 0 < (if q.eval m = 0 then 1 else qabs (q.eval m) / (2 * (qabs (P.n.dot q.n) + 1))) := by
      split
      · decide
      · rename_i h
        exact div_pos' (Jordan.qabs_pos h) (by have := qabs_nonneg (P.n.dot q.n); grind)
    generalize (if q.eval m = 0 then 1 else qabs (q.eval m) / (2 * (qabs (P.n.dot q.n) + 1))) = A at h1 ⊢
    unfold qmin; split <;> assumption

theorem stepBound_le (P : Plane) (m : V3) : ∀ qs, ∀ q ∈ qs, q.eval m ≠ 0 →
    stepBound P m qs ≤ qabs (q.eval m) / (2 * (qabs (P.n.dot q.n) + 1))
  | [], _, h, _ => nomatch h
  | r :: qs, q, hq, hne => by
    unfold stepBound
    rcases List.mem_cons.mp hq with rfl | hq
    · have := qmin_le_left (if q.eval m = 0 then 1 else qabs (q.eval m) / (2 * (qabs (P.n.dot q.n) + 1)))
        (stepBound P m qs)
      simp only [hne, ite_false] at this ⊢
      exact this
    · exact Rat.le_trans (qmin_le_right _ _) (stepBound_le P m qs q hq hne)

theorem sgnQ_step (P q : Plane) (m : V3) (δ : Rat) (hδ : 0 < δ)
    (hle : q.eval m ≠ 0 → δ ≤ qabs (q.eval m) / (2 * (qabs (P.n.dot q.n) + 1))) (s : Rat)
    (hs : s = 1 ∨ s = -1) (hne : q.eval m ≠ 0) :
    sgnQ (q.eval (m + (s * δ) * P.n)) = sgnQ (q.eval m) := by
  have h := RefFacts.step_keeps_sign P q m δ hne hδ (hle hne) s hs
  unfold sgnQ
  by_cases h1 : q.eval m > 0
  · simp [h1, h.1.mp h1]
  · by_cases h2 : q.eval m < 0
    · have : ¬q.eval (m + (s * δ) * P.n) > 0 := fun h' => h1 (h.1.mpr h')
      simp [h1, h2, h.2.mp h2, this]
    · exact absurd (by grind) hne

/-- A cell with no zero keeps its covector under a small step. -/
theorem step_full (P : Plane) (m : V3) (δ : Rat) (hδ : 0 < δ) (s : Rat) (hs : s = 1 ∨ s = -1) :
    ∀ (qs : List Plane) (T : Cov), (∀ q ∈ qs, q.eval m ≠ 0 →
      δ ≤ qabs (q.eval m) / (2 * (qabs (P.n.dot q.n) + 1))) →
      covCell qs T m → Sgn.zero ∉ T → covCell qs T (m + (s * δ) * P.n)
  | [], [], _, _, _ => trivial
  | q :: qs, t :: T, hb, ⟨ht, hT⟩, hz => by
    have hne : q.eval m ≠ 0 := by
      intro h0; apply hz; rw [← ht, h0]; simp [sgnQ]
    refine ⟨?_, step_full P m δ hδ s hs qs T (fun r hr => hb r (List.mem_cons_of_mem _ hr)) hT
      (fun h => hz (List.mem_cons_of_mem _ h))⟩
    rw [sgnQ_step P q m δ hδ (hb q (List.mem_cons_self ..)) s hs hne, ht]
  | [], _ :: _, _, h, _ => h.elim
  | _ :: _, [], _, h, _ => h.elim

theorem idxOf_cons_ne {a b : Sgn} {l : List Sgn} (h : a ≠ b) : (a :: l).idxOf b = l.idxOf b + 1 := by
  simp [List.idxOf_cons, h]

/-- **A point of the side cell.** From a point `m` of a facet whose zero is at carrier `P`, the
step `m + s δ n_P` (`s = 1` for the plus side, `-1` for the minus side) lies in that side cell. -/
theorem side_point (P : Plane) (hP : P.n ≠ ⟨0, 0, 0⟩) (m : V3) (s : Rat) (hs : s = 1 ∨ s = -1) :
    ∀ (qs : List Plane) (φ : Cov), covCell qs φ m → φ.count .zero = 1 →
      qs[φ.idxOf .zero]? = some P →
      ∀ δ, 0 < δ → (∀ q ∈ qs, q.eval m ≠ 0 → δ ≤ qabs (q.eval m) / (2 * (qabs (P.n.dot q.n) + 1))) →
      covCell qs (φ.set (φ.idxOf .zero) (if s = 1 then .pos else .neg)) (m + (s * δ) * P.n)
  | [], [], _, h1, _, _, _, _ => by simp at h1
  | q :: qs, t :: φ, ⟨ht, hφ⟩, h1, hP', δ, hδ, hb => by
    by_cases hz : t = .zero
    · subst hz
      have hi : (Sgn.zero :: φ).idxOf .zero = 0 := by simp
      rw [hi] at hP' ⊢
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hP'
      subst hP'
      simp only [List.set_cons_zero]
      have hφz : Sgn.zero ∉ φ := by
        intro hm
        have : 0 < φ.count .zero := List.count_pos_iff.mpr hm
        simp at h1; omega
      refine ⟨?_, step_full q m δ hδ s hs qs φ (fun r hr => hb r (List.mem_cons_of_mem _ hr)) hφ hφz⟩
      have h0 : q.eval m = 0 := by
        have := ht; unfold sgnQ at this; split at this <;> try (split at this)
        all_goals first | grind | cases this
      have hl := RefFacts.step_leaves_plane q hP m h0 δ hδ
      rcases hs with rfl | rfl
      · simp only [ite_true]; unfold sgnQ; rw [show (1 : Rat) * δ = δ by grind]; simp [hl.1]
      · have : ¬((-1 : Rat) = 1) := by decide
        simp only [this, ite_false]; unfold sgnQ
        have h2 := hl.2
        have : ¬q.eval (m + (-1 * δ) * q.n) > 0 := by grind
        simp [this, h2]
    · rw [idxOf_cons_ne (Ne.symm (fun h => hz h.symm))] at hP' ⊢
      simp only [List.getElem?_cons_succ] at hP'
      simp only [List.set_cons_succ]
      have h1' : φ.count .zero = 1 := by
        have : (t :: φ).count .zero = φ.count .zero := by simp [hz]
        omega
      have hne : q.eval m ≠ 0 := by intro h0; apply hz; rw [← ht, h0]; simp [sgnQ]
      refine ⟨?_, side_point P hP m s hs qs φ hφ h1' hP' δ hδ (fun r hr => hb r (List.mem_cons_of_mem _ hr))⟩
      rw [sgnQ_step P q m δ hδ (hb q (List.mem_cons_self ..)) s hs hne, ht]
  | [], _ :: _, h, _, _, _, _, _ => h.elim
  | _ :: _, [], h, _, _, _, _, _ => h.elim

/-! ## The witness -/

theorem mem_fulls : ∀ {n : Nat} {T : Cov}, T.length = n → Sgn.zero ∉ T → T ∈ fulls n
  | 0, [], _, _ => by simp [fulls]
  | n + 1, t :: T, hl, hz => by
    simp only [fulls, List.mem_flatMap, List.mem_cons, List.mem_nil_iff, or_false]
    refine ⟨T, mem_fulls (by simpa using hl) (fun h => hz (List.mem_cons_of_mem _ h)), ?_⟩
    cases t
    · exact Or.inr rfl
    · exact absurd (List.mem_cons_self ..) hz
    · exact Or.inl rfl

/-- **The witness is the membership.** If each witness answer is the reference's ray parity at a
point of its side cell (`sample`), and the solids are closed flat B-reps on the carriers, then
every witness answer is the solid's membership in the cell: `Exact.witness`. -/
theorem witness_of_samples (L : Ledger) (ps : List Plane) (S : Nat → Solid)
    (hflat : ∀ i, Jordan.FacesFlat (S i)) (hcl : ∀ i, Jordan.Closed (S i))
    (hpl : ∀ i, ∀ f ∈ Jordan.facesL (S i), Scaled ps f.plane)
    (sample : Cov → V3) (hs : ∀ T ∈ L.topes, covCell ps T (sample T))
    (hfull : ∀ T ∈ L.topes, Sgn.zero ∉ T)
    (hin : ∀ T ∈ L.topes, ∀ i, i < L.nb → (L.inside.contains (T, i) = true ↔ (S i).contains (sample T) = true)) :
    ∀ T ∈ L.topes, ∀ i, i < L.nb →
      (L.inside.contains (T, i) = true ↔ memT ps (fun i x => (S i).contains x = true) T i) := by
  intro T hT i hi
  rw [hin T hT i hi]
  have hTf : T ∈ fulls ps.length := mem_fulls (covCell_length (hs T hT)) (hfull T hT)
  constructor
  · intro h; exact ⟨sample T, hs T hT, h⟩
  · rintro ⟨x, hx, hc⟩
    rcases brep_compat ps S hflat hcl hpl T hTf i with hsub | hdis
    · exact hsub _ (hs T hT)
    · exact absurd hc (hdis x hx)

end ForgeExact.DecideOnce
