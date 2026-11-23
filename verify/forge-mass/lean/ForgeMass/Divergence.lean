import Mathlib.Analysis.Calculus.Deriv.Pow
import Mathlib.Analysis.Calculus.Deriv.Mul

/-!
# The integrands' divergences (mass.rs:224–239)

`integrand(x, n)` gives each moment as a flux `Φ · n`: component k is the dot product of the
vector field `Φₖ(a, b, c)` with the face's normal `n = Su × Sv`. By the divergence theorem, the
flux of `Φₖ` through a closed shell is the integral over the solid of `div Φₖ`. Here `div Φₖ` is
the density of each moment: 1 (volume), a, b, c (first moments), a², b², c², ab, bc, ca (second
moments). Area is the integral of `|n|`, a surface integral already.

`div F (a, b, c) = ∂F₁/∂a + ∂F₂/∂b + ∂F₃/∂c`, each partial a one-variable `deriv`.
-/

noncomputable section
namespace ForgeMass

/-- A vector field on ℝ³ by its components. -/
structure Field where
  f1 : ℝ → ℝ → ℝ → ℝ
  f2 : ℝ → ℝ → ℝ → ℝ
  f3 : ℝ → ℝ → ℝ → ℝ

noncomputable def Field.div (F : Field) (a b c : ℝ) : ℝ :=
  deriv (fun t => F.f1 t b c) a + deriv (fun t => F.f2 a t c) b + deriv (fun t => F.f3 a b t) c

/-- mass.rs:227–237: the fields whose flux `integrand` takes, in its order. -/
def phi : Fin 10 → Field
  | 0 => ⟨fun a _ _ => a / 3, fun _ b _ => b / 3, fun _ _ c => c / 3⟩
  | 1 => ⟨fun a _ _ => a * a / 2, fun _ _ _ => 0, fun _ _ _ => 0⟩
  | 2 => ⟨fun _ _ _ => 0, fun _ b _ => b * b / 2, fun _ _ _ => 0⟩
  | 3 => ⟨fun _ _ _ => 0, fun _ _ _ => 0, fun _ _ c => c * c / 2⟩
  | 4 => ⟨fun a _ _ => a * a * a / 3, fun _ _ _ => 0, fun _ _ _ => 0⟩
  | 5 => ⟨fun _ _ _ => 0, fun _ b _ => b * b * b / 3, fun _ _ _ => 0⟩
  | 6 => ⟨fun _ _ _ => 0, fun _ _ _ => 0, fun _ _ c => c * c * c / 3⟩
  | 7 => ⟨fun a b _ => a * a * b / 2, fun _ _ _ => 0, fun _ _ _ => 0⟩
  | 8 => ⟨fun _ _ _ => 0, fun _ b c => b * b * c / 2, fun _ _ _ => 0⟩
  | 9 => ⟨fun _ _ _ => 0, fun _ _ _ => 0, fun a _ c => c * c * a / 2⟩

/-- The moment each field's flux measures: its density. -/
def density : Fin 10 → ℝ → ℝ → ℝ → ℝ
  | 0 => fun _ _ _ => 1
  | 1 => fun a _ _ => a
  | 2 => fun _ b _ => b
  | 3 => fun _ _ c => c
  | 4 => fun a _ _ => a ^ 2
  | 5 => fun _ b _ => b ^ 2
  | 6 => fun _ _ c => c ^ 2
  | 7 => fun a b _ => a * b
  | 8 => fun _ b c => b * c
  | 9 => fun a _ c => c * a

theorem div_phi (k : Fin 10) (a b c : ℝ) : (phi k).div a b c = density k a b c := by
  fin_cases k <;> simp [phi, density, Field.div, deriv_div_const, deriv_id''] <;> ring

end ForgeMass
end
