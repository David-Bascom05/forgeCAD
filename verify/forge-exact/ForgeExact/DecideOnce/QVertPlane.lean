import ForgeExact.DecideOnce.QPropose
import ForgeExact.DecideOnce.QVert

/-!
# Q1 L2: the vertex proposer for a plane and two quadrics, complete

Mirrors the curved Rust session's Milestone 1d (curved-l2-plan.md §4). For a plane
`P = lx x + ly y + lz z + c` with `lz ≠ 0`, the plane's `z` is `Z(x, y) = −(lx x + ly y + c)/lz`.
* `onPlane q P`: `q(x, y, Z(x, y))`, a quadratic in `y` over `ℚ[x]` (a coefficient triple), with
  `onPlane_ev`;
* `elimT` (QPropose.lean) on the two conics: `R(x)`, `N`, `D` with `y = −N/D`;
* the representation `y = −N/D`, `z = −N_z/D_z` with `N_z = lx·X·D − ly·N + c·D` and `D_z = lz·D`;
* the Bezout cofactors for `bezout3`: `A = U`, `B = V`, `C = −(U K_a + V K_b)`, where
  `q = K·P + f` with `K = k₀ + k₁ z` (untrusted; `bezout3` checks the identity);
* the quotients `Q` by `Poly.pdiv` (untrusted; `onVert` checks them).

`proposeVertPlane_complete`: every common point of the plane and the two quadrics is a proposed
sample's point. With `onVert_spec`, the samples are exactly the vertices.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef Lean.Grind

/-- `q(x, y, α₀ + α₁x + α₂y)` as coefficients in `y` over `ℚ[x]`. -/
def substZ (q : Quadric) (α0 α1 α2 : Rat) : Poly × Poly × Poly :=
  ( [q.zz * α0 * α0 + q.lz * α0 + q.c, 2 * q.zz * α0 * α1 + q.xz * α0 + q.lx + q.lz * α1,
      q.xx + q.zz * α1 * α1 + q.xz * α1],
    [2 * q.zz * α2 * α0 + q.yz * α0 + q.ly + q.lz * α2, q.xy + 2 * q.zz * α2 * α1 + q.xz * α2 + q.yz * α1],
    [q.yy + q.zz * α2 * α2 + q.yz * α2] )

/-- The plane's `z = α₀ + α₁x + α₂y`. -/
def planeZOf (P : Quadric) : Rat × Rat × Rat := (-P.c / P.lz, -P.lx / P.lz, -P.ly / P.lz)

def onPlane (q P : Quadric) : Poly × Poly × Poly :=
  let (α0, α1, α2) := planeZOf P
  substZ q α0 α1 α2

/-- The vertex sample for the root `n`. -/
def mkVertP (qa qb P : Quadric) (E : Elim) (n : Num) : VSample :=
  let X : Poly := [0, 1]
  let Nz := padd (psub (pscale P.lx (pmul X E.D)) (pscale P.ly E.N)) (pscale P.c E.D)
  let Dz := pscale P.lz E.D
  let Qof := fun (q : Quadric) => Poly.pdiv (homog3 q E.N E.D Nz Dz) E.R
  ⟨E.R, E.N, E.D, Nz, Dz, n, Qof qa, Qof qb, []⟩

/-- `q = K·P + f`: `K = k₀ + k₁ z` over `ℚ[x][y]` (untrusted; checked through `bezout3`). -/
def quotP (q P : Quadric) : TPoly :=
  let t := restrT q
  let t1 := (t.getD 1 default).b
  let t2 := (t.getD 2 default).b
  let inv : YPoly := ⟨[1 / P.lz]⟩
  let p0 : BPoly := [⟨[P.c, P.lx]⟩, ⟨[P.ly]⟩]
  let k1 := pscale inv t2
  let k0 := pscale inv (psub t1 (pmul k1 p0))
  [⟨k0⟩, ⟨k1⟩]

/-- Every vertex of the plane `P` and the quadrics `qa`, `qb`, each certified, or `none`. -/
def proposeVertPlane (qa qb P : Quadric) (fuel : Nat) : Option (List VSample) := do
  if P.lz = 0 then none
  let E := elimT (onPlane qa P) (onPlane qb P)
  let A : TPoly := [⟨E.U⟩]
  let B : TPoly := [⟨E.V⟩]
  let C : TPoly := pneg (padd (pmul A (quotP qa P)) (pmul B (quotP qb P)))
  if !bezout3 qa qb P A B C E.R then none
  let ns ← rootsOf E.R fuel
  let vs := ns.map (mkVertP qa qb P E)
  if vs.all (fun v => v.onVert qa qb P && v.dOk signFuel) then some vs else none

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem substZ_ev (q : Quadric) (α0 α1 α2 : Rat) (x y : K) :
    let t := substZ q α0 α1 α2
    M.ev t.1 x + y * (M.ev t.2.1 x + y * M.ev t.2.2 x) =
      Quadric.evK M q (x, y, M.q α0 + M.q α1 * x + M.q α2 * y) := by
  simp only [substZ, Model.ev, evalWith_cons, evalWith_nil, Quadric.evK]
  simp only [Model.q, M.ι.map_add, M.ι.map_mul]
  have h2 : M.ι.f 2 = 1 + 1 := by
    rw [show (2 : Rat) = 1 + 1 by grind, M.ι.map_add, M.ι.map_one]
  rw [h2]
  grind

/-- On the plane, `z` is the plane's `z`. -/
theorem planeZOf_eq {P : Quadric} (hP : P.xx = 0 ∧ P.yy = 0 ∧ P.zz = 0 ∧ P.xy = 0 ∧ P.xz = 0 ∧ P.yz = 0)
    (hz : P.lz ≠ 0) {x y z : K} (h : Quadric.evK M P (x, y, z) = 0) :
    z = M.q (planeZOf P).1 + M.q (planeZOf P).2.1 * x + M.q (planeZOf P).2.2 * y := by
  obtain ⟨h1, h2, h3, h4, h5, h6⟩ := hP
  simp only [Quadric.evK, h1, h2, h3, h4, h5, h6, M.q_zero] at h
  simp only [planeZOf]
  have hl : M.q P.lz ≠ 0 := fun h0 => hz ((M.q_eq (a := P.lz) (b := 0)).mp (by rw [h0, M.q_zero]))
  have hinv : M.q P.lz * (M.q P.lz)⁻¹ = 1 := Field.mul_inv_cancel hl
  have e : M.q (-P.c) = -M.q P.c := M.q_neg _
  have e1 : M.q (-P.lx) = -M.q P.lx := M.q_neg _
  have e2 : M.q (-P.ly) = -M.q P.ly := M.q_neg _
  rw [M.q_div _ _ hz, M.q_div _ _ hz, M.q_div _ _ hz, e, e1, e2]
  grind

/-- **Every vertex of the plane and the two quadrics is a proposed sample's point.** -/
theorem proposeVertPlane_complete {qa qb P : Quadric}
    (hP : P.xx = 0 ∧ P.yy = 0 ∧ P.zz = 0 ∧ P.xy = 0 ∧ P.xz = 0 ∧ P.yz = 0) {fuel : Nat}
    {vs : List VSample} (h : proposeVertPlane qa qb P fuel = some vs) {x y z : K}
    (ha : Quadric.evK M qa (x, y, z) = 0) (hb : Quadric.evK M qb (x, y, z) = 0)
    (hp : Quadric.evK M P (x, y, z) = 0) :
    ∃ v ∈ vs, Denotes M v.x x ∧ v.dOk signFuel = true ∧ v.onVert qa qb P = true ∧
      M.ev v.Dy x ≠ 0 ∧ y = -(M.ev v.Ny x) / M.ev v.Dy x := by
  unfold proposeVertPlane at h
  simp only [Option.bind_eq_bind] at h
  by_cases hz : P.lz = 0
  · simp [hz] at h
  · simp only [hz, ite_false] at h
    generalize hE : elimT (onPlane qa P) (onPlane qb P) = E at h
    split at h
    · cases h
    · rename_i hb3
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hb3
      cases hr : rootsOf E.R fuel with
      | none => simp [hr] at h
      | some ns =>
        simp only [hr, Option.bind_some] at h
        split at h
        · rename_i hall
          simp only [Option.some.injEq] at h
          subst h
          have hR := bezout3_spec M (by simpa using hb3) ha hb hp
          obtain ⟨n, hn, hd⟩ := rootsOf_complete M hr hR
          refine ⟨mkVertP qa qb P E n, List.mem_map_of_mem hn, hd, ?_⟩
          have hok := List.all_eq_true.mp hall _ (List.mem_map_of_mem hn)
          simp only [Bool.and_eq_true] at hok
          obtain ⟨hy, _⟩ := vdOk_ne M hd hok.2
          refine ⟨hok.2, hok.1, hy, ?_⟩
          -- y from the linear combination of the two conics
          have hzp := planeZOf_eq M hP hz hp
          have ea := substZ_ev M qa (planeZOf P).1 (planeZOf P).2.1 (planeZOf P).2.2 x y
          have eb := substZ_ev M qb (planeZOf P).1 (planeZOf P).2.1 (planeZOf P).2.2 x y
          rw [← hzp, ha] at ea
          rw [← hzp, hb] at eb
          have hl := linT M (onPlane qa P) (onPlane qb P) x y
          simp only [onPlane] at hl
          simp only [onPlane] at ea eb hE
          rw [hE] at hl
          rw [ea, eb] at hl
          simp only [mkVertP] at hy ⊢
          have hinv : M.ev E.D x * (M.ev E.D x)⁻¹ = 1 := Field.mul_inv_cancel hy
          grind
        · cases h

end Model

-- the plane z = 0, the unit sphere and the plane y = 0: the two vertices (±1, 0, 0)
#guard (proposeVertPlane sphere1 planeY0 planeZ 30).map List.length = some 2
-- a tilted plane x + y + 2z = 1/2, the sphere and the z-cylinder x² + y² = 1/4 (the Rust's test)
def planeTilt : Quadric := ⟨0, 0, 0, 0, 0, 0, 1, 1, 2, -1/2⟩
def cylZ : Quadric := ⟨1, 1, 0, 0, 0, 0, 0, 0, 0, -1/4⟩
#guard (proposeVertPlane sphere1 cylZ planeTilt 30).isSome
-- a plane with no z-term is refused (the exchange of y and z is the next step)
#guard (proposeVertPlane sphere1 planeZ planeY0 30).isNone

end ForgeExact.DecideOnce.Curved
