import Winding2d.Basic

/-!
# The winding number is constant along a segment that misses the polygon

Per side `a → b` and move `p → p'` with the closed segments disjoint (`step`): the change of the
side's crossing is a difference `phi b - phi a` of one vertex function, the signed indicator of
the half-strip between the two rays (half-open at both heights, `inBand`). Round a closed polygon
these cancel (`sum_sides_diff`, the signed `telescope` of Jordan.lean), so the winding number is
the same at `p` and `p'` (`wind_eq_of_seg`), and along any polygonal path (`wind_eq_of_path`).
-/

namespace Winding2d

/-- `v` is in the half-strip right of the upward segment `p → p'`, half-open at both heights. -/
def inBand (p p' v : Pt) : Int := if p.y < v.y ∧ v.y ≤ p'.y ∧ orient p p' v < 0 then 1 else 0

/-- The vertex function of a move `p → p'`. -/
def phi (p p' v : Pt) : Int :=
  if p.y < p'.y then inBand p p' v else if p'.y < p.y then -inBand p' p v else 0

theorem cross_swap (q a b : Pt) : cross q b a = -cross q a b := by
  unfold cross; rw [orient_swap]; grind

section step
variable {p p' a b : Pt}

theorem not_meets_sub_left {c : Pt} (hm : ¬Meets p p' a b) (hc : ∀ z, OnSeg z a c → OnSeg z a b) :
    ¬Meets a c p p' := fun ⟨z, h1, h2⟩ => hm ⟨z, h2, hc z h1⟩

theorem not_meets_sub_right {c : Pt} (hm : ¬Meets p p' a b) (hc : ∀ z, OnSeg z c b → OnSeg z a b) :
    ¬Meets c b p p' := fun ⟨z, h1, h2⟩ => hm ⟨z, h2, hc z h1⟩

/-- `a` below or at `p`'s height, `b` in the band. -/
theorem step_lo_mid (hp : p.y < p'.y) (hm : ¬Meets p p' a b) (ha : a.y ≤ p.y) (hb1 : p.y < b.y)
    (hb2 : b.y ≤ p'.y) : cross p a b - cross p' a b = inBand p p' b - inBand p p' a := by
  obtain ⟨c, hcy, -, hcr, ho⟩ := cut a b p.y ha hb1
  have hs := (sign_same (not_meets_sub_right hm hcr) (by grind) (p := c) (p' := b)
    (by unfold Btw; grind) (by unfold Btw; grind)).2
  have e1 := ho p rfl
  have e2 : orient p p' c = (c.x - p.x) * -(p'.y - p.y) := by simp only [orient]; rw [hcy]; grind
  have k1 := pos_mul_iff (k := b.y - a.y) (x := c.x - p.x) (by grind)
  have k2 := neg_mul_iff (k := p'.y - p.y) (x := -(c.x - p.x)) (by grind)
  have e3 : -(c.x - p.x) * (p'.y - p.y) = (c.x - p.x) * -(p'.y - p.y) := by grind
  rw [e3, ← e2] at k2; rw [← e1] at k1
  unfold cross inBand; grind

/-- `a` in the band, `b` above `p'`'s height. -/
theorem step_mid_hi (hp : p.y < p'.y) (hm : ¬Meets p p' a b) (ha1 : p.y < a.y) (ha2 : a.y ≤ p'.y)
    (hb : p'.y < b.y) : cross p a b - cross p' a b = inBand p p' b - inBand p p' a := by
  obtain ⟨c, hcy, hcl, -, ho⟩ := cut a b p'.y ha2 hb
  have hs := (sign_same (not_meets_sub_left hm hcl) (by grind) (p := a) (p' := c)
    (by unfold Btw; grind) (by unfold Btw; grind)).2
  have e1 := ho p' rfl
  have e2 : orient p p' c = -(c.x - p'.x) * (p'.y - p.y) := by simp only [orient]; rw [hcy]; grind
  have k1 := pos_mul_iff (k := b.y - a.y) (x := c.x - p'.x) (by grind)
  have k2 := neg_mul_iff (k := p'.y - p.y) (x := -(c.x - p'.x)) (by grind)
  rw [← e2] at k2; rw [← e1] at k1
  unfold cross inBand; grind

/-- `a` at or below `p`'s height, `b` above `p'`'s. -/
theorem step_lo_hi (hp : p.y < p'.y) (hm : ¬Meets p p' a b) (ha : a.y ≤ p.y) (hb : p'.y < b.y) :
    cross p a b - cross p' a b = inBand p p' b - inBand p p' a := by
  have hs := (sign_same hm (by grind) (by unfold Btw; grind) (by unfold Btw; grind)).1
  unfold cross inBand; grind

/-- Both in the band. -/
theorem step_mid_mid (hp : p.y < p'.y) (hm : ¬Meets p p' a b) (ha1 : p.y < a.y) (ha2 : a.y ≤ p'.y)
    (hb1 : p.y < b.y) (hb2 : b.y ≤ p'.y) :
    cross p a b - cross p' a b = inBand p p' b - inBand p p' a := by
  have hs := (sign_same (fun h => hm (meets_symm h)) (by grind) (p := a) (p' := b)
    (by unfold Btw; grind) (by unfold Btw; grind)).2
  unfold cross inBand; grind

/-- Both below, or both above, the band. -/
theorem step_out (hp : p.y < p'.y)
    (h : (a.y ≤ p.y ∧ b.y ≤ p.y) ∨ (p'.y < a.y ∧ p'.y < b.y)) :
    cross p a b - cross p' a b = inBand p p' b - inBand p p' a := by
  unfold cross inBand; grind

theorem step_up (hp : p.y < p'.y) (hm : ¬Meets p p' a b) :
    cross p a b - cross p' a b = inBand p p' b - inBand p p' a := by
  have hm' : ¬Meets p p' b a := fun h => hm (meets_flip' h)
  have sw : cross p b a - cross p' b a = inBand p p' a - inBand p p' b →
      cross p a b - cross p' a b = inBand p p' b - inBand p p' a := by
    rw [cross_swap p, cross_swap p']; grind
  by_cases ha : a.y ≤ p.y <;> by_cases ha' : a.y ≤ p'.y <;>
    by_cases hb : b.y ≤ p.y <;> by_cases hb' : b.y ≤ p'.y
  all_goals first
    | exact step_out hp (by grind)
    | exact step_lo_mid hp hm (by grind) (by grind) (by grind)
    | exact sw (step_lo_mid hp hm' (by grind) (by grind) (by grind))
    | exact step_mid_hi hp hm (by grind) (by grind) (by grind)
    | exact sw (step_mid_hi hp hm' (by grind) (by grind) (by grind))
    | exact step_lo_hi hp hm (by grind) (by grind)
    | exact sw (step_lo_hi hp hm' (by grind) (by grind))
    | exact step_mid_mid hp hm (by grind) (by grind) (by grind) (by grind)
    | grind

theorem step_level (hp : p.y = p'.y) (hm : ¬Meets p p' a b) : cross p a b = cross p' a b := by
  by_cases h1 : p.y < b.y ∧ ¬p.y < a.y
  · have hs := (sign_same hm (by grind) (by unfold Btw; grind) (by unfold Btw; grind)).1
    unfold cross; grind
  · by_cases h2 : p.y < a.y ∧ ¬p.y < b.y
    · have hs := (sign_same hm (by grind) (by unfold Btw; grind) (by unfold Btw; grind)).2
      unfold cross; grind
    · unfold cross; grind

/-- **The step identity along one side.** If the closed segments `pp'` and `ab` are disjoint,
the change of `ab`'s crossing from `p` to `p'` is `phi p p' b - phi p p' a`. No general
position: every coincidence of heights is decided by the half-open rule. -/
theorem step (hm : ¬Meets p p' a b) : cross p a b - cross p' a b = phi p p' b - phi p p' a := by
  unfold phi
  by_cases h1 : p.y < p'.y
  · simp only [h1, ite_true]; exact step_up h1 hm
  · by_cases h2 : p'.y < p.y
    · simp only [h1, h2, ite_true, ite_false]
      have := step_up h2 (fun h => hm (meets_flip h)) (a := a) (b := b)
      grind
    · simp only [h1, h2, ite_false]
      have := step_level (by grind) hm; grind

end step

/-! ## Round a closed polygon -/

theorem sum_map_sub {α : Type} (l : List α) (f g : α → Int) :
    (l.map fun x => f x - g x).sum = (l.map f).sum - (l.map g).sum := by
  induction l with
  | nil => rfl
  | cons x l ih => simp only [List.map_cons, List.sum_cons, ih]; grind

theorem chain_telescope (g : Pt → Int) : ∀ (a : Pt) (l : List Pt),
    ((chain (a :: l)).map fun e => g e.2 - g e.1).sum = g (l.getLastD a) - g a
  | a, [] => by simp [chain]
  | a, b :: t => by
    simp only [chain, List.map_cons, List.sum_cons]
    rw [chain_telescope g b t]
    simp only [List.getLastD_cons]; grind

/-- **Round a closed polygon, every vertex is counted once each way.** -/
theorem sum_sides_diff (g : Pt → Int) (vs : List Pt) :
    ((sides vs).map fun e => g e.2 - g e.1).sum = 0 := by
  cases vs with
  | nil => rfl
  | cons v rest =>
    have h := chain_telescope g v (rest ++ [v])
    simp only [List.getLastD_eq_getLast?, List.getLast?_append, List.getLast?_singleton,
      Option.some_or, Option.getD_some] at h
    simp only [sides, List.cons_append]; rw [h]; grind

/-- **Theorem 2.** The winding number is the same at the two ends of any segment that misses
every side. -/
theorem wind_eq_of_seg (vs : List Pt) (p p' : Pt) (h : ∀ e ∈ sides vs, ¬Meets p p' e.1 e.2) :
    wind vs p = wind vs p' := by
  have : wind vs p - wind vs p' = 0 := by
    unfold wind
    rw [← sum_map_sub, List.map_congr_left (fun e he => step (h e he)), sum_sides_diff]
  grind

/-- The same along a polygonal path, every leg of which misses every side. -/
theorem wind_eq_of_path (vs : List Pt) : ∀ (p : Pt) (path : List Pt),
    (∀ s ∈ chain (p :: path), ∀ e ∈ sides vs, ¬Meets s.1 s.2 e.1 e.2) →
    wind vs p = wind vs ((p :: path).getLast (by simp))
  | p, [] => fun _ => rfl
  | p, q :: t => fun h => by
    rw [wind_eq_of_seg vs p q (h (p, q) (by simp [chain])), List.getLast_cons_cons]
    exact wind_eq_of_path vs q t (fun s hs => h s (by simp [chain, hs]))

end Winding2d
