import Lean.Data.Json
import ForgeExact.Ops.Hole
import ForgeExact.Ref.Input

/-!
# The hole's exact reference: L2

research/ops-spec/design.md §2. The spec's `profile` (Ops/Hole.lean) is executable over ℚ. The
driver `forge-hole-ref` takes a hole as the kernel sized it, and the frusta of the kernel's own
revolve profile (forge-model hole.rs `Hole::revolve`), every number an f64 written exactly as
`[m, e]` (ForgeExact/Ref/Input.lean). It answers whether they are the spec's, exactly.

```
{"r": d, "head": {"kind": "plain"} | {"kind": "cbore", "Rc": d, "dc": d}
                | {"kind": "csink", "Rc": d, "dk": d},
 "end": {"kind": "blind", "D": d, "p": d | null} | {"kind": "thru", "L": d},
 "kernel": [[r0, r1, z0, z1], ...]}
```

A through hole is the spec's half-infinite wall truncated at the kernel's depth `L`
(`HoleSolid.truncate`; H1 says it cuts what the wall cuts when `L` clears the part, which
`kernel_thru_clear` checks). The answer: `{"same": b, "wf": b, "ref": [[r0, r1, z0, z1], ...]}`
with `wf` the kernel's size checks as the spec states them (H3 `profile_wf`'s hypotheses) and
`ref` the spec's frusta as `p/q`.
-/

namespace ForgeExact.Ops.HoleRef

open Lean ForgeExact.Ref

def q (j : Json) (k : String) : Except String Rat := do dyadic (← j.getObjVal? k)

def head (j : Json) : Except String (Head Rat) := do
  match ← (← j.getObjVal? "kind").getStr? with
  | "plain" => return .plain
  | "cbore" => return .cbore (← q j "Rc") (← q j "dc")
  | "csink" => return .csink (← q j "Rc") (← q j "dk")
  | k => throw s!"no head {k}"

/-- The end, and for a through hole the kernel's depth. -/
def hend (j : Json) : Except String (HoleEnd Rat × Option Rat) := do
  match ← (← j.getObjVal? "kind").getStr? with
  | "blind" =>
    let p ← match j.getObjVal? "p" with
      | .ok Json.null => pure none
      | .ok v => some <$> dyadic v
      | .error _ => pure none
    return (.blind (← q j "D") p, none)
  | "thru" => return (.thru, some (← q j "L"))
  | k => throw s!"no end {k}"

def frustum (j : Json) : Except String (Rat × Rat × Rat × Rat) := do
  let a ← j.getArr?
  if a.size != 4 then throw "a frustum is [r0, r1, z0, z1]"
  return (← dyadic a[0]!, ← dyadic a[1]!, ← dyadic a[2]!, ← dyadic a[3]!)

/-- The kernel's size checks, as the spec states them (`Head.ok`, `HoleEnd.ok`), decided. -/
def wf (r : Rat) (hd : Head Rat) (e : HoleEnd Rat) : Bool :=
  decide (0 < r) &&
  (match hd with
    | .plain => true
    | .cbore Rc dc => decide (0 < dc) && decide (r < Rc)
    | .csink Rc dk => decide (0 < dk) && decide (r < Rc)) &&
  (match e with
    | .blind D p => decide (hd.seat < D) && (match p with | some p => decide (0 < p) | none => true)
    | .thru => true)

def ratStr (x : Rat) : String := if x.den == 1 then s!"{x.num}" else s!"{x.num}/{x.den}"

def run (line : String) : Except String Json := do
  let j ← Json.parse line
  let r ← q j "r"
  let hd ← head (← j.getObjVal? "head")
  let (e, L) ← hend (← j.getObjVal? "end")
  let kernel ← (← (← j.getObjVal? "kernel").getArr?).toList.mapM frustum
  let H := profile r hd e
  let H := match L with | some L => H.truncate L | none => H
  let spec := H.pieces.map fun f => (f.r₀, f.r₁, f.z₀, f.z₁)
  let thruOk := match L with | some L => decide (hd.seat < L) | none => true
  return Json.mkObj [
    ("same", toJson (spec == kernel)),
    ("wf", toJson (wf r hd e && thruOk)),
    ("ref", toJson (spec.map fun (a, b, c, d) => [ratStr a, ratStr b, ratStr c, ratStr d]))]

end ForgeExact.Ops.HoleRef
