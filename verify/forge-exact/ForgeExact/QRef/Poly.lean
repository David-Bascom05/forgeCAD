import ForgeExact.QRef.Run
import ForgeExact.Poly3.Octree

/-!
# The curved reference for polynomial surfaces (tori)

A history with a torus face comes here (`QRef.run`); one without one goes to the quadric path,
which this does not change. Every face becomes a polynomial (`Poly3.P3`): a torus its quartic, a
quadric face its quadric, a cone also its nappe. Monomials are kept with sorted variables and
like ones collected, so faces on one surface share one carrier after scaling the first
coefficient to 1.

* **Volume**: the octree of `Poly3.boxSign` boxes (the bound about each box's centre,
  `Poly3.about_bound`). Boxes decided by `BF.eval3` are in or out at every point off the wall
  (`Poly3.in_box`, `Poly3.out_box`); `[lo, hi]` are exact. `volume_f` is a floating-point
  estimate: vertical slices split at their polynomials' roots, Gauss–Legendre between the
  points where the slices change structure (a report, never a decision).
* **Empty**: no box in, and every undecided leaf keeps no pattern of its undecided signs
  (`Poly3.empty_of_leaves`, with no exclusions).
* **Manifold**: each undecided leaf is checked (and split, within a budget) until one of these
  holds on the box: one undecided surface and a component of its gradient with a certified
  sign (a smooth sheet); two, a component of the cross product of their gradients with a
  certified sign (they cross transversally) and their four sectors not kept as a checkerboard;
  three, the determinant of their gradients with a certified sign (a transversal triple point)
  and the kept octants a disc on the link (`octantsOk`). A leaf left unchecked makes the verdict
  `ambiguous`: tangent contacts with a torus are not decided (not proven touching, so never
  `nonmanifold`). The argument is in research/kernel-audit/curved-l2-proofs.md §5.
-/

namespace ForgeExact.QRef.Poly

open Lean Ref Quad Alg ForgeExact.Poly3

/-! ## Polynomials, executable -/

def sortVs (vs : List (Fin 3)) : List (Fin 3) :=
  vs.filter (· == 0) ++ vs.filter (· == 1) ++ vs.filter (· == 2)

def lexLe : List Nat → List Nat → Bool
  | [], _ => true
  | _ :: _, [] => false
  | a :: as, b :: bs => a < b || (a == b && lexLe as bs)

def vsLe (a b : List (Fin 3)) : Bool :=
  if a.length != b.length then a.length < b.length else lexLe (a.map (·.val)) (b.map (·.val))

def norm (p : P3) : P3 :=
  ((collect (p.map fun m => ⟨m.a, sortVs m.vs⟩)).filter (·.a != 0)).mergeSort fun m n => vsLe m.vs n.vs

def add3 (p q : P3) : P3 := norm (p ++ q)
def sc3 (k : Rat) (p : P3) : P3 := norm (p.map fun m => ⟨k * m.a, m.vs⟩)
def sub3 (p q : P3) : P3 := add3 p (sc3 (-1) q)
def mul3 (p q : P3) : P3 := norm (p.flatMap fun m => q.map fun n => ⟨m.a * n.a, m.vs ++ n.vs⟩)
def c3 (a : Rat) : P3 := if a == 0 then [] else [⟨a, []⟩]
def var3 (v : Fin 3) : P3 := [⟨1, [v]⟩]

def deriv3 (v : Fin 3) (p : P3) : P3 :=
  norm (p.filterMap fun m =>
    let n := m.vs.count v
    if n == 0 then none else some ⟨m.a * n, m.vs.erase v⟩)

def ofQuadric (q : Quadric) : P3 :=
  norm [⟨q.xx, [0, 0]⟩, ⟨q.yy, [1, 1]⟩, ⟨q.zz, [2, 2]⟩, ⟨q.xy, [0, 1]⟩, ⟨q.xz, [0, 2]⟩,
    ⟨q.yz, [1, 2]⟩, ⟨q.lx, [0]⟩, ⟨q.ly, [1]⟩, ⟨q.lz, [2]⟩, ⟨q.c, []⟩]

/-- The ring torus's quartic, negative inside:
`(|d|² + R² − r²)² − 4R² (|d|² − (d·z)²/|z|²)`, `d = p − o`. Inside, `(ρ − R)² + w² < r²`
(`ρ`, `w` the distances from the axis and along it): the quartic is
`((ρ − R)² + w² − r²)((ρ + R)² + w² − r²)` and its second factor is positive when `r < R`. -/
def torusP (f : QFace) : P3 :=
  let d : Fin 3 → P3 := fun v => add3 (var3 v) (c3 (-(coord f.o v)))
  let s2 := add3 (mul3 (d 0) (d 0)) (add3 (mul3 (d 1) (d 1)) (mul3 (d 2) (d 2)))
  let w := add3 (sc3 f.z.x (d 0)) (add3 (sc3 f.z.y (d 1)) (sc3 f.z.z (d 2)))
  let R2 := f.big * f.big
  let a := add3 s2 (c3 (R2 - f.r * f.r))
  sub3 (mul3 a a) (sc3 (4 * R2) (sub3 s2 (sc3 (1 / f.z.norm2) (mul3 w w))))

def _root_.ForgeExact.QRef.QFace.poly (f : QFace) : P3 := if f.kind == .torus then torusP f else ofQuadric f.quadric

/-- Scaled so the first coefficient is 1, and the scale's sign. -/
def canonP (p : P3) : P3 × Bool :=
  match p.head? with
  | none => (p, true)
  | some m => (sc3 (1 / m.a) p, m.a > 0)

/-! ## The arrangement -/

structure PArr where
  ps : Array P3
  solids : Array (String × Array Lit)
  bf : BF
  deriving Inhabited

def addP (ps : Array P3) (p : P3) : Array P3 × Lit :=
  let (c, s) := canonP p
  match ps.findIdx? (· == c) with
  | some i => (ps, ⟨i, !s⟩)
  | none => (ps.push c, ⟨ps.size, !s⟩)

def arrP (solids : Array QSolid) : PArr := Id.run do
  let mut ps : Array P3 := #[]
  let mut out : Array (String × Array Lit) := #[]
  for s in solids do
    let mut lits : Array Lit := #[]
    for f in s.faces do
      let (ps', l) := addP ps f.poly
      ps := ps'
      lits := lits.push l
      match f.nappe with
      | some n =>
        let (ps'', l2) := addP ps (ofQuadric n)
        ps := ps''
        lits := lits.push l2
      | none => pure ()
    out := out.push (s.op, lits)
  return ⟨ps, out, historyBF out⟩

def signsP (A : PArr) (b : Cube) : Array Int := A.ps.map fun p => Poly3.boxSign p b.c b.h

def PArr.keptK (A : PArr) (sg : Array Int) : Option Bool := A.bf.eval3 fun k => sg.getD k 0

def PArr.anyKept (A : PArr) (sg : Array Int) : Option Bool :=
  let und := ((Array.range sg.size).filter fun i => sg[i]! == 0).toList
  if und.length > 12 then none
  else if BF.allFalse A.bf [] und (fun i => decide (sg.getD i 0 > 0)) then some false
  else some true

def octreeP (A : PArr) (root : Cube) (maxBoxes : Nat) (maxDepth : Nat) : Tree := Id.run do
  let mut level : Array Cube := #[root]
  let mut inVol : Rat := 0
  let mut witness : Option V3 := none
  let mut boxes := 0
  let mut und : Array Cube := #[]
  for depth in [0:maxDepth + 1] do
    let mut next : Array Cube := #[]
    for b in level do
      boxes := boxes + 1
      match A.keptK (signsP A b) with
      | some true =>
        inVol := inVol + 8 * b.h * b.h * b.h
        if witness.isNone then witness := some b.c
      | some false => pure ()
      | none => next := next.push b
    if depth == maxDepth || boxes + 8 * next.size > maxBoxes then
      und := next
      break
    level := next.flatMap children
  let undVol := und.foldl (fun acc b => acc + 8 * b.h * b.h * b.h) 0
  return ⟨inVol, undVol, und, witness, boxes⟩

/-! ## The local manifold check -/

def dot3 (a b : Array P3) : P3 := add3 (mul3 a[0]! b[0]!) (add3 (mul3 a[1]! b[1]!) (mul3 a[2]! b[2]!))

def cross3 (a b : Array P3) : Array P3 :=
  #[sub3 (mul3 a[1]! b[2]!) (mul3 a[2]! b[1]!), sub3 (mul3 a[2]! b[0]!) (mul3 a[0]! b[2]!),
    sub3 (mul3 a[0]! b[1]!) (mul3 a[1]! b[0]!)]

structure Ctx where
  A : PArr
  grads : Array (Array P3)
  /-- the cross products of the gradients, for `i < j` at `i * n + j` -/
  crosses : Array (Array P3)

def Ctx.of (A : PArr) : Ctx :=
  let n := A.ps.size
  let grads := A.ps.map fun p => #[deriv3 0 p, deriv3 1 p, deriv3 2 p]
  let crosses := (Array.range (n * n)).map fun k =>
    let (i, j) := (k / n, k % n)
    if i < j then cross3 grads[i]! grads[j]! else #[]
  ⟨A, grads, crosses⟩

def certNZ (qs : Array P3) (b : Cube) : Bool := qs.any fun q => Poly3.boxSign q b.c b.h != 0

/-- Bits of `s` as the signs of `U` (others as certified). -/
def keptAt (A : PArr) (sg : Array Int) (U : Array Nat) (s : Nat) : Bool :=
  A.bf.eval2 fun k =>
    match U.findIdx? (· == k) with
    | some t => (s >>> t) % 2 == 1
    | none => sg.getD k 0 > 0

/-- Two transversal sheets: their four sectors are not kept as a checkerboard. -/
def pairOk (K : Nat → Bool) : Bool := !(K 0 == K 3 && K 1 == K 2 && K 0 != K 1)

def connected (K : Nat → Bool) (val : Bool) : Bool := Id.run do
  let nodes := (List.range 8).filter fun s => K s == val
  match nodes.head? with
  | none => return true
  | some s0 =>
    let mut seen : Array Nat := #[s0]
    let mut stack : Array Nat := #[s0]
    while !stack.isEmpty do
      let s := stack.back!
      stack := stack.pop
      for t in [0:3] do
        let n := s ^^^ (1 <<< t)
        if K n == val && !seen.contains n then
          seen := seen.push n; stack := stack.push n
    return seen.size == nodes.length

/-- Three transversal sheets: the kept octants form a disc on the link sphere (the
octahedron): connected, with connected complement and no checkerboard round any vertex. -/
def octantsOk (K : Nat → Bool) : Bool := Id.run do
  if (List.range 8).all (K · == K 0) then return true
  if !(connected K true && connected K false) then return false
  for fixed in [0:3] do
    for v in [0:2] do
      let others := (List.range 3).filter (· != fixed)
      let at_ := fun (a b : Nat) => K ((v <<< fixed) ||| (a <<< others[0]!) ||| (b <<< others[1]!))
      if !pairOk fun s => at_ (s % 2) (s / 2) then return false
  return true

inductive LC
  | ok
  | und (note : String)
  deriving Inhabited

/-- The box is checked (`ok`) or split, while the budget lasts. -/
partial def check (C : Ctx) (b : Cube) (depth : Nat) : StateM Nat LC := do
  let A := C.A
  let sg := signsP A b
  if (A.keptK sg).isSome then return .ok
  let U := (Array.range sg.size).filter fun k => sg[k]! == 0
  let n := A.ps.size
  let K := keptAt A sg U
  let good : Bool :=
    if U.size == 1 then certNZ C.grads[U[0]!]! b
    else if U.size == 2 then certNZ C.crosses[U[0]! * n + U[1]!]! b && pairOk K
    else if U.size == 3 then
      certNZ #[dot3 C.grads[U[0]!]! C.crosses[U[1]! * n + U[2]!]!] b && octantsOk K
    else false
  if good then return .ok
  let left ← get
  if depth == 0 || left < 8 then
    return .und s!"surfaces {U.toList} not shown transversal near {fmt b.c} (half-width {qf b.h})"
  set (left - 8)
  for ch in children b do
    match ← check C ch (depth - 1) with
    | .ok => pure ()
    | u => return u
  return .ok

/-! ## Floating-point volume estimate -/

/-- A polynomial as floats: coefficient and exponents of x, y, z. -/
abbrev FP := Array (Float × Nat × Nat × Nat)

def toFP (p : P3) : FP := p.toArray.map fun m => (qf m.a, m.vs.count 0, m.vs.count 1, m.vs.count 2)

def fpow (x : Float) : Nat → Float
  | 0 => 1
  | n + 1 => x * fpow x n

def FP.eval (p : FP) (x y z : Float) : Float :=
  p.foldl (fun acc (a, i, j, k) => acc + a * fpow x i * fpow y j * fpow z k) 0

/-- The polynomial on the vertical line through `(x, y)`, coefficients in `z`, low first. -/
def FP.inZ (p : FP) (x y : Float) : Array Float := Id.run do
  let mut cs : Array Float := Array.replicate 10 0
  for (a, i, j, k) in p do
    if k < cs.size then cs := cs.set! k (cs[k]! + a * fpow x i * fpow y j)
  return cs

def zRoots (fp : Array FP) (x y z0 z1 : Float) : Array Float :=
  fp.foldl (fun acc p => acc ++ froots (p.inZ x y) z0 z1) #[]

def sliceLenP (A : PArr) (fp : Array FP) (x y z0 z1 : Float) : Float := Id.run do
  let rs := ((zRoots fp x y z0 z1).push z0 |>.push z1).qsort (· < ·)
  let mut total : Float := 0
  for k in [0:rs.size - 1] do
    let za := rs[k]!
    let zb := rs[k + 1]!
    if zb ≤ za then continue
    let zm := (za + zb) / 2
    if A.bf.eval2 (fun idx => (fp.getD idx #[]).eval x y zm > 0) then total := total + (zb - za)
  return total

/-- Where `g` (an integer signature) changes along `[a, b]`: a grid of `n`, then bisection. -/
def changes (g : Float → Nat) (a b : Float) (n : Nat) : Array Float := Id.run do
  let mut out : Array Float := #[a, b]
  let mut xa := a
  let mut ca := g a
  for k in [1:n + 1] do
    let xb := a + (b - a) * k.toFloat / n.toFloat
    let cb := g xb
    if cb != ca then
      let mut lo := xa
      let mut hi := xb
      for _ in [0:40] do
        let m := (lo + hi) / 2
        if g m == ca then lo := m else hi := m
      out := out.push ((lo + hi) / 2)
    xa := xb; ca := cb
  return sortUniq out

def yBreaksP (fp : Array FP) (x y0 y1 z0 z1 : Float) : Array Float :=
  changes (fun y => (zRoots fp x y z0 z1).size) y0 y1 64

def sectionAreaP (A : PArr) (fp : Array FP) (x y0 y1 z0 z1 : Float) (pieces : Nat) : Float := Id.run do
  let ys := yBreaksP fp x y0 y1 z0 z1
  let mut total : Float := 0
  for k in [0:ys.size - 1] do
    total := total + integrate (fun y => sliceLenP A fp x y z0 z1) ys[k]! ys[k + 1]! pieces
  return total

def estimateP (A : PArr) (lo hi : V3) : Float × Float := Id.run do
  let fp := A.ps.map toFP
  let (x0, y0, z0) := (qf lo.x, qf lo.y, qf lo.z)
  let (x1, y1, z1) := (qf hi.x, qf hi.y, qf hi.z)
  let sigX := fun (x : Float) =>
    let ys := yBreaksP fp x y0 y1 z0 z1
    (Array.range (ys.size - 1)).foldl
      (fun n k => n * 7 + (zRoots fp x ((ys[k]! + ys[k + 1]!) / 2) z0 z1).size) ys.size
  let xs := changes sigX x0 x1 64
  let mut coarse : Float := 0
  let mut fine : Float := 0
  for k in [0:xs.size - 1] do
    coarse := coarse + integrate (fun x => sectionAreaP A fp x y0 y1 z0 z1 1) xs[k]! xs[k + 1]! 1
    fine := fine + integrate (fun x => sectionAreaP A fp x y0 y1 z0 z1 3) xs[k]! xs[k + 1]! 3
  return (fine, (fine - coarse).abs)

/-! ## The driver -/

def runPoly (j : Json) (solids0 : Array QSolid) : Except String Json := do
  let l ← dyadic (← j.getObjVal? "linear")
  let strict := (j.getObjValAs? String "rule" |>.toOption.getD "strict") != "snap"
  let maxBoxes := (j.getObjValAs? Nat "boxes" |>.toOption.getD 40000)
  let maxDepth := (j.getObjValAs? Nat "depth" |>.toOption.getD 8)
  let checkBudget := (j.getObjValAs? Nat "check_boxes" |>.toOption.getD 60000)
  let mut solids := solids0
  let mut ambiguous : Array String := #[]
  if !strict then
    let (s', n) := snapFaces l solids
    solids := s'
    ambiguous := ambiguous ++ n
  let faces := solids.foldl (fun acc s => acc ++ s.faces) #[]
  -- near decisions between faces of the quadric kinds (tori: by the local check)
  let mut nearNotes : Array String := #[]
  for a in [0:faces.size] do
    for b in [a + 1:faces.size] do
      let f := faces[a]!
      let g := faces[b]!
      if f.kind == .torus || g.kind == .torus then continue
      if (canon f.quadric).1 == (canon g.quadric).1 then continue
      match nearPair l f g with
      | some n => nearNotes := nearNotes.push n
      | none => pure ()
  if strict then ambiguous := ambiguous ++ nearNotes
  let A := arrP solids
  let bb := (faces.foldl (fun b f => b.hull f.box) faces[0]!.box).grow 1
  let ctr := (1 / 2 : Rat) * (bb.lo + bb.hi)
  let d := bb.hi - bb.lo
  let half := (1 / 2 : Rat) * qmax d.x (qmax d.y d.z)
  let tree := octreeP A ⟨ctr, half⟩ maxBoxes maxDepth
  let (est, estErr) := estimateP A bb.lo bb.hi
  let ph := (num j "probe_h").toOption.getD l
  let probes ← match j.getObjVal? "probes" with
    | .ok ps => do (← ps.getArr?).mapM v3
    | .error _ => pure #[]
  let probeAns : Array Int := probes.map fun p =>
    let sg := A.ps.map fun q => Poly3.boxSign q p ph
    if sg.any (· == 0) then 0 else if A.bf.eval2 (fun i => sg[i]! > 0) then 1 else -1
  let empty := tree.witness.isNone && tree.und.all fun b => A.anyKept (signsP A b) == some false
  -- the local check of every undecided leaf
  let mut opens : Array String := #[]
  if !empty then
    let C := Ctx.of A
    let mut budget := checkBudget
    for b in tree.und do
      let (r, left) := (check C b 12).run budget
      budget := left
      match r with
      | LC.ok => pure ()
      | LC.und n => opens := opens.push n; break
  let verdict :=
    if !ambiguous.isEmpty then "ambiguous"
    else if empty then "empty"
    else if tree.witness.isNone then "ambiguous"
    else if !opens.isEmpty then "ambiguous"
    else if !nearNotes.isEmpty then "sliver"
    else "solid"
  let mut notes : Array String := #["polynomial path (a torus face)"]
  if tree.witness.isNone && !empty then notes := notes.push "no box certified in, and emptiness not proven"
  notes := notes ++ ambiguous ++ (if strict then #[] else nearNotes) ++ opens
  return Json.mkObj [
    ("verdict", verdict),
    ("volume_lo", qstr tree.inVol), ("volume_hi", qstr (tree.inVol + tree.undVol)),
    ("volume_lo_f", toJson (qf tree.inVol)), ("volume_hi_f", toJson (qf (tree.inVol + tree.undVol))),
    ("volume_f", toJson est), ("volume_err", toJson estErr), ("volume", qstr tree.inVol),
    ("boxes", tree.boxes), ("carriers", A.ps.size),
    ("probes", toJson probeAns),
    ("notes", toJson notes)]

/-- Histories with a torus face take the polynomial path; the rest the quadric one. -/
def run (line : String) : Except String Json := do
  let j ← Json.parse line
  let ss ← (← j.getObjVal? "solids").getArr?
  let mut solids : Array QSolid := #[]
  for k in [0:ss.size] do solids := solids.push (← solidOf k ss[k]!)
  if solids.any (·.faces.any (·.kind == .torus)) then runPoly j solids else runCurved j

end ForgeExact.QRef.Poly
