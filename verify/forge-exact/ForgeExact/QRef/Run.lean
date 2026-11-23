import ForgeExact.QRef.Decide

/-!
# The curved reference's driver

The answer: `{"verdict": "solid|empty|nonmanifold|sliver|ambiguous", "volume_lo": "p/q",
"volume_hi": "p/q", "volume_lo_f", "volume_hi_f", "volume_f" (estimate), "boxes", "notes"}`.

**Near decisions.** Two carriers of different kinds of contact, within LINEAR of touching
(tangent) or of being one surface, but not exactly, over faces whose boxes (grown by LINEAR)
meet: a sphere or a cylinder against a plane, a sphere, or a parallel cylinder, by the exact gap
between centres or axes and the radii (squared, so in ℚ). Under the strict rule such a pair
makes the verdict `ambiguous`. Under the snap rule (§6.1) planes are first snapped by
`Snap.snapped` and tolerance-equal spheres and parallel cylinders merged into their class's
least-named member; a pair left within LINEAR of touching is an exact sub-tolerance contact, and
the verdict is `sliver` (the kernel must collapse it validly or refuse with ZeroThickness).
Cones are not tested for near contacts.
-/

namespace ForgeExact.QRef

open Lean Ref Quad Alg

/-- `|d − R| ≤ l` but `d ≠ R`, from `d²`. -/
def nearGap (d2 R l : Rat) : Bool :=
  let lo := qmax 0 (R - l)
  d2 != R * R && lo * lo ≤ d2 && d2 ≤ (R + l) * (R + l)

def boxesMeet (l : Rat) (a b : Box) : Bool := ((a.grow l).inter (b.grow l)).nonempty

/-- The squared distance from `p` to the line `o + t z`. -/
def distLine2 (p o z : V3) : Rat := ((p - o).cross z).norm2 / z.norm2

/-- The squared distance from `p` to the plane through `o` with normal `z`. -/
def distPlane2 (p o z : V3) : Rat := let e := z.dot (p - o); e * e / z.norm2

/-- Near decisions between two faces: within LINEAR of touching, or of being one surface. -/
def nearPair (l : Rat) (f g : QFace) : Option String :=
  if !boxesMeet l f.box g.box then none
  else
    let tag := s!"faces {f.solid}.{f.index} and {g.solid}.{g.index}"
    let gap := fun (d2 : Rat) (r1 r2 : Rat) =>
      if nearGap d2 (r1 + r2) l || nearGap d2 (qabs (r1 - r2)) l then
        some s!"{tag} are within LINEAR of touching"
      else none
    let same := fun (d2 : Rat) (r1 r2 : Rat) =>
      if d2 ≤ l * l && qabs (r1 - r2) ≤ l && !(d2 == 0 && r1 == r2) then
        some s!"{tag} are within LINEAR of one surface"
      else none
    match f.kind, g.kind with
    | .sphere, .sphere =>
      let d2 := (f.o - g.o).norm2
      (same d2 f.r g.r).orElse fun _ => gap d2 f.r g.r
    | .cyl, .cyl =>
      if !(f.z.cross g.z).isZero then none
      else
        let d2 := distLine2 g.o f.o f.z
        (same d2 f.r g.r).orElse fun _ => gap d2 f.r g.r
    | .sphere, .plane => gap (distPlane2 f.o g.o g.z) f.r 0
    | .plane, .sphere => gap (distPlane2 g.o f.o f.z) g.r 0
    | .cyl, .plane => if f.z.dot g.z != 0 then none else gap (distPlane2 f.o g.o g.z) f.r 0
    | .plane, .cyl => if f.z.dot g.z != 0 then none else gap (distPlane2 g.o f.o f.z) g.r 0
    | .sphere, .cyl => gap (distLine2 f.o g.o g.z) f.r g.r
    | .cyl, .sphere => gap (distLine2 g.o f.o f.z) f.r g.r
    | _, _ => none

/-- Two curved faces tolerance-equal (one surface to within LINEAR): spheres, parallel
cylinders. -/
def tolSame (l : Rat) (f g : QFace) : Bool :=
  boxesMeet l f.box g.box &&
  match f.kind, g.kind with
  | .sphere, .sphere => (f.o - g.o).norm2 ≤ l * l && qabs (f.r - g.r) ≤ l
  | .cyl, .cyl => (f.z.cross g.z).isZero && distLine2 g.o f.o f.z ≤ l * l && qabs (f.r - g.r) ≤ l
  | _, _ => false

/-- §6.1 on the inputs: planes by `Snap.snapped`, spheres and parallel cylinders merged into
their class's least-named member (classes by repeated merging). -/
def snapFaces (l : Rat) (solids : Array QSolid) : Array QSolid × Array String := Id.run do
  let mut notes : Array String := #[]
  -- planes
  let mut cs : Array Snap.Carrier := #[]
  for s in solids do
    for f in s.faces do
      if f.kind == .plane then
        cs := cs.push ⟨(f.solid, f.index), ⟨f.z, f.z.dot f.o⟩, f.box⟩
  let csl := cs.toList
  if !Snap.spreadOk l 2 csl then
    notes := notes.push "a class of tolerance-equal planes spreads past the cap"
  let mut out := solids
  for c in csl do
    let P := Snap.snapped l csl c
    let (k, i) := c.name
    let o : V3 := (P.c / P.n.norm2) * P.n
    out := out.modify k fun s => { s with faces := s.faces.modify i fun f => { f with o := o, z := P.n } }
  -- curved: the least-named face tolerance-equal (transitively) to each
  let all := out.foldl (fun acc s => acc ++ s.faces) #[]
  let curved := all.filter fun f => f.kind == .sphere || f.kind == .cyl
  let mut rep : Array Nat := Array.range curved.size
  for _ in [0:curved.size] do
    let mut changed := false
    for a in [0:curved.size] do
      for b in [0:curved.size] do
        if rep[b]! < rep[a]! && tolSame l curved[a]! curved[b]! then
          rep := rep.set! a rep[b]!; changed := true
    if !changed then break
  for a in [0:curved.size] do
    let r := curved[rep[a]!]!
    let f := curved[a]!
    if rep[a]! != a then
      out := out.modify f.solid fun s => { s with faces := s.faces.modify f.index fun g =>
        { g with o := r.o, z := if g.z.dot r.z < 0 then (-1 : Rat) * r.z else r.z, r := r.r } }
  return (out, notes)

def qstr (q : Rat) : String := toString q

def runCurved (j : Json) : Except String Json := do
  let l ← dyadic (← j.getObjVal? "linear")
  let strict := (j.getObjValAs? String "rule" |>.toOption.getD "strict") != "snap"
  let maxBoxes := (j.getObjValAs? Nat "boxes" |>.toOption.getD 40000)
  let maxDepth := (j.getObjValAs? Nat "depth" |>.toOption.getD 8)
  let lines := (j.getObjValAs? Nat "lines" |>.toOption.getD 0)
  let ss ← (← j.getObjVal? "solids").getArr?
  let mut solids : Array QSolid := #[]
  for k in [0:ss.size] do solids := solids.push (← solidOf k ss[k]!)
  let mut notes : Array String := #[]
  let mut ambiguous : Array String := #[]
  if !strict then
    let (s', n) := snapFaces l solids
    solids := s'
    ambiguous := ambiguous ++ n
  -- near decisions
  let faces := solids.foldl (fun acc s => acc ++ s.faces) #[]
  let mut nearNotes : Array String := #[]
  for a in [0:faces.size] do
    for b in [a + 1:faces.size] do
      let f := faces[a]!
      let g := faces[b]!
      if (canon f.quadric).1 == (canon g.quadric).1 then continue
      match nearPair l f g with
      | some n => nearNotes := nearNotes.push n
      | none => pure ()
      -- strict: planes tolerance-equal but not one
      if strict && f.kind == .plane && g.kind == .plane then
        let a' : Snap.Carrier := ⟨(f.solid, f.index), ⟨f.z, f.z.dot f.o⟩, f.box⟩
        let b' : Snap.Carrier := ⟨(g.solid, g.index), ⟨g.z, g.z.dot g.o⟩, g.box⟩
        if Snap.tolEq l a' b' then
          nearNotes := nearNotes.push s!"planes of faces {f.solid}.{f.index} and {g.solid}.{g.index} are within LINEAR of one"
  if strict then ambiguous := ambiguous ++ nearNotes
  let A := arrOf solids
  if faces.isEmpty then
    return Json.mkObj [("verdict", "empty"), ("notes", toJson notes), ("probes", toJson (#[] : Array Int))]
  -- the root cube: every face box, grown by 1
  let bb := (faces.foldl (fun b f => b.hull f.box) faces[0]!.box).grow 1
  let ctr := (1 / 2 : Rat) * (bb.lo + bb.hi)
  let d := bb.hi - bb.lo
  let half := (1 / 2 : Rat) * qmax d.x (qmax d.y d.z)
  let tree := octree A ⟨ctr, half⟩ maxBoxes maxDepth
  let (est, estErr) := if lines == 0 then estimate2 A bb.lo bb.hi 1 else (estimate A (ctr - ⟨half, half, half⟩) (ctr + ⟨half, half, half⟩) lines, 0)
  -- probes: a point whose box of half-width probe_h certifies every sign is in (1) or out (-1)
  -- of the regularised boolean at every point of the box (`Quad.member_box`); 0 if not certified
  let ph := (num j "probe_h").toOption.getD l
  let probes ← match j.getObjVal? "probes" with
    | .ok ps => do (← ps.getArr?).mapM v3
    | .error _ => pure #[]
  let probeAns : Array Int := probes.map fun p =>
    let sg := A.qs.map fun q => boxSign q p ph
    if sg.any (· == 0) then 0 else if A.keptOn (fun i => sg[i]! > 0) then 1 else -1
  let cts := contacts A
  -- near touches: pairs with no exact contact
  let faceBoxes := faces.map (·.box)
  for i in [0:A.qs.size] do
    for j in [i + 1:A.qs.size] do
      if cts.any (fun c => c.i == i && c.j == j) then continue
      if (A.qs[i]!).qnorm == 0 && (A.qs[j]!).qnorm == 0 then continue
      match nearTouch A faceBoxes (qf l) i j with
      | some n => nearNotes := nearNotes.push n
      | none => pure ()
  if strict then ambiguous := ambiguous ++ nearNotes.filter (·.startsWith "surfaces")
  let excl := cts.map (·.excl)
  let bad := cts.filterMap (·.bad) ++ apexCheck A solids
  let opens := cts.filterMap (·.open_)
  let empty := tree.witness.isNone && tree.und.all fun b => A.anyKept (signsAt A b) excl == some false
  let verdict :=
    if !ambiguous.isEmpty then "ambiguous"
    else if !bad.isEmpty then "nonmanifold"
    else if empty then "empty"
    else if tree.witness.isNone then "ambiguous"
    else if !opens.isEmpty then "ambiguous"
    else if !nearNotes.isEmpty then "sliver"
    else "solid"
  if tree.witness.isNone && !empty then notes := notes.push "no box certified in, and emptiness not proven"
  notes := notes ++ ambiguous ++ bad ++ (if strict then #[] else nearNotes) ++ opens.extract 0 4
  return Json.mkObj [
    ("verdict", verdict),
    ("volume_lo", qstr tree.inVol), ("volume_hi", qstr (tree.inVol + tree.undVol)),
    ("volume_lo_f", toJson (qf tree.inVol)), ("volume_hi_f", toJson (qf (tree.inVol + tree.undVol))),
    ("volume_f", toJson est), ("volume_err", toJson estErr), ("volume", qstr tree.inVol),
    ("boxes", tree.boxes), ("contacts", cts.size), ("carriers", A.qs.size),
    ("probes", toJson probeAns),
    ("notes", toJson notes)]

def run (line : String) : Except String Json := do
  runCurved (← Json.parse line)

end ForgeExact.QRef
