import ForgeExact.DecideOnce.Nef
import ForgeExact.Jordan

/-!
# `compat` for B-reps: the main hypothesis of (c), discharged

For solids given as the reference's B-reps (faces with loops, membership by ray parity,
`Ref.Solid.contains`), `Exact.compat` holds whenever each solid is closed and flat-faced and every
face plane is a carrier of the arrangement up to a nonzero scale (`brep_compat`). It rests on
`Jordan.contains_const`, Jordan–Brouwer for the reference's ray parity: two points of one full
cell are strictly on one side of every carrier, so of every face plane.
-/

namespace ForgeExact.DecideOnce

open ForgeExact ForgeExact.Ref

/-- A face plane that is a carrier of the arrangement, scaled by a nonzero rational. -/
def Scaled (ps : List Plane) (P : Plane) : Prop :=
  ∃ q ∈ ps, ∃ k : Rat, k ≠ 0 ∧ ∀ x : V3, P.eval x = k * q.eval x

theorem mem_full : ∀ {n : Nat} {T : Cov}, T ∈ fulls n → Sgn.zero ∉ T
  | 0, T, h => by simp [fulls] at h; subst h; simp
  | n + 1, T, h => by
    simp only [fulls, List.mem_flatMap, List.mem_cons, List.mem_nil_iff, or_false] at h
    obtain ⟨l, hl, rfl | rfl⟩ := h <;> simp [mem_full hl]

theorem sgn_in : ∀ {ps : List Plane} {T : Cov} {x : V3}, covCell ps T x → ∀ q ∈ ps, sgnQ (q.eval x) ∈ T
  | [], [], _, _, q, hq => nomatch hq
  | p :: ps, s :: T, x, ⟨hx, hx'⟩, q, hq => by
    rcases List.mem_cons.mp hq with rfl | hq
    · rw [hx]; exact List.mem_cons_self ..
    · exact List.mem_cons_of_mem _ (sgn_in hx' q hq)
  | [], _ :: _, _, h, _, _ => h.elim
  | _ :: _, [], _, h, _, _ => h.elim

/-- Two points of one full cell are strictly on one side of every carrier. -/
theorem same_side (ps : List Plane) {T : Cov} (hT : T ∈ fulls ps.length) {x y : V3}
    (hx : covCell ps T x) (hy : covCell ps T y) (q : Plane) (hq : q ∈ ps) :
    0 < q.eval x * q.eval y := by
  have hs := same_signs hx hy q hq
  have hnz : sgnQ (q.eval x) ≠ .zero := fun h => mem_full hT (h ▸ sgn_in hx q hq)
  rcases hsx : sgnQ (q.eval x) with _ | _ | _
  · rw [hsx] at hs
    have h1 := (sgnQ_neg_iff _).mp hsx
    have h2 := (sgnQ_neg_iff _).mp hs.symm
    have := (Jordan.mul_pos_iff_neg h1 (x := q.eval y)).mpr h2
    exact this
  · exact absurd hsx hnz
  · rw [hsx] at hs
    exact Rat.mul_pos ((sgnQ_pos_iff _).mp hsx) ((sgnQ_pos_iff _).mp hs.symm)

/-- **`compat` for closed B-reps.** Each solid's ray-parity membership is constant on every full
cell of an arrangement holding its face planes (up to scale). -/
theorem brep_compat (ps : List Plane) (S : Nat → Solid) (hflat : ∀ i, Jordan.FacesFlat (S i))
    (hcl : ∀ i, Jordan.Closed (S i)) (hpl : ∀ i, ∀ f ∈ Jordan.facesL (S i), Scaled ps f.plane) :
    ∀ T ∈ fulls ps.length, ∀ i, (covCell ps T).Sub (fun x => (S i).contains x = true) ∨
      ∀ x, covCell ps T x → ¬(S i).contains x = true := by
  intro T hT i
  have key : ∀ x y, covCell ps T x → covCell ps T y → (S i).contains x = (S i).contains y := by
    intro x y hx hy
    apply Jordan.contains_const (S i) (hflat i) (hcl i) x y
    intro f hf
    obtain ⟨q, hq, k, hk, he⟩ := hpl i f hf
    rw [he, he, show k * q.eval x * (k * q.eval y) = (k * k) * (q.eval x * q.eval y) by grind]
    exact Rat.mul_pos (Jordan.sq_pos hk) (same_side ps hT hx hy q hq)
  by_cases h : ∃ x, covCell ps T x ∧ (S i).contains x = true
  · obtain ⟨x, hx, hc⟩ := h
    exact Or.inl fun y hy => by show (S i).contains y = true; rw [← key x y hx hy]; exact hc
  · exact Or.inr fun x hx hc => h ⟨x, hx, hc⟩

end ForgeExact.DecideOnce
