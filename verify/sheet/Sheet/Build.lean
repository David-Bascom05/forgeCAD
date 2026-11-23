import Sheet.Tree
import Sheet.Rules

/-!
# L3: the kernel's joint decisions, correct for every input of a stated domain

build.rs (l.1082-1206) decides, per joint, before it lays anything out:
* `BendAngle` unless `ANG < θ < top − ANG`, `θ = |angle|`, `top = π` (a bend) or `2π` (a hem);
* `Allowance` unless `BA > 0`;
* `Physical` unless `R θ ≤ BA ≤ (R + t) θ` (κ ∈ [0, 1]; the fix-sheet session adds it);
* `MoldLine` unless `la > TOL` and `|la − lb| ≤ TOL` (the two walls' mold lines one segment).

The spec (L1) has a valid fold of a joint exactly when (`Valid`): `0 < θ < top`, `BA > 0`,
`R ≤ BA/θ ≤ R + t`, `t > 0`, `R ≥ 0`, and the mold lines are one segment (`la = lb > 0`). For an
ordinary bend `θ < π` is forced: the setback `ρ sin θ / (1 + cos θ)` is positive exactly for
`θ ∈ (0, π)` (`setback_pos_iff`), and at `θ = π` it is undefined, so no virtual sharp exists.

**The stated domain** (`InDomain`): inputs off the kernel's tolerance bands, where a valid joint
would be refused only for being within a tolerance of a degenerate one: `θ ∉ (0, ANG]`,
`θ ∉ [top − ANG, top)`, `la ∉ (0, TOL]`, `|la − lb| ∉ (0, TOL]`. On it:

* `decide_ok_iff`: the kernel accepts a joint iff the spec has a valid fold of it (refusal iff
  no valid result);
* `decide_bend`: an accepted ordinary joint is a `Bend` with every hypothesis Fold.lean needs, so
  `Tree.refold` holds for every tree of accepted joints (`build_refold`).

The bands themselves are the kernel's refusals of geometrically valid input, listed, not hidden.
-/

namespace Sheet.Build

open Real

structure JointIn where
  R : ℝ
  angle : ℝ
  t : ℝ
  BA : ℝ
  la : ℝ
  lb : ℝ
  hem : Bool

inductive Err
  | bendAngle
  | allowance
  | physical
  | moldLine
  | gauge
  deriving DecidableEq, Repr

noncomputable def top (j : JointIn) : ℝ := if j.hem then 2 * π else π

/-- The kernel's decisions, in its order. -/
noncomputable def decide (ANG TOL : ℝ) (j : JointIn) : Except Err Unit := by
  classical
  exact
    if ¬ (0 < j.t ∧ 0 ≤ j.R) then .error .gauge
    else if ¬ (ANG < |j.angle| ∧ |j.angle| < top j - ANG) then .error .bendAngle
    else if ¬ (0 < j.BA) then .error .allowance
    else if ¬ (j.R * |j.angle| ≤ j.BA ∧ j.BA ≤ (j.R + j.t) * |j.angle|) then .error .physical
    else if ¬ (TOL < j.la ∧ |j.la - j.lb| ≤ TOL) then .error .moldLine
    else .ok ()

/-- The spec: the joint has a valid fold. -/
def Valid (j : JointIn) : Prop :=
  0 < j.t ∧ 0 ≤ j.R ∧ 0 < |j.angle| ∧ |j.angle| < top j ∧ 0 < j.BA ∧
    j.R ≤ j.BA / |j.angle| ∧ j.BA / |j.angle| ≤ j.R + j.t ∧ 0 < j.la ∧ j.la = j.lb

/-- Off the tolerance bands. -/
def InDomain (ANG TOL : ℝ) (j : JointIn) : Prop :=
  ¬ (0 < |j.angle| ∧ |j.angle| ≤ ANG) ∧ ¬ (top j - ANG ≤ |j.angle| ∧ |j.angle| < top j) ∧
    ¬ (0 < j.la ∧ j.la ≤ TOL) ∧ ¬ (0 < |j.la - j.lb| ∧ |j.la - j.lb| ≤ TOL)

theorem phys_iff {R θ t BA : ℝ} (hθ : 0 < θ) :
    (R * θ ≤ BA ∧ BA ≤ (R + t) * θ) ↔ (R ≤ BA / θ ∧ BA / θ ≤ R + t) := by
  rw [le_div_iff₀ hθ, div_le_iff₀ hθ]

/-- **L3, refusal iff no valid result**: on the domain (with `0 ≤ ANG`, `0 ≤ TOL`), the kernel
accepts the joint exactly when the spec has a valid fold of it. -/
theorem decide_ok_iff (ANG TOL : ℝ) (hA : 0 ≤ ANG) (hT : 0 ≤ TOL) (j : JointIn)
    (hD : InDomain ANG TOL j) : decide ANG TOL j = .ok () ↔ Valid j := by
  obtain ⟨d1, d2, d3, d4⟩ := hD
  unfold decide Valid
  constructor
  · intro h
    split_ifs at h with g a b p m
    obtain ⟨g1, g2⟩ := g
    obtain ⟨a1, a2⟩ := a
    have hθ : 0 < |j.angle| := lt_of_le_of_lt hA a1
    obtain ⟨p1, p2⟩ := (phys_iff hθ).mp p
    obtain ⟨m1, m2⟩ := m
    have hla : 0 < j.la := lt_of_le_of_lt hT m1
    have heq : j.la = j.lb := by
      by_contra hne
      have : 0 < |j.la - j.lb| := abs_pos.mpr (sub_ne_zero.mpr hne)
      exact d4 ⟨this, m2⟩
    exact ⟨g1, g2, hθ, by linarith, b, p1, p2, hla, heq⟩
  · rintro ⟨t0, r0, θ0, θ1, ba, k0, k1, la0, leq⟩
    have g : 0 < j.t ∧ 0 ≤ j.R := ⟨t0, r0⟩
    have a : ANG < |j.angle| ∧ |j.angle| < top j - ANG := by
      constructor
      · by_contra h; exact d1 ⟨θ0, not_lt.mp h⟩
      · by_contra h; exact d2 ⟨not_lt.mp h, θ1⟩
    have p := (phys_iff θ0).mpr ⟨k0, k1⟩
    have m : TOL < j.la ∧ |j.la - j.lb| ≤ TOL := by
      constructor
      · by_contra h; exact d3 ⟨la0, not_lt.mp h⟩
      · rw [leq, sub_self, abs_zero]; exact hT
    simp [g, a, ba, p, m]

/-- For an ordinary bend the setback is positive exactly on `(0, π)`: past `π` the mold lines
would lie behind the tangent lines, at `π` there is none. -/
theorem setback_pos_iff (ρ θ : ℝ) (hρ : 0 < ρ) (h0 : 0 < θ) (h2 : θ < 2 * π) (hπ : θ ≠ π) :
    0 < ρ * sin θ / (1 + cos θ) ↔ θ < π := by
  have hc : 0 < 1 + cos θ := by
    have hne : cos θ ≠ -1 := by
      intro h
      obtain ⟨k, hk⟩ := Real.cos_eq_neg_one_iff.mp h
      have : (k : ℝ) * (2 * π) = θ - π := by linarith
      have hk1 : (-1 : ℝ) < k := by
        by_contra hk; push Not at hk
        have := mul_le_mul_of_nonneg_right hk (by positivity : (0 : ℝ) ≤ 2 * π)
        linarith [Real.pi_pos]
      have hk2 : (k : ℝ) < 1 := by
        by_contra hk; push Not at hk
        have := mul_le_mul_of_nonneg_right hk (by positivity : (0 : ℝ) ≤ 2 * π)
        linarith [Real.pi_pos]
      have : k = 0 := by
        have a : (-1 : ℤ) < k := by exact_mod_cast hk1
        have b : k < (1 : ℤ) := by exact_mod_cast hk2
        omega
      subst this; simp at *; exact hπ (by linarith)
    have hle := Real.neg_one_le_cos θ
    rcases hle.lt_or_eq with h | h
    · linarith
    · exact absurd h.symm hne
  rw [div_pos_iff_of_pos_right hc]
  constructor
  · intro h
    by_contra hge; push Not at hge
    have hgt : π < θ := lt_of_le_of_ne hge (Ne.symm hπ)
    have : 0 < sin (θ - π) := Real.sin_pos_of_pos_of_lt_pi (by linarith) (by linarith)
    rw [Real.sin_sub_pi] at this
    nlinarith
  · intro h; exact mul_pos hρ (Real.sin_pos_of_pos_of_lt_pi h0 h)

/-- An accepted ordinary joint is a `Bend`, with every hypothesis of Fold.lean. -/
noncomputable def toBend (j : JointIn) (hj : Valid j) (hb : j.hem = false) : Bend where
  R := j.R
  θ := |j.angle|
  t := j.t
  BA := j.BA
  sense := if 0 < j.angle then .pos else .neg
  hR := hj.2.1
  hθ0 := hj.2.2.1
  hθπ := by have := hj.2.2.2.1; simpa [top, hb] using this
  ht := hj.1
  hBA := hj.2.2.2.2.1

theorem toBend_physical (j : JointIn) (hj : Valid j) (hb : j.hem = false) :
    0 ≤ (toBend j hj hb).κ ∧ (toBend j hj hb).κ ≤ 1 :=
  ((toBend j hj hb).rn_mem_iff).mp ⟨hj.2.2.2.2.2.1, hj.2.2.2.2.2.2.1⟩

/-- **An accepted ordinary joint is a bend of the spec**: on the domain, the kernel's acceptance
gives a `Bend` with the joint's numbers and every hypothesis of Fold.lean, its neutral layer in
the material. -/
theorem accepted_bend (ANG TOL : ℝ) (hA : 0 ≤ ANG) (hT : 0 ≤ TOL) (j : JointIn)
    (hD : InDomain ANG TOL j) (hok : decide ANG TOL j = .ok ()) (hb : j.hem = false) :
    ∃ b : Bend, b.R = j.R ∧ b.θ = |j.angle| ∧ b.t = j.t ∧ b.BA = j.BA ∧ 0 ≤ b.κ ∧ b.κ ≤ 1 :=
  let hv := (decide_ok_iff ANG TOL hA hT j hD).mp hok
  ⟨toBend j hv hb, rfl, rfl, rfl, rfl, toBend_physical j hv hb⟩

/-- **L3, develop and fold**: for every tree of bends (each one an accepted joint, by
`accepted_bend`), the kernel's development and fold are the folded part on every wall, and the
zones glue on every tangent line (`Tree.refold`, by induction on the tree). -/
theorem build_refold (T : Tree) (X : Folded T) (P0 : V2 ≃ V2) :
    (∀ path ∈ T.walls, ∀ q : V2, ∀ w : ℝ,
      Fwall (X.M []) P0 path (develop P0 path q) w = X.M path (q.1, q.2, -w)) ∧
    (∀ path e, path ++ [e] ∈ T.walls → ∀ z w,
      ((jointCoords P0 path e z).1 = -e.bend.s →
        Fzone (X.M []) P0 path e z w = Fwall (X.M []) P0 path z w) ∧
      ((jointCoords P0 path e z).1 = e.bend.BA - e.bend.s →
        Fzone (X.M []) P0 path e z w = Fwall (X.M []) P0 (path ++ [e]) z w)) :=
  Sheet.refold T X P0

end Sheet.Build
