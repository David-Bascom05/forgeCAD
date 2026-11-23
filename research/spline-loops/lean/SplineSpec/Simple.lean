/-
When a profile loop is simple: the spec the kernel's `crosses_itself` decides (forge-ops profile.rs,
`loop_meets`), and the two facts its certified search rests on.

A loop is a cyclic list of pieces, each a rational Bezier curve with positive weights, piece i ending
where piece i + 1 starts. It is **refused** when

* two pieces that are not neighbours come within ε (the kernel's one tolerance, `tol::LINEAR`): they
  touch or cross (`Touch`); or
* two neighbours meet anywhere but their joint (`MeetsOffJoint`), in particular a piece crossing
  itself, the pieces of a spline being neighbours.

The search subdivides pairs of pieces. Two facts decide a pair without more subdivision:

* `apart_of_boxes`: pieces whose control boxes are more than ε apart in one coordinate (a box holds
  its piece: `eval_between`) are more than ε apart, so they do not touch;
* `cone_sep`: two pieces leaving a common joint J, whose control points other than J lie strictly on
  either side of a line through J, have no common point but J (each piece is J plus a nonnegative
  combination of its control points' offsets from J).

A pair is refused as touching when two of its points, evaluated, are within ε. A pair the search
cannot decide within its budget is refused at its locus, undecided.
-/
import SplineSpec.Cert
import Mathlib.Analysis.Calculus.Deriv.MeanValue

open Finset

namespace SplineSpec

/-- Points of the plane. -/
abbrev P2 := ℝ × ℝ

/-- Two pieces `A B : ℝ → P2` on [0, 1] touch within ε. -/
def Touch (A B : ℝ → P2) (ε : ℝ) : Prop :=
  ∃ s ∈ Set.Icc (0 : ℝ) 1, ∃ r ∈ Set.Icc (0 : ℝ) 1, dist (A s) (B r) ≤ ε

/-- Two neighbours (A ending at its 1, B starting at its 0) meet off their joint. -/
def MeetsOffJoint (A B : ℝ → P2) : Prop :=
  ∃ s ∈ Set.Icc (0 : ℝ) 1, ∃ r ∈ Set.Icc (0 : ℝ) 1, A s = B r ∧ ¬ (s = 1 ∧ r = 0)

/-- A loop of n pieces is accepted: no two non-neighbours touch within ε, no two neighbours (and
no piece with itself, at two parameters) meet but at their joint. -/
def LoopSimple (piece : ℕ → ℝ → P2) (n : ℕ) (ε : ℝ) : Prop :=
  (∀ i < n, ∀ j < n, i ≠ j → j ≠ (i + 1) % n → i ≠ (j + 1) % n → ¬ Touch (piece i) (piece j) ε) ∧
  (∀ i < n, ¬ MeetsOffJoint (piece i) (piece ((i + 1) % n))) ∧
  (∀ i < n, ∀ s ∈ Set.Icc (0 : ℝ) 1, ∀ r ∈ Set.Icc (0 : ℝ) 1, piece i s = piece i r → s = r)

/-- **Boxes apart**: if every point of A has first coordinate at most a and every point of B at
least b > a + ε (or the same in the second coordinate, either way round), A and B do not touch. -/
theorem apart_of_boxes (A B : ℝ → P2) (ε a b : ℝ) (hab : a + ε < b)
    (hA : ∀ s ∈ Set.Icc (0 : ℝ) 1, (A s).1 ≤ a) (hB : ∀ r ∈ Set.Icc (0 : ℝ) 1, b ≤ (B r).1) :
    ¬ Touch A B ε := by
  rintro ⟨s, hs, r, hr, hd⟩
  have h1 := hA s hs
  have h2 := hB r hr
  have : |(A s).1 - (B r).1| ≤ dist (A s) (B r) := by
    rw [Prod.dist_eq, Real.dist_eq]; exact le_max_left _ _
  rw [abs_le] at this
  linarith

/-- The same in the second coordinate. -/
theorem apart_of_boxes' (A B : ℝ → P2) (ε a b : ℝ) (hab : a + ε < b)
    (hA : ∀ s ∈ Set.Icc (0 : ℝ) 1, (A s).2 ≤ a) (hB : ∀ r ∈ Set.Icc (0 : ℝ) 1, b ≤ (B r).2) :
    ¬ Touch A B ε := by
  rintro ⟨s, hs, r, hr, hd⟩
  have h1 := hA s hs
  have h2 := hB r hr
  have : |(A s).2 - (B r).2| ≤ dist (A s) (B r) := by
    rw [Prod.dist_eq, Real.dist_eq]; exact le_max_right _ _
  rw [abs_le] at this
  linarith

/-- A point in a cone: `J + Σ λ_k v_k`, λ ≥ 0. -/
def InCone (J : P2) (v : ℕ → P2) (m : ℕ) (X : P2) : Prop :=
  ∃ l : ℕ → ℝ, (∀ k < m, 0 ≤ l k) ∧ X = J + ∑ k ∈ range m, l k • v k

/-- The functional `n · (X - J)` on a cone point is the same combination of `n · v_k`. -/
theorem cone_dot (J : P2) (v : ℕ → P2) (m : ℕ) (n : P2) (l : ℕ → ℝ) :
    n.1 * (J + ∑ k ∈ range m, l k • v k - J).1 + n.2 * (J + ∑ k ∈ range m, l k • v k - J).2
      = ∑ k ∈ range m, l k * (n.1 * (v k).1 + n.2 * (v k).2) := by
  simp only [add_sub_cancel_left, Prod.fst_sum, Prod.snd_sum, Prod.smul_fst, Prod.smul_snd,
    smul_eq_mul, mul_sum, ← sum_add_distrib]
  refine sum_congr rfl fun k _ => by ring

/-- **Cones apart**: if a line through J (normal n) has every offset of one piece strictly on one
side and every offset of the other strictly on the other, a common point of the two cones is J. -/
theorem cone_sep (J : P2) (v w : ℕ → P2) (m m' : ℕ) (n : P2)
    (hv : ∀ k < m, n.1 * (v k).1 + n.2 * (v k).2 < 0) (hw : ∀ k < m', 0 < n.1 * (w k).1 + n.2 * (w k).2)
    (X : P2) (hX : InCone J v m X) (hY : InCone J w m' X) : X = J := by
  obtain ⟨l, hl, rfl⟩ := hX
  obtain ⟨l', hl', he⟩ := hY
  -- n·(X - J) is ≤ 0 from the first cone and ≥ 0 from the second, so 0; then every λ_k is 0
  have d1 := cone_dot J v m n l
  have d2 := cone_dot J w m' n l'
  rw [he] at d1
  have hle : ∑ k ∈ range m, l k * (n.1 * (v k).1 + n.2 * (v k).2) ≤ 0 :=
    sum_nonpos fun k hk => mul_nonpos_of_nonneg_of_nonpos (hl k (mem_range.1 hk)) (hv k (mem_range.1 hk)).le
  have hge : 0 ≤ ∑ k ∈ range m', l' k * (n.1 * (w k).1 + n.2 * (w k).2) :=
    sum_nonneg fun k hk => mul_nonneg (hl' k (mem_range.1 hk)) (hw k (mem_range.1 hk)).le
  have h0 : ∑ k ∈ range m, l k * (n.1 * (v k).1 + n.2 * (v k).2) = 0 := by linarith
  have hz : ∀ k ∈ range m, l k * (n.1 * (v k).1 + n.2 * (v k).2) = 0 :=
    (sum_eq_zero_iff_of_nonpos fun k hk =>
      mul_nonpos_of_nonneg_of_nonpos (hl k (mem_range.1 hk)) (hv k (mem_range.1 hk)).le).1 h0
  have hl0 : ∀ k ∈ range m, l k = 0 := fun k hk => by
    rcases mul_eq_zero.1 (hz k hk) with h | h
    · exact h
    · exact absurd h (ne_of_lt (hv k (mem_range.1 hk)))
  rw [sum_congr rfl fun k hk => by rw [hl0 k hk, zero_smul], sum_const_zero, add_zero]

/-- A Bezier piece's point, a convex combination of its control points one of which is J, is in the
cone of the others' offsets from J. -/
theorem convex_in_cone (J : P2) (a : ℕ → P2) (m : ℕ) (β : ℕ → ℝ) (hβ : ∀ k ≤ m, 0 ≤ β k)
    (hsum : ∑ k ∈ range (m + 1), β k = 1) (ha0 : a 0 = J) :
    InCone J (fun k => a (k + 1) - J) m (∑ k ∈ range (m + 1), β k • a k) := by
  refine ⟨fun k => β (k + 1), fun k hk => hβ (k + 1) (by omega), ?_⟩
  have : ∑ k ∈ range (m + 1), β k • a k = (∑ k ∈ range (m + 1), β k) • J
      + ∑ k ∈ range m, β (k + 1) • (a (k + 1) - J) := by
    rw [sum_range_succ', ha0, sum_smul, sum_range_succ' (fun k => β k • J)]
    simp only [smul_sub, sum_sub_distrib]
    abel
  rw [this, hsum, one_smul]

/-- **A piece whose derivative keeps a positive component along a direction meets itself
nowhere**: `n · C` then increases strictly, so two parameters with one point are one. The kernel's
`one_to_one` finds that component's positivity from the Bernstein coefficients of the derivative's
numerator `A' W - A W'` (`bern_pos`); W² > 0 keeps its sign. -/
theorem injective_of_dir_deriv_pos (C : ℝ → P2) (C' : ℝ → P2) (n : P2)
    (hd : ∀ t ∈ Set.Icc (0 : ℝ) 1, HasDerivAt (fun s => n.1 * (C s).1 + n.2 * (C s).2)
      (n.1 * (C' t).1 + n.2 * (C' t).2) t)
    (hpos : ∀ t ∈ Set.Icc (0 : ℝ) 1, 0 < n.1 * (C' t).1 + n.2 * (C' t).2) :
    ∀ s ∈ Set.Icc (0 : ℝ) 1, ∀ r ∈ Set.Icc (0 : ℝ) 1, C s = C r → s = r := by
  have hmono : StrictMonoOn (fun s => n.1 * (C s).1 + n.2 * (C s).2) (Set.Icc 0 1) := by
    apply strictMonoOn_of_deriv_pos (convex_Icc 0 1)
    · intro t ht
      exact (hd t ht).continuousAt.continuousWithinAt
    · intro t ht
      rw [interior_Icc] at ht
      have ht' : t ∈ Set.Icc (0 : ℝ) 1 := ⟨ht.1.le, ht.2.le⟩
      rw [(hd t ht').deriv]
      exact hpos t ht'
  intro s hs r hr he
  by_contra hne
  rcases lt_or_gt_of_ne hne with h | h
  · have := hmono hs hr h; simp only [he] at this; exact lt_irrefl _ this
  · have := hmono hr hs h; simp only [he] at this; exact lt_irrefl _ this

/-- **A piece leaving a tangent joint**: offsets from J whose side of a line through J has
nonnegative Bernstein coefficients, the last positive, are strictly on that side for every t in
(0, 1]; so a piece weakly on the other side meets it only at J (`half_planes_apart`). -/
theorem bern_pos_off_start (p : ℕ) (c : ℕ → ℝ) (hc : ∀ k ≤ p, 0 ≤ c k) (hp : 0 < c p) {t : ℝ}
    (h0 : 0 < t) (h1 : t ≤ 1) : 0 < SplineLoops.bern p c t := by
  unfold SplineLoops.bern
  rw [sum_range_succ]
  have hrest : 0 ≤ ∑ k ∈ range p, c k * SplineLoops.bb p k t :=
    sum_nonneg fun k hk => mul_nonneg (hc k (by have := mem_range.1 hk; omega))
      (SplineLoops.bb_nonneg p k h0.le h1)
  have hlast : 0 < c p * SplineLoops.bb p p t := by
    unfold SplineLoops.bb
    simp only [Nat.choose_self, Nat.cast_one, one_mul, Nat.sub_self, pow_zero, mul_one]
    exact mul_pos hp (pow_pos h0 p)
  linarith

end SplineSpec
