import ForgeCurved.Sides
import Mathlib.Analysis.Calculus.ContDiff.Defs

/-!
# The sweep's statements for `qsampler_covers` (dol-curved, curved-l2-plan.md module 5)

dol-curved's sampler sweeps planes `ℓ x = c` through an open slab `a < ℓ x < b` that holds no
critical point. It needs two facts, stated here for carriers on any real normed space `V` with a
continuous linear sweep `ℓ`. With `V = ℝ³` the sweep is `d·x`. The plane version, on a sweep
plane with a direction `e` in it, is the instance with `V = ℝ²` and the carriers restricted.

* `ZeroGraphs ℓ ks a b`: the common zeros of `ks` in the slab are finitely many disjoint graphs
  `γ i : (a, b) → V`, `ℓ (γ i t) = t`. These are the curves `Z(c_j) ∩ Z(c_k)` in ℝ³, or the
  curves `Z(c)` in a plane.
* `CrossesAll cs ℓ a b`: every lower-dimensional cell that meets the slab meets every sweep plane
  in it (dol-curved's statement, verbatim up to `ℓ`).

The critical set, decided exactly by the sampler:
* `Transverse ℓ ks x`: `v ↦ (ℓ v, Dc v for c ∈ ks)` is onto. For one carrier in ℝ³ this says
  `∇c ∦ d`. For two it says `∇c_j, ∇c_k, d` are independent, which includes `∇c_j ∦ ∇c_k`;
* `NoTriple`: no three carriers vanish at a point of the slab (no vertex);
* **`Proper`** (added here): the common zeros over each compact `[a', b'] ⊂ (a, b)` are bounded.
  Without it the graph lemma is false. The hyperbola `xy = 1` in the plane `z = 0` is transverse
  to `ℓ = x` everywhere, but over a slab containing `x = 0` its branches are graphs over part of
  `(a, b)` only. For quadrics this is a condition on the leading forms: no asymptotic direction
  of the curve in a level plane. In the CAD it is "the leading coefficient is a nonzero constant".

Proven here: **`crossesAll_edges`**, the edge half of `CrossesAll` from `ZeroGraphs` and
`NoTriple`. Along each graph the other carriers never vanish, so their signs are constant. The
graph lemma itself and the facet half are stated as `GraphLemma` and `FacetCrossing` (see the
README).
-/

namespace ForgeCurved.Sweep

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Set

variable {V : Type} [NormedAddCommGroup V] [NormedSpace ℝ V]

/-- The open slab between two sweep levels. -/
def Slab (ℓ : V →L[ℝ] ℝ) (a b : ℝ) : Set V := {x | a < ℓ x ∧ ℓ x < b}

/-- Transversality to the sweep at `x`: `v ↦ (ℓ v, (Dc v)_{c ∈ ks})` is onto `ℝ × ℝ^ks`. -/
def Transverse (ℓ : V →L[ℝ] ℝ) (ks : List (V → ℝ)) (x : V) : Prop :=
  Function.Surjective fun v : V => (ℓ v, fun i : Fin ks.length => fderiv ℝ ks[i] x v)

/-- The common zeros of `ks` in the slab are finitely many disjoint continuous graphs over the
sweep. -/
def ZeroGraphs (ℓ : V →L[ℝ] ℝ) (ks : List (V → ℝ)) (a b : ℝ) : Prop :=
  ∃ n, ∃ γ : Fin n → ℝ → V, (∀ i, ContinuousOn (γ i) (Ioo a b)) ∧
    (∀ i, ∀ t ∈ Ioo a b, ℓ (γ i t) = t ∧ ∀ c ∈ ks, c (γ i t) = 0) ∧
    (∀ i j, ∀ t ∈ Ioo a b, γ i t = γ j t → i = j) ∧
    ∀ x ∈ Slab ℓ a b, (∀ c ∈ ks, c x = 0) → ∃ i, γ i (ℓ x) = x

/-- No three carriers vanish at one point of the slab. -/
def NoTriple (ℓ : V →L[ℝ] ℝ) (cs : List (V → ℝ)) (a b : ℝ) : Prop :=
  ∀ x ∈ Slab ℓ a b, ∀ i j k (hi : i < cs.length) (hj : j < cs.length) (hk : k < cs.length),
    i ≠ j → j ≠ k → i ≠ k → ¬(cs[i] x = 0 ∧ cs[j] x = 0 ∧ cs[k] x = 0)

/-- The common zeros of `ks` over compact sub-slabs are bounded. -/
def Proper (ℓ : V →L[ℝ] ℝ) (ks : List (V → ℝ)) (a b : ℝ) : Prop :=
  ∀ a' b', a < a' → b' < b → Bornology.IsBounded {x | a' ≤ ℓ x ∧ ℓ x ≤ b' ∧ ∀ c ∈ ks, c x = 0}

/-- No critical point in the slab, for the pairs (curves) and singletons (surfaces) of `cs`. -/
def NoCrit (ℓ : V →L[ℝ] ℝ) (cs : List (V → ℝ)) (a b : ℝ) : Prop :=
  (∀ x ∈ Slab ℓ a b, ∀ c ∈ cs, c x = 0 → Transverse ℓ [c] x) ∧
  (∀ x ∈ Slab ℓ a b, ∀ j k (hj : j < cs.length) (hk : k < cs.length), j ≠ k →
    cs[j] x = 0 → cs[k] x = 0 → Transverse ℓ [cs[j], cs[k]] x) ∧
  NoTriple ℓ cs a b

/-- dol-curved's `CrossesAll`: every lower-dimensional cell meeting the slab meets every sweep
plane in it. -/
def CrossesAll (cs : List (V → ℝ)) (ℓ : V →L[ℝ] ℝ) (a b : ℝ) : Prop :=
  ∀ X, RealisedK cs X → Sgn.zero ∈ X → (∃ x ∈ Slab ℓ a b, covCellK cs X x) →
    ∀ c, a < c → c < b → ∃ x, covCellK cs X x ∧ ℓ x = c

/-- **The graph lemma**, as a statement (open): `C¹` carriers, no critical point, proper ⇒ each
pair's curve is graphs. -/
def GraphLemma (V : Type) [NormedAddCommGroup V] [NormedSpace ℝ V] : Prop :=
  ∀ (ℓ : V →L[ℝ] ℝ) (cs : List (V → ℝ)) (a b : ℝ), (∀ c ∈ cs, ContDiff ℝ 1 c) →
    NoCrit ℓ cs a b → ∀ j k (hj : j < cs.length) (hk : k < cs.length), j ≠ k →
      Proper ℓ [cs[j], cs[k]] a b → ZeroGraphs ℓ [cs[j], cs[k]] a b

/-- **The facet half of `CrossesAll`**, as a statement (open): a facet cell meeting the slab meets
every sweep plane in it. -/
def FacetCrossing (cs : List (V → ℝ)) (ℓ : V →L[ℝ] ℝ) (a b : ℝ) : Prop :=
  ∀ X, RealisedK cs X → X.isFacet = true → (∃ x ∈ Slab ℓ a b, covCellK cs X x) →
    ∀ c, a < c → c < b → ∃ x, covCellK cs X x ∧ ℓ x = c

/-- A continuous function on an interval that never vanishes keeps its sign. -/
theorem sgnK_const_of_ne {g : ℝ → ℝ} {a b : ℝ} (hg : ContinuousOn g (Ioo a b))
    (hne : ∀ t ∈ Ioo a b, g t ≠ 0) {s t : ℝ} (hs : s ∈ Ioo a b) (ht : t ∈ Ioo a b) :
    sgnK (g s) = sgnK (g t) := by
  have hsub : uIcc s t ⊆ Ioo a b := by
    intro z hz
    rcases le_total s t with h | h
    · rw [uIcc_of_le h] at hz; exact ⟨lt_of_lt_of_le hs.1 hz.1, lt_of_le_of_lt hz.2 ht.2⟩
    · rw [uIcc_of_ge h] at hz; exact ⟨lt_of_lt_of_le ht.1 hz.1, lt_of_le_of_lt hz.2 hs.2⟩
  have hivt := intermediate_value_uIcc (hg.mono hsub)
  have hs0 := hne s hs
  have ht0 := hne t ht
  rcases hs0.lt_or_gt with h1 | h1 <;> rcases ht0.lt_or_gt with h2 | h2
  · rw [sgnK_neg h1, sgnK_neg h2]
  · obtain ⟨z, hz, hz0⟩ := hivt (show (0 : ℝ) ∈ uIcc (g s) (g t) from
      ⟨by simp; left; exact h1.le, by simp; right; exact h2.le⟩)
    exact absurd hz0 (hne z (hsub hz))
  · obtain ⟨z, hz, hz0⟩ := hivt (show (0 : ℝ) ∈ uIcc (g s) (g t) from
      ⟨by simp; right; exact h2.le, by simp; left; exact h1.le⟩)
    exact absurd hz0 (hne z (hsub hz))
  · rw [sgnK_pos h1, sgnK_pos h2]

/-- **The edge half of `CrossesAll`.** If every pair's common zeros are graphs over the slab and no
three carriers meet in it, every cell with two zeros that meets the slab meets every sweep plane
in it. -/
theorem crossesAll_edges {cs : List (V → ℝ)} {ℓ : V →L[ℝ] ℝ} {a b : ℝ}
    (hcont : ∀ c ∈ cs, Continuous c)
    (hg : ∀ j k (hj : j < cs.length) (hk : k < cs.length), j ≠ k → ZeroGraphs ℓ [cs[j], cs[k]] a b)
    (h3 : NoTriple ℓ cs a b) (X : Cov) {j k : ℕ} (hjk : j ≠ k)
    (hXj : X[j]? = some Sgn.zero) (hXk : X[k]? = some Sgn.zero)
    (hx : ∃ x ∈ Slab ℓ a b, covCellK cs X x) :
    ∀ c, a < c → c < b → ∃ x, covCellK cs X x ∧ ℓ x = c := by
  obtain ⟨x, hxS, hxX⟩ := hx
  have hxX' : covOfK cs x = X := hxX
  -- the two zero positions are carriers, vanishing at `x`
  have hzero : ∀ m, X[m]? = some Sgn.zero → ∃ hm : m < cs.length, cs[m] x = 0 := by
    intro m hm
    rw [← hxX', getElem?_covOfK] at hm
    rcases h : cs[m]? with _ | c
    · rw [h] at hm; cases hm
    · rw [h] at hm
      have hlt : m < cs.length := (List.getElem?_eq_some_iff.mp h).1
      refine ⟨hlt, ?_⟩
      have hc : cs[m] = c := (List.getElem?_eq_some_iff.mp h).2
      rw [hc]
      exact sgnK_zero_iff.mp (Option.some.inj hm)
  obtain ⟨hj, hxj⟩ := hzero j hXj
  obtain ⟨hk, hxk⟩ := hzero k hXk
  obtain ⟨n, γ, hγc, hγ, -, hcover⟩ := hg j k hj hk hjk
  obtain ⟨i, hi⟩ := hcover x hxS (by simp [hxj, hxk])
  have hℓx : ℓ x ∈ Ioo a b := hxS
  intro c hac hcb
  have hc : c ∈ Ioo a b := ⟨hac, hcb⟩
  refine ⟨γ i c, ?_, (hγ i c hc).1⟩
  show covOfK cs (γ i c) = X
  rw [← hxX']
  -- every carrier keeps its sign along the graph
  apply List.ext_getElem (by simp [covOfK])
  intro m hm1 hm2
  simp only [covOfK, List.getElem_map]
  have hm : m < cs.length := by simpa [covOfK] using hm1
  have hon : ∀ t ∈ Ioo a b, cs[j] (γ i t) = 0 ∧ cs[k] (γ i t) = 0 := fun t ht => by
    have := (hγ i t ht).2; simpa using this
  by_cases hmj : m = j
  · subst hmj; rw [(hon c hc).1, hxj]
  by_cases hmk : m = k
  · subst hmk; rw [(hon c hc).2, hxk]
  have hne : ∀ t ∈ Ioo a b, cs[m] (γ i t) ≠ 0 := fun t ht h0 => by
    have hS : γ i t ∈ Slab ℓ a b := by
      show a < ℓ (γ i t) ∧ ℓ (γ i t) < b
      rw [(hγ i t ht).1]; exact ht
    exact h3 _ hS j k m hj hk hm hjk (Ne.symm hmk) (Ne.symm hmj) ⟨(hon t ht).1, (hon t ht).2, h0⟩
  have hgc : ContinuousOn (fun t => cs[m] (γ i t)) (Ioo a b) :=
    (hcont _ (List.getElem_mem hm)).comp_continuousOn (hγc i)
  have := sgnK_const_of_ne hgc hne hc hℓx
  rw [this, hi]

end ForgeCurved.Sweep
