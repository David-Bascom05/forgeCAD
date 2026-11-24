/-
# The splines of .fg: the definitions

One meaning for every spline `spline(..)` makes. Core Lean only, generic over the number type `K`, so
that the same definitions are

* the specification, over ℝ (or any linear ordered field), in the theorems of `SplineSpec.Basis`,
  `SplineSpec.Interp` and `SplineSpec.Bound`; and
* the executable model, over `Rat`, that `lake exe splineoracle` runs against the Rust.

Conventions: a knot vector is `u : Nat → K`, read at `0 .. n + p` (n poles, degree p); poles and
weights are `Nat → K` per coordinate. A curve of degree p on n poles runs over `[u p, u n]`.
-/

namespace SplineSpec

section Generic

variable {K : Type} [Add K] [Sub K] [Mul K] [Div K] [OfNat K 0] [OfNat K 1] [DecidableEq K]
  [LT K] [LE K] [DecidableRel (α := K) (· < ·)] [DecidableRel (α := K) (· ≤ ·)]

/-! ## B-splines by Cox–de Boor, one span at a time

`nb u j i k t` is the basis function `N_(i,k)` as the polynomial it is on the span `[u j, u (j+1))`:
the recurrence of Cox and de Boor started from `N_(i,0) = [i = j]`. On that span it equals the usual
half-open `N_(i,k)`; at the right end of the curve it gives the left limit, so the curve is closed at
both ends with no special case. A term whose knot difference is zero is zero (the 0/0 = 0 rule). -/

/-- The blending ratio `(t - u i) / (u (i+k) - u i)`, zero where the two knots are one. -/
def omega (u : Nat → K) (i k : Nat) (t : K) : K :=
  if u (i + k) = u i then 0 else (t - u i) / (u (i + k) - u i)

/-- `N_(i,k)` on the span `j`. -/
def nb (u : Nat → K) (j : Nat) : Nat → Nat → K → K
  | i, 0, _ => if i = j then 1 else 0
  | i, k + 1, t => omega u i (k + 1) t * nb u j i k t + (1 - omega u (i + 1) (k + 1) t) * nb u j (i + 1) k t

/-- `Σ_(i < n) f i`. -/
def sumTo (n : Nat) (f : Nat → K) : K :=
  match n with
  | 0 => 0
  | n + 1 => sumTo n f + f n

/-- The span the curve's parameter `t` is in: the last `j` in `[p, n)` with `u j ≤ t` and
`u j < u (j+1)`, else `p`'s first nonempty span. `t` is taken in `[u p, u n]`. -/
def spanOf (u : Nat → K) (p n : Nat) (t : K) : Nat :=
  go p p
where
  /-- the answer so far `best`, looking at `j`. -/
  go (j best : Nat) : Nat :=
    if h : j < n then
      let best' := if u j < u (j + 1) ∧ (u j ≤ t ∨ best = p ∧ ¬ (u best < u (best + 1))) then j else best
      go (j + 1) best'
    else best
  termination_by n - j

/-- The homogeneous point `Σ N_(i,p) w_i P_i` (one coordinate) on span `j`; `P` the pole's
coordinate, `w` the weights. Only `i` in `[j - p, j]` can be nonzero; the sum is over all `i < n`. -/
def homOn (u : Nat → K) (p n j : Nat) (w P : Nat → K) (t : K) : K :=
  sumTo n fun i => nb u j i p t * (w i * P i)

/-- The weight `Σ N_(i,p) w_i` on span `j`. -/
def weightOn (u : Nat → K) (p n j : Nat) (w : Nat → K) (t : K) : K :=
  sumTo n fun i => nb u j i p t * w i

/-- A rational B-spline curve, one coordinate at a time. -/
structure Nurbs (K : Type) where
  /-- degree -/
  p : Nat
  /-- number of poles -/
  n : Nat
  /-- knots `u 0 .. u (n + p)` -/
  u : Nat → K
  /-- weights, one a pole -/
  w : Nat → K
  /-- the poles' coordinates: `P c i` is coordinate `c` of pole `i` -/
  P : Nat → Nat → K

/-- The curve's coordinate `c` at `t`: `Σ N w P / Σ N w` on the span of `t`. -/
def Nurbs.eval (C : Nurbs K) (c : Nat) (t : K) : K :=
  let j := spanOf C.u C.p C.n t
  homOn C.u C.p C.n j C.w (C.P c) t / weightOn C.u C.p C.n j C.w t

/-- A uniform clamped knot vector for n poles of degree p: p + 1 zeros, 1 .. n - p - 1, then p + 1
times n - p. -/
def uniformClamped (p n : Nat) (i : Nat) : K :=
  if i ≤ p then 0 else if i < n then natK (i - p) else natK (n - p)
where
  natK : Nat → K
    | 0 => 0
    | k + 1 => natK k + 1

/-- Counts of `u i` among `u 0 .. u (m - 1)`. -/
def mult (u : Nat → K) (m : Nat) (x : K) : Nat :=
  match m with
  | 0 => 0
  | m + 1 => mult u m x + if u m = x then 1 else 0

/-- What makes a knot vector one: `n + p + 1` knots never falling, a nonempty curve `u p < u n`,
each knot strictly inside of multiplicity at most p (else the curve breaks), each at most p + 1. -/
def KnotsValid (u : Nat → K) (p n : Nat) : Prop :=
  0 < p ∧ p < n ∧ (∀ i, i < n + p → u i ≤ u (i + 1)) ∧ u p < u n ∧
  (∀ i, i ≤ n + p → u p < u i → u i < u n → mult u (n + p + 1) (u i) ≤ p) ∧
  (∀ i, i ≤ n + p → mult u (n + p + 1) (u i) ≤ p + 1)

/-- Weights all positive. -/
def WeightsValid (w : Nat → K) (n : Nat) : Prop := ∀ i, i < n → 0 < w i

/-- The closed control form (`spline(.., cv: true)` closed): poles `Q (i mod m)` for `i < m + p`,
knots `0, 1, .., m + 2p`, the curve over `[p, m + p]`. A `Nurbs` like any other. -/
def closedCV (p m : Nat) (Q : Nat → Nat → K) : Nurbs K where
  p := p
  n := m + p
  u := fun i => uniformClamped.natK i
  w := fun _ => 1
  P := fun c i => Q c (i % m)

/-! ## Through-point interpolation

The through-spline is a cubic spline: on each `[τ i, τ (i+1)]` a cubic, C2 at the inner points,
through `q i` at `τ i`. A piece is given by its four Bezier points in its own parameter
`s = (t - τ i) / h i`; `h i = τ (i+1) - τ i`. -/

/-- How an open end is held: second derivative zero, or a first derivative given (one coordinate). -/
inductive EndCond (K : Type) where
  | natural
  | slope (v : K)

/-- Which through-spline: open with its two end conditions, or periodic (`q m = q 0`). -/
inductive Ends (K : Type) where
  | open_ (start finish : EndCond K)
  | periodic

/-- Bezier points of a cubic piece: `b k`, k = 0..3. -/
abbrev Piece (K : Type) := Nat → K

/-- The Bernstein form at `s`: `Σ C(3,k) s^k (1-s)^(3-k) b k`. -/
def bez3 (b : Piece K) (s : K) : K :=
  let r := 1 - s
  r * r * r * b 0 + (1 + 1 + 1) * s * r * r * b 1 + (1 + 1 + 1) * s * s * r * b 2 + s * s * s * b 3

/-- The Hermite piece from end values and slopes (per unit of t) over a span of length `h`. -/
def hermite (a b ma mb h : K) : Piece K := fun k =>
  match k with
  | 0 => a
  | 1 => a + ma * h / (1 + 1 + 1)
  | 2 => b - mb * h / (1 + 1 + 1)
  | _ => b

/-- The cubic spline's first derivative (per unit of t) at the start of piece `B`, length `h`. -/
def d1start (B : Piece K) (h : K) : K := (1 + 1 + 1) * (B 1 - B 0) / h
/-- At its end. -/
def d1end (B : Piece K) (h : K) : K := (1 + 1 + 1) * (B 3 - B 2) / h
/-- The second derivative (per unit of t, squared) at the start, up to the factor 6. -/
def d2start (B : Piece K) (h : K) : K := (B 2 - (1 + 1) * B 1 + B 0) / (h * h)
/-- At its end, up to the factor 6. -/
def d2end (B : Piece K) (h : K) : K := (B 3 - (1 + 1) * B 2 + B 1) / (h * h)

/-- One row of the slope system: `lo * m (i-1) + di * m i + up * m (i+1) = rhs` (indices cyclic
for the periodic system). -/
structure Row (K : Type) where
  lo : K
  di : K
  up : K
  rhs : K

/-- The rows of the slope system for points `q 0 .. q m` at `τ 0 .. τ m` (one coordinate).
Open: m + 1 rows, unknowns `m 0 .. m m`. Periodic: m rows, unknowns `m 0 .. m (m-1)`, `m m = m 0`.
The middle rows are C2 at `τ i`: `h i m(i-1) + 2(h(i-1) + h i) m i + h(i-1) m(i+1)
= 3 (h i dq(i-1) + h(i-1) dq i)`, `dq i = (q(i+1) - q i) / h i`. A natural end is
`2 m0 + m1 = 3 dq0`, a slope end `m0 = v`. -/
def rows (τ q : Nat → K) (m : Nat) (e : Ends K) : Nat → Row K := fun i =>
  let h := fun i => τ (i + 1) - τ i
  let dq := fun i => (q (i + 1) - q i) / h i
  let three : K := 1 + 1 + 1
  let two : K := 1 + 1
  let mid := fun (i : Nat) (il : Nat) =>
    { lo := h i, di := two * (h il + h i), up := h il, rhs := three * (h i * dq il + h il * dq i) : Row K }
  match e with
  | .open_ s f =>
    if i = 0 then
      match s with
      | .natural => { lo := 0, di := two, up := 1, rhs := three * dq 0 }
      | .slope v => { lo := 0, di := 1, up := 0, rhs := v }
    else if i = m then
      match f with
      | .natural => { lo := 1, di := two, up := 0, rhs := three * dq (m - 1) }
      | .slope v => { lo := 0, di := 1, up := 0, rhs := v }
    else mid i (i - 1)
  | .periodic =>
    if i = 0 then mid 0 (m - 1) else mid i (i - 1)

/-- How many slopes the system has. -/
def unknowns (m : Nat) : Ends K → Nat
  | .open_ _ _ => m + 1
  | .periodic => m

/-- The slope `m i` the solution vector `x` gives, with `m m = m 0` for the periodic system. -/
def slopeAt (m : Nat) (e : Ends K) (x : Nat → K) (i : Nat) : K :=
  match e with
  | .open_ _ _ => x i
  | .periodic => if i = m then x 0 else x i

/-- The unknown left of `i` in its row, cyclically (an open system's first row has 0 there). -/
def leftIx (n i : Nat) : Nat := if i = 0 then n - 1 else i - 1
/-- The unknown right of `i` in its row, cyclically (an open system's last row has 0 there). -/
def rightIx (n i : Nat) : Nat := if i + 1 = n then 0 else i + 1

/-- Row `i` of the system applied to `x`, minus its right side. -/
def residual (τ q : Nat → K) (m : Nat) (e : Ends K) (x : Nat → K) (i : Nat) : K :=
  let r := rows τ q m e i
  let n := unknowns m e
  r.lo * x (leftIx n i) + r.di * x i + r.up * x (rightIx n i) - r.rhs

/-- The pieces made from slopes: piece i the Hermite cubic from `(q i, m i)` to `(q (i+1), m (i+1))`. -/
def piecesOf (τ q : Nat → K) (m : Nat) (e : Ends K) (x : Nat → K) (i : Nat) : Piece K :=
  hermite (q i) (q (i + 1)) (slopeAt m e x i) (slopeAt m e x (i + 1)) (τ (i + 1) - τ i)

/-- The specification of the through-spline: pieces `B 0 .. B (m-1)` over `τ`, each a cubic,
through the points, C1 and C2 at the inner points (and across `τ m ~ τ 0` when periodic), with
the end conditions. This is what `spline([..])` means, coordinate by coordinate. -/
def IsInterp (τ q : Nat → K) (m : Nat) (e : Ends K) (B : Nat → Piece K) : Prop :=
  let h := fun i => τ (i + 1) - τ i
  (∀ i, i < m → B i 0 = q i ∧ B i 3 = q (i + 1)) ∧
  (∀ i, i + 1 < m → d1end (B i) (h i) = d1start (B (i + 1)) (h (i + 1)) ∧
      d2end (B i) (h i) = d2start (B (i + 1)) (h (i + 1))) ∧
  match e with
  | .open_ s f =>
    (match s with
      | .natural => d2start (B 0) (h 0) = 0
      | .slope v => d1start (B 0) (h 0) = v) ∧
    (match f with
      | .natural => d2end (B (m - 1)) (h (m - 1)) = 0
      | .slope v => d1end (B (m - 1)) (h (m - 1)) = v)
  | .periodic =>
    d1end (B (m - 1)) (h (m - 1)) = d1start (B 0) (h 0) ∧
    d2end (B (m - 1)) (h (m - 1)) = d2start (B 0) (h 0)

end Generic

/-! ## Centripetal parameters

`τ 0 = 0`, `τ i = τ (i-1) + s |q i - q (i-1)|^(1/2)`, `s = L / Σ |Δq|^(1/2)`, L the polygon's
length. They are real (fourth roots of rationals); the definition is in `SplineSpec.Interp` over ℝ.
The executable model takes τ as data and the oracle checks the kernel's τ against it with rational
bounds (`SplineSpec.Oracle`). -/

/-! ## The language

`spline(points, ..)` in a sketch (builtins.rs), as one of the forms above:

* `degree:`, `knots:` or `weights:` given: `Nurbs` on the points as poles, open; knots
  `uniformClamped p n` and weights 1 where not given; degree 3, or `n - 1` with fewer points.
* `cv: true` without those: open (`closed: false`) as above, else `closedCV p n` with degree 3 or
  `n - 2` with fewer points.
* otherwise a through-spline at the centripetal parameters: `Ends.periodic` (the points then the
  first again) when `closed: true`, else (the default) `Ends.open_` with
  `slope v` where `start_slope`/`end_slope` (or a tangent tie, its unit tangent) is given and
  `natural` where not. -/

/-- Which of the forms a call is. -/
inductive Form where
  | control (degree : Nat) (closed : Bool)
  | through (closed : Bool)
  deriving Repr, DecidableEq

/-- The language's defaults: from which arguments were given (`degree?`, any of
`knots/weights`, `cv?`, `closed?`, any end slope) and the number of points. None where the call is
refused (a slope with control arguments or `cv: true`, `closed: true` with either, `cv: false` with
control arguments). -/
def formOf (n : Nat) (degree : Option Nat) (knotsOrWeights : Bool) (cv : Option Bool)
    (closed : Option Bool) (sloped : Bool) : Option Form :=
  let control := degree.isSome || knotsOrWeights
  if control && sloped then none
  else if cv = some true && sloped then none
  else if cv = some false && control then none
  else if closed = some true && (control || sloped) then none
  else
    let isCv := cv.getD control
    -- a through-spline is open unless `closed: true` (the owner's rule, 2026-11-23); points on
    -- their own (`cv: true`) are a closed control polygon unless `closed: false`
    let closed' := closed.getD (isCv && !control)
    if isCv then
      if control then
        some (.control (degree.getD (max 1 (min 3 (n - 1)))) false)
      else
        some (.control (max 1 (min 3 (n - 1 - (if closed' then 1 else 0)))) closed')
    else some (.through closed')

end SplineSpec

namespace SplineSpec
/-! The defaults, checked by evaluation. -/
example : formOf 4 none false none none false = some (.through false) := rfl
example : formOf 4 none false none (some true) false = some (.through true) := rfl
example : formOf 4 none false (some true) none false = some (.control 2 true) := rfl
example : formOf 5 none false (some true) (some false) false = some (.control 3 false) := rfl
example : formOf 4 (some 2) false none none false = some (.control 2 false) := rfl
example : formOf 4 none false none (some true) true = none := rfl
end SplineSpec
