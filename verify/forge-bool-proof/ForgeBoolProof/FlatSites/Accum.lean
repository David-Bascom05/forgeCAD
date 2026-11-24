import Mathlib.Topology.MetricSpace.Pseudo.Basic
import Mathlib.Analysis.SpecificLimits.Basic

/-!
# The δ-accumulation lemma: how far tolerance merges carry an entity (flat G11, class T)

research/decide-once/flat-g11.md classes eleven of the 24 busiest flat tolerance sites **T**: a
decision that accepts a point, a cut or a vertex as lying on a stored entity when it is within that
entity's tolerance (`on_edge`'s `d <= edge.tol`, boolean.rs:289 and every site that calls it; the
vertex merges of boolean.rs:2911 and 5019 within `vx.tol`; nary.rs:1514 within
`max(vx.tol, c.tol)`; trim's shallow pair of boolean.rs:3337 left to the dedup within the larger
tolerance). Each is correct under δ-Faithful (`Statement.lean`, `BodyModel.WithinH`) when what it
accepts stays within δ of the exact meet. This file proves the bound those sites cite:

* `accept`: a point within `τ` of a stored entity that is itself within `ε` of its exact meet is
  within `τ + ε` of that meet (one accept);
* `accept_max`: the same with two tolerances, the larger taken (nary.rs:1514);
* `chain`: points merged in a chain, each step within its own tolerance, end within the sum of the
  steps of where they began;
* `accum`: an entity rebuilt through `n` merges, step `k` within `τ k` of the one before (both
  ways: Hausdorff), is within `ε + ∑ τ k` of the exact meet the first was within `ε` of;
* `accum_within`: so within `δ` whenever `ε + ∑ τ k ≤ δ`, the budget each T site must show.

`Near δ A B` is `BodyModel.WithinH δ A B` written out (each point of either set within `δ` of the
other): stated here over any pseudometric space, with Mathlib alone, so the lemmas build without the
body layer; `WithinH` unfolds to it definitionally.

What the kernel does with the sum (drift.md §4, train 6's drift tolerances): an entity's recorded
tolerance is the deviation measured where it was merged, so the per-entity bound is the running
sum, `δ_out = δ_in + tol` (`near_step`). Whether every result's sums stay within the one δ of the
statement is not proven here: that is the budget, per site (`accum_within`'s hypothesis), and it is
where a T site's argument still owes a number.
-/

namespace ForgeBoolProof.FlatSites.Accum

open Finset

variable {E : Type} [PseudoMetricSpace E]

/-- `BodyModel.WithinH` written out: each point of `A` within `δ` of a point of `B`, and back. -/
def Near (δ : ℝ) (A B : Set E) : Prop :=
  (∀ a ∈ A, ∃ c ∈ B, dist a c ≤ δ) ∧ ∀ c ∈ B, ∃ a ∈ A, dist a c ≤ δ

theorem Near.mono {δ δ' : ℝ} (h : δ ≤ δ') {A B : Set E} (hn : Near δ A B) : Near δ' A B :=
  ⟨fun a ha => let ⟨c, hc, hd⟩ := hn.1 a ha; ⟨c, hc, hd.trans h⟩,
   fun c hc => let ⟨a, ha, hd⟩ := hn.2 c hc; ⟨a, ha, hd.trans h⟩⟩

theorem Near.symm {δ : ℝ} {A B : Set E} (h : Near δ A B) : Near δ B A :=
  ⟨fun c hc => let ⟨a, ha, hd⟩ := h.2 c hc; ⟨a, ha, by rw [dist_comm]; exact hd⟩,
   fun a ha => let ⟨c, hc, hd⟩ := h.1 a ha; ⟨c, hc, by rw [dist_comm]; exact hd⟩⟩

/-- Nearness composes: the tolerances add. -/
theorem Near.trans {ε τ : ℝ} {A B C : Set E} (h1 : Near ε A B) (h2 : Near τ B C) :
    Near (ε + τ) A C := by
  constructor
  · intro a ha
    obtain ⟨b, hb, hab⟩ := h1.1 a ha
    obtain ⟨c, hc, hbc⟩ := h2.1 b hb
    exact ⟨c, hc, (dist_triangle a b c).trans (add_le_add hab hbc)⟩
  · intro c hc
    obtain ⟨b, hb, hbc⟩ := h2.2 c hc
    obtain ⟨a, ha, hab⟩ := h1.2 b hb
    exact ⟨a, ha, (dist_triangle a b c).trans (add_le_add hab hbc)⟩

theorem near_refl (A : Set E) : Near 0 A A :=
  ⟨fun a ha => ⟨a, ha, by simp⟩, fun c hc => ⟨c, hc, by simp⟩⟩

/-- **One accept** (`on_edge`'s `d <= edge.tol`): a point within `τ` of a point of the stored
entity `A`, which lies within `ε` of its exact meet `M`, is within `τ + ε` of `M`. -/
theorem accept {ε τ : ℝ} {A M : Set E} {p s : E} (hs : s ∈ A) (hps : dist p s ≤ τ)
    (h : Near ε A M) : ∃ m ∈ M, dist p m ≤ τ + ε := by
  obtain ⟨m, hm, hsm⟩ := h.1 s hs
  exact ⟨m, hm, (dist_triangle p s m).trans (add_le_add hps hsm)⟩

/-- **An accept within the larger of two tolerances** (nary.rs:1514: a vertex and a cut within
`max(vx.tol, c.tol)` are one): the point is within `max τ₁ τ₂ + ε` of the exact meet. -/
theorem accept_max {ε τ₁ τ₂ : ℝ} {A M : Set E} {p s : E} (hs : s ∈ A)
    (hps : dist p s ≤ max τ₁ τ₂) (h : Near ε A M) : ∃ m ∈ M, dist p m ≤ max τ₁ τ₂ + ε :=
  accept hs hps h

/-- **A chain of merges** (a vertex merged into one, then that into another, ...): `n` steps, step
`k` within `τ k`, end within `∑ τ k` of where they began. -/
theorem chain (p : ℕ → E) (τ : ℕ → ℝ) (n : ℕ)
    (hstep : ∀ k < n, dist (p k) (p (k + 1)) ≤ τ k) :
    dist (p 0) (p n) ≤ ∑ k ∈ range n, τ k :=
  dist_le_range_sum_of_dist_le n (fun hk => hstep _ hk)

/-- **One merge, per entity** (`δ_out = δ_in + tol`): an entity rebuilt within `tol` of one that
was within `δ_in` of the exact meet is within `δ_in + tol` of it. -/
theorem near_step {δ_in tol : ℝ} {S S' M : Set E} (h : Near δ_in S M) (hstep : Near tol S' S) :
    Near (δ_in + tol) S' M := by
  have := hstep.trans h
  rwa [add_comm] at this

/-- **The δ-accumulation lemma.** An entity rebuilt through `n` merges (`S (k + 1)` within `τ k`
of `S k`), the first within `ε` of the exact meet `M`, is within `ε + ∑ τ k` of `M`. -/
theorem accum {ε : ℝ} {τ : ℕ → ℝ} {S : ℕ → Set E} {M : Set E} (h0 : Near ε (S 0) M) :
    ∀ n, (∀ k < n, Near (τ k) (S (k + 1)) (S k)) → Near (ε + ∑ k ∈ range n, τ k) (S n) M := by
  intro n
  induction n with
  | zero => intro _; simpa using h0
  | succ n ih =>
    intro hs
    have hn := ih (fun k hk => hs k (Nat.lt_succ_of_lt hk))
    have := near_step hn (hs n (Nat.lt_succ_self n))
    rwa [sum_range_succ, ← add_assoc]

/-- **Within budget**: the T sites' obligation. If the merges' tolerances and the first entity's
distance sum to at most `δ`, the rebuilt entity is within `δ` of its exact meet. -/
theorem accum_within {δ ε : ℝ} {τ : ℕ → ℝ} {S : ℕ → Set E} {M : Set E} (h0 : Near ε (S 0) M)
    (n : ℕ) (hs : ∀ k < n, Near (τ k) (S (k + 1)) (S k)) (hb : ε + ∑ k ∈ range n, τ k ≤ δ) :
    Near δ (S n) M :=
  (accum h0 n hs).mono hb

end ForgeBoolProof.FlatSites.Accum
