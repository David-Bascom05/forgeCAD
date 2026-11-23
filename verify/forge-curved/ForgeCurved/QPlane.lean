import ForgeCurved.QFacet

/-!
# On a sample plane: the trace of a carrier

On the sweep plane `x = c` a carrier `q` restricts to `g(y, z) = q(c, y, z)`, a function on
`ℝ × ℝ`. This file has the facts the in-plane stage of `qsampler_covers` reads:

* `hasFDerivAt_onPlane`: `g`'s derivative is `(∂_y q, ∂_z q)` at `(c, y, z)`;
* `transverseY`, `transverseZ`: `g` is transverse to the level lines of `y` where `∂_z q ≠ 0`,
  and to those of `z` where `∂_y q ≠ 0`;
* `onPlane_expand`: `g(y, z) = A₀(y) + A₁(y) z + zz z²`, with `A₁(y) = xz c + yz y + lz`;
* `no_vertical`: under `frameOk`, no carrier vanishes on a whole vertical segment
  `{y₀} × (s, t)`;
* `avoid`: an open interval holds a level that is no event of a list.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

/-- A carrier on the plane `x = c`. -/
def onPlane (c : ℝ) (q : Quad.Quadric) (w : ℝ × ℝ) : ℝ := qR q (c, w.1, w.2)

/-- The plane's points. -/
def emb (c : ℝ) (w : ℝ × ℝ) : R3 := (c, w.1, w.2)

theorem hasFDerivAt_onPlane (c : ℝ) (q : Quad.Quadric) (w : ℝ × ℝ) :
    HasFDerivAt (onPlane c q)
      (gy q (emb c w) • ContinuousLinearMap.fst ℝ ℝ ℝ + gz q (emb c w) • ContinuousLinearMap.snd ℝ ℝ ℝ) w := by
  have he : HasFDerivAt (emb c)
      ((0 : ℝ × ℝ →L[ℝ] ℝ).prod (ContinuousLinearMap.id ℝ (ℝ × ℝ))) w :=
    (hasFDerivAt_const (𝕜 := ℝ) c w).prodMk (hasFDerivAt_id w)
  have := (hasFDerivAt_qR q (emb c w)).comp w he
  convert this using 1
  · rfl
  · refine ContinuousLinearMap.ext fun v => ?_
    simp [dq, emb]

theorem fderiv_onPlane (c : ℝ) (q : Quad.Quadric) (w v : ℝ × ℝ) :
    fderiv ℝ (onPlane c q) w v = gy q (emb c w) * v.1 + gz q (emb c w) * v.2 := by
  rw [(hasFDerivAt_onPlane c q w).fderiv]; simp

theorem onPlane_contDiff (c : ℝ) (q : Quad.Quadric) : ContDiff ℝ 1 (onPlane c q) := by
  unfold onPlane; exact (qR_contDiff q).comp (by fun_prop)

/-- The level lines of `y`. -/
noncomputable def ℓy : ℝ × ℝ →L[ℝ] ℝ := ContinuousLinearMap.fst ℝ ℝ ℝ
/-- The level lines of `z`. -/
noncomputable def ℓz : ℝ × ℝ →L[ℝ] ℝ := ContinuousLinearMap.snd ℝ ℝ ℝ

theorem transverseY (c : ℝ) (q : Quad.Quadric) (w : ℝ × ℝ) (h : gz q (emb c w) ≠ 0) :
    Transverse ℓy [onPlane c q] w := by
  rintro ⟨s, f⟩
  refine ⟨(s, (f 0 - gy q (emb c w) * s) / gz q (emb c w)), ?_⟩
  simp only [Prod.mk.injEq, ℓy, ContinuousLinearMap.coe_fst', true_and]
  funext i
  fin_cases i
  change fderiv ℝ (onPlane c q) w _ = f 0
  rw [fderiv_onPlane]
  field_simp
  ring

theorem transverseZ (c : ℝ) (q : Quad.Quadric) (w : ℝ × ℝ) (h : gy q (emb c w) ≠ 0) :
    Transverse ℓz [onPlane c q] w := by
  rintro ⟨s, f⟩
  refine ⟨((f 0 - gz q (emb c w) * s) / gy q (emb c w), s), ?_⟩
  simp only [Prod.mk.injEq, ℓz, ContinuousLinearMap.coe_snd', true_and]
  funext i
  fin_cases i
  change fderiv ℝ (onPlane c q) w _ = f 0
  rw [fderiv_onPlane]
  field_simp
  ring

theorem finrank_R2 : Module.finrank ℝ (ℝ × ℝ) = [onPlane 0 default].length + 1 := by
  simp [Module.finrank_prod]

theorem finrank_R2' (g : ℝ × ℝ → ℝ) : Module.finrank ℝ (ℝ × ℝ) = [g].length + 1 := by
  simp [Module.finrank_prod]

theorem onPlane_expand (c : ℝ) (q : Quad.Quadric) (y z : ℝ) :
    onPlane c q (y, z) = (q.xx * c * c + q.xy * c * y + q.yy * y * y + q.lx * c + q.ly * y + q.c) +
      (q.xz * c + q.yz * y + q.lz) * z + q.zz * z * z := by
  simp only [onPlane, qR]; ring

/-- A quadratic `A₀ + A₁ t + Z t²` with three distinct roots is zero. -/
theorem quad_three {A0 A1 Z t1 t2 t3 : ℝ} (h12 : t1 ≠ t2) (h13 : t1 ≠ t3) (h23 : t2 ≠ t3)
    (e1 : A0 + A1 * t1 + Z * t1 * t1 = 0) (e2 : A0 + A1 * t2 + Z * t2 * t2 = 0)
    (e3 : A0 + A1 * t3 + Z * t3 * t3 = 0) : Z = 0 ∧ A1 = 0 := by
  have d12 : A1 + Z * (t1 + t2) = 0 := by
    have : (t1 - t2) * (A1 + Z * (t1 + t2)) = 0 := by linear_combination e1 - e2
    rcases mul_eq_zero.mp this with h | h
    · exact absurd (sub_eq_zero.mp h) h12
    · exact h
  have d13 : A1 + Z * (t1 + t3) = 0 := by
    have : (t1 - t3) * (A1 + Z * (t1 + t3)) = 0 := by linear_combination e1 - e3
    rcases mul_eq_zero.mp this with h | h
    · exact absurd (sub_eq_zero.mp h) h13
    · exact h
  have hZ : Z = 0 := by
    have : Z * (t2 - t3) = 0 := by linear_combination d12 - d13
    rcases mul_eq_zero.mp this with h | h
    · exact h
    · exact absurd (sub_eq_zero.mp h) h23
  refine ⟨hZ, ?_⟩
  rw [hZ] at d12; linarith

/-- **No vertical segment** lies in a carrier's trace, under `frameOk`. -/
theorem no_vertical {qs : List Quad.Quadric} (hf : frameOk qs = true) {q : Quad.Quadric}
    (hq : q ∈ qs) (c y : ℝ) {s t : ℝ} (hst : s < t) :
    ¬ ∀ z ∈ Ioo s t, onPlane c q (y, z) = 0 := by
  intro h
  have h1 := h (s + (t - s) / 4) ⟨by linarith, by linarith⟩
  have h2 := h (s + (t - s) / 2) ⟨by linarith, by linarith⟩
  have h3 := h (s + 3 * (t - s) / 4) ⟨by linarith, by linarith⟩
  rw [onPlane_expand] at h1 h2 h3
  obtain ⟨hZ, hA⟩ := quad_three (by intro h; linarith) (by intro h; linarith)
    (by intro h; linarith) h1 h2 h3
  have := List.all_eq_true.mp hf q hq
  simp only [Bool.or_eq_true, Bool.and_eq_true, bne_iff_ne, ne_eq] at this
  rcases this with ⟨hp, hlz⟩ | hzz
  · obtain ⟨-, -, -, -, h5, h6⟩ := isPlaneQ_iff hp
    rw [h5, h6] at hA
    simp at hA
    exact hlz (by exact_mod_cast hA)
  · exact hzz (by exact_mod_cast hZ)

/-- **An open interval holds a level that is no event.** -/
theorem avoid (E : List Num) {a b : ℝ} (hab : a < b) :
    ∃ t ∈ Ioo a b, ∀ e ∈ E, ¬ Denotes realModel e t := by
  obtain ⟨t, ht, htn⟩ := (Ioo_infinite hab).exists_notMem_finset (E.map valE).toFinset
  refine ⟨t, ht, fun e he hd => htn ?_⟩
  rw [valE_eq hd]
  simp only [List.mem_toFinset, List.mem_map]
  exact ⟨e, he, rfl⟩

end ForgeCurved
