import Sheet.Region
import Winding2d

/-!
# Region.lean's polygon rule is the shared winding number's parity

verify/winding2d proves the signed winding number of a rational polygon once, for sketch text,
sheet metal and the boolean: the same SoS `side` rule (`Winding2d.side`), its crossing
`crossSoS`, the count `windSoS`, which is the half-open winding number at every point, boundary
included (`windSoS_shift`), and Jordan for simple polygons (`wind_simple`: `0` or `σ = ±1` off
it). Here:

* `side_pos_iff`: Region.lean's `side` and winding2d's agree in sign;
* `crosses_iff`: an edge counts in `inPoly` exactly when its SoS crossing is nonzero
  (`crossSoS_ne_zero_iff`);
* `edges_sides`: the edges are winding2d's sides, in order;
* **`inPoly_wind`**: `inPoly p vs` is `windSoS mod 2 ≠ 0`;
* **`inPoly_simple`**: for a simple polygon and a point off it, `inPoly` is `wind ≠ 0`, i.e. the
  point is inside in Jordan's sense (`wind = σ`).

So `inLoop` (polygon XOR lenses) rests on the shared theorem for its polygon part.
-/

namespace Sheet.Winding

open Sheet.Region

def toPt (p : P) : Winding2d.Pt := ⟨p.1, p.2⟩

theorem orient_toPt (a b c : P) : Winding2d.orient (toPt a) (toPt b) (toPt c) = orient a b c := by
  simp [Winding2d.orient, orient, toPt]

theorem sgn_pos (x : ℚ) : 0 < Winding2d.sgn x ↔ 0 < x := by
  unfold Winding2d.sgn; split_ifs with h1 h2 <;> simp_all <;> linarith

theorem neg_sgn_pos (x : ℚ) : 0 < -Winding2d.sgn x ↔ x < 0 := by
  unfold Winding2d.sgn; split_ifs with h1 h2 <;> simp_all <;> linarith

theorem side_pos_iff (a b p : P) :
    0 < Winding2d.side (toPt a) (toPt b) (toPt p) ↔ 0 < side a b p := by
  unfold Winding2d.side side
  rw [orient_toPt]
  simp only [toPt]
  by_cases h1 : orient a b p = 0
  · by_cases h2 : b.1 = a.1
    · simp only [h1, h2, ne_eq, not_true_eq_false, if_false]
      rw [neg_sgn_pos]; constructor <;> intro <;> linarith
    · simp only [h1, h2, ne_eq, not_true_eq_false, not_false_eq_true, if_false, if_true]
      exact sgn_pos _
  · simp only [h1, ne_eq, not_false_eq_true, if_true]; exact sgn_pos _

theorem crosses_iff (p a b : P) :
    crosses p a b = decide (Winding2d.crossSoS (toPt p) (toPt a) (toPt b) ≠ 0) := by
  have h := Winding2d.crossSoS_ne_zero_iff (toPt p) (toPt a) (toPt b)
  rw [side_pos_iff] at h
  simp only [Winding2d.above, toPt] at h
  unfold crosses
  rw [Bool.eq_iff_iff, decide_eq_true_iff, show (toPt p) = ⟨p.1, p.2⟩ from rfl,
    show (toPt a) = ⟨a.1, a.2⟩ from rfl, show (toPt b) = ⟨b.1, b.2⟩ from rfl, h]
  by_cases hA : p.2 < a.2 <;> by_cases hB : p.2 < b.2 <;> simp [hA, hB]

theorem chain_cons (x : Winding2d.Pt) (ys : List Winding2d.Pt) :
    Winding2d.chain (x :: ys) = List.zip (x :: ys) ys := by
  induction ys generalizing x with
  | nil => rfl
  | cons y ys ih => show (x, y) :: Winding2d.chain (y :: ys) = _; rw [ih]; rfl

/-- **The edges are winding2d's sides**, in order. -/
theorem edges_sides (vs : List P) :
    (edges vs).map (fun e => (toPt e.1, toPt e.2)) = Winding2d.sides (vs.map toPt) := by
  cases vs with
  | nil => rfl
  | cons a rest =>
      simp only [edges, Winding2d.sides, List.map_cons]
      rw [List.cons_append, chain_cons, ← List.cons_append]
      rw [show List.zip ((toPt a :: rest.map toPt) ++ [toPt a]) (rest.map toPt ++ [toPt a]) =
          List.zip (toPt a :: rest.map toPt) (rest.map toPt ++ [toPt a]) by
        have := List.zip_append (l₁ := toPt a :: rest.map toPt) (l₂ := rest.map toPt ++ [toPt a])
          (r₁ := [toPt a]) (r₂ := []) (by simp)
        simpa using this]
      rw [show toPt a :: rest.map toPt = (a :: rest).map toPt from rfl,
        show rest.map toPt ++ [toPt a] = (rest ++ [a]).map toPt by simp, List.zip_map]
      rfl

theorem crossSoS_range (q a b : Winding2d.Pt) :
    Winding2d.crossSoS q a b = 0 ∨ Winding2d.crossSoS q a b = 1 ∨ Winding2d.crossSoS q a b = -1 := by
  unfold Winding2d.crossSoS; split_ifs <;> simp

/-- The parity of the nonzero entries of a list of `0, ±1` is the sum's. -/
theorem parity_sum (l : List ℤ) (h : ∀ x ∈ l, x = 0 ∨ x = 1 ∨ x = -1) :
    parity (l.map fun x => decide (x ≠ 0)) = decide (l.sum % 2 ≠ 0) := by
  induction l with
  | nil => simp [parity]
  | cons x xs ih =>
      rw [List.map_cons, parity_cons, ih (fun y hy => h y (by simp [hy])), List.sum_cons]
      rcases h x (by simp) with rfl | rfl | rfl <;> simp
      all_goals by_cases hs : xs.sum % 2 = 1 <;> simp [hs] <;> omega

/-- **`inPoly` is the shared winding number's parity**, at every point. -/
theorem inPoly_wind (p : P) (vs : List P) :
    inPoly p vs = decide (Winding2d.windSoS (vs.map toPt) (toPt p) % 2 ≠ 0) := by
  unfold inPoly Winding2d.windSoS
  rw [← edges_sides, List.map_map]
  have : (edges vs).map (fun e => crosses p e.1 e.2) =
      ((edges vs).map ((fun e => Winding2d.crossSoS (toPt p) e.1 e.2) ∘
        fun e => (toPt e.1, toPt e.2))).map fun x => decide (x ≠ 0) := by
    simp only [List.map_map]; apply List.map_congr_left; intro e _; exact crosses_iff p e.1 e.2
  rw [this, parity_sum]
  intro x hx
  simp only [List.mem_map] at hx
  obtain ⟨e, _, rfl⟩ := hx
  exact crossSoS_range _ _ _

/-- **Jordan, through the shared theorem**: for a simple polygon and a point off it, `inPoly`
is `wind ≠ 0` (`wind ∈ {0, σ}`, `σ = ±1`: `Winding2d.wind_simple`). -/
theorem inPoly_simple (p : P) (vs : List P) (hoff : Winding2d.OffPoly (vs.map toPt) (toPt p))
    (hw : Winding2d.wind (vs.map toPt) (toPt p) = 0 ∨ Winding2d.wind (vs.map toPt) (toPt p) = 1 ∨
      Winding2d.wind (vs.map toPt) (toPt p) = -1) :
    inPoly p vs = decide (Winding2d.wind (vs.map toPt) (toPt p) ≠ 0) := by
  rw [inPoly_wind, Winding2d.windSoS_eq hoff]
  rcases hw with h | h | h <;> rw [h] <;> decide

end Sheet.Winding
