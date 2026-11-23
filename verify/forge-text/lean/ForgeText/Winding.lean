import ForgeText.Seg

/-!
# Which loop lies inside which (kernel/forge-sketch/src/winding.rs)

The rule: loop `A` lies inside loop `B` (the two neither cross nor touch) iff the winding
number of `B` about a point of `A` is not zero. The winding number is the signed count of
crossings of the ray from the point along +x, each end classed by the half-open rule (above the
point's height, or not). For a line the crossing is decided by the sign of an orientation
determinant (exact: forge_math's orient2d on floats; here on integers). A curve's count is the
sum over pieces of it split until each piece's control hull is wholly left (0), wholly above or
below (0), or wholly right (its ends' classes' difference), which is the count of the curve
itself because a Bézier lies in its control hull (the spline-spec project's hull lemma).
-/

namespace ForgeText

/-- The half-open class: above the point's height. -/
def above (p q : Pt) : Bool := decide (q.y > p.y)

/-- Twice the signed area of a, b, c: positive when they run counter-clockwise. -/
def orient (a b c : Pt) : Int := (a.x - c.x) * (b.y - c.y) - (a.y - c.y) * (b.x - c.x)

/-- The signed crossings of the ray from p along +x with the line a to b: +1 going up through
it right of p, -1 going down; `none` if p is on the line (`winding::line`). -/
def lineCross (a b p : Pt) : Option Int :=
  let within := min a.x b.x ≤ p.x ∧ p.x ≤ max a.x b.x ∧ min a.y b.y ≤ p.y ∧ p.y ≤ max a.y b.y
  if within ∧ orient a b p = 0 then none
  else if above p a = above p b then some 0
  else
    let o := orient a b p
    let up := above p b
    if (up ∧ o > 0) ∨ (¬up ∧ o < 0) then some (if up then 1 else -1) else some 0

/-- A polygon's winding number about p: the sum of its sides' crossings (closing side
included); `none` if p lies on it. -/
def polyWinding (vs : List Pt) (p : Pt) : Option Int :=
  match vs with
  | [] => some 0
  | v :: _ =>
    let sides := (vs.zip (vs.drop 1 ++ [v]))
    sides.foldl (fun acc s => do let a ← acc; let c ← lineCross s.1 s.2 p; pure (a + c)) (some 0)

/-- A unit square's winding about its centre (on the doubled grid) is 1, about a point beside
it 0, and a point on a side is undecided. -/
example : polyWinding [⟨0, 0⟩, ⟨2, 0⟩, ⟨2, 2⟩, ⟨0, 2⟩] ⟨1, 1⟩ = some 1 := by decide
example : polyWinding [⟨0, 0⟩, ⟨2, 0⟩, ⟨2, 2⟩, ⟨0, 2⟩] ⟨3, 1⟩ = some 0 := by decide
example : polyWinding [⟨0, 0⟩, ⟨2, 0⟩, ⟨2, 2⟩, ⟨0, 2⟩] ⟨2, 1⟩ = none := by decide

/-- The count's claim (L3, to be proven): it is the polygon's, not its listing's: where the
vertex list starts does not change it. -/
def WindingFromAnyStart : Prop :=
  ∀ (vs : List Pt) (p : Pt) (k : Nat), polyWinding (vs.rotateLeft k) p = polyWinding vs p

/-- The rule's claim (L3, to be proven): for a simple polygon (no two non-adjacent sides
meet) and a point off it, the count is 1 or -1 (inside) or 0 (outside), and two points joined by a
segment missing the polygon count alike. -/
def WindingIsTopological (simple : List Pt → Prop) (missing : List Pt → Pt → Pt → Prop) : Prop :=
  ∀ (vs : List Pt) (p q : Pt), simple vs →
    (∀ w, polyWinding vs p = some w → w = 0 ∨ w = 1 ∨ w = -1) ∧
    (missing vs p q → polyWinding vs p = polyWinding vs q)

end ForgeText
