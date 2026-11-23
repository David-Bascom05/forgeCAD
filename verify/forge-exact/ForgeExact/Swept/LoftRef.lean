import ForgeExact.Swept.TaperRef

/-!
# The loft reference: ruled lofts between a polygon and a scaled copy (DESIGN.md §3.4)

The ruled loft (ForgeExact/Swept/Loft.lean, `Ruled`) from the sketch-frame polygon `loops` at
frame `F` to the same loops scaled by `λ > 0` about the sketch's origin and moved by `F.v`. Its
point at `s` over `q` is `o + μ(s)·(q_x ex + q_y ey) + s·v` with `μ(s) = 1 − s + s λ`, so its
section at `s` is the region scaled by `μ(s)`: membership is exact (`loftMem`), and every wall is
flat (the walls join parallel edges), so the flat comparison applies. With `λ = 1` it is the prism
(`ruled_translate`).

```
{"linear": .., "kind": "loft", "frame": {.. "v": offset of the second section}, "lambda": [m,e],
 "loops": .., "kernel": .. | "kernel_refused": ..}
```
-/

namespace ForgeExact.Swept.LoftRef

open Lean ForgeExact.Ref ForgeExact.Swept ForgeExact.Swept.Ref ForgeExact.Swept.TaperRef

def loftMem (F : Frame3) (lam : Rat) (loops : List (List (Rat × Rat))) (p : V3) : Bool :=
  let s := F.cs p
  if !(0 < s && s < 1) then false
  else
    let μ := 1 - s + s * lam
    inRegion loops (F.cx p / μ, F.cy p / μ)

def runLoft (j : Json) (l : Rat) : Except String Json := do
  let (F, _, _, loops) ← common j
  let lam ← dyadic (← j.getObjVal? "lambda")
  if lam ≤ 0 then throw "the scale is not positive"
  let valid := inDomain F 0 1 0 loops
  match j.getObjVal? "kernel" with
  | .error _ =>
    let why := (j.getObjValAs? String "kernel_refused").toOption.getD ""
    return Json.mkObj [("verdict", if valid then "refused_valid" else "refused_ok"), ("why", why)]
  | .ok kj =>
    if !valid then return Json.mkObj [("verdict", "built_invalid")]
    let kraw ← rawOf kj
    let mut ps : Array Plane := #[framePlane F 0 0 1 0, framePlane F 0 0 1 1]
    let mut pts : Array V3 := #[]
    for lp in loops do
      let n := lp.length
      for i in [0:n] do
        let a := lp[i]!
        let b := lp[(i + 1) % n]!
        let nrm := perp2 (sub2 b a)
        let c := dot2 nrm a
        -- n·(x, y) = μ(s) c
        ps := ps.push (framePlane F nrm.1 nrm.2 (-(c * (lam - 1))) c)
        pts := pts.push (F.pt a.1 a.2 0) |>.push (F.pt (lam * a.1) (lam * a.2) 1)
    return compare l kraw ps (Box.ofPoints pts) (loftMem F lam loops) none

def run (line : String) : Except String Json := do
  let j ← Json.parse line
  let l ← dyadic (← j.getObjVal? "linear")
  match (← (← j.getObjVal? "kind").getStr?) with
  | "loft" => runLoft j l
  | _ => TaperRef.run line

end ForgeExact.Swept.LoftRef
