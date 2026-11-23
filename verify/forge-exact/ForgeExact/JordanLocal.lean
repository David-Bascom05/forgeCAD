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

/-! ## One face, a level move across its plane -/

theorem chi_lvl (f : Face) (w w' : V3) (hz : w.z = w'.z) (hy : w.y < w'.y)
    (hgp : ∀ e ∈ edgesF f, GPH w w' e.1 e.2) :
    (chiE f w ^^ chiE f w') = xorL ((edgesF f).map fun e => crossH w w' e.1 e.2) := by
  unfold chiE
  have per : ∀ e ∈ edgesF f, (crossE w e.1 e.2 ^^ crossE w' e.1 e.2) = crossH w w' e.1 e.2 :=
    fun e he => seg_level w w' e.1 e.2 hz hy (hgp e he).a1 (hgp e he).b1 (hgp e he).d2
  rw [← xorL_map_xor (edgesF f) (fun e => crossE w e.1 e.2) (fun e => crossE w' e.1 e.2)]
  exact xorL_congr per

def yr (w a b : V3) : Rat := (ptv a b (f1 w a b).root).y

theorem crossH_iff (w w' a b : V3) : crossH w w' a b = true ↔
    (f1 w a b).α * (f1 w a b).γ < 0 ∧ w.y < yr w a b ∧ yr w a b < w'.y := by
  unfold crossH yr; rw [decide_eq_true_iff]

theorem f1_lvl {w m : V3} (h : m.z = w.z) (a b : V3) : f1 m a b = f1 w a b := by simp [f1, h]

theorem eH_nx (f : Face) (hnx : f.plane.n.x ≠ 0) (w w' : V3) (hz : w.z = w'.z) (hy : w.y < w'.y)
    (hD : f.plane.eval w ≠ f.plane.eval w') (e : V3 × V3)
    (hfe : f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0) :
    eH w w' e = (crossH w w' e.1 e.2 && decide (0 < (yr w e.1 e.2 - (crossPt f.plane w w').y) *
      ((f.plane.eval w - f.plane.eval w') * f.plane.n.x))) := by
  unfold eH
  cases hc : crossH w w' e.1 e.2
  · rfl
  · obtain ⟨l0, l1, hy', hz'⟩ := over_H w w' e.1 e.2 hz hy hc
    have hE := plane_zero f.plane e.1 e.2 (f1 w e.1 e.2).root hfe.1 hfe.2
    simp only [Bool.true_and]
    rw [Bool.eq_iff_iff, decide_eq_true_iff, decide_eq_true_iff,
      split_x f.plane hnx w w' _ _ hD hE hy' hz']
    have hyr : yr w e.1 e.2 - (crossPt f.plane w w').y =
        (lamH w w' e.1 e.2 - crossL f.plane w w') * (w'.y - w.y) := by
      unfold yr crossPt; rw [← hy']; simp only [ptv]; grind
    rw [hyr, show (lamH w w' e.1 e.2 - crossL f.plane w w') * (w'.y - w.y) *
        ((f.plane.eval w - f.plane.eval w') * f.plane.n.x) = (w'.y - w.y) *
        ((lamH w w' e.1 e.2 - crossL f.plane w w') * ((f.plane.eval w - f.plane.eval w') * f.plane.n.x))
        by grind, mul_pos_iff_left (k := w'.y - w.y) (by grind)]

theorem d0_out_lvl (f : Face) (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hnx : f.plane.n.x ≠ 0) {m : V3} (hm : f.plane.eval m = 0) (hout : OutBox f m) {e : V3 × V3}
    (he : e ∈ edgesF f) (hs : (e.1.z - m.z) * (e.2.z - m.z) < 0) : d0 m e.1 e.2 ≠ 0 :=
  d0_out f hflat hnx hm hout he (st_of_strict hs)

theorem face_level' (f : Face) (w w' : V3) (hz : w.z = w'.z) (hy : w.y < w'.y)
    (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0)
    (hgp : ∀ e ∈ edgesF f, GPH w w' e.1 e.2) (hp : Pass f w w') :
    (piercedE f w ^^ piercedE f w') = xorL ((edgesF f).map (eH w w')) := by
  rcases hp with hs | ⟨hn, h0, h0'⟩ | ⟨hD, hs, hout⟩
  · exact face_level f w w' hz hy hflat hs hgp
  · have p1 : piercedE f w = false := by unfold piercedE; rw [ahead_def, h0]; simp
    have p2 : piercedE f w' = false := by unfold piercedE; rw [ahead_def, h0']; simp
    rw [p1, p2]
    symm
    rw [xorL_congr (g := fun _ => false) (fun e he => by
      by_cases hnx : f.plane.n.x = 0
      · exfalso
        have hny : f.plane.n.y = 0 := by
          simp only [Plane.eval, V3.dot, hnx] at h0 h0'
          rw [← hz] at h0'
          exact mul_cancel (k := w'.y - w.y) (by grind) (by grind)
        have hnz : f.plane.n.z ≠ 0 := fun h => hn (by cases hN : f.plane.n; simp_all)
        have ha := (hflat e he).1
        simp only [Plane.eval, V3.dot, hnx, hny] at ha h0
        exact (hgp e he).a1 (mul_cancel hnz (by grind))
      · unfold eH
        cases hc : crossH w w' e.1 e.2
        · rfl
        · obtain ⟨_, _, hy', hz'⟩ := over_H w w' e.1 e.2 hz hy hc
          have hE := plane_zero f.plane e.1 e.2 (f1 w e.1 e.2).root (hflat e he).1 (hflat e he).2
          have hB : f.plane.eval (ptv w w' (lamH w w' e.1 e.2)) = 0 := by rw [eval_ptv, h0, h0']; grind
          have := plane_x_eq f.plane hnx hB hE hy' hz'
          rw [this]; simp)]
    exact xorL_false _
  · have hl := crossL_facts f.plane w w' hD hs
    have hmz : (crossPt f.plane w w').z = w.z := by unfold crossPt; simp only [ptv]; rw [← hz]; grind
    by_cases hnx : f.plane.n.x = 0
    · have p1 : piercedE f w = false := by unfold piercedE; rw [ahead_def, hnx]; simp
      have p2 : piercedE f w' = false := by unfold piercedE; rw [ahead_def, hnx]; simp
      rw [p1, p2]
      have hyz := n_yz hnx hD
      obtain ⟨k, hk, hb⟩ := hout
      have hover : ∀ e ∈ edgesF f, crossH w w' e.1 e.2 = true →
          eH w w' e = decide ((crossPt f.plane w w').x < (ptv e.1 e.2 (f1 w e.1 e.2).root).x) ∧
          (ptv e.1 e.2 (f1 w e.1 e.2).root).y = (crossPt f.plane w w').y ∧
          (ptv e.1 e.2 (f1 w e.1 e.2).root).z = (crossPt f.plane w w').z ∧
          k.dot (ptv e.1 e.2 (f1 w e.1 e.2).root) < k.dot (crossPt f.plane w w') := by
        intro e he hc
        obtain ⟨_, _, hy', hz'⟩ := over_H w w' e.1 e.2 hz hy hc
        have hE := plane_zero f.plane e.1 e.2 (f1 w e.1 e.2).root (hflat e he).1 (hflat e he).2
        have hlam := split_vert f.plane hnx w w' _ _ hD hE hy' hz'
        have hc' := (crossH_iff w w' e.1 e.2).mp hc
        have hr := Aff.roots _ ((mul_neg_iff _ _ (by intro h0; rw [h0] at hc'; grind)
          (by intro h0; rw [h0] at hc'; grind)).mp hc'.1)
        have hb' : Below f k (ptv w w' (lamH w w' e.1 e.2)) := by unfold crossPt at hb; rw [← hlam] at hb; exact hb
        unfold eH crossPt; rw [← hlam, hc, Bool.true_and]
        exact ⟨rfl, hy'.symm, hz'.symm, below_ptv hb' he (Rat.le_of_lt hr.1) (Rat.le_of_lt hr.2)⟩
      simp only [axes6, List.mem_cons, List.mem_nil_iff, or_false] at hk
      rcases hk with rfl | rfl | rfl | rfl | rfl | rfl
      · rw [xorL_congr (g := fun _ => false) (fun e he => by
          cases hc : crossH w w' e.1 e.2
          · unfold eH; rw [hc]; rfl
          · have := hover e he hc; rw [this.1]; simp only [V3.dot] at this
            simp only [decide_eq_false_iff_not]; grind)]
        exact (xorL_false _).symm
      · rw [xorL_congr (g := fun e => crossH w w' e.1 e.2) (fun e he => by
          cases hc : crossH w w' e.1 e.2
          · unfold eH; rw [hc]; rfl
          · have := hover e he hc; rw [this.1]; simp only [V3.dot] at this
            simp only [decide_eq_true_iff]; grind)]
        rw [← chi_lvl f w w' hz hy hgp, chi_vert f hflat hnx hyz, chi_vert f hflat hnx hyz]
      all_goals
        rw [xorL_congr (g := fun _ => false) (fun e he => by
          cases hc : crossH w w' e.1 e.2
          · unfold eH; rw [hc]; rfl
          · have := hover e he hc; simp only [V3.dot] at this; grind)]
        exact (xorL_false _).symm
    · generalize hL : crossL f.plane w w' = l at hl
      have hM : crossPt f.plane w w' = ptv w w' l := by unfold crossPt; rw [hL]
      have hm0 : f.plane.eval (ptv w w' l) = 0 := hM ▸ hl.2.2.1
      have hout' : OutBox f (ptv w w' l) := hM ▸ hout
      have hchim : chiE f (ptv w w' l) = false := chi_out f hflat hnx _ hm0 hout'
      rw [hM] at hmz
      have he := fun e (he : e ∈ edgesF f) => eH_nx f hnx w w' hz hy hD e (hflat e he)
      rw [hM] at he
      have hdy : 0 < w'.y - w.y := by grind
      have hmy : (ptv w w' l).y = w.y + l * (w'.y - w.y) := by simp only [ptv]
      rcases ne_cases (show (f.plane.eval w - f.plane.eval w') * f.plane.n.x ≠ 0 from
          fun h => by rcases Rat.mul_eq_zero.mp h with h | h; exact hD (by grind); exact hnx h) with hK | hK
      · rw [term_neg f hflat hnx w w' hD hs hout hK]
        by_cases hl0 : l = 0
        · subst hl0
          rw [ptv_zero] at hchim hmy
          rw [hchim]; symm
          rw [xorL_congr (g := fun _ => false) (fun e he' => by
            rw [he e he']
            cases hc : crossH w w' e.1 e.2
            · rfl
            · have := (crossH_iff _ _ _ _).mp hc
              simp only [Bool.true_and, decide_eq_false_iff_not]; intro h
              rw [ptv_zero, Rat.mul_comm, mul_pos_iff_neg hK] at h; grind)]
          exact xorL_false _
        · have hgp' : ∀ e ∈ edgesF f, GPH w (ptv w w' l) e.1 e.2 := fun e he' =>
            ⟨(hgp e he').a1, (hgp e he').b1, fun h => d0_out_lvl f hflat hnx hm0 hout' he' (by rw [hmz]; exact h)⟩
          have hlt : w.y < (ptv w w' l).y := by
            rw [hmy]; have := Rat.mul_pos (by grind : (0:Rat) < l) hdy; grind
          rw [← Bool.xor_false (chiE f w), ← hchim, chi_lvl f w _ hmz.symm hlt hgp']
          apply xorL_congr; intro e he'
          symm
          rw [he e he']
          apply Bool.eq_iff_iff.mpr
          rw [Bool.and_eq_true, decide_eq_true_iff, crossH_iff, crossH_iff]
          have hle : (ptv w w' l).y ≤ w'.y := by
            rw [hmy]; have := Rat.mul_le_mul_of_nonneg_left hl.2.1 (Rat.le_of_lt hdy); grind
          constructor
          · rintro ⟨hc, hpos⟩
            refine ⟨hc.1, hc.2.1, ?_⟩
            apply Classical.byContradiction; intro hn
            have := Rat.mul_nonneg (by grind : (0:Rat) ≤ yr w e.1 e.2 - (ptv w w' l).y)
              (by grind : (0:Rat) ≤ -((f.plane.eval w - f.plane.eval w') * f.plane.n.x))
            grind
          · rintro ⟨h1, h2, h3⟩
            have := Rat.mul_pos (by grind : (0:Rat) < -(yr w e.1 e.2 - (ptv w w' l).y))
              (by grind : (0:Rat) < -((f.plane.eval w - f.plane.eval w') * f.plane.n.x))
            exact ⟨⟨h1, h2, by grind⟩, by grind⟩
      · rw [term_pos f hflat hnx w w' hD hs hout hK]
        by_cases hl1 : l = 1
        · subst hl1
          rw [ptv_one] at hchim hmy
          rw [hchim]; symm
          rw [xorL_congr (g := fun _ => false) (fun e he' => by
            rw [he e he']
            cases hc : crossH w w' e.1 e.2
            · rfl
            · have := (crossH_iff _ _ _ _).mp hc
              simp only [Bool.true_and, decide_eq_false_iff_not]; intro h
              rw [ptv_one, Rat.mul_comm, mul_pos_iff_left hK] at h; grind)]
          exact xorL_false _
        · have hgp' : ∀ e ∈ edgesF f, GPH (ptv w w' l) w' e.1 e.2 := fun e he' =>
            ⟨by rw [hmz]; exact (hgp e he').a1, by rw [hmz]; exact (hgp e he').b1,
              fun h => (hgp e he').d2 (by rw [hmz] at h; exact h)⟩
          have hlt : (ptv w w' l).y < w'.y := by
            rw [hmy]; have := Rat.mul_pos (by grind : (0:Rat) < 1 - l) hdy; grind
          rw [← Bool.false_xor (chiE f w'), ← hchim, chi_lvl f _ w' (by rw [hmz, hz]) hlt hgp']
          apply xorL_congr; intro e he'
          symm
          rw [he e he']
          apply Bool.eq_iff_iff.mpr
          rw [Bool.and_eq_true, decide_eq_true_iff, crossH_iff, crossH_iff]
          have hf1 := f1_lvl hmz e.1 e.2
          have hyr : yr (ptv w w' l) e.1 e.2 = yr w e.1 e.2 := by unfold yr; rw [hf1]
          rw [hf1, hyr]
          have hge : w.y ≤ (ptv w w' l).y := by
            rw [hmy]; have := Rat.mul_nonneg hl.1 (Rat.le_of_lt hdy); grind
          constructor
          · rintro ⟨hc, hpos⟩
            refine ⟨hc.1, ?_, hc.2.2⟩
            apply Classical.byContradiction; intro hn
            have := Rat.mul_nonneg (by grind : (0:Rat) ≤ -(yr w e.1 e.2 - (ptv w w' l).y)) (Rat.le_of_lt hK)
            grind
          · rintro ⟨h1, h2, h3⟩
            have := Rat.mul_pos (by grind : (0:Rat) < yr w e.1 e.2 - (ptv w w' l).y) hK
            exact ⟨⟨h1, by grind, h3⟩, by grind⟩

/-! ## One face, a move along +x -/

theorem chiE_yz (f : Face) {q q' : V3} (hy : q.y = q'.y) (hz : q.z = q'.z) : chiE f q = chiE f q' := by
  unfold chiE
  apply xorL_congr
  intro e _
  simp only [crossE, d0, hy, hz]
  try rfl

theorem face_x (f : Face) (w w' : V3) (hy : w.y = w'.y) (hz : w.z = w'.z)
    (hflat : ∀ e ∈ edgesF f, f.plane.eval e.1 = 0 ∧ f.plane.eval e.2 = 0) (hp : Pass f w w') :
    piercedE f w = piercedE f w' := by
  have hchi := chiE_yz f hy hz
  rcases hp with hs | ⟨_, h0, h0'⟩ | ⟨hD, hs, hout⟩
  · have hahead : aheadB f w = aheadB f w' := by
      unfold aheadB; exact decide_eq_decide.mpr (ahead_same _ _ _ hs)
    simp only [piercedE, hchi, hahead]
  · unfold piercedE; rw [ahead_def, ahead_def, h0, h0']; simp
  · by_cases hnx : f.plane.n.x = 0
    · unfold piercedE; rw [ahead_def, ahead_def, hnx]; simp
    · have hl := crossL_facts f.plane w w' hD hs
      have hm := chi_out f hflat hnx _ hl.2.2.1 hout
      have : chiE f (crossPt f.plane w w') = chiE f w :=
        chiE_yz f (by unfold crossPt; simp only [ptv]; rw [← hy]; grind)
          (by unfold crossPt; simp only [ptv]; rw [← hz]; grind)
      unfold piercedE; rw [← hchi, ← this, hm]; simp

/-! ## The moves, across planes -/

theorem move_rise' (S : Solid) (hflat : FacesFlat S) (hcl : Closed S) (w w' : V3) (hw : w.z < w'.z)
    (hp : ∀ f ∈ facesL S, Pass f w w')
    (hgp : ∀ f ∈ facesL S, ∀ e ∈ edgesF f, GP w w' e.1 e.2) :
    containsE S w = containsE S w' := by
  apply xorL_eq_of_xor
  unfold containsE
  rw [← xorL_map_xor, xorL_congr fun f hf => face_rise f w w' hw (hflat f hf) (hgp f hf) (hp f hf),
    ← xorL_flatMap]
  exact xor_even (e3 w w') (e3_symm w w') _ _ (Nat.le_refl _) hcl

theorem move_level' (S : Solid) (hflat : FacesFlat S) (hcl : Closed S) (w w' : V3)
    (hz : w.z = w'.z) (hy : w.y < w'.y) (hp : ∀ f ∈ facesL S, Pass f w w')
    (hgp : ∀ f ∈ facesL S, ∀ e ∈ edgesF f, GPH w w' e.1 e.2) :
    containsE S w = containsE S w' := by
  apply xorL_eq_of_xor
  unfold containsE
  rw [← xorL_map_xor, xorL_congr fun f hf => face_level' f w w' hz hy (hflat f hf) (hgp f hf) (hp f hf),
    ← xorL_flatMap]
  exact xor_even (eH w w') (eH_symm w w') _ _ (Nat.le_refl _) hcl

theorem move_x' (S : Solid) (hflat : FacesFlat S) (w w' : V3) (hy : w.y = w'.y) (hz : w.z = w'.z)
    (hp : ∀ f ∈ facesL S, Pass f w w') :
    containsE S w = containsE S w' := by
  unfold containsE
  exact xorL_congr fun f hf => face_x f w w' hy hz (hflat f hf) (hp f hf)

/-! ## Outside the box survives the perturbation -/

/-- A point moving with `δ` as `m0 + δ u + δ² v`. -/
def pp (m0 u v : V3) (δ : Rat) : V3 :=
  ⟨m0.x + δ * u.x + δ * δ * v.x, m0.y + δ * u.y + δ * δ * v.y, m0.z + δ * u.z + δ * δ * v.z⟩

/-- The quadratics that keep a moving point on the far side of every vertex, along every axis. -/
def outPolys (f : Face) (m0 u v : V3) : List (Rat × Rat × Rat) :=
  (edgesF f).flatMap fun e => [e.1, e.2].flatMap fun x =>
    axes6.map fun k => (k.dot m0 - k.dot x, k.dot u, k.dot v)

theorem dot_pp (k m0 u v x : V3) (δ : Rat) :
    k.dot (pp m0 u v δ) - k.dot x = poly (k.dot m0 - k.dot x) (k.dot u) (k.dot v) δ := by
  simp only [V3.dot, pp, poly]; grind

theorem out_perturb (f : Face) (m0 u v : V3) (δ : Rat) (h0 : 0 < δ)
    (hb : ∀ t ∈ outPolys f m0 u v, δ ≤ bnd t.1 t.2.1 t.2.2) (hout : OutBox f m0) :
    OutBox f (pp m0 u v δ) := by
  obtain ⟨k, hk, hbk⟩ := hout
  refine ⟨k, hk, fun e he => ?_⟩
  have one : ∀ x ∈ [e.1, e.2], k.dot x < k.dot m0 → k.dot x < k.dot (pp m0 u v δ) := by
    intro x hx hlt
    have hb' := hb (k.dot m0 - k.dot x, k.dot u, k.dot v)
      (List.mem_flatMap.mpr ⟨e, he, List.mem_flatMap.mpr ⟨x, hx, List.mem_map.mpr ⟨k, hk, rfl⟩⟩⟩)
    have := (small_pos _ _ _ δ h0 hb').mpr (Or.inl (by grind))
    rw [← dot_pp] at this; grind
  exact ⟨one e.1 (by simp) (hbk e he).1, one e.2 (by simp) (hbk e he).2⟩

theorem eval_qd (P : Plane) (q : V3) (δ : Rat) :
    P.eval (qd q δ) = P.eval q + P.n.y * δ + P.n.z * (δ * δ) := by
  simp only [Plane.eval, V3.dot, qd]; grind

/-- Where the perturbed segment crosses a plane, as a moving point. -/
def crossMove (P : Plane) (p q : V3) : V3 × V3 × V3 :=
  let D := P.eval p - P.eval q
  let l1 := P.n.y / D
  let l2 := P.n.z / D
  (ptv p q (P.eval p / D), ⟨l1 * (q.x - p.x), l1 * (q.y - p.y) + 1, l1 * (q.z - p.z)⟩,
    ⟨l2 * (q.x - p.x), l2 * (q.y - p.y), l2 * (q.z - p.z) + 1⟩)

theorem crossPt_qd (P : Plane) (p q : V3) (δ : Rat) :
    crossPt P (qd p δ) (qd q δ) = pp (crossMove P p q).1 (crossMove P p q).2.1 (crossMove P p q).2.2 δ := by
  have hD : P.eval (qd p δ) - P.eval (qd q δ) = P.eval p - P.eval q := by rw [eval_qd, eval_qd]; grind
  unfold crossPt crossL crossMove
  rw [hD, eval_qd]
  simp only [ptv, pp, qd, Rat.div_def, V3.mk.injEq]
  refine ⟨?_, ?_, ?_⟩ <;> grind

/-- The plane point over a perturbed point of the plane. -/
def endMove (P : Plane) (p : V3) : V3 × V3 × V3 := (p, ⟨-(P.n.y / P.n.x), 1, 0⟩, ⟨-(P.n.z / P.n.x), 0, 1⟩)

theorem endMove_on (P : Plane) (hnx : P.n.x ≠ 0) (p : V3) (hp : P.eval p = 0) (δ : Rat) :
    P.eval (pp (endMove P p).1 (endMove P p).2.1 (endMove P p).2.2 δ) = 0 := by
  have h1 := Rat.div_mul_cancel (a := P.n.y) hnx
  have h2 := Rat.div_mul_cancel (a := P.n.z) hnx
  simp only [Plane.eval, V3.dot, pp, endMove] at hp ⊢
  generalize P.n.y / P.n.x = A at h1
  generalize P.n.z / P.n.x = B at h2
  grind

theorem endMove_yz (P : Plane) (p : V3) (δ : Rat) :
    (pp (endMove P p).1 (endMove P p).2.1 (endMove P p).2.2 δ).y = (qd p δ).y ∧
    (pp (endMove P p).1 (endMove P p).2.1 (endMove P p).2.2 δ).z = (qd p δ).z := by
  simp only [pp, endMove, qd]; constructor <;> grind

/-! ## The perturbation, for a segment crossing planes outside faces -/

/-- The segment `[p, q]` meets the face's plane, if at all, outside the face's box. -/
def Clear (f : Face) (p q : V3) : Prop :=
  0 < f.plane.eval p * f.plane.eval q ∨
  (f.plane.n ≠ ⟨0, 0, 0⟩ ∧ ∀ t : Rat, 0 ≤ t → t ≤ 1 → OutBox f (ptv p q t))

theorem clear_symm {f : Face} {p q : V3} (h : Clear f p q) : Clear f q p := by
  rcases h with h | ⟨hn, h⟩
  · exact Or.inl (by rw [Rat.mul_comm]; exact h)
  · refine Or.inr ⟨hn, fun t h0 h1 => ?_⟩
    have := h (1 - t) (by grind) (by grind)
    rwa [← ptv_rev, show 1 - (1 - t) = t by grind] at this

theorem clear_end {f : Face} {p q : V3} (h : Clear f p q) (hp : f.plane.eval p = 0) : OutBox f p := by
  rcases h with h | ⟨_, h⟩
  · rw [hp] at h; grind
  · have := h 0 (by decide) (by decide); rwa [ptv_zero] at this

/-- Every quadratic the local argument needs, for one face and an ordered pair. -/
def crossPolys (f : Face) (p q : V3) : List (Rat × Rat × Rat) :=
  outPolys f (crossMove f.plane p q).1 (crossMove f.plane p q).2.1 (crossMove f.plane p q).2.2 ++
  outPolys f (endMove f.plane p).1 (endMove f.plane p).2.1 (endMove f.plane p).2.2

def localPolys (S : Solid) (p q : V3) : List (Rat × Rat × Rat) :=
  (facesL S).flatMap fun f => crossPolys f p q ++ crossPolys f q p

theorem side_perturb (P : Plane) (p q : V3) (δ : Rat) (h0 : 0 < δ)
    (hbp : δ ≤ bnd (P.c - P.n.dot p) (-P.n.y) (-P.n.z)) (hbq : δ ≤ bnd (P.c - P.n.dot q) (-P.n.y) (-P.n.z))
    (hs : 0 < P.eval p * P.eval q) : 0 < P.eval (qd p δ) * P.eval (qd q δ) := by
  have hp := plane_perturb P p δ h0 hbp (fun h => by rw [h] at hs; grind)
  have hq := plane_perturb P q δ h0 hbq (fun h => by rw [h] at hs; grind)
  have e1 : ∀ x : V3, P.eval x = -(P.c - P.n.dot x) := by intro x; simp only [Plane.eval]; grind
  rw [e1, e1] at hs ⊢
  rw [show -(P.c - P.n.dot (qd p δ)) * -(P.c - P.n.dot (qd q δ)) =
    (P.c - P.n.dot (qd p δ)) * (P.c - P.n.dot (qd q δ)) by grind]
  rw [show -(P.c - P.n.dot p) * -(P.c - P.n.dot q) = (P.c - P.n.dot p) * (P.c - P.n.dot q) by grind] at hs
  exact chain_pos hp hq hs

theorem pass_perturb (f : Face) (p q : V3) (δ : Rat) (h0 : 0 < δ)
    (hbp : δ ≤ bnd (f.plane.c - f.plane.n.dot p) (-f.plane.n.y) (-f.plane.n.z))
    (hbq : δ ≤ bnd (f.plane.c - f.plane.n.dot q) (-f.plane.n.y) (-f.plane.n.z))
    (hb : ∀ t ∈ crossPolys f p q, δ ≤ bnd t.1 t.2.1 t.2.2) (hc : Clear f p q) :
    Pass f (qd p δ) (qd q δ) := by
  rcases hc with hs | ⟨hn, hout⟩
  · exact Or.inl (side_perturb f.plane p q δ h0 hbp hbq hs)
  · by_cases h1 : 0 < f.plane.eval (qd p δ) * f.plane.eval (qd q δ)
    · exact Or.inl h1
    · have hD' : f.plane.eval (qd p δ) - f.plane.eval (qd q δ) = f.plane.eval p - f.plane.eval q := by
        rw [eval_qd, eval_qd]; grind
      by_cases h2 : f.plane.eval (qd p δ) = f.plane.eval (qd q δ)
      · refine Or.inr (Or.inl ⟨hn, ?_⟩)
        rw [← h2] at h1 ⊢
        have : f.plane.eval (qd p δ) = 0 := by
          apply Classical.byContradiction; intro h; exact h1 (sq_pos h)
        exact ⟨this, this⟩
      · refine Or.inr (Or.inr ⟨h2, by grind, ?_⟩)
        have hpq : f.plane.eval p ≠ f.plane.eval q := fun h => h2 (by grind)
        have hs : f.plane.eval p * f.plane.eval q ≤ 0 := by
          apply Classical.byContradiction; intro h
          exact h1 (side_perturb f.plane p q δ h0 hbp hbq (by grind))
        have hl := crossL_facts f.plane p q hpq hs
        rw [crossPt_qd]
        apply out_perturb f _ _ _ δ h0 (fun t ht => hb t (List.mem_append_left _ ht))
        exact hout _ hl.1 hl.2.1

/-- **The reference is the exact parity at the perturbed point**, with the point allowed on a face
plane outside the face's box. -/
theorem contains_eq' (S : Solid) (q : V3) (δ : Rat) (h0 : 0 < δ)
    (hb : ∀ t ∈ solidPolys S q, δ ≤ bnd t.1 t.2.1 t.2.2)
    (hb' : ∀ f ∈ facesL S, ∀ t ∈ outPolys f (endMove f.plane q).1 (endMove f.plane q).2.1
      (endMove f.plane q).2.2, δ ≤ bnd t.1 t.2.1 t.2.2)
    (hflat : FacesFlat S) (hoff : ∀ f ∈ facesL S, f.plane.eval q = 0 → OutBox f q) :
    S.contains q = containsE S (qd q δ) := by
  unfold Solid.contains containsE
  rw [← Array.foldl_toList, fold_count, Nat.zero_add, par_len]
  apply xorL_congr
  intro f hf
  by_cases hq : f.plane.eval q = 0
  · have hr : f.pierced q = false := by
      unfold Face.pierced
      have : f.plane.c - f.plane.n.dot q = 0 := by simp only [Plane.eval] at hq; grind
      simp only [this]
      split
      · rfl
      · have : sgn (0 : Rat) = 0 := by unfold sgn; simp
        rw [this]; simp
    rw [hr]; symm
    unfold piercedE
    by_cases hnx : f.plane.n.x = 0
    · rw [ahead_def, hnx]; simp
    · have hyz := endMove_yz f.plane q δ
      rw [chiE_yz f hyz.1.symm hyz.2.symm,
        chi_out f (hflat f hf) hnx _ (endMove_on f.plane hnx q hq δ)
          (out_perturb f _ _ _ δ h0 (hb' f hf) (hoff f hf hq))]
      simp
  · exact pierced_eq f q δ h0 (fun t ht => hb t (List.mem_flatMap.mpr ⟨f, hf, ht⟩)) hq

/-- **Jordan–Brouwer along a segment that may cross face planes.** For a closed B-rep whose faces
lie in their planes, the reference's ray-parity membership is the same at the two ends of any
segment that meets each face's plane, if at all, outside the face's bounding box. -/
theorem contains_const_local (S : Solid) (hflat : FacesFlat S) (hcl : Closed S) (p q : V3)
    (hc : ∀ f ∈ facesL S, Clear f p q) :
    S.contains p = S.contains q := by
  obtain ⟨δ, h0, hbp, hbq, hbpq, hbl⟩ : ∃ δ : Rat, 0 < δ ∧
      (∀ t ∈ solidPolys S p, δ ≤ bnd t.1 t.2.1 t.2.2) ∧
      (∀ t ∈ solidPolys S q, δ ≤ bnd t.1 t.2.1 t.2.2) ∧
      (∀ t ∈ pairPolys S p q, δ ≤ bnd t.1 t.2.1 t.2.2) ∧
      (∀ t ∈ localPolys S p q, δ ≤ bnd t.1 t.2.1 t.2.2) :=
    ⟨bndAll (solidPolys S p ++ solidPolys S q ++ pairPolys S p q ++ localPolys S p q), bndAll_pos _,
      fun t ht => bndAll_le _ t (List.mem_append_left _ (List.mem_append_left _ (List.mem_append_left _ ht))),
      fun t ht => bndAll_le _ t (List.mem_append_left _ (List.mem_append_left _ (List.mem_append_right _ ht))),
      fun t ht => bndAll_le _ t (List.mem_append_left _ (List.mem_append_right _ ht)),
      fun t ht => bndAll_le _ t (List.mem_append_right _ ht)⟩
  have hcl1 : ∀ f ∈ facesL S, ∀ t ∈ crossPolys f p q, δ ≤ bnd t.1 t.2.1 t.2.2 := fun f hf t ht =>
    hbl t (List.mem_flatMap.mpr ⟨f, hf, List.mem_append_left _ ht⟩)
  have hcl2 : ∀ f ∈ facesL S, ∀ t ∈ crossPolys f q p, δ ≤ bnd t.1 t.2.1 t.2.2 := fun f hf t ht =>
    hbl t (List.mem_flatMap.mpr ⟨f, hf, List.mem_append_right _ ht⟩)
  have hplp : ∀ f ∈ facesL S, δ ≤ bnd (f.plane.c - f.plane.n.dot p) (-f.plane.n.y) (-f.plane.n.z) :=
    fun f hf => hbp (f.plane.c - f.plane.n.dot p, -f.plane.n.y, -f.plane.n.z)
      (List.mem_flatMap.mpr ⟨f, hf, List.mem_cons_self ..⟩)
  have hplq : ∀ f ∈ facesL S, δ ≤ bnd (f.plane.c - f.plane.n.dot q) (-f.plane.n.y) (-f.plane.n.z) :=
    fun f hf => hbq (f.plane.c - f.plane.n.dot q, -f.plane.n.y, -f.plane.n.z)
      (List.mem_flatMap.mpr ⟨f, hf, List.mem_cons_self ..⟩)
  rw [contains_eq' S p δ h0 hbp (fun f hf t ht => hcl1 f hf t (List.mem_append_right _ ht)) hflat
      (fun f hf hp => clear_end (hc f hf) hp),
    contains_eq' S q δ h0 hbq (fun f hf t ht => hcl2 f hf t (List.mem_append_right _ ht)) hflat
      (fun f hf hq => clear_end (clear_symm (hc f hf)) hq)]
  have hpass : ∀ f ∈ facesL S, Pass f (qd p δ) (qd q δ) := fun f hf =>
    pass_perturb f p q δ h0 (hplp f hf) (hplq f hf) (hcl1 f hf) (hc f hf)
  have hpass' : ∀ f ∈ facesL S, Pass f (qd q δ) (qd p δ) := fun f hf =>
    pass_perturb f q p δ h0 (hplq f hf) (hplp f hf) (hcl2 f hf) (clear_symm (hc f hf))
  -- the facts general position needs, at either end
  have zP : ∀ f ∈ facesL S, ∀ e ∈ edgesF f,
      e.1.z ≠ (qd p δ).z ∧ e.2.z ≠ (qd p δ).z ∧ e.1.z ≠ (qd q δ).z ∧ e.2.z ≠ (qd q δ).z := by
    intro f hf e he
    exact ⟨(z_perturb p e.1 δ h0 (hbp (e.1.z - p.z, 0, -1) (mem_solidPolys hf he (by simp [edgePolys])))).2,
      (z_perturb p e.2 δ h0 (hbp (e.2.z - p.z, 0, -1) (mem_solidPolys hf he (by simp [edgePolys])))).2,
      (z_perturb q e.1 δ h0 (hbq (e.1.z - q.z, 0, -1) (mem_solidPolys hf he (by simp [edgePolys])))).2,
      (z_perturb q e.2 δ h0 (hbq (e.2.z - q.z, 0, -1) (mem_solidPolys hf he (by simp [edgePolys])))).2⟩
  have dP : ∀ f ∈ facesL S, ∀ e ∈ edgesF f,
      ((e.1.z - (qd p δ).z) * (e.2.z - (qd p δ).z) < 0 → d0 (qd p δ) e.1 e.2 ≠ 0) ∧
      ((e.1.z - (qd q δ).z) * (e.2.z - (qd q δ).z) < 0 → d0 (qd q δ) e.1 e.2 ≠ 0) := by
    intro f hf e he
    exact ⟨d0_ne p e.1 e.2 δ h0 (hbp (d0 p e.1 e.2, -(e.2.z - e.1.z), e.2.y - e.1.y)
        (mem_solidPolys hf he (by simp [edgePolys]))),
      d0_ne q e.1 e.2 δ h0 (hbq (d0 q e.1 e.2, -(e.2.z - e.1.z), e.2.y - e.1.y)
        (mem_solidPolys hf he (by simp [edgePolys])))⟩
  have fP : q.z ≠ p.z → ∀ f ∈ facesL S, ∀ e ∈ edgesF f,
      Fs (qd p δ) (qd q δ) e.1 ≠ 0 ∧ Fs (qd p δ) (qd q δ) e.2 ≠ 0 := by
    intro hz f hf e he
    have hm : e ∈ allEdges S := List.mem_flatMap.mpr ⟨f, hf, he⟩
    constructor
    · rw [Fs_qd]
      exact small_ne _ _ _ δ h0 (hbpq (Fs p q e.1, -(q.z - p.z), q.y - p.y)
        (List.mem_flatMap.mpr ⟨e, hm, by simp⟩)) (Or.inr (Or.inl (by grind)))
    · rw [Fs_qd]
      exact small_ne _ _ _ δ h0 (hbpq (Fs p q e.2, -(q.z - p.z), q.y - p.y)
        (List.mem_flatMap.mpr ⟨e, hm, by simp⟩)) (Or.inr (Or.inl (by grind)))
  have hzz : (qd p δ).z < (qd q δ).z ↔ p.z < q.z := by simp only [qd]; grind
  have hyy : (qd p δ).y < (qd q δ).y ↔ p.y < q.y := by simp only [qd]; grind
  rcases trich p.z q.z with hz | hz | hz
  · apply move_rise' S hflat hcl _ _ (hzz.mpr hz) hpass
    intro f hf e he
    obtain ⟨a1, b1, a2, b2⟩ := zP f hf e he
    exact ⟨a1, b1, a2, b2, (fP (by grind) f hf e he).1, (fP (by grind) f hf e he).2,
      (dP f hf e he).1, (dP f hf e he).2⟩
  · rcases trich p.y q.y with hy | hy | hy
    · apply move_level' S hflat hcl _ _ (by simp only [qd]; grind) (hyy.mpr hy) hpass
      intro f hf e he
      obtain ⟨a1, b1, _, _⟩ := zP f hf e he
      refine ⟨a1, b1, fun hs => (dP f hf e he).2 ?_⟩
      have : (qd q δ).z = (qd p δ).z := by simp only [qd]; grind
      rw [this]; exact hs
    · exact move_x' S hflat _ _ (by simp only [qd]; grind) (by simp only [qd]; grind) hpass
    · symm
      apply move_level' S hflat hcl _ _ (by simp only [qd]; grind) (by simp only [qd]; grind) hpass'
      intro f hf e he
      obtain ⟨_, _, a2, b2⟩ := zP f hf e he
      refine ⟨a2, b2, fun hs => (dP f hf e he).1 ?_⟩
      have : (qd p δ).z = (qd q δ).z := by simp only [qd]; grind
      rw [this]; exact hs
  · symm
    apply move_rise' S hflat hcl _ _ (by simp only [qd]; grind) hpass'
    intro f hf e he
    obtain ⟨a1, b1, a2, b2⟩ := zP f hf e he
    have h1 := fP (by grind) f hf e he
    exact ⟨a2, b2, a1, b1, by rw [Fs_swap]; grind, by rw [Fs_swap]; grind,
      (dP f hf e he).2, (dP f hf e he).1⟩

end ForgeExact.Jordan
