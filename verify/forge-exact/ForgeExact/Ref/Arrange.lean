import Std.Data.HashMap
import ForgeExact.Ref.Geom

/-!
# The flat-face reference boolean

The arrangement of every carrier plane of every solid in the history, cut down to a box around
them. On each plane the 2-D cells (convex polygons between the other planes' lines) are classified
by the history at a point just above and just below their centroid. A cell kept on exactly one
side is a piece of the result's boundary (`ForgeExact.reg_kept`: the regularised union of the kept
3-D cells is the regularised boolean, and its boundary is made of exactly these facets).

From the boundary cells: the exact volume (divergence theorem), the faces (cells of one plane and
one side joined across shared edges), the edges (maximal straight runs where two faces meet), the
vertices, and every non-manifold edge or vertex.

Before step 3's tolerance rule lands, any decision within LINEAR of the other answer (a vertex, an
edge or a face within LINEAR of another body's entity without lying on it, or a sub-LINEAR feature
of the result) makes the verdict `ambiguous`, so a kernel is never blamed for a convention case.
-/

namespace ForgeExact.Ref

open Std

/-- One facet of the result: a cell of plane `plane`, outward along `out • n`, anticlockwise about
the outward normal. -/
structure BCell where
  plane : Nat
  out : Int
  poly : Array V3
  deriving Inhabited

structure Result where
  verdict : String
  volume : Rat
  faces : Nat
  edges : Nat
  vertices : Nat
  /-- shells: boundary components joined across edges -/
  shells : Nat
  /-- why the verdict is ambiguous, or where the result is non-manifold -/
  notes : Array String
  deriving Inhabited

/-- A plain union–find over `0..n`, without path compression (sizes here are small). -/
structure UF where
  parent : Array Nat

namespace UF
def make (n : Nat) : UF := ⟨Array.range n⟩
def find (u : UF) (i : Nat) : Nat := Id.run do
  let mut x := i
  for _ in [0:u.parent.size] do
    let p := u.parent[x]!
    if p == x then break
    x := p
  return x
def union (u : UF) (i j : Nat) : UF :=
  let a := u.find i
  let b := u.find j
  if a == b then u
  else if a < b then ⟨u.parent.set! b a⟩ else ⟨u.parent.set! a b⟩
end UF

def segKey (a b : V3) : V3 × V3 := if a.lt b then (a, b) else (b, a)

def fmtV (v : V3) : String := s!"({qf v.x}, {qf v.y}, {qf v.z})"

/-- The section of `P` by the box, as a convex polygon (possibly empty). -/
def section_ (box : Box) (P : Plane) : Array V3 := Id.run do
  let n := P.n
  let ax := qabs n.x
  let ay := qabs n.y
  let az := qabs n.z
  let e : V3 := if ax ≤ ay && ax ≤ az then ⟨1, 0, 0⟩ else if ay ≤ az then ⟨0, 1, 0⟩ else ⟨0, 0, 1⟩
  let u := n.cross e
  let v := n.cross u
  let bc := (1 / 2 : Rat) * (box.lo + box.hi)
  let c0 := bc - (P.eval bc / n.norm2) * n
  let d := box.hi - box.lo
  let d1 := d.x + d.y + d.z + 1
  let mu := 3 * d1 / u.maxAbs
  let mv := 3 * d1 / v.maxAbs
  let mut poly : Array V3 :=
    #[c0 + mu * u + mv * v, c0 - mu * u + mv * v, c0 - mu * u - mv * v, c0 + mu * u - mv * v]
  let walls : Array (Plane × Bool) := #[
    (⟨⟨1, 0, 0⟩, box.lo.x⟩, true), (⟨⟨1, 0, 0⟩, box.hi.x⟩, false),
    (⟨⟨0, 1, 0⟩, box.lo.y⟩, true), (⟨⟨0, 1, 0⟩, box.hi.y⟩, false),
    (⟨⟨0, 0, 1⟩, box.lo.z⟩, true), (⟨⟨0, 0, 1⟩, box.hi.z⟩, false)]
  for (w, keepPos) in walls do
    let (p, q) := split poly w
    poly := if keepPos then p else q
    if !proper poly then return #[]
  return poly

/-- The 2-D cells of plane `i`: its section split by every other plane. -/
def cellsOn (box : Box) (planes : Array Plane) (i : Nat) : Array (Array V3) := Id.run do
  let s := section_ box planes[i]!
  if !proper s then return #[]
  let mut cells : Array (Array V3) := #[s]
  for j in [0:planes.size] do
    if j == i then continue
    let pj := planes[j]!
    -- a plane that misses the whole section splits nothing
    if s.all (fun v => pj.eval v > 0) || s.all (fun v => pj.eval v < 0) then continue
    let mut next : Array (Array V3) := #[]
    for c in cells do
      let (a, b) := split c pj
      let pa := proper a
      let pb := proper b
      if pa && pb then
        next := next.push a |>.push b
      else if pa then next := next.push a
      else if pb then next := next.push b
    cells := next
  return cells

/-- The boundary cells of the result. On plane `i` only the box `boxes[i]` is looked at: it holds
every face the plane carries with room to spare, and a boundary cell lies inside such a face
(crossing the plane anywhere else changes no solid's membership). The faces' edges are lines of
the arrangement, so no cell inside a face reaches the box's walls. -/
def boundary (solids : Array Solid) (planes : Array Plane) (boxes : Array Box) : Array BCell := Id.run do
  let mut out : Array BCell := #[]
  for i in [0:planes.size] do
    let box := boxes[i]!
    let P := planes[i]!
    for c in cellsOn box planes i do
      let m := centroid c
      -- a step off the plane small enough to cross no other plane
      let mut δ : Rat := 1
      for j in [0:planes.size] do
        if j == i then continue
        let Q := planes[j]!
        let d := qabs (Q.eval m) / (2 * (qabs (P.n.dot Q.n) + 1))
        if d < δ then δ := d
      let above := keptAt solids (m + δ * P.n)
      let below := keptAt solids (m - δ * P.n)
      if above != below then
        let o : Int := if below then 1 else -1
        let nrm := newell c
        let outward := (o : Rat) * P.n
        let poly := if nrm.dot outward > 0 then c else c.reverse
        out := out.push ⟨i, o, poly⟩
  return out

def volumeOf (cells : Array BCell) : Rat :=
  cells.foldl (fun acc bc => Id.run do
    let p := bc.poly
    let mut s := acc
    for k in [1:p.size - 1] do
      s := s + (p[0]!).dot ((p[k]!).cross p[k + 1]!) / 6
    return s) 0

/-- The result's topology from its boundary cells. -/
def topology (cells : Array BCell) :
    Nat × Nat × Nat × Nat × Array String × Array V3 := Id.run do
  -- every cell edge, by its segment
  let mut segs : HashMap (V3 × V3) (Array Nat) := {}
  for ci in [0:cells.size] do
    let p := cells[ci]!.poly
    for k in [0:p.size] do
      let key := segKey p[k]! p[(k + 1) % p.size]!
      segs := segs.insert key ((segs.getD key #[]).push ci)
  -- faces: cells of one plane and side joined across shared segments
  let mut uf := UF.make cells.size
  for (_, l) in segs.toArray do
    for a in l do
      for b in l do
        if a < b && cells[a]!.plane == cells[b]!.plane && cells[a]!.out == cells[b]!.out then
          uf := uf.union a b
  let comp := fun (i : Nat) => uf.find i
  let mut roots : HashMap Nat Unit := {}
  for i in [0:cells.size] do roots := roots.insert (comp i) ()
  let mut notes : Array String := #[]
  -- crease pieces: segments not inside one face
  let mut pieces : Array ((V3 × V3) × Array Nat) := #[]
  let mut shellUF := UF.make cells.size
  for (key, l) in segs.toArray do
    for a in l do
      for b in l do shellUF := shellUF.union a b
    if l.size == 2 && comp l[0]! == comp l[1]! then continue
    if l.size != 2 then
      notes := notes.push s!"non-manifold edge: {l.size} facets at {fmtV key.1} – {fmtV key.2}"
    pieces := pieces.push (key, (l.map comp).qsort (· < ·))
  -- crease pieces at each point
  let mut at_ : HashMap V3 (Array (V3 × Array Nat)) := {}
  for ((a, b), cs) in pieces do
    at_ := at_.insert a ((at_.getD a #[]).push (b, cs))
    at_ := at_.insert b ((at_.getD b #[]).push (a, cs))
  let mut verts : Array V3 := #[]
  let mut junctions := 0
  for (x, ps) in at_.toArray do
    let straight :=
      ps.size == 2 &&
        (let d1 := ps[0]!.1 - x
         let d2 := ps[1]!.1 - x
         (d1.cross d2).isZero && d1.dot d2 < 0 && ps[0]!.2 == ps[1]!.2)
    if straight then junctions := junctions + 1 else verts := verts.push x
  -- a manifold vertex: the facets round it form one cycle
  let mut around : HashMap V3 (Array Nat) := {}
  for ci in [0:cells.size] do
    for v in cells[ci]!.poly do
      around := around.insert v ((around.getD v #[]).push ci)
  for (x, cs) in around.toArray do
    let mut loc := UF.make cells.size
    for ci in cs do
      let p := cells[ci]!.poly
      for k in [0:p.size] do
        if p[k]! == x then
          for y in #[p[(k + 1) % p.size]!, p[(k + p.size - 1) % p.size]!] do
            for cj in segs.getD (segKey x y) #[] do loc := loc.union ci cj
    let r := loc.find cs[0]!
    if cs.any (fun ci => loc.find ci != r) then
      notes := notes.push s!"non-manifold vertex at {fmtV x}"
  let mut shells : HashMap Nat Unit := {}
  for i in [0:cells.size] do shells := shells.insert (shellUF.find i) ()
  return (roots.size, pieces.size - junctions, verts.size, shells.size, notes, verts)

/-- Decisions within `l` of the other answer, among the solids' own entities: a vertex near
another solid's face plane (over the face), two vertices of different solids near each other, two
edges of different solids passing near each other. -/
def nearInputs (solids : Array Solid) (l : Rat) : Array String := Id.run do
  let mut notes : Array String := #[]
  let vertsOf := fun (s : Solid) => (s.faces.foldl (fun acc f =>
    f.loops.foldl (fun acc lp => lp.foldl (fun m v => m.insert v ()) acc) acc)
      (({} : HashMap V3 Unit))).toArray.map (·.1)
  let edgesOf := fun (s : Solid) => (s.faces.foldl (fun acc f =>
    f.loops.foldl (fun acc lp => Id.run do
      let mut acc := acc
      for k in [0:lp.size] do acc := acc.insert (segKey lp[k]! lp[(k + 1) % lp.size]!) ()
      return acc) acc) (({} : HashMap (V3 × V3) Unit))).toArray.map (·.1)
  let vs := solids.map vertsOf
  let es := solids.map edgesOf
  for s in [0:solids.size] do
    for t in [0:solids.size] do
      if s == t then continue
      for f in solids[t]!.faces do
        let pts := f.loops.foldl (· ++ ·) #[]
        let fb := (Box.ofPoints pts).grow l
        for v in vs[s]! do
          if fb.contains v && f.plane.eval v != 0 && f.plane.within v l then
            notes := notes.push s!"vertex {fmtV v} of solid {s} within LINEAR of a face of solid {t}"
      if s < t then
        for v in vs[s]! do
          for w in vs[t]! do
            if v != w && (v - w).norm2 ≤ l * l then
              notes := notes.push s!"vertices {fmtV v} and {fmtV w} of solids {s}, {t} within LINEAR"
        for (a, b) in es[s]! do
          for (c, d) in es[t]! do
            let d1 := b - a
            let d2 := d - c
            let w := c - a
            let cr := d1.cross d2
            if !cr.isZero then
              let n2 := cr.norm2
              let e := w.dot cr
              if e != 0 && e * e ≤ l * l * n2 then
                let ps := (w.cross d2).dot cr / n2
                let pt := (w.cross d1).dot cr / n2
                if 0 ≤ ps && ps ≤ 1 && 0 ≤ pt && pt ≤ 1 then
                  notes := notes.push s!"edges of solids {s}, {t} pass within LINEAR at {fmtV (a + ps * d1)}"
            else
              let off := w.cross d1
              let n1 := d1.norm2
              if !off.isZero && off.norm2 ≤ l * l * n1 then
                let sc := w.dot d1 / n1
                let sd := (d - a).dot d1 / n1
                if qmax sc sd ≥ 0 && qmin sc sd ≤ 1 then
                  notes := notes.push s!"parallel edges of solids {s}, {t} within LINEAR near {fmtV c}"
  return notes

/-- Sub-LINEAR features of the result: two vertices within `l`, or a vertex within `l` of a facet
it is not on. -/
def nearResult (cells : Array BCell) (planes : Array Plane) (verts : Array V3) (l : Rat) :
    Array String := Id.run do
  let mut notes : Array String := #[]
  for i in [0:verts.size] do
    for j in [i + 1:verts.size] do
      if (verts[i]! - verts[j]!).norm2 ≤ l * l then
        notes := notes.push s!"result vertices {fmtV verts[i]!} and {fmtV verts[j]!} within LINEAR"
  for c in cells do
    let P := planes[c.plane]!
    let bb := (Box.ofPoints c.poly).grow l
    for v in verts do
      if bb.contains v && P.eval v != 0 && P.within v l then
        notes := notes.push s!"result vertex {fmtV v} within LINEAR of a face it is not on (a sliver)"
        break
  return notes

/-- Notes once each, in order, at most twelve. -/
def dedupe (ns : Array String) : Array String := Id.run do
  let mut seen : HashMap String Unit := {}
  let mut out : Array String := #[]
  for n in ns do
    if !seen.contains n then
      seen := seen.insert n ()
      out := out.push n
  if out.size > 12 then
    return (out.extract 0 12).push s!"and {out.size - 12} more"
  return out

/-- The reference's answer. `extra`: reasons the input itself is ambiguous (a vertex its planes do
not settle, a class past the spread cap). With `strict` (the rule before §6.1 is applied), any
decision within `l` of the other answer is ambiguous too. Without it (the inputs already snapped
by §6.1), what is left within `l` is an exact sub-tolerance crossing: the result is decided
exactly, and its sub-LINEAR features are reported (`sliver`): the kernel must collapse them
validly within c·LINEAR or refuse with ZeroThickness there. -/
def decide (solids : Array Solid) (l : Rat) (extra : Array String) (strict : Bool := true) :
    Result := Id.run do
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
  if planes.isEmpty then
    return ⟨"empty", 0, 0, 0, 0, 0, extra⟩
  let boxes := onPlane.map fun pts => (Box.ofPoints pts).grow 1
  let cells := boundary solids planes boxes
  let vol := volumeOf cells
  let (nf, ne, nv, ns, nm, verts) := topology cells
  let crossings := nearInputs solids l
  let slivers := nearResult cells planes verts l
  let near := dedupe (extra ++ if strict then crossings ++ slivers else #[])
  let verdict :=
    if !near.isEmpty then "ambiguous"
    else if cells.isEmpty then "empty"
    else if !nm.isEmpty then "nonmanifold"
    else if !slivers.isEmpty then "sliver"
    else "solid"
  let notes := if strict then near ++ nm else dedupe (extra ++ nm ++ slivers ++ crossings)
  return ⟨verdict, vol, nf, ne, nv, ns, notes⟩

end ForgeExact.Ref
