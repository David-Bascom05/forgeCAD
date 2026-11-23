import ForgeExact.DecideOnce.QLine
import ForgeExact.Alg.Sum

/-!
# Q1 L2: edge samples on a rational plane, by a checked univariate representation

An edge sample is a point where a curve `q_j = q_k = 0` crosses a rational sweep plane `x = c`
(design §2.2, curved-l2-plan.md module 3). On the plane each quadric is a quadratic in `z` with
coefficients in `ℚ[y]` (`restr`). The point is represented by
* `y = α`, a root of a polynomial `R(y)` (an `Alg.Num`);
* `z = −N(α) / D(α)`, with `D(α) ≠ 0`.

The data come from an untrusted elimination (a resultant and the first subresultant), and are
checked here, in the style of `Alg.Sum.redCert`:
* `homog q c N D = D² · q(c, y, −N/D)`, a polynomial in `y` (`homog_ev`);
* **on the curve**: `homog q_j = R · Q_j` and `homog q_k = R · Q_k`, each checked as an identity
  of coefficient lists (`Poly.eqv`). Then `q_j` and `q_k` vanish at the point (`onCurve_spec`);
* **the covector**: carrier `q`'s side at the point is the sign of `homog q` at `α`, since
  `D(α)² > 0` (`edgeCov_spec`).

Completeness (every crossing of the curve with the plane is some root of `R`) is the other
half of the certificate: a Bezout identity `R = A·q_j|ₓ₌c + B·q_k|ₓ₌c` in `ℚ[y, z]`. It is
module 3's next step.
-/

namespace ForgeExact.DecideOnce.Curved

set_option linter.unusedSectionVars false

open ForgeExact ForgeExact.Ref ForgeExact.Alg ForgeExact.Quad ForgeExact.QRef Lean.Grind

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- `q` on the plane `x = c`, as `a₀(y) + a₁(y) z + a₂(y) z²`. -/
def restr (q : Quadric) (c : Rat) : Poly × Poly × Poly :=
  ([q.xx * c * c + q.lx * c + q.c, q.xy * c + q.ly, q.yy], [q.xz * c + q.lz, q.yz], [q.zz])

/-- `D² · q(c, y, −N/D)`, a polynomial in `y`. -/
def homog (q : Quadric) (c : Rat) (N D : Poly) : Poly :=
  let a := restr q c
  psub (padd (pmul (pmul D D) a.1) (pmul (pmul N N) a.2.2)) (pmul (pmul D N) a.2.1)

/-- The point `(c, y, −N(y)/D(y))`. -/
def planePt (c : Rat) (N D : Poly) (y : K) : PK K := (M.q c, y, -(M.ev N y) / M.ev D y)

theorem homog_ev (q : Quadric) (c : Rat) (N D : Poly) (y : K) (hD : M.ev D y ≠ 0) :
    M.ev (homog q c N D) y = M.ev D y * M.ev D y * Quadric.evK M q (planePt M c N D y) := by
  simp only [homog, restr, Model.ev, eval_psub, eval_padd, eval_pmul, evalWith_cons, evalWith_nil,
    Quadric.evK, planePt]
  simp only [Model.ev] at hD
  generalize evalWith M.ι D y = d at hD ⊢
  generalize evalWith M.ι N y = n
  simp only [Model.q, M.ι.map_add, M.ι.map_mul]
  have hinv : d * d⁻¹ = 1 := Field.mul_inv_cancel hD
  grind

theorem sq_pos' {d : K} (h : d ≠ 0) : 0 < d * d := by
  rcases LinearOrder.trichotomy d 0 with h' | h' | h'
  · have hn : 0 < -d := by grind
    have := OrderedRing.mul_pos hn hn
    grind
  · exact absurd h' h
  · exact OrderedRing.mul_pos h' h'

theorem signIs_of_pos_mul {d y : K} {s : Int} (hd : 0 < d) (h : SignIs (d * y) s) : SignIs y s := by
  rcases LinearOrder.trichotomy y 0 with hy | hy | hy
  · have := OrderedRing.mul_neg_of_pos_of_neg hd hy
    rcases h with ⟨rfl, h⟩ | ⟨rfl, h⟩ | ⟨rfl, _⟩
    · exact absurd (Std.lt_trans h this) Std.lt_irrefl
    · rw [h] at this; exact absurd this Std.lt_irrefl
    · exact Or.inr (Or.inr ⟨rfl, hy⟩)
  · subst hy
    have h0 : d * 0 = 0 := by grind
    rw [h0] at h
    rcases h with ⟨rfl, h⟩ | ⟨rfl, _⟩ | ⟨rfl, h⟩
    · exact absurd h Std.lt_irrefl
    · exact Or.inr (Or.inl ⟨rfl, rfl⟩)
    · exact absurd h Std.lt_irrefl
  · have := OrderedRing.mul_pos hd hy
    rcases h with ⟨rfl, _⟩ | ⟨rfl, h⟩ | ⟨rfl, h⟩
    · exact Or.inl ⟨rfl, hy⟩
    · rw [h] at this; exact absurd this Std.lt_irrefl
    · exact absurd (Std.lt_trans this h) Std.lt_irrefl

theorem cancel_sq {d v : K} (hd : 0 < d * d) (h : d * d * v = 0) : v = 0 := by
  have hne : d * d ≠ 0 := fun h0 => by rw [h0] at hd; exact Std.lt_irrefl hd
  have hinv : d * d * (d * d)⁻¹ = 1 := Field.mul_inv_cancel hne
  calc v = (d * d)⁻¹ * (d * d * v) := by grind
    _ = 0 := by rw [h]; grind

/-! ## Fuel (the decided rule, curved-l2-plan.md §2)

One fixed constant per call site. A decision that runs out answers `none`; the bridge then skips
the boolean as `Skip::Budget`, to the old path, counted. -/

/-- Fuel for a carrier's sign at a sample (`covOn`, `edgeCov`, `dOk`): `Num.sign`'s bisections,
as exact-curved's contact decisions use (QRef/Decide.lean). -/
def signFuel : Nat := 200

/-- Fuel for root isolation (`QRef.realRoots`' default). -/
def rootFuel : Nat := 60

/-- An edge sample: the plane, the representation, and the two quadrics' quotients. -/
structure ESample where
  c : Rat
  R : Poly
  N : Poly
  D : Poly
  /-- the root `α` of `R` -/
  y : Num
  Qj : Poly
  Qk : Poly

/-- The on-curve certificate for the quadrics `qj`, `qk`. -/
def ESample.onCurve (e : ESample) (qj qk : Quadric) : Bool :=
  Poly.eqv (homog qj e.c e.N e.D) (pmul e.R e.Qj) && Poly.eqv (homog qk e.c e.N e.D) (pmul e.R e.Qk)

/-- `D(α) ≠ 0`, decided. -/
def ESample.dOk (e : ESample) (fuel : Nat) : Bool :=
  match e.y.sign e.D fuel with
  | some s => s != 0
  | none => false

/-- The sample's covector over the quadrics `qs`, decided exactly (`none`: fuel spent). -/
def edgeCov (qs : List Quadric) (e : ESample) (fuel : Nat) : Option Cov :=
  qs.mapM fun q => (e.y.sign (homog q e.c e.N e.D) fuel).map sgnOfInt

theorem dOk_ne {e : ESample} {fuel : Nat} {α : K} (hα : Denotes M e.y α) (h : e.dOk fuel = true) :
    M.ev e.D α ≠ 0 := by
  unfold ESample.dOk at h
  split at h
  · rename_i s hs
    have hsg := sign_spec M e.D fuel hα hs
    intro h0
    rw [h0] at hsg
    rcases hsg with ⟨rfl, h'⟩ | ⟨rfl, _⟩ | ⟨rfl, h'⟩
    · exact Std.lt_irrefl h'
    · simp at h
    · exact Std.lt_irrefl h'
  · cases h

/-- **On the curve**: both quadrics vanish at the sample. -/
theorem onCurve_spec {e : ESample} {qj qk : Quadric} (hc : e.onCurve qj qk = true) {fuel : Nat}
    (hd : e.dOk fuel = true) {α : K} (hα : Denotes M e.y α) (hR : M.ev e.R α = 0) :
    Quadric.evK M qj (planePt M e.c e.N e.D α) = 0 ∧ Quadric.evK M qk (planePt M e.c e.N e.D α) = 0 := by
  have hD := dOk_ne M hα hd
  have hDD := sq_pos' hD
  simp only [ESample.onCurve, Bool.and_eq_true] at hc
  have ej := eqv_spec M hc.1 α
  have ek := eqv_spec M hc.2 α
  rw [homog_ev M _ _ _ _ _ hD] at ej ek
  simp only [Model.ev, eval_pmul] at ej ek
  simp only [Model.ev] at hR
  rw [hR] at ej ek
  constructor
  · have : M.ev e.D α * M.ev e.D α * Quadric.evK M qj (planePt M e.c e.N e.D α) = 0 := by
      rw [ej]; grind
    exact cancel_sq hDD this
  · have : M.ev e.D α * M.ev e.D α * Quadric.evK M qk (planePt M e.c e.N e.D α) = 0 := by
      rw [ek]; grind
    exact cancel_sq hDD this

/-- **The decided covector is the sample's**, in every model. -/
theorem edgeCov_spec [DecidableRel (α := K) (· < ·)] {e : ESample} {fuel : Nat} {α : K}
    (hα : Denotes M e.y α) (hd : e.dOk fuel = true) :
    ∀ (qs : List Quadric) (X : Cov), edgeCov qs e fuel = some X →
      X = covOfK (qs.map fun q => Quadric.evK M q) (planePt M e.c e.N e.D α)
  | [], X, h => by simp [edgeCov] at h; subst h; simp [covOfK]
  | q :: qs, X, h => by
    simp only [edgeCov, List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
      Option.map_eq_some_iff, Option.pure_def, Option.some.injEq] at h
    obtain ⟨s, ⟨i, hi, rfl⟩, rest, hrest, rfl⟩ := h
    have hD := dOk_ne M hα hd
    have hq := sign_spec M (homog q e.c e.N e.D) fuel hα hi
    rw [homog_ev M _ _ _ _ _ hD] at hq
    have hq' := signIs_of_pos_mul (sq_pos' hD) hq
    have ih := edgeCov_spec hα hd qs rest (by simpa [edgeCov] using hrest)
    simp only [covOfK, List.map_cons, List.map_map] at ih ⊢
    rw [sgnK_of_signIs hq', ih]


/-! ## Completeness: every crossing is a root of `R`

`restrB q c` is `q` on the plane `x = c` as a polynomial in `z` with coefficients in `ℚ[y]`
(`Alg.Sum`'s `BPoly`, evaluated by `yHom`). A **Bezout certificate** is `A`, `B` with
`A·q_j|ₓ₌c + B·q_k|ₓ₌c = R`, an identity of coefficient lists checked by `bzero`. Then at every
point of the plane on both quadrics, `R(y) = 0` (`bezout_spec`): every crossing of the curve with
the plane is some root of `R`, so isolating all of `R`'s real roots finds every crossing. -/

/-- `q` on `x = c`, as a polynomial in `z` over `ℚ[y]`. -/
def restrB (q : Quadric) (c : Rat) : BPoly :=
  let a := restr q c
  [⟨a.1⟩, ⟨a.2.1⟩, ⟨a.2.2⟩]

theorem restrB_ev (q : Quadric) (c : Rat) (y z : K) :
    evalWith (yHom M y) (restrB q c) z = Quadric.evK M q (M.q c, y, z) := by
  simp only [restrB, restr, evalWith_cons, evalWith_nil, yHom, Model.ev, Quadric.evK]
  simp only [Model.q, M.ι.map_add, M.ι.map_mul]
  grind

/-- The Bezout certificate `A·q_j + B·q_k = R` on the plane `x = c`. -/
def bezout (qj qk : Quadric) (c : Rat) (A B : BPoly) (R : Poly) : Bool :=
  bzero (psub (padd (pmul A (restrB qj c)) (pmul B (restrB qk c))) (liftY R))

/-- **Every crossing is a root of `R`.** -/
theorem bezout_spec {qj qk : Quadric} {c : Rat} {A B : BPoly} {R : Poly}
    (h : bezout qj qk c A B R = true) {y z : K}
    (hj : Quadric.evK M qj (M.q c, y, z) = 0) (hk : Quadric.evK M qk (M.q c, y, z) = 0) :
    M.ev R y = 0 := by
  have e := ev_bzero M _ h z y
  rw [eval_psub, eval_padd, eval_pmul, eval_pmul, restrB_ev, restrB_ev, ev_liftY, hj, hk] at e
  grind

/-! ## A check that runs (a test, not a theorem)

The unit sphere and the plane `z = 0` meet in the unit circle; on the sweep plane `x = 0` it
crosses at `(0, ±1, 0)`. The sample `y = 1`, `z = −0/1` is certified on both, and its side of
the plane `y = 1/2` is positive. -/

def sphere1 : Quadric := ⟨1, 1, 1, 0, 0, 0, 0, 0, 0, -1⟩
def planeZ : Quadric := ⟨0, 0, 0, 0, 0, 0, 0, 0, 1, 0⟩
def planeYh : Quadric := ⟨0, 0, 0, 0, 0, 0, 0, 1, 0, -1/2⟩
def circleAt0 : ESample := ⟨0, [-1, 0, 1], [0], [1], .rat 1, [1], [0]⟩

#guard circleAt0.onCurve sphere1 planeZ
#guard circleAt0.dOk signFuel
#guard edgeCov [sphere1, planeZ, planeYh] circleAt0 signFuel = some [.zero, .zero, .pos]
-- on x = 0: sphere y² + z² − 1, plane z; y² − 1 = 1·(sphere) + (−z)·(plane)
#guard bezout sphere1 planeZ 0 [⟨[1]⟩] [⟨[]⟩, ⟨[-1]⟩] [-1, 0, 1]

end ForgeExact.DecideOnce.Curved
