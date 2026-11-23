import ForgeExact.Fillet.PlanePlane

/-!
# Chamfers: the planar cut, the round's sibling

research/ops-spec/design.md §5. Stated on the fillet spec's `PlanePair` (Fillet/PlanePlane.lean):
two planes `aᵢ x = nᵢ·x − dᵢ` with unit normals, `c = n₁·n₂`, `c² < 1`, and the material
`S = {a₁ ≤ 0} ∪ {a₂ ≤ 0}`, a concave edge. A convex edge is the same statement for the complement,
`Fillet.cut_eq_fill`: the round's cut of a convex edge is the closing's fill of the concave one.

The kernel (chamfer.rs:848–942) puts a contact on each face at distance `d` from the edge in the
plane square to the edge, and joins the contacts by a plane holding the edge's direction. The
contact on face 1 (`a₁ = 0`) is the line `a₂ = δ₁`; on face 2 the line `a₁ = δ₂`. So the chamfer's
plane is `δ₂ a₂ + δ₁ a₁ = δ₁ δ₂`, and the wedge it fills (concave) or cuts (convex) is
`chamferFill δ₁ δ₂`.

* `contact_dist`: a point of face 1 at level `a₂ = δ` is at squared distance `δ² / (1 − c²)` from
  the edge: `(1 − c²) |w|² − (w·t)² = δ²`. So the kernel's distance `d` is the level
  `δ = d √(1 − c²)`; two-distance is `(δ₁, δ₂)`, equal distance `δ₁ = δ₂`.
* C1 `spring_level`: the round of radius `r` has its spring on face 1 at level `a₂ = r (1 − c)`,
  so the equal chamfer through the round's springs is the one at `δ = r (1 − c)`, i.e. at distance
  `d` with `d² (1 − c²) = r² (1 − c)²` — `d = r cot(θ/2)` for the dihedral `θ`, with no trig.
* C2 `fill_sub_chamfer`: **the round's fill lies inside the chamfer's**: the chamfer through the
  springs fills the round's fill and the circular segment beyond it. Dually (convex) the round's
  cut lies inside the chamfer's cut.
-/

namespace ForgeExact.Ops

open ForgeExact Fillet Lean.Grind

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

namespace Chamfer

variable (P : PlanePair K)

/-- The wedge a chamfer with contact levels `δ₁` (on face 1) and `δ₂` (on face 2) fills at a
concave edge (and cuts, for the complement, at a convex one). -/
def fill (δ₁ δ₂ : K) (x : V K) : Prop :=
  0 < P.a₁ x ∧ 0 < P.a₂ x ∧ δ₂ * P.a₂ x + δ₁ * P.a₁ x ≤ δ₁ * δ₂

/-- The contacts lie on the chamfer's plane. -/
theorem contacts_on_plane (δ₁ δ₂ : K) (x y : V K) (hx₁ : P.a₁ x = 0) (hx₂ : P.a₂ x = δ₁)
    (hy₁ : P.a₁ y = δ₂) (hy₂ : P.a₂ y = 0) :
    δ₂ * P.a₂ x + δ₁ * P.a₁ x = δ₁ * δ₂ ∧ δ₂ * P.a₂ y + δ₁ * P.a₁ y = δ₁ * δ₂ := by
  rw [hx₁, hx₂, hy₁, hy₂]; constructor <;> grind

/-- **The contact's distance.** A point `x` of face 1 at level `a₂ x = δ`, and any point `e` of
the edge: `(1 − c²) |x − e|² − ((x − e)·t)² = δ²`. With `e` the foot of `x` on the edge
(`(x − e)·t = 0`) this is `(1 − c²) d² = δ²`. -/
theorem contact_dist (δ : K) (x e : V K) (hx₁ : P.a₁ x = 0) (hx₂ : P.a₂ x = δ)
    (he₁ : P.a₁ e = 0) (he₂ : P.a₂ e = 0) :
    (1 - P.c * P.c) * V.d2 x e - V.dot (V.sub x e) P.t * V.dot (V.sub x e) P.t = δ * δ := by
  have g := P.gram (V.sub x e)
  have e₁ : V.dot P.n₁ (V.sub x e) = 0 := by
    simp only [PlanePair.dot_sub, PlanePair.a₁] at *; grind
  have e₂ : V.dot P.n₂ (V.sub x e) = δ := by
    simp only [PlanePair.dot_sub, PlanePair.a₂] at *; grind
  rw [e₁, e₂] at g
  simp only [PlanePair.q, V.d2] at *
  grind

/-- **C1.** The round's spring on face 1 is the contact at level `r (1 − c)`. -/
theorem spring_level {r : K} (c₀ : V K) (h₁ : P.a₁ c₀ = r) (h₂ : P.a₂ c₀ = r) (l : K) :
    let p := V.add (V.sub c₀ (V.smul r P.n₁)) (V.smul l P.t)
    P.a₁ p = 0 ∧ P.a₂ p = r * (1 - P.c) := by
  intro p
  obtain ⟨hp₁, _, hp₂⟩ := P.spring₁ c₀ h₁ h₂ l
  refine ⟨hp₁, ?_⟩
  simp only [p] at *; rw [hp₁] at hp₂; grind

/-- C1, as a distance: the equal chamfer through the round's springs is at distance `d` with
`(1 − c²) d² = r² (1 − c)²`. -/
theorem spring_dist {r : K} (c₀ e : V K) (h₁ : P.a₁ c₀ = r) (h₂ : P.a₂ c₀ = r) (l : K)
    (he₁ : P.a₁ e = 0) (he₂ : P.a₂ e = 0) :
    let p := V.add (V.sub c₀ (V.smul r P.n₁)) (V.smul l P.t)
    V.dot (V.sub p e) P.t = 0 →
      (1 - P.c * P.c) * V.d2 p e = (r * (1 - P.c)) * (r * (1 - P.c)) := by
  intro p hfoot
  obtain ⟨hp₁, hp₂⟩ := spring_level P c₀ h₁ h₂ l
  have := contact_dist P (r * (1 - P.c)) p e hp₁ hp₂ he₁ he₂
  rw [hfoot] at this; grind

/-- The algebra of C2: in the crease's cone, past the springs' chord, the point is inside the
ball, `q b < r² (1 − c²)`. -/
theorem q_lt_of_past_chord {r b₁ b₂ : K} (_hr : 0 < r) (hc : -1 < P.c ∧ P.c < 1)
    (hA : 0 < b₁ - P.c * b₂) (hB : 0 < b₂ - P.c * b₁)
    (hpast : b₁ + b₂ < r * (1 + P.c)) :
    P.q b₁ b₂ < r * r * (1 - P.c * P.c) := by
  have pos : 0 < 1 - P.c * P.c := P.one_sub_pos
  -- (1 − c²) q = A² + B² + 2cAB, with A, B the cone coordinates
  have iden : (1 - P.c * P.c) * P.q b₁ b₂ =
      (b₁ - P.c * b₂) * (b₁ - P.c * b₂) + (b₂ - P.c * b₁) * (b₂ - P.c * b₁)
        + 2 * P.c * ((b₁ - P.c * b₂) * (b₂ - P.c * b₁)) := by
    simp only [PlanePair.q]; grind
  have hAB : 0 < (b₁ - P.c * b₂) * (b₂ - P.c * b₁) := mul_pos' hA hB
  -- 2cAB ≤ 2AB: A² + B² + 2cAB < (A + B)²
  have h1 : 2 * P.c * ((b₁ - P.c * b₂) * (b₂ - P.c * b₁))
      < 2 * ((b₁ - P.c * b₂) * (b₂ - P.c * b₁)) := by
    have := mul_lt_mul_of_pos_left' (a := P.c) (b := 1) (c := (b₁ - P.c * b₂) * (b₂ - P.c * b₁))
      hc.2 hAB
    grind
  -- A + B = (1 − c)(b₁ + b₂) < r (1 − c²)
  have hsum : (b₁ - P.c * b₂) + (b₂ - P.c * b₁) < r * (1 - P.c * P.c) := by
    have := mul_lt_mul_of_pos_left' (a := b₁ + b₂) (b := r * (1 + P.c)) (c := 1 - P.c)
      hpast (by grind)
    grind
  have hsq := mul_self_lt_mul_self' (a := (b₁ - P.c * b₂) + (b₂ - P.c * b₁))
    (b := r * (1 - P.c * P.c)) (by grind) hsum
  -- so (1 − c²) q < r² (1 − c²)², and divide by 1 − c² > 0
  have : (1 - P.c * P.c) * P.q b₁ b₂ < (1 - P.c * P.c) * (r * r * (1 - P.c * P.c)) := by
    rw [iden]; grind
  rcases le_or_lt' (r * r * (1 - P.c * P.c)) (P.q b₁ b₂) with h | h
  · have := mul_le_mul_of_nonneg_left' h (le_of_lt pos)
    grind
  · exact h
where le_of_lt {a b : K} (h : a < b) : a ≤ b := by grind

/-- **C2. The round's fill lies in the chamfer's through its springs.** -/
theorem fill_sub_chamfer {r : K} (hr : 0 < r) :
    (Fillet.fill P.S r).Sub (fill P (r * (1 - P.c)) (r * (1 - P.c))) := by
  intro x ⟨hcl, hnS⟩
  have hc := abs_lt_one_of_sq (show P.c * P.c < 1 from P.skew)
  have ha : 0 < P.a₁ x ∧ 0 < P.a₂ x := by
    simp only [PlanePair.S] at hnS
    constructor <;> (apply Classical.byContradiction; intro h; apply hnS; grind)
  rcases (P.closing_iff hr x).mp hcl with hS | ⟨hcone, hq⟩
  · exact absurd hS hnS
  · refine ⟨ha.1, ha.2, ?_⟩
    have hu : 0 < r * (1 - P.c) := mul_pos' hr (by grind)
    -- it suffices that a₁ + a₂ ≤ r (1 − c)
    suffices hs : P.a₁ x + P.a₂ x ≤ r * (1 - P.c) by
      have := mul_le_mul_of_nonneg_left' hs (le_of_lt hu)
      grind
    rcases le_or_lt' (P.a₁ x + P.a₂ x) (r * (1 - P.c)) with h | h
    · exact h
    · exfalso
      simp only [PlanePair.Cone] at hcone
      have := q_lt_of_past_chord P hr hc (b₁ := r - P.a₁ x) (b₂ := r - P.a₂ x)
        (by grind) (by grind) (by grind)
      grind
where le_of_lt {a b : K} (h : a < b) : a ≤ b := by grind

end Chamfer

end ForgeExact.Ops
