/-!
# Where the long point is: the minimum of affine functions on a line (L3, step 1)

The long point of a member end with several cuts is the maximum over the section of the minimum
of the cuts' crossings, each affine in the section point (forge_weld::frame `extreme`). Its
candidate set is complete by two moves, each on a line: along the active cut's level line to a
ridge or the boundary, then along the ridge to an apex or the boundary. Both moves are this
lemma.

`line_max`: on an interval `[lo, hi]`, the minimum of affine functions `α_k + β_k t` is at most its
value at one of: an end of the interval, or a point of the interval where two of the functions
cross. Proof: take a function `k` active at `t`; move the way it rises (or stays) until the
interval ends or another function comes down to it; the minimum is `k`'s value there, no less
than at `t`.
-/

namespace ForgeWeld.Extreme

theorem mdc (x y : Rat) (hy : y ≠ 0) : y * (x / y) = x := by
  rw [Rat.mul_comm]; exact Rat.div_mul_cancel hy

theorem mul_neg' {a b : Rat} (ha : 0 < a) (hb : b < 0) : a * b < 0 := by
  have := Rat.mul_pos ha (show 0 < -b by grind)
  have e : a * -b = -(a * b) := by grind
  grind

/-- An affine function of one variable. -/
structure Aff1 where
  α : Rat
  β : Rat

def Aff1.at (f : Aff1) (t : Rat) : Rat := f.α + f.β * t

/-- `m` is the minimum of the functions at `t`: no more than any, and one of them. -/
def IsMin (fs : List Aff1) (t m : Rat) : Prop := (∀ f ∈ fs, m ≤ f.at t) ∧ ∃ f ∈ fs, f.at t = m

/-- Where `f` and `g` cross (equal values), when their slopes differ. -/
def Cross (f g : Aff1) (t : Rat) : Prop := f.β ≠ g.β ∧ f.at t = g.at t

/-- A candidate of the interval: an end, or a crossing of two of the functions inside it. -/
def Cand (fs : List Aff1) (lo hi t : Rat) : Prop :=
  lo ≤ t ∧ t ≤ hi ∧ (t = lo ∨ t = hi ∨ ∃ f ∈ fs, ∃ g ∈ fs, Cross f g t)

/-- The first stop moving right from `t` while `k` stays below every `j` in `js` (each `j` with a
smaller slope comes down to `k` at its crossing), capped at `hi`: some stop `t'` with
`t ≤ t' ≤ hi`, where every `j` is still no lower than `k`, and which is `hi` or a crossing. -/
theorem stop_right (k : Aff1) {t hi : Rat} (hth : t ≤ hi) :
    ∀ js : List Aff1, (∀ j ∈ js, k.at t ≤ j.at t) →
      ∃ t', t ≤ t' ∧ t' ≤ hi ∧ (∀ j ∈ js, k.at t' ≤ j.at t') ∧
        (t' = hi ∨ ∃ j ∈ js, Cross j k t')
  | [], _ => ⟨hi, hth, Rat.le_refl, by simp, Or.inl rfl⟩
  | j :: js, h => by
    obtain ⟨t', h1, h2, h3, h4⟩ := stop_right k hth js (fun i hi => h i (List.mem_cons_of_mem _ hi))
    have hj := h j (List.mem_cons_self ..)
    by_cases hs : j.β < k.β
    · -- j comes down to k where they cross: c with k.at c = j.at c, c ≥ t
      let c := (j.α - k.α) / (k.β - j.β)
      have hpos : 0 < k.β - j.β := by grind
      have hc : k.at c = j.at c := by
        simp only [Aff1.at, c]
        have : (k.β - j.β) * ((j.α - k.α) / (k.β - j.β)) = j.α - k.α :=
          mdc _ _ (by grind)
        grind
      have htc : t ≤ c := by
        -- (k.β - j.β)(c - t) = j.at t - k.at t ≥ 0
        have e : (k.β - j.β) * (c - t) = j.at t - k.at t := by
          simp only [Aff1.at, c]
          have : (k.β - j.β) * ((j.α - k.α) / (k.β - j.β)) = j.α - k.α :=
            mdc _ _ (by grind)
          grind
        have : 0 ≤ (k.β - j.β) * (c - t) := by rw [e]; grind
        exact Classical.byContradiction fun hn => by
          have : (k.β - j.β) * (c - t) < 0 := mul_neg' hpos (by grind)
          grind
      -- k stays below j up to c
      have below : ∀ s, s ≤ c → k.at s ≤ j.at s := fun s hs => by
        have e : j.at s - k.at s = (k.β - j.β) * (c - s) := by
          simp only [Aff1.at, c]
          have : (k.β - j.β) * ((j.α - k.α) / (k.β - j.β)) = j.α - k.α :=
            mdc _ _ (by grind)
          grind
        have : 0 ≤ (k.β - j.β) * (c - s) := Rat.mul_nonneg (by grind) (by grind)
        grind
      by_cases hcut : c ≤ t'
      · -- stop at c: every other function is still above k there (it was at t', and c ≤ t')
        refine ⟨c, htc, Rat.le_trans hcut h2, ?_, Or.inr ⟨j, List.mem_cons_self .., by grind, hc.symm⟩⟩
        intro i hi
        rcases List.mem_cons.mp hi with rfl | hi
        · exact below c Rat.le_refl
        · -- i was above k on [t, t']: an affine difference nonnegative at both ends of [t, t']
          have a := h i (List.mem_cons_of_mem _ hi)
          have b := h3 i hi
          simp only [Aff1.at] at a b ⊢
          -- c is between t and t': write c = t + λ(t' - t)
          by_cases ht : t = t'
          · subst ht; have : c = t := by grind
            subst this; grind
          · have hlt : t < t' := Rat.lt_of_le_of_ne h1 ht
            -- the difference d(s) = (i.α - k.α) + (i.β - k.β) s is affine, ≥ 0 at t and t'
            have key : (t' - t) * ((i.α - k.α) + (i.β - k.β) * c) =
                (t' - c) * ((i.α - k.α) + (i.β - k.β) * t) + (c - t) * ((i.α - k.α) + (i.β - k.β) * t') := by
              grind
            have p1 : 0 ≤ (t' - c) * ((i.α - k.α) + (i.β - k.β) * t) := Rat.mul_nonneg (by grind) (by grind)
            have p2 : 0 ≤ (c - t) * ((i.α - k.α) + (i.β - k.β) * t') := Rat.mul_nonneg (by grind) (by grind)
            have hsum : 0 ≤ (t' - t) * ((i.α - k.α) + (i.β - k.β) * c) := by rw [key]; grind
            have : 0 ≤ (i.α - k.α) + (i.β - k.β) * c := by
              apply Classical.byContradiction; intro hn
              have : (t' - t) * ((i.α - k.α) + (i.β - k.β) * c) < 0 :=
                mul_neg' (by grind) (by grind)
              grind
            grind
      · -- j is still above k at t' (t' < c)
        refine ⟨t', h1, h2, ?_, ?_⟩
        · intro i hi
          rcases List.mem_cons.mp hi with rfl | hi
          · exact below t' (by grind)
          · exact h3 i hi
        · rcases h4 with h4 | ⟨i, hi, hc⟩
          · exact Or.inl h4
          · exact Or.inr ⟨i, List.mem_cons_of_mem _ hi, hc⟩
    · -- j's slope is no less than k's: it stays above k going right
      refine ⟨t', h1, h2, ?_, ?_⟩
      · intro i hi
        rcases List.mem_cons.mp hi with rfl | hi
        · simp only [Aff1.at] at hj ⊢
          have : 0 ≤ (i.β - k.β) * (t' - t) := Rat.mul_nonneg (by grind) (by grind)
          grind
        · exact h3 i hi
      · rcases h4 with h4 | ⟨i, hi, hc⟩
        · exact Or.inl h4
        · exact Or.inr ⟨i, List.mem_cons_of_mem _ hi, hc⟩

/-- The function seen from the other way along the line (`t ↦ -t`). -/
def Aff1.flip (f : Aff1) : Aff1 := ⟨f.α, -f.β⟩

@[simp] theorem flip_at (f : Aff1) (t : Rat) : f.flip.at t = f.at (-t) := by
  simp only [Aff1.flip, Aff1.at]; grind

/-- **The minimum of affine functions on an interval is at most its value at a candidate**: an end
of the interval, or a crossing of two of the functions inside it. (`m` the minimum at `t`.) -/
theorem line_max (fs : List Aff1) {lo hi t m : Rat} (hlo : lo ≤ t) (hhi : t ≤ hi)
    (hm : IsMin fs t m) : ∃ t', Cand fs lo hi t' ∧ ∀ f ∈ fs, m ≤ f.at t' := by
  obtain ⟨hle, k, hk, hkm⟩ := hm
  by_cases hβ : 0 ≤ k.β
  · obtain ⟨t', h1, h2, h3, h4⟩ := stop_right k hhi fs (fun j hj => by rw [hkm]; exact hle j hj)
    have rise : m ≤ k.at t' := by
      rw [← hkm]; simp only [Aff1.at]
      have : 0 ≤ k.β * (t' - t) := Rat.mul_nonneg hβ (by grind)
      grind
    refine ⟨t', ⟨Rat.le_trans hlo h1, h2, ?_⟩, fun f hf => Rat.le_trans rise (h3 f hf)⟩
    rcases h4 with h4 | ⟨j, hj, hc⟩
    · exact Or.inr (Or.inl h4)
    · exact Or.inr (Or.inr ⟨j, hj, k, hk, hc⟩)
  · -- k falls going right: move left, as moving right on the flipped line from -t
    obtain ⟨t', h1, h2, h3, h4⟩ := stop_right k.flip (t := -t) (hi := -lo) (by grind)
      (fs.map Aff1.flip) (fun j hj => by
        obtain ⟨f, hf, rfl⟩ := List.mem_map.mp hj
        simp only [flip_at, Rat.neg_neg]; rw [hkm]; exact hle f hf)
    have rise : m ≤ k.at (-t') := by
      rw [← hkm]; simp only [Aff1.at]
      have : 0 ≤ (-k.β) * (t' - -t) := Rat.mul_nonneg (by grind) (by grind)
      grind
    refine ⟨-t', ⟨by grind, by grind, ?_⟩, fun f hf => ?_⟩
    · rcases h4 with h4 | ⟨j, hj, hc⟩
      · exact Or.inl (by grind)
      · obtain ⟨f, hf, rfl⟩ := List.mem_map.mp hj
        refine Or.inr (Or.inr ⟨f, hf, k, hk, ?_, ?_⟩)
        · intro he; apply hc.1; simp only [Aff1.flip]; rw [he]
        · have := hc.2; simp only [flip_at] at this; exact this
    · have := h3 f.flip (List.mem_map_of_mem hf)
      simp only [flip_at] at this
      exact Rat.le_trans rise this

end ForgeWeld.Extreme
