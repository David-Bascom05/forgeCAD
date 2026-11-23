import Sheet.Checker
import Sheet.Region

/-!
# The oracle: the Lean model run on what the Rust kernel published

Reads, on stdin, records written by kernel/forge-sheet/tests/lean_diff.rs (every number an
`f64`'s bits in hex, so read exactly):

```
sheet NAME
t T
wall ID  o.x o.y o.z  ex.x ex.y ex.z  ey.x ey.y ey.z  ez.x ez.y ez.z  dev.o.x dev.o.y dev.x.x dev.x.y
bend JOINT A B  ANGLE R BA S LEN  dev.o.x dev.o.y dev.x.x dev.x.y
hem JOINT                   (that joint is a hem)
flat0  o ex ey ez          (12 numbers: the flat build's frame of the first wall)
dev0   o.x o.y x.x x.y     (the first wall's developed place)
vert   f.x f.y f.z  v.x v.y v.z   (a flat vertex and its folded namesake)
end
region NAME
comp                        (a component: its outer loop, then its holes)
loop N                      (then N edges, from their start point)
edge X Y line
edge X Y arc CX CY R CCW
query X Y RUST              (RUST: 1 if the kernel's `holds` said inside)
end
```

and prints, per sheet, each condition's verdict, and per region the queries where the kernel's
answer differs from `inLoop`'s, with the point.
-/

namespace Sheet.Oracle

open Sheet.Checker Sheet.Region

/-- The exact value of an `f64` from its bits; `none` for an infinity or a NaN. -/
def ofBits (b : ℕ) : Option ℚ :=
  let s : ℕ := b / 2 ^ 63 % 2
  let e : ℕ := b / 2 ^ 52 % 2 ^ 11
  let m : ℕ := b % 2 ^ 52
  if e = 2047 then none
  else
    let mag : ℚ := if e = 0 then (m : ℚ) / 2 ^ 1074
      else (((m + 2 ^ 52 : ℕ) : ℚ)) * (2 : ℚ) ^ ((e : ℤ) - 1075)
    some (if s = 1 then -mag else mag)

def hexVal (c : Char) : Option ℕ :=
  if '0' ≤ c ∧ c ≤ '9' then some (c.toNat - '0'.toNat)
  else if 'a' ≤ c ∧ c ≤ 'f' then some (c.toNat - 'a'.toNat + 10)
  else none

def parseHex (s : String) : Option ℕ :=
  s.foldl (fun acc c => acc.bind fun a => (hexVal c).map fun v => 16 * a + v) (some 0)

def num (s : String) : Except String ℚ :=
  match parseHex s >>= ofBits with
  | some q => .ok q
  | none => .error s!"not a finite f64: {s}"

def nums (ws : List String) : Except String (List ℚ) := ws.mapM num

def v3 : List ℚ → Except String V3
  | [a, b, c] => .ok (a, b, c)
  | _ => .error "want 3 numbers"

def frameOf : List ℚ → Except String Frame
  | [a, b, c, d, e, f, g, h, i, j, k, l] => .ok ⟨(a, b, c), (d, e, f), (g, h, i), (j, k, l)⟩
  | _ => .error "want 12 numbers"

def placeOf : List ℚ → Except String Place
  | [a, b, c, d] => .ok ⟨(a, b), (c, d)⟩
  | _ => .error "want 4 numbers"

def words (l : String) : List String := (l.splitOn " ").filter (· ≠ "")

def emptyFrame : Frame := ⟨(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1)⟩

/-- Parse one sheet's records. -/
def parseSheet (ls : List String) : Except String Input := do
  let mut inp : Input := ⟨0, [], [], emptyFrame, ⟨(0, 0), (1, 0)⟩, [], []⟩
  for l in ls do
    match words l with
    | ["t", x] => inp := { inp with t := ← num x }
    | "wall" :: id :: rest =>
      let ns ← nums rest
      let fr ← frameOf (ns.take 12)
      let pl ← placeOf (ns.drop 12)
      inp := { inp with walls := inp.walls ++ [⟨id.toNat!, fr, pl⟩] }
    | "bend" :: j :: a :: b :: rest =>
      match ← nums rest with
      | [ang, r, ba, s, len, o1, o2, x1, x2] =>
        inp := { inp with bends := inp.bends ++
          [⟨j.toNat!, a.toNat!, b.toNat!, ang, r, ba, s, len, ⟨(o1, o2), (x1, x2)⟩⟩] }
      | _ => throw "bend: want 9 numbers"
    | ["hem", j] => inp := { inp with hems := inp.hems ++ [j.toNat!] }
    | "flat0" :: rest => inp := { inp with flat0 := ← frameOf (← nums rest) }
    | "dev0" :: rest => inp := { inp with dev0 := ← placeOf (← nums rest) }
    | "vert" :: rest =>
      let ns ← nums rest
      inp := { inp with verts := inp.verts ++ [(← v3 (ns.take 3), ← v3 (ns.drop 3))] }
    | [] => pure ()
    | w => throw s!"unknown record {w}"
  return inp

/-- A component: its outer loop, then its holes. -/
abbrev Comp := List Loop

def holdsComp (c : Comp) (p : P) : Bool :=
  match c with
  | [] => false
  | o :: hs => inLoop p o && !hs.any (inLoop p)

structure RegionIn where
  comps : List Comp
  queries : List (P × Bool)

def parseRegion (ls : List String) : Except String RegionIn := do
  let mut comps : List Comp := []
  let mut queries : List (P × Bool) := []
  for l in ls do
    match words l with
    | ["comp"] => comps := comps ++ [[]]
    | ["loop", _] =>
      match comps.getLast? with
      | some c => comps := comps.dropLast ++ [c ++ [⟨[], []⟩]]
      | none => throw "loop before comp"
    | "edge" :: x :: y :: kind =>
      let p : P := (← num x, ← num y)
      let arc ← match kind with
        | ["line"] => pure none
        | ["arc", cx, cy, r, ccw] => pure (some ((← num cx, ← num cy), ← num r, ccw == "1"))
        | _ => throw "edge kind"
      match comps.getLast? with
      | some c =>
        match c.getLast? with
        | some lp => comps := comps.dropLast ++ [c.dropLast ++ [⟨lp.pts ++ [p], lp.arcs ++ [arc]⟩]]
        | none => throw "edge before loop"
      | none => throw "edge before comp"
    | ["query", x, y, r] => queries := queries ++ [((← num x, ← num y), r == "1")]
    | [] => pure ()
    | w => throw s!"unknown record {w}"
  return ⟨comps, queries⟩

def showQ (q : ℚ) : String :=
  let f : Float := Float.ofInt q.num / Float.ofNat q.den
  toString f

def verdictStr : Verdict → String
  | .pass => "pass"
  | .fail w => s!"FAIL ({w})"
  | .undecided w => s!"undecided ({w})"

/-- Run every block of the input; one output line per sheet condition and per region. -/
def run (input : String) : List String := Id.run do
  let ls := input.splitOn "\n"
  let mut out : List String := []
  let mut block : List String := []
  let mut head : List String := []
  for l in ls do
    match words l with
    | ["end"] =>
      match head with
      | ["sheet", name] =>
        match parseSheet block with
        | .ok inp =>
          for (what, v) in check inp do
            out := out ++ [s!"sheet {name} {what}: {verdictStr v}"]
        | .error e => out := out ++ [s!"sheet {name}: parse error {e}"]
      | ["region", name] =>
        match parseRegion block with
        | .ok r =>
          let bad := r.queries.filter fun (p, rust) => r.comps.any (holdsComp · p) != rust
          out := out ++ [s!"region {name}: {r.queries.length} queries, {bad.length} disagree"]
          for (p, rust) in bad do
            out := out ++ [s!"  disagree at ({showQ p.1}, {showQ p.2}): kernel {rust}, model {!rust}"]
        | .error e => out := out ++ [s!"region {name}: parse error {e}"]
      | _ => pure ()
      block := []
      head := []
    | w@("sheet" :: _) | w@("region" :: _) => head := w; block := []
    | _ => block := block ++ [l]
  return out

end Sheet.Oracle
