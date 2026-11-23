import ForgeExact.Ref.Input

/-!
# The fillet spec's executable model: each rolled blend judged exactly

`fillet-check` reads the lines `verify/fillet-spec` writes (one edge rolled alone by the kernel, every
f64 exact as `[m, e]`) and judges each in exact rational arithmetic against the spec:

* **the kind** (`specKind`): what the spec's theorems give for the two faces' pair.
  * Plane against plane, a plane parallel to a cylinder's axis, and parallel cylinders give a line
    crease and a **cylinder** (`PlanePair.closing_iff`, `q_eq_cyl`).
  * A plane square to the axis of a cylinder, cone or torus, a plane and a sphere, and coaxial
    surfaces of revolution give a circle crease and a **torus** (`boss_closing`, `hole_closing`,
    `closing_rev`), **of any major radius**: spindle tori included, and the **sphere** at major 0
    (`pivot_sphere`).
  * Coaxial cylinders and parallel planes give no crease.
  * Every other pair gives a procedural **pipe**.
* **the geometry**: the blend's radius is `r` exactly, and its axis is the crease's direction. Each
  sampled ball centre lies on both offset surfaces, and each spring point on its face, with
  `|c − s| = r`, all within `δ` (planes, cylinders and spheres; other faces are judged by kind only).

Directions are parallel or square under the spec's angular snap `τₐ = 1e-8`, the kernel's
`ANGULAR · 1e3`. Every comparison is exact: a distance against `δ` is decided by comparing squares,
so no square root is taken.
-/

namespace ForgeExact.Fillet.Check

open Lean ForgeExact.Ref

structure Q3 where
  x : Rat
  y : Rat
  z : Rat
  deriving Inhabited

namespace Q3
def sub (a b : Q3) : Q3 := ⟨a.x - b.x, a.y - b.y, a.z - b.z⟩
def add (a b : Q3) : Q3 := ⟨a.x + b.x, a.y + b.y, a.z + b.z⟩
def smul (s : Rat) (a : Q3) : Q3 := ⟨s * a.x, s * a.y, s * a.z⟩
def dot (a b : Q3) : Rat := a.x * b.x + a.y * b.y + a.z * b.z
def cross (a b : Q3) : Q3 := ⟨a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x⟩
def n2 (a : Q3) : Rat := dot a a
end Q3

/-- The spec's snaps: `τ` for lengths (tol::LINEAR), `τₐ` for directions. -/
def tau : Rat := mkRat 1 100000
def tauA : Rat := mkRat 1 100000000
/-- The differential test's stated bound on the kernel's closed forms (mm). -/
def delta : Rat := mkRat 1 1000000000

def par (a b : Q3) : Bool := (a.cross b).n2 ≤ tauA * tauA * a.n2 * b.n2
def perp (a b : Q3) : Bool := (a.dot b) * (a.dot b) ≤ tauA * tauA * a.n2 * b.n2

/-- The squared distance of `p` from the axis through `o` along `z`. -/
def axisD2 (o z p : Q3) : Rat := ((p.sub o).cross z).n2 / z.n2

def onAxis (o z p : Q3) : Bool := axisD2 o z p ≤ tau * tau

inductive Surf where
  | plane (o z : Q3)
  | cyl (o z : Q3) (r : Rat)
  | cone (o z : Q3) (r s c : Rat)
  | sphere (o : Q3) (r : Rat)
  | torus (o z : Q3) (major minor : Rat)
  | pipe
  deriving Inhabited

def Surf.axis : Surf → Option (Q3 × Q3)
  | .cyl o z _ | .cone o z _ _ _ | .torus o z _ _ => some (o, z)
  | _ => none

def Surf.name : Surf → String
  | .plane .. => "plane" | .cyl .. => "cylinder" | .cone .. => "cone" | .sphere .. => "sphere"
  | .torus .. => "torus" | .pipe => "pipe"

/-- What the spec's theorems make of a pair of faces. -/
def specKind (a b : Surf) : String :=
  let coax (o z : Q3) (s : Surf) : Bool := match s.axis with
    | some (o', z') => par z z' && onAxis o z o'
    | none => match s with
      | .sphere c _ => onAxis o z c
      | _ => false
  match a, b with
  | .plane _ n, .plane _ m => if par n m then "none" else "cylinder"
  | .plane _ n, .cyl _ z _ | .cyl _ z _, .plane _ n =>
      if perp n z then "cylinder" else if par n z then "torus" else "pipe"
  | .plane _ n, .cone _ z .. | .cone _ z .., .plane _ n
  | .plane _ n, .torus _ z .. | .torus _ z .., .plane _ n => if par n z then "torus" else "pipe"
  | .plane .., .sphere .. | .sphere .., .plane .. => "torus"
  | .cyl o z _, .cyl o' z' _ =>
      if par z z' then (if onAxis o z o' then "none" else "cylinder") else "pipe"
  | .pipe, _ | _, .pipe => "pipe"
  | s, t => match s.axis with
    | some (o, z) => if coax o z t then "torus" else "pipe"
    | none => match t.axis with
      | some (o, z) => if coax o z s then "torus" else "pipe"
      | none => "pipe"

/-- `b √n ≤ a`, decided exactly (`n ≥ 0`). -/
def mulSqrtLe (b n a : Rat) : Bool :=
  if b ≤ 0 then (if a ≥ 0 then true else a * a ≤ b * b * n)
  else a ≥ 0 && b * b * n ≤ a * a

/-- `a ≤ b √n`. -/
def leMulSqrt (a b n : Rat) : Bool := mulSqrtLe (-b) n (-a)

/-- `|v − t| ≤ δ` for `v² = v2`, `v ≥ 0`, decided on squares. -/
def nearRoot (v2 t : Rat) : Bool :=
  let lo := t - delta
  (lo ≤ 0 || lo * lo ≤ v2) && v2 ≤ (t + delta) * (t + delta)

/-- The ball centre `c` stands at signed distance `d` from the surface, within δ (along its
natural normal; for a revolved surface, `d` grows its radius). `none` where not judged here. -/
def onOffset (s : Surf) (d : Rat) (c : Q3) : Option Bool :=
  match s with
  | .plane o z =>
      -- (d − δ)|z| ≤ z·(c − o) ≤ (d + δ)|z|
      let h := z.dot (c.sub o)
      some (mulSqrtLe (d - delta) z.n2 h && leMulSqrt h (d + delta) z.n2)
  | .cyl o z r => some (nearRoot (axisD2 o z c) (r + d))
  | .sphere o r => some (nearRoot (c.sub o).n2 (r + d))
  | _ => none

def onSurf (s : Surf) (p : Q3) : Option Bool := onOffset s 0 p

def q3 (j : Json) : Except String Q3 := do
  let v ← v3 j
  return ⟨v.x, v.y, v.z⟩

def num (j : Json) : Except String Rat := dyadic j

def surf (j : Json) : Except String Surf := do
  let t ← (← j.getObjVal? "t").getStr?
  let g (k : String) : Except String Q3 := do q3 (← j.getObjVal? k)
  let n (k : String) : Except String Rat := do num (← j.getObjVal? k)
  match t with
  | "plane" => return .plane (← g "o") (← g "z")
  | "cylinder" => return .cyl (← g "o") (← g "z") (← n "r")
  | "cone" => return .cone (← g "o") (← g "z") (← n "r") (← n "sin") (← n "cos")
  | "sphere" => return .sphere (← g "o") (← n "r")
  | "torus" => return .torus (← g "o") (← g "z") (← n "major") (← n "minor")
  | _ => return .pipe

/-- The judgement of one line: the spec's kind, the kernel's, and what disagrees. -/
def judge (j : Json) : Except String Json := do
  let src ← (← j.getObjVal? "src").getStr?
  let edge ← (← j.getObjVal? "edge").getNat?
  let r ← num (← j.getObjVal? "r")
  let base := [("src", Json.str src), ("edge", toJson edge)]
  if let .ok why := j.getObjVal? "refused" then
    return Json.mkObj (base ++ [("refused", why)])
  let a ← surf (← j.getObjVal? "a")
  let b ← surf (← j.getObjVal? "b")
  let da ← num (← j.getObjVal? "da")
  let db ← num (← j.getObjVal? "db")
  let radius ← num (← j.getObjVal? "radius")
  let k ← surf (← j.getObjVal? "surf")
  let centres ← (← (← j.getObjVal? "centres").getArr?).mapM q3
  let springs ← (← (← j.getObjVal? "springs").getArr?).mapM fun s => do
    (← s.getArr?).mapM q3
  let spec := specKind a b
  let mut bad : Array String := #[]
  -- the kind
  let kk := k.name
  let kindOk := spec == kk || (spec == "torus" && kk == "sphere")
  unless kindOk do bad := bad.push s!"kind: spec {spec}, kernel {kk}"
  -- the radius and the offsets
  unless radius == r && (da == r || da == -r) && (db == r || db == -r) do
    bad := bad.push "radius: the ball is not r"
  match k with
  | .cyl _ z kr =>
      unless kr == r do bad := bad.push "cylinder's radius is not r"
      let dir := match a, b with
        | .plane _ n, .plane _ m => some (n.cross m)
        | .plane .., .cyl _ w _ | .cyl _ w _, .plane .. | .cyl _ w _, .cyl .. => some w
        | _, _ => none
      if let some t := dir then
        unless par z t do bad := bad.push "cylinder's axis is not the crease's"
  | .torus _ z _ minor =>
      unless minor == r do bad := bad.push "torus's minor radius is not r"
      let ax := (a.axis <|> b.axis).map (·.2) <|> match a, b with
        | .plane _ n, _ | _, .plane _ n => some n
        | _, _ => none
      if let some t := ax then
        unless par z t do bad := bad.push "torus's axis is not the faces' axis"
  | _ => pure ()
  for c in centres do
    for (s, d, nm) in [(a, da, "a"), (b, db, "b")] do
      if let some false := onOffset s d c then
        bad := bad.push s!"centre off the offset of face {nm}"
  for (sp, s) in springs.toList.zip [a, b] do
    for (p, c) in sp.toList.zip centres.toList do
      if let some false := onSurf s p then bad := bad.push "spring point off its face"
      unless nearRoot (p.sub c).n2 r do bad := bad.push "spring point not r from its centre"
  let why := bad.toList.eraseDups
  return Json.mkObj (base ++ [("spec", Json.str spec), ("kernel", Json.str kk),
    ("ok", Json.bool why.isEmpty), ("why", toJson why)])

end ForgeExact.Fillet.Check
