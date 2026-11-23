import Lean.Data.Json
import ForgeExact.Ref.Arrange
import ForgeExact.Snap

/-!
# The driver's format

One JSON object per line on stdin, one answer per line on stdout. Every number is an f64 written
exactly as `[m, e]`, meaning `m · 2^e` (so nothing is lost on the way):

```
{"linear": [m,e],
 "solids": [{"op": "new|add|cut|keep",
             "verts": [[x,y,z], ...],
             "faces": [{"o": [x,y,z], "n": [x,y,z], "loops": [[vertex index, ...], ...]}]}]}
```

A face's carrier is the plane through `o` with normal `n`, the kernel's own frame. Each vertex is
recomputed exactly from the carriers of the faces round it (research/kernel-audit/README.md §6.1,
step 2); the kernel's vertex is only checked against it.

The answer: `{"verdict": "solid|empty|nonmanifold|ambiguous", "volume": "p/q", "volume_f": v,
"faces": F, "edges": E, "vertices": V, "shells": S, "notes": [...]}`.

With `"points": [[x,y,z], ...]` the question is membership instead, no boolean made: the answer is
`{"inside": [...], "near": [...], "notes": [...]}`, per point whether the history's pointwise
boolean (`keptAt`, exact, its ray perturbed symbolically) holds it, and whether it lies within
LINEAR of a face's carrier (there the tolerance rule, not the exact set, decides the side).
-/

namespace ForgeExact.Ref

open Lean

def dyadic (j : Json) : Except String Rat := do
  let a ← j.getArr?
  if a.size != 2 then throw "a number is [m, e]"
  let m ← a[0]!.getInt?
  let e ← a[1]!.getInt?
  if e ≥ 0 then return ((m * 2 ^ e.toNat : Int) : Rat)
  else return mkRat m (2 ^ (-e).toNat)

def v3 (j : Json) : Except String V3 := do
  let a ← j.getArr?
  if a.size != 3 then throw "a point is [x, y, z]"
  return ⟨← dyadic a[0]!, ← dyadic a[1]!, ← dyadic a[2]!⟩

/-- A solid as dumped: its op, the kernel's vertices, its face planes and loops. -/
structure Raw where
  op : String
  verts : Array V3
  planes : Array Plane
  loops : Array (Array (Array Nat))
  deriving Inhabited

def rawOf (j : Json) : Except String Raw := do
  let op ← (← j.getObjVal? "op").getStr?
  let verts ← (← (← j.getObjVal? "verts").getArr?).mapM v3
  let faces ← (← j.getObjVal? "faces").getArr?
  let mut planes : Array Plane := #[]
  let mut loops : Array (Array (Array Nat)) := #[]
  for f in faces do
    let o ← v3 (← f.getObjVal? "o")
    let n ← v3 (← f.getObjVal? "n")
    if n.isZero then throw "a face's normal is zero"
    planes := planes.push ⟨n, n.dot o⟩
    let ls ← (← f.getObjVal? "loops").getArr?
    loops := loops.push (← ls.mapM fun lp => do (← lp.getArr?).mapM fun i => do
      let i ← i.getNat?
      if i ≥ verts.size then throw "a loop names a vertex the solid does not have"
      return i)
  return ⟨op, verts, planes, loops⟩

/-- The solid with its vertices recomputed exactly from `planes` (the face planes, snapped or
not); notes for any vertex those planes do not settle. -/
def solidOf (k : Nat) (r : Raw) (planes : Array Plane) (l : Rat) (strict : Bool) :
    Solid × Array String := Id.run do
  -- the faces round each vertex
  let mut round : Array (Array Nat) := Array.replicate r.verts.size #[]
  for fi in [0:r.loops.size] do
    for lp in r.loops[fi]! do
      for v in lp do
        if !(round[v]!).contains fi then round := round.set! v ((round[v]!).push fi)
  let mut exact : Array V3 := r.verts
  let mut notes : Array String := #[]
  for v in [0:r.verts.size] do
    let fs := round[v]!
    if fs.isEmpty then continue
    match Snap.vertexAt (fs.toList.map fun f => planes[f]!) with
    | none =>
      notes := notes.push s!"solid {k}: vertex {fmtV r.verts[v]!} is not one point of the planes round it"
    | some p =>
      exact := exact.set! v p
      if strict && (p - r.verts[v]!).norm2 > l * l then
        notes := notes.push s!"solid {k}: the kernel's vertex {fmtV r.verts[v]!} is over LINEAR from its planes' meet {fmtV p}"
  let fs : Array Face := (Array.range planes.size).map fun fi =>
    ⟨planes[fi]!, (r.loops[fi]!).map fun lp => lp.map fun i => exact[i]!⟩
  return (⟨r.op, fs⟩, notes)

/-- Every face's carrier, named (solid, face), its box the kernel's vertices of that face. -/
def carriers (raws : Array Raw) : List Snap.Carrier := Id.run do
  let mut cs : Array Snap.Carrier := #[]
  for k in [0:raws.size] do
    let r := raws[k]!
    for fi in [0:r.planes.size] do
      let pts := (r.loops[fi]!).foldl (fun acc lp => acc ++ lp.map fun i => r.verts[i]!) #[]
      if pts.isEmpty then continue
      cs := cs.push ⟨(k, fi), r.planes[fi]!, Box.ofPoints pts⟩
  return cs.toList

def answer (r : Result) : Json :=
  Json.mkObj [
    ("verdict", r.verdict),
    ("volume", toString r.volume),
    ("volume_f", toJson (qf r.volume)),
    ("faces", r.faces), ("edges", r.edges), ("vertices", r.vertices), ("shells", r.shells),
    ("notes", toJson r.notes)]

/-- The cap on a class's spread, in LINEARs (§6.1 suggests 2). -/
def spreadCap : Rat := 2

def run (line : String) : Except String Json := do
  let j ← Json.parse line
  let l ← dyadic (← j.getObjVal? "linear")
  let strict := (j.getObjValAs? String "rule" |>.toOption.getD "strict") != "snap"
  let ss ← (← j.getObjVal? "solids").getArr?
  let raws ← ss.mapM rawOf
  let mut notes : Array String := #[]
  -- §6.1: every carrier decided in its class's representative, or ambiguous past the cap
  let mut planesOf : Array (Array Plane) := raws.map (·.planes)
  if !strict then
    let cs := carriers raws
    if !Snap.spreadOk l spreadCap cs then
      notes := notes.push "a class of tolerance-equal surfaces spreads past the cap"
    let snapped := cs.map fun c => (c.name, Snap.snapped l cs c)
    planesOf := (Array.range raws.size).map fun k =>
      (Array.range raws[k]!.planes.size).map fun fi =>
        (snapped.lookup (k, fi)).getD raws[k]!.planes[fi]!
  let mut solids : Array Solid := #[]
  for k in [0:raws.size] do
    let (s, n) := solidOf k raws[k]! planesOf[k]! l strict
    solids := solids.push s
    notes := notes ++ n
  match j.getObjVal? "points" with
  | .ok ps =>
    let pts ← (← ps.getArr?).mapM v3
    let inside := pts.map fun p => keptAt solids p
    let near := pts.map fun p => solids.any fun s => s.faces.any fun f => f.plane.within p l
    return Json.mkObj [("inside", toJson inside), ("near", toJson near), ("notes", toJson notes)]
  | .error _ => return answer (decide solids l notes strict)

end ForgeExact.Ref
