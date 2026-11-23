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

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

/-! ## The `+x` line through `x` meeting a plane -/

/-- Where the `+x` line through `x` meets the canonical plane `P` (`n.x = 1`). -/
def eP (P : Plane) (x : V3) : V3 := ⟨P.c - P.n.y * x.y - P.n.z * x.z, x.y, x.z⟩

def tpos (x : V3) (P : Plane) : Rat := (eP P x).x

theorem eval_eP (P : Plane) (h : P.n.x = 1) (x : V3) : P.eval (eP P x) = 0 := by
  simp only [Plane.eval, V3.dot, eP, h]; grind

theorem yz_eP (P : Plane) (x : V3) : yz (eP P x) = yz x := rfl

theorem ahead_x (P : Plane) (h : P.n.x = 1) (x : V3) : ahead P x = decide (x.x < tpos x P) := by
  unfold ahead tpos eP; simp only [V3.dot, h]; apply decide_eq_decide.mpr; constructor <;> intro <;> grind

theorem ahead_e (P P' : Plane) (h' : P'.n.x = 1) (x : V3) :
    ahead P' (eP P x) = decide (tpos x P < tpos x P') := by
  unfold ahead tpos eP; simp only [V3.dot, h']; apply decide_eq_decide.mpr; constructor <;> intro <;> grind

/-! ## The planes looked at -/

theorem mem_planes {ins : List Opd} {res : List Face} {P : Plane} (h : P ∈ planes ins res) :
    ∃ Q : Plane, Q.n.x ≠ 0 ∧ Q.canon = P := by
  unfold planes at h
  rw [mem_uniq] at h
  obtain ⟨Q, hQ, rfl⟩ := List.mem_map.mp h
  have := (List.mem_filter.mp hQ).2
  simp [nonvert] at this
  exact ⟨Q, this, rfl⟩

theorem planes_nx {ins : List Opd} {res : List Face} {P : Plane} (h : P ∈ planes ins res) : P.n.x = 1 := by
  obtain ⟨Q, hQ, rfl⟩ := mem_planes h; exact canon_nx Q hQ

theorem soup_in_planes {ins : List Opd} {res : List Face} {t : Plane × Tri} (ht : t ∈ allSoup ins)
    (hv : nonvert t.1 = true) : t.1.canon ∈ planes ins res := by
  unfold planes; rw [mem_uniq]
  exact List.mem_map.mpr ⟨t.1, List.mem_filter.mpr ⟨List.mem_append_left _ (List.mem_map.mpr ⟨t, ht, rfl⟩), hv⟩, rfl⟩

theorem res_in_planes {ins : List Opd} {res : List Face} {f : Face} (hf : f ∈ res)
    (hv : nonvert f.plane = true) : f.plane.canon ∈ planes ins res := by
  unfold planes; rw [mem_uniq]
  exact List.mem_map.mpr ⟨f.plane, List.mem_filter.mpr ⟨List.mem_append_right _ (List.mem_map.mpr ⟨f, hf, rfl⟩), hv⟩, rfl⟩

theorem soup_mem {ins : List Opd} {i : Nat} (hi : i < ins.length) {t : Plane × Tri}
    (ht : t ∈ (ins.getD i default).soup) : t ∈ allSoup ins := by
  unfold allSoup; exact List.mem_flatMap.mpr ⟨ins.getD i default, by simp [hi, List.getElem_mem], ht⟩

/-! ## Separation -/

theorem sepAlong_sound {d : V3} {τ u : Tri} (h : sepAlong d τ u = true) : ∃ H, sepOK H τ u = true := by
  unfold sepAlong at h
  rcases Bool.or_eq_true_iff.mp h with h | h
  · exact ⟨_, h⟩
  · exact ⟨_, h⟩

theorem separated_sound {P Q : Plane} {τ u : Tri} (h : separated P Q τ u = true) :
    ∃ H, sepOK H τ u = true := by
  unfold separated at h
  simp only [Bool.or_eq_true, List.any_eq_true] at h
  rcases h with ((((⟨d, -, h⟩ | h) | h) | ⟨s, -, h⟩) | ⟨s, -, h⟩) | ⟨s, -, s', -, h⟩ <;>
    exact sepAlong_sound h

/-- A plane nonnegative on a triangle of `P` (positive at a vertex) is positive inside it. -/
theorem pos_inside (P H : Plane) (τ : Tri) (p : V3) (hnx : P.n.x ≠ 0) (ha : P.eval τ.a = 0)
    (hb : P.eval τ.b = 0) (hc : P.eval τ.c = 0) (hp : P.eval p = 0) (hin : τ.ins p = true)
    (h0 : 0 ≤ H.eval τ.a) (h1 : 0 ≤ H.eval τ.b) (h2 : 0 ≤ H.eval τ.c)
    (h3 : 0 < H.eval τ.a ∨ 0 < H.eval τ.b ∨ 0 < H.eval τ.c) : 0 < H.eval p := by
  obtain ⟨α, β, γ, hF⟩ := affine_on_nv P H hnx
  rw [hF _ ha] at h0 h3; rw [hF _ hb] at h1 h3; rw [hF _ hc] at h2 h3; rw [hF _ hp]
  exact bary_pos _ _ _ _ α β γ hin h0 h1 h2 h3

theorem sep_of_sepOK (P H : Plane) (τ u : Tri) (e w : V3) (hnx : P.n.x ≠ 0) (ha : P.eval τ.a = 0)
    (hb : P.eval τ.b = 0) (hc : P.eval τ.c = 0) (he : P.eval e = 0) (hw : P.eval w = 0)
    (hie : τ.ins e = true) (hiw : τ.ins w = true) (h : sepOK H τ u = true) : Sep H u e w := by
  unfold sepOK at h
  simp only [Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq] at h
  obtain ⟨⟨⟨⟨⟨⟨h0, h1⟩, h2⟩, h3⟩, h4⟩, h5⟩, h6⟩ := h
  have h3' : 0 < H.eval τ.a ∨ 0 < H.eval τ.b ∨ 0 < H.eval τ.c := by
    rcases h3 with (h | h) | h
    · exact Or.inl h
    · exact Or.inr (Or.inl h)
    · exact Or.inr (Or.inr h)
  exact ⟨pos_inside P H τ e hnx ha hb hc he hie h0 h1 h2 h3', pos_inside P H τ w hnx ha hb hc hw hiw h0 h1 h2 h3',
    h4, h5, h6⟩

end ForgeCertify

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

/-! ## Atoms -/

/-- Three points on a line through two distinct points make no area. -/
theorem collinear_zero (s t a b c : P2) (hst : s ≠ t) (ha : o2 s t a = 0) (hb : o2 s t b = 0)
    (hc : o2 s t c = 0) : o2 a b c = 0 := by
  apply Classical.byContradiction; intro hD
  -- the side function of the line is affine and vanishes at the three vertices: so everywhere
  have key : ∀ p, o2 s t p = 0 := by
    intro p
    have e := bary_affine a b c p (-(t.v - s.v)) (t.u - s.u) (-(t.u - s.u) * s.v + (t.v - s.v) * s.u)
    rw [← o2_affine, ← o2_affine, ← o2_affine, ← o2_affine, ha, hb, hc] at e
    have : o2 a b c * o2 s t p = 0 := by grind
    rcases Rat.mul_eq_zero.mp this with h | h
    · exact absurd h hD
    · exact h
  have := key ⟨s.u - (t.v - s.v), s.v + (t.u - s.u)⟩
  unfold o2 at this
  apply hst
  have h2 : (t.u - s.u) * (t.u - s.u) + (t.v - s.v) * (t.v - s.v) = 0 := by grind
  have p1 := mul_self_nonneg' (t.u - s.u); have p2 := mul_self_nonneg' (t.v - s.v)
  have e1 : (t.u - s.u) * (t.u - s.u) = 0 := by grind
  have e2 : (t.v - s.v) * (t.v - s.v) = 0 := by grind
  have := Rat.mul_eq_zero.mp e1; have := Rat.mul_eq_zero.mp e2
  cases s; cases t; simp only [P2.mk.injEq]; constructor <;> grind
where
  mul_self_nonneg' (a : Rat) : 0 ≤ a * a := by
    rcases lt_trichotomy' a 0 with h | h | h
    · exact Rat.le_of_lt (mul_pos_of_neg_of_neg h h)
    · rw [h]; grind
    · exact Rat.le_of_lt (Rat.mul_pos h h)

/-- `sep2` keeps two proper triangles' interiors apart. -/
theorem sep2_sound (τ σ : Tri) (x : V3) (hτ : τ.D ≠ 0) (hσ : σ.D ≠ 0) (h : sep2 τ σ = true)
    (h1 : τ.ins x = true) (h2 : σ.ins x = true) : False := by
  unfold sep2 at h
  simp only [List.any_eq_true, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq] at h
  obtain ⟨s, hs, hsep⟩ := h
  -- the line's two points are distinct, being a proper triangle's
  have hst : yz s.1 ≠ yz s.2 := by
    obtain ⟨n1, n2, n3⟩ := yz_ne_of_D hτ
    obtain ⟨m1, m2, m3⟩ := yz_ne_of_D hσ
    simp only [List.mem_append, Tri.segs, List.mem_cons, List.mem_nil_iff, or_false] at hs
    rcases hs with (rfl | rfl | rfl) | (rfl | rfl | rfl) <;> assumption
  -- neither triangle lies on the line
  have notall : ∀ t : Tri, t.D ≠ 0 → ¬ (o2 (yz s.1) (yz s.2) (yz t.a) = 0 ∧ o2 (yz s.1) (yz s.2) (yz t.b) = 0 ∧
      o2 (yz s.1) (yz s.2) (yz t.c) = 0) := by
    intro t ht ⟨a, b, c⟩; exact ht (collinear_zero _ _ _ _ _ hst a b c)
  have aff := fun r => o2_affine (yz s.1) (yz s.2) (yz r)
  rcases hsep with ⟨⟨⟨⟨⟨a1, a2⟩, a3⟩, b1⟩, b2⟩, b3⟩ | ⟨⟨⟨⟨⟨a1, a2⟩, a3⟩, b1⟩, b2⟩, b3⟩
  · have p := bary_pos _ _ _ _ _ _ _ h1 (by rw [← aff]; exact a1) (by rw [← aff]; exact a2) (by rw [← aff]; exact a3)
      (by
        rcases lt_trichotomy' 0 (o2 (yz s.1) (yz s.2) (yz τ.a)) with q | q | q
        · left; rw [← aff]; exact q
        rcases lt_trichotomy' 0 (o2 (yz s.1) (yz s.2) (yz τ.b)) with r | r | r
        · right; left; rw [← aff]; exact r
        rcases lt_trichotomy' 0 (o2 (yz s.1) (yz s.2) (yz τ.c)) with w | w | w
        · right; right; rw [← aff]; exact w
        all_goals first | (exfalso; exact notall τ hτ ⟨q.symm, r.symm, w.symm⟩) | (exfalso; grind))
    have n := bary_neg _ _ _ _ _ _ _ h2 (by rw [← aff]; exact b1) (by rw [← aff]; exact b2) (by rw [← aff]; exact b3)
      (by
        rcases lt_trichotomy' (o2 (yz s.1) (yz s.2) (yz σ.a)) 0 with q | q | q
        · left; rw [← aff]; exact q
        rcases lt_trichotomy' (o2 (yz s.1) (yz s.2) (yz σ.b)) 0 with r | r | r
        · right; left; rw [← aff]; exact r
        rcases lt_trichotomy' (o2 (yz s.1) (yz s.2) (yz σ.c)) 0 with w | w | w
        · right; right; rw [← aff]; exact w
        all_goals first | (exfalso; exact notall σ hσ ⟨q, r, w⟩) | (exfalso; grind))
    grind
  · have n := bary_neg _ _ _ _ _ _ _ h1 (by rw [← aff]; exact a1) (by rw [← aff]; exact a2) (by rw [← aff]; exact a3)
      (by
        rcases lt_trichotomy' (o2 (yz s.1) (yz s.2) (yz τ.a)) 0 with q | q | q
        · left; rw [← aff]; exact q
        rcases lt_trichotomy' (o2 (yz s.1) (yz s.2) (yz τ.b)) 0 with r | r | r
        · right; left; rw [← aff]; exact r
        rcases lt_trichotomy' (o2 (yz s.1) (yz s.2) (yz τ.c)) 0 with w | w | w
        · right; right; rw [← aff]; exact w
        all_goals first | (exfalso; exact notall τ hτ ⟨q, r, w⟩) | (exfalso; grind))
    have p := bary_pos _ _ _ _ _ _ _ h2 (by rw [← aff]; exact b1) (by rw [← aff]; exact b2) (by rw [← aff]; exact b3)
      (by
        rcases lt_trichotomy' 0 (o2 (yz s.1) (yz s.2) (yz σ.a)) with q | q | q
        · left; rw [← aff]; exact q
        rcases lt_trichotomy' 0 (o2 (yz s.1) (yz s.2) (yz σ.b)) with r | r | r
        · right; left; rw [← aff]; exact r
        rcases lt_trichotomy' 0 (o2 (yz s.1) (yz s.2) (yz σ.c)) with w | w | w
        · right; right; rw [← aff]; exact w
        all_goals first | (exfalso; exact notall σ hσ ⟨q.symm, r.symm, w.symm⟩) | (exfalso; grind))
    grind

theorem pairwise_two {α : Type} {R : α → α → Prop} : ∀ {l : List α}, l.Pairwise R → ∀ {a b : α},
    a ∈ l → b ∈ l → a ≠ b → R a b ∨ R b a
  | [], _, _, _, ha, _, _ => absurd ha (List.not_mem_nil)
  | c :: l, h, a, b, ha, hb, hab => by
    obtain ⟨hc, hl⟩ := List.pairwise_cons.mp h
    rcases List.mem_cons.mp ha with ea | ha' <;> rcases List.mem_cons.mp hb with eb | hb'
    · exact absurd (ea.trans eb.symm) hab
    · rw [ea]; exact Or.inl (hc b hb')
    · rw [eb]; exact Or.inr (hc a ha')
    · exact pairwise_two hl ha' hb' hab

theorem mem_atoms {ins : List Opd} {P : Plane} {t : Plane × Tri} (ht : t ∈ allSoup ins) (hp : onP P t = true) :
    canonTri t.2 ∈ atoms ins P := by
  unfold atoms; rw [mem_uniq]
  exact List.mem_map.mpr ⟨t, List.mem_filter.mpr ⟨ht, hp⟩, rfl⟩

theorem atoms_mem {ins : List Opd} {P : Plane} {τ : Tri} (h : τ ∈ atoms ins P) :
    ∃ t ∈ allSoup ins, onP P t = true ∧ canonTri t.2 = τ := by
  unfold atoms at h; rw [mem_uniq] at h
  obtain ⟨t, ht, rfl⟩ := List.mem_map.mp h
  exact ⟨t, (List.mem_filter.mp ht).1, (List.mem_filter.mp ht).2, rfl⟩

end ForgeCertify

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

theorem nonvert_ne {Q : Plane} (h : nonvert Q = true) : Q.n ≠ ⟨0, 0, 0⟩ := by
  intro e; simp [nonvert, e] at h

/-- At most one atom holds `x`, and a triangle of the plane holds `x` iff its atom does. -/
theorem atom_lookup (ins : List Opd) (P : Plane) (x : V3) (hA : atomsOK (atoms ins P) = true)
    (hD : ∀ τ ∈ atoms ins P, τ.D ≠ 0) (t : Plane × Tri) (ht : t ∈ allSoup ins) (hp : onP P t = true) :
    (∀ τ₀ ∈ atoms ins P, τ₀.ins x = true → t.2.ins x = decide (canonTri t.2 = τ₀)) ∧
      ((∀ τ ∈ atoms ins P, τ.ins x = false) → t.2.ins x = false) := by
  have hm := mem_atoms ht hp
  rw [← ins_canonTri t.2 x]
  refine ⟨fun τ₀ h0 hin => ?_, fun hno => hno _ hm⟩
  by_cases e : canonTri t.2 = τ₀
  · rw [e, hin]; simp
  · simp only [e, decide_false]
    unfold atomsOK at hA
    have hpw := of_decide_eq_true hA
    apply Classical.byContradiction; intro hn
    have hin' : (canonTri t.2).ins x = true := by simpa using hn
    rcases pairwise_two hpw hm h0 e with h | h
    · exact sep2_sound _ _ x (hD _ hm) (hD _ h0) h hin' hin
    · exact sep2_sound _ _ x (hD _ h0) (hD _ hm) h hin hin'

/-! ## Witnesses -/

theorem wpoint_props (P : Plane) (τ : Tri) (k : Nat × Nat × Nat) (h1 : 0 < k.1) (h2 : 0 < k.2.1)
    (h3 : 0 < k.2.2) (ha : P.eval τ.a = 0) (hb : P.eval τ.b = 0) (hc : P.eval τ.c = 0) (hD : τ.D ≠ 0) :
    P.eval (wpoint τ k) = 0 ∧ τ.ins (wpoint τ k) = true := by
  obtain ⟨k1, k2, k3⟩ := k
  simp only at h1 h2 h3
  have hK : ((k1 + k2 + k3 : Nat) : Rat) ≠ 0 := by
    have : 0 < k1 + k2 + k3 := by omega
    exact_mod_cast (Nat.pos_iff_ne_zero.mp this)
  obtain ⟨r, hr⟩ : ∃ r : Rat, r = 1 / ((k1 + k2 + k3 : Nat) : Rat) := ⟨_, rfl⟩
  have hrK : r * ((k1 : Rat) + k2 + k3) = 1 := by
    rw [hr]; have := Rat.div_mul_cancel (a := 1) hK; push_cast at this ⊢; exact this
  have hw : wpoint τ (k1, k2, k3) = r * ((k1 : Rat) * τ.a + (k2 : Rat) * τ.b + (k3 : Rat) * τ.c) := by
    unfold wpoint; rw [hr]
  rw [hw]
  constructor
  · simp only [Plane.eval, V3.dot] at ha hb hc ⊢; simp; grind
  · simp only [Tri.ins, inside2, Bool.and_eq_true, decide_eq_true_eq]
    unfold Tri.D at hD
    have q1 : (0 : Rat) < k1 := by exact_mod_cast h1
    have q2 : (0 : Rat) < k2 := by exact_mod_cast h2
    have q3 : (0 : Rat) < k3 := by exact_mod_cast h3
    have hr0 : 0 < r := by
      apply Classical.byContradiction; intro hn
      have : r * ((k1 : Rat) + k2 + k3) ≤ 0 := by
        have := Rat.mul_le_mul_of_nonneg_right (a := r) (b := 0) (by grind) (c := (k1 : Rat) + k2 + k3) (by grind)
        grind
      grind
    have e1 : o2 (yz (r * ((k1 : Rat) * τ.a + (k2 : Rat) * τ.b + (k3 : Rat) * τ.c))) (yz τ.b) (yz τ.c) =
        (r * k1) * o2 (yz τ.a) (yz τ.b) (yz τ.c) := by unfold o2 yz; simp; grind
    have e2 : o2 (yz τ.a) (yz (r * ((k1 : Rat) * τ.a + (k2 : Rat) * τ.b + (k3 : Rat) * τ.c))) (yz τ.c) =
        (r * k2) * o2 (yz τ.a) (yz τ.b) (yz τ.c) := by unfold o2 yz; simp; grind
    have e3 : o2 (yz τ.a) (yz τ.b) (yz (r * ((k1 : Rat) * τ.a + (k2 : Rat) * τ.b + (k3 : Rat) * τ.c))) =
        (r * k3) * o2 (yz τ.a) (yz τ.b) (yz τ.c) := by unfold o2 yz; simp; grind
    rw [e1, e2, e3]
    have hD2 := sq_pos' hD
    refine ⟨⟨?_, ?_⟩, ?_⟩
    · have := Rat.mul_pos hD2 (Rat.mul_pos hr0 q1); grind
    · have := Rat.mul_pos hD2 (Rat.mul_pos hr0 q2); grind
    · have := Rat.mul_pos hD2 (Rat.mul_pos hr0 q3); grind

theorem witness_props (ins : List Opd) (P : Plane) (τ : Tri) (ha : P.eval τ.a = 0) (hb : P.eval τ.b = 0)
    (hc : P.eval τ.c = 0) (hD : τ.D ≠ 0) :
    P.eval (witness ins P τ) = 0 ∧ τ.ins (witness ins P τ) = true := by
  unfold witness
  dsimp only
  split
  · rename_i k hk
    have hm := List.mem_of_find?_eq_some hk
    have : 0 < k.1 ∧ 0 < k.2.1 ∧ 0 < k.2.2 := by
      simp only [wcands, List.mem_cons, List.mem_nil_iff, or_false] at hm
      rcases hm with h | h | h | h | h | h | h | h | h | h | h | h | h <;> subst h <;> decide
    exact wpoint_props P τ k this.1 this.2.1 this.2.2 ha hb hc hD
  · have e : centroid3 τ = wpoint τ (1, 1, 1) := by
      unfold centroid3 wpoint; apply v3_ext <;> simp <;> grind
    rw [e]; exact wpoint_props P τ _ (by decide) (by decide) (by decide) ha hb hc hD

end ForgeCertify

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

/-- A face's crossing parity at `x`. -/
def fcross (f : Face) (x : V3) : Bool := par ((faceSegs f).map fun s => crossesYZ x s.1 s.2)

/-- The result's faces, grouped by plane. -/
theorem res_group (res : List Face) (L : List Plane) (hL : L.Nodup) (x : V3)
    (hcov : ∀ f ∈ res, nonvert f.plane = true → f.plane.canon ∈ L) :
    par (res.map fun f => f.pierced x) =
      par (L.map fun P => ahead P x && par ((resOn res P).map fun f => fcross f x)) := by
  have h1 : par (res.map fun f => f.pierced x) = par ((res.filter fun f => nonvert f.plane).map fun f => f.pierced x) := by
    rw [par_filter]; apply par_map_congr; intro f _
    cases h : nonvert f.plane
    · simp [nonvert] at h; simp [Face.pierced, h]
    · rfl
  rw [h1, par_group _ (fun f => f.plane.canon) L hL (fun f hf => hcov f (List.mem_filter.mp hf).1 (List.mem_filter.mp hf).2)]
  apply par_map_congr; intro P _
  rw [List.filter_filter]
  have : ∀ f ∈ (res.filter fun f => decide (f.plane.canon = P) && nonvert f.plane),
      f.pierced x = (ahead P x && fcross f x) := by
    intro f hf
    obtain ⟨-, hc⟩ := List.mem_filter.mp hf
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hc
    rw [pierced_eq, ← hc.1, ahead_canon _ (nonvert_ne hc.2)]; rfl
  rw [par_map_congr _ _ _ this]
  have e : (res.filter fun f => decide (f.plane.canon = P) && nonvert f.plane) = resOn res P := by
    unfold resOn; apply List.filter_congr; intro f _
    by_cases h : f.plane.canon = P <;> simp [h, Bool.and_comm]
  rw [e, par_and_const]

/-- On one plane, the result's crossing at `x` is the parity of the atoms holding `x` where the
boolean jumps. -/
theorem result_on (op : List Bool → Bool) (ins : List Opd) (res : List Face) (P : Plane) (x : V3)
    (h : resultOK op ins res P (atoms ins P) = true) (hD : ∀ τ ∈ atoms ins P, τ.D ≠ 0)
    (g : ∀ s ∈ (resOn res P).flatMap faceSegs ++ (atoms ins P).flatMap Tri.segs, GenSeg x s) :
    par ((resOn res P).map fun f => fcross f x) =
      par (((atoms ins P).filter (jump op ins P)).map fun τ => τ.ins x) := by
  unfold resultOK at h
  have g' : ∀ s ∈ (resOn res P).flatMap faceSegs ++ ((atoms ins P).filter (jump op ins P)).flatMap Tri.segs,
      GenSeg x s := by
    intro s hs
    rcases List.mem_append.mp hs with hs | hs
    · exact g s (List.mem_append_left _ hs)
    · obtain ⟨τ, hτ, hs⟩ := List.mem_flatMap.mp hs
      exact g s (List.mem_append_right _ (List.mem_flatMap.mpr ⟨τ, (List.mem_filter.mp hτ).1, hs⟩))
  have h0 := cross_zero x _ h g'
  rw [List.map_append, par_append, par_flatMap, par_flatMap] at h0
  have e : par (((atoms ins P).filter (jump op ins P)).map fun τ => par ((Tri.segs τ).map fun s => crossesYZ x s.1 s.2)) =
      par (((atoms ins P).filter (jump op ins P)).map fun τ => τ.ins x) := by
    apply par_map_congr; intro τ hτ
    have hτ' := (List.mem_filter.mp hτ).1
    exact tri_cross τ x (hD τ hτ') (fun s hs => g s (List.mem_append_right _ (List.mem_flatMap.mpr ⟨τ, hτ', hs⟩)))
  rw [e] at h0
  unfold fcross
  revert h0
  cases par ((resOn res P).map fun f => par ((faceSegs f).map fun s => crossesYZ x s.1 s.2)) <;>
    cases par (((atoms ins P).filter (jump op ins P)).map fun τ => τ.ins x) <;> simp

end ForgeCertify

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

/-! ## What `tilesOK` says of each soup triangle -/

theorem soup_ok {o : Opd} (ht : tilesOK o = true) {t : Plane × Tri} (h : t ∈ o.soup) :
    t.1.n ≠ ⟨0, 0, 0⟩ ∧ onPlane t.1 t.2 = true ∧ proper t.1 t.2 = true := by
  unfold tilesOK at ht
  simp only [Bool.and_eq_true, beq_iff_eq, List.all_eq_true] at ht
  unfold Opd.soup at h
  obtain ⟨ft, hft, hm⟩ := List.mem_flatMap.mp h
  obtain ⟨u, hu, rfl⟩ := List.mem_map.mp hm
  obtain ⟨⟨hn, htri⟩, -⟩ := ht.2 ft hft
  refine ⟨?_, (htri u hu).1, (htri u hu).2⟩
  intro e; simp only at e; rw [e] at hn; simp [nonzero] at hn

theorem allSoup_ok {ins : List Opd} (ht : ins.all tilesOK = true) {t : Plane × Tri} (h : t ∈ allSoup ins) :
    t.1.n ≠ ⟨0, 0, 0⟩ ∧ onPlane t.1 t.2 = true ∧ proper t.1 t.2 = true := by
  unfold allSoup at h
  obtain ⟨o, ho, h⟩ := List.mem_flatMap.mp h
  exact soup_ok (List.all_eq_true.mp ht o ho) h

theorem onPlane_iff (Q : Plane) (u : Tri) :
    onPlane Q u = true ↔ Q.eval u.a = 0 ∧ Q.eval u.b = 0 ∧ Q.eval u.c = 0 := by
  unfold onPlane; simp [Bool.and_eq_true, beq_iff_eq, and_assoc]

theorem proper_nv {Q : Plane} {u : Tri} (h : proper Q u = true) (hv : nonvert Q = true) : u.D ≠ 0 := by
  unfold proper at h; unfold nonvert at hv; rw [hv] at h; revert h; simp

theorem proper_v {Q : Plane} {u : Tri} (h : proper Q u = true) (hv : nonvert Q = false) : u.Dv Q ≠ 0 := by
  unfold proper at h; unfold nonvert at hv; rw [hv] at h; revert h; simp

/-- An atom's vertices are on its plane, and it is proper. -/
theorem atom_ok {ins : List Opd} (ht : ins.all tilesOK = true) {P : Plane} {τ : Tri} (h : τ ∈ atoms ins P) :
    P.eval τ.a = 0 ∧ P.eval τ.b = 0 ∧ P.eval τ.c = 0 ∧ τ.D ≠ 0 := by
  obtain ⟨t, htm, hp, rfl⟩ := atoms_mem h
  obtain ⟨hn, hon, hpr⟩ := allSoup_ok ht htm
  unfold onP at hp; simp only [Bool.and_eq_true, decide_eq_true_eq] at hp
  obtain ⟨hv, hc⟩ := hp
  rw [onPlane_iff] at hon
  have on : ∀ v, (v = t.2.a ∨ v = t.2.b ∨ v = t.2.c) → P.eval v = 0 := by
    intro v hv'; rw [← hc, canon_zero_iff _ hn]; rcases hv' with rfl | rfl | rfl <;> simp [hon]
  obtain ⟨va, vb, vc⟩ := canonTri_verts t.2
  refine ⟨on _ va, on _ vb, on _ vc, ?_⟩
  rcases canonTri_D t.2 with e | e <;> rw [e]
  · exact proper_nv hpr hv
  · have := proper_nv hpr hv; intro h0; apply this; grind

/-! ## Vectors -/

theorem vec_map {α : Type} [Inhabited α] (l : List α) (f : α → Bool) :
    l.map f = vec l.length fun i => f (l.getD i default) := by
  unfold vec
  apply List.ext_getElem (by simp)
  intro i h1 h2
  simp at h1; simp [h1]

theorem vec_congr (n : Nat) (f g : Nat → Bool) (h : ∀ i < n, f i = g i) : vec n f = vec n g := by
  unfold vec; apply List.map_congr_left; intro i hi; exact h i (List.mem_range.mp hi)

theorem vec_zip (n : Nat) (f g : Nat → Bool) :
    List.zipWith (· ^^ ·) (vec n f) (vec n g) = vec n fun i => f i ^^ g i := by
  unfold vec
  apply List.ext_getElem (by simp)
  intro i h1 h2
  simp

end ForgeCertify

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

/-! ## Nothing tied at `x` -/

/-- Every segment the checker reads. -/
def allSegs (ins : List Opd) (res : List Face) : List Seg :=
  ins.flatMap Opd.segs ++ res.flatMap faceSegs ++
    (planes ins res).flatMap fun P => (atoms ins P).flatMap Tri.segs

/-- `x` is generic: every crossing along the `+x` line through it, and every sweep from where that
line meets a plane to an atom's witness, is untied. -/
structure Gen (ins : List Opd) (res : List Face) (x : V3) : Prop where
  gseg : ∀ s ∈ allSegs ins res, GenSeg x s
  pos : ∀ P ∈ planes ins res, ∀ P' ∈ planes ins res, tpos x P = tpos x P' → P = P'
  qe : ∀ P ∈ planes ins res, ∀ t ∈ allSoup ins, offP P t = true → t.1.eval (eP P x) ≠ 0
  verts : ∀ P ∈ planes ins res, ∀ τ ∈ atoms ins P, ∀ t ∈ allSoup ins, offP P t = true →
    ∀ s, 0 < s → s < 1 → yz (seg (eP P x) (witness ins P τ) s) ≠ yz t.2.a ∧
      yz (seg (eP P x) (witness ins P τ) s) ≠ yz t.2.b ∧ yz (seg (eP P x) (witness ins P τ) s) ≠ yz t.2.c
  vverts : ∀ P ∈ planes ins res, ∀ τ ∈ atoms ins P, ∀ t ∈ allSoup ins, offP P t = true → nonvert t.1 = false →
    ∀ s, 0 < s → s < 1 → t.1.eval (seg (eP P x) (witness ins P τ) s) = 0 →
      (piv t.1 (seg (eP P x) (witness ins P τ) s)).v ≠ (piv t.1 t.2.a).v ∧
      (piv t.1 (seg (eP P x) (witness ins P τ) s)).v ≠ (piv t.1 t.2.b).v ∧
      (piv t.1 (seg (eP P x) (witness ins P τ) s)).v ≠ (piv t.1 t.2.c).v

theorem soup_segs {o : Opd} {t : Plane × Tri} (h : t ∈ o.soup) : ∀ s ∈ t.2.segs, s ∈ o.segs := by
  intro s hs
  unfold Opd.soup at h
  obtain ⟨ft, hft, hm⟩ := List.mem_flatMap.mp h
  obtain ⟨u, hu, rfl⟩ := List.mem_map.mp hm
  exact List.mem_flatMap.mpr ⟨ft, hft, List.mem_append_right _ (List.mem_flatMap.mpr ⟨u, hu, hs⟩)⟩

theorem allSoup_segs {ins : List Opd} {res : List Face} {t : Plane × Tri} (h : t ∈ allSoup ins) :
    ∀ s ∈ t.2.segs, s ∈ allSegs ins res := by
  intro s hs
  unfold allSoup at h
  obtain ⟨o, ho, h⟩ := List.mem_flatMap.mp h
  exact List.mem_append_left _ (List.mem_append_left _ (List.mem_flatMap.mpr ⟨o, ho, soup_segs h s hs⟩))

/-- The edge lines of a proper triangle miss `x`'s projection, in each of the forms `PTGen` asks. -/
theorem le_of_gen {u : Tri} {x : V3} (hD : u.D ≠ 0) (g : ∀ s ∈ u.segs, GenSeg x s) :
    o2 (yz x) (yz u.b) (yz u.c) ≠ 0 ∧ o2 (yz u.a) (yz x) (yz u.c) ≠ 0 ∧ o2 (yz u.a) (yz u.b) (yz x) ≠ 0 := by
  obtain ⟨n1, n2, n3⟩ := yz_ne_of_D hD
  have g1 := (g (u.a, u.b) (by simp [Tri.segs])).2.2 n1
  have g2 := (g (u.b, u.c) (by simp [Tri.segs])).2.2 n2
  have g3 := (g (u.c, u.a) (by simp [Tri.segs])).2.2 n3
  refine ⟨g2, fun h => g3 ?_, fun h => g1 ?_⟩ <;> unfold o2 at * <;> grind

end ForgeCertify

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

theorem witnessOK_sound {w : V3} {Q : Plane} {u : Tri} (h : witnessOK w Q u = true) :
    Q.eval w ≠ 0 ∧ (Q.n.x ≠ 0 → o2 (yz w) (yz u.b) (yz u.c) ≠ 0 ∧ o2 (yz u.a) (yz w) (yz u.c) ≠ 0 ∧
      o2 (yz u.a) (yz u.b) (yz w) ≠ 0) := by
  unfold witnessOK at h
  simp only [Bool.and_eq_true, Bool.or_eq_true, bne_iff_ne, ne_eq, beq_iff_eq] at h
  obtain ⟨⟨⟨⟨h1, -⟩, -⟩, -⟩, h5⟩ := h
  refine ⟨h1, fun hx => ?_⟩
  rcases h5 with h5 | ⟨⟨a, b⟩, c⟩
  · exact absurd h5 hx
  · exact ⟨a, b, c⟩

/-- Every soup triangle meets the constancy's needs on the segment from where the `+x` line through
`x` meets an atom's plane to the atom's witness. -/
theorem triOK_of (op : List Bool → Bool) (ins : List Opd) (res : List Face) (x : V3)
    (ht : ins.all tilesOK = true) (G : Gen ins res x) (P : Plane) (hP : P ∈ planes ins res) (τ : Tri)
    (hτ : τ ∈ atoms ins P) (hin : τ.ins x = true) (hok : atomOK ins P τ = true)
    (t : Plane × Tri) (htm : t ∈ allSoup ins) :
    TriOK P (eP P x) (witness ins P τ) t.1 t.2 := by
  have hPx := planes_nx hP
  obtain ⟨ta, tb, tc, tD⟩ := atom_ok ht hτ
  obtain ⟨hw, hiw⟩ := witness_props ins P τ ta tb tc tD
  have he := eval_eP P hPx x
  have hie : τ.ins (eP P x) = true := hin
  obtain ⟨hn, hon, hpr⟩ := allSoup_ok ht htm
  rw [onPlane_iff] at hon
  obtain ⟨ha, hb, hc⟩ := hon
  have g : ∀ s ∈ t.2.segs, GenSeg x s := fun s hs => G.gseg s (allSoup_segs htm s hs)
  by_cases hp : onP P t = true
  · unfold onP at hp; simp only [Bool.and_eq_true, decide_eq_true_eq] at hp
    exact TriOK.onP hp.2 hn ha hb hc (proper_nv hpr hp.1)
  · have hoff : offP P t = true := by rw [offP_onP]; simpa using hp
    unfold atomOK at hok
    simp only [List.all_eq_true] at hok
    have := hok t htm
    rw [hoff] at this
    simp only [Bool.not_true, Bool.false_or, Bool.and_eq_true] at this
    obtain ⟨hwok, hsep⟩ := this
    obtain ⟨H, hH⟩ := separated_sound hsep
    have S := sep_of_sepOK P H τ t.2 (eP P x) (witness ins P τ) (by grind) ta tb tc he hw hie hiw hH
    obtain ⟨qw, lw⟩ := witnessOK_sound hwok
    cases hv : nonvert t.1
    · have hnx : t.1.n.x = 0 := by simp [nonvert] at hv; exact hv
      exact TriOK.vert H hnx hn ha hb hc (proper_v hpr hv)
        ⟨G.qe P hP t htm hoff, qw, fun s h0 h1 hq => G.vverts P hP τ hτ t htm hoff hv s h0 h1 hq⟩ S
    · have hnx : t.1.n.x ≠ 0 := by simp [nonvert] at hv; exact hv
      have hD := proper_nv hpr hv
      exact TriOK.gen H hnx ha hb hc hD
        ⟨G.qe P hP t htm hoff, qw, le_of_gen hD g, lw hnx, fun s h0 h1 => G.verts P hP τ hτ t htm hoff s h0 h1⟩ S

end ForgeCertify

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

theorem hit_on (Q : Plane) (u : Tri) (z : V3) (h : Q.eval z = 0) : hit Q u z = false := by
  simp only [Plane.eval] at h
  simp [hit, ahead, show Q.c - Q.n.dot z = 0 by grind]

/-- The flips of operand `i` on the plane `P'`, at `x`. -/
def alpha (ins : List Opd) (x : V3) (P' : Plane) (i : Nat) : Bool :=
  par (((ins.getD i default).soup.filter (onP P')).map fun t => t.2.ins x)

/-- The witness's view beyond the atom is what lies beyond the line's point on the plane. -/
theorem beyond_eq (op : List Bool → Bool) (ins : List Opd) (res : List Face) (x : V3)
    (ht : ins.all tilesOK = true) (hcl : ins.all closedOK = true) (G : Gen ins res x) (P : Plane)
    (hP : P ∈ planes ins res) (τ : Tri) (hτ : τ ∈ atoms ins P) (hin : τ.ins x = true)
    (hok : atomOK ins P τ = true) (i : Nat) (hi : i < ins.length) :
    beyond ins P (witness ins P τ) i =
      par ((planes ins res).map fun P' => decide (tpos x P < tpos x P') && alpha ins x P' i) := by
  have hPx := planes_nx hP
  obtain ⟨ta, tb, tc, tD⟩ := atom_ok ht hτ
  obtain ⟨hw, -⟩ := witness_props ins P τ ta tb tc tD
  have he := eval_eP P hPx x
  let T := (ins.getD i default).soup
  have hTm : ∀ t ∈ T, t ∈ allSoup ins := fun t h => soup_mem hi h
  have htiles : tilesOK (ins.getD i default) = true := by
    have := List.all_eq_true.mp ht (ins.getD i default) (by simp [hi, List.getElem_mem]); exact this
  have hclosed : closedOK (ins.getD i default) = true := by
    have := List.all_eq_true.mp hcl (ins.getD i default) (by simp [hi, List.getElem_mem]); exact this
  -- the plane's own triangles are never hit from a point of it
  have h1 : beyond ins P (witness ins P τ) i = par (T.map fun t => hit t.1 t.2 (witness ins P τ)) := by
    unfold beyond; rw [par_filter]; apply par_map_congr; intro t htm
    by_cases hp : onP P t = true
    · have : offP P t = false := by rw [offP_onP]; simpa using hp
      rw [this]
      unfold onP at hp; simp only [Bool.and_eq_true, decide_eq_true_eq] at hp
      have : t.1.eval (witness ins P τ) = 0 := by
        rw [← canon_zero_iff _ (allSoup_ok ht (hTm t htm)).1, hp.2]; exact hw
      rw [hit_on _ _ _ this]; rfl
    · have : offP P t = true := by rw [offP_onP]; simpa using hp
      rw [this]; rfl
  rw [h1]
  -- constancy: the same at the line's point
  have hc := constancy P (eP P x) (witness ins P τ) T (by grind) he hw hclosed
    (fun t htm => triOK_of op ins res x ht G P hP τ hτ hin hok t (hTm t htm))
  rw [← hc]
  rw [soup_group T (planes ins res) (nodup_uniq _) (eP P x) (fun t htm => (allSoup_ok ht (hTm t htm)).1)
    (fun t htm hv => soup_in_planes (hTm t htm) hv)]
  apply par_map_congr; intro P' hP'
  rw [ahead_e P P' (planes_nx hP') x]
  rfl

end ForgeCertify

namespace ForgeCertify

open ForgeExact.Ref

attribute [local instance] Classical.propDecidable

/-- **Soundness at a generic point.** -/
theorem at_generic (op : List Bool → Bool) (ins : List Opd) (res : List Face) (hc : check op ins res = true)
    (x : V3) (G : Gen ins res x) :
    (Solid.mk "" res.toArray).contains x = op (ins.map fun o => (Solid.mk "" o.faces.toArray).contains x) := by
  unfold check at hc
  simp only [Bool.and_eq_true, beq_iff_eq] at hc
  obtain ⟨⟨⟨h0, ht⟩, hcl⟩, hpl⟩ := hc
  rw [List.all_eq_true] at hpl
  let L := planes ins res
  have hL : L.Nodup := nodup_uniq _
  let n := ins.length
  let A := fun (P : Plane) (i : Nat) => par (L.map fun P' => decide (tpos x P < tpos x P') && alpha ins x P' i)
  -- the operands: parities over the planes ahead
  have hins : ins.map (fun o => (Solid.mk "" o.faces.toArray).contains x) =
      vec n fun i => par (L.map fun P => decide (x.x < tpos x P) && alpha ins x P i) := by
    rw [vec_map]
    apply vec_congr; intro i hi
    have hmem : ins.getD i default ∈ ins := by simp [hi, List.getElem_mem]
    have htl : tilesOK (ins.getD i default) = true := List.all_eq_true.mp ht _ hmem
    rw [opd_contains _ x htl (fun s hs => G.gseg s (List.mem_append_left _ (List.mem_append_left _
      (List.mem_flatMap.mpr ⟨_, hmem, hs⟩))))]
    rw [soup_group _ L hL x (fun t htm => (allSoup_ok ht (soup_mem hi htm)).1)
      (fun t htm hv => soup_in_planes (soup_mem hi htm) hv)]
    apply par_map_congr; intro P hP
    rw [ahead_x P (planes_nx hP) x]; rfl
  -- the result: on each plane, the jump at the atom holding the point
  have hres : (Solid.mk "" res.toArray).contains x =
      par (L.map fun P => decide (x.x < tpos x P) &&
        par (((atoms ins P).filter (jump op ins P)).map fun τ => τ.ins x)) := by
    rw [contains_par, res_group res L hL x (fun f hf hv => res_in_planes hf hv)]
    apply par_map_congr; intro P hP
    have hok := hpl P hP
    simp only [Bool.and_eq_true] at hok
    rw [ahead_x P (planes_nx hP) x, result_on op ins res P x hok.2 (fun τ hτ => (atom_ok ht hτ).2.2.2)
      (fun s hs => G.gseg s (by
        rcases List.mem_append.mp hs with hs | hs
        · obtain ⟨f, hf, hs⟩ := List.mem_flatMap.mp hs
          exact List.mem_append_left _ (List.mem_append_right _
            (List.mem_flatMap.mpr ⟨f, (List.mem_filter.mp hf).1, hs⟩))
        · exact List.mem_append_right _ (List.mem_flatMap.mpr ⟨P, hP, hs⟩)))]
  have hjump : ∀ P ∈ L, par (((atoms ins P).filter (jump op ins P)).map fun τ => τ.ins x) =
      (op (vec n fun i => A P i ^^ alpha ins x P i) ^^ op (vec n (A P))) := by
    intro P hP
    have hok := hpl P hP
    simp only [Bool.and_eq_true, List.all_eq_true] at hok
    obtain ⟨⟨hA, hall⟩, -⟩ := hok
    have hD : ∀ τ ∈ atoms ins P, τ.D ≠ 0 := fun τ hτ => (atom_ok ht hτ).2.2.2
    rw [par_filter]
    by_cases hex : ∃ τ₀ ∈ atoms ins P, τ₀.ins x = true
    · obtain ⟨τ₀, h0m, hin⟩ := hex
      rw [par_single (atoms ins P) (by unfold atoms; exact nodup_uniq _) τ₀ h0m
        (fun a => jump op ins P a && a.ins x) (fun τ hτ hne => by
        have : τ.ins x = false := by
          apply Classical.byContradiction; intro hn
          have hin' : τ.ins x = true := by simpa using hn
          unfold atomsOK at hA
          rcases pairwise_two (of_decide_eq_true hA) hτ h0m hne with h | h
          · exact sep2_sound _ _ x (hD _ hτ) (hD _ h0m) h hin' hin
          · exact sep2_sound _ _ x (hD _ h0m) (hD _ hτ) h hin hin'
        rw [this, Bool.and_false])]
      rw [hin, Bool.and_true]
      unfold jump
      dsimp only
      rw [vec_zip]
      have hb : ∀ i < n, beyond ins P (witness ins P τ₀) i = A P i := fun i hi =>
        beyond_eq op ins res x ht hcl G P hP τ₀ h0m hin (hall τ₀ h0m) i hi
      have hm : ∀ i < n, mult ins P τ₀ i = alpha ins x P i := by
        intro i hi
        unfold mult alpha
        apply par_map_congr; intro t htm
        exact ((atom_lookup ins P x hA hD t (soup_mem hi (List.mem_filter.mp htm).1)
          (List.mem_filter.mp htm).2).1 τ₀ h0m hin).symm
      rw [vec_congr n _ _ (fun i hi => by rw [hb i hi, hm i hi]), vec_congr n _ _ hb]
    · have hno : ∀ τ ∈ atoms ins P, τ.ins x = false := by
        intro τ hτ; apply Classical.byContradiction; intro hn; exact hex ⟨τ, hτ, by simpa using hn⟩
      rw [par_all_false _ (fun b hb => by
        obtain ⟨τ, hτ, rfl⟩ := List.mem_map.mp hb; rw [hno τ hτ, Bool.and_false])]
      have ha0 : ∀ i < n, (A P i ^^ alpha ins x P i) = A P i := by
        intro i hi
        have : alpha ins x P i = false := by
          unfold alpha
          apply par_all_false; intro b hb
          obtain ⟨t, htm, rfl⟩ := List.mem_map.mp hb
          exact (atom_lookup ins P x hA hD t (soup_mem hi (List.mem_filter.mp htm).1)
            (List.mem_filter.mp htm).2).2 hno
        rw [this, Bool.xor_false]
      rw [vec_congr n _ _ ha0]; simp
  rw [hres, hins]
  rw [tele n op h0 L hL (tpos x) (fun P hP P' hP' h => G.pos P hP P' hP' h) (alpha ins x) _ x.x rfl]
  apply par_map_congr; intro P hP
  rw [hjump P hP]

end ForgeCertify
