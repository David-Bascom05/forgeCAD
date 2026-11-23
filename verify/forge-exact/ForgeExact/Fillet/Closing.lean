import ForgeExact.Fillet.Basic

/-!
# The closing with a ball

general-method.md §1.2. For a solid `S` and a radius `r > 0`:

* `Free S r c`: the open ball about `c` misses `S` (the centre set M);
* `closing S r`: the points in no free open ball, i.e. `(S ⊕ B_r) ⊖ B_r`;
* `fill S r`: what the closing adds, `closing S r \ S`;
* `opening S r`: the dual, `ℝ³ \ closing (ℝ³ \ S) r`, which a convex round takes.

Everything is squared distance (`V.d2`), so the definitions are first-order over the field.
-/

namespace ForgeExact.Fillet

open Lean.Grind

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

/-- The open ball about `c` misses `S`. -/
def Free (S : Pred (V K)) (r : K) (c : V K) : Prop := ∀ y, S y → r * r ≤ V.d2 y c

/-- The closing: the points no free open ball holds. -/
def closing (S : Pred (V K)) (r : K) : Pred (V K) := fun x => ∀ c, Free S r c → r * r ≤ V.d2 x c

/-- The fill of the closing. -/
def fill (S : Pred (V K)) (r : K) : Pred (V K) := fun x => closing S r x ∧ ¬S x

/-- The opening: the dual of the closing. -/
def opening (S : Pred (V K)) (r : K) : Pred (V K) := fun x => ¬closing (fun y => ¬S y) r x

/-- What the opening takes away. -/
def cut (S : Pred (V K)) (r : K) : Pred (V K) := fun x => S x ∧ ¬opening S r x

theorem closing_extensive (S : Pred (V K)) (r : K) : S.Sub (closing S r) :=
  fun x hx _ hc => hc x hx

theorem free_anti {S T : Pred (V K)} {r : K} (h : S.Sub T) {c : V K} (hc : Free T r c) : Free S r c :=
  fun y hy => hc y (h y hy)

theorem closing_mono {S T : Pred (V K)} {r : K} (h : S.Sub T) : (closing S r).Sub (closing T r) :=
  fun _ hx c hc => hx c (free_anti h hc)

/-- A centre free of `S` is free of its closing: the free balls are the same. -/
theorem free_closing {S : Pred (V K)} {r : K} {c : V K} (hc : Free S r c) : Free (closing S r) r c :=
  fun _ hy => hy c hc

/-- The closing is idempotent. -/
theorem closing_idem (S : Pred (V K)) (r : K) : closing (closing S r) r = closing S r :=
  Pred.ext fun x => ⟨fun h c hc => h c (free_closing hc), fun h => closing_extensive _ r x h⟩

/-- The opening lies inside the solid. -/
theorem opening_sub (S : Pred (V K)) (r : K) : (opening S r).Sub S := by
  intro x hx
  rcases Classical.em (S x) with h | h
  · exact h
  · exact absurd (closing_extensive (fun y => ¬S y) r x h) hx

/-- The fill and the cut are the same object seen from the two sides. -/
theorem cut_eq_fill (S : Pred (V K)) (r : K) : cut S r = fill (fun y => ¬S y) r :=
  Pred.ext fun x => by
    simp only [cut, fill, opening]
    exact ⟨fun ⟨h1, h2⟩ => ⟨Classical.byContradiction h2, fun h => h h1⟩,
      fun ⟨h1, h2⟩ => ⟨Classical.byContradiction h2, fun h => h h1⟩⟩

/-- **Locality** (general-method.md O2): whether `x` is in the closing depends only on `S` within
`2r` of `x`. Two solids that agree on the ball of squared radius `4 r²` about `x` close alike
there. (A free ball that holds `x` lies in that neighbourhood; the triangle inequality, squared:
`d2 y x ≤ 2 d2 y c + 2 d2 x c`.) -/
theorem closing_local {S T : Pred (V K)} {r : K} {x : V K}
    (h : ∀ y, V.d2 y x < 4 * (r * r) → (S y ↔ T y)) : closing S r x ↔ closing T r x := by
  have key : ∀ {A B : Pred (V K)}, (∀ y, V.d2 y x < 4 * (r * r) → (A y ↔ B y)) →
      closing A r x → closing B r x := by
    intro A B hAB hA c hc
    rcases le_or_lt' (r * r) (V.d2 x c) with h1 | h1
    · exact h1
    · exfalso
      -- c holds x; then c is free of A too, by the parallelogram bound
      have hfree : Free A r c := by
        intro y hy
        rcases le_or_lt' (r * r) (V.d2 y c) with h2 | h2
        · exact h2
        · have par : V.d2 y x ≤ 2 * V.d2 y c + 2 * V.d2 x c := by
            have := sq_nonneg' (y.x + x.x - 2 * c.x)
            have := sq_nonneg' (y.y + x.y - 2 * c.y)
            have := sq_nonneg' (y.z + x.z - 2 * c.z)
            simp only [V.d2_def]; grind
          have hy' := (hAB y (by grind)).mp hy
          exact absurd (hc y hy') (by grind)
      have := hA c hfree
      grind
  exact ⟨key h, key fun y hy => (h y hy).symm⟩

end ForgeExact.Fillet
