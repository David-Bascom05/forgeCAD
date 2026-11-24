import ForgeExact.QRef.Poly
import ForgeExact.DecideOnce.QBound

/-!
# The decide-once oracle in the curved reference

With `"oracle": true` on a curved line, the answer gains an `"oracle"` object: the curved
decide-once ledger (`qledgerB`) of the same arrangement, after the same snap. Its bodies are the
tools, each the conjunction of its literals; the history combines them as `historyBF` does.

* `"certified"`: `certified` holds (`curved_exact`: every `ExactK` field holds over ℝ);
  otherwise `"skip"` says why (`carriers` over the cap, `bound` with no inside-`B` certificate
  at any tried radius, `ledger` when `qledgerB` refuses, `form`);
* `"R2"`, `"facets"`, `"edges"`, `"verts"`, `"topes"`: the bounding radius² and the counts;
* `"probes"`: per probe point (exact, not boxed), 1 or −1 for the history's value on its side
  cell by the ledger's `inside` (`witness`: membership), 0 on a carrier, 2 when its cell is not
  a listed tope.
-/

namespace ForgeExact.QRef.Oracle

open Lean ForgeExact.Ref ForgeExact.Quad ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved
  ForgeExact.QRef

def toS (l : Lit) : SForm := .lit l.idx (if l.pos then .pos else .neg)

/-- A tool: the conjunction of its literals. -/
def toolForm (lits : Array Lit) : SForm :=
  match lits.toList with
  | [] => .tt
  | l :: ls => ls.foldl (fun acc l => .and acc (toS l)) (toS l)

def sgnQ (q : Rat) : Sgn := if q > 0 then .pos else if q < 0 then .neg else .zero

/-- The history on the tools' memberships (`historyBF`'s fold). -/
def history (ops : Array String) (ins : Nat → Bool) : Bool := Id.run do
  let mut r := false
  for i in [0:ops.size] do
    let op := ops[i]!
    r := if op == "cut" then r && !ins i else if op == "keep" then r && ins i else r || ins i
  return r

def ceilQ (q : Rat) : Rat := ((q.num + q.den - 1) / q.den : Int)

/-- Radii² to try: the face boxes' corners, from `bCentre`, doubled, then grown. -/
def radii (bb : Box) : List Rat :=
  let c : V3 := ⟨bCentre.1, bCentre.2.1, bCentre.2.2⟩
  let cs := [bb.lo.x, bb.hi.x].flatMap fun x => [bb.lo.y, bb.hi.y].flatMap fun y =>
    [bb.lo.z, bb.hi.z].map fun z => (⟨x, y, z⟩ - c).norm2
  let m := cs.foldl qmax 0
  let r := ceilQ (2 * m) + 1
  [r, 4 * r, 16 * r]

def oracle (A : Arr) (bb : Box) (probes : Array V3) (cap : Nat) : Json := Id.run do
  let skip := fun (why : String) => Json.mkObj [("certified", false), ("skip", why)]
  if A.qs.size > cap then return skip "carriers"
  let qs := A.qs.toList
  let forms := A.solids.toList.map fun s => toolForm s.2
  if !(forms.all (belowB qs.length)) then return skip "form"
  let some R2 := (radii bb).find? fun R2 => (forms.mapM fun F => proposeCert qs F R2).isSome
    | return skip "bound"
  match qledgerB qs forms R2 with
  | none => return skip "ledger"
  | some L =>
    let ok := certified qs forms R2
    let Q := qs ++ [sphereR R2]
    let ops := A.solids.map (·.1)
    let topes := L.topes
    let pr : Array Int := probes.map fun p =>
      let T := Q.map fun q => sgnQ (q.eval p)
      if T.any (· == .zero) then 0
      else if !(topes.contains T) then 2
      else if history ops (fun i => L.inside.contains (T, i)) then 1 else -1
    return Json.mkObj [("certified", ok), ("R2", toString R2),
      ("facets", L.facets.length), ("edges", L.edges.length), ("verts", L.verts.length),
      ("topes", topes.length), ("probes", toJson pr)]

/-- The curved reference's answer, with the oracle's when asked. -/
def run (line : String) : Except String Json := do
  let j ← Json.parse line
  let base ← ForgeExact.QRef.Poly.run line
  if !(j.getObjValAs? Bool "oracle" |>.toOption.getD false) then return base
  let l ← dyadic (← j.getObjVal? "linear")
  let strict := (j.getObjValAs? String "rule" |>.toOption.getD "strict") != "snap"
  let cap := (j.getObjValAs? Nat "oracle_carriers" |>.toOption.getD 10)
  let ss ← (← j.getObjVal? "solids").getArr?
  let mut solids : Array QSolid := #[]
  for k in [0:ss.size] do solids := solids.push (← solidOf k ss[k]!)
  if !strict then solids := (snapFaces l solids).1
  let faces := solids.foldl (fun acc s => acc ++ s.faces) #[]
  if faces.isEmpty || faces.any (·.kind == .torus) then
    return base.setObjVal! "oracle" (Json.mkObj [("certified", false), ("skip", "torus")])
  let bb := faces.foldl (fun b f => b.hull f.box) faces[0]!.box
  let probes ← match j.getObjVal? "probes" with
    | .ok ps => do (← ps.getArr?).mapM v3
    | .error _ => pure #[]
  return base.setObjVal! "oracle" (oracle (arrOf solids) bb probes cap)

end ForgeExact.QRef.Oracle
