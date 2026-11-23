/-
`lake exe splineoracle`: the Rust spline against the specification, case by case.

Each input line is one curve the Rust built, with the data it was built from and the Bezier pieces
it made (every f64 written exactly, `m:e` for m·2^e). The oracle computes the specification's curve
from the same data in exact rationals (`SplineSpec.Defs` at `Rat`) and prints, per line,

  `ok` when every Bezier point of the Rust is within the tolerance of the specification's, else
  `bad <what>`, then (tab-separated) the bound it found, as a decimal, for the record.

The tolerance is the curve's scale (the largest coordinate of its data, at least 1) over 2^30.
Two curves whose Bezier points are within e of each other, piece by piece and at the same piece
parameter, are within e everywhere (each piece is a convex combination of its points:
`SplineLoops.bb_sum`, `bb_nonneg`); for rational pieces the division is bounded below.

* `C p n d u.. w.. P.. | k b.. H..`: a rational B-spline of degree p on n poles of d coordinates
  (knots u, weights w, poles P pole by pole), the Rust's k pieces over breaks b, with H the
  (p+1)·k homogeneous points (x w, y w, z w, w). The specification's curve on each span is
  `homOn / weightOn` (Cox–de Boor); being a polynomial of degree p there, its Bernstein
  coefficients are found exactly from p + 1 values.
* `T d m e.. q.. | b.. H..`: a through-spline on q 0 .. q m (`e` is `periodic`, or `open` and two
  of `natural` / `slope v..`), the Rust's breaks b (its parameters) and its m cubic pieces. The
  specification's parameters are centripetal and irrational: they are enclosed in rational
  intervals; the exact slopes at the Rust's parameters, carried to the true ones by
  `slopes_bound`, give the specification's pieces in intervals, and the distance is to the far end.
-/
import SplineSpec.Defs

namespace SplineSpec.Oracle

open SplineSpec

/-! ## Numbers -/

instance : Inhabited (EndCond Rat) := ⟨.natural⟩
instance : Inhabited (Ends Rat) := ⟨.periodic⟩

def ratAbs (x : Rat) : Rat := if x < 0 then -x else x
def ratMax (a b : Rat) : Rat := if a < b then b else a
def ratMin (a b : Rat) : Rat := if a < b then a else b

/-- `m · 2^e` exactly. -/
def dyadic (m e : Int) : Rat :=
  if e ≥ 0 then (m : Rat) * ((2 ^ e.toNat : Nat) : Rat) else (m : Rat) / ((2 ^ (-e).toNat : Nat) : Rat)

def parseNum (s : String) : Option Rat :=
  match s.splitOn ":" with
  | [m, e] => do
    let m ← m.toInt?
    let e ← e.toInt?
    pure (dyadic m e)
  | _ => none

/-- Outward rounding to a multiple of 2^-PREC, to keep interval endpoints small. -/
def PREC : Nat := 160
def down (x : Rat) : Rat := ((x * ((2 ^ PREC : Nat) : Rat)).floor : Rat) / ((2 ^ PREC : Nat) : Rat)
def up (x : Rat) : Rat := ((x * ((2 ^ PREC : Nat) : Rat)).ceil : Rat) / ((2 ^ PREC : Nat) : Rat)

/-- A closed interval of rationals; `ok = false` once a division met zero. -/
structure RI where
  lo : Rat
  hi : Rat
  ok : Bool
  deriving DecidableEq, Inhabited

def RI.pt (x : Rat) : RI := ⟨x, x, true⟩
def RI.of (lo hi : Rat) : RI := ⟨down lo, up hi, decide (lo ≤ hi)⟩

instance : OfNat RI 0 := ⟨RI.pt 0⟩
instance : OfNat RI 1 := ⟨RI.pt 1⟩
instance : Add RI := ⟨fun a b => ⟨down (a.lo + b.lo), up (a.hi + b.hi), a.ok && b.ok⟩⟩
instance : Sub RI := ⟨fun a b => ⟨down (a.lo - b.hi), up (a.hi - b.lo), a.ok && b.ok⟩⟩
instance : Mul RI := ⟨fun a b =>
  let c := [a.lo * b.lo, a.lo * b.hi, a.hi * b.lo, a.hi * b.hi]
  ⟨down (c.foldl ratMin (a.lo * b.lo)), up (c.foldl ratMax (a.lo * b.lo)), a.ok && b.ok⟩⟩
instance : Div RI := ⟨fun a b =>
  if 0 < b.lo ∨ b.hi < 0 then
    let c := [a.lo / b.lo, a.lo / b.hi, a.hi / b.lo, a.hi / b.hi]
    ⟨down (c.foldl ratMin (a.lo / b.lo)), up (c.foldl ratMax (a.lo / b.lo)), a.ok && b.ok⟩
  else ⟨0, 0, false⟩⟩
instance : LT RI := ⟨fun a b => a.hi < b.lo⟩
instance : LE RI := ⟨fun a b => a.hi ≤ b.lo⟩
instance : DecidableRel (α := RI) (· < ·) := fun a b => inferInstanceAs (Decidable (a.hi < b.lo))
instance : DecidableRel (α := RI) (· ≤ ·) := fun a b => inferInstanceAs (Decidable (a.hi ≤ b.lo))

/-- The largest distance from `x` to a point of `I`. -/
def RI.far (I : RI) (x : Rat) : Rat := ratMax (ratAbs (x - I.lo)) (ratAbs (x - I.hi))
/-- The least absolute value in `I`. -/
def RI.absLo (I : RI) : Rat := if 0 < I.lo then I.lo else if I.hi < 0 then -I.hi else 0
/-- The largest absolute value in `I`. -/
def RI.absHi (I : RI) : Rat := ratMax (ratAbs I.lo) (ratAbs I.hi)

/-- Rational bounds on √x, x ≥ 0, to 2^-80 relative. -/
def sqrtI (x : RI) : RI :=
  let k : Nat := 2 ^ 80
  let lo := if x.lo ≤ 0 then (0 : Rat) else
    let n := x.lo.num.toNat * x.lo.den * k * k
    ((Nat.sqrt n : Nat) : Rat) / ((x.lo.den * k : Nat) : Rat)
  let hi :=
    let h := ratMax x.hi 0
    let n := h.num.toNat * h.den * k * k
    ((Nat.sqrt n + 1 : Nat) : Rat) / ((h.den * k : Nat) : Rat)
  ⟨lo, hi, x.ok && decide (0 ≤ x.lo)⟩

/-- A decimal for the record (about 6 significant figures). -/
def showRat (x : Rat) : String :=
  if x == 0 then "0" else
  let a := ratAbs x
  -- the exponent e with 10^e ≤ a < 10^(e+1)
  let rec findE (e : Int) (fuel : Nat) : Int :=
    match fuel with
    | 0 => e
    | f + 1 =>
      let p : Rat := if e ≥ 0 then ((10 ^ e.toNat : Nat) : Rat) else 1 / ((10 ^ (-e).toNat : Nat) : Rat)
      if a < p then findE (e - 1) f else if a ≥ p * 10 then findE (e + 1) f else e
  let e := findE 0 400
  let p : Rat := if e ≥ 0 then ((10 ^ e.toNat : Nat) : Rat) else 1 / ((10 ^ (-e).toNat : Nat) : Rat)
  let m := ((a / p) * 100000).ceil
  let s := toString m
  (if x < 0 then "-" else "") ++ s.take 1 ++ "." ++ s.drop 1 ++ "e" ++ toString e

/-! ## Exact linear algebra -/

/-- Solves `A x = b` exactly by Gaussian elimination with a nonzero pivot; none if singular. -/
def solve (A : Array (Array Rat)) (b : Array Rat) : Option (Array Rat) := Id.run do
  let n := b.size
  let mut M : Array (Array Rat) := (Array.range n).map fun i => (A[i]!).push b[i]!
  for c in [0:n] do
    -- a pivot at or below c
    let mut piv := n
    for r in [c:n] do
      if piv == n && M[r]![c]! != 0 then piv := r
    if piv == n then return none
    let rowP := M[piv]!
    M := (M.set! piv M[c]!).set! c rowP
    let pr := M[c]!
    for r in [0:n] do
      if r != c then
        let f := M[r]![c]! / pr[c]!
        if f != 0 then
          M := M.set! r ((Array.range (n + 1)).map fun k => M[r]![k]! - f * pr[k]!)
  return some ((Array.range n).map fun i => M[i]![n]! / M[i]![i]!)

def binom (n k : Nat) : Nat := Id.run do
  let mut r := 1
  for i in [0:k] do r := r * (n - i) / (i + 1)
  return r

/-- The Bernstein coefficients of the degree-p polynomial with values `f k` at `k / p`. -/
def bernsteinOf (p : Nat) (f : Array Rat) : Option (Array Rat) :=
  if p = 0 then some f else
  let V := (Array.range (p + 1)).map fun (k : Nat) =>
    let s : Rat := (k : Rat) / (p : Rat)
    (Array.range (p + 1)).map fun (l : Nat) => (binom p l : Rat) * s ^ l * (1 - s) ^ (p - l)
  solve V f

/-! ## Reading a line -/

structure Reader where
  toks : Array String
  pos : Nat

def Reader.next (r : Reader) : Option (String × Reader) :=
  if h : r.pos < r.toks.size then some (r.toks[r.pos], { r with pos := r.pos + 1 }) else none

def Reader.num (r : Reader) : Option (Rat × Reader) := do
  let (s, r) ← r.next
  let x ← parseNum s
  pure (x, r)

def Reader.nat (r : Reader) : Option (Nat × Reader) := do
  let (s, r) ← r.next
  let x ← s.toNat?
  pure (x, r)

def Reader.nums (r : Reader) (k : Nat) : Option (Array Rat × Reader) := Id.run do
  let mut r := r
  let mut out := #[]
  for _ in [0:k] do
    match r.num with
    | some (x, r') => out := out.push x; r := r'
    | none => return none
  return some (out, r)

def Reader.expect (r : Reader) (s : String) : Option Reader := do
  let (t, r) ← r.next
  if t == s then pure r else none

def at! (a : Array Rat) (i : Nat) : Rat := a[i]?.getD 0

/-! ## Control splines -/

/-- The bound for a rational B-spline: none where the Rust's pieces are not the spec's spans. -/
def checkControl (r : Reader) : Except String (Rat × Rat) := do
  let some (p, r) := r.nat | throw "degree"
  let some (n, r) := r.nat | throw "count"
  let some (d, r) := r.nat | throw "dims"
  let some (u, r) := r.nums (n + p + 1) | throw "knots"
  let some (w, r) := r.nums n | throw "weights"
  let some (P, r) := r.nums (n * d) | throw "poles"
  let some r := r.expect "|" | throw "bar"
  let some (k, r) := r.nat | throw "pieces"
  let some (b, r) := r.nums (k + 1) | throw "breaks"
  let some (H, _) := r.nums (k * (p + 1) * 4) | throw "points"
  let U := fun i => at! u i
  -- the spans of the specification: the nonempty [u j, u (j+1)] in [u p, u n]
  let spans := (List.range n).filter fun j => p ≤ j ∧ U j < U (j + 1)
  if spans.length ≠ k then throw s!"{k} pieces where the knots make {spans.length}"
  let mut scale : Rat := 1
  for x in P do scale := ratMax scale (ratAbs x)
  let mut eh : Rat := 0  -- the largest homogeneous difference
  let mut ew : Rat := 0  -- the largest weight difference
  let mut wmin : Rat := -1 -- the least of the spec's Bernstein weights
  for (j, s) in spans.zipIdx do
    if b[s]! ≠ U j ∨ b[s + 1]! ≠ U (j + 1) then throw s!"piece {s} over [{showRat b[s]!}, {showRat b[s+1]!}], span [{showRat (U j)}, {showRat (U (j+1))}]"
    let a := U j
    let h := U (j + 1) - U j
    let ts := (Array.range (p + 1)).map fun (i : Nat) => a + h * ((i : Rat) / (p : Rat))
    let W := fun i => at! w i
    let some wb := bernsteinOf p (ts.map fun t => weightOn U p n j W t) | throw "weight basis"
    for c in [0:4] do
      let coef ← if c = 3 then pure wb else
        if c < d then
          let Pc := fun i => at! P (i * d + c)
          match bernsteinOf p (ts.map fun t => homOn U p n j W Pc t) with
          | some x => pure x
          | none => throw "basis"
        else pure ((Array.range (p + 1)).map fun _ => (0 : Rat))
      for l in [0:p + 1] do
        let rust := H[(s * (p + 1) + l) * 4 + c]!
        let diff := ratAbs (rust - coef[l]!)
        if c = 3 then ew := ratMax ew diff else eh := ratMax eh diff
    for l in [0:p + 1] do
      wmin := if wmin < 0 then wb[l]! else ratMin wmin wb[l]!
  if wmin - ew ≤ 0 then throw "weights near zero"
  -- |x̂/ŵ - x/w| ≤ (|Δx| + |x/w| |Δw|) / ŵ, |x/w| ≤ scale (convex hull), ŵ ≥ wmin - ew
  let bound := (eh + scale * ew) / (wmin - ew)
  return (bound, scale)

/-! ## Through-splines -/

/-- Parses an end condition, one value per coordinate for a slope. -/
def readEnd (r : Reader) (d : Nat) : Option (Array (EndCond Rat) × Reader) := do
  let (t, r) ← r.next
  if t == "natural" then pure ((Array.range d).map fun _ => .natural, r)
  else if t == "slope" then
    let (v, r) ← r.nums d
    pure (v.map .slope, r)
  else none

def checkThrough (r : Reader) : Except String (Rat × Rat) := do
  let some (d, r) := r.nat | throw "dims"
  let some (m, r) := r.nat | throw "count"
  let some (kind, r) := r.next | throw "ends"
  let (ends, r) ← if kind == "periodic" then
      pure ((Array.range d).map fun _ => (Ends.periodic : Ends Rat), r)
    else if kind == "open" then do
      let some (s, r) := readEnd r d | throw "start"
      let some (f, r) := readEnd r d | throw "finish"
      pure ((Array.range d).map fun c => Ends.open_ s[c]! f[c]!, r)
    else throw "kind"
  let some (q, r) := r.nums ((m + 1) * d) | throw "points"
  let some r := r.expect "|" | throw "bar"
  let some (b, r) := r.nums (m + 1) | throw "breaks"
  let some (H, _) := r.nums (m * 4 * 4) | throw "points"
  let Q := fun c i => at! q (i * d + c)
  let mut scale : Rat := 1
  for x in q do scale := ratMax scale (ratAbs x)
  -- the centripetal parameters, enclosed: chord lengths, their square roots, scaled to the length
  let chord := (Array.range m).map fun i =>
    let s := (List.range d).foldl (fun acc c => acc + (Q c (i + 1) - Q c i) * (Q c (i + 1) - Q c i)) 0
    sqrtI (RI.pt s)
  let root := chord.map sqrtI
  let L := chord.foldl (· + ·) 0
  let S := root.foldl (· + ·) 0
  let k := L / S
  let mut tauI : Array RI := #[0]
  let mut acc : RI := 0
  for i in [0:m] do
    acc := acc + root[i]!
    tauI := tauI.push (k * acc)
  if tauI.any (!·.ok) then throw "a repeated point"
  let mut etau : Rat := 0
  for i in [0:m + 1] do etau := ratMax etau (tauI[i]!.far b[i]!)
  let tauHat := fun i => at! b i
  let mut ep : Rat := 0
  let mut eb : Rat := 0
  for c in [0:d] do
    let e := ends[c]!
    let n := unknowns m e
    -- the exact slopes at the Rust's parameters
    let A := (Array.range n).map fun i =>
      let row := rows tauHat (Q c) m e i
      (Array.range n).map fun j =>
        (if j = leftIx n i then row.lo else 0) + (if j = i then row.di else 0) +
          (if j = rightIx n i then row.up else 0)
    let rhs := (Array.range n).map fun i => (rows tauHat (Q c) m e i).rhs
    let some x := solve A rhs | throw "singular"
    let X := fun i => at! x i
    if (List.range n).any fun i => residual tauHat (Q c) m e X i ≠ 0 then throw "not a solution"
    -- carried to the true parameters: residual over the parameter box, over each row's margin
    let tau : Nat → RI := fun i => tauI[i]?.getD 0
    let eI : Ends RI := match e with
      | .periodic => .periodic
      | .open_ s f =>
        let cv : EndCond Rat → EndCond RI := fun | .natural => .natural | .slope v => .slope (RI.pt v)
        .open_ (cv s) (cv f)
    let QI := fun i => RI.pt (Q c i)
    let XI := fun i => RI.pt (X i)
    let mut bm : Rat := 0
    for i in [0:n] do
      let res := residual tau QI m eI XI i
      let row := rows tau QI m eI i
      let margin := row.di.absLo - row.lo.absHi - row.up.absHi
      if !res.ok || margin ≤ 0 then throw s!"row {i} not dominant in the box"
      bm := ratMax bm (res.absHi / margin)
    -- the specification's pieces, in intervals
    let XB := fun i => RI.of (X i - bm) (X i + bm)
    for i in [0:m] do
      let piece := piecesOf tau QI m eI XB i
      for l in [0:4] do
        let rust := H[(i * 4 + l) * 4 + c]!
        let I := piece l
        if !I.ok then throw "interval"
        eb := ratMax eb (I.far rust)
    ep := ratMax ep bm
  -- the Rust's weights are 1 and its fourth coordinate 0 past d
  for i in [0:m] do
    for l in [0:4] do
      if H[(i * 4 + l) * 4 + 3]! ≠ 1 then throw "a weight not 1"
      for c in [d:3] do
        if H[(i * 4 + l) * 4 + c]! ≠ 0 then throw "a coordinate past d"
  if etau > scale / ((2 ^ 30 : Nat) : Rat) then throw s!"parameters off by {showRat etau}"
  return (eb, scale)

def checkLine (line : String) : String :=
  let toks := (line.splitOn " ").filter (· ≠ "") |>.toArray
  let r : Reader := ⟨toks, 1⟩
  let res := match toks[0]? with
    | some "C" => checkControl r
    | some "T" => checkThrough r
    | _ => .error "kind"
  match res with
  | .error e => s!"bad {e}"
  | .ok (bound, scale) =>
    let tol := scale / ((2 ^ 30 : Nat) : Rat)
    if bound ≤ tol then s!"ok\t{showRat bound}" else s!"bad over\t{showRat bound}"

partial def loop (stdin : IO.FS.Stream) (stdout : IO.FS.Stream) : IO Unit := do
  let line ← stdin.getLine
  if line.isEmpty then return
  let l := String.mk (line.toList.reverse.dropWhile Char.isWhitespace).reverse
  if l.isEmpty then loop stdin stdout else
  stdout.putStrLn (checkLine l)
  stdout.flush
  loop stdin stdout

end SplineSpec.Oracle

def main : IO Unit := do
  SplineSpec.Oracle.loop (← IO.getStdin) (← IO.getStdout)
