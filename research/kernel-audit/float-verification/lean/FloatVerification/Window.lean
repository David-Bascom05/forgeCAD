/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import FloatVerification.Orient2d

/-!
# The windows imply the predicates' precondition

The kernel's precondition for an exact slow path (`product_exact` in predicates.rs) is that
every product the predicate forms has a zero factor or is a multiple of `2^-1074`, so that its
rounding error is representable. This file proves that `orient2d`'s window gives it for the
products of its rounded differences:
- `grid_of_ge`: a finite double with `|x| ≥ 2^lo` is a multiple of `2^(lo − 52)`;
- `round_grid`: rounding a multiple of `2^g` gives a multiple of `2^g`;
- `orient2d_window_grid`: with every coordinate zero or at least `2^-480` in magnitude, the two
  products `D₁·D₂`, `D₃·D₄` are multiples of `2^-1064`.
-/

namespace FloatVerification

open IEEE754 hiding F64
open IEEE754.FP
open fv_predicates

/-- A multiple of `2^g`. -/
def OnGrid (g : ℤ) (q : ℚ) : Prop := ∃ k : ℤ, q = k * 2 ^ g

lemma onGrid_zero (g : ℤ) : OnGrid g 0 := ⟨0, by simp⟩

lemma onGrid_sub {g : ℤ} {a b : ℚ} (ha : OnGrid g a) (hb : OnGrid g b) : OnGrid g (a - b) := by
  obtain ⟨k, rfl⟩ := ha; obtain ⟨j, rfl⟩ := hb; exact ⟨k - j, by push_cast; ring⟩

lemma onGrid_coarser {g g' : ℤ} (h : g ≤ g') {q : ℚ} (hq : OnGrid g' q) : OnGrid g q := by
  obtain ⟨k, rfl⟩ := hq
  obtain ⟨p, hp⟩ := FP.two_zpow_dvd h
  exact ⟨k * p, by rw [hp]; push_cast; ring⟩

lemma onGrid_mul {g h : ℤ} {a b : ℚ} (ha : OnGrid g a) (hb : OnGrid h b) : OnGrid (g + h) (a * b) := by
  obtain ⟨k, rfl⟩ := ha; obtain ⟨j, rfl⟩ := hb
  exact ⟨k * j, by rw [two_zpow_add]; push_cast; ring⟩

/-- A finite double with `|x| ≥ 2^lo`, or zero, is a multiple of `2^(lo − 52)`. -/
lemma grid_of_ge {x : F64} (hx : x.isFinite) {lo : ℤ} (h : x.val = 0 ∨ 2 ^ lo ≤ |x.val|) :
    OnGrid (lo - 52) x.val := by
  rcases h with h | h
  · rw [h]; exact onGrid_zero _
  cases x with
  | fin s m e hc =>
    have hpos : (0 : ℚ) < 2 ^ e := two_zpow_pos e
    have hv : |(fin s m e hc : F64).val| = m * 2 ^ e := by
      cases s <;> simp [val, sgn, abs_mul, abs_of_pos hpos]
    rw [hv] at h
    -- `m < 2^53`, so `2^lo ≤ m·2^e < 2^(53+e)` and `e ≥ lo − 52`
    have hm : (m : ℚ) < 2 ^ (53 : ℤ) := by
      have h53 : m < 2 ^ 53 := hc.2.1
      rw [show (53 : ℤ) = ((53 : ℕ) : ℤ) from rfl, zpow_natCast]; exact_mod_cast h53
    have : (2 : ℚ) ^ lo < 2 ^ ((53 : ℤ) + e) := by
      rw [two_zpow_add]; exact lt_of_le_of_lt h (mul_lt_mul_of_pos_right hm hpos)
    have he := two_zpow_lt_iff.mp this
    refine onGrid_coarser (g' := e) (by omega) ⟨if s then -(m : ℤ) else m, ?_⟩
    cases s <;> simp [val, sgn]
  | zero s => exact onGrid_zero _
  | _ => simp_all [isFinite]

/-- Rounding a multiple of `2^g` gives a multiple of `2^g`. -/
lemma round_grid {zs : Bool} {g : ℤ} {q : ℚ} (hq : OnGrid g q)
    (hfin : (round binary64 zs q).isFinite) : OnGrid g (round binary64 zs q).val := by
  by_cases h0 : q = 0
  · subst h0; simp [val]; exact onGrid_zero g
  -- the rounding is `±n·2^e`; if `e ≥ g` it is on the grid, otherwise it is exact
  obtain ⟨k, hk⟩ := hq
  have ha : 0 < |q| := abs_pos.mpr h0
  set e := (roundPos binary64 |q|).2
  by_cases hle : g ≤ e
  · have hU : OnGrid g (uround binary64 |q|) :=
      onGrid_coarser hle ⟨(roundPos binary64 |q|).1, rfl⟩
    have hv := abs_round_val (K := ℚ) (f := binary64) h0 hfin
    simp only [Rat.cast_id] at hv
    rcases abs_cases (round binary64 zs q).val with ⟨h1, _⟩ | ⟨h1, _⟩
    · rw [← h1, hv]; exact hU
    · obtain ⟨j, hj⟩ := hU
      refine ⟨-j, ?_⟩
      have : (round binary64 zs q).val = -|(round binary64 zs q).val| := by rw [h1]; ring
      rw [this, hv, hj]; push_cast; ring
  · -- `e < g`: `|q| / 2^e` is an integer, so the rounding is exact
    have hlt : e < g := lt_of_not_ge hle
    have hint : (rne (|q| / 2 ^ e) : ℚ) = |q| / 2 ^ e := by
      obtain ⟨p, hp⟩ := FP.two_zpow_dvd hlt.le
      have : |q| / 2 ^ e = ((|k| * p).toNat : ℚ) := by
        rw [hk, abs_mul, abs_of_pos (a := (2 : ℚ) ^ g) (two_zpow_pos g), hp]
        have hp0 : 0 ≤ p := by
          have := two_zpow_pos (K := ℚ) g; rw [hp] at this
          have h2 := two_zpow_pos (K := ℚ) e
          by_contra hc
          have : (p : ℚ) < 0 := by exact_mod_cast lt_of_not_ge hc
          nlinarith
        rw [show (((|k| * p).toNat : ℕ) : ℚ) = ((|k| * p : ℤ) : ℚ) by
          rw [← Int.cast_natCast, Int.toNat_of_nonneg (mul_nonneg (abs_nonneg k) hp0)]]
        push_cast; field_simp [(two_zpow_pos (K := ℚ) e).ne']
      rw [this, rne_natCast]
    have hU : uround binary64 |q| = |q| := by
      simp only [uround]; rw [roundPos_fst, hint, div_mul_cancel₀ _ (two_zpow_pos _).ne']
    have hv := round_val_abs (K := ℚ) (f := binary64) h0 hfin
    simp only [Rat.cast_id] at hv
    rw [hU, _root_.sub_self, abs_zero, abs_eq_zero, sub_eq_zero] at hv
    rw [hv]; exact ⟨k, hk⟩

/-- **`orient2d`'s window implies its precondition.** If every coordinate is zero or at least
`2^-480` in magnitude, the products of the rounded differences are multiples of `2^-1064`:
their rounding error is representable (`-1064 ≥ -1074`), even when they are subnormal. -/
theorem orient2d_window_grid (a b c : Point2)
    (hax : a.x.isFinite) (hay : a.y.isFinite) (hbx : b.x.isFinite) (hby : b.y.isFinite)
    (hcx : c.x.isFinite) (hcy : c.y.isFinite)
    (hw : ∀ x ∈ [a.x, a.y, b.x, b.y, c.x, c.y], x.val = 0 ∨ 2 ^ (-480 : ℤ) ≤ |x.val|)
    (h1 : (sub a.x c.x).isFinite) (h2 : (sub b.y c.y).isFinite)
    (h3 : (sub a.y c.y).isFinite) (h4 : (sub b.x c.x).isFinite) :
    OnGrid (-1064) ((sub a.x c.x).val * (sub b.y c.y).val) ∧
      OnGrid (-1064) ((sub a.y c.y).val * (sub b.x c.x).val) := by
  have g : ∀ x : F64, x.isFinite → x ∈ [a.x, a.y, b.x, b.y, c.x, c.y] → OnGrid (-532) x.val :=
    fun x hx hm => by simpa using grid_of_ge hx (hw x hm)
  have d : ∀ p q : F64, p.isFinite → q.isFinite → (sub p q).isFinite →
      OnGrid (-532) p.val → OnGrid (-532) q.val → OnGrid (-532) (sub p q).val := by
    intro p q hp hq' hs gp gq
    rw [sub_eq_round hp hq'] at hs ⊢
    exact round_grid (onGrid_sub gp gq) hs
  have gx := g a.x hax (by simp); have gy := g a.y hay (by simp)
  have gbx := g b.x hbx (by simp); have gby := g b.y hby (by simp)
  have gcx := g c.x hcx (by simp); have gcy := g c.y hcy (by simp)
  constructor
  · simpa using onGrid_mul (d _ _ hax hcx h1 gx gcx) (d _ _ hby hcy h2 gby gcy)
  · simpa using onGrid_mul (d _ _ hay hcy h3 gy gcy) (d _ _ hbx hcx h4 gbx gcx)

end FloatVerification
