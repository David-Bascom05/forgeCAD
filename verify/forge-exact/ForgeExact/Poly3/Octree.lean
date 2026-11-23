import ForgeExact.Poly3.Basic
import ForgeExact.QRef.BF

/-!
# The boolean of solids bounded by polynomials of any degree, decided exactly

Step 4 of the exact reference: tori (degree 4) and whatever else is polynomial. These are the
quadric results (ForgeExact/Quad, ForgeExact/QRef/Octree.lean) for polynomials of any degree:

* `wall_nowhereDense`: the zero sets of nonzero polynomials are nowhere dense;
* `reg_off_wall`: membership is decided exactly off the wall;
* `in_box`, `out_box`: a box that certifies signs (`boxSign`) and decides the formula
  (`BF.eval3`) is in or out at every point off the wall;
* `empty_of_leaves`: emptiness from a cover of out-boxes and ruled-out leaves.

The topology is `Flat`'s on ℚ³; a basic open set is an intersection of open half-spaces.
-/

namespace ForgeExact.Poly3

open ForgeExact Topology Ref Flat QRef

def zeroP : P3 := []

theorem eval_zeroP (x : V3) : zeroP.eval x = 0 := rfl

/-- The wall: the union of the polynomials' zero sets. -/
def wall (ps : List P3) : Pred V3 := fun x => ∃ p ∈ ps, p.eval x = 0

theorem signs_nbhd : ∀ (ps : List P3) {y : V3}, (∀ p ∈ ps, p.eval y ≠ 0) →
    ∃ hs, allPos hs y ∧ ∀ x, allPos hs x → ∀ p ∈ ps, (0 < p.eval y ↔ 0 < p.eval x) ∧
      (p.eval y < 0 ↔ p.eval x < 0)
  | [], _, _ => ⟨[], ⟨fun _ h => by simp at h, fun _ _ _ h => by simp at h⟩⟩
  | p :: ps, y, hy => by
    obtain ⟨a, ha, ha'⟩ := sign_nbhd p (hy p (List.mem_cons_self ..))
    obtain ⟨b, hb, hb'⟩ := signs_nbhd ps fun r hr => hy r (List.mem_cons_of_mem _ hr)
    refine ⟨a ++ b, fun P hP => (List.mem_append.mp hP).elim (ha P) (hb P), fun x hx r hr => ?_⟩
    have hxa : allPos a x := fun P hP => hx P (List.mem_append_left _ hP)
    have hxb : allPos b x := fun P hP => hx P (List.mem_append_right _ hP)
    rcases List.mem_cons.mp hr with rfl | hr
    · exact ha' x hxa
    · exact hb' x hxb r hr

theorem escape (ps : List P3) (hps : ∀ p ∈ ps, p.NZ) :
    ∀ (hs : List Plane) (x : V3), allPos hs x → ∃ y, allPos hs y ∧ ∀ p ∈ ps, p.eval y ≠ 0 := by
  induction ps with
  | nil => exact fun hs x hx => ⟨x, hx, fun _ h => nomatch h⟩
  | cons p ps ih =>
    intro hs x hx
    obtain ⟨y0, hy0, hp0⟩ := escape_one hs p (hps p (List.mem_cons_self ..)) hx
    obtain ⟨b, hb, hb'⟩ := sign_nbhd p hp0
    obtain ⟨y, hy, hoff⟩ := ih (fun r hr => hps r (List.mem_cons_of_mem _ hr)) (hs ++ b) y0
      (fun P hP => (List.mem_append.mp hP).elim (hy0 P) (hb P))
    refine ⟨y, fun P hP => hy P (List.mem_append_left _ hP), fun r hr => ?_⟩
    rcases List.mem_cons.mp hr with rfl | hr
    · have := hb' y fun P hP => hy P (List.mem_append_right _ hP)
      intro h0; rw [h0] at this
      rcases Lean.Grind.LinearOrder.trichotomy (r.eval y0) 0 with h | h | h <;> grind
    · exact hoff r hr

theorem closure_wall_sub (ps : List P3) : (closure (wall ps)).Sub (wall ps) := by
  intro x hx
  apply Classical.byContradiction
  intro hoff
  have hne : ∀ p ∈ ps, p.eval x ≠ 0 := fun p hp h => hoff ⟨p, hp, h⟩
  obtain ⟨hs, hxs, hsub⟩ := signs_nbhd ps hne
  obtain ⟨y, hy, p, hp, hpy⟩ := hx (allPos hs) (isOpen_allPos _) hxs
  have := hsub y hy p hp
  have := hne p hp
  rcases Lean.Grind.LinearOrder.trichotomy (p.eval x) 0 with h | h | h <;> grind

/-- **A finite union of nonzero polynomials' zero sets is nowhere dense.** -/
theorem wall_nowhereDense (ps : List P3) (hps : ∀ p ∈ ps, p.NZ) : NowhereDense (wall ps) := by
  intro x ⟨U, hU, hUsub, hxU⟩
  obtain ⟨hs, hxs, hsU⟩ := hU x hxU
  obtain ⟨y, hy, hoff⟩ := escape ps hps hs x hxs
  obtain ⟨p, hp, h0⟩ := closure_wall_sub ps y (hUsub y (hsU y hy))
  exact hoff p hp h0

/-! ## The history over polynomials -/

/-- A point's sign pattern. -/
def sig (ps : List P3) (x : V3) : Nat → Bool := fun k => decide ((ps.getD k zeroP).eval x > 0)

/-- The regularised history. -/
def R (ps : List P3) (f : BF) : Pred V3 := reg fun x => f.eval2 (sig ps x) = true

theorem getD_lt {ps : List P3} {k : Nat} (hk : k < ps.length) : ps.getD k zeroP = ps[k] := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk]; rfl

theorem getD_ge {ps : List P3} {k : Nat} (hk : ¬k < ps.length) : ps.getD k zeroP = zeroP := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_none (by grind)]; rfl

theorem getD_mem {ps : List P3} {k : Nat} (hk : k < ps.length) : ps.getD k zeroP ∈ ps := by
  rw [getD_lt hk]; exact List.getElem_mem hk

theorem sig_eq {ps : List P3} {x y : V3}
    (h : ∀ p ∈ ps, (0 < p.eval x ↔ 0 < p.eval y) ∧ (p.eval x < 0 ↔ p.eval y < 0)) :
    sig ps y = sig ps x := by
  funext k
  simp only [sig]
  by_cases hk : k < ps.length
  · exact decide_eq_decide.mpr (h _ (getD_mem hk)).1.symm
  · rw [getD_ge hk]; rfl

/-- **Membership off the wall.** -/
theorem reg_off_wall (ps : List P3) (f : BF) {x : V3} (hx : ¬wall ps x) :
    R ps f x ↔ f.eval2 (sig ps x) = true := by
  have hne : ∀ p ∈ ps, p.eval x ≠ 0 := fun p hp h => hx ⟨p, hp, h⟩
  obtain ⟨hs, hxs, hsub⟩ := signs_nbhd ps hne
  have hconst : ∀ p, allPos hs p → (f.eval2 (sig ps p) = true ↔ f.eval2 (sig ps x) = true) :=
    fun p hp => by rw [sig_eq (hsub p hp)]
  constructor
  · intro hreg
    apply Classical.byContradiction; intro hn
    obtain ⟨y, hy, U, _, hU, hyU⟩ := hreg (allPos hs) (isOpen_allPos _) hxs
    exact hn ((hconst y hy).mp (hU y hyU))
  · intro hv
    exact sub_closure _ x ⟨allPos hs, isOpen_allPos _, fun p hp => (hconst p hp).mpr hv, hxs⟩

/-- The sign a box certifies for `p` (`0`: none). -/
def boxSign (p : P3) (c : V3) (h : Rat) : Int :=
  if p.eval c > bound p c h then 1 else if p.eval c < -bound p c h then -1 else 0

theorem boxSign_cases (p : P3) (c : V3) (h : Rat) :
    boxSign p c h = 1 ∨ boxSign p c h = -1 ∨ boxSign p c h = 0 := by
  unfold boxSign
  split
  · exact Or.inl rfl
  · split
    · exact Or.inr (Or.inl rfl)
    · exact Or.inr (Or.inr rfl)

theorem boxSign_spec (p : P3) (c : V3) {h : Rat} (hh : 0 ≤ h) (x : V3) (hx : Near c x h) :
    (boxSign p c h = 1 → 0 < p.eval x) ∧ (boxSign p c h = -1 → p.eval x < 0) := by
  have b := box_bound p c x hh hx
  have l := le_qabs (p.eval x - p.eval c)
  have l' := neg_le_qabs (p.eval x - p.eval c)
  unfold boxSign
  constructor
  · intro h1; split at h1
    · grind
    · split at h1 <;> simp at h1
  · intro h1; split at h1
    · simp at h1
    · split at h1
      · grind
      · simp at h1

def boxSigns (ps : List P3) (c : V3) (h : Rat) : Nat → Int := fun k => boxSign (ps.getD k zeroP) c h

theorem boxSign_zeroP (c : V3) (h : Rat) : boxSign zeroP c h = 0 := by
  unfold boxSign; simp [zeroP, P3.eval, bound]

theorem boxSigns_agree (ps : List P3) (c : V3) {h : Rat} (hh : 0 ≤ h) {x : V3} (hx : Near c x h) :
    BF.Agree (boxSigns ps c h) (sig ps x) := by
  intro k hk
  simp only [boxSigns] at hk ⊢
  have s := boxSign_spec (ps.getD k zeroP) c hh x hx
  simp only [sig]
  rcases boxSign_cases (ps.getD k zeroP) c h with e | e | e
  · exact decide_eq_decide.mpr ⟨fun _ => by simp only [e]; decide, fun _ => s.1 e⟩
  · exact decide_eq_decide.mpr ⟨fun h1 => by have := s.2 e; grind,
      fun h1 => by simp only [e] at h1; exact absurd h1 (by decide)⟩
  · exact absurd e hk

/-- **An in-box.** -/
theorem in_box (ps : List P3) (f : BF) (c : V3) {h : Rat} (hh : 0 ≤ h)
    (hk : f.eval3 (boxSigns ps c h) = some true) {x : V3} (hx : Near c x h) (hw : ¬wall ps x) :
    R ps f x :=
  (reg_off_wall ps f hw).mpr (BF.eval3_sound (boxSigns_agree ps c hh hx) f true hk)

/-- **An out-box.** -/
theorem out_box (ps : List P3) (f : BF) (c : V3) {h : Rat} (hh : 0 ≤ h)
    (hk : f.eval3 (boxSigns ps c h) = some false) {x : V3} (hx : Near c x h) (hw : ¬wall ps x) :
    ¬R ps f x := by
  rw [reg_off_wall ps f hw, BF.eval3_sound (boxSigns_agree ps c hh hx) f false hk]; simp

theorem empty_of_sig (ps : List P3) (hps : ∀ p ∈ ps, p.NZ) (f : BF)
    (hno : ∀ x, ¬wall ps x → f.eval2 (sig ps x) = false) (x : V3) : ¬R ps f x := by
  intro hx
  obtain ⟨y, _, U, hU, hUs, hyU⟩ := hx (fun _ => True) isOpen_univ trivial
  obtain ⟨hs, hys, hsU⟩ := hU y hyU
  obtain ⟨z, hz, hoff⟩ := escape ps hps hs y hys
  have hzw : ¬wall ps z := fun ⟨p, hp, h0⟩ => hoff p hp h0
  have := hUs z (hsU z hz)
  simp only at this
  rw [hno z hzw] at this; simp at this

/-- **Emptiness from the leaves** (as `QRef.empty_of_leaves`). -/
theorem empty_of_leaves (ps : List P3) (hps : ∀ p ∈ ps, p.NZ) (f : BF) (ex : List Excl)
    (hex : ∀ x, ¬wall ps x → BF.excluded ex (sig ps x) = false)
    (hcover : ∀ x, ¬wall ps x → f.eval2 (sig ps x) = false ∨
      ∃ c h, 0 ≤ h ∧ Near c x h ∧ (f.eval3 (boxSigns ps c h) = some false ∨
        BF.allFalse f ex ((List.range ps.length).filter fun k => boxSigns ps c h k = 0)
          (fun k => decide (boxSigns ps c h k > 0)) = true))
    (x : V3) : ¬R ps f x := by
  apply empty_of_sig ps hps f
  intro y hy
  rcases hcover y hy with h | ⟨c, h, hh, hn, hk | hk⟩
  · exact h
  · exact BF.eval3_sound (boxSigns_agree ps c hh hn) f false hk
  · apply BF.allFalse_sound f ex _ _ (sig ps y) hk _ (hex y hy)
    intro m hm
    have ag := boxSigns_agree ps c hh hn m
    by_cases hml : m < ps.length
    · have h0 : boxSigns ps c h m ≠ 0 := fun e =>
        hm (List.mem_filter.mpr ⟨List.mem_range.mpr hml, by simp [e]⟩)
      rw [ag h0]
    · simp only [sig, boxSigns]
      simp only [getD_ge hml, eval_zeroP, boxSign_zeroP]
      exact decide_eq_decide.mpr ⟨fun h1 => absurd h1 (by grind), fun h1 => absurd h1 (by decide)⟩

end ForgeExact.Poly3
