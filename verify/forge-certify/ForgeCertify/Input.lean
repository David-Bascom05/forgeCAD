import Lean.Data.Json
import ForgeExact.Ref.Input
import ForgeCertify.Check

/-!
# The driver: from the kernel's certificate to `check`

Untrusted: whatever this does, `check_sound` holds of the `check` it ends in. One JSON object per
line (numbers exact as `[m, e]` = m·2^e):

```
{"linear": [m,e], "op": "union|inter|diff" | "ops": ["new","add","cut","keep",...],
 "keys": [[x,y,z], ...],
 "operands": [{"faces": [{"o": p, "n": p, "loops": [[key, ...], ...]}],
               "pieces": [{"face": j, "loops": [[key, ...], ...]}]}],
 "result": {"faces": [{"src": [i, j], "o": p, "n": p, "loops": [[key, ...], ...]}]},
 "frames": "x" | "xyz"}
```

Keys are the shared vertices; a face of an operand with no pieces is its own piece; a result face
names the operand face whose plane it carries (bit for bit). The driver applies the tolerance rule
(`ForgeExact.Snap`) to the operands' carriers, places every key at the meet of the snapped planes of
the faces through it, triangulates the pieces exactly, and runs `check` (in each frame asked).

The answer: `{"verdict": "accepted|rejected|scope", "reason": "..."}`.
-/

namespace ForgeCertify

open Lean ForgeExact.Ref ForgeExact

structure RFace where
  o : V3
  n : V3
  loops : Array (Array Nat)
  deriving Inhabited

structure ROpd where
  faces : Array RFace
  pieces : Array (Nat × Array (Array Nat))
  deriving Inhabited

structure RRes where
  faces : Array (Nat × Nat × RFace)
  deriving Inhabited

def natArr (j : Json) : Except String (Array Nat) := do (← j.getArr?).mapM fun x => x.getNat?
def loopsOf (j : Json) : Except String (Array (Array Nat)) := do (← j.getArr?).mapM natArr

def rface (j : Json) : Except String RFace := do
  return ⟨← v3 (← j.getObjVal? "o"), ← v3 (← j.getObjVal? "n"), ← loopsOf (← j.getObjVal? "loops")⟩

def ropd (j : Json) : Except String ROpd := do
  let fs ← (← (← j.getObjVal? "faces").getArr?).mapM rface
  let ps ← match j.getObjVal? "pieces" with
    | .ok p => (← p.getArr?).mapM fun x => do
        return ((← (← x.getObjVal? "face").getNat?), ← loopsOf (← x.getObjVal? "loops"))
    | .error _ => pure #[]
  return ⟨fs, ps⟩

def rres (j : Json) : Except String RRes := do
  let fs ← (← (← j.getObjVal? "faces").getArr?).mapM fun x => do
    let src ← natArr (← x.getObjVal? "src")
    if src.size != 2 then throw "src is [operand, face]"
    return (src[0]!, src[1]!, ← rface x)
  return ⟨fs⟩

/-! ## The boolean -/

def boolOp : String → Option (List Bool → Bool)
  | "union" => some fun l => l.getD 0 false || l.getD 1 false
  | "inter" => some fun l => l.getD 0 false && l.getD 1 false
  | "diff" => some fun l => l.getD 0 false && !l.getD 1 false
  | _ => none

def historyOp (ops : Array String) : List Bool → Bool := fun l =>
  (List.range ops.size).foldl (fun r i =>
    let m := l.getD i false
    if ops[i]! == "cut" then r && !m else if ops[i]! == "keep" then r && m else r || m) false

/-! ## Exact triangulation: vertical slabs -/

/-- The face's plane's own 2-D frame (yz unless the plane contains `x`), and back. -/
def proj (Q : Plane) (p : V3) : P2 := if Q.n.x != 0 then yz p else piv Q p

def liftP (Q : Plane) (q : P2) : V3 :=
  if Q.n.x != 0 then ⟨(Q.c - Q.n.y * q.u - Q.n.z * q.v) / Q.n.x, q.u, q.v⟩
  else if Q.n.z != 0 then ⟨q.u, q.v, (Q.c - Q.n.y * q.v) / Q.n.z⟩
  else ⟨q.u, (Q.c - Q.n.z * q.v) / Q.n.y, q.v⟩

def vAt (p q : P2) (u : Rat) : Rat := p.v + (u - p.u) * (q.v - p.v) / (q.u - p.u)

/-- Where two segments cross strictly inside both, its `u`. -/
def crossU (e f : P2 × P2) : Option Rat :=
  let d := o2 e.1 e.2 f.1
  let d' := o2 e.1 e.2 f.2
  let g := o2 f.1 f.2 e.1
  let g' := o2 f.1 f.2 e.2
  if d * d' < 0 && g * g' < 0 then
    let t := g / (g - g')
    some (e.1.u + t * (e.2.u - e.1.u))
  else none

/-- The region inside the loops (even–odd), cut by vertical lines through every vertex and
crossing, and along the constraint segments (other faces' edges lying in this plane), into
trapezoids, two triangles each. Equal regions with equal constraints give equal triangles. -/
def slabs (Q : Plane) (loops : List (List V3)) (cons : List (P2 × P2)) : List Tri := Id.run do
  let ord := fun (p q : P2) => if p.u < q.u then (p, q) else (q, p)
  let edges : List (P2 × P2) := loops.flatMap fun lp =>
    (List.range lp.length).filterMap fun i =>
      let p := proj Q (lp.getD i default)
      let q := proj Q (lp.getD ((i + 1) % lp.length) default)
      if p.u == q.u then none else some (ord p q)
  let pts := loops.flatten.map fun p => (proj Q p).u
  if pts.isEmpty then return []
  let lo := pts.foldl qmin (pts.getD 0 0)
  let hi := pts.foldl qmax (pts.getD 0 0)
  let cons := (cons.filter fun c => c.1.u != c.2.u).map fun c => ord c.1 c.2
  let cons := cons.filter fun c => c.1.u < hi && lo < c.2.u
  let all := edges.map (·, true) ++ cons.map (·, false)
  let cuts := all.flatMap fun (e, _) => all.filterMap fun (f, _) => crossU e f
  let us := (pts ++ cons.flatMap (fun c => [c.1.u, c.2.u]) ++ cuts).filter fun u => lo ≤ u && u ≤ hi
  let us := (uniq us).mergeSort (· ≤ ·)
  let mut out : List Tri := []
  for k in [0:us.length - 1] do
    let u0 := us.getD k 0
    let u1 := us.getD (k + 1) 0
    let um := (u0 + u1) / 2
    let act := (all.filter fun (e, _) => e.1.u ≤ u0 && u1 ≤ e.2.u).mergeSort
      fun e f => vAt e.1.1 e.1.2 um ≤ vAt f.1.1 f.1.2 um
    let mut inside := false
    let mut prev : Option (P2 × P2) := none
    for (e, isLoop) in act do
      if inside then
        if let some f := prev then
          let a := liftP Q (P2.mk u0 (vAt f.1 f.2 u0))
          let b := liftP Q (P2.mk u1 (vAt f.1 f.2 u1))
          let c := liftP Q (P2.mk u1 (vAt e.1 e.2 u1))
          let d := liftP Q (P2.mk u0 (vAt e.1 e.2 u0))
          for t in [Tri.mk a b c, Tri.mk a c d] do
            if proper Q t then out := t :: out
      if isLoop then inside := !inside
      prev := some e
  return out

/-! ## Interpretation -/

def boxOf (ps : List V3) : Box := Box.ofPoints ps.toArray

structure Interp where
  ins : List Opd
  res : List Face

def keyPlanes (nk : Nat) (fs : List (Plane × Array (Array Nat))) : Array (List Plane) := Id.run do
  let mut acc : Array (List Plane) := Array.replicate nk []
  for (pl, lps) in fs do
    for lp in lps do
      for k in lp do
        if k < nk && !(acc[k]!.contains pl) then acc := acc.set! k (pl :: acc[k]!)
  return acc

def interpret (l : Rat) (keys : Array V3) (opds : Array ROpd) (res : RRes) :
    Except (String × String) Interp := do
  -- carriers: every operand face, its box the kernel's points of its loops
  let mut cs : Array Snap.Carrier := #[]
  for i in [0:opds.size] do
    for j in [0:opds[i]!.faces.size] do
      let f := opds[i]!.faces[j]!
      let pts := f.loops.toList.flatMap fun lp => lp.toList.map fun k => keys.getD k default
      if pts.isEmpty then continue
      cs := cs.push ⟨(i, j), ⟨f.n, f.n.dot f.o⟩, boxOf pts⟩
  let csl := cs.toList
  if !Snap.spreadOk l 2 csl then throw ("scope", "a class of tolerance-equal surfaces spreads past the cap")
  let snapped : Array (Array Plane) := (Array.range opds.size).map fun i =>
    (Array.range opds[i]!.faces.size).map fun j =>
      match csl.find? (·.name == (i, j)) with
      | some c => Snap.snapped l csl c
      | none => let f := opds[i]!.faces[j]!; ⟨f.n, f.n.dot f.o⟩
  -- the result's planes: the operand plane each carries
  let mut rplanes : Array Plane := #[]
  for (i, j, f) in res.faces do
    if i ≥ opds.size || j ≥ opds[i]!.faces.size then throw ("rejected", "a result face names no operand face")
    let g := opds[i]!.faces[j]!
    if !(g.o == f.o && g.n == f.n) then
      throw ("rejected", s!"result face on a surface no operand has (it names operand {i} face {j})")
    rplanes := rplanes.push snapped[i]![j]!
  -- keys at the meets of the snapped planes through them
  let mut users : List (Plane × Array (Array Nat)) := []
  for i in [0:opds.size] do
    for j in [0:opds[i]!.faces.size] do
      users := (snapped[i]![j]!, opds[i]!.faces[j]!.loops) :: users
    for (j, lps) in opds[i]!.pieces do
      users := (snapped[i]!.getD j default, lps) :: users
  for r in [0:res.faces.size] do
    users := (rplanes[r]!, res.faces[r]!.2.2.loops) :: users
  let kp := keyPlanes keys.size users
  let mut exact : Array V3 := keys
  for k in [0:keys.size] do
    if kp[k]!.isEmpty then continue
    match Snap.vertexAt kp[k]! with
    | none => throw ("scope", s!"vertex {fmtV keys[k]!} is not one point of the planes round it")
    | some p =>
      if (p - keys[k]!).norm2 > l * l then
        throw ("rejected", s!"vertex {fmtV keys[k]!} is over LINEAR from its planes' meet {fmtV p}")
      exact := exact.set! k p
  let pt := fun (k : Nat) => exact.getD k default
  let face := fun (pl : Plane) (lps : Array (Array Nat)) =>
    (⟨pl, lps.map fun lp => lp.map pt⟩ : Face)
  -- every operand edge, exact
  let allEdges : List (V3 × V3) := (List.range opds.size).flatMap fun i =>
    opds[i]!.faces.toList.flatMap fun f => f.loops.toList.flatMap fun lp =>
      (List.range lp.size).map fun m => (pt lp[m]!, pt lp[(m + 1) % lp.size]!)
  let consOn := fun (Q : Plane) => (allEdges.filter fun e => Q.eval e.1 == 0 && Q.eval e.2 == 0).map
    fun e => (proj Q e.1, proj Q e.2)
  let ins := (List.range opds.size).map fun i =>
    let o := opds[i]!
    let faces := (List.range o.faces.size).map fun j => face snapped[i]![j]! o.faces[j]!.loops
    let tris := (List.range o.faces.size).map fun j =>
      let ps := o.pieces.toList.filter (·.1 == j)
      let ps := if ps.isEmpty then [(j, o.faces[j]!.loops)] else ps
      let Q := snapped[i]![j]!
      let cs := consOn Q
      ps.flatMap fun (_, lps) => slabs Q (lps.toList.map fun lp => lp.toList.map pt) cs
    (⟨faces, tris⟩ : Opd)
  let resf := (List.range res.faces.size).map fun r => face rplanes[r]! res.faces[r]!.2.2.loops
  return ⟨ins, resf⟩

/-! ## Why a check fails (diagnostic only) -/

def firstBad {α : Type} (l : List α) (p : α → Bool) : Option (Nat × α) :=
  (l.zipIdx.find? fun (a, _) => !p a).map fun (a, i) => (i, a)

def explain (op : List Bool → Bool) (ins : List Opd) (res : List Face) : String := Id.run do
  if op (vec ins.length fun _ => false) then return "the boolean keeps what is far away"
  for i in [0:ins.length] do
    let o := ins[i]!
    if !tilesOK o then
      match firstBad (o.faces.zip o.tris) (fun ft => nonzero ft.1.plane.n &&
          ft.2.all (fun t => onPlane ft.1.plane t && proper ft.1.plane t) &&
          zero1 (faceSegs ft.1 ++ ft.2.flatMap Tri.segs)) with
      | some (j, _) => return s!"operand {i}: the pieces of face {j} do not tile it"
      | none => return s!"operand {i}: faces and triangle lists differ in number"
    if !closedOK o then return s!"operand {i} is not closed"
  for P in planes ins res do
    let pn := s!"plane {qf P.n.x} {qf P.n.y} {qf P.n.z} {qf P.c}"
    let As := atoms ins P
    if !atomsOK As then return s!"{pn}: two pieces overlap without being equal"
    for τ in As do
      if !atomOK ins P τ then
        let w := witness ins P τ
        match firstBad (allSoup ins) (fun t => !offP P t || witnessOK w t.1 t.2) with
        | some _ => return s!"{pn}: the witness {fmtV w} is tied with another surface"
        | none =>
          let bad := (allSoup ins).find? (fun t => offP P t && !separated P t.1 τ t.2)
          let d := match bad with
            | some t => s!" (triangle {fmtV t.2.a} {fmtV t.2.b} {fmtV t.2.c} of plane {qf t.1.n.x} {qf t.1.n.y} {qf t.1.n.z} {qf t.1.c}; piece {fmtV τ.a} {fmtV τ.b} {fmtV τ.c})"
            | none => ""
          return s!"{pn}: a surface passes through the piece at {fmtV w}{d}"
    if !resultOK op ins res P As then
      return s!"{pn}: the result's faces there are not the boolean's (a face missing, extra or misplaced)"
  return "?"

/-! ## Frames -/

/-- The coordinates cycled: `(x, y, z) ↦ (y, z, x)`. Rays along `+x` in the cycled frame run
along `+y` (once) or `+z` (twice) in the original. -/
def cyc (p : V3) : V3 := ⟨p.y, p.z, p.x⟩
def cycPlane (P : Plane) : Plane := ⟨cyc P.n, P.c⟩
def cycFace (f : Face) : Face := ⟨cycPlane f.plane, f.loops.map fun lp => lp.map cyc⟩
def cycTri (t : Tri) : Tri := ⟨cyc t.a, cyc t.b, cyc t.c⟩

def stats (op : List Bool → Bool) (ins : List Opd) (res : List Face) : String :=
  let ps := planes ins res
  s!"tris {(allSoup ins).length} planes {ps.length} atoms {(ps.map fun P => (atoms ins P).length).foldl (· + ·) 0} res faces {res.length} {(op [] : Bool)}"

def run (line : String) : Except String Json := do
  let j ← Json.parse line
  let l ← dyadic (← j.getObjVal? "linear")
  let op ← match j.getObjValAs? String "op" with
    | .ok s => match boolOp s with
      | some f => pure f
      | none => throw s!"unknown op {s}"
    | .error _ => do
      let ops ← (← (← j.getObjVal? "ops").getArr?).mapM fun x => x.getStr?
      pure (historyOp ops)
  let keys ← (← (← j.getObjVal? "keys").getArr?).mapM v3
  let opds ← (← (← j.getObjVal? "operands").getArr?).mapM ropd
  let res ← rres (← j.getObjVal? "result")
  let frames := (j.getObjValAs? String "frames" |>.toOption.getD "xyz")
  let answer := fun (v r : String) => Json.mkObj [("verdict", v), ("reason", r)]
  match interpret l keys opds res with
  | .error (v, r) => return answer v r
  | .ok I =>
    if frames == "stats" then return answer "stats" (stats op I.ins I.res)
    let n := if frames == "x" then 1 else 3
    let mut ins := I.ins
    let mut rs := I.res
    for k in [0:n] do
      if !check op ins rs then
        let axis := ["x", "y", "z"].getD k "?"
        return answer "rejected" s!"({axis} frame) {explain op ins rs}"
      ins := ins.map fun o => ⟨o.faces.map cycFace, o.tris.map fun ts => ts.map cycTri⟩
      rs := rs.map cycFace
    return answer "accepted" ""

end ForgeCertify
