import ForgeExact.DecideOnce.Correct

/-!
# Polyhedral solids as formulas: `compat` proven, and the two-body boolean

`Exact.compat` asks each solid's membership to be constant on every cell of the arrangement. For
a B-rep classified by ray parity that is RefFacts' one assumption (Jordan–Brouwer). For a solid
given as a boolean formula over open half-spaces (`Flat.HForm`, Nef's polyhedra: every flat-faced
solid is one) whose planes are carriers of the arrangement in either sense, it is proven here
(`nef_compat`), on every cell, not only the full ones: all the arrangement's planes keep their
signs across a cell, so every half-space keeps its membership.

`correct_bool`: for two operands and union, intersection or difference, theorem (c)'s regularised
union is `regBool`, the spec's regularised boolean of the two solids.
-/

namespace ForgeExact.DecideOnce

open ForgeExact ForgeExact.Ref Topology Ledger

theorem sgnQ_pos_iff (q : Rat) : sgnQ q = .pos ↔ q > 0 := by
  unfold sgnQ
  by_cases h1 : q > 0
  · simp [h1]
  · by_cases h2 : q < 0 <;> simp [h1, h2]

theorem sgnQ_neg_iff (q : Rat) : sgnQ q = .neg ↔ q < 0 := by
  unfold sgnQ
  by_cases h1 : q > 0
  · simp only [h1, ite_true, reduceCtorEq, false_iff]; grind
  · by_cases h2 : q < 0 <;> simp [h1, h2]

/-- Two points of one cell are on the same side of every plane of the arrangement. -/
theorem same_signs : ∀ {ps : List Plane} {T : Cov} {x y : V3}, covCell ps T x → covCell ps T y →
    ∀ q ∈ ps, sgnQ (q.eval x) = sgnQ (q.eval y)
  | [], [], _, _, _, _, q, hq => nomatch hq
  | p :: ps, s :: T, x, y, ⟨hx, hx'⟩, ⟨hy, hy'⟩, q, hq => by
    rcases List.mem_cons.mp hq with rfl | hq
    · rw [hx, hy]
    · exact same_signs hx' hy' q hq
  | [], _ :: _, _, _, h, _, _, _ => h.elim
  | _ :: _, [], _, _, h, _, _, _ => h.elim

/-- A formula's planes, each a carrier of the arrangement in one sense or the other. -/
def OnCarriers (ps : List Plane) (F : Flat.HForm) : Prop :=
  ∀ p ∈ F.planes, ∃ q ∈ ps, p = q ∨ p = q.neg

theorem den_const (ps : List Plane) {T : Cov} {x y : V3} (hx : covCell ps T x) (hy : covCell ps T y) :
    ∀ F : Flat.HForm, OnCarriers ps F → (F.den x ↔ F.den y)
  | .half p, h => by
    obtain ⟨q, hq, e⟩ := h p (by simp [Flat.HForm.planes])
    have hs := same_signs hx hy q hq
    simp only [Flat.HForm.den]
    rcases e with rfl | rfl
    · rw [← sgnQ_pos_iff, ← sgnQ_pos_iff, hs]
    · rw [Flat.eval_neg, Flat.eval_neg]
      have : ∀ v : Rat, -v > 0 ↔ v < 0 := fun v => by grind
      rw [this, this, ← sgnQ_neg_iff, ← sgnQ_neg_iff, hs]
  | .not f, h => by
    simp only [Flat.HForm.den]
    rw [den_const ps hx hy f h]
  | .and f g, h => by
    simp only [Flat.HForm.den]
    rw [den_const ps hx hy f fun p hp => h p (List.mem_append_left _ hp),
      den_const ps hx hy g fun p hp => h p (List.mem_append_right _ hp)]
  | .or f g, h => by
    simp only [Flat.HForm.den]
    rw [den_const ps hx hy f fun p hp => h p (List.mem_append_left _ hp),
      den_const ps hx hy g fun p hp => h p (List.mem_append_right _ hp)]

/-- **`compat` for polyhedral solids.** -/
theorem nef_compat (ps : List Plane) (F : Nat → Flat.HForm) (hF : ∀ i, OnCarriers ps (F i)) :
    ∀ T i, (covCell ps T).Sub (F i).den ∨ ∀ x, covCell ps T x → ¬(F i).den x := by
  intro T i
  by_cases h : ∃ x, covCell ps T x ∧ (F i).den x
  · obtain ⟨x, hx, hd⟩ := h
    exact Or.inl fun y hy => (den_const ps hx hy (F i) (hF i)).mp hd
  · exact Or.inr fun x hx hd => h ⟨x, hx, hd⟩

/-! ## Two operands -/

/-- Union, intersection and difference as rules on the membership vector (body 0 first). -/
def opRule : Op → Rule
  | .union => fun m => m 0 || m 1
  | .inter => fun m => m 0 && m 1
  | .diff => fun m => m 0 && !m 1

theorem combine_opRule {L : Ledger} (hnb : L.nb = 2) (o : Op) (A : Nat → Pred V3) :
    combine (ruleOp (opRule o)) (opnds L A) = combine o.rule (pair (A 0) (A 1)) := by
  apply Pred.ext
  intro x
  have h0 : 0 < L.nb := by omega
  have h1 : 1 < L.nb := by omega
  simp only [combine, ruleOp, opnds, Op.rule, pair, ite_true]
  cases o <;> by_cases ha : A 0 x <;> by_cases hb : A 1 x <;> simp [opRule, Op.eval, ha, hb, h0, h1]

/-- **Theorem (c) for two solids**: the regularised union of the cells the exact boolean keeps is
the spec's `regBool`. -/
theorem correct_bool {L : Ledger} {ps : List Plane} {A : Nat → Pred V3} (h : Exact L ps A)
    (hnb : L.nb = 2) (o : Op) :
    (∀ φ, φ ∈ L.result (L.keepTab (opRule o)) ↔ XBoundary L ps A (opRule o) φ) ∧
    reg ((decomp h).kept fun T => xkeep L ps A (opRule o) T = true) = regBool o (A 0) (A 1) := by
  obtain ⟨h1, h2⟩ := correct h (opRule o)
  refine ⟨h1, ?_⟩
  rw [h2, combine_opRule hnb]
  rfl

end ForgeExact.DecideOnce
