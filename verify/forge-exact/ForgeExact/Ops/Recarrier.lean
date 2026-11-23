import ForgeExact.Ops.Pattern

/-!
# Re-carrying faces: R1, the result does not depend on the formula read

research/ops-spec/design.md §0.1. Draft, a shell's inner skin and the faces a chamfer trims keep
the body's face lattice and give faces new carriers. The body is a pointwise boolean of its faces'
half-spaces, `S = reg (combine op A)`, but many formulas `op` give the same `S`. The re-carried
body substitutes the new half-spaces `B i` for the old `A i` in a formula. R1 says when the
answer cannot depend on the formula chosen.

`Valid A B` is that one homeomorphism `φ` carries every old half-space onto its new one,
`img φ (A i) = B i`: the faces move, the lattice (every incidence and every side) stays. Then:

* `combine_img`: every pointwise boolean of the new half-spaces is the image of the same boolean of
  the old: `combine op B = img φ (combine op A)`;
* `recarrier_img`: so the re-carried solid is the image of the old one, `img φ S` (P1, `reg_img`);
* **R1** `recarrier_indep`: two formulas that give the same body give the same re-carried body.

Without such a `φ` (a face swept past a vertex, an edge turned round, a new contact) two formulas
of one body can give two re-carried bodies, and the spec refuses (`Event`): deciding `Valid` from
the lattice and the new carriers is the L3 part.
-/

namespace ForgeExact.Ops

open ForgeExact Topology

universe u v

variable {X : Type u} [Topology X]

/-- The new half-spaces are the old ones moved by one homeomorphism. -/
def Valid {ι : Type v} (A B : ι → Pred X) (φ : Homeo X) : Prop := ∀ i, img φ (A i) = B i

/-- A pointwise boolean of the moved half-spaces is the moved boolean. -/
theorem combine_img {ι : Type v} {A B : ι → Pred X} {φ : Homeo X} (h : Valid A B φ)
    (op : (ι → Prop) → Prop) : combine op B = img φ (combine op A) := by
  apply Pred.ext; intro y
  have : (fun i => B i y) = (fun i => A i (φ.inv y)) := by
    funext i; rw [← h i]; rfl
  simp only [combine, img, this]

/-- The re-carried body is the image of the body. -/
theorem recarrier_img {ι : Type v} {A B : ι → Pred X} {φ : Homeo X} (h : Valid A B φ)
    (op : (ι → Prop) → Prop) : reg (combine op B) = img φ (reg (combine op A)) := by
  rw [combine_img h, reg_img]

/-- **R1.** Two formulas of one body re-carry to one body. -/
theorem recarrier_indep {ι : Type v} {A B : ι → Pred X} {φ : Homeo X} (h : Valid A B φ)
    {op₁ op₂ : (ι → Prop) → Prop} (same : reg (combine op₁ A) = reg (combine op₂ A)) :
    reg (combine op₁ B) = reg (combine op₂ B) := by
  rw [recarrier_img h, recarrier_img h, same]

/-- And a solid re-carries to a solid. -/
theorem recarrier_solid {ι : Type v} {A B : ι → Pred X} {φ : Homeo X} (h : Valid A B φ)
    (op : (ι → Prop) → Prop) : Solid (reg (combine op B)) := by
  rw [recarrier_img h]
  exact solid_img φ (reg_reg _)

end ForgeExact.Ops
