import ForgeExact.DecideOnce.Local
import ForgeExact.DecideOnce.BRep
import ForgeExact.JordanLocal
import ForgeExact.DecideOnce.Ledger

/-!
# `LocalCompat` for B-reps, from Jordan–Brouwer along a segment

The frontier's open obligation `LocalCompat` holds for closed flat-faced B-reps classified by the
reference's ray parity when every face is either *near* (its plane a carrier of the local
arrangement, up to scale) or *far* (nonzero normal, and every point of every region cell outside
its bounding box: the face sits in a box disjoint from the region). Two points of a region cell are
joined by a segment inside the cell (full cells are convex, `covCell_conv`), which is strictly on
one side of each near plane and meets each far plane only outside the far face's box; so
`Jordan.contains_const_local` applies.
-/

namespace ForgeExact.DecideOnce

open ForgeExact ForgeExact.Ref Topology

theorem sgnQ_conv {a b t : Rat} (h : sgnQ a = sgnQ b) (h0 : 0 ≤ t) (h1 : t ≤ 1) :
    sgnQ (a + t * (b - a)) = sgnQ a := by
  have e : a + t * (b - a) = (1 - t) * a + t * b := by grind
  rw [e]
  cases hs : sgnQ a
  · have ha := (sgnQ_neg_iff a).mp hs
    have hb := (sgnQ_neg_iff b).mp (h ▸ hs)
    apply (sgnQ_neg_iff _).mpr
    have := Rat.mul_nonneg (by grind : (0:Rat) ≤ 1 - t) (by grind : (0:Rat) ≤ -a)
    have := Rat.mul_nonneg h0 (by grind : (0:Rat) ≤ -b)
    by_cases ht : t = 0
    · subst ht; grind
    · have := Rat.mul_pos (by grind : (0:Rat) < t) (by grind : (0:Rat) < -b); grind
  · have ha := (sgnQ_zero_iff a).mp hs
    have hb := (sgnQ_zero_iff b).mp (h ▸ hs)
    apply (sgnQ_zero_iff _).mpr; rw [ha, hb]; grind
  · have ha := (sgnQ_pos_iff a).mp hs
    have hb := (sgnQ_pos_iff b).mp (h ▸ hs)
    apply (sgnQ_pos_iff _).mpr
    have := Rat.mul_nonneg (by grind : (0:Rat) ≤ 1 - t) (Rat.le_of_lt ha)
    have := Rat.mul_nonneg h0 (Rat.le_of_lt hb)
    by_cases ht : t = 0
    · subst ht; grind
    · have := Rat.mul_pos (by grind : (0:Rat) < t) hb; grind

/-- **Cells are convex.** -/
theorem covCell_conv : ∀ {ps : List Plane} {T : Cov} {x y : V3}, covCell ps T x → covCell ps T y →
    ∀ {t : Rat}, 0 ≤ t → t ≤ 1 → covCell ps T (Jordan.ptv x y t)
  | [], [], _, _, _, _, _, _, _ => trivial
  | p :: ps, s :: T, x, y, ⟨hx, hx'⟩, ⟨hy, hy'⟩, t, h0, h1 => by
    refine ⟨?_, covCell_conv hx' hy' h0 h1⟩
    rw [Jordan.eval_ptv, sgnQ_conv (hx.trans hy.symm) h0 h1, hx]
  | [], _ :: _, _, _, h, _, _, _, _ => h.elim
  | _ :: _, [], _, _, h, _, _, _, _ => h.elim

/-- **`LocalCompat` for closed B-reps whose faces are near or far.** -/
theorem local_compat (ps : List Plane) (In : Cov → Prop) (S : Nat → Solid)
    (hflat : ∀ i, Jordan.FacesFlat (S i)) (hcl : ∀ i, Jordan.Closed (S i))
    (hface : ∀ i, ∀ f ∈ Jordan.facesL (S i), Scaled ps f.plane ∨
      (f.plane.n ≠ ⟨0, 0, 0⟩ ∧ ∀ T x, In T → covCell ps T x → Jordan.OutBox f x)) :
    LocalCompat ps (fun i x => (S i).contains x = true) In := by
  intro T hT hin i
  have key : ∀ x y, covCell ps T x → covCell ps T y → (S i).contains x = (S i).contains y := by
    intro x y hx hy
    apply Jordan.contains_const_local (S i) (hflat i) (hcl i) x y
    intro f hf
    rcases hface i f hf with ⟨q, hq, k, hk, he⟩ | ⟨hn, hfar⟩
    · left
      rw [he, he, show k * q.eval x * (k * q.eval y) = (k * k) * (q.eval x * q.eval y) by grind]
      exact Rat.mul_pos (Jordan.sq_pos hk) (same_side ps hT hx hy q hq)
    · exact Or.inr ⟨hn, fun t h0 h1 => hfar T _ hin (covCell_conv hx hy h0 h1)⟩
  by_cases h : ∃ x, covCell ps T x ∧ (S i).contains x = true
  · obtain ⟨x, hx, hc⟩ := h
    exact Or.inl fun y hy => by show (S i).contains y = true; rw [← key x y hx hy]; exact hc
  · exact Or.inr fun x hx hc => h ⟨x, hx, hc⟩

end ForgeExact.DecideOnce
