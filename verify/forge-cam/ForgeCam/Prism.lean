/-
The 2.5D closed-form checker (kernel/forge-cam-geom/src/slab.rs) is sound: its verdicts imply
G1, G2 and G3 for faces that are prisms along z and moves that are planar.

A planar move keeps the tip at one height z_t while its xy point γ(s) runs along a curve (a
segment or an arc; any curve here). A prism face is {(σ, h) : σ ∈ S, h ∈ H}: a horizontal face
is S = its region, H = {z_f}; a wall is S = its section curve, H = [z0, z1].

The checker's `Clear { lower }` for one face rests on two facts it computes:
(i) 0 ≤ D ≤ |σ − γ(s)| for every instant and every section point (D is the low end of
    slab::dist's interval: the 2D closed form);
(ii) lower ≤ sdRZ(D, h − z_t) for every height h of the face (the Δ branch and bound).
Only the low end of D matters: that is the monotonicity lemma (ForgeCam/Hypograph.lean).
-/
import ForgeCam.Spec

open Metric Set

namespace ForgeCam

/-- The distance in the plane. -/
noncomputable def d2 (a b : ℝ × ℝ) : ℝ := √((a.1 - b.1) ^ 2 + (a.2 - b.2) ^ 2)

/-- The tip of a planar move at instant s. -/
def planar (γ : ℝ → ℝ × ℝ) (zt : ℝ) (s : ℝ) : E3 := mk (γ s).1 (γ s).2 zt

/-- The placement of a planar move: the translation to the tip. -/
def planarMove (γ : ℝ → ℝ × ℝ) (zt : ℝ) : ℝ → E3 → E3 := fun s x => x + planar γ zt s

/-- A face that is a prism along z. -/
def prism (S : Set (ℝ × ℝ)) (H : Set ℝ) : Set E3 := {p | (p 0, p 1) ∈ S ∧ p 2 ∈ H}

/-- What the closed form establishes for one face before it answers `Clear { lower }`. -/
def ClearFace (Z : Set ℝ) (w : ℝ → ℝ) (γ : ℝ → ℝ × ℝ) (zt : ℝ) (S : Set (ℝ × ℝ)) (H : Set ℝ)
    (D lower : ℝ) : Prop :=
  0 ≤ D ∧ (∀ s ∈ I01, ∀ σ ∈ S, D ≤ d2 σ (γ s)) ∧ ∀ h ∈ H, lower ≤ sdRZ Z w D (h - zt)

theorem ρ_sub_planar (f : E3) (γ : ℝ → ℝ × ℝ) (zt s : ℝ) :
    ρ (f - planar γ zt s) = d2 (f 0, f 1) (γ s) := by
  simp [ρ, d2, planar]

/-- **The verdict is sound for one face**: every point of the face is at signed distance at
least `lower` from the moved body, at every instant. -/
theorem clearFace_sound {Z : Set ℝ} {w : ℝ → ℝ} {γ : ℝ → ℝ × ℝ} {zt : ℝ} {S : Set (ℝ × ℝ)}
    {H : Set ℝ} {D lower : ℝ} (h : ClearFace Z w γ zt S H D lower) :
    ∀ s ∈ I01, ∀ f ∈ prism S H, lower ≤ sd (translate (planar γ zt s) (hyp Z w)) f := by
  obtain ⟨hD0, hD, hH⟩ := h
  intro s hs f ⟨hfS, hfH⟩
  rw [sd_translate]
  have hz : (f - planar γ zt s) 2 = f 2 - zt := by simp [planar]
  refine (hH (f 2) hfH).trans ?_
  rw [← hz]
  exact sdRZ_le_sd hD0 (by rw [ρ_sub_planar]; exact hD s hs _ hfS)

/-! ### The bodies are connected -/

/-- The axis point at a point's height. -/
def ax (p : E3) : E3 := mk 0 0 (p 2)

@[simp] theorem smul_add_apply (a b : ℝ) (x y : E3) (i : Fin 3) :
    (a • x + b • y) i = a * x i + b * y i := by simp

theorem seg_ax_subset {Z : Set ℝ} {w : ℝ → ℝ} {x : E3} (hx : x ∈ hyp Z w) :
    segment ℝ x (ax x) ⊆ hyp Z w := by
  rw [segment_subset_iff]
  intro a b ha hb hab
  have e2 : (a • x + b • ax x) 2 = x 2 := by
    simp [ax]; rw [← add_mul, hab, one_mul]
  refine ⟨by rw [e2]; exact hx.1, ?_⟩
  rw [e2]
  refine le_trans ?_ hx.2
  simp only [ρ, smul_add_apply, ax, mk_0, mk_1, mul_zero, add_zero]
  apply Real.sqrt_le_sqrt
  have ha1 : a ≤ 1 := by linarith
  have : a ^ 2 ≤ 1 := by nlinarith
  nlinarith [sq_nonneg (x 0), sq_nonneg (x 1)]

theorem seg_axis_subset {Z : Set ℝ} {w : ℝ → ℝ} (hZ : Z.OrdConnected)
    (hw : ∀ z ∈ Z, 0 ≤ w z) {x y : E3} (hx : x 2 ∈ Z) (hy : y 2 ∈ Z) :
    segment ℝ (ax x) (ax y) ⊆ hyp Z w := by
  rw [segment_subset_iff]
  intro a b ha hb hab
  have hz : a * x 2 + b * y 2 ∈ Z := by
    have : a * x 2 + b * y 2 ∈ segment ℝ (x 2) (y 2) := ⟨a, b, ha, hb, hab, by simp⟩
    rw [segment_eq_uIcc] at this
    exact hZ.uIcc_subset hx hy this
  have e2 : (a • ax x + b • ax y) 2 = a * x 2 + b * y 2 := by simp [ax]
  refine ⟨by rw [e2]; exact hz, ?_⟩
  rw [e2]
  have : ρ (a • ax x + b • ax y) = 0 := by simp [ρ, ax]
  rw [this]; exact hw _ hz

/-- A body whose heights form an interval and whose radius is never negative there is
connected: each point reaches the axis horizontally, and the axis is a segment. -/
theorem hyp_preconnected {Z : Set ℝ} {w : ℝ → ℝ} (hZ : Z.OrdConnected)
    (hw : ∀ z ∈ Z, 0 ≤ w z) : IsPreconnected (hyp Z w) := by
  refine isPreconnected_of_forall_pair fun x hx y hy => ?_
  refine ⟨segment ℝ x (ax x) ∪ (segment ℝ (ax x) (ax y) ∪ segment ℝ (ax y) y), ?_, ?_, ?_, ?_⟩
  · refine union_subset (seg_ax_subset hx) (union_subset (seg_axis_subset hZ hw hx.1 hy.1) ?_)
    rw [segment_symm]; exact seg_ax_subset hy
  · exact Or.inl (left_mem_segment ℝ _ _)
  · exact Or.inr (Or.inr (right_mem_segment ℝ _ _))
  · refine IsPreconnected.union (ax x) (right_mem_segment ℝ _ _)
      (Or.inl (left_mem_segment ℝ _ _)) (convex_segment _ _).isPreconnected ?_
    exact IsPreconnected.union (ax y) (right_mem_segment ℝ _ _) (left_mem_segment ℝ _ _)
      (convex_segment _ _).isPreconnected (convex_segment _ _).isPreconnected

theorem translate_preconnected {X : Set E3} (hX : IsPreconnected X) (t : E3) :
    IsPreconnected (translate t X) :=
  hX.image _ (continuous_id.add continuous_const).continuousOn

/-! ### From the faces to the whole closed set -/

/-- The faces of a closed set, each with the closed form's data. -/
structure Faces (ι : Type*) where
  S : ι → Set (ℝ × ℝ)
  H : ι → Set ℝ
  D : ι → ℝ
  lower : ι → ℝ

/-- The body's top (an axis point at height zb of the tool, with w(zb) ≥ 0) is above A at
every instant: the one fact beyond the faces that the clearance needs (else the body might sit
wholly inside A, touching no face). -/
def TopAbove (Z : Set ℝ) (w : ℝ → ℝ) (zt : ℝ) (A : Set E3) : Prop :=
  ∃ zb ∈ Z, 0 ≤ w zb ∧ ∀ p ∈ A, p 2 < zt + zb

theorem out_of_topAbove {Z : Set ℝ} {w : ℝ → ℝ} {zt : ℝ} {A : Set E3}
    (h : TopAbove Z w zt A) (γ : ℝ → ℝ × ℝ) (s : ℝ) :
    ∃ x ∈ translate (planar γ zt s) (hyp Z w), x ∉ A := by
  obtain ⟨zb, hzb, hw, hA⟩ := h
  refine ⟨mk 0 0 zb + planar γ zt s, ⟨mk 0 0 zb, ⟨by simpa using hzb, by simpa using hw⟩, rfl⟩,
    fun hx => ?_⟩
  have := hA _ hx
  simp [planar] at this; linarith

/-- **The closed-form checker's verdicts give clearance.** A ρ-hypograph body (heights an
interval, radius never negative) on a planar move; a closed set A whose frontier lies in prism
faces; on every face the closed form answered `Clear` with lower ≥ c > 0; and the body's top is
above A. Then at every instant every point of the body is at least c from every point of A. -/
theorem closedForm_clear {ι : Type*} {Z : Set ℝ} {w : ℝ → ℝ} (hZ : Z.OrdConnected)
    (hw : ∀ z ∈ Z, 0 ≤ w z) {γ : ℝ → ℝ × ℝ} {zt : ℝ} {A : Set E3} (hA : IsClosed A)
    (fs : Faces ι) (hfr : frontier A ⊆ ⋃ i, prism (fs.S i) (fs.H i))
    (hv : ∀ i, ClearFace Z w γ zt (fs.S i) (fs.H i) (fs.D i) (fs.lower i))
    {c : ℝ} (hc : 0 < c) (hlow : ∀ i, c ≤ fs.lower i) (htop : TopAbove Z w zt A) :
    ∀ s ∈ I01, ∀ x ∈ planarMove γ zt s '' hyp Z w, ∀ y ∈ A, c ≤ dist x y := by
  intro s hs x hx y hy
  have hx' : x ∈ translate (planar γ zt s) (hyp Z w) := hx
  refine clear_of_frontier (translate_preconnected (hyp_preconnected hZ hw) _) hA hfr hc
    (fun f hf => ?_) (out_of_topAbove htop γ s) x hx' y hy
  obtain ⟨i, hi⟩ := mem_iUnion.mp hf
  exact (hlow i).trans (clearFace_sound (hv i) s hs f hi)

/-- Points at a positive distance from all of A are not in it. -/
theorem disjoint_of_clear {X A : Set E3} {c : ℝ} (hc : 0 < c)
    (h : ∀ x ∈ X, ∀ y ∈ A, c ≤ dist x y) : Disjoint X A :=
  Set.disjoint_left.mpr fun x hx hxA => by
    have := h x hx x hxA; simp at this; linarith

/-! ### G1, G2, G3 for 2.5D -/

/-- **G1 from the closed form.** The cutter T = hyp Z w on a planar move, the part P closed with
its frontier in checked prism faces, every face's lower bound at least δ − ε_in. -/
theorem G1_of_closedForm {ι : Type*} {Z : Set ℝ} {w : ℝ → ℝ} (hZ : Z.OrdConnected)
    (hw : ∀ z ∈ Z, 0 ≤ w z) {γ : ℝ → ℝ × ℝ} {zt : ℝ} {P : Set E3} (hP : IsClosed P)
    (fs : Faces ι) (hfr : frontier P ⊆ ⋃ i, prism (fs.S i) (fs.H i))
    (hv : ∀ i, ClearFace Z w γ zt (fs.S i) (fs.H i) (fs.D i) (fs.lower i))
    {δ εin : ℝ} (hlow : ∀ i, δ - εin ≤ fs.lower i) (htop : TopAbove Z w zt P) :
    G1 (hyp Z w) P (planarMove γ zt) δ εin := by
  intro s hs x hx y hy
  rcases le_or_gt (δ - εin) 0 with hle | hpos
  · exact hle.trans dist_nonneg
  · exact closedForm_clear hZ hw hP fs hfr hv hpos hlow htop s hs x hx y hy

/-- **G2 from the closed form.** The non-cutting body N = hyp Z w is checked against the
stock S₀ as it stood before the piece (every S s ⊆ S₀) and against the fixture's closed hull Fx
(every F s ⊆ Fx), each with a positive clearance c. -/
theorem G2_of_closedForm {ι κ : Type*} {Z : Set ℝ} {w : ℝ → ℝ} (hZ : Z.OrdConnected)
    (hw : ∀ z ∈ Z, 0 ≤ w z) {γ : ℝ → ℝ × ℝ} {zt : ℝ} {c : ℝ} (hc : 0 < c)
    {S₀ Fx : Set E3} {S F : ℝ → Set E3} (hS : ∀ s, S s ⊆ S₀) (hF : ∀ s, F s ⊆ Fx)
    (hS₀ : IsClosed S₀) (ss : Faces ι) (hsfr : frontier S₀ ⊆ ⋃ i, prism (ss.S i) (ss.H i))
    (hsv : ∀ i, ClearFace Z w γ zt (ss.S i) (ss.H i) (ss.D i) (ss.lower i))
    (hslow : ∀ i, c ≤ ss.lower i) (hstop : TopAbove Z w zt S₀)
    (hFx : IsClosed Fx) (xs : Faces κ) (hxfr : frontier Fx ⊆ ⋃ i, prism (xs.S i) (xs.H i))
    (hxv : ∀ i, ClearFace Z w γ zt (xs.S i) (xs.H i) (xs.D i) (xs.lower i))
    (hxlow : ∀ i, c ≤ xs.lower i) (hxtop : TopAbove Z w zt Fx) :
    G2 (hyp Z w) S F (planarMove γ zt) := by
  intro s hs
  constructor
  · have := disjoint_of_clear hc (closedForm_clear hZ hw hS₀ ss hsfr hsv hc hslow hstop s hs)
    exact this.mono_right (interior_subset.trans (hS s))
  · have := disjoint_of_clear hc (closedForm_clear hZ hw hFx xs hxfr hxv hc hxlow hxtop s hs)
    exact this.mono_right (hF s)

/-- **G3 from the closed form.** On a rapid or link, the cutter checked against the stock before
the piece with a positive clearance. -/
theorem G3_of_closedForm {ι : Type*} {Z : Set ℝ} {w : ℝ → ℝ} (hZ : Z.OrdConnected)
    (hw : ∀ z ∈ Z, 0 ≤ w z) {γ : ℝ → ℝ × ℝ} {zt : ℝ} {c : ℝ} (hc : 0 < c)
    {S₀ : Set E3} {S : ℝ → Set E3} (hS : ∀ s, S s ⊆ S₀) (hS₀ : IsClosed S₀) (ss : Faces ι)
    (hsfr : frontier S₀ ⊆ ⋃ i, prism (ss.S i) (ss.H i))
    (hsv : ∀ i, ClearFace Z w γ zt (ss.S i) (ss.H i) (ss.D i) (ss.lower i))
    (hslow : ∀ i, c ≤ ss.lower i) (hstop : TopAbove Z w zt S₀) :
    G3 (hyp Z w) S (planarMove γ zt) := by
  intro s hs
  have := disjoint_of_clear hc (closedForm_clear hZ hw hS₀ ss hsfr hsv hc hslow hstop s hs)
  exact this.mono_right (interior_subset.trans (hS s))

end ForgeCam
