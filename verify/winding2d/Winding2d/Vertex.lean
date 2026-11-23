import Winding2d.Jump

/-!
# Round a vertex, a side's value carries to the next side

`vertex`: at vertex `v` between sides `u → v` and `v → w`, the value on side `ς` of the first
side is the value on side `ς` of the next. The path: a probe near the end of `u → v`, the point
`v + s·ς·c` (`c` strictly left of both sides' lines, `c_good`), a probe near the start of `v → w`.
Each leg is kept off the two sides by one of their lines (`sep`; which line depends on whether
`ς` is the convex side, `ς · orient u v w > 0`), and off every other side by being short
(`vertex_off`, `ev_not_meets`). A straight vertex needs no spike (`no_spike`).
-/

namespace Winding2d

theorem vtx_nxt_succ (vs : List Pt) (i : Nat) (hi : i < vs.length) :
    vtx vs (nxt vs.length i + 1) = vtx vs (i + 2) := by
  unfold nxt; split
  · rename_i h; rw [show i + 2 = (i + 1) + 1 from rfl, h]; simp [vtx]
  · rfl

theorem orient_self_l (a b : Pt) : orient a b a = 0 := by simp only [orient]; grind
theorem orient_self_r (a b : Pt) : orient a b b = 0 := by simp only [orient]; grind

/-- The turn's witness direction: strictly left of both sides. -/
def cdir (u v w : Pt) : Pt :=
  if 0 < orient u v w then ⟨(w.x - v.x) - (v.x - u.x), (w.y - v.y) - (v.y - u.y)⟩
  else if orient u v w < 0 then ⟨(v.x - u.x) - (w.x - v.x), (v.y - u.y) - (w.y - v.y)⟩
  else ⟨-(v.y - u.y), v.x - u.x⟩

section simple
variable {vs : List Pt} (hs : Simple vs)
include hs

/-- No spike: at a straight vertex the two sides run the same way. -/
theorem no_spike (i : Nat) (h0 : orient (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2)) = 0) :
    0 < dot (sub (vtx vs (i + 1)) (vtx vs i)) (sub (vtx vs (i + 2)) (vtx vs (i + 1))) := by
  have hu := hs.nondeg i
  have hv := hs.nondeg (i + 1)
  have hadj := hs.adj i
  rw [show i + 1 + 1 = i + 2 from rfl] at hv
  generalize vtx vs i = u at *
  generalize vtx vs (i + 1) = v at *
  generalize vtx vs (i + 2) = w at *
  apply Classical.byContradiction; intro hd
  have hL := len_pos hu
  have hw := on_line h0 hu
  have hs' := Rat.div_mul_cancel (a := dot (sub w u) (sub v u)) (Rat.ne_of_gt hL)
  generalize dot (sub w u) (sub v u) / dot (sub v u) (sub v u) = ρ at hs' hw
  have e : dot (sub v u) (sub w v) = (ρ - 1) * dot (sub v u) (sub v u) := by
    rw [hw]; simp only [dot, sub, lerp]; grind
  have hρ : ρ ≤ 1 := by
    have := (nonneg_mul_iff hL (x := 1 - ρ)).mp (by grind); grind
  by_cases h1 : ρ = 1
  · apply hv; rw [hw, h1]; ext <;> simp only [lerp] <;> grind
  · by_cases h2 : 0 ≤ ρ
    · apply hv; symm
      exact hadj w (by rw [hw]; exact ⟨ρ, h2, hρ, rfl⟩) onSeg_right
    · -- `u` lies between `v` and `w`
      have hd' : 1 - ρ ≠ 0 := by grind
      have hk := Rat.div_mul_cancel (a := 1) hd'
      have hu' : u = lerp v w (1 / (1 - ρ)) := by
        rw [hw]; ext <;> simp only [lerp] <;> grind
      have h0' : 0 ≤ 1 / (1 - ρ) := Rat.le_of_lt ((pos_mul_iff (by grind : (0:Rat) < 1 - ρ)).mp (by grind))
      have h1' : 1 / (1 - ρ) ≤ 1 := by
        have := (nonneg_mul_iff (by grind : (0:Rat) < 1 - ρ) (x := 1 - 1 / (1 - ρ))).mp (by grind); grind
      exact hu (hadj u onSeg_left (by rw [hu']; exact ⟨_, h0', h1', rfl⟩))

theorem c_good (i : Nat) :
    0 < crs (vtx vs i) (vtx vs (i + 1)) (cdir (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2))) ∧
    0 < crs (vtx vs (i + 1)) (vtx vs (i + 2)) (cdir (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2))) := by
  have ns := no_spike hs i
  have hu := hs.nondeg i
  have hL := len_pos hu
  generalize vtx vs i = u at *
  generalize vtx vs (i + 1) = v at *
  generalize vtx vs (i + 2) = w at *
  unfold cdir
  by_cases h1 : 0 < orient u v w
  · simp only [h1, ite_true]; simp only [crs, orient] at h1 ⊢; constructor <;> grind
  · by_cases h2 : orient u v w < 0
    · simp only [h1, h2, ite_true, ite_false]; simp only [crs, orient] at h2 ⊢; constructor <;> grind
    · have h0 : orient u v w = 0 := by grind
      have := ns h0
      simp only [h1, h2, ite_false]; simp only [crs, orient, dot, sub] at h0 this hL ⊢; constructor <;> grind

theorem nxt_ne {i : Nat} (hi : i < vs.length) : nxt vs.length i ≠ i := by
  have := hs.three; rcases nxt_cases (n := vs.length) hi with ⟨_, b⟩ | ⟨_, b⟩ <;> omega

/-- **Round a vertex.** -/
theorem vertex {i : Nat} (hi : i < vs.length) {ς : Rat} (hς : Sgn ς) :
    val vs i ς = val vs (nxt vs.length i) ς := by
  have hn := nxt_lt (n := vs.length) hi
  have e1 := vtx_nxt vs i hi
  have e2 := vtx_nxt_succ vs i hi
  have hne := nxt_ne hs hi
  have cg := c_good hs i
  have hu := hs.nondeg i
  have hv : vtx vs (i + 1) ≠ vtx vs (i + 2) := hs.nondeg (i + 1)
  have hς2 := sgn_sq hς
  -- the scale `s` of the corner
  have hsE : Ev fun s => s < 1 ∧ ∀ j < vs.length, j ≠ i → j ≠ nxt vs.length i →
      ¬Meets (addv (vtx vs (i + 1)) s (smul (-1) (sub (vtx vs (i + 1)) (vtx vs i))))
        (addv (vtx vs (i + 1)) s (smul ς (cdir (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2)))))
        (vtx vs j) (vtx vs (j + 1)) ∧
      ¬Meets (addv (vtx vs (i + 1)) s (smul ς (cdir (vtx vs i) (vtx vs (i + 1)) (vtx vs (i + 2)))))
        (addv (vtx vs (i + 1)) s (sub (vtx vs (i + 2)) (vtx vs (i + 1))))
        (vtx vs j) (vtx vs (j + 1)) :=
    (ev_lt (by grind : (0:Rat) < 1)).and (ev_forall_lt vs.length _ (fun j hj => by
      by_cases e : j = i ∨ j = nxt vs.length i
      · exact Ev.true.mono fun _ _ _ h1 h2 => absurd e (by grind)
      · have hv' := vertex_off hs hi hj (by grind) (by grind)
        exact ((ev_not_meets_pt (hs.nondeg j) hv' _ _).and (ev_not_meets_pt (hs.nondeg j) hv' _ _)).mono
          fun _ _ h _ _ => h))
  obtain ⟨s, hs0, hsP⟩ := hsE
  obtain ⟨hs1, hlegs⟩ := hsP s hs0 (by grind)
  have k1 := val_spec hs hi hς (t := 1 - s) (by grind) (by grind)
  have k2 := val_spec hs hn hς (t := s) hs0 hs1
  rw [e1, e2] at k2
  have hA : lerp (vtx vs i) (vtx vs (i + 1)) (1 - s) =
      addv (vtx vs (i + 1)) s (smul (-1) (sub (vtx vs (i + 1)) (vtx vs i))) := by
    ext <;> simp only [lerp, addv, smul, sub] <;> grind
  have hB : lerp (vtx vs (i + 1)) (vtx vs (i + 2)) s =
      addv (vtx vs (i + 1)) s (sub (vtx vs (i + 2)) (vtx vs (i + 1))) := by
    ext <;> simp only [lerp, addv, sub] <;> grind
  generalize hU : vtx vs i = u at *
  generalize hV : vtx vs (i + 1) = v at *
  generalize hW : vtx vs (i + 2) = w at *
  generalize cdir u v w = c at *
  generalize hC : addv v s (smul ς c) = C at *
  have k3 : Ev fun κ => ∀ j < vs.length, j ≠ i → j ≠ nxt vs.length i →
      ¬Meets (probe u v (1 - s) ς κ) C (vtx vs j) (vtx vs (j + 1)) ∧
      ¬Meets C (probe v w s ς κ) (vtx vs j) (vtx vs (j + 1)) :=
    ev_forall_lt vs.length _ (fun j hj => by
      by_cases e : j = i ∨ j = nxt vs.length i
      · exact Ev.true.mono fun _ _ _ h1 h2 => absurd e (by grind)
      · obtain ⟨l1, l2⟩ := hlegs j hj (by grind) (by grind)
        have m1 := ev_not_meets (hs.nondeg j) l1 (smul ς (hdir u v)) ⟨0, 0⟩
        have m2 := ev_not_meets (hs.nondeg j) l2 ⟨0, 0⟩ (smul ς (hdir v w))
        refine (m1.and m2).mono fun κ _ ⟨h1, h2⟩ _ _ => ⟨?_, ?_⟩
        · rw [probe_eq, hA]; rw [addv_zero] at h1; exact h1
        · rw [probe_eq, hB]; rw [addv_zero] at h2; exact h2)
  -- on the convex side, the far probe is strictly beyond the other side's line
  have oA : ∀ κ, ς * orient v w (probe u v (1 - s) ς κ) =
      s * (ς * orient u v w) + κ * crs v w (hdir u v) := by
    intro κ
    rw [probe_eq, hA, orient_addv, orient_addv, orient_self_l, crs_smul, crs_smul]
    simp only [crs, sub, orient]; grind
  have oB : ∀ κ, ς * orient u v (probe v w s ς κ) =
      s * (ς * orient u v w) + κ * crs u v (hdir v w) := by
    intro κ
    rw [probe_eq, hB, orient_addv, orient_addv, orient_self_r, crs_smul]
    simp only [crs, sub, orient]; grind
  have k4 : Ev fun κ => 0 < ς * orient u v w →
      0 < ς * orient v w (probe u v (1 - s) ς κ) ∧ 0 < ς * orient u v (probe v w s ς κ) := by
    by_cases ht : 0 < ς * orient u v w
    · have hp := Rat.mul_pos hs0 ht
      refine ((ev_pos_of_pos (o := s * (ς * orient u v w)) (c1 := crs v w (hdir u v)) (c2 := 0) hp).and
        (ev_pos_of_pos (o := s * (ς * orient u v w)) (c1 := crs u v (hdir v w)) (c2 := 0) hp)).mono
        fun κ _ ⟨h1, h2⟩ _ => ⟨?_, ?_⟩
      · rw [oA]; grind
      · rw [oB]; grind
    · exact Ev.true.mono fun _ _ _ h => absurd h ht
  obtain ⟨κ, hκ, hall⟩ := k1.and (k2.and (k3.and k4))
  obtain ⟨w1, w2, w3, w4⟩ := hall κ hκ (by grind)
  rw [← w1, ← w2]
  -- the signs at the corner point and at the ends
  have oCuv : ς * orient u v C = s * crs u v c := by
    rw [← hC, orient_addv, orient_self_r, crs_smul]; grind
  have oCvw : ς * orient v w C = s * crs v w c := by
    rw [← hC, orient_addv, orient_self_l, crs_smul]; grind
  have pC1 := Rat.mul_pos hs0 cg.1
  have pC2 := Rat.mul_pos hs0 cg.2
  have pA := probe_side (t := 1 - s) hu hς hκ
  have pB := probe_side (t := s) hv hς hκ
  have cyc : orient v w u = orient u v w := orient_cycle u v w
  have z1 := orient_self_l u v
  have z2 := orient_self_r u v
  have z3 := orient_self_l v w
  have z4 := orient_self_r v w
  have leg1 : ∀ j < vs.length, ¬Meets (probe u v (1 - s) ς κ) C (vtx vs j) (vtx vs (j + 1)) := by
    intro j hj
    by_cases ji : j = i
    · rw [ji, hU, hV]
      exact sep (P := u) (Q := v) pA (by rw [oCuv]; exact pC1) (by rw [z1]; grind) (by rw [z2]; grind)
    · by_cases jn : j = nxt vs.length i
      · rw [jn, e1, e2]
        by_cases ht : 0 < ς * orient u v w
        · exact sep (P := v) (Q := w) (w4 ht).1 (by rw [oCvw]; exact pC2) (by rw [z3]; grind)
            (by rw [z4]; grind)
        · exact sep (P := u) (Q := v) pA (by rw [oCuv]; exact pC1) (by rw [z2]; grind) (by grind)
      · exact (w3 j hj ji jn).1
  have leg2 : ∀ j < vs.length, ¬Meets C (probe v w s ς κ) (vtx vs j) (vtx vs (j + 1)) := by
    intro j hj
    by_cases ji : j = i
    · rw [ji, hU, hV]
      by_cases ht : 0 < ς * orient u v w
      · exact sep (P := u) (Q := v) (by rw [oCuv]; exact pC1) (w4 ht).2 (by rw [z1]; grind)
          (by rw [z2]; grind)
      · exact sep (P := v) (Q := w) (by rw [oCvw]; exact pC2) pB (by rw [cyc]; grind) (by rw [z3]; grind)
    · by_cases jn : j = nxt vs.length i
      · rw [jn, e1, e2]
        exact sep (P := v) (Q := w) (by rw [oCvw]; exact pC2) pB (by rw [z3]; grind) (by rw [z4]; grind)
      · exact (w3 j hj ji jn).2
  rw [wind_eq_of_seg vs _ _ ((allMiss_iff vs _ _).mpr leg1),
    wind_eq_of_seg vs _ _ ((allMiss_iff vs _ _).mpr leg2)]

end simple

end Winding2d
