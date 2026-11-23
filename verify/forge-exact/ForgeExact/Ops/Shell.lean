import ForgeExact.Fillet.PlanePlane

/-!
# Shell: the sharp inward offset, and the erosion it equals at convex edges

research/ops-spec/design.md §3. The kernel offsets each closed face by `t` and re-intersects the
offset faces (a sharp join). The ball spec's inward offset of a solid is its **erosion**: the points
of `M` whose open `t`-ball misses the closed faces `K` (`Fillet.Free K t`).

At a convex edge of two closed faces, with `M = {a₁ ≤ 0} ∩ {a₂ ≤ 0}` (the fillet spec's
`PlanePair`; unit normals, not parallel):

* `faces`: `K` is the two faces, `{a₁ = 0, a₂ ≤ 0} ∪ {a₂ = 0, a₁ ≤ 0}`;
* S1 `erosion_eq_sharp`: for `x ∈ M` and `t > 0`, `Free K t x ↔ a₁ x ≤ −t ∧ a₂ x ≤ −t`. So the
  erosion is exactly the sharp offset `{a₁ ≤ −t} ∩ {a₂ ≤ −t}`: at convex edges the kernel's sharp
  join *is* the exact inward offset, and the shell `M \ C` has constant thickness `t`.

The nearest face point is the foot on the nearer plane: it lies on its face because the edge is
convex (`a_j` at the foot is `a_j x + s c ≤ −s (1 − c) ≤ 0`).
-/

namespace ForgeExact.Ops

open ForgeExact Fillet Lean.Grind

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

namespace Shell

variable (P : PlanePair K)

/-- The material of a convex edge. -/
def M (x : V K) : Prop := P.a₁ x ≤ 0 ∧ P.a₂ x ≤ 0

/-- Its two faces, both closed (none opened). -/
def faces (y : V K) : Prop := (P.a₁ y = 0 ∧ P.a₂ y ≤ 0) ∨ (P.a₂ y = 0 ∧ P.a₁ y ≤ 0)

/-- A plane's level differs by at most the distance: `(a y − a x)² ≤ |y − x|²`. -/
theorem level_le_d2 (n : V K) (hn : V.dot n n = 1) (d : K) (x y : V K) :
    (V.dot n y - d - (V.dot n x - d)) * (V.dot n y - d - (V.dot n x - d)) ≤ V.d2 y x := by
  have := PlanePair.cs_unit hn (V.sub y x)
  rw [PlanePair.dot_sub] at this
  simp only [V.d2]; grind

/-- The foot of `x` on a plane `n·y = d + 0` with unit `n`: `x − a(x) n`. -/
theorem foot_d2 (n : V K) (hn : V.dot n n = 1) (s : K) (x : V K) :
    V.d2 (V.add x (V.smul s n)) x = s * s := by
  simp only [V.d2_def, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y, V.smul_z]
  simp only [V.dot_def] at hn
  grind

/-- **S1.** At a convex edge the erosion is the sharp offset. -/
theorem erosion_eq_sharp {t : K} (ht : 0 < t) {x : V K} (hx : M P x) :
    Free (faces P) t x ↔ P.a₁ x ≤ -t ∧ P.a₂ x ≤ -t := by
  have cb := abs_lt_one_of_sq (show P.c * P.c < 1 from P.skew)
  have u₁ := P.unit₁
  have u₂ := P.unit₂
  have hm : P.a₁ x ≤ 0 ∧ P.a₂ x ≤ 0 := hx
  -- the levels along the two normals
  have lv₁ : ∀ s, P.a₁ (V.add x (V.smul s P.n₁)) = P.a₁ x + s := by
    intro s; simp only [PlanePair.a₁, PlanePair.dot_add, PlanePair.dot_smul]; rw [u₁]; grind
  have lv₂ : ∀ s, P.a₂ (V.add x (V.smul s P.n₁)) = P.a₂ x + s * P.c := by
    intro s; simp only [PlanePair.a₂, PlanePair.dot_add, PlanePair.dot_smul, PlanePair.c]
    rw [Draft_dot_comm P.n₂ P.n₁]; grind
  have lv₁' : ∀ s, P.a₁ (V.add x (V.smul s P.n₂)) = P.a₁ x + s * P.c := by
    intro s; simp only [PlanePair.a₁, PlanePair.dot_add, PlanePair.dot_smul, PlanePair.c]; grind
  have lv₂' : ∀ s, P.a₂ (V.add x (V.smul s P.n₂)) = P.a₂ x + s := by
    intro s; simp only [PlanePair.a₂, PlanePair.dot_add, PlanePair.dot_smul]; rw [u₂]; grind
  constructor
  · intro hf
    -- the nearer plane's foot is a face point at distance s = min(−a₁, −a₂)
    rcases le_or_lt' (P.a₂ x) (P.a₁ x) with h | h
    · -- face 1 is nearer: s = −a₁ x
      have hy := hf (V.add x (V.smul (-P.a₁ x) P.n₁)) (Or.inl ⟨by rw [lv₁]; grind, by
        rw [lv₂]
        have : 0 ≤ -P.a₁ x * (1 - P.c) := mul_nonneg' (by grind) (by grind)
        grind⟩)
      rw [foot_d2 P.n₁ u₁] at hy
      have hs : t ≤ -P.a₁ x := by
        rcases le_or_lt' t (-P.a₁ x) with h1 | h1
        · exact h1
        · have := mul_self_lt_mul_self' (show 0 ≤ -P.a₁ x by have := hm.1; grind) h1; grind
      grind
    · have hy := hf (V.add x (V.smul (-P.a₂ x) P.n₂)) (Or.inr ⟨by rw [lv₂']; grind, by
        rw [lv₁']
        have : 0 ≤ -P.a₂ x * (1 - P.c) := mul_nonneg' (by grind) (by grind)
        grind⟩)
      rw [foot_d2 P.n₂ u₂] at hy
      have hs : t ≤ -P.a₂ x := by
        rcases le_or_lt' t (-P.a₂ x) with h1 | h1
        · exact h1
        · have := mul_self_lt_mul_self' (show 0 ≤ -P.a₂ x by have := hm.2; grind) h1; grind
      grind
  · rintro ⟨h₁, h₂⟩ y hy
    rcases hy with ⟨hy₁, _⟩ | ⟨hy₂, _⟩
    · have := level_le_d2 P.n₁ u₁ P.d₁ x y
      have e : V.dot P.n₁ y - P.d₁ = 0 := hy₁
      rw [e] at this
      have := mul_self_le_mul_self' (le_of_lt ht) (show t ≤ -(V.dot P.n₁ x - P.d₁) by
        simp only [PlanePair.a₁] at h₁; grind)
      grind
    · have := level_le_d2 P.n₂ u₂ P.d₂ x y
      have e : V.dot P.n₂ y - P.d₂ = 0 := hy₂
      rw [e] at this
      have := mul_self_le_mul_self' (le_of_lt ht) (show t ≤ -(V.dot P.n₂ x - P.d₂) by
        simp only [PlanePair.a₂] at h₂; grind)
      grind
where
  le_of_lt {a b : K} (h : a < b) : a ≤ b := by grind
  Draft_dot_comm (p q : V K) : V.dot p q = V.dot q p := by simp only [V.dot_def]; grind

end Shell

end ForgeExact.Ops
