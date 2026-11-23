import Mathlib

/-!
# Trigonometric polynomials and polynomials of bounded degree

`Trig k` is the span of `e^{iqx}`, |q| ≤ k: the complex trigonometric polynomials of degree at
most k. A real function is in `TrigR k` when its complex values are. `PolyR d` is the real
polynomials of degree at most d. Both are closed under sums and scaling, and multiplying adds
the degrees. These are the facts the degree claims in mass.rs:2416–2436 rest on.
-/

noncomputable section
open Complex Pointwise

namespace ForgeMass

def ex (q : ℤ) : ℝ → ℂ := fun x => Complex.exp (q * x * I)

def gens (k : ℕ) : Set (ℝ → ℂ) := {f | ∃ q : ℤ, |q| ≤ k ∧ f = ex q}

def Trig (k : ℕ) : Submodule ℂ (ℝ → ℂ) := Submodule.span ℂ (gens k)

theorem ex_mul (p q : ℤ) : ex p * ex q = ex (p + q) := by
  funext x
  simp only [ex, Pi.mul_apply, ← Complex.exp_add]
  congr 1
  push_cast
  ring

theorem ex_mem {k : ℕ} {q : ℤ} (h : |q| ≤ k) : ex q ∈ Trig k :=
  Submodule.subset_span ⟨q, h, rfl⟩

theorem gens_mul (k l : ℕ) : gens k * gens l ⊆ gens (k + l) := by
  rintro f ⟨g, ⟨p, hp, rfl⟩, h, ⟨q, hq, rfl⟩, rfl⟩
  refine ⟨p + q, ?_, ?_⟩
  swap
  · exact (ex_mul p q).symm ▸ rfl
  calc |p + q| ≤ |p| + |q| := abs_add_le p q
    _ ≤ (k : ℤ) + l := add_le_add hp hq
    _ = ((k + l : ℕ) : ℤ) := by push_cast; ring

theorem trig_mul {k l : ℕ} {f g : ℝ → ℂ} (hf : f ∈ Trig k) (hg : g ∈ Trig l) :
    f * g ∈ Trig (k + l) := by
  have : f * g ∈ Trig k * Trig l := Submodule.mul_mem_mul hf hg
  unfold Trig at this ⊢
  rw [Submodule.span_mul_span] at this
  exact Submodule.span_mono (gens_mul k l) this

theorem trig_mono {k l : ℕ} (h : k ≤ l) : Trig k ≤ Trig l := by
  apply Submodule.span_mono
  rintro f ⟨q, hq, rfl⟩
  exact ⟨q, le_trans hq (by exact_mod_cast h), rfl⟩

theorem const_mem (k : ℕ) (c : ℂ) : (fun _ => c) ∈ Trig k := by
  have h0 : ex 0 ∈ Trig k := ex_mem (by simp)
  have : (fun _ : ℝ => c) = c • ex 0 := by funext x; simp [ex]
  rw [this]
  exact Submodule.smul_mem _ c h0

theorem cos_mem (m : ℕ) : (fun x : ℝ => ((Real.cos (m * x) : ℝ) : ℂ)) ∈ Trig m := by
  have : (fun x : ℝ => ((Real.cos (m * x) : ℝ) : ℂ)) = (1 / 2 : ℂ) • (ex m + ex (-m)) := by
    funext x
    simp only [ex, Pi.smul_apply, Pi.add_apply, smul_eq_mul, Complex.ofReal_cos]
    rw [Complex.cos]
    push_cast
    ring_nf
  rw [this]
  exact Submodule.smul_mem _ _ (Submodule.add_mem _ (ex_mem (by simp)) (ex_mem (by simp)))

theorem sin_mem (m : ℕ) : (fun x : ℝ => ((Real.sin (m * x) : ℝ) : ℂ)) ∈ Trig m := by
  have : (fun x : ℝ => ((Real.sin (m * x) : ℝ) : ℂ)) = (-I / 2 : ℂ) • (ex m - ex (-m)) := by
    funext x
    simp only [ex, Pi.smul_apply, Pi.sub_apply, smul_eq_mul, Complex.ofReal_sin]
    rw [Complex.sin]
    push_cast
    ring_nf
  rw [this]
  exact Submodule.smul_mem _ _ (Submodule.sub_mem _ (ex_mem (by simp)) (ex_mem (by simp)))

/-! ## Real functions -/

def TrigR (k : ℕ) (f : ℝ → ℝ) : Prop := (fun x => (f x : ℂ)) ∈ Trig k

namespace TrigR

theorem add {k : ℕ} {f g : ℝ → ℝ} (hf : TrigR k f) (hg : TrigR k g) : TrigR k (fun x => f x + g x) := by
  unfold TrigR at *
  have := Submodule.add_mem _ hf hg
  convert this using 1; funext x; simp

theorem mul {k l : ℕ} {f g : ℝ → ℝ} (hf : TrigR k f) (hg : TrigR l g) :
    TrigR (k + l) (fun x => f x * g x) := by
  unfold TrigR at *
  have := trig_mul hf hg
  convert this using 1; funext x; simp

theorem smul {k : ℕ} (c : ℝ) {f : ℝ → ℝ} (hf : TrigR k f) : TrigR k (fun x => c * f x) := by
  unfold TrigR at *
  have := Submodule.smul_mem _ (c : ℂ) hf
  convert this using 1; funext x; simp

theorem mono {k l : ℕ} (h : k ≤ l) {f : ℝ → ℝ} (hf : TrigR k f) : TrigR l f := trig_mono h hf

theorem const (k : ℕ) (c : ℝ) : TrigR k (fun _ => c) := const_mem k c

theorem cos : TrigR 1 Real.cos := by
  have := cos_mem 1
  simpa [TrigR] using this

theorem sin : TrigR 1 Real.sin := by
  have := sin_mem 1
  simpa [TrigR] using this

theorem sum {k : ℕ} {ι : Type*} (s : Finset ι) {f : ι → ℝ → ℝ} (h : ∀ i ∈ s, TrigR k (f i)) :
    TrigR k (fun x => ∑ i ∈ s, f i x) := by
  unfold TrigR at *
  have := Submodule.sum_mem _ (fun i hi => h i hi)
  convert this using 1; funext x; simp [Finset.sum_apply]

end TrigR

/-! ## Polynomials -/

def PolyR (d : ℕ) (f : ℝ → ℝ) : Prop := ∃ p : Polynomial ℝ, p.natDegree ≤ d ∧ ∀ x, f x = p.eval x

namespace PolyR

theorem add {d : ℕ} {f g : ℝ → ℝ} (hf : PolyR d f) (hg : PolyR d g) : PolyR d (fun x => f x + g x) := by
  obtain ⟨p, hp, hpf⟩ := hf
  obtain ⟨q, hq, hqg⟩ := hg
  exact ⟨p + q, (Polynomial.natDegree_add_le _ _).trans (max_le hp hq), fun x => by simp [hpf, hqg]⟩

theorem mul {d e : ℕ} {f g : ℝ → ℝ} (hf : PolyR d f) (hg : PolyR e g) :
    PolyR (d + e) (fun x => f x * g x) := by
  obtain ⟨p, hp, hpf⟩ := hf
  obtain ⟨q, hq, hqg⟩ := hg
  exact ⟨p * q, (Polynomial.natDegree_mul_le).trans (add_le_add hp hq), fun x => by simp [hpf, hqg]⟩

theorem smul {d : ℕ} (c : ℝ) {f : ℝ → ℝ} (hf : PolyR d f) : PolyR d (fun x => c * f x) := by
  obtain ⟨p, hp, hpf⟩ := hf
  exact ⟨Polynomial.C c * p, (Polynomial.natDegree_C_mul_le _ _).trans hp, fun x => by simp [hpf]⟩

theorem mono {d e : ℕ} (h : d ≤ e) {f : ℝ → ℝ} (hf : PolyR d f) : PolyR e f := by
  obtain ⟨p, hp, hpf⟩ := hf
  exact ⟨p, hp.trans h, hpf⟩

theorem const (d : ℕ) (c : ℝ) : PolyR d (fun _ => c) :=
  ⟨Polynomial.C c, by simp, fun x => by simp⟩

theorem id : PolyR 1 (fun x => x) := ⟨Polynomial.X, by simp, fun x => by simp⟩

theorem sum {d : ℕ} {ι : Type*} (s : Finset ι) {f : ι → ℝ → ℝ} (h : ∀ i ∈ s, PolyR d (f i)) :
    PolyR d (fun x => ∑ i ∈ s, f i x) := by
  classical
  induction s using Finset.induction_on with
  | empty => simpa using const d 0
  | insert a s ha ih =>
    simp only [Finset.sum_insert ha]
    exact add (h a (Finset.mem_insert_self a s)) (ih fun i hi => h i (Finset.mem_insert_of_mem hi))

end PolyR

end ForgeMass
end
