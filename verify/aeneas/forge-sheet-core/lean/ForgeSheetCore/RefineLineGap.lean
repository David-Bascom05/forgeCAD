import ForgeSheetCore.RefineDisc
import IEEE754.Expansion

/-!
# `exact::line_gap`'s filter decides `sign(orient(a, b, c)² − l²|b − a|²)` when it fires

The shipped `line_gap` computes `v = o² − l²n` (`o` the rounded orientation `ux vy − uy vx`,
`n = ux² + uy²`) and returns `v`'s sign when `|v| > 16ε (m² + l²n)`, `m = |ux vy| + |uy vx|`.

`line_gap_filter`: on the domain (inputs multiples of `2⁻²⁰⁰`, at most `2¹⁰⁰`), every
operation's result is finite and, when the filter fires, `v` has the exact value's sign: every
rounding lands on the `2⁻¹⁰⁷⁴` grid, so is relatively accurate (`rel_mult`), which is
Filter16.lean's `line_gap_filter_sound`'s model. The filter code is the same in fix-sheet's
exact.rs, so this branch's proof serves both.
-/

namespace forge_sheet_core

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open IEEE754.FP

theorem C0_K4 : 2 ^ 20 * (C0 * C0 * (C0 * C0)) ≤ Kmax := by
  rw [C0_def, Kmax_def]
  have : (2 : ℚ) ^ 20 = 2 ^ (20 : ℤ) := by norm_num
  rw [this, ← two_zpow_add, ← two_zpow_add, ← two_zpow_add]
  exact two_zpow_le (by norm_num)

theorem abs_good {x : _root_.F64} {L : ℤ} (h : Good L x) : Good L (FP.abs x) :=
  ⟨by simpa using h.1, by rw [abs_val]; exact mult_abs h.2⟩

/-- The exact value `line_gap` decides the sign of. -/
def lineQ (ax ay bx «by» cx cy l : _root_.F64) : ℚ :=
  ((bx.val - ax.val) * (cy.val - ay.val) - (by.val - ay.val) * (cx.val - ax.val)) ^ 2 -
    l.val * l.val * ((bx.val - ax.val) * (bx.val - ax.val) + (by.val - ay.val) * (by.val - ay.val))

/-- **The filter branch of `line_gap` is right** on the domain. -/
theorem line_gap_filter {ax ay bx «by» cx cy l : _root_.F64}
    (hax : Good (-200) ax ∧ |ax.val| ≤ C0) (hay : Good (-200) ay ∧ |ay.val| ≤ C0)
    (hbx : Good (-200) bx ∧ |bx.val| ≤ C0) (hby : Good (-200) «by» ∧ |by.val| ≤ C0)
    (hcx : Good (-200) cx ∧ |cx.val| ≤ C0) (hcy : Good (-200) cy ∧ |cy.val| ≤ C0)
    (hl : Good (-200) l ∧ |l.val| ≤ C0) :
    let ux := FP.sub bx ax
    let uy := FP.sub «by» ay
    let vx := FP.sub cx ax
    let vy := FP.sub cy ay
    let o := FP.sub (FP.mul ux vy) (FP.mul uy vx)
    let n := FP.add (FP.mul ux ux) (FP.mul uy uy)
    let w := FP.mul (FP.mul l l) n
    let v := FP.sub (FP.mul o o) w
    let m := FP.add (FP.abs (FP.mul ux vy)) (FP.abs (FP.mul uy vx))
    let T := FP.mul (F64.Ops.ofBits 4386506037058863104) (FP.add (FP.mul m m) w)
    v.isFinite ∧ T.isFinite ∧
      (T.val < |v.val| → (0 < v.val ↔ 0 < lineQ ax ay bx «by» cx cy l) ∧
        (v.val < 0 ↔ lineQ ax ay bx «by» cx cy l < 0)) := by
  intro ux uy vx vy o n w v m T
  have hC := C0_ge
  have hK := C0_K4
  have hs2 := C0_sq
  have hs4 : C0 * C0 ≤ C0 * C0 * (C0 * C0) := by nlinarith
  have hu := uq_pos
  have huc : uq * (C0 * C0 * (C0 * C0)) ≤ C0 * C0 * (C0 * C0) :=
    mul_le_of_le_one_left (by positivity) uq_le1
  -- the differences
  obtain ⟨gux, eux, bux⟩ := sub_good (L := -200) (by norm_num) hbx.1 hax.1
    (abs_sub_le2 hbx.2 hax.2) (by linarith)
  obtain ⟨guy, euy, buy⟩ := sub_good (L := -200) (by norm_num) hby.1 hay.1
    (abs_sub_le2 hby.2 hay.2) (by linarith)
  obtain ⟨gvx, evx, bvx⟩ := sub_good (L := -200) (by norm_num) hcx.1 hax.1
    (abs_sub_le2 hcx.2 hax.2) (by linarith)
  obtain ⟨gvy, evy, bvy⟩ := sub_good (L := -200) (by norm_num) hcy.1 hay.1
    (abs_sub_le2 hcy.2 hay.2) (by linarith)
  -- the orientation's products and difference
  obtain ⟨gp1, ep1, bp1⟩ := mul_good (La := -200) (Lb := -200) (by norm_num) gux gvy
    (B := 16 * (C0 * C0)) (le_trans (abs_mul_le bux bvy) (le_of_eq (by ring))) (by linarith)
  obtain ⟨gp2, ep2, bp2⟩ := mul_good (La := -200) (Lb := -200) (by norm_num) guy gvx
    (B := 16 * (C0 * C0)) (le_trans (abs_mul_le buy bvx) (le_of_eq (by ring))) (by linarith)
  norm_num at gp1 gp2
  obtain ⟨go, eo, bo⟩ := sub_good (L := -400) (by norm_num) gp1 gp2
    (B := 64 * (C0 * C0)) (le_trans (abs_sub_le2 bp1 bp2) (le_of_eq (by ring))) (by linarith)
  -- the squared length
  obtain ⟨ga, ea, ba⟩ := mul_good (La := -200) (Lb := -200) (by norm_num) gux gux
    (B := 16 * (C0 * C0)) (le_trans (abs_mul_le bux bux) (le_of_eq (by ring))) (by linarith)
  obtain ⟨gb, eb, bb⟩ := mul_good (La := -200) (Lb := -200) (by norm_num) guy guy
    (B := 16 * (C0 * C0)) (le_trans (abs_mul_le buy buy) (le_of_eq (by ring))) (by linarith)
  norm_num at ga gb
  obtain ⟨gn, en, bn⟩ := add_good (L := -400) (by norm_num) ga gb
    (B := 64 * (C0 * C0)) (le_trans (abs_add_le2 ba bb) (le_of_eq (by ring))) (by linarith)
  -- l², l²n, o², v
  obtain ⟨gll, ell, bll⟩ := mul_good (La := -200) (Lb := -200) (by norm_num) hl.1 hl.1
    (abs_mul_le hl.2 hl.2) (by linarith)
  norm_num at gll
  obtain ⟨gw, ew, bw⟩ := mul_good (La := -400) (Lb := -400) (by norm_num) gll gn
    (B := 256 * (C0 * C0 * (C0 * C0))) (le_trans (abs_mul_le bll bn) (le_of_eq (by ring)))
    (by linarith)
  obtain ⟨goo, eoo, boo⟩ := mul_good (La := -400) (Lb := -400) (by norm_num) go go
    (B := 16384 * (C0 * C0 * (C0 * C0))) (le_trans (abs_mul_le bo bo) (le_of_eq (by ring)))
    (by linarith)
  norm_num at gw goo
  obtain ⟨gv, ev, -⟩ := sub_good (L := -800) (by norm_num) goo gw
    (B := 33280 * (C0 * C0 * (C0 * C0))) (le_trans (abs_sub_le2 boo bw) (le_of_eq (by ring)))
    (by linarith)
  -- the magnitude m, m², the filter's bound
  have bq1 : |(FP.abs (FP.mul ux vy)).val| ≤ 2 * (16 * (C0 * C0)) := by rw [abs_val, abs_abs]; exact bp1
  have bq2 : |(FP.abs (FP.mul uy vx)).val| ≤ 2 * (16 * (C0 * C0)) := by rw [abs_val, abs_abs]; exact bp2
  obtain ⟨gm, em, bm⟩ := add_good (L := -400) (by norm_num) (abs_good gp1) (abs_good gp2)
    (B := 64 * (C0 * C0)) (le_trans (abs_add_le2 bq1 bq2) (le_of_eq (by ring))) (by linarith)
  rw [abs_val, abs_val] at em
  obtain ⟨gmm, emm, bmm⟩ := mul_good (La := -400) (Lb := -400) (by norm_num) gm gm
    (B := 16384 * (C0 * C0 * (C0 * C0))) (le_trans (abs_mul_le bm bm) (le_of_eq (by ring)))
    (by linarith)
  norm_num at gmm
  obtain ⟨gt, et, bt⟩ := add_good (L := -800) (by norm_num) gmm gw
    (B := 33280 * (C0 * C0 * (C0 * C0))) (le_trans (abs_add_le2 bmm bw) (le_of_eq (by ring)))
    (by linarith)
  have hF : |(F64.Ops.ofBits 4386506037058863104).val| ≤ 16 * uq := by
    rw [FILTER_val, abs_of_pos (by linarith)]
  obtain ⟨gT, eT, -⟩ := mul_good (La := -101) (Lb := -800) (by norm_num) FILTER_good gt
    (abs_mul_le hF bt) (by linarith)
  refine ⟨gv.1, gT.1, fun hgt => ?_⟩
  rw [FILTER_val] at eT
  have hu1 : (uq : ℝ) ≤ 1 / 1024 := by
    have := uq_small
    have h : uq ≤ 1 / 1024 := by linarith
    have h' := (Rat.cast_le (K := ℝ)).mpr h; push_cast at h'; exact h'
  have key := Sheet.Filter16.line_gap_filter_sound (u := (uq : ℝ))
    (UX := (bx.val : ℝ) - ax.val) (UY := (by.val : ℝ) - ay.val)
    (VX := (cx.val : ℝ) - ax.val) (VY := (cy.val : ℝ) - ay.val) (l := (l.val : ℝ))
    (by exact_mod_cast hu.le) hu1
    (by have h := rel_cast eux; push_cast at h; exact h)
    (by have h := rel_cast euy; push_cast at h; exact h)
    (by have h := rel_cast evx; push_cast at h; exact h)
    (by have h := rel_cast evy; push_cast at h; exact h)
    (by have h := rel_cast ep1; push_cast at h; exact h)
    (by have h := rel_cast ep2; push_cast at h; exact h)
    (by have h := rel_cast eo; push_cast at h; exact h)
    (by have h := rel_cast ea; push_cast at h; exact h)
    (by have h := rel_cast eb; push_cast at h; exact h)
    (by have h := rel_cast en; push_cast at h; exact h)
    (by have h := rel_cast ell; push_cast at h; exact h)
    (by have h := rel_cast ew; push_cast at h; exact h)
    (by have h := rel_cast eoo; push_cast at h; exact h)
    (by have h := rel_cast ev; push_cast at h; exact h)
    (by have h := rel_cast em; push_cast at h; exact h)
    (by have h := rel_cast emm; push_cast at h; exact h)
    (by have h := rel_cast et; push_cast at h; exact h)
    (by have h := rel_cast eT; push_cast at h; exact h)
    (by exact_mod_cast hgt)
  dsimp only at key
  have e : (((bx.val : ℝ) - ax.val) * ((cy.val : ℝ) - ay.val) -
      ((by.val : ℝ) - ay.val) * ((cx.val : ℝ) - ax.val)) ^ 2 -
      (l.val : ℝ) * l.val * (((bx.val : ℝ) - ax.val) * ((bx.val : ℝ) - ax.val) +
        ((by.val : ℝ) - ay.val) * ((by.val : ℝ) - ay.val)) =
      ((lineQ ax ay bx «by» cx cy l : ℚ) : ℝ) := by
    unfold lineQ; push_cast; ring
  rw [e] at key
  obtain ⟨k1, k2⟩ := key
  constructor
  · rw [← Rat.cast_pos (K := ℝ), k1, Rat.cast_pos]
  · rw [← Rat.cast_lt_zero (K := ℝ), k2, Rat.cast_lt_zero]

end forge_sheet_core
