/-!
# Glyph segments on the font's integer grid (kernel/forge-font: `Seg`, `layout`)

A glyph's points are the font's own numbers: TrueType coordinates are integers (an implied
on-curve point is a midpoint, so the grid is taken at twice the unit), CFF ones multiples of
2⁻¹⁶. Scaled to one integer grid, every point is a pair of integers, and laying a string out
moves a glyph by whole grid steps (advances and kerning are whole font units). So the layout's
outline is the font's curves exactly, each moved by its pen offset: `place_*` below.
-/

namespace ForgeText

structure Pt where
  x : Int
  y : Int
  deriving DecidableEq, Repr

instance : Add Pt := ⟨fun a b => ⟨a.x + b.x, a.y + b.y⟩⟩

@[simp] theorem Pt.add_x (a b : Pt) : (a + b).x = a.x + b.x := rfl
@[simp] theorem Pt.add_y (a b : Pt) : (a + b).y = a.y + b.y := rfl

/-- A piece of an outline: a line, a quadratic (TrueType) or a cubic (CFF) Bézier by its
control points. The curve itself is the Bézier of those points (the spline-spec project's
definition); nothing here needs more than the control points. -/
inductive Seg where
  | line (a b : Pt)
  | quad (a c b : Pt)
  | cubic (a c d b : Pt)
  deriving DecidableEq, Repr

def Seg.start : Seg → Pt
  | .line a _ | .quad a _ _ | .cubic a _ _ _ => a

def Seg.stop : Seg → Pt
  | .line _ b | .quad _ _ b | .cubic _ _ _ b => b

/-- The same piece run the other way (`Seg::reversed`). -/
def Seg.reverse : Seg → Seg
  | .line a b => .line b a
  | .quad a c b => .quad b c a
  | .cubic a c d b => .cubic b d c a

/-- Moved by `d` (`layout`'s pen offset). -/
def Seg.place (d : Pt) : Seg → Seg
  | .line a b => .line (a + d) (b + d)
  | .quad a c b => .quad (a + d) (c + d) (b + d)
  | .cubic a c e b => .cubic (a + d) (c + d) (e + d) (b + d)

theorem Seg.reverse_reverse (s : Seg) : s.reverse.reverse = s := by
  cases s <;> rfl

theorem Seg.start_reverse (s : Seg) : s.reverse.start = s.stop := by
  cases s <;> rfl

theorem Seg.stop_reverse (s : Seg) : s.reverse.stop = s.start := by
  cases s <;> rfl

theorem Seg.start_place (d : Pt) (s : Seg) : (s.place d).start = s.start + d := by
  cases s <;> rfl

theorem Seg.stop_place (d : Pt) (s : Seg) : (s.place d).stop = s.stop + d := by
  cases s <;> rfl

/-- A contour is closed: each piece ends where the next starts, the last where the first
starts. -/
def Closed (c : List Seg) : Prop :=
  ∀ i, (h : i < c.length) → (c[i]'h).stop = (c[(i + 1) % c.length]'(Nat.mod_lt _ (by omega))).start

/-- Placing keeps the font's kind of piece and its control points, each moved by the same
offset: the laid-out outline is the font's, exactly. -/
theorem Seg.place_kind (d : Pt) (s : Seg) :
    (∃ a b, s = .line a b ∧ s.place d = .line (a + d) (b + d)) ∨
    (∃ a c b, s = .quad a c b ∧ s.place d = .quad (a + d) (c + d) (b + d)) ∨
    (∃ a c e b, s = .cubic a c e b ∧ s.place d = .cubic (a + d) (c + d) (e + d) (b + d)) := by
  cases s with
  | line a b => exact .inl ⟨a, b, rfl, rfl⟩
  | quad a c b => exact .inr (.inl ⟨a, c, b, rfl, rfl⟩)
  | cubic a c e b => exact .inr (.inr ⟨a, c, e, b, rfl, rfl⟩)

end ForgeText
