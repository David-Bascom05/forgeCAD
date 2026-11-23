import ForgeCurved.Separators

/-!
# Lemma D fails for parallel sweep walls: a plate with a hole

Design §2.3's Tier 1 adds planes `d·x = c` (one direction `d`) at rational levels between
consecutive events, and §2.3 states Lemma D: in a slab with at most one event, with the seams in
place, R1–R5 hold. This file shows that R1 already fails for an input in the Q1 domain, so
`LemmaD` (Separators.lean) cannot be proven for a `SepRule` whose walls are those planes.

The input is a plate with a hole: the plane `z = 0`, the cylinder `x² + y² = 1` (axis `z`), and
the cylinder's Tier-0 seam `y = 0` (the plane through the axis with normal `z × x`). With
`d ∝ (2, 3, 6)`, the plate's level is `f = 2x + 3y`. Its events are `f = 2` (the vertex
`(1, 0, 0)`, on the plane, the cylinder and the seam) and `f = √13` (the circle's `d`-critical
point). So any midpoint rule puts a wall level `c_w` in `(2, √13)`.

Take the facet `{z = 0, outside the cylinder, y > 0}` below that wall. At each level `c` in
`(2, c_w)`, the ray `{f = c, y > 0}` minus the disc's chord is two intervals, and both have the
same covector. The open half-planes `3x − 2y > 0` and `3x − 2y < 0` split the cell, because the
line `3x = 2y` meets `{f < √13}` only inside the disc. This is `parallel_walls_not_faithful`.
Whatever the other wall levels are, one wall in `(2, √13)` is enough.

The same happens in an event-free slab whenever a level curve crosses another carrier four times:
two arcs with one sign pattern. The line `3x = 2y` is the cylinder's polar plane
`∇q · u = 0` for the in-level direction `u = (3, −2, 0)`. A two-level (cylindrical) Tier 1 would
add such planes; see the README.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.WallsCounter

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved

/-- The points, as in EndToEnd: `(x, y, z)`. -/
abbrev P3 := ℝ × ℝ × ℝ

/-- The plate's plane `z = 0`. -/
def plate : Carrier P3 ℝ := fun p => p.2.2
/-- The hole's cylinder `x² + y² = 1`. -/
def cyl : Carrier P3 ℝ := fun p => p.1 ^ 2 + p.2.1 ^ 2 - 1
/-- The cylinder's Tier-0 axial seam `y = 0`. -/
def seam : Carrier P3 ℝ := fun p => p.2.1
/-- The sweep level, `d ∝ (2, 3, 6)`. -/
def lev (p : P3) : ℝ := 2 * p.1 + 3 * p.2.1 + 6 * p.2.2
/-- A Tier-1 wall: the plane `d·x = c`. -/
def wall (c : ℝ) : Carrier P3 ℝ := fun p => lev p - c

/-- The arrangement: the input, its seam, and parallel walls at the levels `ws`. -/
def arr (ws : List ℝ) : List (Carrier P3 ℝ) := [plate, cyl, seam] ++ ws.map wall

theorem cov_arr (ws : List ℝ) (p : P3) :
    covOfK (arr ws) p =
      [sgnK (plate p), sgnK (cyl p), sgnK (seam p)] ++ ws.map (fun c => sgnK (lev p - c)) := by
  simp [arr, covOfK, wall, Function.comp_def]

/-- **R1 fails** for the plate with a hole under parallel walls with one level in `(2, √13)`. -/
theorem parallel_walls_not_faithful (ws : List ℝ) {cw : ℝ} (hw : cw ∈ ws) (h2 : 2 < cw)
    (h13 : cw ^ 2 < 13) : ¬FaithfulAll (arr ws) := by
  intro hF
  -- the level `c` between the vertex event and the wall
  set c := (2 + cw) / 2 with hc
  have hc2 : 2 < c := by rw [hc]; linarith
  have hcw : c < cw := by rw [hc]; linarith
  have hcw4 : cw < 4 := by nlinarith
  -- the near point, between the seam and the disc, and the far point, past the disc
  let a : P3 := ((c + 2) / 4, (c - 2) / 6, 0)
  let b : P3 := ((c - 6) / 2, 2, 0)
  have hla : lev a = c := by simp only [lev, a]; ring
  have hlb : lev b = c := by simp only [lev, b]; ring
  have hcov : covOfK (arr ws) b = covOfK (arr ws) a := by
    rw [cov_arr, cov_arr, hla, hlb]
    have e1 : sgnK (plate b) = sgnK (plate a) := by simp [plate, a, b]
    have e2 : sgnK (cyl b) = sgnK (cyl a) := by
      have ha : 0 < cyl a := by simp only [cyl, a]; nlinarith
      have hb : 0 < cyl b := by simp only [cyl, b]; nlinarith
      rw [sgnK_pos ha, sgnK_pos hb]
    have e3 : sgnK (seam b) = sgnK (seam a) := by
      have ha : 0 < seam a := by simp only [seam, a]; linarith
      have hb : 0 < seam b := by simp only [seam, b]; norm_num
      rw [sgnK_pos ha, sgnK_pos hb]
    rw [e1, e2, e3]
  set X := covOfK (arr ws) a
  have hconn := hF.connected X trivial ⟨a, rfl⟩
  -- the two open half-planes
  let U : Pred P3 := fun p => 0 < 3 * p.1 - 2 * p.2.1
  let V : Pred P3 := fun p => 3 * p.1 - 2 * p.2.1 < 0
  have hU : ForgeExact.Topology.IsOpen U := by
    show IsOpen {p : P3 | 0 < 3 * p.1 - 2 * p.2.1}
    exact isOpen_lt continuous_const (by fun_prop)
  have hV : ForgeExact.Topology.IsOpen V := by
    show IsOpen {p : P3 | 3 * p.1 - 2 * p.2.1 < 0}
    exact isOpen_lt (by fun_prop) continuous_const
  -- every point of the cell is off the line `3x = 2y`
  have hcell : ∀ q, covCellK (arr ws) X q →
      0 < cyl q ∧ 0 < seam q ∧ lev q - cw < 0 := by
    intro q hq
    have hq' : covOfK (arr ws) q = covOfK (arr ws) a := hq
    rw [cov_arr, cov_arr] at hq'
    simp only [List.cons_append, List.cons.injEq, List.nil_append] at hq'
    obtain ⟨-, hcy, hse, hws⟩ := hq'
    have hcya : sgnK (cyl a) = .pos := sgnK_pos (by simp only [cyl, a]; nlinarith)
    have hsea : sgnK (seam a) = .pos := sgnK_pos (by simp only [seam, a]; linarith)
    have hwa : sgnK (lev a - cw) = .neg := sgnK_neg (by rw [hla]; linarith)
    have hwq : sgnK (lev q - cw) = sgnK (lev a - cw) := (List.map_inj_left.mp hws) cw hw
    rw [hcya] at hcy; rw [hsea] at hse; rw [hwa] at hwq
    refine ⟨?_, ?_, ?_⟩
    · rcases lt_trichotomy 0 (cyl q) with h | h | h
      · exact h
      · rw [sgnK_zero_iff.mpr h.symm] at hcy; cases hcy
      · rw [sgnK_neg h] at hcy; cases hcy
    · rcases lt_trichotomy 0 (seam q) with h | h | h
      · exact h
      · rw [sgnK_zero_iff.mpr h.symm] at hse; cases hse
      · rw [sgnK_neg h] at hse; cases hse
    · rcases lt_trichotomy (lev q - cw) 0 with h | h | h
      · exact h
      · rw [sgnK_zero_iff.mpr h] at hwq; cases hwq
      · rw [sgnK_pos h] at hwq; cases hwq
  have hplate : ∀ q, covCellK (arr ws) X q → q.2.2 = 0 := by
    intro q hq
    have hq' : covOfK (arr ws) q = covOfK (arr ws) a := hq
    rw [cov_arr, cov_arr] at hq'
    simp only [List.cons_append, List.cons.injEq] at hq'
    have h0 : sgnK (plate q) = .zero := by rw [hq'.1]; exact sgnK_zero_iff.mpr (by simp [plate, a])
    exact sgnK_zero_iff.mp h0
  have hsub : (covCellK (arr ws) X).Sub (U.union V) := by
    intro q hq
    obtain ⟨hcy, hse, hlv⟩ := hcell q hq
    have hz := hplate q hq
    simp only [cyl, seam, lev, hz] at hcy hse hlv
    rcases lt_trichotomy 0 (3 * q.1 - 2 * q.2.1) with h | h | h
    · exact Or.inl h
    · exfalso
      -- on the line, the level `13y/3 < cw` forces the point into the disc
      have hx : q.1 = 2 * q.2.1 / 3 := by linarith
      rw [hx] at hcy hlv
      nlinarith
    · exact Or.inr h
  obtain ⟨z, _, hzU, hzV⟩ := hconn U V hU hV hsub
    ⟨a, rfl, show 0 < 3 * ((c + 2) / 4) - 2 * ((c - 2) / 6) by linarith⟩
    ⟨b, hcov, show 3 * ((c - 6) / 2) - 2 * 2 < 0 by linarith⟩
  exact absurd (lt_trans hzV hzU) (lt_irrefl _)

/-- **`LemmaD` is false for parallel walls.** Any separator rule whose input `fs` has the plate
with a hole as its carriers, its Tier-0 seam as the seams, and parallel `d`-walls with one level
strictly between the events `2` and `√13`, fails `LemmaD` on every domain containing `fs`. -/
theorem lemmaD_parallel_walls_false {ι : Type} (r : SepRule ι) (Dom : List ι → Prop) (fs : List ι)
    (hdom : Dom fs) (hcar : r.carriers fs = [plate, cyl]) (hseams : r.seams fs = [seam])
    (ws : List ℝ) (hwalls : r.walls fs = ws.map wall) {cw : ℝ} (hw : cw ∈ ws) (h2 : 2 < cw)
    (h13 : cw ^ 2 < 13) : ¬LemmaD r Dom := by
  intro hD
  have h := hD fs hdom
  rw [hcar, hseams, hwalls] at h
  exact parallel_walls_not_faithful ws hw h2 h13 (by simpa [arr] using h)

end ForgeCurved.WallsCounter
