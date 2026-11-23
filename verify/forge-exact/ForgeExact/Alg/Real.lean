import ForgeExact.Alg.Model

/-!
# Real algebraic numbers: isolating intervals, exact sign and comparison

A real algebraic number is a rational, or a polynomial `p` with an isolating interval `(a, b)`:
`p(a) p(b) < 0` and `p` has one root in `(a, b)`, certified once by `onePosRoot` of the Möbius
transform (`Iso.ok`). The number it denotes in a model is that root (`Denotes`).

* `Num.cmpRat`: comparison with a rational, by the sign of `p` there.
* `Num.sign`: the sign of a polynomial `q` at the number. Zero is certified by a common factor
  `g` of `p` and `q` (checked: `p = g hp`, `q = g hq`) that changes sign on `(a, b)`; a sign
  ≠ 0 by bisecting until `q` has no root in the interval (`noPosRoot` of its transform), then
  reading `q` at the midpoint.
* `Num.cmp`: comparison of two numbers.
* `Num.neg`, `Num.addRat`, `Num.mulRat`, `Num.inv`: arithmetic with rationals, by transforming
  the polynomial (Sum.lean has the sum and product of two algebraic numbers).

Each is proven against every model (`cmpRat_spec`, `sign_spec`, `cmp_spec`, ...). The algorithms
that propose (gcd, bisection) carry no proof obligation: a failed check, or fuel run out, answers
`none`, never a wrong sign.
-/

namespace ForgeExact.Alg

open Lean.Grind

set_option linter.unusedSectionVars false

/-! ## Roots in an interval through the Möbius transform -/

/-- `(1 + x)^(n-1) p((a + b x)/(1 + x))` for `p` of length `n`: its positive roots are `p`'s roots
in `(a, b)`. -/
def moebius (a b : Rat) : Poly → Poly
  | [] => []
  | c :: p => padd (pscale c (ppow [1, 1] p.length)) (pmul [a, b] (moebius a b p))

section Moebius

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem ev_lin (a b : Rat) (x : K) : M.ev [a, b] x = M.q a + M.q b * x := by
  simp only [Model.ev, evalWith_cons, evalWith_nil]; grind

theorem moebius_eval (a b : Rat) (x : K) (hx : 1 + x ≠ 0) : ∀ p : Poly,
    (1 + x) * M.ev (moebius a b p) x =
      (1 + x) ^ p.length * M.ev p ((M.q a + M.q b * x) / (1 + x))
  | [] => by simp [moebius, Model.ev] <;> grind
  | c :: p => by
    have ih := moebius_eval a b x hx p
    have e1 : M.ev [1, 1] x = 1 + x := by rw [ev_lin]; simp; grind
    have e2 := ev_lin M a b x
    simp only [moebius, Model.ev] at ih e1 e2 ⊢
    rw [eval_padd, eval_pscale, eval_pmul, eval_ppow, e1, e2, evalWith_cons, List.length_cons]
    have hφ : (M.q a + M.q b * x) = (1 + x) * ((M.q a + M.q b * x) / (1 + x)) := by grind
    generalize (M.q a + M.q b * x) / (1 + x) = φ at hφ ih ⊢
    rw [hφ, Semiring.pow_succ]
    generalize evalWith M.ι p φ = P at ih ⊢
    generalize evalWith M.ι (moebius a b p) x = E at ih ⊢
    generalize (1 + x) ^ p.length = W at ih ⊢
    have : (1 + x) * ((1 + x) * φ * E) = (1 + x) * φ * ((1 + x) * E) := by grind
    rw [show (1 + x) * (M.ι.f c * W + (1 + x) * φ * E) =
      (1 + x) * M.ι.f c * W + (1 + x) * ((1 + x) * φ * E) by grind, this, ih]
    grind

/-- The point of `(0, ∞)` that the transform sends to `z ∈ (a, b)`. -/
theorem moebius_root (a b : Rat) (p : Poly) {z : K} (hza : M.q a < z) (hzb : z < M.q b)
    (hz : M.ev p z = 0) :
    0 < (z - M.q a) / (M.q b - z) ∧ M.ev (moebius a b p) ((z - M.q a) / (M.q b - z)) = 0 := by
  have hbz : 0 < M.q b - z := by grind
  have hza' : 0 < z - M.q a := by grind
  have hx : 0 < (z - M.q a) / (M.q b - z) := by
    have := OrderedRing.mul_pos hza' (show 0 < 1 / (M.q b - z) by
      apply Classical.byContradiction; intro hn
      have h1 : (M.q b - z) * (1 / (M.q b - z)) = 1 := by grind
      have : (M.q b - z) * (1 / (M.q b - z)) ≤ 0 :=
        OrderedRing.mul_nonpos_of_nonneg_of_nonpos (by grind) (by grind)
      grind)
    grind
  refine ⟨hx, ?_⟩
  generalize hX : (z - M.q a) / (M.q b - z) = X at hx ⊢
  have h1 : 1 + X ≠ 0 := by grind
  have hφ : (M.q a + M.q b * X) / (1 + X) = z := by
    have : X * (M.q b - z) = z - M.q a := by rw [← hX]; grind
    have h1' : 1 + X ≠ 0 := h1
    grind
  have := moebius_eval M a b X h1 p
  rw [hφ, hz] at this
  have : (1 + X) * M.ev (moebius a b p) X = 0 := by grind
  grind

/-- One root at most in `(a, b)`. -/
theorem moebius_unique (a b : Rat) (p : Poly) (h : onePosRoot (moebius a b p) = true) {y z : K}
    (hya : M.q a < y) (hyb : y < M.q b) (hy : M.ev p y = 0)
    (hza : M.q a < z) (hzb : z < M.q b) (hz : M.ev p z = 0) : y = z := by
  obtain ⟨hy0, hy1⟩ := moebius_root M a b p hya hyb hy
  obtain ⟨hz0, hz1⟩ := moebius_root M a b p hza hzb hz
  have e := onePosRoot_spec M _ h hy0 hz0 hy1 hz1
  have h1 : M.q b - y ≠ 0 := by grind
  have h2 : M.q b - z ≠ 0 := by grind
  have e' : (y - M.q a) * (M.q b - z) = (z - M.q a) * (M.q b - y) := by
    generalize M.q a = A at *
    generalize M.q b = B at *
    have h3 : (y - A) / (B - y) * (B - y) = y - A := by grind
    have h4 : (z - A) / (B - z) * (B - z) = z - A := by grind
    rw [← h3, ← h4, e]; grind
  have hab : M.q b - M.q a ≠ 0 := by grind
  have : (y - z) * (M.q b - M.q a) = 0 := by grind
  grind

/-- No root in `(a, b)`. -/
theorem moebius_none (a b : Rat) (p : Poly) (h : noPosRoot (moebius a b p) = true) {z : K}
    (hza : M.q a < z) (hzb : z < M.q b) : M.ev p z ≠ 0 := by
  intro hz
  obtain ⟨h0, h1⟩ := moebius_root M a b p hza hzb hz
  exact noPosRoot_spec M _ h h0 h1

end Moebius

/-! ## Algebraic numbers -/

/-- A polynomial with an interval, meant to isolate one root. -/
structure Iso where
  p : Poly
  a : Rat
  b : Rat
  deriving Repr, Inhabited, BEq

/-- The isolation certificate. -/
def Iso.ok (r : Iso) : Bool :=
  decide (r.a < r.b) && decide (Poly.ev r.p r.a * Poly.ev r.p r.b < 0) &&
    onePosRoot (moebius r.a r.b r.p)

/-- A real algebraic number: a rational, or an isolated root. -/
inductive Num
  | rat (c : Rat)
  | iso (r : Iso)
  deriving Repr, Inhabited, BEq

/-- `s` is the sign of `y`. -/
def SignIs {K : Type} [Field K] [LT K] (y : K) (s : Int) : Prop :=
  (s = 1 ∧ 0 < y) ∨ (s = 0 ∧ y = 0) ∨ (s = -1 ∧ y < 0)

/-- `o` is how `x` compares with `y`. -/
def CmpIs {K : Type} [LT K] (x y : K) : Ordering → Prop
  | .lt => x < y
  | .eq => x = y
  | .gt => y < x

section Denote

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- The number `n` denotes `x` in the model: for an isolated root, `x` is the one root of `p`
in `(a, b)`. -/
def Denotes : Num → K → Prop
  | .rat c, x => x = M.q c
  | .iso r, x => M.q r.a < x ∧ x < M.q r.b ∧ M.ev r.p x = 0 ∧
      (∀ y, M.q r.a < y → y < M.q r.b → M.ev r.p y = 0 → y = x) ∧
      Poly.ev r.p r.a * Poly.ev r.p r.b < 0

/-- **A certified isolating interval denotes exactly one number.** -/
theorem Iso.denotes_of_ok (r : Iso) (h : r.ok = true) : ∃ x, Denotes M (.iso r) x := by
  simp only [Iso.ok, Bool.and_eq_true, decide_eq_true_eq] at h
  obtain ⟨⟨hab, hs⟩, hu⟩ := h
  have hab' := (M.q_lt).mpr hab
  have hs' : M.ev r.p (M.q r.a) * M.ev r.p (M.q r.b) < 0 := by
    rw [M.ev_q, M.ev_q, ← M.q_mul]; exact (M.q_neg_iff).mpr hs
  obtain ⟨x, h1, h2, h3⟩ := M.root_between r.p hab' hs'
  exact ⟨x, h1, h2, h3, fun y hy1 hy2 hy3 => moebius_unique M r.a r.b r.p hu hy1 hy2 hy3 h1 h2 h3, hs⟩

theorem Denotes.unique {n : Num} {x y : K} (hx : Denotes M n x) (hy : Denotes M n y) : x = y := by
  cases n with
  | rat c => simp only [Denotes] at hx hy; rw [hx, hy]
  | iso r => exact (hx.2.2.2.1 y hy.1 hy.2.1 hy.2.2.1).symm

end Denote

/-! ## Comparison with a rational -/

/-- How the number compares with the rational `c`. -/
def Num.cmpRat : Num → Rat → Ordering
  | .rat d, c => if d < c then .lt else if d = c then .eq else .gt
  | .iso r, c =>
    if c ≤ r.a then .gt
    else if r.b ≤ c then .lt
    else
      let v := Poly.ev r.p c
      if v == 0 then .eq
      else if v * Poly.ev r.p r.b < 0 then .gt
      else .lt

section CmpRat

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- Signs of rationals: `u w < 0`, `0 ≤ v w`, `v ≠ 0` give `u v < 0`. -/
theorem sign_swap {u v w : Rat} (h1 : u * w < 0) (h2 : 0 ≤ v * w) (h3 : v ≠ 0) : u * v < 0 := by
  have h1' : 0 < (-u) * w := by grind
  have goal : 0 < (-u) * v := by
    rcases OrderedRing.mul_pos_iff.mp h1' with ⟨hu, hw⟩ | ⟨hu, hw⟩ <;>
      rcases OrderedRing.mul_nonneg_iff.mp h2 with ⟨hv, hw'⟩ | ⟨hv, hw'⟩
    all_goals first
      | grind
      | exact OrderedRing.mul_pos_iff.mpr (Or.inl ⟨hu, by grind⟩)
      | exact OrderedRing.mul_pos_iff.mpr (Or.inr ⟨hu, by grind⟩)
  grind

theorem cmpRat_spec (n : Num) (c : Rat) {x : K} (hx : Denotes M n x) :
    CmpIs x (M.q c) (n.cmpRat c) := by
  cases n with
  | rat d =>
    simp only [Denotes] at hx; subst hx
    simp only [Num.cmpRat]
    split
    · next h => exact (M.q_lt).mpr h
    split
    · next _ h => subst h; rfl
    · next h h' => exact (M.q_lt).mpr (by grind)
  | iso r =>
    obtain ⟨h1, h2, h3, h4, h5⟩ := hx
    simp only [Num.cmpRat]
    split
    · next hc => have := (M.q_le).mpr hc; simp only [CmpIs]; grind
    split
    · next _ hc => have := (M.q_le).mpr hc; simp only [CmpIs]; grind
    next hca hcb =>
    have hca' := (M.q_lt).mpr (Rat.not_le.mp hca)
    have hcb' := (M.q_lt).mpr (Rat.not_le.mp hcb)
    have hvq : M.ev r.p (M.q c) = M.q (Poly.ev r.p c) := M.ev_q c r.p
    split
    · next hv =>
      have : Poly.ev r.p c = 0 := by simpa using hv
      exact (h4 _ hca' hcb' (by rw [hvq, this]; simp)).symm
    split
    · next hv hs =>
      -- a root in (c, b): it is x
      have hs' : M.ev r.p (M.q c) * M.ev r.p (M.q r.b) < 0 := by
        rw [hvq, M.ev_q, ← M.q_mul]; exact (M.q_neg_iff).mpr hs
      obtain ⟨y, hy1, hy2, hy3⟩ := M.root_between r.p hcb' hs'
      have := h4 y (by grind) hy2 hy3
      simp only [CmpIs]; grind
    · next hv hs =>
      -- otherwise p(a) and p(c) differ in sign: a root in (a, c), which is x
      have hv0 : Poly.ev r.p c ≠ 0 := by simpa using hv
      have hac : Poly.ev r.p r.a * Poly.ev r.p c < 0 := sign_swap h5 (Rat.not_lt.mp hs) hv0
      have hs' : M.ev r.p (M.q r.a) * M.ev r.p (M.q c) < 0 := by
        rw [hvq, M.ev_q, ← M.q_mul]; exact (M.q_neg_iff).mpr hac
      obtain ⟨y, hy1, hy2, hy3⟩ := M.root_between r.p hca' hs'
      have := h4 y hy1 (by grind) hy3
      simp only [CmpIs]; grind

end CmpRat

/-! ## Evaluation over ℚ and polynomial identities -/

/-- ℚ in itself. -/
def idHom : Hom Rat Rat where
  f := id
  map_add _ _ := rfl
  map_mul _ _ := rfl
  map_neg _ := rfl
  map_zero := rfl
  map_one := rfl

theorem Poly.ev_eq (p : Poly) (x : Rat) : Poly.ev p x = evalWith idHom p x := by
  induction p with
  | nil => rfl
  | cons c p ih => simp only [Poly.ev, List.foldr_cons, evalWith_cons] at ih ⊢; rw [← ih]; rfl

/-- The sign of a rational. -/
def qsgn (c : Rat) : Int := if 0 < c then 1 else if c < 0 then -1 else 0

section Ident

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- A checked identity of coefficient lists holds at every point. -/
theorem eqv_spec {p q : Poly} (h : Poly.eqv p q = true) (x : K) : M.ev p x = M.ev q x := by
  simp only [Poly.eqv, Poly.isZero, List.all_eq_true, beq_iff_eq] at h
  have := eval_of_all_zero M.ι (psub p q) x fun c hc => by rw [h c hc]; exact M.ι.map_zero
  rw [eval_psub] at this; simp only [Model.ev]; grind

theorem eqv_spec_q {p q : Poly} (h : Poly.eqv p q = true) (x : Rat) : Poly.ev p x = Poly.ev q x := by
  simp only [Poly.eqv, Poly.isZero, List.all_eq_true, beq_iff_eq] at h
  have := eval_of_all_zero idHom (psub p q) x fun c hc => by rw [h c hc]; rfl
  rw [eval_psub, ← Poly.ev_eq, ← Poly.ev_eq] at this; grind

theorem qsgn_spec (c : Rat) : SignIs (M.q c) (qsgn c) := by
  unfold qsgn SignIs
  split
  · next h => exact Or.inl ⟨rfl, M.q_pos.mpr h⟩
  split
  · next _ h => exact Or.inr (Or.inr ⟨rfl, M.q_neg_iff.mpr h⟩)
  · next h h' => exact Or.inr (Or.inl ⟨rfl, by rw [show c = 0 by grind]; simp⟩)

/-- With no root of `q` in `(A, B)`, `q` keeps one sign there. -/
theorem sign_const (q : Poly) {A B : K} (hq : ∀ z, A < z → z < B → M.ev q z ≠ 0)
    {u v : K} (hu1 : A < u) (hu2 : u < B) (hv1 : A < v) (hv2 : v < B) :
    0 ≤ M.ev q u * M.ev q v := by
  apply Classical.byContradiction; intro hn
  have hn' : M.ev q u * M.ev q v < 0 := by grind
  rcases LinearOrder.trichotomy u v with h | h | h
  · obtain ⟨z, h1, h2, h3⟩ := M.root_between q h hn'
    exact hq z (by grind) (by grind) h3
  · subst h
    have := OrderedRing.sq_nonneg (a := M.ev q u); grind
  · obtain ⟨z, h1, h2, h3⟩ := M.root_between q h (by grind)
    exact hq z (by grind) (by grind) h3

end Ident

/-! ## Bisection -/

/-- The interval halved, the root kept: a rational if it is the midpoint. -/
def Num.bisect : Num → Num
  | .rat c => .rat c
  | .iso r =>
    let m := (r.a + r.b) / 2
    let v := Poly.ev r.p m
    if v == 0 then .rat m
    else if Poly.ev r.p r.a * v < 0 then .iso ⟨r.p, r.a, m⟩
    else .iso ⟨r.p, m, r.b⟩

/-- The interval's width (0 for a rational). -/
def Num.width : Num → Rat
  | .rat _ => 0
  | .iso r => r.b - r.a

/-- A rational box round the number: `lo ≤ x ≤ hi`. -/
def Num.lo : Num → Rat
  | .rat c => c
  | .iso r => r.a
def Num.hi : Num → Rat
  | .rat c => c
  | .iso r => r.b

section Bisect

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem Denotes.lo_le {n : Num} {x : K} (h : Denotes M n x) : M.q n.lo ≤ x := by
  cases n with
  | rat c => simp only [Denotes] at h; simp only [Num.lo]; grind
  | iso r => simp only [Num.lo]; exact Preorder.le_of_lt h.1

theorem Denotes.le_hi {n : Num} {x : K} (h : Denotes M n x) : x ≤ M.q n.hi := by
  cases n with
  | rat c => simp only [Denotes] at h; simp only [Num.hi]; grind
  | iso r => simp only [Num.hi]; exact Preorder.le_of_lt h.2.1

theorem bisect_spec {n : Num} {x : K} (hx : Denotes M n x) : Denotes M n.bisect x := by
  cases n with
  | rat c => exact hx
  | iso r =>
    obtain ⟨h1, h2, h3, h4, h5⟩ := hx
    have hab : r.a < r.b := (M.q_lt).mp (by grind)
    have hm1 : r.a < (r.a + r.b) / 2 := by grind
    have hm2 : (r.a + r.b) / 2 < r.b := by grind
    have hm1' := (M.q_lt).mpr hm1
    have hm2' := (M.q_lt).mpr hm2
    have hvq := M.ev_q ((r.a + r.b) / 2) r.p
    simp only [Num.bisect]
    split
    · next hv =>
      have : Poly.ev r.p ((r.a + r.b) / 2) = 0 := by simpa using hv
      simp only [Denotes]
      exact (h4 _ hm1' hm2' (by rw [hvq, this]; simp)).symm
    split
    · next hv hs =>
      have hs' : M.ev r.p (M.q r.a) * M.ev r.p (M.q ((r.a + r.b) / 2)) < 0 := by
        rw [hvq, M.ev_q, ← M.q_mul]; exact (M.q_neg_iff).mpr hs
      obtain ⟨y, hy1, hy2, hy3⟩ := M.root_between r.p hm1' hs'
      have hyx := h4 y hy1 (by grind) hy3
      subst hyx
      exact ⟨hy1, hy2, hy3, fun z hz1 hz2 hz3 => h4 z hz1 (by grind) hz3, hs⟩
    · next hv hs =>
      have hv0 : Poly.ev r.p ((r.a + r.b) / 2) ≠ 0 := by simpa using hv
      have hmb : Poly.ev r.p ((r.a + r.b) / 2) * Poly.ev r.p r.b < 0 := by
        have := sign_swap (u := Poly.ev r.p r.b) (w := Poly.ev r.p r.a)
          (v := Poly.ev r.p ((r.a + r.b) / 2)) (by grind) (by grind) hv0
        grind
      have hs' : M.ev r.p (M.q ((r.a + r.b) / 2)) * M.ev r.p (M.q r.b) < 0 := by
        rw [hvq, M.ev_q, ← M.q_mul]; exact (M.q_neg_iff).mpr hmb
      obtain ⟨y, hy1, hy2, hy3⟩ := M.root_between r.p hm2' hs'
      have hyx := h4 y (by grind) hy2 hy3
      subst hyx
      exact ⟨hy1, hy2, hy3, fun z hz1 hz2 hz3 => h4 z (by grind) hz2 hz3, hmb⟩

/-- Bisected `k` times. -/
def Num.refine : Nat → Num → Num
  | 0, n => n
  | k + 1, n => Num.refine k n.bisect

theorem refine_spec : ∀ (k : Nat) {n : Num} {x : K}, Denotes M n x → Denotes M (n.refine k) x
  | 0, _, _, h => h
  | k + 1, _, _, h => refine_spec k (bisect_spec M h)

end Bisect

/-! ## The sign of a polynomial at an algebraic number -/

/-- The nonzero sign, by bisecting until `q` has no root in the interval. -/
def Num.signNZ (q : Poly) : Nat → Num → Option Int
  | _, .rat c => some (qsgn (Poly.ev q c))
  | 0, .iso _ => none
  | fuel + 1, .iso r =>
    if noPosRoot (moebius r.a r.b q) then some (qsgn (Poly.ev q ((r.a + r.b) / 2)))
    else Num.signNZ q fuel (Num.bisect (.iso r))

/-- `q` vanishes at the root of `r`, certified: a common factor of `r.p` and `q` that changes sign
on `r`'s interval. -/
def Iso.zeroAt (r : Iso) (q : Poly) : Bool :=
  let g := Poly.gcd r.p q
  Poly.eqv r.p (pmul g (Poly.pdiv r.p g)) && Poly.eqv q (pmul g (Poly.pdiv q g)) &&
    decide (Poly.ev g r.a * Poly.ev g r.b < 0)

/-- **The sign of `q` at the number**, or `none` when the fuel runs out. -/
def Num.sign (q : Poly) (fuel : Nat) : Num → Option Int
  | .rat c => some (qsgn (Poly.ev q c))
  | .iso r => if r.zeroAt q then some 0 else Num.signNZ q fuel (.iso r)

section Sign

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem signNZ_spec (q : Poly) : ∀ (fuel : Nat) {n : Num} {x : K} {s : Int}, Denotes M n x →
    n.signNZ q fuel = some s → SignIs (M.ev q x) s
  | _, .rat c, x, s, hx, h => by
    simp only [Num.signNZ, Option.some.injEq] at h; subst h
    simp only [Denotes] at hx; subst hx; rw [M.ev_q]; exact qsgn_spec M _
  | 0, .iso _, _, _, _, h => by simp [Num.signNZ] at h
  | fuel + 1, .iso r, x, s, hx, h => by
    simp only [Num.signNZ] at h
    split at h
    · next hn =>
      simp only [Option.some.injEq] at h; subst h
      obtain ⟨h1, h2, h3, h4, h5⟩ := hx
      have hab : r.a < r.b := (M.q_lt).mp (by grind)
      have hm1 := (M.q_lt).mpr (show r.a < (r.a + r.b) / 2 by grind)
      have hm2 := (M.q_lt).mpr (show (r.a + r.b) / 2 < r.b by grind)
      have hnr : ∀ z, M.q r.a < z → z < M.q r.b → M.ev q z ≠ 0 :=
        fun z hz1 hz2 => moebius_none M r.a r.b q hn hz1 hz2
      have hc := sign_const M q hnr h1 h2 hm1 hm2
      have hx0 := hnr x h1 h2
      have hm0 := hnr _ hm1 hm2
      have hs := qsgn_spec M (Poly.ev q ((r.a + r.b) / 2))
      rw [← M.ev_q] at hs
      generalize M.ev q (M.q ((r.a + r.b) / 2)) = V at hs hc hm0
      generalize M.ev q x = U at hc hx0 ⊢
      simp only [SignIs] at hs ⊢
      rcases hs with ⟨e, hV⟩ | ⟨e, hV⟩ | ⟨e, hV⟩ <;> rw [e]
      · left; refine ⟨rfl, ?_⟩
        apply Classical.byContradiction; intro hU
        have : U * V < 0 := OrderedRing.mul_neg_of_neg_of_pos (by grind) hV
        grind
      · exact absurd hV hm0
      · right; right; refine ⟨rfl, ?_⟩
        apply Classical.byContradiction; intro hU
        have : U * V < 0 := OrderedRing.mul_neg_of_pos_of_neg (by grind) hV
        grind
    · exact signNZ_spec q fuel (bisect_spec M hx) h

theorem zeroAt_spec (r : Iso) (q : Poly) (h : r.zeroAt q = true) {x : K}
    (hx : Denotes M (.iso r) x) : M.ev q x = 0 := by
  simp only [Iso.zeroAt, Bool.and_eq_true, decide_eq_true_eq] at h
  obtain ⟨⟨hp, hq⟩, hg⟩ := h
  obtain ⟨h1, h2, h3, h4, h5⟩ := hx
  generalize Poly.gcd r.p q = g at hp hq hg
  have hg' : M.ev g (M.q r.a) * M.ev g (M.q r.b) < 0 := by
    rw [M.ev_q, M.ev_q, ← M.q_mul]; exact (M.q_neg_iff).mpr hg
  obtain ⟨y, hy1, hy2, hy3⟩ := M.root_between g (by grind) hg'
  have hpy : M.ev r.p y = 0 := by
    rw [eqv_spec M hp, Model.ev, eval_pmul]; simp only [Model.ev] at hy3; rw [hy3]; grind
  have := h4 y hy1 hy2 hpy
  subst this
  rw [eqv_spec M hq, Model.ev, eval_pmul]; simp only [Model.ev] at hy3; rw [hy3]; grind

/-- **The sign is exact.** -/
theorem sign_spec (q : Poly) (fuel : Nat) {n : Num} {x : K} {s : Int} (hx : Denotes M n x)
    (h : n.sign q fuel = some s) : SignIs (M.ev q x) s := by
  cases n with
  | rat c =>
    simp only [Num.sign, Option.some.injEq] at h; subst h
    simp only [Denotes] at hx; subst hx; rw [M.ev_q]; exact qsgn_spec M _
  | iso r =>
    simp only [Num.sign] at h
    split at h
    · next hz =>
      simp only [Option.some.injEq] at h; subst h
      exact Or.inr (Or.inl ⟨rfl, zeroAt_spec M r q hz hx⟩)
    · exact signNZ_spec M q fuel hx h

end Sign

/-! ## Comparison of two algebraic numbers -/

/-- How `n` compares with `m`, or `none` when the fuel runs out. -/
def Num.cmp : Nat → Num → Num → Option Ordering
  | _, n, .rat c => some (n.cmpRat c)
  | _, .rat c, m => some (m.cmpRat c).swap
  | 0, .iso _, .iso _ => none
  | fuel + 1, .iso r, .iso s =>
    if r.b ≤ s.a then some .lt
    else if s.b ≤ r.a then some .gt
    else if (Num.iso r).sign s.p fuel == some 0 && (Num.iso r).cmpRat s.a == .gt &&
        (Num.iso r).cmpRat s.b == .lt then some .eq
    else Num.cmp fuel (Num.iso r).bisect (Num.iso s).bisect

section Cmp

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem cmpIs_swap {x y : K} {o : Ordering} (h : CmpIs y x o) : CmpIs x y o.swap := by
  cases o <;> simp only [CmpIs, Ordering.swap] at h ⊢ <;> grind

/-- **Comparison is exact.** -/
theorem cmp_spec : ∀ (fuel : Nat) {n m : Num} {x y : K} {o : Ordering}, Denotes M n x →
    Denotes M m y → Num.cmp fuel n m = some o → CmpIs x y o
  | _, n, .rat c, x, y, o, hx, hy, h => by
    cases n <;> (simp only [Num.cmp, Option.some.injEq] at h; subst h; simp only [Denotes] at hy; subst hy; exact cmpRat_spec M _ c hx)
  | _, .rat c, .iso s, x, y, o, hx, hy, h => by
    simp only [Num.cmp, Option.some.injEq] at h; subst h
    simp only [Denotes] at hx; subst hx
    exact cmpIs_swap (cmpRat_spec M _ c hy)
  | 0, .iso _, .iso _, _, _, _, _, _, h => by simp [Num.cmp] at h
  | fuel + 1, .iso r, .iso s, x, y, o, hx, hy, h => by
    simp only [Num.cmp] at h
    split at h
    · next hb =>
      simp only [Option.some.injEq] at h; subst h
      have := (M.q_le).mpr hb; simp only [CmpIs]; have := hx.2.1; have := hy.1; grind
    split at h
    · next _ hb =>
      simp only [Option.some.injEq] at h; subst h
      have := (M.q_le).mpr hb; simp only [CmpIs]; have := hy.2.1; have := hx.1; grind
    split at h
    · next _ _ he =>
      simp only [Option.some.injEq] at h; subst h
      simp only [Bool.and_eq_true, beq_iff_eq] at he
      obtain ⟨⟨hz, ha⟩, hb⟩ := he
      have hz' := sign_spec M s.p fuel hx hz
      have ha' := cmpRat_spec M _ s.a hx; rw [ha] at ha'
      have hb' := cmpRat_spec M _ s.b hx; rw [hb] at hb'
      simp only [SignIs, CmpIs] at hz' ha' hb' ⊢
      have hz0 : M.ev s.p x = 0 := by grind
      exact hy.2.2.2.1 x ha' hb' hz0
    · exact cmp_spec fuel (bisect_spec M hx) (bisect_spec M hy) h

end Cmp

/-! ## Arithmetic with rationals -/

/-- `-x`. -/
def Num.neg : Num → Num
  | .rat c => .rat (-c)
  | .iso r => .iso ⟨pcomp r.p [0, -1], -r.b, -r.a⟩

/-- `x + c`. -/
def Num.addRat (c : Rat) : Num → Num
  | .rat d => .rat (d + c)
  | .iso r => .iso ⟨pcomp r.p [-c, 1], r.a + c, r.b + c⟩

/-- `c x`, for `c > 0`. -/
def Num.mulPos (c : Rat) : Num → Num
  | .rat d => .rat (c * d)
  | .iso r => .iso ⟨pcomp r.p [0, 1 / c], c * r.a, c * r.b⟩

section Arith

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem ev_comp_lin (p : Poly) (u v : Rat) (x : K) :
    M.ev (pcomp p [u, v]) x = M.ev p (M.q u + M.q v * x) := by
  simp only [Model.ev]; rw [eval_pcomp]; congr 1; exact ev_lin M u v x

theorem evq_comp_lin (p : Poly) (u v : Rat) (x : Rat) :
    Poly.ev (pcomp p [u, v]) x = Poly.ev p (u + v * x) := by
  rw [Poly.ev_eq, Poly.ev_eq, eval_pcomp]; congr 1
  simp only [evalWith_cons, evalWith_nil]; show u + x * (v + x * 0) = u + v * x; grind

theorem neg_spec {n : Num} {x : K} (hx : Denotes M n x) : Denotes M n.neg (-x) := by
  cases n with
  | rat c => simp only [Denotes, Num.neg] at hx ⊢; rw [hx, M.q_neg]
  | iso r =>
    obtain ⟨h1, h2, h3, h4, h5⟩ := hx
    have e : ∀ y : K, M.ev (pcomp r.p [0, -1]) y = M.ev r.p (-y) := fun y => by
      rw [ev_comp_lin, M.q_zero, M.q_neg, M.q_one]; congr 1; grind
    simp only [Num.neg, Denotes, M.q_neg]
    refine ⟨by grind, by grind, by rw [e, show - -x = x by grind]; exact h3, fun y hy1 hy2 hy3 => ?_, ?_⟩
    · rw [e] at hy3; have := h4 (-y) (by grind) (by grind) hy3; grind
    · rw [evq_comp_lin, evq_comp_lin]
      have : (0 : Rat) + -1 * -r.b = r.b := by grind
      have h' : (0 : Rat) + -1 * -r.a = r.a := by grind
      rw [this, h']; grind

theorem addRat_spec (c : Rat) {n : Num} {x : K} (hx : Denotes M n x) :
    Denotes M (n.addRat c) (x + M.q c) := by
  cases n with
  | rat d => simp only [Denotes, Num.addRat] at hx ⊢; rw [hx, M.q_add]
  | iso r =>
    obtain ⟨h1, h2, h3, h4, h5⟩ := hx
    have e : ∀ y : K, M.ev (pcomp r.p [-c, 1]) y = M.ev r.p (y - M.q c) := fun y => by
      rw [ev_comp_lin, M.q_neg, M.q_one]; congr 1; grind
    simp only [Num.addRat, Denotes, M.q_add]
    refine ⟨by grind, by grind, by rw [e, show x + M.q c - M.q c = x by grind]; exact h3, fun y hy1 hy2 hy3 => ?_, ?_⟩
    · rw [e] at hy3; have := h4 (y - M.q c) (by grind) (by grind) hy3; grind
    · rw [evq_comp_lin, evq_comp_lin]
      have : -c + 1 * (r.b + c) = r.b := by grind
      have h' : -c + 1 * (r.a + c) = r.a := by grind
      rw [this, h']; grind

theorem mulPos_spec (c : Rat) (hc : 0 < c) {n : Num} {x : K} (hx : Denotes M n x) :
    Denotes M (n.mulPos c) (M.q c * x) := by
  have hc' : 0 < M.q c := M.q_pos.mpr hc
  have hc0 : c ≠ 0 := by grind
  have hinv : M.q (1 / c) * M.q c = 1 := by rw [← M.q_mul, Rat.div_mul_cancel hc0]; simp
  cases n with
  | rat d => simp only [Denotes, Num.mulPos] at hx ⊢; rw [hx, M.q_mul]
  | iso r =>
    obtain ⟨h1, h2, h3, h4, h5⟩ := hx
    have e : ∀ y : K, M.ev (pcomp r.p [0, 1 / c]) y = M.ev r.p (M.q (1 / c) * y) := fun y => by
      rw [ev_comp_lin, M.q_zero]; congr 1; grind
    have e2 : M.q (1 / c) * (M.q c * x) = x := by grind
    simp only [Num.mulPos, Denotes, M.q_mul]
    refine ⟨OrderedRing.mul_lt_mul_of_pos_left h1 hc', OrderedRing.mul_lt_mul_of_pos_left h2 hc',
      by rw [e, e2]; exact h3, fun y hy1 hy2 hy3 => ?_, ?_⟩
    · rw [e] at hy3
      have hi' : 0 < M.q (1 / c) := M.q_pos.mpr (by
        rw [Rat.div_def, Rat.one_mul]; exact Rat.inv_pos.mpr hc)
      have k1 := OrderedRing.mul_lt_mul_of_pos_left hy1 hi'
      have k2 := OrderedRing.mul_lt_mul_of_pos_left hy2 hi'
      have m1 : M.q (1 / c) * (M.q c * M.q r.a) = M.q r.a := by grind
      have m2 : M.q (1 / c) * (M.q c * M.q r.b) = M.q r.b := by grind
      rw [m1] at k1; rw [m2] at k2
      have := h4 _ k1 k2 hy3
      grind
    · rw [evq_comp_lin, evq_comp_lin]
      have : 0 + 1 / c * (c * r.b) = r.b := by
        rw [Rat.zero_add, ← Rat.mul_assoc, Rat.div_mul_cancel hc0, Rat.one_mul]
      have h' : 0 + 1 / c * (c * r.a) = r.a := by
        rw [Rat.zero_add, ← Rat.mul_assoc, Rat.div_mul_cancel hc0, Rat.one_mul]
      rw [this, h']; exact h5

end Arith

end ForgeExact.Alg
