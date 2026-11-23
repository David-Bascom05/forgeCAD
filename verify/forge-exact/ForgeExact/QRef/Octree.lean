import ForgeExact.QRef.BF
import ForgeExact.Quad.Cert

/-!
# The octree's decisions, proven (research/kernel-audit/curved-l2-proofs.md §1)

`R qs f` is the regularised history: the closure of the interior of the points whose sign
pattern (`sig`) makes the formula `f` true. These are the statements `QRef.octree` and the
`empty` verdict rely on:

* `reg_off_wall`: at a point off every quadric, `R` holds iff `f` holds on its pattern;
* `in_box` and `out_box`: a box whose certified signs (`boxSign`) decide `f` (`BF.eval3`) is, at
  every point off the wall, in `R` or out of it (so its volume is counted, or not);
* `empty_of_leaves`: if every point off the wall lies in a decided-out box or in a leaf whose
  remaining patterns `BF.allFalse` rules out (exclusions sound, as `Quad.pairCheck_spec` makes
  them), or is outside every tool, then `R` is empty.

What stays a hypothesis is the cover itself (the leaves and the out-boxes tile the root cube,
and the root cube holds every tool): that is how the octree is built, by halving boxes.
-/

namespace ForgeExact.QRef

open ForgeExact Topology Ref Flat Quad

/-- The zero quadric, for indices past the list. -/
def zeroQ : Quadric := ⟨0, 0, 0, 0, 0, 0, 0, 0, 0, 0⟩

/-- A point's sign pattern. -/
def sig (qs : List Quadric) (x : V3) : Nat → Bool := fun k => decide ((qs.getD k zeroQ).eval x > 0)

/-- The box's certified signs. -/
def boxSigns (qs : List Quadric) (c : V3) (h : Rat) : Nat → Int := fun k => boxSign (qs.getD k zeroQ) c h

/-- The regularised history. -/
def R (qs : List Quadric) (f : BF) : Pred V3 := reg fun x => f.eval2 (sig qs x) = true

theorem getD_lt {qs : List Quadric} {k : Nat} (hk : k < qs.length) : qs.getD k zeroQ = qs[k] := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk]; rfl

theorem getD_ge {qs : List Quadric} {k : Nat} (hk : ¬k < qs.length) : qs.getD k zeroQ = zeroQ := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_none (by grind)]; rfl

theorem getD_mem {qs : List Quadric} {k : Nat} (hk : k < qs.length) : qs.getD k zeroQ ∈ qs := by
  rw [getD_lt hk]; exact List.getElem_mem hk

theorem eval_zeroQ (x : V3) : zeroQ.eval x = 0 := by simp only [zeroQ, Quadric.eval]; grind

theorem boxSign_zeroQ (c : V3) (h : Rat) : boxSign zeroQ c h = 0 := by
  have e1 := eval_zeroQ c
  have e2 : zeroQ.bound c h = 0 := by
    simp only [Quadric.bound, Quadric.qnorm, Quadric.grad, zeroQ, qabs]; grind
  unfold boxSign; rw [e1, e2]; decide

theorem boxSign_cases (q : Quadric) (c : V3) (h : Rat) :
    boxSign q c h = 1 ∨ boxSign q c h = -1 ∨ boxSign q c h = 0 := by
  unfold boxSign
  split
  · exact Or.inl rfl
  · split
    · exact Or.inr (Or.inl rfl)
    · exact Or.inr (Or.inr rfl)

/-- Points whose every quadric keeps `x`'s sign have `x`'s pattern. -/
theorem sig_eq {qs : List Quadric} {x y : V3}
    (h : ∀ q ∈ qs, (0 < q.eval x ↔ 0 < q.eval y) ∧ (q.eval x < 0 ↔ q.eval y < 0)) :
    sig qs y = sig qs x := by
  funext k
  simp only [sig]
  by_cases hk : k < qs.length
  · exact decide_eq_decide.mpr (h _ (getD_mem hk)).1.symm
  · rw [getD_ge hk, eval_zeroQ, eval_zeroQ]

/-- **Membership off the wall.** -/
theorem reg_off_wall (qs : List Quadric) (f : BF) {x : V3} (hx : ¬wall qs x) :
    R qs f x ↔ f.eval2 (sig qs x) = true := by
  have hne : ∀ q ∈ qs, q.eval x ≠ 0 := fun q hq h => hx ⟨q, hq, h⟩
  obtain ⟨hs, hxs, hsub⟩ := signs_nbhd qs hne
  have hconst : ∀ p, allPos hs p → (f.eval2 (sig qs p) = true ↔ f.eval2 (sig qs x) = true) :=
    fun p hp => by rw [sig_eq (hsub p hp)]
  constructor
  · intro hreg
    apply Classical.byContradiction; intro hn
    obtain ⟨y, hy, U, _, hU, hyU⟩ := hreg (allPos hs) (isOpen_allPos _) hxs
    exact hn ((hconst y hy).mp (hU y hyU))
  · intro hv
    exact sub_closure _ x ⟨allPos hs, isOpen_allPos _, fun p hp => (hconst p hp).mpr hv, hxs⟩

/-- A box's certified signs agree with every point of the box. -/
theorem boxSigns_agree (qs : List Quadric) (c : V3) (h : Rat) {p : V3} (hp : inBox c h p) :
    BF.Agree (boxSigns qs c h) (sig qs p) := by
  intro k hk
  simp only [boxSigns] at hk ⊢
  have s := boxSign_spec (qs.getD k zeroQ) c h p hp
  simp only [sig]
  rcases boxSign_cases (qs.getD k zeroQ) c h with e | e | e
  · exact decide_eq_decide.mpr ⟨fun _ => by simp only [e]; decide, fun _ => s.1 e⟩
  · exact decide_eq_decide.mpr ⟨fun h1 => by have := s.2 e; grind, fun h1 => by simp only [e] at h1; exact absurd h1 (by decide)⟩
  · exact absurd e hk

/-- **An in-box**: every point of it off the wall is in `R`. -/
theorem in_box (qs : List Quadric) (f : BF) (c : V3) (h : Rat)
    (hk : f.eval3 (boxSigns qs c h) = some true) {p : V3} (hp : inBox c h p) (hw : ¬wall qs p) :
    R qs f p :=
  (reg_off_wall qs f hw).mpr (BF.eval3_sound (boxSigns_agree qs c h hp) f true hk)

/-- **An out-box**: no point of it off the wall is in `R`. -/
theorem out_box (qs : List Quadric) (f : BF) (c : V3) (h : Rat)
    (hk : f.eval3 (boxSigns qs c h) = some false) {p : V3} (hp : inBox c h p) (hw : ¬wall qs p) :
    ¬R qs f p := by
  rw [reg_off_wall qs f hw, BF.eval3_sound (boxSigns_agree qs c h hp) f false hk]; simp

/-- Emptiness from patterns, for the formula. -/
theorem empty_of_sig (qs : List Quadric) (hqs : ∀ q ∈ qs, q.NZ) (f : BF)
    (hno : ∀ x, ¬wall qs x → f.eval2 (sig qs x) = false) (x : V3) : ¬R qs f x := by
  intro hx
  obtain ⟨y, _, U, hU, hUs, hyU⟩ := hx (fun _ => True) isOpen_univ trivial
  obtain ⟨hs, hys, hsU⟩ := hU y hyU
  obtain ⟨z, hz, hoff⟩ := Quad.escape qs hqs hs y hys
  have hzw : ¬wall qs z := fun ⟨q, hq, h0⟩ => hoff q hq h0
  have := hUs z (hsU z hz)
  simp only at this
  rw [hno z hzw] at this; simp at this

/-- A leaf of the octree: a box, decided out or left undecided. -/
structure Leaf where
  c : V3
  h : Rat

/-- The undecided signs of a leaf, and the completion's start (`Arr.anyKept`). -/
def Leaf.und (qs : List Quadric) (L : Leaf) : List Nat :=
  (List.range qs.length).filter fun k => boxSigns qs L.c L.h k = 0
def Leaf.σ0 (qs : List Quadric) (L : Leaf) : Nat → Bool := fun k => decide (boxSigns qs L.c L.h k > 0)

/-- **Emptiness from the leaves.** If the exclusions hold at every point off the wall (as
`pairCheck_spec` gives them), and every point off the wall is outside every tool, or in a box
decided out, or in a leaf whose remaining patterns `allFalse` rules out, then `R` is empty. -/
theorem empty_of_leaves (qs : List Quadric) (hqs : ∀ q ∈ qs, q.NZ) (f : BF) (ex : List Excl)
    (hex : ∀ x, ¬wall qs x → BF.excluded ex (sig qs x) = false)
    (hcover : ∀ x, ¬wall qs x → f.eval2 (sig qs x) = false ∨
      ∃ L : Leaf, inBox L.c L.h x ∧ (f.eval3 (boxSigns qs L.c L.h) = some false ∨
        BF.allFalse f ex (L.und qs) (L.σ0 qs) = true))
    (x : V3) : ¬R qs f x := by
  apply empty_of_sig qs hqs f
  intro y hy
  rcases hcover y hy with h | ⟨L, hL, h | h⟩
  · exact h
  · exact BF.eval3_sound (boxSigns_agree qs L.c L.h hL) f false h
  · apply BF.allFalse_sound f ex (L.und qs) (L.σ0 qs) (sig qs y) h _ (hex y hy)
    intro m hm
    have ag := boxSigns_agree qs L.c L.h hL m
    simp only [Leaf.σ0]
    by_cases hml : m < qs.length
    · have h0 : boxSigns qs L.c L.h m ≠ 0 := fun e =>
        hm (List.mem_filter.mpr ⟨List.mem_range.mpr hml, by simp [e]⟩)
      rw [ag h0]
    · simp only [sig, boxSigns]
      simp only [getD_ge hml, eval_zeroQ, boxSign_zeroQ]
      exact decide_eq_decide.mpr ⟨fun h1 => absurd h1 (by grind), fun h1 => absurd h1 (by decide)⟩

end ForgeExact.QRef
