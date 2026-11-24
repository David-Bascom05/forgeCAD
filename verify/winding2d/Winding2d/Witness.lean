import Winding2d.SoS

/-!
# Missing is open: a strict witness for two disjoint closed segments

`meets_witness`: if the closed segments `pp'` and `ab` (`a ≠ b`) are disjoint, one of six strict
inequalities holds (`Wit`): `pp'` strictly on one side of the line `ab`, `ab` strictly on one side
of the line `pp'`, or `pp'` strictly beyond `b` or beyond `a` along `ab`. Each is a strict
polynomial inequality, so it survives small moves of `p` and `p'` (`ev_not_meets`), and each
implies the segments are disjoint (`wit_not_meets`).
-/

namespace Winding2d

def dot (u v : Pt) : Rat := u.x * v.x + u.y * v.y
def sub (u v : Pt) : Pt := ⟨u.x - v.x, u.y - v.y⟩
/-- `p + κ u`. -/
def addv (p : Pt) (κ : Rat) (u : Pt) : Pt := ⟨p.x + κ * u.x, p.y + κ * u.y⟩

def Wit (p p' a b : Pt) : Prop :=
  (0 < orient a b p ∧ 0 < orient a b p') ∨ (orient a b p < 0 ∧ orient a b p' < 0) ∨
  (0 < orient p p' a ∧ 0 < orient p p' b) ∨ (orient p p' a < 0 ∧ orient p p' b < 0) ∨
  (0 < dot (sub p b) (sub b a) ∧ 0 < dot (sub p' b) (sub b a)) ∨
  (0 < dot (sub p a) (sub a b) ∧ 0 < dot (sub p' a) (sub a b))

theorem sq_pos' {d : Rat} (h : d ≠ 0) : 0 < d * d := by
  rcases ne_cases h with h | h
  · have := (pos_mul_iff_neg h (x := d)).mpr h; exact this
  · exact Rat.mul_pos h h

theorem mul_self_nonneg' (d : Rat) : 0 ≤ d * d := by
  by_cases h : d = 0
  · subst h; grind
  · exact Rat.le_of_lt (sq_pos' h)

theorem conv_pos {o1 o2 t : Rat} (h1 : 0 < o1) (h2 : 0 < o2) (h0 : 0 ≤ t) (ht : t ≤ 1) :
    0 < (1 - t) * o1 + t * o2 := by
  have := Rat.mul_nonneg (by grind : (0:Rat) ≤ 1 - t) (Rat.le_of_lt h1)
  have := Rat.mul_nonneg h0 (Rat.le_of_lt h2)
  by_cases h : t = 0
  · subst h; grind
  · have := Rat.mul_pos (by grind : (0:Rat) < t) h2; grind

theorem orient_lerp_self (p p' : Pt) (t : Rat) : orient p p' (lerp p p' t) = 0 := by
  simp only [orient, lerp]; grind

theorem dot_lerp (p p' c d : Pt) (t : Rat) :
    dot (sub (lerp p p' t) c) d = (1 - t) * dot (sub p c) d + t * dot (sub p' c) d := by
  simp only [dot, sub, lerp]; grind

theorem wit_not_meets {p p' a b : Pt} (h : Wit p p' a b) : ¬Meets p p' a b := by
  rintro ⟨z, ⟨t, t0, t1, rfl⟩, ⟨s, s0, s1, hz⟩⟩
  have e1 : orient a b (lerp p p' t) = 0 := by rw [hz]; exact orient_lerp_self a b s
  have e2 : orient p p' (lerp a b s) = 0 := by rw [← hz]; exact orient_lerp_self p p' t
  rw [orient_lerp] at e1; rw [orient_lerp] at e2
  have f1 := dot_lerp p p' b (sub b a) t
  have f2 := dot_lerp p p' a (sub a b) t
  rw [hz] at f1 f2
  have g1 : dot (sub (lerp a b s) b) (sub b a) = (s - 1) * dot (sub b a) (sub b a) := by
    simp only [dot, sub, lerp]; grind
  have g2 : dot (sub (lerp a b s) a) (sub a b) = -s * dot (sub b a) (sub b a) := by
    simp only [dot, sub, lerp]; grind
  have hL : 0 ≤ dot (sub b a) (sub b a) := by
    simp only [dot, sub]
    have := mul_self_nonneg' (b.x - a.x); have := mul_self_nonneg' (b.y - a.y); grind
  have k1 := Rat.mul_nonneg (by grind : (0:Rat) ≤ 1 - s) hL
  have k2 := Rat.mul_nonneg s0 hL
  unfold Wit at h
  rcases h with h | h | h | h | h | h
  · have := conv_pos h.1 h.2 t0 t1; grind
  · have := conv_pos (o1 := -orient a b p) (o2 := -orient a b p') (by grind) (by grind) t0 t1; grind
  · have := conv_pos h.1 h.2 s0 s1; grind
  · have := conv_pos (o1 := -orient p p' a) (o2 := -orient p p' b) (by grind) (by grind) s0 s1; grind
  · have := conv_pos h.1 h.2 t0 t1; grind
  · have := conv_pos h.1 h.2 t0 t1; grind

theorem len_pos {a b : Pt} (h : a ≠ b) : 0 < dot (sub b a) (sub b a) := by
  simp only [dot, sub]
  by_cases hx : b.x - a.x = 0
  · have hy : b.y - a.y ≠ 0 := by intro hy; apply h; ext <;> grind
    have := sq_pos' hy; have := mul_self_nonneg' (b.x - a.x); grind
  · have := sq_pos' hx; have := mul_self_nonneg' (b.y - a.y); grind

/-- A point on the line `ab` (`a ≠ b`) is `lerp a b` of its projection. -/
theorem on_line {z a b : Pt} (ho : orient a b z = 0) (hab : a ≠ b) :
    z = lerp a b (dot (sub z a) (sub b a) / dot (sub b a) (sub b a)) := by
  have hL := len_pos hab
  have hs := Rat.div_mul_cancel (a := dot (sub z a) (sub b a)) (Rat.ne_of_gt hL)
  generalize dot (sub z a) (sub b a) / dot (sub b a) (sub b a) = s at hs
  simp only [dot, sub, orient] at hs hL ho
  have hx : (z.x - a.x - s * (b.x - a.x)) * ((b.x - a.x) * (b.x - a.x) + (b.y - a.y) * (b.y - a.y)) = 0 := by
    have := congrArg (· * (b.y - a.y)) ho; grind
  have hy : (z.y - a.y - s * (b.y - a.y)) * ((b.x - a.x) * (b.x - a.x) + (b.y - a.y) * (b.y - a.y)) = 0 := by
    have := congrArg (· * (b.x - a.x)) ho; grind
  have := mul_eq_zero_left (Rat.ne_of_gt hL) hx
  have := mul_eq_zero_left (Rat.ne_of_gt hL) hy
  ext <;> simp only [lerp] <;> grind

theorem root_of_not_strict (o1 o2 : Rat) (h1 : ¬(0 < o1 ∧ 0 < o2)) (h2 : ¬(o1 < 0 ∧ o2 < 0)) :
    ∃ t, 0 ≤ t ∧ t ≤ 1 ∧ (1 - t) * o1 + t * o2 = 0 := by
  by_cases e : (0 < o1 ↔ 0 < o2)
  · by_cases e' : (o1 < 0 ↔ o2 < 0)
    · exact ⟨0, by grind, by grind, by grind⟩
    · obtain ⟨t, h0, h1, he⟩ := exists_root (-o1) (-o2) (by grind)
      exact ⟨t, h0, h1, by grind⟩
  · exact exists_root o1 o2 e

/-- **Disjoint closed segments have a strict witness.** -/
theorem meets_witness {p p' a b : Pt} (hab : a ≠ b) (hm : ¬Meets p p' a b) : Wit p p' a b := by
  apply Classical.byContradiction; intro hW
  unfold Wit at hW
  have hL := len_pos hab
  have onE : ∀ z, orient a b z = 0 → 0 ≤ dot (sub z a) (sub b a) →
      dot (sub z a) (sub b a) ≤ dot (sub b a) (sub b a) → OnSeg z a b := by
    intro z hz h0 h1
    have hs := Rat.div_mul_cancel (a := dot (sub z a) (sub b a)) (Rat.ne_of_gt hL)
    have hu := unit_of _ _ _ (Rat.ne_of_gt hL) hs (by grind)
    exact ⟨_, hu.1, hu.2, on_line hz hab⟩
  have fE : ∀ z, dot (sub z b) (sub b a) = dot (sub z a) (sub b a) - dot (sub b a) (sub b a) ∧
      dot (sub z a) (sub a b) = -dot (sub z a) (sub b a) := by
    intro z; simp only [dot, sub]; constructor <;> grind
  obtain ⟨t, t0, t1, ht⟩ := root_of_not_strict (orient a b p) (orient a b p') (by grind) (by grind)
  have hz : orient a b (lerp p p' t) = 0 := by rw [orient_lerp]; exact ht
  have zout : dot (sub (lerp p p' t) a) (sub b a) < 0 ∨
      dot (sub b a) (sub b a) < dot (sub (lerp p p' t) a) (sub b a) := by
    apply Classical.byContradiction; intro hn
    exact hm ⟨_, ⟨t, t0, t1, rfl⟩, onE _ hz (by grind) (by grind)⟩
  by_cases hA : orient a b p = 0 ∧ orient a b p' = 0
  · -- `pp'` on the line `ab`: some point of it projects into `[0, |ab|²]`
    have k := fE p; have k' := fE p'
    have hdl := dot_lerp p p' a (sub b a)
    by_cases i0 : 0 ≤ dot (sub p a) (sub b a) ∧ dot (sub p a) (sub b a) ≤ dot (sub b a) (sub b a)
    · exact hm ⟨p, onSeg_left, onE p hA.1 i0.1 i0.2⟩
    · by_cases i1 : 0 ≤ dot (sub p' a) (sub b a) ∧ dot (sub p' a) (sub b a) ≤ dot (sub b a) (sub b a)
      · exact hm ⟨p', onSeg_right, onE p' hA.2 i1.1 i1.2⟩
      · obtain ⟨t', t0', t1', ht'⟩ := exists_root (dot (sub p a) (sub b a)) (dot (sub p' a) (sub b a))
          (by grind)
        have h0 : dot (sub (lerp p p' t') a) (sub b a) = 0 := by rw [hdl]; exact ht'
        exact hm ⟨_, ⟨t', t0', t1', rfl⟩, onE _ (by rw [orient_lerp, hA.1, hA.2]; grind)
          (by grind) (by grind)⟩
  · -- the lines cross at `z`, outside `ab`: `a` and `b` are strictly on one side of `pp'`
    have hs := Rat.div_mul_cancel (a := dot (sub (lerp p p' t) a) (sub b a)) (Rat.ne_of_gt hL)
    have hzl := on_line hz hab
    generalize dot (sub (lerp p p' t) a) (sub b a) / dot (sub b a) (sub b a) = s at hs hzl
    have g : (1 - s) * orient p p' a + s * orient p p' b = 0 := by
      rw [← orient_lerp, ← hzl]; exact orient_lerp_self p p' t
    by_cases e34 : orient p p' a = orient p p' b
    · -- then both are 0, `a` and `b` lie on the line `pp'`, and so do `p`, `p'` on `ab`
      have h3 : orient p p' a = 0 := by grind
      have h4 : orient p p' b = 0 := by grind
      by_cases hpp : p = p'
      · subst hpp
        apply hA
        have : orient a b p = orient a b p := rfl
        grind
      · have ea := on_line h3 hpp
        have eb := on_line h4 hpp
        apply hA
        constructor
        · rw [ea, eb]; simp only [orient, lerp]; grind
        · rw [ea, eb]; simp only [orient, lerp]; grind
    · have hd : orient p p' a - orient p p' b ≠ 0 := by grind
      have e3 : orient p p' a = s * (orient p p' a - orient p p' b) := by grind
      have e4 : orient p p' b = (s - 1) * (orient p p' a - orient p p' b) := by grind
      rcases zout with zo | zo
      · have s0 : s < 0 := (neg_mul_iff hL).mp (by grind)
        rcases ne_cases hd with hd | hd
        · have := Rat.mul_pos (by grind : 0 < -s) (by grind : 0 < -(orient p p' a - orient p p' b))
          have := Rat.mul_pos (by grind : 0 < 1 - s) (by grind : 0 < -(orient p p' a - orient p p' b))
          grind
        · have := Rat.mul_pos (by grind : 0 < -s) hd
          have := Rat.mul_pos (by grind : 0 < 1 - s) hd
          grind
      · have s1 : 1 < s := by
          have := (pos_mul_iff hL (x := s - 1)).mp (by grind); grind
        rcases ne_cases hd with hd | hd
        · have := Rat.mul_pos (by grind : 0 < s) (by grind : 0 < -(orient p p' a - orient p p' b))
          have := Rat.mul_pos (by grind : 0 < s - 1) (by grind : 0 < -(orient p p' a - orient p p' b))
          grind
        · have := Rat.mul_pos (by grind : 0 < s) hd
          have := Rat.mul_pos (by grind : 0 < s - 1) hd
          grind

theorem ev_neg_of_neg {o c1 c2 : Rat} (ho : o < 0) : Ev fun ε => o + c1 * ε + c2 * (ε * ε) < 0 :=
  (ev_pos_of_pos (o := -o) (c1 := -c1) (c2 := -c2) (by grind)).mono fun _ _ h => by grind

theorem orient_move3 (p u p' u' a : Pt) (κ : Rat) :
    orient (addv p κ u) (addv p' κ u') a = orient p p' a +
      ((u'.x - u.x) * (a.y - p.y) - (p'.x - p.x) * u.y - ((u'.y - u.y) * (a.x - p.x) - (p'.y - p.y) * u.x)) * κ +
      (-(u'.x - u.x) * u.y + (u'.y - u.y) * u.x) * (κ * κ) := by
  simp only [orient, addv]; grind

theorem orient_move1 (a b p u : Pt) (κ : Rat) :
    orient a b (addv p κ u) = orient a b p + ((b.x - a.x) * u.y - (b.y - a.y) * u.x) * κ + 0 * (κ * κ) := by
  simp only [orient, addv]; grind

theorem dot_move (p u c d : Pt) (κ : Rat) :
    dot (sub (addv p κ u) c) d = dot (sub p c) d + (u.x * d.x + u.y * d.y) * κ + 0 * (κ * κ) := by
  simp only [dot, sub, addv]; grind

/-- **Missing survives small moves.** If the closed segments `pp'` and `ab` (`a ≠ b`) are
disjoint, so are `[p + κu, p' + κu']` and `ab` for every small `κ`. -/
theorem ev_not_meets {p p' a b : Pt} (hab : a ≠ b) (hm : ¬Meets p p' a b) (u u' : Pt) :
    Ev fun κ => ¬Meets (addv p κ u) (addv p' κ u') a b := by
  have w := meets_witness hab hm
  have key : Ev fun κ => Wit (addv p κ u) (addv p' κ u') a b := by
    unfold Wit at w ⊢
    simp only [orient_move3, orient_move1, dot_move]
    rcases w with h | h | h | h | h | h
    · exact ((ev_pos_of_pos h.1).and (ev_pos_of_pos h.2)).mono fun _ _ h => Or.inl h
    · exact ((ev_neg_of_neg h.1).and (ev_neg_of_neg h.2)).mono fun _ _ h => Or.inr (Or.inl h)
    · exact ((ev_pos_of_pos h.1).and (ev_pos_of_pos h.2)).mono fun _ _ h => Or.inr (Or.inr (Or.inl h))
    · exact ((ev_neg_of_neg h.1).and (ev_neg_of_neg h.2)).mono fun _ _ h =>
        Or.inr (Or.inr (Or.inr (Or.inl h)))
    · exact ((ev_pos_of_pos h.1).and (ev_pos_of_pos h.2)).mono fun _ _ h =>
        Or.inr (Or.inr (Or.inr (Or.inr (Or.inl h))))
    · exact ((ev_pos_of_pos h.1).and (ev_pos_of_pos h.2)).mono fun _ _ h =>
        Or.inr (Or.inr (Or.inr (Or.inr (Or.inr h))))
  exact key.mono fun _ _ h => wit_not_meets h

/-- A point off `ab` (`a ≠ b`): every short segment from near it misses `ab`. -/
theorem ev_not_meets_pt {v a b : Pt} (hab : a ≠ b) (hv : ¬OnSeg v a b) (u u' : Pt) :
    Ev fun κ => ¬Meets (addv v κ u) (addv v κ u') a b :=
  ev_not_meets hab (fun ⟨z, ⟨t, _, _, hz⟩, hz'⟩ => hv (by
    have : z = v := by rw [hz]; ext <;> simp only [lerp] <;> grind
    rw [← this]; exact hz')) u u'

end Winding2d
