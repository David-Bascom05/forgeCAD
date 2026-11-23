import ForgeText.Winding
import Winding2d

/-!
# `WindingIsTopological`, from the shared winding theorem (verify/winding2d)

Sketch text's count (`polyWinding`, on the font's integer grid) is winding2d's `wind` on the
same points cast to rationals, wherever the point is off the polygon: the line test
(`lineCross`, forge-sketch's `winding::line`) is winding2d's half-open `cross` there, and the two
lists of sides are the same. So winding2d's Jordan theorem for simple polygons
(`wind_simple_mem`) and its constancy along segments (`wind_eq_of_seg`) are sketch text's.
-/

namespace ForgeText

/-- A grid point as winding2d's rational point. -/
def toQ (p : Pt) : Winding2d.Pt := ⟨p.x, p.y⟩

/-- The two orientation determinants are one: twice the signed area of `a b c`. -/
theorem orient_toQ (a b c : Pt) :
    ((orient a b c : Int) : Rat) = Winding2d.orient (toQ a) (toQ b) (toQ c) := by
  simp only [orient, Winding2d.orient, toQ, Rat.intCast_sub, Rat.intCast_mul]
  grind

theorem pos_toQ (a b c : Pt) : 0 < orient a b c ↔ 0 < Winding2d.orient (toQ a) (toQ b) (toQ c) := by
  rw [← orient_toQ]; exact Rat.intCast_pos.symm

theorem neg_toQ (a b c : Pt) : orient a b c < 0 ↔ Winding2d.orient (toQ a) (toQ b) (toQ c) < 0 := by
  rw [← orient_toQ]
  constructor
  · intro h; exact Rat.intCast_lt_intCast.mpr h
  · intro h; exact Rat.intCast_lt_intCast.mp h

/-- A point on a horizontal line `ab`, between `a` and `b` along it, is on `ab`. -/
theorem onSeg_of_orient_x {z a b : Winding2d.Pt} (ho : Winding2d.orient a b z = 0) (hy : a.y = b.y)
    (hab : a.x ≠ b.x) (hz : Winding2d.Btw z.x a.x b.x) : Winding2d.OnSeg z a b := by
  have hd : b.x - a.x ≠ 0 := by grind
  have ht : (z.x - a.x) / (b.x - a.x) * (b.x - a.x) = z.x - a.x := Rat.div_mul_cancel hd
  have hu := Winding2d.unit_of _ _ _ hd ht (by unfold Winding2d.Btw at hz; grind)
  refine ⟨_, hu.1, hu.2, ?_⟩
  have hzy : (z.y - a.y) * (b.x - a.x) = 0 := by
    simp only [Winding2d.orient] at ho; rw [hy] at ho ⊢; grind
  have := Winding2d.mul_eq_zero_left hd hzy
  ext <;> simp only [Winding2d.lerp] <;> grind

theorem btw_cast {u a b : Int} (h : (a ≤ u ∧ u ≤ b) ∨ (b ≤ u ∧ u ≤ a)) :
    Winding2d.Btw (u : Rat) (a : Rat) (b : Rat) := by
  unfold Winding2d.Btw
  rcases h with h | h
  · exact Or.inl ⟨Rat.intCast_le_intCast.mpr h.1, Rat.intCast_le_intCast.mpr h.2⟩
  · exact Or.inr ⟨Rat.intCast_le_intCast.mpr h.1, Rat.intCast_le_intCast.mpr h.2⟩

/-- `lineCross`'s refusal is only ever on the segment: a point within its box on its line. -/
theorem onSeg_of_within {a b p : Pt}
    (hw : min a.x b.x ≤ p.x ∧ p.x ≤ max a.x b.x ∧ min a.y b.y ≤ p.y ∧ p.y ≤ max a.y b.y)
    (ho : orient a b p = 0) : Winding2d.OnSeg (toQ p) (toQ a) (toQ b) := by
  have hoq : Winding2d.orient (toQ a) (toQ b) (toQ p) = 0 := by
    rw [← orient_toQ, ho]; rfl
  by_cases hy : a.y = b.y
  · by_cases hx : a.x = b.x
    · -- a point segment: p is it
      have e : p = a := by
        cases p; cases a; cases b; simp only at hx hy hw ⊢; subst hx hy; simp only [Pt.mk.injEq]; omega
      subst e; exact Winding2d.onSeg_left
    · refine onSeg_of_orient_x hoq (by simp [toQ, hy]) ?_ (btw_cast (by omega))
      simp only [toQ]; intro h; exact hx (Rat.intCast_inj.mp h)
  · refine Winding2d.onSeg_of_orient hoq ?_ (btw_cast (by omega))
    simp only [toQ]; intro h; exact hy (Rat.intCast_inj.mp h)

/-- **Off the side, the line test is winding2d's half-open crossing.** -/
theorem lineCross_eq {a b p : Pt} (hoff : ¬Winding2d.OnSeg (toQ p) (toQ a) (toQ b)) :
    lineCross a b p = some (Winding2d.cross (toQ p) (toQ a) (toQ b)) := by
  have hpos := pos_toQ a b p
  have hneg := neg_toQ a b p
  unfold lineCross
  rw [ite_cond_eq_false _ _ (eq_false (fun h => hoff (onSeg_of_within h.1 h.2)))]
  simp only [above, Winding2d.cross, toQ, Rat.intCast_lt_intCast] at *
  by_cases ha : p.y < a.y <;> by_cases hb : p.y < b.y <;> simp_all [toQ] <;> split <;> grind

/-- `total` of counts that are all known is their sum. -/
theorem total_map_some {α : Type} (l : List α) (f : α → Int) :
    total (l.map fun x => some (f x)) = some (l.map f).sum := by
  induction l with
  | nil => rfl
  | cons x xs ih => simp [total, ih]

theorem chain_eq_zip : ∀ (l : List Winding2d.Pt), Winding2d.chain l = l.zip l.tail
  | [] => rfl
  | [_] => rfl
  | a :: b :: t => by simp [Winding2d.chain, chain_eq_zip (b :: t)]

theorem zip_toQ : ∀ (l m : List Pt),
    (l.zip m).map (fun s => (toQ s.1, toQ s.2)) = (l.map toQ).zip (m.map toQ)
  | [], _ => rfl
  | _ :: _, [] => rfl
  | a :: l, b :: m => by simp [zip_toQ l m]

theorem zip_append_long {α β : Type} (l : List α) (x : α) (m : List β) (h : l.length = m.length) :
    (l ++ [x]).zip m = l.zip m := by
  have := List.zip_append (l₁ := l) (r₁ := [x]) (l₂ := m) (r₂ := ([] : List β)) h
  simpa using this

/-- The sides are winding2d's, point for point. -/
theorem sides_toQ (vs : List Pt) : (sides vs).map (fun s => (toQ s.1, toQ s.2)) = Winding2d.sides (vs.map toQ) := by
  cases vs with
  | nil => rfl
  | cons v rest =>
    simp only [sides, Winding2d.sides, List.map_cons, chain_eq_zip, List.cons_append, List.tail_cons]
    rw [zip_toQ, ← List.cons_append, zip_append_long _ _ _ (by simp)]
    simp

/-- **Off the polygon, sketch text's count is winding2d's `wind`.** -/
theorem polyWinding_eq (vs : List Pt) (q : Pt) (hq : Winding2d.OffPoly (vs.map toQ) (toQ q)) :
    polyWinding vs q = some (Winding2d.wind (vs.map toQ) (toQ q)) := by
  unfold polyWinding Winding2d.wind
  have hs : ∀ s ∈ sides vs, lineCross s.1 s.2 q = some (Winding2d.cross (toQ q) (toQ s.1) (toQ s.2)) := by
    intro s hs
    apply lineCross_eq
    apply hq (toQ s.1, toQ s.2)
    rw [← sides_toQ]; exact List.mem_map_of_mem hs
  rw [List.map_congr_left hs, total_map_some, ← sides_toQ, List.map_map]
  rfl

/-- **Sketch text's winding is topological** (closes `WindingIsTopological`): for a polygon simple
in winding2d's sense, the count at any point off it is `0`, `1` or `-1`; and two points joined by
a segment that meets no side count alike. -/
theorem winding_is_topological (vs : List Pt) (hs : Winding2d.Simple (vs.map toQ)) :
    (∀ q, Winding2d.OffPoly (vs.map toQ) (toQ q) →
      ∃ w, polyWinding vs q = some w ∧ (w = 0 ∨ w = 1 ∨ w = -1)) ∧
    (∀ p q, Winding2d.OffPoly (vs.map toQ) (toQ p) → Winding2d.OffPoly (vs.map toQ) (toQ q) →
      (∀ e ∈ Winding2d.sides (vs.map toQ), ¬Winding2d.Meets (toQ p) (toQ q) e.1 e.2) →
      polyWinding vs p = polyWinding vs q) := by
  refine ⟨fun q hq => ⟨_, polyWinding_eq vs q hq, Winding2d.wind_simple_mem hs hq⟩, ?_⟩
  intro p q hp hq hm
  rw [polyWinding_eq vs p hp, polyWinding_eq vs q hq, Winding2d.wind_eq_of_seg _ _ _ hm]

/-- The line test on rationals (the kernel's on doubles, whose values are rationals): the same
rule as `lineCross`, the box test spelled with its ends. -/
def lineCrossQ (a b p : Winding2d.Pt) : Option Int :=
  let within := (a.x ≤ p.x ∨ b.x ≤ p.x) ∧ (p.x ≤ a.x ∨ p.x ≤ b.x) ∧
    (a.y ≤ p.y ∨ b.y ≤ p.y) ∧ (p.y ≤ a.y ∨ p.y ≤ b.y)
  if within ∧ Winding2d.orient a b p = 0 then none
  else if decide (p.y < a.y) = decide (p.y < b.y) then some 0
  else
    let o := Winding2d.orient a b p
    let up := decide (p.y < b.y)
    if (up ∧ 0 < o) ∨ (¬up ∧ o < 0) then some (if up then 1 else -1) else some 0

/-- The integer line test is the rational one on the points cast. -/
theorem lineCross_toQ (a b p : Pt) : lineCross a b p = lineCrossQ (toQ a) (toQ b) (toQ p) := by
  have h0 : orient a b p = 0 ↔ Winding2d.orient (toQ a) (toQ b) (toQ p) = 0 := by
    rw [← orient_toQ]; exact ⟨fun h => by simp [h], fun h => Rat.intCast_inj.mp (by simpa using h)⟩
  have hpos := pos_toQ a b p
  have hneg := neg_toQ a b p
  have l : ∀ u v : Int, ((u : Rat) ≤ v ↔ u ≤ v) := fun u v => Rat.intCast_le_intCast
  have lt : ∀ u v : Int, ((u : Rat) < v ↔ u < v) := fun u v => Rat.intCast_lt_intCast
  have w : (min a.x b.x ≤ p.x ∧ p.x ≤ max a.x b.x ∧ min a.y b.y ≤ p.y ∧ p.y ≤ max a.y b.y) ↔
      (((toQ a).x ≤ (toQ p).x ∨ (toQ b).x ≤ (toQ p).x) ∧ ((toQ p).x ≤ (toQ a).x ∨ (toQ p).x ≤ (toQ b).x) ∧
        ((toQ a).y ≤ (toQ p).y ∨ (toQ b).y ≤ (toQ p).y) ∧ ((toQ p).y ≤ (toQ a).y ∨ (toQ p).y ≤ (toQ b).y)) := by
    simp only [toQ, l]; omega
  unfold lineCross lineCrossQ
  simp only [above, gt_iff_lt]
  have ha : decide (p.y < a.y) = decide ((toQ p).y < (toQ a).y) := by simp [toQ, lt]
  have hb : decide (p.y < b.y) = decide ((toQ p).y < (toQ b).y) := by simp [toQ, lt]
  simp only [w, h0, ha, hb, hpos, hneg]

end ForgeText
