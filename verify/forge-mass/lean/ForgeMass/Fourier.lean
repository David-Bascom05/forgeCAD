import ForgeMass.Trig

/-!
# The Fourier fit (mass.rs:1821–1834, 2034–2076)

`dft(samples, k)` takes N = 2k + 1 samples at θⱼ = 2πj/N and returns
* c₀ = (1/N) Σ gⱼ,
* c_m = (2/N) Σ gⱼ cos(mθⱼ), and
* s_m = (2/N) Σ gⱼ sin(mθⱼ),

with `roots` supplying cos and sin of 2π(jm mod N)/N. `Inner::Fourier` then takes
`F(a0 + t) = c₀ + Σ c_m cos(mt) + s_m sin(mt)` and integrates it in closed form.

* `dft_exact`: for a trigonometric polynomial of degree ≤ k, this reconstruction is the function
  itself.
* `G_fourier`: the closed form `g` adds up (mass.rs:2069–2073) is its integral from a0.

`Along::Trig` uses the same fit along b (2m + 1 samples), so `dft_exact` covers it too.
-/

noncomputable section
open Complex Real

namespace ForgeMass

variable (k : ℕ)

def N : ℕ := 2 * k + 1

def θ (j : ℕ) : ℝ := 2 * π * j / N k

def c0 (g : ℝ → ℂ) : ℂ := (1 / N k : ℂ) * ∑ j ∈ Finset.range (N k), g (θ k j)
def cm (g : ℝ → ℂ) (m : ℕ) : ℂ :=
  (2 / N k : ℂ) * ∑ j ∈ Finset.range (N k), g (θ k j) * (Real.cos (m * θ k j) : ℂ)
def sm (g : ℝ → ℂ) (m : ℕ) : ℂ :=
  (2 / N k : ℂ) * ∑ j ∈ Finset.range (N k), g (θ k j) * (Real.sin (m * θ k j) : ℂ)

/-- The fitted trigonometric polynomial. -/
def recon (g : ℝ → ℂ) (t : ℝ) : ℂ :=
  c0 k g + ∑ m ∈ Finset.Icc 1 k, (cm k g m * (Real.cos (m * t) : ℂ) + sm k g m * (Real.sin (m * t) : ℂ))

theorem N_pos : (0 : ℝ) < N k := by unfold N; positivity

/-- Σⱼ e^{ipθⱼ} is N at p = 0 and 0 for 0 < |p| < N. -/
theorem sum_exp (p : ℤ) (hp : |p| < N k) :
    ∑ j ∈ Finset.range (N k), Complex.exp (p * (θ k j : ℂ) * I) = if p = 0 then (N k : ℂ) else 0 := by
  have hN : (N k : ℂ) ≠ 0 := by exact_mod_cast (N_pos k).ne'
  set ζ := Complex.exp (2 * π * p / N k * I)
  have term : ∀ j : ℕ, Complex.exp (p * (θ k j : ℂ) * I) = ζ ^ j := by
    intro j
    rw [← Complex.exp_nat_mul]
    congr 1
    simp only [θ]
    push_cast
    ring
  simp_rw [term]
  split
  · rename_i h0
    subst h0
    simp [ζ]
  · rename_i h0
    have hζ1 : ζ ≠ 1 := by
      intro h1
      rw [Complex.exp_eq_one_iff] at h1
      obtain ⟨n, hn⟩ := h1
      have hpi : (π : ℂ) ≠ 0 := by exact_mod_cast Real.pi_ne_zero
      have : (p : ℂ) = n * N k := by
        field_simp at hn
        linear_combination hn
      have hpz : p = n * (N k : ℤ) := by exact_mod_cast this
      have hn0 : n ≠ 0 := by rintro rfl; simp at hpz; exact h0 hpz
      have : (N k : ℤ) ≤ |p| := by
        rw [hpz, abs_mul]
        have : (1 : ℤ) ≤ |n| := Int.one_le_abs hn0
        nlinarith [abs_nonneg (N k : ℤ), abs_of_nonneg (show (0 : ℤ) ≤ N k by positivity)]
      omega
    rw [geom_sum_eq hζ1]
    have : ζ ^ N k = 1 := by
      rw [← Complex.exp_nat_mul, Complex.exp_eq_one_iff]
      exact ⟨p, by field_simp⟩
    simp [this]

def δ (p : ℤ) : ℂ := if p = 0 then 1 else 0

theorem cos_exp (x : ℝ) : ((Real.cos x : ℝ) : ℂ) = (Complex.exp (x * I) + Complex.exp (-x * I)) / 2 := by
  rw [Complex.ofReal_cos, Complex.cos]

theorem sin_exp (x : ℝ) :
    ((Real.sin x : ℝ) : ℂ) = (Complex.exp (-x * I) - Complex.exp (x * I)) * I / 2 := by
  rw [Complex.ofReal_sin, Complex.sin]

theorem exp_mul_exp (p q : ℤ) (x : ℝ) :
    Complex.exp (p * (x : ℂ) * I) * Complex.exp (q * (x : ℂ) * I) =
      Complex.exp (((p + q : ℤ) : ℂ) * (x : ℂ) * I) := by
  rw [← Complex.exp_add]; congr 1; push_cast; ring

theorem S_delta (p : ℤ) (hp : |p| < N k) :
    ∑ j ∈ Finset.range (N k), Complex.exp (p * (θ k j : ℂ) * I) = N k * δ p := by
  rw [sum_exp k p hp, δ]; split <;> simp

theorem abs_lt_N {p : ℤ} (h1 : -(2 * k : ℤ) ≤ p) (h2 : p ≤ 2 * k) : |p| < (N k : ℤ) := by
  rw [abs_lt]; unfold N; push_cast; constructor <;> omega

theorem c0_ex (q : ℤ) (hq : |q| ≤ k) : c0 k (ex q) = δ q := by
  have hN : (N k : ℂ) ≠ 0 := by exact_mod_cast (N_pos k).ne'
  unfold c0 ex
  have := abs_le.1 hq
  rw [S_delta k q (abs_lt_N k (by omega) (by omega))]
  field_simp

theorem cm_ex (q : ℤ) (hq : |q| ≤ k) (m : ℕ) (hm : m ≤ k) :
    cm k (ex q) m = δ (q + m) + δ (q - m) := by
  have hN : (N k : ℂ) ≠ 0 := by exact_mod_cast (N_pos k).ne'
  unfold cm ex
  have e : ∀ j, Complex.exp (q * (θ k j : ℂ) * I) * ((Real.cos (m * θ k j) : ℝ) : ℂ) =
      (Complex.exp (((q + m : ℤ) : ℂ) * (θ k j : ℂ) * I) +
        Complex.exp (((q - m : ℤ) : ℂ) * (θ k j : ℂ) * I)) / 2 := by
    intro j
    rw [cos_exp, mul_div_assoc', mul_add, ← Complex.exp_add, ← Complex.exp_add]
    push_cast; ring_nf
  have := abs_le.1 hq
  simp_rw [e, ← Finset.sum_div, Finset.sum_add_distrib]
  rw [S_delta k _ (abs_lt_N k (by omega) (by omega)), S_delta k _ (abs_lt_N k (by omega) (by omega))]
  field_simp

theorem sm_ex (q : ℤ) (hq : |q| ≤ k) (m : ℕ) (hm : m ≤ k) :
    sm k (ex q) m = (δ (q - m) - δ (q + m)) * I := by
  have hN : (N k : ℂ) ≠ 0 := by exact_mod_cast (N_pos k).ne'
  unfold sm ex
  have e : ∀ j, Complex.exp (q * (θ k j : ℂ) * I) * ((Real.sin (m * θ k j) : ℝ) : ℂ) =
      (Complex.exp (((q - m : ℤ) : ℂ) * (θ k j : ℂ) * I) -
        Complex.exp (((q + m : ℤ) : ℂ) * (θ k j : ℂ) * I)) * I / 2 := by
    intro j
    rw [sin_exp, mul_div_assoc', ← mul_assoc, mul_sub, ← Complex.exp_add, ← Complex.exp_add]
    push_cast; ring_nf
  have := abs_le.1 hq
  simp_rw [e, ← Finset.sum_div, ← Finset.sum_mul, Finset.sum_sub_distrib]
  rw [S_delta k _ (abs_lt_N k (by omega) (by omega)), S_delta k _ (abs_lt_N k (by omega) (by omega))]
  field_simp

/-- One harmonic of the fit, from the deltas: δ(q+m) e^{−imt} + δ(q−m) e^{imt}. -/
theorem harmonic (q : ℤ) (m : ℕ) (t : ℝ) :
    (δ (q + m) + δ (q - m)) * ((Real.cos (m * t) : ℝ) : ℂ) +
        (δ (q - m) - δ (q + m)) * I * ((Real.sin (m * t) : ℝ) : ℂ) =
      (δ (q + m) + δ (q - m)) * Complex.exp (q * (t : ℂ) * I) := by
  rw [cos_exp, sin_exp]
  have hI : I * I = -1 := Complex.I_mul_I
  unfold δ
  by_cases h1 : q + m = 0 <;> by_cases h2 : q - m = 0
  · have hm : (m : ℤ) = 0 := by omega
    have hq : q = 0 := by omega
    simp [h1, h2, hm, hq]
    have : (m : ℂ) = 0 := by exact_mod_cast hm
    simp [this]
  · have hq : (q : ℂ) = -m := by
      have : q = -(m : ℤ) := by omega
      rw [this]; push_cast; ring
    simp only [h1, h2, ↓reduceIte, add_zero, zero_sub, hq]
    push_cast
    simp only [neg_mul]
    linear_combination (Complex.exp (↑m * ↑t * I) - Complex.exp (-(↑m * ↑t * I))) / 2 * hI
  · have hq : (q : ℂ) = m := by
      have : q = (m : ℤ) := by omega
      rw [this]; push_cast; ring
    simp only [h1, h2, ↓reduceIte, zero_add, sub_zero, hq]
    push_cast
    simp only [neg_mul]
    linear_combination (Complex.exp (-(↑m * ↑t * I)) - Complex.exp (↑m * ↑t * I)) / 2 * hI
  · simp [h1, h2]

theorem recon_ex (q : ℤ) (hq : |q| ≤ k) (t : ℝ) : recon k (ex q) t = ex q t := by
  unfold recon
  rw [c0_ex k q hq]
  have hsum : ∀ m ∈ Finset.Icc 1 k,
      cm k (ex q) m * ((Real.cos (m * t) : ℝ) : ℂ) + sm k (ex q) m * ((Real.sin (m * t) : ℝ) : ℂ) =
        (δ (q + m) + δ (q - m)) * ex q t := by
    intro m hm
    rw [cm_ex k q hq m (Finset.mem_Icc.1 hm).2, sm_ex k q hq m (Finset.mem_Icc.1 hm).2, harmonic]
    rfl
  rw [Finset.sum_congr rfl hsum, ← Finset.sum_mul]
  -- exactly one of q = 0, q = −m, q = m (1 ≤ m ≤ k) holds
  have count : δ q * 1 + ∑ m ∈ Finset.Icc 1 k, (δ (q + m) + δ (q - m)) = 1 := by
    have hqk := abs_le.1 hq
    rcases lt_trichotomy q 0 with hneg | h0 | hpos
    · have hm : (-q).toNat ∈ Finset.Icc 1 k := by
        rw [Finset.mem_Icc]; constructor <;> omega
      rw [Finset.sum_eq_single (-q).toNat]
      · simp only [δ]; split_ifs <;> first | omega | simp
      · intro b _ hb; simp only [δ]; split_ifs <;> first | omega | simp
      · intro h; exact absurd hm h
    · subst h0
      rw [Finset.sum_eq_zero]
      · simp [δ]
      · intro m hm; have := (Finset.mem_Icc.1 hm).1; simp only [δ]; split_ifs <;> first | omega | simp
    · have hm : q.toNat ∈ Finset.Icc 1 k := by
        rw [Finset.mem_Icc]; constructor <;> omega
      rw [Finset.sum_eq_single q.toNat]
      · simp only [δ]; split_ifs <;> first | omega | simp
      · intro b _ hb; simp only [δ]; split_ifs <;> first | omega | simp
      · intro h; exact absurd hm h
  calc δ q + (∑ m ∈ Finset.Icc 1 k, (δ (q + m) + δ (q - m))) * ex q t
      = (δ q * 1 + ∑ m ∈ Finset.Icc 1 k, (δ (q + m) + δ (q - m))) * ex q t := by
        rw [add_mul, mul_one]
        congr 1
        unfold δ ex; split_ifs with h
        · subst h; simp
        · simp
    _ = ex q t := by rw [count, one_mul]

theorem recon_add (g h : ℝ → ℂ) (t : ℝ) : recon k (g + h) t = recon k g t + recon k h t := by
  simp only [recon, c0, cm, sm, Pi.add_apply, add_mul, Finset.sum_add_distrib, mul_add]
  ring

theorem c0_smul (c : ℂ) (g : ℝ → ℂ) : c0 k (c • g) = c * c0 k g := by
  simp only [c0, Pi.smul_apply, smul_eq_mul, ← Finset.mul_sum]; ring

theorem cm_smul (c : ℂ) (g : ℝ → ℂ) (m : ℕ) : cm k (c • g) m = c * cm k g m := by
  simp only [cm, Pi.smul_apply, smul_eq_mul, mul_assoc, ← Finset.mul_sum]; ring

theorem sm_smul (c : ℂ) (g : ℝ → ℂ) (m : ℕ) : sm k (c • g) m = c * sm k g m := by
  simp only [sm, Pi.smul_apply, smul_eq_mul, mul_assoc, ← Finset.mul_sum]; ring

theorem recon_smul (c : ℂ) (g : ℝ → ℂ) (t : ℝ) : recon k (c • g) t = c * recon k g t := by
  unfold recon
  rw [c0_smul, mul_add, Finset.mul_sum]
  congr 1
  apply Finset.sum_congr rfl
  intro m _
  rw [cm_smul, sm_smul]; ring

/-- The fit reproduces every trigonometric polynomial of degree ≤ k (mass.rs:1821–1834). -/
theorem dft_exact {g : ℝ → ℂ} (hg : g ∈ Trig k) (t : ℝ) : recon k g t = g t := by
  induction hg using Submodule.span_induction generalizing t with
  | mem x hx =>
    obtain ⟨q, hq, rfl⟩ := hx
    exact recon_ex k q hq t
  | zero =>
    have := recon_smul k 0 0 t
    simpa using this
  | add x y _ _ hx hy => rw [recon_add, hx, hy]; rfl
  | smul a x _ hx => rw [recon_smul, hx]; rfl

/-- Shifting keeps the degree: F(a0 + ·) is in `Trig k` when F is. -/
theorem trig_shift {g : ℝ → ℂ} (hg : g ∈ Trig k) (a0 : ℝ) : (fun t => g (a0 + t)) ∈ Trig k := by
  induction hg using Submodule.span_induction with
  | mem x hx =>
    obtain ⟨q, hq, rfl⟩ := hx
    have : (fun t => ex q (a0 + t)) = Complex.exp (q * a0 * I) • ex q := by
      funext t; simp only [ex, Pi.smul_apply, smul_eq_mul, ← Complex.exp_add]; congr 1; push_cast; ring
    rw [this]
    exact Submodule.smul_mem _ _ (ex_mem hq)
  | zero => exact Submodule.zero_mem _
  | add x y _ _ hx hy => exact Submodule.add_mem _ hx hy
  | smul a x _ hx => exact Submodule.smul_mem _ a hx

/-! ## The integral in closed form (mass.rs:2069–2073) -/

theorem int_cos (m : ℕ) (hm : 0 < m) (x : ℝ) :
    ∫ t in (0 : ℝ)..x, Real.cos (m * t) = Real.sin (m * x) / m := by
  have hm' : (m : ℝ) ≠ 0 := by exact_mod_cast hm.ne'
  rw [intervalIntegral.integral_comp_mul_left (fun t => Real.cos t) hm', integral_cos]
  simp [smul_eq_mul]; field_simp

theorem int_sin (m : ℕ) (hm : 0 < m) (x : ℝ) :
    ∫ t in (0 : ℝ)..x, Real.sin (m * t) = (1 - Real.cos (m * x)) / m := by
  have hm' : (m : ℝ) ≠ 0 := by exact_mod_cast hm.ne'
  rw [intervalIntegral.integral_comp_mul_left (fun t => Real.sin t) hm', integral_sin]
  simp [smul_eq_mul]; field_simp

/-- What `g` adds up (mass.rs:2069–2073) for coefficients `c0`, `c m`, `s m`: the integral of the
fitted trigonometric polynomial from 0 to x. -/
theorem G_fourier (c0' : ℝ) (c s : ℕ → ℝ) (x : ℝ) :
    ∫ t in (0 : ℝ)..x, (c0' + ∑ m ∈ Finset.Icc 1 k, (c m * Real.cos (m * t) + s m * Real.sin (m * t))) =
      c0' * x + ∑ m ∈ Finset.Icc 1 k,
        (Real.sin (m * x) / m * c m + (1 - Real.cos (m * x)) / m * s m) := by
  have hc : ∀ m : ℕ, Continuous fun t : ℝ => c m * Real.cos (m * t) + s m * Real.sin (m * t) := by
    intro m; fun_prop
  have hs : Continuous fun t : ℝ =>
      ∑ m ∈ Finset.Icc 1 k, (c m * Real.cos (m * t) + s m * Real.sin (m * t)) := by fun_prop
  rw [intervalIntegral.integral_add (f := fun _ => c0')
      (g := fun t => ∑ m ∈ Finset.Icc 1 k, (c m * Real.cos (m * t) + s m * Real.sin (m * t)))
      intervalIntegrable_const (hs.intervalIntegrable _ _),
    intervalIntegral.integral_const, smul_eq_mul, sub_zero,
    intervalIntegral.integral_finsetSum (fun m _ => (hc m).intervalIntegrable _ _)]
  congr 1
  · ring
  apply Finset.sum_congr rfl
  intro m hm
  have hm0 : 0 < m := (Finset.mem_Icc.1 hm).1
  rw [intervalIntegral.integral_add (f := fun t => c m * Real.cos (m * t))
      (g := fun t => s m * Real.sin (m * t))
      ((continuous_const.mul (by fun_prop)).intervalIntegrable _ _)
      ((continuous_const.mul (by fun_prop)).intervalIntegrable _ _),
    intervalIntegral.integral_const_mul, intervalIntegral.integral_const_mul, int_cos m hm0,
    int_sin m hm0]
  ring

/-! ## What `Inner::Fourier` computes, end to end -/

/-- The fit's coefficients from real samples F(a0 + θⱼ), in real arithmetic (mass.rs:1821–1834). -/
def rc0 (f : ℝ → ℝ) (a0 : ℝ) : ℝ := 1 / N k * ∑ j ∈ Finset.range (N k), f (a0 + θ k j)
def rcm (f : ℝ → ℝ) (a0 : ℝ) (m : ℕ) : ℝ :=
  2 / N k * ∑ j ∈ Finset.range (N k), f (a0 + θ k j) * Real.cos (m * θ k j)
def rsm (f : ℝ → ℝ) (a0 : ℝ) (m : ℕ) : ℝ :=
  2 / N k * ∑ j ∈ Finset.range (N k), f (a0 + θ k j) * Real.sin (m * θ k j)

/-- For F a trigonometric polynomial of degree ≤ k along a, `G(a0 + x)` as `Inner::Fourier` adds
it up from the fitted coefficients (mass.rs:2066–2073) is exactly ∫_{a0}^{a0+x} F. -/
theorem fourier_G {f : ℝ → ℝ} (hf : TrigR k f) (hc : Continuous f) (a0 x : ℝ) :
    ∫ t in a0..a0 + x, f t = rc0 k f a0 * x + ∑ m ∈ Finset.Icc 1 k,
      (Real.sin (m * x) / m * rcm k f a0 m + (1 - Real.cos (m * x)) / m * rsm k f a0 m) := by
  have hg := trig_shift k hf a0
  have pt : ∀ t, f (a0 + t) = rc0 k f a0 + ∑ m ∈ Finset.Icc 1 k,
      (rcm k f a0 m * Real.cos (m * t) + rsm k f a0 m * Real.sin (m * t)) := by
    intro t
    have := dft_exact k hg t
    simp only [recon, c0, cm, sm] at this
    apply Complex.ofReal_injective
    rw [← this]
    simp only [rc0, rcm, rsm]
    push_cast
    rfl
  have shift : ∫ t in a0..a0 + x, f t = ∫ t in (0 : ℝ)..x, f (a0 + t) := by
    rw [intervalIntegral.integral_comp_add_left (fun t => f t) a0]; simp
  rw [shift, ← G_fourier k (rc0 k f a0) (rcm k f a0) (rsm k f a0) x]
  congr 1; funext t; exact pt t

end ForgeMass
end
