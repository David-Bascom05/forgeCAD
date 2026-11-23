import ForgeText.Seg

/-!
# A TrueType contour from its points (kernel/forge-font/src/lib.rs: `contour`)

Points come on or off the curve. An off point between two others is a quadratic's control;
two off points in a row have an on point implied midway. The walk starts at the first on point
(or, with none, midway between the first two) and ends back there. The grid is twice the font's
unit, so every implied midpoint is a grid point (`mid` divides an even sum).
-/

namespace ForgeText

def mid (a b : Pt) : Pt := ⟨(a.x + b.x) / 2, (a.y + b.y) / 2⟩

/-- One step of the walk: where the pen is, the pending control, the pieces so far. -/
structure Walk where
  at_ : Pt
  ctrl : Option Pt
  out : List Seg

def step (w : Walk) : Pt × Bool → Walk
  | (p, true) =>
    match w.ctrl with
    | none => { at_ := p, ctrl := none, out := if p = w.at_ then w.out else w.out ++ [.line w.at_ p] }
    | some c => { at_ := p, ctrl := none, out := if p = w.at_ then w.out else w.out ++ [.quad w.at_ c p] }
  | (p, false) =>
    match w.ctrl with
    | none => { w with ctrl := some p }
    | some c =>
      let m := mid c p
      { at_ := m, ctrl := some p, out := if m = w.at_ then w.out else w.out ++ [.quad w.at_ c m] }

/-- The pieces of a contour whose walk starts at `start` over `rest` (the points after it, in
order, the first again appended as an on point to close it). -/
def decode (start : Pt) (rest : List (Pt × Bool)) : List Seg :=
  (rest.foldl step { at_ := start, ctrl := none, out := [] }).out

/-- The pieces chain from `p`: each starts where the one before ended, the first at `p`. -/
def Chain : Pt → List Seg → Prop
  | _, [] => True
  | p, s :: rest => s.start = p ∧ Chain s.stop rest

/-- Where a chain from `p` ends. -/
def endOf : Pt → List Seg → Pt
  | p, [] => p
  | _, s :: rest => endOf s.stop rest

theorem chain_append (p : Pt) (l : List Seg) (s : Seg) :
    Chain p (l ++ [s]) ↔ Chain p l ∧ s.start = endOf p l := by
  induction l generalizing p with
  | nil => simp [Chain, endOf]
  | cons x xs ih => simp [Chain, endOf, ih, and_assoc]

theorem endOf_append (p : Pt) (l : List Seg) (s : Seg) : endOf p (l ++ [s]) = s.stop := by
  induction l generalizing p with
  | nil => rfl
  | cons x xs ih => exact ih _

/-- The walk's invariant: its pieces chain from the start and end where the pen is. -/
def Inv (start : Pt) (w : Walk) : Prop := Chain start w.out ∧ endOf start w.out = w.at_

theorem step_inv (start : Pt) (w : Walk) (q : Pt × Bool) (h : Inv start w) : Inv start (step w q) := by
  obtain ⟨hc, he⟩ := h
  obtain ⟨p, b⟩ := q
  cases b <;> cases hw : w.ctrl <;> simp only [step, hw]
  · exact ⟨hc, he⟩
  · split
    · next heq => exact ⟨hc, by rw [he]; exact heq.symm⟩
    · exact ⟨(chain_append _ _ _).2 ⟨hc, by simp [Seg.start, he]⟩, by simp [endOf_append, Seg.stop]⟩
  · split
    · next heq => exact ⟨hc, by rw [he]; exact heq.symm⟩
    · exact ⟨(chain_append _ _ _).2 ⟨hc, by simp [Seg.start, he]⟩, by simp [endOf_append, Seg.stop]⟩
  · split
    · next heq => exact ⟨hc, by rw [he]; exact heq.symm⟩
    · exact ⟨(chain_append _ _ _).2 ⟨hc, by simp [Seg.start, he]⟩, by simp [endOf_append, Seg.stop]⟩

theorem foldl_inv (start : Pt) (l : List (Pt × Bool)) (w : Walk) (h : Inv start w) :
    Inv start (l.foldl step w) := by
  induction l generalizing w with
  | nil => exact h
  | cons q qs ih => exact ih _ (step_inv start w q h)

/-- An on point leaves the pen on it. -/
theorem step_on_at (w : Walk) (p : Pt) : (step w (p, true)).at_ = p := by
  cases hw : w.ctrl <;> simp [step, hw]

/-- The decoder's claim (`contour`): the pieces chain from the start and, the walk ending on the
start (on the curve), close there: the contour is closed. -/
theorem decode_closes (start : Pt) (rest : List (Pt × Bool)) :
    Chain start (decode start (rest ++ [(start, true)])) ∧
      endOf start (decode start (rest ++ [(start, true)])) = start := by
  have h := foldl_inv start (rest ++ [(start, true)]) { at_ := start, ctrl := none, out := [] }
    ⟨trivial, rfl⟩
  have hat : ((rest ++ [(start, true)]).foldl step { at_ := start, ctrl := none, out := [] }).at_ = start := by
    rw [List.foldl_append]
    simp only [List.foldl_cons, List.foldl_nil]
    exact step_on_at _ _
  unfold decode
  exact ⟨h.1, h.2.trans hat⟩

/-- No piece is a point: a piece is made only where the pen moves. -/
theorem step_out_prefix (w : Walk) (q : Pt × Bool) : ∃ t, (step w q).out = w.out ++ t := by
  obtain ⟨p, b⟩ := q
  cases b <;> cases hc : w.ctrl <;> simp only [step, hc]
  · exact ⟨[], by simp⟩
  · split
    · exact ⟨[], by simp⟩
    · exact ⟨_, rfl⟩
  · split
    · exact ⟨[], by simp⟩
    · exact ⟨_, rfl⟩
  · split
    · exact ⟨[], by simp⟩
    · exact ⟨_, rfl⟩

/-- A contour from its points, as `contour` walks them: from the first on point (or, with none,
from midway between the first two), over the points after it round to the one before it, and
back to the start on the curve. Fewer than two points make nothing. -/
def contourOf (ps : List (Pt × Bool)) : List Seg :=
  if 2 ≤ ps.length then
    match ps.findIdx? (·.2) with
    | some k =>
      let s := (ps.getD k (⟨0, 0⟩, true)).1
      decode s (ps.drop (k + 1) ++ ps.take k ++ [(s, true)])
    | none =>
      let s := mid (ps.getD 0 (⟨0, 0⟩, true)).1 (ps.getD 1 (⟨0, 0⟩, true)).1
      decode s (ps.drop 1 ++ ps.take 1 ++ [(s, true)])
  else []

/-- Every contour so made is closed: its pieces chain from a start and end back on it. -/
theorem contourOf_closes (ps : List (Pt × Bool)) :
    ∃ s, Chain s (contourOf ps) ∧ endOf s (contourOf ps) = s := by
  unfold contourOf
  split
  · split
    · exact ⟨_, decode_closes _ _⟩
    · exact ⟨_, decode_closes _ _⟩
  · exact ⟨⟨0, 0⟩, trivial, rfl⟩

end ForgeText
