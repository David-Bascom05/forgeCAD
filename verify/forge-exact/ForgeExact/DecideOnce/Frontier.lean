import ForgeExact.Regular

/-!
# The local ledger's frontier (decide-once spec §3a): F1 and F2

The bridge builds the ledger only over a region `R` round the tools; outside it the inputs pass
through. What makes that exact:

* `reg_local`: regularisation is local. Two sets that agree on an open set have regularisations
  that agree there.
* **(F1) `far_unchanged`.** Suppose every tool (every body but the target `i₀`) lies in a closed set
  `B`, and the rule keeps nothing outside every body (`¬op ∅`). Then at every point off `B`, the
  regularised boolean is the target if the rule keeps the target alone (`op e₀`), and empty if not.
  `B` is the tools' box, and `R° ⊇ B`, so this covers the whole of `X \ R°`.
* **(F2) `frontier_local`.** Off `B`, and so on and near `∂R`, the regularised boolean coincides
  with the target (or with `∅`) on a whole open neighbourhood. Any local property of the result
  there, manifoldness included, is the target's, and the target is a valid solid.
* `split_at_region`: the regularised boolean is its part in `R°` together with the target's part
  outside (or nothing).

Pure topology over `Topology.lean`; no geometry. No `sorry`; standard axioms only.
-/

namespace ForgeExact.DecideOnce.Frontier

open ForgeExact Topology

universe u v

variable {X : Type u} [Topology X]

/-- **Regularisation is local**: sets agreeing on an open `W` have regularisations agreeing on `W`. -/
theorem reg_local {S T W : Pred X} (hW : IsOpen W) (h : ∀ x, W x → (S x ↔ T x)) :
    ∀ x, W x → (reg S x ↔ reg T x) := by
  -- the interiors agree on `W`
  have hint : ∀ {S T : Pred X}, (∀ x, W x → (S x ↔ T x)) → ∀ x, W x → interior S x → interior T x := by
    intro S T h x hx ⟨U, hU, hUS, hxU⟩
    refine ⟨Pred.inter U W, isOpen_inter _ _ hU hW, fun y ⟨hyU, hyW⟩ => (h y hyW).mp (hUS y hyU), hxU, hx⟩
  have hint' : ∀ {S T : Pred X}, (∀ x, W x → (S x ↔ T x)) → ∀ x, W x → (interior S x ↔ interior T x) :=
    fun h x hx => ⟨hint h x hx, hint (fun y hy => (h y hy).symm) x hx⟩
  -- so do the closures of the interiors, at points of `W`
  have hcl : ∀ {S T : Pred X}, (∀ x, W x → (S x ↔ T x)) → ∀ x, W x → reg S x → reg T x := by
    intro S T h x hx hS V hV hxV
    obtain ⟨y, ⟨hyV, hyW⟩, hy⟩ := hS (Pred.inter V W) (isOpen_inter _ _ hV hW) ⟨hxV, hx⟩
    exact ⟨y, hyV, (hint' h y hyW).mp hy⟩
  intro x hx
  exact ⟨hcl h x hx, hcl (fun y hy => (h y hy).symm) x hx⟩

/-- The membership pattern of a point outside every tool: the target's alone, or nothing. -/
theorem pattern_off {ι : Type v} [DecidableEq ι] {A : ι → Pred X} {i₀ : ι} {x : X}
    (hoff : ∀ i, i ≠ i₀ → ¬A i x) :
    (fun i => A i x) = fun i => i = i₀ ∧ A i₀ x := by
  funext i
  apply propext
  by_cases hi : i = i₀
  · subst hi; simp
  · simp [hi, hoff i hi]

/-- **(F1) Far unchanged.** Every tool inside the closed `B`, and `¬op ∅`: off `B`, the regularised
boolean is the target if `op` keeps the target alone, else empty. The target need only be
regular (`reg (A i₀) = A i₀`, true of a valid solid). -/
theorem far_unchanged {ι : Type v} [DecidableEq ι] (op : (ι → Prop) → Prop) (A : ι → Pred X) (i₀ : ι)
    (B : Pred X) (hB : IsOpen fun x => ¬B x) (htools : ∀ i, i ≠ i₀ → ∀ x, A i x → B x)
    (hempty : ¬op fun _ => False) (hreg : reg (A i₀) = A i₀) :
    ∀ x, ¬B x → (reg (combine op A) x ↔ op (fun i => i = i₀) ∧ A i₀ x) := by
  -- off `B`, the boolean is `op e₀ ∧ A i₀`
  have hpt : ∀ x, ¬B x → (combine op A x ↔ (op (fun i => i = i₀) ∧ A i₀ x)) := by
    intro x hx
    have hoff : ∀ i, i ≠ i₀ → ¬A i x := fun i hi hA => hx (htools i hi x hA)
    unfold combine
    rw [pattern_off hoff]
    by_cases h0 : A i₀ x
    · have : (fun i => i = i₀ ∧ A i₀ x) = fun i => i = i₀ := by funext i; simp [h0]
      rw [this]; simp [h0]
    · have : (fun i => i = i₀ ∧ A i₀ x) = fun _ => False := by funext i; simp [h0]
      rw [this]; simp [h0, hempty]
  intro x hx
  rw [reg_local hB hpt x hx]
  by_cases hk : op (fun i => i = i₀)
  · have : (fun y => op (fun i => i = i₀) ∧ A i₀ y) = A i₀ := by funext y; simp [hk]
    rw [this, hreg]; simp [hk]
  · have : (fun y => op (fun i => i = i₀) ∧ A i₀ y) = fun _ => False := by funext y; simp [hk]
    rw [this]
    simp only [hk, false_and, iff_false]
    intro h
    obtain ⟨y, _, U, _, hUS, hyU⟩ := h (fun _ => True) isOpen_univ trivial
    exact hUS y hyU

/-- **(F2) The frontier is the target's.** Off `B` the regularised boolean coincides with the target
(or with nothing) on a whole open neighbourhood, `X \ B`: so every local property of the result there
(manifoldness, the faces and edges through a frontier cell) is the target's. -/
theorem frontier_local {ι : Type v} [DecidableEq ι] (op : (ι → Prop) → Prop) (A : ι → Pred X) (i₀ : ι)
    (B : Pred X) (hB : IsOpen fun x => ¬B x) (htools : ∀ i, i ≠ i₀ → ∀ x, A i x → B x)
    (hempty : ¬op fun _ => False) (hreg : reg (A i₀) = A i₀) (x : X) (hx : ¬B x) :
    ∃ N : Pred X, IsOpen N ∧ N x ∧ ∀ y, N y →
      (reg (combine op A) y ↔ op (fun i => i = i₀) ∧ A i₀ y) :=
  ⟨fun y => ¬B y, hB, hx, fun y hy => far_unchanged op A i₀ B hB htools hempty hreg y hy⟩

/-- **Global.** With `B` inside the open region `Rᵒ`, the regularised boolean is its own part in `Rᵒ`
together with the target's part outside (or nothing): the core decides the first, the bridge
copies the second. -/
theorem split_at_region {ι : Type v} [DecidableEq ι] (op : (ι → Prop) → Prop) (A : ι → Pred X) (i₀ : ι)
    (B Ro : Pred X) (hB : IsOpen fun x => ¬B x) (hBR : B.Sub Ro)
    (htools : ∀ i, i ≠ i₀ → ∀ x, A i x → B x)
    (hempty : ¬op fun _ => False) (hreg : reg (A i₀) = A i₀) :
    reg (combine op A) = fun x => (Ro x ∧ reg (combine op A) x) ∨ (¬Ro x ∧ op (fun i => i = i₀) ∧ A i₀ x) := by
  apply Pred.ext
  intro x
  by_cases hx : Ro x
  · simp [hx]
  · have hxB : ¬B x := fun h => hx (hBR x h)
    rw [far_unchanged op A i₀ B hB htools hempty hreg x hxB]
    simp [hx]

end ForgeExact.DecideOnce.Frontier
