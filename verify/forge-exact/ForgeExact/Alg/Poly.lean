/-!
# Polynomials as coefficient lists, and their values in a model of the reals

Step 1 of stage 2 of the exact reference (research/kernel-audit/README.md §6.2): exact real
algebraic numbers. This file has the polynomials.

A polynomial is the list of its coefficients from the constant term up. The operations are
generic over the coefficients, so that one set of lemmas covers polynomials over ℚ and
polynomials over ℚ[y] (the bivariate polynomials that certify sums and products of algebraic
numbers, ForgeExact/Alg/Real.lean).

**Why core Lean and not Mathlib.** Mathlib's `Polynomial` and `Real` are noncomputable, so the
executable reference could not run them; this package compiles its reference from the same
definitions it proves things about. The proofs here need only an ordered field and evaluation, so
they are stated for any `Model`: an ordered field `K` (Lean core's `Lean.Grind` classes) with ℚ
embedded and the intermediate value property for polynomials with rational coefficients. The
real numbers with `Rat.cast` are one (any real closed field is). No axiom is assumed: the model is
a hypothesis of every theorem.

Every algorithm in Real.lean is a **certificate check**: an untrusted computation (a gcd, a
minimal polynomial, a bisection) proposes, and a checked identity between coefficient lists
disposes. So the theorems are about evaluation alone (`eval_padd`, `eval_pmul`, ...) and never
about the algorithms that propose.
-/

namespace ForgeExact.Alg

open Lean.Grind

universe u v

/-! ## Coefficient-list operations -/

section Ops

variable {α : Type u}

/-- Sum. -/
def padd [Add α] : List α → List α → List α
  | [], q => q
  | p@(_ :: _), [] => p
  | a :: p, b :: q => (a + b) :: padd p q

/-- Every coefficient times `c`. -/
def pscale [Mul α] (c : α) (p : List α) : List α := p.map (c * ·)

/-- Negation. -/
def pneg [Neg α] (p : List α) : List α := p.map (- ·)

/-- Difference. -/
def psub [Add α] [Neg α] (p q : List α) : List α := padd p (pneg q)

/-- Product. -/
def pmul [Add α] [Mul α] [Zero α] : List α → List α → List α
  | [], _ => []
  | a :: p, q => padd (pscale a q) ((0 : α) :: pmul p q)

/-- `x^k · p`. -/
def pshiftUp [Zero α] (k : Nat) (p : List α) : List α := List.replicate k (0 : α) ++ p

/-- `p^k`. -/
def ppow [Add α] [Mul α] [Zero α] [One α] (p : List α) : Nat → List α
  | 0 => [1]
  | k + 1 => pmul p (ppow p k)

/-- `p (q x)`, by Horner's rule. -/
def pcomp [Add α] [Mul α] [Zero α] (p q : List α) : List α :=
  p.foldr (fun c acc => padd [c] (pmul q acc)) []

end Ops

/-! ## Values -/

/-- A ring map from the coefficients into `K`, with the laws the lemmas use. -/
structure Hom (α : Type u) (K : Type v) [Add α] [Mul α] [Neg α] [Zero α] [One α] [Field K] where
  f : α → K
  map_add : ∀ a b, f (a + b) = f a + f b
  map_mul : ∀ a b, f (a * b) = f a * f b
  map_neg : ∀ a, f (-a) = - f a
  map_zero : f 0 = 0
  map_one : f 1 = 1

section Eval

variable {α : Type u} {K : Type v} [Add α] [Mul α] [Neg α] [Zero α] [One α] [Field K]

/-- The value at `x`, coefficients mapped by `h`: Horner's rule. -/
def evalWith (h : Hom α K) : List α → K → K
  | [], _ => 0
  | c :: p, x => h.f c + x * evalWith h p x

variable (h : Hom α K)

@[simp] theorem evalWith_nil (x : K) : evalWith h [] x = 0 := rfl
@[simp] theorem evalWith_cons (c : α) (p : List α) (x : K) :
    evalWith h (c :: p) x = h.f c + x * evalWith h p x := rfl

theorem eval_padd : ∀ (p q : List α) (x : K),
    evalWith h (padd p q) x = evalWith h p x + evalWith h q x
  | [], q, x => by simp [padd] <;> grind
  | _ :: _, [], x => by simp [padd] <;> grind
  | a :: p, b :: q, x => by
    simp only [padd, evalWith_cons, h.map_add, eval_padd p q x]; grind

theorem eval_pscale (c : α) : ∀ (p : List α) (x : K),
    evalWith h (pscale c p) x = h.f c * evalWith h p x
  | [], x => by simp [pscale] <;> grind
  | a :: p, x => by
    have := eval_pscale c p x
    simp only [pscale, List.map_cons, evalWith_cons, h.map_mul] at this ⊢
    rw [this]; grind

theorem eval_pneg : ∀ (p : List α) (x : K), evalWith h (pneg p) x = - evalWith h p x
  | [], x => by simp [pneg] <;> grind
  | a :: p, x => by
    have := eval_pneg p x
    simp only [pneg, List.map_cons, evalWith_cons, h.map_neg] at this ⊢
    rw [this]; grind

theorem eval_psub (p q : List α) (x : K) :
    evalWith h (psub p q) x = evalWith h p x - evalWith h q x := by
  simp only [psub, eval_padd, eval_pneg]; grind

theorem eval_pmul : ∀ (p q : List α) (x : K),
    evalWith h (pmul p q) x = evalWith h p x * evalWith h q x
  | [], q, x => by simp [pmul] <;> grind
  | a :: p, q, x => by
    simp only [pmul, eval_padd, eval_pscale, evalWith_cons, h.map_zero, eval_pmul p q x]; grind

theorem eval_pshiftUp : ∀ (k : Nat) (p : List α) (x : K),
    evalWith h (pshiftUp k p) x = x ^ k * evalWith h p x
  | 0, p, x => by simp [pshiftUp] <;> grind
  | k + 1, p, x => by
    have := eval_pshiftUp k p x
    simp only [pshiftUp, List.replicate_succ, List.cons_append, evalWith_cons, h.map_zero] at this ⊢
    rw [this]; grind

theorem eval_ppow (p : List α) (x : K) : ∀ k, evalWith h (ppow p k) x = evalWith h p x ^ k
  | 0 => by simp [ppow, h.map_one] <;> grind
  | k + 1 => by simp only [ppow, eval_pmul, eval_ppow p x k]; grind

theorem eval_pcomp (q : List α) (x : K) : ∀ p : List α,
    evalWith h (pcomp p q) x = evalWith h p (evalWith h q x)
  | [] => by simp [pcomp] <;> grind
  | c :: p => by
    have ih := eval_pcomp q x p
    simp only [pcomp, List.foldr_cons] at ih ⊢
    rw [eval_padd, eval_pmul, ih]; simp only [evalWith_cons, evalWith_nil]; grind

/-- A list of zero coefficients (each mapped to zero) is the zero polynomial. -/
theorem eval_of_all_zero : ∀ (p : List α) (x : K), (∀ c ∈ p, h.f c = 0) → evalWith h p x = 0
  | [], _, _ => rfl
  | c :: p, x, hz => by
    simp only [evalWith_cons, hz c (List.mem_cons_self ..),
      eval_of_all_zero p x fun d hd => hz d (List.mem_cons_of_mem _ hd)]; grind

/-- Appending a coefficient that maps to zero changes nothing. -/
theorem eval_append_zero : ∀ (p : List α) (z : α) (x : K), h.f z = 0 →
    evalWith h (p ++ [z]) x = evalWith h p x
  | [], z, x, hz => by simp [hz] <;> grind
  | c :: p, z, x, hz => by
    simp only [List.cons_append, evalWith_cons, eval_append_zero p z x hz]

end Eval

/-! ## Polynomials over ℚ -/

/-- A polynomial with rational coefficients. -/
abbrev Poly := List Rat

namespace Poly

/-- Its value at a rational point (the executable's own evaluation). -/
def ev (p : Poly) (x : Rat) : Rat := p.foldr (fun c acc => c + x * acc) 0

/-- Trailing zero coefficients dropped. -/
def trim (p : Poly) : Poly := (p.reverse.dropWhile (· == 0)).reverse

/-- Every coefficient zero: the zero polynomial. -/
def isZero (p : Poly) : Bool := p.all (· == 0)

/-- The two lists are one polynomial. -/
def eqv (p q : Poly) : Bool := isZero (psub p q)

def deg (p : Poly) : Nat := (trim p).length - 1

/-- The leading coefficient (0 for the zero polynomial). -/
def lead (p : Poly) : Rat := (trim p).getLast?.getD 0

def X : Poly := [0, 1]
def C (c : Rat) : Poly := [c]

/-- `p (x + c)`. -/
def shift (p : Poly) (c : Rat) : Poly := pcomp p [c, 1]

/-- `p (c x)`. -/
def scaleArg (p : Poly) (c : Rat) : Poly := pcomp p [0, c]

/-- Long division by `d` (any nonzero `d`): quotient and remainder, `p = q d + r`. Untrusted:
the results are checked with `eqv` wherever they matter. -/
def divmod (p d : Poly) : Poly × Poly := Id.run do
  let d := trim d
  if d.isEmpty then return ([], p)
  let m := d.length - 1
  let ld := d.getLast!
  let mut r := trim p
  let mut q : Poly := List.replicate (r.length - m) 0
  for _ in [0:p.length + 1] do
    if r.length < m + 1 then break
    let k := r.length - 1
    let c := r.getLast! / ld
    q := q.set (k - m) c
    r := trim ((psub r (pscale c (pshiftUp (k - m) d))).dropLast)
  return (trim q, trim r)

def pmod (p d : Poly) : Poly := (divmod p d).2
def pdiv (p d : Poly) : Poly := (divmod p d).1

/-- Monic: the leading coefficient made 1. -/
def monic (p : Poly) : Poly := let l := lead p; if l == 0 then p else pscale (1 / l) (trim p)

/-- Formal derivative. -/
def deriv (p : Poly) : Poly := (p.zipIdx.map fun (c, i) => c * i).drop 1

/-- The extended gcd: `(g, u, v)` with `g = u p + v q`, `g` monic. Untrusted. -/
def xgcd (p q : Poly) : Poly × Poly × Poly := Id.run do
  let mut r0 := trim p
  let mut r1 := trim q
  let mut u0 : Poly := [1]
  let mut v0 : Poly := []
  let mut u1 : Poly := []
  let mut v1 : Poly := [1]
  for _ in [0:p.length + q.length + 2] do
    if isZero r1 then break
    let (qq, rr) := divmod r0 r1
    let u2 := trim (psub u0 (pmul qq u1))
    let v2 := trim (psub v0 (pmul qq v1))
    r0 := r1; r1 := rr; u0 := u1; v0 := v1; u1 := u2; v1 := v2
  let l := lead r0
  if l == 0 then return (r0, u0, v0)
  return (pscale (1 / l) r0, pscale (1 / l) u0, pscale (1 / l) v0)

def gcd (p q : Poly) : Poly := (xgcd p q).1

/-- The square-free part. Untrusted (used only to propose). -/
def sqfree (p : Poly) : Poly :=
  let g := gcd p (deriv p)
  if deg g == 0 then monic p else monic (pdiv p g)

end Poly

end ForgeExact.Alg
