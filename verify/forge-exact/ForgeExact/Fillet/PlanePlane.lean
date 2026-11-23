import ForgeExact.Fillet.Closing

/-!
# Plane against plane: the blend is a cylinder

The first exact surface of the spec (lean-spec-design.md §3, item 3; general-method.md §9,
"plain pipe"). Two planes with unit normals `n₁`, `n₂` (`c = n₁·n₂`, `c² < 1`), and the material
on the far side of either, `S = {a₁ ≤ 0} ∪ {a₂ ≤ 0}` with `aᵢ x = nᵢ·x − dᵢ`: a concave edge, as
the closing sees it (the edge's wedge is the empty side `a₁ > 0 ∧ a₂ > 0`).

* `free_iff`: a ball of radius `r` misses `S` iff its centre is at least `r` from both planes, i.e.
  on the far side of both offset planes `aᵢ = r`. The free centres are the offset wedge, and the
  crease of its boundary is the line `L` where the offsets meet.
* `closing_iff`: **the closing in closed form.** With `bᵢ = r − aᵢ x` and
  `q b = b₁² − 2c b₁b₂ + b₂²`, a point is in the closing iff it is in `S`, or it is in the cone of
  the crease (`c b₁ < b₂`, `c b₂ < b₁`: between the two normal planes through `L` and the springs)
  and `q b ≥ r² (1 − c²)`.
* `q_eq_cyl`: `q b = |t|² |w|² − (w·t)²` for `w = x − c₀`, `c₀ ∈ L`, `t = n₁ × n₂`, `|t|² = 1 − c²`:
  so `q b = r²(1 − c²)` is exactly the cylinder of radius `r` about `L`. The fill is bounded by that
  cylinder between the springs: **the blend is the cylinder**.
* `spring₁`: the line `c₀ − r n₁ + λ t` lies on face 1, on the cylinder, and on the cone's edge;
  `g1_spring₁`: the cylinder's normal there is `−r n₁`, the face's (G1).
-/

namespace ForgeExact.Fillet

open Lean.Grind

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

/-- Two planes with unit normals, not parallel. -/
structure PlanePair (K : Type) [Field K] [LE K] [LT K] where
  n₁ : V K
  n₂ : V K
  d₁ : K
  d₂ : K
  unit₁ : V.dot n₁ n₁ = 1
  unit₂ : V.dot n₂ n₂ = 1
  skew : V.dot n₁ n₂ * V.dot n₁ n₂ < 1

namespace PlanePair

variable (P : PlanePair K)

def c : K := V.dot P.n₁ P.n₂
def a₁ (x : V K) : K := V.dot P.n₁ x - P.d₁
def a₂ (x : V K) : K := V.dot P.n₂ x - P.d₂
/-- The material of a concave edge between the two planes. -/
def S : Pred (V K) := fun x => P.a₁ x ≤ 0 ∨ P.a₂ x ≤ 0
/-- The edge's direction. -/
def t : V K := V.cross P.n₁ P.n₂

def q (b₁ b₂ : K) : K := b₁ * b₁ - 2 * P.c * b₁ * b₂ + b₂ * b₂

/-- The crease's cone: between the normal planes through the crease line. -/
def Cone (r : K) (x : V K) : Prop :=
  P.c * (r - P.a₁ x) < r - P.a₂ x ∧ P.c * (r - P.a₂ x) < r - P.a₁ x

/-! ### Algebra -/

/-- Lagrange's identity: `|n|² |w|² − (n·w)² = |n × w|²`. -/
theorem lagrange (n w : V K) :
    V.dot n n * V.dot w w - V.dot n w * V.dot n w = V.dot (V.cross n w) (V.cross n w) := by
  simp only [V.dot_def, V.cross_x, V.cross_y, V.cross_z]; grind

theorem dot_self_nonneg (w : V K) : 0 ≤ V.dot w w := by
  have := sq_nonneg' w.x; have := sq_nonneg' w.y; have := sq_nonneg' w.z
  simp only [V.dot_def]; grind

/-- Cauchy–Schwarz for a unit vector. -/
theorem cs_unit {n : V K} (hn : V.dot n n = 1) (w : V K) : V.dot n w * V.dot n w ≤ V.dot w w := by
  have h := lagrange n w; rw [hn] at h; have := dot_self_nonneg (V.cross n w); grind

/-- The Gram determinant of `n₁, n₂, w`, which is `(w · (n₁ × n₂))²`. -/
theorem gram (w : V K) :
    V.dot w w * (1 - P.c * P.c) - P.q (V.dot P.n₁ w) (V.dot P.n₂ w)
      = V.dot w P.t * V.dot w P.t := by
  have u₁ := P.unit₁; have u₂ := P.unit₂
  simp only [q, c, t, V.dot_def, V.cross_x, V.cross_y, V.cross_z] at *
  grind

theorem one_sub_pos : 0 < 1 - P.c * P.c := by have := P.skew; simp only [c]; grind

theorem q_nonneg (u₁ u₂ : K) : 0 ≤ P.q u₁ u₂ := by
  have h1 := sq_nonneg' (u₁ - P.c * u₂)
  have h2 := mul_nonneg' (le_of_lt' P.one_sub_pos) (sq_nonneg' u₂)
  simp only [q]; grind
where le_of_lt' {a b : K} (h : a < b) : a ≤ b := by grind

theorem dot_sub (n x y : V K) : V.dot n (V.sub x y) = V.dot n x - V.dot n y := by
  simp only [V.dot_def, V.sub_x, V.sub_y, V.sub_z]; grind

theorem dot_add (n x y : V K) : V.dot n (V.add x y) = V.dot n x + V.dot n y := by
  simp only [V.dot_def, V.add_x, V.add_y, V.add_z]; grind

theorem dot_smul (n x : V K) (s : K) : V.dot n (V.smul s x) = s * V.dot n x := by
  simp only [V.dot_def, V.smul_x, V.smul_y, V.smul_z]; grind

theorem d2_comm (x y : V K) : V.d2 x y = V.d2 y x := by simp only [V.d2_def]; grind

theorem d2_self (x : V K) : V.d2 x x = 0 := by simp only [V.d2_def]; grind

/-! ### The free centres -/

/-- A ball misses the material iff its centre is at least `r` from both planes. -/
theorem free_iff {r : K} (hr : 0 < r) (ctr : V K) : Free P.S r ctr ↔ r ≤ P.a₁ ctr ∧ r ≤ P.a₂ ctr := by
  constructor
  · intro h
    -- one plane at a time: a centre nearer than r to plane i has a point of {aᵢ ≤ 0} in its ball
    have one : ∀ (n : V K) (d : K), V.dot n n = 1 → (∀ y, V.dot n y - d ≤ 0 → r * r ≤ V.d2 y ctr) →
        r ≤ V.dot n ctr - d := by
      intro n d hn hy
      rcases le_or_lt' r (V.dot n ctr - d) with h1 | h1
      · exact h1
      · exfalso
        rcases le_or_lt' (V.dot n ctr - d) (-r) with h2 | h2
        · have := hy ctr (by grind); rw [d2_self] at this
          have := mul_pos' hr hr; grind
        · -- the point t back along n, t halfway between the centre's height and r
          let s := (V.dot n ctr - d + r) / 2
          let y := V.sub ctr (V.smul s n)
          have hy1 : V.dot n y - d = V.dot n ctr - d - s := by
            simp only [y, dot_sub, dot_smul, hn]; grind
          have hd : V.d2 y ctr = s * s := by
            simp only [y, V.d2_def, V.sub_x, V.sub_y, V.sub_z, V.smul_x, V.smul_y, V.smul_z]
            simp only [V.dot_def] at hn
            grind
          have := hy y (by grind)
          have := sq_lt_of_abs_lt (a := s) (r := r) (by grind) (by grind)
          grind
    exact ⟨one P.n₁ P.d₁ P.unit₁ fun y hy => h y (Or.inl hy),
      one P.n₂ P.d₂ P.unit₂ fun y hy => h y (Or.inr hy)⟩
  · rintro ⟨h1, h2⟩ y hy
    have one : ∀ (n : V K) (d : K), V.dot n n = 1 → r ≤ V.dot n ctr - d → V.dot n y - d ≤ 0 →
        r * r ≤ V.d2 y ctr := by
      intro n d hn hc hy
      have cs := cs_unit hn (V.sub ctr y)
      have e : V.dot n (V.sub ctr y) = (V.dot n ctr - d) - (V.dot n y - d) := by
        rw [dot_sub]; grind
      have sq := mul_self_le_mul_self' (le_of_lt'' hr) (show r ≤ V.dot n (V.sub ctr y) by grind)
      have : V.dot (V.sub ctr y) (V.sub ctr y) = V.d2 y ctr := by
        simp only [V.d2_def, V.dot_def, V.sub_x, V.sub_y, V.sub_z]; grind
      grind
    rcases hy with hy | hy
    · exact one P.n₁ P.d₁ P.unit₁ h1 hy
    · exact one P.n₂ P.d₂ P.unit₂ h2 hy
where le_of_lt'' {a b : K} (h : a < b) : a ≤ b := by grind

/-! ### The closing in closed form -/

/-- **The closing of a concave dihedral edge.** -/
theorem closing_iff {r : K} (hr : 0 < r) (x : V K) :
    closing P.S r x ↔
      P.S x ∨ (P.Cone r x ∧ r * r * (1 - P.c * P.c) ≤ P.q (r - P.a₁ x) (r - P.a₂ x)) := by
  have pos := P.one_sub_pos
  have cb := abs_lt_one_of_sq (show P.c * P.c < 1 from P.skew)
  constructor
  · intro hx
    rcases Classical.em (P.S x) with hS | hS
    · exact Or.inl hS
    · right
      have hx₁ : 0 < P.a₁ x := by simp only [S] at hS; grind
      have hx₂ : 0 < P.a₂ x := by simp only [S] at hS; grind
      -- otherwise a free centre within r of x
      refine Classical.byContradiction fun hn => ?_
      have found : ∃ ctr, Free P.S r ctr ∧ V.d2 x ctr < r * r := by
        let b₁ := r - P.a₁ x
        let b₂ := r - P.a₂ x
        have eb₁ : b₁ = r - P.a₁ x := rfl
        have eb₂ : b₂ = r - P.a₂ x := rfl
        -- the centre x + α n₁ + β n₂, its two heights and its distance
        have move : ∀ α β : K,
            P.a₁ (V.add x (V.add (V.smul α P.n₁) (V.smul β P.n₂))) = P.a₁ x + α + P.c * β ∧
            P.a₂ (V.add x (V.add (V.smul α P.n₁) (V.smul β P.n₂))) = P.a₂ x + P.c * α + β ∧
            V.d2 x (V.add x (V.add (V.smul α P.n₁) (V.smul β P.n₂)))
              = α * α + 2 * P.c * α * β + β * β := by
          intro α β
          have u₁ := P.unit₁; have u₂ := P.unit₂
          simp only [V.dot_def] at u₁ u₂
          simp only [a₁, a₂, c, V.dot_def, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y,
            V.smul_z, V.d2_def]
          refine ⟨by grind, by grind, by grind⟩
        have rr := mul_pos' hr hr
        rcases Classical.em (b₁ ≤ 0 ∧ b₂ ≤ 0) with hb | hb
        · refine ⟨x, (P.free_iff hr x).mpr ⟨by grind, by grind⟩, ?_⟩
          rw [d2_self]; exact rr
        rcases Classical.em (P.Cone r x) with hc | hc
        · -- the nearest point of the crease line
          have hq : P.q b₁ b₂ < r * r * (1 - P.c * P.c) := by
            rcases le_or_lt' (r * r * (1 - P.c * P.c)) (P.q b₁ b₂) with h | h
            · exact absurd ⟨hc, h⟩ hn
            · exact h
          let D := 1 - P.c * P.c
          let α := (b₁ - P.c * b₂) / D
          let β := (b₂ - P.c * b₁) / D
          obtain ⟨m₁, m₂, md⟩ := move α β
          have eα : α = (b₁ - P.c * b₂) / D := rfl
          have eβ : β = (b₂ - P.c * b₁) / D := rfl
          have eD : D = 1 - P.c * P.c := rfl
          have hD : D ≠ 0 := by grind
          have e₁ : α + P.c * β = b₁ := by rw [eα, eβ]; grind
          have e₂ : P.c * α + β = b₂ := by rw [eα, eβ]; grind
          have ed : (α * α + 2 * P.c * α * β + β * β) * D = P.q b₁ b₂ := by
            rw [eα, eβ, eD]; simp only [q]; grind
          refine ⟨V.add x (V.add (V.smul (α) P.n₁) (V.smul (β) P.n₂)), (P.free_iff hr _).mpr ⟨by grind, by grind⟩, ?_⟩
          rw [md]
          refine Classical.byContradiction fun h => ?_
          have := mul_le_mul_of_nonneg_left' (a := r * r) (b := α * α + 2 * P.c * α * β + β * β)
            (c := D) (by grind) (by grind)
          grind
        · simp only [Cone] at hc
          rcases Classical.em (b₂ ≤ P.c * b₁) with h2 | h2
          · rcases le_or_lt' b₁ (-r) with h1 | h1
            · -- far over plane 1: the foot on plane 2
              have b₂pos : 0 < b₂ := by grind
              obtain ⟨m₁, m₂, md⟩ := move 0 b₂
              have : -b₂ < P.c * b₂ := by
                have := mul_pos' (show 0 < P.c + 1 by grind) b₂pos; grind
              refine ⟨V.add x (V.add (V.smul (0) P.n₁) (V.smul (b₂) P.n₂)), (P.free_iff hr _).mpr ⟨by grind, by grind⟩, ?_⟩
              rw [md]; have := sq_lt_of_abs_lt (a := b₂) (r := r) (by grind) (by grind); grind
            · obtain ⟨m₁, m₂, md⟩ := move b₁ 0
              refine ⟨V.add x (V.add (V.smul (b₁) P.n₁) (V.smul (0) P.n₂)), (P.free_iff hr _).mpr ⟨by grind, by grind⟩, ?_⟩
              rw [md]; have := sq_lt_of_abs_lt (a := b₁) (r := r) (by grind) (by grind); grind
          · have h1 : b₁ ≤ P.c * b₂ := by grind
            rcases le_or_lt' b₂ (-r) with h3 | h3
            · have b₁pos : 0 < b₁ := by grind
              obtain ⟨m₁, m₂, md⟩ := move b₁ 0
              have : -b₁ < P.c * b₁ := by
                have := mul_pos' (show 0 < P.c + 1 by grind) b₁pos; grind
              refine ⟨V.add x (V.add (V.smul (b₁) P.n₁) (V.smul (0) P.n₂)), (P.free_iff hr _).mpr ⟨by grind, by grind⟩, ?_⟩
              rw [md]; have := sq_lt_of_abs_lt (a := b₁) (r := r) (by grind) (by grind); grind
            · obtain ⟨m₁, m₂, md⟩ := move 0 b₂
              refine ⟨V.add x (V.add (V.smul (0) P.n₁) (V.smul (b₂) P.n₂)), (P.free_iff hr _).mpr ⟨by grind, by grind⟩, ?_⟩
              rw [md]; have := sq_lt_of_abs_lt (a := b₂) (r := r) (by grind) (by grind); grind
      obtain ⟨ctr, hf, hd⟩ := found
      have := hx ctr hf
      grind
  · rintro (hS | ⟨⟨hc₁, hc₂⟩, hq⟩)
    · exact closing_extensive _ r x hS
    · intro ctr hf
      obtain ⟨f₁, f₂⟩ := (P.free_iff hr ctr).mp hf
      let w := V.sub ctr x
      have s₁ : V.dot P.n₁ w = P.a₁ ctr - P.a₁ x := by simp only [w, dot_sub, a₁]; grind
      have s₂ : V.dot P.n₂ w = P.a₂ ctr - P.a₂ x := by simp only [w, dot_sub, a₂]; grind
      have g := P.gram w
      have gn := sq_nonneg' (V.dot w P.t)
      -- q at the centre's heights is at least q at the crease's (q convex, its slope ≥ 0 there)
      let b₁ := r - P.a₁ x
      let b₂ := r - P.a₂ x
      let u₁ := V.dot P.n₁ w - b₁
      let u₂ := V.dot P.n₂ w - b₂
      have hu₁ : 0 ≤ u₁ := by simp only [u₁, b₁]; grind
      have hu₂ : 0 ≤ u₂ := by simp only [u₂, b₂]; grind
      have split : P.q (V.dot P.n₁ w) (V.dot P.n₂ w)
          = P.q b₁ b₂ + P.q u₁ u₂ + 2 * ((b₁ - P.c * b₂) * u₁ + (b₂ - P.c * b₁) * u₂) := by
        simp only [q, u₁, u₂]; grind
      have p₁ := mul_nonneg' (show 0 ≤ b₁ - P.c * b₂ by grind) hu₁
      have p₂ := mul_nonneg' (show 0 ≤ b₂ - P.c * b₁ by grind) hu₂
      have qn := P.q_nonneg u₁ u₂
      have ww : V.dot w w = V.d2 x ctr := by
        simp only [w, V.d2_def, V.dot_def, V.sub_x, V.sub_y, V.sub_z]; grind
      have : r * r * (1 - P.c * P.c) ≤ V.dot w w * (1 - P.c * P.c) := by grind
      have := le_of_mul_le_mul_left' (a := r * r) (b := V.dot w w) (c := 1 - P.c * P.c)
        (by grind) pos
      grind

/-! ### The blend is the cylinder about the crease; the springs; G1 -/

theorem t_sq : V.dot P.t P.t = 1 - P.c * P.c := by
  have := lagrange P.n₁ P.n₂; rw [P.unit₁, P.unit₂] at this; simp only [t, c]; grind

/-- For a point `c₀` of the crease line, `q b = |t|²|w|² − (w·t)²` with `w = x − c₀`: the fill's
boundary `q b = r²(1 − c²)` is the cylinder of radius `r` about the crease. -/
theorem q_eq_cyl {r : K} (c₀ x : V K) (h₁ : P.a₁ c₀ = r) (h₂ : P.a₂ c₀ = r) :
    P.q (r - P.a₁ x) (r - P.a₂ x)
      = V.dot P.t P.t * V.dot (V.sub x c₀) (V.sub x c₀)
        - V.dot (V.sub x c₀) P.t * V.dot (V.sub x c₀) P.t := by
  have g := P.gram (V.sub x c₀)
  have e₁ : V.dot P.n₁ (V.sub x c₀) = -(r - P.a₁ x) := by simp only [dot_sub, a₁] at *; grind
  have e₂ : V.dot P.n₂ (V.sub x c₀) = -(r - P.a₂ x) := by simp only [dot_sub, a₂] at *; grind
  rw [e₁, e₂] at g
  rw [P.t_sq]
  simp only [q] at *
  grind

/-- **The springs.** The line `c₀ − r n₁ + λ t` lies on face 1, on the cylinder and on the edge of
the crease's cone; so does `c₀ − r n₂ + λ t` on face 2 (by symmetry). -/
theorem spring₁ {r : K} (c₀ : V K) (h₁ : P.a₁ c₀ = r) (h₂ : P.a₂ c₀ = r) (l : K) :
    let p := V.add (V.sub c₀ (V.smul r P.n₁)) (V.smul l P.t)
    P.a₁ p = 0 ∧ P.q (r - P.a₁ p) (r - P.a₂ p) = r * r * (1 - P.c * P.c) ∧
      r - P.a₂ p = P.c * (r - P.a₁ p) := by
  intro p
  have n₁t : V.dot P.n₁ P.t = 0 := by simp only [t, V.dot_def, V.cross_x, V.cross_y, V.cross_z]; grind
  have n₂t : V.dot P.n₂ P.t = 0 := by simp only [t, V.dot_def, V.cross_x, V.cross_y, V.cross_z]; grind
  have u₁ := P.unit₁
  have pa₁ : P.a₁ p = 0 := by
    simp only [p, a₁, dot_add, dot_sub, dot_smul, n₁t, u₁] at *; grind
  have pa₂ : P.a₂ p = r - r * P.c := by
    simp only [p, a₂, a₁, dot_add, dot_sub, dot_smul, n₂t, c] at *
    have : V.dot P.n₂ P.n₁ = V.dot P.n₁ P.n₂ := by simp only [V.dot_def]; grind
    grind
  refine ⟨pa₁, ?_, ?_⟩
  · rw [pa₁, pa₂]; simp only [q]; grind
  · rw [pa₁, pa₂]; grind

/-- **G1 at the spring.** The cylinder's normal at `p = c₀ − r n₁ + λ t` (the part of `p − c₀`
square to the axis) is `−r n₁`: the blend is tangent to face 1 along its spring. -/
theorem g1_spring₁ {r : K} (c₀ : V K) (l : K) :
    let w := V.sub (V.add (V.sub c₀ (V.smul r P.n₁)) (V.smul l P.t)) c₀
    V.sub (V.smul (V.dot P.t P.t) w) (V.smul (V.dot w P.t) P.t)
      = V.smul (V.dot P.t P.t) (V.smul (-r) P.n₁) := by
  intro w
  have n₁t : V.dot P.n₁ P.t = 0 := by simp only [t, V.dot_def, V.cross_x, V.cross_y, V.cross_z]; grind
  have wt : V.dot w P.t = l * V.dot P.t P.t := by
    simp only [w, V.dot_def, V.sub_x, V.sub_y, V.sub_z, V.add_x, V.add_y, V.add_z, V.smul_x,
      V.smul_y, V.smul_z] at *
    grind
  rw [wt]
  apply V.ext' <;>
    simp only [w, V.sub_x, V.sub_y, V.sub_z, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y,
      V.smul_z] <;> grind

end PlanePair

end ForgeExact.Fillet

namespace ForgeExact.Fillet.PlanePair

open Lean.Grind

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

variable (P : PlanePair K)

/-- **The differential test's bound.** A centre within `δ` of both offset planes (what
`fillet-check` decides exactly) is within `2δ / sin θ` of the spec's crease line, `θ` the angle
between the normals (`sin² θ = 1 − c²`): there is a point `c⋆` of the crease with
`|c − c⋆|² (1 − c²) ≤ 4 δ²`. -/
theorem near_crease {r δ : K} (x : V K) (h₁ : -δ ≤ P.a₁ x - r ∧ P.a₁ x - r ≤ δ)
    (h₂ : -δ ≤ P.a₂ x - r ∧ P.a₂ x - r ≤ δ) :
    ∃ cs : V K, P.a₁ cs = r ∧ P.a₂ cs = r ∧ V.d2 x cs * (1 - P.c * P.c) ≤ 4 * (δ * δ) := by
  have pos := P.one_sub_pos
  have cb := abs_lt_one_of_sq (show P.c * P.c < 1 from P.skew)
  let D := 1 - P.c * P.c
  let b₁ := r - P.a₁ x
  let b₂ := r - P.a₂ x
  let α := (b₁ - P.c * b₂) / D
  let β := (b₂ - P.c * b₁) / D
  have eα : α = (b₁ - P.c * b₂) / D := rfl
  have eβ : β = (b₂ - P.c * b₁) / D := rfl
  have eD : D = 1 - P.c * P.c := rfl
  have hD : D ≠ 0 := by grind
  let cs := V.add x (V.add (V.smul α P.n₁) (V.smul β P.n₂))
  have u₁ := P.unit₁; have u₂ := P.unit₂
  simp only [V.dot_def] at u₁ u₂
  have m₁ : P.a₁ cs = P.a₁ x + α + P.c * β := by
    simp only [cs, a₁, c, V.dot_def, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y, V.smul_z]
    grind
  have m₂ : P.a₂ cs = P.a₂ x + P.c * α + β := by
    simp only [cs, a₂, c, V.dot_def, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y, V.smul_z]
    grind
  have md : V.d2 x cs = α * α + 2 * P.c * α * β + β * β := by
    simp only [cs, c, V.d2_def, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y, V.smul_z,
      V.dot_def]
    grind
  have e₁ : α + P.c * β = b₁ := by rw [eα, eβ]; grind
  have e₂ : P.c * α + β = b₂ := by rw [eα, eβ]; grind
  have ed : (α * α + 2 * P.c * α * β + β * β) * D = P.q b₁ b₂ := by
    rw [eα, eβ, eD]; simp only [q]; grind
  refine ⟨cs, by grind, by grind, ?_⟩
  rw [md]
  -- q b ≤ b₁² + 2|c||b₁b₂| + b₂² ≤ 4δ²
  have s₁ := mul_self_le_mul_self_abs (a := b₁) (δ := δ) (by grind) (by grind)
  have s₂ := mul_self_le_mul_self_abs (a := b₂) (δ := δ) (by grind) (by grind)
  have cross : -(2 * P.c * b₁ * b₂) ≤ 2 * (δ * δ) := by
    have h3 := sq_nonneg' (b₁ + P.c * b₂)
    have h4 := sq_nonneg' (b₁ - P.c * b₂)
    have h5 := mul_le_mul_of_nonneg_left' (a := P.c * P.c) (b := 1) (c := b₂ * b₂) (by grind)
      (sq_nonneg' b₂)
    grind
  simp only [q] at ed
  grind
where
  mul_self_le_mul_self_abs {a δ : K} (h1 : -δ ≤ a) (h2 : a ≤ δ) : a * a ≤ δ * δ := by
    rcases le_or_lt' 0 a with h | h
    · exact mul_self_le_mul_self' h h2
    · have := mul_self_le_mul_self' (a := -a) (b := δ) (by grind) (by grind); grind

end ForgeExact.Fillet.PlanePair
