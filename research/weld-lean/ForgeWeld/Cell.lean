import ForgeWeld.Extreme

/-!
# Where the long point is, over a convex cell (L3, step 2)

The section point `(u, v)`; each cut's crossing an affine function `f` of it (`Aff2`); a convex
cell of the section, bounded, as the points where each of its edge functions `g` is `≥ 0`. The
reach is the minimum of the `f`s. `cell_max`: every point of the cell is matched or beaten, in the
reach, by a candidate of the cell:

* a **corner**: two edges' lines meeting (`g₁ = g₂ = 0`, not parallel);
* a **ridge on an edge**: two cuts equal on an edge's line (`f₁ = f₂`, `g = 0`, the ridge not
  parallel to the edge);
* an **apex**: three cuts equal at one point (`f₁ = f₂ = f₃`, two ridges not parallel).

These are forge_weld::frame `extreme`'s candidates on a cell (its loops' corners, its ridges' meetings
with the loops, its apexes), each a single point (the determinants it divides by are the ones
here, not zero).

Proof: three moves, each `move`, stopping at the first edge or the first cut to come down to the
active one: along the active cut's level line; then along the edge it stopped on, or along the
ridge it stopped on. The reach never falls.
-/

namespace ForgeWeld.Extreme

/-- An affine function of the section point. -/
structure Aff2 where
  c : Rat
  a : Rat
  b : Rat

def Aff2.at (f : Aff2) (p : Rat × Rat) : Rat := f.c + f.a * p.1 + f.b * p.2

/-- Its rate along a direction. -/
def Aff2.slope (f : Aff2) (δ : Rat × Rat) : Rat := f.a * δ.1 + f.b * δ.2

/-- Two functions' gradients are not parallel. -/
def det (f g : Aff2) : Rat := f.a * g.b - f.b * g.a

def Aff2.sub (f g : Aff2) : Aff2 := ⟨f.c - g.c, f.a - g.a, f.b - g.b⟩

def along (p δ : Rat × Rat) (t : Rat) : Rat × Rat := (p.1 + t * δ.1, p.2 + t * δ.2)

/-- `f` on the line from `p` along `δ`. -/
def Aff2.on (f : Aff2) (p δ : Rat × Rat) : Aff1 := ⟨f.at p, f.slope δ⟩

@[simp] theorem on_at (f : Aff2) (p δ : Rat × Rat) (t : Rat) :
    (f.on p δ).at t = f.at (along p δ t) := by
  simp only [Aff2.on, Aff1.at, Aff2.at, Aff2.slope, along]; grind

theorem at_along (f : Aff2) (p δ : Rat × Rat) (t : Rat) :
    f.at (along p δ t) = f.at p + t * f.slope δ := by
  simp only [Aff2.at, Aff2.slope, along]; grind

/-- An affine function of `t` no less than zero at `0` and at `t₁` is so between. -/
theorem between_nonneg (h : Aff1) {s t : Rat} (h0 : 0 ≤ h.at 0) (h1 : 0 ≤ h.at t) (hs : 0 ≤ s)
    (hst : s ≤ t) : 0 ≤ h.at s := by
  simp only [Aff1.at] at *
  by_cases ht : t = 0
  · subst ht; have : s = 0 := by grind
    subst this; grind
  · have hpos : 0 < t := by grind
    have key : t * (h.α + h.β * s) = (t - s) * (h.α + h.β * 0) + s * (h.α + h.β * t) := by grind
    have p1 : 0 ≤ (t - s) * (h.α + h.β * 0) := Rat.mul_nonneg (by grind) h0
    have p2 : 0 ≤ s * (h.α + h.β * t) := Rat.mul_nonneg hs h1
    apply Classical.byContradiction; intro hn
    have : t * (h.α + h.β * s) < 0 := mul_neg' hpos (by grind)
    grind

/-- **A move.** From a point of the cell where `k` is the least cut, along `δ` (which `k` does not
fall along, and which leaves the cell: some edge falls along it), to the first edge reached or the
first cut to come down to `k`: still in the cell, `k` still the least, no lower, and stopped by an
edge falling along `δ` or a cut whose rate along `δ` differs from `k`'s. -/
theorem move (F G : List Aff2) (p δ : Rat × Rat) (k : Aff2)
    (hfeas : ∀ g ∈ G, 0 ≤ g.at p) (hmin : ∀ f ∈ F, k.at p ≤ f.at p) (hrise : 0 ≤ k.slope δ)
    (hbound : ∃ g ∈ G, g.slope δ < 0) :
    ∃ t, 0 ≤ t ∧ (∀ g ∈ G, 0 ≤ g.at (along p δ t)) ∧
      (∀ f ∈ F, k.at (along p δ t) ≤ f.at (along p δ t)) ∧ k.at p ≤ k.at (along p δ t) ∧
      ((∃ g ∈ G, g.at (along p δ t) = 0 ∧ g.slope δ ≠ 0) ∨
        ∃ j ∈ F, j.at (along p δ t) = k.at (along p δ t) ∧ j.slope δ ≠ k.slope δ) := by
  obtain ⟨g₀, hg₀, hs₀⟩ := hbound
  -- where g₀ reaches zero: t₀ = g₀(p) / -(slope) ≥ 0
  let t₀ := g₀.at p / -(g₀.slope δ)
  have hneg : 0 < -(g₀.slope δ) := by grind
  have hmul : -(g₀.slope δ) * t₀ = g₀.at p := mdc _ _ (by grind)
  have ht₀ : 0 ≤ t₀ := by
    apply Classical.byContradiction; intro hn
    have := mul_neg' hneg (show t₀ < 0 by grind)
    have := hfeas g₀ hg₀
    grind
  have hroot : g₀.at (along p δ t₀) = 0 := by
    rw [at_along]; grind
  -- the first edge reached: stop_right with the zero function against the edges
  let z : Aff1 := ⟨0, 0⟩
  obtain ⟨t₁, h1, h2, h3, h4⟩ := stop_right z ht₀ (G.map fun g => g.on p δ) (fun j hj => by
    obtain ⟨g, hg, rfl⟩ := List.mem_map.mp hj
    have := hfeas g hg
    simp only [z, Aff2.on, Aff1.at, Aff2.at, Aff2.slope, along] at *; grind)
  have edge : ∃ g ∈ G, g.at (along p δ t₁) = 0 ∧ g.slope δ ≠ 0 := by
    rcases h4 with h4 | ⟨j, hj, hc⟩
    · exact ⟨g₀, hg₀, by rw [h4]; exact hroot, by grind⟩
    · obtain ⟨g, hg, rfl⟩ := List.mem_map.mp hj
      refine ⟨g, hg, ?_, ?_⟩
      · have := hc.2; simp only [z, Aff2.on, Aff1.at, Aff2.at, Aff2.slope, along] at this ⊢; grind
      · have := hc.1; simp only [Aff2.on, z] at this; exact this
  have gnn : ∀ g ∈ G, 0 ≤ g.at (along p δ t₁) := fun g hg => by
    have := h3 _ (List.mem_map_of_mem hg)
    simp only [z, Aff2.on, Aff1.at, Aff2.at, Aff2.slope, along] at this ⊢; grind
  -- the first cut to come down to k, before that edge
  obtain ⟨t₂, i1, i2, i3, i4⟩ := stop_right (k.on p δ) h1 (F.map fun f => f.on p δ) (fun j hj => by
    obtain ⟨f, hf, rfl⟩ := List.mem_map.mp hj
    have := hmin f hf; simp only [Aff2.on, Aff1.at, Aff2.at, Aff2.slope, along] at *; grind)
  refine ⟨t₂, i1, fun g hg => ?_, fun f hf => ?_, ?_, ?_⟩
  · -- every edge is ≥ 0 at 0 and at t₁, so at t₂ between
    have e0 : 0 ≤ (g.on p δ).at 0 := by have := hfeas g hg; simp only [Aff2.on, Aff1.at, Aff2.at, Aff2.slope, along] at *; grind
    have e1 : 0 ≤ (g.on p δ).at t₁ := by simp only [on_at]; exact gnn g hg
    have := between_nonneg (g.on p δ) e0 e1 (i1) i2
    simpa only [on_at] using this
  · have := i3 _ (List.mem_map_of_mem hf); simpa only [on_at] using this
  · rw [at_along]
    have : 0 ≤ t₂ * k.slope δ := Rat.mul_nonneg (i1) hrise
    grind
  · rcases i4 with i4 | ⟨j, hj, hc⟩
    · subst i4; exact Or.inl edge
    · obtain ⟨f, hf, rfl⟩ := List.mem_map.mp hj
      refine Or.inr ⟨f, hf, ?_, ?_⟩
      · have := hc.2; simpa only [on_at] using this
      · have := hc.1; simp only [Aff2.on] at this; exact this

/-- The direction square to a gradient: its rate there is the determinant. -/
def perp (f : Aff2) : Rat × Rat := (-f.b, f.a)

theorem slope_perp (f g : Aff2) : g.slope (perp f) = det f g := by
  simp only [Aff2.slope, perp, det]; grind

theorem slope_neg (g : Aff2) (δ : Rat × Rat) : g.slope (-δ.1, -δ.2) = -(g.slope δ) := by
  simp only [Aff2.slope]; grind

/-- The candidates of a cell. -/
def Cand2 (F G : List Aff2) (q : Rat × Rat) : Prop :=
  (∃ g₁ ∈ G, ∃ g₂ ∈ G, g₁.at q = 0 ∧ g₂.at q = 0 ∧ det g₁ g₂ ≠ 0) ∨
  (∃ f₁ ∈ F, ∃ f₂ ∈ F, ∃ g ∈ G, f₁.at q = f₂.at q ∧ g.at q = 0 ∧ det (f₁.sub f₂) g ≠ 0) ∨
  (∃ f₁ ∈ F, ∃ f₂ ∈ F, ∃ f₃ ∈ F, f₁.at q = f₂.at q ∧ f₁.at q = f₃.at q ∧
    det (f₁.sub f₂) (f₁.sub f₃) ≠ 0)

theorem slope_sub (f g : Aff2) (δ : Rat × Rat) : (f.sub g).slope δ = f.slope δ - g.slope δ := by
  simp only [Aff2.slope, Aff2.sub]; grind

theorem at_sub (f g : Aff2) (q : Rat × Rat) : (f.sub g).at q = f.at q - g.at q := by
  simp only [Aff2.at, Aff2.sub]; grind

/-- `move`, in whichever sense of `d` the active cut does not fall: what has no rate along `d`
keeps its value, and rates along it are compared as along `d`. -/
theorem move' (F G : List Aff2) (p d : Rat × Rat) (hd : d ≠ (0, 0)) (k : Aff2)
    (bounded : ∀ δ : Rat × Rat, δ ≠ (0, 0) → ∃ g ∈ G, g.slope δ < 0)
    (hfeas : ∀ g ∈ G, 0 ≤ g.at p) (hmin : ∀ f ∈ F, k.at p ≤ f.at p) :
    ∃ q, (∀ g ∈ G, 0 ≤ g.at q) ∧ (∀ f ∈ F, k.at q ≤ f.at q) ∧ k.at p ≤ k.at q ∧
      (∀ h : Aff2, h.slope d = 0 → h.at q = h.at p) ∧
      ((∃ g ∈ G, g.at q = 0 ∧ g.slope d ≠ 0) ∨
        ∃ j ∈ F, j.at q = k.at q ∧ j.slope d ≠ k.slope d) := by
  by_cases hs : 0 ≤ k.slope d
  · obtain ⟨t, _, h1, h2, h3, h4⟩ := move F G p d k hfeas hmin hs (bounded d hd)
    refine ⟨along p d t, h1, h2, h3, fun h hh => by rw [at_along, hh]; grind, h4⟩
  · let δ : Rat × Rat := (-d.1, -d.2)
    have hδ : δ ≠ (0, 0) := by
      intro h; apply hd; simp only [δ, Prod.mk.injEq] at h; ext <;> grind
    have hr : 0 ≤ k.slope δ := by rw [slope_neg]; grind
    obtain ⟨t, _, h1, h2, h3, h4⟩ := move F G p δ k hfeas hmin hr (bounded δ hδ)
    refine ⟨along p δ t, h1, h2, h3, fun h hh => by rw [at_along, slope_neg, hh]; grind, ?_⟩
    rcases h4 with ⟨g, hg, e, ne⟩ | ⟨j, hj, e, ne⟩
    · exact Or.inl ⟨g, hg, e, by rw [slope_neg] at ne; grind⟩
    · exact Or.inr ⟨j, hj, e, by rw [slope_neg, slope_neg] at ne; grind⟩

theorem perp_ne (f : Aff2) (h : (f.a, f.b) ≠ (0, 0)) : perp f ≠ (0, 0) := by
  intro e; apply h; simp only [perp, Prod.mk.injEq] at e ⊢; grind

theorem grad_ne_of_slope (f : Aff2) {d : Rat × Rat} (h : f.slope d ≠ 0) : (f.a, f.b) ≠ (0, 0) := by
  intro e; apply h; simp only [Prod.mk.injEq] at e; simp only [Aff2.slope, e.1, e.2]; grind

/-- **The reach over a convex cell is attained at a candidate.** A bounded convex cell (every
direction leaves it: some edge falls along it), a point `p` of it, and `k` the least cut there:
some candidate of the cell (`Cand2`) has every cut at least `k`'s value at `p`. So the maximum over
the cell of the minimum of the cuts is the maximum over its candidates. -/
theorem cell_max (F G : List Aff2)
    (bounded : ∀ δ : Rat × Rat, δ ≠ (0, 0) → ∃ g ∈ G, g.slope δ < 0)
    {p : Rat × Rat} (hp : ∀ g ∈ G, 0 ≤ g.at p) {k : Aff2} (hk : k ∈ F)
    (hmin : ∀ f ∈ F, k.at p ≤ f.at p) :
    ∃ q, (∀ g ∈ G, 0 ≤ g.at q) ∧ (∀ f ∈ F, k.at p ≤ f.at q) ∧ Cand2 F G q := by
  -- first move: along k's level line (any line, if k is flat)
  let d₁ : Rat × Rat := if (k.a, k.b) = (0, 0) then (1, 0) else perp k
  have hd₁ : d₁ ≠ (0, 0) := by
    simp only [d₁]; split
    · simp
    · exact perp_ne k (by assumption)
  have hk₁ : k.slope d₁ = 0 := by
    simp only [d₁]; split
    · rename_i h; simp only [Prod.mk.injEq] at h; simp only [Aff2.slope, h.1, h.2]; grind
    · rw [slope_perp]; simp only [det]; grind
  obtain ⟨q₁, f1, m1, r1, _, s1⟩ := move' F G p d₁ hd₁ k bounded hp hmin
  rcases s1 with ⟨g, hg, g0, gs⟩ | ⟨j, hj, je, js⟩
  · -- on an edge: move along it
    have hd₂ := perp_ne g (grad_ne_of_slope g gs)
    obtain ⟨q₂, f2, m2, r2, c2, s2⟩ := move' F G q₁ (perp g) hd₂ k bounded f1 m1
    have gq : g.at q₂ = 0 := by rw [c2 g (by rw [slope_perp]; simp only [det]; grind)]; exact g0
    refine ⟨q₂, f2, fun f hf => Rat.le_trans r1 (Rat.le_trans r2 (m2 f hf)), ?_⟩
    rcases s2 with ⟨g', hg', g'0, g's⟩ | ⟨j, hj, je, js⟩
    · exact Or.inl ⟨g, hg, g', hg', gq, g'0, by rw [slope_perp] at g's; exact g's⟩
    · refine Or.inr (Or.inl ⟨j, hj, k, hk, g, hg, je, gq, ?_⟩)
      rw [slope_perp, slope_perp] at js
      simp only [det, Aff2.sub] at js ⊢; grind
  · -- on a ridge of j and k: move along it
    have hu : (j.sub k).slope d₁ ≠ 0 := by rw [slope_sub]; grind
    have hd₂ := perp_ne (j.sub k) (grad_ne_of_slope _ hu)
    obtain ⟨q₂, f2, m2, r2, c2, s2⟩ := move' F G q₁ (perp (j.sub k)) hd₂ k bounded f1 m1
    have ridge : j.at q₂ = k.at q₂ := by
      have := c2 (j.sub k) (by rw [slope_perp]; simp only [det]; grind)
      rw [at_sub, at_sub] at this; grind
    refine ⟨q₂, f2, fun f hf => Rat.le_trans r1 (Rat.le_trans r2 (m2 f hf)), ?_⟩
    rcases s2 with ⟨g, hg, g0, gs⟩ | ⟨j', hj', je', js'⟩
    · exact Or.inr (Or.inl ⟨j, hj, k, hk, g, hg, ridge, g0, by rw [slope_perp] at gs; exact gs⟩)
    · refine Or.inr (Or.inr ⟨k, hk, j, hj, j', hj', ridge.symm, je'.symm, ?_⟩)
      rw [slope_perp, slope_perp] at js'
      simp only [det, Aff2.sub] at js' ⊢; grind

end ForgeWeld.Extreme
