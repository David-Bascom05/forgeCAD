import ForgeCurved.Cells
import Mathlib.Analysis.Calculus.Deriv.MeanValue
import Mathlib.Topology.Order.IntermediateValue
import Mathlib.Data.Finset.Sort

/-!
# The one-dimensional facts of the CAD: sign constancy, Thom's lemma, merging root functions

Tier 1 (design §2.3, a cylindrical decomposition with Thom closure) proves R1 one fibre
direction at a time. This file has the facts that do not depend on the geometry:

* `sgnK_const`: a continuous function with no zero on a preconnected set keeps its sign;
* **`thom`** (Thom's lemma): for differentiable `f₀, …, f_{n-1}` on `ℝ` whose derivatives are
  constants or earlier members, every sign condition is an interval (`OrdConnected`);
* **`merge`**: finitely many continuous functions on a preconnected `C`, any two equal everywhere
  on `C` or nowhere, are a strictly ordered family `ρ 0 < … < ρ (m-1)` with each function one of
  the `ρ k`. This turns per-carrier root functions into the CAD's sections over a base cell.
-/

namespace ForgeCurved.Thom

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Set

/-- A continuous function with no zero on a preconnected set keeps its sign. -/
theorem sgnK_const {α : Type*} [TopologicalSpace α] {S : Set α} (hS : IsPreconnected S)
    {g : α → ℝ} (hg : ContinuousOn g S) (hne : ∀ x ∈ S, g x ≠ 0) {x y : α} (hx : x ∈ S)
    (hy : y ∈ S) : sgnK (g x) = sgnK (g y) := by
  have hI := (hS.image g hg).ordConnected
  have key : ∀ x ∈ S, ∀ y ∈ S, 0 < g x → 0 < g y := by
    intro x hx y hy hpos
    by_contra hneg
    have hlt : g y < 0 := lt_of_le_of_ne (not_lt.mp hneg) (hne y hy)
    obtain ⟨z, hz, hz0⟩ := hI.out (mem_image_of_mem g hy) (mem_image_of_mem g hx)
      ⟨hlt.le, hpos.le⟩
    exact hne z hz hz0
  rcases (hne x hx).lt_or_gt with h1 | h1
  · have h2 : g y < 0 := by
      rcases (hne y hy).lt_or_gt with h | h
      · exact h
      · exact absurd (key y hy x hx h) (not_lt.mpr h1.le)
    rw [sgnK_neg h1, sgnK_neg h2]
  · rw [sgnK_pos h1, sgnK_pos (key x hx y hy h1)]

/-- Positivity persists on a preconnected set where the function never vanishes. -/
theorem pos_const {α : Type*} [TopologicalSpace α] {S : Set α} (hS : IsPreconnected S)
    {g : α → ℝ} (hg : ContinuousOn g S) (hne : ∀ x ∈ S, g x ≠ 0) {x y : α} (hx : x ∈ S)
    (hy : y ∈ S) (hpos : 0 < g x) : 0 < g y := by
  have h := sgnK_const hS hg hne hx hy
  rw [sgnK_pos hpos] at h
  rcases lt_trichotomy (g y) 0 with h' | h' | h'
  · rw [sgnK_neg h'] at h; cases h
  · rw [sgnK_zero_iff.mpr h'] at h; cases h
  · exact h'

/-- Sign classes of `ℝ` are convex. -/
theorem sgnK_between {a b c : ℝ} {σ : Sgn} (ha : sgnK a = σ) (hb : sgnK b = σ) (hac : a ≤ c)
    (hcb : c ≤ b) : sgnK c = σ := by
  rcases lt_trichotomy a 0 with h | h | h <;> rcases lt_trichotomy b 0 with h' | h' | h'
  all_goals first
    | (rw [sgnK_neg h] at ha; rw [sgnK_neg h'] at hb)
    | (rw [sgnK_neg h] at ha; rw [sgnK_zero_iff.mpr h'] at hb)
    | (rw [sgnK_neg h] at ha; rw [sgnK_pos h'] at hb)
    | (rw [sgnK_zero_iff.mpr h] at ha; rw [sgnK_neg h'] at hb)
    | (rw [sgnK_zero_iff.mpr h] at ha; rw [sgnK_zero_iff.mpr h'] at hb)
    | (rw [sgnK_zero_iff.mpr h] at ha; rw [sgnK_pos h'] at hb)
    | (rw [sgnK_pos h] at ha; rw [sgnK_neg h'] at hb)
    | (rw [sgnK_pos h] at ha; rw [sgnK_zero_iff.mpr h'] at hb)
    | (rw [sgnK_pos h] at ha; rw [sgnK_pos h'] at hb)
  all_goals subst ha
  all_goals first
    | (exact sgnK_neg (by linarith))
    | (exact sgnK_pos (by linarith))
    | (exact sgnK_zero_iff.mpr (by linarith))
    | (cases hb)

/-- The derivative rule of a Thom family: each member's derivative is a constant or an earlier
member. -/
def ThomFamily (fs : List (ℝ → ℝ)) : Prop :=
  ∀ i (hi : i < fs.length), Differentiable ℝ fs[i] ∧
    ((∃ k : ℝ, ∀ s, deriv fs[i] s = k) ∨ ∃ j, ∃ hj : j < i, deriv fs[i] = fs[j])

/-- The points of `ℝ` where the first `n` members have the signs `σ`. -/
def signSet (fs : List (ℝ → ℝ)) (σ : ℕ → Sgn) (n : ℕ) : Set ℝ :=
  {s | ∀ i (hi : i < fs.length), i < n → sgnK (fs[i] s) = σ i}

/-- A function whose derivative has one sign on an interval is monotone or antitone there, so its
sign classes there are intervals. -/
theorem ordConnected_sign_of_deriv {A : Set ℝ} (hA : A.OrdConnected) {f : ℝ → ℝ}
    (hf : Differentiable ℝ f) {τ : Sgn} (hτ : ∀ s ∈ A, sgnK (deriv f s) = τ) (σ : Sgn) :
    (A ∩ {s | sgnK (f s) = σ}).OrdConnected := by
  have hconv : Convex ℝ A := hA.convex
  have hint : ∀ s ∈ interior A, s ∈ A := fun s hs => interior_subset hs
  constructor
  intro x hx y hy z hz
  have hzA : z ∈ A := hA.out hx.1 hy.1 hz
  refine ⟨hzA, ?_⟩
  have mono_or : MonotoneOn f A ∨ AntitoneOn f A := by
    rcases τ with _ | _ | _
    · right
      apply antitoneOn_of_deriv_nonpos hconv hf.continuous.continuousOn
        (hf.differentiableOn.mono (subset_univ _))
      intro s hs
      have := hτ s (hint s hs)
      rcases lt_trichotomy (deriv f s) 0 with h | h | h
      · exact h.le
      · exact h.le
      · rw [sgnK_pos h] at this; cases this
    · left
      apply monotoneOn_of_deriv_nonneg hconv hf.continuous.continuousOn
        (hf.differentiableOn.mono (subset_univ _))
      intro s hs
      have := hτ s (hint s hs)
      exact (sgnK_zero_iff.mp this).symm.le
    · left
      apply monotoneOn_of_deriv_nonneg hconv hf.continuous.continuousOn
        (hf.differentiableOn.mono (subset_univ _))
      intro s hs
      have := hτ s (hint s hs)
      rcases lt_trichotomy (deriv f s) 0 with h | h | h
      · rw [sgnK_neg h] at this; cases this
      · exact h.ge
      · exact h.le
  rcases mono_or with hm | hm
  · exact sgnK_between hx.2 hy.2 (hm hx.1 hzA hz.1) (hm hzA hy.1 hz.2)
  · have h1 := hm hx.1 hzA hz.1
    have h2 := hm hzA hy.1 hz.2
    exact sgnK_between hy.2 hx.2 h2 h1

/-- **Thom's lemma.** In a family closed under derivation (each derivative a constant or an earlier
member), every sign condition on the first `n` members is an interval. -/
theorem thom {fs : List (ℝ → ℝ)} (hfs : ThomFamily fs) (σ : ℕ → Sgn) :
    ∀ n, (signSet fs σ n).OrdConnected := by
  intro n
  induction n with
  | zero =>
    have : signSet fs σ 0 = univ := by ext s; simp [signSet]
    rw [this]; exact ordConnected_univ
  | succ n ih =>
    by_cases hn : n < fs.length
    · have heq : signSet fs σ (n + 1) = signSet fs σ n ∩ {s | sgnK (fs[n] s) = σ n} := by
        ext s
        simp only [signSet, mem_ofPred_eq, mem_inter_iff]
        constructor
        · intro h
          exact ⟨fun i hi hin => h i hi (by omega), h n hn (by omega)⟩
        · rintro ⟨h, h'⟩ i hi hin
          rcases Nat.lt_succ_iff_lt_or_eq.mp hin with h1 | h1
          · exact h i hi h1
          · subst h1; exact h'
      rw [heq]
      obtain ⟨hd, hder⟩ := hfs n hn
      rcases hder with ⟨k, hk⟩ | ⟨j, hj, hjd⟩
      · exact ordConnected_sign_of_deriv ih hd (τ := sgnK k) (fun s _ => by rw [hk]) (σ n)
      · refine ordConnected_sign_of_deriv ih hd (τ := σ j) (fun s hs => ?_) (σ n)
        rw [hjd]
        exact hs j (lt_trans hj hn) hj
    · have heq : signSet fs σ (n + 1) = signSet fs σ n := by
        ext s
        simp only [signSet, mem_ofPred_eq]
        constructor
        · exact fun h i hi hin => h i hi (by omega)
        · exact fun h i hi hin => h i hi (by omega)
      rw [heq]; exact ih

/-- **Merging root functions.** Continuous functions on a preconnected `C`, any two equal
everywhere on `C` or nowhere, are a strictly ordered family: `ρ 0 < … < ρ (m-1)` at every point of
`C`, with each function equal to one `ρ k` on `C` and each `ρ k` one of the functions. -/
theorem merge {T : Type*} [TopologicalSpace T] {C : Set T} (hC : IsPreconnected C) {ι : Type*}
    [Fintype ι] (g : ι → T → ℝ) (hg : ∀ i, ContinuousOn (g i) C)
    (hed : ∀ i j, (∀ t ∈ C, g i t = g j t) ∨ ∀ t ∈ C, g i t ≠ g j t) :
    ∃ m, ∃ ρ : Fin m → T → ℝ, ∃ idx : ι → Fin m, (∀ k, ContinuousOn (ρ k) C) ∧
      (∀ t ∈ C, StrictMono fun k => ρ k t) ∧ (∀ i, ∀ t ∈ C, g i t = ρ (idx i) t) ∧
      ∀ k, ∃ i, idx i = k := by
  classical
  rcases C.eq_empty_or_nonempty with hC0 | ⟨t₁, ht₁⟩
  · let e := Fintype.equivFin ι
    exact ⟨Fintype.card ι, fun k => g (e.symm k), e, fun k => by simp [hC0], by simp [hC0],
      by simp [hC0], fun k => ⟨e.symm k, by simp⟩⟩
  -- the order of two functions on `C` is their order at `t₁`
  have hlt : ∀ i j, g i t₁ < g j t₁ → ∀ t ∈ C, g i t < g j t := by
    intro i j hij t ht
    rcases hed i j with h | h
    · exact absurd (h t₁ ht₁) hij.ne
    · have := pos_const (g := fun t => g j t - g i t) hC ((hg j).sub (hg i))
        (fun t ht h0 => h t ht (by linarith)) ht₁ ht (by linarith)
      linarith
  have heq : ∀ i j, g i t₁ = g j t₁ → ∀ t ∈ C, g i t = g j t := by
    intro i j hij t ht
    rcases hed i j with h | h
    · exact h t ht
    · exact absurd hij (h t₁ ht₁)
  set Vs := Finset.univ.image fun i => g i t₁
  set m := Vs.card
  set emb := Vs.orderEmbOfFin rfl
  have hmem : ∀ k, emb k ∈ Vs := fun k => Finset.orderEmbOfFin_mem Vs rfl k
  have hsel : ∀ k, ∃ i, g i t₁ = emb k := fun k => by
    obtain ⟨i, -, hi⟩ := Finset.mem_image.mp (hmem k); exact ⟨i, hi⟩
  choose sel hsel using hsel
  have hidx : ∀ i, ∃ k, emb k = g i t₁ := fun i => by
    have : g i t₁ ∈ (Vs : Set ℝ) := by simp [Vs]
    rw [← Finset.range_orderEmbOfFin Vs rfl] at this
    exact this
  choose idx hidx using hidx
  refine ⟨m, fun k => g (sel k), idx, fun k => hg (sel k), fun t ht a b hab => ?_,
    fun i t ht => ?_, fun k => ⟨sel k, ?_⟩⟩
  · apply hlt (sel a) (sel b) _ t ht
    rw [hsel, hsel]; exact emb.strictMono hab
  · exact heq i (sel (idx i)) (by rw [hsel, hidx]) t ht
  · apply emb.injective
    rw [hidx, hsel]

end ForgeCurved.Thom
