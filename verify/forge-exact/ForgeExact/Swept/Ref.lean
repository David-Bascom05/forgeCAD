import Lean.Data.Json
import ForgeExact.Ref.Input
import ForgeExact.Swept.Prism

/-!
# The swept-solid reference: a kernel tool against its feature's definition

One JSON object per line (numbers exact, `[m, e]` = m·2^e, as ForgeExact/Ref/Input.lean):

```
{"linear": [m,e], "kind": "prism",
 "frame": {"o": [x,y,z], "x": [x,y,z], "y": [x,y,z], "v": [x,y,z], "n": [x,y,z] (optional)},
 "lo": [m,e], "hi": [m,e],
 "loops": [[[x,y], ...], ...],
 "kernel": {"op": .., "verts": .., "faces": ..} | "kernel_refused": "why"}
```

With `n` (an extrude along a direction), the travel is `v / (v · n)`: it rises 1 along the
sketch's normal `n` per unit of the range. The definition is the L1 set of ForgeExact/Swept/Prism.lean, `Prism loops F lo hi`, decided at a
point by `prismMem` (`prism_iff`). The reference does not build it from the kernel's B-rep.

The answer compares two point sets on the arrangement of every carrier plane: the definition's
(its caps and one wall per profile edge, exact) and the kernel tool's, each kernel carrier first
snapped to its tolerance class's representative under the boolean's rule (ForgeExact/Snap.lean,
§6.1), the definition's carriers named first so they represent their classes. Each cell of the
arrangement on each plane is classified just above and below by `kernel.contains p ≠ prismMem p`:
a cell where that changes bounds the symmetric difference.

* `agree`: the symmetric difference is empty: the kernel's tool is the definition exactly, up to
  carriers within LINEAR of the definition's (the boolean's own snap).
* `differ`: it is not; the answer gives its volume and where.
* `refused_ok` / `refused_valid`: the kernel refused, and the definition is outside / inside the
  domain (simple loops, holes inside and apart, a positive range, an independent frame).
* `selfcheck`: the definition's own faces (built here from the loops) bound a different set from
  `prismMem`: a fault of this reference, never the kernel's.
-/

namespace ForgeExact.Swept.Ref

open Lean ForgeExact.Ref ForgeExact.Swept

def v2 (j : Json) : Except String (Rat × Rat) := do
  let a ← j.getArr?
  if a.size != 2 then throw "a profile point is [x, y]"
  return (← dyadic a[0]!, ← dyadic a[1]!)

/-- The definition's own B-rep, built from the loops: two caps and a wall per edge. -/
def defRaw (F : Frame3) (lo hi : Rat) (loops : List (List (Rat × Rat))) : Except String Raw := do
  let mut verts : Array V3 := #[]
  let mut planes : Array Plane := #[]
  let mut floops : Array (Array (Array Nat)) := #[]
  let capN := F.ex.cross F.ey
  if capN.isZero then throw "the frame's axes are parallel"
  let mut bottom : Array (Array Nat) := #[]
  let mut top : Array (Array Nat) := #[]
  let mut walls : Array (Plane × Array Nat) := #[]
  for l in loops do
    let base := verts.size
    let n := l.length
    for (x, y) in l do
      verts := verts.push (F.pt x y lo) |>.push (F.pt x y hi)
    bottom := bottom.push ((Array.range n).map fun i => base + 2 * i)
    top := top.push ((Array.range n).map fun i => base + 2 * i + 1)
    for i in [0:n] do
      let j := (i + 1) % n
      let A := verts[base + 2 * i]!
      let B := verts[base + 2 * j]!
      let wn := (B - A).cross F.v
      if wn.isZero then throw "a profile edge has no length, or runs along the travel"
      walls := walls.push (⟨wn, wn.dot A⟩,
        #[base + 2 * i, base + 2 * j, base + 2 * j + 1, base + 2 * i + 1])
  planes := planes.push ⟨capN, capN.dot (F.pt 0 0 lo)⟩ |>.push ⟨capN, capN.dot (F.pt 0 0 hi)⟩
  floops := floops.push bottom |>.push top
  for (p, q) in walls do
    planes := planes.push p
    floops := floops.push #[q]
  return ⟨"new", verts, planes, floops⟩

/-! ## The domain, decided exactly -/

def orient (a b c : Rat × Rat) : Int := sgn ((b.1 - a.1) * (c.2 - a.2) - (b.2 - a.2) * (c.1 - a.1))

/-- Do the closed segments `ab` and `cd` meet? -/
def segsMeet (a b c d : Rat × Rat) : Bool :=
  let d1 := orient a b c
  let d2 := orient a b d
  let d3 := orient c d a
  let d4 := orient c d b
  let within (p q r : Rat × Rat) : Bool :=
    qmin p.1 q.1 ≤ r.1 && r.1 ≤ qmax p.1 q.1 && qmin p.2 q.2 ≤ r.2 && r.2 ≤ qmax p.2 q.2
  (d1 * d2 < 0 && d3 * d4 < 0) ||
    (d1 == 0 && within a b c) || (d2 == 0 && within a b d) ||
    (d3 == 0 && within c d a) || (d4 == 0 && within c d b)

/-- The loops are simple and pairwise apart: no two edges meet, except neighbours of one loop at
their shared end (and there only there). -/
def loopsSimple (loops : List (List (Rat × Rat))) : Bool := Id.run do
  let es : Array (Nat × Nat × (Rat × Rat) × (Rat × Rat)) := Id.run do
    let mut out := #[]
    for li in [0:loops.length] do
      let l := loops[li]!
      for k in [0:(edges l).length] do
        let e := (edges l)[k]!
        out := out.push (li, k, e.1, e.2)
    return out
  for i in [0:es.size] do
    for j in [i + 1:es.size] do
      let (li, ki, a, b) := es[i]!
      let (lj, kj, c, d) := es[j]!
      let n := (loops[li]!).length
      let adjacent := li == lj && (kj == ki + 1 || (ki == 0 && kj + 1 == n))
      if adjacent then
        -- neighbours share one end; they must not overlap past it (no fold back)
        let (shared, other1, other2) := if kj == ki + 1 then (b, a, d) else (a, b, c)
        if orient other1 shared other2 == 0 &&
            ((other1.1 - shared.1) * (other2.1 - shared.1) + (other1.2 - shared.2) * (other2.2 - shared.2)) > 0 then
          return false
      else if segsMeet a b c d then return false
  return true

/-- The definition is in the extrude's domain: an independent frame, a range longer than
`minLength` (the kernel's MIN_LENGTH, LINEAR: a stated precondition, not a validity condition),
simple loops apart from each other, holes inside the outer loop. -/
def inDomain (F : Frame3) (lo hi minLength : Rat) (loops : List (List (Rat × Rat))) : Bool :=
  F.det != 0 && hi - lo > minLength && !loops.isEmpty && loops.all (·.length ≥ 3) && loopsSimple loops &&
    -- every hole inside the outer loop: its first point is in the outer loop's region
    (loops.drop 1).all fun h => inRegion [loops.head!] h.head!

/-! ## Comparing -/

/-- The boundary cells of the set where `mem` holds (`ForgeExact.Ref.boundary` with the history
replaced by any membership). -/
def boundaryOf (mem : V3 → Bool) (planes : Array Plane) (boxes : Array Box) : Array BCell := Id.run do
  let mut out : Array BCell := #[]
  for i in [0:planes.size] do
    let P := planes[i]!
    for c in cellsOn boxes[i]! planes i do
      let m := centroid c
      let mut δ : Rat := 1
      for j in [0:planes.size] do
        if j == i then continue
        let Q := planes[j]!
        let d := qabs (Q.eval m) / (2 * (qabs (P.n.dot Q.n) + 1))
        if d < δ then δ := d
      let above := mem (m + δ * P.n)
      let below := mem (m - δ * P.n)
      if above != below then
        let o : Int := if below then 1 else -1
        let nrm := newell c
        let outward := (o : Rat) * P.n
        let poly := if nrm.dot outward > 0 then c else c.reverse
        out := out.push ⟨i, o, poly⟩
  return out

/-- Every distinct carrier of the solids, with the box of the points on it grown by 1. -/
def arrangementOf (solids : Array Solid) : Array Plane × Array Box := Id.run do
  let mut planes : Array Plane := #[]
  let mut onPlane : Array (Array V3) := #[]
  for s in solids do
    for f in s.faces do
      let c := f.plane.canon
      let pts := f.loops.foldl (· ++ ·) #[]
      match planes.findIdx? (· == c) with
      | some k => onPlane := onPlane.modify k (· ++ pts)
      | none =>
        planes := planes.push c
        onPlane := onPlane.push pts
  return (planes, onPlane.map fun pts => (Box.ofPoints pts).grow 1)

/-- The comparison: the kernel's tool (`kraw`) against a definition given by its carriers (the
box holding it) and its membership. The kernel's carriers are snapped first, the definition's
representing their classes. `self`: the definition's own faces, when it has them, to check the
membership against. -/
def compare (l : Rat) (kraw : Raw) (dplanes : Array Plane) (dbox : Box) (defMem : V3 → Bool)
    (self : Option Solid) (notes0 : Array String := #[]) : Json := Id.run do
  let dcs : List Snap.Carrier := (List.range dplanes.size).map fun i => ⟨(0, i), dplanes[i]!, dbox⟩
  let kcs : List Snap.Carrier := (carriers #[kraw]).map fun c => { c with name := (1, c.name.2) }
  let cs := dcs ++ kcs
  let mut notes := notes0
  if !Snap.spreadOk l spreadCap cs then
    notes := notes.push "a class of tolerance-equal surfaces spreads past the cap"
  let snappedK := (Array.range kraw.planes.size).map fun fi =>
    ((cs.find? (·.name == (1, fi))).map fun c => Snap.snapped l cs c).getD kraw.planes[fi]!
  let (ksolid, kn) := solidOf 1 kraw snappedK l false
  notes := notes ++ kn
  let (kplanes, kboxes) := arrangementOf #[ksolid]
  -- the definition's carriers with its box, then the kernel's not already among them
  let mut planes : Array Plane := #[]
  let mut boxes : Array Box := #[]
  for p in dplanes do
    let c := p.canon
    if !planes.contains c then
      planes := planes.push c
      boxes := boxes.push (dbox.grow 1)
  for i in [0:kplanes.size] do
    match planes.findIdx? (· == kplanes[i]!) with
    | some k => boxes := boxes.modify k fun b => b.hull kboxes[i]!
    | none =>
      planes := planes.push kplanes[i]!
      boxes := boxes.push kboxes[i]!
  let diff := boundaryOf (fun p => ksolid.contains p != defMem p) planes boxes
  let selfBad := match self with
    | some ds => !(boundaryOf (fun p => ds.contains p != defMem p) planes boxes).isEmpty
    | none => false
  let defCells := boundaryOf defMem planes boxes
  let kerCells := boundaryOf ksolid.contains planes boxes
  let verdict := if selfBad then "selfcheck" else if diff.isEmpty then "agree" else "differ"
  let where_ := (diff.extract 0 6).map fun c => fmtV (centroid c.poly)
  return Json.mkObj [("verdict", verdict),
    ("volume", toString (volumeOf defCells)), ("volume_f", toJson (qf (volumeOf defCells))),
    ("kernel_volume_f", toJson (qf (volumeOf kerCells))),
    ("diff_volume_f", toJson (qf (volumeOf diff))),
    ("where", toJson where_), ("notes", toJson notes)]

/-- The frame (with `n`: the travel scaled to rise 1 along it), the range and the loops. -/
def common (j : Json) : Except String (Frame3 × Rat × Rat × List (List (Rat × Rat))) := do
  let fr ← j.getObjVal? "frame"
  let v ← v3 (← fr.getObjVal? "v")
  -- along a direction: the travel scaled to rise 1 along the sketch's normal `n`, exactly
  let v ← match fr.getObjVal? "n" with
    | .ok nj => do
      let n ← v3 nj
      if v.dot n == 0 then throw "the direction lies in the sketch's plane"
      pure ((1 / v.dot n) * v)
    | .error _ => pure v
  let F : Frame3 := ⟨← v3 (← fr.getObjVal? "o"), ← v3 (← fr.getObjVal? "x"),
    ← v3 (← fr.getObjVal? "y"), v⟩
  let lo ← dyadic (← j.getObjVal? "lo")
  let hi ← dyadic (← j.getObjVal? "hi")
  let loops ← ((← (← j.getObjVal? "loops").getArr?).mapM fun lp => do
    return (← (← lp.getArr?).mapM v2).toList)
  return (F, lo, hi, loops.toList)

/-- An extrude along a direction: the direction at least `m` (the kernel's 0.0175, about 1°: a
stated precondition) out of the sketch's plane, as a sine: `(u·n)² ≥ m² |u|² |n|²`. -/
def alongOk (j : Json) : Except String Bool := do
  let fr ← j.getObjVal? "frame"
  match fr.getObjVal? "n", j.getObjVal? "along_min" with
  | .ok nj, .ok mj =>
    let n ← v3 nj
    let u ← v3 (← fr.getObjVal? "v")
    let m ← dyadic mj
    return (u.dot n) * (u.dot n) ≥ m * m * u.dot u * n.dot n
  | _, _ => return true

def runPrism (j : Json) (l : Rat) : Except String Json := do
  let (F, lo, hi, loops) ← common j
  let valid := inDomain F lo hi l loops && (← alongOk j)
  match j.getObjVal? "kernel" with
  | .error _ =>
    let why := (j.getObjValAs? String "kernel_refused").toOption.getD ""
    return Json.mkObj [("verdict", if valid then "refused_valid" else "refused_ok"),
      ("why", why)]
  | .ok kj =>
  if !valid then
    return Json.mkObj [("verdict", "built_invalid"),
      ("notes", toJson #["the kernel built a tool for a definition outside the domain"])]
  let draw ← defRaw F lo hi loops
  let kraw ← rawOf kj
  let (dsolid, _) := solidOf 0 draw draw.planes l false
  return compare l kraw draw.planes (Box.ofPoints draw.verts) (prismMem loops F lo hi) (some dsolid)


end ForgeExact.Swept.Ref
