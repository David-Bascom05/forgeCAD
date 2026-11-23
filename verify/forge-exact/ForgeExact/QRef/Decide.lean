import Std.Data.HashMap
import ForgeExact.QRef.Faces

/-!
# The curved reference: the boolean of solids bounded by planes and quadrics, decided exactly

* **Carriers.** Every face's quadric, scaled so its first nonzero coefficient is 1, so faces on
  one surface share one quadric (an identical cylinder, a cap flush with a face). A solid is the
  conjunction of literals "quadric `i` has sign `±`" (`Lit`).
* **The arrangement** is the sign-pattern decomposition of these quadrics (`Quad.arrangement`,
  proven compatible). A point off every quadric is in the regularised boolean iff the history
  holds on its pattern (`Quad.member_off_wall`); a box that certifies every sign (`Quad.boxSign`,
  `Quad.member_box`) is wholly in or wholly out.
* **Volume**: an octree of such boxes. The boxes certified in give a lower bound and those left
  undecided bound the rest: `[lo, hi]`, exact rationals. `volume_f` is a high-resolution estimate
  from vertical lines, each split exactly at its quadrics' roots (floating point, a report, never
  a decision).
* **Contacts**: two quadrics that touch without crossing. A rational `t` with `A_i + t A_j`
  semidefinite (a root of the pencil's determinant) gives an exclusion certificate
  (`Quad.pairCheck`, proven) and the contact locus, the null space of the pencil's member. Along
  the locus the other quadrics' signs are decided exactly (at algebraic points by
  `Alg.Num.sign`). There the result is non-manifold iff it keeps the two touching sides and not
  the pinched one between them, or the pinched one alone.
* **Verdict**: `nonmanifold` at a certified contact; `empty` when no box is in and every
  undecided box keeps no pattern its certified signs and the exclusions allow; else `solid`
  when some box is certified in. Near decisions (two carriers within LINEAR of touching or of
  being one, not exactly) make it `ambiguous` under the strict rule and `sliver` under the snap
  rule, which first merges tolerance-equal planes (§6.1, `Snap`).
-/

namespace ForgeExact.QRef

open Lean Ref Quad Alg Std

/-! ## Carriers -/

def Quadric.coeffs (q : Quadric) : Array Rat :=
  #[q.xx, q.yy, q.zz, q.xy, q.xz, q.yz, q.lx, q.ly, q.lz, q.c]

def Quadric.ofCoeffs (a : Array Rat) : Quadric :=
  ⟨a[0]!, a[1]!, a[2]!, a[3]!, a[4]!, a[5]!, a[6]!, a[7]!, a[8]!, a[9]!⟩

/-- Scaled so the first nonzero coefficient is 1, and the scale's sign. -/
def canon (q : Quadric) : Quadric × Bool :=
  let a := Quadric.coeffs q
  match a.find? (· != 0) with
  | none => (q, true)
  | some s => (Quadric.ofCoeffs (a.map (· / s)), s > 0)

/-- Quadric `idx` has sign `pos` (`true`: > 0). -/
structure Lit where
  idx : Nat
  pos : Bool
  deriving Inhabited, BEq

structure Arr where
  qs : Array Quadric
  /-- each solid: its op and literals -/
  solids : Array (String × Array Lit)
  /-- each carrier: the faces on it (solid, face) -/
  faces : Array (Array (Nat × Nat))
  deriving Inhabited

def addQ (qs : Array Quadric) (q : Quadric) : Array Quadric × Lit :=
  let (c, s) := canon q
  match qs.findIdx? (· == c) with
  | some i => (qs, ⟨i, !s⟩)
  | none => (qs.push c, ⟨qs.size, !s⟩)

/-- The arrangement's carriers and each solid's literals (`q < 0` inside). -/
def arrOf (solids : Array QSolid) : Arr := Id.run do
  let mut qs : Array Quadric := #[]
  let mut faces : Array (Array (Nat × Nat)) := #[]
  let mut out : Array (String × Array Lit) := #[]
  for s in solids do
    let mut lits : Array Lit := #[]
    for f in s.faces do
      let (qs', l) := addQ qs f.quadric
      if qs'.size > qs.size then faces := faces.push #[]
      qs := qs'
      faces := faces.modify l.idx (·.push (f.solid, f.index))
      lits := lits.push l
      match f.nappe with
      | some n =>
        let (qs'', l2) := addQ qs n
        if qs''.size > qs.size then faces := faces.push #[]
        qs := qs''
        lits := lits.push l2
      | none => pure ()
    out := out.push (s.op, lits)
  return ⟨qs, out, faces⟩

/-- The history on a sign pattern (`ForgeExact.combine`: new and add unite, cut takes away, keep
intersects). -/
def Arr.keptOn (A : Arr) (σ : Nat → Bool) : Bool :=
  A.solids.foldl (fun r (op, lits) =>
    let inS := lits.all fun l => σ l.idx == l.pos
    if op == "cut" then r && !inS else if op == "keep" then r && inS else r || inS) false

/-! ## Three-valued classification over a box -/

/-- Kleene: `some b` decided, `none` unknown. -/
def Arr.keptK (A : Arr) (sg : Array Int) : Option Bool :=
  let and3 : Option Bool → Option Bool → Option Bool := fun a b =>
    match a, b with
    | some false, _ => some false
    | _, some false => some false
    | some true, some true => some true
    | _, _ => none
  let or3 : Option Bool → Option Bool → Option Bool := fun a b =>
    match a, b with
    | some true, _ => some true
    | _, some true => some true
    | some false, some false => some false
    | _, _ => none
  let not3 : Option Bool → Option Bool := fun a => a.map (!·)
  A.solids.foldl (fun r (op, lits) =>
    let inS := lits.foldl (fun acc l =>
      let s := sg[l.idx]!
      and3 acc (if s == 0 then none else some ((s > 0) == l.pos))) (some true)
    if op == "cut" then and3 r (not3 inS) else if op == "keep" then and3 r inS else or3 r inS)
    (some false)

/-- An exclusion: no point has quadric `i` of sign `si` and `j` of sign `sj`. -/
structure Excl where
  i : Nat
  si : Bool
  j : Nat
  sj : Bool
  deriving Inhabited, BEq

/-- Every pattern of the undecided quadrics allowed by the exclusions: does any keep? `none` past
the enumeration cap. -/
def Arr.anyKept (A : Arr) (sg : Array Int) (ex : Array Excl) : Option Bool := Id.run do
  let und := (Array.range sg.size).filter fun i => sg[i]! == 0
  if und.size > 12 then return none
  for m in [0:2 ^ und.size] do
    let σ : Nat → Bool := fun i =>
      match und.findIdx? (· == i) with
      | some k => (m >>> k) % 2 == 1
      | none => sg[i]! > 0
    let allowed := ex.all fun e => !(σ e.i == e.si && σ e.j == e.sj)
    if allowed && A.keptOn σ then return some true
  return some false

/-! ## The octree -/

structure Cube where
  c : V3
  h : Rat
  deriving Inhabited

structure Tree where
  inVol : Rat
  undVol : Rat
  /-- leaves left undecided -/
  und : Array Cube
  /-- a point certified in, if any -/
  witness : Option V3
  boxes : Nat
  deriving Inhabited

def signsAt (A : Arr) (b : Cube) : Array Int := A.qs.map fun q => boxSign q b.c b.h

def children (b : Cube) : Array Cube := Id.run do
  let h := b.h / 2
  let mut out := #[]
  for dx in [-1, 1] do
    for dy in [-1, 1] do
      for dz in [-1, 1] do
        out := out.push ⟨b.c + ⟨(dx : Rat) * h, (dy : Rat) * h, (dz : Rat) * h⟩, h⟩
  return out

/-- Classify boxes level by level, splitting the undecided, until the budget. -/
def octree (A : Arr) (root : Cube) (maxBoxes : Nat) (maxDepth : Nat) : Tree := Id.run do
  let mut level : Array Cube := #[root]
  let mut inVol : Rat := 0
  let mut witness : Option V3 := none
  let mut boxes := 0
  let mut und : Array Cube := #[]
  for depth in [0:maxDepth + 1] do
    let mut next : Array Cube := #[]
    for b in level do
      boxes := boxes + 1
      match A.keptK (signsAt A b) with
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

/-! ## Floating-point volume estimate: exact splits on vertical lines -/

/-- A quadric's coefficients as floats. -/
structure FQ where
  xx : Float
  yy : Float
  zz : Float
  xy : Float
  xz : Float
  yz : Float
  lx : Float
  ly : Float
  lz : Float
  c : Float
  deriving Inhabited

def FQ.of (q : Quadric) : FQ :=
  ⟨qf q.xx, qf q.yy, qf q.zz, qf q.xy, qf q.xz, qf q.yz, qf q.lx, qf q.ly, qf q.lz, qf q.c⟩

/-- `q = a z² + b z + c` on the vertical line through `(x, y)`. -/
def FQ.fz (q : FQ) (x y : Float) : Float × Float × Float :=
  (q.zz, q.xz * x + q.yz * y + q.lz,
   q.xx * x * x + q.yy * y * y + q.xy * x * y + q.lx * x + q.ly * y + q.c)

def estimate (A : Arr) (lo hi : V3) (n : Nat) : Float := Id.run do
  let fq := A.qs.map FQ.of
  let (x0, y0, z0) := (qf lo.x, qf lo.y, qf lo.z)
  let (x1, y1, z1) := (qf hi.x, qf hi.y, qf hi.z)
  let dx := (x1 - x0) / n.toFloat
  let dy := (y1 - y0) / n.toFloat
  let mut total : Float := 0
  for i in [0:n] do
    for j in [0:n] do
      let x := x0 + (i.toFloat + 0.5) * dx
      let y := y0 + (j.toFloat + 0.5) * dy
      let abc := fq.map fun q => q.fz x y
      let mut roots : Array Float := #[z0, z1]
      for (a, b, c) in abc do
        if a == 0 then
          if b != 0 then roots := roots.push (-c / b)
        else
          let d := b * b - 4 * a * c
          if d > 0 then
            let s := d.sqrt
            roots := roots.push ((-b - s) / (2 * a)) |>.push ((-b + s) / (2 * a))
      let rs := (roots.filter fun z => z0 ≤ z && z ≤ z1).qsort (· < ·)
      for k in [0:rs.size - 1] do
        let za := rs[k]!
        let zb := rs[k + 1]!
        if zb ≤ za then continue
        let zm := (za + zb) / 2
        let sg := abc.map fun (a, b, c) => (a * zm + b) * zm + c > 0
        if A.keptOn (fun idx => sg[idx]!) then total := total + (zb - za)
  return total * dx * dy

/-! ## A better estimate: Gauss–Legendre between breakpoints -/

/-- 8-point Gauss–Legendre on [-1, 1]. -/
def glNodes : Array (Float × Float) := #[
  (-0.9602898564975363, 0.1012285362903763), (-0.7966664774136267, 0.2223810344533745),
  (-0.5255324099163290, 0.3137066458778873), (-0.1834346424956498, 0.3626837833783620),
  (0.1834346424956498, 0.3626837833783620), (0.5255324099163290, 0.3137066458778873),
  (0.7966664774136267, 0.2223810344533745), (0.9602898564975363, 0.1012285362903763)]

/-- `∫_a^b f`, by Gauss–Legendre after `t = m + h sin(πu/2)` (square-root ends are smooth in
`u`), on `k` equal pieces. -/
def integrate (f : Float → Float) (a b : Float) (k : Nat) : Float := Id.run do
  if b ≤ a then return 0
  let w := (b - a) / k.toFloat
  let mut total : Float := 0
  let pi := 3.141592653589793
  for p in [0:k] do
    let lo := a + p.toFloat * w
    let m := lo + w / 2
    let h := w / 2
    for (u, wt) in glNodes do
      let t := m + h * Float.sin (pi * u / 2)
      let jac := h * (pi / 2) * Float.cos (pi * u / 2)
      total := total + wt * jac * f t
  return total

/-- Roots in `[lo, hi]` of a polynomial given by coefficients (low first), in floating point:
sign changes on 96 samples, then bisection. -/
def froots (cs : Array Float) (lo hi : Float) : Array Float := Id.run do
  let ev := fun (x : Float) => cs.foldr (fun c acc => c + x * acc) 0
  if cs.all (· == 0) then return #[]
  let n := 96
  let mut out : Array Float := #[]
  let mut xa := lo
  let mut fa := ev lo
  for k in [1:n + 1] do
    let xb := lo + (hi - lo) * k.toFloat / n.toFloat
    let fb := ev xb
    if fa == 0 then out := out.push xa
    else if (fa < 0 && fb > 0) || (fa > 0 && fb < 0) then
      let mut a := xa
      let mut b := xb
      let mut f1 := fa
      for _ in [0:60] do
        let m := (a + b) / 2
        let fm := ev m
        if (fm < 0) == (f1 < 0) then a := m; f1 := fm else b := m
      out := out.push ((a + b) / 2)
    xa := xb; fa := fb
  return out

/-- A polynomial of degree ≤ 4 through its values at five points, low coefficient first. -/
def interp5 (g : Float → Float) (c w : Float) : Array Float := Id.run do
  -- sample at c + w·{-2,-1,0,1,2}, in the variable s = (t − c)/w; return coefficients in t
  let ss : Array Float := #[-2, -1, 0, 1, 2]
  let ys := ss.map fun s => g (c + w * s)
  let mut co : Array Float := #[0, 0, 0, 0, 0]
  for k in [0:5] do
    let mut basis : Array Float := #[1]
    let mut den : Float := 1
    for m in [0:5] do
      if m != k then
        -- basis *= (s − s_m)
        let mut nb : Array Float := Array.replicate (basis.size + 1) 0
        for i in [0:basis.size] do
          nb := nb.set! (i + 1) (nb[i + 1]! + basis[i]!)
          nb := nb.set! i (nb[i]! - ss[m]! * basis[i]!)
        basis := nb
        den := den * (ss[k]! - ss[m]!)
    for i in [0:5] do co := co.set! i (co[i]! + ys[k]! / den * basis[i]!)
  -- s = (t − c)/w: expand Σ co_i ((t − c)/w)^i
  let mut out : Array Float := #[0, 0, 0, 0, 0]
  for i in [0:5] do
    -- ((t − c)/w)^i = Σ_j C(i,j) t^j (−c)^(i−j) / w^i
    let mut binom : Float := 1
    for j in [0:i + 1] do
      out := out.set! j (out[j]! + co[i]! * binom * Float.pow (-c) (i - j).toFloat / Float.pow w i.toFloat)
      binom := binom * (i - j).toFloat / (j + 1).toFloat
  return out

def sortUniq (xs : Array Float) : Array Float :=
  (xs.qsort (· < ·)).foldl (fun acc x => if acc.back?.any (fun y => (x - y).abs < 1e-12) then acc else acc.push x) #[]

/-- The length of the vertical line through `(x, y)` inside the result, between `z0` and `z1`. -/
def sliceLen (A : Arr) (fq : Array FQ) (x y z0 z1 : Float) : Float := Id.run do
  let abc := fq.map fun q => q.fz x y
  let mut roots : Array Float := #[z0, z1]
  for (a, b, c) in abc do
    if a == 0 then
      if b != 0 then roots := roots.push (-c / b)
    else
      let d := b * b - 4 * a * c
      if d > 0 then
        let s := d.sqrt
        roots := roots.push ((-b - s) / (2 * a)) |>.push ((-b + s) / (2 * a))
  let rs := (roots.filter fun z => z0 ≤ z && z ≤ z1).qsort (· < ·)
  let mut total : Float := 0
  for k in [0:rs.size - 1] do
    let za := rs[k]!
    let zb := rs[k + 1]!
    if zb ≤ za then continue
    let zm := (za + zb) / 2
    let sg := abc.map fun (a, b, c) => (a * zm + b) * zm + c > 0
    if A.keptOn (fun idx => sg[idx]!) then total := total + (zb - za)
  return total

/-- Where the section at `x` changes structure along `y`: a section conic turns vertical, or two
cross. -/
def yBreaks (fq : Array FQ) (x y0 y1 : Float) : Array Float := Id.run do
  let mut bps : Array Float := #[y0, y1]
  let cy := (y0 + y1) / 2
  let wy := (y1 - y0) / 4
  for q in fq do
    let disc := fun (y : Float) => let (a, b, c) := q.fz x y; if a == 0 then b else b * b - 4 * a * c
    bps := bps ++ froots (interp5 disc cy wy) y0 y1
    if q.zz == 0 && q.xz == 0 && q.yz == 0 && q.lz == 0 then
      bps := bps ++ froots (interp5 (fun y => (q.fz x y).2.2) cy wy) y0 y1
  for i in [0:fq.size] do
    for j in [i + 1:fq.size] do
      let cross := fun (y : Float) =>
        let (a1, b1, c1) := fq[i]!.fz x y
        let (a2, b2, c2) := fq[j]!.fz x y
        if a1 == 0 && a2 == 0 then b1 * c2 - b2 * c1
        else (a1 * c2 - a2 * c1) * (a1 * c2 - a2 * c1) - (a1 * b2 - a2 * b1) * (b1 * c2 - b2 * c1)
      bps := bps ++ froots (interp5 cross cy wy) y0 y1
  return sortUniq (bps.filter fun y => y0 ≤ y && y ≤ y1)

/-- The area of the section at `x`: the slice length integrated over `y` between breakpoints. -/
def sectionArea (A : Arr) (fq : Array FQ) (x y0 y1 z0 z1 : Float) (pieces : Nat) : Float := Id.run do
  let ys := yBreaks fq x y0 y1
  let mut total : Float := 0
  for k in [0:ys.size - 1] do
    total := total + integrate (fun y => sliceLen A fq x y z0 z1) ys[k]! ys[k + 1]! pieces
  return total

/-- The volume estimate: section areas integrated over `x` between the points where the
sections change structure (the count of `y` breakpoints changes: located on a grid of 512, then
by bisection). -/
def estimate2 (A : Arr) (lo hi : V3) (pieces : Nat) : Float × Float := Id.run do
  let fq := A.qs.map FQ.of
  let (x0, y0, z0) := (qf lo.x, qf lo.y, qf lo.z)
  let (x1, y1, z1) := (qf hi.x, qf hi.y, qf hi.z)
  let n := 512
  -- the section's structure: its y breakpoints, and the z roots between them
  let zc := fun (x y : Float) => fq.foldl (fun n q =>
    let (a, b, c) := q.fz x y
    if a == 0 then (if b != 0 then n + 1 else n) else if b * b - 4 * a * c > 0 then n + 2 else n) 0
  let count := fun (x : Float) =>
    let ys := yBreaks fq x y0 y1
    (Array.range (ys.size - 1)).foldl (fun n k => n * 7 + zc x ((ys[k]! + ys[k + 1]!) / 2)) ys.size
  let mut bps : Array Float := #[x0, x1]
  let cx := (x0 + x1) / 2
  let wx := (x1 - x0) / 4
  for q in fq do
    -- the section conic's determinant, a polynomial in x
    let dq := fun (x : Float) =>
      let m00 := q.yy
      let m01 := q.yz / 2
      let m11 := q.zz
      let m02 := (q.xy * x + q.ly) / 2
      let m12 := (q.xz * x + q.lz) / 2
      let m22 := q.xx * x * x + q.lx * x + q.c
      m00 * (m11 * m22 - m12 * m12) - m01 * (m01 * m22 - m12 * m02) + m02 * (m01 * m12 - m11 * m02)
    bps := bps ++ froots (interp5 dq cx wx) x0 x1
    -- a wall depending on x alone
    if q.yy == 0 && q.zz == 0 && q.xy == 0 && q.xz == 0 && q.yz == 0 && q.ly == 0 && q.lz == 0 then
      bps := bps ++ froots #[q.c, q.lx, q.xx] x0 x1
    let sec := fun (x : Float) => (q.xy * x + q.ly) * (q.xy * x + q.ly) - 4 * q.yy * (q.xx * x * x + q.lx * x + q.c)
    bps := bps ++ froots (interp5 sec cx wx) x0 x1
  let mut xa := x0
  let mut ca := count x0
  for k in [1:n + 1] do
    let xb := x0 + (x1 - x0) * k.toFloat / n.toFloat
    let cb := count xb
    if cb != ca then
      let mut a := xa
      let mut b := xb
      for _ in [0:40] do
        let m := (a + b) / 2
        if count m == ca then a := m else b := m
      bps := bps.push ((a + b) / 2)
    xa := xb; ca := cb
  let xs := sortUniq bps
  -- two resolutions: the finer is the estimate, their difference its error
  let mut coarse : Float := 0
  let mut fine : Float := 0
  for k in [0:xs.size - 1] do
    coarse := coarse + integrate (fun x => sectionArea A fq x y0 y1 z0 z1 1) xs[k]! xs[k + 1]! pieces
    fine := fine + integrate (fun x => sectionArea A fq x y0 y1 z0 z1 3) xs[k]! xs[k + 1]! (3 * pieces)
  return (fine, (fine - coarse).abs)

/-! ## Linear algebra over ℚ (untrusted: every result used is checked) -/

abbrev Mat := Array (Array Rat)

/-- The quadric's symmetric 4×4 matrix: `q(p) = (p, 1)ᵀ M (p, 1)`. -/
def Quadric.mat (q : Quadric) : Mat :=
  #[#[q.xx, q.xy / 2, q.xz / 2, q.lx / 2],
    #[q.xy / 2, q.yy, q.yz / 2, q.ly / 2],
    #[q.xz / 2, q.yz / 2, q.zz, q.lz / 2],
    #[q.lx / 2, q.ly / 2, q.lz / 2, q.c]]

def Mat.add (a b : Mat) : Mat := (Array.range 4).map fun i => (Array.range 4).map fun j => a[i]![j]! + b[i]![j]!
def Mat.smul (k : Rat) (a : Mat) : Mat := a.map (·.map (k * ·))

def det (m : Mat) : Rat := Id.run do
  let mut a := m
  let n := a.size
  let mut d : Rat := 1
  for k in [0:n] do
    match (Array.range n).find? (fun i => i ≥ k && a[i]![k]! != 0) with
    | none => return 0
    | some p =>
      if p != k then
        let t := a[p]!
        a := (a.set! p a[k]!).set! k t
        d := -d
      let piv := a[k]![k]!
      d := d * piv
      for i in [k + 1:n] do
        let f := a[i]![k]! / piv
        if f != 0 then
          a := a.set! i ((Array.range n).map fun j => a[i]![j]! - f * a[k]![j]!)
  return d

/-- A basis of the null space of a matrix with 4 columns, and its pivot columns. -/
def nullSpaceP (m : Mat) : Array (Array Rat) × Array Nat := Id.run do
  let n := 4
  let rows := m.size
  let mut a := m
  let mut pivCols : Array Nat := #[]
  let mut row := 0
  for col in [0:n] do
    if row ≥ rows then break
    match (Array.range rows).find? (fun i => i ≥ row && a[i]![col]! != 0) with
    | none => continue
    | some p =>
      let t := a[p]!
      a := (a.set! p a[row]!).set! row t
      let piv := a[row]![col]!
      a := a.set! row (a[row]!.map (· / piv))
      for i in [0:rows] do
        if i != row then
          let f := a[i]![col]!
          if f != 0 then a := a.set! i ((Array.range n).map fun j => a[i]![j]! - f * a[row]![j]!)
      pivCols := pivCols.push col
      row := row + 1
  let free := (Array.range n).filter fun c => !pivCols.contains c
  let basis := free.map fun fc => Id.run do
    let mut v : Array Rat := Array.replicate n 0
    v := v.set! fc 1
    for r in [0:pivCols.size] do
      v := v.set! pivCols[r]! (-(a[r]![fc]!))
    return v
  return (basis, pivCols)

def nullSpace (m : Mat) : Array (Array Rat) := (nullSpaceP m).1

/-- The submatrix on rows and columns `ix`. -/
def Mat.sub (m : Mat) (ix : Array Nat) : Mat := ix.map fun i => ix.map fun j => m[i]![j]!

/-- `M = Σ d_k ℓ_k ℓ_kᵀ` with every `d_k ≥ 0`, if `M` is positive semidefinite. -/
def ldl (m : Mat) : Option (List (Rat × Affine)) := Id.run do
  let mut a := m
  let mut out : List (Rat × Affine) := []
  for _ in [0:4] do
    match (Array.range 4).find? (fun i => a[i]![i]! != 0) with
    | none =>
      if a.all (·.all (· == 0)) then return some out.reverse else return none
    | some i =>
      let d := a[i]![i]!
      if d < 0 then return none
      let l := a[i]!.map (· / d)
      out := (d, ⟨l[0]!, l[1]!, l[2]!, l[3]!⟩) :: out
      a := (Array.range 4).map fun r => (Array.range 4).map fun c => a[r]![c]! - d * l[r]! * l[c]!
  if a.all (·.all (· == 0)) then return some out.reverse else return none

/-! ## Real roots of a rational polynomial (isolation by Descartes, untrusted; each result an
`Alg.Num` whose isolation is certified by `Iso.ok`) -/

/-- The simplest rational in `[a, b]` (`0 < a ≤ b` or any order handled by sign). -/
partial def simplest (a b : Rat) : Rat :=
  if a ≤ 0 && 0 ≤ b then 0
  else if b < 0 then -(simplest (-b) (-a))
  else
    let fa := a.floor
    if (fa : Rat) == a then a
    else if ((fa + 1 : Int) : Rat) ≤ b then ((fa + 1 : Int) : Rat)
    else (fa : Rat) + 1 / simplest (1 / (b - fa)) (1 / (a - fa))

/-- A bound on the roots' magnitude (Cauchy). -/
def rootBound (p : Poly) : Rat :=
  let t := Poly.trim p
  match t.getLast? with
  | none => 1
  | some l => 1 + t.foldl (fun m c => qmax m (qabs (c / l))) 0

/-- The square root of a rational, if it is rational. -/
def ratSqrt? (q : Rat) : Option Rat :=
  if q < 0 then none
  else
    let n := q.num.toNat
    let d := q.den
    let sn := n.sqrt
    let sd := d.sqrt
    if sn * sn == n && sd * sd == d then some ((sn : Rat) / (sd : Rat)) else none

/-- The real roots of `p` (assumed square-free), each isolated or rational. -/
def realRoots (p : Poly) (fuel : Nat := 60) : Array Num := Id.run do
  let p := Poly.trim p
  if Poly.deg p == 0 then return #[]
  -- linear and quadratic: rational roots exactly
  if Poly.deg p == 1 then return #[.rat (-(p[0]!) / p[1]!)]
  if Poly.deg p == 2 then
    let (c, b, a) := (p[0]!, p[1]!, p[2]!)
    let disc := b * b - 4 * a * c
    if disc < 0 then return #[]
    if disc == 0 then return #[.rat (-b / (2 * a))]
    match ratSqrt? disc with
    | some r => return #[.rat ((-b - r) / (2 * a)), .rat ((-b + r) / (2 * a))]
    | none => pure ()
  let B := rootBound p
  let mut out : Array Num := #[]
  let mut stack : List (Rat × Rat × Nat) := [(-B, B, 0)]
  if Poly.ev p (-B) == 0 then out := out.push (.rat (-B))
  if Poly.ev p B == 0 then out := out.push (.rat B)
  for _ in [0:4000] do
    match stack with
    | [] => break
    | (a, b, d) :: rest =>
      stack := rest
      let tr := moebius a b p
      if noPosRoot tr then continue
      let iso : Iso := ⟨p, a, b⟩
      if onePosRoot tr && iso.ok then
        -- a rational root, if a simple one is there
        let w := (b - a) / 8
        let s := simplest (a + w) (b - w)
        if a < s && s < b && Poly.ev p s == 0 then out := out.push (.rat s)
        else out := out.push (.iso iso)
        continue
      if d ≥ fuel then continue
      let m := (a + b) / 2
      if Poly.ev p m == 0 then out := out.push (.rat m)
      stack := (a, m, d + 1) :: (m, b, d + 1) :: stack
  return out

/-- A rational root inside the number's interval, if there is one (tried at the simplest
rational of the interval as it is refined). -/
def rationalOf (p : Poly) (n : Num) : Option Rat := Id.run do
  let mut u := n
  for _ in [0:80] do
    match u with
    | .rat c => return some c
    | .iso r =>
      let s := simplest r.a r.b
      if Poly.ev p s == 0 then return some s
      u := u.bisect
  return none

/-! ## Contacts -/

def evRat (p : Poly) (x : Rat) : Rat := Poly.ev p x

/-- `q` along `P + t D`, as a polynomial in `t`. -/
def alongLine (q : Quadric) (P D : V3) : Poly :=
  [q.eval P, (q.grad P).dot D, q.quad D]

/-- A rational strictly between two numbers known to be in order. -/
def between (n m : Num) : Option Rat := Id.run do
  let mut u := n
  let mut v := m
  for _ in [0:200] do
    if u.hi < v.lo then return some ((u.hi + v.lo) / 2)
    u := u.bisect; v := v.bisect
  return none

/-- Insertion sort of numbers by `Num.cmp` (equal ones dropped); `none` if a comparison fails. -/
def sortNums (ns : Array Num) : Option (Array Num) := Id.run do
  let mut out : Array Num := #[]
  for n in ns do
    let mut pos := out.size
    let mut dup := false
    for k in [0:out.size] do
      match Num.cmp 200 n out[k]! with
      | some .lt => pos := k; break
      | some .eq => dup := true; break
      | some .gt => pure ()
      | none => return none
    if !dup then out := out.insertIdxIfInBounds pos n
  return some out

structure Contact where
  i : Nat
  j : Nat
  /-- the excluded pattern -/
  excl : Excl
  /-- a point of the contact where the result is non-manifold, if any -/
  bad : Option String
  /-- something left undecided about this contact -/
  open_ : Option String
  deriving Inhabited

def fmt (v : V3) : String := s!"({qf v.x}, {qf v.y}, {qf v.z})"

/-- Is the result non-manifold where quadrics `i`, `j` touch, with the other quadrics' signs
`sg` (`excl` the pattern their contact excludes)? -/
def pinched (A : Arr) (e : Excl) (sg : Nat → Bool) : Bool :=
  let at_ := fun (si sj : Bool) => A.keptOn fun k => if k == e.i then si else if k == e.j then sj else sg k
  let a := at_ e.si (!e.sj)
  let c := at_ (!e.si) e.sj
  let b := at_ (!e.si) (!e.sj)
  (a && c && !b) || (!a && !c && b)

/-- The quadratic part of a 4×4 matrix, as a bilinear form on directions. -/
def bil (M : Mat) (u v : V3) : Rat :=
  let a := #[u.x, u.y, u.z]
  let b := #[v.x, v.y, v.z]
  (Array.range 3).foldl (fun acc i => (Array.range 3).foldl (fun acc j => acc + a[i]! * M[i]![j]! * b[j]!) acc) 0

/-- Two directions spanning the plane normal to `n`. -/
def tangentBasis (n : V3) : V3 × V3 :=
  let e : V3 := if n.x == 0 && n.y == 0 then ⟨1, 0, 0⟩ else ⟨0, 0, 1⟩
  let u := n.cross e
  (u, n.cross u)

/-- The local contact of quadrics `i`, `j` at a rational point `p` of both where
`q_i + t q_j = (x − p)ᵀ S (x − p)` (`M` its matrix), `D` the contact line's direction if the
contact is a line. `.ok (some e)`: a touch, excluding pattern `e` near `p`; `.ok none`: a saddle
(the surfaces cross through the point of tangency); `.error`: undecided. -/
def localTouch (A : Arr) (i j : Nat) (t : Rat) (M : Mat) (p : V3) (D : Option V3) :
    Except String (Option Excl) := do
  let qi := A.qs[i]!
  let n := qi.grad p
  if n.isZero || t == 0 then throw s!"a singular point at {fmt p}"
  -- the sign of S across the contact, in the tangent plane
  let sQ ← match D with
    | some d =>
      let w := n.cross d
      if bil M d d != 0 || bil M d w != 0 then
        -- not a line of contact after all: the point test
        let (u, v) := tangentBasis n
        let a := bil M u u
        let b := bil M u v
        let c := bil M v v
        let d2 := a * c - b * b
        if d2 < 0 then return none
        if d2 == 0 then throw s!"a contact of higher order at {fmt p}"
        pure (decide (a > 0))
      else
        let v := bil M w w
        if v == 0 then throw s!"a contact of higher order at {fmt p}"
        pure (decide (v > 0))
    | none =>
      let (u, v) := tangentBasis n
      let a := bil M u u
      let b := bil M u v
      let c := bil M v v
      let d2 := a * c - b * b
      if d2 < 0 then return none
      if d2 == 0 then throw s!"a contact of higher order at {fmt p}"
      pure (decide (a > 0))
  -- along n, q_i grows (s_i); q_j = −q_i / t to first order (s_j)
  let si := true
  let sj := t < 0
  -- on surface i near p, t q_j = S(x − p): the sign of q_j there
  let σj := decide (t > 0) == sQ
  if (!σj) == sj then return some ⟨i, !si, j, sj⟩ else return some ⟨i, si, j, !sj⟩

/-- At a saddle tangency of quadrics `i`, `j` (above both, below both, and the two between
patterns each over two opposite sectors), is the result non-manifold? It is unless the kept
regions are one side of a surface, of the upper or of the lower envelope, or nothing or all. -/
def saddleBad (A : Arr) (i j : Nat) (t : Rat) (sg : Nat → Bool) : Bool :=
  let si := true
  let sj := decide (t < 0)
  let at_ := fun (a b : Bool) => A.keptOn fun k => if k == i then a else if k == j then b else sg k
  let kA := at_ si sj
  let kB := at_ (!si) (!sj)
  let kX := at_ si (!sj)
  let kY := at_ (!si) sj
  ((kX || kY) && !kA && !kB) || (kA && kB && !(kX && kY))

/-- The contact of quadrics `i` and `j` through the pencil member `A_i + t A_j`: a global exclusion
certificate when the member is semidefinite (checked by `Quad.pairCheck`), and, at each rational
point of the contact locus, the local touch and whether the result is non-manifold there. -/
def contactAt (A : Arr) (i j : Nat) (t : Rat) : Option Contact := Id.run do
  let qi := A.qs[i]!
  let qj := A.qs[j]!
  let M := (Quadric.mat qi).add ((Quadric.mat qj).smul t)
  -- a global exclusion, if the member is semidefinite
  let glob : Option Excl := Id.run do
    let (neg, terms) ← match ldl M with
      | some ts => (false, ts)
      | none => match ldl (M.smul (-1)) with
        | some ts => (true, ts)
        | none => return none
    let a := if neg then qi.neg else qi
    let (b, mu) := if t ≥ 0 then (if neg then qj.neg else qj, t) else (if neg then qj else qj.neg, -t)
    if !pairCheck a b 1 mu terms then return none
    return some ⟨i, neg, j, (neg != decide (t < 0))⟩
  let excl := glob.getD ⟨i, true, j, true⟩
  let ns := nullSpace M
  let others := (Array.range A.qs.size).filter fun k => k != i && k != j
  -- judge a rational contact point
  let judge : V3 → Option V3 → Option String × Option String := fun p D =>
    let sgs := others.map fun k => (k, qsgn ((A.qs[k]!).eval p))
    if sgs.any (fun (_, s) => s == 0) then
      (none, if D.isSome then none else some s!"surfaces {i}, {j} touch at {fmt p} on an edge of a third")
    else if (qi.grad p).isZero || (qj.grad p).isZero then (none, none)  -- an apex: `apexCheck`
    else
      let sg : Nat → Bool := fun k => match sgs.find? (·.1 == k) with
        | some (_, s) => s > 0
        | _ => false
      match localTouch A i j t M p D with
      | .error e => (none, some e)
      | .ok (some e) =>
        if pinched A e sg then (some s!"surfaces {i}, {j} touch at {fmt p}", none) else (none, none)
      | .ok none =>
        if saddleBad A i j t sg then (some s!"surfaces {i}, {j} touch and cross at {fmt p}", none)
        else (none, none)
  let mk := fun (bad : Option String) (op : Option String) =>
    (if glob.isNone && bad.isNone && op.isNone then none else some (⟨i, j, excl, bad, op⟩ : Contact))
  match ns.size with
  | 1 =>
    let v := ns[0]!
    if v[3]! == 0 then return mk none none
    let p : V3 := ⟨v[0]! / v[3]!, v[1]! / v[3]!, v[2]! / v[3]!⟩
    if qi.eval p != 0 then return mk none none
    let (bad, op) := judge p none
    return mk bad op
  | 2 =>
    let (v1, v2) := if ns[0]![3]! != 0 then (ns[0]!, ns[1]!) else (ns[1]!, ns[0]!)
    if v1[3]! == 0 then return mk none none
    let P : V3 := ⟨v1[0]! / v1[3]!, v1[1]! / v1[3]!, v1[2]! / v1[3]!⟩
    let w := v2[3]! / v1[3]!
    let D : V3 := ⟨v2[0]! - w * v1[0]!, v2[1]! - w * v1[1]!, v2[2]! - w * v1[2]!⟩
    let gi := alongLine qi P D
    let ls := others.map fun k => (k, alongLine A.qs[k]! P D)
    let rootsOthers := ls.foldl (fun acc (_, g) => acc ++ realRoots g) #[]
    if Poly.isZero gi then
      -- a contact line: each interval between the other quadrics' roots
      match sortNums rootsOthers with
      | none => return mk none (some "the contact line's subdivision undecided")
      | some rs =>
        let mut samples : Array Rat := #[]
        if rs.isEmpty then samples := #[0]
        else
          samples := samples.push (rs[0]!.lo - 1)
          for k in [0:rs.size - 1] do
            match between rs[k]! rs[k + 1]! with
            | some s => samples := samples.push s
            | none => pure ()
          samples := samples.push (rs[rs.size - 1]!.hi + 1)
        let mut opn : Option String := none
        for s in samples do
          let (bad, op) := judge (P + s * D) (some D)
          if bad.isSome then return mk bad none
          if op.isSome then opn := op
        return mk none opn
    else
      -- isolated contact points: q_i's roots on the line
      let mut opn : Option String := none
      for α in realRoots gi do
        match α with
        | .rat c =>
          let (bad, op) := judge (P + c * D) none
          if bad.isSome then return mk bad none
          if op.isSome then opn := op
        | .iso r =>
          -- an algebraic contact point: decided only with a global certificate
          if glob.isNone then opn := some s!"an algebraic contact point near {fmt (P + ((r.a + r.b) / 2) * D)}"
          else
            let sgOf : Nat → Option Int := fun k =>
              match ls.find? (·.1 == k) with
              | some (_, g) => α.sign g 200
              | none => none
            let sgs := others.map fun k => (k, sgOf k)
            if sgs.all (fun (_, s) => s.isSome && s != some 0) then
              let sg : Nat → Bool := fun k => match sgs.find? (·.1 == k) with
                | some (_, some s) => s > 0
                | _ => false
              if pinched A excl sg then return mk (some s!"surfaces {i}, {j} touch near {fmt (P + ((r.a + r.b) / 2) * D)}") none
      return mk none opn
  | 3 =>
    -- a contact along a conic: the plane of the null space, met by q_i; judged at its rational
    -- points on lines through a point of the plane
    let pts := ns.filter fun v => v[3]! != 0
    if pts.isEmpty then return mk none none
    let v0 := pts[0]!
    let P : V3 := ⟨v0[0]! / v0[3]!, v0[1]! / v0[3]!, v0[2]! / v0[3]!⟩
    let dirs := (ns.filter fun v => v != v0).map fun v =>
      let w := v[3]! / v0[3]!
      (⟨v[0]! - w * v0[0]!, v[1]! - w * v0[1]!, v[2]! - w * v0[2]!⟩ : V3)
    if dirs.size < 2 then return mk none none
    let (U, V) := (dirs[0]!, dirs[1]!)
    let nrm := U.cross V
    let mut found := false
    for D0 in #[U, V, U + V, U - V] do
      let g := alongLine qi P D0
      for α in realRoots g do
        match α with
        | .rat c =>
          let p := P + c * D0
          found := true
          let (bad, op) := judge p (some ((qi.grad p).cross nrm))
          if bad.isSome then return mk bad none
          if op.isSome then return mk none op
        | .iso _ => pure ()
    if found then return mk none none
    return mk none (some s!"surfaces {i}, {j}: a contact along a conic with no rational point found")
  | _ => return mk none none

/-! ## Singular points: cone apexes -/

/-- Directions spread over the sphere (Fibonacci), as rationals. -/
def sphereDirs (n : Nat) : Array (V3 × Float × Float × Float) := Id.run do
  let mut out := #[]
  let ga := 2.399963229728653
  for k in [0:n] do
    let z := 1 - 2 * (k.toFloat + 0.5) / n.toFloat
    let r := (1 - z * z).sqrt
    let th := ga * k.toFloat
    let (x, y) := (r * Float.cos th, r * Float.sin th)
    let toQ := fun (f : Float) => (((f * 1048576).round.toInt64.toInt : Int) : Rat) / 1048576
    out := out.push (⟨toQ x, toQ y, toQ z⟩, x, y, z)
  return out

/-- Components of the marked directions, neighbours within `ang` radians. -/
def components (pts : Array (Float × Float × Float)) (ang : Float) : Nat := Id.run do
  let n := pts.size
  let c := Float.cos ang
  let mut comp : Array Nat := Array.replicate n n
  let mut count := 0
  for s in [0:n] do
    if comp[s]! != n then continue
    comp := comp.set! s count
    let mut stack := [s]
    for _ in [0:n * 4] do
      match stack with
      | [] => break
      | a :: rest =>
        stack := rest
        let (ax, ay, az) := pts[a]!
        for b in [0:n] do
          if comp[b]! == n then
            let (bx, by_, bz) := pts[b]!
            if ax * bx + ay * by_ + az * bz ≥ c then
              comp := comp.set! b count
              stack := b :: stack
    count := count + 1
  return count

/-- The result at a cone's apex lying on other surfaces: its tangent cone there (first order:
the cone's own quadratic form, the other surfaces' gradients), sampled over the sphere of
directions with exact signs; non-manifold if the kept or the dropped directions fall apart.
(Probed: the directions are samples, the signs at each exact.) -/
def apexCheck (A : Arr) (solids : Array QSolid) : Array String := Id.run do
  let mut out : Array String := #[]
  let dirs := sphereDirs 3000
  for s in solids do
    for f in s.faces do
      if f.kind != .cone || f.s == 0 then continue
      let apex := f.o - ((f.r * f.c / f.s) : Rat) * f.z
      let through := (Array.range A.qs.size).filter fun k => (A.qs[k]!).eval apex == 0
      -- only the cone itself (and its nappe) through the apex: a plain apex
      let smooth := through.filter fun k => !((A.qs[k]!).grad apex).isZero
      if smooth.size ≤ 1 then continue
      let sgAt : Nat → V3 → Int := fun k d =>
        let q := A.qs[k]!
        if through.contains k then
          let g := q.grad apex
          if g.isZero then qsgn (q.quad d) else qsgn (g.dot d)
        else qsgn (q.eval apex)
      let mut kept : Array (Float × Float × Float) := #[]
      let mut dropped : Array (Float × Float × Float) := #[]
      for (d, x, y, z) in dirs do
        let sg := (Array.range A.qs.size).map fun k => sgAt k d
        if sg.any (· == 0) then continue
        if A.keptOn (fun k => sg[k]! > 0) then kept := kept.push (x, y, z) else dropped := dropped.push (x, y, z)
      if kept.isEmpty || dropped.isEmpty then continue
      let ang := 0.12
      let ck := components kept ang
      let cd := components dropped ang
      if ck ≥ 2 || cd ≥ 2 then
        out := out.push s!"the apex {fmt apex} of face {f.solid}.{f.index}: the result's tangent cone there falls apart ({ck} kept, {cd} dropped pieces; probed)"
  return out

/-- The pencil's determinant `det(A_i + t A_j)`, a polynomial of degree ≤ 4 in `t`, by
interpolation at five points. Where the two quadrics share a null direction (a cylinder's axis,
a plane's), the determinant is that of the pencil on the complement. -/
def pencilDet (qi qj : Quadric) : Poly := Id.run do
  let ai := Quadric.mat qi
  let aj := Quadric.mat qj
  let (common, piv) := nullSpaceP (ai ++ aj)
  let ix := if common.isEmpty then #[0, 1, 2, 3] else piv
  let ts : Array Rat := #[0, 1, -1, 2, -2]
  let ys := ts.map fun t => det ((ai.add (aj.smul t)).sub ix)
  -- Lagrange
  let mut out : Poly := []
  for k in [0:5] do
    let mut basis : Poly := [1]
    let mut den : Rat := 1
    for m in [0:5] do
      if m != k then
        basis := pmul basis [-ts[m]!, 1]
        den := den * (ts[k]! - ts[m]!)
    out := padd out (pscale (ys[k]! / den) basis)
  return Poly.trim out

/-- Real roots of `p` in floating point (sign changes over all of ℝ through `t = tan θ`, then
bisection): candidates only. -/
def floatRoots (p : Poly) : Array Float := Id.run do
  let cs := (Poly.trim p).map qf
  let ev := fun (x : Float) => cs.foldr (fun c acc => c + x * acc) 0
  let n := 4000
  let mut out : Array Float := #[]
  let pi := 3.141592653589793
  let at_ := fun (k : Nat) => Float.tan (-pi / 2 + pi * (k.toFloat + 0.5) / n.toFloat)
  let mut xa := at_ 0
  let mut fa := ev xa
  for k in [1:n] do
    let xb := at_ k
    let fb := ev xb
    if fa == 0 then out := out.push xa
    else if (fa < 0 && fb > 0) || (fa > 0 && fb < 0) then
      let mut lo := xa
      let mut hi := xb
      let mut flo := fa
      for _ in [0:200] do
        let m := (lo + hi) / 2
        let fm := ev m
        if (fm < 0) == (flo < 0) then lo := m; flo := fm else hi := m
      out := out.push ((lo + hi) / 2)
    xa := xb; fa := fb
  return out

/-- Continued-fraction convergents of a float: rational candidates near it. -/
def convergents (x : Float) : Array Rat := Id.run do
  let mut out : Array Rat := #[]
  let mut y := x
  let mut h0 : Int := 1
  let mut h1 : Int := 0
  let mut k0 : Int := 0
  let mut k1 : Int := 1
  for _ in [0:30] do
    if y.isNaN || y.isInf || y.abs > 1e15 then break
    let a : Int := (y.floor.toInt64).toInt
    let h := a * h0 + h1
    let k := a * k0 + k1
    h1 := h0; h0 := h; k1 := k0; k0 := k
    if k != 0 then out := out.push ((h : Rat) / (k : Rat))
    let frac := y - y.floor
    if frac < 1e-15 then break
    y := 1 / frac
  return out

/-- A float 4×4 solve by Gaussian elimination with partial pivoting (`none` if singular). -/
def fsolve (m : Array (Array Float)) (b : Array Float) : Option (Array Float) := Id.run do
  let mut a := (Array.range 4).map fun i => m[i]!.push b[i]!
  for k in [0:4] do
    let mut p := k
    for r in [k + 1:4] do
      if a[r]![k]!.abs > a[p]![k]!.abs then p := r
    if a[p]![k]! == 0 then return none
    let t := a[p]!
    a := (a.set! p a[k]!).set! k t
    for r in [k + 1:4] do
      let f := a[r]![k]! / a[k]![k]!
      a := a.set! r ((Array.range 5).map fun c => a[r]![c]! - f * a[k]![c]!)
  let mut x : Array Float := Array.replicate 4 0
  for kk in [0:4] do
    let k := 3 - kk
    let mut sacc := a[k]![4]!
    for c in [k + 1:4] do sacc := sacc - a[k]![c]! * x[c]!
    x := x.set! k (sacc / a[k]![k]!)
  return some x

/-- Two quadrics that come within `l` of touching without an exact contact: a near-double root
`t` of the pencil's determinant (a local minimum of its magnitude, tiny against its scale), the
point the near-singular member's null vector gives, and that point within `l` of both surfaces
and inside both faces' boxes (grown by `l`). Floating point: it only flags. -/
def nearTouch (A : Arr) (boxes : Array Box) (l : Float) (i j : Nat) : Option String := Id.run do
  let qi := A.qs[i]!
  let qj := A.qs[j]!
  let fi := (Quadric.mat qi).map (·.map qf)
  let fj := (Quadric.mat qj).map (·.map qf)
  let member := fun (t : Float) => (Array.range 4).map fun r => (Array.range 4).map fun c => fi[r]![c]! + t * fj[r]![c]!
  let fdet := fun (m : Array (Array Float)) =>
    let sub := fun (r c : Nat) => m[r]![c]!
    -- Laplace along the first row of 3×3 minors
    let d3 := fun (a b c d e f g h i : Float) => a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
    sub 0 0 * d3 (sub 1 1) (sub 1 2) (sub 1 3) (sub 2 1) (sub 2 2) (sub 2 3) (sub 3 1) (sub 3 2) (sub 3 3)
    - sub 0 1 * d3 (sub 1 0) (sub 1 2) (sub 1 3) (sub 2 0) (sub 2 2) (sub 2 3) (sub 3 0) (sub 3 2) (sub 3 3)
    + sub 0 2 * d3 (sub 1 0) (sub 1 1) (sub 1 3) (sub 2 0) (sub 2 1) (sub 2 3) (sub 3 0) (sub 3 1) (sub 3 3)
    - sub 0 3 * d3 (sub 1 0) (sub 1 1) (sub 1 2) (sub 2 0) (sub 2 1) (sub 2 2) (sub 3 0) (sub 3 1) (sub 3 2)
  let norm := fun (m : Array (Array Float)) => m.foldl (fun a r => r.foldl (fun a v => a + v.abs) a) 0
  let rel := fun (t : Float) => let m := member t; (fdet m).abs / (Float.pow (norm m) 4 + 1e-300)
  let n := 3000
  let pi := 3.141592653589793
  let at_ := fun (k : Nat) => Float.tan (-pi / 2 + pi * (k.toFloat + 0.5) / n.toFloat)
  let fb := boxes.map fun b => ((qf b.lo.x, qf b.lo.y, qf b.lo.z), (qf b.hi.x, qf b.hi.y, qf b.hi.z))
  for k in [1:n - 1] do
    let (t0, t1, t2) := (at_ (k - 1), at_ k, at_ (k + 1))
    let (r0, r1, r2) := (rel t0, rel t1, rel t2)
    if !(r1 ≤ r0 && r1 ≤ r2) then continue
    -- golden-section refinement of the minimum
    let mut a := t0
    let mut b := t2
    for _ in [0:80] do
      let m1 := a + (b - a) * 0.381966
      let m2 := a + (b - a) * 0.618034
      if rel m1 < rel m2 then b := m2 else a := m1
    let t := (a + b) / 2
    if rel t > 1e-13 then continue
    -- the near-null vector, by one step of inverse iteration
    let m := member t
    let some v := fsolve m #[0.31, -0.57, 0.73, 0.19] | continue
    if v[3]!.abs < 1e-300 then continue
    let p : V3 := ⟨0, 0, 0⟩
    let (px, py, pz) := (v[0]! / v[3]!, v[1]! / v[3]!, v[2]! / v[3]!)
    let _ := p
    -- the distance to each surface, to first order; a singular point (an apex) is no touch
    let dist := fun (q : Quadric) =>
      let f := FQ.of q
      let val := f.xx * px * px + f.yy * py * py + f.zz * pz * pz + f.xy * px * py + f.xz * px * pz +
        f.yz * py * pz + f.lx * px + f.ly * py + f.lz * pz + f.c
      let gx := 2 * f.xx * px + f.xy * py + f.xz * pz + f.lx
      let gy := 2 * f.yy * py + f.xy * px + f.yz * pz + f.ly
      let gz := 2 * f.zz * pz + f.xz * px + f.yz * py + f.lz
      let g := (gx * gx + gy * gy + gz * gz).sqrt
      let scale := (f.lx.abs + f.ly.abs + f.lz.abs + (f.xx.abs + f.yy.abs + f.zz.abs) * (px.abs + py.abs + pz.abs + 1))
      if g < 1e-6 * scale then 1e300 else val.abs / g
    if dist qi > l || dist qj > l then continue
    -- inside the hull of all the faces on either carrier
    let near := fb.any fun ((lx, ly, lz), (hx, hy, hz)) =>
      lx - l ≤ px && px ≤ hx + l && ly - l ≤ py && py ≤ hy + l && lz - l ≤ pz && pz ≤ hz + l
    if near then
      return some s!"surfaces {i}, {j} come within LINEAR of touching near ({px}, {py}, {pz})"
  return none

/-- Every contact between two of the arrangement's quadrics: for each rational root `t` of the
pencil's determinant (proposed in floating point, kept only if exact), the member `A_i + t A_j`
tried for a certificate. -/
def contacts (A : Arr) : Array Contact := Id.run do
  let mut out : Array Contact := #[]
  for i in [0:A.qs.size] do
    for j in [i + 1:A.qs.size] do
      let qi := A.qs[i]!
      let qj := A.qs[j]!
      -- two planes never touch without being one
      if qi.qnorm == 0 && qj.qnorm == 0 then continue
      let d := pencilDet qi qj
      if Poly.isZero d then continue
      let sq := Poly.sqfree d
      let mut ts : Array Rat := #[]
      for r in floatRoots sq do
        for t in convergents r do
          if Poly.ev d t == 0 && !ts.contains t then ts := ts.push t
      for t in ts do
        if t == 0 then continue
        match contactAt A i j t with
        | some c => out := out.push c
        | none => pure ()
  return out

end ForgeExact.QRef
