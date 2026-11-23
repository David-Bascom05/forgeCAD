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

/-- Off the tools' closed set, the boolean is pointwise the target if `op` keeps it alone, else empty. -/
theorem combine_off {ι : Type v} [DecidableEq ι] (op : (ι → Prop) → Prop) (A : ι → Pred X) (i₀ : ι)
    (B : Pred X) (htools : ∀ i, i ≠ i₀ → ∀ x, A i x → B x) (hempty : ¬op fun _ => False) :
    ∀ x, ¬B x → (combine op A x ↔ (op (fun i => i = i₀) ∧ A i₀ x)) := by
  intro x hx
  have hoff : ∀ i, i ≠ i₀ → ¬A i x := fun i hi hA => hx (htools i hi x hA)
  unfold combine
  rw [pattern_off hoff]
  by_cases h0 : A i₀ x
  · have : (fun i => i = i₀ ∧ A i₀ x) = fun i => i = i₀ := by funext i; simp [h0]
    rw [this]; simp [h0]
  · have : (fun i => i = i₀ ∧ A i₀ x) = fun _ => False := by funext i; simp [h0]
    rw [this]; simp [h0, hempty]

/-- **(F3) The stitch, reduced to a statement about the written body.** Suppose:
* the core's kept cells agree with the boolean in the region, off a nowhere-dense wall `W` (as the
  decomposition of `reg_in` does);
* the written body `Out` agrees, off a nowhere-dense set `Z` (the carriers and walls), with the
  core's kept cells inside the region and with the target (or nothing) outside.

Then the written body regularises to the regularised boolean. What the bridge's write must
therefore establish, on its Aeneas translation, is `hout`: the stitched body's membership, off the
carrier planes, is the core's result in `R°` and the input target outside. -/
theorem stitch_reg {ι : Type v} [DecidableEq ι] (op : (ι → Prop) → Prop) (A : ι → Pred X) (i₀ : ι)
    (B U : Pred X) (hBU : B.Sub U) (htools : ∀ i, i ≠ i₀ → ∀ x, A i x → B x)
    (hempty : ¬op fun _ => False)
    (kept W : Pred X) (hW : NowhereDense W) (hkept : ∀ x, U x → ¬W x → (kept x ↔ combine op A x))
    (Out Z : Pred X) (hZ : NowhereDense Z)
    (hout : ∀ x, ¬Z x → (Out x ↔ (U x ∧ kept x) ∨ (¬U x ∧ op (fun i => i = i₀) ∧ A i₀ x))) :
    reg Out = reg (combine op A) := by
  let T : Pred X := fun x => (U x ∧ kept x) ∨ (¬U x ∧ op (fun i => i = i₀) ∧ A i₀ x)
  have h1 : reg Out = reg T := reg_eq_of_sdiff hZ fun x hx => by
    apply Classical.byContradiction; intro hz
    have := hout x hz
    rcases hx with ⟨a, b⟩ | ⟨a, b⟩
    · exact b (this.mp a)
    · exact b (this.mpr a)
  have h2 : reg T = reg (combine op A) := reg_eq_of_sdiff hW fun x hx => by
    apply Classical.byContradiction; intro hw
    by_cases hu : U x
    · have e : T x ↔ kept x := by simp [T, hu]
      rcases hx with ⟨a, b⟩ | ⟨a, b⟩
      · exact b ((hkept x hu hw).mp (e.mp a))
      · exact b (e.mpr ((hkept x hu hw).mpr a))
    · have hb : ¬B x := fun h => hu (hBU x h)
      have e : T x ↔ op (fun i => i = i₀) ∧ A i₀ x := by simp [T, hu]
      have e2 := combine_off op A i₀ B htools hempty x hb
      rcases hx with ⟨a, b⟩ | ⟨a, b⟩
      · exact b (e2.mpr (e.mp a))
      · exact b (e.mpr (e2.mp a))
  rw [h1, h2]

end ForgeExact.DecideOnce.Frontier
