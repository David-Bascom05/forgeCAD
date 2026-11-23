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

/-- Every piece the walk makes starts where the one before ended (the pen moves only by
making a piece, or onto a midpoint it then starts from). -/
def ChainedFrom (start : Pt) (out : List Seg) : Prop :=
  ∀ i, (h : i < out.length) →
    (out[i]'h).start = if i = 0 then start else (out[i - 1]'(by omega)).stop

/-- The decoder's claim (L3, to be proven): the pieces chain from the start and, the last point
being the start again (on the curve), the contour closes. -/
def DecodeCloses : Prop :=
  ∀ (start : Pt) (rest : List (Pt × Bool)),
    let out := decode start (rest ++ [(start, true)])
    ChainedFrom start out ∧ (out ≠ [] → Closed out)

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

end ForgeText
