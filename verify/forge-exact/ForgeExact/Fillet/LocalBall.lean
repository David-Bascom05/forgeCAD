import ForgeExact.Fillet.Closing

/-!
# The local rolling ball: the owner's definition (2026-11-23)

The owner chose the **local rolling ball** for both kinds of fillet (lean-spec-design.md §13).
For a pick `e` with supports `F₁`, `F₂`:

* the obstacle is only the two supports carried on, as solids: `H = H₁ ∪ H₂`. No other generator
  blocks the ball's interior;
* a **centre** is a free centre of `H` whose ball touches each face patch: its springs lie inside the
  faces (`Foot`);
* the **local fill** is the closing's fill of `H`, at the points whose nearest free centre is such a
  centre (`Witness`): the shadow of the caps between the springs, by the crease form of the closing;
* the result is `reg (S ∪ fill)` for a concave pick, and the dual (`localCut`, the same on the
  complement) for a convex one. Interactions with other faces are events on the springs (P3 on
  `Foot`): cliff, roll-over, crowding and overlapping rounds, each a vertex of the crease graph.

`local_eq_closing`: **the closing is its theorem where no third generator is within reach.** If `S`
agrees with `H` within `2r` of `x`, and `x` has a witness, then `x` is in the local fill iff it is in
the fill of `S`'s own closing. So every closing theorem (plane–plane, the revolution corners, the
Pivot, TooLarge) holds for the local definition on its domain.
-/

namespace ForgeExact.Fillet

open Lean.Grind

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

/-- A pick: its two supports carried on as solids, and their face patches. -/
structure Pick (K : Type) [Field K] [LE K] [LT K] where
  H₁ : Pred (V K)
  H₂ : Pred (V K)
  F₁ : Pred (V K)
  F₂ : Pred (V K)

namespace Pick

variable (P : Pick K)

/-- The local obstacle: the two supports, and nothing else. -/
def H : Pred (V K) := Pred.union P.H₁ P.H₂

/-- The ball about `c` touches the face patch `F`: a spring point of `F` on its sphere. -/
def Foot (F : Pred (V K)) (r : K) (c : V K) : Prop := ∃ p, F p ∧ V.d2 p c = r * r

/-- A centre of the rolling ball: free of the two supports, its springs inside both faces. -/
def Centre (r : K) (c : V K) : Prop := Free P.H r c ∧ Foot P.F₁ r c ∧ Foot P.F₂ r c

/-- `c` is the nearest free centre of `x`, and a rolling ball's centre. -/
def Witness (r : K) (x c : V K) : Prop :=
  P.Centre r c ∧ ∀ c', Free P.H r c' → V.d2 x c ≤ V.d2 x c'

/-- **The local fill**: the closing's fill of the two supports where the nearest free centre is
a rolling ball's. -/
def localFill (r : K) : Pred (V K) := fun x => fill P.H r x ∧ ∃ c, P.Witness r x c

/-- The fillet of a concave pick on the solid `S`, before regularisation: `S ∪ localFill`. -/
def concave (S : Pred (V K)) (r : K) : Pred (V K) := Pred.union S (P.localFill r)

/-- The dual pick of a convex edge: the supports' complements carried on, the same faces. -/
def dual : Pick K where
  H₁ := fun x => ¬P.H₁ x
  H₂ := fun x => ¬P.H₂ x
  F₁ := P.F₁
  F₂ := P.F₂

/-- What a convex pick takes away: the local fill of the dual pick, inside `S`. -/
def localCut (S : Pred (V K)) (r : K) : Pred (V K) := fun x => S x ∧ P.dual.localFill r x

/-- The fillet of a convex pick on `S`, before regularisation: `S` less its cut. -/
def convex (S : Pred (V K)) (r : K) : Pred (V K) := fun x => S x ∧ ¬P.localCut S r x

/-- **The closing is the local ball's theorem where no third generator is within reach.** -/
theorem local_eq_closing {S : Pred (V K)} {r : K} {x : V K} (hr : 0 < r)
    (agree : ∀ y, V.d2 y x < 4 * (r * r) → (S y ↔ P.H y)) (hw : ∃ c, P.Witness r x c) :
    P.localFill r x ↔ fill S r x := by
  have hx : S x ↔ P.H x := agree x (by
    have : V.d2 x x = 0 := by simp only [V.d2_def]; grind
    rw [this]; have := mul_pos' hr hr; grind)
  have hc := closing_local (S := S) (T := P.H) (r := r) (x := x) agree
  simp only [localFill, fill]
  constructor
  · rintro ⟨⟨h1, h2⟩, _⟩
    exact ⟨hc.mpr h1, fun h => h2 (hx.mp h)⟩
  · rintro ⟨h1, h2⟩
    exact ⟨⟨hc.mp h1, fun h => h2 (hx.mpr h)⟩, hw⟩

end Pick

end ForgeExact.Fillet
