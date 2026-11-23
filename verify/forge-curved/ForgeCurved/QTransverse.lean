import ForgeCurved.RealModel
import Mathlib.Analysis.Calculus.FDeriv.Mul
import Mathlib.Analysis.Calculus.FDeriv.Prod

/-!
# Transversality to the sweep, from the gradients

The sweep is `x` (`ℓx`, the first coordinate). A quadric carrier's derivative is its gradient
(`hasFDerivAt_qR`). LemmaD's `Transverse` then reduces to the exact conditions the sampler's
event triples certify:

* **`transverse1`**: one carrier is transverse where `(∂_y q, ∂_z q) ≠ 0`. The triple
  `(q, ∂_y q, ∂_z q)` is an event wherever both vanish;
* **`transverse2`**: two carriers are transverse where `∂_y q_j ∂_z q_k − ∂_z q_j ∂_y q_k ≠ 0`.
  That is `critQ q_j q_k`, and the triple `(q_j, q_k, critQ)` is an event wherever it vanishes.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep

/-- The sweep: `x`. -/
noncomputable def ℓx : R3 →L[ℝ] ℝ := ContinuousLinearMap.fst ℝ ℝ (ℝ × ℝ)

/-- The gradient's components. -/
def gx (q : Quad.Quadric) (p : R3) : ℝ :=
  2 * q.xx * p.1 + q.xy * p.2.1 + q.xz * p.2.2 + q.lx
def gy (q : Quad.Quadric) (p : R3) : ℝ :=
  q.xy * p.1 + 2 * q.yy * p.2.1 + q.yz * p.2.2 + q.ly
def gz (q : Quad.Quadric) (p : R3) : ℝ :=
  q.xz * p.1 + q.yz * p.2.1 + 2 * q.zz * p.2.2 + q.lz

/-- The derivative, as a linear map. -/
noncomputable def dq (q : Quad.Quadric) (p : R3) : R3 →L[ℝ] ℝ :=
  gx q p • ContinuousLinearMap.fst ℝ ℝ (ℝ × ℝ) +
    gy q p • ((ContinuousLinearMap.fst ℝ ℝ ℝ).comp (ContinuousLinearMap.snd ℝ ℝ (ℝ × ℝ))) +
    gz q p • ((ContinuousLinearMap.snd ℝ ℝ ℝ).comp (ContinuousLinearMap.snd ℝ ℝ (ℝ × ℝ)))

theorem dq_apply (q : Quad.Quadric) (p v : R3) :
    dq q p v = gx q p * v.1 + gy q p * v.2.1 + gz q p * v.2.2 := by
  simp [dq]

/-- **A quadric's derivative is its gradient.** -/
theorem hasFDerivAt_qR (q : Quad.Quadric) (p : R3) : HasFDerivAt (qR q) (dq q p) p := by
  have h1 : HasFDerivAt (fun p : R3 => p.1) (ContinuousLinearMap.fst ℝ ℝ (ℝ × ℝ)) p :=
    hasFDerivAt_fst
  have h2 : HasFDerivAt (fun p : R3 => p.2.1)
      ((ContinuousLinearMap.fst ℝ ℝ ℝ).comp (ContinuousLinearMap.snd ℝ ℝ (ℝ × ℝ))) p :=
    hasFDerivAt_fst.comp p hasFDerivAt_snd
  have h3 : HasFDerivAt (fun p : R3 => p.2.2)
      ((ContinuousLinearMap.snd ℝ ℝ ℝ).comp (ContinuousLinearMap.snd ℝ ℝ (ℝ × ℝ))) p :=
    hasFDerivAt_snd.comp p hasFDerivAt_snd
  have H := ((((((((((h1.mul h1).const_mul (q.xx : ℝ)).add ((h2.mul h2).const_mul (q.yy : ℝ))).add
    ((h3.mul h3).const_mul (q.zz : ℝ))).add ((h1.mul h2).const_mul (q.xy : ℝ))).add
    ((h1.mul h3).const_mul (q.xz : ℝ))).add ((h2.mul h3).const_mul (q.yz : ℝ))).add
    (h1.const_mul (q.lx : ℝ))).add (h2.const_mul (q.ly : ℝ))).add (h3.const_mul (q.lz : ℝ))).add_const
    (q.c : ℝ)
  convert H using 1
  · funext x; simp only [qR, Pi.mul_apply, Pi.add_apply]; ring
  · refine ContinuousLinearMap.ext fun v => ?_
    simp [dq, gx, gy, gz]
    ring

theorem fderiv_qR (q : Quad.Quadric) (p : R3) : fderiv ℝ (qR q) p = dq q p :=
  (hasFDerivAt_qR q p).fderiv

/-- **One carrier is transverse where its gradient is not along the sweep.** -/
theorem transverse1 (q : Quad.Quadric) (p : R3) (h : gy q p ≠ 0 ∨ gz q p ≠ 0) :
    Transverse ℓx [qR q] p := by
  rintro ⟨s, f⟩
  set t := f 0
  rcases h with hy | hz
  · refine ⟨(s, (t - gx q p * s) / gy q p, 0), ?_⟩
    simp only [Prod.mk.injEq, ℓx, ContinuousLinearMap.coe_fst', true_and]
    funext i
    fin_cases i
    change fderiv ℝ (qR q) p _ = f 0
    rw [fderiv_qR, dq_apply]
    field_simp
    ring
  · refine ⟨(s, 0, (t - gx q p * s) / gz q p), ?_⟩
    simp only [Prod.mk.injEq, ℓx, ContinuousLinearMap.coe_fst', true_and]
    funext i
    fin_cases i
    change fderiv ℝ (qR q) p _ = f 0
    rw [fderiv_qR, dq_apply]
    field_simp
    ring

/-- **Two carriers are transverse where `critQ` does not vanish.** -/
theorem transverse2 (qj qk : Quad.Quadric) (p : R3)
    (h : gy qj p * gz qk p - gz qj p * gy qk p ≠ 0) : Transverse ℓx [qR qj, qR qk] p := by
  rintro ⟨s, f⟩
  set D := gy qj p * gz qk p - gz qj p * gy qk p
  set u := f 0 - gx qj p * s
  set w := f 1 - gx qk p * s
  have hD : D ≠ 0 := h
  refine ⟨(s, (u * gz qk p - gz qj p * w) / D, (gy qj p * w - u * gy qk p) / D), ?_⟩
  simp only [Prod.mk.injEq, ℓx, ContinuousLinearMap.coe_fst', true_and]
  funext i
  fin_cases i
  · change fderiv ℝ (qR qj) p _ = f 0
    rw [fderiv_qR, dq_apply]
    field_simp
    simp only [u, w, D]
    ring
  · change fderiv ℝ (qR qk) p _ = f 1
    rw [fderiv_qR, dq_apply]
    field_simp
    simp only [u, w, D]
    ring

end ForgeCurved
