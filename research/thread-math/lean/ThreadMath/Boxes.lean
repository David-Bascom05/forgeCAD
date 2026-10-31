import Mathlib.Analysis.InnerProductSpace.Basic
import Mathlib.Topology.Order.Compact
import Mathlib.Tactic

/-!
# Boxes: `Body::face_box` for faces straight across in v, and `screw::misses`
-/

set_option linter.style.header false
set_option linter.style.longLine false
set_option linter.unusedSimpArgs false

namespace ThreadMath

/-- A coordinate of S(u, v), affine in v, between its values at the column's two boundary points
v1 ≤ v ≤ v2 lies between them. Needs only affinity in v. -/
theorem between_boundary {A B lo hi v v1 v2 : ℝ} (hv : v1 ≤ v ∧ v ≤ v2)
    (h1 : lo ≤ A + B * v1 ∧ A + B * v1 ≤ hi) (h2 : lo ≤ A + B * v2 ∧ A + B * v2 ≤ hi) :
    lo ≤ A + B * v ∧ A + B * v ≤ hi := by
  rcases le_total 0 B with hB | hB
  · constructor <;> nlinarith [mul_le_mul_of_nonneg_left hv.1 hB, mul_le_mul_of_nonneg_left hv.2 hB, h1.1, h2.2]
  · constructor <;> nlinarith [mul_le_mul_of_nonpos_left hv.1 hB, mul_le_mul_of_nonpos_left hv.2 hB, h1.2, h2.1]

/-- The column through a point of a compact face region reaches the region's boundary above and
below it: the top and bottom of the (compact) column are not interior to the region. -/
theorem column_ends {R : Set (ℝ × ℝ)} (hR : IsCompact R) {u v : ℝ} (hp : (u, v) ∈ R) :
    ∃ v1 v2, v1 ≤ v ∧ v ≤ v2 ∧ (u, v1) ∈ frontier R ∧ (u, v2) ∈ frontier R := by
  have hC : IsCompact {w : ℝ | (u, w) ∈ R} := by
    have hcl : IsClosed {w : ℝ | (u, w) ∈ R} :=
      hR.isClosed.preimage (by fun_prop : Continuous fun w : ℝ => (u, w))
    obtain ⟨a, ha⟩ := (hR.image continuous_snd).isBounded.bddBelow
    obtain ⟨b, hb⟩ := (hR.image continuous_snd).isBounded.bddAbove
    refine Metric.isCompact_of_isClosed_isBounded hcl ?_
    refine (Metric.isBounded_Icc a b).subset ?_
    intro w hw
    exact ⟨ha ⟨(u, w), hw, rfl⟩, hb ⟨(u, w), hw, rfl⟩⟩
  have hne : ({w : ℝ | (u, w) ∈ R}).Nonempty := ⟨v, hp⟩
  obtain ⟨v1, hv1, hmin⟩ := hC.exists_isLeast hne
  obtain ⟨v2, hv2, hmax⟩ := hC.exists_isGreatest hne
  have front : ∀ w, (u, w) ∈ R → (∀ e > 0, ∃ w', |w' - w| < e ∧ (u, w') ∉ R) → (u, w) ∈ frontier R := by
    intro w hw hout
    rw [frontier, Set.mem_diff]
    refine ⟨subset_closure hw, fun hint => ?_⟩
    obtain ⟨e, he, hball⟩ := Metric.isOpen_iff.mp isOpen_interior _ hint
    obtain ⟨w', hw', hn⟩ := hout e he
    apply hn
    apply interior_subset
    apply hball
    simp only [Metric.mem_ball, Prod.dist_eq, dist_self, Real.dist_eq]
    simpa using hw'
  refine ⟨v1, v2, hmin hp, hmax hp, front v1 hv1 ?_, front v2 hv2 ?_⟩
  · intro e he
    refine ⟨v1 - e / 2, by rw [show v1 - e / 2 - v1 = -(e / 2) by ring, abs_neg, abs_of_pos (by linarith)]; linarith, fun hin => ?_⟩
    have := hmin hin; linarith
  · intro e he
    refine ⟨v2 + e / 2, by rw [show v2 + e / 2 - v2 = e / 2 by ring, abs_of_pos (by linarith)]; linarith, fun hin => ?_⟩
    have := hmax hin; linarith

/-- The theorem `face_box` uses: if the surface is affine in v, every point of a compact face
region lies in any box holding the images of its boundary. Not needed: the region simply connected
or u-monotone, the boundary smooth, or the surface being a screw at all (any ruled patch). -/
theorem face_in_hull_of_boundary {R : Set (ℝ × ℝ)} (hR : IsCompact R)
    (A B : ℝ → ℝ) {lo hi : ℝ}
    (hb : ∀ p ∈ frontier R, lo ≤ A p.1 + B p.1 * p.2 ∧ A p.1 + B p.1 * p.2 ≤ hi)
    {p : ℝ × ℝ} (hp : p ∈ R) : lo ≤ A p.1 + B p.1 * p.2 ∧ A p.1 + B p.1 * p.2 ≤ hi := by
  obtain ⟨u, v⟩ := p
  obtain ⟨v1, v2, h1, h2, f1, f2⟩ := column_ends hR hp
  exact between_boundary ⟨h1, h2⟩ (hb _ f1) (hb _ f2)

/-! ## The plain miss -/

variable {V : Type*} [NormedAddCommGroup V] [InnerProductSpace ℝ V]
local notation "⟪" x ", " y "⟫" => (inner ℝ x y : ℝ)

/-- Bessel for the axis a against the orthonormal z, e: ⟪a,e⟫² ≤ 1 - ⟪a,z⟫². -/
theorem bessel_two {a z e : V} (ha : ⟪a, a⟫ = 1) (hz : ⟪z, z⟫ = 1) (he : ⟪e, e⟫ = 1)
    (hze : ⟪z, e⟫ = 0) : ⟪a, e⟫ ^ 2 ≤ 1 - ⟪a, z⟫ ^ 2 := by
  have h := real_inner_self_nonneg (x := a - ⟪a, z⟫ • z - ⟪a, e⟫ • e)
  have hez : ⟪e, z⟫ = 0 := by rw [real_inner_comm z e]; exact hze
  have hza : ⟪z, a⟫ = ⟪a, z⟫ := real_inner_comm a z
  have hea : ⟪e, a⟫ = ⟪a, e⟫ := real_inner_comm a e
  simp only [inner_sub_left, inner_sub_right, inner_smul_left, inner_smul_right, ha, hz, he, hze, hez,
    hza, hea, conj_trivial] at h
  nlinarith

/-- `misses`, the last bound: with a the cylinder's unit axis, z the screw's, P x = x - ⟪a,x⟫ a,
g = |P z| > 0 and u = P z / g, a shell point axis + r e (e a unit vector square to z) reads along u
within r |⟪a,z⟫| of its axis point: its distance from the cylinder's axis, |P(w + r e)|, is at least
the distance of the axis point along u, less r |⟪a,z⟫|. Needs e ⟂ z and g > 0; if a ∥ z the code
returns false instead (`gg ≤ 1e-18`). -/
theorem shell_reading {a z e w : V} {g r : ℝ} (ha : ⟪a, a⟫ = 1) (hz : ⟪z, z⟫ = 1) (he : ⟪e, e⟫ = 1)
    (hze : ⟪z, e⟫ = 0) (hg : 0 < g) (hg2 : g ^ 2 = 1 - ⟪a, z⟫ ^ 2) (hr : 0 ≤ r) :
    let Pz := z - ⟪a, z⟫ • a
    let P : V → V := fun x => x - ⟪a, x⟫ • a
    |⟪Pz, P w⟫| / g - r * |⟪a, z⟫| ≤ ‖P (w + r • e)‖ := by
  intro Pz P
  have hPz : ‖Pz‖ = g := by
    have h : ⟪Pz, Pz⟫ = g ^ 2 := by
      simp only [Pz, inner_sub_left, inner_sub_right, inner_smul_left, inner_smul_right, ha, hz,
        real_inner_comm a z, conj_trivial]; rw [hg2]; ring
    rw [real_inner_self_eq_norm_sq] at h
    exact (sq_eq_sq₀ (norm_nonneg _) hg.le).mp h
  have hcs := abs_real_inner_le_norm Pz (P (w + r • e))
  have split : ⟪Pz, P (w + r • e)⟫ = ⟪Pz, P w⟫ + r * ⟪Pz, P e⟫ := by
    simp only [P, Pz, inner_sub_left, inner_sub_right, inner_add_right, inner_smul_left,
      inner_smul_right, inner_add_left, conj_trivial, ha, real_inner_comm a z]
    ring
  have hPe : ⟪Pz, P e⟫ = -(⟪a, z⟫ * ⟪a, e⟫) := by
    simp only [P, Pz, inner_sub_left, inner_sub_right, inner_smul_left, inner_smul_right, conj_trivial,
      ha, hze, real_inner_comm a z]
    ring
  have hae : |⟪a, e⟫| ≤ g := by
    have := bessel_two ha hz he hze
    rw [← hg2] at this
    exact abs_le_of_sq_le_sq this hg.le
  have hbound : |r * ⟪Pz, P e⟫| ≤ r * (|⟪a, z⟫| * g) := by
    rw [hPe, abs_mul, abs_neg, abs_mul, abs_of_nonneg hr]
    exact mul_le_mul_of_nonneg_left (mul_le_mul_of_nonneg_left hae (abs_nonneg _)) hr
  have htri : |⟪Pz, P w⟫| ≤ |⟪Pz, P (w + r • e)⟫| + |r * ⟪Pz, P e⟫| := by
    rw [split]
    have := abs_sub (⟪Pz, P w⟫ + r * ⟪Pz, P e⟫) (r * ⟪Pz, P e⟫)
    simpa using this
  rw [hPz] at hcs
  rw [sub_le_iff_le_add, div_le_iff₀ hg]
  nlinarith

end ThreadMath
