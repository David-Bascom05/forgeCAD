import ForgeSheetCore.Funs
import AeneasF64.Lemmas
import IEEE754.Computed
import IEEE754.RoundProps
open Aeneas Aeneas.Std Result

/-!
# `place`: forward error of the development's rigid motions, through the f64 bridge

Every float operation of the translated `forge_sheet_core::place` is `verify/ieee754`'s binary64
operation (the bridge's `*_def` lemmas), and each rounds the exact result within `u |r| + η`
(`u = 2⁻⁵³`, `η = 2⁻¹⁰⁷⁵`: `IEEE754.round_err`, subnormals included). `apply_err`: the computed
`m(p)` is within `5 u S + 5 η` of the exact `o + x p.x + x⊥ p.y` of the doubles' values, componentwise,
where `S = |o| + |x.x p.x| + |x.y p.y|` (resp. with `x.y p.x`, `x.x p.y`). The precondition is that
the outputs are finite (debug-checkable); every intermediate result is then finite.
-/

namespace forge_sheet_core

open IEEE754

noncomputable def uu : ℚ := 2 ^ (-53 : ℤ)
noncomputable def η : ℚ := 2 ^ (-1075 : ℤ)

theorem uu_pos : 0 < uu := by unfold uu; positivity
theorem uu_small : uu ≤ 1 / 8 := by
  unfold uu; rw [zpow_neg, inv_le_comm₀ (by positivity) (by norm_num)]
  have : (8 : ℚ) = 2 ^ (3 : ℤ) := by norm_num
  rw [one_div, inv_inv, this]
  exact zpow_le_zpow_right₀ (by norm_num) (by norm_num)
theorem η_pos : 0 < η := by unfold η; positivity

/-- One rounding, `u |r| + η`. -/
theorem round_le {zs : Bool} {r : ℚ} (hfin : (round binary64 zs r).isFinite) :
    |((round binary64 zs r).val : ℚ) - r| ≤ uu * |r| + η := by
  have := round_err binary64 (K := ℚ) hfin
  simp only [binary64_prec, binary64_qmin] at this
  have e53 : (2 : ℚ) ^ (-((53 : ℕ) : ℤ)) = uu := by unfold uu; norm_num
  rw [e53] at this
  refine le_trans this (max_le ?_ ?_)
  · have := η_pos; linarith
  · unfold η; have := abs_nonneg r; have : (0 : ℚ) ≤ uu * |r| := mul_nonneg uu_pos.le this
    norm_num at this ⊢; linarith

theorem add_err {a b : IEEE754.F64} (h : (FP.add a b).isFinite) :
    |(FP.add a b).val - (a.val + b.val)| ≤ uu * |a.val + b.val| + η := by
  obtain ⟨ha, hb⟩ := isFinite_of_add h
  rw [add_eq_round ha hb] at h ⊢; exact round_le h

theorem mul_err {a b : IEEE754.F64} (h : (FP.mul a b).isFinite) :
    |(FP.mul a b).val - a.val * b.val| ≤ uu * |a.val * b.val| + η := by
  obtain ⟨ha, hb⟩ := isFinite_of_mul h
  rw [mul_eq_round ha hb] at h ⊢; exact round_le h

theorem sub_err {a b : IEEE754.F64} (h : (FP.sub a b).isFinite) :
    |(FP.sub a b).val - (a.val - b.val)| ≤ uu * |a.val - b.val| + η := by
  obtain ⟨ha, hb⟩ := isFinite_of_sub h
  rw [sub_eq_round ha hb] at h ⊢; exact round_le h

/-- **Three roundings of `o + x·p − y·q`** (`apply`'s order: `(o ⊕ x⊗p) ⊕ (−y)⊗q`): within
`5 u S + 5 η`, `S = |o| + |x p| + |y q|`. Pure arithmetic on the per-step bounds. -/
theorem three_round {o xp yq t a s r : ℚ}
    (ht : |t - xp| ≤ uu * |xp| + η) (ha : |a - (o + t)| ≤ uu * |o + t| + η)
    (hs : |s - yq| ≤ uu * |yq| + η) (hr : |r - (a + s)| ≤ uu * |a + s| + η) :
    |r - (o + xp + yq)| ≤ 5 * uu * (|o| + |xp| + |yq|) + 5 * η := by
  have hu := uu_pos
  have hu8 := uu_small
  have he := η_pos
  set S := |o| + |xp| + |yq|
  have hS : |o| + |xp| + |yq| = S := rfl
  have h0 := abs_nonneg o
  have h1 := abs_nonneg xp
  have h2 := abs_nonneg yq
  -- magnitudes
  have mt : |t| ≤ (1 + uu) * |xp| + η := by
    have := abs_sub_abs_le_abs_sub t xp; nlinarith
  have mot : |o + t| ≤ (1 + uu) * S + η := by
    have := abs_add_le o t; nlinarith
  have ma : |a| ≤ (1 + uu) * |o + t| + η := by
    have := abs_sub_abs_le_abs_sub a (o + t); nlinarith [abs_nonneg (o + t)]
  have ms : |s| ≤ (1 + uu) * |yq| + η := by
    have := abs_sub_abs_le_abs_sub s yq; nlinarith
  have mot' : |o + t| ≤ (1 + uu) * (|o| + |xp|) + η := by
    have := abs_add_le o t; nlinarith
  have mas : |a + s| ≤ (1 + uu) ^ 2 * S + 4 * η := by
    have := abs_add_le a s
    have k1 : (1 + uu) * |o + t| ≤ (1 + uu) * ((1 + uu) * (|o| + |xp|) + η) :=
      mul_le_mul_of_nonneg_left mot' (by linarith)
    have k2 : (1 + uu) * |yq| ≤ (1 + uu) ^ 2 * |yq| := by
      apply mul_le_mul_of_nonneg_right _ h2; nlinarith
    have k3 : (1 + uu) * η ≤ 2 * η := by nlinarith
    have e : (1 + uu) * ((1 + uu) * (|o| + |xp|) + η) = (1 + uu) ^ 2 * (|o| + |xp|) + (1 + uu) * η := by ring
    have e2 : (1 + uu) ^ 2 * S = (1 + uu) ^ 2 * (|o| + |xp|) + (1 + uu) ^ 2 * |yq| := by
      rw [← hS]; ring
    linarith
  -- the error, step by step
  have tri : |r - (o + xp + yq)| ≤ |r - (a + s)| + |a - (o + t)| + |t - xp| + |s - yq| := by
    have : r - (o + xp + yq) = (r - (a + s)) + (a - (o + t)) + (t - xp) + (s - yq) := by ring
    rw [this]
    have := abs_add_le ((r - (a + s)) + (a - (o + t)) + (t - xp)) (s - yq)
    have := abs_add_le ((r - (a + s)) + (a - (o + t))) (t - xp)
    have := abs_add_le (r - (a + s)) (a - (o + t))
    linarith
  have e1 : uu * |a + s| ≤ uu * ((1 + uu) ^ 2 * S + 4 * η) := mul_le_mul_of_nonneg_left mas hu.le
  have e2 : uu * |o + t| ≤ uu * ((1 + uu) * S + η) := mul_le_mul_of_nonneg_left mot hu.le
  have e3 : uu * |xp| ≤ uu * S := mul_le_mul_of_nonneg_left (by linarith) hu.le
  have e4 : uu * |yq| ≤ uu * S := mul_le_mul_of_nonneg_left (by linarith) hu.le
  have hSn : 0 ≤ S := by linarith
  have k : uu * ((1 + uu) ^ 2 * S + 4 * η) + uu * ((1 + uu) * S + η) + uu * S + uu * S + 4 * η ≤
      5 * uu * S + 5 * η := by
    have e : uu * ((1 + uu) ^ 2 * S + 4 * η) + uu * ((1 + uu) * S + η) + uu * S + uu * S + 4 * η =
        5 * uu * S + 5 * η + (uu * S) * (3 * uu + uu * uu - 1) + η * (5 * uu - 1) := by ring
    rw [e]
    have q1 : (uu * S) * (3 * uu + uu * uu - 1) ≤ 0 :=
      mul_nonpos_of_nonneg_of_nonpos (mul_nonneg hu.le hSn) (by nlinarith)
    have q2 : η * (5 * uu - 1) ≤ 0 := mul_nonpos_of_nonneg_of_nonpos he.le (by linarith)
    linarith
  linarith

/-- The rounded `(o ⊕ x⊗p) ⊕ (−y)⊗q` of doubles, when finite, is within `5 u S + 5 η` of
`o + x p − y q`. -/
theorem fma3_err (o x p y q : IEEE754.F64)
    (h : (FP.add (FP.add o (FP.mul x p)) (FP.mul (FP.neg y) q)).isFinite) :
    |(FP.add (FP.add o (FP.mul x p)) (FP.mul (FP.neg y) q)).val - (o.val + x.val * p.val - y.val * q.val)| ≤
      5 * uu * (|o.val| + |x.val * p.val| + |y.val * q.val|) + 5 * η := by
  obtain ⟨h1, h2⟩ := isFinite_of_add h
  obtain ⟨_, h3⟩ := isFinite_of_add h1
  have := three_round (mul_err h3) (add_err h1) (mul_err h2) (add_err h)
  simp only [FP.val_neg, neg_mul, abs_neg] at this
  have e : o.val + x.val * p.val - y.val * q.val = o.val + x.val * p.val + -(y.val * q.val) := by ring
  rw [e]; exact this

/-- The rounded `(o ⊕ x⊗p) ⊕ y⊗q`, when finite, is within `5 u S + 5 η` of `o + x p + y q`. -/
theorem fma3_err' (o x p y q : IEEE754.F64)
    (h : (FP.add (FP.add o (FP.mul x p)) (FP.mul y q)).isFinite) :
    |(FP.add (FP.add o (FP.mul x p)) (FP.mul y q)).val - (o.val + x.val * p.val + y.val * q.val)| ≤
      5 * uu * (|o.val| + |x.val * p.val| + |y.val * q.val|) + 5 * η := by
  obtain ⟨h1, h2⟩ := isFinite_of_add h
  obtain ⟨_, h3⟩ := isFinite_of_add h1
  exact three_round (mul_err h3) (add_err h1) (mul_err h2) (add_err h)

/-- **`apply` refines the exact motion**: each computed coordinate, when finite, is within
`5 u S + 5 η` of `o + x p.x + x⊥ p.y` on the doubles' values. -/
theorem apply_spec (m : place.Place) (px py : IEEE754.F64) :
    place.apply m px py ⦃ r =>
      (r.1.isFinite → |r.1.val - (m.ox.val + m.xx.val * px.val - m.xy.val * py.val)| ≤
        5 * uu * (|m.ox.val| + |m.xx.val * px.val| + |m.xy.val * py.val|) + 5 * η) ∧
      (r.2.isFinite → |r.2.val - (m.oy.val + m.xy.val * px.val + m.xx.val * py.val)| ≤
        5 * uu * (|m.oy.val| + |m.xy.val * px.val| + |m.xx.val * py.val|) + 5 * η) ⦄ := by
  unfold place.apply
  simp only [lift, Std.bind_ok]
  simp only [WP.spec_ok, F64.Ops.add_def, F64.Ops.mul_def, F64.Ops.neg_def]
  exact ⟨fun h => fma3_err _ _ _ _ _ h, fun h => fma3_err' _ _ _ _ _ h⟩

/-- **Two roundings of a sum of two rounded products** (`vec`'s order: `x⊗p ⊕ y⊗q`): within
`3 u (|x p| + |y q|) + 4 η`. -/
theorem two_round {xp yq t s r : ℚ} (ht : |t - xp| ≤ uu * |xp| + η) (hs : |s - yq| ≤ uu * |yq| + η)
    (hr : |r - (t + s)| ≤ uu * |t + s| + η) :
    |r - (xp + yq)| ≤ 3 * uu * (|xp| + |yq|) + 4 * η := by
  have hu := uu_pos
  have hu8 := uu_small
  have he := η_pos
  have h1 := abs_nonneg xp
  have h2 := abs_nonneg yq
  have mt : |t| ≤ (1 + uu) * |xp| + η := by have := abs_sub_abs_le_abs_sub t xp; nlinarith
  have ms : |s| ≤ (1 + uu) * |yq| + η := by have := abs_sub_abs_le_abs_sub s yq; nlinarith
  have mts : |t + s| ≤ (1 + uu) * (|xp| + |yq|) + 2 * η := by have := abs_add_le t s; linarith
  have tri : |r - (xp + yq)| ≤ |r - (t + s)| + |t - xp| + |s - yq| := by
    have : r - (xp + yq) = (r - (t + s)) + (t - xp) + (s - yq) := by ring
    rw [this]
    have := abs_add_le ((r - (t + s)) + (t - xp)) (s - yq)
    have := abs_add_le (r - (t + s)) (t - xp)
    linarith
  have e1 : uu * |t + s| ≤ uu * ((1 + uu) * (|xp| + |yq|) + 2 * η) := mul_le_mul_of_nonneg_left mts hu.le
  have hP : 0 ≤ |xp| + |yq| := by linarith
  have e : uu * ((1 + uu) * (|xp| + |yq|) + 2 * η) + η + uu * |xp| + η + uu * |yq| + η =
      3 * uu * (|xp| + |yq|) + 3 * η + (uu * (|xp| + |yq|)) * (uu - 1) + η * (2 * uu) := by ring
  have q1 : (uu * (|xp| + |yq|)) * (uu - 1) ≤ 0 :=
    mul_nonpos_of_nonneg_of_nonpos (mul_nonneg hu.le hP) (by linarith)
  have q2 : η * (2 * uu) ≤ η := by nlinarith
  linarith

/-- **`vec` refines the exact rotation**: each computed coordinate, when finite, is within
`3 u (|x.x v.x| + |x.y v.y|) + 4 η` of `x v.x + x⊥ v.y`. -/
theorem vec_spec (m : place.Place) (vx vy : IEEE754.F64) :
    place.vec m vx vy ⦃ r =>
      (r.1.isFinite → |r.1.val - (m.xx.val * vx.val - m.xy.val * vy.val)| ≤
        3 * uu * (|m.xx.val * vx.val| + |m.xy.val * vy.val|) + 4 * η) ∧
      (r.2.isFinite → |r.2.val - (m.xy.val * vx.val + m.xx.val * vy.val)| ≤
        3 * uu * (|m.xy.val * vx.val| + |m.xx.val * vy.val|) + 4 * η) ⦄ := by
  unfold place.vec
  simp only [lift, Std.bind_ok]
  simp only [WP.spec_ok, F64.Ops.add_def, F64.Ops.mul_def, F64.Ops.neg_def]
  refine ⟨fun h => ?_, fun h => ?_⟩
  · obtain ⟨h1, h2⟩ := isFinite_of_add h
    have := two_round (mul_err h1) (mul_err h2) (add_err h)
    simp only [FP.val_neg, neg_mul, abs_neg, ← sub_eq_add_neg] at this
    have he := η_pos
    linarith
  · obtain ⟨h1, h2⟩ := isFinite_of_add h
    have := two_round (mul_err h1) (mul_err h2) (add_err h)
    have he := η_pos
    linarith

/-- **`then` refines the exact composition** `m ∘ inner`: its origin is `m(inner.o)` and its axis
`m.x ⊗ inner.x`, each coordinate, when finite, within the `apply`/`vec` bounds of the exact
values. Along a path of joints the development composes these (`develop` of Tree.lean). -/
theorem then_spec (m inner : place.Place) :
    place.then m inner ⦃ r =>
      (r.ox.isFinite → |r.ox.val - (m.ox.val + m.xx.val * inner.ox.val - m.xy.val * inner.oy.val)| ≤
        5 * uu * (|m.ox.val| + |m.xx.val * inner.ox.val| + |m.xy.val * inner.oy.val|) + 5 * η) ∧
      (r.oy.isFinite → |r.oy.val - (m.oy.val + m.xy.val * inner.ox.val + m.xx.val * inner.oy.val)| ≤
        5 * uu * (|m.oy.val| + |m.xy.val * inner.ox.val| + |m.xx.val * inner.oy.val|) + 5 * η) ∧
      (r.xx.isFinite → |r.xx.val - (m.xx.val * inner.xx.val - m.xy.val * inner.xy.val)| ≤
        3 * uu * (|m.xx.val * inner.xx.val| + |m.xy.val * inner.xy.val|) + 4 * η) ∧
      (r.xy.isFinite → |r.xy.val - (m.xy.val * inner.xx.val + m.xx.val * inner.xy.val)| ≤
        3 * uu * (|m.xy.val * inner.xx.val| + |m.xx.val * inner.xy.val|) + 4 * η) ⦄ := by
  unfold place.then
  step with apply_spec m inner.ox inner.oy as ⟨ox, oy, ha1, ha2⟩
  step with vec_spec m inner.xx inner.xy as ⟨xx, xy, hv1, hv2⟩
  exact ⟨ha1, ha2, hv1, hv2⟩

/-- A rounded difference times a factor at most 2, rounded: within `3 u |D x| + 4 η` of the exact
`D x`. -/
theorem diff_prod {D d x t : ℚ} (hd : |d - D| ≤ uu * |D| + η) (hx : |x| ≤ 2)
    (ht : |t - d * x| ≤ uu * |d * x| + η) : |t - D * x| ≤ 3 * uu * |D * x| + 4 * η := by
  have hu := uu_pos
  have hu8 := uu_small
  have he := η_pos
  have hD := abs_nonneg D
  have hxn := abs_nonneg x
  have md : |d| ≤ (1 + uu) * |D| + η := by have := abs_sub_abs_le_abs_sub d D; nlinarith
  have e1 : |t - D * x| ≤ |t - d * x| + |d - D| * |x| := by
    have : t - D * x = (t - d * x) + (d - D) * x := by ring
    rw [this]
    calc |(t - d * x) + (d - D) * x| ≤ |t - d * x| + |(d - D) * x| := abs_add_le _ _
      _ = |t - d * x| + |d - D| * |x| := by rw [abs_mul]
  have e2 : |d * x| ≤ ((1 + uu) * |D| + η) * |x| := by
    rw [abs_mul]; exact mul_le_mul_of_nonneg_right md hxn
  have e3 : |d - D| * |x| ≤ (uu * |D| + η) * |x| := mul_le_mul_of_nonneg_right hd hxn
  rw [abs_mul]
  have : uu * |d * x| ≤ uu * (((1 + uu) * |D| + η) * |x|) := mul_le_mul_of_nonneg_left e2 hu.le
  have hux : uu * |x| ≤ 1 := by nlinarith
  have k1 : uu * (η * |x|) ≤ η := by
    calc uu * (η * |x|) = η * (uu * |x|) := by ring
      _ ≤ η * 1 := mul_le_mul_of_nonneg_left hux he.le
      _ = η := by ring
  have k2 : η * |x| ≤ 2 * η := by nlinarith
  have k3 : uu * ((1 + uu) * |D| * |x|) + uu * |D| * |x| ≤ 3 * uu * (|D| * |x|) := by
    have : uu * uu * (|D| * |x|) ≤ uu * (|D| * |x|) := by
      apply mul_le_mul_of_nonneg_right _ (mul_nonneg hD hxn); nlinarith
    nlinarith
  nlinarith

/-- Two such products, summed and rounded: within `5 u (|P| + |Q|) + 10 η`. -/
theorem two_round' {P Q t s r : ℚ} (ht : |t - P| ≤ 3 * uu * |P| + 4 * η)
    (hs : |s - Q| ≤ 3 * uu * |Q| + 4 * η) (hr : |r - (t + s)| ≤ uu * |t + s| + η) :
    |r - (P + Q)| ≤ 5 * uu * (|P| + |Q|) + 10 * η := by
  have hu := uu_pos
  have hu8 := uu_small
  have he := η_pos
  have h1 := abs_nonneg P
  have h2 := abs_nonneg Q
  have mt : |t| ≤ (1 + 3 * uu) * |P| + 4 * η := by have := abs_sub_abs_le_abs_sub t P; nlinarith
  have ms : |s| ≤ (1 + 3 * uu) * |Q| + 4 * η := by have := abs_sub_abs_le_abs_sub s Q; nlinarith
  have mts : |t + s| ≤ (1 + 3 * uu) * (|P| + |Q|) + 8 * η := by have := abs_add_le t s; linarith
  have tri : |r - (P + Q)| ≤ |r - (t + s)| + |t - P| + |s - Q| := by
    have : r - (P + Q) = (r - (t + s)) + (t - P) + (s - Q) := by ring
    rw [this]
    have := abs_add_le ((r - (t + s)) + (t - P)) (s - Q)
    have := abs_add_le (r - (t + s)) (t - P)
    linarith
  have e1 : uu * |t + s| ≤ uu * ((1 + 3 * uu) * (|P| + |Q|) + 8 * η) := mul_le_mul_of_nonneg_left mts hu.le
  have hP : 0 ≤ |P| + |Q| := by linarith
  have q1 : uu * (3 * uu * (|P| + |Q|)) ≤ uu * (|P| + |Q|) := by
    apply mul_le_mul_of_nonneg_left _ hu.le; nlinarith
  have q2 : uu * (8 * η) ≤ η := by nlinarith
  nlinarith

/-- **`local` refines the exact inverse motion of a point**, for an axis with components at most
2 (the kernel's are unit): each coordinate, when finite, within `5 u (…) + 10 η` of
`((p − o)·x, (p − o)·x⊥)` on the doubles' values. -/
theorem local_spec (m : place.Place) (px py : IEEE754.F64) (hxx : |m.xx.val| ≤ 2)
    (hxy : |m.xy.val| ≤ 2) :
    place.local m px py ⦃ r =>
      (r.1.isFinite → |r.1.val - ((px.val - m.ox.val) * m.xx.val + (py.val - m.oy.val) * m.xy.val)| ≤
        5 * uu * (|(px.val - m.ox.val) * m.xx.val| + |(py.val - m.oy.val) * m.xy.val|) + 10 * η) ∧
      (r.2.isFinite → |r.2.val - ((px.val - m.ox.val) * -m.xy.val + (py.val - m.oy.val) * m.xx.val)| ≤
        5 * uu * (|(px.val - m.ox.val) * -m.xy.val| + |(py.val - m.oy.val) * m.xx.val|) + 10 * η) ⦄ := by
  unfold place.local
  simp only [lift, Std.bind_ok]
  simp only [WP.spec_ok, F64.Ops.add_def, F64.Ops.mul_def, F64.Ops.neg_def, F64.Ops.sub_def]
  refine ⟨fun h => ?_, fun h => ?_⟩
  · obtain ⟨h1, h2⟩ := isFinite_of_add h
    obtain ⟨hdx, _⟩ := isFinite_of_mul h1
    obtain ⟨hdy, _⟩ := isFinite_of_mul h2
    exact two_round' (diff_prod (sub_err hdx) hxx (mul_err h1))
      (diff_prod (sub_err hdy) hxy (mul_err h2)) (add_err h)
  · obtain ⟨h1, h2⟩ := isFinite_of_add h
    obtain ⟨hdx, _⟩ := isFinite_of_mul h1
    obtain ⟨hdy, _⟩ := isFinite_of_mul h2
    have hn : |(FP.neg m.xy).val| ≤ 2 := by simpa using hxy
    have := two_round' (diff_prod (sub_err hdx) hn (mul_err h1))
      (diff_prod (sub_err hdy) hxx (mul_err h2)) (add_err h)
    simpa using this

/-- **`inverse` refines the exact inverse**: its origin is `local(m, 0)` (within `local_spec`'s
bound of `(−o·x, −o·x⊥)`), its axis `(x.x, −x.y)` exactly. -/
theorem inverse_spec (m : place.Place) (hxx : |m.xx.val| ≤ 2) (hxy : |m.xy.val| ≤ 2) :
    place.inverse m ⦃ r => r.xx = m.xx ∧ r.xy = FP.neg m.xy ∧
      (r.ox.isFinite → |r.ox.val - ((0 - m.ox.val) * m.xx.val + (0 - m.oy.val) * m.xy.val)| ≤
        5 * uu * (|(0 - m.ox.val) * m.xx.val| + |(0 - m.oy.val) * m.xy.val|) + 10 * η) ∧
      (r.oy.isFinite → |r.oy.val - ((0 - m.ox.val) * -m.xy.val + (0 - m.oy.val) * m.xx.val)| ≤
        5 * uu * (|(0 - m.ox.val) * -m.xy.val| + |(0 - m.oy.val) * m.xx.val|) + 10 * η) ⦄ := by
  unfold place.inverse
  have h0 : (F64.Ops.ofBits 0).val = 0 := by decide
  step with local_spec m (F64.Ops.ofBits 0) (F64.Ops.ofBits 0) hxx hxy as ⟨ox, oy, h1, h2⟩
  simp only [lift, Std.bind_ok, WP.spec_ok, F64.Ops.neg_def]
  rw [h0] at h1 h2
  exact ⟨trivial, trivial, h1, h2⟩

end forge_sheet_core
