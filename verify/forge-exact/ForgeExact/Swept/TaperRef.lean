import ForgeExact.Swept.Ref
import ForgeExact.Swept.Taper

/-!
# The taper reference over ℚ (research/sweep-spec/DESIGN.md §2.2)

Exact over ℚ for profiles of lines whose edges have rational lengths (axis-aligned, Pythagorean
directions): every unit direction, normal and mitre is then rational, and so is every event
height (`Taper.edge_length`, `Taper.contact_distance`: each is the root of a linear function).
A profile with an irrational edge length is out of this executable's scope (the same formulas
over the algebraic numbers decide it; that is the Alg step).

`run` follows the decided rule step by step: from `τ = 0`, the next event in `(τ, D]` is the least
root of the edges' lengths and the corners' contacts; at it, the edges that have gone are dropped
and the section is checked: still simple and apart (edge events only: go on), the outer loop
gone (the roof: the exact set ends there, so the solid cut off at that height is due), a hole gone
(dropped), otherwise a corner has met another edge (walls cross: refused).
-/

namespace ForgeExact.Swept.TaperRef

open ForgeExact.Ref ForgeExact.Swept ForgeExact.Swept.Ref

abbrev Q2 := Rat × Rat

def nsqrt (n : Nat) : Option Nat :=
  let r := n.sqrt
  if r * r == n then some r else none

/-- The square root of a rational, when it is rational. -/
def ratSqrt (q : Rat) : Option Rat :=
  if q < 0 then none
  else match nsqrt q.num.toNat, nsqrt q.den with
    | some a, some b => some ((a : Rat) / (b : Rat))
    | _, _ => none

def sub2 (a b : Q2) : Q2 := (a.1 - b.1, a.2 - b.2)
def add2 (a b : Q2) : Q2 := (a.1 + b.1, a.2 + b.2)
def smul2 (k : Rat) (a : Q2) : Q2 := (k * a.1, k * a.2)
def dot2 (a b : Q2) : Rat := a.1 * b.1 + a.2 * b.2
def perp2 (a : Q2) : Q2 := (-a.2, a.1)

/-- A loop ready to offset: its corners at `τ₀` and their mitres (per unit of δ), with each edge's
unit direction. Corner `i` starts edge `i`. -/
structure OLoop where
  corners : Array Q2
  mitres : Array Q2
  dirs : Array Q2
  deriving Inhabited

/-- A loop's offset structure from its corners (in order, material on the left), or none when an
edge has no rational length or two neighbouring edges run back on each other. -/
def oloop (pts : Array Q2) : Option OLoop := Id.run do
  let n := pts.size
  let mut dirs : Array Q2 := #[]
  for i in [0:n] do
    let d := sub2 pts[(i + 1) % n]! pts[i]!
    match ratSqrt (dot2 d d) with
    | some l => if l == 0 then return none else dirs := dirs.push (smul2 (1 / l) d)
    | none => return none
  let mut mitres : Array Q2 := #[]
  for i in [0:n] do
    let np := perp2 dirs[(i + n - 1) % n]!
    let nn := perp2 dirs[i]!
    let den := 1 + dot2 np nn
    if den == 0 then return none
    mitres := mitres.push (smul2 (1 / den) (add2 np nn))
  return some ⟨pts, mitres, dirs⟩

/-- The corners `δ` further on. -/
def OLoop.at (l : OLoop) (δ : Rat) : Array Q2 :=
  (Array.range l.corners.size).map fun i => add2 l.corners[i]! (smul2 δ l.mitres[i]!)

/-- Twice the signed area. -/
def area2 (ps : Array Q2) : Rat := Id.run do
  let mut s := 0
  for i in [0:ps.size] do
    let a := ps[i]!
    let b := ps[(i + 1) % ps.size]!
    s := s + (a.1 * b.2 - b.1 * a.2)
  return s

/-- The least root in `(lo, hi]` of `c₀ + δ·c₁`, if any. -/
def rootIn (c0 c1 lo hi : Rat) : Option Rat :=
  if c1 == 0 then none
  else
    let r := -c0 / c1
    if lo < r && r ≤ hi then some r else none

def qminO (a : Option Rat) (b : Option Rat) : Option Rat :=
  match a, b with
  | some x, some y => some (qmin x y)
  | some x, none => some x
  | none, y => y

/-- Every candidate event of the loops in `(lo, hi]` (δ measured from where the corners are):
an edge's length reaching zero, a corner's distance to another edge's carrier reaching zero, or a
corner on a carrier coming to the edge's end. The true first event is among them (each is a
linear function of δ, and a contact begins at one of these roots). -/
def candidates (ls : Array OLoop) (sg lo hi : Rat) : Option Rat := Id.run do
  let mut best : Option Rat := none
  for l in ls do
    let n := l.corners.size
    for i in [0:n] do
      let j := (i + 1) % n
      let len0 := dot2 (sub2 l.corners[j]! l.corners[i]!) l.dirs[i]!
      let len1 := sg * dot2 (sub2 l.mitres[j]! l.mitres[i]!) l.dirs[i]!
      best := qminO best (rootIn len0 len1 lo hi)
  for a in [0:ls.size] do
    for b in [0:ls.size] do
      let la := ls[a]!
      let lb := ls[b]!
      for k in [0:la.corners.size] do
        for e in [0:lb.corners.size] do
          let e2 := (e + 1) % lb.corners.size
          if a == b && (k == e || k == e2) then continue
          let u := lb.dirs[e]!
          let nrm := perp2 u
          let rel := sub2 la.corners[k]! lb.corners[e]!
          -- signed distance to the carrier, minus the carrier's own move
          let d0 := dot2 nrm rel
          let d1 := sg * (dot2 nrm la.mitres[k]! - 1)
          best := qminO best (rootIn d0 d1 lo hi)
          -- along the edge, from its start and from its end
          let p0 := dot2 u rel
          let p1 := sg * dot2 u (sub2 la.mitres[k]! lb.mitres[e]!)
          best := qminO best (rootIn p0 p1 lo hi)
          let q0 := dot2 u (sub2 la.corners[k]! lb.corners[e2]!)
          let q1 := sg * dot2 u (sub2 la.mitres[k]! lb.mitres[e2]!)
          best := qminO best (rootIn q0 q1 lo hi)
  return best

/-- The loops' corners with each edge of zero length dropped (its two corners merged). -/
def reduce (pts : Array Q2) : Array Q2 := Id.run do
  let mut out : Array Q2 := #[]
  for p in pts do
    if out.isEmpty || out.back! != p then out := out.push p
  while out.size > 1 && out.back! == out[0]! do out := out.pop
  return out

/-- What the taper comes to, and the steps of its sections: each step its start `τ` and the
loops then. `D`: the whole offset (depth × slope), `sg` = ±1. -/
inductive Outcome
  | ok (steps : Array (Rat × Array OLoop))
  | collapses (τ : Rat) (steps : Array (Rat × Array OLoop))
  | wallsCross (τ : Rat)
  | outOfScope (why : String)

instance : Inhabited Outcome := ⟨.outOfScope ""⟩

def simpleAll (ls : Array (Array Q2)) : Bool :=
  loopsSimple (ls.toList.map (·.toList))

partial def walk (sg D : Rat) (τ : Rat) (ls : Array OLoop) (acc : Array (Rat × Array OLoop)) :
    Outcome :=
  let acc := acc.push (τ, ls)
  match candidates ls sg 0 (D - τ) with
  | none => .ok acc
  | some δ =>
    let τ' := τ + δ
    let pts := ls.map fun l => reduce (l.at (sg * δ))
    -- the outer loop gone: no area left
    if (pts[0]!).size < 3 || area2 pts[0]! ≤ 0 then .collapses τ' acc
    else
      let kept := #[pts[0]!] ++ ((pts.extract 1 pts.size).filter fun h => h.size ≥ 3 && area2 h < 0)
      if !simpleAll kept then .wallsCross τ'
      else
        match kept.mapM oloop with
        | none => .outOfScope "an edge with no rational length"
        | some nl => if τ' ≥ D then .ok (acc.push (τ', nl)) else walk sg D τ' nl acc

/-- The taper from its loops (outer CCW, holes CW, as the kernel normalises them). -/
def outcome (loops : List (List Q2)) (sg D : Rat) : Outcome :=
  match (loops.map List.toArray).toArray.mapM oloop with
  | none => .outOfScope "an edge with no rational length"
  | some ls => walk sg D 0 ls #[]

/-- The section at offset `τ` (0 ≤ τ ≤ D) of a taper that builds. -/
def sectionAt (steps : Array (Rat × Array OLoop)) (sg τ : Rat) : List (List Q2) := Id.run do
  let mut cur := steps[0]!
  for st in steps do
    if st.1 ≤ τ then cur := st
  return cur.2.toList.map fun l => (l.at (sg * (τ - cur.1))).toList

/-! ## The driver's taper -/

open Lean

/-- Outer loop anticlockwise, holes clockwise (the kernel's normal form, decided here). -/
def oriented (loops : List (List Q2)) : List (List Q2) :=
  match loops with
  | [] => []
  | o :: hs =>
    let fix (want : Bool) (l : List Q2) := if (area2 l.toArray > 0) == want then l else l.reverse
    fix true o :: hs.map (fix false)

/-- Membership of the taper that builds: the coordinates, the range, then the section at the
height's offset. `side`: +1 when the taper goes along the travel (`lo = 0`), −1 against it. -/
def taperMem (F : Frame3) (lo hi t sg : Rat) (steps : Array (Rat × Array OLoop)) (p : V3) : Bool :=
  let s := F.cs p
  if !(lo < s && s < hi) then false
  else
    let τ := t * qabs s
    inRegion (sectionAt steps sg τ) (F.cx p, F.cy p)

/-- The world plane of `a·x + b·y + c·s = k` in frame coordinates. -/
def framePlane (F : Frame3) (a b c k : Rat) : Plane :=
  let A := F.ey.cross F.v
  let B := F.v.cross F.ex
  let C := F.ex.cross F.ey
  let n := (1 / F.det) * (a * A + b * B + c * C)
  ⟨n, k + n.dot F.o⟩

/-- Every carrier of the taper: the two caps and each step's walls (an edge's carrier at offset
`τ` is `n·(q − c) = sg (τ − τ₀)`, with `τ = t |s|`), and the box of its corners. -/
def taperPlanes (F : Frame3) (lo hi t sg : Rat) (steps : Array (Rat × Array OLoop)) (D : Rat) :
    Array Plane × Box := Id.run do
  let side : Rat := if hi > 0 then 1 else -1
  let mut ps : Array Plane := #[framePlane F 0 0 1 lo, framePlane F 0 0 1 hi]
  let mut pts : Array V3 := #[]
  for k in [0:steps.size] do
    let (τ0, ls) := steps[k]!
    let τ1 := if k + 1 < steps.size then steps[k + 1]!.1 else D
    for l in ls do
      for i in [0:l.corners.size] do
        let n := perp2 l.dirs[i]!
        let c := l.corners[i]!
        -- n·(x, y) − sg·t·side·s = n·c − sg·τ0
        ps := ps.push (framePlane F n.1 n.2 (-(sg * t * side)) (dot2 n c - sg * τ0))
      for τ in [τ0, τ1] do
        let sh := if t == 0 then 0 else side * τ / t
        for q in l.at (sg * (τ - τ0)) do pts := pts.push (F.pt q.1 q.2 sh)
  return (ps, Box.ofPoints pts)

def runTaper (j : Json) (l : Rat) : Except String Json := do
  let (F, lo, hi, loops) ← common j
  -- the slope as the kernel takes it: sin / cos of |angle| (one rounding each, stated)
  let sn ← dyadic (← j.getObjVal? "sin")
  let cs ← dyadic (← j.getObjVal? "cos")
  if cs ≤ 0 || sn ≤ 0 then throw "the taper's angle is not in (0, 90°)"
  let t := sn / cs
  let sg ← dyadic (← j.getObjVal? "sign")
  if !(lo == 0 || hi == 0) then throw "a taper starts at its sketch"
  let depth := hi - lo
  let D := depth * t
  let valid := inDomain F lo hi l loops
  let kernel := j.getObjVal? "kernel"
  let refusedWhy := (j.getObjValAs? String "kernel_refused").toOption.getD ""
  let refused := kernel.toOption.isNone
  if !valid then
    return Json.mkObj [("verdict", if refused then "refused_ok" else "built_invalid")]
  match outcome (oriented loops) sg D with
  | .outOfScope why => return Json.mkObj [("verdict", "out_of_scope"), ("why", why)]
  | .collapses τ steps =>
    -- the outline closes at τ (the roof, the master's ruling): the exact set ends there, so the
    -- due result is the solid cut off at τ / t, with a lint, not a refusal
    match kernel with
    | .error _ =>
      return Json.mkObj [("verdict", "refused_valid"), ("closes", "roof"),
        ("at", toString (τ / t)), ("at_f", toJson (qf (τ / t))), ("why", refusedWhy)]
    | .ok kj =>
      let kraw ← rawOf kj
      let side : Rat := if hi > 0 then 1 else -1
      let (lo', hi') := if side > 0 then (lo, τ / t) else (-(τ / t), hi)
      let (ps, box) := taperPlanes F lo' hi' t sg steps τ
      return compare l kraw ps box (taperMem F lo' hi' t sg steps) none
        #[s!"slabs: {steps.size}", s!"closes at {qf (τ / t)} (a roof)"]
  | .wallsCross τ =>
    return Json.mkObj [("verdict", if refused then "refused_ok" else "built_invalid"),
      ("due", "walls_cross"), ("at", toString (τ / t)), ("at_f", toJson (qf (τ / t))),
      ("why", refusedWhy)]
  | .ok steps =>
    match kernel with
    | .error _ =>
      return Json.mkObj [("verdict", "refused_valid"), ("why", refusedWhy),
        ("slabs", steps.size)]
    | .ok kj =>
      let kraw ← rawOf kj
      let (ps, box) := taperPlanes F lo hi t sg steps D
      return compare l kraw ps box (taperMem F lo hi t sg steps) none
        #[s!"slabs: {steps.size}"]

def run (line : String) : Except String Json := do
  let j ← Json.parse line
  let l ← dyadic (← j.getObjVal? "linear")
  match (← (← j.getObjVal? "kind").getStr?) with
  | "prism" => runPrism j l
  | "taper" => runTaper j l
  | k => throw s!"unknown kind {k}"

end ForgeExact.Swept.TaperRef
