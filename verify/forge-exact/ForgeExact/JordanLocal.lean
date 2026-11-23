import ForgeExact.Jordan

/-!
# Jordan–Brouwer along a segment that crosses face planes

`Jordan.contains_const` needs the two points strictly on one side of every face plane. Here a face
plane may cut the segment `[p, q]`, as long as it does so outside the face: at the crossing point
the face's vertices all lie strictly to one side along a coordinate axis (`OutBox`, the face's
bounding box missed). `contains_const_local` is the result; `local_compat` derives the frontier's
`LocalCompat` from it.

The argument, per face (`face_rise`, `face_level`, `face_x`), for a move `w → w'` whose segment
crosses the face's plane at `m`:
* the crossings that count (`e3`, `eH`) are those of the part of the segment on the side where the
  face is ahead along +x, so the face's term is the crossing parity of `[w, m]` (or `[m, w']`);
* that is `chiE w ^^ chiE m` (or `chiE m ^^ chiE w'`) by the edge identity and the telescope;
* and `chiE m = false`: `m` is on the plane outside the box (`chi_out`, no general position needed).
For a rising move a vertex may sit at `m`'s level, so the split is taken a little past `m`, at
`P = m + ε (w' − w)` (`eps`), where every tie is broken as at `m`.
-/

namespace ForgeExact.Jordan

open ForgeExact.Ref ForgeExact.Flat

/-! ## Outside a face's box -/

def axes6 : List V3 := [⟨1, 0, 0⟩, ⟨-1, 0, 0⟩, ⟨0, 1, 0⟩, ⟨0, -1, 0⟩, ⟨0, 0, 1⟩, ⟨0, 0, -1⟩]

/-- Every vertex of the face is strictly below `x` along `k`. -/
def Below (f : Face) (k x : V3) : Prop := ∀ e ∈ edgesF f, k.dot e.1 < k.dot x ∧ k.dot e.2 < k.dot x

/-- `x` is strictly outside the face's bounding box. -/
def OutBox (f : Face) (x : V3) : Prop := ∃ k ∈ axes6, Below f k x

theorem dot_ptv (k a b : V3) (t : Rat) : k.dot (ptv a b t) = k.dot a + t * (k.dot b - k.dot a) := by
  simp only [V3.dot, ptv]; grind

theorem conv_lt {A B X t : Rat} (h0 : 0 ≤ t) (h1 : t ≤ 1) (ha : A < X) (hb : B < X) :
    A + t * (B - A) < X := by
  have e : X - (A + t * (B - A)) = (1 - t) * (X - A) + t * (X - B) := by grind
  have := Rat.mul_nonneg (by grind : (0:Rat) ≤ 1 - t) (by grind : (0:Rat) ≤ X - A)
  have := Rat.mul_nonneg h0 (by grind : (0:Rat) ≤ X - B)
  by_cases ht : t = 0
  · subst ht; grind
  · have := Rat.mul_pos (by grind : (0:Rat) < t) (by grind : (0:Rat) < X - B); grind

theorem below_ptv {f : Face} {k x : V3} (h : Below f k x) {e : V3 × V3} (he : e ∈ edgesF f) {t : Rat}
    (h0 : 0 ≤ t) (h1 : t ≤ 1) : k.dot (ptv e.1 e.2 t) < k.dot x := by
  rw [dot_ptv]; exact conv_lt h0 h1 (h e he).1 (h e he).2

/-- A point outside the box is on no edge. -/
theorem not_on_edge {f : Face} {x : V3} (h : OutBox f x) {e : V3 × V3} (he : e ∈ edgesF f) {t : Rat}
    (h0 : 0 ≤ t) (h1 : t ≤ 1) : ptv e.1 e.2 t ≠ x := by
  obtain ⟨k, _, hk⟩ := h
  intro hE
  have := below_ptv hk he h0 h1
  rw [hE] at this; grind

/-- Two points of a plane not parallel to +x, with the same y and z, are one. -/
theorem plane_x_eq (P : Plane) (hnx : P.n.x ≠ 0) {E m : V3} (hE : P.eval E = 0) (hm : P.eval m = 0)
    (hy : E.y = m.y) (hz : E.z = m.z) : E = m := by
  have : P.n.x * (E.x - m.x) = 0 := by
    simp only [Plane.eval, V3.dot] at hE hm; rw [hy, hz] at hE; grind
  rcases Rat.mul_eq_zero.mp this with h | h
  · exact absurd h hnx
  · cases E; cases m; simp only [V3.mk.injEq] at *; grind

/-! ## Parity facts -/

theorem exists_of_xorL {α : Type} (g : α → Bool) : ∀ (l : List α), xorL (l.map g) = true → ∃ x ∈ l, g x = true
  | [], h => by simp at h
  | x :: l, h => by
    by_cases hx : g x = true
    · exact ⟨x, by simp, hx⟩
    · simp only [List.map_cons, xorL_cons] at h
      have : g x = false := by simpa using hx
      rw [this, Bool.false_xor] at h
      obtain ⟨y, hy, hg⟩ := exists_of_xorL g l h
      exact ⟨y, List.mem_cons_of_mem _ hy, hg⟩

theorem xorL_and_const {α : Type} (l : List α) (g : α → Bool) (c : Bool) :
    xorL (l.map fun x => g x && c) = (xorL (l.map g) && c) := by
  cases c
  · simp only [Bool.and_false]; exact xorL_false l
  · simp

/-- Straddling the level `h` (the reference's tie-break: on the level counts as below). -/
def st (h : Rat) (a b : V3) : Bool := decide (a.z > h) != decide (b.z > h)

theorem st_even (f : Face) (h : Rat) : xorL ((edgesF f).map fun e => st h e.1 e.2) = false := by
  unfold edgesF
  rw [xorL_flatMap]
  rw [xorL_congr (g := fun _ => false) (fun lp _ => by
    rw [← telescope (fun v => decide (v.z > h)) lp]
    apply xorL_congr; intro e _; unfold st
    cases decide (e.1.z > h) <;> cases decide (e.2.z > h) <;> rfl), xorL_false]

theorem crossE_st (q a b : V3) : crossE q a b = (st q.z a b && decide (0 < d0 q a b * (b.z - a.z))) := rfl

/-- The point of a straddling edge at the level. -/
def lvl (h : Rat) (a b : V3) : V3 := ptv a b ((h - a.z) / (b.z - a.z))

theorem lvl_facts (q a b : V3) (hs : st q.z a b = true) :
    b.z - a.z ≠ 0 ∧ 0 ≤ (q.z - a.z) / (b.z - a.z) ∧ (q.z - a.z) / (b.z - a.z) ≤ 1 ∧
      (lvl q.z a b).z = q.z ∧ ((lvl q.z a b).y - q.y) * (b.z - a.z) = d0 q a b := by
  unfold st at hs
  have hdz : b.z - a.z ≠ 0 := by
    intro h0; apply absurd hs; have : b.z = a.z := by grind
    rw [this]; simp
  have hu := Rat.div_mul_cancel (a := q.z - a.z) hdz
  generalize hU : (q.z - a.z) / (b.z - a.z) = u at hu
  have hcase : (a.z > q.z ∧ ¬b.z > q.z) ∨ (¬a.z > q.z ∧ b.z > q.z) := by
    by_cases h1 : a.z > q.z <;> by_cases h2 : b.z > q.z <;> simp_all
  have hu0 : 0 ≤ u := by
    rcases hcase with ⟨h1, h2⟩ | ⟨h1, h2⟩
    · have hk : b.z - a.z < 0 := by grind
      apply Classical.byContradiction; intro hn
      have := (mul_pos_iff_neg hk (x := u)).mpr (by grind)
      grind
    · have hk : 0 < b.z - a.z := by grind
      apply Classical.byContradiction; intro hn
      have := (mul_pos_iff_left hk (x := -u)).mpr (by grind)
      grind
  have hu1 : u ≤ 1 := by
    rcases hcase with ⟨h1, h2⟩ | ⟨h1, h2⟩
    · have hk : b.z - a.z < 0 := by grind
      apply Classical.byContradiction; intro hn
      have := (mul_pos_iff_neg hk (x := 1 - u)).mpr (by grind)
      grind
    · have hk : 0 < b.z - a.z := by grind
      apply Classical.byContradiction; intro hn
      have := (mul_pos_iff_left hk (x := u - 1)).mpr (by grind)
      grind
  refine ⟨hdz, hu0, hu1, ?_, ?_⟩
  · simp only [lvl, ptv, hU]; grind
  · simp only [lvl, ptv, hU, d0]; grind

theorem lvl_on (P : Plane) (q a b : V3) (ha : P.eval a = 0) (hb : P.eval b = 0) :
    P.eval (lvl q.z a b) = 0 := plane_zero P a b _ ha hb

/-- The sign of the reference's crossing test is that of the level point's offset along +y. -/
theorem pos_iff_lvl (q a b : V3) (hs : st q.z a b = true) :
    0 < d0 q a b * (b.z - a.z) ↔ q.y < (lvl q.z a b).y := by
  obtain ⟨hdz, _, _, _, hy⟩ := lvl_facts q a b hs
  rw [← hy, show ((lvl q.z a b).y - q.y) * (b.z - a.z) * (b.z - a.z) =
    ((b.z - a.z) * (b.z - a.z)) * ((lvl q.z a b).y - q.y) by grind, mul_pos_iff_left (sq_pos hdz)]
  grind

/-! ## Outside the box, on the plane: the crossing parity is even -/

theorem chi_out (f : Face) (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hnx : f.plane.n.x ≠ 0) (m : V3) (hm : f.plane.eval m = 0) (hout : OutBox f m) :
    chiE f m = false := by
  obtain ⟨k, hk, hb⟩ := hout
  have hlv : ∀ e ∈ edgesF f, st m.z e.1 e.2 = true →
      k.dot (lvl m.z e.1 e.2) < k.dot m ∧ (lvl m.z e.1 e.2).z = m.z ∧
      f.plane.eval (lvl m.z e.1 e.2) = 0 ∧
      (crossE m e.1 e.2 = true ↔ m.y < (lvl m.z e.1 e.2).y) := by
    intro e he hs
    obtain ⟨_, h0, h1, hz, _⟩ := lvl_facts m e.1 e.2 hs
    refine ⟨below_ptv hb he h0 h1, hz, lvl_on _ _ _ _ (hflat e he).1 (hflat e he).2, ?_⟩
    rw [crossE_st, hs, Bool.true_and, decide_eq_true_iff, pos_iff_lvl m e.1 e.2 hs]
  unfold chiE
  simp only [axes6, List.mem_cons, List.mem_nil_iff, or_false] at hk
  -- along z: nothing straddles
  have nost : (∀ e ∈ edgesF f, st m.z e.1 e.2 = false) → xorL ((edgesF f).map fun e => crossE m e.1 e.2) = false := by
    intro h
    rw [xorL_congr (g := fun _ => false) (fun e he => by rw [crossE_st, h e he, Bool.false_and])]
    exact xorL_false _
  -- along x: two crossings on either side of `m` in y, both on one side in x, contradict the plane
  have xcase : (∀ e1 ∈ edgesF f, ∀ e2 ∈ edgesF f, st m.z e1.1 e1.2 = true → st m.z e2.1 e2.2 = true →
      0 < ((lvl m.z e1.1 e1.2).x - m.x) * ((lvl m.z e2.1 e2.2).x - m.x)) →
      xorL ((edgesF f).map fun e => crossE m e.1 e.2) = false := by
    intro hsame
    apply Classical.byContradiction
    intro hc
    have hc' : xorL ((edgesF f).map fun e => crossE m e.1 e.2) = true := by simpa using hc
    let g2 : V3 × V3 → Bool := fun e => st m.z e.1 e.2 && !decide (0 < d0 m e.1 e.2 * (e.2.z - e.1.z))
    have hsplit : xorL ((edgesF f).map fun e => st m.z e.1 e.2) =
        (xorL ((edgesF f).map fun e => crossE m e.1 e.2) ^^ xorL ((edgesF f).map g2)) := by
      rw [← xorL_map_xor]
      apply xorL_congr; intro e _
      simp only [g2, crossE_st]
      cases st m.z e.1 e.2 <;> cases decide (0 < d0 m e.1 e.2 * (e.2.z - e.1.z)) <;> rfl
    rw [st_even, hc'] at hsplit
    have hg2 : xorL ((edgesF f).map g2) = true := by
      revert hsplit; cases xorL ((edgesF f).map g2) <;> decide
    obtain ⟨e1, he1, h1⟩ := exists_of_xorL _ _ hc'
    obtain ⟨e2, he2, h2⟩ := exists_of_xorL _ _ hg2
    simp only [g2, Bool.and_eq_true, Bool.not_eq_true', decide_eq_false_iff_not] at h2
    have hs1 : st m.z e1.1 e1.2 = true := by rw [crossE_st] at h1; simp only [Bool.and_eq_true] at h1; exact h1.1
    obtain ⟨_, hz1, hp1, hy1⟩ := hlv e1 he1 hs1
    obtain ⟨_, hz2, hp2, hy2⟩ := hlv e2 he2 h2.1
    have hy1' := hy1.mp h1
    have hy2' : ¬m.y < (lvl m.z e2.1 e2.2).y := by
      intro h; apply h2.2; rw [pos_iff_lvl m e2.1 e2.2 h2.1]; exact h
    have hpos := hsame e1 he1 e2 he2 hs1 h2.1
    generalize lvl m.z e1.1 e1.2 = E1 at *
    generalize lvl m.z e2.1 e2.2 = E2 at *
    have q1 : f.plane.n.x * (E1.x - m.x) = -(f.plane.n.y * (E1.y - m.y)) := by
      simp only [Plane.eval, V3.dot] at hp1 hm; rw [hz1] at hp1; grind
    have q2 : f.plane.n.x * (E2.x - m.x) = -(f.plane.n.y * (E2.y - m.y)) := by
      simp only [Plane.eval, V3.dot] at hp2 hm; rw [hz2] at hp2; grind
    have e : f.plane.n.x * f.plane.n.x * ((E1.x - m.x) * (E2.x - m.x)) =
        f.plane.n.y * f.plane.n.y * ((E1.y - m.y) * (E2.y - m.y)) := by
      calc f.plane.n.x * f.plane.n.x * ((E1.x - m.x) * (E2.x - m.x))
          = (f.plane.n.x * (E1.x - m.x)) * (f.plane.n.x * (E2.x - m.x)) := by grind
        _ = (f.plane.n.y * (E1.y - m.y)) * (f.plane.n.y * (E2.y - m.y)) := by rw [q1, q2]; grind
        _ = _ := by grind
    have l1 := Rat.mul_pos (sq_pos hnx) hpos
    have l2 := Rat.mul_nonneg (by grind : (0:Rat) ≤ E1.y - m.y) (by grind : (0:Rat) ≤ -(E2.y - m.y))
    have l3 := Rat.mul_nonneg (mul_self_nonneg' f.plane.n.y) (by grind : (0:Rat) ≤ -((E1.y - m.y) * (E2.y - m.y)))
    grind
  rcases hk with rfl | rfl | rfl | rfl | rfl | rfl
  · apply xcase; intro e1 he1 e2 he2 s1 s2
    have a1 := (hlv e1 he1 s1).1; have a2 := (hlv e2 he2 s2).1
    simp only [V3.dot] at a1 a2
    have := Rat.mul_pos (by grind : (0:Rat) < -((lvl m.z e1.1 e1.2).x - m.x)) (by grind : (0:Rat) < -((lvl m.z e2.1 e2.2).x - m.x))
    grind
  · apply xcase; intro e1 he1 e2 he2 s1 s2
    have a1 := (hlv e1 he1 s1).1; have a2 := (hlv e2 he2 s2).1
    simp only [V3.dot] at a1 a2
    exact Rat.mul_pos (by grind) (by grind)
  · -- every vertex below in y: no crossing
    rw [xorL_congr (g := fun _ => false) (fun e he => by
      cases hs : st m.z e.1 e.2
      · rw [crossE_st, hs, Bool.false_and]
      · have := hlv e he hs
        simp only [V3.dot] at this
        apply Bool.eq_false_iff.mpr; intro h; have := this.2.2.2.mp h; grind)]
    exact xorL_false _
  · -- every vertex above in y: every straddle crosses
    rw [xorL_congr (g := fun e => st m.z e.1 e.2) (fun e he => by
      cases hs : st m.z e.1 e.2
      · rw [crossE_st, hs, Bool.false_and]
      · have := hlv e he hs
        simp only [V3.dot] at this
        exact (this.2.2.2.mpr (by grind)))]
    exact st_even f m.z
  all_goals
    rw [xorL_congr (g := fun _ => false) (fun e he => by
      cases hs : st m.z e.1 e.2
      · rw [crossE_st, hs, Bool.false_and]
      · have := hlv e he hs
        simp only [V3.dot] at this
        grind)]
    exact xorL_false _

/-- A face parallel to +x has even crossing parity everywhere: on its level every straddling edge
meets the plane's trace at the same point. -/
theorem chi_vert (f : Face) (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hnx : f.plane.n.x = 0) (hn : f.plane.n.y ≠ 0 ∨ f.plane.n.z ≠ 0) (w : V3) : chiE f w = false := by
  unfold chiE
  by_cases hy : f.plane.n.y = 0
  · have hz : f.plane.n.z ≠ 0 := by rcases hn with h | h; exact absurd hy h; exact h
    rw [xorL_congr (g := fun _ => false) (fun e he => by
      have ha := (hflat e he).1; have hb := (hflat e he).2
      simp only [Plane.eval, V3.dot, hnx, hy] at ha hb
      have : e.1.z = e.2.z := mul_cancel hz (by grind)
      rw [crossE_st]; unfold st; rw [this]; simp)]
    exact xorL_false _
  · let K := f.plane.c - f.plane.n.z * w.z - f.plane.n.y * w.y
    rw [xorL_congr (g := fun e => st w.z e.1 e.2 && decide (0 < f.plane.n.y * K)) (fun e he => by
      rw [crossE_st]
      cases hs : st w.z e.1 e.2
      · rfl
      · simp only [Bool.true_and]
        rw [Bool.eq_iff_iff, decide_eq_true_iff, decide_eq_true_iff, pos_iff_lvl w e.1 e.2 hs]
        obtain ⟨_, _, _, hz, _⟩ := lvl_facts w e.1 e.2 hs
        have hp := lvl_on f.plane w e.1 e.2 (hflat e he).1 (hflat e he).2
        generalize lvl w.z e.1 e.2 = E at *
        have e1 : f.plane.n.y * (E.y - w.y) = K := by
          simp only [Plane.eval, V3.dot, hnx] at hp; rw [hz] at hp; simp only [K]; grind
        have e2 : f.plane.n.y * K = (f.plane.n.y * f.plane.n.y) * (E.y - w.y) := by rw [← e1]; grind
        rw [e2, mul_pos_iff_left (sq_pos hy)]; grind)]
    rw [xorL_and_const, st_even]; rfl

/-! ## Crossing part of the segment -/

def zr (w w' a b : V3) : Rat := (ptv a b (f3 w w' a b).root).z

theorem Fs_left (w w' x : V3) (s : Rat) : Fs w (ptv w w' s) x = s * Fs w w' x := by
  simp only [Fs, ptv]; grind

theorem Fs_right (w w' x : V3) (s : Rat) : Fs (ptv w w' s) w' x = (1 - s) * Fs w w' x := by
  simp only [Fs, ptv]; grind

def Aff.sc (k : Rat) (f : Aff) : Aff := ⟨k * f.α, k * f.γ⟩

theorem sc_root (k : Rat) (hk : k ≠ 0) (f : Aff) : (f.sc k).root = f.root := by
  by_cases h : f.α = f.γ
  · unfold Aff.sc Aff.root
    rw [show k * f.α - k * f.γ = 0 by rw [h]; grind, show f.α - f.γ = 0 by rw [h]; grind,
      Rat.div_def, Rat.div_def, Rat.inv_zero]; grind
  · have h1 := root_mul f h
    have h2 := root_mul (f.sc k) (by unfold Aff.sc; intro e; exact h (mul_cancel hk e))
    unfold Aff.sc at h2; simp only at h2
    apply mul_cancel (k := f.α - f.γ) (by grind)
    have : k * ((f.sc k).root * (f.α - f.γ)) = k * (f.root * (f.α - f.γ)) := by
      unfold Aff.sc; rw [h1]; grind
    have := mul_cancel hk this; grind

theorem sc_chg (k : Rat) (hk : k ≠ 0) (f : Aff) : (f.sc k).α * (f.sc k).γ < 0 ↔ f.α * f.γ < 0 := by
  unfold Aff.sc
  rw [show k * f.α * (k * f.γ) = (k * k) * (f.α * f.γ) by grind]
  constructor
  · intro h; apply Classical.byContradiction; intro hn
    have := Rat.mul_nonneg (mul_self_nonneg' k) (by grind : (0:Rat) ≤ f.α * f.γ); grind
  · intro h; have := (mul_pos_iff_left (sq_pos hk) (x := -(f.α * f.γ))).mpr (by grind); grind

theorem f3_left (w w' a b : V3) (s : Rat) : f3 w (ptv w w' s) a b = (f3 w w' a b).sc s := by
  simp only [f3, Aff.sc, Fs_left]

theorem f3_right (w w' a b : V3) (s : Rat) : f3 (ptv w w' s) w' a b = (f3 w w' a b).sc (1 - s) := by
  simp only [f3, Aff.sc, Fs_right]

/-- The whole segment's crossing, by the crossing point's level. -/
theorem crossS_iff (w w' a b : V3) : crossS w w' a b = true ↔
    (f3 w w' a b).α * (f3 w w' a b).γ < 0 ∧ w.z < zr w w' a b ∧ zr w w' a b < w'.z := by
  unfold crossS Aff.X zr
  rw [decide_eq_true_iff, f1_at, f2_at]; grind

theorem crossS_left (w w' a b : V3) (s : Rat) (hs : 0 < s) : crossS w (ptv w w' s) a b = true ↔
    (f3 w w' a b).α * (f3 w w' a b).γ < 0 ∧ w.z < zr w w' a b ∧ zr w w' a b < (ptv w w' s).z := by
  unfold crossS Aff.X zr
  rw [decide_eq_true_iff, f1_at, f2_at, f3_left, sc_root s (by grind), sc_chg s (by grind)]; grind

theorem crossS_right (w w' a b : V3) (s : Rat) (hs : s < 1) : crossS (ptv w w' s) w' a b = true ↔
    (f3 w w' a b).α * (f3 w w' a b).γ < 0 ∧ (ptv w w' s).z < zr w w' a b ∧ zr w w' a b < w'.z := by
  unfold crossS Aff.X zr
  rw [decide_eq_true_iff, f1_at, f2_at, f3_right, sc_root (1 - s) (by grind), sc_chg (1 - s) (by grind)]
  grind

/-! ## Where the plane crosses the segment -/

def crossL (P : Plane) (w w' : V3) : Rat := P.eval w / (P.eval w - P.eval w')
def crossPt (P : Plane) (w w' : V3) : V3 := ptv w w' (crossL P w w')

/-- How a move passes a face: strictly on one side of its plane, within its plane, or across it
outside the face's box. -/
def Pass (f : Face) (w w' : V3) : Prop :=
  0 < f.plane.eval w * f.plane.eval w' ∨
  (f.plane.n ≠ ⟨0, 0, 0⟩ ∧ f.plane.eval w = 0 ∧ f.plane.eval w' = 0) ∨
  (f.plane.eval w ≠ f.plane.eval w' ∧ f.plane.eval w * f.plane.eval w' ≤ 0 ∧
    OutBox f (crossPt f.plane w w'))

theorem crossL_mul (P : Plane) (w w' : V3) (hD : P.eval w ≠ P.eval w') :
    crossL P w w' * (P.eval w - P.eval w') = P.eval w := Rat.div_mul_cancel (by grind)

theorem crossL_facts (P : Plane) (w w' : V3) (hD : P.eval w ≠ P.eval w') (hs : P.eval w * P.eval w' ≤ 0) :
    0 ≤ crossL P w w' ∧ crossL P w w' ≤ 1 ∧ P.eval (crossPt P w w') = 0 ∧
      (crossL P w w' = 0 ↔ P.eval w = 0) ∧ (crossL P w w' = 1 ↔ P.eval w' = 0) := by
  have hm := crossL_mul P w w' hD
  unfold crossPt
  generalize crossL P w w' = l at hm ⊢
  have hDD := sq_pos (by grind : P.eval w - P.eval w' ≠ 0)
  have e1 : l * ((P.eval w - P.eval w') * (P.eval w - P.eval w')) =
      P.eval w * P.eval w - P.eval w * P.eval w' := by
    rw [show l * ((P.eval w - P.eval w') * (P.eval w - P.eval w')) =
      (l * (P.eval w - P.eval w')) * (P.eval w - P.eval w') by grind, hm]; grind
  have hww := mul_self_nonneg' (P.eval w)
  have hww' := mul_self_nonneg' (P.eval w')
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · apply Classical.byContradiction; intro hn
    have := (mul_pos_iff_left hDD (x := -l)).mpr (by grind); grind
  · apply Classical.byContradiction; intro hn
    have := (mul_pos_iff_left hDD (x := l - 1)).mpr (by grind); grind
  · rw [eval_ptv]; grind
  · constructor
    · intro h; rw [h] at hm; grind
    · intro h; rw [h] at hm
      exact mul_cancel (k := P.eval w - P.eval w') (by grind) (by grind)
  · constructor
    · intro h; rw [h] at hm; grind
    · intro h; rw [h] at hm
      exact mul_cancel (k := P.eval w - P.eval w') (by grind) (by grind)

/-- Over a point `B` of the segment, the plane point `E` is past `B` along +x by the side of the
crossing `B` is on. -/
theorem split_x (P : Plane) (hnx : P.n.x ≠ 0) (w w' E : V3) (lam : Rat) (hD : P.eval w ≠ P.eval w')
    (hE : P.eval E = 0) (hy : (ptv w w' lam).y = E.y) (hz : (ptv w w' lam).z = E.z) :
    (ptv w w' lam).x < E.x ↔ 0 < (lam - crossL P w w') * ((P.eval w - P.eval w') * P.n.x) := by
  have hm := crossL_mul P w w' hD
  have hB := eval_ptv P w w' lam
  generalize crossL P w w' = l at hm
  generalize ptv w w' lam = B at hB hy hz ⊢
  have hdiff : P.n.x * (E.x - B.x) = -P.eval B := by
    simp only [Plane.eval, V3.dot] at hE hB ⊢
    rw [← hy, ← hz] at hE; grind
  have e : (lam - l) * ((P.eval w - P.eval w') * P.n.x) = (P.n.x * P.n.x) * (E.x - B.x) := by
    rw [show (P.n.x * P.n.x) * (E.x - B.x) = P.n.x * (P.n.x * (E.x - B.x)) by grind, hdiff, hB]; grind
  rw [e, mul_pos_iff_left (sq_pos hnx)]; grind

/-- On a plane parallel to +x, a segment point under a plane point is the crossing. -/
theorem split_vert (P : Plane) (hnx : P.n.x = 0) (w w' E : V3) (lam : Rat) (hD : P.eval w ≠ P.eval w')
    (hE : P.eval E = 0) (hy : (ptv w w' lam).y = E.y) (hz : (ptv w w' lam).z = E.z) :
    lam = crossL P w w' := by
  have hm := crossL_mul P w w' hD
  have hB := eval_ptv P w w' lam
  generalize crossL P w w' = l at hm
  generalize ptv w w' lam = B at hB hy hz
  have h0 : P.eval B = 0 := by
    simp only [Plane.eval, V3.dot, hnx] at hE hB ⊢
    rw [← hy, ← hz] at hE; grind
  rw [h0] at hB
  exact mul_cancel (k := P.eval w - P.eval w') (by grind) (by grind)

theorem ahead_def (f : Face) (q : V3) : aheadB f q = decide (0 < -(f.plane.eval q) * f.plane.n.x) := by
  unfold aheadB; congr 2; simp only [Plane.eval]; grind

/-- The crossings that count, for a face not parallel to +x: those past the crossing level, on the
side where the face is ahead (`K` is negative when it is ahead of `w`). -/
theorem e3_nx (f : Face) (hnx : f.plane.n.x ≠ 0) (w w' : V3) (hw : w.z < w'.z)
    (hD : f.plane.eval w ≠ f.plane.eval w') (e : V3 × V3)
    (hfe : f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0) :
    e3 w w' e = (crossS w w' e.1 e.2 && decide (0 < (zr w w' e.1 e.2 - (crossPt f.plane w w').z) *
      ((f.plane.eval w - f.plane.eval w') * f.plane.n.x))) := by
  unfold e3
  cases hc : crossS w w' e.1 e.2
  · rfl
  · obtain ⟨l0, l1, hy, hz⟩ := over_S w w' e.1 e.2 hw hc
    have hE := plane_zero f.plane e.1 e.2 (f3 w w' e.1 e.2).root hfe.1 hfe.2
    simp only [Bool.true_and]
    rw [Bool.eq_iff_iff, decide_eq_true_iff, decide_eq_true_iff,
      split_x f.plane hnx w w' _ _ hD hE hy hz]
    have hzr : zr w w' e.1 e.2 - (crossPt f.plane w w').z =
        (lamS w w' e.1 e.2 - crossL f.plane w w') * (w'.z - w.z) := by
      unfold zr crossPt; rw [← hz]; simp only [ptv]; grind
    rw [hzr, show (lamS w w' e.1 e.2 - crossL f.plane w w') * (w'.z - w.z) *
        ((f.plane.eval w - f.plane.eval w') * f.plane.n.x) = (w'.z - w.z) *
        ((lamS w w' e.1 e.2 - crossL f.plane w w') * ((f.plane.eval w - f.plane.eval w') * f.plane.n.x))
        by grind, mul_pos_iff_left (k := w'.z - w.z) (by grind)]

theorem e3_vert (f : Face) (hnx : f.plane.n.x = 0) (w w' : V3) (hw : w.z < w'.z)
    (hD : f.plane.eval w ≠ f.plane.eval w') (e : V3 × V3)
    (hfe : f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0) :
    e3 w w' e = (crossS w w' e.1 e.2 && decide ((crossPt f.plane w w').x < (ptv e.1 e.2 (f3 w w' e.1 e.2).root).x)) := by
  unfold e3
  cases hc : crossS w w' e.1 e.2
  · rfl
  · obtain ⟨l0, l1, hy, hz⟩ := over_S w w' e.1 e.2 hw hc
    have hE := plane_zero f.plane e.1 e.2 (f3 w w' e.1 e.2).root hfe.1 hfe.2
    rw [split_vert f.plane hnx w w' _ _ hD hE hy hz]; rfl

/-! ## The edge identity summed over a face -/

theorem chi_seg (f : Face) (w w' : V3) (hw : w.z < w'.z) (hgp : ∀ e ∈ edgesF f, GP w w' e.1 e.2) :
    (chiE f w ^^ chiE f w') = xorL ((edgesF f).map fun e => crossS w w' e.1 e.2) := by
  unfold chiE
  have per : ∀ e ∈ edgesF f, (crossE w e.1 e.2 ^^ crossE w' e.1 e.2) =
      (crossS w w' e.1 e.2 ^^ (inH w w' e.1 ^^ inH w w' e.2)) := fun e he =>
    xor_rearrange _ _ _ _ (seg_identity w w' e.1 e.2 hw (hgp e he))
  rw [← xorL_map_xor (edgesF f) (fun e => crossE w e.1 e.2) (fun e => crossE w' e.1 e.2),
    xorL_congr per, xorL_map_xor (edgesF f) (fun e => crossS w w' e.1 e.2)
      (fun e => inH w w' e.1 ^^ inH w w' e.2)]
  have : xorL ((edgesF f).map fun e => inH w w' e.1 ^^ inH w w' e.2) = false := by
    unfold edgesF
    rw [xorL_flatMap]
    rw [xorL_congr (g := fun _ => false) (fun lp _ => telescope (inH w w') lp), xorL_false]
  rw [this]; simp

/-! ## Facts at the crossing, outside the box -/

theorem d0_out (f : Face) (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hnx : f.plane.n.x ≠ 0) {m : V3} (hm : f.plane.eval m = 0) (hout : OutBox f m) {e : V3 × V3}
    (he : e ∈ edgesF f) (hs : st m.z e.1 e.2 = true) : d0 m e.1 e.2 ≠ 0 := by
  obtain ⟨hdz, h0, h1, hz, hy⟩ := lvl_facts m e.1 e.2 hs
  have hp := lvl_on f.plane m e.1 e.2 (hflat e he).1 (hflat e he).2
  intro hd
  rw [hd] at hy
  have hy' : (lvl m.z e.1 e.2).y = m.y := by
    rcases Rat.mul_eq_zero.mp hy with h | h
    · grind
    · exact absurd h hdz
  exact not_on_edge hout he h0 h1 (plane_x_eq f.plane hnx hp hm hy' hz)

theorem root01 {w w' a b : V3} (h : crossS w w' a b = true) :
    0 < (f3 w w' a b).root ∧ (f3 w w' a b).root < 1 := by
  have := (crossS_iff w w' a b).mp h
  exact Aff.roots _ ((mul_neg_iff _ _ (by intro h0; rw [h0] at this; grind)
    (by intro h0; rw [h0] at this; grind)).mp this.1)

theorem zr_ne (f : Face) (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hnx : f.plane.n.x ≠ 0) (w w' : V3) (hw : w.z < w'.z) (l : Rat)
    (hm : f.plane.eval (ptv w w' l) = 0) (hout : OutBox f (ptv w w' l)) {e : V3 × V3}
    (he : e ∈ edgesF f) (hc : (f3 w w' e.1 e.2).α * (f3 w w' e.1 e.2).γ < 0) :
    zr w w' e.1 e.2 ≠ (ptv w w' l).z := by
  have hE := plane_zero f.plane e.1 e.2 (f3 w w' e.1 e.2).root (hflat e he).1 (hflat e he).2
  have hr := Aff.roots _ ((mul_neg_iff _ _ (by intro h0; rw [h0] at hc; grind)
    (by intro h0; rw [h0] at hc; grind)).mp hc)
  have hF : Fs w w' (ptv e.1 e.2 (f3 w w' e.1 e.2).root) = 0 := by rw [← f3_at]; exact root_eq _ (chg_ne hc)
  intro hzr
  unfold zr at hzr
  have hy : (ptv e.1 e.2 (f3 w w' e.1 e.2).root).y = (ptv w w' l).y := by
    simp only [Fs, ptv] at hF hzr ⊢
    have : (w'.z - w.z) * (e.1.y + (f3 w w' e.1 e.2).root * (e.2.y - e.1.y) - (w.y + l * (w'.y - w.y))) = 0 := by grind
    rcases Rat.mul_eq_zero.mp this with h | h
    · grind
    · grind
  exact not_on_edge hout he (Rat.le_of_lt hr.1) (Rat.le_of_lt hr.2) (plane_x_eq f.plane hnx hE hm hy hzr)

/-- The bounds the shifted split point `P = m + ε (w' − w)` must stay below. -/
def epsPolys (f : Face) (w w' m : V3) : List (Rat × Rat × Rat) :=
  (edgesF f).flatMap fun e =>
    [(e.1.z - m.z, -(w'.z - w.z), 0), (e.2.z - m.z, -(w'.z - w.z), 0),
     (d0 m e.1 e.2 * (e.2.z - e.1.z),
       ((e.2.y - e.1.y) * (w'.z - w.z) - (w'.y - w.y) * (e.2.z - e.1.z)) * (e.2.z - e.1.z), 0),
     (m.z - zr w w' e.1 e.2, w'.z - w.z, 0)]

theorem lex_c0 {c0 c1 : Rat} (h : c0 ≠ 0) : LexPos c0 c1 0 ↔ 0 < c0 := by unfold LexPos; grind

theorem eps_vert {ε c d : Rat} (hε : 0 < ε) (hd : 0 < d) (hb : ε ≤ bnd c (-d) 0) :
    c - ε * d ≠ 0 ∧ (0 < c - ε * d ↔ 0 < c) := by
  have hp := small_pos c (-d) 0 ε hε hb
  have hn := small_ne c (-d) 0 ε hε hb (Or.inr (Or.inl (by grind)))
  have e : poly c (-d) 0 ε = c - ε * d := by unfold poly; grind
  rw [e] at hp hn
  refine ⟨hn, ?_⟩
  rw [hp]; unfold LexPos; grind

theorem eps_keep {ε c d : Rat} (hε : 0 < ε) (hc : c ≠ 0) (hb : ε ≤ bnd c d 0) :
    c + ε * d ≠ 0 ∧ (0 < c + ε * d ↔ 0 < c) := by
  have hp := small_pos c d 0 ε hε hb
  have hn := small_ne c d 0 ε hε hb (Or.inl hc)
  have e : poly c d 0 ε = c + ε * d := by unfold poly; grind
  rw [e] at hp hn
  exact ⟨hn, by rw [hp, lex_c0 hc]⟩

theorem eps_facts (f : Face) (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hnx : f.plane.n.x ≠ 0) (w w' : V3) (hw : w.z < w'.z) (l ε : Rat) (hε : 0 < ε)
    (hb : ∀ t ∈ epsPolys f w w' (ptv w w' l), ε ≤ bnd t.1 t.2.1 t.2.2)
    (hm : f.plane.eval (ptv w w' l) = 0) (hout : OutBox f (ptv w w' l)) (P : V3)
    (hPy : P.y = (ptv w w' l).y + ε * (w'.y - w.y)) (hPz : P.z = (ptv w w' l).z + ε * (w'.z - w.z)) :
    (∀ e ∈ edgesF f, e.1.z ≠ P.z ∧ e.2.z ≠ P.z ∧ st P.z e.1 e.2 = st (ptv w w' l).z e.1 e.2) ∧
    (∀ e ∈ edgesF f, crossE P e.1 e.2 = crossE (ptv w w' l) e.1 e.2 ∧
      (st P.z e.1 e.2 = true → d0 P e.1 e.2 ≠ 0)) ∧
    (∀ e ∈ edgesF f, (f3 w w' e.1 e.2).α * (f3 w w' e.1 e.2).γ < 0 →
      zr w w' e.1 e.2 ≠ P.z ∧ (zr w w' e.1 e.2 < P.z ↔ zr w w' e.1 e.2 < (ptv w w' l).z)) := by
  generalize hM : ptv w w' l = m at *
  have hdz : 0 < w'.z - w.z := by grind
  have mem : ∀ e ∈ edgesF f, ∀ t, t ∈ [(e.1.z - m.z, -(w'.z - w.z), (0 : Rat)), (e.2.z - m.z, -(w'.z - w.z), 0),
     (d0 m e.1 e.2 * (e.2.z - e.1.z),
       ((e.2.y - e.1.y) * (w'.z - w.z) - (w'.y - w.y) * (e.2.z - e.1.z)) * (e.2.z - e.1.z), 0),
     (m.z - zr w w' e.1 e.2, w'.z - w.z, 0)] → ε ≤ bnd t.1 t.2.1 t.2.2 :=
    fun e he t ht => hb t (List.mem_flatMap.mpr ⟨e, he, ht⟩)
  have hv : ∀ e ∈ edgesF f, e.1.z ≠ P.z ∧ e.2.z ≠ P.z ∧ st P.z e.1 e.2 = st m.z e.1 e.2 := by
    intro e he
    have a := eps_vert hε hdz (mem e he (e.1.z - m.z, -(w'.z - w.z), 0) (List.mem_cons_self ..))
    have b := eps_vert hε hdz (mem e he (e.2.z - m.z, -(w'.z - w.z), 0)
      (List.mem_cons_of_mem _ (List.mem_cons_self ..)))
    refine ⟨by grind, by grind, ?_⟩
    unfold st
    have e1 : decide (e.1.z > P.z) = decide (e.1.z > m.z) := by
      apply decide_eq_decide.mpr; have := a.2; grind
    have e2 : decide (e.2.z > P.z) = decide (e.2.z > m.z) := by
      apply decide_eq_decide.mpr; have := b.2; grind
    rw [e1, e2]
  refine ⟨hv, ?_, ?_⟩
  · intro e he
    have hs := (hv e he).2.2
    have hd : d0 P e.1 e.2 * (e.2.z - e.1.z) = d0 m e.1 e.2 * (e.2.z - e.1.z) +
        ε * (((e.2.y - e.1.y) * (w'.z - w.z) - (w'.y - w.y) * (e.2.z - e.1.z)) * (e.2.z - e.1.z)) := by
      simp only [d0, hPy, hPz]; grind
    cases hsm : st m.z e.1 e.2
    · rw [crossE_st, crossE_st, hs, hsm]; simp
    · have hd0 := d0_out f hflat hnx hm hout he hsm
      have hdz' := (lvl_facts m e.1 e.2 hsm).1
      have k := eps_keep (c := d0 m e.1 e.2 * (e.2.z - e.1.z))
        (d := ((e.2.y - e.1.y) * (w'.z - w.z) - (w'.y - w.y) * (e.2.z - e.1.z)) * (e.2.z - e.1.z))
        hε (fun h0 => by
        rcases Rat.mul_eq_zero.mp h0 with h | h <;> contradiction) (mem e he (d0 m e.1 e.2 * (e.2.z - e.1.z),
       ((e.2.y - e.1.y) * (w'.z - w.z) - (w'.y - w.y) * (e.2.z - e.1.z)) * (e.2.z - e.1.z), 0)
          (List.mem_cons_of_mem _ (List.mem_cons_of_mem _ (List.mem_cons_self ..))))
      rw [← hd] at k
      refine ⟨?_, fun _ h0 => by rw [h0] at k; grind⟩
      rw [crossE_st, crossE_st, hs, hsm, Bool.true_and, Bool.true_and]
      exact decide_eq_decide.mpr k.2
  · intro e he hc
    have hne := zr_ne f hflat hnx w w' hw l (hM ▸ hm) (hM ▸ hout) he hc
    rw [hM] at hne
    have k := eps_keep hε (by grind) (mem e he (m.z - zr w w' e.1 e.2, w'.z - w.z, 0)
      (List.mem_cons_of_mem _ (List.mem_cons_of_mem _ (List.mem_cons_of_mem _ (List.mem_cons_self ..)))))
    have := k.2; have := k.1; rw [hPz]; constructor <;> grind

/-! ## One face, a rising move across its plane -/

theorem ptv_zero (w w' : V3) : ptv w w' 0 = w := by simp only [ptv, Rat.zero_mul, Rat.add_zero]

theorem ptv_one (w w' : V3) : ptv w w' 1 = w' := by simp only [ptv]; cases w; cases w'; simp; grind

theorem Fs_plane (P : Plane) (hnx : P.n.x = 0) (hn : P.n.y ≠ 0 ∨ P.n.z ≠ 0) {w w' a : V3}
    (h1 : P.eval w = 0) (h2 : P.eval w' = 0) (h3 : P.eval a = 0) : Fs w w' a = 0 := by
  simp only [Plane.eval, V3.dot, hnx] at h1 h2 h3
  have ey : P.n.y * Fs w w' a = 0 := by simp only [Fs]; grind
  have ez : P.n.z * Fs w w' a = 0 := by simp only [Fs]; grind
  rcases hn with h | h
  · rcases Rat.mul_eq_zero.mp ey with h' | h'; exact absurd h' h; exact h'
  · rcases Rat.mul_eq_zero.mp ez with h' | h'; exact absurd h' h; exact h'

theorem n_yz {P : Plane} (hnx : P.n.x = 0) {w w' : V3} (hD : P.eval w ≠ P.eval w') :
    P.n.y ≠ 0 ∨ P.n.z ≠ 0 := by
  apply Classical.byContradiction; intro h
  have hy : P.n.y = 0 := by grind
  have hz : P.n.z = 0 := by grind
  apply hD; simp only [Plane.eval, V3.dot, hnx, hy, hz]; grind

/-- The face's term when the plane is crossed with `K < 0`: the face is ahead of `w` only. -/
theorem term_neg (f : Face) (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hnx : f.plane.n.x ≠ 0) (w w' : V3) (hD : f.plane.eval w ≠ f.plane.eval w')
    (hs : f.plane.eval w * f.plane.eval w' ≤ 0) (hout : OutBox f (crossPt f.plane w w'))
    (hK : (f.plane.eval w - f.plane.eval w') * f.plane.n.x < 0) :
    (piercedE f w ^^ piercedE f w') = chiE f w := by
  have hab : (f.plane.eval w * f.plane.n.x) * (f.plane.eval w' * f.plane.n.x) ≤ 0 := by
    rw [show (f.plane.eval w * f.plane.n.x) * (f.plane.eval w' * f.plane.n.x) =
      (f.plane.n.x * f.plane.n.x) * (f.plane.eval w * f.plane.eval w') by grind]
    have := Rat.mul_nonneg (mul_self_nonneg' f.plane.n.x) (by grind : (0:Rat) ≤ -(f.plane.eval w * f.plane.eval w'))
    grind
  generalize ha : f.plane.eval w * f.plane.n.x = a at hab
  generalize hb : f.plane.eval w' * f.plane.n.x = b at hab
  have hK' : a < b := by rw [← ha, ← hb]; grind
  have hb0 : 0 ≤ b := by
    apply Classical.byContradiction; intro hn
    have := Rat.mul_pos (by grind : (0:Rat) < -a) (by grind : (0:Rat) < -b); grind
  have h2 : aheadB f w' = false := by
    rw [ahead_def]; apply decide_eq_false; rw [Rat.neg_mul, hb]; grind
  unfold piercedE; rw [h2, Bool.false_and, Bool.xor_false]
  cases h1 : aheadB f w
  · rw [Bool.false_and]
    rw [ahead_def, decide_eq_false_iff_not, Rat.neg_mul, ha] at h1
    have ha0 : a = 0 := by
      apply Classical.byContradiction; intro hn
      have := Rat.mul_pos (by grind : (0:Rat) < a) (by grind : (0:Rat) < b); grind
    have hw0 : f.plane.eval w = 0 := by
      rw [ha0] at ha; rcases Rat.mul_eq_zero.mp ha with h | h; exact h; exact absurd h hnx
    have hl := (crossL_facts f.plane w w' hD hs)
    have hm : crossPt f.plane w w' = w := by unfold crossPt; rw [hl.2.2.2.1.mpr hw0, ptv_zero]
    rw [hm] at hout
    exact (chi_out f hflat hnx w hw0 hout).symm
  · rfl

theorem term_pos (f : Face) (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hnx : f.plane.n.x ≠ 0) (w w' : V3) (hD : f.plane.eval w ≠ f.plane.eval w')
    (hs : f.plane.eval w * f.plane.eval w' ≤ 0) (hout : OutBox f (crossPt f.plane w w'))
    (hK : 0 < (f.plane.eval w - f.plane.eval w') * f.plane.n.x) :
    (piercedE f w ^^ piercedE f w') = chiE f w' := by
  have hab : (f.plane.eval w * f.plane.n.x) * (f.plane.eval w' * f.plane.n.x) ≤ 0 := by
    rw [show (f.plane.eval w * f.plane.n.x) * (f.plane.eval w' * f.plane.n.x) =
      (f.plane.n.x * f.plane.n.x) * (f.plane.eval w * f.plane.eval w') by grind]
    have := Rat.mul_nonneg (mul_self_nonneg' f.plane.n.x) (by grind : (0:Rat) ≤ -(f.plane.eval w * f.plane.eval w'))
    grind
  generalize ha : f.plane.eval w * f.plane.n.x = a at hab
  generalize hb : f.plane.eval w' * f.plane.n.x = b at hab
  have hK' : b < a := by rw [← ha, ← hb]; grind
  have ha0 : 0 ≤ a := by
    apply Classical.byContradiction; intro hn
    have := Rat.mul_pos (by grind : (0:Rat) < -a) (by grind : (0:Rat) < -b); grind
  have h1 : aheadB f w = false := by
    rw [ahead_def]; apply decide_eq_false; rw [Rat.neg_mul, ha]; grind
  unfold piercedE; rw [h1, Bool.false_and, Bool.false_xor]
  cases h2 : aheadB f w'
  · rw [Bool.false_and]
    rw [ahead_def, decide_eq_false_iff_not, Rat.neg_mul, hb] at h2
    have hb0 : b = 0 := by
      apply Classical.byContradiction; intro hn
      have := Rat.mul_pos (by grind : (0:Rat) < a) (by grind : (0:Rat) < b); grind
    have hw0 : f.plane.eval w' = 0 := by
      rw [hb0] at hb; rcases Rat.mul_eq_zero.mp hb with h | h; exact h; exact absurd h hnx
    have hl := (crossL_facts f.plane w w' hD hs)
    have hm : crossPt f.plane w w' = w' := by unfold crossPt; rw [hl.2.2.2.2.mpr hw0, ptv_one]
    rw [hm] at hout
    exact (chi_out f hflat hnx w' hw0 hout).symm
  · rfl

theorem exists_eps (L : List (Rat × Rat × Rat)) (l : Rat) :
    ∃ ε : Rat, 0 < ε ∧ (l < 1 → l + ε < 1) ∧ ∀ t ∈ L, ε ≤ bnd t.1 t.2.1 t.2.2 := by
  by_cases hl : l < 1
  · refine ⟨qmin (bndAll L) ((1 - l) / 2), ?_, fun _ => ?_, fun t ht => Rat.le_trans (qmin_le_left _ _) (bndAll_le L t ht)⟩
    · have := bndAll_pos L; have := div_pos' (by grind : (0:Rat) < 1 - l) (by decide : (0:Rat) < 2)
      unfold qmin; split <;> assumption
    · have := qmin_le_right (bndAll L) ((1 - l) / 2)
      have h2 := Rat.div_mul_cancel (a := 1 - l) (b := 2) (by decide)
      grind
  · exact ⟨bndAll L, bndAll_pos L, fun h => absurd h hl, fun t ht => bndAll_le L t ht⟩

theorem ptv_shift (w w' : V3) (l ε : Rat) :
    (ptv w w' (l + ε)).y = (ptv w w' l).y + ε * (w'.y - w.y) ∧
    (ptv w w' (l + ε)).z = (ptv w w' l).z + ε * (w'.z - w.z) := by
  simp only [ptv]; constructor <;> grind

theorem st_of_strict {h : Rat} {a b : V3} (hs : (a.z - h) * (b.z - h) < 0) : st h a b = true := by
  unfold st
  rcases (mul_neg_iff _ _ (by intro h0; rw [h0] at hs; grind) (by intro h0; rw [h0] at hs; grind)).mp hs
    with ⟨h1, h2⟩ | ⟨h1, h2⟩
  · have : ¬a.z > h := by grind
    have : b.z > h := by grind
    simp_all
  · have : a.z > h := by grind
    have : ¬b.z > h := by grind
    simp_all

/-- **One face, a rising move.** The per-face identity of `face_step`, for a move that may cross
the face's plane outside its box. -/
theorem face_rise (f : Face) (w w' : V3) (hw : w.z < w'.z)
    (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hgp : ∀ e ∈ edgesF f, GP w w' e.1 e.2) (hp : Pass f w w') :
    (piercedE f w ^^ piercedE f w') = xorL ((edgesF f).map (e3 w w')) := by
  rcases hp with hs | ⟨hn, h0, h0'⟩ | ⟨hD, hs, hout⟩
  · exact face_step f w w' hw hflat hs hgp
  · -- within the plane
    have p1 : piercedE f w = false := by unfold piercedE; rw [ahead_def, h0]; simp
    have p2 : piercedE f w' = false := by unfold piercedE; rw [ahead_def, h0']; simp
    rw [p1, p2]
    symm
    rw [xorL_congr (g := fun _ => false) (fun e he => by
      by_cases hnx : f.plane.n.x = 0
      · exfalso
        have hyz : f.plane.n.y ≠ 0 ∨ f.plane.n.z ≠ 0 := by
          apply Classical.byContradiction; intro h; apply hn
          have : f.plane.n.y = 0 := by grind
          have : f.plane.n.z = 0 := by grind
          cases hN : f.plane.n; simp_all
        exact (hgp e he).fa (Fs_plane f.plane hnx hyz h0 h0' (hflat e he).1)
      · unfold e3
        cases hc : crossS w w' e.1 e.2
        · rfl
        · obtain ⟨_, _, hy, hz⟩ := over_S w w' e.1 e.2 hw hc
          have hE := plane_zero f.plane e.1 e.2 (f3 w w' e.1 e.2).root (hflat e he).1 (hflat e he).2
          have hB : f.plane.eval (ptv w w' (lamS w w' e.1 e.2)) = 0 := by rw [eval_ptv, h0, h0']; grind
          have := plane_x_eq f.plane hnx hB hE hy hz
          rw [this]; simp)]
    exact xorL_false _
  · have hl := crossL_facts f.plane w w' hD hs
    by_cases hnx : f.plane.n.x = 0
    · -- parallel to +x: never pierced, and the counted crossings pair off
      have p1 : piercedE f w = false := by unfold piercedE; rw [ahead_def, hnx]; simp
      have p2 : piercedE f w' = false := by unfold piercedE; rw [ahead_def, hnx]; simp
      rw [p1, p2, xorL_congr (fun e he => e3_vert f hnx w w' hw hD e (hflat e he))]
      have hyz := n_yz hnx hD
      obtain ⟨k, hk, hb⟩ := hout
      -- a crossing point is over the crossing
      have hover : ∀ e ∈ edgesF f, crossS w w' e.1 e.2 = true →
          (ptv e.1 e.2 (f3 w w' e.1 e.2).root).y = (crossPt f.plane w w').y ∧
          (ptv e.1 e.2 (f3 w w' e.1 e.2).root).z = (crossPt f.plane w w').z ∧
          k.dot (ptv e.1 e.2 (f3 w w' e.1 e.2).root) < k.dot (crossPt f.plane w w') := by
        intro e he hc
        obtain ⟨_, _, hy, hz⟩ := over_S w w' e.1 e.2 hw hc
        have hE := plane_zero f.plane e.1 e.2 (f3 w w' e.1 e.2).root (hflat e he).1 (hflat e he).2
        have hlam := split_vert f.plane hnx w w' _ _ hD hE hy hz
        have hr := root01 hc
        have hb' : Below f k (ptv w w' (lamS w w' e.1 e.2)) := by unfold crossPt at hb; rw [← hlam] at hb; exact hb
        unfold crossPt; rw [← hlam]
        exact ⟨hy.symm, hz.symm, below_ptv hb' he (Rat.le_of_lt hr.1) (Rat.le_of_lt hr.2)⟩
      simp only [axes6, List.mem_cons, List.mem_nil_iff, or_false] at hk
      rcases hk with rfl | rfl | rfl | rfl | rfl | rfl
      · rw [xorL_congr (g := fun _ => false) (fun e he => by
          cases hc : crossS w w' e.1 e.2
          · rfl
          · have := (hover e he hc).2.2; simp only [V3.dot] at this
            simp only [Bool.true_and, decide_eq_false_iff_not]; grind)]
        exact (xorL_false _).symm
      · rw [xorL_congr (g := fun e => crossS w w' e.1 e.2) (fun e he => by
          cases hc : crossS w w' e.1 e.2
          · rfl
          · have := (hover e he hc).2.2; simp only [V3.dot] at this
            simp only [Bool.true_and, decide_eq_true_iff]; grind)]
        rw [← chi_seg f w w' hw hgp, chi_vert f hflat hnx hyz, chi_vert f hflat hnx hyz]
      all_goals
        rw [xorL_congr (g := fun _ => false) (fun e he => by
          cases hc : crossS w w' e.1 e.2
          · rfl
          · have := hover e he hc; simp only [V3.dot] at this; grind)]
        exact (xorL_false _).symm
    · -- across the plane: split a little past the crossing
      generalize hL : crossL f.plane w w' = l at hl
      have hM : crossPt f.plane w w' = ptv w w' l := by unfold crossPt; rw [hL]
      have hm0 : f.plane.eval (ptv w w' l) = 0 := hM ▸ hl.2.2.1
      have hout' : OutBox f (ptv w w' l) := hM ▸ hout
      have hchim : chiE f (ptv w w' l) = false := chi_out f hflat hnx _ hm0 hout'
      have he3 := fun e (he : e ∈ edgesF f) => e3_nx f hnx w w' hw hD e (hflat e he)
      rw [hM] at he3
      obtain ⟨ε, hε, hε1, hb⟩ := exists_eps (epsPolys f w w' (ptv w w' l)) l
      have hsh := ptv_shift w w' l ε
      obtain ⟨fv, fc, fz⟩ := eps_facts f hflat hnx w w' hw l ε hε hb hm0 hout' _ hsh.1 hsh.2
      have hchiP : chiE f (ptv w w' (l + ε)) = false := by
        rw [← hchim]; unfold chiE; exact xorL_congr fun e he => (fc e he).1
      have hdz : 0 < w'.z - w.z := by grind
      rcases ne_cases (show (f.plane.eval w - f.plane.eval w') * f.plane.n.x ≠ 0 from
          fun h => by rcases Rat.mul_eq_zero.mp h with h | h; exact hD (by grind); exact hnx h) with hK | hK
      · rw [term_neg f hflat hnx w w' hD hs hout hK]
        have hgpP : ∀ e ∈ edgesF f, GP w (ptv w w' (l + ε)) e.1 e.2 := by
          intro e he
          have g := hgp e he
          exact ⟨g.a1, g.b1, (fv e he).1, (fv e he).2.1,
            by rw [Fs_left]; exact fun h => by rcases Rat.mul_eq_zero.mp h with h | h; grind; exact g.fa h,
            by rw [Fs_left]; exact fun h => by rcases Rat.mul_eq_zero.mp h with h | h; grind; exact g.fb h,
            g.d1, fun h => (fc e he).2 (st_of_strict h)⟩
        have hwP : w.z < (ptv w w' (l + ε)).z := by
          rw [hsh.2]; simp only [ptv]
          have := Rat.mul_nonneg hl.1 (Rat.le_of_lt hdz); have := Rat.mul_pos hε hdz; grind
        rw [← Bool.xor_false (chiE f w), ← hchiP, chi_seg f w _ hwP hgpP]
        apply xorL_congr; intro e he
        symm
        rw [he3 e he]
        apply Bool.eq_iff_iff.mpr
        rw [Bool.and_eq_true, decide_eq_true_iff, crossS_left w w' e.1 e.2 (l + ε) (by grind), crossS_iff]
        have hlt : (ptv w w' l).z ≤ w'.z := by
          simp only [ptv]; have := Rat.mul_le_mul_of_nonneg_left hl.2.1 (Rat.le_of_lt hdz); grind
        constructor
        · rintro ⟨hc, hpos⟩
          have hlt : zr w w' e.1 e.2 < (ptv w w' l).z := by
            apply Classical.byContradiction; intro hn
            have := Rat.mul_nonneg (by grind : (0:Rat) ≤ zr w w' e.1 e.2 - (ptv w w' l).z)
              (by grind : (0:Rat) ≤ -((f.plane.eval w - f.plane.eval w') * f.plane.n.x))
            grind
          exact ⟨hc.1, hc.2.1, ((fz e he hc.1).2).mpr hlt⟩
        · rintro ⟨h1, h2, h3⟩
          have hlt' := ((fz e he h1).2).mp h3
          have := Rat.mul_pos (by grind : (0:Rat) < -(zr w w' e.1 e.2 - (ptv w w' l).z))
            (by grind : (0:Rat) < -((f.plane.eval w - f.plane.eval w') * f.plane.n.x))
          exact ⟨⟨h1, h2, by grind⟩, by grind⟩
      · rw [term_pos f hflat hnx w w' hD hs hout hK]
        by_cases hl1 : l < 1
        · have hε1' := hε1 hl1
          have hgpP : ∀ e ∈ edgesF f, GP (ptv w w' (l + ε)) w' e.1 e.2 := by
            intro e he
            have g := hgp e he
            exact ⟨(fv e he).1, (fv e he).2.1, g.a2, g.b2,
              by rw [Fs_right]; exact fun h => by rcases Rat.mul_eq_zero.mp h with h | h; grind; exact g.fa h,
              by rw [Fs_right]; exact fun h => by rcases Rat.mul_eq_zero.mp h with h | h; grind; exact g.fb h,
              fun h => (fc e he).2 (st_of_strict h), g.d2⟩
          have hPz : (ptv w w' (l + ε)).z < w'.z := by
            simp only [ptv]; have := Rat.mul_lt_mul_of_pos_left hε1' hdz; grind
          rw [← Bool.false_xor (chiE f w'), ← hchiP, chi_seg f _ w' hPz hgpP]
          apply xorL_congr; intro e he
          symm
          rw [he3 e he]
          apply Bool.eq_iff_iff.mpr
          rw [Bool.and_eq_true, decide_eq_true_iff, crossS_right w w' e.1 e.2 (l + ε) hε1', crossS_iff]
          have hge : w.z ≤ (ptv w w' l).z := by
            simp only [ptv]; have := Rat.mul_le_mul_of_nonneg_left hl.1 (Rat.le_of_lt hdz); grind
          constructor
          · rintro ⟨hc, hpos⟩
            have hgt : (ptv w w' l).z < zr w w' e.1 e.2 := by
              apply Classical.byContradiction; intro hn
              have := Rat.mul_nonneg (by grind : (0:Rat) ≤ -(zr w w' e.1 e.2 - (ptv w w' l).z))
                (by grind : (0:Rat) ≤ (f.plane.eval w - f.plane.eval w') * f.plane.n.x)
              grind
            have k := fz e he hc.1
            exact ⟨hc.1, by
              apply Classical.byContradiction; intro hn
              exact absurd (k.2.mp (by grind)) (by grind), hc.2.2⟩
          · rintro ⟨h1, h2, h3⟩
            have k := fz e he h1
            have hgt : (ptv w w' l).z < zr w w' e.1 e.2 := by
              apply Classical.byContradiction; intro hn
              have := k.2.mpr (by have := zr_ne f hflat hnx w w' hw l hm0 hout' he h1; grind)
              grind
            have := Rat.mul_pos (by grind : (0:Rat) < zr w w' e.1 e.2 - (ptv w w' l).z) hK
            have hP : (ptv w w' l).z < (ptv w w' (l + ε)).z := by
              rw [hsh.2]; have := Rat.mul_pos hε hdz; grind
            exact ⟨⟨h1, by grind, h3⟩, by grind⟩
        · -- the crossing is at `w'`: nothing past it, and `w'` is outside the box
          have hl1' : l = 1 := by grind
          subst hl1'
          rw [ptv_one] at hchim
          rw [hchim]
          symm
          rw [xorL_congr (g := fun _ => false) (fun e he => by
            rw [he3 e he, ptv_one]
            cases hc : crossS w w' e.1 e.2
            · rfl
            · have := (crossS_iff _ _ _ _).mp hc
              simp only [Bool.true_and, decide_eq_false_iff_not]
              intro h
              have := Rat.mul_nonneg (by grind : (0:Rat) ≤ -(zr w w' e.1 e.2 - w'.z)) (Rat.le_of_lt hK)
              grind)]
          exact xorL_false _

end ForgeExact.Jordan
