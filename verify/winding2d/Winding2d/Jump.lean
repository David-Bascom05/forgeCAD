import Winding2d.Sides

/-!
# Across a side, the winding number jumps by one

`jump`: the left value of a side of a simple polygon is its right value plus one. From the step
identity: across the short segment between the two probes at the side's midpoint, every other
side's crossings change by a difference of `phi`, which cancels round the polygon; the side
itself contributes exactly `1` (`jump_corr`, by the four axis directions of `hdir`).
-/

namespace Winding2d

theorem sum_map_add {α : Type} (l : List α) (f g : α → Int) :
    (l.map fun x => f x + g x).sum = (l.map f).sum + (l.map g).sum := by
  induction l with
  | nil => rfl
  | cons x l ih => simp only [List.map_cons, List.sum_cons, ih]; grind

theorem probe_x (a b : Pt) (t ς κ : Rat) :
    (probe a b t ς κ).x = a.x + t * (b.x - a.x) + κ * (ς * (hdir a b).x) := by
  simp only [probe, addv, lerp, smul]

theorem probe_y (a b : Pt) (t ς κ : Rat) :
    (probe a b t ς κ).y = a.y + t * (b.y - a.y) + κ * (ς * (hdir a b).y) := by
  simp only [probe, addv, lerp, smul]

/-- The side's own contribution to the jump. -/
theorem jump_corr {a b : Pt} (hab : a ≠ b) {κ : Rat} (hκ : 0 < κ) :
    (cross (probe a b (1 / 2) 1 κ) a b - cross (probe a b (1 / 2) (-1) κ) a b) -
      (phi (probe a b (1 / 2) 1 κ) (probe a b (1 / 2) (-1) κ) b -
        phi (probe a b (1 / 2) 1 κ) (probe a b (1 / 2) (-1) κ) a) = 1 := by
  have hp := orient_probe a b (1 / 2) 1 κ
  have hp' := orient_probe a b (1 / 2) (-1) κ
  have hω := Rat.mul_pos hκ (omega_pos hab)
  have x1 := probe_x a b (1 / 2) 1 κ
  have x2 := probe_x a b (1 / 2) (-1) κ
  have y1 := probe_y a b (1 / 2) 1 κ
  have y2 := probe_y a b (1 / 2) (-1) κ
  generalize probe a b (1 / 2) 1 κ = p at *
  generalize probe a b (1 / 2) (-1) κ = p' at *
  unfold hdir at x1 x2 y1 y2
  by_cases h1 : a.y < b.y
  · simp only [h1, if_true] at x1 x2 y1 y2
    unfold cross phi inBand; grind
  · by_cases h2 : b.y < a.y
    · simp only [h1, h2, if_true, if_false] at x1 x2 y1 y2
      unfold cross phi inBand; grind
    · have hy : a.y = b.y := by grind
      by_cases h3 : a.x < b.x
      · simp only [h1, h2, h3, if_true, if_false] at x1 x2 y1 y2
        have o1 : orient p' p b = -(κ * (b.x - a.x)) := by simp only [orient]; grind
        have o2 : orient p' p a = κ * (b.x - a.x) := by simp only [orient]; grind
        have := Rat.mul_pos hκ (by grind : 0 < b.x - a.x)
        unfold cross phi inBand; grind
      · simp only [h1, h2, h3, if_true, if_false] at x1 x2 y1 y2
        have hx : b.x < a.x := by
          have : a.x ≠ b.x := fun h => hab (by ext <;> grind)
          grind
        have o1 : orient p p' b = κ * (a.x - b.x) := by simp only [orient]; grind
        have o2 : orient p p' a = -(κ * (a.x - b.x)) := by simp only [orient]; grind
        have := Rat.mul_pos hκ (by grind : 0 < a.x - b.x)
        unfold cross phi inBand; grind

theorem sum_phi_zero (vs : List Pt) (p p' : Pt) :
    ((List.range vs.length).map fun j => phi p p' (vtx vs (j + 1)) - phi p p' (vtx vs j)).sum = 0 := by
  have := sum_sides_diff (fun v => phi p p' v) vs
  rw [sides_eq, List.map_map] at this
  exact this

/-- **The jump.** Left of a side, the winding number is one more than right of it. -/
theorem jump {vs : List Pt} (hs : Simple vs) {i : Nat} (hi : i < vs.length) :
    val vs i 1 = val vs i (-1) + 1 := by
  have others := ev_forall_lt vs.length (fun j κ => j ≠ i → ¬Meets
      (addv (lerp (vtx vs i) (vtx vs (i + 1)) (1 / 2)) κ (smul 1 (hdir (vtx vs i) (vtx vs (i + 1)))))
      (addv (lerp (vtx vs i) (vtx vs (i + 1)) (1 / 2)) κ (smul (-1) (hdir (vtx vs i) (vtx vs (i + 1)))))
      (vtx vs j) (vtx vs (j + 1))) (fun j hj => by
    by_cases e : j = i
    · exact Ev.true.mono fun _ _ _ h => absurd e h
    · exact (ev_not_meets_pt (hs.nondeg j)
        (interior_off hs hi hj (Ne.symm e) (by grind) (by grind)) _ _).mono fun _ _ h _ => h)
  have hl := val_spec hs hi (Or.inl rfl) (t := 1 / 2) (by grind) (by grind)
  have hr := val_spec hs hi (Or.inr rfl) (t := 1 / 2) (by grind) (by grind)
  obtain ⟨κ0, hκ0, hall⟩ := (others.and (hl.and hr))
  obtain ⟨ho, hwl, hwr⟩ := hall κ0 hκ0 (by grind)
  rw [← hwl, ← hwr]
  have hc := jump_corr (hs.nondeg i) hκ0
  simp only [← probe_eq] at ho
  generalize probe (vtx vs i) (vtx vs (i + 1)) (1 / 2) 1 κ0 = p at *
  generalize probe (vtx vs i) (vtx vs (i + 1)) (1 / 2) (-1) κ0 = p' at *
  have e1 : wind vs p - wind vs p' =
      ((List.range vs.length).map fun j => cross p (vtx vs j) (vtx vs (j + 1)) -
        cross p' (vtx vs j) (vtx vs (j + 1))).sum := by
    rw [wind_eq_sum, wind_eq_sum, ← sum_map_sub]
  have e2 : ((List.range vs.length).map fun j => cross p (vtx vs j) (vtx vs (j + 1)) -
        cross p' (vtx vs j) (vtx vs (j + 1))).sum =
      ((List.range vs.length).map fun j => phi p p' (vtx vs (j + 1)) - phi p p' (vtx vs j)).sum +
      ((List.range vs.length).map fun j => (cross p (vtx vs j) (vtx vs (j + 1)) -
        cross p' (vtx vs j) (vtx vs (j + 1))) - (phi p p' (vtx vs (j + 1)) - phi p p' (vtx vs j))).sum := by
    rw [← sum_map_add]; congr 1; apply List.map_congr_left; intro j _; grind
  have e3 : ((List.range vs.length).map fun j => (cross p (vtx vs j) (vtx vs (j + 1)) -
        cross p' (vtx vs j) (vtx vs (j + 1))) - (phi p p' (vtx vs (j + 1)) - phi p p' (vtx vs j))).sum = 1 := by
    rw [sum_range_single _ vs.length i hi]
    · exact hc
    · intro j hj hne
      have := step (ho j hj hne); grind
  rw [sum_phi_zero] at e2
  grind

end Winding2d
