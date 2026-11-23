import ForgeCertify.Bridge

/-!
# The checker is sound, at a generic point

`at_generic` : if `check` accepts, then at every point `x` nothing is tied at (`Gen`), the
result's membership is the boolean of the operands' memberships. (`Main.lean`'s theorem then
shows the tied points are nowhere dense, so the regularised sets agree.)

The steps: memberships are triangle-soup parities (`opd_contains`, `face_tris`); grouped by plane
along the `+x` line through `x` (`soup_group`); on each plane the result's crossing is the jump
of the boolean at the atom containing `x` (`result_on`, `atom_lookup`), whose classification holds
where the line meets it (`constancy`); telescoping (`tele`) sums the jumps.
-/

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

theorem par_and_const {α : Type} (l : List α) (b : Bool) (f : α → Bool) :
    par (l.map fun a => b && f a) = (b && par (l.map f)) := by
  cases b
  · simp only [Bool.false_and]; apply par_all_false; intro c hc; obtain ⟨a, _, rfl⟩ := List.mem_map.mp hc; rfl
  · simp

/-! ## Operands as triangle soups -/

/-- All of an operand's segments: faces' loops and triangles' edges. -/
def Opd.segs (o : Opd) : List Seg :=
  (o.faces.zip o.tris).flatMap fun ft => faceSegs ft.1 ++ ft.2.flatMap Tri.segs

theorem opd_contains (o : Opd) (x : V3) (ht : tilesOK o = true) (g : ∀ s ∈ o.segs, GenSeg x s) :
    (Solid.mk "" o.faces.toArray).contains x = par (o.soup.map fun t => hit t.1 t.2 x) := by
  unfold tilesOK at ht
  simp only [Bool.and_eq_true, beq_iff_eq, List.all_eq_true] at ht
  obtain ⟨hlen, hall⟩ := ht
  rw [contains_par]
  have hf : o.faces = (o.faces.zip o.tris).map (·.1) := by rw [List.map_fst_zip]; omega
  rw [hf, List.map_map]
  unfold Opd.soup
  rw [par_flatMap]
  apply par_map_congr
  intro ft hft
  obtain ⟨⟨hn, htri⟩, hz⟩ := hall ft hft
  simp only [Function.comp, List.map_map]
  rw [pierced_eq]
  by_cases hx : ft.1.plane.n.x = 0
  · have : ahead ft.1.plane x = false := by simp [ahead, hx]
    rw [this, Bool.false_and]
    apply Eq.symm; apply par_all_false; intro b hb
    obtain ⟨t, _, rfl⟩ := List.mem_map.mp hb; simp [hit, this]
  · have hD : ∀ t ∈ ft.2, t.D ≠ 0 := by
      intro t htm
      have := (htri t htm)
      have hp := this.2; unfold proper at hp
      rw [show (ft.1.plane.n.x != 0) = true by simpa using hx] at hp; revert hp; simp
    rw [face_tris ft.1 ft.2 x hz hD (fun s hs => g s (List.mem_flatMap.mpr ⟨ft, hft, hs⟩))]
    have : ((fun t : Plane × Tri => hit t.1 t.2 x) ∘ fun t => (ft.1.plane, t)) =
        fun t => ahead ft.1.plane x && t.ins x := by funext t; rfl
    rw [this, par_and_const]

/-! ## Grouped by plane -/

/-- On the canonical plane `P` (not along `x`). -/
def onP (P : Plane) (t : Plane × Tri) : Bool := nonvert t.1 && decide (t.1.canon = P)

theorem offP_onP (P : Plane) (t : Plane × Tri) : offP P t = !onP P t := rfl

/-- A soup's hits, grouped by the planes `L` (which hold every plane not along `x`). -/
theorem soup_group (T : List (Plane × Tri)) (L : List Plane) (hL : L.Nodup) (x : V3)
    (hn : ∀ t ∈ T, t.1.n ≠ ⟨0, 0, 0⟩) (hcov : ∀ t ∈ T, nonvert t.1 = true → t.1.canon ∈ L) :
    par (T.map fun t => hit t.1 t.2 x) =
      par (L.map fun P => ahead P x && par ((T.filter (onP P)).map fun t => t.2.ins x)) := by
  have h1 : par (T.map fun t => hit t.1 t.2 x) = par ((T.filter fun t => nonvert t.1).map fun t => hit t.1 t.2 x) := by
    rw [par_filter]; apply par_map_congr; intro t _
    cases h : nonvert t.1
    · simp [nonvert] at h; simp [hit, ahead, h]
    · rfl
  rw [h1, par_group _ (fun t => t.1.canon) L hL (fun t ht => hcov t (List.mem_filter.mp ht).1 (List.mem_filter.mp ht).2)]
  apply par_map_congr; intro P _
  rw [List.filter_filter]
  have : ∀ t ∈ (T.filter fun t => decide (t.1.canon = P) && nonvert t.1),
      hit t.1 t.2 x = (ahead P x && t.2.ins x) := by
    intro t ht
    obtain ⟨htT, hc⟩ := List.mem_filter.mp ht
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hc
    simp only [hit]; rw [← hc.1, ahead_canon _ (hn t htT)]
  rw [par_map_congr _ _ _ this]
  have e : (T.filter fun t => decide (t.1.canon = P) && nonvert t.1) = T.filter (onP P) := by
    apply List.filter_congr; intro t _; simp only [onP]
    by_cases h : t.1.canon = P <;> simp [h]
  rw [e, par_and_const]

end ForgeCertify
