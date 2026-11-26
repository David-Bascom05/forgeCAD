import ForgeCertify.PT

/-!
# An atom's classification holds on all of it

A closed triangle soup (its edges a zero chain), a plane `P` with `n.x ≠ 0`, and two points `e`,
`w` of `P`. If every triangle of the soup either lies on `P` or is separated from the segment `e w`
by a plane (and nothing about it is tied along the segment), the soup's `+x` hit parity is the same
at `e` and at `w` (`constancy`). Each triangle changes its hit only at its edges' shadows
(`pt_general`, `pt_vert`), and over a zero chain the shadows are even in number (`chainZ`).
-/

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

theorem canon_eval (Q : Plane) (hn : Q.n ≠ ⟨0, 0, 0⟩) :
    ∃ s : Rat, s ≠ 0 ∧ ∀ x, Q.canon.eval x = Q.eval x / s := by
  unfold Plane.canon
  obtain ⟨s, hs⟩ : ∃ s, s = (if Q.n.x != 0 then Q.n.x else if Q.n.y != 0 then Q.n.y else Q.n.z) := ⟨_, rfl⟩
  have hs0 : s ≠ 0 := by
    rw [hs]
    by_cases hx : Q.n.x = 0 <;> by_cases hy : Q.n.y = 0 <;> simp [hx, hy]
    intro hz; apply hn; cases h : Q.n; simp_all
  refine ⟨s, hs0, fun x => ?_⟩
  simp only [← hs, Plane.eval, V3.dot]; simp; simp only [Rat.div_def]; grind

theorem canon_zero_iff (Q : Plane) (hn : Q.n ≠ ⟨0, 0, 0⟩) (x : V3) : Q.canon.eval x = 0 ↔ Q.eval x = 0 := by
  obtain ⟨s, hs, h⟩ := canon_eval Q hn
  rw [h x]
  constructor
  · intro h0
    have := Rat.div_mul_cancel (a := Q.eval x) hs
    rw [h0, Rat.zero_mul] at this; exact this.symm
  · intro h0; rw [h0, Rat.div_def, Rat.zero_mul]

/-- The point the sweep from the segment picks on a line. -/
def SS (e w : V3) (K : Key) (q : V3) : Prop :=
  onKey K q ∧ ∃ s l : Rat, 0 < s ∧ s < 1 ∧ 0 < l ∧ q = seg e w s + l * ex

theorem shadow_form (e w b c : V3) (h : b ≠ c) :
    shadow e w (b, c) ↔ ∃ q, SS e w (key b c) q ∧ inOpen b c q := by
  constructor
  · rintro ⟨s, μ, l, hs0, hs1, hμ0, hμ1, hl, heq⟩
    exact ⟨b + μ * (c - b), ⟨onKey_lerp h μ, s, l, hs0, hs1, hl, heq.symm⟩, μ, hμ0, hμ1, rfl⟩
  · rintro ⟨q, ⟨-, s, l, hs0, hs1, hl, rfl⟩, μ, hμ0, hμ1, heq⟩
    exact ⟨s, μ, l, hs0, hs1, hμ0, hμ1, hl, heq⟩

/-- A triangle of the soup, as the constancy needs it. -/
inductive TriOK (P : Plane) (e w : V3) (Q : Plane) (u : Tri) : Prop
  /-- on `P` itself -/
  | onP (hc : Q.canon = P) (hn : Q.n ≠ ⟨0, 0, 0⟩) (ha : Q.eval u.a = 0) (hb : Q.eval u.b = 0)
      (hc' : Q.eval u.c = 0) (hD : u.D ≠ 0)
  /-- off it, not along `x` -/
  | gen (H : Plane) (hnx : Q.n.x ≠ 0) (ha : Q.eval u.a = 0) (hb : Q.eval u.b = 0) (hc : Q.eval u.c = 0)
      (hD : u.D ≠ 0) (G : PTGen Q u e w) (S : Sep H u e w)
  /-- off it, along `x` -/
  | vert (H : Plane) (hnx : Q.n.x = 0) (hn : Q.n ≠ ⟨0, 0, 0⟩) (ha : Q.eval u.a = 0) (hb : Q.eval u.b = 0)
      (hc : Q.eval u.c = 0) (hD : u.Dv Q ≠ 0) (G : PVGen Q u e w) (S : Sep H u e w)

theorem ne_of_o2 {A B C : P2} (h : o2 A B C ≠ 0) : A ≠ B ∧ B ≠ C ∧ C ≠ A := by
  refine ⟨fun e => h ?_, fun e => h ?_, fun e => h ?_⟩ <;> rw [e] <;> unfold o2 <;> grind

/-- The triangle's vertices are distinct. -/
theorem TriOK.distinct {P Q : Plane} {e w : V3} {u : Tri} (h : TriOK P e w Q u) :
    u.a ≠ u.b ∧ u.b ≠ u.c ∧ u.c ≠ u.a := by
  have of2 : ∀ (f : V3 → P2), o2 (f u.a) (f u.b) (f u.c) ≠ 0 → u.a ≠ u.b ∧ u.b ≠ u.c ∧ u.c ≠ u.a :=
    fun f hf => by
      obtain ⟨a, b, c⟩ := ne_of_o2 hf
      exact ⟨fun e => a (e ▸ rfl), fun e => b (e ▸ rfl), fun e => c (e ▸ rfl)⟩
  cases h with
  | onP _ _ _ _ _ hD => exact of2 yz hD
  | gen _ _ _ _ _ hD => exact of2 yz hD
  | vert _ _ _ _ _ _ hD => exact of2 (piv Q) hD

/-- A point of the segment's line is `b + t (c - b)`. -/
theorem onKey_param {b c q : V3} (h : b ≠ c) (hq : onKey (key b c) q) : ∃ t : Rat, q = b + t * (c - b) := by
  obtain ⟨hb, hc⟩ := onKey_ends h
  have hne : coord (key b c).k c - coord (key b c).k b ≠ 0 := by
    have := lead_ne (c - b) (sub_ne_zero_of_ne h); rw [coord_sub] at this; exact this
  refine ⟨(coord (key b c).k q - coord (key b c).k b) / (coord (key b c).k c - coord (key b c).k b), ?_⟩
  have k := Rat.div_mul_cancel (a := coord (key b c).k q - coord (key b c).k b) hne
  have eq : q = (key b c).o + coord (key b c).k q * (key b c).d := hq
  have eb : b = (key b c).o + coord (key b c).k b * (key b c).d := hb
  have ec : c = (key b c).o + coord (key b c).k c * (key b c).d := hc
  obtain ⟨t, ht⟩ : ∃ t, t = (coord (key b c).k q - coord (key b c).k b) / (coord (key b c).k c - coord (key b c).k b) :=
    ⟨_, rfl⟩
  rw [← ht] at k ⊢
  apply v3_ext_coord; intro j
  have e1 := congrArg (coord j) eq; have e2 := congrArg (coord j) eb; have e3 := congrArg (coord j) ec
  simp only [coord_add, coord_smul, coord_sub] at e1 e2 e3 ⊢
  rw [e1, e2, e3]
  have : t * ((coord (key b c).k c - coord (key b c).k b) * coord j (key b c).d) =
      (coord (key b c).k q - coord (key b c).k b) * coord j (key b c).d := by
    rw [← Rat.mul_assoc, k]
  grind

theorem yz_lerp (b c : V3) (t : Rat) : yz (b + t * (c - b)) = lerp2 (yz b) (yz c) t := by
  simp [yz, lerp2]

/-- Two points of a line with the same yz are one, unless the line runs along `x`. -/
theorem line_yz_inj {b c : V3} (hyz : yz b ≠ yz c) {t t' : Rat}
    (h : yz (b + t * (c - b)) = yz (b + t' * (c - b))) : t = t' := by
  rw [yz_lerp, yz_lerp] at h
  simp only [lerp2, P2.mk.injEq] at h
  by_cases hu : (yz b).u = (yz c).u
  · have hv : (yz b).v ≠ (yz c).v := by intro hv; apply hyz; cases h1 : yz b; cases h2 : yz c; simp_all
    have : t * ((yz c).v - (yz b).v) = t' * ((yz c).v - (yz b).v) := by grind
    have hne : (yz c).v - (yz b).v ≠ 0 := by grind
    grind
  · have : t * ((yz c).u - (yz b).u) = t' * ((yz c).u - (yz b).u) := by grind
    have hne : (yz c).u - (yz b).u ≠ 0 := by grind
    grind

theorem yz_ex (z : V3) (l : Rat) : yz (z + l * ex) = yz z := by simp [yz, ex]; grind

/-- What `chainZ` asks of each edge of a soup triangle: the sweep picks at most one point of its
line, never an end. -/
theorem ss_props (P Q : Plane) (e w : V3) (u : Tri) (hP : P.n.x ≠ 0) (he : P.eval e = 0)
    (hw : P.eval w = 0) (h : TriOK P e w Q u) :
    ∀ E ∈ u.segs, (∀ q q', SS e w (key E.1 E.2) q → SS e w (key E.1 E.2) q' → q = q') ∧
      (∀ q, SS e w (key E.1 E.2) q → q ≠ E.1 ∧ q ≠ E.2) := by
  obtain ⟨d1, d2, d3⟩ := h.distinct
  have hseg : ∀ s, P.eval (seg e w s) = 0 := fun s => by rw [plane_seg, he, hw]; grind
  intro E hE
  have hEd : E.1 ≠ E.2 := by
    simp only [Tri.segs, List.mem_cons, List.mem_nil_iff, or_false] at hE
    rcases hE with rfl | rfl | rfl
    · exact d1
    · exact d2
    · exact d3.symm.symm
  -- the ends are vertices of `u`, on `Q`
  have hEQ : ∀ (hq : Q.eval u.a = 0 ∧ Q.eval u.b = 0 ∧ Q.eval u.c = 0), Q.eval E.1 = 0 ∧ Q.eval E.2 = 0 := by
    intro ⟨ha, hb, hc⟩
    simp only [Tri.segs, List.mem_cons, List.mem_nil_iff, or_false] at hE
    rcases hE with rfl | rfl | rfl <;> simp [ha, hb, hc]
  have hEv : E.1 = u.a ∨ E.1 = u.b ∨ E.1 = u.c := by
    simp only [Tri.segs, List.mem_cons, List.mem_nil_iff, or_false] at hE
    rcases hE with rfl | rfl | rfl <;> simp
  have hEv2 : E.2 = u.a ∨ E.2 = u.b ∨ E.2 = u.c := by
    simp only [Tri.segs, List.mem_cons, List.mem_nil_iff, or_false] at hE
    rcases hE with rfl | rfl | rfl <;> simp
  have onQ : ∀ q, (Q.eval E.1 = 0 ∧ Q.eval E.2 = 0) → onKey (key E.1 E.2) q → Q.eval q = 0 := by
    intro q ⟨h1, h2⟩ hq
    obtain ⟨t, rfl⟩ := onKey_param hEd hq
    rw [eval_lerp, h1, h2]; grind
  cases h with
  | onP hc hn ha hb hc' hD =>
    -- nothing of `P` is swept: the sweep leaves `P` at once
    have empty : ∀ q, ¬ SS e w (key E.1 E.2) q := by
      rintro q ⟨hq, s, l, -, -, hl, rfl⟩
      have h0 := onQ _ (hEQ ⟨ha, hb, hc'⟩) hq
      rw [← canon_zero_iff Q hn, hc] at h0
      have : P.eval (seg e w s + l * ex) = P.eval (seg e w s) + l * P.n.x := by
        simp only [Plane.eval, V3.dot, ex]; simp; grind
      rw [this, hseg] at h0
      rcases Rat.mul_eq_zero.mp (by grind : l * P.n.x = 0) with h | h
      · grind
      · exact hP h
    exact ⟨fun q _ h => absurd h (empty q), fun q h => absurd h (empty q)⟩
  | gen H hnx ha hb hc hD G S =>
    have vno : ∀ s, 0 < s → s < 1 → ∀ v, (v = u.a ∨ v = u.b ∨ v = u.c) → yz (seg e w s) ≠ yz v := by
      intro s h0 h1 v hv
      obtain ⟨n1, n2, n3⟩ := G.verts s h0 h1
      rcases hv with rfl | rfl | rfl <;> assumption
    refine ⟨fun q q' ⟨hq, s, l, hs0, hs1, hl, hqe⟩ ⟨hq', s', l', hs0', hs1', hl', hqe'⟩ => ?_, ?_⟩
    · obtain ⟨t, rfl⟩ := onKey_param hEd hq
      obtain ⟨t', rfl⟩ := onKey_param hEd hq'
      by_cases hyz : yz E.1 = yz E.2
      · exfalso
        have : yz (seg e w s) = yz E.1 := by
          rw [← yz_ex _ l, ← hqe, yz_lerp]; simp only [lerp2]; rw [← hyz]; cases yz E.1; simp; grind
        exact vno s hs0 hs1 E.1 hEv this
      · -- on the projected line the sweep meets it at one parameter
        have hz : ∀ s'' t'' : Rat, yz (seg e w s'') = yz (E.1 + t'' * (E.2 - E.1)) →
            o2 (yz (seg e w s'')) (yz E.1) (yz E.2) = 0 := by
          intro s'' t'' h'; rw [h', yz_lerp]; unfold o2 lerp2; grind
        have z1 := hz s t (by rw [← yz_ex _ l, ← hqe])
        have z2 := hz s' t' (by rw [← yz_ex _ l', ← hqe'])
        -- e off that line
        have hle : o2 (yz e) (yz E.1) (yz E.2) ≠ 0 := by
          obtain ⟨l1, l2, l3⟩ := G.le
          simp only [Tri.segs, List.mem_cons, List.mem_nil_iff, or_false] at hE
          rcases hE with rfl | rfl | rfl
          · intro h0; apply l3; unfold o2 at *; grind
          · exact l1
          · intro h0; apply l2; unfold o2 at *; grind
        have f1 := o2_seg1 e w s (yz E.1) (yz E.2)
        have f2 := o2_seg1 e w s' (yz E.1) (yz E.2)
        have hss : s = s' := by
          rw [z1] at f1; rw [z2] at f2
          have : (s - s') * (o2 (yz w) (yz E.1) (yz E.2) - o2 (yz e) (yz E.1) (yz E.2)) = 0 := by grind
          rcases Rat.mul_eq_zero.mp this with h | h
          · grind
          · rw [show o2 (yz w) (yz E.1) (yz E.2) = o2 (yz e) (yz E.1) (yz E.2) by grind] at f1
            grind
        subst hss
        have : yz (E.1 + t * (E.2 - E.1)) = yz (E.1 + t' * (E.2 - E.1)) := by
          rw [hqe, hqe', yz_ex, yz_ex]
        rw [line_yz_inj hyz this]
    · rintro q ⟨-, s, l, hs0, hs1, hl, rfl⟩
      constructor
      · intro h; exact vno s hs0 hs1 E.1 hEv (by rw [← h, yz_ex])
      · intro h; exact vno s hs0 hs1 E.2 hEv2 (by rw [← h, yz_ex])
  | vert H hnx hn ha hb hc hD G S =>
    have eQ := hEQ ⟨ha, hb, hc⟩
    -- the swept point is on `Q`, so its parameter is the one crossing
    have onq : ∀ (q : V3) (s l : Rat), onKey (key E.1 E.2) q → q = seg e w s + l * ex → Q.eval (seg e w s) = 0 := by
      intro q s l hq hqe
      have := onQ q eQ hq; rw [hqe, eval_ex Q hnx] at this; exact this
    have uniq : ∀ s s', Q.eval (seg e w s) = 0 → Q.eval (seg e w s') = 0 → s = s' := by
      intro s s' h1 h2
      rw [plane_seg] at h1 h2
      have : (s - s') * (Q.eval w - Q.eval e) = 0 := by grind
      rcases Rat.mul_eq_zero.mp this with h | h
      · grind
      · rw [show Q.eval w = Q.eval e by grind] at h1; exact absurd (by grind) G.qe
    have vno : ∀ s, 0 < s → s < 1 → Q.eval (seg e w s) = 0 → ∀ v, (v = u.a ∨ v = u.b ∨ v = u.c) →
        yz (seg e w s) ≠ yz v := by
      intro s h0 h1 hq v hv hyz
      obtain ⟨n1, n2, n3⟩ := G.verts s h0 h1 hq
      have : (piv Q (seg e w s)).v = (piv Q v).v := by
        unfold piv; simp only [yz, P2.mk.injEq] at hyz; split <;> simp [hyz.1, hyz.2]
      rcases hv with rfl | rfl | rfl
      · exact n1 this
      · exact n2 this
      · exact n3 this
    refine ⟨fun q q' ⟨hq, s, l, hs0, hs1, hl, hqe⟩ ⟨hq', s', l', hs0', hs1', hl', hqe'⟩ => ?_, ?_⟩
    · have hs := uniq s s' (onq q s l hq hqe) (onq q' s' l' hq' hqe')
      subst hs
      obtain ⟨t, rfl⟩ := onKey_param hEd hq
      obtain ⟨t', rfl⟩ := onKey_param hEd hq'
      by_cases hyz : yz E.1 = yz E.2
      · exfalso
        have : yz (seg e w s) = yz E.1 := by
          rw [← yz_ex _ l, ← hqe, yz_lerp]; simp only [lerp2]; rw [← hyz]; cases yz E.1; simp; grind
        exact vno s hs0 hs1 (onq _ s l hq hqe) E.1 hEv this
      · have : yz (E.1 + t * (E.2 - E.1)) = yz (E.1 + t' * (E.2 - E.1)) := by
          rw [hqe, hqe', yz_ex, yz_ex]
        rw [line_yz_inj hyz this]
    · rintro q ⟨hq, s, l, hs0, hs1, hl, hqe⟩
      have hq0 := onq q s l hq hqe
      subst hqe
      constructor
      · intro h; exact vno s hs0 hs1 hq0 E.1 hEv (by rw [← h, yz_ex])
      · intro h; exact vno s hs0 hs1 hq0 E.2 hEv2 (by rw [← h, yz_ex])

/-- Each soup triangle's hit changes by its shadows. -/
theorem TriOK.pt {P Q : Plane} {e w : V3} {u : Tri} (hP : P.n.x ≠ 0) (he : P.eval e = 0)
    (hw : P.eval w = 0) (h : TriOK P e w Q u) :
    (hit Q u e ^^ hit Q u w) = par (u.segs.map fun E => decide (shadow e w E)) := by
  cases h with
  | gen H hnx ha hb hc hD G S => exact pt_general Q H u e w hnx ha hb hc hD G S
  | vert H hnx hn ha hb hc hD G S => exact pt_vert Q H u e w hnx hn ha hb hc hD G S
  | onP hc hn ha hb hc' hD =>
    -- `e`, `w` are on `Q`: neither is hit; and no edge of `P` casts a shadow
    have hQe : Q.eval e = 0 := by rw [← canon_zero_iff Q hn, hc]; exact he
    have hQw : Q.eval w = 0 := by rw [← canon_zero_iff Q hn, hc]; exact hw
    have h0 : ∀ z, Q.eval z = 0 → hit Q u z = false := fun z hz => by
      simp [hit, ahead, show Q.c - Q.n.dot z = 0 by simp [Plane.eval] at hz; grind]
    rw [h0 e hQe, h0 w hQw]
    apply Eq.symm; apply par_all_false
    intro bm hbm
    obtain ⟨E, hE, rfl⟩ := List.mem_map.mp hbm
    simp only [decide_eq_false_iff_not]
    rintro ⟨s, μ, l, -, -, -, -, hl, heq⟩
    have hseg : P.eval (seg e w s) = 0 := by rw [plane_seg, he, hw]; grind
    have hE0 : Q.eval (E.1 + μ * (E.2 - E.1)) = 0 := by
      simp only [Tri.segs, List.mem_cons, List.mem_nil_iff, or_false] at hE
      rw [eval_lerp]; rcases hE with rfl | rfl | rfl <;> simp only [ha, hb, hc'] <;> grind
    rw [← heq, ← canon_zero_iff Q hn, hc] at hE0
    have : P.eval (seg e w s + l * ex) = P.eval (seg e w s) + l * P.n.x := by
      simp only [Plane.eval, V3.dot, ex]; simp; grind
    rw [this, hseg] at hE0
    rcases Rat.mul_eq_zero.mp (by grind : l * P.n.x = 0) with h | h
    · grind
    · exact hP h

/-- **Constancy.** A closed soup, each triangle on `P` or separated from the segment: the same hit
parity at both ends. -/
theorem constancy (P : Plane) (e w : V3) (T : List (Plane × Tri)) (hP : P.n.x ≠ 0) (he : P.eval e = 0)
    (hw : P.eval w = 0) (hz : zero1 (T.flatMap fun t => t.2.segs) = true)
    (hT : ∀ t ∈ T, TriOK P e w t.1 t.2) :
    par (T.map fun t => hit t.1 t.2 e) = par (T.map fun t => hit t.1 t.2 w) := by
  have key : (par (T.map fun t => hit t.1 t.2 e) ^^ par (T.map fun t => hit t.1 t.2 w)) = false := by
    rw [← par_map_xor]
    rw [par_map_congr T _ _ (fun t ht => (hT t ht).pt hP he hw)]
    rw [← par_flatMap]
    -- the shadows over the zero chain
    rw [par_map_congr _ _ (fun E => decide (E.1 ≠ E.2 ∧ ∃ q, SS e w (key E.1 E.2) q ∧ inOpen E.1 E.2 q))]
    · apply chainZ _ hz (SS e w)
      · intro E hE _ q q' h1 h2
        obtain ⟨t, ht, hE'⟩ := List.mem_flatMap.mp hE
        exact ((ss_props P t.1 e w t.2 hP he hw (hT t ht)) E hE').1 q q' h1 h2
      · intro E _ _ q h; exact h.1
      · intro E hE _ q h
        obtain ⟨t, ht, hE'⟩ := List.mem_flatMap.mp hE
        exact ((ss_props P t.1 e w t.2 hP he hw (hT t ht)) E hE').2 q h
    · intro E hE
      obtain ⟨t, ht, hE'⟩ := List.mem_flatMap.mp hE
      have hd : E.1 ≠ E.2 := by
        obtain ⟨d1, d2, d3⟩ := (hT t ht).distinct
        simp only [Tri.segs, List.mem_cons, List.mem_nil_iff, or_false] at hE'
        rcases hE' with rfl | rfl | rfl
        · exact d1
        · exact d2
        · exact d3
      rw [Bool.eq_iff_iff]; simp only [decide_eq_true_eq]
      rw [show E = (E.1, E.2) from rfl, shadow_form e w E.1 E.2 hd]
      simp [hd]
  revert key; cases par (T.map fun t => hit t.1 t.2 e) <;> cases par (T.map fun t => hit t.1 t.2 w) <;> simp

end ForgeCertify
