/-
The 2.5D closed-form checker (kernel/forge-cam-geom/src/slab.rs) is sound: its verdicts imply
G1, G2 (corrected, design.md §7b) and G3 for faces that are prisms along z, and moves that are
planar or vertical.

A move puts the tip at (γ(s), ζ(s)) with ζ(s) in a set Zt of heights. A planar move has
Zt = {z_t}, and γ is a segment or an arc (any curve here). A vertical move has γ constant and
Zt = [z0, z1]. A prism face is {(σ, h) : σ ∈ S, h ∈ H}. A horizontal face is S = its region,
H = {z_f}. A wall is S = its section curve, H = [z0, z1].

The checker's `Clear { lower }` for one face rests on two facts it computes:
(i) 0 ≤ D ≤ |σ − γ(s)| for every instant and every section point (D is the low end of
    slab::dist's interval, the 2D closed form);
(ii) lower ≤ sdRZ(D, h − z) for every height h of the face and every tip height z ∈ Zt (the Δ
    branch and bound, whose cells cover H − Zt).
Only the low end of D matters: that is the monotonicity lemma (ForgeCam/Hypograph.lean). Each
lowered face is within μ of the true one, and the claim made is lower − μ (sd is 1-Lipschitz).
-/
import ForgeCam.Spec

open Metric Set

namespace ForgeCam

/-- The distance in the plane. -/
noncomputable def d2 (a b : ℝ × ℝ) : ℝ := √((a.1 - b.1) ^ 2 + (a.2 - b.2) ^ 2)

/-- The tip at instant s. -/
def tip (γ : ℝ → ℝ × ℝ) (ζ : ℝ → ℝ) (s : ℝ) : E3 := mk (γ s).1 (γ s).2 (ζ s)

/-- The placement: the translation to the tip. -/
def move (γ : ℝ → ℝ × ℝ) (ζ : ℝ → ℝ) : ℝ → E3 → E3 := fun s x => x + tip γ ζ s

/-- The tip's heights stay in Zt (Zt = {z_t} planar, [z0, z1] vertical). -/
def HeightsIn (ζ : ℝ → ℝ) (Zt : Set ℝ) : Prop := ∀ s ∈ I01, ζ s ∈ Zt

/-- A face that is a prism along z. -/
def prism (S : Set (ℝ × ℝ)) (H : Set ℝ) : Set E3 := {p | (p 0, p 1) ∈ S ∧ p 2 ∈ H}

/-- What the closed form establishes for one face before it answers `Clear { lower }`. -/
def ClearFace (Z : Set ℝ) (w : ℝ → ℝ) (γ : ℝ → ℝ × ℝ) (Zt : Set ℝ) (S : Set (ℝ × ℝ))
    (H : Set ℝ) (D lower : ℝ) : Prop :=
  0 ≤ D ∧ (∀ s ∈ I01, ∀ σ ∈ S, D ≤ d2 σ (γ s)) ∧ ∀ h ∈ H, ∀ z ∈ Zt, lower ≤ sdRZ Z w D (h - z)

theorem ρ_sub_tip (f : E3) (γ : ℝ → ℝ × ℝ) (ζ : ℝ → ℝ) (s : ℝ) :
    ρ (f - tip γ ζ s) = d2 (f 0, f 1) (γ s) := by
  simp [ρ, d2, tip]

/-- **The verdict is sound for one face**: every point of the face is at signed distance at
least `lower` from the moved body, at every instant. -/
theorem clearFace_sound {Z : Set ℝ} {w : ℝ → ℝ} {γ : ℝ → ℝ × ℝ} {ζ : ℝ → ℝ} {Zt : Set ℝ}
    (hζ : HeightsIn ζ Zt) {S : Set (ℝ × ℝ)} {H : Set ℝ} {D lower : ℝ}
    (h : ClearFace Z w γ Zt S H D lower) :
    ∀ s ∈ I01, ∀ f ∈ prism S H, lower ≤ sd (translate (tip γ ζ s) (hyp Z w)) f := by
  obtain ⟨hD0, hD, hH⟩ := h
  intro s hs f ⟨hfS, hfH⟩
  rw [sd_translate]
  have hz : (f - tip γ ζ s) 2 = f 2 - ζ s := by simp [tip]
  refine (hH (f 2) hfH (ζ s) (hζ s hs)).trans ?_
  rw [← hz]
  exact sdRZ_le_sd hD0 (by rw [ρ_sub_tip]; exact hD s hs _ hfS)

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
structure Faces (ι : Type) where
  S : ι → Set (ℝ × ℝ)
  H : ι → Set ℝ
  D : ι → ℝ
  lower : ι → ℝ

/-- The body's top (an axis point at height zb of the tool, with w(zb) ≥ 0) is above A at
every tip height: ops::certify's `containment` check (tip_z_min + top > every face's max z).
It is the one fact beyond the faces that the clearance needs: without it the body might sit
wholly inside A, touching no face. -/
def TopAbove (Z : Set ℝ) (w : ℝ → ℝ) (Zt : Set ℝ) (A : Set E3) : Prop :=
  ∃ zb ∈ Z, 0 ≤ w zb ∧ ∀ p ∈ A, ∀ z ∈ Zt, p 2 < z + zb

theorem out_of_topAbove {Z : Set ℝ} {w : ℝ → ℝ} {Zt : Set ℝ} {A : Set E3}
    (h : TopAbove Z w Zt A) (γ : ℝ → ℝ × ℝ) {ζ : ℝ → ℝ} (hζ : HeightsIn ζ Zt) {s : ℝ}
    (hs : s ∈ I01) : ∃ x ∈ translate (tip γ ζ s) (hyp Z w), x ∉ A := by
  obtain ⟨zb, hzb, hw, hA⟩ := h
  refine ⟨mk 0 0 zb + tip γ ζ s, ⟨mk 0 0 zb, ⟨by simpa using hzb, by simpa using hw⟩, rfl⟩,
    fun hx => ?_⟩
  have := hA _ hx (ζ s) (hζ s hs)
  simp [tip] at this; linarith

/-- **What the checker certified** for one body (hyp Z w) on one move (γ, ζ) against one closed
set A, at clearance c: A is closed, the body's top is above it, and its frontier lies within μ
of faces each answered `Clear` with lower ≥ c + μ. -/
def Certified (Z : Set ℝ) (w : ℝ → ℝ) (γ : ℝ → ℝ × ℝ) (Zt : Set ℝ) (A : Set E3) (c : ℝ) :
    Prop :=
  IsClosed A ∧ TopAbove Z w Zt A ∧
  ∃ (ι : Type) (fs : Faces ι) (μ : ℝ),
    (∀ f ∈ frontier A, ∃ i, ∃ f' ∈ prism (fs.S i) (fs.H i), dist f f' ≤ μ) ∧
    (∀ i, ClearFace Z w γ Zt (fs.S i) (fs.H i) (fs.D i) (fs.lower i)) ∧
    ∀ i, c + μ ≤ fs.lower i

/-- **The closed-form checker's verdicts give clearance.** For a ρ-hypograph body (heights an
interval, radius never negative) certified against A at c > 0, at every instant every point of
the moved body is at least c from every point of A. -/
theorem certified_clear {Z : Set ℝ} {w : ℝ → ℝ} (hZ : Z.OrdConnected) (hw : ∀ z ∈ Z, 0 ≤ w z)
    {γ : ℝ → ℝ × ℝ} {ζ : ℝ → ℝ} {Zt : Set ℝ} (hζ : HeightsIn ζ Zt) {A : Set E3} {c : ℝ}
    (hc : 0 < c) (h : Certified Z w γ Zt A c) :
    ∀ s ∈ I01, ∀ x ∈ move γ ζ s '' hyp Z w, ∀ y ∈ A, c ≤ dist x y := by
  obtain ⟨hA, htop, ι, fs, μ, hfr, hv, hlow⟩ := h
  intro s hs x hx y hy
  have hx' : x ∈ translate (tip γ ζ s) (hyp Z w) := hx
  refine clear_of_frontier (F := frontier A) (translate_preconnected (hyp_preconnected hZ hw) _)
    hA subset_rfl hc (fun f hf => ?_) (out_of_topAbove htop γ hζ hs) x hx' y hy
  obtain ⟨i, f', hf', hd⟩ := hfr f hf
  have h1 := clearFace_sound hζ (hv i) s hs f' hf'
  have h2 := sd_le_sd_add_dist (translate (tip γ ζ s) (hyp Z w)) f' f
  rw [dist_comm] at h2
  linarith [hlow i]

/-- Points at a positive distance from all of A are not in it. -/
theorem disjoint_of_clear {X A : Set E3} {c : ℝ} (hc : 0 < c)
    (h : ∀ x ∈ X, ∀ y ∈ A, c ≤ dist x y) : Disjoint X A :=
  Set.disjoint_left.mpr fun x hx hxA => by
    have := h x hx x hxA; simp at this; linarith

/-- Points more than c from all of A miss A ⊕ B_c. -/
theorem disjoint_cthickening_of_clear {X A : Set E3} {c c' : ℝ} (hcc : c < c') (hc' : 0 < c')
    (h : ∀ x ∈ X, ∀ y ∈ A, c' ≤ dist x y) : Disjoint X (cthickening c A) :=
  Set.disjoint_left.mpr fun x hx hxA => by
    obtain ⟨y, hy, hd⟩ := mem_thickening_iff.mp (cthickening_subset_thickening' hc' hcc A hxA)
    linarith [h x hx y hy]

/-! ### G1, G2, G3 for 2.5D -/

/-- **G1 from the closed form.** The cutter T = hyp Z w, certified against the part P at
δ − ε_in. (P may be P' = P ∖ Δ, the part less its declared removals: design.md §7b.) -/
theorem G1_of_closedForm {Z : Set ℝ} {w : ℝ → ℝ} (hZ : Z.OrdConnected)
    (hw : ∀ z ∈ Z, 0 ≤ w z) {γ : ℝ → ℝ × ℝ} {ζ : ℝ → ℝ} {Zt : Set ℝ} (hζ : HeightsIn ζ Zt)
    {P : Set E3} {δ εin : ℝ} (h : Certified Z w γ Zt P (δ - εin)) :
    G1 (hyp Z w) P (move γ ζ) δ εin := by
  intro s hs x hx y hy
  rcases le_or_gt (δ - εin) 0 with hle | hpos
  · exact hle.trans dist_nonneg
  · exact certified_clear hZ hw hζ hpos h s hs x hx y hy

/-- **The closed form of G2 from the closed form, with a margin** (the spec's G2 is the open form,
`G2open_of_closedForm` in ForgeCam/Vise.lean). Both the cutter T and the
non-cutting part N are certified against the fixture's closed hull Fx (every F s ⊆ Fx) at some
c' > c, so the whole tool misses F ⊕ B_c; N is certified against the stock S₀ as it stood
before the piece (every S s ⊆ S₀) at some c₀ > 0. It holds on every piece kind: the move is
any planar or vertical one. -/
theorem G2closed_of_closedForm {Zc Zn : Set ℝ} {wc wn : ℝ → ℝ} (hZc : Zc.OrdConnected)
    (hwc : ∀ z ∈ Zc, 0 ≤ wc z) (hZn : Zn.OrdConnected) (hwn : ∀ z ∈ Zn, 0 ≤ wn z)
    {γ : ℝ → ℝ × ℝ} {ζ : ℝ → ℝ} {Zt : Set ℝ} (hζ : HeightsIn ζ Zt)
    {c c' c₀ : ℝ} (hcc : c < c') (hc' : 0 < c') (hc₀ : 0 < c₀)
    {S₀ Fx : Set E3} {S F : ℝ → Set E3} (hS : ∀ s, S s ⊆ S₀) (hF : ∀ s, F s ⊆ Fx)
    (hTF : Certified Zc wc γ Zt Fx c') (hNF : Certified Zn wn γ Zt Fx c')
    (hNS : Certified Zn wn γ Zt S₀ c₀) :
    G2closed (hyp Zc wc) (hyp Zn wn) S F (move γ ζ) c := by
  intro s hs
  have h1 := certified_clear hZc hwc hζ hc' hTF s hs
  have hT : Disjoint (move γ ζ s '' hyp Zc wc) (cthickening c Fx) :=
    disjoint_cthickening_of_clear hcc hc' h1
  have h2 := certified_clear hZn hwn hζ hc' hNF s hs
  have hN : Disjoint (move γ ζ s '' hyp Zn wn) (cthickening c Fx) :=
    disjoint_cthickening_of_clear hcc hc' h2
  have h3 := certified_clear hZn hwn hζ hc₀ hNS s hs
  have hNS' : Disjoint (move γ ζ s '' hyp Zn wn) S₀ := disjoint_of_clear hc₀ h3
  have hU : Disjoint (move γ ζ s '' (hyp Zc wc ∪ hyp Zn wn)) (cthickening c Fx) := by
    rw [image_union]; exact hT.union_left hN
  have hU' : Disjoint (move γ ζ s '' (hyp Zc wc ∪ hyp Zn wn)) (cthickening c (F s)) :=
    Disjoint.mono_right (cthickening_subset_of_subset c (hF s)) hU
  have hS' : Disjoint (move γ ζ s '' hyp Zn wn) (interior (S s)) :=
    Disjoint.mono_right (interior_subset.trans (hS s)) hNS'
  exact ⟨hU', hS'⟩

/-- **G3 from the closed form.** On a rapid or link, the cutter certified against the stock
before the piece at a positive clearance. -/
theorem G3_of_closedForm {Z : Set ℝ} {w : ℝ → ℝ} (hZ : Z.OrdConnected)
    (hw : ∀ z ∈ Z, 0 ≤ w z) {γ : ℝ → ℝ × ℝ} {ζ : ℝ → ℝ} {Zt : Set ℝ} (hζ : HeightsIn ζ Zt)
    {c : ℝ} (hc : 0 < c) {S₀ : Set E3} {S : ℝ → Set E3} (hS : ∀ s, S s ⊆ S₀)
    (h : Certified Z w γ Zt S₀ c) : G3 (hyp Z w) S (move γ ζ) := by
  intro s hs
  exact (disjoint_of_clear hc (certified_clear hZ hw hζ hc h s hs)).mono_right
    (interior_subset.trans (hS s))

/-- The planar move of the first statement: one tip height. -/
theorem heightsIn_planar (zt : ℝ) : HeightsIn (fun _ => zt) {zt} := fun _ _ => rfl

/-- A vertical move between two heights. -/
theorem heightsIn_vertical {z0 z1 : ℝ} {ζ : ℝ → ℝ} (h : ∀ s ∈ I01, z0 ≤ ζ s ∧ ζ s ≤ z1) :
    HeightsIn ζ (Icc z0 z1) := h

end ForgeCam
