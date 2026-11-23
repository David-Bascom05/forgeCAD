import ForgeExact.Ref.Input
import ForgeExact.Quad.Cert
import ForgeExact.Alg.Sum

/-!
# The curved reference's input: faces as exact quadrics

One JSON object per line (Main.lean sends a line with `"curved"` here). Every number is an f64
written exactly as `[m, e]` (ForgeExact/Ref/Input.lean):

```
{"curved": true, "linear": [m,e], "rule": "strict|snap",
 "solids": [{"op": "new|add|cut|keep",
             "faces": [{"kind": "plane",  "o": P, "z": N,             "lo": P, "hi": P},
                       {"kind": "cyl",    "o": P, "z": A, "r": R,     "lo": P, "hi": P},
                       {"kind": "cone",   "o": P, "z": A, "r": R, "s": S, "c": C, "lo": P, "hi": P},
                       {"kind": "sphere", "o": P, "r": R,             "lo": P, "hi": P}]}]}
```

A solid is the intersection of its faces' inner sides: the harness sends only tools that are
(a plane's `z` is its outward normal; a cylinder's, cone's and sphere's solid is inside them). The
quadrics are written from the stored frames without normalising (README §6.2):

* plane: `z · (p − o)`;
* cylinder about `o + t z`, radius `r`: `|z|² |p − o|² − ((p − o)·z)² − r² |z|²`, exact for any `z`;
* sphere: `|p − o|² − r²`;
* cone, radius `r` at `o`, growing by `s / c` per unit along `z`:
  `c² (|d|² − (d·z)²) − (r c + s (d·z))²` with `d = p − o`, and its nappe `r c + s (d·z) > 0`.
  Here `|z|` is taken as 1 (the stored axis is a unit vector to an ulp), so this cone is the
  kernel's to about `r · 1e-16`.
-/

namespace ForgeExact.QRef

open Lean Ref Quad Alg

inductive Kind
  | plane
  | cyl
  | cone
  | sphere
  deriving BEq, Inhabited, Repr

structure QFace where
  solid : Nat
  index : Nat
  kind : Kind
  o : V3
  z : V3
  r : Rat
  s : Rat
  c : Rat
  box : Box
  deriving Inhabited

/-- `(p − o)ᵀ S (p − o) + L · (p − o) + k`, with `S` given by its six entries. -/
def centred (sxx syy szz sxy sxz syz : Rat) (L : V3) (k : Rat) (o : V3) : Quadric :=
  -- the gradient of the quadratic part at o is 2 S o
  let gx := 2 * sxx * o.x + sxy * o.y + sxz * o.z
  let gy := 2 * syy * o.y + sxy * o.x + syz * o.z
  let gz := 2 * szz * o.z + sxz * o.x + syz * o.y
  let qo := sxx * o.x * o.x + syy * o.y * o.y + szz * o.z * o.z + sxy * o.x * o.y +
    sxz * o.x * o.z + syz * o.y * o.z
  ⟨sxx, syy, szz, sxy, sxz, syz, L.x - gx, L.y - gy, L.z - gz, qo - L.dot o + k⟩

/-- The face's quadric: the solid is where it is negative. -/
def QFace.quadric (f : QFace) : Quadric :=
  let z := f.z
  match f.kind with
  | .plane => centred 0 0 0 0 0 0 z 0 f.o
  | .sphere => centred 1 1 1 0 0 0 ⟨0, 0, 0⟩ (-(f.r * f.r)) f.o
  | .cyl =>
    let n := z.norm2
    centred (n - z.x * z.x) (n - z.y * z.y) (n - z.z * z.z) (-2 * z.x * z.y) (-2 * z.x * z.z)
      (-2 * z.y * z.z) ⟨0, 0, 0⟩ (-(f.r * f.r * n)) f.o
  | .cone =>
    let c2 := f.c * f.c
    let s2 := f.s * f.s
    let k := c2 + s2
    -- c²(|d|² − (d·z)²) − (rc + s d·z)² = c²|d|² − (c² + s²)(d·z)² − 2rcs (d·z) − r²c²
    centred (c2 - k * z.x * z.x) (c2 - k * z.y * z.y) (c2 - k * z.z * z.z) (-2 * k * z.x * z.y)
      (-2 * k * z.x * z.z) (-2 * k * z.y * z.z) ((-2 * f.r * f.c * f.s) * z) (-(f.r * f.r * c2)) f.o

/-- The cone's nappe, as a quadric negative on it: `−(r c + s (p − o)·z)`. -/
def QFace.nappe (f : QFace) : Option Quadric :=
  match f.kind with
  | .cone => some (centred 0 0 0 0 0 0 ((-f.s) * f.z) (-(f.r * f.c)) f.o)
  | _ => none

def num (j : Json) (k : String) : Except String Rat := do dyadic (← j.getObjVal? k)

def faceOf (solid index : Nat) (j : Json) : Except String QFace := do
  let kind ← match ← (← j.getObjVal? "kind").getStr? with
    | "plane" => pure Kind.plane
    | "cyl" => pure Kind.cyl
    | "cone" => pure Kind.cone
    | "sphere" => pure Kind.sphere
    | k => throw s!"a face of kind {k}"
  let o ← v3 (← j.getObjVal? "o")
  let z ← match j.getObjVal? "z" with
    | .ok v => v3 v
    | .error _ => pure ⟨0, 0, 1⟩
  let r := (num j "r").toOption.getD 0
  let s := (num j "s").toOption.getD 0
  let c := (num j "c").toOption.getD 0
  let lo ← v3 (← j.getObjVal? "lo")
  let hi ← v3 (← j.getObjVal? "hi")
  if (kind == .plane || kind == .cyl || kind == .cone) && z.isZero then throw "a face's axis is zero"
  return ⟨solid, index, kind, o, z, r, s, c, ⟨lo, hi⟩⟩

structure QSolid where
  op : String
  faces : Array QFace
  deriving Inhabited

def solidOf (k : Nat) (j : Json) : Except String QSolid := do
  let op ← (← j.getObjVal? "op").getStr?
  let fs ← (← j.getObjVal? "faces").getArr?
  let mut faces : Array QFace := #[]
  for i in [0:fs.size] do
    faces := faces.push (← faceOf k i fs[i]!)
  return ⟨op, faces⟩

end ForgeExact.QRef
