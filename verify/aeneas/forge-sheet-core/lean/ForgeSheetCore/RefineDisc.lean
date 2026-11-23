import ForgeSheetCore.ExactModel
import Sheet.Filter16

/-!
# `exact::disc` decides `sign(|p − c|² − r²)` exactly

The shipped `disc` (kernel/forge-sheet-core/src/exact.rs) first tries a float filter: `v` is the
rounded `dx² + dy² − r²`, and when `|v| > 16ε · (dx² + dy² + r²)` (all rounded) it returns `v`'s
sign. Otherwise it builds the expansions and returns the sign of their exact sum.

`disc_spec`: on the domain (every input a multiple of `2⁻²⁰⁰` and at most `2¹⁰⁰` in magnitude),
`disc` returns `sgnQ` of the exact value of the input doubles. Both branches are proved:
* the filter branch by Filter16.lean's `disc_filter_sound`: on the domain every rounding is on
  the `2⁻¹⁰⁷⁴` grid, so relatively accurate (`rel_mult`), which is that theorem's model;
* the exact branch by the refinement specs (RefineExact.lean) and `disc_model`, under the
  expansion theory, which is a hypothesis (`ExpansionTheory`), never an axiom.
-/

namespace forge_sheet_core

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open IEEE754.FP

/-! ## The filter constant: `16 · (ε / 2) = 2⁻⁴⁹` -/

theorem FILTER_eq : exact.FILTER = ok (F64.Ops.ofBits 4386506037058863104) := by
  simp only [exact.FILTER, core.f64.F64.EPSILON, lift, bind_ok, F64.Ops.mul_def]
  congr 1; decide +kernel

theorem FILTER_fin : (F64.Ops.ofBits 4386506037058863104).isFinite := by decide

theorem FILTER_val : (F64.Ops.ofBits 4386506037058863104).val = 16 * uq := by
  rw [show F64.Ops.ofBits 4386506037058863104 = FP.fin false (2 ^ 52) (-101) (by decide) by decide]
  rw [uq_def]; simp [FP.val, FP.sgn]; norm_num

theorem FILTER_good : Good (-101) (F64.Ops.ofBits 4386506037058863104) :=
  ⟨FILTER_fin, ⟨2 ^ 52, by rw [FILTER_val, uq_def]; norm_num⟩⟩

/-! ## One rounding on the grid: finite, relatively accurate, on the grid -/

theorem rnd_good {zs : Bool} {r : ℚ} {L : ℤ} (hm : Mult r L) (hL : -1074 ≤ L) {B : ℚ}
    (hr : |r| ≤ B) (hB : B ≤ 8 * Kmax) :
    Good L (IEEE754.round IEEE754.binary64 zs r) ∧
      |(IEEE754.round IEEE754.binary64 zs r).val - r| ≤ uq * |r| ∧
      |(IEEE754.round IEEE754.binary64 zs r).val| ≤ 2 * B := by
  have hf := fin_of_le8 (zs := zs) (le_trans hr hB)
  have he : |(IEEE754.round IEEE754.binary64 zs r).val - r| ≤ uq * |r| := by
    rw [uq_def]; exact rel_mult (mult_of_le hm hL) hf
  refine ⟨⟨hf, round_mult hm hL hf⟩, he, ?_⟩
  have := abs_sub_abs_le_abs_sub (IEEE754.round IEEE754.binary64 zs r).val r
  have := uq_le1
  have : uq * |r| ≤ 1 * |r| := mul_le_mul_of_nonneg_right uq_le1 (abs_nonneg r)
  linarith

theorem sub_good {a b : _root_.F64} {L : ℤ} (hL : -1074 ≤ L) (ha : Good L a) (hb : Good L b)
    {B : ℚ} (hab : |a.val - b.val| ≤ B) (hB : B ≤ 8 * Kmax) :
    Good L (FP.sub a b) ∧ |(FP.sub a b).val - (a.val - b.val)| ≤ uq * |a.val - b.val| ∧
      |(FP.sub a b).val| ≤ 2 * B := by
  rw [sub_eq_round ha.1 hb.1]
  obtain ⟨n, hn⟩ := ha.2; obtain ⟨m, hm⟩ := hb.2
  exact rnd_good ⟨n - m, by rw [hn, hm]; push_cast; ring⟩ hL hab hB

theorem add_good {a b : _root_.F64} {L : ℤ} (hL : -1074 ≤ L) (ha : Good L a) (hb : Good L b)
    {B : ℚ} (hab : |a.val + b.val| ≤ B) (hB : B ≤ 8 * Kmax) :
    Good L (FP.add a b) ∧ |(FP.add a b).val - (a.val + b.val)| ≤ uq * |a.val + b.val| ∧
      |(FP.add a b).val| ≤ 2 * B := by
  rw [add_eq_round ha.1 hb.1]
  exact rnd_good (mult_add ha.2 hb.2) hL hab hB

theorem mul_good {a b : _root_.F64} {La Lb : ℤ} (hL : -1074 ≤ La + Lb) (ha : Good La a)
    (hb : Good Lb b) {B : ℚ} (hab : |a.val * b.val| ≤ B) (hB : B ≤ 8 * Kmax) :
    Good (La + Lb) (FP.mul a b) ∧ |(FP.mul a b).val - (a.val * b.val)| ≤ uq * |a.val * b.val| ∧
      |(FP.mul a b).val| ≤ 2 * B := by
  rw [mul_eq_round ha.1 hb.1]
  exact rnd_good (mult_mul ha.2 hb.2) hL hab hB

theorem abs_mul_le {x y X Y : ℚ} (hx : |x| ≤ X) (hy : |y| ≤ Y) : |x * y| ≤ X * Y := by
  rw [abs_mul]; exact mul_le_mul hx hy (abs_nonneg _) (le_trans (abs_nonneg _) hx)

/-- A relative bound in ℚ, as Filter16.lean's model in ℝ. -/
theorem rel_cast {x r : ℚ} (h : |x - r| ≤ uq * |r|) : |(x : ℝ) - r| ≤ (uq : ℝ) * |(r : ℝ)| := by
  have := (Rat.cast_le (K := ℝ)).mpr h; push_cast at this; exact this

/-- The float sign of a finite `v` whose sign is `E`'s is `sgnQ E`. -/
theorem sgnM_of_sign {v : _root_.F64} {E : ℚ} (hv : v.isFinite) (h1 : 0 < v.val ↔ 0 < E)
    (h2 : v.val < 0 ↔ E < 0) : sgnM v = sgnQ E := by
  have hZ : Z.isFinite := by decide
  have hZ0 : Z.val = 0 := by decide
  unfold sgnM sgnQ
  simp only [F64.Ops.gt_def, F64.Ops.lt_def]
  have e1 : FP.lt Z v = true ↔ 0 < E := by rw [lt_iff hZ hv, hZ0, h1]
  have e2 : FP.lt v Z = true ↔ E < 0 := by rw [lt_iff hv hZ, hZ0, h2]
  simp only [e1, e2]


theorem abs_sub_le2 {a b A B : ℚ} (ha : |a| ≤ A) (hb : |b| ≤ B) : |a - b| ≤ A + B := by
  rw [abs_le] at *; constructor <;> linarith

theorem abs_add_le2 {a b A B : ℚ} (ha : |a| ≤ A) (hb : |b| ≤ B) : |a + b| ≤ A + B := by
  rw [abs_le] at *; constructor <;> linarith

/-- The exact value `disc` decides the sign of. -/
def discQ (px py cx cy r : _root_.F64) : ℚ :=
  (px.val - cx.val) ^ 2 + (py.val - cy.val) ^ 2 - r.val ^ 2

/-- **The filter branch is right**: on the domain, when `|v| > 16ε·t` (all rounded), `v` is
finite and has the exact value's sign (Filter16.lean's `disc_filter_sound`, its model met by
every rounding here, `rel_mult`). -/
theorem disc_filter {px py cx cy r : _root_.F64}
    (hpx : Good (-200) px ∧ |px.val| ≤ C0) (hpy : Good (-200) py ∧ |py.val| ≤ C0)
    (hcx : Good (-200) cx ∧ |cx.val| ≤ C0) (hcy : Good (-200) cy ∧ |cy.val| ≤ C0)
    (hr : Good (-200) r ∧ |r.val| ≤ C0) :
    let dx := FP.sub px cx
    let dy := FP.sub py cy
    let s := FP.add (FP.mul dx dx) (FP.mul dy dy)
    let v := FP.sub s (FP.mul r r)
    let T := FP.mul (F64.Ops.ofBits 4386506037058863104) (FP.add s (FP.mul r r))
    v.isFinite ∧ T.isFinite ∧
      (T.val < |v.val| → (0 < v.val ↔ 0 < discQ px py cx cy r) ∧
        (v.val < 0 ↔ discQ px py cx cy r < 0)) := by
  intro dx dy s v T
  have hC := C0_ge
  have hK := C0_K
  have hs2 := C0_sq
  have hu := uq_pos
  have huc : uq * (C0 * C0) ≤ C0 * C0 :=
    mul_le_of_le_one_left (by positivity) uq_le1
  obtain ⟨gdx, edx, bdx⟩ := sub_good (L := -200) (by norm_num) hpx.1 hcx.1
    (abs_sub_le2 hpx.2 hcx.2) (by linarith)
  obtain ⟨gdy, edy, bdy⟩ := sub_good (L := -200) (by norm_num) hpy.1 hcy.1
    (abs_sub_le2 hpy.2 hcy.2) (by linarith)
  obtain ⟨ga, ea, ba⟩ := mul_good (La := -200) (Lb := -200) (by norm_num) gdx gdx
    (le_trans (abs_mul_le bdx bdx) (le_of_eq (show 2 * (C0 + C0) * (2 * (C0 + C0)) =
      16 * (C0 * C0) by ring))) (by linarith)
  obtain ⟨gb, eb, bb⟩ := mul_good (La := -200) (Lb := -200) (by norm_num) gdy gdy
    (le_trans (abs_mul_le bdy bdy) (le_of_eq (show 2 * (C0 + C0) * (2 * (C0 + C0)) =
      16 * (C0 * C0) by ring))) (by linarith)
  norm_num at ga gb
  obtain ⟨gs, es, bs⟩ := add_good (L := -400) (by norm_num) ga gb
    (le_trans (abs_add_le2 ba bb) (le_of_eq (show 2 * (16 * (C0 * C0)) + 2 * (16 * (C0 * C0)) =
      64 * (C0 * C0) by ring))) (by linarith)
  obtain ⟨gq, eq, bq⟩ := mul_good (La := -200) (Lb := -200) (by norm_num) hr.1 hr.1
    (abs_mul_le hr.2 hr.2) (by linarith)
  norm_num at gq
  obtain ⟨gv, ev, bv⟩ := sub_good (L := -400) (by norm_num) gs gq
    (le_trans (abs_sub_le2 bs bq) (le_of_eq (show 2 * (64 * (C0 * C0)) + 2 * (C0 * C0) =
      130 * (C0 * C0) by ring))) (by linarith)
  obtain ⟨gt, et, bt⟩ := add_good (L := -400) (by norm_num) gs gq
    (le_trans (abs_add_le2 bs bq) (le_of_eq (show 2 * (64 * (C0 * C0)) + 2 * (C0 * C0) =
      130 * (C0 * C0) by ring))) (by linarith)
  have hF : |(F64.Ops.ofBits 4386506037058863104).val| ≤ 16 * uq := by
    rw [FILTER_val, abs_of_pos (by linarith)]
  obtain ⟨gT, eT, -⟩ := mul_good (La := -101) (Lb := -400) (by norm_num) FILTER_good gt
    (abs_mul_le hF bt) (by nlinarith)
  refine ⟨gv.1, gT.1, fun hgt => ?_⟩
  rw [FILTER_val] at eT
  have hu1 : (uq : ℝ) ≤ 1 / 1024 := by
    have := uq_small
    have h : uq ≤ 1 / 1024 := by linarith
    have h' := (Rat.cast_le (K := ℝ)).mpr h; push_cast at h'; exact h'
  have key := Sheet.Filter16.disc_filter_sound (u := (uq : ℝ))
    (DX := (px.val : ℝ) - cx.val) (DY := (py.val : ℝ) - cy.val) (r := (r.val : ℝ))
    (by exact_mod_cast hu.le) hu1
    (by have h := rel_cast edx; push_cast at h; exact h)
    (by have h := rel_cast edy; push_cast at h; exact h)
    (by have h := rel_cast ea; push_cast at h; exact h)
    (by have h := rel_cast eb; push_cast at h; exact h)
    (by have h := rel_cast es; push_cast at h; exact h)
    (by have h := rel_cast eq; push_cast at h; exact h)
    (by have h := rel_cast ev; push_cast at h; exact h)
    (by have h := rel_cast et; push_cast at h; exact h)
    (by have h := rel_cast eT; push_cast at h; exact h)
    (by exact_mod_cast hgt)
  have e : ((px.val : ℝ) - cx.val) * ((px.val : ℝ) - cx.val) +
      ((py.val : ℝ) - cy.val) * ((py.val : ℝ) - cy.val) - (r.val : ℝ) * r.val =
      ((discQ px py cx cy r : ℚ) : ℝ) := by
    unfold discQ; push_cast; ring
  rw [e] at key
  obtain ⟨k1, k2⟩ := key
  constructor
  · rw [← Rat.cast_pos (K := ℝ), k1, Rat.cast_pos]
  · rw [← Rat.cast_lt_zero (K := ℝ), k2, Rat.cast_lt_zero]


theorem usize_big : 2 ^ 31 ≤ Usize.max := by
  have : U32.max ≤ Usize.max := by scalar_tac
  have : U32.max = 2 ^ 32 - 1 := by simp [U32.max_eq]
  omega

/-- **`disc` is exact** (the shipped `exact::disc`): on the domain, under the expansion theory,
it returns `+1`, `−1` or `+0` as the exact `|p − c|² − r²` of the input doubles is positive,
negative or zero. The filter branch is proved outright; the exact branch uses `Tth`. -/
theorem disc_spec (Tth : ExpansionTheory) (px py cx cy r : _root_.F64)
    (hpx : Good (-200) px ∧ |px.val| ≤ C0) (hpy : Good (-200) py ∧ |py.val| ≤ C0)
    (hcx : Good (-200) cx ∧ |cx.val| ≤ C0) (hcy : Good (-200) cy ∧ |cy.val| ≤ C0)
    (hr : Good (-200) r ∧ |r.val| ≤ C0) :
    exact.disc px py cx cy r ⦃ s => s = sgnQ (discQ px py cx cy r) ⦄ := by
  obtain ⟨hv, hT, hflt⟩ := disc_filter hpx hpy hcx hcy hr
  unfold exact.disc
  simp only [lift, bind_ok, core.f64.F64.abs_eq, FILTER_eq, F64.Ops.sub_def, F64.Ops.add_def,
    F64.Ops.mul_def]
  split
  · rename_i hg
    rw [sgn_eq]; simp only [WP.spec_ok]
    simp only [F64.Ops.gt_def] at hg
    rw [lt_iff hT (by simpa using hv), abs_val] at hg
    obtain ⟨k1, k2⟩ := hflt hg
    exact sgnM_of_sign hv k1 k2
  · have hmax := usize_big
    obtain ⟨x1, x2, x3, x4⟩ := diff_facts hpx hcx
    obtain ⟨y1, y2, y3, y4⟩ := diff_facts hpy hcy
    obtain ⟨r1, r2, r3, r4⟩ := ofL_good hr.1
    have hC := C0_ge
    obtain ⟨a0, a1, a2, a3, a4⟩ := mul_facts Tth x1 x1 x3 x3 x4 x4
    obtain ⟨b0, b1, b2, b3, b4⟩ := mul_facts Tth y1 y1 y3 y3 y4 y4
    obtain ⟨c0, c1, c2, c3, c4⟩ := mul_facts Tth r1 r1 (by rw [r3]; linarith [hr.2])
      (by rw [r3]; linarith [hr.2]) (by omega) (by omega)
    have hs := addL_len (mulL (diffL px cx) (diffL px cx)) (mulL (diffL py cy) (diffL py cy))
    step with exp_diff_spec px cx as ⟨ex, hex⟩
    step with exp_diff_spec py cy as ⟨ey, hey⟩
    step with exp_of_spec r as ⟨er, her⟩
    step with exp_mul_spec (alloc.vec.Vec.deref ex) (alloc.vec.Vec.deref ex)
      (by simp [hex]; omega) (by simp [hex]; omega) as ⟨m1, hm1⟩
    step with exp_mul_spec (alloc.vec.Vec.deref ey) (alloc.vec.Vec.deref ey)
      (by simp [hey]; omega) (by simp [hey]; omega) as ⟨m2, hm2⟩
    step with exp_add_spec (alloc.vec.Vec.deref m1) (alloc.vec.Vec.deref m2)
      (by simp [hm1, hm2, hex, hey]; omega) as ⟨s1, hs1⟩
    step with exp_mul_spec (alloc.vec.Vec.deref er) (alloc.vec.Vec.deref er)
      (by simp [her]; omega) (by simp [her]; omega) as ⟨m3, hm3⟩
    step with exp_neg_spec (alloc.vec.Vec.deref m3) (by simp [hm3, her]; omega) as ⟨n, hn⟩
    step with exp_add_spec (alloc.vec.Vec.deref s1) (alloc.vec.Vec.deref n)
      (by simp [hs1, hn, hm1, hm2, hm3, hex, hey, her, negL]; omega) as ⟨s2, hs2⟩
    step with exp_sign_spec (alloc.vec.Vec.deref s2) as ⟨g, hg⟩
    simp only [hg, deref_val, hs2, hs1, hn, hm1, hm2, hm3, hex, hey, her]
    rw [disc_model Tth px py cx cy r hpx hpy hcx hcy hr]
    rfl

end forge_sheet_core
