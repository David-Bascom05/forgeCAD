import ForgeCurved.Rooms
import Mathlib.Analysis.Calculus.Deriv.Basic
import Mathlib.Order.Filter.Basic

/-!
# The 2D nesting test: membership by a crossing count along a path

Route (iii) §1 step 3 decides whether a traced loop lies in a face's region by counting where a
path crosses the region's trims. The path is an arc of the conic `q_f ∩ {x = c}` on one of the
sweep's sample planes, anchored at a point known to be in the face. This file is the topology under
that count, for any region `R` of any space.

* **`const_on`**: on a parameter interval where the path never meets `frontier R`, membership
  in `R` is constant;
* **`parity_path`**: if the path meets `frontier R` only at finitely many parameters, and
  membership flips at each of them, then membership at the end is membership at the start, flipped
  once per crossing. **The count's parity decides nesting.**
* **`flip_of_transversal`**: the flip at a crossing, from the trim's local model. Near the
  crossing point, `R` is `{c < 0}` (or `{c > 0}`; F6's orientation), and `c ∘ γ` has a nonzero
  derivative at the crossing parameter (the conic crosses the trim transversally, a certified
  edge sample).

Nothing here is a Jordan curve theorem. The region `R` is given: the face's region, with its
frontier on the trims. What is proven is that the count along a path decides membership in it.
-/

namespace ForgeCurved.Nesting

open Set Filter Topology ForgeCurved.Rooms

variable {X : Type} [TopologicalSpace X]

/-- **No frontier point, no change.** -/
theorem const_on {R : Set X} {γ : ℝ → X} {u v : ℝ} (huv : u ≤ v) (hγ : ContinuousOn γ (Icc u v))
    (hoff : ∀ t ∈ Icc u v, γ t ∉ frontier R) : (γ u ∈ R ↔ γ v ∈ R) := by
  have hc : IsPreconnected (γ '' Icc u v) := isPreconnected_Icc.image γ hγ
  have hd : Disjoint (γ '' Icc u v) (frontier R) := by
    rw [Set.disjoint_left]
    rintro _ ⟨t, ht, rfl⟩
    exact hoff t ht
  have hu : γ u ∈ γ '' Icc u v := ⟨u, left_mem_Icc.mpr huv, rfl⟩
  have hv : γ v ∈ γ '' Icc u v := ⟨v, right_mem_Icc.mpr huv, rfl⟩
  rcases room_class hc hd with h | h
  · exact ⟨fun _ => h hv, fun _ => h hu⟩
  · exact ⟨fun h' => absurd h' (Set.disjoint_left.mp h hu),
      fun h' => absurd h' (Set.disjoint_left.mp h hv)⟩

/-- **Membership flips at `t`**: just before and just after, it differs. -/
def Flips (R : Set X) (γ : ℝ → X) (t : ℝ) : Prop :=
  ∃ ε > 0, ∀ s s', t - ε < s → s < t → t < s' → s' < t + ε → (γ s ∈ R ↔ γ s' ∉ R)

/-- **The crossing count's parity decides membership.** The path meets `frontier R`, on
`[a, b]`, only at the sorted parameters `ts` inside `(a, b)`, and membership flips at each. -/
theorem parity_path {R : Set X} {γ : ℝ → X} :
    ∀ (ts : List ℝ) {a b : ℝ}, a ≤ b → ContinuousOn γ (Icc a b) → ts.Pairwise (· < ·) →
      (∀ t ∈ ts, a < t ∧ t < b) → (∀ t ∈ Icc a b, t ∉ ts → γ t ∉ frontier R) →
      (∀ t ∈ ts, Flips R γ t) → (γ b ∈ R ↔ (γ a ∈ R ↔ Even ts.length))
  | [], a, b, hab, hγ, _, _, hoff, _ => by
    have := const_on hab hγ (fun t ht => hoff t ht (by simp))
    simp only [List.length_nil, Even.zero, iff_true]
    exact this.symm
  | t :: rest, a, b, hab, hγ, hs, hin, hoff, hfl => by
    obtain ⟨hat, htb⟩ := hin t (by simp)
    obtain ⟨ε, hε, hflip⟩ := hfl t (by simp)
    have hrest : ∀ t' ∈ rest, t < t' := by
      intro t' ht'; exact List.rel_of_pairwise_cons hs ht'
    -- the next stop after `t`: the next crossing, or `b`
    set nxt := (rest.head?).getD b
    have hnxt : t < nxt := by
      cases hr : rest with
      | nil => simp [nxt, hr, htb]
      | cons t' r => simp only [nxt, hr, List.head?_cons, Option.getD_some]; exact hrest t' (by simp [hr])
    have hnb : nxt ≤ b := by
      cases hr : rest with
      | nil => simp [nxt, hr]
      | cons t' r => simp only [nxt, hr, List.head?_cons, Option.getD_some]; exact (hin t' (by simp [hr])).2.le
    have hnmin : ∀ t' ∈ rest, nxt ≤ t' := by
      cases hr : rest with
      | nil => simp
      | cons t' r =>
        intro t'' ht''
        simp only [nxt, hr, List.head?_cons, Option.getD_some]
        rw [hr] at hs
        rcases List.mem_cons.mp ht'' with h | h
        · exact h.symm ▸ _root_.le_refl _
        · exact (List.rel_of_pairwise_cons (List.Pairwise.of_cons hs) h).le
    set s := max a (t - ε / 2) + (t - max a (t - ε / 2)) / 2
    set s' := t + min (ε / 2) ((nxt - t) / 2)
    have hm1 : max a (t - ε / 2) < t := max_lt hat (by linarith)
    have hs1 : max a (t - ε / 2) < s := by simp only [s]; linarith
    have hs2 : s < t := by simp only [s]; linarith
    have hmin : 0 < min (ε / 2) ((nxt - t) / 2) := lt_min (by linarith) (by linarith)
    have hs'1 : t < s' := by simp only [s']; linarith
    have hs'2 : s' < t + ε := by simp only [s']; linarith [min_le_left (ε / 2) ((nxt - t) / 2)]
    have hs'3 : s' < nxt := by simp only [s']; linarith [min_le_right (ε / 2) ((nxt - t) / 2)]
    have has : a ≤ s := (le_max_left a _).trans hs1.le
    have htε : t - ε < s := by linarith [le_max_right a (t - ε / 2)]
    -- constant on `[a, s]`
    have h1 : γ a ∈ R ↔ γ s ∈ R := by
      refine const_on has (hγ.mono (Icc_subset_Icc le_rfl (by linarith))) fun u hu => ?_
      refine hoff u ⟨hu.1, by linarith [hu.2]⟩ fun hm => ?_
      rcases List.mem_cons.mp hm with h | h
      · linarith [hu.2]
      · linarith [hrest u h, hu.2]
    -- the flip at `t`
    have h2 : γ s ∈ R ↔ γ s' ∉ R := hflip s s' htε hs2 hs'1 hs'2
    -- the rest, from `s'`
    have h3 := parity_path rest (a := s') (b := b) (by linarith) (hγ.mono (Icc_subset_Icc (by linarith) le_rfl))
      (List.Pairwise.of_cons hs)
      (fun t' ht' => ⟨lt_of_lt_of_le hs'3 (hnmin t' ht'), (hin t' (by simp [ht'])).2⟩)
      (fun u hu hu' => hoff u ⟨by linarith [hu.1], hu.2⟩ fun hm => by
        rcases List.mem_cons.mp hm with h | h
        · linarith [hu.1]
        · exact hu' h)
      (fun t' ht' => hfl t' (by simp [ht']))
    rw [h3, List.length_cons, Nat.even_add_one]
    tauto

/-- **The flip at a transversal crossing.** Near `γ t`, `R` is the negative side of `c` (or the
positive side, `pos`), and `c ∘ γ` vanishes at `t` with a nonzero derivative. -/
theorem flip_of_transversal {R : Set X} {γ : ℝ → X} {c : X → ℝ} {t : ℝ} (pos : Bool)
    (hγ : ContinuousAt γ t) {W : Set X} (hW : W ∈ 𝓝 (γ t))
    (hR : ∀ z ∈ W, (z ∈ R ↔ if pos then 0 < c z else c z < 0))
    {d : ℝ} (hd : HasDerivAt (fun s => c (γ s)) d t) (hd0 : d ≠ 0) (h0 : c (γ t) = 0) :
    Flips R γ t := by
  -- `γ` stays in `W` near `t`
  obtain ⟨ε₁, hε₁, hball⟩ := Metric.eventually_nhds_iff.mp (hγ.preimage_mem_nhds hW)
  -- `c ∘ γ` has the sign of `d · (s − t)` near `t`
  have hlin := hasDerivAt_iff_isLittleO.mp hd
  obtain ⟨ε₂, hε₂, hsmall⟩ := Metric.eventually_nhds_iff.mp
    ((hlin.def (c := |d| / 2) (by positivity)))
  refine ⟨min ε₁ ε₂, lt_min hε₁ hε₂, fun s s' h1 h2 h3 h4 => ?_⟩
  have hsign : ∀ u, |u - t| < min ε₁ ε₂ →
      (0 < d * (u - t) → 0 < c (γ u)) ∧ (d * (u - t) < 0 → c (γ u) < 0) := by
    intro u hu
    have hb := hsmall (y := u) (by rw [Real.dist_eq]; exact lt_of_lt_of_le hu (min_le_right _ _))
    simp only [h0, sub_zero, Real.norm_eq_abs, smul_eq_mul] at hb
    have hm : |d| / 2 * |u - t| = |d * (u - t)| / 2 := by rw [abs_mul]; ring
    rw [hm, mul_comm (u - t) d] at hb
    obtain ⟨hb1, hb2⟩ := abs_le.mp hb
    constructor
    · intro hp; rw [abs_of_pos hp] at hb1; linarith
    · intro hn; rw [abs_of_neg hn] at hb2; linarith
  have hsW : γ s ∈ W := hball (by rw [Real.dist_eq, abs_lt]; constructor <;> linarith [min_le_left ε₁ ε₂])
  have hs'W : γ s' ∈ W := hball (by rw [Real.dist_eq, abs_lt]; constructor <;> linarith [min_le_left ε₁ ε₂])
  have hss := hsign s (by rw [abs_lt]; constructor <;> linarith)
  have hss' := hsign s' (by rw [abs_lt]; constructor <;> linarith)
  have e1 : s - t < 0 := by linarith
  have e2 : 0 < s' - t := by linarith
  have hopp : (0 < c (γ s) ∧ c (γ s') < 0) ∨ (c (γ s) < 0 ∧ 0 < c (γ s')) := by
    rcases lt_or_gt_of_ne hd0 with hdn | hdp
    · exact Or.inl ⟨hss.1 (mul_pos_of_neg_of_neg hdn e1), hss'.2 (mul_neg_of_neg_of_pos hdn e2)⟩
    · exact Or.inr ⟨hss.2 (mul_neg_of_pos_of_neg hdp e1), hss'.1 (mul_pos hdp e2)⟩
  rw [hR _ hsW, hR _ hs'W]
  cases pos <;> simp only [Bool.false_eq_true, ↓reduceIte, not_lt] <;>
    rcases hopp with ⟨a1, a2⟩ | ⟨a1, a2⟩ <;> constructor <;> intro h <;> linarith
/-- **Anchored nesting.** From an anchor known to be in the region (a trim-edge sample pushed to
the face side by its orientation), the end is in the region iff the crossing count is even. -/
theorem anchored {R : Set X} {γ : ℝ → X} (ts : List ℝ) {a b : ℝ} (hab : a ≤ b)
    (hγ : ContinuousOn γ (Icc a b)) (hs : ts.Pairwise (· < ·)) (hin : ∀ t ∈ ts, a < t ∧ t < b)
    (hoff : ∀ t ∈ Icc a b, t ∉ ts → γ t ∉ frontier R) (hfl : ∀ t ∈ ts, Flips R γ t)
    (ha : γ a ∈ R) : (γ b ∈ R ↔ Even ts.length) := by
  rw [parity_path ts hab hγ hs hin hoff hfl]
  exact ⟨fun h => h.mp ha, fun h => ⟨fun _ => h, fun _ => ha⟩⟩

end ForgeCurved.Nesting
