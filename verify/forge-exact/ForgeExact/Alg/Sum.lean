import ForgeExact.Alg.Real

/-!
# Sums and products of two algebraic numbers

For isolated roots `α` of `p` and `β` of `q`, `α + β` (and `α β`) is a root of the minimal
polynomial `R` of `x + y` (of `x y`) in `ℚ[x, y]/(p(x), q(y))`, found by linear algebra on the
powers of `x + y` (untrusted). The certificate is a reduction, checked coefficient by coefficient:

  `R(x + y) = Q(x, y) · p(x) + Rm(x, y)` with every coefficient of `Rm` (a polynomial in `y`) a
  multiple of `q(y)`.

At `(α, β)` the right side vanishes, so `R(α + β) = 0` (`sumCert_spec`). `α + β` lies in the sum
of the two intervals; bisecting both until `R` has one root there (`Iso.ok`) isolates it
(`Num.add`, `add_spec`). Products likewise, for positive numbers (`Num.mulPosPos`, `mul_spec`).

Bivariate polynomials are polynomials in `x` whose coefficients are polynomials in `y` (`YPoly`),
so the evaluation lemmas of Poly.lean serve both levels.
-/

namespace ForgeExact.Alg

open Lean.Grind

set_option linter.unusedSectionVars false

/-- A polynomial in `y`, as a coefficient of a polynomial in `x`. -/
structure YPoly where
  c : Poly
  deriving Inhabited

instance : Add YPoly := ⟨fun a b => ⟨padd a.c b.c⟩⟩
instance : Mul YPoly := ⟨fun a b => ⟨pmul a.c b.c⟩⟩
instance : Neg YPoly := ⟨fun a => ⟨pneg a.c⟩⟩
instance : Zero YPoly := ⟨⟨[]⟩⟩
instance : One YPoly := ⟨⟨[1]⟩⟩

/-- A polynomial in `x` and `y`. -/
abbrev BPoly := List YPoly

/-- A rational polynomial in `x`, as a bivariate one. -/
def liftX (p : Poly) : BPoly := p.map fun c => ⟨[c]⟩

/-- A rational polynomial in `y`, as a bivariate constant. -/
def liftY (q : Poly) : BPoly := [⟨q⟩]

/-- Every coefficient of every coefficient is zero. -/
def bzero (s : BPoly) : Bool := s.all fun c => Poly.isZero c.c

section Hom

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- Evaluation of the `y`-coefficients at `y`. -/
def yHom (y : K) : Hom YPoly K where
  f r := M.ev r.c y
  map_add a b := eval_padd M.ι a.c b.c y
  map_mul a b := eval_pmul M.ι a.c b.c y
  map_neg a := eval_pneg M.ι a.c y
  map_zero := rfl
  map_one := by show M.ι.f 1 + y * 0 = 1; rw [M.ι.map_one]; grind

theorem ev_liftX (p : Poly) (x y : K) : evalWith (yHom M y) (liftX p) x = M.ev p x := by
  induction p with
  | nil => rfl
  | cons c p ih =>
    simp only [liftX, List.map_cons, evalWith_cons] at ih ⊢
    rw [ih]; simp only [yHom, Model.ev, evalWith_cons, evalWith_nil]; grind

theorem ev_liftY (q : Poly) (x y : K) : evalWith (yHom M y) (liftY q) x = M.ev q y := by
  simp only [liftY, evalWith_cons, evalWith_nil, yHom]; grind

theorem ev_bzero (s : BPoly) (h : bzero s = true) (x y : K) : evalWith (yHom M y) s x = 0 := by
  apply eval_of_all_zero
  intro c hc
  simp only [bzero, List.all_eq_true] at h
  have := h c hc
  simp only [Poly.isZero, List.all_eq_true, beq_iff_eq] at this
  exact eval_of_all_zero M.ι c.c y fun d hd => by rw [this d hd]; exact M.ι.map_zero

end Hom

/-! ## The certificate -/

/-- `x + y` and `x y`. -/
def sumXY : BPoly := [⟨[0, 1]⟩, ⟨[1]⟩]
def prodXY : BPoly := [⟨[]⟩, ⟨[0, 1]⟩]

/-- Untrusted long division of a bivariate `s` by a monic `d(x)`: quotient and remainder. -/
def bdivMonic (s : BPoly) (d : Poly) : BPoly × BPoly := Id.run do
  let m := d.length - 1
  let mut r : Array YPoly := s.toArray
  let mut q : Array YPoly := Array.replicate (s.length + 1 - m) ⟨[]⟩
  let mut k := r.size
  while k > m do
    k := k - 1
    let c := r[k]!
    if !Poly.isZero c.c then
      q := q.set! (k - m) c
      for t in [0:m + 1] do
        r := r.set! (k - m + t) ⟨Poly.trim (psub r[k - m + t]!.c (pmul c.c [d[t]!]))⟩
  return (q.toList, (r.toList.take m))

/-- The reduction certificate: `R(f(x, y)) = Q p(x) + Rm`, every coefficient of `Rm` a multiple
of `q(y)`. `p` must be monic. -/
def redCert (f : BPoly) (R p q : Poly) : Bool :=
  let s := pcomp (liftX R) f
  let (Q, Rm) := bdivMonic s p
  bzero (psub s (padd (pmul Q (liftX p)) Rm)) &&
    Rm.all fun c => Poly.eqv c.c (pmul (Poly.pdiv c.c q) q)

section Cert

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem ev_redCert {f : BPoly} {R p q : Poly} (h : redCert f R p q = true) {α β : K}
    (hα : M.ev p α = 0) (hβ : M.ev q β = 0) : M.ev R (evalWith (yHom M β) f α) = 0 := by
  simp only [redCert, Bool.and_eq_true, List.all_eq_true] at h
  obtain ⟨h1, h2⟩ := h
  generalize hs : pcomp (liftX R) f = s at h1 h2
  generalize hd : bdivMonic s p = QR at h1 h2
  obtain ⟨Q, Rm⟩ := QR
  have e1 := ev_bzero M _ h1 α β
  rw [eval_psub, eval_padd, eval_pmul, ev_liftX, hα] at e1
  have e2 : evalWith (yHom M β) Rm α = 0 := by
    apply eval_of_all_zero
    intro c hc
    have := eqv_spec M (h2 c hc) β
    simp only [yHom]; rw [this, Model.ev, eval_pmul]; simp only [Model.ev] at hβ; rw [hβ]; grind
  have e3 : evalWith (yHom M β) s α = 0 := by grind
  rw [← hs, eval_pcomp, ev_liftX] at e3
  exact e3

theorem ev_sumXY (α β : K) : evalWith (yHom M β) sumXY α = α + β := by
  simp only [sumXY, evalWith_cons, evalWith_nil, yHom, Model.ev, M.ι.map_zero, M.ι.map_one]; grind

theorem ev_prodXY (α β : K) : evalWith (yHom M β) prodXY α = α * β := by
  simp only [prodXY, evalWith_cons, evalWith_nil, yHom, Model.ev, M.ι.map_zero, M.ι.map_one]; grind

end Cert

/-! ## The minimal polynomial, by linear algebra (untrusted) -/

/-- An element of `ℚ[x, y]/(p, q)`, coefficients of `x^i y^j` at `i n + j`. -/
abbrev Elt := Array Rat

/-- `e` times `x`, reduced by the monic `p` of degree `m`. -/
def mulX (m n : Nat) (p : Poly) (e : Elt) : Elt := Id.run do
  let mut out : Elt := Array.replicate (m * n) 0
  for i in [0:m] do
    for j in [0:n] do
      let c := e[i * n + j]!
      if c == 0 then continue
      if i + 1 < m then out := out.set! ((i + 1) * n + j) (out[(i + 1) * n + j]! + c)
      else
        for t in [0:m] do
          out := out.set! (t * n + j) (out[t * n + j]! - c * p[t]!)
  return out

/-- `e` times `y`, reduced by the monic `q` of degree `n`. -/
def mulY (m n : Nat) (q : Poly) (e : Elt) : Elt := Id.run do
  let mut out : Elt := Array.replicate (m * n) 0
  for i in [0:m] do
    for j in [0:n] do
      let c := e[i * n + j]!
      if c == 0 then continue
      if j + 1 < n then out := out.set! (i * n + j + 1) (out[i * n + j + 1]! + c)
      else
        for t in [0:n] do
          out := out.set! (i * n + t) (out[i * n + t]! - c * q[t]!)
  return out

/-- The first linear relation among `1, e, e², ...`: the minimal polynomial of the
multiplication map `step` from `1`. -/
def minPoly (dim : Nat) (one : Elt) (step : Elt → Elt) : Poly := Id.run do
  -- an echelon basis: (vector, pivot, its expression in the powers)
  let mut basis : Array (Elt × Nat × Array Rat) := #[]
  let mut v := one
  for k in [0:dim + 1] do
    -- reduce v by the basis, tracking the expression
    let mut w := v
    let mut ex : Array Rat := (Array.replicate (k + 1) 0).set! k 1
    for (b, piv, bex) in basis do
      let c := w[piv]!
      if c != 0 then
        w := (Array.range w.size).map fun t => w[t]! - c * b[t]!
        ex := (Array.range ex.size).map fun t => ex[t]! - c * bex.getD t 0
    match (Array.range w.size).find? (fun t => w[t]! != 0) with
    | none => return Poly.trim ex.toList
    | some piv =>
      let c := w[piv]!
      basis := basis.push (w.map (· / c), piv, ex.map (· / c))
    v := step v
  return []

/-- The minimal polynomial of `x + y` (`prod`: of `x y`) mod monic `p`, `q`. -/
def minPolyOf (prod : Bool) (p q : Poly) : Poly :=
  let m := p.length - 1
  let n := q.length - 1
  let one : Elt := (Array.replicate (m * n) 0).set! 0 1
  let step : Elt → Elt := fun e =>
    if prod then mulY m n q (mulX m n p e)
    else (Array.range (m * n)).map fun t => (mulX m n p e)[t]! + (mulY m n q e)[t]!
  minPoly (m * n) one step

/-! ## Sum and product -/

/-- Trailing zeros dropped (a form the proofs follow). -/
def trimR : Poly → Poly
  | [] => []
  | c :: p => let t := trimR p; if t.isEmpty && c == 0 then [] else c :: t

/-- Made monic. -/
def monicR (p : Poly) : Poly :=
  let t := trimR p
  match t.getLast? with
  | some l => if l == 0 then t else pscale (1 / l) t
  | none => t

/-- Isolate the root of `R` that `u + v` is (`prod`: `u v`, both positive), bisecting `u` and
`v` until `R` has one root in the sum (product) of their intervals. -/
def isoLoop (prod : Bool) (R : Poly) : Nat → Num → Num → Option Num
  | 0, _, _ => none
  | _ + 1, .rat c, w => if prod then (if 0 < c then some (w.mulPos c) else none) else some (w.addRat c)
  | _ + 1, w, .rat c => if prod then (if 0 < c then some (w.mulPos c) else none) else some (w.addRat c)
  | k + 1, .iso r, .iso s =>
    let t : Iso := if prod then ⟨R, r.a * s.a, r.b * s.b⟩ else ⟨R, r.a + s.a, r.b + s.b⟩
    if (!prod || (decide (0 ≤ r.a) && decide (0 ≤ s.a))) && t.ok then some (.iso t)
    else isoLoop prod R k (Num.iso r).bisect (Num.iso s).bisect

/-- `α + β` (`prod`: `α β`, for positive numbers): the minimal polynomial, its certificate, then
isolation. `none` when a check fails or the fuel runs out. -/
def Iso.combine (prod : Bool) (fuel : Nat) (r s : Iso) : Option Num :=
  let p := monicR r.p
  let q := monicR s.p
  let R := minPolyOf prod p q
  if R.isEmpty || !redCert (if prod then prodXY else sumXY) R p q then none
  else isoLoop prod R fuel (.iso r) (.iso s)

/-- **`α + β`.** -/
def Num.add (fuel : Nat) : Num → Num → Option Num
  | .rat c, m => some (m.addRat c)
  | n, .rat c => some (n.addRat c)
  | .iso r, .iso s => r.combine false fuel s

/-- `α β` for positive `α`, `β`. -/
def Num.mulPP (fuel : Nat) : Num → Num → Option Num
  | .rat c, m => if 0 < c then some (m.mulPos c) else none
  | n, .rat c => if 0 < c then some (n.mulPos c) else none
  | .iso r, .iso s => r.combine true fuel s

/-- **`α β`**, by signs. -/
def Num.mul (fuel : Nat) (n m : Num) : Option Num :=
  match n.cmpRat 0, m.cmpRat 0 with
  | .eq, _ => some (.rat 0)
  | _, .eq => some (.rat 0)
  | .gt, .gt => n.mulPP fuel m
  | .lt, .lt => n.neg.mulPP fuel m.neg
  | .gt, .lt => (n.mulPP fuel m.neg).map Num.neg
  | .lt, .gt => (n.neg.mulPP fuel m).map Num.neg

section Spec

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

theorem ev_trimR : ∀ (p : Poly) (x : K), M.ev (trimR p) x = M.ev p x
  | [], _ => rfl
  | c :: p, x => by
    have ih := ev_trimR p x
    simp only [trimR]
    split
    · next h =>
      simp only [Bool.and_eq_true, List.isEmpty_iff, beq_iff_eq] at h
      rw [h.1] at ih
      simp only [Model.ev, evalWith_cons, evalWith_nil] at ih ⊢
      rw [h.2, M.ι.map_zero, ← ih]; grind
    · simp only [Model.ev, evalWith_cons] at ih ⊢; rw [ih]

theorem ev_monicR_zero (p : Poly) {x : K} (h : M.ev p x = 0) : M.ev (monicR p) x = 0 := by
  simp only [monicR]
  split
  · split
    · rw [ev_trimR]; exact h
    · simp only [Model.ev]; rw [eval_pscale]; have := ev_trimR M p x; simp only [Model.ev] at this
      rw [this]; simp only [Model.ev] at h; rw [h]; grind
  · rw [ev_trimR]; exact h

/-- An isolated root denotes a root of its polynomial. -/
theorem Denotes.ev_zero {r : Iso} {x : K} (h : Denotes M (.iso r) x) : M.ev r.p x = 0 := h.2.2.1

theorem isoLoop_spec (prod : Bool) (R : Poly) :
    ∀ (fuel : Nat) {u v z : Num} {x y : K}, Denotes M u x → Denotes M v y →
    (prod = true → 0 < x ∧ 0 < y) →
    M.ev R (if prod then x * y else x + y) = 0 →
    isoLoop prod R fuel u v = some z → Denotes M z (if prod then x * y else x + y)
  | 0, _, _, _, _, _, _, _, _, _, h => by simp [isoLoop] at h
  | k + 1, .rat c, w, z, x, y, hx, hy, hpos, hR, h => by
    simp only [Denotes] at hx; subst hx
    simp only [isoLoop] at h
    cases prod
    · simp only [Option.some.injEq, Bool.false_eq_true, ite_false] at h ⊢; subst h
      have := addRat_spec M c hy; rw [show M.q c + y = y + M.q c by grind]; exact this
    · simp only [ite_true] at h ⊢
      split at h
      · next hc => simp only [Option.some.injEq] at h; subst h
                   have := mulPos_spec M c hc hy; exact this
      · simp at h
  | k + 1, .iso r, .rat c, z, x, y, hx, hy, hpos, hR, h => by
    simp only [Denotes] at hy; subst hy
    simp only [isoLoop] at h
    cases prod
    · simp only [Option.some.injEq, Bool.false_eq_true, ite_false] at h ⊢; subst h
      exact addRat_spec M c hx
    · simp only [ite_true] at h ⊢
      split at h
      · next hc => simp only [Option.some.injEq] at h; subst h
                   have := mulPos_spec M c hc hx; rw [show x * M.q c = M.q c * x by grind]; exact this
      · simp at h
  | k + 1, .iso r, .iso s, z, x, y, hx, hy, hpos, hR, h => by
    obtain ⟨hx1, hx2, _, _, _⟩ := id hx
    obtain ⟨hy1, hy2, _, _, _⟩ := id hy
    cases prod with
    | false =>
      simp only [isoLoop, Bool.false_eq_true, ite_false, Bool.not_false, Bool.true_or,
        Bool.true_and] at h hR ⊢
      split at h
      · next hok =>
        simp only [Option.some.injEq] at h; subst h
        obtain ⟨w, hw⟩ := Iso.denotes_of_ok M _ hok
        have := hw.2.2.2.1 (x + y) (by show M.q (r.a + s.a) < _; rw [M.q_add]; grind)
          (by show _ < M.q (r.b + s.b); rw [M.q_add]; grind) hR
        rw [this]; exact hw
      · exact isoLoop_spec false R k (bisect_spec M hx) (bisect_spec M hy) hpos (by simpa using hR)
          (by simpa using h)
    | true =>
      simp only [isoLoop, ite_true, Bool.not_true, Bool.false_or] at h hR ⊢
      split at h
      · next hok =>
        simp only [Option.some.injEq] at h; subst h
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hok
        obtain ⟨⟨ha, hb⟩, hok⟩ := hok
        obtain ⟨w, hw⟩ := Iso.denotes_of_ok M _ hok
        obtain ⟨hx0, hy0⟩ := hpos rfl
        have ha' := M.q_nonneg.mpr ha
        have hb' := M.q_nonneg.mpr hb
        have lo : M.q (r.a * s.a) < x * y := by
          rw [M.q_mul]
          have h1 : M.q r.a * M.q s.a ≤ M.q r.a * y :=
            OrderedRing.mul_le_mul_of_nonneg_left (Preorder.le_of_lt hy1) ha'
          have h2 : M.q r.a * y < x * y := OrderedRing.mul_lt_mul_of_pos_right hx1 hy0
          grind
        have hi : x * y < M.q (r.b * s.b) := by
          rw [M.q_mul]
          have h1 : x * y < M.q r.b * y := OrderedRing.mul_lt_mul_of_pos_right hx2 hy0
          have h2 : M.q r.b * y < M.q r.b * M.q s.b :=
            OrderedRing.mul_lt_mul_of_pos_left hy2 (by grind)
          grind
        have := hw.2.2.2.1 (x * y) lo hi hR
        rw [this]; exact hw
      · exact isoLoop_spec true R k (bisect_spec M hx) (bisect_spec M hy) hpos (by simpa using hR)
          (by simpa using h)

theorem combine_spec (prod : Bool) (fuel : Nat) {r s : Iso} {z : Num} {x y : K}
    (hx : Denotes M (.iso r) x) (hy : Denotes M (.iso s) y) (hpos : prod = true → 0 < x ∧ 0 < y)
    (h : r.combine prod fuel s = some z) : Denotes M z (if prod then x * y else x + y) := by
  have hp := ev_monicR_zero M _ hx.ev_zero
  have hq := ev_monicR_zero M _ hy.ev_zero
  cases prod with
  | false =>
    unfold Iso.combine at h
    simp only [Bool.false_eq_true, ite_false] at h ⊢
    split at h
    · simp at h
    · next hc =>
      simp only [Bool.or_eq_true, Bool.not_eq_true', _root_.not_or, Bool.not_eq_false] at hc
      have hR := ev_redCert M hc.2 hp hq
      rw [ev_sumXY] at hR
      have := isoLoop_spec M false _ fuel hx hy hpos (by simpa using hR) h
      simpa using this
  | true =>
    unfold Iso.combine at h
    simp only [ite_true] at h ⊢
    split at h
    · simp at h
    · next hc =>
      simp only [Bool.or_eq_true, Bool.not_eq_true', _root_.not_or, Bool.not_eq_false] at hc
      have hR := ev_redCert M hc.2 hp hq
      rw [ev_prodXY] at hR
      have := isoLoop_spec M true _ fuel hx hy hpos (by simpa using hR) h
      simpa using this

/-- **The sum is exact.** -/
theorem add_spec (fuel : Nat) {n m z : Num} {x y : K} (hx : Denotes M n x) (hy : Denotes M m y)
    (h : Num.add fuel n m = some z) : Denotes M z (x + y) := by
  cases n with
  | rat c =>
    simp only [Num.add, Option.some.injEq] at h; subst h
    simp only [Denotes] at hx; subst hx
    have := addRat_spec M c hy; rw [show M.q c + y = y + M.q c by grind]; exact this
  | iso r =>
    cases m with
    | rat c =>
      simp only [Num.add, Option.some.injEq] at h; subst h
      simp only [Denotes] at hy; subst hy; exact addRat_spec M c hx
    | iso s =>
      have := combine_spec M false fuel hx hy (by simp) h
      simpa using this

theorem mulPP_spec (fuel : Nat) {n m z : Num} {x y : K} (hx : Denotes M n x) (hy : Denotes M m y)
    (hx0 : 0 < x) (hy0 : 0 < y) (h : Num.mulPP fuel n m = some z) : Denotes M z (x * y) := by
  cases n with
  | rat c =>
    simp only [Num.mulPP] at h
    split at h
    · next hc =>
      simp only [Option.some.injEq] at h; subst h
      simp only [Denotes] at hx; subst hx; exact mulPos_spec M c hc hy
    · simp at h
  | iso r =>
    cases m with
    | rat c =>
      simp only [Num.mulPP] at h
      split at h
      · next hc =>
        simp only [Option.some.injEq] at h; subst h
        simp only [Denotes] at hy; subst hy
        have := mulPos_spec M c hc hx; rw [show x * M.q c = M.q c * x by grind]; exact this
      · simp at h
    | iso s =>
      have := combine_spec M true fuel hx hy (fun _ => ⟨hx0, hy0⟩) h
      simpa using this

/-- **The product is exact.** -/
theorem mul_spec (fuel : Nat) {n m z : Num} {x y : K} (hx : Denotes M n x) (hy : Denotes M m y)
    (h : Num.mul fuel n m = some z) : Denotes M z (x * y) := by
  have cx := cmpRat_spec M n 0 hx
  have cy := cmpRat_spec M m 0 hy
  rw [M.q_zero] at cx cy
  simp only [Num.mul] at h
  generalize n.cmpRat 0 = on at h cx
  generalize m.cmpRat 0 = om at h cy
  cases on <;> cases om <;> simp only [CmpIs] at cx cy <;> simp only at h
  all_goals first
    | (simp only [Option.some.injEq] at h; subst h; simp only [Denotes, M.q_zero]; rw [cx]; grind)
    | (simp only [Option.some.injEq] at h; subst h; simp only [Denotes, M.q_zero]; rw [cy]; grind)
    | exact mulPP_spec M fuel hx hy cx cy h
    | (have := mulPP_spec M fuel (neg_spec M hx) (neg_spec M hy) (by grind) (by grind) h
       rw [show -x * -y = x * y by grind] at this; exact this)
    | (obtain ⟨w, hw, rfl⟩ := Option.map_eq_some_iff.mp h
       have := neg_spec M (mulPP_spec M fuel (neg_spec M hx) hy (by grind) cy hw)
       rw [show -(-x * y) = x * y by grind] at this; exact this)
    | (obtain ⟨w, hw, rfl⟩ := Option.map_eq_some_iff.mp h
       have := neg_spec M (mulPP_spec M fuel hx (neg_spec M hy) cx (by grind) hw)
       rw [show -(x * -y) = x * y by grind] at this; exact this)

end Spec

end ForgeExact.Alg
