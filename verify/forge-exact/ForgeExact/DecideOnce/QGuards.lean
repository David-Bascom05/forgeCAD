import ForgeExact.DecideOnce.QBound

/-!
# Non-vacuity: the certificate holds on representative inputs

`curved_exact_facets` (forge-curved) is stated for every **certified** input (`certified`). This
file shows the hypothesis is not vacuous. The representative curved inputs are certified:

| input | carriers | bodies |
|---|---|---|
| box − cylinder | the box `[−1, 1]³`, the cylinder `x² + y² = 1/4`, its two axial seams | box; cylinder capped by the box's `z` planes |
| plate with a hole | the plate `[−1, 1]² × [0, 1/4]`, the cylinder, its seams | plate; capped cylinder |
| sphere ∩ box | the box, the sphere of radius `3/2`, its seams `x = 0`, `y = 0`, `z = 0` | box; ball |
| cone | `x² + y² = z²`, its two axial seams, the nappe plane `z = 0`, the cap `z = 1` | the cone between `z = 0` and `z = 1` |
| seamed cylinder | the cylinder, its seams, the caps `z = ±1` | the capped cylinder |

`curvedGuards` evaluates `certified` on each. The compiled `curved-guards` executable runs all five
in CI (forge-exact.yml), and fails unless each is certified. The cheapest is also a `#guard` here,
so the plain build checks one too. The interpreted check of all five takes minutes.
-/

namespace ForgeExact.DecideOnce.Curved.Guards

open ForgeExact ForgeExact.Quad ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved

def pl (a b c d : Rat) : Quadric := planeQ a b c d

/-- The box `[−1, 1]³` and its formula. -/
def box : List Quadric :=
  [pl 1 0 0 1, pl 1 0 0 (-1), pl 0 1 0 1, pl 0 1 0 (-1), pl 0 0 1 1, pl 0 0 1 (-1)]
def boxF : SForm :=
  .and (.lit 0 .pos) (.and (.lit 1 .neg) (.and (.lit 2 .pos) (.and (.lit 3 .neg)
    (.and (.lit 4 .pos) (.lit 5 .neg)))))
/-- The cylinder `x² + y² = 1/4` and its two axial seams (QSep: `y = 0` and `−x = 0`). -/
def cyl : Quadric := ⟨1, 1, 0, 0, 0, 0, 0, 0, 0, -1/4⟩
def seams : List Quadric := [pl 0 1 0 0, pl (-1) 0 0 0]
/-- The cylinder capped by the planes at indices 4 and 5. -/
def cylCapF : SForm := .and (.lit 6 .neg) (.and (.lit 4 .pos) (.lit 5 .neg))

def boxMinusCyl : List Quadric × List SForm := (box ++ [cyl] ++ seams, [boxF, cylCapF])
def plateHole : List Quadric × List SForm :=
  ([pl 1 0 0 1, pl 1 0 0 (-1), pl 0 1 0 1, pl 0 1 0 (-1), pl 0 0 1 0, pl 0 0 1 (-1/4)] ++ [cyl] ++
    seams, [boxF, cylCapF])
def sphereBox : List Quadric × List SForm :=
  (box ++ [⟨1, 1, 1, 0, 0, 0, 0, 0, 0, -9/4⟩, pl 1 0 0 0, pl 0 1 0 0, pl 0 0 1 0], [boxF, .lit 6 .neg])
def cone : List Quadric × List SForm :=
  ([⟨1, 1, -1, 0, 0, 0, 0, 0, 0, 0⟩] ++ seams ++ [pl 0 0 1 0, pl 0 0 1 (-1)],
   [.and (.lit 0 .neg) (.and (.lit 3 .pos) (.lit 4 .neg))])
def seamedCyl : List Quadric × List SForm :=
  ([cyl] ++ seams ++ [pl 0 0 1 1, pl 0 0 1 (-1)], [.and (.lit 0 .neg) (.and (.lit 3 .pos) (.lit 4 .neg))])

/-- The bounding sphere used: `|p − c|² < 16`. -/
def R2 : Rat := 16

/-- **The five representative inputs, each certified or not.** -/
def curvedGuards : List (String × Bool) :=
  [("box - cylinder", certified boxMinusCyl.1 boxMinusCyl.2 R2),
   ("plate with a hole", certified plateHole.1 plateHole.2 R2),
   ("sphere ∩ box", certified sphereBox.1 sphereBox.2 R2),
   ("cone", certified cone.1 cone.2 R2),
   ("seamed cylinder", certified seamedCyl.1 seamedCyl.2 R2)]

-- the cone is certified (about 20 s interpreted)
#guard certified cone.1 cone.2 R2

end ForgeExact.DecideOnce.Curved.Guards
