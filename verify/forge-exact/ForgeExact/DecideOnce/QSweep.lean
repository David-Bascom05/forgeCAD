import ForgeExact.DecideOnce.QVert

/-!
# Q1 L2: the sweep's events are vertices and line samples

The sweep direction is `x` (after the global rotation and shear, curved-l2-plan.md §2). Its
events are where a slab's delineability can fail (LemmaD's `NoCrit`):
* **a curve's turning points**: points of `q_j = q_k = 0` where `∇q_j`, `∇q_k` and `eₓ` are
  dependent. That determinant is the `x`-component of `∇q_j × ∇q_k`, a product of linear forms:
  a **quadric** (`critQ`, `critQ_ev`). So a turning point is a vertex of the three quadrics
  `q_j, q_k, critQ q_j q_k`, found and certified by QVert (`onVert_spec`, `bezout3_spec`), and
  their completeness is the vertex proposer's;
* **a surface's critical points**: points of `q = 0` where `∇q ∥ eₓ`, i.e. `∂_y q = ∂_z q = 0`:
  two planes (`gradPlaneY`, `gradPlaneZ`). With `q` they are again a vertex of three quadrics;
* the **vertices** of three carriers themselves.

So every event is a vertex sample, and the event values are their `x`-roots: `Num`s, ordered by
`Num.cmp`, with rational walls or sample planes between them by `QRef.between`.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef Lean.Grind

/-- The gradient's `y`- and `z`-components, as affine forms `(a, b, c, d)` = `a x + b y + c z + d`. -/
def gradY (q : Quadric) : Rat × Rat × Rat × Rat := (q.xy, 2 * q.yy, q.yz, q.ly)
def gradZ (q : Quadric) : Rat × Rat × Rat × Rat := (q.xz, q.yz, 2 * q.zz, q.lz)

/-- The product of two affine forms, as a quadric. -/
def affMul (f g : Rat × Rat × Rat × Rat) : Quadric :=
  let (a, b, c, d) := f
  let (a', b', c', d') := g
  ⟨a * a', b * b', c * c', a * b' + b * a', a * c' + c * a', b * c' + c * b',
   a * d' + d * a', b * d' + d * b', c * d' + d * c', d * d'⟩

/-- The difference of two quadrics. -/
def qsub (p q : Quadric) : Quadric :=
  ⟨p.xx - q.xx, p.yy - q.yy, p.zz - q.zz, p.xy - q.xy, p.xz - q.xz, p.yz - q.yz,
   p.lx - q.lx, p.ly - q.ly, p.lz - q.lz, p.c - q.c⟩

/-- `(∇q_j × ∇q_k)ₓ = ∂_y q_j ∂_z q_k − ∂_z q_j ∂_y q_k`: zero exactly at the turning points of
`q_j ∩ q_k` in the `x`-direction. -/
def critQ (qj qk : Quadric) : Quadric := qsub (affMul (gradY qj) (gradZ qk)) (affMul (gradZ qj) (gradY qk))

/-- `∂_y q = 0` and `∂_z q = 0` as planes (quadrics of degree 1). -/
def gradPlaneY (q : Quadric) : Quadric :=
  let (a, b, c, d) := gradY q
  ⟨0, 0, 0, 0, 0, 0, a, b, c, d⟩
def gradPlaneZ (q : Quadric) : Quadric :=
  let (a, b, c, d) := gradZ q
  ⟨0, 0, 0, 0, 0, 0, a, b, c, d⟩

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- An affine form's value. -/
def affEv (f : Rat × Rat × Rat × Rat) (p : PK K) : K :=
  M.q f.1 * p.1 + M.q f.2.1 * p.2.1 + M.q f.2.2.1 * p.2.2 + M.q f.2.2.2

/-- The gradient's components in the model. -/
def dyK (q : Quadric) (p : PK K) : K :=
  M.q q.xy * p.1 + (1 + 1) * M.q q.yy * p.2.1 + M.q q.yz * p.2.2 + M.q q.ly
def dzK (q : Quadric) (p : PK K) : K :=
  M.q q.xz * p.1 + M.q q.yz * p.2.1 + (1 + 1) * M.q q.zz * p.2.2 + M.q q.lz

theorem q_two : M.q 2 = 1 + 1 := by
  rw [show (2 : Rat) = 1 + 1 by grind, M.q_add, M.q_one]

theorem affEv_gradY (q : Quadric) (p : PK K) : affEv M (gradY q) p = dyK M q p := by
  simp only [affEv, gradY, dyK, M.q_mul, q_two]

theorem affEv_gradZ (q : Quadric) (p : PK K) : affEv M (gradZ q) p = dzK M q p := by
  simp only [affEv, gradZ, dzK, M.q_mul, q_two]

theorem affMul_ev (f g : Rat × Rat × Rat × Rat) (p : PK K) :
    Quadric.evK M (affMul f g) p = affEv M f p * affEv M g p := by
  obtain ⟨a, b, c, d⟩ := f
  obtain ⟨a', b', c', d'⟩ := g
  simp only [affMul, Quadric.evK, affEv, M.q_add, M.q_mul]
  grind

theorem qsub_ev (p q : Quadric) (x : PK K) :
    Quadric.evK M (qsub p q) x = Quadric.evK M p x - Quadric.evK M q x := by
  simp only [qsub, Quadric.evK, M.q_sub]
  grind

/-- **The turning-point quadric is the cross product's `x`-component.** -/
theorem critQ_ev (qj qk : Quadric) (p : PK K) :
    Quadric.evK M (critQ qj qk) p = dyK M qj p * dzK M qk p - dzK M qj p * dyK M qk p := by
  rw [critQ, qsub_ev, affMul_ev, affMul_ev, affEv_gradY, affEv_gradZ, affEv_gradY, affEv_gradZ]

theorem gradPlaneY_ev (q : Quadric) (p : PK K) : Quadric.evK M (gradPlaneY q) p = dyK M q p := by
  simp only [gradPlaneY, gradY, Quadric.evK, dyK, M.q_zero, M.q_mul, q_two]
  grind

theorem gradPlaneZ_ev (q : Quadric) (p : PK K) : Quadric.evK M (gradPlaneZ q) p = dzK M q p := by
  simp only [gradPlaneZ, gradZ, Quadric.evK, dzK, M.q_zero, M.q_mul, q_two]
  grind

/-- At a rational point, `dyK`, `dzK` are exact-curved's gradient components (`Quadric.grad`). -/
theorem dyK_grad (q : Quadric) (p : V3) :
    dyK M q (M.q p.x, M.q p.y, M.q p.z) = M.q (q.grad p).y := by
  simp only [dyK, Quadric.grad, M.q_add, M.q_mul, q_two]
  grind

theorem dzK_grad (q : Quadric) (p : V3) :
    dzK M q (M.q p.x, M.q p.y, M.q p.z) = M.q (q.grad p).z := by
  simp only [dzK, Quadric.grad, M.q_add, M.q_mul, q_two]
  grind

end Model

-- the unit circle (sphere ∩ z = 0) turns in x at (±1, 0, 0): critQ there is 2y, and the vertex
-- (1, 0, 0) certifies on sphere, plane and critQ
#guard critQ sphere1 planeZ == ⟨0, 0, 0, 0, 0, 0, 0, 2, 0, 0⟩
#guard vAt100.onVert sphere1 planeZ (critQ sphere1 planeZ)

end ForgeExact.DecideOnce.Curved
