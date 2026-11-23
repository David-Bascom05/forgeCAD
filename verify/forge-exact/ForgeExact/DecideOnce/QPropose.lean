import ForgeExact.DecideOnce.QEdge
import ForgeExact.DecideOnce.RootCover

/-!
# Q1 L2: the edge proposer, complete

On the plane `x = c`, write `q_j = a₂z² + a₁z + a₀` and `q_k = b₂z² + b₁z + b₀` (coefficients in
`ℚ[y]`, `restr`). The closed forms (checked by computer algebra, and anyway re-checked below):
* `R = (a₂b₀ − a₀b₂)² − (a₂b₁ − a₁b₂)(a₁b₀ − a₀b₁)`, the resultant in `z`;
* `N = b₂a₀ − a₂b₀`, `D = b₂a₁ − a₂b₁`: the linear combination `b₂ q_j − a₂ q_k = D z + N`;
* `homog q_j = a₂ R` and `homog q_k = b₂ R`, so the quotients are `a₂` and `b₂`;
* the Bezout cofactors `U = u₀ + u₁ z`, `V = v₀ + v₁ z` with `U q_j + V q_k = R`.

`proposeEdge qj qk c fuel` takes every root of `R` (`rootsOf`, complete), builds the sample of
each, and checks every certificate (`onCurve`, `dOk`, `bezout`); if any fails, `none`
(`Skip::Budget`).

**`proposeEdge_complete`**: every point `(c, y, z)` of the model on both quadrics is the point of
one of the proposed samples. With `onCurve_spec` (each sample is such a point) the samples are
exactly the crossings of the curve with the plane.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef Lean.Grind

/-- The elimination data for `q_j`, `q_k` on `x = c`: `R`, `N`, `D`, and the cofactors. -/
structure Elim where
  R : Poly
  N : Poly
  D : Poly
  U : BPoly
  V : BPoly

def elim (qj qk : Quadric) (c : Rat) : Elim :=
  let a := restr qj c
  let b := restr qk c
  let (a0, a1, a2) := a
  let (b0, b1, b2) := b
  let m := fun (p q : Poly) => pmul p q
  let X := psub (m a2 b0) (m a0 b2)
  let Y := psub (m a2 b1) (m a1 b2)
  let Z := psub (m a1 b0) (m a0 b1)
  let u0 := padd (psub (psub (m a0 (m b2 b2)) (m a1 (m b1 b2))) (m a2 (m b0 b2))) (m a2 (m b1 b1))
  let u1 := padd (pneg (m a1 (m b2 b2))) (m a2 (m b1 b2))
  let v0 := padd (psub (padd (pneg (m a0 (m a2 b2))) (m a1 (m a1 b2))) (m a1 (m a2 b1))) (m a2 (m a2 b0))
  let v1 := psub (m a1 (m a2 b2)) (m a2 (m a2 b1))
  { R := psub (m X X) (m Y Z)
    N := psub (m b2 a0) (m a2 b0)
    D := psub (m b2 a1) (m a2 b1)
    U := [⟨u0⟩, ⟨u1⟩]
    V := [⟨v0⟩, ⟨v1⟩] }

/-- The sample for the root `n`. -/
def mkEdge (qj qk : Quadric) (c : Rat) (E : Elim) (n : Num) : ESample :=
  ⟨c, E.R, E.N, E.D, n, (restr qj c).2.2, (restr qk c).2.2⟩

/-- Every crossing of `q_j = q_k = 0` with `x = c`, each certified, or `none`. -/
def proposeEdge (qj qk : Quadric) (c : Rat) (fuel : Nat) : Option (List ESample) := do
  let E := elim qj qk c
  if !bezout qj qk c E.U E.V E.R then none
  let ns ← rootsOf E.R fuel
  let es := ns.map (mkEdge qj qk c E)
  if es.all (fun e => e.onCurve qj qk && e.dOk signFuel) then some es else none

section Model

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- `b₂ q_j − a₂ q_k = D z + N` on the plane. -/
theorem lin_ev (qj qk : Quadric) (c : Rat) (y z : K) :
    M.ev (restr qk c).2.2 y * Quadric.evK M qj (M.q c, y, z) -
      M.ev (restr qj c).2.2 y * Quadric.evK M qk (M.q c, y, z) =
    M.ev (elim qj qk c).D y * z + M.ev (elim qj qk c).N y := by
  rw [← restrB_ev, ← restrB_ev]
  simp only [elim, restrB, restr, evalWith_cons, evalWith_nil, yHom, Model.ev, eval_psub,
    eval_pmul]
  grind

/-- **Every crossing is a proposed sample's point.** -/
theorem proposeEdge_complete {qj qk : Quadric} {c : Rat} {fuel : Nat} {es : List ESample}
    (h : proposeEdge qj qk c fuel = some es) {y z : K}
    (hj : Quadric.evK M qj (M.q c, y, z) = 0) (hk : Quadric.evK M qk (M.q c, y, z) = 0) :
    ∃ e ∈ es, Denotes M e.y y ∧ e.dOk signFuel = true ∧ e.onCurve qj qk = true ∧
      planePt M e.c e.N e.D y = (M.q c, y, z) := by
  unfold proposeEdge at h
  simp only [Option.bind_eq_bind] at h
  by_cases hb : bezout qj qk c (elim qj qk c).U (elim qj qk c).V (elim qj qk c).R = true
  · simp only [hb, Bool.not_true, Bool.false_eq_true, ite_false] at h
    cases hr : rootsOf (elim qj qk c).R fuel with
    | none => simp [hr] at h
    | some ns =>
      simp only [hr, Option.bind_some] at h
      split at h
      · rename_i hall
        simp only [Option.some.injEq] at h
        subst h
        have hR := bezout_spec M hb hj hk
        obtain ⟨n, hn, hd⟩ := rootsOf_complete M hr hR
        refine ⟨mkEdge qj qk c (elim qj qk c) n, List.mem_map_of_mem hn, hd, ?_⟩
        have hok := List.all_eq_true.mp hall _ (List.mem_map_of_mem hn)
        simp only [Bool.and_eq_true] at hok
        refine ⟨hok.2, hok.1, ?_⟩
        have hD : M.ev (elim qj qk c).D y ≠ 0 := dOk_ne M (e := mkEdge qj qk c (elim qj qk c) n) hd hok.2
        have hl := lin_ev M qj qk c y z
        rw [hj, hk] at hl
        have hz : z = -(M.ev (elim qj qk c).N y) / M.ev (elim qj qk c).D y := by
          have hinv : M.ev (elim qj qk c).D y * (M.ev (elim qj qk c).D y)⁻¹ = 1 :=
            Field.mul_inv_cancel hD
          grind
        simp only [planePt, mkEdge]
        rw [hz]
      · cases h
  · simp [hb] at h

/-! ## The shear: a generic projection

Two crossings with one `y` (a configuration symmetric in `z`) make `D ≡ 0`, and the proposer
answers `none`. The cure is a shear of every carrier, `(x, y, z) ↦ (x, y + λz, z)`: covectors do
not change under a change of coordinates applied to all carriers alike (`shear_ev`), and for all
but finitely many `λ` the projection separates the crossings. **The decided rule**: `λ` is the
first of `shears` for which every proposal of the boolean succeeds; past the list, `Skip::Budget`. -/

/-- `q(x, y + λz, z)`. -/
def _root_.ForgeExact.Quad.Quadric.shear (l : Rat) (q : Quadric) : Quadric :=
  ⟨q.xx, q.yy, q.zz + l * l * q.yy + l * q.yz, q.xy, q.xz + l * q.xy, q.yz + 2 * l * q.yy,
   q.lx, q.ly, q.lz + l * q.ly, q.c⟩

/-- The shears tried, in order. -/
def shears : List Rat := [0, 1, 2, 3, -1, 1/2, 5, -3, 7, 1/3]

theorem shear_ev (l : Rat) (q : Quadric) (x y z : K) :
    Quadric.evK M (q.shear l) (x, y, z) = Quadric.evK M q (x, y + M.q l * z, z) := by
  simp only [Quadric.evK, Quadric.shear, Model.q, M.ι.map_add, M.ι.map_mul]
  have h2 : M.ι.f 2 = 1 + 1 := by
    rw [show (2 : Rat) = 1 + 1 by grind, M.ι.map_add, M.ι.map_one]
  rw [h2]
  grind

end Model

-- the unit sphere and the plane z = 0, on x = 0: the two crossings (0, ±1, 0)
#guard (proposeEdge sphere1 planeZ 0 30).map List.length = some 2
-- the unit sphere and the cylinder x² + z² = 1/4 on x = 0: four crossings (0, ±√3/2, ±1/2)
def cylY : Quadric := ⟨1, 0, 1, 0, 0, 0, 0, 0, 0, -1/4⟩
-- unsheared, two crossings share y = √3/2: refused, never wrong
#guard (proposeEdge sphere1 cylY 0 30).isNone
-- sheared by λ = 1 they separate: four crossings
#guard (proposeEdge (sphere1.shear 1) (cylY.shear 1) 0 30).map List.length = some 4

end ForgeExact.DecideOnce.Curved
