/-!
# Exact rational geometry for the flat-face reference

Points, planes, convex polygons and the solids the reference decides, over `Rat` (GMP-backed in the
compiled runtime). Every kernel f64 is a dyadic rational, so the input is exact as given.
-/

namespace ForgeExact.Ref

instance : Hashable Rat := ⟨fun q => mixHash (hash q.num) (hash q.den)⟩

def sgn (q : Rat) : Int := if q > 0 then 1 else if q < 0 then -1 else 0
def qabs (q : Rat) : Rat := if q < 0 then -q else q
def qmin (a b : Rat) : Rat := if a ≤ b then a else b
def qmax (a b : Rat) : Rat := if a ≤ b then b else a

structure V3 where
  x : Rat
  y : Rat
  z : Rat
  deriving BEq, Inhabited, Repr, Hashable, DecidableEq

namespace V3
def add (a b : V3) : V3 := ⟨a.x + b.x, a.y + b.y, a.z + b.z⟩
def sub (a b : V3) : V3 := ⟨a.x - b.x, a.y - b.y, a.z - b.z⟩
def smul (k : Rat) (a : V3) : V3 := ⟨k * a.x, k * a.y, k * a.z⟩
def dot (a b : V3) : Rat := a.x * b.x + a.y * b.y + a.z * b.z
def cross (a b : V3) : V3 :=
  ⟨a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x⟩
def isZero (a : V3) : Bool := a.x == 0 && a.y == 0 && a.z == 0
def norm2 (a : V3) : Rat := a.dot a
/-- Lexicographic order, for keys. -/
def lt (a b : V3) : Bool :=
  a.x < b.x || (a.x == b.x && (a.y < b.y || (a.y == b.y && a.z < b.z)))
def maxAbs (a : V3) : Rat := qmax (qabs a.x) (qmax (qabs a.y) (qabs a.z))
end V3

instance : Add V3 := ⟨V3.add⟩
instance : Sub V3 := ⟨V3.sub⟩
instance : HMul Rat V3 V3 := ⟨V3.smul⟩

/-- A rational as a float, for reports only (never for a decision). -/
def qf (q : Rat) : Float :=
  let n := Float.ofInt q.num
  let d := Float.ofNat q.den
  n / d

/-- A plane `n · x = c`. -/
structure Plane where
  n : V3
  c : Rat
  deriving BEq, Inhabited, Hashable, DecidableEq

namespace Plane
def eval (p : Plane) (x : V3) : Rat := p.n.dot x - p.c
/-- The plane scaled so the first nonzero coefficient of its normal is 1: equal point sets give
equal canonical planes. -/
def canon (p : Plane) : Plane :=
  let s := if p.n.x != 0 then p.n.x else if p.n.y != 0 then p.n.y else p.n.z
  ⟨(1 / s) * p.n, p.c / s⟩
/-- `eval x` within `l` of zero in distance: `|n·x − c| ≤ l |n|`, compared squared. -/
def within (p : Plane) (x : V3) (l : Rat) : Bool :=
  let e := p.eval x
  e * e ≤ l * l * p.n.norm2
end Plane

/-- The point on three planes, if they meet in one. -/
def meet3 (a b c : Plane) : Option V3 :=
  let d := a.n.dot (b.n.cross c.n)
  if d == 0 then none
  else some ((1 / d) * (a.c * (b.n.cross c.n) + b.c * (c.n.cross a.n) + c.c * (a.n.cross b.n)))

/-- A box. -/
structure Box where
  lo : V3
  hi : V3
  deriving Inhabited, DecidableEq

def Box.ofPoints (ps : Array V3) : Box :=
  ps.foldl (fun b p => ⟨⟨qmin b.lo.x p.x, qmin b.lo.y p.y, qmin b.lo.z p.z⟩,
    ⟨qmax b.hi.x p.x, qmax b.hi.y p.y, qmax b.hi.z p.z⟩⟩) ⟨ps[0]!, ps[0]!⟩

def Box.grow (b : Box) (g : Rat) : Box := ⟨b.lo - ⟨g, g, g⟩, b.hi + ⟨g, g, g⟩⟩

def Box.contains (b : Box) (p : V3) : Bool :=
  b.lo.x ≤ p.x && p.x ≤ b.hi.x && b.lo.y ≤ p.y && p.y ≤ b.hi.y && b.lo.z ≤ p.z && p.z ≤ b.hi.z

/-! ## Convex polygons -/

/-- Newell's normal: twice the area vector. Zero iff the polygon has no area. -/
def newell (poly : Array V3) : V3 := Id.run do
  let mut s : V3 := ⟨0, 0, 0⟩
  for i in [0:poly.size] do
    s := s + (poly[i]!).cross poly[(i + 1) % poly.size]!
  return s

def proper (poly : Array V3) : Bool := poly.size ≥ 3 && !(newell poly).isZero

/-- Splits a convex polygon by a plane: the parts with `eval ≥ 0` and `eval ≤ 0`. A vertex on the
plane goes to both, and a crossing point is inserted in both, so neighbouring cells keep sharing
their edges' vertices. -/
def split (poly : Array V3) (p : Plane) : Array V3 × Array V3 := Id.run do
  let mut pos : Array V3 := #[]
  let mut neg : Array V3 := #[]
  let n := poly.size
  for i in [0:n] do
    let a := poly[i]!
    let b := poly[(i + 1) % n]!
    let fa := p.eval a
    let fb := p.eval b
    if fa ≥ 0 then pos := pos.push a
    if fa ≤ 0 then neg := neg.push a
    if (fa > 0 && fb < 0) || (fa < 0 && fb > 0) then
      let q := a + (fa / (fa - fb)) * (b - a)
      pos := pos.push q
      neg := neg.push q
  return (pos, neg)

def centroid (poly : Array V3) : V3 :=
  (1 / (poly.size : Rat)) * poly.foldl (· + ·) ⟨0, 0, 0⟩

/-! ## Solids: planar B-reps, membership by ray parity -/

/-- A planar face: its carrier and its loops (outer and holes; orientation is not used). -/
structure Face where
  plane : Plane
  loops : Array (Array V3)
  deriving Inhabited

/-- A solid of the history: its faces and how it combines with the part before it. -/
structure Solid where
  op : String
  faces : Array Face
  deriving Inhabited

/-- Does the horizontal ray from `q` (perturbed to `q + (0, ε, ε²)`) toward +y cross segment
`ab`, in (y, z) coordinates? Exact, and well defined for every `q`: the perturbation decides every
tie the same way (simulation of simplicity on the ray's origin). -/
def crossesYZ (q a b : V3) : Bool :=
  let aa := decide (a.z > q.z)
  let ab := decide (b.z > q.z)
  if aa == ab then false
  else
    let dz := b.z - a.z
    let d0 := (b.y - a.y) * (q.z - a.z) - (q.y - a.y) * dz
    let s := if d0 != 0 then sgn d0 else sgn (-dz)
    s * sgn dz > 0

/-- Does the ray from `p` along +x (origin perturbed as above) pierce the face? `p` is off the
face's plane. -/
def Face.pierced (f : Face) (p : V3) : Bool :=
  let nx := f.plane.n.x
  if nx == 0 then false
  else if sgn (f.plane.c - f.plane.n.dot p) * sgn nx ≤ 0 then false
  else
    let cnt := f.loops.foldl (fun acc lp => Id.run do
      let mut k := acc
      for i in [0:lp.size] do
        if crossesYZ p lp[i]! lp[(i + 1) % lp.size]! then k := k + 1
      return k) 0
    cnt % 2 == 1

/-- Membership of a point off every carrier: an odd number of faces pierced. This is the point
set a closed planar B-rep bounds. -/
def Solid.contains (s : Solid) (p : V3) : Bool :=
  (s.faces.foldl (fun k f => if f.pierced p then k + 1 else k) 0) % 2 == 1

/-- The history as one pointwise boolean (`ForgeExact.combine`): new and add unite, cut takes
away, keep intersects. -/
def keptAt (solids : Array Solid) (p : V3) : Bool :=
  solids.foldl (fun r s =>
    if s.op == "cut" then r && !s.contains p
    else if s.op == "keep" then r && s.contains p
    else r || s.contains p) false

end ForgeExact.Ref
