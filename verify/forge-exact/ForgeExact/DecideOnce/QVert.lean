import ForgeExact.DecideOnce.QEdge

/-!
# Q1 L2: vertex samples, by a checked univariate representation

A vertex is a point on three quadrics (design §2.4). It is represented over its `x`
coordinate (a projection taken generic by the proposer):
* `x = α`, a root of `R(x)` (an `Alg.Num`);
* `y = −N_y(α) / D_y(α)` and `z = −N_z(α) / D_z(α)`, with `D_y(α) D_z(α) ≠ 0`.

The data come from untrusted elimination and are checked as in QEdge.lean:
* `homog3 q = (D_y D_z)² · q(x, −N_y/D_y, −N_z/D_z)`, a polynomial in `x` (`homog3_ev`);
* **on the vertex**: `homog3 q = R · Q_q` for each of the three quadrics (`Poly.eqv`). Then all
  three vanish at the sample (`onVert_spec`);
* **the covector**: carrier `q`'s side at the sample is the sign of `homog3 q` at `α`
  (`vertCov_spec`).

Completeness (every common point of the three is a root of `R`) needs a Bezout identity in
`ℚ[x, y, z]`, the three-variable form of `bezout_spec`. It is the next step; the sweep's events
already find vertices as crossings of edge curves (QSweep), which is the other route.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef Lean.Grind

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- `(D_y D_z)² · q(x, −N_y/D_y, −N_z/D_z)`, a polynomial in `x`. -/
def homog3 (q : Quadric) (Ny Dy Nz Dz : Poly) : Poly :=
  let X : Poly := [0, 1]
  let Y2 := pmul Dy Dy
  let Z2 := pmul Dz Dz
  let YZ := pmul Dy Dz
  let terms : List Poly :=
    [ pscale q.xx (pmul (pmul X X) (pmul Y2 Z2)),
      pscale q.yy (pmul (pmul Ny Ny) Z2),
      pscale q.zz (pmul (pmul Nz Nz) Y2),
      pscale (-q.xy) (pmul (pmul X Ny) (pmul Dy Z2)),
      pscale (-q.xz) (pmul (pmul X Nz) (pmul Dz Y2)),
      pscale q.yz (pmul (pmul Ny Nz) YZ),
      pscale q.lx (pmul X (pmul Y2 Z2)),
      pscale (-q.ly) (pmul Ny (pmul Dy Z2)),
      pscale (-q.lz) (pmul Nz (pmul Dz Y2)),
      pscale q.c (pmul Y2 Z2) ]
  terms.foldr padd []

/-- The point `(x, −N_y(x)/D_y(x), −N_z(x)/D_z(x))`. -/
def vertPt (Ny Dy Nz Dz : Poly) (x : K) : PK K :=
  (x, -(M.ev Ny x) / M.ev Dy x, -(M.ev Nz x) / M.ev Dz x)

theorem homog3_ev (q : Quadric) (Ny Dy Nz Dz : Poly) (x : K) (hy : M.ev Dy x ≠ 0)
    (hz : M.ev Dz x ≠ 0) :
    M.ev (homog3 q Ny Dy Nz Dz) x =
      (M.ev Dy x * M.ev Dz x) * (M.ev Dy x * M.ev Dz x) * Quadric.evK M q (vertPt M Ny Dy Nz Dz x) := by
  simp only [homog3, List.foldr_cons, List.foldr_nil, Model.ev, eval_padd, eval_pmul, eval_pscale,
    evalWith_cons, evalWith_nil, Quadric.evK, vertPt]
  simp only [Model.ev] at hy hz
  generalize evalWith M.ι Dy x = dy at hy ⊢
  generalize evalWith M.ι Dz x = dz at hz ⊢
  generalize evalWith M.ι Ny x = ny
  generalize evalWith M.ι Nz x = nz
  simp only [Model.q, M.ι.map_neg, M.ι.map_zero, M.ι.map_one]
  have hiy : dy * dy⁻¹ = 1 := Field.mul_inv_cancel hy
  have hiz : dz * dz⁻¹ = 1 := Field.mul_inv_cancel hz
  grind

/-- A vertex sample: the representation, the root, and the three quadrics' quotients. -/
structure VSample where
  R : Poly
  Ny : Poly
  Dy : Poly
  Nz : Poly
  Dz : Poly
  /-- the root `α` of `R` -/
  x : Num
  Qa : Poly
  Qb : Poly
  Qc : Poly

/-- The on-vertex certificate for the quadrics `qa`, `qb`, `qc`. -/
def VSample.onVert (v : VSample) (qa qb qc : Quadric) : Bool :=
  Poly.eqv (homog3 qa v.Ny v.Dy v.Nz v.Dz) (pmul v.R v.Qa) &&
  Poly.eqv (homog3 qb v.Ny v.Dy v.Nz v.Dz) (pmul v.R v.Qb) &&
  Poly.eqv (homog3 qc v.Ny v.Dy v.Nz v.Dz) (pmul v.R v.Qc)

/-- `D_y(α) D_z(α) ≠ 0`, decided. -/
def VSample.dOk (v : VSample) (fuel : Nat) : Bool :=
  match v.x.sign (pmul v.Dy v.Dz) fuel with
  | some s => s != 0
  | none => false

/-- The sample's covector over the quadrics `qs`, decided exactly (`none`: fuel spent). -/
def vertCov (qs : List Quadric) (v : VSample) (fuel : Nat) : Option Cov :=
  qs.mapM fun q => (v.x.sign (homog3 q v.Ny v.Dy v.Nz v.Dz) fuel).map sgnOfInt

theorem vdOk_ne {v : VSample} {fuel : Nat} {α : K} (hα : Denotes M v.x α) (h : v.dOk fuel = true) :
    M.ev v.Dy α ≠ 0 ∧ M.ev v.Dz α ≠ 0 := by
  unfold VSample.dOk at h
  split at h
  · rename_i s hs
    have hsg := sign_spec M (pmul v.Dy v.Dz) fuel hα hs
    simp only [Model.ev, eval_pmul] at hsg
    simp only [Model.ev]
    have hne : evalWith M.ι v.Dy α * evalWith M.ι v.Dz α ≠ 0 := by
      intro h0
      rw [h0] at hsg
      rcases hsg with ⟨rfl, h'⟩ | ⟨rfl, _⟩ | ⟨rfl, h'⟩
      · exact Std.lt_irrefl h'
      · simp at h
      · exact Std.lt_irrefl h'
    exact ⟨fun h0 => hne (by rw [h0]; grind), fun h0 => hne (by rw [h0]; grind)⟩
  · cases h

theorem vanish_of_eqv {v : VSample} {q : Quadric} {Q : Poly} {α : K}
    (hc : Poly.eqv (homog3 q v.Ny v.Dy v.Nz v.Dz) (pmul v.R Q) = true)
    (hy : M.ev v.Dy α ≠ 0) (hz : M.ev v.Dz α ≠ 0) (hR : M.ev v.R α = 0) :
    Quadric.evK M q (vertPt M v.Ny v.Dy v.Nz v.Dz α) = 0 := by
  have e := eqv_spec M hc α
  rw [homog3_ev M _ _ _ _ _ _ hy hz] at e
  simp only [Model.ev, eval_pmul] at e hR
  rw [hR] at e
  have hd : M.ev v.Dy α * M.ev v.Dz α ≠ 0 := by
    intro h0
    have hpos := sq_pos' hy
    have hpos' := sq_pos' hz
    have : M.ev v.Dy α * M.ev v.Dy α * (M.ev v.Dz α * M.ev v.Dz α) = 0 := by
      have : M.ev v.Dy α * M.ev v.Dy α * (M.ev v.Dz α * M.ev v.Dz α) =
          (M.ev v.Dy α * M.ev v.Dz α) * (M.ev v.Dy α * M.ev v.Dz α) := by grind
      rw [this, h0]; grind
    have := OrderedRing.mul_pos hpos hpos'
    grind
  exact cancel_sq (sq_pos' hd) (by rw [e]; grind)

/-- **On the vertex**: all three quadrics vanish at the sample. -/
theorem onVert_spec {v : VSample} {qa qb qc : Quadric} (hc : v.onVert qa qb qc = true) {fuel : Nat}
    (hd : v.dOk fuel = true) {α : K} (hα : Denotes M v.x α) (hR : M.ev v.R α = 0) :
    Quadric.evK M qa (vertPt M v.Ny v.Dy v.Nz v.Dz α) = 0 ∧
    Quadric.evK M qb (vertPt M v.Ny v.Dy v.Nz v.Dz α) = 0 ∧
    Quadric.evK M qc (vertPt M v.Ny v.Dy v.Nz v.Dz α) = 0 := by
  obtain ⟨hy, hz⟩ := vdOk_ne M hα hd
  simp only [VSample.onVert, Bool.and_eq_true] at hc
  exact ⟨vanish_of_eqv M hc.1.1 hy hz hR, vanish_of_eqv M hc.1.2 hy hz hR, vanish_of_eqv M hc.2 hy hz hR⟩

/-- **The decided covector is the vertex's**, in every model. -/
theorem vertCov_spec [DecidableRel (α := K) (· < ·)] {v : VSample} {fuel : Nat} {α : K}
    (hα : Denotes M v.x α) (hd : v.dOk fuel = true) :
    ∀ (qs : List Quadric) (X : Cov), vertCov qs v fuel = some X →
      X = covOfK (qs.map fun q => Quadric.evK M q) (vertPt M v.Ny v.Dy v.Nz v.Dz α)
  | [], X, h => by simp [vertCov] at h; subst h; simp [covOfK]
  | q :: qs, X, h => by
    simp only [vertCov, List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
      Option.map_eq_some_iff, Option.pure_def, Option.some.injEq] at h
    obtain ⟨s, ⟨i, hi, rfl⟩, rest, hrest, rfl⟩ := h
    obtain ⟨hy, hz⟩ := vdOk_ne M hα hd
    have hq := sign_spec M (homog3 q v.Ny v.Dy v.Nz v.Dz) fuel hα hi
    rw [homog3_ev M _ _ _ _ _ _ hy hz] at hq
    have hd' : M.ev v.Dy α * M.ev v.Dz α ≠ 0 := by
      intro h0
      have := OrderedRing.mul_pos (sq_pos' hy) (sq_pos' hz)
      have e : M.ev v.Dy α * M.ev v.Dy α * (M.ev v.Dz α * M.ev v.Dz α) =
          (M.ev v.Dy α * M.ev v.Dz α) * (M.ev v.Dy α * M.ev v.Dz α) := by grind
      rw [e, h0] at this; grind
    have hq' := signIs_of_pos_mul (sq_pos' hd') hq
    have ih := vertCov_spec hα hd qs rest (by simpa [vertCov] using hrest)
    simp only [covOfK, List.map_cons, List.map_map] at ih ⊢
    rw [sgnK_of_signIs hq', ih]


/-! ## Completeness: every common point of the three is a root of `R`

Polynomials in `z` with coefficients in `ℚ[x][y]` (`ZC`, over `Alg.Sum`'s `BPoly`, whose inner
variable is here `x` and outer `y`). A **Bezout certificate** `A·q_a + B·q_b + C·q_c = R(x)`,
an identity of coefficient lists, makes `R(x) = 0` at every common zero (`bezout3_spec`): isolating
`R`'s real roots finds every vertex's `x`. -/

/-- A coefficient of a polynomial in `z`: a polynomial in `y` over `ℚ[x]`. -/
structure ZC where
  b : BPoly
  deriving Inhabited

instance : Add ZC := ⟨fun a b => ⟨padd a.b b.b⟩⟩
instance : Mul ZC := ⟨fun a b => ⟨pmul a.b b.b⟩⟩
instance : Neg ZC := ⟨fun a => ⟨pneg a.b⟩⟩
instance : Zero ZC := ⟨⟨[]⟩⟩
instance : One ZC := ⟨⟨[1]⟩⟩

/-- A polynomial in `x`, `y` and `z`. -/
abbrev TPoly := List ZC

/-- Evaluation of the coefficients at `(x, y)`. -/
def zHom (x y : K) : Hom ZC K where
  f r := evalWith (yHom M x) r.b y
  map_add a b := eval_padd (yHom M x) a.b b.b y
  map_mul a b := eval_pmul (yHom M x) a.b b.b y
  map_neg a := eval_pneg (yHom M x) a.b y
  map_zero := rfl
  map_one := by
    show (yHom M x).f 1 + y * 0 = 1
    rw [(yHom M x).map_one]; grind

/-- `q` as a polynomial in `z` over `ℚ[x][y]`. -/
def restrT (q : Quadric) : TPoly :=
  [⟨[⟨[q.c, q.lx, q.xx]⟩, ⟨[q.ly, q.xy]⟩, ⟨[q.yy]⟩]⟩, ⟨[⟨[q.lz, q.xz]⟩, ⟨[q.yz]⟩]⟩, ⟨[⟨[q.zz]⟩]⟩]

theorem restrT_ev (q : Quadric) (x y z : K) :
    evalWith (zHom M x y) (restrT q) z = Quadric.evK M q (x, y, z) := by
  simp only [restrT, evalWith_cons, evalWith_nil, zHom, yHom, Model.ev, Quadric.evK]
  grind

/-- A rational polynomial in `x`, as a trivariate constant. -/
def liftR (R : Poly) : TPoly := [⟨[⟨R⟩]⟩]

theorem liftR_ev (R : Poly) (x y z : K) : evalWith (zHom M x y) (liftR R) z = M.ev R x := by
  simp only [liftR, evalWith_cons, evalWith_nil, zHom, yHom]; grind

/-- Every coefficient is zero. -/
def tzero (t : TPoly) : Bool := t.all fun c => bzero c.b

theorem ev_tzero (t : TPoly) (h : tzero t = true) (x y z : K) : evalWith (zHom M x y) t z = 0 := by
  apply eval_of_all_zero
  intro c hc
  simp only [tzero, List.all_eq_true] at h
  exact ev_bzero M c.b (h c hc) y x

/-- The Bezout certificate `A·q_a + B·q_b + C·q_c = R(x)`. -/
def bezout3 (qa qb qc : Quadric) (A B C : TPoly) (R : Poly) : Bool :=
  tzero (psub (padd (padd (pmul A (restrT qa)) (pmul B (restrT qb))) (pmul C (restrT qc))) (liftR R))

/-- **Every vertex is a root of `R`.** -/
theorem bezout3_spec {qa qb qc : Quadric} {A B C : TPoly} {R : Poly}
    (h : bezout3 qa qb qc A B C R = true) {x y z : K}
    (ha : Quadric.evK M qa (x, y, z) = 0) (hb : Quadric.evK M qb (x, y, z) = 0)
    (hc : Quadric.evK M qc (x, y, z) = 0) : M.ev R x = 0 := by
  have e := ev_tzero M _ h x y z
  rw [eval_psub, eval_padd, eval_padd, eval_pmul, eval_pmul, eval_pmul, restrT_ev, restrT_ev,
    restrT_ev, liftR_ev, ha, hb, hc] at e
  grind

/-! ## A check that runs: three coordinate planes and the unit sphere at (1, 0, 0) -/

def planeX1 : Quadric := ⟨0, 0, 0, 0, 0, 0, 1, 0, 0, -1⟩
def planeY0 : Quadric := ⟨0, 0, 0, 0, 0, 0, 0, 1, 0, 0⟩
/-- `x = 1`, `y = −0/1`, `z = −0/1`. -/
def vAt100 : VSample := ⟨[-1, 1], [0], [1], [0], [1], .rat 1, [1, 1], [0], [0]⟩

#guard vAt100.onVert sphere1 planeY0 planeZ
#guard vAt100.dOk signFuel
#guard vertCov [sphere1, planeY0, planeZ, planeX1, planeYh] vAt100 signFuel =
  some [.zero, .zero, .zero, .zero, .neg]
-- x² − 1 = 1·(x² + y² + z² − 1) + (−y)·(y) + (−z)·(z)
#guard bezout3 sphere1 planeY0 planeZ [⟨[⟨[1]⟩]⟩] [⟨[⟨[]⟩, ⟨[-1]⟩]⟩] [⟨[]⟩, ⟨[⟨[-1]⟩]⟩] [-1, 0, 1]

end ForgeExact.DecideOnce.Curved
