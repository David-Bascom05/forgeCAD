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

/-- The sum of counts, none if any is (a point on a side). -/
def total : List (Option Int) → Option Int
  | [] => some 0
  | x :: xs =>
    match x, total xs with
    | some a, some b => some (a + b)
    | _, _ => none

/-- A polygon's sides, the closing one last. -/
def sides : List Pt → List (Pt × Pt)
  | [] => []
  | v :: rest => (v :: rest).zip (rest ++ [v])

/-- A polygon's winding number about p: the sum of its sides' crossings (closing side
included); `none` if p lies on it. -/
def polyWinding (vs : List Pt) (p : Pt) : Option Int :=
  total ((sides vs).map fun s => lineCross s.1 s.2 p)

/-- A unit square's winding about its centre (on the doubled grid) is 1, about a point beside
it 0, and a point on a side is undecided. -/
example : polyWinding [⟨0, 0⟩, ⟨2, 0⟩, ⟨2, 2⟩, ⟨0, 2⟩] ⟨1, 1⟩ = some 1 := by decide
example : polyWinding [⟨0, 0⟩, ⟨2, 0⟩, ⟨2, 2⟩, ⟨0, 2⟩] ⟨3, 1⟩ = some 0 := by decide
example : polyWinding [⟨0, 0⟩, ⟨2, 0⟩, ⟨2, 2⟩, ⟨0, 2⟩] ⟨2, 1⟩ = none := by decide

/-- One step of rotation: the first vertex moved to the end. -/
def rot1 {α : Type} : List α → List α
  | [] => []
  | a :: t => t ++ [a]

/-- `k` steps of rotation. -/
def rot {α : Type} : Nat → List α → List α
  | 0, l => l
  | k + 1, l => rot1 (rot k l)

def plus : Option Int → Option Int → Option Int
  | some a, some b => some (a + b)
  | _, _ => none

theorem total_cons (x : Option Int) (xs : List (Option Int)) : total (x :: xs) = plus x (total xs) := by
  simp only [total]; cases x <;> cases total xs <;> rfl

theorem plus_comm (a b : Option Int) : plus a b = plus b a := by
  cases a <;> cases b <;> simp [plus, Int.add_comm]

theorem plus_assoc (a b c : Option Int) : plus (plus a b) c = plus a (plus b c) := by
  cases a <;> cases b <;> cases c <;> simp [plus, Int.add_assoc]

theorem total_append (l m : List (Option Int)) : total (l ++ m) = plus (total l) (total m) := by
  induction l with
  | nil => cases h : total m <;> simp [total, plus, h]
  | cons x xs ih => rw [List.cons_append, total_cons, total_cons, ih, plus_assoc]

theorem total_rot1 (l : List (Option Int)) : total (rot1 l) = total l := by
  cases l with
  | nil => rfl
  | cons a t =>
    simp only [rot1]
    rw [total_append, total_cons, total_cons]
    have : total ([] : List (Option Int)) = some 0 := rfl
    rw [this]
    cases a <;> cases total t <;> simp [plus, Int.add_comm]

theorem sides_rot1 (vs : List Pt) : sides (rot1 vs) = rot1 (sides vs) := by
  match vs with
  | [] => rfl
  | [v] => rfl
  | v :: w :: rest =>
    simp only [rot1, sides, List.cons_append, List.zip_cons_cons]
    have h := List.zip_append (l₁ := w :: rest) (r₁ := [v]) (l₂ := rest ++ [v]) (r₂ := [w]) (by simp)
    simpa using h

theorem map_rot1 {α β : Type} (f : α → β) (l : List α) : (rot1 l).map f = rot1 (l.map f) := by
  cases l <;> simp [rot1]

theorem polyWinding_rot1 (vs : List Pt) (p : Pt) : polyWinding (rot1 vs) p = polyWinding vs p := by
  unfold polyWinding
  rw [sides_rot1, map_rot1, total_rot1]

/-- The count is the polygon's, not its listing's: where the vertex list starts (rotated by
any number of steps) does not change it. -/
theorem winding_from_any_start (vs : List Pt) (p : Pt) (k : Nat) :
    polyWinding (rot k vs) p = polyWinding vs p := by
  induction k with
  | zero => rfl
  | succ k ih => simp only [rot]; rw [polyWinding_rot1, ih]

-- The rule's claim (`WindingIsTopological`) is proven in ForgeText.Topological
-- (`winding_is_topological`), from verify/winding2d's Jordan theorem.

end ForgeText
