import ForgeExact.Flat
import ForgeCertify.Const
import ForgeCertify.Tele

/-!
# The checker

What the executable runs, and what `Sound.lean` proves sound. Its input is exact: the operands
and the result as reference faces (`Ref.Face`: a plane and loops of rational points, after the
tolerance rule), the operands' faces triangulated (the certificate, completed), and the boolean as
a function of the operands' memberships.

Checks, all exact:
1. `tilesOK` : each face's triangles lie on its plane, are proper, and with its loops make a zero
   chain;
2. `closedOK` : each operand's triangles make a zero chain;
3. `atomsOK` : on every plane (`n.x ≠ 0`), the distinct triangles (`atoms`) are pairwise disjoint;
4. `atomOK` : each atom's witness (its centroid) is off every other surface's plane and lines, and
   every operand triangle off the atom's plane is separated from the atom by a plane;
5. `resultOK` : on every plane, the result's loops and the atoms where the boolean changes
   (`jump`) make a zero chain.
-/

namespace ForgeCertify

open ForgeExact.Ref

/-! ## Data -/

/-- An operand: its faces and, face by face, their triangles. -/
structure Opd where
  faces : List Face
  tris : List (List Tri)
  deriving Inhabited

/-- Its triangles, each with its face's plane. -/
def Opd.soup (o : Opd) : List (Plane × Tri) :=
  (o.faces.zip o.tris).flatMap fun ft => ft.2.map fun t => (ft.1.plane, t)

def facesContain (fs : List Face) (x : V3) : Bool := par (fs.map fun f => f.pierced x)

/-- The reference's membership for a list of faces. -/
def den (fs : List Face) : ForgeExact.Pred V3 := fun x => (Solid.mk "" fs.toArray).contains x = true

/-! ## Triangles up to their vertex order -/

def sort2 (a b : V3) : V3 × V3 := if v3le a b then (a, b) else (b, a)

/-- The vertices in `v3le` order: equal triangles become equal. -/
def canonTri (t : Tri) : Tri :=
  let (p, q) := sort2 t.a t.b
  let (q, r) := sort2 q t.c
  let (p, q) := sort2 p q
  ⟨p, q, r⟩

def nonvert (Q : Plane) : Bool := Q.n.x != 0

def nonzero (n : V3) : Bool := !(n.x == 0 && n.y == 0 && n.z == 0)

/-- Without repeats, order kept from the end. -/
def uniq {α : Type} [DecidableEq α] : List α → List α
  | [] => []
  | a :: l => if a ∈ uniq l then uniq l else a :: uniq l

theorem mem_uniq {α : Type} [DecidableEq α] (a : α) : ∀ l : List α, a ∈ uniq l ↔ a ∈ l
  | [] => by simp [uniq]
  | b :: l => by
    unfold uniq; split
    · rw [mem_uniq a l]; constructor
      · exact fun h => List.mem_cons_of_mem _ h
      · intro h; rcases List.mem_cons.mp h with rfl | h
        · rename_i hb; exact (mem_uniq a l).mp hb
        · exact h
    · simp [mem_uniq a l]

theorem nodup_uniq {α : Type} [DecidableEq α] : ∀ l : List α, (uniq l).Nodup
  | [] => List.nodup_nil
  | b :: l => by
    unfold uniq; split
    · exact nodup_uniq l
    · exact List.nodup_cons.mpr ⟨by assumption, nodup_uniq l⟩

/-- Every operand's triangles. -/
def allSoup (ins : List Opd) : List (Plane × Tri) := ins.flatMap Opd.soup

/-- The planes looked at: the canonical planes, not along `x`, of every triangle and result face. -/
def planes (ins : List Opd) (res : List Face) : List Plane :=
  uniq (((allSoup ins).map (·.1) ++ res.map (·.plane)).filter nonvert |>.map Plane.canon)

/-- The atoms of a plane: its distinct triangles. -/
def atoms (ins : List Opd) (P : Plane) : List Tri :=
  uniq (((allSoup ins).filter fun t => nonvert t.1 && decide (t.1.canon = P)).map fun t => canonTri t.2)

/-! ## Separation -/

/-- `H` is nonnegative on the atom (positive somewhere) and nonpositive on `u`. -/
def sepOK (H : Plane) (τ u : Tri) : Bool :=
  decide (0 ≤ H.eval τ.a) && decide (0 ≤ H.eval τ.b) && decide (0 ≤ H.eval τ.c) &&
    (decide (0 < H.eval τ.a) || decide (0 < H.eval τ.b) || decide (0 < H.eval τ.c)) &&
    decide (H.eval u.a ≤ 0) && decide (H.eval u.b ≤ 0) && decide (H.eval u.c ≤ 0)

def planeThrough (p n : V3) : Plane := ⟨n, n.dot p⟩

def vmin3 (a b c : Rat) : Rat := qmin a (qmin b c)
def vmax3 (a b c : Rat) : Rat := qmax a (qmax b c)

/-- Along direction `d`, a plane at `u`'s extreme: `u` on one side, the atom on the other. -/
def sepAlong (d : V3) (τ u : Tri) : Bool :=
  sepOK ⟨d, vmax3 (d.dot u.a) (d.dot u.b) (d.dot u.c)⟩ τ u ||
    sepOK (Plane.neg ⟨d, vmin3 (d.dot u.a) (d.dot u.b) (d.dot u.c)⟩) τ u

/-- Some separating plane (the separating axes of two triangles, cheapest first: the coordinate
axes, the two planes' normals, each edge square to either plane, each pair of edges). -/
def separated (P Q : Plane) (τ u : Tri) : Bool :=
  let axes : List V3 := [⟨1, 0, 0⟩, ⟨0, 1, 0⟩, ⟨0, 0, 1⟩]
  axes.any (fun d => sepAlong d τ u) || sepAlong Q.n τ u || sepAlong P.n τ u ||
    τ.segs.any (fun s => sepAlong ((s.2 - s.1).cross P.n) τ u) ||
    u.segs.any (fun s => sepAlong ((s.2 - s.1).cross Q.n) τ u) ||
    τ.segs.any (fun s => u.segs.any fun s' => sepAlong ((s.2 - s.1).cross (s'.2 - s'.1)) τ u)

/-- Two triangles of one plane with disjoint interiors in projection: some edge line has one on
each closed side. -/
def sep2 (τ σ : Tri) : Bool :=
  let side (p q r : V3) := o2 (yz p) (yz q) (yz r)
  let lines := τ.segs ++ σ.segs
  lines.any fun s =>
    let f := fun r => side s.1 s.2 r
    (decide (0 ≤ f τ.a) && decide (0 ≤ f τ.b) && decide (0 ≤ f τ.c) &&
      decide (f σ.a ≤ 0) && decide (f σ.b ≤ 0) && decide (f σ.c ≤ 0)) ||
    (decide (f τ.a ≤ 0) && decide (f τ.b ≤ 0) && decide (f τ.c ≤ 0) &&
      decide (0 ≤ f σ.a) && decide (0 ≤ f σ.b) && decide (0 ≤ f σ.c))

/-! ## The checks -/

def onPlane (Q : Plane) (t : Tri) : Bool := Q.eval t.a == 0 && Q.eval t.b == 0 && Q.eval t.c == 0

def proper (Q : Plane) (t : Tri) : Bool := if Q.n.x != 0 then t.D != 0 else t.Dv Q != 0

def tilesOK (o : Opd) : Bool :=
  o.faces.length == o.tris.length &&
    (o.faces.zip o.tris).all fun ft =>
      nonzero ft.1.plane.n && ft.2.all (fun t => onPlane ft.1.plane t && proper ft.1.plane t) &&
        zero1 (faceSegs ft.1 ++ ft.2.flatMap Tri.segs)

def closedOK (o : Opd) : Bool := zero1 (o.soup.flatMap fun t => t.2.segs)

def atomsOK (As : List Tri) : Bool :=
  decide (As.Pairwise fun a b => sep2 a b = true)

def centroid3 (t : Tri) : V3 := (1 / 3 : Rat) * (t.a + t.b + t.c)

/-- The witness is not tied for a triangle `u` off the atom's plane. -/
def witnessOK (w : V3) (Q : Plane) (u : Tri) : Bool :=
  Q.eval w != 0 && yz w != yz u.a && yz w != yz u.b && yz w != yz u.c &&
    (Q.n.x == 0 || (o2 (yz w) (yz u.b) (yz u.c) != 0 && o2 (yz u.a) (yz w) (yz u.c) != 0 &&
      o2 (yz u.a) (yz u.b) (yz w) != 0))

def offP (P : Plane) (t : Plane × Tri) : Bool := !(nonvert t.1 && decide (t.1.canon = P))

/-- A point strictly inside the triangle, by positive weights. -/
def wpoint (t : Tri) (k : Nat × Nat × Nat) : V3 :=
  (1 / ((k.1 + k.2.1 + k.2.2 : Nat) : Rat)) * ((k.1 : Rat) * t.a + (k.2.1 : Rat) * t.b + (k.2.2 : Rat) * t.c)

/-- The witness weights tried in turn. -/
def wcands : List (Nat × Nat × Nat) :=
  [(1, 1, 1), (1, 2, 4), (4, 1, 2), (2, 4, 1), (2, 3, 6), (6, 2, 3), (3, 6, 2), (5, 7, 11), (11, 5, 7),
   (7, 11, 5), (13, 17, 19), (19, 13, 17), (17, 19, 13)]

/-- The first point inside the atom that is tied with no operand triangle off its plane. -/
def witness (ins : List Opd) (P : Plane) (τ : Tri) : V3 :=
  let S := allSoup ins
  match wcands.find? (fun k => let p := wpoint τ k; S.all fun t => !offP P t || witnessOK p t.1 t.2) with
  | some k => wpoint τ k
  | none => centroid3 τ

/-- The atom's checks against every operand triangle off its plane. -/
def atomOK (ins : List Opd) (P : Plane) (τ : Tri) : Bool :=
  let w := witness ins P τ
  (allSoup ins).all fun t => !offP P t || (witnessOK w t.1 t.2 && separated P t.1 τ t.2)

/-- Each operand's membership just beyond the atom along `+x`, from the witness `w`. -/
def beyond (ins : List Opd) (P : Plane) (w : V3) (i : Nat) : Bool :=
  par (((ins.getD i default).soup.filter (offP P)).map fun t => hit t.1 t.2 w)

/-- Each operand's triangles equal to the atom, mod 2. -/
def mult (ins : List Opd) (P : Plane) (τ : Tri) (i : Nat) : Bool :=
  par (((ins.getD i default).soup.filter fun t => nonvert t.1 && decide (t.1.canon = P)).map
    fun t => decide (canonTri t.2 = τ))

/-- Does the boolean change across the atom. -/
def jump (op : List Bool → Bool) (ins : List Opd) (P : Plane) (τ : Tri) : Bool :=
  let w := witness ins P τ
  let bs := vec ins.length fun i => beyond ins P w i
  let ms := vec ins.length fun i => mult ins P τ i
  op (List.zipWith (· ^^ ·) bs ms) ^^ op bs

def resOn (res : List Face) (P : Plane) : List Face :=
  res.filter fun f => nonvert f.plane && decide (f.plane.canon = P)

def resultOK (op : List Bool → Bool) (ins : List Opd) (res : List Face) (P : Plane) (As : List Tri) : Bool :=
  zero1 ((resOn res P).flatMap faceSegs ++ (As.filter (jump op ins P)).flatMap Tri.segs)

/-- A two-operand boolean as the checker's rule. -/
def binOp : ForgeExact.Op → List Bool → Bool
  | .union => fun l => l.getD 0 false || l.getD 1 false
  | .inter => fun l => l.getD 0 false && l.getD 1 false
  | .diff => fun l => l.getD 0 false && !l.getD 1 false

/-- **The checker.** -/
def check (op : List Bool → Bool) (ins : List Opd) (res : List Face) : Bool :=
  op (vec ins.length fun _ => false) == false &&
  ins.all tilesOK && ins.all closedOK &&
  (planes ins res).all fun P =>
    let As := atoms ins P
    atomsOK As && As.all (atomOK ins P) && resultOK op ins res P As

end ForgeCertify
